//! Face field editing and registry-rule derivation.
//! 注册面字段编辑与注册规范推导。
//!
//! Rendering lives in `render.rs`; this file keeps the edit and rule half.
//! 渲染位于 `render.rs`；本文件保留编辑与规范部分。

use std::path::{Path, PathBuf};

use super::super::FaceManifest;
use crate::RuntimeCheckSpec;
use crate::authoring::parse::*;
use crate::authoring::validation::{
    legacy_rule_path_for_source, rule_path_for_source, source_root, validate_kind_name,
    validate_name,
};

#[path = "render.rs"]
mod render;

impl FaceManifest {
    pub(crate) fn rule_source_path(&self) -> Result<PathBuf, String> {
        let source = Path::new(self.values.get("source").map(String::as_str).unwrap_or(""));
        let directory = source
            .parent()
            .ok_or_else(|| "generated face has no module directory".to_owned())?;
        if let Some(declared) = self.values.get("registry_rule_path")
            && !declared.trim().is_empty()
        {
            let path = Path::new(declared);
            let relative = path.strip_prefix("src/").unwrap_or(path);
            return Ok(source_root().join(relative));
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

    pub(crate) fn rule_source_path_string(&self) -> Result<String, String> {
        if let Some(declared) = self.values.get("registry_rule_path")
            && !declared.trim().is_empty()
        {
            return Ok(declared.clone());
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
        Ok(Some(format!(
            "//! Registry rule.\n//! 注册规范。\n\nuse crate::RegistrationRule;\n\npub const REGISTRATION_RULE: RegistrationRule = {rule};\n"
        )))
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
            | "part_contracts"
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

    /// Whether this face owns a generated registry-rule source file.
    /// 该注册面是否拥有生成的注册规则源文件。
    ///
    /// A face that declares a rule of its own needs one even when it owns no
    /// child registry. The two callers used to disagree — the renderer emitted
    /// `registry_rule: <module>::REGISTRATION_RULE` for a custom rule while the
    /// editor wrote the module only for a registry owner — so saving a leaf face
    /// with a custom rule produced source that referenced a file nobody wrote.
    /// 声明了自己规则的注册面即使不拥有子注册机也需要它。两个调用方过去口径不一致——
    /// 渲染器会为自定义规则发射 `registry_rule: <模块>::REGISTRATION_RULE`，而编辑器
    /// 只为拥有注册机的面写该模块——于是保存一个带自定义规则的叶子面会生成引用不存在
    /// 文件的源码。
    pub(crate) fn owns_rule_source(&self) -> bool {
        let rule = self
            .values
            .get("registration_rule")
            .map_or("", String::as_str)
            .trim();
        self.values.get("needs_registry").map(String::as_str) == Some("true")
            || (!rule.is_empty() && rule != "ANY")
    }

    /// Whether `registry_rule:` is redundant because the canonical sibling rule
    /// derives it.
    /// `registry_rule:` 是否因为同目录规范规则已经推导出它而多余。
    ///
    /// Only a face that owns a registry derives the field: the resolver defaults
    /// every other face to the permissive rule, which is what a child face wants,
    /// since the rule that governs it belongs to its parent. A face whose rule was
    /// moved elsewhere keeps naming it.
    /// 只有拥有注册机的面才推导该字段：解析器把其余面默认成宽松规则——这正是子面的需要，
    /// 因为管它的规则属于它的父级。规则被挪到别处的面仍然写出它。
    pub(crate) fn derives_rule_from_the_sibling(&self) -> bool {
        if self.values.get("needs_registry").map(String::as_str) != Some("true") {
            return false;
        }
        let source = self.values.get("source").map(String::as_str).unwrap_or("");
        let declared = self
            .values
            .get("registry_rule_path")
            .map_or("", String::as_str)
            .trim();
        declared.is_empty() || declared == rule_path_for_source(source)
    }
}

#[cfg(test)]
mod rule_source_ownership_tests {
    use super::super::FaceManifest;
    use std::collections::BTreeMap;

    /// The renderer and the editor must agree on when a face owns a generated
    /// rule source: a custom rule needs one even for a leaf face, and a face that
    /// owns a registry needs one even with `ANY`. They used to disagree, so
    /// saving a leaf face with a custom rule wrote source referencing a module
    /// nobody created.
    /// 渲染器与编辑器必须对"何时拥有生成的规则源"口径一致：自定义规则即使在叶子面上也
    /// 需要它，而拥有注册机的面即使规则是 `ANY` 也需要它。两者过去不一致，于是保存一个
    /// 带自定义规则的叶子面会写出引用不存在模块的源码。
    #[test]
    fn a_custom_rule_makes_a_leaf_face_own_its_rule_source() {
        let manifest = |needs_registry: &str, rule: &str| {
            let mut manifest = FaceManifest {
                values: BTreeMap::new(),
            };
            manifest
                .values
                .insert("needs_registry".to_owned(), needs_registry.to_owned());
            manifest
                .values
                .insert("registration_rule".to_owned(), rule.to_owned());
            manifest
        };

        assert!(!manifest("false", "ANY").owns_rule_source());
        assert!(manifest("false", "parts:paint").owns_rule_source());
        assert!(manifest("true", "ANY").owns_rule_source());
    }

    /// A face that owns a registry and keeps its rule at the canonical path does
    /// not repeat that path to reference the rule: the declaration resolves it
    /// from the sibling module instead. A leaf face, or one whose rule lives
    /// elsewhere, still names it.
    /// 拥有注册机、且规则就在规范路径上的面不必为了引用规则而重复该路径：声明改为从同目录
    /// 模块推导它。叶子面、或规则放在别处的面仍然写出它。
    #[test]
    fn a_registry_face_derives_the_rule_it_keeps_beside_it() {
        let manifest = |needs_registry: &str, declared_path: &str| {
            let mut manifest = FaceManifest {
                values: BTreeMap::new(),
            };
            // `source` is relative to the package `src` directory, which is the
            // form the manifest keeps and `rule_path_for_source` consumes.
            // `source` 相对包的 `src` 目录，这正是清单保存、`rule_path_for_source`
            // 消费的形式。
            manifest
                .values
                .insert("source".to_owned(), "widget/widget.rs".to_owned());
            manifest
                .values
                .insert("needs_registry".to_owned(), needs_registry.to_owned());
            manifest
                .values
                .insert("registration_rule".to_owned(), "parts:paint".to_owned());
            if !declared_path.is_empty() {
                manifest
                    .values
                    .insert("registry_rule_path".to_owned(), declared_path.to_owned());
            }
            manifest
        };

        // Canonical path, whether it is written out or left to the default.
        assert!(manifest("true", "").derives_rule_from_the_sibling());
        assert!(
            manifest("true", "src/widget/registry_rule/registry_rule.rs")
                .derives_rule_from_the_sibling()
        );
        // A rule that was moved elsewhere keeps its explicit reference.
        assert!(!manifest("true", "src/widget/other/rules.rs").derives_rule_from_the_sibling());
        // A face that owns no registry defaults to the permissive rule, so it
        // cannot derive the sibling without changing what it enforces.
        assert!(!manifest("false", "").derives_rule_from_the_sibling());
        assert!(
            !manifest("false", "src/widget/registry_rule/registry_rule.rs")
                .derives_rule_from_the_sibling()
        );
    }
}
