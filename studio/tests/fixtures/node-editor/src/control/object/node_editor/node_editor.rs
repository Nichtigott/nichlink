//! 夹具的 NodeEditor 文件夹面：Control 的直接子对象，拥有自己的 Registry。
//! Fixture NodeEditor folder face: a direct child of Control that owns its own
//! Registry.
//!
//! 这里定义画布宽度的预览函数，以及它们对画布对象所做钳制的调用边。
//! This face defines the canvas-width preview functions and the call edges that
//! reach the canvas object's clamping function.

use crate::control::{ControlFrame, ControlHandle};
use crate::control::object::node_editor::object::clamp_canvas_width;
use nichlink_run_method::{ContractId, FlowContract};

/// NodeEditor 交给子对象的绘制结果。
/// The frame NodeEditor hands to its children for painting.
pub struct NodeEditorFrame;

/// 每个直接子对象必须实现的接口。
/// The interface every direct child must implement.
pub trait NodeEditorHandle {
    fn resize(&self) -> NodeEditorFrame;
}

pub struct NodeEditor;

impl ControlHandle for NodeEditor {
    fn paint(&self) -> ControlFrame {
        ControlFrame
    }
}

/// 预览画布宽度，并把请求交给画布对象做钳制。
/// Preview the canvas width and hand the request to the canvas object to clamp.
pub fn preview_canvas_width(requested: u32) -> u32 {
    let clamped = clamp_canvas_width(requested);
    clamped
}

/// 追踪版预览：记录请求后复用普通预览。
/// Traced preview: record the request, then reuse the plain preview.
pub fn preview_canvas_width_traced(requested: u32) -> u32 {
    let traced = preview_canvas_width(requested);
    traced
}

crate::control_object! {
    kind: NodeEditor,
    exports: ["control.render"],
    needs_registry: true,
    parent: crate::control::NODE_ID,
    handle_contracts: [crate::control::ControlHandle],
    registry_rule_path: "src/control/object/node_editor/registry_rule/registry_rule.rs",
    flow: FlowContract::new(
        ContractId::new("control.render.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ),
}
