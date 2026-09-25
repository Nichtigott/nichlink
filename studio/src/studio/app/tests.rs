//! App interaction regression tests.
//! App 交互回归测试。
//!
//! `app` mounts this file as `mod tests`; the root keeps the shared prelude
//! and fans the groups out to sibling files by concern so each stays
//! reviewable on its own.
//! `app` 把本文件挂载为 `mod tests`；根保留共享前导，并按关注点把测试组
//! 分散到同级文件，使每组都能独立审阅。

// Shared prelude re-exported for the mounted test submodules.
// 为挂载的测试子模块重新导出的共享前导。
pub(super) use std::time::{SystemTime, UNIX_EPOCH};

pub(super) use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEventKind};
pub(super) use ratatui::layout::Rect;

pub(super) use super::support::{
    host_manifest, package_root, resolve_project_from, select_project, with_authoring_context,
};
pub(super) use super::{
    AddState, App, GraftDeclaration, Overlay, StudioPage, app_function_source_range, body_calls,
    function_bodies, function_symbols,
};
// `SearchState` is named by the fixture-gated call-tree tests only, so it is
// imported with them: the default build refuses an unused import.
// `SearchState` 只被门控在夹具上的调用树测试命名，因此与它们一起导入：默认构建拒绝未使用的
// 导入。
#[cfg(feature = "prototype-fixtures")]
pub(super) use super::SearchState;
// The fixture helper exists only with the feature that names the fixture.
// 该夹具助手只在命名该夹具的特性下存在。
#[cfg(feature = "prototype-fixtures")]
pub(super) use super::support::node_editor_fixture;

// The prototype-fixture tests read call relations and build a compiler snapshot
// directly, so they name these instead of reaching them through `app`'s private
// imports. Gated with the tests that use them: with the feature off they would be
// unused imports, which the `-D warnings` gate refuses. The call-tree tests are
// fixture-gated too — a call graph needs a project with one — so the names they
// bring in stay in this group.
// 原型夹具测试直接读取调用关系并构造编译器快照，因此它们直接命名这些名字，而不是经由
// `app` 的私有导入取得。门控与使用它们的测试相同：特性关闭时它们就是未使用导入，会被
// `-D warnings` 门禁拒绝。调用树测试同样门控在夹具上——调用图需要一个有图的工程——因此
// 它们引入的名字留在这一组。
#[cfg(feature = "prototype-fixtures")]
pub(super) use super::{CallRef, CallTreeView, same_symbol, source_path_for};
#[cfg(feature = "prototype-fixtures")]
pub(super) use nichlink_debug_method::{CallEvidence, MirGraph};

// The call-tree tests read a real call graph, so they need the fixture project.
// 调用树测试要读真实调用图，因此需要夹具项目。
#[cfg(feature = "prototype-fixtures")]
#[path = "tests/call_tree.rs"]
mod call_tree;
#[path = "tests/edit.rs"]
mod edit;
// The evidence observation classifies the edges of a real project tree, so it is
// mounted with the fixture tests that load one.
// 证据观察要对一棵真实工程树的边分级，因此与加载工程树的夹具测试一同挂载。
#[cfg(feature = "prototype-fixtures")]
#[path = "tests/evidence.rs"]
mod evidence;
// The live trace the prototype-fixture tests install: mounted only with them, so
// the default build carries neither the helper nor its imports.
// 原型夹具测试安装的实时追踪：只在启用它们时挂载，因此默认构建既不携带该辅助函数，
// 也不携带它的导入。
#[cfg(feature = "prototype-fixtures")]
#[path = "tests/fixtures.rs"]
mod fixtures;
#[path = "tests/forms.rs"]
mod forms;
#[path = "tests/graft.rs"]
mod graft;
#[path = "tests/graph.rs"]
mod graph;
#[path = "tests/navigation.rs"]
mod navigation;
#[path = "tests/project.rs"]
mod project;
#[path = "tests/source.rs"]
mod source;
