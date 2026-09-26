//! Studio focus, overlays, and form state.
//! Studio 焦点、浮层和表单状态。
//!
//! The state vocabulary is grouped by screen; this parent only mounts the
//! group files and re-exports them so every existing path keeps resolving.
//! 状态词汇按界面分组；本父模块只挂载各分组文件并重新导出，
//! 让所有既有路径继续可解析。

#[path = "forms.rs"]
mod forms;
#[path = "graft.rs"]
mod graft;
#[path = "misc.rs"]
mod misc;
#[path = "new_project_field.rs"]
pub(crate) mod new_project_field;
#[path = "pages.rs"]
mod pages;
#[path = "plugin_field.rs"]
pub(crate) mod plugin_field;
#[path = "search.rs"]
mod search;

pub use forms::{AddState, NewProjectState, PluginState};
pub use graft::{GraftDeclaration, GraftPlanRow, GraftState};
pub use misc::{CallRef, CallTreeView, Overlay, ReloadError};
pub use pages::{Focus, StudioPage};
pub use search::{SearchRow, SearchState};

pub(crate) use forms::{face_field_indices, move_face_field};
pub(crate) use misc::{CallTreeMemo, push_call_ref, source_path_for};
// The kernel owns the one `::`-bounded name match. Studio used to carry a
// one-directional copy that also allocated a `format!` per comparison, which
// let the graph it draws disagree with the evidence the kernel merged.
// 内核拥有唯一的"按 `::` 边界匹配名字"规则。Studio 曾带一份单向副本，而且每次比较都会
// 分配一个 `format!`，这会让它画出的图与内核归并的证据不一致。
pub(crate) use nichlink_run_method::mir::same_symbol;
// The layered call tree is kernel vocabulary too: Studio builds the relation
// list, the kernel decides the levels, lanes and cuts. `CallTreeNode` is named
// only by the widget drawer's node text, so it is re-exported with that feature.
// 分层调用树同样是内核词汇：Studio 提供关系列表，内核决定层、车道与裁剪。`CallTreeNode`
// 只被控件绘制方的节点文本命名，因此与该特性一同重导出。
#[cfg(feature = "node-graph")]
pub(crate) use nichlink_run_method::mir::CallTreeNode;
pub(crate) use nichlink_run_method::mir::{CallRelation, call_tree};
