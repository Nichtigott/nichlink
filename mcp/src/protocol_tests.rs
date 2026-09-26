//! Tests for the JSON-RPC framing and dispatch.
//! JSON-RPC 分帧与分派的测试。

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
    let error = run_with(&mut input, &mut Broken).expect_err("a write failure must be an error");
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

/// Answer every line of `input` and return the replies in order.
/// 对 `input` 的每一行作答，并按顺序返回响应。
fn replies(input: &str) -> Vec<Value> {
    let mut input = input.as_bytes();
    let mut output = Vec::new();
    run_with(&mut input, &mut output).expect("the bridge answers");
    String::from_utf8(output)
        .expect("utf-8 replies")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("one reply per line"))
        .collect()
}

/// A line past the cap is refused instead of allocated, its remainder is
/// drained, and the next frame is still answered — which is what proves the
/// drain happened: a leftover tail would have been parsed as the next frame.
/// 超过上限的行被拒绝而不是被分配，其余部分被排空，下一帧仍被作答——这正是排空发生的证据：
/// 残留的尾巴本会被当成下一帧解析。
#[test]
fn an_over_long_line_is_refused_and_the_next_frame_still_answers() {
    let long = "x".repeat(MAX_REQUEST_BYTES + 8);
    let replies = replies(&format!(
        "{long}\n{{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}}\n"
    ));
    assert_eq!(replies.len(), 2, "{replies:?}");
    assert_eq!(replies[0]["error"]["code"], -32600, "{replies:?}");
    let message = replies[0]["error"]["message"]
        .as_str()
        .expect("a message")
        .to_owned();
    assert!(message.contains("exceeds"), "{message}");
    assert_eq!(replies[1]["id"], 1, "{replies:?}");
    assert!(replies[1]["result"].is_object(), "{replies:?}");
}

/// A request must carry the version member, and the wrong one is refused by
/// name rather than answered as if it were 2.0.
/// 请求必须携带版本成员；版本不对的请求按名字被拒，而不是被当成 2.0 作答。
#[test]
fn a_request_without_or_with_another_version_is_refused() {
    let replies = replies(
        "{\"id\":1,\"method\":\"ping\"}\n\
             {\"jsonrpc\":\"1.0\",\"id\":2,\"method\":\"ping\"}\n\
             {\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"ping\"}\n",
    );
    assert_eq!(replies.len(), 3, "{replies:?}");
    for reply in &replies[..2] {
        assert_eq!(reply["error"]["code"], -32600, "{replies:?}");
        let message = reply["error"]["message"].as_str().expect("a message");
        assert!(message.contains("jsonrpc"), "{message}");
    }
    assert_eq!(replies[2]["id"], 3, "{replies:?}");
    assert!(replies[2]["result"].is_object(), "{replies:?}");
}

/// A notification stays silent even with the wrong version: JSON-RPC forbids
/// answering it at all, so the version check must not turn it into a reply.
/// 通知即使版本不对也保持沉默：JSON-RPC 完全禁止对它作答，因此版本检查不能把它变成回复。
#[test]
fn a_notification_with_another_version_is_still_silent() {
    let replies = replies("{\"jsonrpc\":\"1.0\",\"method\":\"notifications/initialized\"}\n");
    assert!(replies.is_empty(), "{replies:?}");
}

/// The registry tool is listed and reachable, and it answers about *this*
/// package: `cargo test` runs a unit-test binary with the package root as its
/// working directory, which is where the bridge resolves its package from, so
/// the namespace it reports has to be this crate's own `CARGO_PKG_NAME` — the
/// same value the declaration macros bake into every identity. A tool missing
/// from the catalog or from the dispatch table answers `unknown tool` here.
/// 注册树工具被列出且可达，并且它作答的是**本**包：`cargo test` 以包根为工作目录运行单元测试
/// 二进制，而桥正是从那里解析包的，因此它报告的命名空间必须是本 crate 自己的
/// `CARGO_PKG_NAME`——也就是声明宏烤进每个身份的那个值。目录里缺失或分派表里缺失的工具会在这里
/// 答 `unknown tool`。
#[test]
fn the_registry_tool_is_listed_and_answers_about_this_package() {
    let listed = replies("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n");
    let names = listed[0]["result"]["tools"]
        .as_array()
        .expect("a tool array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"nichlink.registry"), "{names:?}");

    let called = replies(
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\
         \"params\":{\"name\":\"nichlink.registry\",\"arguments\":{}}}\n",
    );
    assert_eq!(called.len(), 1, "{called:?}");
    assert_eq!(called[0]["result"]["isError"], false, "{called:?}");
    let text = called[0]["result"]["content"][0]["text"]
        .as_str()
        .expect("a text reply");
    assert!(
        text.starts_with(&format!("namespace {}\n", env!("CARGO_PKG_NAME"))),
        "{text}"
    );
    assert!(text.contains("faces "), "{text}");
}

/// The write tool is listed and dispatched, and a request it cannot serve comes
/// back as an error the client can read. The unsupported action is deliberate: it
/// exercises the dispatch arm without copying or writing the package the test
/// binary happens to run in.
/// 写入工具被列出且已分派，而它无法服务的请求以客户端可读的错误回来。有意选一个不支持的动作：
/// 这样既走到分派分支，又不会复制或写入测试二进制碰巧运行所在的那个包。
#[test]
fn the_apply_tool_is_listed_and_dispatched() {
    let listed = replies("{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"tools/list\"}\n");
    let names = listed[0]["result"]["tools"]
        .as_array()
        .expect("a tool array")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect::<Vec<_>>();
    assert!(names.contains(&"nichlink.apply"), "{names:?}");

    let called = replies(
        "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\
         \"params\":{\"name\":\"nichlink.apply\",\"arguments\":{\"action\":\"delete\"}}}\n",
    );
    assert_eq!(called.len(), 1, "{called:?}");
    assert_eq!(called[0]["result"]["isError"], true, "{called:?}");
    let text = called[0]["result"]["content"][0]["text"]
        .as_str()
        .expect("a text reply");
    assert!(text.contains("not implemented"), "{text}");
}

/// A member that is not an object is answered with `-32600` and a null id,
/// in a single request and inside a batch alike; it used to be dropped
/// silently, leaving the client waiting.
/// 不是对象的成员以 `-32600` 与 null id 作答，单请求与批处理里都一样；它此前被静默丢弃，
/// 让客户端一直等。
#[test]
fn a_member_that_is_not_an_object_is_refused_rather_than_dropped() {
    let single = replies("\"hello\"\n");
    assert_eq!(single.len(), 1, "{single:?}");
    assert_eq!(single[0]["error"]["code"], -32600, "{single:?}");
    assert!(single[0]["id"].is_null(), "{single:?}");

    let batch = replies("[1,2]\n");
    assert_eq!(batch.len(), 2, "{batch:?}");
    for reply in &batch {
        assert_eq!(reply["error"]["code"], -32600, "{batch:?}");
        assert!(reply["id"].is_null(), "{batch:?}");
    }
}
