//! Generate the passive crate entry from the folder layout.
//! 根据文件夹布局生成被动 crate 入口。
//!
//! The build script mirrors the local folder layout and derives a conservative
//! registration-face scope before rustc sees the generated module tree. Distributed
//! registration is routed through the collector adapter; this file does not
//! maintain a registration roster.
//! build.rs 镜像本地文件夹布局，并在 rustc 看到生成模块树之前保守推导注册面范围。
//! 分布式注册经由 collector 适配层完成，本文件不维护注册清单。
//!
//! The implementation lives in sibling modules; this file only mounts them,
//! re-exports the surface, and forwards the two entry points.
//! 实现位于同级模块；本文件只负责挂载它们、重新导出对外表面，并转发两个入口。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。
#![warn(missing_docs)]

use std::path::Path;

#[path = "build_input.rs"]
mod build_input;
#[path = "cache.rs"]
mod cache;
#[path = "contracts.rs"]
mod contracts;
#[path = "diagnostics.rs"]
mod diagnostics;
#[path = "discovery.rs"]
mod discovery;
#[path = "entry.rs"]
mod entry;
#[path = "entry_default.rs"]
mod entry_default;
#[path = "face_view.rs"]
pub mod face_view;
#[path = "faces.rs"]
mod faces;
#[path = "graft_plan_check.rs"]
mod graft_plan_check;
#[path = "graft_view.rs"]
mod graft_view;
#[path = "identity_cache.rs"]
mod identity_cache;
#[path = "manifests.rs"]
mod manifests;
#[path = "node.rs"]
mod node;
#[path = "node_id.rs"]
mod node_id;
#[path = "package.rs"]
mod package;
#[path = "pipeline.rs"]
mod pipeline;
#[path = "registration_check.rs"]
mod registration_check;
#[path = "identity.rs"]
mod registry_identity;
#[path = "syntax.rs"]
mod registry_syntax;
#[path = "renderer.rs"]
mod renderer;
#[path = "scaffold.rs"]
pub mod scaffold;
#[path = "scope.rs"]
mod scope;
#[path = "source_layout.rs"]
mod source_layout;
#[path = "static_plan.rs"]
mod static_plan;
#[path = "validation.rs"]
mod validation;

// Public surface, reachable at exactly the paths callers already use.
// 对外表面，保持在调用方已经在用的路径上。
pub use entry::host_entry_source;
pub use face_view::{
    BuildScopeView, FaceView, PruningRow, build_output_is_current, face_views, read_build_scope,
    read_pruning_manifest,
};
pub use graft_view::{DeclaredGraft, DeclaredGraftExpressions, DeclaredGrafts, declared_grafts};
/// The package name Cargo reports for a package root, which is the identity
/// namespace of every face that package compiles.
/// Cargo 为某个包根报告的包名，也就是该包编译的每个面的身份命名空间。
pub use package::package_name;
/// The module path a registration source declares, for authoring surfaces.
/// 注册面源码声明的模块路径，供创作界面使用。
pub use static_plan::source_module_path;

// Crate-internal helpers, re-exported so sibling modules keep their `super::…`
// paths after the split instead of reaching into each other's new homes.
// crate 内部辅助项，重新导出以便同级模块在拆分后保留 `super::…` 路径，
// 不必互相伸进对方的新家。
pub(crate) use build_input::BuildInput;
pub(crate) use cache::{
    CACHE_SCHEMA, cached_parent_id, collect_active_ids, face_source_is_active, module_feature,
    source_is_active, update_discovery_cache, write_if_changed,
};
pub(crate) use contracts::aggregate_contract_errors;
pub(crate) use discovery::{
    discover_root, discover_root_reporting, discovery_fingerprint, emit_rerun_paths,
};
pub use source_layout::{SourceLayout, source_layout};
// `resolve_host_entry` is deliberately absent: production code reaches it only
// through `host_entry_from_environment`, and re-exporting it for tests alone
// would be an unused import in a non-test build.
// 这里刻意不导出 `resolve_host_entry`：生产代码只经 `host_entry_from_environment`
// 到达它，仅为测试而导出会在非测试构建里成为未使用导入。
pub(crate) use entry::{HostEntry, host_entry_from_environment};
pub(crate) use entry_default::default_entry_source;
pub(crate) use faces::{FaceSource, collect_faces, has_selected_face};
pub(crate) use graft_view::{declared_graft_view, host_graft_entries};
pub(crate) use identity_cache::{cache_directory, prime_node_id_cache};
pub(crate) use manifests::{
    write_function_manifest, write_graft_manifest, write_pruning_manifest,
    write_source_scope_manifest,
};
pub(crate) use node::{Node, relative_display};
pub(crate) use node_id::{CACHED_NODE_IDS, node_id};
pub(crate) use renderer::render_lib;
pub(crate) use scope::SourceScope;
pub(crate) use static_plan::static_plan;
pub(crate) use validation::{
    aggregate_parent_macro_errors, aggregate_requirements, aggregate_stable_name_errors,
    face_syntax_errors, parsed_face, unplaced_face_errors,
};

