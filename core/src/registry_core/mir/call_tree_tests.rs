//! Layered call-tree rules, pinned as data.
//! 分层调用树的规则，以数据形式钉住。
//!
//! Every case here is a shape the Studio drawing depends on: column order along
//! the call direction, one lane per node per column, shared callees drawn as one
//! node, cycles that terminate, and the two ways a view can be incomplete.
//! 这里每个用例都是 Studio 绘制所依赖的形状：沿调用方向的列顺序、同一列内每个节点独占一条
//! 车道、共用被调用者只画一个节点、会终止的环，以及视图可能不完整的两种方式。

use super::super::model::CallRelation;
use super::{CallTree, call_tree};
use crate::registry_core::declaration::EvidenceKind;

fn link(caller: &str, callee: &str) -> CallRelation {
    CallRelation::from_symbols(caller, callee, EvidenceKind::Source)
}

/// The fixture the prototype used, as relations: one entry point, one shared
/// implementation called by two consumers, one consumer nothing calls.
/// 原型所用的夹具，以关系表示：一个入口、一个被两个消费者调用的共用实现、一个没人调用的
/// 消费者。
fn fixture() -> Vec<CallRelation> {
    vec![
        link("update", "layout"),
        link("layout", "paint"),
        link("paint", "place"),
        link("paint", "drag"),
        link("paint", "scroll"),
        link("place", "to_screen"),
        link("drag", "to_screen"),
        link("scroll", "to_screen"),
        link("gauge", "to_screen"),
    ]
}

fn levels(tree: &CallTree) -> Vec<(String, i32, usize)> {
    tree.nodes
        .iter()
        .map(|node| (node.symbol.clone(), node.level, node.lane))
        .collect()
}

#[test]
fn the_focus_is_first_and_owns_the_zero_level() {
    let tree = call_tree("paint", &fixture(), 2, 16);
    assert_eq!(tree.nodes[0].symbol, "paint");
    assert_eq!(tree.nodes[0].level, 0);
    assert_eq!(
        tree.nodes[0].parent, None,
        "the focus is placed by nobody, so the drawing has a root"
    );
    assert_eq!(tree.index_of("paint"), Some(0));
    assert!(
        tree.nodes[0].lane > 0,
        "the focus sits on the mean of its neighbours rather than on row zero"
    );
}

#[test]
fn callers_land_upstream_and_callees_downstream() {
    let tree = call_tree("paint", &fixture(), 2, 16);
    let level = |symbol: &str| tree.nodes[tree.index_of(symbol).unwrap()].level;
    assert_eq!(level("layout"), -1, "a caller is one hop upstream");
    assert_eq!(level("update"), -2, "its caller is two hops upstream");
    assert_eq!(level("place"), 1, "a callee is one hop downstream");
    assert_eq!(level("to_screen"), 2, "its callee is two hops downstream");
    assert_eq!(tree.level_span(), (-2, 2));
}

#[test]
fn only_relations_that_run_along_the_axis_are_forward() {
    let tree = call_tree("paint", &fixture(), 2, 16);
    assert!(tree.backward_edges() == 0, "a tree has no back edges yet");
    assert!(tree.edges.iter().all(|edge| edge.forward));
    let callee_callers = tree
        .edges
        .iter()
        .filter(|edge| edge.callee == tree.index_of("to_screen").unwrap())
        .count();
    assert_eq!(
        callee_callers, 3,
        "place, drag and scroll all reach the shared implementation"
    );
}

#[test]
fn a_shared_callee_is_one_node_with_two_parents_on_different_lanes() {
    let tree = call_tree("paint", &fixture(), 2, 16);
    let shared = tree.index_of("to_screen").unwrap();
    let parents = tree
        .edges
        .iter()
        .filter(|edge| edge.callee == shared)
        .map(|edge| edge.caller)
        .collect::<Vec<_>>();
    assert_eq!(parents.len(), 3);
    let lanes = parents
        .iter()
        .map(|parent| tree.nodes[*parent].lane)
        .collect::<Vec<_>>();
    let mut unique = lanes.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        lanes.len(),
        "two parents drawn on one row would hide the diamond: {lanes:?}"
    );
}

#[test]
fn no_two_nodes_share_a_lane_in_one_column() {
    for (focus, depth) in [("paint", 2), ("update", 3), ("to_screen", 2)] {
        let tree = call_tree(focus, &fixture(), depth, 16);
        let mut seen: Vec<(i32, usize)> = Vec::new();
        for node in &tree.nodes {
            let column = (node.level, node.lane);
            assert!(
                !seen.contains(&column),
                "{focus} at depth {depth}: column {column:?} holds two nodes"
            );
            seen.push(column);
        }
        assert_eq!(
            tree.rows(),
            seen.iter().map(|(_, lane)| lane + 1).max().unwrap()
        );
    }
}

#[test]
fn a_cycle_terminates_and_reports_its_crossings() {
    let mut relations = fixture();
    relations.push(link("to_screen", "paint"));
    relations.push(link("paint", "paint"));
    let tree = call_tree("paint", &relations, 3, 16);
    let to_screen = tree.index_of("to_screen").unwrap();
    let paint = tree.index_of("paint").unwrap();
    assert_eq!(
        tree.nodes[to_screen].level, -1,
        "a symbol that calls the focus is placed upstream once, even though the focus also calls it"
    );
    assert_eq!(
        tree.edges.iter().filter(|edge| !edge.forward).count(),
        4,
        "one back edge for the self call plus one crossing edge per consumer"
    );
    assert!(
        tree.edges
            .iter()
            .any(|edge| edge.caller == to_screen && edge.callee == paint && edge.forward),
        "the focus still reaches the mutual callee along the axis"
    );
    assert!(
        tree.edges
            .iter()
            .any(|edge| edge.caller == paint && edge.callee == paint && !edge.forward),
        "the self call is reported rather than dropped"
    );
    for consumer in ["place", "drag", "scroll"] {
        let index = tree.index_of(consumer).unwrap();
        assert!(
            tree.edges
                .iter()
                .any(|edge| edge.caller == index && edge.callee == to_screen && !edge.forward),
            "{consumer} now crosses the axis, and the drawing is told so"
        );
    }
}

