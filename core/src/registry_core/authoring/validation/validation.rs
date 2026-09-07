//! Names and paths shared by file-backed authoring.
//! 文件创作共用的名称与路径校验。

use std::cell::RefCell;
use std::path::{Path, PathBuf};

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

pub(super) fn validate_name(name: &str) -> Result<(), String> {
    let valid = !name.is_empty()
        && name.chars().enumerate().all(|(index, character)| {
            character == '_'
                || character.is_ascii_lowercase()
                || character.is_ascii_digit() && index > 0
        });
    valid
        .then_some(())
        .ok_or_else(|| format!("invalid module name `{name}`; use snake_case ASCII"))
}

/// Validate a Rust type-like kind name used by a registration face.
/// 校验注册面使用的 Rust 类型风格 kind 名称。
pub(super) fn validate_kind_name(kind: &str) -> Result<(), String> {
    let mut chars = kind.chars();
    let valid = chars.next().is_some_and(|first| first.is_ascii_uppercase())
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_');
    valid
        .then_some(())
        .ok_or_else(|| format!("invalid kind `{kind}`; use a PascalCase ASCII identifier"))
}

/// Turn a common human-entered kind into a PascalCase Rust type name.
/// 将常见的人类输入 kind 规范成 PascalCase Rust 类型名。
pub(super) fn normalize_kind_name(kind: &str) -> String {
    if validate_kind_name(kind).is_ok() {
        return kind.to_owned();
    }
    kind.split(|character: char| !character.is_ascii_alphanumeric())
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

pub(super) fn rust_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn normalized_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub(super) fn rule_path_for_source(source: &str) -> String {
    let directory = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    // A registry's contract lives beside the face in a dedicated folder.
    // 注册规范与注册面并列，放在专门的 registry_rule 文件夹中。
    format!(
        "src/{}/registry_rule/registry_rule.rs",
        normalized_path(directory)
    )
}

/// Legacy layout kept readable so existing projects can migrate gradually.
/// 保留旧布局读取能力，现有项目可以逐步迁移。
pub(super) fn legacy_rule_path_for_source(source: &str) -> String {
    let directory = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    format!("src/{}/registry/rules/rules.rs", normalized_path(directory))
}

pub(super) fn is_parent_component(component: std::path::Component<'_>) -> bool {
    matches!(component, std::path::Component::ParentDir)
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
