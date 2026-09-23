//! Read-only Registry lookup operations.
//! Registry 只读查询操作。

use crate::registry_core::declaration::RegistrationSnapshot;
use crate::registry_core::identity::NodeId;
use crate::registry_core::identity::StableFaceId;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::Registry;
use super::entry_pages::RegisteredEntry;

impl Registry {
    pub(super) fn registry_mut(&mut self, wanted: NodeId) -> Option<&mut Registry> {
        if self.header.id == wanted {
            return Some(self);
        }
        let mut path = Vec::new();
        if !self.collect_registry_path(wanted, &mut path) {
            return None;
        }
        self.registry_mut_at_path(&path)
    }

    fn collect_registry_path(&self, wanted: NodeId, path: &mut Vec<NodeId>) -> bool {
        for entry in self.entries.values() {
            let Some(child) = entry.child.as_ref() else {
                continue;
            };
            path.push(entry.info.id);
            if child.header.id == wanted || child.collect_registry_path(wanted, path) {
                return true;
            }
            path.pop();
        }
        false
    }

    fn registry_mut_at_path(&mut self, path: &[NodeId]) -> Option<&mut Registry> {
        let (owner, rest) = path.split_first()?;
        let entry = Arc::make_mut(&mut self.entries).get_mut(owner)?;
        let child = Arc::make_mut(entry.child.as_mut()?);
        if rest.is_empty() {
            Some(child)
        } else {
            child.registry_mut_at_path(rest)
        }
    }

    /// Find the registry with this identity anywhere in the subtree.
    /// 在本子树中查找具有该身份的注册机。
    pub fn registry(&self, wanted: NodeId) -> Option<&Registry> {
        if self.header.id == wanted {
            return Some(self);
        }
        self.entries
            .values()
            .filter_map(|entry| entry.child.as_ref())
            .find_map(|child| child.registry(wanted))
    }

    pub(super) fn entry_at(&self, wanted: NodeId) -> Option<&RegisteredEntry> {
        if let Some(entry) = self.entries.get(&wanted) {
            return Some(entry);
        }
        for entry in self.entries.values() {
            if let Some(found) = entry
                .child
                .as_ref()
                .and_then(|child| child.entry_at(wanted))
            {
                return Some(found);
            }
        }
        None
    }

    /// Look up the registration snapshot of one face by identity.
    /// 按身份查找某个注册面的注册快照。
    pub fn find(&self, id: NodeId) -> Option<&RegistrationSnapshot> {
        self.entry_at(id).map(|entry| entry.info.as_ref())
    }

    /// Return the current logical path of a face, derived from the live tree.
    /// 返回某个注册面的当前逻辑路径，由现存树推导。
    pub fn path_for(&self, id: NodeId) -> Option<String> {
        if let Some(entry) = self.entries.get(&id) {
            return Some(format!("{}/{}", self.header.path, entry.info.registry_name));
        }
        for entry in self.entries.values() {
            if let Some(found) = entry.child.as_ref().and_then(|child| child.path_for(id)) {
                return Some(found);
            }
        }
        None
    }

    /// Return the identity chain from this registry's root down to a face.
    /// 返回从本注册机根到某个注册面的身份链。
    pub fn node_path(&self, id: NodeId) -> Option<Vec<NodeId>> {
        let mut path = vec![self.header.id];
        self.collect_node_path(id, &mut path).then_some(path)
    }

    fn collect_node_path(&self, wanted: NodeId, path: &mut Vec<NodeId>) -> bool {
        for entry in self.entries.values() {
            path.push(entry.info.id);
            if entry.info.id == wanted {
                return true;
            }
            if let Some(child) = entry.child.as_ref()
                && child.collect_node_path(wanted, path)
            {
                return true;
            }
            path.pop();
        }
        false
    }

    pub(super) fn collect_stable_names(
        &self,
        names: &mut BTreeMap<StableFaceId, (NodeId, String)>,
    ) {
        for entry in self.entries.values() {
            if let Some(stable) = entry.info.explicit_stable_face_id() {
                names.insert(stable, (entry.info.id, entry.info.source.file.clone()));
            }
            if let Some(child) = &entry.child {
                child.collect_stable_names(names);
            }
        }
    }

    pub(super) fn collect_node_ids(&self, ids: &mut BTreeSet<NodeId>) {
        for entry in self.entries.values() {
            ids.insert(entry.info.id);
            if let Some(child) = &entry.child {
                child.collect_node_ids(ids);
            }
        }
    }

    /// Collect every face of one kind, in depth-first order.
    /// 按深度优先顺序收集某一 kind 的全部注册面。
    pub fn find_kind(&self, kind: &str) -> Vec<&RegistrationSnapshot> {
        let mut found = Vec::new();
        self.collect_kind(kind, &mut found);
        found
    }

    /// Collect every face the predicate accepts, in depth-first order.
    /// 按深度优先顺序收集谓词接受的全部注册面。
    pub fn find_where(
        &self,
        predicate: impl Fn(&RegistrationSnapshot) -> bool,
    ) -> Vec<&RegistrationSnapshot> {
        self.depth_first()
            .into_iter()
            .filter(|info| predicate(info))
            .collect()
    }

    fn collect_kind<'a>(&'a self, kind: &str, found: &mut Vec<&'a RegistrationSnapshot>) {
        for entry in self.entries.values() {
            if entry.info.kind == kind {
                found.push(entry.info.as_ref());
            }
            if let Some(child) = &entry.child {
                child.collect_kind(kind, found);
            }
        }
    }

    /// Flatten the subtree into a depth-first list of registration snapshots.
    /// 把子树展平成深度优先的注册快照列表。
    pub fn depth_first(&self) -> Vec<&RegistrationSnapshot> {
        let mut entries = Vec::new();
        self.collect_depth_first(&mut entries);
        entries
    }

    fn collect_depth_first<'a>(&'a self, entries: &mut Vec<&'a RegistrationSnapshot>) {
        for entry in self.entries.values() {
            entries.push(entry.info.as_ref());
            if let Some(child) = &entry.child {
                child.collect_depth_first(entries);
            }
        }
    }
}
