//! The compiler's call candidates, and the merge with what actually ran.
//! 编译器给出的调用候选，以及它与"真正跑了什么"的合并。
//!
//! MIR JSONL was a format with a parser, a renderer, and no writer anywhere in the
//! workspace; a `-Zunpretty=mir` text dump was readable only inside Studio and only
//! on a nightly toolchain. These two tools close both halves without re-implementing
//! the parse: `nichlink.mir` reads either form and can emit the canonical JSONL, and
//! `nichlink.unified` hands a graph plus a recorded trace to
//! `debug_method`'s `UnifiedCallGraph`, which is the one place a live call confirms a
//! compiler candidate. The producer of the *text* stays outside this bridge — it is
//! `cargo rustc -Zunpretty=mir` on a nightly toolchain — and saying so is part of the
//! answer rather than a gap to hide.
//! MIR JSONL 过去是一种"有解析器、有渲染器、整个工作区没有写入方"的格式；而 `-Zunpretty=mir` 文本
//! 此前只能在 Studio 里读、且需要 nightly 工具链。这两个工具把两半都补上，而不重新实现解析：
//! `nichlink.mir` 读任一形式并能输出规范 JSONL，`nichlink.unified` 把图与已记录的 trace 交给
//! `debug_method` 的 `UnifiedCallGraph`——真实调用确认编译器候选的唯一地方。*文本*的生产者仍在本桥
//! 之外（nightly 上的 `cargo rustc -Zunpretty=mir`），把这一点说出来是答案的一部分，而不是要藏的缺口。

use std::path::{Path, PathBuf};

use nichlink::EvidenceKind;
use nichlink::mir::MirGraph;
use nichlink_run_method::{CallTrace, read_trace_artifact, trace_artifact_path};
use serde_json::Value;

use crate::protocol::DEFAULT_LIMIT;

/// The most rows or lines one reply carries before it says it truncated.
/// 一条回复在声明被截断之前最多携带的行数。
const MAX_ROWS: usize = 400;

/// Read a MIR artifact and report the graph, or emit it as canonical JSONL.
/// 读取一个 MIR artifact 并报告该图，或把它输出为规范 JSONL。
pub(crate) fn mir(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "nichlink.mir requires path (a MIR text dump or a JSONL artifact)".to_owned()
        })?;
    let (path, graph) = load(root, relative)?;
    if arguments.get("jsonl").and_then(Value::as_bool) == Some(true) {
        // The portable channel's missing writer: Studio could render this and parse
        // it, and nothing ever produced one.
        // 可移植通道缺失的写入方：Studio 能渲染它、也能解析它，而从没有任何东西产出过一份。
        return Ok(bounded(
            &graph.to_jsonl(),
            "lines",
            "pass what this prints to a JSONL-suffixed file and `nichlink.mir` reads it back",
        ));
    }
    let limit = limit(arguments);
    let mut output = format!(
        "file {}\nfunctions {} calls {} locals {}\n",
        path.display(),
        graph.functions.len(),
        graph.calls.len(),
        graph.locals.len()
    );
    output.push_str("calls:\n");
    for call in graph.calls.iter().take(limit) {
        output.push_str(&format!(
            "  {} -> {} (mir line {})\n",
            call.caller, call.callee, call.mir_line
        ));
    }
    if graph.calls.len() > limit {
        output.push_str(&format!("  … +{} more\n", graph.calls.len() - limit));
    }
    output.push_str("locals:\n");
    for local in graph.locals.iter().take(limit) {
        output.push_str(&format!(
            "  {}::{}: {}\n",
            local.function, local.name, local.type_name
        ));
    }
    if graph.locals.len() > limit {
        output.push_str(&format!("  … +{} more\n", graph.locals.len() - limit));
    }
    Ok(output)
}

