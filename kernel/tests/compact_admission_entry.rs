//! The compact admission spelling is the kernel's public entry point.
//! 紧凑 admission 拼法是内核公开的入口。
//!
//! Studio needed this rendering and, because the kernel kept it private, wrote a
//! second copy of the same rules (`FIXR-01`: `studio/src/studio/app/source_index.rs`
//! rendered the compact form itself). This file is the outside-of-the-crate pin:
//! it reaches the renderer through `xirang_kernel::authoring::parse`, which is exactly
//! the path a surface has, so a private renderer cannot satisfy it — the pin stops
//! compiling with `E0603` instead.
//! Studio 需要这份渲染，而内核把它设为私有，于是 Studio 写出了同一套规则的第二份副本
//! （`FIXR-01`：`studio/src/studio/app/source_index.rs` 自己渲染紧凑形式）。本文件是
//! crate 之外的钉子：它经 `xirang_kernel::authoring::parse` 取渲染器，这正是执行面拥有的那条路径，
//! 因此私有渲染器无法满足它——钉子会以 `E0603` 编译失败，而不是悄悄通过。
//!
//! The second half is the proof that the merge changed nothing on the wire: the
//! historical single-list branches (`Admission::allow_paths(…)` /
//! `Admission::deny_paths(…)`) now route through the public renderer, and the bytes
//! they produce must be the bytes consumers already read.
//! 另一半是"并入后线上字节未变"的证明：历史单列表分支（`Admission::allow_paths(…)` /
//! `Admission::deny_paths(…)`）现在走公开渲染器，而它们产出的字节必须正是消费方已在读的字节。

// The renderer lives behind the parser's feature; without `syntax` there is
// nothing here to call. A whole-workspace build turns it on through other members.
// 渲染器在解析器的特性之后；没有 `syntax` 时这里无物可调。整工作区构建会经其他成员打开它。
#![cfg(feature = "syntax")]

use xirang_kernel::authoring::parse::{
    compact_admission, parse_admission_expression, parse_admission_owned,
};
use xirang_kernel::declaration::OwnedAdmission;

/// One policy in the owned form the renderer takes, from path spellings.
/// 一份策略的拥有型形式，由路径拼法构造——渲染器取用的就是它。
fn admission(allowed: &[&str], denied: &[&str]) -> OwnedAdmission {
    let owned = |paths: &[&str]| paths.iter().map(|path| (*path).to_owned()).collect();
    OwnedAdmission {
        allowed_paths: owned(allowed),
        denied_paths: owned(denied),
    }
}

/// The historical spellings keep their exact bytes after the merge.
/// 并入之后，历史拼法的字节逐字不变。
///
/// Each pair below is the expectation this file asserted *before* the single-list
/// branches were routed through the public renderer, so the same table is the
/// before/after对照: `allow_paths(`/`deny_paths(` and the constructor form both
/// render one clause, and `ANY` keeps its spelling.
/// 下面每一对都是本文件在单列表分支改走公开渲染器**之前**就已断言过的期望，因此同一张表就是
/// 前后对照：`allow_paths(`/`deny_paths(` 与构造形式都渲染一个子句，`ANY` 保持自己的拼法。
#[test]
fn historical_single_list_spellings_render_unchanged_bytes() {
    for (expression, expected) in [
        ("crate::Admission::ANY", "ANY"),
        ("crate::Admission::new(&[], &[])", "ANY"),
        (r#"crate::Admission::new(&["a", "b"], &[])"#, "allow:a,b"),
        (r#"crate::Admission::allow_paths(&["a", "b"])"#, "allow:a,b"),
        (r#"crate::Admission::new(&[], &["c"])"#, "deny:c"),
        (r#"crate::Admission::deny_paths(&["c"])"#, "deny:c"),
        (
            r#"crate::Admission::new(&["ui"], &["ui/experimental"])"#,
            "allow:ui;deny:ui/experimental",
        ),
    ] {
        assert_eq!(
            parse_admission_expression(expression).expect("the spelling is readable"),
            expected,
            "the bytes of `{expression}` moved"
        );
    }
}

/// The compact renderer is reachable from outside the crate, and its output is
/// the spelling this crate's own parser reads back.
/// 紧凑渲染器可从 crate 之外调用，且它的输出正是本 crate 自己的解析器读得回的拼法。
///
/// This is the pin `FIXR-01` needed: with the renderer private the call below does
/// not compile, which is what pushed Studio into writing a second copy.
/// 这正是 `FIXR-01` 需要的钉子：渲染器私有时，下面这次调用编译不过——而后者正是把 Studio
/// 逼出第二份副本的原因。
#[test]
fn the_compact_renderer_is_reachable_from_outside_the_crate() {
    assert_eq!(compact_admission(&admission(&[], &[])), "ANY");
    assert_eq!(compact_admission(&admission(&["ui"], &[])), "allow:ui");
    assert_eq!(compact_admission(&admission(&[], &["c"])), "deny:c");
    assert_eq!(
        compact_admission(&admission(&["ui", "controls"], &["ui/experimental"])),
        "allow:ui,controls;deny:ui/experimental"
    );

    // One grammar, two directions: every value the renderer emits must read back
    // as the policy it was given, or a surface following it writes a gate the
    // kernel then refuses.
    // 一套语法、两个方向：渲染器吐出的每个值都必须读回它被给的那份策略，否则照着它写的执行面
    // 会写出内核随后拒绝的门禁。
    for policy in [
        admission(&[], &[]),
        admission(&["ui"], &[]),
        admission(&[], &["c"]),
        admission(&["ui", "controls"], &["ui/experimental"]),
    ] {
        let text = compact_admission(&policy);
        assert_eq!(
            parse_admission_owned(&text).expect("the kernel reads the spelling it emits"),
            policy,
            "{text}"
        );
    }

    // The merged historical branch renders through that very function, byte for
    // byte: `allow_paths(&[…])` and a policy with no deny list are one spelling.
    // 被并入的历史分支正是走这个函数渲染的，逐字节一致：`allow_paths(&[…])` 与一份没有 deny
    // 列表的策略是同一种拼法。
    assert_eq!(
        parse_admission_expression(r#"crate::Admission::allow_paths(&["a", "b"])"#)
            .expect("the historical spelling is readable"),
        compact_admission(&admission(&["a", "b"], &[]))
    );
}
