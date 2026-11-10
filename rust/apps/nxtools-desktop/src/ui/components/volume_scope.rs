use gpui_kit::base::TestSupportExt as _;
use std::rc::Rc;

use gpui_kit::component::checkbox::Checkbox;
use gpui_kit::component::label::Label;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::tag::Tag;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Sizable, Size};
use gpui_kit::{AnyElement, App, IntoElement, SharedString, Window, div, prelude::*, px};

use crate::theme::Theme;
use crate::ui::components::action::focusable_content;
use crate::ui::metrics::{RADIUS_CONTROL, ordinary_rem};
use crate::ui::typography::{
    LINE_HEIGHT_WORKBENCH_BODY as LINE_HEIGHT_BODY,
    LINE_HEIGHT_WORKBENCH_SUPPORT as LINE_HEIGHT_LABEL, TEXT_WORKBENCH_BODY as TEXT_BODY,
    TEXT_WORKBENCH_SUPPORT as TEXT_LABEL,
};
use nxtools_protocol::{IndexDriveMaintenance, IndexRebuildReason};

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
        .py_3()
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .h(ordinary_rem(LINE_HEIGHT_BODY))
                        .text_size(ordinary_rem(TEXT_BODY))
                        .line_height(ordinary_rem(LINE_HEIGHT_BODY))
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
        .h(ordinary_rem(VOLUME_SCOPE_AUTOMATIC_ROW_HEIGHT))
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
                .gap_2()
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
                        .gap_1()
                        .child(
                            div()
                                .h(ordinary_rem(LINE_HEIGHT_BODY))
                                .text_size(ordinary_rem(TEXT_BODY))
                                .truncate()
                                .line_height(ordinary_rem(LINE_HEIGHT_BODY))
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

pub(crate) struct VolumeScopeRowState {
    pub maintenance: Option<IndexDriveMaintenance>,
    pub checked: bool,
    pub show_divider: bool,
}

pub(crate) fn volume_scope_row(
    theme: Theme,
    cx: &App,
    selector: impl Into<SharedString>,
    title: impl Into<SharedString>,
    metadata: impl Into<SharedString>,
    state: VolumeScopeRowState,
    on_click: impl Fn(&bool, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let VolumeScopeRowState {
        maintenance,
        checked,
        show_divider,
    } = state;
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
    let status = maintenance.map(|state| {
        let colors = gpui_kit::component::Theme::global(cx).colors;
        let (tag, label, foreground, description) = match state {
            IndexDriveMaintenance::InitialBuilding => {
                (Tag::info(), "首次建立", colors.info_foreground, None)
            }
            IndexDriveMaintenance::Rebuilding { reason } => {
                let reason = match reason {
                    IndexRebuildReason::JournalWrapped => "日志已覆盖",
                    IndexRebuildReason::BulkChanges => "变更过多",
                    IndexRebuildReason::CursorInvalid => "日志游标无效",
                    IndexRebuildReason::Requested => "手动请求",
                };
                (
                    Tag::warning(),
                    "正在重建",
                    colors.warning_foreground,
                    Some(format!("正在重建，搜索结果可能不完整 · {reason}")),
                )
            }
            IndexDriveMaintenance::Ready => {
                (Tag::success(), "就绪", colors.success_foreground, None)
            }
        };
        let tag_selector = format!("{selector}-maintenance-tag");
        let tag = div()
            .id(SharedString::from(format!(
                "{selector}-maintenance-tag-label"
            )))
            .role(gpui_kit::Role::Label)
            .aria_label(label)
            .test_support()
            .debug_selector(move || tag_selector.clone())
            .flex_none()
            .child(
                tag.with_size(Size::Small).child(
                    Label::new(label)
                        .text_size(ordinary_rem(10.0))
                        .line_height(ordinary_rem(14.0))
                        .text_color(foreground),
                ),
            );
        let description = description.map(|text| {
            let description_selector = format!("{selector}-maintenance-description");
            let tooltip_text = text.clone();
            div()
                .id(SharedString::from(format!("{selector}-maintenance-detail")))
                .role(gpui_kit::Role::Label)
                .aria_label(text.clone())
                .test_support()
                .debug_selector(move || description_selector.clone())
                .w_full()
                .min_w_0()
                .truncate()
                .text_size(ordinary_rem(10.0))
                .line_height(ordinary_rem(14.0))
                .text_color(theme.text_secondary)
                .tooltip(move |window, cx| Tooltip::new(tooltip_text.clone()).build(window, cx))
                .child(text)
        });
        (tag.into_any_element(), description)
    });
    let content = ListItem::new(selector.clone())
        .debug_selector(move || row_selector.clone())
        .accessibility_label(title.clone())
        .w_full()
        .h(ordinary_rem(VOLUME_SCOPE_ROW_HEIGHT))
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
                .gap_2()
                .child(
                    Checkbox::new(checkbox_id.clone())
                        .checked(checked)
                        .tab_stop(false)
                        .with_size(Size::Small),
                )
                .child(if let Some((tag, description)) = status {
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .child(
                            div()
                                .w_full()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .debug_selector(move || title_selector.clone())
                                        .min_w_0()
                                        .flex_1()
                                        .truncate()
                                        .text_size(ordinary_rem(TEXT_BODY))
                                        .line_height(ordinary_rem(LINE_HEIGHT_BODY))
                                        .text_color(theme.text)
                                        .child(title),
                                )
                                .child(
                                    div()
                                        .id(SharedString::from(format!("{selector}-status")))
                                        .debug_selector(move || metadata_selector.clone())
                                        .w(ordinary_rem(100.0))
                                        .flex_none()
                                        .truncate()
                                        .text_size(ordinary_rem(TEXT_LABEL))
                                        .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                                        .text_color(theme.text_muted)
                                        .tooltip(move |window, cx| {
                                            Tooltip::new(metadata_tooltip.clone()).build(window, cx)
                                        })
                                        .child(metadata),
                                )
                                .child(tag),
                        )
                        .when_some(description, |row, description| row.child(description))
                        .into_any_element()
                } else {
                    div()
                        .min_w_0()
                        .flex_1()
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .debug_selector(move || title_selector.clone())
                                .h(ordinary_rem(LINE_HEIGHT_BODY))
                                .min_w_0()
                                .flex_1()
                                .truncate()
                                .text_size(ordinary_rem(TEXT_BODY))
                                .line_height(ordinary_rem(LINE_HEIGHT_BODY))
                                .text_color(theme.text)
                                .child(title),
                        )
                        .child(
                            div()
                                .id(SharedString::from(format!("{selector}-status")))
                                .debug_selector(move || metadata_selector.clone())
                                .h(ordinary_rem(LINE_HEIGHT_LABEL))
                                .w(ordinary_rem(196.0))
                                .min_w_0()
                                .flex_none()
                                .truncate()
                                .text_size(ordinary_rem(TEXT_LABEL))
                                .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
                                .text_color(theme.text_muted)
                                .tooltip(move |window, cx| {
                                    Tooltip::new(metadata_tooltip.clone()).build(window, cx)
                                })
                                .child(metadata),
                        )
                        .into_any_element()
                }),
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
        .h(ordinary_rem(VOLUME_SCOPE_ROW_HEIGHT))
        .px_3()
        .flex()
        .items_center()
        .text_size(ordinary_rem(TEXT_LABEL))
        .line_height(ordinary_rem(LINE_HEIGHT_LABEL))
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
        fn render(&mut self, _: &mut Window, cx: &mut gpui_kit::Context<Self>) -> impl IntoElement {
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
                    cx,
                    "synthetic-volume",
                    "合成卷",
                    "不读取真实卷",
                    VolumeScopeRowState {
                        maintenance: None,
                        checked: volume.get(),
                        show_divider: false,
                    },
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
