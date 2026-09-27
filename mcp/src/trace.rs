//! The runtime evidence a host recorded, read back and checked against this tree.
//! 宿主记录下的运行期证据，读回来并与这棵树对照校验。
//!
//! A trace says what actually ran, which no static read can: `nichlink.callgraph`
//! is a lexical candidate list and says so. The artifact format, its reader, and
//! the headless call report all already existed — the four renderers had no
//! caller anywhere in the workspace — so this module is the outlet, not a second
//! implementation. It keeps Studio's rule: a report is refused when the artifact
//! describes a different tree, because frames are node identities and identities
//! from another tree render a plausible, wrong call tree.
//! trace 说明真正跑了什么，这是任何静态读取都做不到的：`nichlink.callgraph` 是词法候选清单，它
//! 自己也这么声明。artifact 格式、它的读取器、无终端调用报告本来都存在——那四个渲染器在整个工作区
//! 里没有一个调用者——因此本模块是出口而不是第二份实现。它保留 Studio 的规矩：当 artifact 描述的
//! 是另一棵树时拒绝出报告，因为帧是节点身份，而另一棵树的身份会渲染出一棵看似合理但错误的调用树。

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use nichlink_build_method::face_views;
use nichlink_run_method::runtime::trace::locals::LocalValue;
use nichlink_run_method::{
    CallTrace, TraceArtifact, read_trace_artifact, render_call_report_for_trace,
    trace_artifact_path,
};
use serde_json::Value;

use crate::apply::load_registry;
use crate::registry::namespace;

/// The most report lines one reply carries before it says it truncated.
/// 一条回复在声明被截断之前最多携带的报告行数。
const MAX_REPORT_LINES: usize = 400;

/// What reading this package's recorded trace produced.
/// 读取本包已记录的 trace 得到了什么。
///
/// Absence and refusal are *answers*, not errors: "no host recorded one" and "this
/// artifact describes another tree" are both things an agent can act on, and a
/// second tool needing the same trace (the trace-driven `nichlink.converge`) must
/// inherit both verbatim rather than inventing its own wording.
/// 缺失与拒绝都是**答案**而不是错误：「没有宿主记录过」与「这份 artifact 描述的是另一棵树」都是代理
/// 能据以行动的东西；而需要同一份 trace 的第二个工具（由 trace 驱动的 `nichlink.converge`）必须原样
/// 继承两者，而不是另造一套说法。
pub(crate) enum RecordedTrace {
    /// The ready-to-print answer for "no artifact here", with the way to produce one.
    /// 「这里没有 artifact」的现成答案，并带上产出它的办法。
    Absent(String),
    /// The header plus the refusal, when the artifact describes a different tree.
    /// 当 artifact 描述另一棵树时：表头加上拒绝理由。
    Refused(String),
    /// The artifact, its call trace, and the header that describes it.
    /// artifact、它的调用 trace，以及描述它的表头。
    Verified {
        path: PathBuf,
        artifact: TraceArtifact,
        call_trace: Box<CallTrace>,
        header: String,
    },
}

