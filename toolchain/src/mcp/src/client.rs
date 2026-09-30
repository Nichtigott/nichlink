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
//! - `nichlink-mcp --call <tool> --json '{…}'` runs exactly one tool and **makes the exit code the
//!   verdict**: `0` answered, `1` the tool refused (its text goes to stderr), `2` the request itself
//!   was malformed. Plain `--key value` arguments work too, so the common call needs no JSON.
//!   `--call` 只跑一个工具，并**让退出码成为判定**：`0` 作答、`1` 工具拒绝（文本走 stderr）、`2` 请求本身
//!   畸形。也接受普通的 `--key value` 参数，因此常见的调用不必写 JSON。
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
A failing test? `check {face}` runs that face and makes the exit code the verdict. Then take the \
assertion's own words — `search {literal}` finds where that text is produced (comments and string \
literals included). A name? `search {query}`. Who reaches it, and what it reaches? `callgraph \
{function, source: true}` (static heuristic: it sees static calls, not dynamic dispatch). Nobody \
reaches it? `callgraph {orphans: true}` lists what this package defines but never calls. One \
function's own lines? `read {path, line}` or `inspect {path}`. Which tests a change reaches? \
`affected`. Registration faces: `registry`, `explain`, `diff`, then `apply` (preview by default). \
With `root`, every path argument is relative to that root (root \"kernel\" means path \"src/…\").";

/// One line per tool: its name, then the first sentence of its description.
/// 每个工具一行：名字，然后是它描述的第一句。
///
/// The first sentence is the part that gets read — the evaluation's digests quote it — so the list
/// is the ordered, cheap view of the same catalogue `tools/list` advertises in full.
/// 第一句才是被读到的部分——评测里那些摘要引的就是它——因此这张表是同一个目录的"有序、便宜"的视图，
/// 而 `tools/list` 给的仍是完整形态。
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
            Some(format!("{name} — {first}"))
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
    crate::mcp::tools::run_tool(root, name, arguments)
}

/// What a run of the binary should exit with, when it is not serving stdio.
/// 不是在做 stdio 服务时，这个二进制应当以什么状态退出。
pub enum Client {
    /// Serve the stdio bridge as before.
    /// 照旧做 stdio 桥服务。
    Serve,
    /// Call one tool; the number is the exit code.
    /// 调用一个工具；那个数字是退出码。
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
    match arguments[0].as_str() {
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
        "--call" => match call_from_arguments(&arguments[1..]) {
            Ok(text) => Client::Called(emit_or_stop(&text).unwrap_or(0)),
            Err(Refusal::Tool(text)) => {
                eprintln!("{text}");
                Client::Called(1)
            }
            Err(Refusal::Usage(text)) => {
                eprintln!("{text}");
                Client::Called(2)
            }
        },
        other => {
            eprintln!("unknown argument `{other}`\n\n{USAGE}");
            Client::Called(2)
        }
    }
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
nichlink-mcp --call <tool>   run one tool; exit 0 answered, 1 refused, 2 malformed
    [--json '<object>'] [--root <path>] [--<key> <value> …]";

/// Assemble and run one `--call`.
/// 组装并执行一次 `--call`。
fn call_from_arguments(arguments: &[String]) -> Result<String, Refusal> {
    let Some(name) = arguments.first() else {
        return Err(Refusal::Usage(format!(
            "`--call` needs a tool name\n\n{USAGE}"
        )));
    };
    if name.starts_with('-') {
        return Err(Refusal::Usage(format!(
            "`--call` needs a tool name, not `{name}`\n\n{USAGE}"
        )));
    }
    let mut object = Map::new();
    let mut rest = arguments[1..].iter();
    // `--json` sets the base and the explicit pairs win, so a caller can start from a pasted object
    // and still override one key.
    // `--json` 设基底，显式键值对胜出，因此调用方可以从粘贴的对象出发、仍然覆盖某一项。
    let mut overrides: Vec<(String, Value)> = Vec::new();
    let mut base: Option<Map<String, Value>> = None;
    while let Some(flag) = rest.next() {
        let key = match flag.strip_prefix("--") {
            Some(key) if !key.is_empty() => key.to_owned(),
            _ => {
                return Err(Refusal::Usage(format!(
                    "expected `--<key>`, got `{flag}`\n\n{USAGE}"
                )));
            }
        };
        if key == "json" {
            let Some(text) = rest.next() else {
                return Err(Refusal::Usage("`--json` needs an object".to_owned()));
            };
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
        let Some(value) = rest.next() else {
            return Err(Refusal::Usage(format!("`--{key}` needs a value")));
        };
        overrides.push((key, scalar(value)));
    }
    for (key, value) in base.unwrap_or_default() {
        object.insert(key, value);
    }
    for (key, value) in overrides {
        object.insert(key, value);
    }
    let root = crate::mcp::protocol::package_root();
    call_tool(&root, name, &Value::Object(object)).map_err(Refusal::Tool)
}

/// A command-line value with the obvious JSON type.
/// 命令行取值按最显然的 JSON 类型解释。
fn scalar(value: &str) -> Value {
    match value {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        other => match other.parse::<i64>() {
            Ok(number) if number.to_string() == other => Value::from(number),
            _ => Value::from(other),
        },
    }
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
