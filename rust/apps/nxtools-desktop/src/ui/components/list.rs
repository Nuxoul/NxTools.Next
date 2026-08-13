use gpui::{AnyElement, IntoElement, div, prelude::*, px};

use crate::theme::Theme;
use crate::ui::metrics::{CONTROL_HEIGHT_DEFAULT, SETTINGS_ROW_MIN_HEIGHT, SPACING_6, SPACING_8};
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL};

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub(crate) enum StatusTone {
    Neutral,
    Accent,
    Success,
    Warning,
    Danger,
}

#[allow(dead_code)]
pub(crate) fn status_badge(
    theme: Theme,
    label: impl Into<gpui::SharedString>,
    tone: StatusTone,
) -> AnyElement {
    let color = match tone {
        StatusTone::Neutral => theme.text_muted,
        StatusTone::Accent => theme.accent,
        StatusTone::Success => theme.success,
        StatusTone::Warning => theme.warning,
        StatusTone::Danger => theme.danger,
    };
    div()
        .h(px(CONTROL_HEIGHT_DEFAULT))
        .px(px(SPACING_8))
        .rounded(px(CONTROL_HEIGHT_DEFAULT / 2.0))
        .border_1()
        .border_color(theme.border_subtle)
        .text_size(px(TEXT_LABEL))
        .text_color(color)
        .flex()
        .items_center()
        .child(label.into())
        .into_any_element()
}

#[allow(dead_code)]
pub(crate) fn compact_list_row(
    theme: Theme,
    title: impl Into<gpui::SharedString>,
    detail: impl Into<gpui::SharedString>,
    selected: bool,
) -> AnyElement {
    div()
        .w_full()
        .min_h(px(SETTINGS_ROW_MIN_HEIGHT))
        .px(px(SPACING_8))
        .py(px(SPACING_6))
        .flex()
        .flex_col()
        .justify_center()
        .bg(if selected {
            theme.element_selected
        } else {
            theme.canvas
        })
        .hover(move |style| style.bg(theme.surface_hover))
        .child(
            div()
                .text_size(px(TEXT_BODY))
                .line_height(px(LINE_HEIGHT_BODY))
                .child(title.into()),
        )
        .child(
            div()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_muted)
                .child(detail.into()),
        )
        .into_any_element()
}

#[allow(dead_code)]
pub(crate) fn empty_state(
    theme: Theme,
    title: impl Into<gpui::SharedString>,
    description: impl Into<gpui::SharedString>,
) -> AnyElement {
    div()
        .w_full()
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(px(SPACING_6))
        .text_color(theme.text_muted)
        .child(
            div()
                .text_size(px(TEXT_BODY))
                .text_color(theme.text_secondary)
                .child(title.into()),
        )
        .child(div().text_size(px(TEXT_LABEL)).child(description.into()))
        .into_any_element()
}
