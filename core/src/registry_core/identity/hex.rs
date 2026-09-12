//! Hexadecimal decoding helpers shared by identity parsing and plugin
//! fingerprints.
//! 身份解析和插件指纹共用的十六进制解码辅助。

/// Decode one hexadecimal digit (0-9, a-f, A-F).
/// 解码一个十六进制数字符（0-9、a-f、A-F）。
pub const fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Decode a whole even-length hexadecimal string.
/// 解码完整偶数长度的十六进制字符串。
///
/// Returns `None` for odd lengths or any non-hex digit.
/// 长度为奇数或含有非十六进制字符时返回 `None`。
pub fn hex_decode(value: &str) -> Option<Vec<u8>> {
    if !value.len().is_multiple_of(2) {
        return None;
    }
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len() / 2);
    let chunk = bytes.as_chunks::<2>().0;
    for pair in chunk {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        decoded.push((high << 4) | low);
    }
    Some(decoded)
}

#[cfg(test)]
mod tests {
    use super::{hex_decode, hex_nibble};

    #[test]
    fn nibble_decodes_all_digit_cases() {
        assert_eq!(hex_nibble(b'0'), Some(0));
        assert_eq!(hex_nibble(b'9'), Some(9));
        assert_eq!(hex_nibble(b'a'), Some(10));
        assert_eq!(hex_nibble(b'f'), Some(15));
        assert_eq!(hex_nibble(b'A'), Some(10));
        assert_eq!(hex_nibble(b'F'), Some(15));
        assert_eq!(hex_nibble(b'g'), None);
    }

    #[test]
    fn hex_decode_round_trips_even_lengths() {
        assert_eq!(hex_decode("ba7816bf"), Some(vec![0xba, 0x78, 0x16, 0xbf]));
        assert_eq!(hex_decode("abc"), None);
        assert_eq!(hex_decode("zz"), None);
        assert_eq!(hex_decode(""), Some(vec![]));
    }
}
