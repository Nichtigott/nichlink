//! Pure TUI geometry and selection movement, plus the editor handoff.
//! 纯 TUI 几何与选中移动，以及编辑器交接。

use std::path::PathBuf;

use super::*;

impl App {
    pub(super) fn near_divider(&self, column: u16, row: u16) -> bool {
        self.hot.workspace_area.contains((column, row).into())
            && column.abs_diff(self.hot.tree_area.right()) <= 1
    }

    pub(super) fn near_graph_divider(&self, column: u16, row: u16) -> bool {
        self.hot.graph_area.contains((column, row).into())
            && column.abs_diff(self.hot.graph_tree_area.right()) <= 1
    }

    pub(super) fn resize_graph_split(&mut self, column: u16) {
        if self.hot.graph_area.width == 0 {
            return;
        }
        let relative = column.saturating_sub(self.hot.graph_area.x) as u32;
        self.graph_split_percent =
            ((relative * 100) / u32::from(self.hot.graph_area.width)).clamp(35, 65) as u16;
    }

    pub(super) fn resize_split(&mut self, column: u16) {
        if self.hot.workspace_area.width == 0 {
            return;
        }
        let relative = column.saturating_sub(self.hot.workspace_area.x) as u32;
        self.split_percent =
            ((relative * 100) / u32::from(self.hot.workspace_area.width)).clamp(25, 70) as u16;
    }

    pub(super) fn move_selection(&mut self, delta: isize) {
        let nodes = self.visible_nodes();
        let current = nodes
            .iter()
            .position(|(id, _)| *id == self.selected)
            .unwrap_or_default();
        let last = nodes.len().saturating_sub(1) as isize;
        let next = (current as isize + delta).clamp(0, last) as usize;
        if let Some((id, _)) = nodes.get(next)
            && self.selected != *id
        {
            self.selected = *id;
            self.details_selected = 0;
        }
    }

    pub(super) fn toggle_selected(&mut self) {
        if !self.owns_registry(self.selected) {
            return;
        }
        if !self.collapsed.insert(self.selected) {
            self.collapsed.remove(&self.selected);
        }
    }

    pub(super) fn selected_parent(&self) -> NodeId {
        self.selected_info()
            .filter(|info| info.needs_registry)
            .map(|info| info.id)
            .or_else(|| {
                self.selected_info()
                    .and_then(|info| self.registry.find(info.parent))
                    .filter(|info| info.needs_registry)
                    .map(|info| info.id)
            })
            .unwrap_or(self.registry.id())
    }

    pub(super) fn open_editor_at(&mut self, node: NodeId, line: Option<u32>) {
        let Some(info) = self.registry.find(node) else {
            return;
        };
        let path = source_path_for(&info.source.file);
        if !path.is_file() {
            self.alert(format!(
                "Editor failed: source file not found at {}",
                path.display()
            ));
            return;
        }
        self.editor_request = Some((path, line.unwrap_or(info.source.line)));
    }

    /// Open an arbitrary runtime source location, including a local value.
    /// 打开任意运行时源码位置，包括局部变量位置。
    ///
    /// The failure is returned as well as written to `event`: `event` is the only
    /// feedback channel this screen has, so a caller that shows a success banner
    /// right afterwards has to be able to keep the failure instead of assigning
    /// over it (audit `LGC-LG-50`).
    /// 失败既写进 `event` 也作为返回值交回：`event` 是本界面唯一的反馈通道，因此紧接着要显示
    /// 成功横幅的调用方必须能保住这条失败，而不是把它覆盖掉（审计 `LGC-LG-50`）。
    /// Open an arbitrary runtime source location, including a local value.
    /// 打开任意运行时源码位置，包括局部变量位置。
    ///
    /// The failure is returned as well as written to `event`: `event` is the only
    /// feedback channel this screen has, so a caller that shows a success banner
    /// right afterwards has to be able to keep the failure instead of assigning
    /// over it (audit `LGC-LG-50`).
    /// 失败既写进 `event` 也作为返回值交回：`event` 是本界面唯一的反馈通道，因此紧接着要显示
    /// 成功横幅的调用方必须能保住这条失败，而不是把它覆盖掉（审计 `LGC-LG-50`）。
    pub(super) fn open_editor_file(&mut self, path: PathBuf, line: u32) -> Result<(), String> {
        if !path.is_file() {
            let failure = format!("Editor failed: source file not found at {}", path.display());
            self.alert(failure.clone());
            return Err(failure);
        }
        self.editor_request = Some((path, line.max(1)));
        Ok(())
    }
}

impl App {
    pub(crate) fn graph_locals(&self, item: &CallRef) -> Vec<nichlink_run_method::LocalValue> {
        self.runtime_trace
            .locals()
            .iter()
            .filter(|local| {
                self.runtime_trace
                    .path_for_local(nichlink_run_method::LocalId(local.id))
                    .iter()
                    .any(|call| call.function == item.function)
            })
            .cloned()
            .collect()
    }
}
