//! Parsing and rendering helpers for authored registration faces.
//! 注册面创作文件的解析与渲染辅助函数。

use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    parse_face as parse_face_syntax, OwnedAdmission, OwnedFlowContract, OwnedRegistrationRule,
    OwnedRequirementSpec, RuntimeCheckSpec,
};

use super::validation::{normalized_path, rust_string, source_root};

pub(super) fn module_source_from_node_path(module: &str) -> Option<String> {
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

/// Read the parent's declared `kind` from its attached source file.
/// 从父注册面的附属源文件读取它声明的 `kind`。
pub(super) fn kind_from_source_path(source: &str) -> Option<String> {
    let path = source_root().join(source);
    let text = fs::read_to_string(path).ok()?;
    parse_face_syntax(&text)
        .ok()?
        .and_then(|face| face.path("kind"))
}

pub(super) fn quoted_list_field(text: &str, marker: &str) -> Vec<String> {
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

pub(super) fn source_path_from_file(path: &Path) -> String {
    if let Ok(relative) = path.strip_prefix(source_root()) {
        return normalized_path(relative);
    }
    let roots = ["compile_error_demo", "control", "engine", "trimmed_core"];
    let mut components = path.components();
    while let Some(component) = components.next() {
        let text = component.as_os_str().to_string_lossy();
        if roots.contains(&text.as_ref()) {
            let mut result = PathBuf::from(text.as_ref());
            result.extend(components.map(|part| part.as_os_str()));
            return normalized_path(&result);
        }
    }
    normalized_path(path)
}

pub(super) fn module_name_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("module")
        .to_owned()
}

pub(super) fn parse_admission_expression(expression: &str) -> Result<String, String> {
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

pub(super) fn rule_syntax_for_source(path: &Path) -> Result<String, String> {
    let Some(parent) = path.parent() else {
        return Ok("ANY".to_owned());
    };
    let canonical = parent.join("registry_rule/registry_rule.rs");
    let legacy = parent.join("registry/rules/rules.rs");
    let Ok(text) = fs::read_to_string(&canonical).or_else(|_| fs::read_to_string(&legacy)) else {
        return Ok("ANY".to_owned());
    };
    let expression = text
        .lines()
        .find_map(|line| {
            line.split_once('=')
                .map(|(_, value)| value.trim().trim_end_matches(';'))
        })
        .unwrap_or("");
    if expression.contains("RegistrationRule::ANY") {
        return Ok("ANY".to_owned());
    }
    let values = quoted_list_field(expression, "RegistrationRule::new(");
    if !values.is_empty() {
        return Ok(format!("allow:{}", values.join(",")));
    }
    Ok("ANY".to_owned())
}

/// Render the compact registration-rule syntax as a const Rust expression.
/// 将紧凑注册规范语法渲染成 const Rust 表达式。
pub(super) fn render_registration_rule(value: &str) -> Result<String, String> {
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
    let simple = rule.required_preset.is_none()
        && rule.required_parts.is_empty()
        && rule.required_exports.is_empty()
        && rule.required_handle_traits.is_empty()
        && rule.required_part_traits.is_empty();
    if simple {
        if rule.allowed_kinds.is_empty() && rule.denied_kinds.is_empty() {
            return Ok("crate::RegistrationRule::ANY".to_owned());
        }
        if rule.denied_kinds.is_empty() {
            return Ok(format!(
                "crate::RegistrationRule::new(&[{}], &[])",
                render_values(&rule.allowed_kinds)
            ));
        }
        if rule.allowed_kinds.is_empty() {
            return Ok(format!(
                "crate::RegistrationRule::new(&[], &[{}])",
                render_values(&rule.denied_kinds)
            ));
        }
    }
    let preset = rule
        .required_preset
        .as_deref()
        .map(|value| format!("Some(\"{}\")", rust_string(value)))
        .unwrap_or_else(|| "None".to_owned());
    Ok(format!(
        "crate::RegistrationRule::new_with_contract(&[{}], &[{}], {}, &[{}], &[{}], &[{}], &[{}])",
        render_values(&rule.allowed_kinds),
        render_values(&rule.denied_kinds),
        preset,
        render_values(&rule.required_parts),
        render_values(&rule.required_exports),
        render_values(&rule.required_handle_traits),
        render_values(&rule.required_part_traits),
    ))
}

pub(super) fn render_face_list(field: &str, value: &str) -> String {
    let values = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("\"{}\"", rust_string(value)))
        .collect::<Vec<_>>();
    if values.is_empty() {
        String::new()
    } else {
        format!("             \x20   {field}: [{}],\n", values.join(", "))
    }
}

pub(super) fn render_literal_list(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| format!("\"{}\"", rust_string(item)))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn render_path_list(value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn render_impls(kind: &str, value: &str) -> String {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|trait_path| format!("impl {trait_path} for {kind} {{}}\n"))
        .collect()
}

pub(super) fn render_requirements(value: &str) -> String {
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

pub(super) fn render_expression_list(value: &str) -> Result<String, String> {
    RuntimeCheckSpec::parse_list(value).map(|checks| {
        checks
            .into_iter()
            .map(RuntimeCheckSpec::expression)
            .collect::<Vec<_>>()
            .join(", ")
    })
}

pub(super) fn replace_string_field(source: &mut String, field: &str, value: &str) {
    let marker = format!("{field}: \"");
    let Some(start) = source.find(&marker) else {
        return;
    };
    let value_start = start + marker.len();
    let Some(end) = source[value_start..].find('"') else {
        return;
    };
    source.replace_range(value_start..value_start + end, &rust_string(value));
}

/// Render the editor's `id|version|input|output` form as a Rust expression.
/// 将编辑器中的 `id|version|input|output` 形式渲染为 Rust 表达式。
pub(super) fn render_flow_expression(value: &str) -> Result<String, String> {
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
pub(super) fn parse_flow_value(value: &str) -> Result<Option<OwnedFlowContract>, String> {
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
pub(super) fn render_flow_provider(value: &str) -> Result<String, String> {
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
pub(super) fn parse_flow_expression(value: &str) -> Result<String, String> {
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

pub(super) fn path_ends_with(expression: &syn::Expr, suffix: &str) -> bool {
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

pub(super) fn literal_string_expr(expression: &syn::Expr) -> Result<String, String> {
    let syn::Expr::Lit(literal) = expression else {
        return Err("generated face expects a string literal".to_owned());
    };
    let syn::Lit::Str(value) = &literal.lit else {
        return Err("generated face expects a string literal".to_owned());
    };
    Ok(value.value())
}

pub(super) fn render_optional_source(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        Ok("None".to_owned())
    } else {
        Ok(format!("Some(\"{}\")", rust_string(value)))
    }
}

pub(super) fn split_csv_owned(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_owned)
        .collect()
}

pub(super) fn parse_requirements_owned(value: &str) -> Vec<OwnedRequirementSpec> {
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

pub(super) fn parse_requirements(value: &str) -> Result<(), String> {
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

pub(super) fn parse_optional_source(value: &str) -> Result<(), String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("none") {
        return Ok(());
    }
    if value.contains(['\n', '\r']) {
        return Err("getting_from_other_registry cannot contain a newline".to_owned());
    }
    Ok(())
}

pub(super) fn parse_registration_rule_owned(value: &str) -> Result<OwnedRegistrationRule, String> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("any") {
        return Ok(OwnedRegistrationRule {
            allowed_kinds: Vec::new(),
            denied_kinds: Vec::new(),
            required_preset: None,
            required_parts: Vec::new(),
            required_exports: Vec::new(),
            required_handle_traits: Vec::new(),
            required_part_traits: Vec::new(),
        });
    }
    let mut rule = OwnedRegistrationRule {
        allowed_kinds: Vec::new(),
        denied_kinds: Vec::new(),
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
            "allow" => rule.allowed_kinds.extend(values),
            "deny" => rule.denied_kinds.extend(values),
            "parts" => rule.required_parts.extend(values),
            "exports" => rule.required_exports.extend(values),
            "handle" | "handle_trait" | "handle_traits" => {
                rule.required_handle_traits.extend(values)
            }
            "part_trait" | "part_traits" => rule.required_part_traits.extend(values),
            "preset" if body.trim().is_empty() => {
                return Err("registration_rule preset cannot be empty".to_owned())
            }
            "preset" => rule.required_preset = Some(body.trim().to_owned()),
            key => return Err(format!("unknown registration_rule clause `{key}`")),
        }
    }
    Ok(rule)
}

pub(super) fn parse_admission_owned(value: &str) -> Result<OwnedAdmission, String> {
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

pub(super) fn render_admission(value: &str) -> Result<String, String> {
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
