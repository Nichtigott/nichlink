//! Tests for the `graft_plan!` / `static_graft_plan!` entry grammar.
//! `graft_plan!` / `static_graft_plan!` 入口语法的测试。

use super::graft_entries;

#[test]
fn graft_parser_collects_single_and_full_cuts() {
    let source = r#"
nichlink::static_graft_plan!(FRAMEWORK,
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
    let source =
        r#"nichlink::graft_plan!(framework, cut ["root/a1" to "root/a3"] graft "replacement");"#;
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
    let source = r#"nichlink::static_graft_plan!(FRAMEWORK, cut "root/a to b" graft "g");"#;
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
        r#"nichlink::static_graft_plan!(FRAMEWORK, cut "root/a" full graft "replacement");"#;
    let entries = graft_entries(source).unwrap();
    assert_eq!(entries[0].cut, "root/a");
    assert!(entries[0].full);
}

#[test]
fn graft_parser_collects_declaration_only_static_plans() {
    let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
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
let _ = nichlink::graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
}
"#;
    assert!(graft_entries(in_function).unwrap().is_empty());

    let in_test_module = r#"
#[cfg(test)]
mod tests {
nichlink::static_graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
}
"#;
    assert!(graft_entries(in_test_module).unwrap().is_empty());

    let at_entry = r#"
#[cfg(feature = "optional-graft")]
nichlink::static_graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
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
    let source = r#"fn plan() { nichlink::graft!(framework, cut "root/a" graft "replacement"); }"#;
    assert!(graft_entries(source).unwrap().is_empty());
}

/// The typed form keeps both sides as Rust expressions so the compiler and
/// any editor that resolves Rust paths can see the real target.
/// 类型化形式把两侧都保留为 Rust 表达式，编译器和任何能解析 Rust 路径的编辑器
/// 因此都能看到真实目标。
#[test]
fn graft_parser_keeps_typed_expressions() {
    let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
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
    let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
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
    let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
        cut(crate::control::NODE_ID) graft "button_fast",
    );"#;
    let error = graft_entries(source).unwrap_err();
    assert!(
        error.message.contains("both sides"),
        "unexpected message: {}",
        error.message
    );
}
