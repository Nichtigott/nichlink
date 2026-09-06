//! File-backed add, edit, and delete operations.
//! 文件化的新增、编辑和删除操作。

use super::*;

pub struct AuthoringChange {
    pub message: String,
    pub source: PathBuf,
}

/// Complete registration face accepted by the atomic add operation.
/// 原子 add 操作接收的完整注册面。
#[derive(Clone, Copy, Debug)]
pub struct NewModuleFace<'a> {
    pub module: &'a str,
    pub kind: &'a str,
    pub preset: &'a str,
    pub parts: &'a str,
    pub name_zh: &'a str,
    pub name_en: &'a str,
    pub summary_zh: &'a str,
    pub summary_en: &'a str,
    pub params: &'a str,
    pub exports: &'a str,
    pub handle: &'a str,
    pub stable_name: &'a str,
    pub parent: NodeId,
    pub needs_registry: bool,
    pub registry_name: &'a str,
    pub getting_from_other_registry: &'a str,
    pub registry_rule_path: &'a str,
    pub registration_rule: &'a str,
    pub admission: &'a str,
    pub handle_traits: &'a str,
    pub handle_contracts: &'a str,
    pub part_traits: &'a str,
    pub requires: &'a str,
    pub provides: &'a str,
    pub expected_output: &'a str,
    pub actual_output: &'a str,
    pub runtime_checks: &'a str,
    pub flow: &'a str,
    pub flow_provider: &'a str,
}

/// Editable subset used by the Studio's multi-field edit form.
/// Studio 多字段编辑表单使用的可编辑字段集合。
#[derive(Clone, Copy, Debug)]
pub struct ModuleFacePatch<'a> {
    /// New module directory/file name. Changing it migrates the whole subtree.
    /// 新模块目录和文件名；修改它会迁移整棵子树。
    pub module: &'a str,
    pub kind: &'a str,
    pub preset: &'a str,
    pub parts: &'a str,
    pub name_zh: &'a str,
    pub name_en: &'a str,
    pub summary_zh: &'a str,
    pub summary_en: &'a str,
    pub params: &'a str,
    pub exports: &'a str,
    pub handle: &'a str,
    pub stable_name: &'a str,
    pub needs_registry: bool,
    pub registry_name: &'a str,
    pub getting_from_other_registry: &'a str,
    pub registry_rule_path: &'a str,
    pub registration_rule: &'a str,
    pub admission: &'a str,
    pub handle_traits: &'a str,
    pub handle_contracts: &'a str,
    pub part_traits: &'a str,
    pub requires: &'a str,
    pub provides: &'a str,
    pub expected_output: &'a str,
    pub actual_output: &'a str,
    pub runtime_checks: &'a str,
    pub flow: &'a str,
    pub flow_provider: &'a str,
}

fn apply_new_face_values(
    face: &mut FaceManifest,
    configured: &NewModuleFace<'_>,
) -> Result<(), String> {
    for (field, value) in [
        ("preset", configured.preset),
        ("parts", configured.parts),
        ("name_zh", configured.name_zh),
        ("name_en", configured.name_en),
        ("summary_zh", configured.summary_zh),
        ("summary_en", configured.summary_en),
        ("params", configured.params),
        ("exports", configured.exports),
        ("handle", configured.handle),
        ("stable_name", configured.stable_name),
        (
            "getting_from_other_registry",
            configured.getting_from_other_registry,
        ),
        ("handle_traits", configured.handle_traits),
        ("handle_contracts", configured.handle_contracts),
        ("part_traits", configured.part_traits),
        ("requires", configured.requires),
        ("provides", configured.provides),
        ("expected_output", configured.expected_output),
        ("actual_output", configured.actual_output),
        ("runtime_checks", configured.runtime_checks),
        ("flow", configured.flow),
        ("flow_provider", configured.flow_provider),
    ] {
        if !value.trim().is_empty() {
            face.edit(field, value.trim())?;
        }
    }
    face.edit("needs_registry", &configured.needs_registry.to_string())?;
    if !configured.registry_name.trim().is_empty() {
        face.edit("registry_name", configured.registry_name.trim())?;
    }
    face.edit("registration_rule", configured.registration_rule.trim())?;
    face.edit("admission", configured.admission.trim())?;
    // The rule file lives beside the face. Keep this path derived from the
    // source tree; accepting an arbitrary path would split one face across two
    // unrelated directories.
    // 规则文件固定与注册面同目录派生；不接受任意路径，避免一个注册面被拆到
    // 两个无关目录。
    let _ = configured.registry_rule_path;
    Ok(())
}

