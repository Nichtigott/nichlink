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

    // This view exposes what has a caller here: namespace, name, registration rule,
    // dependency admission and the allowed dependency paths stay readable through the
    // `RegistryMetadata` fields the tree implementation and `dump` read, and have no
    // separate accessor, because a reader with no caller is how the surface grows
    // silently.
    // 本视图只暴露这里确有调用者的东西：命名空间、名字、注册规则、依赖准入与允许的依赖路径经
    // `RegistryMetadata` 的字段可读——树实现与 `dump` 读的就是它们——而不另设访问器，因为无人
    // 调用的读取器正是表面静默膨胀的方式。

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
