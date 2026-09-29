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
