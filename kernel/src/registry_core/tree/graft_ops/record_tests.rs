//! Behaviour tests for recorded-graft reconciliation and application.
//! 记录嫁接对账与应用的行为测试。
//!
//! Mounted from `record.rs` as a test-only sibling (see the split note there):
//! the tests pin the public behaviour of `resolve_record`/`overlay_recorded`
//! without enlarging the production page.
//! 由 `record.rs` 作为仅测试的同级模块挂载（见那里的拆分说明）：测试钉住
//! `resolve_record`/`overlay_recorded` 的公开行为，而不撑大生产页面。

use super::*;

use crate::Registry;
use crate::registry_core::identity::NodeId;
use crate::registry_core::plugin::graft::document::GraftPlanDocument;
use crate::registry_core::release::StaticGraftCut;

use super::super::fixtures::{FRAMEWORK, face};

fn namespace() -> &'static str {
    "record-test"
}

fn node(source: &str, kind: &str) -> NodeId {
    NodeId::from_namespaced_path(namespace(), source, kind)
}

/// A base tree with one replaceable face at `root/a` plus an external tree
/// carrying `slow` and `fast` implementations of the same slot.
/// 一棵在 `root/a` 有一个可替换面的原树，外加一棵带同槽位 `slow` 与 `fast`
/// 实现的外部树。
fn base_with_external() -> (Registry, Registry) {
    let mut base = Registry::root_for_namespace(FRAMEWORK, namespace());
    let mut target = face(namespace(), "a.rs", "A", "a");
    target.id = node("a.rs", "A");
    base.register_snapshot_batch([target]).unwrap();

    let mut external = Registry::root_for_namespace(FRAMEWORK, "record-external");
    let mut slow = face("record-external", "slow.rs", "Slow", "slow");
    slow.id = NodeId::from_namespaced_path("record-external", "slow.rs", "Slow");
    let mut fast = face("record-external", "fast.rs", "Fast", "fast");
    fast.id = NodeId::from_namespaced_path("record-external", "fast.rs", "Fast");
    external.register_snapshot_batch([slow, fast]).unwrap();
    (base, external)
}

fn record(target: NodeId, target_path: &str, graft: &str, full: bool) -> RecordedGraft {
    RecordedGraft::new(
        graft,
        GraftPlanDocument::new(target, target_path, graft, full),
    )
}

fn slot_of(resolved: ResolvedRecord) -> NodeId {
    resolved.into_slot().expect("a resolved slot").0
}

#[test]
fn identity_and_path_agree_resolves_to_the_target() {
    let (base, _) = base_with_external();
    let target = node("a.rs", "A");
    let resolved = base
        .resolve_record(&record(target, "root/a", "fast", false))
        .expect("agreeing selectors resolve");
    assert_eq!(slot_of(resolved), target);
}

#[test]
fn a_missing_identity_resolves_by_path_with_drift() {
    let (base, _) = base_with_external();
    let resolved = base
        .resolve_record(&record(node("gone.rs", "Gone"), "root/a", "fast", false))
        .expect("a resolvable path is not fatal");
    assert!(matches!(resolved, ResolvedRecord::Slot { slot, .. } if slot == node("a.rs", "A")));
    assert!(matches!(
        resolved.reports(),
        [RecordReport::IdentityDrifted { .. }]
    ));
}

#[test]
fn contradictory_identity_and_path_are_refused() {
    let mut base = Registry::root_for_namespace(FRAMEWORK, namespace());
    let mut a = face(namespace(), "a.rs", "A", "a");
    a.id = node("a.rs", "A");
    let mut b = face(namespace(), "b.rs", "B", "b");
    b.id = node("b.rs", "B");
    base.register_snapshot_batch([a, b]).unwrap();

    let error = base
        .resolve_record(&record(node("a.rs", "A"), "root/b", "fast", false))
        .expect_err("contradictory selectors are refused");
    let rendered = format!("{error}");
    assert!(rendered.contains("root/a"), "{rendered}");
    assert!(rendered.contains("root/b"), "{rendered}");
}

/// A directory selector that disagrees with the `graft` its plan names leaves two
/// candidate implementations and no way to tell which one the author meant, so
/// the record is refused instead of silently using the file's.
/// 目录选择器与计划里的 `graft` 不一致时会留下两个候选实现，且无法判断作者指的是哪一
/// 个，因此直接拒绝该记录，而不是静默采用文件里的那个。
#[test]
fn a_directory_that_disagrees_with_its_plan_is_refused() {
    let (base, _) = base_with_external();
    let target = node("a.rs", "A");
    let renamed = RecordedGraft::new(
        "renamed_directory",
        GraftPlanDocument::new(target, "root/a", "fast", false),
    );

    let error = base
        .resolve_record(&renamed)
        .expect_err("a disagreeing directory is refused");
    let rendered = format!("{error}");
    assert!(rendered.contains("renamed_directory"), "{rendered}");
    assert!(rendered.contains("graft=fast"), "{rendered}");
    assert!(rendered.contains("rename"), "{rendered}");
}

