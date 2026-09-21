//! 集成测试：把 README 那棵 Control / Button 树当成真实宿主，逐项验证构建期
//! 生成、注册、父级规则、graft、项目外实现和发布态静态计划。
//! Integration tests: treat the README Control/Button tree as a real host and
//! verify the generated plan, registration, the parent rule, grafting, the
//! out-of-project implementation, and the release-time static plan.

use control_button::{FRAMEWORK, base_registry, builtin_static_plan};
use control_button_graft::FRAMEWORK as GRAFT_FRAMEWORK;
use nichlink_run_method::registry_core::{
    FrameworkId, GraftPlan, NodeId, OwnedFlowContract, Registry,
};

/// 读取某一逻辑路径上的 kind；找不到就失败。
/// Read the kind at a logical path, failing when the path is absent.
fn kind_at(registry: &Registry, wanted: &str) -> String {
    registry
        .depth_first()
        .iter()
        .find(|info| registry.path_for(info.id).as_deref() == Some(wanted))
        .map(|info| info.kind.clone())
        .unwrap_or_else(|| panic!("no registration face at `{wanted}`"))
}

fn slot_plan() -> GraftPlan {
    GraftPlan::command(FRAMEWORK, "cut root/control/button graft button_fast")
        .expect("the graft command parses")
}

/// `source` 现在由声明点推导，但必须与旧方案注入的值完全相同。
/// `source` is derived at the declaration site now, and must equal what the
/// previous design injected.
#[test]
fn built_in_tree_has_the_expected_paths_and_derived_sources() {
    let registry = base_registry();
    let mut rows = registry
        .depth_first()
        .iter()
        .map(|info| {
            format!(
                "{} kind={} source={}",
                registry.path_for(info.id).unwrap_or_default(),
                info.kind,
                info.source.file
            )
        })
        .collect::<Vec<_>>();
    rows.sort();

    assert_eq!(
        rows,
        [
            "root/control kind=Control source=control/control.rs",
            "root/control/button kind=Button source=control/object/button/button.rs",
        ]
    );
}

/// 叶子面以自身名字载入，公开模块路径和类型名都不变。
/// A leaf face loads under its own name, so its public module path and type
/// name are unchanged.
#[test]
fn leaf_face_module_path_stays_clean() {
    assert_eq!(
        std::any::type_name::<control_button::control::object::button::Button>(),
        "control_button::control::object::button::Button"
    );
}

/// 文件夹面的条目仍按原有路径可寻址（同名子模块 + 重导出）。
/// A folder face's items stay addressable at their original path through the
/// same-named child module and its re-export.
#[test]
fn folder_face_items_stay_reachable() {
    let _: NodeId = control_button::control::NODE_ID;
    let _ = std::marker::PhantomData::<control_button::control::Control>;
    let _ = control_button::control::ControlFrame;
}

/// 面文件以 `//!` 开头（见 `src/control/control.rs`）仍然编译，并且源码路径
/// 就是仓库相对路径，而不是任何构建副本。
/// Face files keep their `//!` header (see `src/control/control.rs`), and the
/// recorded source is the repository-relative path rather than a build copy.
#[test]
fn face_sources_point_at_the_real_files() {
    let registry = base_registry();
    let sources: Vec<&str> = registry
        .depth_first()
        .iter()
        .map(|info| info.source.file.as_str())
        .collect();
    assert!(sources.contains(&"control/control.rs"));
    assert!(sources.contains(&"control/object/button/button.rs"));
    assert!(
        !sources
            .iter()
            .any(|source| source.contains("registration_sources")),
        "no materialised copy may appear in an identity: {sources:?}"
    );
}

/// 父级规则是最低结构要求：缺少必需 export 的子对象整批拒绝。
/// The parent rule is a minimum shape: a child missing a required export is
/// rejected as a batch.
#[test]
fn parent_rule_rejects_a_child_that_misses_a_required_export() {
    let mut registry = Registry::root_for_namespace(FRAMEWORK, env!("CARGO_PKG_NAME"));
    registry
        .register_all(&control_button::registrations())
        .expect("built-in faces register");

    let mut broken = control_button::control::object::button::REGISTRATION.into_snapshot();
    broken.kind = "BrokenButton".to_owned();
    broken.exports.clear();
    broken.id =
        NodeId::from_namespaced_path(&broken.namespace, &broken.source.file, "BrokenButton");

    let error = registry
        .register_snapshot_batch([broken])
        .expect_err("the parent rule rejects a child without `control.render`");
    let rendered = error.to_string();
    assert!(
        rendered.contains("control.render"),
        "the diagnostic must name the missing requirement: {rendered}"
    );
}

/// 父级规则不筛选 kind：满足结构的其它 kind 可以进入同一个 Registry。
/// The parent rule is not a kind filter: any shape that satisfies it may enter.
#[test]
fn parent_rule_admits_a_new_kind_that_satisfies_it() {
    let mut registry = base_registry();
    let mut sibling = control_button::control::object::button::REGISTRATION.into_snapshot();
    sibling.kind = "Toggle".to_owned();
    sibling.registry_name = "toggle".to_owned();
    sibling.id = NodeId::from_namespaced_path(&sibling.namespace, &sibling.source.file, "Toggle");

    registry
        .register_snapshot_batch([sibling])
        .expect("structure, not kind, decides admission");
    assert_eq!(kind_at(&registry, "root/control/toggle"), "Toggle");
}

