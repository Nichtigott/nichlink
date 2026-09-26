//! Read-only MCP stdio bridge for compact NichLink source and registry queries.
//! 面向紧凑 NichLink 源码与注册树查询的只读 MCP stdio 桥。
//!
//! Five tools index Rust source text; `nichlink.registry` is the sixth and the
//! first that does not: it reports the registration tree the *build* derives,
//! through the same `face_views` the CLI's `explain` uses, so an agent can ask
//! what the registry is instead of reconstructing it from macro names. Contract,
//! admission, and registration-rule fields are still absent: those need the
//! built face snapshots, not a source scan.
//! 五个工具索引 Rust 源码文本；`nichlink.registry` 是第六个，也是第一个不这么做的：它报告
//! **构建**推导出的注册树，走的是 CLI 的 `explain` 所用的同一个 `face_views`，因此代理可以
//! 直接问注册树是什么，而不是从宏名重建。contract、admission 与 registration rule 字段仍然
//! 没有：那些需要已构建的面快照，而不是源码扫描。
//!
//! JSON-RPC frames arrive on stdin and responses leave on stdout. The bridge
//! only reads below the configured `NICH_LINK_PACKAGE_ROOT`; it never writes to
//! the registry or the filesystem. Naming a package does run `cargo metadata`
//! (through `nichlink_build_method::package_name`), because the package name is
//! the `NodeId` namespace and Cargo is its authority.
//! JSON-RPC 帧从 stdin 进入、响应从 stdout 输出。桥只读取配置的
//! `NICH_LINK_PACKAGE_ROOT` 之下的内容，绝不写入注册树或文件系统。为包命名确实会运行
//! `cargo metadata`（经 `nichlink_build_method::package_name`），因为包名就是 `NodeId`
//! 命名空间，而 Cargo 是它的权威。

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

#[path = "registry.rs"]
mod registry;

#[path = "index.rs"]
mod index;
