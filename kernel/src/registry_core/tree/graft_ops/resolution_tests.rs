//! Tests for graft selector resolution and staged-identity rebasing.
//! 嫁接选择器解析与暂存身份重新定基的测试。

use super::*;
use crate::registry_core::plugin::graft::{GraftCut, GraftPlan};

use super::super::fixtures::{FRAMEWORK, face};

/// Two distinct faces can carry the same `registry_name`. A string selector
/// that matches both must be refused instead of silently choosing one, because
/// which file happens to come first is not a decision the author made.
/// 两个不同的面可以带同一个 `registry_name`。匹配到两者的字符串选择器必须被拒绝，而不是静默
/// 选一个——文件谁先出现并不是作者做出的决定。
///
/// The fixture puts the twins under *different* parents (cousins), because
/// `plan_batch` refuses same-parent twins now — and `resolve_node` answers for the
/// whole tree, so cousins are just as ambiguous for it. The intent is preserved:
/// the assertion is still "more than one match is reported", not "siblings may
/// collide" (audit `LGC-LG-07`).
/// 夹具把双胞胎放在**不同**父级下（堂兄弟），因为 `plan_batch` 现在拒绝同父级双胞胎——而
/// `resolve_node` 是整棵树一起回答的，因此堂兄弟对它与对兄弟一样多义。意图被保住：断言仍然是
/// "匹配到多个就报出"，而不是"兄弟可以撞名"（审计 `LGC-LG-07`）。
#[test]
fn an_ambiguous_replacement_selector_is_refused() {
    let namespace = "ambiguous";
    let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
    let mut base = face(namespace, "a.rs", "A", "a");
    base.needs_registry = true;
    base.id = NodeId::from_namespaced_path(namespace, "a.rs", "A");
    root.register_snapshot_batch([base.clone()]).unwrap();

    let mut external = Registry::root_for_namespace(FRAMEWORK, "external");
    let mut branch_a = face("external", "branch_a.rs", "BranchA", "branch-a");
    branch_a.needs_registry = true;
    let mut branch_b = face("external", "branch_b.rs", "BranchB", "branch-b");
    branch_b.needs_registry = true;
    external
        .register_snapshot_batch([branch_a.clone(), branch_b.clone()])
        .unwrap();
    let mut first = face("external", "first.rs", "First", "replacement");
    first.parent = branch_a.id;
    let mut second = face("external", "second.rs", "Second", "replacement");
    second.parent = branch_b.id;
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

/// The *cut* selector's path arm refuses a path two faces share. A collision stays
/// reachable after registration (an in-place edit may rename a face: that
/// validation checks namespace, identity, parent, rule and contract — not the
/// `registry_name`), so the arm that used to take the first match has to report the ambiguity.
/// Silently re-selecting a node is the hidden behaviour this workspace refuses, and
/// the first-match lookup it used to share with `resolve_record` is pinned right
/// here next to the strict reading, because that contrast is what makes the debt
/// decidable instead of prose (audit `LGC-LG-07`).
/// **切口**选择器的路径那一支拒绝两个面共用的路径。注册之后这种碰撞仍可达（就地编辑可以给
/// 面改名：那道校验检查命名空间、身份、父级、规则与契约——但不检查 `registry_name`），
/// 因此过去取第一个匹配的那一支必须报出多义。静默改选节点正是本工作区拒绝的隐性行为；
/// 而它过去与 `resolve_record` 共用的那个"取第一个匹配"查找就在这里与严格读法并列钉住，
/// 因为正是这个对照让欠账可判定，而不是散文。
#[test]
fn an_ambiguous_cut_path_is_refused() {
    let namespace = "ambiguous-path";
    let mut root = Registry::root_for_namespace(FRAMEWORK, namespace);
    let a = face(namespace, "a.rs", "A", "slot");
    let b = face(namespace, "b.rs", "B", "other");
    root.register_snapshot_batch([a.clone(), b.clone()])
        .unwrap();

    // Rename `other` onto `slot`, through the in-place edit path this workspace
    // still allows.
    // 把 `other` 改名到 `slot`，走本工作区仍然允许的就地编辑路径。
    let mut renamed = b.clone();
    renamed.registry_name = "slot".to_owned();
    root.apply_snapshot_replacement(b.id, renamed)
        .expect("an in-place rename is what makes two paths collide");

    assert_eq!(
        root.resolve_path_strict("root/slot"),
        Resolution::Ambiguous(2),
        "the strict reading counts both owners of the path"
    );
    assert!(
        root.resolve_path("root/slot").is_some(),
        "the legacy lookup still answers with one of them, and which one is not the point"
    );

    let mut external = Registry::root_for_namespace(FRAMEWORK, "external");
    let replacement = face("external", "replacement.rs", "Replacement", "replacement");
    external
        .register_snapshot_batch([replacement.clone()])
        .unwrap();
    let plan = GraftPlan::new(FRAMEWORK).cut("root/slot", "replacement");
    let error = root
        .overlay(&plan, &external)
        .expect_err("an ambiguous cut path must be refused");
    let rendered = format!("{error}");
    assert!(rendered.contains("matches 2"), "{rendered}");
    assert!(rendered.contains("root/slot"), "{rendered}");

    // Identity is never ambiguous: the same target named by its compile-time id
    // still resolves, which is the remedy the message points at.
    // 身份从不含糊：同一个目标用编译期 id 命名仍然解析——正是消息指向的补救办法。
    use crate::registry_core::release::StaticGraftCut;
    let by_id = [StaticGraftCut::from_ids(a.id, replacement.id, false)];
    assert!(
        root.overlay_static(&by_id, &external).is_ok(),
        "naming the target by identity resolves it: {rendered}"
    );
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
    external
        .register_snapshot_batch([replacement.clone()])
        .unwrap();

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
    external
        .register_snapshot_batch([replacement.clone()])
        .unwrap();

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
    replacement.id = NodeId::from_namespaced_path(external_namespace, "graft/fast_a.rs", "FastA");
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
