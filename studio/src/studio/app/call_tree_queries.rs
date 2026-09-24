//! Spatial call-tree queries owned by App.
//! App 所有的空间调用树查询。
//!
//! The three-column view and this tree are built from the same `call_relations`,
//! so they cannot disagree about who calls whom. What this module adds is the
//! kernel's layout on top of that relation list, a memo so one frame does not pay
//! for a dozen source scans, and the two gestures that move through the result:
//! one hop along the call direction, and re-centring on a node.
//! 三列视图与本调用树由同一个 `call_relations` 构建，因此两者在"谁调用谁"上不可能不一致。
//! 本模块在其上加入内核的布局、一个让一帧不必付十几次源码扫描代价的备忘，以及穿过结果的两种
//! 手势：沿调用方向移动一跳，以及以某个节点为新的焦点。

use std::collections::VecDeque;

use super::*;

/// One arrow press in the tree: the four directions a reader can see.
/// 树里的方向键：读者能看见的四个方向。
#[derive(Clone, Copy, Debug)]
pub(super) enum TreeStep {
    /// Toward the callers.
    /// 朝调用者。
    Up,
    /// Toward the callees.
    /// 朝被调用者。
    Down,
    /// Toward the previous sibling.
    /// 朝前一个同级。
    Left,
    /// Toward the next sibling.
    /// 朝后一个同级。
    Right,
}

impl TreeStep {
    /// The word the status line uses for this step.
    /// 状态行描述这一步时用的词。
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// How many hops the spatial call tree draws on each side of its focus.
/// 空间调用树在焦点两侧各绘制多少跳。
pub(crate) const CALL_TREE_DEPTH: usize = 2;
/// How many nodes the spatial call tree may hold, focus included.
/// 空间调用树最多容纳多少节点（含焦点）。
pub(crate) const CALL_TREE_NODES: usize = 16;

impl App {
    /// The call tree's rows, in the tree's own order: the focus first, then the
    /// nodes the kernel placed.
    /// 调用树的行，按树自身的顺序：焦点在首位，随后是内核放置的节点。
    ///
    /// Every row is a node now, so `Enter`, `e` and the mouse all act on
    /// something the drawing actually shows. The outline this replaced also
    /// carried header and transform rows, which is why the removal of the
    /// outline is visible here as a shorter, node-only list.
    /// 现在每一行都是节点，因此 `Enter`、`e` 与鼠标操作的对象都是画面上真实存在的东西。
    /// 它取代的大纲还带有分节与 transform 行，所以大纲的移除在这里表现为一份更短、只含节点的
    /// 列表。
    pub(crate) fn call_tree_targets(&self, item: &CallRef) -> Vec<Option<CallRef>> {
        self.call_tree_view(item).refs
    }

    /// The spatial call tree around one focus, memoised per source stamp.
    /// 某个焦点周围的空间调用树，按源码戳备忘。
    ///
    /// Building one walks the relations of every node it places, and each of
    /// those walks reads every source file, so one frame would otherwise pay for
    /// a dozen scans. The memo holds the two sides a frame can ask about and is
    /// keyed on the source stamp, so an edit invalidates it.
    /// 构建一棵树要遍历它放置的每个节点的关系，而每次遍历都要读一遍全部源文件，因此一帧
    /// 否则要付十几次扫描的代价。备忘保存一帧可能问到的两侧，并以源码戳为键，因此源码改动会
    /// 让它失效。
    pub(crate) fn call_tree_view(&self, focus: &CallRef) -> CallTreeView {
        if let Some(hit) = self.tree_cache.borrow().iter().find(|entry| {
            entry.stamp == self.last_source_stamp
                && entry.focus.node == focus.node
                && entry.focus.function == focus.function
        }) {
            return hit.view.clone();
        }
        let view = self.build_call_tree_view(focus);
        let mut cache = self.tree_cache.borrow_mut();
        cache.insert(
            0,
            CallTreeMemo {
                stamp: self.last_source_stamp,
                focus: focus.clone(),
                view: view.clone(),
            },
        );
        cache.truncate(2);
        view
    }

