//! Search result and source preview rendering.
//! 搜索结果与源码预览渲染。

use super::*;

use super::search_detail::draw_search_detail;

pub(super) fn draw_search(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &mut App,
    search: &SearchState,
) -> (Rect, usize, usize) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(2),
        ])
        .split(area.inner(ratatui::layout::Margin {
            vertical: 1,
            horizontal: 2,
        }));
    draw_query_bar(frame, inner[0], search);
    if search.graph_mode {
        draw_search_graph(frame, inner[1], app, search);
        frame.render_widget(
            Paragraph::new("A/B: input ↓ center ↓ output   Tab focus   ↑↓ select   Enter follow/open   Ctrl-W compare   Esc back")
                .alignment(Alignment::Center)
                .style(Style::default().fg(MUTED)),
            inner[2],
        );
        return (inner[1], search.offset, search.compare_offset);
    }
    let body = inner[1];
    let (list_area, compare_area, detail_area) = if search.compare_query.is_some() {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(56), Constraint::Percentage(44)])
            .split(body);
        let lists = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(columns[0]);
        (lists[0], Some(lists[1]), columns[1])
    } else {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(31),
                Constraint::Percentage(23),
                Constraint::Percentage(46),
            ])
            .split(body);
        draw_search_lane(frame, columns[1], app, search);
        (columns[0], None, columns[2])
    };
    let offset = draw_search_list(
        frame,
        list_area,
        app,
        SearchListOptions {
            query: &search.query,
            selected: search.selected,
            offset: search.offset,
            folded: &search.folded,
            active: search.active_pane == 0,
            title: " SEARCH A ",
        },
    );
    let compare_offset = compare_area.map_or(0, |area| {
        app.overlay_compare_list_area = area;
        let query = search.compare_query.as_deref().unwrap_or("");
        draw_search_list(
            frame,
            area,
            app,
            SearchListOptions {
                query,
                selected: search.compare_selected,
                offset: search.compare_offset,
                folded: &search.folded,
                active: search.active_pane == 1,
                title: " SEARCH B ",
            },
        )
    });
    let active_query = if search.active_pane == 1 {
        search.compare_query.as_deref().unwrap_or("")
    } else {
        &search.query
    };
    let active_selected = if search.active_pane == 1 {
        search.compare_selected
    } else {
        search.selected
    };
    if search.compare_query.is_some() {
        draw_search_detail(
            frame,
            detail_area,
            app,
            active_query,
            active_selected,
            &search.folded,
        );
    } else {
        draw_search_preview(
            frame,
            detail_area,
            app,
            active_query,
            active_selected,
            &search.folded,
        );
    }
    frame.render_widget(
        Paragraph::new("type to filter   Ctrl-W second pane   Tab switch   ↑↓ select   ←→ fold   Enter edit   Esc close")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED)),
        inner[2],
    );
    (list_area, offset, compare_offset)
}

/// Two compact query fields stay visible while comparing two files/functions.
/// 对比两个文件或函数时，两个紧凑搜索框始终可见。
fn draw_query_bar(frame: &mut Frame<'_>, area: Rect, search: &SearchState) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);
    let compare = search
        .compare_query
        .as_deref()
        .unwrap_or("Ctrl-W to add comparison");
    let focus = if search.graph_mode {
        search.graph_focus
    } else {
        search.active_pane
    };
    for (index, (title, query, active)) in [
        (" SEARCH A ", search.query.as_str(), focus == 0),
        (" SEARCH B ", compare, focus == 1),
    ]
    .into_iter()
    .enumerate()
    {
        frame.render_widget(
            Paragraph::new(if index == 1 && search.compare_query.is_none() {
                query.to_owned()
            } else {
                format!("/ {query}")
            })
            .style(
                Style::default()
                    .fg(if active { INK } else { MUTED })
                    .bg(Color::Rgb(25, 34, 40)),
            )
            .block(panel(title, if active { CYAN } else { MUTED })),
            columns[index],
        );
    }
}

