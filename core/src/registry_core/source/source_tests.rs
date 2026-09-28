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

/// Masking one line at a time cannot see where a multi-line construct starts, so a
/// declaration inside a multi-line string, a raw string or a block comment was read as
/// code — the K4/K6 family, for the spellings that cross a line.
/// 逐行掩码看不见多行构造从哪里开始，因此多行字符串、raw 字符串或块注释里的声明会被读成代码
/// ——K4/K6 家族在跨行拼写上的翻版。
#[test]
fn multiline_literals_and_block_comments_do_not_leak_code() {
    let multiline = "fn real() {\n    let s = \"\nfn ghost() {\n\";\n    let _ = 1;\n}\n";
    let lines = multiline.lines().collect::<Vec<_>>();
    assert_eq!(
        function_source_range(&lines, "ghost"),
        None,
        "a `fn` inside a multi-line string is prose"
    );

    let string_kind = "fn real() {\n    let s = \"\nkind: Ghost\n\";\n}\n";
    assert!(
        registration_kinds(string_kind).is_empty(),
        "a `kind:` inside a multi-line string is prose"
    );

    let block_comment = "/*\nkind: Ghost\n*/\nfn real() {}\n";
    assert!(
        registration_kinds(block_comment).is_empty(),
        "a `kind:` inside a block comment is prose"
    );

    let raw = "fn real() {\n    let s = r#\"\nfn raw_ghost() {\n\"#;\n}\n";
    let lines = raw.lines().collect::<Vec<_>>();
    assert_eq!(
        function_source_range(&lines, "raw_ghost"),
        None,
        "a `fn` inside a raw string is prose"
    );
}

/// An identifier is Unicode. Scanning one byte at a time indexed `héllo` as `h`, so the
/// wrong symbol reached the MCP index, Studio and the build's function manifest.
/// 标识符是 Unicode 的。一次扫描一个字节会把 `héllo` 索引成 `h`，于是错误的符号进入 MCP 索引、
/// Studio 与构建产出的函数清单。
#[test]
fn a_non_ascii_identifier_is_indexed_whole() {
    let names = function_symbols("pub fn héllo() {}\npub fn plain() {}\n")
        .into_iter()
        .map(|function| function.name)
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        ["héllo", "plain"],
        "a non-ASCII name is one identifier, not its first ASCII letter"
    );

    let calls = direct_calls("pub fn caller() { héllo(); plain(); }", "caller");
    assert_eq!(
        calls,
        ["héllo", "plain"],
        "the call scanner agrees about the name"
    );
}

/// The masking fallback keeps the mask, never the unmasked source.
/// 掩码兜底保留掩码，绝不返回未掩码的源码。
///
/// The arm is unreachable today: the scan only writes ASCII spaces, and only at character
/// boundaries, so `mask` cannot turn valid UTF-8 into invalid bytes (audit `KRN-K-16` says
/// so itself). That is why this is a white-box pin on the *direction* rather than on a
/// reproducible input: the conversion takes only the bytes, so it has no access to the
/// source it would have to return — the worst direction ("comments and strings read as
/// code, so invented functions and calls") is unspellable.
/// 这一支今天不可达：扫描只写 ASCII 空格、且只在字符边界上写，因此 `mask` 不可能把合法 UTF-8
/// 变成非法字节（审计 `KRN-K-16` 自己就这么说）。这正是本条钉子钉的是**方向**而不是某个可复现
/// 输入的原因：转换只接收字节，因此拿不到"它本该返回的源码"——最坏的那个方向（"注释与字符串被当成
/// 代码，于是凭空造出函数与调用"）根本写不出来。
#[test]
fn the_mask_fallback_keeps_the_mask() {
    // A byte sequence the scanner cannot produce from a `&str`: 0xFF is not UTF-8.
    // 扫描器不可能从 `&str` 产出的字节序列：0xFF 不是 UTF-8。
    let text = masked_text(vec![b'l', b'e', b't', 0xff, b'x']);
    assert_eq!(
        text, "let\u{fffd}x",
        "the bytes that were masked stay masked, and the invalid byte is replaced rather \
         than dropped"
    );
}

/// Only a `kind:` inside a registration macro body is a declaration kind.
/// 只有注册宏体内的 `kind:` 才是声明 kind。
///
/// Red before the fix: the rule was "this line contains `kind:`", so an ordinary binding
/// contributed a kind named after its type, and a declaration whose value sat on the next
/// line contributed nothing. Both directions are invisible in the source query's output
/// (audit `KRN-K-17`).
/// 修前为红：规则是"本行含 `kind:`"，于是普通绑定按它的类型名贡献了一个 kind，而取值在下一行的
/// 声明什么都没贡献。两个方向在源码查询的输出里都看不出来（审计 `KRN-K-17`）。
#[test]
fn registration_kinds_are_bounded_by_the_declaration_body() {
    assert!(
        registration_kinds("let kind: String = value;").is_empty(),
        "an ordinary binding is not a registration declaration"
    );
    assert!(
        registration_kinds("fn f() {\n    let kind: String = value;\n}\n").is_empty(),
        "a binding inside a function body is not a declaration either"
    );
    assert_eq!(
        registration_kinds("crate::object! {\n    kind:\n        Widget,\n}\n"),
        ["Widget"],
        "a value on the next line is still that declaration's kind"
    );
    assert_eq!(
        registration_kinds("crate::control_object! { kind: Button, }"),
        ["Button"],
        "a one-line declaration still works"
    );
    assert!(
        registration_kinds(
            "fn f() {\n    let kind: String = value;\n}\ncrate::object! {\n    kind: Tool,\n}\n"
        ) == ["Tool"],
        "only the declaration's kind is collected"
    );
}
