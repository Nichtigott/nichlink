//! Write guards: every operation that creates, rewrites, moves, or deletes
//! something in the project the reader opened.
//! 写入守卫：每一个在读者打开的项目里创建、重写、移动或删除东西的操作。
//!
//! The distinction this module exists for is read versus write. `package_root`
//! resolves a project from the session, then the environment, then the working
//! directory — a *guess*, and a reasonable one for a read, because reading the
//! wrong tree is visible. A write cannot afford it: a delete that resolved its
//! root that way moved a module out of whichever project the environment happened
//! to name, which is the bug the delete overlay still carries the comment about.
//! So writers go through [`with_selected_project`], which has no fallback, and a
//! future call site cannot repeat the bug by forgetting to establish a context.
//! 本模块存在的意义就是"读"与"写"的区分。`package_root` 依次从会话、环境、工作目录解析项目
//! ——一个**猜测**，对读取来说还算合理，因为读错树看得见。写入承担不起：一次这样解析根的删除把
//! 模块从环境变量恰好指到的那个项目里搬走了，也就是删除浮层至今在注释里记着的那个 bug。因此写入
//! 方都走 [`with_selected_project`]：它没有回落，将来的调用点也不会因为忘记建立上下文而重演。
//!
//! A launched session always has a project — `resolve_project` adopts it before the
//! terminal is taken over — so the refusal below is reachable only from a library
//! caller, a test, or a session whose selection was cleared.
//! 已启动的会话总有项目——`resolve_project` 在接管终端之前就采纳了它——因此下面的拒绝只可能来自
//! 库调用方、测试，或被清空选择的会话。

use std::path::PathBuf;

use super::support::{package_namespace, selected_root};

/// The selected project's root, or a named refusal.
/// 已选中项目的根目录，或一条具名拒绝。
pub(super) fn selected_package_root() -> Result<PathBuf, String> {
    selected_root()
        .ok_or_else(|| "no project is selected; open a project before writing to it".to_owned())
}

/// Run one write inside the selected project's authoring context.
/// 在已选中项目的创作上下文里执行一次写入。
pub(super) fn with_selected_project<T>(
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let root = selected_package_root()?;
    nichlink_run_method::AuthoringContext::new(root, package_namespace()).scope(operation)
}
