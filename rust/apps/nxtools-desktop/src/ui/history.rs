use gpui_kit::component::{Selectable, Sizable};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::component::Icon;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::label::Label;
use gpui_kit::component::list::ListItem;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::toolbar::Toolbar;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::{
    Anchor, AnyElement, App, Bounds, ClickEvent, Entity, ObjectFit, Pixels, Point, RenderOnce,
    SharedString, Window, div, img, point, prelude::*, px,
};
use nxtools_clipboard::{
    ClipboardAction, ClipboardItem, ClipboardKind, ClipboardOpenTarget, ClipboardOpenTargetKind,
    resolve_open_targets,
};
use nxtools_core::ClipboardModifierActions;

use crate::icons::MaterialIcon;
use crate::theme::Theme;
use crate::ui::HOVER_ACTION_RAIL_WIDTH;
use crate::ui::action_from_modifiers;
use crate::ui::components::action::focusable_content;
use crate::ui::components::menu::{MenuTone, context_menu_item};
use crate::ui::components::project_menu::{self, MenuItem};
use crate::ui::metrics::RADIUS_CONTROL;
use crate::ui::typography::{LINE_HEIGHT_BODY, TEXT_BODY};

pub const IMAGE_PREVIEW_FALLBACK_HEIGHT: f32 = 96.0;
#[cfg(test)]
const IMAGE_PREVIEW_MAX_HEIGHT: f32 = 270.0;
pub const IMAGE_LIST_MAX_HEIGHT: f32 = 144.0;
const ROW_VERTICAL_PADDING: f32 = 10.0; // Official4px vertical inset plus1pxcard border each edge.
const METADATA_HEIGHT: f32 = 24.0;
const CONTENT_GAP: f32 = 4.0;
const TEXT_CONTENT_HEIGHT: f32 = 24.0;
const IMAGE_WELL_VERTICAL_PADDING: f32 = 8.0;
pub const TEXT_ROW_HEIGHT: f32 =
    ROW_VERTICAL_PADDING + METADATA_HEIGHT + CONTENT_GAP + TEXT_CONTENT_HEIGHT;
pub const TEXT_CARD_MAX_HEIGHT: f32 = 210.0;
pub const IMAGE_ROW_CHROME_HEIGHT: f32 =
    ROW_VERTICAL_PADDING + METADATA_HEIGHT + CONTENT_GAP + IMAGE_WELL_VERTICAL_PADDING;
