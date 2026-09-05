use std::rc::Rc;

use gpui::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::checkbox::Checkbox;
use gpui_component::{Sizable, Size};

use crate::theme::Theme;
use crate::ui::metrics::{RADIUS_CONTROL, SPACING_2, SPACING_8, SPACING_12};
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL};

const VOLUME_SCOPE_AUTOMATIC_ROW_HEIGHT: f32 = 48.0;
const VOLUME_SCOPE_ROW_HEIGHT: f32 = 40.0;

pub(crate) fn volume_scope_setting(
    theme: Theme,
    description: impl Into<SharedString>,
    rows: Vec<AnyElement>,
) -> AnyElement {
    div()
        .debug_selector(|| "quick-launch-volume-scope-setting".into())
        .w_full()
        .py(px(SPACING_12))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .flex_col()
        .gap(px(SPACING_8))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(SPACING_2))
                .child(
                    div()
                        .h(px(LINE_HEIGHT_BODY))
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .text_color(theme.text)
                        .child("索引卷"),
                )
                .child(
                    div()
                        .h(px(LINE_HEIGHT_LABEL))
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .child(description.into()),
                ),
        )
        .child(
            div()
                .id("quick-launch-volume-scope")
                .debug_selector(|| "quick-launch-volume-scope".into())
                .w_full()
                .overflow_hidden()
                .rounded(px(RADIUS_CONTROL))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.canvas)
                .children(rows),
        )
        .into_any_element()
}

pub(crate) fn automatic_volume_scope_row(
    theme: Theme,
    checked: bool,
    on_click: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let on_click = Rc::new(on_click);
    let checkbox_on_click = Rc::clone(&on_click);
    let state_selector = format!(
        "quick-launch-volume-scope-automatic-{}",
        if checked { "checked" } else { "unchecked" }
    );
    div()
        .id("quick-launch-volume-scope-automatic")
        .debug_selector(|| "quick-launch-volume-scope-automatic".into())
        .w_full()
        .h(px(VOLUME_SCOPE_AUTOMATIC_ROW_HEIGHT))
        .cursor_pointer()
        .hover(move |style| style.bg(theme.surface_hover))
        .border_b_1()
        .border_color(theme.border_subtle)
        .on_click(move |_event, window, cx| {
            on_click(&!checked, window, cx);
        })
        .child(
            div()
                .debug_selector(move || state_selector.clone())
                .size_full()
                .child(
                    Checkbox::new("quick-launch-volume-scope-automatic-checkbox")
                        .checked(checked)
                        .with_size(Size::Small)
                        .w_full()
                        .h_full()
                        .px(px(SPACING_12))
                        .items_center()
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(SPACING_2))
                                .child(
                                    div()
                                        .h(px(LINE_HEIGHT_BODY))
                                        .text_size(px(TEXT_BODY))
                                        .line_height(px(LINE_HEIGHT_BODY))
                                        .text_color(theme.text)
                                        .child("自动包含全部 NTFS 卷"),
                                )
                                .child(
                                    div()
                                        .h(px(LINE_HEIGHT_LABEL))
                                        .text_size(px(TEXT_LABEL))
                                        .line_height(px(LINE_HEIGHT_LABEL))
                                        .text_color(theme.text_muted)
                                        .child("新发现的卷会自动加入索引范围"),
                                ),
                        )
                        .on_click(move |checked, window, cx| {
                            cx.stop_propagation();
                            checkbox_on_click(checked, window, cx);
                        }),
                ),
        )
        .into_any_element()
}

pub(crate) fn volume_scope_row(
    theme: Theme,
    selector: impl Into<SharedString>,
    title: impl Into<SharedString>,
    metadata: impl Into<SharedString>,
    checked: bool,
    show_divider: bool,
    on_click: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let on_click = Rc::new(on_click);
    let checkbox_on_click = Rc::clone(&on_click);
    let selector = selector.into();
    let title_selector = format!("{selector}-title");
    let metadata_selector = format!("{selector}-metadata");
    let checkbox_id: SharedString = format!("{selector}-checkbox").into();
    let row_selector = selector.to_string();
    let state_selector = format!(
        "{selector}-{}",
        if checked { "checked" } else { "unchecked" }
    );
    div()
        .id(selector.clone())
        .debug_selector(move || row_selector.clone())
        .w_full()
        .h(px(VOLUME_SCOPE_ROW_HEIGHT))
        .cursor_pointer()
        .hover(move |style| style.bg(theme.surface_hover))
        .when(show_divider, |row| {
            row.border_b_1().border_color(theme.border_subtle)
        })
        .on_click(move |_event, window, cx| {
            on_click(&!checked, window, cx);
        })
        .child(
            div()
                .debug_selector(move || state_selector.clone())
                .size_full()
                .child(
                    Checkbox::new(checkbox_id)
                        .checked(checked)
                        .with_size(Size::Small)
                        .w_full()
                        .h_full()
                        .px(px(SPACING_12))
                        .items_center()
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .items_center()
                                .gap(px(SPACING_12))
                                .child(
                                    div()
                                        .debug_selector(move || title_selector.clone())
                                        .h(px(LINE_HEIGHT_BODY))
                                        .min_w_0()
                                        .flex_1()
                                        .truncate()
                                        .text_size(px(TEXT_BODY))
                                        .line_height(px(LINE_HEIGHT_BODY))
                                        .text_color(theme.text)
                                        .child(title.into()),
                                )
                                .child(
                                    div()
                                        .debug_selector(move || metadata_selector.clone())
                                        .h(px(LINE_HEIGHT_LABEL))
                                        .flex_none()
                                        .text_size(px(TEXT_LABEL))
                                        .line_height(px(LINE_HEIGHT_LABEL))
                                        .text_color(theme.text_muted)
                                        .child(metadata.into()),
                                ),
                        )
                        .on_click(move |checked, window, cx| {
                            cx.stop_propagation();
                            checkbox_on_click(checked, window, cx);
                        }),
                ),
        )
        .into_any_element()
}

pub(crate) fn empty_volume_scope_row(theme: Theme, message: &'static str) -> AnyElement {
    div()
        .debug_selector(|| "quick-launch-volume-scope-empty".into())
        .w_full()
        .h(px(VOLUME_SCOPE_ROW_HEIGHT))
        .px(px(SPACING_12))
        .flex()
        .items_center()
        .text_size(px(TEXT_LABEL))
        .line_height(px(LINE_HEIGHT_LABEL))
        .text_color(theme.text_muted)
        .child(message)
        .into_any_element()
}
