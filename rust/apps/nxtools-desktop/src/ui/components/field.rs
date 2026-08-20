use gpui::{AnyElement, Entity, FontWeight, IntoElement, Styled, div, prelude::*, px, relative};
use gpui_component::input::{Input, InputState};

use crate::theme::Theme;
use crate::ui::metrics::{CONTROL_HEIGHT_INPUT, RADIUS_CONTROL, SPACING_4, SPACING_6, SPACING_8};
use crate::ui::typography::{LINE_HEIGHT_LABEL, TEXT_LABEL};

pub(crate) fn field_shell(theme: Theme, selector: &'static str, control: Input) -> AnyElement {
    let control = Styled::h(control, relative(1.0));
    div()
        .debug_selector(move || selector.into())
        .h(px(CONTROL_HEIGHT_INPUT))
        .w_full()
        .min_w_0()
        .rounded(px(RADIUS_CONTROL))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.canvas)
        .overflow_hidden()
        .flex()
        .items_center()
        .child(div().size_full().min_w_0().child(control))
        .into_any_element()
}

pub(crate) fn labeled_field(
    theme: Theme,
    label: &'static str,
    description: Option<&'static str>,
    control: &Entity<InputState>,
    selector: &'static str,
) -> AnyElement {
    div()
        .w_full()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(SPACING_4))
        .child(
            div()
                .debug_selector(move || format!("{selector}-label"))
                .h(px(LINE_HEIGHT_LABEL))
                .flex()
                .items_center()
                .gap(px(SPACING_6))
                .child(
                    div()
                        .flex_none()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.text_secondary)
                        .child(label),
                )
                .when_some(description, |row, description| {
                    row.child(
                        div()
                            .debug_selector(move || format!("{selector}-description"))
                            .min_w_0()
                            .flex_1()
                            .truncate()
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .text_size(px(TEXT_LABEL))
                            .line_height(px(LINE_HEIGHT_LABEL))
                            .text_color(theme.text_muted)
                            .child(description),
                    )
                }),
        )
        .child(field_shell(
            theme,
            selector,
            Input::new(control)
                .appearance(false)
                .bordered(false)
                .focus_bordered(false),
        ))
        .into_any_element()
}

#[allow(dead_code)] // Migrated path pickers consume this in later P0 slices.
pub(crate) fn path_picker_shell(
    selector: &'static str,
    field: AnyElement,
    picker: AnyElement,
) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .h(px(CONTROL_HEIGHT_INPUT))
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap(px(SPACING_8))
        .child(div().flex_1().min_w_0().child(field))
        .child(picker)
        .into_any_element()
}

#[allow(dead_code)] // Numeric settings migrate without changing their parsing owner.
pub(crate) fn numeric_field_shell(
    theme: Theme,
    selector: &'static str,
    control: Input,
) -> AnyElement {
    field_shell(theme, selector, control)
}

pub(crate) fn inline_error(
    theme: Theme,
    selector: &'static str,
    message: impl Into<gpui::SharedString>,
) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .w_full()
        .rounded(px(RADIUS_CONTROL))
        .border_l_2()
        .border_color(theme.danger)
        .bg(theme.danger_soft)
        .px(px(SPACING_8))
        .py(px(SPACING_6))
        .text_size(px(TEXT_LABEL))
        .line_height(px(LINE_HEIGHT_LABEL))
        .text_color(theme.danger)
        .child(message.into())
        .into_any_element()
}
