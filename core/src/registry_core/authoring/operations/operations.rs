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
    if field == "kind" {
        let next_kind = normalize_kind_name(value);
        let current_kind = face.values.get("kind").cloned().unwrap_or_default();
        if next_kind != current_kind {
            return Err(
                "kind is part of NodeId and cannot be edited; use an explicit subtree migration"
                    .to_owned(),
            );
        }
    }
    face.edit(field, value)?;
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
    let original_kind = face.values.get("kind").cloned().unwrap_or_default();
    let normalized_kind = normalize_kind_name(patch.kind.trim());
    if normalized_kind != original_kind {
        return Err(
            "kind is part of NodeId and cannot be edited; use an explicit subtree migration"
                .to_owned(),
        );
    }
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
    let relative = Path::new(&face.source.file);
    if relative.is_absolute() || relative.components().any(is_parent_component) {
        return Err("only package-local generated modules can be edited".to_owned());
    }
    let source = source_root().join(relative);
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
