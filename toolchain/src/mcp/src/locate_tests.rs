//! Pins for `nichlink.locate`: it ranks text, it says so, and its empty answer is actionable.
//! `nichlink.locate` 的钉子：它排的是文本、它说了这一点、空答案可执行。
//!
//! The failure these exist against is the axis the seventh round measured: an agent with a symptom
//! and no way to spend words for places sends one or two extra calls (`search {literal}` after a
//! failed guess, or a `callgraph` on a name it invented). Each pin here is one half of that trade:
//! the ranking finds a doc-only match, an unmatched symptom names the root and the route back, and
//! the reply never claims more than text.
//! 这些钉子针对的失败正是第七轮量到的那条轴：手上有症状、却没有"用词换地方"的入口的代理，会多发一到
//! 两次调用（猜错之后再 `search {literal}`，或对一个自己编的名字发 `callgraph`）。这里每条钉子守着这笔
//! 交易的一半：排序能找到"只在文档里出现"的匹配；没匹配上时答案点名根与回去的路；答案从不声称超出文本。

use serde_json::json;

/// The scratch package the other tool tests use, so `locate` is read against the same tree.
/// 与其它工具测试同一个临时包，因此 `locate` 读的是同一棵树。
fn scratch(label: &str) -> std::path::PathBuf {
    crate::mcp::tools::tools_tests::scratch_package(label)
}

/// A symptom whose words live only in the doc comment still ranks that function first.
/// 词只出现在文档注释里的症状，仍然把那个函数排在第一。
#[test]
fn a_doc_only_symptom_ranks_the_function_first() {
    let root = scratch("locate-doc");
    let answer =
        super::locate(&root, &json!({"symptom": "what this function is for"})).expect("an answer");
    let first = answer
        .lines()
        .find(|line| line.trim_start().starts_with("src/"))
        .unwrap_or_default();
    assert!(
        first.contains("src/lib.rs") && first.contains("`used`"),
        "the doc's own words point at `used`: {answer}"
    );
    assert!(
        first.contains("its doc overlaps"),
        "the reason is printed with the row: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A symptom nothing matches carries the root, the route back, and no invented candidate.
/// 什么都匹配不上的症状带着根、回去的路，以及一个都不编造的候选。
#[test]
fn an_unmatched_symptom_names_the_root_and_the_route_back() {
    let root = scratch("locate-empty");
    let answer =
        super::locate(&root, &json!({"symptom": "zebra quixote walrus"})).expect("an answer");
    assert!(
        answer.starts_with(&format!("no matches in {}", root.display())),
        "the empty answer carries the root: {answer}"
    );
    assert!(
        answer.contains("search {literal}") || answer.contains("search {{literal}}"),
        "and the route back: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The ranking never claims to be a verdict, and the answer names the next call.
/// 排序从不声称自己是判定；答案还点名下一次调用。
#[test]
fn the_ranking_says_what_it_cannot_see() {
    let root = scratch("locate-bounds");
    let answer =
        super::locate(&root, &json!({"symptom": "what this function is for"})).expect("an answer");
    assert!(
        answer.contains("not covered by the ranking")
            && answer.contains("reads text only")
            && answer.contains("a hit is a place to look, not the defect"),
        "the boundary rides with the answer: {answer}"
    );
    assert!(
        answer.lines().any(|line| line.starts_with("next")),
        "the next call is named: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A call with no symptom is refused **with the accepted shape**.
/// 没给症状的调用被拒绝，**并带上可接受的形状**。
#[test]
fn a_missing_symptom_is_refused_with_the_shape() {
    let root = scratch("locate-shape");
    let error = super::locate(&root, &json!({})).expect_err("a symptom is required");
    assert!(
        error.contains("locate needs `symptom`") && error.contains("accepted shape"),
        "the refusal carries the shape: {error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
