//! The bridge's write path: agent-initiated edits, previewed before they land.
//! 桥的写入路径：代理发起的编辑，落盘前先预览。
//!
//! The bridge does not re-implement editing. It builds the same `Registry` Studio
//! builds from the package's own source, installs the same `AuthoringContext`, and
//! calls the same executor, so an agent's edit passes the kernel's admission,
//! parent-rule, and topology checks — and is refused for the same reasons a
//! human's edit in Studio is.
//! 桥不重新实现编辑。它从包自己的源码构建出与 Studio 相同的 `Registry`，装上相同的
//! `AuthoringContext`，调用同一个执行器，因此代理的编辑会经过内核的准入、父规则与拓扑校验——
//! 也会因为与人在 Studio 里编辑相同的原因被拒绝。
//!
//! **Preview is the default, and it is the real operation.** A preview runs the
//! executor against a throwaway copy of the package (`crate::preview`), then reports
//! two things: the diff between the copy and the project (which files would change,
//! and how), and the registration faces the copy derives afterwards. That is why the
//! preview cannot drift from the apply: both call `run`, and only the directory
//! differs.
//! **预览是默认，而且它就是真实操作。** 预览在一份一次性的包副本上运行执行器
//! （`crate::preview`），然后报告两件事：副本与项目之间的 diff（哪些文件会变、怎么变），以及
//! 副本随后推导出的注册面。这正是预览不可能与落盘漂移的原因：两者都调用 `run`，差的只是目录。

use std::path::{Path, PathBuf};

use nichlink::Registry;
use nichlink_build_method::{face_views, source_layout};
use nichlink_run_method::{AuthoringContext, NewModuleFace};
use serde_json::Value;

use crate::nodes::{parent_id, resolve_node};
use crate::preview::{copy_package, declaration_line, diff_package, remove_copy};
use crate::registry::namespace;

/// One `nichlink.apply` request.
/// 一次 `nichlink.apply` 请求。
enum Action {
    /// Create a module registration face.
    /// 创建一个模块注册面。
    Add,
    /// Rewrite the fields a request names, keeping the rest of the face.
    /// 重写请求点名的字段，面的其余部分保持不动。
    Edit,
    /// Change a face's module name, keeping every other field.
    /// 改一个面的模块名，其余字段全部保留。
    Rename,
    /// Move one face's module subtree into the recoverable trash.
    /// 把一个面的模块子树移入可恢复的回收目录。
    Delete,
}

/// Run one edit request, previewing unless `apply` is true.
/// 执行一次编辑请求；除非 `apply` 为真，否则只预览。
///
/// `root` is the resolved package root. A root Cargo cannot name is refused here
/// before anything is copied or written — the same refusal the registry query
/// makes, for the same reason: a namespace that is guessed authors faces no host
/// compiles.
/// `root` 是已解析的包根。Cargo 说不出名字的包根在这里、在复制或写入任何东西之前就被拒绝——
/// 与注册树查询相同的拒绝，理由也相同：猜出来的命名空间会创作出没有宿主编译的注册面。
pub(crate) fn apply(root: &Path, arguments: &Value) -> Result<String, String> {
    let action = match arguments.get("action").and_then(Value::as_str) {
        Some("add") => Action::Add,
        Some("edit") => Action::Edit,
        Some("rename") => Action::Rename,
        Some("delete") => Action::Delete,
        Some(other) => {
            return Err(format!(
                "action `{other}` is not implemented; this tool supports `add`, `edit`, \
                 `rename`, and `delete`"
            ));
        }
        None => {
            return Err(
                "nichlink.apply requires `action` (`add`, `edit`, `rename`, or `delete`)"
                    .to_owned(),
            );
        }
    };
    let namespace = namespace(root)?;
    let apply = arguments
        .get("apply")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let work = if apply {
        root.to_path_buf()
    } else {
        copy_package(root)?
    };
    let outcome = match action {
        Action::Add => run_add(&work, &namespace, arguments),
        Action::Edit | Action::Rename => run_edit(&work, &namespace, arguments, action),
        Action::Delete => run_delete(&work, &namespace, arguments),
    };
    let outcome = match outcome {
        // A preview ran in the copy, so the path the executor reported belongs to
        // the copy. Report it where it *would* be written: an agent reading
        // `would write /tmp/...` would be told about a directory that is deleted a
        // moment later.
        // 预览在副本里运行，因此执行器报告的路径属于副本。把它报告在**将要**写入的位置：读到
        // `would write /tmp/...` 的代理会被告知一个随后就被删掉的目录。
        Ok(mut outcome) => {
            // The declaration anchor is read while the tree it happened in still
            // exists: for a preview that is the copy, for an apply it is the project.
            // 声明锚点是在改动发生的那棵树仍然存在时读取的：预览是副本，落盘是项目。
            let original = outcome.source.clone();
            let relative = original
                .strip_prefix(&work)
                .map(Path::to_path_buf)
                .unwrap_or_else(|_| {
                    original
                        .strip_prefix(root)
                        .map(Path::to_path_buf)
                        .unwrap_or_else(|_| original.clone())
                });
            outcome.declaration = declaration_line(&original, &relative);
            if let Ok(relative) = outcome.source.strip_prefix(&work) {
                outcome.source = root.join(relative);
            }
            // The executor's own message names paths too, so a preview would
            // otherwise print a directory that is deleted a moment later.
            // 执行器自己的消息也会点名路径，否则预览会打印出一个随后就被删掉的目录。
            let from = work.display().to_string();
            if from != root.display().to_string() {
                outcome.message = outcome.message.replace(&from, &root.display().to_string());
            }
            outcome
        }
        Err(error) => {
            // A failed preview leaves the copy behind for nothing; a failed apply
            // keeps the project, and the executor has already refused before
            // touching it.
            // 失败的预览不必留下副本；失败的落盘保留项目，而执行器在碰它之前就已经拒绝。
            remove_copy(root, &work);
            return Err(error);
        }
    };
    let report = report(&work, &namespace, &outcome, apply)?;
    // The copy's diff is computed before it goes away.
    let diff = if apply {
        String::new()
    } else {
        diff_package(root, &work)?
    };
    remove_copy(root, &work);
    let mut text = report;
    if !diff.is_empty() {
        text.push_str("\ndiff:\n");
        text.push_str(&diff);
    }
    Ok(text)
}

