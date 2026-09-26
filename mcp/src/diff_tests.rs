//! Tests for the tree diff: a match, an addition, a removal, and a face that
//! changed identity under a file that did not move.
//! 树 diff 的测试：匹配、新增、移除，以及文件没动而身份变了的面。

use std::path::{Path, PathBuf};

use nichlink_build_method::face_views;
use serde_json::json;

use super::diff;

/// A throwaway package with one hand-written root face.
/// 一个含一个手写根面的一次性包。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-diff-{label}");
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

/// Publish a build manifest with exactly the rows given.
/// 发布一份只含给定行的构建清单。
fn publish(root: &Path, rows: &[String]) {
    let out = root.join("target/nichlink/out");
    std::fs::create_dir_all(&out).expect("build output");
    let mut text = String::from("# node\tsource\tsymbol\n");
    for row in rows {
        text.push_str(row);
        text.push('\n');
    }
    std::fs::write(out.join("pruning_manifest.tsv"), text).expect("pruning manifest");
}

/// A manifest matching the sources is reported as a match, not as an empty diff.
/// 与源码一致的清单被报成一致，而不是一份空 diff。
#[test]
fn a_matching_build_reports_no_delta() {
    let (root, name) = package("match");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    publish(&root, &[format!("{}\t{}\t-", face.id, face.source)]);
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("added 0  gone 0  reidentified 0"), "{reply}");
    assert!(reply.contains("face for face"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A face the build never saw is added; a face it saw and the sources no longer
/// declare is gone.
/// 构建从未见过的面是新增；构建见过而源码不再声明的是移除。
#[test]
fn an_addition_and_a_removal_are_both_named() {
    let (root, name) = package("delta");
    let old = nichlink::identity::NodeId::from_namespaced_path(&name, "old/old.rs", "Old");
    publish(&root, &[format!("{old}\told/old.rs\t-")]);
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("added 1  gone 1"), "{reply}");
    assert!(reply.contains("+ root/button"), "{reply}");
    assert!(reply.contains("- old/old.rs"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The same source with a different identity is the one case a text diff would
/// miss entirely.
/// 同一份源码、不同的身份，正是文本 diff 完全看不见的那种情况。
#[test]
fn a_changed_identity_under_an_unmoved_file_is_reported() {
    let (root, name) = package("reidentified");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    let renamed = nichlink::identity::NodeId::from_namespaced_path(&name, &face.source, "Renamed");
    publish(&root, &[format!("{renamed}\t{}\t-", face.source)]);
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("reidentified 1"), "{reply}");
    assert!(reply.contains("~ root/button"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Without a build the answer names the command that produces one.
/// 没有构建时，答案点名产出它的命令。
#[test]
fn a_missing_build_asks_for_one_instead_of_diffing_nothing() {
    let (root, _) = package("no-build");
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("no build evidence"), "{reply}");
    assert!(reply.contains("nichlink check"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}
