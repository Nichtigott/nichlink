//! Pins for the adoption ledger: what parses, what a lapsed lease says, and that nothing here
//! renews itself.
//! 采信台账的钉子：什么能解析、失效的租约说什么，以及这里没有任何东西会自行续期。

use super::*;

fn entry(files: &[&str], fingerprint: &str) -> AdoptionEntry {
    AdoptionEntry {
        anchor: "root/control/button".to_owned(),
        certifies: "call chain and implementation".to_owned(),
        evidence: "converge: 6 live edges".to_owned(),
        verifier: "maintainer".to_owned(),
        at: "2026-09-29T14:00:00Z".to_owned(),
        files: files.iter().map(|file| (*file).to_owned()).collect(),
        fingerprint: fingerprint.to_owned(),
        reason: "re-confirmed after the chain fix".to_owned(),
    }
}

#[test]
fn a_ledger_line_round_trips_through_its_own_parser() {
    let ledger = "# anchor|certifies|evidence|verifier|at|files|fingerprint|reason\nroot/control/\
                  button|chain+impl|converge: 6 live|maintainer|2026-09-29T14:00:00Z|a.rs,b.rs|\
                  deadbeef|first adoption\n";
    let entries = parse_adoption(ledger).expect("the line parses");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].anchor, "root/control/button");
    assert_eq!(entries[0].files, vec!["a.rs".to_owned(), "b.rs".to_owned()]);
    assert_eq!(entries[0].fingerprint, "deadbeef");
    assert_eq!(entries[0].reason, "first adoption");
}

#[test]
fn a_line_missing_a_field_is_refused_with_its_line_number() {
    let ledger = "root/control/button|chain|evidence|maintainer|2026-09-29|a.rs\n";
    let error = parse_adoption(ledger).expect_err("six fields is not a line");
    assert_eq!(error.line, 1);
    assert!(error.message.contains("8 fields"), "{error:?}");
}

#[test]
fn an_entry_naming_no_file_is_refused() {
    let ledger = "a|b|c|d|e| |f|g\n";
    let error = parse_adoption(ledger).expect_err("an empty field is refused");
    assert!(error.message.contains("empty field"), "{error:?}");
}

#[test]
fn the_fingerprint_ignores_the_order_files_were_read_in() {
    let first = adoption_fingerprint(&[
        ("a.rs".to_owned(), "alpha".to_owned()),
        ("b.rs".to_owned(), "beta".to_owned()),
    ]);
    let second = adoption_fingerprint(&[
        ("b.rs".to_owned(), "beta".to_owned()),
        ("a.rs".to_owned(), "alpha".to_owned()),
    ]);
    assert_eq!(first, second);
    let different = adoption_fingerprint(&[
        ("a.rs".to_owned(), "alpha".to_owned()),
        ("b.rs".to_owned(), "gamma".to_owned()),
    ]);
    assert_ne!(first, different);
}

#[test]
fn an_unchanged_tree_keeps_the_lease_and_a_changed_byte_lapses_it() {
    let files = vec![
        ("a.rs".to_owned(), "alpha".to_owned()),
        ("b.rs".to_owned(), "beta".to_owned()),
    ];
    let held = entry(&["a.rs", "b.rs"], &adoption_fingerprint(&files));
    assert_eq!(verdict_of(&held, &files), AdoptionVerdict::Provisional);
    let moved = vec![
        ("a.rs".to_owned(), "alpha".to_owned()),
        ("b.rs".to_owned(), "beta ".to_owned()),
    ];
    assert!(matches!(
        verdict_of(&held, &moved),
        AdoptionVerdict::Lapsed { .. }
    ));
}

#[test]
fn a_missing_file_lapses_the_lease_and_is_the_one_named() {
    let files = vec![("a.rs".to_owned(), "alpha".to_owned())];
    let held = entry(&["a.rs", "gone.rs"], "whatever");
    assert_eq!(
        verdict_of(&held, &files),
        AdoptionVerdict::Lapsed {
            file: "gone.rs".to_owned()
        }
    );
}

/// The verdict and the fingerprint it was taken from come out of one call, and the thin reading
/// cannot drift from it.
/// 判定与它据以做出的指纹出自同一次调用，而那条薄读法不会与它漂移。
///
/// `verdict_of` used to compute the fingerprint and drop it, which is why the one reader that shows
/// it rebuilt the rule by hand (measured in the W8 round: 18,177 characters of reasoning, 11.5% of
/// that arm's whole chain).
/// `verdict_of` 过去算出指纹又丢掉，这正是唯一展示它的那个读者手工重推规则的原因（W8 实测：18,177
/// 字符的推理，占该臂整条链的 11.5%）。
#[test]
fn the_state_carries_the_fingerprint_its_verdict_came_from() {
    let files = vec![
        ("a.rs".to_owned(), "alpha".to_owned()),
        ("b.rs".to_owned(), "beta".to_owned()),
    ];
    let recorded = adoption_fingerprint(&files);
    let held = entry(&["a.rs", "b.rs"], &recorded);
    let state = state_of(&held, &files);
    assert_eq!(state.verdict, AdoptionVerdict::Provisional);
    assert_eq!(
        state.current, recorded,
        "an unchanged tree gives back the fingerprint it was taken on"
    );
    assert_eq!(
        state_of(&held, &files).verdict,
        verdict_of(&held, &files),
        "the thin reading agrees with the state it is read from"
    );
    // 反证：同一个条目、动一个字节 ⇒ 两个值必须不同，且判定要跟着翻。
    let moved = vec![
        ("a.rs".to_owned(), "alpha".to_owned()),
        ("b.rs".to_owned(), "beta!".to_owned()),
    ];
    let after = state_of(&held, &moved);
    assert_ne!(after.current, recorded, "a moved byte changes the value");
    assert!(matches!(after.verdict, AdoptionVerdict::Lapsed { .. }));
    assert_eq!(after.verdict, verdict_of(&held, &moved));
}

/// A file the entry names but the tree no longer has still yields a value, so the reply can show
/// what the bytes amount to now instead of going silent.
/// 条目点名而树里已经没有的文件仍然给出一个值，因此回复能显示"这些字节现在算出来是多少"而不是沉默。
#[test]
fn a_missing_file_still_yields_a_current_fingerprint() {
    let files = vec![("a.rs".to_owned(), "alpha".to_owned())];
    let held = entry(&["a.rs", "gone.rs"], "whatever");
    let state = state_of(&held, &files);
    assert_eq!(
        state.verdict,
        AdoptionVerdict::Lapsed {
            file: "gone.rs".to_owned()
        }
    );
    assert_eq!(
        state.current.len(),
        64,
        "a fingerprint, not an empty string"
    );
}
