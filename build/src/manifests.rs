//! Build audit manifests.
//! 构建审计清单。

use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use super::registry_identity::NodeId;
use super::registry_syntax::GraftSyntax;
use super::types::Node;
use super::{SourceScope, collect_faces, parsed_face, relative_display, write_if_changed};

pub(crate) fn write_pruning_manifest(src: &Path, nodes: &[Node], out_dir: &Path) {
    let mut rows = Vec::new();
    collect_pruning_symbols(src, nodes, &mut rows);
    write_rows(out_dir.join("pruning_manifest.tsv"), rows);
}

pub(crate) fn write_function_manifest(src: &Path, nodes: &[Node], out_dir: &Path) {
    let mut rows = Vec::new();
    collect_function_symbols(src, nodes, &mut rows);
    write_rows(out_dir.join("function_manifest.tsv"), rows);
}

pub(crate) fn write_source_scope_manifest(
    src: &Path,
    nodes: &[Node],
    scope: &SourceScope,
    out_dir: &Path,
) {
    let Some(selected) = &scope.roots else {
        write_if_changed(
            &out_dir.join("source_scope.tsv"),
            &format!(
                "# mode\t{}\n# result\tall\n# selected\tall\n# reason\t{}\n",
                if scope.reason == "scope-all" {
                    "explicit"
                } else {
                    "auto"
                },
                scope.reason
            ),
        );
        return;
    };
    let mut output = format!(
        "# mode\t{}\n# selected\t{}\n# node\tsource\tmodule\n",
        if scope.reason == "scope-all" {
            "explicit"
        } else {
            "auto"
        },
        selected.len()
    );
    for face in collect_faces(src, nodes) {
        if selected.contains(&face.id) {
            writeln!(
                output,
                "{}\t{}\t{}",
                face.id,
                relative_display(src, &face.source),
                face.module
            )
            .unwrap();
        }
    }
    write_if_changed(&out_dir.join("source_scope.tsv"), &output);
}

/// Persist host graft selectors as data-only build metadata.
/// 将宿主 graft 选择器持久化为只含数据的构建元信息。
pub(crate) fn write_graft_manifest(out_dir: &Path, grafts: &[GraftSyntax]) {
    let mut output = String::from("# cut\tgraft\tfull\tline\tcolumn\n");
    for graft in grafts {
        writeln!(
            output,
            "{}\t{}\t{}\t{}\t{}",
            graft.cut, graft.graft, graft.full, graft.location.line, graft.location.column
        )
        .unwrap();
    }
    write_if_changed(&out_dir.join("graft_plan.tsv"), &output);
}

fn write_rows(path: impl AsRef<Path>, mut rows: Vec<(NodeId, String, String)>) {
    rows.sort();
    rows.dedup();
    let mut output = String::from("# node\tsource\tsymbol\n");
    for (id, source, symbol) in rows {
        writeln!(output, "{id}\t{source}\t{symbol}").unwrap();
    }
    write_if_changed(path.as_ref(), &output);
}

fn collect_function_symbols(src: &Path, nodes: &[Node], rows: &mut Vec<(NodeId, String, String)>) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && !relative.starts_with("registry_core/")
                && let Some(face) = parsed_face(&source, &relative)
            {
                let id = super::registry_identity::package_node_id(
                    &relative,
                    &face.path("kind").unwrap_or_else(|| node.name.clone()),
                );
                let module = relative
                    .rsplit_once('/')
                    .map_or_else(
                        || relative.trim_end_matches(".rs").to_owned(),
                        |(parent, _)| parent.to_owned(),
                    )
                    .replace('/', "::");
                let mut impl_type = None;
                for line in source.lines() {
                    let trimmed = line.trim();
                    if let Some(rest) = trimmed.strip_prefix("impl ") {
                        let ty = rest
                            .split_once(" for ")
                            .map_or(rest, |(_, implementation)| implementation)
                            .split('{')
                            .next()
                            .unwrap_or_default()
                            .trim();
                        if !ty.is_empty() && !trimmed.ends_with('}') {
                            impl_type = Some(ty.trim_end_matches(['<', '>']).to_owned());
                        }
                    }
                    if let Some(name) = function_name(trimmed) {
                        rows.push((
                            id,
                            relative.clone(),
                            format!(
                                "{}::{}{}",
                                module,
                                impl_type
                                    .as_deref()
                                    .map(|ty| format!("{ty}::"))
                                    .unwrap_or_default(),
                                name
                            ),
                        ));
                    }
                    if trimmed == "}" {
                        impl_type = None;
                    }
                }
            }
        }
        collect_function_symbols(src, &node.children, rows);
    }
}

fn function_name(line: &str) -> Option<String> {
    let marker = line.find("fn ")?;
    let name = line[marker + 3..]
        .chars()
        .take_while(|ch| *ch == '_' || ch.is_ascii_alphanumeric())
        .collect::<String>();
    (!name.is_empty()).then_some(name)
}

fn collect_pruning_symbols(src: &Path, nodes: &[Node], rows: &mut Vec<(NodeId, String, String)>) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if let Ok(source) = fs::read_to_string(file)
                && !relative.starts_with("registry_core/")
                && let Some(face) = parsed_face(&source, &relative)
            {
                let id = super::registry_identity::package_node_id(
                    &relative,
                    &face.path("kind").unwrap_or_else(|| node.name.clone()),
                );
                let module = relative
                    .rsplit_once('/')
                    .map_or_else(
                        || relative.trim_end_matches(".rs").to_owned(),
                        |(parent, _)| parent.to_owned(),
                    )
                    .replace('/', "::");
                let mut found = false;
                for item in source.lines().filter_map(parse_pruning_item) {
                    found = true;
                    rows.push((id, relative.clone(), format!("{module}::{item}")));
                }
                if !found {
                    rows.push((id, relative, "-".to_owned()));
                }
            }
        }
        collect_pruning_symbols(src, &node.children, rows);
    }
}

fn parse_pruning_item(line: &str) -> Option<String> {
    let line = line.trim();
    let name = if line.contains("PRUNING_TABLE")
        && (line.starts_with("static ") || line.starts_with("pub static "))
    {
        "PRUNING_TABLE"
    } else if line.contains("pruning_probe") && line.contains("fn pruning_probe") {
        "pruning_probe"
    } else if line.contains("optional_pruning_probe") && line.contains("fn optional_pruning_probe")
    {
        "Button::optional_pruning_probe"
    } else {
        return None;
    };
    Some(name.to_owned())
}
