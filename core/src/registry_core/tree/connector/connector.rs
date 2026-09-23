//! Registration connector and dependency admission checks.
//! 注册连接器与依赖准入检查。

use super::Registry;
use crate::registry_core::declaration::{RegistrationSnapshot, SourceLocation};
use crate::registry_core::diagnostic::{DiagnosticSource, RegistrationState, RegistryError};
use crate::registry_core::identity::NodeId;
use crate::registry_core::lexicon::path_is_strictly_under;

impl Registry {
    pub(super) fn registration_chain(&self, wanted: NodeId) -> Vec<RegistrationState> {
        let mut chain = vec![RegistrationState {
            node: self.header.id,
            path: self.header.path.clone(),
            source: DiagnosticSource::from(SourceLocation {
                file: "<registry-root>",
                line: 0,
                column: 0,
                function: "Registry::root",
            }),
            state: format!("entries={}", self.len()),
        }];
        self.collect_registration_chain(wanted, &mut chain);
        chain
    }

    fn collect_registration_chain(
        &self,
        wanted: NodeId,
        chain: &mut Vec<RegistrationState>,
    ) -> bool {
        for entry in self.entries.values() {
            let is_target = entry.info.id == wanted;
            let contains_target = entry
                .child
                .as_ref()
                .is_some_and(|child| child.entry_at(wanted).is_some());
            if !is_target && !contains_target {
                continue;
            }
            chain.push(RegistrationState {
                node: entry.info.id,
                path: format!("{}/{}", self.header.path, entry.info.registry_name),
                source: entry.info.source.clone().into(),
                state: format!(
                    "kind={} exports={:?} child_registry={}",
                    entry.info.kind,
                    entry.info.exports,
                    entry.child.is_some()
                ),
            });
            if !is_target {
                entry
                    .child
                    .as_ref()
                    .expect("contains_target requires a child registry")
                    .collect_registration_chain(wanted, chain);
            }
            return true;
        }
        false
    }

    /// Find a provider on the parent chain of one registry.
    /// 在某个注册机的父链上查找能力提供者。
    fn ancestor_providers(
        &self,
        target: NodeId,
        capability: &str,
        expected_kind: &str,
    ) -> Vec<&RegistrationSnapshot> {
        for entry in self.entries.values() {
            let Some(child) = entry.child.as_ref() else {
                continue;
            };
            if child.header.id != target && child.registry(target).is_none() {
                continue;
            }
            let mut providers = Vec::new();
            if entry.info.provides.iter().any(|value| value == capability)
                && entry.info.kind == expected_kind
            {
                providers.push(entry.info.as_ref());
            }
            providers.extend(child.ancestor_providers(target, capability, expected_kind));
            return providers;
        }
        Vec::new()
    }

    /// Find any parent-chain provider, used to explain a kind mismatch.
    /// 查找父链上的任意能力提供者，用于解释 provider kind 不匹配。
    fn ancestor_capability(
        &self,
        target: NodeId,
        capability: &str,
    ) -> Option<&RegistrationSnapshot> {
        for entry in self.entries.values() {
            let Some(child) = entry.child.as_ref() else {
                continue;
            };
            if child.header.id != target && child.registry(target).is_none() {
                continue;
            }
            if entry.info.provides.iter().any(|value| value == capability) {
                return Some(entry.info.as_ref());
            }
            if let Some(found) = child.ancestor_capability(target, capability) {
                return Some(found);
            }
        }
        None
    }

    fn provider_for(
        &self,
        target: NodeId,
        capability: &str,
        expected_kind: &str,
    ) -> Result<Option<&RegistrationSnapshot>, Vec<&RegistrationSnapshot>> {
        let ancestors = self.ancestor_providers(target, capability, expected_kind);
        if ancestors.len() == 1 {
            return Ok(ancestors.into_iter().next());
        }
        if ancestors.len() > 1 {
            return Err(ancestors);
        }
        let providers = self.find_where(|info| {
            info.provides.iter().any(|value| value == capability) && info.kind == expected_kind
        });
        match providers.len() {
            0 => Ok(None),
            1 => Ok(providers.into_iter().next()),
            _ => Err(providers),
        }
    }

