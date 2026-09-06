//! App interaction regression tests.
//! App 交互回归测试。

use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEventKind};
use ratatui::layout::Rect;

use super::support::{host_manifest, package_root};
use super::{
    advance_graph_focus, app_function_source_range, body_calls, function_bodies, function_symbols,
    visible_search_rows, AddState, App, Overlay, SearchState, StudioPage,
};

static PROJECT_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

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

    app.handle_overlay_click(12, 10);

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
    for _ in 0..8 {
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
fn add_form_starts_with_editable_bilingual_summary() {
    let add = AddState::new(nichlink::ROOT_NODE_ID);
    assert!(add.values[9].is_empty());
    assert!(add.values[10].is_empty());
    assert_eq!(add.values[11], "NichLink 创建的注册模块。");
    assert_eq!(add.values[12], "A registration module created by NichLink.");
}

#[test]
fn standalone_studio_resolves_one_project_root_and_manifest() {
    let _guard = PROJECT_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let root = package_root();
    assert!(root.is_dir());
    assert_eq!(
        host_manifest().file_name().and_then(|name| name.to_str()),
        Some("Cargo.toml")
    );
}

#[test]
fn new_project_and_explicit_root_face_compile() {
    let _guard = PROJECT_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let original_root = std::env::var_os("NICH_LINK_PACKAGE_ROOT");
    let original_manifest = std::env::var_os("NICH_LINK_HOST_MANIFEST");
    let original_namespace = std::env::var_os("NICH_LINK_NAMESPACE");
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
    assert!(root
        .join("src/workspace/registry_rule/registry_rule.rs")
        .is_file());
    let workspace =
        std::fs::read_to_string(root.join("src/workspace/workspace.rs")).expect("workspace face");
    assert!(workspace.starts_with("// generated-by=NichLink"));

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
    assert!(root
        .join("src/workspace/object/panel/registry_rule/registry_rule.rs")
        .is_file());
    let panel = std::fs::read_to_string(root.join("src/workspace/object/panel/panel.rs"))
        .expect("panel face");
    assert!(panel.contains("crate::workspace::object::panel::registry_rule::REGISTRATION_RULE"));

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
    match original_root {
        Some(value) => std::env::set_var("NICH_LINK_PACKAGE_ROOT", value),
        None => std::env::remove_var("NICH_LINK_PACKAGE_ROOT"),
    }
    match original_manifest {
        Some(value) => std::env::set_var("NICH_LINK_HOST_MANIFEST", value),
        None => std::env::remove_var("NICH_LINK_HOST_MANIFEST"),
    }
    match original_namespace {
        Some(value) => std::env::set_var("NICH_LINK_NAMESPACE", value),
        None => std::env::remove_var("NICH_LINK_NAMESPACE"),
    }
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
    let _guard = PROJECT_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let original_root = std::env::var_os("NICH_LINK_PACKAGE_ROOT");
    let original_manifest = std::env::var_os("NICH_LINK_HOST_MANIFEST");
    let original_namespace = std::env::var_os("NICH_LINK_NAMESPACE");
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
    match original_root {
        Some(value) => std::env::set_var("NICH_LINK_PACKAGE_ROOT", value),
        None => std::env::remove_var("NICH_LINK_PACKAGE_ROOT"),
    }
    match original_manifest {
        Some(value) => std::env::set_var("NICH_LINK_HOST_MANIFEST", value),
        None => std::env::remove_var("NICH_LINK_HOST_MANIFEST"),
    }
    match original_namespace {
        Some(value) => std::env::set_var("NICH_LINK_NAMESPACE", value),
        None => std::env::remove_var("NICH_LINK_NAMESPACE"),
    }
}

#[test]
fn new_project_starts_with_an_empty_registration_tree() {
    let _guard = PROJECT_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let original_root = std::env::var_os("NICH_LINK_PACKAGE_ROOT");
    let original_manifest = std::env::var_os("NICH_LINK_HOST_MANIFEST");
    let original_namespace = std::env::var_os("NICH_LINK_NAMESPACE");
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
    match original_root {
        Some(value) => std::env::set_var("NICH_LINK_PACKAGE_ROOT", value),
        None => std::env::remove_var("NICH_LINK_PACKAGE_ROOT"),
    }
    match original_manifest {
        Some(value) => std::env::set_var("NICH_LINK_HOST_MANIFEST", value),
        None => std::env::remove_var("NICH_LINK_HOST_MANIFEST"),
    }
    match original_namespace {
        Some(value) => std::env::set_var("NICH_LINK_NAMESPACE", value),
        None => std::env::remove_var("NICH_LINK_NAMESPACE"),
    }
}

#[test]
fn edit_save_button_writes_changes_and_adopts_the_old_control_scaffold() {
    let _guard = PROJECT_ENV_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner());
    let original_root = std::env::var_os("NICH_LINK_PACKAGE_ROOT");
    let original_manifest = std::env::var_os("NICH_LINK_HOST_MANIFEST");
    let original_namespace = std::env::var_os("NICH_LINK_NAMESPACE");
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
    std::env::set_var("NICH_LINK_PACKAGE_ROOT", &root);
    std::env::set_var("NICH_LINK_HOST_MANIFEST", root.join("Cargo.toml"));
    std::env::set_var("NICH_LINK_NAMESPACE", "legacy-app");

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
    match original_root {
        Some(value) => std::env::set_var("NICH_LINK_PACKAGE_ROOT", value),
        None => std::env::remove_var("NICH_LINK_PACKAGE_ROOT"),
    }
    match original_manifest {
        Some(value) => std::env::set_var("NICH_LINK_HOST_MANIFEST", value),
        None => std::env::remove_var("NICH_LINK_HOST_MANIFEST"),
    }
    match original_namespace {
        Some(value) => std::env::set_var("NICH_LINK_NAMESPACE", value),
        None => std::env::remove_var("NICH_LINK_NAMESPACE"),
    }
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
    assert!(callers
        .iter()
        .any(|item| item.function == "preview_canvas_width_traced"));
    assert!(callees
        .iter()
        .any(|item| item.function == "clamp_canvas_width"));
    let canvas = callees
        .iter()
        .find(|item| item.function == "clamp_canvas_width")
        .expect("Canvas call");
    let (reverse_callers, _) = app.call_relations(canvas.node, "clamp_canvas_width");
    assert!(reverse_callers
        .iter()
        .any(|item| item.function == "preview_canvas_width"));
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
    assert!(targets
        .iter()
        .flatten()
        .any(|item| item.function == "clamp_canvas_width"));
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
    assert!(rows
        .iter()
        .any(|row| row.signature.contains("accept_canvas")));
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn searching_a_parameter_name_finds_its_function() {
    let app = App::load();
    let rows = app.search_rows("canvas_name", &BTreeSet::new());
    assert!(rows.iter().any(|row| row.function == "accept_canvas"));
}
