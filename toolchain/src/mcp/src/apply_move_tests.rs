//! Pins for `move`: the refusals, and the three pieces of logic a move rests on.
//! `move` 的钉子：它拒绝什么，以及一次搬动所依赖的三处逻辑。
//!
//! The **full path** (build a host, move a face, `cargo check` the moved tree) needs a cargo run, so it
//! is run as an end-to-end demonstration on a real project — the transcript is in the commit that
//! introduced this action. What lives here is what that demonstration cannot keep honest on its own: the
//! **line-level** `parent:` rewrite and its refusals, the alias the tree derives, and the record
//! directory's retention rule.
//! **完整路径**（建宿主、搬一个面、对搬完的树 `cargo check`）要跑 cargo，因此它作为真项目上的端到端演示
//! 运行——过程记录在引入本动作的那笔提交里。住在这里的是那场演示自己守不住的东西：**行级**的 `parent:`
//! 改写与它的拒绝、树推导出的别名，以及记录目录的保留规则。

use serde_json::json;

use super::{KEEP_RECORDS, next_number, prune, rewrite_parent_line, run_move, under};

/// A throwaway directory, named for the **module** as well as the label.
/// 一个一次性目录，名字里既有**模块**也有标签。
///
/// The module namespace is not decoration: two test files whose labels repeat used to compute the same
/// scratch path, and each fixture's opening `remove_dir_all` deleted the directory the other test was
/// using — the symptom is a gate that goes red on a different test every run (audit `M7`, §M7.57).
/// 模块命名空间不是装饰：两个标签重复的测试文件过去会算出同一个 scratch 路径，而每个夹具开头那句
/// `remove_dir_all` 会删掉另一个测试正在用的目录——症状是同一道门禁每次红在另一条测试上
/// （审计 `M7`，§M7.57）。
fn scratch(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let module = module_path!().replace("::", "-");
    let root = std::env::temp_dir().join(format!("xirang-{module}-{label}-{sequence}"));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch root");
    root
}

/// The refusal text of one call, without asking the shared `Outcome` to be `Debug`.
/// 一次调用的拒绝文本；不去要求共享的 `Outcome` 实现 `Debug`。
fn refusal(result: Result<super::Outcome, String>) -> String {
    match result {
        Ok(_) => panic!("this request was expected to be refused"),
        Err(refused) => refused,
    }
}

/// A request without `node` is refused with the shape to paste, built from the tree.
/// 不带 `node` 的请求被拒绝，并给出可粘贴的形状——形状由那棵树搭出来。
#[test]
fn a_request_without_a_node_names_the_shape() {
    let root = scratch("shape");
    let refused = refusal(run_move(&root, &root, false, &json!({"to": "root"})));
    assert!(
        refused.contains("`node`") && refused.contains("\"action\":\"move\""),
        "the refusal names the key and the accepted shape: {refused}"
    );
    // The other direction: `to` is its own key, and the sentence has to name that one when it is the
    // missing half.
    // 反向：`to` 是另一个键，缺的是它时那句话必须点名它。
    let refused = refusal(run_move(&root, &root, false, &json!({"node": "root/a"})));
    assert!(
        refused.contains("`to`"),
        "the refusal names the missing key: {refused}"
    );
}

/// A face declares exactly one parent, so a file that spells `parent:` twice is refused by name.
/// 一个面只声明一个父级，因此把 `parent:` 拼了两次的文件会被点名拒绝。
#[test]
fn the_parent_line_is_found_by_line_and_never_guessed() {
    let spelling = "crate::input::NODE_ID";
    let one = "crate::input_object! {\n    kind: Tile,\n    parent: crate::board::NODE_ID,\n}\n";
    let edit = rewrite_parent_line(one, spelling).expect("one parent line is enough");
    assert!(
        edit.before.contains("crate::board::NODE_ID"),
        "the line as it stands is kept"
    );
    assert!(
        edit.after.contains("crate::input::NODE_ID"),
        "the replacement is the new parent's typed spelling: {}",
        edit.after
    );

    let none = "crate::input_object! {\n    kind: Tile,\n}\n";
    let refused = rewrite_parent_line(none, spelling).expect_err("no parent line is refused");
    assert!(
        refused.contains("0 time(s)"),
        "the refusal counts what it found: {refused}"
    );

    let twice = "parent: crate::a::NODE_ID,\nparent: crate::b::NODE_ID,\n";
    let refused = rewrite_parent_line(twice, spelling).expect_err("two are refused");
    assert!(
        refused.contains("2 time(s)"),
        "the refusal counts what it found: {refused}"
    );

    // A spelling the tree never writes is refused rather than rewritten: this action knows the two
    // spellings `renderer` emits, and a third one would be a guess.
    // 这棵树从不写的拼法被拒绝而不是被改写：本动作认识 `renderer` 发出的那两种，第三种就是猜。
    let strange = "parent: some_macro!(Tile),\n";
    let refused = rewrite_parent_line(strange, spelling).expect_err("an unknown shape is refused");
    assert!(
        refused.contains("neither a typed"),
        "the refusal says which shapes it knows: {refused}"
    );
}

/// The root's parent line is the other spelling the tree writes, and the root is a destination.
/// 根的 parent 行是这棵树写的另一种拼法，而根本身也是一个目的地。
#[test]
fn the_root_is_a_destination_with_its_own_spelling() {
    let typed = "    parent: crate::board::NODE_ID,\n";
    let edit =
        rewrite_parent_line(typed, super::ROOT_PARENT_SPELLING).expect("the root is a destination");
    assert_eq!(
        edit.after.trim(),
        "parent: crate::root_node_id(crate::XIRANG_NAMESPACE),",
        "moving to the root writes the root's own spelling, keeping the comma"
    );
    let already_root = "    parent: crate::root_node_id(crate::XIRANG_NAMESPACE),\n";
    rewrite_parent_line(already_root, super::ROOT_PARENT_SPELLING)
        .expect("the root's spelling is accepted as a source too");
}

/// Whole segments only: `board_faster` is not under `board`.
/// 只按整段比较：`board_faster` 不在 `board` 之下。
#[test]
fn a_claim_is_matched_by_segment_not_by_string_prefix() {
    assert!(under("board", "board"));
    assert!(under("board::object::tile2", "board"));
    assert!(
        !under("board_faster", "board"),
        "a longer segment that merely starts the same way is a different subtree"
    );
    assert!(!under("boardroom::tile", "board"));
}

/// The record directory keeps the most recent five, and numbers the next one after the highest.
/// 记录目录只保留最近五条，并把下一条编号取在最大值之后。
#[test]
fn the_records_keep_the_most_recent_five() {
    let root = scratch("records");
    let directory = root.join("moves");
    for number in 1..=7u32 {
        std::fs::create_dir_all(directory.join(number.to_string())).expect("record directory");
    }
    assert_eq!(
        next_number(&directory),
        8,
        "the next number follows the highest"
    );
    prune(&directory, KEEP_RECORDS);
    let mut left: Vec<u32> = std::fs::read_dir(&directory)
        .expect("records")
        .flatten()
        .filter_map(|entry| entry.file_name().to_string_lossy().parse::<u32>().ok())
        .collect();
    left.sort_unstable();
    assert_eq!(
        left,
        vec![3, 4, 5, 6, 7],
        "the oldest were dropped, not the newest"
    );
    let _ = std::fs::remove_dir_all(&root);
}
