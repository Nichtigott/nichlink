//! JSONL MIR artifact parser.
//! JSONL MIR artifact 解析器。
//!
//! The compact artifact is hand-parsed so the kernel keeps zero JSON
//! dependency: every line is one object with a `kind` discriminator.
//! 紧凑 artifact 采用手写解析，kernel 因此保持零 JSON 依赖：每行是一个带 `kind`
//! 判别字段的对象。

use std::collections::HashMap;

use super::model::{MirCall, MirGraph, MirLocal, MirParseError};

impl MirGraph {
    /// Parse the compact JSONL artifact, one record per non-blank line.
    /// 解析紧凑 JSONL artifact，每个非空行一条记录。
    ///
    /// A malformed line fails the whole parse with its 1-based line number;
    /// blank lines are skipped. Each record needs a `kind` of `function`,
    /// `call`, or `local`, and any other kind is rejected.
    /// 任一行格式错误都会以使整个解析失败，并带上其以 1 起始的行号；空行跳过。
    /// 每条记录需要 `kind` 为 `function`、`call` 或 `local`，其他类型一律拒绝。
    pub fn from_jsonl(input: &str) -> Result<Self, MirParseError> {
        let mut graph = Self::default();
        for (index, raw) in input.lines().enumerate() {
            let line = index + 1;
            if raw.trim().is_empty() {
                continue;
            }
            let fields = parse_object(raw).map_err(|message| MirParseError { line, message })?;
            let kind = required(&fields, "kind", line)?;
            match kind {
                "function" => {
                    graph
                        .functions
                        .insert(required(&fields, "name", line)?.to_owned());
                }
                "call" => graph.calls.push(MirCall {
                    caller: required(&fields, "caller", line)?.to_owned(),
                    callee: required(&fields, "callee", line)?.to_owned(),
                    mir_line: number(&fields, "mir_line", line)?,
                }),
                "local" => graph.locals.push(MirLocal {
                    function: required(&fields, "function", line)?.to_owned(),
                    name: required(&fields, "name", line)?.to_owned(),
                    type_name: required(&fields, "type", line)?.to_owned(),
                    mir_line: number(&fields, "mir_line", line)?,
                }),
                other => {
                    return Err(MirParseError {
                        line,
                        message: format!("unsupported record kind `{other}`"),
                    });
                }
            }
        }
        Ok(graph)
    }
}

fn required<'a>(
    fields: &'a HashMap<String, String>,
    key: &str,
    line: usize,
) -> Result<&'a str, MirParseError> {
    fields
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| MirParseError {
            line,
            message: format!("missing `{key}`"),
        })
}

fn number(
    fields: &HashMap<String, String>,
    key: &str,
    line: usize,
) -> Result<usize, MirParseError> {
    required(fields, key, line)?
        .parse()
        .map_err(|_| MirParseError {
            line,
            message: format!("`{key}` is not an integer"),
        })
}

fn parse_object(input: &str) -> Result<HashMap<String, String>, String> {
    let bytes = input.as_bytes();
    let mut cursor = 0;
    skip_space(bytes, &mut cursor);
    if bytes.get(cursor) != Some(&b'{') {
        return Err("record must start with `{`".to_owned());
    }
    cursor += 1;
    let mut fields = HashMap::new();
    loop {
        skip_space(bytes, &mut cursor);
        if bytes.get(cursor) == Some(&b'}') {
            return Ok(fields);
        }
        let key = quoted(bytes, &mut cursor)?;
        skip_space(bytes, &mut cursor);
        if bytes.get(cursor) != Some(&b':') {
            return Err("expected `:` after key".to_owned());
        }
        cursor += 1;
        skip_space(bytes, &mut cursor);
        let value = if bytes.get(cursor) == Some(&b'"') {
            quoted(bytes, &mut cursor)?
        } else {
            let start = cursor;
            while cursor < bytes.len()
                && !matches!(bytes[cursor], b',' | b'}' | b' ' | b'\n' | b'\r' | b'\t')
            {
                cursor += 1;
            }
            if start == cursor {
                return Err("expected a value".to_owned());
            }
            String::from_utf8(bytes[start..cursor].to_vec())
                .map_err(|_| "value is not UTF-8".to_owned())?
        };
        if fields.insert(key, value).is_some() {
            return Err("duplicate field".to_owned());
        }
        skip_space(bytes, &mut cursor);
        match bytes.get(cursor) {
            Some(b',') => cursor += 1,
            Some(b'}') => return Ok(fields),
            _ => return Err("expected `,` or `}`".to_owned()),
        }
    }
}

