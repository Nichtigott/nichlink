//! Source rendering for a registration face.
//! 注册面的源码渲染。

use std::path::Path;

use super::super::FaceManifest;
use crate::authoring::GENERATED_MARKER;
use crate::authoring::context::{normalized_path, rule_path_for_source, rust_string};
use crate::authoring::parse::*;

/// On-disk keys: the fields this renderer writes back into a face file — the whole
/// on-disk field set, listed once. The name says which of the two classes this list is,
/// so a maintainer can tell the writable set from the rest without reading the body.
/// 落盘键：本渲染器会写回注册面文件的字段——全部落盘字段，只列一次。名字本身说明了这份清单属于
/// 两类里的哪一类，因此维护者不必读正文就能把"会落盘的集合"与其余键分开。
///
/// `FaceManifest::values` carries three classes of key and marks none of them
/// (audit `LGC-LG-38`), so this list is the one place a maintainer can read what
/// actually lands on disk:
/// - the fields below: read here and re-emitted;
/// - keys derived from the file's own location, which are inputs to that
///   emission rather than fields — `source`, `parent_source`, `registry_rule_path`
///   and the `namespace`/`parent_node`/`parent_kind`/`provided_parts`/
///   `required_parts` metadata;
/// - keys no template emits — `registry_name`, `handle`, `params`, which `edit`
///   refuses by name instead of storing an edit nothing would write.
///
/// The round-trip pin below takes this list as the set of rows it must cover and
/// asserts the two are equal, so the renderer and its evidence cannot drift into
/// two half-lists.
/// `FaceManifest::values` 混装三类键且都不加标记（审计 `LGC-LG-38`），因此这份清单是维护者能读到
/// "真正落盘的是什么"的唯一位置：
/// - 下面这些字段：从这里读出并重新发射；
/// - 由文件自身位置派生的键，它们是那次发射的**输入**而不是字段——`source`、`parent_source`、
///   `registry_rule_path`，以及 `namespace`/`parent_node`/`parent_kind`/`provided_parts`/
///   `required_parts` 这些元数据；
/// - 任何模板都不发射的键——`registry_name`、`handle`、`params`，`edit` 会按名拒绝它们，而不是
///   存下一次没有任何东西会写出的编辑。
///
/// 下面的往返钉子把这份清单当作它必须覆盖的行集合，并断言两者相等，因此渲染器与它的证据不可能各自
/// 漂成半份清单。
const ON_DISK_FIELDS: &[&str] = &[
    "kind",
    "preset",
    "parts",
    "name_zh",
    "name_en",
    "summary_zh",
    "summary_en",
    "exports",
    "stable_name",
    "getting_from_other_registry",
    "requires",
    "provides",
    "runtime_checks",
    "flow",
    "flow_provider",
    "handle_traits",
    "handle_contracts",
    "part_traits",
    "part_contracts",
    "needs_registry",
    "registration_rule",
    "admission",
];

/// Off-disk keys: the keys `values` carries that are not on-disk fields — inputs, derived
/// metadata, and the keys no template emits. The name states the class, mirroring
/// [`ON_DISK_FIELDS`]; nothing here lands on a file, and a key that no template emits is
/// refused by name instead of being stored.
/// 非落盘键：`values` 里并非落盘字段的键——输入、派生元数据，以及任何模板都不发射的键。名字写出
/// 了类别，与 [`ON_DISK_FIELDS`] 相对；这里没有任何东西会落盘，而任何模板都不发射的键会被按名
/// 拒绝，而不是被存下来。
///
/// They are listed beside [`ON_DISK_FIELDS`] so the partition is stated in one
/// place, and the pin checks that none of them slipped into the rendered set.
/// 它们与 [`ON_DISK_FIELDS`] 并列，好让这份划分只在一处陈述；钉子会核对它们没有混进渲染集合。
const OFF_DISK_KEYS: &[&str] = &[
    "admission_line",
    "declaration_line",
    "handle",
    "module",
    "namespace",
    "params",
    "parent_kind",
    "parent_node",
    "parent_registry_name",
    "parent_source",
    "plugin",
    "provided_parts",
    "registry_name",
    "registry_rule_path",
    "required_parts",
    "root",
    "source",
];

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
        // Every key a manifest may carry is one of two lists (the constants at the
        // top of this file): a field this renderer writes, or an input/metadata key.
        // A third kind is a field nothing here can reproduce, and the refusal this
        // file already makes for `plugin:` is the same refusal that keeps such a key
        // from being ignored — the editor may only rewrite a declaration it can
        // reproduce in full.
        // 清单可能携带的每个键都属于两份清单之一（本文件顶部的常量）：本渲染器会写出的字段，或者
        // 输入/元数据键。第三种键是这里无法复现的字段，而本文件对 `plugin:` 已经做出的那次拒绝，正是
        // 让这样的键不会被忽略的同一道拒绝——编辑器只应重写自己能完整复现的声明。
        if let Some(unknown) = self.values.keys().find(|key| {
            !ON_DISK_FIELDS.contains(&key.as_str()) && !OFF_DISK_KEYS.contains(&key.as_str())
        }) {
            return Err(format!(
                "this face carries `{unknown}`, which no template emits and the editor cannot \
                 rewrite; edit that line in the file by hand"
            ));
        }
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
        // The strict entry, because this result is written back into the author's
        // file: an entry the lossy published path would drop has to refuse the
        // rewrite instead of deleting a field the author wrote.
        // 严格入口：这个结果会被写回作者的文件，因此有损入口会丢掉的条目必须让这次重写失败，而不是
        // 删掉作者写下的一个字段。
        let requirements =
            try_render_requirements(value("requires")).map_err(|error| error.to_string())?;
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
#[path = "render_tests.rs"]
mod render_tests;
