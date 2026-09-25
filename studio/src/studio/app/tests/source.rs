//! Source index, call scanning, symbol search, and MIR evidence tests.
//! 源码索引、调用扫描、符号搜索与 MIR 证据测试。

use super::*;

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
    let Some(fixture) = node_editor_fixture() else {
        return;
    };
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
    let Some(fixture) = node_editor_fixture() else {
        return;
    };
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
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
    // Live evidence comes from a trace, MIR from the graph: install the trace
    // that actually observed this call, so the assertion below distinguishes the
    // two instead of comparing against the standalone demo sample.
    // 实测证据来自追踪，MIR 来自图：安装一条真正观测到这次调用的追踪，使下面的断言区分
    // 两者，而不是拿独立演示样本来比较。
    app.runtime_trace = super::fixtures::fixture_live_trace(&center, canvas);
    assert_eq!(app.call_evidence(&center, canvas), CallEvidence::Live);
    assert_eq!(app.mir_candidates_for("preview_canvas_width").len(), 1);
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn directory_style_source_paths_resolve_to_attached_files() {
    let Some(fixture) = node_editor_fixture() else {
        return;
    };
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
    let path = source_path_for("control/object/node_editor/object/");
    assert!(path.is_file());
    assert!(path.ends_with("control/object/node_editor/object/object.rs"));
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn searching_a_file_adds_compact_source_symbols() {
    let Some(fixture) = node_editor_fixture() else {
        return;
    };
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
    let app = App::load();
    let rows = app.search_rows("control/object/node_editor/node_editor.rs");
    assert!(rows.iter().any(|row| row.function == "NodeEditor"));
    assert!(rows.iter().all(|row| !row.text.contains("declared-at=")));
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn searching_a_function_name_finds_the_source_symbol() {
    let Some(fixture) = node_editor_fixture() else {
        return;
    };
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
    let app = App::load();
    let rows = app.search_rows("accept_canvas");
    assert!(rows.iter().any(|row| row.function == "accept_canvas"));
    assert!(
        rows.iter()
            .any(|row| row.signature.contains("accept_canvas"))
    );
}

#[test]
#[cfg(feature = "prototype-fixtures")]
fn searching_a_parameter_name_finds_its_function() {
    let Some(fixture) = node_editor_fixture() else {
        return;
    };
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
    let app = App::load();
    let rows = app.search_rows("canvas_name");
    assert!(rows.iter().any(|row| row.function == "accept_canvas"));
}
