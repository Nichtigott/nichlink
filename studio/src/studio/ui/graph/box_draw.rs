//! Drawing for the call tree's node boxes.
//! 调用树节点盒的绘制。
//!
//! Boxes are drawn before edges so that an edge stamped onto a border merges into
//! a port (`┼`) instead of erasing it, and the arrow heads are drawn last so they
//! stay visible. Every write goes through [`put`], [`stamp`] or [`paint`], which
//! clip to the canvas: the panel's chrome, its neighbours and its border are never
//! touched. The same three writers serve both layouts, so a box and an edge cannot
//! look different depending on which way the tree runs.
//! 盒子先于边绘制，因此盖在边框上的边会合并成端口（`┼`）而不是把它擦掉；箭头最后绘制，因此
//! 保持可见。每次写入都经过 [`put`]、[`stamp`] 或 [`paint`]，三者都裁剪到画布：面板自身的
//! 说明行、相邻面板与边框都不会被碰到。三个写入函数同时服务两种排布，因此盒子与边不会因为树
//! 的走向不同而长得不一样。

use super::boxes::{NodeBox, NodeCanvas, badges, label_rows};
use super::glyphs;
use super::*;

/// Where the reader is: the box under the cursor, the boxes only the compiler
/// could vouch for, and whether this panel holds the focus. One struct because
/// both layouts pass exactly this along.
/// 读者在哪：游标所在的盒子、只有编译器能作证的盒子，以及本面板是否持有焦点。合成一个结构，
/// 因为两种排布传递的正是这些。
#[derive(Clone, Copy)]
pub(super) struct Selection<'a> {
    /// Index of the box under the cursor.
    /// 游标所在盒子的下标。
    pub(super) cursor: usize,
    /// Whether each node has MIR candidates the compiler found and no live
    /// observation confirmed.
    /// 每个节点是否有编译器发现、而没有实时观测确认的 MIR 候选。
    pub(super) uncertain: &'a [bool],
    /// Whether this panel holds the keyboard focus.
    /// 本面板是否持有键盘焦点。
    pub(super) focused: bool,
}

/// The two rectangles every write clips to.
/// 每次写入都要裁剪到的两个矩形。
#[derive(Clone, Copy)]
pub(super) struct Clip {
    /// The panel's inner rectangle: text may use all of it.
    /// 面板内部矩形：文本可以用满它。
    pub(super) body: Rect,
    /// The canvas rectangle: lines stay inside it.
    /// 画布矩形：线留在其中。
    pub(super) view: Rect,
}

impl NodeCanvas {
    /// The clipping rectangles of the horizontal canvas.
    /// 横向画布的裁剪矩形。
    pub(super) fn clip(&self) -> Clip {
        Clip {
            body: self.body,
            view: self.view,
        }
    }
}

/// Draw a laid-out horizontal canvas: chrome, boxes, then edges.
/// 绘制已排好版的横向画布：说明行、盒子，然后边。
pub(super) fn draw(
    canvas: &NodeCanvas,
    frame: &mut Frame<'_>,
    tree: &CallTreeView,
    selection: Selection<'_>,
) {
    draw_chrome(canvas, frame);
    for drawn in &canvas.boxes {
        draw_box(canvas.clip(), frame, tree, drawn, selection, None);
    }
    draw_edges(canvas, frame, tree);
}

/// The axis sentence and the level ruler, aligned under each column.
/// 轴说明与层标尺，对齐到每一列下方。
fn draw_chrome(canvas: &NodeCanvas, frame: &mut Frame<'_>) {
    put(
        canvas.clip(),
        frame,
        canvas.body.x,
        canvas.body.y,
        "← callers · call direction · callees →",
        Style::default().fg(MUTED),
    );
    for (level, (x, width)) in &canvas.columns {
        let label = if *level == 0 {
            "focus".to_owned()
        } else {
            format!("{level:+}")
        };
        let offset = (*width as usize).saturating_sub(label.chars().count()) / 2;
        put(
            canvas.clip(),
            frame,
            x.saturating_add(offset as u16),
            canvas.body.y.saturating_add(1),
            &label,
            Style::default().fg(MUTED),
        );
    }
}

