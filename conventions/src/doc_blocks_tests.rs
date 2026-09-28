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

/// The exemption is about the record *directory*, not only about the file's own name: a
/// report inside `docs/audit-2026-09-28/` is a record whatever it is called.
/// 豁免针对记录**目录**，而不只是文件名本身：`docs/audit-2026-09-28/` 里的报告无论叫什么都是记录。
///
/// Red before the fix: the rule compared the file name to `RECORD_PREFIXES`, so this file
/// and every peer report that did not happen to start with `audit-` were scanned as living
/// documentation — their Rust excerpts had to parse and their `.rs:NNN` anchors had to
/// resolve *today*, which turns a record into a document somebody must keep following
/// (audit `LGC-LG-53`). The reports were renamed `audit-*` to get the gate quiet again; the
/// boundary is what needed fixing.
/// 修前为红：规则把文件名与 `RECORD_PREFIXES` 比较，因此本文件以及每个名字碰巧不以 `audit-`
/// 开头的同伴报告都会被当成活文档扫描——它们摘录的 Rust 必须能解析、它们的 `.rs:NNN` 锚点必须
/// **此刻**仍然成立，于是记录变成了必须有人持续维护的文档（审计 `LGC-LG-53`）。报告当时被改名成
/// `audit-*` 来让门禁复绿；该修的是那条边界。
#[test]
fn a_report_inside_a_record_directory_is_exempt() {
    let root = synthetic(&[(
        "docs/audit-2026-09-28/verify-note.md",
        "```rust\npub struct Broken {\n```\n",
    )]);
    let found = findings(&root);
    assert!(
        found.is_empty(),
        "a file under a record directory is a record, whatever it is named: {found:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The other half of the boundary: a file whose *name* carries the prefix is a record
/// wherever it sits, and a living document in an ordinary directory is still covered.
/// 边界的另一半：名字带前缀的文件无论在哪儿都是记录，而普通目录里的活文档仍在覆盖范围内。
#[test]
fn the_exemption_does_not_swallow_living_documents() {
    let record = synthetic(&[("docs/audit-probe.md", "```rust\npub struct Broken {\n```\n")]);
    assert!(
        findings(&record).is_empty(),
        "a file named like a record is exempt by name"
    );
    let _ = std::fs::remove_dir_all(&record);

    let living = synthetic(&[("docs/notes.md", "```rust\npub struct Broken {\n```\n")]);
    assert_eq!(
        findings(&living).len(),
        1,
        "an ordinary document is still covered"
    );
    let _ = std::fs::remove_dir_all(&living);
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

/// The tag set is one set, and the half that lagged was the **doc-comment** one: a `///`
/// fence tagged `rs` was skipped, so a broken example in a crate's own documentation passed
/// while the identical README fence was already checked (the markdown half lowercased the
/// tag and accepted `rs`). Both halves read `names_rust` now.
/// tag 集合是一套，而落后的是**文档注释**那一半：`///` 围栏标成 `rs` 时被跳过，于是 crate 自己
/// 文档里的坏例子能过，而一模一样的 README 围栏却早已被查（markdown 那半边先小写并接受 `rs`）。
/// 现在两半边都读 `names_rust`。
#[test]
fn an_rs_tagged_doc_comment_fence_is_covered_too() {
    for tag in ["rs", "Rust"] {
        let markdown = synthetic(&[(
            "README.md",
            format!("# A host\n\n```{tag}\npub struct Broken {{\n```\n").as_str(),
        )]);
        assert_eq!(
            findings(&markdown).len(),
            1,
            "the markdown half already read `{tag}`"
        );
        let _ = std::fs::remove_dir_all(&markdown);

        let source = format!("//! ```{tag}\n//! pub struct Broken {{\n//! ```\n");
        let documented = synthetic(&[
            ("thing/Cargo.toml", "[package]\nname = \"thing\"\n"),
            ("thing/src/lib.rs", source.as_str()),
        ]);
        assert_eq!(
            findings(&documented).len(),
            1,
            "a `///` fence tagged `{tag}` is a Rust fence too"
        );
        let _ = std::fs::remove_dir_all(&documented);
    }
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

/// The field-list reading is for `name: Type` excerpts, and it needs a colon. Without one
/// there is nothing that makes the excerpt Rust rather than a word — `let _ = Excerpt {
/// TODO };` is a valid struct literal, so a fence whose whole body was `TODO` passed while
/// `fix this later` was reported.
/// 字段列表读法是为 `name: Type` 这种摘录准备的，它需要一个冒号。没有冒号时，没有任何东西让这段
/// 摘录成为 Rust 而不是一个单词——`let _ = Excerpt { TODO };` 是合法的结构体字面量，因此整段只有
/// `TODO` 的围栏能过，而 `fix this later` 会被报出。
#[test]
fn a_bare_identifier_is_not_a_rust_excerpt() {
    assert!(
        parses("TODO").is_err(),
        "a word is not Rust, however the reading wraps it"
    );
    assert!(
        parses("name: u8,\ncount: usize").is_ok(),
        "a field list still is, and that is what the reading exists for"
    );
}

/// CommonMark pairs a fence with a closing fence of **the same character** and **at least the same
/// length**, and the info string is what follows that run — not what follows the first three
/// characters of it. Reading a bare prefix instead made two whole shapes invisible: a `~~~rust`
/// block was closed by a ```` ``` ```` line inside it (so the broken code after that line was never
/// parsed), and a ```` ````rust ```` fence had its info string read as `` `rust `` (so the block
/// never opened at all). Audit `G-03`.
/// CommonMark 用**同一字符**、**长度不短于开围栏**的围栏来闭合，而 info string 是那串字符**之后**
/// 的文本，不是前三个字符之后的文本。只比前缀让两种形状整段隐形：`~~~rust` 块被块内的一行
/// ```` ``` ```` 闭合（那一行之后的坏代码从未被解析），而 ```` ````rust ```` 围栏的 info string 被读成
/// `` `rust ``（整块从未打开）。审计 `G-03`。
#[test]
fn a_fence_is_paired_by_its_own_character_and_length() {
    let root = synthetic(&[(
        "docs/probe.md",
        "# Probe\n\n~~~rust\nlet kept = 1;\n```\nlet _ = ;\n~~~\n\n\
         ````rust\nlet _ = ;\n````\n\n```rust\nfn control_is_fine() {}\n```\n",
    )]);
    let found = findings(&root);
    let lines: Vec<usize> = found.iter().map(|finding| finding.line).collect();
    assert_eq!(
        lines,
        vec![3, 9],
        "the tilde fence is not closed by a backtick line, and a four-backtick \
         fence is still a Rust fence: {found:#?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A module the tree mounts behind `#[cfg(test)]` is not documentation a reader is shown, whatever
/// its name says. Location and name are hints, and a hint misses a real case: the tree ships
/// `core/src/registry_core/tree/graft_ops/fixtures.rs`, mounted behind `#[cfg(test)]`, whose name
/// is not test-shaped — so this gate checked its comments as reader-facing documentation while the
/// size gate (which asks the same identity question, audit `G-05`) had already stopped doing so.
/// 树以 `#[cfg(test)]` 挂载的模块不是给读者看的文档，无论它叫什么。位置与名字都是提示，而提示会漏
/// 掉真实情形：树里出厂了 `core/src/registry_core/tree/graft_ops/fixtures.rs`，它挂在 `#[cfg(test)]`
/// 之后、名字又不是测试形状——于是这道门禁把它的注释当读者文档检查，而尺寸门禁（问的是同一个身份
/// 问题，审计 `G-05`）早就不这么做了。
#[test]
fn a_cfg_test_mounted_module_is_skipped_whatever_its_name() {
    let root = synthetic(&[
        ("src/lib.rs", "pub mod inner;\n"),
        (
            "src/inner.rs",
            "#[cfg(test)]\n#[path = \"inner/fixtures.rs\"]\npub mod fixtures;\n",
        ),
        // A fence that does not parse: checking this file as reader-facing documentation reports it.
        // 一份解析不了的围栏：把这个文件当读者文档检查就会报出它。
        (
            "src/inner/fixtures.rs",
            "//! ```rust\n//! pub struct Unclosed {\n//! ```\n",
        ),
    ]);
    assert!(
        doc_comment_findings(&root).is_empty(),
        "a `#[cfg(test)]`-mounted module is not reader-facing documentation"
    );
    let _ = std::fs::remove_dir_all(&root);
}
