//! Edit overlay keyboard handling.
//! 编辑注册面浮层键盘处理。

use super::super::*;

impl App {
    pub(super) fn handle_edit_overlay_key(
        &mut self,
        key: KeyEvent,
        id: NodeId,
        mut edit: AddState,
    ) {
        if !edit.editing && matches!(key.code, KeyCode::Char('q')) {
            self.overlay = None;
            return;
        }
        if edit.editing {
            match key.code {
                KeyCode::Enter => edit.editing = false,
                KeyCode::Backspace => {
                    edit.values[edit.field].pop();
                }
                KeyCode::Char(character) => edit.values[edit.field].push(character),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up => move_face_field(&mut edit, -1),
                KeyCode::Down | KeyCode::Tab => move_face_field(&mut edit, 1),
                KeyCode::Enter | KeyCode::Char(' ') if edit.field == 2 => {
                    edit.values[2] = (edit.values[2] != "true").to_string();
                }
                KeyCode::Enter if edit.is_editable(edit.field) => edit.editing = true,
                KeyCode::Char('s') => {
                    self.submit_edit(id, &edit);
                    return;
                }
                _ => {}
            }
        }
        self.overlay = Some(Overlay::Edit(id, edit));
    }
}
