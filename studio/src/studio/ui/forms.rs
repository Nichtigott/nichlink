//! Form overlay rendering for Studio.
//! Studio 表单浮层渲染。
//!
//! This root mounts one page per form and re-exports its entry point: the
//! add/edit face form in `face`, the new-project form in `project`, the
//! plugin form in `plugin`, the graft composer in `graft`, and the delete
//! confirmation in `delete`. The shared field dictionary and field guide stay
//! in `face_fields`.
//! 本模块根为每个表单挂载一个页面并重导出其入口：新增/编辑注册面表单在 `face`，
//! 新建项目表单在 `project`，插件表单在 `plugin`，graft 撰写器在 `graft`，
//! 删除确认在 `delete`。共享字段词典与字段指南仍在 `face_fields`。

use super::*;

#[path = "forms/delete.rs"]
mod delete;
#[path = "forms/face.rs"]
mod face;
#[path = "forms/face_fields.rs"]
mod face_fields;
#[path = "forms/graft.rs"]
mod graft;
#[path = "forms/plugin.rs"]
mod plugin;
#[path = "forms/project.rs"]
mod project;

pub(super) use delete::draw_delete;
pub(super) use face::{draw_add, draw_edit};
pub(super) use graft::draw_graft;
pub(super) use plugin::draw_plugin;
pub(super) use project::draw_new_project;
