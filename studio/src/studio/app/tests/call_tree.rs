//! Spatial call-tree regression tests.
//! 空间调用树回归测试。
//!
//! These run against the node-editor fixture for the same reason the call-graph
//! tests do: the rules under test are about a *shape* — the focus owns column
//! zero, callers fill the left, callees the right, the cursor moves along the
//! axis — and a shape needs a call graph whose edges are already pinned.
//! 这些测试与调用图测试一样跑在 node-editor 夹具上：被测规则关于一种*形状*——焦点独占第零列、
//! 调用者填左、被调用者填右、游标沿轴移动——而形状需要一个调用边已被钉住的调用图。
//!
//! Mounted only with the `prototype-fixtures` feature, so the default build
//! carries neither these tests nor their fixture selection.
//! 仅在启用 `prototype-fixtures` 特性时挂载，因此默认构建既不携带这些测试，也不携带它们的
//! 夹具选择。

use super::*;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// The node-editor fixture, whose call graph the graph tests already pin.
/// node-editor 夹具，它的调用图已被调用图测试钉住。
fn fixture_app() -> App {
    let fixture =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/node-editor");
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
    App::load()
}

/// The registered functions of the loaded project, one per name.
/// 当前载入项目中已注册的函数，每个名字取一个。
fn registered_functions(app: &App, limit: usize) -> Vec<CallRef> {
    let mut functions: Vec<CallRef> = Vec::new();
    for info in app.registry.depth_first() {
        if info.source.function.is_empty()
            || functions
                .iter()
                .any(|existing: &CallRef| same_symbol(&existing.function, &info.source.function))
        {
            continue;
        }
        functions.push(CallRef {
            node: info.id,
            function: info.source.function.clone(),
            file: info.source.file.clone(),
        });
        if functions.len() >= limit {
            break;
        }
    }
    functions
}

/// Open the provenance graph on one registered function and hand back its state.
/// 在某个已注册函数上打开溯源图，并交回它的状态。
fn open_graph(app: &mut App, function: &str) -> SearchState {
    let rows = app.search_rows(function);
    let selected = rows
        .iter()
        .position(|row| row.function == function && row.node.is_some())
        .unwrap_or_else(|| panic!("{function} is not a registered function"));
    app.overlay = Some(Overlay::Search(SearchState {
        query: function.to_owned(),
        selected,
        ..SearchState::default()
    }));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    let Some(Overlay::Search(opened)) = app.overlay.as_ref() else {
        panic!("Enter on a call row opens the graph");
    };
    opened.clone()
}

/// The tree around one function of the loaded project.
/// 当前载入项目中某个函数周围的调用树。
fn view_around(app: &App, function: &str) -> CallTreeView {
    let rows = app.search_rows(function);
    let row = rows
        .iter()
        .find(|row| row.function == function && row.node.is_some())
        .unwrap_or_else(|| panic!("{function} is not a registered function"));
    app.call_tree_view(&CallRef {
        node: row.node.expect("the row carries a node"),
        function: row.function.clone(),
        file: row.path.clone(),
    })
}

#[test]
fn every_shape_the_tree_promises_holds_around_a_real_focus() {
    let app = fixture_app();
    let functions = registered_functions(&app, 3);
    assert!(
        functions.len() >= 2,
        "the fixture registers several functions to focus on"
    );
    for focus in &functions {
        let view = app.call_tree_view(focus);
        assert_eq!(view.len(), view.refs.len(), "refs and nodes stay aligned");
        assert!(
            same_symbol(&view.tree.nodes[0].symbol, &focus.function),
            "{} owns row zero",
            focus.function
        );
        let mut seen: Vec<(i32, usize)> = Vec::new();
        for (index, node) in view.tree.nodes.iter().enumerate() {
            if index == 0 {
                assert_eq!(node.parent, None, "the focus is placed by nobody");
            } else {
                let parent = &view.tree.nodes[node.parent.expect("a placed node has a parent")];
                assert_eq!(
                    node.level.unsigned_abs(),
                    parent.level.unsigned_abs() + 1,
                    "{} hangs exactly one hop off {}",
                    node.symbol,
                    parent.symbol
                );
                assert!(
                    parent.level == 0 || (parent.level > 0) == (node.level > 0),
                    "{} stays on {}'s side of the focus",
                    node.symbol,
                    parent.symbol
                );
            }
            let cell = (node.level, node.lane);
            assert!(
                !seen.contains(&cell),
                "column {cell:?} holds two nodes, so one would be drawn on the other"
            );
            seen.push(cell);
        }
        for edge in &view.tree.edges {
            let caller = view.tree.nodes[edge.caller].level;
            let callee = view.tree.nodes[edge.callee].level;
            assert_eq!(
                edge.forward,
                callee > caller,
                "forward must mean left to right: {caller} → {callee}"
            );
        }
    }
}

