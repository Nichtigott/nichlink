//! The freshness level of a member's published records, and what this answer paid to
//! know it.
//! 成员已发布记录的新鲜度等级，以及这份答案为知道它付出了什么。
//!
//! The verdict itself is unchanged and stays where it was: `build_output_is_current`
//! hashes the sources, and nothing here substitutes a timestamp for that hash — this
//! repository has been burned by `cp -a`/`rsync -a` keeping an mtime while the content
//! changed.
//! 判据本身没有改，也仍在原处：`build_output_is_current` 对源码取哈希，这里没有任何东西拿时间戳
//! 代替那个哈希——本仓踩过 `cp -a`/`rsync -a` 保留 mtime 而内容已变的坑。
//!
//! What this module adds is the *level* a report prints, because "the same answer every
//! time" and "the same answer without paying for the hash again" are different claims:
//! 本模块加的是报告打印的**等级**，因为"每次都给出同一个答案"与"不再付一次哈希的钱也给出同一个
//! 答案"是两个不同的声称：
//!
//! - `freshness: content-verified at HH:MM:SS` — the content hash ran for this answer.
//! - `freshness: reused (content-verified at HH:MM:SS, Ns ago; window 30s)` — this answer
//!   read a verification taken inside the declared window. It never reads as `current`:
//!   复用与"当前"是两个词，把它们混同正是这套等级要阻止的事。
//! - `build stale (run `nichlink check`)` — the record no longer describes these sources,
//!   or carries no fingerprint at all.
//! - `freshness: not checked (its records were not used for this answer)` — the member
//!   contributed nothing to this answer, so nothing was paid for it.
//! - `unknown (...)` — a caller asked for reuse only, and the window held nothing.
//!
//! The window is [`REUSE_WINDOW_SECONDS`], a constant this module documents and its tests
//! assert. A caller overrides the policy per request with the `freshness` argument
//! (`reuse` is the default, `verify` pays immediately, `reuse-only` never pays), and every
//! tool whose answer can carry a member census advertises it.
//! 窗口是本模块文档化、其测试也断言的常数 [`REUSE_WINDOW_SECONDS`]。调用方按请求用 `freshness`
//! 参数覆盖策略（默认 `reuse`，`verify` 立即付费，`reuse-only` 永不自付），而每个答案可能带成员
//! 普查的工具都会声明它。
//!
//! The rule the levels exist to protect is **no claim without a check**: a member whose
//! records this answer used is verified, or its verification is reused *and labelled as
//! reused*; a member the answer did not use says so instead of borrowing the word. The
//! single-package build evidence (`build_evidence.rs`) answers a different question — the
//! package's own output — and keeps its own `current`/`stale` spelling.
//! 这些等级要守的规则是**不校验就不声称**：据以作答的成员会被核验，或复用上一次核验并**标明是
//! 复用**；没有被这份答案用到的成员如实说出来，而不是借用那个词。单包的构建证据
//! （`build_evidence.rs`）回答的是另一个问题——这个包自己的产物——并保留它自己的
//! `current`/`stale` 拼法。
//!
//! The verification is remembered per **process, per thread**, which is what makes a
//! long-lived bridge cheap: the second call about the same member inside the window buys
//! nothing new. It is not a hidden memoization — the report says `reused` and names the
//! moment — and it is not process-wide, so two tests running side by side do not read each
//! other's verdicts.
//! 核验按**进程内、线程内**记着，这正是长驻桥便宜的原因：窗口内关于同一个成员的下一次调用不再
//! 买新东西。它不是隐藏的记忆化——报告写着 `reused` 并点名时刻——也不是进程级的，因此并行跑的两条
//! 测试不会读到彼此的裁决。

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::build_time::build_output_is_current;

/// How long, in seconds, a content verification may be reused without paying again.
/// 一次内容核验可以不付费复用的时长（秒）。
///
/// The window is a declared constant rather than a tuning detail: the answer prints it
/// (`window 30s`), this module's doc names it, and its tests assert it, so a caller can
/// read what "reused" means without reading this file. It is short on purpose — the point
/// is to make a burst of calls on one member cheap, not to remember a tree for a session.
/// 窗口是声明出来的常数而不是调参细节：答案里印着它（`window 30s`），本模块文档点名它，其测试
/// 断言它，因此调用方不用读这个文件就知道"复用"意味着什么。它有意很短——目的是让针对同一个成员的
/// 一串调用便宜，而不是把一棵树记一整个会话。
pub(crate) const REUSE_WINDOW_SECONDS: u64 = 30;

/// What a census row prints for a member this answer did not use.
/// 这份答案没有用到的成员，其普查行打印的内容。
pub(crate) const NOT_CHECKED: &str =
    "freshness: not checked (its records were not used for this answer)";

/// What a `reuse-only` call prints when the window holds no verification.
/// `reuse-only` 调用在窗口内没有核验时打印的内容。
pub(crate) const UNKNOWN: &str = "unknown (no content verification inside the reuse window, and \
                                  this call asked not to pay for one)";

/// The word every stale verdict is spelled with, in one place.
/// 每个"过期"裁决共用的唯一词形。
const STALE: &str = "build stale (run `nichlink check`)";

/// How much of the freshness question one call is willing to pay for.
/// 一次调用愿意为新鲜度问题付多少。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Policy {
    /// Reuse a verification from inside [`REUSE_WINDOW_SECONDS`], otherwise pay.
    /// 复用 [`REUSE_WINDOW_SECONDS`] 之内的核验，否则付费。
    Reuse,
    /// Pay for a content verification now, whatever the window holds.
    /// 现在就付一次内容核验，无论窗口里有什么。
    Verify,
    /// Never pay; print [`UNKNOWN`] when the window holds nothing.
    /// 永不自付；窗口里没有东西时打印 [`UNKNOWN`]。
    ReuseOnly,
}

