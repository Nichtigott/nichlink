//! NichLink Studio: an optional Ratatui adapter for the registration core.
//! NichLink Studio：注册核心的可选 Ratatui 适配器。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。
#![warn(missing_docs)]

#[path = "studio/studio.rs"]
mod studio;

pub use studio::launch;
