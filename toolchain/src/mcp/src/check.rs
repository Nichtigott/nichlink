//! An observed test run: which face was tested, what the run said, and nothing it did not say.
//! 一次被观测到的测试运行：测的是哪个面、运行说了什么，以及它没说的一律不算。
//!
//! This is the one tool that runs a process, and it exists because the alternative was measured and
//! found wanting: a defect compiled only under a non-default feature cannot fail on the default
//! face, so a green default run is not evidence about that feature, and neither this bridge nor the
//! control tool could say which face a red run would appear on. `nichlink.status` answers the
//! static half (which faces exist, which one is the default); this answers the other half by
//! **observing** a run instead of guessing one. The design is `docs/design-mcp-test-faces.md`.
//! 这是唯一会跑进程的工具，它存在的理由是：另一条路被量过、而且不够用——只在非默认特性下编译的缺陷在
//! 默认面上不可能失败，因此默认面全绿不是关于那个特性的证据，而桥与对照工具都答不了"红会出现在哪个面"。
//! `nichlink.status` 答静态那一半（有哪些面、哪个是默认面），这里用**观测**一次运行来答另一半，而不是
//! 猜它。设计见 `docs/design-mcp-test-faces.md`。
//!
//! Three rules keep observation honest, and each one is a pin:
//! 三条规矩让"观测"保持诚实，每一条都有钉子：
//!
//! - **The request says the face.** There is no default face here: a caller who does not say which
//!   face to test is refused by name, because that choice is the whole point of the tool.
//!   **请求要说出面。** 这里没有默认面：不说测哪个面的调用方会被按名拒绝，因为那个选择正是这个工具的意义。
//! - **A timeout is `unknown`, never a pass**, and the answer says what was killed (the direct child
//!   only — its own children may survive) and where the log so far is.
//!   **超时是 `unknown`，绝不是通过**，而且答案说清杀的是什么（只杀直接子进程——它自己的子进程可能还活着）
//!   以及到目前为止的日志在哪。
//! - **Nothing ran is not a pass.** A log with no `test result:` line reports that, rather than an
//!   empty success.
//!   **什么都没跑不是通过。** 没有 `test result:` 行的日志会这么说，而不是给一个空的成功。

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::mcp::build_evidence::out_dir;
use crate::mcp::truncation::withheld;

/// How long a run may take before it is declared unknown.
/// 一次运行在被判为 unknown 之前最长可以跑多久。
const DEFAULT_TIMEOUT_MS: u64 = 900_000;

/// The shortest and longest timeout a request may ask for.
/// 请求可以要求的最短与最长超时。
const MIN_TIMEOUT_MS: u64 = 1_000;
const MAX_TIMEOUT_MS: u64 = 3_600_000;

/// How many detail lines of each kind the summary keeps.
/// 摘要对每一类明细保留多少行。
const SAMPLE: usize = 12;

/// What a log said: the lines the reply carries, and how many `test result:` lines it had.
/// 日志说了什么：回复携带的行，以及它有几条 `test result:` 行。
///
/// The count is separate from the lines because it decides the verdict: a log with no result line is
/// not a pass, and the verdict line has to be able to say that without re-reading the log.
/// 这个计数与那些行分开，因为它决定判定：没有结果行的日志不是通过，而判定行必须能说出这件事，不能再读
/// 一遍日志。
pub(crate) struct Observation {
    /// The `result` and `failed` lines, in the order the log carried them.
    /// 按日志里的顺序排列的 `result` 与 `failed` 行。
    pub(crate) lines: Vec<String>,
    /// How many `test result:` lines the log had; zero means nothing ran.
    /// 日志里有几条 `test result:` 行；零意味着什么都没跑。
    pub(crate) results: usize,
}

/// What one finished (or killed) command produced.
/// 一条结束（或被杀死）的命令产出了什么。
pub(crate) struct RunOutcome {
    /// The exit code, when the process reported one.
    /// 进程报告了退出码时的退出码。
    pub(crate) code: Option<i32>,
    /// Whether the timeout fired and the child was killed.
    /// 超时是否触发、子进程是否被杀。
    pub(crate) timed_out: bool,
    /// How long it ran.
    /// 它跑了多久。
    pub(crate) elapsed: Duration,
    /// Where the full output landed.
    /// 完整输出的落点。
    pub(crate) log: PathBuf,
}

