//! Spatial call-tree regression tests.
//! 空间调用树回归测试。
//!
//! These run against the node-editor fixture for the same reason the call-graph
//! tests do: the rules under test are about a *shape* — the focus owns column
//! zero, callers fill the left, callees the right, the cursor moves along the
//! axis — and a shape needs a call graph whose edges are already pinned.
//! 这些测试与调用图测试一样跑在 node-editor 夹具上：被测规则关于一种*形状*——焦点独占第零列、
//! 调用者填左、被调用者填右、游标沿轴移动——而形状需要一个调用边已被钉住的调用图。
//!
//! Mounted only with the `prototype-fixtures` feature, so the default build
//! carries neither these tests nor their fixture selection.
//! 仅在启用 `prototype-fixtures` 特性时挂载，因此默认构建既不携带这些测试，也不携带它们的
//! 夹具选择。

use super::*;

use ratatui::Terminal;
use ratatui::backend::TestBackend;

/// The search state these tests draw with: the hand-drawn canvas, whose picture
/// and chrome they assert. With the `node-graph` feature off the field does not
/// exist and the canvas is what runs anyway.
/// 这些测试用来绘制的搜索状态：手绘画布，因为它们断言的是它的画面与说明行。`node-graph`
/// 特性关闭时该字段不存在，而运行的本来就是画布。
fn canvas_drawn(mut state: SearchState) -> SearchState {
    #[cfg(feature = "node-graph")]
    {
        state.tree_canvas = true;
    }
    state
}

/// The node-editor fixture, whose call graph the graph tests already pin.
/// node-editor 夹具，它的调用图已被调用图测试钉住。
fn fixture_app() -> Option<App> {
    let fixture = node_editor_fixture()?;
    select_project(
        fixture.clone(),
        fixture.join("Cargo.toml"),
        "nichlink.fixture.node-editor",
    );
    Some(App::load())
}

/// The registered functions of the loaded project, one per name.
/// 当前载入项目中已注册的函数，每个名字取一个。
fn registered_functions(app: &App, limit: usize) -> Vec<CallRef> {
    let mut functions: Vec<CallRef> = Vec::new();
    for info in app.registry.depth_first() {
        if info.source.function.is_empty()
            || functions
                .iter()
                .any(|existing: &CallRef| same_symbol(&existing.function, &info.source.function))
        {
            continue;
        }
        functions.push(CallRef {
            node: info.id,
            function: info.source.function.clone(),
            file: info.source.file.clone(),
        });
        if functions.len() >= limit {
            break;
        }
    }
    functions
}

/// Open the provenance graph on one registered function and hand back its state.
/// 在某个已注册函数上打开溯源图，并交回它的状态。
fn open_graph(app: &mut App, function: &str) -> SearchState {
    let rows = app.search_rows(function);
    let selected = rows
        .iter()
        .position(|row| row.function == function && row.node.is_some())
        .unwrap_or_else(|| panic!("{function} is not a registered function"));
    app.overlay = Some(Overlay::Search(SearchState {
        query: function.to_owned(),
        selected,
        ..SearchState::default()
    }));
    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    let Some(Overlay::Search(opened)) = app.overlay.as_ref() else {
        panic!("Enter on a call row opens the graph");
    };
    opened.clone()
}

/// The tree around one function of the loaded project.
/// 当前载入项目中某个函数周围的调用树。
fn view_around(app: &App, function: &str) -> CallTreeView {
    let rows = app.search_rows(function);
    let row = rows
        .iter()
        .find(|row| row.function == function && row.node.is_some())
        .unwrap_or_else(|| panic!("{function} is not a registered function"));
    app.call_tree_view(&CallRef {
        node: row.node.expect("the row carries a node"),
        function: row.function.clone(),
        file: row.path.clone(),
    })
}

/// The rendered page as rows of characters.
/// 把渲染出的页面取成一行行字符。
fn rendered_rows(terminal: &Terminal<TestBackend>) -> Vec<Vec<char>> {
    let buffer = terminal.backend().buffer();
    let width = buffer.area.width as usize;
    buffer
        .content
        .chunks(width)
        .map(|cells| {
            cells
                .iter()
                .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
                .collect()
        })
        .collect()
}

