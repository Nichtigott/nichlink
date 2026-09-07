//! Atomic registration batches.
//! 原子注册批次。

use super::*;

impl Registry {
    /// Submit one batch atomically. Failure leaves the receiver unchanged.
    /// 一批提交作为原子事务处理，失败时接收者完全不变。
    pub fn register_batch<I>(&mut self, submissions: I) -> RegistryResult<()>
    where
        I: IntoIterator<Item = RegistrationInfo>,
    {
        self.register_snapshot_batch(submissions.into_iter().map(RegistrationInfo::into_snapshot))
    }

    /// Register a borrowed slice of compiled declarations atomically.
    /// 原子注册一个借用的编译期声明切片。
    pub fn register_all(&mut self, registrations: &[RegistrationInfo]) -> RegistryResult<()> {
        self.register_batch(registrations.iter().copied())
    }

    /// Atomically attach file-backed snapshots without borrowing or leaking their metadata.
    /// 原子挂载文件快照，不借用也不泄漏快照中的元数据。
    pub fn register_snapshot_batch<I>(&mut self, submissions: I) -> RegistryResult<()>
    where
        I: IntoIterator<Item = RegistrationSnapshot>,
    {
        let submissions = submissions.into_iter().collect::<Vec<_>>();
        self.plan_batch(&submissions)?;
        let mut staged = self.clone();
        let mut pending = submissions;
        pending.sort_by_key(|snapshot| (snapshot.parent, snapshot.id));
        let mut failures = Vec::new();
        let mut progressed = true;
        while !pending.is_empty() && progressed {
            progressed = false;
            let mut waiting = Vec::new();
            for snapshot in pending {
                if staged.registry(snapshot.parent).is_some() {
                    match staged.submit_snapshot_at(snapshot.parent, snapshot) {
                        Ok(()) => progressed = true,
                        Err(error) => failures.push(*error),
                    }
                } else {
                    waiting.push(snapshot);
                }
            }
            pending = waiting;
        }
        for snapshot in pending {
            failures.push(RegistryError::new(
                snapshot.id,
                format!(
                    "<missing-parent:{}>/{}",
                    snapshot.parent, snapshot.registry_name
                ),
                snapshot.source,
                format!(
                    "parent registry `{}` was not found for `{}`",
                    snapshot.parent, snapshot.kind
                ),
            ));
        }
        if failures.is_empty() {
            if let Some(error) = staged.connector_error() {
                return Err(error.into());
            }
            *self = staged;
            Ok(())
        } else {
            Err(Box::new(RegistryError {
                node: self.header.id,
                path: self.header.path.clone(),
                source: DiagnosticSource::from(SourceLocation {
                    file: "<owned-snapshot-batch>",
                    line: 0,
                    column: 0,
                    function: "Registry::register_snapshot_batch",
                }),
                message: format!("snapshot batch rejected ({} error(s))", failures.len()),
                source_chain: Vec::new(),
                call_path: Vec::new(),
                registration_chain: Vec::new(),
                children: failures,
            }))
        }
    }

    /// Check the batch graph before touching the transactional working copy.
    /// 在创建事务副本前检查批次图，避免无意义的重复扫描。
    fn plan_batch(&self, submissions: &[RegistrationSnapshot]) -> RegistryResult<()> {
        let mut parents = BTreeMap::new();
        let mut stable_names = BTreeMap::<StableFaceId, (NodeId, String)>::new();
        let mut existing_ids = BTreeSet::new();
        self.collect_stable_names(&mut stable_names);
        self.collect_node_ids(&mut existing_ids);
        for snapshot in submissions {
            if snapshot.namespace != self.header.namespace {
                return Err(Box::new(RegistryError::new(
                    snapshot.id,
                    snapshot.registry_name.clone(),
                    snapshot.source.clone(),
                    format!(
                        "registration namespace `{}` does not match registry namespace `{}`",
                        snapshot.namespace, self.header.namespace
                    ),
                )));
            }
            if existing_ids.contains(&snapshot.id)
                || parents.insert(snapshot.id, snapshot.parent).is_some()
            {
                return Err(Box::new(RegistryError::new(
                    snapshot.id,
                    snapshot.registry_name.clone(),
                    snapshot.source.clone(),
                    format!(
                        "duplicate registration node identity `{}` in batch",
                        snapshot.id
                    ),
                )));
            }
            if let Some(stable) = snapshot.explicit_stable_face_id() {
                if let Some((existing, existing_path)) = stable_names.get(&stable) {
                    if *existing != snapshot.id {
                        return Err(Box::new(RegistryError::new(
                            snapshot.id,
                            snapshot.registry_name.clone(),
                            snapshot.source.clone(),
                            format!(
                                "duplicate stable face name `{}` (already owned by {} at {})",
                                snapshot.stable_name.as_deref().unwrap_or_default(),
                                existing,
                                existing_path
                            ),
                        )));
                    }
                } else {
                    stable_names.insert(stable, (snapshot.id, snapshot.source.file.clone()));
                }
            }
        }
        for snapshot in submissions {
            let mut current = snapshot.parent;
            let mut seen = BTreeSet::new();
            while let Some(parent) = parents.get(&current).copied() {
                if !seen.insert(current) {
                    return Err(Box::new(RegistryError::new(
                        snapshot.id,
                        snapshot.registry_name.clone(),
                        snapshot.source.clone(),
                        format!("registration parent cycle reaches `{current}`"),
                    )));
                }
                current = parent;
            }
            if self.registry(current).is_none() {
                return Err(Box::new(RegistryError::new(
                    snapshot.id,
                    format!(
                        "<missing-parent:{}>/{}",
                        snapshot.parent, snapshot.registry_name
                    ),
                    snapshot.source.clone(),
                    format!(
                        "parent registry `{}` was not found for `{}`",
                        snapshot.parent, snapshot.kind
                    ),
                )));
            }
        }
        Ok(())
    }

