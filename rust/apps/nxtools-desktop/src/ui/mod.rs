pub(crate) mod components;
mod history;
pub(crate) mod metrics;
pub(crate) mod motion;
mod preview;
pub(crate) mod typography;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui::{
    App, AppContext, Bounds, Context, Div, Entity, Focusable, ImageSource, ListAlignment,
    ListState, Pixels, Point, Render, ScrollHandle, Subscription, WeakEntity, Window, div, list,
    point, prelude::*, px, size,
};
use gpui_component::Icon;
use gpui_component::PixelsExt;
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::menu::ContextMenuExt;
use gpui_component::tooltip::Tooltip;
use nxtools_clipboard::{
    BUILTIN_ALL_CATEGORY_ID, ClipboardAction, ClipboardCategory, ClipboardCategoryKind,
    ClipboardHistoryError, ClipboardHistoryPage, ClipboardItem, ClipboardKind,
    ClipboardSearchScope,
};
use nxtools_core::{ClipboardClickAction, ClipboardClickModifier, ClipboardModifierActions};
use nxtools_protocol::{ClipboardPanelRequest, PlatformCommandResult};
use tracing::{error, info, warn};
use unicode_width::UnicodeWidthStr;

use crate::clipboard::ClipboardApplication;
use crate::icons::MaterialIcon;
use crate::theme::{Theme, UI_FONT_FAMILY};
use crate::ui::components::menu::{MenuTone, context_menu_item, context_menu_shell};
use crate::ui::components::state::empty_state;
use crate::ui::metrics::RADIUS_CONTROL;
use crate::ui::typography::{TEXT_BODY, TEXT_META};
use history::{
    CategorySourceMenu, CategorySourceOption, ClipboardItemAction, HistoryRowHandlers, history_row,
};
pub(crate) use preview::{
    ClipboardPreview, PREVIEW_APPEAR_DURATION, PreviewLayout, preview_layout,
    preview_media_surface, preview_shell,
};

const CATEGORY_GAP: f32 = 4.0;
const CATEGORY_ICON_WIDTH: f32 = 12.0;
const CATEGORY_ICON_TEXT_GAP: f32 = 4.0;
const CATEGORY_HORIZONTAL_PADDING: f32 = 14.0;
const CATEGORY_TEXT_CELL_WIDTH: f32 = 7.0;
const PANEL_CONTENT_INSET: f32 = 8.0;
const PANEL_SECTION_GAP: f32 = 6.0;
const POPUP_OPEN_GRACE: Duration = Duration::from_millis(180);
const HISTORY_PAGE_PREFETCH_ITEMS: usize = 8;
const IMAGE_RESOURCE_CACHE_CAPACITY: usize = 96;

fn schedule_entity_update<T, F>(entity: WeakEntity<T>, update: F, cx: &mut App)
where
    T: 'static,
    F: FnOnce(&mut T, &mut Context<T>) + 'static,
{
    cx.defer(move |cx| {
        let _ = entity.update(cx, update);
    });
}

fn panel_list_width(viewport_width: f32) -> f32 {
    (viewport_width - PANEL_CONTENT_INSET * 2.0).max(28.0)
}

fn inset_panel_section<T: gpui::Styled>(section: T) -> T {
    section.mx(px(PANEL_CONTENT_INSET))
}

fn panel_resize_hitbox(handle: Div) -> Div {
    handle.absolute().right_0().bottom_0().size(px(16.0))
}

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

#[derive(Debug)]
struct BoundedImageResourceCache {
    capacity: usize,
    entries: HashSet<PathBuf>,
    recency: VecDeque<PathBuf>,
}

impl BoundedImageResourceCache {
    fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: HashSet::new(),
            recency: VecDeque::new(),
        }
    }

    fn touch(&mut self, path: &Path) -> Option<PathBuf> {
        if path.as_os_str().is_empty() {
            return None;
        }
        if self.entries.contains(path) {
            if let Some(index) = self.recency.iter().position(|entry| entry == path) {
                self.recency.remove(index);
            }
            self.recency.push_back(path.to_path_buf());
            return None;
        }

        let path = path.to_path_buf();
        self.entries.insert(path.clone());
        self.recency.push_back(path);
        if self.entries.len() <= self.capacity {
            return None;
        }

        let evicted = self.recency.pop_front()?;
        self.entries.remove(&evicted);
        Some(evicted)
    }

    #[cfg(test)]
    fn len(&self) -> usize {
        self.entries.len()
    }
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
        let theme = Theme::zed_dark();
        div()
            .pl(self.position.x - px(36.0))
            .pt(self.position.y - px(13.0))
            .child(
                div()
                    .h(px(26.0))
                    .px_2()
                    .rounded(px(RADIUS_CONTROL))
                    .border_1()
                    .border_color(theme.border_active)
                    .bg(theme.surface_raised)
                    .shadow_md()
                    .text_size(px(TEXT_META))
                    .text_color(theme.accent_text)
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

#[cfg(all(windows, not(test)))]
pub(crate) fn native_window_handle(window: &Window) -> Option<i64> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let handle = HasWindowHandle::window_handle(window).ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get() as i64),
        _ => None,
    }
}

#[cfg(any(not(windows), test))]
pub(crate) fn native_window_handle(_window: &Window) -> Option<i64> {
    None
}

pub(crate) struct ClipboardPanel {
    theme: Theme,
    application: WeakEntity<ClipboardApplication>,
    target: nxtools_protocol::ForegroundContext,
    entries: Rc<Vec<ClipboardItem>>,
    total_entries: usize,
    has_more_entries: bool,
    page_request_pending: bool,
    status: PanelStatus,
    pending_action: Option<(uuid::Uuid, ClipboardAction, i64)>,
    is_open: bool,
    hide_suppressed_until: Instant,
    categories: Vec<ClipboardCategory>,
    category_sources: HashMap<String, HashSet<String>>,
    active_category_id: String,
    category_scroll: ScrollHandle,
    category_editor: Option<CategoryEditorState>,
    modifier_actions: ClipboardModifierActions,
    active_modifier_action: Option<(ClipboardClickModifier, ClipboardClickAction)>,
    list_scroll: ListState,
    image_resource_cache: Rc<RefCell<BoundedImageResourceCache>>,
    list_is_scrolled: bool,
    list_width: f32,
    search_input: Entity<InputState>,
    search_scope: ClipboardSearchScope,
    edit_input: Entity<InputState>,
    category_name_input: Entity<InputState>,
    editing_item_id: Option<i64>,
    last_deleted_item_id: Option<i64>,
    _subscriptions: Vec<Subscription>,
}

