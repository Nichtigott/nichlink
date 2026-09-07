//! Ratatui rendering for Studio.
//! Studio 的 Ratatui 渲染。

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap,
};

mod graph;
use graph::draw_search_graph;
mod forms;
use forms::{draw_add, draw_delete, draw_edit, draw_graft, draw_new_project, draw_plugin};
mod search;
use search::{draw_search, format_admission, format_registration_rule};
mod search_detail;

use super::app::{
    AddState, App, CallRef, Focus, Overlay, SearchState, app_function_source_range,
    face_field_indices, source_path_for,
};
use nichlink::FACE_FIELD_NAMES;

const INK: Color = Color::Rgb(214, 225, 231);
const MUTED: Color = Color::Rgb(112, 132, 143);
const CYAN: Color = Color::Rgb(75, 201, 220);
const GREEN: Color = Color::Rgb(97, 210, 151);
const MAGENTA: Color = Color::Rgb(223, 116, 186);
const PANEL: Color = Color::Rgb(16, 23, 28);

pub fn draw(frame: &mut Frame<'_>, app: &mut App) {
    let area = frame.area();
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(8, 12, 15))),
        area,
    );
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            // Seven logo rows plus status and borders keep the complete mark visible.
            // 七行标题、状态行和边框一起保留完整字标，避免裁掉底行。
            Constraint::Length(10),
            Constraint::Min(9),
            Constraint::Length(5),
            Constraint::Length(2),
        ])
        .split(area);
    draw_brand(frame, rows[0], app);
    draw_workspace(frame, rows[1], app);
    draw_event(frame, rows[2], app);
    draw_keys(frame, rows[3]);
    draw_overlay(frame, app);
}

fn draw_brand(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let art = vec![
        Line::from(r"________   ___  ________  ___  ___  ___       ___  ________   ___  __"),
        Line::from(r" |\   ___  \|\  \|\   ____\|\  \|\  \|\  \     |\  \|\   ___  \|\  \|\  \"),
        Line::from(
            r"   \ \  \\ \  \ \  \ \  \___|\ \  \\\  \ \  \    \ \  \ \  \\ \  \ \  \/  /|_",
        ),
        Line::from(
            r"     \ \  \\ \  \ \  \ \  \    \ \   __  \ \  \    \ \  \ \  \\ \  \ \   ___  \",
        ),
        Line::from(
            r"       \ \  \\ \  \ \  \ \  \____\ \  \ \  \ \  \____\ \  \ \  \\ \  \ \  \\ \  \",
        ),
        Line::from(
            r"         \ \__\\ \__\ \__\ \_______\ \__\ \__\ \_______\ \__\ \__\\ \__\ \__\\ \__\",
        ),
        Line::from(
            r"           \|__| \|__|\|__|\|_______|\|__|\|__|\|_______|\|__|\|__| \|__|\|__| \|__|",
        ),
        Line::from(vec![
            Span::styled(
                "  LIVE  ",
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
        ]),
    ];
    frame.render_widget(
        Paragraph::new(art)
            .alignment(Alignment::Center)
            .style(Style::default().fg(INK).bg(Color::Rgb(12, 55, 65)))
            .block(panel(" NICH LINK // REGISTRATION STUDIO ", CYAN)),
        area,
    );
}

fn draw_workspace(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
    app.workspace_area = area;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(app.split_percent),
            Constraint::Percentage(100 - app.split_percent),
        ])
        .split(area);
    app.tree_area = columns[0];
    app.details_area = columns[1];
    draw_tree(frame, columns[0], app);
    draw_details(frame, columns[1], app);
}

fn draw_tree(frame: &mut Frame<'_>, area: Rect, app: &mut App) {
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

fn draw_details(frame: &mut Frame<'_>, area: Rect, app: &App) {
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

fn draw_event(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let color = if app.event.to_ascii_lowercase().contains("fail") {
        Color::LightRed
    } else {
        GREEN
    };
    let event = app.reload_error.as_ref().map_or_else(
        || app.event.clone(),
        |error| format!("{}\n[phase={}] {}", app.event, error.phase, error.message),
    );
    frame.render_widget(
        Paragraph::new(event)
            .style(Style::default().fg(color))
            .wrap(Wrap { trim: false })
            .block(panel(" EVENT / BUILD ", MAGENTA)),
        area,
    );
}

fn draw_keys(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new(" 1 search   2 inspect   3 data   4 compare   q quit   / search   n new   a add   g graft draft   p plugin   e edit   d delete   Enter fold   m MIR   r/F5 reload   b/F9 build   ←/→ resize ")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED)),
        area,
    );
}