#[test]
fn a_record_with_no_live_slot_is_skipped_not_fatal() {
    let (base, _) = base_with_external();
    let resolved = base
        .resolve_record(&record(node("gone.rs", "Gone"), "root/gone", "fast", false))
        .expect("a missing slot is not fatal");
    assert_eq!(
        resolved,
        ResolvedRecord::Unkept {
            reports: vec![RecordReport::UnkeptSlot {
                selector: "fast".to_owned(),
                target_path: "root/gone".to_owned(),
            }],
        }
    );
}

#[test]
fn a_string_declaration_yields_to_the_record() {
    let (base, external) = base_with_external();
    let target = node("a.rs", "A");
    let outcome = base
        .overlay_recorded(
            &[record(target, "root/a", "fast", false)],
            &[StaticGraftCut::new("root/a", "slow", false)],
            &external,
        )
        .expect("the record overlay publishes");
    assert_eq!(
        outcome
            .effective
            .find(target)
            .map(|info| info.kind.as_str()),
        Some("Fast")
    );
    assert!(outcome.reports.iter().any(|report| matches!(
        report,
        RecordReport::DeclarationOverridden { recorded, .. } if recorded == "fast"
    )));
}

#[test]
fn a_typed_declaration_stays_final() {
    let (base, external) = base_with_external();
    let target = node("a.rs", "A");
    let slow = NodeId::from_namespaced_path("record-external", "slow.rs", "Slow");
    let outcome = base
        .overlay_recorded(
            &[record(target, "root/a", "fast", false)],
            &[StaticGraftCut::from_ids(target, slow, false)],
            &external,
        )
        .expect("the typed declaration publishes");
    assert_eq!(
        outcome
            .effective
            .find(target)
            .map(|info| info.kind.as_str()),
        Some("Slow")
    );
    assert!(outcome.reports.iter().any(|report| matches!(
        report,
        RecordReport::TypedDeclarationKept { recorded, .. } if recorded == "fast"
    )));
}

#[test]
fn an_unresolved_record_selector_falls_back_to_the_declaration() {
    let (base, external) = base_with_external();
    let target = node("a.rs", "A");
    let outcome = base
        .overlay_recorded(
            &[record(target, "root/a", "absent", false)],
            &[StaticGraftCut::new("root/a", "slow", false)],
            &external,
        )
        .expect("the declaration still publishes");
    assert_eq!(
        outcome
            .effective
            .find(target)
            .map(|info| info.kind.as_str()),
        Some("Slow")
    );
    assert!(outcome.reports.iter().any(|report| matches!(
        report,
        RecordReport::RecordSelectorUnresolved { graft, .. } if graft == "absent"
    )));
}

#[test]
fn full_discards_base_children_and_non_full_keeps_them() {
    let mut base = Registry::root_for_namespace(FRAMEWORK, namespace());
    let mut target = face(namespace(), "a.rs", "A", "a");
    target.needs_registry = true;
    target.id = node("a.rs", "A");
    let mut child = face(namespace(), "old.rs", "Old", "old");
    child.parent = target.id;
    base.register_snapshot_batch([target.clone(), child])
        .unwrap();

    let external_namespace = "record-external-full";
    let mut external = Registry::root_for_namespace(FRAMEWORK, external_namespace);
    let mut replacement = face(external_namespace, "fast.rs", "Fast", "fast");
    replacement.needs_registry = true;
    replacement.id = NodeId::from_namespaced_path(external_namespace, "fast.rs", "Fast");
    let mut new_child = face(external_namespace, "new.rs", "New", "new");
    new_child.parent = replacement.id;
    external
        .register_snapshot_batch([replacement, new_child])
        .unwrap();

    let full = base
        .overlay_recorded(
            &[record(target.id, "root/a", "fast", true)],
            &[StaticGraftCut::new("root/a", "fast", false)],
            &external,
        )
        .expect("a full record publishes");
    assert_eq!(full.effective.find_kind("Old").len(), 0);
    assert_eq!(full.effective.find_kind("New").len(), 1);

    let kept = base
        .overlay_recorded(
            &[record(target.id, "root/a", "fast", false)],
            &[StaticGraftCut::new("root/a", "fast", false)],
            &external,
        )
        .expect("a plain record publishes");
    assert_eq!(kept.effective.find_kind("Old").len(), 1);
    assert_eq!(kept.effective.find_kind("New").len(), 0);
}

