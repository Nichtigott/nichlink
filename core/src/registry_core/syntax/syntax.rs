//! Parser shared by build-time checks and live authoring.
//! 构建期检查与实时编辑共用的注册面解析器。
//!
//! Two grammars live here: the registration-face parser in [`face`] and the
//! host-entry declarations in [`entries`]. The module tree mirrors that split.
//! 这里有两种语法：位于 [`face`] 的注册面解析器，以及位于 [`entries`] 的宿主入口声明。
//! 模块树与这一分工一致。

#[path = "face.rs"]
pub mod face;
pub use face::*;
#[path = "entries.rs"]
pub mod entries;
pub use entries::*;
#[path = "nesting.rs"]
pub(crate) mod nesting;
#[path = "reference_scan.rs"]
pub(crate) mod reference_scan;

// The module stays crate-private; the one function other crates need is
// re-exported so the workspace keeps a single nesting measurement.
// 模块保持 crate 私有；其他 crate 需要的那一个函数在此重导出，使工作区只保留一份嵌套度量。
pub use nesting::guard_nesting;

#[cfg(test)]
#[path = "deep_input_tests.rs"]
mod deep_input_tests;
