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
/// The face field front end, re-exported so a host does not have to depend on
/// the proc-macro crate itself.
/// 注册面字段前端；再导出后宿主无需自己依赖 proc-macro crate。
pub use nichlink_macro::face_fields;
/// The editor-only field mirror used by generated aliases.
/// 生成的别名使用的、仅供编辑器的字段镜像。
pub use nichlink_macro::face_fields_mirror;
/// The default `registry_rule` resolver used by the declarative face arms.
/// 声明式注册面分支使用的 `registry_rule` 默认值解析器。
#[doc(hidden)]
pub use nichlink_macro::face_rule_or as __face_rule_or;

#[cfg(feature = "authoring")]
#[allow(ambiguous_glob_reexports)]
pub use authoring::*;
pub use call_report::*;
pub use entry::*;
pub use runtime::*;
