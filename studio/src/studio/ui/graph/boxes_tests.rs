//! Behaviour tests for the call tree's box geometry and drawing.
//! 调用树盒子几何与绘制的行为测试。
//!
//! Mounted by `boxes` as a sibling test-only page, for the same size reason the
//! drawing lives in `box_draw`: they pin behaviour, and keeping them in the
//! production page would push it past the file budget.
//! 由 `boxes` 作为同级仅测试页面挂载，尺寸理由与绘制放在 `box_draw` 相同：它们钉的是行为，
//! 留在生产页面会让它超出文件预算。

use super::*;

use nichlink_run_method::EvidenceKind;
use nichlink_run_method::mir::CallTreeEdge;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// A tree the shape of the fixture's: one caller, the focus, one callee, and a
/// callee that hides one of its callers.
/// 与夹具同形的树：一个调用者、焦点、一个被调用者，以及一个藏起了一个调用者的被调用者。
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
    view.tree.edges = vec![
        CallTreeEdge {
            caller: 1,
            callee: 0,
            evidence: EvidenceKind::Source,
            forward: true,
        },
        CallTreeEdge {
            caller: 0,
            callee: 2,
            evidence: EvidenceKind::Source,
            forward: true,
        },
    ];
    view.refs = vec![None; view.tree.nodes.len()];
    view
}

/// The rendered canvas as rows of characters.
/// 把渲染出的画布取成一行行字符。
fn rows(width: u16, height: u16, cursor: usize) -> Vec<Vec<char>> {
    let view = tree();
    let body = Rect {
        x: 0,
        y: 0,
        width,
        height,
    };
    let canvas = NodeCanvas::layout(&view, body, cursor).expect("the panel fits a box");
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("a test terminal");
    terminal
        .draw(|frame| {
            super::super::box_draw::draw(
                &canvas,
                frame,
                &view,
                super::super::box_draw::Selection {
                    cursor,
                    uncertain: &[false, false, true],
                    focused: true,
                },
            );
        })
        .expect("the canvas draws");
    let buffer = terminal.backend().buffer();
    buffer
        .content
        .chunks(buffer.area.width as usize)
        .map(|cells| {
            cells
                .iter()
                .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
                .collect()
        })
        .collect()
}

/// The picture: rows joined for substring checks.
/// 画面：把各行拼起来做子串检查。
fn picture(width: u16, height: u16, cursor: usize) -> String {
    rows(width, height, cursor)
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// A long name is wrapped at an underscore and never split mid-row, so the whole
/// name is on screen instead of a clipped tail.
/// 长名字在下划线处换行、绝不从中间劈开，因此整个名字都在屏幕上，而不是一个被裁掉的尾部。
#[test]
fn long_names_wrap_after_an_underscore() {
    assert_eq!(
        wrap_name("preview_canvas_width", 15),
        vec!["preview_canvas_".to_owned(), "width".to_owned()]
    );
    // Each underscore is a break opportunity, so a narrow column reads as
    // word-sized pieces rather than as one long slice and one short one.
    // 每个下划线都是一个断点，因此窄列读起来是一段段词，而不是一长一短两段。
    // The first underscore that fits is the break, and a remainder that fits the
    // column exactly is left whole rather than split for symmetry.
    // 能放下的第一个下划线就是断点，而恰好放得下整列的后半段保持完整，不为了对称再切一刀。
    assert_eq!(
        wrap_name("clamp_canvas_width", 12),
        vec!["clamp_".to_owned(), "canvas_width".to_owned()]
    );
    assert_eq!(wrap_name("short", 20), vec!["short".to_owned()]);
    for width in 4..20u16 {
        for row in wrap_name("preview_canvas_width_traced", width) {
            assert!(
                row.chars().count() <= width as usize,
                "{row} is wider than {width}"
            );
        }
    }
}

/// The whole name is drawn, not a clipped tail: every part of it appears inside
/// the box, and so does the cut badge the tree asked for.
/// 画出的是整个名字而不是被裁的尾部：每一段都出现在盒子里，树要求的裁剪徽标也在。
#[test]
fn the_whole_name_and_its_badges_are_on_screen() {
    let page = rows(120, 14, 0);
    let view = tree();
    let canvas = NodeCanvas::layout(
        &view,
        Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 14,
        },
        0,
    )
    .expect("the panel fits a box");
    for (rect, index) in canvas.hits() {
        let mut shown = String::new();
        for y in rect.y.saturating_add(1)..rect.bottom().saturating_sub(1) {
            let line = (rect.x.saturating_add(1)..rect.right().saturating_sub(1))
                .filter_map(|x| {
                    page.get(y as usize)
                        .and_then(|row| row.get(x as usize))
                        .copied()
                })
                .collect::<String>();
            shown.push_str(line.trim_end());
        }
        assert!(
            shown.contains(&view.tree.nodes[index].symbol),
            "box {index} shows {shown:?}, not {:?}",
            view.tree.nodes[index].symbol
        );
    }
    let picture = page
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        picture.contains("←1"),
        "the cut badge is missing:\n{picture}"
    );
    // The name is not clipped into an ellipsis the way the ledger clipped it.
    // 名字不会像账本那样被裁成省略号。
    assert!(!picture.contains('…'), "clipped label:\n{picture}");
}

