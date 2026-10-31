use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{AnyView, App, SharedString, Window, div, prelude::*, px};

use crate::ui::typography::{LINE_HEIGHT_LABEL, TEXT_LABEL, UI_MONO_FONT_FAMILY};

pub(crate) const TOOLTIP_MAX_WIDTH: f32 = 344.0;
pub(crate) const TOOLTIP_MAX_HEIGHT: f32 = 136.0;
pub(crate) const TOOLTIP_SURFACE_MAX_WIDTH: f32 = 360.0;
pub(crate) const TOOLTIP_SURFACE_MAX_HEIGHT: f32 = 160.0;

pub(crate) fn bounded_text_tooltip(
    selector: impl Into<SharedString>,
    message: impl Into<SharedString>,
    monospace: bool,
) -> impl Fn(&mut Window, &mut App) -> AnyView {
    let selector = selector.into();
    let message = message.into();
    move |window, cx| {
        let tooltip_selector = selector.clone();
        let tooltip_message = message.clone();
        Tooltip::element(move |_, _| {
            let selector = tooltip_selector.clone();
            let message = tooltip_message.clone();
            div()
                .debug_selector(move || selector.to_string())
                .max_w(px(TOOLTIP_MAX_WIDTH))
                .max_h(px(TOOLTIP_MAX_HEIGHT))
                .overflow_y_scrollbar()
                .whitespace_normal()
                .when(monospace, |surface| {
                    surface.font_family(UI_MONO_FONT_FAMILY)
                })
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .child(message)
        })
        .max_w(px(TOOLTIP_SURFACE_MAX_WIDTH))
        .max_h(px(TOOLTIP_SURFACE_MAX_HEIGHT))
        .build(window, cx)
    }
}

/// Optional explanatory copy: one focusable Kit information button and one
/// official Tooltip owner. No business callback or custom popup controller.
pub(crate) fn description_help(
    selector: impl Into<SharedString>,
    message: impl Into<SharedString>,
) -> gpui_kit::AnyElement {
    use crate::icons::MaterialIcon;
    use gpui_kit::base::TestSupportExt;
    use gpui_kit::component::{
        Icon, Sizable,
        button::{Button, ButtonVariants},
    };
    let selector = selector.into();
    let message = message.into();
    let tooltip_selector = format!("{selector}-tooltip");
    let accessible_description: SharedString = format!("说明：{message}").into();
    let button_selector = selector.clone();
    div()
        .id(selector.clone())
        .debug_selector(move || button_selector.to_string())
        .flex_none()
        .tooltip(move |window, cx| {
            let message = message.clone();
            let selector = tooltip_selector.clone();
            Tooltip::element(move |_, _| {
                let selector = selector.clone();
                div()
                    .id(SharedString::from(selector.clone()))
                    .test_support()
                    .debug_selector(move || selector.clone())
                    .max_w(px(TOOLTIP_MAX_WIDTH))
                    .whitespace_normal()
                    .child(message.clone())
            })
            .build(window, cx)
        })
        .child(
            Button::new(gpui_kit::SharedString::from(format!("{selector}-button")))
                .icon(Icon::new(MaterialIcon::Info))
                .xsmall()
                .ghost()
                .accessibility_label(accessible_description),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{Context, IntoElement, Modifiers, Render, TestAppContext, size};
    use std::time::Duration;
    struct HelpFixture {
        start_focus: gpui_kit::FocusHandle,
    }
    impl Render for HelpFixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
                .p(px(16.0))
                .flex()
                .items_start()
                .tab_group()
                .child(
                    div()
                        .id("help-focus-start")
                        .debug_selector(|| "help-focus-start".into())
                        .track_focus(&self.start_focus)
                        .tab_stop(true)
                        .child("起点"),
                )
                .child(description_help(
                    "test-feature-help",
                    "完整功能说明，不截断也不占常驻说明行。",
                ))
        }
    }
    #[gpui_kit::test]
    fn description_help_uses_real_kit_hover_and_focus_without_business_effects(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let executor = cx.executor();
        let start_focus = cx.update(|cx| cx.focus_handle());
        let fixture_focus = start_focus.clone();
        let (_, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| HelpFixture {
                start_focus: fixture_focus,
            });
            gpui_kit::base::Root::new(view, window, cx)
        });
        window.simulate_resize(size(px(460.0), px(320.0)));
        window.update(|window, _| window.activate_window());
        window.run_until_parked();
        window.refresh().expect("initial help trigger");
        let trigger = window
            .debug_bounds("test-feature-help")
            .expect("compact trigger");
        assert!(trigger.size.width <= px(24.0));
        let has_tooltip = |window: &mut gpui_kit::VisualTestContext| {
            window.update(|window, _| {
                gpui_kit::base::test_support::snapshots(window)
                    .iter()
                    .any(|s| {
                        s.path()
                            .iter()
                            .any(|id| id == &gpui_kit::ElementId::from("test-feature-help-tooltip"))
                    })
            })
        };
        assert!(!has_tooltip(window));
        window.simulate_mouse_move(trigger.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(700));
        window.run_until_parked();
        window.refresh().expect("Kit tooltip hover frame");
        assert!(
            window.debug_bounds("test-feature-help-tooltip").is_some(),
            "official Tooltip renders the complete copy"
        );
        assert!(has_tooltip(window));
        window.simulate_mouse_move(
            gpui_kit::point(px(440.0), px(300.0)),
            None,
            Modifiers::default(),
        );
        executor.advance_clock(Duration::from_millis(700));
        window.run_until_parked();
        window.refresh().expect("dismiss help before focus test");
        assert!(!has_tooltip(window));
        window.update(|window, cx| start_focus.focus(window, cx));
        window.run_until_parked();
        window
            .refresh()
            .expect("paint explicit initial focus before Tab");
        assert!(
            window.update(|window, _| start_focus.is_focused(window)),
            "initial fixture focus exists before Tab"
        );
        window.simulate_keystrokes("tab");
        window.run_until_parked();
        window.refresh().expect("focusable information button");
        let help_snapshots =
            window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        assert!(
            help_snapshots.iter().any(|s| s.label()
                == Some("说明：完整功能说明，不截断也不占常驻说明行。")
                && s.focused() == Some(true)),
            "keyboard help snapshot: {help_snapshots:?}"
        );
        window.simulate_mouse_move(
            gpui_kit::point(px(440.0), px(300.0)),
            None,
            Modifiers::default(),
        );
        executor.advance_clock(Duration::from_millis(700));
        window.run_until_parked();
        window.refresh().expect("tooltip leaves with trigger");
        assert!(!has_tooltip(window));
        assert_eq!(
            window.debug_bounds("test-feature-help"),
            Some(trigger),
            "show/hide/focus does not move layout"
        );
    }
}
