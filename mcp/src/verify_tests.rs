//! Tests for the verify loop: a healthy tree passes and the run publishes the
//! evidence the delta reads, and a broken tree fails by naming the node.
//! 校验闭环的测试：健康的树通过、且那次运行发布了差异所读的证据；坏掉的树以点名节点的失败作答。

use std::path::{Path, PathBuf};

use serde_json::json;

use super::verify;
use crate::apply::apply;

/// A throwaway package with no faces.
/// 一个没有注册面的一次性包。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-verify-{label}");
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

/// The provider/consumer pair the kernel admits.
/// 内核允许的那一对提供者与消费者。
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
    .expect("the consumer is admitted");
}

/// A healthy tree passes, and because the same call published the build evidence the
/// delta below it describes *this* tree: face for face, not the difference against a
/// stale build.
/// 健康的树通过；而且同一次调用发布了构建证据，因此它下面的差异描述的是**这棵**树：逐面一致，而不是
/// 与一次过期构建之间的差别。
#[test]
fn a_healthy_tree_passes_and_the_run_publishes_the_evidence() {
    let (root, _) = package("healthy");
    apply(
        &root,
        &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
    )
    .expect("the face is added");
    let reply = verify(&root, &json!({})).expect("the verdict renders");
    assert!(reply.contains("verdict ok"), "{reply}");
    assert!(reply.contains("build current"), "{reply}");
    assert!(
        reply.contains("the build matches the sources face for face"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Two packages verified from one process each keep their own namespace.
/// 在一个进程里校验两个包时，各自保留自己的命名空间。
///
/// The build pipeline's namespace used to be a first-write-wins process pin, so
/// the *second* package's published evidence was stamped with the first one's
/// namespace and this tool reported every face as re-identified — a wrong answer
/// that looks like a right one. Measured on CI and reproduced here with
/// `--test-threads=1`: `reidentified 1` with the first package's id on the run
/// side. The check is now scoped to the run, and this pins it through the tool
/// an agent actually calls rather than through the internals.
/// 构建管线的命名空间过去是"先到先得"的进程固定值，因此**第二个**包发布的证据会盖上第一个包的
/// 命名空间，本工具于是把每个面都报成身份变了——一个看起来正确的错误答案。CI 上实测到，并在本地用
/// `--test-threads=1` 复现：`reidentified 1`，运行侧是第一个包的 id。现在该判断被限定在这一次运行
/// 内，而这条测试通过代理真正调用的工具、而不是内部实现来钉住它。
#[test]
fn a_second_package_in_one_process_keeps_its_own_namespace() {
    for label in ["first", "second"] {
        let (root, name) = package(label);
        apply(
            &root,
            &json!({"action": "add", "apply": true, "fields": {"module": "label", "kind": "Label"}}),
        )
        .expect("the face is added");
        let reply = verify(&root, &json!({})).expect("the verdict renders");
        assert!(reply.contains(&format!("namespace {name}")), "{reply}");
        assert!(
            reply.contains("the build matches the sources face for face"),
            "{reply}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}

/// A tree whose requirement has no provider fails the verdict, and the diagnostic the
/// kernel already wrote names the offending node and its source location.
/// 需求没有提供者的树会让判断失败，而内核本来就写好的诊断点名了出问题的节点与它的源码位置。
#[test]
fn a_broken_tree_fails_the_verdict_and_names_the_node() {
    let (root, _) = package("broken");
    provider_and_consumer(&root);
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
    let reply = verify(&root, &json!({})).expect("the verdict renders");
    assert!(reply.contains("verdict failed"), "{reply}");
    // The requirements diagnostic names the phase, the consumer node, its source
    // line, the field it wanted and the provider kind it expected — so the reply is
    // actionable without a second call.
    // requirements 诊断点名了阶段、消费者节点、它的源码行、它想要的字段与它期望的提供者 kind——
    // 因此这份回复不需要第二次调用就能行动。
    assert!(reply.contains("phase=requirements"), "{reply}");
    assert!(reply.contains("missing capability"), "{reply}");
    assert!(reply.contains("field=cap.render"), "{reply}");
    assert!(reply.contains("expected=Label"), "{reply}");
    assert!(reply.contains("label/object/slider/slider.rs"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A directory that is not a package is refused by name before anything runs.
/// 不是包的目录会在任何东西运行之前被按名拒绝。
#[test]
fn a_directory_without_a_manifest_is_refused() {
    let bare = std::env::temp_dir().join(format!("mcp-verify-bare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bare);
    std::fs::create_dir_all(&bare).expect("bare directory");
    let error = verify(&bare, &json!({})).expect_err("a non-package is refused");
    assert!(error.contains("needs a Cargo package"), "{error}");
    let _ = std::fs::remove_dir_all(&bare);
}
