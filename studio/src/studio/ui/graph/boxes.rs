//! Geometry for the call tree's node boxes.
//! 调用树节点盒的几何。
//!
//! The ledger this replaced drew one label per node on one row: a name longer
//! than its column was clipped, and the reader had to rebuild the boxes and the
//! ports from the text. The canvas draws them instead — each node is a bordered
//! cell whose content wraps the whole name — and this page decides where those
//! cells go: one column per hop, one lane per chain, wide enough for the widest
//! name in the column and tall enough for it to wrap.
//! 它取代的账本每个节点只在一行上画一个标签：名字超过列宽就被裁掉，读者得从文字里重建
//! 盒子与端口。画布直接把它们画出来——每个节点是一个带边框、内容按整名换行的单元格——
//! 而本页决定这些单元格放在哪：一跳一列、一条链一条车道，宽度取该列最宽的名字，高度取
//! 换行所需。

use std::collections::BTreeMap;

use super::*;

/// Cells between two columns: room for a bend, an evidence mark and an arrow.
/// 两列之间的单元格：容纳一个转折、一个证据标记和一个箭头。
pub(super) const GAP: u16 = 5;
/// The gap a cramped panel falls back to. Three cells still hold a port, a run,
/// an evidence mark and an arrow head; five hold them with room to breathe. The
/// narrow gap is what lets an 80-column terminal show a neighbour at all, and a
/// neighbour is what makes an edge visible.
/// 拥挤面板回退到的空隙。三格仍放得下端口、一段线、一个证据标记与一个箭头；五格则让它们有
/// 余地呼吸。窄空隙正是让 80 列终端也能显示邻列的原因，而邻列正是边可见的原因。
const TIGHT_GAP: u16 = 3;
/// Narrowest box the canvas will draw, borders included: below this a name is a
/// meaningless sliver.
/// 画布会画的最窄盒子（含边框）：再窄，名字就成了毫无意义的窄条。
pub(super) const MIN_W: u16 = 7;
/// Content rows a box will spend on a name before it abbreviates instead.
/// 盒子在改为缩写之前愿意花在名字上的内容行数。
const MAX_ROWS: usize = 2;
/// Widest box, borders included: a wider box pushes whole columns off screen.
/// 最宽的盒子（含边框）：再宽就会把整列挤出屏幕。
pub(super) const MAX_W: u16 = 26;
/// Rows the panel keeps for itself: the axis sentence, the ruler, the status row.
/// 面板留给自己的行：轴说明、层标尺、状态行。
const CHROME: u16 = 3;

/// One drawn node box.
/// 一个画出的节点盒。
#[derive(Clone, Copy, Debug)]
pub(super) struct NodeBox {
    /// Index into the tree's node list.
    /// 在内核节点列表中的下标。
    pub index: usize,
    /// Column this box belongs to.
    /// 该盒子所在的列。
    pub level: i32,
    /// The rectangle the box occupies, borders included.
    /// 盒子占据的矩形（含边框）。
    pub rect: Rect,
}

/// The boxes on screen, plus the column geometry that placed them.
/// 屏幕上的盒子，以及放置它们的列几何。
pub(super) struct NodeCanvas {
    /// Boxes that fit, in node order.
    /// 放得下的盒子，按节点顺序。
    pub(super) boxes: Vec<NodeBox>,
    /// Level to its left edge and width.
    /// 层号到它的左边界与宽度。
    pub(super) columns: BTreeMap<i32, (u16, u16)>,
    /// The panel's inner rectangle.
    /// 面板内部矩形。
    pub(super) body: Rect,
    /// The part of the body the boxes may use: below the ruler, above the status.
    /// 盒子可用的部分：标尺之下、状态行之上。
    pub(super) view: Rect,
}