/// Every node in the visible window has a box, the focus is centered on its own
/// column, and the two other columns sit on the sides the axis sentence names.
/// 可见窗口内每个节点都有盒子，焦点独占中间列，另外两列分别落在轴说明所说的两侧。
#[test]
fn the_call_direction_is_the_horizontal_axis() {
    let view = tree();
    let canvas = NodeCanvas::layout(
        &view,
        Rect {
            x: 0,
            y: 0,
            width: 120,
            height: 14,
        },
        0,
    )
    .expect("the panel fits a box");
    assert_eq!(canvas.hits().len(), 3, "every visible node gets a box");
    let column = |level: i32| canvas.columns.get(&level).map(|(x, _)| *x);
    let (left, center, right) = (
        column(-1).expect("the caller column"),
        column(0).expect("the focus column"),
        column(1).expect("the callee column"),
    );
    assert!(left < center && center < right, "{left} {center} {right}");
    for (rect, index) in canvas.hits() {
        assert!(
            rect.width >= 10 && rect.height >= 3,
            "node {index} has no room: {rect:?}"
        );
        assert!(rect.right() <= 120, "node {index} runs off the panel");
    }
}

/// Edges attach to the box borders: the caller's right border carries a port and
/// the callee's row carries the arrow, so a reader can follow the line without
/// reading the status row first.
/// 边接在盒子边框上：调用者的右边框带端口，被调用者那一行带箭头，因此读者不必先读状态行就
/// 能顺着线走。
#[test]
fn edges_attach_to_borders_and_end_in_an_arrow() {
    let picture = picture(120, 14, 0);
    assert!(picture.contains('┼'), "no port on a border:\n{picture}");
    assert_eq!(
        picture.matches('▶').count(),
        2,
        "both edges end in an arrow:\n{picture}"
    );
    // The evidence marker sits just before the arrow head.
    // 证据标记紧挨在箭头之前。
    assert!(picture.contains("~▶"), "no evidence marker:\n{picture}");
}

