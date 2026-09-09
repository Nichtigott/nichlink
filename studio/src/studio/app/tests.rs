//! App interaction regression tests.
//! App 交互回归测试。

use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEventKind};
use ratatui::layout::Rect;

use super::support::{host_manifest, package_root, select_project};
use super::{
    AddState, App, Overlay, SearchState, StudioPage, advance_graph_focus,
    app_function_source_range, body_calls, function_bodies, function_symbols, visible_search_rows,
};

#[test]
fn workspace_page_shortcuts_route_to_the_expected_mode() {
    let mut app = App::load();
    app.handle_key(KeyEvent::from(KeyCode::Char('1')));
    assert_eq!(app.page, StudioPage::Search);
    assert!(matches!(app.overlay, Some(Overlay::Search(_))));

    app.handle_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.page, StudioPage::Inspect);
    assert!(app.overlay.is_none());

    app.handle_key(KeyEvent::from(KeyCode::Char('3')));
    assert_eq!(app.page, StudioPage::Data);
    assert!(matches!(app.overlay, Some(Overlay::Search(ref search)) if search.graph_mode));

    app.handle_key(KeyEvent::from(KeyCode::Esc));
    app.handle_key(KeyEvent::from(KeyCode::Esc));
    app.handle_key(KeyEvent::from(KeyCode::Char('4')));
    assert_eq!(app.page, StudioPage::Compare);
    assert!(
        matches!(app.overlay, Some(Overlay::Search(ref search)) if search.compare_query.is_some())
    );
}

#[test]
fn folding_a_call_hides_only_its_descendants() {
    let lines = [
        "call tree:",
        "  |-- root",
        "    |-- layout",
        "      `-- bounds",
        "",
        "  |-- sibling",
    ]
    .map(str::to_owned);
    let folded = BTreeSet::from([1]);

    let rows = visible_search_rows(&lines, &folded);
    let source_rows = rows.iter().map(|row| row.source_index).collect::<Vec<_>>();

    assert_eq!(source_rows, [0, 1, 5]);
    assert!(rows[1].has_children);
}

#[test]
fn divider_drag_is_clamped_and_stops_on_release() {
    let mut app = App::load();
    app.workspace_area = Rect::new(10, 5, 100, 20);
    app.tree_area = Rect::new(10, 5, 45, 20);

    app.handle_mouse(MouseEventKind::Down(MouseButton::Left), 55, 10);
    app.handle_mouse(MouseEventKind::Drag(MouseButton::Left), 105, 10);
    assert_eq!(app.split_percent, 70);

    app.handle_mouse(MouseEventKind::Up(MouseButton::Left), 105, 10);
    app.handle_mouse(MouseEventKind::Drag(MouseButton::Left), 35, 10);
    assert_eq!(app.split_percent, 70);
}

#[test]
fn add_overlay_click_selects_a_field_and_toggles_the_checkbox() {
    let mut app = App::load();
    app.overlay = Some(Overlay::Add(AddState::new(app.selected_parent())));
    app.overlay_area = Rect::new(5, 5, 70, 18);
    app.overlay_list_area = Rect::new(7, 7, 66, 10);

    app.handle_overlay_click(12, 16);

    let Some(Overlay::Add(add)) = app.overlay else {
        panic!("add overlay should remain open");
    };
    assert_eq!(add.field, 2);
    assert_eq!(add.values[2], "true");
    assert!(!add.editing);
}

#[test]
fn add_overlay_keyboard_navigation_reaches_kind_field() {
    let mut app = App::load();
    app.overlay = Some(Overlay::Add(AddState::new(app.selected_parent())));
    for _ in 0..2 {
        app.handle_overlay_key(crossterm::event::KeyEvent::from(
            crossterm::event::KeyCode::Down,
        ));
    }

    let Some(Overlay::Add(add)) = app.overlay else {
        panic!("add overlay should remain open");
    };
    assert_eq!(add.field, 8);
}

#[test]
fn add_navigation_never_changes_the_visible_field_set() {
    let mut app = App::load();
    app.overlay = Some(Overlay::Add(AddState::new(app.selected_parent())));
    let Some(Overlay::Add(initial)) = app.overlay.as_ref() else {
        panic!("add overlay should be open");
    };
    let before = super::face_field_indices(initial);

    for _ in 0..8 {
        app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
    }

    let Some(Overlay::Add(current)) = app.overlay.as_ref() else {
        panic!("add overlay should remain open");
    };
    let after = super::face_field_indices(current);
    assert_eq!(before, after, "moving to kind changed the form layout");
    assert_eq!(after.len(), nichlink::FACE_FIELD_COUNT);
}

