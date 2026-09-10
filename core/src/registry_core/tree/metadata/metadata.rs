//! Registry metadata and counters.
//! 注册机元数据与计数。

use crate::registry_core::declaration::OwnedRegistrationRule;
use crate::registry_core::identity::NodeId;

use super::Registry;

impl Registry {
    pub fn namespace(&self) -> &str {
        &self.header.namespace
    }
    pub fn name(&self) -> &str {
        &self.header.name
    }
    pub fn path(&self) -> &str {
        &self.header.path
    }
    pub fn id(&self) -> NodeId {
        self.header.id
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn registration_rule(&self) -> &OwnedRegistrationRule {
        &self.header.registration_rule
    }
    pub fn dependency_admission_accepts(&self, path: &str) -> bool {
        self.header.admission.accepts(path)
    }
    pub fn allowed_dependency_paths(&self) -> &[String] {
        &self.header.admission.allowed_paths
    }

    pub fn registry_count(&self) -> usize {
        1 + self
            .entries
            .values()
            .filter_map(|entry| entry.child.as_ref())
            .map(|child| child.registry_count())
            .sum::<usize>()
    }

    pub fn node_count(&self) -> usize {
        self.entries
            .values()
            .map(|entry| 1 + entry.child.as_ref().map_or(0, |child| child.node_count()))
            .sum()
    }

    pub fn check_count(&self) -> usize {
        self.entries
            .values()
            .map(|entry| {
                entry.info.runtime_checks.len()
                    + entry.child.as_ref().map_or(0, |child| child.check_count())
            })
            .sum()
    }
}
