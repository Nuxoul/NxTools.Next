use gpui_kit::component::button::{Button, ButtonCustomVariant, ButtonGroup, ButtonVariants};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Selectable, Sizable};
use gpui_kit::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};

use crate::icons::MaterialIcon;
use crate::theme::Theme;
#[cfg(test)]
use crate::ui::metrics::CONTROL_HEIGHT_ACTION;
use crate::ui::metrics::RADIUS_CONTROL;

pub(crate) type ClickHandler = Box<dyn Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static>;

/// One existing GPUI click boundary for pointer and keyboard activation.
/// GPUI generates Enter/Space clicks for a tracked focus handle; do not add
/// another activation handler, which would dispatch twice.
pub(crate) fn focused_action(
    id: impl Into<gpui_kit::ElementId>,
    focus: &gpui_kit::FocusHandle,
    theme: Theme,
    disabled: bool,
    window: &Window,
    _cx: &mut App,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
) -> gpui_kit::base::Button {
    let focused = focus.is_focused(window) && !disabled && window.last_input_was_keyboard();
    gpui_kit::base::Button::new(id)
        .track_focus(focus)
        .disabled(disabled)
        .relative()
        .border_2()
        .border_color(if focused {
            theme.focus_ring
        } else {
            gpui_kit::transparent_black().into()
        })
        .focus_visible(move |style| style.border_color(theme.focus_ring))
        .rounded(px(RADIUS_CONTROL))
        .when(!disabled, |element| element.cursor_pointer())
        .on_click(move |event, window, cx| {
            on_click(event, window, cx);
            cx.stop_propagation();
        })
}

pub(crate) fn focusable_content(
    id: impl Into<gpui_kit::ElementId>,
    theme: Theme,
    disabled: bool,
    content: AnyElement,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
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

#[derive(gpui_kit::IntoElement)]
struct FocusableContent {
    id: gpui_kit::ElementId,
    theme: Theme,
    disabled: bool,
    content: AnyElement,
    on_click: ClickHandler,
}
impl gpui_kit::RenderOnce for FocusableContent {
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
        let focused = focus.is_focused(window) && !disabled && window.last_input_was_keyboard();
        focused_action(id, &focus, theme, disabled, window, cx, on_click)
            .border_0()
            .w_full()
            .flex()
            .flex_col()
            .child(div().w_full().min_w_0().child(content))
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
        on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id,
            label: label.into(),
            on_click: Box::new(on_click),
        }
    }
}

pub(crate) fn primary_button_variant(_theme: Theme, cx: &App) -> ButtonCustomVariant {
    let colors = gpui_kit::component::Theme::global(cx).colors;
    ButtonCustomVariant::new(cx)
        .color(colors.button_primary)
        .foreground(colors.button_primary_foreground)
        .hover(colors.button_primary_hover)
        .active(colors.button_primary_active)
}

/// Custom visual content does not provide a Button name; callers must supply its action meaning.
pub(crate) fn primary_action_button(
    id: &'static str,
    accessible_name: impl Into<SharedString>,
    label: impl IntoElement,
    theme: Theme,
    disabled: bool,
    _cx: &App,
) -> Button {
    use gpui_kit::component::Disableable;
    Button::new(id)
        .debug_selector(move || id.to_string())
        .accessibility_label(accessible_name)
        .small()
        .primary()
        .disabled(disabled)
        .when(disabled, |button| button.bg(theme.surface_raised))
        .child(
            div()
                .when(disabled, |label| label.text_color(theme.text_secondary))
                .child(label),
        )
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct DangerHoverButtonPalette {
    rest: gpui_kit::Rgba,
    foreground: gpui_kit::Rgba,
    border: gpui_kit::Rgba,
    hover: gpui_kit::Rgba,
    active: gpui_kit::Rgba,
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
        rest: gpui_kit::transparent_black().into(),
        foreground: theme.icon_muted,
        border: gpui_kit::transparent_black().into(),
        hover: theme.danger_soft,
        active: theme.danger_soft,
    }
}

pub(crate) fn danger_hover_button_variant(theme: Theme, cx: &App) -> ButtonCustomVariant {
    let palette = danger_hover_button_palette(theme);
    ButtonCustomVariant::new(cx)
        .color(palette.rest.into())
        .foreground(palette.foreground.into())
        .hover(palette.hover.into())
        .active(palette.active.into())
}

pub(crate) fn danger_ghost_button_variant(theme: Theme, cx: &App) -> ButtonCustomVariant {
    let palette = danger_ghost_button_palette(theme);
    ButtonCustomVariant::new(cx)
        .color(palette.rest.into())
        .foreground(palette.foreground.into())
        .hover(palette.hover.into())
        .active(palette.active.into())
}

pub(crate) fn action_button(id: &'static str, label: impl Into<SharedString>) -> Button {
    // The debug selector makes every shared action button addressable by the
    // production geometry gates; it is a no-op outside test builds.
    Button::new(id)
        .secondary()
        .debug_selector(move || id.to_string())
        .label(label)
        .small()
}

