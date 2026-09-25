//! Tests for the documentation-block gate.
//! 文档代码块门禁的测试。

use super::*;
use crate::workspace_root;

/// A fence that nests past the kernel's measurement is *reported*, not fatal.
/// 嵌套越过内核度量的围栏会被**报告**，而不是致命。
///
/// Without the guard this test does not fail — it aborts the test process, which
/// is the whole point: `syn` is recursive descent, so a documentation gate that
/// parses untrusted-in-shape fences can be killed by one of them.
/// 没有守卫时这条测试不是失败——它会 abort 测试进程，而这正是要点：`syn` 是递归下降的，因此
/// 一道会解析"形状不可信"围栏的文档门禁可以被其中一份围栏打死。
#[test]
fn a_pathologically_nested_fence_is_reported_not_fatal() {
    let code = format!("let x = {}1{};", "(".repeat(60_000), ")".repeat(60_000));
    let message = parses(&code).expect_err("a fence past the nesting limit must be refused");
    assert!(
        message.contains("nests"),
        "the refusal must explain itself: {message}"
    );
}

/// A fence that never closes is not "nothing to check": the reader sees the
/// code, and `syn` never gets it.
/// 从不闭合的围栏不是"没有东西要检查"：读者看得到那段代码，而 `syn` 从没拿到过它。
#[test]
fn an_unterminated_fence_is_reported() {
    let root = synthetic(&[("README.md", "```rust\npub struct Broken {\n")]);
    let found = findings(&root);
    assert_eq!(
        found.len(),
        1,
        "an unclosed fence must be reported: {found:#?}"
    );
    assert!(
        found[0].error.contains("unterminated") || found[0].error.contains("closed"),
        "the refusal must explain itself: {:#?}",
        found[0]
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Every living document under `docs/` is covered, however deep it sits.
/// `docs/` 下的每份活文档都在覆盖范围内，无论它有多深。
#[test]
fn a_fence_in_a_nested_docs_file_is_covered() {
    let root = synthetic(&[(
        "docs/reference/zz_audit_probe.md",
        "```rust\npub struct Broken {\n```\n",
    )]);
    let found = findings(&root);
    assert_eq!(
        found.len(),
        1,
        "a document under docs/ is living documentation: {found:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A throwaway checkout with the given files under it.
/// 一个只含给定文件的一次性检出。
fn synthetic(files: &[(&str, &str)]) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-doc-blocks-{}-{}-{sequence}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    for (relative, contents) in files {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dir");
        std::fs::write(&path, contents).expect("fixture file");
    }
    crate::fixture_manifest(&root);
    root
}
/// Every Rust block in the READMEs and docs parses.
/// README 与文档里的每个 Rust 块都能解析。
#[test]
fn documented_rust_blocks_parse() {
    let root = workspace_root();
    let found = findings(&root);
    assert!(
        found.is_empty(),
        "these documented Rust blocks no longer parse: {found:#?}"
    );
}

/// The gate can fail, demonstrated without touching the repository.
/// 门禁能失败，且演示过程不触碰仓库。
#[test]
fn a_broken_block_is_reported() {
    let directory = tempfile::tempdir().unwrap();
    crate::fixture_manifest(directory.path());
    std::fs::write(
        directory.path().join("README.md"),
        "# A host\n\n```rust\npub struct Broken {\n```\n",
    )
    .unwrap();
    let found = findings(directory.path());
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].line, 3);
}

/// A statement excerpt and a whole file are both accepted.
/// 语句摘录与整份文件都被接受。
#[test]
fn both_readings_are_accepted() {
    assert!(parses("let value = build()?;").is_ok());
    assert!(parses("pub struct Canvas;\n").is_ok());
    assert!(parses("let value = ;").is_err());
}

/// The sanctioned `macro-input` tag is honoured in markdown, and the other fence
/// character and the short language name are read.
/// markdown 里被认可的 `macro-input` 标记会被尊重，另一种围栏字符与语言简称也会被读。
#[test]
fn a_sanctioned_tag_and_every_fence_spelling_are_read() {
    let root = synthetic(&[(
        "docs/probe.md",
        "# Probe\n\n```rust,macro-input\nkind: Probe,\n```\n\n~~~rust\nfn broken( {\n~~~\n\n```rs\nfn also_broken( {\n```\n",
    )]);
    let found = findings(&root);
    assert_eq!(
        found.len(),
        2,
        "the sanctioned tag is green; the tilde and `rs` fences are reported: {found:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
