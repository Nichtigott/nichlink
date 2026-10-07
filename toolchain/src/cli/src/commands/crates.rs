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
use std::path::PathBuf;

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
    let mut release = false;
    for arg in args.by_ref() {
        match arg.as_str() {
            "-h" | "--help" => return crate::cli::usage(out),
            "--check" => {
                write = false;
                revert = false;
            }
            "--write" => write = true,
            "--revert" => revert = true,
            // The development shape mounts a fragment's files out of the host package; the release
            // shape copies them in, because crates.io rejects a package whose `#[path]` reaches
            // outside it (audit `M7`, §M7.42). The two shapes land in the same directories, so one is
            // reverted before the other is written.
            // 开发形状把碎片的文件从宿主包里挂载出来；发布形状把它们复制进去，因为 crates.io 拒绝 `#[path]`
            // 伸到包外的包（审计 `M7`，§M7.42）。两个形状落在同一批目录里，因此写一个之前要先撤回另一个。
            "--release" => release = true,
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
    if release {
        for planned in &planned {
            line(
                out,
                format!(
                    "crate {} at {} (release shape, {} copied file(s))",
                    planned.package,
                    planned.directory.display(),
                    planned.mounts.len() + 1
                ),
            )?;
        }
        line(
            out,
            format!(
                "facade {}-facade at {} (release shape, resolves the host through cargo at build time)",
                declaration.package_prefix,
                package_root
                    .parent()
                    .unwrap_or(&package_root)
                    .join(format!("{}-facade", declaration.package_prefix))
                    .display(),
            ),
        )?;
    } else {
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
    }
    let (root, workspace) = crate::build_time::partition_roots(&package_root);
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
            if release {
                "preview only: pass --write to create these packages in the release shape"
                    .to_owned()
            } else {
                "preview only: pass --write to create these packages and merge the workspace config"
                    .to_owned()
            },
        )?;
        return Ok(());
    }
    if release {
        let mut packages = Vec::new();
        for planned in &planned {
            packages.push(crate::build_time::plan_release_ghost(
                &package_root,
                planned,
                &faces,
            )?);
        }
        packages.push(crate::build_time::plan_release_facade(
            &package_root,
            &package,
            &declaration.package_prefix,
            &package,
            &planned,
        )?);
        crate::build_time::guard_shape(
            &packages
                .iter()
                .map(|package| package.directory.as_path())
                .collect::<Vec<_>>(),
            true,
        )?;
        let written = crate::build_time::write_release(&root, workspace.as_deref(), &packages)?;
        return line(
            out,
            format!(
                "wrote {} file(s) under {}; the release shape carries its own sources, so there is \
                 nothing to remap{}",
                written.files.len(),
                root.display(),
                members_reply(written.members_changed, workspace.is_some())
            ),
        );
    }
    crate::build_time::guard_shape(
        &planned
            .iter()
            .map(|planned| planned.directory.as_path())
            .collect::<Vec<_>>(),
        false,
    )?;
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
