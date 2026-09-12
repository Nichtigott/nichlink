//! Parser shared by build-time checks and live authoring.
//! 构建期检查与实时编辑共用的注册面解析器。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::Visit;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxLocation {
    pub line: usize,
    pub column: usize,
}

#[derive(Clone, Debug)]
struct FieldSyntax {
    tokens: TokenStream,
    location: SyntaxLocation,
}

/// Parsed registration declaration, independent of the macro that consumes it.
/// 已解析的注册声明，与消费它的宏实现无关。
#[derive(Clone, Debug)]
pub struct FaceSyntax {
    pub macro_name: String,
    pub location: SyntaxLocation,
    /// Exclusive end of the macro invocation in the source file.
    /// 宏调用在源码中的排他结束位置。
    pub end: SyntaxLocation,
    fields: BTreeMap<String, FieldSyntax>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParentSyntax {
    Root,
    FromPath { source: String, kind: String },
    NodePath(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceSyntaxError {
    pub message: String,
    pub location: Option<SyntaxLocation>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceReferences {
    pub paths: BTreeSet<String>,
    pub conservative: bool,
}

impl fmt::Display for FaceSyntaxError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(location) = &self.location {
            write!(
                formatter,
                "{}:{}: {}",
                location.line, location.column, self.message
            )
        } else {
            formatter.write_str(&self.message)
        }
    }
}

impl std::error::Error for FaceSyntaxError {}

impl FaceSyntax {
    pub fn field(&self, name: &str) -> Option<String> {
        self.fields.get(name).map(|field| compact(&field.tokens))
    }

    pub fn field_location(&self, name: &str) -> Option<&SyntaxLocation> {
        self.fields.get(name).map(|field| &field.location)
    }

    pub fn path(&self, name: &str) -> Option<String> {
        let field = self.fields.get(name)?;
        syn::parse2::<syn::Path>(field.tokens.clone())
            .ok()
            .map(|path| path_to_string(&path))
    }

    pub fn string(&self, name: &str) -> Option<String> {
        let field = self.fields.get(name)?;
        syn::parse2::<syn::LitStr>(field.tokens.clone())
            .ok()
            .map(|literal| literal.value())
    }

    pub fn boolean(&self, name: &str) -> Option<bool> {
        let field = self.fields.get(name)?;
        syn::parse2::<syn::LitBool>(field.tokens.clone())
            .ok()
            .map(|literal| literal.value)
    }

    pub fn localized(&self, name: &str, language: &str) -> Option<String> {
        let field = self.fields.get(name)?;
        let group = only_group(&field.tokens, Delimiter::Brace)?;
        parse_fields(group.stream(), group.span())
            .ok()?
            .get(language)
            .and_then(|field| syn::parse2::<syn::LitStr>(field.tokens.clone()).ok())
            .map(|literal| literal.value())
    }

    pub fn string_list(&self, name: &str) -> Option<Vec<String>> {
        let field = self.fields.get(name)?;
        let group = only_group(&field.tokens, Delimiter::Bracket)?;
        split_top_level(group.stream())
            .into_iter()
            .map(|tokens| syn::parse2::<syn::LitStr>(tokens).map(|literal| literal.value()))
            .collect::<Result<Vec<_>, _>>()
            .ok()
    }

    pub fn path_list(&self, name: &str) -> Option<Vec<String>> {
        let field = self.fields.get(name)?;
        let group = only_group(&field.tokens, Delimiter::Bracket)?;
        split_top_level(group.stream())
            .into_iter()
            .map(|tokens| syn::parse2::<syn::Path>(tokens).map(|path| path_to_string(&path)))
            .collect::<Result<Vec<_>, _>>()
            .ok()
    }

    pub fn requirements(&self, name: &str) -> Option<Vec<(String, String)>> {
        let field = self.fields.get(name)?;
        let group = only_group(&field.tokens, Delimiter::Bracket)?;
        split_top_level(group.stream())
            .into_iter()
            .map(parse_requirement)
            .collect::<Option<Vec<_>>>()
    }

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

/// Parse every NichLink face declaration in one Rust source file.
/// 解析一个 Rust 源文件中的全部 NichLink 注册面声明。
pub fn parse_faces(source: &str) -> Result<Vec<FaceSyntax>, FaceSyntaxError> {
    let file =
        syn::parse_file(source).map_err(|error| syntax_error(error.span(), error.to_string()))?;
    let mut visitor = FaceVisitor {
        faces: Vec::new(),
        exports_consts: BTreeMap::new(),
        error: None,
    };
    visitor.visit_file(&file);
    if let Some(error) = visitor.error {
        Err(error)
    } else {
        let mut faces = visitor.faces;
        // An inherent `impl Kind { pub const EXPORTS: ... = &[...]; }` supplies
        // the export list when the attribute omits `exports = [...]`.
        // 属性未写 `exports = [...]` 时，由固有
        // `impl Kind { pub const EXPORTS: ... = &[...]; }` 提供 exports 列表。
        for face in &mut faces {
            if !face.fields.contains_key("exports") {
                let kind = face.fields.get("kind").map(|field| compact(&field.tokens));
                if let Some(exports) = kind.and_then(|kind| visitor.exports_consts.get(&kind)) {
                    face.fields.insert("exports".to_owned(), exports.clone());
                }
            }
        }
        Ok(faces)
    }
}

/// Parse exactly one face, returning `None` when the file has no declaration.
/// 解析唯一注册面；文件没有声明时返回 `None`。
pub fn parse_face(source: &str) -> Result<Option<FaceSyntax>, FaceSyntaxError> {
    let mut faces = parse_faces(source)?;
    if faces.len() > 1 {
        return Err(FaceSyntaxError {
            message: "expected one registration face in this file".to_owned(),
            location: faces.get(1).map(|face| face.location.clone()),
        });
    }
    Ok(faces.pop())
}

/// Replace only the registration macro, preserving the surrounding Rust code.
/// 只替换注册宏，保留同一文件里的其余 Rust 实现。
pub fn replace_face_macro(source: &str, replacement: &str) -> Result<String, FaceSyntaxError> {
    let current = parse_face(source)?.ok_or_else(|| FaceSyntaxError {
        message: "source has no registration face".to_owned(),
        location: None,
    })?;
    let next = parse_face(replacement)?.ok_or_else(|| FaceSyntaxError {
        message: "replacement has no registration face".to_owned(),
        location: None,
    })?;
    let current_start = source_offset(source, &current.location)?;
    let current_end = source_offset(source, &current.end)?;
    let replacement_start = source_offset(replacement, &next.location)?;
    let replacement_end = source_offset(replacement, &next.end)?;
    let mut output =
        String::with_capacity(source.len() + replacement_end.saturating_sub(replacement_start));
    output.push_str(&source[..current_start]);
    output.push_str(&replacement[replacement_start..replacement_end]);
    output.push_str(&source[current_end..]);
    Ok(output)
}

fn source_offset(source: &str, location: &SyntaxLocation) -> Result<usize, FaceSyntaxError> {
    let line_start = if location.line <= 1 {
        0
    } else {
        source
            .match_indices('\n')
            .nth(location.line - 2)
            .map(|(index, _)| index + 1)
            .ok_or_else(|| FaceSyntaxError {
                message: "macro span points outside source".to_owned(),
                location: Some(location.clone()),
            })?
    };
    let offset = line_start + location.column.saturating_sub(1);
    source
        .is_char_boundary(offset)
        .then_some(offset)
        .ok_or_else(|| FaceSyntaxError {
            message: "macro span is not on a UTF-8 boundary".to_owned(),
            location: Some(location.clone()),
        })
}

/// Collect paths used by executable expressions, excluding imports, comments,
/// strings, and registration-macro metadata.
/// 收集可执行表达式使用的路径，排除导入、注释、字符串和注册宏元数据。
pub fn source_references(source: &str) -> Result<SourceReferences, FaceSyntaxError> {
    let file =
        syn::parse_file(source).map_err(|error| syntax_error(error.span(), error.to_string()))?;
    let mut visitor = ReferenceVisitor::default();
    visitor.visit_file(&file);
    Ok(visitor.references)
}

struct FaceVisitor {
    faces: Vec<FaceSyntax>,
    /// Inherent `EXPORTS` consts collected per kind name, keyed by kind.
    /// 按 kind 收集的固有 `EXPORTS` 常量。
    exports_consts: BTreeMap<String, FieldSyntax>,
    error: Option<FaceSyntaxError>,
}

#[derive(Default)]
struct ReferenceVisitor {
    references: SourceReferences,
}

impl<'ast> Visit<'ast> for ReferenceVisitor {
    fn visit_expr_path(&mut self, expression: &'ast syn::ExprPath) {
        self.references
            .paths
            .insert(path_to_string(&expression.path));
        syn::visit::visit_expr_path(self, expression);
    }

    fn visit_expr_call(&mut self, expression: &'ast syn::ExprCall) {
        if !matches!(expression.func.as_ref(), syn::Expr::Path(_)) {
            self.references.conservative = true;
        }
        syn::visit::visit_expr_call(self, expression);
    }

    fn visit_type_trait_object(&mut self, object: &'ast syn::TypeTraitObject) {
        self.references.conservative = true;
        syn::visit::visit_type_trait_object(self, object);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        self.references.conservative = true;
        syn::visit::visit_item_extern_crate(self, item);
    }

    fn visit_macro(&mut self, item: &'ast syn::Macro) {
        let name = item
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string());
        if matches!(
            name.as_deref(),
            Some("include" | "include_str" | "include_bytes" | "macro_rules")
        ) {
            self.references.conservative = true;
        }
    }
}

impl<'ast> Visit<'ast> for FaceVisitor {
    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        if self.error.is_some() {
            return;
        }
        let Some(attr) = item.attrs.iter().find(|attr| is_object_attribute(attr)) else {
            return;
        };
        match struct_face_fields(item, attr) {
            Ok(fields) => self.faces.push(FaceSyntax {
                macro_name: "object".to_owned(),
                location: location(attr.span()),
                end: end_location(item.span()),
                fields,
            }),
            Err(error) => self.error = Some(error),
        }
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if self.error.is_some() || item.trait_.is_some() {
            return;
        }
        let syn::Type::Path(self_ty) = &*item.self_ty else {
            return;
        };
        let Some(kind) = self_ty
            .path
            .segments
            .last()
            .map(|segment| segment.ident.to_string())
        else {
            return;
        };
        for impl_item in &item.items {
            let syn::ImplItem::Const(constant) = impl_item else {
                continue;
            };
            if constant.ident != "EXPORTS" {
                continue;
            }
            if let Some(literals) = exports_literal_tokens(&constant.expr) {
                let mut tokens = TokenStream::new();
                for (index, literal) in literals.iter().enumerate() {
                    if index > 0 {
                        tokens.extend([TokenTree::Punct(proc_macro2::Punct::new(
                            ',',
                            proc_macro2::Spacing::Alone,
                        ))]);
                    }
                    tokens.extend(literal.clone());
                }
                self.exports_consts.insert(
                    kind.clone(),
                    FieldSyntax {
                        tokens: TokenStream::from(TokenTree::Group(proc_macro2::Group::new(
                            Delimiter::Bracket,
                            tokens,
                        ))),
                        location: location(constant.ident.span()),
                    },
                );
            }
        }
    }
}

fn is_object_attribute(attr: &syn::Attribute) -> bool {
    attr.path()
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "object")
}

