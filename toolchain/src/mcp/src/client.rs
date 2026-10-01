//! The one-shot client: call one tool without writing an MCP client first.
//! 一次性客户端：不必先写一个 MCP 客户端就能调一个工具。
//!
//! Measured need (2026-09-29 chain-fit evaluation): the bridge had **no directly callable
//! command**, so an agent had to write and debug its own JSON-RPC client before it could ask the
//! first question. In round 1 of that evaluation the arm spent **blocks 0–10 of its 25 reasoning
//! blocks** on that plumbing — building the binary, writing a Python driver, debugging a hang on
//! `notifications/initialized`, a `pkill` that matched its own shell, and a tool list its own
//! script truncated at 6,000 characters — and only **block 11** named the defect. Codegraph, with
//! a CLI, named it at **block 3**. Two commands here are the whole fix:
//! 量出来的需求（2026-09-29 思维链拟合评测）：桥**没有可直接调用的命令**，因此 agent 必须先写、再调试一个
//! 自己的 JSON-RPC 客户端，才问得出第一个问题。那次评测第 1 题里，该组把 25 个推理块中的**块 0–10** 花在
//! 这层管道上——构建二进制、写 Python 驱动、排查 `notifications/initialized` 的挂起、一次把自己 shell 也
//! 匹配上的 `pkill`、以及被自己脚本截断在 6,000 字符的清单——**块 11** 才点名病灶。而有 CLI 的 codegraph
//! 在**块 3** 就点名了。这里的两条命令就是全部修法：
//!
//! - `nichlink-mcp --list` prints the workflow table and one line per tool, so the surface arrives
//!   ordered and short instead of as ~36,000 characters of prose in one truncated payload.
//!   `--list` 打印流程表与每个工具一行，于是描述面以"有组织且短"的形式到达，而不是一帧约 36,000 字符、
//!   还被截断的散文。
//! - `nichlink-mcp --call <tool> --json '{…}'` runs exactly one tool; **its exit code is the
//!   call's, never the tool's**: `0` the tool answered (its text goes to stdout), `1` it refused (its
//!   text goes to stderr), `2` the request itself was malformed (a usage error). The tool's own
//!   **verdict is in the reply body** — `check`, the one tool that judges a run, puts it on the
//!   reply's **first line**. Round 7 measured the cost of the old promise that a call's exit code was
//!   the tool's verdict: `check` on the failing fixture `target/round7/s5` answered with `exit   101`
//!   on line six while this client exited `0`, so anything that read the exit code as the verdict read
//!   a red face as green. Plain `--key value` arguments work too, so the common call needs no JSON.
//!   `--call` 只跑一个工具；**它的退出码是这次调用的，绝不是工具的**：`0` 工具作答（文本走 stdout）、
//!   `1` 工具拒绝（文本走 stderr）、`2` 请求本身畸形（用法错）。工具自己的**判定在回复正文里**——
//!   `check` 这个唯一判定一次运行的工具把它放在回复的**第一行**。第七轮量到了旧承诺的代价——那句说
//!   "一次调用的退出码就是工具的判定"的话：`check` 在失败夹具 `target/round7/s5` 上把 `exit   101`
//!   写在第六行，而这个客户端以 `0` 退出，于是任何"把退出码当判定"的读法都把红面读成了绿面。也接受
//!   普通的 `--key value` 参数，因此常见的调用不必写 JSON。
//!
//! Neither shape replaces the stdio bridge: with no arguments this binary still serves it.
//! 两种形状都不取代 stdio 桥：不带参数时这个二进制仍然做服务。

use std::io::Write;
use std::path::Path;

use serde_json::{Map, Value};

