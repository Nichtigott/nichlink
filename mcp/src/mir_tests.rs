//! Tests for the MIR channel, the merge, and the delta: the JSONL this tool
//! emits reads back as a snapshot of a named tree, a live call confirms its
//! compiler candidate while the rest stay candidates, and two snapshots diff
//! only when their trees allow it.
//! MIR 通道、合并与差异的测试：本工具输出的 JSONL 能读回来、且是一棵被点名树的快照；真实调用确认
//! 它的编译器候选、其余保持候选；两份快照只有在各自的树允许时才作差。

use std::path::{Path, PathBuf};

use nichlink_run_method::{CallTrace, trace_artifact_path, write_trace_artifact};
use serde_json::json;

use super::{mir, unified};

/// The MIR text `rustc -Zunpretty=mir` prints for a two-call function: one call the
/// trace below confirms, one it never makes.
/// `rustc -Zunpretty=mir` 为一个含两次调用的函数打印的 MIR 文本：一次被下面的 trace 确认，一次
/// 从未发生。
const MIR_TEXT: &str = "fn crate::outer(_1: f32) -> f32 {\n    let mut _2: f32;\n    _2 = crate::inner(move _1);\n    _2 = crate::other(move _1);\n    return;\n}\nfn crate::inner(_1: f32) -> f32 { return; }\nfn crate::other(_1: f32) -> f32 { return; }\n";

/// The same function after an edit: one call survived, one was dropped, one was
/// added.
/// 同一次编辑之后的同一个函数：一次调用活了下来、一次被去掉、一次被新增。
const AFTER_TEXT: &str = "fn crate::outer(_1: f32) -> f32 {\n    let mut _2: f32;\n    _2 = crate::inner(move _1);\n    _2 = crate::new(move _1);\n    return;\n}\nfn crate::inner(_1: f32) -> f32 { return; }\nfn crate::new(_1: f32) -> f32 { return; }\n";

