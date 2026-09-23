//! Pure Rust-source lexer shared by search, callgraph, and indexing tools.
//! 供搜索、调用图与索引工具共用的纯 Rust 源码词法器。
//!
//! Every function here is a text transformation only: callers own file I/O.
//! 这里的所有函数只做文本变换，文件 I/O 由调用方负责。
//!
//! Function discovery lives here; `calls` scans call sites inside an extracted
//! body and `walk` owns the recursive source traversal.
//! 函数发现位于本页；`calls` 扫描已提取函数体内的调用点，`walk` 拥有递归源码遍历。

#[path = "calls.rs"]
mod calls;
pub use calls::*;
#[path = "walk.rs"]
mod walk;
pub use walk::*;

/// One Rust function discovered in a source file.
/// 在源码文件中发现的一个 Rust 函数。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceFunction {
    /// Function name as written after the `fn` token.
    /// `fn` token 之后书写的函数名。
    pub name: String,
    /// Source text from the start of the line through the opening brace, trimmed.
    /// 从行首到左花括号的源码文本，已去除首尾空白。
    pub signature: String,
    /// Source text between the opening and closing braces.
    /// 左、右花括号之间的源码文本。
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

/// Collect the deduplicated `kind:` values used in registration declarations.
/// 收集注册声明中出现过的 `kind:` 取值，去重并排序。
pub fn registration_kinds(source: &str) -> Vec<String> {
    let mut kinds = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(kind) = trimmed.split_once("kind:").map(|(_, remainder)| remainder) {
            let kind = kind
                .trim()
                .split(|character: char| {
                    character == ',' || character == '}' || character.is_whitespace()
                })
                .next()
                .unwrap_or_default();
            if !kind.is_empty()
                && kind
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')
            {
                kinds.push(kind.to_owned());
            }
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
    fn registration_kinds_are_compact_and_deduplicated() {
        let kinds = registration_kinds(
            "crate::control_object! { kind: Button, }\ncrate::control_object! { kind: Button, }",
        );
        assert_eq!(kinds, ["Button"]);
    }
}
