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
//! - **Naming a face is the point, but the default is a convenience.** A caller who says no face
//!   gets `default`: the round measured 12 such calls and **6 of them left the tool for a shell
//!   `cargo test`** instead of naming one — a refusal that costs a turn to learn nothing. The
//!   face still decides the answer, and `status` says which faces exist.
//!   **点名面是要点，但缺省是便利。** 不说面的调用方拿到 `default`：那一轮量到 12 次这样的调用，
//!   其中 **6 次离开工具去 shell 跑 `cargo test`**，而不是补上一个面名——一次白花一轮的拒绝。面仍然
//!   决定答案，而 `status` 说出有哪些面。
//! - **A timeout is `unknown`, never a pass**, and the answer says what was killed (the direct child
//!   only — its own children may survive) and where the log so far is.
//!   **超时是 `unknown`，绝不是通过**，而且答案说清杀的是什么（只杀直接子进程——它自己的子进程可能还活着）
//!   以及到目前为止的日志在哪。
//! - **Nothing ran is not a pass.** A log with no `test result:` line reports that, rather than an
//!   empty success.
//!   **什么都没跑不是通过。** 没有 `test result:` 行的日志会这么说，而不是给一个空的成功。
//!
//! online: a check has to see the sources and the build output as they are right now — that is what `check`/`verify` are for, and the record would be a second opinion about the thing being checked.

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
    /// The names of the tests that failed, in the order the log carried them.
    /// 失败的测试名，按日志里的顺序排列。
    ///
    /// Audit `W5-5`: routing by failure **kind** needs to know which tests failed, not only how many
    /// result lines there were. A family-shaped red and a missing build product are the two cases
    /// where the cheapest next call is not the literal search the default hint names.
    /// 审计 `W5-5`：按失败**种类**路由需要知道失败的是哪些测试，而不只是有几条结果行。同族形状的红与
    /// 缺席的构建产物，正是"最便宜的下一次调用不是默认那条字面检索"的两种情形。
    pub(crate) failed: Vec<String>,
    /// The first `path:line` a compiler or runtime diagnostic named, when the log carried one.
    /// 日志里编译器或运行期诊断点名的第一个 `path:line`（若日志里有）。
    ///
    /// This is what makes the "nothing ran" route a **call** rather than a shrug: `why` needs a
    /// location, and rustc's own `--> file:line` is one.
    /// 正是它让"什么都没跑"那条路由成为一次**调用**而不是耸肩：`why` 需要一个位置，而 rustc 自己的
    /// `--> file:line` 就是一个。
    pub(crate) first_location: Option<String>,
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
    if let Some(next) = next_step(face, outcome.timed_out, outcome.code, observed) {
        lines.push(next);
    }
    lines
}

