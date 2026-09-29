//! Call-site scanning over already-extracted function bodies.
//! 对已提取函数体做调用点扫描。
//!
//! Comments, string literals, and macro invocations are not calls; a qualified
//! `Type::method(…)` and a turbofished `method::<T>(…)` still are.
//! 注释、字符串字面量与宏调用不算调用；限定路径 `Type::method(…)` 与带 turbofish 的
//! `method::<T>(…)` 仍算调用。

use std::collections::BTreeSet;

use super::{is_ident_continue, is_ident_start, mask_non_code};

/// Match a real function call in a body, ignoring comments, strings, char literals and
/// macro invocations.
/// 只匹配函数体里的真实调用，跳过注释、字符串、字符字面量与宏调用。
///
/// The scan runs on the kernel's one masking rule (`mask_non_code`) instead of a
/// hand-rolled one. The local scanner lost the call after a `'"'` char literal and after a
/// raw string carrying an interior quote, and it invented one inside a nested comment,
/// while `direct_calls` on the same body saw the truth — the disagreement reached Studio's
/// call graph. What stays local is only the macro policy below, which is why these two
/// functions are not one.
/// 扫描跑在内核唯一的掩码规则（`mask_non_code`）上，而不是自己手写一套。本地扫描器在 `'"'`
/// 字符字面量之后、在带内部引号的 raw 字符串之后各丢了一个调用，又在嵌套注释里凭空造了一个，
/// 而 `direct_calls` 对同一函数体看到的是真相——这份分歧一路进了 Studio 的调用图。留在本地的
/// 只有下面那条宏策略，这也是这两个函数为什么没有合成一个。
pub fn body_calls(body: &str, target: &str) -> bool {
    if target.is_empty() {
        return false;
    }
    let masked = mask_non_code(body);
    let bytes = masked.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        // Identifiers are Unicode here too, and the boundary decode keeps the slice safe.
        // 这里的标识符同样是 Unicode 的，在边界解码也让切片保持安全。
        let Some(character) = masked[index..].chars().next() else {
            break;
        };
        if !is_ident_start(character) {
            index += character.len_utf8();
            continue;
        }
        let start = index;
        index += character.len_utf8();
        while let Some(next) = masked[index..].chars().next() {
            if !is_ident_continue(next) {
                break;
            }
            index += next.len_utf8();
        }
        // Whitespace after a name is ASCII in masked text, so byte stepping cannot land
        // inside a character.
        // 掩码文本里名字之后的空白是 ASCII，因此按字节前进不会落在字符中间。
        let mut lookahead = index;
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if bytes.get(lookahead) == Some(&b'!') {
            let macro_name = &masked[start..index];
            if matches!(macro_name, "control_object" | "external_object") {
                index = lookahead + 1;
                // Whitespace between the `!` and its delimiter is legal Rust, and reading
                // the delimiter without skipping it made the whole macro body look like
                // ordinary code: `control_object! { g(); }` reported `g` as a call.
                // `!` 与它的定界符之间的空白是合法 Rust；不跳过它直接读定界符，会让整个宏体看起来
                // 像普通代码：`control_object! { g(); }` 会把 `g` 报成调用。
                while index < bytes.len() && bytes[index].is_ascii_whitespace() {
                    index += 1;
                }
                if let Some(open) = bytes.get(index).copied() {
                    let close = match open {
                        b'(' => Some(b')'),
                        b'[' => Some(b']'),
                        b'{' => Some(b'}'),
                        _ => None,
                    };
                    if let Some(close) = close {
                        let mut depth = 0usize;
                        while index < bytes.len() {
                            if bytes[index] == open {
                                depth += 1;
                            }
                            if bytes[index] == close {
                                depth = depth.saturating_sub(1);
                                if depth == 0 {
                                    index += 1;
                                    break;
                                }
                            }
                            index += 1;
                        }
                    }
                }
                continue;
            }
            // Keep scanning ordinary macros: closures passed to tracing or
            // instrumentation macros still contain real function calls.
            // 普通宏内部继续扫描；追踪宏闭包里的调用仍是真实调用。
            index = lookahead + 1;
            continue;
        }
        if &masked[start..index] != target {
            continue;
        }
        if bytes.get(lookahead) == Some(&b':') && bytes.get(lookahead + 1) == Some(&b':') {
            lookahead += 2;
            while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                lookahead += 1;
            }
            if bytes.get(lookahead) == Some(&b'<') {
                let mut angle_depth = 0usize;
                while lookahead < bytes.len() {
                    match bytes[lookahead] {
                        b'<' => angle_depth += 1,
                        b'>' if angle_depth > 0 => {
                            angle_depth -= 1;
                            if angle_depth == 0 {
                                lookahead += 1;
                                break;
                            }
                        }
                        _ => {}
                    }
                    lookahead += 1;
                }
                while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                    lookahead += 1;
                }
            }
        }
        if bytes.get(lookahead) == Some(&b'(') {
            return true;
        }
    }
    false
}

