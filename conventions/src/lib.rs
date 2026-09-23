//! Executable gates for the workspace rules that prose cannot enforce.
//! 把只有散文陈述的工作区规则变成可执行门禁。
//!
//! `AGENTS.md` and `docs/roadmap-1.0.md` state rules that a human audit can
//! check by hand: the kernel does no I/O, modules are mounted a certain way,
//! every published crate carries the missing-documentation lint, and documented
//! Rust still parses. Before this crate existed, a violating commit passed
//! every gate, because nothing walked the source tree. A rule that cannot fail
//! a build decays, and the fourth audit round found it already had: the
//! roadmap marked the 450-line ceiling done while fifteen files exceeded it,
//! and a `compile_fail` doctest cited as a pin was compiled by no CI command.
//! `AGENTS.md` 与 `docs/roadmap-1.0.md` 陈述了一些人工审计才能检查的规则：内核不做 I/O、
//! 模块按特定方式挂载、每个已发布 crate 都开着缺失文档 lint、文档里的 Rust 仍然能解析。
//! 在本 crate 出现之前，违反这些规则的提交能通过全部门禁，因为没有任何程序遍历源码树。
//! 无法让构建失败的规则会腐化，第四轮审计发现它已经腐化了：路线图把 450 行上限标成已完成，
//! 而 15 个文件超过它；一个被引为"钉子"的 `compile_fail` doctest 没有任何 CI 命令编译它。
//!
//! Why a separate crate instead of a test inside an existing one: these gates
//! inspect the *repository*, and a published crate ships its `tests/` directory
//! in the `.crate` file, so a workspace walker living there would fail for
//! anyone who unpacks the crate. `publish = false` is what makes the home
//! honest, exactly like the two example hosts.
//! 为什么用独立 crate 而不是在已有 crate 里加测试：这些门禁检查的是**仓库**，而已发布的
//! crate 会把 `tests/` 目录一起打进 `.crate` 文件，放在那里的工作区遍历器会让任何解包
//! 该 crate 的人失败。`publish = false` 让这个归属是诚实的，正如两个示例宿主那样。
#![warn(missing_docs)]

use std::{
    fs,
    path::{Path, PathBuf},
};

#[path = "doc_blocks.rs"]
pub mod doc_blocks;
#[path = "lint.rs"]
pub mod lint;
#[path = "mounting.rs"]
pub mod mounting;
#[path = "purity.rs"]
pub mod purity;
#[path = "size.rs"]
pub mod size;

/// Locate the workspace root from this crate's own manifest directory.
/// 从本 crate 自己的清单目录定位工作区根。
///
/// A gate that silently skipped when the layout looked wrong would be worse
/// than no gate, so a missing root is a panic with the path it expected.
/// 布局不对时静默跳过的门禁比没有门禁更糟，因此找不到根就带着期望路径 panic。
pub fn workspace_root() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root = manifest
        .parent()
        .expect("conventions/ must sit directly under the workspace root")
        .to_path_buf();
    let manifest_file = root.join("Cargo.toml");
    let contents = fs::read_to_string(&manifest_file)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", manifest_file.display()));
    assert!(
        contents.contains("[workspace]"),
        "{} is not a workspace root",
        manifest_file.display()
    );
    root
}

/// Every crate directory in the workspace that owns a `src/` tree, sorted.
/// 工作区中拥有 `src/` 树的每个 crate 目录，已排序。
///
/// Derived from the checkout instead of a hardcoded list so a newly added crate
/// is covered the day it appears rather than the day someone remembers.
/// 从检出推导而不是硬编码列表，因此新增的 crate 出现当天就被覆盖，而不是等到有人想起来。
pub fn crate_directories(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    collect_crate_directories(root, &mut found);
    if let Ok(examples) = fs::read_dir(root.join("examples")) {
        for entry in examples.flatten() {
            let path = entry.path();
            if path.join("src").is_dir() {
                collect_crate_directories(&path, &mut found);
            }
        }
    }
    found.sort();
    found.dedup();
    found
}

/// Collect `directory` and, one level down, any child owning a `src/` tree.
/// 收集 `directory` 本身，以及下一层中任何拥有 `src/` 树的子目录。
fn collect_crate_directories(directory: &Path, found: &mut Vec<PathBuf>) {
    if directory.join("src").is_dir() {
        found.push(directory.to_path_buf());
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && path.join("src").is_dir() {
            found.push(path);
        }
    }
}

/// Every `.rs` file under `directory`, sorted, without following symlinks.
/// `directory` 下每个 `.rs` 文件，已排序，不跟随符号链接。
pub fn rust_sources(directory: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_rust_sources(directory, &mut files);
    files.sort();
    files
}

/// Recursive worker for [`rust_sources`].
/// [`rust_sources`] 的递归实现。
fn collect_rust_sources(directory: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rust_sources(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// Read a file as lines, panicking with the path when it cannot be read.
/// 按行读取文件；读不到时带着路径 panic。
pub fn lines(path: &Path) -> Vec<String> {
    let contents = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
    contents.lines().map(str::to_owned).collect()
}

/// Render a path relative to the workspace root, with `/` separators.
/// 以工作区根为基准渲染路径，使用 `/` 分隔符。
pub fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Whether a line is a Rust comment rather than code.
/// 某一行是注释而不是代码。
///
/// The gates below look for tokens that also appear in prose (`std::fs` in a
/// doc comment explaining why it is banned, for example), so comments are
/// skipped instead of being reported.
/// 下面的门禁查找的词也会出现在散文里（例如解释为何禁止 `std::fs` 的文档注释），因此跳过
/// 注释而不是把它们报出来。
pub fn is_comment(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("//")
}
