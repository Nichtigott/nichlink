//! The publishing surface: the seven thin execution surfaces of NichLink in one crate.
//! 发布面：NichLink 的七个薄执行面合成一个 crate。
//!
//! Each module was its own crate before the merge; the module name is the merge plan's
//! name for that surface (`build_method` -> `build_method`, `run_method` -> `runtime`, ...).
//! 每个模块在合并前都是独立 crate；模块名取合并方案里那一面的名字
//! （`build_method` -> `build_method`、`run_method` -> `runtime`，等）。
//!
//! The root globs every module so the historical crate-root paths (`crate::host!()` and
//! friends) still resolve; `run`-shaped names live in their modules, and the build entry
//! is written `crate::build_method::run()`.
//! 根部把每个模块 glob 出来，使历史上的 crate 根路径仍然可解析；`run` 形状的名字留在各自
//! 模块里，构建入口写成 `crate::build_method::run()`。
#![warn(missing_docs)]

// The merged crate has to be able to name itself. Macros that the old per-crate layout expanded
// into carry `::nichlink_toolchain::…` paths — they were written when every surface was its own
// crate and the host was an external caller — and the in-crate test modules that arrived with
// batch 2 were written the same way. Without this alias those expansions fail with "cannot find
// `nichlink_toolchain` in the crate root", which is what parked the `(b)` residue.
// 合并后的 crate 必须能叫出自己的名字。旧的分 crate 布局下，那些宏展开出来的路径是
// `::nichlink_toolchain::…`——写它们的时候每个执行面都是独立 crate、宿主是外部调用者——而随批 2
// 一起进来的 crate 内测试模块也按同样方式书写。没有这个别名，那些展开会以
// "cannot find `nichlink_toolchain` in the crate root" 失败，这正是 `(b)` 遗留当初被搁置的原因。
extern crate self as nichlink_toolchain;

/// The identity namespace the in-crate face probes declare under.
/// crate 内的面探针声明注册面时所用的身份命名空间。
///
/// The probes (the `call_evidence` collector probe and its siblings) declare faces with the
/// declaration macros, which read `crate::NICHLINK_NAMESPACE` — **this** crate's root, because a
/// probe is mounted inside the crate (audit `M7`, P3.3). The value is the one those probes used to
/// bake in by hand, so nothing about them moves. It exists only for test builds: a published
/// toolchain crate is not a host and has no identity domain of its own.
/// 探针（`call_evidence` 的采集探针及其同类）用声明宏声明注册面，而声明宏读的是
/// `crate::NICHLINK_NAMESPACE`——**本** crate 的根，因为探针挂载在 crate 之内（审计 `M7`，P3.3）。
/// 这个值就是那些探针过去手工烤进去的那个，因此它们什么都不移动。它只存在于测试构建里：发布出去的工具链
/// crate 不是宿主、没有自己的身份域。
#[cfg(test)]
pub const NICHLINK_NAMESPACE: &str = env!("CARGO_PKG_NAME");

// t115-mount: begin
#[cfg(feature = "build")]
#[path = "build_method/src/lib.rs"]
pub mod build_method;
#[cfg(feature = "evidence")]
#[path = "call_evidence/src/lib.rs"]
pub mod call_evidence;
#[cfg(feature = "cli")]
#[path = "cli/src/lib.rs"]
pub mod cli;
#[cfg(feature = "mcp")]
#[path = "mcp/src/lib.rs"]
pub mod mcp;
#[cfg(feature = "plugins")]
#[path = "plugin_host/src/lib.rs"]
pub mod plugin_host;
#[cfg(feature = "run")]
#[path = "run_method/src/lib.rs"]
pub mod run_method;
#[cfg(feature = "studio")]
#[path = "studio/src/lib.rs"]
pub mod studio;
// t115-mount: end

// t115-root-glob: begin
// The historical crate root of each surface, lifted so `crate::X` (and therefore
// `nichlink_toolchain::X`) resolves for every module item.  Explicit re-exports below
// win over these globs where a name would be ambiguous.
// 把每个执行面的历史 crate 根提升到根上，使 `crate::X`（也就是 `nichlink_toolchain::X`）
// 对每个模块条目都可解析；下面显式再导出的名字在冲突处优先于这些 glob。
#[cfg(feature = "build")]
pub use self::build_method::*;
#[cfg(feature = "evidence")]
pub use self::call_evidence::*;
#[cfg(feature = "cli")]
pub use self::cli::*;
// `mcp` contributes exactly one root-level name (`run`, its stdio entry) and that name is
// claimed explicitly above, so its glob would re-export nothing — which `unused_imports`
// reports under `-D warnings`. The module keeps its path: `nichlink_toolchain::mcp::run`
// is how the bridge entry is addressed, and the module itself is still mounted below.
// `mcp` 只贡献一个根级名字（`run`，它的 stdio 入口），而那个名字已被上面的显式再导出占用，
// 于是它的 glob 什么也带不上来——这在 `-D warnings` 下会被 `unused_imports` 报出来。模块路径
// 保留：桥的入口按 `nichlink_toolchain::mcp::run` 寻址，模块本身仍在上面挂载。
#[cfg(feature = "plugins")]
pub use self::plugin_host::*;
#[cfg(feature = "run")]
pub use self::run_method::*;
#[cfg(feature = "studio")]
pub use self::studio::*;

// Two names are carried by more than one of the globs above, so the paragraph at the top of
// this block has to be made true rather than merely asserted: `run` arrives from `build_method`,
// `mcp` and `cli`, and `mir` arrives both from `call_evidence` and from `runtime` (which
// re-exports the kernel's `mir`). The explicit re-exports below win over the globs and name the
// host-facing one of each pair — the build-script entry `build_method::run()` and the kernel's
// `mir` module — while every other name stays reachable through the globs. Nothing else is
// promised at this level: the module path is still the official address
// (`nichlink_toolchain::mcp::run`, `nichlink_toolchain::cli::run`, …). Measured by
// `cargo clippy --all-targets --all-features -- -D warnings`, which reported
// `ambiguous_glob_reexports` for both names before this pair existed.
// 上面这些 glob 里有两个名字被多个模块同时带上，因此这一段开头那句话得**做到**而不只是写着：
// `run` 会从 `build_method`、`mcp`、`cli` 三处来，`mir` 会同时从 `call_evidence` 与 `runtime`
// （后者重导出了内核的 `mir`）来。下面这两行显式再导出优先于 glob，并各自点名宿主面向的那一个
// ——构建脚本入口 `build_method::run()` 与内核的 `mir` 模块；其余名字仍可经 glob 取得。
// 这一层不再承诺别的：官方地址依旧是模块路径（`nichlink_toolchain::mcp::run`、
// `nichlink_toolchain::cli::run` 等）。判据是
// `cargo clippy --all-targets --all-features -- -D warnings`：在这一对出现之前，这两个名字都会
// 报 `ambiguous_glob_reexports`。
#[cfg(feature = "build")]
pub use self::build_method::run;
#[cfg(feature = "run")]
pub use self::run_method::mir;
// t115-root-glob: end
