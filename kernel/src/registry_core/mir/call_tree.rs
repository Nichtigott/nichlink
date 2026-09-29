//! Layered call tree over normalized call relations.
//! 归一化调用关系上的分层调用树。
//!
//! Studio's spatial tree needs three things from the kernel, and none of them
//! touches the filesystem: which symbols a focus reaches inside a hop and node
//! budget, where each of them sits (column = hop, lane = row inside the
//! column), and how many edges the drawing had to leave out. [`call_tree`]
//! computes all three from a relation list the caller already holds, so the
//! kernel keeps its side of the bargain — vocabulary and pure methods only.
//! Studio 的空间调用树需要内核提供三件事，而没有一件会碰文件系统：在跳数与节点预算内
//! 焦点能到达哪些符号、它们各自坐在哪里（列 = 跳数，行 = 该列内的车道），以及这张图不得
//! 不漏掉多少条边。[`call_tree`] 从调用方已有的关系列表算出这三件事，因此内核仍然只提供
//! 词汇与纯方法。
//!
//! Two invariants shape the layout, and both are testable:
//! 两条不变量决定布局，且两条都可测：
//!
//! - Every relation runs caller → callee and callers are placed at negative
//!   levels, so an edge is drawn left to right exactly when
//!   `callee.level > caller.level`. The others are back or cross edges; they are
//!   reported with `forward = false` instead of being silently dropped.
//!   每条关系都由调用者指向被调用者，而调用者放在负层，因此一条边恰好当
//!   `callee.level > caller.level` 时被画成从左到右。其余的是回边或横边，它们以
//!   `forward = false` 报告出来，而不是被悄悄丢掉。
//! - Two nodes never share a lane in the same column, even when two parents call
//!   the same function: two parents drawn on one row is exactly the diamond this
//!   diagram exists to show.
//!   同一列内两个节点绝不共用一条车道，即使两个父节点调用同一个函数：两个父节点画在同一
//!   行上，恰恰抹掉了这张图要展示的那个菱形。
//!
//! A node is expanded only the first time it is placed, which both keeps a cyclic
//! call graph from recursing forever and gives every symbol its nearest hop. The
//! price is that edges which would need a second, farther copy of a symbol are
//! reported as non-forward.
//! 一个节点只在首次放置时展开，这既让带环的调用图不会无限递归，也让每个符号落在它最近的
//! 跳数上。代价是：那些需要该符号第二份更远副本的边，会以非前向边的形式报告出来。

use std::collections::BTreeMap;

use super::merge::same_symbol;
use super::model::CallRelation;
use crate::registry_core::declaration::EvidenceKind;

/// One placed symbol in the call tree.
/// 调用树中已放置的一个符号。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallTreeNode {
    /// Symbol label, spelled the way it was first seen.
    /// 符号标签，按首次见到的写法保留。
    pub symbol: String,
    /// Signed hop from the focus: negative upstream, zero the focus, positive
    /// downstream.
    /// 相对焦点的带符号跳数：负为上游，零为焦点，正为下游。
    pub level: i32,
    /// Row inside this node's column. Two nodes never share one.
    /// 本节点所在列内的行号。同一列内两个节点绝不共用一行。
    pub lane: usize,
    /// Index of the node that placed this one; `None` for the focus.
    /// 放置本节点的节点下标；焦点为 `None`。
    pub parent: Option<usize>,
    /// Distinct callers pointing at this node that the tree did not draw,
    /// because they were past the hop budget or the node budget ran out.
    /// 指向本节点、但树中没有画出的调用者个数（已去重），原因是超出跳数预算或节点预算用尽。
    pub cut_callers: usize,
    /// Distinct callees of this node that the tree did not draw.
    /// 本节点调用、但树中没有画出的被调用者个数（已去重）。
    pub cut_callees: usize,
}

