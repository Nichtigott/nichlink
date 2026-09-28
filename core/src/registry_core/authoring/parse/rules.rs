//! Registration-rule field rendering and parsing for authored faces.
//! 注册面 registration_rule 字段的渲染与解析。
//!
//! A rule is written as a compact `preset:…;parts:…;exports:…` list and held as
//! a `RegistrationRule` builder expression. An empty or `ANY` value means "no
//! structural requirement", which is not the same as a rule that rejects
//! everything.
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

/// Reduce a registry-rule source text to its compact syntax form.
/// 将注册规则源码文本化简为紧凑语法形式。
///
/// Every spelling is spelled by [`compact_registration_rule`] and by nothing else:
/// this branch only reads the markers out of the source text and hands the value to
/// the one renderer, so the source form and the owned form cannot drift apart.
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

/// Render `capability=>provider` pairs as a Rust `requires:` list.
/// 把 `capability=>provider` 对渲染成 Rust 的 `requires:` 列表。
pub fn render_requirements(value: &str) -> String {
    value
        .split(',')
        .filter_map(|item| {
            let (capability, provider) = item.split_once("=>")?;
            Some(format!(
                "\"{}\" => \"{}\"",
                rust_string(capability.trim()),
                rust_string(provider.trim())
            ))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Parse `capability=>provider` pairs into their owned snapshot form.
/// 将 `capability=>provider` 对解析为快照使用的拥有型形式。
pub fn parse_requirements_owned(value: &str) -> Vec<OwnedRequirementSpec> {
    value
        .split(',')
        .filter_map(|item| {
            let (capability, provider) = item.split_once("=>")?;
            Some(OwnedRequirementSpec {
                capability: capability.trim().to_owned(),
                provider: provider.trim().to_owned(),
            })
        })
        .collect()
}

/// Check that every `requires` entry uses the `capability=>provider` shape.
/// 校验每条 `requires` 条目都使用 `capability=>provider` 形状。
pub fn parse_requirements(value: &str) -> Result<(), FaceParseError> {
    if value.trim().is_empty() {
        return Ok(());
    }
    for item in value.split(',') {
        let (capability, provider) = item
            .split_once("=>")
            .ok_or_else(|| "requires entries must use capability=>provider syntax".to_owned())?;
        if capability.trim().is_empty() || provider.trim().is_empty() {
            return Err("requires entries cannot be empty".to_owned().into());
        }
    }
    Ok(())
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
