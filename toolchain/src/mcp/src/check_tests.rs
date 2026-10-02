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
    // The third argument is this reply's own lines: since T-22 the hint is instantiated from them,
    // so an empty slice is what a reply with no `why` line carries — and that is the shape that must
    // still say where to look.
    // 第三个参数是这次回复自己的那些行：自 T-22 起这条提示由它们实例化，因此空切片就是"没有 `why` 行"的
    // 回复所携带的形状——而正是它仍须说出下一步去哪找。
    let failing = next_step(false, Some(101), &[]).expect("a failing run has a next step");
    assert!(failing.contains("search {literal"), "{failing}");
    assert!(
        next_step(false, Some(0), &[]).is_none(),
        "a passing run needs no pointer"
    );
    assert!(
        next_step(true, None, &[]).is_none(),
        "a timeout is unknown, not a failure to chase"
    );
}

/// The failing assertion's own words ride with the failing test's name.
/// 失败断言自己的话与失败测试的名字同行给出。
///
/// Measured in W8's h1 (step 5): the reply carried `verdict failed` and `failed <name>`, and the arm
/// then read the raw log itself ("Let's read the check log raw") to get the assertion — one call and
/// one `sed` spent on a line we had already read. The log below is that one, in cargo's own shape;
/// it is written rather than shelled out because the panic line contains a quote that no `sh -c`
/// quoting survives intact.
/// W8 的 h1 第 5 步量到：回复带了 `verdict failed` 与 `failed <name>`，那一臂随后自己去读原始日志
/// （「Let's read the check log raw」）拿断言——一次调用加一次 `sed`，花在我们早已读过的那一行上。下面这份
/// 日志就是那一份，按 cargo 自己的形状；直接写盘而不是走 shell，因为那行 panic 里的引号在任何 `sh -c`
/// 的引法下都活不下来。
#[test]
fn a_failing_assertion_rides_with_its_test_name() {
    let root = scratch("why");
    let log = root.join("check-default.log");
    std::fs::write(
        &log,
        "test result: FAILED. 0 passed; 1 failed; 0 ignored\n\
         test the_rendered_offsets_add_up ... FAILED\n\
         \n\
         failures:\n\
         \n\
         ---- the_rendered_offsets_add_up stdout ----\n\
         thread 'main' panicked at tests/offsets.rs:19:5:\n\
         assertion `left == right` failed\n\
         \x20 left: 24\n\
         \x20 right: 31\n\
         note: run with `RUST_BACKTRACE=1` for a backtrace\n",
    )
    .expect("the fixture log");
    let observed = observation(&log).expect("the log reads");
    let joined = observed.lines.join("\n");
    assert!(
        joined.contains("failed the_rendered_offsets_add_up"),
        "the name is still reported: {joined}"
    );
    assert!(
        joined.contains(
            "why    the_rendered_offsets_add_up: thread 'main' panicked at tests/offsets.rs:19:5: \
             assertion `left == right` failed"
        ),
        "and the assertion's own words ride with it, capped at two lines: {joined}"
    );
    assert!(
        !joined.contains("RUST_BACKTRACE") && !joined.contains("left: 24"),
        "the log's advice and the diff below the message are not part of it: {joined}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A run with nothing failing gets no `why` lines: the field is the assertion's, not a template.
/// 没有失败项的运行不产生 `why` 行：那一栏属于断言，不是模板。
#[test]
fn a_green_run_carries_no_assertion_words() {
    let root = scratch("why-green");
    let log = root.join("check-default.log");
    std::fs::write(&log, "test result: ok. 3 passed; 0 failed; 0 ignored\n").expect("log");
    let observed = observation(&log).expect("the log reads");
    assert!(
        !observed.lines.iter().any(|line| line.starts_with("why")),
        "{:?}",
        observed.lines
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The next hint carries a literal taken out of **this reply's own** `why` line, and what is
/// stripped from it is exactly the three parts that would match nothing.
/// 这条提示携带的字面量取自**这次回复自己的** `why` 行，而从它身上剥掉的正是那三种"匹配不到任何东西"的部分。
///
/// Measured (W8): `next` lines were followed 28% of the time, and the only one followed reliably
/// named a concrete thing to read. A placeholder (`<that phrase>`) leaves the filling-in to the
/// caller — which is the thinking the hint exists to remove.
/// 实测（W8）：`next` 行被采纳 28%，而唯一被稳定采纳的是点名了"具体要读什么"的那条。占位符
/// （`<that phrase>`）把填空留给调用方——而填空正是这条提示存在来省掉的思考。
#[test]
fn the_next_hint_carries_a_searchable_literal_from_this_reply() {
    // The harness prefix carries a process id; the generic clause is shared by many tests; the
    // numbers are values. None of the three is in any source file.
    // harness 前缀带进程号；通用从句被许多测试共用；数字是取值。三者都不在任何源码里。
    let lines = vec![
        "why    the_rendered_offsets_add_up: thread 'the_rendered_offsets_add_up' (100729) \
         panicked at tests/offsets.rs:18:5: assertion `left == right` failed: the rendered offsets \
         add up to 160, not 136"
            .to_owned(),
    ];
    let phrase = super::literal_to_search(&lines).expect("the message yields a phrase");
    assert_eq!(
        phrase, "the rendered offsets add up to",
        "the process id, the generic clause and the numbers are all gone"
    );
    let next = super::next_step(false, Some(101), &lines).expect("a failing run has a next step");
    assert!(
        next.contains("`search {literal: \"the rendered offsets add up to\"}`"),
        "and the hint carries it as a copyable call: {next}"
    );
    assert!(
        !next.contains("<that phrase>"),
        "no placeholder is left when the reply had the words: {next}"
    );
}

/// With nothing to take a phrase from, the hint falls back to the placeholder rather than inventing.
/// 无从取词时，提示落回占位符，而不是编一句。
#[test]
fn the_next_hint_falls_back_to_a_placeholder_rather_than_inventing_one() {
    let lines = vec!["result test result: FAILED. 1 passed; 1 failed".to_owned()];
    assert!(super::literal_to_search(&lines).is_none());
    let next = super::next_step(false, Some(101), &lines).expect("a next step");
    assert!(
        next.contains("<a short phrase from the assertion>"),
        "{next}"
    );
}

/// Every column head survives the sample, with its count.
/// 每一栏的栏头都活过抽样，连同它的计数。
///
/// Measured (round 8, g4): the default sample was "the first five lines", and on that tree the five
/// were all the numeric-constant column — so the answer never showed `branch-level:` at all, while
/// that level's whole question is the branch column. A column head carries its own count, which is
/// what tells a reader whether the part they did not get could matter.
/// 量到的（第八轮 g4）：默认抽样是"最前面五行"，而在那棵树上那五行全属数值常量栏 ⇒ 答案里**根本看不到**
/// `branch-level:`，而那一关的全部问题就是分支栏。栏头自带计数，正是它告诉读者"没拿到的部分会不会要紧"。
#[test]
fn every_column_head_survives_the_census_sample() {
    let census: Vec<String> = vec![
        "  constants: 1 named numeric constant(s)".to_owned(),
        "  respelled 1000 is written again at src/a.rs:3".to_owned(),
        "  respelled 1000 is written again at src/b.rs:4".to_owned(),
        "  unreferenced `LIMIT` is not read anywhere outside tests".to_owned(),
        "  entry plan: 0 `cut(` site(s)".to_owned(),
        "  declarations: 4 production `pub fn` name(s) appear in no test file".to_owned(),
        "  decl   no test names `a` (src/a.rs:1)".to_owned(),
        "  test-reachable: 1 of 15 production function(s) no test can reach".to_owned(),
        "  branch-level: 2 constructively unreachable arm(s) in this tree".to_owned(),
        "  `if false` guards an arm in `word` at src/a.rs:9".to_owned(),
        "  not covered by the branch-level column: a data-dependent condition".to_owned(),
        "  not covered: runtime behaviour and performance".to_owned(),
    ];
    let sample = super::census_sample(&census);
    for line in &census {
        let head = line
            .split_whitespace()
            .next()
            .is_some_and(|word| word.ends_with(':'));
        if head {
            assert!(
                sample.contains(line),
                "每一栏的头都在：{line}\n--- sample ---\n{}",
                sample.join("\n")
            );
        }
    }
    assert!(
        sample.iter().any(|line| line.contains("withheld")),
        "扣掉的行要说出来：{}",
        sample.join("\n")
    );
    assert!(
        sample.len() <= super::CENSUS_SAMPLE + census.len(),
        "样本仍然有界：{}",
        sample.len()
    );
}

/// The counter-proof, on the same shape: nothing withheld, no truncation line.
/// 反证（同一形状）：没有扣掉任何行时，不出现截断行。
#[test]
fn a_census_that_fits_says_nothing_about_withholding() {
    let census: Vec<String> = vec![
        "  branch-level: 1 constructively unreachable arm(s) in this tree".to_owned(),
        "  `if false` guards an arm in `word` at src/a.rs:9".to_owned(),
        "  not covered: runtime behaviour".to_owned(),
    ];
    let sample = super::census_sample(&census);
    assert_eq!(sample, census, "装得下就原样给出");
    assert!(
        !sample.iter().any(|line| line.contains("withheld")),
        "没有截断就不许提截断：{}",
        sample.join("\n")
    );
}
