//! Click hot-zone caches refreshed by every Studio draw.
//! 每次 Studio 绘制时刷新的点击热区缓存。

use ratatui::layout::Rect;

/// The rectangles Studio hit-tests pointer events against.
/// Studio 用于命中测试指针事件的矩形集合。
///
/// Every draw recomputes these from the current frame, so they are one cache
/// rather than thirty independent fields on `App`. The rectangle names are
/// unchanged, so a read or write site only gains one `hot.` hop.
/// 每次绘制都会根据当前帧重新计算它们，因此它们是同一份缓存，而不是 `App` 上的
/// 三十个独立字段。矩形名称保持不变，读写点只多一跳 `hot.`。
#[derive(Clone, Debug, Default)]
pub struct HotZones {
    /// Click area of the registry-tree pane.
    /// 注册树面板的点击区域。
    pub tree_area: Rect,
    /// Click area of the selected face's detail pane.
    /// 所选注册面详情面板的点击区域。
    pub details_area: Rect,
    /// Click area of the whole workspace body, below the brand.
    /// 品牌之下整个工作区主体的点击区域。
    pub workspace_area: Rect,
    /// Click area of the open overlay's frame.
    /// 当前浮层外框的点击区域。
    pub overlay_area: Rect,
    /// Click rows of the overlay's primary list.
    /// 浮层主列表的点击行区域。
    pub overlay_list_area: Rect,
    /// Click target of the delete screen's Cancel button.
    /// 删除界面 Cancel 按钮的点击目标。
    pub delete_cancel_area: Rect,
    /// Click target of the delete screen's Confirm button.
    /// 删除界面 Confirm 按钮的点击目标。
    pub delete_confirm_area: Rect,
    /// Click target of the focused form's Cancel button.
    /// 当前表单 Cancel 按钮的点击目标。
    pub action_cancel_area: Rect,
    /// Click target reserved for a Validate button; no screen sets it yet.
    /// 预留给 Validate 按钮的点击目标；目前没有界面设置它。
    pub action_validate_area: Rect,
    /// Click target reserved for an Edit button; no screen sets it yet.
    /// 预留给 Edit 按钮的点击目标；目前没有界面设置它。
    pub action_edit_area: Rect,
    /// Click target of the focused form's Confirm button.
    /// 当前表单 Confirm 按钮的点击目标。
    pub action_confirm_area: Rect,
    /// Click target of the focused form's Exit or Close button.
    /// 当前表单 Exit/Close 按钮的点击目标。
    pub action_exit_area: Rect,
    /// Clickable compose rows of the external-graft screen.
    /// 外部 graft 界面可点击的撰写区行。
    pub graft_compose_area: Rect,
    /// Click area of the whole provenance-graph page.
    /// 整个溯源图页面的点击区域。
    pub graph_area: Rect,
    /// Click area of the graph page's detail pane.
    /// 调用图页详情面板的点击区域。
    pub graph_detail_area: Rect,
    /// Click area of the graph page's provenance pane.
    /// 调用图页溯源面板的点击区域。
    pub graph_provenance_area: Rect,
    /// Click area of the call-tree pane.
    /// 调用树面板的点击区域。
    pub graph_tree_area: Rect,
    /// Click area of the data-flow pane.
    /// 数据流面板的点击区域。
    pub graph_data_area: Rect,
}
