//! 夹具的 Control 文件夹面：它拥有一个 Registry，所有直接子对象都必须满足旁边的规则。
//! Fixture Control folder face: it owns a Registry, and every direct child must
//! satisfy the rule kept beside it.

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
    // 省略 `registry_rule:` 时，拥有注册机的面解析到注册面旁边那份规范规则。
    // An omitted `registry_rule:` resolves to the canonical rule beside the face.
    registry_rule_path: "src/control/registry_rule/registry_rule.rs",
    flow: FlowContract::new(
        ContractId::new("control.frame.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ),
}
