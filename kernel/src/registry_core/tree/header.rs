//! Immutable Registry metadata.
//! Registry 不可变元数据。
//!
//! Every field here is read somewhere in the tree. That is the rule for this struct:
//! a field with no reader is deleted rather than kept quiet by a module-level
//! `#[allow(dead_code)]`, and it returns when a reader appears.
//! 这里每个字段都在树的某处被读取。本结构遵守的规则就是这一条：没有读取者的字段会被删除，
//! 而不是用模块级 `#[allow(dead_code)]` 压住；等读取者出现时它再回来。

use crate::registry_core::declaration::FrameworkId;
use crate::registry_core::declaration::{OwnedAdmission, OwnedRegistrationRule};
use crate::registry_core::identity::NodeId;

#[derive(Clone, Debug)]
pub(super) struct RegistryHeader {
    pub(super) namespace: String,
    pub(super) framework: FrameworkId,
    pub(super) path: String,
    pub(super) id: NodeId,
    pub(super) registration_rule: OwnedRegistrationRule,
    pub(super) registration_rule_path: String,
    pub(super) admission: OwnedAdmission,
}
