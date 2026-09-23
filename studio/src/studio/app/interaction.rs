//! App keyboard, pointer, overlay, and selection interaction.
//! App 键盘、指针、浮层与选择交互。

use super::*;

impl App {
    /// Dispatch one terminal event to the active overlay or the base key and pointer maps.
    /// 把一个终端事件分派给当前浮层，或基础键盘与指针映射。
    ///
    /// The caller reads events; quit intent is recorded in `should_quit` for the loop.
    /// 调用方负责读取事件；退出意图记入 `should_quit`，由事件循环检查。
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
