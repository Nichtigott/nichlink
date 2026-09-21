//! Control 文件夹面：它拥有一个 Registry，所有直接子对象都要满足它旁边的规则。
//! Control folder face: it owns a Registry, and every direct child must satisfy
//! the rule kept beside it.

use crate::control::registry_rule::REGISTRATION_RULE;

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
    handle: Control,
    needs_registry: true,
    parent: crate::root_node_id(env!("CARGO_PKG_NAME")),
    registry_rule_path: "src/control/registry_rule/registry_rule.rs",
    registry_rule: REGISTRATION_RULE,
}
