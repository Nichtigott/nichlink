//! Overlay (graft-application) operations.
//! 覆盖（嫁接应用）操作。
//!
//! An overlay builds an effective tree from a base tree plus external graft
//! implementations: each cut addresses a logical path in the base tree, and the
//! selected external face occupies that slot while the target's untouched
//! siblings and child registry remain. Selector resolution and identity
//! rebasing live in [`super::resolution`]; the command-dispatch half of the
//! original module lives in [`super`].
//! 覆盖会从基树加外部嫁接实现生成一棵有效树：每个切口指向基树中的逻辑路径，选中的
//! 外部注册面占据该槽位，目标未受影响的兄弟与子注册机保留。选择器解析与身份重新
//! 定基位于 [`super::resolution`]；原模块的命令派发部分位于 [`super`]。

use std::collections::BTreeSet;
use std::sync::Arc;

use super::Registry;
use crate::plugin::{GraftCut, GraftPlan};
use crate::registry_core::declaration::FrameworkId;
use crate::registry_core::declaration::RegistrationSnapshot;
use crate::registry_core::diagnostic::RegistryResult;
use crate::registry_core::identity::NodeId;
use crate::registry_core::plugin::graft::GraftError;
use crate::registry_core::release::{CutTarget, StaticGraftCut};

/// One selector, borrowed from either a dynamic plan or the static table.
/// 一个选择器，借用自动态计划或静态表。
#[derive(Clone, Copy)]
pub(super) enum GraftTargetRef<'a> {
    Path(&'a str),
    Id(NodeId),
}

impl GraftTargetRef<'_> {
    pub(super) fn describe(self) -> String {
        match self {
            Self::Path(path) => path.to_owned(),
            Self::Id(id) => id.to_string(),
        }
    }
}

/// One cut, borrowed from either a dynamic plan or the static table.
/// 一个切口，借用自动态计划或静态表。
#[derive(Clone, Copy)]
pub(super) struct GraftCutRef<'a> {
    pub(super) cut: GraftTargetRef<'a>,
    pub(super) graft: GraftTargetRef<'a>,
    pub(super) end: Option<GraftTargetRef<'a>>,
    pub(super) subtree: bool,
}

impl<'a> GraftCutRef<'a> {
    pub(super) fn dynamic(cut: &'a GraftCut) -> Self {
        Self {
            cut: GraftTargetRef::Path(&cut.cut),
            graft: GraftTargetRef::Path(&cut.graft),
            end: cut.end.as_deref().map(GraftTargetRef::Path),
            subtree: cut.subtree,
        }
    }

    pub(super) fn static_cut(cut: &'a StaticGraftCut) -> Self {
        fn borrow(target: CutTarget) -> GraftTargetRef<'static> {
            match target {
                CutTarget::Path(path) => GraftTargetRef::Path(path),
                CutTarget::Id(id) => GraftTargetRef::Id(id),
            }
        }
        Self {
            cut: borrow(cut.cut()),
            graft: borrow(cut.graft()),
            end: cut.cut_end().map(borrow),
            subtree: cut.full(),
        }
    }
}

/// The outcome of resolving a string graft selector.
/// 字符串 graft 选择器的解析结果。
pub(super) enum Resolution {
    One(NodeId),
    Missing,
    Ambiguous(usize),
}

impl Registry {
    /// Build an effective tree by overlaying external graft implementations.
    ///
    /// Neither `self` nor `external` is moved or edited. Each cut addresses a
    /// logical path in the base tree; the selected external face occupies that
    /// slot while the target's untouched siblings and child registry remain.
    /// 从外部 graft 实现生成有效注册树；不会移动或编辑原树、外部树。每个 cut
    /// 指向原树逻辑路径，外部实现只覆盖该槽位，兄弟和未覆盖子树继续继承。
    pub fn overlay(&self, plan: &GraftPlan, external: &Registry) -> RegistryResult<Self> {
        self.overlay_cuts(
            plan.framework,
            plan.cuts.iter().map(GraftCutRef::dynamic),
            external,
        )
    }

    /// Apply build-captured selectors directly from read-only data.
    ///
    /// This skips `GraftPlan`, `Vec`, and selector `String` construction. The
    /// returned effective Registry remains a runtime object because an
    /// external implementation may be loaded after the binary was built.
    /// 直接应用构建阶段捕获的只读 selector；不会构造 `GraftPlan`、`Vec` 或
    /// selector `String`。由于外部实现可能在二进制生成后才加载，返回的有效
    /// Registry 仍属于运行时对象。
    pub fn overlay_static(
        &self,
        cuts: &[StaticGraftCut],
        external: &Registry,
    ) -> RegistryResult<Self> {
        self.overlay_cuts(
            self.header.framework,
            cuts.iter().map(GraftCutRef::static_cut),
            external,
        )
    }