    /// Walk the relations reachable inside the budgets, then let the kernel lay
    /// the tree out.
    /// 在预算内遍历可达的关系，然后让内核完成树的布局。
    fn build_call_tree_view(&self, focus: &CallRef) -> CallTreeView {
        let mut symbols: Vec<CallRef> = vec![focus.clone()];
        let mut relations: Vec<CallRelation> = Vec::new();
        let mut queue: VecDeque<(usize, usize)> = VecDeque::from([(0usize, 0usize)]);
        while let Some((index, hop)) = queue.pop_front() {
            if hop >= CALL_TREE_DEPTH {
                continue;
            }
            let item = symbols[index].clone();
            let (callers, callees) = self.call_relations(item.node, &item.function);
            // Callees before callers, breadth-first by hop: the kernel expands in
            // exactly this order, so both sides agree on which symbols fit the
            // node budget and which ones become the cut counts.
            // 先被调用者后调用者，按跳数广度优先：内核也按这个顺序展开，因此两侧对
            // "哪些符号放得进节点预算、哪些变成裁剪计数"的看法一致。
            let mut pairs: Vec<(CallRef, CallRef)> = callees
                .iter()
                .map(|callee| (item.clone(), callee.clone()))
                .collect();
            pairs.extend(callers.iter().map(|caller| (caller.clone(), item.clone())));
            for (caller, callee) in pairs {
                let evidence = self.call_evidence(&caller, &callee);
                relations.push(CallRelation::from_symbols(
                    caller.function.clone(),
                    callee.function.clone(),
                    evidence,
                ));
                let other = if caller.node == item.node && caller.function == item.function {
                    callee
                } else {
                    caller
                };
                if let Some(placed) = push_call_symbol(&mut symbols, other) {
                    queue.push_back((placed, hop + 1));
                }
            }
        }
        let tree = call_tree(
            &focus.function,
            &relations,
            CALL_TREE_DEPTH,
            CALL_TREE_NODES,
        );
        let refs = tree
            .nodes
            .iter()
            .map(|node| {
                symbols
                    .iter()
                    .find(|symbol| same_symbol(&symbol.function, &node.symbol))
                    .cloned()
            })
            .collect();
        CallTreeView { tree, refs }
    }

