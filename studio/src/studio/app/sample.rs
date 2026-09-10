//! Built-in live trace sample for the standalone Studio demo.
//! 独立 Studio 演示使用的内置实时追踪样本。

use nichlink_debug_method::CallTrace;
use nichlink_run_method::{NodeId, SourceLocation};

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