/// One call relation whose two ends are both placed in the tree.
/// 一条两端都已放置在树中的调用关系。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallTreeEdge {
    /// Index of the calling node.
    /// 调用方节点的下标。
    pub caller: usize,
    /// Index of the called node.
    /// 被调用方节点的下标。
    pub callee: usize,
    /// Strongest evidence supporting this relation.
    /// 支持该关系的最强证据。
    pub evidence: EvidenceKind,
    /// Whether the edge runs along the drawn axis, i.e. left to right.
    /// 该边是否沿绘制轴（即从左到右）延伸。
    pub forward: bool,
}

/// A focus and every symbol it reaches inside the budgets.
/// 一个焦点，以及它在预算内能到达的全部符号。
///
/// `nodes[0]` is always the focus; the rest follow in placement order, which is
/// breadth-first over the hops, so a caller that owns a raw index also owns a
/// stable cursor order.
/// `nodes[0]` 始终是焦点；其余按放置顺序排列，即按跳数广度优先。因此持有裸下标的调用方
/// 也就持有一份稳定的游标顺序。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallTree {
    /// Placed nodes, focus first.
    /// 已放置的节点，焦点在首位。
    pub nodes: Vec<CallTreeNode>,
    /// Every relation whose two ends are both placed.
    /// 两端都已放置的全部关系。
    pub edges: Vec<CallTreeEdge>,
    /// Whether at least one symbol was dropped because the node budget ran out.
    /// 是否至少有一个符号因为节点预算用尽而被丢弃。
    pub truncated: bool,
}

impl CallTree {
    /// Rows the drawing needs, i.e. the highest lane in use plus one.
    /// 绘制所需行数，即使用中的最大车道加一。
    pub fn rows(&self) -> usize {
        self.nodes
            .iter()
            .map(|node| node.lane + 1)
            .max()
            .unwrap_or(1)
    }

    /// Lowest and highest level in use, i.e. the columns the drawing needs.
    /// 使用中的最低与最高层，即绘制所需的列范围。
    pub fn level_span(&self) -> (i32, i32) {
        self.nodes.iter().fold((0, 0), |(low, high), node| {
            (low.min(node.level), high.max(node.level))
        })
    }

    /// Index of the placed node matching a symbol, as the kernel matches
    /// symbols elsewhere: a bare name matches a qualified path suffix.
    /// 匹配某个符号的节点下标；匹配规则与内核别处一致：裸名可匹配限定路径的后缀。
    pub fn index_of(&self, symbol: &str) -> Option<usize> {
        self.nodes
            .iter()
            .position(|node| same_symbol(&node.symbol, symbol))
    }

    /// Forced edges, i.e. the relations the axis could not draw left to right.
    /// 反向边，即无法沿轴从左到右画出的那些关系。
    pub fn backward_edges(&self) -> usize {
        self.edges.iter().filter(|edge| !edge.forward).count()
    }
}

