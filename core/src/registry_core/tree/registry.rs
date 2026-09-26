// The one registry type, including atomic recursive registration and queries.
// 唯一的 Registry 类型，包含原子递归注册和查询。

use std::sync::Arc;

use super::entry_pages::EntryPages;
use super::header::RegistryHeader;
pub use super::index::{RegistryIndex, RegistryStorageStats};
use crate::registry_core::declaration::FrameworkId;
use crate::registry_core::declaration::{
    Admission, OwnedAdmission, OwnedRegistrationRule, RegistrationRule,
};
use crate::registry_core::identity::{NodeId, ROOT_NODE_ID, root_node_id};

/// Parent and child levels use this exact type.
/// 父级和子级使用完全相同的类型。
#[derive(Clone, Debug)]
pub struct Registry {
    pub(super) header: Arc<RegistryHeader>,
    // The map is immutable while a snapshot is shared. Transactions copy only
    // the map page they mutate, not the whole registry tree.
    // 快照共享时 map 不可变；事务只复制实际修改的 map 页，不复制整棵树。
    pub(super) entries: Arc<EntryPages>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::root()
    }
}

impl Registry {
    /// Create the one top-level registry.
    /// 创建唯一的顶层注册机。
    pub fn root() -> Self {
        Self::new(
            FrameworkId::new("nichlink.default"),
            "nichlink.default".to_owned(),
            "root",
            ROOT_NODE_ID,
            RegistrationRule::ANY.into_owned(),
            "<registry-root>",
            Admission::ANY.into_owned(),
        )
    }

    /// Create a root for one framework namespace.
    /// 为一个框架命名空间创建根注册机。
    pub fn root_for(framework: FrameworkId) -> Self {
        Self::root_for_namespace(framework, framework.0)
    }

    /// Create an isolated root for one package or application namespace.
    /// 为一个包或应用命名空间创建隔离的根。
    pub fn root_for_namespace(framework: FrameworkId, namespace: impl Into<String>) -> Self {
        let namespace = namespace.into();
        let id = root_node_id(&namespace);
        Self::new(
            framework,
            namespace,
            "root",
            id,
            RegistrationRule::ANY.into_owned(),
            "<registry-root>",
            Admission::ANY.into_owned(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        framework: FrameworkId,
        namespace: String,
        path: &str,
        id: NodeId,
        registration_rule: OwnedRegistrationRule,
        registration_rule_path: impl Into<String>,
        admission: OwnedAdmission,
    ) -> Self {
        Self {
            header: Arc::new(RegistryHeader {
                namespace,
                framework,
                path: path.to_owned(),
                id,
                registration_rule,
                registration_rule_path: registration_rule_path.into(),
                admission,
            }),
            entries: Arc::new(EntryPages::default()),
        }
    }

    /// The framework this registry was created under.
    /// 该注册机创建时所用的框架。
    pub fn framework(&self) -> FrameworkId {
        self.header.framework
    }
}