    pub(super) fn overlay_cuts<'a>(
        &self,
        framework: FrameworkId,
        cuts: impl IntoIterator<Item = GraftCutRef<'a>>,
        external: &Registry,
    ) -> RegistryResult<Self> {
        if framework != self.header.framework || external.header.framework != framework {
            return Err(self.graft_error(self.header.id, GraftError::FrameworkMismatch(framework)));
        }
        let mut staged = self.clone();
        let mut seen = BTreeSet::new();
        for cut in cuts {
            let replacement = match cut.graft {
                GraftTargetRef::Path(value) => match external.resolve_node(value) {
                    Resolution::One(id) => Some(id),
                    Resolution::Missing => None,
                    Resolution::Ambiguous(matches) => {
                        return Err(staged.graft_error(
                            staged.header.id,
                            GraftError::AmbiguousReplacement {
                                selector: value.to_owned(),
                                matches,
                            },
                        ));
                    }
                },
                GraftTargetRef::Id(id) => external.find(id).map(|_| id),
            }
            .ok_or_else(|| {
                // The selector is what identifies the failure; the base root id
                // the variant would carry identifies nothing. See
                // `graft_selector_error` for why the payload stays a `NodeId`.
                // 失败的身份是选择器；变体原本携带的基树根 id 什么都指不出来。载荷为何仍是
                // `NodeId` 见 `graft_selector_error`。
                staged.graft_selector_error(
                    staged.header.id,
                    GraftError::UnknownReplacement(staged.header.id),
                    format!(
                        "graft replacement `{}` is not registered",
                        cut.graft.describe()
                    ),
                )
            })?;
            let replacement = external
                .find(replacement)
                .expect("resolve_node returned a registered external face");
            for target in staged.resolve_cut_targets(cut)? {
                if !seen.insert(target) {
                    return Err(
                        staged.graft_error(target, GraftError::DuplicateCut(cut.cut.describe()))
                    );
                }
                staged.apply_overlay_face(target, replacement, cut.subtree, external)?;
            }
        }
        if let Some(error) = staged.connector_error_with_external(Some(external)) {
            return Err(error.into());
        }
        Ok(staged)
    }

    fn apply_overlay_face(
        &mut self,
        target: NodeId,
        replacement: &RegistrationSnapshot,
        replace_subtree: bool,
        external: &Registry,
    ) -> RegistryResult<()> {
        let target_info = self
            .find(target)
            .cloned()
            .ok_or_else(|| self.graft_error(target, GraftError::UnknownTarget(target)))?;
        if !replacement.flow.is_declared() || !target_info.flow.is_declared() {
            return Err(self.graft_error(target, GraftError::ContractUndeclared));
        }
        if !replacement
            .flow
            .semantically_compatible_with(&target_info.flow)
        {
            return Err(self.graft_error(
                target,
                GraftError::ContractMismatch {
                    expected: target_info.flow.clone(),
                    received: replacement.flow.clone(),
                },
            ));
        }
        let parent = self.registry(target_info.parent).ok_or_else(|| {
            self.graft_error(target, GraftError::UnknownTarget(target_info.parent))
        })?;
        let parent_path = parent.header.path.clone();
        let mut candidate = replacement.clone();
        candidate.id = target_info.id;
        candidate.parent = target_info.parent;
        candidate.registry_name = target_info.registry_name.clone();
        let failures = parent.header.registration_rule.validate(&candidate);
        if !failures.is_empty() {
            let mut error = self.graft_error(
                target,
                GraftError::ContractMismatch {
                    expected: target_info.flow.clone(),
                    received: replacement.flow.clone(),
                },
            );
            *error.message_mut() = format!(
                "graft overlay rejected by destination rule: {}",
                failures.join("; ")
            );
            return Err(error);
        }
        let Some(owner) = self.registry_mut(target_info.parent) else {
            return Err(self.graft_error(target, GraftError::UnknownTarget(target_info.parent)));
        };
        let Some(entry) = Arc::make_mut(&mut owner.entries).get_mut(&target) else {
            return Err(self.graft_error(target, GraftError::UnknownTarget(target)));
        };
        let child = if replace_subtree {
            let mut child = external
                .entry_at(replacement.id)
                .and_then(|entry| entry.child.clone());
            if let Some(registry) = &mut child {
                let old_path = registry.header.path.clone();
                let new_path = format!("{}/{}", parent_path, target_info.registry_name);
                let child = Arc::make_mut(registry);
                // The copied child registry still carries the external
                // replacement's identity. Reconfigure its root before
                // rebasing descendants so `registry(target.id)` and parent
                // lookups continue to address the logical base slot.
                // 外部实现的子注册机仍带着 replacement 身份；先重配置根节点，
                // 再重基后代，确保逻辑槽位的查询和父链保持一致。
                child.reconfigure(&candidate);
                child.rebase_parent_ids(replacement.id, target_info.id);
                child.rebase_paths(&old_path, &new_path);
            }
            child
        } else {
            let mut kept = entry.child.clone();
            if let Some(registry) = &mut kept {
                let registry = Arc::make_mut(registry);
                // The face now declares the replacement's rule, so the registry
                // it owns has to enforce that rule too. A base child the new
                // rule rejects would leave the tree inconsistent, so the graft is
                // refused instead of installing a rule nobody satisfies.
                // 该面现在声明的是替换件的规则，因此它拥有的子注册机也必须执行该规则。
                // 新规则会拒绝的原有子级会让树自相矛盾，因此宁可拒绝这次嫁接，也不装上一
                // 条没人满足的规则。
                //
                // `reconfigure` also takes the replacement's `namespace`, while the
                // children this branch keeps are the base tree's faces — so this
                // subtree's header namespace is foreign to its own entries. That is
                // inert today and deliberately left alone: `header.namespace` is
                // read only by `register_snapshot_batch`, and the kernel is the only
                // caller that can reach a child registry mutably (`registry_mut` is
                // `pub(super)`), so no host can register into a grafted subtree and
                // observe the mismatch. If a public mutable child accessor ever
                // appears, this is the line to fix — reconfigure the child with the
                // *target's* namespace, because the registry still belongs to the
                // base slot. Do not "fix" the `full` branch the same way: the
                // subtree copied there really is the external implementation, and
                // rebasing its path does not make its identity the base's.
                // `reconfigure` 还会取替换件的 `namespace`，而本分支保留的子级仍是原树的
                // 注册面——因此这棵子树的头部 namespace 与自己的条目不同源。今天它是惰性的，
                // 且刻意不动：`header.namespace` 只被 `register_snapshot_batch` 读取，而内核
                // 是唯一能以可变方式触达子注册机的调用方（`registry_mut` 是 `pub(super)`），
                // 因此没有宿主能向被嫁接的子树注册并观察到这处不一致。若将来出现公开的可变子
                // 注册机入口，要修的就是这一行：用**目标槽位**的 namespace 重配置子级，因为这棵
                // 注册机仍属于原槽位。不要把 `full` 分支照此"修正"：那里复制来的子树确实就是
                // 外部实现，重基路径并不会让它拥有原槽位的身份。
                registry.reconfigure(&candidate);
                if let Some(violation) =
                    registry.child_violating(candidate.id, &candidate.registry_rule)
                {
                    let mut error = self.graft_error(
                        target,
                        GraftError::ContractMismatch {
                            expected: target_info.flow.clone(),
                            received: replacement.flow.clone(),
                        },
                    );
                    *error.message_mut() = format!(
                        "graft overlay would leave an existing child violating the new rule: {violation}"
                    );
                    return Err(error);
                }
            }
            kept
        };
        entry.info = Arc::new(candidate);
        entry.child = child;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_core::declaration::RegistrationRule;

    use super::super::fixtures::{FRAMEWORK, face, flow};

    /// A non-full graft keeps the base children, so the registry it leaves
    /// behind has to enforce the replacement's rule — and must not be installed
    /// when a kept child cannot satisfy it.
    /// 非 full 嫁接会保留原有子级，因此留下的子注册机必须执行替换件的规则；当保留下来的
    /// 子级无法满足它时，就不该安装这次嫁接。
    #[test]
    fn a_non_full_graft_does_not_install_a_rule_its_children_violate() {
        let namespace = "rule-graft";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let mut owner = face(namespace, "owner.rs", "Owner", "owner");
        owner.needs_registry = true;
        owner.id = NodeId::from_namespaced_path(namespace, "owner.rs", "Owner");
        let mut child = face(namespace, "child.rs", "Child", "child");
        child.parent = owner.id;
        child.exports = vec!["x".to_owned()];
        root.register_snapshot_batch([owner.clone(), child.clone()])
            .unwrap();

        let mut replacement = face("external", "replacement.rs", "Replacement", "replacement");
        replacement.id = NodeId::from_namespaced_path("external", "replacement.rs", "Replacement");
        replacement.needs_registry = true;
        replacement.registry_rule = RegistrationRule {
            required_exports: &["z"],
            ..RegistrationRule::ANY
        }
        .into_owned();
        let mut external = Registry::root_for_namespace(FRAMEWORK, "external");
        external
            .register_snapshot_batch([replacement.clone()])
            .unwrap();

        let plan = GraftPlan::new(FRAMEWORK).cut("root/owner", "replacement");
        let error = root
            .overlay(&plan, &external)
            .expect_err("a kept child that violates the new rule must refuse the graft");
        let rendered = format!("{error}");
        assert!(rendered.contains("violating the new rule"), "{rendered}");
    }

    #[test]
    fn overlay_keeps_base_siblings_and_source_trees_untouched() {
        let namespace = "overlay";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let mut a = face(namespace, "a.rs", "A", "a");
        a.needs_registry = true;
        a.id = NodeId::from_namespaced_path(namespace, "a.rs", "A");
        let mut a1 = face(namespace, "a1.rs", "A1", "a1");
        a1.parent = a.id;
        let a2 = {
            let mut value = face(namespace, "a2.rs", "A2", "a2");
            value.parent = a.id;
            value
        };
        let mut original = face("external", "original.rs", "Original", "replacement");
        original.id = NodeId::from_namespaced_path("external", "original.rs", "Original");
        original.flow = flow("render.v1");
        let mut external = Registry::root_for_namespace(FRAMEWORK, "external");
        root.register_snapshot_batch([a.clone(), a1.clone(), a2.clone()])
            .unwrap();
        external
            .register_snapshot_batch([original.clone()])
            .unwrap();
        let plan = GraftPlan::new(FRAMEWORK).cut("root/a/a1", "replacement");

        let effective = root.overlay(&plan, &external).unwrap();
        let effective_static = root
            .overlay_static(
                &[StaticGraftCut::new("root/a/a1", "replacement", false)],
                &external,
            )
            .unwrap();
        assert_eq!(effective.find_kind("Original").len(), 1);
        assert_eq!(effective_static.find_kind("Original").len(), 1);
        assert_eq!(effective.path_for(a2.id).as_deref(), Some("root/a/a2"));
        assert_eq!(root.find(a1.id).map(|item| item.kind.as_str()), Some("A1"));
        assert_eq!(
            external.find(original.id).map(|item| item.kind.as_str()),
            Some("Original")
        );
        assert_eq!(effective.path_for(a1.id).as_deref(), Some("root/a/a1"));
    }

    /// A replacement selector that matches nothing must be reported by the text
    /// the author wrote. The old error reused the base tree's root id, which
    /// names a face that *is* registered, so the message pointed the reader at
    /// the wrong identity and dropped the failing selector.
    /// 匹配不到任何面的替换选择器必须按作者写下的文本报出。旧错误复用了基树根 id，而它
    /// 指向一个**确实**注册过的面，消息因此把读者引向错误的身份，还丢掉了失败的选择器。
    #[test]
    fn an_unknown_replacement_names_the_selector() {
        let namespace = "unknown-replacement";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let mut target = face(namespace, "a.rs", "A", "a");
        target.flow = flow("render.v1");
        root.register_snapshot_batch([target]).unwrap();
        let external = Registry::root_for_namespace(FRAMEWORK, "external");

        let plan = GraftPlan::new(FRAMEWORK).cut("root/a", "absent_replacement");
        let error = root
            .overlay(&plan, &external)
            .expect_err("an absent replacement must be refused");
        let rendered = format!("{error}");
        assert!(rendered.contains("absent_replacement"), "{rendered}");
        assert!(rendered.contains("graft replacement"), "{rendered}");
    }

    /// The overlay result has an output path: the dump of the effective tree
    /// names the replacement at the base slot and leaves both inputs untouched.
    /// This is the host-side answer to "what does this plan produce", usable
    /// only where both registries exist.
    /// 覆盖结果有出口：有效树的 dump 在基座槽位上写出替换件，并且两个输入都保持不变。
    /// 这是宿主侧对"这个计划产生什么"的回答，只有在两棵注册树都存在时才可用。
    #[test]
    fn the_effective_tree_dumps_through_the_overlay_result() {
        let namespace = "effective-dump";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let mut target = face(namespace, "a.rs", "A", "a");
        target.flow = flow("render.v1");
        root.register_snapshot_batch([target]).unwrap();
        let mut external = Registry::root_for_namespace(FRAMEWORK, "external");
        let mut replacement = face("external", "b.rs", "B", "replacement");
        replacement.flow = flow("render.v1");
        external.register_snapshot_batch([replacement]).unwrap();

        let dump = root
            .dump_effective(
                &[StaticGraftCut::new("root/a", "replacement", false)],
                &external,
            )
            .expect("the overlay applies");
        assert!(dump.contains("kind=B"), "{dump}");
        assert!(dump.contains("path=root/a"), "{dump}");
        // Both inputs are still exactly what they were.
        // 两个输入仍然是它们原来的样子。
        assert!(root.dump().contains("kind=A"), "base tree untouched");
        assert!(
            external.dump().contains("kind=B"),
            "external tree untouched"
        );
    }
}
