//! Persistent entry pages used by Registry transactions.
//! Registry 事务使用的持久化条目页。

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::registry_core::declaration::RegistrationSnapshot;
use crate::registry_core::identity::NodeId;
use crate::tree::Registry;

#[derive(Clone, Debug)]
pub(super) struct RegisteredEntry {
    pub(super) info: Arc<RegistrationSnapshot>,
    pub(super) child: Option<Arc<Registry>>,
}

pub(super) const ENTRY_PAGE_COUNT: usize = 32;

#[derive(Clone, Debug)]
pub(super) struct EntryPages {
    pub(super) pages: [Arc<BTreeMap<NodeId, RegisteredEntry>>; ENTRY_PAGE_COUNT],
    pub(super) len: usize,
}

impl Default for EntryPages {
    fn default() -> Self {
        Self {
            pages: std::array::from_fn(|_| Arc::new(BTreeMap::new())),
            len: 0,
        }
    }
}

impl EntryPages {
    fn page(id: NodeId) -> usize {
        usize::from(id.into_bytes()[0]) % ENTRY_PAGE_COUNT
    }

    pub(super) fn get(&self, id: &NodeId) -> Option<&RegisteredEntry> {
        self.pages[Self::page(*id)].get(id)
    }

    pub(super) fn get_mut(&mut self, id: &NodeId) -> Option<&mut RegisteredEntry> {
        Arc::make_mut(&mut self.pages[Self::page(*id)]).get_mut(id)
    }

    pub(super) fn values(&self) -> impl Iterator<Item = &RegisteredEntry> {
        self.pages.iter().flat_map(|page| page.values())
    }

    pub(super) fn values_mut(&mut self) -> impl Iterator<Item = &mut RegisteredEntry> {
        self.pages
            .iter_mut()
            .flat_map(|page| Arc::make_mut(page).values_mut())
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&NodeId, &RegisteredEntry)> {
        self.pages.iter().flat_map(|page| page.iter())
    }

    pub(super) fn len(&self) -> usize {
        self.len
    }

    pub(super) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(super) fn insert(&mut self, id: NodeId, entry: RegisteredEntry) -> Option<RegisteredEntry> {
        let page = &mut self.pages[Self::page(id)];
        let previous = Arc::make_mut(page).insert(id, entry);
        if previous.is_none() {
            self.len += 1;
        }
        previous
    }

    pub(super) fn remove(&mut self, id: &NodeId) -> Option<RegisteredEntry> {
        let page = &mut self.pages[Self::page(*id)];
        let removed = Arc::make_mut(page).remove(id);
        if removed.is_some() {
            self.len -= 1;
        }
        removed
    }
}