/// The first column where `key` appears on one line.
/// `key` 在该行上首次出现的列。
fn find_key(line: &[char], key: &str) -> Option<usize> {
    let key = key.chars().collect::<Vec<_>>();
    if key.is_empty() || key.len() > line.len() {
        return None;
    }
    (0..=line.len() - key.len()).find(|start| line[*start..*start + key.len()] == key[..])
}

/// The call-tree panel's own lines, cut out by the corners it drew.
/// 调用树面板自身的各行，按它画出的四角裁出。
///
/// Reading the panel out of the page by its borders keeps this test independent
/// of the layout percentages: whatever rectangle the panel gets, the picture is
/// the rectangle it drew.
/// 按边框从页面里裁出面板，使本测试与布局百分比无关：面板拿到哪个矩形，图就是它画出的那个
/// 矩形。
fn tree_panel(rows: &[Vec<char>]) -> Option<Vec<Vec<char>>> {
    const TITLE: &str = "CALL TREE · depth";
    let top = rows
        .iter()
        .position(|row| row.iter().collect::<String>().contains(TITLE))?;
    // The page holds three panels side by side, so the corners that belong to
    // this one are the nearest ones around its own title, not the page's edges.
    // 页面上并排三个面板，因此属于本面板的四角是它自己标题两侧最近的那两个，而不是页面边缘。
    let title_at = find_key(&rows[top], TITLE)?;
    let left = rows[top][..title_at]
        .iter()
        .rposition(|cell| *cell == '╭')?;
    let right = title_at + rows[top][title_at..].iter().position(|cell| *cell == '╮')?;
    let mut panel = Vec::new();
    for row in &rows[top..] {
        if row.len() <= right || row.len() <= left {
            return None;
        }
        let line = row[left..=right].to_vec();
        let closed = line.first() == Some(&'╰');
        panel.push(line);
        if closed {
            return Some(panel);
        }
    }
    None
}

/// The text one box shows, read back from the page: its inner rows with trailing
/// blanks trimmed, joined. A wrapped name reconstructs exactly, because wrapping
/// inserts no characters — which is the point of drawing boxes instead of clipped
/// labels.
/// 从页面读回一个盒子展示的文本：内部各行去掉尾部空白后拼接。换行不插入任何字符，因此换行
/// 后的名字能精确还原——这正是画盒子而不是画被裁标签的意义。
fn box_text(page: &[Vec<char>], rect: Rect) -> String {
    let mut text = String::new();
    for y in rect.y.saturating_add(1)..rect.bottom().saturating_sub(1) {
        let Some(row) = page.get(y as usize) else {
            continue;
        };
        let line = (rect.x.saturating_add(1)..rect.right().saturating_sub(1))
            .filter_map(|x| row.get(x as usize).copied())
            .collect::<String>();
        text.push_str(line.trim_end());
    }
    text
}

#[test]
fn every_shape_the_tree_promises_holds_around_a_real_focus() {
    let Some(app) = fixture_app() else {
        return;
    };
    let functions = registered_functions(&app, 3);
    assert!(
        functions.len() >= 2,
        "the fixture registers several functions to focus on"
    );
    for focus in &functions {
        let view = app.call_tree_view(focus);
        assert_eq!(view.len(), view.refs.len(), "refs and nodes stay aligned");
        assert!(
            same_symbol(&view.tree.nodes[0].symbol, &focus.function),
            "{} owns row zero",
            focus.function
        );
        let mut seen: Vec<(i32, usize)> = Vec::new();
        for (index, node) in view.tree.nodes.iter().enumerate() {
            if index == 0 {
                assert_eq!(node.parent, None, "the focus is placed by nobody");
            } else {
                let parent = &view.tree.nodes[node.parent.expect("a placed node has a parent")];
                assert_eq!(
                    node.level.unsigned_abs(),
                    parent.level.unsigned_abs() + 1,
                    "{} hangs exactly one hop off {}",
                    node.symbol,
                    parent.symbol
                );
                assert!(
                    parent.level == 0 || (parent.level > 0) == (node.level > 0),
                    "{} stays on {}'s side of the focus",
                    node.symbol,
                    parent.symbol
                );
            }
            let cell = (node.level, node.lane);
            assert!(
                !seen.contains(&cell),
                "column {cell:?} holds two nodes, so one would be drawn on the other"
            );
            seen.push(cell);
        }
        for edge in &view.tree.edges {
            let caller = view.tree.nodes[edge.caller].level;
            let callee = view.tree.nodes[edge.callee].level;
            assert_eq!(
                edge.forward,
                callee > caller,
                "forward must mean left to right: {caller} → {callee}"
            );
        }
    }
}

