//! Registration-rule field rendering and parsing for authored faces.
//! 注册面 registration_rule 字段的渲染与解析。
//!
//! A rule is written as a compact `preset:…;parts:…;exports:…` list and held as
//! a `RegistrationRule` builder expression. An empty or `ANY` value means "no
//! structural requirement", which is not the same as a rule that rejects
//! everything.
//! 规则写成紧凑的 `preset:…;parts:…;exports:…` 列表，并以 `RegistrationRule`
//! 构建表达式保存。空值或 `ANY` 表示"没有结构要求"，这与拒绝一切的规则不同。
//!
//! The compact spelling is rendered by [`compact_registration_rule`], which is
//! **public API**: a surface that shows or saves the compact form calls it instead
//! of spelling the grammar a second time. Studio assembled the clauses itself while
//! this renderer was private, which is the third member of the `FIXR-01` family
//! (`core/tests/registration_rule_entry.rs`).
//! 紧凑拼法由 [`compact_registration_rule`] 渲染，而它是**公开 API**：展示或保存紧凑形式的
//! 执行面调用它，而不是把语法再拼一遍。本渲染器私有时 Studio 只能自己装配子句——那是 `FIXR-01`
//! 家族的第三例（`core/tests/registration_rule_entry.rs`）。

use crate::registry_core::authoring::validation::rust_string;
use crate::registry_core::declaration::{OwnedRegistrationRule, OwnedRequirementSpec};

use super::{FaceParseError, quoted_list_field, split_csv_owned};

/// Read a quoted value field of the form `.marker("value")`.
/// 读取 `.marker("value")` 形式的引号值字段。
fn quoted_value_field(text: &str, marker: &str) -> Option<String> {
    let rest = text.split_once(marker)?.1;
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_owned())
}

/// The rule that requires nothing: the value `ANY` renders and the value an empty
/// spelling parses into.
/// 没有任何要求的规则：`ANY` 渲染出的那一份，也是空拼法解析出的那一份。
fn rule_requiring_nothing() -> OwnedRegistrationRule {
    OwnedRegistrationRule {
        required_preset: None,
        required_parts: Vec::new(),
        required_exports: Vec::new(),
        required_handle_traits: Vec::new(),
        required_part_traits: Vec::new(),
    }
}

/// Render one registration rule as the compact clause form the editor carries.
/// 将一条注册规则渲染成编辑器所携带的紧凑子句形式。
///
/// This is the kernel's only renderer for the compact registration-rule spelling,
/// and it is public for the same reason `compact_admission` is: the alternative was
/// a second implementation outside the kernel. Studio's Edit form and inspector
/// show this value, and with the renderer private Studio assembled the same clauses
/// itself (`FIXR-01`, same family as the admission copy t8 removed).
/// 这是内核唯一的紧凑注册规范拼法渲染器，公开的理由与 `compact_admission` 相同：另一条路是内核
/// 之外的第二份实现。Studio 的 Edit 表单与检视器展示的就是这个值，而渲染器私有时它只能自己装配
/// 同样的子句（`FIXR-01`，与 t8 删掉的 admission 副本同族）。
///
/// One clause per field, in the historical order (`preset:…`, `parts:…`,
/// `exports:…`, `handle:…`, `part_trait:…`), `;` between them, and `ANY` when the
/// rule requires nothing. A field that is absent renders nothing, while a present
/// but empty preset keeps the historical `preset:` byte.
/// 每个字段一个子句，顺序沿用历史（`preset:…`、`parts:…`、`exports:…`、`handle:…`、
/// `part_trait:…`），之间用 `;`，规则没有任何要求时为 `ANY`。缺席的字段不渲染任何内容，而**出现
/// 但为空**的 preset 保留历史的 `preset:` 字节。
///
/// The output is exactly what [`parse_registration_rule_owned`] reads back, and the
/// historical source-text branch ([`rule_syntax_from_text`]) renders through this
/// function, so the bytes consumers already read did not move. Both halves are
/// pinned from outside the crate by `core/tests/registration_rule_entry.rs`.
/// 输出正是 [`parse_registration_rule_owned`] 读得回的那一份，而历史源码文本分支
/// （[`rule_syntax_from_text`]）经本函数渲染，因此消费方已在读的字节没有移动。两半都由 crate
/// 之外的 `core/tests/registration_rule_entry.rs` 钉住。
///
/// A surface that shows or saves the compact form calls this function; it must not
/// decide the spelling itself.
/// 展示或保存紧凑形式的执行面调用本函数，不得自行决定拼法。
pub fn compact_registration_rule(rule: &OwnedRegistrationRule) -> String {
    let mut clauses = Vec::new();
    if let Some(preset) = &rule.required_preset {
        clauses.push(format!("preset:{preset}"));
    }
    for (key, values) in [
        ("parts", &rule.required_parts),
        ("exports", &rule.required_exports),
        ("handle", &rule.required_handle_traits),
        ("part_trait", &rule.required_part_traits),
    ] {
        if !values.is_empty() {
            clauses.push(format!("{key}:{}", values.join(",")));
        }
    }
    if clauses.is_empty() {
        "ANY".to_owned()
    } else {
        clauses.join(";")
    }
}

