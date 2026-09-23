//! Host entry, host onboarding, and graft-plan macros.
//! 宿主入口、宿主接入与 graft 计划宏。

/// Declare the single host entry used by source-scope discovery.
/// 声明源码作用域发现使用的唯一宿主入口。
#[macro_export]
macro_rules! application {
    (entry = $entry:path $(,)?) => {
        #[doc(hidden)]
        pub const NICHLINK_APPLICATION_ENTRY: &str = stringify!($entry);
    };
}

/// Declare this crate as a NichLink host and pull in the registration plan
/// captured at build time.
/// 声明当前 crate 为 NichLink 宿主，并引入构建时捕获的注册计划。
///
/// `nichlink-build` renders the discovered registration tree to
/// `OUT_DIR/generated_lib.rs`; this macro includes it at the crate root so
/// `builtin_static_plan()` and the per-level `{name}_object!` aliases are
/// available crate-wide. It expands to
/// `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))` — writing that
/// line directly is an equivalent, advanced alternative.
/// `nichlink-build` 把发现的注册树渲染到 `OUT_DIR/generated_lib.rs`；
/// 此宏将其包含到 crate 根，使 `builtin_static_plan()` 与各层级的
/// `{name}_object!` 别名在整个 crate 内可用。它展开为
/// `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`，
/// 直接书写该行是等价的高级写法。
///
/// `include!` only accepts a literal path, so the generated entry's file name
/// cannot be read from the kernel lexicon here. Pinning the two texts together
/// makes a drift a compile error instead of a host that includes a file the
/// build step no longer writes.
/// `include!` 只接受字面量路径，因此这里无法从内核 lexicon 读取生成入口的文件名。
/// 把两份文本钉在一起，漂移会变成编译错误，而不是去 include 一个构建步骤已不再写的
/// 文件。
const _: () = assert!(::nichlink::lexicon::same_text(
    ::nichlink::lexicon::GENERATED_LIB_FILE,
    "generated_lib.rs"
));

/// Pull the build-time registration plan into the crate root.
/// 把构建期捕获的注册计划引入 crate 根。
///
/// Call it once per host crate whose build script has run the NichLink build
/// step; the included file supplies `builtin_static_plan()` and the per-level
/// object aliases. The `include!` target lives under `OUT_DIR`, so a crate that
/// never runs that step fails to compile rather than silently missing faces.
/// 在已由 build script 运行 NichLink 构建步骤的宿主 crate 中调用一次；被包含的文件
/// 提供 `builtin_static_plan()` 与各层级 object 别名。`include!` 目标位于 `OUT_DIR`，
/// 未运行该步骤的 crate 会编译失败，而不是静默缺少注册面。
#[macro_export]
macro_rules! host {
    () => {
        include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"));
    };
}

/// Declare graft selectors for build-time capture without constructing a
/// runtime `GraftPlan`.
/// 声明供构建阶段捕获的 graft selector，不构造运行时 `GraftPlan`。
///
/// Put this at the host crate entry. `nichlink-build` validates the grammar and
/// stores the cuts in the generated `StaticPlan`. Use the dynamic
/// [`graft_plan!`](crate::graft_plan) expression only when code needs to build
/// or edit a plan at runtime.
/// 将它放在宿主 crate 入口。`nichlink-build` 校验语法并把切口写入生成的
/// `StaticPlan`；只有运行时代码确实要构造或编辑计划时才使用动态
/// [`graft_plan!`](crate::graft_plan) 表达式。
#[macro_export]
macro_rules! static_graft_plan {
    ($framework:expr, $($cuts:tt)+) => {
        const _: $crate::FrameworkId = $framework;
        const _: &str = stringify!($($cuts)+);
    };
}

/// Build a persistent external graft overlay without touching source files.
/// 构造持久化外部 graft 覆盖计划，不移动或修改任何源码文件。
///
/// ```
/// # use nichlink_run_method::{FrameworkId, graft_plan};
/// # let framework = FrameworkId::new("example");
/// let plan = graft_plan!(framework,
///     cut ["root/a1/b2"] graft "canvas_fast",
///     cut ["root/a"] full graft "a_fast",
/// );
/// assert_eq!(plan.cuts.len(), 2);
/// ```
#[macro_export]
macro_rules! graft_plan {
    ($framework:expr, $($cuts:tt)+) => {{
        let mut plan = $crate::GraftPlan::new($framework);
        $crate::__graft_plan_cuts!(plan; $($cuts)+);
        plan
    }};
}

#[doc(hidden)]
#[macro_export]
macro_rules! __graft_plan_cuts {
    ($plan:ident;) => {};
    ($plan:ident; cut [$start:literal to $end:literal] full graft $graft:literal $(, $($rest:tt)*)?) => {{
        let mut cut = $crate::GraftCut::range($start, $end, $graft);
        cut.subtree = true;
        $plan.cuts.push(cut);
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut [$start:literal to $end:literal] graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts
            .push($crate::GraftCut::range($start, $end, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut [$path:literal] full graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::GraftCut::subtree($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut [$path:literal] graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::GraftCut::new($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut $path:literal full graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::GraftCut::subtree($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
    ($plan:ident; cut $path:literal graft $graft:literal $(, $($rest:tt)*)?) => {{
        $plan.cuts.push($crate::GraftCut::new($path, $graft));
        $crate::__graft_plan_cuts!($plan; $($($rest)*)?);
    }};
}
