//! Tool catalog, the query implementations, and the write path's dispatch.
//! 工具目录、查询实现与写入路径的分派。
//!
//! The catalog below is the authority on how many tools there are and what each one reads. This
//! paragraph describes the *families* instead of counting them, because a count here goes stale the
//! moment a tool is added — it said "five + eight, plus `apply`" while the catalog held seventeen
//! (audit `L4`).
//! 下面的目录才是"有几个工具、每个读什么"的权威。这一段描述的是**几类**而不是数个数，因为这里
//! 的数字会在新加一个工具的那一刻过期——它曾说"五个 + 八个，外加 `apply`"，而目录里其实有十七个
//! （审计 `L4`）。
//!
//! One family reads Rust source text — `nichlink.search` also matches registry faces
//! (logical path, kind, module, registry_name) and annotates each with the build's verdict.
//! Another answers from evidence that is not
//! source text: `nichlink.registry` derives the tree the build derives
//! (`nichlink_build_method::face_views`), `nichlink.explain` reads the files the
//! build *published* (`target/nichlink/out`) for scope and release pruning,
//! `nichlink.diff` states the face-level delta between those two sides, and
//! `nichlink.trace` reads a recorded trace artifact, refusing one that describes
//! another tree; `nichlink.mir` reads a `-Zunpretty=mir` dump or a JSONL artifact
//! and can emit that JSONL, which nothing in the workspace ever wrote; and
//! `nichlink.unified` merges the two, where a live call confirms a compiler
//! candidate. `nichlink.apply` is the write path and previews before it writes
//! (`apply.rs` explains the contract). What none of them reports is contract,
//! admission, or registration-rule data: those live in the built
//! `RegistrationSnapshot`s, which need the compiled registrations rather than a
//! scan or a manifest.
//! 一类工具读取 Rust 源码文本——`nichlink.search` 还会匹配注册面（逻辑路径、kind、module、registry_name），
//! 并把构建的判断标在每一条上。另一类用非源码文本的证据作答：`nichlink.registry` 推导出构建所推导
//! 的那棵树（`nichlink_build_method::face_views`）；`nichlink.explain` 读构建**发布**的文件
//! （`target/nichlink/out`），回答作用域与发布剪枝；`nichlink.diff` 说出两侧的面级差异；
//! `nichlink.trace` 读取已记录的 trace artifact，并拒绝描述另一棵树的那份；`nichlink.mir` 读
//! `-Zunpretty=mir` 转储或 JSONL artifact，并能输出那份无人写过的 JSONL；`nichlink.unified` 把两者
//! 合并，真实调用在其中确认编译器候选。`nichlink.apply` 是写入路径，落盘前先预览（契约见 `apply.rs`）。
//! 它们都没有报告的是 contract、admission 与 registration rule 数据：那些住在已构建的
//! `RegistrationSnapshot` 里，需要已编译的注册，而不是扫描或清单。

use serde_json::{Value, json};
use std::path::Path;

use crate::apply::apply;
use crate::callgraph::callgraph;
use crate::converge::converge;
use crate::diff::diff;
use crate::evidence::explain;
use crate::grafts::grafts;
use crate::impact::impact;
use crate::index::{load_one, load_sources, required_path, resolve_root};
use crate::mir::{mir, unified};
use crate::overlay::overlay;
use crate::protocol::{MAX_READ_LINES, error_response, success};
use crate::registry::registry;
use crate::search::search;
use crate::trace::trace;
use crate::usages::usages;
use crate::verify::verify;

