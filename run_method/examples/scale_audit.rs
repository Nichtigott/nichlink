//! Repeatable registration scale audit for release qualification.
//!
//! Two things are checked, not one: that the registry still answers correctly at
//! scale (the page counts and the incremental transaction below), and that it does
//! so inside a per-node time budget. Without the budget a tenfold regression would
//! only make the printed numbers larger, which nobody reads in a CI log.
//!
//! The ceilings are order-of-magnitude guards, not a grade for the machine: they
//! carry roughly eight times the headroom measured when the baseline was recorded
//! (`docs/performance-baseline.md`), and both can be raised through the
//! environment on a slow runner without editing this file.
//! 这里检查两件事而不是一件：注册机在规模上仍然给出正确答案（下面的页数与增量事务），以及
//! 它在那份每节点时间预算之内。没有预算时，十倍的性能退化只会让打印出来的数字更大，而 CI
//! 日志里没人会去读那些数字。
//! 上限是数量级的守卫，而不是给机器打分：它们带着记录基线时实测值约八倍的余量
//! （`docs/performance-baseline.md`），两者都可以在慢机器上经环境变量抬高，无需改本文件。

use std::time::Instant;

use nichlink_run_method::{
    FrameworkId, NodeId, OwnedAdmission, OwnedFlowContract, OwnedLocalizedText,
    OwnedObjectContract, OwnedRegistrationRule, OwnedSourceLocation, RegistrationSnapshot,
    Registry, root_node_id,
};

/// One per-node ceiling in microseconds, or `default` when the environment
/// overrides it.
/// 每节点上限（微秒）；环境变量覆盖时用覆盖值。
fn ceiling(name: &str, default: u128) -> u128 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn snapshot(namespace: &str, index: usize, parent: NodeId) -> RegistrationSnapshot {
    let kind = format!("Face{index}");
    RegistrationSnapshot {
        namespace: namespace.to_owned(),
        id: NodeId::from_namespaced_path(namespace, &format!("scale/{index}.rs"), &kind),
        parent,
        kind: kind.clone(),
        preset: "default".to_owned(),
        parts: String::new(),
        params: String::new(),
        handle: kind.clone(),
        stable_name: None,
        name: OwnedLocalizedText {
            zh: kind.clone(),
            en: kind.clone(),
        },
        summary: OwnedLocalizedText {
            zh: String::new(),
            en: String::new(),
        },
        exports: Vec::new(),
        needs_registry: false,
        registry_name: format!("face-{index}"),
        getting_from_other_registry: None,
        registry_rule_path: "<scale-audit>".to_owned(),
        registry_rule: OwnedRegistrationRule {
            required_preset: None,
            required_parts: Vec::new(),
            required_exports: Vec::new(),
            required_handle_traits: Vec::new(),
            required_part_traits: Vec::new(),
        },
        admission: OwnedAdmission {
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
        },
        requires: Vec::new(),
        provides: Vec::new(),
        contract: OwnedObjectContract {
            required_parts: Vec::new(),
            provided_parts: Vec::new(),
        },
        flow: OwnedFlowContract::none(),
        flow_provider: None,
        handle_traits: Vec::new(),
        part_traits: Vec::new(),
        runtime_checks: Vec::new(),
        plugin: None,
        source: OwnedSourceLocation {
            file: format!("scale/{index}.rs"),
            line: 1,
            column: 1,
            function: kind,
        },
    }
}

fn main() {
    let sizes = std::env::args()
        .skip(1)
        .map(|value| value.parse::<usize>().expect("size must be an integer"))
        .collect::<Vec<_>>();
    let sizes = if sizes.is_empty() {
        vec![10_000, 100_000]
    } else {
        sizes
    };
    println!(
        "nodes\tregister_ms\tregister_budget_ms\tindex_ms\tindex_budget_ms\tentries\tpages\tstatic_face_bytes"
    );
    // 40 µs and 20 µs per node: roughly eight times the 5.2 µs and 2.6 µs measured
    // for 100 000 nodes when this budget was added.
    // 每节点 40 µs 与 20 µs：约等于加入本预算时 100 000 个节点实测 5.2 µs 与 2.6 µs 的八倍。
    let register_ceiling = ceiling("NICHLINK_SCALE_REGISTER_US", 40);
    let index_ceiling = ceiling("NICHLINK_SCALE_INDEX_US", 20);
    for size in sizes {
        let namespace = format!("scale-{size}");
        let root = Registry::root_for_namespace(FrameworkId::new("nichlink.scale"), &namespace);
        let parent = root_node_id(&namespace);
        let submissions = (0..size)
            .map(|index| snapshot(&namespace, index, parent))
            .collect::<Vec<_>>();
        let mut registry = root;
        let register_start = Instant::now();
        registry
            .register_snapshot_batch(submissions)
            .expect("generated scale batch must register");
        let register_ms = register_start.elapsed().as_millis();
        let index_start = Instant::now();
        let index = registry.index();
        let index_ms = index_start.elapsed().as_millis();
        let stats = registry.storage_stats();
        let register_budget_ms = register_ceiling * size as u128 / 1000;
        let index_budget_ms = index_ceiling * size as u128 / 1000;
        println!(
            "{size}\t{register_ms}\t{register_budget_ms}\t{index_ms}\t{index_budget_ms}\t{}\t{}\t{}",
            index.len(),
            stats.pages,
            size * std::mem::size_of::<nichlink::StaticFace>()
        );
        assert!(
            register_ms <= register_budget_ms,
            "registering {size} nodes took {register_ms} ms, over the {register_budget_ms} ms              budget ({register_ceiling} µs per node); raise NICHLINK_SCALE_REGISTER_US if this              machine is simply slower, and update docs/performance-baseline.md if the baseline moved"
        );
        assert!(
            index_ms <= index_budget_ms,
            "indexing {size} nodes took {index_ms} ms, over the {index_budget_ms} ms budget              ({index_ceiling} µs per node); raise NICHLINK_SCALE_INDEX_US if this machine is              simply slower, and update docs/performance-baseline.md if the baseline moved"
        );
        let extra = snapshot(&namespace, size, parent);
        registry
            .register_snapshot_batch([extra])
            .expect("incremental transaction must register");
        assert_eq!(registry.index().len(), size + 2);
    }
}
