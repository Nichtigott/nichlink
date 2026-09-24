//! Tests for the branch port surface.
//! 分支端口面的测试。
//!
//! They live beside `ports.rs` rather than inside it for the same reason
//! `record_tests.rs` does: an inline test module would push the production file
//! over its budget and drown the rules it exists to state. The worked example
//! here is the tree from the design discussion, as plain data.
//! 它们和 `record_tests.rs` 出于同样的理由放在 `ports.rs` 旁边而不是里面：内联测试模块
//! 会把生产文件顶出预算，并淹没它本要陈述的规则。这里的算例就是设计讨论中的那棵树，以纯数据
//! 形式给出。

use super::*;

/// One face of the worked example, by hand.
/// 手工构造示例树里的一个注册面。
fn face(path: &str, name: &str, provides: &[&str]) -> DeclaredPort {
    DeclaredPort {
        node: NodeId::from_namespaced_path("app", &format!("{path}.rs"), name),
        path: path.to_owned(),
        registry_name: name.to_owned(),
        provides: provides.iter().map(|port| (*port).to_owned()).collect(),
    }
}

/// The tree from the design discussion:
/// `root -> A1 -> B1 -> {C1, C2}` and `root -> A2 -> {B2 -> {C3, C4}, B3}`,
/// with `C1` providing `render` and `C4` providing `layout`.
/// 设计讨论里的那棵树：
/// `root -> A1 -> B1 -> {C1, C2}` 与 `root -> A2 -> {B2 -> {C3, C4}, B3}`，
/// 其中 `C1` 提供 `render`，`C4` 提供 `layout`。
fn worked_example() -> PortIndex {
    PortIndex::new(vec![
        face("root/A1", "A1", &[]),
        face("root/A1/B1", "B1", &[]),
        face("root/A1/B1/C1", "C1", &["render"]),
        face("root/A1/B1/C2", "C2", &[]),
        face("root/A2", "A2", &[]),
        face("root/A2/B2", "B2", &["own"]),
        face("root/A2/B2/C3", "C3", &[]),
        face("root/A2/B2/C4", "C4", &["layout"]),
        face("root/A2/B3", "B3", &[]),
    ])
}

/// A branch is named by its own `registry_name` and searched over its whole
/// subtree, so a reference from a sibling branch resolves without naming
/// any intermediate node.
/// 分支用自身的 `registry_name` 命名，并在其整棵子树内搜索，因此来自兄弟分支的引用不必
/// 命名任何中间节点就能解析。
#[test]
fn a_branch_exposes_its_whole_subtree_by_its_own_name() {
    let index = worked_example();
    assert_eq!(
        index.resolve("B2", "layout"),
        Resolution::One {
            node: face("root/A2/B2/C4", "C4", &[]).node,
            path: "root/A2/B2/C4".to_owned(),
        }
    );
    // The branch's own declaration counts too, and equality-inclusive
    // matching is what makes that true.
    // 分支自身的声明也算，而"包含相等"的匹配正是这一点的前提。
    assert!(matches!(index.resolve("B2", "own"), Resolution::One { .. }));
    // A branch does not see through its siblings.
    // 分支看不穿它的兄弟。
    assert_eq!(
        index.resolve("B1", "layout"),
        Resolution::UnknownPort {
            branch: "B1".to_owned()
        }
    );
    // And a top-level branch is addressable the same way as a nested one.
    // 顶层分支的寻址方式与嵌套分支完全相同。
    assert!(matches!(
        index.resolve("A1", "render"),
        Resolution::One { .. }
    ));
}

/// Ambiguity is reported with both paths, at the branch handle and at the
/// port, and never resolved by an implicit winner.
/// 歧义在分支句柄与端口两处都带着两条路径报出，绝不由隐式胜出者解析。
#[test]
fn ambiguity_is_reported_rather_than_resolved() {
    let mut entries = vec![
        face("root/A2/B2", "B2", &[]),
        face("root/A2/B2/C3", "C3", &["render"]),
        face("root/A2/B2/C4", "C4", &["render"]),
    ];
    let index = PortIndex::new(entries.clone());
    assert_eq!(
        index.resolve("B2", "render"),
        Resolution::AmbiguousPort {
            paths: vec!["root/A2/B2/C3".to_owned(), "root/A2/B2/C4".to_owned()],
        }
    );

    // The same handle appearing twice is ambiguous before the port is even
    // considered.
    // 同一个句柄出现两次时，还没看端口就已经有歧义。
    entries.push(face("root/A3/B2", "B2", &["render"]));
    let index = PortIndex::new(entries);
    assert!(matches!(
        index.resolve("B2", "render"),
        Resolution::AmbiguousBranch { .. }
    ));
    assert_eq!(
        index.ambiguous_branch_names(),
        vec![AmbiguousBranchName {
            name: "B2".to_owned(),
            paths: vec!["root/A2/B2".to_owned(), "root/A3/B2".to_owned()],
        }]
    );
}

