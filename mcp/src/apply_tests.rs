//! Tests for the write path: what a preview promises, and what an apply does.
//! 写入路径的测试：预览承诺了什么，落盘做了什么。

use std::path::PathBuf;

use nichlink_build_method::face_views;
use serde_json::json;

use super::apply;

/// A throwaway package with no faces: `cargo metadata` can name it (a manifest
/// and a library target), and every face in it is one the tests add.
/// 一个没有注册面的一次性包：`cargo metadata` 能给它命名（有清单与库目标），而它里面的每个面都是
/// 测试加进去的。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-apply-{label}");
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

/// The path one reply says it wrote, as the reply spells it.
/// 一条回复说自己写下的路径，按回复的写法。
fn written(reply: &str) -> PathBuf {
    reply
        .lines()
        .find_map(|line| line.strip_prefix("would write "))
        .or_else(|| reply.lines().find_map(|line| line.strip_prefix("applied ")))
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("no written path in: {reply}"))
}

/// A preview is the real operation on a copy: it reports the face it would create
/// and the tree that would result, and it writes nothing into the project. The
/// second half is the load-bearing one — a preview that leaked into the project
/// would make the confirmation step a lie.
/// 预览是在副本上运行的真实操作：它报告会创建哪个面、会得到哪棵树，而它不向项目写入任何东西。
/// 后半句是承重的那一半——泄漏进项目的预览会让"确认"这一步变成谎言。
#[test]
fn a_preview_reports_the_resulting_tree_and_leaves_the_project_alone() {
    let (root, name) = package("preview");
    let reply = apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the preview runs");
    assert!(reply.contains("action preview"), "{reply}");
    assert!(reply.contains("namespace "), "{reply}");
    assert!(reply.contains("faces 1"), "{reply}");
    assert!(reply.contains("root/button"), "{reply}");
    assert!(reply.contains("diff:"), "{reply}");

    let target = written(&reply);
    assert!(
        target.starts_with(&root),
        "the report must name a path in the project: {reply}"
    );
    assert!(
        !target.exists(),
        "a preview must not write into the project: {reply}"
    );
    assert!(
        face_views(&root, &name)
            .expect("the project reads")
            .is_empty(),
        "the project must still have no faces"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An apply writes the face, and the next call can aim with the tree the previous
/// reply reported: a child is added under the logical path the first apply
/// produced, which is the loop that makes the tool usable by an agent.
/// 落盘会写下这个面，而下一次调用可以用上一条回复所报告的树来瞄准：子面被加在第一次落盘产生的
/// 逻辑路径之下——正是这个回路让工具对代理可用。
#[test]
fn an_apply_writes_the_face_and_its_path_can_aim_the_next_call() {
    let (root, name) = package("apply");
    let control = json!({
        "action": "add",
        "parent": "root",
        "apply": true,
        "fields": {"module": "control", "kind": "Control", "needs_registry": true},
    });
    let reply = apply(&root, &control).expect("the root face is created");
    assert!(reply.contains("action apply"), "{reply}");
    assert!(reply.contains("faces 1"), "{reply}");
    assert!(written(&reply).is_file(), "{reply}");

    let button = json!({
        "action": "add",
        "parent": "root/control",
        "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
    });
    let preview = apply(&root, &button).expect("the child previews against the real tree");
    assert!(
        preview.contains("root/control/button"),
        "the logical path from the first reply must be a usable parent: {preview}"
    );
    assert!(
        written(&preview).ends_with("control/object/button/button.rs"),
        "the executor's own placement decides the file: {preview}"
    );
    assert!(preview.contains("faces 2"), "{preview}");
    assert!(
        !written(&preview).exists(),
        "the child is still only previewed: {preview}"
    );

    let applied = apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root/control",
            "apply": true,
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the child is created");
    assert!(applied.contains("faces 2"), "{applied}");
    let faces = face_views(&root, &name).expect("the project reads");
    assert_eq!(faces.len(), 2, "{faces:?}");
    assert!(
        faces.iter().any(|face| face.path == "root/control/button"),
        "{faces:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A package Cargo cannot name is refused before anything is copied or written:
/// the namespace is the identity of every face the edit would create, and a guess
/// would author faces no host compiles.
/// Cargo 说不出名字的包在复制或写入任何东西之前就被拒绝：命名空间是这次编辑会创建的每个面的身份，
/// 而猜出来的值会创作出没有宿主编译的面。
#[test]
fn a_package_that_cannot_be_named_is_refused_with_the_way_out() {
    let bare = std::env::temp_dir().join(format!("mcp-apply-nameless-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&bare);
    std::fs::create_dir_all(&bare).expect("bare directory");
    let error = apply(
        &bare,
        &json!({"action": "add", "fields": {"module": "button"}}),
    )
    .expect_err("a package Cargo cannot name has no namespace");
    assert!(error.contains("identity namespace"), "{error}");
    assert!(error.contains("NICH_LINK_NAMESPACE"), "{error}");
    let _ = std::fs::remove_dir_all(&bare);
}

/// An action the tool does not implement is refused by name instead of being
/// guessed at; `delete` moves a directory and will be added with its own guards.
/// 工具没有实现的动作按名字被拒，而不是被猜着执行；`delete` 会搬走目录，将带着自己的守卫再加。
#[test]
fn an_action_that_is_not_implemented_is_refused_by_name() {
    let (root, _) = package("unsupported");
    let error = apply(&root, &json!({"action": "delete", "node": "root/button"}))
        .expect_err("delete is not implemented yet");
    assert!(error.contains("not implemented"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}
