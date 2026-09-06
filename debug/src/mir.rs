//! Small rustc/MIR bridge for NichLink's debug model.
//! NichLink 调试模型使用的轻量 rustc/MIR 桥接层。
//!
//! The optional nightly command emits textual MIR; the runtime model keeps
//! only small, evidence-labeled candidates and never treats them as live facts.

use std::collections::{BTreeSet, HashMap};
use std::fmt::{self, Write as _};

use nichlink::{CallEdge, CallTrace, EvidenceKind, SourceLocation};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirCall {
    pub caller: String,
    pub callee: String,
    pub mir_line: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirLocal {
    pub function: String,
    pub name: String,
    pub type_name: String,
    pub mir_line: usize,
}

/// Unified evidence attached to one logical call relation.
/// 一条逻辑调用关系携带的统一证据等级。
pub type CallEvidence = EvidenceKind;

/// One call edge normalized across runtime, MIR, and source evidence.
/// 跨运行时、MIR、源码证据统一表示的一条调用边。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallRelation {
    pub caller: String,
    pub callee: String,
    pub evidence: EvidenceKind,
    pub source: Option<SourceLocation>,
    pub mir_line: Option<usize>,
    pub caller_frame: Option<u64>,
    pub callee_frame: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MirGraph {
    pub functions: BTreeSet<String>,
    pub calls: Vec<MirCall>,
    pub locals: Vec<MirLocal>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirParseError {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for MirParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "MIR graph line {}: {}", self.line, self.message)
    }
}

impl std::error::Error for MirParseError {}

impl MirGraph {
    /// Parse textual MIR emitted by rustc -Zunpretty=mir.
    /// 解析 rustc -Zunpretty=mir 输出的文本。
    ///
    /// Only function, local, and direct-call records are kept. Runtime
    /// CallTrace remains authoritative for calls that actually executed.
    /// 这里只保留函数、局部变量和直接调用；真实执行的调用仍以 CallTrace 为准。
    pub fn from_mir_text(input: &str) -> Self {
        let mut graph = Self::default();
        let mut function = String::new();
        for raw in input.lines() {
            let trimmed = raw.trim();
            if let Some(name) = mir_function_name(trimmed) {
                function = name.to_owned();
                graph.functions.insert(function.clone());
                continue;
            }
            if function.is_empty() {
                continue;
            }
            if let Some(local) = mir_local(trimmed, &function, graph.locals.len() + 1) {
                graph.locals.push(local);
            }
            if let Some(callee) = mir_call(trimmed) {
                graph.calls.push(MirCall {
                    caller: function.clone(),
                    callee,
                    mir_line: graph.calls.len() + 1,
                });
            }
        }
        graph
    }

    pub fn from_jsonl(input: &str) -> Result<Self, MirParseError> {
        let mut graph = Self::default();
        for (index, raw) in input.lines().enumerate() {
            let line = index + 1;
            if raw.trim().is_empty() {
                continue;
            }
            let fields = parse_object(raw).map_err(|message| MirParseError { line, message })?;
            let kind = required(&fields, "kind", line)?;
            match kind {
                "function" => {
                    graph
                        .functions
                        .insert(required(&fields, "name", line)?.to_owned());
                }
                "call" => graph.calls.push(MirCall {
                    caller: required(&fields, "caller", line)?.to_owned(),
                    callee: required(&fields, "callee", line)?.to_owned(),
                    mir_line: number(&fields, "mir_line", line)?,
                }),
                "local" => graph.locals.push(MirLocal {
                    function: required(&fields, "function", line)?.to_owned(),
                    name: required(&fields, "name", line)?.to_owned(),
                    type_name: required(&fields, "type", line)?.to_owned(),
                    mir_line: number(&fields, "mir_line", line)?,
                }),
                other => {
                    return Err(MirParseError {
                        line,
                        message: format!("unsupported record kind `{other}`"),
                    })
                }
            }
        }
        Ok(graph)
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        writeln!(output, "MIR CANDIDATES").unwrap();
        for call in &self.calls {
            writeln!(
                output,
                "  {} -> {} @ MIR line {}",
                call.caller, call.callee, call.mir_line
            )
            .unwrap();
        }
        if self.calls.is_empty() {
            output.push_str("  (no static call candidates)\n");
        }
        writeln!(output, "LOCALS").unwrap();
        for local in &self.locals {
            writeln!(
                output,
                "  {}::{}: {} @ MIR line {}",
                local.function, local.name, local.type_name, local.mir_line
            )
            .unwrap();
        }
        output
    }

