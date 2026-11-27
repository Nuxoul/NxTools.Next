//! Production chrome boundary; synthetic input is not native/DPI acceptance.
use super::*;
use gpui_kit::{Context, Render, TestAppContext, Window, size};
use std::{cell::Cell, rc::Rc};

struct CaptionFixture(Rc<Cell<usize>>);
impl Render for CaptionFixture {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let calls = self.0.clone();
        client_titlebar(
            crate::theme::Theme::zed_dark(),
            "caption-fixture",
            "caption-close",
            "合成窗口",
            move |_, _, _| calls.set(calls.get() + 1),
        )
    }
}
#[gpui_kit::test]
fn shared_caption_matches_browser_height_and_single_owner_click(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let calls = Rc::new(Cell::new(0));
    let captured = calls.clone();
    let (_, surface) = cx.add_window_view(move |_, _| CaptionFixture(captured));
    surface.simulate_resize(size(px(480.), px(260.)));
    surface.refresh().expect("production caption");
    let title = surface
        .debug_bounds("caption-fixture")
        .expect("shared title");
    let close = surface
        .debug_bounds("caption-close")
        .expect("official caption Button");
    assert_eq!(title.size.height, px(34.));
    assert_eq!(close.size, size(px(34.), px(33.)));
    surface.simulate_event(gpui_kit::MouseDownEvent {
        position: close.center(),
        button: MouseButton::Left,
        ..Default::default()
    });
    surface.simulate_event(gpui_kit::MouseUpEvent {
        position: close.center(),
        button: MouseButton::Left,
        modifiers: Default::default(),
        click_count: 1,
    });
    surface.run_until_parked();
    assert_eq!(
        calls.get(),
        1,
        "the original close callback is the only dispatcher"
    );
    let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
    assert!(nodes.iter().any(|node| node.label() == Some("关闭")));
    assert!(surface.debug_bounds("window-minimize").is_none());
    assert!(surface.debug_bounds("window-maximize").is_none());
    surface.update(|window, cx| window.focus_next(cx));
    surface.simulate_keystrokes("space");
    surface.simulate_event(gpui_kit::KeyUpEvent {
        keystroke: gpui_kit::Keystroke::parse("space").expect("Space"),
    });
    surface.run_until_parked();
    assert_eq!(calls.get(), 2, "keyboard release uses the same one owner");
}
#[test]
fn custom_caption_options_disable_native_decoration_without_changing_visibility() {
    let options = caption_window_options();
    assert!(matches!(
        options.window_background,
        gpui_kit::WindowBackgroundAppearance::Transparent
    ));
    assert!(
        options.show && options.focus,
        "creation alpha does not change presentation policy"
    );
    assert!(
        options
            .titlebar
            .as_ref()
            .expect("explicit transparent native chrome")
            .appears_transparent
    );
    assert!(matches!(
        options.window_decorations,
        Some(gpui_kit::WindowDecorations::Client)
    ));
    assert!(
        !options.app_owns_titlebar_drag,
        "Windows retains its real caption move/double-click/restore loop"
    );
}
