//! Source index and path safety for the MCP bridge's read side.
//! MCP 桥读取一侧的源码索引与路径安全。

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
    // A link that resolves outside the root is not part of the index: the root is
    // the declared scope of every answer this bridge gives, and a file outside it
    // would make those answers larger than the package. The walk already refuses
    // to descend into such a directory; this drops a linked *file*. It is a skip
    // rather than an error because the entry is outside the question, while a
    // direct `inspect`/`read` of the same path is refused by name.
    // 解析到根外的链接不属于本索引：根是这个桥给出的每个回答所声明的范围，而根外的文件会让
    // 回答比包更大。遍历已经拒绝进入这样的目录；这里丢弃的是被链接的**文件**。之所以跳过而
    // 不是报错，是因为该条目在问题范围之外，而对同一路径的直接 `inspect`/`read` 会按名字被拒。
    paths
        .into_iter()
        .filter(|path| is_safe_child(root, path))
        .map(|path| load_file(root, &path))
        .collect()
}

/// The filesystem facts the kernel's source walk asks this surface for.
/// 内核源码遍历向本执行面索取的文件系统事实。
///
/// The root is resolved once and carried, because the walk's own facts are not
/// enough to keep it inside the tree: `is_dir` follows a symbolic link, so a
/// link under the root can point at an ancestor or at a tree outside the package
/// entirely. Every fact this surface reports therefore goes through the
/// canonical form of the path.
/// 根只解析一次并随行携带，因为遍历自身的事实不足以把它留在树内：`is_dir` 会跟随符号链接，
/// 因此根下的一个链接可以指向祖先，或指向包外的整棵树。因此本执行面报告的每个事实都经路径
/// 的规范形式。
struct StdSourceTree {
    /// The canonical source root.
    /// 规范化的源码根。
    root: PathBuf,
}

impl nichlink::source::SourceTree for StdSourceTree {
    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir() && is_safe_child(&self.root, path)
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
        if !is_safe_child(&self.root, path) {
            return Err(format!(
                "{} cannot be resolved inside the configured source root",
                path.display()
            ));
        }
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))
    }
}

fn collect_rs(directory: &Path, paths: &mut Vec<PathBuf>) -> Result<(), String> {
    let tree = StdSourceTree {
        root: fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf()),
    };
    nichlink::source::collect_rust_sources(
        &tree,
        directory,
        nichlink::source::SourceWalk {
            skip_target: true,
            ..nichlink::source::SourceWalk::EVERYTHING
        },
        |_, _| nichlink::source::Keep::Yes,
        paths,
    )?;
    // The write path's own recoverable trash lives under `.nichlink/` and holds
    // `.rs` files, so without this filter a deleted fact keeps answering `status`
    // and `search` from its backup — the bridge indexing its own private state.
    // 写入路径自己的可恢复回收目录在 `.nichlink/` 下，而里面就是 `.rs` 文件；没有这道过滤，一个被
    // 删掉的东西会一直从它的备份里回答 `status` 与 `search`——桥在索引自己的私有状态。
    paths.retain(|path| {
        !path
            .components()
            .any(|component| component.as_os_str() == nichlink::lexicon::NICHLINK_DIR)
    });
    Ok(())
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
    // The `strip_prefix` below only names the file. This is the check that
    // decides whether it may be read at all, and it resolves symbolic links,
    // which a prefix comparison cannot: a link inside the root that points out of
    // it passes the prefix test and fails this one.
    // 下面的 `strip_prefix` 只用来给文件命名。决定它是否可读的是这道检查，而它解析符号
    // 链接——前缀比较做不到：根内指向根外的链接能通过前缀检查，但过不了这一道。
    if !is_safe_child(root, path) {
        return Err(format!(
            "{} cannot be resolved inside the configured source root",
            path.display()
        ));
    }
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

pub(crate) fn is_safe_child(root: &Path, path: &Path) -> bool {
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

    /// The write path's recoverable trash lives under `.nichlink/` and holds
    /// `.rs` files, so a deleted fact must not keep answering `status` and
    /// `search` from its own backup.
    /// 写入路径的可恢复回收目录在 `.nichlink/` 下、里面就是 `.rs` 文件，因此被删掉的东西不得继续
    /// 从它自己的备份里回答 `status` 与 `search`。
    #[test]
    fn the_recoverable_trash_is_not_indexed() {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("mcp-scan-{}-{sequence}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).expect("source directory");
        std::fs::create_dir_all(root.join(".nichlink/trash/faces")).expect("trash directory");
        std::fs::write(root.join("src/live.rs"), "pub fn live() {}\n").expect("live file");
        std::fs::write(
            root.join(".nichlink/trash/faces/deleted.rs"),
            "pub fn deleted() {}\n",
        )
        .expect("backup file");
        let files = load_sources(&root).expect("the scan reads the tree");
        let names = files
            .iter()
            .map(|file| file.relative.clone())
            .collect::<Vec<_>>();
        assert!(names.iter().any(|name| name == "src/live.rs"), "{names:?}");
        assert!(
            !names.iter().any(|name| name.contains(".nichlink")),
            "the trash must not be indexed: {names:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

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

    /// A throwaway source root, unique per call.
    /// 每次调用唯一的临时源码根。
    fn temporary_root(tag: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-mcp-{tag}-{}-{sequence}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create root");
        root
    }

    /// A file genuinely inside the root is read, so the guard is about location
    /// and not about links as such.
    /// 真正位于根内的文件照常读取，因此这道守卫针对的是位置而不是链接本身。
    #[test]
    fn a_file_inside_the_source_root_is_still_read() {
        let root = temporary_root("inside");
        fs::write(root.join("inside.rs"), "fn inside() {}\n").expect("write");
        let sources = load_sources(&root).expect("the walk reads the root");
        assert_eq!(sources.len(), 1);
        assert!(sources[0].source.contains("fn inside"));
        let _ = fs::remove_dir_all(&root);
    }

    /// A link out of the root is neither walked into nor read. A prefix
    /// comparison accepts both, because the link's own path is inside the root.
    /// 指向根外的链接既不会被进入，也读不到。前缀比较两者都会接受，因为链接自身的路径就在
    /// 根内。
    #[cfg(unix)]
    #[test]
    fn a_link_out_of_the_source_root_is_neither_walked_nor_read() {
        let root = temporary_root("escape");
        let outside = root.with_file_name(format!(
            "{}-outside",
            root.file_name().expect("a name").to_string_lossy()
        ));
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&outside).expect("create outside tree");
        fs::write(outside.join("secret.rs"), "fn secret() {}\n").expect("write outside file");
        std::os::unix::fs::symlink(&outside, root.join("linked")).expect("link the outside tree");
        std::os::unix::fs::symlink(outside.join("secret.rs"), root.join("secret.rs"))
            .expect("link the outside file");

        let sources = load_sources(&root).expect("the walk survives an escaping link");
        assert!(
            sources
                .iter()
                .all(|file| !file.source.contains("fn secret")),
            "a file outside the root must not be indexed: {sources:?}"
        );
        let error = load_one(&root, "secret.rs").expect_err("a link out of the root is refused");
        assert!(error.contains("source root"), "{error}");

        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }
}
