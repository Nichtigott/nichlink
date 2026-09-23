//! Immutable Registry metadata.
//! Registry 不可变元数据。
//!
//! Every field here is read somewhere in the tree; the struct used to carry a
//! `name` that only `reconfigure` ever wrote and an `admission()` accessor that
//! nobody called, both of which were kept quiet by a module-level
//! `#[allow(dead_code)]`. They were removed rather than allowed: the workspace
//! deletes a zero-caller item and lets it return when a caller appears.
//! 这里每个字段都在树的某处被读取；本结构曾带一个只有 `reconfigure` 会写、从不被读的
//! `name`，以及一个无人调用的 `admission()` 访问器，两者都被模块级
//! `#[allow(dead_code)]` 压住了。它们被删除而不是被允许保留：本工作区删除零调用者的项，
//! 等真实调用者出现时再让它回来。

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
