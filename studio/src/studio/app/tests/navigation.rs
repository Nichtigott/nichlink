//! Page routing and divider-drag tests.
//! 页面路由与分隔条拖拽测试。

use super::*;

#[test]
fn workspace_page_shortcuts_route_to_the_expected_mode() {
    let mut app = App::load();
    app.handle_key(KeyEvent::from(KeyCode::Char('1')));
    assert_eq!(app.page, StudioPage::Search);
    assert!(matches!(app.overlay, Some(Overlay::Search(_))));

    app.handle_key(KeyEvent::from(KeyCode::Esc));
    assert_eq!(app.page, StudioPage::Inspect);
    assert!(app.overlay.is_none());

    app.handle_key(KeyEvent::from(KeyCode::Char('3')));
    assert_eq!(app.page, StudioPage::Data);
    assert!(matches!(app.overlay, Some(Overlay::Search(ref search)) if search.graph_mode));

    app.handle_key(KeyEvent::from(KeyCode::Esc));
    app.handle_key(KeyEvent::from(KeyCode::Esc));
    app.handle_key(KeyEvent::from(KeyCode::Char('4')));
    assert_eq!(app.page, StudioPage::Compare);
    assert!(
        matches!(app.overlay, Some(Overlay::Search(ref search)) if search.compare_query.is_some())
    );
}

#[test]
fn divider_drag_is_clamped_and_stops_on_release() {
    let mut app = App::load();
    app.hot.workspace_area = Rect::new(10, 5, 100, 20);
    app.hot.tree_area = Rect::new(10, 5, 45, 20);

    app.handle_mouse(MouseEventKind::Down(MouseButton::Left), 55, 10);
    app.handle_mouse(MouseEventKind::Drag(MouseButton::Left), 105, 10);
    assert_eq!(app.split_percent, 70);

    app.handle_mouse(MouseEventKind::Up(MouseButton::Left), 105, 10);
    app.handle_mouse(MouseEventKind::Drag(MouseButton::Left), 35, 10);
    assert_eq!(app.split_percent, 70);
}
