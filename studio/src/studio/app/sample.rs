//! Built-in live trace sample for the standalone Studio demo.
//! 独立 Studio 演示使用的内置实时追踪样本。

use nichlink_debug_method::CallTrace;
use nichlink_run_method::{LocalKind, NodeId, SourceLocation};

pub(super) fn sample_live_trace() -> CallTrace {
    let mut trace = CallTrace::full();
    let source = SourceLocation {
        file: "<studio-sample>",
        line: 1,
        column: 1,
        function: "studio_sample",
    };
    trace.with_at(
        NodeId::from_path("<studio-sample>", "StudioSample"),
        "studio_sample",
        source,
        |trace| {
            // One observed local, so the DATA panel demonstrates what it is for.
            // Without it the panel rendered "no live locals captured" forever, and
            // its "built-in sample" title named a sample that could never show a
            // value.
            // 一个被观测到的局部值，使 DATA 面板演示出它存在的意义。没有它，面板永远渲染
            // "no live locals captured"，而它那个 "built-in sample" 标题指的是一个永远显示
            // 不出任何数值的样本。
            trace.local("requested_width", "u32", "1280", LocalKind::Binding);
            trace.with_at(
                NodeId::from_path("<studio-sample>", "StudioSampleResult"),
                "studio_sample_result",
                source,
                |_| {},
            );
        },
    );
    trace
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The sample records one observed local, so the DATA panel can show a value
    /// instead of the placeholder it rendered forever. Without it the panel's
    /// "built-in sample" title named a sample that could never display anything.
    /// 样本记录一个被观测到的局部值，因此 DATA 面板能显示数值，而不是它一直渲染的占位。
    /// 没有它，面板那个 "built-in sample" 标题指的是一个永远显示不出任何东西的样本。
    #[test]
    fn the_built_in_sample_carries_one_observed_local() {
        let trace = sample_live_trace();
        assert_eq!(trace.locals().len(), 1, "{:?}", trace.locals());
        let local = &trace.locals()[0];
        assert_eq!(local.name, "requested_width");
        assert_eq!(local.value, "1280");
    }
}
