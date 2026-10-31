use gpui_kit::component::Sizable;
use gpui_kit::component::input::Input;
use gpui_kit::{AnyElement, IntoElement, div, prelude::*};

use crate::theme::Theme;
use gpui_kit::component::alert::Alert;

pub(crate) fn field_shell(_theme: Theme, selector: &'static str, control: Input) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .w_full()
        .min_w_0()
        .child(
            control
                .with_size(gpui_kit::component::Size::Medium)
                .appearance(true)
                .bordered(true)
                .focus_bordered(true),
        )
        .into_any_element()
}

pub(crate) fn path_picker_shell(
    selector: &'static str,
    field: AnyElement,
    picker: AnyElement,
) -> AnyElement {
    div()
        .debug_selector(move || selector.into())
        .h_8()
        .w_full()
        .min_w_0()
        .flex()
        .items_center()
        .gap_2()
        .child(div().flex_1().min_w_0().child(field))
        .child(picker)
        .into_any_element()
}

pub(crate) fn inline_error(
    _theme: Theme,
    selector: &'static str,
    message: impl Into<gpui_kit::SharedString>,
) -> AnyElement {
    let alert_selector = format!("{selector}-kit-alert");
    div()
        .debug_selector(move || selector.into())
        .w_full()
        .min_w_0()
        .child(
            div()
                .debug_selector(move || alert_selector.clone())
                .w_full()
                .child(Alert::error(selector, message.into()).xsmall()),
        )
        .into_any_element()
}
