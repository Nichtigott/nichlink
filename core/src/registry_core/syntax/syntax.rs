//! Parser shared by build-time checks and live authoring.
//! 构建期检查与实时编辑共用的注册面解析器。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};

use crate::registry_core::declaration::FACE_FIELD_ORDER;
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
    /// The `cfg` gates the declaration carries, if any, exactly as written.
    /// 声明携带的 `cfg` 门控（若有），按原文保留。
    pub cfg: Option<String>,
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

    /// The `cfg` attribute written on the declaration, if any.
    /// 声明上写下的 `cfg` 属性（若有）。
    pub fn cfg(&self) -> Option<&str> {
        self.cfg.as_deref()
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
        error: None,
    };
    visitor.visit_file(&file);
    if let Some(error) = visitor.error {
        Err(error)
    } else {
        Ok(visitor.faces)
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
    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        if self.error.is_some() {
            return;
        }
        let Some(segment) = item.mac.path.segments.last() else {
            return;
        };
        let macro_name = segment.ident.to_string();
        let is_face_macro = matches!(macro_name.as_str(), "control_object" | "external_object")
            || macro_name.ends_with("_object");
        if !is_face_macro {
            return;
        }
        match parse_fields(item.mac.tokens.clone(), item.mac.span()) {
            Ok(fields) => {
                let span = item.span();
                let cfg = item.attrs.iter().find_map(|attribute| {
                    attribute
                        .path()
                        .is_ident("cfg")
                        .then(|| {
                            attribute
                                .parse_args::<TokenStream>()
                                .ok()
                                .map(|tokens| compact(&tokens))
                        })
                        .flatten()
                });
                self.faces.push(FaceSyntax {
                    macro_name,
                    cfg,
                    location: location(span),
                    end: end_location(span),
                    fields,
                })
            }
            Err(error) => self.error = Some(error),
        }
    }
}

