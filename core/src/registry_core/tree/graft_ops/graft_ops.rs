//! Replacement and graft commands.
//! 替换与嫁接命令。
//!
//! This page owns the command-dispatch half of the historical module: validating
//! and committing authored replacements, migrating a subtree to a new source
//! path, and the shared graft diagnostics. The overlay application itself lives
//! in `overlay`, its selector resolution and identity rebasing in
//! `resolution`. Every operation is still an inherent method on [`Registry`],
//! so the `tree::graft_ops::*` call paths are unchanged.
//! 本页拥有历史模块中命令派发的那一半：校验并提交作者侧替换、把子树迁移到新的源码
//! 路径，以及共用的嫁接近诊断。覆盖应用本身位于 `overlay`，其选择器解析与身份重新
//! 定基位于 `resolution`。所有操作仍是 [`Registry`] 的固有方法，因此
//! `tree::graft_ops::*` 的调用路径保持不变。

use super::entry_pages::RegisteredEntry;
use std::sync::Arc;

use super::Registry;
use crate::registry_core::declaration::{
    OwnedRegistrationRule, RegistrationInfo, RegistrationSnapshot, SourceLocation,
};
use crate::registry_core::diagnostic::{RegistryError, RegistryResult};
use crate::registry_core::identity::NodeId;
use crate::registry_core::plugin::graft::GraftError;

#[cfg(test)]
#[path = "fixtures.rs"]
mod fixtures;
#[path = "overlay.rs"]
mod overlay;
#[path = "record.rs"]
mod record;
#[path = "resolution.rs"]
mod resolution;

// The record vocabulary is the kernel half of the `.nichlink` wiring; a runtime
// surface names it through this page.
// 记录词表是 `.nichlink` 接线的内核一半；运行期执行面经本页命名它。
pub use self::record::*;

impl Registry {
    /// Validate a source-path migration by replacing one whole subtree in a
    /// staged registry. The live registry is untouched.
    /// 在暂存注册树中校验整个源码路径子树迁移，实时注册树不会被修改。
    pub fn validate_snapshot_migration(
        &self,
        current: NodeId,
        replacements: Vec<RegistrationSnapshot>,
    ) -> RegistryResult<()> {
        let existing = self.find(current).ok_or_else(|| {
            Box::new(RegistryError::new(
                current,
                self.header.path.clone(),
                SourceLocation {
                    file: "<migration>",
                    line: 0,
                    column: 0,
                    function: "Registry::validate_snapshot_migration",
                },
                "migrated face is no longer present in the registry",
            ))
        })?;
        let Some(root) = replacements
            .iter()
            .find(|item| item.parent == existing.parent)
        else {
            return Err(Box::new(RegistryError::new(
                current,
                self.path_for(current)
                    .unwrap_or_else(|| self.header.path.clone()),
                existing.source.clone(),
                "migrated subtree has no replacement root with the original parent",
            )));
        };
        let mut staged = self.clone();
        if staged.take_entry(current).is_none() {
            return Err(Box::new(RegistryError::new(
                current,
                self.header.path.clone(),
                root.source.clone(),
                "migrated subtree could not be detached from the registry",
            )));
        }
        staged.register_snapshot_batch(replacements)
    }

    /// Validate an authored replacement without committing it, by way of its
    /// snapshot form.
    /// 校验作者侧替换但不提交，经由其快照形式完成。
    pub fn validate_replacement(
        &self,
        current: NodeId,
        info: RegistrationInfo,
    ) -> RegistryResult<()> {
        self.validate_snapshot_replacement(current, info.into_snapshot())
    }

