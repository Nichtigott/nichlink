//! Studio state and commands.
//! Studio 状态与命令。

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant, UNIX_EPOCH};

mod source_index;
pub(super) use source_index::function_source_range as app_function_source_range;
use source_index::*;
mod navigation;
use navigation::*;
mod sample;
use sample::sample_live_trace;
mod graph_queries;
mod interaction;
mod keyboard;
mod keyboard_overlay;
mod lifecycle;
mod mutations;
mod pointer;
mod search_queries;
#[path = "state/state.rs"]
mod state;
mod support;
pub use state::*;
use support::host_manifest;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use nichlink::{
    NodeId, PluginCatalog, PluginMode, PluginRecord, PluginSource, RegistrationSnapshot, Registry,
    FACE_FIELD_COUNT, FACE_PRIMARY_FIELDS,
};
use nichlink_debug::{CallEvidence, CallTrace, MirCall, MirGraph};

#[derive(Debug)]
pub struct App {
    pub registry: Registry,
    /// Values captured by the current live diagnostic sample.
    /// 当前运行诊断样本捕获的值。
    pub runtime_trace: CallTrace,
    /// Optional compiler snapshot, loaded only when requested from the graph.
    /// 可选的编译器快照，只在调用图中明确请求时加载。
    pub mir_graph: Option<MirGraph>,
    pub selected: NodeId,
    pub page: StudioPage,
    /// Selected field in the main face inspector.
    /// 主注册面检视器当前选中的字段。
    pub details_selected: usize,
    pub collapsed: BTreeSet<NodeId>,
    pub focus: Focus,
    pub overlay: Option<Overlay>,
    pub event: String,
    pub reload_error: Option<ReloadError>,
    pub should_quit: bool,
    editor_request: Option<(PathBuf, u32)>,
    pub tree_area: ratatui::layout::Rect,
    pub details_area: ratatui::layout::Rect,
    pub workspace_area: ratatui::layout::Rect,
    pub overlay_area: ratatui::layout::Rect,
    pub overlay_list_area: ratatui::layout::Rect,
    pub overlay_compare_list_area: ratatui::layout::Rect,
    pub delete_cancel_area: ratatui::layout::Rect,
    pub delete_confirm_area: ratatui::layout::Rect,
    pub action_cancel_area: ratatui::layout::Rect,
    pub action_confirm_area: ratatui::layout::Rect,
    pub action_exit_area: ratatui::layout::Rect,
    pub tree_offset: usize,
    pub split_percent: u16,
    pub graph_split_percent: u16,
    pub graph_area: ratatui::layout::Rect,
    pub graph_detail_area: ratatui::layout::Rect,
    pub graph_provenance_area: ratatui::layout::Rect,
    pub graph_callers_area: ratatui::layout::Rect,
    pub graph_center_area: ratatui::layout::Rect,
    pub graph_callees_area: ratatui::layout::Rect,
    pub graph_tree_a_area: ratatui::layout::Rect,
    pub graph_tree_b_area: ratatui::layout::Rect,
    pub graph_data_a_area: ratatui::layout::Rect,
    pub graph_data_b_area: ratatui::layout::Rect,
    pub graph_a_input_area: ratatui::layout::Rect,
    pub graph_a_center_area: ratatui::layout::Rect,
    pub graph_a_output_area: ratatui::layout::Rect,
    pub graph_b_input_area: ratatui::layout::Rect,
    pub graph_b_center_area: ratatui::layout::Rect,
    pub graph_b_output_area: ratatui::layout::Rect,
    graph_dragging_divider: bool,
    dragging_divider: bool,
    last_source_stamp: u128,
    last_source_check: Instant,
}

