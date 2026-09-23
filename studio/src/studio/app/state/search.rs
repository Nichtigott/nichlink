//! Search and provenance-graph state.
//! 搜索与溯源图状态。

use nichlink_run_method::NodeId;

/// Search session state for one or two queries, including graph navigation.
/// 一次或两次查询的搜索会话状态，含调用图导航。
#[derive(Clone, Debug, Default)]
pub struct SearchState {
    /// Primary search text.
    /// 主搜索文本。
    pub query: String,
    /// Highlighted row in the primary result list.
    /// 主结果列表中高亮的行。
    pub selected: usize,
    /// First visible row of the primary result list.
    /// 主结果列表首个可见行的下标。
    pub offset: usize,
    /// Secondary search text; `None` while the comparison pane is closed.
    /// 第二路搜索文本；对比面板关闭时为 `None`。
    pub compare_query: Option<String>,
    /// Highlighted row in the comparison result list.
    /// 对比结果列表中高亮的行。
    pub compare_selected: usize,
    /// First visible row of the comparison result list.
    /// 对比结果列表首个可见行的下标。
    pub compare_offset: usize,
    /// Which pane takes input: 0 primary, 1 comparison.
    /// 接收输入的面板：0 主面板，1 对比面板。
    pub active_pane: usize,
    /// When true, the search result is shown as a navigable provenance graph.
    /// 为 true 时，搜索结果显示为可导航的溯源图。
    pub graph_mode: bool,
    /// Node anchoring the provenance graph.
    /// 溯源图的中心节点。
    pub center: Option<NodeId>,
    /// Function anchoring the provenance graph.
    /// 溯源图的中心函数。
    pub center_function: Option<String>,
    /// Source line of the center function.
    /// 中心函数所在的源码行。
    pub center_line: Option<u32>,
    /// Highlighted row in the graph's relation list.
    /// 调用图关系列表中高亮的行。
    pub graph_selected: usize,
    /// Highlighted row in the call-tree outline.
    /// 调用树大纲中高亮的行。
    pub outline_selected: usize,
    /// Whether the outline list, rather than the relation list, owns the selection.
    /// 选中项属于大纲列表还是调用图关系列表。
    pub outline_focus: bool,
    /// Focused column on the four-column graph page: A, B, call tree, data.
    /// 四列调用页当前焦点：A、B、调用树、数据流。
    pub graph_focus: usize,
    /// Active graph column set: 0 side A, 1 side B.
    /// 当前调用图列组：0 为 A 侧，1 为 B 侧。
    pub graph_side: usize,
    /// Highlighted row in the data-flow list.
    /// 数据流列表中高亮的行。
    pub data_selected: usize,
    /// Side B's graph center node.
    /// B 侧调用图的中心节点。
    pub compare_center: Option<NodeId>,
    /// Side B's graph center function.
    /// B 侧调用图的中心函数。
    pub compare_center_function: Option<String>,
    /// Side B's center function source line.
    /// B 侧中心函数所在的源码行。
    pub compare_center_line: Option<u32>,
    /// Side B's highlighted graph relation row.
    /// B 侧调用图关系列表中高亮的行。
    pub compare_graph_selected: usize,
    /// Side B's highlighted call-tree outline row.
    /// B 侧调用树大纲中高亮的行。
    pub compare_outline_selected: usize,
    /// Side B's highlighted data-flow row.
    /// B 侧数据流列表中高亮的行。
    pub compare_data_selected: usize,
}

/// One rendered search result row, before it becomes a Ratatui list item.
/// 一条渲染前的搜索结果行，之后会成为 Ratatui 列表项。
///
/// The struct used to carry a `source_index` that the three search builders
/// filled in with a descending counter and nothing ever read. It is gone; a
/// zero-caller field returns when a caller appears.
/// 本结构曾带一个 `source_index`，三个搜索构造器往里写一个递减计数，而没有任何地方读它。
/// 它已删除；零调用者的字段会在真实调用者出现时回来。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchRow {
    /// Indentation depth in the rendered result tree.
    /// 结果树中的缩进深度。
    pub depth: usize,
    /// Registration node the row points at, when it has one.
    /// 该行指向的注册节点（若有）。
    pub node: Option<NodeId>,
    /// Registry path shown for the row.
    /// 该行显示的注册表路径。
    pub path: String,
    /// Function name the row points at.
    /// 该行指向的函数名。
    pub function: String,
    /// Source line to jump to, when known.
    /// 可跳转的源码行（已知时）。
    pub line: Option<u32>,
    /// Function signature shown for context.
    /// 用于提供上下文的函数签名。
    pub signature: String,
    /// Full match text or breadcrumb shown in the row.
    /// 该行显示的完整匹配文本或面包屑。
    pub text: String,
}
