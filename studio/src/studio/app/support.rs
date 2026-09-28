//! Shared Studio support: project context and manifests, the `cargo` probe, and the
//! interaction geometry plus editor handoff the UI drives.
//! Studio 共享支撑：项目上下文与清单、`cargo` 探测，以及界面驱动的交互几何与编辑器交接。
//!
//! Three unrelated subjects share this module, and this line is the honest map of
//! them: resolving the selected project from the session/environment/manifest,
//! asking `cargo` for a MIR snapshot or a host's binary targets, and the pure TUI
//! helpers (divider geometry, selection movement, editor handoff). Splitting them is
//! tracked as the naming findings `NAM-01`/`STU-S-15`; until then a reader importing
//! this module should know it also brings in `std::process` and `std::env`.
//! 三个互不相干的主语共用本模块，这一行就是它们诚实的分布图：从会话/环境/清单解析已选项目、
//! 向 `cargo` 要 MIR 快照或宿主的二进制 target，以及纯 TUI 辅助（分隔条几何、选中移动、编辑器
//! 交接）。拆分它们由命名条目 `NAM-01`/`STU-S-15` 跟踪；在那之前，import 本模块的读者就知道它
//! 也带进了 `std::process` 与 `std::env`。

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use super::namespace::{manifest_for, namespace_for};

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
/// 把本 Studio 会话切换到一个项目，且不改动进程级环境变量。文件监听器使用线程时这依然安全。
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
/// A launched session always has a selected project, because `resolve_project`
/// adopts one before the terminal is taken over; its namespace came from the
/// project's own manifest, so the environment branch below is what an
/// *unselected* session (a library caller, a test) still gets.
/// 已启动的会话总有选中的项目，因为 `resolve_project` 在接管终端之前就采纳了一个；它的命名空间
/// 来自项目自己的清单，因此下面的环境分支是**未选中**项目的会话（库调用方、测试）仍然会走的。
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

/// Run one **read** in an authoring context, resolving the project if the session
/// has not adopted one.
/// 在一个创作上下文里执行一次**读取**；会话尚未采纳项目时按规则解析它。
pub(super) fn with_authoring_context<T>(operation: impl FnOnce() -> T) -> T {
    nichlink_run_method::AuthoringContext::new(package_root(), package_namespace()).scope(operation)
}

/// The selected project's root, when the session has adopted one.
/// 已选中项目的根目录（会话采纳了项目时）。
///
/// The write guards in `super::writers` are built on this, and they are the only
/// callers that need the distinction: `package_root` deliberately falls back, and
/// a writer must not.
/// `super::writers` 里的写入守卫建立在它之上，而它们是唯一需要这个区分的调用方：
/// `package_root` 有意回落，而写入方不得回落。
pub(super) fn selected_root() -> Option<PathBuf> {
    PROJECT_CONTEXT.with(|current| {
        current
            .borrow()
            .as_ref()
            .map(|project| project.root.clone())
    })
}

