//! The consumer half of a write: what a change does to the tree's readers outside its own file.
//! 一次写入的消费方那一半：这次改动对"这个文件之外读它的东西"做了什么。
//!
//! Split out of `apply_tests.rs` when that file crossed the 800-code-line test ceiling — the same
//! split, for the same reason, that `apply_refusals_tests.rs` records.
//! 在那个文件越过 800 行代码行的测试上限时拆出来——与 `apply_refusals_tests.rs` 记录的同一种拆分、同一个理由。

use serde_json::json;

use super::apply;
use super::apply_tests::package;

/// The consumer story a deepen tells is the **same** in the preview and after the write (audit `W5-3`).
/// deepen 讲的消费方说法，在预览里与写入之后是**同一件**（审计 `W5-3`）。
///
/// This is the acceptance's real content: a preview that told a different consumer story from the one
/// that lands would be an advertisement. Both replies are produced by the **same executor** running on
/// whichever tree it was handed, so this pin is checking that the report does not filter them — and
/// that a tree with plan entries gets them named, with the reason they keep pointing where they point.
/// 这正是验收的实质：一个与落盘说法不同的消费方故事，就是广告。两条回复由**同一个执行器**在它拿到的那棵树
/// 上产出，因此这条钉子查的是报告没有把它们过滤掉——以及有计划条目的树会看到它们被点名，并看到"它们为何仍然
/// 指向原处"。
#[test]
fn the_consumer_story_is_the_same_in_the_preview_and_after_the_write() {
    let (root, _) = package("deepen-consumers");
    apply(
        &root,
        &json!({"action": "add", "parent": "root", "apply": true,
                "fields": {"module": "control", "kind": "Control", "needs_registry": true}}),
    )
    .expect("the parent face");
    apply(
        &root,
        &json!({"action": "add", "parent": "root/control", "apply": true,
                "fields": {"module": "button", "kind": "Button"}}),
    )
    .expect("the leaf face");
    // One plan that targets the face being deepened: this is the consumer the diff exists for.
    // 一条针对"被做深的那个面"的计划：这就是 diff 为之存在的消费方。
    let names = crate::mcp::registry::namespace(&root).expect("the fixture's namespace");
    let target = nichlink_kernel::identity::NodeId::from_namespaced_path(
        &names,
        "control/object/button/button.rs",
        "Button",
    );
    let plan = root.join(".nichlink/external-grafts/swap/graft.plan");
    std::fs::create_dir_all(plan.parent().expect("plan directory")).expect("plan dir");
    std::fs::write(
        &plan,
        format!(
            "version = 1\ntarget = {target}\ntarget_path = root/control/button\ngraft = swap\nfull = false\n"
        ),
    )
    .expect("the plan");
    let preview = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button",
                "inside": {"parts": {"label": "String"}}}),
    )
    .expect("the deepen preview runs");
    let landed = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button", "apply": true,
                "inside": {"parts": {"label": "String"}}}),
    )
    .expect("the deepen applies");
    let consumers = |reply: &str| {
        reply
            .lines()
            .filter(|line| line.starts_with("consumers  ") || line.starts_with("slots      "))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        consumers(&preview),
        consumers(&landed),
        "the preview is not an advertisement:\n{preview}\n---\n{landed}"
    );
    assert!(
        preview.contains("the layer adds 1 filling option(s) inside `Button`"),
        "the layer's own options are named: {preview}"
    );
    assert!(
        preview.contains(
            "1 of 1 graft plan entry(ies) target this face or its subtree (swap targets \
             root/control/button)"
        ) && preview.contains("leaves every")
            && preview.contains("target and path alone"),
        "the consumer line names the plan and the reason it keeps pointing there: {preview}"
    );
}
