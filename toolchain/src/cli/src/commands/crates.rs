//! `nichlink crates`: plan the crate split a host declared, and optionally write it.
//! `nichlink crates`：规划宿主声明的 crate 拆分，并可选地把它写下来。
//!
//! The declaration (`add_crates.rs`) is an authoring input, so this is the authoring action's CLI
//! face: `--check` (the default) prints what would be created and refuses a shape that cannot work,
//! `--write` creates the ghost packages and merges the workspace-root config they need. Splitting is a
//! thing you commit, so the write half is **idempotent**: a second run rewrites nothing.
//! 声明（`add_crates.rs`）是创作输入，因此这是创作动作的 CLI 面：`--check`（默认）打印会创建什么、并对
//! 无法成立的形状点名拒绝；`--write` 创建幽灵包并合并它们所需的工作区根配置。拆分是要提交的东西，因此写入
//! 那一半是**幂等**的：第二遍什么都不重写。

use std::io::Write;
use std::path::{Path, PathBuf};

use super::resolve_package;

/// Write one line, turning an I/O failure into the string this command returns.
/// 写一行，把 I/O 失败转成本命令返回的字符串。
fn line(out: &mut dyn Write, text: String) -> Result<(), String> {
    writeln!(out, "{text}").map_err(|error| error.to_string())
}

/// Plan (and with `--write`, create) the crates the host's declaration names.
/// 规划（并在 `--write` 时创建）宿主声明点名的那些 crate。
pub(crate) fn crates(
    args: &mut impl Iterator<Item = String>,
    out: &mut dyn Write,
) -> Result<(), String> {
    let mut directory: Option<PathBuf> = None;
    let mut write = false;
    let mut revert = false;
    for arg in args.by_ref() {
        match arg.as_str() {
            "-h" | "--help" => return crate::cli::usage(out),
            "--check" => {
                write = false;
                revert = false;
            }
            "--write" => write = true,
            "--revert" => revert = true,
            _ if arg.starts_with('-') => return Err(format!("unexpected argument '{arg}'")),
            _ if directory.is_none() => directory = Some(PathBuf::from(arg)),
            _ => return Err("crates accepts at most one project path".to_owned()),
        }
    }
    let directory = match directory {
        Some(directory) => directory,
        None => std::env::current_dir()
            .map_err(|error| format!("cannot read the current directory: {error}"))?,
    };
    let (package_root, package) = resolve_package(&directory.to_string_lossy())?;
    let declaration = crate::build_time::read_shape_declaration(&package_root)?.ok_or_else(|| {
        format!(
            "{} has no {}: a host without a declaration is one crate, so there is nothing to plan",
            package_root.display(),
            nichlink_kernel::lexicon::ADD_CRATES_FILE
        )
    })
            .map_err(|error| error.to_string())?;
    let out_dir = super::build_out_dir(&package_root);
    let rows = crate::build_time::read_pruning_manifest(&out_dir).map_err(|error| {
        format!("{error}\nway forward: run `nichlink check` first — the plan reads the faces the build published")
    })
            .map_err(|error| error.to_string())?;
    let faces: Vec<(String, String, crate::build_time::NodeId)> = rows
        .iter()
        .map(|row| {
            (
                row.source.clone(),
                crate::build_time::source_module_path(&row.source),
                row.id,
            )
        })
        .collect();
    let planned = crate::build_time::plan_crates(&package_root, &package, &declaration, &faces)?;
    let facade = crate::build_time::plan_facade(
        &package_root,
        &declaration.package_prefix,
        &package,
        &package,
        &planned,
    )?;
    if planned.is_empty() {
        return line(out, format!("nothing declared: {package} names no crate"));
    }
    for planned in &planned {
        line(
            out,
            format!(
                "crate {} at {} ({} mounted file(s), {} remap entr(ies))",
                planned.package,
                planned.directory.display(),
                planned.mounts.len(),
                planned.remap.len()
            ),
        )?;
    }
    if let Some(facade) = &facade {
        line(
            out,
            format!(
                "facade {} at {} (dependencies: {})",
                facade.package,
                facade.directory.display(),
                facade.dependencies.join(", ")
            ),
        )?;
    }
    let (root, workspace) = partition_roots(&package_root);
    if revert {
        let reverted = crate::build_time::revert_partition(
            &root,
            workspace.as_deref(),
            &planned,
            facade.as_ref(),
        )?;
        return line(
            out,
            format!(
                "removed {} generated package(s) under {}; workspace config {}{}",
                reverted.files.len(),
                root.display(),
                if reverted.config_changed {
                    "updated"
                } else {
                    "carried none of this action's entries"
                },
                members_reply(reverted.members_changed, workspace.is_some())
            ),
        );
    }
    if !write {
        line(
            out,
            "preview only: pass --write to create these packages and merge the workspace config"
                .to_owned(),
        )?;
        return Ok(());
    }
    let written =
        crate::build_time::write_partition(&root, workspace.as_deref(), &planned, facade.as_ref())?;
    line(
        out,
        format!(
            "wrote {} file(s) under {}; workspace config {}{}",
            written.files.len(),
            root.display(),
            if written.config_changed {
                "updated"
            } else {
                "already carried the remap"
            },
            members_reply(written.members_changed, workspace.is_some())
        ),
    )?;
    Ok(())
}

