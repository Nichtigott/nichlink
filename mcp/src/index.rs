//! Source index and path safety for the read-only MCP bridge.
//! 只读 MCP 桥的源码索引与路径安全。

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

/// One indexed Rust function with its static direct-call set.
/// 一个已索引的 Rust 函数及其静态直接调用集合。
#[derive(Clone, Debug)]
pub(crate) struct Function {
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) end_line: usize,
    pub(crate) calls: Vec<String>,
}

/// One indexed Rust source file.
/// 一个已索引的 Rust 源文件。
#[derive(Clone, Debug)]
pub(crate) struct SourceFile {
    pub(crate) relative: String,
    pub(crate) source: String,
    pub(crate) functions: Vec<Function>,
}

pub(crate) fn required_path(arguments: &Value) -> Result<String, String> {
    arguments
        .get("path")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| "path is required".to_owned())
}

pub(crate) fn load_sources(root: &Path) -> Result<Vec<SourceFile>, String> {
    if !root.is_dir() {
        return Err(format!("source root does not exist: {}", root.display()));
    }
    let mut paths = Vec::new();
    collect_rs(root, &mut paths)?;
    paths.sort();
    paths
        .into_iter()
        .map(|path| load_file(root, &path))
        .collect()
}

/// The filesystem facts the kernel's source walk asks this surface for.
/// 内核源码遍历向本执行面索取的文件系统事实。
struct StdSourceTree;

impl nichlink::source::SourceTree for StdSourceTree {
    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn entries(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
        fs::read_dir(path)
            .map_err(|error| format!("read {}: {error}", path.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("read directory entry: {error}"))
            })
            .collect()
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
    }
}

fn collect_rs(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    nichlink::source::collect_rust_sources(
        &StdSourceTree,
        directory,
        nichlink::source::SourceWalk {
            skip_target: true,
            ..nichlink::source::SourceWalk::EVERYTHING
        },
        |_, _| nichlink::source::Keep::Yes,
        paths,
    )
}

pub(crate) fn load_one(root: &Path, relative: &str) -> Result<SourceFile, String> {
    let path = root.join(relative);
    if !is_safe_child(root, &path) {
        return Err("path must stay inside the configured source root".to_owned());
    }
    if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
        return Err("only Rust source files can be read".to_owned());
    }
    load_file(root, &path)
}

fn load_file(root: &Path, path: &Path) -> Result<SourceFile, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "source path escaped root".to_owned())?
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/");
    Ok(SourceFile {
        functions: parse_functions(&source),
        relative,
        source,
    })
}

fn is_safe_child(root: &Path, path: &Path) -> bool {
    let root = fs::canonicalize(root).ok();
    let path = fs::canonicalize(path).ok();
    match (root, path) {
        (Some(root), Some(path)) => path.starts_with(root),
        _ => false,
    }
}

pub(crate) fn resolve_root(base: &Path, requested: Option<&str>) -> Result<PathBuf, String> {
    let base = fs::canonicalize(base)
        .map_err(|error| format!("source root does not exist: {} ({error})", base.display()))?;
    let candidate = requested.map_or_else(|| base.clone(), |value| base.join(value));
    let candidate = fs::canonicalize(&candidate).map_err(|error| {
        format!(
            "requested root is not readable: {} ({error})",
            candidate.display()
        )
    })?;
    if !candidate.starts_with(&base) {
        return Err("requested root must stay inside NICH_LINK_PACKAGE_ROOT".to_owned());
    }
    if !candidate.is_dir() {
        return Err(format!(
            "requested root is not a directory: {}",
            candidate.display()
        ));
    }
    Ok(candidate)
}

fn parse_functions(source: &str) -> Vec<Function> {
    nichlink::source::function_symbols(source)
        .into_iter()
        .map(|function| Function {
            calls: nichlink::source::direct_calls(&function.body, &function.name),
            name: function.name,
            line: function.line as usize,
            end_line: function.end_line as usize,
        })
        .collect()
}

pub(crate) fn display_list(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_owned()
    } else {
        items.join(", ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_indexes_functions_and_direct_calls() {
        let functions = parse_functions(
            "fn source() { let value = helper(1); sink(value); }\nfn helper(_: i32) {}\nfn sink(_: i32) {}",
        );
        assert_eq!(functions.len(), 3);
        assert_eq!(functions[0].name, "source");
        assert_eq!(functions[0].calls, ["helper", "sink"]);
    }

    #[test]
    fn registration_kinds_are_compact_and_deduplicated() {
        let kinds = nichlink::source::registration_kinds(
            "crate::control_object! { kind: Button, }\ncrate::control_object! { kind: Button, }",
        );
        assert_eq!(kinds, ["Button"]);
    }
}
