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

use nichlink::lexicon;

use super::entry::path_mentions_module;
use super::faces::FaceSource;
use super::node::{Node, relative_display};
use super::registry_identity::{self, NodeId};
use super::validation::parsed_face;

/// Node identities primed from the discovery cache before scope inference.
/// 作用域推导前由发现缓存预热的 node 身份缓存。
pub(crate) static CACHED_NODE_IDS: OnceLock<BTreeMap<String, (NodeId, String)>> = OnceLock::new();

pub(crate) fn node_id(src: &Path, node: &Node) -> Option<NodeId> {
    let file = node.file.as_ref()?;
    let relative = relative_display(src, file);
    // Registry infrastructure defines the macros and support types; it is not
    // a user registration face and must never enter the face discovery pass.
    // 注册基础设施只提供宏和支撑类型，不是用户注册面，不能进入注册面发现。
    if lexicon::is_registration_path(&relative) {
        return None;
    }
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
    use crate::registry_identity::NodeId;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    fn face(module: &str, source: PathBuf) -> FaceSource {
        FaceSource {
            id: crate::registry_identity::package_node_id(&source.to_string_lossy(), "Kind"),
            source,
            module: module.to_owned(),
        }
    }

    #[test]
    fn select_module_subtree_keeps_the_whole_subtree_once() {
        let root = std::env::temp_dir().join(format!(
            "nichlink-subtree-{}-{}",
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
