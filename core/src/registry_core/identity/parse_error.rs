//! Error type for `NodeId` parsing.
//! `NodeId` 解析的错误类型。

/// Error returned when a 32-digit hexadecimal identity cannot be parsed.
/// 三十二位十六进制身份无法解析时返回的错误。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseNodeIdError;

impl std::fmt::Display for ParseNodeIdError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("node identity must contain exactly 32 hexadecimal digits")
    }
}

impl std::error::Error for ParseNodeIdError {}

#[cfg(test)]
mod tests {
    use super::ParseNodeIdError;
    use std::error::Error;

    #[test]
    fn message_names_the_expectation() {
        let error = ParseNodeIdError;
        assert_eq!(
            error.to_string(),
            "node identity must contain exactly 32 hexadecimal digits"
        );
        assert!(error.source().is_none());
    }
}
