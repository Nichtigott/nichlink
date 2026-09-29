//! Tests for the graft-plan rows: an entry that cannot be read is counted, not dropped.
//! graft 计划记录的测试：读不了的目录项要被计数，而不是被丢掉。

use std::io;

use super::{UNREADABLE_ENTRY, entry_rows};

/// A directory entry that could not be read becomes a counted row instead of vanishing.
/// 读不了的目录项会变成一条被计数的记录，而不是消失。
///
/// The loop over `read_dir` used `flatten()`, which drops the `Err` arm: a plan *file* that could
/// not be read was counted while an unreadable *entry* disappeared, so `records N` could
/// under-count without saying anything — and that number is what a maintainer reads before
/// pruning a release (audit `L1`).
/// 遍历 `read_dir` 的循环过去用 `flatten()`，它会丢掉 `Err` 那一半：读不了的**计划文件**会被计数，
/// 而读不了的**目录项**就此消失，于是 `records N` 可以在一个字都不说的情况下少数——而这个数字正是
/// 维护者在发布剪枝前读的东西（审计 `L1`）。
#[test]
fn an_unreadable_entry_is_a_counted_row() {
    let error = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
    let rows = entry_rows(Err(error), &[], None);
    assert_eq!(rows.len(), 1, "the error becomes one row, not nothing");
    let row = &rows[0];
    assert_eq!(row.selector, UNREADABLE_ENTRY);
    assert!(
        row.error
            .as_deref()
            .is_some_and(|error| error.contains("cannot read a directory entry")),
        "the row carries why it exists: {:?}",
        row.error
    );
    assert!(
        row.target.is_none() && row.target_path.is_none() && row.graft.is_none(),
        "there is nothing else to say about a directory entry that was never read"
    );
}
