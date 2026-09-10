//! Pure name and path validation shared by file-backed authoring.
//! 文件创作共用的纯名称与路径校验。

use std::path::Path;

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
pub fn normalize_kind_name(kind: &str) -> String {
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

pub fn rust_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn normalized_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn rule_path_for_source(source: &str) -> String {
    let directory = Path::new(source).parent().unwrap_or_else(|| Path::new(""));
    // A registry's contract lives beside the face in a dedicated folder.
    // 注册规范与注册面并列，放在专门的 registry_rule 文件夹中。
    format!(
        "src/{}/registry_rule/registry_rule.rs",
        normalized_path(directory)
    )
}

pub fn is_parent_component(component: std::path::Component<'_>) -> bool {
    matches!(component, std::path::Component::ParentDir)
}
