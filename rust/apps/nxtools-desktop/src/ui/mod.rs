mod history;

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use gpui::{
    App, AppContext, Bounds, Context, Entity, Focusable, ListAlignment, ListState, Pixels, Point,
    Render, ResizeEdge, ScrollHandle, Subscription, WeakEntity, Window, div, list, point,
    prelude::*, px, size,
};
use gpui_component::Icon;
use gpui_component::PixelsExt;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::{ContextMenuExt, PopupMenuItem};
use gpui_component::tooltip::Tooltip;
use nxtools_clipboard::{
    BUILTIN_ALL_CATEGORY_ID, ClipboardAction, ClipboardCategory, ClipboardCategoryKind,
    ClipboardHistoryError, ClipboardItem, ClipboardKind,
};
use nxtools_protocol::{ClipboardPanelRequest, PlatformCommandResult};
use tracing::{error, info};

use crate::clipboard::ClipboardApplication;
use crate::icons::MaterialIcon;
use crate::theme::Theme;
use history::{CategorySourceMenu, CategorySourceOption, ClipboardItemAction, history_row};

pub(crate) const PANEL_WIDTH: f32 = 300.0;
pub(crate) const PANEL_HEIGHT: f32 = 440.0;

#[derive(Debug, Clone, PartialEq, Eq)]
enum PanelStatus {
    Ready(String),
    Notice(String),
    Pending(&'static str),
    Error(String),
}

#[derive(Debug, Clone)]
struct CategoryEditorState {
    category_id: Option<String>,
}

#[derive(Clone)]
struct CategoryDrag {
    category: ClipboardCategory,
    position: Point<Pixels>,
}

impl CategoryDrag {
    fn at(mut self, position: Point<Pixels>) -> Self {
        self.position = position;
        self
    }
}

impl Render for CategoryDrag {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .pl(self.position.x - px(36.0))
            .pt(self.position.y - px(13.0))
            .child(
                div()
                    .h(px(26.0))
                    .px_2()
                    .rounded(px(5.0))
                    .border_1()
                    .border_color(gpui::rgb(0x38d6c7))
                    .bg(gpui::rgb(0x292929))
                    .shadow_md()
                    .text_xs()
                    .text_color(gpui::rgb(0xddfbf7))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(Icon::new(category_icon(self.category.kind)).size(px(12.0)))
                    .child(self.category.name.clone()),
            )
    }
}

fn category_icon(kind: ClipboardCategoryKind) -> MaterialIcon {
    match kind {
        ClipboardCategoryKind::All => MaterialIcon::Dashboard,
        ClipboardCategoryKind::Favorite => MaterialIcon::Star,
        ClipboardCategoryKind::Text => MaterialIcon::TextFields,
        ClipboardCategoryKind::Image => MaterialIcon::Image,
        ClipboardCategoryKind::Files | ClipboardCategoryKind::Custom => MaterialIcon::Folder,
    }
}

impl PanelStatus {
    fn label(&self) -> &str {
        match self {
            Self::Ready(label) | Self::Notice(label) | Self::Error(label) => label,
            Self::Pending(label) => label,
        }
    }

    fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    fn should_render(&self) -> bool {
        !matches!(self, Self::Ready(_))
    }
}

#[cfg(windows)]
pub(crate) fn native_window_handle(window: &Window) -> Option<i64> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as i64),
        _ => None,
    }
}

#[cfg(not(windows))]
pub(crate) fn native_window_handle(_window: &Window) -> Option<i64> {
    None
}

pub(crate) struct ClipboardPanel {
    theme: Theme,
    application: WeakEntity<ClipboardApplication>,
    target: nxtools_protocol::ForegroundContext,
    entries: Vec<ClipboardItem>,
    status: PanelStatus,
    pending_action: Option<(uuid::Uuid, ClipboardAction)>,
    is_open: bool,
    categories: Vec<ClipboardCategory>,
    category_sources: HashMap<String, HashSet<String>>,
    active_category_id: String,
    category_scroll: ScrollHandle,
    category_editor: Option<CategoryEditorState>,
    active_modifier_action: Option<ClipboardAction>,
    list_scroll: ListState,
    list_is_scrolled: bool,
    list_width: f32,
    search_input: Entity<InputState>,
    edit_input: Entity<InputState>,
    category_name_input: Entity<InputState>,
    editing_item_id: Option<i64>,
    last_deleted_item_id: Option<i64>,
    _subscriptions: Vec<Subscription>,
}

impl ClipboardPanel {
    pub(crate) fn new(
        entries: Vec<ClipboardItem>,
        categories: Vec<ClipboardCategory>,
        target: nxtools_protocol::ForegroundContext,
        application: WeakEntity<ClipboardApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("搜索剪贴板历史"));
        search_input.focus_handle(cx).focus(window);
        let edit_input = cx.new(|cx| {
            InputState::new(window, cx)
                .auto_grow(5, 12)
                .placeholder("输入剪贴板文本")
        });
        let category_name_input = cx.new(|cx| InputState::new(window, cx).placeholder("分类名称"));

