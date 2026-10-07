//! Pins for `promote`: what it refuses, and the two pieces of logic the landing rests on.
//! `promote` 的钉子：它拒绝什么，以及落地所依赖的那两处逻辑。
//!
//! The **full path** (build a host with `new_project --faces`, graft an external crate, write the
//! record, promote, and `cargo check` the landed tree) needs a cargo run at every step, so it is
//! run as an end-to-end demonstration rather than as an in-crate test — the transcript is in the
//! commit that introduced this action. What lives here is what that demonstration cannot keep
//! honest on its own: the refusals, the declaration→patch mapping, and the entry surgery.
//! **完整路径**（用 `new_project --faces` 建宿主、嫁接一个外部 crate、写记录、promote、再对落地的树
//! `cargo check`）每一步都要跑 cargo，因此它作为端到端演示运行，而不是仓库内测试——过程记录在引入本动作
//! 的那笔提交里。住在这里的是那场演示自己守不住的东西：拒绝族、声明到 patch 的映射、以及条目手术。

use serde_json::json;

use super::source::{External, external_file};
use super::{repoint_graft, run_promote};

/// The refusal text of one call, without asking the shared `Outcome` to be `Debug`.
/// 一次调用的拒绝文本；不去要求共享的 `Outcome` 实现 `Debug`。
fn refusal(result: Result<super::Outcome, String>) -> String {
    match result {
        Ok(_) => panic!("this request was expected to be refused"),
        Err(refused) => refused,
    }
}

/// A throwaway package root.
/// 一个一次性包根。
fn scratch(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-promote-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("fixture root");
    root
}

/// A request without `selector` is refused with the shape to paste.
/// 不带 `selector` 的请求被拒绝，并给出可粘贴的形状。
#[test]
fn a_request_without_a_selector_names_the_shape() {
    let root = scratch("selector");
    let refused = refusal(run_promote(&root, &root, false, &json!({"confirm": true})));
    assert!(
        refused.contains("`selector`") && refused.contains("\"action\":\"promote\""),
        "the refusal names the key and the accepted shape: {refused}"
    );
    // The reverse direction: the shape sentence must not be the *only* thing said — a caller that
    // already passed a selector has to be refused for the real reason.
    // 反向：形状那句话不许是唯一被说的——已经给了 selector 的调用方必须因真正的原因被拒。
    assert!(
        !refused.contains("no such") && !refused.contains("No such file"),
        "the shape refusal is about the missing key, not about the disk: {refused}"
    );
}

/// `confirm` is the caller's own word, and it is required: this action rewrites source and retires
/// a record, and a preview is the only place to read what it would do.
/// `confirm` 是调用方自己要说的话，而且必须说：这个动作会改源码、退役记录，而预览是唯一读到它会做什么的地方。
#[test]
fn promote_requires_the_callers_own_confirm() {
    let root = scratch("confirm");
    let refused = refusal(run_promote(
        &root,
        &root,
        false,
        &json!({"selector": "button_fast"}),
    ));
    assert!(
        refused.contains("confirm: true") && refused.contains("preview"),
        "the refusal says why and where to look first: {refused}"
    );
}

/// A selector with no record under it is refused by naming the path it looked in.
/// 没有对应记录的 selector 会被拒绝，并点名它查找的路径。
#[test]
fn a_missing_record_is_refused_with_the_path_it_looked_in() {
    let root = scratch("record");
    std::fs::create_dir_all(root.join(".nichlink/external-grafts")).expect("records directory");
    let refused = refusal(run_promote(
        &root,
        &root,
        false,
        &json!({"selector": "button_fast", "confirm": true}),
    ));
    assert!(
        refused.contains(".nichlink/external-grafts/button_fast/graft.plan"),
        "the refusal names the file it wanted: {refused}"
    );
}

