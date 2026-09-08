// File-backed authoring for NichLink registration faces.
// NichLink 注册面的文件化创作支持。

use std::collections::BTreeMap;
#[allow(unused_imports)]
use std::fs::{self, OpenOptions};
#[allow(unused_imports)]
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(unused_imports)]
use crate::{
    parse_face as parse_face_syntax, NodeId, OwnedAdmission, OwnedFlowContract,
    OwnedLocalizedText, OwnedObjectContract, OwnedRegistrationRule, OwnedRequirementSpec,
    OwnedSourceLocation, ParentSyntax, RegistrationSnapshot, Registry, RuntimeCheckSpec,
    ROOT_NODE_ID,
};

use super::filesystem::{atomic_write, create_new};
#[allow(unused_imports)]
use super::parse::{
    kind_from_source_path, module_name_from_path, module_source_from_node_path,
    parse_admission_expression, parse_admission_owned, parse_flow_expression, parse_flow_value,
    parse_optional_source, parse_registration_rule_owned, parse_requirements,
    parse_requirements_owned, render_admission, render_expression_list, render_face_list,
    render_flow_expression, render_flow_provider, render_impls, render_literal_list,
    render_optional_source, render_path_list, render_registration_rule, render_requirements,
    rule_syntax_for_source, source_path_from_file, split_csv_owned,
};
#[allow(unused_imports)]
use super::validation::{
    authoring_namespace, is_parent_component, normalize_kind_name, normalized_path, package_root,
    rule_path_for_source, rust_string, rust_type_name, source_root, validate_kind_name,
    validate_name,
};
pub use super::validation::AuthoringContext;
use self::manifest::FaceManifest;

pub use self::operations::{
    add_module, add_module_from_face, add_module_with_registration, delete_module, edit_module,
    edit_module_face, generated_snapshots, generated_snapshots_from, AuthoringChange,
    ModuleFacePatch, NewModuleFace,
};
pub use self::external_graft::{ExternalGraftPlanFile, create_external_graft};

const GENERATED_MARKER: &str = "// generated-by=NichLink";

/// Field order shared by the Studio form and the file authoring API.
/// Studio 表单与文件创作 API 共用的字段顺序。
///
/// The field order mirrors every field accepted by a parent-specific object macro.
/// Keeping one shared order prevents Add/Edit from silently dropping metadata.
/// 字段顺序覆盖父级专属 object 宏的全部可编辑字段；统一顺序可避免
/// Add/Edit 静默丢失注册面信息。
pub const FACE_FIELD_NAMES: [&str; 29] = [
    "parent",
    "module",
    "needs registry",
    "registry name",
    "registration rule",
    "admission",
    "parts",
    "exports",
    "kind",
    "name zh",
    "name en",
    "summary zh",
    "summary en",
    "preset",
    "params",
    "handle",
    "stable name",
    "other registry",
    "rule path",
    "handle traits",
    "handle contracts",
    "part traits",
    "requires",
    "provides",
    "expected output",
    "actual output",
    "runtime checks",
    "flow",
    "flow provider",
];

pub const FACE_FIELD_COUNT: usize = FACE_FIELD_NAMES.len();

/// Fields needed to create a useful face. The remaining fields are advanced
/// contracts and are inherited or derived until explicitly changed.
/// 创建可用注册面所需的字段；其余是高级合同，默认继承或推导。
pub const FACE_PRIMARY_FIELDS: &[usize] = &[0, 1, 2, 8, 9, 10, 11, 12, 7, 23];

