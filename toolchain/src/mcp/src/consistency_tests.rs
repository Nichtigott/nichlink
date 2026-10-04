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
        "/// The local family every child is expected to use.\npub fn to_local(x: i32) -> i32 { x + \
         7 }\n\n/// The other family, which one child may drift onto.\npub fn to_world(x: i32) -> \
         i32 { x + 31 }\n\npub struct Control;\npub struct ControlParts;\n\ncrate::root_object! \
         {\n    kind: Control,\n    parts: ControlParts,\n    needs_registry: true,\n    parent: \
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
                "pub struct {kind};\npub struct {kind}Parts;\n\n/// This widget's offset, in \
                 the family its siblings use.\npub fn offset(x: i32) -> i32 {{ \
                 crate::control::to_local(x) }}\n\ncrate::control_object! {{\n    kind: {kind},\n    \
                 parts: {kind}Parts,\n    exports: [\"control.render\"],\n    handle_traits: \
                 [\"ControlHandle\"],\n    parent: crate::control::NODE_ID,\n}}\n"
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
    // This pin is about the comparison, not about what the session has already been told (audit
    // `W2-2` shortens the bounds block from the second call on, which `the_bounds_are_said_once`
    // pins on its own). Forgetting makes each call below a first call, so the assertion is about
    // the answer rather than about the ledger.
    // 这条钉子比的是**比对**，不是这个会话已经被说过什么（审计 `W2-2` 让第二次调用起的边界块变短，那件事
    // 由 `the_bounds_are_said_once` 自己钉）。先忘掉，让下面每次调用都是第一次，于是断言比的是答案而不是账本。
    crate::mcp::session::forget();
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
    crate::mcp::session::forget();
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
        super::declared_shape(&parsed)
            .into_iter()
            .map(|field| (field.key, field.spelling, field.value))
            .collect::<Vec<_>>(),
        vec![("parts", "parts", "AParts".to_owned())],
        "the shape is the fields the declaration carries, under their own spelling"
    );
}