    /// Validate an authored replacement without committing it.
    /// 校验 authored 替换，但不修改当前注册树。
    pub fn validate_snapshot_replacement(
        &self,
        current: NodeId,
        info: RegistrationSnapshot,
    ) -> RegistryResult<()> {
        let existing = self.find(current).ok_or_else(|| {
            Box::new(RegistryError::new(
                current,
                self.header.path.clone(),
                info.source.clone(),
                "edited face is no longer present in the registry",
            ))
        })?;
        if info.namespace != self.header.namespace {
            return Err(Box::new(RegistryError::new(
                info.id,
                self.header.path.clone(),
                info.source.clone(),
                format!(
                    "replacement namespace `{}` does not match registry namespace `{}`",
                    info.namespace, self.header.namespace
                ),
            )));
        }
        if info.id != current {
            return Err(Box::new(RegistryError::new(
                current,
                self.header.path.clone(),
                info.source.clone(),
                format!(
                    "node identity cannot be edited in place (current `{current}`, requested `{}`); use subtree migration",
                    info.id
                ),
            )));
        }
        if info.parent != existing.parent {
            return Err(Box::new(RegistryError::new(
                current,
                self.header.path.clone(),
                info.source.clone(),
                format!(
                    "parent registry cannot be edited in place (current `{}`, requested `{}`); use subtree migration",
                    existing.parent, info.parent
                ),
            )));
        }
        let target = self.registry(info.parent).ok_or_else(|| {
            Box::new(RegistryError::new(
                info.id,
                format!("<missing-parent:{}>/{}", info.parent, info.registry_name),
                info.source.clone(),
                format!("parent registry `{}` was not found", info.parent),
            ))
        })?;
        let rule_failures = target.header.registration_rule.validate(&info);
        if !rule_failures.is_empty() {
            return Err(Box::new(RegistryError::new(
                info.id,
                format!("{}/{}", target.header.path, info.registry_name),
                info.source.clone(),
                format!(
                    "registration rule rejected edited face: {}",
                    rule_failures.join("; ")
                ),
            )));
        }
        let contract_failures = info.contract.validate(&info.kind);
        if !contract_failures.is_empty() {
            return Err(Box::new(RegistryError::new(
                info.id,
                format!("{}/{}", target.header.path, info.registry_name),
                info.source.clone(),
                format!(
                    "edited construction contract rejected: {}",
                    contract_failures.join("; ")
                ),
            )));
        }
        // The edit may tighten the rule this face enforces on its own children.
        // Checking only the edited face against *its* parent left the tree
        // internally inconsistent: a child that no longer satisfies the new rule
        // stayed put, and only some later insertion failed.
        // 这次编辑可能收紧该面对自己子级的规则。只校验被编辑的面与其父级，会让树内部
        // 自相矛盾：不再满足新规则的子级留在原处，只有之后某次插入才会失败。
        if self.registry(info.id).is_some()
            && let Some(violation) = self.child_violating(info.id, &info.registry_rule)
        {
            return Err(Box::new(RegistryError::new(
                info.id,
                format!("<edited>/{}/{}", info.kind, info.registry_name),
                info.source.clone(),
                format!(
                    "registration rule rejected existing child {violation}; edit or move it first"
                ),
            )));
        }
        let mut staged = self.clone();
        let source = info.source.clone();
        if !staged.replace_info(current, info) {
            return Err(Box::new(RegistryError::new(
                current,
                self.header.path.clone(),
                source,
                "edited face is no longer present in the registry",
            )));
        }
        if let Some(error) = staged.connector_error() {
            return Err(error.into());
        }
        Ok(())
    }

    /// Commit one already-authored replacement after validating it against the
    /// same structural and connector rules used by registration.
    /// 使用与注册相同的结构和连接校验，提交一个已经生成的替换快照。
    pub fn apply_snapshot_replacement(
        &mut self,
        current: NodeId,
        info: RegistrationSnapshot,
    ) -> RegistryResult<()> {
        self.validate_snapshot_replacement(current, info.clone())?;
        let mut staged = self.clone();
        if !staged.replace_info(current, info) {
            return Err(self.graft_error(current, GraftError::UnknownTarget(current)));
        }
        *self = staged;
        Ok(())
    }

