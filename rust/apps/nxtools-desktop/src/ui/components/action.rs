use gpui::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Icon, Selectable, Sizable};

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::metrics::{CONTROL_HEIGHT_ACTION, RADIUS_CONTROL, RADIUS_NONE, SPACING_8};

pub(crate) type ClickHandler = Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static>;

/// One existing GPUI click boundary for pointer and keyboard activation.
/// GPUI generates Enter/Space clicks for a tracked focus handle; do not add
/// another activation handler, which would dispatch twice.
pub(crate) fn focused_action(
    id: impl Into<gpui::ElementId>,
    focus: &gpui::FocusHandle,
    theme: Theme,
    disabled: bool,
    window: &Window,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    let focused = focus.is_focused(window) && !disabled;
    let pointer_focus = focus.clone();
    let key_focus = focus.clone();
    div()
        .id(id)
        .relative()
        .border_2()
        .border_color(if focused {
            theme.focus_ring
        } else {
            gpui::transparent_black().into()
        })
        .rounded(px(RADIUS_CONTROL))
        .when(!disabled, |element| element.track_focus(focus))
        .when(!disabled, |element| element.cursor_pointer())
        .on_mouse_down(gpui::MouseButton::Left, move |_, window, _| {
            if !disabled {
                pointer_focus.focus(window);
            }
        })
        .capture_key_down(move |event, window, cx| {
            if disabled || !key_focus.is_focused(window) {
                return;
            }
            if event.keystroke.key == "tab"
                && !event.keystroke.modifiers.control
                && !event.keystroke.modifiers.alt
                && !event.keystroke.modifiers.platform
            {
                if event.keystroke.modifiers.shift {
                    window.focus_prev();
                } else {
                    window.focus_next();
                }
                window.prevent_default();
                cx.stop_propagation();
            } else if event.is_held && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                window.prevent_default();
                cx.stop_propagation();
            }
        })
        .on_click(move |event, window, cx| {
            if !disabled {
                on_click(event, window, cx);
            }
            cx.stop_propagation();
        })
}

pub(crate) fn focusable_content(
    id: impl Into<gpui::ElementId>,
    theme: Theme,
    disabled: bool,
    content: AnyElement,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    FocusableContent {
        id: id.into(),
        theme,
        disabled,
        content,
        on_click: Box::new(on_click),
    }
    .into_any_element()
}

#[derive(gpui::IntoElement)]
struct FocusableContent {
    id: gpui::ElementId,
    theme: Theme,
    disabled: bool,
    content: AnyElement,
    on_click: ClickHandler,
}
impl gpui::RenderOnce for FocusableContent {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            theme,
            disabled,
            content,
            on_click,
        } = self;
        let focus = window
            .use_keyed_state(id.clone(), cx, |_, cx| cx.focus_handle().tab_stop(true))
            .read(cx)
            .clone();
        let focused = focus.is_focused(window) && !disabled;
        focused_action(id, &focus, theme, disabled, window, on_click)
            .border_0()
            .w_full()
            .child(content)
            .when(focused, |action| {
                action.child(
                    div()
                        .absolute()
                        .inset_0()
                        .border_2()
                        .border_color(theme.focus_ring),
                )
            })
    }
}

pub(crate) struct SegmentedOption {
    pub id: &'static str,
    pub label: SharedString,
    pub on_click: ClickHandler,
}

impl SegmentedOption {
    pub(crate) fn new(
        id: &'static str,
        label: impl Into<SharedString>,
        on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            on_click: Box::new(on_click),
        }
    }
}