/// Two callers of one focus get one lane each, and the edge between a lane and
/// the focus's own lane is routed vertically in the gap: that bend is what keeps
/// the two callers readable instead of collapsing them onto one row.
/// 一个焦点的两个调用者各占一条车道，而车道与焦点所在车道之间的边在空隙里纵向走线：正是这个
/// 转折让两个调用者可读，而不是把它们挤到同一行。
#[test]
fn a_fan_in_draws_one_lane_per_caller() {
    let mut view = tree();
    // A second caller, one lane below the first, plus its edge to the focus.
    // 第二个调用者，在第一个下方一条车道，以及它到焦点的边。
    view.tree.nodes.push(CallTreeNode {
        symbol: "panel::paint".to_owned(),
        level: -1,
        lane: 1,
        parent: Some(0),
        cut_callers: 0,
        cut_callees: 0,
    });
    view.tree.edges.push(CallTreeEdge {
        caller: 3,
        callee: 0,
        evidence: EvidenceKind::Live,
        forward: true,
    });
    view.refs = vec![None; view.tree.nodes.len()];

    let body = Rect {
        x: 0,
        y: 0,
        width: 120,
        height: 16,
    };
    let canvas = NodeCanvas::layout(&view, body, 0).expect("the panel fits the boxes");
    let callers = canvas
        .hits()
        .into_iter()
        .filter(|(_, index)| view.tree.nodes[*index].level < 0)
        .collect::<Vec<_>>();
    assert_eq!(callers.len(), 2, "each caller has its own box");
    assert_ne!(
        callers[0].0.y, callers[1].0.y,
        "two callers share a lane, so one is drawn on the other"
    );

    let mut terminal = Terminal::new(TestBackend::new(120, 16)).expect("a test terminal");
    terminal
        .draw(|frame| {
            super::super::box_draw::draw(
                &canvas,
                frame,
                &view,
                super::super::box_draw::Selection {
                    cursor: 0,
                    uncertain: &[false, false, false, false],
                    focused: true,
                },
            );
        })
        .expect("the canvas draws");
    let buffer = terminal.backend().buffer();
    let picture = buffer
        .content
        .chunks(buffer.area.width as usize)
        .map(|cells| {
            cells
                .iter()
                .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n");
    println!("{picture}");
    assert_eq!(
        picture.matches('▶').count(),
        3,
        "one arrow per edge:\n{picture}"
    );
    assert!(
        picture.contains('│') || picture.contains('┼'),
        "the fan-in needs vertical routing:\n{picture}"
    );
}

/// Two boxes and a gap are the smallest tree that can show an edge, so the
/// packing must reach for that before it settles for one readable box.
/// 两个盒子加一个空隙是能展示一条边的最小树，因此排版必须先争取到它，再退而求其次只画一个
/// 可读的盒子。
#[test]
fn a_narrow_panel_still_gets_a_neighbour() {
    let view = tree();
    let body = |width| Rect {
        x: 0,
        y: 0,
        width,
        height: 12,
    };
    // The roomy gap needs 7 + 5 + 7 = 19; the tight one only 7 + 3 + 7 = 17.
    // 宽松空隙需要 7 + 5 + 7 = 19，窄空隙只要 7 + 3 + 7 = 17。
    let canvas = NodeCanvas::layout(&view, body(19), 0).expect("19 cells fit a box");
    assert_eq!(
        canvas.columns.len(),
        2,
        "19 cells hold two boxes and the roomy gap"
    );
    let canvas = NodeCanvas::layout(&view, body(17), 0).expect("17 cells fit a box");
    assert_eq!(
        canvas.columns.len(),
        2,
        "17 cells hold two boxes once the gap tightens, and two boxes are what shows an edge"
    );
    let canvas = NodeCanvas::layout(&view, body(16), 0).expect("16 cells fit a box");
    assert_eq!(
        canvas.columns.len(),
        1,
        "one cell short of the tight packing: one column is the honest answer"
    );
}

/// A cramped panel still draws one box per column, and the vertical scroll keeps
/// the cursor's own lane in view rather than showing the top of the tree.
/// 狭窄的面板仍然每列画一个盒子，且纵向滚动让游标所在车道保持可见，而不是显示树的顶部。
#[test]
fn a_cramped_panel_still_shows_the_cursor_lane() {
    let view = tree();
    let canvas = NodeCanvas::layout(
        &view,
        Rect {
            x: 0,
            y: 0,
            width: 40,
            height: 8,
        },
        2,
    )
    .expect("a narrow panel still fits one box");
    let callee = canvas
        .hits()
        .into_iter()
        .find(|(_, index)| *index == 2)
        .expect("the cursor's node is drawn");
    assert!(
        callee.0.bottom() <= 8,
        "the cursor's box is off panel: {:?}",
        callee.0
    );
    assert!(
        canvas.columns.len() < 3,
        "40 cells cannot hold three boxes plus gaps"
    );
}
