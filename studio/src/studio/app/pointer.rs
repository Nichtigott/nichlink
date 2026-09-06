//! Pointer and overlay-click interaction.
//! 指针与浮层点击交互。

use super::*;

impl App {
    pub(super) fn handle_mouse(&mut self, kind: MouseEventKind, column: u16, row: u16) {
        if self.overlay.is_some() {
            match kind {
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                    let up = matches!(kind, MouseEventKind::ScrollUp);
                    let graph = matches!(self.overlay, Some(Overlay::Search(ref search)) if search.graph_mode);
                    if graph {
                        let point = (column, row).into();
                        let in_a = self.graph_a_input_area.contains(point)
                            || self.graph_a_center_area.contains(point)
                            || self.graph_a_output_area.contains(point);
                        let in_b = self.graph_b_input_area.contains(point)
                            || self.graph_b_center_area.contains(point)
                            || self.graph_b_output_area.contains(point);
                        let in_tree = self.graph_tree_a_area.contains(point)
                            || self.graph_tree_b_area.contains(point);
                        let in_data = self.graph_data_a_area.contains(point)
                            || self.graph_data_b_area.contains(point);
                        let tree_b = self.graph_tree_b_area.contains((column, row).into());
                        let data_b = self.graph_data_b_area.contains((column, row).into());
                        if let Some(Overlay::Search(search)) = self.overlay.as_mut() {
                            if in_a {
                                search.graph_focus = 0;
                                search.graph_side = 0;
                                search.selected = if up {
                                    search.selected.saturating_sub(1)
                                } else {
                                    search.selected.saturating_add(1)
                                };
                            } else if in_b {
                                search.graph_focus = 1;
                                search.graph_side = 1;
                                search.compare_selected = if up {
                                    search.compare_selected.saturating_sub(1)
                                } else {
                                    search.compare_selected.saturating_add(1)
                                };
                            } else if in_tree {
                                search.graph_focus = 2;
                                search.graph_side = if tree_b { 1 } else { 0 };
                                if search.graph_side == 1 {
                                    search.compare_outline_selected = if up {
                                        search.compare_outline_selected.saturating_sub(1)
                                    } else {
                                        search.compare_outline_selected.saturating_add(1)
                                    };
                                } else {
                                    search.outline_selected = if up {
                                        search.outline_selected.saturating_sub(1)
                                    } else {
                                        search.outline_selected.saturating_add(1)
                                    };
                                }
                            } else if in_data {
                                search.graph_focus = 3;
                                search.graph_side = if data_b { 1 } else { 0 };
                                if search.graph_side == 1 {
                                    search.compare_data_selected = if up {
                                        search.compare_data_selected.saturating_sub(1)
                                    } else {
                                        search.compare_data_selected.saturating_add(1)
                                    };
                                } else {
                                    search.data_selected = if up {
                                        search.data_selected.saturating_sub(1)
                                    } else {
                                        search.data_selected.saturating_add(1)
                                    };
                                }
                            }
                            search.outline_focus = search.graph_focus == 2;
                        }
                    } else {
                        self.handle_overlay_key(KeyEvent::from(if up {
                            KeyCode::Up
                        } else {
                            KeyCode::Down
                        }));
                    }
                }
                MouseEventKind::Down(MouseButton::Left) if self.near_graph_divider(column, row) => {
                    self.graph_dragging_divider = true;
                    self.resize_graph_split(column);
                }
                MouseEventKind::Drag(MouseButton::Left) if self.graph_dragging_divider => {
                    self.resize_graph_split(column)
                }
                MouseEventKind::Up(MouseButton::Left) => self.graph_dragging_divider = false,
                MouseEventKind::Down(MouseButton::Left) => self.handle_overlay_click(column, row),
                _ => {}
            }
            return;
        }
        match kind {
            MouseEventKind::ScrollUp => {
                if self.details_area.contains((column, row).into()) {
                    self.focus = Focus::Details;
                    self.details_selected = self.details_selected.saturating_sub(1);
                } else {
                    self.focus = Focus::Tree;
                    self.move_selection(-1);
                }
            }
            MouseEventKind::ScrollDown => {
                if self.details_area.contains((column, row).into()) {
                    self.focus = Focus::Details;
                    self.details_selected = (self.details_selected + 1)
                        .min(self.detail_field_count().saturating_sub(1));
                } else {
                    self.focus = Focus::Tree;
                    self.move_selection(1);
                }
            }
            MouseEventKind::Down(MouseButton::Left) if self.near_divider(column, row) => {
                self.dragging_divider = true;
                self.resize_split(column);
            }
            MouseEventKind::Down(MouseButton::Left)
                if self.tree_area.contains((column, row).into()) =>
            {
                let index = self.tree_offset
                    + row.saturating_sub(self.tree_area.y.saturating_add(1)) as usize;
                if let Some((id, _)) = self.visible_nodes().get(index) {
                    let clicked = *id;
                    if clicked == self.selected {
                        self.toggle_selected();
                    } else {
                        self.selected = clicked;
                        self.details_selected = 0;
                    }
                }
            }
            MouseEventKind::Down(MouseButton::Left)
                if self.details_area.contains((column, row).into()) =>
            {
                self.focus = Focus::Details;
                let index = row.saturating_sub(self.details_area.y.saturating_add(1)) as usize;
                self.details_selected = index.min(self.detail_field_count().saturating_sub(1));
            }
            MouseEventKind::Drag(MouseButton::Left) if self.dragging_divider => {
                self.resize_split(column)
            }
            MouseEventKind::Up(MouseButton::Left) => self.dragging_divider = false,
            _ => {}
        }
    }

    pub(super) fn handle_overlay_click(&mut self, column: u16, row: u16) {
        let point = (column, row).into();
        if !self.overlay_area.contains(point) {
            self.overlay = None;
            return;
        }
        if self.delete_cancel_area.contains(point) {
            self.overlay = None;
            return;
        }
        if self.delete_confirm_area.contains(point) {
            self.handle_overlay_key(KeyEvent::from(KeyCode::Enter));
            return;
        }
        // Every graph column has its own focus and selection.
        // 调用图的每一列都有独立焦点和选择项。
        if let Some(Overlay::Search(search)) = self.overlay.as_ref() {
            if search.graph_mode {
                let in_a_input = self.graph_a_input_area.contains(point);
                let in_a_center = self.graph_a_center_area.contains(point);
                let in_a_output = self.graph_a_output_area.contains(point);
                let in_b_input = self.graph_b_input_area.contains(point);
                let in_b_center = self.graph_b_center_area.contains(point);
                let in_b_output = self.graph_b_output_area.contains(point);
                let in_tree = self.graph_tree_a_area.contains(point)
                    || self.graph_tree_b_area.contains(point);
                let in_data = self.graph_data_a_area.contains(point)
                    || self.graph_data_b_area.contains(point);
                let tree_b = self.graph_tree_b_area.contains(point);
                let data_b = self.graph_data_b_area.contains(point);
                if in_a_input
                    || in_a_center
                    || in_a_output
                    || in_b_input
                    || in_b_center
                    || in_b_output
                {
                    let side = if in_b_input || in_b_center || in_b_output {
                        1
                    } else {
                        0
                    };
                    let input = if side == 0 { in_a_input } else { in_b_input };
                    let center = if side == 0 { in_a_center } else { in_b_center };
                    let area = if input {
                        if side == 0 {
                            self.graph_a_input_area
                        } else {
                            self.graph_b_input_area
                        }
                    } else if center {
                        if side == 0 {
                            self.graph_a_center_area
                        } else {
                            self.graph_b_center_area
                        }
                    } else if side == 0 {
                        self.graph_a_output_area
                    } else {
                        self.graph_b_output_area
                    };
                    let item = self.graph_item(search, side);
                    let (caller_len, callee_len) = item
                        .as_ref()
                        .map(|item| {
                            let (callers, callees) = self.call_relations(item.node, &item.function);
                            (callers.len(), callees.len())
                        })
                        .unwrap_or((0, 0));
                    let relation_index = row.saturating_sub(area.y.saturating_add(1)) as usize / 2;
                    let selected = if input {
                        relation_index.min(caller_len.saturating_sub(1))
                    } else if center {
                        caller_len
                    } else {
                        (caller_len + 1 + relation_index).min(caller_len + callee_len)
                    };
                    if let Some(Overlay::Search(search)) = self.overlay.as_mut() {
                        search.graph_focus = side;
                        search.graph_side = side;
                        if side == 0 {
                            search.graph_selected = selected;
                        } else {
                            search.compare_graph_selected = selected;
                        }
                    }
                    return;
                }
                if in_tree || in_data {
                    let side = if in_tree {
                        if tree_b {
                            1
                        } else {
                            0
                        }
                    } else if data_b {
                        1
                    } else {
                        0
                    };
                    let top = if in_tree {
                        if tree_b {
                            self.graph_tree_b_area.y
                        } else {
                            self.graph_tree_a_area.y
                        }
                    } else if data_b {
                        self.graph_data_b_area.y
                    } else {
                        self.graph_data_a_area.y
                    };
                    let raw_selected = row.saturating_sub(top.saturating_add(1)) as usize;
                    let selected = self
                        .graph_item(search, side)
                        .map(|item| {
                            if in_tree {
                                raw_selected
                                    .min(self.call_tree_targets(&item).len().saturating_sub(1))
                            } else {
                                raw_selected.min(self.graph_locals(&item).len().saturating_sub(1))
                            }
                        })
                        .unwrap_or_default();
                    if let Some(Overlay::Search(search)) = self.overlay.as_mut() {
                        search.graph_focus = if in_tree { 2 } else { 3 };
                        search.graph_side = side;
                        search.outline_focus = in_tree;
                        if in_tree {
                            if search.graph_side == 1 {
                                search.compare_outline_selected = selected;
                            } else {
                                search.outline_selected = selected;
                            }
                        } else {
                            if search.graph_side == 1 {
                                search.compare_data_selected = selected;
                            } else {
                                search.data_selected = selected;
                            }
                        }
                    }
                    return;
                }
            }
        }
        if self.action_cancel_area.contains(point) || self.action_exit_area.contains(point) {
            self.overlay = None;
            return;
        }
        if self.action_confirm_area.contains(point) {
            // A button click is a submit action even while a text field owns
            // keyboard input. Routing it through `s` used to type into the
            // field instead of saving.
            // 鼠标确认始终表示提交；不能伪装成字符 `s`，否则编辑字段会吞掉保存操作。
            match self.overlay.clone() {
                Some(Overlay::NewProject(project)) => self.submit_new_project(&project),
                Some(Overlay::Add(add)) => self.submit_add(&add),
                Some(Overlay::Edit(id, edit)) => self.submit_edit(id, &edit),
                Some(Overlay::Plugin(plugin)) => self.submit_plugin(&plugin),
                _ => {}
            }
            return;
        }
        if !self.overlay_list_area.contains(point)
            && !self.overlay_compare_list_area.contains(point)
        {
            return;
        }
        let clicked_compare = self.overlay_compare_list_area.contains(point);
        let visible_row = if clicked_compare {
            row.saturating_sub(self.overlay_compare_list_area.y.saturating_add(1)) as usize
        } else {
            row.saturating_sub(self.overlay_list_area.y.saturating_add(1)) as usize
        };
        let search_target = match self.overlay.as_ref() {
            Some(Overlay::Search(search)) => {
                let query = if clicked_compare {
                    search.compare_query.as_deref().unwrap_or("")
                } else {
                    &search.query
                };
                let last = self
                    .search_rows(query, &search.folded)
                    .len()
                    .saturating_sub(1);
                Some(
                    (if clicked_compare {
                        search.compare_offset
                    } else {
                        search.offset
                    } + visible_row)
                        .min(last),
                )
            }
            _ => None,
        };
        match self.overlay.as_mut() {
            Some(Overlay::Search(search)) => {
                if clicked_compare {
                    search.compare_selected = search_target.unwrap_or_default();
                    search.active_pane = 1;
                } else {
                    search.selected = search_target.unwrap_or_default();
                    search.active_pane = 0;
                }
            }
            Some(Overlay::NewProject(project)) if visible_row < project.values.len() => {
                project.field = visible_row;
                if visible_row == 2 {
                    project.values[2] = if project.values[2] == "binary" {
                        "library".to_owned()
                    } else {
                        "binary".to_owned()
                    };
                } else {
                    project.editing = true;
                }
            }
            Some(Overlay::Add(add)) => {
                let fields = face_field_indices(add);
                let Some(field) = fields.get(visible_row).copied() else {
                    return;
                };
                add.field = field;
                if field == 2 {
                    add.values[2] = (add.values[2] != "true").to_string();
                } else {
                    add.editing = true;
                }
            }
            Some(Overlay::Edit(_, edit)) => {
                let fields = face_field_indices(edit);
                let Some(field) = fields.get(visible_row).copied() else {
                    return;
                };
                edit.field = field;
                if field == 2 {
                    edit.values[2] = (edit.values[2] != "true").to_string();
                } else {
                    edit.editing = true;
                }
            }
            Some(Overlay::Plugin(plugin)) if visible_row < plugin.values.len() => {
                plugin.field = visible_row;
                if visible_row == 0 || visible_row == 6 {
                    plugin.values[visible_row] = match visible_row {
                        0 if plugin.values[0] == "official" => "user".to_owned(),
                        0 => "official".to_owned(),
                        6 if plugin.values[6] == "extension" => "replacement".to_owned(),
                        _ => "extension".to_owned(),
                    };
                } else {
                    plugin.editing = true;
                }
            }
            _ => {}
        }
    }
}
