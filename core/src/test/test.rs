// generated-by=NichLink
//! Test face and its recursively requested registry.
//! Test 注册面及其递归申请的注册机。

use crate::{NoParts, NoPreset};

/// Registration-only handle marker for the Test face.
/// 仅用于 Test 注册面的 handle 标记，不代表运行时 object 实现。
pub struct Test;

crate::root_object! {
    kind: Test,
    preset: NoPreset,
    parts: NoParts,
    name: { zh: "test", en: "Test" },
    summary: { zh: "NichLink 创建的注册模块。", en: "A registration module created by NichLink." },
    params: "Test",
    exports: ["module.test"],
    handle: Test,
    needs_registry: true,
    registry_name: test,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
    getting_from_other_registry: None,
    registry_rule_path: "src/test/registry_rule/registry_rule.rs",
    registry_rule: crate::test::registry_rule::REGISTRATION_RULE,
    requires: [],
    provides: ["module.test"],
    expected_output: "()",
    actual_output: "()",
    runtime_checks: [],
}
