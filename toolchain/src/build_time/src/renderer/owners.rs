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
use crate::build_time::registry_identity;
use crate::build_time::registry_syntax::GraftSyntax;
use crate::build_time::static_plan::StaticFaceRecord;

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
    // Match the **whole** expression against the owner table, taking the longest module that is a
    // prefix at a `::` boundary. Extracting "the module" first only works for `<module>::<ITEM>`; a
    // registration rule spells `<module>::REGISTRATION.registry_rule`, and splitting at the last `::`
    // produced `panel::frame::REGISTRATION`, an owner nobody has — which is why the facade emitted that
    // line bare and failed to resolve it (measured, §M7.38).
    // 用**整条**表达式去匹配属主表，取最长的、在 `::` 边界上构成前缀的模块。先抠出"那个模块"只对
    // `<模块>::<ITEM>` 成立；而注册规则写的是 `<模块>::REGISTRATION.registry_rule`，按最后一个 `::`
    // 切会得到 `panel::frame::REGISTRATION`——一个谁都不是的属主，这正是 facade 把那一行原样发出去、
    // 解析不到的原因（实测，§M7.38）。
    let owner = shape
        .owners
        .iter()
        .filter(|(path, _)| rest == path || rest.starts_with(&format!("{path}::")))
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

/// The registration rule one face asserts against, spelled for the crate that compiles it.
/// 某个面据以断言的那条注册规则，按编译它的 crate 拼写。
///
/// A parent that is an **ancestor shell** owns no `REGISTRATION` — the shell carries the ancestor's
/// `NODE_ID` and nothing else — so it asserts against `ANY`; so does a face whose parent is not a face at
/// all. The ghost reached that answer by accident (its face list holds no shells); the facade named the
/// shell and could not resolve it until this rule existed (measured, §M7.38).
/// **祖先壳**不拥有 `REGISTRATION`（壳只携带祖先的 `NODE_ID`），因此按 `ANY` 断言；父级根本不是面的面也一样。
/// 幽灵是**碰巧**得到这个答案的（它的面表里没有壳）；facade 点名了那个壳，直到有了这条规则才解析得到
/// （实测，§M7.38）。
pub(super) fn parent_rule(
    shape: &ShapeRender<'_>,
    modules: &std::collections::BTreeMap<registry_identity::NodeId, &str>,
    parent: registry_identity::NodeId,
    registry: &str,
) -> String {
    let parent_bytes = parent.into_bytes();
    let is_shell = shape.ancestors.iter().any(|(_, id)| *id == parent_bytes);
    match modules.get(&parent) {
        Some(module) if !is_shell => format!("{module}::REGISTRATION.registry_rule"),
        _ => format!("{registry}::RegistrationRule::ANY"),
    }
}

/// One `StaticGraftCut` constructor, spelled for the crate that emits it.
/// 一个 `StaticGraftCut` 构造式，按发射它的 crate 拼写。
///
/// A **typed** cut is emitted verbatim, so the compiler resolves the Rust expressions the author
/// wrote instead of a selector string — through `owned`, because `crate::` inside a facade means the
/// facade. A **string** cut keeps the historical form, with a range split into `new_range` from the
/// two endpoint fields — never by re-splitting the path text, which would cut a path that literally
/// contains `" to "` in half.
/// **类型化**切口原样发射，编译器因此解析作者写的 Rust 表达式，而不是选择器字符串——经由 `owned`，因为
/// facade 里的 `crate::` 指的是 facade 自己。**字符串**切口保持原有形式，区间用两个端点字段拆成
/// `new_range`——绝不重新拆分路径文本，否则字面含有 `" to "` 的路径会被拦腰截断。
pub(super) fn graft_constructor(
    shape: &ShapeRender<'_>,
    graft: &GraftSyntax,
    registry: &str,
) -> String {
    match &graft.expressions {
        Some(expressions) => match &expressions.cut_end {
            Some(end) => format!(
                "{registry}::StaticGraftCut::from_id_range({}, {}, {}, {})",
                owned(shape, &expressions.cut),
                owned(shape, end),
                owned(shape, &expressions.graft),
                graft.full
            ),
            None => format!(
                "{registry}::StaticGraftCut::from_ids({}, {}, {})",
                owned(shape, &expressions.cut),
                owned(shape, &expressions.graft),
                graft.full
            ),
        },
        None => match &graft.cut_end {
            Some(end) => format!(
                "{registry}::StaticGraftCut::new_range({:?}, {end:?}, {:?}, {})",
                graft.cut, graft.graft, graft.full
            ),
            None => format!(
                "{registry}::StaticGraftCut::new({:?}, {:?}, {})",
                graft.cut, graft.graft, graft.full
            ),
        },
    }
}

/// Whether a typed graft expression names a module **this** crate compiles.
/// 某个类型化 graft 表达式是否点名了**本** crate 编译的模块。
///
/// A crate may only name modules it has, and a partition is exactly a crate *not* having some: the
/// host hands `panel::frame` away, so a cut written inside it is a module the host no longer
/// compiles. Emitting the entry anyway failed with `error[E0433]: cannot find 'frame' in 'panel'`
/// (measured on the first end-to-end build whose host carried a graft **inside** a claimed subtree —
/// the fixture before it had no graft at all, which is why this hid). The entry belongs to the crates
/// that compile the face: the ghost that owns it, and the facade that carries the union.
/// 一个 crate 只许点名它拥有的模块，而分区正是一个 crate **没有**某些模块：宿主把 `panel::frame` 交出去，
/// 因此写在其**内**的切口是宿主不再编译的模块。照旧发射这条条目会报
/// `error[E0433]: cannot find 'frame' in 'panel'`（首次端到端构建实测：宿主的 graft 落在被认领的子树
/// **之内**——再往前那个夹具的宿主根本没有 graft，所以它一直躲着）。这条条目属于编译该面的 crate：拥有它的
/// 幽灵，以及携带并集的 facade。
///
/// `true` for an expression that names no module this rule can judge: a string selector is data, and a
/// typed cut the parser did not resolve to a `::NODE_ID` path names nothing here to check.
/// 对这条规则判不了的表达式返回 `true`：字符串选择器是数据，而解析器没能解析成 `::NODE_ID` 路径的类型化
/// 切口在这里没有可查的东西。
pub(super) fn names_a_compiled_module(faces: &[StaticFaceRecord], expression: &str) -> bool {
    match face_module(expression) {
        Some(module) => {
            let module = module.strip_prefix("crate::").unwrap_or(module);
            faces.iter().any(|face| face.module == module)
        }
        None => true,
    }
}

#[cfg(test)]
#[path = "owners_tests.rs"]
mod owners_tests;
