//! Source indexing and registry snapshot helpers for Studio.
//! Studio 的源码索引与注册快照辅助逻辑。
//!
//! The pure Rust-source lexer lives in the kernel `source` module; this shim
//! keeps the historical paths and hosts the Studio-specific file-bound helpers.
//! 纯 Rust 源码词法器本体在 kernel 的 `source` 模块；本 shim 保留历史路径，
//! 并承载 Studio 特有的文件绑定辅助逻辑。

// Kernel lexer re-exports. Historical Studio paths stay valid through these.
// kernel 词法器重导出，Studio 历史路径经由它们保持可用。
pub use nichlink_run_method::source::{body_calls, function_source_range, function_symbols};

use super::support::{package_namespace, package_root, with_authoring_context};
use super::*;

/// Compose compiled, external, and newly authored faces into one snapshot.
/// 将已编译、外部和刚落盘的注册面装配成同一个快照。
pub(super) fn load_registry() -> Result<Registry, String> {
    // Studio owns an empty, namespace-isolated root and lets the authoring UI
    // add faces. A host can choose a stable namespace per library through the
    // environment when several libraries share one process.
    let namespace = package_namespace();
    let mut registry = Registry::root_for_namespace(
        nichlink_run_method::FrameworkId::new("nichlink.studio"),
        namespace,
    );
    // Scan only the host package's `src/` tree. When Studio is launched from
    // the NichLink workspace itself there is no host `src/`; show an empty
    // registry instead of mistaking build/debug fixtures for faces.
    // 只扫描宿主包的 `src/`。直接从 NichLink workspace 启动时没有宿主 `src/`，
    // 此时显示空注册树，不要把 build/debug 测试夹具误认成注册面。
    let source_root = package_root().join("src");
    if !source_root.is_dir() {
        return Ok(registry);
    }
    let snapshots =
        with_authoring_context(|| nichlink_run_method::generated_snapshots_from(&source_root))
            .map_err(|error| format!("cannot load generated registration faces: {error}"))?;
    registry
        .register_snapshot_batch(snapshots)
        .map_err(|error| format!("generated registration faces were rejected: {error}"))?;
    Ok(registry)
}

/// Render one admission policy as a compact single-line summary.
/// 将一条 admission 策略渲染成紧凑的单行摘要。
pub(crate) fn admission_text(admission: &nichlink_run_method::OwnedAdmission) -> String {
    if admission.allowed_paths.is_empty() && admission.denied_paths.is_empty() {
        return "ANY".to_owned();
    }
    if !admission.allowed_paths.is_empty() {
        return format!("allow:{}", admission.allowed_paths.join(","));
    }
    format!("deny:{}", admission.denied_paths.join(","))
}

/// Render one registry rule as compact `preset:...;parts:...` clauses.
/// 将一条注册规则渲染成紧凑的 `preset:...;parts:...` 子句。
pub(crate) fn registration_rule_text(rule: &nichlink_run_method::OwnedRegistrationRule) -> String {
    let mut clauses = Vec::new();
    if let Some(preset) = &rule.required_preset {
        clauses.push(format!("preset:{preset}"));
    }
    if !rule.required_parts.is_empty() {
        clauses.push(format!("parts:{}", rule.required_parts.join(",")));
    }
    if !rule.required_exports.is_empty() {
        clauses.push(format!("exports:{}", rule.required_exports.join(",")));
    }
    if !rule.required_handle_traits.is_empty() {
        clauses.push(format!("handle:{}", rule.required_handle_traits.join(",")));
    }
    if !rule.required_part_traits.is_empty() {
        clauses.push(format!(
            "part_trait:{}",
            rule.required_part_traits.join(",")
        ));
    }
    if clauses.is_empty() {
        "ANY".to_owned()
    } else {
        clauses.join(";")
    }
}

/// Locate the 1-based declaration line of a function in a registry source file.
/// 在注册面源码文件中定位函数声明的 1 起始行号。
pub(crate) fn function_line(file: &str, function: &str) -> Option<u32> {
    let text = std::fs::read_to_string(source_path_for(file)).ok()?;
    function_symbols(&text)
        .into_iter()
        .find(|item| item.name == function)
        .map(|item| item.line)
}

#[cfg(test)]
pub(super) fn function_bodies(source: &str) -> Vec<(String, String)> {
    function_symbols(source)
        .into_iter()
        .map(|function| (function.name, function.body))
        .collect()
}
