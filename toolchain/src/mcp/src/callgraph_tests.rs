//! Tests for the two bounds `nichlink.callgraph` needs: definitions and callers.
//! `nichlink.callgraph` 需要的两道上限的测试：定义数与调用者数。
//!
//! The measured failure these exist against: a common name (`new`) has 142
//! definitions in the NichUI corpus, and every call site of that name was listed for
//! each of them, which arrived as a 4.5 MB reply. An answer that an agent cannot read
//! is not an answer, so both bounds are pinned here.
//! 这些测试所针对的实测失败：常见名（`new`）在 NichUI 语料里有 142 个定义，而每个定义都列出该名字
//! 在树里的每一个调用点，最终以 4.5 MB 的回复抵达。代理读不下的答案不算答案，因此两道上限都钉在这里。

use std::path::{Path, PathBuf};

use serde_json::json;

use super::{callgraph, is_call_to};

/// A throwaway source root: `definitions` files each declaring `fn new`, and one
/// file holding `callers` functions that all call it.
/// 一个一次性源码根：`definitions` 个文件各声明一个 `fn new`，外加一个文件，里面有 `callers`
/// 个函数都调用它。
fn tree(label: &str, definitions: usize, callers: usize) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "mcp-callgraph-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    for index in 0..definitions {
        let directory = root.join(format!("defs{index}"));
        std::fs::create_dir_all(&directory).expect("definition directory");
        std::fs::write(
            directory.join(format!("defs{index}.rs")),
            "pub fn new() {}\n",
        )
        .expect("definition");
    }
    let callers_directory = root.join("callers");
    std::fs::create_dir_all(&callers_directory).expect("caller directory");
    let mut body = String::new();
    for index in 0..callers {
        body.push_str(&format!("pub fn caller{index}() {{ new(); }}\n"));
    }
    std::fs::write(callers_directory.join("callers.rs"), body).expect("callers");
    root
}

