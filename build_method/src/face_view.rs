//! Read-only registration-face view for operator commands.
//! 面向操作命令的只读注册面视图。
//!
//! `nichlink explain` has to answer "what is this node, and why is it (not)
//! shipped" for a host it cannot link. It therefore reads the same discovery
//! tree, the same face parser, and the same parent/identity rules the build
//! uses, and never re-implements them: this module is the one place a command
//! turns a package root into per-face rows. The scope/pruning manifest readers
//! live in `scope_view`, mounted below and re-exported so the public
//! `face_view::…` paths are unchanged.
//! `nichlink explain` 必须为它无法链接的宿主回答"这个节点是什么、为什么（没）被发布"。
//! 因此它读的是构建使用的同一棵发现树、同一个注册面解析器与同一套父级/身份规则，
//! 绝不重新实现：本模块是命令把包根变成逐面行数据的唯一位置。作用域/修剪清单读取方
//! 位于 `scope_view`，在下方挂载并重新导出，因此公开的 `face_view::…` 路径保持不变。
//!
//! Read-only: nothing here writes, and nothing here consults process-wide build
//! state such as a pinned namespace — `package` is passed in, so two fixtures in
//! one process cannot contaminate each other's identities.
//! 只读：这里不写任何东西，也不查询进程级的构建状态（例如被固定的命名空间）——
//! `package` 由调用方传入，因此同一进程里的两个夹具不会污染彼此的身份。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use nichlink_kernel::lexicon;

use super::registry_identity::NodeId;
use super::registry_syntax::{FaceSyntax, ParentSyntax};
use super::{Node, discover_root_reporting, parsed_face, relative_display, source_module_path};

// Split decision: the discovery walk (this page) and the published-manifest
// readers (`scope_view`) change for different reasons — one follows source and
// parent rules, the other a frozen TSV layout — so they are separate files. The
// re-export keeps `crate::face_view::{BuildScopeView, PruningRow,
// read_build_scope, read_pruning_manifest}` and the root whitelist in `lib.rs`
// byte-compatible for every caller.
// 拆分决定：发现遍历（本页）与已发布清单读取方（`scope_view`）因不同原因变化——
// 一个跟随源码与父级规则，另一个跟随冻结的 TSV 版式——因此分成两个文件。重新导出
// 使 `crate::face_view::{BuildScopeView, PruningRow, read_build_scope,
// read_pruning_manifest}` 与 `lib.rs` 的根部白名单对所有调用方逐字节兼容。
#[path = "scope_view.rs"]
mod scope_view;

pub use self::scope_view::{
    BuildScopeView, PruningRow, build_output_is_current, read_build_scope, read_pruning_manifest,
};

/// One registration face as the build's own discovery sees it.
/// 构建自身的发现过程看到的一个注册面。
///
/// `path` is the logical registry path a runtime tree would report, derived from
/// the `registry_name` chain; `source` is relative to the package's `src/`, so a
/// caller can line these rows up with `source_scope.tsv` and
/// `pruning_manifest.tsv`, which use the same spelling.
/// `path` 是运行期树会报告的逻辑注册路径，由 `registry_name` 链推导；`source` 相对包
/// 的 `src/`，因此调用方可以与 `source_scope.tsv`、`pruning_manifest.tsv` 对齐——
/// 它们用的是同一种写法。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaceView {
    /// The face's source-instance identity.
    /// 该面的源码实例身份。
    pub id: NodeId,
    /// The resolved parent registry's identity.
    /// 已解析的父注册机身份。
    pub parent: NodeId,
    /// The declared `kind`, which is one of the identity hash inputs.
    /// 声明的 `kind`，它是身份散列的输入之一。
    pub kind: String,
    /// The registry name under the parent, from the declaration or the macro
    /// default. It is not an overlay slot: `slot` is the graft cut a
    /// `static_graft_plan!` targets, and the plugin host's plugin slot.
    /// 父级下的注册面名，来自声明或宏默认值。它不是叠加层的槽位：`slot` 指
    /// `static_graft_plan!` 针对的 graft 切口，以及插件宿主的插件槽。
    pub registry_name: String,
    /// The Rust module path the source declares.
    /// 源码声明的 Rust 模块路径。
    pub module: String,
    /// Source path relative to the package's `src/`.
    /// 相对包 `src/` 的源码路径。
    pub source: String,
    /// Whether the face owns a child registry.
    /// 该面是否拥有子注册机。
    pub owns_registry: bool,
    /// The logical registry path a runtime tree would report.
    /// 运行期树会报告的逻辑注册路径。
    pub path: String,
    /// Whether the face resolved a parent the current tree knows. A face whose
    /// parent cannot be resolved is still returned, because an operator asking
    /// "what is this node" must get an answer that names the broken parent
    /// instead of a command that refuses to speak.
    /// 该面是否解析到了一个当前树认识的父级。父级解析不出来的面仍会返回，因为操作者
    /// 问"这个节点是什么"时必须得到一个指明坏父级的回答，而不是一条拒绝说话的命令。
    pub parent_resolved: bool,
}

