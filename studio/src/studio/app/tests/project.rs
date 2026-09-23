//! Project root, manifest, and new-project wizard tests.
//! 项目根、清单与新项目向导测试。

use super::*;

#[test]
fn standalone_studio_resolves_one_project_root_and_manifest() {
    let root = package_root();
    assert!(root.is_dir());
    assert_eq!(
        host_manifest().file_name().and_then(|name| name.to_str()),
        Some("Cargo.toml")
    );
}

#[test]
fn installed_studio_never_exports_cargo_git_cache_paths() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let checkout = std::env::temp_dir().join(format!("cargo-git-checkout-{suffix}"));
    std::fs::create_dir_all(checkout.join("core")).expect("core sibling");
    std::fs::create_dir_all(checkout.join("build_method")).expect("build sibling");
    std::fs::create_dir_all(checkout.join("studio")).expect("studio sibling");

    let source = nichlink_build_method::scaffold::detected_source(
        &checkout.join("studio"),
        &checkout.join("outside-bin/nichlink-studio"),
    );
    let (core, build) = nichlink_build_method::scaffold::dependency_specs(&source);

    assert!(core.contains("git = \"https://github.com/Nichtigott/nichlink\""));
    assert!(build.contains("branch = \"main\""));
    assert!(!core.contains(checkout.to_string_lossy().as_ref()));
    assert!(!build.contains(checkout.to_string_lossy().as_ref()));
    let _ = std::fs::remove_dir_all(checkout);
}