/// Test this tree's faces by running them, and report only what the run said.
/// 跑一遍这棵树的面来做测试，并只报出这次运行说了什么。
pub(crate) fn check(root: &Path, arguments: &Value) -> Result<String, String> {
    // No `face` means the default face, not a refusal. The round measured what the refusal cost:
    // 12 calls arrived without `face` and 6 of them did not correct the shape but left the tool for
    // a shell `cargo test` instead — while the two faces a question usually needs share one red, so
    // the second call bought the same answer. Choosing a face is still what this tool is *for*: pass
    // one whenever you mean a specific face.
    // 没给 `face` 就是默认面，不是拒绝。那一轮量到这次拒绝的代价：12 次调用没带 `face`，其中 6 次没有
    // 改对形状，而是离开工具去跑 shell 的 `cargo test`——而一道题通常需要的两个面常常共享同一条红，
    // 第二次调用买到的是同一个答案。选面仍然是这工具的**意义**：想指定某个面时就显式传它。
    let face = arguments
        .get("face")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|face| !face.is_empty())
        .unwrap_or("default");
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
    // Cargo writes its products inside the tree unless told otherwise, and the round measured what
    // that costs: `check` on a restored tree leaves a `target/` whose artifacts are NEWER than the
    // sources, so the next `cargo test` (or the next `check`) reuses the old binary and reports a
    // green that belongs to the previous round's code. A verifier asked to take every baseline with
    // an out-of-tree target had no way to do it through this tool at all.
    // 不特别指定时，cargo 把产物写在树内，而那一轮量到了它的代价：在还原过的树上跑 `check`，会留下一个
    // 产物**新于**源码的 `target/`，于是下一次 `cargo test`（或下一次 `check`）复用旧二进制、报出一个
    // 属于**上一轮代码**的绿。有验证者被要求"基线一律用树外 target"，而通过这个工具**根本做不到**。
    if let Some(directory) = arguments.get("target_dir").and_then(Value::as_str) {
        command.env("CARGO_TARGET_DIR", directory);
    }
    match face {
        "default" => {}
        "all" => {
            command.arg("--all-features");
        }
        feature => {
            command.arg("--features").arg(feature);
        }
    }
    // The log is the second reason a tree gets a `target/`: `out_dir(root)` is *inside* the tree, so
    // even with cargo redirected the tool would leave `<root>/target/nichlink/out/` behind. When the
    // caller names a tree-external target, the log rides with it.
    // 日志是"树里长出 `target/`"的第二个原因：`out_dir(root)` 在树**内**，所以即使 cargo 被改道，工具
    // 仍会留下 `<root>/target/nichlink/out/`。调用方指定了树外 target 时，日志跟着它走。
    let log = match arguments.get("target_dir").and_then(Value::as_str) {
        Some(directory) => std::path::Path::new(directory).join(format!("check-{face}.log")),
        None => out_dir(root).join(format!("check-{face}.log")),
    };
    let outcome = run_command(&mut command, timeout, &log)?;
    let observed = observation(
        &outcome.log,
        arguments.get("verbose").and_then(Value::as_bool) == Some(true),
    )?;
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
/// The column a head opens, as the name before its colon.
/// 一个栏头开启的那一栏，取它冒号前的名字。
fn column_name(line: &str) -> String {
    line.split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_end_matches(':')
        .to_owned()
}

/// Whether one census line is a column head rather than a row under one.
/// 一条总账行是**栏头**，而不是它下面的数据行。
///
/// The rule is the same one the sample has always used — the first word ends with `:` — factored
/// out so the heads pass and the "one row under each head" pass cannot drift apart.
/// 规则就是抽样一直用的那条——首个词以 `:` 结尾——抽成函数，好让"取栏头"与"每栏取一行"两遍不会漂移。
fn is_column_head(line: &str) -> bool {
    line.split_whitespace()
        .next()
        .is_some_and(|word| word.ends_with(':'))
}

