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
    fn resolve_target(&self, target: GraftTargetRef<'_>) -> Option<NodeId> {
        match target {
            GraftTargetRef::Path(path) => self.resolve_path(path),
            GraftTargetRef::Id(id) => self.find(id).map(|_| id),
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
        let start = self.resolve_target(cut.cut).ok_or_else(|| {
            // The cut selector, not the base root id, is the failing identity.
            // 失败的身份是切口选择器，而不是基树根 id。
            self.graft_selector_error(
                self.header.id,
                GraftError::UnknownTarget(self.header.id),
                format!("graft target `{}` is not registered", cut.cut.describe()),
            )
        })?;
        let Some(end_target) = cut.end else {
            return Ok(vec![start]);
        };
        let end = self.resolve_target(end_target).ok_or_else(|| {
            // `start` is a *resolved* node; putting it in the `UnknownTarget`
            // payload would claim the wrong face is missing. The range end is
            // the selector that failed, so name it (and the cut it closes).
            // `start` 是**已解析**的节点；把它塞进 `UnknownTarget` 载荷会谎报缺失的是另一个
            // 面。真正失败的是区间终点选择器，因此报出它（以及它所闭合的切口）。
            self.graft_selector_error(
                start,
                GraftError::UnknownTarget(start),
                format!(
                    "graft range end `{}` in cut `{}` is not registered",
                    end_target.describe(),
                    cut.cut.describe()
                ),
            )
        })?;
        let start_info = self.find(start).expect("resolved cut start");
        let end_info = self.find(end).expect("resolved cut end");
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
            return if self.find(id).is_some() {
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
mod tests {
    use super::*;
    use crate::registry_core::plugin::graft::{GraftCut, GraftPlan};

    use super::super::fixtures::{FRAMEWORK, face};

    /// Two distinct faces can carry the same slot name. A string selector that
    /// matches both must be refused instead of silently choosing one, because
    /// which file happens to come first is not a decision the author made.
    /// 两个不同的面可以带同一个槽位名。匹配到两者的字符串选择器必须被拒绝，而不是静默
    /// 选一个——文件谁先出现并不是作者做出的决定。
    #[test]
    fn an_ambiguous_replacement_selector_is_refused() {
        let namespace = "ambiguous";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let mut base = face(namespace, "a.rs", "A", "a");
        base.needs_registry = true;
        base.id = NodeId::from_namespaced_path(namespace, "a.rs", "A");
        root.register_snapshot_batch([base.clone()]).unwrap();

        let mut external = Registry::root_for_namespace(FRAMEWORK, "external");
        let mut first = face("external", "first.rs", "First", "replacement");
        first.id = NodeId::from_namespaced_path("external", "first.rs", "First");
        let mut second = face("external", "second.rs", "Second", "replacement");
        second.id = NodeId::from_namespaced_path("external", "second.rs", "Second");
        external
            .register_snapshot_batch([first.clone(), second.clone()])
            .unwrap();

        let plan = GraftPlan::new(FRAMEWORK).cut("root/a", "replacement");
        let error = root
            .overlay(&plan, &external)
            .expect_err("an ambiguous selector must be refused");
        let rendered = format!("{error}");
        assert!(rendered.contains("matches 2"), "{rendered}");
        assert!(rendered.contains("replacement"), "{rendered}");
    }

    /// An unresolvable cut selector must be reported by the text the author
    /// wrote, not by the base tree's root id: the root id names a face that is
    /// present, so the old message pointed at the wrong thing entirely.
    /// 无法解析的切口选择器必须按作者写下的文本报出，而不是基树根 id：根 id 指的是一个
    /// 确实存在的面，旧消息因此指错了对象。
    #[test]
    fn an_unknown_cut_target_names_the_selector() {
        let namespace = "unknown-cut";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let target = face(namespace, "a.rs", "A", "a");
        root.register_snapshot_batch([target]).unwrap();
        let mut external = Registry::root_for_namespace(FRAMEWORK, "external");
        let replacement = face("external", "replacement.rs", "Replacement", "replacement");
        external.register_snapshot_batch([replacement]).unwrap();

        let plan = GraftPlan::new(FRAMEWORK).cut("root/absent", "replacement");
        let error = root
            .overlay(&plan, &external)
            .expect_err("an absent cut target must be refused");
        let rendered = format!("{error}");
        assert!(rendered.contains("root/absent"), "{rendered}");
        assert!(rendered.contains("graft target"), "{rendered}");
    }

    /// The failing selector of a range is the end, not the already-resolved
    /// start; naming the start would blame a face that is present.
    /// 区间失败的选择器是终点，而不是已经解析成功的起点；报出起点会怪罪一个存在的面。
    #[test]
    fn an_unknown_cut_range_end_names_the_end_selector() {
        let namespace = "unknown-range-end";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let first = face(namespace, "a.rs", "A", "a");
        root.register_snapshot_batch([first]).unwrap();
        let mut external = Registry::root_for_namespace(FRAMEWORK, "external");
        let replacement = face("external", "replacement.rs", "Replacement", "replacement");
        external.register_snapshot_batch([replacement]).unwrap();

        let mut plan = GraftPlan::new(FRAMEWORK);
        plan.cuts
            .push(GraftCut::range("root/a", "root/absent", "replacement"));
        let error = root
            .overlay(&plan, &external)
            .expect_err("an absent range end must be refused");
        let rendered = format!("{error}");
        assert!(rendered.contains("root/absent"), "{rendered}");
        assert!(rendered.contains("range end"), "{rendered}");
        assert!(rendered.contains("in cut"), "{rendered}");
    }

    /// A range covers the contiguous run of siblings in **registry-name** order,
    /// which is neither the order the faces were registered in nor the order
    /// their files sit on disk.
    /// 区间覆盖的是按 **registry 名** 顺序连续的那一段兄弟，既不是注册顺序，也不是
    /// 文件在磁盘上的顺序。
    #[test]
    fn a_range_covers_the_siblings_between_its_endpoints_in_registry_name_order() {
        let namespace = "range-order";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        // Registered as beta, alpha, mid: registration order is deliberately not
        // name order, so a span that followed registration order would cover a
        // different set.
        // 注册顺序是 beta、alpha、mid：故意让它与名字顺序不同，因此按注册顺序展开的
        // 跨度会覆盖另一个集合。
        let beta = face(namespace, "beta.rs", "Beta", "beta");
        let alpha = face(namespace, "alpha.rs", "Alpha", "alpha");
        let mid = face(namespace, "mid.rs", "Mid", "mid");
        let outside = face(namespace, "zeta.rs", "Zeta", "zeta");
        root.register_snapshot_batch([beta.clone(), alpha.clone(), mid.clone(), outside.clone()])
            .unwrap();

        // Name order is `alpha, beta, mid, zeta`, so the span keeps those three
        // siblings in that order. Registration order was `beta, alpha, mid,
        // zeta`: a span that followed it would start at `beta`, and the order it
        // returned would differ too.
        // 名字顺序是 `alpha, beta, mid, zeta`，因此跨度按该顺序保留这三个兄弟。注册
        // 顺序曾是 `beta, alpha, mid, zeta`：按它展开的跨度会从 `beta` 开始，返回的
        // 顺序也不同。
        let cut = GraftCut::range("root/alpha", "root/mid", "replacement");
        let targets = root
            .resolve_cut_targets(GraftCutRef::dynamic(&cut))
            .expect("both endpoints are registered");
        assert_eq!(targets, vec![alpha.id, beta.id, mid.id]);
        assert!(
            !targets.contains(&outside.id),
            "a name-ordered span between `alpha` and `mid` excludes `zeta`"
        );
    }

    /// A range written from the later sibling to the earlier one is refused, not
    /// silently swapped: the span is ordered by registry name, and a swap would
    /// hide that the order the author had in mind is not the one that decides.
    /// 从靠后的兄弟写到靠前的兄弟会被拒绝，而不是静默交换：跨度按 registry 名排序，
    /// 交换会掩盖"作者心里的顺序不是决定顺序的那一个"。
    #[test]
    fn a_backwards_range_is_refused_with_both_endpoints_and_the_order_rule() {
        let namespace = "range-backwards";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let alpha = face(namespace, "alpha.rs", "Alpha", "alpha");
        let mid = face(namespace, "mid.rs", "Mid", "mid");
        root.register_snapshot_batch([mid, alpha]).unwrap();

        let cut = GraftCut::range("root/mid", "root/alpha", "replacement");
        let error = root
            .resolve_cut_targets(GraftCutRef::dynamic(&cut))
            .expect_err("a backwards range must be refused");
        let rendered = format!("{error}");
        assert!(rendered.contains("root/mid"), "{rendered}");
        assert!(rendered.contains("root/alpha"), "{rendered}");
        assert!(rendered.contains("written backwards"), "{rendered}");
        assert!(rendered.contains("registry name"), "{rendered}");
        assert!(rendered.contains("write the range as"), "{rendered}");
    }

    #[test]
    fn full_cut_inherits_external_subtree_without_moving_source_trees() {
        let namespace = "overlay-full";
        let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
        let mut target = face(namespace, "base/a.rs", "BaseA", "a");
        target.needs_registry = true;
        target.id = NodeId::from_namespaced_path(namespace, "base/a.rs", "BaseA");
        let mut base_child = face(namespace, "base/old.rs", "OldChild", "old");
        base_child.parent = target.id;
        let sibling = face(namespace, "base/sibling.rs", "Sibling", "sibling");
        root.register_snapshot_batch([target.clone(), base_child.clone(), sibling.clone()])
            .unwrap();

        let external_namespace = "overlay-full-external";
        let mut replacement = face(external_namespace, "graft/fast_a.rs", "FastA", "fast_a");
        replacement.needs_registry = true;
        replacement.id =
            NodeId::from_namespaced_path(external_namespace, "graft/fast_a.rs", "FastA");
        let mut external_child = face(external_namespace, "graft/new.rs", "NewChild", "new");
        external_child.parent = replacement.id;
        let mut external = Registry::root_for_namespace(FRAMEWORK, external_namespace);
        external
            .register_snapshot_batch([replacement.clone(), external_child.clone()])
            .unwrap();

        let plan = GraftPlan::new(FRAMEWORK).cut("root/a", "fast_a");
        let single = root.overlay(&plan, &external).unwrap();
        assert!(!single.find_kind("OldChild").is_empty());
        assert_eq!(single.find_kind("NewChild").len(), 0);

        let full_plan = GraftPlan::new(FRAMEWORK);
        let full_plan = GraftPlan {
            framework: full_plan.framework,
            cuts: vec![GraftCut::subtree("root/a", "fast_a")],
        };
        let effective = root.overlay(&full_plan, &external).unwrap();
        assert_eq!(effective.find_kind("FastA").len(), 1);
        assert_eq!(effective.find_kind("OldChild").len(), 0);
        assert_eq!(effective.find_kind("NewChild").len(), 1);
        assert_eq!(
            effective.path_for(external_child.id).as_deref(),
            Some("root/a/new")
        );
        assert_eq!(
            effective.path_for(sibling.id).as_deref(),
            Some("root/sibling")
        );
        assert_eq!(root.find_kind("OldChild").len(), 1);
        assert_eq!(external.find_kind("NewChild").len(), 1);
    }
}