/// The face layout this repository's own generator writes is one `external_file` finds.
/// 本仓库自己的生成器写出的注册面版式，是 `external_file` 找得到的那一种。
///
/// The scaffold and `apply add` put a face whose module is `panel::object::frame` at
/// `src/panel/object/frame/frame.rs` — its own directory, named after itself — and the generated tree
/// mounts it from there. Only `src/<module>.rs` and `src/<module>/mod.rs` were searched before, so
/// `promote` refused every face this tool itself creates, about a file that was right there one
/// directory deeper (measured against a scaffolded host, audit `M7`, §M7.39).
/// 脚手架与 `apply add` 把模块为 `panel::object::frame` 的注册面放在
/// `src/panel/object/frame/frame.rs`——以自己的名字命名的目录里——生成树也从那里挂载它。此前只搜
/// `src/<模块>.rs` 与 `src/<模块>/mod.rs`，因此 `promote` 拒绝本工具自己创建的每一个注册面，而那个文件
/// 就在那里、只是深了一层（对照脚手架宿主实测，审计 `M7`，§M7.39）。
#[test]
fn the_generators_own_face_layout_is_found() {
    let root = scratch("layout");
    let generated = root.join("src/panel/object/frame/frame.rs");
    std::fs::create_dir_all(generated.parent().expect("a parent")).expect("face directory");
    std::fs::write(&generated, "// generated-by=NichLink\n").expect("the face");
    assert_eq!(
        external_file(&root, "panel::object::frame").expect("the generator's layout is found"),
        generated
    );

    // The single-segment case follows the same rule: module `fast` is `src/fast/fast.rs`.
    // 单段的情形同一条规则：模块 `fast` 就是 `src/fast/fast.rs`。
    let single = root.join("src/fast/fast.rs");
    std::fs::create_dir_all(single.parent().expect("a parent")).expect("face directory");
    std::fs::write(&single, "// generated-by=NichLink\n").expect("the face");
    assert_eq!(
        external_file(&root, "fast").expect("a one-segment module is found too"),
        single
    );

    // A crate that carries the flat spelling as well keeps answering what it answered before.
    // 同时带平坦拼写的 crate 仍给出与从前相同的答案。
    let flat = root.join("src/panel/object/frame.rs");
    std::fs::create_dir_all(flat.parent().expect("a parent")).expect("face directory");
    std::fs::write(&flat, "// flat\n").expect("the face");
    assert_eq!(
        external_file(&root, "panel::object::frame").expect("the flat form still wins"),
        flat
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The declaration→patch mapping: an external `external_object!` is read through the kernel's own
/// face parser, and the fields come out in the spellings a host declaration is written from.
/// 声明到 patch 的映射：外部的 `external_object!` 经内核自己的面解析器读取，字段以宿主声明的拼法出来。
#[test]
fn an_external_declaration_is_read_into_the_hosts_spellings() {
    let source = "\
nichlink_toolchain::runtime::external_object! {
    source: \"button_fast/button_fast.rs\",
    kind: ButtonFast,
    preset: NoPreset,
    parts: NoParts,
    name: { zh: \"快速按钮\", en: \"Fast button\" },
    summary: { zh: \"项目外实现\", en: \"Out-of-project implementation\" },
    exports: [\"control.render\"],
    needs_registry: false,
    parent: root_node_id(env!(\"CARGO_PKG_NAME\")),
    registry_rule: RegistrationRule::ANY,
    handle_traits: [\"ControlHandle\"],
    requires: [],
    provides: [],
    runtime_checks: [],
}
";
    let face = nichlink_kernel::syntax::parse_face(source)
        .expect("the declaration parses")
        .expect("it declares one face");
    let external = External::from_syntax(
        &face,
        std::path::Path::new("x.rs"),
        "control.render.v1|1|In|Out".to_owned(),
    )
    .expect("the mapping accepts an `ANY` rule");
    assert_eq!(external.kind, "ButtonFast");
    assert_eq!(external.name_zh, "快速按钮");
    assert_eq!(external.name_en, "Fast button");
    assert_eq!(external.exports, "control.render");
    assert_eq!(external.handle_traits, "ControlHandle");
    // The patch takes the compact rule spelling, not the Rust path the declaration writes.
    // patch 收的是紧凑的规则拼法，不是声明写下的 Rust 路径。
    assert_eq!(external.registration_rule, "ANY");
    assert_eq!(external.flow, "control.render.v1|1|In|Out");
    // An external-only field is not carried into the host's declaration at all.
    // 只属于外部的字段根本不会被带进宿主的声明。
    assert!(
        face.field("source").is_some(),
        "the external declaration does declare `source`, and this is the field the mapping leaves behind"
    );
}

/// A rule richer than `ANY` names a module inside the external crate, and a path field points at
/// *that* crate's files — both are refused by name rather than copied into a tree that lacks them.
/// 比 `ANY` 更复杂的规则命名的是外部 crate 内部的模块，而路径字段指向的是*那个* crate 的文件——两者都按名
/// 拒绝，而不是复制进一棵没有它们的树。
#[test]
fn a_path_that_belongs_to_the_external_crate_is_refused_by_name() {
    let rich = "\
external_object! {
    kind: ButtonFast,
    parent: root_node_id(env!(\"CARGO_PKG_NAME\")),
    registry_rule: RegistrationRule::preset(Preset::A),
}
";
    let face = nichlink_kernel::syntax::parse_face(rich)
        .expect("parses")
        .expect("one face");
    let refused = External::from_syntax(&face, std::path::Path::new("x.rs"), String::new())
        .expect_err("a rule module inside the external crate is refused");
    assert!(
        refused.contains("registry_rule") && refused.contains("ANY"),
        "the refusal names the field and what is supported: {refused}"
    );

    let admitted = "\
external_object! {
    kind: ButtonFast,
    parent: root_node_id(env!(\"CARGO_PKG_NAME\")),
    admission: allowed([\"crate::inner\"]),
}
";
    let face = nichlink_kernel::syntax::parse_face(admitted)
        .expect("parses")
        .expect("one face");
    let refused = External::from_syntax(&face, std::path::Path::new("x.rs"), String::new())
        .expect_err("an admission names paths inside the declaring crate");
    assert!(
        refused.contains("admission") && refused.contains("that"),
        "the refusal says whose paths those are: {refused}"
    );
}

/// Rewriting one declaration leaves the neighbours exactly as they were, and **keeps the entry**:
/// the plan's naming is what keeps a face in the generated tree, so this action never removes one.
/// 改写一条声明时邻居逐字不变，而且**条目保留**：计划的点名才是面留在生成树里的原因，因此这个动作从不删条目。
///
/// The location is found in the bytes rather than by line. That used to be forced by a defect
/// (`GraftSyntax::location` reported the macro's line for every entry); the defect is fixed
/// (2026-10-06, `ac96d69`: each entry now carries the line of its own `cut(…)`), and the byte search
/// stays because it is the stronger method — it does not care whether a location is right, and this
/// pin now checks that the entries report *their own* distinct lines instead of assuming they do not.
/// 位置在字节里找而不是按行。这过去是被一个缺陷逼的（`GraftSyntax::location` 给每条条目都报宏那一行）；
/// 缺陷已修（2026-10-06，`ac96d69`：每条条目现在携带它自己那条 `cut(…)` 的行），而按字节找保留下来，因为它
/// 是更强的方法——它不依赖位置对不对；本钉子现在改为断言各条目报**各自的**行，而不是假设它们报同一行。
#[test]
fn repointing_one_entry_keeps_the_others_as_they_were() {
    let source = "\
static_graft_plan!(
    FRAMEWORK,
    cut(crate::a::NODE_ID) graft(ext::a_fast::NODE_ID),
    cut(crate::b::NODE_ID)
        graft(ext::b_fast::NODE_ID),
    cut(crate::c::NODE_ID) graft(ext::c_fast::NODE_ID),
);
";
    let before = nichlink_kernel::syntax::entries::graft_entries(source).expect("parses");
    assert_eq!(before.len(), 3, "three entries to start");
    // The entries report their own lines (the fixture spreads them over lines 3–6), and the
    // neighbours' lines are what the rewrite must not disturb.
    // 各条目报自己的行（夹具把它们摊在第 3–6 行），而邻居的行正是这次改写不许扰动的东西。
    assert_eq!(
        before
            .iter()
            .map(|item| item.location.line)
            .collect::<Vec<_>>(),
        vec![3, 4, 6],
        "each entry carries the line of its own cut"
    );

    // The middle entry, spread over two lines: the neighbours must come back untouched.
    // 中间那条（摊在两行上）：邻居必须原样回来。
    let middle = repoint_graft(source, &before[1].cut, &before[1].graft).expect("the span closes");
    let after = nichlink_kernel::syntax::entries::graft_entries(&middle).expect("still parses");
    assert_eq!(after.len(), 3, "the entry count is unchanged: {middle}");
    assert_eq!(after[0].graft, "ext::a_fast::NODE_ID", "{middle}");
    assert_eq!(after[2].graft, "ext::c_fast::NODE_ID", "{middle}");
    assert_eq!(
        after[1].graft, before[1].cut,
        "the promoted entry names its own cut expression: {middle}"
    );
    assert!(
        !middle.contains("b_fast"),
        "the external implementation is gone from that entry: {middle}"
    );

    // The last entry: no next entry to bound anything, and no comma to take with it.
    // 最后一条：没有下一条可以框定边界，也没有逗号可以带走。
    let last =
        repoint_graft(&middle, &after[2].cut, &after[2].graft).expect("the last span closes");
    let rest = nichlink_kernel::syntax::entries::graft_entries(&last).expect("still parses");
    assert_eq!(rest.len(), 3, "still three entries: {last}");
    assert_eq!(rest[2].graft, rest[2].cut, "{last}");
    assert_eq!(rest[0].graft, "ext::a_fast::NODE_ID", "{last}");

    // The real plan's shape: two entries, the first one promoted, long expressions.
    // 真实计划的形状：两条目、改第一条、长表达式。
    let two = "\
static_graft_plan!(
    FRAMEWORK,
    cut(crate::button::NODE_ID) graft(control_button_graft::button_fast::NODE_ID),
    cut(crate::slider::NODE_ID) graft(control_button_graft::slider_fast::NODE_ID),
);
";
    let parsed = nichlink_kernel::syntax::entries::graft_entries(two).expect("parses");
    let first =
        repoint_graft(two, &parsed[0].cut, &parsed[0].graft).expect("the first span closes");
    let left = nichlink_kernel::syntax::entries::graft_entries(&first).expect("still parses");
    assert_eq!(left.len(), 2, "{first}");
    assert_eq!(left[0].graft, left[0].cut, "{first}");
    assert_eq!(
        left[1].graft, "control_button_graft::slider_fast::NODE_ID",
        "{first}"
    );
    assert!(!first.contains("button_fast"), "{first}");
}
