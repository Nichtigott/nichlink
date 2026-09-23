//! Ungrouped Studio state: reload errors, overlays, and call references.
//! 未归类的 Studio 状态：重载错误、浮层与调用引用。

use std::path::PathBuf;

use super::super::support::package_root;
use super::forms::{AddState, NewProjectState, PluginState};
use super::graft::GraftState;
use super::search::SearchState;
use nichlink_run_method::{NodeId, Registry};

/// Structured error retained after a failed hot reload.
/// 热重载失败后保留的结构化错误。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReloadError {
    /// Reload stage that failed, such as `"reload"` or `"hot_reload"`.
    /// 失败的重载阶段，例如 `"reload"` 或 `"hot_reload"`。
    pub phase: &'static str,
    /// Human-readable failure text from the loader.
    /// 加载器给出的可读失败信息。
    pub message: String,
}

/// Modal screen currently covering the base workspace.
/// 当前覆盖基础工作区的模态界面。
#[derive(Clone, Debug)]
pub enum Overlay {
    /// Search results, optionally shown as a provenance graph.
    /// 搜索结果，可选以溯源图显示。
    Search(SearchState),
    /// New-project wizard.
    /// 新建项目向导。
    NewProject(NewProjectState),
    /// Add a registration face under the current parent.
    /// 在当前父注册面下添加注册面。
    Add(AddState),
    /// Edit the fields of one existing face.
    /// 编辑某个已有注册面的字段。
    Edit(NodeId, AddState),
    /// Plugin selection form.
    /// 插件选择表单。
    Plugin(PluginState),
    /// External graft authoring screen.
    /// 外部 graft 创作界面。
    Graft(GraftState),
    /// Delete confirmation for one node.
    /// 针对某个节点的删除确认。
    Delete(NodeId),
}

/// One reference to a function at a node, used for call navigation.
/// 指向某个节点上函数的引用，用于调用跳转。
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallRef {
    /// Node that declares the function.
    /// 声明该函数的节点。
    pub node: NodeId,
    /// Function name as resolved for this reference.
    /// 该引用解析出的函数名。
    pub function: String,
    /// Source file the function lives in, empty when unknown.
    /// 函数所在的源文件；未知时为空。
    pub file: String,
}

pub(crate) fn push_call_ref(
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
