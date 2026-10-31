//! Audit #3: inspect actual shared Button nodes, not tooltip/content wrappers.
use super::*;
use gpui_kit::base::{Root, test_support};
use gpui_kit::component::Disableable;
use gpui_kit::{Context, ElementId, Entity, KeyUpEvent, Keystroke, Render, Role, TestAppContext};
use std::{cell::Cell, rc::Rc};

struct NamedActions {
    disabled: bool,
    loading: bool,
    custom: bool,
    calls: Rc<Cell<usize>>,
}
impl Render for NamedActions {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let calls = self.calls.clone();
        let icon_calls = self.calls.clone();
        let content = if self.custom {
            gpui_kit::canvas(|_, _, _| (), |_, (), _, _| {})
                .size(px(1.0))
                .into_any_element()
        } else {
            "应用".into_any_element()
        };
        div()
            .flex()
            .gap_2()
            .child(
                primary_action_button(
                    "named-primary",
                    "应用全局设置",
                    content,
                    Theme::zed_dark(),
                    self.disabled,
                    cx,
                )
                .loading(self.loading)
                .w(px(96.0))
                .h(px(32.0))
                .on_click(move |_, _, _| calls.set(calls.get() + 1)),
            )
            .child(icon_action_tooltip_surface(
                icon_action_button("named-icon", MaterialIcon::Folder, "选择默认保存目录")
                    .disabled(self.disabled)
                    .loading(self.loading)
                    .on_click(move |_, _, _| icon_calls.set(icon_calls.get() + 1)),
                "选择默认保存目录",
                "named-icon-tooltip",
            ))
            .child(icon_action(
                "already-named-icon",
                MaterialIcon::ContentCopy,
                "复制配置路径",
            ))
    }
}
fn actual(
    surface: &mut gpui_kit::VisualTestContext,
    id: &'static str,
) -> test_support::ElementSnapshot {
    surface
        .update(|window, _| test_support::snapshots(window))
        .into_iter()
        .find(|node| node.path().last() == Some(&ElementId::from(id)))
        .expect("actual original Button node")
}
fn fixture(
    cx: &mut TestAppContext,
) -> (
    Entity<NamedActions>,
    gpui_kit::AnyWindowHandle,
    Rc<Cell<usize>>,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let calls = Rc::new(Cell::new(0));
    let held = calls.clone();
    let view = cx.new(|_| NamedActions {
        disabled: false,
        loading: false,
        custom: false,
        calls: held,
    });
    let content = view.clone();
    let (_, surface) = cx.add_window_view(move |window, cx| Root::new(content, window, cx));
    surface.update(|window, _| window.activate_window());
    surface.refresh().expect("render actual helper Buttons");
    let handle = surface.update(|window, _| window.window_handle());
    (view, handle, calls)
}
#[gpui_kit::test]
fn accessible_name_icon_is_on_button_not_outer_tooltip(cx: &mut TestAppContext) {
    let (view, handle, calls) = fixture(cx);
    let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
    for (disabled, loading) in [(false, false), (true, false), (false, true)] {
        view.update_in(&mut surface, |view, _, cx| {
            view.disabled = disabled;
            view.loading = loading;
            cx.notify();
        });
        surface.refresh().expect("same icon semantic owner");
        let button = actual(&mut surface, "named-icon");
        assert_eq!(button.role(), Some(Role::Button));
        assert_eq!(
            button.label(),
            Some("选择默认保存目录"),
            "outer tooltip must not substitute for actual Button name"
        );
        let count = surface
            .update(|window, _| test_support::snapshots(window))
            .iter()
            .filter(|node| {
                node.role() == Some(Role::Button) && node.label() == Some("复制配置路径")
            })
            .count();
        assert_eq!(
            count, 1,
            "already-named icon_action retains one semantic Button"
        );
        let before = calls.get();
        surface.simulate_click(button.bounds().center(), Default::default());
        assert_eq!(calls.get(), before + usize::from(!disabled && !loading));
        if !disabled && !loading {
            surface.update(|window, cx| window.blur(cx));
            for _ in 0..4 {
                surface.update(|window, cx| window.focus_next(cx));
                surface.refresh().expect("actual icon Tab focus frame");
                if surface
                    .update(|window, _| test_support::has_observed_focus(window, button.path()))
                {
                    break;
                }
            }
            assert!(
                surface.update(|window, _| test_support::has_observed_focus(window, button.path())),
                "actual icon must own keyboard focus"
            );
            for key in ["enter", "space"] {
                let before = calls.get();
                surface.simulate_keystrokes(key);
                assert_eq!(calls.get(), before);
                surface.simulate_event(KeyUpEvent {
                    keystroke: Keystroke::parse(key).expect("icon release"),
                });
                assert_eq!(calls.get(), before + 1);
            }
        }
    }
}
#[gpui_kit::test]
fn accessible_name_primary_custom_content_states_keep_original_dispatch_and_bounds(
    cx: &mut TestAppContext,
) {
    let (view, handle, calls) = fixture(cx);
    let mut surface = gpui_kit::VisualTestContext::from_window(handle, cx);
    let bounds = actual(&mut surface, "named-primary").bounds();
    for custom in [false, true] {
        for (disabled, loading) in [(false, false), (true, false), (false, true)] {
            view.update_in(&mut surface, |view, _, cx| {
                view.custom = custom;
                view.disabled = disabled;
                view.loading = loading;
                cx.notify();
            });
            surface.refresh().expect("actual primary semantic state");
            let button = actual(&mut surface, "named-primary");
            assert_eq!(button.role(), Some(Role::Button));
            assert_eq!(
                button.label(),
                Some("应用全局设置"),
                "child content must not be confused with explicit Button name"
            );
            assert_eq!(button.bounds(), bounds);
            let before = calls.get();
            surface.simulate_click(bounds.center(), Default::default());
            if disabled || loading {
                assert_eq!(calls.get(), before);
            } else {
                assert_eq!(calls.get(), before + 1);
            }
            surface.update(|window, cx| {
                window.blur(cx);
                window.focus_next(cx);
            });
            surface.refresh().expect("actual Tab target");
            for key in ["enter", "space"] {
                let before = calls.get();
                surface.simulate_keystrokes(key);
                assert_eq!(calls.get(), before, "original activation waits for release");
                surface.simulate_event(KeyUpEvent {
                    keystroke: Keystroke::parse(key).expect("release"),
                });
                assert_eq!(calls.get(), before + usize::from(!disabled && !loading));
            }
        }
    }
}