impl ClipboardPanel {
    pub(crate) fn new(
        page: ClipboardHistoryPage,
        categories: Vec<ClipboardCategory>,
        modifier_actions: ClipboardModifierActions,
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
        let size_application = application.clone();
        let size_subscription = cx.observe_window_bounds(window, move |_panel, window, cx| {
            let viewport = window.viewport_size();
            let _ = size_application.update(cx, |application, cx| {
                application.record_panel_size(
                    viewport.width.as_f32(),
                    viewport.height.as_f32(),
                    cx,
                );
            });
        });

        let mut status = if target.process_name.is_empty() {
            PanelStatus::Ready("选择一条记录进行粘贴".into())
        } else {
            PanelStatus::Ready(format!("将粘贴到 {}", target.process_name))
        };
        let entries = page.items;
        let list_scroll = ListState::new(entries.len(), ListAlignment::Top, px(180.0));
        let panel = cx.entity().downgrade();
        let preview_application = application.clone();
        list_scroll.set_scroll_handler(move |event, _window, app| {
            schedule_entity_update(
                preview_application.clone(),
                |application, cx| application.suspend_preview_for_scroll(cx),
                app,
            );
            let load_next_page = panel.update(app, |panel, cx| {
                if panel.list_is_scrolled != event.is_scrolled {
                    panel.list_is_scrolled = event.is_scrolled;
                    cx.notify();
                }
                let should_load = panel.has_more_entries
                    && !panel.page_request_pending
                    && event
                        .visible_range
                        .end
                        .saturating_add(HISTORY_PAGE_PREFETCH_ITEMS)
                        >= event.count;
                if should_load {
                    panel.page_request_pending = true;
                }
                should_load
            });
            if matches!(load_next_page, Ok(true)) {
                schedule_entity_update(panel.clone(), |panel, cx| panel.load_next_page(cx), app);
            }
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
            entries: Rc::new(entries),
            total_entries: page.total,
            has_more_entries: page.has_more,
            page_request_pending: false,
            status,
            pending_action: None,
            is_open: true,
            hide_suppressed_until: Instant::now() + POPUP_OPEN_GRACE,
            categories,
            category_sources,
            active_category_id,
            category_scroll: ScrollHandle::new(),
            category_editor: None,
            modifier_actions,
            active_modifier_action: None,
            list_scroll,
            image_resource_cache: Rc::new(RefCell::new(BoundedImageResourceCache::new(
                IMAGE_RESOURCE_CACHE_CAPACITY,
            ))),
            list_is_scrolled: false,
            list_width: 0.0,
            search_input,
            search_scope: ClipboardSearchScope::Content,
            edit_input,
            category_name_input,
            editing_item_id: None,
            last_deleted_item_id: None,
            _subscriptions: vec![
                search_subscription,
                keystroke_subscription,
                size_subscription,
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
        let search_scope = self.search_scope;
        match self.application.update(cx, |application, _| {
            application.query_entries_page(query, &category_id, search_scope, 0)
        }) {
            Ok(Ok(page)) => {
                self.entries = Rc::new(page.items);
                self.total_entries = page.total;
                self.has_more_entries = page.has_more;
                self.page_request_pending = false;
                self.list_scroll.reset(self.entries.len());
                self.list_is_scrolled = false;
            }
            Ok(Err(error)) => {
                self.page_request_pending = false;
                error!(%error, "failed to query clipboard popup history");
                self.entries = Rc::new(Vec::new());
                self.total_entries = 0;
                self.has_more_entries = false;
                self.status = PanelStatus::Error("查询失败，请查看日志".into());
            }
            Err(error) => {
                self.page_request_pending = false;
                error!(%error, "clipboard application became unavailable");
                self.entries = Rc::new(Vec::new());
                self.total_entries = 0;
                self.has_more_entries = false;
                self.status = PanelStatus::Error("剪贴板核心不可用".into());
            }
        }
    }

    fn load_next_page(&mut self, cx: &mut Context<Self>) {
        if !self.has_more_entries {
            self.page_request_pending = false;
            return;
        }
        let query = self.search_input.read(cx).value().to_string();
        let category_id = self.active_category_id.clone();
        let search_scope = self.search_scope;
        let offset = self.entries.len();
        match self.application.update(cx, |application, _| {
            application.query_entries_page(&query, &category_id, search_scope, offset)
        }) {
            Ok(Ok(page)) => {
                let appended = page.items.len();
                Rc::make_mut(&mut self.entries).extend(page.items);
                self.total_entries = page.total;
                self.has_more_entries = page.has_more;
                self.page_request_pending = false;
                if appended > 0 {
                    self.list_scroll.splice(offset..offset, appended);
                }
            }
            Ok(Err(error)) => {
                self.page_request_pending = false;
                error!(%error, offset, "failed to load next clipboard history page");
                self.status = PanelStatus::Error("加载更多记录失败，请查看日志".into());
            }
            Err(error) => {
                self.page_request_pending = false;
                error!(%error, offset, "clipboard application unavailable while paging");
                self.status = PanelStatus::Error("剪贴板核心不可用".into());
            }
        }
        cx.notify();
    }

    fn toggle_search_scope(&mut self, cx: &mut Context<Self>) {
        self.search_scope = match self.search_scope {
            ClipboardSearchScope::Content => ClipboardSearchScope::Extended,
            ClipboardSearchScope::Extended => ClipboardSearchScope::Content,
        };
        let query = self.search_input.read(cx).value().to_string();
        self.reload_entries(&query, cx);
        cx.notify();
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
                self.pending_action = Some((command_id, action, entry.id));
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
        let Some((pending_id, action, item_id)) = self.pending_action else {
            return;
        };
        if pending_id != correlation_id {
            return;
        }
        self.pending_action = None;
        if result.success {
            info!(%correlation_id, ?action, "clipboard action completed");
            let application = self.application.clone();
            let panel = (action == ClipboardAction::CopyOnly).then(|| cx.entity().downgrade());
            schedule_entity_update(
                application,
                move |application, cx| {
                    if let Err(error) = application.mark_item_used(item_id) {
                        warn!(%error, item_id, "clipboard action succeeded but usage could not be recorded");
                    }
                    if let Some(panel) = panel {
                        schedule_entity_update(panel, |panel, cx| panel.refresh_entries(cx), cx);
                    }
                },
                cx,
            );
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
        preview_window_handle: i64,
        foreground_window_handle: i64,
        foreground_process_id: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dismiss = PopupDismissContext {
            popup_window_handle,
            preview_window_handle,
            current_process_id: std::process::id(),
            paste_pending: self.pending_action.is_some(),
            is_open: self.is_open,
        };
        if should_hide_for_foreground_change(
            dismiss,
            foreground_window_handle,
            foreground_process_id,
            Instant::now() < self.hide_suppressed_until,
        ) {
            self.close(window, cx);
        }
    }

    pub(crate) fn handle_pointer_pressed(
        &mut self,
        popup_window_handle: i64,
        preview_window_handle: i64,
        pressed_root_window_handle: i64,
        pressed_process_id: u32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let dismiss = PopupDismissContext {
            popup_window_handle,
            preview_window_handle,
            current_process_id: std::process::id(),
            paste_pending: self.pending_action.is_some(),
            is_open: self.is_open,
        };
        if should_hide_for_pointer_press(dismiss, pressed_root_window_handle, pressed_process_id) {
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
        schedule_entity_update(
            application,
            move |application, cx| {
                application.dismiss_preview(cx);
                let reason = match application.hide_popup(popup_window, window_handle) {
                    Ok(_) => return,
                    Err(error) => {
                        error!(%error, "failed to request clipboard popup hide");
                        format!("无法隐藏面板：{error}")
                    }
                };
                let _ = panel.update(cx, |panel, cx| {
                    panel.is_open = true;
                    panel.status = PanelStatus::Error(reason);
                    cx.notify();
                });
            },
            cx,
        );
    }
}

impl Render for ClipboardPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let modifier_actions = self.modifier_actions;
        let entries_empty = self.entries.is_empty();
        let search_query = self.search_input.read(cx).value().trim().to_owned();
        let status_is_error = self.status.is_error();
        let status_should_render = self.status.should_render();
        let status_label = self.status.label().to_owned();
        let summary = self
            .active_category()
            .map(|category| category.name.clone())
            .unwrap_or_else(|| "全部".into());
        let header_summary = clipboard_header_summary(&summary, self.total_entries);
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
        let list_width = panel_list_width(window.viewport_size().width.as_f32());
        if self.list_width > 0.0 && (self.list_width - list_width).abs() > 0.5 {
            self.list_scroll.reset(self.entries.len());
            self.list_is_scrolled = false;
        }
        self.list_width = list_width;
        let available_image_width = (list_width - 24.0).max(28.0);
        let scroll_to_top = self.list_scroll.clone();
        let panel = cx.entity().downgrade();
        let categories = self.categories.clone();
        let category_strip_width = category_strip_width(&categories);
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
        let search_scope_panel = cx.entity().downgrade();
        let extended_search = self.search_scope == ClipboardSearchScope::Extended;
        let move_application = self.application.clone();
        let resize_application = self.application.clone();
        let content = div()
            .relative()
            .size_full()
            .font_family(UI_FONT_FAMILY)
            .text_color(theme.text)
            .flex()
            .flex_col()
            .gap(px(PANEL_SECTION_GAP))
            .on_modifiers_changed(move |event, _window, app| {
                let _ = modifier_panel.update(app, |panel, cx| {
                    let action =
                        modifier_feedback_from_modifiers(event.modifiers, modifier_actions);
                    if panel.active_modifier_action != action {
                        panel.active_modifier_action = action;
                        cx.notify();
                    }
                });
            })
            .child(inset_panel_section(
                div()
                    .id("clipboard-panel-header")
                    .mt(px(PANEL_SECTION_GAP))
                    .h(px(24.0))
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .cursor_default()
                    .on_mouse_down(gpui::MouseButton::Left, move |_event, window, cx| {
                        cx.stop_propagation();
                        let Some(handle) = native_window_handle(window) else {
                            return;
                        };
                        let _ = move_application.update(cx, |application, _cx| {
                            application.begin_window_move(handle);
                        });
                    })
                    .child(
                        div()
                            .text_size(px(TEXT_BODY))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("剪贴板"),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_1()
                            .text_size(px(TEXT_META))
                            .text_color(theme.text_muted)
                            .truncate()
                            .child(header_summary),
                    )
                    .when_some(modifier_action, |header, (modifier, action)| {
                        header.child(
                            div()
                                .h(px(21.0))
                                .px_2()
                                .rounded(px(RADIUS_CONTROL))
                                .border_1()
                                .border_color(theme.accent)
                                .bg(theme.accent_soft)
                                .text_size(px(TEXT_META))
                                .text_color(theme.accent_text)
                                .flex()
                                .items_center()
                                .child(modifier_action_label(modifier, action)),
                        )
                    })
                    .when(status_should_render, |header| {
                        header.child(
                            div()
                                .max_w(px(150.0))
                                .h(px(21.0))
                                .px_2()
                                .rounded(px(RADIUS_CONTROL))
                                .border_1()
                                .border_color(if status_is_error {
                                    theme.danger
                                } else {
                                    theme.border
                                })
                                .bg(if status_is_error {
                                    theme.danger_soft
                                } else {
                                    theme.accent_soft
                                })
                                .text_size(px(TEXT_META))
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
            ))
            .child(inset_panel_section(
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
                                move |event, window, app| {
                                    app.stop_propagation();
                                    let delta = event.delta.pixel_delta(px(48.0));
                                    let maximum = scroll.max_offset().width.as_f32().max(0.0);
                                    let current = scroll.offset().x.as_f32();
                                    let update = resolve_category_wheel_offset(
                                        current,
                                        maximum,
                                        delta.x.as_f32(),
                                        delta.y.as_f32(),
                                    );
                                    if update.changed {
                                        scroll.set_offset(point(px(update.offset), px(0.0)));
                                        window.refresh();
                                    }
                                }
                            })
                            .child(
                                div()
                                    .h(px(26.0))
                                    .w(px(category_strip_width))
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
                            .rounded(px(RADIUS_CONTROL))
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
            ))
            .child(inset_panel_section(
                div().h(px(28.0)).flex().items_center().gap_2().child(
                    div()
                        .min_w_0()
                        .flex_1()
                        .h(px(28.0))
                        .rounded(px(RADIUS_CONTROL))
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
                            div()
                                .min_w_0()
                                .flex_1()
                                .text_size(px(TEXT_BODY))
                                .text_color(theme.text)
                                .child(
                                    Input::new(&self.search_input)
                                        .appearance(false)
                                        .bordered(false)
                                        .focus_bordered(false),
                                ),
                        )
                        .child(
                            div()
                                .id("search-scope-toggle")
                                .size(px(20.0))
                                .flex_none()
                                .rounded(px(RADIUS_CONTROL))
                                .flex()
                                .items_center()
                                .justify_center()
                                .cursor_pointer()
                                .bg(if extended_search {
                                    theme.accent_soft
                                } else {
                                    gpui::rgba(0x00000000)
                                })
                                .text_color(if extended_search {
                                    theme.accent_text
                                } else {
                                    theme.text_muted
                                })
                                .hover(move |style| style.bg(theme.surface_hover))
                                .tooltip(move |window, cx| {
                                    Tooltip::new(if extended_search {
                                        "关闭扩展搜索"
                                    } else {
                                        "扩展搜索：图片名、文件路径、来源进程和窗口标题"
                                    })
                                    .build(window, cx)
                                })
                                .on_click(move |_event, _window, app| {
                                    let _ = search_scope_panel.update(app, |panel, cx| {
                                        panel.toggle_search_scope(cx);
                                    });
                                })
                                .child(Icon::new(MaterialIcon::ManageSearch).size(px(14.0))),
                        ),
                ),
            ))
            .child(inset_panel_section(
                div()
                    .mb(px(PANEL_CONTENT_INSET))
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .bg(theme.canvas)
                    .overflow_hidden()
                    .when(entries_empty, |list| {
                        list.child(empty_state(
                            theme,
                            "clipboard-history-empty",
                            if search_query.is_empty() {
                                MaterialIcon::History
                            } else {
                                MaterialIcon::Search
                            },
                            if search_query.is_empty() {
                                "暂无剪贴板记录"
                            } else {
                                "没有匹配的剪贴板记录"
                            },
                            if search_query.is_empty() {
                                "复制内容后会自动出现在这里。"
                            } else {
                                "尝试缩短关键词，或切换分类和搜索范围。"
                            },
                        ))
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
                        let shell_application = self.application.clone();
                        let image_resource_cache = Rc::clone(&self.image_resource_cache);
                        let theme = self.theme;
                        container.child(
                            list(self.list_scroll.clone(), move |index, _window, app| {
                                let Some(entry) = entries.get(index).cloned() else {
                                    return div().into_any_element();
                                };
                                for resource_path in [
                                    entry.image_path.as_str(),
                                    entry.source.process_icon_path.as_str(),
                                ]
                                .into_iter()
                                .filter(|path| !path.is_empty())
                                {
                                    if let Some(evicted) = image_resource_cache
                                        .borrow_mut()
                                        .touch(Path::new(resource_path))
                                    {
                                        ImageSource::from(evicted).remove_asset(app);
                                    }
                                }
                                let action_entry = entry.clone();
                                let management_entry = entry.clone();
                                let process_name = entry.source.process_name.to_lowercase();
                                let category_hint = if custom_categories.is_empty() {
                                    Some("请先新建分类".to_owned())
                                } else if process_name.is_empty() {
                                    Some("来源未知".to_owned())
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
                                let open_application = shell_application.clone();
                                let on_open = Rc::new(
                                    move |target: nxtools_clipboard::ClipboardOpenTarget,
                                          _event: &gpui::ClickEvent,
                                          _window: &mut Window,
                                          app: &mut App| {
                                        app.stop_propagation();
                                        let _ = open_application.update(app, |application, _cx| {
                                            application.open_quick_target(&target);
                                        });
                                    },
                                );
                                let preview_application = shell_application.clone();
                                let on_preview = Rc::new(
                                    move |entered: bool,
                                          item: ClipboardItem,
                                          panel_bounds: Bounds<Pixels>,
                                          pointer_position: Point<Pixels>,
                                          panel_scale_factor: f32,
                                          app: &mut App| {
                                        let _ =
                                            preview_application.update(app, |application, cx| {
                                                application.schedule_preview(
                                                    entered,
                                                    item,
                                                    panel_bounds,
                                                    pointer_position,
                                                    panel_scale_factor,
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
                                    modifier_actions,
                                    CategorySourceMenu {
                                        options: category_options,
                                        unavailable_hint: category_hint,
                                    },
                                    HistoryRowHandlers {
                                        action: on_action,
                                        management: on_management,
                                        open: on_open,
                                        preview: on_preview,
                                    },
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
                                        .rounded(px(RADIUS_CONTROL))
                                        .border_1()
                                        .border_color(theme.border)
                                        .bg(theme.surface_raised)
                                        .shadow_md()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(theme.surface_hover))
                                        .text_size(px(TEXT_META))
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
                                            app.stop_propagation();
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
                                        .rounded(px(RADIUS_CONTROL))
                                        .border_1()
                                        .border_color(theme.border_active)
                                        .bg(theme.surface_raised)
                                        .shadow_md()
                                        .cursor_pointer()
                                        .hover(move |style| style.bg(theme.surface_hover))
                                        .text_size(px(TEXT_META))
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
            ))
            .when(is_editing, |panel| {
                panel.child(
                    div()
                        .absolute()
                        .top(px(36.0))
                        .left_2()
                        .right_2()
                        .bottom_2()
                        .p_2()
                        .rounded(px(RADIUS_CONTROL))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.surface_raised)
                        .shadow_lg()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .text_size(px(TEXT_BODY))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .child("编辑文本"),
                                )
                                .child(
                                    div()
                                        .text_size(px(TEXT_META))
                                        .text_color(theme.text_muted)
                                        .child("保存后统一为纯文本"),
                                ),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_h_0()
                                .p_2()
                                .rounded(px(RADIUS_CONTROL))
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
                                        .text_size(px(TEXT_META))
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
                        .p_2()
                        .rounded(px(RADIUS_CONTROL))
                        .border_1()
                        .border_color(theme.border)
                        .bg(theme.surface_raised)
                        .shadow_lg()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .text_size(px(TEXT_BODY))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .child(category_editor_title),
                        )
                        .child(
                            div()
                                .h(px(32.0))
                                .px_2()
                                .rounded(px(RADIUS_CONTROL))
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
                                .text_size(px(TEXT_META))
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
            .child(panel_resize_hitbox(
                div()
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
                    .on_mouse_down(gpui::MouseButton::Left, move |_event, window, cx| {
                        cx.stop_propagation();
                        let Some(handle) = native_window_handle(window) else {
                            return;
                        };
                        let _ = resize_application.update(cx, |application, _cx| {
                            application.begin_window_resize(handle);
                        });
                    }),
            ));
        panel_window_frame(theme, content)
    }
}

fn panel_window_frame(theme: Theme, content: impl IntoElement) -> gpui::AnyElement {
    div()
        .id("clipboard-panel-window")
        .size_full()
        .child(
            div()
                .id("clipboard-panel-shell")
                .size_full()
                .overflow_hidden()
                .border_1()
                .border_color(theme.border)
                .bg(theme.canvas)
                .child(content),
        )
        .into_any_element()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PopupDismissContext {
    popup_window_handle: i64,
    preview_window_handle: i64,
    current_process_id: u32,
    paste_pending: bool,
    is_open: bool,
}

fn should_hide_for_foreground_change(
    dismiss: PopupDismissContext,
    foreground_window_handle: i64,
    foreground_process_id: u32,
    within_open_grace: bool,
) -> bool {
    dismiss.is_open
        && !dismiss.paste_pending
        && !within_open_grace
        && dismiss.popup_window_handle != 0
        && foreground_window_handle != 0
        && foreground_process_id != dismiss.current_process_id
        && foreground_window_handle != dismiss.popup_window_handle
        && foreground_window_handle != dismiss.preview_window_handle
}

fn should_hide_for_pointer_press(
    dismiss: PopupDismissContext,
    pressed_root_window_handle: i64,
    pressed_process_id: u32,
) -> bool {
    dismiss.is_open
        && !dismiss.paste_pending
        && dismiss.popup_window_handle != 0
        && pressed_process_id != dismiss.current_process_id
        && pressed_root_window_handle != dismiss.popup_window_handle
        && pressed_root_window_handle != dismiss.preview_window_handle
}

fn modifier_from_modifiers(modifiers: gpui::Modifiers) -> Option<ClipboardClickModifier> {
    match (
        modifiers.control,
        modifiers.alt,
        modifiers.shift,
        modifiers.platform || modifiers.function,
    ) {
        (true, false, false, false) => Some(ClipboardClickModifier::Control),
        (false, true, false, false) => Some(ClipboardClickModifier::Alt),
        (false, false, true, false) => Some(ClipboardClickModifier::Shift),
        _ => None,
    }
}

fn modifier_feedback_from_modifiers(
    modifiers: gpui::Modifiers,
    actions: ClipboardModifierActions,
) -> Option<(ClipboardClickModifier, ClipboardClickAction)> {
    modifier_from_modifiers(modifiers).map(|modifier| (modifier, actions.action(modifier)))
}

fn action_from_modifiers(
    modifiers: gpui::Modifiers,
    actions: ClipboardModifierActions,
) -> Option<ClipboardAction> {
    let (_, action) = modifier_feedback_from_modifiers(modifiers, actions)?;
    match action {
        ClipboardClickAction::OriginalPaste => Some(ClipboardAction::OriginalPaste),
        ClipboardClickAction::CopyOnly => Some(ClipboardAction::CopyOnly),
        ClipboardClickAction::PlainText => Some(ClipboardAction::PlainText),
        ClipboardClickAction::WithoutLineBreaks => Some(ClipboardAction::WithoutLineBreaks),
        ClipboardClickAction::Disabled => None,
    }
}

fn modifier_action_label(modifier: ClipboardClickModifier, action: ClipboardClickAction) -> String {
    format!(
        "{} · {}",
        modifier_label(modifier),
        click_action_label(action)
    )
}

fn clipboard_header_summary(category: &str, total_entries: usize) -> String {
    format!("{category} · {total_entries} 条")
}

fn modifier_label(modifier: ClipboardClickModifier) -> &'static str {
    match modifier {
        ClipboardClickModifier::Control => "Ctrl",
        ClipboardClickModifier::Alt => "Alt",
        ClipboardClickModifier::Shift => "Shift",
    }
}

fn click_action_label(action: ClipboardClickAction) -> &'static str {
    match action {
        ClipboardClickAction::OriginalPaste => "原格式粘贴",
        ClipboardClickAction::CopyOnly => "仅复制",
        ClipboardClickAction::PlainText => "纯文本／路径粘贴",
        ClipboardClickAction::WithoutLineBreaks => "去换行粘贴",
        ClipboardClickAction::Disabled => "不执行",
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CategoryWheelUpdate {
    offset: f32,
    changed: bool,
}

fn resolve_category_wheel_offset(
    current: f32,
    maximum: f32,
    delta_x: f32,
    delta_y: f32,
) -> CategoryWheelUpdate {
    if maximum <= 0.0 {
        return CategoryWheelUpdate {
            offset: 0.0,
            changed: current != 0.0,
        };
    }
    let delta = if delta_x.abs() > delta_y.abs() {
        delta_x
    } else {
        delta_y
    };
    let offset = (current + delta).clamp(-maximum, 0.0);
    CategoryWheelUpdate {
        offset,
        changed: offset != current,
    }
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

fn category_tab_width(category: &ClipboardCategory) -> f32 {
    CATEGORY_HORIZONTAL_PADDING
        + CATEGORY_ICON_WIDTH
        + CATEGORY_ICON_TEXT_GAP
        + UnicodeWidthStr::width(category.name.as_str()) as f32 * CATEGORY_TEXT_CELL_WIDTH
}

fn category_strip_width(categories: &[ClipboardCategory]) -> f32 {
    let tabs = categories.iter().map(category_tab_width).sum::<f32>();
    let gaps = categories.len().saturating_sub(1) as f32 * CATEGORY_GAP;
    tabs + gaps
}

fn category_tab(
    theme: Theme,
    category: ClipboardCategory,
    selected: bool,
    panel: WeakEntity<ClipboardPanel>,
) -> gpui::AnyElement {
    let tab_width = category_tab_width(&category);
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
        .w(px(tab_width))
        .flex_none()
        .px(px(7.0))
        .rounded(px(RADIUS_CONTROL))
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
            context_menu_shell(menu)
                .item(
                    context_menu_item(
                        theme,
                        "clipboard-category-menu-rename",
                        "重命名",
                        MaterialIcon::Edit,
                        MenuTone::Normal,
                    )
                    .on_click({
                        let panel = panel.clone();
                        let category = edit_category.clone();
                        move |_event, window, app| {
                            let _ = panel.update(app, |panel, cx| {
                                panel.begin_category_editor(Some(&category), window, cx);
                            });
                        }
                    }),
                )
                .item(
                    context_menu_item(
                        theme,
                        "clipboard-category-menu-delete",
                        "删除分类",
                        MaterialIcon::Delete,
                        MenuTone::Danger,
                    )
                    .on_click({
                        let panel = panel.clone();
                        let category_id = delete_category_id.clone();
                        move |_event, _window, app| {
                            let _ = panel.update(app, |panel, cx| {
                                panel.delete_category(&category_id, cx);
                            });
                        }
                    }),
                )
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
        .rounded(px(RADIUS_CONTROL))
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
    panel_size: nxtools_core::ClipboardPanelSize,
    cx: &App,
) -> (Bounds<gpui::Pixels>, Option<gpui::DisplayId>) {
    let desired_size = size(px(panel_size.width as f32), px(panel_size.height as f32));
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
                size: desired_size,
            },
            None,
        );
    };

    let screen = display.bounds();
    let panel_size = size(
        desired_size.width.min(screen.size.width),
        desired_size.height.min(screen.size.height),
    );
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
    use gpui::{ScrollDelta, ScrollWheelEvent, TestAppContext};
    use gpui_component::Root;
    use std::cell::Cell;

    #[test]
    fn history_count_belongs_to_the_header_not_the_search_row() {
        assert_eq!(clipboard_header_summary("全部", 134), "全部 · 134 条");
    }

    #[gpui::test]
    fn production_clipboard_history_uses_the_shared_empty_state(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let page = ClipboardHistoryPage {
            items: Vec::new(),
            total: 0,
            has_more: false,
        };
        let categories = vec![ClipboardCategory {
            id: BUILTIN_ALL_CATEGORY_ID.into(),
            name: "全部".into(),
            kind: ClipboardCategoryKind::All,
            sort_order: 0,
        }];
        let (_root, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(520.0), px(440.0)));
            let panel = cx.new(|cx| {
                ClipboardPanel::new(
                    page,
                    categories,
                    ClipboardModifierActions::default(),
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "editor".into(),
                        window_title: "document".into(),
                    },
                    WeakEntity::new_invalid(),
                    window,
                    cx,
                )
            });
            Root::new(panel, window, cx)
        });

        window.refresh().expect("render clipboard empty state");
        let state = window
            .debug_bounds("clipboard-history-empty")
            .expect("clipboard shared empty state");
        let icon = window
            .debug_bounds("clipboard-history-empty-icon")
            .expect("clipboard empty icon");
        let title = window
            .debug_bounds("clipboard-history-empty-title")
            .expect("clipboard empty title");
        let description = window
            .debug_bounds("clipboard-history-empty-description")
            .expect("clipboard empty description");
        assert_eq!(icon.size, size(px(18.0), px(18.0)));
        assert_eq!(title.size.height, px(18.0));
        assert_eq!(description.size.height, px(16.0));
        assert_eq!(title.left(), description.left());
        assert!(state.size.width < px(520.0));
    }

    #[gpui::test]
    fn panel_shell_fills_the_native_window_without_a_uniform_transparent_gutter(
        cx: &mut TestAppContext,
    ) {
        struct PanelFrameFixture(Rc<Cell<u32>>);

        impl Render for PanelFrameFixture {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                let presses = Rc::clone(&self.0);
                panel_window_frame(
                    Theme::zed_dark(),
                    div()
                        .size_full()
                        .on_mouse_down(gpui::MouseButton::Left, move |_event, _window, _cx| {
                            presses.set(presses.get() + 1)
                        }),
                )
            }
        }

        let window = cx.add_empty_window();
        let presses = Rc::new(Cell::new(0));
        let fixture_presses = Rc::clone(&presses);
        window.draw(
            point(px(0.0), px(0.0)),
            size(px(100.0), px(100.0)),
            move |_window, cx| cx.new(|_| PanelFrameFixture(fixture_presses)),
        );

        window.simulate_mouse_down(
            point(px(2.0), px(50.0)),
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        assert_eq!(presses.get(), 1);
    }

    #[gpui::test]
    fn inset_sections_leave_safe_edges_and_resize_stays_on_the_native_corner(
        cx: &mut TestAppContext,
    ) {
        struct SpacingFixture {
            content_presses: Rc<Cell<u32>>,
            resize_presses: Rc<Cell<u32>>,
        }

        impl Render for SpacingFixture {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                let content_presses = Rc::clone(&self.content_presses);
                let resize_presses = Rc::clone(&self.resize_presses);
                panel_window_frame(
                    Theme::zed_dark(),
                    div()
                        .relative()
                        .size_full()
                        .flex()
                        .flex_col()
                        .child(inset_panel_section(div().flex_1().on_mouse_down(
                            gpui::MouseButton::Left,
                            move |_event, _window, _cx| {
                                content_presses.set(content_presses.get() + 1);
                            },
                        )))
                        .child(panel_resize_hitbox(div().on_mouse_down(
                            gpui::MouseButton::Left,
                            move |_event, _window, cx| {
                                cx.stop_propagation();
                                resize_presses.set(resize_presses.get() + 1);
                            },
                        ))),
                )
            }
        }

        let window = cx.add_empty_window();
        let content_presses = Rc::new(Cell::new(0));
        let resize_presses = Rc::new(Cell::new(0));
        let fixture_content_presses = Rc::clone(&content_presses);
        let fixture_resize_presses = Rc::clone(&resize_presses);
        window.draw(
            point(px(0.0), px(0.0)),
            size(px(100.0), px(40.0)),
            move |_window, cx| {
                cx.new(|_| SpacingFixture {
                    content_presses: fixture_content_presses,
                    resize_presses: fixture_resize_presses,
                })
            },
        );

        window.simulate_mouse_down(
            point(px(4.0), px(20.0)),
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        assert_eq!(content_presses.get(), 0);

        window.simulate_mouse_down(
            point(px(9.0), px(20.0)),
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        assert_eq!(content_presses.get(), 1);

        window.simulate_mouse_down(
            point(px(99.0), px(39.0)),
            gpui::MouseButton::Left,
            gpui::Modifiers::default(),
        );
        assert_eq!(resize_presses.get(), 1);
    }

    #[test]
    fn transparent_panel_frame_does_not_paint_a_rectangular_drop_shadow() {
        let source = include_str!("mod.rs");
        let frame_start = source
            .find("fn panel_window_frame")
            .expect("clipboard panel frame implementation");
        let frame_end = source[frame_start..]
            .find("#[derive(Debug, Clone, Copy, PartialEq, Eq)]")
            .expect("end of clipboard panel frame implementation");
        let frame_source = &source[frame_start..frame_start + frame_end];

        assert!(
            !frame_source.contains(".shadow_"),
            "drop shadows paint a visible rectangular halo into a transparent Windows surface"
        );
    }

    #[test]
    fn transparent_panel_uses_a_square_outer_shell_without_corner_gutters() {
        let source = include_str!("mod.rs");
        let frame = source
            .split("fn panel_window_frame")
            .nth(1)
            .and_then(|source| source.split("struct PopupDismissContext").next())
            .expect("panel window frame source");

        assert!(frame.contains(".size_full()"));
        assert!(
            !frame.contains(".rounded("),
            "a rounded transparent root exposes the rectangular HWND behind its corners"
        );
    }

    #[gpui::test]
    fn rendered_category_bar_moves_on_vertical_wheel(cx: &mut TestAppContext) {
        struct CategoryScrollFixture(ScrollHandle);

        impl Render for CategoryScrollFixture {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                let rendered_handle = self.0.clone();
                let wheel_handle = self.0.clone();
                div()
                    .id("category-fixture-scroll")
                    .w(px(120.0))
                    .h(px(26.0))
                    .overflow_x_scroll()
                    .track_scroll(&rendered_handle)
                    .on_scroll_wheel(move |event, window, app| {
                        app.stop_propagation();
                        let delta = event.delta.pixel_delta(px(48.0));
                        let update = resolve_category_wheel_offset(
                            wheel_handle.offset().x.as_f32(),
                            wheel_handle.max_offset().width.as_f32(),
                            delta.x.as_f32(),
                            delta.y.as_f32(),
                        );
                        if update.changed {
                            wheel_handle.set_offset(point(px(update.offset), px(0.0)));
                            window.refresh();
                        }
                    })
                    .child(
                        div()
                            .h(px(26.0))
                            .w(px(8.0 * 48.0 + 7.0 * CATEGORY_GAP))
                            .flex()
                            .flex_none()
                            .gap_1()
                            .children((0_usize..8).map(|index| {
                                div()
                                    .id(("category-fixture", index))
                                    .w(px(48.0))
                                    .h(px(26.0))
                                    .flex_none()
                            })),
                    )
            }
        }

        let window = cx.add_empty_window();
        let handle = ScrollHandle::new();
        let fixture_handle = handle.clone();

        window.draw(
            point(px(0.0), px(0.0)),
            size(px(120.0), px(30.0)),
            move |_window, cx| cx.new(|_| CategoryScrollFixture(fixture_handle)),
        );

        assert!(handle.max_offset().width > px(0.0));
        window.simulate_event(ScrollWheelEvent {
            position: point(px(10.0), px(10.0)),
            delta: ScrollDelta::Lines(point(0.0, -1.0)),
            ..Default::default()
        });

        assert!(handle.offset().x < px(0.0));
    }

    #[gpui::test]
    fn panel_close_side_effects_are_deferred_outside_the_current_entity_lease(
        cx: &mut TestAppContext,
    ) {
        let entity = cx.update(|cx| cx.new(|_| 0_u8));
        let weak = entity.downgrade();

        entity.update(cx, |value, cx| {
            *value += 1;
            schedule_entity_update(weak, |value, _cx| *value += 1, cx);
        });
        cx.background_executor.run_until_parked();

        assert_eq!(cx.read(|cx| *entity.read(cx)), 2);
    }

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
        assert!(should_hide_for_foreground_change(
            dismiss_context(10, 0, false, true),
            20,
            200,
            false,
        ));
        assert!(!should_hide_for_foreground_change(
            dismiss_context(10, 0, true, true),
            20,
            200,
            false,
        ));
        assert!(!should_hide_for_foreground_change(
            dismiss_context(10, 0, false, true),
            10,
            200,
            false,
        ));
        assert!(!should_hide_for_foreground_change(
            dismiss_context(10, 0, false, false),
            20,
            200,
            false,
        ));
    }

    #[test]
    fn internal_windows_and_the_opening_grace_period_do_not_hide_the_panel() {
        assert!(!should_hide_for_foreground_change(
            dismiss_context(10, 30, false, true),
            30,
            100,
            false,
        ));
        assert!(!should_hide_for_foreground_change(
            dismiss_context(10, 30, false, true),
            40,
            100,
            false,
        ));
        assert!(!should_hide_for_foreground_change(
            dismiss_context(10, 30, false, true),
            20,
            200,
            true,
        ));
        assert!(should_hide_for_foreground_change(
            dismiss_context(10, 30, false, true),
            20,
            200,
            false,
        ));
    }

    #[test]
    fn only_an_external_pointer_press_hides_the_open_panel() {
        let open = dismiss_context(10, 30, false, true);
        assert!(should_hide_for_pointer_press(open, 20, 200));
        assert!(!should_hide_for_pointer_press(open, 10, 200,));
        assert!(!should_hide_for_pointer_press(open, 30, 200,));
        assert!(!should_hide_for_pointer_press(open, 40, 100,));
        assert!(!should_hide_for_pointer_press(
            dismiss_context(10, 30, true, true),
            20,
            200,
        ));
    }

    fn dismiss_context(
        popup_window_handle: i64,
        preview_window_handle: i64,
        paste_pending: bool,
        is_open: bool,
    ) -> PopupDismissContext {
        PopupDismissContext {
            popup_window_handle,
            preview_window_handle,
            current_process_id: 100,
            paste_pending,
            is_open,
        }
    }

    #[test]
    fn panel_metrics_match_the_reference_window() {
        let size = nxtools_core::ClipboardPanelSize::default();
        assert_eq!(size.width, 300);
        assert_eq!(size.height, 440);
        assert_eq!(PANEL_CONTENT_INSET, 8.0);
        assert_eq!(PANEL_SECTION_GAP, 6.0);
        assert_eq!(panel_list_width(size.width as f32), 284.0);
        assert_eq!(panel_list_width(240.0), 224.0);
        assert_eq!(panel_list_width(560.0), 544.0);
    }

    #[test]
    fn clipboard_image_resources_are_bounded_and_recent_entries_are_retained() {
        let mut cache = BoundedImageResourceCache::new(3);
        assert_eq!(cache.touch(Path::new("one.png")), None);
        assert_eq!(cache.touch(Path::new("two.png")), None);
        assert_eq!(cache.touch(Path::new("three.png")), None);
        assert_eq!(cache.touch(Path::new("one.png")), None);
        assert_eq!(
            cache.touch(Path::new("four.png")),
            Some(PathBuf::from("two.png"))
        );
        assert_eq!(cache.len(), 3);
        assert!(cache.entries.contains(Path::new("one.png")));
        assert!(cache.entries.contains(Path::new("three.png")));
        assert!(cache.entries.contains(Path::new("four.png")));
    }

    #[test]
    fn exactly_one_supported_modifier_selects_an_accelerated_action() {
        let actions = ClipboardModifierActions::default();
        assert_eq!(
            action_from_modifiers(
                gpui::Modifiers {
                    control: true,
                    ..Default::default()
                },
                actions
            ),
            Some(ClipboardAction::CopyOnly)
        );
        assert_eq!(
            action_from_modifiers(
                gpui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
                actions
            ),
            Some(ClipboardAction::PlainText)
        );
        assert_eq!(
            action_from_modifiers(
                gpui::Modifiers {
                    shift: true,
                    ..Default::default()
                },
                actions
            ),
            Some(ClipboardAction::WithoutLineBreaks)
        );
        assert_eq!(
            action_from_modifiers(
                gpui::Modifiers {
                    control: true,
                    shift: true,
                    ..Default::default()
                },
                actions
            ),
            None
        );
    }

    #[test]
    fn configured_modifier_action_can_be_changed_or_disabled() {
        let actions = ClipboardModifierActions {
            control: ClipboardClickAction::PlainText,
            alt: ClipboardClickAction::Disabled,
            shift: ClipboardClickAction::OriginalPaste,
        };
        assert_eq!(
            action_from_modifiers(
                gpui::Modifiers {
                    control: true,
                    ..Default::default()
                },
                actions,
            ),
            Some(ClipboardAction::PlainText)
        );
        assert_eq!(
            action_from_modifiers(
                gpui::Modifiers {
                    alt: true,
                    ..Default::default()
                },
                actions,
            ),
            None
        );
    }

    #[test]
    fn confirmed_clipboard_action_marks_the_item_used_before_the_panel_closes() {
        let source = include_str!("mod.rs");
        let handler = source
            .split("pub(crate) fn handle_command_result")
            .nth(1)
            .and_then(|source| source.split("pub(crate) fn handle_hide_failure").next())
            .expect("clipboard command-result handler source");

        assert!(handler.contains("application.mark_item_used(item_id)"));
        assert!(handler.find("if result.success").is_some());
        assert!(
            handler.find("application.mark_item_used(item_id)")
                < handler.find("self.close(window, cx)")
        );
    }

    #[test]
    fn panel_render_never_reads_the_parent_application_entity() {
        let source = include_str!("mod.rs");
        let render_start = source
            .find("impl Render for ClipboardPanel")
            .expect("clipboard panel render implementation");
        let render_end = source[render_start..]
            .find("fn panel_window_frame")
            .expect("end of clipboard panel render implementation");
        let render_source = &source[render_start..render_start + render_end];

        assert!(
            !render_source.contains("application.read(cx).clipboard_settings()"),
            "ClipboardPanel::render must use cached settings; reading ClipboardApplication while a notification owns it causes GPUI double_lease_panic"
        );
    }

    #[test]
    fn category_wheel_is_horizontal_clamped_and_uses_vertical_wheel_input() {
        assert_eq!(
            resolve_category_wheel_offset(0.0, 120.0, 0.0, -48.0),
            CategoryWheelUpdate {
                offset: -48.0,
                changed: true
            }
        );
        assert_eq!(
            resolve_category_wheel_offset(-100.0, 120.0, 0.0, -48.0),
            CategoryWheelUpdate {
                offset: -120.0,
                changed: true
            }
        );
        assert_eq!(
            resolve_category_wheel_offset(-80.0, 120.0, 32.0, -10.0),
            CategoryWheelUpdate {
                offset: -48.0,
                changed: true
            }
        );
        assert_eq!(
            resolve_category_wheel_offset(-20.0, 0.0, 0.0, -48.0),
            CategoryWheelUpdate {
                offset: 0.0,
                changed: true
            }
        );
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
