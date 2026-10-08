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

use std::path::Path;

#[path = "build_input.rs"]
mod build_input;
#[path = "contracts.rs"]
mod contracts;

#[path = "crate_plan.rs"]
pub(crate) mod crate_plan;

// Its only caller is the authoring CLI face (`nichlink crates`), so the module rides that feature:
// a default build has no writer, and dead code is a warning this workspace refuses.
// 它唯一的调用方是创作面的 CLI（`nichlink crates`），因此该模块随那个特性走：默认构建没有写入方，而死代码是
// 本工作区拒绝的告警。
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "crate_facade.rs"]
pub(crate) mod crate_facade;

#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "crate_members.rs"]
pub(crate) mod crate_members;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "crate_release.rs"]
pub(crate) mod crate_release;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "crate_write.rs"]
pub(crate) mod crate_write;
#[path = "diagnostics.rs"]
mod diagnostics;
#[path = "discovery_cache.rs"]
mod discovery_cache;
#[path = "discovery_node.rs"]
mod discovery_node;
#[path = "entry.rs"]
mod entry;
#[path = "entry_default.rs"]
mod entry_default;
#[path = "face_syntax_check.rs"]
mod face_syntax_check;
#[path = "face_view.rs"]
pub mod face_view;
#[path = "graft_plan_check.rs"]
mod graft_plan_check;
#[path = "graft_view.rs"]
mod graft_view;
#[path = "graph.rs"]
mod graph;
#[path = "identity_cache.rs"]
mod identity_cache;
#[path = "manifests.rs"]
mod manifests;
#[path = "node_identity.rs"]
mod node_identity;
#[path = "package.rs"]
mod package;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[path = "partition_view.rs"]
pub(crate) mod partition_view;
#[path = "pipeline.rs"]
mod pipeline;

#[path = "publish_lock.rs"]
pub(crate) mod publish_lock;
#[path = "registration_phase.rs"]
mod registration_phase;
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
#[path = "scope_faces.rs"]
mod scope_faces;
#[path = "shape_decl.rs"]
mod shape_decl;
#[path = "source_layout.rs"]
mod source_layout;
#[path = "source_walk.rs"]
mod source_walk;
#[path = "static_plan.rs"]
mod static_plan;

/// The one directory name the build treats as the in-repo compile-failure demo.
/// 构建视为本仓库编译失败演示目录的、唯一那个目录名。
///
/// The demo host in this repository declares it and turns on the feature below to
/// see the diagnostics; every build rule that skips, gates, or plan-excludes it
/// keys on this name. It is an explicit list of one, not an implicit rule, because
/// the name belongs to the *demo* and not to the protocol: a third-party host that
/// happens to have a top-level directory of this name gets its faces gated behind a
/// feature it never declared, silently — audit `LGC-LG-16`, marked latent because it
/// needs the directory name to match by chance. Turning it off outright would change
/// the shipped demo host's build, so the fix is to make the name one documented
/// thing rather than a string repeated five times.
/// 本仓库里的演示宿主声明它并打开下面那个特性，以看到那些诊断；每一条跳过它、门控它或在静态计划里
/// 排除它的构建规则都以这个名字为键。它是一份显式的、只有一项的清单，而不是一条隐式规则，因为这个名字
/// 属于**演示**而不是协议：恰好有同名顶层目录的第三方宿主，会让自己的面被静默门控在一个它从未声明的
/// 特性之后——审计 `LGC-LG-16`，标为潜伏是因为需要目录名恰好命中才会生效。直接取消它则会改变出厂的演示
/// 宿主的构建，因此修法是把这个名字变成"一处有文档的东西"，而不是重复五次的字符串。
///
/// It never enters identity: `file!()` and the `NodeId` hash are computed from the
/// face file's own path, and nothing here rewrites a path.
/// 它从不参与身份：`file!()` 与 `NodeId` 散列都由注册面文件自己的路径算出，这里不改写任何路径。
pub(crate) const DEMO_ONLY_DIRECTORY: &str = "compile_error_demo";