fn parse_fields(
    tokens: TokenStream,
    fallback_span: Span,
) -> Result<BTreeMap<String, FieldSyntax>, FaceSyntaxError> {
    let mut fields = BTreeMap::new();
    for field in split_face_fields(tokens) {
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

/// Render a token stream as compact Rust source text.
/// 把 token 流渲染成紧凑的 Rust 源码文本。
///
/// `TokenStream`'s own rendering spaces out `::`, which is valid but noisy in
/// generated code. A typed graft selector is emitted verbatim, so keep it close
/// to what the author wrote.
/// `TokenStream` 自身渲染会在 `::` 周围加空格，虽然合法，但在生成代码里很吵。
/// 类型化 graft 选择器会被原样发射，因此尽量贴近作者的写法。
pub fn compact_tokens(tokens: TokenStream) -> String {
    tokens
        .to_string()
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace(") ", ")")
}

/// Split a typed cut's tokens on a top-level `to`, returning both sides as
/// compact source text: `cut(a to b)` names a sibling range verbatim.
/// 在类型化切口的 token 里按顶层 `to` 切分，返回两侧的紧凑源码文本：
/// `cut(a to b)` 就是用原样 Rust 表达兄弟区间。
fn split_typed_range(tokens: Vec<TokenTree>) -> Option<(String, String)> {
    let position = tokens
        .iter()
        .position(|token| matches!(token, TokenTree::Ident(value) if value == "to"))?;
    let start = tokens[..position].iter().cloned().collect::<TokenStream>();
    let finish = tokens[position + 1..]
        .iter()
        .cloned()
        .collect::<TokenStream>();
    if start.is_empty() || finish.is_empty() {
        return None;
    }
    Some((compact_tokens(start), compact_tokens(finish)))
}

/// Split a face body into `name: value` fields.
/// 把注册面主体切成 `name: value` 字段。
///
/// Authoring is hand-written, so this reader is deliberately as tolerant as the
/// compile-time front end: `,` and `;` both separate fields, a forgotten
/// separator still ends a field at the next `name:`, a trailing separator is
/// ignored, and the fields may appear in any order. A path's colon is joint
/// with the next colon, so `crate::x` stays inside its value rather than
/// starting a field.
/// 作者是手写注册面的，因此这个读取器刻意与编译期前端一样宽容：`,` 与 `;` 都算分隔
/// 符，漏写分隔符也能在下一个 `name:` 处收尾，末尾多余的分隔符被忽略，字段顺序任意。
/// 路径的冒号与下一个冒号相连，因此 `crate::x` 留在自己的值里，不会被当成新字段。
#[doc(hidden)]
pub fn split_face_fields(tokens: TokenStream) -> Vec<TokenStream> {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    let mut fields = Vec::new();
    let mut current = TokenStream::new();
    let mut value_started = false;
    let mut angles = 0usize;
    let mut index = 0;
    while index < tokens.len() {
        if let TokenTree::Punct(punct) = &tokens[index] {
            match punct.as_char() {
                // `<` and `>` are punctuation, not delimiters, so a generic
                // argument list has to be tracked by hand: its commas separate
                // type arguments, not fields. `>>` closes two levels at once.
                // `<`/`>` 是标点而不是定界符，因此泛型实参列表必须手工跟踪深度：其中
                // 的逗号分隔的是类型实参而不是字段。`>>` 一次关闭两层。
                '<' => {
                    angles += if punct.spacing() == proc_macro2::Spacing::Joint {
                        2
                    } else {
                        1
                    };
                    current.extend([tokens[index].clone()]);
                    value_started = true;
                    index += 1;
                    continue;
                }
                '>' => {
                    let close = if punct.spacing() == proc_macro2::Spacing::Joint {
                        2
                    } else {
                        1
                    };
                    angles = angles.saturating_sub(close);
                    current.extend([tokens[index].clone()]);
                    index += 1;
                    continue;
                }
                ',' | ';' if angles == 0 => {
                    if !current.is_empty() {
                        fields.push(std::mem::take(&mut current));
                        value_started = false;
                    }
                    index += 1;
                    continue;
                }
                _ => {}
            }
        }
        if angles == 0 && value_started && starts_face_field(&tokens, index) {
            fields.push(std::mem::take(&mut current));
            value_started = false;
            continue;
        }
        value_started = true;
        current.extend([tokens[index].clone()]);
        index += 1;
    }
    if !current.is_empty() {
        fields.push(current);
    }
    fields
}

/// Whether a `name:` begins at this token, which is how a missing separator is
/// detected. `crate::x` does not: its first colon is joint with the second.
/// 这里是否开始了 `name:`——漏写分隔符就是靠它发现的。`crate::x` 不算：它的第一个
/// 冒号与第二个相连。
fn starts_face_field(tokens: &[TokenTree], index: usize) -> bool {
    let Some(TokenTree::Ident(name)) = tokens.get(index) else {
        return false;
    };
    // Only a name the vocabulary knows starts a field. That is what keeps a
    // closure's typed parameter (`|a: u32| a`) or any other `name:` inside a
    // value from splitting the field in two.
    // 只有词表认识的键才算字段起点。闭包的类型标注参数（`|a: u32| a`）或值里其它
    // `name:` 形状因此不会把字段切成两半。
    if !FACE_FIELD_ORDER.contains(&name.to_string().as_str()) {
        return false;
    }
    match tokens.get(index + 1) {
        Some(TokenTree::Punct(colon)) if colon.as_char() == ':' => {
            colon.spacing() == proc_macro2::Spacing::Alone
        }
        _ => false,
    }
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
mod tests {
    use super::{ParentSyntax, parse_face, replace_face_macro, source_references};

    #[test]
    fn parses_multiline_registration_tokens_and_locations() {
        let source = r#"
crate::control_object! {
    kind: Button,
    name: { zh: "按钮", en: "Button" },
    requires: [
        "layout.viewport" => "ControlRegistry",
        "draw.basic" => "BasicDrawing",
    ],
    parent: crate::NodeId::from_path("control/control.rs", "ControlRegistry"),
    getting_from_other_registry: Some("engine"),
}
"#;
        let face = parse_face(source).unwrap().unwrap();

        assert_eq!(face.path("kind").as_deref(), Some("Button"));
        assert_eq!(face.localized("name", "zh").as_deref(), Some("按钮"));
        assert_eq!(face.field_location("requires").unwrap().line, 5);
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
crate::control_object! {
    kind: Workspace,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
}
"#;
        let face = parse_face(source).unwrap().unwrap();

        assert_eq!(face.parent(), Some(ParentSyntax::Root));
    }

    #[test]
    fn rejects_duplicate_fields() {
        let source = "crate::control_object! { kind: First, kind: Second }";
        let error = parse_face(source).unwrap_err();
        assert!(error.message.contains("duplicate field `kind`"));
    }

    #[test]
    fn replacing_a_face_preserves_its_rust_implementation() {
        let source = r#"pub struct Button;
impl Button { pub fn paint(&self) -> u32 { 7 } }
crate::control_object! {
    kind: Button,
    registry_name: button,
}
#[test] fn paints() { assert_eq!(Button.paint(), 7); }
"#;
        let replacement = r#"crate::control_object! {
    kind: Button,
    registry_name: button_graft,
}"#;

        let copied = replace_face_macro(source, replacement).unwrap();

        assert!(copied.contains("pub fn paint(&self) -> u32 { 7 }"));
        assert!(copied.contains("registry_name: button_graft"));
        assert!(!copied.contains("registry_name: button,"));
        assert!(copied.contains("#[test] fn paints()"));
    }

    #[test]
    fn source_references_ignore_imports_strings_comments_and_registration_data() {
        let source = r#"
use crate::unused::Thing;
fn run() {
    crate::control::object::button::dispatch_action("unused::fake()", true);
    // crate::comment::fake();
    crate::control_object! { kind: Fake, parent: crate::hidden::NODE_ID }
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
    /// The `cfg` gate the declaration carries, if any, exactly as written.
    /// 声明携带的 `cfg` 门控（若有），按原文保留。
    pub cfg: Option<String>,
    /// Present when `cut(...)` / `graft(...)` supplied Rust expressions. The
    /// renderer emits those expressions verbatim, so the compiler — and any
    /// editor that resolves Rust paths — sees the real target instead of a
    /// string the tooling would have to interpret.
    /// 当 `cut(...)` / `graft(...)` 给出 Rust 表达式时存在。渲染器会原样发射这些
    /// 表达式，因此编译器和任何能解析 Rust 路径的编辑器看到的都是真实目标，
    /// 而不是需要工具自己解释的字符串。
    pub expressions: Option<GraftExpressions>,
}

/// Rust expressions used by a typed graft cut, in source order.
/// 类型化 graft 切口使用的 Rust 表达式，按源码顺序。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraftExpressions {
    pub cut: String,
    pub cut_end: Option<String>,
    pub graft: String,
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
        /// Only a declaration written as an item belongs in the build's plan: a
        /// `graft_plan!` inside a function is a runtime expression, and a plan
        /// inside `#[cfg(test)] mod tests` describes tests. Both used to be
        /// captured, shipping test cuts into the release table and pinning faces
        /// against pruning.
        /// 只有写成条目的声明才属于构建计划：函数里的 `graft_plan!` 是运行时表达式，
        /// `#[cfg(test)] mod tests` 里的计划描述的是测试。两者过去都会被收集，把测试
        /// 切口带进发布表，并让注册面躲过剪枝。
        fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
            self.collect(item);
        }

        fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
            // A module the compiler may drop cannot contribute to the plan.
            // 编译器可能丢弃的模块不能参与计划。
            if item
                .attrs
                .iter()
                .any(|attribute| attribute.path().is_ident("cfg"))
            {
                return;
            }
            syn::visit::visit_item_mod(self, item);
        }
    }

    impl<'a> Visitor<'a> {
        fn collect(&mut self, item: &syn::ItemMacro) {
            let cfg = item.attrs.iter().find_map(|attribute| {
                attribute
                    .path()
                    .is_ident("cfg")
                    .then(|| {
                        attribute
                            .parse_args::<TokenStream>()
                            .ok()
                            .map(|tokens| compact(&tokens))
                    })
                    .flatten()
            });
            let mac = &item.mac;
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
                // `cut(<expr>)` / `cut(<expr> to <expr>)` names the target with
                // Rust expressions, so the compiler and any editor that resolves
                // Rust paths see the real face instead of a string the tooling
                // would have to interpret.
                // `cut(<expr>)` / `cut(<expr> to <expr>)` 用 Rust 表达式命名目标，
                // 让编译器和任何能解析 Rust 路径的编辑器看到真实的注册面，而不是
                // 需要工具自己解释的字符串。
                let typed_cut = match &path_token {
                    Some(TokenTree::Group(group))
                        if group.delimiter() == Delimiter::Parenthesis =>
                    {
                        let inner = group.stream().into_iter().collect::<Vec<_>>();
                        Some(match split_typed_range(inner) {
                            Some((start, finish)) => (start, Some(finish)),
                            None => (compact_tokens(group.stream()), None),
                        })
                    }
                    _ => None,
                };
                let (cut, end, cut_span, typed) = match typed_cut {
                    Some((start, finish)) => (start, finish, mac.span(), true),
                    None => {
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
                                        syntax_error(
                                            literal.span(),
                                            "graft cut path must be a string",
                                        )
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
                                    "graft cut expects `[path]`, a path string, or `(<expression>)`",
                                ));
                                return;
                            }
                        };
                        let range = path_text.is_none()
                            && path_tokens.len() == 3
                            && matches!(&path_tokens[1], TokenTree::Ident(value) if value == "to");
                        let resolved = if range {
                            let start = syn::parse2::<syn::LitStr>(path_tokens[0].clone().into())
                                .map_err(|_| {
                                    syntax_error(path_span, "graft range start must be a string")
                                });
                            let finish = syn::parse2::<syn::LitStr>(path_tokens[2].clone().into())
                                .map_err(|_| {
                                    syntax_error(path_span, "graft range end must be a string")
                                });
                            match (start, finish) {
                                (Ok(start), Ok(finish)) => {
                                    Ok((start.value(), Some(finish.value())))
                                }
                                (Err(error), _) | (_, Err(error)) => Err(error),
                            }
                        } else {
                            Ok((
                                path_text.unwrap_or_else(|| {
                                    syn::parse2::<syn::LitStr>(
                                        path_tokens.clone().into_iter().collect(),
                                    )
                                    .map(|value| value.value())
                                    .unwrap_or_else(|_| {
                                        path_tokens
                                            .iter()
                                            .map(ToString::to_string)
                                            .collect::<String>()
                                    })
                                }),
                                None,
                            ))
                        };
                        let (cut, end) = match resolved {
                            Ok(pair) => pair,
                            Err(error) => {
                                self.error = Some(error);
                                return;
                            }
                        };
                        (cut, end, path_span, false)
                    }
                };
                if cut.is_empty() {
                    self.error = Some(syntax_error(cut_span, "graft cut path is empty"));
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
                let graft_token = tokens.get(index).cloned();
                let (graft, graft_expr) = match graft_token {
                    Some(TokenTree::Group(group))
                        if group.delimiter() == Delimiter::Parenthesis =>
                    {
                        let text = compact_tokens(group.stream());
                        (text.clone(), Some(text))
                    }
                    Some(TokenTree::Literal(value)) => {
                        let Ok(text) = syn::parse_str::<syn::LitStr>(&value.to_string()) else {
                            self.error = Some(syntax_error(
                                value.span(),
                                "graft implementation must be a string literal or `(<expression>)`",
                            ));
                            return;
                        };
                        (text.value(), None)
                    }
                    _ => {
                        self.error = Some(syntax_error(
                            mac.span(),
                            "graft expects a string literal or `(<expression>)`",
                        ));
                        return;
                    }
                };
                // A cut names both sides the same way, so the generated table
                // never has to mix a resolved identity with an unresolved name.
                // 一条切口的命名方式必须一致，生成表因此不会混合已解析身份与未解析名称。
                let expressions = match (typed, graft_expr) {
                    (true, Some(graft)) => Some(GraftExpressions {
                        cut: cut.clone(),
                        cut_end: end.clone(),
                        graft,
                    }),
                    (false, None) => None,
                    _ => {
                        self.error = Some(syntax_error(
                            mac.span(),
                            "a graft cut must name both sides with Rust expressions or both with strings",
                        ));
                        return;
                    }
                };
                let mut cut = cut;
                if let Some(end) = end {
                    cut.push_str(" to ");
                    cut.push_str(&end);
                }
                self.entries.push(GraftSyntax {
                    cut,
                    graft,
                    full,
                    location: location(mac.span()),
                    cfg: cfg.clone(),
                    expressions,
                });
                index += 1;
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
    use super::{application_entries, graft_entries, parse_face, split_face_fields};
    use proc_macro2::TokenStream;

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
nichlink::static_graft_plan!(FRAMEWORK,
    cut ["root/a1/b2"] graft "canvas_fast",
    cut ["root/a"] full graft "a_fast",
);
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
        // A declaration belongs at the entry, so the fixture is an item; the
        // tests below cover plans that sit somewhere else.
        // 声明应当写在入口处，因此夹具写成条目；位置不当的计划由下面的测试覆盖。
        let source =
            r#"nichlink::static_graft_plan!(FRAMEWORK, cut "root/a" full graft "replacement");"#;
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

    /// Only an item-position declaration belongs to the build's plan.
    /// 只有条目位置的声明才属于构建计划。
    #[test]
    fn graft_parser_ignores_declarations_that_are_not_items() {
        let in_function = r#"
fn plan() {
    let _ = nichlink::graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
}
"#;
        assert!(graft_entries(in_function).unwrap().is_empty());

        let in_test_module = r#"
#[cfg(test)]
mod tests {
    nichlink::static_graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
}
"#;
        assert!(graft_entries(in_test_module).unwrap().is_empty());

        let at_entry = r#"
#[cfg(feature = "optional-graft")]
nichlink::static_graft_plan!(FRAMEWORK, cut "root/a" graft "replacement");
"#;
        let entries = graft_entries(at_entry).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(
            entries[0].cfg.as_deref(),
            Some("feature = \"optional-graft\"")
        );
    }

    #[test]
    fn graft_parser_ignores_removed_macro_names() {
        let source =
            r#"fn plan() { nichlink::graft!(framework, cut "root/a" graft "replacement"); }"#;
        assert!(graft_entries(source).unwrap().is_empty());
    }

    /// The typed form keeps both sides as Rust expressions so the compiler and
    /// any editor that resolves Rust paths can see the real target.
    /// 类型化形式把两侧都保留为 Rust 表达式，编译器和任何能解析 Rust 路径的编辑器
    /// 因此都能看到真实目标。
    #[test]
    fn graft_parser_keeps_typed_expressions() {
        let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
            cut(crate::control::object::button::NODE_ID)
                graft(graft_crate::button_fast::NODE_ID),
        );"#;
        let entries = graft_entries(source).unwrap();
        let expressions = entries[0].expressions.as_ref().expect("typed expressions");
        assert_eq!(
            expressions.cut, "crate::control::object::button::NODE_ID",
            "the host path is preserved verbatim so the compiler resolves it"
        );
        assert_eq!(expressions.graft, "graft_crate::button_fast::NODE_ID");
        assert_eq!(expressions.cut_end, None);
        assert!(!entries[0].full);
    }

    #[test]
    fn graft_parser_keeps_a_typed_sibling_range() {
        let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
            cut(crate::control::object::button::NODE_ID to crate::control::object::slider::NODE_ID)
                graft(graft_crate::fast::NODE_ID),
        );"#;
        let entries = graft_entries(source).unwrap();
        let expressions = entries[0].expressions.as_ref().expect("typed expressions");
        assert_eq!(expressions.cut, "crate::control::object::button::NODE_ID");
        assert_eq!(
            expressions.cut_end.as_deref(),
            Some("crate::control::object::slider::NODE_ID")
        );
        // The description keeps the readable range form for diagnostics.
        // 描述字段保留可读的区间形式，供诊断使用。
        assert_eq!(
            entries[0].cut,
            "crate::control::object::button::NODE_ID to crate::control::object::slider::NODE_ID"
        );
    }

    /// A cut may not mix a resolved identity with an unresolved name.
    /// 一条切口不允许混合"已解析身份"与"未解析名称"。
    #[test]
    fn graft_parser_rejects_mixed_typed_and_string_sides() {
        let source = r#"nichlink::static_graft_plan!(FRAMEWORK,
            cut(crate::control::NODE_ID) graft "button_fast",
        );"#;
        let error = graft_entries(source).unwrap_err();
        assert!(
            error.message.contains("both sides"),
            "unexpected message: {}",
            error.message
        );
    }
    /// The build has to follow a declaration's `cfg` gate, so the parser keeps
    /// it exactly as written.
    /// 构建需要跟随声明的 `cfg` 门控，因此解析器按原文保留它。
    #[test]
    fn a_declaration_keeps_its_cfg_gate() {
        let face = parse_face(
            "// generated-by=NichLink\n#[cfg(feature = \"optional-face\")]\n\
             crate::root_object! {\n    kind: Widget,\n}\n",
        )
        .expect("parse")
        .expect("face");
        assert_eq!(face.cfg(), Some("feature = \"optional-face\""));
    }

    /// A generic argument list and a closure parameter list are values, not
    /// field lists: their commas and `name:` pairs must not split a field.
    /// 泛型实参列表与闭包参数列表是值而不是字段列表：其中的逗号与 `name:` 都不该把
    /// 字段切开。
    #[test]
    fn face_field_splitting_survives_generics_and_closures() {
        let body: TokenStream = syn::parse_str(
            "kind: Tool, preset: crate::P<u8, u16>, flow: |a: u32| a, needs_registry: true",
        )
        .expect("token stream");
        let fields = split_face_fields(body);
        let names = fields
            .iter()
            .map(|field| {
                field
                    .clone()
                    .into_iter()
                    .next()
                    .map_or_else(String::new, |token| token.to_string())
            })
            .collect::<Vec<_>>();
        assert_eq!(names, ["kind", "preset", "flow", "needs_registry"]);
        assert!(
            fields[1].to_string().contains("P < u8 , u16 >"),
            "the generic comma stays in the value: {}",
            fields[1]
        );
        assert!(
            fields[2].to_string().contains("| a : u32 | a"),
            "the closure parameter stays in the value: {}",
            fields[2]
        );
        assert_eq!(
            split_face_fields(syn::parse_str("kind: X;").expect("token stream")).len(),
            1
        );
    }
}
