//! Layered call-tree drawing for the call-graph overlay.
//! 调用图浮层的分层调用树绘制。
//!
//! The panel draws the call direction as its horizontal axis: the focus owns the
//! middle column, callers fill the columns to its left, callees the columns to
//! its right, and every edge that runs along the axis is therefore drawn left to
//! right and ends in `→` on the callee. The reader gets the axis sentence, a
//! level ruler, one row per lane, and — when a budget cut something — the count
//! of what is missing written on the node itself, because an overview that hides
//! its own gaps is worse than no overview.
//! 本面板以调用方向为横轴：焦点占中间一列，调用者填它左边的列，被调用者填右边，因此每条沿轴
//! 的边都从左向右画，并在被调用者一侧以 `→` 收尾。读者会看到轴说明、层标尺、每条车道一行，
//! 以及预算切掉东西时写在节点自身上的缺失计数——因为一个藏起自己缺口的鸟瞰图比没有更糟。
//!
//! Two things this replaced are worth naming. The outline that used to live here
//! showed the selected function's `input` / `transform` / `output` sections; the
//! tree shows structure instead, so the signature and the first transform moved
//! to the panel's last row and the rest is gone. And the tree is a *view of the
//! focus*: nodes that the budgets or the direction rule left out are counted,
//! never silently dropped.
//! 这里被取代的两件事值得点名。此前的大纲显示所选函数的 `input` / `transform` /
//! `output` 分节；树展示的是结构，因此签名与第一条 transform 移到了面板最后一行，其余
//! 不再显示。另外，树是*焦点的视图*：被预算或方向规则排除在外的节点会被计数，绝不静默丢弃。

use super::*;

use super::cells::{
    COL_W, STRIDE, between, first_visible_level, incoming_off_axis, node_label, put, row_offset,
    visible_columns,
};

/// Rows the panel spends on the axis sentence, the ruler and the status line.
/// 面板花在轴说明、标尺与状态行上的行数。
const CHROME: u16 = 3;

pub(super) fn draw_call_tree(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    selected: Option<&CallRef>,
    cursor: usize,
    focused: bool,
    title: &str,
) {
    let Some(item) = selected else {
        frame.render_widget(
            Paragraph::new("Select a call node.").block(panel(title, MUTED)),
            area,
        );
        return;
    };
    let view = app.call_tree_view(item);
    // The budget belongs in the panel title: the border row is otherwise empty,
    // and a count that shares a row with the ruler gets clipped by it.
    // 预算放在面板标题里：边框行本来是空的，而和标尺共用一行的计数会被标尺裁掉。
    let depth = view
        .tree
        .nodes
        .iter()
        .map(|node| node.level.unsigned_abs())
        .max()
        .unwrap_or(0);
    let mut heading = format!("{title} · depth {depth} · {} nodes", view.len());
    if view.tree.truncated {
        heading.push_str(" · truncated");
    }
    frame.render_widget(panel(heading, if focused { GREEN } else { MAGENTA }), area);
    if view.is_empty() {
        return;
    }
    let cursor = cursor.min(view.len().saturating_sub(1));
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    if inner.height < CHROME + 1 || inner.width < COL_W {
        // Below this size a tree is a wall of clipped labels; the border and the
        // panel title are the honest answer.
        // 小于这个尺寸时树只会是一堵裁剩的标签墙；边框与面板标题才是诚实的答案。
        return;
    }
    let cursor_style = Style::default()
        .fg(Color::Black)
        .bg(if focused { GREEN } else { CYAN })
        .add_modifier(Modifier::BOLD);
    let node = &view.tree.nodes[cursor];
    let first = first_visible_level(&view, node.level, inner.width);
    let body_rows = inner.height - CHROME;
    let first_body_y = inner.y + CHROME - 1;
    let offset = row_offset(&view, node.lane, body_rows);
    let column_x = |level: i32| -> Option<u16> {
        let step = level - first;
        if step < 0 || step as u16 >= visible_columns(inner.width) {
            return None;
        }
        Some(inner.x + step as u16 * STRIDE)
    };
    let row_y = |lane: usize| -> Option<u16> {
        let step = lane.checked_sub(offset)?;
        (step < body_rows as usize).then_some(first_body_y + step as u16)
    };

    // MIR candidates are the closest thing Studio has to the prototype's
    // "undecided": the compiler said these functions call each other, and no
    // live observation confirmed it. They are marked, never merged into the
    // edges the axis draws.
    // MIR 候选是 Studio 里最接近原型"未决"的东西：编译器说这些函数互相调用，而没有实时观测
    // 确认。它们只被标记，绝不并入轴所画的那些边。
    let uncertain = view
        .refs
        .iter()
        .map(|item| {
            item.as_ref()
                .is_some_and(|item| !app.mir_candidates_for(&item.function).is_empty())
        })
        .collect::<Vec<bool>>();
    draw_edges(frame, inner, &view, &column_x, &row_y, first_body_y);
    for (index, candidate) in view.tree.nodes.iter().enumerate() {
        let (Some(x), Some(y)) = (column_x(candidate.level), row_y(candidate.lane)) else {
            continue;
        };
        let label = node_label(
            candidate,
            incoming_off_axis(&view, index),
            uncertain.get(index).copied().unwrap_or(false),
        );
        let style = if index == cursor {
            cursor_style
        } else if candidate.level == 0 {
            Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(INK)
        };
        put(frame, inner, x, y, &label, style, inner.right());
    }
    let candidates = uncertain.iter().filter(|flag| **flag).count();
    draw_chrome(frame, inner, &view, first, &column_x, candidates);
    draw_status(frame, inner, app, &view, cursor, inner.y + inner.height - 1);
}

