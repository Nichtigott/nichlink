//! Pins for the flow-field parsers, including the nesting guard they share.
//! flow 字段解析器的钉子，含它们共用的嵌套守卫。

use super::*;

/// A deeply nested type path is refused instead of aborting the process.
/// 深度嵌套的类型路径会被拒绝，而不是让进程 abort。
///
/// The value comes from a manifest or the editor, so it is untrusted input to a
/// recursive-descent parser. Before the guard was added here, a 542-byte value
/// (`A<` repeated 180 times) overflowed the stack: not a panic, a `fatal runtime
/// error: stack overflow, aborting`, which no caller can catch. The assertion below
/// could not even run before the fix.
/// 这个值来自清单或编辑器，因此对递归下降解析器而言是不可信输入。在这里加上守卫之前，一条 542
/// 字节的值（`A<` 重复 180 次）会撑爆栈：那不是 panic，而是
/// `fatal runtime error: stack overflow, aborting`，任何调用方都捕获不了。修复之前，下面的断言
/// 根本跑不到。
#[test]
fn a_deep_type_path_is_refused_not_fatal() {
    let deep = format!("{}u8{}", "A<".repeat(200), ">".repeat(200));
    let error = render_flow_provider(&deep).expect_err("nesting is refused");
    assert!(
        error.message.contains("above the limit"),
        "the refusal names the nesting limit: {error}"
    );
}

/// The same guard covers the expression reader, which parses a generated face's
/// flow expression back out of a file.
/// 同一道守卫覆盖表达式读取器——它从文件里把生成面的 flow 表达式解析回来。
#[test]
fn a_deep_flow_expression_is_refused_not_fatal() {
    let deep = format!("{}NONE{}", "Wrapper::<".repeat(200), ">".repeat(200));
    let error = parse_flow_expression(&deep).expect_err("nesting is refused");
    assert!(
        error.message.contains("above the limit"),
        "the refusal names the nesting limit: {error}"
    );
}

/// A shallow path still renders, so the guard refuses nesting rather than the
/// feature.
/// 浅层路径照旧渲染，因此守卫拒绝的是嵌套而不是这个功能。
#[test]
fn a_shallow_path_still_renders() {
    assert_eq!(
        render_flow_provider("crate::flow::Render").expect("a shallow path"),
        "    flow_provider: crate::flow::Render,\n"
    );
}
