//! The selected project: its root, its manifest, its namespace, and the
//! package-relative source paths derived from them.
//! 已选项目：它的根、它的清单、它的命名空间，以及由它们推出的包内相对源码路径。
//!
//! Nothing here writes; the write-side guard lives in `super::write_guard`.
//! 本模块不做任何写入；写入侧的守卫在 `super::write_guard`。

use std::cell::RefCell;
use std::path::{Path, PathBuf};

use super::namespace::{manifest_for, namespace_for};

/// One Studio session's selected project: its root, its manifest, and the namespace
/// the session authors under.
/// 一个 Studio 会话选中的项目：它的根、它的清单，以及会话撰写所用的命名空间。
#[derive(Clone)]
struct ProjectContext {
    root: PathBuf,
    manifest: PathBuf,
    namespace: String,
}

thread_local! {
    static PROJECT_CONTEXT: RefCell<Option<ProjectContext>> = const { RefCell::new(None) };
}

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
            crate::run_method::lexicon::resolve_namespace(
                std::env::var(crate::run_method::lexicon::NAMESPACE_ENV)
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
    crate::run_method::AuthoringContext::new(package_root(), package_namespace()).scope(operation)
}

/// The selected project's root, when the session has adopted one.
/// 已选中项目的根目录（会话采纳了项目时）。
///
/// The write guards in `super::write_guard` are built on this, and they are the only
/// callers that need the distinction: `package_root` deliberately falls back, and
/// a writer must not.
/// `super::write_guard` 里的写入守卫建立在它之上，而它们是唯一需要这个区分的调用方：
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
        std::env::var_os(crate::run_method::lexicon::PACKAGE_ROOT_ENV).map(PathBuf::from);
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
/// names. Splitting them left the namespace at `xirang.default` for a launched
/// Studio even though the host's own faces compile under the package name
/// (`host!()` defines `crate::XIRANG_NAMESPACE` from it, audit `M7`, P3.3) — so Studio rebuilt the registration tree in a
/// different identity domain from the one the host compiled, and every recorded
/// `NodeId` (a trace, a graft record) named a node this session could not find.
/// 解析与采纳是同一步：调用方拿到根，会话同时带着该根、它的清单，以及清单写明的身份命名空间启动。
/// 把两者分开会让启动后的 Studio 停在 `xirang.default`，而宿主自己的面是用包名编译的
/// （`host!()` 用它定义 `crate::XIRANG_NAMESPACE`，审计 `M7`，P3.3）——于是 Studio 在一个与宿主编译
/// 产物不同的身份域里重建注册树，任何
/// 已记录的 `NodeId`（trace、graft 记录）都指不到本会话能找的节点。
pub(super) fn resolve_project(explicit: Option<&Path>) -> Result<PathBuf, String> {
    let configured =
        std::env::var_os(crate::run_method::lexicon::PACKAGE_ROOT_ENV).map(PathBuf::from);
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
        std::env::var(crate::run_method::lexicon::NAMESPACE_ENV)
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
        return usable(
            path,
            current,
            "the path argument",
            "xirang-toolchain <path>",
        );
    }
    if let Some(path) = configured {
        return usable(
            path,
            current,
            crate::run_method::lexicon::PACKAGE_ROOT_ENV,
            "XIRANG_PACKAGE_ROOT",
        );
    }
    if current_holds_package && let Some(current) = current {
        return Ok(current.to_path_buf());
    }
    Err(
        "no project to open: pass a project path, set XIRANG_PACKAGE_ROOT, or start Studio \
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
        || relative.starts_with(".xirang")
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
