//! Pins for `nichlink.consistency`: the sibling set and the deviation rule, both decidable.
//! `nichlink.consistency` 的钉子：同族集合与离群判据，两者都可判定。
//!
//! The failure these guard is the one the capability answers: two siblings calling one family of
//! names while a third calls another, with every file locally plausible. What is pinned here is the
//! rule the answer turns on — a name most siblings call and this one does not, and a name it calls
//! that no sibling does — plus the shape of the refusals.
//! 这些钉子守的正是这条能力要回答的失败：两个兄弟调用同一族名字、第三个调用另一族，而每个文件单看都
//! 自洽。钉住的是答案所依赖的那条规则 —— "多数兄弟都调、而它没调的名字"与"它调了、别的兄弟都没调的名字"
//! —— 以及拒绝的形状。

use std::collections::BTreeSet;

use serde_json::json;

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

/// The deviating sibling is named, and the note says which way it deviates.
/// 不一样的兄弟被点名，注里说清它往哪边不一样。
#[test]
fn the_deviating_sibling_is_named_with_the_direction() {
    let sets = vec![
        ("button".to_owned(), set(&["to_world", "scale_x"])),
        ("slider".to_owned(), set(&["to_local", "scale_x"])),
        ("timeline".to_owned(), set(&["to_local", "scale_x"])),
    ];
    let rows = super::deviations(&sets);
    assert_eq!(rows.len(), 1, "one sibling differs: {rows:?}");
    assert_eq!(rows[0].0, "button");
    let notes = rows[0].1.join("; ");
    assert!(
        notes.contains("does not call `to_local`, which the other siblings call"),
        "the missing majority name is named: {notes}"
    );
    assert!(
        notes.contains("calls `to_world`, which no sibling calls"),
        "the lone name is named: {notes}"
    );
}

/// Unanimity is not an outlier, and neither is a name shared by exactly half.
/// 全体一致不是离群；恰好一半共享的名字也不算。
#[test]
fn unanimity_and_halves_are_not_outliers() {
    let same = vec![
        ("a".to_owned(), set(&["x", "y"])),
        ("b".to_owned(), set(&["x", "y"])),
    ];
    assert!(super::deviations(&same).is_empty(), "unanimous");

    let halves = vec![
        ("a".to_owned(), set(&["x", "only_a"])),
        ("b".to_owned(), set(&["x", "only_b"])),
    ];
    let rows = super::deviations(&halves);
    assert_eq!(
        rows.len(),
        2,
        "a name exactly half the siblings call is not a majority: {rows:?}"
    );
}

/// The refusals carry the accepted shape, for a missing parent and for an unknown signal.
/// 拒绝带可接受形状：缺 `parent` 与未知的 `by` 各一条。
#[test]
fn the_refusals_carry_the_shape() {
    let root = std::env::temp_dir();
    let missing = super::consistency(&root, &json!({})).expect_err("a parent is required");
    assert!(
        missing.contains("consistency needs `parent`") && missing.contains("accepted shape"),
        "{missing}"
    );
    let unknown = super::consistency(&root, &json!({"parent": "root/control", "by": "colour"}))
        .expect_err("the signal is checked before the tree is read");
    assert!(
        unknown.contains("`api`, `kind` or `source`") && unknown.contains("accepted shape"),
        "{unknown}"
    );
}

/// The sibling set is exactly the faces one level under the parent.
/// 同族集合正好是父级下一层的那些面。
#[test]
fn the_sibling_set_is_one_level_deep() {
    // The rule is a path rule, so it is pinned on paths rather than on a whole fixture.
    // 这条规则是路径规则，因此按路径钉，而不是搭一整套夹具。
    for (path, parent, wanted) in [
        ("root/control/button", "root/control", true),
        ("root/control/button/label", "root/control", false),
        ("root/control", "root/control", false),
        ("root/other/button", "root/control", false),
    ] {
        let above = path.rsplit_once('/').map(|(above, _)| above);
        assert_eq!(above == Some(parent), wanted, "`{path}` under `{parent}`");
    }
}