/// Run one command, its output going straight to a log file.
/// 跑一条命令，输出直接写进日志文件。
///
/// The output goes to the file rather than to a pipe on purpose: a pipe nobody drains fills up and
/// deadlocks the child, which is a failure mode this repository has already paid for once.
/// 输出写文件而不是走管道是有意的：没人排空的管道会填满并让子进程死锁,而这个失败模式本仓已经付过一次代价。
pub(crate) fn run_command(
    command: &mut Command,
    timeout: Duration,
    log: &Path,
) -> Result<RunOutcome, String> {
    if let Some(parent) = log.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
    }
    let handle =
        File::create(log).map_err(|error| format!("cannot write {}: {error}", log.display()))?;
    let errors = handle
        .try_clone()
        .map_err(|error| format!("cannot duplicate the log handle: {error}"))?;
    let started = Instant::now();
    let mut child = command
        .stdout(Stdio::from(handle))
        .stderr(Stdio::from(errors))
        .stdin(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot run the check: {error}"))?;
    let mut timed_out = false;
    let code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {
                if started.elapsed() >= timeout {
                    timed_out = true;
                    // Only the direct child is killed: this is a documented bound, not an
                    // oversight, and the answer repeats it.
                    // 只杀直接子进程：这是写明的边界而不是疏漏，答案里也再说一遍。
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(error) => return Err(format!("cannot wait for the check: {error}")),
        }
    };
    Ok(RunOutcome {
        code,
        timed_out,
        elapsed: started.elapsed(),
        log: log.to_path_buf(),
    })
}

/// The run's verdict, as the first line of the reply.
/// 这次运行的判定，作为回复的第一行。
///
/// Round 7 measured the defect this line exists for, on the frozen fixture `target/round7/s5`:
/// `cargo test` failed with exit 101, the reply said so on its **sixth** line (`exit   101`), and
/// the one-shot client exited `0` — so a reader that stopped at the top of the reply, or took the
/// client's exit code for the verdict, read a failing face as green. The verdict is now line 1, and
/// the first two lines are enough to tell a red face from a green one.
/// 第七轮在冻结夹具 `target/round7/s5` 上量到这一行存在的理由：`cargo test` 以 101 失败，回复把它写在
/// **第六**行（`exit   101`），而一次性客户端以 `0` 退出——于是只在回复开头停下的读者、或把客户端退出码
/// 当判定的读者，会把失败的面读成绿。现在判定是第 1 行，头两行就足以区分红面与绿面。
///
/// It is the **run's** verdict, never the call's: a timeout, a signal and a log with no
/// `test result:` line are all `unknown`, because none of them is evidence of a pass, and the
/// client's own `0` (answered) says nothing about any of them.
/// 它是**这次运行**的判定，绝不是这次调用的：超时、信号、以及没有 `test result:` 行的日志都是
/// `unknown`，因为它们都不是通过的证据，而客户端自己的 `0`（作答）对其中任何一个都没说什么。
fn verdict(timed_out: bool, code: Option<i32>, results: usize) -> String {
    match (timed_out, code) {
        (true, _) => "verdict  unknown (the run timed out; a killed run is not a pass)".to_owned(),
        (false, Some(0)) if results > 0 => "verdict  passed (cargo exit 0)".to_owned(),
        (false, Some(0)) => {
            "verdict  unknown (cargo exit 0, but the log has no `test result:` line: \
                             nothing ran, and that is not a pass)"
                .to_owned()
        }
        (false, Some(code)) => format!("verdict  failed (cargo exit {code})"),
        (false, None) => {
            "verdict  unknown (the process was signalled; a signalled run is not a pass)".to_owned()
        }
    }
}

