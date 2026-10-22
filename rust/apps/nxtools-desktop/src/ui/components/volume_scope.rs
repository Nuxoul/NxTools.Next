use std::rc::Rc;

use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Sizable, Size};
use gpui_kit::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};

use crate::theme::Theme;
use crate::ui::components::action::focusable_content;
use crate::ui::metrics::{RADIUS_CONTROL, SPACING_4, SPACING_8, SPACING_12};
use crate::ui::typography::{
    LINE_HEIGHT_WORKBENCH_BODY as LINE_HEIGHT_BODY,
    LINE_HEIGHT_WORKBENCH_SUPPORT as LINE_HEIGHT_LABEL, TEXT_WORKBENCH_BODY as TEXT_BODY,
    TEXT_WORKBENCH_SUPPORT as TEXT_LABEL,
};

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
                .items_center()
                .gap(px(SPACING_4))
                .child(
                    div()
                        .h(px(LINE_HEIGHT_BODY))
                        .text_size(px(TEXT_BODY))
                        .line_height(px(LINE_HEIGHT_BODY))
                        .text_color(theme.text)
                        .child("索引卷"),
                )
                .child(super::tooltip::description_help(
                    "quick-launch-volume-description",
                    description,
                )),
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
    let state_selector = format!(
        "quick-launch-volume-scope-automatic-{}",
        if checked { "checked" } else { "unchecked" }
    );
    let content = ListItem::new("quick-launch-volume-scope-automatic")
        .debug_selector(|| "quick-launch-volume-scope-automatic".into())
        .accessibility_label("自动包含全部 NTFS 卷")
        .tooltip(|window, cx| Tooltip::new("新发现的卷会自动加入索引范围").build(window, cx))
        .w_full()
        .h(px(VOLUME_SCOPE_AUTOMATIC_ROW_HEIGHT))
        .py_0()
        .cursor_pointer()
        .border_b_1()
        .border_color(theme.border_subtle)
        .child(
            div()
                .debug_selector(move || state_selector.clone())
                .size_full()
                .w_full()
                .flex()
                .items_center()
                .gap(px(SPACING_8))
                .child(
                    Checkbox::new("quick-launch-volume-scope-automatic-checkbox")
                        .checked(checked)
                        .tab_stop(false)
                        .with_size(Size::Small),
                )
                .child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap(px(SPACING_4))
                        .child(
                            div()
                                .h(px(LINE_HEIGHT_BODY))
                                .text_size(px(TEXT_BODY))
                                .truncate()
                                .line_height(px(LINE_HEIGHT_BODY))
                                .text_color(theme.text)
                                .child("自动包含全部 NTFS 卷"),
                        ),
                ),
        )
        .into_any_element();
    focusable_content(
        "quick-launch-volume-scope-automatic-action",
        theme,
        false,
        content,
        move |_, window, cx| on_click(&!checked, window, cx),
    )
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
    let selector = selector.into();
    let title: SharedString = title.into();
    let title_selector = format!("{selector}-title");
    let metadata_selector = format!("{selector}-metadata");
    let checkbox_id: SharedString = format!("{selector}-checkbox").into();
    let row_selector = selector.to_string();
    let metadata = metadata.into();
    let metadata_tooltip = metadata.clone();
    let state_selector = format!(
        "{selector}-{}",
        if checked { "checked" } else { "unchecked" }
    );
    let action_id = SharedString::from(format!("{selector}-action"));
    let content = ListItem::new(selector.clone())
        .debug_selector(move || row_selector.clone())
        .accessibility_label(title.clone())
        .w_full()
        .h(px(VOLUME_SCOPE_ROW_HEIGHT))
        .py_0()
        .cursor_pointer()
        .when(show_divider, |row| {
            row.border_b_1().border_color(theme.border_subtle)
        })
        .child(
            div()
                .debug_selector(move || state_selector.clone())
                .w_full()
                .flex()
                .items_center()
                .gap(px(SPACING_8))
                .child(
                    Checkbox::new(checkbox_id.clone())
                        .checked(checked)
                        .tab_stop(false)
                        .with_size(Size::Small),
                )
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
                        .child(title),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("{selector}-status")))
                        .debug_selector(move || metadata_selector.clone())
                        .h(px(LINE_HEIGHT_LABEL))
                        .w(px(196.0))
                        .min_w_0()
                        .flex_none()
                        .truncate()
                        .text_size(px(TEXT_LABEL))
                        .line_height(px(LINE_HEIGHT_LABEL))
                        .text_color(theme.text_muted)
                        .tooltip(move |window, cx| {
                            Tooltip::new(metadata_tooltip.clone()).build(window, cx)
                        })
                        .child(metadata),
                ),
        )
        .into_any_element();
    focusable_content(action_id, theme, false, content, move |_, window, cx| {
        on_click(&!checked, window, cx)
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    pub(super) struct VolumeFixture {
        pub(super) automatic: Rc<Cell<bool>>,
        pub(super) volume: Rc<Cell<bool>>,
    }
    impl gpui_kit::Render for VolumeFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui_kit::Context<Self>) -> impl IntoElement {
            let automatic = self.automatic.clone();
            let volume = self.volume.clone();
            div()
                .w(px(320.0))
                .flex()
                .flex_col()
                .child(automatic_volume_scope_row(
                    Theme::zed_dark(),
                    automatic.get(),
                    move |checked, _, _| automatic.set(*checked),
                ))
                .child(volume_scope_row(
                    Theme::zed_dark(),
                    "synthetic-volume",
                    "合成卷",
                    "不读取真实卷",
                    volume.get(),
                    false,
                    move |checked, _, _| volume.set(*checked),
                ))
        }
    }
    #[gpui_kit::test]
    fn volume_scope_keyboard_uses_one_real_row_action(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let automatic = Rc::new(Cell::new(false));
        let volume = Rc::new(Cell::new(false));
        let automatic_view = automatic.clone();
        let volume_view = volume.clone();
        let (_, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|_| VolumeFixture {
                automatic: automatic_view,
                volume: volume_view,
            });
            gpui_kit::base::Root::new(view, window, cx)
        });
        window.refresh().expect("render real volume controls");
        let row = window
            .debug_bounds("synthetic-volume")
            .expect("volume content budget");
        window.update(|window, cx| window.focus_next(cx));
        window.refresh().expect("focus automatic row");
        window.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: gpui_kit::Keystroke::parse("space").expect("toggle key"),
            is_held: false,
            prefer_character_input: false,
        });
        window.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("space").expect("toggle release"),
        });
        assert!(
            automatic.get(),
            "Space must change actual automatic-scope state"
        );
        window.simulate_keystrokes("tab");
        window
            .refresh()
            .expect("focus volume row without inert checkbox stop");
        window.simulate_event(gpui_kit::KeyDownEvent {
            keystroke: gpui_kit::Keystroke::parse("enter").expect("volume key"),
            is_held: false,
            prefer_character_input: false,
        });
        window.simulate_event(gpui_kit::KeyUpEvent {
            keystroke: gpui_kit::Keystroke::parse("enter").expect("volume release"),
        });
        assert!(
            volume.get(),
            "Enter must change actual selected-volume state"
        );
        assert!(
            automatic.get(),
            "Tab must advance rather than toggle previous row twice"
        );
        window
            .refresh()
            .expect("focus ring preserves volume geometry");
        assert_eq!(window.debug_bounds("synthetic-volume"), Some(row));
        assert_eq!(row.size.height, px(40.0));
    }
}

#[cfg(test)]
mod queue01_tests {
    use super::tests::VolumeFixture;
    use super::*;
    #[gpui_kit::test]
    fn queue01_volume_rows_are_stateless_kit_list_items(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let (_, window) = cx.add_window_view(|window, cx| {
            let view = cx.new(|_| VolumeFixture {
                automatic: Rc::new(std::cell::Cell::new(true)),
                volume: Rc::new(std::cell::Cell::new(false)),
            });
            gpui_kit::base::Root::new(view, window, cx)
        });
        window.refresh().expect("production volume rows");
        let snapshots = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        for id in ["quick-launch-volume-scope-automatic", "synthetic-volume"] {
            assert!(
                snapshots
                    .iter()
                    .any(|snapshot| snapshot.path().last() == Some(&gpui_kit::ElementId::from(id))),
                "actual ListItem: {id}"
            );
        }
    }
}
