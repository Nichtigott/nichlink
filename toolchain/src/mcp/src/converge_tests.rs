//! Tests for the converged report: an answered requirement names its provider, an
//! unanswered one is named as such, the read plan lists the neighbourhood, and a
//! recorded run collapses the tree to the faces that actually ran.
//! 收敛报告的测试：被满足的需求点名它的提供者，未被满足的被如实点名，读计划列出邻域，而一次已记录的
//! 运行会把整棵树收敛到真正跑过的那些面。

use std::path::{Path, PathBuf};

use crate::run_method::{
    CallTrace, LocalKind, SourceLocation, trace_artifact_path, write_trace_artifact,
};
use serde_json::json;

use super::converge;
use crate::mcp::apply::apply;
use crate::mcp::converge_trace::{converge_from_trace, converge_from_trace_with};

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
    let root_id = nichlink_kernel::root_node_id(namespace);
    let label = nichlink_kernel::identity::NodeId::from_namespaced_path(
        namespace,
        "label/label.rs",
        "Label",
    );
    let mut trace = CallTrace::full();
    trace.with_at(root_id, "main", at("src/main.rs", 9, "main"), |trace| {
        // A value captured in a frame whose file declares no face: it belongs to the
        // run, not to any face, and the report must not attach it to one.
        // 在文件不声明任何面的帧里捕获的值：它属于这次运行而不属于任何面，报告不得把它挂到某个面上。
        let seed = trace.local("seen_in_main", "u8", 7, LocalKind::Binding);
        // A *connected* pair outside every face: an edge must be attributed by its
        // ends, not shown under whichever face happens to be printed.
        // 一对在任何面之外、且彼此相连的值：边必须按它的两端归属，而不是被挂在恰好被打印的那个面下。
        trace.transform(seed, "grown_in_main", "u8", 8);
        trace.with_at(
            label,
            "Label::render",
            at("src/label/label.rs", 12, "Label::render"),
            |trace| {
                let count = trace.local("count", "usize", 1, LocalKind::Binding);
                trace.transform(count, "shown", "usize", 2);
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

/// A run that touched two faces, so both file bounds have something to withhold.
/// 一次碰了两个面的运行，使那两处按文件的截断都有东西可扣。
fn two_face_run(root: &Path, namespace: &str) {
    for (module, kind) in [("label", "Label"), ("gauge", "Gauge")] {
        apply(
            root,
            &json!({"action": "add", "apply": true, "fields": {"module": module, "kind": kind}}),
        )
        .expect("the face is added");
    }
    let root_id = nichlink_kernel::root_node_id(namespace);
    let mut trace = CallTrace::full();
    trace.with_at(root_id, "main", at("src/main.rs", 9, "main"), |trace| {
        let label = nichlink_kernel::identity::NodeId::from_namespaced_path(
            namespace,
            "label/label.rs",
            "Label",
        );
        trace.with_at(
            label,
            "Label::render",
            at("src/label/label.rs", 12, "Label::render"),
            |_| {},
        );
        let gauge = nichlink_kernel::identity::NodeId::from_namespaced_path(
            namespace,
            "gauge/gauge.rs",
            "Gauge",
        );
        trace.with_at(
            gauge,
            "Gauge::render",
            at("src/gauge/gauge.rs", 5, "Gauge::render"),
            |_| {},
        );
    });
    write_trace_artifact(&trace, &trace_artifact_path(root), namespace)
        .expect("the artifact writes");
}

/// Both file bounds of this report — the list of faces that ran and the read plan —
/// say how many files they withheld. They used to stop at `limit` silently, while the
/// `faces that ran` and `read plan` headline counts left the reader to notice the
/// difference.
/// 本报告的两处按文件的截断——跑过的面的清单与读计划——都会说出扣下了多少个文件。它们过去在
/// `limit` 处默默停下，只有 `faces that ran` 与 `read plan` 的头条计数留给读者自己去发现差别。
#[test]
fn a_run_cut_by_the_limit_says_how_many_files_it_withheld() {
    let (root, name) = package("trace-bounded");
    two_face_run(&root, &name);
    let reply = converge(&root, &json!({"trace": true, "limit": 1})).expect("the report renders");
    assert!(
        reply.contains("faces that ran (2 of 2 declared, matched by source file)"),
        "{reply}"
    );
    assert_eq!(
        reply
            .matches("… truncated: 1 of 2 files withheld at the limit of 1")
            .count(),
        2,
        "the ran-file list and the read plan each declare theirs: {reply}"
    );
    assert!(
        reply.contains("read plan (2 files)"),
        "the headline still counts every file: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The values a run captured are attached to the face whose file the capturing frame
/// belongs to — and a value captured in a frame that belongs to no face is not.
/// 一次运行捕获的值会被挂到"捕获它的帧所属的文件"的那个面上——而属于任何面的帧里捕获的值不会被挂。
#[test]
fn the_values_a_run_captured_are_attached_to_the_faces_that_ran() {
    let (root, name) = package("trace-values");
    recorded_run(&root, &name, &name);
    let reply = converge(&root, &json!({"trace": true})).expect("the report renders");
    assert!(reply.contains("values (2)"), "{reply}");
    assert!(
        reply.contains("count: usize = 1  [let, observed]"),
        "{reply}"
    );
    assert!(
        reply.contains("shown: usize = 2  [let, observed]"),
        "{reply}"
    );
    assert!(reply.contains("edges (1)"), "{reply}");
    assert!(reply.contains("count -> shown  (transform)"), "{reply}");
    assert!(
        !reply.contains("seen_in_main") && !reply.contains("grown_in_main"),
        "a value from a frame outside every face must not be attached to one: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The per-face detail cap is a parameter whose default is the constant, so the branch
/// that reports a stopped detail block is reachable: at the default cap this run prints
/// its values, and at a cap of one the same run stops the detail, says it stopped, and
/// names the cap that did it. The sentence used to have no nail at all, because reaching
/// it took a report past the constant.
/// 逐面细节上限是一个默认值取常数的参数，因此"报告细节停止"的那一支是可达的：在默认上限下这次运行会
/// 打印它的值，而在上限为 1 时同一次运行停止细节、说出它停了，并点名是哪道上限做的。那句话过去完全
/// 没有钉子，因为要到达它需要一份越过常数的报告。
#[test]
fn the_per_face_detail_cap_says_when_it_stopped_and_names_the_cap() {
    let (root, name) = package("trace-detail-cap");
    recorded_run(&root, &name, &name);
    let faces = crate::build_method::face_views(&root, &name).expect("faces derive");
    let full = converge_from_trace(&root, &faces, 40).expect("the report renders");
    assert!(
        full.contains("values (2)"),
        "at the default cap this run's values print: {full}"
    );
    assert!(
        !full.contains("lines of per-face detail"),
        "this run does not reach the default cap, so the sentence must be absent: {full}"
    );
    let capped = converge_from_trace_with(&root, &faces, 40, 1).expect("the report renders");
    assert!(
        !capped.contains("values ("),
        "the cap must stop the detail, not only label it: {capped}"
    );
    assert!(
        capped.contains("lines of per-face detail withheld at the limit of 1"),
        "the sentence names the cap that did it: {capped}"
    );
    assert!(
        capped.contains("the exact count is not computed"),
        "a stopped block was never counted, and the sentence says so: {capped}"
    );
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
