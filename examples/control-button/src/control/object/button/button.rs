//! Button 叶子面：它是 Control 的直接子对象，不再拥有自己的 Registry。
//! Button leaf face: a direct child of Control that owns no Registry of its own.

use crate::control::{ControlFrame, ControlHandle};
use nichlink_run_method::{ContractId, FlowContract, NON_EMPTY_TEXT};

pub struct Button;

impl ControlHandle for Button {
    fn paint(&self) -> ControlFrame {
        ControlFrame
    }
}

crate::control_object! {
    kind: Button,
    exports: ["control.render"],
    parent: crate::control::NODE_ID,
    handle_contracts: [crate::control::ControlHandle],
    flow: FlowContract::new(
        ContractId::new("control.render.v1"),
        1,
        "ControlInput",
        "ControlFrame",
    ),
    // A real declared check: a Button's label crosses into the renderer as text
    // and must not be blank. The kernel cannot observe that label, so the host
    // calls `Registry::health_check` at the boundary; the declaration only
    // states what "valid" means.
    // 一条真实的已声明检查：Button 的标签以文本形式跨入渲染器，不能为空白。内核无法
    // 观测该标签，因此由宿主在边界处调用 `Registry::health_check`；声明只说明“有效”
    // 的含义。
    // `runtime_checks` is not part of `NodeId` (namespace + relative path + kind),
    // so declaring it must not move identity; `built_in_tree_has_the_expected_paths_and_derived_sources`
    // and `outline()` pin that.
    // `runtime_checks` 不是 `NodeId`（命名空间 + 相对路径 + kind）的组成部分，因此声明
    // 它不该移动身份；`built_in_tree_has_the_expected_paths_and_derived_sources` 与
    // `outline()` 钉住这一点。
    runtime_checks: [NON_EMPTY_TEXT],
}
