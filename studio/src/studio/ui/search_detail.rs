//! Search details rendering.
//! 搜索详情渲染。

use super::*;

pub(super) fn draw_search_detail(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    query: &str,
    selected: usize,
) {
    let rows = app.search_rows(query);
    let Some(row) = rows.get(selected) else {
        frame.render_widget(
            Paragraph::new("No matching registration face.").block(panel(" DETAILS ", MUTED)),
            area,
        );
        return;
    };
    let Some(node) = row.node else {
        frame.render_widget(
            Paragraph::new(row.text.as_str()).block(panel(" DETAILS ", MUTED)),
            area,
        );
        return;
    };
    let Some(info) = app.registry.find(node) else {
        return;
    };
    // One row per fact. `path` is already the source file, so a combined
    // file:line:function row would only repeat it; and `kind`, its handle and
    // its parameter metadata are one string — both macros expand `params` and
    // `handle` from `stringify!($kind)` — so three rows stated it three times.
    // 一个事实一行。`path` 本身就是源文件，再拼一行 file:line:function 只是重复它；
    // 而 `kind`、它的 handle 与参数元数据是同一个字符串——两个宏都用
    // `stringify!($kind)` 展开 `params` 与 `handle`——三行说了三遍同一件事。
    let values = [
        ("path", row.path.clone()),
        ("function", row.function.clone()),
        ("face node", info.id.to_string()),
        ("kind", info.kind.clone()),
        (
            "preset / parts",
            format!("{} / {}", info.preset, info.parts),
        ),
        ("needs registry", info.needs_registry.to_string()),
        (
            "registration rule",
            super::format_registration_rule(&info.registry_rule),
        ),
        ("admission", super::format_admission(&info.admission)),
        ("summary", info.summary.en.to_owned()),
        (
            "other registry",
            info.getting_from_other_registry
                .clone()
                .unwrap_or_else(|| "-".to_owned()),
        ),
        ("exports", info.exports.join(", ")),
        (
            "requires",
            info.requires
                .iter()
                .map(|item| format!("{} <- {}", item.capability, item.provider))
                .collect::<Vec<_>>()
                .join(", "),
        ),
        ("provides", info.provides.join(", ")),
    ];
    let lines = values
        .iter()
        .map(|(name, value)| search_field(name, value, query))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .block(panel(" DETAILS / REGISTRATION FACE ", GREEN)),
        area,
    );
}

fn search_field(name: &str, value: &str, query: &str) -> Line<'static> {
    let highlighted = !query.trim().is_empty()
        && value
            .to_ascii_lowercase()
            .contains(&query.to_ascii_lowercase());
    Line::from(vec![
        Span::styled(format!("{name:<15}"), Style::default().fg(MUTED)),
        Span::styled(
            value.to_owned(),
            if highlighted {
                Style::default().fg(MAGENTA).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(INK)
            },
        ),
    ])
}
