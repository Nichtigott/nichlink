//! Build-time identity helpers layered on the kernel identities.
//! 基于 kernel 身份的构建期身份辅助函数。

use std::sync::OnceLock;

pub use nichlink::registry_core::identity::NodeId;

/// Version of the identity input and persisted catalog formats.
/// 身份输入与持久化目录格式的版本。
///
/// The kernel owns the value: a build that carried its own copy could accept a
/// `NICH_LINK_SCOPE` or a plugin lock the kernel rejects, or the other way
/// round, with no compiler noticing.
/// 该值由内核拥有：构建若自带一份副本，就可能接受内核拒绝的 `NICH_LINK_SCOPE`
/// 或插件锁（反之亦然），而编译器不会察觉。
pub use nichlink::registry_core::identity::IDENTITY_SCHEMA;

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

/// Pin a deterministic namespace for the whole test process.
/// 为整个测试进程固定一个确定的命名空间。
///
/// The override is first-write-wins. Freezing it at fixture creation means a
/// concurrent `run_for` test can no longer flip the namespace between another
/// test's rule-collection and rule-lookup scans, which previously made the
/// parent-rule lookup miss and rendered empty diagnostics.
/// 该覆盖是先到先得。在夹具创建时冻结后，并发的 `run_for` 测试就无法在
/// 另一个测试收集规则与查找规则之间翻转命名空间——那会导致父规则查找
/// 失配、诊断渲染为空。
#[cfg(test)]
pub(crate) fn freeze_test_namespace() {
    set_package_namespace("nichlink-build-method-tests".to_owned());
}

/// Return the namespace of the package whose build script is currently running.
/// 返回当前正在运行的构建脚本所属包的命名空间。
///
/// Cargo exposes the consuming package name to a build-script process. Using
/// that value keeps build-time identities byte-for-byte compatible with the
/// `env!("CARGO_PKG_NAME")` value captured by the declaration macros.
/// Cargo 把消费方包名暴露给构建脚本进程。使用该值让构建期身份与声明宏捕获的
/// `env!("CARGO_PKG_NAME")` 逐字节一致。
pub fn package_namespace() -> String {
    PACKAGE_NAMESPACE_OVERRIDE
        .get()
        .cloned()
        .unwrap_or_else(|| {
            std::env::var("CARGO_PKG_NAME").unwrap_or_else(|_| "nichlink.default".to_owned())
        })
}

/// Compute the identity used by generated plans and caches.
/// 计算生成计划与缓存所使用的身份。
pub fn package_node_id(relative_path: &str, declared_name: &str) -> NodeId {
    let namespace = package_namespace();
    NodeId::from_namespaced_path(&namespace, relative_path, declared_name)
}

/// Compute the root identity used by generated plans and caches.
/// 计算生成计划与缓存所使用的根身份。
pub fn package_root_node_id() -> NodeId {
    let namespace = package_namespace();
    NodeId::from_namespaced_path(&namespace, "<root>", "root")
}

#[cfg(test)]
mod tests {
    use super::{NodeId, freeze_test_namespace, package_namespace, package_node_id};

    #[test]
    fn package_identity_matches_the_macro_namespace_algorithm() {
        // Pin the namespace before reading it: the override is first-write-wins,
        // so a concurrent test pinning the test namespace between the two reads
        // below would make the comparison disagree with itself.
        // 先固定命名空间再读取：该覆盖是先到先得，若并发测试在这两次读取之间固定
        // 测试命名空间，比较就会与自身不一致。
        freeze_test_namespace();
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
