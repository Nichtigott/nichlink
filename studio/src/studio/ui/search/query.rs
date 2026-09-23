//! Search query bar rendering.
//! 搜索查询栏渲染。

use super::super::*;

/// Two compact query fields stay visible while comparing two files/functions.
/// 对比两个文件或函数时，两个紧凑搜索框始终可见。
pub(super) fn draw_query_bar(frame: &mut Frame<'_>, area: Rect, search: &SearchState) {
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
