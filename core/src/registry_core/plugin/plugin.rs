// Plugin contracts and atomic graft descriptions.
// 插件合同与原子嫁接描述。

pub use self::plugin_policy::{PluginAdapter, PluginMode, PluginPolicy, PluginRuntime, PluginSource};

pub use self::contracts::{
    ContractId, FlowContract, FlowContractProvider, FlowSemantic, FrameworkId, OwnedFlowContract,
};
pub use self::trust::{
    PluginRevocation, PluginSignatureVerifier, PluginTrustError, PluginTrustPolicy,
};
pub use self::artifact::{
    PluginArtifact, PluginAssurance, PluginDecision, PluginRejectReason, VerifiedPluginArtifact,
};
pub use self::graft::{CutGraftCommand, GraftCut, GraftError, GraftPlan};
pub use self::catalog::{PluginCatalog, PluginRecord};
pub use self::manifest::PluginManifest;

/// Version of the host/plugin execution ABI.
/// 宿主与插件执行 ABI 的版本。
pub const PLUGIN_ABI_VERSION: u32 = 1;
