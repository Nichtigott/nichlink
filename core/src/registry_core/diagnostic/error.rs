//! Structured registration failures and their rendered evidence.
//! 结构化注册失败及其渲染出的证据。

use super::*;

/// One structured registration failure, with the evidence that produced it.
/// 一条结构化注册失败，以及产生它的证据。
///
/// The fields are private so the construction contract is the only way in:
/// [`RegistryError::new`] starts an error with its four essential facts, and
/// the `with_*`/`*_mut` methods attach the optional chains afterwards.
/// 字段私有，构造契约是唯一的入口：[`RegistryError::new`] 用四项必要事实开启一条错误，
/// 可选链条随后由 `with_*`/`*_mut` 方法挂上。
#[derive(Clone, Debug)]
pub struct RegistryError {
    node: NodeId,
    path: String,
    source: DiagnosticSource,
    message: String,
    source_chain: Vec<ProvenanceStep>,
    call_path: Vec<CallSite>,
    registration_chain: Vec<RegistrationState>,
    children: Vec<RegistryError>,
}

/// Heap-backed result for the comparatively rich structured error.
/// 对字段较多的结构化错误使用堆分配的返回结果。
pub type RegistryResult<T> = Result<T, Box<RegistryError>>;

/// One hop of the registration chain that reached the failing face.
/// 抵达失败注册面所经注册链上的一跳。
#[derive(Clone, Debug)]
pub struct RegistrationState {
    /// Identity of the face registered at this hop.
    /// 该跳注册的注册面身份。
    pub node: NodeId,
    /// Logical registry path of this hop.
    /// 该跳的逻辑注册表路径。
    pub path: String,
    /// Declaration source this hop was registered from.
    /// 该跳注册自的声明源码位置。
    pub source: DiagnosticSource,
    /// Free-form snapshot of what this hop held when the chain was built.
    /// 构建该链时这一跳所持状态的自由文本快照。
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
    /// Start an error from the four facts every failure carries.
    /// 用每条失败都携带的四项事实开启一条错误。
    ///
    /// The optional chains (`source_chain`, `call_path`, `registration_chain`,
    /// `children`) start empty and are attached by the builder methods.
    /// 可选链条（`source_chain`、`call_path`、`registration_chain`、`children`）起始为空，
    /// 由 builder 方法挂上。
    pub fn new(
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

    // The four essential facts plus the aggregated children are readable again.
    // B3a removed every reader because nothing in the workspace called them, but
    // `Registry::health_check` is a documented host API now, so a host that
    // receives one of these errors must be able to map it back to its `node`,
    // `path`, `source`, and `children` instead of scraping `Display`. The other
    // three readers (`source_chain`, `call_path`, `registration_chain`) stay
    // deleted until a caller needs them, and the `*_mut` setters plus
    // `with_children` remain the construction/refinement path.
    // Principle: a zero-caller deletion is reversed when a real caller appears.
    // 四项必要事实与聚合的子错误重新可读。B3a 因工作区内无调用者删掉了所有读取器，但
    // `Registry::health_check` 现在是有文档的宿主 API，因此拿到这类错误的宿主必须能把
    // 它映射回 `node`、`path`、`source` 与 `children`，而不是去解析 `Display`。另外三个
    // 读取器（`source_chain`、`call_path`、`registration_chain`）在有调用者之前保持删除，
    // `*_mut` setter 与 `with_children` 仍是构造/细化路径。
    // 原则：零调用者删除在真实调用者出现时予以撤销。

    /// The identity of the registration face this failure is about.
    /// 该失败所针对的注册面身份。
    pub fn node(&self) -> NodeId {
        self.node
    }

    /// The logical path the failure was reported at.
    /// 报告该失败时所在的逻辑路径。
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The declaration source this failure points back to.
    /// 该失败指回的声明源码位置。
    pub fn source(&self) -> &DiagnosticSource {
        &self.source
    }

    /// The human-readable failure message.
    /// 人类可读的失败消息。
    pub fn message(&self) -> &str {
        &self.message
    }

    /// The aggregated child failures, one per failed sub-check.
    /// 聚合的子失败，每个失败的子检查一条。
    pub fn children(&self) -> &[RegistryError] {
        &self.children
    }

    /// Attach the aggregated child failures.
    /// 挂上聚合的子失败。
    pub fn with_children(mut self, children: Vec<RegistryError>) -> Self {
        self.children = children;
        self
    }

    /// Replace the aggregated child failures in place.
    /// 就地替换聚合的子失败。
    pub fn children_mut(&mut self) -> &mut Vec<RegistryError> {
        &mut self.children
    }

    /// Replace the failure message in place.
    /// 就地替换失败消息。
    pub fn message_mut(&mut self) -> &mut String {
        &mut self.message
    }

    /// Replace the source chain in place.
    /// 就地替换来源链。
    pub fn source_chain_mut(&mut self) -> &mut Vec<ProvenanceStep> {
        &mut self.source_chain
    }

    /// Replace the observed call path in place.
    /// 就地替换已观测调用路径。
    pub fn call_path_mut(&mut self) -> &mut Vec<CallSite> {
        &mut self.call_path
    }

    /// Replace the registration chain in place.
    /// 就地替换注册链。
    pub fn registration_chain_mut(&mut self) -> &mut Vec<RegistrationState> {
        &mut self.registration_chain
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A host that receives a structured error can recover the facts it was
    /// required to hand to `RegistryError::new` — node, path, source, message —
    /// plus the aggregated children, instead of parsing `Display`.
    /// `Registry::health_check` is the documented caller that made these readers
    /// necessary again after B3a removed them.
    /// 收到结构化错误的宿主能取回它必须交给 `RegistryError::new` 的那些事实——node、
    /// path、source、message——以及聚合的子错误，而不必解析 `Display`。
    /// `Registry::health_check` 正是这些读取器在 B3a 删除之后重新必要的那个有文档的
    /// 调用者。
    #[test]
    fn an_error_still_exposes_the_facts_a_host_must_report() {
        let node = NodeId::from_raw([7; 16]);
        let source = SourceLocation {
            file: "button.rs",
            line: 3,
            column: 1,
            function: "Button",
        };
        let child = RegistryError::new(
            node,
            "root/control/button",
            source,
            "check `non_empty_text` failed",
        );
        let error = RegistryError::new(
            node,
            "root/control/button",
            source,
            "runtime health check failed",
        )
        .with_children(vec![child]);

        assert_eq!(error.node(), node);
        assert_eq!(error.path(), "root/control/button");
        assert_eq!(error.source().function(), "Button");
        assert_eq!(error.message(), "runtime health check failed");
        assert_eq!(error.children().len(), 1);
        assert_eq!(
            error.children()[0].message(),
            "check `non_empty_text` failed"
        );
    }
}
