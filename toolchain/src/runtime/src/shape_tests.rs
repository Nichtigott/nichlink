//! Pins for the shape a host declares: it validates, and a bad one is named.
//! 宿主声明的形状的钉子：它会被校验，而坏的会被点名。

use super::{Crate, Shape, Subtree, add_crates};
use nichlink_kernel::identity::NodeId;

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

/// A crate that claims a face by identity is accepted, and its claim is visible to the host's own
/// crate without any tree: the count is the half a host can check about itself.
/// 按身份认领一个面的 crate 会被接受，而它的认领不需要任何树就对宿主自己的 crate 可见：那个计数正是宿主能对
/// 自己查的那一半。
#[test]
fn a_crate_that_claims_a_face_by_identity_is_accepted() {
    static SLIDER: NodeId = NodeId::from_bytes(b"host::control::object::slider");
    static SHAPE: Shape = Shape {
        package_prefix: "myapp",
        crates: &[Crate::named("slider").faces(&[SLIDER])],
    };
    let shape = add_crates(&SHAPE);
    assert_eq!(shape.crates[0].name(), "slider");
    assert_eq!(shape.crates[0].named_faces().len(), 1);
    assert!(shape.crates[0].subtrees().is_empty());
}