fn census_sample(census: &[String]) -> Vec<String> {
    // The sample keeps the **head of every column**, its first few rows, and every column's
    // "not covered" line. A flat take of the first five lines hid a whole column once: on the `g4`
    // tree the five kept lines were all the numeric-constant column, so the default answer never
    // showed `branch-level:` or the reachability summary at all — and that level's question *is*
    // the branch column. The column heads are the unindented lines (rows are indented by two
    // spaces), and each one carries its own count, which is what a reader needs in order to know
    // whether the part they did not get could matter.
    // 抽样保留**每一栏的开头**、它的头几行、以及每栏的 `not covered` 行。曾经"平取前五行"把整整一栏藏掉：
    // 在 `g4` 那棵树上留下的五行全属数值常量栏，于是默认答案里**根本看不到** `branch-level:` 与可达性汇总
    // ——而那一关的问题**就是**分支栏。栏头就是不缩进的那些行（数据行缩进两个空格），而每个栏头自带计数，正是
    // 读者判断"没拿到的部分会不会要紧"所需要的东西。
    // A column **head** is the line whose first word ends with `:` (`branch-level:`, `declarations:`,
    // `test-reachable:`, …); the rows under it start with a label or a backtick. Every line in this
    // vector is indented, which is why the rule cannot key on indentation.
    // **栏头**是首个词以 `:` 结尾的那些行（`branch-level:`、`declarations:`、`test-reachable:`…），它下面的
    // 数据行以标签或反引号开头。这个向量里每一行都带缩进，所以规则不能按缩进判定。
    let mut kept: Vec<String> = Vec::new();
    for line in census {
        if is_column_head(line) && !kept.contains(line) {
            kept.push(line.clone());
        }
    }
    // **One row under every head**, before the flat take. A column whose head survives while its
    // decisive row is withheld is a column the reader cannot dispose of without a second call —
    // and on a census whose first five lines all belong to one column, that is exactly what
    // happened: the `branch-level:` head came through with its count and the single row naming the
    // dead arm did not, so "dispose of every column" was impossible from this reply. The audit
    // registered that as a defect (`the_sample_keeps_the_branch_head_but_drops_the_branch_row`
    // pinned the broken shape); this keeps one row per column so every head arrives with something
    // concrete under it.
    // **每栏保留栏头下面的一行**，先于平取。栏头活下来而它那一行判据被扣下，等于那一栏要第二跳才能处置
    // ——而一份前五行全属同一栏的总账正是这样：`branch-level:` 栏头带着计数过来了，唯一那行点名死臂的
    // 数据行没有，于是"逐栏处置"在这份回复里根本做不到。审计把这一点登记为缺陷（那条钉子钉的是坏形状）；
    // 这里改成每栏留一行，让每个栏头都带着具体的东西到达。
    let mut served: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut current: Option<String> = None;
    for line in census {
        if is_column_head(line) {
            current = Some(column_name(line));
            continue;
        }
        // A row belongs to the **last head before it**, not to "the line after a head": the heads
        // arrive consecutively, so index arithmetic takes the next column's row (measured — the
        // branch column's "row" came out as a `respelled 1000` line, and the row naming the dead
        // arm only survived by luck of the flat take). Keeping the first row *per column* is what
        // makes "one row for every column" true on a census big enough for the cap to bite.
        // 一行属于它**之前的最后一个栏头**，而不是"栏头后面的那一行"：各栏头是连续到达的，用下标算术会取到
        // 下一栏的行（实测——分支栏的"行"取成了 `respelled 1000`，而点名死臂的那行只是靠平取侥幸活下来）。
        // 保留**每栏的第一行**，才让"每栏都有一行"在总账大到上限真的咬下去时依然成立。
        let Some(head) = current.clone() else {
            continue;
        };
        if served.insert(head) && !kept.contains(line) {
            kept.push(line.clone());
        }
    }
    for line in census.iter().take(CENSUS_SAMPLE) {
        if !kept.contains(line) {
            kept.push(line.clone());
        }
    }
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
            "census rows",
            "pass `census: true` for the whole table (every column head is already here with its count)",

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
fn next_step(
    face: &str,
    timed_out: bool,
    code: Option<i32>,
    observed: &Observation,
) -> Option<String> {
    if timed_out || code == Some(0) {
        return None;
    }
    let lines = &observed.lines;
    // Audit `W5-5`: route by **what kind of red this is**, because the cheapest next call differs by
    // kind, and a battery that has to rediscover which kind it is pays for that every run. The
    // default (an assertion's words) keeps the literal search below; the other two kinds are the ones
    // this bridge already has a purpose-built tool for.
    // 审计 `W5-5`：按**这是哪一种红**路由，因为最便宜的下一次调用因种类而异，而一套每次都要重新发现
    // "这是哪一种"的电池会为此反复付费。默认（断言自己的话）走下面的字面检索；另外两种正是本桥已经有
    // 专用工具的情形。
    if observed.results == 0 {
        let at = observed
            .first_location
            .clone()
            .unwrap_or_else(|| format!("{face}/<the file the error names>:<line>"));
        return Some(format!(
            "next   **nothing ran**: the log has no `test result:` line, so this red is the build's, \
             not a test's — `why {{at: \"{at}\"}}` gathers the upstream facts at that location, and \
             `check {{verbose: true}}` prints the compiler's own lines whole"
        ));
    }
    // The signal is the tree's **own** vocabulary for that family, not a guess: the kernel exports
    // the runtime-check nouns, and a red in this family carries the one that names the check. The
    // words are matched case-insensitively because a failure message is prose an author wrote.
    // 信号是这棵树**自己的**族词汇，不是猜的：内核导出了运行期校验的名词，而这个族的红携带点名那条校验
    // 的那一个。匹配不分大小写，因为失败消息是作者写的散文。
    let coordinates = observed
        .failed
        .iter()
        .chain(observed.lines.iter())
        .any(|line| {
            let lowered = line.to_lowercase();
            // The check's own spelling, taken from the kernel rather than typed here.
            // 这条校验自己的拼写，取自内核而不是在这里手打。
            lowered.contains(nichlink_kernel::RuntimeCheckSpec::CoordinatesInViewport.name())
                || lowered.contains("coordinate")
                || lowered.contains("viewport")
        });
    if coordinates {
        let parent = face
            .trim_end_matches('/')
            .rsplit_once('/')
            .map_or("root", |(above, _)| above);
        return Some(format!(
            "next   this red is in the **coordinates family**: `consistency {{parent: \"{parent}\"}}` \
             compares the siblings that must agree on those values and names the one that drifted, \
             and it hands back the `fix` request that pulls it back — one call instead of a hunt \
             through the family by hand"
        ));
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
fn observation(log: &Path, verbose: bool) -> Result<Observation, String> {
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
        // Audit `W1-3`: a run has one group per test binary (unit tests, then one per integration
        // file), and every group prints its own `test result:` line. The ones that carry a failure
        // are the answer and are printed one by one; the groups that **passed** are a count and the
        // first line, because a reader who needs the rest asked for `verbose: true` — that is the
        // fold, and it is where the reply's tail came from on a wide tree.
        // 审计 `W1-3`：一次运行每个测试二进制品一组（单元测试，然后每个集成文件一组），每组印自己那行
        // `test result:`。带失败的那几行就是答案、逐行印出；**通过**的那些折成一个计数加第一行，因为需要
        // 其余部分的读者会要 `verbose: true`——这就是那次折叠，也正是宽树上回复尾部的来路。
        let (passed, failed): (Vec<&String>, Vec<&String>) = results
            .iter()
            .partition(|line| line.contains("test result: ok"));
        for line in failed.iter().take(SAMPLE) {
            lines.push(format!("result {line}"));
        }
        if failed.len() > SAMPLE {
            lines.push(withheld(
                failed.len() - SAMPLE,
                failed.len(),
                SAMPLE,
                "failing result lines",
                "read the log named above",
            ));
        }
        if passed.len() == 1 || verbose {
            for line in passed.iter().take(SAMPLE) {
                lines.push(format!("result {line}"));
            }
            if passed.len() > SAMPLE {
                lines.push(withheld(
                    passed.len() - SAMPLE,
                    passed.len(),
                    SAMPLE,
                    "passing result lines",
                    "read the log named above",
                ));
            }
        } else if !passed.is_empty() {
            lines.push(format!(
                "result {} group(s) passed — pass `verbose: true` for each line; first: {}",
                passed.len(),
                passed[0]
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
    // The routing facts, read from the same text: a diagnostic's own location (rustc's `--> file:line`)
    // and whether the failing tests look like the coordinates family. Both are **observed**, never
    // guessed from the face's name.
    // 路由所需的事实，同样取自这段文本：诊断自己的位置（rustc 的 `--> file:line`），以及失败的测试是否
    // 像坐标族。两者都是**观察到的**，从不按面的名字去猜。
    let first_location = text.lines().find_map(|line| {
        let trimmed = line.trim();
        let rest = trimmed.strip_prefix("--> ")?;
        let (path, _) = rest.rsplit_once(':')?;
        // `then` rather than `Some(..).filter(|_| ..)`: the predicate never looks at the value, and
        // clippy 1.99 says so (the filter form reads as if it did).
        // 用 `then` 而不是 `Some(..).filter(|_| ..)`：谓词根本不看那个值，clippy 1.99 正是这么说的
        // （filter 那种写法读起来像它会看）。
        (!path.is_empty()).then(|| rest.trim().to_owned())
    });
    let failed = failed
        .iter()
        .map(|name| (*name).clone())
        .collect::<Vec<_>>();
    Ok(Observation {
        lines,
        results: results.len(),
        failed,
        first_location,
    })
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod check_tests;
