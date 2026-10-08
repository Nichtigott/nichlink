//! Recoverable folder-backed module migration.
//! 可恢复的目录型模块迁移。

use super::*;

/// Rename a standard generated module and every authored child below it.
/// 重命名标准生成模块，并同步迁移其下的全部注册子对象。
pub(super) fn migrate_module_subtree(
    registry: &Registry,
    id: NodeId,
    source: PathBuf,
    mut face: FaceManifest,
    requested_module: &str,
) -> Result<AuthoringChange, String> {
    let old_dir = source
        .parent()
        .ok_or_else(|| "generated source has no module directory".to_owned())?
        .to_path_buf();
    let new_dir = old_dir
        .parent()
        .ok_or_else(|| "generated module directory has no parent".to_owned())?
        .join(requested_module);
    let old_file_name = source
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if old_file_name != face.values.get("module").map(String::as_str).unwrap_or("")
        || old_dir.file_name().and_then(|name| name.to_str()) != Some(old_file_name)
    {
        return Err("only standard `<module>/<module>.rs` faces can be renamed".to_owned());
    }
    if new_dir.exists() {
        return Err(format!("module `{requested_module}` already exists"));
    }
    let source_root = source_root();
    let old_relative = old_dir
        .strip_prefix(&source_root)
        .map_err(|_| "generated module is outside the package source tree".to_owned())?;
    let new_relative = old_relative
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join(requested_module);
    let old_module_path = normalized_path(old_relative).replace('/', "::");
    let new_module_path = normalized_path(&new_relative).replace('/', "::");

    face.values
        .insert("module".to_owned(), requested_module.to_owned());
    face.values.insert(
        "source".to_owned(),
        normalized_path(&new_relative.join(format!("{requested_module}.rs"))),
    );
    if let Some(rule_path) = face.values.get_mut("registry_rule_path") {
        let old_prefix = format!("src/{}/", normalized_path(old_relative));
        let new_prefix = format!("src/{}/", normalized_path(&new_relative));
        if rule_path.starts_with(&old_prefix) {
            *rule_path = rule_path.replacen(&old_prefix, &new_prefix, 1);
        }
    }

    fs::rename(&old_dir, &new_dir).map_err(|error| {
        format!(
            "cannot rename {} to {}: {error}",
            old_dir.display(),
            new_dir.display()
        )
    })?;
    let moved_source = new_dir.join(format!("{old_file_name}.rs"));
    let renamed_source = new_dir.join(format!("{requested_module}.rs"));
    if let Err(error) = fs::rename(&moved_source, &renamed_source) {
        let _ = fs::rename(&new_dir, &old_dir);
        return Err(format!("cannot rename module source file: {error}"));
    }
    let mut originals = Vec::new();
    if let Err(error) = rewrite_migrated_sources(
        &new_dir,
        &source,
        &face,
        MigrationPaths {
            old_module_path: &old_module_path,
            new_module_path: &new_module_path,
            old_relative,
            new_relative: &new_relative,
        },
        &mut originals,
    ) {
        rollback_migration(
            &new_dir,
            &old_dir,
            &originals,
            old_file_name,
            requested_module,
        );
        return Err(error);
    }

    let snapshots = match generated_snapshots() {
        Ok(snapshots) => snapshots
            .into_iter()
            .filter(|snapshot| {
                let prefix = format!("{}/", normalized_path(&new_relative));
                snapshot.source.file
                    == normalized_path(&new_relative.join(format!("{requested_module}.rs")))
                    || snapshot.source.file.starts_with(&prefix)
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            rollback_migration(
                &new_dir,
                &old_dir,
                &originals,
                old_file_name,
                requested_module,
            );
            return Err(error);
        }
    };
    if snapshots.is_empty() {
        rollback_migration(
            &new_dir,
            &old_dir,
            &originals,
            old_file_name,
            requested_module,
        );
        return Err("renamed module produced no registration face".to_owned());
    }
    if let Err(error) = registry.validate_snapshot_migration(id, snapshots) {
        rollback_migration(
            &new_dir,
            &old_dir,
            &originals,
            old_file_name,
            requested_module,
        );
        return Err(format!("registration rejected after rename:\n{error}"));
    }

    Ok(AuthoringChange {
        message: format!("renamed module `{old_file_name}` to `{requested_module}`"),
        source: new_dir.join(format!("{requested_module}.rs")),
    })
}

struct MigrationPaths<'a> {
    old_module_path: &'a str,
    new_module_path: &'a str,
    old_relative: &'a Path,
    new_relative: &'a Path,
}