/// The feature that turns the demo directory's diagnostics into build errors.
/// 把演示目录的诊断变成构建错误的那个特性。
///
/// The generated tree carries its `compile_error!` behind this `cfg`, and
/// `pipeline` declares the cfg name to `rustc`, so the name has to be spelled the
/// same way in both places — one constant instead of three literals.
/// 生成树把它的 `compile_error!` 挂在这个 `cfg` 之后，而 `pipeline` 会向 `rustc` 声明该 cfg 名，
/// 因此两处必须拼写一致——用一处常量代替三个字面量。
pub(crate) const DEMO_ONLY_FEATURE: &str = "compile_error_demo";

// Public surface, reachable at exactly the paths callers already use.
// 对外表面，保持在调用方已经在用的路径上。
pub use entry::host_entry_source;
pub use face_view::{
    BuildScopeView, FaceView, FileRow, PruningRow, build_output_is_current, face_views,
    face_views_and_unreadable, face_views_from_pruning, face_views_with_external, read_build_scope,
    read_file_manifest, read_pruning_manifest, read_shape_manifest,
};
pub use graft_view::{
    DeclaredGraft, DeclaredGraftExpressions, DeclaredGrafts, GraftPlanRow, OVERLAY_NOTE,
    OverlayProjection, OverlaySlot, declared_grafts, graft_plan_rows, overlay_projection,
};
/// The package name Cargo reports for a package root, which is the identity
/// namespace of every face that package compiles.
/// Cargo 为某个包根报告的包名，也就是该包编译的每个面的身份命名空间。
pub use package::identity_namespace;
pub use package::package_name;
/// The module path a registration source declares, for authoring surfaces.
/// 注册面源码声明的模块路径，供创作界面使用。
pub use static_plan::source_module_path;