fn quoted(bytes: &[u8], cursor: &mut usize) -> Result<String, String> {
    if bytes.get(*cursor) != Some(&b'"') {
        return Err("expected a quoted string".to_owned());
    }
    *cursor += 1;
    let mut value = String::new();
    while let Some(byte) = bytes.get(*cursor).copied() {
        *cursor += 1;
        match byte {
            b'"' => return Ok(value),
            b'\\' => {
                let escaped = bytes.get(*cursor).copied().ok_or("unterminated escape")?;
                *cursor += 1;
                match escaped {
                    b'"' => value.push('"'),
                    b'\\' => value.push('\\'),
                    b'/' => value.push('/'),
                    b'b' => value.push('\u{8}'),
                    b'f' => value.push('\u{c}'),
                    b'n' => value.push('\n'),
                    b'r' => value.push('\r'),
                    b't' => value.push('\t'),
                    b'u' => value.push(decode_unicode_escape(bytes, cursor)?),
                    _ => return Err("unsupported string escape".to_owned()),
                }
            }
            byte if byte.is_ascii() => value.push(byte as char),
            _ => {
                // Raw UTF-8 is valid JSON, and it is exactly what this crate's own writer
                // emits: rejecting every byte at or above 0x80 made a non-ASCII symbol
                // (`crate::héllo`, and Rust identifiers may be non-ASCII) impossible to
                // read back in any encoding — the error told the producer to escape the
                // string while `\u` was refused too.
                // 原始 UTF-8 是合法的 JSON，而且正是本 crate 自己的写入器产出的形式：拒绝所有
                // ≥ 0x80 的字节让非 ASCII 符号（`crate::héllo`，而 Rust 标识符可以是非 ASCII）
                // 在任何编码下都读不回来——那条错误让生产者去转义，而 `\u` 同样被拒。
                let start = *cursor - 1;
                let width = utf8_width(byte);
                let end = start + width;
                if width == 0 || end > bytes.len() {
                    return Err("invalid UTF-8 in string".to_owned());
                }
                let text = core::str::from_utf8(&bytes[start..end])
                    .map_err(|_| "invalid UTF-8 in string".to_owned())?;
                value.push_str(text);
                *cursor = end;
            }
        }
    }
    Err("unterminated string".to_owned())
}

/// How many bytes the UTF-8 sequence that starts with `byte` occupies, or zero when it
/// cannot start one.
/// 以 `byte` 开头的 UTF-8 序列占几个字节；它不能作为起始时为零。
fn utf8_width(byte: u8) -> usize {
    match byte {
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => 0,
    }
}

/// Decode a `\uXXXX` escape, pairing a surrogate when JSON wrote one.
/// 解码 `\uXXXX` 转义；JSON 写成代理对时把一对合起来解。
fn decode_unicode_escape(bytes: &[u8], cursor: &mut usize) -> Result<char, String> {
    let code = hex4(bytes, cursor)?;
    if (0xD800..0xDC00).contains(&code) {
        // A high surrogate must be followed by a low one: JSON writes an astral code
        // point as two escapes, and neither half is a `char` on its own.
        // 高代理后面必须跟一个低代理：JSON 把星平面码点写成两个转义，而任何一半单独都不是 `char`。
        if bytes.get(*cursor..*cursor + 2) != Some(b"\\u") {
            return Err("a high surrogate needs a low surrogate".to_owned());
        }
        *cursor += 2;
        let low = hex4(bytes, cursor)?;
        if !(0xDC00..0xE000).contains(&low) {
            return Err("a high surrogate needs a low surrogate".to_owned());
        }
        let combined = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
        return char::from_u32(combined).ok_or_else(|| "invalid \\u escape".to_owned());
    }
    char::from_u32(code).ok_or_else(|| "invalid \\u escape".to_owned())
}

