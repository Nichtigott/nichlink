//! Editing round-trip regression tests.
//! 编辑往返回归测试。

use super::*;

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
    app.hot.overlay_area = Rect::new(5, 5, 70, 18);
    app.hot.action_confirm_area = Rect::new(10, 18, 14, 3);
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
