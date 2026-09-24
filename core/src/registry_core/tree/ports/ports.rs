//! The branch port surface: which capability names a branch exposes, and how a
//! reference written `Branch::port` resolves.
//! 分支端口面：一个分支对外暴露了哪些能力名，以及写成 `Branch::port` 的引用如何解析。
//!
//! Why this exists: a call site that spells the whole tree out
//! (`A::B::C::thing`) is coupled to where the implementation happens to live, so
//! replacing a branch forces every caller to follow the new shape. Naming the
//! **branch** and the **port** instead makes the reference independent of the
//! shape: the branch is still there after a replacement, only its contents
//! changed.
//! 为什么需要它：把整棵树拼出来的调用点（`A::B::C::thing`）与"实现恰好住在哪"耦合，因此替换
//! 一个分支会逼每个调用方跟着新形状走。改为命名**分支**与**端口**，引用就不再依赖形状：替换
//! 之后分支还在，变的只是它的内容。
//!
//! The vocabulary is not new. A face already declares the capability names it
//! makes available in `provides`, and the names it needs in `requires`; this
//! module adds only the *scoping rule* a reference obeys:
//! 词汇不是新造的。注册面本来就在 `provides` 里声明它对外提供的能力名、在 `requires` 里声明
//! 它需要的能力名；本模块只增加引用所遵守的**作用域规则**：
//!
//! 1. the branch handle is a node's `registry_name`, not its path: `B1` and `B2`
//!    diverge at `A`, so a reference names `B2`, and it resolves absolutely —
//!    the answer does not depend on where the reference is written;
//! 1. 分支句柄是节点的 `registry_name`，不是路径：`B1` 与 `B2` 在 `A` 处分叉，因此引用写
//!    `B2`；解析是绝对的——答案不取决于引用写在哪儿；
//! 2. the port resolves inside that branch's own subtree, the branch itself
//!    included;
//! 2. 端口在该分支自己的子树内解析，分支自身也算在内；
//! 3. a handle or a port that matches more than one face is **reported, never
//!    resolved silently** — an implicit "nearest wins" rule is exactly the kind
//!    of hidden behaviour this workspace refuses.
//! 3. 匹配到多个面的句柄或端口**一律报出，绝不静默解析**——隐式的"最近者优先"正是本工作区
//!    拒绝的那类隐性行为。
//!
//! Boundary: same-branch references do **not** need this index. Inside a branch
//! its own internals are reachable directly (`C4::…`); only the boundary is
//! closed, because requiring a port for every internal collaboration would add
//! noise without adding a guarantee.
//! 边界：同分支内部的引用**不需要**本索引。分支内部可以直接引用自己的内部（`C4::…`）；封闭
//! 的只有边界，因为要求每次内部协作都先设计一个端口只会增加噪音，不增加任何保证。

use crate::registry_core::identity::NodeId;
use crate::registry_core::lexicon;

use super::Registry;

/// One face as the port surface sees it: what it is called, where it sits, and
/// which capability names it declares.
/// 端口面所看到的一个注册面：它叫什么、位于何处、声明了哪些能力名。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclaredPort {
    /// The face's identity.
    /// 该注册面的身份。
    pub node: NodeId,
    /// The face's logical path, used to test "inside this branch".
    /// 该注册面的逻辑路径，用于判定"是否在该分支内"。
    pub path: String,
    /// The name a reference may use as a branch handle.
    /// 引用可作为分支句柄使用的名字。
    pub registry_name: String,
    /// The capability names the face declares it provides.
    /// 该注册面声明提供的能力名。
    pub provides: Vec<String>,
}

/// One capability name a branch exposes, and the face that provides it.
/// 一个分支暴露的一个能力名，以及提供它的注册面。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExposedPort {
    /// The declared capability name.
    /// 声明的能力名。
    pub port: String,
    /// The providing face.
    /// 提供该能力名的注册面。
    pub node: NodeId,
    /// The providing face's logical path.
    /// 提供该能力名的注册面逻辑路径。
    pub path: String,
}

/// A branch handle that matches more than one node.
/// 匹配到多个节点的分支句柄。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AmbiguousBranchName {
    /// The duplicated handle.
    /// 重复的句柄。
    pub name: String,
    /// Every node carrying it, by logical path.
    /// 携带它的每个节点，按逻辑路径列出。
    pub paths: Vec<String>,
}

