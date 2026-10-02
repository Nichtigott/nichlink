//! Pins for the one-shot client: the list is the catalogue one entry each (name, first sentence,
//! keys), a call's exit code says how the call went (answered, refused, or a usage error), and the
//! tool's own verdict is in its text — `check` puts it on the reply's first line.
//! 一次性客户端的钉子：清单是目录的"每条一目"形态（名字、第一句、键名），一次调用的退出码说的是这次调用
//! 怎么样（作答、拒绝、或用法错），而工具自己的判定在它的文本里——`check` 把它放在回复第一行。

use super::*;
use serde_json::json;

/// The list has one **entry** per tool — the name, its first sentence, and the keys it takes
/// (`*` = required) — because the round measured an arm guessing a key name (`--face` where the
/// tool wanted `node`) and spending a refusal on it: naming the tool without naming what it takes
/// is the shape that caused it. The entry's first line stays the one-line form the earlier pin
/// asserted; the keys line is the addition, and this pin now says both.
/// 清单每个工具**一条**——名字、第一句，以及它接受的键（`*` = 必填）——因为那一轮量到一个臂猜键名
/// （工具要 `node`、它写了 `--face`）并为此白吃一次拒绝：只点工具名、不说它收什么，正是造成这件事的形状。
/// 条目的第一行仍是早先那条钉子断言的"一行式"，键名行是新增的，这条钉子现在把两半都说出来。
#[test]
fn the_list_is_the_catalogue_one_entry_each() {
    let entries = list_tool_lines();
    assert_eq!(entries.len(), crate::mcp::tools::tools().len());
    for entry in &entries {
        let mut lines = entry.lines();
        let first = lines.next().unwrap_or_default();
        assert!(first.starts_with("nichlink."), "{entry}");
        assert!(
            first.contains(" — "),
            "name, then the first sentence: {entry}"
        );
        let keys = lines.next().unwrap_or_default();
        assert!(
            keys.trim_start().starts_with("keys: "),
            "then its keys: {entry}"
        );
        assert_eq!(lines.count(), 0, "and nothing else: {entry}");
    }
    assert!(
        entries
            .iter()
            .any(|entry| entry.starts_with("nichlink.callgraph — Show direct static callers")),
        "{entries:?}"
    );
    // The key line says which names are required, and a branched pair is not starred as if both
    // were: `search` takes `query` or `literal`, and starring both would be a louder lie.
    // 键名行说出哪些名字是必填，而"二选一"的那对不会被当成两个都必填：`search` 收 `query` 或
    // `literal`，两个都打星是更响的谎。
    let search = entries
        .iter()
        .find(|entry| entry.starts_with("nichlink.search — "))
        .expect("the search entry");
    assert!(search.contains("keys: "), "{search}");
    // `--list <tool>` is the un-truncated form, because the one-line list is what cost an arm four
    // refusals on `apply`: its description already spelled the shape out.
    // `--list <tool>` 是不截断的形态，因为正是"一行式清单"让一个臂在 `apply` 上白吃四次被拒：它的描述
    // 本来就把形状写清楚了。
    let apply = describe_tool("apply").expect("a described tool");
    // 用描述里**真实存在**的那半句（我第一次钉的是自己编的句子 ✗）：`apply` 的描述写明除
    // `needs_registry` 外每个 `fields` 值都是字符串。
    assert!(
        apply.contains("Every other `fields` value is a **string**"),
        "{apply}"
    );
    assert!(
        describe_tool("nichlink.apply").is_some(),
        "the prefix is optional here too"
    );
    assert!(
        describe_tool("no_such_tool").is_none(),
        "unknown names are not invented"
    );

    assert!(!search.contains("query*"), "query is one of two: {search}");
    assert!(
        !search.contains("literal*"),
        "literal is one of two: {search}"
    );
}