impl NodeCanvas {
    /// Place the boxes for one tree around `cursor`, or `None` when the panel is
    /// too small for a box at all.
    /// 围绕 `cursor` 为一条树摆放盒子；面板小到放不下一个盒子时返回 `None`。
    pub(super) fn layout(tree: &CallTreeView, body: Rect, cursor: usize) -> Option<Self> {
        let nodes = &tree.tree.nodes;
        let cursor = cursor.min(nodes.len().checked_sub(1)?);
        if body.height < CHROME + 3 || body.width < MIN_W {
            return None;
        }
        let (low, high) = tree.tree.level_span();
        let widths = column_widths(nodes);
        let (gap, fitted) = packed_columns(&widths, low, high, nodes[cursor].level, body.width);
        let mut columns = BTreeMap::new();
        let mut x = body.x;
        for (level, width) in fitted {
            columns.insert(level, (x, width));
            x = x.saturating_add(width).saturating_add(gap);
        }
        let view = Rect {
            x: body.x,
            y: body.y.saturating_add(2),
            width: body.width,
            height: body.height.saturating_sub(CHROME),
        };
        let heights = lane_heights(nodes, &columns);
        // Every lane is one box tall plus one blank routing row, so the canvas is
        // their sum; the cursor's own band is what has to stay in view, so the
        // scroll is exactly the overflow below it.
        // 每条车道是一个盒子高加一条空白走线行，因此画布就是它们的和；必须留在视野里的是
        // 游标自己那一段，因此滚动量正是它下方溢出的部分。
        let mut tops = vec![0u16; heights.len()];
        let mut y = view.y;
        for (lane, entry) in tops.iter_mut().enumerate() {
            *entry = y;
            y = y.saturating_add(heights[lane]).saturating_add(1);
        }
        let cursor_lane = nodes[cursor].lane;
        let shift = tops[cursor_lane]
            .saturating_add(heights[cursor_lane])
            .saturating_sub(view.y)
            .saturating_sub(view.height);
        let boxes = nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let &(x, width) = columns.get(&node.level)?;
                let top = tops[node.lane].saturating_sub(shift);
                let height = heights[node.lane];
                if top < view.y || top.saturating_add(height) > view.bottom() {
                    return None;
                }
                Some(NodeBox {
                    index,
                    level: node.level,
                    rect: Rect {
                        x,
                        y: top,
                        width,
                        height,
                    },
                })
            })
            .collect();
        Some(Self {
            boxes,
            columns,
            body,
            view,
        })
    }

    /// Every box on screen with the node it draws, for hit testing.
    /// 屏幕上每个盒子及其绘制的节点，用于命中测试。
    pub(super) fn hits(&self) -> Vec<(Rect, usize)> {
        self.boxes
            .iter()
            .map(|drawn| (drawn.rect, drawn.index))
            .collect()
    }
}

/// Whether a node has an incoming edge the axis cannot draw, which is what the
/// status row reports for it.
/// 某个节点是否有无法沿轴画出的入边，也就是状态行为它报告的情况。
pub(super) fn incoming_off_axis(view: &CallTreeView, index: usize) -> bool {
    view.tree
        .edges
        .iter()
        .any(|edge| edge.callee == index && !edge.forward)
}

/// The content width each column needs, clamped to what a box may be.
/// 每列需要的内容宽度，已限制在盒子允许的范围内。
fn column_widths(nodes: &[CallTreeNode]) -> BTreeMap<i32, u16> {
    let mut widths: BTreeMap<i32, u16> = BTreeMap::new();
    for node in nodes {
        let want = (name_cells(&node.symbol) + 2).clamp(MIN_W, MAX_W);
        let slot = widths.entry(node.level).or_insert(MIN_W);
        *slot = (*slot).max(want);
    }
    widths
}