/// Build the `fields` map for a `#[nichlink::object]` struct. Attribute
/// arguments are stored under their key; `name(...)`/`summary(...)` groups are
/// rewritten into the `{ zh: ..., en: ... }` shape and `requires(...)` /
/// `provides(...)` into `[ ... ]` so the existing accessors keep working.
/// 为 `#[nichlink::object]` 结构体构造 `fields` 映射。属性参数按键存储；
/// `name(...)`/`summary(...)` 组改写成 `{ zh: ..., en: ...}` 形状，
/// `requires(...)`/`provides(...)` 改写成 `[ ... ]`，现有取值函数因此无需改动。
fn struct_face_fields(
    item: &syn::ItemStruct,
    attr: &syn::Attribute,
) -> Result<std::collections::BTreeMap<String, FieldSyntax>, FaceSyntaxError> {
    let mut fields = std::collections::BTreeMap::new();
    insert_field(
        &mut fields,
        "kind",
        item.ident.to_token_stream(),
        item.ident.span(),
    )?;
    match &item.fields {
        syn::Fields::Named(named) => {
            for field in &named.named {
                let Some(name) = &field.ident else { continue };
                match name.to_string().as_str() {
                    "preset" | "parts" => insert_field(
                        &mut fields,
                        &name.to_string(),
                        field.ty.to_token_stream(),
                        name.span(),
                    )?,
                    other => {
                        return Err(syntax_error(
                            name.span(),
                            format!(
                                "unknown `object` field `{other}`; a face struct only carries `preset` and `parts` types"
                            ),
                        ));
                    }
                }
            }
        }
        syn::Fields::Unit => {}
        syn::Fields::Unnamed(_) => {
            return Err(syntax_error(
                item.span(),
                "`#[nichlink::object]` expects a plain struct (no tuple fields)",
            ));
        }
    }

    let args = match &attr.meta {
        syn::Meta::Path(_) => TokenStream::new(),
        syn::Meta::List(list) => list.tokens.clone(),
        _ => {
            return Err(syntax_error(
                attr.span(),
                "`#[nichlink::object]` expects `object` or `object(...)`",
            ));
        }
    };
    for chunk in split_top_level(args) {
        let mut trees = chunk.into_iter();
        let Some(TokenTree::Ident(name)) = trees.next() else {
            return Err(syntax_error(
                attr.span(),
                "expected `name = value` or `name(...)` in the `object` attribute",
            ));
        };
        let key = name.to_string();
        match trees.next() {
            Some(TokenTree::Punct(punct)) if punct.as_char() == '=' => {
                let value: TokenStream = trees.collect();
                if value.is_empty() {
                    return Err(syntax_error(name.span(), format!("`{key}` has no value")));
                }
                insert_field(&mut fields, &key, value, name.span())?;
            }
            Some(TokenTree::Group(group)) if group.delimiter() == Delimiter::Parenthesis => {
                let converted = convert_group(&key, group.stream(), name.span())?;
                insert_field(&mut fields, &key, converted, name.span())?;
            }
            _ => {
                return Err(syntax_error(
                    name.span(),
                    format!("expected `=` or `(...)` after `{key}`"),
                ));
            }
        }
    }
    Ok(fields)
}

