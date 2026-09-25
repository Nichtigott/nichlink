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
#[path = "naming.rs"]
pub mod naming;
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
/// Derived from the root manifest's `members` list rather than from a directory
/// shape, so a newly added crate is covered the day it is listed. The shape-based
/// walk this replaced stopped two levels down, which meant a member nested deeper
/// was invisible to *every* gate at once, while a directory that merely looked like
/// a crate (`studio/tests/fixtures/node-editor/`, a fixture host that is not a
/// member) had to be excluded by hand. The manifest is the source of truth: a crate
/// is a member because it is listed.
/// 从根清单的 `members` 列表推导，而不是从目录形状推导，因此新增的 crate 在被列出的当天就被覆盖。
/// 这里替换掉的按形状遍历只走两层：嵌套更深的成员会同时对所有门禁隐形，而只是"看起来像 crate"
/// 的目录（`studio/tests/fixtures/node-editor/`——一个并非成员的 fixture 宿主）必须靠手工排除。
/// 清单才是真相来源：一个 crate 之所以是成员，是因为它被列出来。
pub fn crate_directories(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for member in workspace_members(root) {
        let path = root.join(&member);
        if is_real_directory(&path.join("src")) {
            found.push(path);
        }
    }
    found.sort();
    found.dedup();
    found
}

/// The root manifest's `members` list, in the order written.
/// 根清单的 `members` 列表，按书写顺序。
///
/// The array has to be read *as an array*: a reader that took everything after
/// `members =` up to the next section header would walk into `[workspace.package]`
/// and take that section's keys for members. That mistake happened once in this
/// repository and stayed invisible only because no directory of the borrowed name
/// exists — the pin below keeps it from happening again.
/// 必须把这个数组当数组读：一个把 `members =` 之后直到下一个段落头的内容全吃进去的读者，会走进
/// `[workspace.package]` 并把那一节的键当成成员。这个错误在本仓库里发生过一次，只因不存在被借名
/// 的目录才隐形——下面的钉子让它不会再发生。
///
/// A manifest that names no members is a panic rather than an empty list: every gate
/// walks this list, so an empty one would read as "everything is clean".
/// 没有列出任何成员的清单会 panic 而不是返回空列表：每个门禁都遍历这份列表，空列表会读作
/// "一切干净"。
pub fn workspace_members(root: &Path) -> Vec<String> {
    let manifest = root.join("Cargo.toml");
    let text = fs::read_to_string(&manifest)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", manifest.display()));
    let mut members = Vec::new();
    let mut in_workspace = false;
    let mut collecting = false;
    for line in text.lines() {
        // A comment can carry quotes; the manifest's own comments must not become
        // member names.
        // 注释里可能带引号；清单自己的注释不得变成成员名。
        let trimmed = line.split('#').next().unwrap_or("").trim();
        if let Some(header) = trimmed.strip_prefix('[') {
            in_workspace = header.trim_end_matches(']').trim() == "workspace";
            collecting = false;
            continue;
        }
        if !in_workspace {
            continue;
        }
        if !collecting {
            let Some(rest) = trimmed.strip_prefix("members") else {
                continue;
            };
            let Some(rest) = rest.trim_start().strip_prefix('=') else {
                continue;
            };
            collecting = true;
            members.extend(quoted(rest));
            // The closing bracket ends the array, not the next section.
            // 结束数组的是右方括号，而不是下一个段落。
            if rest.contains(']') {
                collecting = false;
            }
            continue;
        }
        members.extend(quoted(trimmed));
        if trimmed.contains(']') {
            collecting = false;
        }
    }
    assert!(
        !members.is_empty(),
        "{} names no workspace members; an empty list would make every gate read as clean",
        manifest.display()
    );
    members
}

/// Every `"…"`-quoted value in `text`, in order.
/// `text` 中每个 `"…"` 引号值，按顺序。
fn quoted(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('"') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('"') else {
            break;
        };
        found.push(after[..end].to_owned());
        rest = &after[end + 1..];
    }
    found
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

