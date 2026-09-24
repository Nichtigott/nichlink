//! Source rendering for a registration face.
//! 注册面的源码渲染。

use std::path::Path;

use super::super::FaceManifest;
use crate::authoring::GENERATED_MARKER;
use crate::authoring::parse::*;
use crate::authoring::validation::{normalized_path, rule_path_for_source, rust_string};

impl FaceManifest {
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
        if !value(nichlink::lexicon::FACE_FIELD_PLUGIN).is_empty() {
            return Err(
                "this face declares `plugin:`, which the editor cannot rewrite yet; \
                 edit that line in the file by hand"
                    .to_owned(),
            );
        }
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
        let module_doc = if value("needs_registry") == "true" {
            format!(
                "//! {kind} face and its recursively requested registry.\n//! {kind} 注册面及其递归申请的注册机。"
            )
        } else {
            format!("//! {kind} registration face.\n//! {kind} 注册面。")
        };
        let handle_doc = format!(
            "/// Registration-only marker for the {kind} face.\n/// 仅用于 {kind} 注册面的 handle 标记，不代表运行时 object 实现。"
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
        let runtime_decl = if !runtime_checks.is_empty() {
            format!("    runtime_checks: [{runtime_checks}],\n")
        } else {
            String::new()
        };
        let source = format!(
            "{module_doc}\n\nuse crate::{{NoParts, NoPreset}};\n\n{handle_doc}\npub struct {kind};\n\ncrate::{object_macro}! {{\n    kind: {kind},\n{preset_decl}{parts_decl}{name_decl}{summary_decl}{exports_decl}{stable_decl}{needs_decl}{parent_decl}{getting_decl}{registry_fields}{admission_decl}{handle_traits}{handle_contracts_decl}{part_traits}{part_contracts_decl}{requirements_decl}{provides_decl}{flow}{flow_provider}{runtime_decl}}}\n"
        );
        Ok(format!("{GENERATED_MARKER}\n{source}"))
    }
}

fn is_default_type(value: &str, default: &str) -> bool {
    let value = value.trim();
    value == default || value.rsplit("::").next() == Some(default)
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
