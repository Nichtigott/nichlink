//! Folder-backed registration discovery.
//! 文件夹注册面发现。

use std::fs;
use std::path::Path;

use super::types::Node;

pub(crate) fn discover_root(src: &Path) -> Vec<Node> {
    let mut nodes: Vec<Node> = fs::read_dir(src)
        .expect("src directory must exist")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_owned();
            if matches!(name.as_str(), "lib.rs" | "main.rs" | "bin") {
                return None;
            }
            if path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
                panic!(
                    "registration source `{}` must use `<name>/<name>.rs` layout",
                    path.display()
                );
            }
            if path.is_dir() && valid_name(&name) {
                let attached = path.join(format!("{name}.rs"));
                return Some(Node {
                    name,
                    file: attached.is_file().then_some(attached),
                    children: discover_children(&path),
                });
            }
            None
        })
        .collect();
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    nodes.retain(has_source);
    nodes
}

fn discover_children(dir: &Path) -> Vec<Node> {
    let mut nodes: Vec<Node> = fs::read_dir(dir)
        .expect("module directory must exist")
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_str()?.to_owned();
            if path.is_file() {
                if path.file_stem().and_then(|stem| stem.to_str())
                    == dir.file_name().and_then(|stem| stem.to_str())
                {
                    return None;
                }
                if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
                    return None;
                }
                panic!(
                    "registration source `{}` must use `<name>/<name>.rs` layout",
                    path.display()
                );
            }
            if path.is_dir() && valid_name(&name) {
                let attached = path.join(format!("{name}.rs"));
                return Some(Node {
                    name,
                    file: attached.is_file().then_some(attached),
                    children: discover_children(&path),
                });
            }
            None
        })
        .collect();
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    nodes.retain(has_source);
    nodes
}

fn has_source(node: &Node) -> bool {
    node.file.is_some() || node.children.iter().any(has_source)
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.chars().enumerate().all(|(index, ch)| {
            ch == '_' || ch.is_ascii_alphanumeric() && (index > 0 || !ch.is_ascii_digit())
        })
}

pub(crate) fn emit_rerun_paths(src: &Path, nodes: &[Node]) {
    println!("cargo:rerun-if-changed={}", src.display());
    let mut files = Vec::new();
    collect_source_files(nodes, &mut files);
    files.sort();
    files.dedup();
    for file in files {
        println!("cargo:rerun-if-changed={}", file.display());
    }
}

pub(crate) fn discovery_fingerprint(src: &Path, nodes: &[Node]) -> String {
    let mut files = Vec::new();
    collect_source_files(nodes, &mut files);
    files.sort();
    let mut input = Vec::new();
    for file in files {
        input.extend_from_slice(super::relative_display(src, &file).as_bytes());
        input.push(0);
        input.extend_from_slice(&fs::read(&file).unwrap_or_default());
        input.push(0xff);
    }
    super::registry_identity::NodeId::from_bytes(&input).to_string()
}

pub(crate) fn collect_source_files(nodes: &[Node], files: &mut Vec<std::path::PathBuf>) {
    for node in nodes {
        if let Some(file) = &node.file {
            files.push(file.clone());
        }
        collect_source_files(&node.children, files);
    }
}
