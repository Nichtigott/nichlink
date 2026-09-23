//! Pure name and path validation shared by file-backed authoring.
//! 文件创作共用的纯名称与路径校验。

use std::path::Path;

/// Accept a snake_case ASCII module name, or explain what is wrong with it.
/// 接受 snake_case ASCII 模块名，否则说明它的问题所在。
pub fn validate_name(name: &str) -> Result<(), String> {
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
pub fn validate_kind_name(kind: &str) -> Result<(), String> {
    let mut chars = kind.chars();
    let valid = chars.next().is_some_and(|first| first.is_ascii_uppercase())
        && chars.all(|character| character.is_ascii_alphanumeric() || character == '_');
    valid
        .then_some(())
        .ok_or_else(|| format!("invalid kind `{kind}`; use a PascalCase ASCII identifier"))
}

/// Turn a common human-entered kind into a PascalCase Rust type name.
/// 将常见的人类输入 kind 规范成 PascalCase Rust 类型名。
///
/// The fallback used to be this function's own copy of the splitting and
/// capitalizing loop, byte-for-byte the body of [`super::pascal_case`]. The two
/// were free to drift — a non-ASCII separator handled in one and not the other —
/// and the already-valid branch is what keeps a name like `Control_Handle`
/// intact instead of collapsing it to `ControlHandle`. The fallback is now the
/// shared function; `normalizing_a_kind_uses_the_shared_pascal_case_fallback`
/// pins both branches.
/// 回退分支过去是本函数自己的一份切分与首字母大写循环，与 [`super::pascal_case`]
/// 的函数体逐字节相同。两者可以各自漂移——例如某一侧处理了非 ASCII 分隔符而另一侧
/// 没有——而已合法分支正是让 `Control_Handle` 保持原样、不被压成 `ControlHandle`
/// 的那一支。现在回退直接使用共享函数；
/// `normalizing_a_kind_uses_the_shared_pascal_case_fallback` 钉住两个分支。
pub fn normalize_kind_name(kind: &str) -> String {
    if validate_kind_name(kind).is_ok() {
        return kind.to_owned();
    }
    super::pascal_case(kind)
}

/// Escape a value so it can be embedded in a Rust string literal.
/// 转义取值，使其能嵌入 Rust 字符串字面量。
pub fn rust_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Render a path with `/` separators, independent of the host platform.
/// 以 `/` 分隔符渲染路径，与宿主平台无关。
pub fn normalized_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// Derive the canonical rule-file path that sits beside a face's source file.
/// 由注册面源文件推导其旁规范的规则文件路径。
pub fn rule_path_for_source(source: &str) -> String {
    let directory = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    // A registry's contract lives beside the face in a dedicated folder.
    // 注册规范与注册面并列，放在专门的 registry_rule 文件夹中。
    format!(
        "src/{}/registry_rule/registry_rule.rs",
        normalized_path(directory)
    )
}

/// Whether a path component is a `..` step.
/// 判断路径分量是否为 `..`。
pub fn is_parent_component(component: std::path::Component<'_>) -> bool {
    matches!(component, std::path::Component::ParentDir)
}

#[cfg(test)]
mod tests {
    use super::normalize_kind_name;
    use crate::registry_core::authoring::pascal_case;

    /// A kind that already is a PascalCase ident must come back untouched, and
    /// anything else must go through the shared `pascal_case` fallback; neither
    /// branch may keep a private copy of the capitalizing loop.
    /// 已经是 PascalCase 标识符的 kind 必须原样返回，其余一律走共享的 `pascal_case`
    /// 回退；两个分支都不得再持有独立的大写循环副本。
    #[test]
    fn normalizing_a_kind_uses_the_shared_pascal_case_fallback() {
        assert_eq!(normalize_kind_name("Button"), "Button");
        assert_eq!(normalize_kind_name("Control_Handle"), "Control_Handle");
        assert_eq!(normalize_kind_name("some-kind"), "SomeKind");
        assert_eq!(normalize_kind_name("some_kind"), "SomeKind");
        assert_eq!(normalize_kind_name("a.b"), "AB");
        assert_eq!(normalize_kind_name("9lives"), "9lives");
        assert_eq!(normalize_kind_name(""), "");
        // The valid branch is the reason `Control_Handle` survives: the fallback
        // would drop the separator.
        // 合法分支正是 `Control_Handle` 得以保留的原因：回退会丢掉分隔符。
        assert_eq!(pascal_case("Control_Handle"), "ControlHandle");
        assert_ne!(normalize_kind_name("Control_Handle"), "ControlHandle");
    }
}
