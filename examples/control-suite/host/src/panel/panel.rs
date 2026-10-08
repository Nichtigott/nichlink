//! Panel folder face: a second root child that owns its own Registry.
//! Panel 文件夹面：第二个根子面，拥有自己的 Registry。

use nichlink_toolchain::run_method::{ContractId, FlowContract};

crate::root_object! {
    kind: Panel,
    needs_registry: true,
    parent: crate::root_node_id(crate::NICHLINK_NAMESPACE),
    flow: FlowContract::new(
        ContractId::new("panel.frame.v1"),
        1,
        "PanelInput",
        "PanelFrame",
    ),
}

/// 父面交给子对象的绘制结果。
/// The frame a parent face hands to its children for painting.
pub struct PanelFrame;

/// 每个直接子对象必须实现的接口。
/// The interface every direct child must implement.
pub trait PanelHandle {
    fn paint(&self) -> PanelFrame;
}

pub struct Panel;
