//! Historical path: `studio/src/studio/app/support.rs`.
//! 历史路径：`studio/src/studio/app/support.rs`。
//!
//! The module used to carry three unrelated subjects under one category name:
//! project context, the `cargo` probe, and the TUI geometry plus editor handoff.
//! They now live in `project_context`, `cargo_probe` and `geometry`, mounted beside
//! this file by `app.rs`; this page only re-exports them so every call site that
//! already writes `super::support::…` keeps resolving (audit `STU-S-15`).
//! 本模块过去把一个类别名挂在三个互不相干的主语之下——项目上下文、`cargo` 探测、TUI 几何与
//! 编辑器交接。它们现在住在 `project_context`、`cargo_probe` 与 `geometry`，由 `app.rs` 挂载在
//! 本文件旁边；本页只重导出它们，使已经在写 `super::support::…` 的调用点继续可解析
//! （审计 `STU-S-15`）。

pub(super) use super::{cargo_probe::*, project_context::*};
