use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{App, ClickEvent, ObjectFit, Window, div, img, prelude::*, px};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Icon, IconName};
use nxtools_clipboard::{ClipboardAction, ClipboardItem, ClipboardKind};

use crate::theme::Theme;
use crate::ui::action_from_modifiers;

pub const IMAGE_PREVIEW_FALLBACK_HEIGHT: f32 = 96.0;
pub const IMAGE_PREVIEW_MAX_HEIGHT: f32 = 270.0;
const ROW_VERTICAL_PADDING: f32 = 8.0;
const METADATA_HEIGHT: f32 = 20.0;
const CONTENT_GAP: f32 = 4.0;
const TEXT_CONTENT_HEIGHT: f32 = 23.0;
const IMAGE_WELL_VERTICAL_PADDING: f32 = 8.0;
pub const TEXT_ROW_HEIGHT: f32 =
    ROW_VERTICAL_PADDING + METADATA_HEIGHT + CONTENT_GAP + TEXT_CONTENT_HEIGHT;
pub const IMAGE_ROW_CHROME_HEIGHT: f32 =
    ROW_VERTICAL_PADDING + METADATA_HEIGHT + CONTENT_GAP + IMAGE_WELL_VERTICAL_PADDING;
pub const ROW_GAP: f32 = 4.0;
pub type HistoryActionHandler = Rc<dyn Fn(ClipboardAction, &ClickEvent, &mut Window, &mut App)>;
pub type HistoryManagementHandler =
    Rc<dyn Fn(ClipboardItemAction, &ClickEvent, &mut Window, &mut App)>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardItemAction {
    Edit,
    TogglePinned,
    ToggleFavorite,
    Delete,
    ToggleCategorySource { category_id: String, remove: bool },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorySourceOption {
    pub category_id: String,
    pub name: String,
    pub included: bool,
}

pub fn history_row(
    theme: Theme,
    entry: ClipboardItem,
    index: usize,
    available_image_width: f32,
    category_options: Vec<CategorySourceOption>,
    on_action: HistoryActionHandler,
    on_management: HistoryManagementHandler,
) -> impl IntoElement {
    let source = if entry.source.process_name.is_empty() {
        "未知来源".to_owned()
    } else {
        entry.source.process_name.clone()
    };
    let source_icon_path = entry.source.process_icon_path.clone();
    let relative_time = relative_time(entry.captured_at_unix_ms);
    let usage = (entry.copy_count > 1).then(|| format!("{}×", entry.copy_count));
    let card_height = row_card_height(&entry, available_image_width);
    let slot_height = card_height + ROW_GAP;
    let can_edit = entry.kind == ClipboardKind::Text;
    let is_pinned = entry.is_pinned;
    let is_favorite = entry.is_favorite;
    let content = match entry.kind {
        ClipboardKind::Text => text_content(theme, &entry),
        ClipboardKind::Image => image_content(theme, &entry, available_image_width),
        ClipboardKind::Files => files_content(theme, &entry),
    };
    let click_action = Rc::clone(&on_action);
    let original_action = Rc::clone(&on_action);
    let copy_action = Rc::clone(&on_action);
    let plain_action = Rc::clone(&on_action);
    let without_breaks_action = Rc::clone(&on_action);
    let edit_action = Rc::clone(&on_management);
    let pin_action = Rc::clone(&on_management);
    let favorite_action = Rc::clone(&on_management);
    let delete_action = Rc::clone(&on_management);
    let menu_edit_action = Rc::clone(&on_management);
    let menu_pin_action = Rc::clone(&on_management);
    let menu_favorite_action = Rc::clone(&on_management);
    let menu_delete_action = Rc::clone(&on_management);
    let menu_category_action = Rc::clone(&on_management);
    let without_breaks_disabled = entry.kind != ClipboardKind::Text;
    let group_name = format!("history-row-actions-{index}");

    div()
        .id(("history-row-slot", index))
        .h(px(slot_height))
        .pb(px(ROW_GAP))
        .child(
            div()
                .id(("history-row", index))
                .group(group_name.clone())
                .h(px(card_height))
                .px(px(7.0))
                .py(px(4.0))
                .rounded(px(6.0))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.surface)
                .cursor_pointer()
                .hover(move |style| {
                    style
                        .bg(theme.surface_hover)
                        .border_color(theme.border_active)
                })
                .on_click(move |event, window, cx| {
                    let Some(action) = action_from_click(event) else {
                        return;
                    };
                    click_action(action, event, window, cx);
                })
                .flex()
                .flex_col()
                .gap(px(CONTENT_GAP))
                .child(
                    div()
                        .relative()
                        .h(px(METADATA_HEIGHT))
                        .flex()
                        .items_center()
                        .gap(px(5.0))
                        .when_some(usage, |row, usage| row.child(usage_badge(theme, usage)))
                        .child(source_icon(theme, source, source_icon_path, index))
                        .child(time_label(theme, relative_time))
                        .child(div().flex_1())
                        .when(is_pinned, |row| {
                            row.child(state_icon(theme, IconName::ArrowUp, "已置顶"))
                        })
                        .when(is_favorite, |row| {
                            row.child(state_icon(theme, IconName::Star, "已收藏"))
                        })
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .invisible()
                                .group_hover(group_name, |style| style.visible())
                                .h(px(METADATA_HEIGHT))
                                .pl_1()
                                .bg(theme.surface_hover)
                                .flex()
                                .items_center()
                                .gap(px(2.0))
                                .when(can_edit, |actions| {
                                    actions.child(management_button(
                                        theme,
                                        IconName::Replace,
                                        "编辑文本",
                                        ClipboardItemAction::Edit,
                                        edit_action,
                                        false,
                                    ))
                                })
                                .child(management_button(
                                    theme,
                                    IconName::ArrowUp,
                                    if is_pinned { "取消置顶" } else { "置顶" },
                                    ClipboardItemAction::TogglePinned,
                                    pin_action,
                                    is_pinned,
                                ))
                                .child(management_button(
                                    theme,
                                    if is_favorite {
                                        IconName::StarOff
                                    } else {
                                        IconName::Star
                                    },
                                    if is_favorite {
                                        "取消收藏"
                                    } else {
                                        "收藏"
                                    },
                                    ClipboardItemAction::ToggleFavorite,
                                    favorite_action,
                                    is_favorite,
                                ))
                                .child(management_button(
                                    theme,
                                    IconName::Delete,
                                    "删除",
                                    ClipboardItemAction::Delete,
                                    delete_action,
                                    false,
                                )),
                        ),
                )
                .child(content)
                .context_menu(move |menu, window, cx| {
                    let menu = menu
                        .item(PopupMenuItem::new("粘贴").on_click({
                            let action = Rc::clone(&original_action);
                            move |event, window, cx| {
                                action(ClipboardAction::OriginalPaste, event, window, cx);
                            }
                        }))
                        .item(PopupMenuItem::new("复制").on_click({
                            let action = Rc::clone(&copy_action);
                            move |event, window, cx| {
                                action(ClipboardAction::CopyOnly, event, window, cx);
                            }
                        }))
                        .item(PopupMenuItem::new("纯文本粘贴").on_click({
                            let action = Rc::clone(&plain_action);
                            move |event, window, cx| {
                                action(ClipboardAction::PlainText, event, window, cx);
                            }
                        }))
                        .item(
                            PopupMenuItem::new("去除换行")
                                .disabled(without_breaks_disabled)
                                .on_click({
                                    let action = Rc::clone(&without_breaks_action);
                                    move |event, window, cx| {
                                        action(
                                            ClipboardAction::WithoutLineBreaks,
                                            event,
                                            window,
                                            cx,
                                        );
                                    }
                                }),
                        )
                        .separator()
                        .item(PopupMenuItem::new("编辑").disabled(!can_edit).on_click({
                            let action = Rc::clone(&menu_edit_action);
                            move |event, window, cx| {
                                action(ClipboardItemAction::Edit, event, window, cx);
                            }
                        }))
                        .item(
                            PopupMenuItem::new(if is_pinned { "取消置顶" } else { "置顶" })
                                .on_click({
                                    let action = Rc::clone(&menu_pin_action);
                                    move |event, window, cx| {
                                        action(
                                            ClipboardItemAction::TogglePinned,
                                            event,
                                            window,
                                            cx,
                                        );
                                    }
                                }),
                        )
                        .item(
                            PopupMenuItem::new(if is_favorite {
                                "取消收藏"
                            } else {
                                "收藏"
                            })
                            .on_click({
                                let action = Rc::clone(&menu_favorite_action);
                                move |event, window, cx| {
                                    action(ClipboardItemAction::ToggleFavorite, event, window, cx);
                                }
                            }),
                        )
                        .item(PopupMenuItem::new("删除").on_click({
                            let action = Rc::clone(&menu_delete_action);
                            move |event, window, cx| {
                                action(ClipboardItemAction::Delete, event, window, cx);
                            }
                        }));
                    if category_options.is_empty() {
                        menu
                    } else {
                        let options = category_options.clone();
                        let management = Rc::clone(&menu_category_action);
                        menu.separator().submenu(
                            "来源分类",
                            window,
                            cx,
                            move |submenu, _window, _cx| {
                                options
                                    .clone()
                                    .into_iter()
                                    .fold(submenu, |submenu, option| {
                                        let label = if option.included {
                                            format!("✓ {}", option.name)
                                        } else {
                                            option.name.clone()
                                        };
                                        let management = Rc::clone(&management);
                                        submenu.item(PopupMenuItem::new(label).on_click(
                                            move |event, window, cx| {
                                                management(
                                                    ClipboardItemAction::ToggleCategorySource {
                                                        category_id: option.category_id.clone(),
                                                        remove: option.included,
                                                    },
                                                    event,
                                                    window,
                                                    cx,
                                                );
                                            },
                                        ))
                                    })
                            },
                        )
                    }
                }),
        )
}

