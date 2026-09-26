//! Tool catalog, the query implementations, and the write path's dispatch.
//! 工具目录、查询实现与写入路径的分派。
//!
//! Five tools read Rust source text. Six more answer from evidence that is not
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
//! 五个工具读取 Rust 源码文本，另外六个用非源码文本的证据作答：`nichlink.registry` 推导出构建
//! 所推导的那棵树（`nichlink_build_method::face_views`）；`nichlink.explain` 读构建**发布**的文件
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
use crate::index::{load_one, load_sources, required_path, resolve_root};
use crate::mir::{mir, unified};
use crate::protocol::{DEFAULT_LIMIT, MAX_READ_LINES, error_response, success};
use crate::registry::registry;
use crate::trace::trace;
use crate::usages::usages;
use crate::verify::verify;

pub(crate) fn tools() -> Vec<Value> {
    vec![
        tool(
            "nichlink.search",
            "Find source files and Rust function declarations by name.",
            json!({"type":"object","properties":{"query":{"type":"string"},"root":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200}},"required":["query"]}),
        ),
        tool(
            "nichlink.inspect",
            "Summarize functions and registration declarations in one Rust file.",
            json!({"type":"object","properties":{"path":{"type":"string"},"root":{"type":"string"}},"required":["path"]}),
        ),
        tool(
            "nichlink.callgraph",
            "Show direct static callers and callees for one function.",
            json!({"type":"object","properties":{"function":{"type":"string"},"path":{"type":"string"},"root":{"type":"string"}},"required":["function"]}),
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
             reports — or by identity. **A request is previewed unless `apply` is true**: the \
             preview runs the real operation on a throwaway copy and returns the file diff plus \
             the registration tree it produces; `apply: true` writes it and names the files it \
             wrote. Every reply is the tree that results, so the next call can be aimed with it.",
            json!({"type":"object","properties":{
                "action":{"type":"string","enum":["add","edit","rename","delete"]},
                "node":{"type":"string","description":"edit/rename/delete: the face, by logical path or identity"},
                "parent":{"type":"string","description":"add: the parent's logical path or identity; defaults to the registry root"},
                "fields":{"type":"object","description":"the face's fields; edit and rename change only the keys given, add takes the rest as defaults"},
                "apply":{"type":"boolean","description":"false (the default) previews on a copy; true writes to the project"},
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
             module, parent, slot — or the tree projection the build scoped. `nichlink.registry` \
             derives from source text and is therefore always fresh; this reads the files the build \
             *published* under `target/nichlink/out`, so it answers what ships: whether the scope \
             selected the face and whether release pruning strips its symbols. A missing or stale \
             build is reported as such, and an unbuilt project is told which command publishes the \
             evidence. `node` names one face by logical path or identity; omit it for the projection, \
             bounded by `limit`. Declared graft state stays in `nichlink grafts`; contract and \
             admission fields need a loaded registry and are not reported here.",
            json!({"type":"object","properties":{"node":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.diff",
            "Report the face-level delta between the sources now and the build's own manifest: which \
             faces were added, which are gone, and which changed identity under a file that did not \
             move (a `kind` change is an identity change, so only this comparison sees it). The unit \
             is the face rather than the line, because the face is what the registry ships. Needs a \
             prior `nichlink check` or `build`; a project with no build evidence is told so instead \
             of being handed an empty diff.",
            json!({"type":"object","properties":{"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.trace",
            "Read this package's recorded trace artifact (`NICH_LINK_TRACE_FILE`, else \
             `<root>/.nichlink/traces/nichlink.trace`) and answer with the headless call report it \
             implies — what actually ran, which no static read can tell you. The artifact's identity \
             is checked first (namespace, registry root, every frame's node), and an artifact that \
             describes a different tree is refused by name rather than rendered, because frames are \
             node identities and foreign ones would draw a plausible, wrong call tree. Absence is \
             reported together with the way to produce one; `query` filters the report and a long \
             report is truncated with its total named.",
            json!({"type":"object","properties":{"query":{"type":"string"},"root":{"type":"string"}}}),
        ),
        tool(
            "nichlink.mir",
            "Read a MIR artifact and report the compiler's call candidates, or emit it as canonical \
             JSONL. A `rustc -Zunpretty=mir` text dump and the compact JSONL artifact are both \
             accepted, chosen by extension: JSONL parses strictly, so a malformed line fails the \
             whole read, while a text dump never fails because a line that is not a call is simply \
             not a call. The JSONL form is the portable channel Studio could already render and \
             parse and nothing in this workspace ever wrote — `jsonl: true` makes this tool that \
             writer, and what it prints reads back here. The text producer stays \
             `cargo rustc -Zunpretty=mir` on a nightly toolchain; a missing artifact says exactly \
             that instead of reporting an empty graph.",
            json!({"type":"object","properties":{"path":{"type":"string"},"jsonl":{"type":"boolean"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["path"]}),
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
            "nichlink.usages",
            "Report the neighbourhood of one face: its parent and children as the tree has them, the \
             fields the write path accepts read back from the generated module (preset, parts, the \
             localized names, exports, requires, provides, handle traits and contracts, registration \
             rule, admission, flow, runtime checks) — so a contract `nichlink.apply` can set becomes \
             reportable — and which other faces mention the same capability tokens. Capability matches \
             are on declared tokens rather than a resolved graph, and the reply says so. A hand-written \
             module has no generated field list and the executor refuses to invent one, so those faces \
             are counted as unreadable rather than shown empty. Declared graft cuts are not reported \
             here; the CLI's `explain --json` carries them.",
            json!({"type":"object","properties":{"node":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["node"]}),
        ),
        tool(
            "nichlink.converge",
            "The converged starting point for one face in a single answer: the build's scope and \
             pruning verdicts, the tree's edges, the declared fields, and — the verdict no single \
             tool can give — whether each `capability=>ProviderKind` requirement is actually \
             answered by something in the package, named when it is and UNANSWERED when it is not. \
             Ends with the files to read (this face, its parent, its children) and which tool has \
             the detail. Everything is composed from what the other tools report.",
            json!({"type":"object","properties":{"node":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":200},"root":{"type":"string"}},"required":["node"]}),
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
    let result = match name {
        "nichlink.search" => search(&root, arguments),
        "nichlink.inspect" => inspect(&root, arguments),
        "nichlink.callgraph" => callgraph(&root, arguments),
        "nichlink.read" => read_source(&root, arguments),
        "nichlink.status" => status(&root),
        "nichlink.registry" => registry(&root),
        "nichlink.explain" => explain(&root, arguments),
        "nichlink.diff" => diff(&root, arguments),
        "nichlink.trace" => trace(&root, arguments),
        "nichlink.mir" => mir(&root, arguments),
        "nichlink.unified" => unified(&root, arguments),
        "nichlink.usages" => usages(&root, arguments),
        "nichlink.converge" => converge(&root, arguments),
        "nichlink.verify" => verify(&root, arguments),
        "nichlink.apply" => apply(&root, arguments),
        _ => Err(format!("unknown tool `{name}`")),
    };
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

fn search(root: &Path, arguments: &Value) -> Result<String, String> {
    let query = arguments
        .get("query")
        .and_then(Value::as_str)
        .ok_or_else(|| "nichlink.search requires query".to_owned())?
        .trim()
        .to_ascii_lowercase();
    if query.is_empty() {
        return Err("query must not be empty".to_owned());
    }
    let limit = arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize);
    let files = load_sources(root)?;
    let mut results = Vec::new();
    for file in &files {
        if file.relative.to_ascii_lowercase().contains(&query) {
            results.push(format!("file  {}", file.relative));
        }
        for function in &file.functions {
            if function.name.to_ascii_lowercase().contains(&query) {
                results.push(format!(
                    "fn    {} -> {}:{}",
                    function.name, file.relative, function.line
                ));
            }
        }
        if results.len() >= limit {
            break;
        }
    }
    if results.is_empty() {
        return Ok("no matches".to_owned());
    }
    results.truncate(limit);
    Ok(results.join("\n"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tools_list_is_mcp_shaped() {
        let listed = tools();
        assert!(listed.iter().any(|tool| tool["name"] == "nichlink.search"));
        assert!(
            listed
                .iter()
                .all(|tool| tool["inputSchema"]["type"] == "object")
        );
    }
    /// A line number a caller sends is unbounded; the range must stay forward and
    /// inside the file whatever it is.
    /// 调用方发来的行号没有上界；无论它是什么，区间都必须朝前且落在文件内。
    ///
    /// `center + context` used to overflow: debug panicked, release produced
    /// `start > end` with an empty body and `isError: false`.
    /// `center + context` 过去会溢出：debug 直接 panic，release 产出 `start > end`、正文为空、
    /// 却仍报 `isError: false`。
    #[test]
    fn a_huge_line_number_still_yields_a_forward_range() {
        let root = std::env::temp_dir().join(format!("nichlink-mcp-read-{}", std::process::id()));
        std::fs::create_dir_all(root.join("src")).expect("fixture dir");
        std::fs::write(root.join("src/lib.rs"), "fn a() {}\nfn b() {}\n").expect("fixture file");
        for line in [u64::MAX, u64::MAX / 2, usize::MAX as u64, 1] {
            let arguments = serde_json::json!({"path": "src/lib.rs", "line": line});
            let output = read_source(&root, &arguments).expect("the read succeeds");
            let header = output.lines().next().expect("a header");
            let range = header.split_once(':').expect("path:range").1;
            let (start, end) = range.split_once('-').expect("start-end");
            let start: usize = start.parse().expect("a start");
            let end: usize = end.parse().expect("an end");
            assert!(start <= end, "line {line} inverted the range: {header}");
            assert!(end <= 2, "line {line} read past the file: {header}");
        }
        let _ = std::fs::remove_dir_all(&root);
    }
}
