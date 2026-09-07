//! Source-preserving graft candidate creation.
//! 保留实现源码的 graft 候选创建。

use std::fs;
use std::path::{Path, PathBuf};

use crate::{NodeId, ROOT_NODE_ID, Registry, replace_face_macro};

use super::filesystem::create_new;
use super::operations::{apply_new_face_values, generated_paths};
use super::validation::{
    is_parent_component, normalized_path, package_root, rule_path_for_source, validate_name,
};
use super::{AuthoringChange, FaceManifest, NewModuleFace};

#[path = "apply.rs"]
mod apply;
#[path = "draft.rs"]
mod draft;

pub use apply::{apply_graft_draft, validate_graft_draft};
use apply::{is_attached_module, normalize_for_target};
pub use draft::{GraftDraft, graft_drafts};

/// Copy one face's Rust implementation into a sibling graft candidate.
///
/// Only the registration declaration is regenerated. Structs, impl blocks,
/// functions, comments, and tests in the original file are preserved verbatim.
/// 复制一个注册面的 Rust 实现，创建同级 graft 候选。只重新生成注册声明；
/// 原文件中的 struct、impl、函数、注释和测试保持原样。
pub fn copy_module_for_graft(
    registry: &Registry,
    source_id: NodeId,
    configured: &NewModuleFace<'_>,
) -> Result<(AuthoringChange, GraftDraft), String> {
    validate_name(configured.module)?;
    let original = registry
        .find(source_id)
        .ok_or_else(|| format!("graft source `{source_id}` is not registered"))?;
    if configured.parent != original.parent {
        return Err("a graft copy must start beside the face it replaces".to_owned());
    }
    if configured.kind.trim() != original.kind {
        return Err(format!(
            "a source-preserving graft copy keeps kind `{}`; rename Rust types after creation",
            original.kind
        ));
    }
    let (_, original_path) = generated_paths(registry, source_id)?;

    let parent = if configured.parent == ROOT_NODE_ID {
        registry.id()
    } else {
        configured.parent
    };
    let parent_face = if parent == registry.id() {
        None
    } else {
        let face = registry
            .find(parent)
            .ok_or_else(|| format!("parent `{parent}` is not registered"))?;
        if !face.needs_registry {
            return Err(format!("parent `{parent}` does not own a Registry"));
        }
        Some(face)
    };
    let relative_dir = match parent_face {
        None => PathBuf::from(configured.module),
        Some(face) => {
            let source = Path::new(&face.source.file);
            if source.is_absolute() || source.components().any(is_parent_component) {
                return Err("external parent source cannot be authored by this package".to_owned());
            }
            source
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join("object")
                .join(configured.module)
        }
    };
    let source_relative = relative_dir.join(format!("{}.rs", configured.module));
    let draft_root = package_root()
        .join(".nichlink/grafts")
        .join(configured.module);
    let destination = draft_root.join("src").join(&source_relative);
    if draft_root.exists() {
        return Err(format!(
            "graft draft `{}` already exists",
            configured.module
        ));
    }

    let (parent_source, parent_kind) = parent_face
        .map(|face| (face.source.file.as_str(), face.kind.as_str()))
        .unwrap_or(("<root>", "root"));
    let mut face = FaceManifest::parse_source(&original_path)?;
    let source_relative_text = normalized_path(&source_relative);
    let parent_node = parent.to_string();
    for (key, value) in [
        ("module", configured.module),
        ("source", source_relative_text.as_str()),
        ("parent_node", parent_node.as_str()),
        ("parent_source", parent_source),
        ("parent_kind", parent_kind),
    ] {
        face.values.insert(key.to_owned(), value.to_owned());
    }
    face.values.insert(
        "registry_rule_path".to_owned(),
        rule_path_for_source(&source_relative_text),
    );
    if let Some(target_registry) = registry.registry(parent) {
        face.inherit_registry_contract(target_registry.registration_rule());
    }
    apply_new_face_values(&mut face, configured)?;
    let mut active_face = face.clone();
    normalize_for_target(&mut active_face, original, &original_path)?;
    let active_snapshot = original.clone().merge_authored(active_face.to_snapshot()?);
    registry
        .validate_snapshot_replacement(source_id, active_snapshot)
        .map_err(|error| format!("graft candidate rejected:\n{error}"))?;

    let original_source = fs::read_to_string(&original_path)
        .map_err(|error| format!("cannot read {}: {error}", original_path.display()))?;
    let rendered = face.render_source()?;
    let copied = replace_face_macro(&original_source, &rendered)
        .map_err(|error| format!("cannot copy registration declaration: {error}"))?;
    let rule_source = face.render_rule_source()?;
    let rule_path = face.rule_source_path_string()?;

    let staging = draft_root.with_extension(format!("nichlink-tmp-{}", std::process::id()));
    if staging.exists() {
        return Err(format!(
            "stale graft staging directory: {}",
            staging.display()
        ));
    }
    fs::create_dir_all(&staging)
        .map_err(|error| format!("cannot create graft staging directory: {error}"))?;
    let populate = (|| {
        let staged_module = staging.join("src").join(&relative_dir);
        fs::create_dir_all(&staged_module)
            .map_err(|error| format!("cannot create graft module directory: {error}"))?;
        if is_attached_module(&original_path) {
            copy_support_files(
                original_path
                    .parent()
                    .expect("original source has a parent"),
                &staged_module,
                &original_path,
            )?;
        }
        if let Some(rule_source) = rule_source {
            let rule_relative = Path::new(&rule_path)
                .strip_prefix("src")
                .unwrap_or_else(|_| Path::new(&rule_path));
            let staged_rule = staging.join("src").join(rule_relative);
            fs::create_dir_all(staged_rule.parent().expect("rule source has a parent"))
                .map_err(|error| format!("cannot create graft rule directory: {error}"))?;
            create_new(&staged_rule, &rule_source)?;
        }
        create_new(&staging.join("src").join(&source_relative), &copied)?;
        create_new(
            &staging.join("graft.plan"),
            &format!(
                "version=1\ntarget={}\ntarget_source={}\ndraft_source=src/{}\n",
                source_id,
                original.source.file,
                normalized_path(&source_relative)
            ),
        )
    })();
    if let Err(error) = populate {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    if let Err(error) = fs::rename(&staging, &draft_root) {
        let _ = fs::remove_dir_all(&staging);
        return Err(format!("cannot publish graft candidate directory: {error}"));
    }

    Ok((
        AuthoringChange {
            message: format!(
                "copied `{}` to graft candidate `{}`",
                original_path.display(),
                source_relative.display()
            ),
            source: destination.clone(),
        },
        GraftDraft {
            target: source_id,
            name: configured.module.to_owned(),
            root: draft_root,
            source: destination,
        },
    ))
}

fn copy_support_files(source: &Path, destination: &Path, face: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("cannot create {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("cannot scan graft source {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("cannot read graft source entry: {error}"))?;
        let path = entry.path();
        if path == face {
            continue;
        }
        let name = entry.file_name();
        let target = destination.join(&name);
        if path.is_dir() {
            if matches!(name.to_str(), Some("object" | "registry" | "registry_rule")) {
                continue;
            }
            fs::create_dir_all(&target)
                .map_err(|error| format!("cannot create {}: {error}", target.display()))?;
            copy_support_files(&path, &target, face)?;
        } else {
            fs::copy(&path, &target).map_err(|error| {
                format!(
                    "cannot copy graft support file {} to {}: {error}",
                    path.display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuthoringContext, FrameworkId, NewModuleFace, generated_snapshots_from};
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn graft_copy_keeps_the_original_implementation_body() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nichlink-graft-copy-{stamp}"));
        let source_dir = root.join("src/button");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(
            source_dir.join("button.rs"),
            r#"// generated-by=NichLink
mod paint_support;
pub struct Button;
impl Button { pub fn paint(&self) -> u32 { paint_support::COLOR } }

crate::root_object! {
    kind: Button,
    registry_name: button,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
    flow: crate::FlowContract::new(crate::ContractId::new("render.v1"), 1, "LocalCoordinates", "CanvasFrame"),
}
"#,
        )
        .unwrap();
        fs::write(
            source_dir.join("paint_support.rs"),
            "pub const COLOR: u32 = 42;\n",
        )
        .unwrap();
        let namespace = "graft-copy-test";
        let context = AuthoringContext::new(&root, namespace);
        let (draft, registry) = context.scope(|| {
            let mut snapshot = generated_snapshots_from(&root.join("src"))
                .unwrap()
                .pop()
                .unwrap();
            snapshot.contract.provided_parts.push("paint".to_owned());
            let source_id = snapshot.id;
            let parent = snapshot.parent;
            let mut registry = Registry::root_for_namespace(FrameworkId::new("test"), namespace);
            registry.register_snapshot_batch([snapshot]).unwrap();
            let configured = NewModuleFace {
                module: "button_graft",
                kind: "Button",
                preset: "NoPreset",
                parts: "NoParts",
                name_zh: "Button",
                name_en: "Button",
                summary_zh: "",
                summary_en: "",
                params: "Button",
                exports: "",
                handle: "Button",
                stable_name: "",
                parent,
                needs_registry: false,
                registry_name: "button_graft",
                getting_from_other_registry: "",
                registry_rule_path: "",
                registration_rule: "ANY",
                admission: "ANY",
                handle_traits: "",
                handle_contracts: "",
                part_traits: "",
                requires: "",
                provides: "",
                expected_output: "()",
                actual_output: "()",
                runtime_checks: "",
                flow: "render.v1|1|LocalCoordinates|CanvasFrame",
                flow_provider: "",
            };

            let draft = copy_module_for_graft(&registry, source_id, &configured)
                .unwrap()
                .1;
            (draft, registry)
        });

        let copied = fs::read_to_string(draft.source()).unwrap();
        assert!(copied.contains("pub fn paint(&self) -> u32 { paint_support::COLOR }"));
        assert!(!copied.contains("registry_name: button,"));
        assert_eq!(
            fs::read_to_string(draft.source().parent().unwrap().join("paint_support.rs")).unwrap(),
            "pub const COLOR: u32 = 42;\n"
        );
        assert_eq!(
            generated_snapshots_from(&root.join("src")).unwrap().len(),
            1
        );
        assert!(draft.root().join("graft.plan").is_file());
        let validated = context
            .scope(|| validate_graft_draft(&registry, &draft))
            .unwrap();
        assert_eq!(validated.contract.provided_parts, ["paint"]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn graft_draft_is_validated_again_and_applied_atomically() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("nichlink-graft-apply-{stamp}"));
        let source_dir = root.join("src/button");
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(
            source_dir.join("button.rs"),
            r#"// generated-by=NichLink
pub struct Button;
impl Button { pub fn paint(&self) -> u32 { 1 } }

crate::root_object! {
    kind: Button,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
    flow: crate::FlowContract::new(crate::ContractId::new("render.v1"), 1, "LocalCoordinates", "CanvasFrame"),
}
"#,
        )
        .unwrap();
        let namespace = "graft-apply-test";
        let context = AuthoringContext::new(&root, namespace);
        context.scope(|| {
            let snapshot = generated_snapshots_from(&root.join("src"))
                .unwrap()
                .pop()
                .unwrap();
            let source_id = snapshot.id;
            let mut registry = Registry::root_for_namespace(FrameworkId::new("test"), namespace);
            registry
                .register_snapshot_batch([snapshot.clone()])
                .unwrap();
            let configured = NewModuleFace {
                module: "button_graft",
                kind: "Button",
                preset: "NoPreset",
                parts: "NoParts",
                name_zh: "Button",
                name_en: "Button",
                summary_zh: "",
                summary_en: "",
                params: "Button",
                exports: "",
                handle: "Button",
                stable_name: "",
                parent: snapshot.parent,
                needs_registry: false,
                registry_name: "button_graft",
                getting_from_other_registry: "",
                registry_rule_path: "",
                registration_rule: "ANY",
                admission: "ANY",
                handle_traits: "",
                handle_contracts: "",
                part_traits: "",
                requires: "",
                provides: "",
                expected_output: "()",
                actual_output: "()",
                runtime_checks: "",
                flow: "render.v1|1|LocalCoordinates|CanvasFrame",
                flow_provider: "",
            };
            let (_, draft) = copy_module_for_graft(&registry, source_id, &configured).unwrap();
            let edited = fs::read_to_string(draft.source())
                .unwrap()
                .replace("{ 1 }", "{ 2 }");
            fs::write(draft.source(), edited).unwrap();
            apply_graft_draft(&mut registry, &draft).unwrap();

            let active = fs::read_to_string(source_dir.join("button.rs")).unwrap();
            assert!(active.contains("pub fn paint(&self) -> u32 { 2 }"));
            assert!(draft.root().join("original/button.rs").is_file());
            assert_eq!(registry.find(source_id).unwrap().registry_name, "button");
        });
        fs::remove_dir_all(root).unwrap();
    }
}
