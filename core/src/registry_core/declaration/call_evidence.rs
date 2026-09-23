//! Observed call edges, evidence, and the trace mode that records them.
//! 已观测的调用边、证据，以及记录它们的追踪模式。

use super::*;

/// One call edge that was observed while a `CallTrace` frame was active.
/// `CallTrace` 中实际观察到的一条调用边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallSite {
    /// Registry node the active frame belongs to.
    /// 当前活动帧所属的注册机节点。
    pub node: NodeId,
    /// Logical function name active in that frame.
    /// 该帧中处于活动状态的逻辑函数名。
    pub function: &'static str,
    /// Identifier of the trace frame that observed this call, unique per invocation.
    /// 观察到本次调用的追踪帧编号，每次调用实例都不同。
    pub frame_id: u64,
    /// Stable declaration/callsite location. `None` is reserved for callers
    /// that construct a synthetic frame directly.
    /// 稳定的声明/调用位置；`None` 仅保留给直接构造合成帧的调用者。
    pub source: Option<SourceLocation>,
}

// ---------------------------------------------------------------------------
// Observed call edges and evidence. Shared by run_method tracing and
// debug_method tooling; defined here because they are pure protocol nouns.
// 已观测调用边与证据。由 run_method 追踪与 debug_method 工具共用；
// 作为纯协议名词定义在 kernel。

/// One call edge that was observed while a `CallTrace` frame was active.
/// `CallTrace` 中实际观察到的一条调用边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallEdge {
    /// Site that observed the call while its frame was active.
    /// 调用发生、其帧处于活动状态的调用点。
    pub caller: CallSite,
    /// Site recorded as the target of that call.
    /// 被记录为该调用目标的调用点。
    pub callee: CallSite,
}

/// Provenance of a relationship across runtime, MIR, and source evidence.
/// 运行时、MIR 与源码证据共用的关系来源。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    /// Confirmed by a live `CallTrace` observation.
    Live,
    /// Candidate found by source analysis without runtime confirmation.
    Source,
    /// Candidate inferred from MIR; it may not have executed.
    Mir,
    /// Supplied by an external adapter without a local trace.
    External,
    /// A relation whose producer is not known.
    Unknown,
}

impl EvidenceKind {
    /// Compact marker used by text, DOT, and Studio renderers.
    /// 文本、DOT 与 Studio 渲染器共用的紧凑标记。
    pub const fn marker(self) -> &'static str {
        match self {
            Self::Live => "+",
            Self::Mir => "?",
            Self::Source => "~",
            Self::External => "x",
            Self::Unknown => "!",
        }
    }

    /// Stable machine and human readable label.
    /// 稳定的机器和人类可读标签。
    pub const fn label(self) -> &'static str {
        match self {
            Self::Live => "live",
            Self::Mir => "mir",
            Self::Source => "source",
            Self::External => "external",
            Self::Unknown => "unknown",
        }
    }

    /// Only a live observation confirms that an edge executed.
    /// 只有 Live 观察能确认边实际执行过。
    pub const fn confirmed(self) -> bool {
        matches!(self, Self::Live)
    }
}

/// A call edge with invocation IDs removed for topology queries.
/// 去掉调用实例编号、用于拓扑查询的逻辑调用边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalCallEdge {
    /// Calling side with invocation ids dropped, so repeated calls share one edge.
    /// 去掉调用实例编号后的调用方，因此重复调用共享同一条边。
    pub caller: CallSite,
    /// Called side with invocation ids dropped, so repeated calls share one edge.
    /// 去掉调用实例编号后的被调用方，因此重复调用共享同一条边。
    pub callee: CallSite,
    /// Provenance that justifies treating this edge as real.
    /// 支撑该边成立的证据来源。
    pub evidence: EvidenceKind,
}

// ---------------------------------------------------------------------------
// Plugin manifests and flow contracts.
// These types are declared here because `RegistrationInfo`/`RegistrationSnapshot`
// carry them; the runtime plugin modules re-export them at their historical
// paths. 以下类型因被注册声明持有而定义在 kernel；runtime 插件模块在原路径重导出。

/// Runtime collection policy.
/// 运行时追踪收集策略。
///
/// `Off` is the release-safe default for runtime traces. `ErrorsOnly`
/// keeps evidence only for a failed result/panic scope, while `Full` retains
/// every observed frame, local, and data edge.
/// `Off` 是运行时追踪的发布安全默认值；`ErrorsOnly` 只保留
/// 失败结果或 panic 作用域中的证据；`Full` 保留全部帧、局部值和数据边。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TraceMode {
    /// Do not allocate or retain runtime evidence.
    /// 不分配也不保留运行时证据。
    #[default]
    Off,
    /// Retain evidence only when an error scope fails.
    /// 只有错误作用域失败时才保留证据。
    ErrorsOnly,
    /// Retain all observed runtime evidence.
    /// 保留全部观察到的运行时证据。
    Full,
}

impl TraceMode {
    /// Parses the value accepted by `NICH_LINK_TRACE`.
    /// 解析 `NICH_LINK_TRACE` 支持的值。
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" | "0" | "disabled" => Some(Self::Off),
            "errors-only" | "errors_only" | "errors" => Some(Self::ErrorsOnly),
            "full" | "all" => Some(Self::Full),
            _ => None,
        }
    }
}

#[cfg(test)]
mod trace_mode_tests {
    use super::TraceMode;

    #[test]
    fn parse_accepts_documented_spellings() {
        assert_eq!(TraceMode::parse("off"), Some(TraceMode::Off));
        assert_eq!(TraceMode::parse("ERRORS-ONLY"), Some(TraceMode::ErrorsOnly));
        assert_eq!(TraceMode::parse(" full "), Some(TraceMode::Full));
        assert_eq!(TraceMode::parse("sometimes"), None);
    }
}