/// Write the workspace manifest a fixture root needs.
/// 为夹具根写下它需要的工作区清单。
///
/// Every gate now reads the root manifest's `members` list rather than guessing a
/// crate directory from its shape, so a fixture has to declare what it means. The
/// list is derived from the files the fixture actually wrote: a member is the
/// directory that owns a `src/` tree. A fixture with no crate at all still gets a
/// manifest naming one placeholder member, because a missing manifest and an empty
/// member list are both panics — a gate that walked nothing would read as clean.
/// 每个门禁现在读根清单的 `members` 列表，而不是从目录形状猜一个 crate 目录，因此夹具必须声明
/// 它指的是什么。列表从夹具实际写出的文件推导：拥有 `src/` 树的目录就是成员。完全没有 crate 的
/// 夹具也会得到一份清单、其中有一个占位成员，因为缺清单与空成员列表都会 panic——什么都没遍历到的
/// 门禁会读作干净。
#[cfg(test)]
pub(crate) fn fixture_manifest(root: &Path) {
    let mut members = Vec::new();
    collect_fixture_members(root, root, &mut members);
    members.sort();
    members.dedup();
    if members.is_empty() {
        members.push("fixture".to_owned());
        fs::create_dir_all(root.join("fixture/src")).expect("fixture member");
        fs::write(root.join("fixture/src/lib.rs"), "// placeholder\n").expect("fixture member");
    }
    let list = members
        .iter()
        .map(|member| format!("\"{member}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        root.join("Cargo.toml"),
        format!("[workspace]\nmembers = [{list}]\nresolver = \"2\"\n"),
    )
    .expect("fixture manifest");
}

/// Collect the directories under `directory` that own a `src/` tree, relative to
/// `root`.
/// 收集 `directory` 下拥有 `src/` 树的那些目录，以 `root` 为基准。
#[cfg(test)]
fn collect_fixture_members(root: &Path, directory: &Path, members: &mut Vec<String>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !is_real_directory(&path) {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("src") {
            let owner = path.parent().unwrap_or(root).strip_prefix(root);
            let text = owner
                .map(|owner| owner.to_string_lossy().replace('\\', "/"))
                .unwrap_or_default();
            members.push(if text.is_empty() {
                ".".to_owned()
            } else {
                text
            });
            continue;
        }
        collect_fixture_members(root, &path, members);
    }
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

    /// The members array ends at its closing bracket, not at the next section. A
    /// reader that ran on would take `[workspace.package]`'s `name` for a member.
    /// members 数组在右方括号处结束，而不是在下一个段落处。一直读下去的读者会把
    /// `[workspace.package]` 的 `name` 当成成员。
    #[test]
    fn the_members_array_ends_at_its_bracket() {
        let root = synthetic("members");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"core\",\n           \"macro\"]\nresolver = \"2\"\n\n\
             [workspace.package]\nname = \"borrowed\"\n",
        )
        .expect("fixture manifest");
        assert_eq!(
            workspace_members(&root),
            vec!["core".to_owned(), "macro".to_owned()]
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A member nested deeper than the old two-level walk reached is still found.
    /// 嵌套比过去两层遍历更深的成员依然会被找到。
    #[test]
    fn a_deeply_nested_member_is_a_crate_directory() {
        let root = synthetic("deep");
        fs::create_dir_all(root.join("examples/inner/host/src")).expect("fixture crate");
        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"examples/inner/host\"]\n",
        )
        .expect("fixture manifest");
        assert_eq!(
            crate_directories(&root),
            vec![root.join("examples/inner/host")]
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A manifest that lists no members is a panic, not a silently empty walk.
    /// 没有列出任何成员的清单会 panic，而不是静默地空遍历。
    #[test]
    #[should_panic(expected = "names no workspace members")]
    fn a_manifest_without_members_is_a_panic() {
        let root = synthetic("no-members");
        fs::write(root.join("Cargo.toml"), "[workspace]\nresolver = \"2\"\n")
            .expect("fixture manifest");
        let _ = crate_directories(&root);
    }
}
