//! 替换 `Slider` 的项目外实现，用于兄弟区间切口。
//! An out-of-project replacement for `Slider`, used by a sibling-range cut.

use nichlink_run_method::registry_core::{
    ContractId, FlowContract, NoParts, NoPreset, RegistrationRule, root_node_id,
};

pub struct SliderFast;

nichlink_run_method::external_object! {
    source: "slider_fast/slider_fast.rs",
    kind: SliderFast,
    preset: NoPreset,
    parts: NoParts,
    name: { zh: "快速滑块", en: "Fast slider" },
    summary: { zh: "项目外实现", en: "Out-of-project implementation" },
    exports: ["control.render"],
    needs_registry: false,
    parent: root_node_id(env!("CARGO_PKG_NAME")),
    getting_from_other_registry: None,
    registry_rule_path: "slider_fast/slider_fast.rs",
    registry_rule: RegistrationRule::ANY,
    handle_traits: ["ControlHandle"],
    requires: [],
    provides: [],
    flow: FlowContract::new(
        ContractId::new("control.render.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ),
    runtime_checks: [],
}
