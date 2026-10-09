use std::rc::Rc;

use gpui::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};
use gpui_component::checkbox::Checkbox;
use gpui_component::{Sizable, Size};

use crate::theme::Theme;
use crate::ui::components::action::focusable_content;
use crate::ui::metrics::{RADIUS_CONTROL, SPACING_4, SPACING_8, SPACING_12};
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
                .gap(px(SPACING_4))
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
    let state_selector = format!(
        "quick-launch-volume-scope-automatic-{}",
        if checked { "checked" } else { "unchecked" }
    );
    let content = div()
        .id("quick-launch-volume-scope-automatic")
        .debug_selector(|| "quick-launch-volume-scope-automatic".into())
        .w_full()
        .h(px(VOLUME_SCOPE_AUTOMATIC_ROW_HEIGHT))
        .cursor_pointer()
        .hover(move |style| style.bg(theme.surface_hover))
        .border_b_1()
        .border_color(theme.border_subtle)
        .child(
            div()
                .debug_selector(move || state_selector.clone())
                .size_full()
                .px(px(SPACING_12))
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
                        .flex_col()
                        .justify_center()
                        .gap(px(SPACING_4))
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
    let title_selector = format!("{selector}-title");
    let metadata_selector = format!("{selector}-metadata");
    let checkbox_id: SharedString = format!("{selector}-checkbox").into();
    let row_selector = selector.to_string();
    let state_selector = format!(
        "{selector}-{}",
        if checked { "checked" } else { "unchecked" }
    );
    let action_id = SharedString::from(format!("{selector}-action"));
    let content = div()
        .id(selector.clone())
        .debug_selector(move || row_selector.clone())
        .w_full()
        .h(px(VOLUME_SCOPE_ROW_HEIGHT))
        .cursor_pointer()
        .hover(move |style| style.bg(theme.surface_hover))
        .when(show_divider, |row| {
            row.border_b_1().border_color(theme.border_subtle)
        })
        .child(
            div()
                .debug_selector(move || state_selector.clone())
                .size_full()
                .px(px(SPACING_12))
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
    struct VolumeFixture {
        automatic: Rc<Cell<bool>>,
        volume: Rc<Cell<bool>>,
    }
    impl gpui::Render for VolumeFixture {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
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
    #[gpui::test]
    fn volume_scope_keyboard_uses_one_real_row_action(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        cx.update(crate::theme::apply_component_theme);
        let automatic = Rc::new(Cell::new(false));
        let volume = Rc::new(Cell::new(false));
        let automatic_view = automatic.clone();
        let volume_view = volume.clone();
        let (_, window) = cx.add_window_view(move |_, _| VolumeFixture {
            automatic: automatic_view,
            volume: volume_view,
        });
        window.refresh().expect("render real volume controls");
        let row = window
            .debug_bounds("synthetic-volume")
            .expect("volume content budget");
        window.update(|window, _| window.focus_next());
        window.refresh().expect("focus automatic row");
        window.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse("space").expect("toggle key"),
            is_held: false,
        });
        window.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("space").expect("toggle release"),
        });
        assert!(
            automatic.get(),
            "Space must change actual automatic-scope state"
        );
        window.simulate_keystrokes("tab");
        window
            .refresh()
            .expect("focus volume row without inert checkbox stop");
        window.simulate_event(gpui::KeyDownEvent {
            keystroke: gpui::Keystroke::parse("enter").expect("volume key"),
            is_held: false,
        });
        window.simulate_event(gpui::KeyUpEvent {
            keystroke: gpui::Keystroke::parse("enter").expect("volume release"),
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
