//! Search result and source preview rendering.
//! 搜索结果与源码预览渲染。
//!
//! This root owns the `draw_search` layout and re-exports the pieces: the
//! query bar in `query`, the result lists in `results`, and the selected
//! symbol preview and call lane in `detail`. The source-detail page stays in
//! `super::search_detail`.
//! 本模块根承载 `draw_search` 布局并重导出各部分：查询栏在 `query`，
//! 结果列表在 `results`，选中符号预览与调用关系栏在 `detail`。
//! 源码详情页仍位于 `super::search_detail`。

use super::*;

use super::search_detail::draw_search_detail;

#[path = "search/detail.rs"]
mod detail;
use detail::{draw_search_lane, draw_search_preview};
#[path = "search/query.rs"]
mod query;
use query::draw_query_bar;
#[path = "search/results.rs"]
mod results;
use results::{SearchListOptions, draw_search_list};

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
        // Graph-mode footer. Every key below is matched in the `search.graph_mode`
        // branch of `handle_search_overlay_key`: Tab/BackTab move focus, ↑↓ move
        // the selection, Enter follows or opens, `m` loads the MIR snapshot, Esc
        // goes back. `Ctrl-W` is deliberately absent because it is only matched
        // in the list branch below.
        // 调用图页脚。下列按键都在 `handle_search_overlay_key` 的
        // `search.graph_mode` 分支匹配：Tab/BackTab 切换焦点、↑↓ 移动选择、
        // Enter 跟随或打开、`m` 载入 MIR 快照、Esc 返回。刻意不写 `Ctrl-W`，
        // 因为它只在下面的列表分支匹配。
        frame.render_widget(
            Paragraph::new("A/B: input ↓ center ↓ output   Tab focus   ↑↓ select   Enter follow/open   m MIR   Esc back")
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
            active: search.active_pane == 0,
            title: " SEARCH A ",
        },
    );
    let compare_offset = compare_area.map_or(0, |area| {
        app.hot.overlay_compare_list_area = area;
        let query = search.compare_query.as_deref().unwrap_or("");
        draw_search_list(
            frame,
            area,
            app,
            SearchListOptions {
                query,
                selected: search.compare_selected,
                offset: search.compare_offset,
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
        draw_search_detail(frame, detail_area, app, active_query, active_selected);
    } else {
        draw_search_preview(frame, detail_area, app, active_query, active_selected);
    }
    // List-mode footer (this branch is only reached when `search.graph_mode`
    // is false). Ctrl-W is matched in the list branch of
    // `handle_search_overlay_key` and opens the second pane; Enter promotes the
    // selected row into the call graph (`graph_mode = true`), so it is not an
    // edit binding. There is no `←→ fold`: `search_rows` returns a flat list
    // with no fold state, so those keys are intentionally unbound.
    // 列表页脚（只有 `search.graph_mode` 为 false 时才走到这里）。Ctrl-W 在
    // `handle_search_overlay_key` 的列表分支匹配，用于打开第二个窗格；Enter 把
    // 选中行推进调用图（`graph_mode = true`），因此不是编辑键。这里没有
    // `←→ fold`：`search_rows` 返回无折叠状态的扁平列表，这两个键有意不绑定。
    frame.render_widget(
        Paragraph::new("type to filter   Ctrl-W second pane   Tab switch   ↑↓ select   Enter call graph   Esc close")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED)),
        inner[2],
    );
    (list_area, offset, compare_offset)
}

pub(super) use crate::studio::app::{
    admission_text as format_admission, registration_rule_text as format_registration_rule,
};