/// Compact first-page preview: the selected symbol and only its direct calls.
/// 第一页紧凑预览：只显示选中符号及其直接调用关系。
fn draw_search_preview(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    query: &str,
    selected: usize,
    folded: &std::collections::BTreeSet<usize>,
) {
    let rows = app.search_rows(query, folded);
    let Some(row) = rows.get(selected) else {
        frame.render_widget(
            Paragraph::new("Type a file or function name.").block(panel(" PREVIEW ", MUTED)),
            area,
        );
        return;
    };
    let Some(node) = row.node else {
        frame.render_widget(
            Paragraph::new(row.text.as_str()).block(panel(" PREVIEW ", MUTED)),
            area,
        );
        return;
    };
    let Some(info) = app.registry.find(node) else {
        return;
    };
    let source = source_path_for(&info.source.file);
    let text = std::fs::read_to_string(source).unwrap_or_default();
    let mut lines = vec![Line::from(vec![
        Span::styled(
            if row.function.is_empty() {
                "󰈙 "
            } else {
                "ƒ "
            },
            Style::default().fg(CYAN),
        ),
        Span::styled(
            if row.function.is_empty() {
                info.registry_name.to_owned()
            } else {
                row.function.clone()
            },
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ),
    ])];
    lines.push(Line::from(Span::styled(
        info.source.file.to_owned(),
        Style::default().fg(MUTED),
    )));
    if row.function.is_empty() {
        lines.push(Line::from(Span::styled(
            "OBJECTS / FUNCTIONS",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        )));
        for line in text
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with("pub fn ")
                    || trimmed.starts_with("fn ")
                    || trimmed.starts_with("impl ")
            })
            .take(12)
        {
            let mut highlighted = highlight_rust_line(line.trim().trim_end_matches('{').trim());
            highlighted
                .spans
                .insert(0, Span::styled("  · ", Style::default().fg(CYAN)));
            lines.push(highlighted);
        }
    } else {
        let source_lines = text.lines().collect::<Vec<_>>();
        let (callers, callees) = app.call_relations(node, &row.function);
        let signature = text
            .lines()
            .find(|line| line.contains(&format!("{}(", row.function)))
            .map(|line| line.trim().trim_end_matches('{').trim().to_owned());
        if let Some(signature) = signature {
            lines.push(highlight_rust_line(&signature));
        }
        if let Some((start, end)) = app_function_source_range(&source_lines, &row.function) {
            lines.push(Line::from(Span::styled(
                "SOURCE PREVIEW",
                Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
            )));
            let available = area.height.saturating_sub(lines.len() as u16 + 8) as usize;
            for (offset, source_line) in source_lines[start..=end]
                .iter()
                .enumerate()
                .take(available.max(4))
            {
                let mut highlighted = highlight_rust_line(source_line);
                highlighted.spans.insert(
                    0,
                    Span::styled(
                        format!(
                            "{:>4} ",
                            row.line.unwrap_or(info.source.line) + offset as u32
                        ),
                        Style::default().fg(MUTED),
                    ),
                );
                lines.push(highlighted);
            }
        }
        lines.push(Line::from(Span::styled(
            "← USED BY",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        )));
        if callers.is_empty() {
            lines.push(Line::from(Span::styled(
                "  · no direct caller",
                Style::default().fg(MUTED),
            )));
        } else {
            for caller in callers.iter().take(8) {
                lines.push(Line::from(vec![
                    Span::styled("  ← ", Style::default().fg(CYAN)),
                    Span::styled(caller.function.clone(), Style::default().fg(INK)),
                ]));
            }
        }
        lines.push(Line::from(Span::styled(
            "CALLS →",
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        )));
        if callees.is_empty() {
            lines.push(Line::from(Span::styled(
                "  · no direct callee",
                Style::default().fg(MUTED),
            )));
        } else {
            for callee in callees.iter().take(8) {
                lines.push(Line::from(vec![
                    Span::styled("  → ", Style::default().fg(CYAN)),
                    Span::styled(callee.function.clone(), Style::default().fg(INK)),
                ]));
            }
        }
        lines.push(Line::from(Span::styled(
            "Enter opens the call graph",
            Style::default().fg(MUTED),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(" SELECTED SYMBOL ", GREEN)),
        area,
    );
}

/// A narrow relationship lane kept between results and the preview.
/// 放在结果和预览之间的窄调用关系栏。
fn draw_search_lane(frame: &mut Frame<'_>, area: Rect, app: &App, search: &SearchState) {
    let rows = app.search_rows(&search.query, &search.folded);
    let Some(row) = rows.get(search.selected) else {
        frame.render_widget(
            Paragraph::new("select a symbol").block(panel(" RELATION ", MUTED)),
            area,
        );
        return;
    };
    let Some(node) = row.node else {
        return;
    };
    let function = if row.function.is_empty() {
        app.registry
            .find(node)
            .map(|info| info.source.function.as_str())
            .unwrap_or("")
    } else {
        row.function.as_str()
    };
    let (callers, callees) = app.call_relations(node, function);
    let mut lines = vec![Line::from(Span::styled(
        "INPUT / OUTPUT",
        Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
    ))];
    for caller in callers.iter().take(5) {
        lines.push(Line::from(vec![
            Span::styled("← ", Style::default().fg(CYAN)),
            Span::styled(caller.function.clone(), Style::default().fg(INK)),
        ]));
    }
    lines.push(Line::from(Span::styled("────", Style::default().fg(MUTED))));
    lines.push(Line::from(Span::styled(
        if function.is_empty() {
            "file"
        } else {
            function
        },
        Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled("────", Style::default().fg(MUTED))));
    for callee in callees.iter().take(5) {
        lines.push(Line::from(vec![
            Span::styled("→ ", Style::default().fg(CYAN)),
            Span::styled(callee.function.clone(), Style::default().fg(INK)),
        ]));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .block(panel(" CALL LANE ", CYAN)),
        area,
    );
}

struct SearchListOptions<'a> {
    query: &'a str,
    selected: usize,
    offset: usize,
    folded: &'a std::collections::BTreeSet<usize>,
    active: bool,
    title: &'static str,
}

fn draw_search_list(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    options: SearchListOptions<'_>,
) -> usize {
    let SearchListOptions {
        query,
        selected,
        offset,
        folded,
        active,
        title,
    } = options;
    let rows = app.search_rows(query, folded);
    let items = rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let marker = if row.function.is_empty() {
                "󰈙"
            } else if row.has_children {
                if folded.contains(&row.source_index) {
                    "▸"
                } else {
                    "▾"
                }
            } else {
                "ƒ"
            };
            let style = if index == selected && active {
                Style::default().fg(Color::Black).bg(CYAN)
            } else {
                Style::default().fg(INK)
            };
            let line = search_row_line(row, query, marker);
            ListItem::new(line).style(style)
        })
        .collect::<Vec<_>>();
    let selected = selected.min(rows.len().saturating_sub(1));
    let mut state = ListState::default()
        .with_selected((!rows.is_empty()).then_some(selected))
        .with_offset(offset);
    frame.render_stateful_widget(
        List::new(items).block(panel(title, if active { CYAN } else { MAGENTA })),
        area,
        &mut state,
    );
    state.offset()
}

