//! Observed value edges and provenance queries.
//! 已观测值边与来源查询。

use std::collections::{BTreeSet, VecDeque};
use std::fmt::Write as _;

use super::*;

/// One value transformation observed by the trace.
/// 追踪中观察到的一次值变换。
///
/// `source` is optional only for adapters that cannot provide a callsite;
/// built-in transformation APIs always fill it.
/// `source` 仅在外部适配器无法提供调用点时为空；内置变换 API 总会填充它。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataEdge {
    pub from: u64,
    pub to: u64,
    pub label: String,
    pub source: Option<SourceLocation>,
}

#[derive(Clone, Copy, Debug)]
pub struct DataHop<'a> {
    pub edge: &'a DataEdge,
    pub value: Option<&'a LocalValue>,
}

impl CallTrace {
    #[track_caller]
    pub fn transform(
        &mut self,
        input: LocalId,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl std::fmt::Display,
    ) -> LocalId {
        let caller = std::panic::Location::caller();
        self.transform_at_callsite(
            input,
            name,
            type_name,
            value.to_string(),
            caller.file(),
            caller.line(),
            caller.column(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn transform_at_callsite(
        &mut self,
        input: LocalId,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl Into<String>,
        file: &'static str,
        line: u32,
        column: u32,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let source = self.callsite_source(file, line, column);
        let output = self.local_at(name, type_name, value, LocalKind::Binding, source);
        self.push_edge(DataEdge {
            from: input.0,
            to: output.0,
            label: "transform".to_owned(),
            source: Some(source),
        });
        output
    }

    #[track_caller]
    pub fn return_value(
        &mut self,
        input: LocalId,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl std::fmt::Display,
    ) -> LocalId {
        let caller = std::panic::Location::caller();
        self.return_at_callsite(
            input,
            name,
            type_name,
            value.to_string(),
            caller.file(),
            caller.line(),
            caller.column(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn return_at_callsite(
        &mut self,
        input: LocalId,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl Into<String>,
        file: &'static str,
        line: u32,
        column: u32,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let source = self.callsite_source(file, line, column);
        let output = self.local_at(name, type_name, value, LocalKind::Output, source);
        self.push_edge(DataEdge {
            from: input.0,
            to: output.0,
            label: "return".to_owned(),
            source: Some(source),
        });
        output
    }

    #[track_caller]
    pub fn consume(
        &mut self,
        input: LocalId,
        function: impl Into<String>,
        parameter: impl Into<String>,
    ) -> LocalId {
        let caller = std::panic::Location::caller();
        self.consume_at_callsite(
            function,
            parameter,
            input,
            caller.file(),
            caller.line(),
            caller.column(),
        )
    }

    pub fn consume_at_callsite(
        &mut self,
        function: impl Into<String>,
        parameter: impl Into<String>,
        input: LocalId,
        file: &'static str,
        line: u32,
        column: u32,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let source = self.callsite_source(file, line, column);
        let function = function.into();
        let parameter = parameter.into();
        let output = self.local_at(
            parameter.clone(),
            "consumer",
            format!("{function}::{parameter}"),
            LocalKind::Consumer,
            source,
        );
        self.push_edge(DataEdge {
            from: input.0,
            to: output.0,
            label: format!("used by {function}::{parameter}"),
            source: Some(source),
        });
        output
    }

    pub fn data_edges(&self) -> &[DataEdge] {
        &self.edges
    }

    fn push_edge(&mut self, edge: DataEdge) {
        let index = self.edges.len();
        self.outgoing_index
            .entry(edge.from)
            .or_default()
            .push(index);
        self.incoming_index.entry(edge.to).or_default().push(index);
        self.edges.push(edge);
    }

    pub fn provenance(&self, id: LocalId) -> Vec<&LocalValue> {
        let mut result = Vec::new();
        self.collect_upstream(id.0, &mut BTreeSet::new(), &mut result);
        result
    }

    pub fn consumers(&self, id: LocalId) -> Vec<&DataEdge> {
        self.outgoing_index
            .get(&id.0)
            .into_iter()
            .flatten()
            .filter_map(|index| self.edges.get(*index))
            .collect()
    }

    pub fn outgoing(&self, id: LocalId) -> Vec<DataHop<'_>> {
        self.consumers(id)
            .into_iter()
            .map(|edge| DataHop {
                edge,
                value: self.find_local(LocalId(edge.to)),
            })
            .collect()
    }

    pub fn incoming(&self, id: LocalId) -> Vec<DataHop<'_>> {
        self.incoming_index
            .get(&id.0)
            .into_iter()
            .flatten()
            .filter_map(|index| self.edges.get(*index))
            .map(|edge| DataHop {
                edge,
                value: self.find_local(LocalId(edge.from)),
            })
            .collect()
    }

    pub fn downstream(&self, id: LocalId) -> Vec<&LocalValue> {
        let mut result = Vec::new();
        let mut queue = VecDeque::from([id.0]);
        let mut visited = BTreeSet::from([id.0]);
        while let Some(current) = queue.pop_front() {
            for index in self.outgoing_index.get(&current).into_iter().flatten() {
                let Some(edge) = self.edges.get(*index) else {
                    continue;
                };
                if !visited.insert(edge.to) {
                    continue;
                }
                if let Some(local) = self.find_local(LocalId(edge.to)) {
                    result.push(local);
                }
                queue.push_back(edge.to);
            }
        }
        result
    }

    pub fn render_provenance(&self, id: LocalId) -> String {
        let mut output = String::new();
        let Some(selected) = self.find_local(id) else {
            return format!("local {} not found", id.0);
        };
        writeln!(
            output,
            "value {} {} = {} ({}) [{}] @ {}",
            selected.id,
            selected.name,
            selected.value,
            selected.type_name,
            selected.observation.label(),
            selected.source
        )
        .unwrap();
        output.push_str("upstream:\n");
        self.render_upstream(id.0, 1, &mut BTreeSet::new(), &mut output);
        output.push_str("downstream:\n");
        self.render_downstream(id.0, 1, &mut BTreeSet::new(), &mut output);
        output
    }

    fn render_upstream(
        &self,
        id: u64,
        depth: usize,
        visited: &mut BTreeSet<u64>,
        output: &mut String,
    ) {
        if !visited.insert(id) {
            return;
        }
        for hop in self.incoming(LocalId(id)) {
            let source = hop
                .value
                .map(|local| format!("{} {} = {}", local.id, local.name, local.value))
                .unwrap_or_else(|| format!("local {}", hop.edge.from));
            writeln!(
                output,
                "{} `-- {source} [{}]",
                "  ".repeat(depth),
                hop.edge.label
            )
            .unwrap();
            self.render_upstream(hop.edge.from, depth + 1, visited, output);
        }
    }

    fn render_downstream(
        &self,
        id: u64,
        depth: usize,
        visited: &mut BTreeSet<u64>,
        output: &mut String,
    ) {
        if !visited.insert(id) {
            return;
        }
        for hop in self.outgoing(LocalId(id)) {
            let target = hop
                .value
                .map(|local| format!("{} {} = {}", local.id, local.name, local.value))
                .unwrap_or_else(|| format!("local {}", hop.edge.to));
            writeln!(
                output,
                "{} `-- {target} [{}]",
                "  ".repeat(depth),
                hop.edge.label
            )
            .unwrap();
            self.render_downstream(hop.edge.to, depth + 1, visited, output);
        }
    }

    fn collect_upstream<'a>(
        &'a self,
        id: u64,
        visited: &mut BTreeSet<u64>,
        result: &mut Vec<&'a LocalValue>,
    ) {
        if !visited.insert(id) {
            return;
        }
        for index in self.incoming_index.get(&id).into_iter().flatten() {
            if let Some(edge) = self.edges.get(*index) {
                self.collect_upstream(edge.from, visited, result);
            }
        }
        if let Some(local) = self.find_local(LocalId(id)) {
            result.push(local);
        }
    }

    pub fn render_data_flow(&self) -> String {
        let mut output = String::new();
        for local in &self.locals {
            let frame = local
                .frame_id
                .and_then(|id| self.frame(id))
                .map(|frame| frame.call.function)
                .unwrap_or("<outside-call>");
            writeln!(
                output,
                "{} [{}|{}] {}#{} depth={}: {} = {} @ {} ({})",
                local.id,
                local.kind.label(),
                local.observation.label(),
                frame,
                local.frame_id.unwrap_or(0),
                local.frame_id.map_or(0, |id| self.frame_depth(id)),
                local.name,
                local.value,
                local.source,
                local.type_name
            )
            .unwrap();
        }
        if !self.edges.is_empty() {
            output.push_str("edges:\n");
            for edge in &self.edges {
                let source = edge
                    .source
                    .map_or_else(String::new, |location| format!(" @ {location}"));
                writeln!(
                    output,
                    "  {} -> {} ({}){}",
                    edge.from, edge.to, edge.label, source
                )
                .unwrap();
            }
        }
        output
    }
}
