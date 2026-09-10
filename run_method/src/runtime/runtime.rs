// Observed call edges and evidence shared by tracing and tooling.
// The types are protocol nouns defined in the kernel; this module keeps the
// historical re-export path for the run_method surface.
// 追踪与工具共用的已观测调用边与证据。
// 类型本体是定义在 kernel 的协议名词；本模块保留 run_method 面的历史重导出路径。

pub use self::trace::{
    CallSite, CallTrace, DataEdge, DataHop, FramePath, LocalId, LocalKind, LocalValue, TraceMode,
    Observation,
};
pub use nichlink::{CallEdge, EvidenceKind, LogicalCallEdge};
