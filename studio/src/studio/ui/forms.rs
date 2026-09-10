//! Form overlay rendering for Studio.
//! Studio 表单浮层渲染。

use super::*;

mod face_fields;
use face_fields::{draw_face_field_help, face_field_presentation, face_field_value};

pub(super) fn draw_add(
    frame: &mut Frame<'_>,
    area: Rect,
    add: &AddState,
) -> (Rect, Rect, Rect, Rect) {
    draw_face_form(frame, area, add, " ADD REGISTRATION FACE ")
}

pub(super) fn draw_new_project(
    frame: &mut Frame<'_>,
    area: Rect,
    project: &super::super::app::NewProjectState,
) -> (Rect, Rect, Rect, Rect) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(4)])
        .split(area);
    let names = ["directory", "package", "kind"];
    let rows = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let selected = index == project.field;
            let marker = if selected && project.editing {
                "●"
            } else if selected {
                "›"
            } else {
                " "
            };
            let value = if project.values[index].is_empty() {
                "_"
            } else {
                &project.values[index]
            };
            ListItem::new(format!("{marker} {name:<12} {value}")).style(if selected {
                Style::default().fg(Color::Black).bg(CYAN)
            } else {
                Style::default().fg(INK)
            })
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(project.field));
    frame.render_stateful_widget(
        List::new(rows).block(panel(" NEW NICH LINK PROJECT ", GREEN)),
        inner[0],
        &mut state,
    );
    let buttons = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner[1]);
    let button = |label: &'static str, color: Color| {
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(Style::default().fg(color))
            .block(panel("", color))
    };
    frame.render_widget(button("Create [s]", GREEN), buttons[0]);
    frame.render_widget(button("Cancel [Esc]", MUTED), buttons[1]);
    frame.render_widget(button("Exit [q]", Color::LightRed), buttons[2]);
    frame.render_widget(
        Paragraph::new("↑↓ field   Enter edit/toggle")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(PANEL)),
        buttons[3],
    );
    (inner[0], buttons[1], buttons[0], buttons[2])
}

fn draw_face_form(
    frame: &mut Frame<'_>,
    area: Rect,
    add: &AddState,
    title: &'static str,
) -> (Rect, Rect, Rect, Rect) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(9), Constraint::Length(4)])
        .split(area);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(inner[0]);
    let fields = face_field_indices(add);
    let name_width = fields
        .iter()
        .map(|index| face_field_presentation(*index).label.len())
        .max()
        .unwrap_or(1);
    let rows = fields
        .iter()
        .map(|index| {
            let selected = *index == add.field;
            let marker = if selected && add.editing {
                "●"
            } else if selected {
                "›"
            } else {
                " "
            };
            let presentation = face_field_presentation(*index);
            let value = face_field_value(add, *index);
            let style = if selected {
                Style::default().fg(Color::Black).bg(CYAN)
            } else {
                Style::default().fg(INK)
            };
            let role = if add.locked_fields.contains(index) {
                face_fields::FaceFieldRole::ReadOnly
            } else if add.parent_requirements.contains_key(index) {
                face_fields::FaceFieldRole::Required
            } else {
                presentation.role
            };
            ListItem::new(format!(
                "{marker} {} {:<name_width$} {value}",
                role.marker(),
                presentation.label,
            ))
            .style(style)
        })
        .collect::<Vec<_>>();
    let selected_row = fields.iter().position(|index| *index == add.field);
    let visible = body[0].height.saturating_sub(2) as usize;
    let offset = selected_row
        .unwrap_or_default()
        .saturating_sub(visible.saturating_sub(1));
    let mut state = ListState::default()
        .with_selected(selected_row)
        .with_offset(offset);
    frame.render_stateful_widget(
        List::new(rows).block(panel(title, GREEN)),
        body[0],
        &mut state,
    );
    draw_face_field_help(frame, body[1], add);
    let buttons = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner[1]);
    let button = |label: &'static str, color: Color| {
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(Style::default().fg(color))
            .block(panel("", color))
    };
    frame.render_widget(button("Save [s]", GREEN), buttons[0]);
    frame.render_widget(button("Cancel [Esc]", MUTED), buttons[1]);
    frame.render_widget(button("Exit [q]", Color::LightRed), buttons[2]);
    frame.render_widget(
        Paragraph::new("* required  ◇ derived  ↳ read only  · optional  ? when used")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(PANEL)),
        buttons[3],
    );
    (body[0], buttons[1], buttons[0], buttons[2])
}

pub(super) fn draw_edit(
    frame: &mut Frame<'_>,
    area: Rect,
    edit: &AddState,
) -> (Rect, Rect, Rect, Rect) {
    draw_face_form(frame, area, edit, " EDIT REGISTRATION FACE ")
}

pub(super) fn draw_plugin(
    frame: &mut Frame<'_>,
    area: Rect,
    plugin: &super::super::app::PluginState,
) -> (Rect, Rect, Rect, Rect) {
    const NAMES: [&str; 7] = [
        "source",
        "framework",
        "package",
        "version",
        "crate",
        "checksum",
        "mode",
    ];
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(9), Constraint::Length(4)])
        .split(area);
    let rows = NAMES
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let selected = index == plugin.field;
            let marker = if selected && plugin.editing {
                "●"
            } else if selected {
                "›"
            } else {
                " "
            };
            let value = if plugin.values[index].is_empty() {
                "_"
            } else {
                &plugin.values[index]
            };
            let style = if selected {
                Style::default().fg(Color::Black).bg(CYAN)
            } else {
                Style::default().fg(INK)
            };
            ListItem::new(format!("{marker} {name:<12} {value}")).style(style)
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        List::new(rows).block(panel(" SELECT PLUGIN ", GREEN)),
        inner[0],
    );
    let buttons = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(inner[1]);
    let button = |label: &'static str, color: Color| {
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(Style::default().fg(color))
            .block(panel("", color))
    };
    frame.render_widget(button("Save [s]", GREEN), buttons[0]);
    frame.render_widget(button("Cancel [Esc]", MUTED), buttons[1]);
    frame.render_widget(button("Exit [q]", Color::LightRed), buttons[2]);
    frame.render_widget(
        Paragraph::new("↑↓ field   Enter edit/toggle")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(PANEL)),
        buttons[3],
    );
    (inner[0], buttons[1], buttons[0], buttons[2])
}

pub(super) fn draw_delete(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    id: nichlink_run_method::NodeId,
) -> (Rect, Rect) {
    let path = app
        .registry
        .path_for(id)
        .unwrap_or_else(|| "<unknown>".to_owned());
    let text = vec![
        Line::from(Span::styled(
            "Move this generated module to recoverable trash?",
            Style::default().fg(INK),
        )),
        Line::from(""),
        field("path", &path),
        field("node", &id.to_string()),
        Line::from(""),
    ];
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .block(panel(" DELETE REGISTRATION FACE ", Color::LightRed)),
        area,
    );
    let buttons = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Length(16),
            Constraint::Length(3),
            Constraint::Length(16),
            Constraint::Percentage(30),
        ])
        .split(Rect::new(
            area.x,
            area.bottom().saturating_sub(4),
            area.width,
            3,
        ));
    frame.render_widget(
        Paragraph::new("Cancel")
            .alignment(Alignment::Center)
            .block(panel(" n / Esc ", MUTED)),
        buttons[1],
    );
    frame.render_widget(
        Paragraph::new("Delete")
            .alignment(Alignment::Center)
            .style(Style::default().fg(Color::LightRed))
            .block(panel(" Enter / y ", Color::LightRed)),
        buttons[3],
    );
    (buttons[1], buttons[3])
}