fn convert_group(
    key: &str,
    tokens: TokenStream,
    span: Span,
) -> Result<TokenStream, FaceSyntaxError> {
    match key {
        "name" | "summary" => {
            // `name(zh = "...", en = "...")` → `{ zh: "...", en: "..." }`
            let mut body = TokenStream::new();
            for (index, chunk) in split_top_level(tokens).into_iter().enumerate() {
                if index > 0 {
                    body.extend([TokenTree::Punct(proc_macro2::Punct::new(
                        ',',
                        proc_macro2::Spacing::Alone,
                    ))]);
                }
                let mut trees = chunk.into_iter();
                let Some(TokenTree::Ident(lang)) = trees.next() else {
                    return Err(syntax_error(
                        span,
                        format!("{key}(...) expects `zh = \"...\"` / `en = \"...\"`"),
                    ));
                };
                let Some(TokenTree::Punct(punct)) = trees.next() else {
                    return Err(syntax_error(
                        lang.span(),
                        format!("expected `=` after `{lang}`"),
                    ));
                };
                if punct.as_char() != '=' {
                    return Err(syntax_error(
                        punct.span(),
                        format!("expected `=` after `{lang}`"),
                    ));
                }
                let value: TokenStream = trees.collect();
                if value.is_empty() {
                    return Err(syntax_error(lang.span(), format!("`{lang}` has no value")));
                }
                body.extend([TokenTree::Ident(lang)]);
                body.extend([TokenTree::Punct(proc_macro2::Punct::new(
                    ':',
                    proc_macro2::Spacing::Alone,
                ))]);
                body.extend(value);
            }
            Ok(TokenStream::from(TokenTree::Group(
                proc_macro2::Group::new(Delimiter::Brace, body),
            )))
        }
        "requires" | "provides" => Ok(TokenStream::from(TokenTree::Group(
            proc_macro2::Group::new(Delimiter::Bracket, tokens),
        ))),
        _ => Err(syntax_error(
            span,
            format!(
                "unknown `object` group `{key}(...)`; expected name(...), summary(...), requires(...), provides(...)"
            ),
        )),
    }
}

