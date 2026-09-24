//! Keyboard and overlay-key interaction.
//! 键盘与浮层键盘交互。

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
            KeyCode::Char('/') => self.overlay = Some(Overlay::Search(SearchState::default())),
            KeyCode::Char('n') => self.overlay = Some(Overlay::NewProject(NewProjectState::new())),
            KeyCode::Char('a') => {
                let parent = self.selected_parent();
                let mut add = AddState::new(parent);
                // Keep the default parent readable; the resolver still accepts node identity.
                // 默认父节点显示可读名称；解析器仍接受 node identity。
                add.values[face_field::PARENT] = self
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
                    // The form shows the paths, not the labels: the labels follow
                    // them, in the file and in the compiled registration alike.
                    // `params` and `handle` are not shown at all — both are the
                    // kind by rule, so a row for either would only repeat it.
                    // 表单展示路径而不是标签：标签跟随路径——文件里与编译后的注册信息里
                    // 都是如此。`params` 与 `handle` 完全不展示：按规则两者都等于 kind，
                    // 任何一行的存在都只是重复它。
                    let (handle_contracts, part_contracts) =
                        std::fs::read_to_string(source_path_for(&info.source.file))
                            .ok()
                            .map(|source| declaration_contract_paths(&source))
                            .unwrap_or_default();
                    let mut edit = AddState::new(info.parent);
                    edit.locked_fields.insert(face_field::PARENT);
                    edit.values[face_field::PARENT] = self
                        .registry
                        .path_for(info.parent)
                        .unwrap_or_else(|| "root".to_owned());
                    // `module` describes the source directory/file, while
                    // `registry_name` is the name in the registration tree.
                    // They often start equal, but a module rename must not
                    // make the editor appear to revert after reload.
                    // `module` 表示源码目录/文件名，`registry_name` 表示注册树
                    // 中的槽位名。两者初始值可能相同，但重命名后必须分别读取。
                    edit.values[face_field::MODULE] = std::path::Path::new(&info.source.file)
                        .file_stem()
                        .and_then(|stem| stem.to_str())
                        .filter(|stem| !stem.is_empty())
                        .unwrap_or(info.registry_name.as_str())
                        .to_owned();
                    edit.values[face_field::NEEDS_REGISTRY] = info.needs_registry.to_string();
                    edit.values[face_field::TREE_SLOT] = info.registry_name.to_owned();
                    edit.values[face_field::REGISTRY_RULE] =
                        registration_rule_text(&info.registry_rule);
                    edit.values[face_field::ADMISSION] = admission_text(&info.admission);
                    edit.values[face_field::PARTS] = info.parts.to_owned();
                    edit.values[face_field::EXPORTS] = info.exports.join(",");
                    edit.values[face_field::KIND] = info.kind.to_owned();
                    edit.values[face_field::NAME_ZH] = info.name.zh.to_owned();
                    edit.values[face_field::NAME_EN] = info.name.en.to_owned();
                    edit.values[face_field::SUMMARY_ZH] = info.summary.zh.to_owned();
                    edit.values[face_field::SUMMARY_EN] = info.summary.en.to_owned();
                    edit.values[face_field::PRESET] = info.preset.to_owned();
                    edit.values[face_field::STABLE_NAME] =
                        info.stable_name.clone().unwrap_or_default();
                    edit.values[face_field::GETTING_FROM_OTHER_REGISTRY] =
                        info.getting_from_other_registry.clone().unwrap_or_default();
                    edit.values[face_field::REGISTRY_RULE_PATH] =
                        info.registry_rule_path.to_owned();
                    edit.values[face_field::HANDLE_TRAITS] = info.handle_traits.join(",");
                    edit.values[face_field::HANDLE_CONTRACTS] = handle_contracts;
                    edit.values[face_field::PART_TRAITS] = info.part_traits.join(",");
                    edit.values[face_field::REQUIRES] = info
                        .requires
                        .iter()
                        .map(|requirement| {
                            format!("{}=>{}", requirement.capability, requirement.provider)
                        })
                        .collect::<Vec<_>>()
                        .join(",");
                    edit.values[face_field::PROVIDES] = info.provides.join(",");
                    edit.values[face_field::RUNTIME_CHECKS] = info
                        .runtime_checks
                        .iter()
                        .map(|check| check.name())
                        .collect::<Vec<_>>()
                        .join(",");
                    edit.values[face_field::FLOW] = if info.flow.is_declared() {
                        format!(
                            "{}|{}|{}|{}",
                            info.flow.id, info.flow.version, info.flow.input, info.flow.output
                        )
                    } else {
                        String::new()
                    };
                    edit.values[face_field::FLOW_PROVIDER] =
                        info.flow_provider.as_deref().unwrap_or_default().to_owned();
                    edit.values[face_field::PART_CONTRACTS] = part_contracts;
                    if let Some(parent_face) = self.registry.find(info.parent) {
                        edit.apply_parent_rule(&parent_face.registry_rule);
                    }
                    self.overlay = Some(Overlay::Edit(info.id, edit));
                }
            }
            KeyCode::Char('g') if self.selected != self.registry.id() => self.open_graft(),
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
