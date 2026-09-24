//! Add/edit form and registration-face field tests.
//! 新增/编辑表单与注册面字段测试。

use super::*;
use nichlink_run_method::face_field;

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
    assert_eq!(add.values[face_field::NEEDS_REGISTRY], "true");
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
    assert_eq!(add.field, face_field::KIND);
}

#[test]
fn add_navigation_walks_one_fixed_field_order() {
    // The layout used to be rebuilt from the focused field, which is why this
    // test watched for the visible set changing. It is a constant now, so the
    // claim worth pinning is the other direction: every Down press lands on the
    // next entry of that one order, and no slot is skipped or repeated.
    // 布局过去会按聚焦字段重建，这正是本测试当初盯住可见集合变化的原因。现在它是常量，
    // 因此值得钉住的是另一个方向：每次 Down 落在该顺序的下一个条目上，没有槽位被跳过或重复。
    let mut app = App::load();
    app.overlay = Some(Overlay::Add(AddState::new(app.selected_parent())));
    let order = super::super::face_field_indices();
    let mut visited = Vec::new();
    for step in 0..9 {
        if step > 0 {
            app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
        }
        let Some(Overlay::Add(current)) = app.overlay.as_ref() else {
            panic!("add overlay should remain open");
        };
        visited.push(current.field);
    }
    assert_eq!(visited, order[..9], "navigation skipped a form row");
    assert_eq!(order.len(), nichlink_run_method::FACE_FIELD_COUNT);
}

#[test]
fn add_form_starts_with_editable_bilingual_summary() {
    let add = AddState::new(nichlink_run_method::ROOT_NODE_ID);
    assert!(add.values[face_field::NAME_ZH].is_empty());
    assert!(add.values[face_field::NAME_EN].is_empty());
    assert!(add.values[face_field::SUMMARY_ZH].is_empty());
    assert!(add.values[face_field::SUMMARY_EN].is_empty());
    assert!(add.values[face_field::PART_CONTRACTS].is_empty());
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
        [
            face_field::PARTS,
            face_field::EXPORTS,
            face_field::PRESET,
            face_field::HANDLE_CONTRACTS,
            face_field::PART_CONTRACTS,
        ]
    );
    assert_eq!(
        add.parent_requirements
            .get(&face_field::HANDLE_CONTRACTS)
            .map(String::as_str),
        Some("handle implements ControlHandle")
    );
    assert_eq!(add.values[face_field::PRESET], "ActionParts");
    assert_eq!(add.values[face_field::EXPORTS], "control.render");
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
    add.field = face_field::STABLE_NAME;
    app.overlay = Some(Overlay::Add(add));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));

    assert!(matches!(app.overlay, Some(Overlay::Add(ref add)) if add.editing));
}

#[test]
fn derived_storage_fields_do_not_open_a_fake_editor() {
    let mut app = App::load();
    let mut add = AddState::new(app.selected_parent());
    add.field = face_field::REGISTRY_RULE_PATH;
    app.overlay = Some(Overlay::Add(add));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));

    assert!(matches!(app.overlay, Some(Overlay::Add(ref add)) if !add.editing));
}
