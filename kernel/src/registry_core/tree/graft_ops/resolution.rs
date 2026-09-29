//! Graft selector resolution and staged-identity rebasing.
//! 嫁接选择器的解析与暂存身份的重新定基。
//!
//! A graft names its cut target and its replacement either by logical path or by
//! compile-time identity; a cut may also span a sibling range. This page turns
//! those selectors into concrete [`NodeId`]s and rebases the copied external
//! subtree onto the logical base slot.
//! 嫁接用逻辑路径或编译期身份命名切口目标与替换件；切口也可能覆盖一段兄弟区间。
//! 本页把这些选择器变成具体的 [`NodeId`]，并把复制来的外部子树重新定基到逻辑基座槽位。

use std::sync::Arc;

use crate::registry_core::declaration::RegistrationSnapshot;
use crate::registry_core::diagnostic::RegistryResult;
use crate::registry_core::identity::NodeId;
use crate::registry_core::plugin::graft::GraftError;

use super::Registry;
use super::overlay::{GraftCutRef, GraftTargetRef, Resolution};

impl Registry {
    /// The *one* node at `path`, or why there is not exactly one.
    /// `path` 上**唯一**的那个节点，或者"为何不唯一"。
    ///
    /// The graft-cut selector's path arm resolves through this, so a path two nodes
    /// share is reported as ambiguous instead of the first match winning. It used to
    /// take the first match while [`Registry::resolve_node`] — the same question asked
    /// of a name, a kind, or a path — answered `Ambiguous`, and that difference was a
    /// recorded debt rather than a second rule (`ports.rs`, audit `LGC-LG-07`).
    /// 移植切口选择器的路径那一支经此解析，因此两个节点共用的路径会被报成多义，而不是让第一个
    /// 匹配胜出。它过去取第一个匹配，而 [`Registry::resolve_node`]——对名字、kind 或路径问同一个
    /// 问题——回答 `Ambiguous`；那个差别是记账中的欠账，而不是第二条规则（`ports.rs`，审计
    /// `LGC-LG-07`）。
    ///
    /// A shared path is still reachable after registration refuses same-named siblings
    /// (`plan_batch`), because an in-place edit may rename a face
    /// (`validate_snapshot_replacement` checks namespace, identity, parent, rule and
    /// contract — not the `registry_name`). Refusing here is what keeps that state from
    /// silently reselecting a different node.
    /// 在注册拒绝兄弟同名（`plan_batch`）之后，共用路径仍然可达：就地编辑可以给面改名
    /// （`validate_snapshot_replacement` 检查命名空间、身份、父级、规则与契约——但不检查
    /// `registry_name`）。在这里拒绝，正是让那种状态不会静默地改选另一个节点的原因。
    pub(super) fn resolve_path_strict(&self, path: &str) -> Resolution {
        let mut matches = self.depth_first().into_iter().filter(|info| {
            self.path_for(info.id)
                .is_some_and(|candidate| candidate == path)
        });
        let Some(first) = matches.next() else {
            return Resolution::Missing;
        };
        match matches.count() {
            0 => Resolution::One(first.id),
            extra => Resolution::Ambiguous(extra + 1),
        }
    }

    /// The **first** node whose logical path equals `path`, or `None`.
    /// 逻辑路径等于 `path` 的**第一个**节点；没有则 `None`。
    ///
    /// This is the one remaining first-match lookup in the tree, and its boundary is
    /// what makes it decidable rather than a second rule: its only caller is
    /// [`Registry::resolve_record`], which reads the path a record stored as a *hint*
    /// and compares it against that record's durable identity — a hint that disagrees
    /// is refused, and a hint that stands in for a missing identity is reported as
    /// drift. No node is chosen silently there. The graft-cut selector used to be its
    /// second caller and no longer is: that arm goes through
    /// [`Registry::resolve_path_strict`] (audit `LGC-LG-07`).
    /// 这是树里仅存的一处"取第一个匹配"，而它的边界正是让它可判定、而不是第二条规则的东西：它
    /// 唯一的调用方是 [`Registry::resolve_record`]，那里只把记录存下的路径当作**提示**，并与该
    /// 记录里耐久的身份对比——答得不一样的提示被拒绝，而顶替缺失身份的提示会被报成漂移。那里不
    /// 存在静默选节点。移植切口选择器过去是它的第二个调用方，现在不再是：那一支走
    /// [`Registry::resolve_path_strict`]（审计 `LGC-LG-07`）。
    pub(super) fn resolve_path(&self, path: &str) -> Option<NodeId> {
        self.depth_first().into_iter().find_map(|info| {
            self.path_for(info.id)
                .filter(|candidate| candidate == path)
                .map(|_| info.id)
        })
    }

