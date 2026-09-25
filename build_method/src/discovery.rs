//! Folder-backed registration discovery.
//! 文件夹注册面发现。

use std::fs;
use std::path::Path;

use super::Node;
use super::registry_syntax::parse_face;

/// A registration face that lives where the build cannot compile it.
/// 长在构建无法编译的位置上的注册面。
#[derive(Clone, Debug)]
pub(crate) struct UnplacedFace {
    /// The file, relative to the package root.
    /// 文件，以包根为基准。
    pub(crate) relative: String,
    /// One-based line when the failure is inside the file.
    /// 失败发生在文件内部时为从 1 开始的行号。
    pub(crate) line: usize,
    /// The diagnostic phase this belongs to.
    /// 本条属于哪个诊断阶段。
    pub(crate) phase: &'static str,
    /// What is wrong, ready to print.
    /// 问题所在，可直接打印。
    pub(crate) message: String,
}

/// Discover the registration tree, ignoring unplaceable files.
/// 发现注册树，忽略无法安放的文件。
///
/// The findings a caller needs are collected by [`discover_root_reporting`];
/// this entry point exists for the readers that only want the tree.
/// 调用方需要的发现结果由 [`discover_root_reporting`] 收集；本入口供只要树的读者使用。
pub(crate) fn discover_root(src: &Path) -> Vec<Node> {
    let mut unplaced = Vec::new();
    discover_root_reporting(src, &mut unplaced)
}

/// Discover the registration tree and report the faces it cannot place.
/// 发现注册树，并报告它无法安放的注册面。
///
/// A `.rs` file that is not a registration face is skipped in silence: every Rust
/// project has ordinary modules next to its faces, and refusing them would make
/// the tool unusable on a real crate. A file that *is* a face, or that is meant
/// to be one and does not parse, is reported: the build could never compile it,
/// and silence there is exactly the failure this project exists to refuse.
/// 不是注册面的 `.rs` 文件静默跳过：每个 Rust 项目都会在注册面旁边放普通模块，拒绝它们
/// 会让工具在真实 crate 上不可用。而**是**注册面的文件、或本意是面却解析不了的文件会被
/// 报告：构建永远编译不到它，此处沉默正是本项目存在的意义所在——拒绝的那种失败。
pub(crate) fn discover_root_reporting(src: &Path, unplaced: &mut Vec<UnplacedFace>) -> Vec<Node> {
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
                record_unplaced(unplaced, src, &path);
            }
            if path.is_dir() && valid_name(&name) {
                let attached = path.join(format!("{name}.rs"));
                return Some(Node {
                    name,
                    file: attached.is_file().then_some(attached),
                    children: discover_children(src, &path, unplaced),
                });
            }
            None
        })
        .collect();
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    nodes.retain(has_source);
    nodes
}

fn discover_children(src: &Path, dir: &Path, unplaced: &mut Vec<UnplacedFace>) -> Vec<Node> {
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
                record_unplaced(unplaced, src, &path);
            }
            if path.is_dir() && valid_name(&name) {
                let attached = path.join(format!("{name}.rs"));
                return Some(Node {
                    name,
                    file: attached.is_file().then_some(attached),
                    children: discover_children(src, &path, unplaced),
                });
            }
            None
        })
        .collect();
    nodes.sort_by(|left, right| left.name.cmp(&right.name));
    nodes.retain(has_source);
    nodes
}

/// Record a `.rs` file that cannot become a module, unless it is an ordinary one.
/// 记录无法成为模块的 `.rs` 文件，除非它只是普通模块。
fn record_unplaced(unplaced: &mut Vec<UnplacedFace>, src: &Path, path: &Path) {
    let Ok(source) = fs::read_to_string(path) else {
        // An unreadable file makes no claim we can judge; the build's own reads
        // report it where the file actually mattered.
        // 读不了的文件没有可判断的声明；构建自己的读取会在该文件真正要紧的地方报告它。
        return;
    };
    match parse_face(&source) {
        Ok(None) => {}
        Ok(Some(face)) => unplaced.push(UnplacedFace {
            relative: super::relative_display(src, path),
            line: face.location.line,
            phase: "face-layout",
            message: "registration face is outside the `<name>/<name>.rs` layout, so the build can never compile it; move the file to `<name>/<name>.rs`".to_owned(),
        }),
        Err(error) => unplaced.push(UnplacedFace {
            relative: super::relative_display(src, path),
            line: error.location.as_ref().map_or(0, |location| location.line),
            phase: "face-syntax",
            message: error.message,
        }),
    }
}

fn has_source(node: &Node) -> bool {
    node.file.is_some() || node.children.iter().any(has_source)
}

/// Whether a directory name may become a module name.
/// 目录名是否可以成为模块名。
///
/// Discovery and the admission scan must agree on this: a directory discovery
/// skips is a directory whose faces never reach the generated tree, so the
/// admission scan must skip it too — otherwise a face that can never be
/// compiled still vetoes the build.
/// 发现过程与 admission 扫描必须在这一点上一致：发现过程跳过的目录，其注册面永远
/// 进不了生成树，因此 admission 扫描也必须跳过——否则一个永远编译不到的面仍然能否决
/// 构建。
pub(crate) fn valid_name(name: &str) -> bool {
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