pub const ROW_GAP: f32 = 8.0;
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
    pub list_image: Option<PathBuf>,
    pub source_icon_color: Option<[u8; 3]>,
    pub text_character_count: Option<usize>,
    pub open_targets: Option<Vec<ClipboardOpenTarget>>,
    pub(crate) extension_actions: Vec<crate::extensions::manual_actions::ManualAction>,
    pub(crate) extension_feedback:
        Option<gpui_kit::Entity<super::extension_feedback::InlineExtensionFeedback>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipboardItemAction {
    Edit,
    TogglePinned,
    ToggleFavorite,
    Delete,
    ToggleCategorySource {
        category_id: String,
        remove: bool,
    },
    RunExtension {
        action: crate::extensions::manual_actions::ManualAction,
    },
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

#[allow(clippy::too_many_arguments)]
pub fn history_row(
    theme: Theme,
    entry: ClipboardItem,
    index: usize,
    selected: bool,
    available_image_width: f32,
    modifier_actions: ClipboardModifierActions,
    category_menu: CategorySourceMenu,
    handlers: HistoryRowHandlers,
    actions_focused: bool,
) -> impl IntoElement {
    let HistoryRowHandlers {
        action: on_action,
        management: on_management,
        open: on_open,
        preview: on_preview,
        list_image,
        source_icon_color,
        text_character_count,
        open_targets,
        extension_actions,
        extension_feedback,
    } = handlers;
    let source = if entry.source.process_name.is_empty() {
        "未知来源".to_owned()
    } else {
        entry.source.process_name.clone()
    };
    let source_icon_path = entry.source.process_icon_path.clone();
    let relative_time = relative_time(entry.captured_at_unix_ms);
    let usage = (entry.copy_count > 1).then(|| format!("{}×", entry.copy_count));
    let feedback_height = if extension_feedback.is_some() {
        super::extension_feedback::ITEM_FEEDBACK_HEIGHT
    } else {
        0.0
    };
    let card_height = row_card_height(&entry, available_image_width) + feedback_height;
    let slot_height = card_height + ROW_GAP;
    let can_edit = entry.kind == ClipboardKind::Text;
    let is_pinned = entry.is_pinned;
    let is_favorite = entry.is_favorite;
    let open_targets = open_targets.unwrap_or_else(|| resolve_open_targets(&entry));
    let content = match entry.kind {
        ClipboardKind::Text => text_content(
            theme,
            &entry,
            text_character_count,
            Rc::clone(&on_action),
            modifier_actions,
            TEXT_CARD_MAX_HEIGHT
                - ROW_VERTICAL_PADDING
                - METADATA_HEIGHT
                - CONTENT_GAP
                - feedback_height,
        ),
        ClipboardKind::Image => image_content(theme, &entry, available_image_width, list_image),
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
    let category_items = if let Some(hint) = category_menu.unavailable_hint {
        vec![MenuItem::new("clipboard-menu-category-unavailable", hint).disabled(true)]
    } else {
        category_menu
            .options
            .into_iter()
            .map(|option| {
                let management = Rc::clone(&menu_category_action);
                MenuItem::new(
                    format!("clipboard-menu-category-{}", option.category_id),
                    option.name,
                )
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
                    )
                })
            })
            .collect()
    };
    let mut menu_items = vec![
        MenuItem::new("clipboard-menu-paste", "粘贴")
            .icon(MaterialIcon::ContentPaste)
            .on_click(move |event, window, cx| {
                original_action(ClipboardAction::OriginalPaste, event, window, cx)
            }),
        MenuItem::new("clipboard-menu-copy", "复制")
            .icon(MaterialIcon::ContentCopy)
            .on_click(move |event, window, cx| {
                copy_action(ClipboardAction::CopyOnly, event, window, cx)
            }),
        MenuItem::new("clipboard-menu-plain-paste", "纯文本粘贴")
            .icon(MaterialIcon::TextFields)
            .on_click(move |event, window, cx| {
                plain_action(ClipboardAction::PlainText, event, window, cx)
            }),
        MenuItem::new("clipboard-menu-without-breaks", "去除换行")
            .icon(MaterialIcon::Cancel)
            .disabled(without_breaks_disabled)
            .on_click(move |event, window, cx| {
                without_breaks_action(ClipboardAction::WithoutLineBreaks, event, window, cx)
            }),
        MenuItem::separator(),
        MenuItem::new("clipboard-menu-edit", "编辑")
            .icon(MaterialIcon::Edit)
            .disabled(!can_edit)
            .on_click(move |event, window, cx| {
                menu_edit_action(ClipboardItemAction::Edit, event, window, cx)
            }),
        MenuItem::new(
            "clipboard-menu-pin",
            if is_pinned { "取消置顶" } else { "置顶" },
        )
        .icon(MaterialIcon::ArrowUpward)
        .on_click(move |event, window, cx| {
            menu_pin_action(ClipboardItemAction::TogglePinned, event, window, cx)
        }),
        MenuItem::new(
            "clipboard-menu-favorite",
            if is_favorite {
                "取消收藏"
            } else {
                "收藏"
            },
        )
        .icon(MaterialIcon::Star)
        .on_click(move |event, window, cx| {
            menu_favorite_action(ClipboardItemAction::ToggleFavorite, event, window, cx)
        }),
        MenuItem::new("clipboard-menu-delete", "删除")
            .icon(MaterialIcon::Delete)
            .tone(MenuTone::Danger)
            .on_click(move |event, window, cx| {
                menu_delete_action(ClipboardItemAction::Delete, event, window, cx)
            }),
        MenuItem::separator(),
        MenuItem::new("clipboard-menu-categories", "来源分类").submenu(category_items),
    ];
    if crate::extensions::manual_actions::clipboard_context(&entry).is_some() {
        let children = extension_actions
            .into_iter()
            .map(|action| {
                let management = on_management.clone();
                MenuItem::new(
                    format!(
                        "clipboard-extension-{}-{}",
                        action.package_id, action.handler_id
                    ),
                    action.title.clone(),
                )
                .on_click(move |event, window, cx| {
                    management(
                        ClipboardItemAction::RunExtension {
                            action: action.clone(),
                        },
                        event,
                        window,
                        cx,
                    )
                })
            })
            .collect::<Vec<_>>();
        if !children.is_empty() {
            menu_items.push(
                MenuItem::new("clipboard-menu-extension", "扩展处理")
                    .icon(MaterialIcon::ManageSearch)
                    .submenu(children),
            );
        }
    }
    let menu_trigger_id = format!("clipboard-history-menu-{}", entry.id);
    let preview_entry = entry.clone();

    div()
        .id(("history-row-slot", index))
        .debug_selector(move || format!("history-row-slot-{index}"))
        .when(entry.kind != ClipboardKind::Text, |slot| {
            slot.h(px(slot_height))
        })
        .when(entry.kind == ClipboardKind::Text, |slot| {
            slot.min_h(px(TEXT_ROW_HEIGHT + ROW_GAP))
                .max_h(px(TEXT_CARD_MAX_HEIGHT + ROW_GAP))
        })
        .pb(px(ROW_GAP))
        .child(focusable_content(
            ("history-row-action", index),
            theme,
            false,
            project_menu::context_menu(
                menu_trigger_id,
                ListItem::new(("history-row", index))
                    .debug_selector(move || format!("clipboard-item-{}", entry.id))
                    .accessibility_label(format!("剪贴板记录 {}", entry.id))
                    .selected(selected)
                    .aria_selected(selected)
                    .group(group_name.clone())
                    .when(entry.kind != ClipboardKind::Text, |row| {
                        row.h(px(card_height))
                    })
                    .when(entry.kind == ClipboardKind::Text, |row| {
                        row.min_h(px(TEXT_ROW_HEIGHT))
                            .max_h(px(TEXT_CARD_MAX_HEIGHT))
                    })
                    .px_2()
                    .border_1()
                    .border_color(theme.border_subtle)
                    .rounded_md()
                    .bg(theme.surface)
                    .py_1()
                    .overflow_hidden()
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
                    .child(
                        div()
                            .w_full()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .gap(px(CONTENT_GAP))
                            .child(
                                div()
                                    .relative()
                                    .h(px(METADATA_HEIGHT))
                                    .flex_shrink_0()
                                    .pr(px(HOVER_ACTION_RAIL_WIDTH))
                                    .flex()
                                    .items_center()
                                    .gap(px(4.0))
                                    .child(source_icon(
                                        theme,
                                        source,
                                        source_icon_path,
                                        index,
                                        entry.kind,
                                        source_icon_color,
                                    ))
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
                                            .w(px(HOVER_ACTION_RAIL_WIDTH))
                                            .h(px(METADATA_HEIGHT))
                                            .flex()
                                            .items_center()
                                            .justify_end()
                                            .gap(px(4.0))
                                            .when(actions_focused, |slot| slot.invisible())
                                            .group_hover(group_name.clone(), |style| {
                                                style.invisible()
                                            })
                                            .when_some(usage, |row, usage| {
                                                row.child(usage_badge(theme, usage))
                                            })
                                            .child(time_label(theme, relative_time)),
                                    )
                                    .child(
                                        div()
                                            .debug_selector(move || {
                                                format!("clipboard-history-actions-{index}")
                                            })
                                            .map(|rail| {
                                                #[cfg(test)]
                                                let rail = rail
                                                    .child(feedback_tests::paint_probe("actions"));
                                                rail
                                            })
                                            .absolute()
                                            .top_0()
                                            .right_0()
                                            .w(px(HOVER_ACTION_RAIL_WIDTH))
                                            .when(!actions_focused, |rail| rail.invisible())
                                            .group_hover(group_name, |style| style.visible())
                                            .h(px(METADATA_HEIGHT))
                                            .pl_1()
                                            .flex()
                                            .items_center()
                                            .justify_end()
                                            .child(
                                                Toolbar::new(("clipboard-history-toolbar", index))
                                                    .xsmall()
                                                    .h(px(METADATA_HEIGHT))
                                                    .p_0()
                                                    .gap_1()
                                                    .when(!open_targets.is_empty(), |actions| {
                                                        actions.content(open_targets_button(
                                                            theme,
                                                            index,
                                                            open_targets,
                                                            on_open,
                                                        ))
                                                    })
                                                    .when(can_edit, |actions| {
                                                        actions.content(management_button(
                                                            theme,
                                                            index,
                                                            MaterialIcon::Edit,
                                                            "编辑文本",
                                                            ClipboardItemAction::Edit,
                                                            edit_action,
                                                            false,
                                                        ))
                                                    })
                                                    .content(management_button(
                                                        theme,
                                                        index,
                                                        MaterialIcon::Keep,
                                                        if is_pinned {
                                                            "取消置顶"
                                                        } else {
                                                            "置顶"
                                                        },
                                                        ClipboardItemAction::TogglePinned,
                                                        pin_action,
                                                        is_pinned,
                                                    ))
                                                    .content(management_button(
                                                        theme,
                                                        index,
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
                                                    .content(management_button(
                                                        theme,
                                                        index,
                                                        MaterialIcon::Delete,
                                                        "删除",
                                                        ClipboardItemAction::Delete,
                                                        delete_action,
                                                        false,
                                                    )),
                                            ),
                                    ),
                            )
                            .child(content)
                            .when_some(extension_feedback, |content, feedback| {
                                content.child(feedback)
                            }),
                    )
                    .into_any_element(),
                menu_items,
            ),
            move |event, window, cx| {
                let Some(action) = action_from_click(event, modifier_actions) else {
                    return;
                };
                click_action(action, event, window, cx);
            },
        ))
}

