//! Read-only Registry indexes and storage counters.
//! Registry 只读索引与存储统计。

use std::collections::BTreeMap;

use super::Registry;
use crate::registry_core::identity::{NodeId, StableFaceId};
use std::sync::Arc;

/// A flattened lookup index built from one registry subtree.
/// 由一棵注册子树构建的扁平查找索引。
#[derive(Clone, Debug, Default)]
pub struct RegistryIndex {
    pub(super) by_id: BTreeMap<NodeId, String>,
    pub(super) by_path: BTreeMap<String, NodeId>,
    pub(super) by_kind: BTreeMap<String, Vec<NodeId>>,
    pub(super) by_stable: BTreeMap<StableFaceId, Vec<NodeId>>,
}

/// How much storage one registry tree occupies, as page and entry counts.
/// 一棵注册树占用的存储规模，以页与条目计数表示。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegistryStorageStats {
    /// Entry-page count of the whole container, shared or not.
    /// 整个容器的条目页数，无论是否共享。
    pub pages: usize,
    /// Number of direct entries this registry holds.
    /// 该注册机持有的直接条目数。
    pub entries: usize,
    /// Pages still backed by a container shared with another registry.
    /// 后备容器仍与其他注册机共享的页数。
    pub shared_pages: usize,
}

impl RegistryIndex {
    // `is_empty` was removed with the other zero-caller queries in B3a, but
    // `len` stays because `run_method`'s scale-audit example reports index size.
    // Clippy's `len_without_is_empty` then fires on a public type whose only
    // other consumer is an example, so the allowance is deliberate.
    // B3a 随其余零调用者查询一并删除了 `is_empty`，但 `len` 保留——run_method 的
    // scale-audit 示例用它报告索引规模。clippy 的 `len_without_is_empty` 因此会在这个
    // 公开类型上触发（另一个消费方只是示例），该允许是有意为之。
    /// Number of identities in the flattened index, including every descendant.
    /// 扁平索引中的身份总数，包含全部后代。
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }
}

impl Registry {
    /// Count how many pages and entries this registry tree occupies.
    /// 统计该注册树占用的页数与条目数。
    pub fn storage_stats(&self) -> super::RegistryStorageStats {
        let entries_shared = Arc::strong_count(&self.entries) > 1;
        super::RegistryStorageStats {
            pages: super::entry_pages::ENTRY_PAGE_COUNT,
            entries: self.entries.len(),
            // A cloned registry shares the immutable EntryPages container, so
            // every page is logically shared even before a page-level write
            // separates the containers. Once separated, inspect each page Arc
            // to report the remaining fine-grained sharing accurately.
            // 克隆注册机会共享不可变 EntryPages 容器，因此在页级写入前所有页都属于共享；
            // 容器分离后再检查每个页 Arc，准确反映细粒度共享。
            shared_pages: if entries_shared {
                super::entry_pages::ENTRY_PAGE_COUNT
            } else {
                self.entries
                    .pages
                    .iter()
                    .filter(|page| Arc::strong_count(page) > 1)
                    .count()
            },
        }
    }

    /// Build a flattened lookup index over this registry and every descendant.
    /// 为该注册机及其全部后代构建扁平查找索引。
    pub fn index(&self) -> RegistryIndex {
        let mut index = RegistryIndex::default();
        self.fill_index(&mut index);
        index
    }

    fn fill_index(&self, index: &mut RegistryIndex) {
        index.by_id.insert(self.header.id, self.header.path.clone());
        index
            .by_path
            .insert(self.header.path.clone(), self.header.id);
        for entry in self.entries.values() {
            let entry_path = format!("{}/{}", self.header.path, entry.info.registry_name);
            index.by_id.insert(entry.info.id, entry_path.clone());
            index.by_path.insert(entry_path, entry.info.id);
            index
                .by_kind
                .entry(entry.info.kind.clone())
                .or_default()
                .push(entry.info.id);
            if let Some(stable) = entry.info.explicit_stable_face_id() {
                index
                    .by_stable
                    .entry(stable)
                    .or_default()
                    .push(entry.info.id);
            }
            if let Some(child) = &entry.child {
                child.fill_index(index);
            }
        }
    }
}
