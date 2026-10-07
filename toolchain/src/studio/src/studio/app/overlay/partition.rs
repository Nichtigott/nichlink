//! Partition overlay keyboard handling (audit `M7`, P3.6).
//! 分区浮层的键盘处理（审计 `M7`，P3.6）。
//!
//! Each of the three actions arms on its key and runs only on `y`, so a partition is never written by
//! a stray keystroke — the same two-press discipline the delete and graft overlays use, for the same
//! reason: these write files the user then commits.
//! 三个动作各自用它的按键装备、只在 `y` 上执行，因此一次拆分绝不会被一个误触的键写下去——与删除、graft 两个
//! 浮层同一套"两次按键"纪律，理由也相同：它们写下的文件随后会被用户提交。

use super::super::*;

impl App {
    pub(super) fn handle_partition_overlay_key(
        &mut self,
        key: KeyEvent,
        mut state: PartitionState,
    ) {
        // Any key consumes an armed action, so confirming takes two deliberate presses and nothing
        // else can carry the arm into an unrelated action.
        // 任何按键都会消费掉已装备的动作，因此确认需要两次有意的按键，别的动作不可能把这个装备状态带过去。
        let armed = state.pending.take();
        match key.code {
            KeyCode::Char('q') if armed.is_none() => {
                self.overlay = None;
                return;
            }
            KeyCode::Char('y') => {
                if let Some(action) = armed {
                    self.overlay = Some(Overlay::Partition(state));
                    self.run_partition_action(action);
                    return;
                }
            }
            KeyCode::Char('w') => state.pending = Some(PartitionAction::WriteDevelopment),
            KeyCode::Char('R') => state.pending = Some(PartitionAction::WriteRelease),
            KeyCode::Char('x') => state.pending = Some(PartitionAction::Revert),
            KeyCode::Char('r') => {
                self.overlay = Some(Overlay::Partition(state));
                self.refresh_partition();
                return;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                state.selected = state.selected.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let last = state
                    .view
                    .as_ref()
                    .map(|view| view.packages.len().saturating_sub(1))
                    .unwrap_or(0);
                state.selected = (state.selected + 1).min(last);
            }
            _ => {}
        }
        self.overlay = Some(Overlay::Partition(state));
    }
}