    /// Find any provider for a capability, without applying the kind filter.
    /// 查找能力的任意提供者，不提前套用 kind 过滤。
    fn provider_for_capability(
        &self,
        target: NodeId,
        capability: &str,
    ) -> Option<&RegistrationSnapshot> {
        self.ancestor_capability(target, capability).or_else(|| {
            self.find_where(|info| info.provides.iter().any(|value| value == capability))
                .into_iter()
                .next()
        })
    }

    fn external_provider_rejected(
        root: &Registry,
        owner_id: NodeId,
        provider: &RegistrationSnapshot,
    ) -> Option<String> {
        let provider_path = root.path_for(provider.id)?;
        let owner = root.registry(owner_id)?;
        let owner_path = owner.path();
        // Why equality counts as external here: the provider may be the owner's
        // own parent face. `ancestor_providers` returns the face whose child
        // registry *is* the owner, and a child registry's path equals its owning
        // face's path, so the shared equality-inclusive `path_is_under` would
        // call that provider "internal" and skip the owner's admission gate. The
        // pre-B2 spelling `starts_with("{owner_path}/")` was false at equality,
        // so the gate stayed on; `path_is_strictly_under` keeps that meaning.
        // Pinned by `an_ancestor_provider_is_still_gated_by_the_owner_admission`.
        // 为什么相等在这里算外部：提供者可能是拥有者自己的父面。`ancestor_providers`
        // 返回的那个面，其子注册机**正是**拥有者，而子注册机的路径等于拥有它的面的路径，
        // 因此共享的“含相等”`path_is_under` 会把该提供者当作“内部”并跳过拥有者的准入
        // 检查。B2 之前的写法 `starts_with("{owner_path}/")` 在相等时为假，门禁因此仍
        // 生效；`path_is_strictly_under` 保留这一含义。
        // 由 `an_ancestor_provider_is_still_gated_by_the_owner_admission` 钉住。
        let is_external = !path_is_strictly_under(&provider_path, owner_path);
        if is_external && !owner.header.admission.accepts(&provider_path) {
            Some(provider_path)
        } else {
            None
        }
    }

    /// Return all connector failures in the staged tree as one error tree.
    /// 将暂存注册树中的全部连接器失败聚合成一棵错误树。
    pub(super) fn connector_error(&self) -> Option<RegistryError> {
        self.connector_error_with_external(None)
    }

    /// Validate connectors while allowing providers from an external overlay
    /// registry. Providers are read-only evidence; they are never copied into
    /// the effective tree.
    /// 在允许外部覆盖注册机提供者的情况下校验连接器。外部提供者只作为只读证据，
    /// 不会被复制进有效树。
    pub(super) fn connector_error_with_external(
        &self,
        external: Option<&Registry>,
    ) -> Option<RegistryError> {
        let failures = self.connector_errors_from(self, external);
        if failures.is_empty() {
            None
        } else {
            Some(
                RegistryError::new(
                    self.header.id,
                    self.header.path.clone(),
                    DiagnosticSource::from(SourceLocation {
                        file: "<registry-connector>",
                        line: 0,
                        column: 0,
                        function: "Registry::connector_error",
                    }),
                    format!(
                        "registration connector rejected ({} face(s))",
                        failures.len()
                    ),
                )
                .with_children(failures),
            )
        }
    }

