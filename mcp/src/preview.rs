//! Throwaway package copies and the diff a preview reports.
//! 一次性包副本与预览报告的 diff。
//!
//! Copying, rather than writing and reverting, is what makes a preview safe: if
//! anything goes wrong between the two, the project was never touched. The copy
//! skips the directories that are neither an input to an edit nor small: the build
//! output `target/`, the version-control store `.git/`, and NichLink's own runtime
//! data `.nichlink/` — the same three the diff walk skips, because both walks decide
//! one thing.
//! 用复制而不是"先写再回滚"，正是预览安全的原因：两者之间无论哪里出错，项目从未被碰过。副本跳过
//! 那些既不是编辑输入、又不见得小的目录：构建产物 `target/`、版本库 `.git/`，以及 NichLink 自己的
//! 运行期数据 `.nichlink/`——与 diff 遍历跳过的是同三个，因为两处遍历决定的是同一件事。

use std::path::{Path, PathBuf};

/// A throwaway copy of the package, so a preview cannot touch the project.
/// 包的一次性副本，因此预览碰不到项目。
///
/// The build output, the version-control store, and NichLink's own runtime data are
/// skipped: they are not inputs to the edit, and any of the three can be large
/// (`.git/` alone runs to gigabytes in a real repository). The earlier wording named
/// only the build output, which is why every preview copied that store too and walked
/// it into the diff (audit `BR-C2`).
/// 构建产物、版本库与 NichLink 自己的运行期数据都被跳过：它们不是编辑的输入，而这三者中任何一个
/// 都可能很大（真实仓库里仅 `.git/` 就常是 GB 级）。早先的措辞只点名了构建产物，于是每一次预览都
/// 连带复制了那个库、并把它带进 diff（审计 `BR-C2`）。
pub(crate) fn copy_package(root: &Path) -> Result<PathBuf, String> {
    let destination = std::env::temp_dir().join(format!(
        "nichlink-mcp-preview-{}-{}",
        std::process::id(),
        next_sequence()
    ));
    let _ = std::fs::remove_dir_all(&destination);
    copy_directory(root, &destination)?;
    Ok(destination)
}

fn next_sequence() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

/// Whether a directory name is one the preview copy and its diff both skip.
/// 一个目录名是否属于预览副本与其 diff 都跳过的那一类。
///
/// One list, because the two walks answer the same question: a directory the copy
/// skipped but the diff did not would be reported as removed, and one the diff
/// skipped but the copy did not is invisible by construction — either way the report
/// would describe a change the edit never made (audit `BR-C2`, `BR-4`).
/// 只有一份清单，因为两处遍历回答的是同一个问题：副本跳过而 diff 没跳过的目录会被报成删除，反过来
/// 的目录则从一开始就看不见——两种都会让报告描述一次编辑根本没做过的改动（审计 `BR-C2`、`BR-4`）。
fn skipped_directory(name: &std::ffi::OsStr) -> bool {
    name == "target" || name == ".git" || name == nichlink::lexicon::NICHLINK_DIR
}

fn copy_directory(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to)
        .map_err(|error| format!("cannot create {}: {error}", to.display()))?;
    let entries = std::fs::read_dir(from)
        .map_err(|error| format!("cannot read {}: {error}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read a directory entry: {error}"))?;
        let name = entry.file_name();
        if skipped_directory(&name) {
            continue;
        }
        let source = entry.path();
        let destination = to.join(&name);
        if source.is_dir() {
            copy_directory(&source, &destination)?;
        } else if source.is_file() {
            std::fs::copy(&source, &destination)
                .map_err(|error| format!("cannot copy {}: {error}", source.display()))?;
        }
    }
    Ok(())
}

pub(crate) fn remove_copy(root: &Path, work: &Path) {
    if work != root {
        let _ = std::fs::remove_dir_all(work);
    }
}

