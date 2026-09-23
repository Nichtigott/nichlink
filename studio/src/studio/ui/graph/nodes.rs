//! Call-tree node rendering for the call-graph overlay.
//! 调用图浮层的调用树节点渲染。

use super::*;

pub(super) fn draw_call_tree(
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
    let Some(info) = app.registry.find(item.node) else {
        return;
    };
    let source = source_path_for(&info.source.file);
    let text = std::fs::read_to_string(source).unwrap_or_default();
    let source_lines = text.lines().collect::<Vec<_>>();
    let (callers, callees) = app.call_relations(item.node, &item.function);
    let mir_candidates = app.mir_candidates_for(&item.function);
    let signature = source_lines
        .iter()
        .find(|line| line.contains(&format!("{}(", item.function)))
        .map(|line| line.trim().trim_end_matches('{').trim_end().to_owned())
        .unwrap_or_else(|| item.function.clone());
    let transforms = app.call_tree_transforms(item);
    let mut entries = Vec::new();
    let signature_width = area.width.saturating_sub(4).max(8) as usize;
    let signature_lines = wrap_text(&signature, signature_width)
        .into_iter()
        .enumerate()
        .map(|(index, chunk)| {
            Line::from(vec![
                Span::styled(
                    if index == 0 { "ƒ " } else { "  " },
                    Style::default().fg(CYAN),
                ),
                Span::styled(
                    chunk,
                    Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
                ),
            ])
        })
        .collect::<Vec<_>>();
    entries.push((signature_lines, Some(item.clone())));
    entries.push((
        vec![Line::from(Span::styled(
            "├─ input",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        ))],
        None,
    ));
    if !mir_candidates.is_empty() {
        entries.push((
            vec![Line::from(Span::styled(
                format!("│  ? {} MIR candidate edge(s)", mir_candidates.len()),
                Style::default().fg(MUTED),
            ))],
            None,
        ));
    }
    if callers.is_empty() {
        entries.push((
            vec![Line::from(Span::styled(
                "│  · manual / external entry",
                Style::default().fg(MUTED),
            ))],
            None,
        ));
    } else {
        for caller in callers {
            let target = caller.clone();
            let line = function_line(&caller.file, &caller.function).unwrap_or(1);
            entries.push((
                vec![Line::from(vec![
                    Span::styled("│  ← ", Style::default().fg(CYAN)),
                    Span::styled(caller.function.clone(), Style::default().fg(INK)),
                    Span::styled(
                        format!("  {}:{line}", caller.file),
                        Style::default().fg(MUTED),
                    ),
                ])],
                Some(target),
            ));
        }
    }
    entries.push((
        vec![Line::from(Span::styled(
            "├─ transform",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        ))],
        None,
    ));
    if transforms.is_empty() {
        entries.push((
            vec![Line::from(Span::styled(
                "│  · direct value",
                Style::default().fg(MUTED),
            ))],
            None,
        ));
    } else {
        for transform in transforms {
            entries.push((
                vec![Line::from(vec![
                    Span::styled("│  ↳ ", Style::default().fg(CYAN)),
                    Span::styled(transform, Style::default().fg(INK)),
                ])],
                None,
            ));
        }
    }
    entries.push((
        vec![Line::from(Span::styled(
            "└─ output",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        ))],
        None,
    ));
    if callees.is_empty() {
        entries.push((
            vec![Line::from(Span::styled(
                "   · returned to caller",
                Style::default().fg(MUTED),
            ))],
            None,
        ));
    } else {
        for callee in callees {
            let target = callee.clone();
            let line = function_line(&callee.file, &callee.function).unwrap_or(1);
            entries.push((
                vec![Line::from(vec![
                    Span::styled("   → ", Style::default().fg(CYAN)),
                    Span::styled(callee.function.clone(), Style::default().fg(INK)),
                    Span::styled(
                        format!("  {}:{line}", callee.file),
                        Style::default().fg(MUTED),
                    ),
                ])],
                Some(target),
            ));
        }
    }
    let items = entries
        .into_iter()
        .map(|(lines, _)| ListItem::new(lines))
        .collect::<Vec<_>>();
    let selected = (!items.is_empty()).then_some(cursor.min(items.len().saturating_sub(1)));
    let mut state = ListState::default().with_selected(selected);
    frame.render_stateful_widget(
        List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(if focused { GREEN } else { CYAN })
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ")
            .block(panel(title, if focused { GREEN } else { MAGENTA })),
        area,
        &mut state,
    );
}

/// Hard-wrap a signature label to the available panel width.
/// 将签名标签按可用面板宽度硬换行。
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let chars = text.chars().collect::<Vec<_>>();
    if chars.is_empty() {
        return vec![String::new()];
    }
    chars
        .chunks(width)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}
