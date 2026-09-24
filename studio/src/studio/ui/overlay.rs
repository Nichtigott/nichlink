//! Overlay router: centered modal sizing and per-overlay dispatch.
//! 浮层路由：居中模态尺寸计算与逐浮层分发。

use super::*;

pub(super) fn draw_overlay(frame: &mut Frame<'_>, app: &mut App) {
    let Some(overlay) = app.overlay.clone() else {
        app.hot.overlay_area = Rect::default();
        app.hot.overlay_list_area = Rect::default();
        app.hot.delete_cancel_area = Rect::default();
        app.hot.delete_confirm_area = Rect::default();
        app.hot.action_cancel_area = Rect::default();
        app.hot.action_confirm_area = Rect::default();
        app.hot.action_exit_area = Rect::default();
        app.hot.graft_compose_area = Rect::default();
        app.hot.graph_area = Rect::default();
        app.hot.graph_tree_area = Rect::default();
        app.hot.graph_data_area = Rect::default();
        app.hot.graph_detail_area = Rect::default();
        app.hot.graph_provenance_area = Rect::default();
        app.hot.action_validate_area = Rect::default();
        app.hot.action_edit_area = Rect::default();
        return;
    };
    let area = if matches!(overlay, Overlay::Search(ref search) if search.graph_mode) {
        centered(98, 92, frame.area())
    } else {
        centered(86, 78, frame.area())
    };
    app.hot.overlay_area = area;
    app.hot.overlay_list_area = Rect::default();
    app.hot.delete_cancel_area = Rect::default();
    app.hot.delete_confirm_area = Rect::default();
    app.hot.action_cancel_area = Rect::default();
    app.hot.action_validate_area = Rect::default();
    app.hot.action_edit_area = Rect::default();
    app.hot.action_confirm_area = Rect::default();
    app.hot.action_exit_area = Rect::default();
    app.hot.graft_compose_area = Rect::default();
    app.hot.graph_area = Rect::default();
    app.hot.graph_tree_area = Rect::default();
    app.hot.graph_data_area = Rect::default();
    app.hot.graph_detail_area = Rect::default();
    app.hot.graph_provenance_area = Rect::default();
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().style(Style::default().bg(PANEL)), area);
    match overlay {
        Overlay::Search(search) => {
            let (list_area, offset) = draw_search(frame, area, app, &search);
            app.hot.overlay_list_area = list_area;
            if let Some(Overlay::Search(current)) = &mut app.overlay {
                current.offset = offset;
            }
        }
        Overlay::NewProject(project) => {
            let (list, cancel, confirm, exit) = draw_new_project(frame, area, &project);
            app.hot.overlay_list_area = list;
            app.hot.action_cancel_area = cancel;
            app.hot.action_confirm_area = confirm;
            app.hot.action_exit_area = exit;
        }
        Overlay::Add(add) => {
            let (list, cancel, confirm, exit) = draw_add(frame, area, &add);
            app.hot.overlay_list_area = list;
            app.hot.action_cancel_area = cancel;
            app.hot.action_confirm_area = confirm;
            app.hot.action_exit_area = exit;
        }
        Overlay::Edit(_, edit) => {
            let (list, cancel, confirm, exit) = draw_edit(frame, area, &edit);
            app.hot.overlay_list_area = list;
            app.hot.action_cancel_area = cancel;
            app.hot.action_confirm_area = confirm;
            app.hot.action_exit_area = exit;
        }
        Overlay::Plugin(plugin) => {
            let (list, cancel, confirm, exit) = draw_plugin(frame, area, &plugin);
            app.hot.overlay_list_area = list;
            app.hot.action_cancel_area = cancel;
            app.hot.action_confirm_area = confirm;
            app.hot.action_exit_area = exit;
        }
        Overlay::Graft(graft) => {
            let (list, cancel, confirm, exit, compose) = draw_graft(frame, area, &graft);
            app.hot.overlay_list_area = list;
            app.hot.action_cancel_area = cancel;
            app.hot.action_confirm_area = confirm;
            app.hot.action_exit_area = exit;
            app.hot.graft_compose_area = compose;
        }
        Overlay::Delete(id) => {
            let (cancel, confirm) = draw_delete(frame, area, app, id);
            app.hot.delete_cancel_area = cancel;
            app.hot.delete_confirm_area = confirm;
        }
    }
}