/// What the agent is told once, at `initialize`, and again by `--list`.
/// agent 在 `initialize` 时被告知一次、`--list` 时再看到的东西。
///
/// It is a **workflow table**, because that is what the evaluation's failure was about: the old
/// text was one 202-character sentence against ~28,000 characters of per-tool prose, and an agent
/// that reads the first clause of each description had nothing that said which tool answers which
/// symptom. The table names the symptom, the call, and the one thing the call cannot answer.
/// 它是一张**流程表**，因为评测失败的地方正在这里：旧文本是一条 202 字符的句子，对面是约 28,000 字符的
/// 逐工具散文，而只读每条描述第一小句的 agent 手上没有任何东西说"哪种症状用哪个工具"。表里点名症状、
/// 该发的调用、以及这次调用答不了的那一件事。
pub const INSTRUCTIONS: &str = "\
Read the symptom first, then take the shortest route it names. A failing test? Run the suite you \
already have (`cargo test`) and keep its output: the failing assertion's own words are the next \
clue. `check {face}` is the same run aimed at **one face** — reach for it when the default face is \
green and you suspect another one (`face: \"all\"`, or a feature name), because it names the face \
it ran. **Its verdict is the first line of the reply, not this client's exit code**: the exit code \
says only how the call went (`0` answered, `1` refused, `2` a usage error), and `check` answers \
even about a failing run — so read the `verdict  passed (cargo exit 0)` line or the \
`verdict  failed (cargo exit 101)` line before anything else. **A range-type question** \
（「这里还有别的问题吗」）is the other workflow and it starts from the same reply: `check` ends with a \
whole-tree census of static facts (pass `census: true` for the whole table), and every column says \
what it does not cover — dispose of each column before opening files one by one. \
**范围型问题**（「这里还有别的问题吗」）是另一条工作流，也从同一次回复开始：`check` 末尾带一张整树\
静态事实普查（`census: true` 给整表），每栏都写明它不覆盖什么——先把每栏处置掉，再逐个打开文件。 \
**If the symptom is in something the \
code produces** (a rendered report, a generated record), `search {literal}` on that product's own \
words finds the line that produces it, usually in one call. **If the symptom is the assertion's \
message**, search a short, stable phrase from it: the test framework appends `left:`/`right:` and \
the numbers, so the whole line matches nothing. **A symbol name you can already read** (from a test \
or a trace) goes straight to `callgraph {function}` — one call gives its callers, its callees, its \
own lines and its doc, and `search {query}` first would only cost a second call. A name you do not \
have yet: `callgraph` hop by hop from the caller you do have. `callgraph {orphans: true}` lists what \
this package defines and nothing here calls. One function's own lines: `read {path, line}` — the \
window is ±8 lines by default, so pass `context` for a wider one and `whole` for the whole file. \
Which \
tests a change reaches: `affected`. Registration faces: `registry`, `explain`, `diff`, then `apply` \
(a preview unless `apply: true`). \\
             If the tree carries an adoption ledger (`.nichlink/adopted/entries`), `adopted` reads it \
             and says which entries still hold; a byte that moved makes an entry need a **person**, \
             not an edit, and a confirmation is one more line appended. 若这棵树带采信台账 \
             （`.nichlink/adopted/entries`），`adopted` 读它并说出哪些条目仍然成立；字节一动，条目就需要 \
             **人来确认**而不是改代码，而确认就是**再追加一行**。 A route the ledger never named is a **new anchor** — a first confirmation, not a \
             renewal; the read names that action. 台账从未点名的路线是**新锚**——首次确认、不是续期，\
             读取时会把该动作点出来。 **Did the design get carried through?** `conformance {anchor}` \
             says whether the ledger's claim still holds and where it lapsed; \
             `consistency --specimen <anchor>` compares the siblings against the declared shape those \
             files carry, and names who lacks which declaration. **设计落实下去了吗？** \
             `conformance {anchor}` 判台账的声明是否仍成立、在哪个文件失效；\
             `consistency --specimen <anchor>` 把兄弟与那批文件携带的已声明形状比对，点名谁缺哪条声明。 \
             **An object that should be deeper inside is not a new face**: \
             `apply {action: \"deepen\", node, inside, apply: true}` writes a parts layer into that \
             face's own file and leaves the tree, the public path and the factory pins alone, and \
             its reply prices the other reading (another face under it). 把一个对象做深**不是**加面：\
             `apply {action: \"deepen\", node, inside, apply: true}` 把零件层写进那个面自己的文件，\
             不动树、公开路径与出厂形状钉子，并在回复里给出另一种读法的代价。 \
             **Independence is evidence you construct**: two green runs do \
             not show that two defects are independent — fix or revoke one and show the other's \
             symptom is still there. **独立性是构造出来的证据**：两次全绿不能说明两处缺陷互相独立\
             ——只修其一（或撤销其一），再证明另一处的症状仍在。 With `root`, every path argument is relative to that root (root \
\"kernel\" means path \"src/…\").";