/// The head of the reply: the verdict first, then what produced it.
/// 回复的开头：判定在最前，然后是产出它的一切。
///
/// Only [`check`]'s own callers see the census, and it is appended *after* this: the verdict is the
/// thing a reader stops at, and no later section may push it down.
/// 只有 [`check`] 自己的调用方会看到总账，而它接在这之后：判定是读者会停下的那件事，后面任何一段都不
/// 能把它挤下去。
/// The tree's own size, in one line: the fact `status` was opened for.
/// 这棵树自己的规模，一行：也就是当初为它打开 `status` 的那个事实。
fn tree_line(root: &Path) -> String {
    match crate::mcp::source_index::load_sources(root) {
        Ok(files) => format!(
            "{} rust file(s), {} function(s)",
            files.len(),
            files.iter().map(|file| file.functions.len()).sum::<usize>()
        ),
        Err(_) => "not readable as sources".to_owned(),
    }
}

fn head(
    root: &Path,
    face: &str,
    outcome: &RunOutcome,
    timeout: Duration,
    observed: &Observation,
) -> Vec<String> {
    let mut lines = vec![
        verdict(outcome.timed_out, outcome.code, observed.results),
        format!("check  cargo test{}", flags(face)),
        format!("face   {face}"),
        format!("root   {}", root.display()),
        // The counts `status` exists for ride on the head of the run they are about. Measured: that
        // tool was called as an opening and a closing ritual **8 times in 46 calls** (W8), and every
        // one of those answered "N files, M functions" — which is one line here, on the answer the
        // caller was going to run anyway.
        // `status` 存在的理由——那两个计数——搭在这次运行的头部。实测：那个工具在 46 次调用里被当作开场与
        // 收场的仪式用了 **8 次**（W8），而每一次都只回答了"N 个文件、M 个函数"——它在这里是一行，而且在
        // 调用方本来就要跑的那次答案上。
        format!("tree   {}", tree_line(root)),
        format!("elapsed {} ms", outcome.elapsed.as_millis()),
        format!("log    {}", outcome.log.display()),
    ];
    match (outcome.timed_out, outcome.code) {
        (true, _) => lines.push(format!(
            "exit   unknown (timed out after {} ms; the direct child was killed — its own children \
             may survive — and the log so far is at {})",
            timeout.as_millis(),
            outcome.log.display()
        )),
        (false, Some(code)) => lines.push(format!("exit   {code}")),
        (false, None) => lines.push("exit   unknown (the process was signalled)".to_owned()),
    }
    lines.extend(observed.lines.iter().cloned());
    if let Some(next) = next_step(outcome.timed_out, outcome.code, &observed.lines) {
        lines.push(next);
    }
    lines
}

/// Test this tree's faces by running them, and report only what the run said.
/// 跑一遍这棵树的面来做测试，并只报出这次运行说了什么。
pub(crate) fn check(root: &Path, arguments: &Value) -> Result<String, String> {
    let face = arguments
        .get("face")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|face| !face.is_empty())
        .ok_or_else(|| {
            "nichlink.check requires `face`: `default`, `all`, or one feature name. There is no \
             default here because choosing the face is what this tool is for"
                .to_owned()
        })?;
    if face.contains(|character: char| character.is_whitespace() || character == '-') {
        return Err(format!(
            "`face` is `default`, `all`, or one feature name, not `{face}`"
        ));
    }
    let timeout = Duration::from_millis(
        arguments
            .get("timeout_ms")
            .and_then(Value::as_u64)
            .unwrap_or(DEFAULT_TIMEOUT_MS)
            .clamp(MIN_TIMEOUT_MS, MAX_TIMEOUT_MS),
    );
    let mut command = Command::new("cargo");
    command.arg("test").current_dir(root);
    match face {
        "default" => {}
        "all" => {
            command.arg("--all-features");
        }
        feature => {
            command.arg("--features").arg(feature);
        }
    }
    let log = out_dir(root).join(format!("check-{face}.log"));
    let outcome = run_command(&mut command, timeout, &log)?;
    let observed = observation(&outcome.log)?;
    // The verdict is line 1 (see `verdict`), ahead of everything the run produced, because round 7
    // measured a reader taking the one-shot client's `0` for a green face while the only failure
    // report sat on line six. The `exit` line below is the run's own code, and it stays.
    // 判定是第 1 行（见 `verdict`），排在这次运行产出的一切之前，因为第七轮量到有读者把一次性客户端的
    // `0` 当成绿面，而唯一那句失败报告在第六行。下面的 `exit` 行是运行自己的码，保留。
    let mut lines = head(root, face, &outcome, timeout, &observed);
    // The tree-wide half: a symptom narrows the scope and this tool answers for a face; an open
    // "are there other problems" has nothing to narrow it, and the census is what answers that
    // without anybody hand-sweeping the tree.
    // 全树那一半：症状会收窄范围，而本工具按面作答；开放式的问题"还有别的问题吗"没有东西替它收窄，
    // 总账就是那个不必有人手工扫树也能回答它的东西。
    //
    // It is **sampled** by default and whole on request. The face's own verdict is what a caller
    // ran this tool for; the census is the open question's answer, and a caller that asked for one
    // face was paying for both. The sample keeps the first rows — the constants and the nearest
    // findings — plus the two lines that say what the census is *not*, because those are the ones
    // that stop an inventory from reading as a measurement.
    // 它默认**抽样**、按要求给全表。调用方跑这个工具为的是**面自己的结论**，而总账是那个开放式问题的
    // 答案——只问了一个面的调用方在为两者付钱。样本保留最前面的行——常量与最近的发现——外加说出这份
    // 总账**不是**什么的那两行，因为正是那两行不让一份清单被读成一次量度。
    match crate::mcp::claims::census(
        root,
        arguments.get("census").and_then(Value::as_bool) == Some(true),
    ) {
        Ok(census) => {
            if arguments.get("census").and_then(Value::as_bool) == Some(true) {
                lines.extend(census);
            } else {
                lines.extend(census_sample(&census));
            }
        }
        Err(reason) => lines.push(format!("census unavailable ({reason})")),
    }
    Ok(lines.join("\n"))
}

