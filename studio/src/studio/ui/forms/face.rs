//! Registration-face add/edit form rendering.
//! 注册面新增/编辑表单渲染。

use super::super::*;

use super::face_fields::{self, draw_face_field_help, face_field_presentation, face_field_value};

pub(crate) fn draw_add(
    frame: &mut Frame<'_>,
    area: Rect,
    add: &AddState,
) -> (Rect, Rect, Rect, Rect) {
    draw_face_form(frame, area, add, " ADD REGISTRATION FACE ")
}

pub(crate) fn draw_edit(
    frame: &mut Frame<'_>,
    area: Rect,
    edit: &AddState,
) -> (Rect, Rect, Rect, Rect) {
    draw_face_form(frame, area, edit, " EDIT REGISTRATION FACE ")
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