/// The const a registration-rule source file declares.
/// 注册规则源文件声明的那个常量。
const RULE_CONST: &str = "REGISTRATION_RULE";

/// Read the rule a registration-rule source file declares, through the AST.
/// 通过 AST 读取注册规则源文件所声明的规则。
///
/// This is the strict sibling of [`rule_syntax_from_text`], added without touching the
/// published entry's signature: `syn::parse_file` locates the `REGISTRATION_RULE` const
/// and reads *its initializer*, so a clause inside a comment is not part of the rule and
/// an `=` outside the initializer cannot mislead the read. Anything it cannot recognize
/// — no such const, an initializer that is not a `RegistrationRule` builder chain, an
/// unknown clause, an argument that is not the string literal or list of string literals
/// the renderer emits — is **refused** instead of silently degrading to `ANY`
/// (audit `KRN-K-10`).
/// 这是 [`rule_syntax_from_text`] 的严格兄弟，且不改动已发布入口的签名：`syn::parse_file` 定位
/// `REGISTRATION_RULE` 常量并读取**它的初始器**，因此注释里的子句不属于规则，初始器之外的 `=`
/// 也不会误导读取。任何它认不出的东西——没有该常量、初始器不是 `RegistrationRule` 构建链、未知
/// 子句、参数不是渲染器发射的字符串字面量或其列表——都会被**拒绝**，而不是静默降级成 `ANY`
/// （审计 `KRN-K-10`）。
pub fn try_rule_syntax_from_text(text: &str) -> Result<String, FaceParseError> {
    let file = syn::parse_file(text).map_err(|error| {
        FaceParseError::new(format!(
            "registration-rule source does not parse as Rust: {error}"
        ))
    })?;
    let expression = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Const(item) if item.ident == RULE_CONST => Some(&item.expr),
            _ => None,
        })
        .ok_or_else(|| {
            FaceParseError::new(format!(
                "registration-rule source declares no `{RULE_CONST}` const"
            ))
        })?;
    Ok(compact_registration_rule(&read_rule_expression(
        expression,
    )?))
}

/// Read one `RegistrationRule` builder expression out of the AST.
/// 从 AST 里读出一条 `RegistrationRule` 构建表达式。
fn read_rule_expression(expression: &syn::Expr) -> Result<OwnedRegistrationRule, FaceParseError> {
    match expression {
        // `RegistrationRule::ANY` is the spelling that requires nothing.
        syn::Expr::Path(path)
            if path
                .path
                .segments
                .last()
                .is_some_and(|segment| segment.ident == "ANY") =>
        {
            Ok(rule_requiring_nothing())
        }
        // `RegistrationRule::new()` starts the chain.
        syn::Expr::Call(call) if is_builder_start(&call.func) => {
            if !call.args.is_empty() {
                return Err(FaceParseError::new(
                    "`RegistrationRule::new` takes no arguments".to_owned(),
                ));
            }
            Ok(rule_requiring_nothing())
        }
        syn::Expr::MethodCall(call) => {
            let mut rule = read_rule_expression(&call.receiver)?;
            let method = call.method.to_string();
            match method.as_str() {
                "require_preset" => {
                    rule.required_preset = Some(single_string_argument(&call.args, &method)?);
                }
                "require_parts" => rule.required_parts = string_list_argument(&call.args, &method)?,
                "require_exports" => {
                    rule.required_exports = string_list_argument(&call.args, &method)?;
                }
                "require_handle_traits" => {
                    rule.required_handle_traits = string_list_argument(&call.args, &method)?;
                }
                "require_part_traits" => {
                    rule.required_part_traits = string_list_argument(&call.args, &method)?;
                }
                other => {
                    return Err(FaceParseError::new(format!(
                        "unknown registration-rule clause `.{other}(…)`;                          the reader only knows the `require_*` calls the renderer emits"
                    )));
                }
            }
            Ok(rule)
        }
        _ => Err(FaceParseError::new(format!(
            "`{RULE_CONST}` is not a `RegistrationRule` builder expression"
        ))),
    }
}