/// Create the standard `<parent>/object/<name>/<name>.rs` registration face.
/// 创建标准的 `<parent>/object/<name>/<name>.rs` 注册面。
pub fn add_module(registry: &Registry, spec: &str) -> Result<AuthoringChange, String> {
    add_module_with_registration(registry, spec).map(|(change, _)| change)
}

/// Create a module and return the same registration face for immediate commit.
/// 创建模块并返回同一注册面，以便立即提交到当前 Registry。
pub fn add_module_with_registration(
    registry: &Registry,
    spec: &str,
) -> Result<(AuthoringChange, RegistrationSnapshot), String> {
    let mut fields = spec.split_whitespace();
    let name = fields
        .next()
        .ok_or_else(|| "usage: add <module-name> [parent-node]".to_owned())?;
    validate_name(name)?;
    let parent = fields
        .next()
        .map(str::parse::<NodeId>)
        .transpose()
        .map_err(|_| "parent must be a 32-digit node identity".to_owned())?
        .unwrap_or(ROOT_NODE_ID);
    if fields.next().is_some() {
        return Err("usage: add <module-name> [parent-node]".to_owned());
    }

    create_module(registry, name, parent, None)
}

/// Validate and create a fully configured face in one filesystem transaction.
/// 一次校验并创建完整注册面，避免逐字段编辑留下半成品。
pub fn add_module_from_face(
    registry: &Registry,
    spec: &NewModuleFace<'_>,
) -> Result<(AuthoringChange, RegistrationSnapshot), String> {
    validate_name(spec.module)?;
    create_module(registry, spec.module, spec.parent, Some(spec))
}

/// Load authored faces into an owned snapshot that can be dropped after a reload.
/// 将创作注册面加载为可随热刷新释放的拥有所有权快照。
pub fn generated_snapshots() -> Result<Vec<RegistrationSnapshot>, String> {
    generated_snapshots_from(&source_root())
}

/// Load generated registration faces from an explicit project root.
pub fn generated_snapshots_from(root: &Path) -> Result<Vec<RegistrationSnapshot>, String> {
    let mut sources = Vec::new();
    collect_face_sources(root, &mut sources)?;
    sources.sort();
    sources
        .into_iter()
        .map(|source| {
            FaceManifest::parse_source(&source)
                .and_then(|face| face.to_snapshot())
                .map_err(|error| format!("{}: {error}", source.display()))
        })
        .collect()
}

