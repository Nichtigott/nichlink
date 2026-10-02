//! Tests for the `graft_plan!` / `static_graft_plan!` entry grammar.
//! `graft_plan!` / `static_graft_plan!` 入口语法的测试。

use super::{graft_entries, render_graft_expression};

#[test]
fn graft_parser_collects_single_and_full_cuts() {
    let source = r#"
nichlink_kernel::static_graft_plan!(FRAMEWORK,
cut ["root/a1/b2"] graft "canvas_fast",
cut ["root/a"] full graft "a_fast",
);
"#;
    let entries = graft_entries(source).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].cut, "root/a1/b2");
    assert!(!entries[0].full);
    assert_eq!(entries[1].graft, "a_fast");
    assert!(entries[1].full);
}

#[test]
fn graft_parser_keeps_range_endpoints() {
    let source = r#"nichlink_kernel::graft_plan!(framework, cut ["root/a1" to "root/a3"] graft "replacement");"#;
    let entries = graft_entries(source).unwrap();
    // Both endpoints survive as separate data; the start is `cut`, the far
    // endpoint is `cut_end`.
    // 两个端点都作为独立数据保留：起点是 `cut`，远端是 `cut_end`。
    assert_eq!(entries[0].cut, "root/a1");
    assert_eq!(entries[0].cut_end.as_deref(), Some("root/a3"));
}

/// A single string path that happens to contain `" to "` is not a range: only
/// a `to` token between two literals is. The parser must keep it whole, or
/// every consumer that used to re-split `cut` cuts it in half.
/// 一条字面含有 `" to "` 的字符串路径不是区间：只有两个字面量之间的 `to` token
/// 才是。解析器必须完整保留它，否则每个过去重新拆分 `cut` 的消费方都会把它拦腰
/// 截断。
#[test]
fn a_path_containing_the_range_word_is_a_single_cut() {
    let source = r#"nichlink_kernel::static_graft_plan!(FRAMEWORK, cut "root/a to b" graft "g");"#;
    let entries = graft_entries(source).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].cut, "root/a to b");
    assert_eq!(entries[0].cut_end, None);
}

#[test]
fn graft_parser_accepts_unbracketed_single_cut() {
    // A declaration belongs at the entry, so the fixture is an item; the
    // tests below cover plans that sit somewhere else.
    // 声明应当写在入口处，因此夹具写成条目；位置不当的计划由下面的测试覆盖。
    let source =
        r#"nichlink_kernel::static_graft_plan!(FRAMEWORK, cut "root/a" full graft "replacement");"#;
    let entries = graft_entries(source).unwrap();
    assert_eq!(entries[0].cut, "root/a");
    assert!(entries[0].full);
}

#[test]
fn graft_parser_collects_declaration_only_static_plans() {
    let source = r#"nichlink_kernel::static_graft_plan!(FRAMEWORK,
cut "root/a" graft "replacement",
);"#;
    let entries = graft_entries(source).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].cut, "root/a");
    assert_eq!(entries[0].graft, "replacement");
}

/// Only an item-position declaration belongs to the build's plan.
/// 只有条目位置的声明才属于构建计划。
#[test]
fn graft_parser_ignores_declarations_that_are_not_items() {
    let in_function = r#"
fn plan() {
let _ = nichlink_kernel::graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
}
"#;
    assert!(graft_entries(in_function).unwrap().is_empty());

    let in_test_module = r#"
#[cfg(test)]
mod tests {
nichlink_kernel::static_graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
}
"#;
    assert!(graft_entries(in_test_module).unwrap().is_empty());

    let at_entry = r#"
#[cfg(feature = "optional-graft")]
nichlink_kernel::static_graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
"#;
    let entries = graft_entries(at_entry).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].cfg.as_deref(),
        Some("feature = \"optional-graft\"")
    );
}

#[test]
fn graft_parser_ignores_removed_macro_names() {
    let source =
        r#"fn plan() { nichlink_kernel::graft!(framework, cut "root/a" graft "replacement"); }"#;
    assert!(graft_entries(source).unwrap().is_empty());
}

/// The typed form keeps both sides as Rust expressions so the compiler and
/// any editor that resolves Rust paths can see the real target.
/// 类型化形式把两侧都保留为 Rust 表达式，编译器和任何能解析 Rust 路径的编辑器
/// 因此都能看到真实目标。
#[test]
fn graft_parser_keeps_typed_expressions() {
    let source = r#"nichlink_kernel::static_graft_plan!(FRAMEWORK,
        cut(crate::control::object::button::NODE_ID)
            graft(graft_crate::button_fast::NODE_ID),
    );"#;
    let entries = graft_entries(source).unwrap();
    let expressions = entries[0].expressions.as_ref().expect("typed expressions");
    assert_eq!(
        expressions.cut, "crate::control::object::button::NODE_ID",
        "the host path is preserved verbatim so the compiler resolves it"
    );
    assert_eq!(expressions.graft, "graft_crate::button_fast::NODE_ID");
    assert_eq!(expressions.cut_end, None);
    assert!(!entries[0].full);
}

