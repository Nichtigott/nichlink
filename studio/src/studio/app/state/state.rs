//! Studio focus, overlays, and form state.
//! Studio 焦点、浮层和表单状态。

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::support::package_root;
use nichlink::{FACE_FIELD_COUNT, FACE_PRIMARY_FIELDS, NodeId, Registry};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Focus {
    Tree,
    Details,
}

/// Top-level Studio workspace selected by the user.
/// Studio 用户当前选择的顶层工作区。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StudioPage {
    Search,
    Inspect,
    Data,
    Compare,
}

impl StudioPage {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Search => "SEARCH",
            Self::Inspect => "INSPECT",
            Self::Data => "DATA",
            Self::Compare => "COMPARE",
        }
    }
}

/// Structured error retained after a failed hot reload.
/// 热重载失败后保留的结构化错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReloadError {
    pub phase: &'static str,
    pub message: String,
}

#[derive(Clone, Debug)]
pub enum Overlay {
    Search(SearchState),
    NewProject(NewProjectState),
    Add(AddState),
    Edit(NodeId, AddState),
    Plugin(PluginState),
    Delete(NodeId),
}

/// Fields used by the New Project wizard.
/// New Project 向导使用的字段。
#[derive(Clone, Debug)]
pub struct NewProjectState {
    pub values: [String; 3],
    pub field: usize,
    pub editing: bool,
}

impl NewProjectState {
    pub(super) fn new() -> Self {
        Self {
            values: [
                "./nichlink-app".to_owned(),
                "nichlink-app".to_owned(),
                "binary".to_owned(),
            ],
            field: 0,
            editing: false,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SearchState {
    pub query: String,
    pub selected: usize,
    pub folded: BTreeSet<usize>,
    pub offset: usize,
    pub compare_query: Option<String>,
    pub compare_selected: usize,
    pub compare_offset: usize,
    pub active_pane: usize,
    /// When true, the search result is shown as a navigable provenance graph.
    /// 为 true 时，搜索结果显示为可导航的溯源图。
    pub graph_mode: bool,
    pub center: Option<NodeId>,
    pub center_function: Option<String>,
    pub center_line: Option<u32>,
    pub graph_selected: usize,
    pub outline_selected: usize,
    pub outline_focus: bool,
    /// Focused column on the four-column graph page: A, B, call tree, data.
    /// 四列调用页当前焦点：A、B、调用树、数据流。
    pub graph_focus: usize,
    pub graph_side: usize,
    pub data_selected: usize,
    pub compare_center: Option<NodeId>,
    pub compare_center_function: Option<String>,
    pub compare_center_line: Option<u32>,
    pub compare_graph_selected: usize,
    pub compare_outline_selected: usize,
    pub compare_data_selected: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SearchRow {
    pub source_index: usize,
    pub depth: usize,
    pub has_children: bool,
    pub node: Option<NodeId>,
    pub path: String,
    pub function: String,
    pub line: Option<u32>,
    pub signature: String,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallRef {
    pub node: NodeId,
    pub function: String,
    pub file: String,
}

pub(super) fn mir_name_matches(symbol: &str, function: &str) -> bool {
    symbol == function || symbol.ends_with(&format!("::{function}"))
}

pub(super) fn push_call_ref(
    registry: &Registry,
    output: &mut Vec<CallRef>,
    node: NodeId,
    function: impl Into<String>,
) {
    let function = function.into();
    if output
        .iter()
        .any(|item| item.node == node && item.function == function)
    {
        return;
    }
    output.push(CallRef {
        node,
        function,
        file: registry
            .find(node)
            .map(|info| info.source.file.as_str())
            .unwrap_or("")
            .to_owned(),
    });
}

/// Resolve a registration source to an actual file on disk.
/// 将注册面路径解析为磁盘上的真实文件。
///
/// Core faces are stored relative to `src`; Studio and external faces carry
/// their package-relative prefix. Older directory-style faces are normalized
/// to the directory's attached `<name>.rs` file before an editor is launched.
/// 核心注册面相对于 `src` 保存；Studio 和外部注册面带有包路径前缀。
/// 旧的目录型注册面会在打开编辑器前补成目录下的 `<name>.rs` 文件。
pub(crate) fn source_path_for(file: &str) -> PathBuf {
    let package_root = package_root();
    let relative = std::path::Path::new(file.trim_end_matches('/'));
    let path = if relative.starts_with("src")
        || relative.starts_with("studio")
        || relative.starts_with(".nichlink")
    {
        package_root.join(relative)
    } else {
        package_root.join("src").join(relative)
    };
    if path.is_dir()
        && let Some(name) = path.file_name().and_then(|name| name.to_str())
    {
        let attached = path.join(format!("{name}.rs"));
        if attached.is_file() {
            return attached;
        }
    }
    path
}

#[derive(Clone, Debug)]
pub struct AddState {
    pub values: [String; FACE_FIELD_COUNT],
    pub field: usize,
    pub editing: bool,
    pub advanced: bool,
}

pub(crate) fn face_field_indices(add: &AddState) -> Vec<usize> {
    if add.advanced || !FACE_PRIMARY_FIELDS.contains(&add.field) {
        (0..FACE_FIELD_COUNT).collect()
    } else {
        FACE_PRIMARY_FIELDS.to_vec()
    }
}

/// Plugin selection form. The UI writes one crate anchor, never an object list.
/// 插件选择表单。界面只写入一个 crate 锚点，不维护对象清单。
#[derive(Clone, Debug)]
pub struct PluginState {
    pub values: [String; 7],
    pub field: usize,
    pub editing: bool,
}

impl PluginState {
    pub(super) fn new() -> Self {
        Self {
            values: [
                "official".to_owned(),
                "nichlink.default".to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "extension".to_owned(),
            ],
            field: 0,
            editing: false,
        }
    }
}

impl AddState {
    pub(super) fn new(parent: NodeId) -> Self {
        Self {
            values: [
                parent.to_string(),
                String::new(),
                "false".to_owned(),
                String::new(),
                "ANY".to_owned(),
                "ANY".to_owned(),
                "NoParts".to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                // preset, params, handle, stable name, external registry,
                // rule path, handle traits, handle contracts, part traits,
                // requires, provides
                "NoPreset".to_owned(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                "()".to_owned(),
                "()".to_owned(),
                // runtime checks, explicit flow, flow provider
                String::new(),
                String::new(),
                String::new(),
            ],
            field: 0,
            editing: false,
            advanced: false,
        }
    }
}
