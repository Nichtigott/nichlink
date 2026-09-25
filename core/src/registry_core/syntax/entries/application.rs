//! The `application!` entry grammar.
//! `application!` 入口语法。
//!
//! Parsing the macro token stream keeps comments and string literals out of
//! discovery, so the build step can reject duplicate or malformed declarations
//! before it attempts scope inference.
//! 解析宏 token 流可以排除注释和字符串，构建阶段能在推导作用域前拒绝重复或损坏声明。

use proc_macro2::{TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::Visit;

use super::super::{
    FaceSyntaxError, SyntaxLocation, location, path_to_string, split_top_level, syntax_error,
};

/// Return the host entry paths declared with `application!(entry = ...)`.
/// 返回通过 `application!(entry = ...)` 声明的宿主入口路径。
///
/// Parsing the macro token stream keeps comments and string literals out of
/// discovery. The build step can therefore reject duplicate or malformed
/// declarations before it attempts scope inference.
/// 解析宏 token 流可以排除注释和字符串，构建阶段能在推导作用域前拒绝重复或损坏声明。
pub fn application_entries(source: &str) -> Result<Vec<(String, SyntaxLocation)>, FaceSyntaxError> {
    let file = super::super::nesting::parse_file(source)?;
    let mut entries = Vec::new();
    let mut visitor = ApplicationVisitor {
        entries: &mut entries,
        error: None,
    };
    visitor.visit_file(&file);
    if let Some(error) = visitor.error {
        Err(error)
    } else {
        Ok(entries)
    }
}

struct ApplicationVisitor<'a> {
    entries: &'a mut Vec<(String, SyntaxLocation)>,
    error: Option<FaceSyntaxError>,
}

impl<'ast> Visit<'ast> for ApplicationVisitor<'_> {
    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if self.error.is_some() {
            return;
        }
        let Some(segment) = item.mac.path.segments.last() else {
            return;
        };
        if segment.ident != "application" {
            syn::visit::visit_item_macro(self, item);
            return;
        }
        let tokens = split_top_level(item.mac.tokens.clone());
        if tokens.len() != 1 {
            self.error = Some(syntax_error(
                item.mac.span(),
                "application! expects exactly `entry = <path>`",
            ));
            return;
        }
        let mut tokens = tokens[0].clone().into_iter();
        let Some(TokenTree::Ident(name)) = tokens.next() else {
            self.error = Some(syntax_error(
                item.mac.span(),
                "application! entry must start with `entry`",
            ));
            return;
        };
        if name != "entry"
            || !matches!(tokens.next(), Some(TokenTree::Punct(punct)) if punct.as_char() == '=')
        {
            self.error = Some(syntax_error(
                name.span(),
                "application! expects `entry = <path>`",
            ));
            return;
        }
        let path = tokens.collect::<TokenStream>();
        let Ok(path) = syn::parse2::<syn::Path>(path) else {
            self.error = Some(syntax_error(
                name.span(),
                "application! entry must be a Rust path",
            ));
            return;
        };
        self.entries
            .push((path_to_string(&path), location(item.mac.span())));
    }
}

#[cfg(test)]
mod tests {
    use super::application_entries;

    #[test]
    fn application_entry_parser_ignores_comments_and_strings() {
        let source = r#"
// application!(entry = crate::wrong)
const TEXT: &str = "application!(entry = crate::also_wrong)";
nichlink::application!(entry = crate::app::run);
"#;
        let entries = application_entries(source).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "crate::app::run");
        assert_eq!(entries[0].1.line, 4);
    }

    #[test]
    fn application_entry_parser_rejects_malformed_declarations() {
        let error = application_entries("application!(crate::main)").unwrap_err();
        assert!(!error.message.is_empty());
    }

    #[test]
    fn application_entry_parser_keeps_duplicates_visible_to_the_build_policy() {
        let source = "application!(entry = crate::main); application!(entry = crate::run);";
        let entries = application_entries(source).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "crate::main");
        assert_eq!(entries[1].0, "crate::run");
    }
}