// Crate-internal helpers, re-exported so sibling modules keep their `super::…`
// paths after the split instead of reaching into each other's new homes.
// crate 内部辅助项，重新导出以便同级模块在拆分后保留 `super::…` 路径，
// 不必互相伸进对方的新家。
pub(crate) use build_input::BuildInput;
pub(crate) use contracts::aggregate_contract_errors;
pub(crate) use discovery_cache::{
    CACHE_SCHEMA, cached_parent_id, update_discovery_cache, write_if_changed,
};
pub use source_layout::{SourceLayout, source_layout};
pub(crate) use source_walk::{
    discover_root_reporting, discovery_fingerprint, emit_rerun_paths, source_stamp,
};
// The read path of the bridge is the only caller of these two: the member map exists to verify the
// members of a workspace beside each other, and the budget is the rule it runs under. They are
// re-exported **under that feature** because a build without the bridge would otherwise carry an
// unused import — and the definitions themselves stay ungated, since the walk uses both.
// 只有桥的读路径调用这两个：成员映射存在的意义是把工作区的成员并排核验，而预算是它运行所依的规则。
// 它们**在那个特性下**才被重导出，因为不带桥的构建否则会背上一个未使用导入——而定义本身不设门控，
// 因为遍历两处都在用。
#[cfg(feature = "mcp")]
pub(crate) use source_walk::{parallel_map_with_threshold, worker_budget};
// `resolve_host_entry` is deliberately absent: production code reaches it only
// through `host_entry_from_environment`, and re-exporting it for tests alone
// would be an unused import in a non-test build.
// 这里刻意不导出 `resolve_host_entry`：生产代码只经 `host_entry_from_environment`
// 到达它，仅为测试而导出会在非测试构建里成为未使用导入。
// The authoring surface's CLI face (`nichlink crates`) is the only caller of these, so they ride the
// same feature: a default build has no use for them, and an unused re-export is a warning this
// workspace refuses.
// 这些名字唯一的调用方是创作面的 CLI（`nichlink crates`），因此它们随着同一个特性走：默认构建用不到它们，
// 而未使用的再导出是本工作区拒绝的告警。
#[cfg(feature = "cli")]
pub(crate) use crate_facade::plan_facade;
#[cfg(feature = "cli")]
pub(crate) use crate_plan::plan as plan_crates;
#[cfg(feature = "cli")]
pub(crate) use crate_release::plan_facade as plan_release_facade;
#[cfg(feature = "cli")]
pub(crate) use crate_release::plan_ghost as plan_release_ghost;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
#[cfg(feature = "cli")]
pub(crate) use crate_write::partition_roots;
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
pub(crate) use crate_write::{guard_shape, revert_partition, write_partition, write_release};
pub(crate) use discovery_node::{Node, relative_display};
pub(crate) use entry::{HostEntry, host_entry_from_environment};
pub(crate) use entry_default::default_entry_source;
pub(crate) use face_syntax_check::{
    aggregate_parent_macro_errors, aggregate_requirements, aggregate_stable_name_errors,
    face_syntax_errors, parsed_face, unplaced_face_errors,
};
pub(crate) use graft_view::{declared_graft_view, host_graft_entries};
pub(crate) use graph::{write_generation, write_graph_manifest};
pub(crate) use identity_cache::{cache_directory, prime_node_id_cache};
pub(crate) use manifests::{
    write_file_manifest, write_function_manifest, write_graft_manifest, write_pruning_manifest,
    write_shape_manifest, write_source_scope_manifest,
};
pub(crate) use node_identity::{CACHED_NODE_IDS, node_id};
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
pub use partition_view::{OnDisk, PackageView, PartitionView};
#[cfg(any(feature = "cli", feature = "studio", feature = "mcp"))]
pub(crate) use partition_view::{apply_declaration_edit, package_directory_of};
#[cfg(feature = "cli")]
pub(crate) use registry_identity::NodeId;
pub(crate) use renderer::render_lib;
pub(crate) use scope::{
    SourceScope, collect_active_ids, face_source_is_active, module_feature, source_is_active,
};
pub(crate) use scope_faces::{FaceSource, collect_faces};
#[cfg(any(feature = "cli", feature = "studio"))]
pub(crate) use shape_decl::read_shape_declaration;
// The declaration's two writers ride the authoring surfaces: the CLI's `crates` verb, the bridge's
// `declare`/`undeclare`, and Studio's delete key all edit the same hand-written file through them.
// 声明的两个写入方随创作面走：CLI 的 `crates` 动词、桥的 `declare`/`undeclare`，以及 Studio 的删除键，
// 都经它们编辑同一个手写文件。
#[cfg(any(feature = "cli", feature = "mcp", feature = "studio"))]
pub(crate) use shape_decl::{DeclarationEdit, declare, undeclare};
pub(crate) use static_plan::static_plan;

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
    let (only, facade) = legacy_shape_from_environment();
    // `false`: this entry is the **structured** caller — it returns diagnostics instead of panicking,
    // and a caller that wants cargo directives is a build script, which is `run_for_partition`'s job
    // (two tests pin that an unwritable tree is an `Err` here, not a panic).
    // `false`：这个入口是**结构化**调用方——它返回诊断而不 panic；想要 cargo 指令的调用方是构建脚本，那是
    // `run_for_partition` 的职责（两条测试钉住：这里写不了生成树时是 `Err`，不是 panic）。
    run_shape(manifest, out_dir, package, only, facade, false)
}

/// Run the pipeline for **one fragment of a partition**: this package compiles `only` (a comma-joined
/// list of claimed subtrees) instead of the whole host, or — with `None` and `facade` — it is the
/// cross-crate half.
/// 为**划分出来的一个碎片**运行管线：这个包编译 `only`（逗号分隔的认领子树表）而不是整个宿主；或者以
/// `None` + `facade` 表示它就是跨 crate 那一半。
///
/// The claim travels as an **argument** rather than as an environment variable, because a generated
/// build script could only set one with `unsafe { std::env::set_var(…) }` under Rust 2024, and a claim
/// is a literal the generator already knows.
/// 认领以**参数**传递而不是环境变量：Rust 2024 下生成的构建脚本只能靠 `unsafe { std::env::set_var(…) }`
/// 设置环境变量，而认领本来就是生成器已知的字面量。
pub fn run_for_partition(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<&str>,
    facade: bool,
) -> Result<(), String> {
    run_shape(
        manifest,
        out_dir,
        package,
        only.map(str::to_owned),
        facade,
        true,
    )
}

