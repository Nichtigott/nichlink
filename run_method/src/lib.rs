//! NichLink runtime: trace state, macros, and the authoring executor.
//! The registry tree and plugin protocol live in the kernel; this crate is
//! the run_method execution surface.
//! NichLink 运行期：trace 状态、宏与 authoring 执行器。
//! 注册树与插件协议在 kernel；本 crate 是 run_method 执行面。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。
#![warn(missing_docs)]

#[macro_use]
#[path = "macros/macros.rs"]
pub mod macros;
#[cfg(feature = "authoring")]
#[path = "authoring/authoring.rs"]
pub mod authoring;
#[path = "call_report/call_report.rs"]
pub mod call_report;
#[path = "plugin/plugin.rs"]
pub mod plugin;
#[path = "registry/registry.rs"]
pub mod registry;
#[path = "runtime/runtime.rs"]
pub mod runtime;

pub use nichlink::registry_core;
// This glob names the same kernel items the historical `nichlink::*` glob did,
// and it overlaps this crate's own `authoring` shim (`parse`, `snapshot`,
// `validation`). Both paths are the same surface, so the overlap is allowed
// here rather than resolved.
// 本 glob 命名的内核条目与历史上的 `nichlink::*` 完全相同，并与本 crate 自己的
// `authoring` shim（`parse`、`snapshot`、`validation`）重叠。两条路径同属一个执行面，
// 因此这里允许重叠而不做消解。
#[allow(ambiguous_glob_reexports)]
pub use nichlink::registry_core::*;
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
pub use runtime::*;
