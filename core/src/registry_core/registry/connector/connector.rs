//! Registration connector and dependency admission checks.
//! 注册连接器与依赖准入检查。

use super::*;

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
        let is_external =
            provider_path != owner_path && !provider_path.starts_with(&format!("{owner_path}/"));
        if is_external && !owner.header.admission.accepts(&provider_path) {
            Some(provider_path)
        } else {
            None
        }
    }

    /// Return all connector failures in the staged tree as one error tree.
    /// 将暂存注册树中的全部连接器失败聚合成一棵错误树。
    pub(super) fn connector_error(&self) -> Option<RegistryError> {
        let failures = self.connector_errors_from(self);
        if failures.is_empty() {
            None
        } else {
            Some(RegistryError {
                node: self.header.id,
                path: self.header.path.clone(),
                source: DiagnosticSource::from(SourceLocation {
                    file: "<registry-connector>",
                    line: 0,
                    column: 0,
                    function: "Registry::connector_error",
                }),
                message: format!(
                    "registration connector rejected ({} face(s))",
                    failures.len()
                ),
                source_chain: Vec::new(),
                call_path: Vec::new(),
                registration_chain: Vec::new(),
                children: failures,
            })
        }
    }

    fn connector_errors_from(&self, root: &Registry) -> Vec<RegistryError> {
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
                                "requirement `{}` is ambiguous; provider kind `{}` appears at: {}",
                                requirement.capability, requirement.provider, paths
                            ),
                        ));
                        continue;
                    }
                };
                if let Some(provider) = provider {
                    if let Some(provider_path) =
                        Self::external_provider_rejected(root, entry.info.parent, provider)
                    {
                        failures.push(RegistryError::new(
                            entry.info.id,
                            path.clone(),
                            entry.info.source.clone(),
                            format!(
                                "dependency admission rejected external provider `{provider_path}` for requirement `{}`",
                                requirement.capability,
                            ),
                        ));
                    }
                    continue;
                }
                if let Some(provider) =
                    root.provider_for_capability(entry.info.parent, &requirement.capability)
                    && let Some(provider_path) =
                        Self::external_provider_rejected(root, entry.info.parent, provider)
                {
                    failures.push(RegistryError::new(
                            entry.info.id,
                            path.clone(),
                            entry.info.source.clone(),
                            format!(
                                "dependency admission rejected external provider `{provider_path}` for requirement `{}` (provider kind `{}`, expected `{}`)",
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
                        let provider_path = root.path_for(provider.id).unwrap_or_default();
                        format!(
                            "capability `{}` is provided by `{}` at `{}` (kind `{}`), expected kind `{}`",
                            requirement.capability,
                            provider.registry_name,
                            provider_path,
                            provider.kind,
                            requirement.provider
                        )
                    })
                    .unwrap_or_else(|| {
                        format!(
                            "capability `{}` is missing; expected provider kind `{}`",
                            requirement.capability, requirement.provider
                        )
                    });
                failures.push(RegistryError::new(
                    entry.info.id,
                    path.clone(),
                    entry.info.source.clone(),
                    format!("requirement `{}` failed: {detail}", requirement.capability),
                ));
            }
            if !failures.is_empty() {
                let mut error = RegistryError::new(
                    entry.info.id,
                    path,
                    entry.info.source.clone(),
                    format!(
                        "registration connector rejected `{}` (registration rule validation; rule `{}`)",
                        entry.info.kind, entry.info.registry_rule_path
                    ),
                );
                error.registration_chain = root.registration_chain(entry.info.id);
                error.children = failures;
                errors.push(error);
            }
            if let Some(child) = entry.child.as_ref() {
                errors.extend(child.connector_errors_from(root));
            }
        }
        errors
    }
}
