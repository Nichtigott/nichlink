//! Call graph and data-flow rendering for Studio.
//! Studio 调用图与数据流渲染。
use super::*;

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
    app.graph_area = area;
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
        app.graph_callers_area = columns[0];
        app.graph_center_area = columns[0];
        app.graph_callees_area = columns[0];
        app.graph_tree_a_area = columns[1];
        app.graph_data_a_area = columns[2];
        app.graph_tree_b_area = Rect::default();
        app.graph_data_b_area = Rect::default();
        app.graph_detail_area = columns[2];
        app.graph_provenance_area = columns[2];
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
    app.graph_callers_area = columns[0];
    app.graph_center_area = columns[1];
    app.graph_callees_area = columns[2];
    app.graph_detail_area = columns[3];
    app.graph_provenance_area = columns[3];

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
    app.graph_tree_a_area = tree[0];
    app.graph_tree_b_area = tree[1];
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
    app.graph_data_a_area = data[0];
    app.graph_data_b_area = data[1];
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
    let rows = app.search_rows(query, &search.folded);
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

fn draw_graph_triptych(
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
            app.graph_a_input_area = rows[0];
            app.graph_a_center_area = rows[2];
            app.graph_a_output_area = rows[4];
        }
        _ => {
            app.graph_b_input_area = rows[0];
            app.graph_b_center_area = rows[2];
            app.graph_b_output_area = rows[4];
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

fn draw_flow_arrow(frame: &mut Frame<'_>, area: Rect, glyph: &str) {
    frame.render_widget(
        Paragraph::new(glyph)
            .alignment(Alignment::Center)
            .style(Style::default().fg(CYAN).add_modifier(Modifier::BOLD)),
        area,
    );
}

#[allow(clippy::too_many_arguments)]
fn draw_relation_column(
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

fn draw_call_tree(
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

/// Selectable runtime values for the active function.
/// 当前函数的可选择运行时值。
fn draw_data_flow_panel(
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
    let panel_title = format!("{title} · {}", item.function);
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
