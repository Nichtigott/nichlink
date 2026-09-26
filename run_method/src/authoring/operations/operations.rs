//! File-backed add, edit, and delete operations.
//! 文件化的新增、编辑和删除操作。

use super::*;
#[path = "authored.rs"]
mod authored;
pub use authored::{AuthoredFace, authored_face};
#[path = "create.rs"]
mod create;
use create::create_module;
#[path = "delete.rs"]
mod delete;
pub use delete::delete_module;
#[path = "trash.rs"]
mod trash;
use trash::stash_face_source;
#[path = "face_values.rs"]
mod face_values;
use face_values::ModuleFaceValues;
#[path = "face_write.rs"]
mod face_write;
use face_write::{FaceWrite, apply_module_face_values};
#[path = "migration.rs"]
mod migration;
use migration::{migrate_kind_subtree, migrate_module_subtree};
#[path = "paths.rs"]
mod paths;
use paths::generated_paths;

/// The outcome of one file-backed authoring operation.
/// 一次文件化创作操作的结果。
pub struct AuthoringChange {
    /// A human-readable summary of what changed.
    /// 描述改动内容的人类可读摘要。
    pub message: String,
    /// The source file the change was written to.
    /// 改动写入的源码文件。
    pub source: PathBuf,
}

/// The complete field set for creating one module registration face.
/// 创建一个模块注册面所需的完整字段集合。
#[derive(Clone, Copy, Debug)]
pub struct NewModuleFace<'a> {
    /// New module directory and file name, and the face's identity path segment.
    /// 新模块的目录与文件名，同时是该注册面身份路径的一段。
    pub module: &'a str,
    /// Face kind name, for example `Button`.
    /// 注册面种类名，例如 `Button`。
    pub kind: &'a str,
    /// Preset type path that constructs this face.
    /// 构造本注册面的 preset 类型路径。
    pub preset: &'a str,
    /// Parts type path that supplies this face's construction parts.
    /// 提供本注册面构造 parts 的 parts 类型路径。
    pub parts: &'a str,
    /// Localized display name, Chinese half.
    /// 本地化显示名称的中文部分。
    pub name_zh: &'a str,
    /// Localized display name, English half.
    /// 本地化显示名称的英文部分。
    pub name_en: &'a str,
    /// Localized one-line description, Chinese half.
    /// 本地化单行描述的中文部分。
    pub summary_zh: &'a str,
    /// Localized one-line description, English half.
    /// 本地化单行描述的英文部分。
    pub summary_en: &'a str,
    /// Export names this face declares, as manifest text.
    /// 本注册面声明的导出名称，以清单文本给出。
    pub exports: &'a str,
    /// Optional author-owned identity that survives source moves; empty means none.
    /// 可选的作者逻辑身份，可跨源码移动保持不变；为空表示没有。
    pub stable_name: &'a str,
    /// Node identity of the face this one registers under.
    /// 本注册面所挂载到的父节点身份。
    pub parent: NodeId,
    /// Whether this face owns a child Registry.
    /// 本注册面是否拥有一个子注册机。
    pub needs_registry: bool,
    /// External registry this face is provisioned from; empty means none.
    /// 本注册面从其获取内容的外部注册机；为空表示没有。
    pub getting_from_other_registry: &'a str,
    /// Rule for faces entering the Registry this face owns.
    /// 进入本注册面所拥有 Registry 的注册规范。
    pub registration_rule: &'a str,
    /// External dependency gate for this face's registry.
    /// 本注册面所属注册机对外部依赖的门禁。
    pub admission: &'a str,
    /// Interface names declared by the handle type.
    /// handle 类型声明实现的接口名称。
    pub handle_traits: &'a str,
    /// Contract types the handle type must implement.
    /// handle 类型必须实现的合同类型。
    pub handle_contracts: &'a str,
    /// Interface names declared by the parts type.
    /// parts 类型声明实现的接口名称。
    pub part_traits: &'a str,
    /// Contract types the parts type must implement.
    /// parts 类型必须实现的合同类型。
    pub part_contracts: &'a str,
    /// Capability requirements this face declares.
    /// 本注册面声明的能力需求。
    pub requires: &'a str,
    /// Capability names this face makes available to other faces.
    /// 本注册面向其他注册面提供的能力名称。
    pub provides: &'a str,
    /// Runtime value checks the host applies; empty accepts any value.
    /// 宿主执行的运行期取值校验；为空时接受任何取值。
    pub runtime_checks: &'a str,
    /// Explicit flow contract for grafting; empty selects the default.
    /// 供嫁接使用的显式数据流合同；为空时选用默认值。
    pub flow: &'a str,
    /// Type that supplies the compile-time flow contract, when explicit.
    /// 显式提供编译期数据流合同的类型路径（如果显式给出）。
    pub flow_provider: &'a str,
}

