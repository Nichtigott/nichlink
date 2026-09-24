//! 替换 `Button` 的外部实现。它以 `#[path]` 之外的普通模块方式载入，因此
//! 文件可以照常以 `//!` 开头，编辑器也能原生解析它。
//! The out-of-project replacement for `Button`. It loads as an ordinary module
//! (the author writes `mod button_fast;`), so the file keeps its `//!` header and
//! editor tooling resolves it natively.

use nichlink_run_method::registry_core::{
    ContractId, FlowContract, NoParts, NoPreset, RegistrationRule, root_node_id,
};

pub struct ButtonFast;

nichlink_run_method::external_object! {
    source: "button_fast/button_fast.rs",
    kind: ButtonFast,
    preset: NoPreset,
    parts: NoParts,
    name: { zh: "快速按钮", en: "Fast button" },
    summary: { zh: "项目外实现", en: "Out-of-project implementation" },
    exports: ["control.render"],
    needs_registry: false,
    parent: root_node_id(env!("CARGO_PKG_NAME")),
    getting_from_other_registry: None,
    registry_rule_path: "button_fast/button_fast.rs",
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
