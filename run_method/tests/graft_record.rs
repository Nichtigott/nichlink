//! Integration tests: a `.nichlink` graft record must reach `Registry::overlay`.
//! 集成测试：`.nichlink` 里的 graft 记录必须抵达 `Registry::overlay`。
//!
//! `a_record_on_disk_reaches_overlay` is the test that would have caught the gap
//! this feature closes: before it, a `graft.plan` Studio wrote changed the screen
//! and nothing else, because no production code ever opened the file.
//! `a_record_on_disk_reaches_overlay` 正是能抓住本特性所填缺口的那条测试：在此之前
//! Studio 写下的 `graft.plan` 只改变屏幕、不改变别的，因为没有任何生产代码打开过该
//! 文件。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use nichlink_run_method::registry_core::lexicon;
use nichlink_run_method::registry_core::{
    Admission, FrameworkId, NodeId, OwnedFlowContract, OwnedLocalizedText, OwnedObjectContract,
    OwnedSourceLocation, RegistrationRule, RegistrationSnapshot, Registry, StaticGraftCut,
    root_node_id,
};
use nichlink_run_method::{
    GraftPlanDocument, RecordReport, apply_recorded_grafts, graft_record_root,
};
// The ungated tests never call the loader directly; only the no-`authoring` pin
// does, so the names stay out of the `authoring` build.
// 不受门控的测试不直接调用加载器；只有那条“无 authoring”钉子会调用，因此这些名字
// 不出现在 `authoring` 构建里。
#[cfg(not(feature = "authoring"))]
use nichlink_run_method::{LoadedGraft, load_graft_records};

const FRAMEWORK: FrameworkId = FrameworkId::new("graft-record-test");
const BASE_NAMESPACE: &str = "graft-record-base";
const EXTERNAL_NAMESPACE: &str = "graft-record-external";

/// A throwaway package root removed when the test ends.
/// 测试结束时删除的一次性包根。
struct TempRoot(PathBuf);

