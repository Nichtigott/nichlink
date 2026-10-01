//! Pins for `nichlink.why`: it gathers the upstream facts in one call, and it names what it left out.
//! `nichlink.why` 的钉子：一次收齐上游事实，并点名它没答的部分。
//!
//! The measured failure these exist against is the hop count: a symptom sent agents to `callgraph`,
//! then to `read` for the contract, then to the registry — 5, 6, 5 and 8 instrument calls against the
//! control's 5, 2, 2 and 2 on the four injected-defect questions. Each pin here is one of the facts
//! that used to cost its own call.
//! 这些钉子针对的失败是跳数：一个症状把代理依次送去 `callgraph`、`read`（看契约）、注册面 —— 四道注入
//! 缺陷因而花了 5、6、5、8 次仪器调用，而对照是 5、2、2、2。这里每条钉子都是过去各自要花一次调用的
//! 事实之一。

use serde_json::json;

/// The scratch package the other tool tests use.
/// 与其它工具测试同一个临时包。
fn scratch(label: &str) -> std::path::PathBuf {
    crate::mcp::tools::tools_tests::scratch_package(label)
}

/// A `path:line` inside a definition answers with its contract, its callers, and what it left out.
/// 定义内的 `路径:行号` 会答出契约、调用者，以及它没答的部分。
#[test]
fn a_line_answers_with_contract_callers_and_bounds() {
    let root = scratch("why-line");
    let answer = super::why(&root, &json!({"at": "src/lib.rs:3"})).expect("an answer");
    assert!(
        answer.contains("the definition `used`"),
        "the line is placed in its definition: {answer}"
    );
    assert!(
        answer.contains("contract") && answer.contains("What this function is for."),
        "the contract rides along: {answer}"
    );
    assert!(
        answer.contains("call_used") && answer.contains("callers"),
        "the callers ride along: {answer}"
    );
    assert!(
        answer.contains("not covered here") && answer.contains("check {face}"),
        "the bounds name the tools that answer the rest: {answer}"
    );
}

/// A file with no ledger says so rather than staying silent about adoption.
/// 没有台账的文件会说出来，而不是对采信保持沉默。
#[test]
fn adoption_is_answered_even_when_there_is_no_ledger() {
    let root = scratch("why-ledger");
    let answer = super::why(&root, &json!({"at": "src/lib.rs:3"})).expect("an answer");
    assert!(
        answer.contains("no ledger at .nichlink/adopted/entries"),
        "the absence is stated: {answer}"
    );
}

/// A malformed `at` is refused **with the accepted shape**, and a node question is redirected.
/// 形状不对的 `at` 被拒绝并**带上可接受形状**；节点类问题被指到 `explain`。
#[test]
fn a_malformed_at_is_refused_with_the_shape() {
    let root = scratch("why-shape");
    let shape = super::why(&root, &json!({"at": "src/lib.rs"})).expect_err("a line is required");
    assert!(
        shape.contains("`path:line`") && shape.contains("accepted shape"),
        "the refusal carries the shape: {shape}"
    );
    let node = super::why(&root, &json!({})).expect_err("`at` is required");
    assert!(
        node.contains("explain {node}") && node.contains("accepted shape"),
        "a node question is redirected: {node}"
    );
}

/// A line no function covers lists what the file does define, instead of guessing one.
/// 没有任何函数覆盖的行会列出该文件确实定义了什么，而不是猜一个。
#[test]
fn a_line_outside_every_definition_lists_the_definitions() {
    let root = scratch("why-outside");
    let answer = super::why(&root, &json!({"at": "src/lib.rs:1"})).expect("an answer");
    assert!(
        answer.contains("no function covers") && answer.contains("used"),
        "the index is printed rather than a guess: {answer}"
    );
}