/// A face before its logical path is known, collected so parent lookup can see
/// every face at once.
/// 逻辑路径尚不可知时的注册面；先全部收集，父级查找才能一次看到所有面。
struct RawFace {
    id: NodeId,
    parent: ParentSpec,
    kind: String,
    registry_name: String,
    module: String,
    source: String,
    owns_registry: bool,
}

/// The parent a declaration names, kept unresolved until every face's module is
/// known.
/// 声明命名的父级；在知道每个面的模块之前保持未解析。
enum ParentSpec {
    Root,
    FromPath {
        source: String,
        kind: String,
    },
    NodePath(String),
    /// A `parent` field exists but does not parse; the build reports this as a
    /// `static-plan` diagnostic instead of guessing.
    /// `parent` 字段存在但解析不了；构建对此报 `static-plan` 诊断，而不是猜。
    Unparsed,
}

/// Every registration face under `root`'s `src/`, with the identities a build
/// with namespace `package` would compute.
/// `root` 的 `src/` 下每个注册面，身份使用命名空间 `package` 的构建会算出的值。
///
/// This is [`face_views_and_unreadable`] with the second half dropped. A caller
/// that has to say "there is a registration file I could not read" — rather than
/// answer with fewer faces and no reason — calls that one.
/// 这是 [`face_views_and_unreadable`] 丢掉后半段的结果。必须说出"有一个注册面文件我读不了"、
/// 而不是少报几个面却不说原因的调用方，应当调用那一个。
pub fn face_views(root: &Path, package: &str) -> Result<Vec<FaceView>, String> {
    Ok(face_views_and_unreadable(root, package)?.0)
}

