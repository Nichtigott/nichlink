//! Face editing and source rendering.
//! 注册面编辑与源码渲染。

use std::path::{Path, PathBuf};

use super::super::FaceManifest;
use crate::registry_core::authoring::parse::*;
use crate::registry_core::authoring::validation::{
    legacy_rule_path_for_source, normalized_path, rule_path_for_source, rust_string, source_root,
    validate_kind_name, validate_name,
};
use crate::registry_core::authoring::GENERATED_MARKER;
use crate::{OwnedRegistrationRule, RuntimeCheckSpec};

impl FaceManifest {
    pub(crate) fn rule_source_path(&self) -> Result<PathBuf, String> {
        let source = Path::new(self.values.get("source").map(String::as_str).unwrap_or(""));
        let directory = source
            .parent()
            .ok_or_else(|| "generated face has no module directory".to_owned())?;
        if let Some(declared) = self.values.get("registry_rule_path") {
            if !declared.trim().is_empty() {
                let path = Path::new(declared);
                let relative = path.strip_prefix("src/").unwrap_or(path);
                return Ok(source_root().join(relative));
            }
        }
        let canonical = source_root()
            .join(directory)
            .join("registry_rule/registry_rule.rs");
        if canonical.is_file() {
            return Ok(canonical);
        }
        // Existing faces authored before the dedicated folder remain valid.
        // 兼容早期使用 registry/rules/rules.rs 的注册面。
        Ok(source_root().join(
            Path::new(&legacy_rule_path_for_source(
                self.values.get("source").map(String::as_str).unwrap_or(""),
            ))
            .strip_prefix("src/")
            .unwrap_or_else(|_| Path::new("")),
        ))
    }

    #[allow(dead_code)]
    pub(crate) fn rule_source_path_string(&self) -> Result<String, String> {
        if let Some(declared) = self.values.get("registry_rule_path") {
            if !declared.trim().is_empty() {
                return Ok(declared.clone());
            }
        }
        Ok(rule_path_for_source(
            self.values.get("source").map(String::as_str).unwrap_or(""),
        ))
    }

    pub(crate) fn rule_module_path(&self) -> Result<String, String> {
        let rule_path = self.rule_source_path_string()?;
        let module = rule_path
            .strip_prefix("src/")
            .unwrap_or(&rule_path)
            .rsplit_once('/')
            .map_or_else(|| rule_path.as_str(), |(directory, _)| directory)
            .replace('/', "::");
        Ok(format!("crate::{module}::REGISTRATION_RULE"))
    }

    pub(crate) fn render_rule_source(&self) -> Result<Option<String>, String> {
        if self.values.get("needs_registry").map(String::as_str) != Some("true") {
            return Ok(None);
        }
        let rule = render_registration_rule(
            self.values
                .get("registration_rule")
                .map(String::as_str)
                .unwrap_or("ANY"),
        )?;
        Ok(Some(format!("//! Registry rule.\n//! 注册规范。\n\nuse crate::RegistrationRule;\n\npub const REGISTRATION_RULE: RegistrationRule = {rule};\n")))
    }

    pub(crate) fn inherit_registry_contract(&mut self, rule: &OwnedRegistrationRule) {
        self.values.insert(
            "handle_traits".to_owned(),
            rule.required_handle_traits.join(","),
        );
        self.values.insert(
            "part_traits".to_owned(),
            rule.required_part_traits.join(","),
        );
        let contracts = rule
            .required_handle_traits
            .iter()
            .filter_map(|trait_name| match trait_name.as_str() {
                "ControlHandle" => Some("crate::control::ControlHandle"),
                _ => None,
            })
            .collect::<Vec<_>>();
        self.values
            .insert("handle_contracts".to_owned(), contracts.join(","));
    }

    pub(crate) fn edit(&mut self, field: &str, value: &str) -> Result<(), String> {
        match field {
            "kind"
            | "preset"
            | "parts"
            | "handle"
            | "name_zh"
            | "name_en"
            | "summary_zh"
            | "summary_en"
            | "params"
            | "stable_name"
            | "exports"
            | "registry_name"
            | "getting_from_other_registry"
            | "registration_rule"
            | "admission"
            | "handle_traits"
            | "handle_contracts"
            | "part_traits"
            | "requires"
            | "provides"
            | "expected_output"
            | "actual_output"
            | "runtime_checks"
            | "flow"
            | "flow_provider" => {
                if value.contains(['\n', '\r']) {
                    return Err("field value cannot contain a newline".to_owned());
                }
                if field == "registry_name" {
                    validate_name(value)?;
                }
                if field == "kind" {
                    validate_kind_name(value)?;
                }
                if field == "registration_rule" {
                    parse_registration_rule_owned(value)?;
                }
                if field == "admission" {
                    parse_admission_owned(value)?;
                }
                if field == "getting_from_other_registry" {
                    parse_optional_source(value)?;
                }
                if field == "requires" {
                    parse_requirements(value)?;
                }
                if field == "runtime_checks" {
                    RuntimeCheckSpec::parse_list(value)?;
                }
                if field == "flow" {
                    parse_flow_value(value)?;
                }
                if field == "flow_provider" && !value.trim().is_empty() {
                    syn::parse_str::<syn::Path>(value)
                        .map_err(|_| "flow_provider must be a Rust type path".to_owned())?;
                }
                let key = if field == "admission" {
                    "admission"
                } else {
                    field
                };
                self.values.insert(key.to_owned(), value.to_owned());
                Ok(())
            }
            "needs_registry" if matches!(value, "true" | "false") => {
                self.values.insert(field.to_owned(), value.to_owned());
                Ok(())
            }
            "needs_registry" => Err("needs_registry must be `true` or `false`".to_owned()),
            _ => Err(format!(
                "field `{field}` is not editable; use the registration face fields"
            )),
        }
    }

