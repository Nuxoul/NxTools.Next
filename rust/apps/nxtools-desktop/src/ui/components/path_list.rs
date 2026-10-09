use std::rc::Rc;

use gpui::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Icon, Sizable};

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::components::action::{danger_ghost_button_variant, labelled_icon_action};
use crate::ui::metrics::{
    CONTROL_HEIGHT_ACTION, RADIUS_CONTROL, RADIUS_NONE, SPACING_4, SPACING_8, SPACING_12,
};
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL};

const PATH_LIST_ROW_HEIGHT: f32 = 40.0;
const PATH_LIST_ICON_RAIL_WIDTH: f32 = 16.0;
const PATH_LIST_DELETE_HIT_TARGET: f32 = 40.0;

type PathSelectCallback = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type PathRemoveCallback = Rc<dyn Fn(usize, String, &mut Window, &mut App)>;

#[derive(Clone)]
struct PathRowCallbacks {
    select: PathSelectCallback,
    remove: PathRemoveCallback,
}

pub(crate) fn exact_path_tooltip(path: &str) -> SharedString {
    path.to_owned().into()
}

pub(crate) fn path_exclusion_setting(
    theme: Theme,
    paths: &[String],
    selected_index: Option<usize>,
    on_select: impl Fn(usize, &mut Window, &mut App) + 'static,
    on_remove: impl Fn(usize, String, &mut Window, &mut App) + 'static,
    on_add: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let callbacks = PathRowCallbacks {
        select: Rc::new(on_select),
        remove: Rc::new(on_remove),
    };
    let add = labelled_icon_action(
        "quick-launch-add-index-exclusion",
        MaterialIcon::Add,
        "添加目录",
    )
    .ghost()
    .border_1()
    .border_color(theme.border_subtle)
    .bg(theme.surface_raised)
    .on_click(on_add);
    let mut rows = Vec::with_capacity(paths.len().max(1));
    if paths.is_empty() {
        rows.push(empty_path_row(theme));
    } else {
        for (index, path) in paths.iter().cloned().enumerate() {
            rows.push(path_row(
                theme,
                index,
                path,
                selected_index == Some(index),
                index + 1 < paths.len(),
                callbacks.clone(),
                cx,
            ));
        }
    }

    div()
        .debug_selector(|| "quick-launch-exclusion-setting".into())
        .w_full()
        .py(px(SPACING_12))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .flex_col()
        .gap(px(SPACING_8))
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap(px(SPACING_12))
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(SPACING_4))
                        .child(
                            div()
                                .h(px(LINE_HEIGHT_BODY))
                                .text_size(px(TEXT_BODY))
                                .line_height(px(LINE_HEIGHT_BODY))
                                .text_color(theme.text)
                                .child("排除目录"),
                        )
                        .child(
                            div()
                                .h(px(LINE_HEIGHT_LABEL))
                                .truncate()
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(LINE_HEIGHT_LABEL))
                                .text_color(theme.text_muted)
                                .child("目录及其所有子项不会进入速启结果"),
                        ),
                )
                .child(
                    div()
                        .debug_selector(|| "quick-launch-add-index-exclusion".into())
                        .h(px(CONTROL_HEIGHT_ACTION))
                        .flex_none()
                        .child(add),
                ),
        )
        .child(
            div()
                .id("quick-launch-exclusion-list")
                .debug_selector(|| "quick-launch-exclusion-list".into())
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

fn path_row(
    theme: Theme,
    index: usize,
    path: String,
    selected: bool,
    show_divider: bool,
    callbacks: PathRowCallbacks,
    cx: &App,
) -> AnyElement {
    let row_selector: SharedString = format!("quick-launch-exclusion-row-{index}").into();
    let path_selector: SharedString = format!("quick-launch-exclusion-path-{index}").into();
    let tooltip_selector: SharedString =
        format!("quick-launch-exclusion-path-tooltip-{index}").into();
    let delete_selector = format!("quick-launch-exclusion-delete-{index}");
    let delete_id: SharedString = format!("quick-launch-exclusion-delete-button-{index}").into();
    let tooltip_path = exact_path_tooltip(&path);
    let remove_path = path.clone();
    let row_select = Rc::clone(&callbacks.select);
    let remove = Rc::clone(&callbacks.remove);
    let delete_variant = danger_ghost_button_variant(theme, cx);

    div()
        .id(row_selector.clone())
        .debug_selector(move || row_selector.to_string())
        .relative()
        .w_full()
        .h(px(PATH_LIST_ROW_HEIGHT))
        .flex_none()
        .cursor_pointer()
        .flex()
        .items_center()
        .pl(px(SPACING_12))
        .bg(if selected {
            theme.element_selected
        } else {
            theme.canvas
        })
        .hover(move |style| style.bg(theme.surface_hover))
        .when(show_divider, |row| {
            row.border_b_1().border_color(theme.border_subtle)
        })
        .when(selected, |row| {
            row.child(
                div()
                    .absolute()
                    .left_0()
                    .top(px(SPACING_8))
                    .bottom(px(SPACING_8))
                    .w(px(2.0))
                    .bg(theme.accent),
            )
        })
        .on_click(move |_event, window, cx| row_select(index, window, cx))
        .child(
            div()
                .w(px(PATH_LIST_ICON_RAIL_WIDTH))
                .flex_none()
                .flex()
                .items_center()
                .child(
                    Icon::new(MaterialIcon::Folder)
                        .size(px(14.0))
                        .text_color(theme.icon_muted),
                ),
        )
        .child(
            div()
                .id(path_selector.clone())
                .debug_selector(move || path_selector.to_string())
                .min_w_0()
                .h(px(LINE_HEIGHT_BODY))
                .flex_1()
                .ml(px(SPACING_8))
                .truncate()
                .text_size(px(TEXT_BODY))
                .line_height(px(LINE_HEIGHT_BODY))
                .text_color(theme.text_secondary)
                .tooltip(move |window, cx| {
                    let tooltip_path = tooltip_path.clone();
                    let tooltip_selector = tooltip_selector.clone();
                    Tooltip::element(move |_, _| {
                        let tooltip_selector = tooltip_selector.clone();
                        div()
                            .debug_selector(move || tooltip_selector.to_string())
                            .child(tooltip_path.clone())
                    })
                    .build(window, cx)
                })
                .child(path),
        )
        .child(
            div()
                .debug_selector(move || delete_selector.clone())
                .size(px(PATH_LIST_DELETE_HIT_TARGET))
                .flex_none()
                .child(
                    Button::new(delete_id)
                        .icon(Icon::new(MaterialIcon::Delete).size(px(14.0)))
                        .small()
                        .size(px(PATH_LIST_DELETE_HIT_TARGET))
                        .rounded(px(RADIUS_NONE))
                        .custom(delete_variant)
                        .tooltip("移除排除目录")
                        .on_click(move |_event, window, cx| {
                            cx.stop_propagation();
                            remove(index, remove_path.clone(), window, cx);
                        }),
                ),
        )
        .into_any_element()
}

fn empty_path_row(theme: Theme) -> AnyElement {
    div()
        .debug_selector(|| "quick-launch-exclusion-empty".into())
        .w_full()
        .h(px(PATH_LIST_ROW_HEIGHT))
        .px(px(SPACING_12))
        .flex()
        .items_center()
        .text_size(px(TEXT_LABEL))
        .line_height(px(LINE_HEIGHT_LABEL))
        .text_color(theme.text_muted)
        .child("尚未添加排除目录")
        .into_any_element()
}
