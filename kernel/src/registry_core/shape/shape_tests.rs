//! Pins for the crate-shape rule: it names what is wrong, and its boundaries are whole segments.
//! crate 形状规则的钉子：它点名错在哪，而它的边界是整个段。

use super::{DeclaredCrate, subtree_overlaps, validate_shape};

/// A declaration that holds together is accepted, and the rule reads it the way the callers write it.
/// 一份成立的声明会被接受，而规则按调用方的写法读它。
#[test]
fn a_shape_that_holds_together_is_accepted() {
    let crates = [
        DeclaredCrate {
            name: "widgets",
            subtrees: &["control::object"],
        },
        DeclaredCrate {
            name: "rules",
            subtrees: &["control::registry_rule"],
        },
    ];
    assert_eq!(validate_shape("myapp", &crates), Ok(()));
}

/// The boundary is a whole `::` segment, so a sibling whose name merely starts the same is not nested.
/// 边界是整个 `::` 段，因此名字只是前缀相同的兄弟不算嵌套。
#[test]
fn the_boundary_is_a_whole_segment() {
    assert!(subtree_overlaps("control", "control::object"));
    assert!(subtree_overlaps("control::object", "control::object"));
    assert!(!subtree_overlaps("control", "control_extra"));
    assert!(!subtree_overlaps(
        "control::object",
        "control::registry_rule"
    ));
}

/// Two crates claiming overlapping subtrees is the refusal this rule exists for, and it names both.
/// 两个 crate 认领重叠的子树正是这条规则存在的理由，而它把两个都点名。
#[test]
fn two_crates_claiming_overlapping_subtrees_are_named() {
    let crates = [
        DeclaredCrate {
            name: "widgets",
            subtrees: &["control::object"],
        },
        DeclaredCrate {
            name: "tiny",
            subtrees: &["control::object::button"],
        },
    ];
    let refusal = validate_shape("myapp", &crates).expect_err("refused");
    assert!(refusal.contains("`widgets`"), "{refusal}");
    assert!(refusal.contains("`tiny`"), "{refusal}");
    assert!(refusal.contains("exactly one crate"), "{refusal}");
}

/// One crate naming two subtrees that nest inside each other is refused for the same reason.
/// 一个 crate 点名两棵互相嵌套的子树，因同一个理由被拒。
#[test]
fn one_crate_naming_nested_subtrees_is_named() {
    let crates = [DeclaredCrate {
        name: "widgets",
        subtrees: &["control::object", "control::object::slider"],
    }];
    let refusal = validate_shape("myapp", &crates).expect_err("refused");
    assert!(refusal.contains("`widgets`"), "{refusal}");
}

/// An empty prefix, an unnamed crate and a crate with no subtree are three different refusals.
/// 空前缀、没有名字的 crate、以及没有子树的 crate，是三种不同的拒绝。
#[test]
fn the_empty_shapes_are_refused_where_they_are_empty() {
    assert!(
        validate_shape("", &[])
            .expect_err("an empty prefix")
            .contains("package_prefix")
    );
    let unnamed = [DeclaredCrate {
        name: "",
        subtrees: &["control"],
    }];
    assert!(validate_shape("myapp", &unnamed).is_err());
    let empty = [DeclaredCrate {
        name: "widgets",
        subtrees: &[],
    }];
    assert!(
        validate_shape("myapp", &empty)
            .expect_err("no subtrees")
            .contains("empty crate")
    );
    let twice = [
        DeclaredCrate {
            name: "widgets",
            subtrees: &["control"],
        },
        DeclaredCrate {
            name: "widgets",
            subtrees: &["rules"],
        },
    ];
    assert!(
        validate_shape("myapp", &twice)
            .expect_err("declared twice")
            .contains("twice")
    );
}
