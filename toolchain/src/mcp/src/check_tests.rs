//! Pins for the observed run: a failed run reports what it said, a timeout is `unknown` rather than
//! a pass, and a log with no result line says nothing ran.
//! 被观测运行的钉子：失败的运行如实报出它说的，超时是 `unknown` 而不是通过，没有结果行的日志说"什么都没跑"。

use std::process::Command;
use std::time::Duration;

use super::{next_step, observation, run_command};

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
    let joined = observed.join("\n");
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
    let joined = observation(&log).expect("the log reads").join("\n");
    assert!(joined.contains("nothing ran"), "{joined}");
    assert!(joined.contains("not a pass"), "{joined}");
    let _ = std::fs::remove_dir_all(&root);
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