#[test]
fn the_memo_answers_the_same_focus_and_a_different_one() {
    let Some(app) = fixture_app() else {
        return;
    };
    let functions = registered_functions(&app, 2);
    let focus = functions.first().expect("a focus to build around").clone();
    let first = app.call_tree_view(&focus);
    let second = app.call_tree_view(&focus);
    assert_eq!(first.tree, second.tree, "the memo returns the same tree");
    let other = app.call_tree_view(functions.get(1).expect("a second focus"));
    assert!(
        !same_symbol(&other.tree.nodes[0].symbol, &first.tree.nodes[0].symbol),
        "a different focus must not be served from the first focus's entry"
    );
}

/// The arrow keys follow the drawn picture, not the model's own axes: the same
/// four keys keep their printed meaning in either drawer.
/// 方向键跟随画出来的图，而不是模型自己的轴：四个键在两种绘制方里都保持字面含义。
#[test]
fn the_arrows_follow_the_drawn_grid() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "paint_node_editor");
    search.graph_focus = 0;
    search.outline_selected = 0;
    app.overlay = Some(Overlay::Search(search));
    let view = view_around(&app, "paint_node_editor");
    let selected = |app: &App| match app.overlay.as_ref() {
        Some(Overlay::Search(state)) => state.outline_selected,
        _ => panic!("the graph stays open while hopping"),
    };
    let node = |index: usize| view.tree.nodes[index].clone();

    // The library drawer runs the calls down the screen: ↓ is a hop along a drawn
    // edge, and ← → move between the lanes of one band.
    // 控件绘制方把调用沿屏幕向下排：↓ 沿一条画出来的边走一跳，← → 在一条带的车道间移动。
    app.tree_top_down = true;
    app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
    let down = selected(&app);
    assert_eq!(
        node(down).level,
        1,
        "↓ leaves the focus: {}",
        node(down).symbol
    );
    assert_eq!(node(down).parent, Some(0), "↓ follows an edge that exists");
    app.handle_overlay_key(KeyEvent::from(KeyCode::Right));
    let right = selected(&app);
    assert_eq!(
        node(right).level,
        1,
        "→ stays in the band: {}",
        node(right).symbol
    );
    assert!(node(right).lane > node(down).lane, "→ takes the next lane");
    app.handle_overlay_key(KeyEvent::from(KeyCode::Left));
    assert_eq!(selected(&app), down, "← returns to the lane it came from");
    app.handle_overlay_key(KeyEvent::from(KeyCode::Up));
    assert_eq!(selected(&app), 0, "↑ returns to the focus");

    // A left-to-right drawing swaps the axes, and the same four keys keep their
    // printed meaning: the picture is the authority, not the model.
    // 从左到右的绘制把两条轴换过来，而同样的四个键保持字面含义：以图为准，而不是以模型为准。
    app.tree_top_down = false;
    app.handle_overlay_key(KeyEvent::from(KeyCode::Right));
    let right = selected(&app);
    assert_eq!(
        node(right).level,
        1,
        "→ is the call direction here: {}",
        node(right).symbol
    );
    app.handle_overlay_key(KeyEvent::from(KeyCode::Up));
    let up = selected(&app);
    assert_eq!(
        node(up).level,
        1,
        "↑ moves between lanes here: {}",
        node(up).symbol
    );
    app.handle_overlay_key(KeyEvent::from(KeyCode::Left));
    assert_eq!(selected(&app), 0, "← returns to the focus");
}

