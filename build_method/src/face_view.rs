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

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use nichlink::lexicon;

use super::registry_identity::NodeId;
use super::registry_syntax::{FaceSyntax, ParentSyntax};
use super::{Node, discover_root, parsed_face, relative_display, source_module_path};

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

pub use self::scope_view::{BuildScopeView, PruningRow, read_build_scope, read_pruning_manifest};

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
    /// The slot name under the parent, from the declaration or the macro default.
    /// 父级下的槽位名，来自声明或宏默认值。
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
pub fn face_views(root: &Path, package: &str) -> Result<Vec<FaceView>, String> {
    let src = root.join("src");
    if !src.is_dir() {
        return Err(format!("no source tree at {}", src.display()));
    }
    let nodes = discover_root(&src);
    let mut raw = Vec::new();
    collect(&src, &nodes, package, &mut raw);
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
            path: logical_path(face.id, &names, &mut paths),
            parent_resolved,
        })
        .collect::<Vec<_>>();
    views.sort_by(|left, right| (&left.path, left.id).cmp(&(&right.path, right.id)));
    Ok(views)
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
/// The map is shared across faces so a chain is walked once, and a parent the
/// face set does not contain stops at `root` — the same fallback the unresolved
/// parent already reports through `parent_resolved`.
/// 映射在所有面之间共享，因此每条链只走一次；面集合中不存在的父级止于 `root`——
/// 与未解析父级通过 `parent_resolved` 报告的正是同一个回退。
fn logical_path(
    id: NodeId,
    names: &[(NodeId, NodeId, String)],
    paths: &mut BTreeMap<NodeId, String>,
) -> String {
    if let Some(path) = paths.get(&id) {
        return path.clone();
    }
    let Some((_, parent, name)) = names.iter().find(|(candidate, _, _)| *candidate == id) else {
        return "root".to_owned();
    };
    let parent_path = paths
        .get(parent)
        .cloned()
        .unwrap_or_else(|| "root".to_owned());
    let path = format!("{parent_path}/{name}");
    paths.insert(id, path.clone());
    path
}

#[cfg(test)]
mod tests {
    use super::{face_views, read_build_scope, read_pruning_manifest};
    use std::fs;
    use std::path::PathBuf;

    /// A throwaway package root; the caller writes `src/` files into it.
    /// 一次性的包根；调用方往其中写 `src/` 文件。
    fn temporary_root(label: &str) -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "nichlink-face-view-{label}-{}-{}-{sequence}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(root.join("src")).expect("fixture src");
        root
    }

    /// The logical path and the registry name follow the chain a runtime tree
    /// would build, and the default slot name is the module's last segment.
    /// 逻辑路径与 registry 名跟随运行期树会构建的链，默认槽位名取模块末段。
    #[test]
    fn faces_carry_their_logical_path_and_default_slot_name() {
        let root = temporary_root("paths");
        let panel = root.join("src/control/object");
        fs::create_dir_all(&panel).expect("panel directory");
        fs::write(
            panel.join("object.rs"),
            "crate::root_object! {\n    kind: Control,\n    needs_registry: true,\n}\n",
        )
        .expect("panel face");
        let button = root.join("src/control/object/button");
        fs::create_dir_all(&button).expect("button directory");
        fs::write(
            button.join("button.rs"),
            "crate::root_object! {\n    kind: Button,\n    parent: crate::control::object::NODE_ID,\n}\n",
        )
        .expect("button face");

        let faces = face_views(&root, "host").expect("faces");
        assert_eq!(faces.len(), 2);
        let control = faces
            .iter()
            .find(|face| face.kind == "Control")
            .expect("control face");
        assert_eq!(control.path, "root/object");
        assert!(control.owns_registry);
        assert!(control.parent_resolved);
        let button = faces
            .iter()
            .find(|face| face.kind == "Button")
            .expect("button face");
        assert_eq!(button.registry_name, "button");
        assert!(button.parent_resolved);
        assert!(
            button.id
                == nichlink::identity::NodeId::from_namespaced_path(
                    "host",
                    "control/object/button/button.rs",
                    "Button"
                ),
            "identity must be the namespace + relative path + kind hash"
        );

        fs::remove_dir_all(&root).expect("cleanup");
    }

    /// A parent the tree cannot resolve keeps the face visible and says so,
    /// rather than dropping it or guessing a different parent.
    /// 树解析不出的父级仍让该面可见并如实说明，而不是丢掉它或改猜另一个父级。
    #[test]
    fn an_unresolved_parent_is_reported_not_guessed() {
        let root = temporary_root("unresolved-parent");
        let folder = root.join("src/child");
        fs::create_dir_all(&folder).expect("child directory");
        fs::write(
            folder.join("child.rs"),
            "crate::root_object! {\n    kind: Child,\n    parent: crate::missing::NODE_ID,\n}\n",
        )
        .expect("child face");

        let faces = face_views(&root, "host").expect("faces");
        assert_eq!(faces.len(), 1);
        assert!(!faces[0].parent_resolved);

        fs::remove_dir_all(&root).expect("cleanup");
    }

    /// The two manifest readers agree with what the writers in `manifests.rs`
    /// emit, including the `# result all` spelling for a whole-tree scope.
    /// 两个清单读取方与 `manifests.rs` 的写入方一致，包括整树作用域的 `# result all`
    /// 写法。
    #[test]
    fn manifests_round_trip_through_the_readers() {
        let root = temporary_root("manifests");
        let out = root.join("out");
        fs::create_dir_all(&out).expect("out dir");
        fs::write(
            out.join("source_scope.tsv"),
            "# mode\tauto\n# selected\t1\n# node\tsource\tmodule\nabc\tcontrol/button.rs\tcontrol::button\n",
        )
        .expect("scope manifest");
        let scope = read_build_scope(&out).expect("scope");
        assert_eq!(scope.mode, "auto");
        assert!(!scope.all);
        assert_eq!(scope.selected_sources.len(), 1);
        assert!(scope.selected_sources.contains("control/button.rs"));
        assert!(scope.keeps("control::button"), "a selected face is kept");
        assert!(
            scope.keeps("control"),
            "a selected face keeps its parent registry"
        );
        assert!(
            !scope.keeps("control_extra"),
            "the boundary is the `::` segment, not a text prefix"
        );

        fs::write(
            out.join("source_scope.tsv"),
            "# mode\texplicit\n# result\tall\n# selected\tall\n# reason\tscope-all\n",
        )
        .expect("all scope");
        let scope = read_build_scope(&out).expect("all scope");
        assert!(scope.all);
        assert_eq!(scope.reason.as_deref(), Some("scope-all"));

        fs::write(
            out.join("pruning_manifest.tsv"),
            "# node\tsource\tsymbol\n00000000000000000000000000000000\tcontrol/button.rs\t-\n",
        )
        .expect("pruning manifest");
        let rows = read_pruning_manifest(&out).expect("pruning rows");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source, "control/button.rs");
        assert_eq!(rows[0].symbol, "-");

        assert!(read_build_scope(&root.join("missing")).is_err());
        fs::remove_dir_all(&root).expect("cleanup");
    }
}
