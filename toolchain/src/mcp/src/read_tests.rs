//! Tests for `nichlink.read`'s three shapes: the window, the explicit range, and the
//! whole file — plus the total-line header every one of them carries.
//! `nichlink.read` 三种形状的测试：窗口、显式区间与整文件——外加它们每一个都携带的总行数标头。
//!
//! The measured failure these exist against is the shape the window forced: a 473-line
//! file took `ceil(473/81)` = six calls, and no call said how much was left, because the
//! header carried no total. So the nails are about *one* call answering a whole file, a
//! window still stating the total, and a reply past the cap going through the one
//! truncation outlet rather than being quietly short.
//! 这些测试所针对的实测失败正是窗口逼出来的形状：一个 473 行的文件要 `ceil(473/81)` = 六次调用，而没有
//! 一次说明还剩多少，因为标头不带总数。因此钉子关心的是：**一次**调用回答一整个文件；窗口仍然说出总数；
//! 越过上限的回复走唯一那个截断出口，而不是无声地变短。

use std::path::PathBuf;

use serde_json::json;

use super::{MAX_FULL_READ_LINES, read_source};
use crate::mcp::truncation::PHRASE;

/// A throwaway source root holding one file.
/// 一个装有一份源码文件的一次性源码根。
struct Root(PathBuf);

