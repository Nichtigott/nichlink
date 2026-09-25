//! What the shipped Studio trace can confirm, observed instead of argued.
//! 出厂 Studio 追踪实际能确认什么：观察，而不是论证。

use super::*;

/// Every caller-to-callee edge the graph page could draw for the loaded project.
/// 已加载工程里调用图页面可能画出的每一条调用者→被调用者边。
///
/// The enumeration follows the page's own definition of an edge: for each
/// function the source scan sees, ask the app for its callers and callees. That
/// is the same `call_relations` the tree and the detail pane read, so a
/// classification over this list covers every edge they could render.
/// 枚举沿用页面自己对边的定义：对源码扫描看到的每个函数，向 app 问它的调用者与被调用者。
/// 这与树和详情面板读的是同一个 `call_relations`，因此对这份列表分级就覆盖了它们可能渲染的
/// 每一条边。
fn drawn_edges(app: &App) -> Vec<(CallRef, CallRef)> {
    let mut edges = Vec::new();
    for info in app.registry.depth_first() {
        let Ok(text) = std::fs::read_to_string(source_path_for(&info.source.file)) else {
            continue;
        };
        for symbol in function_symbols(&text) {
            let center = CallRef {
                node: info.id,
                function: symbol.name.clone(),
                file: info.source.file.clone(),
            };
            let (callers, callees) = app.call_relations(info.id, &symbol.name);
            for caller in callers {
                edges.push((caller, center.clone()));
            }
            for callee in callees {
                edges.push((center.clone(), callee));
            }
        }
    }
    edges
}

/// The drawn edges that a live observation confirmed, as function-name pairs.
/// 被实时观察确认的已画出边，以函数名对表示。
///
/// `drawn_edges` reaches every edge from both endpoints — once as the caller's
/// callee and once as the callee's caller — so the set below is what collapses
/// those two sightings into the one relationship they describe.
/// `drawn_edges` 会从两个端点各到达每条边一次——一次作为调用方的被调用者，一次作为被调用方
/// 的调用者——下面这个集合正是把这两次目击收敛为它们所描述的那一条关系。
fn confirmed_edges(app: &App) -> Vec<(String, String)> {
    let mut confirmed = std::collections::BTreeSet::new();
    for (caller, callee) in drawn_edges(app) {
        if app.call_evidence(&caller, &callee) == CallEvidence::Live {
            confirmed.insert((caller.function, callee.function));
        }
    }
    confirmed.into_iter().collect()
}

/// The fixture project, loaded the way the graph tests load it.
/// 夹具工程，按调用图测试的方式加载。
fn load_fixture() -> Option<App> {
    let fixture = node_editor_fixture()?;
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
    Some(App::load())
}

/// `U5` in `docs/audit-production-readiness.md` recorded an argument — the
/// shipped sample's nodes cannot come from the same provenance as a real
/// project's, so `CallEvidence::Live` was said to be unreachable — and never an
/// observation. This is the observation, and it is not the trivial "no trace, no
/// live edge": the sample *is* installed, and it still confirms nothing.
/// `docs/audit-production-readiness.md` 里的 `U5` 记下的是一条论证——出厂样本的节点不可能
/// 与真实工程的节点同源，因此称 `CallEvidence::Live` 不可达——而从来不是一次观察。这里就是
/// 那次观察，而且它不是"没有追踪就没有实测边"这种废话：样本**确实**装上了，却仍然什么都没确认。
#[test]
fn the_shipped_trace_confirms_nothing_in_a_real_project() {
    let Some(app) = load_fixture() else {
        return;
    };

    // The premise, asserted rather than assumed: a trace is installed and it does
    // carry an edge.
    // 前提是被断言的而不是被假设的：确实装了一条追踪，而且它确实带一条边。
    let sample_edges = app.runtime_trace.call_edges();
    assert!(
        !sample_edges.is_empty(),
        "the sample must be installed for this observation to mean anything"
    );
    // And that trace belongs to no node of the loaded project, which is *why* it
    // cannot confirm one: identity is the whole test.
    // 而这条追踪不属于已加载工程的任何节点，这正是它无法确认任何边的原因：身份就是全部判据。
    for edge in &sample_edges {
        assert!(
            app.registry.find(edge.caller.node).is_none(),
            "the sample's caller {} is a node of this project after all",
            edge.caller.function
        );
    }

    let drawn = drawn_edges(&app);
    assert!(
        !drawn.is_empty(),
        "the fixture project has a call graph, so this cannot pass vacuously"
    );
    assert_eq!(
        confirmed_edges(&app),
        Vec::<(String, String)>::new(),
        "the shipped sample confirmed an edge of a project it knows nothing about"
    );
}

/// The other half of the observation: the same enumeration does report `Live`
/// once a trace over the loaded faces is installed. Without this, the test above
/// would also pass if `call_evidence` could never answer `Live` at all — which is
/// exactly the confusion `U5` was about.
/// 观察的另一半：一旦装上针对已加载注册面的追踪，同一份枚举就会报出 `Live`。没有这一半，
/// 上面那条测试在 `call_evidence` 根本答不出 `Live` 时也会通过——而 `U5` 问的正是这个区别。
#[test]
fn a_trace_over_the_loaded_faces_confirms_the_edge_it_observed() {
    let Some(mut app) = load_fixture() else {
        return;
    };
    let node_editor = app
        .registry
        .depth_first()
        .into_iter()
        .find(|info| {
            info.registry_name == "node_editor" && info.source.file.ends_with("node_editor.rs")
        })
        .expect("the fixture declares the NodeEditor face");
    let (_, callees) = app.call_relations(node_editor.id, "preview_canvas_width");
    let canvas = callees
        .iter()
        .find(|item| item.function == "clamp_canvas_width")
        .expect("the fixture's source scan finds this call")
        .clone();
    let center = CallRef {
        node: node_editor.id,
        function: "preview_canvas_width".to_owned(),
        file: node_editor.source.file.clone(),
    };
    app.runtime_trace = super::fixtures::fixture_live_trace(&center, &canvas);
    assert_eq!(
        confirmed_edges(&app),
        vec![(
            "preview_canvas_width".to_owned(),
            "clamp_canvas_width".to_owned()
        )],
        "a trace over the loaded faces must confirm exactly the edge it recorded"
    );
}