fn open_targets_button(
    _theme: Theme,
    index: usize,
    targets: Vec<ClipboardOpenTarget>,
    handler: HistoryOpenHandler,
) -> AnyElement {
    let single_target_tooltip = targets.first().map(open_target_tooltip).unwrap_or("打开");
    let button = Button::new(("clipboard-open-target", index))
        .ghost()
        .xsmall()
        .debug_selector(move || format!("clipboard-open-target-{index}"))
        .tooltip(if targets.len() == 1 {
            single_target_tooltip
        } else {
            "选择并打开"
        })
        .accessibility_label(single_target_tooltip)
        .icon(MaterialIcon::OpenInNew);
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
            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _window, cx| {
                let theme = Theme::active(cx);
                targets.clone().into_iter().fold(
                    menu.min_w(px(180.0)).max_w(px(320.0)),
                    |menu, target| {
                        let handler = Rc::clone(&handler);
                        let label = format!(
                            "{}  {}",
                            open_target_menu_prefix(&target.kind),
                            compact_target_label(&target.value)
                        );
                        let icon = match target.kind {
                            ClipboardOpenTargetKind::Web => MaterialIcon::Language,
                            ClipboardOpenTargetKind::Directory => MaterialIcon::Folder,
                            ClipboardOpenTargetKind::File => MaterialIcon::OpenInNew,
                            ClipboardOpenTargetKind::RegistryKey => MaterialIcon::Settings,
                            ClipboardOpenTargetKind::SystemTool => MaterialIcon::Terminal,
                        };
                        menu.item(
                            context_menu_item(
                                theme,
                                "clipboard-open-target-menu-item",
                                label,
                                icon,
                                MenuTone::Normal,
                            )
                            .on_click(move |event, window, cx| {
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
    _theme: Theme,
    index: usize,
    icon: MaterialIcon,
    tooltip: &'static str,
    action: ClipboardItemAction,
    handler: HistoryManagementHandler,
    active: bool,
) -> impl IntoElement {
    let key = match &action {
        ClipboardItemAction::Edit => "edit",
        ClipboardItemAction::TogglePinned => "pin",
        ClipboardItemAction::ToggleFavorite => "favorite",
        ClipboardItemAction::Delete => "delete",
        ClipboardItemAction::ToggleCategorySource { .. } => "category",
        ClipboardItemAction::RunExtension { .. } => "extension",
    };
    Button::new(gpui_kit::SharedString::from(format!(
        "clipboard-item-action-{index}-{key}"
    )))
    .debug_selector(move || format!("clipboard-item-action-{index}-{key}"))
    .ghost()
    .xsmall()
    .selected(active)
    .accessibility_label(tooltip)
    .tooltip(tooltip)
    .icon(icon)
    .on_click(move |event, window, cx| {
        cx.stop_propagation();
        handler(action.clone(), event, window, cx);
    })
}
fn state_icon(theme: Theme, icon: MaterialIcon, tooltip: &'static str) -> impl IntoElement {
    div()
        .id(gpui_kit::SharedString::from(format!(
            "clipboard-state-icon-{tooltip}"
        )))
        .debug_selector(move || format!("clipboard-state-{tooltip}"))
        .map(|indicator| {
            #[cfg(test)]
            let indicator = indicator.child(feedback_tests::paint_probe(tooltip));
            indicator
        })
        .size(px(16.0))
        .flex_none()
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

fn text_content(
    theme: Theme,
    entry: &ClipboardItem,
    character_count: Option<usize>,
    _on_action: HistoryActionHandler,
    _modifier_actions: ClipboardModifierActions,
    maximum_height: f32,
) -> gpui_kit::AnyElement {
    let character_count = character_count.unwrap_or_else(|| entry.text.chars().count());
    let source: String = entry
        .text
        .chars()
        .take(nxtools_clipboard::CLIPBOARD_SUMMARY_CHARACTERS)
        .collect();
    div()
        .debug_selector(|| "clipboard-history-content".into())
        .min_w_0()
        .min_h(px(TEXT_CONTENT_HEIGHT))
        .max_h(px(maximum_height))
        .flex_shrink_0()
        .overflow_hidden()
        .flex()
        .items_start()
        .gap(px(8.0))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .overflow_hidden()
                .child(HistoryText {
                    id: entry.id,
                    source,
                }),
        )
        .child(metric_chip(theme, format!("{character_count} 字符")))
        .into_any_element()
}

#[cfg(test)]
thread_local! {
    pub(super) static RENDERED_MARKDOWN: std::cell::RefCell<std::collections::HashMap<i64, gpui_kit::WeakEntity<super::markdown_document::MarkdownDocument>>> = std::cell::RefCell::default();
}
/// Scoped to the mounted virtual row, never a full-payload cache.
struct HistoryMarkdown {
    source: String,
    state: Option<Entity<super::markdown_document::MarkdownDocument>>,
}
impl HistoryMarkdown {
    fn new(source: String, window: &mut Window, cx: &mut gpui_kit::Context<Self>) -> Self {
        let state = super::markdown_document::document(
            &source,
            super::markdown_document::automatic_mode(&source),
            window,
            cx,
        );
        Self { source, state }
    }
}

#[derive(gpui_kit::IntoElement)]
struct HistoryText {
    id: i64,
    source: String,
}
impl RenderOnce for HistoryText {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let source = self.source;
        let cache = window.use_keyed_state(
            SharedString::from(format!("clipboard-row-markdown-{}", self.id)),
            cx,
            |window, cx| HistoryMarkdown::new(source.clone(), window, cx),
        );
        if cache.read(cx).source != source {
            cache.update(cx, |cached, cx| {
                *cached = HistoryMarkdown::new(source.clone(), window, cx)
            });
        }
        let state = cache.read(cx).state.clone();
        #[cfg(test)]
        if let Some(state) = &state {
            RENDERED_MARKDOWN.with_borrow_mut(|rows| {
                rows.retain(|_, state| state.upgrade().is_some());
                rows.insert(self.id, state.downgrade());
            });
        }
        match state {
            Some(state) => div()
                .debug_selector(|| "clipboard-history-markdown".into())
                .w_full()
                .overflow_hidden()
                .child(state)
                .into_any_element(),
            None => Label::new(source).text_sm().truncate().into_any_element(),
        }
    }
}

fn files_content(theme: Theme, entry: &ClipboardItem) -> gpui_kit::AnyElement {
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
        .debug_selector(|| "clipboard-history-content".into())
        .min_w_0()
        .h(px(TEXT_CONTENT_HEIGHT))
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(Label::new(preview).min_w_0().flex_1().text_sm().truncate())
        .child(metric_chip(theme, format!("{} 项", entry.file_paths.len())))
        .into_any_element()
}

fn image_content(
    theme: Theme,
    entry: &ClipboardItem,
    available_image_width: f32,
    list_image: Option<PathBuf>,
) -> gpui_kit::AnyElement {
    let preview_height =
        image_list_preview_height(entry.image_width, entry.image_height, available_image_width);
    let image_path = list_image.unwrap_or_else(|| PathBuf::from(&entry.image_path));
    let image_exists = !image_path.as_os_str().is_empty();
    div()
        .w_full()
        .h(px(preview_height + 8.0))
        .p(px(4.0))
        .rounded(px(RADIUS_CONTROL))
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
                    .text_size(px(TEXT_BODY))
                    .line_height(px(LINE_HEIGHT_BODY))
                    .text_color(theme.text_secondary)
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
    kind: ClipboardKind,
    color: Option<[u8; 3]>,
) -> impl IntoElement {
    let icon = if process_icon_path.is_empty() {
        Icon::new(match kind {
            ClipboardKind::Text => MaterialIcon::TextFields,
            ClipboardKind::Image => MaterialIcon::Image,
            ClipboardKind::Files => MaterialIcon::Folder,
        })
        .size(px(16.0))
        .text_color(theme.text_secondary)
        .into_any_element()
    } else {
        img(PathBuf::from(process_icon_path))
            .size(px(16.0))
            .object_fit(ObjectFit::Contain)
            .into_any_element()
    };
    let tooltip_source = source.clone();
    div()
        .id(("clipboard-source", index))
        .debug_selector(move || format!("clipboard-source-{index}"))
        .flex_1()
        .min_w_0()
        .h(px(METADATA_HEIGHT))
        .flex()
        .items_center()
        .gap(px(4.0))
        .overflow_hidden()
        .tooltip(move |window, cx| Tooltip::new(tooltip_source.clone()).build(window, cx))
        .child(SourceIconTint {
            color,
            content: div()
                .size_full()
                .flex()
                .items_center()
                .gap(px(4.0))
                .px_1()
                .child(div().size(px(16.0)).flex_none().child(icon))
                .child(
                    div()
                        .debug_selector(move || format!("clipboard-source-name-{index}"))
                        .min_w_0()
                        .flex_1()
                        .child(
                            Label::new(source)
                                .text_xs()
                                .text_color(theme.text_secondary)
                                .truncate(),
                        ),
                )
                .into_any_element(),
        })
}

#[derive(gpui_kit::IntoElement)]
struct SourceIconTint {
    color: Option<[u8; 3]>,
    content: AnyElement,
}
impl RenderOnce for SourceIconTint {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let color = self
            .color
            .map(|[r, g, b]| {
                gpui_kit::rgb((u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b))
                    .opacity(0.36)
            })
            .unwrap_or_else(|| {
                gpui_kit::component::Theme::global(cx)
                    .colors
                    .muted_foreground
                    .to_rgb()
                    .opacity(0.22)
            });
        div()
            .w_full()
            .h(px(METADATA_HEIGHT))
            .rounded_sm()
            .overflow_hidden()
            .bg(gpui_kit::linear_gradient(
                90.0,
                gpui_kit::linear_color_stop(color, 0.0),
                gpui_kit::linear_color_stop(color.opacity(0.0), 1.0),
            ))
            .child(self.content)
    }
}

fn usage_badge(_theme: Theme, text: String) -> impl IntoElement {
    Label::new(text).text_xs().min_w(px(20.0)).truncate()
}

fn time_label(_theme: Theme, text: String) -> impl IntoElement {
    Label::new(text).text_xs().max_w(px(92.0)).px_1().truncate()
}

fn metric_chip(_theme: Theme, text: String) -> impl IntoElement {
    Label::new(text).text_xs()
}

pub(super) fn row_card_height(entry: &ClipboardItem, available_image_width: f32) -> f32 {
    if entry.kind == ClipboardKind::Image {
        image_list_preview_height(entry.image_width, entry.image_height, available_image_width)
            + IMAGE_ROW_CHROME_HEIGHT
    } else if entry.kind == ClipboardKind::Text {
        // Only a fallback for rows not measured by ListState yet, never a rendered size.
        TEXT_ROW_HEIGHT
    } else {
        TEXT_ROW_HEIGHT
    }
}

fn image_list_preview_height(width: u32, height: u32, available_width: f32) -> f32 {
    image_preview_height_with_cap(width, height, available_width, IMAGE_LIST_MAX_HEIGHT)
}
#[cfg(test)]
fn image_preview_height(width: u32, height: u32, available_width: f32) -> f32 {
    image_preview_height_with_cap(width, height, available_width, IMAGE_PREVIEW_MAX_HEIGHT)
}

fn image_preview_height_with_cap(
    width: u32,
    height: u32,
    available_width: f32,
    max_height: f32,
) -> f32 {
    if width == 0 || height == 0 || available_width <= 0.0 {
        return IMAGE_PREVIEW_FALLBACK_HEIGHT.min(max_height);
    }
    let scale = 1.0_f32
        .min(available_width / width as f32)
        .min(max_height / height as f32);
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
    fn row_metrics_keep_large_preview_and_compact_list_caps_separate() {
        assert_eq!(IMAGE_PREVIEW_MAX_HEIGHT, 270.0);
        assert_eq!(IMAGE_LIST_MAX_HEIGHT, 144.0);
        assert_eq!(image_preview_height(1_000, 2_000, 246.0), 270.0);
        assert_eq!(image_list_preview_height(1_000, 2_000, 246.0), 144.0);
    }

    #[test]
    fn row_metrics_match_the_reference_panel() {
        assert_eq!(TEXT_ROW_HEIGHT, 62.0);
        assert_eq!(ROW_GAP, 8.0);
        assert_eq!(IMAGE_ROW_CHROME_HEIGHT, 46.0);
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
    fn history_text_retains_only_compact_minimum_not_fixed_multiline_height() {
        assert_eq!(TEXT_CONTENT_HEIGHT, 24.0);
        assert_eq!(TEXT_ROW_HEIGHT, 62.0);
    }

    #[test]
    fn history_rows_keep_multiple_hover_actions() {
        let source = include_str!("history.rs");
        let row = source
            .split("pub fn history_row(")
            .nth(1)
            .and_then(|source| source.split("fn open_targets_button(").next())
            .expect("history row implementation");
        assert!(row.contains("open_targets_button("));
        assert!(row.contains("management_button("));
    }
    #[test]
    fn history_rows_keep_official_states_with_requested_cell_separation() {
        // User requests separate bordered cells; official selected/hover states remain.
        let source = include_str!("history.rs");
        let row = source
            .split("pub fn history_row(")
            .nth(1)
            .and_then(|source| source.split("fn open_targets_button(").next())
            .expect("production row");
        assert!(row.contains("ListItem::new("));
        assert!(row.contains(".selected(selected)"));
        assert!(!row.contains("compact_result_row("));
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

#[cfg(test)]
#[path = "history_menu_tests.rs"]
mod menu_tests;

#[cfg(test)]
#[path = "history_kit_migration_tests.rs"]
mod kit_migration_tests;

#[cfg(test)]
#[path = "history_feedback_tests.rs"]
mod feedback_tests;
