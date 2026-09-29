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
            .find_registry(parent)
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
    let kind = nichlink_kernel::authoring::pascal_case(name);
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

    let module_directory = source.parent().expect("source has a parent");
    let mut partial = PartialWrite::new();
    // Recorded before the directory exists: on a rollback only what this call
    // created may be removed again.
    // 在目录存在之前记录：回滚时只能移除本次调用创建的东西。
    partial.track_directory(module_directory);
    fs::create_dir_all(module_directory)
        .map_err(|error| format!("cannot create module directory: {error}"))?;
    write_new_module_files(&face, &source, &mut partial).map_err(|error| partial.report(error))?;

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

/// Write the new face's file and, when it owns one, its generated registry rule.
/// 写入新注册面的文件，以及它拥有时生成的注册规则文件。
///
/// Every file and directory this creates is recorded in `partial` *before* it can
/// fail, so the caller can take the half-written module back. Its errors are the
/// bare messages the caller used to receive; the rollback report is added by
/// [`PartialWrite::report`].
/// 本函数创建的每个文件与目录都在可能失败之前记入 `partial`，因此调用方能把半成品模块收回。它返回
/// 的错误就是调用方过去收到的那条裸消息；回滚报告由 [`PartialWrite::report`] 追加。
fn write_new_module_files(
    face: &FaceManifest,
    source: &Path,
    partial: &mut PartialWrite,
) -> Result<(), String> {
    let rule_source = face.render_rule_source()?;
    let rule = face.rule_source_path()?;
    if let Some(rule_source) = rule_source {
        let rule_directory = rule.parent().expect("rule source has a parent");
        partial.track_directory(rule_directory);
        fs::create_dir_all(rule_directory)
            .map_err(|error| format!("cannot create registry rule directory: {error}"))?;
        create_new(&rule, &rule_source)?;
        partial.track_file(rule);
    }
    let source_text = face.render_source()?;
    create_new(source, &source_text)
}

/// The half-written state a failed creation has to take back.
/// 失败创建必须收回的半成品状态。
///
/// The three rollback points this replaced each discarded their own error, so the
/// caller saw only the first failure and believed the tree had been restored: a
/// rule file could stay behind, and `<module>/` could stay as an empty directory
/// (audit `LGC-LG-48`). A rollback failure is now part of the message, and only
/// directories this call created are removed — an existing directory with
/// somebody else's content in it is never touched.
/// 它取代的三个回滚点各自丢弃了自己的错误，因此调用方只看到最初那条错误，并相信树已被恢复：
/// 规则文件可能留在磁盘上，`<module>/` 可能留成空目录（审计 `LGC-LG-48`）。回滚失败现在写进
/// 消息，而且只移除本次调用创建的目录——已有目录里有别人的内容时绝不碰它。
struct PartialWrite {
    /// Paths this call created with `create_new`.
    /// 本次调用用 `create_new` 创建的文件路径。
    created_files: Vec<PathBuf>,
    /// Directories this call created, shallow first.
    /// 本次调用创建的目录，浅的在前。
    created_directories: Vec<PathBuf>,
}

impl PartialWrite {
    fn new() -> Self {
        Self {
            created_files: Vec::new(),
            created_directories: Vec::new(),
        }
    }

    /// Remember a directory, but only if it is not there yet.
    /// 记住一个目录，仅当它还不存在时。
    fn track_directory(&mut self, directory: &Path) {
        if !directory.exists() {
            self.created_directories.push(directory.to_path_buf());
        }
    }

    /// Remember a file this call just created.
    /// 记住本次调用刚创建的文件。
    fn track_file(&mut self, file: PathBuf) {
        self.created_files.push(file);
    }

    /// Compose what the caller sees: the original error, plus whatever the
    /// rollback could not undo.
    /// 组装调用方看到的内容：最初的错误，加上回滚没能撤销的部分。
    fn report(self, error: String) -> String {
        let mut failures = Vec::new();
        for file in &self.created_files {
            if let Err(rollback) = fs::remove_file(file) {
                failures.push(format!("could not remove {}: {rollback}", file.display()));
            }
        }
        for directory in self.created_directories.iter().rev() {
            // `DirectoryNotEmpty` is not a rollback failure: it means the
            // directory holds something of the caller's, which is not ours to
            // delete. `NotFound` means a shallower removal already took it.
            // `DirectoryNotEmpty` 不是回滚失败：它说明目录里有调用方的东西，不该由我们删。
            // `NotFound` 说明更浅的一次移除已经带走了它。
            if let Err(rollback) = fs::remove_dir(directory)
                && !matches!(
                    rollback.kind(),
                    std::io::ErrorKind::DirectoryNotEmpty | std::io::ErrorKind::NotFound
                )
            {
                failures.push(format!(
                    "could not remove {}: {rollback}",
                    directory.display()
                ));
            }
        }
        if failures.is_empty() {
            error
        } else {
            format!("{error}; rollback incomplete: {}", failures.join("; "))
        }
    }
}

#[cfg(test)]
#[path = "create_tests.rs"]
mod create_tests;
