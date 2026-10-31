use std::rc::Rc;

use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::components::action::{danger_ghost_button_variant, labelled_icon_action};
use crate::ui::metrics::{RADIUS_CONTROL, ordinary_rem};
use crate::ui::typography::{
    LINE_HEIGHT_WORKBENCH_BODY as LINE_HEIGHT_BODY,
    LINE_HEIGHT_WORKBENCH_SUPPORT as LINE_HEIGHT_LABEL, TEXT_WORKBENCH_BODY as TEXT_BODY,
    TEXT_WORKBENCH_SUPPORT as TEXT_LABEL,
};

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
    on_add: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
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
    .outline()
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
        .py_3()
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .w_full()
                .flex()
                .items_center()
                .gap_3()
                .child(
                    div()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(
                            div()
                                .h(ordinary_rem(LINE_HEIGHT_BODY))
                                .text_size(ordinary_rem(TEXT_BODY))
                                .line_height(ordinary_rem(LINE_HEIGHT_BODY))
                                .text_color(theme.text)
                                .child("排除目录"),
                        )
                        .child(crate::ui::components::tooltip::description_help(
                            "quick-launch-exclusion-help",
                            "目录及其所有子项不会进入速启结果",
                        )),
                )
                .child(
                    div()
                        .debug_selector(|| "quick-launch-add-index-exclusion".into())
                        .h_8()
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
    let remove_name = format!("移除排除目录：{path}");

    ListItem::new(row_selector.clone())
        .debug_selector(move || row_selector.to_string())
        .accessibility_label(path.clone())
        .selected(selected)
        .aria_selected(selected)
        .w_full()
        .min_h(px(PATH_LIST_ROW_HEIGHT))
        .py_0()
        .flex_none()
        .cursor_pointer()
        .when(show_divider, |row| {
            row.child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(px(1.0))
                    .bg(theme.border_subtle),
            )
        })
        .on_click(move |_event, window, cx| row_select(index, window, cx))
        .child(
            div()
                .w_full()
                .min_w_0()
                .flex()
                .items_center()
                .child(
                    div()
                        .w(ordinary_rem(PATH_LIST_ICON_RAIL_WIDTH))
                        .flex_none()
                        .flex()
                        .items_center()
                        .child(
                            Icon::new(MaterialIcon::Folder)
                                .small()
                                .text_color(theme.icon_muted),
                        ),
                )
                .child(
                    div()
                        .id(path_selector.clone())
                        .debug_selector(move || path_selector.to_string())
                        .min_w_0()
                        .h(ordinary_rem(LINE_HEIGHT_BODY))
                        .flex_1()
                        .ml_2()
                        .truncate()
                        .text_size(ordinary_rem(TEXT_BODY))
                        .line_height(ordinary_rem(LINE_HEIGHT_BODY))
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
                                .accessibility_label(remove_name)
                                .icon(Icon::new(MaterialIcon::Delete).small())
                                // Named 40px delete hit contract; Kit keeps radius/content/state grammar.
                                .with_size(px(PATH_LIST_DELETE_HIT_TARGET))
                                .custom(delete_variant)
                                .tooltip("移除排除目录")
                                .on_click(move |_event, window, cx| {
                                    cx.stop_propagation();
                                    remove(index, remove_path.clone(), window, cx);
                                }),
                        ),
                ),
        )
        .into_any_element()
}

fn empty_path_row(theme: Theme) -> AnyElement {
    div()
        .debug_selector(|| "quick-launch-exclusion-empty".into())
        .w_full()
        .min_h(px(PATH_LIST_ROW_HEIGHT))
        .px_3()
        .flex()
        .items_center()
        .text_size(ordinary_rem(TEXT_LABEL))
        .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
        .text_color(theme.text_muted)
        .child("尚未添加排除目录")
        .into_any_element()
}

#[cfg(test)]
mod queue01_tests {
    use super::*;
    use std::cell::Cell;
    struct PathFixture {
        selects: Rc<Cell<usize>>,
        removes: Rc<Cell<usize>>,
    }
    impl gpui_kit::Render for PathFixture {
        fn render(&mut self, _: &mut Window, cx: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            let selects = self.selects.clone();
            let removes = self.removes.clone();
            div().w(px(320.0)).child(path_exclusion_setting(
                Theme::zed_dark(),
                &[
                    r"C:\synthetic\合成长路径".into(),
                    r"C:\synthetic\第二路径".into(),
                ],
                Some(0),
                move |_, _, _| selects.set(selects.get() + 1),
                move |_, path, _, _| {
                    assert_eq!(path, r"C:\synthetic\合成长路径");
                    removes.set(removes.get() + 1);
                },
                |_, _, _| {},
                cx,
            ))
        }
    }
    #[gpui_kit::test]
    fn queue01_path_list_uses_kit_row_and_one_delete_dispatch(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let selects = Rc::new(Cell::new(0));
        let removes = Rc::new(Cell::new(0));
        let selected = selects.clone();
        let removed = removes.clone();
        let (_, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| PathFixture {
                selects: selected,
                removes: removed,
            });
            gpui_kit::base::Root::new(view, window, cx)
        });
        window.refresh().expect("actual path row");
        let snapshots = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        assert!(
            snapshots.iter().any(|snapshot| snapshot.path().last()
                == Some(&gpui_kit::ElementId::from("quick-launch-exclusion-row-0"))),
            "actual stateless ListItem"
        );
        let row = window
            .debug_bounds("quick-launch-exclusion-row-0")
            .expect("fixed row");
        assert_eq!(row.size.height, px(40.0));
        let delete = window
            .debug_bounds("quick-launch-exclusion-delete-0")
            .expect("trailing action");
        let button = snapshots
            .iter()
            .find(|snapshot| {
                snapshot.path().last()
                    == Some(&gpui_kit::ElementId::from(
                        "quick-launch-exclusion-delete-button-0",
                    ))
            })
            .expect("styled delete action with preserved hit budget")
            .bounds();
        assert_eq!(button.size, gpui_kit::size(px(40.0), px(40.0)));
        assert!(button.top() >= row.top() && button.bottom() <= row.bottom());
        assert!(delete.left() >= row.left() && delete.right() <= row.right());
        window.simulate_click(delete.center(), gpui_kit::Modifiers::default());
        window.run_until_parked();
        assert_eq!(removes.get(), 1);
        assert_eq!(selects.get(), 0, "delete must not also select the row");
    }

    #[gpui_kit::test]
    fn accessible_name_excluded_path_delete_buttons_identify_each_object(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let removes = Rc::new(Cell::new(0));
        let held = removes.clone();
        let (_, surface) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| PathFixture {
                selects: Rc::new(Cell::new(0)),
                removes: held,
            });
            gpui_kit::base::Root::new(view, window, cx)
        });
        surface.refresh().expect("actual excluded path rows");
        let nodes = surface.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        for (index, path) in [r"C:\synthetic\合成长路径", r"C:\synthetic\第二路径"]
            .into_iter()
            .enumerate()
        {
            let id = gpui_kit::ElementId::from(gpui_kit::SharedString::from(format!(
                "quick-launch-exclusion-delete-button-{index}"
            )));
            let button = nodes
                .iter()
                .find(|node| node.path().last() == Some(&id))
                .expect("actual trailing delete Button");
            assert_eq!(button.role(), Some(gpui_kit::Role::Button));
            assert_eq!(
                button.label(),
                Some(format!("移除排除目录：{path}").as_str())
            );
            assert_eq!(button.bounds().size, gpui_kit::size(px(40.0), px(40.0)));
        }
        assert_eq!(removes.get(), 0, "semantic observation cannot remove paths");
    }
}
