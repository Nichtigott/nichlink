//! Top-down geometry for the call tree's node boxes.
//! 调用树节点盒的自上而下几何。
//!
//! A terminal is wide in rows and poor in columns, and the horizontal canvas
//! needs one column per hop: on an 80-column pane the call tree gets 19 cells, so
//! two boxes have to shrink to seven cells each and their names to tails. The same
//! tree laid out *downwards* costs one margin column for the edges and then lets
//! every box use the whole width, one band per hop — callers above, callees below,
//! which is how a call tree is drawn everywhere else.
//! 终端行多列少，而横向画布每一跳要占一整列：80 列的面板上调用树只有 19 格，于是两个盒子各缩到
//! 七格、名字只剩尾部。同一棵树**竖着**排只需为边走线留一列边距，然后每个盒子都能用满整宽，
//! 一跳一条带——调用者在上、被调用者在下，这本来就是别处画调用树的方式。

use std::collections::BTreeMap;

use super::boxes::{MAX_W, MIN_W, NodeBox, label_rows};
use super::*;

/// Margin columns reserved for the vertical rail the edges run along.
/// 为边走线的竖直轨道预留的边距列数。
const RAIL: u16 = 2;
/// Rows the panel keeps for itself: the axis sentence and the status row.
/// 面板留给自己的行：轴说明与状态行。
const CHROME: u16 = 2;

/// The boxes of one top-down tree, with the band each level occupies.
/// 一棵自上而下的树的盒子，以及每一层占用的带。
pub(super) struct VerticalCanvas {
    /// Boxes that fit, in node order.
    /// 放得下的盒子，按节点顺序。
    pub(super) boxes: Vec<NodeBox>,
    /// The panel's inner rectangle.
    /// 面板内部矩形。
    pub(super) body: Rect,
    /// The part of the body the boxes may use: below the axis, above the status.
    /// 盒子可用的部分：轴说明之下、状态行之上。
    pub(super) view: Rect,
}

impl VerticalCanvas {
    /// Place the bands for one tree around `cursor`, or `None` when the panel is
    /// too small for a box at all.
    /// 围绕 `cursor` 为一条树摆放各带；面板小到放不下一个盒子时返回 `None`。
    pub(super) fn layout(tree: &CallTreeView, body: Rect, cursor: usize) -> Option<Self> {
        let nodes = &tree.tree.nodes;
        let cursor = cursor.min(nodes.len().checked_sub(1)?);
        if body.height < CHROME + 3 || body.width < RAIL + MIN_W {
            return None;
        }
        let view = Rect {
            x: body.x,
            y: body.y.saturating_add(1),
            width: body.width,
            height: body.height.saturating_sub(CHROME),
        };
        let width = nodes
            .iter()
            .map(|node| (name_cells(&node.symbol) + 2).clamp(MIN_W, MAX_W))
            .max()
            .unwrap_or(MIN_W)
            .min(body.width.saturating_sub(RAIL))
            .max(MIN_W);
        // One band per level, upstream first: the reader follows the call
        // direction downwards, so a caller is always drawn above its callee.
        // 每层一条带，上游在前：读者沿调用方向向下读，因此调用者总画在它的被调用者之上。
        let mut levels = nodes.iter().map(|node| node.level).collect::<Vec<_>>();
        levels.sort_unstable();
        levels.dedup();
        let heights = band_heights(nodes, &levels, width);
        let mut tops = BTreeMap::new();
        let mut y = view.y;
        for level in &levels {
            tops.insert(*level, y);
            y = y.saturating_add(heights[level]).saturating_add(1);
        }
        // The cursor's own band is what has to stay in view, so the scroll is
        // exactly the overflow below it.
        // 必须留在视野里的是游标自己那条带，因此滚动量正是它下方溢出的部分。
        let cursor_level = nodes[cursor].level;
        let shift = tops[&cursor_level]
            .saturating_add(heights[&cursor_level])
            .saturating_sub(view.y)
            .saturating_sub(view.height);
        let mut boxes = Vec::new();
        for (index, node) in nodes.iter().enumerate() {
            let Some(top) = tops.get(&node.level).map(|top| top.saturating_sub(shift)) else {
                continue;
            };
            if top < view.y {
                continue;
            }
            let rows = box_rows(node, width);
            if top.saturating_add(rows) > view.bottom() {
                continue;
            }
            boxes.push(NodeBox {
                index,
                level: node.level,
                rect: Rect {
                    x: view.x.saturating_add(RAIL),
                    y: top,
                    width,
                    height: rows,
                },
            });
        }
        // A band's first drawn box carries the level on its title, so the margin
        // stays free for the rail.
        // 每条带第一个画出的盒子把层号写在标题上，因此边距留给轨道。
        Some(Self { boxes, body, view })
    }

    /// Every box on screen with the node it draws, for hit testing.
    /// 屏幕上每个盒子及其绘制的节点，用于命中测试。
    pub(super) fn hits(&self) -> Vec<(Rect, usize)> {
        self.boxes
            .iter()
            .map(|drawn| (drawn.rect, drawn.index))
            .collect()
    }

    /// The level label a band's first box shows on its border.
    /// 一条带的第一个盒子写在边框上的层号标签。
    pub(super) fn band_label(level: i32) -> String {
        if level == 0 {
            "focus".to_owned()
        } else {
            format!("{level:+}")
        }
    }