    fn connector_errors_from(
        &self,
        root: &Registry,
        external: Option<&Registry>,
    ) -> Vec<RegistryError> {
        let mut errors = Vec::new();
        for entry in self.entries.values() {
            let path = format!("{}/{}", self.header.path, entry.info.registry_name);
            let mut failures = Vec::new();
            for requirement in &entry.info.requires {
                let provider = root.provider_for(
                    entry.info.parent,
                    &requirement.capability,
                    &requirement.provider,
                );
                let provider = match provider {
                    Ok(provider) => provider,
                    Err(providers) => {
                        let paths = providers
                            .into_iter()
                            .filter_map(|provider| root.path_for(provider.id))
                            .collect::<Vec<_>>()
                            .join(", ");
                        failures.push(RegistryError::new(
                            entry.info.id,
                            path.clone(),
                            entry.info.source.clone(),
                            format!(
                                "input `{}` cannot connect: provider kind `{}` is ambiguous at {}",
                                requirement.capability, requirement.provider, paths
                            ),
                        ));
                        continue;
                    }
                };
                let provider = if provider.is_none() {
                    external.and_then(|registry| {
                        registry
                            .provider_for(
                                entry.info.parent,
                                &requirement.capability,
                                &requirement.provider,
                            )
                            .ok()
                            .flatten()
                    })
                } else {
                    provider
                };
                if let Some(provider) = provider {
                    let provider_path = root
                        .path_for(provider.id)
                        .or_else(|| external.and_then(|registry| registry.path_for(provider.id)));
                    if let Some(provider_path) = provider_path.filter(|provider_path| {
                        let Some(owner) = root.registry(entry.info.parent) else {
                            return true;
                        };
                        let owner_path = owner.path();
                        // Same strict classification as `external_provider_rejected`:
                        // an ancestor provider with the owner's own path is still
                        // external, so the gate applies. See the comment there.
                        // 与 `external_provider_rejected` 相同的严格分类：路径等于拥有者
                        // 的祖先提供者仍算外部，门禁照常生效。理由见那处注释。
                        let is_external = !path_is_strictly_under(provider_path, owner_path);
                        is_external && !owner.header.admission.accepts(provider_path)
                    }) {
                        failures.push(RegistryError::new(
                            entry.info.id,
                            path.clone(),
                            entry.info.source.clone(),
                            format!(
                                "input `{}` cannot connect to `{provider_path}`: the parent Registry admission gate rejects that external branch",
                                requirement.capability,
                            ),
                        ));
                    }
                    continue;
                }
                if let Some(provider) = root
                    .provider_for_capability(entry.info.parent, &requirement.capability)
                    .or_else(|| {
                        external.and_then(|registry| {
                            registry
                                .provider_for_capability(entry.info.parent, &requirement.capability)
                        })
                    })
                    && let Some(provider_path) =
                        Self::external_provider_rejected(root, entry.info.parent, provider)
                {
                    failures.push(RegistryError::new(
                            entry.info.id,
                            path.clone(),
                            entry.info.source.clone(),
                            format!(
                                "input `{}` cannot connect to `{provider_path}`: admission rejects the branch and provider kind `{}` does not match expected `{}`",
                                requirement.capability, provider.kind, requirement.provider,
                            ),
                        ));
                    continue;
                }
                let detail = root
                    .ancestor_capability(entry.info.parent, &requirement.capability)
                    .or_else(|| {
                        root.find_where(|info| {
                            info.provides
                                .iter()
                                .any(|value| value == &requirement.capability)
                        })
                        .into_iter()
                        .next()
                    })
                    .map(|provider| {
                        let provider_path = root
                            .path_for(provider.id)
                            .or_else(|| {
                                external.and_then(|registry| registry.path_for(provider.id))
                            })
                            .unwrap_or_default();
                        format!(
                            "input `{}` found `{}` at `{}`, but its kind is `{}` instead of `{}`",
                            requirement.capability,
                            provider.registry_name,
                            provider_path,
                            provider.kind,
                            requirement.provider
                        )
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "input `{}` has no provider; expected provider kind `{}`",
                            requirement.capability, requirement.provider
                        )
                    });
                failures.push(RegistryError::new(
                    entry.info.id,
                    path.clone(),
                    entry.info.source.clone(),
                    format!("data-flow attachment failed: {detail}"),
                ));
            }
            if !failures.is_empty() {
                let mut error = RegistryError::new(
                    entry.info.id,
                    path,
                    entry.info.source.clone(),
                    format!(
                        "data-flow connector rejected `{}`; one or more declared inputs could not attach",
                        entry.info.kind
                    ),
                );
                *error.registration_chain_mut() = root.registration_chain(entry.info.id);
                *error.children_mut() = failures;
                errors.push(error);
            }
            if let Some(child) = entry.child.as_ref() {
                errors.extend(child.connector_errors_from(root, external));
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry_core::declaration::{
        Admission, FrameworkId, OwnedFlowContract, OwnedLocalizedText, OwnedObjectContract,
        OwnedRequirementSpec, OwnedSourceLocation, RegistrationRule,
    };
    use crate::registry_core::identity::root_node_id;

    /// One minimal owned face. Mirrors the transaction tests' builder so the
    /// connector test does not need a second production entry point.
    /// 一条最小的 owned 注册面。与 transaction 测试的构建器一致，连接器测试因此不需要
    /// 额外的生产入口。
    fn snapshot(
        namespace: &str,
        source: &str,
        kind: &str,
        registry_name: &str,
    ) -> RegistrationSnapshot {
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
            registry_name: registry_name.to_owned(),
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
                file: source.to_owned(),
                line: 1,
                column: 1,
                function: kind.to_owned(),
            },
        }
    }