/// Every registration face under `root`'s `src/`, plus every registration file
/// the walk could not place.
/// `root` 的 `src/` 下每个注册面，外加遍历无法安放的每个注册面文件。
///
/// The second half is why this exists. A registration file that does not parse
/// (two macro invocations, a truncated field list) is not a face, so it cannot
/// appear in the first half — and until this entry existed, dropping it was
/// silent: the caller answered "this package declares what I could list", with no
/// count and no reason, while the build reported the same file as a `face-syntax`
/// diagnostic. A file that *is* a face but sits outside the `<name>/<name>.rs`
/// layout is reported the same way, because the view drops it for the same
/// reason. Each string is `"<path relative to the package root>:<line> <reason>"`;
/// the line is `0` when the failure is about the file's place rather than its
/// text.
/// 后半段正是它存在的原因。解析不了的注册面文件（两处宏调用、字段列表被截断）不是面，因此不可能
/// 出现在前半段——而在这个入口存在之前，丢弃它是静默的：调用方回答"这个包声明的就是我能列出的
/// 这些"，没有计数也没有原因，而构建把同一个文件报成 `face-syntax` 诊断。确实是面、却长在
/// `<name>/<name>.rs` 布局之外的文件同样被报出，因为视图丢弃它的理由是同一个。每条字符串形如
/// `"<相对包根的路径>:<行号> <原因>"`；当失败关乎文件的位置而不是它的文本时，行号为 `0`。
///
/// A child whose parent file is in the second half is reported with
/// `parent_resolved == false` in the first half, which is the honest answer: the
/// chain really is broken. Its `path` still falls back to `root/<name>`, because
/// that is the only name the view has for it; a caller that must not print a
/// path the runtime never had has to check `parent_resolved` (and the second half)
/// before using it.
/// 父面文件落在后半段的子面，在前半段里以 `parent_resolved == false` 出现，这是诚实的答案：
/// 那条链确实是断的。它的 `path` 仍回退成 `root/<name>`，因为那是视图手里唯一的名字；绝不能
/// 打印运行期从未有过的路径的调用方，必须先用 `parent_resolved`（以及后半段）核对再使用它。
pub fn face_views_and_unreadable(
    root: &Path,
    package: &str,
) -> Result<(Vec<FaceView>, Vec<String>), String> {
    // The build's own layout resolution, so this read-only view reports the paths
    // and identities the build computes — including for a library target outside
    // `src/`, where the identity path keeps its leading directory component.
    // 用构建自己的布局解析，因此这个只读视图报告的路径与身份就是构建计算出的那些——包括 `src/`
    // 之外的库目标，那种情况下身份路径保留它开头的目录分量。
    let layout = super::source_layout(root)?;
    let src = &layout.scan_root;
    if !src.is_dir() {
        return Err(format!("no source tree at {}", src.display()));
    }
    // `discover_root_reporting` is the same walk the build runs, and it already
    // collects the files it cannot place; `discover_root` used to throw that list
    // away here, which is what made the drop silent.
    // `discover_root_reporting` 就是构建走的那趟遍历，它本来就收集安放不了的文件；
    // 这里过去用 `discover_root` 把那张清单扔掉了，这正是丢弃变静默的原因。
    let mut unplaced = Vec::new();
    let nodes = discover_root_reporting(src, &mut unplaced);
    // A file that *is* the attached `<name>/<name>.rs` of a node is not "unplaced":
    // the walk placed it, and the build reports it through the same syntax rule it
    // applies to every node file. Both classes have to be in the answer, or a
    // broken face file at the expected path would still vanish without a word.
    // 确实是某个节点所附的 `<name>/<name>.rs` 的文件不属于"安放不了"：遍历已给它安排了位置，而
    // 构建经它对每个节点文件都用的同一条语法规则报告它。两类都必须进这个答案，否则长在预期位置上的
    // 坏面文件仍会一声不响地消失。
    let mut unreadable = unplaced
        .iter()
        .map(|face| format!("{}:{} {}", face.relative, face.line, face.message))
        .collect::<Vec<_>>();
    unreadable.extend(
        super::face_syntax_check::face_syntax_errors(src, &nodes)
            .iter()
            .map(|error| format!("{}:{} {}", error.source, error.line, error.message)),
    );
    let identity = &layout.identity_base;
    let mut raw = Vec::new();
    collect(identity, &nodes, package, &mut raw);
    let modules = raw
        .iter()
        .map(|face| (face.module.clone(), face.id))
        .collect::<BTreeMap<_, _>>();
    let resolved = raw
        .into_iter()
        .map(|face| {
            let (parent, parent_resolved) = resolve_parent(&face.parent, package, &modules);
            (face, parent, parent_resolved)
        })
        .collect::<Vec<_>>();
    let mut paths = BTreeMap::new();
    paths.insert(root_node_id(package), "root".to_owned());
    let names = resolved
        .iter()
        .map(|(face, parent, _)| (face.id, *parent, face.registry_name.clone()))
        .collect::<Vec<_>>();
    let mut walking = BTreeSet::new();
    let mut views = resolved
        .into_iter()
        .map(|(face, parent, parent_resolved)| FaceView {
            id: face.id,
            parent,
            kind: face.kind,
            registry_name: face.registry_name,
            module: face.module,
            source: face.source,
            owns_registry: face.owns_registry,
            path: logical_path(face.id, &names, &mut paths, &mut walking),
            parent_resolved,
        })
        .collect::<Vec<_>>();
    views.sort_by(|left, right| (&left.path, left.id).cmp(&(&right.path, right.id)));
    Ok((views, unreadable))
}

/// Resolve one collected parent against the module-to-face map, matching the
/// build's three parent forms.
/// 用"模块 → 面"映射解析一个已收集的父级，对应构建的三种父级形式。
fn resolve_parent(
    parent: &ParentSpec,
    package: &str,
    modules: &BTreeMap<String, NodeId>,
) -> (NodeId, bool) {
    match parent {
        ParentSpec::Root => (root_node_id(package), true),
        ParentSpec::FromPath { source, kind } => {
            (NodeId::from_namespaced_path(package, source, kind), true)
        }
        ParentSpec::NodePath(module) => {
            // The build strips the `crate::` prefix and maps the module back to
            // the face that owns it (`cached_parent_id`). A module the tree does
            // not own stays unresolved so the operator sees the broken chain.
            // 构建剥掉 `crate::` 前缀，把模块映射回拥有它的面（`cached_parent_id`）。
            // 树不拥有的模块保持未解析，操作者因此看到断链。
            let key = module.strip_prefix("crate::").unwrap_or(module);
            match modules.get(key) {
                Some(id) => (*id, true),
                None => (root_node_id(package), false),
            }
        }
        ParentSpec::Unparsed => (root_node_id(package), false),
    }
}

/// The root identity for one package, matching the build's own helper.
/// 某个包的根身份，与构建自己的辅助函数一致。
fn root_node_id(package: &str) -> NodeId {
    NodeId::from_namespaced_path(package, "<root>", "root")
}

