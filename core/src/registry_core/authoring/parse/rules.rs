//! Registration-rule field rendering and parsing for authored faces.
//! 注册面 registration_rule 字段的渲染与解析。
//!
//! A rule is written as a compact `preset:…;parts:…;exports:…` list and held as
//! a `RegistrationRule` builder expression. An empty or `ANY` value means "no
//! structural requirement", which is not the same as a rule that rejects
//! everything.
//! 规则写成紧凑的 `preset:…;parts:…;exports:…` 列表，并以 `RegistrationRule`
//! 构建表达式保存。空值或 `ANY` 表示"没有结构要求"，这与拒绝一切的规则不同。

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

/// Reduce a registry-rule source text to its compact syntax form.
/// 将注册规则源码文本化简为紧凑语法形式。
pub fn rule_syntax_from_text(text: &str) -> String {
    let expression = text
        .split_once('=')
        .map(|(_, value)| value.trim().trim_end_matches(';'))
        .unwrap_or("");
    if expression.contains("RegistrationRule::ANY") {
        return "ANY".to_owned();
    }
    let mut clauses = Vec::new();
    if let Some(preset) = quoted_value_field(expression, ".require_preset(") {
        clauses.push(format!("preset:{preset}"));
    }
    for (marker, key) in [
        (".require_parts(", "parts"),
        (".require_exports(", "exports"),
        (".require_handle_traits(", "handle"),
        (".require_part_traits(", "part_trait"),
    ] {
        let values = quoted_list_field(expression, marker);
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
        return Ok(OwnedRegistrationRule {
            required_preset: None,
            required_parts: Vec::new(),
            required_exports: Vec::new(),
            required_handle_traits: Vec::new(),
            required_part_traits: Vec::new(),
        });
    }
    let mut rule = OwnedRegistrationRule {
        required_preset: None,
        required_parts: Vec::new(),
        required_exports: Vec::new(),
        required_handle_traits: Vec::new(),
        required_part_traits: Vec::new(),
    };
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