/// A throwaway package holding one MIR text dump.
/// 一个装有一份 MIR 文本转储的一次性包。
///
/// The manifest is load-bearing since the snapshot convention: the JSONL this
/// tool writes names the tree it came from, so a root Cargo can name is what
/// makes the writer able to identify its output at all.
/// 自快照约定起，清单是承重的：本工具写出的 JSONL 点名了它的来源树，因此一个 Cargo 能命名的根
/// 正是写入方得以标识其产物的前提。
fn root(label: &str) -> (PathBuf, PathBuf) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory =
        std::env::temp_dir().join(format!("mcp-mir-{label}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).expect("source directory");
    std::fs::write(
        directory.join("Cargo.toml"),
        "[package]\nname = \"mcp-mir\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("manifest");
    std::fs::write(directory.join("src/lib.rs"), "// host entry\n").expect("library target");
    let path = directory.join("dump.mir");
    std::fs::write(&path, MIR_TEXT).expect("the MIR dump");
    (directory, path)
}

/// Emit one artifact as JSONL through the tool, which is the writer under test.
/// 经本工具把一份 artifact 输出成 JSONL，那正是被测的写入方。
fn emit(directory: &Path, source: &str, target: &str) -> String {
    let jsonl = mir(directory, &json!({"path": source, "jsonl": true})).expect("the JSONL emits");
    std::fs::write(directory.join(target), &jsonl).expect("the emitted artifact");
    jsonl
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
/// readable by the same tool — which is the writer that never existed. What it
/// writes is a *snapshot*: the header names the tree the artifact came from.
/// 图报出那份转储声明了什么，而本工具输出的 JSONL 能被同一个工具读回来——那正是从未存在过的写入方。
/// 它写出的是**快照**：表头点名了 artifact 的来源树。
#[test]
fn the_jsonl_channel_round_trips_through_this_tool() {
    let (directory, _) = root("round-trip");
    let report = mir(&directory, &json!({"path": "dump.mir"})).expect("the graph renders");
    assert!(report.contains("crate::outer -> crate::inner"), "{report}");
    assert!(report.contains("crate::outer -> crate::other"), "{report}");
    assert!(report.contains(": f32"), "locals are reported: {report}");
    let jsonl = emit(&directory, "dump.mir", "dump.jsonl");
    assert!(jsonl.contains("\"kind\":\"call\""), "{jsonl}");
    assert!(
        jsonl.starts_with("{\"kind\":\"snapshot\",\"namespace\":\"mcp-mir\",\"root\":\""),
        "the writer's output names its tree first: {jsonl}"
    );
    let reread = mir(&directory, &json!({"path": "dump.jsonl"})).expect("the JSONL reads back");
    assert!(reread.contains("crate::outer -> crate::inner"), "{reread}");
    assert!(reread.contains("crate::outer -> crate::other"), "{reread}");
    assert!(
        reread.contains("snapshot namespace=mcp-mir"),
        "the reader reports the identity the writer stamped: {reread}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// Two snapshots of one tree diff into the relations and functions that moved,
/// and the counts point from the baseline forward.
/// 同一棵树的两份快照作差，得到移动过的关系与函数，而计数从基线指向后一份。
#[test]
fn a_delta_names_what_changed_between_two_snapshots() {
    let (directory, _) = root("delta");
    emit(&directory, "dump.mir", "before.jsonl");
    std::fs::write(directory.join("after.mir"), AFTER_TEXT).expect("the edited dump");
    emit(&directory, "after.mir", "after.jsonl");

    let report = mir(
        &directory,
        &json!({"path": "after.jsonl", "against": "before.jsonl"}),
    )
    .expect("the delta renders");
    assert!(
        report.contains("delta: relations added 1 gone 1  functions added 1 gone 1"),
        "{report}"
    );
    assert!(
        report.contains("added:\n  crate::outer -> crate::new"),
        "{report}"
    );
    assert!(
        report.contains("gone:\n  crate::outer -> crate::other"),
        "{report}"
    );
    assert!(
        report.contains("functions added:\n  + crate::new"),
        "{report}"
    );
    assert!(
        report.contains("functions gone:\n  - crate::other"),
        "{report}"
    );
    assert!(report.contains("snapshot namespace=mcp-mir"), "{report}");
    assert!(
        !report.contains("cannot be ruled out"),
        "both artifacts name their tree, so the comparison is sound: {report}"
    );

    // The direction is the claim: swapping the two swaps added and gone.
    let reversed = mir(
        &directory,
        &json!({"path": "before.jsonl", "against": "after.jsonl"}),
    )
    .expect("the reverse delta renders");
    assert!(
        reversed.contains("added:\n  crate::outer -> crate::other"),
        "{reversed}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// A `-Zunpretty=mir` text dump cannot name its tree, and the delta says so
/// instead of presenting a cross-tree comparison as sound — the ordinary case,
/// not a broken one.
/// `-Zunpretty=mir` 文本转储说不出自己的树，delta 会如实说明，而不是把跨树比较说成可靠的——这是
/// 常态，不是坏掉。
#[test]
fn an_unidentified_artifact_makes_the_delta_say_what_it_cannot_rule_out() {
    let (directory, _) = root("delta-unidentified");
    std::fs::write(directory.join("after.mir"), AFTER_TEXT).expect("the edited dump");
    let report = mir(
        &directory,
        &json!({"path": "after.mir", "against": "dump.mir"}),
    )
    .expect("the delta renders");
    assert!(
        report.contains("unidentified (no snapshot record names the tree)"),
        "{report}"
    );
    assert!(
        report.contains("a comparison across two trees cannot be ruled out"),
        "{report}"
    );
    assert!(
        report.contains("delta: relations added 1 gone 1"),
        "{report}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// A snapshot of another tree is refused by name: a call-graph delta across two
/// trees would be a plausible, wrong answer.
/// 另一棵树的快照会被按名字拒绝：跨两棵树的调用图差异会是一个看似合理、实则错误的答案。
#[test]
fn a_delta_across_two_named_trees_is_refused() {
    let (directory, _) = root("delta-refused");
    emit(&directory, "dump.mir", "before.jsonl");
    let foreign = foreign_snapshot();
    std::fs::write(directory.join("foreign.jsonl"), &foreign).expect("the foreign snapshot");

    let report = mir(
        &directory,
        &json!({"path": "foreign.jsonl", "against": "before.jsonl"}),
    )
    .expect("the refusal is an answer, not a tool failure");
    assert!(
        report.contains("REFUSED: the two artifacts describe different trees"),
        "{report}"
    );
    assert!(report.contains("somewhere-else"), "{report}");
    assert!(report.contains("mcp-mir"), "{report}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// Re-emitting another tree's artifact here would relabel a tree that is not
/// this package's, so the writer refuses instead of minting a false snapshot.
/// 在这里重新输出另一棵树的 artifact 会给一棵不属于本包的树换标签，因此写入方拒绝，而不是铸出
/// 一份假快照。
#[test]
fn re_emitting_another_trees_snapshot_is_refused() {
    let (directory, _) = root("relabel");
    std::fs::write(directory.join("foreign.jsonl"), foreign_snapshot())
        .expect("the foreign snapshot");
    let error = mir(&directory, &json!({"path": "foreign.jsonl", "jsonl": true}))
        .expect_err("relabelling is refused");
    assert!(error.contains("would mislabel it"), "{error}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// The merge joins this package's trace with this package's tree, so a MIR
/// snapshot naming another tree is refused rather than merged.
/// 合并把本包的 trace 与本包的树接起来，因此点名另一棵树的 MIR 快照会被拒绝，而不是被合并。
#[test]
fn a_merge_with_a_foreign_snapshot_is_refused() {
    let (directory, _) = root("merge-refused");
    record_trace(&directory);
    std::fs::write(directory.join("foreign.jsonl"), foreign_snapshot())
        .expect("the foreign snapshot");
    let report = unified(&directory, &json!({"path": "foreign.jsonl"}))
        .expect("the refusal is an answer, not a tool failure");
    assert!(
        report.contains("REFUSED: the MIR artifact describes a different tree"),
        "{report}"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

/// Emitting and comparing are two different requests; asking for both is refused
/// rather than silently answering one of them.
/// 输出与比较是两个不同的请求；同时要两者会被拒绝，而不是默默回答其中一个。
#[test]
fn asking_to_emit_and_compare_at_once_is_refused() {
    let (directory, _) = root("both");
    let error = mir(
        &directory,
        &json!({"path": "dump.mir", "jsonl": true, "against": "dump.mir"}),
    )
    .expect_err("one request at a time");
    assert!(error.contains("ask for one of them"), "{error}");
    let _ = std::fs::remove_dir_all(&directory);
}

/// A hand-written snapshot of a tree that is not this package's.
/// 一份手写的、属于另一棵树的快照。
fn foreign_snapshot() -> String {
    format!(
        "{{\"kind\":\"snapshot\",\"namespace\":\"somewhere-else\",\"root\":\"{}\"}}\n{{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"b\",\"mir_line\":1}}\n",
        nichlink::root_node_id("somewhere-else")
    )
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
///
/// The escaping case writes a real file *outside* the root, because existence is
/// answered before containment (canonicalization cannot judge a path that is not
/// there). Pointing at `../../etc/passwd` instead made this test platform-dependent:
/// under Linux's `/tmp` that path exists and the containment check refuses it, while
/// under macOS's `/var/folders/…` and Windows' temp directory nothing is there, so the
/// existence check refuses first with a different sentence. The security claim is
/// about an artifact that *is* there — an outside file must not be read — so the
/// fixture puts one there.
/// 逃逸那一条会在根**之外**写一个真实文件，因为存在性先于归属作答（规范化无法判断一个不存在的
/// 路径）。改用 `../../etc/passwd` 会让这条测试依赖平台：Linux 的 `/tmp` 下该路径存在、由归属检查
/// 拒绝，而 macOS 的 `/var/folders/…` 与 Windows 的临时目录下什么都不存在、由存在性检查先以另一句
/// 话拒绝。安全主张针对的是**确实存在**的 artifact——根之外的文件不得被读取——因此夹具自己放一个。
#[test]
fn a_missing_artifact_and_an_escaping_path_are_refused() {
    let (directory, _) = root("refused");
    let missing = mir(&directory, &json!({"path": "nope.mir"})).expect_err("missing is refused");
    assert!(missing.contains("Zunpretty=mir"), "{missing}");

    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .expect("a fixture directory name")
        .to_owned();
    let outside = directory.with_file_name(format!("{name}-outside"));
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&outside).expect("the outside directory");
    std::fs::write(outside.join("secret.mir"), MIR_TEXT).expect("the outside artifact");
    let escaping = mir(
        &directory,
        &json!({"path": format!("../{name}-outside/secret.mir")}),
    )
    .expect_err("an escaping path is refused");
    assert!(escaping.contains("must stay inside"), "{escaping}");

    let _ = std::fs::remove_dir_all(&outside);
    let _ = std::fs::remove_dir_all(&directory);
}
