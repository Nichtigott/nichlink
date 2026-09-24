//! Parsing and rendering helpers for authored registration faces.
//! 注册面创作文件的解析与渲染辅助函数。
//!
//! Only source-text transformations live here. The two filesystem-bound
//! entry points (`kind_from_source_path` and `rule_syntax_for_source`) stay
//! in the run_method authoring shim as thin wrappers around
//! `kind_from_source_text` / `rule_syntax_from_text`.
//! 这里只保留源码文本层面的变换。两个绑定文件系统的入口
//! （`kind_from_source_path` 与 `rule_syntax_for_source`）留在 run_method
//! 的 authoring shim 中，作为 `kind_from_source_text` 与
//! `rule_syntax_from_text` 的薄包装。
//!
//! The field families are split by concern: `flow` translates flow contracts,
//! `admission` the dependency gate, and `rules` the registration rule. This
//! page keeps the shared error type plus the generic list, path, and module
//! helpers, and re-exports every field family so `parse::*` is unchanged.
//! 字段族按关注点拆分：`flow` 转换数据流合同，`admission` 处理依赖门禁，`rules`
//! 处理注册规范。本页保留共用的错误类型以及通用的列表、路径与模块辅助函数，并再
//! 导出每个字段族，因此 `parse::*` 保持不变。

use std::fmt;
use std::path::PathBuf;

use crate::registry_core::declaration::RuntimeCheckSpec;
use crate::registry_core::syntax::parse_face as parse_face_syntax;

use super::validation::{normalized_path, rust_string};

#[path = "flow.rs"]
mod flow;
pub use flow::*;
#[path = "admission.rs"]
mod admission;
pub use admission::*;
#[path = "rules.rs"]
mod rules;
pub use rules::*;

/// Why authored face text was refused.
/// 创作注册面文本被拒绝的原因。
///
/// The parser used to answer with a bare `String`; an error type keeps the
/// message a caller renders while still telling a parse failure apart from any
/// other string that happens to travel through the same code.
/// 解析器过去用裸 `String` 作答；有了错误类型，调用方既能渲染消息，又能把解析失败与
/// 恰好流经同一段代码的其它字符串区分开。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceParseError {
    /// Human-readable reason, rendered verbatim by `Display`.
    /// 人类可读的原因，`Display` 原样渲染。
    pub message: String,
}

impl FaceParseError {
    /// A refusal with its human-readable reason.
    /// 带人类可读理由的拒绝。
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for FaceParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for FaceParseError {}

impl From<String> for FaceParseError {
    fn from(message: String) -> Self {
        Self { message }
    }
}

/// The historical string form, for callers that only render the failure.
/// 历史字符串形式，供只做渲染的调用方使用。
impl From<FaceParseError> for String {
    fn from(error: FaceParseError) -> Self {
        error.message
    }
}

/// The source file a `crate::…` module path names, in this crate's
/// `<dir>/<name>.rs` layout; `None` when the path cannot name a file at all.
/// `crate::…` 模块路径在本 crate 的 `<dir>/<name>.rs` 布局下指向的源文件；该路径根本无法
/// 命名文件时为 `None`。
pub fn module_source_from_node_path(module: &str) -> Option<String> {
    let module = module.strip_prefix("crate::").unwrap_or(module);
    let mut segments = module.split("::").filter(|segment| !segment.is_empty());
    let first = segments.next()?;
    let mut path = PathBuf::from(first);
    let mut last = first;
    for segment in segments {
        path.push(segment);
        last = segment;
    }
    path.push(format!("{last}.rs"));
    Some(normalized_path(&path))
}

/// Read the declared `kind` out of a registration-face source text.
/// 从注册面源码文本中读取它声明的 `kind`。
pub fn kind_from_source_text(text: &str) -> Option<String> {
    parse_face_syntax(text)
        .ok()?
        .and_then(|face| face.path("kind"))
}

/// Read the quoted strings of one bracketed field, e.g. `marker: ["a", "b"]`.
/// 读取一个方括号字段里的引号字符串，例如 `marker: ["a", "b"]`。
pub fn quoted_list_field(text: &str, marker: &str) -> Vec<String> {
    let Some(start) = text.find(marker) else {
        return Vec::new();
    };
    let rest = &text[start + marker.len()..];
    let Some(open) = rest.find('[') else {
        return Vec::new();
    };
    let Some(close) = rest[open + 1..].find(']') else {
        return Vec::new();
    };
    rest[open + 1..open + 1 + close]
        .split(',')
        .filter_map(|value| {
            value
                .trim()
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .map(str::to_owned)
        })
        .collect()
}

/// The module name implied by a source file path: its file stem, or `module`
/// when the stem is not valid UTF-8.
/// 源文件路径隐含的模块名：文件主干名；主干不是合法 UTF-8 时为 `module`。
pub fn module_name_from_path(path: &std::path::Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("module")
        .to_owned()
}

/// Render one comma-separated value as a quoted list field, indented and
/// newline-terminated; an empty value renders as nothing at all.
/// 把一个逗号分隔的取值渲染成带引号的列表字段（缩进且以换行结尾）；取值为空时不渲染任何
/// 内容。
///
/// The empty case is the point: a field with no entries must disappear from the
/// macro rather than appear as `field: []`, because the authoring writer diffs
/// the rendered text and an empty-but-present field would look like an edit.
/// 空取值这一点是关键：没有条目的字段必须从宏里消失，而不是写成 `field: []`——创作写入方
/// 会对渲染文本做差异比较，"存在但为空"的字段会被当成一次修改。
pub fn render_face_list(field: &str, value: &str) -> String {
    let values = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("\"{}\"", rust_string(value)))
        .collect::<Vec<_>>();
    if values.is_empty() {
        String::new()
    } else {
        format!("    {field}: [{}],\n", values.join(", "))
    }
}