/// Build the layered tree a focus reaches inside a hop and node budget.
/// 构建焦点在跳数与节点预算内到达的分层树。
///
/// `depth` counts hops outwards and `limit` counts nodes including the focus; a
/// `limit` of zero is treated as one, so the focus is always present and callers
/// can rely on `nodes[0]`.
/// `depth` 是向外的跳数，`limit` 是包含焦点在内的节点数；`limit` 为零时按一处理，
/// 因此焦点始终存在，调用方可依赖 `nodes[0]`。
pub fn call_tree(focus: &str, relations: &[CallRelation], depth: usize, limit: usize) -> CallTree {
    let limit = limit.max(1);
    let mut nodes = vec![CallTreeNode {
        symbol: focus.to_owned(),
        level: 0,
        lane: 0,
        parent: None,
        cut_callers: 0,
        cut_callees: 0,
    }];
    let mut truncated = false;
    let mut queue = std::collections::VecDeque::from([0usize]);
    while let Some(index) = queue.pop_front() {
        let level = nodes[index].level;
        if level.unsigned_abs() as usize >= depth {
            continue;
        }
        let symbol = nodes[index].symbol.clone();
        // Callees before callers: the tree grows along the call direction, so a
        // mutual recursion reports one back edge instead of dragging the three
        // consumers of its downstream half up into the upstream column. Studio
        // walks its own relations in this order too, so both sides agree on
        // which symbols made it in when the budget runs out.
        // 先被调用者后调用者：树沿调用方向生长，因此互相递归只报告一条回边，而不会把下游那一半
        // 的三个消费者拖进上游列。Studio 也按这个顺序遍历自己的关系，因此预算用尽时两侧对
        // "哪些符号进来了"的看法一致。
        //
        // Expansion only ever walks *away* from the focus: an upstream node
        // expands its callers, a downstream node expands its callees. Without
        // that rule a callee's other callers would be placed in the focus's own
        // column, and the column that is supposed to say "this is the node you
        // asked about" would fill up with its siblings.
        // 展开只会*离开*焦点:上游节点展开它的调用者,下游节点展开它的被调用者。没有这条规则,
        // 某个被调用者的其它调用者会被放进焦点自己那一列,而那一列本应表示"这就是你要问的
        // 节点",却会被它的兄弟节点填满。
        if level >= 0 {
            for relation in relations
                .iter()
                .filter(|relation| same_symbol(&relation.caller, &symbol))
            {
                if let Some(placed) = place(
                    &relation.callee,
                    level + 1,
                    index,
                    &mut nodes,
                    limit,
                    &mut truncated,
                ) {
                    queue.push_back(placed);
                }
            }
        }
        if level <= 0 {
            for relation in relations
                .iter()
                .filter(|relation| same_symbol(&relation.callee, &symbol))
            {
                if let Some(placed) = place(
                    &relation.caller,
                    level - 1,
                    index,
                    &mut nodes,
                    limit,
                    &mut truncated,
                ) {
                    queue.push_back(placed);
                }
            }
        }
    }
    count_cuts(&mut nodes, relations);
    let edges = collect_edges(&nodes, relations);
    assign_lanes(&mut nodes);
    CallTree {
        nodes,
        edges,
        truncated,
    }
}

/// Place one symbol if it is new and there is room; report a fresh index only,
/// so an already placed symbol is never expanded twice. A new symbol that does
/// not fit sets `truncated`, because that is the moment the view stops being
/// complete.
/// 若该符号是新的且还有位置就放置它；只报告新放置的下标，因此已放置的符号不会被展开两次。
/// 新符号放不下时会置位 `truncated`，因为那正是这个视图不再完整的时刻。
fn place(
    symbol: &str,
    level: i32,
    parent: usize,
    nodes: &mut Vec<CallTreeNode>,
    limit: usize,
    truncated: &mut bool,
) -> Option<usize> {
    if nodes.iter().any(|node| same_symbol(&node.symbol, symbol)) {
        return None;
    }
    if nodes.len() >= limit {
        *truncated = true;
        return None;
    }
    nodes.push(CallTreeNode {
        symbol: symbol.to_owned(),
        level,
        lane: 0,
        parent: Some(parent),
        cut_callers: 0,
        cut_callees: 0,
    });
    Some(nodes.len() - 1)
}

/// Count, per node, the distinct neighbours the tree left out.
/// 为每个节点统计树漏掉的不同邻居数量。
fn count_cuts(nodes: &mut [CallTreeNode], relations: &[CallRelation]) {
    let symbols = nodes
        .iter()
        .map(|node| node.symbol.clone())
        .collect::<Vec<_>>();
    for index in 0..nodes.len() {
        let symbol = symbols[index].clone();
        let mut missing_callers: Vec<String> = Vec::new();
        let mut missing_callees: Vec<String> = Vec::new();
        for relation in relations {
            if same_symbol(&relation.callee, &symbol)
                && !symbols
                    .iter()
                    .any(|placed| same_symbol(placed, &relation.caller))
                && !missing_callers
                    .iter()
                    .any(|seen| same_symbol(seen, &relation.caller))
            {
                missing_callers.push(relation.caller.clone());
            }
            if same_symbol(&relation.caller, &symbol)
                && !symbols
                    .iter()
                    .any(|placed| same_symbol(placed, &relation.callee))
                && !missing_callees
                    .iter()
                    .any(|seen| same_symbol(seen, &relation.callee))
            {
                missing_callees.push(relation.callee.clone());
            }
        }
        nodes[index].cut_callers = missing_callers.len();
        nodes[index].cut_callees = missing_callees.len();
    }
}