#[test]
fn add_form_starts_with_editable_bilingual_summary() {
    let add = AddState::new(nichlink::ROOT_NODE_ID);
    assert!(add.values[9].is_empty());
    assert!(add.values[10].is_empty());
    assert!(add.values[11].is_empty());
    assert!(add.values[12].is_empty());
    assert!(add.values[29].is_empty());
}

#[test]
fn parent_rule_marks_the_fields_a_child_must_supply() {
    let mut add = AddState::new(nichlink::ROOT_NODE_ID);
    add.apply_parent_rule(&nichlink::OwnedRegistrationRule {
        required_preset: Some("ActionParts".to_owned()),
        required_parts: vec!["paint".to_owned()],
        required_exports: vec!["control.render".to_owned()],
        required_handle_traits: vec!["ControlHandle".to_owned()],
        required_part_traits: vec!["ActionPartsContract".to_owned()],
    });

    assert_eq!(
        add.parent_requirements.keys().copied().collect::<Vec<_>>(),
        [6, 7, 13, 20, 29]
    );
    assert_eq!(
        add.parent_requirements.get(&20).map(String::as_str),
        Some("handle implements ControlHandle")
    );
    assert_eq!(add.values[13], "ActionParts");
    assert_eq!(add.values[7], "control.render");
}

#[test]
fn edit_recovers_compiler_checked_contract_paths_from_source() {
    let source = r#"crate::root_object! {
        kind: Button,
        handle_contracts: [crate::ui::ControlHandle],
        part_contracts: [crate::ui::ActionPartsContract],
    }"#;

    assert_eq!(
        super::keyboard::declaration_contract_paths(source),
        (
            "crate::ui::ControlHandle".to_owned(),
            "crate::ui::ActionPartsContract".to_owned(),
        )
    );
}

#[test]
fn stable_identity_is_editable_from_the_form() {
    let mut app = App::load();
    let mut add = AddState::new(app.selected_parent());
    add.field = 16;
    app.overlay = Some(Overlay::Add(add));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));

    assert!(matches!(app.overlay, Some(Overlay::Add(ref add)) if add.editing));
}

#[test]
fn derived_storage_fields_do_not_open_a_fake_editor() {
    let mut app = App::load();
    let mut add = AddState::new(app.selected_parent());
    add.field = 18;
    app.overlay = Some(Overlay::Add(add));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));

    assert!(matches!(app.overlay, Some(Overlay::Add(ref add)) if !add.editing));
}