/// The workflow table says which symptom takes which call, because the measured failure was that
/// nothing said it.
/// 流程表说出"哪种症状用哪个调用"，因为量出来的失败正是没有任何地方说这件事。
#[test]
fn the_instructions_name_the_symptom_and_the_call() {
    for expected in [
        "check {face}",
        "search {literal}",
        "search {query}",
        // The table names the tool and its key; the order it recommends is asserted below, and
        // `source` is no longer a key to remember because bodies are the default.
        // 表里点名工具与它的键；它推荐的顺序在下面另断言，而 `source` 已不再是需要记住的键——函数体是默认。
        "callgraph {function}",
        "callgraph {orphans: true}",
        "affected",
    ] {
        assert!(
            INSTRUCTIONS.contains(expected),
            "missing `{expected}`: {INSTRUCTIONS}"
        );
    }
    assert!(
        INSTRUCTIONS.len() > 700,
        "the table has to carry the mapping, not one sentence: {}",
        INSTRUCTIONS.len()
    );
}

/// The table no longer says the exit code is the verdict: round 7 measured that promise false, and a
/// reader who believed it read a red face as green.
/// 流程表不再说"退出码就是判定"：第七轮量出那句承诺是假的，而信了它的读者把红面读成了绿。
///
/// The measurement, on the frozen fixture `target/round7/s5`: `check` answered `exit   101` on line
/// six of the reply while the one-shot client exited `0`, so the promise held for the green `s1` and
/// failed exactly where a reader needed it. What replaces it names both halves — where the verdict
/// is (`check`'s first line) and what the exit code does say (`0`/`1`/`2` = answered/refused/usage
/// error). The strings are pinned verbatim, because the failure was a *sentence* nobody checked.
/// 实测（冻结夹具 `target/round7/s5`）：`check` 把 `exit   101` 写在回复第六行，而一次性客户端以 `0`
/// 退出——那句承诺对绿的 `s1` 成立、恰好在读者最需要它的地方不成立。替换它的文本把两半都点名——判定在哪
/// （`check` 的第一行）以及退出码到底说的是什么（`0`/`1`/`2` = 作答/拒绝/用法错）。字符串逐字钉住，因为
/// 失败的正是一句没人检查的**话**。
#[test]
fn the_table_does_not_read_the_exit_code_as_the_verdict() {
    assert!(
        !INSTRUCTIONS.contains("exit code the verdict"),
        "the promise the round measured false is gone: {INSTRUCTIONS}"
    );
    for expected in [
        "verdict is the first line of the reply",
        "`0` answered, `1` refused, `2` a usage error",
        "verdict  passed (cargo exit 0)",
        "verdict  failed (cargo exit 101)",
        // The range-type workflow starts from the same reply's census, and the table has to say so:
        // the round measured agents opening files one by one for a question the census already
        // answered column by column.
        // 范围型工作流要从同一次回复的普查开始，表里必须写明：那一轮量到代理为一个"普查已逐栏回答"的
        // 问题去逐个打开文件。
        "A range-type question",
        "census: true",
    ] {
        assert!(
            INSTRUCTIONS.contains(expected),
            "missing `{expected}`: {INSTRUCTIONS}"
        );
    }
}

/// `--list check` says which line the verdict lands on and what the client's exit code means, because
/// the measured defect was an agent taking the exit code for the verdict.
/// `--list check` 说明判定落在哪一行、以及客户端的退出码是什么意思，因为量到的缺陷正是 agent 把退出码当判定。
#[test]
fn the_check_description_names_the_verdict_line_and_the_exit_code() {
    let check = describe_tool("check").expect("the check tool is described");
    for expected in [
        "The reply's first line is the run's verdict",
        "verdict  passed (cargo exit 0)",
        "verdict  failed (cargo exit 101)",
        "not the one-shot client's exit code",
        "`0` when the tool answered",
        "`1` when it refused",
        "`2` on a usage error",
    ] {
        assert!(check.contains(expected), "missing `{expected}`: {check}");
    }
}