fn collect(src: &Path, nodes: &[Node], package: &str, faces: &mut Vec<RawFace>) {
    for node in nodes {
        if let Some(file) = &node.file {
            let relative = relative_display(src, file);
            if !lexicon::is_registration_path(&relative)
                && let Ok(source) = fs::read_to_string(file)
                && let Some(face) = parsed_face(&source, &relative)
                && let Some(kind) = face.path("kind")
            {
                let module = source_module_path(&relative);
                let registry_name = face.path("registry_name").unwrap_or_else(|| {
                    // The macro default, not a new rule: a face that omits
                    // `registry_name` takes its module's last segment
                    // (`__face_string_or!(last_path_segment(module_path!()))` in
                    // `run_method/src/macros/face_objects.rs`). A command that
                    // guessed anything else would print a logical path the
                    // runtime never had.
                    // 这是宏的默认值，不是新规则：省略 `registry_name` 的面取模块名末段
                    // （`run_method/src/macros/face_objects.rs` 中的
                    // `__face_string_or!(last_path_segment(module_path!()))`）。命令若
                    // 猜成别的，就会打印出运行期从未有过的逻辑路径。
                    module.rsplit("::").next().unwrap_or(&module).to_owned()
                });
                faces.push(RawFace {
                    id: NodeId::from_namespaced_path(package, &relative, &kind),
                    parent: parent_of(&face),
                    kind,
                    registry_name,
                    module,
                    source: relative,
                    owns_registry: face.boolean("needs_registry").unwrap_or(false),
                });
            }
        }
        collect(src, &node.children, package, faces);
    }
}

/// Read one face's parent declaration without resolving it, exactly the way the
/// build decides "root or declared".
/// 读取一个面的父级声明而不解析它，判断方式与构建"根还是显式声明"完全一致。
///
/// No `parent` field means the package root. A `parent` field that parses to a
/// module or a source/kind pair is kept as data; one that does not parse is
/// `Unparsed` and becomes a reported unresolved parent rather than a guess. The
/// build turns the same case into a `static-plan` diagnostic.
/// 没有 `parent` 字段即包根。解析成模块或 source/kind 对的 `parent` 字段按数据保留；
/// 解析不了的记为 `Unparsed`，成为被上报的未解析父级而不是猜测。构建对同一情形报的
/// 是 `static-plan` 诊断。
fn parent_of(face: &FaceSyntax) -> ParentSpec {
    if face.field("parent").is_none() {
        return ParentSpec::Root;
    }
    match face.parent() {
        Some(ParentSyntax::Root) => ParentSpec::Root,
        Some(ParentSyntax::FromPath { source, kind }) => ParentSpec::FromPath { source, kind },
        Some(ParentSyntax::NodePath(module)) => ParentSpec::NodePath(module),
        None => ParentSpec::Unparsed,
    }
}

/// The logical registry path of one face, walking the resolved parent chain.
/// 一个面的逻辑注册路径，沿已解析的父级链行走。
///
/// The parent's own path is resolved before this one is formed, so the order the
/// discovery walk happened to visit the nodes in is not an input to the answer.
/// The map still caches a finished chain, because a tree with many faces shares
/// chains and each one is then walked once.
/// 父面自己的路径先生成，再拼出本面，因此发现遍历恰好按什么顺序访问节点不是这个答案的输入。
/// 映射仍缓存已完成的链：面很多的树会共用链，而每条链只走一次。
///
/// Two walks stop at `root` instead of inventing a prefix: a parent that the face
/// set does not contain (the unresolved-parent case `parent_resolved` reports),
/// and a chain that comes back to a face already being resolved. The second is a
/// cycle, which no runtime tree can have — the registry rejects it — but a
/// read-only view of a tree the build would refuse must terminate rather than
/// recurse until the stack runs out.
/// 两种走法会停在 `root` 而不是编造前缀：面集合里不存在的父级（也就是 `parent_resolved`
/// 报告的那种未解析父级），以及回到一个正在解析中的面的链。后者是环，运行期的树不可能有——
/// 注册机会拒绝它——但"构建会拒绝的树"的只读视图必须能终止，而不是递归到栈耗尽。
fn logical_path(
    id: NodeId,
    names: &[(NodeId, NodeId, String)],
    paths: &mut BTreeMap<NodeId, String>,
    walking: &mut BTreeSet<NodeId>,
) -> String {
    if let Some(path) = paths.get(&id) {
        return path.clone();
    }
    let Some((_, parent, name)) = names.iter().find(|(candidate, _, _)| *candidate == id) else {
        return "root".to_owned();
    };
    let parent_path = if !names.iter().any(|(candidate, _, _)| candidate == parent) {
        "root".to_owned()
    } else if walking.insert(id) {
        let path = logical_path(*parent, names, paths, walking);
        walking.remove(&id);
        path
    } else {
        "root".to_owned()
    };
    let path = format!("{parent_path}/{name}");
    paths.insert(id, path.clone());
    path
}

#[cfg(test)]
#[path = "face_view_tests.rs"]
mod face_view_tests;
