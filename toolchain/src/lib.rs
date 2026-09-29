//! The publishing surface: the seven thin execution surfaces of NichLink in one crate.
//! 发布面：NichLink 的七个薄执行面合成一个 crate。
//!
//! Each module was its own crate before the merge; the module name is the merge plan's
//! name for that surface (`build_method` -> `build_time`, `run_method` -> `runtime`, ...).
//! 每个模块在合并前都是独立 crate；模块名取合并方案里那一面的名字
//! （`build_method` -> `build_time`、`run_method` -> `runtime`，等）。
//!
//! The root globs every module so the historical crate-root paths (`crate::host!()` and
//! friends) still resolve; `run`-shaped names live in their modules, and the build entry
//! is written `crate::build_time::run()`.
//! 根部把每个模块 glob 出来，使历史上的 crate 根路径仍然可解析；`run` 形状的名字留在各自
//! 模块里，构建入口写成 `crate::build_time::run()`。
#![warn(missing_docs)]

// t115-mount: begin
#[cfg(feature = "build")]
#[path = "build_time/src/lib.rs"]
pub mod build_time;
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
#[path = "runtime/src/lib.rs"]
pub mod runtime;
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
pub use self::build_time::*;
#[cfg(feature = "evidence")]
pub use self::call_evidence::*;
#[cfg(feature = "cli")]
pub use self::cli::*;
#[cfg(feature = "mcp")]
pub use self::mcp::*;
#[cfg(feature = "plugins")]
pub use self::plugin_host::*;
#[cfg(feature = "run")]
pub use self::runtime::*;
#[cfg(feature = "studio")]
pub use self::studio::*;
// t115-root-glob: end
