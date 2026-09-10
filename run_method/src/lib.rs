//! NichLink runtime: trace state, macros, and the authoring executor.
//! The registry tree and plugin protocol live in the kernel; this crate is
//! the run_method execution surface.
//! NichLink 运行期：trace 状态、宏与 authoring 执行器。
//! 注册树与插件协议在 kernel；本 crate 是 run_method 执行面。

#[macro_use]
#[path = "macros/macros.rs"]
pub mod macros;
#[cfg(feature = "authoring")]
pub mod authoring;
#[path = "call_report/call_report.rs"]
pub mod call_report;
#[path = "entry/entry.rs"]
pub mod entry;
pub mod plugin;
pub mod registry;
pub mod runtime;

pub use nichlink::registry_core;
#[allow(ambiguous_glob_reexports)]
pub use nichlink::*;

#[cfg(feature = "authoring")]
#[allow(ambiguous_glob_reexports)]
pub use authoring::*;
pub use call_report::*;
pub use entry::*;
pub use runtime::*;
