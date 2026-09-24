//! Control 文件夹面：它拥有一个 Registry，所有直接子对象都要满足它旁边的规则。
//! Control folder face: it owns a Registry, and every direct child must satisfy
//! the rule kept beside it.

use nichlink_run_method::{ContractId, FlowContract};

/// 父注册面交给子对象的绘制结果。
/// The frame a parent face hands to its children for painting.
pub struct ControlFrame;

/// 每个直接子对象必须实现的接口。
/// The interface every direct child must implement.
pub trait ControlHandle {
    fn paint(&self) -> ControlFrame;
}

pub struct Control;

crate::root_object! {
    kind: Control,
    needs_registry: true,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
    // 规则不再重复写第二遍：`needs_registry: true` 的面省略 `registry_rule:` 时，
    // 规则解析到注册面旁边那份规范规则（`super::registry_rule::REGISTRATION_RULE`）。
    // The rule is no longer written twice: a face with `needs_registry: true` that
    // omits `registry_rule:` resolves to the canonical rule beside the face
    // (`super::registry_rule::REGISTRATION_RULE`).
    registry_rule_path: "src/control/registry_rule/registry_rule.rs",
    // A folder face may publish its own flow contract, so a `full` cut can
    // replace the whole subtree only when the replacement agrees with it.
    // 文件夹面也可以发布自己的数据流合同；因此只有替换端与之兼容时，`full`
    // 切口才允许换掉整棵子树。
    flow: FlowContract::new(
        ContractId::new("control.frame.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ),
}
