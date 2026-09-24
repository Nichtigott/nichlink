//! Drawing for the top-down call tree.
//! 自上而下调用树的绘制。
//!
//! The same three writers the horizontal canvas uses ([`put`], [`stamp`], [`paint`])
//! draw this one, so a box and a mark look the same whichever way the tree runs;
//! what changes is the route of an edge: out of the caller's left border, down the
//! margin rail that every edge shares, and into the callee's top border with a
//! `▼` above it.
//! 本画布使用与横向画布相同的三个写入函数（[`put`]、[`stamp`]、[`paint`]），因此盒子与标记
//! 不因树的走向而改变外观；不同的是边的走线：从调用者左边框出来，沿所有边共用的边距轨道向下，带
//! 着 `▼` 进入被调用者的上边框。

use super::box_draw::{Clip, Selection, draw_box, paint, put, stamp};
use super::box_vertical::VerticalCanvas;
use super::boxes::NodeBox;
use super::*;

/// Draw a laid-out top-down canvas: chrome, boxes, then edges.
/// 绘制已排好版的自上而下画布：说明行、盒子，然后边。
pub(super) fn draw_vertical(
    canvas: &VerticalCanvas,
    frame: &mut Frame<'_>,
    tree: &CallTreeView,
    selection: Selection<'_>,
) {
    put(
        canvas.clip(),
        frame,
        canvas.body.x,
        canvas.body.y,
        "↑ callers · call direction · callees ↓",
        Style::default().fg(MUTED),
    );
    for drawn in &canvas.boxes {
        let label = canvas
            .names_band(drawn)
            .then(|| VerticalCanvas::band_label(drawn.level));
        draw_box(canvas.clip(), frame, tree, drawn, selection, label);
    }
    draw_edges_down(canvas, frame, tree);
}

/// Route every edge down the rail from its caller to its callee.
/// 让每条边沿轨道从调用者向下走到它的被调用者。
///
/// The rail is one margin column, so edges that share it merge into a junction
/// instead of fighting for the same cells, and every box keeps the full panel
/// width for its name. The last step is a short horizontal run along the blank
/// row above the callee, ending in `▼` on the callee's top border.
/// 轨道只占一列边距，因此共用它的边会合并成路口而不是争抢同一批单元格，而每个盒子都把面板整宽
/// 留给自己的名字。最后一步是沿被调用者上方那条空行的短横走，以 `▼` 收在被调用者的上边框上。
fn draw_edges_down(canvas: &VerticalCanvas, frame: &mut Frame<'_>, tree: &CallTreeView) {
    let clip = canvas.clip();
    let line = Style::default().fg(MUTED);
    let rail = canvas.body.x;
    let mut entries: Vec<(usize, u16)> = Vec::new();
    for edge in &tree.tree.edges {
        let (Some(caller), Some(callee)) = (
            box_of_vertical(canvas, edge.caller),
            box_of_vertical(canvas, edge.callee),
        ) else {
            continue;
        };
        // Downwards is the call direction, so an edge that would have to run
        // upwards is left to the status row instead of drawn against the axis.
        // 向下才是调用方向，因此必须向上走的边交给状态行说明，而不是逆着轴画。
        if caller.rect.y >= callee.rect.y {
            continue;
        }
        let from_y = caller.rect.y + caller.rect.height / 2;
        let approach = callee.rect.y.saturating_sub(1);
        let to_x = callee.rect.x + callee.rect.width / 2;
        let lane = entries
            .iter()
            .filter(|(callee, row)| *callee == edge.callee && *row == approach)
            .count()
            .min(2) as u16;
        entries.push((edge.callee, approach));
        let arrow_x = to_x.saturating_sub(lane);
        let mark_x = arrow_x.saturating_sub(1);
        // Port on the caller's left border, then out to the rail and down.
        // 在调用者左边框留下端口，然后出到轨道并向下。
        stamp(clip, frame, caller.rect.x, from_y, '─', line);
        for x in rail..caller.rect.x {
            stamp(clip, frame, x, from_y, '─', line);
        }
        for y in from_y..=approach {
            stamp(clip, frame, rail, y, '│', line);
        }
        for x in rail..mark_x {
            stamp(clip, frame, x, approach, '─', line);
        }
        // The port on the callee's top border, then the head just above it.
        // 被调用者上边框上的端口，然后紧挨它上方画箭头。
        stamp(clip, frame, arrow_x, callee.rect.y, '│', line);
        paint(
            clip,
            frame,
            mark_x,
            approach,
            edge.evidence.marker().chars().next().unwrap_or('?'),
            Style::default().fg(if edge.evidence.confirmed() {
                GREEN
            } else {
                MUTED
            }),
        );
        paint(
            clip,
            frame,
            arrow_x,
            approach,
            '▼',
            Style::default().fg(CYAN),
        );
    }
}

impl VerticalCanvas {
    /// The clipping rectangles of the top-down canvas.
    /// 自上而下画布的裁剪矩形。
    pub(super) fn clip(&self) -> Clip {
        Clip {
            body: self.body,
            view: self.view,
        }
    }
}

/// The box that draws one node in the top-down canvas.
/// 自上而下画布中绘制某个节点的盒子。
fn box_of_vertical(canvas: &VerticalCanvas, index: usize) -> Option<&NodeBox> {
    canvas.boxes.iter().find(|drawn| drawn.index == index)
}