pub(crate) fn tools() -> Vec<Value> {
    vec![
        tool(
            "nichlink.search",
            "Find registration faces, source files, and Rust function declarations by name. Faces \
             come first and match on logical path, kind, module, or registry_name — the spellings the \
             other tools use — and each carries the build's verdict: `ok`, `added since build`, \
             `re-identified` (a `kind` change under an unmoved file, with both identities), or \
             `build unknown` when nothing has been published. A manifest that no longer describes \
             these sources is announced above the verdicts as `stale (run nichlink check)`, because \
             they then describe the tree that build saw rather than this one. A registration file \
             the derivation could not parse is counted as `unparsable faces N` above the hits, and \
             that line is **not** subject to `limit`: it reports the tree that was read rather \
             than one more result row, so it keeps appearing when the limit has already cut the \
             face list short. Below them the file \
             and function hits are unchanged. The tree half needs the identity namespace; a root \
             Cargo cannot name still answers the source half and says the tree half is unavailable.",
            json!({"type":"object","properties":{"query":{"type":"string"},"root":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200}},"required":["query"]}),
        ),
        tool(
            "nichlink.inspect",
            "Summarize functions and registration declarations in one Rust file.",
            json!({"type":"object","properties":{"path":{"type":"string"},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "nichlink.callgraph",
            "Show direct static callers and callees for one function. `limit` bounds how many \
             definitions are listed (default 5, at most 50); a truncation line names how many \
             were withheld, and `path` selects one definition when several share the name.",
            json!({"type":"object","properties":{"function":{"type":"string"},"path":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":50},"root":{"type":"string"}},"required":["function"]}),
        ),
        tool(
            "nichlink.read",
            "Read a bounded source window around a line.",
            json!({"type":"object","properties":{"path":{"type":"string"},"line":{"type":"integer","minimum":1},"context":{"type":"integer","minimum":0,"maximum":120},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "nichlink.status",
            "Report the source root and indexed Rust file/function counts.",
            json!({"type":"object","properties":{"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.apply",
            "Edit this package's registration faces through the same authoring executor Studio \
             uses, so the kernel's admission, parent-rule, and topology checks run on the change. \
             `action` is `add` (create `fields.module` under `parent`), `edit` (change the \
             `fields` the request names and keep the rest), `rename` (change `fields.module`), or \
             `delete` (move the face's module into NichLink's recoverable trash). `node` names \
             the face and `parent` the parent, by logical path — the one nichlink.registry \
             reports — or by identity. `handle_contracts` and `part_contracts` are set by `add` \
             only: the executor's edit field order does not carry them, so `edit`/`rename` refuse \
             those two keys by name rather than returning success while the value is dropped. \
             **A request is previewed unless `apply` is true**: the \
             preview runs the real operation on a throwaway copy and returns the file diff plus \
             the registration tree it produces; `apply: true` writes it and names the files it \
             wrote. Every reply is the tree that results, so the next call can be aimed with it.",
            json!({"type":"object","properties":{
                "action":{"type":"string","enum":["add","edit","rename","delete"]},
                "node":{"type":"string","description":"edit/rename/delete: the face, by logical path or identity"},
                "parent":{"type":"string","description":"add: the parent's logical path or identity; defaults to the registry root"},
                "fields":{"type":"object","description":"the face's fields; edit and rename change only the keys given, add takes the rest as defaults"},
                "apply":{"type":"boolean","description":"false (the default) previews on a copy; true writes to the project"},
                "confirm":{"type":"boolean","description":"delete: must be true. A delete is the one operation whose preview a caller can step past by accident, so the request says it rather than the bridge adding it"},
                "root":{"type":"string"}
            },"required":["action"]}),
        ),
        tool(
            "nichlink.registry",
            "Report the registration faces this package declares, as the build derives them: \
             logical path, kind, source, and the NodeId the host compiled. Contract, admission, \
             and registration-rule data need the built snapshots and are not included. `root` is \
             a package root; omitting it uses NICH_LINK_PACKAGE_ROOT.",
            json!({"type":"object","properties":{"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.explain",
            "Report the build's own evidence for one face — identity, logical path, kind, source, \
             module, parent, registry_name — or the tree projection the build scoped. \
             `nichlink.registry` derives from source text and is therefore never `stale`; this reads \
             the files the build *published* under `target/nichlink/out`, so it answers what ships: \
             whether the scope selected the face and whether release pruning strips its symbols. \
             The `build` line says `current`, or `stale (run nichlink check)` when the published \
             output no longer describes these sources; an unbuilt project is told which command \
             publishes the evidence. `node` names one face by logical path or identity; omit it for \
             the projection, bounded by `limit`. With `overlay: true` it renders the *overlay* \
             projection instead — which slot each declared cut replaces, and which faces the scope \
             prunes — the published state after replacement, from the same traversal the CLI's \
             `explain --overlay` uses. That is a static projection and not `Registry::dump`: an \
             overlay needs two live registries, and the reply carries the note saying where the \
             live tree comes from. `overlay` and `node` are mutually exclusive. Declared graft \
             state stays in `nichlink grafts`; contract and admission fields need a loaded registry \
             and are not reported here.",
            json!({"type":"object","properties":{"node":{"type":"string"},"overlay":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.diff",
            "Report the face-level delta between two sides of this package. By default the sources \
             now against the build's own manifest: which faces were added, which are gone, and which \
             changed identity under a file that did not move (a `kind` change is an identity change, \
             so only this comparison sees it). The unit is the face rather than the line, because the \
             face is what the registry ships. Needs a prior `nichlink check` or `build`; a project \
             with no build evidence is told so instead of being handed an empty diff. With \
             `records: true` the other pair: every external graft record against the sources, where a \
             record stores the identity it was written for, so a face that changed identity under an \
             unmoved slot breaks it silently. A record comes back `ok`; `undeclared` (the identity \
             is in the tree, but no `static_graft_plan!` cut names its slot — the release prunes \
             that slot, which is the same verdict `nichlink.grafts` gives and the same one the \
             build refuses); `stale` (nothing in the tree has that identity or that path); \
             `re-identified` (the path is there, the identity moved); or `unreadable` (a record \
             file that could not be read), which is counted rather than dropped. `undeclared` wins \
             over `stale`/`re-identified`, because whether a record can ever take effect is the \
             question that changes what a caller does. There is no separate bucket for a typed \
             cut: the plan's path is always a logical path (every writer uses `registry.path_for`), \
             and with the identity absent nothing resolves a typed declaration's module, so such a \
             record is judged by the declaration rule like any other — and the reply prints the \
             path it looked for rather than guessing.",
            json!({"type":"object","properties":{"records":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.trace",
            "Read this package's recorded trace artifact (`NICH_LINK_TRACE_FILE`, else \
             `<root>/.nichlink/traces/nichlink.trace`) and answer with the headless call report it \
             implies — what actually ran, which no static read can tell you. With `values: true` the \
             same read answers what that run *saw* instead: the recorded locals grouped by the frame \
             they belong to (name, type, rendered value, role, observed-or-inferred, callsite), the \
             locals recorded outside every traced call, and the observed data edges between them. A \
             local or edge naming something the artifact does not contain is counted rather than \
             dropped, so an incomplete artifact cannot read as a complete one. The artifact's identity \
             is checked first (namespace, registry root, every frame's node), and an artifact that \
             describes a different tree is refused by name rather than rendered, because frames are \
             node identities and foreign ones would draw a plausible, wrong call tree. Absence is \
             reported together with the way to produce one; `query` filters either report and a long \
             one is truncated with its total named.",
            json!({"type":"object","properties":{"query":{"type":"string"},"values":{"type":"boolean"},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.mir",
            "Read a MIR artifact and report the compiler's call candidates, emit it as canonical \
             JSONL, or report the call-graph delta against another artifact. A `rustc \
             -Zunpretty=mir` text dump and the compact JSONL artifact are both accepted, chosen by \
             extension: JSONL parses strictly, so a malformed line fails the whole read, while a \
             text dump never fails because a line that is not a call is simply not a call. The JSONL \
             form is the portable channel Studio could already render and parse and nothing in this \
             workspace ever wrote — `jsonl: true` makes this tool that writer, and what it prints \
             reads back here. What it writes is a *snapshot*: a header naming the identity namespace \
             and registry root of the tree the artifact came from, which is what makes two artifacts \
             comparable — a snapshot of another tree is refused by name, and an unidentified one (a \
             text dump can't name its tree) makes the delta say what it cannot rule out. With \
             `against`, that other artifact is the baseline and the reply is the call-graph delta \
             from it forward: added and gone relations, plus function symbols. The text producer \
             stays `cargo rustc -Zunpretty=mir` on a nightly toolchain; a missing artifact says \
             exactly that instead of reporting an empty graph.",
            json!({"type":"object","properties":{"path":{"type":"string"},"against":{"type":"string"},"jsonl":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "nichlink.unified",
            "Merge a MIR artifact with this package's recorded trace: the compiler's call candidates \
             together with what actually ran, where a live call confirms its candidate and carries \
             `evidence=Live` while the rest stay `evidence=Mir`. This is `debug_method`'s \
             `UnifiedCallGraph`, the one place the two evidence sources are joined, so the merge \
             cannot drift from the library's own. The trace is read from this package's artifact \
             path; when none has been recorded the merge still answers and labels every relation a \
             compiler candidate rather than failing, because an absent trace is a weaker answer and \
             not a broken one.",
            json!({"type":"object","properties":{"path":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "nichlink.grafts",
            "Every external graft plan under `.nichlink/external-grafts/<selector>/graft.plan` \
             joined with the host entry's `static_graft_plan!` declarations: the selector, the \
             logical path and replacement each plan targets, whether it covers the whole subtree, \
             and whether the host entry names that slot. A plan the entry does not declare is \
             reported as `NOT declared by the host entry` and counted under `unkept plans`: the \
             release prunes that slot, so the record can never take effect — the build warns about \
             it and a cargo log is where that warning goes to die. An unreadable plan carries its \
             reason rather than being skipped, and with no readable declaration the state is \
             `declaration unknown` rather than a false `not declared`. A declaration is about \
             the *slot* a plan targets, not the implementation it selects, so each row shows the \
             plan's own target and graft next to the declaration's cut and graft. Read-only.",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":400},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.impact",
            "The transitive blast radius of a change to one face, over the three dependency kinds \
             this tree actually declares: a face's descendants (its registry owns them), the faces \
             whose `requires` names a capability it provides, and the declared graft cuts that hand \
             it over. Each reached node carries its shortest hop distance and the chain of reasons \
             that got there, so a capability cycle is a shorter path rather than a hang. A face the \
             traversal did not reach is reported as unreached within `depth` — which is not proof of \
             independence — and graft records or recorded traces naming the same identity are not \
             traversed, which the reply states.",
            json!({"type":"object","properties":{"node":{"type":"string"},"depth":{"type":"integer","minimum":1,"maximum":16},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["node"]}),
        ),
        tool(
            "nichlink.usages",
            "Report the neighbourhood of one face: its parent and children as the tree has them, every \
             field the write path accepts read back from the generated module (`module`, `preset`, \
             `parts`, `name_zh`, `name_en`, `summary_zh`, `summary_en`, `stable_name`, `exports`, \
             `requires`, `provides`, `handle_traits`, `handle_contracts`, `part_traits`, \
             `part_contracts`, `registration_rule`, `admission`, `flow`, `flow_provider`, \
             `runtime_checks`, `getting_from_other_registry`, `needs_registry`) — so every field \
             `nichlink.apply` can set becomes reportable — and which other faces mention the same \
             capability tokens. Capability matches are on declared tokens rather than a resolved graph, \
             and the reply says so. A hand-written module has no generated field list and the executor \
             refuses to invent one, so those faces are counted as unreadable rather than shown empty. \
             Declared graft cuts are not reported here; the CLI's `explain --json` carries them.",
            json!({"type":"object","properties":{"node":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["node"]}),
        ),
        tool(
            "nichlink.converge",
            "The converged starting point for one face in a single answer, or for a recorded run. With \
             `node`, it returns the build's scope and pruning verdicts, the tree's edges, the declared \
             fields, and — the verdict no single tool can give — whether each \
             `capability=>ProviderKind` requirement is actually answered by something in the package, \
             named when it is and UNANSWERED when it is not; it ends with the files to read (this \
             face, its parent, its children) and which tool has the detail. With `trace: true` it \
             starts from the recorded run instead and collapses the whole tree to the files that both \
             declare a face and actually ran, with the frames that landed in each; frames are matched \
             to faces by source file, which the reply states, because a face is a declaration and a \
             frame is a function. Everything is composed from what the other tools report.",
            json!({"type":"object","properties":{"node":{"type":"string"},"trace":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.verify",
            "Run the kernel's registration validation over this package and report the tree delta the \
             run just published. It drives the same entry the CLI's `check` drives, so a verdict here \
             cannot drift from `nichlink check`, and it refreshes the build evidence as a side effect — \
             which is why the delta below it describes the tree that was just verified rather than the \
             last build. A failed verdict is the answer and not a tool failure: the reply says `verdict \
             failed` with the diagnostics (each naming its phase, node, source and line) and `isError` \
             stays false, because the verification itself succeeded.",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": input_schema })
}

/// One tool's implementation: the package root plus the `arguments` object.
/// 一个工具的实现：包根加上 `arguments` 对象。
type Handler = fn(&Path, &Value) -> Result<String, String>;

/// The dispatch table, in the catalog's order.
/// 分派表，顺序与目录一致。
///
/// This is one ordered table rather than a `match` because the two lists used to be
/// aligned only by hand: the catalog put `nichlink.apply` sixth while the dispatch put
/// it last, and adding a tool to one side alone was silent — the missing arm only
/// surfaced at call time as `unknown tool` (audit `BR-12`).
/// `tools_tests::the_dispatch_table_follows_the_catalog` pins this table's names and
/// their order against `tools()`, so a tool added to the catalog without a handler, or
/// in another position, fails the suite instead of shipping.
/// 这里用一张有序表而不是 `match`，因为两张名单过去只靠手工对齐：目录把 `nichlink.apply` 放在第 6，
/// 分派把它放在最后；而只往一侧新增工具是静默的——缺的那条臂只在调用时以 `unknown tool` 现身
/// （审计 `BR-12`）。`tools_tests::the_dispatch_table_follows_the_catalog` 用 `tools()` 钉住本表的
/// 名字与顺序：只往目录里加工具而漏掉处理函数、或把它放错位置，都会让测试失败而不是出厂。
const DISPATCH: &[(&str, Handler)] = &[
    ("nichlink.search", search),
    ("nichlink.inspect", inspect),
    ("nichlink.callgraph", callgraph),
    ("nichlink.read", read_source),
    ("nichlink.status", status_tool),
    ("nichlink.apply", apply),
    ("nichlink.registry", registry_tool),
    ("nichlink.explain", explain_tool),
    ("nichlink.diff", diff),
    ("nichlink.trace", trace),
    ("nichlink.mir", mir),
    ("nichlink.unified", unified),
    ("nichlink.grafts", grafts),
    ("nichlink.impact", impact),
    ("nichlink.usages", usages),
    ("nichlink.converge", converge),
    ("nichlink.verify", verify),
];

/// `nichlink.status` reads no arguments, so it joins the table through a shim.
/// `nichlink.status` 不读参数，因此经一个转接函数进入分派表。
fn status_tool(root: &Path, _arguments: &Value) -> Result<String, String> {
    status(root)
}

/// `nichlink.registry` reads no arguments either (its `root` is resolved above).
/// `nichlink.registry` 同样不读参数（它的 `root` 已在上面解析）。
fn registry_tool(root: &Path, _arguments: &Value) -> Result<String, String> {
    registry(root)
}

/// `nichlink.explain` is the one name that routes to two implementations: `overlay:
/// true` renders the overlay projection, everything else the per-face report.
/// `nichlink.explain` 是唯一会分到两个实现的名字：`overlay: true` 渲染覆盖投影，其余走逐面报告。
fn explain_tool(root: &Path, arguments: &Value) -> Result<String, String> {
    if arguments.get("overlay").and_then(Value::as_bool) == Some(true) {
        overlay(root, arguments)
    } else {
        explain(root, arguments)
    }
}

pub(crate) fn tool_call(root: &Path, id: Value, params: &Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error_response(id, -32602, "tools/call requires name".to_owned());
    };
    let arguments = params.get("arguments").unwrap_or(&Value::Null);
    let requested_root = arguments.get("root").and_then(Value::as_str);
    let root = match resolve_root(root, requested_root) {
        Ok(root) => root,
        Err(error) => {
            return success(
                id,
                json!({ "content": [{"type":"text","text":error}], "isError": true }),
            );
        }
    };
    let result = DISPATCH
        .iter()
        .find(|(listed, _)| *listed == name)
        .map_or_else(
            || Err(format!("unknown tool `{name}`")),
            |(_, handler)| (*handler)(&root, arguments),
        );
    match result {
        Ok(value) => success(
            id,
            json!({ "content": [{"type":"text","text":value}], "isError": false }),
        ),
        Err(error) => success(
            id,
            json!({ "content": [{"type":"text","text":error}], "isError": true }),
        ),
    }
}

fn inspect(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = required_path(arguments)?;
    let file = load_one(root, &relative)?;
    let mut output = format!("file {}\n", file.relative);
    for function in &file.functions {
        output.push_str(&format!(
            "fn {} lines {}-{} calls=[{}]\n",
            function.name,
            function.line,
            function.end_line,
            function.calls.join(", ")
        ));
    }
    let registrations = nichlink::source::registration_kinds(&file.source);
    if !registrations.is_empty() {
        output.push_str("registrations: ");
        output.push_str(&registrations.join(", "));
        output.push('\n');
    }
    if file.functions.is_empty() && registrations.is_empty() {
        output.push_str("no function or registration declaration found\n");
    }
    Ok(output)
}

fn read_source(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = required_path(arguments)?;
    let file = load_one(root, &relative)?;
    let total = file.source.lines().count();
    let center = arguments
        .get("line")
        .and_then(Value::as_u64)
        .map_or(1, |line| line.max(1) as usize);
    let context = arguments
        .get("context")
        .and_then(Value::as_u64)
        .map_or(40, |value| value.min(120) as usize);
    // A caller-supplied line number is unbounded, and `center + context` used to
    // overflow: a panic in a debug build, and in release a wrapped range whose start is
    // past its end while the reply still says `isError: false` — a wrong answer an agent
    // would trust. Clamp the centre to the file first, then do the arithmetic
    // saturating.
    // 调用方给的行号没有上界，而 `center + context` 过去会溢出：debug 构建里 panic，release
    // 里回绕成一个起点超过终点的区间、回复却仍写着 `isError: false`——这是 agent 会相信的错误
    // 答案。先把中心夹到文件内，再用饱和运算做后面的加法。
    let total = total.max(1);
    let center = center.min(total);
    let start = center.saturating_sub(context).max(1);
    let end = center
        .saturating_add(context)
        .min(total)
        .min(start.saturating_add(MAX_READ_LINES - 1));
    let mut output = format!("{}:{}-{}\n", file.relative, start, end);
    for (index, line) in file.source.lines().enumerate() {
        let line_number = index + 1;
        if (start..=end).contains(&line_number) {
            output.push_str(&format!("{line_number:>5} | {line}\n"));
        }
    }
    Ok(output)
}

fn status(root: &Path) -> Result<String, String> {
    let files = load_sources(root)?;
    let functions = files.iter().map(|file| file.functions.len()).sum::<usize>();
    Ok(format!(
        "root {}\nrust_files={} functions={} tool=nichlink-mcp",
        root.display(),
        files.len(),
        functions
    ))
}

#[cfg(test)]
#[path = "tools_tests.rs"]
mod tools_tests;
