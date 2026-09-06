//! Call graph and evidence queries owned by App.
//! App 所有的调用图与证据查询。

use super::*;

impl App {
    pub fn call_chain(&self, center: NodeId, selected_function: Option<&str>) -> Vec<CallRef> {
        let default_function = self
            .registry
            .find(center)
            .map(|info| info.source.function.as_str())
            .unwrap_or("");
        let function = selected_function
            .filter(|name| !name.is_empty())
            .unwrap_or(default_function);
        let (callers, callees) = self.call_relations(center, function);
        callers
            .into_iter()
            .chain(std::iter::once(CallRef {
                node: center,
                function: function.to_owned(),
                file: self
                    .registry
                    .find(center)
                    .map(|info| info.source.file.as_str())
                    .unwrap_or("")
                    .to_owned(),
            }))
            .chain(callees)
            .collect()
    }

    /// Return the navigable entries in the call-tree preview.
    /// 返回调用树预览中可以跳转的条目。
    ///
    /// Non-node rows (section headers and transforms) are represented by
    /// `None`, so the UI can keep selection and Enter navigation in lockstep.
    pub(crate) fn call_tree_targets(&self, item: &CallRef) -> Vec<Option<CallRef>> {
        let (callers, callees) = self.call_relations(item.node, &item.function);
        let mut targets = vec![Some(item.clone()), None];
        if callers.is_empty() {
            targets.push(None);
        } else {
            targets.extend(callers.into_iter().map(Some));
        }
        targets.push(None);
        targets.extend(self.call_tree_transforms(item).into_iter().map(|_| None));
        targets.push(None);
        if callees.is_empty() {
            targets.push(None);
        } else {
            targets.extend(callees.into_iter().map(Some));
        }
        targets
    }

    /// Extract a few value-changing statements for the compact call tree.
    /// 提取紧凑调用树中少量改变值的语句。
    pub(crate) fn call_tree_transforms(&self, item: &CallRef) -> Vec<String> {
        let Some(info) = self.registry.find(item.node) else {
            return Vec::new();
        };
        let source = source_path_for(&info.source.file);
        let Ok(text) = std::fs::read_to_string(source) else {
            return Vec::new();
        };
        let lines = text.lines().collect::<Vec<_>>();
        function_source_range(&lines, &item.function)
            .map(|(start, end)| {
                lines[start..=end]
                    .iter()
                    .filter_map(|line| {
                        let trimmed = line.trim();
                        (trimmed.starts_with("let ") || trimmed.starts_with("return "))
                            .then_some(trimmed.to_owned())
                    })
                    .take(4)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Find real function-to-function edges from source bodies. Registration
    /// parents and `use` statements are intentionally ignored.
    /// 从源码函数体提取真实函数调用边，明确忽略注册父子关系和 `use`。
    pub fn call_relations(&self, center: NodeId, function: &str) -> (Vec<CallRef>, Vec<CallRef>) {
        let mut definitions = Vec::new();
        let mut bodies = Vec::new();
        for info in self.registry.depth_first() {
            let source = source_path_for(&info.source.file);
            let Ok(text) = std::fs::read_to_string(source) else {
                continue;
            };
            for function in function_symbols(&text) {
                definitions.push((function.name.clone(), info.id));
                bodies.push((function.name, info.id, function.body));
            }
        }
        let target_name = if function.is_empty() {
            self.registry
                .find(center)
                .map(|info| info.source.function.as_str())
                .unwrap_or("")
        } else {
            function
        };
        let mut callers = Vec::new();
        let mut callees = Vec::new();
        for (name, node, body) in bodies {
            let calls_target =
                body_calls(&body, target_name) && !(node == center && name == target_name);
            if calls_target {
                let file = self
                    .registry
                    .find(node)
                    .map(|info| info.source.file.as_str())
                    .unwrap_or("");
                let reference = CallRef {
                    node,
                    function: name.clone(),
                    file: file.to_owned(),
                };
                if !callers
                    .iter()
                    .any(|item: &CallRef| item.node == node && item.function == reference.function)
                {
                    callers.push(reference);
                }
            }
            if node == center && name == target_name {
                for (callee, callee_node) in &definitions {
                    if callee != target_name
                        && body_calls(&body, callee)
                        && !callees.iter().any(|item: &CallRef| {
                            item.node == *callee_node && item.function == *callee
                        })
                    {
                        let file = self
                            .registry
                            .find(*callee_node)
                            .map(|info| info.source.file.as_str())
                            .unwrap_or("");
                        callees.push(CallRef {
                            node: *callee_node,
                            function: callee.clone(),
                            file: file.to_owned(),
                        });
                    }
                }
            }
        }
        for edge in self.runtime_trace.call_edges() {
            if edge.callee.node == center && edge.callee.function == target_name {
                push_call_ref(
                    &self.registry,
                    &mut callers,
                    edge.caller.node,
                    edge.caller.function,
                );
            }
            if edge.caller.node == center && edge.caller.function == target_name {
                push_call_ref(
                    &self.registry,
                    &mut callees,
                    edge.callee.node,
                    edge.callee.function,
                );
            }
        }
        if let Some(graph) = &self.mir_graph {
            for call in &graph.calls {
                if mir_name_matches(&call.callee, target_name) {
                    self.push_resolved_mir_refs(&mut callers, &definitions, &call.caller);
                }
                if mir_name_matches(&call.caller, target_name) {
                    self.push_resolved_mir_refs(&mut callees, &definitions, &call.callee);
                }
            }
        }
        (callers, callees)
    }

    fn push_resolved_mir_refs(
        &self,
        output: &mut Vec<CallRef>,
        definitions: &[(String, NodeId)],
        symbol: &str,
    ) {
        for (name, node) in definitions {
            if mir_name_matches(symbol, name) {
                push_call_ref(&self.registry, output, *node, name);
            }
        }
    }

    pub fn call_evidence(&self, caller: &CallRef, callee: &CallRef) -> CallEvidence {
        if self.runtime_trace.call_edges().iter().any(|edge| {
            edge.caller.node == caller.node
                && edge.caller.function == caller.function
                && edge.callee.node == callee.node
                && edge.callee.function == callee.function
        }) {
            return CallEvidence::Live;
        }
        if self.mir_graph.as_ref().is_some_and(|graph| {
            graph.calls.iter().any(|edge| {
                mir_name_matches(&edge.caller, &caller.function)
                    && mir_name_matches(&edge.callee, &callee.function)
            })
        }) {
            return CallEvidence::Mir;
        }
        CallEvidence::Source
    }

    pub fn owns_registry(&self, id: NodeId) -> bool {
        self.registry.registry(id).is_some()
    }
}