/// The answer to "does `Branch::port` name exactly one face?".
/// "`Branch::port` 是否恰好命名一个注册面？"的答案。
///
/// Every failure is a distinct variant carrying the facts a diagnostic needs
/// (which paths collided), so a caller never has to re-derive them.
/// 每种失败都是独立的变体，并携带诊断所需的事实（哪些路径冲突），因此调用方不需要自己重新
/// 推导。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Exactly one face provides the port inside the branch.
    /// 分支内恰好有一个注册面提供该端口。
    One {
        /// The providing face.
        /// 提供该端口的注册面。
        node: NodeId,
        /// Its logical path.
        /// 它的逻辑路径。
        path: String,
    },
    /// No node in the tree carries that branch handle.
    /// 树中没有任何节点携带该分支句柄。
    UnknownBranch,
    /// Several nodes carry that branch handle, so the reference is ambiguous
    /// before the port is even considered.
    /// 多个节点携带该分支句柄，因此还没看端口，引用就已经有歧义。
    AmbiguousBranch {
        /// Every carrier, by logical path.
        /// 每个携带者，按逻辑路径列出。
        paths: Vec<String>,
    },
    /// The branch exists; nothing in it provides that port.
    /// 分支存在，但其中没有东西提供该端口。
    UnknownPort {
        /// The branch handle that was searched.
        /// 被搜索的分支句柄。
        branch: String,
    },
    /// Several faces inside the branch provide that port.
    /// 分支内有多个注册面提供该端口。
    AmbiguousPort {
        /// Every provider, by logical path.
        /// 每个提供者，按逻辑路径列出。
        paths: Vec<String>,
    },
}

/// The port surface of one tree, built once and queried per reference.
/// 一棵树的端口面：构建一次，按引用查询。
#[derive(Clone, Debug, Default)]
pub struct PortIndex {
    entries: Vec<DeclaredPort>,
}

impl PortIndex {
    /// Build an index over the given faces.
    /// 在给定注册面之上构建索引。
    ///
    /// The order of `entries` does not matter: every answer is computed from
    /// the whole set, so two builds of the same tree cannot disagree.
    /// `entries` 的顺序无关紧要：每个答案都由全集算出，因此同一棵树构建两次不会给出不同答案。
    pub fn new(entries: Vec<DeclaredPort>) -> Self {
        Self { entries }
    }

    /// Every face the index knows.
    /// 索引知道的每个注册面。
    pub fn entries(&self) -> &[DeclaredPort] {
        &self.entries
    }

    /// The ports one branch exposes, sorted by name then path.
    /// 一个分支暴露的端口，先按名字再按路径排序。
    ///
    /// Collisions are **not** collapsed here: the caller reports them (see
    /// [`PortIndex::resolve`]), because a renderer that published an arbitrary
    /// winner would hide the very conflict the rule exists to surface.
    /// 冲突**不会**在这里被合并：由调用方报出（见 [`PortIndex::resolve`]），因为发布一个随意
    /// 胜出者的渲染器会藏起这条规则本就要暴露的冲突。
    pub fn exposed_ports(&self, branch: &str) -> Vec<ExposedPort> {
        let mut found = Vec::new();
        for node in self.nodes_named(branch) {
            for entry in self.inside(node) {
                for port in &entry.provides {
                    found.push(ExposedPort {
                        port: port.clone(),
                        node: entry.node,
                        path: entry.path.clone(),
                    });
                }
            }
        }
        found.sort_by(|left, right| {
            left.port
                .cmp(&right.port)
                .then_with(|| left.path.cmp(&right.path))
        });
        found
    }

    /// Resolve `Branch::port` to exactly one face, or say precisely why not.
    /// 把 `Branch::port` 解析到恰好一个注册面，或者精确说明为何不能。
    pub fn resolve(&self, branch: &str, port: &str) -> Resolution {
        let carriers = self.nodes_named(branch);
        if carriers.is_empty() {
            return Resolution::UnknownBranch;
        }
        let mut paths: Vec<String> = carriers.iter().map(|entry| entry.path.clone()).collect();
        if carriers.len() > 1 {
            paths.sort();
            return Resolution::AmbiguousBranch { paths };
        }
        let mut providers: Vec<&DeclaredPort> = self
            .inside(carriers[0])
            .filter(|entry| entry.provides.iter().any(|name| name == port))
            .collect();
        providers.sort_by(|left, right| left.path.cmp(&right.path));
        match providers.as_slice() {
            [] => Resolution::UnknownPort {
                branch: branch.to_owned(),
            },
            [only] => Resolution::One {
                node: only.node,
                path: only.path.clone(),
            },
            many => Resolution::AmbiguousPort {
                paths: many.iter().map(|entry| entry.path.clone()).collect(),
            },
        }
    }

