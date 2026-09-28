//! The call-graph delta between two MIR artifacts.
//! 两份 MIR artifact 之间的调用图差异。
//!
//! A MIR artifact is a snapshot of *some* tree's compiler candidates, and the
//! question an agent actually has is "did this refactor change the call graph" —
//! which only a comparison of two snapshots answers. This is pure set arithmetic
//! over the relations and function symbols, so the same pair always yields the
//! same answer wherever it is diffed; whether the two artifacts are even allowed
//! to be compared is a separate question, and it belongs to the surface that can
//! see the snapshots' identities.
//! MIR artifact 是**某棵**树的编译器候选的快照，而代理真正有的问题是"这次重构改了调用图吗"——
//! 只有两份快照相比较才答得出来。这里是关于关系与函数符号的纯集合运算，因此同一对在任何地方作差都
// 给出同一答案；两个 artifact 是否**允许**比较是另一个问题，属于那个能看到快照身份的执行面。

use std::collections::BTreeSet;

use super::model::MirGraph;

/// One call relation in a delta, named by its two symbols.
/// 差异中的一条调用关系，由其两端符号命名。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirCallDelta {
    /// Function symbol making the call.
    /// 发起调用的函数符号。
    pub caller: String,
    /// Function symbol being called.
    /// 被调用的函数符号。
    pub callee: String,
}

/// What one artifact's call graph adds to, or drops from, another's.
/// 一份 artifact 的调用图相对另一份增加或去掉了什么。
///
/// Relations are matched as `(caller, callee)` pairs and deduplicated, because a
/// call graph has an edge once however many times the source repeats it; the
/// MIR line number is deliberately not part of the identity, since it moves when
/// anything above it in the same function changes.
/// 关系按 `(caller, callee)` 对匹配并去重，因为一条边在调用图里只存在一次，无论源码重复多少次；
/// MIR 行号有意不参与身份，因为同一函数里它上方任何改动都会让它移动。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MirDelta {
    /// Relations `after` has that the baseline does not.
    /// `after` 有而基线没有的关系。
    pub added: Vec<MirCallDelta>,
    /// Relations the baseline has that `after` does not.
    /// 基线有而 `after` 没有的关系。
    pub gone: Vec<MirCallDelta>,
    /// Function symbols `after` declares that the baseline does not.
    /// `after` 声明而基线没有的函数符号。
    pub functions_added: Vec<String>,
    /// Function symbols the baseline declares that `after` does not.
    /// 基线声明而 `after` 没有的函数符号。
    pub functions_gone: Vec<String>,
}

impl MirGraph {
    /// The call-graph delta from `self` (the baseline) to `after`.
    /// 从 `self`（基线）到 `after` 的调用图差异。
    pub fn delta(&self, after: &MirGraph) -> MirDelta {
        let before = call_pairs(self);
        let now = call_pairs(after);
        MirDelta {
            added: difference(&now, &before),
            gone: difference(&before, &now),
            functions_added: after
                .functions
                .difference(&self.functions)
                .cloned()
                .collect(),
            functions_gone: self
                .functions
                .difference(&after.functions)
                .cloned()
                .collect(),
        }
    }
}

/// The distinct call relations of one graph, sorted.
/// 一张图的去重调用关系，已排序。
fn call_pairs(graph: &MirGraph) -> BTreeSet<(String, String)> {
    graph
        .calls
        .iter()
        .map(|call| (call.caller.clone(), call.callee.clone()))
        .collect()
}

/// The pairs in `left` that `right` does not have, as sorted rows.
/// `left` 有而 `right` 没有的对，按排序后的行给出。
fn difference(
    left: &BTreeSet<(String, String)>,
    right: &BTreeSet<(String, String)>,
) -> Vec<MirCallDelta> {
    left.difference(right)
        .map(|(caller, callee)| MirCallDelta {
            caller: caller.clone(),
            callee: callee.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::MirGraph;

    /// A graph from JSONL, which is how a snapshot arrives.
    /// 从 JSONL 构建一张图——快照就是这样到来的。
    fn graph(records: &str) -> MirGraph {
        MirGraph::from_jsonl(records).expect("a fixture graph parses")
    }

    /// An added call and a dropped one are both named, and a relation repeated
    /// in the artifact counts once: the edge is what changed, not the sighting.
    /// 新增的调用与被去掉的调用都会被点名，而 artifact 里重复出现的关系只算一次：变的是那条边，
    /// 不是观测次数。
    #[test]
    fn added_and_gone_relations_are_named_once_each() {
        let before = graph(
            "{\"kind\":\"function\",\"name\":\"a\"}\n{\"kind\":\"function\",\"name\":\"b\"}\n{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"b\",\"mir_line\":3}\n",
        );
        let after = graph(
            "{\"kind\":\"function\",\"name\":\"a\"}\n{\"kind\":\"function\",\"name\":\"c\"}\n{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"c\",\"mir_line\":5}\n{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"c\",\"mir_line\":9}\n",
        );
        let delta = before.delta(&after);
        assert_eq!(delta.added.len(), 1, "{delta:?}");
        assert_eq!(delta.added[0].caller, "a");
        assert_eq!(delta.added[0].callee, "c");
        assert_eq!(delta.gone.len(), 1, "{delta:?}");
        assert_eq!(delta.gone[0].callee, "b");
        assert_eq!(delta.functions_added, ["c"]);
        assert_eq!(delta.functions_gone, ["b"]);
    }

    /// A MIR line that moved is not a graph change: the edge survived the edit,
    /// and reporting it would call every refactor a call-graph change.
    /// MIR 行号移动不是调用图变化：边在编辑中活了下来，报出来会把每次重构都说成调用图变化。
    #[test]
    fn a_moved_mir_line_is_not_a_delta() {
        let before =
            graph("{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"b\",\"mir_line\":3}\n");
        let after =
            graph("{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"b\",\"mir_line\":30}\n");
        let delta = before.delta(&after);
        assert_eq!(delta, super::MirDelta::default(), "{delta:?}");
    }

    /// The baseline direction is the claim: `added` is what the second artifact
    /// has, so swapping the two swaps the lists.
    /// 基线的方向就是主张：`added` 是第二份 artifact 有的东西，因此交换两者就交换两份清单。
    #[test]
    fn the_delta_reads_from_the_baseline_to_the_other_artifact() {
        let one = graph("{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"b\",\"mir_line\":1}\n");
        let two = graph(
            "{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"b\",\"mir_line\":1}\n{\"kind\":\"call\",\"caller\":\"b\",\"callee\":\"c\",\"mir_line\":2}\n",
        );
        assert_eq!(one.delta(&two).added.len(), 1);
        assert_eq!(two.delta(&one).gone.len(), 1);
        assert!(one.delta(&two).gone.is_empty());
    }
}
