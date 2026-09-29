//! Pins for the NAM-33 verb-table gate: counter-proofs first, then the workspace ratchet.
//! NAM-33 动词表门禁的钉子：先反证，再是工作区棘轮。

use super::verb_table::{Violation, violations, violations_in};

fn names(found: &[Violation]) -> Vec<String> {
    found.iter().map(|item| item.name.clone()).collect()
}

/// A `get_` that does not return an `Option` is the table's `find_`.
/// 不返回 `Option` 的 `get_` 就是表里的 `find_`。
#[test]
fn a_get_that_is_not_an_option_is_reported() {
    let source = "pub fn get_face(&self) -> Face {\n    todo!()\n}\n";
    let found = violations_in("x/src/lib.rs", source);
    assert_eq!(names(&found), vec!["get_face".to_owned()]);
    assert!(found[0].why.contains("find_"));
}

/// The same name returning an `Option` is exactly what the table asks for.
/// 同名但返回 `Option` 的，正是表里要求的样子。
#[test]
fn a_get_that_is_an_option_is_exempt() {
    let source = "pub fn get_face(&self) -> Option<Face> {\n    todo!()\n}\n";
    assert!(violations_in("x/src/lib.rs", source).is_empty());
}

/// A bare verb is an entry position's privilege: `run` in `build.rs` is fine,
/// `run` in a library file is not.
/// 裸动词是入口位的特权：`build.rs` 里的 `run` 可以，库文件里的 `run` 不行。
#[test]
fn a_bare_verb_is_only_allowed_at_an_entry_position() {
    assert!(violations_in("toolchain/src/build_time/build.rs", "fn run() {}\n").is_empty());
    assert!(violations_in("toolchain/src/cli/src/bin/nichlink.rs", "fn main() {}\n").is_empty());
    let found = violations_in("toolchain/src/runtime/src/lib.rs", "pub fn run() {}\n");
    assert_eq!(names(&found), vec!["run".to_owned()]);
}

/// An unknown bare verb is reported wherever it sits.
/// 不在清单里的裸动词在任何位置都报。
#[test]
fn a_bare_table_verb_outside_an_entry_position_is_reported() {
    let found = violations_in("x/src/lib.rs", "pub fn handle() {}\n");
    assert_eq!(names(&found), vec!["handle".to_owned()]);
    // A single-word *noun* is not a bare verb: the table has nothing to say about it.
    // 单词**名词**不是裸动词：表对它没有话可说。
    assert!(violations_in("x/src/lib.rs", "pub fn dispatch() {}\n").is_empty());
}

/// `collect_` on a private function is the recursive helper the table calls `visit_`.
/// 私有函数上的 `collect_` 正是表里叫作 `visit_` 的递归 helper。
#[test]
fn a_private_collect_is_reported() {
    let source = "fn collect_leaves(node: &Node, out: &mut Vec<Id>) {\n    todo!()\n}\n";
    let found = violations_in("x/src/lib.rs", source);
    assert_eq!(names(&found), vec!["collect_leaves".to_owned()]);
    assert!(
        violations_in(
            "x/src/lib.rs",
            "pub fn collect_leaves() -> Vec<Id> { todo!() }\n"
        )
        .is_empty()
    );
}

/// A doc comment that merely mentions a violating name is not a declaration.
/// 仅仅**提到**违规名字的文档注释不是声明。
#[test]
fn a_mention_is_not_a_declaration() {
    let source = "/// Calls `get_face` under the hood.\n/// 底层会调用 `get_face`。\npub fn face_name(&self) -> Face {\n    todo!()\n}\n";
    assert!(violations_in("x/src/lib.rs", source).is_empty());
}

/// The workspace itself: the count is pinned so the gate has a ratchet, and the
/// list is printed so a new violation is visible in the failure output.
/// 工作区本身：数量被钉住，使门禁成为棘轮；违规清单也会打印，因此新增违规一看即知。
#[test]
fn the_shipped_workspace_verb_table_holds() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root");
    let found = violations(root);
    println!("verb-table violations: {}", found.len());
    for item in &found {
        println!("  {}:{} {}", item.file, item.line, item.name);
    }
    // PINNED is the measured workspace count at the time this gate landed; it may fall
    // but must not grow.
    // PINNED 是本门禁落地时实测的工作区数量；它只许下降，不许上升。
    const PINNED: usize = 81;
    assert!(
        found.len() <= PINNED,
        "the verb table grew {} violations (pinned {PINNED}); see the list above",
        found.len()
    );
}
