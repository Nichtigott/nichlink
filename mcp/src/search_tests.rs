//! Tests for `nichlink.search`'s tree half: a face is found by logical path,
//! `kind`, module or slot name, and each hit says what the build thinks of it.
//! `nichlink.search` 树那一半的测试：面可以按逻辑路径、`kind`、模块或槽位名找到，而每个命中都说
//! 出构建对它的看法。
//!
//! The classification rule itself is pinned through `nichlink.diff`'s tests as
//! well; what this file pins is that search reads that one rule, so a face cannot
//! be `ok` in one tool and `added` in the other.
//! 分类规则本身也由 `nichlink.diff` 的测试钉住；本文件钉的是 search 读的正是那一条规则，因此一个面
//! 不可能在一个工具里是 `ok`、在另一个里是 `added`。

use std::path::{Path, PathBuf};

use serde_json::json;

use super::search;

/// A throwaway package with two hand-written root faces.
/// 一个含两个手写根面的一次性包。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-search-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    face(
        &root,
        "gauge/gauge.rs",
        "Gauge",
        "pub fn gauge_value() -> u8 { 1 }\n\ncrate::root_object! {\n    kind: Gauge,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    face(
        &root,
        "button/button.rs",
        "Button",
        "crate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    (root, name)
}

/// Write one face's module.
/// 写入一个面的模块。
fn face(root: &Path, relative: &str, _kind: &str, source: &str) {
    let path = root.join("src").join(relative);
    std::fs::create_dir_all(path.parent().expect("face parent")).expect("face directory");
    std::fs::write(path, source).expect("face source");
}

/// Publish the build evidence for this package.
/// 发布本包的构建证据。
fn publish(root: &Path, name: &str) {
    nichlink_build_method::check_for(root, &root.join("target/nichlink/out"), name)
        .expect("a healthy tree checks clean");
}

/// A face is found by its logical path, kind, module and slot name, and with no
/// build published the verdict is `build unknown` rather than a guess.
/// 面可以按其逻辑路径、kind、模块与槽位名找到；没有发布构建时结论是 `build unknown` 而不是猜。
#[test]
fn a_face_is_found_and_an_unbuilt_tree_says_the_verdict_is_unknown() {
    let (root, _) = package("unbuilt");
    let reply = search(&root, &json!({"query": "gauge"})).expect("the search answers");
    assert!(reply.contains("face  root/gauge"), "{reply}");
    assert!(reply.contains("kind=Gauge"), "{reply}");
    assert!(reply.contains("module=gauge"), "{reply}");
    assert!(reply.contains("source=gauge/gauge.rs"), "{reply}");
    assert!(
        reply.contains("[build unknown (run `nichlink check`)]"),
        "an unbuilt tree must not invent a verdict: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Once the build published, a face it saw is `ok` and one it never saw is
/// `added since build`.
/// 构建发布之后，它见过的面是 `ok`，它从未见过的是 `added since build`。
#[test]
fn a_published_face_is_ok_and_a_new_one_is_added_since_build() {
    let (root, name) = package("published");
    publish(&root, &name);
    let known = search(&root, &json!({"query": "root/button"})).expect("the search answers");
    assert!(known.contains("[ok]"), "{known}");

    face(
        &root,
        "dial/dial.rs",
        "Dial",
        "crate::root_object! {\n    kind: Dial,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    let added = search(&root, &json!({"query": "dial"})).expect("the search answers");
    assert!(added.contains("face  root/dial"), "{added}");
    assert!(added.contains("[added since build]"), "{added}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A `kind` change under an unmoved file is `re-identified` with the old and new
/// identities — the one change no text search can see.
/// 文件没动而 `kind` 变了会报成 `re-identified` 并带出旧、新身份——这是文本搜索看不见的那种变化。
#[test]
fn a_kind_change_under_an_unmoved_file_is_re_identified() {
    let (root, name) = package("re-identified");
    publish(&root, &name);
    face(
        &root,
        "button/button.rs",
        "Button",
        "crate::root_object! {\n    kind: RenamedButton,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    let reply = search(&root, &json!({"query": "root/button"})).expect("the search answers");
    assert!(reply.contains("[re-identified ("), "{reply}");
    assert!(reply.contains(" -> "), "{reply}");
    assert!(
        reply.contains("build output is stale"),
        "the verdicts are about the build that was published, and the reply says so: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The source half is unchanged: a file path and a function declaration by name
/// still answer exactly as before, below the face hits.
/// 源码那一半不变：按名字匹配的文件路径与函数声明仍和以前一样作答，排在面命中之后。
#[test]
fn the_source_half_still_answers_file_and_function_names() {
    let (root, _) = package("source-half");
    let by_file = search(&root, &json!({"query": "gauge"})).expect("the search answers");
    assert!(by_file.contains("face  root/gauge"), "{by_file}");
    assert!(by_file.contains("file  src/gauge/gauge.rs"), "{by_file}");

    let by_function = search(&root, &json!({"query": "gauge_value"})).expect("the search answers");
    assert!(
        by_function.contains("fn    gauge_value -> src/gauge/gauge.rs:"),
        "{by_function}"
    );

    let none =
        search(&root, &json!({"query": "nothing-matches-this"})).expect("the search answers");
    assert_eq!(none, "no matches");
    let _ = std::fs::remove_dir_all(&root);
}

/// A root whose identity namespace cannot be learned still answers the source
/// half, and says the tree half is unavailable instead of pretending it is empty.
/// 身份命名空间无从得知的根仍然回答源码那一半，并说明树那一半不可用，而不是假装它是空的。
#[test]
fn a_root_without_a_package_still_answers_the_source_half() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("mcp-search-bare-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    std::fs::write(root.join("src/widget.rs"), "pub fn widget_spin() {}\n").expect("source file");

    let reply = search(&root, &json!({"query": "widget"})).expect("the search answers");
    assert!(reply.contains("tree  unavailable ("), "{reply}");
    assert!(reply.contains("file  src/widget.rs"), "{reply}");
    assert!(
        reply.contains("fn    widget_spin -> src/widget.rs:1"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