/// Forget the selected project.
/// 忘掉已选中的项目。
///
/// Test-only, and the reason it exists is the thread-local: a test harness reuses
/// threads, so a test that needs the refusal cannot assume this thread has no
/// selection — it clears one first.
/// 仅测试用，它存在的理由是线程局部：测试框架会复用线程，因此需要这条拒绝的测试不能假设本线程
/// 没有选择——它先清掉一个。
#[cfg(test)]
pub(super) fn clear_project_context() {
    PROJECT_CONTEXT.with(|current| *current.borrow_mut() = None);
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
///
/// Resolving and adopting are one step: the caller gets the root, and the session
/// starts with that root, its manifest, and the identity namespace the manifest
/// names. Splitting them left the namespace at `nichlink.default` for a launched
/// Studio even though the host's own build script stamps
/// `env!("CARGO_PKG_NAME")` — so Studio rebuilt the registration tree in a
/// different identity domain from the one the host compiled, and every recorded
/// `NodeId` (a trace, a graft record) named a node this session could not find.
/// 解析与采纳是同一步：调用方拿到根，会话同时带着该根、它的清单，以及清单写明的身份命名空间启动。
/// 把两者分开会让启动后的 Studio 停在 `nichlink.default`，而宿主自己的构建脚本盖的是
/// `env!("CARGO_PKG_NAME")`——于是 Studio 在一个与宿主编译产物不同的身份域里重建注册树，任何
/// 已记录的 `NodeId`（trace、graft 记录）都指不到本会话能找的节点。
pub(super) fn resolve_project(explicit: Option<&Path>) -> Result<PathBuf, String> {
    let configured =
        std::env::var_os(nichlink_run_method::lexicon::PACKAGE_ROOT_ENV).map(PathBuf::from);
    let current = std::env::current_dir().ok();
    let root = resolve_project_from(
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
    )?;
    let manifest = manifest_for(&root);
    let namespace = namespace_for(
        &manifest,
        std::env::var(nichlink_run_method::lexicon::NAMESPACE_ENV)
            .ok()
            .as_deref(),
    );
    select_project(root.clone(), manifest, namespace);
    Ok(root)
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
/// 解析用于 MIR 检查与重建的 Cargo 清单。
pub(super) fn host_manifest() -> PathBuf {
    if let Some(manifest) = PROJECT_CONTEXT.with(|current| {
        current
            .borrow()
            .as_ref()
            .map(|project| project.manifest.clone())
    }) {
        return manifest;
    }
    manifest_for(&package_root())
}

/// Run `cargo rustc` against the host target that actually exists, passing
/// `rustc_args` through to that target.
/// 针对宿主实际存在的 target 运行 `cargo rustc`，并把 `rustc_args` 透传给该 target。
///
/// MIR inspection passed `--lib` unconditionally, so a binary-only host —
/// including the default output of `nichlink new` — could not be inspected at
/// all. A library target is preferred; when cargo reports that the package has
/// none, its binary targets are tried **one at a time**, named from cargo's own
/// metadata. Cargo decides which targets exist, so a custom `[lib]`/`[[bin]]` path
/// cannot make the answer wrong.
/// MIR 检视此前无条件传 `--lib`，因此仅含二进制的宿主——包括 `nichlink new` 的默认
/// 产物——完全无法被检视。优先选择库 target；当 cargo 报告该包没有库 target 时，改**逐个**
/// 尝试它的二进制 target，名字取自 cargo 自己的 metadata。哪些 target 存在由 cargo 决定，因此
/// 自定义 `[lib]`/`[[bin]]` 路径不会让答案出错。
pub(super) fn cargo_rustc_mir(
    manifest: &Path,
    rustc_args: &[&str],
) -> Result<std::process::Output, String> {
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let invoke = |selector: &[&str]| {
        std::process::Command::new(&cargo)
            .args(["rustc", "--manifest-path"])
            .arg(manifest)
            .args(selector)
            .arg("--quiet")
            .arg("--")
            .args(rustc_args)
            .output()
    };
    let missing_target = |error: std::io::Error| {
        format!("cannot run cargo rustc for {}: {error}", manifest.display())
    };
    let output = invoke(&["--lib"]).map_err(missing_target)?;
    let no_library = String::from_utf8_lossy(&output.stderr).contains("no library targets");
    if output.status.success() || !no_library {
        return Ok(output);
    }
    // `--bins` is not enough for a host with more than one binary target: cargo refuses to hand the
    // extra `rustc` arguments to several targets at once ("extra arguments to `rustc` can only be
    // passed to one target"), so a two-bin host could not be inspected at all (audit `S14`). The
    // names come from `cargo metadata`, and each target is tried in cargo's order; the first that
    // compiles is the answer. The metadata call is a fallback of a fallback: if it fails, the old
    // `--bins` path still runs rather than turning a case that used to work into an error.
    // 对"多于一个二进制 target"的宿主，`--bins` 不够：cargo 拒绝把额外的 `rustc` 参数同时交给多个
    // target（"extra arguments to `rustc` can only be passed to one target"），因此"两个 bin"的宿主
    // 完全无法被检视（审计 `S14`）。名字来自 `cargo metadata`，按 cargo 的顺序逐个尝试，第一个编译
    // 通过的就是答案。这次 metadata 调用是"回退的回退"：它失败时仍走原来的 `--bins` 路径，而不是把
    // 本来能用的情形变成错误。
    let Ok(names) = bin_target_names(&cargo, manifest) else {
        return invoke(&["--bins"]).map_err(missing_target);
    };
    let mut last = None;
    for name in names {
        let output = invoke(&["--bin", name.as_str()]).map_err(missing_target)?;
        if output.status.success() {
            return Ok(output);
        }
        last = Some(output);
    }
    Ok(last.unwrap_or(output))
}

/// The binary target names cargo reports for the package at `manifest`.
/// cargo 为 `manifest` 处的包报告的二进制 target 名字。
///
/// The list comes from `cargo metadata`, not from reading the manifest: a custom `[[bin]]` path
/// stays cargo's business, which is the property [`cargo_rustc_mir`] documents.
/// 这份清单来自 `cargo metadata`，而不是自己读清单：自定义的 `[[bin]]` 路径仍归 cargo 管，这正是
/// [`cargo_rustc_mir`] 所记录的那条性质。
fn bin_target_names(cargo: &std::ffi::OsStr, manifest: &Path) -> Result<Vec<String>, String> {
    let output = std::process::Command::new(cargo)
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .map_err(|error| {
            format!(
                "cannot run cargo metadata for {}: {error}",
                manifest.display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed for {}: {}",
            manifest.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let metadata: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|error| {
        format!(
            "cargo metadata for {} is not JSON: {error}",
            manifest.display()
        )
    })?;
    Ok(metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|package| package["targets"].as_array().into_iter().flatten())
        .filter(|target| {
            target["kind"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
        })
        .filter_map(|target| target["name"].as_str().map(str::to_owned))
        .collect())
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
    ///
    /// The failure is returned as well as written to `event`: `event` is the only
    /// feedback channel this screen has, so a caller that shows a success banner
    /// right afterwards has to be able to keep the failure instead of assigning
    /// over it (audit `LGC-LG-50`).
    /// 失败既写进 `event` 也作为返回值交回：`event` 是本界面唯一的反馈通道，因此紧接着要显示
    /// 成功横幅的调用方必须能保住这条失败，而不是把它覆盖掉（审计 `LGC-LG-50`）。
    pub(super) fn open_editor_file(&mut self, path: PathBuf, line: u32) -> Result<(), String> {
        if !path.is_file() {
            let failure = format!("Editor failed: source file not found at {}", path.display());
            self.event = failure.clone();
            return Err(failure);
        }
        self.editor_request = Some((path, line.max(1)));
        Ok(())
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
