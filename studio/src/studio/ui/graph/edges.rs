//! Caller/callee relation columns for the call-graph overlay.
//! 调用图浮层的调用者/被调用者关系列。

use super::*;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw_relation_column(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    nodes: Vec<CallRef>,
    selected: Option<&CallRef>,
    static_count: Option<usize>,
    app: &App,
    center: Option<&CallRef>,
    callers: bool,
) {
    let is_center = title.contains("CENTER");
    let mut lines = Vec::new();
    if nodes.is_empty() {
        lines.push(Line::from(Span::styled(
            "no runtime call edges",
            Style::default().fg(MUTED),
        )));
    } else {
        for (index, item) in nodes.iter().enumerate() {
            let active = selected.is_some_and(|selected| {
                item.node == selected.node && item.function == selected.function
            });
            let marker = if active { "◆" } else { "◇" };
            let arrow = if title.contains("CALLERS") {
                "←"
            } else if title.contains("CALLEES") {
                "→"
            } else {
                "●"
            };
            let evidence = center.map_or("·", |center| {
                if callers {
                    app.call_evidence(item, center).marker()
                } else if title.contains("CALLEES") {
                    app.call_evidence(center, item).marker()
                } else {
                    "●"
                }
            });
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{arrow} {evidence}{marker} "),
                    Style::default().fg(CYAN),
                ),
                Span::styled(
                    item.function.clone(),
                    Style::default()
                        .fg(if active { GREEN } else { INK })
                        .add_modifier(if is_center || active {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::styled(format!(" · {}", item.file), Style::default().fg(MUTED)),
            ]));
            if index + 1 < nodes.len() {
                lines.push(Line::from(Span::styled("│", Style::default().fg(MUTED))));
            }
        }
    }
    if let Some(count) = static_count.filter(|count| *count > 0) {
        lines.push(Line::from(Span::styled(
            format!("? {count} MIR candidate edge(s)"),
            Style::default().fg(MUTED),
        )));
    }
    let selected_line = nodes
        .iter()
        .position(|item| {
            selected.is_some_and(|selected| {
                item.node == selected.node && item.function == selected.function
            })
        })
        .map_or(0, |index| index * 2);
    let viewport = area.height.saturating_sub(2) as usize;
    let scroll = selected_line.saturating_sub(viewport.saturating_sub(1));
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .scroll((scroll.min(u16::MAX as usize) as u16, 0))
            .block(panel(
                title,
                if nodes.iter().any(|item| {
                    selected.is_some_and(|selected| {
                        item.node == selected.node && item.function == selected.function
                    })
                }) {
                    CYAN
                } else {
                    MAGENTA
                },
            )),
        area,
    );
}
