//! Live-trace fixtures for the prototype-fixture tests.
//! 原型夹具测试使用的实时追踪夹具。
//!
//! The standalone Studio ships a demo trace (`sample_live_trace`) that belongs to
//! no host project, so a test asserting about *live* evidence has to build a
//! trace over the faces it loaded. Relying on the demo sample instead is what
//! made two of these tests unreachable: the sample records no locals and exactly
//! one unrelated call edge, so `graph_locals` always returned nothing and
//! `call_evidence` could never answer `Live` for the fixture's call.
//! 独立 Studio 自带一条不属于任何宿主项目的演示追踪（`sample_live_trace`），因此断言
//! **实测**证据的测试必须为自己加载的注册面构造追踪。依赖演示样本正是其中两条测试不可
//! 达的原因：样本不记录任何局部值，也只有一条无关的调用边，于是 `graph_locals` 永远返回
//! 空，`call_evidence` 也永远无法对夹具的那次调用回答 `Live`。

use super::CallRef;
use nichlink_debug_method::{CallTrace, LocalKind, SourceLocation};

/// A live trace over the fixture project: the caller is entered, two locals are
/// recorded inside its frame, and it calls the callee.
/// 一条针对夹具项目的实时追踪：进入调用方，在其帧内记录两个局部值，然后它调用被调方。
///
/// Two locals, not one: the DATA panel's cursor stops at `data_len - 1`, so a
/// single local could not show that the cursor moved. The function names are the
/// fixture's own, because `with_at` wants `&'static str`; the assertions that
/// follow are what keep this in step with the fixture.
/// 两个而不是一个局部值：DATA 面板的游标停在 `data_len - 1`，只有一个局部值就演示不出
/// 游标移动。函数名取自夹具本身，因为 `with_at` 需要 `&'static str`；紧随其后的断言
/// 保证这里与夹具保持一致。
pub(super) fn fixture_live_trace(caller: &CallRef, callee: &CallRef) -> CallTrace {
    debug_assert_eq!(caller.function, "preview_canvas_width");
    debug_assert_eq!(callee.function, "clamp_canvas_width");
    let caller_source = SourceLocation {
        file: "<fixture-trace>",
        line: 1,
        column: 1,
        function: "preview_canvas_width",
    };
    let callee_source = SourceLocation {
        file: "<fixture-trace>",
        line: 2,
        column: 1,
        function: "clamp_canvas_width",
    };
    let mut trace = CallTrace::full();
    trace.with_at(
        caller.node,
        "preview_canvas_width",
        caller_source,
        |trace| {
            trace.local_at(
                "canvas_name",
                "&str",
                "node_editor",
                LocalKind::Input,
                caller_source,
            );
            trace.local_at("width", "u32", "320", LocalKind::Binding, caller_source);
            trace.with_at(callee.node, "clamp_canvas_width", callee_source, |_| {});
        },
    );
    trace
}