pub(crate) fn labelled_icon_action(
    id: &'static str,
    icon: MaterialIcon,
    label: &'static str,
) -> Button {
    action_button(id, label).icon(Icon::new(icon).small())
}

pub(crate) fn icon_action(
    id: &'static str,
    icon: MaterialIcon,
    tooltip_label: &'static str,
) -> Button {
    icon_action_button(id, icon, tooltip_label).tooltip(tooltip_label)
}

/// Low-level icon Button: the semantic name is required even when an outer Tooltip owns hover.
pub(crate) fn icon_action_button(
    id: &'static str,
    icon: MaterialIcon,
    accessible_name: impl Into<SharedString>,
) -> Button {
    Button::new(id)
        .secondary()
        .debug_selector(move || id.to_string())
        .accessibility_label(accessible_name)
        .icon(Icon::new(icon).small())
        .small()
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
    _theme: Theme,
    selector: &'static str,
    selected_id: &'static str,
    options: Vec<SegmentedOption>,
    _cx: &App,
) -> AnyElement {
    segmented_control_sized(
        selector,
        selected_id,
        options,
        gpui_kit::component::Size::Small,
    )
}

pub(crate) fn input_segmented_control(
    _theme: Theme,
    selector: &'static str,
    selected_id: &'static str,
    options: Vec<SegmentedOption>,
    _cx: &App,
) -> AnyElement {
    segmented_control_sized(
        selector,
        selected_id,
        options,
        gpui_kit::component::Size::Medium,
    )
}

