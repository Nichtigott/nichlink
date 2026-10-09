//! Node identity lookup and module-subtree selection for the first-pass scope.
//! 首轮作用域的 node 身份查询与模块子树选择。
//!
//! Identity must be the one discovery and the generated plan use, so the lookup
//! reads the shared discovery cache before it re-parses a face, and the subtree
//! selector walks the same module-matching rule the entry scan uses.
//! 身份必须与发现及生成计划所用的一致，因此查询先读共享发现缓存再重新解析注册面，
//! 子树选择器则沿用入口扫描所用的同一条模块匹配规则。

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use xirang_kernel::lexicon;

use super::discovery_node::{Node, relative_display};
use super::entry::path_mentions_module;
use super::face_syntax_check::parsed_face;
use super::registry_identity::{self, NodeId};
use super::scope_faces::FaceSource;

/// Node identities primed from the discovery cache before scope inference.
/// 作用域推导前由发现缓存预热的 node 身份缓存。
///
/// The cache is process-wide, so an entry is keyed by **namespace and relative
/// source path together**, and a lookup rechecks the identity it finds. One
/// process serves several packages — a bridge or a test binary runs `check_for`
/// for each of them — and two of those packages can hold the same relative
/// source path with different identities. Keying on the path alone handed the
/// second package the first package's `NodeId`: a wrong answer that looked like
/// a right one, because every identity this build publishes is derived from it.
/// 该缓存是进程级的，因此条目的键是**命名空间与相对源码路径的组合**，读取时还会复核找到的身份。
/// 一个进程会服务多个包——桥或测试二进制会为每个包跑 `check_for`——其中两个可能持有相同的相对
/// 源码路径而身份不同。只用路径作键会把第一个包的身份交给第二个包：一个"看起来正确"的错误答案，
/// 因为这次构建发布的每个身份都由它派生。
pub(crate) static CACHED_NODE_IDS: OnceLock<NodeIdCache> = OnceLock::new();

/// Namespace-keyed identities primed from the discovery cache.
/// 由发现缓存预热的、按命名空间分键的身份缓存。
///
/// [`NodeIdCache::get`] is the only read path, and it is where both halves of the
/// fix live; the map itself is deliberately dumb.
/// [`NodeIdCache::get`] 是唯一读取路径，修复的两半都在这里；这张表本身刻意保持简单。
#[derive(Debug, Default)]
pub(crate) struct NodeIdCache {
    entries: BTreeMap<(String, String), (NodeId, String)>,
}

impl NodeIdCache {
    /// Record `relative`'s identity and declared kind for `namespace`.
    /// 记录 `relative` 在 `namespace` 下的身份与声明 kind。
    ///
    /// The kind is kept beside the identity because the read side rechecks the
    /// two against each other; the caller still verifies the pair it inserts.
    /// 同时留下 kind，是因为读取侧要拿它俩互相复核；插入方仍会先验证自己要插入的这对值。
    pub(crate) fn insert(&mut self, namespace: &str, relative: &str, id: NodeId, kind: &str) {
        self.entries.insert(
            (namespace.to_owned(), relative.to_owned()),
            (id, kind.to_owned()),
        );
    }

    /// The identity and declared kind cached for `relative` **in the namespace in
    /// force on this thread**, or `None`.
    /// 本线程当前生效的命名空间下 `relative` 的身份与声明 kind；没有则为 `None`。
    ///
    /// Two conditions, both necessary, both checked here:
    /// 1. the key carries the namespace, so another package's entry is out of
    ///    reach even while it sits in the same process-wide map;
    /// 2. the stored identity must equal the one recomputed from the stored kind
    ///    under the namespace in force, so an entry that entered the cache while
    ///    a different namespace was in force is refused rather than returned.
    ///
    /// 两个条件缺一不可，都在这里检查：
    /// 1. 键带命名空间，因此别的包的条目即使就躺在这张进程级表里也取不到；
    /// 2. 存下的身份必须等于用存下的 kind 在**当前生效命名空间**下现算的身份，因此在别的命名空间
    ///    生效时进缓存的条目会被拒绝而不是被返回。
    pub(crate) fn get(&self, relative: &str) -> Option<&(NodeId, String)> {
        let namespace = registry_identity::package_namespace();
        let entry = self.entries.get(&(namespace, relative.to_owned()))?;
        (entry.0 == registry_identity::package_node_id(relative, &entry.1)).then_some(entry)
    }
}

