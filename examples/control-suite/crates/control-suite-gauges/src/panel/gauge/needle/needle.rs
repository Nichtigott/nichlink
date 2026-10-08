//! Needle leaf face: a child of Gauge, inside the `gauges` crate.
//! Needle 叶子面：Gauge 的子对象，属于 `gauges` 这个 crate。

crate::gauge_object! {
    kind: Needle,
    parent: crate::panel::gauge::NODE_ID,
}

pub struct Needle;