    fn replace_info(&mut self, wanted: NodeId, info: RegistrationSnapshot) -> bool {
        let Some(parent) = self.find(wanted).map(|entry| entry.parent) else {
            return false;
        };
        let Some(registry) = self.registry_mut(parent) else {
            return false;
        };
        let parent_path = registry.header.path.clone();
        let framework = registry.header.framework;
        let Some(entry) = Arc::make_mut(&mut registry.entries).get_mut(&wanted) else {
            return false;
        };
        let mut child = entry.child.take();
        match (&mut child, info.needs_registry) {
            (Some(child_registry), true) => {
                let old_path = child_registry.header.path.clone();
                let new_path = format!("{parent_path}/{}", info.registry_name);
                let child_registry = Arc::make_mut(child_registry);
                child_registry.reconfigure(&info);
                child_registry.rebase_paths(&old_path, &new_path);
            }
            (None, true) => {
                child = Some(Arc::new(Registry::new(
                    framework,
                    info.namespace.clone(),
                    &format!("{parent_path}/{}", info.registry_name),
                    info.id,
                    info.registry_rule.clone(),
                    info.registry_rule_path.clone(),
                    info.admission.clone(),
                )));
            }
            (Some(child_registry), false) if !child_registry.entries.is_empty() => {
                entry.child = Some(child_registry.clone());
                return false;
            }
            (Some(_), false) => child = None,
            (None, false) => {}
        }
        *entry = RegisteredEntry {
            info: Arc::new(info),
            child,
        };
        true
    }

    /// The first direct child of `parent` that `rule` would reject.
    /// `rule` 会拒绝的、`parent` 的第一个直接子级。
    ///
    /// Only direct children: a rule governs what a face admits, not what its
    /// grandchildren admit.
    /// 只看直接子级：规则约束的是一个面接纳什么，而不是它的孙辈接纳什么。
    fn child_violating(&self, parent: NodeId, rule: &OwnedRegistrationRule) -> Option<String> {
        self.depth_first()
            .iter()
            .filter(|child| child.parent == parent)
            .find_map(|child| {
                let failures = rule.validate(child);
                (!failures.is_empty()).then(|| format!("`{}`: {}", child.kind, failures.join("; ")))
            })
    }

    fn graft_error(&self, node: NodeId, error: GraftError) -> Box<RegistryError> {
        Box::new(RegistryError::new(
            node,
            self.path_for(node)
                .unwrap_or_else(|| self.header.path.clone()),
            SourceLocation {
                file: "<graft>",
                line: 0,
                column: 0,
                function: "Registry::overlay",
            },
            error.to_string(),
        ))
    }

    /// Build the error for a graft selector that named no registered face.
    ///
    /// `GraftError::UnknownTarget`/`UnknownReplacement` carry a `NodeId`, but a
    /// *path* selector has no identity to put there. Filling in the base tree's
    /// root id — the historical behaviour — names an unrelated face and drops
    /// the selector the author actually wrote, so the rendered error cannot say
    /// what failed. The public variants keep their shape; the selector is
    /// carried through `message`, which is the field operators read. A selector
    /// written as a `NodeId` is described the same way, so one path covers both.
    /// 为“没有匹配到任何已注册面”的嫁接选择器构造错误。
    /// `GraftError::UnknownTarget`/`UnknownReplacement` 携带的是 `NodeId`，而**路径**
    /// 选择器没有身份可放。填入基树根 id（历史行为）会指向一个无关的面，并丢掉作者真正
    /// 写下的选择器，渲染出的错误因此说不出哪里失败。公开变体形状不变，选择器经 `message`
    /// 承载——那才是运维读的字段。写成 `NodeId` 的选择器用同样方式描述，因此一条路径覆盖
    /// 两种写法。
    fn graft_selector_error(
        &self,
        node: NodeId,
        error: GraftError,
        message: String,
    ) -> Box<RegistryError> {
        let mut error = self.graft_error(node, error);
        *error.message_mut() = message;
        error
    }

