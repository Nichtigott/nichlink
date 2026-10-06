//! Writing a planned partition: the ghost's three files, and the workspace config they need.
//! 写下一次已规划的拆分：幽灵的三份文件，以及它们所需的那份工作区配置。
//!
//! The planner decides; this module is the mechanical half. Two rules shape it:
//! 规划器做决定；本模块是机械的那一半。两条规则塑造了它：
//!
//! 1. **Every write is idempotent and compares content** (`write_if_changed`): running the action
//!    twice changes nothing the second time, because a partition is a thing you commit, and a
//!    commit-shaped action that rewrites identical bytes is a diff nobody asked for.
//!    **每次写入都幂等且先比内容**（`write_if_changed`）：这个动作跑第二遍不会改任何东西，因为一次拆分是
//!    要提交的东西，而一个把相同字节重写一遍的提交形状动作，是一份没人要的 diff。
//! 2. **The workspace config is merged, never written**: `rustflags` are per **invocation** and cargo
//!    finds config from the current directory, so the remap has to live at the workspace root — a file
//!    the user owns. Keys this action does not understand are left exactly as they are, and a shape it
//!    cannot merge safely is **refused by name** with the lines to add by hand.
//!    **工作区配置是合并的、从不整体写下**：`rustflags` 是每次调用生效的，而 cargo 从当前目录找 config，
//!    因此 remap 必须住在工作区根——那是用户拥有的文件。本动作看不懂的键**原样保留**，而它无法安全合并的
//!    形状会被**点名拒绝**，并给出该手写上去的那几行。

use std::fs;
use std::path::Path;

use super::crate_plan::PlannedCrate;
use super::discovery_cache::write_if_changed;

/// What one write pass changed, for the reply the caller renders.
/// 一次写入改了哪些东西，供调用方渲染回复。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Written {
    /// Files this pass created or rewrote, relative to the workspace root.
    /// 本次创建或重写的文件，相对工作区根。
    pub(crate) files: Vec<String>,
    /// Whether the workspace config was touched (false when it already carried every entry).
    /// 工作区配置是否被改动（它已经带着每一条时是 false）。
    pub(crate) config_changed: bool,
}

/// Write every planned crate, then make sure the workspace config carries the remap.
/// 写下每个已规划的 crate，然后确保工作区配置带着那些 remap。
pub(crate) fn write_partition(
    workspace_root: &Path,
    planned: &[PlannedCrate],
) -> Result<Written, String> {
    let mut written = Written::default();
    let mut remap: Vec<(String, String)> = Vec::new();
    for planned in planned {
        remap.extend(planned.remap.iter().cloned());
    }
    remap.sort();
    remap.dedup();
    // The config comes **first**: a `rustflags` this action will not merge into is a refusal, and a
    // refusal must leave the tree exactly as it found it — writing the packages first would leave a
    // partition with no remap, which is a partition whose identities have silently moved.
    // 配置**先**做：本动作不肯合并的 `rustflags` 是一句拒绝，而拒绝必须让树保持原样——先写包会留下一次没有
    // remap 的拆分，而那样的拆分里身份已经悄悄搬了家。
    written.config_changed = merge_workspace_config(workspace_root, &remap)?;
    for planned in planned {
        let lib = planned.directory.join("src/lib.rs");
        let build = planned.directory.join("build.rs");
        let manifest = planned.directory.join("Cargo.toml");
        for (path, content) in [
            (&manifest, &planned.cargo_toml),
            (&lib, &planned.lib_rs),
            (&build, &planned.build_rs),
        ] {
            write_ghost_file(path, content)?;
            written.files.push(relative(workspace_root, path));
        }
    }
    written.files.sort();
    written.files.dedup();
    Ok(written)
}

/// Create the file's directory and write it if the bytes differ.
/// 创建文件的目录，并在字节不同时写入。
fn write_ghost_file(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    write_if_changed(path, content)
}

