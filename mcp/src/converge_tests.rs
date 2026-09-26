//! Tests for the converged report: an answered requirement names its provider, an
//! unanswered one is named as such, the read plan lists the neighbourhood, and a
//! recorded run collapses the tree to the faces that actually ran.
//! 收敛报告的测试：被满足的需求点名它的提供者，未被满足的被如实点名，读计划列出邻域，而一次已记录的
//! 运行会把整棵树收敛到真正跑过的那些面。

use std::path::{Path, PathBuf};

use nichlink_run_method::{CallTrace, SourceLocation, trace_artifact_path, write_trace_artifact};
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
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    (root, name)
}

/// One recorded callsite.
/// 一个已记录的调用点。
fn at(file: &'static str, line: u32, function: &'static str) -> SourceLocation {
    SourceLocation {
        file,
        line,
        column: 1,
        function,
    }
}

/// A run whose frames are: `main` outside any face, then the `Label` face twice —
/// once under the plain `src/…` spelling a standalone package's `file!()` records,
/// and once under the workspace-relative spelling a package inside a workspace
/// records. A face is matched by file, so both must land on the same face.
/// 一次运行，帧为：任何面之外的 `main`，然后是 `Label` 面两次——一次用独立包的 `file!()` 记录的
/// 平铺 `src/…` 拼法，一次用工作区内包记录的相对工作区拼法。面是按文件匹配的，因此两次都必须落到
/// 同一个面上。
fn recorded_run(root: &Path, namespace: &str, recorded_as: &str) {
    apply(
        root,
        &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
    )
    .expect("the face is added");
    let root_id = nichlink::root_node_id(namespace);
    let label =
        nichlink::identity::NodeId::from_namespaced_path(namespace, "label/label.rs", "Label");
    let mut trace = CallTrace::full();
    trace.with_at(root_id, "main", at("src/main.rs", 9, "main"), |trace| {
        trace.with_at(
            label,
            "Label::render",
            at("src/label/label.rs", 12, "Label::render"),
            |trace| {
                trace.with_at(
                    label,
                    "Label::paint",
                    at("crates/host/src/label/label.rs", 40, "Label::paint"),
                    |_| {},
                );
            },
        );
    });
    write_trace_artifact(&trace, &trace_artifact_path(root), recorded_as)
        .expect("the artifact writes");
}

/// The convergence this step exists for: a recorded run turns the whole tree into
/// the files that both declare a face and ran, names the frames that landed there,
/// and counts what fell outside.
/// 这一步存在的意义就是这次收敛：一次已记录的运行把整棵树变成"既声明了面、又真的跑了"的那些文件，
/// 点名落在其中的帧，并把落在外面的一并计数。
#[test]
fn a_recorded_run_collapses_the_tree_to_the_faces_that_ran() {
    let (root, name) = package("trace");
    recorded_run(&root, &name, &name);
    let reply = converge(&root, &json!({"trace": true})).expect("the report renders");
    assert!(reply.contains("frames 3"), "{reply}");
    assert!(
        reply.contains("faces that ran (1 of 1 declared, matched by source file)"),
        "{reply}"
    );
    assert!(reply.contains("root/label"), "{reply}");
    assert!(reply.contains("kind=Label"), "{reply}");
    assert!(reply.contains("2 frame(s)"), "{reply}");
    assert!(
        reply.contains("Label::render  src/label/label.rs:12"),
        "the plain `src/…` spelling must land on the face: {reply}"
    );
    assert!(
        reply.contains("Label::paint  crates/host/src/label/label.rs:40"),
        "a workspace-relative spelling must land on the same face: {reply}"
    );
    assert!(
        reply.contains("frames in a face 2 / outside any declared face 1"),
        "{reply}"
    );
    assert!(reply.contains("src/main.rs (1)"), "{reply}");
    assert!(reply.contains("read plan (1 files)"), "{reply}");
    assert!(reply.contains("label/label.rs"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Absence is an answer, not an error — the same sentence `nichlink.trace` gives,
/// because both tools read the same artifact and neither invents its own wording.
/// 缺失是答案而不是错误——与 `nichlink.trace` 同一句话，因为两个工具读的是同一份 artifact，谁也不
/// 另造一套说法。
#[test]
fn an_absent_run_is_answered_with_the_way_to_record_one() {
    let (root, _) = package("trace-absent");
    let reply = converge(&root, &json!({"trace": true})).expect("absence is an answer");
    assert!(reply.contains("trace absent"), "{reply}");
    assert!(reply.contains("NICH_LINK_TRACE"), "{reply}");
    assert!(reply.contains("src/main.rs"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A run recorded under another namespace is refused, not silently mapped onto
/// this tree's faces.
/// 在另一个命名空间下记录的运行会被拒绝，而不是被悄悄映射到这棵树的面。
#[test]
fn a_run_from_another_tree_is_refused_by_name() {
    let (root, name) = package("trace-foreign");
    recorded_run(&root, &name, "some-other-package");
    let reply = converge(&root, &json!({"trace": true})).expect("the refusal is an answer");
    assert!(
        reply.contains("REFUSED: the artifact does not describe this tree"),
        "{reply}"
    );
    assert!(
        reply.contains("some-other-package"),
        "the refusal names what it found: {reply}"
    );
    assert!(!reply.contains("faces that ran"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
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
