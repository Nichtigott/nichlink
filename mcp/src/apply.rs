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

use nichlink::{NodeId, Registry};
use nichlink_build_method::{face_views, source_layout};
use nichlink_run_method::{AuthoringContext, NewModuleFace};
use serde_json::Value;

use crate::preview::{copy_package, diff_package, remove_copy};
use crate::registry::namespace;

/// One `nichlink.apply` request.
/// 一次 `nichlink.apply` 请求。
enum Action {
    /// Create a module registration face.
    /// 创建一个模块注册面。
    Add,
    /// Rewrite one existing face's fields.
    /// 重写一个已存在注册面的字段。
    Edit,
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
        Some(other) => {
            return Err(format!(
                "action `{other}` is not implemented; this tool supports `add` and `edit`"
            ));
        }
        None => return Err("nichlink.apply requires `action` (`add` or `edit`)".to_owned()),
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
        Action::Edit => run_edit(&work, &namespace, arguments),
    };
    let outcome = match outcome {
        // A preview ran in the copy, so the path the executor reported belongs to
        // the copy. Report it where it *would* be written: an agent reading
        // `would write /tmp/...` would be told about a directory that is deleted a
        // moment later.
        // 预览在副本里运行，因此执行器报告的路径属于副本。把它报告在**将要**写入的位置：读到
        // `would write /tmp/...` 的代理会被告知一个随后就被删掉的目录。
        Ok(mut outcome) => {
            if let Ok(relative) = outcome.source.strip_prefix(&work) {
                outcome.source = root.join(relative);
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
    /// The file the executor wrote, inside whichever tree it ran in.
    /// 执行器写入的文件，位于它运行的那棵树里。
    source: PathBuf,
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
    })
}

/// Rewrite one face's fields.
/// 重写一个注册面的字段。
fn run_edit(root: &Path, namespace: &str, arguments: &Value) -> Result<Outcome, String> {
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| "edit requires `node`: a logical path or a 32-digit identity".to_owned())?;
    let fields = arguments.get("fields").unwrap_or(&Value::Null);
    let registry = load_registry(root, namespace)?;
    let id = resolve_node(root, namespace, target)?;
    // An edit rewrites the whole face from the fields this surface models, so the
    // caller has to state what it wants the face to be, not only what differs —
    // that is the executor's contract, and half a patch would silently erase the
    // fields it did not mention.
    // 编辑会用本执行面建模的字段整体重写该面，因此调用方必须说出它要这个面**成为**什么，而不只是
    // 差异——这是执行器的契约，半个补丁会静默抹掉它没有提到的字段。
    let patch = nichlink_run_method::ModuleFacePatch {
        module: text(fields, "module"),
        kind: text(fields, "kind"),
        preset: text(fields, "preset"),
        parts: text(fields, "parts"),
        name_zh: text(fields, "name_zh"),
        name_en: text(fields, "name_en"),
        summary_zh: text(fields, "summary_zh"),
        summary_en: text(fields, "summary_en"),
        exports: text(fields, "exports"),
        stable_name: text(fields, "stable_name"),
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
    let change = AuthoringContext::new(root.to_path_buf(), namespace.to_owned())
        .scope(|| nichlink_run_method::edit_module_face(&registry, id, &patch))?;
    Ok(Outcome {
        message: change.message,
        source: change.source,
    })
}

/// The one string field of a request, or the empty string.
/// 请求里的一个字符串字段；没有时为空串。
fn text<'a>(fields: &'a Value, key: &str) -> &'a str {
    fields.get(key).and_then(Value::as_str).unwrap_or("")
}

/// The parent identity a request names, by identity or by logical path.
/// 请求命名的父身份，可按身份或按逻辑路径给出。
///
/// A logical path is the agent-friendly spelling, and it is resolved against the
/// same derivation the registry query reports — so an agent can take a path from
/// `nichlink.registry` and use it here without translating it.
/// 逻辑路径是对代理友好的写法，而且它按注册树查询所报告的那份推导解析——因此代理可以把
/// `nichlink.registry` 给出的路径直接用在这里，不必翻译。
fn parent_id(
    root: &Path,
    namespace: &str,
    arguments: &Value,
    fields: &Value,
) -> Result<NodeId, String> {
    // `parent` is a sibling of `fields` in the request, because it names a place in
    // the tree rather than a field of the new face; `fields.parent` is accepted as
    // an alias so a caller that puts every value in one object still works.
    // `parent` 在请求里与 `fields` 平级，因为它命名的是树里的位置而不是新面的一个字段；
    // `fields.parent` 作为别名接受，因此把所有取值放进一个对象的调用方也能用。
    let configured = arguments
        .get("parent")
        .or_else(|| fields.get("parent"))
        .and_then(Value::as_str);
    match configured {
        // The default is the *namespaced* root: the bare `ROOT_NODE_ID` is a
        // different identity, and a face hung under it would report a logical path
        // of `root/...` while living in no tree the host compiled.
        // 默认值是**带命名空间的**根：裸的 `ROOT_NODE_ID` 是另一个身份，挂在它下面的面会报告
        // `root/...` 的逻辑路径，却不住在宿主编译过的任何树里。
        None | Some("") => Ok(nichlink::root_node_id(namespace)),
        Some(value) => match value.parse::<NodeId>() {
            Ok(id) => Ok(id),
            Err(_) => resolve_node(root, namespace, value),
        },
    }
}

/// Resolve a logical path or an identity to a face identity.
/// 把逻辑路径或身份解析成一个面的身份。
fn resolve_node(root: &Path, namespace: &str, target: &str) -> Result<NodeId, String> {
    if let Ok(id) = target.parse::<NodeId>() {
        return Ok(id);
    }
    let wanted = target.trim_start_matches('/');
    let mut faces = face_views(root, namespace)?;
    // The registry root has no face row of its own unless something declares it,
    // so `root` is answered from the namespace directly.
    // 注册树根没有自己的面行（除非有东西声明了它），因此 `root` 直接由命名空间作答。
    if wanted == "root" {
        return Ok(nichlink::root_node_id(namespace));
    }
    faces.retain(|face| face.path == wanted);
    match faces.len() {
        0 => Err(format!("no registration face at `{target}`")),
        1 => Ok(faces[0].id),
        _ => Err(format!("`{target}` is ambiguous")),
    }
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
    let verb = if applied { "applied" } else { "would write" };
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
    Ok(format!(
        "action {}\nnamespace {namespace}\n{verb} {}\n{}\nfaces {}\n{list}\n",
        if applied { "apply" } else { "preview" },
        outcome.source.display(),
        outcome.message,
        faces.len()
    ))
}

#[cfg(test)]
#[path = "apply_tests.rs"]
mod apply_tests;
