//! App keyboard, pointer, overlay, and selection interaction.
//! App 键盘、指针、浮层与选择交互。

use super::*;

impl App {
    pub fn handle(&mut self, event: Event) {
        match event {
            Event::Key(key) if key.kind == crossterm::event::KeyEventKind::Press => {
                self.handle_key(key)
            }
            Event::Mouse(mouse) => self.handle_mouse(mouse.kind, mouse.column, mouse.row),
            Event::Resize(_, _) | Event::FocusGained | Event::FocusLost | Event::Paste(_) => {}
            Event::Key(_) => {}
        }
    }
}
