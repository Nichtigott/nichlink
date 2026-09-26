//! Tests for the converged report: an answered requirement names its provider, an
//! unanswered one is named as such, and the read plan lists the neighbourhood.
//! 收敛报告的测试：被满足的需求点名它的提供者，未被满足的被如实点名，读计划列出邻域。

use std::path::{Path, PathBuf};

use serde_json::json;

use super::converge;
use crate::apply::apply;

/// A throwaway package with no faces.
/// 一个没有注册面的一次性包。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-converge-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    (root, name)
}

/// Build the pair the kernel admits: a provider that owns a registry, and a
/// consumer under it whose requirement names the provider's kind.
/// 构造内核允许的那一对：一个拥有注册机的提供者，以及它之下的消费者，消费者的需求点名提供者的 kind。
fn provider_and_consumer(root: &Path) {
    apply(
        root,
        &json!({"action": "add", "apply": true,
                "fields": {"module": "label", "kind": "Label", "needs_registry": true,
                           "provides": "cap.render"}}),
    )
    .expect("the provider is admitted");
    apply(
        root,
        &json!({"action": "add", "parent": "root/label", "apply": true,
                "fields": {"module": "slider", "kind": "Slider",
                           "requires": "cap.render=>Label"}}),
    )
    .expect("the consumer is admitted under its provider");
}

/// An answered requirement names who answers it, and the read plan carries the
/// parent and the child so the agent knows which files to open.
/// 被满足的需求点名谁满足它，读计划带上父级与子面，让代理知道该打开哪些文件。
#[test]
fn an_answered_requirement_names_its_provider_and_the_plan_lists_the_neighbourhood() {
    let (root, _) = package("answered");
    provider_and_consumer(&root);
    apply(
        &root,
        &json!({"action": "add", "parent": "root/label", "apply": true,
                "fields": {"module": "knob", "kind": "Knob"}}),
    )
    .expect("the sibling is admitted");
    let reply = converge(&root, &json!({"node": "root/label/slider"})).expect("the report renders");
    assert!(
        reply.contains("cap.render => Label  answered by root/label"),
        "{reply}"
    );
    assert!(reply.contains("read plan (2 files)"), "{reply}");
    assert!(reply.contains("label/object/slider/slider.rs"), "{reply}");
    assert!(reply.contains("(parent)"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The verdict no single tool can give, in the moment it matters most: once the
/// provider's source stops offering the capability, the package's own faces *are*
/// rejected, and every registry-backed tool refuses to load the tree at all. This
/// one reports the rejection as its headline finding — naming the descendant node
/// and its source location — and still prints the read plan, because that needs
/// only the source-derived tree.
/// 任何单一工具都给不出的判断，而且出现在它最重要的时刻：提供者的源码一旦不再提供该能力，本包自己的面
/// **就是**被拒绝的，所有依赖注册机的工具都会拒绝加载这棵树。这一个把那次拒绝当成头条发现报出来——
/// 点名后代节点与它的源码位置——并且仍然打印读计划，因为那只需要源码推导出的树。
#[test]
fn a_rejected_tree_is_reported_as_the_verdict_rather_than_hidden_behind_an_error() {
    let (root, _) = package("unanswered");
    provider_and_consumer(&root);
    let before =
        converge(&root, &json!({"node": "root/label/slider"})).expect("the report renders");
    assert!(before.contains("answered by root/label"), "{before}");
    let path = root.join("src/label/label.rs");
    let source = std::fs::read_to_string(&path).expect("the provider's source");
    let without = source
        .lines()
        .filter(|line| !line.contains("provides:"))
        .collect::<Vec<_>>()
        .join("\n");
    assert_ne!(
        source, without,
        "the fixture must have a provides field to drop"
    );
    std::fs::write(&path, without).expect("the hand edit lands");
    let after = converge(&root, &json!({"node": "root/label/slider"})).expect("the report renders");
    assert!(
        after.contains("kernel verdict: this package's own faces are rejected"),
        "{after}"
    );
    assert!(after.contains("no provider"), "{after}");
    assert!(after.contains("cap.render"), "{after}");
    assert!(
        after.contains("label/object/slider/slider.rs"),
        "the verdict must name the descendant and where it is: {after}"
    );
    assert!(
        after.contains("read plan ("),
        "the read plan needs only the source tree: {after}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The cross-file break is already prevented where it would be introduced: the write
/// path refuses an edit that would leave a descendant's requirement unanswered, and
/// the refusal names the descendant and its source location. This is the half
/// `nichlink.converge` cannot show, because the kernel does not let the state exist.
/// 那个跨文件破坏在它会被引入的地方就已经被拦住了：写入路径拒绝一次会让后代的需求失去答案的编辑，
/// 而拒绝里点名了那个后代与它的源码位置。这是 `nichlink.converge` 展示不了的另一半，因为内核不
/// 允许那个状态存在。
#[test]
fn the_write_path_refuses_an_edit_that_would_orphan_a_requirement() {
    let (root, _) = package("orphan-refused");
    provider_and_consumer(&root);
    let error = apply(
        &root,
        &json!({"action": "edit", "node": "root/label", "apply": true, "fields": {"provides": ""}}),
    )
    .expect_err("the kernel refuses to orphan the requirement");
    assert!(error.contains("no provider"), "{error}");
    assert!(error.contains("cap.render"), "{error}");
    assert!(
        error.contains("label/object/slider/slider.rs"),
        "the refusal must name the descendant and where it is: {error}"
    );
    assert!(
        root.join("src/label/label.rs").is_file()
            && std::fs::read_to_string(root.join("src/label/label.rs"))
                .expect("the provider's source")
                .contains("provides:"),
        "a refused edit must leave the provider's source alone"
    );
    let _ = std::fs::remove_dir_all(&root);
}