/// The member-list half of a write or revert reply.
/// 写入或撤回回复里属于成员清单的那一半。
fn members_reply(changed: bool, in_a_workspace: bool) -> String {
    if !in_a_workspace {
        return "; no enclosing workspace, so no member list".to_owned();
    }
    if changed {
        "; workspace members updated".to_owned()
    } else {
        "; workspace members already listed".to_owned()
    }
}

/// The directory cargo reads config from when it builds the generated packages, and the workspace
/// manifest they are materialized into (`None` when the host belongs to no workspace).
/// cargo 构建这些生成包时读取配置的目录，以及把它们物化进去的那份工作区清单（宿主不属于任何工作区时是
/// `None`）。
///
/// The config location is **not** the host package when there is no enclosing workspace: the generated
/// packages are the host's siblings, cargo finds config from the directory it is invoked in and that
/// directory's ancestors, and a `.cargo/config.toml` inside the host is never read while cargo builds
/// a sibling. Measured: with the config written inside the host, the remap never reached the ghost and
/// every mounted face failed `assert_static_identity` (audit `M7`, §M7.40). The directory both the host
/// and its siblings share is the parent.
/// 没有外层工作区时，配置的位置**不是**宿主包本身：生成的包是宿主的同级包，cargo 从它被调用的目录及其祖先
/// 目录找配置，而宿主的 `.cargo/config.toml` 在 cargo 构建一个同级包时永远不会被读到。实测：配置写在宿主
/// 里面时，remap 从未到达幽灵，于是每个被挂载的面都让 `assert_static_identity` 失败（审计 `M7`，§M7.40）。
/// 宿主与它的同级包共同拥有的那个目录，就是父目录。
///
/// Writing it there is not "leaking into an unrelated directory": the partition **puts its packages** in
/// that directory, so it is the partition's build area, and every rustflags entry this action adds is a
/// `--remap-path-prefix` scoped to one absolute host path — a no-op for anything else built from there.
/// An existing config in that directory is merged, never replaced, on the same terms as everywhere else.
/// 写在那里并不是"漏进一个无关目录"：拆分**把它的包放在**那个目录里，因此它就是这次拆分的构建区，而本动作
/// 加的每一条 rustflags 都是作用域限于某一个宿主绝对路径的 `--remap-path-prefix`——对从那里构建的其它东西
/// 是空操作。那里已有的配置会被合并、从不替换，条件与别处相同。
fn partition_roots(package_root: &Path) -> (PathBuf, Option<PathBuf>) {
    let workspace = workspace_root_of(package_root);
    if workspace == package_root {
        let parent = package_root.parent().unwrap_or(package_root).to_path_buf();
        (parent, None)
    } else {
        (workspace.clone(), Some(workspace))
    }
}

/// The outer-most ancestor that is a workspace, or the package itself when there is none.
/// 最外层的、是工作区的祖先目录；没有时就是包自己。
///
/// `rustflags` belong to the workspace root because they are per **invocation** and cargo finds config
/// from the current directory: a config inside the package would only apply when cargo is run from
/// there, which is not where a workspace builds from.
/// `rustflags` 属于工作区根，因为它们是每次调用生效的，而 cargo 从当前目录找 config：放在包里的 config 只在
/// 从那个目录跑 cargo 时才生效，而工作区并不是从那里构建的。
fn workspace_root_of(package_root: &Path) -> PathBuf {
    let mut root = package_root.to_path_buf();
    let mut current = package_root.parent();
    while let Some(parent) = current {
        let manifest = parent.join("Cargo.toml");
        let is_workspace = std::fs::read_to_string(&manifest)
            .map(|text| text.lines().any(|line| line.trim() == "[workspace]"))
            .unwrap_or(false);
        if is_workspace {
            root = parent.to_path_buf();
            current = parent.parent();
        } else {
            break;
        }
    }
    root
}