        let search_subscription = cx.subscribe_in(
            &search_input,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    let query = state.read(cx).value().to_string();
                    this.reload_entries(&query, cx);
                    cx.notify();
                }
            },
        );
        let activation_subscription = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() && this.pending_action.is_none() {
                this.close(window, cx);
            }
        });
        let keystroke_subscription = cx.observe_keystrokes(|this, event, window, cx| {
            if event.keystroke.key.eq_ignore_ascii_case("escape") {
                if this.category_editor.is_some() {
                    this.cancel_category_editor(window, cx);
                } else if this.editing_item_id.is_some() {
                    this.cancel_edit(window, cx);
                } else {
                    this.close(window, cx);
                }
            } else if this.category_editor.is_some()
                && event.keystroke.key.eq_ignore_ascii_case("enter")
                && event.keystroke.modifiers.control
            {
                this.save_category_editor(window, cx);
            } else if this.editing_item_id.is_some()
                && event.keystroke.key.eq_ignore_ascii_case("enter")
                && event.keystroke.modifiers.control
            {
                this.save_edit(window, cx);
            }
        });

        let mut status = if target.process_name.is_empty() {
            PanelStatus::Ready("选择一条记录进行粘贴".into())
        } else {
            PanelStatus::Ready(format!("将粘贴到 {}", target.process_name))
        };
        let list_scroll = ListState::new(entries.len(), ListAlignment::Top, px(180.0));
        let panel = cx.entity().downgrade();
        list_scroll.set_scroll_handler(move |event, _window, app| {
            let _ = panel.update(app, |panel, cx| {
                if panel.list_is_scrolled != event.is_scrolled {
                    panel.list_is_scrolled = event.is_scrolled;
                    cx.notify();
                }
            });
        });
        let category_sources =
            match Self::load_category_source_snapshot(&application, &categories, cx) {
                Ok(sources) => sources,
                Err(reason) => {
                    error!(%reason, "failed to load clipboard category sources");
                    status = PanelStatus::Error("分类来源加载失败，请查看日志".into());
                    HashMap::new()
                }
            };
        let active_category_id = categories
            .iter()
            .find(|category| category.id == BUILTIN_ALL_CATEGORY_ID)
            .or_else(|| categories.first())
            .map(|category| category.id.clone())
            .unwrap_or_else(|| BUILTIN_ALL_CATEGORY_ID.into());
        Self {
            theme: Theme::zed_dark(),
            application,
            target,
            entries,
            status,
            pending_action: None,
            is_open: true,
            categories,
            category_sources,
            active_category_id,
            category_scroll: ScrollHandle::new(),
            category_editor: None,
            active_modifier_action: None,
            list_scroll,
            list_is_scrolled: false,
            list_width: 0.0,
            search_input,
            edit_input,
            category_name_input,
            editing_item_id: None,
            last_deleted_item_id: None,
            _subscriptions: vec![
                search_subscription,
                activation_subscription,
                keystroke_subscription,
            ],
        }
    }

    fn load_category_source_snapshot(
        application: &WeakEntity<ClipboardApplication>,
        categories: &[ClipboardCategory],
        cx: &mut Context<Self>,
    ) -> Result<HashMap<String, HashSet<String>>, String> {
        categories
            .iter()
            .filter(|category| category.kind == ClipboardCategoryKind::Custom)
            .map(|category| {
                let sources = application
                    .update(cx, |application, _| {
                        application.category_sources(&category.id)
                    })
                    .map_err(|error| format!("clipboard application unavailable: {error}"))?
                    .map_err(|error| error.to_string())?
                    .into_iter()
                    .map(|source| source.to_lowercase())
                    .collect();
                Ok((category.id.clone(), sources))
            })
            .collect()
    }

    fn reload_entries(&mut self, query: &str, cx: &mut Context<Self>) {
        let category_id = self.active_category_id.clone();
        match self.application.update(cx, |application, _| {
            application.query_entries(query, &category_id)
        }) {
            Ok(Ok(entries)) => {
                self.entries = entries;
                self.list_scroll.reset(self.entries.len());
                self.list_is_scrolled = false;
            }
            Ok(Err(error)) => {
                error!(%error, "failed to query clipboard popup history");
                self.entries.clear();
                self.status = PanelStatus::Error("查询失败，请查看日志".into());
            }
            Err(error) => {
                error!(%error, "clipboard application became unavailable");
                self.entries.clear();
                self.status = PanelStatus::Error("剪贴板核心不可用".into());
            }
        }
    }

    fn select_category(&mut self, category_id: String, cx: &mut Context<Self>) {
        if !self
            .categories
            .iter()
            .any(|category| category.id == category_id)
        {
            return;
        }
        self.active_category_id = category_id;
        let query = self.search_input.read(cx).value().to_string();
        self.reload_entries(&query, cx);
        cx.notify();
    }

    fn active_category(&self) -> Option<&ClipboardCategory> {
        self.categories
            .iter()
            .find(|category| category.id == self.active_category_id)
    }

    fn begin_category_editor(
        &mut self,
        category: Option<&ClipboardCategory>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let value = category
            .map(|category| category.name.clone())
            .unwrap_or_default();
        self.editing_item_id = None;
        self.category_editor = Some(CategoryEditorState {
            category_id: category.map(|category| category.id.clone()),
        });
        self.category_name_input
            .update(cx, |state, cx| state.set_value(value, window, cx));
        self.category_name_input.focus_handle(cx).focus(window);
        cx.notify();
    }

    fn cancel_category_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.category_editor = None;
        self.search_input.focus_handle(cx).focus(window);
        cx.notify();
    }

    fn save_category_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.category_editor.clone() else {
            return;
        };
        let name = self.category_name_input.read(cx).value().to_string();
        let result = if let Some(category_id) = editor.category_id.as_deref() {
            self.application.update(cx, |application, _| {
                application.rename_category(category_id, &name)
            })
        } else {
            self.application
                .update(cx, |application, _| application.create_category(&name))
        };
        if let Some(category) = self.resolve_item_action("保存分类", result, cx) {
            self.category_editor = None;
            self.reload_categories(cx);
            self.active_category_id = category.id;
            self.refresh_entries(cx);
            self.search_input.focus_handle(cx).focus(window);
            self.status = PanelStatus::Notice("分类已保存".into());
        }
        cx.notify();
    }

    fn delete_category(&mut self, category_id: &str, cx: &mut Context<Self>) {
        let result = self.application.update(cx, |application, _| {
            application.delete_category(category_id)
        });
        if self.resolve_item_action("删除分类", result, cx).is_some() {
            if self.active_category_id == category_id {
                self.active_category_id = BUILTIN_ALL_CATEGORY_ID.into();
            }
            self.reload_categories(cx);
            self.refresh_entries(cx);
            self.status = PanelStatus::Notice("分类已删除".into());
        }
        cx.notify();
    }

    fn reorder_category(
        &mut self,
        source_id: &str,
        target_id: Option<&str>,
        cx: &mut Context<Self>,
    ) {
        if target_id == Some(source_id) {
            return;
        }
        let current_ids = self
            .categories
            .iter()
            .map(|category| category.id.clone())
            .collect::<Vec<_>>();
        let Some(ordered_ids) = reordered_category_ids(&current_ids, source_id, target_id) else {
            return;
        };
        let result = self.application.update(cx, |application, _| {
            application.reorder_categories(&ordered_ids)
        });
        if let Some(categories) = self.resolve_item_action("分类排序", result, cx) {
            self.categories = categories;
        }
        cx.notify();
    }

    fn reload_categories(&mut self, cx: &mut Context<Self>) {
        let result = self
            .application
            .update(cx, |application, _| application.categories());
        let Some(categories) = self.resolve_item_action("加载分类", result, cx) else {
            return;
        };
        let category_sources =
            match Self::load_category_source_snapshot(&self.application, &categories, cx) {
                Ok(sources) => sources,
                Err(reason) => {
                    error!(%reason, "failed to reload clipboard category sources");
                    self.status = PanelStatus::Error("分类来源加载失败，请查看日志".into());
                    return;
                }
            };
        self.categories = categories;
        self.category_sources = category_sources;
    }

    fn refresh_entries(&mut self, cx: &mut Context<Self>) {
        let query = self.search_input.read(cx).value().to_string();
        self.reload_entries(&query, cx);
    }

    fn handle_item_action(
        &mut self,
        entry: &ClipboardItem,
        action: ClipboardItemAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            ClipboardItemAction::Edit => self.begin_edit(entry, window, cx),
            ClipboardItemAction::TogglePinned => {
                let result = self.application.update(cx, |application, _| {
                    application.set_item_pinned(entry.id, !entry.is_pinned)
                });
                if self.resolve_item_action("置顶", result, cx).is_some() {
                    self.status = PanelStatus::Notice(if entry.is_pinned {
                        "已取消置顶".into()
                    } else {
                        "已置顶".into()
                    });
                    self.refresh_entries(cx);
                }
            }
            ClipboardItemAction::ToggleFavorite => {
                let result = self.application.update(cx, |application, _| {
                    application.set_item_favorite(entry.id, !entry.is_favorite)
                });
                if self.resolve_item_action("收藏", result, cx).is_some() {
                    self.status = PanelStatus::Notice(if entry.is_favorite {
                        "已取消收藏".into()
                    } else {
                        "已收藏".into()
                    });
                    self.refresh_entries(cx);
                }
            }
            ClipboardItemAction::Delete => {
                let result = self
                    .application
                    .update(cx, |application, _| application.delete_item(entry.id));
                if self.resolve_item_action("删除", result, cx).is_some() {
                    self.last_deleted_item_id = Some(entry.id);
                    self.status = PanelStatus::Notice("已删除，可在底部撤销".into());
                    self.refresh_entries(cx);
                }
            }
            ClipboardItemAction::ToggleCategorySource {
                category_id,
                remove,
            } => {
                let process_name = entry.source.process_name.trim();
                if process_name.is_empty() {
                    self.status = PanelStatus::Error("该条目没有可用的来源程序".into());
                    cx.notify();
                    return;
                }
                let result = if remove {
                    self.application.update(cx, |application, _| {
                        application.remove_category_source(&category_id, process_name)
                    })
                } else {
                    self.application.update(cx, |application, _| {
                        application.add_category_source(&category_id, process_name)
                    })
                };
                if let Some(sources) = self.resolve_item_action("更新来源分类", result, cx) {
                    self.category_sources.insert(
                        category_id.clone(),
                        sources
                            .into_iter()
                            .map(|source| source.to_lowercase())
                            .collect(),
                    );
                    if self.active_category_id == category_id {
                        self.refresh_entries(cx);
                    }
                    self.status = PanelStatus::Notice(if remove {
                        "已从分类移除来源".into()
                    } else {
                        "已将来源加入分类".into()
                    });
                }
            }
        }
        cx.notify();
    }

    fn begin_edit(&mut self, entry: &ClipboardItem, window: &mut Window, cx: &mut Context<Self>) {
        if entry.kind != ClipboardKind::Text {
            self.status = PanelStatus::Error("只有文本条目可以编辑".into());
            cx.notify();
            return;
        }
        self.category_editor = None;
        self.editing_item_id = Some(entry.id);
        self.edit_input.update(cx, |state, cx| {
            state.set_value(entry.text.clone(), window, cx)
        });
        self.edit_input.focus_handle(cx).focus(window);
        cx.notify();
    }

    fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.editing_item_id = None;
        self.search_input.focus_handle(cx).focus(window);
        cx.notify();
    }

    fn save_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item_id) = self.editing_item_id else {
            return;
        };
        let text = self.edit_input.read(cx).value().to_string();
        let result = self.application.update(cx, |application, _| {
            application.update_item_text(item_id, &text)
        });
        if self.resolve_item_action("保存", result, cx).is_some() {
            self.editing_item_id = None;
            self.status = PanelStatus::Notice("文本已保存为纯文本".into());
            self.refresh_entries(cx);
            self.search_input.focus_handle(cx).focus(window);
        }
        cx.notify();
    }

    fn restore_last_deleted(&mut self, cx: &mut Context<Self>) {
        let Some(item_id) = self.last_deleted_item_id else {
            return;
        };
        let result = self
            .application
            .update(cx, |application, _| application.restore_item(item_id));
        if self.resolve_item_action("撤销删除", result, cx).is_some() {
            self.last_deleted_item_id = None;
            self.status = PanelStatus::Notice("已撤销删除".into());
            self.refresh_entries(cx);
        }
        cx.notify();
    }

    fn resolve_item_action<T, E>(
        &mut self,
        action: &str,
        result: Result<Result<T, ClipboardHistoryError>, E>,
        cx: &mut Context<Self>,
    ) -> Option<T>
    where
        E: std::fmt::Display,
    {
        match result {
            Ok(Ok(value)) => Some(value),
            Ok(Err(error)) => {
                error!(%error, %action, "clipboard item action failed");
                self.status = PanelStatus::Error(format!("{action}失败：{error}"));
                cx.notify();
                None
            }
            Err(error) => {
                error!(%error, %action, "clipboard application became unavailable");
                self.status = PanelStatus::Error(format!("{action}失败：剪贴板核心不可用"));
                cx.notify();
                None
            }
        }
    }

    fn activate_entry(
        &mut self,
        entry: &ClipboardItem,
        action: ClipboardAction,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.pending_action.is_some() {
            self.status = PanelStatus::Pending("正在等待剪贴板操作结果…");
            cx.notify();
            return;
        }
        if action == ClipboardAction::WithoutLineBreaks && entry.kind != ClipboardKind::Text {
            self.status = PanelStatus::Error("去换行仅适用于文本条目".into());
            cx.notify();
            return;
        }
        if action != ClipboardAction::CopyOnly && self.target.window_handle == 0 {
            self.status = PanelStatus::Error("原目标窗口已经不可用".into());
            cx.notify();
            return;
        }
        let target = self.target.clone();
        let entry = entry.clone();
        match self.application.update(cx, |application, _| {
            application.request_clipboard_action(&entry, &target, action)
        }) {
            Ok(Ok(command_id)) => {
                self.pending_action = Some((command_id, action));
                self.status = PanelStatus::Pending(if action == ClipboardAction::CopyOnly {
                    "正在复制…"
                } else {
                    "正在粘贴…"
                });
                cx.notify();
            }
            Ok(Err(error)) => {
                self.status = PanelStatus::Error(format!("无法粘贴：{error}"));
                cx.notify();
            }
            Err(error) => {
                error!(%error, "clipboard application became unavailable");
                self.status = PanelStatus::Error("无法粘贴：剪贴板核心不可用".into());
                cx.notify();
            }
        }
    }

    pub(crate) fn handle_command_result(
        &mut self,
        correlation_id: uuid::Uuid,
        result: PlatformCommandResult,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some((pending_id, action)) = self.pending_action else {
            return;
        };
        if pending_id != correlation_id {
            return;
        }
        self.pending_action = None;
        if result.success {
            info!(%correlation_id, ?action, "clipboard action completed");
            if action == ClipboardAction::CopyOnly {
                self.status = PanelStatus::Notice("已复制".into());
                window.activate_window();
                cx.notify();
            } else {
                self.close(window, cx);
            }
        } else {
            let reason = result.error.unwrap_or_else(|| "Windows 未返回原因".into());
            error!(%correlation_id, %reason, "paste command failed");
            self.status = PanelStatus::Error(format!("粘贴失败：{}", reason));
            window.activate_window();
            cx.notify();
        }
    }

    pub(crate) fn handle_hide_failure(
        &mut self,
        reason: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_open = true;
        self.status = PanelStatus::Error(format!("隐藏面板失败：{reason}"));
        window.activate_window();
        cx.notify();
    }

    pub(crate) fn handle_topmost_failure(
        &mut self,
        reason: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.status = PanelStatus::Error(format!("面板置顶失败：{reason}"));
        window.activate_window();
        cx.notify();
    }

    pub(crate) fn handle_foreground_changed(
        &mut self,
        popup_window_handle: i64,
        foreground_window_handle: i64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if should_hide_for_foreground_change(
            popup_window_handle,
            foreground_window_handle,
            self.pending_action.is_some(),
            self.is_open,
        ) {
            self.close(window, cx);
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.is_open {
            return;
        }
        let Some(window_handle) = native_window_handle(window) else {
            error!("cannot hide clipboard popup because its native window handle is unavailable");
            self.status = PanelStatus::Error("无法隐藏面板：窗口句柄不可用".into());
            cx.notify();
            return;
        };
        let popup_window = window.window_handle();
        self.is_open = false;
        let application = self.application.clone();
        let panel = cx.entity().downgrade();
        cx.defer(move |cx| {
            let result = application.update(cx, |application, _| {
                application.hide_popup(popup_window, window_handle)
            });
            let reason = match result {
                Ok(Ok(_)) => return,
                Ok(Err(error)) => {
                    error!(%error, "failed to request clipboard popup hide");
                    format!("无法隐藏面板：{error}")
                }
                Err(error) => {
                    error!(%error, "clipboard application became unavailable while hiding popup");
                    "无法隐藏面板：剪贴板核心不可用".into()
                }
            };
            let _ = panel.update(cx, |panel, cx| {
                panel.is_open = true;
                panel.status = PanelStatus::Error(reason);
                cx.notify();
            });
        });
    }
}

