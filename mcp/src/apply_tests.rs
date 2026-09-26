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
        .or_else(|| {
            reply
                .lines()
                .find_map(|line| line.strip_prefix("would move "))
        })
        .or_else(|| reply.lines().find_map(|line| line.strip_prefix("moved ")))
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

/// A partial edit keeps every field the request did not name. The executor rebuilds
/// a face from the values it is given, so without reading the face back first this
/// request would blank the kind, the exports, and everything else it stayed silent
/// about — and the kernel would either reject it or, worse, accept a face the author
/// never wrote.
/// 局部编辑保留请求没有点名的每个字段。执行器用拿到的取值整体重建一个面，因此若不先读回该面，这次
/// 请求就会抹掉 kind、exports 以及它未提及的一切——内核要么拒绝，要么更糟：接受一个作者从未写过的面。
#[test]
fn a_partial_edit_keeps_the_fields_it_does_not_name() {
    let (root, name) = package("partial-edit");
    let created = apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "apply": true,
            "fields": {
                "module": "button",
                "kind": "Button",
                "name_en": "Button",
                "exports": "root.render",
                "handle_contracts": "ControlHandle",
            },
        }),
    )
    .expect("the face is created");
    let file = written(&created);
    let before = std::fs::read_to_string(&file).expect("the written face");

    let edited = apply(
        &root,
        &json!({
            "action": "edit",
            "node": "root/button",
            "apply": true,
            "fields": {"name_en": "Push button"},
        }),
    )
    .expect("only one field is named");
    let after = std::fs::read_to_string(written(&edited)).expect("the edited face");
    assert!(after.contains("en: \"Push button\""), "{after}");
    assert!(
        after.contains("kind: Button"),
        "the kind the request did not name must survive: {after}"
    );
    assert!(
        after.contains("ControlHandle"),
        "the contract the request did not name must survive: {after}"
    );
    assert_ne!(before, after, "the named field did change");
    let faces = face_views(&root, &name).expect("the project reads");
    assert_eq!(faces.len(), 1, "{faces:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A rename moves the module and keeps the face. Its preview shows both halves —
/// the file that goes and the file that arrives — because a rename that only
/// reported the new file would hide the removal from the caller confirming it.
/// 改名搬走模块并保留这个面。它的预览把两半都显示出来——走掉的文件与到来的文件——因为只报告新文件的
/// 改名会把删除这一半藏起来，而调用方正要据此确认。
#[test]
fn a_rename_moves_the_module_and_keeps_the_face() {
    let (root, name) = package("rename");
    apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "apply": true,
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the face is created");

    let preview = apply(
        &root,
        &json!({
            "action": "rename",
            "node": "root/button",
            "fields": {"module": "dial"},
        }),
    )
    .expect("the rename previews");
    assert!(preview.contains("- src/button/button.rs"), "{preview}");
    assert!(preview.contains("+ src/dial/dial.rs"), "{preview}");
    assert!(preview.contains("root/dial"), "{preview}");
    assert!(
        root.join("src/button/button.rs").is_file(),
        "the preview must not move anything: {preview}"
    );

    let applied = apply(
        &root,
        &json!({
            "action": "rename",
            "node": "root/button",
            "apply": true,
            "fields": {"module": "dial"},
        }),
    )
    .expect("the rename is applied");
    assert!(applied.contains("root/dial"), "{applied}");
    assert!(root.join("src/dial/dial.rs").is_file(), "{applied}");
    assert!(!root.join("src/button/button.rs").exists(), "{applied}");
    let faces = face_views(&root, &name).expect("the project reads");
    assert_eq!(faces.len(), 1, "{faces:?}");
    assert_eq!(faces[0].path, "root/dial");
    assert_eq!(faces[0].kind, "Button", "the face itself is unchanged");
    let _ = std::fs::remove_dir_all(&root);
}

/// A delete preview removes nothing, and the applied delete moves the module out of
/// the tree. It is recoverable by design — the executor moves the directory into
/// NichLink's trash rather than unlinking it — which is why the request can be
/// confirmed rather than merely trusted.
/// 删除预览不搬走任何东西，而落盘的删除把模块移出树。它按设计可恢复——执行器把目录移进 NichLink
/// 的回收目录而不是删掉——这正是这次请求可以被"确认"而不只是被信任的原因。
#[test]
fn a_delete_removes_nothing_until_it_is_applied() {
    let (root, name) = package("delete");
    apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "apply": true,
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the face is created");

    let preview = apply(&root, &json!({"action": "delete", "node": "root/button"}))
        .expect("the delete previews");
    assert!(preview.contains("faces 0"), "{preview}");
    assert!(preview.contains("- src/button/button.rs"), "{preview}");
    assert!(
        root.join("src/button/button.rs").is_file(),
        "a preview must not delete anything: {preview}"
    );

    let applied = apply(
        &root,
        &json!({"action": "delete", "node": "root/button", "apply": true}),
    )
    .expect("the delete is applied");
    assert!(applied.contains("faces 0"), "{applied}");
    assert!(!root.join("src/button/button.rs").exists(), "{applied}");
    assert!(
        face_views(&root, &name)
            .expect("the project reads")
            .is_empty(),
        "the face is gone from the tree"
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
/// guessed at; `graft` and the rest of the production chain are still to come.
/// 工具没有实现的动作按名字被拒，而不是被猜着执行；`graft` 与生产链路其余部分仍待做。
#[test]
fn an_action_that_is_not_implemented_is_refused_by_name() {
    let (root, _) = package("unsupported");
    let error = apply(&root, &json!({"action": "graft", "node": "root/button"}))
        .expect_err("graft is not implemented yet");
    assert!(error.contains("not implemented"), "{error}");
    let _ = std::fs::remove_dir_all(&root);
}
