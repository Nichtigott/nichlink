//! The compiler's call candidates, the merge with what actually ran, and the
//! delta between two snapshots.
//! 编译器给出的调用候选、它与"真正跑了什么"的合并，以及两份快照之间的差异。
//!
//! MIR JSONL was a format with a parser, a renderer, and no writer anywhere in the
//! workspace; a `-Zunpretty=mir` text dump was readable only inside Studio and only
//! on a nightly toolchain. These two tools close both halves without re-implementing
//! the parse: `nichlink.mir` reads either form and emits the canonical JSONL, and
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
//!
//! What makes a written artifact a **snapshot** is the header this tool stamps into
//! it: the identity namespace and registry root of the package it came from. A MIR
//! dump is a snapshot of *some* tree and rustc's text format cannot say which, so a
//! delta across two dumps is only meaningful when both name their tree — a foreign
//! snapshot is refused by name, and an unidentified one makes the delta say what it
//! cannot rule out. That is the same convention the trace artifact carries.
//! 让写出的 artifact 成为**快照**的是本工具盖进去的表头：它来源包的身份命名空间与注册树根。MIR
//! 转储是**某棵**树的快照，而 rustc 的文本格式说不出是哪一棵，因此两份转储之间的 delta 只有在两者
//! 都点名了自己的树时才有意义——外来快照会被按名字拒绝，未标识的则让 delta 说出它无法排除什么。
//! 这与 trace artifact 携带的是同一种约定。

use std::path::{Path, PathBuf};

use nichlink::EvidenceKind;
use nichlink::mir::{MirGraph, MirSnapshot};
use nichlink_run_method::{CallTrace, read_trace_artifact, trace_artifact_path};
use serde_json::Value;

use crate::protocol::DEFAULT_LIMIT;

/// The most rows or lines one reply carries before it says it truncated.
/// 一条回复在声明被截断之前最多携带的行数。
const MAX_ROWS: usize = 400;