#[test]
fn a_downstream_node_does_not_drag_its_other_callers_into_the_tree() {
    let mut relations = fixture();
    relations.push(link("other", "place"));
    let tree = call_tree("paint", &relations, 3, 16);
    let place = tree.index_of("place").unwrap();
    assert!(tree.nodes[place].level > 0, "place sits downstream");
    assert_eq!(
        tree.index_of("other"),
        None,
        "a downstream node never walks back out to its other callers"
    );
    assert_eq!(
        tree.nodes[place].cut_callers, 1,
        "the caller it did not walk is counted instead of dropped"
    );
}

#[test]
fn only_the_focus_occupies_the_zero_level() {
    for focus in ["paint", "to_screen", "update"] {
        let tree = call_tree(focus, &fixture(), 2, 16);
        let zero = tree
            .nodes
            .iter()
            .filter(|node| node.level == 0)
            .map(|node| node.symbol.as_str())
            .collect::<Vec<_>>();
        assert_eq!(zero, vec![focus], "focus {focus} owns column zero alone");
    }
}

#[test]
fn the_hop_budget_cuts_the_far_side_and_records_what_it_cut() {
    let tree = call_tree("paint", &fixture(), 1, 16);
    assert_eq!(tree.level_span(), (-1, 1));
    assert_eq!(tree.index_of("update"), None, "two hops up is out of reach");
    assert_eq!(tree.index_of("to_screen"), None, "two hops down as well");
    let paint = tree.index_of("paint").unwrap();
    assert_eq!(
        tree.nodes[paint].cut_callees, 0,
        "the callees themselves are placed; only their callees are not"
    );
    let place = tree.index_of("place").unwrap();
    assert_eq!(
        tree.nodes[place].cut_callees, 1,
        "place calls to_screen, which this budget does not reach"
    );
    let layout = tree.index_of("layout").unwrap();
    assert_eq!(tree.nodes[layout].cut_callers, 1, "update is out of reach");
    assert!(!tree.truncated, "a hop cut is not a node-budget cut");
}

#[test]
fn the_node_budget_truncates_and_says_so() {
    let tree = call_tree("paint", &fixture(), 4, 3);
    assert_eq!(tree.nodes.len(), 3);
    assert!(tree.truncated);
    assert_eq!(tree.nodes[0].symbol, "paint");
    let cut = tree
        .nodes
        .iter()
        .map(|node| node.cut_callees)
        .sum::<usize>()
        + tree
            .nodes
            .iter()
            .map(|node| node.cut_callers)
            .sum::<usize>();
    assert!(cut > 0, "the drawing has to know what it left out");
}

#[test]
fn a_full_budget_that_lost_nothing_is_not_truncated() {
    let tree = call_tree("update", &fixture(), 4, 16);
    assert!(tree.index_of("to_screen").is_some(), "four hops reach it");
    assert!(
        !tree.truncated,
        "nothing was dropped, so nothing is claimed"
    );
}

#[test]
fn an_unreached_consumer_is_reported_as_a_cut_caller() {
    let tree = call_tree("to_screen", &fixture(), 2, 2);
    let shared = tree.index_of("to_screen").unwrap();
    assert_eq!(
        tree.nodes[shared].cut_callers, 3,
        "a two-node budget keeps one caller and reports the other three"
    );
    assert_eq!(tree.index_of("place"), Some(1));
    assert!(tree.truncated);
}

#[test]
fn the_strongest_evidence_wins_for_one_pair() {
    let relations = vec![
        CallRelation::from_symbols("a", "b", EvidenceKind::Source),
        CallRelation::from_symbols("a", "b", EvidenceKind::Live),
        CallRelation::from_symbols("a", "b", EvidenceKind::Mir),
    ];
    let tree = call_tree("a", &relations, 1, 8);
    assert_eq!(tree.edges.len(), 1, "one pair draws one edge");
    assert_eq!(tree.edges[0].evidence, EvidenceKind::Live);
}

#[test]
fn a_bare_name_matches_a_qualified_symbol() {
    let relations = vec![CallRelation::from_symbols(
        "crate::ui::paint",
        "crate::ui::place",
        EvidenceKind::Mir,
    )];
    let tree = call_tree("paint", &relations, 1, 8);
    assert_eq!(tree.nodes.len(), 2);
    assert_eq!(tree.nodes[1].symbol, "crate::ui::place");
    assert_eq!(tree.index_of("place"), Some(1));
}

#[test]
fn the_same_input_always_draws_the_same_tree() {
    let first = call_tree("paint", &fixture(), 3, 16);
    let second = call_tree("paint", &fixture(), 3, 16);
    assert_eq!(levels(&first), levels(&second));
    assert_eq!(first.edges, second.edges);
}

#[test]
fn an_empty_relation_list_leaves_the_focus_alone() {
    let tree = call_tree("orphan", &[], 2, 8);
    assert_eq!(tree.nodes.len(), 1);
    assert!(tree.edges.is_empty());
    assert_eq!(tree.rows(), 1);
    assert_eq!(tree.level_span(), (0, 0));
    assert!(!tree.truncated);
}
