//! Pure Rust-source lexer shared by search, callgraph, and indexing tools.
//! 供搜索、调用图与索引工具共用的纯 Rust 源码词法器。
//!
//! Every function here is a text transformation only: callers own file I/O.
//! 这里的所有函数只做文本变换，文件 I/O 由调用方负责。

use std::collections::BTreeSet;

/// One Rust function discovered in a source file.
/// 在源码文件中发现的一个 Rust 函数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFunction {
    pub name: String,
    pub signature: String,
    pub body: String,
    /// 1-based line of the opening `fn` token.
    /// 以 1 起始的 `fn` 起始行行号。
    pub line: u32,
    /// 1-based line of the closing brace.
    /// 以 1 起始的右花括号所在行行号。
    pub end_line: u32,
}

/// Find the 0-based inclusive line range of a function by name.
/// 按名称查找函数的 0 起始闭区间行范围。
pub fn function_source_range(lines: &[&str], name: &str) -> Option<(usize, usize)> {
    let start = lines.iter().position(|line| {
        let trimmed = line.trim_start();
        trimmed.contains("fn ") && trimmed.contains(&format!("{name}("))
    })?;
    let mut depth = 0usize;
    let mut opened = false;
    for (index, line) in lines.iter().enumerate().skip(start) {
        for character in line.chars() {
            match character {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' if opened => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
        if opened && depth == 0 {
            return Some((start, index));
        }
    }
    Some((start, start))
}

/// Index function bodies without treating comments, strings, or macro text as Rust.
/// 扫描函数体时屏蔽注释、字符串和宏文本，避免把它们误认成 Rust 函数。
pub fn function_symbols(source: &str) -> Vec<SourceFunction> {
    let masked = mask_non_code(source);
    let bytes = masked.as_bytes();
    let mut result = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if !is_ident_start(bytes[index]) {
            index += 1;
            continue;
        }
        let token_start = index;
        index += 1;
        while index < bytes.len() && is_ident_continue(bytes[index]) {
            index += 1;
        }
        if &masked[token_start..index] != "fn" {
            continue;
        }
        let mut name_start = index;
        while name_start < bytes.len() && bytes[name_start].is_ascii_whitespace() {
            name_start += 1;
        }
        if name_start >= bytes.len() || !is_ident_start(bytes[name_start]) {
            continue;
        }
        let mut name_end = name_start + 1;
        while name_end < bytes.len() && is_ident_continue(bytes[name_end]) {
            name_end += 1;
        }
        let name = masked[name_start..name_end].to_owned();
        let mut open = name_end;
        let mut angle_depth = 0usize;
        while open < bytes.len() {
            match bytes[open] {
                b'<' => angle_depth += 1,
                b'>' if angle_depth > 0 => angle_depth -= 1,
                b'{' if angle_depth == 0 => break,
                b';' if angle_depth == 0 => break,
                _ => {}
            }
            open += 1;
        }
        if open >= bytes.len() || bytes[open] != b'{' {
            continue;
        }
        let mut depth = 1usize;
        let mut close = open + 1;
        while close < bytes.len() && depth > 0 {
            match bytes[close] {
                b'{' => depth += 1,
                b'}' => depth = depth.saturating_sub(1),
                _ => {}
            }
            close += 1;
        }
        if depth != 0 {
            continue;
        }
        let line_start = source[..token_start].rfind('\n').map_or(0, |line| line + 1);
        let line = source[..token_start]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count() as u32
            + 1;
        let end_line = source[..close]
            .bytes()
            .filter(|byte| *byte == b'\n')
            .count() as u32
            + 1;
        let signature = source[line_start..open].trim().to_owned();
        result.push(SourceFunction {
            name,
            signature,
            body: source[open + 1..close - 1].to_owned(),
            line,
            end_line,
        });
        index = close;
    }
    result
}

fn is_ident_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_ident_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Replace comments and quoted literals with spaces while preserving offsets.
/// 用空格替换注释和引号字面量，同时保留原始偏移量。
fn mask_non_code(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                masked[index] = b' ';
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
            masked[index] = b' ';
            if index + 1 < bytes.len() {
                masked[index + 1] = b' ';
            }
            index += 2;
            let mut depth = 1usize;
            while index < bytes.len() && depth > 0 {
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    depth += 1;
                    masked[index] = b' ';
                    masked[index + 1] = b' ';
                    index += 2;
                } else if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                    depth = depth.saturating_sub(1);
                    masked[index] = b' ';
                    masked[index + 1] = b' ';
                    index += 2;
                } else {
                    if bytes[index] != b'\n' {
                        masked[index] = b' ';
                    }
                    index += 1;
                }
            }
            continue;
        }
        if bytes[index] == b'"' || bytes[index] == b'\'' {
            let quote = bytes[index];
            masked[index] = b' ';
            index += 1;
            while index < bytes.len() {
                let escaped = bytes[index] == b'\\';
                if bytes[index] != b'\n' {
                    masked[index] = b' ';
                }
                index += 1;
                if escaped && index < bytes.len() {
                    if bytes[index] != b'\n' {
                        masked[index] = b' ';
                    }
                    index += 1;
                } else if bytes[index - 1] == quote {
                    break;
                }
            }
            continue;
        }
        index += 1;
    }
    String::from_utf8(masked).unwrap_or_else(|_| source.to_owned())
}

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

/// Collect the deduplicated struct names used in `#[nichlink::object]`
/// registration declarations.
/// 收集 `#[nichlink::object]` 注册声明中的结构体名，去重并排序。
pub fn registration_kinds(source: &str) -> Vec<String> {
    let mut kinds = Vec::new();
    let mut pending_attribute = false;
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.contains("#[nichlink::object") {
            pending_attribute = true;
        }
        if pending_attribute && let Some((_, rest)) = trimmed.split_once("pub struct ") {
            let kind: String = rest
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            if !kind.is_empty()
                && kind
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')
            {
                kinds.push(kind);
            }
            pending_attribute = false;
        }
    }
    kinds.sort();
    kinds.dedup();
    kinds
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn registration_kinds_are_compact_and_deduplicated() {
        let kinds = registration_kinds(
            "#[nichlink::object]\npub struct Button;\n#[nichlink::object(parent = crate::control::NODE_ID)]\npub struct Button;",
        );
        assert_eq!(kinds, ["Button"]);
    }
}
