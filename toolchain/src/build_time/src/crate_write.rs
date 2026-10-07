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

use super::crate_facade::PlannedFacade;
use super::crate_members;
use super::crate_plan::{GENERATED_MARKER, PlannedCrate};
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
    /// Whether the workspace manifest's `members` was touched (false when there is no enclosing
    /// workspace, or when it already listed every generated package).
    /// 工作区清单的 `members` 是否被改动（没有外层工作区、或它已经列出每一个生成包时是 false）。
    pub(crate) members_changed: bool,
}

/// Write every planned crate, then make sure the workspace config carries the remap and the
/// workspace manifest lists the new packages.
/// 写下每个已规划的 crate，然后确保工作区配置带着那些 remap、且工作区清单列出了这些新包。
///
/// `root` is the directory cargo reads config from when it builds the generated packages, and
/// `workspace` is the workspace manifest they have to be members of — `None` when the host is not in
/// one. Two callers pass both — the write half and the revert half — so the decision lives in one
/// place: a package generated beside a host that belongs to a workspace has to be a member of it, and
/// without a workspace the generated packages are standalone siblings with nothing to materialize
/// them into.
/// `root` 是 cargo 构建这些生成包时读取配置的目录，`workspace` 是它们必须成为成员的那份工作区清单——宿主
/// 不在任何工作区里时是 `None`。两个调用方都传这两样——写入那一半与撤回那一半——因此判断只有一处：生成在
/// 某个属于工作区的宿主旁边的包必须是它的成员，而没有工作区时，生成的包就是彼此独立的同级包，没有任何东西可以
/// 把它们物化进去。
pub(crate) fn write_partition(
    root: &Path,
    workspace: Option<&Path>,
    planned: &[PlannedCrate],
    facade: Option<&PlannedFacade>,
) -> Result<Written, String> {
    let mut written = Written::default();
    let mut remap: Vec<(String, String)> = Vec::new();
    let mut directories: Vec<(&Path, &str)> = planned
        .iter()
        .map(|planned| (planned.directory.as_path(), planned.package.as_str()))
        .collect();
    if let Some(facade) = facade {
        directories.push((facade.directory.as_path(), facade.package.as_str()));
    }
    for (directory, package) in &directories {
        let directory = *directory;
        if !directory.exists() {
            continue;
        }
        let marker = [directory.join("Cargo.toml"), directory.join("src/lib.rs")]
            .iter()
            .any(|path| {
                fs::read_to_string(path)
                    .map(|text| text.contains(GENERATED_MARKER))
                    .unwrap_or(false)
            });
        if !marker {
            return Err(format!(
                "add_crates: {package} at {} does not look like a package this action created (no \
                 `{GENERATED_MARKER}` line in its Cargo.toml or src/lib.rs), so it is left alone.\
                 \nway forward: remove it by hand if it really is a leftover",
                directory.display()
            ));
        }
        fs::remove_dir_all(directory).map_err(|error| {
            format!("add_crates: cannot remove {}: {error}", directory.display())
        })?;
        written.files.push(relative(root, directory));
    }
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
    written.config_changed = merge_workspace_config(root, &remap)?;
    // The member list is **computed** here for the same reason and **written** after the packages
    // exist: a refusal leaves the tree untouched, while a list naming a package that is not there is
    // the state cargo refuses to load (audit `M7`, P3.5).
    // 成员清单在这里**算出来**是同一个理由，而**写**在包存在之后：拒绝让树保持原样，而一份点名了不存在的包的
    // 清单是 cargo 拒绝加载的状态（审计 `M7`，P3.5）。
    let members = match workspace {
        Some(workspace_root) => {
            let directories: Vec<&Path> = directories
                .iter()
                .map(|(directory, _)| *directory)
                .collect();
            let entries = crate_members::entries(workspace_root, &directories);
            let path = workspace_root.join("Cargo.toml");
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("add_crates: cannot read {}: {error}", path.display()))?;
            crate_members::merged(&text, &entries)?.map(|text| (path, text))
        }
        None => None,
    };
    // The facade is one more package with the same three files, so it goes through the same writer —
    // and a partition that hands work away always has one, because handing work away is what makes the
    // cross-crate half exist (audit `M7`, §M7.33).
    // facade 就是再多一个、同样三份文件的包，因此走同一个写入方——而一次交出工作的拆分总有它，因为"交出工作"
    // 正是跨 crate 那一半存在的原因（审计 `M7`，§M7.33）。
    let mut packages: Vec<(&Path, &str, &str, &str)> = planned
        .iter()
        .map(|planned| {
            (
                planned.directory.as_path(),
                planned.cargo_toml.as_str(),
                planned.lib_rs.as_str(),
                planned.build_rs.as_str(),
            )
        })
        .collect();
    if let Some(facade) = facade {
        packages.push((
            facade.directory.as_path(),
            facade.cargo_toml.as_str(),
            facade.lib_rs.as_str(),
            facade.build_rs.as_str(),
        ));
    }
    for (directory, cargo_toml, lib_rs, build_rs) in packages {
        for (path, content) in [
            (directory.join("Cargo.toml"), cargo_toml),
            (directory.join("src/lib.rs"), lib_rs),
            (directory.join("build.rs"), build_rs),
        ] {
            write_ghost_file(&path, content)?;
            written.files.push(relative(root, &path));
        }
    }
    if let Some((path, text)) = members {
        write_ghost_file(&path, &text)?;
        written.members_changed = true;
    }
    written.files.sort();
    written.files.dedup();
    Ok(written)
}

