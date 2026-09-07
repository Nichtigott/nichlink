// Runtime call frames and observed data edges.
// 运行时调用帧与已观测的数据边。

use std::collections::BTreeMap;
use crate::registry_core::declaration::SourceLocation;
use crate::registry_core::identity::NodeId;
use super::*;

pub use self::edges::{DataEdge, DataHop};
pub use self::locals::{LocalId, LocalKind, LocalValue, Observation};

/// Runtime collection policy.
/// 运行时追踪收集策略。
///
/// `Off` is the release-safe default for [`CallTrace::runtime`]. `ErrorsOnly`
/// keeps evidence only for a failed result/panic scope, while `Full` retains
/// every observed frame, local, and data edge.
/// `Off` 是 [`CallTrace::runtime`] 的发布安全默认值；`ErrorsOnly` 只保留
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
    /// The policy used by an application runtime when no debug mode is chosen.
    /// 未显式选择调试模式时，应用运行时采用的策略。
    pub const fn application_default() -> Self {
        if cfg!(debug_assertions) {
            Self::ErrorsOnly
        } else {
            Self::Off
        }
    }

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

    /// Reads the optional process-level override once when a trace is created.
    /// 创建追踪器时读取一次可选的进程级覆盖配置。
    pub fn from_env() -> Self {
        std::env::var("NICH_LINK_TRACE")
            .ok()
            .and_then(|value| Self::parse(&value))
            .unwrap_or_else(Self::application_default)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallSite {
    pub node: NodeId,
    pub function: &'static str,
    pub frame_id: u64,
    /// Stable declaration/callsite location. `None` is reserved for callers
    /// that construct a synthetic frame directly.
    /// 稳定的声明/调用位置；`None` 仅保留给直接构造合成帧的调用者。
    pub source: Option<SourceLocation>,
}

#[derive(Clone, Debug)]
pub struct CallTrace {
    pub(super) mode: TraceMode,
    pub(super) frames: Vec<FrameRecord>,
    pub(super) frame_index: BTreeMap<u64, usize>,
    pub(super) current: Vec<u64>,
    pub(super) locals: Vec<LocalValue>,
    pub(super) local_index: BTreeMap<u64, usize>,
    pub(super) local_name_index: BTreeMap<String, Vec<u64>>,
    pub(super) local_function_index: BTreeMap<&'static str, Vec<u64>>,
    pub(super) edges: Vec<DataEdge>,
    pub(super) outgoing_index: BTreeMap<u64, Vec<usize>>,
    pub(super) incoming_index: BTreeMap<u64, Vec<usize>>,
    pub(super) next_local_id: u64,
    pub(super) next_frame_id: u64,
    pub(super) error_scope_depth: usize,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct TraceMark {
    frames: usize,
    locals: usize,
    edges: usize,
    next_local_id: u64,
    next_frame_id: u64,
}

impl Default for CallTrace {
    fn default() -> Self { Self::runtime() }
}

#[derive(Clone, Debug)]
pub(super) struct FrameRecord { pub(super) call: CallSite, pub(super) parent: Option<u64> }

#[derive(Clone, Copy, Debug)]
pub struct FrameView<'a> { pub call: &'a CallSite, pub parent: Option<u64> }

impl CallTrace {
    /// Creates a collector using the application default policy.
    /// 使用应用默认策略创建收集器。
    ///
    /// Debug builds default to `ErrorsOnly`; release builds default to `Off`.
    /// 调试构建默认使用 `ErrorsOnly`，发布构建默认使用 `Off`。
    pub fn new() -> Self { Self::runtime() }

    /// Creates a collector using the application default policy.
    /// 使用应用默认策略创建收集器。
    pub fn runtime() -> Self { Self::with_mode(TraceMode::from_env()) }

    /// Creates a disabled collector with no-op recording methods.
    /// 创建关闭收集器，所有记录方法都是 no-op。
    pub fn disabled() -> Self { Self::with_mode(TraceMode::Off) }

    /// Creates an errors-only collector.
    /// 创建只保留失败证据的收集器。
    pub fn errors_only() -> Self { Self::with_mode(TraceMode::ErrorsOnly) }

    /// Creates a full collector.
    /// 创建完整收集器。
    pub fn full() -> Self { Self::with_mode(TraceMode::Full) }

    pub fn with_mode(mode: TraceMode) -> Self {
        Self {
            mode,
            frames: Vec::new(),
            frame_index: BTreeMap::new(),
            current: Vec::new(),
            locals: Vec::new(),
            local_index: BTreeMap::new(),
            local_name_index: BTreeMap::new(),
            local_function_index: BTreeMap::new(),
            edges: Vec::new(),
            outgoing_index: BTreeMap::new(),
            incoming_index: BTreeMap::new(),
            next_local_id: 0,
            next_frame_id: 0,
            error_scope_depth: 0,
        }
    }

    pub const fn mode(&self) -> TraceMode { self.mode }

    /// Changes the policy and clears evidence that no longer matches it.
    /// 修改策略，并清理不再符合新策略的证据。
    pub fn set_mode(&mut self, mode: TraceMode) {
        self.mode = mode;
        self.clear();
    }

    pub const fn is_collecting(&self) -> bool { !matches!(self.mode, TraceMode::Off) }

    /// Removes all collected evidence without changing the policy.
    /// 清除全部已收集证据，但不修改策略。
    pub fn clear(&mut self) {
        self.frames.clear();
        self.frame_index.clear();
        self.current.clear();
        self.locals.clear();
        self.local_index.clear();
        self.local_name_index.clear();
        self.local_function_index.clear();
        self.edges.clear();
        self.outgoing_index.clear();
        self.incoming_index.clear();
        self.next_local_id = 0;
        self.next_frame_id = 0;
        self.error_scope_depth = 0;
    }

    pub(super) fn mark(&self) -> TraceMark {
        TraceMark {
            frames: self.frames.len(),
            locals: self.locals.len(),
            edges: self.edges.len(),
            next_local_id: self.next_local_id,
            next_frame_id: self.next_frame_id,
        }
    }

    pub(super) fn rollback(&mut self, mark: TraceMark) {
        self.frames.truncate(mark.frames);
        self.locals.truncate(mark.locals);
        self.edges.truncate(mark.edges);
        self.next_local_id = mark.next_local_id;
        self.next_frame_id = mark.next_frame_id;
        self.rebuild_indexes();
    }

    fn rebuild_indexes(&mut self) {
        self.frame_index.clear();
        for (index, frame) in self.frames.iter().enumerate() {
            self.frame_index.insert(frame.call.frame_id, index);
        }
        self.local_index.clear();
        self.local_name_index.clear();
        self.local_function_index.clear();
        for (index, local) in self.locals.iter().enumerate() {
            self.local_index.insert(local.id, index);
            self.local_name_index.entry(local.name.clone()).or_default().push(local.id);
            if let Some(frame_id) = local.frame_id
                && let Some(function) = self.frame(frame_id).map(|frame| frame.call.function) {
                    self.local_function_index.entry(function).or_default().push(local.id);
                }
        }
        self.outgoing_index.clear();
        self.incoming_index.clear();
        for (index, edge) in self.edges.iter().enumerate() {
            self.outgoing_index.entry(edge.from).or_default().push(index);
            self.incoming_index.entry(edge.to).or_default().push(index);
        }
    }

    /// Runs a fallible operation and commits evidence only when it fails.
    /// 执行一个可失败操作，只有失败时才提交追踪证据。
    ///
    /// Nested scopes share the outer transaction. This means a successful
    /// inner call remains available when its parent later returns `Err`.
    /// 嵌套作用域共享外层事务，因此内部成功但父调用最终失败时，内部证据仍会保留。
    pub fn with_result<T, E>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E> {
        if !matches!(self.mode, TraceMode::ErrorsOnly) {
            return operation(self);
        }
        let outer = self.error_scope_depth == 0;
        let mark = outer.then(|| self.mark());
        self.error_scope_depth += 1;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| operation(self)));
        self.error_scope_depth -= 1;
        match result {
            Ok(Ok(value)) => {
                if let Some(mark) = mark {
                    self.rollback(mark);
                }
                Ok(value)
            }
            Ok(Err(error)) => Err(error),
            Err(payload) => std::panic::resume_unwind(payload),
        }
    }

    pub(super) fn callsite_source(&self, file: &'static str, line: u32, column: u32) -> SourceLocation {
        SourceLocation { file, line, column, function: self.current.last().and_then(|id| self.frame(*id)).map(|frame| frame.call.function).unwrap_or("<runtime>") }
    }

    pub fn call_edges(&self) -> Vec<CallEdge> {
        let mut result = Vec::new();
        let mut seen = std::collections::BTreeSet::new();
        for frame in &self.frames {
            let Some(parent) = frame.parent.and_then(|id| self.frame(id)) else { continue };
            let edge = CallEdge { caller: parent.call.clone(), callee: frame.call.clone() };
            if seen.insert((edge.caller.frame_id, edge.callee.frame_id)) { result.push(edge); }
        }
        result
    }

    pub fn logical_call_edges(&self) -> Vec<LogicalCallEdge> {
        let mut seen = std::collections::BTreeSet::new();
        self.call_edges().into_iter().filter_map(|edge| {
            let key = (edge.caller.node, edge.caller.function, edge.callee.node, edge.callee.function);
            seen.insert(key).then_some(LogicalCallEdge { caller: edge.caller, callee: edge.callee, evidence: EvidenceKind::Live })
        }).collect()
    }
}
