//! Persistent declarations for immutable external grafts.
//! 不可变外部 graft 的持久化声明。

use std::fs;
use std::path::PathBuf;

use crate::{NodeId, Registry};

use super::super::validation::package_root;

/// An external overlay declaration owned by the host package.
/// 宿主包拥有的外部覆盖声明。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExternalGraftPlanFile {
    pub target: NodeId,
    pub graft: String,
    pub full: bool,
    pub root: PathBuf,
}

impl ExternalGraftPlanFile {
    pub fn plan_path(&self) -> PathBuf {
        self.root.join("graft.plan")
    }
}

/// Create an external graft plan without touching the base source tree.
/// 创建外部 graft 计划，不接触原树源码。
pub fn create_external_graft(
    registry: &Registry,
    target: NodeId,
    graft: impl Into<String>,
    full: bool,
) -> Result<ExternalGraftPlanFile, String> {
    let target_path = registry
        .path_for(target)
        .ok_or_else(|| format!("graft target `{target}` is not registered"))?;
    let graft = graft.into();
    if graft.trim().is_empty() || graft.contains(['/', '\\']) {
        return Err("external graft name must be a non-empty selector".to_owned());
    }

    let root = package_root()
        .join(".nichlink/external-grafts")
        .join(&graft);
    if root.exists() {
        return Err(format!("external graft `{graft}` already exists"));
    }
    fs::create_dir_all(&root)
        .map_err(|error| format!("cannot create external graft directory: {error}"))?;

    let plan = format!(
        "version=1\ntarget={target}\ntarget_path={target_path}\ngraft={graft}\nfull={}\n",
        if full { "true" } else { "false" }
    );
    if let Err(error) = fs::write(root.join("graft.plan"), plan) {
        let _ = fs::remove_dir_all(&root);
        return Err(format!("cannot write external graft plan: {error}"));
    }

    Ok(ExternalGraftPlanFile {
        target,
        graft,
        full,
        root,
    })
}
