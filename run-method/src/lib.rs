//! Compatibility shell: `nichlink-run-method` is the host-facing crate name this surface had before
//! the nine-to-three merge. A host written against it keeps compiling unchanged — `host!()`, the
//! trace types, the lexicon and the authoring entry all arrive through one glob.
//! 兼容外壳：`nichlink-run-method` 是九→三合并之前这个面向宿主的 crate 名。按旧名写成的宿主原样继续
//! 编译——`host!()`、trace 类型、词汇表与 authoring 入口都经一次 glob 到达。
//!
//! It is a **shell**, not a second implementation: everything it exposes is re-exported from
//! `nichlink-toolchain`, so there is one implementation and this crate cannot drift from it. New code
//! should depend on `nichlink-toolchain` directly; this exists for manifests already in the world.
//! 它是**外壳**，不是第二份实现：它暴露的一切都从 `nichlink-toolchain` 重导出，因此实现只有一份，本 crate
//! 无法与它漂移。新代码应当直接依赖 `nichlink-toolchain`；这个壳是为已经在世界上的清单而存在的。
#![warn(missing_docs)]

pub use nichlink_toolchain::*;