#[test]
fn enter_re_centres_on_a_tree_node_and_opens_the_editor_only_at_the_focus() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    search.outline_selected = 1;
    app.overlay = Some(Overlay::Search(search));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    let Some(Overlay::Search(recentred)) = app.overlay.as_ref() else {
        panic!("Enter on a tree node re-centres instead of closing the graph");
    };
    assert_eq!(recentred.outline_selected, 0, "the new focus is row zero");
    assert!(
        recentred.center_function.as_deref() != Some("preview_canvas_width"),
        "the graph moved to another function"
    );
    assert!(app.event.contains("re-centred"), "{}", app.event);

    app.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
    assert!(
        app.overlay.is_none(),
        "Enter at the focus itself opens the editor"
    );
}

#[test]
fn the_drawn_tree_states_its_axis_and_keeps_the_focus_in_view() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    app.overlay = Some(Overlay::Search(canvas_drawn(search)));
    let mut terminal = Terminal::new(TestBackend::new(160, 48)).expect("a test terminal");
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut app))
        .expect("the graph page draws");
    let output = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert!(
        output.contains("call direction"),
        "the panel must name the axis it draws: {output}"
    );
    assert!(
        output.contains("← callers · call direction · callees →"),
        "the axis sentence must survive the panel width: {output}"
    );
    assert!(
        output.contains("depth "),
        "the header reports the hop budget: {output}"
    );
    assert!(
        output.contains("focus"),
        "the level ruler labels column zero: {output}"
    );
    assert!(
        output.contains("preview_canvas_width"),
        "the focus itself is on screen: {output}"
    );
}
/// The picture the TUI draws must be the shape the model describes: every node
/// in the window has a box that shows its **whole** name, the boxes sit on the
/// columns the axis sentence promises, the edges leave ports and end in arrows,
/// and the header counts what the model holds. The drawing also publishes the
/// rectangles it drew — the same ones a click hits — so the test reads the boxes
/// back from the page with them.
/// TUI 画出的图必须是模型描述的形状：窗口内每个节点都有一个展示**整名**的盒子，盒子落在轴
/// 说明承诺的列上，边从端口离开并以箭头收尾，表头数出模型持有的数量。绘制还会公布它画出的
/// 矩形（也就是点击命中的那些），因此测试用它们从页面读回盒子。
#[test]
fn the_drawn_tree_is_the_shape_the_model_describes() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    // Wider than the default split: the shape under test is the columns on both
    // sides of the focus, and 60% leaves room for two of them at this size.
    // 比默认分栏更宽：被测形状是焦点两侧的列，而 60% 在这个尺寸下只放得下其中两列。
    app.graph_split_percent = 80;
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    search.outline_selected = 0;
    app.overlay = Some(Overlay::Search(canvas_drawn(search)));

    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("the graph stays open while the tree is drawn");
    };
    let center = app.graph_item(search).expect("the graph has a centre");
    let view = app.call_tree_view(&center);
    assert!(view.len() > 1, "the fixture's focus has neighbours to draw");

    let mut terminal = Terminal::new(TestBackend::new(200, 50)).expect("a test terminal");
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut app))
        .expect("the graph page draws");
    let page = rendered_rows(&terminal);
    let panel = tree_panel(&page).expect("the call tree panel is on screen");
    println!("── CALL TREE around {} ──", center.function);
    for line in &panel {
        println!("{}", line.iter().collect::<String>());
    }
    let panel_text = panel
        .iter()
        .map(|line| line.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");

    // The title counts the model: a header that lies about its node count is
    // worse than no header.
    // 标题数出模型：一个把节点数说错的表头比没有表头更糟。
    let depth = view
        .tree
        .nodes
        .iter()
        .map(|node| node.level.unsigned_abs())
        .max()
        .unwrap_or(0);
    let title = panel
        .first()
        .expect("the panel has a title")
        .iter()
        .collect::<String>();
    assert!(title.contains(&format!("depth {depth}")), "{title}");
    assert!(title.contains(&format!("{} nodes", view.len())), "{title}");
    assert_eq!(
        title.contains("truncated"),
        view.tree.truncated,
        "the header must say so exactly when the model cut something: {title}"
    );

    // The boxes the drawing published are the boxes on screen, and each shows the
    // whole name of its node: a box is what makes a 27-character identifier
    // readable in a 26-cell column.
    // 绘制公布的盒子就是屏幕上的盒子，且每个都展示其节点的整名：盒子正是让一个 27 字符的
    // 标识符在 26 格宽的列里可读的东西。
    let drawn = app.hot.graph_tree_boxes.clone();
    assert!(!drawn.is_empty(), "the canvas drew no box:\n{panel_text}");
    assert!(
        drawn.iter().any(|(_, index)| *index == 0),
        "the focus must have a box:\n{panel_text}"
    );
    for (rect, index) in &drawn {
        let symbol = &view.tree.nodes[*index].symbol;
        let shown = box_text(&page, *rect);
        assert!(
            shown.contains(symbol.as_str()),
            "box {index} shows {shown:?}, not {symbol:?}"
        );
        assert!(
            page.iter().all(|row| row.len() >= rect.right() as usize),
            "box {index} runs off the page"
        );
    }

    // The axis sentence is a promise about the picture: a node one hop right of
    // another is drawn to its right.
    // 轴说明是对图本身的承诺：比另一个节点多一跳的节点画在它右边。
    let column_of = |level: i32| {
        drawn
            .iter()
            .find(|(_, index)| view.tree.nodes[*index].level == level)
            .map(|(rect, _)| rect.x)
    };
    let (left, center_x, right) = (
        column_of(-1).expect("the caller column is drawn"),
        column_of(0).expect("the focus column is drawn"),
        column_of(1).expect("the callee column is drawn"),
    );
    assert!(
        left < center_x && center_x < right,
        "{left} {center_x} {right}"
    );

    // The ruler names the middle column, and the legend on the bottom border
    // explains the markers the drawing uses in place.
    // 标尺点出中间那一列，底边图例解释绘制就地使用的标记。
    let ruler = panel[2].iter().collect::<String>();
    assert!(
        ruler.contains("focus"),
        "the ruler names column zero: {ruler}"
    );
    let legend = panel
        .last()
        .expect("the panel has a bottom border")
        .iter()
        .collect::<String>();
    assert!(
        legend.contains("+ live"),
        "the legend explains the markers: {legend}"
    );

    // Every edge whose two ends are drawn leaves a port on the caller's border
    // and ends in an arrow on the callee's row, with the evidence mark beside it.
    // 两端都画出的每条边都会在调用者边框上留下端口，并在被调用者那一行以箭头收尾，旁边带证据
    // 标记。
    let drawn_edges = view
        .tree
        .edges
        .iter()
        .filter(|edge| {
            let (Some(caller), Some(callee)) = (
                drawn.iter().find(|(_, index)| *index == edge.caller),
                drawn.iter().find(|(_, index)| *index == edge.callee),
            ) else {
                return false;
            };
            caller.0.right() < callee.0.x
        })
        .count();
    assert!(drawn_edges > 0, "the fixture has an edge to draw");
    assert!(
        panel_text.contains('┼'),
        "no port on a border:\n{panel_text}"
    );
    assert_eq!(
        panel_text.matches('▶').count(),
        drawn_edges,
        "one arrow per drawn edge:\n{panel_text}"
    );
    assert!(panel_text.contains("~▶"), "no evidence mark:\n{panel_text}");

    // The status row is the cursor's signature; it is inside the panel, below the
    // canvas, and belongs to row zero because the cursor is the focus.
    // 状态行是游标的签名；它在面板内、画布之下，且因为游标就是焦点而属于第零行。
    let status = panel[panel.len() - 2].iter().collect::<String>();
    assert!(
        status.contains(&center.function),
        "the status row states the cursor's signature: {status}"
    );
}

