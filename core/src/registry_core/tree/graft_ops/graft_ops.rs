//! Replacement and graft operations.
//! 替换与嫁接操作。

use super::*;
use crate::plugin::{GraftCut, GraftPlan};
use crate::registry_core::release::StaticGraftCut;

#[derive(Clone, Copy)]
struct GraftCutRef<'a> {
    cut: &'a str,
    graft: &'a str,
    end: Option<&'a str>,
    subtree: bool,
}

impl<'a> GraftCutRef<'a> {
    fn dynamic(cut: &'a GraftCut) -> Self {
        Self {
            cut: &cut.cut,
            graft: &cut.graft,
            end: cut.end.as_deref(),
            subtree: cut.subtree,
        }
    }

    fn static_cut(cut: &'a StaticGraftCut) -> Self {
        let (path, end) = cut
            .cut()
            .split_once(" to ")
            .map_or((cut.cut(), None), |(start, end)| (start, Some(end)));
        Self {
            cut: path,
            graft: cut.graft(),
            end,
            subtree: cut.full(),
        }
    }
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

    fn overlay_cuts<'a>(
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
            let replacement = external.resolve_node(cut.graft).ok_or_else(|| {
                staged.graft_error(
                    staged.header.id,
                    GraftError::UnknownReplacement(staged.header.id),
                )
            })?;
            let replacement = external
                .find(replacement)
                .expect("resolve_node returned a registered external face");
            for target in staged.resolve_cut_targets(cut)? {
                if !seen.insert(target) {
                    return Err(
                        staged.graft_error(target, GraftError::DuplicateCut(cut.cut.to_owned()))
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
            error.message = format!(
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
            entry.child.clone()
        };
        entry.info = Arc::new(candidate);
        entry.child = child;
        Ok(())
    }

    fn resolve_path(&self, path: &str) -> Option<NodeId> {
        self.depth_first().into_iter().find_map(|info| {
            self.path_for(info.id)
                .filter(|candidate| candidate == path)
                .map(|_| info.id)
        })
    }

    fn resolve_cut_targets(&self, cut: GraftCutRef<'_>) -> RegistryResult<Vec<NodeId>> {
        let start = self.resolve_path(cut.cut).ok_or_else(|| {
            self.graft_error(self.header.id, GraftError::UnknownTarget(self.header.id))
        })?;
        let Some(end_path) = cut.end else {
            return Ok(vec![start]);
        };
        let end = self
            .resolve_path(end_path)
            .ok_or_else(|| self.graft_error(start, GraftError::UnknownTarget(start)))?;
        let start_info = self.find(start).expect("resolved cut start");
        let end_info = self.find(end).expect("resolved cut end");
        if start_info.parent != end_info.parent {
            return Err(self.graft_error(start, GraftError::InvalidRange(end_path.to_owned())));
        }
        let parent = self
            .registry(start_info.parent)
            .ok_or_else(|| self.graft_error(start, GraftError::UnknownTarget(start_info.parent)))?;
        let mut siblings = parent
            .entries
            .values()
            .map(|entry| (entry.info.registry_name.as_str(), entry.info.id))
            .collect::<Vec<_>>();
        siblings.sort_unstable_by_key(|(name, _)| *name);
        let first = siblings
            .iter()
            .position(|(_, id)| *id == start)
            .unwrap_or(0);
        let last = siblings
            .iter()
            .position(|(_, id)| *id == end)
            .unwrap_or(first);
        let (low, high) = if first <= last {
            (first, last)
        } else {
            (last, first)
        };
        Ok(siblings[low..=high].iter().map(|(_, id)| *id).collect())
    }

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
                    &info.registry_name,
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

    fn reconfigure(&mut self, info: &RegistrationSnapshot) {
        let header = Arc::make_mut(&mut self.header);
        header.namespace = info.namespace.clone();
        header.name = info.registry_name.clone();
        header.id = info.id;
        // The path is owned by the registry node and is updated separately by
        // `rebase_paths`; reconfigure only replaces identity and contracts.
        header.registration_rule = info.registry_rule.clone();
        header.registration_rule_path = info.registry_rule_path.clone();
        header.admission = info.admission.clone();
    }

    fn resolve_node(&self, value: &str) -> Option<NodeId> {
        if let Ok(id) = value.parse::<NodeId>() {
            return self.find(id).map(|_| id);
        }
        self.depth_first().into_iter().find_map(|info| {
            let path = self.path_for(info.id)?;
            (info.registry_name == value || info.kind == value || path == value).then_some(info.id)
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

    pub(super) fn take_entry(&mut self, wanted: NodeId) -> Option<RegisteredEntry> {
        let parent = self.find(wanted)?.parent;
        let registry = self.registry_mut(parent)?;
        Arc::make_mut(&mut registry.entries).remove(&wanted)
    }

    fn rebase_parent_ids(&mut self, old: NodeId, new: NodeId) {
        for entry in Arc::make_mut(&mut self.entries).values_mut() {
            if entry.info.parent == old {
                Arc::make_mut(&mut entry.info).parent = new;
            }
            if let Some(child) = entry.child.as_mut() {
                Arc::make_mut(child).rebase_parent_ids(old, new);
            }
        }
    }

    fn rebase_paths(&mut self, old_prefix: &str, new_prefix: &str) {
        if let Some(suffix) = self.header.path.strip_prefix(old_prefix) {
            let next_path = format!("{new_prefix}{suffix}");
            Arc::make_mut(&mut self.header).path = next_path;
        }
        for entry in Arc::make_mut(&mut self.entries).values_mut() {
            if let Some(child) = entry.child.as_mut() {
                Arc::make_mut(child).rebase_paths(old_prefix, new_prefix);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Admission, CutGraftCommand, OwnedFlowContract, OwnedLocalizedText, OwnedObjectContract,
        OwnedSourceLocation, RegistrationRule,
    };

    const FRAMEWORK: FrameworkId = FrameworkId::new("graft-test");

    fn flow(id: &str) -> OwnedFlowContract {
        OwnedFlowContract {
            id: id.to_owned(),
            version: 1,
            input: "LocalCoordinates".to_owned(),
            output: "CanvasFrame".to_owned(),
        }
    }

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
                expected_output: "()".to_owned(),
                actual_output: "()".to_owned(),
            },
            flow: flow("render.v1"),
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

    #[test]
    fn cut_command_supports_single_node_and_full_subtree_forms() {
        let single = CutGraftCommand::parse("cut [root/a1] graft replacement").unwrap();
        assert_eq!(single.cut, "root/a1");
        assert!(!single.full);
        let full = CutGraftCommand::parse("cut [root/a] full graft replacement").unwrap();
        assert_eq!(full.cut, "root/a");
        assert!(full.full);
    }

    #[test]
    fn full_cut_inherits_external_subtree_without_moving_source_trees() {
        let namespace = "overlay-full";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let mut target = face(namespace, "base/a.rs", "BaseA", "a");
        target.needs_registry = true;
        target.id = NodeId::from_namespaced_path(namespace, "base/a.rs", "BaseA");
        let mut base_child = face(namespace, "base/old.rs", "OldChild", "old");
        base_child.parent = target.id;
        let sibling = face(namespace, "base/sibling.rs", "Sibling", "sibling");
        root.register_snapshot_batch([target.clone(), base_child.clone(), sibling.clone()])
            .unwrap();

        let external_namespace = "overlay-full-external";
        let mut replacement = face(external_namespace, "graft/fast_a.rs", "FastA", "fast_a");
        replacement.needs_registry = true;
        replacement.id =
            NodeId::from_namespaced_path(external_namespace, "graft/fast_a.rs", "FastA");
        let mut external_child = face(external_namespace, "graft/new.rs", "NewChild", "new");
        external_child.parent = replacement.id;
        let mut external = Registry::root_for_namespace(FRAMEWORK, external_namespace);
        external
            .register_snapshot_batch([replacement.clone(), external_child.clone()])
            .unwrap();

        let plan = GraftPlan::new(FRAMEWORK).cut("root/a", "fast_a");
        let single = root.overlay(&plan, &external).unwrap();
        assert!(!single.find_kind("OldChild").is_empty());
        assert_eq!(single.find_kind("NewChild").len(), 0);

        let full_plan = GraftPlan::new(FRAMEWORK);
        let full_plan = GraftPlan {
            framework: full_plan.framework,
            cuts: vec![GraftCut::subtree("root/a", "fast_a")],
        };
        let effective = root.overlay(&full_plan, &external).unwrap();
        assert_eq!(effective.find_kind("FastA").len(), 1);
        assert_eq!(effective.find_kind("OldChild").len(), 0);
        assert_eq!(effective.find_kind("NewChild").len(), 1);
        assert_eq!(
            effective.path_for(external_child.id).as_deref(),
            Some("root/a/new")
        );
        assert_eq!(
            effective.path_for(sibling.id).as_deref(),
            Some("root/sibling")
        );
        assert_eq!(root.find_kind("OldChild").len(), 1);
        assert_eq!(external.find_kind("NewChild").len(), 1);
    }
}