/// The policy a request asks for, by the `freshness` argument it carries.
/// 请求按它携带的 `freshness` 参数要求的策略。
///
/// An unknown value reads as the default rather than failing the call: the argument
/// narrows a cost, and a typo in it must not take a query down with it. The default is the
/// one that cannot mislead — reuse is always labelled as reuse.
/// 无法识别的取值按默认处理而不是让调用失败：这个参数只是收窄一项开销，其中的错别字不该把一次查询
/// 一起带走。默认值是那个不会误导人的——复用永远被标明为复用。
pub(crate) fn policy_from(arguments: &Value) -> Policy {
    match arguments.get("freshness").and_then(Value::as_str) {
        Some("verify") => Policy::Verify,
        Some("reuse-only") => Policy::ReuseOnly,
        _ => Policy::Reuse,
    }
}

thread_local! {
    /// The policy this thread's current tool call asked for.
    /// 本线程当前工具调用所要求的策略。
    static POLICY: Cell<Policy> = const { Cell::new(Policy::Reuse) };
    /// The verifications this thread has already paid for, keyed by `(root, out)`.
    /// 本线程已经付过费的核验，按 `(root, out)` 索引。
    static VERIFIED: RefCell<HashMap<(PathBuf, PathBuf), Verified>> =
        RefCell::new(HashMap::new());
}

/// One content verification, with when it happened and what it said.
/// 一次内容核验，连同它发生的时刻与它给出的结论。
struct Verified {
    /// The monotonic moment the hash ran, which is what the window is measured against.
    /// 哈希运行的单调时刻，窗口就是相对它度量的。
    at: Instant,
    /// That moment as `HH:MM:SS` (UTC), which is what the report prints.
    /// 那个时刻的 `HH:MM:SS`（UTC），也就是报告打印的。
    clock: String,
    /// Whether the output still described the sources at that moment.
    /// 那一刻产物是否仍在描述这批源码。
    current: bool,
}

/// Set the policy for the calls that follow on this thread.
/// 为本线程随后的调用设置策略。
pub(crate) fn set_policy(policy: Policy) {
    POLICY.with(|cell| cell.set(policy));
}

/// The policy this thread is under.
/// 本线程当前所处的策略。
fn policy() -> Policy {
    POLICY.with(Cell::get)
}

/// The freshness line for one published output directory.
/// 一个已发布产物目录的新鲜度行。
pub(crate) fn line(root: &Path, out: &Path) -> String {
    let key = (root.to_path_buf(), out.to_path_buf());
    match policy() {
        Policy::Verify => verify_now(&key, root, out),
        Policy::Reuse => match remembered(&key) {
            Some(verified) => reused_line(&verified),
            None => verify_now(&key, root, out),
        },
        Policy::ReuseOnly => match remembered(&key) {
            Some(verified) => reused_line(&verified),
            None => UNKNOWN.to_owned(),
        },
    }
}

/// The verification this thread took inside the window, if it took one.
/// 本线程在窗口之内做过的核验（如果有）。
///
/// An expired entry is dropped rather than answered: the window is the whole permission to
/// reuse, and holding the entry any longer would only invite a second reader to forget
/// that.
/// 过期的条目被丢弃而不是被拿来作答：窗口就是复用的全部许可，再留着它只会招来第二个读取方忘记这
/// 一点。
fn remembered(key: &(PathBuf, PathBuf)) -> Option<Verified> {
    let window = Duration::from_secs(REUSE_WINDOW_SECONDS);
    VERIFIED.with(|store| {
        let mut store = store.borrow_mut();
        match store.get(key) {
            Some(verified) if verified.at.elapsed() < window => Some(Verified {
                at: verified.at,
                clock: verified.clock.clone(),
                current: verified.current,
            }),
            Some(_) => {
                store.remove(key);
                None
            }
            None => None,
        }
    })
}

/// Pay for the content hash and remember it.
/// 为内容哈希付费并记住它。
fn verify_now(key: &(PathBuf, PathBuf), root: &Path, out: &Path) -> String {
    let verified = Verified {
        at: Instant::now(),
        clock: wall_clock(),
        current: build_output_is_current(root, out),
    };
    let line = if verified.current {
        format!("freshness: content-verified at {}", verified.clock)
    } else {
        STALE.to_owned()
    };
    VERIFIED.with(|store| store.borrow_mut().insert(key.clone(), verified));
    line
}

/// The line for a verification the window let this call reuse.
/// 窗口允许本次调用复用的那次核验，其对应的行。
fn reused_line(verified: &Verified) -> String {
    let age = verified.at.elapsed().as_secs();
    if verified.current {
        format!(
            "freshness: reused (content-verified at {}, {age}s ago; window {REUSE_WINDOW_SECONDS}s)",
            verified.clock
        )
    } else {
        format!(
            "{STALE}; reused, content-verified at {}, {age}s ago; window {REUSE_WINDOW_SECONDS}s",
            verified.clock
        )
    }
}

/// The current wall-clock time as `HH:MM:SS` in UTC.
/// 当前挂钟时间的 UTC `HH:MM:SS`。
///
/// UTC rather than local time because the bridge reads no time zone and a report whose
/// clock silently followed the environment would be a different token on two machines.
/// 用 UTC 而不是本地时间，因为本桥不读时区，而一个悄悄跟着环境走的时刻会让两台机器上的报告印出
/// 不同的凭据。
pub(crate) fn wall_clock() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);
    let (hours, minutes, seconds) = ((seconds / 3600) % 24, (seconds / 60) % 60, seconds % 60);
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
#[path = "freshness_tests.rs"]
mod freshness_tests;
