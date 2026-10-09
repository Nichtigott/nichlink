//! Pins for the adoption gate: a lease left alone stays green, a lapsed one is a finding, and the
//! shipped checkout has none.
//! 采信门禁的钉子：没被碰过的租约保持绿、失效的租约是一条发现，而出厂检出里一条都没有。

use super::findings;
use std::path::{Path, PathBuf};
use xirang_kernel::adoption::adoption_fingerprint;

fn scratch(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "xirang-adoption-gate-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    root
}

fn write_fixture(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("dirs");
    std::fs::write(path, text).expect("file");
}

fn ledger(root: &Path, source: &str) {
    let fingerprint = adoption_fingerprint(&[("src/lib.rs".to_owned(), source.to_owned())]);
    write_fixture(
        &root.join(".xirang/adopted/entries"),
        &format!(
            "root/button|chain+impl|trace run|maintainer|2026-09-29T14:00:00Z|src/lib.rs|{fingerprint}|first adoption\n"
        ),
    );
}

#[test]
fn a_lease_whose_bytes_are_unchanged_is_not_a_finding() {
    let root = scratch("green");
    write_fixture(&root.join("src/lib.rs"), "// entry\n");
    ledger(&root, "// entry\n");
    assert!(findings(&root).is_empty(), "{:?}", findings(&root));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_lapsed_lease_names_the_file_and_asks_a_person() {
    let root = scratch("lapsed");
    write_fixture(&root.join("src/lib.rs"), "// entry changed\n");
    ledger(&root, "// entry\n");
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(
        found[0].contains("adoption lapsed at src/lib.rs"),
        "{found:?}"
    );
    assert!(found[0].contains("needs confirmation"), "{found:?}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_ledger_line_that_does_not_parse_is_a_finding() {
    let root = scratch("unparsable");
    write_fixture(&root.join(".xirang/adopted/entries"), "a|b|c\n");
    let found = findings(&root);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("8 fields"), "{found:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The gate the workspace run actually executes: this checkout's ledgers all hold.
/// 工作区运行时真正执行的那道门禁：本检出的每份台账都成立。
#[test]
fn the_shipped_checkout_has_no_lapsed_adoption() {
    let root = crate::workspace_root();
    let found = findings(&root);
    assert!(found.is_empty(), "{found:#?}");
}
