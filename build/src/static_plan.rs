//! Static built-in topology validation.
//! 内置注册树静态拓扑校验。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registry_identity;
use super::types::Node;
use super::{
    cached_parent_id, face_source_is_active, node_id, parsed_face, relative_display, SourceScope,
};

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
    records.sort_by_key(|record| record.id);

    let ids = records
        .iter()
        .map(|record| record.id)
        .collect::<BTreeSet<_>>();
    let owners = records
        .iter()
        .map(|record| (record.id, record.owns_registry))
        .collect::<BTreeMap<_, _>>();
    for record in &records {
        if record.parent != registry_identity::package_root_node_id()
            && !ids.contains(&record.parent)
        {
            errors.push(
                BuildDiagnostic::new("static-plan", "parent node is missing")
                    .at(record.source.clone(), 0)
                    .field("parent")
                    .expected("registered parent")
                    .actual(record.parent.to_string()),
            );
        } else if record.parent != registry_identity::package_root_node_id()
            && owners.get(&record.parent) == Some(&false)
        {
            errors.push(
                BuildDiagnostic::new("static-plan", "parent does not own a registry")
                    .at(record.source.clone(), 0)
                    .field("parent")
                    .expected("registry owner")
                    .actual(record.parent.to_string()),
            );
        }
    }

    let parents = records
        .iter()
        .map(|record| (record.id, record.parent))
        .collect::<BTreeMap<_, _>>();
    for record in &records {
        let mut current = record.id;
        let mut seen = BTreeSet::new();
        while current != registry_identity::package_root_node_id() {
            if !seen.insert(current) {
                errors.push(
                    BuildDiagnostic::new("static-plan", "parent cycle detected")
                        .at(record.source.clone(), 0)
                        .field("parent")
                        .actual(current.to_string()),
                );
                break;
            }
            let Some(parent) = parents.get(&current).copied() else {
                break;
            };
            current = parent;
        }
    }
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
            if !relative.starts_with("registry_core/") {
                if let Ok(source) = fs::read_to_string(file) {
                    if let Some(face) = parsed_face(&source, &relative) {
                        if face.field("plugin").is_none() {
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