fn management_button(
    theme: Theme,
    icon: IconName,
    tooltip: &'static str,
    action: ClipboardItemAction,
    handler: HistoryManagementHandler,
    active: bool,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(format!(
            "clipboard-item-action-{tooltip}"
        )))
        .size(px(20.0))
        .rounded(px(4.0))
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .hover(move |style| style.bg(theme.surface_raised))
        .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx))
        .child(Icon::new(icon).size(px(13.0)).text_color(if active {
            theme.accent
        } else {
            theme.text_secondary
        }))
        .on_click(move |event, window, cx| {
            cx.stop_propagation();
            handler(action.clone(), event, window, cx);
        })
}

fn state_icon(theme: Theme, icon: IconName, tooltip: &'static str) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(format!(
            "clipboard-state-icon-{tooltip}"
        )))
        .size(px(16.0))
        .flex()
        .items_center()
        .justify_center()
        .tooltip(move |window, cx| Tooltip::new(tooltip).build(window, cx))
        .child(Icon::new(icon).size(px(12.0)).text_color(theme.accent))
}

fn action_from_click(event: &ClickEvent) -> Option<ClipboardAction> {
    let modifiers = event.modifiers();
    if modifiers.modified() {
        action_from_modifiers(modifiers)
    } else {
        Some(ClipboardAction::OriginalPaste)
    }
}

