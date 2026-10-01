//! Pins for the adoption surface: a lease reads as provisional, a moved byte reads as lapsed
//! with the file named, and a preview writes nothing.
//! 采信面的钉子：租约读作 provisional、动过的字节读作 lapsed 并点名文件、预览什么都不写。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;

use super::adopted;

fn package(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-adopted-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_fixture(&root.join("src/lib.rs"), "// entry\n");
    root
}

fn write_fixture(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dirs");
    std::fs::write(path, text).expect("fixture file");
}

/// Write a ledger whose single entry was taken on the bytes in `src/lib.rs` right now.
/// 写一份台账，其中唯一一条条目是针对 `src/lib.rs` **此刻**的字节做出的。
fn adopt_current(root: &Path) -> String {
    let contents = std::fs::read_to_string(root.join("src/lib.rs")).expect("fixture source");
    let fingerprint =
        nichlink_kernel::adoption::adoption_fingerprint(&[("src/lib.rs".to_owned(), contents)]);
    let line = format!(
        "root/button|chain+impl|converge: 6 live edges|maintainer|2026-09-29T14:00:00Z|src/lib.rs|{fingerprint}|first adoption\n"
    );
    write_fixture(&root.join(".nichlink/adopted/entries"), &line);
    line
}

#[test]
fn a_lease_whose_bytes_are_unchanged_reads_as_provisional() {
    let root = package("provisional");
    adopt_current(&root);
    let text = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(text.contains("(provisional)"), "{text}");
    assert!(text.contains("provisional 1  lapsed 0"), "{text}");
    assert!(
        !text.contains("verified") && !text.contains("guaranteed"),
        "an adoption is a lease, never a certificate: {text}"
    );
    // The state is not the whole answer: a route the ledger does not name needs the action spelled
    // out, because the round measured an arm that read the rules and concluded it should do nothing.
    // 状态不是全部答案：台账没点名的路线需要把**动作**说出来——那轮量到的正是一臂读懂了规矩、
    // 于是推出"什么都不该做"。
    assert!(
        text.contains("new anchor") && text.contains("apply: true"),
        "the read names the action for a route the ledger does not carry: {text}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_moved_byte_lapses_the_lease_and_the_file_is_named() {
    let root = package("lapsed");
    adopt_current(&root);
    write_fixture(&root.join("src/lib.rs"), "// entry changed\n");
    let text = adopted(&root, &json!({})).expect("the ledger answers");
    assert!(
        text.contains("adoption lapsed at src/lib.rs; needs confirmation"),
        "{text}"
    );
    assert!(text.contains("provisional 0  lapsed 1"), "{text}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_renewal_is_a_preview_until_it_is_applied_and_confirmed() {
    let root = package("renew");
    let request = json!({
        "anchor": "root/button",
        "certifies": "chain+impl",
        "evidence": "trace run",
        "verifier": "maintainer",
        "reason": "re-confirmed after the chain fix",
        "files": ["src/lib.rs"],
    });
    let preview = adopted(&root, &request).expect("a preview");
    assert!(preview.contains("nothing was written"), "{preview}");
    assert!(!root.join(".nichlink/adopted/entries").exists());
    let applied = adopted(
        &root,
        &json!({
            "anchor": "root/button",
            "certifies": "chain+impl",
            "evidence": "trace run",
            "verifier": "maintainer",
            "reason": "re-confirmed after the chain fix",
            "files": ["src/lib.rs"],
            "apply": true,
            "confirm": true,
        }),
    )
    .expect("an append");
    assert!(applied.contains("append-only"), "{applied}");
    let written = std::fs::read_to_string(root.join(".nichlink/adopted/entries")).expect("ledger");
    assert!(
        written.contains("re-confirmed after the chain fix"),
        "{written}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The anchor's claim is answered in three states: unknown, lapsed at a named file, and the newest
/// revision in force when an anchor has a history.
/// 一个 anchor 的声明有三种状态：没有条目、在点名文件上失效、以及有历史时**最后一条**生效。
#[test]
fn conformance_answers_unknown_lapsed_and_in_force() {
    let root = std::env::temp_dir().join(format!("mcp-conformance-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".nichlink/adopted")).expect("ledger directory");
    std::fs::write(root.join("src.rs"), "pub fn one() {}\n").expect("a file to cover");
    let ledger = root.join(".nichlink/adopted/entries");
    std::fs::write(
        &ledger,
        "root/control/button|first reading|traced once|nich|2026-10-01T10:00:00+08:00|src.rs|cafe|first\n\
         root/control/button|second reading|traced twice|nich|2026-10-01T11:00:00+08:00|src.rs|cafe|second\n",
    )
    .expect("ledger");

    let unknown = super::conformance(&root, &json!({"anchor": "root/nope"})).expect("an answer");
    assert!(
        unknown.starts_with("no ledger entry names `root/nope`"),
        "{unknown}"
    );

    let answer =
        super::conformance(&root, &json!({"anchor": "root/control/button"})).expect("an answer");
    assert!(
        answer.contains("2 revision(s)") && answer.contains("2026-10-01T11:00:00+08:00"),
        "the newest revision is the one in force (its `at` is printed): {answer}"
    );
    assert!(
        answer.contains("lapsed at") && answer.contains("needs a **person**"),
        "a fingerprint that does not match the bytes is lapsed, and the file is named: {answer}"
    );
    assert!(
        answer.contains("covers     src.rs") && answer.contains("not covered here"),
        "the covered files and the bounds ride along: {answer}"
    );

    let missing = super::conformance(&root, &json!({})).expect_err("an anchor is required");
    assert!(
        missing.contains("`anchor`") && missing.contains("accepted shape"),
        "{missing}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