/// A source-tree mutation that needs one rebuild before it becomes executable.
/// 一次源码树变更；它需要经过一次重建才会成为可执行注册面。
impl FaceManifest {
    pub(crate) fn new(
        name: &str,
        kind: &str,
        parent: NodeId,
        parent_source: &str,
        parent_kind: &str,
        source: &str,
    ) -> Self {
        let mut values = BTreeMap::new();
        let namespace = authoring_namespace();
        for (key, value) in [
            ("namespace", namespace.as_str()),
            ("module", name),
            ("registry_name", name),
            ("kind", kind),
            ("preset", "NoPreset"),
            ("parts", "NoParts"),
            ("handle", kind),
            ("registration_rule", "ANY"),
            ("admission", "ANY"),
            ("source", source),
            ("parent_node", &parent.to_string()),
            ("parent_source", parent_source),
            ("parent_kind", parent_kind),
            ("name_zh", ""),
            ("name_en", ""),
            ("summary_zh", ""),
            ("summary_en", ""),
            ("params", ""),
            ("stable_name", ""),
            ("exports", ""),
            ("provides", ""),
            ("needs_registry", "false"),
            ("getting_from_other_registry", ""),
            ("handle_traits", ""),
            ("handle_contracts", ""),
            ("part_traits", ""),
            ("part_contracts", ""),
            ("requires", ""),
            ("expected_output", "()"),
            ("actual_output", "()"),
            ("runtime_checks", ""),
            ("flow", ""),
            ("flow_provider", ""),
        ] {
            values.insert(key.to_owned(), value.to_owned());
        }
        values.insert("registry_rule_path".to_owned(), rule_path_for_source(source));
        values.insert("provided_parts".to_owned(), String::new());
        values.insert("required_parts".to_owned(), String::new());
        Self { values }
    }

    fn parse_source(path: &Path) -> Result<Self, String> {
        manifest::parse::source(path)
    }

}

// Fixture-backed authoring tests stay in the parent integration prototype.
#[cfg(all(test, any()))]
mod tests {

    use super::{
        generated_snapshots_from, normalize_kind_name, parse_registration_rule_owned,
        rust_type_name, validate_kind_name, validate_name, FaceManifest, GENERATED_MARKER,
    };
    use crate::ROOT_NODE_ID;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn generated_face_uses_the_standard_folder_identity() {
        let mut face = FaceManifest::new(
            "command_palette",
            "CommandPalette",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "command_palette/command_palette.rs",
        );
        face.edit("stable_name", "command.palette").expect("stable identity edits");
        let source = face.render_source().expect("default face renders");

        assert!(source.contains("kind: CommandPalette"));
        assert!(source.starts_with(GENERATED_MARKER));
        assert!(source.contains("registry_name: command_palette"));
        assert!(source.contains("stable_name: \"command.palette\""));
        assert!(!source.contains("nichlink:"));
        assert!(source.contains("//! CommandPalette registration face."));
        assert!(source.contains("Registration-only handle marker for the CommandPalette face."));
        assert!(source.contains("crate::control_object!"));
        assert!(source.contains("preset: NoPreset"));
        assert!(source.contains("parts: NoParts"));
        assert!(source.contains("parent: crate::ROOT_NODE_ID"));
        assert!(!source.contains("handle_contracts: []"));
        assert!(!source.contains("requires: []"));
        assert_eq!(rust_type_name("command_palette"), "CommandPalette");
        assert!(validate_name("command_palette").is_ok());
        assert!(validate_name("../escape").is_err());
        assert_eq!(
            face.to_snapshot().expect("root face snapshot").parent,
            ROOT_NODE_ID
        );
    }

    #[test]
    fn manifest_exposes_a_stable_read_only_view() {
        let face = FaceManifest::new(
            "button",
            "Button",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "control/button/button.rs",
        );

        assert_eq!(face.get("kind"), Some("Button"));
        assert!(face
            .fields()
            .any(|(key, value)| key == "registry_name" && value == "button"));
    }