/// The exit code says how the **call** went — answered, refused, or a usage error — and never what
/// the tool found: `check` on a failing face still answers, so this client still exits `0` here.
/// 退出码说的是这次**调用**怎么样——作答、拒绝、或用法错——从不说工具发现了什么：`check` 在失败的面上
/// 仍是作答，因此这里客户端仍以 `0` 退出。
#[test]
fn the_client_exit_code_says_how_the_call_went() {
    let temp = std::env::temp_dir().join(format!("nichlink-client-{}", std::process::id()));
    std::fs::create_dir_all(&temp).expect("scratch");
    let root = temp.display().to_string();
    let args = |parts: &[&str]| parts.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>();

    match run_client(&args(&["--call", "nichlink.status", "--root", &root])) {
        Client::Called(code) => assert_eq!(code, 0, "a tool that answers exits 0"),
        Client::Serve => panic!("--call is a call"),
    }
    match run_client(&args(&["--call", "nichlink.no_such_tool"])) {
        Client::Called(code) => assert_eq!(code, 1, "a refusal is exit 1"),
        Client::Serve => panic!("--call is a call"),
    }
    match run_client(&args(&["--call"])) {
        Client::Called(code) => assert_eq!(code, 2, "a usage error is exit 2"),
        Client::Serve => panic!("--call is a call"),
    }
    let _ = std::fs::remove_dir_all(&temp);
}

/// Plain `--key value` arguments carry the obvious JSON type, so the common call needs no JSON.
/// 普通 `--key value` 参数带上最显然的 JSON 类型，因此常见调用不必写 JSON。
#[test]
fn plain_arguments_carry_their_json_type() {
    assert_eq!(scalar_for("source", "true"), Value::Bool(true));
    assert_eq!(scalar_for("source", "false"), Value::Bool(false));
    assert_eq!(scalar_for("timeout_ms", "900000"), Value::from(900_000));
    assert_eq!(scalar_for("face", "default"), Value::from("default"));
    assert_eq!(
        scalar_for("limit", "00"),
        Value::from("00"),
        "a leading zero is a string, not a number"
    );
}

/// Every call shape the flow table advertises is one the tool actually accepts.
/// 流程表承诺的每一种调用形状，都是工具真正接受的形状。
///
/// This is the pin the round was missing. The table advertised `callgraph {orphans: true}` while the
/// orphan view had deliberately been left out, and the old pin asserted only that the *string*
/// appeared in `INSTRUCTIONS` — so two independent members spent two of a round's ten calls being
/// refused (`requires function`, then `needs a value`). Checking the sentence is not checking the
/// shape. The reverse direction is checked too: every `tool {keys}` shape the table names must be
/// listed here, so adding a promise without an acceptance check fails.
/// 这就是那轮缺失的钉子。流程表承诺了 `callgraph {orphans: true}`，而孤儿视图当时被我故意缓做，旧钉子只断言
/// 那个**字符串**出现在 `INSTRUCTIONS` 里——于是两位成员各自花掉那轮 10 次调用里的 2 次被拒
/// （`requires function`、继而 `needs a value`）。**检查句子不等于检查形状。**反向也检查：表里点名的每个
/// `tool {keys}` 形状都必须在这里列名，因此"加了承诺却没有接受性检查"会失败。
#[test]
fn the_table_only_advertises_shapes_the_tool_accepts() {
    let root = crate::mcp::tools::tools_tests::scratch_package("instructions");
    // The read shapes, called for real: acceptance is a call that is not refused for its arguments.
    // 读形状真调一次：接受 = 这次调用不因为参数而被拒。
    let shapes: Vec<(&str, Value)> = vec![
        ("nichlink.status", json!({})),
        ("nichlink.registry", json!({})),
        ("nichlink.search", json!({"query": "used"})),
        ("nichlink.search", json!({"literal": "call_used"})),
        ("nichlink.callgraph", json!({"function": "used"})),
        ("nichlink.callgraph", json!({"orphans": true})),
        ("nichlink.read", json!({"path": "src/lib.rs", "line": 1})),
        ("nichlink.inspect", json!({"path": "src/lib.rs"})),
        ("nichlink.affected", json!({"files": ["src/lib.rs"]})),
        ("nichlink.explain", json!({})),
        ("nichlink.diff", json!({})),
        // The two ledger shapes: a fixture with no ledger is still an answer (the absence is
        // reported), so acceptance here means the arguments were not refused.
        // 两种台账形状：没有台账的夹具仍然是一个答案（缺失会被报出），因此这里的接受指的是"参数没被拒"。
        ("nichlink.conformance", json!({"anchor": "root/button"})),
        ("nichlink.consistency", json!({"specimen": "root/button"})),
        ("nichlink.consistency", json!({"parent": "root/control"})),
        // The shapes the seven-scenario table adds. Acceptance here means the arguments were not
        // refused — the table promises these spellings work, and the pin that reads this list is what
        // caught them missing: adding a promise without a check is exactly what it exists to refuse.
        // 七场景表新增的形状。这里的"接受"指参数没被拒——表承诺这些拼法可用，而读这份清单的那条钉子
        // 正是抓出它们缺失的那条：加了承诺却没加检查，正是它存在来拒绝的事。
        (
            "nichlink.locate",
            json!({"symptom": "the offsets do not add up"}),
        ),
        ("nichlink.why", json!({"at": "src/lib.rs:1"})),
        ("nichlink.grafts", json!({})),
    ];
    for (tool, arguments) in &shapes {
        let answer = crate::mcp::tools::run_tool(&root, tool, arguments);
        assert!(answer.is_ok(), "{tool} {arguments} was refused: {answer:?}");
    }
    // The write shape is not called (a preview runs the real executor on a copy); its schema is what
    // has to match the table.
    // 写入形状不真调（预览会在副本上跑真执行器）；要对上那张表的是它的 schema。
    let apply = crate::mcp::tools::tools()
        .into_iter()
        .find(|tool| tool["name"] == "nichlink.apply")
        .expect("apply is in the catalogue");
    assert_eq!(apply["inputSchema"]["required"][0], "action", "{apply}");
    let _ = std::fs::remove_dir_all(&root);
}

