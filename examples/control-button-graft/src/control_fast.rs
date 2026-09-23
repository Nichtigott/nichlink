//! 替换整个 `Control` 子树的项目外实现。`full` 切口会丢弃原子树，改用这里携带的
//! 子树；合同必须与 `Control` 自己发布的 `control.frame.v1` 兼容。
//! An out-of-project replacement for the whole `Control` subtree. A `full` cut
//! discards the original subtree and uses the one carried here, so the contract
//! must stay compatible with the `control.frame.v1` that `Control` publishes.

use nichlink_run_method::registry_core::{
    ContractId, FlowContract, NoParts, NoPreset, RegistrationRule, root_node_id,
};

pub struct ControlFast;

nichlink_run_method::external_object! {
    source: "control_fast/control_fast.rs",
    kind: ControlFast,
    preset: NoPreset,
    parts: NoParts,
    name: { zh: "快速控件", en: "Fast control" },
    summary: { zh: "整体替换", en: "Whole-subtree replacement" },
    params: "ControlFast",
    exports: ["control.render"],
    handle: ControlFast,
    needs_registry: true,
    registry_name: control_fast,
    parent: root_node_id(env!("CARGO_PKG_NAME")),
    getting_from_other_registry: None,
    registry_rule_path: "control_fast/control_fast.rs",
    registry_rule: RegistrationRule::ANY,
    handle_traits: ["ControlHandle"],
    requires: [],
    provides: [],
    expected_output: "ControlFrame",
    actual_output: "ControlFrame",
    flow: FlowContract::new(
        ContractId::new("control.frame.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ),
    runtime_checks: [],
}
