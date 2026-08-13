use gpui::{AnyElement, App, IntoElement, Window, div, prelude::*, px};
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::{Icon, Selectable, Sizable};

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::metrics::{CONTROL_HEIGHT_ACTION, RADIUS_CONTROL, SPACING_8};

pub(crate) type ClickHandler = Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static>;

pub(crate) struct SegmentedOption {
    pub id: &'static str,
    pub label: &'static str,
    pub selected: bool,
    pub on_click: ClickHandler,
}

impl SegmentedOption {
    pub(crate) fn new(
        id: &'static str,
        label: &'static str,
        selected: bool,
        on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        Self {
            id,
            label,
            selected,
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

pub(crate) fn action_button(id: &'static str, label: &'static str) -> Button {
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
    Button::new(id)
        .icon(Icon::new(icon).size(px(14.0)))
        .tooltip(tooltip_label)
        .small()
        .size(px(CONTROL_HEIGHT_ACTION))
        .rounded(px(RADIUS_CONTROL))
}

pub(crate) fn segmented_control(
    theme: Theme,
    selector: &'static str,
    options: Vec<SegmentedOption>,
    cx: &App,
) -> AnyElement {
    let selected_variant = primary_button_variant(theme, cx);
    div()
        .debug_selector(move || selector.into())
        .h(px(CONTROL_HEIGHT_ACTION))
        .rounded(px(RADIUS_CONTROL))
        .overflow_hidden()
        .flex()
        .children(options.into_iter().map(move |option| {
            let button = Button::new(option.id)
                .label(option.label)
                .small()
                .h_full()
                .w_full()
                .rounded(px(0.0))
                .selected(option.selected)
                .when(option.selected, |button| button.custom(selected_variant))
                .when(!option.selected, |button| button.ghost())
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
