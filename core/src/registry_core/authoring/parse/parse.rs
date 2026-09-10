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

use std::path::PathBuf;

use crate::{
    OwnedAdmission, OwnedFlowContract, OwnedRegistrationRule, OwnedRequirementSpec,
    RuntimeCheckSpec, parse_face as parse_face_syntax,
};

use super::validation::{normalized_path, rust_string};

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

fn quoted_value_field(text: &str, marker: &str) -> Option<String> {
    let rest = text.split_once(marker)?.1;
    let start = rest.find('"')? + 1;
    let end = rest[start..].find('"')? + start;
    Some(rest[start..end].to_owned())
}

pub fn module_name_from_path(path: &std::path::Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("module")
        .to_owned()
}

pub fn parse_admission_expression(expression: &str) -> Result<String, String> {
    let expression = expression.trim().trim_end_matches(',');
    if expression.is_empty() || expression.contains("Admission::ANY") {
        return Ok("ANY".to_owned());
    }
    let paths = quoted_list_field(expression, "allow_paths(");
    if !paths.is_empty() {
        return Ok(format!("allow:{}", paths.join(",")));
    }
    let paths = quoted_list_field(expression, "deny_paths(");
    if !paths.is_empty() {
        return Ok(format!("deny:{}", paths.join(",")));
    }
    Err("generated face has an invalid admission expression".to_owned())
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
pub fn render_registration_rule(value: &str) -> Result<String, String> {
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

pub fn render_literal_list(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| format!("\"{}\"", rust_string(item)))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn render_path_list(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn render_impls(kind: &str, value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|trait_path| format!("impl {trait_path} for {kind} {{}}\n"))
        .collect()
}

/// Derive the human-facing trait labels from compiler-checked Rust paths.
/// 从参与编译检查的 Rust 路径派生人类可读的 trait 名称。
pub fn trait_names_from_paths(value: &str) -> Result<String, String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(|source| {
            let path = syn::parse_str::<syn::Path>(source)
                .map_err(|_| format!("`{source}` is not a Rust trait path"))?;
            path.segments
                .last()
                .map(|segment| segment.ident.to_string())
                .ok_or_else(|| format!("`{source}` has no trait name"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|names| names.join(","))
}

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

pub fn render_expression_list(value: &str) -> Result<String, String> {
    RuntimeCheckSpec::parse_list(value).map(|checks| {
        checks
            .into_iter()
            .map(RuntimeCheckSpec::expression)
            .collect::<Vec<_>>()
            .join(", ")
    })
}

/// Render the editor's `id|version|input|output` form as a Rust expression.
/// 将编辑器中的 `id|version|input|output` 形式渲染为 Rust 表达式。
pub fn render_flow_expression(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return Ok(String::new());
    }
    let mut fields = value.split('|');
    let id = fields.next().unwrap_or_default().trim();
    let version = fields.next().unwrap_or_default().trim();
    let input = fields.next().unwrap_or_default().trim();
    let output = fields.next().unwrap_or_default().trim();
    if fields.next().is_some() || id.is_empty() || input.is_empty() || output.is_empty() {
        return Err("flow must use id|version|input|output syntax".to_owned());
    }
    let version = version
        .parse::<u32>()
        .map_err(|_| "flow version must be an unsigned integer".to_owned())?;
    Ok(format!(
        "    flow: crate::FlowContract::new(crate::ContractId::new(\"{}\"), {version}, \"{}\", \"{}\"),\n",
        rust_string(id),
        rust_string(input),
        rust_string(output),
    ))
}

/// Parse the compact flow value into an owned contract for a reload snapshot.
/// 将紧凑 flow 值解析为热重载快照使用的拥有型合同。
pub fn parse_flow_value(value: &str) -> Result<Option<OwnedFlowContract>, String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return Ok(None);
    }
    let mut fields = value.split('|');
    let id = fields.next().unwrap_or_default().trim();
    let version = fields.next().unwrap_or_default().trim();
    let input = fields.next().unwrap_or_default().trim();
    let output = fields.next().unwrap_or_default().trim();
    if fields.next().is_some() || id.is_empty() || input.is_empty() || output.is_empty() {
        return Err("flow must use id|version|input|output syntax".to_owned());
    }
    let version = version
        .parse::<u32>()
        .map_err(|_| "flow version must be an unsigned integer".to_owned())?;
    Ok(Some(OwnedFlowContract {
        id: id.to_owned(),
        version,
        input: input.to_owned(),
        output: output.to_owned(),
    }))
}

/// Render an explicitly selected flow provider type.
/// 渲染显式选择的数据流合同提供者类型。
pub fn render_flow_provider(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    syn::parse_str::<syn::Path>(value)
        .map_err(|_| "flow_provider must be a Rust type path".to_owned())?;
    Ok(format!("    flow_provider: {value},\n"))
}

/// Parse a flow expression back into the editor's compact form.
/// 将 flow 表达式解析回编辑器使用的紧凑形式。
pub fn parse_flow_expression(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.ends_with("FlowContract::NONE") {
        return Ok(String::new());
    }
    let expression = syn::parse_str::<syn::Expr>(value)
        .map_err(|_| "generated face has an invalid flow expression".to_owned())?;
    let syn::Expr::Call(call) = expression else {
        return Err("generated face has an invalid flow expression".to_owned());
    };
    if !path_ends_with(&call.func, "FlowContract::new") || call.args.len() != 4 {
        return Err("generated face has an invalid flow expression".to_owned());
    }
    let mut args = call.args.iter();
    let id_call = args
        .next()
        .ok_or_else(|| "generated face has an invalid flow id".to_owned())?;
    let syn::Expr::Call(id_call) = id_call else {
        return Err("generated face has an invalid flow id".to_owned());
    };
    if !path_ends_with(&id_call.func, "ContractId::new") || id_call.args.len() != 1 {
        return Err("generated face has an invalid flow id".to_owned());
    }
    let id = literal_string_expr(
        id_call
            .args
            .first()
            .ok_or_else(|| "generated face has an invalid flow id".to_owned())?,
    )?;
    let version = match args
        .next()
        .ok_or_else(|| "generated face has an invalid flow version".to_owned())?
    {
        syn::Expr::Lit(literal) => match &literal.lit {
            syn::Lit::Int(value) => value
                .base10_parse::<u32>()
                .map_err(|_| "generated face has an invalid flow version".to_owned())?,
            _ => return Err("generated face has an invalid flow version".to_owned()),
        },
        _ => return Err("generated face has an invalid flow version".to_owned()),
    };
    let input = literal_string_expr(
        args.next()
            .ok_or_else(|| "generated face has an invalid flow input".to_owned())?,
    )?;
    let output = literal_string_expr(
        args.next()
            .ok_or_else(|| "generated face has an invalid flow output".to_owned())?,
    )?;
    Ok(format!("{id}|{version}|{input}|{output}"))
}

pub fn path_ends_with(expression: &syn::Expr, suffix: &str) -> bool {
    let syn::Expr::Path(path) = expression else {
        return false;
    };
    let actual = path
        .path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::");
    actual.ends_with(suffix)
}

pub fn literal_string_expr(expression: &syn::Expr) -> Result<String, String> {
    let syn::Expr::Lit(literal) = expression else {
        return Err("generated face expects a string literal".to_owned());
    };
    let syn::Lit::Str(value) = &literal.lit else {
        return Err("generated face expects a string literal".to_owned());
    };
    Ok(value.value())
}

pub fn render_optional_source(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        Ok("None".to_owned())
    } else {
        Ok(format!("Some(\"{}\")", rust_string(value)))
    }
}