impl Render for ClipboardPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let entries_empty = self.entries.is_empty();
        let status_is_error = self.status.is_error();
        let status_should_render = self.status.should_render();
        let status_label = self.status.label().to_owned();
        let summary = self
            .active_category()
            .map(|category| category.name.clone())
            .unwrap_or_else(|| "全部".into());
        let modifier_action = self.active_modifier_action;
        let is_scrolled = self.list_is_scrolled;
        let is_editing = self.editing_item_id.is_some();
        let is_editing_category = self.category_editor.is_some();
        let category_editor_title = self
            .category_editor
            .as_ref()
            .map(|editor| {
                if editor.category_id.is_some() {
                    "重命名分类"
                } else {
                    "新建分类"
                }
            })
            .unwrap_or("新建分类");
        let can_undo_delete = self.last_deleted_item_id.is_some();
        let list_width = (window.viewport_size().width.as_f32() - 12.0).max(28.0);
        if self.list_width > 0.0 && (self.list_width - list_width).abs() > 0.5 {
            self.list_scroll.reset(self.entries.len());
            self.list_is_scrolled = false;
        }
        self.list_width = list_width;
        let available_image_width = (list_width - 24.0).max(28.0);
        let scroll_to_top = self.list_scroll.clone();
        let panel = cx.entity().downgrade();
        let categories = self.categories.clone();
        let active_category_id = self.active_category_id.clone();
        let category_panel = cx.entity().downgrade();
        let add_category_panel = cx.entity().downgrade();
        let category_scroll = self.category_scroll.clone();
        let modifier_panel = cx.entity().downgrade();
        let undo_panel = cx.entity().downgrade();
        let cancel_edit_panel = cx.entity().downgrade();
        let save_edit_panel = cx.entity().downgrade();
        let cancel_category_panel = cx.entity().downgrade();
        let save_category_panel = cx.entity().downgrade();
        div()
            .relative()
            .size_full()
            .p(px(6.0))
            .rounded(px(8.0))
            .border_1()
            .border_color(theme.border)
            .bg(theme.canvas)
            .shadow_lg()
            .text_color(theme.text)
            .flex()
            .flex_col()
            .gap(px(4.0))
            .on_modifiers_changed(move |event, _window, app| {
                let _ = modifier_panel.update(app, |panel, cx| {
                    let action = action_from_modifiers(event.modifiers);
                    if panel.active_modifier_action != action {
                        panel.active_modifier_action = action;
                        cx.notify();
                    }
                });
            })
            .child(
                div()
                    .id("clipboard-panel-header")
                    .h(px(24.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_default()
                    .on_mouse_down(gpui::MouseButton::Left, |_event, window, _cx| {
                        window.start_window_move();
                    })
                    .child(
                        div()
                            .text_sm()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("剪贴板"),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .truncate()
                            .child(summary),
                    )
                    .when_some(modifier_action, |header, action| {
                        header.child(
                            div()
                                .h(px(21.0))
                                .px_2()
                                .rounded(px(5.0))
                                .border_1()
                                .border_color(theme.accent)
                                .bg(theme.accent_soft)
                                .text_xs()
                                .text_color(theme.accent_text)
                                .flex()
                                .items_center()
                                .child(modifier_action_label(action)),
                        )
                    })
                    .when(status_should_render, |header| {
                        header.child(
                            div()
                                .max_w(px(150.0))
                                .h(px(21.0))
                                .px_2()
                                .rounded(px(5.0))
                                .border_1()
                                .border_color(if status_is_error {
                                    theme.danger
                                } else {
                                    theme.border
                                })
                                .bg(theme.surface_raised)
                                .text_xs()
                                .text_color(if status_is_error {
                                    theme.danger
                                } else {
                                    theme.text_secondary
                                })
                                .flex()
                                .items_center()
                                .truncate()
                                .child(status_label),
                        )
                    }),
            )
            .child(
                div()
                    .h(px(26.0))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("clipboard-category-scroll")
                            .min_w_0()
                            .flex_1()
                            .h(px(26.0))
                            .overflow_x_scroll()
                            .track_scroll(&category_scroll)
                            .on_scroll_wheel({
                                let scroll = category_scroll.clone();
                                move |event, _window, app| {
                                    app.stop_propagation();
                                    let delta = event.delta.pixel_delta(px(48.0));
                                    let maximum = scroll.max_offset().width.as_f32().max(0.0);
                                    let current = scroll.offset().x.as_f32();
                                    let next = resolve_category_wheel_offset(
                                        current,
                                        maximum,
                                        delta.x.as_f32(),
                                        delta.y.as_f32(),
                                    );
                                    scroll.set_offset(point(px(next), px(0.0)));
                                }
                            })
                            .child(
                                div()
                                    .h(px(26.0))
                                    .flex()
                                    .flex_none()
                                    .items_center()
                                    .gap_1()
                                    .children(categories.into_iter().map(|category| {
                                        category_tab(
                                            theme,
                                            category.clone(),
                                            category.id == active_category_id,
                                            category_panel.clone(),
                                        )
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .id("add-clipboard-category")
                            .size(px(24.0))
                            .rounded(px(5.0))
                            .cursor_pointer()
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_color(theme.text_secondary)
                            .hover(move |style| style.bg(theme.surface_hover))
                            .drag_over::<CategoryDrag>(move |style, _, _, _| {
                                style.bg(theme.accent_soft).text_color(theme.accent_text)
                            })
                            .on_drop({
                                let panel = add_category_panel.clone();
                                move |drag: &CategoryDrag, _window, app| {
                                    let _ = panel.update(app, |panel, cx| {
                                        panel.reorder_category(&drag.category.id, None, cx);
                                    });
                                }
                            })
                            .on_click(move |_event, window, app| {
                                let _ = add_category_panel.update(app, |panel, cx| {
                                    panel.begin_category_editor(None, window, cx);
                                });
                            })
                            .tooltip(|window, cx| Tooltip::new("新建分类").build(window, cx))
                            .child(Icon::new(MaterialIcon::Add).size(px(14.0))),
                    ),
            )
            .child(
                div()
                    .h(px(28.0))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .h(px(28.0))
                            .rounded(px(6.0))
                            .border_1()
                            .border_color(theme.border)
                            .bg(theme.surface)
                            .px_2()
                            .flex()
                            .items_center()
                            .child(
                                Icon::new(MaterialIcon::Search)
                                    .size(px(14.0))
                                    .text_color(theme.text_muted),
                            )
                            .child(
                                div().min_w_0().flex_1().text_color(theme.text).child(
                                    Input::new(&self.search_input)
                                        .appearance(false)
                                        .bordered(false)
                                        .focus_bordered(false),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.text_muted)
                            .flex()
                            .items_center()
                            .child(format!("{} 条", self.entries.len())),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .bg(theme.canvas)
                    .overflow_hidden()
                    .when(entries_empty, |list| {
                        list.child(
                            div()
                                .size_full()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_sm()
                                .text_color(theme.text_muted)
                                .child("暂无剪贴板记录"),
                        )
                    })
                    .when(!entries_empty, |container| {
                        let entries = self.entries.clone();
                        let custom_categories = self
                            .categories
                            .iter()
                            .filter(|category| category.kind == ClipboardCategoryKind::Custom)
                            .cloned()
                            .collect::<Vec<_>>();
                        let category_sources = self.category_sources.clone();
                        let history_panel = cx.entity().downgrade();
                        let theme = self.theme;
                        container.child(
                            list(self.list_scroll.clone(), move |index, _window, _app| {
                                let Some(entry) = entries.get(index).cloned() else {
                                    return div().into_any_element();
                                };
                                let action_entry = entry.clone();
                                let management_entry = entry.clone();
                                let process_name = entry.source.process_name.to_lowercase();
                                let category_hint = if custom_categories.is_empty() {
                                    Some("来源分类（请先新建分类）".to_owned())
                                } else if process_name.is_empty() {
                                    Some("来源分类（来源未知）".to_owned())
                                } else {
                                    None
                                };
                                let category_options = if category_hint.is_some() {
                                    Vec::new()
                                } else {
                                    custom_categories
                                        .iter()
                                        .map(|category| CategorySourceOption {
                                            category_id: category.id.clone(),
                                            name: category.name.clone(),
                                            included: category_sources
                                                .get(&category.id)
                                                .is_some_and(|sources| {
                                                    sources.contains(&process_name)
                                                }),
                                        })
                                        .collect()
                                };
                                let panel = history_panel.clone();
                                let on_action = Rc::new(
                                    move |action: ClipboardAction,
                                          _event: &gpui::ClickEvent,
                                          window: &mut Window,
                                          app: &mut App| {
                                        let _ = panel.update(app, |this, cx| {
                                            this.activate_entry(&action_entry, action, window, cx);
                                        });
                                    },
                                );
                                let management_panel = history_panel.clone();
                                let on_management = Rc::new(
                                    move |action: ClipboardItemAction,
                                          _event: &gpui::ClickEvent,
                                          window: &mut Window,
                                          app: &mut App| {
                                        let _ = management_panel.update(app, |this, cx| {
                                            this.handle_item_action(
                                                &management_entry,
                                                action,
                                                window,
                                                cx,
                                            );
                                        });
                                    },
                                );
                                history_row(
                                    theme,
                                    entry,
                                    index,
                                    available_image_width,
                                    CategorySourceMenu {
                                        options: category_options,
                                        unavailable_hint: category_hint,
                                    },
                                    on_action,
                                    on_management,
                                )
                                .into_any_element()
                            })
                            .w_full()
                            .h_full(),
                        )
                    })
                    .when(is_scrolled, |list| {
                        list.child(
                            div()
                                .absolute()
                                .bottom(px(if can_undo_delete { 44.0 } else { 8.0 }))
                                .left_0()
                                .right_0()
                                .flex()
                                .justify_center()
                                .child(
                                    div()
                                        .id("back-to-top")
                                        .h(px(28.0))
                                        .px_3()
                                        .rounded(px(5.0))
                                        .border_1()
                                        .border_color(theme.border)
                                        .bg(theme.surface_raised)
                                        .shadow_md()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(theme.surface_hover))
                                        .text_xs()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            Icon::new(MaterialIcon::ArrowUpward)
                                                .size(px(15.0))
                                                .text_color(theme.text_secondary),
                                        )
                                        .child("回到开头")
                                        .on_click(move |_event, _window, app| {
                                            scroll_to_top.scroll_to(Default::default());
                                            let _ = panel.update(app, |panel, cx| {
                                                panel.list_is_scrolled = false;
                                                cx.notify();
                                            });
                                        }),
                                ),
                        )
                    })
                    .when(can_undo_delete, |list| {
                        list.child(
                            div()
                                .absolute()
                                .bottom_2()
                                .left_0()
                                .right_0()
                                .flex()
                                .justify_center()
                                .child(
                                    div()
                                        .id("undo-clipboard-delete")
                                        .h(px(30.0))
                                        .px_3()
                                        .rounded(px(6.0))
                                        .border_1()
                                        .border_color(theme.border_active)
                                        .bg(theme.surface_raised)
                                        .shadow_md()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(theme.surface_hover))
                                        .text_xs()
                                        .text_color(theme.text)
                                        .flex()
                                        .items_center()
                                        .gap_1()
                                        .child(
                                            Icon::new(MaterialIcon::Undo)
                                                .size(px(14.0))
                                                .text_color(theme.accent),
                                        )
                                        .child("撤销删除")
                                        .on_click(move |_event, _window, app| {
                                            app.stop_propagation();
                                            let _ = undo_panel.update(app, |panel, cx| {
                                                panel.restore_last_deleted(cx);
                                            });
                                        }),
                                ),
                        )
                    }),
            )
            .when(is_editing, |panel| {
                panel.child(
                    div()
                        .absolute()
                        .top(px(36.0))
                        .left_2()
                        .right_2()
                        .bottom_2()
                        .p_3()
                        .rounded(px(7.0))
                        .border_1()
                        .border_color(theme.border_active)
                        .bg(theme.canvas)
                        .shadow_lg()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .text_sm()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child("编辑文本"),
                                )
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme.text_muted)
                                        .child("保存后统一为纯文本"),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .p_2()
                                .rounded(px(6.0))
                                .border_1()
                                .border_color(theme.border)
                                .bg(theme.surface)
                                .child(
                                    Input::new(&self.edit_input)
                                        .appearance(false)
                                        .bordered(false)
                                        .focus_bordered(false),
                                ),
                        )
                        .child(
                            div()
                                .h(px(30.0))
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .flex_1()
                                        .text_xs()
                                        .text_color(theme.text_muted)
                                        .child("Ctrl+Enter 保存 · Esc 取消"),
                                )
                                .child(dialog_button(
                                    theme,
                                    "取消",
                                    false,
                                    move |_event, window, app| {
                                        let _ = cancel_edit_panel.update(app, |panel, cx| {
                                            panel.cancel_edit(window, cx);
                                        });
                                    },
                                ))
                                .child(dialog_button(
                                    theme,
                                    "保存",
                                    true,
                                    move |_event, window, app| {
                                        let _ = save_edit_panel.update(app, |panel, cx| {
                                            panel.save_edit(window, cx);
                                        });
                                    },
                                )),
                        ),
                )
            })
            .when(is_editing_category, |panel| {
                panel.child(
                    div()
                        .absolute()
                        .top(px(104.0))
                        .left_3()
                        .right_3()
                        .p_3()
                        .rounded(px(7.0))
                        .border_1()
                        .border_color(theme.border_active)
                        .bg(theme.canvas)
                        .shadow_lg()
                        .flex()
                        .flex_col()
                        .gap_3()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(category_editor_title),
                        )
                        .child(
                            div()
                                .h(px(32.0))
                                .px_2()
                                .rounded(px(6.0))
                                .border_1()
                                .border_color(theme.border)
                                .bg(theme.surface)
                                .flex()
                                .items_center()
                                .child(
                                    Input::new(&self.category_name_input)
                                        .appearance(false)
                                        .bordered(false)
                                        .focus_bordered(false),
                                ),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.text_muted)
                                .child("在条目右键菜单中把来源程序加入分类"),
                        )
                        .child(
                            div()
                                .h(px(30.0))
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(div().flex_1())
                                .child(dialog_button(
                                    theme,
                                    "取消",
                                    false,
                                    move |_event, window, app| {
                                        let _ = cancel_category_panel.update(app, |panel, cx| {
                                            panel.cancel_category_editor(window, cx);
                                        });
                                    },
                                ))
                                .child(dialog_button(
                                    theme,
                                    "保存",
                                    true,
                                    move |_event, window, app| {
                                        let _ = save_category_panel.update(app, |panel, cx| {
                                            panel.save_category_editor(window, cx);
                                        });
                                    },
                                )),
                        ),
                )
            })
            .child(
                div()
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .size(px(16.0))
                    .cursor_e_resize()
                    .text_xs()
                    .text_color(theme.text_muted)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(MaterialIcon::OpenInFull)
                            .size(px(14.0))
                            .text_color(theme.text_muted),
                    )
                    .on_mouse_down(gpui::MouseButton::Left, |_event, window, _cx| {
                        window.start_window_resize(ResizeEdge::BottomRight);
                    }),
            )
    }
}

