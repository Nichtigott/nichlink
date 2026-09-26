//! Tests for the trace report: absence is actionable, a foreign artifact is
//! refused, and a matching one renders.
//! trace 报告的测试：缺失要可行动，异树 artifact 要被拒绝，匹配的要能渲染。

use std::path::{Path, PathBuf};

use nichlink_build_method::face_views;
use nichlink_run_method::{CallTrace, LocalKind, trace_artifact_path, write_trace_artifact};
use serde_json::json;

use super::trace;

/// A throwaway package with one hand-written root face, plus that face's identity.
/// 一个含一个手写根面的一次性包，外加该面的身份。
fn package(label: &str) -> (PathBuf, String, nichlink::identity::NodeId) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-trace-{label}");
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
    let id = face_views(&root, &name).expect("faces derive")[0].id;
    (root, name, id)
}

/// Record one frame into an artifact, the way a host would.
/// 像宿主那样把一个帧记进 artifact。
fn record(root: &Path, namespace: &str, node: nichlink::identity::NodeId) {
    let mut recorded = CallTrace::full();
    recorded.with(node, "button", |recorded| {
        recorded.local("input", "u32", 1, LocalKind::Input);
    });
    write_trace_artifact(&recorded, &trace_artifact_path(root), namespace)
        .expect("the artifact writes");
}

/// No artifact is the normal state, and the answer must say how one appears.
/// 没有 artifact 是常态，答案必须说出它从哪来。
#[test]
fn an_absent_trace_names_the_way_to_produce_one() {
    let (root, _, _) = package("absent");
    let reply = trace(&root, &json!({})).expect("the report renders");
    assert!(reply.contains("trace absent"), "{reply}");
    assert!(reply.contains("write_trace_artifact"), "{reply}");
    assert!(reply.contains("NICH_LINK_TRACE=full"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A trace recorded under this namespace, naming this face, renders a report.
/// 在本命名空间下记录、且点名本面的 trace，会渲染出一份报告。
#[test]
fn a_matching_artifact_renders_the_call_report() {
    let (root, name, id) = package("matching");
    record(&root, &name, id);
    let reply = trace(&root, &json!({})).expect("the report renders");
    assert!(reply.contains("frames 1"), "{reply}");
    assert!(!reply.contains("REFUSED"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// An artifact from another package is refused rather than rendered: its frames
/// are identities from a domain this tree does not share.
/// 来自另一个包的 artifact 被拒绝而不是被渲染：它的帧来自这棵树不共享的身份域。
#[test]
fn a_foreign_artifact_is_refused_by_name() {
    let (root, _, id) = package("foreign");
    record(&root, "some-other-package", id);
    let reply = trace(&root, &json!({})).expect("the refusal renders");
    assert!(reply.contains("REFUSED"), "{reply}");
    assert!(reply.contains("namespace"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A frame naming a node this tree does not have is refused too, and the count is
/// named so the agent knows how much of the trace is unusable.
/// 帧点名的节点不在这棵树里时同样拒绝，并说出数量，让代理知道这份 trace 有多少不可用。
#[test]
fn a_frame_from_another_tree_is_refused_with_its_count() {
    let (root, name, _) = package("ghost-frame");
    let ghost = nichlink::identity::NodeId::from_namespaced_path(&name, "ghost/ghost.rs", "Ghost");
    record(&root, &name, ghost);
    let reply = trace(&root, &json!({})).expect("the refusal renders");
    assert!(reply.contains("REFUSED"), "{reply}");
    assert!(reply.contains("frame(s) name nodes"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}