    /// Resolve a cut selector written either as a logical path or as the
    /// compile-time identity of the target face.
    /// 解析切口选择器：写法可以是逻辑路径，也可以是目标注册面的编译期身份。
    fn resolve_target(&self, target: GraftTargetRef<'_>) -> Resolution {
        match target {
            GraftTargetRef::Path(path) => self.resolve_path_strict(path),
            GraftTargetRef::Id(id) => {
                if self.find_registry(id).is_some() {
                    Resolution::One(id)
                } else {
                    Resolution::Missing
                }
            }
        }
    }

    /// One cut selector as a node, refusing a selector that names none or more than
    /// one face.
    /// 把一个切口选择器变成节点；点名零个面或不止一个面的选择器一律拒绝。
    fn resolve_cut_selector(&self, selector: GraftTargetRef<'_>) -> RegistryResult<NodeId> {
        match self.resolve_target(selector) {
            Resolution::One(id) => Ok(id),
            // The cut selector, not the base root id, is the failing identity.
            // 失败的身份是切口选择器，而不是基树根 id。
            Resolution::Missing => Err(self.graft_selector_error(
                self.header.id,
                GraftError::UnknownTarget(self.header.id),
                format!("graft target `{}` is not registered", selector.describe()),
            )),
            Resolution::Ambiguous(matches) => {
                let described = selector.describe();
                Err(self.graft_selector_error(
                    self.header.id,
                    GraftError::AmbiguousReplacement {
                        selector: described.clone(),
                        matches,
                    },
                    format!(
                        "graft target `{described}` matches {matches} registered faces; use the face's NODE_ID"
                    ),
                ))
            }
        }
    }

    /// Expand a cut selector into every node it covers, in sibling order.
    /// 把切口选择器展开成它覆盖的每个节点，按兄弟顺序排列。
    ///
    /// "Sibling order" is **registry-name order**, not file order, registration
    /// order, or the order the tree renders. The span is the contiguous run of
    /// siblings between the two endpoints once they are sorted by
    /// `registry_name` — the same sort the sibling listing and the outline use.
    /// This is not observable from the folder layout: a directory that lists
    /// `a3, a1, a2` still has `a1 to a3` cover exactly those three faces.
    /// “兄弟顺序”是 **registry 名顺序**，不是文件顺序、注册顺序或树渲染顺序。跨度是
    /// 两个端点按 `registry_name` 排序后位于两者之间、连续的那一段——与兄弟列表和
    /// 大纲使用的是同一种排序。这一点无法从目录布局看出：一个列成 `a3, a1, a2` 的
    /// 目录，`a1 to a3` 仍然恰好覆盖这三个面。
    ///
    /// Both endpoints must share one parent, and the start must sort before the
    /// end. A backwards range is an error rather than a silent swap: swapping
    /// would select the same set while hiding that the author's mental model
    /// (tree order) is not the one that decides, so the next range they write in
    /// that model would quietly cover the wrong faces.
    /// 两个端点必须同属一个父级，且起点必须排在终点之前。写反的区间是错误而不是静默
    /// 交换：交换会选中同一个集合，却掩盖了作者的心智模型（树序）并不是决定顺序的那
    /// 一个，于是他们按该模型写的下一个区间会悄悄覆盖错误的面。
    pub(super) fn resolve_cut_targets(&self, cut: GraftCutRef<'_>) -> RegistryResult<Vec<NodeId>> {
        let start = self.resolve_cut_selector(cut.cut)?;
        let Some(end_target) = cut.end else {
            return Ok(vec![start]);
        };
        let end = match self.resolve_target(end_target) {
            Resolution::One(id) => id,
            // `start` is a *resolved* node; putting it in the `UnknownTarget`
            // payload would claim the wrong face is missing. The range end is
            // the selector that failed, so name it (and the cut it closes).
            // `start` 是**已解析**的节点；把它塞进 `UnknownTarget` 载荷会谎报缺失的是另一个
            // 面。真正失败的是区间终点选择器，因此报出它（以及它所闭合的切口）。
            Resolution::Missing => {
                return Err(self.graft_selector_error(
                    start,
                    GraftError::UnknownTarget(start),
                    format!(
                        "graft range end `{}` in cut `{}` is not registered",
                        end_target.describe(),
                        cut.cut.describe()
                    ),
                ));
            }
            Resolution::Ambiguous(matches) => {
                return Err(self.graft_selector_error(
                    start,
                    GraftError::AmbiguousReplacement {
                        selector: end_target.describe(),
                        matches,
                    },
                    format!(
                        "graft range end `{}` in cut `{}` matches {matches} registered faces; use the face's NODE_ID",
                        end_target.describe(),
                        cut.cut.describe()
                    ),
                ));
            }
        };
        let start_info = self.find_registry(start).expect("resolved cut start");
        let end_info = self.find_registry(end).expect("resolved cut end");
        if start_info.parent != end_info.parent {
            return Err(self.graft_error(
                start,
                GraftError::InvalidRange(format!(
                    "graft range `{}` must share one parent",
                    end_target.describe()
                )),
            ));
        }
        let parent = self
            .registry(start_info.parent)
            .ok_or_else(|| self.graft_error(start, GraftError::UnknownTarget(start_info.parent)))?;
        let mut siblings = parent
            .entries
            .values()
            .map(|entry| (entry.info.registry_name.as_str(), entry.info.id))
            .collect::<Vec<_>>();
        siblings.sort_unstable_by_key(|(name, _)| *name);
        // Both positions are infallible: the endpoints were resolved through
        // this tree and the parent check above proved they are entries of this
        // same parent. Defaulting to `0`/`first` instead would silently widen the
        // span to the parent's first sibling, which is a wrong-tree graft rather
        // than a visible failure.
        // 两个位置都不会失败：端点是在这棵树里解析出来的，上面的父级检查又证明它们是
        // 同一个父级的条目。若改成回退到 `0`/`first`，跨度会静默扩到父级第一个兄弟，
        // 那是错误的树，而不是可见的失败。
        let first = siblings
            .iter()
            .position(|(_, id)| *id == start)
            .expect("a resolved cut start is an entry of its own parent");
        let last = siblings
            .iter()
            .position(|(_, id)| *id == end)
            .expect("a resolved cut end shares the start's parent");
        if last < first {
            return Err(self.graft_error(
                start,
                GraftError::InvalidRange(format!(
                    "graft range `{}` to `{}` is written backwards: siblings order by registry name, where `{}` comes before `{}`; write the range as `{}` to `{}`",
                    cut.cut.describe(),
                    end_target.describe(),
                    end_target.describe(),
                    cut.cut.describe(),
                    end_target.describe(),
                    cut.cut.describe()
                )),
            ));
        }
        Ok(siblings[first..=last].iter().map(|(_, id)| *id).collect())
    }

