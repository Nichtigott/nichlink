//! Search overlay keyboard handling.
//! 搜索浮层键盘处理。

use super::super::*;

impl App {
    pub(super) fn handle_search_overlay_key(&mut self, key: KeyEvent, mut search: SearchState) {
        if search.graph_mode {
            let center = if search.graph_side == 1 {
                search.compare_center.or(search.center)
            } else {
                search.center
            };
            let Some(center) = center else {
                if matches!(key.code, KeyCode::Tab | KeyCode::BackTab) {
                    if key.code == KeyCode::Tab {
                        advance_graph_focus(&mut search);
                    } else {
                        retreat_graph_focus(&mut search);
                    }
                }
                self.overlay = Some(Overlay::Search(search));
                return;
            };
            let center_function = if search.graph_side == 1 {
                search.compare_center_function.as_deref()
            } else {
                search.center_function.as_deref()
            };
            let chain = self.call_chain(center, center_function);
            let tree_len = self
                .graph_item(&search, search.graph_side)
                .map(|item| self.call_tree_targets(&item).len())
                .unwrap_or_default();
            let tree_cursor = if search.graph_side == 1 {
                search.compare_outline_selected
            } else {
                search.outline_selected
            };
            let tree_item = self.graph_tree_item(&search, search.graph_side, tree_cursor);
            let data_len = self
                .graph_tree_item(&search, search.graph_side, tree_cursor)
                .map(|item| self.graph_locals(&item).len())
                .unwrap_or_default();
            let selected_index = if search.graph_side == 1 {
                search
                    .compare_graph_selected
                    .min(chain.len().saturating_sub(1))
            } else {
                search.graph_selected.min(chain.len().saturating_sub(1))
            };
            match key.code {
                KeyCode::Char('/') => {
                    self.page = StudioPage::Search;
                    search.graph_mode = false;
                    search.center = None;
                    search.center_function = None;
                    search.center_line = None;
                    search.compare_center = None;
                    search.compare_center_function = None;
                    search.compare_center_line = None;
                    search.graph_selected = 0;
                    search.compare_graph_selected = 0;
                }
                KeyCode::Char('m') => match self.load_mir_snapshot() {
                    Ok(calls) => {
                        self.event =
                            format!("MIR snapshot loaded in memory: {calls} candidate calls.")
                    }
                    Err(error) => self.event = format!("MIR unavailable: {error}"),
                },
                KeyCode::Tab => advance_graph_focus(&mut search),
                KeyCode::BackTab => retreat_graph_focus(&mut search),
                KeyCode::Char(character) if search.graph_focus < 2 => {
                    if search.graph_focus == 0 {
                        search.query.push(character);
                        search.selected = 0;
                        search.offset = 0;
                    } else {
                        search
                            .compare_query
                            .get_or_insert_with(String::new)
                            .push(character);
                        search.compare_selected = 0;
                        search.compare_offset = 0;
                    }
                }
                KeyCode::Backspace if search.graph_focus < 2 => {
                    if search.graph_focus == 0 {
                        search.query.pop();
                        search.selected = 0;
                        search.offset = 0;
                    } else if let Some(query) = search.compare_query.as_mut() {
                        query.pop();
                        search.compare_selected = 0;
                        search.compare_offset = 0;
                    }
                }
                // In the single-pane graph the arrows are free, so they mean what
                // the drawing says: ← walks upstream (who calls this), → walks
                // downstream (what this calls). With a comparison pane the same
                // keys keep switching sides, because there is no room for both.
                // 单面板调用图里方向键是空闲的，因此它们表示画面上写着的东西：← 走上游
                // （谁在调它），→ 走下游（它调用了谁）。开了对比面板时这两个键仍用于切换
                // 左右侧，因为放不下两种含义。
                KeyCode::Left
                    if search.graph_focus == 2
                        && search.compare_query.is_none()
                        && search.compare_center.is_none() =>
                {
                    self.hop_call_tree(&mut search, false)
                }
                KeyCode::Right
                    if search.graph_focus == 2
                        && search.compare_query.is_none()
                        && search.compare_center.is_none() =>
                {
                    self.hop_call_tree(&mut search, true)
                }
                KeyCode::Up => match search.graph_focus {
                    0 => search.graph_selected = search.graph_selected.saturating_sub(1),
                    1 => {
                        search.compare_graph_selected =
                            search.compare_graph_selected.saturating_sub(1)
                    }
                    2 if search.graph_side == 1 => {
                        search.compare_outline_selected =
                            search.compare_outline_selected.saturating_sub(1)
                    }
                    2 => search.outline_selected = search.outline_selected.saturating_sub(1),
                    3 if search.graph_side == 1 => {
                        search.compare_data_selected =
                            search.compare_data_selected.saturating_sub(1)
                    }
                    _ => search.data_selected = search.data_selected.saturating_sub(1),
                },
                KeyCode::Down => match search.graph_focus {
                    0 => {
                        search.graph_selected =
                            (search.graph_selected + 1).min(chain.len().saturating_sub(1))
                    }
                    1 => {
                        search.compare_graph_selected =
                            (search.compare_graph_selected + 1).min(chain.len().saturating_sub(1))
                    }
                    2 if search.graph_side == 1 => {
                        search.compare_outline_selected =
                            (search.compare_outline_selected + 1).min(tree_len.saturating_sub(1))
                    }
                    2 => {
                        search.outline_selected =
                            (search.outline_selected + 1).min(tree_len.saturating_sub(1))
                    }
                    3 if search.graph_side == 1 => {
                        search.compare_data_selected =
                            (search.compare_data_selected + 1).min(data_len.saturating_sub(1))
                    }
                    _ => {
                        search.data_selected =
                            (search.data_selected + 1).min(data_len.saturating_sub(1))
                    }
                },
                KeyCode::Left if search.graph_focus >= 2 => search.graph_side = 0,
                KeyCode::Right if search.graph_focus >= 2 && search.compare_query.is_some() => {
                    search.graph_side = 1
                }
                KeyCode::Enter if search.graph_focus < 2 => {
                    if let Some(item) = chain.get(selected_index) {
                        if search.graph_focus == 1 {
                            if search.compare_center == Some(item.node)
                                && search.compare_center_function.as_deref()
                                    == Some(item.function.as_str())
                            {
                                self.selected = item.node;
                                self.open_editor_at(
                                    item.node,
                                    self.source_function_line(item.node, &item.function),
                                );
                                self.overlay = None;
                                return;
                            }
                            search.compare_center = Some(item.node);
                            search.compare_center_function = Some(item.function.clone());
                            search.compare_center_line =
                                self.source_function_line(item.node, &item.function);
                            search.compare_graph_selected = self
                                .call_chain(item.node, Some(&item.function))
                                .iter()
                                .position(|candidate| {
                                    candidate.node == item.node
                                        && candidate.function == item.function
                                })
                                .unwrap_or(0);
                        } else {
                            if search.center == Some(item.node)
                                && search.center_function.as_deref() == Some(item.function.as_str())
                            {
                                self.selected = item.node;
                                self.open_editor_at(
                                    item.node,
                                    self.source_function_line(item.node, &item.function),
                                );
                                self.overlay = None;
                                return;
                            }
                            search.center = Some(item.node);
                            search.center_function = Some(item.function.clone());
                            search.center_line =
                                self.source_function_line(item.node, &item.function);
                            search.graph_selected = self
                                .call_chain(item.node, Some(&item.function))
                                .iter()
                                .position(|candidate| {
                                    candidate.node == item.node
                                        && candidate.function == item.function
                                })
                                .unwrap_or(0);
                        }
                        search.outline_selected = 0;
                        search.data_selected = 0;
                    }
                }
                KeyCode::Enter if search.graph_focus == 2 => {
                    let outline_index = if search.graph_side == 1 {
                        search.compare_outline_selected
                    } else {
                        search.outline_selected
                    };
                    if let Some(target) =
                        self.graph_tree_item(&search, search.graph_side, outline_index)
                    {
                        // Enter on a tree node means "now show me the tree around
                        // this one"; at the focus itself it opens the editor,
                        // which is the gesture the three-column view uses too.
                        // 在树节点上按 Enter 表示"现在给我看围绕它的那棵树"；在焦点本身
                        // 上则打开编辑器，三列视图的手势也是如此。
                        if !self.recentre_call_tree(&mut search, target.clone()) {
                            let line = self.source_function_line(target.node, &target.function);
                            self.selected = target.node;
                            self.open_editor_at(target.node, line);
                            self.overlay = None;
                            return;
                        }
                    }
                }
                KeyCode::Enter if search.graph_focus == 3 => {
                    if let Some(item) = tree_item.as_ref() {
                        let locals = self.graph_locals(item);
                        let local_index = if search.graph_side == 1 {
                            search.compare_data_selected
                        } else {
                            search.data_selected
                        };
                        if let Some(local) = locals.get(local_index) {
                            self.open_editor_file(
                                source_path_for(local.source.file),
                                local.source.line,
                            );
                            self.overlay = None;
                            return;
                        }
                    }
                }
                KeyCode::Enter => {}
                KeyCode::Char('e') if search.graph_focus == 3 => {
                    if let Some(item) = tree_item.as_ref() {
                        let local_index = if search.graph_side == 1 {
                            search.compare_data_selected
                        } else {
                            search.data_selected
                        };
                        if let Some(local) = self.graph_locals(item).get(local_index) {
                            self.open_editor_file(
                                source_path_for(local.source.file),
                                local.source.line,
                            );
                            self.overlay = None;
                            return;
                        }
                    }
                }
                KeyCode::Char('e') if search.graph_focus == 2 => {
                    if let Some(item) = tree_item.as_ref() {
                        self.selected = item.node;
                        self.open_editor_at(
                            item.node,
                            self.source_function_line(item.node, &item.function),
                        );
                        self.overlay = None;
                        return;
                    }
                }
                KeyCode::Char('e') => {
                    if let Some(item) = chain.get(search.graph_selected) {
                        if let Some(line) = self.source_function_line(item.node, &item.function) {
                            self.selected = item.node;
                            self.open_editor_at(item.node, Some(line));
                        }
                        return;
                    }
                }
                _ => {}
            }
            self.overlay = Some(Overlay::Search(search));
            return;
        }
        // Ctrl-W opens a second independent search pane; Tab switches focus.
        // Ctrl-W 打开第二个独立搜索窗；Tab 在两个搜索窗之间切换。
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('w') {
            if search.compare_query.is_none() {
                search.compare_query = Some(String::new());
            }
            self.page = StudioPage::Compare;
            search.active_pane = 1 - search.active_pane.min(1);
            self.overlay = Some(Overlay::Search(search));
            return;
        }
        if key.code == KeyCode::Tab && search.compare_query.is_some() {
            search.active_pane = 1 - search.active_pane.min(1);
            self.overlay = Some(Overlay::Search(search));
            return;
        }
        let active_query = if search.active_pane == 1 {
            search.compare_query.as_deref().unwrap_or("")
        } else {
            &search.query
        };
        let active_selected = if search.active_pane == 1 {
            search.compare_selected
        } else {
            search.selected
        };
        match key.code {
            // No Left/Right/Space arm here: the old code toggled a fold set, but
            // `search_rows` returns a flat list where every row has no children,
            // so the arm could never match and the footer no longer advertises
            // it. Removed instead of implemented.
            // 这里没有 Left/Right/Space 分支：旧代码切换折叠集合，但
            // `search_rows` 返回的扁平列表里没有任何行有子节点，该分支永远匹配
            // 不上，页脚也不再展示它。选择删除而不是实现。
            KeyCode::Char(character) => {
                if search.active_pane == 1 {
                    search
                        .compare_query
                        .get_or_insert_with(String::new)
                        .push(character);
                    search.compare_selected = 0;
                    search.compare_offset = 0;
                } else {
                    search.query.push(character);
                    search.selected = 0;
                    search.offset = 0;
                }
            }
            KeyCode::Backspace => {
                if search.active_pane == 1 {
                    if let Some(query) = search.compare_query.as_mut() {
                        query.pop();
                    }
                    search.compare_selected = 0;
                    search.compare_offset = 0;
                } else {
                    search.query.pop();
                    search.selected = 0;
                    search.offset = 0;
                }
            }
            KeyCode::Up => {
                if search.active_pane == 1 {
                    search.compare_selected = search.compare_selected.saturating_sub(1);
                } else {
                    search.selected = search.selected.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                let last = self.search_rows(active_query).len().saturating_sub(1);
                if search.active_pane == 1 {
                    search.compare_selected = (search.compare_selected + 1).min(last);
                } else {
                    search.selected = (search.selected + 1).min(last);
                }
            }
            KeyCode::Enter => {
                let row = self
                    .search_rows(active_query)
                    .get(active_selected)
                    .cloned()
                    .unwrap_or(SearchRow {
                        depth: 0,
                        node: None,
                        path: String::new(),
                        function: String::new(),
                        line: None,
                        signature: String::new(),
                        text: "No matching call row".to_owned(),
                    });
                if let Some(node) = row.node {
                    let function = (!row.function.is_empty()).then(|| row.function.clone());
                    if search.active_pane == 1 {
                        search.compare_center = Some(node);
                        search.compare_center_function = function;
                        search.compare_center_line = row.line;
                    } else {
                        search.center = Some(node);
                        search.center_function = function;
                        search.center_line = row.line;
                    }
                    search.graph_mode = true;
                    self.page = if search.active_pane == 1 {
                        StudioPage::Compare
                    } else {
                        StudioPage::Data
                    };
                    search.graph_side = if search.active_pane == 1 { 1 } else { 0 };
                    search.graph_focus = search.graph_side;
                    let chain = self.call_chain(
                        node,
                        if search.active_pane == 1 {
                            search.compare_center_function.as_deref()
                        } else {
                            search.center_function.as_deref()
                        },
                    );
                    let position = chain
                        .iter()
                        .position(|item| {
                            item.node == node
                                && (row.function.is_empty() || item.function == row.function)
                        })
                        .unwrap_or(0);
                    if search.active_pane == 1 {
                        search.compare_graph_selected = position;
                    } else {
                        search.graph_selected = position;
                    }
                    self.overlay = Some(Overlay::Search(search));
                } else {
                    self.event = format!("Selected {}", row.text);
                    self.overlay = Some(Overlay::Search(search));
                }
                return;
            }
            _ => {}
        }
        self.overlay = Some(Overlay::Search(search));
    }
}
