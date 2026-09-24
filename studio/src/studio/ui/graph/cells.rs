//! Cell geometry and text fitting for the call-tree drawing.
//! 调用树绘制的单元格几何与文本适配。
//!
//! These are the parts of the drawing that can be decided without knowing what a
//! call is: how many columns fit, which of them the window shows, where a lane
//! lands, and how a label is cut to fit. They live apart from the drawing itself
//! so the drawing stays about calls — direction, evidence, and what the budgets
//! left out.
//! 这些是无需知道"调用"是什么就能决定的部分：放得下几列、窗口显示哪几列、某条车道落在哪里、
//! 标签怎样裁才放得下。它们与绘制本身分开，好让绘制只关心调用——方向、证据，以及预算漏掉了
//! 什么。

use super::*;

/// Cells one node label occupies.
/// 一个节点标签占用的单元格数。
pub(super) const COL_W: u16 = 15;
/// Cells between two columns: they hold the edge, its evidence marker and `→`.
/// 两列之间的单元格：容纳边、它的证据标记与 `→`。
pub(super) const GAP: u16 = 5;
/// Column pitch.
/// 列间距。
pub(super) const STRIDE: u16 = COL_W + GAP;

/// Columns that fit side by side.
/// 可以并排放下的列数。
pub(super) fn visible_columns(width: u16) -> u16 {
    ((width + GAP) / STRIDE).max(1)
}

/// Leftmost level the window shows, keeping the cursor's column in view.
/// 窗口显示的最左层，并让游标所在列保持可见。
pub(super) fn first_visible_level(view: &CallTreeView, level: i32, width: u16) -> i32 {
    let (low, high) = view.tree.level_span();
    let columns = visible_columns(width) as i32;
    if high - low < columns {
        return low;
    }
    (level - columns / 2).clamp(low, high - columns + 1)
}

/// First lane the window shows, keeping the cursor's row in view.
/// 窗口显示的第一条车道，并让游标所在行保持可见。
pub(super) fn row_offset(view: &CallTreeView, lane: usize, rows: u16) -> usize {
    let total = view.tree.rows();
    if total <= rows as usize {
        return 0;
    }
    lane.saturating_sub(rows as usize / 2)
        .min(total - rows as usize)
}

/// Whether a node has an incoming edge the axis could not draw.
/// 某个节点是否有无法沿轴画出的入边。
pub(super) fn incoming_off_axis(view: &CallTreeView, index: usize) -> bool {
    view.tree
        .edges
        .iter()
        .any(|edge| edge.callee == index && !edge.forward)
}

/// The label one node renders as, badges included.
/// 一个节点渲染出的标签，含徽标。
pub(super) fn node_label(node: &CallTreeNode, off_axis: bool, uncertain: bool) -> String {
    let mut badges = String::new();
    if node.cut_callers > 0 {
        badges.push_str(&format!("←{}", node.cut_callers));
    }
    if node.cut_callees > 0 {
        badges.push_str(&format!("→{}", node.cut_callees));
    }
    if uncertain {
        badges.push('?');
    }
    let head = usize::from(off_axis);
    let badge_cells = badges.chars().count() + usize::from(!badges.is_empty());
    let room = (COL_W as usize).saturating_sub(badge_cells + head);
    let marker = if off_axis { "◀" } else { "" };
    format!("{marker}{}{badges}", clip(&node.symbol, room))
}

/// A qualified name keeps its last segment (`RelativeCo…::to_screen`), because
/// the object is the context and the function is the point. A bare name keeps
/// its *tail*: `preview_canvas_width` and `preview_canvas_width_traced` share a
/// prefix, so keeping the head would draw two identical labels and the reader
/// would have no way to tell which column held which.
/// 限定名保住所属对象之后的最后一段（`RelativeCo…::to_screen`），因为对象是上下文、函数才是
/// 重点。裸名则保留*尾部*：`preview_canvas_width` 与 `preview_canvas_width_traced` 共享前缀，
/// 保头会画出两个一模一样的标签，读者无从分辨哪一列是哪个。
pub(super) fn clip(text: &str, max: usize) -> String {
    let characters = text.chars().collect::<Vec<_>>();
    if characters.len() <= max {
        return text.to_owned();
    }
    let keep = max.saturating_sub(1);
    let tail_of = |count: usize| -> String {
        characters[characters.len() - count..]
            .iter()
            .collect::<String>()
    };
    match text.rfind("::") {
        Some(index) => {
            let tail = text[index..].chars().count();
            if tail + 2 <= max {
                let head = max.saturating_sub(tail + 1);
                format!(
                    "{}…{}",
                    characters[..head].iter().collect::<String>(),
                    &text[index..]
                )
            } else {
                format!("…{}", tail_of(keep))
            }
        }
        None => format!("…{}", tail_of(keep)),
    }
}

/// Rows strictly between two lanes, so a bend reads as a bend.
/// 两条车道之间的行（不含端点），使转折读起来像转折。
pub(super) fn between(from: u16, to: u16) -> std::ops::RangeInclusive<u16> {
    // Only called for two different lanes; equal lanes need no bend.
    // 仅在两条车道不同时调用；车道相同不需要转折。
    if from < to {
        from + 1..=to - 1
    } else {
        to + 1..=from - 1
    }
}

/// Write one line of text into the frame buffer, clipped to the panel.
/// 把一行文本写入帧缓冲，并裁剪到面板范围内。
pub(super) fn put(
    frame: &mut Frame<'_>,
    area: Rect,
    x: u16,
    y: u16,
    text: &str,
    style: Style,
    limit: u16,
) {
    let limit = limit.min(area.right());
    for (column, character) in (x..).zip(text.chars()) {
        if column >= limit || !area.contains((column, y).into()) {
            break;
        }
        let cell = &mut frame.buffer_mut()[(column, y)];
        cell.set_char(character);
        cell.set_style(style);
    }
}
