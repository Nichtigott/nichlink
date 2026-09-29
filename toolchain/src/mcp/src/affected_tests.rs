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
