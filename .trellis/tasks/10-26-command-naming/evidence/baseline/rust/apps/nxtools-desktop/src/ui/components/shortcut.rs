//! Shortcut presentation only. Existing feature owners retain key handling and focus.
use gpui_kit::component::{kbd::Kbd, label::Label};
use gpui_kit::{Div, Keystroke, SharedString, div, prelude::*};

pub(crate) fn key(stroke: &'static str) -> Kbd {
    Kbd::new(Keystroke::parse(stroke).expect("valid static shortcut hint"))
}

pub(crate) fn hint(stroke: &'static str, label: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .flex_none()
        .child(key(stroke))
        .child(Label::new(label).text_xs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Context, IntoElement, Render, TestAppContext, Window, px, size};
    struct Hints;
    impl Render for Hints {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div().size_full().child(
                div()
                    .debug_selector(|| "shortcut-hints".into())
                    .w_full()
                    .flex()
                    .flex_wrap()
                    .gap_2()
                    .child(hint("ctrl-enter", "保存"))
                    .child(hint("escape", "取消")),
            )
        }
    }
    #[gpui_kit::test]
    fn official_shortcut_hints_paint_keys_inside_narrow_content_without_focus_owner(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (_, window) = cx.add_window_view(|_, _| Hints);
        window.simulate_resize(size(px(208.0), px(80.0)));
        window.refresh().expect("render real Kbd");
        let hints = window.debug_bounds("shortcut-hints").expect("hint row");
        for selector in ["kbd:ctrl-enter", "kbd:escape"] {
            let key = window
                .debug_bounds(selector)
                .expect("official Kbd paints selector");
            assert!(key.left() >= hints.left() && key.right() <= hints.right());
            assert!(key.top() >= hints.top() && key.bottom() <= hints.bottom());
        }
        assert!(
            window.update(|window, _| gpui_kit::base::test_support::snapshots(window)
                .iter()
                .all(|node| node.role() != Some(gpui_kit::Role::Button))),
            "hints do not introduce actionable controls"
        );
    }
}