    #[test]
    fn generated_face_is_discovered_again_from_disk() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("nichlink-face-scan-{}-{stamp}", std::process::id()));
        let module = root.join("control/object/persisted");
        fs::create_dir_all(&module).expect("temporary module directory");
        let mut face = FaceManifest::new(
            "persisted",
            "Persisted",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "control/object/persisted/persisted.rs",
        );
        face.edit("flow", "render.canvas.v1|1|LocalCoordinates|CanvasFrame")
            .unwrap();
        face.edit("flow_provider", "Button").unwrap();
        face.edit("name_zh", "持久化").unwrap();
        face.edit("name_en", "Persisted").unwrap();
        face.edit("runtime_checks", "FINITE_NUMBER,text_length(1,32)")
            .unwrap();
        fs::write(
            module.join("persisted.rs"),
            face.render_source().expect("temporary face source"),
        )
        .expect("temporary source");

        let snapshots = generated_snapshots_from(&root).expect("owned face can be reloaded");

        assert_eq!(snapshots.len(), 1);
        assert_eq!(
            snapshots[0].source.file,
            "control/object/persisted/persisted.rs"
        );
        assert_eq!(snapshots[0].kind, "Persisted");
        assert_eq!(snapshots[0].parent, ROOT_NODE_ID);
        assert_eq!(snapshots[0].name.zh, "持久化");
        assert_eq!(snapshots[0].name.en, "Persisted");
        assert_eq!(snapshots[0].flow.id, "render.canvas.v1");
        assert_eq!(snapshots[0].flow_provider.as_deref(), Some("Button"));
        assert_eq!(snapshots[0].runtime_checks.len(), 2);
        fs::remove_dir_all(root).expect("temporary module cleanup");
    }

    #[test]
    fn authored_snapshot_owns_reloadable_metadata() {
        let mut face = FaceManifest::new(
            "owned",
            "Owned",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "control/object/owned/owned.rs",
        );
        face.edit("summary_zh", "可释放的快照").unwrap();
        face.edit("summary_en", "A droppable snapshot").unwrap();
        face.edit("exports", "control.owned,control.debug").unwrap();
        face.edit("registration_rule", "preset:NoPreset")
            .unwrap();
        face.edit(
            "flow",
            "render.canvas.v1|1|LocalCoordinates|CanvasFrame",
        )
        .unwrap();
        face.edit("runtime_checks", "FINITE_NUMBER,number_in_range(0,10)")
            .unwrap();

        let snapshot = face.to_snapshot().expect("owned snapshot parses");
        assert_eq!(snapshot.kind, "Owned");
        assert_eq!(snapshot.summary.zh, "可释放的快照");
        assert_eq!(snapshot.exports, ["control.owned", "control.debug"]);
        assert_eq!(
            snapshot.registry_rule.required_preset.as_deref(),
            Some("NoPreset")
        );
        assert_eq!(snapshot.flow.id, "render.canvas.v1");
        assert_eq!(snapshot.flow.version, 1);
        assert_eq!(snapshot.flow.input, "LocalCoordinates");
        assert_eq!(snapshot.runtime_checks.len(), 2);
        let compiled = crate::control::object::button::REGISTRATION.into_snapshot();
        assert_eq!(compiled.flow_provider.as_deref(), Some("Button"));
        let rendered = face.render_source().expect("flow and checks render");
        assert!(rendered.contains("flow: crate::FlowContract::new"));
        assert!(rendered.contains("number_in_range(0, 10)"));
        assert!(rendered.contains(
            "\n    flow: crate::FlowContract::new(crate::ContractId::new(\"render.canvas.v1\"), 1, \"LocalCoordinates\", \"CanvasFrame\"),\n    runtime_checks: [crate::FINITE_NUMBER, crate::RuntimeCheckSpec::number_in_range(0, 10)],"
        ));
    }

    #[test]
    fn authored_snapshot_recovers_the_complete_face() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "nichlink-face-fields-{}-{stamp}",
            std::process::id()
        ));
        let module = root.join("control/object/complete");
        fs::create_dir_all(&module).expect("temporary module directory");
        let source = r#"use crate::{NoParts, NoPreset};
