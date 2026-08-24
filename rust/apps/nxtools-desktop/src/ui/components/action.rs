use gpui::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Icon, Selectable, Sizable};

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::metrics::{CONTROL_HEIGHT_ACTION, RADIUS_CONTROL, SPACING_8};

pub(crate) type ClickHandler = Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static>;

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
        .foreground(theme.accent_text.into())
        .border(theme.focus_ring.into())
        .hover(theme.border_active.into())
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
    Button::new(id)
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
                .rounded(px(0.0))
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
}