/// One box: border, wrapped name, and the badges that say what the drawing left
/// out. `band` names the hop on the border, which is how the top-down layout
/// keeps its margin free for the rail.
/// 一个盒子：边框、换行后的名字，以及说明图省略了什么徽标。`band` 把跳数写在边框上，自上而下
/// 的排布正是靠它把边距留给轨道。
pub(super) fn draw_box(
    clip: Clip,
    frame: &mut Frame<'_>,
    tree: &CallTreeView,
    drawn: &NodeBox,
    selection: Selection<'_>,
    band: Option<String>,
) {
    let node = &tree.tree.nodes[drawn.index];
    let selected = drawn.index == selection.cursor;
    let center = drawn.level == 0;
    let accent = if selected {
        if selection.focused { CYAN } else { MAGENTA }
    } else if center {
        GREEN
    } else {
        MUTED
    };
    // The cursor's box is drawn with a heavier border: the reader has to be able
    // to tell "the node I am on" from "the node the tree is centered on".
    // 游标的盒子用更重的边框：读者必须能分清"我所在的节点"与"树以它为圆心的节点"。
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(if selected {
            BorderType::Double
        } else {
            BorderType::Rounded
        })
        .border_style(Style::default().fg(accent));
    if let Some(band) = band {
        block = block.title(Span::styled(
            format!(" {band} "),
            Style::default().fg(MUTED),
        ));
    }
    frame.render_widget(block, drawn.rect);
    let inner = Rect {
        x: drawn.rect.x.saturating_add(1),
        y: drawn.rect.y.saturating_add(1),
        width: drawn.rect.width.saturating_sub(2),
        height: drawn.rect.height.saturating_sub(2),
    };
    let style = Style::default()
        .fg(if selected || center { accent } else { INK })
        .add_modifier(if selected || center {
            Modifier::BOLD
        } else {
            Modifier::empty()
        });
    for (row, line) in label_rows(&node.symbol, inner.width).iter().enumerate() {
        if row as u16 >= inner.height {
            break;
        }
        put(
            clip,
            frame,
            inner.x,
            inner.y.saturating_add(row as u16),
            line,
            style,
        );
    }
    let text_badges = badges(
        node,
        selection
            .uncertain
            .get(drawn.index)
            .copied()
            .unwrap_or(false),
    );
    let width = text_badges.chars().count() as u16;
    if !text_badges.is_empty() && width <= inner.width && inner.height > 0 {
        put(
            clip,
            frame,
            inner.right().saturating_sub(width),
            inner.bottom().saturating_sub(1),
            &text_badges,
            Style::default().fg(MAGENTA),
        );
    }
}

/// Route every edge through the gap between its two columns.
/// 让每条边穿过它两列之间的空隙。
fn draw_edges(canvas: &NodeCanvas, frame: &mut Frame<'_>, tree: &CallTreeView) {
    // Several edges can enter the same box on the same row — a fan-in. Each one
    // keeps its own arrow head and its own evidence mark by taking the next
    // column left of the border, so the reader can count the edges and see what
    // each one rests on. Past three they share the last column: the gap is five
    // cells wide, and a fourth head would be drawn over the bend.
    // 同一个盒子的同一行可以有多条边进入——扇入。每条边通过占用边框左侧的下一个列来保留自己的
    // 箭头与证据标记，读者因此能数出边的条数并看出各自依据什么。超过三条时共用最后一列：空隙
    // 只有五格宽，第四个箭头会画在转折上。
    let clip = canvas.clip();
    let mut entries: Vec<(usize, u16)> = Vec::new();
    for edge in &tree.tree.edges {
        let (Some(caller), Some(callee)) =
            (box_of(canvas, edge.caller), box_of(canvas, edge.callee))
        else {
            continue;
        };
        // The axis is the call direction, so an edge that cannot run left to
        // right is left to the status row rather than drawn backwards.
        // 横轴就是调用方向，因此无法从左向右的边交给状态行说明，而不是反着画。
        if caller.rect.right() >= callee.rect.x {
            continue;
        }
        let from_y = caller.rect.y + caller.rect.height / 2;
        let to_y = callee.rect.y + callee.rect.height / 2;
        let bend = caller
            .rect
            .right()
            .saturating_add(callee.rect.x.saturating_sub(caller.rect.right()) / 2);
        let line = Style::default().fg(MUTED);
        let lane = entries
            .iter()
            .filter(|(callee, row)| *callee == edge.callee && *row == to_y)
            .count()
            .min(2) as u16;
        entries.push((edge.callee, to_y));
        let arrow_x = callee.rect.x.saturating_sub(1 + lane);
        // The mark sits immediately left of this edge's own head, which is what
        // makes a single edge read `~▶`. In a fan-in a later edge's head can land
        // on an earlier edge's mark; the arrows still count one per edge, and the
        // legend names every mark shape.
        // 标记紧挨在本条边自己的箭头左侧，这正是单条边读作 `~▶` 的原因。扇入时后画边的箭头
        // 可能落在前一条边的标记上；箭头仍然一条边一个，且图例列出了所有标记形状。
        let mark_x = arrow_x.saturating_sub(1);
        if from_y == to_y {
            // Both ends sit on the same lane, so the edge is one straight run:
            // a bend here would draw a one-cell stem that stands for no bend.
            // 两端在同一条车道上，因此这条边就是一条直线：在这里转折会画出一根并不代表任何
            // 转折的一格残端。
            for x in caller.rect.right()..mark_x {
                stamp(clip, frame, x, from_y, '─', line);
            }
        } else {
            let (top, bottom) = if from_y <= to_y {
                (from_y, to_y)
            } else {
                (to_y, from_y)
            };
            for y in top..=bottom {
                stamp(clip, frame, bend, y, '│', line);
            }
            for x in caller.rect.right()..bend {
                stamp(clip, frame, x, from_y, '─', line);
            }
            for x in bend.saturating_add(1)..mark_x {
                stamp(clip, frame, x, to_y, '─', line);
            }
        }
        // A line stamped onto a border merges into a port, which is what makes
        // the attachment visible.
        // 盖在边框上的线会合并成端口，这正是"接在这里"看得见的原因。
        stamp(
            clip,
            frame,
            caller.rect.right().saturating_sub(1),
            from_y,
            '─',
            line,
        );
        stamp(clip, frame, callee.rect.x, to_y, '─', line);
        paint(
            clip,
            frame,
            mark_x,
            to_y,
            edge.evidence.marker().chars().next().unwrap_or('?'),
            Style::default().fg(if edge.evidence.confirmed() {
                GREEN
            } else {
                MUTED
            }),
        );
        paint(clip, frame, arrow_x, to_y, '▶', Style::default().fg(CYAN));
    }
}

