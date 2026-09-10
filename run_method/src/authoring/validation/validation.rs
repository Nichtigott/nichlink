//! Names and paths shared by file-backed authoring.
//! 文件创作共用的名称与路径校验。
//!
//! The pure validators live in the kernel `authoring` module; this shim
//! keeps the `AuthoringContext` and its environment fallback chain, and
//! re-exports the kernel helpers so the historical
//! `nichlink_run_method::authoring::validation` paths keep working.
//! 纯校验函数位于 kernel 的 `authoring` 模块；本 shim 保留
//! `AuthoringContext` 及其环境变量回落链，并重导出 kernel 辅助函数，
//! 保证 `nichlink_run_method::authoring::validation` 历史路径继续可用。

use std::cell::RefCell;
use std::path::{Path, PathBuf};

pub use nichlink::authoring::validation::*;

/// Filesystem and identity scope for one authoring operation.
/// 单次注册面创作操作使用的文件系统与身份上下文。
#[derive(Clone, Debug)]
pub struct AuthoringContext {
    package_root: PathBuf,
    namespace: String,
}

thread_local! {
    static ACTIVE_CONTEXT: RefCell<Option<AuthoringContext>> = const { RefCell::new(None) };
}

struct ContextRestore(Option<AuthoringContext>);

impl Drop for ContextRestore {
    fn drop(&mut self) {
        ACTIVE_CONTEXT.with(|active| {
            *active.borrow_mut() = self.0.take();
        });
    }
}

impl AuthoringContext {
    pub fn new(package_root: impl Into<PathBuf>, namespace: impl Into<String>) -> Self {
        Self {
            package_root: package_root.into(),
            namespace: namespace.into(),
        }
    }

    /// Run an operation in this context without changing process environment.
    /// 在此上下文中执行操作，不修改进程环境变量。
    pub fn scope<T>(&self, operation: impl FnOnce() -> T) -> T {
        let previous = ACTIVE_CONTEXT.with(|active| active.replace(Some(self.clone())));
        let _restore = ContextRestore(previous);
        operation()
    }
}

pub(super) fn authoring_namespace() -> String {
    ACTIVE_CONTEXT
        .with(|active| {
            active
                .borrow()
                .as_ref()
                .map(|context| context.namespace.clone())
        })
        .or_else(|| std::env::var("NICH_LINK_NAMESPACE").ok())
        .unwrap_or_else(|| "nichlink.default".to_owned())
}

pub(super) fn rust_type_name(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
                .unwrap_or_default()
        })
        .collect()
}

/// Legacy layout kept readable so existing projects can migrate gradually.
/// 保留旧布局读取能力，现有项目可以逐步迁移。
pub(super) fn legacy_rule_path_for_source(source: &str) -> String {
    let directory = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    format!("src/{}/registry/rules/rules.rs", normalized_path(directory))
}

pub(super) fn package_root() -> PathBuf {
    if let Some(root) = ACTIVE_CONTEXT.with(|active| {
        active
            .borrow()
            .as_ref()
            .map(|context| context.package_root.clone())
    }) {
        return root;
    }
    if let Some(configured) = std::env::var_os("NICH_LINK_PACKAGE_ROOT") {
        let path = PathBuf::from(configured);
        return if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(path)
        };
    }
    if let Ok(current) = std::env::current_dir()
        && current.join("Cargo.toml").is_file()
    {
        return current;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub(super) fn source_root() -> PathBuf {
    package_root().join("src")
}

#[cfg(test)]
mod tests {
    use super::{AuthoringContext, authoring_namespace, package_root};
    use std::path::Path;

    #[test]
    fn nested_authoring_contexts_restore_the_previous_project() {
        let outer = AuthoringContext::new("/tmp/nichlink-outer", "outer");
        let inner = AuthoringContext::new("/tmp/nichlink-inner", "inner");

        outer.scope(|| {
            assert_eq!(package_root(), Path::new("/tmp/nichlink-outer"));
            assert_eq!(authoring_namespace(), "outer");
            inner.scope(|| {
                assert_eq!(package_root(), Path::new("/tmp/nichlink-inner"));
                assert_eq!(authoring_namespace(), "inner");
            });
            assert_eq!(package_root(), Path::new("/tmp/nichlink-outer"));
            assert_eq!(authoring_namespace(), "outer");
        });
    }

    #[test]
    fn panicking_authoring_context_still_restores_its_parent() {
        let outer = AuthoringContext::new("/tmp/nichlink-outer", "outer");
        let inner = AuthoringContext::new("/tmp/nichlink-inner", "inner");

        outer.scope(|| {
            let result = std::panic::catch_unwind(|| inner.scope(|| panic!("expected panic")));
            assert!(result.is_err());
            assert_eq!(package_root(), Path::new("/tmp/nichlink-outer"));
            assert_eq!(authoring_namespace(), "outer");
        });
    }
}