/// One line per tool: its name, then the first sentence of its description.
/// 每个工具一行：名字，然后是它描述的第一句。
///
/// The first sentence is the part that gets read — the evaluation's digests quote it — so the list
/// is the ordered, cheap view of the same catalogue `tools/list` advertises in full.
/// 第一句才是被读到的部分——评测里那些摘要引的就是它——因此这张表是同一个目录的"有序、便宜"的视图，
/// 而 `tools/list` 给的仍是完整形态。
/// The property names a tool accepts, following `anyOf`/`oneOf` one level down, with the names it
/// insists on.
/// 一个工具接受的属性名（向下跟一层 `anyOf`/`oneOf`），以及它坚持要的那些名字。
fn visit_keys(schema: Option<&Value>, names: &mut Vec<String>, required: &mut Vec<String>) {
    let Some(schema) = schema else { return };
    if let Some(items) = schema.get("required").and_then(Value::as_array) {
        for item in items.iter().filter_map(Value::as_str) {
            if !required.iter().any(|seen| seen == item) {
                required.push(item.to_owned());
            }
        }
    }
    if let Some(map) = schema.get("properties").and_then(Value::as_object) {
        for key in map.keys() {
            if !names.iter().any(|seen| seen == key) {
                names.push(key.clone());
            }
        }
    }
    // Branched names are collected **without** the required mark: a `anyOf` branch stating that it
    // needs one of two keys does not mean both are required, and starring both would be a louder
    // lie than the missing star. The refusal still names the pair.
    // 分支里的名字**不**带必填标记：`anyOf` 的一支说"两个键要其一"，不等于两个都必填，把两个都打星
    // 是比"没星"更响的谎。拒绝文案仍会把那一对点出来。
    for branch in ["anyOf", "oneOf"] {
        if let Some(items) = schema.get(branch).and_then(Value::as_array) {
            for item in items {
                let mut ignored = Vec::new();
                visit_keys(Some(item), names, &mut ignored);
            }
        }
    }
}

