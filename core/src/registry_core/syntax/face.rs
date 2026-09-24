//! Registration-face parser shared by build-time checks and live authoring.
//! 构建期检查与实时编辑共用的注册面解析器。
//!
//! This page owns the face grammar: finding declarations in a Rust file,
//! exposing their fields, and splicing a replacement macro back into the source.
//! The lexical helpers live in `tokens` and the field-value decoders in
//! `fields`; both are re-exported here, so `syntax::*` keeps its one namespace.
//! 本页拥有注册面语法：在 Rust 文件中发现声明、暴露其字段，并把替换用宏拼回源码。
//! 词法辅助函数位于 `tokens`，字段取值解码位于 `fields`；两者都在此再导出，
//! `syntax::*` 因此仍只有一个命名空间。

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use proc_macro2::{TokenStream, TokenTree};

use syn::spanned::Spanned;
use syn::visit::Visit;

#[path = "tokens.rs"]
mod tokens;
use tokens::end_location;
pub use tokens::{
    compact_tokens, location, path_to_string, split_face_fields, split_top_level, syntax_error,
};

#[path = "fields.rs"]
mod fields;
use fields::parse_fields;

/// A position inside a parsed source file, one-based in both axes.
/// 已解析源文件里的位置，两个轴都从 1 开始。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxLocation {
    /// One-based line number.
    /// 从 1 开始的行号。
    pub line: usize,
    /// One-based column number, counted in characters.
    /// 从 1 开始的列号，按字符计。
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
    /// The registration macro's name exactly as written (`control_object`,
    /// `root_object`, a generated `*_object!` alias, …).
    /// 注册宏的名字，按源码原文（`control_object`、`root_object`、生成的
    /// `*_object!` 别名等）。
    pub macro_name: String,
    /// The `cfg` gates the declaration carries, if any, exactly as written.
    /// 声明携带的 `cfg` 门控（若有），按原文保留。
    pub cfg: Option<String>,
    /// Where the macro invocation starts.
    /// 宏调用的起点位置。
    pub location: SyntaxLocation,
    /// Exclusive end of the macro invocation in the source file.
    /// 宏调用在源码中的排他结束位置。
    pub end: SyntaxLocation,
    fields: BTreeMap<String, FieldSyntax>,
}

/// How a declaration names its parent, kept as written rather than resolved.
/// 声明如何命名其父级，按原文保留而不在此解析。
///
/// The three shapes exist because the three registrations that accept a parent
/// spell it differently; resolving them is the build-time identity code's job.
/// 之所以有三种形状，是因为接受父级的三类注册写法不同；解析它们是构建期身份代码的事。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParentSyntax {
    /// The package root, written as `crate::root_node_id(env!("CARGO_PKG_NAME"))`.
    /// 包根，写成 `crate::root_node_id(env!("CARGO_PKG_NAME"))`。
    Root,
    /// The parent is named by a path and a kind, as `NodeId::from_path` takes them.
    /// 父级由路径与 kind 命名，即 `NodeId::from_path` 接收的两个参数。
    FromPath {
        /// The `source` literal.
        /// `source` 字面量。
        source: String,
        /// The `kind` literal.
        /// `kind` 字面量。
        kind: String,
    },
    /// The parent is named by an expression path (typically `crate::x::NODE_ID`).
    /// 父级由一个表达式路径命名（通常是 `crate::x::NODE_ID`）。
    NodePath(String),
}

/// Why a declaration could not be parsed, and where when that is known.
/// 声明为何解析不了；位置已知时一并给出。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceSyntaxError {
    /// Human-readable reason.
    /// 人类可读的原因。
    pub message: String,
    /// Where parsing failed; `None` when the failure is about the file as a whole.
    /// 解析失败的位置；失败针对整个文件时为 `None`。
    pub location: Option<SyntaxLocation>,
}

/// Paths one file's executable expressions mention, and how much the scan could
/// actually see.
/// 一个文件的可执行表达式提到的路径，以及这次扫描实际能看到多少。
///
/// A caller that prunes faces from this list must respect `conservative`: the
/// scan is lexical, so anything it cannot follow has to widen the result instead
/// of silently shrinking it.
/// 依据这份清单裁剪注册面的调用方必须尊重 `conservative`：扫描是词法层面的，凡它跟不下去
/// 的东西都必须让结果变宽，而不是悄悄变窄。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SourceReferences {
    /// Every path reached through an expression, with imports excluded.
    /// 经表达式触达的每个路径，不含导入。
    pub paths: BTreeSet<String>,
    /// Set when the scan met something it cannot follow (macro expansion, a trait
    /// object, `include!`), so the caller must not prune on this list alone.
    /// 扫描遇到跟不下去的东西（宏展开、trait object、`include!`）时置位，调用方不得仅凭
    /// 这份清单裁剪。
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

/// Whether `source` declares a registration face.
/// 源码是否声明了注册面。
///
/// A generated-marker line short-circuits the answer: a snapshot this tooling
/// wrote is a face by construction, and parsing it again is wasted work. For
/// every other file the parse decides.
/// 生成标记行会短路答案：本工具写出的快照按构造就是注册面，再解析一遍是白费。其余
/// 文件一律由解析裁决。
pub fn is_face_source(source: &str, marker: &str) -> bool {
    source.lines().any(|line| line == marker) || matches!(parse_face(source), Ok(Some(_)))
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

/// Render a token stream as compact Rust source text.
/// 把 token 流渲染成紧凑的 Rust 源码文本。
///
/// The by-reference twin of [`compact_tokens`], kept for the field reader.
/// [`compact_tokens`] 的按引用版本，供字段读取器使用。
pub(super) fn compact(tokens: &TokenStream) -> String {
    compact_tokens(tokens.clone())
}

/// Split a typed cut's tokens on a top-level `to`, returning both sides as
/// compact source text: `cut(a to b)` names a sibling range verbatim.
/// 在类型化切口的 token 里按顶层 `to` 切分，返回两侧的紧凑源码文本：
/// `cut(a to b)` 就是用原样 Rust 表达兄弟区间。
pub(super) fn split_typed_range(tokens: Vec<TokenTree>) -> Option<(String, String)> {
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
    stable_name: "button",
}
#[test] fn paints() { assert_eq!(Button.paint(), 7); }
"#;
        let replacement = r#"crate::control_object! {
    kind: Button,
    stable_name: "button_graft",
}"#;

        let copied = replace_face_macro(source, replacement).unwrap();

        assert!(copied.contains("pub fn paint(&self) -> u32 { 7 }"));
        assert!(copied.contains("stable_name: \"button_graft\""));
        assert!(!copied.contains("stable_name: \"button\","));
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
}