fn insert_field(
    fields: &mut std::collections::BTreeMap<String, FieldSyntax>,
    key: &str,
    tokens: TokenStream,
    span: Span,
) -> Result<(), FaceSyntaxError> {
    if fields
        .insert(
            key.to_owned(),
            FieldSyntax {
                tokens,
                location: location(span),
            },
        )
        .is_some()
    {
        return Err(syntax_error(span, format!("duplicate field `{key}`")));
    }
    Ok(())
}

/// Extract the string literals from `&["a", "b"]` (or `["a", "b"]`) EXPORTS
/// initializers; anything else (const references, computed values) returns
/// `None` and leaves the export list unresolved.
/// 从 `&["a", "b"]`（或 `["a", "b"]`）形式的 EXPORTS 初始化表达式提取字符串
/// 字面量；其它形式（常量引用、计算值）返回 `None`，exports 保持未解析。
fn exports_literal_tokens(expr: &syn::Expr) -> Option<Vec<TokenStream>> {
    let expr = match expr {
        syn::Expr::Reference(reference) => &*reference.expr,
        other => other,
    };
    let syn::Expr::Array(array) = expr else {
        return None;
    };
    array
        .elems
        .iter()
        .map(|elem| match elem {
            syn::Expr::Lit(literal) if matches!(literal.lit, syn::Lit::Str(_)) => {
                Some(literal.to_token_stream())
            }
            _ => None,
        })
        .collect()
}