fn rewrite_migrated_sources(
    new_dir: &Path,
    old_source: &Path,
    root_face: &FaceManifest,
    paths: MigrationPaths<'_>,
    originals: &mut Vec<(PathBuf, String)>,
) -> Result<(), String> {
    let old_source_name = old_source
        .file_name()
        .ok_or_else(|| "generated source has no file name".to_owned())?;
    let new_source = new_dir.join(old_source_name).with_file_name(format!(
        "{}.rs",
        root_face
            .values
            .get("module")
            .map(String::as_str)
            .unwrap_or_default()
    ));
    let old_prefix = format!("src/{}/", normalized_path(paths.old_relative));
    let new_prefix = format!("src/{}/", normalized_path(paths.new_relative));
    let rust_old = format!("crate::{}", paths.old_module_path);
    let rust_new = format!("crate::{}", paths.new_module_path);
    let mut files = Vec::new();
    visit_rust_files(new_dir, &mut files)?;
    for path in files {
        let original = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        originals.push((path.clone(), original.clone()));
        let mut updated = replace_source_prefix(
            &replace_module_path(&original, &rust_old, &rust_new),
            &old_prefix,
            &new_prefix,
        );
        if path == new_source {
            updated = replace_source_prefix(
                &replace_module_path(&root_face.render_source()?, &rust_old, &rust_new),
                &old_prefix,
                &new_prefix,
            );
        }
        if updated != original {
            atomic_write(&path, &updated)?;
        }
    }
    Ok(())
}

/// Replace a Rust module path only where it is a whole sequence of `::` segments.
/// 只在整段 `::` 序列处替换 Rust 模块路径。
///
/// A plain `str::replace` matched text prefixes, so renaming `control` rewrote a
/// sibling's `crate::run_method::control_extra::NODE_ID` into `crate::run_method::widget_extra::NODE_ID` —
/// a module silently pointing at something that does not exist, with the write
/// reported as "changed". The boundary is a `::` segment, which is the same rule
/// `build_method`'s scope view states for subtree selection (`control` keeps
/// `control::object::button` and never `control_extra`).
///
/// One suffix belongs to the renamed module itself rather than to a longer
/// sibling name: the generated alias `{module}_object!` names the very face whose
/// module moved. A `::`-only rule left `crate::run_method::control_object!` behind inside a
/// file whose parent had become `widget`, and the parser then refused that file by
/// name (`registration macro \`control_object!\` does not match parent \`widget\``).
/// The alias is matched together with its `!`, so a sibling's
/// `control_extra_object!` is still left alone.
/// 有一个后缀属于被改名的模块自己，而不属于更长的兄弟名：生成的别名 `{module}_object!` 指的正是
/// 模块搬走了的那个面。只认 `::` 的规则会把 `crate::run_method::control_object!` 留在父级已变成 `widget` 的
/// 文件里，解析器随后按名拒绝该文件（`registration macro \`control_object!\` does not match
/// parent \`widget\``）。别名连同它的 `!` 一起匹配，因此兄弟的 `control_extra_object!` 仍然原样保留。
/// 朴素的 `str::replace` 匹配文本前缀，因此把 `control` 改名会把兄弟模块的
/// `crate::run_method::control_extra::NODE_ID` 改写成 `crate::run_method::widget_extra::NODE_ID`——一个悄悄指向不存在
/// 东西的模块，而写入被报成"已改变"。边界是 `::` 段，这正是 `build_method` 的作用域视图为子树选择
/// 写下的规则（`control` 保留 `control::object::button`，绝不保留 `control_extra`）。
fn replace_module_path(text: &str, old: &str, new: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find(old) {
        let starts_at_a_boundary = index == 0 || !is_identifier_byte(rest.as_bytes()[index - 1]);
        let after = index + old.len();
        let ends_at_a_boundary = after >= rest.len()
            || !is_identifier_byte(rest.as_bytes()[after])
            || rest[after..].starts_with("_object!");
        output.push_str(&rest[..index]);
        output.push_str(if starts_at_a_boundary && ends_at_a_boundary {
            new
        } else {
            old
        });
        rest = &rest[after..];
    }
    output.push_str(rest);
    output
}