/// The box that draws one node, if it is on screen.
/// 绘制某个节点的盒子（若在屏幕上）。
fn box_of(canvas: &NodeCanvas, index: usize) -> Option<&NodeBox> {
    canvas.boxes.iter().find(|drawn| drawn.index == index)
}

/// Write text into the frame, clipped to the panel body.
/// 把文本写入帧缓冲，并裁剪到面板内部。
///
/// This is the one text writer on the canvas: the status row below it uses the
/// same function, so clipping and styling cannot drift between the boxes and the
/// row that explains them.
/// 这是画布上唯一的文本写入函数：它下面的状态行也用同一个，因此裁剪与样式不会在盒子与
/// 解释它们的行之间各自漂移。
pub(super) fn put(clip: Clip, frame: &mut Frame<'_>, x: u16, y: u16, text: &str, style: Style) {
    if y < clip.body.y || y >= clip.body.bottom() {
        return;
    }
    for (offset, character) in text.chars().enumerate() {
        let x = x.saturating_add(offset as u16);
        if x < clip.body.x {
            continue;
        }
        if x >= clip.body.right() {
            break;
        }
        if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
            cell.set_char(character);
            cell.set_style(style);
        }
    }
}

/// Write one terminal glyph — an arrow head or an evidence mark — over whatever
/// is in the cell.
/// 把一个终结字形（箭头或证据标记）盖在单元格已有内容之上。
///
/// These are not lines, so they must not go through the merge table: merging an
/// arrow into a run would keep the run and drop the arrow.
/// 它们不是线，因此不能走合并表：把箭头并进一段线会保留线而丢掉箭头。
pub(super) fn paint(clip: Clip, frame: &mut Frame<'_>, x: u16, y: u16, glyph: char, style: Style) {
    if x < clip.view.x || y < clip.view.y || x >= clip.view.right() || y >= clip.view.bottom() {
        return;
    }
    if let Some(cell) = frame.buffer_mut().cell_mut((x, y)) {
        cell.set_char(glyph);
        cell.set_style(style);
    }
}

/// Write one line glyph, merging it with whatever the cell already holds.
/// 写入一个线字形，并与单元格里已有的内容合并。
pub(super) fn stamp(clip: Clip, frame: &mut Frame<'_>, x: u16, y: u16, wanted: char, style: Style) {
    if x < clip.view.x || y < clip.view.y || x >= clip.view.right() || y >= clip.view.bottom() {
        return;
    }
    let Some(cell) = frame.buffer_mut().cell_mut((x, y)) else {
        return;
    };
    let existing = cell.symbol().chars().next().unwrap_or(' ');
    let merged = glyphs::merged(existing, wanted);
    if merged == existing {
        return;
    }
    cell.set_char(merged);
    cell.set_style(style);
}