/// Whether `func` is the `RegistrationRule::new` call that starts a builder chain.
/// `func` 是否是启动构建链的 `RegistrationRule::new` 调用。
fn is_builder_start(func: &syn::Expr) -> bool {
    matches!(func, syn::Expr::Path(path)
        if path.path.segments.last().is_some_and(|segment| segment.ident == "new"))
}

/// The single string literal argument of `.{method}(…)`.
/// `.{method}(…)` 的唯一字符串字面量参数。
fn single_string_argument(
    arguments: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
    method: &str,
) -> Result<String, FaceParseError> {
    let mut arguments = arguments.iter();
    let Some(syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(value),
        ..
    })) = arguments.next()
    else {
        return Err(FaceParseError::new(format!(
            "`.{method}(…)` needs one string literal argument"
        )));
    };
    if arguments.next().is_some() {
        return Err(FaceParseError::new(format!(
            "`.{method}(…)` takes exactly one argument"
        )));
    }
    Ok(value.value())
}

/// The string-literal list argument of `.{method}(…)`, written as `&["…"]` or `["…"]`.
/// `.{method}(…)` 的字符串字面量列表参数，写作 `&["…"]` 或 `["…"]`。
fn string_list_argument(
    arguments: &syn::punctuated::Punctuated<syn::Expr, syn::Token![,]>,
    method: &str,
) -> Result<Vec<String>, FaceParseError> {
    let mut arguments = arguments.iter();
    let Some(argument) = arguments.next() else {
        return Err(FaceParseError::new(format!(
            "`.{method}(…)` needs one list argument"
        )));
    };
    if arguments.next().is_some() {
        return Err(FaceParseError::new(format!(
            "`.{method}(…)` takes exactly one argument"
        )));
    }
    let array = match argument {
        syn::Expr::Array(array) => array,
        syn::Expr::Reference(reference) => match reference.expr.as_ref() {
            syn::Expr::Array(array) => array,
            _ => {
                return Err(FaceParseError::new(format!(
                    "`.{method}(…)` needs a list of string literals"
                )));
            }
        },
        _ => {
            return Err(FaceParseError::new(format!(
                "`.{method}(…)` needs a list of string literals"
            )));
        }
    };
    array
        .elems
        .iter()
        .map(|element| match element {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(value),
                ..
            }) => Ok(value.value()),
            _ => Err(FaceParseError::new(format!(
                "`.{method}(…)` needs string literals in its list"
            ))),
        })
        .collect()
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod rules_tests;