    /// Branch handles that match more than one node, sorted by handle.
    /// 匹配到多个节点的分支句柄，按句柄排序。
    ///
    /// A reference cannot be checked against an ambiguous handle, so every one
    /// of these is a build-time problem in its own right, independent of which
    /// ports are referenced.
    /// 引用无法针对有歧义的句柄做检查，因此每一个这样的句柄本身就是构建期问题，与引用了哪些
    /// 端口无关。
    pub fn ambiguous_branch_names(&self) -> Vec<AmbiguousBranchName> {
        let mut names: Vec<&str> = self
            .entries
            .iter()
            .map(|entry| entry.registry_name.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
            .into_iter()
            .filter_map(|name| {
                let carriers = self.nodes_named(name);
                (carriers.len() > 1).then(|| AmbiguousBranchName {
                    name: name.to_owned(),
                    paths: carriers.iter().map(|entry| entry.path.clone()).collect(),
                })
            })
            .collect()
    }

    /// The nodes whose `registry_name` is `name`.
    /// `registry_name` 为 `name` 的节点。
    fn nodes_named<'a>(&'a self, name: &str) -> Vec<&'a DeclaredPort> {
        let mut found: Vec<&DeclaredPort> = self
            .entries
            .iter()
            .filter(|entry| entry.registry_name == name)
            .collect();
        found.sort_by(|left, right| left.path.cmp(&right.path));
        found
    }

    /// Every face inside one branch, the branch itself included.
    /// 一个分支内的每个注册面，包含该分支自身。
    fn inside<'a>(&'a self, branch: &'a DeclaredPort) -> impl Iterator<Item = &'a DeclaredPort> {
        self.entries.iter().filter(move |entry| {
            // `path_is_under` counts the prefix itself as "under", which is what
            // makes a branch's own ports addressable; the stricter predicate
            // would silently drop `B2::own`.
            // `path_is_under` 把前缀本身也算作"位于其下"，这正是分支自身的端口可被寻址的原因；
            // 更严格的那个谓词会静默丢掉 `B2::own`。
            lexicon::path_is_under(&entry.path, &branch.path)
        })
    }
}

impl Registry {
    /// Build the port surface of this tree.
    /// 构建这棵树的端口面。
    ///
    /// The walk is the only part that needs the tree; every rule lives in
    /// [`PortIndex`], so a caller can also assemble an index from declarations
    /// it already has (the build does) without walking anything.
    /// 只有遍历需要树；所有规则都住在 [`PortIndex`] 里，因此调用方也可以用手上已有的声明直接
    /// 组装索引（构建步骤就是这样），不必遍历。
    pub fn port_index(&self) -> PortIndex {
        let mut entries = Vec::new();
        collect_declared_ports(self, &mut entries);
        PortIndex::new(entries)
    }
}

/// Walk one registry and its child registries, collecting declared ports.
/// 遍历一个注册机及其子注册机，收集声明的端口。
fn collect_declared_ports(registry: &Registry, entries: &mut Vec<DeclaredPort>) {
    for entry in registry.entries.values() {
        let info = &entry.info;
        // A node that owns a child registry has its authoritative path in that
        // registry's header; a leaf only has a derived one. Both are relative
        // to the same parent, so the derived form never disagrees with the
        // authoritative one it hangs from.
        // 拥有子注册机的节点，其权威路径在那份子注册机的头部；叶子只有推导出来的路径。两者都
        // 相对同一个父级，因此推导形式永远不会与它所挂靠的权威形式不一致。
        let path = entry.child.as_ref().map_or_else(
            || format!("{}/{}", registry.header.path, info.registry_name),
            |child| child.header.path.clone(),
        );
        entries.push(DeclaredPort {
            node: info.id,
            path,
            registry_name: info.registry_name.clone(),
            provides: info.provides.clone(),
        });
        if let Some(child) = &entry.child {
            collect_declared_ports(child, entries);
        }
    }
}

// The tests live in their own file, as `record.rs` does: an inline module would
// push this file over its budget and bury the rules above.
// 测试放在独立文件里，与 `record.rs` 一样：内联模块会把本文件顶出预算，并埋掉上面的规则。
#[cfg(test)]
#[path = "ports_tests.rs"]
mod ports_tests;