/// Clicking a box selects the node that box draws: the canvas publishes the
/// rectangles it drew, and the pointer handler hit-tests exactly those, so a
/// click cannot land on a rectangle the reader cannot see.
/// 点击盒子即选中该盒子绘制的节点：画布公布它画出的矩形，指针处理正是对这些矩形做命中测试，
/// 因此点击不可能落在读者看不见的矩形上。
#[test]
fn a_click_lands_on_the_box_under_the_pointer() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    app.graph_split_percent = 80;
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    search.outline_selected = 0;
    app.overlay = Some(Overlay::Search(canvas_drawn(search)));
    let mut terminal = Terminal::new(TestBackend::new(200, 50)).expect("a test terminal");
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut app))
        .expect("the graph page draws");
    let Some((rect, index)) = app
        .hot
        .graph_tree_boxes
        .iter()
        .find(|(_, index)| *index != 0)
        .copied()
    else {
        panic!("the fixture draws a box besides the focus");
    };
    let (x, y) = (rect.x + rect.width / 2, rect.y + 1);
    app.handle_mouse(MouseEventKind::Down(MouseButton::Left), x, y);
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("the graph stays open after a click inside it");
    };
    assert_eq!(
        search.outline_selected, index,
        "the click should select the box it landed on"
    );
    assert_eq!(search.graph_focus, 0, "clicking a box focuses the tree");
}