fn search_row_line(row: &super::super::app::SearchRow, query: &str, marker: &str) -> Line<'static> {
    let marker = Span::styled(format!("{marker} "), Style::default().fg(CYAN));
    if row.function.is_empty() {
        let path = row.path.as_str();
        let split = path.rfind('/').map(|index| index + 1).unwrap_or(0);
        return Line::from(vec![
            marker,
            Span::styled(path[..split].to_owned(), Style::default().fg(MUTED)),
            Span::styled(
                path[split..].to_owned(),
                search_hit_style(query, &path[split..]),
            ),
        ]);
    }
    let function_style = search_hit_style(query, &row.function);
    Line::from(vec![
        marker,
        Span::styled(row.path.clone(), Style::default().fg(MUTED)),
        Span::styled(" -> fn ", Style::default().fg(MUTED)),
        Span::styled(row.function.clone(), function_style),
    ])
}

fn search_hit_style(query: &str, value: &str) -> Style {
    if !query.trim().is_empty()
        && value
            .to_ascii_lowercase()
            .contains(&query.trim().to_ascii_lowercase())
    {
        Style::default().fg(INK).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(INK)
    }
}

/// Highlight a compact Rust source line without pulling a parser into Studio.
/// 不引入解析器，只给紧凑 Rust 源码行提供足够清晰的颜色层次。
fn highlight_rust_line(source: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let chars = source.chars().collect::<Vec<_>>();
    let mut index = 0;
    while index < chars.len() {
        if chars[index].is_whitespace() {
            let start = index;
            while index < chars.len() && chars[index].is_whitespace() {
                index += 1;
            }
            spans.push(Span::raw(chars[start..index].iter().collect::<String>()));
            continue;
        }
        if chars[index] == '/' && chars.get(index + 1) == Some(&'/') {
            spans.push(Span::styled(
                chars[index..].iter().collect::<String>(),
                Style::default().fg(MUTED),
            ));
            break;
        }
        if chars[index] == '"' {
            let start = index;
            index += 1;
            while index < chars.len() {
                if chars[index] == '\\' {
                    index = (index + 2).min(chars.len());
                    continue;
                }
                index += 1;
                if chars[index - 1] == '"' {
                    break;
                }
            }
            spans.push(Span::styled(
                chars[start..index].iter().collect::<String>(),
                Style::default().fg(MAGENTA),
            ));
            continue;
        }
        if chars[index].is_ascii_digit() {
            let start = index;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || matches!(chars[index], '.' | '_'))
            {
                index += 1;
            }
            spans.push(Span::styled(
                chars[start..index].iter().collect::<String>(),
                Style::default().fg(Color::Yellow),
            ));
            continue;
        }
        if chars[index].is_ascii_alphabetic() || chars[index] == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric() || chars[index] == '_')
            {
                index += 1;
            }
            let token = chars[start..index].iter().collect::<String>();
            let style = if matches!(
                token.as_str(),
                "fn" | "pub"
                    | "impl"
                    | "let"
                    | "mut"
                    | "if"
                    | "else"
                    | "match"
                    | "return"
                    | "struct"
                    | "enum"
                    | "trait"
                    | "for"
                    | "in"
                    | "use"
                    | "self"
                    | "Self"
            ) {
                Style::default().fg(CYAN).add_modifier(Modifier::BOLD)
            } else if token.chars().next().is_some_and(char::is_uppercase)
                || matches!(token.as_str(), "f32" | "f64" | "u32" | "usize" | "String")
            {
                Style::default().fg(GREEN)
            } else {
                Style::default().fg(INK)
            };
            spans.push(Span::styled(token, style));
            continue;
        }
        spans.push(Span::styled(
            chars[index].to_string(),
            Style::default().fg(MUTED),
        ));
        index += 1;
    }
    Line::from(spans)
}