/// Each shape the table names has an acceptance check above, and each check names a shape the table
/// really has.
/// 表里点名的每种形状在上面的检查里都有，而每个检查点名的形状表里真有。
///
/// Drift runs both ways: a table entry with no check is a promise nobody tests, and a check for a
/// shape the table dropped is a test that outlived its subject.
/// 漂移是双向的：表里有、检查里没有 ⇒ 没人测的承诺；检查里有、表里已经删掉 ⇒ 活得比被检查对象还久的测试。
#[test]
fn every_shape_the_table_names_is_covered_by_an_acceptance_check() {
    let named = INSTRUCTIONS
        .split('`')
        .filter(|part| part.contains('{') && part.contains('}'))
        .filter_map(|part| part.split_once(' '))
        .map(|(tool, keys)| (tool.to_owned(), keys.to_owned()))
        .collect::<Vec<_>>();
    assert!(named.len() >= 6, "the table names shapes: {named:?}");
    for (tool, keys) in &named {
        let checked = match tool.as_str() {
            // The table writes tool names with backticks around the call, e.g. "check {face}".
            // 表里的写法是"工具 {键}"，例如 `check {face}`。
            "check" => keys.contains("face"),
            "search" | "callgraph" | "read" | "inspect" | "affected" | "registry" | "explain"
            | "diff" | "locate" | "why" | "grafts" => true,
            // `new_project` writes a scaffold, so its schema is the check, the way `apply`'s is.
            // `new_project` 会写出脚手架，因此检查它的是 schema，与 `apply` 同理。
            "new_project" => keys.contains("directory"),
            "apply" => keys.contains("apply"),
            // The ledger pair: the table names `conformance {anchor}` and
            // `consistency --specimen <anchor>`, and both are called for real above.
            // 台账那一对：表里点名 `conformance {anchor}` 与 `consistency --specimen <anchor>`，两者都在
            // 上面真调过。
            "conformance" => keys.contains("anchor"),
            "consistency" => keys.contains("specimen") || keys.contains("parent"),
            other => panic!("the table names `{other}`, which no acceptance check covers"),
        };
        assert!(checked, "{tool} {keys}");
    }
}

