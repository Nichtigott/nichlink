//! Declaration-level registration contract checks.
//! 注册声明层合同检查。

use std::fs;
use std::path::Path;

use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registry_syntax::{FaceSyntax, ParentSyntax};
use super::types::Node;
use super::{SourceScope, node_id, parsed_face, relative_display};

pub(crate) fn aggregate_contract_errors(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    collect_contract_errors(src, nodes, include_demo, scope, false, &mut errors);
    errors
}

fn collect_contract_errors(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
    selected_ancestor: bool,
    errors: &mut BuildDiagnostics,
) {
    for node in nodes {
        if node.name == "compile_error_demo" && !include_demo
            || !scope.includes(src, node, selected_ancestor)
        {
            continue;
        }
        let selected_here = selected_ancestor
            || node_id(src, node).is_some_and(|id| {
                scope
                    .roots
                    .as_ref()
                    .is_some_and(|roots| roots.contains(&id))
            });
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if !relative.starts_with("registry_core/")
                && let Ok(source) = fs::read_to_string(file)
                && let Some(face) = parsed_face(&source, &relative)
            {
                check_output(&face, &relative, src, node, errors);
                check_parent_rule(&face, &relative, src, node, errors);
            }
        }
        collect_contract_errors(
            src,
            &node.children,
            include_demo,
            scope,
            selected_here,
            errors,
        );
    }
}

fn check_output(
    face: &FaceSyntax,
    relative: &str,
    src: &Path,
    node: &Node,
    errors: &mut BuildDiagnostics,
) {
    let (Some(expected), Some(actual)) =
        (face.string("expected_output"), face.string("actual_output"))
    else {
        return;
    };
    if expected == actual {
        return;
    }
    let kind = face.path("kind").unwrap_or_else(|| "<unknown>".to_owned());
    let handle = face
        .path("handle")
        .unwrap_or_else(|| "<unknown>".to_owned());
    let line = face
        .field_location("actual_output")
        .map_or(face.location.line, |location| location.line);
    let id = node_id(src, node).map_or_else(|| "<unknown>".to_owned(), |id| id.to_string());
    errors.push(
        BuildDiagnostic::new("contract", "output contract does not match")
            .node(id, kind)
            .at(relative, line)
            .function(handle)
            .field("output")
            .expected(expected)
            .actual(actual),
    );
}

fn check_parent_rule(
    face: &FaceSyntax,
    relative: &str,
    src: &Path,
    node: &Node,
    errors: &mut BuildDiagnostics,
) {
    if matches!(face.parent(), Some(ParentSyntax::Root) | None) {
        return;
    }
    let Some(parent_rule) = parent_rule_source(src, face) else {
        return;
    };
    let id = node_id(src, node).map_or_else(|| "<unknown>".to_owned(), |id| id.to_string());
    let kind = face.path("kind").unwrap_or_else(|| "<unknown>".to_owned());
    let handle = face
        .path("handle")
        .unwrap_or_else(|| "<unknown>".to_owned());
    for required in rule_method_strings(&parent_rule, "require_handle_traits") {
        if !face
            .string_list("handle_traits")
            .unwrap_or_default()
            .iter()
            .any(|value| value == &required)
        {
            let line = face
                .field_location("handle_traits")
                .map_or(face.location.line, |location| location.line);
            errors.push(
                BuildDiagnostic::new("contract", "required handle trait is missing")
                    .node(id.clone(), kind.clone())
                    .at(relative, line)
                    .function(handle.clone())
                    .field("handle_traits")
                    .expected(required),
            );
        }
    }
    for (method, field, label) in [
        ("require_exports", "exports", "export"),
        ("require_part_traits", "part_traits", "part trait"),
    ] {
        for required in rule_method_strings(&parent_rule, method) {
            if !face
                .string_list(field)
                .unwrap_or_default()
                .iter()
                .any(|value| value == &required)
            {
                let line = face
                    .field_location(field)
                    .map_or(face.location.line, |location| location.line);
                errors.push(
                    BuildDiagnostic::new("contract", format!("required {label} is missing"))
                        .node(id.clone(), kind.clone())
                        .at(relative, line)
                        .function(handle.clone())
                        .field(field)
                        .expected(required),
                );
            }
        }
    }
}

fn parent_rule_source(src: &Path, face: &FaceSyntax) -> Option<String> {
    let path = face.string("registry_rule_path")?;
    let relative = path.strip_prefix("src/").unwrap_or(&path);
    fs::read_to_string(src.join(relative)).ok()
}

fn rule_method_strings(source: &str, method: &str) -> Vec<String> {
    let marker = format!(".{method}(");
    let Some(start) = source.find(&marker) else {
        return Vec::new();
    };
    let args = &source[start + marker.len()..];
    let end = args.find(')').unwrap_or(args.len());
    args[..end]
        .split('"')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, value)| value.to_owned())
        .collect()
}
