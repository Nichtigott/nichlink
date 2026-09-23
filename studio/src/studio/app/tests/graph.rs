//! Call-graph navigation and panel focus tests.
//! 调用图导航与面板焦点测试。

use super::*;

#[test]
#[cfg(feature = "prototype-fixtures")]
fn graph_navigation_moves_across_real_call_edges() {
    let fixture =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/node-editor");
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
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
    let fixture =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/node-editor");
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
    let mut app = App::load();
    // The DATA panel lists the trace's locals for the selected tree row, and tree
    // cursor 0 is the center function itself, so this test installs a trace with
    // locals inside `preview_canvas_width`. The standalone demo sample has no
    // locals for any host project, which is why this panel used to be unreachable.
    // DATA 面板列出选中树行对应的追踪局部值，而树游标 0 就是中心函数本身，因此本条测试
    // 安装一条在 `preview_canvas_width` 帧内带局部值的追踪。独立演示样本对任何宿主项目
    // 都没有局部值，这正是该面板此前不可达的原因。
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
    let callee = app
        .call_relations(center.node, &center.function)
        .1
        .into_iter()
        .find(|item| item.function == "clamp_canvas_width")
        .expect("the fixture's preview_canvas_width calls clamp_canvas_width");
    app.runtime_trace = super::fixtures::fixture_live_trace(&center, &callee);
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
    let fixture =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/node-editor");
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
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
    let fixture =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/node-editor");
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
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