fn text_content(theme: Theme, entry: &ClipboardItem) -> gpui::AnyElement {
    let character_count = entry.text.chars().count();
    div()
        .min_w_0()
        .h(px(TEXT_CONTENT_HEIGHT))
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text)
                .truncate()
                .child(entry.text.clone()),
        )
        .child(metric_chip(theme, format!("{character_count} 字符")))
        .into_any_element()
}

fn files_content(theme: Theme, entry: &ClipboardItem) -> gpui::AnyElement {
    let preview = entry
        .file_paths
        .iter()
        .map(|path| {
            PathBuf::from(path)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.clone())
        })
        .collect::<Vec<_>>()
        .join("  ·  ");
    div()
        .min_w_0()
        .h(px(TEXT_CONTENT_HEIGHT))
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .min_w_0()
                .flex_1()
                .text_size(px(12.0))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text)
                .truncate()
                .child(preview),
        )
        .child(metric_chip(theme, format!("{} 项", entry.file_paths.len())))
        .into_any_element()
}

fn image_content(
    theme: Theme,
    entry: &ClipboardItem,
    available_image_width: f32,
) -> gpui::AnyElement {
    let preview_height =
        image_preview_height(entry.image_width, entry.image_height, available_image_width);
    div()
        .w_full()
        .h(px(preview_height + 8.0))
        .p(px(4.0))
        .rounded(px(5.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.image_well)
        .flex()
        .items_center()
        .justify_center()
        .child(
            img(PathBuf::from(&entry.image_path))
                .w_full()
                .h_full()
                .object_fit(ObjectFit::Contain),
        )
        .into_any_element()
}

fn source_icon(
    theme: Theme,
    source: String,
    process_icon_path: String,
    index: usize,
) -> impl IntoElement {
    let icon = if process_icon_path.is_empty() {
        Icon::new(IconName::Frame)
            .size(px(15.0))
            .text_color(theme.text_secondary)
            .into_any_element()
    } else {
        img(PathBuf::from(process_icon_path))
            .size(px(16.0))
            .object_fit(ObjectFit::Contain)
            .into_any_element()
    };
    div()
        .id(("clipboard-source", index))
        .size(px(16.0))
        .flex()
        .items_center()
        .justify_center()
        .tooltip(move |window, cx| Tooltip::new(source.clone()).build(window, cx))
        .child(icon)
}

fn usage_badge(theme: Theme, text: String) -> impl IntoElement {
    div()
        .min_w(px(20.0))
        .h(px(18.0))
        .px(px(6.0))
        .rounded(px(9.0))
        .bg(theme.surface_raised)
        .text_size(px(9.0))
        .text_color(theme.text_secondary)
        .flex()
        .items_center()
        .justify_center()
        .truncate()
        .child(text)
}

fn time_label(theme: Theme, text: String) -> impl IntoElement {
    div()
        .max_w(px(92.0))
        .h(px(18.0))
        .px(px(7.0))
        .rounded(px(9.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface_raised)
        .text_size(px(9.0))
        .text_color(theme.text_muted)
        .flex()
        .items_center()
        .truncate()
        .child(text)
}

fn metric_chip(theme: Theme, text: String) -> impl IntoElement {
    div()
        .h(px(18.0))
        .px(px(7.0))
        .rounded(px(9.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface_raised)
        .text_size(px(9.0))
        .text_color(theme.text_muted)
        .flex()
        .items_center()
        .child(text)
}

fn row_card_height(entry: &ClipboardItem, available_image_width: f32) -> f32 {
    if entry.kind == ClipboardKind::Image {
        image_preview_height(entry.image_width, entry.image_height, available_image_width)
            + IMAGE_ROW_CHROME_HEIGHT
    } else {
        TEXT_ROW_HEIGHT
    }
}

fn image_preview_height(width: u32, height: u32, available_width: f32) -> f32 {
    if width == 0 || height == 0 || available_width <= 0.0 {
        return IMAGE_PREVIEW_FALLBACK_HEIGHT;
    }
    let scale = 1.0_f32
        .min(available_width / width as f32)
        .min(IMAGE_PREVIEW_MAX_HEIGHT / height as f32);
    (height as f32 * scale).max(1.0)
}

fn relative_time(timestamp_ms: i64) -> String {
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(timestamp_ms);
    let seconds = now_ms.saturating_sub(timestamp_ms).max(0) / 1_000;
    match seconds {
        0..=59 => "刚刚".into(),
        60..=3_599 => format!("{} 分钟前", seconds / 60),
        3_600..=86_399 => format!("{} 小时前", seconds / 3_600),
        _ => format!("{} 天前", seconds / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn row_metrics_match_the_reference_panel() {
        assert_eq!(TEXT_ROW_HEIGHT, 55.0);
        assert_eq!(ROW_GAP, 4.0);
        assert_eq!(IMAGE_PREVIEW_MAX_HEIGHT, 270.0);
        assert_eq!(IMAGE_ROW_CHROME_HEIGHT, 40.0);
    }

    #[test]
    fn image_preview_preserves_aspect_ratio_without_upscaling() {
        assert!((image_preview_height(270, 33, 246.0) - 30.07).abs() < 0.01);
        assert_eq!(image_preview_height(100, 120, 246.0), 120.0);
        assert_eq!(image_preview_height(1_000, 2_000, 246.0), 270.0);
        assert!((image_preview_height(1_000, 10, 246.0) - 2.46).abs() < 0.01);
    }
}
