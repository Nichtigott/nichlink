//! Call graph and data-flow rendering for Studio.
//! Studio 调用图与数据流渲染。
//!
//! The module root owns the overlay entry point and the center/compare
//! routing; panel layout, call-tree nodes, relation edges, and live values
//! live in the mounted submodules and stay reachable through this path.
//! 模块根承载浮层入口与 center/compare 路由；面板布局、调用树节点、
//! 调用关系边与实时值位于挂载的子模块，并继续经此路径可达。

use super::*;

#[path = "graph/cells.rs"]
mod cells;
#[path = "graph/data.rs"]
mod data;
#[path = "graph/edges.rs"]
mod edges;
#[path = "graph/layout.rs"]
mod layout;
#[path = "graph/nodes.rs"]
mod nodes;

use data::draw_data_flow_panel;
use layout::draw_graph_triptych;
use nodes::draw_call_tree;

pub(super) fn draw_search_graph(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &mut App,
    search: &SearchState,
) {
    if search.center.is_none() && search.compare_center.is_none() {
        frame.render_widget(
            Paragraph::new("No center selected.").block(panel(" PROVENANCE ", MUTED)),
            area,
        );
        return;
    }
    app.hot.graph_area = area;
    let has_compare = search.compare_query.is_some() || search.compare_center.is_some();
    if !has_compare {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(app.graph_split_percent),
                Constraint::Percentage((100 - app.graph_split_percent) / 2),
                Constraint::Percentage((100 - app.graph_split_percent) / 2),
            ])
            .split(area);
        let a = search_center_ref(app, search, 0);
        app.hot.graph_callers_area = columns[0];
        app.hot.graph_center_area = columns[0];
        app.hot.graph_callees_area = columns[0];
        app.hot.graph_tree_a_area = columns[1];
        app.hot.graph_data_a_area = columns[2];
        app.hot.graph_tree_b_area = Rect::default();
        app.hot.graph_data_b_area = Rect::default();
        app.hot.graph_detail_area = columns[2];
        app.hot.graph_provenance_area = columns[2];
        draw_graph_triptych(
            frame,
            columns[0],
            app,
            a.as_ref(),
            search.graph_selected,
            search.graph_focus == 0,
            "A",
        );
        draw_call_tree(
            frame,
            columns[1],
            app,
            a.as_ref(),
            search.outline_selected,
            search.graph_focus == 2,
            "CALL TREE",
        );
        let a_data = app.graph_tree_item(search, 0, search.outline_selected);
        draw_data_flow_panel(
            frame,
            columns[2],
            app,
            a_data.as_ref(),
            search.data_selected,
            search.graph_focus == 3,
            "DATA FLOW",
        );
        return;
    }
    let left = app.graph_split_percent / 2;
    let right = 100_u16.saturating_sub(app.graph_split_percent) / 2;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(left),
            Constraint::Percentage(left),
            Constraint::Percentage(right),
            Constraint::Percentage(right),
        ])
        .split(area);
    app.hot.graph_callers_area = columns[0];
    app.hot.graph_center_area = columns[1];
    app.hot.graph_callees_area = columns[2];
    app.hot.graph_detail_area = columns[3];
    app.hot.graph_provenance_area = columns[3];

    let a = search_center_ref(app, search, 0);
    let b = search_center_ref(app, search, 1);
    draw_graph_triptych(
        frame,
        columns[0],
        app,
        a.as_ref(),
        search.graph_selected,
        search.graph_focus == 0,
        "A",
    );
    draw_graph_triptych(
        frame,
        columns[1],
        app,
        b.as_ref(),
        search.compare_graph_selected,
        search.graph_focus == 1,
        "B",
    );

    let tree = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(columns[2]);
    app.hot.graph_tree_a_area = tree[0];
    app.hot.graph_tree_b_area = tree[1];
    draw_call_tree(
        frame,
        tree[0],
        app,
        a.as_ref(),
        search.outline_selected,
        search.graph_focus == 2 && search.graph_side == 0,
        "CALL TREE / A",
    );
    draw_call_tree(
        frame,
        tree[1],
        app,
        b.as_ref(),
        search.compare_outline_selected,
        search.graph_focus == 2 && search.graph_side == 1,
        "CALL TREE / B",
    );

    let data = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(columns[3]);
    app.hot.graph_data_a_area = data[0];
    app.hot.graph_data_b_area = data[1];
    let a_data = app.graph_tree_item(search, 0, search.outline_selected);
    let b_data = app.graph_tree_item(search, 1, search.compare_outline_selected);
    draw_data_flow_panel(
        frame,
        data[0],
        app,
        a_data.as_ref(),
        search.data_selected,
        search.graph_focus == 3 && search.graph_side == 0,
        "DATA FLOW / A",
    );
    draw_data_flow_panel(
        frame,
        data[1],
        app,
        b_data.as_ref(),
        search.compare_data_selected,
        search.graph_focus == 3 && search.graph_side == 1,
        "DATA FLOW / B",
    );
}

/// Resolve the selected call reference for one comparison side.
/// 解析某一比较侧当前选中的调用引用。
fn search_center_ref(app: &App, search: &SearchState, side: usize) -> Option<CallRef> {
    let (center, function) = if side == 0 {
        (search.center, search.center_function.as_deref())
    } else {
        (
            search.compare_center,
            search.compare_center_function.as_deref(),
        )
    };
    if let Some(node) = center {
        return app.registry.find(node).map(|info| CallRef {
            node,
            function: function
                .filter(|name| !name.is_empty())
                .unwrap_or(&info.source.function)
                .to_owned(),
            file: info.source.file.to_owned(),
        });
    }
    let query = if side == 0 {
        &search.query
    } else {
        search.compare_query.as_deref().unwrap_or("")
    };
    let selected = if side == 0 {
        search.selected
    } else {
        search.compare_selected
    };
    let rows = app.search_rows(query);
    let row = rows.get(selected)?;
    let node = row.node?;
    let info = app.registry.find(node)?;
    Some(CallRef {
        node,
        function: if row.function.is_empty() {
            info.source.function.to_owned()
        } else {
            row.function.clone()
        },
        file: info.source.file.to_owned(),
    })
}