/// Where a written face's declaration sits, as `<path>:<line>`.
/// 被写入的面的声明位置，写作 `<path>:<line>`。
///
/// The executor validates and reports refusals with a position (the kernel's
/// diagnostics carry `file:line:column`), so a *successful* write has to answer the
/// same question or the caller has to guess: a face's declaration is the macro
/// invocation, which is the line an editor or a follow-up edit wants.
/// 执行器在拒绝时带着位置报告（内核诊断携带 `file:line:column`），因此**成功**的写入必须回答同一个
/// 问题，否则调用方只能猜：一个面的声明就是那次宏调用，而那正是编辑器或后续编辑想要的那一行。
pub(crate) fn declaration_line(file: &Path, relative: &Path) -> Option<String> {
    let text = std::fs::read_to_string(file).ok()?;
    let line = text.lines().position(|line| line.contains("! {"))?;
    // Forward slashes, like every other tree-relative path this bridge reports
    // (`FaceView.source`, the diff headers): a caller comparing the anchor with a
    // path out of `nichlink.registry` must not have to know which platform produced
    // it. The absolute path in the line above stays native, like `status`'s root.
    // 正斜杠，与本桥报告的每一条树内相对路径一致（`FaceView.source`、diff 头）：把锚点与
    // `nichlink.registry` 给出的路径相比的调用方，不该需要知道它由哪个平台产生。上面那行的绝对
    // 路径保持本机写法，与 `status` 的 root 一致。
    let relative = crate::index::portable_path(relative);
    Some(format!("{relative}:{}", line + 1))
}

/// A line diff between the project and the copy it was previewed in.
/// 项目与它据以预览的副本之间的逐行 diff。
///
/// Deliberately not a diff *algorithm*: the common prefix and suffix are trimmed
/// and everything between them is printed as removed then added. That cannot
/// mislabel a line as unchanged (the part it claims is common really is), and for
/// the files these edits touch — a module source and its registry rule — a
/// matching diff would mostly cost code. A caller that needs precise hunks should
/// read the written file, whose path this report gives exactly.
/// 有意不做 diff **算法**：裁掉公共前缀与公共后缀，中间部分先按删除、再按新增打印。它不会把某行
/// 误标为未变（它声明公共的部分确实是公共的），而对这些编辑会碰的文件——一个模块源与其注册规则
/// ——真正的匹配式 diff 基本只是在多写代码。需要精确 hunk 的调用方应当去读被写入的那个文件，
/// 本报告会给出它的准确路径。
pub(crate) fn diff_package(original: &Path, work: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_files(work, work, &mut files)?;
    files.sort();
    let mut output = String::new();
    for relative in files {
        let after = std::fs::read_to_string(work.join(&relative)).unwrap_or_default();
        match std::fs::read_to_string(original.join(&relative)) {
            Ok(before) if before == after => continue,
            Ok(before) => {
                output.push_str(&format!("~ {relative}\n"));
                output.push_str(&line_diff(&before, &after));
            }
            Err(_) => {
                output.push_str(&format!("+ {relative}\n"));
                for line in after.lines() {
                    output.push_str(&format!("+{line}\n"));
                }
            }
        }
    }
    // A file the edit *removed* is part of the answer too.
    // 被编辑**删除**的文件同样是答案的一部分。
    let mut before = Vec::new();
    collect_files(original, original, &mut before)?;
    before.sort();
    for relative in before {
        if !original.join(&relative).exists() {
            continue;
        }
        if !work.join(&relative).exists() {
            output.push_str(&format!("- {relative}\n"));
        }
    }
    Ok(output)
}

fn collect_files(root: &Path, directory: &Path, files: &mut Vec<String>) -> Result<(), String> {
    let entries = std::fs::read_dir(directory)
        .map_err(|error| format!("cannot read {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read a directory entry: {error}"))?;
        let name = entry.file_name();
        if skipped_directory(&name) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if let Ok(relative) = path.strip_prefix(root) {
            files.push(crate::index::portable_path(relative));
        }
    }
    Ok(())
}

fn line_diff(before: &str, after: &str) -> String {
    let before = before.lines().collect::<Vec<_>>();
    let after = after.lines().collect::<Vec<_>>();
    let mut head = 0;
    while head < before.len() && head < after.len() && before[head] == after[head] {
        head += 1;
    }
    let mut tail = 0;
    while tail < before.len() - head
        && tail < after.len() - head
        && before[before.len() - 1 - tail] == after[after.len() - 1 - tail]
    {
        tail += 1;
    }
    let mut output = String::new();
    for line in &before[head..before.len() - tail] {
        output.push_str(&format!("-{line}\n"));
    }
    for line in &after[head..after.len() - tail] {
        output.push_str(&format!("+{line}\n"));
    }
    output
}

#[cfg(test)]
#[path = "preview_tests.rs"]
mod preview_tests;