fn collect_face_sources(directory: &Path, sources: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?;
    for entry in entries {
        let path = entry
            .map_err(|error| format!("cannot scan {}: {error}", directory.display()))?
            .path();
        if path.is_dir() {
            collect_face_sources(&path, sources)?;
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            && !path
                .components()
                .any(|component| component.as_os_str() == "registry_core")
            && !path
                .components()
                .any(|component| component.as_os_str() == "compile_error_demo")
            && fs::read_to_string(&path)
                .map(|source| {
                    source
                        .lines()
                        .any(|line| line.trim_start().starts_with("crate::control_object!"))
                        || source.lines().any(|line| line == GENERATED_MARKER)
                })
                .unwrap_or(false)
        {
            sources.push(path);
        }
    }
    Ok(())
}

fn create_module(
    registry: &Registry,
    name: &str,
    parent: NodeId,
    configured: Option<&NewModuleFace<'_>>,
) -> Result<(AuthoringChange, RegistrationSnapshot), String> {
    // The legacy root token means "this Registry's root". Studio uses a
    // package namespace, so its concrete root identity is intentionally not
    // the process-wide legacy constant.
    // 旧根标识表示“当前 Registry 的根”。Studio 使用包命名空间，因此具体根
    // 标识不会等于进程级旧常量。
    let parent = if parent == ROOT_NODE_ID {
        registry.id()
    } else {
        parent
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
        None => PathBuf::from(name),
        Some(face) => {
            let source = Path::new(&face.source.file);
            if source.is_absolute() || source.components().any(is_parent_component) {
                return Err("external parent source cannot be authored by this package".to_owned());
            }
            source
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join("object")
                .join(name)
        }
    };
    let source_relative = relative_dir.join(format!("{name}.rs"));
    let source = source_root().join(&source_relative);
    if source.exists() {
        return Err(format!(
            "module `{}` already exists",
            source_relative.display()
        ));
    }

    let kind = rust_type_name(name);
    let (parent_source, parent_kind) = parent_face
        .map(|face| (face.source.file.as_str(), face.kind.as_str()))
        .unwrap_or(("<root>", "root"));
    let mut face = FaceManifest::new(
        name,
        &kind,
        parent,
        parent_source,
        parent_kind,
        &normalized_path(&source_relative),
    );
    // The target registry owns the mounting contract. New faces inherit its
    // interface requirements so Add never asks users to copy parent metadata.
    // 目标注册机拥有挂载合同。新注册面自动继承接口要求，Add 不要求用户重复填写父级元数据。
    if let Some(target_registry) = registry.registry(parent) {
        face.inherit_registry_contract(target_registry.registration_rule());
    }
    if let Some(configured) = configured {
        if !configured.kind.trim().is_empty() {
            let normalized_kind = normalize_kind_name(configured.kind.trim());
            face.edit("kind", &normalized_kind)?;
        }
        apply_new_face_values(&mut face, configured)?;
    }
    let info = face.to_snapshot()?;

    // Validate against a staged registry before touching the source tree. This
    // keeps an invalid add from leaving half-written files behind.
    // 写入源码树前先在暂存注册机中校验，避免失败的 add 留下半成品文件。
    registry
        .clone()
        .register_snapshot_batch([info.clone()])
        .map_err(|error| format!("registration rejected:\n{error}"))?;

    fs::create_dir_all(source.parent().expect("source has a parent"))
        .map_err(|error| format!("cannot create module directory: {error}"))?;
    let rule_source = face.render_rule_source()?;
    let rule = face.rule_source_path()?;
    if rule_source.is_some() {
        fs::create_dir_all(rule.parent().expect("rule source has a parent"))
            .map_err(|error| format!("cannot create registry rule directory: {error}"))?;
    }
    let source_text = face.render_source()?;
    let mut rule_created = false;
    if let Some(rule_source) = rule_source {
        create_new(&rule, &rule_source)?;
        rule_created = true;
    }
    if let Err(error) = create_new(&source, &source_text) {
        if rule_created {
            let _ = fs::remove_file(&rule);
        }
        return Err(error);
    }

    Ok((
        AuthoringChange {
            message: format!(
                "created `{}` under parent {}",
                source_relative.display(),
                parent
            ),
            source,
        },
        info,
    ))
}

/// Edit one field in a NichLink-generated registration face.
/// 编辑由 NichLink 生成的注册面中的一个字段。
pub fn edit_module(registry: &Registry, spec: &str) -> Result<AuthoringChange, String> {
    let mut fields = spec.splitn(3, char::is_whitespace);
    let id = fields
        .next()
        .ok_or_else(|| "usage: edit <node> <field> <value>".to_owned())?
        .parse::<NodeId>()
        .map_err(|_| "edit requires a 32-digit node identity".to_owned())?;
    let field = fields
        .next()
        .ok_or_else(|| "usage: edit <node> <field> <value>".to_owned())?;
    let value = fields
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "usage: edit <node> <field> <value>".to_owned())?;

    let (_, source) = generated_paths(registry, id)?;
    let mut face = FaceManifest::parse_source(&source)?;
    let normalized_kind = (field == "kind").then(|| normalize_kind_name(value));
    let kind_changed = if let Some(next_kind) = normalized_kind.as_ref() {
        let current_kind = face.values.get("kind").cloned().unwrap_or_default();
        next_kind != &current_kind
    } else {
        false
    };
    face.edit(field, normalized_kind.as_deref().unwrap_or(value))?;
    if kind_changed {
        return migrate_kind_subtree(registry, id, source, face);
    }
    let authored = face.to_snapshot()?;
    let existing = registry
        .find(id)
        .ok_or_else(|| format!("node `{id}` is not registered"))?;
    registry
        .validate_snapshot_replacement(id, existing.clone().merge_authored(authored))
        .map_err(|error| format!("registration rejected:\n{error}"))?;
    let old_source = fs::read_to_string(&source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
    if let Err(error) = face
        .render_source()
        .and_then(|source_text| atomic_write(&source, &source_text))
    {
        // A face spans two files. Restore the manifest if the source write
        // fails, so a failed edit never leaves the pair out of sync.
        // 一个注册面跨越两个文件；源文件写入失败时恢复 manifest，避免
        // 编辑失败后两份文件内容不一致。
        let _ = atomic_write(&source, &old_source);
        return Err(error);
    }

    Ok(AuthoringChange {
        message: format!("updated `{field}` in {}", source.display()),
        source,
    })
}

/// Edit all Studio-owned face fields in one filesystem transaction.
/// 在一次文件事务中编辑 Studio 管理的全部注册面字段。
pub fn edit_module_face(
    registry: &Registry,
    id: NodeId,
    patch: &ModuleFacePatch<'_>,
) -> Result<AuthoringChange, String> {
    let (_, source) = generated_paths(registry, id)?;
    let mut face = FaceManifest::parse_source(&source)?;
    let old_module = face.values.get("module").cloned().unwrap_or_default();
    let requested_module = patch.module.trim();
    validate_name(requested_module)?;
    let module_changed = requested_module != old_module;
    let original_kind = face.values.get("kind").cloned().unwrap_or_default();
    let normalized_kind = normalize_kind_name(patch.kind.trim());
    let kind_changed = normalized_kind != original_kind;
    face.edit("kind", &normalized_kind)?;
    face.edit("preset", patch.preset)?;
    face.edit("parts", patch.parts)?;
    face.edit("name_zh", patch.name_zh)?;
    face.edit("name_en", patch.name_en)?;
    face.edit("summary_zh", patch.summary_zh)?;
    face.edit("summary_en", patch.summary_en)?;
    face.edit("params", patch.params)?;
    face.edit("exports", patch.exports)?;
    face.edit("handle", patch.handle)?;
    face.edit("stable_name", patch.stable_name)?;
    face.edit("needs_registry", &patch.needs_registry.to_string())?;
    face.edit("registry_name", patch.registry_name)?;
    face.edit(
        "getting_from_other_registry",
        patch.getting_from_other_registry,
    )?;
    face.edit("registration_rule", patch.registration_rule)?;
    face.edit("admission", patch.admission)?;
    face.edit("handle_traits", patch.handle_traits)?;
    face.edit("handle_contracts", patch.handle_contracts)?;
    face.edit("part_traits", patch.part_traits)?;
    face.edit("requires", patch.requires)?;
    face.edit("provides", patch.provides)?;
    face.edit("expected_output", patch.expected_output)?;
    face.edit("actual_output", patch.actual_output)?;
    face.edit("runtime_checks", patch.runtime_checks)?;
    face.edit("flow", patch.flow)?;
    face.edit("flow_provider", patch.flow_provider)?;
    if !patch.needs_registry
        && registry
            .registry(id)
            .is_some_and(|owned_registry| !owned_registry.is_empty())
    {
        return Err("cannot disable a Registry while it still owns child entries".to_owned());
    }
    if module_changed {
        return migrate_module_subtree(registry, id, source, face, requested_module);
    }
    if kind_changed {
        return migrate_kind_subtree(registry, id, source, face);
    }
    // Parse the complete edited face before touching either file.
    // 在修改任一文件前先解析完整注册面，保证错误编辑不会落盘。
    let authored = face.to_snapshot()?;
    let existing = registry
        .find(id)
        .ok_or_else(|| format!("node `{id}` is not registered"))?;
    let merged = existing.clone().merge_authored(authored);
    registry
        .validate_snapshot_replacement(id, merged)
        .map_err(|error| format!("registration rejected:\n{error}"))?;
    let old_source = fs::read_to_string(&source)
        .map_err(|error| format!("cannot read {}: {error}", source.display()))?;
    let rule = face.rule_source_path()?;
    let old_rule = fs::read_to_string(&rule).ok();
    let rendered = face.render_source()?;
    if let Err(error) = atomic_write(&source, &rendered) {
        let _ = atomic_write(&source, &old_source);
        return Err(error);
    }
    if face.values.get("needs_registry").map(String::as_str) == Some("true") {
        let rule_source = face
            .render_rule_source()?
            .ok_or_else(|| "registry rule source was unexpectedly omitted".to_owned())?;
        if let Err(error) = (|| {
            fs::create_dir_all(rule.parent().expect("rule source has a parent"))
                .map_err(|error| format!("cannot create registry rule directory: {error}"))?;
            atomic_write(&rule, &rule_source)
        })() {
            let _ = atomic_write(&source, &old_source);
            if let Some(old_rule) = old_rule {
                let _ = atomic_write(&rule, &old_rule);
            } else {
                let _ = fs::remove_file(&rule);
            }
            return Err(error);
        }
    } else if old_rule.is_some() {
        // Disabling a Registry also removes its generated rule source. Leaving
        // that file behind would make the next folder scan rediscover a stale
        // support subtree that the face no longer owns.
        // 关闭 Registry 时同步删除生成的规则源文件。否则下一次扫描会重新发现
        // 一个注册面已经不再拥有的陈旧 support 子树。
        if let Err(error) = fs::remove_file(&rule) {
            let _ = atomic_write(&source, &old_source);
            if let Some(old_rule) = old_rule {
                let _ = atomic_write(&rule, &old_rule);
            }
            return Err(format!(
                "cannot remove registry rule {}: {error}",
                rule.display()
            ));
        }
    }
    Ok(AuthoringChange {
        message: format!("updated registration face {}", source.display()),
        source,
    })
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
fn migrate_kind_subtree(
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

/// Rename a standard generated module and every authored child below it.
/// 重命名标准生成模块，并同步迁移其下的全部注册子对象。
///
/// A module rename changes every descendant's source-derived identity. The
/// filesystem move and the registry validation therefore happen as one
/// recoverable operation instead of pretending that the old NodeId survived.
/// 模块重命名会改变所有后代由源码路径派生的身份，因此文件迁移和注册校验必须
/// 作为一个可恢复事务执行，不能假装旧 NodeId 仍然有效。
fn migrate_module_subtree(
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

/// Move one generated module subtree into the recoverable NichLink trash.
/// 将一个生成模块子树移动到可恢复的 NichLink 回收目录。
pub fn delete_module(registry: &Registry, spec: &str) -> Result<AuthoringChange, String> {
    let mut fields = spec.split_whitespace();
    let id = fields
        .next()
        .ok_or_else(|| "usage: delete <node> confirm".to_owned())?
        .parse::<NodeId>()
        .map_err(|_| "delete requires a 32-digit node identity".to_owned())?;
    if fields.next() != Some("confirm") || fields.next().is_some() {
        return Err("usage: delete <node> confirm".to_owned());
    }

    let (name, source) = generated_paths(registry, id)?;
    let module_dir = source
        .parent()
        .ok_or_else(|| "generated source has no module directory".to_owned())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("clock error: {error}"))?
        .as_secs();
    let trash = package_root()
        .join(".nichlink")
        .join("trash")
        .join(format!("{name}-{id}-{stamp}"));
    fs::create_dir_all(trash.parent().expect("trash has a parent"))
        .map_err(|error| format!("cannot create NichLink trash: {error}"))?;
    fs::rename(module_dir, &trash)
        .map_err(|error| format!("cannot move module to trash: {error}"))?;

    Ok(AuthoringChange {
        message: format!("moved `{name}` to {}", trash.display()),
        source: trash,
    })
}

fn generated_paths(registry: &Registry, id: NodeId) -> Result<(String, PathBuf), String> {
    let face = registry
        .find(id)
        .ok_or_else(|| format!("node `{id}` is not registered"))?;
    let declared = Path::new(&face.source.file);
    let root = source_root();
    let source = if declared.is_absolute() {
        // Some collectors preserve an absolute declaration path. It is safe to
        // edit only when that path resolves below the active package's `src/`.
        // 某些收集器会保留绝对声明路径；只有确认它位于当前包 `src/`
        // 目录下时才允许编辑。
        let canonical_root = fs::canonicalize(&root)
            .map_err(|error| format!("cannot resolve package source root: {error}"))?;
        let canonical_source = fs::canonicalize(declared)
            .map_err(|error| format!("cannot resolve registration source: {error}"))?;
        if !canonical_source.starts_with(&canonical_root) {
            return Err("only package-local generated modules can be edited".to_owned());
        }
        canonical_source
    } else {
        if declared.components().any(is_parent_component) {
            return Err("only package-local generated modules can be edited".to_owned());
        }
        root.join(declared)
    };
    let relative = source.strip_prefix(&root).unwrap_or(declared);
    let text = fs::read_to_string(&source)
        .map_err(|_| "this module was not generated by NichLink".to_owned())?;
    if !is_nichlink_owned_source(relative, &text) {
        return Err("this module was not generated by NichLink".to_owned());
    }
    Ok((face.registry_name.to_owned(), source))
}

/// Recognize current generated files and the short-lived New Project scaffold.
/// 识别当前生成文件，以及曾经由 New Project 生成的旧脚手架。
fn is_nichlink_owned_source(relative: &Path, text: &str) -> bool {
    text.lines().any(|line| line == GENERATED_MARKER)
        || (relative == Path::new("control/control.rs")
            && text.contains("pub struct ControlRegistry;")
            && text.contains("registry_rule_path:")
            && text.contains("registry_rule:"))
}
