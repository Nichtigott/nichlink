//! Pins for the one-shot client: the list is the catalogue in one line each, and a call's exit code
//! is its verdict.
//! 一次性客户端的钉子：清单是目录的一行式形态，而一次调用的退出码就是它的判定。

use super::*;

/// The list has one line per tool, each naming the tool and then its first sentence.
/// 清单每个工具一行，先点名工具、再给它的第一句。
#[test]
fn the_list_is_the_catalogue_one_line_each() {
    let lines = list_tool_lines();
    assert_eq!(lines.len(), crate::mcp::tools::tools().len());
    for line in &lines {
        assert!(line.starts_with("nichlink."), "{line}");
        assert!(
            line.contains(" — "),
            "name, then the first sentence: {line}"
        );
        assert!(!line.contains('\n'), "one line each: {line}");
    }
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("nichlink.callgraph — Show direct static callers")),
        "{lines:?}"
    );
}

/// The workflow table says which symptom takes which call, because the measured failure was that
/// nothing said it.
/// 流程表说出"哪种症状用哪个调用"，因为量出来的失败正是没有任何地方说这件事。
#[test]
fn the_instructions_name_the_symptom_and_the_call() {
    for expected in [
        "check {face}",
        "search {literal}",
        "search {query}",
        "callgraph {function, source: true}",
        "callgraph {orphans: true}",
        "affected",
    ] {
        assert!(
            INSTRUCTIONS.contains(expected),
            "missing `{expected}`: {INSTRUCTIONS}"
        );
    }
    assert!(
        INSTRUCTIONS.len() > 700,
        "the table has to carry the mapping, not one sentence: {}",
        INSTRUCTIONS.len()
    );
}

/// A call makes its verdict an exit code: answered, refused, or malformed.
/// 一次调用把判定变成退出码：作答、拒绝、或畸形。
#[test]
fn a_call_exits_with_its_verdict() {
    let temp = std::env::temp_dir().join(format!("nichlink-client-{}", std::process::id()));
    std::fs::create_dir_all(&temp).expect("scratch");
    let root = temp.display().to_string();
    let args = |parts: &[&str]| parts.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>();

    match run_client(&args(&["--call", "nichlink.status", "--root", &root])) {
        Client::Called(code) => assert_eq!(code, 0, "a tool that answers exits 0"),
        Client::Serve => panic!("--call is a call"),
    }
    match run_client(&args(&["--call", "nichlink.no_such_tool"])) {
        Client::Called(code) => assert_eq!(code, 1, "a refusal is exit 1"),
        Client::Serve => panic!("--call is a call"),
    }
    match run_client(&args(&["--call"])) {
        Client::Called(code) => assert_eq!(code, 2, "a malformed request is exit 2"),
        Client::Serve => panic!("--call is a call"),
    }
    let _ = std::fs::remove_dir_all(&temp);
}

/// Plain `--key value` arguments carry the obvious JSON type, so the common call needs no JSON.
/// 普通 `--key value` 参数带上最显然的 JSON 类型，因此常见调用不必写 JSON。
#[test]
fn plain_arguments_carry_their_json_type() {
    assert_eq!(scalar("true"), Value::Bool(true));
    assert_eq!(scalar("false"), Value::Bool(false));
    assert_eq!(scalar("900000"), Value::from(900_000));
    assert_eq!(scalar("default"), Value::from("default"));
    assert_eq!(
        scalar("00"),
        Value::from("00"),
        "a leading zero is a string, not a number"
    );
}