    /// Whether this box is the first of its band, i.e. the one that names it.
    /// 该盒子是否是本带第一个，也就是为这条带命名的那一个。
    pub(super) fn names_band(&self, drawn: &NodeBox) -> bool {
        self.boxes
            .iter()
            .find(|candidate| candidate.level == drawn.level)
            .is_some_and(|first| first.index == drawn.index)
    }
}

/// The rows one band needs: its boxes stacked, a blank row between them.
/// 一条带需要的行数：其中各盒子堆叠，盒子之间留一条空行。
fn band_heights(nodes: &[CallTreeNode], levels: &[i32], width: u16) -> BTreeMap<i32, u16> {
    let mut heights = BTreeMap::new();
    for level in levels {
        let mut rows = 0u16;
        for node in nodes.iter().filter(|node| node.level == *level) {
            rows = rows.saturating_add(box_rows(node, width)).saturating_add(1);
        }
        heights.insert(*level, rows.saturating_sub(1));
    }
    heights
}

/// The rows one box needs: borders, its label rows and a row for its badges.
/// 一个盒子需要的行数：边框、标签各行，以及一行给徽标。
fn box_rows(node: &CallTreeNode, width: u16) -> u16 {
    let content = width.saturating_sub(2).max(1);
    let rows = label_rows(&node.symbol, content).len() + usize::from(!badges(node).is_empty());
    (rows as u16).saturating_add(2).max(3)
}

/// The badges a box shows, without the wire-side mark the horizontal canvas adds.
/// 盒子展示的徽标，不含横向画布会加的那一侧标记。
fn badges(node: &CallTreeNode) -> String {
    super::boxes::badges(node, false)
}

/// The cells a name takes on screen.
/// 名字在屏幕上占用的单元格数。
fn name_cells(name: &str) -> u16 {
    name.chars().count() as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tree the shape of the fixture's: one caller, the focus, one callee.
    /// 与夹具同形的树：一个调用者、焦点、一个被调用者。
    fn tree() -> CallTreeView {
        let mut view = CallTreeView::default();
        view.tree.nodes = vec![
            CallTreeNode {
                symbol: "preview_canvas_width".to_owned(),
                level: 0,
                lane: 0,
                parent: None,
                cut_callers: 0,
                cut_callees: 0,
            },
            CallTreeNode {
                symbol: "preview_canvas_width_traced".to_owned(),
                level: -1,
                lane: 0,
                parent: Some(0),
                cut_callers: 0,
                cut_callees: 0,
            },
            CallTreeNode {
                symbol: "clamp_canvas_width".to_owned(),
                level: 1,
                lane: 0,
                parent: Some(0),
                cut_callers: 1,
                cut_callees: 0,
            },
        ];
        view.refs = vec![None; view.tree.nodes.len()];
        view
    }

    /// Upstream is above: bands are ordered by level, they do not overlap, and
    /// every box uses the same width so the rail has a straight column to run in.
    /// 上游在上：各带按层号排列、互不重叠，且每个盒子同宽，使轨道有一条笔直的列可走。
    #[test]
    fn bands_stack_upstream_first_at_one_width() {
        let view = tree();
        let canvas = VerticalCanvas::layout(
            &view,
            Rect {
                x: 0,
                y: 0,
                width: 40,
                height: 24,
            },
            0,
        )
        .expect("the panel fits three bands");
        assert_eq!(canvas.hits().len(), 3, "every node is drawn");
        let mut tops: Vec<(i32, u16)> = canvas
            .hits()
            .into_iter()
            .map(|(rect, index)| (view.tree.nodes[index].level, rect.y))
            .collect();
        tops.sort_unstable();
        assert_eq!(
            tops.iter().map(|(level, _)| *level).collect::<Vec<_>>(),
            vec![-1, 0, 1],
            "upstream first"
        );
        assert!(
            tops.windows(2).all(|pair| pair[0].1 < pair[1].1),
            "a caller is drawn above its callee: {tops:?}"
        );
        let widths = canvas
            .hits()
            .into_iter()
            .map(|(rect, _)| rect.width)
            .collect::<Vec<_>>();
        assert!(
            widths.windows(2).all(|pair| pair[0] == pair[1]),
            "one width keeps the rail straight: {widths:?}"
        );
        assert_eq!(
            canvas.body.x + RAIL,
            canvas.hits()[0].0.x,
            "the rail is kept"
        );
    }

    /// A panel too short for every band scrolls to the cursor's own band instead
    /// of showing the top of the tree.
    /// 放不下所有带的矮面板滚动到游标自己那条带，而不是显示树的顶部。
    #[test]
    fn a_short_panel_scrolls_to_the_cursor_band() {
        let view = tree();
        let canvas = VerticalCanvas::layout(
            &view,
            Rect {
                x: 0,
                y: 0,
                width: 40,
                height: 8,
            },
            2,
        )
        .expect("the panel fits one band");
        let drawn = canvas
            .hits()
            .into_iter()
            .find(|(_, index)| *index == 2)
            .expect("the cursor's node is drawn");
        assert!(
            drawn.0.bottom() <= 8,
            "the cursor's band is on the panel: {:?}",
            drawn.0
        );
    }
}
