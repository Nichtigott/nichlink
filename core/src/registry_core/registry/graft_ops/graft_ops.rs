//! Replacement and graft operations.
//! 替换与嫁接操作。

use super::*;

impl Registry {
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

    /// Replace one registered implementation with another atomically.
    /// 原子地用一个已注册实现替换另一个实现。
    ///
    /// The replacement keeps the target's child registry and display slot.
    /// A candidate carrying its own populated child registry is rejected so
    /// grafting cannot silently discard a subtree.
    /// 替换会保留目标的子注册机和展示插槽。候选实现若带有非空子注册机会被拒绝，
    /// 避免嫁接悄悄丢失一棵子树。
    pub fn graft(&mut self, request: GraftRequest) -> RegistryResult<()> {
        if request.framework != self.header.framework {
            return Err(self.graft_error(
                request.target,
                GraftError::FrameworkMismatch(request.framework),
            ));
        }
        let target = self.find(request.target).cloned().ok_or_else(|| {
            self.graft_error(request.target, GraftError::UnknownTarget(request.target))
        })?;
        let replacement = self.find(request.replacement).cloned().ok_or_else(|| {
            self.graft_error(
                request.replacement,
                GraftError::UnknownReplacement(request.replacement),
            )
        })?;
        if target.id == replacement.id {
            return Err(self.graft_error(target.id, GraftError::SameNode));
        }
        if self
            .node_path(target.id)
            .zip(self.node_path(replacement.id))
            .is_some_and(|(target_path, replacement_path)| {
                replacement_path.starts_with(&target_path)
                    || target_path.starts_with(&replacement_path)
            })
        {
            return Err(self.graft_error(target.id, GraftError::OverlappingSubtree));
        }
        if !target.flow.is_declared() || !replacement.flow.is_declared() {
            return Err(self.graft_error(target.id, GraftError::ContractUndeclared));
        }
        if target.flow != request.target_contract
            || replacement.flow != request.replacement_contract
        {
            return Err(self.graft_error(
                target.id,
                GraftError::ContractMismatch {
                    expected: request.target_contract.clone(),
                    received: target.flow.clone(),
                },
            ));
        }
        if !replacement.flow.semantically_compatible_with(&target.flow) {
            return Err(self.graft_error(
                replacement.id,
                GraftError::ContractMismatch {
                    expected: target.flow.clone(),
                    received: replacement.flow.clone(),
                },
            ));
        }

        // Validate the candidate against the target parent's rule as well.
        // A candidate may come from another branch with a different admission
        // rule; moving it must not bypass the destination contract.
        // 候选来自另一分支时，还要按目标父注册机的规范重新校验，不能绕过目标门槛。
        let mut destination_info = replacement.clone();
        destination_info.parent = target.parent;
        destination_info.registry_name = target.registry_name.clone();
        if let Some(parent) = self.registry(target.parent) {
            let failures = parent.header.registration_rule.validate(&destination_info);
            if !failures.is_empty() {
                let mut error = self.graft_error(
                    target.id,
                    GraftError::ContractMismatch {
                        expected: target.flow.clone(),
                        received: replacement.flow.clone(),
                    },
                );
                error.message = format!(
                    "graft candidate rejected by destination rule: {}",
                    failures.join("; ")
                );
                return Err(error);
            }
        }

        let mut staged = self.clone();
        let candidate = staged
            .take_entry(request.replacement)
            .expect("graft replacement was found in the live tree");
        if candidate
            .child
            .as_ref()
            .is_some_and(|child| !child.is_empty())
        {
            return Err(self.graft_error(request.replacement, GraftError::ReplacementHasChildren));
        }
        let mut target_entry = staged
            .take_entry(request.target)
            .expect("graft target was found in the live tree");
        let old_id = target_entry.info.id;
        let mut replacement_info = (*candidate.info).clone();
        replacement_info.parent = target_entry.info.parent;
        // The target name is the stable logical slot. The candidate keeps its
        // own node identity and kind while occupying that slot.
        // 目标名称是稳定的逻辑槽位；候选保留自己的 node identity 和 kind，但占据该槽位。
        replacement_info.registry_name = target_entry.info.registry_name.clone();
        let child = target_entry.child.take();
        if let Some(mut child) = child {
            let old_path = child.header.path.clone();
            let new_path = format!(
                "{}/{}",
                self.path_for(replacement_info.parent)
                    .unwrap_or_else(|| self.header.path.clone()),
                replacement_info.registry_name
            );
            let child_ref = Arc::make_mut(&mut child);
            child_ref.reconfigure(&replacement_info);
            child_ref.rebase_paths(&old_path, &new_path);
            child_ref.rebase_parent_ids(old_id, replacement_info.id);
            target_entry.child = Some(child);
        }
        let target_parent = replacement_info.parent;
        target_entry.info = Arc::new(replacement_info);
        if !staged.insert_entry_at(target_parent, target_entry) {
            return Err(self.graft_error(request.target, GraftError::UnknownTarget(request.target)));
        }
        if let Some(error) = staged.connector_error() {
            return Err(error.into());
        }
        *self = staged;
        Ok(())
    }

    /// Resolve and execute a human-facing graft command using registered names,
    /// kinds, paths, or node identity values. Contracts come from the live nodes.
    /// 解析并执行面向人的嫁接命令；端点可用名称、kind、路径或 node identity，合同从线上节点读取。
    pub fn graft_command(&mut self, command: &str) -> RegistryResult<()> {
        let command = GraftCommand::parse(command).map_err(|message| {
            self.graft_error(self.header.id, GraftError::InvalidCommand(message))
        })?;
        let target = self.resolve_node(&command.target).ok_or_else(|| {
            self.graft_error(self.header.id, GraftError::UnknownTarget(self.header.id))
        })?;
        let replacement = self.resolve_node(&command.replacement).ok_or_else(|| {
            self.graft_error(
                self.header.id,
                GraftError::UnknownReplacement(self.header.id),
            )
        })?;
        let target_flow = self
            .find(target)
            .map(|info| info.flow.clone())
            .unwrap_or_else(crate::OwnedFlowContract::none);
        let replacement_flow = self
            .find(replacement)
            .map(|info| info.flow.clone())
            .unwrap_or_else(crate::OwnedFlowContract::none);
        self.graft(GraftRequest::new(
            self.header.framework,
            target,
            replacement,
            target_flow,
            replacement_flow,
            crate::PluginSource::User,
        ))
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
                function: "Registry::graft",
            },
            error.to_string(),
        ))
    }

    pub(super) fn take_entry(&mut self, wanted: NodeId) -> Option<RegisteredEntry> {
        let parent = self.find(wanted)?.parent;
        let registry = self.registry_mut(parent)?;
        Arc::make_mut(&mut registry.entries).remove(&wanted)
    }

    fn insert_entry_at(&mut self, parent: NodeId, entry: RegisteredEntry) -> bool {
        if let Some(registry) = self.registry_mut(parent) {
            Arc::make_mut(&mut registry.entries).insert(entry.info.id, entry);
            return true;
        }
        false
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
