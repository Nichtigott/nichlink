//! Registry tree inspection output.
//! 注册树检视输出。

use std::fmt::Write as _;

use super::Registry;
use crate::registry_core::declaration::SourceLocation;
use crate::registry_core::diagnostic::{RegistryError, RegistryResult};
use crate::registry_core::identity::NodeId;
use crate::registry_core::runtime::{CallTrace, RuntimeValue};

impl Registry {
    fn ordered_entries(&self) -> Vec<&super::RegisteredEntry> {
        let mut entries = self.entries.values().collect::<Vec<_>>();
        entries.sort_by(|left, right| {
            (&left.info.registry_name, left.info.id)
                .cmp(&(&right.info.registry_name, right.info.id))
        });
        entries
    }

    pub fn health_check(
        &self,
        node: NodeId,
        value: &RuntimeValue,
        trace: &CallTrace,
    ) -> RegistryResult<()> {
        let Some(entry) = self.entry_at(node) else {
            return Err(RegistryError::new(
                node,
                format!("<unknown:{node}>"),
                SourceLocation {
                    file: "<runtime>",
                    line: 0,
                    column: 0,
                    function: "health_check",
                },
                "object path was not registered",
            )
            .into());
        };
        let path = self
            .path_for(node)
            .expect("a registered entry always has a display path");
        let mut failures = Vec::new();
        for check in &entry.info.runtime_checks {
            if let Err(failure) = check.run(value) {
                let mut child = RegistryError::new(
                    node,
                    path.clone(),
                    entry.info.source.clone(),
                    format!("check `{}`: {}", failure.check, failure.message),
                );
                child.source_chain = failure.provenance.steps;
                child.call_path = trace.current_path();
                child.registration_chain = self.registration_chain(node);
                failures.push(child);
            }
        }
        if failures.is_empty() {
            return Ok(());
        }
        let mut error = RegistryError::new(
            node,
            path,
            entry.info.source.clone(),
            "runtime health check failed",
        );
        error.call_path = trace.current_path();
        error.registration_chain = self.registration_chain(node);
        error.children = failures;
        Err(Box::new(error))
    }

    pub fn dump(&self) -> String {
        let mut output = String::new();
        writeln!(
            output,
            "REGISTRATION TREE  nodes={} registries={} checks={}",
            self.node_count(),
            self.registry_count(),
            self.check_count()
        )
        .unwrap();
        writeln!(
            output,
            "root [registry] kind=root node={} entries={} path={}",
            self.header.id,
            self.len(),
            self.header.path
        )
        .unwrap();
        self.render_dump_entries("", &mut output);
        output
    }

    /// Render the compact tree used by the resident terminal inspector.
    /// 渲染常驻终端检视器使用的紧凑树图。
    pub fn tree_outline(&self) -> String {
        let mut output = String::new();
        writeln!(
            output,
            "TREE  nodes={} registries={} checks={}",
            self.node_count(),
            self.registry_count(),
            self.check_count()
        )
        .unwrap();
        writeln!(output, "root [registry] node={}", self.header.id).unwrap();
        self.render_outline_entries("", &mut output);
        output
    }

    fn render_outline_entries(&self, prefix: &str, output: &mut String) {
        let count = self.entries.len();
        for (index, entry) in self.ordered_entries().into_iter().enumerate() {
            let last = index + 1 == count;
            let connector = if last { "`--" } else { "|--" };
            let state = entry.child.as_ref().map_or_else(
                || "object".to_owned(),
                |child| format!("registry entries={}", child.len()),
            );
            writeln!(
                output,
                "{}{} {} [{}] kind={} node={}",
                prefix, connector, entry.info.registry_name, state, entry.info.kind, entry.info.id
            )
            .unwrap();
            if let Some(child) = entry.child.as_ref() {
                let child_prefix = format!("{}{}", prefix, if last { "   " } else { "|  " });
                child.render_outline_entries(&child_prefix, output);
            }
        }
    }

    fn render_dump_entries(&self, prefix: &str, output: &mut String) {
        let count = self.entries.len();
        for (index, entry) in self.ordered_entries().into_iter().enumerate() {
            let last = index + 1 == count;
            let connector = if last { "`--" } else { "|--" };
            let path = format!("{}/{}", self.header.path, entry.info.registry_name);
            let state = entry.child.as_ref().map_or_else(
                || "object".to_owned(),
                |child| format!("registry entries={}", child.len()),
            );
            writeln!(
                output,
                "{}{} {} [{}] kind={} node={} path={} declared-at={} function={} checks={}",
                prefix,
                connector,
                entry.info.registry_name,
                state,
                entry.info.kind,
                entry.info.id,
                path,
                entry.info.source,
                entry.info.source.function,
                entry.info.runtime_checks.len()
            )
            .unwrap();
            if let Some(child) = entry.child.as_ref() {
                let child_prefix = format!("{}{}", prefix, if last { "   " } else { "|  " });
                child.render_dump_entries(&child_prefix, output);
            }
        }
    }
}
