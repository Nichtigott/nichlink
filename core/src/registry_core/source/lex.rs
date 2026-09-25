//! The lexical helpers `mask_non_code` needs, kept beside it rather than inside it.
//! `mask_non_code` 需要的词法辅助函数，放在它旁边而不是它内部。
//!
//! `source.rs` is at the 450-line ceiling, and a raw string is a lexer rule rather than
//! a scanner caller: it belongs in its own file.
//! `source.rs` 已在 450 行上限上，而原始字符串是词法规则而不是扫描器调用方：它该有自己的文件。

/// The byte just past the raw string that starts at `index`, if one starts there.
/// 从 `index` 开始的原始字符串结束后的那个字节（若此处确实是一个原始字符串）。
///
/// Covers `r"…"`, `r#"…"#`, and the `br`/`cr` prefixes. The leading `b`/`c` is checked
/// from its own token boundary, so `br"…"` is a byte raw string while `subr"…"` is not
/// a raw string at all.
/// 覆盖 `r"…"`、`r#"…"#` 以及 `br`/`cr` 前缀。开头的 `b`/`c` 从它自己的 token 边界起算，
/// 因此 `br"…"` 是字节原始字符串，而 `subr"…"` 根本不是原始字符串。
pub(super) fn raw_string_end(bytes: &[u8], index: usize) -> Option<usize> {
    let (token_start, r_at) = match bytes.get(index) {
        Some(&b'b') | Some(&b'c') if bytes.get(index + 1) == Some(&b'r') => (index, index + 1),
        Some(&b'r') if index == 0 || !is_identifier_byte(bytes[index - 1]) => (index, index),
        _ => return None,
    };
    if token_start > 0 && is_identifier_byte(bytes[token_start - 1]) {
        return None;
    }
    let mut cursor = r_at + 1;
    let mut hashes = 0usize;
    while bytes.get(cursor) == Some(&b'#') {
        hashes += 1;
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b'"') {
        return None;
    }
    cursor += 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'"' {
            let mut after = cursor + 1;
            let mut seen = 0usize;
            while seen < hashes && bytes.get(after) == Some(&b'#') {
                seen += 1;
                after += 1;
            }
            if seen == hashes {
                return Some(after);
            }
        }
        cursor += 1;
    }
    // Unterminated: the rest of the file is the literal, as the lexer would have it.
    // 未闭合：文件的其余部分就是这个字面量，与词法器一致。
    Some(bytes.len())
}

/// Whether a byte can be part of an identifier.
/// 某个字节是否可以属于标识符。
pub(super) fn is_identifier_byte(byte: u8) -> bool {
    byte == b'_' || byte.is_ascii_alphanumeric()
}