/// Keep one edge per caller/callee pair, with the strongest evidence seen.
/// 每对调用者/被调用者只保留一条边，证据取见过的最强者。
fn collect_edges(nodes: &[CallTreeNode], relations: &[CallRelation]) -> Vec<CallTreeEdge> {
    let mut edges: Vec<CallTreeEdge> = Vec::new();
    for relation in relations {
        let Some(caller) = nodes
            .iter()
            .position(|node| same_symbol(&node.symbol, &relation.caller))
        else {
            continue;
        };
        let Some(callee) = nodes
            .iter()
            .position(|node| same_symbol(&node.symbol, &relation.callee))
        else {
            continue;
        };
        let forward = nodes[callee].level > nodes[caller].level;
        match edges
            .iter_mut()
            .find(|edge| edge.caller == caller && edge.callee == callee)
        {
            Some(edge) => {
                if strength(relation.evidence) > strength(edge.evidence) {
                    edge.evidence = relation.evidence;
                }
            }
            None => edges.push(CallTreeEdge {
                caller,
                callee,
                evidence: relation.evidence,
                forward,
            }),
        }
    }
    edges
}

/// Order evidence from unconvincing to confirmed.
/// 把证据从最不可信排到已确认。
fn strength(evidence: EvidenceKind) -> u8 {
    match evidence {
        EvidenceKind::Unknown => 0,
        EvidenceKind::External => 1,
        EvidenceKind::Source => 2,
        EvidenceKind::Mir => 3,
        EvidenceKind::Live => 4,
    }
}

/// Give every node its lane: leaves take the next free row of their column, a
/// parent sits on the mean of its children, and a taken row pushes the node
/// down. The mean keeps a parent beside the calls it owns; the push-down is what
/// stops two parents of one shared child from landing on the same row.
/// 给每个节点分配车道：叶子取本列下一个空行，父节点坐在子节点的中位，行被占用就向下推。
/// 中位让父节点挨着它自己的那些调用；向下推则是阻止同一个共用子节点的两个父节点落在同一行
/// 的东西。
fn assign_lanes(nodes: &mut [CallTreeNode]) {
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for (index, node) in nodes.iter().enumerate() {
        if let Some(parent) = node.parent {
            children[parent].push(index);
        }
    }
    let mut taken: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    let mut next: BTreeMap<i32, usize> = BTreeMap::new();
    assign_lane(0, nodes, &children, &mut taken, &mut next);
}

/// Assign one node's lane after its children have theirs.
/// 在子节点取好车道之后分配该节点的车道。
fn assign_lane(
    index: usize,
    nodes: &mut [CallTreeNode],
    children: &[Vec<usize>],
    taken: &mut BTreeMap<i32, Vec<usize>>,
    next: &mut BTreeMap<i32, usize>,
) {
    let level = nodes[index].level;
    let base = if children[index].is_empty() {
        let counter = next.entry(level).or_insert(0);
        let lane = *counter;
        *counter += 1;
        lane
    } else {
        for child in children[index].clone() {
            assign_lane(child, nodes, children, taken, next);
        }
        let lanes = children[index]
            .iter()
            .map(|child| nodes[*child].lane)
            .collect::<Vec<_>>();
        let low = lanes.iter().copied().min().unwrap_or(0);
        let high = lanes.iter().copied().max().unwrap_or(0);
        (low + high) / 2
    };
    let column = taken.entry(level).or_default();
    let mut lane = base;
    while column.contains(&lane) {
        lane += 1;
    }
    column.push(lane);
    nodes[index].lane = lane;
}

#[cfg(test)]
#[path = "call_tree_tests.rs"]
mod call_tree_tests;