fn parse_fields(
    tokens: TokenStream,
    fallback_span: Span,
) -> Result<BTreeMap<String, FieldSyntax>, FaceSyntaxError> {
    let mut fields = BTreeMap::new();
    for field in split_top_level(tokens) {
        let mut tokens = field.into_iter();
        let Some(TokenTree::Ident(name)) = tokens.next() else {
            return Err(syntax_error(
                fallback_span,
                "expected a registration field name",
            ));
        };
        let Some(TokenTree::Punct(colon)) = tokens.next() else {
            return Err(syntax_error(
                name.span(),
                format!("expected `:` after `{name}`"),
            ));
        };
        if colon.as_char() != ':' {
            return Err(syntax_error(
                colon.span(),
                format!("expected `:` after `{name}`"),
            ));
        }
        let value = tokens.collect::<TokenStream>();
        if value.is_empty() {
            return Err(syntax_error(
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
            return Err(syntax_error(
                name.span(),
                format!("duplicate field `{field_name}`"),
            ));
        }
    }
    Ok(fields)
}

#[doc(hidden)]
pub fn split_top_level(tokens: TokenStream) -> Vec<TokenStream> {
    let mut items = Vec::new();
    let mut current = TokenStream::new();
    for token in tokens {
        if matches!(&token, TokenTree::Punct(punct) if punct.as_char() == ',') {
            if !current.is_empty() {
                items.push(current);
                current = TokenStream::new();
            }
        } else {
            current.extend([token]);
        }
    }
    if !current.is_empty() {
        items.push(current);
    }
    items
}

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

fn literal_string(expression: &syn::Expr) -> Option<String> {
    let syn::Expr::Lit(expression) = expression else {
        return None;
    };
    let syn::Lit::Str(value) = &expression.lit else {
        return None;
    };
    Some(value.value())
}

fn only_group(tokens: &TokenStream, delimiter: Delimiter) -> Option<proc_macro2::Group> {
    let mut tokens = tokens.clone().into_iter();
    let TokenTree::Group(group) = tokens.next()? else {
        return None;
    };
    (group.delimiter() == delimiter && tokens.next().is_none()).then_some(group)
}

#[doc(hidden)]
pub fn path_to_string(path: &syn::Path) -> String {
    let mut output = String::new();
    if path.leading_colon.is_some() {
        output.push_str("::");
    }
    for (index, segment) in path.segments.iter().enumerate() {
        if index > 0 {
            output.push_str("::");
        }
        output.push_str(&segment.ident.to_string());
    }
    output
}

fn compact(tokens: &TokenStream) -> String {
    tokens
        .to_string()
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace(") ", ")")
}

#[doc(hidden)]
pub fn location(span: Span) -> SyntaxLocation {
    let start = span.start();
    SyntaxLocation {
        line: start.line,
        column: start.column + 1,
    }
}

fn end_location(span: Span) -> SyntaxLocation {
    let end = span.end();
    SyntaxLocation {
        line: end.line,
        column: end.column + 1,
    }
}

#[doc(hidden)]
pub fn syntax_error(span: Span, message: impl Into<String>) -> FaceSyntaxError {
    FaceSyntaxError {
        message: message.into(),
        location: Some(location(span)),
    }
}

#[cfg(test)]
mod struct_form_tests {
    use super::{ParentSyntax, parse_face, replace_face_macro};

    #[test]
    fn explicit_root_face_with_full_rule_metadata_parses() {
        let source = "use nichlink_run_method as nichlink;\n\n#[nichlink::object(\n    needs_registry = true,\n    registry_name = control,\n    parent = crate::ROOT_NODE_ID,\n    registry_rule_path = \"src/control/registry_rule/registry_rule.rs\",\n    registry_rule = crate::control::registry_rule::REGISTRATION_RULE,\n)]\npub struct ControlRegistry;\n";
        let face = parse_face(source).unwrap().unwrap();
        assert_eq!(face.path("kind").as_deref(), Some("ControlRegistry"));
    }

    #[test]
    fn parses_object_struct_with_attribute_metadata() {
        let source = r#"
#[nichlink::object(
    parent = crate::control::NODE_ID,
    needs_registry = true,
    name(zh = "按钮", en = "Button"),
    summary(zh = "摘要", en = "Summary"),
    requires("layout.viewport" => "ControlRegistry"),
    provides("control.render"),
)]
pub struct Button {
    preset: ControlPreset,
    parts: ButtonParts,
}
"#;
        let face = parse_face(source).unwrap().unwrap();

        assert_eq!(face.macro_name, "object");
        assert_eq!(face.path("kind").as_deref(), Some("Button"));
        assert_eq!(face.path("preset").as_deref(), Some("ControlPreset"));
        assert_eq!(face.path("parts").as_deref(), Some("ButtonParts"));
        assert_eq!(face.boolean("needs_registry"), Some(true));
        assert_eq!(face.localized("name", "zh").as_deref(), Some("按钮"));
        assert_eq!(face.localized("summary", "en").as_deref(), Some("Summary"));
        assert_eq!(
            face.requirements("requires").unwrap(),
            [("layout.viewport".to_owned(), "ControlRegistry".to_owned())]
        );
        assert_eq!(face.string_list("provides").unwrap(), ["control.render"]);
        assert_eq!(
            face.parent(),
            Some(ParentSyntax::NodePath("crate::control".to_owned()))
        );
    }

    #[test]
    fn impl_exports_const_supplies_the_export_list() {
        let source = r#"
#[nichlink::object]
pub struct Button {
    preset: ControlPreset,
    parts: ButtonParts,
}

impl Button {
    pub const EXPORTS: &'static [&'static str] = &["control.preview", "control.render"];
}
"#;
        let face = parse_face(source).unwrap().unwrap();

        assert_eq!(
            face.string_list("exports").unwrap(),
            ["control.preview", "control.render"]
        );
        let exports = face.field("exports").expect("exports field");
        assert!(exports.starts_with('['), "{exports}");
        assert!(exports.contains("\"control.preview\""), "{exports}");
        assert!(exports.contains("\"control.render\""), "{exports}");
    }

    #[test]
    fn attribute_exports_win_over_the_impl_const() {
        let source = r#"
#[nichlink::object(exports = ["control.attr"])]
pub struct Button {}

impl Button {
    pub const EXPORTS: &'static [&'static str] = &["control.impl"];
}
"#;
        let face = parse_face(source).unwrap().unwrap();

        assert_eq!(face.string_list("exports").unwrap(), ["control.attr"]);
    }

    #[test]
    fn unknown_struct_field_is_rejected() {
        let source = "#[nichlink::object] pub struct Button { wat: Thing }";
        let error = parse_face(source).unwrap_err();
        assert!(error.message.contains("unknown `object` field `wat`"));
    }

    #[test]
    fn unknown_group_is_rejected() {
        let source = "#[nichlink::object(frobnicate(a = \"b\"))] pub struct Button {}";
        let error = parse_face(source).unwrap_err();
        assert!(
            error
                .message
                .contains("unknown `object` group `frobnicate(...)`"),
            "{}",
            error.message
        );
    }

    #[test]
    fn replacing_a_struct_face_preserves_its_impl_block() {
        let source = r#"#[nichlink::object]
pub struct Button { preset: P, parts: Q }
impl Button { pub const EXPORTS: &'static [&'static str] = &["a"]; }
#[test] fn works() {}
"#;
        let replacement = r#"#[nichlink::object]