fn should_hide_for_foreground_change(
    popup_window_handle: i64,
    foreground_window_handle: i64,
    paste_pending: bool,
    is_open: bool,
) -> bool {
    is_open
        && !paste_pending
        && popup_window_handle != 0
        && foreground_window_handle != 0
        && foreground_window_handle != popup_window_handle
}

fn action_from_modifiers(modifiers: gpui::Modifiers) -> Option<ClipboardAction> {
    match (
        modifiers.control,
        modifiers.alt,
        modifiers.shift,
        modifiers.platform || modifiers.function,
    ) {
        (false, false, false, false) => None,
        (true, false, false, false) => Some(ClipboardAction::CopyOnly),
        (false, true, false, false) => Some(ClipboardAction::PlainText),
        (false, false, true, false) => Some(ClipboardAction::WithoutLineBreaks),
        _ => None,
    }
}

fn modifier_action_label(action: ClipboardAction) -> &'static str {
    match action {
        ClipboardAction::CopyOnly => "Ctrl · 仅复制",
        ClipboardAction::PlainText => "Alt · 纯文本",
        ClipboardAction::WithoutLineBreaks => "Shift · 去换行",
        ClipboardAction::OriginalPaste => "原格式粘贴",
    }
}

fn resolve_category_wheel_offset(current: f32, maximum: f32, delta_x: f32, delta_y: f32) -> f32 {
    if maximum <= 0.0 {
        return 0.0;
    }
    let delta = if delta_x.abs() > delta_y.abs() {
        delta_x
    } else {
        delta_y
    };
    (current + delta).clamp(-maximum, 0.0)
}