/// A panel too narrow for a neighbour says so where the reader looks — the title
/// — and the keys it names really widen the tree, clamped like the divider drag.
/// 窄到放不下邻列的面板在读者看的地方——标题——说明这一点，而它点名的按键确实能把树加宽，
/// 且与分隔线拖动使用同一范围。
#[test]
fn a_narrow_panel_names_the_keys_that_widen_it() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    // The panel would draw this tree downwards at this width; the hint exists for
    // the reader who asked for left-to-right anyway.
    // 这个宽度下面板本会把树向下画；提示是给"偏要横向"的读者看的。
    search.tree_vertical = Some(false);
    app.overlay = Some(Overlay::Search(search));
    // The page gives the tree a share of its width, so the narrow case is asked
    // for explicitly: at 35% of 44 columns only one tree column fits, which is
    // when the panel says how to widen it.
    // 页面把宽度分一份给树，因此窄的情况要显式要求：44 列的 35% 只放得下树的一列，这也正是
    // 面板说明如何加宽的时机。
    app.graph_split_percent = 35;
    let mut terminal = Terminal::new(TestBackend::new(44, 24)).expect("a test terminal");
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut app))
        .expect("the graph page draws");
    let page = rendered_rows(&terminal)
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        page.contains("CALL TREE"),
        "the panel is drawn even when one column is all that fits:\n{page}"
    );

    // The keys are named in the page footer, which needs a page wide enough to
    // hold the sentence: assert that where it is readable.
    // 这些按键写在页面页脚里，而页脚需要足够宽的页面才放得下整句：因此在那读得清的地方断言。
    let Some(mut wide) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut wide, "preview_canvas_width");
    search.graph_focus = 0;
    wide.overlay = Some(Overlay::Search(search));
    let mut terminal = Terminal::new(TestBackend::new(200, 40)).expect("a test terminal");
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut wide))
        .expect("the graph page draws");
    let footer = rendered_rows(&terminal)
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        footer.contains("[ ] split"),
        "the footer must name the split keys:\n{footer}"
    );

    assert_eq!(
        app.graph_split_percent, 35,
        "the test asked for a narrow tree"
    );
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char(']')));
    assert!(app.graph_split_percent > 35, "] gives the tree more room");
    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('[')));
    assert_eq!(app.graph_split_percent, 35, "[ takes it back");
    for _ in 0..40 {
        app.handle_overlay_key(KeyEvent::from(KeyCode::Char(']')));
    }
    assert_eq!(
        app.graph_split_percent, 65,
        "the key stops where the divider drag stops"
    );
}

