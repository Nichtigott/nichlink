//! Build-time identity helpers layered on the kernel identities.
//! 基于 kernel 身份的构建期身份辅助函数。

use std::sync::OnceLock;

pub use nichlink::registry_core::identity::NodeId;

/// Version of the identity input and persisted catalog formats.
/// 身份输入与持久化目录格式的版本。
pub const IDENTITY_SCHEMA: &str = "3";

/// Explicit namespace for standalone runs (CLI, tests). Cargo build scripts
/// read `CARGO_PKG_NAME` from the environment instead.
/// 独立运行（CLI、测试）使用的显式命名空间；Cargo build script 改从环境变量读取。
static PACKAGE_NAMESPACE_OVERRIDE: OnceLock<String> = OnceLock::new();

/// Pin the package namespace for the rest of the process. Used by
/// `run_for`, which executes outside Cargo and therefore has no
/// `CARGO_PKG_NAME` in its environment.
/// 为当前进程固定包命名空间。由 `run_for` 使用——它在 Cargo 之外执行，
/// 环境里没有 `CARGO_PKG_NAME`。
pub fn set_package_namespace(namespace: String) {
    let _ = PACKAGE_NAMESPACE_OVERRIDE.set(namespace);
}

/// Return the namespace of the package whose build script is currently running.
///
/// Cargo exposes the consuming package name to a build-script process. Using
/// that value keeps build-time identities byte-for-byte compatible with the
/// `env!("CARGO_PKG_NAME")` value captured by the declaration macros.
pub fn package_namespace() -> String {
    PACKAGE_NAMESPACE_OVERRIDE
        .get()
        .cloned()
        .unwrap_or_else(|| {
            std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "nichlink.default".to_owned())
        })
}

/// Compute the identity used by generated plans and caches.
pub fn package_node_id(relative_path: &str, declared_name: &str) -> NodeId {
    let namespace = package_namespace();
    NodeId::from_namespaced_path(&namespace, relative_path, declared_name)
}

/// Compute the root identity used by generated plans and caches.
pub fn package_root_node_id() -> NodeId {
    let namespace = package_namespace();
    NodeId::from_namespaced_path(&namespace, "<root>", "root")
}

#[cfg(test)]
mod tests {
    use super::{NodeId, package_namespace, package_node_id};

    #[test]
    fn package_identity_matches_the_macro_namespace_algorithm() {
        let namespace = package_namespace();
        let path = "control/object/button/button.rs";
        let kind = "Button";
        assert_eq!(
            package_node_id(path, kind),
            NodeId::from_namespaced_path(&namespace, path, kind)
        );
        assert_ne!(
            NodeId::from_namespaced_path("library-a", path, kind),
            NodeId::from_namespaced_path("library-b", path, kind)
        );
    }
}