/// 宿主与项目外 crate 必须共享同一个 framework，overlay 才接受外部树。
/// Host and out-of-project crate must share one framework for `overlay`.
#[test]
fn host_and_external_share_one_framework() {
    assert_eq!(FRAMEWORK, GRAFT_FRAMEWORK);
}

/// 项目外面由 `external_object!` 显式声明来源，不参与宿主的生成树。
/// An out-of-project face declares its origin explicitly and takes no part in
/// the host's generated tree.
#[test]
fn out_of_project_face_declares_its_own_source() {
    let external = control_button_graft::external_registry();
    let faces = external.depth_first();
    let face = faces
        .iter()
        .find(|info| info.kind == "ButtonFast")
        .expect("the external face registers into its own registry");

    assert_eq!(face.registry_name, "button_fast");
    assert_eq!(face.source.file, "button_fast/button_fast.rs");
    assert_eq!(face.namespace, "nichlink-example-control-button-graft");
    assert_eq!(
        external.path_for(face.id).as_deref(),
        Some("root/button_fast")
    );
}

/// graft 替换逻辑槽位，原树和外部树都不被修改。
/// Grafting replaces the logical slot; neither tree is modified.
#[test]
fn graft_replaces_the_slot_and_leaves_both_trees_untouched() {
    let base = base_registry();
    let external = control_button_graft::external_registry();

    let effective = base
        .overlay(&slot_plan(), &external)
        .expect("the overlay publishes");

    assert_eq!(kind_at(&effective, "root/control/button"), "ButtonFast");
    assert_eq!(kind_at(&base, "root/control/button"), "Button");
    assert_eq!(kind_at(&external, "root/button_fast"), "ButtonFast");
    // The slot keeps its logical path; only the implementation changed.
    // 槽位保留逻辑路径，换掉的只是实现。
    assert_eq!(
        kind_at(&effective, "root/control"),
        "Control",
        "the parent face survives the replacement"
    );
}

/// 数据流合同不兼容时，整组计划不发布。
/// When the flow contract is incompatible, nothing is published.
#[test]
fn graft_rejects_an_incompatible_flow_contract() {
    let base = base_registry();
    let external = external_registry_with_flow("control.render.v1", 2);

    let error = base
        .overlay(&slot_plan(), &external)
        .expect_err("a different flow version must be refused");
    let rendered = error.to_string();
    assert!(
        rendered.contains("control.render.v1"),
        "the diagnostic must name the contract: {rendered}"
    );
    assert_eq!(kind_at(&base, "root/control/button"), "Button");
}

/// framework 不同时，外部树不会被接受。
/// A foreign framework is never accepted.
#[test]
fn graft_rejects_a_foreign_framework() {
    let base = base_registry();
    let plan = GraftPlan::command(
        FrameworkId::new("other.framework"),
        "cut root/control/button graft button_fast",
    )
    .expect("the graft command parses");

    let external = control_button_graft::external_registry();
    assert!(
        base.overlay(&plan, &external).is_err(),
        "overlay must reject a tree from another framework"
    );
}

/// 入口声明的 graft 计划被固化成发布态静态表。
/// The plan declared at the entry is frozen into the release-time static table.
#[test]
fn static_plan_carries_faces_and_the_declared_graft() {
    let plan = builtin_static_plan();

    assert_eq!(plan.faces().len(), 2, "both built-in faces are retained");
    assert_eq!(plan.grafts().len(), 1, "the entry declared one graft");
    let cut = plan.grafts()[0];
    assert_eq!(
        cut.cut().id(),
        Some(control_button::control::object::button::NODE_ID),
        "the typed cut names the host face by compile-time identity"
    );
    assert_eq!(
        cut.graft().id(),
        Some(control_button_graft::button_fast::NODE_ID),
        "the typed graft names the external face by compile-time identity"
    );
    assert!(!cut.full(), "a plain cut keeps the target's children");
}

/// 发布路径用静态选择器 overlay，不需要构造动态计划。
/// The release path overlays straight from the static selectors.
#[test]
fn release_path_overlays_the_declared_static_graft() {
    let base = base_registry();
    let external = control_button_graft::external_registry();

    let effective = base
        .overlay_static(builtin_static_plan().grafts(), &external)
        .expect("the static overlay publishes");

    assert_eq!(kind_at(&effective, "root/control/button"), "ButtonFast");
    assert_eq!(kind_at(&base, "root/control/button"), "Button");
}

/// 外部面带上一个不同的合同版本，用来验证 overlay 的合同比较。
/// An external face carrying a different contract version, used to exercise the
/// overlay's contract comparison.
fn external_registry_with_flow(id: &str, version: u32) -> Registry {
    let mut snapshot = control_button_graft::button_fast::REGISTRATION.into_snapshot();
    snapshot.flow = OwnedFlowContract {
        id: id.to_owned(),
        version,
        input: "ControlInput".to_owned(),
        output: "ControlFrame".to_owned(),
    };
    let namespace = snapshot.namespace.clone();
    let mut registry = Registry::root_for_namespace(FRAMEWORK, namespace);
    registry
        .register_snapshot_batch([snapshot])
        .expect("the modified external face registers");
    registry
}
