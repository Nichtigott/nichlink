//! Evidence-aware merge of static MIR candidates and live call edges.
//! 静态 MIR 候选与 live 调用边的证据归并。

use crate::registry_core::declaration::{CallEdge, EvidenceKind};

use super::model::{CallRelation, MirCall};

/// Merge static MIR candidates with live edges into one evidence-aware
/// relation list.
/// 将静态 MIR 候选与 live 边归并为一份带证据等级的关系列表。
///
/// Live edges win over MIR candidates with the same logical symbols. A
/// static candidate that was not observed remains visible as `Mir`, so a
/// missing branch is not silently mistaken for a successful call.
/// 逻辑符号相同的边以 Live 证据为准。未被观察到的静态候选仍保留为
/// `Mir`，不会把未执行分支误报成已经成功调用。
pub fn merge_call_relations(
    static_calls: &[MirCall],
    runtime_calls: &[CallEdge],
) -> Vec<CallRelation> {
    let mut relations = Vec::new();
    for edge in runtime_calls {
        relations.push(CallRelation {
            caller: edge.caller.function.to_owned(),
            callee: edge.callee.function.to_owned(),
            evidence: EvidenceKind::Live,
            source: edge.callee.source,
            mir_line: None,
            caller_frame: Some(edge.caller.frame_id),
            callee_frame: Some(edge.callee.frame_id),
        });
    }
    for edge in static_calls {
        if relations.iter().any(|relation| {
            same_symbol(&relation.caller, &edge.caller)
                && same_symbol(&relation.callee, &edge.callee)
        }) {
            continue;
        }
        relations.push(CallRelation {
            caller: edge.caller.clone(),
            callee: edge.callee.clone(),
            evidence: EvidenceKind::Mir,
            source: None,
            mir_line: Some(edge.mir_line),
            caller_frame: None,
            callee_frame: None,
        });
    }
    relations
}

/// Whether two logical symbols name the same function.
/// 两个逻辑符号是否指向同一个函数。
///
/// A live trace names a function by its bare symbol (`button`) while a MIR
/// candidate carries the fully qualified path (`crate::ui::button`), so the
/// match must also accept one side being a path suffix of the other. The suffix
/// has to start right after a `::` boundary: a bare `ends_with` would treat
/// `button` as a suffix of `fastbutton` and silently discard a real candidate.
/// `strip_suffix` plus the boundary check preserves the original behaviour
/// without allocating the two `format!("::{right}")` strings on every
/// comparison — this runs in the nested merge loop, so those allocations were
/// paid per relation per candidate.
/// live trace 用裸符号（`button`）命名函数，而 MIR 候选取的是全限定路径
/// （`crate::ui::button`），因此匹配还须接受一方是另一方路径后缀。后缀必须紧接 `::`
/// 边界：裸的 `ends_with` 会把 `button` 当成 `fastbutton` 的后缀，静默丢掉一个真实候选。
/// `strip_suffix` 加边界检查在保持原行为的同时，不再每次比较都分配两个
/// `format!("::{right}")` 字符串——这段代码跑在归并的嵌套循环里，这些分配是按“每条关系
/// × 每个候选”付出的。
/// Pinned by `same_symbol_requires_a_path_boundary_before_the_suffix`.
/// 由 `same_symbol_requires_a_path_boundary_before_the_suffix` 钉住。
pub fn same_symbol(left: &str, right: &str) -> bool {
    left == right
        || left
            .strip_suffix(right)
            .is_some_and(|prefix| prefix.ends_with("::"))
        || right
            .strip_suffix(left)
            .is_some_and(|prefix| prefix.ends_with("::"))
}

#[cfg(test)]
mod tests {
    use super::super::model::{CallEvidence, MirCall};
    use super::{merge_call_relations, same_symbol};
    use crate::registry_core::declaration::{CallEdge, CallSite, EvidenceKind, SourceLocation};
    use crate::registry_core::identity::NodeId;

    fn live_edge(caller: &'static str, callee: &'static str) -> CallEdge {
        fn site(function: &'static str, line: usize) -> CallSite {
            CallSite {
                node: NodeId::from_path("a.rs", "A"),
                function,
                frame_id: 1,
                source: Some(SourceLocation {
                    file: "a.rs",
                    line: line.try_into().unwrap(),
                    column: 1,
                    function,
                }),
            }
        }
        CallEdge {
            caller: site(caller, 1),
            callee: site(callee, 2),
        }
    }

    #[test]
    fn live_edges_win_over_same_symbol_mir_candidates() {
        let static_calls = vec![MirCall {
            caller: "crate::a".to_owned(),
            callee: "crate::b".to_owned(),
            mir_line: 1,
        }];
        let relations = merge_call_relations(&static_calls, &[live_edge("a", "b")]);
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].evidence, CallEvidence::Live);
    }

    #[test]
    fn unobserved_mir_candidate_stays_visible_as_mir() {
        let static_calls = vec![MirCall {
            caller: "crate::a".to_owned(),
            callee: "crate::c".to_owned(),
            mir_line: 7,
        }];
        let relations = merge_call_relations(&static_calls, &[]);
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].evidence, EvidenceKind::Mir);
        assert_eq!(relations[0].mir_line, Some(7));
        assert!(!relations[0].evidence.confirmed());
    }

    /// A suffix match is only a match when the suffix starts at a `::`
    /// boundary: `button` must not absorb `fastbutton`.
    /// 后缀只有在 `::` 边界处开始才算匹配：`button` 不能吞掉 `fastbutton`。
    #[test]
    fn same_symbol_requires_a_path_boundary_before_the_suffix() {
        assert!(same_symbol("crate::ui::button", "button"));
        assert!(same_symbol("button", "crate::ui::button"));
        assert!(same_symbol("crate::ui::button", "crate::ui::button"));
        assert!(!same_symbol("crate::fastbutton", "button"));
        assert!(!same_symbol("fastbutton", "button"));
        assert!(!same_symbol("crate::ui::button", "crate::other::button"));
        // An empty side is a degenerate suffix: it only matches through `::`
        // the same way the old `ends_with` form did.
        // 空的一侧是退化后缀：与旧的 `ends_with` 写法一样，只有经 `::` 才匹配。
        assert!(!same_symbol("crate::ui", ""));
        assert!(!same_symbol("", "crate::ui"));
        assert!(same_symbol("crate::ui::", ""));
    }
}