/// One tool's whole description, for the caller that needs the shape the one-line list truncates.
/// 一个工具的完整描述，供需要"一行式清单所截掉的那部分形状"的调用方使用。
///
/// The round measured what the truncation costs: the `apply` description already spells out that the
/// values live under `fields` and that every value must be a string, and an arm still spent four
/// refusals discovering it, because the one-shot client only ever showed the first sentence.
/// 那一轮量出了截断的代价：`apply` 的描述本来就写明"取值在 `fields` 下、每个值必须是字符串"，而一个臂
/// 仍花了四次被拒才发现它——因为一次性客户端从来只显示第一句。
pub fn describe_tool(name: &str) -> Option<String> {
    let wanted = if name.contains('.') {
        name.to_owned()
    } else {
        format!("nichlink.{name}")
    };
    crate::mcp::tools::tools().into_iter().find_map(|tool| {
        let tool_name = tool.get("name")?.as_str()?;
        if tool_name != wanted {
            return None;
        }
        let description = tool
            .get("description")
            .and_then(Value::as_str)
            .unwrap_or("");
        let schema = tool.get("inputSchema");
        let mut properties: Vec<String> = Vec::new();
        let mut required: Vec<String> = Vec::new();
        visit_keys(schema, &mut properties, &mut required);
        let keys = properties
            .iter()
            .map(|key| {
                if required.contains(key) {
                    format!("{key}*")
                } else {
                    key.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
        Some(format!(
            "{tool_name}\n    keys: {keys} (* = required)\n{description}"
        ))
    })
}

/// One line per tool: its name, the first sentence of its description, and the keys it takes
/// (`*` = required), so a caller does not have to guess a key name and spend a refusal learning it.
/// 每个工具一行：名字、描述第一小句，以及它接受的键（`*` = 必填），这样调用方不必靠猜键名、再花一次
/// 被拒来学会它。
pub fn list_tool_lines() -> Vec<String> {
    crate::mcp::tools::tools()
        .into_iter()
        .filter_map(|tool| {
            let name = tool.get("name")?.as_str()?.to_owned();
            let description = tool
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("");
            let first = first_sentence(description);
            // The shape of the call, on the same line as its name: the round measured an arm
            // guessing a key (`--face` where the tool wanted `node`) and burning a refusal on it,
            // because this list named the tool without naming what it takes.
            // 调用的形状与名字同一行：那一轮量到一个臂猜键名（工具要 `node`、它写了 `--face`）并为此
            // 白吃一次拒绝——因为这张表只点了工具名，没说它收什么。
            let schema = tool.get("inputSchema");
            let mut properties: Vec<String> = Vec::new();
            let mut required: Vec<String> = Vec::new();
            visit_keys(schema, &mut properties, &mut required);
            let keys = properties
                .iter()
                .map(|key| {
                    if required.contains(key) {
                        format!("{key}*")
                    } else {
                        key.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            Some(format!("{name} — {first}\n    keys: {keys} (* = required)"))
        })
        .collect()
}

/// The first sentence of a description, which is the part a reader keeps.
/// 描述的第一句，也就是读者会留下的那部分。
fn first_sentence(description: &str) -> String {
    let flat = description.split_whitespace().collect::<Vec<_>>().join(" ");
    match flat.find(". ") {
        Some(at) => flat[..=at].trim_end_matches(". ").to_owned(),
        None => flat,
    }
}

/// Run one tool and return its text, or the text of its refusal.
/// 跑一个工具并回它的文本，或它拒绝时的文本。
pub fn call_tool(root: &Path, name: &str, arguments: &Value) -> Result<String, String> {
    // The base is chosen by the caller (`--root`) or by the process's own directory; the tools then
    // resolve their `path`/`file` arguments against it.
    // 基底由调用方（`--root`）或进程自身所在目录决定；各工具随后相对它解析自己的 `path`/`file` 参数。
    crate::mcp::tools::run_tool(root, name, arguments)
}

/// What a run of the binary should exit with, when it is not serving stdio.
/// 不是在做 stdio 服务时，这个二进制应当以什么状态退出。
pub enum Client {
    /// Serve the stdio bridge as before.
    /// 照旧做 stdio 桥服务。
    Serve,
    /// Call one tool; the number is the exit code of the **call** — answered (`0`), refused (`1`), or
    /// a usage error (`2`) — never the tool's own verdict, which is in the reply's text.
    /// 调用一个工具；那个数字是这次**调用**的退出码——作答（`0`）、拒绝（`1`）、用法错（`2`）——绝不是
    /// 工具自己的判定，后者在回复的文本里。
    Called(i32),
}

/// Print one block, treating a reader that closed the pipe as a finished reader.
/// 打印一段文本，并把"读者关掉了管道"当作读者读完了。
///
/// Measured the hard way: `--list | head -6` made `println!` panic with `Broken pipe`, which is the
/// one thing an agent piping our output through `head` or `grep` must never see. A closed pipe is
/// not a failure of ours; it is the reader saying it has enough.
/// 硬撞出来的：`--list | head -6` 让 `println!` 以 `Broken pipe` 崩溃，而这正是把我们的输出管给 `head`
/// 或 `grep` 的 agent 绝不能看到的东西。管道被关掉不是我们的失败，而是读者说它够了。
fn emit(text: &str) -> Result<(), std::io::Error> {
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    writeln!(lock, "{text}")
}

/// Emit, or say what stopped us: `Some(code)` when the caller should exit with it.
/// 输出，或者说清是什么让我们停下：需要以它退出时回 `Some(code)`。
fn emit_or_stop(text: &str) -> Option<i32> {
    match emit(text) {
        Ok(()) => None,
        Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => Some(0),
        Err(error) => {
            eprintln!("nichlink-toolchain: cannot write stdout: {error}");
            Some(2)
        }
    }
}

/// Decide what this invocation is, and do it.
/// 判断这次调用是什么，并执行它。
///
/// Arguments are the ones after the program name. `--call` takes the tool name, then any of
/// `--json <object>`, `--root <path>`, or `--<key> <value>` pairs; a bare `true`/`false` becomes a
/// boolean and digits become a number, so the common call reads like a command rather than like a
/// hand-written protocol frame.
/// 参数是程序名之后的那些。`--call` 接受工具名，然后是 `--json <对象>`、`--root <路径>`，或任意
/// `--<键> <值>` 对；裸的 `true`/`false` 变成布尔、数字串变成数字，因此常见的调用读起来像一条命令，
/// 而不像手写的协议帧。
pub fn run_client(arguments: &[String]) -> Client {
    if arguments.is_empty() {
        return Client::Serve;
    }
    // W1.7: `--log <file>` appends one JSON line per call — `{"request":[…],"response":"…","exit":N}`
    // — so a measured run needs no wrapper script around every call. The round paid one extra step
    // per instrument call for exactly that wrapper.
    // W1.7：`--log <文件>` 每次调用追一行 JSON —— `{"request":[…],"response":"…","exit":N}` —— 因此
    // 一次被测量的运行不必为每次调用套一个包装脚本。那一轮正是为这个包装每次仪器调用多花一步。
    let (log, arguments) = split_log(arguments);
    let arguments = arguments.as_slice();
    let request: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match arguments[0].as_str() {
        // `--list <tool>` prints one tool's **whole** description: the one-line list is for choosing
        // a tool, and this form is for calling it without guessing the shape the list truncates.
        // `--list <tool>` 打印**一个**工具的完整描述：一行式清单用来挑工具，这个形态用来"不必猜清单
        // 截掉的形状"就能调用。
        "--list" | "-l" if arguments.get(1).is_some_and(|name| !name.starts_with('-')) => {
            let name = arguments.get(1).map(String::as_str).unwrap_or_default();
            match describe_tool(name) {
                Some(text) => Client::Called(emit_or_stop(&text).unwrap_or(0)),
                None => {
                    eprintln!("unknown tool `{name}`; --list names them all");
                    Client::Called(1)
                }
            }
        }
        "--list" | "-l" => {
            for block in std::iter::once(INSTRUCTIONS.to_owned())
                .chain(std::iter::once(String::new()))
                .chain(list_tool_lines())
            {
                if let Some(code) = emit_or_stop(&block) {
                    return Client::Called(code);
                }
            }
            Client::Called(0)
        }
        "--help" | "-h" => Client::Called(emit_or_stop(USAGE).unwrap_or(0)),
        // A bare tool name is a call: `nichlink-mcp callgraph --function x` reads the way a command
        // line reads, and requiring `--call` first cost the round a refused call.
        // 裸工具名就是一次调用：`nichlink-mcp callgraph --function x` 是命令行的读法，而此前必须先写
        // `--call` 让那一轮白吃了一次拒绝。
        other if other.starts_with("nichlink.") || !other.starts_with('-') => {
            answered(&log, &request, call_from_arguments(arguments))
        }
        "--call" => answered(&log, &request, call_from_arguments(&arguments[1..])),
        other => {
            eprintln!("unknown argument `{other}`\n\n{USAGE}");
            Client::Called(2)
        }
    }
}

/// Emit one call's answer, and write the log line when the caller asked for one.
/// 输出一次调用的答案；调用方要了日志就写一行。
fn answered(
    log: &Option<std::path::PathBuf>,
    request: &[&str],
    outcome: Result<String, Refusal>,
) -> Client {
    let (text, code) = match outcome {
        Ok(text) => (text, 0),
        Err(Refusal::Tool(text)) => (text, 1),
        Err(Refusal::Usage(text)) => (text, 2),
    };
    if let Some(path) = log {
        let line = serde_json::json!({
            "request": request,
            "response": text,
            "exit": code,
        });
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            use std::io::Write;
            let _ = writeln!(file, "{line}");
        }
    }
    match code {
        0 => Client::Called(emit_or_stop(&text).unwrap_or(0)),
        _ => {
            eprintln!("{text}");
            Client::Called(code)
        }
    }
}

/// Pull `--log <file>` out of the arguments; the rest is parsed as before.
/// 把 `--log <文件>` 从实参里摘出来；其余照旧解析。
fn split_log(arguments: &[String]) -> (Option<std::path::PathBuf>, Vec<String>) {
    let mut log = None;
    let mut rest = Vec::with_capacity(arguments.len());
    let mut at = 0;
    while at < arguments.len() {
        if arguments[at] == "--log" && at + 1 < arguments.len() {
            log = Some(std::path::PathBuf::from(&arguments[at + 1]));
            at += 2;
            continue;
        }
        rest.push(arguments[at].clone());
        at += 1;
    }
    (log, rest)
}

/// The first argument that names a catalogue tool, and where it sat.
/// 第一个点名目录里某个工具的实参，以及它出现的位置。
fn find_catalogue_name(arguments: &[String]) -> Option<(usize, &String)> {
    arguments.iter().enumerate().find(|(_, token)| {
        let bare = token.strip_prefix("nichlink.").unwrap_or(token);
        let wanted = format!("nichlink.{bare}");
        crate::mcp::tools::tools()
            .into_iter()
            .any(|tool| tool.get("name").and_then(|name| name.as_str()) == Some(wanted.as_str()))
    })
}

/// How a `--call` can fail.
/// `--call` 可能怎么失败。
enum Refusal {
    /// The tool answered with a refusal; its text is the answer.
    /// 工具以拒绝作答；它的文本就是答案。
    Tool(String),
    /// The request itself was malformed.
    /// 请求本身畸形。
    Usage(String),
}

/// The usage text.
/// 用法文本。
const USAGE: &str = "\
nichlink-mcp                 serve the stdio bridge (JSON-RPC 2.0)
nichlink-mcp --list          the workflow table and one line per tool
nichlink-mcp --call <tool>   run one tool; exit 0 answered, 1 refused, 2 usage error
                             (the tool's own verdict is in its output, e.g. `check`'s first line)
    [--json '<object>'] [--root <path>] [--<key> <value> …]";

/// Assemble and run one `--call`.
/// 组装并执行一次 `--call`。
fn call_from_arguments(arguments: &[String]) -> Result<String, Refusal> {
    // The tool name may sit **anywhere** among the flags. A round measured the cost of requiring it
    // first: `--call --json … --root … s3` was refused with "needs a tool name, not `--json`" and
    // that call bought nothing.
    // 工具名可以出现在开关之间的**任意位置**。有一轮量到"必须放最前"的代价：`--call --json … --root
    // … s3` 被拒成 "needs a tool name, not `--json`"，那次调用一无所获。
    let (raw, remaining): (&String, Vec<&String>) = match arguments.first() {
        Some(first) if !first.starts_with('-') => (first, arguments[1..].iter().collect()),
        _ => match find_catalogue_name(arguments) {
            Some((index, name)) => (
                name,
                arguments
                    .iter()
                    .enumerate()
                    .filter(|(at, _)| *at != index)
                    .map(|(_, token)| token)
                    .collect(),
            ),
            None => {
                let spelled = arguments.first().map(String::as_str).unwrap_or("");
                return Err(Refusal::Usage(format!(
                    "`--call` needs a tool name (one of the names `--list` prints), not `{spelled}`\n\n{USAGE}"
                )));
            }
        },
    };
    // The `nichlink.` prefix is the catalogue's spelling; on a command line whose only tool source is
    // this bridge it is redundant, so a bare `callgraph` resolves to `nichlink.callgraph`.
    // `nichlink.` 前缀是目录里的拼法；在命令行上，唯一的工具来源就是这个桥，因此它是冗余的 ——
    // 裸写 `callgraph` 就解析成 `nichlink.callgraph`。
    let resolved = if raw.contains('.') {
        raw.to_owned()
    } else {
        format!("nichlink.{raw}")
    };
    let name = resolved.as_str();
    let mut object = Map::new();
    let mut at = 0usize;
    // `--json` sets the base and the explicit pairs win, so a caller can start from a pasted object
    // and still override one key.
    // `--json` 设基底，显式键值对胜出，因此调用方可以从粘贴的对象出发、仍然覆盖某一项。
    let mut overrides: Vec<(String, Value)> = Vec::new();
    let mut base: Option<Map<String, Value>> = None;
    while at < remaining.len() {
        let flag = remaining[at];
        let key = match flag.strip_prefix("--") {
            Some(key) if !key.is_empty() => key.to_owned(),
            _ => {
                return Err(Refusal::Usage(format!(
                    "expected `--<key>`, got `{flag}`\n\n{USAGE}"
                )));
            }
        };
        at += 1;
        if key == "json" {
            let Some(text) = remaining.get(at) else {
                return Err(Refusal::Usage("`--json` needs an object".to_owned()));
            };
            at += 1;
            let parsed: Value = serde_json::from_str(text)
                .map_err(|error| Refusal::Usage(format!("`--json` is not valid JSON: {error}")))?;
            match parsed {
                Value::Object(map) => base = Some(map),
                other => {
                    return Err(Refusal::Usage(format!(
                        "`--json` must be an object, got {}",
                        type_of(&other)
                    )));
                }
            }
            continue;
        }
        // A boolean is a flag: `--orphans` alone means true, and only a token that is not another
        // `--flag` is taken as a value. The round measured the cost of requiring a value here —
        // `--orphans` answered "needs a value" while the flow table recommended exactly that shape.
        // 布尔就是开关：`--orphans` 单独出现意为 true，只有不是另一个 `--flag` 的记号才被当作取值。
        // 那轮量出了"这里必须给值"的代价——流程表推荐的形状恰恰是 `--orphans`，而它回的是 "needs a
        // value"。
        let value = match remaining.get(at) {
            Some(next) if !next.starts_with("--") => {
                at += 1;
                scalar_for(&key, next)
            }
            _ => Value::Bool(true),
        };
        overrides.push((key, value));
    }
    for (key, value) in base.unwrap_or_default() {
        object.insert(key, value);
    }
    for (key, value) in overrides {
        // A repeated flag is how a list is spelled on a command line: `--files a --files b` becomes
        // the array the tool asks for. Without it the only spelling was a pasted `--json` object,
        // which the round measured costing three refused calls.
        // 重复的开关就是命令行上写列表的方式：`--files a --files b` 变成工具要的那个数组。没有它，
        // 唯一的写法是粘贴一个 `--json` 对象——那轮量到它花了三次被拒的调用。
        match object.get_mut(&key) {
            Some(Value::Array(items)) => items.push(value),
            Some(existing) => {
                let first = std::mem::take(existing);
                object.insert(key, Value::Array(vec![first, value]));
            }
            None => {
                object.insert(key, value);
            }
        }
    }
    // `--root` names the tree the question is about, so it decides the base every other argument is
    // resolved against. Forwarding it as an argument instead made it a path *inside* the caller's own
    // root, and a scenario round measured what that costs: a call made from the wrong directory got a
    // 475-file answer about the checkout, and only the root line on the answer caught it.
    // `--root` 命名的是"问题问的是哪棵树"，因此它决定其它每个参数相对什么解析。把它当作参数转发，会让它
    // 变成调用方自己根**内部**的一个路径，而一个情景轮量出了代价：在错的目录里发出的一次调用拿回的是关于本
    // 检出的 475 个文件，只有答案上的那行 root 抓住了它。
    let base = match object
        .remove("root")
        .and_then(|value| value.as_str().map(str::to_owned))
    {
        Some(path) => {
            let candidate = Path::new(&path);
            let absolute = if candidate.is_absolute() {
                candidate.to_path_buf()
            } else {
                std::env::current_dir()
                    .unwrap_or_else(|_| std::path::PathBuf::from("."))
                    .join(candidate)
            };
            match std::fs::canonicalize(&absolute) {
                Ok(found) => found,
                Err(error) => {
                    return Err(Refusal::Usage(format!("--root {path}: {error}")));
                }
            }
        }
        None => crate::mcp::protocol::package_root(),
    };
    call_tool(&base, name, &Value::Object(object)).map_err(Refusal::Tool)
}

/// A command-line value with the type its **key** implies.
/// 命令行取值按**键**所暗示的类型解释。
///
/// Text is the default, because a command line is text: turning every digit string into a number is
/// what made `--literal 1000` unexpressible in the round (the search wanted text and got a number).
/// Only the keys that are numbers by name get a number, and `true`/`false` are booleans anywhere.
/// 默认是文本，因为命令行本来就是文本：把每个数字串都变成数字，正是那轮 `--literal 1000` 无法表达的
/// 原因（检索要文本、拿到的是数字）。只有**名字上就是数字**的键才转数字，而 `true`/`false` 在任何键上
/// 都是布尔。
fn scalar_for(key: &str, value: &str) -> Value {
    match value {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        other if numeric_key(key) => match other.parse::<i64>() {
            Ok(number) if number.to_string() == other => Value::from(number),
            _ => Value::from(other),
        },
        other => Value::from(other),
    }
}

/// Whether a key names a number, so its value may become one.
/// 某个键是否点名一个数字，从而它的取值可以变成数字。
fn numeric_key(key: &str) -> bool {
    matches!(key, "limit" | "line" | "context" | "count" | "depth")
        || key.ends_with("_ms")
        || key.ends_with("_bytes")
}

/// The name of a JSON value's type, for a refusal that says what it got.
/// JSON 取值的类型名，供"它收到了什么"的拒绝使用。
fn type_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