    /// The audit's counterexample. Face `P` owns a registry, provides `cap`, and
    /// its admission allows only the unrelated path `allowed`; its child registry
    /// `RP` copies that admission; leaf `L` under `RP` requires `cap` from `P`.
    /// `P`'s path `root/p` is exactly `RP`'s path, the ancestor-provider case, so
    /// the connector must treat the provider as external and let the gate reject
    /// it — as the pre-B2 `starts_with("{owner_path}/")` spelling did. The
    /// equality-inclusive `path_is_under` would classify it as internal and drop
    /// the gate silently.
    /// 审计给出的反例。注册面 `P` 拥有一个注册机、提供 `cap`，其准入只允许无关路径
    /// `allowed`；它的子注册机 `RP` 复制该准入；`RP` 下的叶子 `L` 要求 `P` 提供
    /// `cap`。`P` 的路径 `root/p` 恰好就是 `RP` 的路径，即“祖先提供者”情形，因此连接器
    /// 必须把该提供者当作外部、让门禁拒绝它——B2 之前的 `starts_with("{owner_path}/")`
    /// 写法正是如此。含相等的 `path_is_under` 会把它当成内部并静默跳过门禁。
    #[test]
    fn an_ancestor_provider_is_still_gated_by_the_owner_admission() {
        let namespace = "connector-ancestor-provider";
        let mut registry = Registry::root_for_namespace(FrameworkId::new("test"), namespace);

        let mut owner = snapshot(namespace, "src/owner.rs", "Owner", "p");
        owner.needs_registry = true;
        owner.provides = vec!["cap".to_owned()];
        owner.admission = Admission::new(&["allowed"], &[]).into_owned();
        let owner_id = owner.id;
        let owner_kind = owner.kind.clone();
        registry
            .register_snapshot_batch([owner])
            .expect("the providing owner registers without requirements");

        // The fixture must actually exercise equality; otherwise the test would
        // pass for the wrong reason.
        // fixture 必须真的走到“相等”，否则测试会因为错误的原因通过。
        assert_eq!(registry.path_for(owner_id).as_deref(), Some("root/p"));
        assert_eq!(
            registry
                .registry(owner_id)
                .expect("the owner's child registry exists")
                .path(),
            "root/p"
        );

        let mut leaf = snapshot(namespace, "src/leaf.rs", "Leaf", "leaf");
        leaf.parent = owner_id;
        leaf.requires = vec![OwnedRequirementSpec {
            capability: "cap".to_owned(),
            provider: owner_kind,
        }];

        let error = registry
            .register_snapshot_batch([leaf])
            .expect_err("an ancestor provider equal to the owner's path must not be internal")
            .to_string();
        assert!(
            error.contains("admission gate rejects"),
            "the owner's gate must reject the provider at its own path: {error}"
        );
    }
}