/// The complete field set for editing one module registration face in place.
/// 原地编辑一个模块注册面所需的完整字段集合。
#[derive(Clone, Copy, Debug)]
pub struct ModuleFacePatch<'a> {
    /// New module directory/file name. Changing it migrates the whole subtree.
    /// 新模块目录和文件名；修改它会迁移整棵子树。
    pub module: &'a str,
    /// Face kind name, for example `Button`.
    /// 注册面种类名，例如 `Button`。
    pub kind: &'a str,
    /// Preset type path that constructs this face.
    /// 构造本注册面的 preset 类型路径。
    pub preset: &'a str,
    /// Parts type path that supplies this face's construction parts.
    /// 提供本注册面构造 parts 的 parts 类型路径。
    pub parts: &'a str,
    /// Localized display name, Chinese half.
    /// 本地化显示名称的中文部分。
    pub name_zh: &'a str,
    /// Localized display name, English half.
    /// 本地化显示名称的英文部分。
    pub name_en: &'a str,
    /// Localized one-line description, Chinese half.
    /// 本地化单行描述的中文部分。
    pub summary_zh: &'a str,
    /// Localized one-line description, English half.
    /// 本地化单行描述的英文部分。
    pub summary_en: &'a str,
    /// Export names this face declares, as manifest text.
    /// 本注册面声明的导出名称，以清单文本给出。
    pub exports: &'a str,
    /// Optional author-owned identity that survives source moves; empty means none.
    /// 可选的作者逻辑身份，可跨源码移动保持不变；为空表示没有。
    pub stable_name: &'a str,
    /// Whether this face owns a child Registry.
    /// 本注册面是否拥有一个子注册机。
    pub needs_registry: bool,
    /// External registry this face is provisioned from; empty means none.
    /// 本注册面从其获取内容的外部注册机；为空表示没有。
    pub getting_from_other_registry: &'a str,
    /// Rule for faces entering the Registry this face owns.
    /// 进入本注册面所拥有 Registry 的注册规范。
    pub registration_rule: &'a str,
    /// External dependency gate for this face's registry.
    /// 本注册面所属注册机对外部依赖的门禁。
    pub admission: &'a str,
    /// Interface names declared by the handle type.
    /// handle 类型声明实现的接口名称。
    pub handle_traits: &'a str,
    /// Contract types the handle type must implement.
    /// handle 类型必须实现的合同类型。
    pub handle_contracts: &'a str,
    /// Interface names declared by the parts type.
    /// parts 类型声明实现的接口名称。
    pub part_traits: &'a str,
    /// Contract types the parts type must implement.
    /// parts 类型必须实现的合同类型。
    pub part_contracts: &'a str,
    /// Capability requirements this face declares.
    /// 本注册面声明的能力需求。
    pub requires: &'a str,
    /// Capability names this face makes available to other faces.
    /// 本注册面向其他注册面提供的能力名称。
    pub provides: &'a str,
    /// Runtime value checks the host applies; empty accepts any value.
    /// 宿主执行的运行期取值校验；为空时接受任何取值。
    pub runtime_checks: &'a str,
    /// Explicit flow contract for grafting; empty selects the default.
    /// 供嫁接使用的显式数据流合同；为空时选用默认值。
    pub flow: &'a str,
    /// Type that supplies the compile-time flow contract, when explicit.
    /// 显式提供编译期数据流合同的类型路径（如果显式给出）。
    pub flow_provider: &'a str,
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
    let values = ModuleFaceValues::from_new(spec);
    validate_name(values.module)?;
    create_module(registry, values.module, spec.parent, Some(&values))
}

/// Load authored faces into an owned snapshot that can be dropped after a reload.
/// 将创作注册面加载为可随热刷新释放的拥有所有权快照。
pub fn generated_snapshots() -> Result<Vec<RegistrationSnapshot>, String> {
    generated_snapshots_from(&source_root())
}

