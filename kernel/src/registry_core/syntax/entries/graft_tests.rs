//! Tests for the `graft_plan!` / `static_graft_plan!` entry grammar.
//! `graft_plan!` / `static_graft_plan!` 入口语法的测试。

use super::{graft_entries, render_graft_expression};

#[test]
fn graft_parser_collects_single_and_full_cuts() {
    let source = r#"
xirang_kernel::static_graft_plan!(FRAMEWORK,
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
    let source = r#"xirang_kernel::graft_plan!(framework, cut ["root/a1" to "root/a3"] graft "replacement");"#;
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
    let source = r#"xirang_kernel::static_graft_plan!(FRAMEWORK, cut "root/a to b" graft "g");"#;
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
        r#"xirang_kernel::static_graft_plan!(FRAMEWORK, cut "root/a" full graft "replacement");"#;
    let entries = graft_entries(source).unwrap();
    assert_eq!(entries[0].cut, "root/a");
    assert!(entries[0].full);
}

#[test]
fn graft_parser_collects_declaration_only_static_plans() {
    let source = r#"xirang_kernel::static_graft_plan!(FRAMEWORK,
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
let _ = xirang_kernel::graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
}
"#;
    assert!(graft_entries(in_function).unwrap().is_empty());

    let in_test_module = r#"
#[cfg(test)]
mod tests {
xirang_kernel::static_graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
}
"#;
    assert!(graft_entries(in_test_module).unwrap().is_empty());

    let at_entry = r#"
#[cfg(feature = "optional-graft")]
xirang_kernel::static_graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
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
        r#"fn plan() { xirang_kernel::graft!(framework, cut "root/a" graft "replacement"); }"#;
    assert!(graft_entries(source).unwrap().is_empty());
}

/// The typed form keeps both sides as Rust expressions so the compiler and
/// any editor that resolves Rust paths can see the real target.
/// 类型化形式把两侧都保留为 Rust 表达式，编译器和任何能解析 Rust 路径的编辑器
/// 因此都能看到真实目标。
#[test]
fn graft_parser_keeps_typed_expressions() {
    let source = r#"xirang_kernel::static_graft_plan!(FRAMEWORK,
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
    let source = r#"xirang_kernel::static_graft_plan!(FRAMEWORK,
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
    let source = r#"xirang_kernel::static_graft_plan!(FRAMEWORK,
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

/// `full` on a range is refused by name, in **both** directions of the grammar, and the two legal
/// neighbours still work.
/// 区间上的 `full` 在语法的**两个**方向上都按名拒绝，而它两个合法邻居照旧可用。
///
/// Measured before this pin: the shape was accepted, and the build emitted
/// `StaticGraftCut::from_id_range(…, true)` — a cut that replaces each slot of a sibling run
/// together with its subtree. Nothing anywhere said which of those two readings it means, and a
/// tool that writes declarations into a host's own source must not emit a spelling whose meaning it
/// cannot vouch for. The way out is explicit: one cut per face, or drop `full`.
/// 本钉子之前实测：这一形被接受，构建发出 `StaticGraftCut::from_id_range(…, true)`——一条把一段兄弟里
/// **每个**槽位连同其子树一起替换的切口。两种读法里到底是哪一种，哪里都没写；而一个往宿主自己的源码里写
/// 声明的工具，绝不能发出自己保证不了含义的拼写。出路是显式的那两条：逐面写切口，或者去掉 `full`。
#[test]
fn full_on_a_range_is_refused_in_both_directions() {
    for source in [
        r#"static_graft_plan! { cut ["root/a" to "root/c"] full graft "replacement" }"#,
        r#"static_graft_plan! { cut(crate::a::NODE_ID to crate::b::NODE_ID) full graft(fast::NODE_ID) }"#,
    ] {
        let error = graft_entries(source)
            .expect_err("a range with `full` is refused")
            .to_string();
        assert!(
            error.contains("`full` on a range cut has no defined meaning"),
            "{source}: {error}"
        );
        assert!(
            error.contains("Write one cut per face, or drop `full`"),
            "the refusal carries the way out: {source}: {error}"
        );
    }

    let refused = render_graft_expression(
        "crate::a::NODE_ID",
        Some("crate::b::NODE_ID"),
        true,
        "fast::NODE_ID",
    )
    .expect_err("the renderer must not write what its own parser refuses");
    assert!(
        refused
            .message
            .contains("`full` on a range cut has no defined meaning"),
        "{refused:?}"
    );

    // The two neighbours stay legal: a range without `full`, and `full` without a range.
    // 两个邻居照旧合法：不带 `full` 的区间，以及不带区间的 `full`。
    let range = render_graft_expression(
        "crate::a::NODE_ID",
        Some("crate::b::NODE_ID"),
        false,
        "fast::NODE_ID",
    )
    .expect("a range without `full` still renders");
    assert!(graft_entries(&format!("static_graft_plan! {{ {range} }}")).is_ok());
    let single = render_graft_expression("crate::a::NODE_ID", None, true, "fast::NODE_ID")
        .expect("`full` without a range still renders");
    let parsed = graft_entries(&format!("static_graft_plan! {{ {single} }}"))
        .expect("and still parses back");
    assert!(parsed[0].full && parsed[0].cut_end.is_none());
}

/// Every entry reports the line its own cut is on, not the line the macro starts on.
/// 每条条目报告的是**它自己那条切口**所在的行，而不是宏开始的那一行。
///
/// This used to be one location for all of them — the `static_graft_plan!(` line — which made
/// `graft_plan.tsv` claim two cuts sat on one line and pointed every refusal message at the macro.
/// Measured before the fix on the example host: both rows said `48 1` while the cuts are on lines
/// 50 and 52 (audit 2026-10-06).
/// 过去它们共用一个位置——`static_graft_plan!(` 那一行——于是 `graft_plan.tsv` 声称两条切口在同一行，
/// 而每条拒绝文案都指向那个宏。修前在示例宿主上实测：两行都写 `48 1`，而两条切口在第 50 与第 52 行
/// （审计 2026-10-06）。
#[test]
fn each_entry_reports_its_own_location() {
    let source = "xirang_kernel::static_graft_plan!(\n    FRAMEWORK,\n    cut(a::one::NODE_ID)\n        graft(g::one::NODE_ID),\n    cut(a::two::NODE_ID) graft(g::two::NODE_ID),\n);\n";
    let entries = graft_entries(source).expect("the declaration parses");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].location.line, 3, "{:?}", entries[0].location);
    assert_eq!(entries[1].location.line, 5, "{:?}", entries[1].location);
    assert_ne!(
        entries[0].location, entries[1].location,
        "two entries on two lines cannot share one location"
    );
}

