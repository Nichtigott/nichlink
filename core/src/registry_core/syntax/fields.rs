//! Field-value decoding for parsed registration faces.
//! 已解析注册面的字段取值解码。
//!
//! [`FaceSyntax`] stores each field's raw tokens; this layer turns one field's
//! tokens into the typed value a caller asked for — path, string, boolean, list,
//! requirement pair, optional string, parent — and builds the field map from a
//! face body in the first place.
//! [`FaceSyntax`] 保存每个字段的原始 token；本层把一个字段的 token 变成调用方索取的
//! 有类型值——路径、字符串、布尔、列表、需求对、可选字符串、父级——并负责从注册面
//! 主体构建字段映射。

use std::collections::BTreeMap;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};

use super::tokens::{literal_string, location, only_group, path_to_string, split_top_level};
use super::{
    FaceSyntax, FaceSyntaxError, FieldSyntax, ParentSyntax, SyntaxLocation, compact,
    split_face_fields,
};

impl FaceSyntax {
    /// One field's tokens as compact source text, exactly as written.
    /// 某个字段的 token 渲染成的紧凑源码文本，按原文输出。
    ///
    /// This is the escape hatch for fields the typed readers below do not cover:
    /// it answers `None` only when the field is absent, never because of its shape.
    /// 这是下面那些有类型读取器未覆盖字段的出口：只有字段缺失时才返回 `None`，绝不因形状
    /// 而返回。
    pub fn field(&self, name: &str) -> Option<String> {
        self.fields.get(name).map(|field| compact(&field.tokens))
    }

    /// The `cfg` attribute written on the declaration, if any.
    /// 声明上写下的 `cfg` 属性（若有）。
    pub fn cfg(&self) -> Option<&str> {
        self.cfg.as_deref()
    }

    /// Where one field was written, so a diagnostic can point at it.
    /// 某个字段写在哪，供诊断指向它。
    pub fn field_location(&self, name: &str) -> Option<&SyntaxLocation> {
        self.fields.get(name).map(|field| &field.location)
    }

    /// One field as a Rust path; `None` when it is absent or is not a path.
    /// 某个字段作为 Rust 路径；字段缺失或不是路径时为 `None`。
    pub fn path(&self, name: &str) -> Option<String> {
        let field = self.fields.get(name)?;
        syn::parse2::<syn::Path>(field.tokens.clone())
            .ok()
            .map(|path| path_to_string(&path))
    }

    /// One field as a string literal; `None` when it is absent or is not one.
    /// 某个字段作为字符串字面量；字段缺失或不是字面量时为 `None`。
    pub fn string(&self, name: &str) -> Option<String> {
        let field = self.fields.get(name)?;
        syn::parse2::<syn::LitStr>(field.tokens.clone())
            .ok()
            .map(|literal| literal.value())
    }

    /// One field as a boolean literal; `None` when it is absent or is not one.
    /// 某个字段作为布尔字面量；字段缺失或不是字面量时为 `None`。
    pub fn boolean(&self, name: &str) -> Option<bool> {
        let field = self.fields.get(name)?;
        syn::parse2::<syn::LitBool>(field.tokens.clone())
            .ok()
            .map(|literal| literal.value)
    }

    /// One localized field's text in one language, as written
    /// (`name: { zh: "…", en: "…" }`).
    /// 某个本地化字段在指定语言下的文本，按原文（`name: { zh: "…", en: "…" }`）。
    pub fn localized(&self, name: &str, language: &str) -> Option<String> {
        let field = self.fields.get(name)?;
        let group = only_group(&field.tokens, Delimiter::Brace)?;
        parse_fields(group.stream(), group.span())
            .ok()?
            .get(language)
            .and_then(|field| syn::parse2::<syn::LitStr>(field.tokens.clone()).ok())
            .map(|literal| literal.value())
    }

    /// One bracketed list field as strings, in the written order.
    /// 某个方括号列表字段的字符串，按书写顺序。
    pub fn string_list(&self, name: &str) -> Option<Vec<String>> {
        let field = self.fields.get(name)?;
        let group = only_group(&field.tokens, Delimiter::Bracket)?;
        split_top_level(group.stream())
            .into_iter()
            .map(|tokens| syn::parse2::<syn::LitStr>(tokens).map(|literal| literal.value()))
            .collect::<Result<Vec<_>, _>>()
            .ok()
    }

    /// One bracketed list field as Rust paths, in the written order.
    /// 某个方括号列表字段的 Rust 路径，按书写顺序。
    pub fn path_list(&self, name: &str) -> Option<Vec<String>> {
        let field = self.fields.get(name)?;
        let group = only_group(&field.tokens, Delimiter::Bracket)?;
        split_top_level(group.stream())
            .into_iter()
            .map(|tokens| syn::parse2::<syn::Path>(tokens).map(|path| path_to_string(&path)))
            .collect::<Result<Vec<_>, _>>()
            .ok()
    }

    /// One list field as `capability => provider` pairs.
    /// 某个列表字段的 `capability => provider` 对。
    pub fn requirements(&self, name: &str) -> Option<Vec<(String, String)>> {
        let field = self.fields.get(name)?;
        let group = only_group(&field.tokens, Delimiter::Bracket)?;
        split_top_level(group.stream())
            .into_iter()
            .map(parse_requirement)
            .collect::<Option<Vec<_>>>()
    }

