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

/// Run the read-only MCP stdio bridge over the process's own streams.
/// 在进程自己的流上运行只读 MCP stdio 桥。
///
/// A transport failure is reported rather than swallowed. This process exists to
/// answer on stdout, so an unreadable stdin or an unwritable stdout means it can
/// no longer do its job; exiting zero there would tell the caller the session
/// ended normally.
/// 传输失败会被报告而不是吞掉。本进程的存在意义就是在 stdout 上作答，因此 stdin 读不了或
/// stdout 写不了意味着它再也做不了这件事；在那里以 0 退出会告诉调用方这次会话正常结束。
pub fn run() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    run_with(&mut stdin.lock(), &mut io::BufWriter::new(stdout.lock()))
}

/// Run the bridge over explicit streams, so the framing can be pinned by a test.
/// 在显式给定的流上运行桥，使分帧行为可以被测试钉住。
///
/// The streams are parameters rather than the process's own because the two
/// failure modes worth pinning — an unreadable request line and an unwritable
/// response — cannot be produced on a real terminal from inside a test.
/// 两个流作为参数传入而不是取进程自己的，是因为值得钉住的两种失败——请求行读不了、响应写不了
/// ——在测试里无法在真实终端上造出来。
pub fn run_with(input: &mut dyn BufRead, output: &mut dyn Write) -> Result<(), String> {
    let root = package_root();
    for line in input.lines() {
        let line = line.map_err(|error| format!("cannot read stdin: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        let responses: Vec<Value> = match serde_json::from_str::<Value>(&line) {
            // A batch is a JSON array of requests, and JSON-RPC answers it with
            // one response per request that is not a notification. Dispatching
            // the array as if it were a request collapsed it into one error.
            // 批处理是请求的 JSON 数组，JSON-RPC 对其中每个非通知请求各回一个响应。把数组
            // 当成单个请求分派会把它折叠成一个错误。
            Ok(Value::Array(requests)) if requests.is_empty() => vec![error_response(
                Value::Null,
                -32600,
                "invalid request: empty batch".to_owned(),
            )],
            Ok(Value::Array(requests)) => requests
                .iter()
                .map(|request| dispatch(&root, request))
                .filter(|response| !response.is_null())
                .collect(),
            Ok(request) => {
                let response = dispatch(&root, &request);
                if response.is_null() {
                    Vec::new()
                } else {
                    vec![response]
                }
            }
            Err(error) => vec![error_response(
                Value::Null,
                -32700,
                format!("invalid JSON: {error}"),
            )],
        };
        for response in responses {
            serde_json::to_writer(&mut *output, &response)
                .map_err(|error| format!("cannot write stdout: {error}"))?;
            output
                .write_all(b"\n")
                .and_then(|()| output.flush())
                .map_err(|error| format!("cannot write stdout: {error}"))?;
        }
    }
    Ok(())
}

fn dispatch(root: &Path, request: &Value) -> Value {
    // JSON-RPC 2.0: a request without an `id` member is a notification and MUST
    // NOT be answered. An explicit `null` id is still a request, which is why
    // the check is for the member's absence rather than for a null value.
    // JSON-RPC 2.0：没有 `id` 成员的请求是通知，**不得**作答。显式的 `null` id 仍是
    // 请求，因此这里判断的是成员缺失，而不是值为 null。
    let Some(id) = request.get("id").cloned() else {
        return Value::Null;
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A writer that fails on every write, standing in for a closed pipe.
    /// 每次写入都失败的写端，用来代替已关闭的管道。
    struct Broken;

    impl Write for Broken {
        fn write(&mut self, _buffer: &[u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
        }
    }

    /// A request whose answer cannot be delivered is an error, not a quiet
    /// shutdown: the caller is waiting for a reply.
    /// 应答送不出去的请求是错误，而不是安静地结束：调用方在等回复。
    #[test]
    fn an_unwritable_response_is_reported() {
        let mut input = &b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n"[..];
        let error =
            run_with(&mut input, &mut Broken).expect_err("a write failure must be an error");
        assert!(error.contains("cannot write stdout"), "{error}");
    }

    /// A readable end of input ends the session successfully.
    /// 输入正常结束会让本次会话成功收尾。
    #[test]
    fn a_closed_input_ends_the_session_cleanly() {
        let mut input = &b""[..];
        let mut output = Vec::new();
        run_with(&mut input, &mut output).expect("an empty stdin is a clean end");
        assert!(output.is_empty());
    }

    /// A notification — a request with no `id` member — is never answered:
    /// JSON-RPC forbids a response to it, and the client that sent it is not
    /// waiting for one.
    /// 通知——没有 `id` 成员的请求——绝不被作答：JSON-RPC 禁止对它作答，发送它的客户端
    /// 也不在等回复。
    #[test]
    fn a_notification_gets_no_reply() {
        let mut input = &b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"method\":\"ping\"}\n"[..];
        let mut output = Vec::new();
        run_with(&mut input, &mut output).expect("a notification is not a failure");
        assert!(
            output.is_empty(),
            "no reply may be written for a notification: {}",
            String::from_utf8_lossy(&output)
        );
    }

    /// A batch is answered element by element: each request in the array gets
    /// its own response object, and the array is never collapsed into one error.
    /// 批处理逐元素作答：数组里的每个请求得到自己的响应对象，数组绝不会被折叠成一个错误。
    #[test]
    fn a_batch_gets_one_reply_per_element() {
        let mut input = &b"[{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"},{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}]\n"[..];
        let mut output = Vec::new();
        run_with(&mut input, &mut output).expect("a batch is not a failure");
        let text = String::from_utf8(output).expect("utf-8 replies");
        let replies = text
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).expect("one reply per line"))
            .collect::<Vec<_>>();
        assert_eq!(replies.len(), 2, "one reply per element: {text}");
        assert_eq!(replies[0]["id"], 1, "{text}");
        assert!(replies[0]["result"].is_object(), "{text}");
        assert_eq!(replies[1]["id"], 2, "{text}");
        assert!(replies[1]["result"]["tools"].is_array(), "{text}");
    }
}
