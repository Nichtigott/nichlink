//! NichLink's registry protocol and runtime core.

// Allow moved plugin shell modules to keep their self-referential
// `pub use nichlink::...` compatibility re-exports.
// 让迁移过来的插件壳模块保留自引用的 `pub use nichlink::...` 兼容重导出。
extern crate self as nichlink;

#[path = "registry_core.rs"]
pub mod registry_core;

pub use registry_core::*;
