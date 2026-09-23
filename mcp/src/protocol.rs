//! JSON-RPC framing and method dispatch for the read-only MCP bridge.
//! 只读 MCP 桥的 JSON-RPC 分帧与方法分派。

use serde_json::{Value, json};
use std::env;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use crate::tools::{tool_call, tools};

/// Protocol revision advertised during initialization.
/// 初始化时通告的协议版本。
const PROTOCOL_VERSION: &str = "2025-06-18";
/// Default result cap for `nichlink.search`.
/// `nichlink.search` 的默认结果上限。
pub(crate) const DEFAULT_LIMIT: usize = 40;
/// Hard cap on lines returned by `nichlink.read`.
/// `nichlink.read` 返回行数的硬上限。
pub(crate) const MAX_READ_LINES: usize = 240;

/// The project this bridge reads, resolved by `lexicon`'s rule.
/// 本桥读取的项目，按 `lexicon` 的规则解析。
///
/// The bridge calls the same `lexicon::resolve_package_root` the authoring
/// executor and Studio call, so the agent, the editor and the executor cannot
/// disagree about which project is open. Only the last-resort fallback is the
/// bridge's own and it stays the working directory, deliberately: a stdio bridge
/// is started inside the project the agent is working on, whereas the crate's
/// own manifest path is a path on the machine that compiled it — wrong for an
/// installed binary.
/// 本桥调用创作执行器与 Studio 所调用的同一个 `lexicon::resolve_package_root`，因此代理、
/// 编辑器与执行器不会对"打开的是哪个项目"产生分歧。只有最后兜底属于本桥自己，并且有意保持
/// 为当前目录：stdio 桥是在代理正在处理的项目里启动的，而本 crate 自己的清单路径是编译它的
/// 那台机器上的路径——对已安装的二进制来说是错的。
fn package_root() -> PathBuf {
    let configured = env::var_os(nichlink::lexicon::PACKAGE_ROOT_ENV).map(PathBuf::from);
    let current = env::current_dir().ok();
    let fallback = current.clone().unwrap_or_else(|| PathBuf::from("."));
    nichlink::lexicon::resolve_package_root(
        configured.as_deref(),
        current.as_deref(),
        current
            .as_ref()
            .is_some_and(|directory| directory.join("Cargo.toml").is_file()),
        &fallback,
    )
}

/// Run the read-only MCP stdio bridge until stdin closes or the transport
/// fails.
/// 运行只读 MCP stdio 桥，直到 stdin 关闭或传输失败。
pub fn run() {
    let root = package_root();
    let stdin = io::stdin();
    let mut output = io::BufWriter::new(io::stdout().lock());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => dispatch(&root, &request),
            Err(error) => error_response(Value::Null, -32700, format!("invalid JSON: {error}")),
        };
        if response.is_null() {
            continue;
        }
        if serde_json::to_writer(&mut output, &response).is_err() {
            break;
        }
        if output.write_all(b"\n").is_err() || output.flush().is_err() {
            break;
        }
    }
}

fn dispatch(root: &Path, request: &Value) -> Value {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or("");
    let params = request.get("params").unwrap_or(&Value::Null);
    match method {
        "initialize" => success(
            id,
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": { "tools": { "listChanged": false } },
                "serverInfo": { "name": "nichlink-mcp", "version": env!("CARGO_PKG_VERSION") },
                "instructions": "Use nichlink.search before reading source; callgraph is static-heuristic."
            }),
        ),
        "notifications/initialized" | "notifications/cancelled" => Value::Null,
        "ping" => success(id, json!({})),
        "tools/list" => success(id, json!({ "tools": tools() })),
        "tools/call" => tool_call(root, id, params),
        _ => error_response(id, -32601, format!("unknown method `{method}`")),
    }
}

/// Wrap a JSON-RPC result payload. 包装 JSON-RPC 结果负载。
pub(crate) fn success(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

/// Wrap a JSON-RPC error payload. 包装 JSON-RPC 错误负载。
pub(crate) fn error_response(id: Value, code: i64, message: String) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
