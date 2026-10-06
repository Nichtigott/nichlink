//! Pins for the shape a host declares: it validates, and a bad one is named.
//! 宿主声明的形状的钉子：它会被校验，而坏的会被点名。

use super::{Crate, Shape, Subtree, add_crates};

/// A shape that holds together is accepted and handed back unchanged.
/// 一份成立的形状会被接受，并原样交回。
#[test]
fn a_declared_shape_is_accepted_and_handed_back() {
    static WIDGETS: Subtree = Subtree::new("host::control::object");
    static SHAPE: Shape = Shape {
        package_prefix: "myapp",
        crates: &[Crate::named("widgets").at(&[WIDGETS])],
    };
    let shape = add_crates(&SHAPE);
    assert_eq!(shape.package_prefix, "myapp");
    assert_eq!(shape.crates[0].name(), "widgets");
    assert_eq!(
        shape.crates[0].subtrees()[0].module(),
        "host::control::object"
    );
}

/// A shape whose crates overlap fails where it can be seen, and the refusal names both crates.
/// 两个 crate 重叠的形状会在看得见的地方失败，而拒绝把两个 crate 都点名。
#[test]
fn an_overlapping_shape_is_refused_by_name() {
    static OUTER: Subtree = Subtree::new("host::control::object");
    static INNER: Subtree = Subtree::new("host::control::object::button");
    static SHAPE: Shape = Shape {
        package_prefix: "myapp",
        crates: &[
            Crate::named("widgets").at(&[OUTER]),
            Crate::named("tiny").at(&[INNER]),
        ],
    };
    let refusal = std::panic::catch_unwind(|| add_crates(&SHAPE))
        .expect_err("an overlapping declaration is refused");
    let message = refusal
        .downcast_ref::<String>()
        .cloned()
        .unwrap_or_else(|| {
            refusal
                .downcast_ref::<&str>()
                .map(|s| (*s).to_owned())
                .unwrap_or_default()
        });
    assert!(message.contains("`widgets`"), "{message}");
    assert!(message.contains("`tiny`"), "{message}");
    assert!(message.contains("exactly one crate"), "{message}");
}
