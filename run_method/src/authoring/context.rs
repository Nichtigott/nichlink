//! The authoring execution context: `AuthoringContext` and the process-level
//! paths and namespaces one file-bound authoring operation reads.
//! 创作执行上下文：`AuthoringContext`，以及一次绑定文件的创作操作所读取的进程级路径与命名空间。
//!
//! The file was named `validation` because the kernel's validators are re-exported
//! here — the validators themselves live in the kernel `authoring` module. What this
//! file *holds* is the context and its environment fallback chain, so that name led a
//! reader looking for validation rules to the wrong file; audit `NAM-03` asked for the
//! rename to `context`, and this is it. The historical public path
//! `nichlink_run_method::authoring::validation` is **not** gone: `authoring.rs` keeps it
//! as an alias (`pub use self::context as validation;`), and that alias is also how the
//! re-export pinned in `conventions/src/shims.rs` stays reachable at the old address.
//! 本文件曾名为 `validation`，是因为内核的校验函数在这里重导出——校验函数本体在 kernel 的
//! `authoring` 模块。本文件**装**的是上下文及其环境变量回落链，因此那个名字会把找校验规则的读者
//! 引到错的文件；审计 `NAM-03` 要的正是改成 `context`，这就是那次改名。历史公开路径
//! `nichlink_run_method::authoring::validation` **并没有消失**：`authoring.rs` 用别名
//! （`pub use self::context as validation;`）保留了它，钉在 `conventions/src/shims.rs` 里的那条
//! 重导出也正是靠它继续在旧地址上可达。

use std::cell::RefCell;
use std::path::{Path, PathBuf};

pub use nichlink_kernel::authoring::validation::*;

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
    /// Pair a package root with the namespace its faces are authored under.
    /// 把一个包根与其注册面创作所用的命名空间配对。
    ///
    /// The values are inert until [`AuthoringContext::scope`] installs them, so
    /// constructing one never touches process-global state.
    /// 这些取值在 [`AuthoringContext::scope`] 安装之前不生效，因此构造本身不会触碰
    /// 进程级状态。
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

/// The namespace this operation authors under: the active context first, then
/// the environment, then the documented default.
/// 本次操作创作所用的命名空间：先活动上下文，再环境变量，最后文档化的默认值。
///
/// The decision itself is `nichlink_kernel::lexicon::resolve_namespace`, shared with
/// Studio and the MCP bridge; this function only supplies what it reads.
/// 决策本身是 `nichlink_kernel::lexicon::resolve_namespace`，与 Studio 和 MCP 桥共用；本函数
/// 只负责提供它读取的东西。
pub(super) fn authoring_namespace() -> String {
    ACTIVE_CONTEXT
        .with(|active| {
            active
                .borrow()
                .as_ref()
                .map(|context| context.namespace.clone())
        })
        .unwrap_or_else(|| {
            nichlink_kernel::lexicon::resolve_namespace(
                std::env::var(nichlink_kernel::lexicon::NAMESPACE_ENV)
                    .ok()
                    .as_deref(),
            )
            .to_owned()
        })
}

/// Legacy layout kept readable so existing projects can migrate gradually.
/// 保留旧布局读取能力，现有项目可以逐步迁移。
pub(super) fn legacy_rule_path_for_source(source: &str) -> String {
    let directory = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    format!("src/{}/registry/rules/rules.rs", normalized_path(directory))
}

/// The package this operation writes into: the active context first, then the
/// shared `lexicon` rule over the environment and the working directory.
/// 本次操作写入的包：先活动上下文，再对环境和当前目录套用共用的 `lexicon` 规则。
pub(super) fn package_root() -> PathBuf {
    if let Some(root) = ACTIVE_CONTEXT.with(|active| {
        active
            .borrow()
            .as_ref()
            .map(|context| context.package_root.clone())
    }) {
        return root;
    }
    let configured =
        std::env::var_os(nichlink_kernel::lexicon::PACKAGE_ROOT_ENV).map(PathBuf::from);
    let current = std::env::current_dir().ok();
    nichlink_kernel::lexicon::resolve_package_root(
        configured.as_deref(),
        current.as_deref(),
        current
            .as_ref()
            .is_some_and(|directory| directory.join("Cargo.toml").is_file()),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
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
