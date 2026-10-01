//! Pins for the observed run: a failed run reports what it said, a timeout is `unknown` rather than
//! a pass, and a log with no result line says nothing ran.
//! 被观测运行的钉子：失败的运行如实报出它说的，超时是 `unknown` 而不是通过，没有结果行的日志说"什么都没跑"。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use super::{Observation, RunOutcome, head, next_step, observation, run_command};

/// A throwaway directory for the logs.
/// 一个一次性的日志目录。
fn scratch(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "nichlink-mcp-check-{label}-{}-{sequence}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("scratch dir");
    root
}

/// A failing run comes back with its exit code and its own result line, and nothing invented.
/// 失败的运行带着自己的退出码与结果行回来，没有编造的东西。
#[test]
fn a_failing_run_reports_what_it_said() {
    let root = scratch("failed");
    let log = root.join("check-all.log");
    let outcome = run_command(
        Command::new("sh").arg("-c").arg(
            "printf 'test result: FAILED. 1 passed; 1 failed; 0 ignored\\ntest alpha ... FAILED\\n'; \
             exit 101",
        ),
        Duration::from_secs(30),
        &log,
    )
    .expect("the command runs");
    assert_eq!(outcome.code, Some(101), "{:?}", outcome.code);
    assert!(!outcome.timed_out);
    let observed = observation(&log).expect("the log reads");
    assert_eq!(observed.results, 1, "one `test result:` line was read");
    let joined = observed.lines.join("\n");
    assert!(joined.contains("1 passed; 1 failed"), "{joined}");
    assert!(joined.contains("failed alpha"), "{joined}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A timeout is `unknown`: it says what was killed and that the run is not evidence of a pass.
/// 超时是 `unknown`：它说出杀的是什么，并说明这次运行不是通过的证据。
#[test]
fn a_timeout_is_unknown_and_never_a_pass() {
    let root = scratch("timeout");
    let log = root.join("check-default.log");
    let outcome = run_command(
        Command::new("sh").arg("-c").arg("sleep 30"),
        Duration::from_millis(1_200),
        &log,
    )
    .expect("the command runs");
    assert!(outcome.timed_out, "{:?}", outcome.code);
    assert!(
        outcome.elapsed < Duration::from_secs(10),
        "it was killed early"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A log with no result line reports that nothing ran, instead of reading as an empty success.
/// 没有结果行的日志报出"什么都没跑"，而不是被读成一次空的成功。
#[test]
fn a_log_without_a_result_line_says_nothing_ran() {
    let root = scratch("nothing");
    let log = root.join("check-extra.log");
    std::fs::write(
        &log,
        "   Compiling something v0.1.0\nerror: could not compile\n",
    )
    .expect("log");
    let joined = observation(&log).expect("the log reads").lines.join("\n");
    assert!(joined.contains("nothing ran"), "{joined}");
    assert!(joined.contains("not a pass"), "{joined}");
    let _ = std::fs::remove_dir_all(&root);
}

/// A finished command, as the verdict sees it.
/// 一条结束了的命令，按判定看到的样子。
fn outcome(code: Option<i32>, timed_out: bool) -> RunOutcome {
    RunOutcome {
        code,
        timed_out,
        elapsed: Duration::from_millis(200),
        log: PathBuf::from("/tree/target/nichlink/out/check-default.log"),
    }
}

/// One observed log, with the number of `test result:` lines the verdict reads.
/// 一份被观测的日志，带上判定要读的 `test result:` 行数。
fn observed(lines: &[&str], results: usize) -> Observation {
    Observation {
        lines: lines.iter().map(|line| (*line).to_owned()).collect(),
        results,
    }
}

/// The verdict is in the first two lines of the reply, so a reader that stops at the top cannot read
/// a failing face as green.
/// 判定在回复头两行里，因此只在开头停下的读者不会把失败的面读成绿。
///
/// This is the pin for the defect round 7 measured on the frozen fixture `target/round7/s5`: the
/// failure was on line six (`exit   101`) while the one-shot client exited `0`. Both halves are
/// pinned here — the head says `failed`, and the `exit` line is still there for the code itself.
/// 这条钉子钉住第七轮在冻结夹具 `target/round7/s5` 上量到的缺陷：失败在第六行（`exit   101`），而一次性
/// 客户端以 `0` 退出。两半都在这里钉住——头部说 `failed`，而 `exit` 行仍保留着那个码本身。
#[test]
fn the_first_two_lines_are_the_verdict() {
    let failing = head(
        Path::new("/tree"),
        "default",
        &outcome(Some(101), false),
        Duration::from_secs(900),
        &observed(
            &["result test result: FAILED. 0 passed; 3 failed; 0 ignored"],
            1,
        ),
    );
    let top = failing
        .iter()
        .take(2)
        .cloned()
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        top.contains("verdict  failed (cargo exit 101)"),
        "the verdict is on the reply's first line: {top}"
    );
    assert!(
        !top.contains("passed"),
        "a failing run cannot be read as green from the head: {top}"
    );
    assert!(
        failing.iter().any(|line| line.starts_with("exit   101")),
        "the `exit` line stays, with the run's own code: {failing:?}"
    );

    let passing = head(
        Path::new("/tree"),
        "default",
        &outcome(Some(0), false),
        Duration::from_secs(900),
        &observed(&["result test result: ok. 1 passed; 0 failed"], 1),
    );
    assert!(
        passing[0].starts_with("verdict  passed (cargo exit 0)"),
        "{passing:?}"
    );

    // A log with no `test result:` line is not a pass even when cargo exited 0, and a killed run is
    // not a pass either: both are `unknown`, and the first line says so.
    // 没有 `test result:` 行的日志即使 cargo 以 0 退出也不是通过，被杀死的运行同样不是：两者都是
    // `unknown`，而第一行就这么说。
    let nothing = head(
        Path::new("/tree"),
        "default",
        &outcome(Some(0), false),
        Duration::from_secs(900),
        &observed(&["result none (nothing ran, and that is not a pass)"], 0),
    );
    assert!(
        nothing[0].starts_with("verdict  unknown (cargo exit 0"),
        "{nothing:?}"
    );
    let timed = head(
        Path::new("/tree"),
        "default",
        &outcome(None, true),
        Duration::from_secs(900),
        &observed(&[], 0),
    );
    assert!(timed[0].starts_with("verdict  unknown"), "{timed:?}");
}

/// A run that did not pass says where to look next, and one that passed says nothing extra.
/// 没有通过的运行会说下一步去哪找，而通过的运行不多说一句。
#[test]
fn a_failing_run_says_where_to_look_next() {
    let failing = next_step(false, Some(101)).expect("a failing run has a next step");
    assert!(failing.contains("search {literal"), "{failing}");
    assert!(
        next_step(false, Some(0)).is_none(),
        "a passing run needs no pointer"
    );
    assert!(
        next_step(true, None).is_none(),
        "a timeout is unknown, not a failure to chase"
    );
}