#[test]
fn graft_creates_external_overlay_plan_without_touching_source() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-external-graft-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("create source root");
    select_project(root.clone(), root.join("Cargo.toml"), "external-graft-test");

    let mut app = App::load();
    let mut add = AddState::new(app.registry.id());
    add.values[1] = "canvas".to_owned();
    add.values[8] = "Canvas".to_owned();
    app.submit_add(&add);
    let target = app
        .registry
        .depth_first()
        .into_iter()
        .find(|face| face.registry_name == "canvas")
        .expect("canvas face")
        .id;
    app.selected = target;
    let source = root.join("src").join(
        app.registry
            .find(target)
            .expect("target metadata")
            .source
            .file
            .as_str(),
    );
    let before = std::fs::read(&source).expect("source exists");
    app.handle_key(KeyEvent::from(KeyCode::Char('g')));
    assert!(
        app.event.contains("External graft plan created"),
        "{}",
        app.event
    );
    let (plan_path, line) = app
        .take_editor_request()
        .expect("external plan editor request");
    assert!(plan_path.ends_with("external-grafts/canvas_graft/graft.plan"));
    assert_eq!(line, 1);
    assert_eq!(std::fs::read(&source).expect("source remains"), before);
    assert!(
        root.join(".nichlink/external-grafts/canvas_graft/graft.plan")
            .is_file()
    );

    let _ = std::fs::remove_dir_all(&root);
}

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
    std::fs::create_dir_all(checkout.join("build")).expect("build sibling");
    std::fs::create_dir_all(checkout.join("studio")).expect("studio sibling");

    let source = nichlink_build::scaffold::detected_source(
        &checkout.join("studio"),
        &checkout.join("outside-bin/nichlink-studio"),
    );
    let (core, build) = nichlink_build::scaffold::dependency_specs(&source);

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
    assert!(manifest.contains("nichlink-core"));
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
    assert!(panel.contains("crate::workspace::object::panel::registry_rule::REGISTRATION_RULE"));
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
    let mut project = super::NewProjectState::new();
    project.values[0] = root.display().to_string();
    project.values[1] = "empty-app".to_owned();
    app.submit_new_project(&project);

    assert!(!root.join("src/control").exists());
    assert!(app.registry.depth_first().is_empty());

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn edit_save_button_writes_changes_and_adopts_the_old_control_scaffold() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-legacy-{suffix}"));
    let control = root.join("src/control/control.rs");
    let rule = root.join("src/control/registry_rule/registry_rule.rs");
    std::fs::create_dir_all(rule.parent().expect("rule parent")).expect("create fixture");
    std::fs::write(
        &control,
        "pub struct ControlRegistry;\n\ncrate::control_object! {\n    kind: ControlRegistry,\n    needs_registry: true,\n    registry_name: control,\n    parent: crate::ROOT_NODE_ID,\n    registry_rule_path: \"src/control/registry_rule/registry_rule.rs\",\n    registry_rule: crate::control::registry_rule::REGISTRATION_RULE,\n}\n",
    )
    .expect("write legacy control");
    std::fs::write(
        &rule,
        "use crate::RegistrationRule;\npub const REGISTRATION_RULE: RegistrationRule = RegistrationRule::ANY;\n",
    )
    .expect("write legacy rule");
    select_project(root.clone(), root.join("Cargo.toml"), "legacy-app");

    let mut app = App::load();
    app.selected = app.registry.depth_first()[0].id;
    app.handle_key(KeyEvent::from(KeyCode::Char('e')));
    let Some(Overlay::Edit(id, mut edit)) = app.overlay.take() else {
        panic!("e should open the Edit form");
    };
    edit.field = 11;
    edit.values[11] = "已保存的摘要。".to_owned();
    edit.editing = true;
    app.overlay = Some(Overlay::Edit(id, edit));
    app.overlay_area = Rect::new(5, 5, 70, 18);
    app.action_confirm_area = Rect::new(10, 18, 14, 3);
    app.handle_overlay_click(12, 19);

    assert!(!app.event.starts_with("Edit failed"), "{}", app.event);
    let source = std::fs::read_to_string(&control).expect("edited control");
    assert!(source.starts_with("// generated-by=NichLink"));
    assert!(source.contains("已保存的摘要。"));
    assert_eq!(
        app.selected_info().map(|info| info.summary.zh.as_str()),
        Some("已保存的摘要。")
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn startup_accepts_a_pre_hierarchy_generated_face_without_parent() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-old-face-{suffix}"));
    let source = root.join("src/test/test.rs");
    std::fs::create_dir_all(source.parent().expect("face parent")).expect("create fixture");
    std::fs::write(
        &source,
        "// generated-by=NichLink\npub struct Test;\ncrate::control_object! { kind: Test, }\n",
    )
    .expect("write old generated face");
    select_project(root.clone(), root.join("Cargo.toml"), "old-face-test");

    let app = App::load();

    assert!(app.reload_error.is_none(), "{}", app.event);
    assert_eq!(app.registry.depth_first().len(), 1);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn editing_module_name_moves_the_face_and_keeps_generated_source_compact() {
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!("nichlink-studio-rename-{suffix}"));
    std::fs::create_dir_all(root.join("src")).expect("create source root");
    select_project(root.clone(), root.join("Cargo.toml"), "rename-test");

    let mut app = App::load();
    let root_id = app.registry.id();
    let mut add = AddState::new(root_id);
    add.values[1] = "test".to_owned();
    add.values[2] = "true".to_owned();
    add.values[8] = "Test".to_owned();
    app.submit_add(&add);
    assert!(!app.event.starts_with("Add failed"), "{}", app.event);
    let old_source = root.join("src/test/test.rs");
    let initial = std::fs::read_to_string(&old_source).expect("new face source");
    assert!(initial.contains("crate::root_object! {\n    kind: Test,"));
    assert!(initial.contains("crate::root_object!"));
    assert!(!initial.contains("preset:"));
    assert!(!initial.contains("parts:"));
    assert!(!initial.contains("exports:"));
    assert!(!initial.contains("name:"));
    assert!(!initial.contains("registry_name:"));
    assert!(initial.contains("parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\"))"));

    let id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "test")
        .expect("new face in registry")
        .id;
    let mut child = AddState::new(id);
    child.values[1] = "child".to_owned();
    child.values[2] = "true".to_owned();
    child.values[8] = "Child".to_owned();
    app.submit_add(&child);
    assert!(!app.event.starts_with("Add failed"), "{}", app.event);
    let child_id = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "child")
        .expect("child registry in tree")
        .id;
    let mut leaf = AddState::new(child_id);
    leaf.values[1] = "leaf".to_owned();
    leaf.values[8] = "Leaf".to_owned();
    app.submit_add(&leaf);
    assert!(!app.event.starts_with("Add failed"), "{}", app.event);
    assert!(
        std::fs::read_to_string(root.join("src/test/object/child/object/leaf/leaf.rs"))
            .expect("three-level child source")
            .contains("crate::child_object!")
    );

    let mut edit = AddState::new(root_id);
    edit.values[1] = "panel".to_owned();
    edit.values[2] = "true".to_owned();
    edit.values[3] = "test".to_owned();
    edit.values[8] = "Test".to_owned();
    app.submit_edit(id, &edit);

    assert!(!app.event.starts_with("Edit failed"), "{}", app.event);
    assert!(!old_source.exists());
    let new_source = root.join("src/panel/panel.rs");
    assert!(new_source.is_file());
    let new_child = root.join("src/panel/object/child/child.rs");
    assert!(new_child.is_file());
    assert!(
        root.join("src/panel/object/child/object/leaf/leaf.rs")
            .is_file()
    );
    assert!(
        std::fs::read_to_string(&new_child)
            .expect("renamed child source")
            .contains("crate::panel::NODE_ID")
    );
    assert!(
        std::fs::read_to_string(&new_child)
            .expect("renamed child source")
            .contains("crate::panel_object!")
    );
    assert_eq!(
        app.selected_info().map(|info| info.source.file.as_str()),
        Some("panel/panel.rs")
    );
    // Re-opening Edit after reload must derive module from the source path,
    // not from registry_name. This guards against a rename being displayed as
    // the old module and then moved back on the next save.
    // reload 后重新打开编辑表单时，module 必须来自源码路径，而不是
    // registry_name；这样不会把已重命名的模块显示成旧名称并移回去。
    app.handle_key(KeyEvent::from(KeyCode::Char('e')));
    let Some(Overlay::Edit(_, reopened)) = app.overlay.take() else {
        panic!("e should reopen the Edit form after module migration");
    };
    assert_eq!(reopened.values[1], "panel");
    assert_eq!(reopened.values[3], "test");
    // Kind is an identity field too, but it is migrated atomically instead of
    // being silently ignored by the form.
    // kind 同样属于身份字段；它应通过原子迁移生效，而不是被表单静默忽略。
    let migrated_id = app
        .selected_info()
        .expect("renamed face remains selected")
        .id;
    let mut kind_edit = reopened;
    kind_edit.values[8] = "Panel".to_owned();
    app.submit_edit(migrated_id, &kind_edit);
    assert!(!app.event.starts_with("Edit failed"), "{}", app.event);
    assert_eq!(
        app.selected_info().map(|info| info.kind.as_str()),
        Some("Panel")
    );
    let panel_id = app
        .selected_info()
        .expect("migrated face remains selected")
        .id;
    let child_parent = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| info.registry_name == "child")
        .map(|info| info.parent);
    assert_eq!(child_parent, Some(panel_id));
    assert!(
        std::fs::read_to_string(&new_source)
            .expect("renamed face source")
            .contains("kind: Panel")
    );

    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn function_bodies_capture_calls_but_ignore_use_and_registration_text() {
    let bodies = function_bodies(
        "use crate::Thing;\n\nfn outer() { inner(); crate::control_object!(); }\nfn inner() {}",
    );
    assert_eq!(
        bodies
            .iter()
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        ["outer", "inner"]
    );
    assert!(bodies[0].1.contains("inner()"));
    assert!(!body_calls("control_object!(inner());", "inner"));
    assert!(!body_calls("// inner()\nlet text = \"inner()\";", "inner"));
    assert!(body_calls("crate::inner ();", "inner"));
}

