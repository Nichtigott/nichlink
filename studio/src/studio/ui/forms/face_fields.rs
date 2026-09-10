//! Registration-face field labels, defaults, and contextual help.
//! 注册面字段标签、默认值与上下文帮助。
//!
//! The static field dictionary lives in the kernel `authoring` module; this
//! shim keeps the Studio-facing paths and hosts the AddState-bound rendering.
//! 静态字段词典本体在 kernel 的 `authoring` 模块；本 shim 保留 Studio 侧
//! 路径，并承载绑定 AddState 的渲染逻辑。

use super::*;

pub(super) use nichlink_run_method::{FaceFieldRole, face_field_default, face_field_presentation};

pub(super) fn face_field_value(add: &AddState, index: usize) -> String {
    let stored = add.values[index].trim();
    match (index, stored) {
        (2, "true") => "[x]".to_owned(),
        (2, "false" | "") => "[ ] <default>".to_owned(),
        (4 | 5, "ANY") => "<default: ANY>".to_owned(),
        (6, "NoParts") => "<default: NoParts>".to_owned(),
        (13, "NoPreset") => "<default: NoPreset>".to_owned(),
        (24 | 25, "()") => "<default: ()>".to_owned(),
        (_, "") => face_field_default(&add.values, index),
        _ => stored.to_owned(),
    }
}

pub(super) fn draw_face_field_help(frame: &mut Frame<'_>, area: Rect, add: &AddState) {
    let field = face_field_presentation(add.field);
    let parent_requirement = add.parent_requirements.get(&add.field);
    let role = if add.locked_fields.contains(&add.field) {
        FaceFieldRole::ReadOnly
    } else if parent_requirement.is_some() {
        FaceFieldRole::Required
    } else {
        field.role
    };
    let effective = face_field_value(add, add.field);
    let lines = vec![
        Line::from(Span::styled(
            field.group,
            Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            field.label,
            Style::default().fg(INK).add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("status   ", Style::default().fg(MUTED)),
            Span::styled(
                role.label(),
                Style::default().fg(if matches!(role, FaceFieldRole::Required) {
                    Color::LightRed
                } else {
                    CYAN
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled("effective", Style::default().fg(MUTED)),
            Span::raw(format!("  {effective}")),
        ]),
        Line::from(vec![
            Span::styled("default  ", Style::default().fg(MUTED)),
            Span::raw(format!("  {}", field.default)),
        ]),
        Line::from(vec![
            Span::styled("parent   ", Style::default().fg(MUTED)),
            Span::raw(format!(
                "  {}",
                parent_requirement.map_or("no added requirement", String::as_str)
            )),
        ]),
        Line::from(""),
        Line::from(field.help),
        Line::from(""),
        Line::from(Span::styled(
            "Default and derived values are omitted from generated source.",
            Style::default().fg(MUTED),
        )),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" FIELD GUIDE ", CYAN)),
        area,
    );
}
