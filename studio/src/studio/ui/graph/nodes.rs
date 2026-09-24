//! Node-box call-tree drawing for the call-graph overlay.
//! 调用图浮层的节点盒调用树绘制。
//!
//! The panel draws the call direction as its horizontal axis: the focus owns the
//! middle column, callers fill the columns to its left, callees the columns to
//! its right, and every edge therefore runs left to right and ends in `▶` on the
//! callee. Each function is a *box* whose content wraps its whole name, so the
//! reader gets names rather than clipped tails; the cursor's box has the heavier
//! border and the focus's box the green one; and a cut the budgets made is
//! written on the box itself, because an overview that hides its own gaps is
//! worse than no overview.
//! 本面板以调用方向为横轴：焦点独占中间列，调用者填左边的列，被调用者填右边，因此每条边都
//! 从左向右画，并在被调用者一侧以 `▶` 收尾。每个函数是一个*盒子*，内容按整名换行，因此读者
//! 拿到的是名字而不是被裁掉的尾巴；游标的盒子边框更重、焦点的盒子是绿色；预算造成的裁剪写在
//! 盒子本身上——因为一个藏起自己缺口的鸟瞰图比没有更糟。
//!
//! Geometry lives in `boxes`, drawing in `box_draw`: this page only decides what
//! the panel is for and what its chrome says.
//! 几何在 `boxes`，绘制在 `box_draw`：本页只决定面板做什么、说明行说什么。

use super::*;

use super::box_draw;
use super::box_draw::{Clip, Selection, put};
use super::box_draw_vertical;
use super::box_vertical::VerticalCanvas;
use super::boxes::{NodeCanvas, incoming_off_axis};

/// The legend shown along the panel's bottom border: the markers the drawing uses
/// and cannot explain in place.
/// 沿面板底边显示的图例：图使用、且无法就地解释的标记。
const LEGEND: &str = " + live · ? candidate · ~ source · x external · ! unknown · ←n/→n not drawn ";

/// The width at which the left-to-right canvas can still give two columns a
/// readable box. Narrower than this, one column is all that fits, and one column
/// has no edges — so the tree is drawn downwards instead, where every box gets the
/// full width and the edges still show. `v` overrides the choice.
/// 从左到右的画布还能给两列各一个可读盒子的最小宽度。比这更窄时只放得下一列，而一列没有边
/// ——因此改为向下画，那里每个盒子都拿到整宽，边也仍然看得见。`v` 可以覆盖这个选择。
const HORIZONTAL_ENOUGH: u16 = 2 * 14 + 5;

/// What the caller knows about this panel: which box the cursor is on, whether
/// the panel holds the focus, what to call it, and which way the reader asked the
/// tree to run.
/// 调用方对应该本面板知道的事：游标在哪个盒子上、面板是否持有焦点、它叫什么，以及读者要求树
/// 朝哪边画。
pub(super) struct TreePanel {
    /// Index of the box under the cursor.
    /// 游标所在盒子的下标。
    pub(super) cursor: usize,
    /// Whether this panel holds the keyboard focus.
    /// 本面板是否持有键盘焦点。
    pub(super) focused: bool,
    /// The panel's title.
    /// 面板标题。
    pub(super) title: &'static str,
    /// `None` follows the panel's shape; see [`SearchState::tree_vertical`].
    /// `None` 跟随面板形状；见 [`SearchState::tree_vertical`]。
    pub(super) vertical: Option<bool>,
    /// Whether to draw through the hand-drawn canvases rather than the
    /// `rataflow` widget, and only when the `node-graph` feature is linked.
    /// 是否用与手绘画布绘制而不是 `rataflow` 控件；仅在链接 `node-graph` 特性时存在。
    #[cfg(feature = "node-graph")]
    pub(super) canvas: bool,
}