/// A panel too narrow for two columns draws the tree downwards instead: the edges
/// stay visible, every box gets the panel's full width, and the hop is named on
/// the box rather than in a ruler the panel cannot spare.
/// 窄到放不下两列的面板改为向下画树：边仍然可见，每个盒子拿到面板整宽，跳数写在盒子上而不是
/// 写在面板腾不出的标尺里。
#[test]
fn a_narrow_panel_draws_the_tree_downwards() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    // A tree pane narrower than the width that fits two readable columns: the
    // page draws the tree downwards instead.
    // 比"两列都可读"所需宽度更窄的树面板：页面改为向下画树。
    app.graph_split_percent = 35;
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    app.overlay = Some(Overlay::Search(canvas_drawn(search)));
    let mut terminal = Terminal::new(TestBackend::new(80, 32)).expect("a test terminal");
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut app))
        .expect("the graph page draws");
    let page = rendered_rows(&terminal)
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    // The panel is 19 cells wide here, so it clips its own axis sentence: the
    // arrow and the first word are what a narrow reader is promised.
    // 这里的面板只有 19 格宽，因此它自己的轴说明也被裁：窄读者得到的是箭头与第一个词。
    assert!(
        page.contains("↑ callers"),
        "the axis sentence must follow the layout:\n{page}"
    );
    assert!(
        page.contains('▼'),
        "a top-down edge ends in a down arrow:\n{page}"
    );
    assert!(
        page.contains("clamp_canvas_"),
        "the name wraps in a box that has the panel's width:\n{page}"
    );
    assert!(
        page.contains("focus"),
        "the band that holds the focus says so:\n{page}"
    );

    // Forced top-down in a wide panel, the title names the mode it is in.
    // 在宽面板里强制自上而下时，标题写出它所在的模式。
    let Some(mut wide) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut wide, "preview_canvas_width");
    search.graph_focus = 0;
    search.tree_vertical = Some(true);
    wide.overlay = Some(Overlay::Search(canvas_drawn(search)));
    let mut terminal = Terminal::new(TestBackend::new(200, 50)).expect("a test terminal");
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut wide))
        .expect("the graph page draws");
    let page = rendered_rows(&terminal)
        .iter()
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        page.contains("↑ callers · call direction · callees ↓"),
        "the wide panel shows the whole axis sentence:\n{page}"
    );
    assert!(page.contains("vertical (v)"), "{page}");

    // Bands stack: two boxes in one band would share a row, and these do not.
    // 各带堆叠：同一条带里的两个盒子会共用一行，而这些不会。
    let boxes = app.hot.graph_tree_boxes.clone();
    assert!(boxes.len() >= 2, "both ends of the edge are drawn");
    let rows = boxes.iter().map(|(rect, _)| rect.y).collect::<Vec<_>>();
    let mut distinct = rows.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        rows.len(),
        "boxes stack instead of sharing rows"
    );
}

/// `v` cycles how the tree is laid out, and the panel says which way it ended up.
/// `v` 循环切换树的排布，面板写出它最终采用的方向。
#[test]
fn v_cycles_the_tree_layout() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    assert_eq!(search.tree_vertical, None, "the panel decides by default");
    app.overlay = Some(Overlay::Search(search));
    for expected in [Some(true), Some(false), None] {
        app.handle_overlay_key(KeyEvent::from(KeyCode::Char('v')));
        let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
            panic!("the graph stays open");
        };
        assert_eq!(search.tree_vertical, expected, "{}", app.event);
    }
}

