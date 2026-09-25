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
/// 解析 Studio 读取与编辑其源码的项目。
///
/// The rule is `lexicon`'s, plus one refusal it cannot make: Studio is an
/// editor, so "no project" has to be an error rather than a path to write into.
/// 规则来自 `lexicon`，外加一条它无法做出的拒绝：Studio 是编辑器，因此"没有项目"必须是
/// 错误，而不是一个可以往里写的路径。
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
    resolve_project_from(
        None,
        None,
        configured.as_deref(),
        current.as_deref(),
        current
            .as_ref()
            .is_some_and(|directory| directory.join("Cargo.toml").is_file()),
    )
    // `launch` already refused an unresolvable project before the editor started,
    // so this branch is unreachable in a running session. The working directory is
    // the last resort rather than the directory this crate was compiled in, which
    // is what a reader would otherwise be editing: the checkout, or the installed
    // crate's sources.
    // `launch` 已在编辑器启动前拒绝了解析不出的项目，因此这个分支在运行中的会话里不可达。
    // 兜底取当前目录，而不是本 crate 编译时所在的目录——否则读者编辑的会是检出目录或已安装
    // crate 的源码。
    .unwrap_or_else(|_| current.unwrap_or_else(|| PathBuf::from(".")))
}

/// Resolve the project Studio should open, or say why it cannot.
/// 解析 Studio 应当打开的项目，或说明为什么不能。
pub(super) fn resolve_project(explicit: Option<&Path>) -> Result<PathBuf, String> {
    let configured =
        std::env::var_os(nichlink_run_method::lexicon::PACKAGE_ROOT_ENV).map(PathBuf::from);
    let current = std::env::current_dir().ok();
    resolve_project_from(
        PROJECT_CONTEXT
            .with(|session| {
                session
                    .borrow()
                    .as_ref()
                    .map(|project| project.root.clone())
            })
            .as_deref(),
        explicit,
        configured.as_deref(),
        current.as_deref(),
        current
            .as_ref()
            .is_some_and(|directory| directory.join("Cargo.toml").is_file()),
    )
}

/// The resolution rule itself, with every input a parameter so it can be pinned.
/// 解析规则本身；每个输入都是参数，因此可以被钉住。
///
/// Order: the session's own selection, an explicit path, the environment, then
/// the working directory when it holds a package. Every value that names
/// something unusable is refused by name instead of being used or skipped.
/// 顺序：本会话自己的选择、显式路径、环境变量，最后是持有包的当前目录。任何指不到东西的
/// 取值都按名字被拒绝，而不是被使用或被跳过。
pub(super) fn resolve_project_from(
    selected: Option<&Path>,
    explicit: Option<&Path>,
    configured: Option<&Path>,
    current: Option<&Path>,
    current_holds_package: bool,
) -> Result<PathBuf, String> {
    if let Some(path) = selected {
        return usable(path, current, "the selected project", "select_project");
    }
    if let Some(path) = explicit {
        return usable(path, current, "the path argument", "nichlink-studio <path>");
    }
    if let Some(path) = configured {
        return usable(
            path,
            current,
            nichlink_run_method::lexicon::PACKAGE_ROOT_ENV,
            "NICH_LINK_PACKAGE_ROOT",
        );
    }
    if current_holds_package && let Some(current) = current {
        return Ok(current.to_path_buf());
    }
    Err(
        "no project to open: pass a project path, set NICH_LINK_PACKAGE_ROOT, or start Studio \
         from a directory that holds a Cargo.toml"
            .to_owned(),
    )
}

/// Accept one candidate root, or refuse it by name.
/// 接受一个候选根，或按名字拒绝它。
fn usable(
    path: &Path,
    current: Option<&Path>,
    what: &str,
    remedy: &str,
) -> Result<PathBuf, String> {
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        current.unwrap_or_else(|| Path::new(".")).join(path)
    };
    if resolved.is_dir() {
        return Ok(resolved);
    }
    Err(format!(
        "{what} names `{}`, which is not a directory; check {remedy}",
        resolved.display()
    ))
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
            && column.abs_diff(self.hot.graph_tree_area.right()) <= 1
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

/// The node-editor fixture host, when this checkout has it.
/// node-editor 夹具宿主，当本检出有它时。
///
/// The fixture is a nested package, and cargo does not put a nested package into a
/// published `.crate`: a consumer who runs `cargo test --all-features` on the published
/// `nichlink-studio` has no fixture to index. The contract `prototype-fixtures` names is
/// a *checkout* contract — `AGENTS.md` says the fixture is not part of the published
/// surface — so the tests that need it skip when it is absent instead of panicking. It
/// is always present in this checkout, so they always run here.
/// 该夹具是嵌套包，而 cargo 不会把嵌套包放进发布的 `.crate`：在已发布
/// `nichlink-studio` 上跑 `cargo test --all-features` 的消费者没有夹具可索引。
/// `prototype-fixtures` 命名的契约是**检出**契约——`AGENTS.md` 说该夹具不属于发布面——
/// 因此需要它的测试在夹具缺席时跳过而不是 panic。本检出里它始终存在，因此这些测试始终运行。
#[cfg(all(test, feature = "prototype-fixtures"))]
pub(super) fn node_editor_fixture() -> Option<std::path::PathBuf> {
    let root =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/node-editor");
    if root.join("Cargo.toml").is_file() {
        Some(root)
    } else {
        eprintln!(
            "skipping: the node-editor fixture is a checkout fixture and is not in this package"
        );
        None
    }
}