/// Whether a byte can be part of an identifier or a module segment.
/// 某个字节能否属于一个标识符或模块段。
fn is_identifier_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// Replace a `src/<module>/` source prefix only where it starts a path.
/// 只在路径起始处替换 `src/<module>/` 源前缀。
///
/// The other half of a migration rewrite is bounded by `::` segments; this half is a
/// path, so its bound is the byte in front of it. A path that merely *contains*
/// `src/control/` — a sibling pointing at another tree, e.g. `dep/src/control/x.rs` —
/// keeps its own meaning, and the trailing `/` is the far bound, which is why
/// `src/control_extra/` was never at risk from this one. A plain `str::replace` had
/// no bound on the left at all, so the whole migrated subtree (a sibling included)
/// could be rewritten silently (audit `LGC-LG-10`).
/// 迁移改写的另一半以 `::` 段为界；这一半是路径，它的界是它前面的那个字节。只是**包含**
/// `src/control/` 的路径——例如兄弟模块指向另一棵树的 `dep/src/control/x.rs`——保持自己的含义；
/// 尾部的 `/` 是另一侧的界，因此 `src/control_extra/` 从来不受这一处影响。朴素的 `str::replace`
/// 左侧完全没有界，于是整棵被迁移的子树（包括兄弟模块）都可能被静默改写（审计 `LGC-LG-10`）。
fn replace_source_prefix(text: &str, old: &str, new: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find(old) {
        let starts_a_path = index == 0 || !is_path_byte(rest.as_bytes()[index - 1]);
        output.push_str(&rest[..index]);
        output.push_str(if starts_a_path { new } else { old });
        rest = &rest[index + old.len()..];
    }
    output.push_str(rest);
    output
}

/// Whether a byte can be part of a path.
/// 某个字节能否属于路径。
fn is_path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'/' | b'\\' | b'-')
}

#[cfg(test)]
#[path = "migration_tests.rs"]
mod migration_tests;

fn visit_rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            visit_rust_files(&path, files)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn rollback_migration(
    new_dir: &Path,
    old_dir: &Path,
    originals: &[(PathBuf, String)],
    old_name: &str,
    new_name: &str,
) {
    for (path, contents) in originals.iter().rev() {
        let _ = atomic_write(path, contents);
    }
    let _ = fs::rename(new_dir, old_dir);
    let _ = fs::rename(
        old_dir.join(format!("{new_name}.rs")),
        old_dir.join(format!("{old_name}.rs")),
    );
}

/// Change a face kind by migrating its identity in one validated transaction.
/// 修改注册面的 kind，并在一次校验事务中迁移其身份。
///
/// `kind` participates in `NodeId`, so changing it cannot use an in-place
/// replacement. The source file stays where it is; descendants are reparsed
/// so their parent IDs follow the new kind, then the complete subtree is
/// validated before the caller reloads the live registry.
/// `kind` 是 `NodeId` 的组成部分，不能原地替换。源码文件位置保持不变，
/// 重新解析后代以跟随新的父级身份，并在刷新实时注册树前校验整棵子树。
pub(super) fn migrate_kind_subtree(
    registry: &Registry,
    id: NodeId,
    source: PathBuf,
    face: FaceManifest,
) -> Result<AuthoringChange, String> {
    let old_source = fs::read_to_string(&source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
    let rendered = face.render_source()?;
    atomic_write(&source, &rendered)?;

    let source_root = source_root();
    let relative = source
        .strip_prefix(&source_root)
        .map_err(|_| "generated module is outside the package source tree".to_owned())?;
    let relative = normalized_path(relative);
    let directory = Path::new(&relative)
        .parent()
        .map(normalized_path)
        .unwrap_or_default();
    let prefix = if directory.is_empty() {
        String::new()
    } else {
        format!("{directory}/")
    };
    let snapshots = match generated_snapshots() {
        Ok(snapshots) => snapshots
            .into_iter()
            .filter(|snapshot| {
                snapshot.source.file == relative
                    || (!prefix.is_empty() && snapshot.source.file.starts_with(&prefix))
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            let _ = atomic_write(&source, &old_source);
            return Err(error);
        }
    };
    if snapshots.is_empty() {
        let _ = atomic_write(&source, &old_source);
        return Err("kind migration produced no registration face".to_owned());
    }
    if let Err(error) = registry.validate_snapshot_migration(id, snapshots) {
        let _ = atomic_write(&source, &old_source);
        return Err(format!("registration rejected after kind change:\n{error}"));
    }

    Ok(AuthoringChange {
        message: format!("updated kind in {}", source.display()),
        source,
    })
}
