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
    let mut lines = vec![
        format!("check  cargo test{}", flags(face)),
        format!("face   {face}"),
        format!("root   {}", root.display()),
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
    lines.extend(observation(&outcome.log)?);
    Ok(lines.join("\n"))
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

/// What the log said, as the run said it.
/// 日志说了什么，按运行自己说的样子。
fn observation(log: &Path) -> Result<Vec<String>, String> {
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
    Ok(lines)
}

#[cfg(test)]
#[path = "check_tests.rs"]
mod check_tests;
