//! Studio state and commands.
//! Studio 状态与命令。

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant, UNIX_EPOCH};

mod source_index;
pub(super) use source_index::function_source_range as app_function_source_range;
use source_index::*;
pub(crate) use source_index::{admission_text, function_line, registration_rule_text};
mod navigation;
use navigation::*;
mod sample;
use sample::sample_live_trace;
mod call_tree_queries;
mod graft;
mod graph_queries;
#[path = "hot_zones.rs"]
mod hot_zones;
pub use hot_zones::HotZones;
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

use std::cell::RefCell;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use nichlink_debug_method::{CallEvidence, CallTrace, MirCall, MirGraph};
use nichlink_run_method::{
    NodeId, PluginCatalog, PluginMode, PluginRecord, PluginSource, RegistrationSnapshot, Registry,
    face_field,
};

/// Studio's top-level state: the loaded registry, active page, and UI cursors.
/// Studio 的顶层状态：已加载的注册表、当前页面与界面游标。
///
/// `lifecycle` owns construction and reload; interactions mutate it in place.
/// `lifecycle` 负责构造与重载；交互在原位修改它。
#[derive(Debug)]
pub struct App {
    /// Loaded registration tree, replaced wholesale by every successful reload.
    /// 已加载的注册树，每次成功重载时整体替换。
    pub registry: Registry,
    /// Values captured by the current live diagnostic sample.
    /// 当前运行诊断样本捕获的值。
    pub runtime_trace: CallTrace,
    /// Optional compiler snapshot, loaded only when requested from the graph.
    /// 可选的编译器快照，只在调用图中明确请求时加载。
    pub mir_graph: Option<MirGraph>,
    /// Node currently highlighted in the tree and inspectors.
    /// 当前在树与检视器中高亮的节点。
    pub selected: NodeId,
    /// Active top-level workspace.
    /// 当前顶层工作区。
    pub page: StudioPage,
    /// Selected field in the main face inspector.
    /// 主注册面检视器当前选中的字段。
    pub details_selected: usize,
    /// Tree nodes whose children are hidden.
    /// 子节点被隐藏的树节点。
    pub collapsed: BTreeSet<NodeId>,
    /// Which Inspect pane owns keyboard focus.
    /// 哪个 Inspect 面板拥有键盘焦点。
    pub focus: Focus,
    /// Open modal screen, or `None` on the base workspace.
    /// 当前打开的模态界面；基础工作区为 `None`。
    pub overlay: Option<Overlay>,
    /// Latest status line shown in the event log.
    /// 事件日志中显示的最新状态行。
    pub event: String,
    /// Failure from the most recent reload, cleared on success.
    /// 最近一次重载的失败信息，成功时清空。
    pub reload_error: Option<ReloadError>,
    /// Set when the user asked to leave; the loop checks it after each draw.
    /// 用户请求退出时置位；事件循环每次绘制后检查。
    pub should_quit: bool,
    editor_request: Option<(PathBuf, u32)>,
    /// Click hot-zones refreshed by every draw.
    /// 每次绘制刷新的点击热区。
    pub hot: HotZones,
    /// Index of the first visible tree row.
    /// 树中首个可见行的下标。
    pub tree_offset: usize,
    /// Percentage of the Inspect workspace given to the tree pane.
    /// Inspect 工作区中分给树面板的百分比。
    pub split_percent: u16,
    /// Percentage of the graph page width given to its left column.
    /// 调用图页面宽度中分给左侧列的百分比。
    pub graph_split_percent: u16,
    graph_dragging_divider: bool,
    dragging_divider: bool,
    last_source_stamp: u128,
    /// Memoised call trees, newest first; see `CallTreeMemo`.
    /// 被备忘的调用树，最新的在前；见 `CallTreeMemo`。
    tree_cache: RefCell<Vec<CallTreeMemo>>,
    last_source_check: Instant,
}

impl App {
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

    /// Count MIR callers and callees of a function as `(callers, callees)`.
    /// 统计一个函数在 MIR 中的调用者与被调用者，返回 `(调用者, 被调用者)`。
    ///
    /// Both counts are zero until a MIR snapshot has been loaded.
    /// 在载入 MIR 快照之前，两个计数都为零。
    pub fn mir_relation_counts(&self, function: &str) -> (usize, usize) {
        let Some(graph) = self.mir_graph.as_ref() else {
            return (0, 0);
        };
        graph.calls.iter().fold((0, 0), |(callers, callees), edge| {
            (
                callers + usize::from(same_symbol(&edge.callee, function)),
                callees + usize::from(same_symbol(&edge.caller, function)),
            )
        })
    }

    /// Visible tree rows and their depth, honoring the collapsed set.
    /// 可见的树行及其深度，遵循折叠集合。
    ///
    /// The root is always first, so the list is never empty.
    /// 根始终在首位，因此列表永不为空。
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

    /// Display name of a node: `root`, its registry name, or `<missing>`.
    /// 节点的显示名：`root`、其注册名，或 `<missing>`。
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
