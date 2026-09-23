//! Live-value data-flow panel for the call-graph overlay.
//! 调用图浮层的实时值数据流面板。
//!
//! TODO(trace-ingest): this panel reads `App::runtime_trace`, which
//! `App::new` fills with the built-in `sample_live_trace()`. There is no code
//! that loads a recorded `CallTrace` from a host run (a trace artifact file or
//! a `NICH_LINK_TRACE`-style environment variable), so the values shown here
//! are illustrative, not observed. Wire that ingest path and drop the
//! "built-in sample" label before claiming live values.
//! TODO(trace-ingest)：本面板读取 `App::runtime_trace`，而 `App::new` 用内置的
//! `sample_live_trace()` 填充它。目前没有任何代码从宿主运行记录中载入真实
//! `CallTrace`（trace artifact 文件或 `NICH_LINK_TRACE` 风格的环境变量），因此
//! 这里显示的值是示例而非实测。接上该 ingest 路径后才能去掉 "built-in sample"
//! 标记并宣称实时值。

use super::*;

/// Selectable runtime values for the active function.
/// 当前函数的可选择运行时值。
pub(super) fn draw_data_flow_panel(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    selected: Option<&CallRef>,
    cursor: usize,
    focused: bool,
    title: &str,
) {
    let Some(item) = selected else {
        frame.render_widget(
            Paragraph::new("Select a call node.").block(panel(title, MUTED)),
            area,
        );
        return;
    };
    let locals = app.graph_locals(item);
    let mut rows = Vec::new();
    if locals.is_empty() {
        rows.push(Line::from(Span::styled(
            "· no live locals captured",
            Style::default().fg(MUTED),
        )));
    } else {
        for local in locals {
            let id = nichlink_run_method::LocalId(local.id);
            let upstream = app
                .runtime_trace
                .incoming(id)
                .into_iter()
                .filter_map(|hop| {
                    hop.value.map(|candidate| {
                        format!(
                            "{}={} [{}]",
                            candidate.name, candidate.value, hop.edge.label
                        )
                    })
                })
                .take(2)
                .collect::<Vec<_>>();
            let downstream = app
                .runtime_trace
                .outgoing(id)
                .into_iter()
                .filter_map(|hop| {
                    hop.value.map(|candidate| {
                        format!(
                            "{}={} [{}]",
                            candidate.name, candidate.value, hop.edge.label
                        )
                    })
                })
                .take(2)
                .collect::<Vec<_>>();
            let frame_id = local.frame_id.unwrap_or(0);
            let mut line = vec![
                Span::styled(
                    format!("#{} {} ", frame_id, local.kind.label()),
                    Style::default().fg(CYAN),
                ),
                Span::styled(
                    format!("{} ", local.name),
                    Style::default().fg(INK).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("= {} [{}]", local.value, local.observation.label()),
                    Style::default().fg(GREEN),
                ),
            ];
            if !upstream.is_empty() {
                line.push(Span::styled(
                    format!("  ← {}", upstream.join(", ")),
                    Style::default().fg(MAGENTA),
                ));
            }
            if !downstream.is_empty() {
                line.push(Span::styled(
                    format!("  → {}", downstream.join(", ")),
                    Style::default().fg(CYAN),
                ));
            }
            line.push(Span::styled(
                format!("  {}", local.source),
                Style::default().fg(MUTED),
            ));
            rows.push(Line::from(line));
        }
    }
    let selected_index = (!rows.is_empty()).then_some(cursor.min(rows.len().saturating_sub(1)));
    let mut state = ListState::default().with_selected(selected_index);
    // "built-in sample" is the honest label for the values below: see the
    // module TODO. It stays until a real trace ingest path exists.
    // "built-in sample" 是对下方数值的诚实标注：见模块级 TODO。在真实 trace
    // ingest 路径出现之前保留。
    let panel_title = format!("{title} · built-in sample · {}", item.function);
    frame.render_stateful_widget(
        List::new(rows)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(if focused { GREEN } else { CYAN })
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ")
            .block(panel(panel_title, if focused { GREEN } else { MUTED })),
        area,
        &mut state,
    );
}
