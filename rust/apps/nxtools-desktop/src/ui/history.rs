use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui::{
    AnyElement, App, Bounds, ClickEvent, Corner, ObjectFit, Pixels, Point, Rgba, Window, div, img,
    point, prelude::*, px,
};
use gpui_component::Icon;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::menu::DropdownMenu;
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::tooltip::Tooltip;
use nxtools_clipboard::{
    ClipboardAction, ClipboardItem, ClipboardKind, ClipboardOpenTarget, ClipboardOpenTargetKind,
    resolve_open_targets,
};
use nxtools_core::ClipboardModifierActions;

use crate::icons::MaterialIcon;
use crate::theme::{COMPACT_RADIUS, META_TEXT_SIZE, RESULT_TITLE_SIZE, Theme};
use crate::ui::action_from_modifiers;
use crate::ui::metrics::RADIUS_CONTROL;

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
pub type HistoryOpenHandler = Rc<dyn Fn(ClipboardOpenTarget, &ClickEvent, &mut Window, &mut App)>;
pub type HistoryPreviewHandler =
    Rc<dyn Fn(bool, ClipboardItem, Bounds<Pixels>, Point<Pixels>, f32, &mut App)>;

pub struct HistoryRowHandlers {
    pub action: HistoryActionHandler,
    pub management: HistoryManagementHandler,
    pub open: HistoryOpenHandler,
    pub preview: HistoryPreviewHandler,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategorySourceMenu {
    pub options: Vec<CategorySourceOption>,
    pub unavailable_hint: Option<String>,
}

pub fn history_row(
    theme: Theme,
    entry: ClipboardItem,
    index: usize,
    available_image_width: f32,
    modifier_actions: ClipboardModifierActions,
    category_menu: CategorySourceMenu,
    handlers: HistoryRowHandlers,
) -> impl IntoElement {
    let HistoryRowHandlers {
        action: on_action,
        management: on_management,
        open: on_open,
        preview: on_preview,
    } = handlers;
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
    let open_targets = resolve_open_targets(&entry);
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
    let preview_entry = entry.clone();

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
                .rounded(px(COMPACT_RADIUS))
                .border_1()
                .border_color(history_row_border(theme, false))
                .bg(history_row_background(theme, false))
                .overflow_hidden()
                .cursor_pointer()
                .hover(move |style| {
                    style
                        .bg(history_row_background(theme, true))
                        .border_color(history_row_border(theme, true))
                })
                .on_hover(move |hovered, window, cx| {
                    let panel_bounds = window.bounds();
                    let mouse = window.mouse_position();
                    on_preview(
                        *hovered,
                        preview_entry.clone(),
                        panel_bounds,
                        point(
                            panel_bounds.origin.x + mouse.x,
                            panel_bounds.origin.y + mouse.y,
                        ),
                        window.scale_factor(),
                        cx,
                    );
                })
                .on_click(move |event, window, cx| {
                    let Some(action) = action_from_click(event, modifier_actions) else {
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
                            row.child(state_icon(theme, MaterialIcon::Keep, "已置顶"))
                        })
                        .when(is_favorite, |row| {
                            row.child(state_icon(theme, MaterialIcon::Star, "已收藏"))
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
                                .when(!open_targets.is_empty(), |actions| {
                                    actions.child(open_targets_button(
                                        theme,
                                        index,
                                        open_targets,
                                        on_open,
                                    ))
                                })
                                .when(can_edit, |actions| {
                                    actions.child(management_button(
                                        theme,
                                        MaterialIcon::Edit,
                                        "编辑文本",
                                        ClipboardItemAction::Edit,
                                        edit_action,
                                        false,
                                    ))
                                })
                                .child(management_button(
                                    theme,
                                    MaterialIcon::Keep,
                                    if is_pinned { "取消置顶" } else { "置顶" },
                                    ClipboardItemAction::TogglePinned,
                                    pin_action,
                                    is_pinned,
                                ))
                                .child(management_button(
                                    theme,
                                    MaterialIcon::Star,
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
                                    MaterialIcon::Delete,
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
                        .min_w(px(0.0))
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
                    let options = category_menu.options.clone();
                    let unavailable_hint = category_menu.unavailable_hint.clone();
                    let management = Rc::clone(&menu_category_action);
                    menu.separator().submenu(
                        "来源分类",
                        window,
                        cx,
                        move |submenu, _window, _cx| {
                            let submenu = submenu.min_w(px(0.0));
                            if let Some(hint) = unavailable_hint.clone() {
                                submenu.item(PopupMenuItem::new(hint).disabled(true))
                            } else {
                                options
                                    .clone()
                                    .into_iter()
                                    .fold(submenu, |submenu, option| {
                                        let management = Rc::clone(&management);
                                        submenu.item(
                                            PopupMenuItem::new(option.name.clone())
                                                .checked(option.included)
                                                .on_click(move |event, window, cx| {
                                                    management(
                                                        ClipboardItemAction::ToggleCategorySource {
                                                            category_id: option.category_id.clone(),
                                                            remove: option.included,
                                                        },
                                                        event,
                                                        window,
                                                        cx,
                                                    );
                                                }),
                                        )
                                    })
                            }
                        },
                    )
                }),
        )
}

fn open_targets_button(
    theme: Theme,
    index: usize,
    targets: Vec<ClipboardOpenTarget>,
    handler: HistoryOpenHandler,
) -> AnyElement {
    let single_target_tooltip = targets.first().map(open_target_tooltip).unwrap_or("打开");
    let button = Button::new(("clipboard-open-target", index))
        .ghost()
        .compact()
        .h(px(20.0))
        .w(px(20.0))
        .p_0()
        .tooltip(if targets.len() == 1 {
            single_target_tooltip
        } else {
            "选择并打开"
        })
        .icon(
            Icon::new(MaterialIcon::OpenInNew)
                .size(px(13.0))
                .text_color(theme.text_secondary),
        );
    if targets.len() == 1 {
        let target = targets[0].clone();
        button
            .on_click(move |event, window, cx| {
                cx.stop_propagation();
                handler(target.clone(), event, window, cx);
            })
            .into_any_element()
    } else {
        button
            .dropdown_menu_with_anchor(Corner::TopRight, move |menu, _window, _cx| {
                targets.clone().into_iter().fold(
                    menu.min_w(px(180.0)).max_w(px(320.0)),
                    |menu, target| {
                        let handler = Rc::clone(&handler);
                        let label = format!(
                            "{}  {}",
                            open_target_menu_prefix(&target.kind),
                            compact_target_label(&target.value)
                        );
                        menu.item(
                            PopupMenuItem::new(label).on_click(move |event, window, cx| {
                                cx.stop_propagation();
                                handler(target.clone(), event, window, cx);
                            }),
                        )
                    },
                )
            })
            .into_any_element()
    }
}

fn open_target_tooltip(target: &ClipboardOpenTarget) -> &'static str {
    match target.kind {
        ClipboardOpenTargetKind::RegistryKey => "跳转到注册表项",
        ClipboardOpenTargetKind::SystemTool => "运行系统工具",
        ClipboardOpenTargetKind::Directory => "打开目录",
        ClipboardOpenTargetKind::File => "在资源管理器中定位",
        ClipboardOpenTargetKind::Web => "打开链接",
    }
}

fn open_target_menu_prefix(kind: &ClipboardOpenTargetKind) -> &'static str {
    match kind {
        ClipboardOpenTargetKind::RegistryKey => "注册表",
        ClipboardOpenTargetKind::SystemTool => "系统工具",
        ClipboardOpenTargetKind::Directory => "目录",
        ClipboardOpenTargetKind::File => "文件",
        ClipboardOpenTargetKind::Web => "链接",
    }
}

fn compact_target_label(value: &str) -> String {
    const MAX_CHARS: usize = 42;
    let mut characters = value.chars();
    let prefix = characters.by_ref().take(MAX_CHARS).collect::<String>();
    if characters.next().is_some() {
        format!("{prefix}…")
    } else {
        prefix
    }
}

fn management_button(
    theme: Theme,
    icon: MaterialIcon,
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
        .rounded(px(RADIUS_CONTROL))
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

fn state_icon(theme: Theme, icon: MaterialIcon, tooltip: &'static str) -> impl IntoElement {
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

fn action_from_click(
    event: &ClickEvent,
    modifier_actions: ClipboardModifierActions,
) -> Option<ClipboardAction> {
    let modifiers = event.modifiers();
    if modifiers.modified() {
        action_from_modifiers(modifiers, modifier_actions)
    } else {
        Some(ClipboardAction::OriginalPaste)
    }
}

fn text_content(theme: Theme, entry: &ClipboardItem) -> gpui::AnyElement {
    let character_count = entry.text.chars().count();
    let summary = single_line_summary(&entry.text);
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
                .text_size(px(RESULT_TITLE_SIZE))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text)
                .truncate()
                .child(summary),
        )
        .child(metric_chip(theme, format!("{character_count} 字符")))
        .into_any_element()
}

fn single_line_summary(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
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
                .text_size(px(RESULT_TITLE_SIZE))
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
    let image_path = PathBuf::from(&entry.image_path);
    let image_exists = image_path.is_file();
    div()
        .w_full()
        .h(px(preview_height + 8.0))
        .p(px(4.0))
        .rounded(px(COMPACT_RADIUS))
        .bg(theme.image_well)
        .flex()
        .items_center()
        .justify_center()
        .when(image_exists, |preview| {
            preview.child(
                img(image_path)
                    .w_full()
                    .h_full()
                    .object_fit(ObjectFit::Contain),
            )
        })
        .when(!image_exists, |preview| {
            preview.child(
                div()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .child("图片文件已失效"),
            )
        })
        .into_any_element()
}

fn source_icon(
    theme: Theme,
    source: String,
    process_icon_path: String,
    index: usize,
) -> impl IntoElement {
    let icon = if process_icon_path.is_empty() {
        Icon::new(MaterialIcon::Image)
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
        .h(px(16.0))
        .px(px(4.0))
        .rounded(px(RADIUS_CONTROL))
        .bg(theme.element)
        .text_size(px(META_TEXT_SIZE))
        .text_color(theme.text_secondary)
        .flex()
        .items_center()
        .justify_center()
        .truncate()
        .child(text)
}

fn history_row_border(theme: Theme, hovered: bool) -> Rgba {
    if hovered {
        theme.border
    } else {
        theme.border_subtle
    }
}

fn history_row_background(theme: Theme, hovered: bool) -> Rgba {
    if hovered {
        theme.surface_hover
    } else {
        theme.surface_raised
    }
}

fn time_label(theme: Theme, text: String) -> impl IntoElement {
    div()
        .max_w(px(92.0))
        .h(px(18.0))
        .px(px(2.0))
        .text_size(px(META_TEXT_SIZE))
        .text_color(theme.text_muted)
        .flex()
        .items_center()
        .truncate()
        .child(text)
}

fn metric_chip(theme: Theme, text: String) -> impl IntoElement {
    div()
        .h(px(18.0))
        .px(px(5.0))
        .rounded(px(RADIUS_CONTROL))
        .bg(theme.element)
        .text_size(px(META_TEXT_SIZE))
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
        assert_eq!(image_preview_height(100, 120, 186.0), 120.0);
        assert_eq!(image_preview_height(100, 120, 506.0), 120.0);
        assert_eq!(image_preview_height(900, 1_800, 186.0), 270.0);
    }

    #[test]
    fn history_text_summary_is_always_one_visual_line() {
        assert_eq!(
            single_line_summary("第一行\r\n第二行\t第三段"),
            "第一行 第二行 第三段"
        );
    }

    #[test]
    fn history_rows_use_launcher_style_neutral_layering() {
        let theme = Theme::zed_dark();
        assert_eq!(history_row_border(theme, false), theme.border_subtle);
        assert_eq!(history_row_border(theme, true), theme.border);
        assert_eq!(history_row_background(theme, false), theme.surface_raised);
        assert_eq!(history_row_background(theme, true), theme.surface_hover);

        let source = include_str!("history.rs");
        for function in ["fn image_content(", "fn time_label(", "fn metric_chip("] {
            let body = source
                .split(function)
                .nth(1)
                .and_then(|source| source.split("\n}").next())
                .expect("visual helper source");
            assert!(
                !body.contains(".border_1()"),
                "{function} should not add a nested outline"
            );
        }
    }
}
