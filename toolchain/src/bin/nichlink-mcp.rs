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
    // No arguments = serve the stdio bridge, exactly as before; `--list` and `--call` are the
    // one-shot client the chain-fit evaluation measured the absence of.
    // 不带参数＝照旧做 stdio 桥服务；`--list` 与 `--call` 就是思维链拟合评测量出来的那个"缺失的一次性
    // 客户端"。
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match nichlink_toolchain::mcp::client::run_client(&arguments) {
        nichlink_toolchain::mcp::client::Client::Serve => {
            if let Err(error) = nichlink_toolchain::mcp::run() {
                eprintln!("nichlink-toolchain: {error}");
                std::process::exit(1);
            }
        }
        // The wait for a refresh a call started lives in the library's one-shot client, so an
        // embedding host and this binary behave the same (audit `M7`, P1.2).
        // 调用所启动的刷新那一段等待住在库里的一次性客户端里，因此嵌入宿主与本二进制行为一致（审计 `M7`，P1.2）。
        nichlink_toolchain::mcp::client::Client::Called(code) => std::process::exit(code),
    }
}
