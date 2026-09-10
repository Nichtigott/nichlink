//! Shared structured diagnostics and terminal tree rendering.
//! 共享结构化诊断与终端树形渲染。

use std::fmt::{self, Write as _};

use crate::registry_core::declaration::{CallSite, ProvenanceStep};
use crate::registry_core::declaration::{OwnedSourceLocation, SourceLocation};
use crate::registry_core::identity::NodeId;

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
    #[doc(hidden)]
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

// ---------------------------------------------------------------------------
// Build-phase diagnostics. Pure data plus in-memory rendering; the build
// surface collects them, the kernel owns the model.
// 构建期诊断。纯数据与内存渲染；由 build 面收集，模型归 kernel 所有。

/// One declaration error before it is rendered for rustc or a terminal.
/// 在输出给 rustc 或终端之前的一条声明错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildDiagnostic {
    pub phase: &'static str,
    pub branch: String,
    pub node: String,
    pub source: String,
    pub line: usize,
    pub function: String,
    pub field: String,
    pub expected: String,
    pub actual: String,
    pub provider: String,
    pub message: String,
}

impl BuildDiagnostic {
    pub fn new(phase: &'static str, message: impl Into<String>) -> Self {
        Self {
            phase,
            branch: String::new(),
            node: String::new(),
            source: String::new(),
            line: 0,
            function: String::new(),
            field: String::new(),
            expected: String::new(),
            actual: String::new(),
            provider: String::new(),
            message: message.into(),
        }
    }

    pub fn at(mut self, source: impl Into<String>, line: usize) -> Self {
        self.source = source.into();
        self.line = line;
        self
    }

    pub fn node(mut self, node: impl Into<String>, kind: impl Into<String>) -> Self {
        self.node = format!("{} ({})", node.into(), kind.into());
        self
    }

    pub fn branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = branch.into();
        self
    }

    pub fn function(mut self, function: impl Into<String>) -> Self {
        self.function = function.into();
        self
    }

    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = field.into();
        self
    }

    pub fn expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = expected.into();
        self
    }

    pub fn actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = actual.into();
        self
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = provider.into();
        self
    }
}

/// Sorted, deduplicated collection of [`BuildDiagnostic`]s.
/// 排序去重后的 [`BuildDiagnostic`] 集合。
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuildDiagnostics {
    items: Vec<BuildDiagnostic>,
}

impl BuildDiagnostics {
    pub fn push(&mut self, diagnostic: BuildDiagnostic) {
        self.items.push(diagnostic);
    }

    pub fn extend(&mut self, other: Self) {
        self.items.extend(other.items);
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn render(&self) -> String {
        if self.items.is_empty() {
            return String::new();
        }
        let mut items = self.items.clone();
        items.sort_by(|left, right| {
            (
                left.phase,
                &left.source,
                left.line,
                &left.node,
                &left.message,
            )
                .cmp(&(
                    right.phase,
                    &right.source,
                    right.line,
                    &right.node,
                    &right.message,
                ))
        });
        items.dedup();
        let mut output = String::from("NICHLink BUILD CHECK FAILED / NichLink 构建检查失败\n");
        for (index, diagnostic) in items.iter().enumerate() {
            if index > 0 {
                output.push('\n');
            }
            render_build_item(&mut output, diagnostic);
        }
        output
    }
}

fn render_build_item(output: &mut String, diagnostic: &BuildDiagnostic) {
    let phase = match diagnostic.phase {
        "requirements" => "requirements / 注册需求",
        "contract" => "contract / 注册合同",
        "stable-identity" => "stable identity / 稳定标识",
        "static-plan" => "static plan / 静态计划",
        other => other,
    };
    writeln!(
        output,
        "+-- phase={phase} branch={}",
        unknown(&diagnostic.branch)
    )
    .unwrap();
    if !diagnostic.node.is_empty() {
        writeln!(output, "|   node={}", diagnostic.node).unwrap();
    }
    if !diagnostic.source.is_empty() {
        if diagnostic.line == 0 {
            writeln!(output, "|   source={}", diagnostic.source).unwrap();
        } else {
            writeln!(
                output,
                "|   source={}:{}",
                diagnostic.source, diagnostic.line
            )
            .unwrap();
        }
    }
    if !diagnostic.function.is_empty() {
        writeln!(output, "|   function={}", diagnostic.function).unwrap();
    }
    if !diagnostic.field.is_empty() {
        writeln!(output, "|   field={}", diagnostic.field).unwrap();
    }
    if !diagnostic.expected.is_empty() {
        writeln!(output, "|   expected={}", diagnostic.expected).unwrap();
    }
    if !diagnostic.actual.is_empty() {
        writeln!(output, "|   actual={}", diagnostic.actual).unwrap();
    }
    if !diagnostic.provider.is_empty() {
        writeln!(output, "|   provider={}", diagnostic.provider).unwrap();
    }
    writeln!(output, "`-- {}", diagnostic.message).unwrap();
}

fn unknown(value: &str) -> &str {
    if value.is_empty() { "<unknown>" } else { value }
}

#[cfg(test)]
mod build_diagnostic_tests {
    use super::{BuildDiagnostic, BuildDiagnostics};

