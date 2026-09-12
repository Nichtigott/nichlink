//! Build-time declaration validation.
//! 构建期注册声明校验。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registration_check;
use super::registry_syntax::{FaceSyntax, parse_face};
use super::types::Node;
use super::{SourceScope, collect_active_ids, relative_display};

pub(crate) fn aggregate_requirements(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
    cache_units: Option<&Path>,
) -> BuildDiagnostics {
    let active = scope.roots.as_ref().map(|_| {
        let mut active = std::collections::BTreeSet::new();
        collect_active_ids(src, nodes, scope, false, &mut active);
        active
    });
    registration_check::aggregate(src, include_demo, active.as_ref(), cache_units)
}

pub(crate) fn aggregate_stable_name_errors(src: &Path, nodes: &[Node]) -> BuildDiagnostics {
    let mut names = BTreeMap::<String, (String, usize)>::new();
    let mut errors = BuildDiagnostics::default();
    collect_stable_names(src, nodes, &mut names, &mut errors);
    errors
}

fn collect_stable_names(
    src: &Path,
    nodes: &[Node],
    names: &mut BTreeMap<String, (String, usize)>,
    errors: &mut BuildDiagnostics,
) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if !relative.starts_with("registry_core/")
                && let Ok(source) = fs::read_to_string(file)
                && let Some(face) = parsed_face(&source, &relative)
                && let Some(stable_name) = face.string("stable_name")
            {
                let line = face
                    .field_location("stable_name")
                    .map_or(face.location.line, |location| location.line);
                if let Some((previous_file, previous_line)) = names.get(&stable_name) {
                    errors.push(
                        BuildDiagnostic::new(
                            "stable-identity",
                            format!("duplicate stable_name `{stable_name}`"),
                        )
                        .at(relative, line)
                        .field("stable_name")
                        .expected(format!(
                            "unique; already declared at {previous_file}:{previous_line}"
                        ))
                        .actual(stable_name),
                    );
                } else {
                    names.insert(stable_name, (relative, line));
                }
            }
        }
        collect_stable_names(src, &node.children, names, errors);
    }
}

pub(crate) fn parsed_face(source: &str, display_path: &str) -> Option<FaceSyntax> {
    parse_face(source)
        .unwrap_or_else(|error| panic!("invalid registration face in {display_path}: {error}"))
}
