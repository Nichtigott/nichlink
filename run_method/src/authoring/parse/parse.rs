//! Parsing and rendering helpers for authored registration faces.
//! 注册面创作文件的解析与渲染辅助函数。
//!
//! The pure source-text transformations live in the kernel `authoring`
//! module; this shim keeps the historical
//! `nichlink_run_method::authoring::parse` path and re-implements the two
//! filesystem-bound entry points as thin wrappers.
//! 纯源码文本变换位于 kernel 的 `authoring` 模块；本 shim 保留
//! `nichlink_run_method::authoring::parse` 历史路径，并把两个绑定文件
//! 系统的入口重新实现为薄包装。

use std::fs;
use std::path::{Path, PathBuf};

pub use nichlink::authoring::parse::*;

use super::validation::{normalized_path, source_root};

/// Read the parent's declared `kind` from its attached source file.
/// 从父注册面的附属源文件读取它声明的 `kind`。
pub(super) fn kind_from_source_path(source: &str) -> Option<String> {
    let path = source_root().join(source);
    let text = fs::read_to_string(path).ok()?;
    nichlink::authoring::parse::kind_from_source_text(&text)
}

pub(super) fn source_path_from_file(path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(source_root()) {
        return normalized_path(relative);
    }
    let roots = ["compile_error_demo", "control", "engine", "trimmed_core"];
    let mut components = path.components();
    while let Some(component) = components.next() {
        let text = component.as_os_str().to_string_lossy();
        if roots.contains(&text.as_ref()) {
            let mut result = PathBuf::from(text.as_ref());
            result.extend(components.map(|part| part.as_os_str()));
            return normalized_path(&result);
        }
    }
    normalized_path(path)
}

pub(super) fn rule_syntax_for_source(path: &Path) -> Result<String, String> {
    let Some(parent) = path.parent() else {
        return Ok("ANY".to_owned());
    };
    let canonical = parent.join("registry_rule/registry_rule.rs");
    let legacy = parent.join("registry/rules/rules.rs");
    let Ok(text) = fs::read_to_string(&canonical).or_else(|_| fs::read_to_string(&legacy)) else {
        return Ok("ANY".to_owned());
    };
    Ok(nichlink::authoring::parse::rule_syntax_from_text(&text))
}