    pub(crate) fn render_source(&self) -> Result<String, String> {
        let value = |key| self.values.get(key).map(String::as_str).unwrap_or("");
        let registry_name = if value("registry_name").is_empty() {
            value("module")
        } else {
            value("registry_name")
        };
        let kind = value("kind");
        let preset = if value("preset").is_empty() {
            "NoPreset"
        } else {
            value("preset")
        };
        let parts = if value("parts").is_empty() {
            "NoParts"
        } else {
            value("parts")
        };
        let handle = if value("handle").is_empty() {
            value("kind")
        } else {
            value("handle")
        };
        let module_doc = if value("needs_registry") == "true" {
            format!(
                "//! {kind} face and its recursively requested registry.\n//! {kind} 注册面及其递归申请的注册机。"
            )
        } else {
            format!("//! {kind} registration face.\n//! {kind} 注册面。")
        };
        let handle_doc = format!(
            "/// Registration-only handle marker for the {kind} face.\n/// 仅用于 {kind} 注册面的 handle 标记，不代表运行时 object 实现。"
        );
        let has_custom_rule = !value("registration_rule").trim().is_empty()
            && value("registration_rule").trim() != "ANY";
        let registration_rule = if value("needs_registry") == "true" || has_custom_rule {
            self.rule_module_path()?
        } else {
            "crate::RegistrationRule::ANY".to_owned()
        };
        let parent = if value("parent_source") == "<root>" {
            "crate::root_node_id(env!(\"CARGO_PKG_NAME\"))".to_owned()
        } else {
            let source = Path::new(value("parent_source"));
            let module = source
                .parent()
                .map_or_else(|| source.to_string_lossy().into_owned(), normalized_path);
            format!("crate::{module}::NODE_ID")
        };
        let admission = render_admission(value("admission"))?;
        let exports = value("exports")
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| format!("\"{}\"", rust_string(item)))
            .collect::<Vec<_>>()
            .join(", ");
        let exports_decl = if exports.is_empty() {
            String::new()
        } else {
            format!("    exports: [{exports}],\n")
        };
        let handle_traits = render_face_list("handle_traits", value("handle_traits"));
        let handle_contracts = render_path_list(value("handle_contracts"));
        let handle_contracts_decl = if handle_contracts.is_empty() {
            String::new()
        } else {
            format!("             \x20   handle_contracts: [{handle_contracts}],\n")
        };
        let handle_impls = render_impls(kind, value("handle_contracts"));
        let part_traits = render_face_list("part_traits", value("part_traits"));
        let requirements = render_requirements(value("requires"));
        let provides = render_literal_list(value("provides"));
        let runtime_checks = render_expression_list(value("runtime_checks"))?;
        let flow = render_flow_expression(value("flow"))?;
        let flow_provider = render_flow_provider(value("flow_provider"))?;
        let getting = render_optional_source(value("getting_from_other_registry"))?;
        let registry_rule_path = if value("registry_rule_path").is_empty() {
            rule_path_for_source(value("source"))
        } else {
            value("registry_rule_path").to_owned()
        };
        let stable_decl = if value("stable_name").is_empty() {
            String::new()
        } else {
            format!(
                "\x20   stable_name: \"{}\",\n",
                rust_string(value("stable_name"))
            )
        };
        let name_zh = if value("name_zh").is_empty() {
            kind
        } else {
            value("name_zh")
        };
        let name_en = if value("name_en").is_empty() {
            kind
        } else {
            value("name_en")
        };
        let name_decl = if name_zh == kind && name_en == kind {
            String::new()
        } else {
            format!(
                "    name: {{ zh: \"{}\", en: \"{}\" }},\n",
                rust_string(name_zh),
                rust_string(name_en)
            )
        };
        let summary_decl = if value("summary_zh").is_empty() && value("summary_en").is_empty() {
            String::new()
        } else {
            format!(
                "    summary: {{ zh: \"{}\", en: \"{}\" }},\n",
                rust_string(value("summary_zh")),
                rust_string(value("summary_en"))
            )
        };
        // Keep structural type markers explicit; prose and identity fields may
        // use defaults without losing the contract surface.
        // 保留结构类型标记；说明文字和身份字段可以使用默认值。
        let preset_decl = if !is_default_type(preset, "NoPreset") {
            format!("    preset: {preset},\n")
        } else {
            String::new()
        };
        let parts_decl = if !is_default_type(parts, "NoParts") {
            format!("    parts: {parts},\n")
        } else {
            String::new()
        };
        let custom_shape =
            !is_default_type(preset, "NoPreset") || !is_default_type(parts, "NoParts");
        let preset_decl = if custom_shape && preset_decl.is_empty() {
            format!("    preset: {preset},\n")
        } else {
            preset_decl
        };
        let parts_decl = if custom_shape && parts_decl.is_empty() {
            format!("    parts: {parts},\n")
        } else {
            parts_decl
        };
        let params_decl = if !value("params").is_empty() && value("params") != kind {
            format!("    params: \"{}\",\n", rust_string(value("params")))
        } else {
            String::new()
        };
        let handle_decl = if handle != kind || custom_shape {
            format!("    handle: {handle},\n")
        } else {
            String::new()
        };
        let needs_decl = if value("needs_registry") == "true" {
            "    needs_registry: true,\n".to_owned()
        } else {
            String::new()
        };
        let parent_decl = if value("parent_source") == "<root>" {
            String::new()
        } else {
            format!("    parent: {parent},\n")
        };
        let registry_decl = if registry_name == value("module") {
            String::new()
        } else {
            format!("    registry_name: {registry_name},\n")
        };
        let canonical_rule_path = rule_path_for_source(value("source"));
        let registry_fields = if value("needs_registry") == "true" || has_custom_rule {
            let rule_path = if registry_rule_path == canonical_rule_path {
                String::new()
            } else {
                format!(
                    "    registry_rule_path: \"{}\",\n",
                    rust_string(&registry_rule_path)
                )
            };
            format!("{rule_path}    registry_rule: {registration_rule},\n")
        } else {
            String::new()
        };
        let getting_decl = if !getting.is_empty() && getting != "None" {
            format!("    getting_from_other_registry: {getting},\n")
        } else {
            String::new()
        };
        let admission_decl = if value("admission").is_empty() || value("admission") == "ANY" {
            String::new()
        } else {
            format!("    admission: {admission},\n")
        };
        let requirements_decl = if !requirements.is_empty() {
            format!("    requires: [{requirements}],\n")
        } else {
            String::new()
        };
        let provides_decl = if !provides.is_empty() {
            format!("    provides: [{provides}],\n")
        } else {
            String::new()
        };
        let output_decl = if value("expected_output") != "()" || value("actual_output") != "()" {
            format!(
                "    expected_output: \"{}\",\n    actual_output: \"{}\",\n",
                rust_string(value("expected_output")),
                rust_string(value("actual_output"))
            )
        } else {
            String::new()
        };
        let runtime_decl = if !runtime_checks.is_empty() {
            format!("    runtime_checks: [{runtime_checks}],\n")
        } else {
            String::new()
        };
        let source = format!(
            "{module_doc}\n\n\
             use crate::{{NoParts, NoPreset}};\n\n\
             {handle_doc}\n\
             pub struct {kind};\n\n\
             {handle_impls}\
             crate::control_object! {{\n\
                 kind: {kind},\n\
             {preset_decl}{parts_decl}{name_decl}{summary_decl}{params_decl}\
                 {exports_decl}\
             {handle_decl}\
             {stable_decl}\
             {needs_decl}{registry_decl}{parent_decl}{getting_decl}{registry_fields}{admission_decl}\
             {handle_traits}\
             {handle_contracts_decl}\
             {part_traits}\
             {requirements_decl}{provides_decl}{output_decl}{flow}{flow_provider}{runtime_decl}\
             }}\n",
            kind = kind,
            module_doc = module_doc,
            handle_doc = handle_doc,
            preset_decl = preset_decl,
            parts_decl = parts_decl,
            name_decl = name_decl,
            summary_decl = summary_decl,
            params_decl = params_decl,
            exports_decl = exports_decl,
            handle_decl = handle_decl,
            handle_contracts_decl = handle_contracts_decl,
            handle_impls = handle_impls,
            stable_decl = stable_decl,
            needs_decl = needs_decl,
            registry_fields = registry_fields,
            getting_decl = getting_decl,
            admission_decl = admission_decl,
            handle_traits = handle_traits,
            part_traits = part_traits,
            requirements_decl = requirements_decl,
            provides_decl = provides_decl,
            output_decl = output_decl,
            runtime_decl = runtime_decl,
            flow = flow,
            flow_provider = flow_provider,
            parent_decl = parent_decl,
        );
        Ok(format!("{GENERATED_MARKER}\n{source}"))
    }
}

fn is_default_type(value: &str, default: &str) -> bool {
    let value = value.trim();
    value == default || value.rsplit("::").next() == Some(default)
}
