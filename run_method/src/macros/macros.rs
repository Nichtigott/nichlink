//! Registration-face macros.
//! 注册面宏。

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
/// `builtin_static_plan()` is available crate-wide. It expands to
/// `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))` — writing that
/// line directly is an equivalent, advanced alternative.
/// `nichlink-build` 把发现的注册树渲染到 `OUT_DIR/generated_lib.rs`；
/// 此宏将其包含到 crate 根，使 `builtin_static_plan()` 在整个 crate 内
/// 可用。它展开为 `include!(concat!(env!("OUT_DIR"), "/generated_lib.rs"))`，
/// 直接书写该行是等价的高级写法。
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

#[macro_export]
macro_rules! trace_call {
    ($trace:expr, $node:expr, $function:literal, $operation:expr) => {{ $trace.with($node, $function, $operation) }};
}

/// Record a fallible invocation according to the trace policy.
/// 按追踪策略记录一个可失败的函数调用。
///
/// In `errors-only` mode, successful scopes are discarded and `Err`/panic
/// scopes remain available for diagnostics. Other modes keep their normal
/// behavior.
/// 在 `errors-only` 模式下成功作用域会回滚，`Err` 或 panic 作用域会保留供诊断；
/// 其它模式保持正常收集行为。
#[macro_export]
macro_rules! trace_call_result {
    ($trace:expr, $operation:expr) => {{ $trace.with_result($operation) }};
}

/// Capture a function input or local value at the macro call site.
/// 在宏调用位置记录函数输入或局部值。
#[macro_export]
macro_rules! trace_value {
    ($trace:expr, $name:literal, $type_name:literal, $value:expr, $kind:expr) => {{ $trace.local($name, $type_name, $value, $kind) }};
}

/// Capture a transformed value and connect it to its producer.
/// 记录转换后的值，并把它连接到生产者。
#[macro_export]
macro_rules! trace_transform {
    ($trace:expr, $input:expr, $name:literal, $type_name:literal, $value:expr) => {{ $trace.transform($input, $name, $type_name, $value) }};
}

/// Capture a value consumed by another function parameter.
/// 记录值被另一个函数参数消费的位置。
#[macro_export]
macro_rules! trace_consume {
    ($trace:expr, $input:expr, $function:literal, $parameter:literal) => {{ $trace.consume($input, $function, $parameter) }};
}

/// Capture a function return value and connect it to its input.
/// 记录函数返回值，并把它连接到输入值。
#[macro_export]
macro_rules! trace_return {
    ($trace:expr, $input:expr, $name:literal, $type_name:literal, $value:expr) => {{ $trace.return_value($input, $name, $type_name, $value) }};
}
