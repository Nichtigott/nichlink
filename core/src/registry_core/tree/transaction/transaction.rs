//! Atomic registration batches.
//! 原子注册批次。

use super::entry_pages::RegisteredEntry;
use std::sync::Arc;

use std::collections::{BTreeMap, BTreeSet};

use super::Registry;
use crate::registry_core::declaration::{RegistrationInfo, RegistrationSnapshot, SourceLocation};
use crate::registry_core::diagnostic::{DiagnosticSource, RegistryError, RegistryResult};
use crate::registry_core::identity::{NodeId, StableFaceId};

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
        // `plan_batch` above is the authoritative missing-parent check: it walks
        // every snapshot's parent chain against the *base* tree and rejects a
        // chain that never reaches a registered registry, all before any staging
        // happens. Because every snapshot in `pending` therefore has its parent
        // in the base tree or in an earlier-registered batch entry, the loop can
        // always make progress and cannot leave `pending` non-empty. The
        // historical trailing loop rebuilt the same `<missing-parent:…>` error
        // and was unreachable; keeping two copies risked the two drifting apart,
        // so it was removed.
        // 上面的 `plan_batch` 是缺父检查的权威：它在任何暂存之前，沿每个快照的父链对照
        // **基树**走一遍，拒绝走不到已注册注册机的链。因此 `pending` 里每个快照的父级要么
        // 在基树里、要么在更早注册的批次条目里，循环总能推进，不可能留下非空的 `pending`。
        // 历史上重建同一 `<missing-parent:…>` 错误的尾部循环因此不可达；留两份副本只会让
        // 它们彼此漂移，故删除。
        if failures.is_empty() {
            if let Some(error) = staged.connector_error() {
                return Err(error.into());
            }
            *self = staged;
            Ok(())
        } else {
            Err(Box::new(
                RegistryError::new(
                    self.header.id,
                    self.header.path.clone(),
                    DiagnosticSource::from(SourceLocation {
                        file: "<owned-snapshot-batch>",
                        line: 0,
                        column: 0,
                        function: "Registry::register_snapshot_batch",
                    }),
                    format!("snapshot batch rejected ({} error(s))", failures.len()),
                )
                .with_children(failures),
            ))
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
            *error.children_mut() = contract_failures
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
    use crate::registry_core::declaration::{
        Admission, FrameworkId, OwnedFlowContract, OwnedLocalizedText, OwnedObjectContract,
        OwnedSourceLocation, RegistrationRule,
    };
    use crate::registry_core::identity::root_node_id;

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

    #[test]
    fn parent_rule_is_a_minimum_shape_not_a_kind_filter() {
        let namespace = "structural-rule";
        let mut registry = Registry::root_for_namespace(FrameworkId::new("test"), namespace);
        Arc::make_mut(&mut registry.header).registration_rule = RegistrationRule::new()
            .require_preset("ActionParts")
            .require_parts(&["paint"])
            .require_exports(&["control.render"])
            .require_handle_traits(&["ControlHandle"])
            .require_part_traits(&["ActionParts"])
            .into_owned();
        let mut child = snapshot(namespace, "AnyChildKind");
        child.preset = "ActionParts".to_owned();
        child.contract.provided_parts = vec!["paint".to_owned(), "extra".to_owned()];
        child.exports = vec!["control.render".to_owned(), "control.inspect".to_owned()];
        child.handle_traits = vec!["ControlHandle".to_owned(), "Debug".to_owned()];
        child.part_traits = vec!["ActionParts".to_owned(), "Clone".to_owned()];

        registry
            .register_snapshot_batch([child])
            .expect("a child may use any kind and provide more than the minimum shape");
    }

    #[test]
    fn parent_rule_aggregates_every_missing_structural_requirement() {
        let namespace = "broken-structural-rule";
        let mut registry = Registry::root_for_namespace(FrameworkId::new("test"), namespace);
        Arc::make_mut(&mut registry.header).registration_rule = RegistrationRule::new()
            .require_preset("ActionParts")
            .require_parts(&["paint"])
            .require_exports(&["control.render"])
            .require_handle_traits(&["ControlHandle"])
            .require_part_traits(&["ActionParts"])
            .into_owned();

        let error = registry
            .register_snapshot_batch([snapshot(namespace, "Button")])
            .expect_err("missing parent requirements must reject the child")
            .to_string();

        for missing in [
            "preset `ActionParts` is required",
            "structural part `paint`",
            "export `control.render`",
            "interface `ControlHandle`",
            "interface `ActionParts`",
        ] {
            assert!(
                error.contains(missing),
                "missing diagnostic: {missing}\n{error}"
            );
        }
    }
}