fn reordered_category_ids(
    current: &[String],
    source_id: &str,
    target_id: Option<&str>,
) -> Option<Vec<String>> {
    if target_id == Some(source_id) {
        return None;
    }
    let mut ordered = current.to_vec();
    let source_index = ordered.iter().position(|id| id == source_id)?;
    let source = ordered.remove(source_index);
    let target_index = target_id
        .and_then(|target_id| ordered.iter().position(|id| id == target_id))
        .unwrap_or(ordered.len());
    ordered.insert(target_index, source);
    Some(ordered)
}

fn category_tab(
    theme: Theme,
    category: ClipboardCategory,
    selected: bool,
    panel: WeakEntity<ClipboardPanel>,
) -> gpui::AnyElement {
    let label = category.name.clone();
    let tooltip_label = if category.kind == ClipboardCategoryKind::Custom {
        format!("{} · 拖动排序 · 右键管理", category.name)
    } else {
        format!("{} · 拖动排序", category.name)
    };
    let icon = category_icon(category.kind);
    let category_id = category.id.clone();
    let click_category_id = category.id.clone();
    let drop_target_id = category.id.clone();
    let edit_category = category.clone();
    let delete_category_id = category.id.clone();
    let drag = CategoryDrag {
        category: category.clone(),
        position: point(px(0.0), px(0.0)),
    };
    let is_custom = category.kind == ClipboardCategoryKind::Custom;
    let tab = div()
        .id(gpui::SharedString::from(format!("category-{category_id}")))
        .h(px(24.0))
        .px(px(7.0))
        .rounded(px(5.0))
        .border_1()
        .border_color(if selected {
            theme.accent
        } else {
            gpui::rgba(0x00000000)
        })
        .bg(if selected {
            theme.accent_soft
        } else {
            gpui::rgba(0x00000000)
        })
        .text_xs()
        .text_color(if selected {
            theme.accent_text
        } else {
            theme.text_secondary
        })
        .flex()
        .items_center()
        .gap_1()
        .cursor_pointer()
        .hover(move |style| style.bg(theme.surface_hover))
        .tooltip(move |window, cx| Tooltip::new(tooltip_label.clone()).build(window, cx))
        .on_click({
            let panel = panel.clone();
            move |_event, _window, app| {
                let category_id = click_category_id.clone();
                let _ = panel.update(app, |panel, cx| {
                    panel.select_category(category_id, cx);
                });
            }
        })
        .on_drag(drag, |drag: &CategoryDrag, position, _window, cx| {
            cx.new(|_| drag.clone().at(position))
        })
        .drag_over::<CategoryDrag>(move |style, drag, _, _| {
            if drag.category.id == drop_target_id {
                style
            } else {
                style.border_color(theme.accent).bg(theme.accent_soft)
            }
        })
        .on_drop({
            let panel = panel.clone();
            let target_id = category.id.clone();
            move |drag: &CategoryDrag, _window, app| {
                let _ = panel.update(app, |panel, cx| {
                    panel.reorder_category(&drag.category.id, Some(&target_id), cx);
                });
            }
        })
        .child(Icon::new(icon).size(px(12.0)).text_color(if selected {
            theme.accent_text
        } else {
            theme.text_secondary
        }))
        .child(label);
    if is_custom {
        tab.context_menu(move |menu, _window, _cx| {
            menu.item(PopupMenuItem::new("重命名").on_click({
                let panel = panel.clone();
                let category = edit_category.clone();
                move |_event, window, app| {
                    let _ = panel.update(app, |panel, cx| {
                        panel.begin_category_editor(Some(&category), window, cx);
                    });
                }
            }))
            .item(PopupMenuItem::new("删除分类").on_click({
                let panel = panel.clone();
                let category_id = delete_category_id.clone();
                move |_event, _window, app| {
                    let _ = panel.update(app, |panel, cx| {
                        panel.delete_category(&category_id, cx);
                    });
                }
            }))
        })
        .into_any_element()
    } else {
        tab.into_any_element()
    }
}