pub(super) fn draw_call_tree(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &mut App,
    selected: Option<&CallRef>,
    tree_panel: TreePanel,
) -> Vec<(Rect, usize)> {
    // `panel` is the border helper from the UI prelude, so this binding keeps its
    // own name instead of shadowing it.
    // `panel` 是 UI 前导里的边框辅助函数，因此这个绑定另起名字而不是遮蔽它。
    let cursor = tree_panel.cursor;
    let focused = tree_panel.focused;
    let title = tree_panel.title;
    let vertical = tree_panel.vertical;
    // The library drawer exists only when its feature is linked; the field is
    // gated with it, so the default build has no branch to take.
    // 库绘制只在其特性被链接时存在；该字段与它一同门控，因此默认构建没有分支可走。
    #[cfg(feature = "node-graph")]
    let canvas = tree_panel.canvas;
    let Some(item) = selected else {
        frame.render_widget(
            Paragraph::new("Select a call node.").block(panel(title, MUTED)),
            area,
        );
        return Vec::new();
    };
    let view = app.call_tree_view(item);
    // The budget belongs in the panel title: the border row is otherwise empty,
    // and a count that shares a row with the ruler gets clipped by it.
    // 预算放在面板标题里：边框行本来是空的，而和标尺共享一行的计数会被标尺裁掉。
    let depth = view
        .tree
        .nodes
        .iter()
        .map(|node| node.level.unsigned_abs())
        .max()
        .unwrap_or(0);
    let cursor = cursor.min(view.len().saturating_sub(1));
    let inner = area.inner(ratatui::layout::Margin {
        vertical: 1,
        horizontal: 1,
    });
    // The layout runs before the title is drawn, because the title has to say
    // which way the tree runs and how to change it.
    // 排版先于标题绘制，因为标题要说清树朝哪边画、以及怎么改。
    let vertical = vertical.unwrap_or(inner.width < HORIZONTAL_ENOUGH);
    // What the arrow keys will read: the widget and the top-down canvas run the
    // levels down the screen, the left-to-right canvas runs them across it.
    // 方向键要读的东西：控件与自上而下的画布把层画在向下方向，从左到右的画布把它画在横向。
    #[cfg(feature = "node-graph")]
    let library = !canvas;
    #[cfg(not(feature = "node-graph"))]
    let library = false;
    app.tree_top_down = library || vertical;
    let horizontal = (!vertical && !view.is_empty())
        .then(|| NodeCanvas::layout(&view, inner, cursor))
        .flatten();
    let down = (vertical && !view.is_empty())
        .then(|| VerticalCanvas::layout(&view, inner, cursor))
        .flatten();
    // The keys that change this panel are named once, in the page footer: a title
    // narrow enough to need them is also too narrow to hold them.
    // 改变本面板的按键只在页脚里说一次：窄到需要它们的标题，也窄到放不下它们。
    let mut heading = format!("{title} · depth {depth} · {} nodes", view.len());
    if down.is_some() {
        heading.push_str(" · vertical (v)");
    }
    #[cfg(feature = "node-graph")]
    if !canvas {
        heading.push_str(" · rataflow (g)");
    }
    if view.tree.truncated {
        heading.push_str(" · truncated");
    }
    frame.render_widget(
        panel(heading, if focused { GREEN } else { MAGENTA }).title_bottom(
            Line::from(Span::styled(LEGEND, Style::default().fg(MUTED)))
                .alignment(Alignment::Center),
        ),
        area,
    );
    // MIR candidates are the closest thing Studio has to the prototype's
    // "undecided": the compiler said these functions call each other, and no live
    // observation confirmed it. They are marked on the box, never merged into the
    // edges the axis draws.
    // MIR 候选是 Studio 里最接近原型"未决"的东西：编译器说这些函数互相调用，而没有实时观测
    // 确认。它们标在盒子上，绝不并入轴所画的那些边。
    let uncertain = view
        .refs
        .iter()
        .map(|item| {
            item.as_ref()
                .is_some_and(|item| !app.mir_candidates_for(&item.function).is_empty())
        })
        .collect::<Vec<bool>>();
    let selection = Selection {
        cursor,
        uncertain: &uncertain,
        focused,
    };
    #[cfg(feature = "node-graph")]
    if !canvas {
        // The widget keeps its own viewport, so it is rebuilt only when the tree
        // it draws changes — a moved cursor is a selection, not a new graph.
        // 控件保留自己的视口，因此只在它绘制的树变化时重建——游标移动是选择，不是新图。
        let key = flow_key(&view);
        match app.graph_flow.as_ref() {
            // A different tree: build it and fit it into the panel.
            // 另一棵树：构建它并让它铺满面板。
            None => {
                app.graph_flow =
                    node_graph::build(&view, cursor, &uncertain)
                        .ok()
                        .map(|mut flow| {
                            flow.request_fit_view();
                            (key, cursor, flow)
                        });
            }
            Some((cached, marked, _)) if *cached != key => {
                app.graph_flow =
                    node_graph::build(&view, cursor, &uncertain)
                        .ok()
                        .map(|mut flow| {
                            flow.request_fit_view();
                            (key, cursor, flow)
                        });
            }
            // The same tree with the cursor somewhere else: the mark lives in the
            // node's text, so it is re-stamped — and the viewport travels with it,
            // because pan and zoom are the reader's, not the mark's.
            // 同一棵树但游标移了位：标记住在节点文本里，因此重盖一次——而视口随行，因为平移与
            // 缩放属于读者，不属于标记。
            Some((_, marked, flow)) if *marked != cursor => {
                let viewport = flow.viewport;
                if let Ok(mut rebuilt) = node_graph::build(&view, cursor, &uncertain) {
                    rebuilt.viewport = viewport;
                    app.graph_flow = Some((key, cursor, rebuilt));
                }
            }
            Some(_) => {}
        }
        if let Some((_, _, flow)) = app.graph_flow.as_mut() {
            // The cursor is the widget's selection, and it is written into the
            // node's own title as well, so it is visible without colour.
            // 游标就是控件的选中项，同时也写进节点自己的标题，因此没有颜色也看得见。
            flow.select_node(&cursor.to_string());
            frame.render_widget(&mut *flow, inner);
            // The panel keeps the chrome that explains the drawing, which the
            // widget knows nothing about.
            // 面板保留解释这张图的说明行，那是控件不知道的东西。
            draw_status(
                frame,
                Clip {
                    body: inner,
                    view: inner,
                },
                app,
                &view,
                cursor,
            );
            return Vec::new();
        }
    }
    let hits = if let Some(canvas) = &horizontal {
        box_draw::draw(canvas, frame, &view, selection);
        canvas.hits()
    } else if let Some(canvas) = &down {
        box_draw_vertical::draw_vertical(canvas, frame, &view, selection);
        canvas.hits()
    } else {
        // Below this size a box is a wall of clipped borders; the border and the
        // panel title are the honest answer.
        // 小于这个尺寸时盒子只会是一堵裁剩的边框墙；边框与面板标题才是诚实的答案。
        return Vec::new();
    };
    if let Some(clip) = horizontal
        .as_ref()
        .map(NodeCanvas::clip)
        .or_else(|| down.as_ref().map(VerticalCanvas::clip))
    {
        draw_status(frame, clip, app, &view, cursor);
    }
    hits
}

