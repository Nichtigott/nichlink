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
use std::path::Path;

use nichlink_build_method::face_views;
use nichlink_run_method::{read_trace_artifact, render_call_report_for_trace, trace_artifact_path};
use serde_json::Value;

use crate::apply::load_registry;
use crate::registry::namespace;

/// The most report lines one reply carries before it says it truncated.
/// 一条回复在声明被截断之前最多携带的报告行数。
const MAX_REPORT_LINES: usize = 400;

/// Read this package's trace artifact and answer with the call report it implies.
/// 读取本包的 trace artifact，并用它蕴含的调用报告作答。
pub(crate) fn trace(root: &Path, arguments: &Value) -> Result<String, String> {
    let path = trace_artifact_path(root);
    if !path.is_file() {
        // Absence is the common case today, so the answer carries the way to
        // produce one instead of a bare "not found".
        // 缺失是今天的常态，因此答案要带上产出它的办法，而不是一句光秃秃的 "not found"。
        return Ok(format!(
            "trace absent: {}\nA host writes one by recording with the `trace_call!` family and running \
             with `NICH_LINK_TRACE` (the mode) or `NICH_LINK_TRACE_FILE` (the path) set; a project \
             scaffolded by `nichlink new` demonstrates that whole chain in its `src/main.rs`. \
             `nichlink check` reports the static side only.\n",
            path.display()
        ));
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
        return Ok(format!(
            "{header}REFUSED: the artifact does not describe this tree\n  {}\nRerun the host after \
             `nichlink check` so the recorded identities match the compiled ones.\n",
            mismatch.join("\n  ")
        ));
    }
    let registry = load_registry(root, &namespace)?;
    let query = arguments.get("query").and_then(Value::as_str);
    let report = render_call_report_for_trace(&registry, &call_trace, query);
    Ok(bounded(header, &report))
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
