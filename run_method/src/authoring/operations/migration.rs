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
    collect_rust_files(new_dir, &mut files)?;
    for path in files {
        let original = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        originals.push((path.clone(), original.clone()));
        let mut updated = original
            .replace(&rust_old, &rust_new)
            .replace(&old_prefix, &new_prefix);
        if path == new_source {
            updated = root_face
                .render_source()?
                .replace(&old_prefix, &new_prefix)
                .replace(&rust_old, &rust_new);
        }
        if updated != original {
            atomic_write(&path, &updated)?;
        }
    }
    Ok(())
}

fn collect_rust_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            collect_rust_files(&path, files)?;
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