/// Draw the axis sentence and the level ruler.
/// 绘制轴说明与层标尺。
fn draw_chrome(
    frame: &mut Frame<'_>,
    inner: Rect,
    view: &CallTreeView,
    first: i32,
    column_x: &impl Fn(i32) -> Option<u16>,
    candidates: usize,
) {
    // The sentence has to fit the panel, because a clipped axis sentence is a
    // missing axis: whatever is dropped is the direction the reader needed.
    // 这句话必须放得下面板，因为被裁掉的轴说明等于没有轴：丢掉的那一段正是读者需要的那段。
    put(
        frame,
        inner,
        inner.x,
        inner.y,
        "← callers · call direction · callees →",
        Style::default().fg(MUTED),
        inner.right(),
    );
    let mut ruler_text = String::new();
    if candidates > 0 {
        ruler_text.push_str(&format!("? = {candidates} with MIR candidates"));
    }
    let ruler_x = inner
        .right()
        .saturating_sub(ruler_text.chars().count() as u16)
        .max(inner.x);
    if !ruler_text.is_empty() {
        put(
            frame,
            inner,
            ruler_x,
            inner.y + 1,
            &ruler_text,
            Style::default().fg(MUTED),
            inner.right(),
        );
    }
    let ruler = Style::default().fg(MUTED);
    let (_, high) = view.tree.level_span();
    for level in first..=high {
        let Some(x) = column_x(level) else {
            continue;
        };
        let text = if level == 0 {
            "focus".to_owned()
        } else {
            format!("{level:+}")
        };
        if x + text.chars().count() as u16 > ruler_x {
            break;
        }
        put(frame, inner, x, inner.y + 1, &text, ruler, inner.right());
    }
}

/// Draw the status row: the cursor's signature and its first transform, the two
/// things the outline used to show for the selected function.
/// 绘制状态行：游标的签名与它的第一条 transform，也就是大纲此前为所选函数展示的两件事。
fn draw_status(
    frame: &mut Frame<'_>,
    inner: Rect,
    app: &App,
    view: &CallTreeView,
    cursor: usize,
    y: u16,
) {
    let Some(item) = view.item(cursor) else {
        return;
    };
    let mut status = format!("ƒ {}", signature_of(&item));
    if let Some(transform) = app.call_tree_transforms(&item).first() {
        status.push_str(&format!("   ↳ {transform}"));
    }
    // The badges are explained where the cursor is, so they do not need a
    // legend that would cost a whole row.
    // 徽标就在游标所在处解释，因此不需要一条会占掉整行的图例。
    let node = &view.tree.nodes[cursor];
    if node.cut_callers > 0 {
        status.push_str(&format!("   ←{} caller(s) not drawn", node.cut_callers));
    }
    if node.cut_callees > 0 {
        status.push_str(&format!("   →{} callee(s) not drawn", node.cut_callees));
    }
    if incoming_off_axis(view, cursor) {
        status.push_str("   ◀ off-axis edge");
    }
    put(
        frame,
        inner,
        inner.x,
        y,
        &status,
        Style::default().fg(MUTED),
        inner.right(),
    );
}

/// The declared signature of one function, read from its source.
/// 某个函数的声明签名，从其源码读出。
fn signature_of(item: &CallRef) -> String {
    let Ok(text) = std::fs::read_to_string(source_path_for(&item.file)) else {
        return item.function.clone();
    };
    text.lines()
        .find(|line| line.contains(&format!("{}(", item.function)))
        .map(|line| line.trim().trim_end_matches('{').trim().to_owned())
        .unwrap_or_else(|| item.function.clone())
}

/// Draw every edge that runs along the axis, in the gap after its caller.
/// 在调用者之后的空隙里，绘制每条沿轴的边。
fn draw_edges(
    frame: &mut Frame<'_>,
    inner: Rect,
    view: &CallTreeView,
    column_x: &impl Fn(i32) -> Option<u16>,
    row_y: &impl Fn(usize) -> Option<u16>,
    first_body_y: u16,
) {
    for edge in &view.tree.edges {
        if !edge.forward {
            continue;
        }
        let caller = &view.tree.nodes[edge.caller];
        let callee = &view.tree.nodes[edge.callee];
        // Only a hop of exactly one column has room for a line: anything longer
        // would run through the labels of the columns in between.
        // 只有恰好跨一列的跳数才有画线的空间：更长的线会穿过中间那些列的标签。
        if callee.level != caller.level + 1 {
            continue;
        }
        let (Some(from), Some(to), Some(y)) = (
            column_x(caller.level),
            column_x(callee.level),
            row_y(callee.lane),
        ) else {
            continue;
        };
        if y < first_body_y {
            continue;
        }
        let stem = from + COL_W;
        if let Some(caller_y) = row_y(caller.lane).filter(|caller_y| *caller_y != y) {
            for bend_y in between(caller_y, y) {
                put(
                    frame,
                    inner,
                    stem,
                    bend_y,
                    "│",
                    Style::default().fg(MUTED),
                    inner.right(),
                );
            }
        }
        for offset in 0..2 {
            put(
                frame,
                inner,
                stem + offset,
                y,
                "─",
                Style::default().fg(MUTED),
                inner.right(),
            );
        }
        put(
            frame,
            inner,
            stem + 2,
            y,
            edge.evidence.marker(),
            Style::default().fg(if edge.evidence.confirmed() {
                GREEN
            } else {
                MUTED
            }),
            inner.right(),
        );
        put(
            frame,
            inner,
            to.saturating_sub(1),
            y,
            "→",
            Style::default().fg(CYAN),
            inner.right(),
        );
    }
}
