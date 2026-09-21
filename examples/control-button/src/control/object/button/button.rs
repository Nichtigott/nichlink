//! Button 叶子面：它是 Control 的直接子对象，不再拥有自己的 Registry。
//! Button leaf face: a direct child of Control that owns no Registry of its own.

use crate::control::{ControlFrame, ControlHandle};
use nichlink_run_method::{ContractId, FlowContract};

pub struct Button;

impl ControlHandle for Button {
    fn paint(&self) -> ControlFrame {
        ControlFrame
    }
}

crate::control_object! {
    kind: Button,
    exports: ["control.render"],
    handle: Button,
    parent: crate::control::NODE_ID,
    handle_traits: ["ControlHandle"],
    handle_contracts: [crate::control::ControlHandle],
    expected_output: "ControlFrame",
    actual_output: "ControlFrame",
    flow: FlowContract::new(
        ContractId::new("control.render.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ),
}
