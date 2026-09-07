//! Parser shared by build-time checks and live authoring.
//! 构建期检查与实时编辑共用的注册面解析器。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use proc_macro2::{Delimiter, Span, TokenStream, TokenTree};
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
            Ok(fields) => self.faces.push(FaceSyntax {
                macro_name,
                location: location(item.mac.span()),
                fields,
            }),
            Err(error) => self.error = Some(error),
        }
    }
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

fn split_top_level(tokens: TokenStream) -> Vec<TokenStream> {
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

fn path_to_string(path: &syn::Path) -> String {
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

fn location(span: Span) -> SyntaxLocation {
    let start = span.start();
    SyntaxLocation {
        line: start.line,
        column: start.column + 1,
    }
}

fn syntax_error(span: Span, message: impl Into<String>) -> FaceSyntaxError {
    FaceSyntaxError {
        message: message.into(),
        location: Some(location(span)),
    }
}

#[cfg(test)]
mod tests {
    use super::{ParentSyntax, parse_face, source_references};

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
