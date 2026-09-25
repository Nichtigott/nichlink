//! The MCP stdio bridge binary.
//! MCP stdio 桥的二进制入口。
//!
//! All behaviour lives in the library, so a host embedding the bridge and the
//! installed `nichlink mcp` path run exactly the same code.
//! 所有行为都在库里，因此嵌入桥的宿主与已安装的 `nichlink mcp` 跑的是同一份代码。

// The published surface must be readable on docs.rs without leaving the page,
// so the lint is on for the whole crate; `clippy -D warnings` makes a new
// undocumented public item a failure.
// 发布表面必须能在 docs.rs 上不跳页读懂，因此 lint 开在整个 crate 上；
// `clippy -D warnings` 会让新增的、没有文档的公开项变成失败。
#![warn(missing_docs)]

fn main() {
    if let Err(error) = nichlink_mcp::run() {
        eprintln!("nichlink-mcp: {error}");
        std::process::exit(1);
    }
}