    fn submit_snapshot_at(
        &mut self,
        target: NodeId,
        snapshot: RegistrationSnapshot,
    ) -> RegistryResult<()> {
        let target_path = self
            .registry(target)
            .expect("the caller resolved the parent registry")
            .header
            .path
            .clone();
        let framework = self.header.framework;
        let registry = self
            .registry_mut(target)
            .expect("the caller resolved the parent registry");
        let rule_failures = registry.header.registration_rule.validate(&snapshot);
        if !rule_failures.is_empty() {
            return Err(RegistryError::new(
                snapshot.id,
                format!("{target_path}/{}", snapshot.registry_name),
                snapshot.source.clone(),
                format!(
                    "registration rule rejected `{}` for registry `{target}` (rule `{}`): {}",
                    snapshot.kind,
                    registry.header.registration_rule_path,
                    rule_failures.join("; ")
                ),
            )
            .into());
        }
        let contract_failures = snapshot.contract.validate(&snapshot.kind);
        if !contract_failures.is_empty() {
            let mut error = RegistryError::new(
                snapshot.id,
                format!("{target_path}/{}", snapshot.registry_name),
                snapshot.source.clone(),
                "construction contract rejected registration",
            );
            error.children = contract_failures
                .into_iter()
                .map(|failure| {
                    RegistryError::new(
                        snapshot.id,
                        format!("{target_path}/{}", snapshot.registry_name),
                        snapshot.source.clone(),
                        failure,
                    )
                })
                .collect();
            return Err(Box::new(error));
        }
        if snapshot
            .plugin
            .is_some_and(|plugin| plugin.mode == crate::PluginMode::Replacement)
            && !snapshot.flow.is_declared()
        {
            return Err(RegistryError::new(
                snapshot.id,
                format!("{target_path}/{}", snapshot.registry_name),
                snapshot.source.clone(),
                "replacement plugin must declare a flow contract",
            )
            .into());
        }
        let segment = snapshot.registry_name.clone();
        let child = snapshot.needs_registry.then(|| {
            Arc::new(Registry::new(
                framework,
                snapshot.namespace.clone(),
                &segment,
                &format!("{target_path}/{segment}"),
                snapshot.id,
                snapshot.registry_rule.clone(),
                snapshot.registry_rule_path.clone(),
                snapshot.admission.clone(),
            ))
        });
        Arc::make_mut(&mut registry.entries).insert(
            snapshot.id,
            RegisteredEntry {
                info: Arc::new(snapshot),
                child,
            },
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        OwnedFlowContract, OwnedLocalizedText, OwnedObjectContract, OwnedSourceLocation,
        root_node_id,
    };

    fn snapshot(namespace: &str, kind: &str) -> RegistrationSnapshot {
        RegistrationSnapshot {
            namespace: namespace.to_owned(),
            id: NodeId::from_namespaced_path(namespace, "src/item.rs", kind),
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
            registry_name: kind.to_owned(),
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
            flow: OwnedFlowContract::none(),
            flow_provider: None,
            handle_traits: Vec::new(),
            part_traits: Vec::new(),
            runtime_checks: Vec::new(),
            plugin: None,
            source: OwnedSourceLocation {
                file: "src/item.rs".to_owned(),
                line: 1,
                column: 1,
                function: kind.to_owned(),
            },
        }
    }

    #[test]
    fn rejects_snapshot_from_another_namespace_before_mutation() {
        let mut registry = Registry::root_for_namespace(FrameworkId::new("test"), "library-a");
        let error = registry
            .register_snapshot_batch([snapshot("library-b", "Thing")])
            .expect_err("cross-namespace snapshots must be rejected");
        assert!(error.to_string().contains("namespace"));
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn same_names_are_independent_in_two_registries() {
        let mut left = Registry::root_for_namespace(FrameworkId::new("test"), "library-a");
        let mut right = Registry::root_for_namespace(FrameworkId::new("test"), "library-a");
        left.register_snapshot_batch([snapshot("library-a", "Thing")])
            .unwrap();
        right
            .register_snapshot_batch([snapshot("library-a", "Thing")])
            .unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(right.len(), 1);
        assert_eq!(left.find_kind("Thing").len(), 1);
        assert_eq!(right.find_kind("Thing").len(), 1);
    }
}
