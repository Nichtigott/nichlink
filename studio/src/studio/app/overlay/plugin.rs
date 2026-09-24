//! Plugin overlay keyboard handling.
//! 插件浮层键盘处理。

use super::super::*;

impl App {
    pub(super) fn handle_plugin_overlay_key(&mut self, key: KeyEvent, mut plugin: PluginState) {
        if !plugin.editing && matches!(key.code, KeyCode::Char('q')) {
            self.overlay = None;
            return;
        }
        if plugin.editing {
            match key.code {
                KeyCode::Enter => plugin.editing = false,
                KeyCode::Backspace => {
                    plugin.values[plugin.field].pop();
                }
                KeyCode::Char(character) => plugin.values[plugin.field].push(character),
                _ => {}
            }
        } else {
            match key.code {
                KeyCode::Up => plugin.field = plugin.field.saturating_sub(1),
                KeyCode::Down | KeyCode::Tab => plugin.field = (plugin.field + 1).min(6),
                KeyCode::Enter if plugin.field == 0 || plugin.field == 6 => {
                    plugin.values[plugin.field] = match plugin.field {
                        plugin_field::SOURCE
                            if plugin.values[plugin_field::SOURCE] == "official" =>
                        {
                            "user".to_owned()
                        }
                        0 => "official".to_owned(),
                        plugin_field::MODE if plugin.values[plugin_field::MODE] == "extension" => {
                            "replacement".to_owned()
                        }
                        _ => "extension".to_owned(),
                    };
                }
                KeyCode::Enter => plugin.editing = true,
                KeyCode::Char('s') => {
                    self.submit_plugin(&plugin);
                    return;
                }
                _ => {}
            }
        }
        self.overlay = Some(Overlay::Plugin(plugin));
    }
}
