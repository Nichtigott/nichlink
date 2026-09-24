//! Slider 叶子面：Control 的第二个直接子对象，用来演示同一父级下的兄弟区间替换。
//! Slider leaf face: Control's second direct child, used to demonstrate a graft
//! over a contiguous range of siblings under one parent.

use crate::control::{ControlFrame, ControlHandle};
use nichlink_run_method::{ContractId, FlowContract};

pub struct Slider;

impl ControlHandle for Slider {
    fn paint(&self) -> ControlFrame {
        ControlFrame
    }
}

crate::control_object! {
    kind: Slider,
    exports: ["control.render"],
    parent: crate::control::NODE_ID,
    handle_contracts: [crate::control::ControlHandle],
    flow: FlowContract::new(
        ContractId::new("control.render.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ),
}
