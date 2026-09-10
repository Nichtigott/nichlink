//! Declaration-level registration contract checks.
//! 注册声明层合同检查。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registry_syntax::{FaceSyntax, ParentSyntax};
use super::types::Node;
use super::{SourceScope, cached_parent_id, node_id, parsed_face, relative_display};

type ParentRules = BTreeMap<super::registry_identity::NodeId, String>;

pub(crate) fn aggregate_contract_errors(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    let mut parent_rules = ParentRules::new();
    collect_parent_rules(src, nodes, &mut parent_rules);
    collect_contract_errors(
        src,
        nodes,
        include_demo,
        scope,
        false,
        &parent_rules,
        &mut errors,
    );
    errors
}

fn collect_parent_rules(src: &Path, nodes: &[Node], rules: &mut ParentRules) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && let Some(face) = parsed_face(&source, &relative)
                && face.boolean("needs_registry") == Some(true)
                && let Some(id) = node_id(src, node)
                && let Some(rule) = declared_rule_source(src, &relative, &face)
            {
                rules.insert(id, rule);
            }
        }
        collect_parent_rules(src, &node.children, rules);
    }
}

fn declared_rule_source(src: &Path, relative: &str, face: &FaceSyntax) -> Option<String> {
    let explicit = face.string("registry_rule_path").map(|path| {
        let relative = path.strip_prefix("src/").unwrap_or(&path);
        src.join(relative)
    });
    let canonical = Path::new(relative)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("registry_rule/registry_rule.rs");
    explicit
        .and_then(|path| fs::read_to_string(path).ok())
        .or_else(|| fs::read_to_string(src.join(canonical)).ok())
        .or_else(|| face.field("registry_rule"))
}

fn collect_contract_errors(
    src: &Path,
    nodes: &[Node],
    include_demo: bool,
    scope: &SourceScope,
    selected_ancestor: bool,
    parent_rules: &ParentRules,
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
                check_parent_rule(&face, &relative, src, node, parent_rules, errors);
            }
        }
        collect_contract_errors(
            src,
            &node.children,
            include_demo,
            scope,
            selected_here,
            parent_rules,
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
    parent_rules: &ParentRules,
    errors: &mut BuildDiagnostics,
) {
    if matches!(face.parent(), Some(ParentSyntax::Root) | None) {
        return;
    }
    let Some(parent_rule) = cached_parent_id(src, face).and_then(|id| parent_rules.get(&id)) else {
        return;
    };
    let id = node_id(src, node).map_or_else(|| "<unknown>".to_owned(), |id| id.to_string());
    let kind = face.path("kind").unwrap_or_else(|| "<unknown>".to_owned());
    let handle = face
        .path("handle")
        .or_else(|| face.path("kind"))
        .unwrap_or_else(|| "<unknown>".to_owned());
    if let Some(required) = rule_method_strings(parent_rule, "require_preset").first()
        && face.path("preset").as_deref() != Some(required.as_str())
    {
        let line = face
            .field_location("preset")
            .map_or(face.location.line, |location| location.line);
        errors.push(
            BuildDiagnostic::new("contract", "required preset is missing")
                .node(id.clone(), kind.clone())
                .at(relative, line)
                .function(handle.clone())
                .field("preset")
                .expected(required),
        );
    }
    for required in rule_method_strings(parent_rule, "require_handle_traits") {
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
        for required in rule_method_strings(parent_rule, method) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn child_errors_are_checked_against_the_parent_registry_rule() {
        let src = temporary_directory("parent-contract");
        let control = src.join("control/control.rs");
        let button = src.join("control/object/button/button.rs");
        let rule = src.join("control/registry_rule/registry_rule.rs");
        write(
            &control,
            r#"crate::root_object! {
    kind: Control,
    needs_registry: true,
    parent: crate::ROOT_NODE_ID,
    registry_rule_path: "src/control/registry_rule/registry_rule.rs",
    registry_rule: crate::control::registry_rule::REGISTRATION_RULE,
}"#,
        );
        write(
            &button,
            r#"crate::control_object! {
    kind: BrokenButton,
    preset: WrongPreset,
    parts: BrokenParts,
    parent: crate::control::NODE_ID,
    exports: ["control.preview"],
}"#,
        );
        write(
            &rule,
            r#"RegistrationRule::new()
    .require_preset("ActionParts")
    .require_exports(&["control.render"])
    .require_handle_traits(&["ControlHandle"])
    .require_part_traits(&["ActionPartsContract"]);"#,
        );
        let nodes = vec![Node {
            name: "control".to_owned(),
            file: Some(control),
            children: vec![Node {
                name: "button".to_owned(),
                file: Some(button),
                children: Vec::new(),
            }],
        }];
        let diagnostics = aggregate_contract_errors(
            &src,
            &nodes,
            false,
            &SourceScope {
                roots: None,
                reason: "test",
            },
        )
        .render();
        for expected in [
            "required preset is missing",
            "required export is missing",
            "required handle trait is missing",
            "required part trait is missing",
        ] {
            assert!(diagnostics.contains(expected), "{diagnostics}");
        }
        fs::remove_dir_all(src).expect("temporary fixture cleanup");
    }

    fn write(path: &Path, source: &str) {
        fs::create_dir_all(path.parent().expect("fixture parent"))
            .expect("temporary fixture directory");
        fs::write(path, source).expect("temporary fixture source");
    }

    fn temporary_directory(label: &str) -> PathBuf {
        crate::registry_identity::freeze_test_namespace();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock after Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "nichlink-build-{label}-{}-{stamp}",
            std::process::id()
        ));
        fs::create_dir_all(&path).expect("temporary fixture root");
        path
    }
}