/// Render a comma-separated value as quoted literals joined by `, `, for
/// inlining inside a bracketed list.
/// 把逗号分隔的取值渲染成以 `, ` 连接的带引号字面量，供内联进方括号列表。
pub fn render_literal_list(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| format!("\"{}\"", rust_string(item)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render a comma-separated value as Rust paths joined by `, `, keeping the
/// author's spelling because the compiler resolves it.
/// 把逗号分隔的取值渲染成以 `, ` 连接的 Rust 路径，保留作者的写法，因为由编译器解析。
pub fn render_path_list(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Derive the human-facing trait labels from compiler-checked Rust paths.
/// 从参与编译检查的 Rust 路径派生人类可读的 trait 名称。
///
/// This is the authoring side of the same rule the macro applies through
/// `__face_trait_labels_or!`: a path decides the label, and a label without a
/// path stands alone as the unchecked claim it is.
/// 这是宏经 `__face_trait_labels_or!` 施加的同一条规则在创作侧的写法：有路径时由路径决定
/// 标签，没有路径的标签则独立成立——它本来就是一条未经检查的声明。
pub fn trait_names_from_paths(value: &str) -> Result<String, FaceParseError> {
    value
        .split(',')
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(|source| {
            let path = syn::parse_str::<syn::Path>(source)
                .map_err(|_| FaceParseError::new(format!("`{source}` is not a Rust trait path")))?;
            path.segments
                .last()
                .map(|segment| segment.ident.to_string())
                .ok_or_else(|| FaceParseError::new(format!("`{source}` has no trait name")))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|names| names.join(","))
}

/// Render a runtime-check list back to the expression text the macro takes,
/// refusing anything `RuntimeCheckSpec` cannot parse.
/// 把运行期检查列表渲染回宏接受的表达式文本；`RuntimeCheckSpec` 解析不了的一律拒绝。
pub fn render_expression_list(value: &str) -> Result<String, FaceParseError> {
    RuntimeCheckSpec::parse_list(value)
        .map_err(FaceParseError::from)
        .map(|checks| {
            checks
                .into_iter()
                .map(RuntimeCheckSpec::expression)
                .collect::<Vec<_>>()
                .join(", ")
        })
}

/// Render an optional string field: an empty value or `none` becomes `None`,
/// anything else becomes `Some("…")` with the text escaped.
/// 渲染可选字符串字段：空值或 `none` 变成 `None`，其余变成转义后的 `Some("…")`。
pub fn render_optional_source(value: &str) -> Result<String, FaceParseError> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        Ok("None".to_owned())
    } else {
        Ok(format!("Some(\"{}\")", rust_string(value)))
    }
}

/// Split a comma-separated list into trimmed, non-empty owned strings.
/// 把逗号分隔的列表切成去空白、去空项的自有字符串。
pub fn split_csv_owned(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Accept or refuse an optional string field's text, which is the same rule
/// `render_optional_source` applies — minus the rendering.
/// 接受或拒绝可选字符串字段的文本，与 `render_optional_source` 用的是同一条规则——只是
/// 不做渲染。
pub fn parse_optional_source(value: &str) -> Result<(), FaceParseError> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return Ok(());
    }
    if value.contains(['\n', '\r']) {
        return Err("getting_from_other_registry cannot contain a newline"
            .to_owned()
            .into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::trait_names_from_paths;

    #[test]
    fn compiler_checked_trait_paths_supply_searchable_labels() {
        assert_eq!(
            trait_names_from_paths("crate::ui::ControlHandle, crate::parts::ActionParts").unwrap(),
            "ControlHandle,ActionParts"
        );
        assert!(trait_names_from_paths("not a path").is_err());
    }
}