impl App {
    pub fn load_mir_snapshot(&mut self) -> Result<usize, String> {
        let manifest = host_manifest();
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
        let output = Command::new(cargo)
            .args(["rustc", "--manifest-path"])
            .arg(&manifest)
            .args(["--lib", "--quiet", "--", "-Zunpretty=mir"])
            .output()
            .map_err(|error| {
                format!("cannot run cargo rustc for {}: {error}", manifest.display())
            })?;
        if !output.status.success() {
            let detail = String::from_utf8_lossy(&output.stderr);
            return Err(if detail.trim().is_empty() {
                format!("cargo rustc exited with {}", output.status)
            } else if detail.contains("option `Z` is only accepted") {
                "MIR inspection requires a nightly rustc; normal Studio builds stay on stable."
                    .to_owned()
            } else {
                detail.trim().to_owned()
            });
        }
        let graph = MirGraph::from_mir_text(&String::from_utf8_lossy(&output.stdout));
        let calls = graph.calls.len();
        self.mir_graph = Some(graph);
        Ok(calls)
    }

    /// Static candidates whose caller or callee mentions a selected function.
    /// 返回涉及所选函数的静态候选调用。
    pub fn mir_candidates_for(&self, function: &str) -> Vec<&MirCall> {
        let Some(graph) = self.mir_graph.as_ref() else {
            return Vec::new();
        };
        graph
            .calls
            .iter()
            .filter(|call| {
                call.caller == function
                    || call.callee == function
                    || call.caller.ends_with(&format!("::{function}"))
                    || call.callee.ends_with(&format!("::{function}"))
            })
            .collect()
    }

    pub fn mir_relation_counts(&self, function: &str) -> (usize, usize) {
        let Some(graph) = self.mir_graph.as_ref() else {
            return (0, 0);
        };
        graph.calls.iter().fold((0, 0), |(callers, callees), edge| {
            (
                callers + usize::from(mir_name_matches(&edge.callee, function)),
                callees + usize::from(mir_name_matches(&edge.caller, function)),
            )
        })
    }

    pub fn visible_nodes(&self) -> Vec<(NodeId, usize)> {
        let root = self.registry.id();
        let mut nodes = vec![(root, 0)];
        if self.collapsed.contains(&root) {
            return nodes;
        }
        for info in self.registry.depth_first() {
            let Some(path) = self.registry.node_path(info.id) else {
                continue;
            };
            if path
                .iter()
                .take(path.len().saturating_sub(1))
                .any(|id| self.collapsed.contains(id))
            {
                continue;
            }
            nodes.push((info.id, path.len().saturating_sub(1)));
        }
        nodes
    }

    pub fn node_name(&self, id: NodeId) -> &str {
        if id == self.registry.id() {
            "root"
        } else {
            self.registry
                .find(id)
                .map_or("<missing>", |info| info.registry_name.as_str())
        }
    }

    /// Render one compact tree label: object name followed by its owner.
    /// 渲染紧凑的树节点标签：对象名称后跟所属注册面。
    pub fn node_label(&self, id: NodeId) -> String {
        if id == self.registry.id() {
            return "root".to_owned();
        }
        let name = self.node_name(id);
        let owner = self
            .registry
            .find(id)
            .and_then(|info| self.registry.path_for(info.parent))
            .unwrap_or_else(|| "root".to_owned());
        format!("{name}  <- {owner}")
    }

    pub(super) fn graph_item(&self, search: &SearchState, side: usize) -> Option<CallRef> {
        let (node, function) = if side == 0 {
            (search.center, search.center_function.as_deref())
        } else {
            (
                search.compare_center,
                search.compare_center_function.as_deref(),
            )
        };
        let node = node?;
        let info = self.registry.find(node)?;
        Some(CallRef {
            node,
            function: function.unwrap_or(&info.source.function).to_owned(),
            file: info.source.file.to_owned(),
        })
    }

    pub(crate) fn graph_tree_item(
        &self,
        search: &SearchState,
        side: usize,
        cursor: usize,
    ) -> Option<CallRef> {
        let center = self.graph_item(search, side)?;
        self.call_tree_targets(&center).get(cursor)?.clone()
    }
}

#[cfg(test)]
mod tests;