#[test]
fn new_project_and_explicit_root_face_compile() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-new-{suffix}"));

    let mut app = App::load();
    app.handle_key(KeyEvent::from(KeyCode::Char('n')));
    let Some(Overlay::NewProject(mut project)) = app.overlay.take() else {
        panic!("n should open the New Project wizard");
    };
    project.values[0] = root.display().to_string();
    project.values[1] = "sample-app".to_owned();
    project.values[2] = "binary".to_owned();
    app.overlay = Some(Overlay::NewProject(project));
    app.handle_key(KeyEvent::from(KeyCode::Char('s')));

    assert!(root.join("Cargo.toml").is_file());
    assert!(root.join("build.rs").is_file());
    assert!(root.join("src/main.rs").is_file());
    assert!(
        std::fs::read_to_string(root.join("src/main.rs"))
            .expect("generated binary entry")
            .contains("builtin_static_plan().len()")
    );
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("project manifest");
    assert!(manifest.contains("nichlink-run-method"));
    assert!(manifest.contains("nichlink-build"));
    assert!(app.registry.depth_first().is_empty());

    let root_id = app.registry.id();
    let mut owner = AddState::new(root_id);
    owner.values[0] = root_id.to_string();
    owner.values[1] = "workspace".to_owned();
    owner.values[2] = "true".to_owned();
    owner.values[3] = "workspace".to_owned();
    owner.values[8] = "Workspace".to_owned();
    app.submit_add(&owner);
    assert!(
        root.join("src/workspace/registry_rule/registry_rule.rs")
            .is_file()
    );
    let workspace =
        std::fs::read_to_string(root.join("src/workspace/workspace.rs")).expect("workspace face");
    assert!(workspace.starts_with("// generated-by=NichLink"));
    assert!(workspace.contains("crate::root_object!"));
    assert!(workspace.contains("crate::root_object! {\n    kind: Workspace,"));

    let workspace_id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "workspace")
        .unwrap_or_else(|| panic!("explicit workspace registry; event={}", app.event))
        .id;
    let mut child = AddState::new(workspace_id);
    child.values[0] = workspace_id.to_string();
    child.values[1] = "panel".to_owned();
    child.values[2] = "true".to_owned();
    child.values[3] = "panel".to_owned();
    child.values[8] = "Panel".to_owned();
    app.submit_add(&child);
    assert!(
        root.join("src/workspace/object/panel/registry_rule/registry_rule.rs")
            .is_file()
    );
    let panel = std::fs::read_to_string(root.join("src/workspace/object/panel/panel.rs"))
        .expect("panel face");
    assert!(panel.contains("crate::workspace_object!"));
    // A face that owns a registry does not repeat the rule path: the declaration
    // resolves `registry_rule:` to the canonical module beside the face, and the
    // file written just above is that module.
    // 拥有注册机的面不再重复规则路径：声明把 `registry_rule:` 解析到注册面旁边的规范
    // 模块，而上面刚写下的那个文件就是该模块。
    assert!(panel.contains("needs_registry: true"));
    assert!(
        !panel.contains("registry_rule:"),
        "the derived rule must not be written back: {panel}"
    );
    let panel_id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "panel")
        .expect("panel registry")
        .id;
    let mut leaf = AddState::new(panel_id);
    leaf.values[0] = panel_id.to_string();
    leaf.values[1] = "button".to_owned();
    leaf.values[8] = "Button".to_owned();
    app.submit_add(&leaf);
    let button =
        std::fs::read_to_string(root.join("src/workspace/object/panel/object/button/button.rs"))
            .expect("third-level button face");
    assert!(button.contains("crate::panel_object!"));
    assert!(button.contains("parent: crate::workspace::object::panel::NODE_ID"));

    // `control_object!` is no longer the generic implementation. It is
    // generated only as the declaration name for children of `control`.
    // `control_object!` 不再是通用实现，只作为 control 子对象的声明名生成。
    let mut control = AddState::new(root_id);
    control.values[0] = root_id.to_string();
    control.values[1] = "control".to_owned();
    control.values[2] = "true".to_owned();
    control.values[8] = "Control".to_owned();
    app.submit_add(&control);
    let control_id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "control")
        .expect("control registry")
        .id;
    let mut control_child = AddState::new(control_id);
    control_child.values[0] = control_id.to_string();
    control_child.values[1] = "slider".to_owned();
    control_child.values[8] = "Slider".to_owned();
    app.submit_add(&control_child);
    let slider = std::fs::read_to_string(root.join("src/control/object/slider/slider.rs"))
        .expect("control child face");
    assert!(slider.contains("crate::control_object!"));
    assert!(slider.contains("parent: crate::control::NODE_ID"));

    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).expect("project manifest");
    assert!(manifest.contains("edition = \"2024\""));

    let check = std::process::Command::new("cargo")
        .args(["check", "--offline"])
        .current_dir(&root)
        .output()
        .expect("cargo check generated project");
    assert!(
        check.status.success(),
        "generated project failed to compile:\n{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn new_project_wizard_toggles_library_kind() {
    let mut app = App::load();
    app.handle_key(KeyEvent::from(KeyCode::Char('n')));
    app.handle_key(KeyEvent::from(KeyCode::Down));
    app.handle_key(KeyEvent::from(KeyCode::Down));
    app.handle_key(KeyEvent::from(KeyCode::Enter));
    assert!(matches!(app.overlay, Some(Overlay::NewProject(ref project))
        if project.values[2] == "library"));
}

#[test]
fn new_project_wizard_creates_library_entrypoint() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-lib-{suffix}"));

    let mut app = App::load();
    app.handle_key(KeyEvent::from(KeyCode::Char('n')));
    let Some(Overlay::NewProject(mut project)) = app.overlay.take() else {
        panic!("n should open the New Project wizard");
    };
    project.values[0] = root.display().to_string();
    project.values[1] = "sample-framework".to_owned();
    project.values[2] = "library".to_owned();
    app.overlay = Some(Overlay::NewProject(project));
    app.handle_key(KeyEvent::from(KeyCode::Char('s')));

    assert!(root.join("src/lib.rs").is_file());
    assert!(!root.join("src/main.rs").exists());
    assert!(!root.join("src/control").exists());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn new_project_starts_with_an_empty_registration_tree() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-empty-{suffix}"));

    let mut app = App::load();
    let mut project = super::super::NewProjectState::new();
    project.values[0] = root.display().to_string();
    project.values[1] = "empty-app".to_owned();
    app.submit_new_project(&project);

    assert!(!root.join("src/control").exists());
    assert!(app.registry.depth_first().is_empty());

    let _ = std::fs::remove_dir_all(&root);
}
