//! Pins for `nichlink.consistency`: the sibling set and the deviation rule, both decidable.
//! `nichlink.consistency` 的钉子：同族集合与离群判据，两者都可判定。
//!
//! The failure these guard is the one the capability answers: two siblings calling one family of
//! names while a third calls another, with every file locally plausible. What is pinned here is the
//! rule the answer turns on — a name most siblings call and this one does not, and a name it calls
//! that no sibling does — plus the shape of the refusals.
//! 这些钉子守的正是这条能力要回答的失败：两个兄弟调用同一族名字、第三个调用另一族，而每个文件单看都
//! 自洽。钉住的是答案所依赖的那条规则 —— "多数兄弟都调、而它没调的名字"与"它调了、别的兄弟都没调的名字"
//! —— 以及拒绝的形状。
//!
//! The `specimen` half has its own pins, on a package built here rather than described: the second
//! baseline is a ledger entry, not a majority, so the tree and the ledger both have to be real for
//! the comparison to be exercised at all.
//! `specimen` 那一半有它自己的钉子，钉在**这里搭出来**的包上而不是描述上：第二个基准是台账条目而不是
//! 多数派，因此要真的走到这次比较，树与台账都必须是真的。

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::json;

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn write_fixture(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().expect("parent")).expect("fixture dirs");
    std::fs::write(path, text).expect("fixture file");
}

/// A package with one root face and three children, plus a ledger certifying the first child.
/// 一个包：一个根面、三个子面，外加一份采信第一个子面的台账。
///
/// The layout is the one the build can compile (`<name>/<name>.rs`), because the derivation refuses
/// anything else and a fixture that is not what the tool reads would pin nothing. Each child is
/// given the same declared shape, so a pin can take one declaration away and watch exactly one row
/// appear.
/// 布局是构建编得过的那一种（`<name>/<name>.rs`），因为推导拒绝别的形状，而一个"不是工具所读的东西"
/// 的夹具什么都钉不住。每个子面拿到同样的已声明形状，因此钉子可以取走一条声明，看着恰好一行出现。
fn specimen_package(label: &str) -> PathBuf {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-consistency-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    write_fixture(
        &root.join("Cargo.toml"),
        &format!(
            "[package]\nname = \"fixture-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"
        ),
    );
    write_fixture(&root.join("src/lib.rs"), "//! fixture\npub mod control;\n");
    write_fixture(
        &root.join("src/control/control.rs"),
        "pub struct Control;\npub struct ControlParts;\n\ncrate::root_object! {\n    kind: \
         Control,\n    parts: ControlParts,\n    needs_registry: true,\n    parent: \
         crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    write_fixture(
        &root.join("src/control/object/object.rs"),
        "pub mod button;\npub mod slider;\npub mod timeline;\n",
    );
    for (name, kind) in [
        ("button", "Button"),
        ("slider", "Slider"),
        ("timeline", "Timeline"),
    ] {
        write_fixture(
            &root.join(format!("src/control/object/{name}/{name}.rs")),
            &format!(
                "pub struct {kind};\npub struct {kind}Parts;\n\ncrate::control_object! {{\n    \
                 kind: {kind},\n    parts: {kind}Parts,\n    exports: [\"control.render\"],\n    \
                 handle_traits: [\"ControlHandle\"],\n    parent: crate::control::NODE_ID,\n}}\n"
            ),
        );
    }
    root
}

/// Write a ledger whose one entry certifies the bytes `src/control/object/<name>/<name>.rs` has now.
/// 写一份台账，其中唯一一条条目针对 `src/control/object/<name>/<name>.rs` **此刻**的字节。
fn adopt(root: &Path, anchor: &str, file: &str) {
    let contents = std::fs::read_to_string(root.join(file)).expect("fixture source");
    let fingerprint =
        nichlink_kernel::adoption::adoption_fingerprint(&[(file.to_owned(), contents)]);
    write_fixture(
        &root.join(".nichlink/adopted/entries"),
        &format!(
            "{anchor}|the reference shape|traced once|nich|2026-10-01T10:00:00+08:00|{file}|\
             {fingerprint}|first adoption\n"
        ),
    );
}