/// The shared body of [`run_for`] and [`run_for_partition`].
/// [`run_for`] 与 [`run_for_partition`] 共用的主体。
fn run_shape(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<String>,
    facade: bool,
    emit_cargo_directives: bool,
) -> Result<(), String> {
    std::fs::create_dir_all(out_dir)
        .map_err(|error| format!("create {}: {error}", out_dir.display()))?;
    // A **build script** has to emit cargo's directives, and one of them matters here: the
    // `rustc-check-cfg=cfg(rust_analyzer)` line that keeps the generated tree's rust-analyzer mirror
    // from warning as an unexpected cfg. Passing `false` (the CLI's setting) left every generated
    // package warning on its own build — measured: `unexpected_cfgs` on `partitioned-button-facade`.
    // **构建脚本**必须发 cargo 指令，其中一条在这里很要紧：`rustc-check-cfg=cfg(rust_analyzer)`，它让生成树里
    // 给 rust-analyzer 的镜像不再被当成未知 cfg 警告。传 `false`（CLI 的设置）会让每个生成包在自己的构建里报警
    // ——实测：`partitioned-button-facade` 上的 `unexpected_cfgs`。
    check_for_shape(
        manifest,
        out_dir,
        package,
        only,
        facade,
        emit_cargo_directives,
    )
    .map_err(|diagnostics| diagnostics.render_build_diagnostics())
}

/// The shape the **earlier** generator declared, read from the two variables it set.
/// **早先**的生成器声明的形状，从它设置的两个变量读回。
///
/// Kept so a tree someone already partitioned keeps rendering exactly the same plan without
/// re-running the writer; the current generator writes neither variable, and its build scripts set
/// nothing at all.
/// 保留它是为了让已经划分过的树无需重跑写入方就继续渲染出同一份计划；现在的生成器两个变量都不写，
/// 它生成的构建脚本什么都不设置。
fn legacy_shape_from_environment() -> (Option<String>, bool) {
    (
        std::env::var(nichlink_kernel::lexicon::SHAPE_ONLY_ENV).ok(),
        std::env::var(nichlink_kernel::lexicon::SHAPE_FACADE_ENV).is_ok(),
    )
}

/// Run the same discovery and validation pipeline as [`run_for`], but hand the
/// caller the structured [`nichlink_kernel::BuildDiagnostics`] instead of the text a
/// terminal or `compile_error!` reads.
/// 运行与 [`run_for`] 相同的发现与校验管线，但把结构化的
/// [`nichlink_kernel::BuildDiagnostics`] 交给调用方，而不是终端或 `compile_error!` 读的文本。
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
) -> Result<(), nichlink_kernel::BuildDiagnostics> {
    check_for_shape(manifest, out_dir, package, None, false, false)
}

/// [`check_for`] for one fragment of a partition: it compiles `only`, or it is the facade.
/// [`check_for`] 用于划分出来的一个碎片：它编译 `only`，或者它就是 facade。
pub fn check_for_shape(
    manifest: &Path,
    out_dir: &Path,
    package: &str,
    only: Option<String>,
    facade: bool,
    emit_cargo_directives: bool,
) -> Result<(), nichlink_kernel::BuildDiagnostics> {
    std::fs::create_dir_all(out_dir).map_err(|error| {
        let mut diagnostics = nichlink_kernel::BuildDiagnostics::default();
        diagnostics.push(nichlink_kernel::BuildDiagnostic::new(
            "out-dir",
            format!("create {}: {error}", out_dir.display()),
        ));
        diagnostics
    })?;
    registry_identity::set_package_namespace(package.to_owned());
    let input = BuildInput::new(
        manifest.to_path_buf(),
        out_dir.to_path_buf(),
        emit_cargo_directives,
    )
    .with_shape(only, facade);
    // `package` is the authority for this run, not the process-wide pin: a
    // bridge or a test binary runs several packages in one process, and the pin
    // is first-write-wins, so a second run would otherwise publish identities
    // stamped with the first package's namespace. The scope also serializes
    // concurrent runs, because that namespace is process-wide.
    // 本次运行的权威是 `package` 而不是进程级固定值：桥或测试二进制会在一个进程里跑多个包，而该
    // 固定值先到先得，因此第二次运行发布的身份会盖着第一个包的命名空间。这个作用域同时串行化并发
    // 运行，因为那个命名空间是进程级的。
    match registry_identity::run_as_package(package, || pipeline::run(&input)) {
        Some(diagnostics) => Err(diagnostics),
        None => Ok(()),
    }
}