    #[test]
    fn render_keeps_source_and_contract_fields_together() {
        let mut diagnostics = BuildDiagnostics::default();
        let diagnostic = BuildDiagnostic::new("contract", "output contract does not match")
            .branch("control")
            .node("abc", "Button")
            .at("control/button.rs", 12)
            .function("Button::render")
            .field("output")
            .expected("Canvas")
            .actual("String")
            .provider("CanvasProvider");
        diagnostics.push(diagnostic.clone());
        diagnostics.push(diagnostic);
        let rendered = diagnostics.render();
        assert!(rendered.contains("phase=contract / 注册合同"));
        assert!(rendered.contains("source=control/button.rs:12"));
        assert!(rendered.contains("expected=Canvas"));
        assert_eq!(
            rendered.matches("output contract does not match").count(),
            1
        );
    }
}

// ---------------------------------------------------------------------------
// Face topology validation. Pure graph checks over build-collected records;
// the build surface collects the records, the kernel owns the rules.
// 注册面拓扑校验。对构建期收集的记录做纯图检查；
// 记录由 build 面收集，规则归 kernel 所有。

/// One registration face participating in a topology check.
/// 参与拓扑校验的一个注册面。
#[derive(Clone, Debug)]
pub struct TopologyRecord {
    pub id: NodeId,
    pub parent: NodeId,
    pub owns_registry: bool,
    pub source: String,
}

/// Sort `records` by identity, then check missing parents, parents that do
/// not own a registry, and parent cycles. Returns the collected diagnostics.
/// 将 `records` 按身份排序，然后检查缺失的父节点、父节点不持有注册表、
/// 以及父链成环；返回收集到的诊断。
pub fn validate_face_topology(
    records: &mut [TopologyRecord],
    package_root: NodeId,
) -> BuildDiagnostics {
    let mut errors = BuildDiagnostics::default();
    records.sort_by_key(|record| record.id);

    let ids = records
        .iter()
        .map(|record| record.id)
        .collect::<std::collections::BTreeSet<_>>();
    let owners = records
        .iter()
        .map(|record| (record.id, record.owns_registry))
        .collect::<std::collections::BTreeMap<_, _>>();
    for record in records.iter() {
        if record.parent != package_root && !ids.contains(&record.parent) {
            errors.push(
                BuildDiagnostic::new("static-plan", "parent node is missing")
                    .at(record.source.clone(), 0)
                    .field("parent")
                    .expected("registered parent")
                    .actual(record.parent.to_string()),
            );
        } else if record.parent != package_root && owners.get(&record.parent) == Some(&false) {
            errors.push(
                BuildDiagnostic::new("static-plan", "parent does not own a registry")
                    .at(record.source.clone(), 0)
                    .field("parent")
                    .expected("registry owner")
                    .actual(record.parent.to_string()),
            );
        }
    }

    let parents = records
        .iter()
        .map(|record| (record.id, record.parent))
        .collect::<std::collections::BTreeMap<_, _>>();
    for record in records.iter() {
        let mut current = record.id;
        let mut seen = std::collections::BTreeSet::new();
        while current != package_root {
            if !seen.insert(current) {
                errors.push(
                    BuildDiagnostic::new("static-plan", "parent cycle detected")
                        .at(record.source.clone(), 0)
                        .field("parent")
                        .actual(current.to_string()),
                );
                break;
            }
            let Some(parent) = parents.get(&current).copied() else {
                break;
            };
            current = parent;
        }
    }
    errors
}

#[cfg(test)]
mod topology_tests {
    use super::{TopologyRecord, validate_face_topology};
    use crate::registry_core::identity::{NodeId, ROOT_NODE_ID};

    fn record(id: u8, parent: u8, owns_registry: bool) -> TopologyRecord {
        TopologyRecord {
            id: NodeId::from_raw([id, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            parent: NodeId::from_raw([parent, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
            owns_registry,
            source: format!("face-{id}.rs"),
        }
    }

    #[test]
    fn missing_parent_is_reported() {
        let mut records = vec![record(2, 9, true)];
        let errors = validate_face_topology(&mut records, ROOT_NODE_ID);
        assert_eq!(errors.render().matches("parent node is missing").count(), 1);
    }

    #[test]
    fn parent_without_registry_is_reported() {
        let mut records = vec![record(1, 0, false), record(2, 1, true)];
        let errors = validate_face_topology(&mut records, ROOT_NODE_ID);
        assert_eq!(
            errors
                .render()
                .matches("parent does not own a registry")
                .count(),
            1
        );
    }

    #[test]
    fn parent_cycle_is_reported() {
        let mut records = vec![record(1, 2, true), record(2, 1, true)];
        let errors = validate_face_topology(&mut records, ROOT_NODE_ID);
        assert_eq!(errors.render().matches("parent cycle detected").count(), 2);
    }

    #[test]
    fn valid_chain_is_clean() {
        let root = ROOT_NODE_ID;
        let mut records = vec![
            TopologyRecord {
                parent: root,
                ..record(1, 0, true)
            },
            TopologyRecord {
                parent: NodeId::from_raw([1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
                ..record(2, 0, true)
            },
        ];
        let errors = validate_face_topology(&mut records, root);
        assert!(errors.is_empty());
    }
}