fn dialog_button(
    theme: Theme,
    label: &'static str,
    primary: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(gpui::SharedString::from(format!(
            "clipboard-dialog-button-{label}"
        )))
        .h(px(28.0))
        .px_3()
        .rounded(px(5.0))
        .border_1()
        .border_color(if primary { theme.accent } else { theme.border })
        .bg(if primary {
            theme.accent_soft
        } else {
            theme.surface_raised
        })
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(if primary {
            theme.accent_text
        } else {
            theme.text_secondary
        })
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .hover(move |style| style.bg(theme.surface_hover))
        .on_click(move |event, window, cx| {
            cx.stop_propagation();
            on_click(event, window, cx);
        })
        .child(label)
}

pub(crate) fn panel_bounds(
    request: &ClipboardPanelRequest,
    cx: &App,
) -> (Bounds<gpui::Pixels>, Option<gpui::DisplayId>) {
    let panel_size = size(px(PANEL_WIDTH), px(PANEL_HEIGHT));
    let displays = cx.displays();
    let display = displays
        .iter()
        .find(|display| u32::from(display.id()) == request.cursor_display_index)
        .cloned()
        .or_else(|| displays.first().cloned());
    let scale = (request.cursor_monitor_dpi.max(96) as f32) / 96.0;
    let cursor = point(
        px(request.cursor_x as f32 / scale),
        px(request.cursor_y as f32 / scale),
    );
    let Some(display) = display else {
        return (
            Bounds {
                origin: point(cursor.x + px(12.0), cursor.y + px(12.0)),
                size: panel_size,
            },
            None,
        );
    };

    let screen = display.bounds();
    let max_x = screen.right() - panel_size.width;
    let max_y = screen.bottom() - panel_size.height;
    let origin = point(
        clamp_pixels(cursor.x + px(12.0), screen.left(), max_x),
        clamp_pixels(cursor.y + px(12.0), screen.top(), max_y),
    );
    (
        Bounds {
            origin,
            size: panel_size,
        },
        Some(display.id()),
    )
}