pub struct Button { preset: P2, parts: Q2 }"#;

        let copied = replace_face_macro(source, replacement).unwrap();

        assert!(copied.contains("preset: P2"));
        assert!(copied.contains("pub const EXPORTS"));
        assert!(copied.contains("#[test] fn works()"));
        assert!(!copied.contains("preset: P,"));
    }
}

#[cfg(test)]
mod tests {
    use super::{ParentSyntax, parse_face, replace_face_macro, source_references};

    #[test]
    fn parses_multiline_registration_tokens_and_locations() {
        let source = r#"
#[nichlink::object(
    name(zh = "按钮", en = "Button"),
    requires(
        "layout.viewport" => "ControlRegistry",
        "draw.basic" => "BasicDrawing",
    ),
    parent = crate::NodeId::from_path("control/control.rs", "ControlRegistry"),
    getting_from_other_registry = Some("engine"),
)]
pub struct Button;
"#;
        let face = parse_face(source).unwrap().unwrap();

        assert_eq!(face.path("kind").as_deref(), Some("Button"));
        assert_eq!(face.localized("name", "zh").as_deref(), Some("按钮"));
        assert_eq!(face.field_location("requires").unwrap().line, 4);
        assert_eq!(
            face.requirements("requires").unwrap(),
            [
                ("layout.viewport".to_owned(), "ControlRegistry".to_owned()),
                ("draw.basic".to_owned(), "BasicDrawing".to_owned())
            ]
        );
        assert_eq!(
            face.parent(),
            Some(ParentSyntax::FromPath {
                source: "control/control.rs".to_owned(),
                kind: "ControlRegistry".to_owned(),
            })
        );
        assert_eq!(
            face.option_string("getting_from_other_registry"),
            Some(Some("engine".to_owned()))
        );
    }

    #[test]
    fn parses_namespaced_package_root_parent() {
        let source = r#"
#[nichlink::object(parent = crate::root_node_id(env!("CARGO_PKG_NAME")))]
pub struct Workspace;
"#;
        let face = parse_face(source).unwrap().unwrap();

        assert_eq!(face.parent(), Some(ParentSyntax::Root));
    }

    #[test]
    fn rejects_duplicate_fields() {
        let source =
            r#"#[nichlink::object(stable_name = "a", stable_name = "b")] pub struct Button;"#;
        let error = parse_face(source).unwrap_err();
        assert!(error.message.contains("duplicate field `stable_name`"));
    }

    #[test]
    fn replacing_a_face_preserves_its_rust_implementation() {
        let source = r#"impl Button { pub fn paint(&self) -> u32 { 7 } }
#[nichlink::object(registry_name = button)]
pub struct Button;
#[test] fn paints() { assert_eq!(Button.paint(), 7); }
"#;
        let replacement = r#"#[nichlink::object(registry_name = button_graft)]
pub struct Button;"#;

        let copied = replace_face_macro(source, replacement).unwrap();

        assert!(copied.contains("pub fn paint(&self) -> u32 { 7 }"));
        assert!(copied.contains("registry_name = button_graft"));
        assert!(!copied.contains("registry_name = button,"));
        assert!(copied.contains("#[test] fn paints()"));
    }

    #[test]
    fn source_references_ignore_imports_strings_comments_and_registration_data() {
        let source = r#"
use crate::unused::Thing;
fn run() {
    crate::control::object::button::dispatch_action("unused::fake()", true);
    // crate::comment::fake();
    #[nichlink::object(parent = crate::hidden::NODE_ID)] pub struct Fake;
}
"#;
        let references = source_references(source).unwrap();
        assert!(
            references
                .paths
                .contains("crate::control::object::button::dispatch_action")
        );
        assert!(!references.paths.iter().any(|path| path.contains("unused")));
        assert!(!references.paths.iter().any(|path| path.contains("hidden")));
        assert!(!references.conservative);
    }
}