/// The exposed surface keeps collisions in place, sorted, so a renderer
/// cannot publish an arbitrary winner by accident.
/// 暴露面保留冲突并按序输出，因此渲染器不会意外发布一个随意的胜出者。
#[test]
fn the_exposed_surface_keeps_collisions_visible() {
    let index = PortIndex::new(vec![
        face("root/A2/B2", "B2", &["own"]),
        face("root/A2/B2/C3", "C3", &["render"]),
        face("root/A2/B2/C4", "C4", &["layout", "render"]),
    ]);
    let ports: Vec<(String, String)> = index
        .exposed_ports("B2")
        .into_iter()
        .map(|exposed| (exposed.port, exposed.path))
        .collect();
    assert_eq!(
        ports,
        vec![
            ("layout".to_owned(), "root/A2/B2/C4".to_owned()),
            ("own".to_owned(), "root/A2/B2".to_owned()),
            ("render".to_owned(), "root/A2/B2/C3".to_owned()),
            ("render".to_owned(), "root/A2/B2/C4".to_owned()),
        ]
    );
}

/// The walker reads the tree's retained declarations, so an index built
/// from a real registry sees what a host registered.
/// 遍历器读取树保留的声明，因此由真实注册机构建的索引能看到宿主注册了什么。
#[test]
fn the_walker_reads_the_retained_declarations() {
    use crate::registry_core::declaration::{
        Admission, FrameworkId, OwnedLocalizedText, OwnedObjectContract, OwnedSourceLocation,
        RegistrationRule, RegistrationSnapshot,
    };
    use crate::registry_core::identity::root_node_id;

    let namespace = "port-index-test";
    let mut registry = Registry::root_for_namespace(FrameworkId::new("port-test"), namespace);
    let mut snapshot = RegistrationSnapshot {
        namespace: namespace.to_owned(),
        id: NodeId::from_namespaced_path(namespace, "src/button.rs", "Button"),
        parent: root_node_id(namespace),
        kind: "Button".to_owned(),
        preset: "NoPreset".to_owned(),
        parts: "NoParts".to_owned(),
        params: "Button".to_owned(),
        handle: "Button".to_owned(),
        stable_name: None,
        name: OwnedLocalizedText {
            zh: "按钮".to_owned(),
            en: "Button".to_owned(),
        },
        summary: OwnedLocalizedText {
            zh: String::new(),
            en: String::new(),
        },
        exports: Vec::new(),
        needs_registry: false,
        registry_name: "button".to_owned(),
        getting_from_other_registry: None,
        registry_rule_path: "<test>".to_owned(),
        registry_rule: RegistrationRule::ANY.into_owned(),
        admission: Admission::ANY.into_owned(),
        requires: Vec::new(),
        provides: vec!["paint".to_owned()],
        contract: OwnedObjectContract {
            required_parts: Vec::new(),
            provided_parts: Vec::new(),
        },
        flow: crate::registry_core::declaration::OwnedFlowContract::none(),
        flow_provider: None,
        handle_traits: Vec::new(),
        part_traits: Vec::new(),
        runtime_checks: Vec::new(),
        plugin: None,
        source: OwnedSourceLocation {
            file: "src/button.rs".to_owned(),
            line: 1,
            column: 1,
            function: "Button".to_owned(),
        },
    };
    snapshot.parent = registry.header.id;
    registry
        .register_snapshot_batch([snapshot])
        .expect("the face registers");

    let index = registry.port_index();
    assert!(matches!(
        index.resolve("button", "paint"),
        Resolution::One { .. }
    ));
    assert_eq!(
        index.resolve("button", "absent"),
        Resolution::UnknownPort {
            branch: "button".to_owned()
        }
    );
}