/// Read a MIR artifact and report the graph, emit it as canonical JSONL, or
/// report the call-graph delta against another artifact.
/// 读取一个 MIR artifact 并报告该图、把它输出为规范 JSONL，或报告它与另一个 artifact 的调用图差异。
pub(crate) fn mir(root: &Path, arguments: &Value) -> Result<String, String> {
    let relative = arguments
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            "nichlink.mir requires path (a MIR text dump or a JSONL artifact)".to_owned()
        })?;
    let (path, mut graph) = load(root, relative)?;
    if let Some(against) = arguments.get("against").and_then(Value::as_str) {
        if arguments.get("jsonl").and_then(Value::as_bool) == Some(true) {
            return Err(
                "`jsonl` emits one snapshot and `against` compares two artifacts; ask for one of \
                 them"
                    .to_owned(),
            );
        }
        let (baseline_path, baseline) =
            load(root, against).map_err(|error| format!("baseline {error}"))?;
        return Ok(delta_report(
            &baseline_path,
            &baseline,
            &path,
            &graph,
            limit(arguments),
        ));
    }
    if arguments.get("jsonl").and_then(Value::as_bool) == Some(true) {
        // The portable channel's missing writer: Studio could render this and parse
        // it, and nothing ever produced one. What it writes is a snapshot, so the
        // artifact names the tree it came from — and it refuses to relabel another
        // tree's artifact rather than mislabeling it as this package's.
        // 可移植通道缺失的写入方：Studio 能渲染它、也能解析它，而从没有任何东西产出过一份。它写出的是
        // 一份快照，因此 artifact 点名了自己的来源树——而它拒绝给另一棵树的 artifact 换标签，而不是把它
        // 错标成本包的。
        confirmed_snapshot(&mut graph, &snapshot_for(root)?, &path)?;
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
    if let Some(snapshot) = &graph.snapshot {
        output.push_str(&identity_line(snapshot));
        output.push('\n');
    }
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
    // The trace belongs to this package's tree, so a MIR snapshot naming a
    // *different* tree must not be merged with it: the merge would look like a
    // confirmation of something the compiler never saw. An unidentified artifact
    // is still mergeable, because nothing proves it foreign.
    // trace 属于本包的树，因此点名了**另一棵**树的 MIR 快照不得与它合并：那次合并看上去会像是确认了
    // 编译器从未见过的什么东西。未标识的 artifact 仍可合并，因为没有东西证明它是外来的。
    if let Some(existing) = &graph.snapshot {
        let snapshot = snapshot_for(root)?;
        if existing.namespace != snapshot.namespace || existing.root != snapshot.root {
            return Ok(format!(
                "REFUSED: the MIR artifact describes a different tree than the trace\n  {} names \
                 namespace `{}` root {}\n  this package is `{}` root {}\nA merge across two trees \
                 would confirm relations that never existed together.\n",
                path.display(),
                existing.namespace,
                existing.root,
                snapshot.namespace,
                snapshot.root
            ));
        }
    }
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

/// The snapshot header this package's artifacts must carry.
/// 本包 artifact 必须携带的快照表头。
fn snapshot_for(root: &Path) -> Result<MirSnapshot, String> {
    let namespace = crate::registry::namespace(root)?;
    Ok(MirSnapshot {
        root: nichlink::root_node_id(&namespace),
        namespace,
    })
}

/// Stamp this package's snapshot onto an artifact, refusing to relabel another
/// tree's.
/// 把本包的快照盖到 artifact 上，拒绝给另一棵树的 artifact 换标签。
///
/// Re-stamping would turn a foreign artifact into one that claims to be this
/// package's, which is the one thing the header exists to prevent; an artifact
/// that already names this tree is left with its own (identical) header.
/// 重新盖戳会把一份外来 artifact 变成自称属于本包的 artifact，而这正是表头存在的意义所在；
/// 已经点名本树的 artifact 保留它自己的（相同的）表头。
fn confirmed_snapshot(
    graph: &mut MirGraph,
    snapshot: &MirSnapshot,
    path: &Path,
) -> Result<(), String> {
    if let Some(existing) = &graph.snapshot
        && (existing.namespace != snapshot.namespace || existing.root != snapshot.root)
    {
        return Err(format!(
            "{} already names the tree of `{}` (root {}), and this package is `{}` (root {}); \
             re-emitting it here would mislabel it — run this tool in the package it came from, or \
             pass the artifact unchanged",
            path.display(),
            existing.namespace,
            existing.root,
            snapshot.namespace,
            snapshot.root
        ));
    }
    graph.snapshot = Some(snapshot.clone());
    Ok(())
}

/// Report the call-graph delta from a baseline artifact to another one.
/// 报告从基线 artifact 到另一个 artifact 的调用图差异。
///
/// The direction is the claim: `added` is what the second artifact has and the
/// baseline does not, so a reader can hand in "before" and "after" in either
/// order and still know which way the counts point. A mismatch the snapshots
/// prove is refused rather than rendered, and an artifact that does not name its
/// tree makes the delta say what it cannot rule out — a `-Zunpretty=mir` dump
/// cannot name its tree, and that is the ordinary case, not a broken one.
/// 方向就是主张：`added` 是第二份 artifact 有而基线没有的东西，因此读取方无论按什么顺序交进"之前"
/// 与"之后"，都知道计数指向哪边。快照证实的身份不符会被拒绝而不是被渲染，而没有点名自己那棵树的
/// artifact 会让 delta 说出它无法排除什么——`-Zunpretty=mir` 转储说不出自己的树，而那是常态，不是坏掉。
fn delta_report(
    baseline_path: &Path,
    baseline: &MirGraph,
    path: &Path,
    after: &MirGraph,
    limit: usize,
) -> String {
    let mut output = format!(
        "mir {}\nbaseline {}\n",
        path.display(),
        baseline_path.display()
    );
    output.push_str(&format!("after  {}\n", described_tree(after)));
    output.push_str(&format!("before {}\n", described_tree(baseline)));
    if let (Some(now), Some(then)) = (&after.snapshot, &baseline.snapshot)
        && (now.namespace != then.namespace || now.root != then.root)
    {
        return format!(
            "{output}REFUSED: the two artifacts describe different trees, so a call-graph delta \
             between them would be a plausible, wrong answer\n  {} is namespace `{}` root {}\n  {} \
             is namespace `{}` root {}\nDiff two snapshots of one tree, or fix the namespace with \
             NICH_LINK_NAMESPACE.\n",
            path.display(),
            now.namespace,
            now.root,
            baseline_path.display(),
            then.namespace,
            then.root
        );
    }
    if after.snapshot.is_none() || baseline.snapshot.is_none() {
        output.push_str(
            "one or both artifacts do not name the tree they came from, so a comparison across two \
             trees cannot be ruled out\n",
        );
    }
    let delta = baseline.delta(after);
    output.push_str(&format!(
        "delta: relations added {} gone {}  functions added {} gone {}\n",
        delta.added.len(),
        delta.gone.len(),
        delta.functions_added.len(),
        delta.functions_gone.len()
    ));
    output.push_str("added:\n");
    for call in delta.added.iter().take(limit) {
        output.push_str(&format!("  {} -> {}\n", call.caller, call.callee));
    }
    if delta.added.len() > limit {
        output.push_str(&format!("  … +{} more\n", delta.added.len() - limit));
    }
    output.push_str("gone:\n");
    for call in delta.gone.iter().take(limit) {
        output.push_str(&format!("  {} -> {}\n", call.caller, call.callee));
    }
    if delta.gone.len() > limit {
        output.push_str(&format!("  … +{} more\n", delta.gone.len() - limit));
    }
    output.push_str("functions added:\n");
    for name in delta.functions_added.iter().take(limit) {
        output.push_str(&format!("  + {name}\n"));
    }
    if delta.functions_added.len() > limit {
        output.push_str(&format!(
            "  … +{} more\n",
            delta.functions_added.len() - limit
        ));
    }
    output.push_str("functions gone:\n");
    for name in delta.functions_gone.iter().take(limit) {
        output.push_str(&format!("  - {name}\n"));
    }
    if delta.functions_gone.len() > limit {
        output.push_str(&format!(
            "  … +{} more\n",
            delta.functions_gone.len() - limit
        ));
    }
    output
}

/// One artifact's tree, as the reply says it: named when its snapshot names it,
/// and unidentified otherwise.
/// 一份 artifact 的树，按回复的说法：快照点名了它时就点名，否则就是未标识。
fn described_tree(graph: &MirGraph) -> String {
    match &graph.snapshot {
        Some(snapshot) => identity_line(snapshot),
        None => "unidentified (no snapshot record names the tree)".to_owned(),
    }
}

/// The identity of one snapshot as one line.
/// 一份快照的身份，一行。
fn identity_line(snapshot: &MirSnapshot) -> String {
    format!(
        "snapshot namespace={} root={}",
        snapshot.namespace, snapshot.root
    )
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
