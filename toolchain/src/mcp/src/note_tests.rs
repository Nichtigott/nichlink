use super::*;

/// The budget bites out loud, and the pointer says where the rest is.
/// 上限生效时**说出声**，并给出剩下的在哪。
#[test]
fn a_bounded_note_says_where_the_rest_is() {
    let lines = vec![
        (34, "first".to_owned()),
        (35, "second".to_owned()),
        (36, "third".to_owned()),
    ];
    assert_eq!(
        numbered(&lines, 3, "`read {path, line}` has the rest"),
        "34: first / 35: second / 36: third",
        "an unbitten budget adds nothing"
    );
    assert_eq!(
        numbered(&lines, 2, "`read {path, line}` has the rest"),
        "34: first / 35: second (first 2 lines; `read {path, line}` has the rest)",
        "a bitten budget names the count and the way on"
    );
    assert!(numbered(&[], 3, "…").is_empty(), "nothing to attach");
}

/// The brief form keeps the source line numbers, not positions, and never invents characters.
/// 简短形式保留**源码行号**而不是位置，且从不凭空造字符。
#[test]
fn the_brief_form_takes_twelve_and_gives_back_what_it_has() {
    assert_eq!(brief("edc72845cc31aabbccdd"), "edc72845cc31");
    assert_eq!(brief("short"), "short", "a short value is returned whole");
    assert_eq!(brief(""), "");
}
