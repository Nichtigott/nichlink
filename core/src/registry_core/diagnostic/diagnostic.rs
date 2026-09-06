//! Shared structured diagnostics and terminal tree rendering.
//! 共享结构化诊断与终端树形渲染。

use std::fmt::{self, Write as _};

use crate::registry_core::declaration::{OwnedSourceLocation, SourceLocation};
use crate::registry_core::identity::NodeId;
use crate::registry_core::runtime::{CallSite, ProvenanceStep};

#[derive(Clone, Debug)]
pub struct RegistryError {
    pub node: NodeId,
    pub path: String,
    pub source: DiagnosticSource,
    pub message: String,
    pub source_chain: Vec<ProvenanceStep>,
    pub call_path: Vec<CallSite>,
    pub registration_chain: Vec<RegistrationState>,
    pub children: Vec<RegistryError>,
}

/// Heap-backed result for the comparatively rich structured error.
/// 对字段较多的结构化错误使用堆分配的返回结果。
pub type RegistryResult<T> = Result<T, Box<RegistryError>>;

#[derive(Clone, Debug)]
pub struct RegistrationState {
    pub node: NodeId,
    pub path: String,
    pub source: DiagnosticSource,
    pub state: String,
}

/// Source data shared by compiled declarations and reloadable snapshots.
/// 编译期声明与可重载快照共用的源码位置。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSource {
    /// Location embedded in a compiled registration declaration.
    /// 编译期注册声明内嵌的位置。
    Static(SourceLocation),
    /// Location owned by a reloadable registration snapshot.
    /// 可重载注册快照自有的位置。
    Owned(OwnedSourceLocation),
}

impl DiagnosticSource {
    /// Return the logical function or handle named by the declaration.
    /// 返回声明所指向的逻辑函数或 handle 名称。
    pub fn function(&self) -> &str {
        match self {
            Self::Static(source) => source.function,
            Self::Owned(source) => &source.function,
        }
    }
}

impl From<SourceLocation> for DiagnosticSource {
    fn from(source: SourceLocation) -> Self {
        Self::Static(source)
    }
}

impl From<OwnedSourceLocation> for DiagnosticSource {
    fn from(source: OwnedSourceLocation) -> Self {
        Self::Owned(source)
    }
}

impl fmt::Display for DiagnosticSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Static(source) => source.fmt(formatter),
            Self::Owned(source) => source.fmt(formatter),
        }
    }
}

impl RegistryError {
    pub(crate) fn new(
        node: NodeId,
        path: impl Into<String>,
        source: impl Into<DiagnosticSource>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            node,
            path: path.into(),
            source: source.into(),
            message: message.into(),
            source_chain: Vec::new(),
            call_path: Vec::new(),
            registration_chain: Vec::new(),
            children: Vec::new(),
        }
    }

    fn render_at(&self, depth: usize, output: &mut String) {
        let indent = "  ".repeat(depth);
        writeln!(
            output,
            "{indent}{} {} [{}] branch={} function={}",
            self.node,
            self.path,
            self.source,
            self.path,
            self.source.function()
        )
        .unwrap();
        writeln!(output, "{indent}+-- error: {}", self.message).unwrap();
        if !self.registration_chain.is_empty() {
            writeln!(output, "{indent}+-- registration chain:").unwrap();
            for (index, state) in self.registration_chain.iter().enumerate() {
                let branch = if index + 1 == self.registration_chain.len() {
                    "`--"
                } else {
                    "|--"
                };
                writeln!(
                    output,
                    "{indent}|  {branch} {} {} [{}] function={} {}",
                    state.node,
                    state.path,
                    state.source,
                    state.source.function(),
                    state.state
                )
                .unwrap();
            }
        }
        if !self.source_chain.is_empty() {
            writeln!(output, "{indent}+-- source chain:").unwrap();
            for (index, step) in self.source_chain.iter().enumerate() {
                let branch = if index + 1 == self.source_chain.len() {
                    "`--"
                } else {
                    "|--"
                };
                writeln!(
                    output,
                    "{indent}|  {branch} {} {}::{} => {}",
                    step.node, step.object, step.operation, step.value
                )
                .unwrap();
            }
        }
        if !self.call_path.is_empty() {
            write!(output, "{indent}+-- call path: ").unwrap();
            for (index, call) in self.call_path.iter().enumerate() {
                if index > 0 {
                    output.push_str(" -> ");
                }
                write!(output, "{} {}", call.node, call.function).unwrap();
            }
            output.push('\n');
            let located_calls = self
                .call_path
                .iter()
                .filter_map(|call| call.source.map(|source| (call, source)))
                .collect::<Vec<_>>();
            if !located_calls.is_empty() {
                writeln!(output, "{indent}+-- call sites:").unwrap();
                for (index, (call, source)) in located_calls.iter().enumerate() {
                    let branch = if index + 1 == located_calls.len() {
                        "`--"
                    } else {
                        "|--"
                    };
                    writeln!(
                        output,
                        "{indent}|  {branch} {} {} at {}",
                        call.node, call.function, source
                    )
                    .unwrap();
                }
            }
        }
        for child in &self.children {
            child.render_at(depth + 1, output);
        }
    }
}

impl fmt::Display for RegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output = String::new();
        self.render_at(0, &mut output);
        formatter.write_str(&output)
    }
}

impl std::error::Error for RegistryError {}