/// A name with several definitions is named as ambiguous, the extra definitions
/// are withheld with their count, and the caller list is capped rather than
/// dumped.
/// 有多个定义的名字会被说成歧义，多出来的定义连同数量一起被扣下，调用者清单被截断而不是倾倒。
#[test]
fn a_common_name_is_bounded_and_its_ambiguity_is_named() {
    let root = tree("ambiguous", 3, 25);
    let reply =
        callgraph(&root, &json!({"function": "new", "limit": 1})).expect("the answer renders");
    assert!(reply.contains("matches 3"), "{reply}");
    assert!(reply.contains("note: 3 definitions match"), "{reply}");
    assert!(reply.contains("pass `path` to select one"), "{reply}");
    // One definition shown, two withheld, both stated.
    // 显示一个定义、扣下两个，两件事都写出来。
    assert!(
        reply.contains("… truncated: 2 of 3 definitions withheld at the limit of 1"),
        "{reply}"
    );
    // The caller list is capped: 25 callers, 20 shown.
    // 调用者清单有上限：25 个调用者，显示 20 个。
    assert!(reply.contains("callers (25):"), "{reply}");
    assert!(
        reply.contains("… truncated: 5 of 25 callers withheld at the limit of 20"),
        "{reply}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// `path` removes the ambiguity, so the note must disappear — it is advice, not
/// boilerplate.
/// `path` 消除了歧义，因此那句提示必须消失——它是建议，不是套话。
#[test]
fn a_path_filter_removes_the_ambiguity_note() {
    let root = tree("filtered", 3, 2);
    let reply = callgraph(&root, &json!({"function": "new", "path": "defs1/defs1.rs"}))
        .expect("the answer renders");
    assert!(reply.contains("matches 1"), "{reply}");
    assert!(!reply.contains("definitions match"), "{reply}");
    assert!(reply.contains("defs1/defs1.rs:1 fn new"), "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The catalog's contract for `limit` is the implementation's behaviour, pinned here rather than
/// only declared there: absent `limit` shows 5 definitions, a `limit` past the declared maximum of
/// 50 is *clamped* (the description says "at most 50", not "refused"), and a `limit` below the
/// declared minimum of 1 behaves as 1. The contract used to be undeclared while the
/// implementation read the key, which made `raise \`limit\`` advice a caller could not act on
/// (audit `LGC-LG-43`).
/// catalog 对 `limit` 的契约就是实现的行为，并且钉在这里、而不只是声明在那里：不传 `limit` 显示
/// 5 个定义；超过声明上限 50 的 `limit` 被**夹到 50**（描述说的是"至多 50"，不是"拒绝"）；低于声明
/// 下限 1 的 `limit` 按 1 处理。过去这个键未声明而实现却在读它，于是 `raise \`limit\`` 那条建议是
/// 调用方无法执行的（审计 `LGC-LG-43`）。
#[test]
fn the_limit_contract_is_the_one_the_catalog_declares() {
    let root = tree("limits", 60, 1);
    let default = callgraph(&root, &json!({"function": "new"})).expect("the answer renders");
    assert!(default.contains("matches 60"), "{default}");
    assert!(
        default.contains("… truncated: 55 of 60 definitions withheld at the limit of 5"),
        "the default shows 5 of 60 definitions: {default}"
    );
    let clamped =
        callgraph(&root, &json!({"function": "new", "limit": 999})).expect("the answer renders");
    assert!(
        clamped.contains("… truncated: 10 of 60 definitions withheld at the limit of 50"),
        "a limit past the declared maximum is clamped to 50, not refused: {clamped}"
    );
    let zero =
        callgraph(&root, &json!({"function": "new", "limit": 0})).expect("the answer renders");
    assert!(
        zero.contains("… truncated: 59 of 60 definitions withheld at the limit of 1"),
        "a limit below the declared minimum behaves as 1: {zero}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The bodies are in the answer unless the caller asks for the short one.
/// 函数体默认就在答案里，除非调用方点名要短的那份。
///
/// Measured: every evaluation round spent one extra call after this tool had shown only names, so
/// the default is now the answer that needs no follow-up, and `source: false` is the opt-out that
/// says so in the reply.
/// 实测：评测每一轮都在这个工具只给了名字之后多花一次调用，因此默认改成"不需要补问"的那份答案，而
/// `source: false` 是那个会在回复里说明自己的退出选项。
/// The source block starts at the doc comment's first line, because that pairing is what lets a
/// reader catch a promise the implementation contradicts — the round-4 trap was decided by it.
/// 源码块从文档注释首行起打，因为正是这个并排让读者抓住"承诺与实现相矛盾"——第 4 题的陷阱就由它裁决。
#[test]
fn the_source_block_starts_at_the_docs_first_line() {
    let root = crate::mcp::tools::tools_tests::scratch_package("docline");
    let answer = callgraph(&root, &json!({"function": "used"})).expect("an answer");
    let source = answer
        .split("  source:\n")
        .nth(1)
        .expect("the bodies are included by default");
    let first = source.lines().next().unwrap_or_default();
    assert!(
        first.trim_start().starts_with("///"),
        "the doc rides along: {first:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_bodies_are_included_unless_asked_otherwise() {
    let root = tree("bodies", 1, 1);
    let full = callgraph(&root, &json!({"function": "new"})).expect("an answer");
    assert!(
        full.contains("source:"),
        "the bodies come by default: {full}"
    );
    let short = callgraph(&root, &json!({"function": "new", "source": false})).expect("an answer");
    assert!(!short.contains("  source:"), "{short}");
    assert!(short.contains("bodies omitted on request"), "{short}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The orphan view names what nothing here calls, and says what its count can and cannot see.
/// 孤儿视图点名这里没人调用的东西，并说清它的计数看得见什么、看不见什么。
#[test]
fn the_orphan_view_names_what_nothing_calls() {
    let root = crate::mcp::tools::tools_tests::scratch_package("orphans");
    // One test function, which no other function calls: the harness is its caller, so it belongs in
    // the count the view reports rather than in the list it prints.
    // 一个测试函数，没有任何函数调用它：测试框架才是它的调用者，因此它属于视图报出的那个**计数**，
    // 而不属于它打印的那张**清单**。
    std::fs::create_dir_all(root.join("tests")).expect("fixture test dir");
    std::fs::write(
        root.join("tests/orphans.rs"),
        "#[test]\nfn the_only_test() { assert!(fixture_orphans::used() == 1); }\n",
    )
    .expect("fixture test file");
    let answer = callgraph(&root, &json!({"orphans": true})).expect("an answer");
    assert!(answer.contains("fn    orphan ->"), "{answer}");
    assert!(
        !answer.contains("fn    used ->"),
        "`used` is called by `call_used`: {answer}"
    );
    assert!(
        answer.contains("static"),
        "the count says it is static: {answer}"
    );
    assert!(
        answer.contains("invisible to it"),
        "and says what it cannot see: {answer}"
    );
    assert!(
        !answer.contains("fn    the_only_test ->"),
        "a test function is not listed as an orphan: {answer}"
    );
    assert!(
        answer.contains("1 function(s) in test files have no static caller"),
        "but the test half is counted and named: {answer}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The rule "a call names a definition" is one rule: the bare name and the qualified path both
/// count, and a name the call does not end on at a `::` boundary does not.
/// "一次调用点名某个定义"是一条规则：裸名与限定路径都算，而调用并不在 `::` 边界上以其结尾的名字不算。
#[test]
fn a_call_names_a_definition_by_the_one_rule() {
    assert!(is_call_to("paint", "paint"));
    assert!(is_call_to("::paint", "paint"));
    assert!(is_call_to("control::paint", "paint"));
    assert!(is_call_to("crate::control::paint", "paint"));
    assert!(!is_call_to("repaint", "paint"));
    assert!(!is_call_to("paint_brush", "paint"));
    assert!(!is_call_to("paint", "pain"));
}

/// One implementation, three consumers: `orphans`, the census's test-reachability column and
/// `affected` all reach a definition through `is_call_to`, and none of them keeps a clause of its
/// own.
/// 一份实现、三个消费方：`orphans`、总账的测试可达性栏与 `affected` 都经 `is_call_to` 到达定义，谁也
/// 不留自己的子句。
///
/// This is the pin behaviour tests cannot give. Two byte-identical clauses behave identically until
/// one of them is edited, so a behaviour test stays green through exactly the change this refinement
/// exists to prevent; only reading the sources sees the copy. The counts are taken over every
/// shipping source file in this checkout and the body assertions are anchored to the three
/// consumers, so a fourth spelling — in any file, or text elsewhere in these files — fails here.
/// 这是行为测试给不出的钉子。两段逐字相同的子句在被改动之前行为一致，因此正是这次收敛要防的那种改动
/// 会让行为测试保持绿色；只有读源码才看得见那份副本。计数取自本检出**每一个出厂源码文件**，函数体断言
/// 锚定在三个消费方上，因此第四份拼写——不论出现在哪个文件、还是这些文件里别处的文本——都会在这里失败。
#[test]
fn every_view_reaches_a_definition_through_the_one_predicate() {
    let sources = shipped_rust_sources();
    let joined = sources
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        joined.matches("fn is_call_to(").count(),
        1,
        "the predicate is defined once, in the module that owns the call graph"
    );
    assert_eq!(
        joined.matches("strip_suffix(name)").count(),
        1,
        "the rule's body is spelled exactly once in this checkout's shipping sources"
    );
    for (file, consumer) in [
        ("toolchain/src/mcp/src/callgraph.rs", "fn orphan_answer"),
        ("toolchain/src/mcp/src/claims.rs", "fn reachability_column"),
        (
            "toolchain/src/mcp/src/affected.rs",
            "pub(crate) fn affected",
        ),
    ] {
        let source = sources
            .iter()
            .find(|(path, _)| path == file)
            .map(|(_, text)| text.as_str())
            .unwrap_or_else(|| panic!("{file} is part of this checkout"));
        let body = body_of(source, consumer);
        assert!(
            body.contains("is_call_to("),
            "{file} does not reach a definition through the shared predicate: {body}"
        );
        assert!(
            !body.contains("ends_with("),
            "{file} must not keep its own matching clause: {body}"
        );
    }
}

/// Every shipping Rust file in this checkout, as `(path relative to the checkout root, text)`.
/// 本检出里每一个出厂 Rust 文件，形如 `(相对检出根的路径, 文本)`。
///
/// Test files are left out on purpose: this rule is about the code that ships, and a test is
/// allowed to quote it. Collecting the paths (rather than `include_str!`-ing a hand-kept list) is
/// what makes the counts above cover the whole checkout instead of the files that happened to
/// consume the predicate the day this pin was written.
/// 有意排除测试文件：这条规则针对的是会出厂的代码，而测试允许引用它。收集路径（而不是把一份手工清单
/// `include_str!` 进来）正是让上面的计数覆盖**整个检出**、而不是"写下这条钉子那天恰好用到该判据的那几个
/// 文件"的原因。
fn shipped_rust_sources() -> Vec<(String, String)> {
    // The verb table spells a private recursive helper as `visit_` (D-5): `walk` is a bare verb,
    // and `collect_` on a private function is the table's `visit_`.
    // 动词表把私有的递归辅助函数拼作 `visit_`（D-5）：`walk` 是裸动词，而私有函数上的 `collect_` 正是
    // 表里叫作 `visit_` 的那种东西。
    fn visit_sources(root: &Path, directory: &Path, found: &mut Vec<(String, String)>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit_sources(root, &path, found);
                continue;
            }
            let spelling = path.to_string_lossy();
            let is_rust = path.extension().and_then(|ext| ext.to_str()) == Some("rs");
            if !is_rust || spelling.ends_with("_tests.rs") || spelling.contains("/tests/") {
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .map(|relative| relative.to_string_lossy().into_owned())
                .unwrap_or_default();
            found.push((relative, std::fs::read_to_string(&path).unwrap_or_default()));
        }
    }
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("this crate lives inside the checkout")
        .to_path_buf();
    let mut found = Vec::new();
    for member in ["kernel", "toolchain", "macro", "conventions", "examples"] {
        visit_sources(&checkout, &checkout.join(member).join("src"), &mut found);
    }
    found
}

/// The source of one top-level function: from the line that declares it to the closing brace at
/// column zero, so a body assertion cannot be satisfied by text elsewhere in the file.
/// 一个顶层函数的源码：从声明它的那一行到列零处的收尾花括号——这样函数体断言不会被文件里别处的文本
/// 满足。
fn body_of<'a>(source: &'a str, declaration: &str) -> &'a str {
    let start = source
        .find(declaration)
        .expect("the declaration is in the file");
    let rest = &source[start..];
    let end = rest.find("\n}\n").map_or(rest.len(), |end| end + 2);
    &rest[..end]
}
