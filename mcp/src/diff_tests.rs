//! Tests for the tree diff: a match, an addition, a removal, and a face that
//! changed identity under a file that did not move.
//! 树 diff 的测试：匹配、新增、移除，以及文件没动而身份变了的面。

use std::path::{Path, PathBuf};

use nichlink_build_method::face_views;
use serde_json::json;

use super::diff;

/// A throwaway package with one hand-written root face.
/// 一个含一个手写根面的一次性包。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-diff-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src/button")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    std::fs::write(
        root.join("src/button/button.rs"),
        "pub struct Button;\n\ncrate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    )
    .expect("face");
    (root, name)
}

/// Publish a build manifest with exactly the rows given.
/// 发布一份只含给定行的构建清单。
fn publish(root: &Path, rows: &[String]) {
    let out = root.join("target/nichlink/out");
    std::fs::create_dir_all(&out).expect("build output");
    let mut text = String::from("# node\tsource\tsymbol\n");
    for row in rows {
        text.push_str(row);
        text.push('\n');
    }
    std::fs::write(out.join("pruning_manifest.tsv"), text).expect("pruning manifest");
}

/// A manifest matching the sources is reported as a match, not as an empty diff.
/// 与源码一致的清单被报成一致，而不是一份空 diff。
#[test]
fn a_matching_build_reports_no_delta() {
    let (root, name) = package("match");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    publish(&root, &[format!("{}\t{}\t-", face.id, face.source)]);
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("added 0  gone 0  reidentified 0"), "{reply}");
    assert!(reply.contains("face for face"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A face the build never saw is added; a face it saw and the sources no longer
/// declare is gone.
/// 构建从未见过的面是新增；构建见过而源码不再声明的是移除。
#[test]
fn an_addition_and_a_removal_are_both_named() {
    let (root, name) = package("delta");
    let old = nichlink::identity::NodeId::from_namespaced_path(&name, "old/old.rs", "Old");
    publish(&root, &[format!("{old}\told/old.rs\t-")]);
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("added 1  gone 1"), "{reply}");
    assert!(reply.contains("+ root/button"), "{reply}");
    assert!(reply.contains("- old/old.rs"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The same source with a different identity is the one case a text diff would
/// miss entirely.
/// 同一份源码、不同的身份，正是文本 diff 完全看不见的那种情况。
#[test]
fn a_changed_identity_under_an_unmoved_file_is_reported() {
    let (root, name) = package("reidentified");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    let renamed = nichlink::identity::NodeId::from_namespaced_path(&name, &face.source, "Renamed");
    publish(&root, &[format!("{renamed}\t{}\t-", face.source)]);
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("reidentified 1"), "{reply}");
    assert!(reply.contains("~ root/button"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Without a build the answer names the command that produces one.
/// 没有构建时，答案点名产出它的命令。
#[test]
fn a_missing_build_asks_for_one_instead_of_diffing_nothing() {
    let (root, _) = package("no-build");
    let reply = diff(&root, &json!({})).expect("the diff renders");
    assert!(reply.contains("no build evidence"), "{reply}");
    assert!(reply.contains("nichlink check"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Write one external graft record under `.nichlink/external-grafts/<selector>/graft.plan`.
/// 在 `.nichlink/external-grafts/<selector>/graft.plan` 下写一条外部 graft 记录。
fn record(root: &Path, selector: &str, target: nichlink::identity::NodeId, path: &str) {
    let directory = root.join(".nichlink/external-grafts").join(selector);
    std::fs::create_dir_all(&directory).expect("record directory");
    std::fs::write(
        directory.join("graft.plan"),
        format!("version=1\ntarget={target}\ntarget_path={path}\ngraft={selector}\nfull=false\n"),
    )
    .expect("record file");
}

/// The record side: a record whose identity the tree still has is `ok`, one whose slot moved
/// identity is `re-identified`, and one whose slot is gone is `stale`. There is no `unmatched`
/// bucket: it existed for "the identity is absent but the plan's path looks like a Rust expression
/// (`::`)", and no plan writer produces that — both write `registry.path_for` — while a typed
/// *declaration* cannot be matched at all when the identity is absent (nothing resolves its module,
/// so `names_face` admits only string cuts there). The bucket described an input that cannot occur
/// and its heading claimed a real case; a record naming something this tree has not got is `stale`,
/// and the reply prints the path it looked for (audit `m1`). The third record below is the old
/// fixture for that bucket, kept as a `stale` row on purpose: it is what the fabricated input now
/// honestly is.
/// 记录那一侧：身份仍在树里的记录是 `ok`；槽位换了身份的是 `re-identified`；槽位消失的是 `stale`。
/// 没有 `unmatched` 桶：它是为"身份缺席、而计划的路径看起来像 Rust 表达式（`::`）"而设的，而没有
/// 任何计划写入方会产出那种东西——两处都写 `registry.path_for`——而类型化的**声明**在身份缺席时
/// 根本匹配不上（没有任何东西能解析它的模块，因此 `names_face` 在那种情况下只接受字符串切口）。那个
/// 桶描述的是一个不可能出现的输入，标题却声称描述真实情形；点名了本树没有的东西的记录就是 `stale`，
/// 回复会打印它查找过的路径（审计 `m1`）。下面第三条记录就是那个桶的旧夹具，有意保留为 `stale`
/// 行：那个伪造输入如今诚实地就是这个。
#[test]
fn the_record_side_tells_a_stale_record_from_a_re_identified_one() {
    let (root, name) = package("records");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    let stale_identity =
        nichlink::identity::NodeId::from_namespaced_path(&name, &face.source, "Renamed");
    // `kept_fast`'s slot is named by the host entry, which is what makes it `ok` rather than
    // `undeclared`: a record no cut names is pruned by the release.
    // `kept_fast` 的槽位由宿主入口点名，这正是它成为 `ok` 而不是 `undeclared` 的原因：没有任何切口
    // 点名的记录会被发布剪掉。
    std::fs::write(
        root.join("src/lib.rs"),
        "// host entry\nnichlink_run_method::static_graft_plan!(FRAMEWORK, cut \"root/button\" graft \"kept_fast\");\n",
    )
    .expect("host entry");
    record(&root, "kept_fast", face.id, "root/button");
    record(&root, "moved_fast", stale_identity, "root/button");
    record(&root, "ghost_fast", stale_identity, "root/ghost");
    record(
        &root,
        "typed_fast",
        stale_identity,
        "crate::control::NODE_ID",
    );
    let reply = diff(&root, &json!({"records": true})).expect("the diff renders");
    assert!(
        reply.contains("records 4 (external graft plans)"),
        "{reply}"
    );
    assert!(
        reply.contains("ok 1  undeclared 0  stale 2  re-identified 1  unreadable 0"),
        "{reply}"
    );
    assert!(
        !reply.contains("unmatched"),
        "the bucket that described an impossible input is gone: {reply}"
    );
    assert!(reply.contains("kept_fast -> root/button"), "{reply}");
    assert!(
        reply.contains(
            "- ghost_fast -> root/ghost (no face in this tree has that identity or that path)"
        ),
        "{reply}"
    );
    assert!(
        reply.contains(&format!("~ moved_fast {stale_identity} -> {}", face.id)),
        "the record that moved identity names the identity the tree has now: {reply}"
    );
    assert!(
        reply.contains("- typed_fast -> crate::control::NODE_ID"),
        "a path that names nothing in this tree is stale, and the reply prints the path it looked \
         for: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A record whose identity the tree has is not `ok` unless a `static_graft_plan!` cut names its
/// slot. `diff {"records":true}` used to read only the identity and report `ok`, while
/// `nichlink.grafts` reported the same row as `[NOT declared by the host entry]`, counted it under
/// `unkept plans N`, and the build refused it outright: one record, two health conclusions, and the
/// agent that read the first one would ship it (audit `M1`). The row already carried
/// `declared`, so the split costs nothing.
/// 身份在树里的记录，只有在某条 `static_graft_plan!` 切口点名它的槽位时才是 `ok`。
/// `diff {"records":true}` 过去只读身份就报 `ok`，而 `nichlink.grafts` 把同一行报成
/// `[NOT declared by the host entry]`、计入 `unkept plans N`，构建还直接拒绝它：同一条记录、两个健康
/// 结论，而读到前者的代理会把它发出去（审计 `M1`）。该行本就携带 `declared`，分流不需额外代价。
#[test]
fn an_undeclared_record_is_not_reported_as_ok() {
    let (root, name) = package("undeclared");
    let face = face_views(&root, &name).expect("faces derive")[0].clone();
    // The identity is present in the tree; no host entry names this slot.
    // 身份在树里；没有任何宿主入口点名这个槽位。
    record(&root, "orphan_fast", face.id, "root/button");
    let reply = diff(&root, &json!({"records": true})).expect("the diff renders");
    assert!(
        reply.contains("ok 0  undeclared 1  stale 0  re-identified 0  unreadable 0"),
        "an undeclared record is not ok: {reply}"
    );
    assert!(
        !reply.contains("  orphan_fast -> root/button"),
        "it must not be listed under `ok:` either: {reply}"
    );
    assert!(
        reply.contains("! orphan_fast -> root/button  (no cut in the host entry names it)"),
        "and the reply says which case it is: {reply}"
    );
    assert!(
        reply.contains("the release prunes these slots"),
        "with the consequence, the same one `nichlink.grafts` reports: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An unreadable record is counted rather than dropped, and a package with no records says
/// `records 0` instead of claiming a clean bill of health on nothing.
/// 读不了的记录会被计数而不是被丢掉；没有记录的包会说 `records 0`，而不是在没有东西时声称一切正常。
#[test]
fn an_unreadable_record_is_counted_and_no_records_is_not_a_verdict() {
    let (root, _) = package("records-unreadable");
    let directory = root.join(".nichlink/external-grafts/broken_fast");
    std::fs::create_dir_all(&directory).expect("record directory");
    std::fs::write(directory.join("graft.plan"), "version=9\n").expect("record file");
    let reply = diff(&root, &json!({"records": true})).expect("the diff renders");
    assert!(reply.contains("unreadable 1"), "{reply}");
    assert!(
        reply.contains("ok 0  undeclared 0  stale 0  re-identified 0  unreadable 1"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);

    let (bare, _) = package("records-none");
    let empty = diff(&bare, &json!({"records": true})).expect("the diff renders");
    assert!(
        empty.contains("records 0 (external graft plans)"),
        "{empty}"
    );
    let _ = std::fs::remove_dir_all(&bare);
}