pub(crate) fn node_id(src: &Path, node: &Node) -> Option<NodeId> {
    let file = node.file.as_ref()?;
    let relative = relative_display(src, file);
    // Registry infrastructure defines the macros and support types; it is not
    // a user registration face and must never enter the face discovery pass.
    // 注册基础设施只提供宏和支撑类型，不是用户注册面，不能进入注册面发现。
    if lexicon::is_registration_path(&relative) {
        return None;
    }
    // The cache is process-wide: `NodeIdCache::get` keeps to the namespace in
    // force and rechecks the identity it returns, so a second package cannot read
    // the first package's identity for the same relative path.
    // 该缓存是进程级的：`NodeIdCache::get` 只认当前生效的命名空间并复核返回的身份，因此第二个包
    // 不会拿到第一个包在同一相对路径上的身份。
    if let Some((id, _)) = CACHED_NODE_IDS.get().and_then(|cache| cache.get(&relative)) {
        return Some(*id);
    }
    let source = fs::read_to_string(file).ok()?;
    let kind = parsed_face(&source, &relative)?.path("kind")?;
    Some(registry_identity::package_node_id(&relative, &kind))
}

pub(crate) fn collect_node_ids(src: &Path, nodes: &[Node], ids: &mut BTreeSet<NodeId>) {
    for node in nodes {
        if let Some(id) = node_id(src, node) {
            ids.insert(id);
        }
        collect_node_ids(src, &node.children, ids);
    }
}

/// Select every face at or under `module` and return their sources for BFS.
/// 选中该模块及其子树内的全部注册面，返回其源码文本供 BFS 继续扫描。
pub(crate) fn select_module_subtree(
    faces: &[FaceSource],
    module: &str,
    selected: &mut BTreeSet<NodeId>,
) -> Vec<String> {
    faces
        .iter()
        .filter(|face| path_mentions_module(&face.module, module))
        .filter(|face| selected.insert(face.id))
        .filter_map(|face| fs::read_to_string(&face.source).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{FaceSource, select_module_subtree};
    use crate::build_method::registry_identity::NodeId;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn face(module: &str, source: PathBuf) -> FaceSource {
        FaceSource {
            id: crate::build_method::registry_identity::package_node_id(
                &source.to_string_lossy(),
                "Kind",
            ),
            source,
            module: module.to_owned(),
        }
    }

    #[test]
    fn select_module_subtree_keeps_the_whole_subtree_once() {
        let root = std::env::temp_dir().join(format!(
            "xirang-subtree-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let a = root.join("a/a.rs");
        let b = root.join("b/b.rs");
        let child = root.join("b/child/child.rs");
        for file in [&a, &b, &child] {
            std::fs::create_dir_all(file.parent().expect("parent")).expect("mkdir");
            std::fs::write(file, "// face\n").expect("write face");
        }
        let faces = vec![
            face("a", a.clone()),
            face("b", b.clone()),
            face("b::child", child.clone()),
        ];
        let mut selected = BTreeSet::new();
        let sources = select_module_subtree(&faces, "b", &mut selected);
        assert_eq!(sources.len(), 2);
        assert_eq!(selected.len(), 2);
        // Selecting again is a no-op.
        assert!(select_module_subtree(&faces, "b", &mut selected).is_empty());
        // Sibling "a" is untouched.
        let ids: Vec<NodeId> = faces.iter().map(|face| face.id).collect();
        assert!(!selected.contains(&ids[0]));

        let _ = std::fs::remove_dir_all(&root);
    }
}