pub(crate) fn primary_button_variant(theme: Theme, cx: &App) -> ButtonCustomVariant {
    ButtonCustomVariant::new(cx)
        .color(theme.accent.into())
        .foreground(theme.image_well.into())
        .border(theme.accent.into())
        .hover(theme.accent_hover.into())
        .active(theme.accent.into())
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DangerHoverButtonPalette {
    rest: gpui::Rgba,
    foreground: gpui::Rgba,
    border: gpui::Rgba,
    hover: gpui::Rgba,
    active: gpui::Rgba,
}

fn danger_hover_button_palette(theme: Theme) -> DangerHoverButtonPalette {
    DangerHoverButtonPalette {
        rest: theme.surface_raised,
        foreground: theme.text_secondary,
        border: theme.border_subtle,
        hover: theme.danger_soft,
        active: theme.danger_soft,
    }
}

fn danger_ghost_button_palette(theme: Theme) -> DangerHoverButtonPalette {
    DangerHoverButtonPalette {
        rest: gpui::transparent_black().into(),
        foreground: theme.icon_muted,
        border: gpui::transparent_black().into(),
        hover: theme.danger_soft,
        active: theme.danger_soft,
    }
}

pub(crate) fn danger_hover_button_variant(theme: Theme, cx: &App) -> ButtonCustomVariant {
    let palette = danger_hover_button_palette(theme);
    ButtonCustomVariant::new(cx)
        .color(palette.rest.into())
        .foreground(palette.foreground.into())
        .border(palette.border.into())
        .hover(palette.hover.into())
        .active(palette.active.into())
}

pub(crate) fn danger_ghost_button_variant(theme: Theme, cx: &App) -> ButtonCustomVariant {
    let palette = danger_ghost_button_palette(theme);
    ButtonCustomVariant::new(cx)
        .color(palette.rest.into())
        .foreground(palette.foreground.into())
        .border(palette.border.into())
        .hover(palette.hover.into())
        .active(palette.active.into())
}

pub(crate) fn action_button(id: &'static str, label: impl Into<SharedString>) -> Button {
    // The debug selector makes every shared action button addressable by the
    // production geometry gates; it is a no-op outside test builds.
    Button::new(id)
        .debug_selector(move || id.to_string())
        .label(label)
        .small()
        .h(px(CONTROL_HEIGHT_ACTION))
        .px(px(SPACING_8))
        .rounded(px(RADIUS_CONTROL))
}

pub(crate) fn labelled_icon_action(
    id: &'static str,
    icon: MaterialIcon,
    label: &'static str,
) -> Button {
    action_button(id, label).icon(Icon::new(icon).size(px(14.0)))
}

pub(crate) fn icon_action(
    id: &'static str,
    icon: MaterialIcon,
    tooltip_label: &'static str,
) -> Button {
    icon_action_button(id, icon).tooltip(tooltip_label)
}

pub(crate) fn icon_action_button(id: &'static str, icon: MaterialIcon) -> Button {
    Button::new(id)
        .debug_selector(move || id.to_string())
        .icon(Icon::new(icon).size(px(14.0)))
        .small()
        .size(px(CONTROL_HEIGHT_ACTION))
        .rounded(px(RADIUS_CONTROL))
}

pub(crate) fn icon_action_tooltip_surface(
    button: Button,
    tooltip_label: &'static str,
    tooltip_selector: &'static str,
) -> AnyElement {
    div()
        .id(tooltip_selector)
        .tooltip(move |window, cx| {
            Tooltip::element(move |_, _| {
                div()
                    .debug_selector(move || tooltip_selector.into())
                    .child(tooltip_label)
            })
            .build(window, cx)
        })
        .child(button)
        .into_any_element()
}

pub(crate) fn segmented_control(
    theme: Theme,
    selector: &'static str,
    selected_id: &'static str,
    options: Vec<SegmentedOption>,
    cx: &App,
) -> AnyElement {
    assert_eq!(
        options
            .iter()
            .filter(|option| option.id == selected_id)
            .count(),
        1,
        "a segmented control must have exactly one selected option"
    );
    let selected_variant = ButtonCustomVariant::new(cx)
        .color(theme.accent_soft.into())
        .foreground(theme.text.into())
        .border(gpui::transparent_black())
        .hover(theme.surface_hover.into())
        .active(theme.element_selected.into());
    div()
        .debug_selector(move || selector.into())
        .h(px(CONTROL_HEIGHT_ACTION))
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(theme.border_subtle)
        .overflow_hidden()
        .flex()
        .children(options.into_iter().map(move |option| {
            let selected = option.id == selected_id;
            let button = Button::new(option.id)
                .label(option.label)
                .small()
                .h_full()
                .w_full()
                .rounded(px(RADIUS_NONE))
                .selected(selected)
                .when(selected, |button| button.custom(selected_variant))
                .when(!selected, |button| button.ghost())
                .on_click(option.on_click);
            div()
                .debug_selector(move || option.id.into())
                .h_full()
                .flex_1()
                .min_w_0()
                .child(button)
        }))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;

    struct FocusedActionFixture {
        focus: gpui::FocusHandle,
        calls: std::rc::Rc<std::cell::Cell<usize>>,
        disabled: bool,
    }

    impl gpui::Render for FocusedActionFixture {
        fn render(&mut self, window: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            let calls = self.calls.clone();
            focused_action(
                "foundation-action",
                &self.focus,
                Theme::zed_dark(),
                self.disabled,
                window,
                move |_, _, _| calls.set(calls.get() + 1),
            )
            .debug_selector(|| "foundation-action".into())
            .w(px(120.0))
            .h(px(CONTROL_HEIGHT_ACTION))
            .child("配置")
        }
    }

    #[gpui::test]
    fn foundation_focused_action_uses_one_dispatch_and_rejects_disabled_input(
        cx: &mut TestAppContext,
    ) {
        use gpui::{KeyDownEvent, KeyUpEvent, Keystroke, Modifiers};
        cx.update(gpui_component::init);
        cx.update(crate::theme::apply_component_theme);
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let fixture_calls = calls.clone();
        let (fixture, window) = cx.add_window_view(move |_, cx| FocusedActionFixture {
            focus: cx.focus_handle().tab_stop(true),
            calls: fixture_calls,
            disabled: false,
        });
        window.refresh().expect("render shared focused action");
        let bounds = window
            .debug_bounds("foundation-action")
            .expect("action hit bounds");
        window.simulate_click(bounds.center(), Modifiers::default());
        assert_eq!(calls.get(), 1);
        window.refresh().expect("paint focused action");
        for key in ["enter", "space"] {
            let before = calls.get();
            window.simulate_keystrokes(key);
            window.simulate_event(KeyDownEvent {
                keystroke: Keystroke::parse(key).expect("activation stroke"),
                is_held: true,
            });
            assert_eq!(
                calls.get(),
                before,
                "activation waits for release; repeat never dispatches"
            );
            window.simulate_event(KeyUpEvent {
                keystroke: Keystroke::parse(key).expect("release stroke"),
            });
            assert_eq!(
                calls.get(),
                before + 1,
                "one keyboard gesture must dispatch once"
            );
        }
        let before = calls.get();
        window.simulate_keystrokes("ctrl-enter");
        window.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("ctrl-enter").expect("modified release"),
        });
        assert_eq!(calls.get(), before);
        fixture.update(window, |fixture, cx| {
            fixture.disabled = true;
            cx.notify();
        });
        window.refresh().expect("render disabled/busy eligibility");
        window.simulate_click(bounds.center(), Modifiers::default());
        window.simulate_keystrokes("enter");
        window.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("enter").expect("disabled release"),
        });
        assert_eq!(
            calls.get(),
            before,
            "disabled or busy actions must reject pointer and keyboard"
        );
        assert_eq!(window.debug_bounds("foundation-action"), Some(bounds));
    }

    #[gpui::test]
    fn destructive_action_is_neutral_at_rest_and_danger_only_on_interaction(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let theme = Theme::zed_dark();
        let palette = danger_hover_button_palette(theme);
        assert_eq!(palette.rest, theme.surface_raised);
        assert_eq!(palette.foreground, theme.text_secondary);
        assert_eq!(palette.border, theme.border_subtle);
        assert_eq!(palette.hover, theme.danger_soft);
        assert_eq!(palette.active, theme.danger_soft);

        let actual = cx.update(|cx| danger_hover_button_variant(theme, cx));
        let expected = cx.update(|cx| {
            ButtonCustomVariant::new(cx)
                .color(palette.rest.into())
                .foreground(palette.foreground.into())
                .border(palette.border.into())
                .hover(palette.hover.into())
                .active(palette.active.into())
        });

        assert!(
            actual == expected,
            "the destructive action tier must stay neutral at rest and use danger_soft only for hover/press"
        );

        let ghost = danger_ghost_button_palette(theme);
        assert_eq!(ghost.rest, gpui::transparent_black().into());
        assert_eq!(ghost.foreground, theme.icon_muted);
        assert_eq!(ghost.border, gpui::transparent_black().into());
        assert_eq!(ghost.hover, theme.danger_soft);
        assert_eq!(ghost.active, theme.danger_soft);
    }

    #[gpui::test]
    fn primary_button_variant_provides_solid_accent_anchor(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let theme = Theme::zed_dark();
        let actual = cx.update(|cx| primary_button_variant(theme, cx));
        let expected = cx.update(|cx| {
            ButtonCustomVariant::new(cx)
                .color(theme.accent.into())
                .foreground(theme.image_well.into())
                .border(theme.accent.into())
                .hover(theme.accent_hover.into())
                .active(theme.accent.into())
        });
        assert!(
            actual == expected,
            "primary button must provide a solid accent visual anchor with high-contrast text"
        );
    }
}
