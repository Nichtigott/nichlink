//! Call-site scanning over already-extracted function bodies.
//! 对已提取函数体做调用点扫描。
//!
//! Comments, string literals, and macro invocations are not calls; a qualified
//! `Type::method(…)` and a turbofished `method::<T>(…)` still are.
//! 注释、字符串字面量与宏调用不算调用；限定路径 `Type::method(…)` 与带 turbofish 的
//! `method::<T>(…)` 仍算调用。

use std::collections::BTreeSet;

use super::{is_ident_continue, is_ident_start, mask_non_code};

/// Match a real function call in a body, ignoring comments, strings and macros.
/// 只匹配函数体里的真实调用，跳过注释、字符串和宏调用。
pub fn body_calls(body: &str, target: &str) -> bool {
    if target.is_empty() {
        return false;
    }
    let bytes = body.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }
        if bytes[index] == b'"' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' {
                    index = (index + 2).min(bytes.len());
                    continue;
                }
                let end = bytes[index] == b'"';
                index += 1;
                if end {
                    break;
                }
            }
            continue;
        }
        let is_start = bytes[index].is_ascii_alphabetic() || bytes[index] == b'_';
        if !is_start {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() && (bytes[index].is_ascii_alphanumeric() || bytes[index] == b'_')
        {
            index += 1;
        }
        let mut lookahead = index;
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
        }
        if bytes.get(lookahead) == Some(&b'!') {
            let macro_name = &body[start..index];
            if matches!(macro_name, "control_object" | "external_object") {
                index = lookahead + 1;
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
        if &body[start..index] != target {
            continue;
        }
        while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
            lookahead += 1;
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
        if !is_ident_start(bytes[index]) {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() && is_ident_continue(bytes[index]) {
            index += 1;
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
}