/// What the cached widget was built from: the focus's symbol and the shape of the
/// tree around it. A re-centred focus changes it; a moved cursor does not.
/// 缓存的控件是从什么构建的：焦点的符号与它周围那棵树的形状。重新居中会改变它；游标移动不会。
#[cfg(feature = "node-graph")]
fn flow_key(view: &CallTreeView) -> String {
    let focus = view
        .tree
        .nodes
        .first()
        .map(|node| node.symbol.as_str())
        .unwrap_or("");
    format!(
        "{focus}:{}:{}",
        view.tree.nodes.len(),
        view.tree.edges.len()
    )
}

/// Draw the status row: the cursor's signature and its first transform, plus the
/// gaps that belong to this node alone.
/// 绘制状态行：游标的签名与它的第一条 transform，以及只属于这个节点的缺口。
fn draw_status(frame: &mut Frame<'_>, clip: Clip, app: &App, view: &CallTreeView, cursor: usize) {
    let Some(item) = view.item(cursor) else {
        return;
    };
    let mut status = format!("ƒ {}", signature_of(&item));
    if let Some(transform) = app.call_tree_transforms(&item).first() {
        status.push_str(&format!("   ↳ {transform}"));
    }
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
        clip,
        frame,
        clip.body.x,
        clip.body.bottom().saturating_sub(1),
        &status,
        Style::default().fg(MUTED),
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
