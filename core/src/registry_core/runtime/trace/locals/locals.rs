//! Runtime locals and local-value queries.
//! 运行时局部值与局部值查询。

use std::collections::BTreeSet;

use crate::registry_core::declaration::SourceLocation;

use super::*;

/// The role of a value in one function invocation.
/// 一个函数调用中局部值的角色。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalKind {
    Input,
    Binding,
    Output,
    Consumer,
}

/// Whether a local value came from a live trace or a static inference.
/// 局部值来自实时追踪还是静态推断。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Observation {
    Observed,
    Unobserved,
}

impl Observation {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Observed => "observed",
            Self::Unobserved => "unobserved",
        }
    }
}

impl LocalKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Input => "input",
            Self::Binding => "let",
            Self::Output => "return",
            Self::Consumer => "consumer",
        }
    }
}

/// A value captured or inferred for a single invocation.
/// 单次调用中捕获或推断的值。
///
/// `observation` is `Unobserved` for MIR-only locals. Such values describe a
/// possible binding and never claim that a runtime value was seen.
/// `observation` 为 `Unobserved` 时表示仅由 MIR 推断，不能当作运行时实值。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocalValue {
    pub id: u64,
    pub name: String,
    pub type_name: String,
    pub value: String,
    pub kind: LocalKind,
    pub source: SourceLocation,
    pub frame_id: Option<u64>,
    pub observation: Observation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LocalId(pub u64);

impl CallTrace {
    #[track_caller]
    pub fn local(
        &mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl std::fmt::Display,
        kind: LocalKind,
    ) -> LocalId {
        let caller = std::panic::Location::caller();
        self.local_at_callsite(
            name,
            type_name,
            value.to_string(),
            kind,
            caller.file(),
            caller.line(),
            caller.column(),
        )
    }

    pub fn local_at(
        &mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl Into<String>,
        kind: LocalKind,
        source: SourceLocation,
    ) -> LocalId {
        self.local_with_observation(name, type_name, value, kind, source, Observation::Observed)
    }

    fn local_with_observation(
        &mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl Into<String>,
        kind: LocalKind,
        source: SourceLocation,
        observation: Observation,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let id = LocalId(self.next_local_id);
        self.next_local_id = self.next_local_id.wrapping_add(1);
        let local = LocalValue {
            id: id.0,
            name: name.into(),
            type_name: type_name.into(),
            value: value.into(),
            kind,
            source,
            frame_id: self.current.last().copied(),
            observation,
        };
        self.locals.push(local);
        self.local_index.insert(id.0, self.locals.len() - 1);
        let local = self.locals.last().expect("local was just pushed");
        self.local_name_index
            .entry(local.name.clone())
            .or_default()
            .push(id.0);
        if let Some(frame_id) = local.frame_id {
            if let Some(function) = self.frame(frame_id).map(|frame| frame.call.function) {
                self.local_function_index
                    .entry(function)
                    .or_default()
                    .push(id.0);
            }
        }
        id
    }

    /// Add a MIR candidate whose runtime value was not observed.
    /// 添加运行时未观察到的 MIR 候选局部变量。
    pub fn inferred_local(
        &mut self,
        function: impl Into<String>,
        name: impl Into<String>,
        type_name: impl Into<String>,
        line: u32,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let function = function.into();
        let name = name.into();
        self.local_with_observation(
            format!("{function}::{name}"),
            type_name,
            "<not observed>",
            LocalKind::Binding,
            SourceLocation {
                file: "<rustc-mir>",
                line,
                column: 0,
                function: "<rustc-mir>",
            },
            Observation::Unobserved,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn local_at_callsite(
        &mut self,
        name: impl Into<String>,
        type_name: impl Into<String>,
        value: impl Into<String>,
        kind: LocalKind,
        file: &'static str,
        line: u32,
        column: u32,
    ) -> LocalId {
        if matches!(self.mode, TraceMode::Off) {
            return LocalId(0);
        }
        let function = self
            .current
            .last()
            .and_then(|id| self.frame(*id))
            .map(|frame| frame.call.function)
            .unwrap_or("<local>");
        self.local_at(
            name,
            type_name,
            value,
            kind,
            SourceLocation {
                file,
                line,
                column,
                function,
            },
        )
    }

    pub fn locals(&self) -> &[LocalValue] {
        &self.locals
    }

    pub fn path_for_local(&self, id: LocalId) -> Vec<CallSite> {
        self.find_local(id)
            .and_then(|local| local.frame_id)
            .map_or_else(Vec::new, |frame_id| self.path_for(frame_id))
    }

    pub fn find_local(&self, id: LocalId) -> Option<&LocalValue> {
        self.local_index
            .get(&id.0)
            .and_then(|index| self.locals.get(*index))
    }

    pub fn matching_locals(&self, query: &str) -> Vec<&LocalValue> {
        let query = query.trim().to_ascii_lowercase();
        if query.is_empty() {
            return self.locals.iter().collect();
        }
        let exact_ids = self
            .local_name_index
            .iter()
            .filter(|(name, _)| name.to_ascii_lowercase() == query)
            .flat_map(|(_, ids)| ids.iter().copied())
            .chain(
                self.local_function_index
                    .iter()
                    .filter(|(function, _)| function.to_ascii_lowercase() == query)
                    .flat_map(|(_, ids)| ids.iter().copied()),
            )
            .collect::<BTreeSet<_>>();
        if !exact_ids.is_empty() {
            return exact_ids
                .into_iter()
                .filter_map(|id| self.find_local(LocalId(id)))
                .collect();
        }
        self.locals
            .iter()
            .filter(|local| {
                local.name.to_ascii_lowercase().contains(&query)
                    || local.type_name.to_ascii_lowercase().contains(&query)
                    || local.value.to_ascii_lowercase().contains(&query)
                    || local.source.file.to_ascii_lowercase().contains(&query)
                    || local.source.function.to_ascii_lowercase().contains(&query)
                    || local
                        .frame_id
                        .is_some_and(|frame_id| self.frame_chain_matches(frame_id, &query))
            })
            .collect()
    }
}