#[test]
fn function_index_accepts_qualified_visibility_and_impl_methods() {
    let source = r#"
            // fn ignored() {}
            pub(crate) async fn load(value: usize)
            where
                usize: Copy,
            {
                self.render::<usize>(value);
            }

            impl Widget {
                unsafe fn render(&self, value: usize) {
                    Type::paint(value);
                }
            }
        "#;
    let functions = function_symbols(source);
    assert_eq!(
        functions
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        ["load", "render"]
    );
    assert!(functions[0].signature.contains("pub(crate) async fn load"));
    assert!(functions[0].body.contains("self.render::<usize>(value)"));
    assert!(body_calls(&functions[0].body, "render"));
    assert!(body_calls(&functions[0].body, "render"));
    assert!(body_calls(&functions[1].body, "paint"));
    assert!(!body_calls(&functions[1].body, "load"));
}

#[test]
fn source_preview_range_is_limited_to_the_selected_function() {
    let source = [
        "fn first() {",
        "    one();",
        "}",
        "",
        "pub(crate) fn second() {",
        "    two();",
        "}",
    ];
    let range = app_function_source_range(&source, "second").expect("function range");
    assert_eq!(range, (4, 6));
}

#[test]
fn call_scanner_ignores_use_and_macro_but_accepts_qualified_calls() {
    let body = r#"
            use crate::paint;
            control_object!(paint());
            self.paint::<Color>();
            Widget::layout();
            // paint()
            let text = "layout()";
        "#;
    assert!(body_calls(body, "paint"));
    assert!(body_calls(body, "layout"));
    assert!(!body_calls("use crate::paint;", "paint"));
    assert!(!body_calls("control_object!(paint());", "paint"));
    assert!(!body_calls("let text = \"paint()\";", "paint"));
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn call_relations_report_real_cross_file_function_calls() {
    let app = App::load();
    let node_editor = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| {
            info.registry_name == "node_editor" && info.source.file.ends_with("node_editor.rs")
        })
        .expect("NodeEditor face");
    let (callers, callees) = app.call_relations(node_editor.id, "preview_canvas_width");
    assert!(
        callers
            .iter()
            .any(|item| item.function == "preview_canvas_width_traced")
    );
    assert!(
        callees
            .iter()
            .any(|item| item.function == "clamp_canvas_width")
    );
    let canvas = callees
        .iter()
        .find(|item| item.function == "clamp_canvas_width")
        .expect("Canvas call");
    let (reverse_callers, _) = app.call_relations(canvas.node, "clamp_canvas_width");
    assert!(
        reverse_callers
            .iter()
            .any(|item| item.function == "preview_canvas_width")
    );
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn mir_candidates_are_optional_and_keep_live_evidence_distinct() {
    let mut app = App::load();
    app.mir_graph = Some(
            MirGraph::from_jsonl(
                "{\"kind\":\"call\",\"caller\":\"preview_canvas_width\",\"callee\":\"clamp_canvas_width\",\"mir_line\":7}\n",
            )
            .unwrap(),
        );
    let node_editor = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| {
            info.registry_name == "node_editor" && info.source.file.ends_with("node_editor.rs")
        })
        .expect("NodeEditor face");
    let (_, callees) = app.call_relations(node_editor.id, "preview_canvas_width");
    let canvas = callees
        .iter()
        .find(|item| item.function == "clamp_canvas_width")
        .expect("MIR candidate should resolve to a known function");
    let center = CallRef {
        node: node_editor.id,
        function: "preview_canvas_width".to_owned(),
        file: node_editor.source.file.to_owned(),
    };
    assert_eq!(
        app.call_evidence(&center, canvas),
        super::CallEvidence::Live
    );
    assert_eq!(app.mir_candidates_for("preview_canvas_width").len(), 1);
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn graph_navigation_moves_across_real_call_edges() {
    let mut app = App::load();
    app.overlay = Some(Overlay::Search(SearchState {
        query: "preview_canvas_width".to_owned(),
        ..SearchState::default()
    }));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Enter,
    ));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("search graph should open");
    };
    assert!(search.graph_mode);
    let before = search.graph_selected;
    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Down,
    ));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("search graph should remain open");
    };
    assert!(search.graph_selected > before);
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn graph_tab_reaches_both_tree_and_data_panels() {
    let mut app = App::load();
    app.overlay = Some(Overlay::Search(SearchState {
        query: "preview_canvas_width".to_owned(),
        ..SearchState::default()
    }));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Enter));
    let Some(Overlay::Search(search)) = app.overlay.as_mut() else {
        panic!("graph overlay should stay open");
    };
    search.compare_query = Some("preview_canvas_width_traced".to_owned());
    search.compare_center = search.center;
    search.compare_center_function = search.center_function.clone();

    for expected in [(1, 1), (2, 0), (2, 1), (3, 0), (3, 1)] {
        app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Tab));
        let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
            panic!("graph overlay should stay open");
        };
        assert_eq!((search.graph_focus, search.graph_side), expected);
    }

    app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Left));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Down));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("graph overlay should stay open");
    };
    assert_eq!((search.graph_focus, search.graph_side), (3, 0));
    assert_eq!(search.data_selected, 1);
}

