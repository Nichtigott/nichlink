//! Face editing and source rendering.
//! 注册面编辑与源码渲染。

use std::path::{Path, PathBuf};

use super::super::FaceManifest;
use crate::RuntimeCheckSpec;
use crate::authoring::GENERATED_MARKER;
use crate::authoring::parse::*;
use crate::authoring::validation::{
    legacy_rule_path_for_source, normalized_path, rule_path_for_source, rust_string, source_root,
    validate_kind_name, validate_name,
};

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

    #[allow(dead_code)]
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

    pub(crate) fn render_source(&self) -> Result<String, String> {
        // Rebuilding the declaration would drop `plugin:`: the manifest layer
        // reads it, but nothing here can render its expression back, and the
        // snapshot models no plugin value. Refusing loudly beats silently
        // deleting a field the author wrote — the editor can only rewrite a
        // declaration it can reproduce in full.
        // 重建声明会丢掉 `plugin:`：manifest 层读得到它，但这里无法把它的表达式渲染
        // 回去，快照也不建模插件值。响亮拒绝胜过静默删掉作者写的字段——编辑器只应重写
        // 自己能完整复现的声明。
        let value = |key| self.values.get(key).map(String::as_str).unwrap_or("");
        if !value("plugin").is_empty() {
            return Err(
                "this face declares `plugin:`, which the editor cannot rewrite yet; \
                 edit that line in the file by hand"
                    .to_owned(),
            );
        }
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
        let registration_rule = if self.owns_rule_source() {
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
            format!("crate::{}::NODE_ID", module.replace('/', "::"))
        };
        // The declaration macro names the registry that owns this face. The
        // generated aliases are emitted by the build crate from the folder
        // tree; `__nichlink_object!` remains their single hidden implementation.
        // 注册声明的宏名表达当前注册面所属的父注册机。别名由 build crate
        // 根据文件夹树生成，`__nichlink_object!` 仍是唯一隐藏实现。
        let object_macro = if value("parent_source") == "<root>" {
            "root_object".to_owned()
        } else {
            let source = Path::new(value("parent_source"));
            let module = source
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("registry");
            format!("{module}_object")
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
            format!("    handle_contracts: [{handle_contracts}],\n")
        };
        let handle_impls = render_impls(kind, value("handle_contracts"));
        let part_traits = render_face_list("part_traits", value("part_traits"));
        let part_contracts = render_path_list(value("part_contracts"));
        let part_contracts_decl = if part_contracts.is_empty() {
            String::new()
        } else {
            format!("    part_contracts: [{part_contracts}],\n")
        };
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
        // Keep the parent visible even for root faces. The macro still has a
        // root fallback for hand-written declarations, but generated faces
        // should show their registration target explicitly.
        // 即使父级是 root 也保留 parent 字段。手写声明仍可使用宏的 root
        // 默认值，但生成注册面应明确展示自己的挂载目标。
        let parent_decl = format!("    parent: {parent},\n");
        let registry_decl = if registry_name == value("module") {
            String::new()
        } else {
            format!("    registry_name: {registry_name},\n")
        };
        let canonical_rule_path = rule_path_for_source(value("source"));
        let registry_fields = if self.owns_rule_source() {
            let canonical = registry_rule_path == canonical_rule_path;
            let rule_path = if canonical {
                String::new()
            } else {
                format!(
                    "    registry_rule_path: \"{}\",\n",
                    rust_string(&registry_rule_path)
                )
            };
            // A face that owns a registry derives its rule from the canonical
            // module beside it, so writing the full path back would repeat the
            // face's own location in every declaration. A leaf face with a custom
            // rule still names it: the default for a face that owns no registry is
            // permissive, not the sibling rule.
            // 拥有注册机的面从旁边的规范模块推导规则，因此把完整路径写回去等于在每份声明里
            // 重复注册面自己的位置。带自定义规则的叶子面仍需写出它：不拥有注册机的面默认是
            // 宽松规则，而不是同目录规则。
            let derived = canonical && self.derives_rule_from_the_sibling();
            let rule = if derived {
                String::new()
            } else {
                format!("    registry_rule: {registration_rule},\n")
            };
            format!("{rule_path}{rule}")
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
            "{module_doc}\n\nuse crate::{{NoParts, NoPreset}};\n\n{handle_doc}\npub struct {kind};\n\n{handle_impls}crate::{object_macro}! {{\n    kind: {kind},\n{preset_decl}{parts_decl}{name_decl}{summary_decl}{params_decl}{exports_decl}{handle_decl}{stable_decl}{needs_decl}{registry_decl}{parent_decl}{getting_decl}{registry_fields}{admission_decl}{handle_traits}{handle_contracts_decl}{part_traits}{part_contracts_decl}{requirements_decl}{provides_decl}{output_decl}{flow}{flow_provider}{runtime_decl}}}\n"
        );
        Ok(format!("{GENERATED_MARKER}\n{source}"))
    }
}

fn is_default_type(value: &str, default: &str) -> bool {
    let value = value.trim();
    value == default || value.rsplit("::").next() == Some(default)
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

#[cfg(test)]
mod plugin_preservation_tests {
    use crate::authoring::manifest::parse;

    /// Rebuilding a declaration must never delete a field it cannot reproduce:
    /// a face carrying `plugin:` refuses the rewrite instead of losing it.
    /// 重建声明绝不能删掉自己无法复现的字段：带 `plugin:` 的面拒绝重写，而不是把它
    /// 弄丢。
    #[test]
    fn a_face_with_a_plugin_refuses_a_silent_rewrite() {
        let root = std::env::temp_dir().join("nichlink-plugin-face-fixture");
        let face = root.join("widget/widget.rs");
        std::fs::create_dir_all(face.parent().expect("fixture parent")).expect("fixture dir");
        std::fs::write(
            &face,
            "// generated-by=NichLink\n\
             crate::root_object! {\n\
                 kind: Widget,\n\
                 parent: crate::ROOT_NODE_ID,\n\
                 plugin: crate::PluginSpec::new(\"widget\"),\n\
             }\n",
        )
        .expect("fixture face");

        let manifest = parse::source(&face).expect("face manifest");
        let error = manifest
            .render_source()
            .expect_err("a plugin field must refuse the rewrite")
            .to_owned();
        assert!(error.contains("plugin:"), "{error}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