    /// Emit a compact, line-oriented artifact that build.rs can consume.
    /// 输出 build.rs 可消费的紧凑逐行 artifact。
    pub fn to_jsonl(&self) -> String {
        let mut output = String::new();
        for function in &self.functions {
            writeln!(
                output,
                "{{\"kind\":\"function\",\"name\":\"{}\"}}",
                escape_json(function)
            )
            .unwrap();
        }
        for call in &self.calls {
            writeln!(
                output,
                "{{\"kind\":\"call\",\"caller\":\"{}\",\"callee\":\"{}\",\"mir_line\":{}}}",
                escape_json(&call.caller),
                escape_json(&call.callee),
                call.mir_line
            )
            .unwrap();
        }
        for local in &self.locals {
            writeln!(output, "{{\"kind\":\"local\",\"function\":\"{}\",\"name\":\"{}\",\"type\":\"{}\",\"mir_line\":{}}}", escape_json(&local.function), escape_json(&local.name), escape_json(&local.type_name), local.mir_line).unwrap();
        }
        output
    }
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn mir_function_name(line: &str) -> Option<&str> {
    let line = line.strip_prefix("fn ")?;
    let end = line.find('(')?;
    let name = line[..end].trim();
    (!name.is_empty()).then_some(name)
}

fn mir_local(line: &str, function: &str, mir_line: usize) -> Option<MirLocal> {
    let line = line.strip_prefix("let ")?;
    let line = line.strip_prefix("mut ").unwrap_or(line);
    let (name, type_name) = line.split_once(':')?;
    let name = name.trim();
    let type_name = type_name.trim().trim_end_matches(';').trim();
    (name.starts_with('_') && !type_name.is_empty()).then(|| MirLocal {
        function: function.to_owned(),
        name: name.to_owned(),
        type_name: type_name.to_owned(),
        mir_line,
    })
}

fn mir_call(line: &str) -> Option<String> {
    let (_, expression) = line.split_once(" = ")?;
    let open = expression.find('(')?;
    let callee = expression[..open].trim().trim_start_matches("move ");
    if callee.is_empty()
        || callee.starts_with("if ")
        || callee.starts_with("match ")
        || matches!(
            callee,
            "assert" | "debug_assert" | "drop" | "panic" | "format" | "Some" | "Ok" | "Err"
        )
    {
        return None;
    }
    Some(callee.to_owned())
}

/// Static MIR candidates plus calls observed in one live run.
/// 静态 MIR 候选边与一次运行中真实观察到的调用边。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UnifiedCallGraph {
    pub static_calls: Vec<MirCall>,
    pub runtime_calls: Vec<CallEdge>,
}

impl UnifiedCallGraph {
    pub fn new(static_graph: &MirGraph, trace: &CallTrace) -> Self {
        Self {
            static_calls: static_graph.calls.clone(),
            runtime_calls: trace.call_edges(),
        }
    }

    /// Delegate topology queries to the petgraph-backed adapter.
    /// 将拓扑查询委托给基于 petgraph 的适配层。
    pub fn topology(&self) -> crate::adapters::CallGraph {
        crate::adapters::CallGraph::from_relations(&self.relations())
    }

    /// Merge all known edges into one evidence-aware relation list.
    /// 将所有已知边合并为一份带证据等级的关系列表。
    ///
    /// Live edges win over MIR candidates with the same logical symbols. A
    /// static candidate that was not observed remains visible as `Mir`, so a
    /// missing branch is not silently mistaken for a successful call.
    /// 逻辑符号相同的边以 Live 证据为准。未被观察到的静态候选仍保留为
    /// `Mir`，不会把未执行分支误报成已经成功调用。
    pub fn relations(&self) -> Vec<CallRelation> {
        let mut relations = Vec::new();
        for edge in &self.runtime_calls {
            relations.push(CallRelation {
                caller: edge.caller.function.to_owned(),
                callee: edge.callee.function.to_owned(),
                evidence: EvidenceKind::Live,
                source: edge.callee.source,
                mir_line: None,
                caller_frame: Some(edge.caller.frame_id),
                callee_frame: Some(edge.callee.frame_id),
            });
        }
        for edge in &self.static_calls {
            if relations.iter().any(|relation| {
                same_symbol(&relation.caller, &edge.caller)
                    && same_symbol(&relation.callee, &edge.callee)
            }) {
                continue;
            }
            relations.push(CallRelation {
                caller: edge.caller.clone(),
                callee: edge.callee.clone(),
                evidence: EvidenceKind::Mir,
                source: None,
                mir_line: Some(edge.mir_line),
                caller_frame: None,
                callee_frame: None,
            });
        }
        relations
    }

