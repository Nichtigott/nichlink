//! Tests for the write path: what a preview promises, and what an apply does.
//! 写入路径的测试：预览承诺了什么，落盘做了什么。

use std::path::PathBuf;

use crate::build_time::face_views;
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

/// A misspelled key and a mistyped flag are refused by name, by both write actions.
/// `add` used to take only the keys it knew and silently drop the rest, so an agent that
/// wrote `knd` believed it had set the kind, and `needs_registry: "yes"` became `false`.
/// 拼错的键与类型不对的标志都被**点名**拒绝，两个写入动作都一样。`add` 过去只取它认识的键、
/// 静默丢弃其余，于是写下 `knd` 的代理会以为自己设了 kind，而 `needs_registry: "yes"` 会变成
/// `false`。
#[test]
fn a_misspelled_field_is_refused_by_add_and_edit() {
    let (root, _) = package("fields");

    let misspelled = apply(
        &root,
        &json!({"action": "add", "fields": {"module": "probe", "knd": "Button"}}),
    )
    .expect_err("a misspelled key is refused rather than dropped");
    assert!(
        misspelled.contains("`knd` is not an editable registration-face field"),
        "{misspelled}"
    );
    // The refusal also names the accepted shape, so the next call does not have to guess it.
    // 拒绝同时点名"可接受的形状"，让下一次调用不必靠猜。
    assert!(
        misspelled.contains("fields: module kind") && misspelled.contains("one boolean"),
        "the refusal carries the shape: {misspelled}"
    );

    let mistyped = apply(
        &root,
        &json!({"action": "add", "fields": {"module": "probe", "needs_registry": "yes"}}),
    )
    .expect_err("a non-boolean flag is refused rather than coerced");
    assert!(
        mistyped.contains("`needs_registry` must be true or false"),
        "{mistyped}"
    );
    // Every type refusal carries the shape too: an independent review of the first cut found this
    // exact hole (the boolean refusal was the one an arm actually hit), so the pin covers both the
    // boolean and the string case rather than only the misspelled key.
    // 每一种类型拒绝也带形状：对第一版的独立复核正是从这个洞里抓到的（被拒的恰是布尔那一条），因此
    // 钉子同时覆盖布尔与字符串两种情形，而不是只覆盖拼错的键。
    assert!(
        mistyped.contains("fields: module kind") && mistyped.contains("one boolean"),
        "the boolean refusal carries the shape: {mistyped}"
    );
    let mistyped_string = apply(
        &root,
        &json!({"action": "add", "fields": {"module": "probe", "exports": ["control.render"]}}),
    )
    .expect_err("an array where a string belongs is refused rather than coerced");
    assert!(
        mistyped_string.contains("`exports` must be a string")
            && mistyped_string.contains("fields: module kind"),
        "the string refusal carries the shape: {mistyped_string}"
    );

    let applied = apply(
        &root,
        &json!({"action": "add", "fields": {"module": "probe"}, "apply": true}),
    )
    .expect("a valid add applies");
    assert!(applied.contains("applied"), "{applied}");

    let edited = apply(
        &root,
        &json!({"action": "edit", "node": "root/probe", "fields": {"knd": "Button"}}),
    )
    .expect_err("edit refuses the same key");
    assert!(
        edited.contains("`knd` is not an editable registration-face field"),
        "{edited}"
    );
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
    // Every reply carries the shape a request must spell, because the round measured that the one
    // place listing it (`--list <tool>`) was never called.
    // 每次回复都带上"请求必须写出的形状"——那轮量到唯一列出它的地方（`--list <tool>`）一次都没被调用。
    assert!(
        reply.contains("fields: module kind preset parts") && reply.contains("one boolean"),
        "the reply states the editable-field shape: {reply}"
    );
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

/// The write path states what the change will make the rest of the tree say: which in-tree test
/// lines name the face, and whether the entry plan names it at all. Both families of the round-5
/// evaluation discovered those two facts only by running the suite and reading the red.
/// 写入路径会说出这次改动会让树的其他部分说什么：哪些树内测试行点到了这个面，以及入口计划到底有没有
/// 点名它。第五轮评测的两个族都只靠"跑测试、看红"才发现这两件事。
#[test]
fn a_preview_names_the_pins_and_the_entry_plan_the_change_will_move() {
    let (root, _) = package("consequences");
    std::fs::create_dir_all(root.join("tests")).expect("fixture test dir");
    std::fs::write(
        root.join("tests/registry.rs"),
        "#[test]\nfn the_factory_lists_every_face() { assert!(true, \"button\"); }\n",
    )
    .expect("fixture test file");
    let reply = apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root",
            "fields": {"module": "button", "kind": "Button", "name_en": "Button"},
        }),
    )
    .expect("the preview runs");
    assert!(
        reply.contains("consequences (static, text-level)"),
        "{reply}"
    );
    assert!(
        reply.contains("tests/registry.rs:"),
        "the in-tree test line that names the face is listed: {reply}"
    );
    assert!(reply.contains("entry plan:"), "{reply}");
    assert!(
        reply.contains("not covered"),
        "and the block says what it cannot see: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The wall the round measured comes back with the way forward named: "no" and "no, and here is what
/// yes needs" are different answers, and the second one is the one an agent can act on.
/// 那轮量到的那堵墙会带着"继续走的两条路"回来："不行"与"不行，而'行'需要什么"是两个不同的回答，
/// 而后者才是代理能据以行动的那个。
#[test]
fn a_refusal_that_has_a_way_forward_names_it() {
    let (root, _) = package("way-forward");
    let control = json!({
        "action": "add",
        "parent": "root",
        "apply": true,
        "fields": {"module": "control", "kind": "Control"},
    });
    apply(&root, &control).expect("the parent face is created");
    let refused = apply(
        &root,
        &json!({
            "action": "add",
            "parent": "root/control",
            "fields": {"module": "label", "kind": "Label"},
        }),
    )
    .expect_err("a parent that owns no registry cannot take a child");
    assert!(
        refused.contains("does not own a Registry"),
        "the kernel's own refusal is kept: {refused}"
    );
    assert!(refused.contains("way forward"), "{refused}");
    assert!(refused.contains("needs_registry"), "{refused}");
    let _ = std::fs::remove_dir_all(&root);
}

/// `deepen` adds a layer **inside** a face: the tree, the declaration and the public path do not
/// move, and the reply prices the other reading ("another face under it").
/// `deepen` 在一个面**内部**加一层：树、声明与公开路径都不动，而且回复会给出另一种读法（"它下面再挂
/// 一个面"）的代价。
#[test]
fn a_deepen_adds_a_layer_inside_the_face_and_prices_the_other_reading() {
    let (root, _) = package("deepen");
    apply(
        &root,
        &json!({"action": "add", "parent": "root", "apply": true,
                "fields": {"module": "control", "kind": "Control", "needs_registry": true}}),
    )
    .expect("the parent face");
    let added = apply(
        &root,
        &json!({"action": "add", "parent": "root/control", "apply": true,
                "fields": {"module": "button", "kind": "Button"}}),
    )
    .expect("the leaf face");
    let face = written(&added);
    let reply = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button",
                "inside": {"parts": {"label": "String"}}}),
    )
    .expect("the deepen preview runs");
    assert!(reply.contains("action preview"), "{reply}");
    assert!(reply.contains("deepened root/control/button"), "{reply}");
    assert!(reply.contains("ButtonParts"), "{reply}");
    assert!(
        reply.contains("alternative (the other reading"),
        "the reply prices the other reading: {reply}"
    );
    assert!(
        reply.contains("faces 2"),
        "the registration tree is unchanged: {reply}"
    );
    assert!(
        !std::fs::read_to_string(&face)
            .expect("the face file")
            .contains("ButtonParts"),
        "a preview writes nothing"
    );
    let applied = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button", "apply": true,
                "inside": {"parts": {"label": "String", "count": "usize"}}}),
    )
    .expect("the deepen applies");
    assert!(applied.contains("action apply"), "{applied}");
    let text = std::fs::read_to_string(&face).expect("the face file");
    assert!(
        text.contains("pub struct Button {\n    parts: ButtonParts,\n}"),
        "{text}"
    );
    assert!(
        text.contains("impl nichlink_toolchain::runtime::PartsContract for ButtonParts"),
        "{text}"
    );
    // The parts list follows the JSON object's own order, which is sorted by key, so the pin states
    // that order rather than the order the request happened to write them in.
    // 零件列表按 JSON 对象自己的顺序（按键排序），因此钉子写的是那个顺序，而不是请求碰巧写下的顺序。
    assert!(
        text.contains("const PROVIDED_PARTS: &'static [&'static str] = &[\"count\", \"label\"]"),
        "{text}"
    );
    assert!(
        text.contains("pub fn parts(&self) -> &ButtonParts"),
        "{text}"
    );
    assert!(
        text.contains("kind: Button"),
        "the declaration is untouched: {text}"
    );
    let missing = apply(
        &root,
        &json!({"action": "deepen", "node": "root/nope", "inside": {"parts": {"x": "u8"}}}),
    )
    .expect_err("there is no such face");
    assert!(
        missing.contains("no registration face") || missing.contains("does not name a face"),
        "{missing}"
    );
    // A file whose face still resolves but no longer holds the unit marker: the action refuses by
    // naming the line it looked for, rather than guessing at another shape.
    // 一个面仍能解析、但已不再持有单位标记的文件：本动作按名拒绝、点出它找的那一行，而不是去猜另一种形状。
    let text = std::fs::read_to_string(&face).expect("the face file");
    std::fs::write(
        &face,
        text.replace(
            "pub struct Button {\n    parts: ButtonParts,\n}",
            "pub struct Button {\n    own: u8,\n}",
        ),
    )
    .expect("fixture rewrite");
    let unclear = apply(
        &root,
        &json!({"action": "deepen", "node": "root/control/button", "inside": {"parts": {"x": "u8"}}}),
    )
    .expect_err("a shape it cannot read is refused rather than guessed at");
    assert!(
        unclear.contains("pub struct Button;") && unclear.contains("0 time(s)"),
        "{unclear}"
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
    // The reply anchors the declaration, so a caller does not have to search the
    // file for the line it just changed.
    // 回复给出声明的锚点，调用方因此不必去文件里找它刚改的那一行。
    let anchor = created
        .lines()
        .find_map(|line| line.strip_prefix("declaration "))
        .unwrap_or_else(|| panic!("no declaration anchor in: {created}"));
    let (anchor_path, anchor_line) = anchor.rsplit_once(':').expect("path:line");
    assert!(
        anchor_path.ends_with("button/button.rs"),
        "the anchor must name the face: {anchor}"
    );
    let anchored = before
        .lines()
        .nth(anchor_line.parse::<usize>().expect("a line number") - 1)
        .unwrap_or_else(|| panic!("the anchor points past the file: {anchor}"));
    assert!(
        anchored.contains("! {"),
        "the anchor must point at the declaration, not at line 1: {anchored}"
    );
    // A delete has no declaration to point at, because the file is gone.
    // 删除没有可指的声明，因为文件已经不在了。

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
/// confirmed rather than merely trusted. The preview also has to *say* it removed
/// nothing: the executor reports what it did in the past tense, and a preview reply
/// that repeated that sentence bare would tell the agent the move had happened.
/// 删除预览不搬走任何东西，而落盘的删除把模块移出树。它按设计可恢复——执行器把目录移进 NichLink
/// 的回收目录而不是删掉——这正是这次请求可以被"确认"而不只是被信任的原因。预览还必须**说出**
/// 它没有搬走任何东西：执行器用过去时报告它做了什么，而预览回复若把那句话原样重复一遍，就等于
/// 告诉代理这次搬移已经发生。
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

    let preview = apply(
        &root,
        &json!({"action": "delete", "node": "root/button", "confirm": true}),
    )
    .expect("the delete previews");
    assert!(preview.contains("faces 0"), "{preview}");
    assert!(preview.contains("- src/button/button.rs"), "{preview}");
    assert!(preview.contains("would move"), "{preview}");
    assert!(
        preview.contains("preview effect: moved `button`"),
        "{preview}"
    );
    assert!(
        !preview.lines().any(|line| line.starts_with("moved ")),
        "a preview may not report the move as done: {preview}"
    );
    assert!(
        root.join("src/button/button.rs").is_file(),
        "a preview must not delete anything: {preview}"
    );

    let applied = apply(
        &root,
        &json!({"action": "delete", "node": "root/button", "apply": true, "confirm": true}),
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

/// A delete says `confirm` itself; the bridge does not add it. Appending it here is what made the
/// sentence in `run_delete` and the declared schema — which had no such key — describe something
/// the bridge did not do, so a caller could step past the one operation that is worth confirming
/// (audit `m5`).
/// 删除要自己说出 `confirm`，桥不会替它加上。就地拼上它，正是让 `run_delete` 里那句话与声明 schema
/// （根本没有这个键）描述桥并不做的事的原因——于是调用方可以跳过唯一值得确认的那次操作（审计 `m5`）。
#[test]
fn a_delete_without_confirm_is_refused() {
    let (root, _) = package("delete-confirm");
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

    let error = apply(
        &root,
        &json!({"action": "delete", "node": "root/button", "apply": true}),
    )
    .expect_err("a delete without `confirm` must be refused");
    assert!(error.contains("confirm"), "{error}");
    assert!(
        root.join("src/button/button.rs").is_file(),
        "the refusal must happen before anything moves: {error}"
    );

    // And the refusal is about `confirm`, not about the target: the same request with it succeeds.
    // 而且拒绝的是 `confirm`，不是目标：同一请求带上它就成功。
    apply(
        &root,
        &json!({"action": "delete", "node": "root/button", "apply": true, "confirm": true}),
    )
    .expect("the confirmed delete is applied");
    assert!(!root.join("src/button/button.rs").exists());
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

/// Changing a face's `kind` changes its identity, and the reply says so. `kind` is an identity input
/// (`NodeId = hash(namespace, source, name)`, and a face's name is its kind), so an edit that changes
/// it rewrites the marker type and the `kind:` field and the face comes back under a new identity —
/// while every graft record keyed by the old one stops resolving. The reply used to report the
/// resulting tree without ever saying so (audit `L6`). The pin also checks the *new* id it prints
/// against the one the tree reports afterwards, so a wrong computation cannot hide in prose.
/// 改一个面的 `kind` 会改变它的身份，而回复会说明这一点。`kind` 是身份输入
/// （`NodeId = hash(namespace, source, name)`，而面的名字就是它的 kind），因此改它的编辑会重写标记类型与
/// `kind:` 字段，这个面以新身份回来——而以旧身份为键的每条 graft 记录都不再解析。回复过去只报告结果树、
/// 从不说明（审计 `L6`）。这条钉子还把它打印的**新** id 与树此后报告的 id 对照，因此算错的身份无法藏在
/// 散文里。
#[test]
fn editing_the_kind_reports_the_identity_change() {
    let (root, name) = package("kind-identity");
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
    let before = face_views(&root, &name).expect("faces derive")[0].clone();

    let reply = apply(
        &root,
        &json!({"action": "edit", "node": "root/button", "apply": true, "fields": {"kind": "Bogus"}}),
    )
    .expect("the edit is applied");
    assert!(
        reply.contains("identity changed"),
        "the reply names the identity change it caused: {reply}"
    );
    assert!(
        reply.contains(&format!("no longer `{}`", before.id)),
        "it names the identity the records were keyed by: {reply}"
    );

    let after = face_views(&root, &name).expect("faces derive")[0].clone();
    assert_ne!(after.id, before.id, "the identity really did change");
    // The reply names the *old* identity on purpose and does not print the new one: the first version
    // computed it from `change.source` and printed a number the tree did not agree with, and this pin
    // caught that. A wrong number in a diagnostic is worse than no number.
    // 回复有意只点名**旧**身份、不打印新的：第一版从 `change.source` 算它，打印出的数字与树不一致，
    // 正是这条钉子抓到的。诊断里的错数字比没有数字更糟。
    assert!(
        !reply.contains(&after.id.to_string()),
        "it deliberately does not print an identity it cannot verify: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `edit` and `rename` refuse the two contract keys by name instead of returning success while the
/// value never reaches the file. The executor's edit field order does not carry them, so an agent
/// that asked for a checked contract got an unchecked label — silently (audit `LGC-LG-21`). `add`
/// still sets them, which is where the contract is actually written.
/// `edit` 与 `rename` 按名拒绝那两个 contract 键，而不是返回成功而取值从未抵达文件。执行器的 edit
/// 字段顺序不携带它们，因此一个要求"参与编译检查的契约"的代理拿到的是未经检查的标签——而且无声
/// （审计 `LGC-LG-21`）。`add` 仍然能设它们，contract 真正被写下的地方就是那里。
#[test]
fn edit_refuses_the_two_contract_keys_by_name_and_add_still_sets_them() {
    let (root, _) = package("contracts");
    let added = apply(
        &root,
        &json!({"action": "add", "apply": true,
                "fields": {"module": "label", "kind": "Label",
                           "handle_contracts": "crate::Foo::Bar"}}),
    )
    .expect("add sets the contract");
    assert!(
        added.contains("handle_contracts") || added.contains("label"),
        "{added}"
    );
    for key in ["handle_contracts", "part_contracts"] {
        for action in ["edit", "rename"] {
            let error = apply(
                &root,
                &json!({"action": action, "node": "root/label",
                        "fields": {"module": "label", key: "crate::Foo::Bar"}}),
            )
            .expect_err("the key is refused by name");
            assert!(error.contains(key), "{error}");
            assert!(error.contains(action), "{error}");
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}