impl Drop for Root {
    /// Remove the fixture tree once the test that built it is done.
    /// 构建它的测试结束后删除夹具树。
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A source root whose `src/lib.rs` has `lines` numbered comment lines.
/// 一个源码根，其 `src/lib.rs` 有 `lines` 行编号注释。
fn root_with_lines(label: &str, lines: usize) -> Root {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-read-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture directory");
    let body = (1..=lines)
        .map(|number| format!("// line {number}"))
        .collect::<Vec<_>>()
        .join("\n");
    let source = if lines == 0 {
        String::new()
    } else {
        format!("{body}\n")
    };
    std::fs::write(root.join("src/lib.rs"), source).expect("fixture file");
    Root(root)
}

/// The content lines of a reply: everything after the header.
/// 一份回复的内容行：标头之后的全部。
fn body_lines(reply: &str) -> Vec<&str> {
    reply.lines().skip(1).collect()
}

/// The `start`/`end`/`total` triple a header states.
/// 标头声明的 `start`/`end`/`total` 三元组。
///
/// Every step names what it could not read, because the mutation this is aimed at is the
/// header losing its total: the failure has to point at the header, not at a parse.
/// 每一步都点名它读不出什么，因为这里瞄准的变异正是标头丢掉总数：失败必须指向标头，而不是一次解析。
fn header(reply: &str) -> (usize, usize, usize) {
    let line = reply.lines().next().expect("a header");
    let (_, rest) = line
        .rsplit_once(':')
        .unwrap_or_else(|| panic!("the header must be `path:START-END (N lines)`; got `{line}`"));
    let (range, total) = rest.split_once(" (").unwrap_or_else(|| {
        panic!("the header must state the file's total as `path:START-END (N lines)`; got `{line}`")
    });
    let (start, end) = range
        .split_once('-')
        .unwrap_or_else(|| panic!("the header must carry `START-END`; got `{line}`"));
    let total = total.trim_end_matches(')').trim_end_matches(" lines");
    let number = |part: &str| {
        part.parse()
            .unwrap_or_else(|_| panic!("the header must carry whole numbers; got `{line}`"))
    };
    (number(start), number(end), number(total))
}

/// The first nail: a 473-line file comes back from **one** call, and the header states
/// the file's total so the caller can see it did.
/// 第一枚钉子：一个 473 行的文件由**一次**调用带回，而标头声明了文件的总行数，调用方因此看得见它读完了。
///
/// The old shape could not do this at any price: `ceil(473/81)` calls, with no call able
/// to say whether it was the last one. The mutation this pins is the header losing the
/// total — then the count below has nothing to compare against and the nail goes red.
/// 旧形状无论花多大代价都做不到：`ceil(473/81)` 次调用，且没有一次说得出它是不是最后一次。这里钉住的变异
/// 是标头丢掉总数——那样下面的计数就没有可比的东西，钉子随之变红。
#[test]
fn one_whole_file_call_returns_all_473_lines() {
    let fixture = root_with_lines("whole", 473);
    let reply = read_source(&fixture.0, &json!({"path": "src/lib.rs", "whole": true}))
        .expect("the whole file reads");
    let (start, end, total) = header(&reply);
    assert_eq!(
        (start, end, total),
        (1, 473, 473),
        "one call must cover the whole file and say its total: {reply}"
    );
    let body = body_lines(&reply);
    assert_eq!(
        body.len(),
        473,
        "every line of the file must be printed by this one call"
    );
    assert!(reply.contains("// line 473"), "{reply}");
}

/// The second nail: a window read says how much file is left.
/// 第二枚钉子：窗口读取说出还剩多少文件。
///
/// The window is still the default and still small — that is the maintainer's rule, a
/// bridge does not feed the whole tree by default — but the header now carries the
/// total, so a caller can tell a window from the file.
/// 窗口仍是默认、仍然小——那是维护者的原则，本桥默认不投喂整棵树——但标头现在带着总数，调用方因此分得出
/// 窗口与整个文件。
#[test]
fn a_window_read_states_the_file_total() {
    let fixture = root_with_lines("window", 473);
    let reply =
        read_source(&fixture.0, &json!({"path": "src/lib.rs", "line": 200})).expect("a window");
    let (start, end, total) = header(&reply);
    assert_eq!(
        total, 473,
        "a window must still state the file total: {reply}"
    );
    assert!(
        end - start < 81,
        "the default window stays the small reply it was: {reply}"
    );
    assert!(start <= 200 && 200 <= end, "{reply}");
}

/// The third nail: past the cap the reply goes through the one truncation outlet.
/// 第三枚钉子：越过上限时，回复走唯一那个截断出口。
#[test]
fn a_reply_past_the_cap_carries_the_one_truncation_sentence() {
    let fixture = root_with_lines("truncated", MAX_FULL_READ_LINES + 100);
    let reply = read_source(&fixture.0, &json!({"path": "src/lib.rs", "whole": true}))
        .expect("the read answers");
    assert!(
        reply.contains(PHRASE),
        "a short answer must say it is short: {reply}"
    );
    assert!(
        reply.contains(&format!(
            "{} of {} lines withheld at the limit of {MAX_FULL_READ_LINES}",
            MAX_FULL_READ_LINES + 100 - MAX_FULL_READ_LINES,
            MAX_FULL_READ_LINES + 100
        )),
        "the sentence names the withheld count and the cap that cut it: {reply}"
    );
    let (start, end, total) = header(&reply);
    assert_eq!(
        (start, end, total),
        (1, MAX_FULL_READ_LINES + 100, MAX_FULL_READ_LINES + 100),
        "the header still describes the file, not the shortened reply: {reply}"
    );
}

/// `lines` reads exactly the forward range it named, and the header keeps the total.
/// `lines` 精确读取它点名的那段朝前区间，而标头保留总行数。
#[test]
fn lines_reads_an_explicit_range() {
    let fixture = root_with_lines("range", 400);
    let reply = read_source(
        &fixture.0,
        &json!({"path": "src/lib.rs", "lines": "120-260"}),
    )
    .expect("the range reads");
    let (start, end, total) = header(&reply);
    assert_eq!((start, end, total), (120, 260, 400), "{reply}");
    let body = body_lines(&reply);
    assert_eq!(body.len(), 141, "{reply}");
    assert!(body[0].contains("// line 120"), "{reply}");
    assert!(body[140].contains("// line 260"), "{reply}");
}

/// A range past the end of the file is clamped to it, and the header says so.
/// 越过文件末尾的区间被夹到文件末尾，而标头说明了这一点。
#[test]
fn a_range_past_the_end_is_clamped_to_the_file() {
    let fixture = root_with_lines("clamped", 10);
    let reply = read_source(&fixture.0, &json!({"path": "src/lib.rs", "lines": "8-900"}))
        .expect("the range reads");
    let (start, end, total) = header(&reply);
    assert_eq!((start, end, total), (8, 10, 10), "{reply}");
}

/// The three shapes are exclusive, and mixing them is refused with the spelling rather
/// than answered by letting one win.
/// 三种形状互斥，混用会被拒绝并给出正确拼法，而不是让其中一个胜出。
#[test]
fn mixing_the_three_shapes_is_refused() {
    let fixture = root_with_lines("mixed", 20);
    for (arguments, expected) in [
        (
            json!({"path": "src/lib.rs", "whole": true, "lines": "1-2"}),
            "ask for one of them",
        ),
        (
            json!({"path": "src/lib.rs", "lines": "1-2", "line": 3}),
            "ask for one of them",
        ),
        (
            json!({"path": "src/lib.rs", "lines": "1-2", "context": 3}),
            "ask for one of them",
        ),
        (
            json!({"path": "src/lib.rs", "lines": "2-1"}),
            "must run forward",
        ),
        (json!({"path": "src/lib.rs", "lines": "0-4"}), "1-based"),
        (json!({"path": "src/lib.rs", "lines": "1..4"}), "START-END"),
    ] {
        let error = read_source(&fixture.0, &arguments).expect_err("the request is refused");
        assert!(error.contains(expected), "{arguments}: {error}");
    }
}

/// A line number a caller sends is unbounded; the range must stay forward and inside
/// the file whatever it is.
/// 调用方发来的行号没有上界；无论它是什么，区间都必须朝前且落在文件内。
///
/// `center + context` used to overflow: debug panicked, release produced `start > end`
/// with an empty body and `isError: false`.
/// `center + context` 过去会溢出：debug 直接 panic，release 产出 `start > end`、正文为空、却仍报
/// `isError: false`。
#[test]
fn a_huge_line_number_still_yields_a_forward_range() {
    let fixture = root_with_lines("huge", 2);
    for line in [u64::MAX, u64::MAX / 2, u64::MAX / 4, 1] {
        let reply = read_source(&fixture.0, &json!({"path": "src/lib.rs", "line": line}))
            .expect("the read succeeds");
        let (start, end, total) = header(&reply);
        assert!(start <= end, "line {line} inverted the range: {reply}");
        assert!(end <= 2, "line {line} read past the file: {reply}");
        assert_eq!(total, 2, "{reply}");
    }
}

/// An empty file is an answer with a header, not a gap.
/// 空文件是带标头的答案，而不是空白。
#[test]
fn an_empty_file_still_has_a_header() {
    let fixture = root_with_lines("empty", 0);
    let reply = read_source(&fixture.0, &json!({"path": "src/lib.rs", "whole": true}))
        .expect("the read answers");
    assert_eq!(header(&reply).2, 0, "{reply}");
    assert!(
        !reply.contains(PHRASE),
        "an empty file withheld nothing: {reply}"
    );
}

/// The description is the contract: `tools/list` has to name the two new arguments and
/// the total-line header, because an agent can only call what it is told about.
/// 描述就是契约：`tools/list` 必须点名两个新参数与总行数标头，因为代理只能调用它被告知的东西。
#[test]
fn the_read_description_names_the_shapes_and_the_total() {
    let listed = crate::mcp::tools::tools();
    let read = listed
        .iter()
        .find(|tool| tool["name"] == "nichlink.read")
        .expect("nichlink.read is advertised");
    let description = read["description"]
        .as_str()
        .unwrap_or_else(|| panic!("nichlink.read has no description: {read}"));
    for expected in ["whole", "lines", "N lines", "total"] {
        assert!(
            description.contains(expected),
            "`{expected}` is part of nichlink.read's contract: {description}"
        );
    }
    for key in ["whole", "lines", "line", "context"] {
        assert!(
            read["inputSchema"]["properties"].get(key).is_some(),
            "nichlink.read does not advertise `{key}`: {read}"
        );
    }
}

/// The whole-file shape is reachable exactly as `tools/list` describes it, so the
/// advertised spelling is the one the implementation reads.
/// 整文件形状可以完全按 `tools/list` 描述的拼法到达，因此声明出来的拼法就是实现读取的那一种。
#[test]
fn the_advertised_spelling_is_the_one_read() {
    let fixture = root_with_lines("round-trip", 5);
    let reply = read_source(&fixture.0, &json!({"path": "src/lib.rs", "lines": "2-4"}))
        .expect("the advertised `lines` spelling reads");
    assert_eq!(header(&reply), (2, 4, 5), "{reply}");
}