/// How many census rows the default `check` keeps before it points at `census: true`.
/// 默认的 `check` 在指向 `census: true` 之前保留多少行总账。
const CENSUS_SAMPLE: usize = 5;

/// The census rows the default reply carries: the head of the table, and the lines that say what
/// the table is not.
/// 默认回复携带的总账行：表头一段，以及说出这份表**不是**什么的那几行。
///
/// The boundary lines are kept unconditionally rather than by position, because they are not rows
/// of the inventory — they are the sentence that keeps the inventory from being read as one, and a
/// sample that dropped them would be exactly the false green the census exists against. What is
/// left out is counted through [`crate::mcp::truncation::withheld`], the one truncation outlet, so
/// the withheld count, the cap and the way to the rest are all named.
/// 边界行是**无条件**保留的，而不是按位置保留：它们不是这份清单的行，而是让这份清单不被读成一次量度
/// 的那句话，丢掉它们的抽样恰恰是这份总账要对付的那种假绿。省下的部分经
/// [`crate::mcp::truncation::withheld`]——唯一那个截断出口——计数，因此被扣下的数量、上限与拿到其余
/// 部分的办法都会被点名。
fn census_sample(census: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = census.iter().take(CENSUS_SAMPLE).cloned().collect();
    for line in census {
        if line.starts_with("  not covered") && !kept.contains(line) {
            kept.push(line.clone());
        }
    }
    let omitted = census.len().saturating_sub(kept.len());
    if omitted > 0 {
        kept.push(withheld(
            omitted,
            census.len(),
            CENSUS_SAMPLE,
            "census lines",
            "pass `census: true` for the whole table",
        ));
    }
    kept
}