#[test]
fn graph_tab_skips_absent_comparison_panels() {
    let mut search = SearchState {
        graph_mode: true,
        ..SearchState::default()
    };

    advance_graph_focus(&mut search);
    assert_eq!((search.graph_focus, search.graph_side), (2, 0));
    advance_graph_focus(&mut search);
    assert_eq!((search.graph_focus, search.graph_side), (3, 0));
    advance_graph_focus(&mut search);
    assert_eq!((search.graph_focus, search.graph_side), (0, 0));
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn graph_enter_promotes_callers_and_opens_center_source() {
    let mut app = App::load();
    app.overlay = Some(Overlay::Search(SearchState {
        query: "preview_canvas_width".to_owned(),
        ..SearchState::default()
    }));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Enter,
    ));

    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Down,
    ));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Enter,
    ));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("graph should stay open after promoting a callee");
    };
    assert_eq!(
        search.center_function.as_deref(),
        Some("clamp_canvas_width")
    );

    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Enter,
    ));
    assert!(app.overlay.is_none());
    let Some((path, line)) = app.take_editor_request() else {
        panic!("center Enter should request an editor");
    };
    assert!(path.is_file());
    assert!(path.ends_with("control/object/node_editor/object/object.rs"));
    assert!(line > 1);
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn call_tree_targets_keep_rows_navigable() {
    let app = App::load();
    let node_editor = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| {
            info.registry_name == "node_editor" && info.source.file.ends_with("node_editor.rs")
        })
        .expect("NodeEditor face");
    let center = CallRef {
        node: node_editor.id,
        function: "preview_canvas_width".to_owned(),
        file: node_editor.source.file.to_owned(),
    };
    let targets = app.call_tree_targets(&center);
    assert_eq!(
        targets
            .first()
            .and_then(Option::as_ref)
            .map(|item| item.function.as_str()),
        Some("preview_canvas_width")
    );
    assert!(
        targets
            .iter()
            .flatten()
            .any(|item| item.function == "clamp_canvas_width")
    );
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn directory_style_source_paths_resolve_to_attached_files() {
    let path = source_path_for("control/object/node_editor/object/");
    assert!(path.is_file());
    assert!(path.ends_with("control/object/node_editor/object/object.rs"));
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn searching_a_file_adds_compact_source_symbols() {
    let app = App::load();
    let rows = app.search_rows(
        "control/object/node_editor/node_editor.rs",
        &BTreeSet::new(),
    );
    assert!(rows.iter().any(|row| row.function == "NodeEditor"));
    assert!(rows.iter().all(|row| !row.text.contains("declared-at=")));
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn searching_a_function_name_finds_the_source_symbol() {
    let app = App::load();
    let rows = app.search_rows("accept_canvas", &BTreeSet::new());
    assert!(rows.iter().any(|row| row.function == "accept_canvas"));
    assert!(
        rows.iter()
            .any(|row| row.signature.contains("accept_canvas"))
    );
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn searching_a_parameter_name_finds_its_function() {
    let app = App::load();
    let rows = app.search_rows("canvas_name", &BTreeSet::new());
    assert!(rows.iter().any(|row| row.function == "accept_canvas"));
}
