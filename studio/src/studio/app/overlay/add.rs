//! Add overlay keyboard handling.
//! 添加注册面浮层键盘处理。

use super::super::*;

impl App {
    pub(super) fn handle_add_overlay_key(&mut self, key: KeyEvent, mut add: AddState) {
        if !add.editing && matches!(key.code, KeyCode::Char('q')) {
            self.overlay = None;
            return;
        }
        if add.editing {
            match key.code {
                KeyCode::Enter => add.editing = false,
                KeyCode::Backspace => {
                    add.values[add.field].pop();
                }
                KeyCode::Char(character) => add.values[add.field].push(character),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up => move_face_field(&mut add, -1),
                KeyCode::Down | KeyCode::Tab => move_face_field(&mut add, 1),
                KeyCode::Enter | KeyCode::Char(' ') if add.field == face_field::NEEDS_REGISTRY => {
                    add.values[face_field::NEEDS_REGISTRY] =
                        (add.values[face_field::NEEDS_REGISTRY] != "true").to_string();
                }
                KeyCode::Enter if add.is_editable(add.field) => add.editing = true,
                KeyCode::Char('s') => {
                    self.submit_add(&add);
                    return;
                }
                _ => {}
            }
        }
        self.overlay = Some(Overlay::Add(add));
    }
}