/// The library drawer, for comparison with the hand-drawn canvases: `g` swaps
/// them at runtime, and both must put the same three nodes and the same two hops
/// on screen. The test prints the panel so the shapes can be compared by eye.
/// 库绘制，用于与手绘画布比较：`g` 在运行期切换两者，而两者都必须把同样的三个节点与两跳放上
/// 屏幕。测试会打印面板，便于用眼睛比较形状。
#[cfg(feature = "node-graph")]
#[test]
fn the_library_drawer_shows_the_same_tree() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    app.overlay = Some(Overlay::Search(search));
    // Wide enough that the panel can print the whole title, `(g)` included.
    // 宽到面板能打印完整标题，包括 `(g)`。
    let mut terminal = Terminal::new(TestBackend::new(200, 40)).expect("a test terminal");
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut app))
        .expect("the graph page draws");
    let page = rendered_rows(&terminal);
    let panel = tree_panel(&page).expect("the call tree panel is on screen");
    println!("── rataflow drawer ──");
    for line in &panel {
        println!("{}", line.iter().collect::<String>());
    }
    let text = panel
        .iter()
        .map(|line| line.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        text.contains("rataflow (g)"),
        "the title names the drawer:\n{text}"
    );
    for name in [
        "preview_canvas_width",
        "preview_canvas_width_traced",
        "clamp_canvas_width",
    ] {
        assert!(text.contains(name), "missing {name}:\n{text}");
    }
    assert!(
        text.contains('▶') || text.contains('╰') || text.contains('─'),
        "the widget must route the edges:\n{text}"
    );
}

/// Moving the tree cursor is *visible* in the library drawer: the mark travels
/// with it, which is what tells the reader that ↑↓ did something. This is the
/// regression the reader reported — a selection that only changed a colour was
/// no feedback at all in a terminal without one.
/// 库绘制里移动树游标是**看得见**的：标记跟着它走，这正是告诉读者 ↑↓ 起了作用的东西。这正是
/// 读者报告的回归——只改颜色的选中项，在没有颜色的终端里等于没有反馈。
#[cfg(feature = "node-graph")]
#[test]
fn the_library_drawer_marks_the_cursor_where_it_is() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    app.graph_split_percent = 80;
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    app.overlay = Some(Overlay::Search(search));
    let mut terminal = Terminal::new(TestBackend::new(170, 40)).expect("a test terminal");

    let marked = |terminal: &Terminal<TestBackend>| -> String {
        rendered_rows(terminal)
            .iter()
            .map(|row| row.iter().collect::<String>())
            .find(|row| row.contains("▶ "))
            .unwrap_or_default()
    };

    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut app))
        .expect("the graph page draws");
    let first = marked(&terminal);
    assert!(
        first.contains("preview_canvas_width"),
        "the focus starts under the cursor:\n{first}"
    );
    assert!(
        first.contains("▶"),
        "the cursor carries a mark that survives a colourless terminal"
    );

    app.handle_overlay_key(KeyEvent::from(KeyCode::Down));
    terminal
        .draw(|frame| crate::studio::ui::draw(frame, &mut app))
        .expect("the graph page draws");
    let second = marked(&terminal);
    assert_ne!(first, second, "the mark must move with the cursor");
    assert!(
        second.contains('▶'),
        "and it must still be a mark:\n{second}"
    );
}

/// The graph page's letters are commands, and typing is not one of them: the query
/// is edited in the list page, which `/` returns to. The catch-all typing arm used
/// to sit in front of `e`, so that key never fired — a conflict rustc cannot see,
/// because the arm it shadowed carries a guard.
/// 调用图页的字母是命令，"输入"不是其中之一：查询在列表页编辑，`/` 回到那里。兜底的输入分支
/// 曾经排在 `e` 前面，于是那个键永远不会触发——这是 rustc 看不见的冲突，因为它遮蔽的那条 arm
/// 带着守卫。
#[test]
fn the_graph_pages_letters_are_commands() {
    let Some(mut app) = fixture_app() else {
        return;
    };
    let mut search = open_graph(&mut app, "preview_canvas_width");
    search.graph_focus = 0;
    app.overlay = Some(Overlay::Search(search));

    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('z')));
    let Some(Overlay::Search(search)) = app.overlay.as_ref() else {
        panic!("the graph stays open");
    };
    assert_eq!(
        search.query, "preview_canvas_width",
        "a letter must not edit the query on this page"
    );

    app.handle_overlay_key(KeyEvent::from(KeyCode::Char('e')));
    assert!(
        app.overlay.is_none(),
        "`e` opens the cursor's source instead of being swallowed"
    );
    let Some((path, line)) = app.take_editor_request() else {
        panic!("`e` must request an editor");
    };
    assert!(path.is_file(), "{}", path.display());
    assert!(line > 1, "the request names a line");
}
