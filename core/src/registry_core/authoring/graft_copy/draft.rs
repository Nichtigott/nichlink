//! Persistent, inactive graft drafts.
//! 可持久化、未激活的 graft 草稿。

use std::fs;
use std::path::{Path, PathBuf};

use crate::NodeId;

use super::super::validation::{is_parent_component, package_root};

/// An inactive source copy prepared for one target face.
/// 为一个目标注册面准备、尚未进入正式注册树的源码副本。
#[derive(Clone, Debug)]
pub struct GraftDraft {
    pub(super) target: NodeId,
    pub(super) name: String,
    pub(super) root: PathBuf,
    pub(super) source: PathBuf,
}

impl GraftDraft {
    pub const fn target(&self) -> NodeId {
        self.target
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn source(&self) -> &Path {
        &self.source
    }

    fn load(root: PathBuf) -> Result<Self, String> {
        let plan_path = root.join("graft.plan");
        let plan = fs::read_to_string(&plan_path)
            .map_err(|error| format!("cannot read {}: {error}", plan_path.display()))?;
        let field = |name: &str| {
            plan.lines()
                .find_map(|line| line.strip_prefix(&format!("{name}=")))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("{} is missing `{name}`", plan_path.display()))
        };
        if field("version")? != "1" {
            return Err(format!(
                "{} uses an unsupported version",
                plan_path.display()
            ));
        }
        let target = field("target")?
            .parse::<NodeId>()
            .map_err(|_| format!("{} has an invalid target", plan_path.display()))?;
        let relative = Path::new(field("draft_source")?);
        if relative.is_absolute() || relative.components().any(is_parent_component) {
            return Err(format!(
                "{} has an unsafe draft source",
                plan_path.display()
            ));
        }
        let name = root
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| "graft draft has an invalid directory name".to_owned())?
            .to_owned();
        let source = root.join(relative);
        if !source.is_file() {
            return Err(format!(
                "graft draft source {} is missing",
                source.display()
            ));
        }
        Ok(Self {
            target,
            name,
            root,
            source,
        })
    }
}

/// Load inactive graft drafts owned by the current package.
/// 读取当前包拥有、尚未激活的 graft 草稿。
pub fn graft_drafts() -> Result<Vec<GraftDraft>, String> {
    let directory = package_root().join(".nichlink/grafts");
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut roots = fs::read_dir(&directory)
        .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.is_dir() && path.join("graft.plan").is_file())
        .collect::<Vec<_>>();
    roots.sort();
    roots.into_iter().map(GraftDraft::load).collect()
}
