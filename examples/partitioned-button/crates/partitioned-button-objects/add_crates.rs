//! The crate shape this host declares: `control` (which contains both grafted faces) becomes a
//! crate of its own. The graft plan in `src/lib.rs` is untouched by this — the cuts follow the
//! faces wherever the boundary puts them.
//! 这个宿主声明的 crate 形状：`control`（两条被 graft 的面都在它下面）自成一个 crate。`src/lib.rs` 里
//! 的 graft 计划一个字都不用改 —— 切口跟着面走，边界把它放哪儿都行。

use nichlink_toolchain::run_method::{Crate, Shape};

/// Which subtrees of this host become crates of their own.
/// 这个宿主里哪些子树各自成为一个 crate。
pub fn add_crates() -> Shape {
    Shape::of(
        "partitioned-button",
        &[Crate::named("objects").at(&[crate::control::SUBTREE])],
    )
}
