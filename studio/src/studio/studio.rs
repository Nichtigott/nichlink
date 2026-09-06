//! NichLink Studio: an optional Ratatui adapter for the registration core.
//! NichLink Studio：注册核心的可选 Ratatui 适配器。

#[path = "app/app.rs"]
mod app;
mod terminal;
#[path = "ui/ui.rs"]
mod ui;

use std::io::{self, stdout};

use crossterm::event;
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::App;
use terminal::{install_panic_restore, open_editor_in_terminal};

pub fn launch() -> io::Result<()> {
    install_panic_restore();
    enable_raw_mode()?;
    let _terminal_guard = TerminalGuard;
    let mut output = stdout();
    execute!(output, EnterAlternateScreen, event::EnableMouseCapture)?;
    let backend = CrosstermBackend::new(output);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let result = run_loop(&mut terminal);
    terminal.show_cursor()?;
    result
}

/// Restore terminal state even when setup or the event loop returns an error.
/// 即使初始化或事件循环返回错误，也要恢复终端状态。
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), event::DisableMouseCapture, LeaveAlternateScreen);
    }
}

/// Draw only after input or a terminal event; idle Studio has no redraw loop.
/// 仅在输入或终端事件后绘制；Studio 空闲时没有重绘循环。
fn run_loop(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let mut app = App::load();
    let mut redraw = true;
    loop {
        if redraw {
            terminal.draw(|frame| ui::draw(frame, &mut app))?;
            redraw = false;
        }
        if app.should_quit {
            return Ok(());
        }
        if event::poll(std::time::Duration::from_millis(250))? {
            app.handle(event::read()?);
            redraw = true;
        }
        // The method is internally throttled, so checking after input keeps
        // reload latency low without adding work to the hot path.
        // 方法内部自带节流；每次输入后检查即可降低刷新延迟，不增加热路径开销。
        let event_before_reload = app.event.clone();
        app.poll_hot_reload();
        redraw |= app.event != event_before_reload;
        if let Some((path, line)) = app.take_editor_request() {
            let result = open_editor_in_terminal(&path, line);
            app.event = match result {
                Ok(editor) => format!("{editor} opened in a new terminal."),
                Err(error) => format!("Editor failed: {error}"),
            };
            redraw = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::terminal::editor_script;
    use std::path::Path;

    #[test]
    fn editor_script_keeps_nvim_command_plain() {
        let script = editor_script("nvim --clean", Path::new("src/thing's.rs"), 12);
        assert!(script.contains("nvim --clean +12 'src/thing'\\''s.rs'"));
        assert!(!script.contains("vim.treesitter"));
        assert!(!script.contains("syntax=rust"));
    }

    #[test]
    fn editor_script_passes_unknown_editors_a_single_safe_path() {
        let script = editor_script("my-editor --wait", Path::new("src/a b.rs"), 7);
        assert_eq!(script, "exec my-editor --wait 'src/a b.rs'");
    }

    #[test]
    fn vim_script_is_plain_too() {
        let script = editor_script("vim", Path::new("src/main.rs"), 3);
        assert!(script.contains("vim +3 'src/main.rs'"));
        assert!(!script.contains("vim.treesitter"));
    }
}