/// What one successful executor call changed.
/// 一次成功的执行器调用改了什么。
struct Outcome {
    message: String,
    /// `<path>:<line>` of the face declaration this change produced, when there is
    /// one (a delete moves the file away and has none).
    /// 本次改动产生的面声明所在的 `<path>:<line>`；删除把文件搬走，因此没有。
    declaration: Option<String>,
    /// The file the executor wrote — or, for a delete, the trash path it moved the
    /// module to.
    /// 执行器写入的文件——对删除而言，则是它把模块搬到的回收路径。
    source: PathBuf,
    /// Whether the path is a destination rather than a rewritten source.
    /// 该路径是目的地，而不是被重写的源文件。
    moved: bool,
}

/// Create a face.
/// 创建一个注册面。
fn run_add(root: &Path, namespace: &str, arguments: &Value) -> Result<Outcome, String> {
    let fields = arguments.get("fields").unwrap_or(&Value::Null);
    let module = text(fields, "module");
    if module.is_empty() {
        return Err("add requires `fields.module`, the new module's name".to_owned());
    }
    let registry = load_registry(root, namespace)?;
    let parent = parent_id(root, namespace, arguments, fields)?;
    let face = NewModuleFace {
        module,
        kind: text(fields, "kind"),
        preset: text(fields, "preset"),
        parts: text(fields, "parts"),
        name_zh: text(fields, "name_zh"),
        name_en: text(fields, "name_en"),
        summary_zh: text(fields, "summary_zh"),
        summary_en: text(fields, "summary_en"),
        exports: text(fields, "exports"),
        stable_name: text(fields, "stable_name"),
        parent,
        needs_registry: fields
            .get("needs_registry")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        getting_from_other_registry: text(fields, "getting_from_other_registry"),
        registration_rule: text(fields, "registration_rule"),
        admission: text(fields, "admission"),
        handle_traits: text(fields, "handle_traits"),
        handle_contracts: text(fields, "handle_contracts"),
        part_traits: text(fields, "part_traits"),
        part_contracts: text(fields, "part_contracts"),
        requires: text(fields, "requires"),
        provides: text(fields, "provides"),
        runtime_checks: text(fields, "runtime_checks"),
        flow: text(fields, "flow"),
        flow_provider: text(fields, "flow_provider"),
    };
    // `registration_rule` and `admission` are spelled `admission` in the manifest
    // and `registration_rule` in the struct; both are taken from the same fields.
    // `registration_rule` 与 `admission` 在清单里写作 `admission`、在结构体里写作
    // `registration_rule`；两者都取自同一组字段。
    // `add_module_from_face` returns the same face it registered, so the caller can
    // commit it; this tool's feedback is the re-derived tree, so the snapshot half is
    // dropped rather than registered into a throwaway registry.
    // `add_module_from_face` 会返回它注册的那个面，供调用方提交；本工具的反馈是重新推导出的树，
    // 因此那一半被丢掉，而不是提交进一次性的注册树。
    let (change, _) = AuthoringContext::new(root.to_path_buf(), namespace.to_owned())
        .scope(|| nichlink_run_method::add_module_from_face(&registry, &face))?;
    Ok(Outcome {
        message: change.message,
        source: change.source,
        declaration: None,
        moved: false,
    })
}