/// Add the missing `--remap-path-prefix` entries to the workspace root's `.cargo/config.toml`.
/// 把缺少的 `--remap-path-prefix` 条目加到工作区根的 `.cargo/config.toml`。
///
/// Returns whether the file changed. Three shapes are handled, and the fourth is refused:
/// 返回文件是否被改动。三种形状会被处理，第四种被拒绝：
///
/// - no file: one is written with a `[build]` section and the entries;
/// - a file with no `[build]` section: the section is appended;
/// - a file with a `[build]` section whose `rustflags` is a **single-line** array: the missing entries
///   are inserted into that array, and every other line and key is left exactly as it was;
/// - anything else (a multi-line array, a `rustflags` that is not an array, a `[build]` written in
///   another syntax) is refused by name, with the entries to add by hand — merging blindly into a file
///   the user maintains is how a tool eats somebody's configuration.
/// - 没有文件：写一份，带 `[build]` 节与那些条目；
/// - 有文件但没有 `[build]` 节：追加该节；
/// - 有 `[build]` 节、且 `rustflags` 是**单行**数组：把缺少的条目插进那个数组，其余每一行、每一个键原样保留；
/// - 其它一切（多行数组、`rustflags` 不是数组、用别的语法写的 `[build]`）一律**点名拒绝**，并给出该手写上去的
///   条目——盲目往用户维护的文件里合并，正是一件工具吃掉别人配置的方式。
fn merge_workspace_config(
    workspace_root: &Path,
    remap: &[(String, String)],
) -> Result<bool, String> {
    let path = workspace_root.join(".cargo/config.toml");
    let entries: Vec<String> = remap
        .iter()
        .map(|(from, to)| format!("\"--remap-path-prefix={from}={to}\""))
        .collect();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let content = format!(
                "# Added by NichLink's partition action: the remap keeps every mounted face's `file!()`\n\
                 # spelling identical to the host's, which is what keeps its identity (audit `M7`, P3.3).\n\
                 # 由 NichLink 的拆分动作添加：remap 让每个被挂载面的 `file!()` 拼写与宿主一致，\n\
                 # 而这正是保住它身份的东西（审计 `M7`，P3.3）。\n\
                 [build]\nrustflags = [{}]\n",
                entries.join(", ")
            );
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
            }
            return write_if_changed(&path, &content).map(|()| true);
        }
        Err(error) => {
            return Err(format!(
                "add_crates: cannot read {}: {error}",
                path.display()
            ));
        }
    };
    let missing: Vec<&String> = entries
        .iter()
        .filter(|entry| !text.contains(entry.as_str()))
        .collect();
    if missing.is_empty() {
        return Ok(false);
    }
    let merged = match merge_rustflags(&text, &missing) {
        Some(merged) => merged,
        None => {
            return Err(format!(
                "add_crates: {} already has a `rustflags` this action will not merge into (it is not a \
                 single-line array). Add these entries to that array by hand, then run this again:\n{}",
                path.display(),
                missing
                    .iter()
                    .map(|entry| format!("    {entry}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    };
    write_if_changed(&path, &merged).map(|()| true)
}

/// Insert entries into a single-line `rustflags` array, or `None` for a shape this action refuses.
/// 把条目插进单行的 `rustflags` 数组；形状无法处理时返回 `None`。
fn merge_rustflags(text: &str, missing: &[&String]) -> Option<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut in_build = false;
    for line in &mut lines {
        let trimmed = line.trim_start().to_owned();
        if trimmed.starts_with('[') {
            in_build = trimmed == "[build]";
            if in_build {
                continue;
            }
        }
        if !in_build || !trimmed.starts_with("rustflags") {
            continue;
        }
        let (_, value) = trimmed.split_once('=')?;
        let value = value.trim();
        if !value.starts_with('[') || !value.ends_with(']') {
            return None;
        }
        let inner = &value[1..value.len() - 1];
        let existing: Vec<&str> = inner
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .collect();
        let mut combined: Vec<String> = existing.iter().map(|item| (*item).to_owned()).collect();
        combined.extend(missing.iter().map(|entry| (*entry).clone()));
        let indent = line.len() - line.trim_start().len();
        *line = format!(
            "{}rustflags = [{}]",
            " ".repeat(indent),
            combined.join(", ")
        );
        return Some(lines.join("\n") + "\n");
    }
    // No `[build]` section at all: append one.
    let mut merged = text.trim_end().to_owned();
    merged.push_str("\n\n[build]\nrustflags = [");
    merged.push_str(
        &missing
            .iter()
            .map(|entry| (*entry).clone())
            .collect::<Vec<_>>()
            .join(", "),
    );
    merged.push_str("]\n");
    Some(merged)
}

/// A path relative to the workspace root, with `/` separators.
/// 相对工作区根的路径，使用 `/` 分隔符。
fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
#[path = "crate_write_tests.rs"]
mod crate_write_tests;