/// Load generated registration faces from an explicit project root.
/// 从显式的项目根加载生成的注册面。
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

/// The filesystem facts the kernel's source walk asks this surface for.
/// 内核源码遍历向本执行面索取的文件系统事实。
struct StdSourceTree;

impl nichlink::source::SourceTree for StdSourceTree {
    fn is_directory(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn entries(&self, path: &Path) -> Result<Vec<PathBuf>, String> {
        fs::read_dir(path)
            .map_err(|error| format!("cannot scan {}: {error}", path.display()))?
            .map(|entry| {
                entry
                    .map(|entry| entry.path())
                    .map_err(|error| format!("cannot scan {}: {error}", path.display()))
            })
            .collect()
    }

    fn read_text(&self, path: &Path) -> Result<String, String> {
        fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
    }
}

fn collect_face_sources(directory: &Path, sources: &mut Vec<PathBuf>) -> Result<(), String> {
    nichlink::source::collect_rust_sources(
        &StdSourceTree,
        directory,
        nichlink::source::SourceWalk {
            skip_target: false,
            skip_registry_core: true,
            skip_compile_error_demo: true,
        },
        |_, source| match source {
            None => nichlink::source::Keep::NeedSource,
            Some(text) if crate::syntax::is_face_source(text, GENERATED_MARKER) => {
                nichlink::source::Keep::Yes
            }
            Some(_) => nichlink::source::Keep::No,
        },
        sources,
    )
}

/// Edit all Studio-owned face fields in one filesystem transaction.
/// 在一次文件事务中编辑 Studio 管理的全部注册面字段。
///
/// This is the only authored-face edit entry point. The string-parsing
/// `edit_module` was deleted in B3a: it edited the manifest but never wrote or
/// removed the registry-rule source, so `edit <id> needs_registry true` left a
/// face referencing a `super::registry_rule` module it did not own. The doctest
/// below pins the deletion at compile time.
/// 这是唯一的注册面编辑入口。字符串解析式的 `edit_module` 已在 B3a 删除：它只改清单，
/// 从不写入或删除注册规则源，于是 `edit <id> needs_registry true` 会留下一个引用着
/// 自己并不拥有的 `super::registry_rule` 模块的注册面。下面的 doctest 在编译期钉住
/// 这次删除。
///
/// ```compile_fail,E0433
/// let _ = nichlink_run_method::edit_module;
/// ```
pub fn edit_module_face(
    registry: &Registry,
    id: NodeId,
    patch: &ModuleFacePatch<'_>,
) -> Result<AuthoringChange, String> {
    let (_, source) = generated_paths(registry, id)?;
    let mut face = FaceManifest::parse_source(&source)?;
    let values = ModuleFaceValues::from_patch(patch);
    let old_module = face.values.get("module").cloned().unwrap_or_default();
    let requested_module = values.module.trim();
    validate_name(requested_module)?;
    let module_changed = requested_module != old_module;
    let original_kind = face.values.get("kind").cloned().unwrap_or_default();
    let kind_changed = normalize_kind_name(values.kind.trim()) != original_kind;
    apply_module_face_values(&mut face, &values, FaceWrite::Edit)?;
    if !values.needs_registry
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
    // The rewrite rebuilds the file from the fields this surface models, so
    // anything hand-added to it is not in `rendered`. Keep the previous text
    // recoverable before the first byte changes: deleting a face already goes
    // through the trash, and a rewrite deserves the same way back. Nothing is
    // stashed when the rewrite is a no-op, so an unedited save leaves no litter.
    // 重写会用本执行面建模的字段重建文件，因此手工加进文件的内容不在 `rendered` 里。在
    // 第一个字节改变之前把先前的文本留成可恢复的：删除注册面本就经 trash 走，重写也应当
    // 有同样的回退方式。重写没有实质变化时不留备份，因此未改动的保存不会留下垃圾。
    let backup = if rendered == old_source {
        None
    } else {
        Some(stash_face_source(&source, id, &old_source)?)
    };
    if let Err(error) = atomic_write(&source, &rendered) {
        let _ = atomic_write(&source, &old_source);
        return Err(error);
    }
    if face.owns_rule_source() {
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
        message: match &backup {
            Some(backup) => format!(
                "updated registration face {} (previous text kept at {})",
                source.display(),
                backup.display()
            ),
            None => format!("updated registration face {}", source.display()),
        },
        source,
    })
}