/// Reduce a registry-rule source text to its compact syntax form, tolerantly.
/// 以宽容的方式将注册规则源码文本化简为紧凑语法形式。
///
/// Every spelling is spelled by [`compact_registration_rule`] and by nothing else:
/// this branch only reads the markers out of the source text and hands the value to
/// the one renderer, so the source form and the owned form cannot drift apart.
///
/// **This entry is lossy and tolerant, and it is kept because it is published API**
/// (`-> String` in the released `0.1.x` line): it reads the *whole file text*, so a
/// clause inside a comment is read as a real one, and it degrades silently when the
/// first `=` it finds is not the `REGISTRATION_RULE` initializer. Code that must not
/// misread a hand-written rule calls [`try_rule_syntax_from_text`], which reads the
/// const's initializer through the AST and refuses anything it cannot recognize; that
/// is what the workspace's own call sites use (audit `KRN-K-10`).
///
/// 每一种拼法都只由 [`compact_registration_rule`] 拼出：本分支只从源码文本里读出标记，然后把
/// 取值交给那唯一的渲染器，因此源码形式与拥有型形式不会分叉。
pub fn rule_syntax_from_text(text: &str) -> String {
    let expression = text
        .split_once('=')
        .map(|(_, value)| value.trim().trim_end_matches(';'))
        .unwrap_or("");
    if expression.contains("RegistrationRule::ANY") {
        return compact_registration_rule(&rule_requiring_nothing());
    }
    compact_registration_rule(&OwnedRegistrationRule {
        required_preset: quoted_value_field(expression, ".require_preset("),
        required_parts: quoted_list_field(expression, ".require_parts("),
        required_exports: quoted_list_field(expression, ".require_exports("),
        required_handle_traits: quoted_list_field(expression, ".require_handle_traits("),
        required_part_traits: quoted_list_field(expression, ".require_part_traits("),
    })
}

/// Render the compact registration-rule syntax as a const Rust expression.
/// 将紧凑注册规范语法渲染成 const Rust 表达式。
pub fn render_registration_rule(value: &str) -> Result<String, FaceParseError> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("any") {
        return Ok("crate::RegistrationRule::ANY".to_owned());
    }
    let rule = parse_registration_rule_owned(value)?;
    let render_values = |values: &[String]| {
        values
            .iter()
            .map(|value| format!("\"{}\"", rust_string(value)))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut expression = "crate::RegistrationRule::new()".to_owned();
    if let Some(preset) = rule.required_preset {
        expression.push_str(&format!(".require_preset(\"{}\")", rust_string(&preset)));
    }
    for (method, values) in [
        ("require_parts", &rule.required_parts),
        ("require_exports", &rule.required_exports),
        ("require_handle_traits", &rule.required_handle_traits),
        ("require_part_traits", &rule.required_part_traits),
    ] {
        if !values.is_empty() {
            expression.push_str(&format!(".{method}(&[{}])", render_values(values)));
        }
    }
    Ok(expression)
}