pub(super) fn format_admission(admission: &nichlink::OwnedAdmission) -> String {
    if admission.allowed_paths.is_empty() && admission.denied_paths.is_empty() {
        return "ANY".to_owned();
    }
    if !admission.allowed_paths.is_empty() {
        return format!("allow:{}", admission.allowed_paths.join(","));
    }
    format!("deny:{}", admission.denied_paths.join(","))
}

pub(super) fn format_registration_rule(rule: &nichlink::OwnedRegistrationRule) -> String {
    let mut text = if rule.allowed_kinds.is_empty() && rule.denied_kinds.is_empty() {
        "ANY".to_owned()
    } else if !rule.allowed_kinds.is_empty() {
        format!("allow:{}", rule.allowed_kinds.join(","))
    } else {
        format!("deny:{}", rule.denied_kinds.join(","))
    };
    if let Some(preset) = &rule.required_preset {
        text.push_str(&format!(";preset:{preset}"));
    }
    if !rule.required_parts.is_empty() {
        text.push_str(&format!(";parts:{}", rule.required_parts.join(",")));
    }
    if !rule.required_exports.is_empty() {
        text.push_str(&format!(";exports:{}", rule.required_exports.join(",")));
    }
    if !rule.required_handle_traits.is_empty() {
        text.push_str(&format!(
            ";handle:{}",
            rule.required_handle_traits.join(",")
        ));
    }
    if !rule.required_part_traits.is_empty() {
        text.push_str(&format!(
            ";part_trait:{}",
            rule.required_part_traits.join(",")
        ));
    }
    text
}
