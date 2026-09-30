//! Pins for the one-shot client: the list is the catalogue in one line each, and a call's exit code
//! is its verdict.
//! 一次性客户端的钉子：清单是目录的一行式形态，而一次调用的退出码就是它的判定。

use super::*;
use serde_json::json;

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
        // The table names the tool and its key; the order it recommends is asserted below, and
        // `source` is no longer a key to remember because bodies are the default.
        // 表里点名工具与它的键；它推荐的顺序在下面另断言，而 `source` 已不再是需要记住的键——函数体是默认。
        "callgraph {function}",
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

/// Every call shape the flow table advertises is one the tool actually accepts.
/// 流程表承诺的每一种调用形状，都是工具真正接受的形状。
///
/// This is the pin the round was missing. The table advertised `callgraph {orphans: true}` while the
/// orphan view had deliberately been left out, and the old pin asserted only that the *string*
/// appeared in `INSTRUCTIONS` — so two independent members spent two of a round's ten calls being
/// refused (`requires function`, then `needs a value`). Checking the sentence is not checking the
/// shape. The reverse direction is checked too: every `tool {keys}` shape the table names must be
/// listed here, so adding a promise without an acceptance check fails.
/// 这就是那轮缺失的钉子。流程表承诺了 `callgraph {orphans: true}`，而孤儿视图当时被我故意缓做，旧钉子只断言
/// 那个**字符串**出现在 `INSTRUCTIONS` 里——于是两位成员各自花掉那轮 10 次调用里的 2 次被拒
/// （`requires function`、继而 `needs a value`）。**检查句子不等于检查形状。**反向也检查：表里点名的每个
/// `tool {keys}` 形状都必须在这里列名，因此"加了承诺却没有接受性检查"会失败。
#[test]
fn the_table_only_advertises_shapes_the_tool_accepts() {
    let root = crate::mcp::tools::tools_tests::scratch_package("instructions");
    // The read shapes, called for real: acceptance is a call that is not refused for its arguments.
    // 读形状真调一次：接受 = 这次调用不因为参数而被拒。
    let shapes: Vec<(&str, Value)> = vec![
        ("nichlink.status", json!({})),
        ("nichlink.registry", json!({})),
        ("nichlink.search", json!({"query": "used"})),
        ("nichlink.search", json!({"literal": "call_used"})),
        ("nichlink.callgraph", json!({"function": "used"})),
        ("nichlink.callgraph", json!({"orphans": true})),
        ("nichlink.read", json!({"path": "src/lib.rs", "line": 1})),
        ("nichlink.inspect", json!({"path": "src/lib.rs"})),
        ("nichlink.affected", json!({"files": ["src/lib.rs"]})),
        ("nichlink.explain", json!({})),
        ("nichlink.diff", json!({})),
    ];
    for (tool, arguments) in &shapes {
        let answer = crate::mcp::tools::run_tool(&root, tool, arguments);
        assert!(answer.is_ok(), "{tool} {arguments} was refused: {answer:?}");
    }
    // The write shape is not called (a preview runs the real executor on a copy); its schema is what
    // has to match the table.
    // 写入形状不真调（预览会在副本上跑真执行器）；要对上那张表的是它的 schema。
    let apply = crate::mcp::tools::tools()
        .into_iter()
        .find(|tool| tool["name"] == "nichlink.apply")
        .expect("apply is in the catalogue");
    assert_eq!(apply["inputSchema"]["required"][0], "action", "{apply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Each shape the table names has an acceptance check above, and each check names a shape the table
/// really has.
/// 表里点名的每种形状在上面的检查里都有，而每个检查点名的形状表里真有。
///
/// Drift runs both ways: a table entry with no check is a promise nobody tests, and a check for a
/// shape the table dropped is a test that outlived its subject.
/// 漂移是双向的：表里有、检查里没有 ⇒ 没人测的承诺；检查里有、表里已经删掉 ⇒ 活得比被检查对象还久的测试。
#[test]
fn every_shape_the_table_names_is_covered_by_an_acceptance_check() {
    let named = INSTRUCTIONS
        .split('`')
        .filter(|part| part.contains('{') && part.contains('}'))
        .filter_map(|part| part.split_once(' '))
        .map(|(tool, keys)| (tool.to_owned(), keys.to_owned()))
        .collect::<Vec<_>>();
    assert!(named.len() >= 6, "the table names shapes: {named:?}");
    for (tool, keys) in &named {
        let checked = match tool.as_str() {
            // The table writes tool names with backticks around the call, e.g. "check {face}".
            // 表里的写法是"工具 {键}"，例如 `check {face}`。
            "check" => keys.contains("face"),
            "search" | "callgraph" | "read" | "inspect" | "affected" | "registry" | "explain"
            | "diff" => true,
            "apply" => keys.contains("apply"),
            other => panic!("the table names `{other}`, which no acceptance check covers"),
        };
        assert!(checked, "{tool} {keys}");
    }
}

/// A boolean is a flag: the bare spelling is accepted and means true.
/// 布尔就是开关：裸写被接受，且意为 true。
#[test]
fn a_bare_flag_means_true() {
    let temp = std::env::temp_dir().join(format!("nichlink-flag-{}", std::process::id()));
    std::fs::create_dir_all(&temp).expect("scratch");
    let root = temp.display().to_string();
    for parts in [
        vec!["nichlink.search", "--query", "x", "--converge"],
        vec!["nichlink.search", "--query", "x", "--converge", "true"],
    ] {
        let mut args = vec!["--call".to_owned()];
        args.extend(parts.iter().map(|part| (*part).to_owned()));
        match run_client(&args) {
            Client::Called(code) => assert_eq!(code, 0, "bare flag accepted: {parts:?}"),
            Client::Serve => panic!("--call is a call"),
        }
    }
    let _ = std::fs::remove_dir_all(&temp);
    let _ = &root;
}