/// Read this package's trace artifact and check that it describes this tree.
/// 读取本包的 trace artifact，并核验它描述的是这棵树。
pub(crate) fn read_verified(root: &Path) -> Result<RecordedTrace, String> {
    let path = trace_artifact_path(root);
    if !path.is_file() {
        // Absence is the common case today, so the answer carries the way to
        // produce one instead of a bare "not found".
        // 缺失是今天的常态，因此答案要带上产出它的办法，而不是一句光秃秃的 "not found"。
        return Ok(RecordedTrace::Absent(format!(
            "trace absent: {}\nA host writes one by recording with the `trace_call!` family and running \
             with `NICH_LINK_TRACE` (the mode) or `NICH_LINK_TRACE_FILE` (the path) set; a project \
             scaffolded by `nichlink new` demonstrates that whole chain in its `src/main.rs`. \
             `nichlink check` reports the static side only.\n",
            path.display()
        )));
    }
    let (artifact, call_trace) = read_trace_artifact(&path)?;
    let namespace = namespace(root)?;
    let expected_root = nichlink::root_node_id(&namespace);
    let header = format!(
        "artifact {}\nnamespace {} (recorded {})\nroot {} (recorded {})\nmode {:?}\nframes {} locals {} edges {}\n",
        path.display(),
        namespace,
        artifact.namespace,
        expected_root,
        artifact.root,
        artifact.mode,
        artifact.frames.len(),
        artifact.locals.len(),
        artifact.edges.len(),
    );
    let mut mismatch = Vec::new();
    if artifact.namespace != namespace {
        mismatch.push(format!(
            "namespace: the artifact was recorded under `{}`, this package is `{namespace}`",
            artifact.namespace
        ));
    }
    if artifact.root != expected_root {
        mismatch.push(format!(
            "root: the artifact was minted under `{}`, this package's root is `{expected_root}`",
            artifact.root
        ));
    }
    // The root is a node the tree has even though no face declares it, so a host
    // that records a root frame is not reporting a foreign tree.
    // 根是没有面声明、但树确实拥有的节点，因此记录了根帧的宿主并不是在报告一棵异树。
    let mut known: HashSet<_> = face_views(root, &namespace)?
        .into_iter()
        .map(|face| face.id)
        .collect();
    known.insert(expected_root);
    let unresolved: Vec<_> = artifact
        .frames
        .iter()
        .filter(|frame| !known.contains(&frame.node))
        .collect();
    if let Some(first) = unresolved.first() {
        mismatch.push(format!(
            "{} of {} frame(s) name nodes this tree does not have, e.g. {} in `{}`",
            unresolved.len(),
            artifact.frames.len(),
            first.node,
            first.function
        ));
    }
    if !mismatch.is_empty() {
        return Ok(RecordedTrace::Refused(format!(
            "{header}REFUSED: the artifact does not describe this tree\n  {}\nRerun the host after \
             `nichlink check` so the recorded identities match the compiled ones.\n",
            mismatch.join("\n  ")
        )));
    }
    Ok(RecordedTrace::Verified {
        path,
        artifact,
        call_trace: Box::new(call_trace),
        header,
    })
}

/// Read this package's trace artifact and answer with the call report it implies.
/// 读取本包的 trace artifact，并用它蕴含的调用报告作答。
pub(crate) fn trace(root: &Path, arguments: &Value) -> Result<String, String> {
    match read_verified(root)? {
        RecordedTrace::Absent(answer) | RecordedTrace::Refused(answer) => Ok(answer),
        RecordedTrace::Verified {
            call_trace,
            header,
            artifact,
            ..
        } => {
            let query = arguments.get("query").and_then(Value::as_str);
            if arguments.get("values").and_then(Value::as_bool) == Some(true) {
                return Ok(bounded(header, &render_values(&artifact, query)));
            }
            let namespace = namespace(root)?;
            let registry = load_registry(root, &namespace)?;
            let report = render_call_report_for_trace(&registry, &call_trace, query);
            Ok(bounded(header, &report))
        }
    }
}