/// A refusal inside one entry points at that entry's cut, not at the macro that carries it.
/// 一条条目内部的拒绝指向**那条条目**的切口，而不是承载它的宏。
#[test]
fn a_refusal_points_at_the_entry_it_is_about() {
    let source = "xirang_kernel::static_graft_plan!(\n    FRAMEWORK,\n    cut(a::one::NODE_ID) graft(g::one::NODE_ID),\n    cut(a::two::NODE_ID) wrong(g::two::NODE_ID),\n);\n";
    let error = graft_entries(source).expect_err("the second entry is malformed");
    let location = error
        .location
        .expect("a refusal inside an entry can point at it");
    assert_eq!(location.line, 4, "the refusal belongs to the fourth line");
}

/// `full` after `graft` is refused by naming where `full` belongs — not by saying the grammar is
/// wrong. Measured: this is the spelling readers reach for (`cut(a) to b graft(c) full`), and the
/// older message ("expects `graft <implementation>`") sent them looking for a missing `graft` that
/// was already there.
/// `full` 写在 `graft` 之后时，拒绝要点出 `full` 该写在哪，而不是说语法不对。实测：这正是读者会写出的
/// 拼法（`cut(a) to b graft(c) full`），而旧文案（"expects `graft <implementation>`"）会让人去找一个
/// 早就写在那里的 `graft`。
#[test]
fn full_written_after_graft_is_refused_by_naming_where_it_belongs() {
    let source = r#"xirang_kernel::graft_plan!(framework, cut ["root/a" to "root/a3"] graft "replacement" full);"#;
    let error = graft_entries(source).expect_err("refused").to_string();
    assert!(
        error.contains("**before** `graft`"),
        "it says where `full` belongs: {error}"
    );
    assert!(
        !error.contains("expects `graft <implementation>`"),
        "and does not send the reader looking for a `graft` that is already written: {error}"
    );
}
