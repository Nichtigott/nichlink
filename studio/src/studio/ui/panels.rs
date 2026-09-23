//! Main workspace panels: brand, registration tree, and face inspector.
//! 主工作区面板：标题、注册树与注册面检视器。
//!
//! The status line and footer live in `super::status`; the overlay router in
//! `super::overlay`. This page only draws the persistent three-column body.
//! 状态行与页脚位于 `super::status`；浮层路由位于 `super::overlay`。
//! 本页只绘制常驻的三栏主体。

use super::*;

pub(super) fn draw_brand(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let mut art = NICH_LINK_MARK
        .into_iter()
        .map(Line::from)
        .collect::<Vec<_>>();
    art.push(Line::from(vec![
        // The legend says "LIVE SAMPLE" on purpose: Studio only ever installs
        // `sample_live_trace()` today, so calling it plain "LIVE" advertised a
        // recorded trace that does not exist yet. The real ingest path is
        // tracked as a TODO in `app::lifecycle`.
        // 图例写作 "LIVE SAMPLE" 是有意的：Studio 目前只会装入
        // `sample_live_trace()`。直接写 "LIVE" 会宣称一份尚不存在的真实记录。
        // 真实 ingest 路径记录在 `app::lifecycle` 的 TODO 中。
        Span::styled(
            "  LIVE SAMPLE  ",
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                "{}  nodes {}  registries {}  checks {}  studio adapter",
                app.page.label(),
                app.registry.node_count(),
                app.registry.registry_count(),
                app.registry.check_count()
            ),
            Style::default().fg(MUTED),
        ),
    ]));
    frame.render_widget(
        Paragraph::new(art)
            .alignment(Alignment::Center)
            .style(Style::default().fg(INK).bg(Color::Rgb(12, 55, 65)))
            .block(panel(" NICH LINK // REGISTRATION STUDIO ", CYAN)),
        area,
    );
}

pub(super) fn draw_workspace(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    app.hot.workspace_area = area;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(app.split_percent),
            Constraint::Percentage(100 - app.split_percent),
        ])
        .split(area);
    app.hot.tree_area = columns[0];
    app.hot.details_area = columns[1];
    draw_tree(frame, columns[0], app);
    draw_details(frame, columns[1], app);
}

pub(super) fn draw_tree(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    let nodes = app.visible_nodes();
    let selected = nodes
        .iter()
        .position(|(id, _)| *id == app.selected)
        .unwrap_or_default();
    let items = nodes
        .into_iter()
        .map(|(id, depth)| {
            let selected = id == app.selected;
            let branch = if app.owns_registry(id) {
                if app.collapsed.contains(&id) {
                    "▸"
                } else {
                    "▾"
                }
            } else {
                "·"
            };
            let style = if selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(CYAN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(INK)
            };
            ListItem::new(format!(
                "{}{} {}",
                "  ".repeat(depth),
                branch,
                app.node_label(id)
            ))
            .style(style)
        })
        .collect::<Vec<_>>();
    let border = if app.focus == Focus::Tree {
        CYAN
    } else {
        MUTED
    };
    let mut state = ListState::default()
        .with_selected(Some(selected))
        .with_offset(app.tree_offset);
    frame.render_stateful_widget(
        List::new(items).block(panel(" REGISTRATION TREE ", border)),
        area,
        &mut state,
    );
    app.tree_offset = state.offset();
}

pub(super) fn draw_details(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let values = if let Some(info) = app.selected_info() {
        vec![
            ("name", info.registry_name.to_owned()),
            ("kind", info.kind.to_owned()),
            ("node", info.id.to_string()),
            (
                "path",
                app.registry
                    .path_for(info.id)
                    .unwrap_or_else(|| "<unknown>".to_owned()),
            ),
            ("parent", info.parent.to_string()),
            (
                "preset / parts",
                format!("{} / {}", info.preset, info.parts),
            ),
            ("params", info.params.to_owned()),
            ("handle", info.handle.to_owned()),
            ("handle interfaces", info.handle_traits.join(", ")),
            ("parts interfaces", info.part_traits.join(", ")),
            ("declared", info.source.describe()),
            (
                "registration rule",
                format!(
                    "{} ({})",
                    format_registration_rule(&info.registry_rule),
                    info.registry_rule_path
                ),
            ),
            ("dependency admission", format_admission(&info.admission)),
            ("exports", info.exports.join(", ")),
        ]
    } else {
        vec![
            ("name", "root".to_owned()),
            ("node", app.registry.id().to_string()),
        ]
    };
    let inner_width = area.width.saturating_sub(2) as usize;
    let selected = app.details_selected.min(values.len().saturating_sub(1));
    let lines = values
        .into_iter()
        .enumerate()
        .flat_map(|(index, (name, value))| {
            let mut field_lines = detail_field(name, value, inner_width);
            if index == selected
                && let Some(first) = field_lines.first_mut()
            {
                first.spans.insert(
                    0,
                    Span::styled(
                        "› ",
                        Style::default()
                            .fg(if app.focus == Focus::Details {
                                Color::Black
                            } else {
                                CYAN
                            })
                            .add_modifier(Modifier::BOLD),
                    ),
                );
                if app.focus == Focus::Details {
                    first.style = Style::default().bg(CYAN).fg(Color::Black);
                }
            }
            field_lines
        })
        .collect::<Vec<_>>();
    let border = if app.focus == Focus::Details {
        CYAN
    } else {
        MUTED
    };
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" FACE INSPECTOR ", border)),
        area,
    );
}