    pub fn render(&self) -> String {
        let mut output = String::new();
        let relations = self.relations();
        output.push_str("CALL RELATIONS\n");
        if relations.is_empty() {
            output.push_str("  (none)\n");
        } else {
            for relation in relations {
                let location = relation
                    .source
                    .map_or_else(String::new, |source| format!(" @ {source}"));
                let mir_line = relation
                    .mir_line
                    .map_or_else(String::new, |line| format!(" @ MIR {line}"));
                let frames = match (relation.caller_frame, relation.callee_frame) {
                    (Some(caller), Some(callee)) => format!(" frames={caller}->{callee}"),
                    _ => String::new(),
                };
                writeln!(
                    output,
                    "  {} {} -> {} [{}]{}{}{}",
                    relation.evidence.marker(),
                    relation.caller,
                    relation.callee,
                    relation.evidence.label(),
                    location,
                    mir_line,
                    frames,
                )
                .unwrap();
            }
        }
        output
    }
}

fn same_symbol(left: &str, right: &str) -> bool {
    left == right || left.ends_with(&format!("::{right}")) || right.ends_with(&format!("::{left}"))
}

fn required<'a>(
    fields: &'a HashMap<String, String>,
    key: &str,
    line: usize,
) -> Result<&'a str, MirParseError> {
    fields
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| MirParseError {
            line,
            message: format!("missing `{key}`"),
        })
}

fn number(
    fields: &HashMap<String, String>,
    key: &str,
    line: usize,
) -> Result<usize, MirParseError> {
    required(fields, key, line)?
        .parse()
        .map_err(|_| MirParseError {
            line,
            message: format!("`{key}` is not an integer"),
        })
}

fn parse_object(input: &str) -> Result<HashMap<String, String>, String> {
    let bytes = input.as_bytes();
    let mut cursor = 0;
    skip_space(bytes, &mut cursor);
    if bytes.get(cursor) != Some(&b'{') {
        return Err("record must start with `{`".to_owned());
    }
    cursor += 1;
    let mut fields = HashMap::new();
    loop {
        skip_space(bytes, &mut cursor);
        if bytes.get(cursor) == Some(&b'}') {
            return Ok(fields);
        }
        let key = quoted(bytes, &mut cursor)?;
        skip_space(bytes, &mut cursor);
        if bytes.get(cursor) != Some(&b':') {
            return Err("expected `:` after key".to_owned());
        }
        cursor += 1;
        skip_space(bytes, &mut cursor);
        let value = if bytes.get(cursor) == Some(&b'"') {
            quoted(bytes, &mut cursor)?
        } else {
            let start = cursor;
            while cursor < bytes.len()
                && !matches!(bytes[cursor], b',' | b'}' | b' ' | b'\n' | b'\r' | b'\t')
            {
                cursor += 1;
            }
            if start == cursor {
                return Err("expected a value".to_owned());
            }
            String::from_utf8(bytes[start..cursor].to_vec())
                .map_err(|_| "value is not UTF-8".to_owned())?
        };
        if fields.insert(key, value).is_some() {
            return Err("duplicate field".to_owned());
        }
        skip_space(bytes, &mut cursor);
        match bytes.get(cursor) {
            Some(b',') => cursor += 1,
            Some(b'}') => return Ok(fields),
            _ => return Err("expected `,` or `}`".to_owned()),
        }
    }
}

fn quoted(bytes: &[u8], cursor: &mut usize) -> Result<String, String> {
    if bytes.get(*cursor) != Some(&b'"') {
        return Err("expected a quoted string".to_owned());
    }
    *cursor += 1;
    let mut value = String::new();
    while let Some(byte) = bytes.get(*cursor).copied() {
        *cursor += 1;
        match byte {
            b'"' => return Ok(value),
            b'\\' => {
                let escaped = bytes.get(*cursor).copied().ok_or("unterminated escape")?;
                *cursor += 1;
                value.push(match escaped {
                    b'"' => '"',
                    b'\\' => '\\',
                    b'n' => '\n',
                    b'r' => '\r',
                    b't' => '\t',
                    _ => return Err("unsupported string escape".to_owned()),
                });
            }
            byte if byte.is_ascii() => value.push(byte as char),
            _ => return Err("non-ASCII strings must be JSON escaped".to_owned()),
        }
    }
    Err("unterminated string".to_owned())
}

