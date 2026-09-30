//! Tests for `nichlink.search`'s tree half: a face is found by logical path,
//! `kind`, module or `registry_name`, and each hit says what the build thinks of it.
//! `nichlink.search` 树那一半的测试：面可以按逻辑路径、`kind`、模块或 `registry_name` 找到，而每个命中都说
//! 出构建对它的看法。
//!
//! The classification rule itself is pinned through `nichlink.diff`'s tests as
//! well; what this file pins is that search reads that one rule, so a face cannot
//! be `ok` in one tool and `added` in the other.
//! 分类规则本身也由 `nichlink.diff` 的测试钉住；本文件钉的是 search 读的正是那一条规则，因此一个面
//! 不可能在一个工具里是 `ok`、在另一个里是 `added`。

use std::path::{Path, PathBuf};

use serde_json::json;

use super::search;

/// A throwaway package with two hand-written root faces.
/// 一个含两个手写根面的一次性包。
fn package(label: &str) -> (PathBuf, String) {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-search-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    face(
        &root,
        "gauge/gauge.rs",
        "Gauge",
        "pub fn gauge_value() -> u8 { 1 }\n\ncrate::root_object! {\n    kind: Gauge,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    face(
        &root,
        "button/button.rs",
        "Button",
        "crate::root_object! {\n    kind: Button,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    (root, name)
}

/// Write one face's module.
/// 写入一个面的模块。
fn face(root: &Path, relative: &str, _kind: &str, source: &str) {
    let path = root.join("src").join(relative);
    std::fs::create_dir_all(path.parent().expect("face parent")).expect("face directory");
    std::fs::write(path, source).expect("face source");
}

/// Publish the build evidence for this package.
/// 发布本包的构建证据。
fn publish(root: &Path, name: &str) {
    crate::build_time::check_for(root, &root.join("target/nichlink/out"), name)
        .expect("a healthy tree checks clean");
}

/// A face is found by its logical path, kind, module and `registry_name`, and with no
/// build published the verdict is `build unknown` rather than a guess.
/// 面可以按其逻辑路径、kind、模块与 `registry_name` 找到；没有发布构建时结论是 `build unknown` 而不是猜。
#[test]
fn a_face_is_found_and_an_unbuilt_tree_says_the_verdict_is_unknown() {
    let (root, _) = package("unbuilt");
    let reply = search(&root, &json!({"query": "gauge"})).expect("the search answers");
    assert!(reply.contains("face  root/gauge"), "{reply}");
    assert!(reply.contains("kind=Gauge"), "{reply}");
    assert!(reply.contains("module=gauge"), "{reply}");
    assert!(reply.contains("source=gauge/gauge.rs"), "{reply}");
    assert!(
        reply.contains("[build unknown (run `nichlink check`)]"),
        "an unbuilt tree must not invent a verdict: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// Once the build published, a face it saw is `ok` and one it never saw is
/// `added since build`.
/// 构建发布之后，它见过的面是 `ok`，它从未见过的是 `added since build`。
#[test]
fn a_published_face_is_ok_and_a_new_one_is_added_since_build() {
    let (root, name) = package("published");
    publish(&root, &name);
    let known = search(&root, &json!({"query": "root/button"})).expect("the search answers");
    assert!(known.contains("[ok]"), "{known}");

    face(
        &root,
        "dial/dial.rs",
        "Dial",
        "crate::root_object! {\n    kind: Dial,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    let added = search(&root, &json!({"query": "dial"})).expect("the search answers");
    assert!(added.contains("face  root/dial"), "{added}");
    assert!(added.contains("[added since build]"), "{added}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A `kind` change under an unmoved file is `re-identified` with the old and new
/// identities — the one change no text search can see.
/// 文件没动而 `kind` 变了会报成 `re-identified` 并带出旧、新身份——这是文本搜索看不见的那种变化。
#[test]
fn a_kind_change_under_an_unmoved_file_is_re_identified() {
    let (root, name) = package("re-identified");
    publish(&root, &name);
    face(
        &root,
        "button/button.rs",
        "Button",
        "crate::root_object! {\n    kind: RenamedButton,\n    parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")),\n}\n",
    );
    let reply = search(&root, &json!({"query": "root/button"})).expect("the search answers");
    assert!(reply.contains("[re-identified ("), "{reply}");
    assert!(reply.contains(" -> "), "{reply}");
    assert!(
        reply.contains("build stale (run `nichlink check`)"),
        "the verdicts are about the build that was published, and the reply says so: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The source half is unchanged: a file path and a function declaration by name
/// still answer exactly as before, below the face hits.
/// 源码那一半不变：按名字匹配的文件路径与函数声明仍和以前一样作答，排在面命中之后。
#[test]
fn the_source_half_still_answers_file_and_function_names() {
    let (root, _) = package("source-half");
    let by_file = search(&root, &json!({"query": "gauge"})).expect("the search answers");
    assert!(by_file.contains("face  root/gauge"), "{by_file}");
    assert!(by_file.contains("file  src/gauge/gauge.rs"), "{by_file}");

    let by_function = search(&root, &json!({"query": "gauge_value"})).expect("the search answers");
    assert!(
        by_function.contains("fn    gauge_value -> src/gauge/gauge.rs:"),
        "{by_function}"
    );

    let none =
        search(&root, &json!({"query": "nothing-matches-this"})).expect("the search answers");
    // The intent is unchanged — a query that matches nothing says so — and the answer now also
    // names the question that would have matched it, because names and text are different searches.
    // 意图没变——什么都没匹配上的查询会说出来——而答案现在也点名那个本可以匹配的问题，因为名字与文本
    // 是两种检索。
    assert!(none.starts_with("no matches"), "{none}");
    assert!(none.contains("pass `literal`"), "{none}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The result bound is a bound **and says so**: the scan used to stop at `limit` with
/// nothing in the reply saying rows were left out, so a caller could not tell a complete
/// answer from a full one. The count is taken over the whole scan, so the sentence can say
/// `of N` rather than a lower bound.
/// 结果上限是上限**而且自己说出来**：扫描过去在 `limit` 处停下，而回复里没有任何东西说还有行被
/// 落下，因此调用方分不出完整答案与塞满的答案。计数取自整次扫描，因此那句话说的是「N 中的 M」，
/// 而不是一个下界。
#[test]
fn a_result_list_cut_by_the_limit_says_how_many_it_withheld() {
    let (root, _) = package("bounded");
    let reply = search(&root, &json!({"query": "gauge", "limit": 1})).expect("the search answers");
    // `gauge` matches the face, the file and the function: three rows, one shown.
    // `gauge` 匹配面、文件与函数：三行，显示一行。
    assert!(
        reply.contains("… truncated: 2 of 3 results withheld at the limit of 1"),
        "{reply}"
    );
    assert!(reply.contains("face  root/gauge"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A root whose identity namespace cannot be learned still answers the source
/// half, and says the tree half is unavailable instead of pretending it is empty.
/// 身份命名空间无从得知的根仍然回答源码那一半，并说明树那一半不可用，而不是假装它是空的。
#[test]
fn a_root_without_a_package_still_answers_the_source_half() {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root =
        std::env::temp_dir().join(format!("mcp-search-bare-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("source directory");
    std::fs::write(root.join("src/widget.rs"), "pub fn widget_spin() {}\n").expect("source file");

    let reply = search(&root, &json!({"query": "widget"})).expect("the search answers");
    assert!(reply.contains("tree  unavailable ("), "{reply}");
    assert!(reply.contains("file  src/widget.rs"), "{reply}");
    assert!(
        reply.contains("fn    widget_spin -> src/widget.rs:1"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A registration file the derivation cannot parse is counted in the reply instead of vanishing:
/// a read-only tree query used to report a smaller tree as if it were the whole one, which is what
/// `LGC-LG-11` recorded on the producer side and what the consumer half now says out loud.
/// 推导解析不了的注册面文件被计入回复而不是消失：只读的树查询过去把一棵更小的树当成完整的树报出去
/// ——这正是 `LGC-LG-11` 在生产端记录的事，而消费端现在把它说出来。
fn broken_face(root: &std::path::Path, label: &str) {
    let directory = root.join("src").join(label);
    std::fs::create_dir_all(&directory).expect("module directory");
    std::fs::write(
        directory.join(format!("{label}.rs")),
        "crate::root_object! {\n    kind: Broken,\n",
    )
    .expect("truncated face");
}

#[test]
fn the_tree_half_counts_unparsable_registration_files() {
    let (root, _) = package("unparsable");
    broken_face(&root, "broken");
    let reply = search(&root, &json!({"query": "gauge"})).expect("the search answers");
    assert!(reply.contains("unparsable faces 1"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The count of registration files the derivation could not parse is a fact about the tree that
/// was read, so it is reported even when the query matches no face: a query naming a face that
/// lives in a broken file used to come back as nothing special, with the broken file unmentioned
/// (`LGC-LG-11`). The half fixed here is that the line no longer depends on a face having matched.
/// 推导解析不了的注册面文件数量是**被读到的那棵树**的事实，因此即使查询没有命中任何面也要报出来：
/// 一个点名了住在坏文件里的面的查询，过去会回一条看不出异常的结果，而那个坏文件一字未提
/// （`LGC-LG-11`）。这里修的一半是：这一行不再取决于"有没有面命中"。
#[test]
fn an_unparsable_file_is_reported_even_when_no_face_matches() {
    let (root, _) = package("unparsable-no-match");
    broken_face(&root, "broken");
    let reply =
        search(&root, &json!({"query": "nothing-matches-this"})).expect("the search answers");
    assert!(
        reply.contains("unparsable faces 1"),
        "the tree read must say what it could not parse: {reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The convergence entry answers in layers and says what it did not look at: the tree's own
/// verdicts, the chain the name leads into (composed from the call-graph tool rather than
/// re-derived), the next step, and the bounds. One call localizes where three used to.
/// 收敛入口分层作答，并说出它**没有**看什么：树自己的裁决、这个名字引向的链（由调用图工具**组合**而来，
/// 不是重新推导）、下一步，以及边界。过去要三次调用才能定位的事，现在一次就够。
#[test]
fn the_convergence_layer_labels_each_layer_and_its_bounds() {
    let (root, _name) = package("converge");
    let text = search(&root, &json!({"query": "Button", "converge": true})).expect("an answer");
    assert!(text.contains("chain:"), "{text}");
    assert!(text.contains("next: pass `path`"), "{text}");
    assert!(text.contains("bounds: static only"), "{text}");
    assert!(
        text.contains("no static function match") || text.contains("callers ("),
        "the chain layer is the call graph's own answer: {text}"
    );
    // Without the flag the answer is exactly what it was: layers are opt-in, so no existing
    // caller's shape moves.
    // 不带这个开关时答案与从前完全一样：分层是显式请求的，因此既有调用方的形状不变。
    let plain = search(&root, &json!({"query": "Button"})).expect("an answer");
    assert!(!plain.contains("chain:"), "{plain}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A literal search finds raw text — what a failing assertion's message sends you looking for — and
/// it finds it where a masked view would not: inside a string literal and inside a comment. Asking
/// for a name and a literal at once is refused by name, because the two answers are not the same
/// shape.
/// 字面检索找的是原始文本——失败断言的文案正是让人去找的那个东西——而且它能在屏蔽过的视图找不到的
/// 地方找到：字符串字面量里与注释里。同时要名字与字面会被按名拒绝，因为两种答案不是同一种形状。
#[test]
fn a_literal_search_finds_text_in_strings_and_comments() {
    let (root, _name) = package("literal");
    std::fs::write(
        root.join("src/literal_probe.rs"),
        "// the message: plugin does not target framework\n\
         pub const MESSAGE: &str = \"plugin does not target framework `graft-test`\";\n",
    )
    .expect("probe source");
    let text = search(&root, &json!({"literal": "does not target framework"})).expect("an answer");
    assert!(text.contains("raw bytes, case-sensitive"), "{text}");
    assert!(text.contains("src/literal_probe.rs:1:"), "{text}");
    assert!(text.contains("src/literal_probe.rs:2:"), "{text}");
    let error = search(&root, &json!({"query": "Button", "literal": "x"})).expect_err("a refusal");
    assert!(error.contains("pass one of them"), "{error}");
    let none = search(&root, &json!({"literal": "nowhere-at-all"})).expect("an answer");
    assert!(none.contains("no matches"), "{none}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Neither argument at all: the tool names both, so the caller is not left guessing which one it
/// wanted.
/// 两个参数都不给：工具把两个都点名，调用方不必猜它要的是哪一个。
#[test]
fn a_search_with_neither_a_name_nor_a_literal_names_both() {
    let (root, _name) = package("neither");
    let error = search(&root, &json!({})).expect_err("a refusal");
    assert!(
        error.contains("`query`") && error.contains("`literal`"),
        "{error}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A name that matches nothing points at the other question instead of stopping at "no matches":
/// the spellings that fail as names are the ones the literal mode answers.
/// 按名字查不到时，答案指向另一个问题，而不是停在"no matches"：按名字失败的拼法正是字面模式能答的。
#[test]
fn a_name_that_matches_nothing_points_at_the_literal_mode() {
    let (root, _name) = package("hint");
    let text = search(&root, &json!({"query": "FrameworkMismatch"})).expect("an answer");
    assert!(text.contains("no matches"), "{text}");
    assert!(text.contains("pass `literal`"), "{text}");
    assert!(text.contains("Type::method"), "{text}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A name that is in the build's published record is answered **without deriving the sources** —
/// that is what the four published face facts are for — and a query that only the module can
/// satisfy still falls back to the derivation, because a record does not carry a module.
/// 名字出现在构建已发布的记录里时，答案是**不推导源码**就给出的——那四项已发布的面事实正是为此——而只有
/// 模块能满足的查询仍然回落到推导，因为记录不携带模块。
#[test]
fn a_published_record_answers_a_name_without_deriving_the_sources() {
    use crate::mcp::published::derivations;

    let (root, _name) = package("record-first");
    let out = root.join("target/nichlink/out");
    std::fs::create_dir_all(&out).expect("output dir");
    std::fs::write(
        out.join("pruning_manifest.tsv"),
        "# node\tsource\tsymbol\tpath\tkind\tregistry_name\tparent\n\
         bdb4427ce81c9bc51e56bee7667fd2be\tbutton/button.rs\t-\talpha::beta::gamma\tGamma\t\
         gamma\t-\n",
    )
    .expect("published record");

    let before = derivations();
    let answered = search(&root, &json!({"query": "gamma"})).expect("an answer");
    assert!(
        answered.contains("answered from the build's published records"),
        "{answered}"
    );
    assert!(answered.contains("kind=Gamma"), "{answered}");
    assert!(answered.contains("registry=gamma"), "{answered}");
    assert_eq!(
        derivations(),
        before,
        "the record answered it, so no source should have been walked: {answered}"
    );

    // `button` is the fixture face's **module**, which a record does not carry: the same query on
    // the same tree has to derive.
    let before = derivations();
    let derived = search(&root, &json!({"query": "button"})).expect("an answer");
    assert!(
        derived.contains("module=button"),
        "a module-only match comes from the sources: {derived}"
    );
    assert!(
        derivations() > before,
        "the module half can only be answered by deriving: {derived}"
    );
    let _ = std::fs::remove_dir_all(&root);
}
