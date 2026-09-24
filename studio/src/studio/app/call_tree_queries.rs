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

use std::cmp::Ordering;
use std::collections::VecDeque;

use super::*;

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
    /// Upstream is "who calls this" and downstream is "what this calls". The
    /// cursor walks the drawn tree's own edges, and because the caller half is
    /// drawn on the left, moving toward the focus *is* moving downstream there —
    /// which is why the mapping flips with the side.
    /// 上游是"谁在调它"，下游是"它调用了谁"。游标走的是所画树自己的边；由于调用者半边画在
    /// 左边，在那半边朝焦点走就是往下游走，因此映射随所在半边翻转。
    pub(super) fn hop_call_tree(&mut self, search: &mut SearchState, downstream: bool) {
        let side = search.graph_side;
        let Some(focus) = self.graph_item(search, side) else {
            return;
        };
        let view = self.call_tree_view(&focus);
        let cursor = if side == 1 {
            search.compare_outline_selected
        } else {
            search.outline_selected
        };
        let Some(node) = view.tree.nodes.get(cursor) else {
            return;
        };
        let child = |from: usize| {
            view.tree.nodes.iter().position(|candidate| {
                candidate.parent == Some(from) && (candidate.level > 0) == downstream
            })
        };
        let target = match (node.level.cmp(&0), downstream) {
            (Ordering::Less, true) | (Ordering::Greater, false) => node.parent,
            _ => child(cursor),
        };
        let Some(target) = target else {
            self.event = format!(
                "{} has no {} hop inside the {CALL_TREE_DEPTH}-hop call tree",
                node.symbol,
                if downstream { "downstream" } else { "upstream" }
            );
            return;
        };
        if side == 1 {
            search.compare_outline_selected = target;
        } else {
            search.outline_selected = target;
        }
        search.data_selected = 0;
        self.event = format!(
            "Call tree cursor: {} ({} hop)",
            view.tree.nodes[target].symbol,
            if downstream { "downstream" } else { "upstream" }
        );
    }

    /// Make one tree node the focus of its side.
    /// 让某个树节点成为它那一侧的焦点。
    ///
    /// Returns `false` when the node is the focus already; the caller then opens
    /// the editor, which is the same gesture the three-column view uses.
    /// 该节点已是焦点时返回 `false`；此时调用方改为打开编辑器，与三列视图的手势一致。
    pub(super) fn recentre_call_tree(&mut self, search: &mut SearchState, target: CallRef) -> bool {
        let (center, function) = if search.graph_side == 1 {
            (
                search.compare_center,
                search.compare_center_function.as_deref(),
            )
        } else {
            (search.center, search.center_function.as_deref())
        };
        if center == Some(target.node) && function == Some(target.function.as_str()) {
            return false;
        }
        let line = self.source_function_line(target.node, &target.function);
        self.event = format!("Call tree re-centred on {}", target.function);
        if search.graph_side == 1 {
            search.compare_center = Some(target.node);
            search.compare_center_function = Some(target.function);
            search.compare_center_line = line;
            search.compare_outline_selected = 0;
        } else {
            search.center = Some(target.node);
            search.center_function = Some(target.function);
            search.center_line = line;
            search.outline_selected = 0;
        }
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