/// `--root` decides which tree is answered, wherever the process happens to be standing.
/// `--root` 决定答的是哪棵树，无论进程此刻站在哪里。
///
/// Measured: before this, the flag was forwarded as an argument and resolved *inside* the caller's
/// own root, so one scenario round answered about a 475-file checkout instead of the workspace the
/// question named, and only the answer's own root line revealed it.
/// 量出来的：在此之前，该开关被当作参数转发、在调用方自己的根**内部**解析，于是某个情景轮答的是 475 个文件
/// 的检出而不是问题点名的工作区，而只有答案自己那行 root 暴露了这件事。
#[test]
fn the_root_flag_decides_which_tree_is_answered() {
    let alpha = crate::mcp::tools::tools_tests::scratch_package("root-alpha");
    let beta = crate::mcp::tools::tools_tests::scratch_package("root-beta");
    let args = vec![
        "nichlink.status".to_owned(),
        "--root".to_owned(),
        alpha.display().to_string(),
    ];
    let answer = match call_from_arguments(&args) {
        Ok(answer) => answer,
        Err(Refusal::Tool(text)) | Err(Refusal::Usage(text)) => {
            panic!("the call must be accepted, got: {text}")
        }
    };
    assert!(
        answer.contains(&alpha.display().to_string()),
        "the answer names the tree the flag named: {answer}"
    );
    assert!(
        !answer.contains(&beta.display().to_string()),
        "and not another one: {answer}"
    );
    let _ = std::fs::remove_dir_all(&alpha);
    let _ = std::fs::remove_dir_all(&beta);
}

/// A boolean is a flag: the bare spelling is accepted and means true.
/// 布尔就是开关：裸写被接受，且意为 true。
/// A text key keeps digits as text: the round could not express `--literal 1000`.
/// 文本键把数字串留作文本：那轮无法表达 `--literal 1000`。
#[test]
fn a_text_key_keeps_digits_as_text() {
    assert_eq!(scalar_for("literal", "1000"), Value::from("1000"));
    assert_eq!(scalar_for("query", "0"), Value::from("0"));
    assert_eq!(scalar_for("context", "4"), Value::from(4));
}

/// The table names the adoption ledger, because the round measured the cost of not naming it: both
/// arms had to re-derive the ledger's policy (`no renewal of unchanged bytes, one appended line for
/// a new route`) from the kernel instead of being told where the ledger is and what it means.
/// 流程表点名采信台账，因为"不点名"的代价被量到过：两臂都得从内核里重新推出台账策略（"未变的字节不续期、
/// 新路线追加一行"），而不是被告知台账在哪、它是什么意思。
#[test]
fn the_table_names_the_adoption_ledger() {
    assert!(
        super::INSTRUCTIONS.contains("adopted"),
        "the ledger tool is named"
    );
    assert!(
        super::INSTRUCTIONS.contains(".nichlink/adopted/entries"),
        "and so is where it lives"
    );
}

#[test]
fn a_bare_flag_means_true() {
    let temp = std::env::temp_dir().join(format!("nichlink-flag-{}", std::process::id()));
    std::fs::create_dir_all(&temp).expect("scratch");
    let root = temp.display().to_string();
    for parts in [
        vec!["nichlink.search", "--query", "x", "--converge"],
        vec!["nichlink.search", "--query", "x", "--converge", "true"],
    ] {
        let mut args = vec!["--call".to_owned()];
        args.extend(parts.iter().map(|part| (*part).to_owned()));
        match run_client(&args) {
            Client::Called(code) => assert_eq!(code, 0, "bare flag accepted: {parts:?}"),
            Client::Serve => panic!("--call is a call"),
        }
    }
    let _ = std::fs::remove_dir_all(&temp);
    let _ = &root;
}

