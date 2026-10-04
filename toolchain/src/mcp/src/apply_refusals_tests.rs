//! Pins for the write path's refusals and for the block every reply carries.
//! 写入路径的拒绝、以及每条回复都带的那个块的钉子。
//!
//! Split out of `apply_tests.rs` for the 800-code-line test ratchet, and it is the subject
//! `apply_refusals.rs` owns: what a refusal says, and what the constant `consequences` block does
//! when the same session has already read it (audit `W2-5`, D3).
//! 从 `apply_tests.rs` 拆出来是为了 800 代码行的测试棘轮，而它是 `apply_refusals.rs` 自己的题目：拒绝说什么、
//! 以及同一个会话已经读过之后那个常量 `consequences` 块怎么办（审计 `W2-5`，D3）。

use serde_json::json;

use crate::mcp::apply::apply;
use crate::mcp::apply::apply_tests::package;

/// The consequences disclaimer is printed once per session, and referred back to afterwards.
/// consequences 的免责声明每个会话印一次，此后只回指。
///
/// Audit `W2-5` (D3): the `entry plan` line and the "not covered" sentence are constant for a tree,
/// and a workflow that writes twice pays for them twice. The second write gets one line that points
/// at the first; a **different project** gets its own first time.
/// 审计 `W2-5`（D3）：`entry plan` 行与"not covered"那句对一棵树是常量，而写两次的工作流会为它们付两次。
/// 第二次写入拿到一行指向第一次的回指；**另一个项目**有自己的第一次。
#[test]
fn the_write_disclaimer_is_said_once_per_session_and_project() {
    crate::mcp::session::forget();
    let (root, _) = package("consequences-once");
    // The parent's own creation is the first write, so **it** carries the whole disclaimer: every
    // write prints one, which is exactly why the second one should not.
    // 父面的创建就是第一次写入，因此**它**带着完整免责声明：每一次写入都印一份，而正是这一点让第二次不该再印。
    let first = apply(
        &root,
        &json!({"action": "add", "parent": "root", "apply": true,
                "fields": {"module": "control", "kind": "Control", "needs_registry": true}}),
    )
    .expect("the parent face");
    assert!(
        first.contains("not covered: this lists test lines that spell"),
        "the first write carries the whole disclaimer: {first}"
    );
    let second = apply(
        &root,
        &json!({"action": "add", "parent": "root/control", "apply": true,
                "fields": {"module": "slider", "kind": "Slider"}}),
    )
    .expect("the second write");
    assert!(
        second.contains("not covered: unchanged from this session's earlier write")
            && !second.contains("not covered: this lists test lines that spell"),
        "the second refers back instead of repeating: {second}"
    );
    // The comparison is between the two **disclaimer lines**, not between two whole replies: the
    // parent's creation and a child's add say different things about the tree, and the claim here is
    // only about the constant block.
    // 比的是两条**免责声明行**，不是两份整回复：父面的创建与子面的新增对树说的话不同，而这里的主张只关于
    // 那个常量块。
    let disclaimer = |reply: &str| {
        reply
            .lines()
            .find(|line| line.trim_start().starts_with("not covered:"))
            .unwrap_or_default()
            .to_owned()
    };
    assert!(
        disclaimer(&second).len() < disclaimer(&first).len(),
        "and the disclaimer itself is shorter ({} vs {})",
        disclaimer(&second).len(),
        disclaimer(&first).len()
    );
    // Another project is another first time.
    // 另一个项目是另一次第一次。
    let (other, _) = package("consequences-elsewhere");
    // Again the parent's creation is that project's first write.
    // 同样，父面的创建是那个项目的第一次写入。
    let elsewhere = apply(
        &other,
        &json!({"action": "add", "parent": "root", "apply": true,
                "fields": {"module": "control", "kind": "Control", "needs_registry": true}}),
    )
    .expect("the other parent");
    assert!(
        elsewhere.contains("not covered: this lists test lines that spell"),
        "another project has its own first time: {elsewhere}"
    );
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&other);
}
