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

/// Refuse to start when there is no project to open, with the reason.
/// 没有可打开的项目时拒绝启动，并给出原因。
///
/// Called before the terminal is taken over, so a launch with nothing to edit
/// fails with a message and a non-zero exit instead of showing an empty tree and
/// letting the next authoring command write into whatever directory was left.
/// 在接管终端之前调用，因此"没有东西可编辑"的启动会带着消息与非零退出失败，而不是先显示
/// 一棵空树，再让随后的创作命令写进剩下那个目录。
pub(super) fn preflight(explicit: Option<&std::path::Path>) -> Result<PathBuf, String> {
    support::resolve_project(explicit)
}

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
    /// The `rataflow` widget that draws the call tree, kept between frames
    /// because pan and zoom are *its* state: rebuilding it every frame would
    /// reset the viewport every frame. The key names the tree it was built from,
    /// and the cursor it marks, so a re-centred focus rebuilds it, a moved cursor
    /// re-stamps the mark while keeping the viewport, and a frame that changes
    /// neither reuses it untouched (which is what lets a drag run).
    /// 绘制调用树的 `rataflow` 控件，跨帧保留：平移与缩放是**它**的状态，每帧重建就等于每帧
    /// 重置视口。键记录它是从哪棵树构建的，因此重新居中会重建它，而游标移动不会。
    #[cfg(feature = "node-graph")]
    pub graph_flow: Option<(String, usize, rataflow::Flow)>,
    /// Which model axis the call tree was last drawn down the screen: `true` when
    /// levels run downwards (the widget, or the top-down canvas), `false` when
    /// they run across it. The arrow keys read this, so they mean what the picture
    /// shows instead of what the model is called.
    /// 调用树上次把哪个模型轴画在屏幕向下的方向：`true` 表示层向下延伸（控件，或自上而下的
    /// 画布），`false` 表示层横着延伸。方向键读它，因此键的含义与图一致，而不是与模型的叫法
    /// 一致。
    pub tree_top_down: bool,
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

    pub(super) fn graph_item(&self, search: &SearchState) -> Option<CallRef> {
        let node = search.center?;
        let info = self.registry.find(node)?;
        Some(CallRef {
            node,
            function: search
                .center_function
                .clone()
                .unwrap_or_else(|| info.source.function.to_owned()),
            file: info.source.file.to_owned(),
        })
    }

    pub(crate) fn graph_tree_item(&self, search: &SearchState, cursor: usize) -> Option<CallRef> {
        let center = self.graph_item(search)?;
        self.call_tree_targets(&center).get(cursor)?.clone()
    }
}

#[cfg(test)]
mod tests;