/// Rewrite the fields a request names, keeping everything it does not.
/// 重写请求点名的字段，其余全部保留。
///
/// The executor's own contract is the opposite — it rebuilds a face from the values
/// it is given, blank included — and that contract is right for an editor showing
/// every field. An agent asking for two changes would have had to restate
/// twenty-three fields and would silently blank any it forgot, so this path reads
/// the face back through `authored_face`, overlays the request, and hands the
/// complete set to the executor. The executor still decides everything that
/// matters; only the *input* is completed for the caller.
/// 执行器自己的契约相反——它用拿到的取值整体重建一个面，包括空值——而那对展示所有字段的编辑器是
/// 正确的。一个只要求两处改动的代理否则必须复述二十三个字段，且会静默抹掉它忘掉的任何一个，因此
/// 这条路径经 `authored_face` 读回该面、覆盖请求、再把完整的一组交给执行器。仍然由执行器决定所有
/// 要紧的事；只是**输入**替调用方补全了。
fn run_edit(
    root: &Path,
    namespace: &str,
    arguments: &Value,
    action: Action,
) -> Result<Outcome, String> {
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| "edit requires `node`: a logical path or a 32-digit identity".to_owned())?;
    let fields = arguments.get("fields").unwrap_or(&Value::Null);
    let registry = load_registry(root, namespace)?;
    let id = resolve_node(root, namespace, target)?;
    if matches!(action, Action::Rename) {
        let module = fields.get("module").and_then(Value::as_str).unwrap_or("");
        if module.is_empty() {
            return Err("rename requires `fields.module`, the new module name".to_owned());
        }
    }
    // Both halves run inside one context, and not only the write: reading the face
    // back resolves its path from the context's package root, so a read outside it
    // would look for the face under whatever directory the process happens to be in
    // — a bug this pin caught, because the CLI-shaped manual run had
    // `NICH_LINK_PACKAGE_ROOT` set and the unit test did not.
    // 两半都在同一个上下文里运行，而且不只是写入那一半：读回该面时它的路径由上下文的包根解析，因此
    // 在上下文之外读会去进程碰巧所在的目录里找那个面——正是这个缺陷被钉子抓住：手工的 CLI 式运行设了
    // `NICH_LINK_PACKAGE_ROOT`，而单元测试没设。
    let context = AuthoringContext::new(root.to_path_buf(), namespace.to_owned());
    let change = context.scope(|| -> Result<_, String> {
        let mut authored = nichlink_run_method::authored_face(&registry, id)?;
        overlay(&mut authored, fields)?;
        nichlink_run_method::edit_module_face(&registry, id, &authored.as_patch())
    })?;
    Ok(Outcome {
        message: change.message,
        source: change.source,
        declaration: None,
        moved: false,
    })
}

/// Move a face's module subtree into the trash.
/// 把一个面的模块子树移入回收目录。
///
/// Deletion is recoverable by design — the executor moves the directory rather than
/// unlinking it — and the request has to say `confirm` for the same reason the CLI
/// does: a delete is the one operation whose preview a caller can skip past by
/// accident.
/// 删除按设计是可恢复的——执行器搬走目录而不是删掉它——而请求必须像 CLI 一样说出 `confirm`：
/// 删除是唯一一种调用方可能不小心跳过其预览的操作。
fn run_delete(root: &Path, namespace: &str, arguments: &Value) -> Result<Outcome, String> {
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "delete requires `node`: a logical path or a 32-digit identity".to_owned()
        })?;
    let registry = load_registry(root, namespace)?;
    let id = resolve_node(root, namespace, target)?;
    let change = AuthoringContext::new(root.to_path_buf(), namespace.to_owned())
        .scope(|| nichlink_run_method::delete_module(&registry, &format!("{id} confirm")))?;
    Ok(Outcome {
        message: change.message,
        source: change.source,
        declaration: None,
        moved: true,
    })
}