/// Read four hex digits at `cursor`.
/// 在 `cursor` 处读四位十六进制。
fn hex4(bytes: &[u8], cursor: &mut usize) -> Result<u32, String> {
    let digits = bytes
        .get(*cursor..*cursor + 4)
        .ok_or_else(|| "truncated \\u escape".to_owned())?;
    let text = core::str::from_utf8(digits).map_err(|_| "invalid \\u escape".to_owned())?;
    let code = u32::from_str_radix(text, 16).map_err(|_| "invalid \\u escape".to_owned())?;
    *cursor += 4;
    Ok(code)
}

/// Skip the whitespace between two JSON tokens.
/// 跳过两个 JSON token 之间的空白。
fn skip_space(bytes: &[u8], cursor: &mut usize) {
    while bytes
        .get(*cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        *cursor += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::MirGraph;

    #[test]
    fn parses_jsonl_without_json_dependency() {
        let graph = MirGraph::from_jsonl(
            "{\"kind\":\"function\",\"name\":\"crate::a\"}\n{\"kind\":\"call\",\"caller\":\"crate::a\",\"callee\":\"crate::b\",\"mir_line\":12}\n{\"kind\":\"local\",\"function\":\"crate::a\",\"name\":\"_1\",\"type\":\"f32\",\"mir_line\":14}\n",
        )
        .unwrap();
        assert!(graph.functions.contains("crate::a"));
        assert_eq!(graph.calls[0].mir_line, 12);
        assert_eq!(graph.locals[0].type_name, "f32");
    }
    /// A non-ASCII symbol round-trips, and an escaped form decodes.
    /// 非 ASCII 符号能往返，转义形式也能解码。
    ///
    /// The writer emits raw UTF-8; the reader used to reject every byte at or above
    /// 0x80 *and* every `\u` escape, so no encoding of `crate::héllo` could be read
    /// back — while the refusal message told the producer to escape the string.
    /// 写入器产出原始 UTF-8；读取器过去既拒绝所有 ≥ 0x80 的字节，也拒绝每一个 `\u` 转义，
    /// 因此 `crate::héllo` 在任何编码下都读不回来——而拒绝信息还在让生产者去转义。
    #[test]
    fn a_non_ascii_symbol_round_trips() {
        let line = "{\"kind\":\"function\",\"name\":\"crate::héllo\"}\n";
        let graph = MirGraph::from_jsonl(line).expect("raw UTF-8 is valid JSON");
        assert!(
            graph.functions.iter().any(|name| name == "crate::héllo"),
            "the raw form reads back: {:?}",
            graph.functions
        );
        let again = MirGraph::from_jsonl(&graph.to_jsonl()).expect("the writer's own output");
        assert!(
            again.functions.iter().any(|name| name == "crate::héllo"),
            "the artifact round-trips: {:?}",
            again.functions
        );

        // An escaped form is accepted too, including an astral code point written as a
        // surrogate pair.
        // 转义形式同样被接受，包括写成代理对的星平面码点。
        let escaped = MirGraph::from_jsonl(
            "{\"kind\":\"function\",\"name\":\"caf\\u00e9 \\ud83d\\ude00\"}\n",
        )
        .expect("escapes are accepted");
        assert!(
            escaped.functions.iter().any(|name| name == "café 😀"),
            "escapes decode: {:?}",
            escaped.functions
        );
    }
}
