//! Registry metadata and counters.
//! 注册机元数据与计数。

use crate::registry_core::identity::NodeId;

use super::Registry;

impl Registry {
    /// The logical path this registry occupies in the tree.
    /// 该注册机在树中占据的逻辑路径。
    pub fn path(&self) -> &str {
        &self.header.path
    }
    /// The stable identity of this registry.
    /// 该注册机的稳定身份。
    pub fn id(&self) -> NodeId {
        self.header.id
    }
    /// Number of direct entries this registry holds.
    /// 该注册机持有的直接条目数。
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    /// Whether this registry holds no direct entries.
    /// 该注册机是否不持有任何直接条目。
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    // `namespace`, `name`, `registration_rule`, `dependency_admission_accepts`
    // and `allowed_dependency_paths` were zero-caller readers and were removed
    // in B3a. The header fields they exposed stay: the tree implementation and
    // `dump` read them directly, so only the public door was closed.
    // `namespace`、`name`、`registration_rule`、`dependency_admission_accepts`
    // 与 `allowed_dependency_paths` 是零调用者读取器，B3a 已删除。它们暴露的 header
    // 字段保留：树实现与 `dump` 直接读取它们，因此关掉的只是公开入口。

    /// Number of registries in this subtree, including this one.
    /// 本子树中的注册机数量，含自身。
    pub fn registry_count(&self) -> usize {
        1 + self
            .entries
            .values()
            .filter_map(|entry| entry.child.as_ref())
            .map(|child| child.registry_count())
            .sum::<usize>()
    }

    /// Number of faces registered below this registry.
    /// 本注册机之下注册的面数量。
    pub fn node_count(&self) -> usize {
        self.entries
            .values()
            .map(|entry| 1 + entry.child.as_ref().map_or(0, |child| child.node_count()))
            .sum()
    }

    /// Total runtime checks declared below this registry.
    /// 本注册机之下声明的运行期检查总数。
    pub fn check_count(&self) -> usize {
        self.entries
            .values()
            .map(|entry| {
                entry.info.runtime_checks.len()
                    + entry.child.as_ref().map_or(0, |child| child.check_count())
            })
            .sum()
    }
}