    pub(super) fn take_entry(&mut self, wanted: NodeId) -> Option<RegisteredEntry> {
        let parent = self.find(wanted)?.parent;
        let registry = self.registry_mut(parent)?;
        Arc::make_mut(&mut registry.entries).remove(&wanted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_core::declaration::RegistrationRule;
    use crate::registry_core::plugin::graft::CutGraftCommand;

    use super::fixtures::{FRAMEWORK, face};

    /// Tightening a face's rule must not leave an existing child behind that the
    /// new rule rejects: the file-authoring path gates its write on this very
    /// check, so accepting it wrote a tree that can never be built again.
    /// 收紧某个面的规则时，不能把新规则会拒绝的既有子级留下：文件授权路径正是以这项
    /// 检查作为写盘闸门，接受它就等于写下一棵再也构建不出来的树。
    #[test]
    fn an_edit_that_invalidates_an_existing_child_is_refused() {
        let namespace = "rule-edit";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let mut owner = face(namespace, "owner.rs", "Owner", "owner");
        owner.needs_registry = true;
        owner.id = NodeId::from_namespaced_path(namespace, "owner.rs", "Owner");
        let mut child = face(namespace, "child.rs", "Child", "child");
        child.parent = owner.id;
        child.exports = vec!["x".to_owned()];
        root.register_snapshot_batch([owner.clone(), child.clone()])
            .unwrap();

        let mut tightened = owner.clone();
        tightened.registry_rule = RegistrationRule {
            required_exports: &["y"],
            ..RegistrationRule::ANY
        }
        .into_owned();
        let error = root
            .validate_snapshot_replacement(owner.id, tightened)
            .expect_err("a rule that rejects an existing child must be refused");
        let rendered = format!("{error}");
        assert!(rendered.contains("rejected existing child"), "{rendered}");
        assert!(rendered.contains("Child"), "{rendered}");
    }

    #[test]
    fn cut_command_supports_single_node_and_full_subtree_forms() {
        let single = CutGraftCommand::parse("cut [root/a1] graft replacement").unwrap();
        assert_eq!(single.cut, "root/a1");
        assert!(!single.full);
        let full = CutGraftCommand::parse("cut [root/a] full graft replacement").unwrap();
        assert_eq!(full.cut, "root/a");
        assert!(full.full);
    }

    /// The command grammar's separator is the standalone word `to`, and both
    /// endpoints come out as separate fields.
    /// 命令语法的分隔符是独立的词 `to`，两个端点以独立字段返回。
    #[test]
    fn cut_command_reads_a_range_as_two_endpoints() {
        let range = CutGraftCommand::parse("cut [root/a to root/b] graft replacement").unwrap();
        assert_eq!(range.cut, "root/a");
        assert_eq!(range.end.as_deref(), Some("root/b"));
        // The far endpoint is the rest of the text, spaces and all, exactly as
        // the previous substring split treated it.
        // 远端是余下的整段文本（含空格），与过去的子串拆分完全一致。
        let chained =
            CutGraftCommand::parse("cut [root/a to root/b to root/c] graft replacement").unwrap();
        assert_eq!(chained.cut, "root/a");
        assert_eq!(chained.end.as_deref(), Some("root/b to root/c"));
    }

    /// A `to` with no text on one side is ordinary path text, not a separator;
    /// this is the edge `split_once(" to ")` also left alone.
    /// 某一侧没有文本的 `to` 是普通路径文本而非分隔符；这也是
    /// `split_once(" to ")` 同样不处理的边界。
    #[test]
    fn cut_command_keeps_a_word_boundary_to_as_path_text() {
        let lone = CutGraftCommand::parse("cut [to] graft replacement").unwrap();
        assert_eq!(lone.cut, "to");
        assert_eq!(lone.end, None);
        let trailing = CutGraftCommand::parse("cut [root/a to] graft replacement").unwrap();
        assert_eq!(trailing.cut, "root/a to");
        assert_eq!(trailing.end, None);
    }
}
