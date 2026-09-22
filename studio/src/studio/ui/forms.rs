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

pub(super) fn draw_graft(
    frame: &mut Frame<'_>,
    area: Rect,
    graft: &super::super::app::GraftState,
) -> (Rect, Rect, Rect, Rect, Rect) {
    let inner = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(9), Constraint::Length(4)])
        .split(area);
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(inner[0]);

    let marker = |selected: bool| {
        if selected && graft.editing {
            "●"
        } else if selected {
            "›"
        } else {
            " "
        }
    };
    let compose = |line: String, selected: bool| {
        let style = if selected {
            Style::default().fg(Color::Black).bg(CYAN)
        } else {
            Style::default().fg(INK)
        };
        Line::from(Span::styled(line, style))
    };
    let scope = if graft.full {
        "full — replaces the subtree"
    } else {
        "cut — keeps the children"
    };
    let mut lines = vec![
        field("target", &graft.target_path),
        compose(
            format!(
                "{} selector     {}",
                marker(graft.field == 0 && graft.pane == 0),
                if graft.selector.is_empty() {
                    "_"
                } else {
                    &graft.selector
                }
            ),
            graft.pane == 0 && graft.field == 0,
        ),
        compose(
            format!(
                "{} scope        {scope}",
                marker(graft.field == 1 && graft.pane == 0)
            ),
            graft.pane == 0 && graft.field == 1,
        ),
        Line::from(""),
        Line::from(Span::styled("DECLARED SLOT", Style::default().fg(MUTED))),
    ];
    match &graft.declaration {
        super::super::app::GraftDeclaration::Declared {
            expression,
            line,
            cfg,
        } => {
            lines.push(Line::from(Span::styled(
                format!("✓ declared at line {line}: {expression}"),
                Style::default().fg(GREEN),
            )));
            if let Some(cfg) = cfg {
                lines.push(Line::from(Span::styled(
                    format!("  gated by cfg({cfg}) — the build follows that feature"),
                    Style::default().fg(MUTED),
                )));
            }
        }
        super::super::app::GraftDeclaration::Absent { entry } => {
            lines.push(Line::from(Span::styled(
                "✗ this slot is not declared by the host entry",
                Style::default().fg(Color::LightRed),
            )));
            lines.push(Line::from(Span::styled(
                format!("  {} will not ship this face", entry.display()),
                Style::default().fg(MUTED),
            )));
        }
        super::super::app::GraftDeclaration::Unknown { reason } => {
            lines.push(Line::from(Span::styled(
                format!("? {reason}"),
                Style::default().fg(MUTED),
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "PRE-FLIGHT",
        Style::default().fg(MUTED),
    )));
    if graft.flow_declared {
        lines.push(Line::from(Span::styled(
            "✓ flow contract declared",
            Style::default().fg(GREEN),
        )));
    } else {
        lines.push(Line::from(Span::styled(
            "! no flow contract: overlay requires one on both sides",
            Style::default().fg(Color::LightRed),
        )));
    }
    if graft.inherited_children > 0 {
        lines.push(Line::from(Span::styled(
            format!(
                "· {} child face(s) {}",
                graft.inherited_children,
                if graft.full {
                    "are dropped by a full cut"
                } else {
                    "are inherited by the overlay"
                }
            ),
            Style::default().fg(MUTED),
        )));
    }
    if graft
        .plans
        .iter()
        .any(|plan| plan.selector == graft.selector.trim())
    {
        lines.push(Line::from(Span::styled(
            "! that selector already exists; o opens it, d deletes it",
            Style::default().fg(Color::LightRed),
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "ENTRY DECLARATION (paste into static_graft_plan!)",
        Style::default().fg(MUTED),
    )));
    if let Some(error) = graft_declaration_error(graft) {
        lines.push(Line::from(Span::styled(error, Style::default().fg(MUTED))));
    } else {
        let document = nichlink_run_method::GraftPlanDocument::new(
            graft.target,
            graft.target_path.clone(),
            graft.selector.trim(),
            graft.full,
        );
        lines.push(Line::from(Span::styled(
            format!("  {}", document.declaration()),
            Style::default().fg(CYAN),
        )));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" COMPOSE EXTERNAL GRAFT ", GREEN)),
        body[0],
    );

    let rows = if graft.plans.is_empty() {
        vec![ListItem::new("no plans yet").style(Style::default().fg(MUTED))]
    } else {
        graft
            .plans
            .iter()
            .enumerate()
            .map(|(index, plan)| {
                let selected = graft.pane == 1 && index == graft.plan_selected;
                let scope = if plan.full { "full" } else { "cut" };
                let detail = if let Some(error) = &plan.error {
                    format!("{}  invalid: {error}", plan.selector)
                } else {
                    format!("{}  {}  {scope}", plan.selector, plan.target_path)
                };
                ListItem::new(format!(" {} {detail}", if selected { "›" } else { " " })).style(
                    if selected {
                        Style::default().fg(Color::Black).bg(CYAN)
                    } else {
                        Style::default().fg(INK)
                    },
                )
            })
            .collect()
    };
    let mut state = ListState::default().with_selected(
        (!graft.plans.is_empty()).then_some(graft.plan_selected.min(graft.plans.len() - 1)),
    );
    let border = if graft.pane == 1 { CYAN } else { MUTED };
    frame.render_stateful_widget(
        List::new(rows).block(panel(" EXISTING PLANS ", border)),
        body[1],
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
    frame.render_widget(button("Write [s]", GREEN), buttons[0]);
    frame.render_widget(button("Close [Esc]", MUTED), buttons[1]);
    frame.render_widget(button("Exit [q]", Color::LightRed), buttons[2]);
    frame.render_widget(
        Paragraph::new("↑↓ row  Tab pane  Enter edit/toggle  o open  f full  d delete")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED).bg(PANEL)),
        buttons[3],
    );
    // The two clickable compose rows: the panel's inner first line is `target`,
    // so `selector` and `scope` start one row lower.
    // 撰写区可点击的两行：面板内首行是 `target`，因此 `selector` 与 `scope` 各低一行。
    let compose = Rect::new(
        body[0].x.saturating_add(1),
        body[0].y.saturating_add(2),
        body[0].width.saturating_sub(2),
        2,
    );
    (body[1], buttons[1], buttons[0], buttons[2], compose)
}

/// A selector the entry declaration cannot be rendered for yet.
/// 还无法渲染入口声明的选择器。
fn graft_declaration_error(graft: &super::super::app::GraftState) -> Option<String> {
    nichlink_run_method::validate_graft_selector(graft.selector.trim())
        .err()
        .map(|message| format!("  ({message})"))
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