#[test]
fn graft_parser_keeps_a_typed_sibling_range() {
    let source = r#"nichlink_kernel::static_graft_plan!(FRAMEWORK,
        cut(crate::control::object::button::NODE_ID to crate::control::object::slider::NODE_ID)
            graft(graft_crate::fast::NODE_ID),
    );"#;
    let entries = graft_entries(source).unwrap();
    let expressions = entries[0].expressions.as_ref().expect("typed expressions");
    assert_eq!(expressions.cut, "crate::control::object::button::NODE_ID");
    assert_eq!(
        expressions.cut_end.as_deref(),
        Some("crate::control::object::slider::NODE_ID")
    );
    // The top-level fields carry the same two endpoints as data; the joined
    // `"start to end"` text is gone, so nothing downstream has to re-split.
    // 顶层字段以数据形式携带同样的两个端点；拼接的 `"start to end"` 文本已删除，
    // 下游无需再拆分。
    assert_eq!(entries[0].cut, "crate::control::object::button::NODE_ID");
    assert_eq!(
        entries[0].cut_end.as_deref(),
        Some("crate::control::object::slider::NODE_ID")
    );
}

/// A cut may not mix a resolved identity with an unresolved name.
/// 一条切口不允许混合"已解析身份"与"未解析名称"。
#[test]
fn graft_parser_rejects_mixed_typed_and_string_sides() {
    let source = r#"nichlink_kernel::static_graft_plan!(FRAMEWORK,
        cut(crate::control::NODE_ID) graft "button_fast",
    );"#;
    let error = graft_entries(source).unwrap_err();
    assert!(
        error.message.contains("both sides"),
        "unexpected message: {}",
        error.message
    );
}

/// A rendered declaration parses back into the same structure — the pair cannot drift.
/// 渲染出来的声明解析回来是同一个结构——这一对无法漂移。
///
/// This is the acceptance for the writing half: the bridge is about to write graft declarations
/// into host sources, and the only thing that keeps a generated spelling honest is that the
/// repository's own parser reads it back as the declaration the caller meant.
/// 这是**写出**那一半的验收：桥即将把 graft 声明写进宿主源码，而让生成的拼写保持诚实的唯一办法，是本仓
/// 自己的解析器把它读回成调用方本意的那条声明。
#[test]
fn a_rendered_declaration_parses_back_into_the_same_structure() {
    let line = render_graft_expression(
        "crate::control::object::button::NODE_ID",
        None,
        true,
        "control_button_graft::button_fast::NODE_ID",
    )
    .expect("the typed-expression form renders");
    let source = format!("static_graft_plan! {{ {line} }}");
    let parsed = graft_entries(&source).expect("and parses back");
    assert_eq!(parsed.len(), 1, "{parsed:?}");
    assert_eq!(parsed[0].cut, "crate::control::object::button::NODE_ID");
    assert_eq!(
        parsed[0].graft,
        "control_button_graft::button_fast::NODE_ID"
    );
    assert!(parsed[0].full, "`full` survives the round trip");
    assert!(parsed[0].cut_end.is_none());
    assert!(parsed[0].expressions.is_some(), "it is the typed form");
}

/// A range keeps both endpoints through the round trip.
/// 区间经回环后两个端点都在。
#[test]
fn a_rendered_range_keeps_both_endpoints() {
    let line = render_graft_expression(
        "crate::control::object::button::NODE_ID",
        Some("crate::control::object::slider::NODE_ID"),
        false,
        "control_button_graft::button_fast::NODE_ID",
    )
    .expect("a typed range renders");
    let parsed =
        graft_entries(&format!("static_graft_plan! {{ {line} }}")).expect("and parses back");
    assert_eq!(parsed[0].cut, "crate::control::object::button::NODE_ID");
    assert_eq!(
        parsed[0].cut_end.as_deref(),
        Some("crate::control::object::slider::NODE_ID"),
        "the far endpoint is data, not part of the start path"
    );
    assert!(!parsed[0].full);
}

/// The renderer refuses the spellings it cannot guarantee, and says why.
/// 渲染器拒绝它无法保证的拼写，并说出原因。
///
/// The `to` case is the grammar's own trap: the parser reads a top-level `to` ident as a range
/// separator, so a path whose segment is `to` would silently become a range. Writing it would put a
/// declaration in a host that means something else than the caller asked for.
/// `to` 那一条是语法自带的陷阱：解析器把顶层的 `to` 识别符读成区间分隔符，因此段名为 `to` 的路径会静默变成
/// 区间。写下去就等于在宿主里放了一条与调用方所求不同的声明。
#[test]
fn the_renderer_refuses_a_spelling_it_cannot_guarantee() {
    let refused = render_graft_expression("crate::to::NODE_ID", None, false, "x::y")
        .expect_err("a `to` segment is read as a range separator");
    assert!(refused.message.contains("range separator"), "{refused:?}");
    let refused = render_graft_expression("crate::a::NODE_ID", None, false, "\"a string\"")
        .expect_err("the literal spelling is a different shape");
    assert!(refused.message.contains("another spelling"), "{refused:?}");
    let refused = render_graft_expression("", None, false, "x::y").expect_err("empty is refused");
    assert!(refused.message.contains("must not be empty"), "{refused:?}");
    assert!(
        refused.location.is_none(),
        "nothing was written yet, so there is no location to point at"
    );
}