/// Render `capability=>provider` pairs as a Rust `requires:` list, dropping any entry that does
/// not have that shape.
/// 把 `capability=>provider` 对渲染成 Rust 的 `requires:` 列表，形状不符的条目一律丢掉。
///
/// **This entry is lossy, and it stays that way because it is published API**: it is
/// `-> String` in the released `0.1.x` line, so a malformed entry disappears from the rendered
/// list instead of failing the rewrite, and a caller that writes the result back into a file
/// would delete a field the author wrote. Code that has to *refuse* a wrong `requires` list calls
/// [`try_render_requirements`], which is what the manifest renderer uses (audit `LGC-LG-28`).
/// **本入口是有损的，而且因为它是已发布的 API 而保持这样**：在已发布的 `0.1.x` 线上它是
/// `-> String`，因此畸形条目会从渲染结果里消失，而不是让重写失败；把结果写回文件的调用方会就此删掉
/// 作者写下的一个字段。需要**拒绝**错误 `requires` 列表的代码请调用 [`try_render_requirements`]——
/// manifest 渲染器用的就是它（审计 `LGC-LG-28`）。
pub fn render_requirements(value: &str) -> String {
    value
        .split(',')
        .filter_map(|item| requirement_item(item).ok())
        .map(rendered_requirement)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Render a `requires` list strictly: every entry has to be well-formed, or the render fails.
/// 严格渲染 `requires` 列表：每条条目都必须合规，否则渲染失败。
///
/// This is the entry the manifest renderer uses, because dropping an entry is not a rendering
/// detail — it is a deletion. `render_source` writes its result back into the author's file, so an
/// entry the lossy path skipped would be gone from the declaration after one edit; refusing is what
/// keeps the editor's contract ("only rewrite a declaration it can reproduce in full") true for
/// this field too (audit `LGC-LG-28`). The published [`render_requirements`] keeps its lossy shape
/// for callers already compiled against it.
/// 这是 manifest 渲染器使用的入口，因为丢掉一条条目不是渲染细节——它是一次删除。`render_source`
/// 会把结果写回作者的文件，因此有损路径跳过的条目会在一次编辑之后从声明里消失；拒绝才是让编辑器那句
/// 契约（"只重写自己能完整复现的声明"）对这个字段同样成立的做法（审计 `LGC-LG-28`）。已发布的
/// [`render_requirements`] 为已编译的调用方保留其有损形状。
pub fn try_render_requirements(value: &str) -> Result<String, FaceParseError> {
    value
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(|item| requirement_item(item).map(rendered_requirement))
        .collect::<Result<Vec<_>, _>>()
        .map(|rows| rows.join(", "))
}

/// One parsed entry as the declaration writes it.
/// 一条已解析条目在声明里的写法。
fn rendered_requirement(requirement: OwnedRequirementSpec) -> String {
    format!(
        "\"{}\" => \"{}\"",
        rust_string(&requirement.capability),
        rust_string(&requirement.provider)
    )
}

/// Parse `capability=>provider` pairs into their owned snapshot form, dropping any entry that
/// does not have that shape.
/// 将 `capability=>provider` 对解析为快照使用的拥有型形式，形状不符的条目一律丢掉。
///
/// **This entry is lossy, and it stays that way because it is published API**: it is
/// `-> Vec<…>` in the released `0.1.x` line, so a malformed entry is skipped and the well-formed
/// rest is returned — it does not report an error, and callers must not read a short list as
/// proof that the source was well-formed. Code that has to *refuse* a wrong `requires` list calls
/// [`try_parse_requirements_owned`], which is what the kernel itself uses (audit `LGC-LG-28`).
/// **本入口是有损的，而且因为它是已发布的 API 而保持这样**：在已发布的 `0.1.x` 线上它是
/// `-> Vec<…>`，因此畸形条目会被跳过、合规的其余条目照常返回——它不会报错，调用方也不得把
/// 一份更短的列表当作"源文件合规"的证明。需要**拒绝**错误 `requires` 列表的代码请调用
/// [`try_parse_requirements_owned`]——内核自己用的就是它（审计 `LGC-LG-28`）。
pub fn parse_requirements_owned(value: &str) -> Vec<OwnedRequirementSpec> {
    value
        .split(',')
        .filter_map(|item| requirement_item(item).ok())
        .collect()
}

/// Read a `requires` list strictly: every entry has to be well-formed, or the read fails.
/// 严格读取 `requires` 列表：每条条目都必须合规，否则读取失败。
///
/// This is the entry the kernel itself uses, because dropping an input is not a syntax error —
/// it is a missing check. A `requires` list feeds connector admission, so an entry that never
/// reaches the connector is a dependency the author believes they declared and nobody enforces;
/// a hand-written or older file whose list is misspelled used to lose it silently (audit
/// `LGC-LG-28`). The published [`parse_requirements_owned`] keeps its lossy shape for callers
/// already compiled against it; new code that has to refuse a wrong list calls this one.
/// 这是内核自己使用的入口，因为丢掉一项输入不是语法错误，而是少了一道检查。`requires` 列表是连接器
/// 准入的输入，因此一条从未抵达连接器的条目就是作者以为声明了、却无人执行的依赖；一份手写或旧版本
/// 写下的、列表拼错的文件过去会静默失去它（审计 `LGC-LG-28`）。已发布的
/// [`parse_requirements_owned`] 为已编译的调用方保留其有损形状；需要拒绝错误列表的新代码调用本入口。
pub fn try_parse_requirements_owned(
    value: &str,
) -> Result<Vec<OwnedRequirementSpec>, FaceParseError> {
    value
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(requirement_item)
        .collect()
}

/// One `requires` entry, or the reason it is not one.
/// 一条 `requires` 条目，或者它不成其为条目的原因。
///
/// The per-entry rule is written once and read by both entry points: the strict reader turns its
/// `Err` into a refusal, the published reader drops that entry and keeps the rest. They used to
/// be two separate implementations — the validator refused a malformed entry while the parser
/// dropped it — which is how a misspelled `requires` list lost a requirement silently (audit
/// `LGC-LG-28`).
/// 逐条规则只写一次、由两个入口共读：严格读取器把它的 `Err` 变成一次拒绝，而已发布的读取器丢掉那条
/// 条目并保留其余。它们过去是两份各自实现——校验器拒绝畸形条目，解析器把它丢掉——一个拼错的
/// `requires` 列表就是这样静默丢掉一条要求的（审计 `LGC-LG-28`）。
fn requirement_item(item: &str) -> Result<OwnedRequirementSpec, FaceParseError> {
    let item = item.trim();
    let (capability, provider) = item.split_once("=>").ok_or_else(|| {
        FaceParseError::from("requires entries must use capability=>provider syntax".to_owned())
    })?;
    if capability.trim().is_empty() || provider.trim().is_empty() {
        return Err("requires entries cannot be empty".to_owned().into());
    }
    Ok(OwnedRequirementSpec {
        capability: capability.trim().to_owned(),
        provider: provider.trim().to_owned(),
    })
}

/// Check that every `requires` entry uses the `capability=>provider` shape.
/// 校验每条 `requires` 条目都使用 `capability=>provider` 形状。
pub fn parse_requirements(value: &str) -> Result<(), FaceParseError> {
    try_parse_requirements_owned(value).map(|_| ())
}

/// Parse the compact registration-rule value into an owned rule.
/// 将紧凑注册规范值解析为拥有型规则。
pub fn parse_registration_rule_owned(value: &str) -> Result<OwnedRegistrationRule, FaceParseError> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("any") {
        return Ok(rule_requiring_nothing());
    }
    let mut rule = rule_requiring_nothing();
    for clause in value
        .split(';')
        .map(str::trim)
        .filter(|clause| !clause.is_empty())
    {
        let (key, body) = clause
            .split_once(':')
            .ok_or_else(|| "registration_rule clauses must use key:value syntax".to_owned())?;
        let values = split_csv_owned(body);
        if values.is_empty() && key.trim() != "preset" {
            return Err("registration_rule clauses must list at least one value"
                .to_owned()
                .into());
        }
        match key.trim().to_ascii_lowercase().as_str() {
            "parts" => rule.required_parts.extend(values),
            "exports" => rule.required_exports.extend(values),
            "handle" | "handle_trait" | "handle_traits" => {
                rule.required_handle_traits.extend(values)
            }
            "part_trait" | "part_traits" => rule.required_part_traits.extend(values),
            "preset" if body.trim().is_empty() => {
                return Err("registration_rule preset cannot be empty".to_owned().into());
            }
            "preset" => rule.required_preset = Some(body.trim().to_owned()),
            key => return Err(format!("unknown registration_rule clause `{key}`").into()),
        }
    }
    Ok(rule)
}

#[cfg(test)]
mod tests {
    use super::{parse_registration_rule_owned, render_registration_rule};

    #[test]
    fn registration_rules_only_describe_minimum_structure() {
        let rule = parse_registration_rule_owned(
            "preset:ActionParts;parts:paint;exports:control.render;handle:ControlHandle;part_trait:ActionParts",
        )
        .unwrap();
        assert_eq!(rule.required_preset.as_deref(), Some("ActionParts"));
        assert_eq!(rule.required_parts, ["paint"]);
        assert_eq!(rule.required_exports, ["control.render"]);
        assert_eq!(rule.required_handle_traits, ["ControlHandle"]);
        assert_eq!(rule.required_part_traits, ["ActionParts"]);

        let source = render_registration_rule(
            "preset:ActionParts;parts:paint;exports:control.render;handle:ControlHandle;part_trait:ActionParts",
        )
        .unwrap();
        assert!(source.contains("RegistrationRule::new()"));
        assert!(source.contains("require_parts(&[\"paint\"])"));
        assert!(!source.contains("allow"));
    }

    #[test]
    fn kind_filters_are_not_registration_rules() {
        let error = parse_registration_rule_owned("allow:Button").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("unknown registration_rule clause `allow`"),
            "{error}"
        );
    }
}
