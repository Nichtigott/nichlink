//! Validation and atomic activation for graft drafts.
//! graft 草稿的校验与原子激活。

use std::fs;
use std::path::Path;

use crate::{RegistrationSnapshot, Registry, replace_face_macro};

use super::super::operations::generated_paths;
use super::super::{AuthoringChange, FaceManifest};
use super::GraftDraft;

/// Re-read a draft and validate its active form without touching source files.
/// 重新读取草稿并校验其激活形态，不改动正式源码。
pub fn validate_graft_draft(
    registry: &Registry,
    draft: &GraftDraft,
) -> Result<RegistrationSnapshot, String> {
    let target = registry
        .find(draft.target)
        .ok_or_else(|| format!("graft target `{}` is not registered", draft.target))?;
    let (_, target_source) = generated_paths(registry, draft.target)?;
    let mut face = FaceManifest::parse_source(&draft.source)?;
    normalize_for_target(&mut face, target, &target_source)?;
    let snapshot = target.clone().merge_authored(face.to_snapshot()?);
    if !is_attached_module(&target_source)
        && (snapshot.needs_registry != target.needs_registry
            || snapshot.registry_rule != target.registry_rule)
    {
        return Err(
            "a single-file graft cannot change its child Registry contract; migrate it to the standard folder-backed layout first"
                .to_owned(),
        );
    }
    registry
        .validate_snapshot_replacement(draft.target, snapshot.clone())
        .map_err(|error| format!("graft candidate rejected:\n{error}"))?;
    Ok(snapshot)
}

/// Atomically replace the target implementation with a validated draft.
/// 校验通过后，以原子目录替换激活 graft 草稿。
pub fn apply_graft_draft(
    registry: &mut Registry,
    draft: &GraftDraft,
) -> Result<AuthoringChange, String> {
    let snapshot = validate_graft_draft(registry, draft)?;
    let (_, target_source) = generated_paths(registry, draft.target)?;
    let target_dir = target_source
        .parent()
        .ok_or_else(|| "graft target source has no module directory".to_owned())?;
    let draft_dir = draft
        .source
        .parent()
        .ok_or_else(|| "graft draft source has no module directory".to_owned())?;
    let source = fs::read_to_string(&draft.source).map_err(|error| {
        format!(
            "cannot read graft draft {}: {error}",
            draft.source.display()
        )
    })?;
    let mut face = FaceManifest::parse_source(&draft.source)?;
    let target = registry
        .find(draft.target)
        .ok_or_else(|| format!("graft target `{}` is not registered", draft.target))?;
    normalize_for_target(&mut face, target, &target_source)?;
    let active_declaration = face.render_source()?;
    let active_source = replace_face_macro(&source, &active_declaration)
        .map_err(|error| format!("cannot activate graft declaration: {error}"))?;

    let mut staged_registry = registry.clone();
    staged_registry
        .apply_snapshot_replacement(draft.target, snapshot)
        .map_err(|error| error.to_string())?;
    if !is_attached_module(&target_source) {
        apply_single_file_graft(&target_source, draft, &active_source)?;
        *registry = staged_registry;
        return Ok(applied_change(draft, target_source));
    }

    let staging = target_dir.with_extension(format!("nichlink-graft-{}", std::process::id()));
    let backup = draft.root.join("original");
    if staging.exists() || backup.exists() {
        return Err("graft staging or backup path already exists".to_owned());
    }
    copy_tree(target_dir, &staging)?;
    overlay_draft(draft_dir, &staging, &draft.source)?;
    fs::write(
        staging.join(
            target_source
                .file_name()
                .ok_or_else(|| "graft target has no source filename".to_owned())?,
        ),
        active_source,
    )
    .map_err(|error| format!("cannot stage graft source: {error}"))?;
    if let Some(rule_source) = face.render_rule_source()? {
        let rule = face.rule_source_path()?;
        let relative = rule
            .strip_prefix(target_dir)
            .map_err(|_| "graft rule escaped the target module".to_owned())?;
        let staged_rule = staging.join(relative);
        fs::create_dir_all(staged_rule.parent().expect("rule has a parent"))
            .map_err(|error| format!("cannot stage graft rule: {error}"))?;
        fs::write(staged_rule, rule_source)
            .map_err(|error| format!("cannot stage graft rule: {error}"))?;
    }

    fs::rename(target_dir, &backup)
        .map_err(|error| format!("cannot back up graft target: {error}"))?;
    if let Err(error) = fs::rename(&staging, target_dir) {
        let _ = fs::rename(&backup, target_dir);
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("cannot activate graft source: {error}"));
    }
    *registry = staged_registry;
    Ok(applied_change(draft, target_source))
}

