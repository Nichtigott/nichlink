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
use self::parse::{
    kind_from_source_path, module_name_from_path, module_source_from_node_path,
    parse_admission_expression, parse_admission_owned, parse_flow_expression, parse_flow_value,
    parse_optional_source, parse_registration_rule_owned, parse_requirements,
    parse_requirements_owned, render_admission, render_expression_list, render_face_list,
    render_flow_expression, render_flow_provider, render_impls, render_literal_list,
    render_optional_source, render_path_list, render_registration_rule, render_requirements,
    rule_syntax_for_source, source_path_from_file, split_csv_owned, trait_names_from_paths,
};
#[allow(unused_imports)]
use self::validation::{
    authoring_namespace, is_parent_component, normalize_kind_name, normalized_path, package_root,
    rule_path_for_source, rust_string, rust_type_name, source_root, validate_kind_name,
    validate_name,
};
pub use self::validation::AuthoringContext;
use self::manifest::FaceManifest;

pub use self::operations::{
    add_module, add_module_from_face, add_module_with_registration, delete_module, edit_module,
    edit_module_face, generated_snapshots, generated_snapshots_from, AuthoringChange,
    ModuleFacePatch, NewModuleFace,
};
pub use self::external_graft::{ExternalGraftPlanFile, create_external_graft};

const GENERATED_MARKER: &str = "// generated-by=NichLink";

// Field dictionary shared by the Studio form and the file authoring API.
// The definitions live in the kernel `authoring` module.
// Studio 表单与文件创作 API 共用的字段词典；定义位于 kernel 的
// `authoring` 模块。
pub use nichlink::authoring::{FACE_FIELD_COUNT, FACE_FIELD_NAMES};

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
