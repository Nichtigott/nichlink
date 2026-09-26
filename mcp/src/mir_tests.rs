//! Tests for the MIR channel and the merge: the JSONL this tool emits reads back,
//! and a live call confirms its compiler candidate while the rest stay candidates.
//! MIR 通道与合并的测试：本工具输出的 JSONL 能读回来，而真实调用确认它的编译器候选、其余保持候选。

use std::path::{Path, PathBuf};

use nichlink_run_method::{CallTrace, trace_artifact_path, write_trace_artifact};
use serde_json::json;

use super::{mir, unified};

/// The MIR text `rustc -Zunpretty=mir` prints for a two-call function: one call the
/// trace below confirms, one it never makes.
/// `rustc -Zunpretty=mir` 为一个含两次调用的函数打印的 MIR 文本：一次被下面的 trace 确认，一次
/// 从未发生。
const MIR_TEXT: &str = "fn crate::outer(_1: f32) -> f32 {\n    let mut _2: f32;\n    _2 = crate::inner(move _1);\n    _2 = crate::other(move _1);\n    return;\n}\nfn crate::inner(_1: f32) -> f32 { return; }\nfn crate::other(_1: f32) -> f32 { return; }\n";

/// A throwaway root holding one MIR text dump.
/// 一个装有一份 MIR 文本转储的一次性根目录。
fn root(label: &str) -> (PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("mcp-mir-{label}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("fixture directory");
    let path = directory.join("dump.mir");
    std::fs::write(&path, MIR_TEXT).expect("the MIR dump");
    (directory, path)
}

/// Record a trace whose only call edge is `crate::outer -> crate::inner`.
/// 记录一份唯一调用边是 `crate::outer -> crate::inner` 的 trace。
fn record_trace(directory: &Path) {
    let mut trace = CallTrace::full();
    let outer =
        nichlink::identity::NodeId::from_namespaced_path("mcp-mir", "outer/outer.rs", "Outer");
    let inner =
        nichlink::identity::NodeId::from_namespaced_path("mcp-mir", "inner/inner.rs", "Inner");
    trace.with(outer, "crate::outer", |trace| {
        trace.with(inner, "crate::inner", |_| {});
    });
    write_trace_artifact(&trace, &trace_artifact_path(directory), "mcp-mir")
        .expect("the trace artifact writes");
}

/// The graph reports what the dump declared, and the JSONL this tool emits is
/// readable by the same tool — which is the writer that never existed.
/// 图报出那份转储声明了什么，而本工具输出的 JSONL 能被同一个工具读回来——那正是从未存在过的写入方。
#[test]
fn the_jsonl_channel_round_trips_through_this_tool() {
    let (directory, _) = root("round-trip");
    let report = mir(&directory, &json!({"path": "dump.mir"})).expect("the graph renders");
    assert!(report.contains("crate::outer -> crate::inner"), "{report}");
    assert!(report.contains("crate::outer -> crate::other"), "{report}");
    assert!(report.contains(": f32"), "locals are reported: {report}");
    let jsonl =
        mir(&directory, &json!({"path": "dump.mir", "jsonl": true})).expect("the JSONL renders");
    assert!(jsonl.contains("\"kind\":\"call\""), "{jsonl}");
    std::fs::write(directory.join("dump.jsonl"), &jsonl).expect("the emitted artifact");
    let reread = mir(&directory, &json!({"path": "dump.jsonl"})).expect("the JSONL reads back");
    assert!(reread.contains("crate::outer -> crate::inner"), "{reread}");
    assert!(reread.contains("crate::outer -> crate::other"), "{reread}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// The merge itself: the call the trace confirms is `Live`, the call it never made
/// stays a compiler candidate — and the count line says both.
/// 合并本身：trace 确认的那次调用是 `Live`，它从未发生的那次仍是编译器候选——而计数行把两者都说出来。
#[test]
fn the_merge_confirms_a_live_call_and_keeps_the_rest_as_candidates() {
    let (directory, _) = root("merged");
    record_trace(&directory);
    let report = unified(&directory, &json!({"path": "dump.mir"})).expect("the merge renders");
    assert!(
        report.contains("relations 2 (live 1, compiler candidates 1)"),
        "{report}"
    );
    assert!(
        report.contains("crate::outer -> crate::inner  evidence=Live"),
        "{report}"
    );
    assert!(
        report.contains("crate::outer -> crate::other  evidence=Mir"),
        "{report}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// Without a trace the merge still answers, and it says what that means: every
/// relation is a compiler candidate.
/// 没有 trace 时合并仍然作答，并说明这意味着什么：每条关系都是编译器候选。
#[test]
fn a_missing_trace_degrades_to_candidates_rather_than_failing() {
    let (directory, _) = root("no-trace");
    let report = unified(&directory, &json!({"path": "dump.mir"})).expect("the merge renders");
    assert!(report.contains("trace none"), "{report}");
    assert!(report.contains("compiler candidates 2"), "{report}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// A missing artifact and a path that escapes the root are both refused with the way
/// out, because "produce a dump with nightly" is the answer an agent needs.
/// 缺失的 artifact 与逃出根目录的路径都会被拒绝并给出出路，因为"用 nightly 产出一份转储"正是代理
/// 需要的答案。
#[test]
fn a_missing_artifact_and_an_escaping_path_are_refused() {
    let (directory, _) = root("refused");
    let missing = mir(&directory, &json!({"path": "nope.mir"})).expect_err("missing is refused");
    assert!(missing.contains("Zunpretty=mir"), "{missing}");
    let escaping = mir(&directory, &json!({"path": "../../etc/passwd"}))
        .expect_err("an escaping path is refused");
    assert!(escaping.contains("must stay inside"), "{escaping}");
    let _ = std::fs::remove_dir_all(&directory);
}
