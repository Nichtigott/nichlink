//! Static built-in topology validation.
//! 内置注册树静态拓扑校验。

use std::fs;
use std::path::Path;

use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registry_identity;
use super::types::Node;
use super::{
    SourceScope, cached_parent_id, face_source_is_active, node_id, parsed_face, relative_display,
};
use nichlink::{TopologyRecord, validate_face_topology};

#[derive(Clone, Debug)]
pub(crate) struct StaticFaceRecord {
    pub(crate) id: registry_identity::NodeId,
    pub(crate) parent: registry_identity::NodeId,
    pub(crate) owns_registry: bool,
    pub(crate) source: String,
    pub(crate) module: String,
}

pub(crate) fn source_module_path(relative: &str) -> String {
    relative
        .rsplit_once('/')
        .map_or_else(
            || relative.trim_end_matches(".rs").to_owned(),
            |(parent, _)| parent.to_owned(),
        )
        .replace('/', "::")
}

pub(crate) fn static_plan(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
) -> (Vec<StaticFaceRecord>, BuildDiagnostics) {
    let mut records = Vec::new();
    let mut errors = BuildDiagnostics::default();
    collect_static_faces(src, nodes, scope, false, &mut records, &mut errors);

    let mut topology = records
        .iter()
        .map(|record| TopologyRecord {
            id: record.id,
            parent: record.parent,
            owns_registry: record.owns_registry,
            source: record.source.clone(),
        })
        .collect::<Vec<_>>();
    let mut topology_errors =
        validate_face_topology(&mut topology, registry_identity::package_root_node_id());
    errors.extend(std::mem::take(&mut topology_errors));
    (records, errors)
}

fn collect_static_faces(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
    selected_ancestor: bool,
    records: &mut Vec<StaticFaceRecord>,
    errors: &mut BuildDiagnostics,
) {
    for node in nodes {
        if node.name == "compile_error_demo" || !scope.includes(src, node, selected_ancestor) {
            continue;
        }
        let selected_here = selected_ancestor
            || node_id(src, node).is_some_and(|id| {
                scope
                    .roots
                    .as_ref()
                    .is_some_and(|roots| roots.contains(&id))
            });
        if face_source_is_active(src, node, scope, selected_ancestor) {
            let file = node
                .file
                .as_ref()
                .expect("an active registration face has a source file");
            let relative = relative_display(src, file);
            if !relative.starts_with("registry_core/")
                && let Ok(source) = fs::read_to_string(file)
                && let Some(face) = parsed_face(&source, &relative)
                && face.field("plugin").is_none()
            {
                let id = node_id(src, node).expect("a parsed face has an identity");
                let parent = cached_parent_id(src, &face).or_else(|| {
                    face.field("parent")
                        .is_none()
                        .then_some(registry_identity::package_root_node_id())
                });
                match parent {
                    Some(parent) => records.push(StaticFaceRecord {
                        id,
                        parent,
                        owns_registry: face.boolean("needs_registry").unwrap_or(false),
                        module: source_module_path(&relative),
                        source: relative,
                    }),
                    None => errors.push(
                        BuildDiagnostic::new(
                            "static-plan",
                            "parent declaration cannot be resolved",
                        )
                        .at(relative, 0),
                    ),
                }
            }
        }
        collect_static_faces(
            src,
            &node.children,
            scope,
            selected_ancestor || selected_here,
            records,
            errors,
        );
    }
}
