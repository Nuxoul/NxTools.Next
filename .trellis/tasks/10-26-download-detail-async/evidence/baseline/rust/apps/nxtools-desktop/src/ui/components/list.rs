#![cfg(test)]
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};

use crate::theme::Theme;
use crate::ui::components::action::focusable_content;
use crate::ui::metrics::{SETTINGS_ROW_MIN_HEIGHT, SPACING_8};
use crate::ui::typography::{LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL};

pub(crate) fn compact_list_row(
    theme: Theme,
    selector: SharedString,
    title: impl Into<gpui_kit::SharedString>,
    detail: impl Into<gpui_kit::SharedString>,
    selected: bool,
    enabled: bool,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let title = title.into();
    let detail = detail.into();
    let marker_selector = SharedString::from(format!("{selector}-marker"));
    let title_selector = SharedString::from(format!("{selector}-title"));
    let detail_selector = SharedString::from(format!("{selector}-detail"));
    let tooltip_selector = SharedString::from(format!("{selector}-tooltip"));
    let tooltip_detail = detail.clone();
    let action_id = SharedString::from(format!("{selector}-action"));
    let content = div()
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
        .when(selected, |row| {
            row.child(
                div()
                    .debug_selector(move || marker_selector.to_string())
                    .absolute()
                    .left_0()
                    .top(px(SPACING_8))
                    .bottom(px(SPACING_8))
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
        .into_any_element();
    focusable_content(action_id, theme, false, content, on_click)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::{KeyDownEvent, KeyUpEvent, Keystroke, TestAppContext};
    use std::{cell::Cell, rc::Rc};

    struct SelectionRowsFixture {
        radio_calls: Rc<Cell<usize>>,
        category_calls: Rc<Cell<usize>>,
    }

    impl gpui_kit::Render for SelectionRowsFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            let radio_calls = self.radio_calls.clone();
            let category_calls = self.category_calls.clone();
            div()
                .w(px(320.0))
                .flex()
                .flex_col()
                .child(
                    gpui_kit::component::radio::Radio::new("keyboard-radio")
                        .debug_selector(|| "keyboard-radio".into())
                        .h(px(SETTINGS_ROW_MIN_HEIGHT))
                        .label("视频轨道")
                        .on_change(move |_, _, _| radio_calls.set(radio_calls.get() + 1)),
                )
                .child(compact_list_row(
                    Theme::zed_dark(),
                    "keyboard-category".into(),
                    "已禁用分类",
                    "仍可选中编辑",
                    false,
                    false,
                    move |_, _, _| category_calls.set(category_calls.get() + 1),
                ))
        }
    }

    #[gpui_kit::test]
    fn selection_rows_support_keyboard_without_changing_content_budget(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let radio_calls = Rc::new(Cell::new(0));
        let category_calls = Rc::new(Cell::new(0));
        let fixture_radio = radio_calls.clone();
        let fixture_category = category_calls.clone();
        let (_, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| SelectionRowsFixture {
                radio_calls: fixture_radio,
                category_calls: fixture_category,
            });
            gpui_kit::base::Root::new(view, window, cx)
        });
        window.refresh().expect("render production selection rows");
        let radio = window
            .debug_bounds("keyboard-radio")
            .expect("actual radio row");
        let category = window
            .debug_bounds("keyboard-category")
            .expect("actual category row");
        assert_eq!(radio.size.height, px(SETTINGS_ROW_MIN_HEIGHT));
        assert_eq!(category.size.height, px(SETTINGS_ROW_MIN_HEIGHT));
        window.update(|window, cx| window.focus_next(cx));
        window.refresh().expect("focus radio row");
        for key in ["enter", "space"] {
            let before = radio_calls.get();
            window.simulate_event(KeyDownEvent {
                keystroke: Keystroke::parse(key).expect("selection key"),
                is_held: false,
                prefer_character_input: false,
            });
            assert_eq!(radio_calls.get(), before, "activation waits for release");
            window.simulate_event(KeyUpEvent {
                keystroke: Keystroke::parse(key).expect("selection release"),
            });
            assert_eq!(
                radio_calls.get(),
                before + 1,
                "one selection per keyboard gesture"
            );
        }
        window.simulate_keystrokes("tab");
        window.refresh().expect("focus category row");
        window.simulate_event(KeyDownEvent {
            keystroke: Keystroke::parse("space").expect("category key"),
            is_held: false,
            prefer_character_input: false,
        });
        window.simulate_event(KeyUpEvent {
            keystroke: Keystroke::parse("space").expect("category release"),
        });
        assert_eq!(
            category_calls.get(),
            1,
            "disabled category remains selectable for editing"
        );
        assert_eq!(radio_calls.get(), 2, "Tab must move to the next action");
        window
            .refresh()
            .expect("paint selected focus without layout shift");
        assert_eq!(window.debug_bounds("keyboard-radio"), Some(radio));
        assert_eq!(window.debug_bounds("keyboard-category"), Some(category));
    }
}
