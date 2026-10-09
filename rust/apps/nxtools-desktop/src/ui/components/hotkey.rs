use gpui::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};

use crate::theme::Theme;
use crate::ui::metrics::{CONTROL_HEIGHT_INPUT, RADIUS_CONTROL, SPACING_4, SPACING_8, SPACING_12};
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL, UI_MONO_FONT_FAMILY,
};

pub(crate) const HOTKEY_FIELD_WIDTH: f32 = 260.0;
const HOTKEY_STATE_MARKER_SIZE: f32 = 6.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HotkeyFieldState {
    Applied,
    Waiting,
    Recording,
    Applying,
    Failed,
}

pub(crate) fn hotkey_setting_row(
    theme: Theme,
    selector: &'static str,
    title: impl Into<SharedString>,
    description: impl Into<SharedString>,
    field: AnyElement,
) -> AnyElement {
    let title_selector = format!("{selector}-title");
    let description_selector = format!("{selector}-description");
    div()
        .debug_selector(move || selector.into())
        .min_h(px(64.0))
        .py(px(SPACING_8))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_center()
        .gap(px(SPACING_12))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(SPACING_4))
                .child(
                    div()
                        .debug_selector(move || title_selector.clone())
                        .h(px(LINE_HEIGHT_BODY))
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .text_color(theme.text)
                        .child(title.into()),
                )
                .child(
                    div()
                        .debug_selector(move || description_selector.clone())
                        .h(px(LINE_HEIGHT_LABEL))
                        .min_w_0()
                        .truncate()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(description.into()),
                ),
        )
        .child(field)
        .into_any_element()
}

impl HotkeyFieldState {
    fn selector_suffix(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Waiting => "waiting",
            Self::Recording => "recording",
            Self::Applying => "applying",
            Self::Failed => "failed",
        }
    }

    fn accepts_input(self) -> bool {
        matches!(self, Self::Applied | Self::Failed)
    }
}

pub(crate) fn hotkey_field(
    theme: Theme,
    selector: &'static str,
    value: impl Into<SharedString>,
    status: impl Into<SharedString>,
    state: HotkeyFieldState,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let value = value.into();
    let status = status.into();
    let control_selector = format!("{selector}-control");
    let value_selector = format!("{selector}-value");
    let status_selector = format!("{selector}-status");
    let state_selector = format!("{selector}-state-{}", state.selector_suffix());
    let (border_color, value_color, status_color, marker_color) = match state {
        HotkeyFieldState::Applied => (
            theme.border_subtle,
            theme.text,
            theme.text_muted,
            theme.success,
        ),
        HotkeyFieldState::Waiting => (
            theme.border,
            theme.text_secondary,
            theme.text_muted,
            theme.warning,
        ),
        HotkeyFieldState::Recording => (
            theme.border_active,
            theme.accent_text,
            theme.accent_text,
            theme.accent,
        ),
        HotkeyFieldState::Applying => (
            theme.border_active,
            theme.text_secondary,
            theme.text_muted,
            theme.accent,
        ),
        HotkeyFieldState::Failed => (theme.danger, theme.text, theme.danger, theme.danger),
    };

    let trigger = crate::ui::components::action::focusable_content(
        SharedString::from(format!("{selector}-action")),
        theme,
        !state.accepts_input(),
        div()
            .id(selector)
            .debug_selector(move || control_selector.clone())
            .h(px(CONTROL_HEIGHT_INPUT))
            .w_full()
            .rounded(px(RADIUS_CONTROL))
            .border_1()
            .border_color(border_color)
            .bg(theme.canvas)
            .px(px(SPACING_12))
            .flex()
            .items_center()
            .justify_center()
            .when(state.accepts_input(), |field| {
                field.hover(move |field| field.bg(theme.surface_hover))
            })
            .child(
                div()
                    .debug_selector(move || value_selector.clone())
                    .h(px(LINE_HEIGHT_BODY))
                    .max_w_full()
                    .truncate()
                    .overflow_hidden()
                    .whitespace_nowrap()
                    .font_family(UI_MONO_FONT_FAMILY)
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .text_color(value_color)
                    .child(value),
            )
            .into_any_element(),
        on_click,
    );
    div()
        .debug_selector(move || selector.into())
        .w(px(HOTKEY_FIELD_WIDTH))
        .min_w(px(HOTKEY_FIELD_WIDTH))
        .flex()
        .flex_col()
        .gap(px(SPACING_4))
        .child(trigger)
        .child(
            div()
                .h(px(LINE_HEIGHT_LABEL))
                .w_full()
                .flex()
                .items_center()
                .gap(px(SPACING_8))
                .child(
                    div()
                        .debug_selector(move || state_selector.clone())
                        .size(px(HOTKEY_STATE_MARKER_SIZE))
                        .flex_none()
                        .rounded(px(HOTKEY_STATE_MARKER_SIZE / 2.0))
                        .bg(marker_color),
                )
                .child(
                    div()
                        .debug_selector(move || status_selector.clone())
                        .h(px(LINE_HEIGHT_LABEL))
                        .min_w_0()
                        .flex_1()
                        .truncate()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(status_color)
                        .child(status),
                ),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};
    struct HotkeyFixture {
        calls: Rc<Cell<usize>>,
        state: HotkeyFieldState,
    }
    impl gpui::Render for HotkeyFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            hotkey_field(
                Theme::zed_dark(),
                "keyboard-hotkey",
                "Ctrl+Space",
                "合成录制入口",
                self.state,
                move |_, _, _| calls.set(calls.get() + 1),
            )
        }
    }
    #[gpui::test]
    fn hotkey_entry_keyboard_activates_once_and_rejects_applying(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        cx.update(crate::theme::apply_component_theme);
        let calls = Rc::new(Cell::new(0));
        let rendered_calls = calls.clone();
        let (fixture, window) = cx.add_window_view(move |_, _| HotkeyFixture {
            calls: rendered_calls,
            state: HotkeyFieldState::Applied,
        });
        window.refresh().expect("render actual hotkey entry");
        let bounds = window
            .debug_bounds("keyboard-hotkey-control")
            .expect("bounded recording trigger");
        window.update(|window, _| window.focus_next());
        window.refresh().expect("focus recording entry");
        for key in ["enter", "space"] {
            let before = calls.get();
            window.simulate_event(gpui::KeyDownEvent {
                keystroke: gpui::Keystroke::parse(key).expect("recording key"),
                is_held: false,
            });
            assert_eq!(
                calls.get(),
                before,
                "entry must activate on release, not record the trigger"
            );
            window.simulate_event(gpui::KeyUpEvent {
                keystroke: gpui::Keystroke::parse(key).expect("recording release"),
            });
            assert_eq!(calls.get(), before + 1);
        }
        fixture.update(window, |fixture, cx| {
            fixture.state = HotkeyFieldState::Applying;
            cx.notify();
        });
        window.refresh().expect("render applying eligibility");
        window.simulate_click(bounds.center(), gpui::Modifiers::default());
        window.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse("enter").expect("busy key"),
            is_held: false,
        });
        window.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").expect("busy release"),
        });
        assert_eq!(
            calls.get(),
            2,
            "applying rejects both pointer and keyboard entry"
        );
        assert_eq!(window.debug_bounds("keyboard-hotkey-control"), Some(bounds));
    }
}