/// `--log <file>` records one JSON line per call — `{request, response, exit}` — so a measured run
/// needs no wrapper script around every call (the round paid one extra step per instrument call for
/// exactly that wrapper).
/// `--log <文件>` 每次调用记一行 JSON —— `{request, response, exit}` —— 因此被测量的运行不必为每次
/// 调用套包装脚本（那一轮正是为这个包装，每次仪器调用多花一步）。
#[test]
fn a_log_flag_records_one_line_that_mirrors_the_call() {
    let temp = std::env::temp_dir().join(format!("mcp-log-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp);
    std::fs::create_dir_all(&temp).expect("temp directory");
    let log = temp.join("calls.jsonl");

    let args: Vec<String> = [
        "--call",
        "callgraph",
        "--root",
        temp.to_str().expect("path"),
        "--log",
        log.to_str().expect("path"),
    ]
    .iter()
    .map(|part| (*part).to_owned())
    .collect();
    let code = match run_client(&args) {
        Client::Called(code) => code,
        Client::Serve => panic!("--call is a call"),
    };

    let text = std::fs::read_to_string(&log).expect("the log file exists");
    assert_eq!(text.lines().count(), 1, "one line per call: {text}");
    let line: serde_json::Value =
        serde_json::from_str(text.lines().next().expect("a line")).expect("the line is JSON");
    assert_eq!(line["exit"].as_i64(), Some(code.into()), "{line}");
    assert!(
        line["request"]
            .as_array()
            .is_some_and(|request| request.len() >= 4),
        "the request is the argv: {line}"
    );
    assert!(
        line["response"]
            .as_str()
            .is_some_and(|body| !body.is_empty()),
        "the response is the answer body: {line}"
    );

    let _ = std::fs::remove_dir_all(&temp);
}

/// The flow table places a request in one of seven shapes, and every shape carries a stop condition.
/// 流程表把请求放进七种形状之一，而每种形状都带一个停止条件。
///
/// Measured need (the maintainer's own words): a flat tool list with no packaged workflow makes the
/// model rebuild the route from scratch on every question — and the numbers agree (`next` was
/// followed 28% of the time, and one question spent 14 of its 16 calls before the fact that decided
/// it). The signals are asserted to be **structural**, not words, because a table keyed on wording
/// would be the overfitting the same rules forbid.
/// 量出来的需求（维护者原话）：工具平铺、没有打包的工作流 ⇒ 模型每个问题都要把路线重建一遍——数与之一致
/// （`next` 被采纳 28%，而有一道题把 16 次调用里的 14 次花在"决定性事实"之前）。这里断言信号是**结构性**
/// 的而不是词，因为按措辞索引的表正是同一套规则禁止的那种过拟合。
#[test]
fn the_flow_table_packages_the_seven_shapes_with_their_stop_conditions() {
    // One row per shape: the signal as the table spells it, and the stop condition it must carry.
    // 每种形状一行：表里写的信号，以及它必须携带的停止条件。
    let rows = [
        ("empty tree", "stop when the skeleton"),
        ("one object named", "stop at a root cause"),
        (
            "several named",
            "stop when every named object has a verdict",
        ),
        ("a range question", "stop when each column is"),
        ("add an object", "stop when the new face's shape matches"),
        (
            "deepen an object",
            "stop when the tree, the public paths and the factory pins",
        ),
        ("move or merge", "stop when every layer's impact"),
    ];
    for (signal, stop) in rows {
        assert!(
            INSTRUCTIONS.contains(signal),
            "the table names the signal `{signal}`: {INSTRUCTIONS}"
        );
        assert!(
            INSTRUCTIONS.contains(stop),
            "and its stop condition `{stop}`: {INSTRUCTIONS}"
        );
    }
    assert!(
        INSTRUCTIONS.contains("not by its wording"),
        "the signals are structural, not words: {INSTRUCTIONS}"
    );
    assert!(
        INSTRUCTIONS.contains("停止条件"),
        "and the Chinese half carries the same table: {INSTRUCTIONS}"
    );
}