/// What to do about a run that did not pass, in the answer rather than in a preamble.
/// 对于一次没有通过的运行该怎么办——写在答案里，而不是开场白里。
///
/// A failing assertion's message is a string literal in the tree, so the cheapest next call is the
/// literal search that finds where it is produced; that pointer is the reason `literal` exists and
/// the one shape of guidance this bridge has measured to work.
/// 失败断言的那句话是树里的一个字符串字面量，因此下一个最便宜的调用就是找出它在哪产出的字面检索；这条指引
/// 正是 `literal` 存在的理由，也是本桥实测唯一生效的那种指引形态。
fn next_step(timed_out: bool, code: Option<i32>, lines: &[String]) -> Option<String> {
    if timed_out || code == Some(0) {
        return None;
    }
    // The hint is **instantiated from this answer's own state**: the literal it tells the caller to
    // search is taken out of the `why` line this reply just printed, so the next call is copyable
    // instead of a template. Measured (W8): `next` lines were followed 28% of the time, and the only
    // one followed reliably named a concrete thing to read — a placeholder leaves the filling-in to
    // the caller, which is the thinking the hint exists to remove.
    // 这条提示**由这次答案自己的状态实例化**：它让调用方去搜的那个字面量，取自这次回复刚打印出来的 `why` 行
    // ——于是下一次调用是**可粘贴的**，而不是一个模板。实测（W8）：`next` 行被采纳 28%，而唯一被稳定采纳的是
    // 那条点名了"具体要读什么"的——占位符把"填空"留给调用方，而填空正是这条提示存在来省掉的思考。
    let search = match literal_to_search(lines) {
        Some(phrase) => format!("`search {{literal: \"{phrase}\"}}`"),
        None => "`search {literal: \"<a short phrase from the assertion>\"}`".to_owned(),
    };
    Some(format!(
        "next   the `why` lines above are the failing assertion's own words: a short, stable \
         phrase from one is a string literal in this tree, so {search} finds the line that produced \
         it. If two red things may be independent, two green runs are not the evidence: fix one and \
         re-run, and say which red survived. And a probe you built yourself that disagrees with the \
         source is a reason to re-read that line (`read`, `search {{literal}}`) before rebuilding — a \
         second look is cheaper than a second build"
    ))
}

/// The literal worth searching for, taken out of the `why` line this reply carries.
/// 值得拿去搜的那个字面量，取自这次回复携带的 `why` 行。
///
/// What has to be dropped, and why each part is there: the harness prefix (`thread '<name>'
/// (<pid>) panicked at <file>:<line>: `) carries a **process id**, which no source contains; the
/// generic `assertion \`left == right\` failed:` clause is shared by many tests and matches the wrong
/// one; and the numbers the framework appends are the values, not the words. What is left is the
/// message the test itself wrote — the phrase that is actually a string literal in this tree.
/// 必须丢掉的、以及每一部分为什么在：harness 前缀（`thread '<名>' (<pid>) panicked at <文件>:<行>: `）
/// 里带着**进程号**，任何源码里都没有它；通用的 `assertion \`left == right\` failed:` 从句被许多测试共用，
/// 会匹到错的那一个；框架追加的数字是取值，不是词。剩下的才是测试自己写下的那句话——也就是这棵树里真正
/// 作为字符串字面量存在的短语。
fn literal_to_search(lines: &[String]) -> Option<String> {
    const WORDS: usize = 6;
    let message = lines
        .iter()
        .find_map(|line| line.strip_prefix("why    "))?
        .split_once(": ")?
        .1;
    let tail = match message.split_once("panicked at ") {
        Some((_, rest)) => rest.split_once(": ").map_or(rest, |(_, tail)| tail),
        None => message,
    };
    let tail = match tail.split_once("failed: ") {
        Some((_, rest)) => rest,
        None => tail,
    };
    let phrase = tail
        .split_whitespace()
        .filter(|word| {
            !word
                .chars()
                .all(|character| character.is_ascii_digit() || "(),.:;`'\"".contains(character))
        })
        .take(WORDS)
        .collect::<Vec<_>>()
        .join(" ");
    (phrase.split_whitespace().count() >= 3).then(|| phrase.replace('"', "'"))
}

/// The exact flags the command carried, so the answer names what it ran.
/// 命令实际带的开关，让答案说清它跑的是什么。
fn flags(face: &str) -> String {
    match face {
        "default" => String::new(),
        "all" => " --all-features".to_owned(),
        feature => format!(" --features {feature}"),
    }
}