fn draw_overlay(frame: &mut Frame<'_>, app: &mut App) {
    let Some(overlay) = app.overlay.clone() else {
        app.overlay_area = Rect::default();
        app.overlay_list_area = Rect::default();
        app.overlay_compare_list_area = Rect::default();
        app.delete_cancel_area = Rect::default();
        app.delete_confirm_area = Rect::default();
        app.action_cancel_area = Rect::default();
        app.action_confirm_area = Rect::default();
        app.action_exit_area = Rect::default();
        app.graph_area = Rect::default();
        app.graph_detail_area = Rect::default();
        app.graph_provenance_area = Rect::default();
        app.graph_callers_area = Rect::default();
        app.graph_center_area = Rect::default();
        app.graph_callees_area = Rect::default();
        app.graph_tree_a_area = Rect::default();
        app.graph_tree_b_area = Rect::default();
        app.graph_data_a_area = Rect::default();
        app.graph_data_b_area = Rect::default();
        app.graph_a_input_area = Rect::default();
        app.graph_a_center_area = Rect::default();
        app.graph_a_output_area = Rect::default();
        app.graph_b_input_area = Rect::default();
        app.graph_b_center_area = Rect::default();
        app.graph_b_output_area = Rect::default();
        app.action_validate_area = Rect::default();
        app.action_edit_area = Rect::default();
        return;
    };
    let area = if matches!(overlay, Overlay::Search(ref search) if search.graph_mode) {
        centered(98, 92, frame.area())
    } else {
        centered(86, 78, frame.area())
    };
    app.overlay_area = area;
    app.overlay_list_area = Rect::default();
    app.overlay_compare_list_area = Rect::default();
    app.delete_cancel_area = Rect::default();
    app.delete_confirm_area = Rect::default();
    app.action_cancel_area = Rect::default();
    app.action_validate_area = Rect::default();
    app.action_edit_area = Rect::default();
    app.action_confirm_area = Rect::default();
    app.action_exit_area = Rect::default();
    app.graph_area = Rect::default();
    app.graph_detail_area = Rect::default();
    app.graph_provenance_area = Rect::default();
    app.graph_callers_area = Rect::default();
    app.graph_center_area = Rect::default();
    app.graph_callees_area = Rect::default();
    app.graph_tree_a_area = Rect::default();
    app.graph_tree_b_area = Rect::default();
    app.graph_data_a_area = Rect::default();
    app.graph_data_b_area = Rect::default();
    app.graph_a_input_area = Rect::default();
    app.graph_a_center_area = Rect::default();
    app.graph_a_output_area = Rect::default();
    app.graph_b_input_area = Rect::default();
    app.graph_b_center_area = Rect::default();
    app.graph_b_output_area = Rect::default();
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().style(Style::default().bg(PANEL)), area);
    match overlay {
        Overlay::Search(search) => {
            let (list_area, offset, compare_offset) = draw_search(frame, area, app, &search);
            app.overlay_list_area = list_area;
            if let Some(Overlay::Search(current)) = &mut app.overlay {
                current.offset = offset;
                current.compare_offset = compare_offset;
            }
        }
        Overlay::NewProject(project) => {
            let (list, cancel, confirm, exit) = draw_new_project(frame, area, &project);
            app.overlay_list_area = list;
            app.action_cancel_area = cancel;
            app.action_confirm_area = confirm;
            app.action_exit_area = exit;
        }
        Overlay::Add(add) => {
            let (list, cancel, confirm, exit) = draw_add(frame, area, &add);
            app.overlay_list_area = list;
            app.action_cancel_area = cancel;
            app.action_confirm_area = confirm;
            app.action_exit_area = exit;
        }
        Overlay::Edit(_, edit) => {
            let (list, cancel, confirm, exit) = draw_edit(frame, area, &edit);
            app.overlay_list_area = list;
            app.action_cancel_area = cancel;
            app.action_confirm_area = confirm;
            app.action_exit_area = exit;
        }
        Overlay::Graft(graft) => {
            let (validate, apply, cancel, edit) = draw_graft(frame, area, &graft);
            app.action_validate_area = validate;
            app.action_confirm_area = apply;
            app.action_cancel_area = cancel;
            app.action_edit_area = edit;
        }
        Overlay::Plugin(plugin) => {
            let (list, cancel, confirm, exit) = draw_plugin(frame, area, &plugin);
            app.overlay_list_area = list;
            app.action_cancel_area = cancel;
            app.action_confirm_area = confirm;
            app.action_exit_area = exit;
        }
        Overlay::Delete(id) => {
            let (cancel, confirm) = draw_delete(frame, area, app, id);
            app.delete_cancel_area = cancel;
            app.delete_confirm_area = confirm;
        }
    }
}

fn panel(title: impl Into<String>, color: Color) -> Block<'static> {
    Block::default()
        .title(title.into())
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(color))
        .style(Style::default().bg(PANEL))
}

fn field(name: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{name:<15}"), Style::default().fg(MUTED)),
        Span::styled(value.to_owned(), Style::default().fg(INK)),
    ])
}

fn detail_field(name: &str, value: String, width: usize) -> Vec<Line<'static>> {
    if 15 + value.chars().count() <= width {
        return vec![field(name, &value)];
    }
    vec![
        Line::from(Span::styled(name.to_owned(), Style::default().fg(MUTED))),
        Line::from(Span::styled(format!("  {value}"), Style::default().fg(INK))),
    ]
}

fn centered(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn rendered_text(width: u16, height: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        let mut app = App::load();
        terminal.draw(|frame| draw(frame, &mut app)).unwrap();
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect()
    }

    #[test]
    fn renders_brand_and_primary_panels_at_wide_size() {
        let output = rendered_text(140, 48);
        assert!(output.contains("NICH LINK"));
        assert!(output.contains("________"));
        assert!(output.contains("|__|"));
        assert!(output.contains("REGISTRATION TREE"));
        assert!(output.contains("FACE INSPECTOR"));
    }

    #[test]
    fn narrow_terminal_stays_renderable_without_panicking() {
        let output = rendered_text(64, 20);
        assert!(!output.is_empty());
    }
}