/// Replace one line of a fixture face file, so a pin can take a declaration away and put it back.
/// 替换夹具面文件里的一行，使钉子能取走一条声明、再把它放回去。
fn replace(root: &Path, file: &str, from: &str, to: &str) {
    let path = root.join(file);
    let text = std::fs::read_to_string(&path).expect("fixture source");
    assert!(text.contains(from), "{file} must state `{from}`: {text}");
    std::fs::write(&path, text.replace(from, to)).expect("fixture write");
}

/// The specimen's shape is the ledger's baseline, and the sibling that lacks a declaration is named.
/// 标本的形状就是台账的基准，而缺了一条声明的兄弟被点名。
///
/// Both directions are walked: take a declaration away and exactly one row appears; put it back and
/// the answer is clean again. A comparison that could not go back would be a one-way assertion.
/// 两个方向都走：取走一条声明 ⇒ 恰好一行出现；放回去 ⇒ 答案重新干净。回不去的比较就是单向断言。
#[test]
fn the_specimen_comparison_names_the_declaration_a_sibling_lacks() {
    let root = specimen_package("specimen-lacks");
    adopt(
        &root,
        "root/control/button",
        "src/control/object/button/button.rs",
    );
    let file = "src/control/object/slider/slider.rs";

    let clean =
        super::consistency(&root, &json!({"specimen": "root/control/button"})).expect("an answer");
    assert!(
        clean.contains("conformance: 0 of 2 sibling(s)"),
        "the fixture starts unanimous: {clean}"
    );
    // The two declared shape facts differ per object on purpose (`parts` names the object's own
    // type), and the row must not read that as drift — that would make every family an outlier.
    // 两条已声明形状里有一条按对象不同是**故意的**（`parts` 点名对象自己的类型），而行不得把它读成漂移
    // ——那会让每一个同族都成了离群。
    assert!(
        !clean.contains("lacks `parts`") && !clean.contains("outlier"),
        "an object's own parts type is not a deviation: {clean}"
    );

    replace(&root, file, "    parts: SliderParts,\n", "");
    let missing =
        super::consistency(&root, &json!({"specimen": "root/control/button"})).expect("an answer");
    assert!(
        missing.contains("outlier     slider: lacks `parts`"),
        "the missing declaration is named, in the specimen's terms: {missing}"
    );
    assert!(
        missing.contains("conformance: 1 of 2 sibling(s)"),
        "exactly one sibling differs: {missing}"
    );

    let anchor = "root/control/button";
    assert!(
        missing.contains(anchor) && missing.contains("provisional"),
        "the ledger state rides with the comparison: {missing}"
    );

    replace(
        &root,
        file,
        "    exports: [\"control.render\"],\n",
        "    exports: [\"control.render\", \"control.own\"],\n",
    );
    let extra = super::consistency(&root, &json!({"specimen": anchor})).expect("an answer");
    assert!(
        extra.contains("conformance: 1 of 2 sibling(s)"),
        "a label the specimen does not state is that sibling's own business: {extra}"
    );

    replace(
        &root,
        file,
        "    exports: [\"control.render\", \"control.own\"],\n",
        "    parts: SliderParts,\n    exports: [\"control.render\"],\n",
    );
    let restored = super::consistency(&root, &json!({"specimen": anchor})).expect("an answer");
    assert!(
        restored.contains("conformance: 0 of 2 sibling(s)"),
        "putting the declaration back makes the answer clean again: {restored}"
    );
    assert!(
        restored.contains("not covered by this comparison") && restored.contains("next"),
        "the bounds and the next call ride along: {restored}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A declaration that cannot be read is a reason, never an empty shape.
/// 读不出来的声明给的是原因，绝不是一个空形状。
///
/// The two facts are different — "it states none of these fields" and "its declaration could not be
/// read" — and filling the second with the first is how a comparison silently answers
/// `0 deviations` about a file it never understood.
/// 这是两件不同的事实——"它一个字段都没声明"与"它的声明读不出来"——而用前者去填后者，正是一份比较对
/// 一个它从没读懂的文件静默答出 `0 deviations` 的来路。
///
/// Pinned on the function rather than through a sibling: a file the tree refused never becomes a
/// sibling, which the next pin shows from the other side.
/// 钉在这个函数上而不是经由某个兄弟：被树拒绝的文件根本成不了兄弟，而这一点由下一条钉子从另一面展示。
#[test]
fn one_face_turns_every_unreadable_declaration_into_a_reason() {
    let none = super::one_face("pub struct Timeline;\n").expect_err("no declaration");
    assert!(none.contains("no registration face"), "{none}");
    let two = super::one_face(
        "crate::control_object! { kind: A, parent: crate::control::NODE_ID }\n\
         crate::control_object! { kind: B, parent: crate::control::NODE_ID }\n",
    )
    .expect_err("two declarations");
    assert!(
        two.contains("2 registration faces"),
        "the count is the reason: {two}"
    );
    let parsed = super::one_face(
        "crate::control_object! { kind: A, parts: AParts, parent: crate::control::NODE_ID }\n",
    )
    .expect("one declaration parses");
    assert_eq!(
        super::declared_shape(&parsed),
        vec![("parts".to_owned(), "AParts".to_owned())],
        "the shape is the fields the declaration carries"
    );
}

/// A file the derivation dropped is named, so the sibling count is not read as the whole family.
/// 被推导丢掉的文件会被点名，因此同族计数不会被读成整个同族。
///
/// A file the build refuses is not in the tree, so it is not in the sibling set either. The count is
/// therefore correct and incomplete at once, and the answer has to carry the tree's own drop line —
/// otherwise "1 sibling" reads as "this family has one sibling" when it has two.
/// 构建拒绝的文件不在树里，因此也不在同族集合里。计数因此既正确又不完整，而答案必须带上树自己那一行
/// 丢弃记录——否则"1 sibling"会被读成"这个同族只有一个兄弟"，而它有两个。
#[test]
fn a_file_the_tree_dropped_is_named_beside_the_count() {
    let root = specimen_package("specimen-dropped");
    adopt(
        &root,
        "root/control/button",
        "src/control/object/button/button.rs",
    );
    write_fixture(
        &root.join("src/control/object/timeline/timeline.rs"),
        "crate::control_object! { kind: Timeline, parts: TimelineParts, parent: \
         crate::control::NODE_ID }\n\
         crate::control_object! { kind: TimelineAgain, parts: TimelineParts, parent: \
         crate::control::NODE_ID }\n",
    );
    let answer =
        super::consistency(&root, &json!({"specimen": "root/control/button"})).expect("an answer");
    assert!(
        answer.contains("expected one registration face in this file")
            && answer.contains("is not in the sibling set below"),
        "the drop is carried rather than swallowed: {answer}"
    );
    assert!(
        answer.contains("1 sibling(s)"),
        "the count is the tree's own, and the drop line is what keeps it honest: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// An anchor no ledger entry names is answered, and the answer says where the anchor comes from.
/// 台账没有条目点名的 anchor 会被作答，而且答案说清这个锚是从哪来的。
#[test]
fn an_anchor_without_a_ledger_entry_is_answered_rather_than_guessed() {
    let root = specimen_package("specimen-no-ledger");
    let answer =
        super::consistency(&root, &json!({"specimen": "root/control/button"})).expect("an answer");
    assert!(
        answer.starts_with("no ledger entry names `root/control/button`"),
        "{answer}"
    );
    assert!(
        answer.contains("an adoption is a lease") && answer.contains("`adopted` lists the anchors"),
        "the baseline's rule and the way to the ledger are named: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `specimen` and `parent` are two baselines, so asking for both is refused with the shape.
/// `specimen` 与 `parent` 是两个基准，因此同时问会被拒绝并带上形状。
#[test]
fn the_two_baselines_are_not_accepted_together() {
    let root = std::env::temp_dir();
    let both = super::consistency(
        &root,
        &json!({"parent": "root/control", "specimen": "root/control/button"}),
    )
    .expect_err("two baselines are refused");
    assert!(
        both.contains("not both") && both.contains("accepted shape"),
        "{both}"
    );
    let missing = super::consistency(&root, &json!({})).expect_err("a baseline is required");
    assert!(
        missing.contains("`parent` or `specimen`") && missing.contains("accepted shape"),
        "{missing}"
    );
}

/// Every shape field this comparison reads is a field name the kernel's own vocabulary carries.
/// 这次比较读的每个形状字段，都是内核自己词表携带的字段名。
///
/// This is the acceptance pin for the list: a typo, or a field the parser does not know, would make
/// the comparison answer `lacks` about a declaration that is really there — and a list can be
/// checked against the vocabulary that owns it without a fixture.
/// 这是那张清单的接受性钉子：一个拼写错误、或解析器不认识的字段，会让比较对一条**确实在**的声明答出
/// `lacks`——而这张清单可以对着拥有它的词表检查，不需要夹具。
#[test]
fn the_shape_fields_are_the_kernels_own_vocabulary() {
    let vocabulary = nichlink_kernel::declaration::FACE_FIELD_ORDER;
    for (name, _) in super::SHAPE_FIELDS {
        assert!(
            vocabulary.contains(name),
            "`{name}` is not a face field the kernel declares: {vocabulary:?}"
        );
    }
    // And every field name the lines print is one of the compared ones.
    assert_eq!(
        super::shape_field_names(),
        super::SHAPE_FIELDS
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
    );
}

/// The deviating sibling is named, and the note says which way it deviates.
/// 不一样的兄弟被点名，注里说清它往哪边不一样。
#[test]
fn the_deviating_sibling_is_named_with_the_direction() {
    let sets = vec![
        ("button".to_owned(), set(&["to_world", "scale_x"])),
        ("slider".to_owned(), set(&["to_local", "scale_x"])),
        ("timeline".to_owned(), set(&["to_local", "scale_x"])),
    ];
    let rows = super::deviations(&sets);
    assert_eq!(rows.len(), 1, "one sibling differs: {rows:?}");
    assert_eq!(rows[0].0, "button");
    let notes = rows[0].1.join("; ");
    assert!(
        notes.contains("does not call `to_local`, which the other siblings call"),
        "the missing majority name is named: {notes}"
    );
    assert!(
        notes.contains("calls `to_world`, which no sibling calls"),
        "the lone name is named: {notes}"
    );
}

/// Unanimity is not an outlier, and neither is a name shared by exactly half.
/// 全体一致不是离群；恰好一半共享的名字也不算。
#[test]
fn unanimity_and_halves_are_not_outliers() {
    let same = vec![
        ("a".to_owned(), set(&["x", "y"])),
        ("b".to_owned(), set(&["x", "y"])),
    ];
    assert!(super::deviations(&same).is_empty(), "unanimous");

    let halves = vec![
        ("a".to_owned(), set(&["x", "only_a"])),
        ("b".to_owned(), set(&["x", "only_b"])),
    ];
    let rows = super::deviations(&halves);
    assert_eq!(
        rows.len(),
        2,
        "a name exactly half the siblings call is not a majority: {rows:?}"
    );
}

/// The refusals carry the accepted shape, for a missing parent and for an unknown signal.
/// 拒绝带可接受形状：缺 `parent` 与未知的 `by` 各一条。
#[test]
fn the_refusals_carry_the_shape() {
    let root = std::env::temp_dir();
    let missing = super::consistency(&root, &json!({})).expect_err("a parent is required");
    assert!(
        missing.contains("consistency needs `parent`") && missing.contains("accepted shape"),
        "{missing}"
    );
    let unknown = super::consistency(&root, &json!({"parent": "root/control", "by": "colour"}))
        .expect_err("the signal is checked before the tree is read");
    assert!(
        unknown.contains("`api`, `kind` or `source`") && unknown.contains("accepted shape"),
        "{unknown}"
    );
}

/// The sibling set is exactly the faces one level under the parent.
/// 同族集合正好是父级下一层的那些面。
#[test]
fn the_sibling_set_is_one_level_deep() {
    // The rule is a path rule, so it is pinned on paths rather than on a whole fixture.
    // 这条规则是路径规则，因此按路径钉，而不是搭一整套夹具。
    for (path, parent, wanted) in [
        ("root/control/button", "root/control", true),
        ("root/control/button/label", "root/control", false),
        ("root/control", "root/control", false),
        ("root/other/button", "root/control", false),
    ] {
        let above = path.rsplit_once('/').map(|(above, _)| above);
        assert_eq!(above == Some(parent), wanted, "`{path}` under `{parent}`");
    }
}