impl TempRoot {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-graft-record-{tag}-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create temp root");
        Self(root)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// One minimal registration face with a declared, compatible flow.
/// 一个带已声明且兼容数据流合同的最小注册面。
fn face(namespace: &str, source: &str, kind: &str, slot: &str) -> RegistrationSnapshot {
    RegistrationSnapshot {
        namespace: namespace.to_owned(),
        id: NodeId::from_namespaced_path(namespace, source, kind),
        parent: root_node_id(namespace),
        kind: kind.to_owned(),
        preset: "NoPreset".to_owned(),
        parts: "NoParts".to_owned(),
        params: kind.to_owned(),
        handle: kind.to_owned(),
        stable_name: None,
        name: OwnedLocalizedText {
            zh: kind.to_owned(),
            en: kind.to_owned(),
        },
        summary: OwnedLocalizedText {
            zh: String::new(),
            en: String::new(),
        },
        exports: Vec::new(),
        needs_registry: false,
        registry_name: slot.to_owned(),
        getting_from_other_registry: None,
        registry_rule_path: "<test>".to_owned(),
        registry_rule: RegistrationRule::ANY.into_owned(),
        admission: Admission::ANY.into_owned(),
        requires: Vec::new(),
        provides: Vec::new(),
        contract: OwnedObjectContract {
            required_parts: Vec::new(),
            provided_parts: Vec::new(),
        },
        flow: OwnedFlowContract {
            id: "render.v1".to_owned(),
            version: 1,
            input: "LocalCoordinates".to_owned(),
            output: "CanvasFrame".to_owned(),
        },
        flow_provider: None,
        handle_traits: Vec::new(),
        part_traits: Vec::new(),
        runtime_checks: Vec::new(),
        plugin: None,
        source: OwnedSourceLocation {
            file: source.to_owned(),
            line: 1,
            column: 1,
            function: kind.to_owned(),
        },
    }
}

/// The base tree: one replaceable face at `root/a`.
/// 原树：`root/a` 处一个可替换的注册面。
fn base_registry() -> Registry {
    let mut registry = Registry::root_for_namespace(FRAMEWORK, BASE_NAMESPACE);
    registry
        .register_snapshot_batch([face(BASE_NAMESPACE, "a.rs", "A", "a")])
        .expect("the base face registers");
    registry
}

/// The external tree: `slow` and `fast` implementations of the same slot.
/// 外部树：同槽位的 `slow` 与 `fast` 实现。
fn external_registry() -> Registry {
    let mut registry = Registry::root_for_namespace(FRAMEWORK, EXTERNAL_NAMESPACE);
    registry
        .register_snapshot_batch([
            face(EXTERNAL_NAMESPACE, "slow.rs", "Slow", "slow"),
            face(EXTERNAL_NAMESPACE, "fast.rs", "Fast", "fast"),
        ])
        .expect("the external faces register");
    registry
}

fn base_slot() -> NodeId {
    NodeId::from_namespaced_path(BASE_NAMESPACE, "a.rs", "A")
}

fn kind_at(registry: &Registry, id: NodeId) -> Option<String> {
    registry.find(id).map(|info| info.kind.clone())
}

/// Write one `graft.plan` under its directory selector.
/// 在一个目录选择器下写入一条 `graft.plan`。
fn write_plan(root: &Path, selector: &str, document: &GraftPlanDocument) {
    let directory = graft_record_root(root).join(selector);
    std::fs::create_dir_all(&directory).expect("create record directory");
    std::fs::write(directory.join(lexicon::GRAFT_PLAN_FILE), document.render())
        .expect("write the record");
}

/// The one declaration that hands `root/a` over, written in string form.
/// 交出 `root/a` 的那一条声明，用字符串形式书写。
fn string_declaration() -> StaticGraftCut {
    StaticGraftCut::new("root/a", "slow", false)
}

/// The test that closes the original gap: a plan file on disk must change the
/// effective tree, not merely exist.
/// 填补最初缺口的那条测试：磁盘上的计划文件必须改变有效树，而不只是存在。
#[test]
fn a_record_on_disk_reaches_overlay() {
    let root = TempRoot::new("reaches-overlay");
    write_plan(
        root.path(),
        // The directory is the selector and must equal the document's `graft`;
        // a mismatch is refused (see the mismatch test below).
        // 目录就是选择器，必须等于文档里的 `graft`；不一致会被拒绝（见下面的不匹配测试）。
        "fast",
        &GraftPlanDocument::new(base_slot(), "root/a", "fast", false),
    );

    let base = base_registry();
    let outcome = apply_recorded_grafts(
        &base,
        &external_registry(),
        &[string_declaration()],
        root.path(),
    )
    .expect("the record overlay publishes");

    assert_eq!(
        kind_at(&outcome.effective, base_slot()).as_deref(),
        Some("Fast")
    );
    assert_eq!(kind_at(&base, base_slot()).as_deref(), Some("A"));
    assert!(outcome.reports.iter().any(|report| matches!(
        report,
        RecordReport::DeclarationOverridden { recorded, .. } if recorded == "fast"
    )));
}

/// A hand-renamed directory leaves two candidate implementations and no way to
/// tell which one the author meant, so the whole apply is refused and the message
/// names both.
/// 手工改名的目录会留下两个候选实现，且无法判断作者指的是哪一个，因此整次应用被拒绝，
/// 消息同时报出两者。
#[test]
fn a_directory_that_disagrees_with_the_document_is_refused() {
    let root = TempRoot::new("directory-mismatch");
    write_plan(
        root.path(),
        "renamed_directory",
        &GraftPlanDocument::new(base_slot(), "root/a", "fast", false),
    );

    let error = apply_recorded_grafts(
        &base_registry(),
        &external_registry(),
        &[string_declaration()],
        root.path(),
    )
    .expect_err("a disagreeing directory is refused");

    assert!(error.contains("renamed_directory"), "{error}");
    assert!(error.contains("graft=fast"), "{error}");
    assert!(error.contains("rename"), "{error}");
}

/// A plan that does not parse is a broken artifact, not evidence: the whole apply
/// is refused, every unreadable plan is named, and nothing is silently skipped.
/// 解析不了的计划是坏产物而不是证据：整次应用被拒绝，每个不可读计划都被报出，不会有
/// 任何东西被静默跳过。
#[test]
fn an_unparseable_plan_fails_the_apply() {
    let root = TempRoot::new("unreadable");
    write_plan(
        root.path(),
        // The directory is the selector and must equal the document's `graft`;
        // a mismatch is refused (see the mismatch test below).
        // 目录就是选择器，必须等于文档里的 `graft`；不一致会被拒绝（见下面的不匹配测试）。
        "fast",
        &GraftPlanDocument::new(base_slot(), "root/a", "fast", false),
    );
    let broken = graft_record_root(root.path()).join("broken");
    std::fs::create_dir_all(&broken).expect("create broken directory");
    std::fs::write(
        broken.join(lexicon::GRAFT_PLAN_FILE),
        "version=9\ntarget_path=root/a\ngraft=fast\nfull=false\n",
    )
    .expect("write the broken plan");

    let error = apply_recorded_grafts(
        &base_registry(),
        &external_registry(),
        &[string_declaration()],
        root.path(),
    )
    .expect_err("a broken plan fails the apply");

    assert!(error.contains("1 external graft plan(s)"), "{error}");
    assert!(error.contains("broken"), "{error}");
    assert!(error.contains("version `9`"), "{error}");
    // The readable record did not sneak through either: the apply is all-or-nothing.
    // 可读的那条记录也没有偷偷生效：这次应用是全有或全无。
    assert!(error.contains("no record was applied"), "{error}");
}

/// The loader is not feature-gated: this test compiles and runs with
/// `default-features`, where the `authoring` executor (and `syn`) is absent.
/// 加载器不受特性门控：本测试在 `default-features`（没有 authoring 执行器与 `syn`）
/// 下编译并运行。
#[cfg(not(feature = "authoring"))]
#[test]
fn the_loader_works_without_the_authoring_feature() {
    let root = TempRoot::new("no-authoring");
    write_plan(
        root.path(),
        // The directory is the selector and must equal the document's `graft`;
        // a mismatch is refused (see the mismatch test below).
        // 目录就是选择器，必须等于文档里的 `graft`；不一致会被拒绝（见下面的不匹配测试）。
        "fast",
        &GraftPlanDocument::new(base_slot(), "root/a", "fast", false),
    );

    let loaded = load_graft_records(root.path()).expect("the record loads");
    assert_eq!(loaded.len(), 1);
    assert!(matches!(loaded[0], LoadedGraft::Record(_)));
    let outcome = apply_recorded_grafts(
        &base_registry(),
        &external_registry(),
        &[string_declaration()],
        root.path(),
    )
    .expect("the record overlay publishes without authoring");
    assert_eq!(
        kind_at(&outcome.effective, base_slot()).as_deref(),
        Some("Fast")
    );
}
