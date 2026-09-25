//! Project root, manifest, and new-project wizard tests.
//! 项目根、清单与新项目向导测试。

use super::super::state::new_project_field;
use super::*;
use nichlink_run_method::face_field;

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
    project.values[new_project_field::DIRECTORY] = root.display().to_string();
    project.values[new_project_field::PACKAGE] = "sample-app".to_owned();
    project.values[new_project_field::KIND] = "binary".to_owned();
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
    owner.values[face_field::PARENT] = root_id.to_string();
    owner.values[face_field::MODULE] = "workspace".to_owned();
    owner.values[face_field::NEEDS_REGISTRY] = "true".to_owned();
    owner.values[face_field::TREE_SLOT] = "workspace".to_owned();
    owner.values[face_field::KIND] = "Workspace".to_owned();
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
    child.values[face_field::PARENT] = workspace_id.to_string();
    child.values[face_field::MODULE] = "panel".to_owned();
    child.values[face_field::NEEDS_REGISTRY] = "true".to_owned();
    child.values[face_field::TREE_SLOT] = "panel".to_owned();
    child.values[face_field::KIND] = "Panel".to_owned();
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
    leaf.values[face_field::PARENT] = panel_id.to_string();
    leaf.values[face_field::MODULE] = "button".to_owned();
    leaf.values[face_field::KIND] = "Button".to_owned();
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
    control.values[face_field::PARENT] = root_id.to_string();
    control.values[face_field::MODULE] = "control".to_owned();
    control.values[face_field::NEEDS_REGISTRY] = "true".to_owned();
    control.values[face_field::KIND] = "Control".to_owned();
    app.submit_add(&control);
    let control_id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "control")
        .expect("control registry")
        .id;
    let mut control_child = AddState::new(control_id);
    control_child.values[face_field::PARENT] = control_id.to_string();
    control_child.values[face_field::MODULE] = "slider".to_owned();
    control_child.values[face_field::KIND] = "Slider".to_owned();
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
        if project.values[new_project_field::KIND] == "library"));
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
    project.values[new_project_field::DIRECTORY] = root.display().to_string();
    project.values[new_project_field::PACKAGE] = "sample-framework".to_owned();
    project.values[new_project_field::KIND] = "library".to_owned();
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
    project.values[new_project_field::DIRECTORY] = root.display().to_string();
    project.values[new_project_field::PACKAGE] = "empty-app".to_owned();
    app.submit_new_project(&project);

    assert!(!root.join("src/control").exists());
    assert!(app.registry.depth_first().is_empty());

    let _ = std::fs::remove_dir_all(&root);
}

/// A project that cannot be resolved is refused by name, never replaced by the
/// directory this crate was compiled in. That directory is the checkout or the
/// installed crate's sources, and it is exactly where a launch with nothing open
/// used to write the next face.
/// 解析不出的项目按名字被拒绝，绝不由本 crate 编译时所在的目录顶替。那目录是检出目录或
/// 已安装 crate 的源码，而"没打开任何项目"的启动过去正是把下一个注册面写在那里。
#[test]
fn an_unresolvable_project_is_refused_instead_of_falling_back() {
    let missing = std::env::temp_dir().join("nichlink-studio-missing-project");

    let nothing = resolve_project_from(None, None, None, None, false)
        .expect_err("no project at all must be an error");
    assert!(nothing.contains("no project to open"), "{nothing}");
    assert!(
        nothing.contains("NICH_LINK_PACKAGE_ROOT"),
        "the message must say how to point Studio at a project: {nothing}"
    );

    let explicit = resolve_project_from(None, Some(&missing), None, None, false)
        .expect_err("a path argument that is not a directory must be refused");
    assert!(explicit.contains("path argument"), "{explicit}");
    assert!(
        explicit.contains("nichlink-studio-missing-project"),
        "{explicit}"
    );

    let configured = resolve_project_from(None, None, Some(&missing), None, false)
        .expect_err("a configured root that is not a directory must be refused");
    assert!(
        configured.contains("NICH_LINK_PACKAGE_ROOT"),
        "{configured}"
    );

    let selected = resolve_project_from(Some(&missing), None, None, None, false)
        .expect_err("a selected project that vanished must be refused");
    assert!(selected.contains("select_project"), "{selected}");
}

/// The working directory is the only implicit project, and only when it holds a
/// package: a directory without a manifest is not guessed at.
/// 当前目录是唯一的隐式项目，而且只在它持有包时成立；没有清单的目录不会被猜。
#[test]
fn only_a_working_directory_that_holds_a_package_is_opened() {
    let current = std::env::temp_dir();
    assert_eq!(
        resolve_project_from(None, None, None, Some(&current), true),
        Ok(current.clone())
    );
    assert!(
        resolve_project_from(None, None, None, Some(&current), false).is_err(),
        "a directory without a Cargo.toml is not a project"
    );
}

/// A relative candidate resolves against the working directory, so a relative
/// argument or environment value names the directory the shell would name.
/// 相对候选值相对当前目录解析，因此相对参数或相对环境变量指向 shell 会指向的目录。
#[test]
fn a_relative_candidate_resolves_against_the_working_directory() {
    let root = std::env::temp_dir();
    let child = root.join("nichlink-studio-relative");
    std::fs::create_dir_all(&child).expect("fixture directory");
    assert_eq!(
        resolve_project_from(
            None,
            Some(std::path::Path::new("nichlink-studio-relative")),
            None,
            Some(&root),
            false,
        ),
        Ok(child.clone())
    );
    let _ = std::fs::remove_dir_all(&child);
}
