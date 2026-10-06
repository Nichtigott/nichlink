//! Naming the crate that compiles each module, for the **facade** (audit `M7`, §M7.33).
//! 为 **facade** 点出编译每个模块的那个 crate（审计 `M7`，§M7.33）。
//!
//! A facade compiles no faces of its own: it carries the cross-crate half — the graft cut table and the
//! contract assertions — and `crate::` inside it means the facade itself. So every `crate::<module>` it
//! emits is rewritten to `<owner>::<module>`, where the owner comes from the plan: a claim, its root
//! face and its ancestor shells all belong to that ghost, and every other module belongs to the host.
//! facade 自己不编译任何面：它携带跨 crate 那一半——graft 切口表与契约断言——而它里面的 `crate::` 指的是
//! facade 自己。因此它发射的每个 `crate::<模块>` 都改写成 `<owner>::<模块>`，属主来自规划：一条认领、它的根面
//! 与它的祖先壳都归那个幽灵，其余每个模块归宿主。
//!
//! It lives beside the pass rather than inside it because the rewrite is one rule with its own tests, and
//! the pass is already at its size budget.
//! 它住在 pass 旁边而不是里面，因为这条改写是一条有自己钉子的规则，而 pass 已经用满了尺寸预算。

use super::tree::ShapeRender;

/// The crate that compiles one module, as an expression the facade can name.
/// 编译某个模块的那个 crate，写成 facade 能点名的表达式。
///
/// In facade mode every `crate::<module>` becomes `<owner>::<module>`, because `crate::` there means
/// the facade itself; a bare module path (the registration-rules block writes them that way) gets the
/// same owner prefix. The lookup takes the **longest** owner whose module is a prefix at a `::`
/// boundary, so a claim's root face and its descendants are all attributed correctly.
/// facade 模式里每个 `crate::<模块>` 都变成 `<owner>::<模块>`，因为那里的 `crate::` 指的是 facade 自己；
/// 裸模块路径（注册规则块就是这么写的）加同一个属主前缀。查表取**最长**的、模块在 `::` 边界上构成前缀的属主，
/// 因此一条认领的根面与它的后代都能归属正确。
pub(super) fn owned(shape: &ShapeRender<'_>, expression: &str) -> String {
    let trimmed = expression.trim();
    if !shape.facade {
        return trimmed.to_owned();
    }
    let had_prefix = trimmed.starts_with("crate::");
    let rest = trimmed.strip_prefix("crate::").unwrap_or(trimmed);
    let module = rest.rsplit_once("::").map_or(rest, |(module, _)| module);
    let owner = shape
        .owners
        .iter()
        .filter(|(path, _)| module == path || module.starts_with(&format!("{path}::")))
        .max_by_key(|(path, _)| path.len())
        .map(|(_, owner)| owner.clone());
    match (owner, had_prefix) {
        (Some(owner), _) => format!("{owner}::{rest}"),
        (None, true) => format!("crate::{rest}"),
        (None, false) => rest.to_owned(),
    }
}

/// The module a typed graft expression names, i.e. everything before the
/// `::NODE_ID` the expression has to end with.
/// 类型化 graft 表达式所命名的模块，即表达式结尾那个 `::NODE_ID` 之前的全部内容。
///
/// `None` means the expression does not end in `NODE_ID`, so there is no module
/// to name and no assertion to emit. That is a boundary, not a fallback: a cut
/// that cannot name both types is a cut whose substitution nothing checks, and
/// the caller is expected to say so rather than guess.
/// 返回 `None` 表示该表达式不以 `NODE_ID` 结尾，因此没有可命名的模块，也没有断言可发射。
/// 这是边界而不是兜底：命名不出两端类型的切口，其替换没有任何东西检查，调用方应当如实
/// 说明，而不是猜。
pub(super) fn face_module(expression: &str) -> Option<&str> {
    expression.trim().strip_suffix("::NODE_ID")
}

#[cfg(test)]
#[path = "owners_tests.rs"]
mod owners_tests;