#[test]
fn granularity_is_overridden_as_one_atomic_record() {
    let mut base = Registry::root_for_namespace(FRAMEWORK, namespace());
    let mut target = face(namespace(), "a.rs", "A", "a");
    target.needs_registry = true;
    target.id = node("a.rs", "A");
    let mut child = face(namespace(), "old.rs", "Old", "old");
    child.parent = target.id;
    base.register_snapshot_batch([target.clone(), child])
        .unwrap();

    let external_namespace = "record-external-atomic";
    let mut external = Registry::root_for_namespace(FRAMEWORK, external_namespace);
    let mut replacement = face(external_namespace, "fast.rs", "Fast", "fast");
    replacement.needs_registry = true;
    replacement.id = NodeId::from_namespaced_path(external_namespace, "fast.rs", "Fast");
    let mut new_child = face(external_namespace, "new.rs", "New", "new");
    new_child.parent = replacement.id;
    external
        .register_snapshot_batch([replacement, new_child])
        .unwrap();

    // The declaration asks for a plain cut; the record asks for a full one.
    // The record wins *both* halves: its implementation and its granularity,
    // never the declaration's `full` with the record's `graft`.
    // 声明要求普通切口；记录要求整棵子树。记录同时赢下两半：它的实现与它的粒度，
    // 绝不会出现“记录的 graft 配声明的 full”。
    let outcome = base
        .overlay_recorded(
            &[record(target.id, "root/a", "fast", true)],
            &[StaticGraftCut::new("root/a", "fast", false)],
            &external,
        )
        .expect("the record publishes");
    assert_eq!(outcome.effective.find_kind("Old").len(), 0);
    assert_eq!(outcome.effective.find_kind("New").len(), 1);
    assert!(outcome.reports.iter().any(|report| matches!(
        report,
        RecordReport::GranularityOverridden {
            declared_full: false,
            recorded_full: true,
            ..
        }
    )));
}

#[test]
fn a_stale_target_path_uses_the_current_path_for() {
    let (base, external) = base_with_external();
    let target = node("a.rs", "A");
    let outcome = base
        .overlay_recorded(
            &[record(target, "root/renamed", "fast", false)],
            &[StaticGraftCut::new("root/a", "slow", false)],
            &external,
        )
        .expect("the identity still resolves the slot");
    assert_eq!(
        outcome.effective.path_for(target).as_deref(),
        Some("root/a")
    );
    assert_eq!(
        outcome
            .effective
            .find(target)
            .map(|info| info.kind.as_str()),
        Some("Fast")
    );
    assert!(outcome.reports.iter().any(|report| matches!(
        report,
        RecordReport::IdentityDrifted { target_path, .. } if target_path == "root/renamed"
    )));
}

#[test]
fn a_plan_for_an_undeclared_slot_is_skipped_with_a_report() {
    let (base, external) = base_with_external();
    let target = node("a.rs", "A");
    let outcome = base
        .overlay_recorded(&[record(target, "root/a", "fast", false)], &[], &external)
        .expect("an undeclared slot is skipped, not fatal");
    assert!(outcome.reports.iter().any(|report| matches!(
        report,
        RecordReport::UnkeptSlot { target_path, .. } if target_path == "root/a"
    )));
}

/// With no records the recorded path must be byte-for-byte the static one:
/// untouched declarations are passed through as their typed cuts, so a host
/// that adopts the loader does not change what `overlay_static` produced.
/// 没有记录时，记录路径必须与静态路径逐字节相同：未受影响的声明按类型化切口原样
/// 传入，因此采用加载器的宿主不会改变 `overlay_static` 的产物。
#[test]
fn no_records_reproduces_overlay_static() {
    let (base, external) = base_with_external();
    let declared = [StaticGraftCut::new("root/a", "slow", false)];
    let recorded = base
        .overlay_recorded(&[], &declared, &external)
        .expect("the declaration publishes");
    let statically = base
        .overlay_static(&declared, &external)
        .expect("the declaration publishes");
    assert_eq!(
        recorded
            .effective
            .find(node("a.rs", "A"))
            .map(|info| info.kind.as_str()),
        statically
            .find(node("a.rs", "A"))
            .map(|info| info.kind.as_str()),
    );
    assert!(recorded.reports.is_empty());
}