/// The columns to show and the width each one gets.
/// 要显示的列，以及每列得到的宽度。
///
/// The cursor's column is placed first, but it may not take so much width that no
/// neighbour can follow: a tree drawn as one box has no edges, and an edge is what
/// the panel exists to show. So it is capped at `available - GAP - MIN_W`, and
/// neighbours are then added right and left while they fit. A neighbour that
/// cannot have its wanted width is narrowed rather than dropped, down to
/// [`MIN_W`]; past that the label abbreviates instead of wrapping, so a narrow box
/// stays short.
/// 先放游标所在列，但它不能宽到让邻列无处可放：画成一个盒子的树没有边，而边正是本面板存在的
/// 理由。因此它被限制在 `available - GAP - MIN_W`，随后向左右加入放得下的邻列。得不到所需宽度
/// 的邻列会被收窄而不是丢弃，直到 [`MIN_W`]；再窄就由标签改为缩写而非换行，使窄盒子保持矮。
/// Pack the columns, trying the roomy gap first and the tight one when the roomy
/// gap would leave the tree with a single column.
/// 打包各列：先用宽松空隙，若宽松空隙会让树只剩一列，则改用窄空隙。
fn packed_columns(
    widths: &BTreeMap<i32, u16>,
    low: i32,
    high: i32,
    cursor_level: i32,
    available: u16,
) -> (u16, BTreeMap<i32, u16>) {
    let mut single = None;
    for gap in [GAP, TIGHT_GAP] {
        let chosen = visible_columns(widths, low, high, cursor_level, available, gap);
        if chosen.len() > 1 {
            return (gap, chosen);
        }
        single = Some((gap, chosen));
    }
    single.unwrap_or((GAP, BTreeMap::new()))
}

/// One packing attempt at a given gap.
/// 以给定空隙做一次打包尝试。
fn visible_columns(
    widths: &BTreeMap<i32, u16>,
    low: i32,
    high: i32,
    cursor_level: i32,
    available: u16,
    gap: u16,
) -> BTreeMap<i32, u16> {
    let mut chosen = BTreeMap::new();
    let has_neighbours = cursor_level > low || cursor_level < high;
    let reserve = if has_neighbours { gap + MIN_W } else { 0 };
    let want = widths.get(&cursor_level).copied().unwrap_or(MIN_W);
    let own = want
        .min(available.saturating_sub(reserve))
        .max(MIN_W.min(available));
    chosen.insert(cursor_level, own);
    let mut remaining = available.saturating_sub(own);
    let mut first = cursor_level;
    let mut last = cursor_level;
    loop {
        let mut grew = false;
        for candidate in [last.saturating_add(1), first.saturating_sub(1)] {
            if candidate > high || candidate < low || chosen.contains_key(&candidate) {
                continue;
            }
            let Some(room) = remaining.checked_sub(gap) else {
                continue;
            };
            if room < MIN_W {
                continue;
            }
            let width = widths.get(&candidate).copied().unwrap_or(MIN_W).min(room);
            chosen.insert(candidate, width);
            remaining = remaining.saturating_sub(gap + width);
            if candidate > last {
                last = candidate;
            } else {
                first = candidate;
            }
            grew = true;
        }
        if !grew {
            return chosen;
        }
    }
}

/// The height each lane needs: its tallest box.
/// 每条车道需要的高度：其中最高的盒子。
fn lane_heights(nodes: &[CallTreeNode], columns: &BTreeMap<i32, (u16, u16)>) -> Vec<u16> {
    let lanes = nodes.iter().map(|node| node.lane + 1).max().unwrap_or(1);
    let mut heights = vec![3u16; lanes];
    for node in nodes {
        let Some(&(_, width)) = columns.get(&node.level) else {
            continue;
        };
        heights[node.lane] = heights[node.lane].max(box_height(node, width));
    }
    heights
}

/// The rows one box needs: borders plus a content row per wrapped name row, plus
/// one more when the node has badges to show.
/// 一个盒子需要的行数：边框，加上换行后每个名字行一行，节点有徽标时再加一行。
fn box_height(node: &CallTreeNode, width: u16) -> u16 {
    let content = width.saturating_sub(2).max(1);
    let rows =
        label_rows(&node.symbol, content).len() + usize::from(!badges(node, false).is_empty());
    (rows as u16).saturating_add(2).max(3)
}

