//! MCP stdio bridge for compact NichLink source and registry queries, plus
//! previewed authoring writes.
//! 面向紧凑 NichLink 源码与注册树查询、以及先预览后落盘的创作写入的 MCP stdio 桥。
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
//! JSON-RPC frames arrive on stdin and responses leave on stdout. Reads stay
//! below the configured `NICH_LINK_PACKAGE_ROOT`. Writes exist — `nichlink.apply`
//! — and they go through the **same authoring executor Studio uses**, inside an
//! `AuthoringContext` built from the resolved package root and the namespace
//! Cargo reports, so an agent's edit passes the kernel's admission and topology
//! checks instead of re-implementing them here. A write is previewed first: the
//! tool runs the real operation against a throwaway copy of the package and
//! reports the resulting tree and file diff; only `apply: true` touches the
//! project. Naming a package runs `cargo metadata` (through
//! `nichlink_build_method::package_name`), because the package name is the
//! `NodeId` namespace and Cargo is its authority.
//! JSON-RPC 帧从 stdin 进入、响应从 stdout 输出。读取仍在配置的
//! `NICH_LINK_PACKAGE_ROOT` 之下。写入是存在的——`nichlink.apply`——而且它走**与 Studio
//! 相同的 authoring 执行器**，运行在由已解析包根与 Cargo 报告的命名空间构成的
//! `AuthoringContext` 里，因此代理的编辑会经过内核的准入与拓扑校验，而不是在这里重新实现一遍。
//! 写入先预览：工具在一份一次性的包副本上运行真实操作，报告将得到的树与文件 diff；只有
//! `apply: true` 才会碰真实项目。为包命名会运行 `cargo metadata`（经
//! `nichlink_build_method::package_name`），因为包名就是 `NodeId` 命名空间，而 Cargo 是它
//! 的权威。

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

#[path = "apply.rs"]
mod apply;

#[path = "nodes.rs"]
mod nodes;

#[path = "preview.rs"]
mod preview;

#[path = "index.rs"]
mod index;
