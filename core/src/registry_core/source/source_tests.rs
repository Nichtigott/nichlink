//! Tests for the source scanners: what counts as code, and what does not.
//! 源码扫描器的测试：什么算代码，什么不算。

use super::*;

/// A lifetime is not an opening quote: every function with one used to
/// vanish from the index, and every call after a `&'static` was missed.
/// 生命周期不是引号起始：带生命周期的函数过去会从索引里整体消失，`&'static` 之后的调用
/// 也全部漏掉。
#[test]
fn lifetimes_do_not_mask_the_code_that_follows_them() {
    let source = "fn f<'a>(s: &'a str) -> &'a str { s }\nfn g() {}\n";
    let names = function_symbols(source)
        .into_iter()
        .map(|function| function.name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["f", "g"], "both functions must be indexed");

    // The same apostrophe in a body: the call after it must still be seen.
    // 同一个撇号出现在函数体里：它之后的调用仍必须被看到。
    let calls = direct_calls("let s: &'static str = paint();", "current");
    assert_eq!(calls, ["paint"]);
}

/// A character literal is still masked, so a `'{'` in one opens nothing.
/// 字符字面量仍被屏蔽，因此其中的 `'{'` 不会打开任何东西。
#[test]
fn character_literals_are_masked_like_strings() {
    let lines = ["fn first() {", "    let brace = '{';", "}", ""];
    assert_eq!(function_source_range(&lines, "first"), Some((0, 2)));
    let source = "fn f(c: char) { let _ = c == '{'; }\nfn g() {}\n";
    let names = function_symbols(source)
        .into_iter()
        .map(|function| function.name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["f", "g"]);
}

/// A brace inside a string is not a brace, and a function that never closes
/// is not a one-line function.
/// 字符串里的花括号不是花括号，而没有闭合的函数也不是单行函数。
#[test]
fn a_brace_in_a_string_does_not_close_the_function() {
    let lines = [
        "fn first() {",
        "    let s = \"{\";",
        "    other();",
        "}",
        "",
    ];
    assert_eq!(function_source_range(&lines, "first"), Some((0, 3)));
    let unterminated = ["fn first() {", "    other();"];
    assert_eq!(function_source_range(&unterminated, "first"), None);
}

#[test]
fn function_symbols_ignore_comments_and_index_impl_methods() {
    let source = r#"
            // fn ignored() {}
            pub(crate) async fn load(value: usize)
            where
                usize: Copy,
            {
                self.render::<usize>(value);
            }

            impl Widget {
                unsafe fn render(&self, value: usize) {
                    Type::paint(value);
                }
            }
        "#;
    let functions = function_symbols(source);
    assert_eq!(
        functions
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        ["load", "render"]
    );
    assert!(functions[0].signature.contains("pub(crate) async fn load"));
    assert!(functions[0].body.contains("self.render::<usize>(value)"));
    assert_eq!(functions[0].line, 3);
    assert_eq!(functions[0].end_line, 8);
    assert!(body_calls(&functions[0].body, "render"));
    assert!(body_calls(&functions[1].body, "paint"));
    assert!(!body_calls(&functions[1].body, "load"));
}

/// Line numbers survive a file with many functions, which is also the shape
/// the shared forward scan has to keep correct while it stops counting twice.
/// 行号在许多函数的文件里仍然正确——这也正是共享的前向扫描在不再重数之后必须保持
/// 正确的形状。
#[test]
fn every_function_reports_its_own_lines_in_a_long_file() {
    let mut source = String::new();
    for index in 0..200 {
        source.push_str(&format!("// filler {index}\n"));
        source.push_str(&format!("fn f{index}() {{\n    let x = {index};\n}}\n"));
    }
    let functions = function_symbols(&source);
    assert_eq!(functions.len(), 200);
    for (index, function) in functions.iter().enumerate() {
        assert_eq!(function.name, format!("f{index}"));
        let line = (index * 4 + 2) as u32;
        assert_eq!(function.line, line, "{}", function.name);
        assert_eq!(function.end_line, line + 2, "{}", function.name);
    }
}

#[test]
fn function_source_range_is_limited_to_the_named_function() {
    let lines = [
        "fn first() {",
        "    one();",
        "}",
        "",
        "pub(crate) fn second() {",
        "    two();",
        "}",
    ];
    assert_eq!(function_source_range(&lines, "second"), Some((4, 6)));
    assert_eq!(function_source_range(&lines, "missing"), None);
}

#[test]
fn registration_kinds_are_compact_and_deduplicated() {
    let kinds = registration_kinds(
        "crate::control_object! { kind: Button, }\ncrate::control_object! { kind: Button, }",
    );
    assert_eq!(kinds, ["Button"]);
}

/// A raw string is not closed by its first interior quote, so the code inside it is
/// prose: braces do not close a body, `fn` does not declare, and a call is not a call.
/// 原始字符串不由第一个内部引号闭合，因此它内部的代码是散文：花括号不闭合函数体、`fn` 不声明、
/// 调用也不是调用。
#[test]
fn raw_strings_do_not_leak_code() {
    // A JSON payload with an interior quote used to close the function early.
    // 带内部引号的 JSON 载荷过去会提前闭合函数。
    let lines = vec![
        "fn a() {",
        "    let s = r#\"x\" } \"#;",
        "    let t = 1;",
        "}",
        "fn b() {",
        "}",
    ];
    assert_eq!(
        function_source_range(&lines, "a"),
        Some((0, 3)),
        "the brace inside the raw string must not close the body"
    );

    // A declaration inside the payload is not a declaration.
    // 载荷内部的声明不是声明。
    let names = function_symbols("fn outer() {\n    let s = r#\"x\" fn ghost() {} \"#;\n}\n")
        .into_iter()
        .map(|function| function.name)
        .collect::<Vec<_>>();
    assert_eq!(names, ["outer"], "`ghost` lives inside a raw string");

    // A call inside the payload is not a call, and the `br`/`cr` prefixes are raw
    // strings too.
    // 载荷内部的调用不是调用，而 `br`/`cr` 前缀同样是原始字符串。
    assert!(direct_calls("let s = r#\"{\"k\": \"drop()\"}\"#;", "f").is_empty());
    assert!(direct_calls("let s = br#\"drop()\"#;", "f").is_empty());
}

/// The function-range start test runs on masked text and needs a whole identifier.
/// 函数范围的起点判断跑在屏蔽文本上，并要求完整标识符。
#[test]
fn a_phantom_function_range_needs_real_code() {
    // A name that appears only in a comment declares nothing.
    // 只出现在注释里的名字不声明任何东西。
    assert_eq!(
        function_source_range(
            &["// fn ghost() { details", "fn real() {", "    body();", "}"],
            "ghost"
        ),
        None
    );
    // `new` is a substring of `renew`, not a function name here.
    // 在这里 `new` 是 `renew` 的子串，而不是函数名。
    assert_eq!(function_source_range(&["fn renew() {", "}"], "new"), None);
    assert_eq!(
        function_source_range(&["fn new() {", "}"], "new"),
        Some((0, 1))
    );
}

/// A `kind:` in a comment is prose, not a declaration.
/// 注释里的 `kind:` 是散文，而不是声明。
#[test]
fn registration_kinds_come_from_code_only() {
    assert!(registration_kinds("// kind: Ghost").is_empty());
    let kinds = registration_kinds("crate::object! {\n    kind: Widget,\n}\n");
    assert_eq!(kinds, ["Widget"], "a real declaration is still collected");
}
