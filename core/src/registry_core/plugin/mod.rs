#[path = "artifact/artifact.rs"]
pub mod artifact;
#[path = "catalog/catalog.rs"]
pub mod catalog;
#[path = "contracts/contracts.rs"]
pub mod contracts;
#[path = "graft/graft.rs"]
pub mod graft;
#[path = "manifest/manifest.rs"]
pub mod manifest;
#[path = "plugin_policy/plugin_policy.rs"]
pub mod plugin_policy;
#[path = "trust/trust.rs"]
pub mod trust;

include!("plugin.rs");
