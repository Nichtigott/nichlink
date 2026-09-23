//! Read-only MCP stdio bridge for compact NichLink Rust source queries.
//! 面向紧凑 NichLink Rust 源码查询的只读 MCP stdio 桥。
//!
//! The five tools below index Rust sources only; this batch ships no registry
//! or contract query, despite any older wording that suggested one.
//! 下列五个工具只索引 Rust 源码；本批次不提供注册树或合同查询，
//! 早前暗示具备该能力的表述已更正。
//!
//! JSON-RPC frames arrive on stdin and responses leave on stdout. The bridge
//! only reads Rust sources below the configured `NICH_LINK_PACKAGE_ROOT`; it
//! never writes to the registry or the filesystem.
//! JSON-RPC 帧从 stdin 进入、响应从 stdout 输出。桥只读取配置的
//! `NICH_LINK_PACKAGE_ROOT` 之下的 Rust 源码，绝不写入注册树或文件系统。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。
#![warn(missing_docs)]

#[path = "protocol.rs"]
mod protocol;
pub use protocol::run;

#[path = "tools.rs"]
mod tools;

#[path = "index.rs"]
mod index;