// ---------------------------------------------------------------------------
// Application and graft declarations. These extend the face parser with the
// two host-entry grammars; both build_method and run_method consume them.
// 应用与嫁接声明。在注册面解析器之上补充两个宿主入口语法；
// build_method 与 run_method 共同消费。

/// A host-declared external graft cut discovered before code generation.
/// 在代码生成前从宿主入口发现的一条外部 graft 切口。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftSyntax {
    pub cut: String,
    pub graft: String,
    pub full: bool,
    pub location: SyntaxLocation,
}

/// Return the host entry paths declared with `application!(entry = ...)`.
/// 返回通过 `application!(entry = ...)` 声明的宿主入口路径。
///
/// Parsing the macro token stream keeps comments and string literals out of
/// discovery. The build step can therefore reject duplicate or malformed
/// declarations before it attempts scope inference.
/// 解析宏 token 流可以排除注释和字符串，构建阶段能在推导作用域前拒绝重复或损坏声明。
pub fn application_entries(source: &str) -> Result<Vec<(String, SyntaxLocation)>, FaceSyntaxError> {
    let file =
        syn::parse_file(source).map_err(|error| syntax_error(error.span(), error.to_string()))?;
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

/// Parse static and dynamic graft declarations without executing them.
/// 解析静态和动态 graft 声明，不执行宏。
pub fn graft_entries(source: &str) -> Result<Vec<GraftSyntax>, FaceSyntaxError> {
    let file =
        syn::parse_file(source).map_err(|error| syntax_error(error.span(), error.to_string()))?;
    let mut entries = Vec::new();
    struct Visitor<'a> {
        entries: &'a mut Vec<GraftSyntax>,
        error: Option<FaceSyntaxError>,
    }
    impl<'ast> Visit<'ast> for Visitor<'_> {
        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            let Some(name) = mac
                .path
                .segments
                .last()
                .map(|segment| segment.ident.to_string())
            else {
                return;
            };
            if !matches!(name.as_str(), "graft_plan" | "static_graft_plan") {
                return;
            }
            let tokens = mac.tokens.clone().into_iter().collect::<Vec<_>>();
            let mut index = 0;
            while index < tokens.len() {
                if !matches!(&tokens[index], TokenTree::Ident(value) if value == "cut") {
                    index += 1;
                    continue;
                }
                index += 1;
                let path_token = tokens.get(index).cloned();
                index += 1;
                let (path_tokens, path_span, path_text) = match path_token {
                    Some(TokenTree::Group(group)) => (
                        group.stream().into_iter().collect::<Vec<_>>(),
                        group.span(),
                        None,
                    ),
                    Some(TokenTree::Literal(literal)) => {
                        let text = syn::parse_str::<syn::LitStr>(&literal.to_string())
                            .map(|value| value.value())
                            .map_err(|_| {
                                syntax_error(literal.span(), "graft cut path must be a string")
                            });
                        let Ok(text) = text else {
                            self.error = text.err();
                            return;
                        };
                        (Vec::new(), literal.span(), Some(text))
                    }
                    _ => {
                        self.error = Some(syntax_error(
                            mac.span(),
                            "graft cut expects `[path]` or a path string",
                        ));
                        return;
                    }
                };
                let (cut, end) = if path_tokens.len() == 3
                    && matches!(&path_tokens[1], TokenTree::Ident(value) if value == "to")
                {
                    let start = syn::parse2::<syn::LitStr>(path_tokens[0].clone().into())
                        .map_err(|_| syntax_error(path_span, "graft range start must be a string"));
                    let finish = syn::parse2::<syn::LitStr>(path_tokens[2].clone().into())
                        .map_err(|_| syntax_error(path_span, "graft range end must be a string"));
                    match (start, finish) {
                        (Ok(start), Ok(finish)) => (start.value(), Some(finish.value())),
                        (Err(error), _) | (_, Err(error)) => {
                            self.error = Some(error);
                            return;
                        }
                    }
                } else {
                    (
                        path_text.unwrap_or_else(|| {
                            syn::parse2::<syn::LitStr>(path_tokens.clone().into_iter().collect())
                                .map(|value| value.value())
                                .unwrap_or_else(|_| {
                                    path_tokens
                                        .iter()
                                        .map(ToString::to_string)
                                        .collect::<String>()
                                })
                        }),
                        None,
                    )
                };
                if cut.is_empty() {
                    self.error = Some(syntax_error(path_span, "graft cut path is empty"));
                    return;
                }
                let full =
                    matches!(tokens.get(index), Some(TokenTree::Ident(value)) if value == "full");
                if full {
                    index += 1;
                }
                if !matches!(tokens.get(index), Some(TokenTree::Ident(value)) if value == "graft") {
                    self.error = Some(syntax_error(
                        mac.span(),
                        "graft cut expects `graft <implementation>`",
                    ));
                    return;
                }
                index += 1;
                let Some(TokenTree::Literal(value)) = tokens.get(index) else {
                    self.error = Some(syntax_error(
                        mac.span(),
                        "graft implementation must be a string literal",
                    ));
                    return;
                };
                let Ok(graft) = syn::parse_str::<syn::LitStr>(&value.to_string()) else {
                    self.error = Some(syntax_error(
                        value.span(),
                        "graft implementation must be a string literal",
                    ));
                    return;
                };
                let mut cut = cut;
                if let Some(end) = end {
                    cut.push_str(" to ");
                    cut.push_str(&end);
                }
                self.entries.push(GraftSyntax {
                    cut,
                    graft: graft.value(),
                    full,
                    location: location(mac.span()),
                });
                index += 1;
            }
        }
    }
    let mut visitor = Visitor {
        entries: &mut entries,
        error: None,
    };
    visitor.visit_file(&file);
    visitor.error.map_or(Ok(entries), Err)
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
mod declaration_tests {
    use super::{application_entries, graft_entries};

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

