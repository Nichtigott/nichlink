//! Throwaway package copies and the diff a preview reports.
//! 一次性包副本与预览报告的 diff。
//!
//! Copying, rather than writing and reverting, is what makes a preview safe: if
//! anything goes wrong between the two, the project was never touched. The copy
//! skips `target/` and NichLink's own runtime directory — they are not inputs to an
//! edit, and `target/` is the one directory that can be large.
//! 用复制而不是"先写再回滚"，正是预览安全的原因：两者之间无论哪里出错，项目从未被碰过。副本跳过
//! `target/` 与 NichLink 自己的运行期目录——它们不是编辑的输入，而 `target/` 是唯一可能很大的
//! 目录。

use std::path::{Path, PathBuf};

/// A throwaway copy of the package, so a preview cannot touch the project.
/// 包的一次性副本，因此预览碰不到项目。
///
/// Build output and NichLink's own runtime data are skipped: they are not inputs
/// to the edit, and `target/` is the one directory that can be large.
/// 构建产物与 NichLink 自己的运行期数据被跳过：它们不是编辑的输入，而 `target/` 是唯一可能很大的
/// 目录。
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

fn copy_directory(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to)
        .map_err(|error| format!("cannot create {}: {error}", to.display()))?;
    let entries = std::fs::read_dir(from)
        .map_err(|error| format!("cannot read {}: {error}", from.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("cannot read a directory entry: {error}"))?;
        let name = entry.file_name();
        if name == "target" || name == nichlink::lexicon::NICHLINK_DIR {
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
        if name == "target" || name == nichlink::lexicon::NICHLINK_DIR {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if let Ok(relative) = path.strip_prefix(root) {
            files.push(relative.to_string_lossy().replace('\\', "/"));
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
