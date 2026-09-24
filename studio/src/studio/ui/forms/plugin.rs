//! Plugin selection form rendering.
//! 插件选择表单渲染。

use super::super::*;

use crate::studio::app::PluginState;

pub(crate) fn draw_plugin(
    frame: &mut Frame<'_>,
    area: Rect,
    plugin: &PluginState,
) -> (Rect, Rect, Rect, Rect) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(9), Constraint::Length(4)])
        .split(area);
    let rows = plugin_field::LABELS
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