fn clamp_pixels(value: gpui::Pixels, minimum: gpui::Pixels, maximum: gpui::Pixels) -> gpui::Pixels {
    if maximum < minimum || value < minimum {
        minimum
    } else if value > maximum {
        maximum
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_status_uses_typed_error_state() {
        assert!(!PanelStatus::Ready("ready".into()).is_error());
        assert!(!PanelStatus::Ready("ready".into()).should_render());
        assert!(!PanelStatus::Notice("copied".into()).is_error());
        assert!(PanelStatus::Notice("copied".into()).should_render());
        assert!(!PanelStatus::Pending("pending").is_error());
        assert!(PanelStatus::Pending("pending").should_render());
        assert!(PanelStatus::Error("failed".into()).is_error());
        assert!(PanelStatus::Error("failed".into()).should_render());
    }

    #[test]
    fn external_foreground_window_hides_open_popup_but_paste_activation_does_not() {
        assert!(should_hide_for_foreground_change(10, 20, false, true));
        assert!(!should_hide_for_foreground_change(10, 20, true, true));
        assert!(!should_hide_for_foreground_change(10, 10, false, true));
        assert!(!should_hide_for_foreground_change(10, 20, false, false));
    }

    #[test]
    fn panel_metrics_match_the_reference_window() {
        assert_eq!(PANEL_WIDTH, 300.0);
        assert_eq!(PANEL_HEIGHT, 440.0);
    }

    #[test]
    fn exactly_one_supported_modifier_selects_an_accelerated_action() {
        assert_eq!(
            action_from_modifiers(gpui::Modifiers {
                control: true,
                ..Default::default()
            }),
            Some(ClipboardAction::CopyOnly)
        );
        assert_eq!(
            action_from_modifiers(gpui::Modifiers {
                alt: true,
                ..Default::default()
            }),
            Some(ClipboardAction::PlainText)
        );
        assert_eq!(
            action_from_modifiers(gpui::Modifiers {
                shift: true,
                ..Default::default()
            }),
            Some(ClipboardAction::WithoutLineBreaks)
        );
        assert_eq!(
            action_from_modifiers(gpui::Modifiers {
                control: true,
                shift: true,
                ..Default::default()
            }),
            None
        );
    }

    #[test]
    fn category_wheel_is_horizontal_clamped_and_uses_vertical_wheel_input() {
        assert_eq!(resolve_category_wheel_offset(0.0, 120.0, 0.0, -48.0), -48.0);
        assert_eq!(
            resolve_category_wheel_offset(-100.0, 120.0, 0.0, -48.0),
            -120.0
        );
        assert_eq!(
            resolve_category_wheel_offset(-80.0, 120.0, 32.0, -10.0),
            -48.0
        );
        assert_eq!(resolve_category_wheel_offset(-20.0, 0.0, 0.0, -48.0), 0.0);
    }

    #[test]
    fn category_drop_reorders_before_target_or_at_end() {
        let ids = vec!["all".to_owned(), "favorite".to_owned(), "work".to_owned()];
        assert_eq!(
            reordered_category_ids(&ids, "all", Some("work")),
            Some(vec!["favorite".into(), "all".into(), "work".into()])
        );
        assert_eq!(
            reordered_category_ids(&ids, "favorite", None),
            Some(vec!["all".into(), "work".into(), "favorite".into()])
        );
        assert_eq!(reordered_category_ids(&ids, "missing", None), None);
    }
}
