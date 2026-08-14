use gpui::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
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
