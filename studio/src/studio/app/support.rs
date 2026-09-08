//! Shared interaction geometry and editor helpers.
//! 交互几何与编辑器辅助。

use std::cell::RefCell;
use std::path::{Path, PathBuf};

const NICHLINK_REPOSITORY: &str = "https://github.com/Nichtigott/nichlink";

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

pub(super) fn package_namespace() -> String {
    PROJECT_CONTEXT
        .with(|current| {
            current
                .borrow()
                .as_ref()
                .map(|project| project.namespace.clone())
        })
        .or_else(|| std::env::var("NICH_LINK_NAMESPACE").ok())
        .unwrap_or_else(|| "nichlink.default".to_owned())
}

pub(super) fn with_authoring_context<T>(operation: impl FnOnce() -> T) -> T {
    nichlink::AuthoringContext::new(package_root(), package_namespace()).scope(operation)
}

/// Resolve the project whose sources Studio reads and edits.
pub(super) fn package_root() -> PathBuf {
    if let Some(root) = PROJECT_CONTEXT.with(|current| {
        current
            .borrow()
            .as_ref()
            .map(|project| project.root.clone())
    }) {
        return root;
    }
    if let Some(configured) = std::env::var_os("NICH_LINK_PACKAGE_ROOT") {
        let path = PathBuf::from(configured);
        if path.is_absolute() {
            return path;
        }
        return std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path);
    }
    // When Studio is launched from a host project, that project is the natural
    // target. This keeps `cargo run --manifest-path .../studio/Cargo.toml`
    // useful without requiring an environment variable.
    // 从宿主项目目录启动 Studio 时，当前目录就是默认目标，无需额外环境变量。
    if let Ok(current) = std::env::current_dir()
        && current.join("Cargo.toml").is_file()
    {
        return current;
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_owned()
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

/// Select portable Git dependencies for installed Studio binaries. Path
/// dependencies are reserved for a binary running from this checkout's own
/// target directory; Cargo's Git cache also contains sibling crates, but must
/// never leak into a generated manifest.
pub(super) fn nichlink_dependency_specs(
    studio_manifest: &Path,
    current_exe: &Path,
) -> (String, String) {
    let workspace = studio_manifest.parent().unwrap_or_else(|| Path::new("."));
    let runs_from_workspace = current_exe.starts_with(workspace.join("target"));
    if runs_from_workspace && workspace.join("core").is_dir() && workspace.join("build").is_dir() {
        let core = toml_path(&workspace.join("core"));
        let build = toml_path(&workspace.join("build"));
        (
            format!("nichlink-core = {{ package = \"nichlink-core\", path = \"{core}\" }}"),
            format!("nichlink-build = {{ path = \"{build}\" }}"),
        )
    } else {
        (
            format!(
                "nichlink-core = {{ package = \"nichlink-core\", git = \"{NICHLINK_REPOSITORY}\", branch = \"main\", version = \"0.1.0\" }}"
            ),
            format!(
                "nichlink-build = {{ git = \"{NICHLINK_REPOSITORY}\", branch = \"main\", version = \"0.1.0\" }}"
            ),
        )
    }
}

fn toml_path(path: &Path) -> String {
    path.display().to_string().replace('\\', "\\\\")
}

use super::*;

impl App {
    pub(super) fn near_divider(&self, column: u16, row: u16) -> bool {
        self.workspace_area.contains((column, row).into())
            && column.abs_diff(self.tree_area.right()) <= 1
    }

    pub(super) fn near_graph_divider(&self, column: u16, row: u16) -> bool {
        self.graph_area.contains((column, row).into())
            && column.abs_diff(self.graph_callees_area.x) <= 1
    }

    pub(super) fn resize_graph_split(&mut self, column: u16) {
        if self.graph_area.width == 0 {
            return;
        }
        let relative = column.saturating_sub(self.graph_area.x) as u32;
        self.graph_split_percent =
            ((relative * 100) / u32::from(self.graph_area.width)).clamp(35, 65) as u16;
    }

    pub(super) fn resize_split(&mut self, column: u16) {
        if self.workspace_area.width == 0 {
            return;
        }
        let relative = column.saturating_sub(self.workspace_area.x) as u32;
        self.split_percent =
            ((relative * 100) / u32::from(self.workspace_area.width)).clamp(25, 70) as u16;
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
    pub(crate) fn graph_locals(&self, item: &CallRef) -> Vec<nichlink::LocalValue> {
        self.runtime_trace
            .locals()
            .iter()
            .filter(|local| {
                self.runtime_trace
                    .path_for_local(nichlink::LocalId(local.id))
                    .iter()
                    .any(|call| call.function == item.function)
            })
            .cloned()
            .collect()
    }
}
