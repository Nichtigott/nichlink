//! Small rustc/MIR bridge for NichLink's debug model.
//! NichLink 调试模型使用的轻量 rustc/MIR 桥接层。
//!
//! Static parsing and evidence merging live in the kernel `mir` module;
//! this file keeps the runtime-coupled merge entry point and the petgraph
//! topology adapter wiring.
//! 静态解析与证据归并在 kernel 的 `mir` 模块；本文件保留与运行期耦合的
//! 归并入口和 petgraph 拓扑适配接线。

use std::fmt::Write as _;

pub use nichlink_run_method::registry_core::mir::{
    CallEvidence, CallRelation, MirCall, MirGraph, MirLocal, MirParseError, merge_call_relations,
};
use nichlink_run_method::{CallEdge, CallTrace};

/// Static MIR candidates plus calls observed in one live run.
/// 静态 MIR 候选边与一次运行中真实观察到的调用边。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnifiedCallGraph {
    pub static_calls: Vec<MirCall>,
    pub runtime_calls: Vec<CallEdge>,
}

impl UnifiedCallGraph {
    pub fn new(static_graph: &MirGraph, trace: &CallTrace) -> Self {
        Self {
            static_calls: static_graph.calls.clone(),
            runtime_calls: trace.call_edges(),
        }
    }

    /// Delegate topology queries to the petgraph-backed adapter.
    /// 将拓扑查询委托给基于 petgraph 的适配层。
    pub fn topology(&self) -> crate::adapters::CallGraph {
        crate::adapters::CallGraph::from_relations(&self.relations())
    }

    /// Merge all known edges into one evidence-aware relation list.
    /// 将所有已知边合并为一份带证据等级的关系列表。
    ///
    /// Live edges win over MIR candidates with the same logical symbols. A
    /// static candidate that was not observed remains visible as `Mir`, so a
    /// missing branch is not silently mistaken for a successful call.
    /// 逻辑符号相同的边以 Live 证据为准。未被观察到的静态候选仍保留为
    /// `Mir`，不会把未执行分支误报成已经成功调用。
    pub fn relations(&self) -> Vec<CallRelation> {
        merge_call_relations(&self.static_calls, &self.runtime_calls)
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        let relations = self.relations();
        output.push_str("CALL RELATIONS\n");
        if relations.is_empty() {
            output.push_str("  (none)\n");
        } else {
            for relation in relations {
                let location = relation
                    .source
                    .map_or_else(String::new, |source| format!(" @ {source}"));
                let mir_line = relation
                    .mir_line
                    .map_or_else(String::new, |line| format!(" @ MIR {line}"));
                let frames = match (relation.caller_frame, relation.callee_frame) {
                    (Some(caller), Some(callee)) => format!(" frames={caller}->{callee}"),
                    _ => String::new(),
                };
                writeln!(
                    output,
                    "  {} {} -> {} [{}]{}{}{}",
                    relation.evidence.marker(),
                    relation.caller,
                    relation.callee,
                    relation.evidence.label(),
                    location,
                    mir_line,
                    frames,
                )
                .unwrap();
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::{CallEvidence, MirGraph, UnifiedCallGraph};
    use nichlink_run_method::{CallTrace, NodeId, SourceLocation};

    #[test]
    fn keeps_static_and_live_edges_distinct() {
        let graph = MirGraph::from_jsonl(
            "{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"b\",\"mir_line\":1}\n",
        )
        .unwrap();
        let mut trace = CallTrace::full();
        trace.with_at(
            NodeId::from_path("a.rs", "A"),
            "a",
            SourceLocation {
                file: "a.rs",
                line: 1,
                column: 1,
                function: "a",
            },
            |trace| {
                trace.with_at(
                    NodeId::from_path("b.rs", "B"),
                    "b",
                    SourceLocation {
                        file: "b.rs",
                        line: 2,
                        column: 1,
                        function: "b",
                    },
                    |_| {},
                );
            },
        );
        let unified = UnifiedCallGraph::new(&graph, &trace);
        assert_eq!(unified.static_calls.len(), 1);
        assert_eq!(unified.runtime_calls.len(), 1);
        assert!(unified.render().contains("CALL RELATIONS"));
        assert_eq!(unified.relations()[0].evidence, CallEvidence::Live);
        assert_eq!(unified.relations().len(), 1);
        assert_eq!(unified.topology().edge_count(), 1);
    }

    #[test]
    fn keeps_unobserved_mir_candidate_as_a_distinct_relation() {
        let graph = MirGraph::from_jsonl(
            "{\"kind\":\"call\",\"caller\":\"crate::a\",\"callee\":\"crate::c\",\"mir_line\":7}\n",
        )
        .unwrap();
        let trace = CallTrace::full();
        let unified = UnifiedCallGraph::new(&graph, &trace);
        let relations = unified.relations();
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].evidence, CallEvidence::Mir);
        assert_eq!(relations[0].mir_line, Some(7));
        assert!(!relations[0].evidence.confirmed());
    }
}