pub(super) fn normalize_for_target(
    face: &mut FaceManifest,
    target: &RegistrationSnapshot,
    target_source: &Path,
) -> Result<(), String> {
    if face.values.get("kind").map(String::as_str) != Some(target.kind.as_str()) {
        return Err(format!(
            "graft draft kind must remain `{}` so the target identity and child links stay stable",
            target.kind
        ));
    }
    let module = target_source
        .file_stem()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "graft target has an invalid module name".to_owned())?;
    let parent = target.parent.to_string();
    for (key, value) in [
        ("namespace", target.namespace.as_str()),
        ("module", module),
        ("source", target.source.file.as_str()),
        ("parent_node", parent.as_str()),
        ("registry_name", target.registry_name.as_str()),
        ("registry_rule_path", target.registry_rule_path.as_str()),
    ] {
        face.values.insert(key.to_owned(), value.to_owned());
    }
    Ok(())
}

fn applied_change(draft: &GraftDraft, source: std::path::PathBuf) -> AuthoringChange {
    AuthoringChange {
        message: format!(
            "applied graft draft `{}`; previous source kept at {}",
            draft.name,
            draft.root.join("original").display()
        ),
        source,
    }
}

fn apply_single_file_graft(target: &Path, draft: &GraftDraft, source: &str) -> Result<(), String> {
    let staging = target.with_extension(format!("nichlink-graft-{}", std::process::id()));
    let backup_dir = draft.root.join("original");
    let backup = backup_dir.join(
        target
            .file_name()
            .ok_or_else(|| "graft target has no source filename".to_owned())?,
    );
    if staging.exists() || backup.exists() {
        return Err("graft staging or backup path already exists".to_owned());
    }
    fs::create_dir_all(&backup_dir)
        .map_err(|error| format!("cannot create graft backup directory: {error}"))?;
    fs::write(&staging, source).map_err(|error| format!("cannot stage graft source: {error}"))?;
    fs::rename(target, &backup).map_err(|error| format!("cannot back up graft target: {error}"))?;
    if let Err(error) = fs::rename(&staging, target) {
        let _ = fs::rename(&backup, target);
        let _ = fs::remove_file(&staging);
        return Err(format!("cannot activate graft source: {error}"));
    }
    Ok(())
}

pub(super) fn is_attached_module(source: &Path) -> bool {
    source.file_stem().is_some() && source.file_stem() == source.parent().and_then(Path::file_name)
}

fn copy_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("cannot scan {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("cannot read source entry: {error}"))?;
        let path = entry.path();
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            copy_tree(&path, &target)?;
        } else {
            fs::copy(&path, &target).map_err(|error| {
                format!(
                    "cannot copy {} to {}: {error}",
                    path.display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

fn overlay_draft(source: &Path, destination: &Path, face: &Path) -> Result<(), String> {
    for entry in fs::read_dir(source)
        .map_err(|error| format!("cannot scan graft draft {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("cannot read graft draft entry: {error}"))?;
        let path = entry.path();
        if path == face || entry.file_name() == "object" {
            continue;
        }
        let target = destination.join(entry.file_name());
        if path.is_dir() {
            if target.exists() {
                overlay_draft(&path, &target, face)?;
            } else {
                copy_tree(&path, &target)?;
            }
        } else {
            fs::copy(&path, &target).map_err(|error| {
                format!(
                    "cannot stage graft support file {} to {}: {error}",
                    path.display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}
