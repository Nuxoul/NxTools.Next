use gpui::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::tooltip::Tooltip;

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
    selector: SharedString,
    title: impl Into<gpui::SharedString>,
    detail: impl Into<gpui::SharedString>,
    selected: bool,
    enabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let title = title.into();
    let detail = detail.into();
    let marker_selector = SharedString::from(format!("{selector}-marker"));
    let title_selector = SharedString::from(format!("{selector}-title"));
    let detail_selector = SharedString::from(format!("{selector}-detail"));
    let tooltip_selector = SharedString::from(format!("{selector}-tooltip"));
    let tooltip_detail = detail.clone();
    div()
        .id(selector.clone())
        .debug_selector(move || selector.to_string())
        .relative()
        .w_full()
        .h(px(SETTINGS_ROW_MIN_HEIGHT))
        .flex_none()
        .px(px(SPACING_8))
        .cursor_pointer()
        .flex()
        .flex_col()
        .justify_center()
        .bg(if selected {
            theme.element_selected
        } else {
            theme.canvas
        })
        .hover(move |style| style.bg(theme.surface_hover))
        .tooltip(move |window, cx| {
            let tooltip_detail = tooltip_detail.clone();
            let tooltip_selector = tooltip_selector.clone();
            Tooltip::element(move |_, _| {
                let tooltip_selector = tooltip_selector.clone();
                div()
                    .debug_selector(move || tooltip_selector.to_string())
                    .child(tooltip_detail.clone())
            })
            .build(window, cx)
        })
        .on_click(on_click)
        .when(selected, |row| {
            row.child(
                div()
                    .debug_selector(move || marker_selector.to_string())
                    .absolute()
                    .left_0()
                    .top(px(SPACING_6))
                    .bottom(px(SPACING_6))
                    .w(px(2.0))
                    .bg(theme.accent),
            )
        })
        .child(
            div()
                .debug_selector(move || title_selector.to_string())
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(px(TEXT_BODY))
                .line_height(px(LINE_HEIGHT_BODY))
                .text_color(if enabled {
                    theme.text
                } else {
                    theme.text_muted
                })
                .child(title),
        )
        .child(
            div()
                .debug_selector(move || detail_selector.to_string())
                .overflow_hidden()
                .whitespace_nowrap()
                .text_size(px(TEXT_LABEL))
                .line_height(px(LINE_HEIGHT_LABEL))
                .text_color(theme.text_muted)
                .child(detail),
        )
        .when(!enabled, |row| row.opacity(0.62))
        .into_any_element()
}

pub(crate) fn radio_list_row(
    theme: Theme,
    selector: SharedString,
    title: impl Into<gpui::SharedString>,
    detail: impl Into<gpui::SharedString>,
    selected: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let title = title.into();
    let detail = detail.into();
    let radio_selector = SharedString::from(format!("{selector}-radio"));
    let radio_state_selector = SharedString::from(format!(
        "{selector}-radio-{}",
        if selected { "selected" } else { "unselected" }
    ));
    let title_selector = SharedString::from(format!("{selector}-title"));
    let detail_selector = SharedString::from(format!("{selector}-detail"));

    div()
        .id(selector.clone())
        .debug_selector(move || selector.to_string())
        .w_full()
        .h(px(SETTINGS_ROW_MIN_HEIGHT))
        .flex_none()
        .px(px(SPACING_8))
        .border_b_1()
        .border_color(theme.border_subtle)
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(SPACING_8))
        .bg(if selected {
            theme.element_selected
        } else {
            theme.canvas
        })
        .hover(move |style| style.bg(theme.surface_hover))
        .on_click(on_click)
        .child(
            div()
                .debug_selector(move || radio_selector.to_string())
                .size(px(14.0))
                .flex_none()
                .rounded_full()
                .border_1()
                .border_color(if selected { theme.accent } else { theme.border })
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .debug_selector(move || radio_state_selector.to_string())
                        .size(px(SPACING_6))
                        .rounded_full()
                        .when(selected, |dot| dot.bg(theme.accent)),
                ),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .justify_center()
                .child(
                    div()
                        .debug_selector(move || title_selector.to_string())
                        .h(px(LINE_HEIGHT_BODY))
                        .flex()
                        .items_center()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .text_color(theme.text)
                        .child(title),
                )
                .child(
                    div()
                        .debug_selector(move || detail_selector.to_string())
                        .h(px(LINE_HEIGHT_LABEL))
                        .flex()
                        .items_center()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(detail),
                ),
        )
        .into_any_element()
}