/// The failing assertion's own words, per failing test, read out of the same log.
/// 失败断言自己的话，按失败的测试逐个，从同一份日志里读出。
///
/// Cargo prints them under `---- <name> stdout ----`, and they are the one thing the reply did not
/// carry. Measured in W8's h1 (step 5): the arm got `verdict failed` and the failing test's name,
/// then read the raw log itself — "Let's read the check log raw" — while this module's own `next`
/// line already said the message is a string in the tree. **Knowing what is missing and making the
/// caller fetch it anyway is a call spent on something we had in hand.**
/// cargo 把这句话印在 `---- <name> stdout ----` 下面，而它正是回复没有携带的那样东西。W8 的 h1 第 5 步
/// 量到：那一臂拿到 `verdict failed` 与失败的测试名，随后自己去读原始日志——「Let's read the check log
/// raw」——而本模块自己的 `next` 行早就写着"那句话是树里的一个字符串"。**知道自己少给了什么、却让调用方
/// 自己去取，就是把一次调用花在我们手里已有的东西上。**
fn why_lines(text: &str, names: &[String]) -> Vec<String> {
    const MESSAGE_LINES: usize = 2;
    const MESSAGE_CHARS: usize = 200;
    let lines = text.lines().collect::<Vec<_>>();
    let mut out = Vec::new();
    for name in names {
        let marker = format!("---- {name} stdout ----");
        let Some(start) = lines.iter().position(|line| line.trim() == marker) else {
            continue;
        };
        let mut said = Vec::new();
        for line in lines.iter().skip(start + 1) {
            let trimmed = line.trim();
            if trimmed.starts_with("----") || trimmed.starts_with("note:") {
                break;
            }
            if trimmed.is_empty() {
                if said.is_empty() {
                    continue;
                }
                break;
            }
            said.push(trimmed);
            if said.len() == MESSAGE_LINES {
                break;
            }
        }
        if said.is_empty() {
            continue;
        }
        let message = said.join(" ");
        let message = if message.chars().count() > MESSAGE_CHARS {
            let cut = message.chars().take(MESSAGE_CHARS).collect::<String>();
            format!("{cut}…")
        } else {
            message
        };
        out.push(format!("why    {name}: {message}"));
    }
    out
}

/// What the log said, as the run said it — and how many `test result:` lines it had, because a log
/// with none is not a pass and the verdict line has to be able to say so.
/// 日志说了什么，按运行自己说的样子——以及它有几条 `test result:` 行，因为没有的那种日志不是通过，
/// 而判定行必须能说出这件事。
fn observation(log: &Path) -> Result<Observation, String> {
    let mut file =
        File::open(log).map_err(|error| format!("cannot read {}: {error}", log.display()))?;
    let mut text = String::new();
    let _ = file.seek(SeekFrom::Start(0));
    file.read_to_string(&mut text)
        .map_err(|error| format!("cannot read {}: {error}", log.display()))?;
    let results = text
        .lines()
        .filter(|line| line.contains("test result:"))
        .map(|line| line.trim().to_owned())
        .collect::<Vec<_>>();
    let mut failed = Vec::new();
    for line in BufReader::new(text.as_bytes())
        .lines()
        .map_while(Result::ok)
    {
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("test ")
            && let Some(name) = name.strip_suffix(" ... FAILED")
        {
            failed.push(name.to_owned());
        }
    }
    let mut lines = Vec::new();
    if results.is_empty() {
        lines.push(
            "result none (no `test result:` line in the log: nothing ran, and that is not a pass)"
                .to_owned(),
        );
    } else {
        lines.extend(
            results
                .iter()
                .take(SAMPLE)
                .map(|line| format!("result {line}")),
        );
        if results.len() > SAMPLE {
            lines.push(withheld(
                results.len() - SAMPLE,
                results.len(),
                SAMPLE,
                "result lines",
                "read the log named above",
            ));
        }
    }
    lines.extend(
        failed
            .iter()
            .take(SAMPLE)
            .map(|name| format!("failed {name}")),
    );
    if failed.len() > SAMPLE {
        lines.push(withheld(
            failed.len() - SAMPLE,
            failed.len(),
            SAMPLE,
            "failing tests",
            "read the log named above",
        ));
    }
    // The assertion's own words ride with the name of the test that produced them: the caller asked
    // what failed, and the answer is these lines, not a pointer to where they live.
    // 断言自己的话与产出它的测试名同行给出：调用方问的是"什么失败了"，而答案就是这几行，而不是"它们住在哪"
    // 的一条指引。
    lines.extend(why_lines(&text, &failed));
    Ok(Observation {
        lines,
        results: results.len(),
    })
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod check_tests;
