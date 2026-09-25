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

#[path = "doc_anchors.rs"]
pub mod doc_anchors;
#[path = "doc_blocks.rs"]
pub mod doc_blocks;
#[path = "lint.rs"]
pub mod lint;
#[path = "mounting.rs"]
pub mod mounting;
#[path = "purity.rs"]
pub mod purity;
#[path = "release_workflow.rs"]
pub mod release_workflow;
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
    if is_real_directory(&directory.join("src")) {
        found.push(directory.to_path_buf());
    }
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if is_skipped_directory(&path) {
            continue;
        }
        if is_real_directory(&path) && is_real_directory(&path.join("src")) {
            found.push(path);
        }
    }
}

/// Directory names a walk never enters.
/// 遍历永不进入的目录名。
///
/// `target/` holds build output, and `build_method` writes generated Rust into it
/// (`<member>/target/nichlink/out/*.rs`). A generated file is not source: the
/// mounting gate once read `include!` out of a stale artifact, which is a failure
/// a maintainer cannot fix by editing the file the gate names.
/// `target/` 存放构建产物，而 `build_method` 会把生成的 Rust 写进去
/// （`<member>/target/nichlink/out/*.rs`）。生成的文件不是源码：挂载门禁曾从一份过期产物里
/// 读到 `include!`，而那是维护者无法通过编辑门禁点名的那个文件来修复的失败。
const SKIPPED_DIRECTORIES: &[&str] = &["target"];

/// Whether `path` names a directory that a walk should skip by name.
/// `path` 是否是按名字应当跳过的目录。
pub(crate) fn is_skipped_directory(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| SKIPPED_DIRECTORIES.contains(&name))
}

/// Whether `path` is a directory that is really there, as opposed to a symbolic
/// link to one.
/// `path` 是否是一个真实存在的目录，而不是指向某个目录的符号链接。
///
/// [`rust_sources`] promises not to follow symlinks, and `Path::is_dir` breaks
/// that promise: a link such as `core/src/zz -> /tmp/elsewhere` used to pull files
/// from outside the checkout into every gate that walks a crate, and the purity
/// gate reported `/tmp/elsewhere/evil.rs` as kernel I/O.
/// [`rust_sources`] 承诺不跟随符号链接，而 `Path::is_dir` 会破坏这个承诺：像
/// `core/src/zz -> /tmp/elsewhere` 这样的链接过去会把检出之外的文件拉进每一个遍历 crate 的
/// 门禁，纯净性门禁曾把 `/tmp/elsewhere/evil.rs` 报成内核 I/O。
///
/// `symlink_metadata` describes the link itself, so a linked directory is not a
/// directory here.
/// `symlink_metadata` 描述的是链接本身，因此被链接的目录在这里不算目录。
pub(crate) fn is_real_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
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
        if is_real_directory(&path) {
            if !is_skipped_directory(&path) {
                collect_rust_sources(&path, files);
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A throwaway directory tree for a walker test.
    /// 供遍历器测试使用的一次性目录树。
    fn synthetic(name: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-conventions-{name}-{}-{}",
            std::process::id(),
            sequence
        ));
        fs::create_dir_all(&root).expect("fixture root");
        root
    }

    /// A linked directory is not entered. The walk promises not to follow
    /// symlinks, and `Path::is_dir` used to break that promise by pulling files
    /// from outside the checkout into every gate.
    /// 被链接的目录不会被进入。遍历承诺不跟随符号链接，而 `Path::is_dir` 过去会破坏这个承诺，
    /// 把检出之外的文件拉进每一个门禁。
    #[cfg(unix)]
    #[test]
    fn a_linked_directory_is_not_walked_into() {
        let root = synthetic("link");
        let outside = root.join("outside");
        fs::create_dir_all(&outside).expect("outside dir");
        fs::write(outside.join("evil.rs"), "fn evil() {}\n").expect("outside file");
        let inside = root.join("inside");
        fs::create_dir_all(&inside).expect("inside dir");
        fs::write(inside.join("good.rs"), "fn good() {}\n").expect("inside file");
        std::os::unix::fs::symlink(&outside, inside.join("zz_link")).expect("fixture link");
        let found = rust_sources(&inside);
        assert_eq!(
            found,
            vec![inside.join("good.rs")],
            "a linked directory must not contribute files: {found:#?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// Build output is not source. `<member>/target/nichlink/out/*.rs` is where
    /// `build_method` writes generated Rust, and a stale artifact must not be read
    /// as a declaration the repository ships.
    /// 构建产物不是源码。`<member>/target/nichlink/out/*.rs` 是 `build_method` 写生成 Rust 的
    /// 地方，而一份过期产物不得被读成仓库出厂的声明。
    #[test]
    fn build_output_is_not_walked_into() {
        let root = synthetic("target");
        fs::create_dir_all(root.join("target/nichlink/out")).expect("artifact dir");
        fs::write(
            root.join("target/nichlink/out/generated.rs"),
            "include!(\"stale\");\n",
        )
        .expect("artifact file");
        fs::write(root.join("lib.rs"), "fn kept() {}\n").expect("source file");
        let found = rust_sources(&root);
        assert_eq!(
            found,
            vec![root.join("lib.rs")],
            "a build artifact must not be read as source: {found:#?}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
