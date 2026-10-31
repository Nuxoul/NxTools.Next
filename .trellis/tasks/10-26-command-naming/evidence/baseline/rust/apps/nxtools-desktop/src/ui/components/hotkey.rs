use gpui_kit::component::{Disableable, button::Button, label::Label};
use gpui_kit::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*};

use crate::theme::Theme;
use crate::ui::metrics::ordinary_rem;
use crate::ui::typography::UI_MONO_FONT_FAMILY;

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
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let value = value.into();
    let status = status.into();
    let control_selector = format!("{selector}-control");
    let value_selector = format!("{selector}-value");
    let status_selector = format!("{selector}-status");
    let state_selector = format!("{selector}-state-{}", state.selector_suffix());
    let (value_color, status_color, marker_color) = match state {
        HotkeyFieldState::Applied => (theme.text, theme.text_muted, theme.success),
        HotkeyFieldState::Waiting => (theme.text_secondary, theme.text_muted, theme.warning),
        HotkeyFieldState::Recording => (theme.accent_text, theme.accent_text, theme.accent),
        HotkeyFieldState::Applying => (theme.text_secondary, theme.text_muted, theme.accent),
        HotkeyFieldState::Failed => (theme.text, theme.danger, theme.danger),
    };

    let trigger = div()
        .debug_selector(move || control_selector.clone())
        .w_full()
        .child(
            Button::new(SharedString::from(format!("{selector}-action")))
                .outline()
                .w_full()
                .disabled(!state.accepts_input())
                .accessibility_label(value.clone())
                .child(
                    div()
                        .debug_selector(move || value_selector.clone())
                        .max_w_full()
                        .truncate()
                        .font_family(UI_MONO_FONT_FAMILY)
                        .text_color(value_color)
                        .child(Label::new(value).text_sm()),
                )
                .on_click(on_click),
        );
    div()
        .debug_selector(move || selector.into())
        .w(ordinary_rem(HOTKEY_FIELD_WIDTH))
        .max_w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap_1()
        .child(trigger)
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .debug_selector(move || state_selector.clone())
                        .size(ordinary_rem(HOTKEY_STATE_MARKER_SIZE))
                        .flex_none()
                        .rounded_full()
                        .bg(marker_color),
                )
                .child(
                    div()
                        .debug_selector(move || status_selector.clone())
                        .min_w_0()
                        .flex_1()
                        .text_color(status_color)
                        .child(Label::new(status).text_sm()),
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
    impl gpui_kit::Render for HotkeyFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
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
    #[gpui_kit::test]
    fn hotkey_entry_keyboard_activates_once_and_rejects_applying(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let calls = Rc::new(Cell::new(0));
        let rendered_calls = calls.clone();
        let (fixture, window) = cx.add_window_view(move |_, _| HotkeyFixture {
            calls: rendered_calls,
            state: HotkeyFieldState::Applied,
        });
        window.refresh().expect("render actual hotkey entry");
        let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        assert!(
            nodes.iter().any(|node| node.path().last()
                == Some(&gpui_kit::ElementId::from("keyboard-hotkey-action"))
                && node.label().is_some_and(|label| label == "Ctrl+Space")),
            "actual styled recorder Button with complete accessible value"
        );
        let bounds = window
            .debug_bounds("keyboard-hotkey-control")
            .expect("bounded recording trigger");
        window.update(|window, cx| window.focus_next(cx));
        window.refresh().expect("focus recording entry");
        assert!(
            window.update(
                |window, _| gpui_kit::base::test_support::snapshots(window)
                    .iter()
                    .any(|node| node.path().last()
                        == Some(&gpui_kit::ElementId::from("keyboard-hotkey-action"))
                        && node.focused() == Some(true))
            ),
            "actual recorder focus before key release"
        );
        for key in ["enter", "space"] {
            let before = calls.get();
            window.simulate_event(gpui_kit::KeyDownEvent {
                keystroke: gpui_kit::Keystroke::parse(key).expect("recording key"),
                is_held: false,
                prefer_character_input: false,
            });
            assert_eq!(
                calls.get(),
                before,
                "entry must activate on release, not record the trigger"
            );
            window.simulate_event(gpui_kit::KeyUpEvent {
                keystroke: gpui_kit::Keystroke::parse(key).expect("recording release"),
            });
            assert_eq!(calls.get(), before + 1);
        }
        fixture.update(window, |fixture, cx| {
            fixture.state = HotkeyFieldState::Applying;
            cx.notify();
        });
        window.refresh().expect("render applying eligibility");
        window.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
        window.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: gpui_kit::Keystroke::parse("enter").expect("busy key"),
            is_held: false,
            prefer_character_input: false,
        });
        window.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("enter").expect("busy release"),
        });
        assert_eq!(
            calls.get(),
            2,
            "applying rejects both pointer and keyboard entry"
        );
        assert_eq!(window.debug_bounds("keyboard-hotkey-control"), Some(bounds));
    }
}