pub fn split_csv_owned(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

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

pub fn parse_requirements(value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Ok(());
    }
    for item in value.split(',') {
        let (capability, provider) = item
            .split_once("=>")
            .ok_or_else(|| "requires entries must use capability=>provider syntax".to_owned())?;
        if capability.trim().is_empty() || provider.trim().is_empty() {
            return Err("requires entries cannot be empty".to_owned());
        }
    }
    Ok(())
}

pub fn parse_optional_source(value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return Ok(());
    }
    if value.contains(['\n', '\r']) {
        return Err("getting_from_other_registry cannot contain a newline".to_owned());
    }
    Ok(())
}

pub fn parse_registration_rule_owned(value: &str) -> Result<OwnedRegistrationRule, String> {
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
            return Err("registration_rule clauses must list at least one value".to_owned());
        }
        match key.trim().to_ascii_lowercase().as_str() {
            "parts" => rule.required_parts.extend(values),
            "exports" => rule.required_exports.extend(values),
            "handle" | "handle_trait" | "handle_traits" => {
                rule.required_handle_traits.extend(values)
            }
            "part_trait" | "part_traits" => rule.required_part_traits.extend(values),
            "preset" if body.trim().is_empty() => {
                return Err("registration_rule preset cannot be empty".to_owned());
            }
            "preset" => rule.required_preset = Some(body.trim().to_owned()),
            key => return Err(format!("unknown registration_rule clause `{key}`")),
        }
    }
    Ok(rule)
}

