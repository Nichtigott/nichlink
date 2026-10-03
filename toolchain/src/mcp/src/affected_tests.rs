//! Pins for `nichlink.affected`: what a changed file reaches, and what it says when nothing
//! reaches it.
//! `nichlink.affected` 的钉子：一个改动过的文件能触到什么，以及什么都没有时它说什么。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;

use super::affected;

/// A throwaway single package: one source file with a definition, and one test file calling it.
/// 一个一次性单包：一个带定义的文件，和一个调用它的测试文件。
fn package(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-affected-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_fixture(
        &root.join("Cargo.toml"),
        "[package]\nname = \"affected-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_fixture(&root.join("src/lib.rs"), "// entry\n");
    write_fixture(&root.join("src/shared.rs"), "pub fn shared_helper() {}\n");
    write_fixture(
        &root.join("tests/uses_shared.rs"),
        "#[test]\nfn it_calls_shared() {\n    shared_helper();\n}\n",
    );
    root
}

fn write_fixture(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture dirs");
    std::fs::write(path, text).expect("fixture file");
}

#[test]
fn a_changed_file_names_the_tests_that_call_its_definitions() {
    let root = package("named");
    let text = affected(&root, &json!({"files": ["src/shared.rs"]})).expect("an answer");
    assert!(text.contains("src/shared.rs: 1 definition(s)"), "{text}");
    assert!(
        text.contains("tests/uses_shared.rs"),
        "the test that calls the definition is the answer: {text}"
    );
}

#[test]
fn a_changed_file_no_test_reaches_says_so() {
    let root = package("unreached");
    write_fixture(&root.join("src/quiet.rs"), "pub fn quiet_helper() {}\n");
    let text = affected(&root, &json!({"files": ["src/quiet.rs"]})).expect("an answer");
    assert!(
        text.contains("none reference these definitions"),
        "an empty list would read as `nothing to run` rather than `run the suite`: {text}"
    );
}

#[test]
fn a_path_outside_the_index_is_named_rather_than_dropped() {
    let root = package("missing");
    let text = affected(&root, &json!({"files": ["src/nowhere.rs"]})).expect("an answer");
    assert!(text.contains("not in the index"), "{text}");
}

#[test]
fn an_empty_file_list_is_a_usage_error() {
    let root = package("empty");
    let error = affected(&root, &json!({"files": []})).expect_err("no paths is a usage error");
    assert!(error.contains("at least one path"), "{error}");
}

/// Known defect T-27, pinned as it behaves today: a file whose only definition is `Store::new`
/// "affects" a test that calls `Entry::new`, because the index records the bare `new` on both
/// sides and `is_call_to` cannot tell the two owners apart.
/// 已登记缺陷 T-27，按今天的样子钉住：只定义了 `Store::new` 的文件会"影响"一个只调用 `Entry::new`
/// 的测试，因为索引两侧记的都是裸名 `new`，`is_call_to` 分不出两个属主。
#[test]
fn a_same_named_method_on_another_type_is_reported_as_affected() {
    let root = package("t27");
    write_fixture(
        &root.join("src/entry.rs"),
        "pub struct Entry;\nimpl Entry {\n    pub fn new() -> Entry { Entry }\n}\n",
    );
    write_fixture(
        &root.join("src/store.rs"),
        "pub struct Store;\nimpl Store {\n    pub fn new() -> Store { Store }\n}\n",
    );
    write_fixture(
        &root.join("tests/uses_entry.rs"),
        "#[test]\nfn it_builds_an_entry() {\n    let _ = Entry::new();\n}\n",
    );
    let text = affected(&root, &json!({"files": ["src/store.rs"]})).expect("an answer");
    // The false positive itself: nothing in `tests/uses_entry.rs` touches `Store`.
    // Flip this assertion when the match carries the owning type.
    // 误报本体：`tests/uses_entry.rs` 里没有任何东西碰 `Store`。当匹配带上属主类型时翻转这条断言。
    assert!(
        text.contains("tests/uses_entry.rs"),
        "T-27: the test calling `Entry::new` is listed for a change to `Store::new`: {text}"
    );
    assert!(
        !text.contains("uses_shared"),
        "the unrelated test is not listed, so the answer is not just every test: {text}"
    );
    // And the reply carries no self-disclosure of the collision: the `bare_on_qualified` note in
    // `affected.rs` fires only on a call string containing `::`, while `direct_calls` records just
    // the bare identifier before `(`, so the note can never fire on this path.
    // 而回复不带任何关于这次撞名的自证：`affected.rs` 里的 `bare_on_qualified` 注只在调用串含 `::`
    // 时触发，而 `direct_calls` 只记录 `(` 前的裸标识符，因此这条注在这条路上永远不会出现。
    assert!(
        !text.contains("qualified") && !text.contains("bare"),
        "no wording admits the name collision (fix: record the qualifier in \
         `kernel::source::direct_calls` and match owner-to-owner in `callgraph::is_call_to`): {text}"
    );
    // The true positive still works, so the pin is about the extra row, not the whole list.
    // 真阳性仍然成立，因此这枚钉子钉的是多出来的那一行，而不是整张清单。
    let entry = affected(&root, &json!({"files": ["src/entry.rs"]})).expect("an answer");
    assert!(entry.contains("tests/uses_entry.rs"), "{entry}");
}

/// One path, a comma list, and a JSON array are the same request.
/// 一个路径、逗号列表、JSON 数组是同一个请求。
///
/// The round measured the comma list failing: `--files "a.rs,b.rs"` reached the tool as **one** path
/// named `a.rs,b.rs` and answered `not in the index`, while the same two paths in a JSON array
/// worked. Nothing about the argument's type says "one path" — its spelling does.
/// 那一轮量到逗号列表失败：`--files "a.rs,b.rs"` 以**一个**名为 `a.rs,b.rs` 的路径到达并答
/// `not in the index`，而同样两个路径写成 JSON 数组就成功。参数的**类型**并没有说"这是一个路径"——
/// 是它的**拼法**在说。
#[test]
fn the_three_spellings_of_several_files_agree() {
    let root = package("spellings");
    let one = affected(&root, &json!({"files": ["src/shared.rs"]})).expect("an answer");
    let comma = affected(&root, &json!({"files": "src/shared.rs"})).expect("an answer");
    assert_eq!(
        one, comma,
        "a one-element string is the same request as a one-element array"
    );
    // And a real comma list is read as two paths, not one.
    // 而真正的逗号列表被读成两个路径，不是一个。
    write_fixture(&root.join("src/quiet.rs"), "pub fn quiet_helper() {}\n");
    let two = affected(&root, &json!({"files": "src/shared.rs, src/quiet.rs"})).expect("an answer");
    assert!(
        !two.contains("not in the index"),
        "neither path may be reported as unknown: {two}"
    );
    assert!(
        two.contains("src/shared.rs: 1 definition(s)")
            && two.contains("src/quiet.rs: 1 definition(s)"),
        "both paths are answered: {two}"
    );
}