    /// One field as a tri-state string: `Some(Some(text))` for `Some("text")`,
    /// `Some(None)` for `None`, and `None` when the field is absent or shaped
    /// differently.
    /// 某个字段的三态字符串：`Some("text")` 得 `Some(Some(text))`，`None` 得
    /// `Some(None)`，字段缺失或形状不同得 `None`。
    ///
    /// The outer `Option` separates "not written" from "written as `None`", which
    /// an edit must not collapse: one means leave the field alone.
    /// 外层 `Option` 把"没写"与"写成 `None`"分开，编辑时不能合并：前者意味着不要动这个字段。
    pub fn option_string(&self, name: &str) -> Option<Option<String>> {
        let field = self.fields.get(name)?;
        let expression = syn::parse2::<syn::Expr>(field.tokens.clone()).ok()?;
        match expression {
            syn::Expr::Path(path) if path.path.is_ident("None") => Some(None),
            syn::Expr::Call(call) => {
                let syn::Expr::Path(function) = *call.func else {
                    return None;
                };
                if !function.path.is_ident("Some") || call.args.len() != 1 {
                    return None;
                }
                let syn::Expr::Lit(argument) = call.args.first()? else {
                    return None;
                };
                let syn::Lit::Str(value) = &argument.lit else {
                    return None;
                };
                Some(Some(value.value()))
            }
            _ => None,
        }
    }

    /// The declared parent, classified as far as the text allows; `None` when the
    /// field is absent or its expression is not one of the three known shapes.
    /// 声明的父级，按文本能分类到的程度给出；字段缺失或表达式不属于三种已知形状时为
    /// `None`。
    pub fn parent(&self) -> Option<ParentSyntax> {
        let field = self.fields.get("parent")?;
        let expression = syn::parse2::<syn::Expr>(field.tokens.clone()).ok()?;
        match expression {
            syn::Expr::Path(path) => {
                let path = path_to_string(&path.path);
                if path.ends_with("ROOT_NODE_ID") {
                    Some(ParentSyntax::Root)
                } else {
                    path.strip_suffix("::NODE_ID")
                        .map(|module| ParentSyntax::NodePath(module.to_owned()))
                }
            }
            syn::Expr::Call(call) => parse_parent_call(call),
            _ => None,
        }
    }
}

/// Build the field map of one face body, rejecting a malformed `name: value`.
/// 构建一个注册面主体的字段映射，拒绝格式错误的 `name: value`。
pub(super) fn parse_fields(
    tokens: TokenStream,
    fallback_span: Span,
) -> Result<BTreeMap<String, FieldSyntax>, FaceSyntaxError> {
    let mut fields = BTreeMap::new();
    for field in split_face_fields(tokens) {
        let mut tokens = field.into_iter();
        let Some(TokenTree::Ident(name)) = tokens.next() else {
            return Err(super::syntax_error(
                fallback_span,
                "expected a registration field name",
            ));
        };
        let Some(TokenTree::Punct(colon)) = tokens.next() else {
            return Err(super::syntax_error(
                name.span(),
                format!("expected `:` after `{name}`"),
            ));
        };
        if colon.as_char() != ':' {
            return Err(super::syntax_error(
                colon.span(),
                format!("expected `:` after `{name}`"),
            ));
        }
        let value = tokens.collect::<TokenStream>();
        if value.is_empty() {
            return Err(super::syntax_error(
                name.span(),
                format!("field `{name}` has no value"),
            ));
        }
        let field_name = name.to_string();
        if fields
            .insert(
                field_name.clone(),
                FieldSyntax {
                    tokens: value,
                    location: location(name.span()),
                },
            )
            .is_some()
        {
            return Err(super::syntax_error(
                name.span(),
                format!("duplicate field `{field_name}`"),
            ));
        }
    }
    Ok(fields)
}

/// Read one `"capability" => "provider"` requirement pair.
/// 读取一条 `"capability" => "provider"` 需求对。
fn parse_requirement(tokens: TokenStream) -> Option<(String, String)> {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let arrow = tokens.windows(2).position(|pair| {
        matches!(&pair[0], TokenTree::Punct(punct) if punct.as_char() == '=')
            && matches!(&pair[1], TokenTree::Punct(punct) if punct.as_char() == '>')
    })?;
    let left = tokens[..arrow].iter().cloned().collect::<TokenStream>();
    let right = tokens[arrow + 2..].iter().cloned().collect::<TokenStream>();
    let capability = syn::parse2::<syn::LitStr>(left).ok()?.value();
    let provider = syn::parse2::<syn::LitStr>(right).ok()?.value();
    Some((capability, provider))
}

/// Read the `parent:` constructor call forms the authoring layer writes.
/// 读取创作层写下的 `parent:` 构造函数形式。
fn parse_parent_call(call: syn::ExprCall) -> Option<ParentSyntax> {
    let syn::Expr::Path(function) = *call.func else {
        return None;
    };
    let function = path_to_string(&function.path);
    if function.ends_with("root_node_id") && call.args.len() == 1 {
        return Some(ParentSyntax::Root);
    }
    if !function.ends_with("NodeId::from_path") || call.args.len() != 2 {
        return None;
    }
    let mut arguments = call.args.iter();
    let source = literal_string(arguments.next()?)?;
    let kind = literal_string(arguments.next()?)?;
    Some(ParentSyntax::FromPath { source, kind })
}