fn segmented_control_sized(
    selector: &'static str,
    selected_id: &'static str,
    options: Vec<SegmentedOption>,
    size: gpui_kit::component::Size,
) -> AnyElement {
    assert_eq!(
        options
            .iter()
            .filter(|option| option.id == selected_id)
            .count(),
        1,
        "a segmented control must have exactly one selected option"
    );
    let option_width = 1.0 / options.len() as f32;
    div()
        .debug_selector(move || selector.into())
        .w_full()
        .min_w_0()
        .child(
            ButtonGroup::new(selector)
                .with_size(size)
                .outline()
                .w_full()
                .children(options.into_iter().map(move |option| {
                    Button::new(option.id)
                        .debug_selector(move || option.id.into())
                        .label(option.label)
                        .selected(option.id == selected_id)
                        .w(gpui_kit::relative(option_width))
                        .flex_none()
                        .min_w_0()
                        .whitespace_nowrap()
                        .on_click(option.on_click)
                })),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {

    #[gpui_kit::test]
    fn installed_navigation_pointer_focus_has_no_outline_but_keyboard_focus_does(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let action_calls = calls.clone();
        let fixture = cx.new(move |cx| FocusedActionFixture {
            focus: cx.focus_handle().tab_stop(true),
            calls: action_calls,
            disabled: false,
            outline: std::cell::Cell::new(false),
        });
        let view = fixture.clone();
        let (_, window) =
            cx.add_window_view(move |window, cx| gpui_kit::base::Root::new(view, window, cx));
        window.refresh().expect("production action");
        let bounds = window
            .debug_bounds("foundation-action")
            .expect("real hit bounds");
        window.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
        window.refresh().expect("pointer focus presentation");
        assert_eq!(calls.get(), 1);
        assert!(
            !fixture.read_with(window, |fixture, _| fixture.outline.get()),
            "mouse click must not leave the settings rectangle"
        );
        window.simulate_keystrokes("tab");
        window.refresh().expect("keyboard focus presentation");
        assert!(
            fixture.read_with(window, |fixture, _| fixture.outline.get()),
            "Tab must retain a visible keyboard focus indicator"
        );
        assert_eq!(window.debug_bounds("foundation-action"), Some(bounds));
        window.simulate_keystrokes("enter");
        window.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("enter").expect("Enter"),
        });
        assert_eq!(calls.get(), 2, "existing KeyUp boundary dispatches once");
    }

    use gpui_kit::TestAppContext;

    use super::*;

    struct FocusedActionFixture {
        focus: gpui_kit::FocusHandle,
        calls: std::rc::Rc<std::cell::Cell<usize>>,
        disabled: bool,
        outline: std::cell::Cell<bool>,
    }

    impl gpui_kit::Render for FocusedActionFixture {
        fn render(
            &mut self,
            window: &mut Window,
            cx: &mut gpui_kit::Context<Self>,
        ) -> impl IntoElement {
            let calls = self.calls.clone();
            let mut action = focused_action(
                "foundation-action",
                &self.focus,
                Theme::zed_dark(),
                self.disabled,
                window,
                cx,
                move |_, _, _| calls.set(calls.get() + 1),
            );
            self.outline
                .set(action.style().border_color == Some(Theme::zed_dark().focus_ring.into()));
            action
                .debug_selector(|| "foundation-action".into())
                .w(px(120.0))
                .h(px(CONTROL_HEIGHT_ACTION))
                .child("配置")
        }
    }

    #[gpui_kit::test]
    fn foundation_focused_action_uses_one_dispatch_and_rejects_disabled_input(
        cx: &mut TestAppContext,
    ) {
        use gpui_kit::{KeyDownEvent, KeyUpEvent, Keystroke, Modifiers};
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let fixture_calls = calls.clone();
        let fixture = cx.new(move |cx| FocusedActionFixture {
            focus: cx.focus_handle().tab_stop(true),
            calls: fixture_calls,
            disabled: false,
            outline: std::cell::Cell::new(false),
        });
        let view = fixture.clone();
        let (_, window) =
            cx.add_window_view(move |window, cx| gpui_kit::base::Root::new(view, window, cx));
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
                prefer_character_input: false,
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

    #[gpui_kit::test]
    fn destructive_action_is_neutral_at_rest_and_danger_only_on_interaction(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
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
                .hover(palette.hover.into())
                .active(palette.active.into())
        });

        assert!(
            actual == expected,
            "the destructive action tier must stay neutral at rest and use danger_soft only for hover/press"
        );

        let ghost = danger_ghost_button_palette(theme);
        assert_eq!(ghost.rest, gpui_kit::transparent_black().into());
        assert_eq!(ghost.foreground, theme.icon_muted);
        assert_eq!(ghost.border, gpui_kit::transparent_black().into());
        assert_eq!(ghost.hover, theme.danger_soft);
        assert_eq!(ghost.active, theme.danger_soft);
    }

    struct PrimaryContrastFixture {
        disabled: bool,
        colors: std::rc::Rc<std::cell::RefCell<Vec<gpui_kit::Hsla>>>,
        calls: std::rc::Rc<std::cell::Cell<usize>>,
    }

    impl gpui_kit::Render for PrimaryContrastFixture {
        fn render(&mut self, _: &mut Window, cx: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            let theme = Theme::zed_dark();
            let colors = self.colors.clone();
            let calls = self.calls.clone();
            div().text_color(Theme::zed_dark().text).child(
                primary_action_button(
                    "upgrade-apply",
                    "应用升级设置",
                    gpui_kit::canvas(
                        |_, _, _| (),
                        move |_, (), window, _| colors.borrow_mut().push(window.text_style().color),
                    )
                    .size(px(1.0)),
                    theme,
                    self.disabled,
                    cx,
                )
                .w(px(72.0))
                .h(px(32.0))
                .on_click(move |_, _, _| calls.set(calls.get() + 1)),
            )
        }
    }

    #[gpui_kit::test]
    fn primary_apply_text_keeps_readable_contrast_in_normal_hover_and_disabled_states(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let colors = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
        let recorded = colors.clone();
        let calls = std::rc::Rc::new(std::cell::Cell::new(0));
        let action_calls = calls.clone();
        let (view, window) = cx.add_window_view(move |_, _| PrimaryContrastFixture {
            disabled: false,
            colors: recorded,
            calls: action_calls,
        });
        window.refresh().expect("actual primary rendering");
        let normal = *colors.borrow().last().expect("painted foreground");
        assert_eq!(
            normal,
            window.read(|cx| gpui_kit::component::Theme::global(cx)
                .colors
                .button_primary_foreground),
            "normal Apply text must use the exact official primary foreground"
        );
        let bounds = window
            .debug_bounds("upgrade-apply")
            .expect("primary hit target");
        window.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
        assert_eq!(calls.get(), 1);
        window.simulate_mouse_move(bounds.center(), None, gpui_kit::Modifiers::default());
        window.refresh().expect("actual hover foreground");
        assert_eq!(
            *colors.borrow().last().expect("hover foreground"),
            normal,
            "hover must not replace the primary foreground with another color"
        );
        view.update(window, |view, cx| {
            view.disabled = true;
            cx.notify();
        });
        window.refresh().expect("disabled primary rendering");
        let disabled = *colors.borrow().last().expect("disabled foreground");
        assert!(
            disabled.a >= 0.9,
            "disabled label must remain readable rather than fading into its neutral background"
        );
        assert_eq!(
            disabled,
            gpui_kit::Hsla::from(Theme::zed_dark().text_secondary)
        );
        assert_eq!(window.debug_bounds("upgrade-apply"), Some(bounds));
        window.simulate_click(bounds.center(), gpui_kit::Modifiers::default());
        assert_eq!(calls.get(), 1, "disabled Apply must not dispatch");
    }

    #[gpui_kit::test]
    fn primary_button_variant_provides_solid_accent_anchor(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let theme = Theme::zed_dark();
        let actual = cx.update(|cx| primary_button_variant(theme, cx));
        let expected = cx.update(|cx| {
            let colors = gpui_kit::component::Theme::global(cx).colors;
            ButtonCustomVariant::new(cx)
                .color(colors.button_primary)
                .foreground(colors.button_primary_foreground)
                .hover(colors.button_primary_hover)
                .active(colors.button_primary_active)
        });
        assert!(
            actual == expected,
            "primary button must provide a solid accent visual anchor with high-contrast text"
        );
    }
}

#[cfg(test)]
#[path = "action_name_tests.rs"]
mod name_tests;
