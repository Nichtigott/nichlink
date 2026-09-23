//! Graft records: pure reconciliation of `.nichlink` declarations against a tree.
//! 嫁接记录：把 `.nichlink` 声明与注册树做纯对账。
//!
//! A `graft.plan` file is a declaration an authoring surface writes and a
//! runtime host reads; it is not compiled and the build never applies it. Two
//! readers open it without applying it: the build's undeclared-plan warning
//! (`build_method::graft_plan_check::planned_slots`) and the CLI's `grafts`
//! command. This page owns the pure half of wiring it into
//! [`Registry::overlay`](crate::Registry::overlay): deciding which live slot a
//! record addresses, whether a declaration keeps that slot alive, and whether
//! the record or the declaration names the implementation that finally occupies
//! it. Reading `.nichlink/...` is an execution-surface job and lives in
//! `nichlink-run-method`.
//! `graft.plan` 文件是创作界面写下、运行期宿主读取的声明；它不参与编译，构建也
//! 从不应用它。有两个读者会打开它但不应用：构建的未声明计划警告
//! （`build_method::graft_plan_check::planned_slots`）与 CLI 的 `grafts` 命令。
//! 本页拥有把它接进 [`Registry::overlay`](crate::Registry::overlay)
//! 的纯逻辑那一半：判定一条记录指向哪个现存槽位、是否有声明让该槽位活着，以及最终
//! 由记录还是声明命名占据该槽位的实现。读取 `.nichlink/...` 属于执行面的工作，位于
//! `nichlink-run-method`。

use crate::registry_core::plugin::graft::document::GraftPlanDocument;

// Split decision: the record pipeline changes for three unrelated reasons, so it
// is three sibling pages under three names a reader can look up directly.
// `reconcile` answers "which live slot does this record address" (identity/path
// drift and contradiction); `reports` is the advisory vocabulary and its
// rendering, which grows whenever a new observation is worth printing; `apply`
// owns precedence and builds the one `GraftPlan`, the only page that may touch
// the overlay machinery. `record.rs` keeps the record noun itself and the
// public entry points, re-exported so `graft_ops::record::*` and the
// `graft_ops::*` glob in `run_method` keep naming every public item.
// 拆分决定：记录管线因三个互不相关的原因变化，因此拆成三个同级页面，读者可以直接按名
// 查找。`reconcile` 回答“这条记录指向哪个现存槽位”（身份/路径漂移与矛盾）；`reports`
// 是提示性词表及其渲染，每当出现值得打印的新观察就会增长；`apply` 拥有优先级并构建
// 唯一那份 `GraftPlan`，是唯一可以触碰覆盖机制的页面。`record.rs` 保留记录名词本身与
// 公开入口，并重新导出，使 `graft_ops::record::*` 与 `run_method` 里的
// `graft_ops::*` glob 继续命名每个公开项。
#[path = "apply.rs"]
mod apply;
#[path = "reconcile.rs"]
mod reconcile;
#[path = "reports.rs"]
mod reports;

// Tests live in a sibling test-only page for the same size reason the rest of
// the module was split: they pin behaviour, not the structure of this page, and
// keeping them here would push `record.rs` past the file budget while hiding the
// production surface. The module name `record_tests` is unchanged, so the paths
// the method docs cite (`record_tests::…`) still resolve.
// 测试因与本模块其余部分相同的尺寸理由放在同级的仅测试页面：它们钉的是行为而非本页
// 结构，留在这里会让 `record.rs` 超出文件预算并淹没生产表面。模块名 `record_tests`
// 不变，因此方法文档引用的路径（`record_tests::…`）仍然成立。
#[cfg(test)]
#[path = "record_tests.rs"]
mod record_tests;

pub use self::apply::RecordedOverlay;
pub use self::reconcile::ResolvedRecord;
pub use self::reports::RecordReport;

/// One persisted graft record, keyed by the directory that held it.
/// 一条持久化嫁接记录，以持有它的目录为键。
///
/// `selector` is the directory name under `.nichlink/external-grafts/`; it is a
/// lookup key, not the declared implementation. The document is authoritative
/// for `target`, `target_path`, `graft`, and `full`.
/// `selector` 是 `.nichlink/external-grafts/` 下的目录名，只是查找键，不是声明的
/// 实现。`target`、`target_path`、`graft`、`full` 以文档为准。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecordedGraft {
    /// The `.nichlink/external-grafts/` directory name that held this record; a
    /// lookup key, not the declared implementation.
    /// 持有该记录的 `.nichlink/external-grafts/` 目录名；只是查找键，不是声明的实现。
    pub selector: String,
    /// The plan document read back from that directory; authoritative for the
    /// record's target, path, graft, and granularity.
    /// 从该目录读回的切面文档；记录的 target、路径、graft 与粒度以它为准。
    pub document: GraftPlanDocument,
}

impl RecordedGraft {
    /// Pair a directory selector with the document read back from it.
    /// 把目录选择器与从其中读回的文档配对。
    pub fn new(selector: impl Into<String>, document: GraftPlanDocument) -> Self {
        Self {
            selector: selector.into(),
            document,
        }
    }
}