/// List the distinct direct callees of one function body, in sorted order.
/// 列出函数体内的直接调用目标（去重并排序）。
///
/// Comments, string literals, and macro invocations are ignored, and the
/// function's own name (self-recursion) is excluded.
/// 忽略注释、字符串字面量与宏调用，并排除函数自身（自递归）。
pub fn direct_calls(body: &str, current: &str) -> Vec<String> {
    let masked = mask_non_code(body);
    let bytes = masked.as_bytes();
    let mut calls = BTreeSet::new();
    let mut index = 0usize;
    while index < bytes.len() {
        // Identifiers are Unicode, so the scan decodes the character at the boundary
        // instead of testing one byte: a non-ASCII name was cut short here too.
        // 标识符是 Unicode 的，因此这里在边界处解码字符而不是测一个字节：非 ASCII 的名字在这里
        // 同样会被截断。
        let Some(character) = masked[index..].chars().next() else {
            break;
        };
        if !is_ident_start(character) {
            index += character.len_utf8();
            continue;
        }
        let start = index;
        index += character.len_utf8();
        while let Some(next) = masked[index..].chars().next() {
            if !is_ident_continue(next) {
                break;
            }
            index += next.len_utf8();
        }
        let candidate = &masked[start..index];
        if candidate == current || matches!(candidate, "if" | "for" | "while" | "match" | "loop") {
            continue;
        }
        let mut lookahead = index;
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if bytes.get(lookahead) == Some(&b'(') {
            calls.insert(candidate.to_owned());
        }
    }
    calls.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{body_calls, direct_calls};

    #[test]
    fn call_scanner_ignores_use_and_macro_but_accepts_qualified_calls() {
        let body = r#"
            use crate::paint;
            control_object!(paint());
            self.paint::<Color>();
            Widget::layout();
            // paint()
            let text = "layout()";
        "#;
        assert!(body_calls(body, "paint"));
        assert!(body_calls(body, "layout"));
        assert!(!body_calls("use crate::paint;", "paint"));
        assert!(!body_calls("control_object!(paint());", "paint"));
        assert!(!body_calls("let text = \"paint()\";", "paint"));
    }

    #[test]
    fn direct_calls_are_sorted_distinct_and_exclude_the_function_itself() {
        let body = "let value = helper(1); sink(value); helper(2); if ok() {}";
        assert_eq!(direct_calls(body, "source"), ["helper", "ok", "sink"]);
        assert_eq!(
            direct_calls("fn source() { source(); }", "source"),
            Vec::<String>::new()
        );
        assert_eq!(direct_calls("// helper()", "source"), Vec::<String>::new());
    }

    /// The two scanners agree on one body. The hand-rolled scanner in `body_calls` lost the
    /// call after a `'"'` char literal and after a raw string carrying an interior quote,
    /// and invented one inside a nested comment, while `direct_calls` saw the truth — a
    /// disagreement that reached Studio's call graph, where one graph was wrong about calls
    /// the other reported.
    /// 两个扫描器在同一函数体上答案一致。`body_calls` 里手写的那套在 `'"'` 字符字面量之后、
    /// 在带内部引号的 raw 字符串之后各丢了一个调用，又在嵌套注释里凭空造了一个，而
    /// `direct_calls` 看到的是真相——这份分歧一路进了 Studio 的调用图，两张图对彼此报出的调用
    /// 各错一处。
    #[test]
    fn the_two_call_scanners_agree_on_one_body() {
        let bodies = [
            "fn f() { let quote = '\"'; g(); }",
            "fn f() { let raw = r##\"a \"# b\"##; g(); }",
            "fn f() { /* outer /* inner */ g(); */ h(); }",
        ];
        for body in bodies {
            let listed = direct_calls(body, "f");
            for target in ["g", "h"] {
                assert_eq!(
                    body_calls(body, target),
                    listed.iter().any(|call| call == target),
                    "`{target}` in `{body}`: the two scanners disagree"
                );
            }
        }
    }

    /// The one policy that stays local: a call inside a declaration macro's body is a
    /// declaration, not a call, so `body_calls` is silent where `direct_calls` reports it.
    /// This is why the two are not one function.
    /// 唯一留在本地的那条策略：声明宏体内的调用是声明而不是调用，因此 `direct_calls` 报出来的
    /// 东西，`body_calls` 在这里是沉默的。这正是它们没有合成一个函数的原因。
    #[test]
    fn a_declaration_macro_body_is_not_a_call_site() {
        let body = "fn f() { control_object! { g(); } }";
        assert!(!body_calls(body, "g"));
        assert_eq!(direct_calls(body, "f"), ["g"]);
    }
}
