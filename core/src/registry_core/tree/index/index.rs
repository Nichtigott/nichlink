//! Read-only Registry indexes and storage counters.
//! Registry 只读索引与存储统计。

use std::collections::BTreeMap;

use super::Registry;
use crate::registry_core::identity::{NodeId, StableFaceId};
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct RegistryIndex {
    pub(super) by_id: BTreeMap<NodeId, String>,
    pub(super) by_path: BTreeMap<String, NodeId>,
    pub(super) by_kind: BTreeMap<String, Vec<NodeId>>,
    pub(super) by_stable: BTreeMap<StableFaceId, Vec<NodeId>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegistryStorageStats {
    pub pages: usize,
    pub entries: usize,
    pub shared_pages: usize,
}

impl RegistryIndex {
    pub fn id_for_path(&self, path: &str) -> Option<NodeId> {
        self.by_path.get(path).copied()
    }
    pub fn path_for_id(&self, id: NodeId) -> Option<&str> {
        self.by_id.get(&id).map(String::as_str)
    }
    pub fn ids_for_kind(&self, kind: &str) -> &[NodeId] {
        self.by_kind.get(kind).map(Vec::as_slice).unwrap_or(&[])
    }
    pub fn ids_for_stable(&self, id: StableFaceId) -> &[NodeId] {
        self.by_stable.get(&id).map(Vec::as_slice).unwrap_or(&[])
    }
    pub fn len(&self) -> usize {
        self.by_id.len()
    }
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

impl Registry {
    pub fn storage_stats(&self) -> super::RegistryStorageStats {
        let entries_shared = Arc::strong_count(&self.entries) > 1;
        super::RegistryStorageStats {
            pages: super::ENTRY_PAGE_COUNT,
            entries: self.entries.len(),
            // A cloned registry shares the immutable EntryPages container, so
            // every page is logically shared even before a page-level write
            // separates the containers. Once separated, inspect each page Arc
            // to report the remaining fine-grained sharing accurately.
            // 克隆注册机会共享不可变 EntryPages 容器，因此在页级写入前所有页都属于共享；
            // 容器分离后再检查每个页 Arc，准确反映细粒度共享。
            shared_pages: if entries_shared {
                super::ENTRY_PAGE_COUNT
            } else {
                self.entries
                    .pages
                    .iter()
                    .filter(|page| Arc::strong_count(page) > 1)
                    .count()
            },
        }
    }

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
