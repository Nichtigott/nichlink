//! Build-time declaration validation.
//! 构建期注册声明校验。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use super::diagnostics::{BuildDiagnostic, BuildDiagnostics};
use super::registration_check;
use super::registry_syntax::{FaceSyntax, ParentSyntax, parse_face};
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

/// Check that a parent-specific macro agrees with the declared parent.
/// 校验父级专属宏名与声明的 parent 一致。
pub(crate) fn aggregate_parent_macro_errors(src: &Path, nodes: &[Node]) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    collect_parent_macro_errors(src, nodes, &mut errors);
    errors
}

fn collect_parent_macro_errors(src: &Path, nodes: &[Node], errors: &mut BuildDiagnostics) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && let Some(face) = parsed_face(&source, &relative)
            {
                let declared = face
                    .macro_name
                    .strip_suffix("_object")
                    .filter(|name| *name != "external")
                    .filter(|name| *name != "control" || source.contains("generated-by=NichLink"));
                if let Some(declared) = declared {
                    let Some(parent) = face.parent() else {
                        errors.push(
                            BuildDiagnostic::new(
                                "parent-macro",
                                "parent-specific registration macro requires an explicit parent",
                            )
                            .at(relative.clone(), face.location.line)
                            .field("parent")
                            .expected(if declared == "root" {
                                "parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\"))".to_owned()
                            } else {
                                format!("parent: crate::{declared}::NODE_ID")
                            })
                            .actual("missing"),
                        );
                        collect_parent_macro_errors(src, &node.children, errors);
                        continue;
                    };
                    let expected = match parent {
                        ParentSyntax::Root => "root".to_owned(),
                        ParentSyntax::FromPath { source, .. } => Path::new(&source)
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or("root")
                            .to_owned(),
                        ParentSyntax::NodePath(module) => module
                            .rsplit("::")
                            .find(|segment| !segment.is_empty())
                            .unwrap_or("root")
                            .to_owned(),
                    };
                    if declared != expected {
                        errors.push(
                            BuildDiagnostic::new(
                                "parent-macro",
                                "registration macro does not match its parent registry",
                            )
                            .at(relative.clone(), face.location.line)
                            .field("parent")
                            .expected(format!("crate::{expected}_object!"))
                            .actual(format!("crate::{}_object!", declared)),
                        );
                    }
                }
            }
        }
        collect_parent_macro_errors(src, &node.children, errors);
    }
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

#[cfg(test)]
mod tests {
    use super::aggregate_parent_macro_errors;
    use crate::types::Node;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn validates_all_three_parent_specific_declaration_levels() {
        let root = temporary_directory("parent-macros");
        let workspace = root.join("workspace/workspace.rs");
        let panel = root.join("workspace/object/panel/panel.rs");
        let child = root.join("workspace/object/panel/object/child/child.rs");
        write_face(
            &workspace,
            "crate::root_object! { kind: Workspace, parent: crate::ROOT_NODE_ID, }",
        );
        write_face(
            &panel,
            "crate::workspace_object! { kind: Panel, parent: crate::workspace::NODE_ID, }",
        );
        write_face(
            &child,
            "crate::panel_object! { kind: Child, parent: crate::workspace::object::panel::NODE_ID, }",
        );
        let nodes = vec![Node {
            name: "workspace".to_owned(),
            file: Some(workspace),
            children: vec![Node {
                name: "panel".to_owned(),
                file: Some(panel),
                children: vec![Node {
                    name: "child".to_owned(),
                    file: Some(child),
                    children: Vec::new(),
                }],
            }],
        }];
        assert!(aggregate_parent_macro_errors(&root, &nodes).is_empty());
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    #[test]
    fn reports_the_expected_macro_for_a_wrong_parent_spelling() {
        let root = temporary_directory("wrong-parent-macro");
        let child = root.join("panel/object/child/child.rs");
        write_face(
            &child,
            "crate::wrong_object! { kind: Child, parent: crate::panel::NODE_ID, }",
        );
        let nodes = vec![Node {
            name: "child".to_owned(),
            file: Some(child),
            children: Vec::new(),
        }];
        let rendered = aggregate_parent_macro_errors(&root, &nodes).render();
        assert!(rendered.contains("phase=parent-macro"));
        assert!(rendered.contains("expected=crate::panel_object!"));
        assert!(rendered.contains("actual=crate::wrong_object!"));
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    #[test]
    fn reports_a_missing_explicit_parent() {
        let root = temporary_directory("missing-parent");
        let child = root.join("panel/object/child/child.rs");
        write_face(&child, "crate::panel_object! { kind: Child, }");
        let nodes = vec![Node {
            name: "child".to_owned(),
            file: Some(child),
            children: Vec::new(),
        }];
        let rendered = aggregate_parent_macro_errors(&root, &nodes).render();
        assert!(
            rendered.contains("parent-specific registration macro requires an explicit parent")
        );
        assert!(rendered.contains("expected=parent: crate::panel::NODE_ID"));
        fs::remove_dir_all(root).expect("temporary fixture cleanup");
    }

    fn write_face(path: &std::path::Path, source: &str) {
        fs::create_dir_all(path.parent().expect("fixture parent"))
            .expect("temporary fixture directory");
        fs::write(path, source).expect("temporary registration face");
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