/// Take a partition back: remove the ghost packages this action created, and its config entries.
/// 把一次拆分撤回：删掉本动作创建的幽灵包，以及它加进配置里的那些条目。
///
/// Two lines are held here, and both matter more on the way out than on the way in:
/// 这里守两条线，而两条在**撤出**时比进入时更要紧：
///
/// - **A directory this action did not create is never removed.** The generated `Cargo.toml`/`lib.rs`
///   carry a `Generated by NichLink` line; a directory without one is refused by name, because the
///   alternative is a tool deleting somebody's package that happens to sit where a ghost would.
/// - **配置文件只减去它自己加的那些条目**, and a file that is left with nothing but headers this action
///   added is removed rather than left as an empty shell.
/// - **本动作没有创建的目录永不删除**。生成的 `Cargo.toml`/`lib.rs` 带着 `Generated by NichLink` 一行；
///   没有这一行的目录会被点名拒绝——另一种做法是工具删掉别人的包，只因为它恰好躺在幽灵会在的位置。
/// - **配置文件只减去它自己加的那些条目**，而只剩本动作加进去的表头的文件会被删掉，而不是留一个空壳。
pub(crate) fn revert_partition(
    root: &Path,
    workspace: Option<&Path>,
    planned: &[PlannedCrate],
    facade: Option<&PlannedFacade>,
) -> Result<Written, String> {
    let mut written = Written::default();
    let mut remap: Vec<(String, String)> = Vec::new();
    // One list of packages to take back: every ghost this action planned, plus the facade, which is
    // planned as the host's sibling and carries the cross-crate half (audit `M7`, §M7.33).
    // 一份要撤回的包清单：本动作规划的每个幽灵，加上 facade——它被规划成宿主的同级目录、承载跨 crate 那一半
    // （审计 `M7`，§M7.33）。
    let mut directories: Vec<(&Path, &str)> = planned
        .iter()
        .map(|planned| (planned.directory.as_path(), planned.package.as_str()))
        .collect();
    if let Some(facade) = facade {
        directories.push((facade.directory.as_path(), facade.package.as_str()));
    }
    // The member list is stripped **first**, and only in memory: taking the packages out and then
    // refusing the manifest edit would leave a list naming packages that no longer exist, which is
    // exactly the state cargo refuses to load (audit `M7`, P3.5).
    // 成员清单**先**剥、而且只在内存里剥：把包拿掉之后再拒绝清单编辑，会留下一份点名了已不存在的包的清单，
    // 而那正是 cargo 拒绝加载的状态（审计 `M7`，P3.5）。
    let members = match workspace {
        Some(workspace_root) => {
            let paths: Vec<&Path> = directories
                .iter()
                .map(|(directory, _)| *directory)
                .collect();
            let entries = crate_members::entries(workspace_root, &paths);
            let path = workspace_root.join("Cargo.toml");
            let text = fs::read_to_string(&path)
                .map_err(|error| format!("add_crates: cannot read {}: {error}", path.display()))?;
            crate_members::stripped(&text, &entries)?.map(|text| (path, text))
        }
        None => None,
    };
    for (directory, package) in directories {
        if !directory.exists() {
            continue;
        }
        let marker = [directory.join("Cargo.toml"), directory.join("src/lib.rs")]
            .iter()
            .any(|path| {
                fs::read_to_string(path)
                    .map(|text| text.contains(GENERATED_MARKER))
                    .unwrap_or(false)
            });
        if !marker {
            return Err(format!(
                "add_crates: {package} at {} does not look like a package this action created (no \
                 `{GENERATED_MARKER}` line in its Cargo.toml or src/lib.rs), so it is left alone.\
                 \nway forward: remove it by hand if it really is a leftover",
                directory.display()
            ));
        }
        fs::remove_dir_all(directory).map_err(|error| {
            format!("add_crates: cannot remove {}: {error}", directory.display())
        })?;
        written.files.push(relative(root, directory));
    }
    for planned in planned {
        remap.extend(planned.remap.iter().cloned());
    }
    remap.sort();
    remap.dedup();
    written.config_changed = strip_workspace_config(root, &remap)?;
    if let Some((path, text)) = members {
        write_ghost_file(&path, &text)?;
        written.members_changed = true;
    }
    written.files.sort();
    written.files.dedup();
    Ok(written)
}

