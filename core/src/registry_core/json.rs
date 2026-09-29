//! JSON string encoding, shared by every artifact this workspace writes.
//! 本工作区写出的每个 JSON 工件共用的 JSON 字符串编码。
//!
//! Three artifacts need it, and they used to disagree: the build-diagnostics
//! document, the MIR JSONL artifact, and the generated editor-snippet file. Two
//! of the three carried a private "replace `\` and `"`" copy, which is not JSON
//! encoding at all — RFC 8259 also requires every code point below U+0020 to be
//! escaped — so those two could emit a raw tab or newline inside a string.
//! 有三份工件需要它，而它们此前并不一致：构建诊断文档、MIR JSONL 工件、以及生成的编辑器
//! 片段文件。其中两份各带一份私有的“只替换 `\` 与 `"`”副本，那根本不是 JSON 编码——
//! RFC 8259 还要求转义所有低于 U+0020 的码点——因此那两份会在字符串里原样写出制表符或换行。

use std::fmt::Write as _;

/// Append `value` to `output` as one quoted JSON string literal.
/// 把 `value` 作为一个带引号的 JSON 字符串字面量追加到 `output`。
///
/// # Why the obvious implementation is wrong
/// 为什么直白写法是错的
///
/// `value.replace('\\', "\\\\").replace('"', "\\\"")` is the shape a reader
/// reaches for first, and it is wrong in a way this workspace's own tests cannot
/// see: a tab or a newline inside a name is emitted raw, which makes the record
/// invalid for every conforming reader, while the lenient parser in this crate
/// still accepts it. A round-trip test therefore passes on invalid output. That
/// is exactly what happened to `MirGraph::to_jsonl`, and it is why the pinning
/// test below asserts the absence of raw control characters rather than a round
/// trip.
/// `value.replace('\\', "\\\\").replace('"', "\\\"")` 是读者最先想到的写法，而它的错处
/// 恰是本工作区自己的测试看不见的：名字里的制表符或换行会被原样写出，使记录对任何合规读取器
/// 非法，而本 crate 的宽松解析器仍然接受它。于是往返测试会在非法输出上通过——`MirGraph::to_jsonl`
/// 正是如此，这也是下面的钉子测试断言“不存在原样控制字符”而不是做往返的原因。
///
/// # Boundary
/// 边界
///
/// The escape set is exactly RFC 8259's: the quotation mark, the reverse
/// solidus, and U+0000–U+001F (the short form where JSON defines one, `\u00XX`
/// otherwise, so even a NUL is representable). Nothing else is escaped, so
/// non-ASCII text stays readable in the artifact.
/// 转义集恰好是 RFC 8259 的那一份：引号、反斜杠、以及 U+0000–U+001F（JSON 定义了短形式
/// 的用短形式，其余用 `\u00XX`，因此连 NUL 也能表示）。其余字符一律不转义，非 ASCII 文本
/// 因此在工件里保持可读。
pub fn push_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control < '\u{20}' => {
                // Writing into a `String` cannot fail, and the kernel does not
                // panic on an impossible error.
                // 写入 `String` 不可能失败，内核也不为一个不可能的错误 panic。
                let _ = write!(output, "\\u{:04x}", control as u32);
            }
            other => output.push(other),
        }
    }
    output.push('"');
}

/// Encode `value` as a complete JSON string, quotation marks included.
/// 把 `value` 编码为完整的 JSON 字符串，含两侧引号。
pub fn json_string(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    push_json_string(&mut output, value);
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The pin for the defect that opened this module: a control character must
    /// never reach an artifact raw.
    /// 打开本模块的那个缺陷的钉子：控制字符绝不能原样进入工件。
    #[test]
    fn control_characters_are_escaped() {
        let encoded = json_string("a\tb\nc\u{1}d");
        assert_eq!(encoded, r#""a\tb\nc\u0001d""#);
        assert!(
            !encoded.chars().any(|character| character < '\u{20}'),
            "no raw control character may survive: {encoded:?}"
        );
    }

    /// The structural characters and the empty string keep their exact forms.
    /// 结构字符与空串保持它们的确切形式。
    #[test]
    fn structural_characters_use_the_short_forms() {
        assert_eq!(json_string(r#"say "hi" \ ok"#), r#""say \"hi\" \\ ok""#);
        assert_eq!(json_string(""), r#""""#);
        assert_eq!(json_string("控件"), r#""控件""#);
    }
}
