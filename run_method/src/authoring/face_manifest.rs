//! File-backed authoring for NichLink registration faces.
//! NichLink 注册面的文件化创作支持。

use std::collections::BTreeMap;
use std::path::Path;

use crate::NodeId;

use super::manifest::FaceManifest;
use super::validation::{authoring_namespace, rule_path_for_source};

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
            ("runtime_checks", ""),
            ("flow", ""),
            ("flow_provider", ""),
        ] {
            values.insert(key.to_owned(), value.to_owned());
        }
        values.insert(
            "registry_rule_path".to_owned(),
            rule_path_for_source(source),
        );
        values.insert("provided_parts".to_owned(), String::new());
        values.insert("required_parts".to_owned(), String::new());
        Self { values }
    }

    pub(super) fn parse_source(path: &Path) -> Result<Self, String> {
        super::manifest::parse::source(path)
    }
}

pub use super::external_graft::{
    ExternalGraftPlanEntry, ExternalGraftPlanFile, create_external_graft, external_graft_root,
    list_external_grafts, read_external_graft, remove_external_graft, rewrite_external_graft,
};
pub use super::operations::{
    AuthoringChange, ModuleFacePatch, NewModuleFace, add_module, add_module_from_face,
    add_module_with_registration, delete_module, edit_module_face, generated_snapshots,
    generated_snapshots_from,
};
pub use super::validation::AuthoringContext;

// Field dictionary and slot names shared by the Studio form and the file
// authoring API. The definitions live in the kernel `authoring` module.
// Studio 表单与文件创作 API 共用的字段词典与槽位名；定义位于 kernel 的
// `authoring` 模块。
pub use nichlink::authoring::{FACE_FIELD_COUNT, face_field};