/// Remove this action's `--remap-path-prefix` entries from the workspace config.
/// 从工作区配置里移掉本动作加的 `--remap-path-prefix` 条目。
///
/// Returns whether the file changed. The same shapes the forward merge handles are handled here, and
/// the same unrecognizable shape is refused — with the entries to delete by hand.
/// 返回文件是否被改动。正向合并处理的那几种形状在这里同样处理，同样无法辨认的形状同样被拒绝——并给出该手动
/// 删掉的那些条目。
fn strip_workspace_config(
    workspace_root: &Path,
    remap: &[(String, String)],
) -> Result<bool, String> {
    let path = workspace_root.join(".cargo/config.toml");
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(false);
    };
    let prefixes: Vec<String> = remap
        .iter()
        .map(|(from, to)| format!("--remap-path-prefix={from}={to}"))
        .collect();
    if !prefixes.iter().any(|prefix| text.contains(prefix.as_str())) {
        return Ok(false);
    }
    let stripped = match strip_rustflags(&text, &prefixes) {
        Some(stripped) => stripped,
        None => {
            return Err(format!(
                "add_crates: {} has a `rustflags` this action will not rewrite (it is not a single-line \
                 array). Delete these entries by hand:\n{}",
                path.display(),
                prefixes
                    .iter()
                    .map(|prefix| format!("    \"{prefix}\"", prefix = prefix))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
    };
    if nothing_but_our_shell(&stripped) {
        return fs::remove_file(&path)
            .map(|()| true)
            .map_err(|error| format!("cannot remove {}: {error}", path.display()));
    }
    write_if_changed(&path, &stripped).map(|()| true)
}

/// Take our entries out of a single-line `rustflags` array, or `None` for a shape this action refuses.
/// 把我们的条目从单行 `rustflags` 数组里拿掉；形状无法处理时返回 `None`。
fn strip_rustflags(text: &str, prefixes: &[String]) -> Option<String> {
    let mut lines: Vec<String> = text.lines().map(str::to_owned).collect();
    let mut in_build = false;
    for line in &mut lines {
        let trimmed = line.trim_start().to_owned();
        if trimmed.starts_with('[') {
            in_build = trimmed == "[build]";
            continue;
        }
        if !in_build || !trimmed.starts_with("rustflags") {
            continue;
        }
        let (_, value) = trimmed.split_once('=')?;
        let value = value.trim();
        if !value.starts_with('[') || !value.ends_with(']') {
            return None;
        }
        let kept: Vec<String> = value[1..value.len() - 1]
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .filter(|item| {
                let bare = item.trim_matches('"');
                !prefixes.iter().any(|prefix| bare == prefix)
            })
            .map(str::to_owned)
            .collect();
        let indent = line.len() - line.trim_start().len();
        if kept.is_empty() {
            *line = String::new();
        } else {
            *line = format!("{}rustflags = [{}]", " ".repeat(indent), kept.join(", "));
        }
        return Some(lines.join("\n") + "\n");
    }
    Some(text.to_owned())
}

/// Whether a config is now only the shell this action would have added, and can therefore go.
/// 配置现在是否只剩本动作会加的那层壳，因而可以被删掉。
fn nothing_but_our_shell(text: &str) -> bool {
    text.lines().all(|line| {
        let trimmed = line.trim();
        trimmed.is_empty() || trimmed.starts_with('#') || trimmed == "[build]"
    })
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
