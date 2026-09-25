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
