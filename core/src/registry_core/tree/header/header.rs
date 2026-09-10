//! Immutable Registry metadata.
//! Registry 不可变元数据。

use crate::registry_core::declaration::{OwnedAdmission, OwnedRegistrationRule};
use crate::registry_core::identity::NodeId;
use crate::plugin::FrameworkId;

#[derive(Clone, Debug)]
pub(super) struct RegistryHeader {
    pub(super) namespace: String,
    pub(super) framework: FrameworkId,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) id: NodeId,
    pub(super) registration_rule: OwnedRegistrationRule,
    pub(super) registration_rule_path: String,
    pub(super) admission: OwnedAdmission,
}

impl RegistryHeader {
    pub(super) fn admission(&self) -> &OwnedAdmission {
        &self.admission
    }
}