/// Render the values the run recorded, grouped by the frame they were recorded in,
/// plus the observed data edges between them.
/// 渲染这次运行记录下的值——按记录它们时所在的帧分组——以及它们之间被观察到的数据边。
///
/// The call report says what ran; this says what it saw. A local belongs to a frame
/// by `frame_id`, and one recorded outside every traced call is reported as such
/// rather than folded into a frame it never had; a local or edge naming something
/// this artifact does not contain is *counted* instead of being silently dropped, so
/// an incomplete artifact cannot read as a complete one.
/// 调用报告说跑了什么，这里说它看见了什么。局部值靠 `frame_id` 属于某个帧，而在任何被追踪调用之外记录
/// 的那个会被如实报成如此，而不是被塞进它从未属于的帧；点名的东西不在本 artifact 里的局部值或边会被
/// **计数**而不是悄悄丢掉，因此一份不完整的 artifact 不会读起来像完整的。
fn render_values(artifact: &TraceArtifact, query: Option<&str>) -> String {
    let keeps = |name: &str, type_name: &str, value: &str| match query {
        Some(query) => name.contains(query) || type_name.contains(query) || value.contains(query),
        None => true,
    };
    let line = |local: &LocalValue| {
        format!(
            "  {}: {} = {}  [{}, {}]  @ {}:{}\n",
            local.name,
            local.type_name,
            local.value,
            local.kind.label(),
            local.observation.label(),
            local.source.file,
            local.source.line
        )
    };
    let mut lines = 0usize;
    let mut shown = 0usize;
    let mut output = format!(
        "values {} locals {} edges\n",
        artifact.locals.len(),
        artifact.edges.len()
    );
    for frame in &artifact.frames {
        if lines >= MAX_REPORT_LINES {
            break;
        }
        let inside: Vec<_> = artifact
            .locals
            .iter()
            .filter(|local| local.frame_id == Some(frame.frame_id))
            .filter(|local| keeps(&local.name, &local.type_name, &local.value))
            .collect();
        if inside.is_empty() {
            continue;
        }
        let at = frame
            .source
            .as_ref()
            .map(|source| format!("{}:{}", source.file, source.line))
            .unwrap_or_else(|| "-".to_owned());
        output.push_str(&format!(
            "frame {} {}  ({at})  {} local(s)\n",
            frame.frame_id,
            frame.function,
            inside.len()
        ));
        lines += 1;
        for local in inside {
            output.push_str(&line(local));
            lines += 1;
            shown += 1;
        }
    }
    let outside: Vec<_> = artifact
        .locals
        .iter()
        .filter(|local| local.frame_id.is_none())
        .filter(|local| keeps(&local.name, &local.type_name, &local.value))
        .collect();
    if !outside.is_empty() && lines < MAX_REPORT_LINES {
        output.push_str("outside any traced frame\n");
        lines += 1;
        for local in &outside {
            output.push_str(&line(local));
            lines += 1;
            shown += 1;
        }
    }
    let dangling = artifact
        .locals
        .iter()
        .filter(|local| {
            local
                .frame_id
                .is_some_and(|id| !artifact.frames.iter().any(|frame| frame.frame_id == id))
        })
        .count();
    if dangling > 0 {
        output.push_str(&format!(
            "locals naming a frame this artifact does not have {dangling}\n"
        ));
    }
    let name_of = |id: u64| {
        artifact
            .locals
            .iter()
            .find(|local| local.id == id)
            .map(|local| local.name.as_str())
            .unwrap_or("<not recorded>")
    };
    let edges: Vec<_> = artifact
        .edges
        .iter()
        .filter(|edge| match query {
            Some(query) => {
                name_of(edge.from).contains(query)
                    || name_of(edge.to).contains(query)
                    || edge.label.contains(query)
            }
            None => true,
        })
        .collect();
    output.push_str(&format!("data edges {}\n", artifact.edges.len()));
    lines += 1;
    for edge in &edges {
        if lines >= MAX_REPORT_LINES {
            break;
        }
        let at = edge
            .source
            .as_ref()
            .map(|source| format!("{}:{}", source.file, source.line))
            .unwrap_or_else(|| "-".to_owned());
        output.push_str(&format!(
            "  {} -> {}  ({})  @ {at}\n",
            name_of(edge.from),
            name_of(edge.to),
            edge.label
        ));
        lines += 1;
    }
    let unrecorded = artifact
        .edges
        .iter()
        .filter(|edge| {
            !artifact.locals.iter().any(|local| local.id == edge.from)
                || !artifact.locals.iter().any(|local| local.id == edge.to)
        })
        .count();
    if unrecorded > 0 {
        output.push_str(&format!(
            "edges naming a local this artifact does not have {unrecorded}\n"
        ));
    }
    if shown == 0 && query.is_some() {
        output.push_str("no recorded value matches the query\n");
    }
    output
}

/// Keep one reply inside a size an agent can read, and say when it did not.
/// 把一条回复限制在代理读得下的规模里，并在截断时说出来。
fn bounded(header: String, report: &str) -> String {
    let lines = report.lines().count();
    if lines <= MAX_REPORT_LINES {
        return format!("{header}{report}");
    }
    let head = report
        .lines()
        .take(MAX_REPORT_LINES)
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{header}{head}\n… truncated: {lines} lines total, {MAX_REPORT_LINES} shown. Narrow with `query`.\n"
    )
}

#[cfg(test)]
#[path = "trace_tests.rs"]
mod trace_tests;