/// Overlay a request's fields onto a face read back from the tree.
/// 把请求的字段覆盖到从树上读回的那个面上。
///
/// Every key is matched by name: a misspelled field is refused instead of being
/// dropped, which is the direction that keeps an agent from believing it changed
/// something it did not.
/// 每个键按名字匹配：拼错的字段被拒绝而不是被丢弃——正是这个方向让代理不会以为自己改了什么而
/// 其实没改。
fn overlay(authored: &mut nichlink_run_method::AuthoredFace, fields: &Value) -> Result<(), String> {
    let Some(object) = fields.as_object() else {
        return Ok(());
    };
    for (key, value) in object {
        let text = || {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("`{key}` must be a string"))
        };
        match key.as_str() {
            "module" => authored.module = text()?,
            "kind" => authored.kind = text()?,
            "preset" => authored.preset = text()?,
            "parts" => authored.parts = text()?,
            "name_zh" => authored.name_zh = text()?,
            "name_en" => authored.name_en = text()?,
            "summary_zh" => authored.summary_zh = text()?,
            "summary_en" => authored.summary_en = text()?,
            "exports" => authored.exports = text()?,
            "stable_name" => authored.stable_name = text()?,
            "needs_registry" => {
                authored.needs_registry = value
                    .as_bool()
                    .ok_or_else(|| "`needs_registry` must be true or false".to_owned())?;
            }
            "getting_from_other_registry" => authored.getting_from_other_registry = text()?,
            "registration_rule" => authored.registration_rule = text()?,
            "admission" => authored.admission = text()?,
            "handle_traits" => authored.handle_traits = text()?,
            "handle_contracts" => authored.handle_contracts = text()?,
            "part_traits" => authored.part_traits = text()?,
            "part_contracts" => authored.part_contracts = text()?,
            "requires" => authored.requires = text()?,
            "provides" => authored.provides = text()?,
            "runtime_checks" => authored.runtime_checks = text()?,
            "flow" => authored.flow = text()?,
            "flow_provider" => authored.flow_provider = text()?,
            other => {
                return Err(format!(
                    "`{other}` is not an editable registration-face field"
                ));
            }
        }
    }
    Ok(())
}

/// The one string field of a request, or the empty string.
/// 请求里的一个字符串字段；没有时为空串。
fn text<'a>(fields: &'a Value, key: &str) -> &'a str {
    fields.get(key).and_then(Value::as_str).unwrap_or("")
}

/// Build the registry the executor validates against: the package's own faces,
/// under the namespace Cargo reports.
/// 构建执行器据以校验的注册树：该包自己的注册面，位于 Cargo 报告的命名空间之下。
fn load_registry(root: &Path, namespace: &str) -> Result<Registry, String> {
    let mut registry =
        Registry::root_for_namespace(nichlink::FrameworkId::new("nichlink.mcp"), namespace);
    let source_root = source_layout(root)?.scan_root;
    let snapshots = AuthoringContext::new(root.to_path_buf(), namespace.to_owned())
        .scope(|| nichlink_run_method::generated_snapshots_from(&source_root))?;
    registry
        .register_snapshot_batch(snapshots)
        .map_err(|error| format!("the package's own faces were rejected: {error}"))?;
    Ok(registry)
}

/// What happened, in the shape the agent reads next.
/// 发生了什么，以代理接下来要读的形状给出。
fn report(
    root: &Path,
    namespace: &str,
    outcome: &Outcome,
    applied: bool,
) -> Result<String, String> {
    let verb = match (outcome.moved, applied) {
        (false, true) => "applied",
        (false, false) => "would write",
        (true, true) => "moved",
        (true, false) => "would move",
    };
    // The tree the change produces is the strongest part of the preview: it is the
    // same derivation the registry query reports, run on the tree the executor just
    // changed, so a validation failure shows up here rather than after a write.
    // 变更产生的树是预览最强的部分：它与注册树查询报告的是同一份推导，只是跑在执行器刚改过的
    // 那棵树上，因此校验失败会在这里出现，而不是在写入之后。
    let faces = face_views(root, namespace)?;
    let list = faces
        .iter()
        .map(|face| format!("  {}  {}  {}", face.path, face.kind, face.source))
        .collect::<Vec<_>>()
        .join("\n");
    let declaration = outcome
        .declaration
        .as_ref()
        .map(|anchor| format!("declaration {anchor}\n"))
        .unwrap_or_default();
    Ok(format!(
        "action {}\nnamespace {namespace}\n{verb} {}\n{declaration}{}\nfaces {}\n{list}\n",
        if applied { "apply" } else { "preview" },
        outcome.source.display(),
        outcome.message,
        faces.len()
    ))
}

#[cfg(test)]
#[path = "apply_tests.rs"]
mod apply_tests;
