//! Keyboard and overlay-key interaction.
//! 键盘与浮层键盘交互。

use super::support::with_authoring_context;
use super::*;

impl App {
    pub(super) fn handle_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }
        if self.overlay.is_some() {
            self.handle_overlay_key(key);
            return;
        }
        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('1') => self.open_page(StudioPage::Search),
            KeyCode::Char('2') => self.open_page(StudioPage::Inspect),
            KeyCode::Char('3') => self.open_page(StudioPage::Data),
            KeyCode::Char('4') => self.open_page(StudioPage::Compare),
            KeyCode::Char('/') => self.overlay = Some(Overlay::Search(SearchState::default())),
            KeyCode::Char('n') => self.overlay = Some(Overlay::NewProject(NewProjectState::new())),
            KeyCode::Char('a') => {
                let parent = self.selected_parent();
                let mut add = AddState::new(parent);
                // Keep the default parent readable; the resolver still accepts node identity.
                // 默认父节点显示可读名称；解析器仍接受 node identity。
                add.values[0] = self
                    .registry
                    .path_for(parent)
                    .unwrap_or_else(|| "root".to_owned());
                if let Some(parent_face) = self.registry.find(parent) {
                    add.apply_parent_rule(&parent_face.registry_rule);
                }
                self.overlay = Some(Overlay::Add(add));
            }
            KeyCode::Char('p') => self.overlay = Some(Overlay::Plugin(PluginState::new())),
            KeyCode::Char('d') => {
                if self.selected != self.registry.id() {
                    self.overlay = Some(Overlay::Delete(self.selected));
                }
            }
            KeyCode::Char('e') if self.selected != self.registry.id() => {
                if let Some(info) = self.selected_info() {
                    let (handle_contracts, part_contracts) =
                        std::fs::read_to_string(source_path_for(&info.source.file))
                            .ok()
                            .map(|source| declaration_contract_paths(&source))
                            .unwrap_or_default();
                    let mut edit = AddState::new(info.parent);
                    edit.locked_fields.insert(0);
                    edit.values[0] = self
                        .registry
                        .path_for(info.parent)
                        .unwrap_or_else(|| "root".to_owned());
                    // `module` describes the source directory/file, while
                    // `registry_name` is the name in the registration tree.
                    // They often start equal, but a module rename must not
                    // make the editor appear to revert after reload.
                    // `module` 表示源码目录/文件名，`registry_name` 表示注册树
                    // 中的槽位名。两者初始值可能相同，但重命名后必须分别读取。
                    edit.values[1] = std::path::Path::new(&info.source.file)
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .filter(|stem| !stem.is_empty())
                        .unwrap_or(info.registry_name.as_str())
                        .to_owned();
                    edit.values[2] = info.needs_registry.to_string();
                    edit.values[3] = info.registry_name.to_owned();
                    edit.values[4] = registration_rule_text(&info.registry_rule);
                    edit.values[5] = admission_text(&info.admission);
                    edit.values[6] = info.parts.to_owned();
                    edit.values[7] = info.exports.join(",");
                    edit.values[8] = info.kind.to_owned();
                    edit.values[9] = info.name.zh.to_owned();
                    edit.values[10] = info.name.en.to_owned();
                    edit.values[11] = info.summary.zh.to_owned();
                    edit.values[12] = info.summary.en.to_owned();
                    edit.values[13] = info.preset.to_owned();
                    edit.values[14] = info.params.to_owned();
                    edit.values[15] = info.handle.to_owned();
                    edit.values[16] = info.stable_name.clone().unwrap_or_default();
                    edit.values[17] = info.getting_from_other_registry.clone().unwrap_or_default();
                    edit.values[18] = info.registry_rule_path.to_owned();
                    edit.values[19] = info.handle_traits.join(",");
                    edit.values[20] = handle_contracts;
                    edit.values[21] = info.part_traits.join(",");
                    edit.values[22] = info
                        .requires
                        .iter()
                        .map(|requirement| {
                            format!("{}=>{}", requirement.capability, requirement.provider)
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    edit.values[23] = info.provides.join(",");
                    edit.values[24] = info.contract.expected_output.to_owned();
                    edit.values[25] = info.contract.actual_output.to_owned();
                    edit.values[26] = info
                        .runtime_checks
                        .iter()
                        .map(|check| check.name())
                        .collect::<Vec<_>>()
                        .join(",");
                    edit.values[27] = if info.flow.is_declared() {
                        format!(
                            "{}|{}|{}|{}",
                            info.flow.id, info.flow.version, info.flow.input, info.flow.output
                        )
                    } else {
                        String::new()
                    };
                    edit.values[28] = info.flow_provider.as_deref().unwrap_or_default().to_owned();
                    edit.values[29] = part_contracts;
                    if let Some(parent_face) = self.registry.find(info.parent) {
                        edit.apply_parent_rule(&parent_face.registry_rule);
                    }
                    self.overlay = Some(Overlay::Edit(info.id, edit));
                }
            }
            KeyCode::Char('g') if self.selected != self.registry.id() => {
                let selector = self
                    .registry
                    .find(self.selected)
                    .map(|info| format!("{}_graft", info.registry_name))
                    .unwrap_or_else(|| "replacement".to_owned());
                match with_authoring_context(|| {
                    nichlink_run_method::create_external_graft(
                        &self.registry,
                        self.selected,
                        selector,
                        false,
                    )
                }) {
                    Ok(plan) => {
                        let plan_path = plan.plan_path();
                        self.open_editor_file(plan_path.clone(), 1);
                        self.event = format!(
                            "External graft plan created at {}; edit it, then reload",
                            plan_path.display()
                        );
                    }
                    Err(error) => self.event = format!("External graft failed: {error}"),
                }
            }
            KeyCode::Char('r') | KeyCode::F(5) => self.reload(),
            KeyCode::Char('b') | KeyCode::F(9) => self.build_all(),
            KeyCode::Tab => {
                self.focus = if self.focus == Focus::Tree {
                    Focus::Details
                } else {
                    Focus::Tree
                };
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.focus == Focus::Tree {
                    self.move_selection(-1);
                } else {
                    self.details_selected = self.details_selected.saturating_sub(1);
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.focus == Focus::Tree {
                    self.move_selection(1);
                } else {
                    self.details_selected = (self.details_selected + 1)
                        .min(self.detail_field_count().saturating_sub(1));
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.split_percent = self.split_percent.saturating_sub(2).max(25)
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.split_percent = (self.split_percent + 2).min(70)
            }
            KeyCode::Enter
                if self.focus == Focus::Details && self.selected != self.registry.id() =>
            {
                self.handle_key(KeyEvent::from(KeyCode::Char('e')));
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.toggle_selected(),
            _ => {}
        }
    }
}

pub(super) fn declaration_contract_paths(source: &str) -> (String, String) {
    let Ok(Some(face)) = nichlink_run_method::parse_face(source) else {
        return (String::new(), String::new());
    };
    let paths = |field| face.path_list(field).unwrap_or_default().join(",");
    (paths("handle_contracts"), paths("part_contracts"))
}
