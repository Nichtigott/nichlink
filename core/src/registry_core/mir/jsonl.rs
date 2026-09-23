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
                value.push(match escaped {
                    b'"' => '"',
                    b'\\' => '\\',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    _ => return Err("unsupported string escape".to_owned()),
                });
            }
            byte if byte.is_ascii() => value.push(byte as char),
            _ => return Err("non-ASCII strings must be JSON escaped".to_owned()),
        }
    }
    Err("unterminated string".to_owned())
}

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
}