pub fn parse_admission_owned(value: &str) -> Result<OwnedAdmission, String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("any") {
        return Ok(OwnedAdmission {
            allowed_paths: Vec::new(),
            denied_paths: Vec::new(),
        });
    }
    let (mode, paths) = value.split_once(':').ok_or_else(|| {
        "admission must be ANY, allow:path/prefix, or deny:path/prefix".to_owned()
    })?;
    let paths = split_csv_owned(paths);
    if paths.is_empty() {
        return Err("admission must list at least one path".to_owned());
    }
    match mode.trim().to_ascii_lowercase().as_str() {
        "allow" => Ok(OwnedAdmission {
            allowed_paths: paths,
            denied_paths: Vec::new(),
        }),
        "deny" => Ok(OwnedAdmission {
            allowed_paths: Vec::new(),
            denied_paths: paths,
        }),
        _ => Err("admission must be ANY, allow:path/prefix, or deny:path/prefix".to_owned()),
    }
}

pub fn render_admission(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("any") {
        return Ok("crate::Admission::ANY".to_owned());
    }
    let (mode, kinds) = value.split_once(':').ok_or_else(|| {
        "admission must be ANY, allow:path/prefix, or deny:path/prefix".to_owned()
    })?;
    let rendered = kinds
        .split(',')
        .map(str::trim)
        .filter(|kind| !kind.is_empty())
        .map(|kind| format!("\"{}\"", rust_string(kind)))
        .collect::<Vec<_>>();
    if rendered.is_empty() {
        return Err("admission must list at least one path".to_owned());
    }
    match mode.to_ascii_lowercase().as_str() {
        "allow" => Ok(format!(
            "crate::Admission::new(&[{}], &[])",
            rendered.join(", ")
        )),
        "deny" => Ok(format!(
            "crate::Admission::new(&[], &[{}])",
            rendered.join(", ")
        )),
        _ => Err("admission must be ANY, allow:path/prefix, or deny:path/prefix".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_registration_rule_owned, render_registration_rule, trait_names_from_paths};

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
        assert!(error.contains("unknown registration_rule clause `allow`"));
    }

    #[test]
    fn compiler_checked_trait_paths_supply_searchable_labels() {
        assert_eq!(
            trait_names_from_paths("crate::ui::ControlHandle, crate::parts::ActionParts").unwrap(),
            "ControlHandle,ActionParts"
        );
        assert!(trait_names_from_paths("not a path").is_err());
    }
}