    /// The first sibling whose registry name, kind, or path equals `value`.
    /// 第一个 registry 名、kind 或路径等于 `value` 的兄弟节点。
    pub(super) fn resolve_node(&self, value: &str) -> Resolution {
        if let Ok(id) = value.parse::<NodeId>() {
            return if self.find_registry(id).is_some() {
                Resolution::One(id)
            } else {
                Resolution::Missing
            };
        }
        let mut matches = self
            .depth_first()
            .into_iter()
            .filter(|info| {
                self.path_for(info.id).is_some_and(|path| {
                    info.registry_name == value || info.kind == value || path == value
                })
            })
            .map(|info| info.id);
        let Some(first) = matches.next() else {
            return Resolution::Missing;
        };
        match matches.count() {
            0 => Resolution::One(first),
            // Any one of them would be a silent, arbitrary choice, and renaming
            // an unrelated file could flip which implementation occupies the
            // slot. The caller has to say which face it meant.
            // 任选其一都是静默且随意的选择，而且重命名一个无关文件就可能翻转谁占住这个
            // 槽位。调用方必须说明它指的是哪个面。
            extra => Resolution::Ambiguous(extra + 1),
        }
    }

    /// Rewrite the parent of every descendant copied from the external tree.
    /// 改写从外部树复制来的每个后代的父链。
    pub(super) fn rebase_parent_ids(&mut self, old: NodeId, new: NodeId) {
        for entry in Arc::make_mut(&mut self.entries).values_mut() {
            if entry.info.parent == old {
                Arc::make_mut(&mut entry.info).parent = new;
            }
            if let Some(child) = entry.child.as_mut() {
                Arc::make_mut(child).rebase_parent_ids(old, new);
            }
        }
    }

    /// Re-root every copied path from `old_prefix` onto `new_prefix`.
    /// 把每条复制来的路径从 `old_prefix` 重新挂到 `new_prefix` 下。
    pub(super) fn rebase_paths(&mut self, old_prefix: &str, new_prefix: &str) {
        if let Some(suffix) = self.header.path.strip_prefix(old_prefix) {
            let next_path = format!("{new_prefix}{suffix}");
            Arc::make_mut(&mut self.header).path = next_path;
        }
        for entry in Arc::make_mut(&mut self.entries).values_mut() {
            if let Some(child) = entry.child.as_mut() {
                Arc::make_mut(child).rebase_paths(old_prefix, new_prefix);
            }
        }
    }

    /// Replace identity and contracts without touching the owned path.
    /// 只替换身份与合同，不动它自己拥有的路径。
    pub(super) fn reconfigure(&mut self, info: &RegistrationSnapshot) {
        let header = Arc::make_mut(&mut self.header);
        header.namespace = info.namespace.clone();
        header.id = info.id;
        // The path is owned by the registry node and is updated separately by
        // `rebase_paths`; reconfigure only replaces identity and contracts.
        // 路径归注册机节点所有，由 `rebase_paths` 单独更新；reconfigure 只替换身份与合同。
        header.registration_rule = info.registry_rule.clone();
        header.registration_rule_path = info.registry_rule_path.clone();
        header.admission = info.admission.clone();
    }
}

#[cfg(test)]
#[path = "resolution_tests.rs"]
mod resolution_tests;
