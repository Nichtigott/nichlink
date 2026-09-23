//! Shared interaction geometry and editor helpers.
//! 交互几何与编辑器辅助。

use std::cell::RefCell;
use std::path::{Path, PathBuf};

#[derive(Clone)]
struct ProjectContext {
    root: PathBuf,
    manifest: PathBuf,
    namespace: String,
}

thread_local! {
    static PROJECT_CONTEXT: RefCell<Option<ProjectContext>> = const { RefCell::new(None) };
}

/// Switch this Studio session to a project without mutating process-global
/// environment variables. This remains safe when file watchers use threads.
pub(super) fn select_project(root: PathBuf, manifest: PathBuf, namespace: impl Into<String>) {
    PROJECT_CONTEXT.with(|current| {
        *current.borrow_mut() = Some(ProjectContext {
            root,
            manifest,
            namespace: namespace.into(),
        });
    });
}

/// The namespace Studio authors under: the selected project first, then the
/// environment, then the documented default.
/// Studio 创作所用的命名空间：先选中的项目，再环境变量，最后文档化的默认值。
///
/// Studio used to carry its own copy of this fallback chain and of
/// `package_root`'s; both now come from `lexicon`, so the editor and the
/// executor it calls cannot disagree about which project is open.
/// Studio 此前自带这套回落链与 `package_root` 的副本；两者现在都来自 `lexicon`，因此
/// 编辑器与它调用的执行器不会对"打开的是哪个项目"产生分歧。
pub(super) fn package_namespace() -> String {
    PROJECT_CONTEXT
        .with(|current| {
            current
                .borrow()
                .as_ref()
                .map(|project| project.namespace.clone())
        })
        .unwrap_or_else(|| {
            nichlink_run_method::lexicon::resolve_namespace(
                std::env::var(nichlink_run_method::lexicon::NAMESPACE_ENV)
                    .ok()
                    .as_deref(),
            )
            .to_owned()
        })
}

pub(super) fn with_authoring_context<T>(operation: impl FnOnce() -> T) -> T {
    nichlink_run_method::AuthoringContext::new(package_root(), package_namespace()).scope(operation)
}

/// Resolve the project whose sources Studio reads and edits.
///
/// The rule is `lexicon`'s; only the last-resort fallback is Studio's own.
/// 规则来自 `lexicon`；只有最后兜底属于 Studio 自己。
pub(super) fn package_root() -> PathBuf {
    if let Some(root) = PROJECT_CONTEXT.with(|current| {
        current
            .borrow()
            .as_ref()
            .map(|project| project.root.clone())
    }) {
        return root;
    }
    let configured =
        std::env::var_os(nichlink_run_method::lexicon::PACKAGE_ROOT_ENV).map(PathBuf::from);
    let current = std::env::current_dir().ok();
    // When Studio is launched from a host project, that project is the natural
    // target. This keeps `cargo run --manifest-path .../studio/Cargo.toml`
    // useful without requiring an environment variable.
    // 从宿主项目目录启动 Studio 时，当前目录就是默认目标，无需额外环境变量。
    let fallback = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new("."));
    nichlink_run_method::lexicon::resolve_package_root(
        configured.as_deref(),
        current.as_deref(),
        current
            .as_ref()
            .is_some_and(|directory| directory.join("Cargo.toml").is_file()),
        fallback,
    )
}

/// Resolve the Cargo manifest used for MIR inspection and rebuilds.
pub(super) fn host_manifest() -> PathBuf {
    if let Some(manifest) = PROJECT_CONTEXT.with(|current| {
        current
            .borrow()
            .as_ref()
            .map(|project| project.manifest.clone())
    }) {
        return manifest;
    }
    if let Some(configured) = std::env::var_os("NICH_LINK_HOST_MANIFEST") {
        let path = PathBuf::from(configured);
        let path = if path.is_absolute() {
            path
        } else {
            package_root().join(path)
        };
        return if path.is_dir() {
            path.join("Cargo.toml")
        } else {
            path
        };
    }
    package_root().join("Cargo.toml")
}

use super::*;

impl App {
    pub(super) fn near_divider(&self, column: u16, row: u16) -> bool {
        self.hot.workspace_area.contains((column, row).into())
            && column.abs_diff(self.hot.tree_area.right()) <= 1
    }

    pub(super) fn near_graph_divider(&self, column: u16, row: u16) -> bool {
        self.hot.graph_area.contains((column, row).into())
            && column.abs_diff(self.hot.graph_callees_area.x) <= 1
    }

    pub(super) fn resize_graph_split(&mut self, column: u16) {
        if self.hot.graph_area.width == 0 {
            return;
        }
        let relative = column.saturating_sub(self.hot.graph_area.x) as u32;
        self.graph_split_percent =
            ((relative * 100) / u32::from(self.hot.graph_area.width)).clamp(35, 65) as u16;
    }

    pub(super) fn resize_split(&mut self, column: u16) {
        if self.hot.workspace_area.width == 0 {
            return;
        }
        let relative = column.saturating_sub(self.hot.workspace_area.x) as u32;
        self.split_percent =
            ((relative * 100) / u32::from(self.hot.workspace_area.width)).clamp(25, 70) as u16;
    }

    pub(super) fn move_selection(&mut self, delta: isize) {
        let nodes = self.visible_nodes();
        let current = nodes
            .iter()
            .position(|(id, _)| *id == self.selected)
            .unwrap_or_default();
        let last = nodes.len().saturating_sub(1) as isize;
        let next = (current as isize + delta).clamp(0, last) as usize;
        if let Some((id, _)) = nodes.get(next)
            && self.selected != *id
        {
            self.selected = *id;
            self.details_selected = 0;
        }
    }

    pub(super) fn toggle_selected(&mut self) {
        if !self.owns_registry(self.selected) {
            return;
        }
        if !self.collapsed.insert(self.selected) {
            self.collapsed.remove(&self.selected);
        }
    }

    pub(super) fn selected_parent(&self) -> NodeId {
        self.selected_info()
            .filter(|info| info.needs_registry)
            .map(|info| info.id)
            .or_else(|| {
                self.selected_info()
                    .and_then(|info| self.registry.find(info.parent))
                    .filter(|info| info.needs_registry)
                    .map(|info| info.id)
            })
            .unwrap_or(self.registry.id())
    }

    pub(super) fn open_editor_at(&mut self, node: NodeId, line: Option<u32>) {
        let Some(info) = self.registry.find(node) else {
            return;
        };
        let path = source_path_for(&info.source.file);
        if !path.is_file() {
            self.event = format!("Editor failed: source file not found at {}", path.display());
            return;
        }
        self.editor_request = Some((path, line.unwrap_or(info.source.line)));
    }

    /// Open an arbitrary runtime source location, including a local value.
    /// 打开任意运行时源码位置，包括局部变量位置。
    pub(super) fn open_editor_file(&mut self, path: PathBuf, line: u32) {
        if !path.is_file() {
            self.event = format!("Editor failed: source file not found at {}", path.display());
            return;
        }
        self.editor_request = Some((path, line.max(1)));
    }
}

impl App {
    pub(crate) fn graph_locals(&self, item: &CallRef) -> Vec<nichlink_run_method::LocalValue> {
        self.runtime_trace
            .locals()
            .iter()
            .filter(|local| {
                self.runtime_trace
                    .path_for_local(nichlink_run_method::LocalId(local.id))
                    .iter()
                    .any(|call| call.function == item.function)
            })
            .cloned()
            .collect()
    }
}