#[test]
fn the_memo_answers_the_same_focus_and_a_different_one() {
    let app = fixture_app();
    let functions = registered_functions(&app, 2);
    let focus = functions.first().expect("a focus to build around").clone();
    let first = app.call_tree_view(&focus);
    let second = app.call_tree_view(&focus);
    assert_eq!(first.tree, second.tree, "the memo returns the same tree");
    let other = app.call_tree_view(functions.get(1).expect("a second focus"));
    assert!(
        !same_symbol(&other.tree.nodes[0].symbol, &first.tree.nodes[0].symbol),
        "a different focus must not be served from the first focus's entry"
    );
}

#[test]
fn arrows_hop_along_the_axis_without_switching_sides() {
    let mut app = fixture_app();
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 2;
    search.outline_selected = 0;
    search.graph_side = 0;
    app.overlay = Some(Overlay::Search(search));
    let view = view_around(&app, "preview_canvas_width");
    let has_downstream = view.tree.nodes.iter().any(|node| node.level > 0);
    let has_upstream = view.tree.nodes.iter().any(|node| node.level < 0);

    app.handle_overlay_key(KeyEvent::from(KeyCode::Right));
    let Some(Overlay::Search(after_right)) = app.overlay.as_ref() else {
        panic!("the graph stays open while hopping");
    };
    assert_eq!(
        after_right.graph_side, 0,
        "the arrow hops, it does not switch side"
    );
    if has_downstream {
        let landed = after_right.outline_selected;
        assert!(landed > 0, "→ leaves the focus for a downstream node");
        assert!(
            view.tree.nodes[landed].level > 0,
            "→ landed upstream: {}",
            view.tree.nodes[landed].symbol
        );
        app.handle_overlay_key(KeyEvent::from(KeyCode::Left));
        let Some(Overlay::Search(after_left)) = app.overlay.as_ref() else {
            panic!("the graph stays open while hopping");
        };
        assert_eq!(after_left.outline_selected, 0, "← walks back to the focus");
    } else {
        assert!(
            app.event.contains("no downstream hop"),
            "an honest refusal: {}",
            app.event
        );
    }

    app.handle_overlay_key(KeyEvent::from(KeyCode::Left));
    let Some(Overlay::Search(upstream)) = app.overlay.as_ref() else {
        panic!("the graph stays open while hopping");
    };
    if has_upstream {
        let landed = upstream.outline_selected;
        assert!(landed > 0, "← leaves the focus for an upstream node");
        assert!(
            view.tree.nodes[landed].level < 0,
            "← landed downstream: {}",
            view.tree.nodes[landed].symbol
        );
    } else {
        assert!(
            app.event.contains("no upstream hop"),
            "an honest refusal: {}",
            app.event
        );
    }
}

#[test]
fn enter_re_centres_on_a_tree_node_and_opens_the_editor_only_at_the_focus() {
    let mut app = fixture_app();
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 2;
    search.outline_selected = 1;
    app.overlay = Some(Overlay::Search(search));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    let Some(Overlay::Search(recentred)) = app.overlay.as_ref() else {
        panic!("Enter on a tree node re-centres instead of closing the graph");
    };
    assert_eq!(recentred.outline_selected, 0, "the new focus is row zero");
    assert!(
        recentred.center_function.as_deref() != Some("preview_canvas_width"),
        "the graph moved to another function"
    );
    assert!(app.event.contains("re-centred"), "{}", app.event);

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    assert!(
        app.overlay.is_none(),
        "Enter at the focus itself opens the editor"
    );
}

#[test]
fn the_drawn_tree_states_its_axis_and_keeps_the_focus_in_view() {
    let mut app = fixture_app();
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 2;
    app.overlay = Some(Overlay::Search(search));
    let mut terminal = Terminal::new(TestBackend::new(160, 48)).expect("a test terminal");
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut app))
        .expect("the graph page draws");
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        output.contains("call direction"),
        "the panel must name the axis it draws: {output}"
    );
    assert!(
        output.contains("← callers · call direction · callees →"),
        "the axis sentence must survive the panel width: {output}"
    );
    assert!(
        output.contains("depth "),
        "the header reports the hop budget: {output}"
    );
    assert!(
        output.contains("focus"),
        "the level ruler labels column zero: {output}"
    );
    assert!(
        output.contains("preview_canvas_width"),
        "the focus itself is on screen: {output}"
    );
}
