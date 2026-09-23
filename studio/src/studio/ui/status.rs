//! Event log panel and the footer key help.
//! 事件日志面板与页脚按键提示。

use super::*;

pub(super) fn draw_event(frame: &mut Frame<'_>, area: Rect, app: &App) {
    // Two ways an event asks for the warning colour, and both are explicit in the
    // text the code sets: a `Warning:` prefix, or a failure being reported. The
    // prefix exists so a message that is a warning *and* an ordinary successful
    // action — "the plan was written, but nothing declares its slot" — can still
    // be shown as one, instead of the severity having to be smuggled in through
    // the wording.
    // 事件请求警示色有两条途径，且都写在其文本里：`Warning:` 前缀，或正在报告一次
    // 失败。前缀的存在是为了让“既是警告、又是一次普通成功动作”的消息（“计划写好了，
    // 但没有任何声明命名它的槽位”）也能按警告显示，而不必把严重程度偷偷塞进措辞里。
    let event_is_warning =
        app.event.starts_with("Warning:") || app.event.to_ascii_lowercase().contains("fail");
    let color = if event_is_warning {
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

pub(super) fn draw_keys(frame: &mut Frame<'_>, area: Rect) {
    // This is the persistent workspace footer, so it lists exactly the keys
    // that `App::handle_key` handles on the Inspect/workspace page. `m MIR` is
    // deliberately absent: it is only matched inside the call-graph branch of
    // `handle_search_overlay_key`, and the graph footer in `super::search`
    // advertises it instead. Advertising it here made the footer lie whenever
    // no search overlay was open.
    // 这是常驻工作区页脚，只列出 `App::handle_key` 在 Inspect/工作区页处理的
    // 按键。这里刻意不写 `m MIR`：它只在 `handle_search_overlay_key` 的调用图
    // 分支里匹配，改由 `super::search` 的调用图页脚展示。写在这里会在没有搜索
    // 浮层时谎报按键。
    frame.render_widget(
        Paragraph::new(" 1 search   2 inspect   3 data   4 compare   q quit   / search   n new   a add   g graft   p plugin   e edit   d delete   Enter fold   r/F5 reload   b/F9 build   ←/→ resize ")
            .alignment(Alignment::Center)
            .style(Style::default().fg(MUTED)),
        area,
    );
}