pub struct Complete;
crate::control_object! {
    kind: Complete,
    preset: NoPreset,
    parts: NoParts,
    name: { zh: "完整", en: "Complete" },
    summary: { zh: "摘要", en: "Summary" },
    params: "CompleteParams",
    exports: ["control.complete"],
    handle: Complete,
    needs_registry: false,
    registry_name: complete,
    parent: crate::ROOT_NODE_ID,
    getting_from_other_registry: Some("src/engine/registry/rules/rules.rs"),
    registry_rule_path: "src/control/registry/rules/rules.rs",
    registry_rule: crate::RegistrationRule::ANY,
    admission: crate::Admission::ANY,
    requires: ["layout.viewport" => "ControlRegistry"],
    provides: ["control.complete"],
    expected_output: "Complete",
    actual_output: "Complete",
    runtime_checks: [],
}
"#;
        fs::write(module.join("complete.rs"), source).expect("temporary face source");

        let snapshots = generated_snapshots_from(&root).expect("complete face parses");
        let snapshot = &snapshots[0];
        assert_eq!(snapshot.preset, "NoPreset");
        assert_eq!(snapshot.parts, "NoParts");
        assert_eq!(snapshot.handle, "Complete");
        assert_eq!(snapshot.requires[0].capability, "layout.viewport");
        assert_eq!(snapshot.provides, ["control.complete"]);
        assert_eq!(snapshot.contract.expected_output, "Complete");
        assert_eq!(
            snapshot.getting_from_other_registry.as_deref(),
            Some("src/engine/registry/rules/rules.rs")
        );
        assert!(snapshot.source.line > 1);
        assert!(snapshot.contract.validate(&snapshot.kind).is_empty());
        fs::remove_dir_all(root).expect("temporary module cleanup");
    }

    #[test]
    fn owned_snapshot_validation_reports_all_shape_failures() {
        let face = FaceManifest::new(
            "broken",
            "Broken",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "control/object/broken/broken.rs",
        );
        let mut snapshot = face.to_snapshot().expect("snapshot parses");
        snapshot.contract.required_parts.push("label".to_owned());
        snapshot.contract.expected_output = "Button".to_owned();
        snapshot.contract.actual_output = "Other".to_owned();
        let parent_rule =
            parse_registration_rule_owned("parts:label;handle:ControlHandle").unwrap();
        let mut failures = parent_rule.validate(&snapshot);
        failures.extend(snapshot.contract.validate(&snapshot.kind));
        assert_eq!(failures.len(), 4);
        assert!(failures.iter().any(|failure| failure.contains("label")));
        assert!(failures
            .iter()
            .any(|failure| failure.contains("returns `Other`")));
    }

    #[test]
    fn repeated_face_scans_return_equal_owned_metadata() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "nichlink-face-intern-{}-{stamp}",
            std::process::id()
        ));
        let module = root.join("control/object/interned");
        fs::create_dir_all(&module).expect("temporary module directory");
        let face = FaceManifest::new(
            "interned",
            "Interned",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "control/object/interned/interned.rs",
        );
        fs::write(
            module.join("interned.rs"),
            face.render_source().expect("temporary face source"),
        )
        .expect("temporary source");

        let first = generated_snapshots_from(&root).expect("first scan");
        let second = generated_snapshots_from(&root).expect("second scan");
        assert_eq!(first, second);
        fs::remove_dir_all(root).expect("temporary module cleanup");
    }

    #[test]
    fn editable_face_fields_reach_source_and_snapshot() {
        let mut face = FaceManifest::new(
            "diagnostic_panel",
            "DiagnosticPanel",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "diagnostic_panel/diagnostic_panel.rs",
        );
        face.edit("registry_name", "diagnostics").unwrap();
        face.edit("parts", "crate::NoParts").unwrap();
        face.edit("summary_zh", "诊断面板").unwrap();
        face.edit("summary_en", "Diagnostic panel").unwrap();
        face.edit("registration_rule", "parts:diagnostic")
            .unwrap();

        let source = face.render_source().expect("edited face renders");
        assert!(source.contains("control_object!"));
        assert!(source.contains("registry_name: diagnostics"));
        assert!(source.contains("registry::rules::REGISTRATION_RULE"));

        let info = face.to_snapshot().expect("edited face parses");
        assert_eq!(info.registry_name, "diagnostics");
        assert_eq!(info.parts, "crate::NoParts");
        assert_eq!(info.summary.zh, "诊断面板");
        assert_eq!(info.summary.en, "Diagnostic panel");
        assert_eq!(info.registry_rule.required_parts, ["diagnostic"]);
    }

    #[test]
    fn kind_is_editable_when_add_builds_a_face() {
        let mut face = FaceManifest::new(
            "custom_control",
            "CustomControl",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "custom_control/custom_control.rs",
        );
        face.edit("kind", "InspectorPanel").unwrap();

        let source = face.render_source().expect("kind edit renders");
        assert!(source.contains("kind: InspectorPanel"));
        assert_eq!(face.to_snapshot().unwrap().kind, "InspectorPanel");
        assert!(validate_kind_name("InspectorPanel").is_ok());
        assert!(validate_kind_name("bad-kind").is_err());
        assert_eq!(normalize_kind_name("test"), "Test");
        assert_eq!(normalize_kind_name("test_widget"), "TestWidget");
        assert_eq!(normalize_kind_name("test-widget"), "TestWidget");
        assert_eq!(normalize_kind_name("1test"), "1test");
    }

    #[test]
    fn registration_rule_face_can_describe_interfaces_and_shape() {
        let rule = parse_registration_rule_owned(
            "preset:ActionParts;parts:label,action;exports:control.button;handle:ControlHandle;part_trait:ActionParts",
        )
        .expect("extended rule parses");
        assert_eq!(rule.required_preset.as_deref(), Some("ActionParts"));
        assert_eq!(rule.required_parts, ["label", "action"]);
        assert_eq!(rule.required_exports, ["control.button"]);
        assert_eq!(rule.required_handle_traits, ["ControlHandle"]);
        assert_eq!(rule.required_part_traits, ["ActionParts"]);

        let mut face = FaceManifest::new(
            "button",
            "Button",
            ROOT_NODE_ID,
            "<root>",
            "root",
            "button/button.rs",
        );
        face.edit(
            "registration_rule",
            "preset:ActionParts;parts:label,action;exports:control.button;handle:ControlHandle;part_trait:ActionParts",
        )
        .unwrap();
        let source = face.render_source().expect("extended rule renders");
        assert!(source.contains("registry::rules::REGISTRATION_RULE"));
    }

    #[test]
    fn generated_face_inherits_parent_registry_interfaces() {
        let mut root = crate::Registry::root();
        let mut engine = crate::engine::REGISTRATION;
        engine.registry_rule = crate::engine::registry::rules::REGISTRATION_RULE;
        root.register_batch([engine])
            .expect("engine mounts at root");

        let parent = root
            .registry(crate::engine::NODE_ID)
            .expect("engine registry");
        let mut face = FaceManifest::new(
            "fast",
            "Fast",
            crate::engine::NODE_ID,
            "engine/engine.rs",
            "EngineRegistry",
            "engine/object/fast/fast.rs",
        );
        face.inherit_registry_contract(parent.registration_rule());
        face.values
            .insert("handle_contracts".to_owned(), "crate::control::ControlHandle".to_owned());
        let source = face.render_source().expect("inherited face renders");
        // `EngineRole` was an earlier test-only placeholder; inherited source must use the
        // parent's actual contract and must not silently reintroduce that fixture name.
        // `EngineRole` 曾是测试占位合同；继承生成的源码必须使用父级真实合同，不能偷偷带回该名称。
        assert!(!source.contains("EngineRole"));
        assert!(source.contains("crate::control_object!"));
        assert!(source.contains("handle_contracts: [crate::control::ControlHandle]"));
        assert!(source.contains("impl crate::control::ControlHandle for Fast {}"));
    }
}