/// Run the build-time discovery and validation pipeline from a Cargo build
/// script.
/// 从 Cargo build script 运行构建期发现与校验管线。
pub fn run() {
    let input = BuildInput::from_environment();
    pipeline::run(&input);
}

/// Run the discovery and validation pipeline for an explicit host project,
/// outside a Cargo build script. `manifest` is the package **root directory** —
/// the one holding `Cargo.toml`, not the file — because every path this builds on
/// (`src/`, the cache, the generated output) is resolved against it. `package`
/// pins the identity namespace that Cargo would otherwise inject through
/// `CARGO_PKG_NAME`. `out_dir` receives the generated plan and manifests; when
/// validation fails the rendered diagnostics are returned.
/// 在 Cargo build script 之外为显式指定的宿主项目运行发现与校验管线。`manifest` 是包的
/// **根目录**——装着 `Cargo.toml` 的那个目录，而不是文件本身——因为本管线搭出的每条路径
/// （`src/`、缓存、生成产物）都以它为基准。`package` 固定身份命名空间（Cargo 本来会通过
/// `CARGO_PKG_NAME` 注入）。`out_dir` 接收生成的计划与清单；校验失败时返回渲染后的诊断。
pub fn run_for(manifest: &Path, out_dir: &Path, package: &str) -> Result<(), String> {
    // Keep the historical plain-text IO error: `check_for` reports an unwritable
    // `out_dir` as a diagnostic so its structured caller sees it too, but a
    // build-script-era caller must still get `create <dir>: <reason>`.
    // 保留历史上的纯文本 IO 错误：`check_for` 把不可写的 `out_dir` 也报成一条诊断，
    // 让结构化调用方同样看得见；而构建脚本时代的调用方仍应拿到
    // `create <dir>: <reason>`。
    std::fs::create_dir_all(out_dir)
        .map_err(|error| format!("create {}: {error}", out_dir.display()))?;
    check_for(manifest, out_dir, package).map_err(|diagnostics| diagnostics.render())
}

/// Run the same discovery and validation pipeline as [`run_for`], but hand the
/// caller the structured [`nichlink::BuildDiagnostics`] instead of the text a
/// terminal or `compile_error!` reads.
/// 运行与 [`run_for`] 相同的发现与校验管线，但把结构化的
/// [`nichlink::BuildDiagnostics`] 交给调用方，而不是终端或 `compile_error!` 读的文本。
///
/// This is the machine-readable twin of `run_for`: a CI job or `nichlink check
/// --json` needs to count and serialize individual failures, and re-parsing the
/// rendered frame would make the layout part of the contract. The rendered text
/// stays byte-identical because `run_for` renders exactly this value with the
/// unchanged `render()`.
/// 这是 `run_for` 的机器可读孪生：CI 或 `nichlink check --json` 需要逐条计数并序列化
/// 失败，而重新解析渲染文本会让版式变成契约。渲染文本保持逐字节一致，因为 `run_for`
/// 用未改动的 `render()` 渲染的正是这个值。
pub fn check_for(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
) -> Result<(), nichlink::BuildDiagnostics> {
    std::fs::create_dir_all(out_dir).map_err(|error| {
        let mut diagnostics = nichlink::BuildDiagnostics::default();
        diagnostics.push(nichlink::BuildDiagnostic::new(
            "out-dir",
            format!("create {}: {error}", out_dir.display()),
        ));
        diagnostics
    })?;
    registry_identity::set_package_namespace(package.to_owned());
    let input = BuildInput::new(manifest.to_path_buf(), out_dir.to_path_buf(), false);
    match pipeline::run(&input) {
        Some(diagnostics) => Err(diagnostics),
        None => Ok(()),
    }
}
