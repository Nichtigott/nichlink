//! `from_jsonl`: one record per line, and nothing may be silently lost.
//! `from_jsonl`：每行一条记录，不许有任何东西被静默丢掉。

use super::MirGraph;

/// Two records glued onto one line are a malformed line, not a shorter graph.
/// 粘在同一行上的两条记录是格式错误的行，而不是一个更短的图。
///
/// Red before the fix: the first `}` returned success and the rest of the line was never
/// read, so the second record vanished with no line number and no diagnostic. This file
/// is the call-graph evidence, so a silently shorter graph is a wrong delta nobody can
/// trace (audit `KRN-K-15`).
/// 修前为红：第一个 `}` 就返回成功，行内剩余字节从未被读，于是第二条记录消失，既没有行号也没有
/// 诊断。这个文件就是调用图证据，因此静默变短的图意味着一个无人能追查的错误 delta（审计
/// `KRN-K-15`）。
#[test]
fn a_second_record_on_the_same_line_is_refused() {
    let one = "{\"kind\":\"function\",\"name\":\"a\"}";
    let other = "{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"b\",\"mir_line\":1}";

    let error = MirGraph::from_jsonl(&format!("{one}{other}")).expect_err("a glued record");
    assert_eq!(
        error.line, 1,
        "the line number points at the line: {error:?}"
    );
    assert!(
        error.message.contains("trailing text"),
        "the refusal names the leftover bytes: {error:?}"
    );

    // The same two records, one per line, are still read as two.
    // 同样两条记录，各占一行时仍被读成两条。
    let graph = MirGraph::from_jsonl(&format!("{one}\n{other}\n")).expect("two lines");
    assert_eq!(graph.functions.len(), 1);
    assert_eq!(graph.calls.len(), 1);
}

/// Trailing whitespace after the closing brace is not "trailing text".
/// 闭合花括号之后的空白不是"残留文本"。
#[test]
fn whitespace_after_a_record_is_not_trailing_text() {
    let line = "{\"kind\":\"function\",\"name\":\"a\"}   \n";
    let graph = MirGraph::from_jsonl(line).expect("a padded line is still one record");
    assert_eq!(graph.functions.len(), 1);
}
