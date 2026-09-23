//! Delete overlay keyboard handling.
//! 删除浮层键盘处理。

use super::super::*;

impl App {
    pub(super) fn handle_delete_overlay_key(&mut self, key: KeyEvent, id: NodeId) {
        match key.code {
            KeyCode::Char('y') | KeyCode::Enter => {
                let spec = format!("{id} confirm");
                self.event = match nichlink_run_method::delete_module(&self.registry, &spec) {
                    Ok(change) => format!("{}; press r to reload", change.message),
                    Err(error) => format!("Delete failed: {error}"),
                };
            }
            KeyCode::Char('n') => {}
            _ => self.overlay = Some(Overlay::Delete(id)),
        }
    }
}
