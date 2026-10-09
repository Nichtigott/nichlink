//! Which subtrees of this host become crates of their own.
//! 这个宿主里哪些子树各自成为一个 crate。

use xirang_toolchain::runtime::{Crate, Shape};

/// Three crates over three disjoint subtrees; `panel` itself stays with the host.
/// 三棵互不相邻的子树各自成一个 crate；`panel` 自己留在宿主里。
pub fn add_crates() -> Shape {
    Shape::of(
        "control-suite",
        &[
            Crate::named("objects").at(&[crate::control::SUBTREE]),
            Crate::named("frames").at(&[crate::panel::frame::SUBTREE]),
            Crate::named("gauges").at(&[crate::panel::gauge::SUBTREE]),
        ],
    )
}
