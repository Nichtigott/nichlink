//! Module creation transaction: stage, validate, then write.
//! 模块创建事务：先暂存校验，再写入源码树。

use super::*;

pub(super) fn create_module(
    registry: &Registry,
    name: &str,
    parent: NodeId,
    configured: Option<&ModuleFaceValues<'_>>,
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

    // The kernel owns the one PascalCase derivation; this used to be a local
    // copy that split only on `_`, so a name with another separator produced a
    // kind that is not a valid Rust type name.
    // PascalCase 推导只有内核那一份；这里曾是一份只按 `_` 切分的本地副本，因此名字里
    // 出现别的分隔符时会产出并非合法 Rust 类型名的 kind。
    let kind = nichlink::authoring::pascal_case(name);
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
    if let Some(configured) = configured {
        apply_module_face_values(&mut face, configured, FaceWrite::Create)?;
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
