//! Add/edit form and registration-face field tests.
//! 新增/编辑表单与注册面字段测试。

use super::*;

#[test]
fn add_overlay_click_selects_a_field_and_toggles_the_checkbox() {
    let mut app = App::load();
    app.overlay = Some(Overlay::Add(AddState::new(app.selected_parent())));
    app.hot.overlay_area = Rect::new(5, 5, 70, 18);
    app.hot.overlay_list_area = Rect::new(7, 7, 66, 10);

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
    let before = super::super::face_field_indices(initial);

    for _ in 0..8 {
        app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
    }

    let Some(Overlay::Add(current)) = app.overlay.as_ref() else {
        panic!("add overlay should remain open");
    };
    let after = super::super::face_field_indices(current);
    assert_eq!(before, after, "moving to kind changed the form layout");
    assert_eq!(after.len(), nichlink_run_method::FACE_FIELD_COUNT);
}

#[test]
fn add_form_starts_with_editable_bilingual_summary() {
    let add = AddState::new(nichlink_run_method::ROOT_NODE_ID);
    assert!(add.values[9].is_empty());
    assert!(add.values[10].is_empty());
    assert!(add.values[11].is_empty());
    assert!(add.values[12].is_empty());
    assert!(add.values[29].is_empty());
}

#[test]
fn parent_rule_marks_the_fields_a_child_must_supply() {
    let mut add = AddState::new(nichlink_run_method::ROOT_NODE_ID);
    add.apply_parent_rule(&nichlink_run_method::OwnedRegistrationRule {
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
        super::super::keyboard::declaration_contract_paths(source),
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