/// The rows a box shows for one name.
/// 一个盒子为某个名字展示的各行。
///
/// A name that fits in [`MAX_ROWS`] rows is wrapped — the whole name is on screen,
/// which is the point of drawing boxes. A name that would need more is
/// *abbreviated* to its tail with a leading `…` instead, because five rows of
/// three characters each is a sliver, not a label: a narrow column should read
/// `…canvas_width`, not `pre`/`vie`/`w_c`/`anv`/`as_`.
/// 放得进 [`MAX_ROWS`] 行的名字按整名换行——整个名字都在屏幕上，这正是画盒子的意义。需要更多
/// 行的名字改为保留尾部并冠以 `…`，因为五行各三个字符是窄条而不是标签：窄列应当读作
/// `…canvas_width`，而不是 `pre`/`vie`/`w_c`/`anv`/`as_`。
pub(super) fn label_rows(symbol: &str, width: u16) -> Vec<String> {
    let wrapped = wrap_name(symbol, width);
    if wrapped.len() <= MAX_ROWS {
        return wrapped;
    }
    // The longest tail that *actually* wraps into the budget wins. Counting
    // characters instead would over-promise: a break after an underscore leaves
    // rows shorter than the column, so `width * rows` characters can still need
    // more than `rows` rows.
    // 胜出的是**实际**能折进预算的最长尾部。按字符数计算会许下做不到的承诺：在下划线后断开
    // 会让行比列短，因此 `width * rows` 个字符仍可能超过 `rows` 行。
    let characters = symbol.chars().collect::<Vec<_>>();
    for keep in (1..characters.len()).rev() {
        let tail = characters[characters.len() - keep..]
            .iter()
            .collect::<String>();
        let candidate = format!("…{tail}");
        let rows = wrap_name(&candidate, width);
        if rows.len() <= MAX_ROWS {
            return rows;
        }
    }
    wrapped
}

/// The badges a box shows: what the tree left out, and whether the compiler was
/// the only witness.
/// 盒子展示的徽标：树省略了什么，以及是否只有编译器作证。
pub(super) fn badges(node: &CallTreeNode, uncertain: bool) -> String {
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
    badges
}

/// The cells a name takes on screen. Identifiers are ASCII in practice; counting
/// characters keeps a wide glyph from being split in half.
/// 名字在屏幕上占用的单元格数。标识符实际上都是 ASCII；按字符计数可以避免把一个宽字符
/// 劈成两半。
fn name_cells(name: &str) -> u16 {
    name.chars().count() as u16
}

/// Split a name into rows of at most `width` cells, preferring a break after an
/// underscore so `preview_canvas_width` reads as `preview_canvas_` + `width`
/// rather than as an arbitrary slice.
/// 把名字切成每行不超过 `width` 个单元格，优先在下划线之后断开，使
/// `preview_canvas_width` 读作 `preview_canvas_` + `width`，而不是任意切片。
pub(super) fn wrap_name(name: &str, width: u16) -> Vec<String> {
    let width = width.max(1) as usize;
    let characters = name.chars().collect::<Vec<_>>();
    let mut rows = Vec::new();
    let mut start = 0;
    while start < characters.len() {
        let mut end = (start + width).min(characters.len());
        if end < characters.len()
            && let Some(break_at) = characters[start..end]
                .iter()
                .rposition(|character| *character == '_')
            && break_at > 0
        {
            end = start + break_at + 1;
        }
        rows.push(characters[start..end].iter().collect());
        start = end;
    }
    if rows.is_empty() {
        rows.push(String::new());
    }
    rows
}

// The tests live in a sibling test-only page: they pin the geometry's behaviour,
// and keeping them here would push this page past the file budget while hiding
// the production surface behind them.
// 测试放在同级的仅测试页面：它们钉的是几何的行为，留在这里会让本页超出文件预算并把生产
// 表面埋在后面。
#[cfg(test)]
#[path = "boxes_tests.rs"]
mod boxes_tests;