    #[test]
    fn graft_parser_collects_single_and_full_cuts() {
        let source = r#"
fn application_plan() {
    let _plan = nichlink::graft_plan!(framework,
        cut ["root/a1/b2"] graft "canvas_fast",
        cut ["root/a"] full graft "a_fast",
    );
}
"#;
        let entries = graft_entries(source).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].cut, "root/a1/b2");
        assert!(!entries[0].full);
        assert_eq!(entries[1].graft, "a_fast");
        assert!(entries[1].full);
    }

    #[test]
    fn graft_parser_keeps_range_endpoints() {
        let source = r#"nichlink::graft_plan!(framework, cut ["root/a1" to "root/a3"] graft "replacement");"#;
        let entries = graft_entries(source).unwrap();
        assert_eq!(entries[0].cut, "root/a1 to root/a3");
    }

    #[test]
    fn graft_parser_accepts_unbracketed_single_cut() {
        let source = r#"fn plan() { nichlink::graft_plan!(framework, cut "root/a" full graft "replacement"); }"#;
        let entries = graft_entries(source).unwrap();
        assert_eq!(entries[0].cut, "root/a");
        assert!(entries[0].full);
    }

    #[test]
    fn graft_parser_collects_declaration_only_static_plans() {
        let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
    cut "root/a" graft "replacement",
);"#;
        let entries = graft_entries(source).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].cut, "root/a");
        assert_eq!(entries[0].graft, "replacement");
    }

    #[test]
    fn graft_parser_ignores_removed_macro_names() {
        let source =
            r#"fn plan() { nichlink::graft!(framework, cut "root/a" graft "replacement"); }"#;
        assert!(graft_entries(source).unwrap().is_empty());
    }
}