/// A displayed field name is the one the declaration wrote, and the other spelling is not a drift.
/// 显示出来的字段名就是声明写下的那一个，而另一种拼写不算漂移。
///
/// Audit `F1`: the comparison used to report a trait field under a fixed spelling, so a reader of a
/// face that wrote `handle_contracts` was handed `handle_traits` — a name whose value does not live
/// where the file keeps it. The two spellings are one field (so choosing either is not a deviation)
/// and the printed name is the file's own (so what a reader copies back is what is really there).
/// 审计 `F1`：这次比较过去用一个固定拼写报告 trait 字段，于是读到一个写 `handle_contracts` 的面的人，
/// 拿到的是 `handle_traits`——一个取值并不住在文件保存它的位置的名字。两种拼写是同一个字段（选哪个都
/// 不算偏离），而印出来的名字是文件自己的（读者抄回去的就是真正在那里的东西）。
#[test]
fn a_displayed_field_name_is_the_spelling_the_declaration_wrote() {
    let root = specimen_package("specimen-spelling");
    adopt(
        &root,
        "root/control/button",
        "src/control/object/button/button.rs",
    );
    let button = "src/control/object/button/button.rs";
    replace(
        &root,
        button,
        "    handle_traits: [\"ControlHandle\"],\n",
        "    handle_contracts: [crate::control::ControlHandle],\n",
    );
    let answer =
        super::consistency(&root, &json!({"specimen": "root/control/button"})).expect("an answer");
    assert!(
        answer.contains("handle_contracts `ControlHandle`")
            && !answer.contains("handle_traits `ControlHandle`"),
        "the specimen's own spelling is what a reader is handed: {answer}"
    );
    assert!(
        answer.contains("conformance: 0 of 2 sibling(s)"),
        "the siblings write the other spelling of the same field, so nothing is lacking: {answer}"
    );

    // And the other way round: the sibling that really lacks the field is named under the
    // specimen's spelling, because that is the name the baseline carries.
    let timeline = "src/control/object/timeline/timeline.rs";
    replace(
        &root,
        timeline,
        "    handle_traits: [\"ControlHandle\"],\n",
        "",
    );
    let missing =
        super::consistency(&root, &json!({"specimen": "root/control/button"})).expect("an answer");
    assert!(
        missing.contains("outlier     timeline: lacks `handle_contracts`"),
        "the row names the spelling the specimen wrote: {missing}"
    );
    let _ = std::fs::remove_dir_all(&root);
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
    for field in super::SHAPE_FIELDS {
        assert!(
            vocabulary.contains(&field.key),
            "`{}` is not a face field the kernel declares: {vocabulary:?}",
            field.key
        );
        // Every spelling too: the printed lines hand a reader one of these, and a spelling the
        // kernel does not carry is the `F1` defect in its other form — a name that cannot be pasted
        // back because the parser never knew it.
        // 每一种拼写也要查：印出来的行递给读者的就是其中之一，而内核不携带的拼写是 `F1` 缺陷的另一种
        // 形态——一个放不回去的名字，因为解析器从不认识它。
        for spelling in field.spellings {
            assert!(
                vocabulary.contains(spelling),
                "`{spelling}` is not a face field the kernel declares: {vocabulary:?}"
            );
        }
    }
    // And every field name the lines print is one of the compared ones.
    assert_eq!(
        super::shape_field_names(),
        super::SHAPE_FIELDS
            .iter()
            .flat_map(|field| field.spellings.iter().copied())
            .collect::<Vec<_>>()
    );
    // The two trait fields are the only ones with a second spelling, and each pair is one key.
    // 只有两个 trait 字段有第二种拼写，而每一对共用一个键。
    let two_spellings = super::SHAPE_FIELDS
        .iter()
        .filter(|field| field.spellings.len() > 1)
        .map(|field| field.key)
        .collect::<Vec<_>>();
    assert_eq!(two_spellings, vec!["handle_traits", "part_traits"]);
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
    let rows = super::deviations(&sets, super::Wording::Calls);
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
    assert!(
        super::deviations(&same, super::Wording::Calls).is_empty(),
        "unanimous"
    );

    let halves = vec![
        ("a".to_owned(), set(&["x", "only_a"])),
        ("b".to_owned(), set(&["x", "only_b"])),
    ];
    let rows = super::deviations(&halves, super::Wording::Calls);
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
        unknown.contains("`api`, `kind`, `source` or `shape`")
            && unknown.contains("accepted shape"),
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

/// The `api` signal reads the calls the sibling's own file makes, and names the one that drifted.
/// `api` 信号读的是那个兄弟自己的文件所做的调用，并点名漂移的那一个。
///
/// The failure this guards was measured on a real host: the directory prefix was built with a second
/// `src/`, so every source filter missed and the whole signal answered `0 call(s)` on **every** tree.
/// A pin on the rule alone could not see it — the rule was right and the wiring was wrong — which is
/// why this one goes through the tool and a package.
/// 它守的失败是在一个真宿主上量到的：目录前缀里多拼了一次 `src/`，于是每一个源码过滤都落空、整个信号在
/// **每一棵**树上都答 `0 call(s)`。只钉规则看不见它——规则是对的、接线是错的——因此这一条经由工具与一个
/// 包来钉。
#[test]
fn the_api_signal_reads_each_siblings_own_calls_and_names_the_drifter() {
    let root = specimen_package("api-signal");
    let file = "src/control/object/slider/slider.rs";
    let clean =
        super::consistency(&root, &json!({"parent": "root/control", "by": "api"})).expect("answer");
    assert!(
        clean.contains("1 call(s): to_local") && !clean.contains("\n  outlier"),
        "every sibling's own call is read and unanimity is not an outlier: {clean}"
    );

    replace(
        &root,
        file,
        "crate::control::to_local(x)",
        "crate::control::to_world(x)",
    );
    let drifted =
        super::consistency(&root, &json!({"parent": "root/control", "by": "api"})).expect("answer");
    assert!(
        drifted.contains("outlier     slider: does not call `to_local`")
            && drifted.contains("calls `to_world`, which no sibling calls"),
        "the drifter and both directions are named: {drifted}"
    );
    assert!(
        drifted.contains("outliers: 1 of 3"),
        "exactly one sibling differs: {drifted}"
    );

    replace(
        &root,
        file,
        "crate::control::to_world(x)",
        "crate::control::to_local(x)",
    );
    let back =
        super::consistency(&root, &json!({"parent": "root/control", "by": "api"})).expect("answer");
    assert!(
        back.contains("outliers: 0 of 3"),
        "putting the call back makes the family unanimous again: {back}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The comparison derives its tree instead of reading published records, and it says that it did.
/// 这次比较推导自己的树，而不是读发布记录，并且它说出了这一点。
///
/// The record carries no `path` and no declared fields, so reading it made this tool refuse on
/// exactly the trees that have been built — the normal case. This is the coupling pin for that: a
/// swap back to `member.tree()` fails here even on a tree with no records to disagree about.
/// 记录不携带 `path`、也不携带已声明字段，因此读它会让本工具恰好在**已经构建过**的树上拒答——那是常态。
/// 这是那件事的耦合钉子：改回 `member.tree()` 会在这里失败，即便是在一棵没有记录可分歧的树上。
#[test]
fn the_comparison_derives_rather_than_reading_the_record() {
    let source = include_str!("consistency.rs");
    assert!(
        source.contains("derived_tree()"),
        "the derived reading is the documented fallback every caller states"
    );
    assert!(
        !source.contains("member.tree()"),
        "the record answers neither the sibling set's text nor its declared fields"
    );
    let root = specimen_package("derived-evidence");
    let answer = super::consistency(&root, &json!({"parent": "root/control", "by": "kind"}))
        .expect("answer");
    assert!(
        answer.contains("tree derived now") && answer.contains("no published records at"),
        "the answer says which tree it read, and why it derived: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A family whose only difference is an **extra declared field** is named — in **one default call**.
/// 唯一差别是**多声明了一个字段**的同族会被点名 —— 而在**一次缺省调用**里。
///
/// The measured failure this guards: S5's family differed exactly this way (one sibling declared
/// `exports`, the others did not) and `consistency --parent root` answered `outliers: 0 of 3`,
/// because the only signals were `api` (call names) and a declared-*value* comparison — a sibling
/// carrying a field the rest do not is neither. The promise the workflow table makes ("one call
/// names the outlier") therefore did not hold for the drift it most often has to catch.
/// 这条守着的实测失败：S5 那个同族的差别正是这样（一个兄弟声明了 `exports`，其余没有），而
/// `consistency --parent root` 回的是 `outliers: 0 of 3` —— 因为当时只有 `api`（调用的名字）与"声明**取值**"
/// 两种信号，而"兄弟携带了别人没有的字段"两者都不是。于是流程表承诺的那句"一次调用点名离群者"在最常要
/// 抓的那类漂移上并不成立。
#[test]
fn a_family_outlier_that_declares_an_extra_field_is_named() {
    let root = specimen_package("family-extra-field");
    let clean = super::consistency(&root, &json!({"parent": "root/control"})).expect("an answer");
    assert!(
        clean.contains("by api") && clean.contains("by shape"),
        "a request without `by` runs both signals: {clean}"
    );
    assert!(
        clean.contains("outliers: 0 of"),
        "the fixture starts unanimous: {clean}"
    );

    // The fixture declares `exports` on **every** child, so the S5 shape is reproduced by taking it
    // away everywhere first and then giving it back to exactly one sibling: the family's only
    // difference becomes one extra declared field.
    // 夹具给**每个**子面都声明了 `exports`，因此复现 S5 的形状要先把它从所有兄弟身上取掉、再只还给一个：
    // 于是这个同族唯一的差别就是"多声明了一个字段"。
    for name in ["button", "slider", "timeline"] {
        replace(
            &root,
            &format!("src/control/object/{name}/{name}.rs"),
            "    exports: [\"control.render\"],\n",
            "",
        );
    }
    let before = super::consistency(&root, &json!({"parent": "root/control"})).expect("an answer");
    assert!(
        before.contains("outliers: 0 of"),
        "with the field gone from every child the family is unanimous again: {before}"
    );
    let file = "src/control/object/slider/slider.rs";
    replace(
        &root,
        file,
        "    parts: SliderParts,\n",
        "    parts: SliderParts,\n    exports: [\"control.render\"],\n",
    );
    let extra = super::consistency(&root, &json!({"parent": "root/control"})).expect("an answer");
    assert!(
        extra.contains("declares `exports`, which no sibling declares"),
        "the extra declaration is named: {extra}"
    );
    assert!(
        extra.contains("outlier") && extra.contains("outliers: 1 of"),
        "exactly one sibling is the outlier: {extra}"
    );
    // And the field itself is shown per sibling, so a reader can see the difference it turned on.
    // 而且每个兄弟各自声明了哪些字段都印出来，读者看得见这次判定依据的差别。
    assert!(
        extra.contains("field(s): exports") || extra.contains("exports"),
        "the shape signal shows the declared fields: {extra}"
    );
}

/// An outlier carries the **lines** it deviates on, and only those.
/// 离群者带上它偏离的那些**行**，也只带那些行。
///
/// Audit `W4-6` measured the two ways this can go: the row named a field and left the reader to open
/// the file (a second instrument call), while the compared tool answered the same question with a
/// 6,781-character slab of the whole file. This is the middle: the decisive line, quoted with its
/// neighbour's counterpart, capped at [`super::EXCERPT_LIMIT`].
/// 审计 `W4-6` 量到这条路的两种走法：一行只点名字段、把开文件留给读者（于是多一次仪器调用），而对照工具对
/// 同一个问题回的是整份文件 6,781 字符的片段。这里是中间那一种：决定性的那一行，连着对位的那一行一起引用，
/// 上限是 [`super::EXCERPT_LIMIT`]。
#[test]
fn an_outlier_carries_the_lines_it_deviates_on() {
    let root = specimen_package("outlier-lines");
    // The specimen path: the ledger certifies `button`, and `timeline` loses the declaration.
    adopt(
        &root,
        "root/control/button",
        "src/control/object/button/button.rs",
    );
    let timeline = "src/control/object/timeline/timeline.rs";
    replace(&root, timeline, "    exports: [\"control.render\"],\n", "");
    let specimen =
        super::consistency(&root, &json!({"specimen": "root/control/button"})).expect("an answer");
    assert!(
        specimen.contains("outlier     timeline: lacks `exports`"),
        "the row still names the field: {specimen}"
    );
    assert!(
        specimen.contains("src/control/object/timeline/timeline.rs:")
            && specimen.contains("src/control/object/button/button.rs:"),
        "and quotes the outlier's own declaration beside the specimen's: {specimen}"
    );

    // The parent path: only the sibling that really carries the extra field is quoted, and no more
    // than the cap.
    // 父面那一支：只有真的多声明了字段的那个兄弟被引用，而且不超过上限。
    let slider = "src/control/object/slider/slider.rs";
    replace(&root, slider, "    exports: [\"control.render\"],\n", "");
    replace(
        &root,
        slider,
        "    parts: SliderParts,\n",
        "    parts: SliderParts,\n    exports: [\"control.render\"],\n",
    );
    let parent = super::consistency(&root, &json!({"parent": "root/control"})).expect("an answer");
    let quoted = parent
        .lines()
        .filter(|line| line.trim_start().starts_with("src/") && line.contains(".rs:"))
        .count();
    assert!(
        quoted > 0 && quoted <= super::EXCERPT_LIMIT,
        "some line is quoted, within the cap: {parent}"
    );
    assert!(
        parent
            .lines()
            .any(|line| line.trim_start().starts_with("src/")
                && line.contains(".rs:")
                && line.contains("exports")),
        "and the quoted line is the declaration the deviation is about: {parent}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The answer opens with the tree's size and closes with a conservative verdict plus real commands.
/// 答案以树的大小开头，以保守的裁定与真实命令收尾。
///
/// Audit `W4-5`: the debug battery's sessions could not tell an answer that was finished from one
/// that had stopped early — both ended on a `next` line. The rule here is the conservative half: a
/// clean family is `closed`, anything named as a deviation or unreadable is `open` with the call
/// that resolves it, and every such call is one a reader can paste.
/// 审计 `W4-5`：debug 电池里的会话分不出"答完了"与"提早停了"——两者都以 `next` 行收尾。这里的规则是保守的
/// 那一半：干净的同族是 `closed`，任何被点名的偏离或读不了的文件都是 `open`，并附上解决它的那次调用，而
/// 每一个这样的调用都是读者可以照抄的。
#[test]
fn the_answer_opens_with_the_census_and_closes_conservatively() {
    let root = specimen_package("closure");
    let closed = super::consistency(&root, &json!({"parent": "root/control"})).expect("an answer");
    assert!(
        closed.starts_with("tree: ")
            && closed.contains(" face(s), ")
            && closed.contains(" file(s)"),
        "the census is the first line: {closed}"
    );
    assert!(
        closed.contains("closure    closed"),
        "a family nobody deviates from and every file read is closed: {closed}"
    );
    assert_eq!(
        closed.matches("closure    ").count(),
        1,
        "exactly one closure line: {closed}"
    );

    // Take a declaration away: the answer is now open, and the call it leaves is a real coordinate.
    // 取走一条声明：答案变成开的，而它留下的调用是一个真实坐标。
    let file = "src/control/object/slider/slider.rs";
    replace(&root, file, "    exports: [\"control.render\"],\n", "");
    let open = super::consistency(&root, &json!({"parent": "root/control"})).expect("an answer");
    assert!(
        open.contains("closure    open"),
        "a named deviation is not a closed answer: {open}"
    );
    assert!(
        open.contains("remaining  --call read --path src/control/object/slider/slider.rs --line ")
            || open.contains(
                "remaining  --call read --path src/control/object/button/button.rs --line "
            ),
        "and it carries a runnable `read` for the line that decides it: {open}"
    );
    for line in open.lines().filter(|line| line.starts_with("remaining")) {
        assert!(
            !line.contains('{') && !line.contains('<'),
            "no template in a command a reader is told to run: {line}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The comparison's boundary block is printed once per session, and referred back to afterwards.
/// 比对的边界块每个会话印一次，此后只回指。
///
/// Audit `W2-2`: this block is a constant, and the round measured constant blocks as the payload
/// every answer pays for. The second comparison in the same session gets one line that points at the
/// first — and a **different root** gets its own first time, which is what keeps two trees (or two
/// tests' scratch directories) from sharing a ledger.
/// 审计 `W2-2`：这个块是常量，而那一轮量到常量块正是每条答案都在付费的载荷。同一会话里的第二次比对拿到一行
/// 指向第一次的回指——而**另一个根**有自己的第一次，正是这一条让两棵树（或两个测试的临时目录）不共享账本。
#[test]
fn the_bounds_are_said_once_per_session_and_root() {
    crate::mcp::session::forget();
    let first = specimen_package("bounds-once");
    let whole = super::consistency(&first, &json!({"parent": "root/control"})).expect("an answer");
    assert!(
        whole.contains("not covered by this comparison"),
        "the first answer carries the whole block: {whole}"
    );
    let second = super::consistency(&first, &json!({"parent": "root/control"})).expect("an answer");
    assert!(
        second.contains("not covered: unchanged from this session's earlier comparison")
            && !second.contains("not covered by this comparison"),
        "the second refers back instead of repeating: {second}"
    );
    assert!(
        second.len() < whole.len(),
        "and it is shorter ({} vs {})",
        second.len(),
        whole.len()
    );
    let elsewhere = specimen_package("bounds-once-elsewhere");
    let other = super::consistency(&elsewhere, &json!({"parent": "root/control"})).expect("answer");
    assert!(
        other.contains("not covered by this comparison"),
        "another root has its own first time: {other}"
    );
    // Both paths of the comparison follow the rule, and the specimen path shares the key with the
    // parent path: one tree's boundary is one boundary.
    // 比对的两条路都守这条规则，而且标本那一支与父面那一支共用键：一棵树的边界就是一条边界。
    let specimen =
        super::consistency(&first, &json!({"specimen": "root/control/button"})).expect("an answer");
    assert!(
        !specimen.contains("not covered by this comparison"),
        "the specimen path refers back too: {specimen}"
    );
    let _ = std::fs::remove_dir_all(&first);
    let _ = std::fs::remove_dir_all(&elsewhere);
}
