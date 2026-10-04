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

/// Known defect (audit r3), pinned as it behaves today: a symptom whose words live **only** in a
/// function body's string literal finds nothing at all. The per-function corpus is the name and
/// the doc comments; the one body reach is the whole-phrase bonus, and a `format!` hole breaks
/// even that, so the answer is the empty one while the words sit in the tree.
/// 已登记缺陷（审计 r3），按今天的样子钉住：词只活在**函数体字符串字面量**里的症状一个也找不到。
/// 逐函数的语料只有名字与文档注释；唯一触及函数体的是整句加成，而一个 `format!` 占位符连它也打破，
/// 于是答案是空的那一种，尽管这些词就在树里。
#[test]
fn a_symptom_that_lives_only_in_a_body_literal_finds_nothing() {
    let root = scratch("locate-body");
    std::fs::write(
        root.join("src/limit.rs"),
        "pub fn enforce(value: u32) -> Result<(), String> {\n\
         \x20   if value > 9 {\n\
         \x20       return Err(format!(\"flurb quota {} exceeded\", value));\n\
         \x20   }\n\
         \x20   Ok(())\n\
         }\n",
    )
    .expect("fixture source");
    let answer =
        super::locate(&root, &json!({"symptom": "flurb quota exceeded"})).expect("an answer");
    // Fix: score the masked body text per function in `locate`'s ranking loop; flip this then.
    // 修法：在 `locate` 的排序循环里对掩码后的函数体文本逐函数计分；修复后翻转这条断言。
    assert!(
        answer.starts_with("no matches in"),
        "the words are a string literal in `enforce`'s body, and the corpus is name + doc only: {answer}"
    );
    assert!(
        !answer.contains("enforce"),
        "the function that produces the message is never named: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Known defect (audit r1), pinned as it behaves today: the ranking never demotes a test file —
/// `looks_like_a_test` only appends a `test file` reason to the row — so when a symptom hits five
/// test files and one real source file, all five visible slots are tests and the file that
/// produces the symptom is withheld unnamed.
/// 已登记缺陷（审计 r1），按今天的样子钉住：排序从不给测试文件降权——`looks_like_a_test` 只是给行尾
/// 追加一个 `test file` 理由——于是当症状命中五个测试文件与一个真因源文件时，五个可见槽位全是测试，
/// 而产生症状的那个文件被匿名扣下。
#[test]
fn the_five_visible_slots_can_all_be_test_files() {
    let root = scratch("locate-crowded");
    std::fs::write(
        root.join("src/grumble.rs"),
        "pub fn enforce(level: u32) -> Result<(), String> {\n\
         \x20   if level > 3 {\n\
         \x20       return Err(\"grumble quota exceeded\".to_owned());\n\
         \x20   }\n\
         \x20   Ok(())\n\
         }\n",
    )
    .expect("fixture source");
    std::fs::create_dir_all(root.join("tests")).expect("fixture test dir");
    for index in 0..5 {
        std::fs::write(
            root.join(format!("tests/t{index}.rs")),
            "#[test]\nfn grumble_quota_exceeded_surfaces() {\n\
             \x20   assert_eq!(\"grumble quota exceeded\", \"grumble quota exceeded\");\n\
             }\n",
        )
        .expect("fixture test");
    }
    let answer =
        super::locate(&root, &json!({"symptom": "grumble quota exceeded"})).expect("an answer");
    assert!(answer.contains("6 candidate(s)"), "{answer}");
    assert_eq!(
        answer.matches("test file").count(),
        5,
        "all five visible rows are test files: {answer}"
    );
    // Fix: demote test rows in `locate`'s `rows.sort_by` (or cap their share of the visible
    // slots); flip this assertion then.
    // 修法：在 `locate` 的 `rows.sort_by` 里给测试行降权（或限制它们占可见槽位的份额）；修复后翻转
    // 这条断言。
    assert!(
        !answer.contains("src/grumble.rs"),
        "the file that produces the symptom is withheld unnamed: {answer}"
    );
    assert!(
        answer.contains("withheld"),
        "the cap says it cut one row, without naming which: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A symptom about a whole file gets the file-level shapes, not just "try other words".
/// 关于**一整个文件**的症状得到文件级的形状，而不只是"换几个词试试"。
///
/// Measured (W8, h4): the arm spent a call here on a symptom that was about one file's algorithms
/// contradicting their own documentation, and the reply named neither that this is a file-level
/// question nor the tools that answer one. Zero matches is the decidable signal.
/// 量到的（W8 的 h4）：那一臂在这里花掉一次调用，症状说的是一个文件的算法与它自己的文档相反，而回复既没说
/// 这是文件级问题，也没说哪些工具答得了。零命中就是可判定的信号。
#[test]
fn a_symptom_about_a_whole_file_names_the_file_level_shapes() {
    let root = scratch("locate-file-level");
    let answer = super::locate(&root, &json!({"symptom": "zebra quixote walrus"}))
        .expect("a symptom with no match still answers");
    for shape in [
        "file-level question",
        "--call digest --file <file>",
        "--call read --path <file> --whole",
        "--call check --face <face>",
    ] {
        assert!(
            answer.contains(shape),
            "the pointer names `{shape}`: {answer}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The answer routes by the shape of the question, and the call it names is the top candidate's.
/// 答案按问题的形状路由，而它点名的调用用的是排名第一的那个候选。
///
/// Audit `W4-7`: an "understanding" question (what does this file do) is answered by the whole file,
/// while a structural one (what is in it, who calls it) is answered by the bounded rows — and the
/// answer knows which file it ranked first, so the call it prints is one a reader can paste rather
/// than a template with `{path}` in it.
/// 审计 `W4-7`："理解型"问题（这个文件是干什么的）由整份文件回答，而"结构型"问题（里面有什么、谁调用它）
/// 由有界的那几行回答——而答案知道自己把哪个文件排第一，因此它印出来的调用是读者可以照抄的，而不是一个
/// 带 `{path}` 的模板。
#[test]
fn the_routing_names_the_top_candidate_and_the_whole_file_call() {
    let root = scratch("locate-routing");
    let answer = super::locate(&root, &json!({"symptom": "used"})).expect("a ranked answer");
    assert!(
        answer.contains("shape  understanding question"),
        "the routing rides on the answer: {answer}"
    );
    assert!(
        answer.contains("--call read --path src/lib.rs --whole"),
        "and it names the file the ranking put first, whole: {answer}"
    );
    assert!(
        answer.contains("--call read --path src/lib.rs --line ")
            && answer.contains("--call callgraph --function used"),
        "as does the `next` line: {answer}"
    );
    assert!(
        !answer.lines().any(|line| line.contains("{path}")),
        "no template survives where a real name was available: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
