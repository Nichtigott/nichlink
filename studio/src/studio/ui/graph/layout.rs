//! Panel layout and hot-zone geometry for the call-graph overlay.
//! 调用图浮层的面板布局与热区几何。

use super::*;

use super::edges::draw_relation_column;

pub(super) fn draw_graph_triptych(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &mut App,
    selected: Option<&CallRef>,
    cursor: usize,
    focused: bool,
    side: &'static str,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(4),
        ])
        .split(area);
    match side {
        "A" => {
            app.hot.graph_a_input_area = rows[0];
            app.hot.graph_a_center_area = rows[2];
            app.hot.graph_a_output_area = rows[4];
        }
        _ => {
            app.hot.graph_b_input_area = rows[0];
            app.hot.graph_b_center_area = rows[2];
            app.hot.graph_b_output_area = rows[4];
        }
    }
    let (callers, callees, highlighted) = selected
        .map(|selected| {
            let (callers, callees) = app.call_relations(selected.node, &selected.function);
            let chain = app.call_chain(selected.node, Some(&selected.function));
            let highlighted = chain
                .get(cursor)
                .cloned()
                .unwrap_or_else(|| selected.clone());
            (callers, callees, Some(highlighted))
        })
        .unwrap_or_default();
    let (mir_callers, mir_callees) = selected
        .map(|item| app.mir_relation_counts(&item.function))
        .unwrap_or_default();
    draw_relation_column(
        frame,
        rows[0],
        &format!("CALLERS {side}"),
        callers,
        highlighted.as_ref(),
        (mir_callers > 0).then_some(mir_callers),
        app,
        selected,
        true,
    );
    draw_flow_arrow(frame, rows[1], "↓");
    draw_relation_column(
        frame,
        rows[2],
        &format!("CENTER {side}"),
        selected.into_iter().cloned().collect(),
        selected,
        None,
        app,
        selected,
        false,
    );
    draw_flow_arrow(frame, rows[3], "↓");
    draw_relation_column(
        frame,
        rows[4],
        &format!("CALLEES {side}"),
        callees,
        highlighted.as_ref(),
        (mir_callees > 0).then_some(mir_callees),
        app,
        selected,
        false,
    );
    if focused {
        frame.render_widget(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(CYAN)),
            area,
        );
    }
}

/// Draw the downward connector between two relation columns.
/// 绘制两个关系列之间的向下连接箭头。
fn draw_flow_arrow(frame: &mut Frame<'_>, area: Rect, glyph: &str) {
    frame.render_widget(
        Paragraph::new(glyph)
            .alignment(Alignment::Center)
            .style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
        area,
    );
}