/// Merge a MIR artifact with this package's recorded trace.
/// 把一个 MIR artifact 与本包已记录的 trace 合并。
pub(crate) fn unified(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "nichlink.unified requires path (a MIR text dump or a JSONL artifact)".to_owned()
        })?;
    let (path, graph) = load(root, relative)?;
    let trace_path = trace_artifact_path(root);
    // An absent trace is not an error: it makes every relation a compiler candidate,
    // which is exactly what the labels below then say.
    // 缺失 trace 不是错误：它让每条关系都是编译器候选，而下面那些标签正是这么说的。
    let (trace, trace_note) = if trace_path.is_file() {
        let (_, trace) = read_trace_artifact(&trace_path)?;
        (trace, format!("trace {}", trace_path.display()))
    } else {
        (
            CallTrace::disabled(),
            format!(
                "trace none ({}) — every relation below is a compiler candidate",
                trace_path.display()
            ),
        )
    };
    let merged = nichlink_debug_method::UnifiedCallGraph::new(&graph, &trace);
    let relations = merged.relations();
    let live = relations
        .iter()
        .filter(|relation| relation.evidence == EvidenceKind::Live)
        .count();
    let limit = limit(arguments);
    let mut output = format!(
        "mir {}\n{trace_note}\nrelations {} (live {live}, compiler candidates {})\n",
        path.display(),
        relations.len(),
        relations.len() - live
    );
    for relation in relations.iter().take(limit) {
        output.push_str(&format!(
            "  {} -> {}  evidence={:?} source={}\n",
            relation.caller,
            relation.callee,
            relation.evidence,
            relation
                .source
                .as_ref()
                .map(|location| format!("{}:{}", location.file, location.line))
                .unwrap_or_else(|| "-".to_owned())
        ));
    }
    if relations.len() > limit {
        output.push_str(&format!("  … +{} more\n", relations.len() - limit));
    }
    Ok(output)
}

/// Read and parse one MIR artifact, choosing the parser by extension.
/// 读取并解析一个 MIR artifact，按扩展名选择解析器。
///
/// A JSONL artifact parses strictly — a malformed line fails the whole read — while a
/// `-Zunpretty=mir` text dump never fails: a line that is not a call is simply not a
/// call. That asymmetry belongs to the formats, and this keeps it rather than
/// flattening it.
/// JSONL artifact 严格解析——一行畸形就整体失败——而 `-Zunpretty=mir` 文本转储从不失败：不是调用的
/// 行就只是不是调用。这个不对称属于格式本身，这里保留它而不是抹平它。
///
/// Existence is answered before containment on purpose: the containment check
/// canonicalizes, so a missing path is not "outside the root" — it is missing, and
/// the reply says how to produce one.
/// 存在性有意先于归属检查：归属检查要规范化路径，因此缺失的路径不是"在根之外"，而是不存在，
/// 回复会说明如何产出一份。
fn load(root: &Path, relative: &str) -> Result<(PathBuf, MirGraph), String> {
    let path = root.join(relative);
    if !path.is_file() {
        return Err(format!(
            "{} is not a readable file; produce a text dump with `cargo rustc -Zunpretty=mir` on a \
             nightly toolchain, or pass the JSONL this tool emits",
            path.display()
        ));
    }
    if !crate::index::is_safe_child(root, &path) {
        return Err("path must stay inside the configured source root".to_owned());
    }
    let source = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let jsonl = path.extension().and_then(|extension| extension.to_str()) == Some("jsonl");
    let graph = if jsonl {
        MirGraph::from_jsonl(&source).map_err(|error| format!("{error:?}"))?
    } else {
        MirGraph::from_mir_text(&source)
    };
    Ok((path, graph))
}

/// The caller's row bound, clamped.
/// 调用方给的行数上限，已夹取。
fn limit(arguments: &Value) -> usize {
    arguments
        .get("limit")
        .and_then(Value::as_u64)
        .map_or(DEFAULT_LIMIT, |value| value.clamp(1, 200) as usize)
}

/// Keep one reply inside a size an agent can read, and say when it did not.
/// 把一条回复限制在代理读得下的规模里，并在截断时说出来。
fn bounded(text: &str, unit: &str, hint: &str) -> String {
    let lines = text.lines().count();
    if lines <= MAX_ROWS {
        return text.to_owned();
    }
    let head = text.lines().take(MAX_ROWS).collect::<Vec<_>>().join("\n");
    format!("{head}\n… truncated: {lines} {unit} total, {MAX_ROWS} shown. {hint}.\n")
}

#[cfg(test)]
#[path = "mir_tests.rs"]
mod mir_tests;
