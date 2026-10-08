//! Widget leaf face: a child of Frame, inside the `frames` crate.
//! Widget 叶子面：Frame 的子对象，属于 `frames` 这个 crate。

crate::frame_object! {
    kind: Widget,
    parent: crate::panel::frame::NODE_ID,
}

pub struct Widget;
