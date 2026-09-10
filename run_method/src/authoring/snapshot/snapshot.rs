//! Conversion from authored fields to a registration snapshot.
//! 将创作字段转换成注册快照。
//!
//! The pure conversion lives in the kernel `authoring` module; this shim
//! keeps the `FaceManifest::to_snapshot` method and injects the
//! process-bound namespace fallback.
//! 纯转换位于 kernel 的 `authoring` 模块；本 shim 保留
//! `FaceManifest::to_snapshot` 方法，并注入绑定进程状态的命名空间回落值。

use super::manifest::FaceManifest;
use super::validation::authoring_namespace;
use crate::RegistrationSnapshot;

impl FaceManifest {
    pub(crate) fn to_snapshot(&self) -> Result<RegistrationSnapshot, String> {
        nichlink::authoring::snapshot::snapshot_from_values(&self.values, &authoring_namespace())
    }
}
