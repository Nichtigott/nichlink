//! Tests for the build-evidence report: it must name what the build decided, and
//! it must say "unknown" rather than guess when the build never ran.
//! 构建证据报告的测试：它必须说出构建决定了什么，而在构建从未跑过时必须说"未知"而不是猜。

use std::path::{Path, PathBuf};

use nichlink_build_method::face_views;
use serde_json::json;

use super::explain;

/// A throwaway package holding one hand-written root face, so `face_views`
/// derives exactly one node to report on.
/// 一个含一个手写根面的一次性包，因此 `face_views` 恰好推导出一个可报告节点。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-evidence-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/button")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(
        root.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face");
    (root, name)
}

/// Publish the two files the build writes, so the report has something to read.
/// 发布构建写下的那两个文件，让报告有东西可读。
fn publish(root: &Path, name: &str, all: bool) {
    let out = root.join("target/nichlink/out");
    std::fs::create_dir_all(&out).expect("build output");
    let faces = face_views(root, name).expect("faces derive");
    let id = faces[0].id;
    std::fs::write(
        out.join("source_scope.tsv"),
        if all {
            "# mode\tauto\n# result\tall\n# selected\tall\n".to_owned()
        } else {
            format!("# mode\tauto\n# result\tselected\n# selected\t-\n# id\t{id}\n")
        },
    )
    .expect("scope");
    std::fs::write(
        out.join("pruning_manifest.tsv"),
        format!(
            "# node\tsource\tsymbol\n{id}\t{}\t{}\n",
            faces[0].source, "button_symbol"
        ),
    )
    .expect("pruning");
}

/// The one question no other tool can answer: is this face in the shipped scope,
/// and does release pruning strip it. The answer comes from the build's own files.
/// 没有别的工具能回答的那个问题：这个面在发布作用域里吗，发布剪枝会不会剥掉它。答案来自构建
/// 自己的文件。
#[test]
fn the_report_answers_scope_and_pruning_from_the_builds_own_files() {
    let (root, name) = package("scope");
    publish(&root, &name, true);
    let reply = explain(&root, &json!({"node": "root/button"})).expect("the report renders");
    assert!(reply.contains("path root/button"), "{reply}");
    assert!(reply.contains("kind Button"), "{reply}");
    assert!(reply.contains("slot button"), "{reply}");
    assert!(reply.contains("scope selected (all=true"), "{reply}");
    assert!(reply.contains("pruning strips button_symbol"), "{reply}");
    // No `discovery.fingerprint` was published, so the report must admit the
    // build output is not known to be current.
    // 没有发布 `discovery.fingerprint`，因此报告必须承认无法认为构建产物是新鲜的。
    assert!(reply.contains("build stale"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A face the scope does not select is a pruning candidate, and the report says so
/// by name instead of reporting a generic "not in the tree".
/// 作用域没选中的面是剪枝候选，报告要指名道姓地说出来，而不是笼统地报"不在树里"。
#[test]
fn a_face_outside_the_scope_is_reported_as_not_selected() {
    let (root, name) = package("unselected");
    publish(&root, &name, false);
    let reply = explain(&root, &json!({"node": "root/button"})).expect("the report renders");
    assert!(reply.contains("scope not-selected"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Without a build there is no evidence, and the report says which command
/// produces it rather than inventing a verdict.
/// 没有构建就没有证据，报告要说出哪个命令会产出它，而不是编一个结论。
#[test]
fn a_missing_build_is_unknown_rather_than_guessed() {
    let (root, _) = package("no-build");
    let reply = explain(&root, &json!({"node": "root/button"})).expect("the report renders");
    assert!(reply.contains("scope unknown"), "{reply}");
    assert!(reply.contains("pruning unknown"), "{reply}");
    assert!(reply.contains("nichlink check"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A face whose manifest symbol is `-` has nothing tracked, and saying "strips -"
/// would read as a symbol named `-`. The end-to-end run against a real host is
/// what caught that wording.
/// 清单符号为 `-` 的面没有可跟踪的东西，而说"strips -"会被读成有个叫 `-` 的符号。真实宿主上的
/// 端到端运行抓到了这个措辞。
#[test]
fn a_face_with_no_tracked_symbol_is_not_reported_as_stripped() {
    let (root, name) = package("no-symbol");
    let out = root.join("target/nichlink/out");
    std::fs::create_dir_all(&out).expect("build output");
    let faces = face_views(&root, &name).expect("faces derive");
    std::fs::write(
        out.join("pruning_manifest.tsv"),
        format!(
            "# node\tsource\tsymbol\n{}\t{}\t-\n",
            faces[0].id, faces[0].source
        ),
    )
    .expect("pruning");
    let reply = explain(&root, &json!({"node": "root/button"})).expect("the report renders");
    assert!(reply.contains("pruning nothing to strip"), "{reply}");
    assert!(!reply.contains("pruning strips -"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The tree projection is bounded, because it is the one answer here that grows
/// with the project.
/// 树的投影是有上限的，因为它是这里唯一随项目变大的答案。
#[test]
fn the_tree_projection_is_bounded_by_limit() {
    let (root, name) = package("projection");
    publish(&root, &name, true);
    let reply = explain(&root, &json!({"limit": 1})).expect("the projection renders");
    assert!(reply.contains("slots:"), "{reply}");
    assert!(reply.contains("root/button"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}