    /// Move the call-tree cursor one hop along the call direction.
    /// 让调用树游标沿调用方向移动一跳。
    ///
    /// Move the tree cursor one step the way the reader pressed, along the
    /// *drawn* picture rather than along the model's own axes.
    /// 让树游标朝读者按下的方向走一步，沿**画出来的**图走，而不是沿模型自己的轴。
    ///
    /// Which model axis is "up" depends on how the panel draws: a top-down tree has
    /// levels running down the screen, a left-to-right one has them running across
    /// it. The picture is the authority — the panel's own sentence says which way
    /// the calls run — so the four keys keep their printed meaning in both layouts,
    /// and a level step follows a drawn edge that exists instead of jumping to
    /// whatever happens to be numbered next.
    /// 哪个模型轴是"上"取决于面板怎么画：自上而下的树让层沿屏幕向下延伸，从左到右的树让层横着
    /// 延伸。以图为准——面板自己的那句话写明了调用方向——因此四个键在两种排布里都保持字面含义，
    /// 而沿层的移动走的是确实存在的那条画出来的边，而不是跳到"编号恰好下一个"的节点上。
    pub(super) fn hop_call_tree(&mut self, search: &mut SearchState, step: TreeStep) {
        let Some(focus) = self.graph_item(search) else {
            return;
        };
        let view = self.call_tree_view(&focus);
        let nodes = &view.tree.nodes;
        let cursor = search.outline_selected;
        let Some(node) = nodes.get(cursor) else {
            return;
        };
        // The grid the panel drew: which model coordinate runs down the screen.
        // 面板画出的网格：哪个模型坐标沿屏幕向下。
        let (level_step, lane_step) = match (self.tree_top_down, step) {
            (true, TreeStep::Up) | (false, TreeStep::Left) => (-1, 0),
            (true, TreeStep::Down) | (false, TreeStep::Right) => (1, 0),
            (true, TreeStep::Left) | (false, TreeStep::Up) => (0, -1),
            _ => (0, 1),
        };
        let target = if level_step != 0 {
            // One hop along a drawn edge: the node that placed this one, or one it
            // placed. The same lane is preferred, so a branch stays a branch.
            // 沿一条画出来的边走一跳：放置本节点的节点，或它放置的节点。优先同一条车道，
            // 使一条分支保持是一条分支。
            let wanted = node.level + level_step;
            let mut candidates = nodes
                .iter()
                .enumerate()
                .filter(|(index, other)| {
                    other.level == wanted
                        && (other.parent == Some(cursor) || node.parent == Some(*index))
                })
                .map(|(index, other)| (index, other.lane));
            let same_lane = candidates.clone().find(|(_, lane)| *lane == node.lane);
            same_lane
                .or_else(|| candidates.next())
                .map(|(index, _)| index)
        } else {
            // Sideways is the lane the reader sees: the nearest neighbour in that
            // direction inside the same band.
            // 横向就是读者看到的车道：同一条带内该方向上最近的一个。
            nodes
                .iter()
                .enumerate()
                .filter(|(_, other)| other.level == node.level)
                .filter(|(_, other)| match lane_step {
                    -1 => other.lane < node.lane,
                    _ => other.lane > node.lane,
                })
                .min_by_key(|(_, other)| other.lane.abs_diff(node.lane))
                .map(|(index, _)| index)
        };
        let Some(target) = target else {
            self.event = format!(
                "{} has no {} node inside the {CALL_TREE_DEPTH}-hop call tree",
                node.symbol,
                step.label()
            );
            return;
        };
        search.outline_selected = target;
        search.data_selected = 0;
        self.event = format!(
            "Call tree cursor: {} ({})",
            nodes[target].symbol,
            step.label()
        );
    }

    /// Make one tree node the focus of its side.
    /// 让某个树节点成为它那一侧的焦点。
    ///
    /// Returns `false` when the node is the focus already; the caller then opens
    /// the editor.
    /// 该节点已是焦点时返回 `false`；此时调用方改为打开编辑器。
    pub(super) fn recentre_call_tree(&mut self, search: &mut SearchState, target: CallRef) -> bool {
        let (center, function) = (search.center, search.center_function.as_deref());
        if center == Some(target.node) && function == Some(target.function.as_str()) {
            return false;
        }
        let line = self.source_function_line(target.node, &target.function);
        self.event = format!("Call tree re-centred on {}", target.function);
        search.center = Some(target.node);
        search.center_function = Some(target.function);
        search.center_line = line;
        search.outline_selected = 0;
        search.data_selected = 0;
        true
    }
}

/// Add one symbol to the tree's node list, mirroring the kernel's placement
/// rule: an already placed symbol is not placed again, and the node budget
/// bounds the list. Symbols match the way the kernel matches them, so a bare
/// name and its qualified path are one node rather than two.
/// 把一个符号加入树的节点列表，规则与内核的放置一致：已放置的符号不再放置，节点预算限制
/// 列表长度。符号按内核的方式匹配，因此裸名与它的限定路径是同一个节点而不是两个。
fn push_call_symbol(symbols: &mut Vec<CallRef>, item: CallRef) -> Option<usize> {
    if symbols
        .iter()
        .any(|existing| same_symbol(&existing.function, &item.function))
    {
        return None;
    }
    if symbols.len() >= CALL_TREE_NODES {
        return None;
    }
    symbols.push(item);
    Some(symbols.len() - 1)
}
