//! Plugin protocol: manifests, catalogs, trust, slots, and graft overlays.
//! 插件协议：manifest、目录、信任、槽位与嫁接覆盖。

#[path = "artifact/artifact.rs"]
pub mod artifact;
#[path = "catalog/catalog.rs"]
pub mod catalog;
#[path = "contracts/contracts.rs"]
pub mod contracts;
#[path = "graft/graft.rs"]
pub mod graft;
#[path = "plugin_policy/plugin_policy.rs"]
pub mod plugin_policy;
#[path = "slot/slot.rs"]
pub mod slot;
#[path = "trust/trust.rs"]
pub mod trust;

// A contract type owned by `declaration` is not re-exported here: it would be a
// second name for the same item under the same glob layer, which is exactly
// what `ambiguous_glob_reexports` reports. The module pages that used to carry
// those copies (`plugin::contracts`, `plugin::manifest`) are gone.
// `declaration` 拥有的合同类型不在这里再导出：那会在同一 glob 层给同一个 item 造出
// 第二个名字，正是 `ambiguous_glob_reexports` 报告的东西。过去承载这些副本的
// `plugin::contracts`、`plugin::manifest` 页面已经删除。

pub use self::plugin_policy::{PluginAdapter, PluginPolicy, PluginRuntime};

pub use self::artifact::{
    PluginArtifact, PluginAssurance, PluginDecision, PluginRejectReason, VerifiedPluginArtifact,
};
pub use self::catalog::{PluginCatalog, PluginRecord};
/// The document module's historical path on this surface.
/// 文档模块在本执行面上的历史路径。
pub use self::graft::document as graft_document;
pub use self::graft::document::{
    GRAFT_PLAN_VERSION, GraftPlanDocument, GraftPlanDocumentError, validate_graft_selector,
};
pub use self::graft::{CutGraftCommand, GraftCut, GraftError, GraftPlan};
pub use self::slot::{
    PluginChannel, SlotValidationError, validate_artifact, validate_operation_name,
};
pub use self::trust::{
    PluginRevocation, PluginSignatureVerifier, PluginTrustError, PluginTrustPolicy,
};

/// Version of the host/plugin execution ABI.
/// 宿主与插件执行 ABI 的版本。
pub const PLUGIN_ABI_VERSION: u32 = 1;
