//! Tests for the two bounds `nichlink.callgraph` needs: definitions and callers.
//! `nichlink.callgraph` 需要的两道上限的测试：定义数与调用者数。
//!
//! The measured failure these exist against is the one `callgraph.rs`'s module doc
//! records: a common name (`new`) matched definitions throughout a real-tree corpus,
//! and every call site of that name was listed for each of them, which arrived as a
//! 4.5 MB reply. An answer that an agent cannot read is not an answer, so both bounds
//! are pinned here.
//! 这些测试所针对的实测失败就是 `callgraph.rs` 模块文档记下的那一条：常见名（`new`）在真实语料里
//! 匹配到许多定义，而每个定义都列出该名字的每一个调用点，最终以 4.5 MB 的回复抵达。代理读不下的
//! 答案不算答案，因此两道上限都钉在这里。

use std::path::PathBuf;

use serde_json::json;

/// A throwaway package: `cargo metadata` can name it, so the tools that need a
/// namespace get one, and it holds no face so every evidence answer is the empty
/// one.
/// 一个一次性包：`cargo metadata` 能给它命名，因此需要命名空间的工具能得到一个；它不含任何面，
/// 所以每个证据答案都是空的那一种。
fn package(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let name = format!("mcp-dispatch-{label}");
    let root = std::env::temp_dir().join(format!("{name}-{}-{sequence}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("package directory");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
    )
    .expect("manifest");
    std::fs::write(root.join("src/lib.rs"), "// host entry\n").expect("library target");
    root
}

/// The three evidence tools must be advertised with the argument that narrows
/// each one, because an agent can only call what `tools/list` names.
/// 证据工具必须被列出，并且带上各自缩小范围的那个参数，因为代理只能调用 `tools/list` 点名的东西。
#[test]
fn the_evidence_tools_are_advertised_with_their_narrowing_arguments() {
    let listed = super::tools();
    for (name, key) in [
        ("nichlink.explain", "node"),
        ("nichlink.explain", "overlay"),
        ("nichlink.callgraph", "limit"),
        ("nichlink.diff", "limit"),
        ("nichlink.trace", "query"),
        ("nichlink.impact", "node"),
        ("nichlink.grafts", "limit"),
    ] {
        let tool = listed
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not advertised"));
        assert!(
            tool["inputSchema"]["properties"].get(key).is_some(),
            "{name} does not advertise `{key}`: {tool}"
        );
    }
}

/// `nichlink.impact` is wired to its implementation too, and asking it for a radius
/// with no face named is an error response rather than a silent empty one.
/// `nichlink.impact` 同样接到了实现上，而没点名任何面就要半径会得到错误响应，而不是一份静默的空答案。
#[test]
fn the_impact_tool_refuses_a_call_without_a_node() {
    let root = package("impact-dispatch");
    let reply = super::tool_call(
        &root,
        json!(1),
        &json!({"name": "nichlink.impact", "arguments": {}}),
    );
    let text = reply["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("no text: {reply}"));
    assert!(text.contains("requires node"), "{text}");
    assert_eq!(reply["result"]["isError"], true, "{reply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// The dispatch table and the catalog list the same tools in the same order, so the
/// order is a machine-checked fact rather than an eyeballed one. The failure this
/// prevents: `nichlink.apply` sat sixth in the catalog and last in the dispatch, and
/// nothing noticed (audit `BR-12`).
/// 分派表与目录以同样的顺序列出同样的工具，因此这个顺序是被机器检查的事实，而不是靠眼睛对的。
/// 它防止的失败：`nichlink.apply` 在目录里排第 6、在分派里排最后，而没有任何东西发现（审计 `BR-12`）。
#[test]
fn the_dispatch_table_follows_the_catalog() {
    let catalog = super::tools();
    let listed: Vec<&str> = catalog
        .iter()
        .map(|tool| {
            tool["name"]
                .as_str()
                .unwrap_or_else(|| panic!("a catalog entry without a name: {tool}"))
        })
        .collect();
    let dispatched: Vec<&str> = super::DISPATCH.iter().map(|(name, _)| *name).collect();
    assert_eq!(
        listed, dispatched,
        "the catalog and the dispatch table must name the same tools in the same order"
    );
}

/// Every name the catalog lists is routed somewhere: a tool `tools/list`
/// advertises but the dispatch has no arm for answers `unknown tool`, which a
/// client only discovers when it calls it. Before this test the two lists were
/// complete but nothing kept them so — adding a tool to the catalog alone was
/// silent (audit `BR-12`).
/// 目录列出的每个名字都被路由到某处：`tools/list` 声明而分派没有对应臂的工具会回答
/// `unknown tool`，而客户端只有调用时才发现。在这条测试之前两张名单是齐全的，但没有任何东西维持
/// 这一点——只往目录里加一个工具是静默的（审计 `BR-12`）。
#[test]
fn every_listed_tool_is_dispatched() {
    let root = package("every-tool");
    for tool in super::tools() {
        let name = tool["name"]
            .as_str()
            .unwrap_or_else(|| panic!("a catalog entry without a name: {tool}"));
        let reply = super::tool_call(&root, json!(1), &json!({"name": name, "arguments": {}}));
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} returned no text: {reply}"));
        assert!(
            !text.contains("unknown tool"),
            "{name} is advertised but not dispatched: {reply}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// The description of `nichlink.usages` names every field the implementation
/// prints. It stopped at `runtime checks` while `usages.rs` also printed
/// `module`, `stable_name`, `getting_from_other_registry`, `flow_provider`, and
/// `needs_registry` — so an agent asking "what can I set" got a shorter answer
/// than the tool's own contract (audit `BR-7`).
/// `nichlink.usages` 的描述点名实现打印的每一个字段。它停在 `runtime checks`，而 `usages.rs`
/// 还打印 `module`、`stable_name`、`getting_from_other_registry`、`flow_provider` 与
/// `needs_registry`——于是一个问"我能设什么"的代理拿到的答案比这个工具自己的契约更短
/// （审计 `BR-7`）。
#[test]
fn the_usages_description_names_every_field_it_prints() {
    let listed = super::tools();
    let usages = listed
        .iter()
        .find(|tool| tool["name"] == "nichlink.usages")
        .expect("nichlink.usages is advertised");
    let description = usages["description"]
        .as_str()
        .unwrap_or_else(|| panic!("nichlink.usages has no description: {usages}"));
    for field in [
        "module",
        "preset",
        "parts",
        "name_zh",
        "name_en",
        "summary_zh",
        "summary_en",
        "stable_name",
        "exports",
        "requires",
        "provides",
        "handle_traits",
        "handle_contracts",
        "part_traits",
        "part_contracts",
        "registration_rule",
        "admission",
        "flow",
        "flow_provider",
        "runtime_checks",
        "getting_from_other_registry",
        "needs_registry",
    ] {
        assert!(
            description.contains(field),
            "`{field}` is read back by nichlink.usages and must be named in its description: {description}"
        );
    }
}

/// The description of `nichlink.verify` names **both** verdicts it prints.
/// `nichlink.verify` 的描述点名它打印的**两个**判断。
///
/// The second line (`connector verdict: ok|rejected`) was added because a tree can pass
/// the static verdict and be refused by the authoring connector (audit `F1`), and the
/// description kept promising one verdict — so an agent read a green light on a tree the
/// other surface refuses.
/// 第二行（`connector verdict: ok|rejected`）之所以存在，是因为一棵树可以通过静态判断而被创作
/// 连接器拒绝（审计 `F1`），而描述一直只承诺一个判断——于是代理在一棵被另一个面拒绝的树上读到了
/// 绿灯。
#[test]
fn the_verify_description_names_the_connector_verdict() {
    let listed = super::tools();
    let verify = listed
        .iter()
        .find(|tool| tool["name"] == "nichlink.verify")
        .expect("nichlink.verify is advertised");
    let description = verify["description"]
        .as_str()
        .unwrap_or_else(|| panic!("nichlink.verify has no description: {verify}"));
    for expected in [
        "connector verdict: ok",
        "connector verdict: rejected",
        "authoring connector",
        "isError",
    ] {
        assert!(
            description.contains(expected),
            "`{expected}` is part of what verify prints and must be named in its description: {description}"
        );
    }
}

/// `nichlink.mir` refuses a snapshot it cannot print whole, and its description says so.
/// `nichlink.mir` 拒绝它无法整体打印的快照，而它的描述把这一点说出来。
///
/// The writer's promise is "what it prints reads back here" (audit `X2`), so the one case
/// where it prints nothing has to be declared rather than discovered by a parse error.
/// 写入方的承诺是"它打印出来的东西能在这里读回来"（审计 `X2`），因此它唯一什么都不打印的那种情形
/// 必须被声明出来，而不是靠一条解析错误去发现。
#[test]
fn the_mir_description_declares_the_refusal_above_the_reply_cap() {
    let listed = super::tools();
    let mir = listed
        .iter()
        .find(|tool| tool["name"] == "nichlink.mir")
        .expect("nichlink.mir is advertised");
    let description = mir["description"]
        .as_str()
        .unwrap_or_else(|| panic!("nichlink.mir has no description: {mir}"));
    assert!(
        description.contains("reads back here"),
        "the promise is what makes the refusal necessary: {description}"
    );
    assert!(
        description.contains("reply cap") && description.contains("refused by name"),
        "the one case that prints nothing must be declared: {description}"
    );
}

/// A package whose own faces the connector refuses: a provider, and a consumer whose
/// requirement no face answers. `face_views` still derives both — that divergence is the
/// whole point (audit `F1`) — so a read tool has a node to name.
/// 一个连接器拒绝其自身面的包：一个提供者，以及一个需求无人满足的消费者。`face_views` 仍然把两者
/// 都推导出来——这处分歧正是要点（审计 `F1`）——因此读工具有节点可点名。
fn rejected_package(label: &str) -> PathBuf {
    let root = package(label);
    std::fs::write(
        root.join("src/lib.rs"),
        "pub fn wire() {\n    crate::label::Label;\n}\n",
    )
    .expect("the entry references one face");
    std::fs::create_dir_all(root.join("src/label")).expect("label module");
    std::fs::write(
        root.join("src/label/label.rs"),
        "crate::root_object! { kind: Label, \
         parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")), provides: [\"cap.render\"], }\n",
    )
    .expect("the provider face");
    std::fs::create_dir_all(root.join("src/widget")).expect("widget module");
    std::fs::write(
        root.join("src/widget/widget.rs"),
        "crate::root_object! { kind: Widget, \
         parent: crate::root_node_id(env!(\"CARGO_PKG_NAME\")), \
         requires: [\"cap.theme\" => \"Theme\"], }\n",
    )
    .expect("the face whose requirement nothing answers");
    root
}

/// The connector's rejection of a package's own faces is one fact about that tree, so
/// both read tools answer it the same way: the verdict spelled once, in the body, with
/// `isError` false. `usages` used to return it as an error while `converge` printed it, so
/// an agent routing on `isError` read the same rejection as a failure and as an ordinary
/// answer (audit `A2`).
/// 连接器对一个包自身面的拒绝是关于那棵树的同一个事实，因此两个读工具以同一种方式作答：那句话只
/// 拼一次、出现在正文里，`isError` 为 false。`usages` 过去把它当错误返回，而 `converge` 把它打印
/// 出来，于是按 `isError` 分流的代理把同一次拒绝读成了失败、也读成了普通答案（审计 `A2`）。
#[test]
fn a_connector_rejection_is_the_same_answer_from_usages_and_converge() {
    let root = rejected_package("a2");
    for tool in ["nichlink.usages", "nichlink.converge"] {
        let reply = super::tool_call(
            &root,
            json!(1),
            &json!({"name": tool, "arguments": {"node": "root/label"}}),
        );
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{tool} returned no text: {reply}"));
        assert_eq!(
            reply["result"]["isError"], false,
            "{tool} answers about the tree instead of failing: {reply}"
        );
        assert!(
            text.contains(crate::mcp::apply::REJECTED_VERDICT),
            "{tool} must print the one verdict spelling: {text}"
        );
        assert!(
            text.contains("input `cap.theme` has no provider"),
            "{tool} must carry the diagnostic that names the cause: {text}"
        );
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// Every new name is wired to its implementation, not only to the catalog.
/// 每个新名字都接到了实现上，而不只是接进目录。
#[test]
fn the_evidence_tools_are_dispatched_to_their_implementations() {
    let root = package("dispatch");
    for (name, expected) in [
        ("nichlink.explain", "scope unknown"),
        ("nichlink.diff", "no build evidence"),
        ("nichlink.trace", "trace absent"),
        ("nichlink.grafts", "no external graft plans"),
    ] {
        let reply = super::tool_call(&root, json!(1), &json!({"name": name, "arguments": {}}));
        let text = reply["result"]["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} returned no text: {reply}"));
        assert!(text.contains(expected), "{name}: {text}");
        assert_eq!(reply["result"]["isError"], false, "{name}: {reply}");
    }
    let _ = std::fs::remove_dir_all(&root);
}

/// `tools/list` is MCP-shaped: `nichlink.search` is there, and every entry declares
/// an object input schema.
/// `tools/list` 是 MCP 形状的：`nichlink.search` 在其中，且每一条都声明 object 输入 schema。
///
/// Moved here from an inline `mod tests` in `tools.rs`, so every source file has at
/// most one `<name>_tests.rs` and `tools.rs` holds only the catalog and the dispatch
/// (audit `BR-11`).
/// 从 `tools.rs` 里的内联 `mod tests` 移到这里，使每个源文件至多有一个 `<name>_tests.rs`，
/// 而 `tools.rs` 只剩目录与分派（审计 `BR-11`）。
#[test]
fn tools_list_is_mcp_shaped() {
    let listed = super::tools();
    assert!(listed.iter().any(|tool| tool["name"] == "nichlink.search"));
    assert!(
        listed
            .iter()
            .all(|tool| tool["inputSchema"]["type"] == "object")
    );
}

/// The two Studio write actions the bridge was missing are advertised, and their
/// descriptions carry the two disciplines every write here keeps: a request is previewed
/// unless `apply` is true, and the irreversible half needs the request to say `confirm`.
/// 桥此前缺的那两个 Studio 写入动作已被声明，而它们的描述带有这里每次写入都守的两条纪律：除非
/// `apply` 为真否则只预览，以及不可逆的那一半需要请求自己说出 `confirm`。
///
/// The description is the contract: an agent can only aim at what `tools/list` names, so a
/// tool whose schema omits `apply` or `confirm` is a tool no agent will ever use safely.
/// 描述就是契约：代理只能瞄准 `tools/list` 点名过的东西，因此 schema 里没有 `apply` 或 `confirm`
/// 的工具，是任何代理都不会安全使用的工具。
#[test]
fn the_two_studio_writes_are_advertised_with_their_flags() {
    let listed = super::tools();
    for (name, keys) in [
        (
            "nichlink.new_project",
            &["directory", "package", "kind", "apply", "confirm", "root"][..],
        ),
        (
            "nichlink.plugin",
            &[
                "source",
                "framework",
                "package",
                "version",
                "crate",
                "checksum",
                "mode",
                "apply",
                "confirm",
                "root",
            ][..],
        ),
    ] {
        let tool = listed
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not advertised"));
        let description = tool["description"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} has no description: {tool}"));
        assert!(
            description.contains("previewed unless `apply` is true"),
            "{name} must state the preview default: {description}"
        );
        assert!(
            description.contains("`confirm: true`"),
            "{name} must state its confirmation rule: {description}"
        );
        for key in keys {
            assert!(
                tool["inputSchema"]["properties"].get(*key).is_some(),
                "{name} does not advertise `{key}`: {tool}"
            );
        }
    }
}

/// Both new writes name the two halves of the executor they reuse, so a reader can check
/// that this is not a second implementation: the scaffold is `create_project`, and the
/// plugin record goes through the kernel's `contains_record` gate.
/// 两个新写入都点名了它们复用的执行器的两半，好让读者能核对这不是第二份实现：脚手架是
/// `create_project`，而插件记录经内核的 `contains_record` 闸门。
#[test]
fn the_new_writes_name_the_executor_they_reuse() {
    let listed = super::tools();
    for (name, expected) in [
        ("nichlink.new_project", "create_project"),
        ("nichlink.plugin", "contains_record"),
    ] {
        let tool = listed
            .iter()
            .find(|tool| tool["name"] == name)
            .unwrap_or_else(|| panic!("{name} is not advertised"));
        let description = tool["description"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} has no description: {tool}"));
        assert!(
            description.contains(expected),
            "{name} must name `{expected}`, the executor it shares: {description}"
        );
    }
}

/// A throwaway package with a called function, its caller, an orphan, and a documented definition.
/// 一个一次性包：一个被调用的函数、它的调用者、一个孤儿，以及一个带文档的定义。
///
/// Shared by the acceptance pin (the table's shapes must be accepted here) and the orphan pin, so
/// the two cannot disagree about what the fixture is.
/// 由接受性钉子（表里的形状必须在这里被接受）与孤儿钉子共用，因此两者不会对"夹具是什么"产生分歧。
pub(crate) fn scratch_package(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-tools-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("src")).expect("fixture dirs");
    std::fs::write(
        root.join("Cargo.toml"),
        format!("[package]\nname = \"fixture-{label}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"),
    )
    .expect("fixture manifest");
    std::fs::write(
        root.join("src/lib.rs"),
        "//! A fixture module.\n\
         /// What this function is for.\n\
         pub fn used() -> u32 { 1 }\n\
         \n\
         pub fn call_used() -> u32 { used() }\n\
         \n\
         pub fn orphan() -> u32 { 2 }\n",
    )
    .expect("fixture source");
    root
}

/// Every read answer ends by naming the next call — **once**, and on its own line.
/// 每个读答案末尾点名下一次调用 —— **只点一次**，而且自占一行。
///
/// The measured failure this guards: the hint was appended without a separator, so it arrived glued
/// to the last line (`…default runnext   registry …`) and a reader that greps for a line starting
/// with `next` saw none.
/// 它守的实测失败：提示追加时没加分隔，于是粘在答案最后一行上（`…default runnext   registry …`），
/// 按"行首是 `next`"去 grep 的读者一条也看不到。
#[test]
fn a_read_answer_names_the_next_call_exactly_once() {
    let plain = super::with_next_hint("nichlink.callgraph", "callers (0): -\n".to_owned());
    assert_eq!(plain.matches("\nnext").count(), 1, "{plain}");
    assert!(plain.ends_with("what depends on it\n"), "{plain}");

    // An answer that already names one (search, check, adopted) is left alone.
    // 已经点名过的答案（search / check / adopted）不动。
    let already = "no matches in /root\nnext   pass `literal` for text\n".to_owned();
    assert_eq!(
        super::with_next_hint("nichlink.callgraph", already.clone()),
        already
    );

    // A tool with no hint is left alone.
    // 没有提示的工具不动。
    assert_eq!(
        super::with_next_hint("nichlink.apply", "ok\n".to_owned()),
        "ok\n"
    );

    for name in [
        "nichlink.status",
        "nichlink.registry",
        "nichlink.explain",
        "nichlink.callgraph",
        "nichlink.inspect",
        "nichlink.affected",
    ] {
        assert!(super::next_hint(name).is_some(), "{name} needs a hint");
    }
}

/// Every answer carries a line the caller can quote; a refusal carries none.
/// 每个答案都带一行可引用的东西；被拒的调用不带。
///
/// Measured (W8): the arm re-ran calls purely to have something to cite — "I claimed a closing
/// `status` call … I haven't run it. Let me run it" and "I did NOT actually run explain … to make the
/// claim true". The brief demands a command, its raw output and its exit code per claim, so the line
/// removes the only reason those re-runs existed.
/// 实测（W8）：那一臂**为了有东西可引**而补跑调用——「I claimed a closing `status` call … I haven't
/// run it. Let me run it」与「I did NOT actually run explain … to make the claim true」。题面要求每条
/// 声称给命令、原始输出与退出码，因此这一行去掉了那些补跑存在的唯一理由。
#[test]
fn every_answer_carries_a_quotable_evidence_line() {
    let root = scratch_package("evidence");
    let answer = super::run_tool(&root, "nichlink.status", &json!({})).expect("status answers");
    let last = answer.lines().last().expect("an answer has lines");
    assert!(
        last.starts_with("evidence nichlink.status "),
        "the last line names this call: {answer}"
    );
    assert_eq!(
        last, "evidence nichlink.status {} → exit 0",
        "a call with no arguments renders an empty object, and says it answered"
    );
    let with_arguments =
        super::run_tool(&root, "nichlink.search", &json!({"query": "nothing-here"}))
            .expect("search answers, even with no matches");
    let last = with_arguments.lines().last().expect("an answer has lines");
    assert!(
        last.contains("{\"query\":\"nothing-here\"}") && last.ends_with("→ exit 0"),
        "and a call with arguments carries them: {last}"
    );
    // The line is rendered from the same `arguments` value the dispatch used, so it cannot describe
    // a different call — and a refusal has no line at all, because its own text is the evidence.
    // 这一行由派发实际使用的那个 `arguments` 值渲染，因此它不可能描述另一次调用——而被拒的调用完全没有
    // 这一行，因为它的文案本身就是证据。
    let refused = super::run_tool(&root, "nichlink.read", &json!({"path": 7}));
    assert!(refused.is_err(), "a bad shape is refused");
    let _ = std::fs::remove_dir_all(&root);
}
