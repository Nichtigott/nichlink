//! Call-graph navigation and panel focus tests.
//! 调用图导航与面板焦点测试。

// Every test here needs the fixture project, so the prelude is imported with the
// feature that mounts them: without it this file has no tests and the import
// would be unused.
// 这里的每个测试都需要夹具项目，因此前导与挂载它们的特性一起导入：没有该特性时本文件没有
// 测试，那个导入就成了未使用。
#[cfg(feature = "prototype-fixtures")]
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
    let (before, view) = {
        let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
            panic!("search graph should open");
        };
        assert!(search.graph_mode);
        assert_eq!(search.graph_focus, 0, "the tree holds the focus on entry");
        let center = app
            .graph_item(search)
            .expect("the cursor stands on a function");
        (search.outline_selected, app.call_tree_view(&center))
    };
    // Draw once before navigating: that is what tells the arrows which model axis
    // runs down the screen, and the panel is the authority on it. The key that
    // means "down the screen" must therefore reach a callee in either drawer.
    // 导航前先绘制一次：正是它告诉方向键哪个模型轴沿屏幕向下，而面板是这件事的权威。因此
    // "沿屏幕向下"的那个键在两种绘制方里都必须到达被调用者。
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 48)).expect("a terminal");
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut app))
        .expect("the graph page draws");
    let down_the_screen = if app.tree_top_down {
        crossterm::event::KeyCode::Down
    } else {
        crossterm::event::KeyCode::Right
    };
    app.handle_overlay_key(crossterm::event::KeyEvent::from(down_the_screen));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("search graph should remain open");
    };
    let landed = search.outline_selected;
    assert_ne!(landed, before, "the arrow moves the cursor");
    assert!(
        view.tree.nodes[landed].level > 0,
        "down the screen reaches a callee: {}",
        view.tree.nodes[landed].symbol
    );
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
    // Two panes: Tab moves the tree → the values, and wraps.
    for expected in [1, 0] {
        app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Tab));
        let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
            panic!("graph overlay should stay open");
        };
        assert_eq!(search.graph_focus, expected);
        assert_eq!(search.outline_focus, expected == 0);
    }

    // The value pane is where the data cursor moves; the tree's cursor is
    // untouched while it has the focus.
    app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Tab));
    app.handle_overlay_key(crossterm::event::KeyEvent::from(KeyCode::Down));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("graph overlay should stay open");
    };
    assert_eq!(search.graph_focus, 1);
    assert_eq!(search.data_selected, 1);
    assert_eq!(search.outline_selected, 0);
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

    // → hops downstream to a callee, and Enter makes that callee the centre.
    // → 跳向下游的被调用者，Enter 让那个被调用者成为新的圆心。
    app.handle_overlay_key(crossterm::event::KeyEvent::from(
        crossterm::event::KeyCode::Right,
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