fn skip_space(bytes: &[u8], cursor: &mut usize) {
    while bytes
        .get(*cursor)
        .is_some_and(|byte| byte.is_ascii_whitespace())
    {
        *cursor += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::{CallEvidence, MirGraph, UnifiedCallGraph};
    use crate::{CallTrace, NodeId, SourceLocation};

    #[test]
    fn parses_jsonl_without_json_dependency() {
        let graph = MirGraph::from_jsonl(
            "{\"kind\":\"function\",\"name\":\"crate::a\"}\n{\"kind\":\"call\",\"caller\":\"crate::a\",\"callee\":\"crate::b\",\"mir_line\":12}\n{\"kind\":\"local\",\"function\":\"crate::a\",\"name\":\"_1\",\"type\":\"f32\",\"mir_line\":14}\n",
        )
        .unwrap();
        assert!(graph.functions.contains("crate::a"));
        assert_eq!(graph.calls[0].mir_line, 12);
        assert_eq!(graph.locals[0].type_name, "f32");
    }

    #[test]
    fn parses_native_textual_mir() {
        let graph = MirGraph::from_mir_text(
            "fn crate::outer(_1: f32) -> f32 {\n    let mut _2: f32;\n    _2 = crate::inner(move _1);\n    return;\n}\n",
        );
        assert!(graph.functions.contains("crate::outer"));
        assert_eq!(graph.calls[0].callee, "crate::inner");
        assert_eq!(graph.locals[0].name, "_2");
    }

    #[test]
    fn keeps_static_and_live_edges_distinct() {
        let graph = MirGraph::from_jsonl(
            "{\"kind\":\"call\",\"caller\":\"a\",\"callee\":\"b\",\"mir_line\":1}\n",
        )
        .unwrap();
        let mut trace = CallTrace::full();
        trace.with_at(
            NodeId::from_path("a.rs", "A"),
            "a",
            SourceLocation {
                file: "a.rs",
                line: 1,
                column: 1,
                function: "a",
            },
            |trace| {
                trace.with_at(
                    NodeId::from_path("b.rs", "B"),
                    "b",
                    SourceLocation {
                        file: "b.rs",
                        line: 2,
                        column: 1,
                        function: "b",
                    },
                    |_| {},
                );
            },
        );
        let unified = UnifiedCallGraph::new(&graph, &trace);
        assert_eq!(unified.static_calls.len(), 1);
        assert_eq!(unified.runtime_calls.len(), 1);
        assert!(unified.render().contains("CALL RELATIONS"));
        assert_eq!(unified.relations()[0].evidence, CallEvidence::Live);
        assert_eq!(unified.relations().len(), 1);
        assert_eq!(unified.topology().edge_count(), 1);
    }

    #[test]
    fn keeps_unobserved_mir_candidate_as_a_distinct_relation() {
        let graph = MirGraph::from_jsonl(
            "{\"kind\":\"call\",\"caller\":\"crate::a\",\"callee\":\"crate::c\",\"mir_line\":7}\n",
        )
        .unwrap();
        let trace = CallTrace::full();
        let unified = UnifiedCallGraph::new(&graph, &trace);
        let relations = unified.relations();
        assert_eq!(relations.len(), 1);
        assert_eq!(relations[0].evidence, CallEvidence::Mir);
        assert_eq!(relations[0].mir_line, Some(7));
        assert!(!relations[0].evidence.confirmed());
    }

    #[test]
    fn jsonl_artifact_round_trips() {
        let graph = MirGraph::from_jsonl(
            "{\"kind\":\"function\",\"name\":\"crate::a\"}\n{\"kind\":\"call\",\"caller\":\"crate::a\",\"callee\":\"crate::b\",\"mir_line\":3}\n",
        )
        .unwrap();
        let parsed = MirGraph::from_jsonl(&graph.to_jsonl()).unwrap();
        assert_eq!(parsed, graph);
    }
}
