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
//! executor against a throwaway copy of the package (`crate::mcp::preview`), then reports
//! two things: the diff between the copy and the project (which files would change,
//! and how), and the registration faces the copy derives afterwards. That is why the
//! preview cannot drift from the apply: both call `run`, and only the directory
//! differs.
//! **预览是默认，而且它就是真实操作。** 预览在一份一次性的包副本上运行执行器
//! （`crate::mcp::preview`），然后报告两件事：副本与项目之间的 diff（哪些文件会变、怎么变），以及
//! 副本随后推导出的注册面。这正是预览不可能与落盘漂移的原因：两者都调用 `run`，差的只是目录。

use std::path::{Path, PathBuf};

use crate::build_time::{face_views, source_layout};
use crate::runtime::{AuthoringContext, NewModuleFace};
use nichlink_kernel::Registry;
use serde_json::Value;

use crate::mcp::apply_target::Target;
use crate::mcp::preview::{copy_package, declaration_line, diff_package};
use crate::mcp::registry::namespace;
use crate::mcp::resolve::{parent_id, resolve_node};

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
    /// Add a layer **inside** one face, leaving its declaration, path and tree row alone.
    /// 在一个面**内部**加一层，不动它的声明、路径与树行。
    Deepen,
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
        Some("deepen") => Action::Deepen,
        Some(other) => {
            return Err(format!(
                "action `{other}` is not implemented; this tool supports `add`, `edit`, \
                 `rename`, `delete`, and `deepen`"
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
    let target = if apply {
        Target::Project
    } else {
        Target::Copy(copy_package(root)?)
    };
    let outcome = match action {
        Action::Add => run_add(target.work_dir(root), &namespace, arguments),
        Action::Edit | Action::Rename => {
            run_edit(target.work_dir(root), &namespace, arguments, action)
        }
        Action::Delete => run_delete(target.work_dir(root), &namespace, arguments),
        Action::Deepen => run_deepen(target.work_dir(root), &namespace, arguments),
    };
    let outcome = match outcome {
        // A preview ran in the copy, so the paths the executor reported belong to
        // the copy. They are rewritten here, before anything reads them, rather
        // than left to a reader to reinterpret: an agent reading `would write
        // /tmp/...` would be told about a directory that is deleted a moment later.
        // 预览在副本里运行，因此执行器报告的路径属于副本。它们在这里、在任何读取方之前被改写，
        // 而不是留给读者自行解释：读到 `would write /tmp/...` 的代理会被告知一个随后就被删掉的目录。
        Ok(mut outcome) => {
            // The declaration anchor is read while the tree it happened in still
            // exists: for a preview that is the copy, for an apply it is the project.
            // 声明锚点是在改动发生的那棵树仍然存在时读取的：预览是副本，落盘是项目。
            let original = outcome.source.clone();
            outcome.declaration =
                declaration_line(&original, &target.report_relative(&original, root));
            outcome.source = target.report_path(&outcome.source, root);
            outcome.message = target.report_message(outcome.message, root);
            outcome
        }
        Err(error) => {
            // A failed preview leaves the copy behind for nothing; a failed apply
            // keeps the project, and the executor has already refused before
            // touching it.
            // 失败的预览不必留下副本；失败的落盘保留项目，而执行器在碰它之前就已经拒绝。
            target.discard(root);
            return Err(refused_with_a_way_forward(error));
        }
    };
    let report = report(root, &target, &namespace, &outcome)?;
    // The copy's diff is computed before it goes away.
    let diff = match &target {
        Target::Project => String::new(),
        Target::Copy(work) => diff_package(root, work)?,
    };
    target.discard(root);
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
    /// Whether this change came from `deepen`, whose reply also states the *other* reading of
    /// "make it deeper" and what that one costs.
    /// 这次改动是否来自 `deepen`——它的回复还会说出"把它做深"的**另一种读法**及那种读法的代价。
    alternative: bool,
}

/// The face fields a request may carry, which is the set `overlay` matches by name.
/// 请求可以携带的注册面字段，也就是 `overlay` 按名字匹配的那一组。
const EDITABLE_FIELDS: &[&str] = &[
    "module",
    "kind",
    "preset",
    "parts",
    "name_zh",
    "name_en",
    "summary_zh",
    "summary_en",
    "exports",
    "stable_name",
    "needs_registry",
    "getting_from_other_registry",
    "registration_rule",
    "admission",
    "handle_traits",
    "handle_contracts",
    "part_traits",
    "part_contracts",
    "requires",
    "provides",
    "runtime_checks",
    "flow",
    "flow_provider",
];

/// The first thing wrong with a request's `fields`, if anything is.
/// 请求的 `fields` 里第一处不对的地方（若有）。
///
/// Both write actions share this so they answer alike: a misspelled key is refused by
/// name rather than dropped, and a key whose value cannot be a field is refused instead of
/// being coerced to the empty string or to `false`. `add` used to take only the keys it
/// knew and silently ignore the rest, so an agent that wrote `knd` believed it had set the
/// kind — the exact direction this check exists to close.
/// 两个写入动作共用它，因此它们答得一样：拼错的键被点名拒绝而不是被丢弃；值不可能成为字段的键
/// 也被拒绝，而不是被强转成空串或 `false`。`add` 过去只取它认识的键、静默忽略其余，于是写下
/// `knd` 的代理会以为自己设了 kind——这道检查正是为了关掉这个方向。
fn invalid_field(fields: &Value, action: Action) -> Option<String> {
    let object = fields.as_object()?;
    for (key, value) in object {
        // `fields.parent` is the documented alias of the request's sibling `parent`, and
        // it names a place in the tree rather than a field of the face.
        // `fields.parent` 是与请求平级的 `parent` 的文档化别名，它命名的是树里的位置而不是面的字段。
        if key == "parent" {
            if !value.is_string() {
                return Some(
                    "`parent` must be a string (a logical path or an identity)".to_owned(),
                );
            }
            continue;
        }
        if !EDITABLE_FIELDS.contains(&key.as_str()) {
            // The refusal names the shape as well as the mistake: the round measured five refusals
            // in one family, all of them field spelling, and a refusal that only says "no" costs the
            // next call as well.
            // 拒绝同时说出"形状"和"错在哪"：那轮量到一个族里五次被拒、全是字段拼法，而只会说"不行"的
            // 拒绝还要再花掉下一次调用。
            return Some(format!(
                "`{key}` is not an editable registration-face field\n{}",
                editable_fields_line()
            ));
        }
        // The executor's edit field order does not carry the two contract keys: they
        // are rewritten on `add` and *kept* on `edit`, so accepting them here returned
        // success while the value never reached the file — an agent that asked for a
        // checked contract got an unchecked label instead, silently (audit
        // `LGC-LG-21`). Refusing by name is the loud half of the same choice.
        // 执行器的 edit 字段顺序不携带这两个 contract 键：它们在 `add` 时被写、在 `edit` 时被**保留**，
        // 因此在这里接受它们会返回成功而取值从未抵达文件——一个要求"参与编译检查的契约"的代理拿到的
        // 是未经检查的标签，而且无声（审计 `LGC-LG-21`）。按名拒绝是同一个选择的响亮那一半。
        if matches!(action, Action::Edit | Action::Rename)
            && matches!(key.as_str(), "handle_contracts" | "part_contracts")
        {
            return Some(format!(
                "`{key}` cannot be changed by `{}`: the executor's edit field order does not \
                 carry it, so the write would be dropped. Set it in `add`, or leave it as the \
                 file has it",
                if matches!(action, Action::Rename) {
                    "rename"
                } else {
                    "edit"
                }
            ));
        }
        if key == "needs_registry" {
            if !value.is_boolean() {
                return Some(format!(
                    "`needs_registry` must be true or false\n{}",
                    editable_fields_line()
                ));
            }
        } else if !value.is_string() {
            return Some(format!(
                "`{key}` must be a string (this shape takes strings, not arrays or objects)\n{}",
                editable_fields_line()
            ));
        }
    }
    None
}

/// Create a face.
/// 创建一个注册面。
fn run_add(root: &Path, namespace: &str, arguments: &Value) -> Result<Outcome, String> {
    let fields = arguments.get("fields").unwrap_or(&Value::Null);
    if let Some(problem) = invalid_field(fields, Action::Add) {
        return Err(problem);
    }
    let module = text(fields, "module");
    if module.is_empty() {
        return Err(
            "add requires `fields.module`, the new module's name — accepted shape: \
             {\"action\":\"add\",\"parent\":\"<node>\",\"fields\":{\"module\":\"<snake_case>\",\
             \"kind\":\"<Kind>\",\"exports\":\"<export>\"},\"apply\":true}"
                .to_owned(),
        );
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
        .scope(|| crate::runtime::add_module_from_face(&registry, &face))?;
    Ok(Outcome {
        message: change.message,
        source: change.source,
        declaration: None,
        moved: false,
        alternative: false,
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
            return Err(
                "rename requires `fields.module`, the new module name — accepted shape: \
                 {\"action\":\"rename\",\"node\":\"<node>\",\"fields\":{\"module\":\"<snake_case>\"},\
                 \"apply\":true}"
                    .to_owned(),
            );
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
    let (change, previous_kind, new_kind) = context.scope(|| -> Result<_, String> {
        let mut authored = crate::runtime::authored_face(&registry, id)?;
        let previous = authored.kind.clone();
        overlay(&mut authored, fields, action)?;
        let kind = authored.kind.clone();
        let change = crate::runtime::edit_module_face(&registry, id, &authored.as_patch())?;
        Ok((change, previous, kind))
    })?;
    // `kind` is an identity input — `NodeId = hash(namespace, source, name)`, and a face's name is its
    // kind — so an edit that changes it rewrites the marker type and the `kind:` field, and the face
    // comes back under a *new* identity while every graft record keyed by the old one stops
    // resolving. The reply used to report the resulting tree without ever saying so (audit `L6`);
    // one sentence closes it, and it points at the tool that lists the fallout.
    // `kind` 是身份输入——`NodeId = hash(namespace, source, name)`，而面的名字就是它的 kind——因此改它的
    // 编辑会重写标记类型与 `kind:` 字段，这个面以**新身份**回来，而以旧身份为键的每条 graft 记录都不再
    // 解析。回复过去只报告结果树、从不说明（审计 `L6`）；一句话即可闭合，并指向列出后果的工具。
    let mut message = change.message;
    if previous_kind != new_kind {
        // The *new* identity is deliberately not printed: computing it here would mean reproducing
        // the kernel's `(namespace, relative path, declared name)` triple from a `PathBuf` that is
        // not spelled the way `face_views` spells it, and the pin below caught exactly that (the
        // number it printed was not the one the tree reported). A wrong number in a diagnostic is
        // worse than no number, so the reply names the old identity — the one graft records are
        // keyed by — and points at the tool that lists the fallout.
        // **新**身份有意不打印：在这里算它意味着用 `face_views` 不采用的那种拼法，从一个 `PathBuf`
        // 复现内核的 `(命名空间, 相对路径, 声明名)` 三元组——而下面的钉子恰好抓到了这一点（它打印的数字
        // 并不是树报告的那个）。诊断里的错数字比没有数字更糟，因此回复只点名**旧**身份——graft 记录正是
        // 以它为键——并指向列出后果的工具。
        message.push_str(&format!(
            "\nidentity changed: `kind` `{previous_kind}` → `{new_kind}` is an identity input \
             (`NodeId = hash(namespace, source, name)`, and a face's name is its kind), so this face \
             is no longer `{id}`; graft records keyed by the old identity no longer resolve — \
             `nichlink.diff {{\"records\": true}}` lists them\n"
        ));
    }
    Ok(Outcome {
        message,
        source: change.source,
        declaration: None,
        moved: false,
        alternative: false,
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
    // The request says `confirm` itself. This used to be appended right here, which made the
    // sentence above — and the declared schema, which had no such key at all — describe something
    // the bridge did not do: a caller could leave it out and the delete still went through
    // (audit `m5`).
    // 请求自己说出 `confirm`。过去是在这里就地拼上去的，于是上面那句话——以及根本没有这个键的声明
    // schema——描述的是桥并不做的事：调用方可以省掉它，删除照样执行（审计 `m5`）。
    if arguments.get("confirm").and_then(Value::as_bool) != Some(true) {
        return Err(
            "delete requires `confirm: true`: a delete is the one operation whose preview a caller \
             can step past by accident"
                .to_owned(),
        );
    }
    let registry = load_registry(root, namespace)?;
    let id = resolve_node(root, namespace, target)?;
    let change = AuthoringContext::new(root.to_path_buf(), namespace.to_owned())
        .scope(|| crate::runtime::delete_module(&registry, &format!("{id} confirm")))?;
    Ok(Outcome {
        message: change.message,
        source: change.source,
        declaration: None,
        moved: true,
        alternative: false,
    })
}

/// Add a layer **inside** one face: a parts type the face owns, plus an accessor.
/// 在一个面**内部**加一层：它自己拥有的零件类型，以及一个访问器。
///
/// The measured reason this exists: family b of the round-5 evaluation asked for "this object is
/// not deep enough inside", and the only write the bridge had (`add`) works on the registration
/// tree — so "deeper" could only be read as "another face under it". That reading moves four
/// families of factory-shape assertions and adds a segment to the public module path, so the arm
/// that kept the gate green delivered nothing and wrote an essay about why. This action adds the
/// layer the object's own contract already talks about, touches exactly one file, and leaves the
/// tree, the public paths and the pins alone.
/// 这个动作的存在有实测原因：第五轮五族 b 的题面是"这个对象内部还不够"，而桥当时唯一的写
/// （`add`）作用在注册树上——于是"更深"只能被读成"它下面再挂一个面"。那条路会移动四类按出厂形状
/// 钉死的断言、并让公开模块路径多一段，于是守住门的那一臂交不出东西、只写了一篇"为什么不可能"。
/// 本动作加的是对象自己契约已经说到的那一层，只碰一个文件，树、公开路径与钉子都不动。
fn run_deepen(root: &Path, namespace: &str, arguments: &Value) -> Result<Outcome, String> {
    let target = arguments
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "deepen requires `node`: the face, by logical path or identity".to_owned()
        })?;
    let parts = arguments
        .get("inside")
        .and_then(|inside| inside.get("parts"))
        .and_then(Value::as_object)
        .filter(|parts| !parts.is_empty())
        .ok_or_else(|| {
            "deepen requires `inside.parts`: an object of `field: Type` pairs, at least one — \
             accepted shape: {\"action\":\"deepen\",\"node\":\"<node>\",\
             \"inside\":{\"parts\":{\"<field>\":\"<Type>\"}},\"apply\":true}"
                .to_owned()
        })?;
    let mut fields: Vec<(String, String)> = Vec::new();
    for (name, value) in parts {
        let named_well = !name.is_empty()
            && !name.starts_with(|character: char| character.is_ascii_digit())
            && name.chars().all(|character| {
                character == '_' || character.is_ascii_lowercase() || character.is_ascii_digit()
            });
        if !named_well {
            return Err(format!(
                "`{name}` is not a snake_case field name; this action writes Rust, so the names it \
                 writes have to be Rust names"
            ));
        }
        let kind = value
            .as_str()
            .ok_or_else(|| format!("`{name}` must be a type string, like `String`"))?
            .trim();
        if kind.is_empty() || kind.contains([';', '{', '}', '#', '\n']) {
            return Err(format!(
                "`{name}: {kind}` is not a type this action writes; it writes a simple type \
                 (`String`, `&'static str`, an integer, a float, a bool)"
            ));
        }
        fields.push((name.clone(), kind.to_owned()));
    }
    let _registry = load_registry(root, namespace)?;
    let id = resolve_node(root, namespace, target)?;
    let faces = face_views(root, namespace)?;
    let face = faces.iter().find(|face| face.id == id).ok_or_else(|| {
        format!("`{target}` does not name a face in this tree, so there is nothing to deepen")
    })?;
    let kind = face.kind.clone();
    let path = root.join("src").join(&face.source);
    // Every message names the file **relative to the package**, because a preview's absolute path is
    // a throwaway copy deleted a moment later — and the write path's own rule is that a preview must
    // never tell a caller to look somewhere that will not exist. An independent review of the first
    // cut caught this by quoting the `/tmp/nichlink-toolchain-preview-…` path out of a refusal.
    // 所有文案都用**相对包**的路径：预览的绝对路径是一份随后就被删掉的一次性副本，而写入路径自己的
    // 规矩就是"预览绝不能叫调用方去看一个随后不存在的目录"。第一版的独立复核正是从一条拒绝文案里
    // 把 `/tmp/nichlink-toolchain-preview-…` 抄出来抓到这个的。
    let named = format!("src/{}", face.source);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("{named} is not readable: {error}"))?;
    let marker = format!("pub struct {kind};");
    let markers = text.lines().filter(|line| line.trim() == marker).count();
    if markers != 1 {
        return Err(format!(
            "deepen needs the line `{marker}` in {named} exactly once to hold the layer, and found \
             it {markers} time(s); this action deepens a face whose shape it can read rather than \
             guessing at another shape"
        ));
    }
    let listed = fields
        .iter()
        .map(|(name, _)| format!("\"{name}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let declarations = fields
        .iter()
        .map(|(name, held_kind)| {
            format!(
                "    /// The {name} this face carries.\n    /// 这个面携带的 {name}。\n    \
                 pub {name}: {held_kind},\n"
            )
        })
        .collect::<String>();
    let held = format!("pub struct {kind} {{\n    parts: {kind}Parts,\n}}");
    let mut text = text.replacen(&marker, &held, 1);
    text.push_str(&format!(
        "\n/// The layer inside this face: the parts its own declaration already describes.\n\
         /// 这个面内部的那一层：它自己的声明已经描述过的那些零件。\npub struct {kind}Parts {{\n\
         {declarations}}}\n\
         \nimpl nichlink_toolchain::runtime::PartsContract for {kind}Parts {{\n\
         \x20   type Output = {kind}Parts;\n\
         \x20   const PROVIDED_PARTS: &'static [&'static str] = &[{listed}];\n}}\n\
         \nimpl {kind} {{\n\
         \x20   /// The layer this face carries.\n\
         \x20   /// 这个面携带的那一层。\n\
         \x20   pub fn parts(&self) -> &{kind}Parts {{\n\
         \x20       &self.parts\n\
         \x20   }}\n}}\n"
    ));
    std::fs::write(&path, text).map_err(|error| format!("{named} is not writable: {error}"))?;
    Ok(Outcome {
        message: format!(
            "deepened {}: `{kind}` now holds `{kind}Parts` with {} part(s) ({listed}); its \
             declaration, its public path and its tree row were not touched",
            face.path,
            fields.len()
        ),
        source: path,
        declaration: None,
        moved: false,
        alternative: true,
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
fn overlay(
    authored: &mut crate::runtime::AuthoredFace,
    fields: &Value,
    action: Action,
) -> Result<(), String> {
    // The same predicate `add` uses, so the two actions cannot drift apart on which key
    // they accept or on how a value's type is answered.
    // 与 `add` 用同一个判定，因此两个动作在"接受哪个键"和"值的类型怎么答"上不会漂移。
    if let Some(problem) = invalid_field(fields, action) {
        return Err(problem);
    }
    let Some(object) = fields.as_object() else {
        return Ok(());
    };
    for (key, value) in object {
        let text = || {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                format!(
                    "`{key}` must be a string (this shape takes strings, not arrays or objects)\n{}",
                    editable_fields_line()
                )
            })
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
                authored.needs_registry = value.as_bool().ok_or_else(|| {
                    format!(
                        "`needs_registry` must be true or false\n{}",
                        editable_fields_line()
                    )
                })?;
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

/// The verdict line every read tool prints when the connector refuses this package's
/// own faces.
/// 连接器拒绝本包自己的面时，每个读工具都会打印的那句判断。
///
/// One fact, one spelling: `converge` prints it in its verdict block and `usages`
/// prints it instead of returning an error, so an agent routing on `isError` — or
/// reading the body — gets the same answer from either. The write path is the one
/// place it stays an error, because a request it cannot carry out is a tool failure
/// rather than a fact about the tree.
/// 一个事实一种拼法：`converge` 在它的判断块里打印它，`usages` 打印它而不是返回一个错误，因此
/// 无论代理按 `isError` 分流还是读正文，从哪一个工具得到的都是同一个答案。写入路径是它仍作为错误
/// 的唯一地方，因为一次它无法执行的请求是工具故障，而不是关于这棵树的事实。
pub(crate) const REJECTED_VERDICT: &str = "kernel verdict: this package's own faces are rejected";

/// Build the registry the executor validates against: the package's own faces,
/// under the namespace Cargo reports.
/// 构建执行器据以校验的注册树：该包自己的注册面，位于 Cargo 报告的命名空间之下。
pub(crate) fn load_registry(root: &Path, namespace: &str) -> Result<Registry, String> {
    let mut registry =
        Registry::root_for_namespace(nichlink_kernel::FrameworkId::new("nichlink.mcp"), namespace);
    let source_root = source_layout(root)?.scan_root;
    let snapshots = AuthoringContext::new(root.to_path_buf(), namespace.to_owned())
        .scope(|| crate::runtime::generated_snapshots_from(&source_root))?;
    registry
        .register_snapshot_batch(snapshots)
        .map_err(|error| format!("the package's own faces were rejected: {error}"))?;
    Ok(registry)
}

/// What happened, in the shape the agent reads next.
/// 发生了什么，以代理接下来要读的形状给出。
fn report(
    root: &Path,
    target: &Target,
    namespace: &str,
    outcome: &Outcome,
) -> Result<String, String> {
    let applied = target.applied();
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
    let faces = face_views(target.work_dir(root), namespace)?;
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
    // The executor reports what it did in the past tense, and in a preview it did
    // that on the copy: printing the sentence bare reads as a claim that this
    // project changed, which is the one thing a preview must never say. The label
    // scopes it to the preview instead of hiding it, because the sentence carries
    // the parent identity and the old-to-new module names the diff does not.
    // 执行器用过去时报告它做了什么，而在预览里它是在副本上做的：把这句原样打印，读起来就是
    // "本项目已改变"，而这正是预览绝对不能说的话。给它一个标签、把它限定在预览里，而不是藏
    // 起来——那句话里带着 diff 没有的父级身份与改名前后。
    let reported = if applied {
        outcome.message.clone()
    } else {
        format!("preview effect: {}", outcome.message)
    };
    let consequences = consequences(target.work_dir(root), root, namespace, outcome)?;
    let mut reply = format!(
        "action {}\nnamespace {namespace}\n{}\n{verb} {}\n{declaration}{reported}\nfaces {}\n{list}\n{consequences}",
        if applied { "apply" } else { "preview" },
        editable_fields_line(),
        outcome.source.display(),
        faces.len()
    );
    if outcome.alternative {
        // The other reading of "make it deeper", priced. Family b of the round measured that this
        // decision is what costs the iterations — the tree, the public path, the slot and four
        // families of pins all move together — so the write path states both readings and lets the
        // caller choose instead of letting a red run do the arithmetic.
        // "把它做深"的另一种读法，连同它的价钱。五族 b 量到的正是：这个决定才是迭代成本所在——树、
        // 公开路径、槽位与四类钉子是一起动的——因此写入路径把两种读法都摆出来让调用方选，而不是让
        // 一次红运行去做这道算术。
        reply.push_str(
            "alternative (the other reading of `make it deeper`): hang another face **under** this \
             one — that needs `needs_registry: true` plus its own `registration_rule`, adds a row \
             per child to the registration tree, adds a segment to this face's public module path, \
             forces its graft slot to `full graft`, and moves the factory-shape assertions that \
             count faces, scope rows and slots; the layer above was measured to leave all of them \
             alone\n",
        );
        // The one shape change this action *does* make, said before the write: the face stops being a
        // unit struct. An independent review of the first cut listed it as a boundary, and a
        // consequence a caller has to discover by compiling is exactly what this block exists to
        // prevent.
        // 这个动作**确实**会带来的一处形状变化，写在写之前：这个面不再是单位结构体。第一版的独立复核
        // 把它列为一条边界，而"要编译一次才知道的后果"正是这一节要消灭的东西。
        reply.push_str(
            "note   this face stops being a unit struct: anything that used the type as a value \
             (`let _ = Kind;`, a literal pattern) has to be updated by hand — this action writes the \
             face's own file and does not touch callers\n",
        );
    }
    Ok(reply)
}

/// The editable fields, as a request must spell them: the shape the refusals point at and every
/// successful reply repeats. `--list <tool>` carries the same list, but the round measured that
/// nothing calls it (zero uses in seventeen subjects), so the shape travels with the answer instead.
/// 可编辑字段，按请求必须写出的形状：拒绝文案指向它，每次成功的回复也重复它。`--list <tool>` 载着同一
/// 份清单，但那轮量到**没有任何一次调用**去读它（17 个样本里 0 次），因此形状随答案一起走。
fn editable_fields_line() -> String {
    format!(
        "fields: {} — every value is a string; `needs_registry` is the one boolean, and \
         `exports`/`handle_traits`/`requires` are spelled as strings rather than arrays",
        EDITABLE_FIELDS.join(" ")
    )
}

/// What this change will make the rest of the tree say, as static facts with their boundary.
/// 这次改动会让这棵树的其他部分说什么——按静态事实给出，并带上它们的边界。
///
/// Both families of the round-5 evaluation spent their most expensive iterations discovering this by
/// experiment: a new face moves the factory enumerations, and "the application ships this face" is a
/// different question from "the build discovered it". Both facts are already in the tree, so the
/// write path states them before the write instead of leaving them to be rediscovered by a red run.
/// 第五轮评测的两个族都把最贵的迭代花在"靠实验发现这件事"上：新面会移动出厂形状的枚举，而"这个应用
/// 是否发布这个面"与"构建发现了它"是两个问题。两件事本来就写在这棵树里，因此写入路径在写之前把它们
/// 说出来，而不是留给一次红运行去重新发现。
fn consequences(
    work: &Path,
    project: &Path,
    namespace: &str,
    outcome: &Outcome,
) -> Result<String, String> {
    let changed = outcome
        .source
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    if changed.is_empty() {
        return Ok(String::new());
    }
    let faces = face_views(work, namespace)?;
    let Some(face) = faces.iter().find(|face| face.source.ends_with(&changed)) else {
        return Ok(String::new());
    };
    let leaf = face.path.rsplit('/').next().unwrap_or_default().to_owned();
    let tokens = [face.registry_name.clone(), face.kind.clone(), leaf];
    let sources = crate::mcp::source_index::load_sources(project)?;
    const PINS: usize = 5;
    let mut pins: Vec<String> = Vec::new();
    let mut shipped_by_a_cut = false;
    for file in &sources {
        let test = crate::mcp::callgraph::looks_like_a_test(&file.relative, &file.source);
        for (index, line) in file.source.lines().enumerate() {
            let names_it = tokens
                .iter()
                .any(|token| !token.is_empty() && line.contains(token.as_str()));
            if !names_it {
                continue;
            }
            if test {
                let text = line.trim();
                let kept: String = text.chars().take(96).collect();
                pins.push(format!("  {}:{}  {kept}", file.relative, index + 1));
            }
            if line.contains("cut(") || line.contains("graft(") {
                shipped_by_a_cut = true;
            }
        }
    }
    let cuts = sources
        .iter()
        .map(|file| file.source.matches("cut(").count())
        .sum::<usize>();
    let grafts = sources
        .iter()
        .map(|file| file.source.matches("graft(").count())
        .sum::<usize>();
    let mut report = format!(
        "consequences (static, text-level): {} in-tree test line(s) name this face\n",
        pins.len()
    );
    for line in pins.iter().take(PINS) {
        report.push_str(line);
        report.push('\n');
    }
    if pins.len() > PINS {
        report.push_str(&format!(
            "{}\n",
            crate::mcp::truncation::withheld(
                pins.len() - PINS,
                pins.len(),
                PINS,
                "pin lines",
                "grep the face's name in test files"
            )
        ));
    }
    report.push_str(&format!(
        "  entry plan: {cuts} `cut(` and {grafts} `graft(` site(s); this face's name appears at {} \
         of them — whether the application ships it is the plan's own business\n",
        if shipped_by_a_cut { "one" } else { "none" }
    ));
    report.push_str(
        "  not covered: this lists test lines that spell the face's name; a test that counts faces \
         without naming it, or reaches it through another spelling, does not appear here — run the \
         suite before believing either list\n",
    );
    Ok(report)
}

/// An executor refusal, plus the way forward when the wall is one the round measured.
/// 执行器的拒绝；当这堵墙是那轮量到的那一堵时，附上继续走的两条路。
///
/// Two walls cost the evaluation real time, and both are decidable from the message itself: a face
/// cannot take children until it declares a registry of its own, and a face that owns children
/// cannot stay behind a plain cut. Naming the two ways forward is not a licence to skip the rule —
/// the executor still refuses — it is the difference between "no" and "no, and here is what yes
/// needs".
/// 有两堵墙花了评测的真实时间，而两堵都能从消息本身判定：一个面在声明自己的注册机之前不能接收子级；
/// 而一个面一旦拥有子级，它的槽位就不能再是普通切口。点出两条路不是绕过规则的许可证——执行器照样
/// 拒绝——它是"不行"与"不行，而'行'需要什么"之间的差别。
fn refused_with_a_way_forward(error: String) -> String {
    // The kernel spells it `does not own a Registry`; the match is case-folded so a wording change in
    // its capitalisation cannot silently drop the way forward.
    // 内核把它写成 `does not own a Registry`；这里按大小写折叠来匹配，免得它改了首字母就把"继续走的
    // 路"悄悄丢掉。
    let folded = error.to_lowercase();
    if folded.contains("does not own a registry") {
        return format!(
            "{error}\nway forward: either declare `needs_registry: true` on that parent together \
             with its own `registration_rule` (then a child can hang under it), or attach this face \
             to a parent that already owns a registry — both are changes to declarations, and the \
             kernel's rule stays as it is"
        );
    }
    if folded.contains("non-empty child registry") {
        return format!(
            "{error}\nway forward: the slot whose face now owns children has to be declared as a \
             full replacement (`cut(…) full graft(…)`) rather than a plain cut, or those children \
             have to move out from under it — the factory assertions move with either decision, and \
             that is the decision, not a workaround"
        );
    }
    error
}

#[cfg(test)]
#[path = "apply_tests.rs"]
mod apply_tests;
