//! Gauge folder face: a sibling of Frame under Panel, owning its own Registry.
//! Gauge 文件夹面：Panel 之下与 Frame 平级，拥有自己的 Registry。

crate::panel_object! {
    kind: Gauge,
    needs_registry: true,
    parent: crate::panel::NODE_ID,
}

pub struct Gauge;
