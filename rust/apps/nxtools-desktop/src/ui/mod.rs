mod capture_controls;
pub(crate) mod components;
mod efficiency;
mod extension_feedback;
#[cfg(debug_assertions)]
pub(crate) mod final_gallery;
mod history;
mod history_filters;
mod icon_colors;
pub(crate) mod markdown_document;
mod markdown_edit;
pub(crate) mod metrics;
pub(crate) mod motion;
#[cfg(debug_assertions)]
pub(crate) mod native_acceptance;
mod preview;
pub(crate) mod preview_image;
mod preview_text;
mod search_input;
mod summary_targets;
pub(crate) mod typography;

use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::EntityInputHandler as _;
use gpui_kit::component::Icon;
use gpui_kit::component::WindowExt;
use gpui_kit::component::alert::Alert;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::empty::{Empty, EmptyDescription, EmptyHeader, EmptyMedia, EmptyTitle};
use gpui_kit::component::form::{Field, Form};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::input::{
    InputGroup, InputGroupAddon, InputGroupAddonAlignment, InputGroupButton,
};
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::component::tab::Tab;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Selectable, Sizable};
use gpui_kit::{
    App, AppContext, Bounds, Context, Div, Entity, Focusable, ImageSource, KeystrokeEvent,
    ListAlignment, ListState, MouseButton, Pixels, Point, Render, ScrollHandle, Subscription,
    WeakEntity, Window, div, list, point, prelude::*, px, size,
};
use nxtools_clipboard::{
    BUILTIN_ALL_CATEGORY_ID, ClipboardAction, ClipboardCategory, ClipboardCategoryKind,
    ClipboardHistoryError, ClipboardHistoryPage, ClipboardItem, ClipboardKind, ClipboardPageCursor,
    ClipboardSearchScope,
};
use nxtools_core::{ClipboardClickAction, ClipboardClickModifier, ClipboardModifierActions};
use nxtools_protocol::{ClipboardPanelRequest, PlatformCommandResult};
use tracing::{error, info, warn};
use unicode_width::UnicodeWidthStr;

use crate::clipboard::ClipboardApplication;
use crate::icons::MaterialIcon;
use crate::popup_lifecycle::PopupCloseReason;
use crate::theme::Theme;
use crate::ui::components::action::focused_action;
use crate::ui::components::menu::{MenuTone, context_menu_item, context_menu_shell};
use crate::ui::components::state::progress_state;
use crate::ui::metrics::{
    CONTROL_HEIGHT_ACTION, CONTROL_HEIGHT_INPUT, GROUP_HEADER_HEIGHT,
    NOTIFICATION_META_BLOCK_HEIGHT, RADIUS_CONTROL,
};
use crate::ui::typography::{LINE_HEIGHT_BODY, TEXT_BODY, TEXT_META};
use history::{
    CategorySourceMenu, CategorySourceOption, ClipboardItemAction, HistoryRowHandlers, history_row,
};
pub(crate) use preview::{
    ClipboardPreview, PREVIEW_APPEAR_DURATION, PreviewLayout, preview_layout_with_image_mode,
    preview_media_surface, preview_shell,
};

#[cfg(test)]
pub(crate) use preview::preview_layout;

const CATEGORY_GAP: f32 = 4.0;
const CATEGORY_ICON_WIDTH: f32 = 12.0;
const CATEGORY_ICON_TEXT_GAP: f32 = 4.0;
const CATEGORY_HORIZONTAL_PADDING: f32 = 16.0;
const CATEGORY_TEXT_CELL_WIDTH: f32 = 7.0;
const PANEL_CONTENT_INSET: f32 = 8.0;
const CATEGORY_EDITOR_TOP: f32 = 104.0;
const CATEGORY_EDITOR_HEIGHT: f32 = 244.0;
const CATEGORY_EDITOR_SOURCE_LIST_HEIGHT: f32 = 72.0;
// 180 ms lost the race against the OS returning focus to the summoning
// app after a hotkey summon: the foreground-changed dismissal fired before
// the user could move the mouse in, so the panel opened and instantly hid.
const POPUP_OPEN_GRACE: Duration = Duration::from_millis(900);
const HISTORY_PAGE_PREFETCH_ITEMS: usize = 8;
const CLIPBOARD_ACTION_TIMEOUT: Duration = Duration::from_secs(12);
// Up to five 20px actions plus four 4px gaps, with 4px leading clearance.
pub(crate) const HOVER_ACTION_RAIL_WIDTH: f32 = 140.0;
const IMAGE_RESOURCE_CACHE_CAPACITY: usize = 96;
const CLIPBOARD_SEARCH_DEBOUNCE: Duration = Duration::from_millis(100);
const HISTORY_PAGE_SIZE: usize = 80;
const HISTORY_WINDOW_SIZE: usize = HISTORY_PAGE_SIZE * 4;
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

fn source_process_names_from_entries(entries: &[ClipboardItem]) -> Vec<String> {
    let mut names = HashMap::new();
    for entry in entries {
        let source = entry.source.process_name.trim();
        if !source.is_empty() {
            names
                .entry(source.to_lowercase())
                .or_insert_with(|| source.to_owned());
        }
    }
    let mut names = names.into_values().collect::<Vec<_>>();
    names.sort_by_key(|source| source.to_lowercase());
    names
}

fn next_clipboard_selection(current: Option<usize>, delta: isize, count: usize) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let Some(current) = current else {
        return Some(if delta < 0 { count - 1 } else { 0 });
    };
    Some((current as isize + delta).rem_euclid(count as isize) as usize)
}

fn clipboard_search_is_current(
    captured_generation: u64,
    current_generation: u64,
    captured_query: &str,
    current_query: &str,
) -> bool {
    captured_generation == current_generation && captured_query == current_query
}
#[allow(clippy::too_many_arguments)]
fn clipboard_query_state_is_current(
    captured_generation: u64,
    current_generation: u64,
    captured_query: &str,
    current_query: &str,
    captured_category: &str,
    current_category: &str,
    captured_scope: ClipboardSearchScope,
    current_scope: ClipboardSearchScope,
) -> bool {
    captured_generation == current_generation
        && captured_query == current_query
        && captured_category == current_category
        && captured_scope == current_scope
}

fn inset_panel_section<T: gpui_kit::Styled>(section: T) -> T {
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardPageRequest {
    Replace,
    Append,
    Prepend,
}

#[derive(Debug, Clone)]
struct CategoryEditorState {
    category_id: Option<String>,
    selected_sources: HashSet<String>,
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::active(cx);
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

    #[cfg(test)]
    fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    #[cfg(test)]
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
    history_worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    application: WeakEntity<ClipboardApplication>,
    session_id: uuid::Uuid,
    target: nxtools_protocol::ForegroundContext,
    entries: Rc<RefCell<Vec<ClipboardItem>>>,
    window_start_offset: usize,
    next_page_cursor: Option<ClipboardPageCursor>,
    summary_counts: HashMap<i64, usize>,
    full_item_request: Option<uuid::Uuid>,
    capture_pause: crate::clipboard::CapturePauseSnapshot,
    open_targets: HashMap<i64, Vec<nxtools_clipboard::ClipboardOpenTarget>>,
    open_targets_pending: bool,
    total_entries: usize,
    has_more_entries: bool,
    page_request_pending: bool,
    status: PanelStatus,
    notification_source: gpui_kit::AnyWindowHandle,
    pending_action: Option<(uuid::Uuid, ClipboardAction, i64)>,
    pending_action_started_at: Option<(uuid::Uuid, Instant)>,
    is_open: bool,
    fixed_display: bool,
    hide_suppressed_until: Instant,
    last_observed_size: gpui_kit::Size<Pixels>,
    size_tracking_enabled: bool,
    categories: Vec<ClipboardCategory>,
    source_process_names: Vec<String>,
    category_editor: Option<CategoryEditorState>,
    active_category_id: String,
    category_scroll: ScrollHandle,
    category_sources: HashMap<String, HashSet<String>>,
    category_request_generation: u64,
    modifier_actions: ClipboardModifierActions,
    text_card_max_height: u32,
    active_modifier_action: Option<(ClipboardClickModifier, ClipboardClickAction)>,
    list_scroll: ListState,
    preview_pointer: Rc<std::cell::Cell<Point<Pixels>>>,
    image_resource_cache: Rc<RefCell<BoundedImageResourceCache>>,
    list_thumbnails: HashMap<String, Option<PathBuf>>,
    thumbnail_pending: HashSet<String>,
    thumbnail_batch_pending: bool,
    source_icon_colors: HashMap<String, Option<[u8; 3]>>,
    icon_colors_pending: bool,
    list_is_scrolled: bool,
    list_width: f32,
    focus_scope: gpui_kit::FocusHandle,
    search_input: Entity<InputState>,
    search_scope: ClipboardSearchScope,
    history_filters: history_filters::HistoryFilters,
    armed_number_key: Option<efficiency::ArmedNumberKey>,
    last_shortcut_rejection: Option<(&'static str, Instant)>,
    selected_entry_index: Option<usize>,
    query_dispatch_generation: u64,
    search_pending: bool,
    query_error: Option<String>,
    edit_input: Entity<TextareaState>,
    edit_rich: Entity<markdown_editor::BlockEditorState>,
    edit_original: String,
    edit_rich_baseline: String,
    edit_source_mode: bool,
    edit_rich_source: String,
    edit_generation: u64,
    edit_save_pending: Option<uuid::Uuid>,
    category_name_input: Entity<InputState>,
    editing_item_id: Option<i64>,
    last_deleted_item_id: Option<i64>,
    dialog_owned_keystroke: bool,
    extension_feedback: HashMap<i64, Entity<extension_feedback::InlineExtensionFeedback>>,
    _subscriptions: Vec<Subscription>,
}
impl ClipboardPanel {
    /// Puts keyboard focus into the clipboard search box so typed text lands
    /// there by default, even when the panel window was re-shown after the OS
    /// handed focus elsewhere.
    pub(crate) fn focus_search_input(&self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.search_input.focus_handle(cx);
        if !focus.is_focused(window) {
            focus.focus(window, cx);
            info!(event = "clipboard.search.focus_restored", session_id = %self.session_id);
            cx.notify();
        }
    }

    pub(crate) fn focus_search_for_session(
        &self,
        session_id: uuid::Uuid,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.session_id == session_id && self.search_input_ready(window, cx) {
            self.focus_search_input(window, cx);
        }
    }

    fn hover_preview_ready(&self) -> bool {
        self.is_open
            && self.category_editor.is_none()
            && self.editing_item_id.is_none()
            && self.armed_number_key.is_none()
            && self.pending_action.is_none()
            && self.pending_action_started_at.is_none()
            && self.full_item_request.is_none()
            && !self.search_pending
            && self.query_error.is_none()
    }

    pub(crate) fn enable_size_tracking(&mut self, window: &Window) {
        // Capture confirmed initial viewport, not hidden HWND defaults.
        self.last_observed_size = window.viewport_size();
        self.size_tracking_enabled = true;
    }

    fn observe_panel_bounds(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.preview_pointer
            .set(window.bounds().origin + window.mouse_position());
        let viewport = window.viewport_size();
        if !self.size_tracking_enabled || !self.is_open || viewport == self.last_observed_size {
            return;
        }
        self.last_observed_size = viewport;
        let session_id = self.session_id;
        let handle = window.window_handle();
        schedule_entity_update(
            self.application.clone(),
            move |application, cx| {
                application.observe_panel_size(session_id, handle, viewport, cx);
            },
            cx,
        );
    }

    pub(crate) fn handle_platform_reconnected(&mut self, cx: &mut Context<Self>) {
        self.full_item_request = None;
        self.abandon_pending_action(
            "Windows 适配器已重连，请重试粘贴",
            "platform_reconnected",
            cx,
        );
    }

    pub(crate) fn handle_platform_disconnected(&mut self, cx: &mut Context<Self>) {
        self.full_item_request = None;
        self.abandon_pending_action(
            "Windows 适配器已断开，请重试粘贴",
            "platform_disconnected",
            cx,
        );
    }

    fn abandon_pending_action(
        &mut self,
        message: &'static str,
        failure_kind: &'static str,
        cx: &mut Context<Self>,
    ) {
        let pending_action = self.pending_action.take();
        let pending_started_at = self.pending_action_started_at.take();
        if pending_action.is_some() || pending_started_at.is_some() {
            let operation_id = pending_action
                .map(|(command_id, _, _)| command_id)
                .or_else(|| pending_started_at.map(|(operation_id, _)| operation_id));
            let elapsed_ms = pending_started_at
                .map(|(_, started_at)| started_at.elapsed().as_millis())
                .unwrap_or_default();
            warn!(
                event = "clipboard.action.failed",
                session_id = %self.session_id,
                correlation_id = ?operation_id,
                operation_id = ?operation_id,
                failure_kind,
                elapsed_ms,
                "clipboard action invalidated by PlatformHost lifecycle"
            );
            self.set_operation_status_for(
                operation_id.unwrap_or_else(uuid::Uuid::new_v4),
                PanelStatus::Error(message.into()),
                cx,
            );
            cx.notify();
        }
    }

    fn set_operation_status(&mut self, status: PanelStatus, cx: &mut Context<Self>) {
        self.set_operation_status_for(uuid::Uuid::new_v4(), status, cx);
    }

    fn set_operation_status_for(
        &mut self,
        operation_id: uuid::Uuid,
        status: PanelStatus,
        cx: &mut Context<Self>,
    ) {
        let level = match &status {
            PanelStatus::Ready(_) => return,
            PanelStatus::Notice(_) => crate::notifications::NotificationLevel::Success,
            PanelStatus::Pending(_) => crate::notifications::NotificationLevel::Progress,
            PanelStatus::Error(_) => crate::notifications::NotificationLevel::Error,
        };
        let message = status.label().to_owned();
        self.status = status;
        let source = self.notification_source;
        let id = format!("clipboard-operation-{}-{operation_id}", self.session_id);
        if let Some(center) = crate::notifications::current(cx) {
            cx.defer(move |cx| {
                center.update(cx, |center, cx| {
                    center.push(
                        crate::notifications::AppNotification::new(id, level, "剪贴板", message)
                            .in_window(source),
                        cx,
                    )
                });
            });
        }
    }
    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) -> bool {
        let Some(next) = next_clipboard_selection(
            self.selected_entry_index,
            delta,
            self.entries.borrow().len(),
        ) else {
            return false;
        };
        self.selected_entry_index = Some(next);
        self.list_scroll.scroll_to_reveal_item(next);
        cx.notify();
        true
    }

    fn request_visible_thumbnails(&mut self, paths: Vec<String>, cx: &mut Context<Self>) {
        if self.thumbnail_batch_pending || !self.is_open {
            return;
        }
        let paths = paths
            .into_iter()
            .filter(|path| {
                !path.is_empty()
                    && !self.list_thumbnails.contains_key(path)
                    && !self.thumbnail_pending.contains(path)
            })
            .take(4)
            .collect::<Vec<_>>();
        if paths.is_empty() {
            return;
        }
        self.thumbnail_batch_pending = true;
        self.thumbnail_pending.extend(paths.iter().cloned());
        let worker = self.history_worker.clone();
        let generation = self.query_dispatch_generation;
        cx.spawn(async move |panel, cx| {
            let result = worker
                .list_thumbnails_async(paths.iter().map(PathBuf::from).collect())
                .await;
            let _ = panel.update(cx, |panel, cx| {
                panel.thumbnail_batch_pending = false;
                for path in &paths {
                    panel.thumbnail_pending.remove(path);
                }
                if !panel.is_open || panel.query_dispatch_generation != generation {
                    cx.notify();
                    return;
                }
                let active = panel
                    .entries
                    .borrow()
                    .iter()
                    .map(|item| item.image_path.clone())
                    .collect::<HashSet<_>>();
                panel
                    .list_thumbnails
                    .retain(|path, _| active.contains(path));
                if let Ok(results) = result {
                    for (path, thumbnail) in results {
                        let path = path.to_string_lossy().into_owned();
                        if active.contains(&path) {
                            panel.list_thumbnails.insert(path, thumbnail);
                        }
                    }
                } else {
                    for path in paths {
                        if active.contains(&path) {
                            panel.list_thumbnails.insert(path, None);
                        }
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn activate_selected_from_keyboard(
        &mut self,
        action: ClipboardAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(index) = self.selected_entry_index else {
            return false;
        };
        let Some(entry) = self.entries.borrow().get(index).cloned() else {
            return false;
        };
        self.activate_entry(&entry, action, window, cx);
        true
    }

    fn schedule_search(&mut self, query: String, generation: u64, cx: &mut Context<Self>) {
        let panel = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            cx.background_executor()
                .timer(CLIPBOARD_SEARCH_DEBOUNCE)
                .await;
            let _ = panel.update(cx, |panel, cx| {
                let current_query = panel.search_input.read(cx).value().to_string();
                if !panel.is_open
                    || !clipboard_search_is_current(
                        generation,
                        panel.query_dispatch_generation,
                        &query,
                        &current_query,
                    )
                {
                    return;
                }
                panel.request_entries(
                    query.clone(),
                    generation,
                    0,
                    ClipboardPageRequest::Replace,
                    None,
                    None,
                    cx,
                );
            });
        })
        .detach();
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        page: ClipboardHistoryPage,
        categories: Vec<ClipboardCategory>,
        category_sources: HashMap<String, HashSet<String>>,
        source_process_names: Vec<String>,
        modifier_actions: ClipboardModifierActions,
        target: nxtools_protocol::ForegroundContext,
        history_worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
        application: WeakEntity<ClipboardApplication>,
        session_id: uuid::Uuid,
        initial_error: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        markdown_document::init(cx);
        let search_input = cx.new(|cx| InputState::new(window, cx).placeholder("搜索剪贴板历史"));
        search_input.focus_handle(cx).focus(window, cx);
        let edit_input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(5, 12)
                .placeholder("输入剪贴板文本")
        });
        let edit_rich = cx.new(|cx| {
            markdown_editor::BlockEditorState::new(window, cx)
                .with_image_policy(markdown_editor::ImagePolicy::Placeholder)
        });
        let category_name_input = cx.new(|cx| InputState::new(window, cx).placeholder("分类名称"));

        let search_subscription = cx.subscribe_in(
            &search_input,
            window,
            |this, state, event: &InputEvent, _window, cx| {
                if matches!(event, InputEvent::Change) {
                    this.dismiss_preview_for_input("search_changed", cx);
                    let query = state.read(cx).value().to_string();
                    this.query_dispatch_generation =
                        this.query_dispatch_generation.wrapping_add(1).max(1);
                    let generation = this.query_dispatch_generation;
                    this.search_pending = true;
                    this.schedule_search(query, generation, cx);
                    cx.notify();
                }
            },
        );
        let key_listener = cx.listener(|this, event: &KeystrokeEvent, window, cx| {
            if window.window_handle() != this.notification_source {
                return;
            }
            this.dialog_owned_keystroke = window.has_active_dialog(cx);
            if this.dialog_owned_keystroke {
                return;
            }
            if !this.history_keyboard_ready(window, cx) || this.search_is_composing(window, cx) {
                if window.window_handle() == this.notification_source {
                    this.log_rejected_number_shortcut(&event.keystroke, window, cx);
                }
                return;
            }
            if window.focused(cx).is_none() {
                this.focus_search_input(window, cx);
                let key_event = gpui_kit::KeyDownEvent {
                    keystroke: event.keystroke.clone(),
                    is_held: this.armed_number_key.is_some(),
                    prefer_character_input: false,
                };
                this.number_key_down(&key_event, window, cx);
                // The dispatch path was captured before restoring focus.
                if this.armed_number_key.is_some() {
                    cx.stop_propagation();
                    return;
                }
            }
            let key = event.keystroke.key.as_str();
            if event.keystroke.modifiers.modified()
                && !(key.eq_ignore_ascii_case("enter") && event.keystroke.modifiers.control)
            {
                return;
            }
            if !matches!(key.to_ascii_lowercase().as_str(), "pageup" | "pagedown")
                && !this.search_input.focus_handle(cx).is_focused(window)
            {
                return;
            }
            let handled = match key.to_ascii_lowercase().as_str() {
                "up" => this.move_selection(-1, cx),
                "down" => this.move_selection(1, cx),
                "pageup" => this.move_page(-1, window, cx),
                "pagedown" => this.move_page(1, window, cx),
                "enter" => this.activate_selected_from_keyboard(
                    if event.keystroke.modifiers.control {
                        ClipboardAction::CopyOnly
                    } else {
                        ClipboardAction::OriginalPaste
                    },
                    window,
                    cx,
                ),
                _ => false,
            };
            if handled {
                cx.stop_propagation();
            }
        });
        let navigation_subscription = cx.intercept_keystrokes(key_listener);
        let keystroke_subscription = cx.observe_keystrokes(|this, event, window, cx| {
            if window.window_handle() != this.notification_source {
                return;
            }
            if std::mem::take(&mut this.dialog_owned_keystroke) || window.has_active_dialog(cx) {
                return;
            }
            if event.keystroke.key.eq_ignore_ascii_case("escape") {
                this.full_item_request = None;
                if this.category_editor.is_some() {
                    this.cancel_category_editor(window, cx);
                } else if this.editing_item_id.is_some() {
                    this.cancel_edit(window, cx);
                } else {
                    this.close_with_reason(window, cx, PopupCloseReason::Escape);
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

        let activation_subscription = cx.observe_window_activation(window, |panel, window, cx| {
            if window.is_window_active() && panel.search_input_ready(window, cx) {
                panel.focus_search_input(window, cx);
            }
        });
        let focus_scope = cx.focus_handle();
        let focus_lost_subscription = cx.on_focus_lost(window, |panel, window, cx| {
            // A strong handle can outlive its row. Restore only while this panel is
            // still the mounted ancestor, never after a different owner replaced it.
            if window.focus_lost_restore_target(cx).as_ref() == Some(&panel.focus_scope)
                && panel.search_input_ready(window, cx)
            {
                panel.focus_search_input(window, cx);
            }
        });
        let bounds_subscription = cx.observe_window_bounds(window, |panel, window, cx| {
            panel.observe_panel_bounds(window, cx);
        });
        let mut status = if target.process_name.is_empty() {
            PanelStatus::Ready("选择一条记录进行粘贴".into())
        } else {
            PanelStatus::Ready(format!("将粘贴到 {}", target.process_name))
        };
        if let Some(error) = initial_error.clone() {
            status = PanelStatus::Error(error);
        }
        let entries = page.items;
        let source_process_names = if source_process_names.is_empty() {
            source_process_names_from_entries(&entries)
        } else {
            source_process_names
        };
        let next_page_cursor = entries.last().map(ClipboardPageCursor::from);
        let list_scroll = ListState::new(entries.len(), ListAlignment::Top, px(180.0));
        let panel = cx.entity().downgrade();
        let preview_application = application.clone();
        list_scroll.set_scroll_handler(move |event, _window, app| {
            schedule_entity_update(
                preview_application.clone(),
                |application, cx| application.suspend_preview_for_scroll(cx),
                app,
            );
            let load_request = panel.update(app, |panel, cx| {
                if panel.list_is_scrolled != event.is_scrolled {
                    panel.list_is_scrolled = event.is_scrolled;
                    cx.notify();
                }
                let should_load_previous = panel.window_start_offset > 0
                    && !panel.page_request_pending
                    && !panel.search_pending
                    && event.visible_range.start <= HISTORY_PAGE_PREFETCH_ITEMS;
                let should_load_next = panel.has_more_entries
                    && !panel.page_request_pending
                    && !panel.search_pending
                    && event
                        .visible_range
                        .end
                        .saturating_add(HISTORY_PAGE_PREFETCH_ITEMS)
                        >= event.count;
                let request = if should_load_previous {
                    Some(ClipboardPageRequest::Prepend)
                } else if should_load_next {
                    Some(ClipboardPageRequest::Append)
                } else {
                    None
                };
                if request.is_some() {
                    panel.page_request_pending = true;
                }
                request
            });
            match load_request {
                Ok(Some(ClipboardPageRequest::Append)) => {
                    schedule_entity_update(
                        panel.clone(),
                        |panel, cx| panel.load_next_page(cx),
                        app,
                    );
                }
                Ok(Some(ClipboardPageRequest::Prepend)) => {
                    schedule_entity_update(
                        panel.clone(),
                        |panel, cx| panel.load_previous_page(cx),
                        app,
                    );
                }
                _ => {}
            }
        });
        let category_sources = category_sources;
        let active_category_id = categories
            .iter()
            .find(|category| category.id == BUILTIN_ALL_CATEGORY_ID)
            .or_else(|| categories.first())
            .map(|category| category.id.clone())
            .unwrap_or_else(|| BUILTIN_ALL_CATEGORY_ID.into());
        let has_entries = !entries.is_empty();
        let notification_subscription = crate::notifications::current(cx)
            .map(|center| cx.observe(&center, |_, _, cx| cx.notify()));
        Self {
            history_worker,
            theme: Theme::zed_dark(),
            application,
            session_id,
            target,
            entries: Rc::new(RefCell::new(entries)),
            window_start_offset: 0,
            next_page_cursor,
            total_entries: page.total,
            summary_counts: HashMap::new(),
            full_item_request: None,
            capture_pause: crate::clipboard::CapturePauseSnapshot::default(),
            open_targets: HashMap::new(),
            open_targets_pending: false,
            has_more_entries: page.has_more,
            page_request_pending: false,
            notification_source: window.window_handle(),
            status,
            pending_action: None,
            pending_action_started_at: None,
            is_open: true,
            fixed_display: false,
            categories,
            hide_suppressed_until: Instant::now() + POPUP_OPEN_GRACE,
            last_observed_size: window.viewport_size(),
            size_tracking_enabled: false,
            category_sources,
            source_process_names,
            active_category_id,
            category_request_generation: 0,
            category_editor: None,
            category_scroll: ScrollHandle::new(),
            modifier_actions,
            text_card_max_height: nxtools_core::default_clipboard_text_card_max_height(),
            active_modifier_action: None,
            list_scroll,
            preview_pointer: Rc::new(std::cell::Cell::new(
                window.bounds().origin + window.mouse_position(),
            )),
            image_resource_cache: Rc::new(RefCell::new(BoundedImageResourceCache::new(
                IMAGE_RESOURCE_CACHE_CAPACITY,
            ))),
            list_thumbnails: HashMap::new(),
            thumbnail_pending: HashSet::new(),
            thumbnail_batch_pending: false,
            source_icon_colors: HashMap::new(),
            icon_colors_pending: false,
            list_is_scrolled: false,
            list_width: 0.0,
            focus_scope,
            search_input,
            search_scope: ClipboardSearchScope::Content,
            history_filters: history_filters::HistoryFilters::default(),
            query_dispatch_generation: 0,
            selected_entry_index: has_entries.then_some(0),
            search_pending: false,
            armed_number_key: None,
            last_shortcut_rejection: None,
            query_error: initial_error,
            edit_input,
            edit_rich,
            edit_original: String::new(),
            edit_rich_baseline: String::new(),
            edit_rich_source: String::new(),
            edit_source_mode: false,
            edit_generation: 0,
            edit_save_pending: None,
            category_name_input,
            editing_item_id: None,
            last_deleted_item_id: None,
            dialog_owned_keystroke: false,
            extension_feedback: HashMap::new(),
            _subscriptions: vec![
                search_subscription,
                keystroke_subscription,
                navigation_subscription,
                bounds_subscription,
                activation_subscription,
                focus_lost_subscription,
            ]
            .into_iter()
            .chain(notification_subscription)
            .collect(),
        }
    }

    #[cfg(test)]
    pub(crate) fn text_card_max_height_for_test(&self) -> u32 {
        self.text_card_max_height
    }

    pub(crate) fn with_text_card_max_height(mut self, height: u32) -> Self {
        self.text_card_max_height = height.clamp(
            nxtools_core::CLIPBOARD_TEXT_CARD_MIN_HEIGHT,
            nxtools_core::CLIPBOARD_TEXT_CARD_MAX_HEIGHT,
        );
        self
    }

    pub(crate) fn set_text_card_max_height(&mut self, height: u32, cx: &mut Context<Self>) {
        let height = height.clamp(
            nxtools_core::CLIPBOARD_TEXT_CARD_MIN_HEIGHT,
            nxtools_core::CLIPBOARD_TEXT_CARD_MAX_HEIGHT,
        );
        if self.text_card_max_height != height {
            self.text_card_max_height = height;
            let scroll_top = self.list_scroll.logical_scroll_top();
            self.list_scroll.reset(self.entries.borrow().len());
            self.list_scroll.scroll_to(scroll_top);
            cx.notify();
        }
    }

    fn update_next_page_cursor(&mut self) {
        self.next_page_cursor = self.entries.borrow().last().map(ClipboardPageCursor::from);
    }

    fn trim_history_window_from_front(&mut self) {
        let current_len = self.entries.borrow().len();
        let removed = current_len.saturating_sub(HISTORY_WINDOW_SIZE);
        if removed == 0 {
            return;
        }
        self.entries.borrow_mut().drain(..removed);
        self.list_scroll.splice(0..removed, 0);
        self.window_start_offset = self.window_start_offset.saturating_add(removed);
        self.selected_entry_index = self
            .selected_entry_index
            .and_then(|index| index.checked_sub(removed));
        self.update_next_page_cursor();
    }

    fn trim_history_window_from_back(&mut self) {
        let current_len = self.entries.borrow().len();
        let keep_len = current_len.min(HISTORY_WINDOW_SIZE);
        let removed = current_len.saturating_sub(keep_len);
        if removed == 0 {
            return;
        }
        self.entries.borrow_mut().truncate(keep_len);
        self.list_scroll.splice(keep_len..current_len, 0);
        if self
            .selected_entry_index
            .is_some_and(|index| index >= keep_len)
        {
            self.selected_entry_index = None;
        }
        self.update_next_page_cursor();
    }

    #[allow(clippy::too_many_arguments)]
    fn request_entries(
        &mut self,
        query: String,
        generation: u64,
        offset: usize,
        request: ClipboardPageRequest,
        known_total: Option<usize>,
        cursor: Option<ClipboardPageCursor>,
        cx: &mut Context<Self>,
    ) {
        let category_id = self.active_category_id.clone();
        let search_scope = self.search_scope;
        let filters = self.history_filters.query.clone();
        let filter_diagnostic = self.history_filters.pending.take();
        let replacing = request == ClipboardPageRequest::Replace;
        self.query_error = None;
        if replacing {
            self.search_pending = true;
            self.page_request_pending = false;
            self.status = PanelStatus::Pending("正在搜索…");
        } else {
            self.page_request_pending = true;
            self.status = PanelStatus::Pending("正在加载更多…");
        }
        let worker = self.history_worker.clone();
        let panel = cx.entity().downgrade();
        let captured_category = category_id.clone();
        cx.spawn(async move |_, cx| {
            let result = worker
                .query_summary_page_async(
                    &query,
                    &category_id,
                    search_scope,
                    offset,
                    HISTORY_PAGE_SIZE,
                    known_total,
                    cursor,
                    filters,
                )
                .await;
            if let Some((id, started)) = filter_diagnostic {
                let outcome = match &result {
                    Ok(_) => "loaded",
                    Err(ClipboardHistoryError::QuerySuperseded) => "superseded",
                    Err(_) => "failed",
                };
                info!(event = "clipboard.history_filter.finished", correlation_id = %id,
                    outcome, elapsed_ms = started.elapsed().as_millis() as u64);
            }
            let _ = panel.update(cx, |panel, cx| {
                let current_query = panel.search_input.read(cx).value().to_string();
                if !clipboard_query_state_is_current(
                    generation,
                    panel.query_dispatch_generation,
                    &query,
                    &current_query,
                    &captured_category,
                    &panel.active_category_id,
                    search_scope,
                    panel.search_scope,
                ) {
                    return;
                }
                let request_is_stale = match request {
                    ClipboardPageRequest::Replace => false,
                    ClipboardPageRequest::Append => {
                        offset
                            != panel
                                .window_start_offset
                                .saturating_add(panel.entries.borrow().len())
                    }
                    ClipboardPageRequest::Prepend => offset >= panel.window_start_offset,
                };
                if request_is_stale {
                    panel.page_request_pending = false;
                    panel.status = PanelStatus::Ready(String::new());
                    cx.notify();
                    return;
                }
                let result = result.map(|page| panel.unpack_summary_page(page));
                match result {
                    Ok(page) => match request {
                        ClipboardPageRequest::Append => {
                            let appended = page.items.len();
                            let old_len = panel.entries.borrow().len();
                            panel.entries.borrow_mut().extend(page.items);
                            if appended > 0 {
                                panel.list_scroll.splice(old_len..old_len, appended);
                            }
                            panel.total_entries = page.total;
                            panel.page_request_pending = false;
                            panel.query_error = None;
                            panel.status = PanelStatus::Ready(String::new());
                            panel.trim_history_window_from_front();
                            panel.update_next_page_cursor();
                            panel.has_more_entries = panel
                                .window_start_offset
                                .saturating_add(panel.entries.borrow().len())
                                < panel.total_entries;
                        }
                        ClipboardPageRequest::Prepend => {
                            let prepended = page.items.len();
                            panel.entries.borrow_mut().splice(0..0, page.items);
                            if prepended > 0 {
                                panel.list_scroll.splice(0..0, prepended);
                                panel.selected_entry_index = panel
                                    .selected_entry_index
                                    .map(|index| index.saturating_add(prepended));
                            }
                            panel.window_start_offset = offset;
                            panel.total_entries = page.total;
                            panel.page_request_pending = false;
                            panel.query_error = None;
                            panel.status = PanelStatus::Ready(String::new());
                            panel.trim_history_window_from_back();
                            panel.update_next_page_cursor();
                            panel.has_more_entries = panel
                                .window_start_offset
                                .saturating_add(panel.entries.borrow().len())
                                < panel.total_entries;
                        }
                        ClipboardPageRequest::Replace => {
                            panel.open_targets.clear();
                            let has_entries = !page.items.is_empty();
                            let next_page_cursor = page.items.last().map(ClipboardPageCursor::from);
                            *panel.entries.borrow_mut() = page.items;
                            panel.window_start_offset = 0;
                            panel.next_page_cursor = next_page_cursor;
                            panel.total_entries = page.total;
                            panel.has_more_entries = page.has_more;
                            panel.page_request_pending = false;
                            panel.search_pending = false;
                            panel.selected_entry_index = has_entries.then_some(0);
                            panel.list_scroll.reset(panel.entries.borrow().len());
                            panel.list_is_scrolled = false;
                            panel.query_error = None;
                            panel.status = PanelStatus::Ready(String::new());
                        }
                    },
                    Err(ClipboardHistoryError::QuerySuperseded) => {
                        panel.page_request_pending = false;
                        if replacing {
                            panel.search_pending = false;
                        }
                        panel.query_error = None;
                        panel.status = PanelStatus::Ready(String::new());
                    }
                    Err(error) => {
                        let message = error.to_string();
                        error!(%error, offset, ?request, "clipboard worker query failed");
                        panel.page_request_pending = false;
                        panel.search_pending = false;
                        panel.query_error = Some(message.clone());
                        if replacing {
                            panel.selected_entry_index = None;
                            *panel.entries.borrow_mut() = Vec::new();
                            panel.window_start_offset = 0;
                            panel.next_page_cursor = None;
                            panel.total_entries = 0;
                            panel.has_more_entries = false;
                            panel.list_scroll.reset(0);
                        }
                        panel.status = PanelStatus::Error(if replacing {
                            "查询失败，请查看日志".into()
                        } else {
                            format!("加载更多失败：{message}")
                        });
                    }
                }
                panel.prune_list_metadata();
                cx.notify();
            });
        })
        .detach();
    }

    fn load_next_page(&mut self, cx: &mut Context<Self>) {
        if !self.has_more_entries {
            self.page_request_pending = false;
            return;
        }
        let query = self.search_input.read(cx).value().to_string();
        let generation = self.query_dispatch_generation;
        let offset = self
            .window_start_offset
            .saturating_add(self.entries.borrow().len());
        let cursor = self.next_page_cursor;
        self.request_entries(
            query,
            generation,
            offset,
            ClipboardPageRequest::Append,
            Some(self.total_entries),
            cursor,
            cx,
        );
    }

    fn load_previous_page(&mut self, cx: &mut Context<Self>) {
        if self.window_start_offset == 0 {
            self.page_request_pending = false;
            return;
        }
        let query = self.search_input.read(cx).value().to_string();
        let generation = self.query_dispatch_generation;
        let offset = self.window_start_offset.saturating_sub(HISTORY_PAGE_SIZE);
        self.request_entries(
            query,
            generation,
            offset,
            ClipboardPageRequest::Prepend,
            Some(self.total_entries),
            None,
            cx,
        );
    }

    fn toggle_search_scope(&mut self, cx: &mut Context<Self>) {
        self.search_scope = match self.search_scope {
            ClipboardSearchScope::Content => ClipboardSearchScope::Extended,
            ClipboardSearchScope::Extended => ClipboardSearchScope::Content,
        };
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        self.search_pending = false;
        self.page_request_pending = false;
        let query = self.search_input.read(cx).value().to_string();
        let generation = self.query_dispatch_generation;
        self.request_entries(
            query,
            generation,
            0,
            ClipboardPageRequest::Replace,
            None,
            None,
            cx,
        );
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
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        self.search_pending = false;
        self.page_request_pending = false;
        let query = self.search_input.read(cx).value().to_string();
        let generation = self.query_dispatch_generation;
        self.request_entries(
            query,
            generation,
            0,
            ClipboardPageRequest::Replace,
            None,
            None,
            cx,
        );
        cx.notify();
    }

    fn active_category(&self) -> Option<&ClipboardCategory> {
        self.categories
            .iter()
            .find(|category| category.id == self.active_category_id)
    }

    fn category_source_choices(&self) -> Vec<String> {
        let mut choices = HashMap::new();
        for source in &self.source_process_names {
            let source = source.trim();
            if !source.is_empty() {
                choices
                    .entry(source.to_lowercase())
                    .or_insert_with(|| source.to_owned());
            }
        }
        for sources in self.category_sources.values() {
            for source in sources {
                let source = source.trim();
                if !source.is_empty() {
                    choices
                        .entry(source.to_lowercase())
                        .or_insert_with(|| source.to_owned());
                }
            }
        }
        let mut choices = choices.into_values().collect::<Vec<_>>();
        choices.sort_by_key(|source| source.to_lowercase());
        choices
    }

    fn toggle_category_editor_source(&mut self, process_name: &str, cx: &mut Context<Self>) {
        let process_name = process_name.trim().to_lowercase();
        if process_name.is_empty() {
            return;
        }
        let Some(editor) = self.category_editor.as_mut() else {
            return;
        };
        if !editor.selected_sources.remove(&process_name) {
            editor.selected_sources.insert(process_name);
        }
        cx.notify();
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
        let category_id = category.map(|category| category.id.clone());
        let selected_sources = category_id
            .as_deref()
            .and_then(|category_id| self.category_sources.get(category_id))
            .cloned()
            .unwrap_or_default();
        self.editing_item_id = None;
        self.category_editor = Some(CategoryEditorState {
            category_id,
            selected_sources,
        });
        schedule_entity_update(
            self.application.clone(),
            |application, cx| application.dismiss_preview(cx),
            cx,
        );
        self.category_name_input
            .update(cx, |state, cx| state.set_value(value, window, cx));
        self.category_name_input.focus_handle(cx).focus(window, cx);
        cx.notify();
    }

    fn cancel_category_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.category_editor = None;
        self.search_input.focus_handle(cx).focus(window, cx);
        cx.notify();
    }

    fn save_category_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = self.category_editor.clone() else {
            return;
        };
        let name = self.category_name_input.read(cx).value().to_string();
        let selected_sources = editor.selected_sources.iter().cloned().collect::<Vec<_>>();
        let source_count = selected_sources.len();
        let operation_id = uuid::Uuid::new_v4();
        let category_id_for_log = editor.category_id.clone().unwrap_or_default();
        let worker = self.history_worker.clone();
        let panel = cx.entity().downgrade();
        let window_handle = window.window_handle();
        self.set_operation_status_for(operation_id, PanelStatus::Pending("正在保存分类…"), cx);
        info!(
            correlation_id = %operation_id,
            category_id = %category_id_for_log,
            source_count,
            "clipboard.category.save_started"
        );
        cx.spawn(async move |_, cx| {
            let result = async {
                let category = if let Some(category_id) = editor.category_id.as_deref() {
                    worker.rename_category_async(category_id, &name).await?
                } else {
                    worker.create_category_async(&name).await?
                };
                worker
                    .replace_category_sources_async(&category.id, &selected_sources)
                    .await?;
                Ok::<_, ClipboardHistoryError>(category)
            }
            .await;
            let _ = panel.update(cx, |panel, cx| {
                match result {
                    Ok(category) => {
                        info!(
                            correlation_id = %operation_id,
                            category_id = %category.id,
                            source_count,
                            "clipboard.category.save_completed"
                        );
                        panel.category_editor = None;
                        panel.active_category_id = category.id;
                        panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Notice("分类已保存".into()),
                            cx,
                        );
                        panel.reload_categories(cx);
                        let panel_for_focus = cx.entity().downgrade();
                        cx.defer(move |cx| {
                            let _ = window_handle.update(cx, |_, window, app| {
                                let _ = panel_for_focus.update(app, |panel, cx| {
                                    panel.search_input.focus_handle(cx).focus(window, cx);
                                });
                            });
                        });
                    }
                    Err(error) => {
                        error!(
                            correlation_id = %operation_id,
                            category_id = %category_id_for_log,
                            source_count,
                            error = %error,
                            "clipboard.category.save_failed"
                        );
                        panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Error(format!("保存分类失败：{error}")),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn delete_category(&mut self, category_id: &str, cx: &mut Context<Self>) {
        let category_id = category_id.to_owned();
        let worker = self.history_worker.clone();
        let panel = cx.entity().downgrade();
        let operation_id = uuid::Uuid::new_v4();
        self.set_operation_status_for(operation_id, PanelStatus::Pending("正在删除分类…"), cx);
        cx.spawn(async move |_, cx| {
            let result = worker.delete_category_async(&category_id).await;
            let _ = panel.update(cx, |panel, cx| {
                match result {
                    Ok(()) => {
                        if panel.active_category_id == category_id {
                            panel.active_category_id = BUILTIN_ALL_CATEGORY_ID.into();
                        }
                        panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Notice("分类已删除".into()),
                            cx,
                        );
                        panel.reload_categories(cx);
                    }
                    Err(error) => {
                        error!(%error, "failed to delete clipboard category");
                        panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Error(format!("删除分类失败：{error}")),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
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
        let worker = self.history_worker.clone();
        let panel = cx.entity().downgrade();
        let operation_id = uuid::Uuid::new_v4();
        self.set_operation_status_for(operation_id, PanelStatus::Pending("正在保存分类排序…"), cx);
        cx.spawn(async move |_, cx| {
            let result = worker.reorder_categories_async(&ordered_ids).await;
            let _ = panel.update(cx, |panel, cx| {
                match result {
                    Ok(categories) => {
                        panel.categories = categories;
                        panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Notice("分类排序已保存".into()),
                            cx,
                        );
                    }
                    Err(error) => {
                        error!(%error, "failed to reorder clipboard categories");
                        panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Error(format!("分类排序失败：{error}")),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn reload_categories(&mut self, cx: &mut Context<Self>) {
        self.category_request_generation = self.category_request_generation.wrapping_add(1).max(1);
        let generation = self.category_request_generation;
        self.status = PanelStatus::Pending("正在加载分类…");
        let worker = self.history_worker.clone();
        let panel = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = async {
                let categories = worker.categories_async().await?;
                let mut sources = HashMap::new();
                for category in categories
                    .iter()
                    .filter(|category| category.kind == ClipboardCategoryKind::Custom)
                {
                    let values = worker.category_sources_async(&category.id).await?;
                    sources.insert(
                        category.id.clone(),
                        values
                            .into_iter()
                            .map(|source| source.to_lowercase())
                            .collect(),
                    );
                }
                let source_process_names = worker.source_process_names_async().await?;
                Ok::<_, ClipboardHistoryError>((categories, sources, source_process_names))
            }
            .await;
            let _ = panel.update(cx, |panel, cx| {
                if panel.category_request_generation != generation {
                    return;
                }
                match result {
                    Ok((categories, sources, source_process_names)) => {
                        panel.categories = categories;
                        panel.category_sources = sources;
                        panel.source_process_names = source_process_names;
                        if !panel
                            .categories
                            .iter()
                            .any(|category| category.id == panel.active_category_id)
                        {
                            panel.active_category_id = BUILTIN_ALL_CATEGORY_ID.into();
                        }
                        panel.status = PanelStatus::Ready(String::new());
                        panel.refresh_entries(cx);
                    }
                    Err(error) => {
                        error!(%error, "failed to load clipboard categories");
                        panel.set_operation_status(
                            PanelStatus::Error("分类加载失败，请重试".into()),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn refresh_entries(&mut self, cx: &mut Context<Self>) {
        self.query_dispatch_generation = self.query_dispatch_generation.wrapping_add(1).max(1);
        self.search_pending = false;
        self.page_request_pending = false;
        let query = self.search_input.read(cx).value().to_string();
        let generation = self.query_dispatch_generation;
        self.request_entries(
            query,
            generation,
            0,
            ClipboardPageRequest::Replace,
            None,
            None,
            cx,
        );
    }

    fn handle_item_action(
        &mut self,
        entry: &ClipboardItem,
        action: ClipboardItemAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match action {
            ClipboardItemAction::RunExtension { action } => {
                if self.summary_counts.contains_key(&entry.id) {
                    self.load_full_item_for_management(
                        entry.id,
                        ClipboardItemAction::RunExtension { action },
                        window,
                        cx,
                    );
                    return;
                }
                let Some(context) = crate::extensions::manual_actions::clipboard_context(entry)
                else {
                    return;
                };
                let origin = crate::extensions::manual_actions::ClipboardOrigin {
                    panel: cx.entity().downgrade(),
                    session_id: self.session_id,
                    window: window.window_handle(),
                    item_id: entry.id,
                };
                let source = origin.window;
                cx.defer(move |cx| {
                    let _ = source.update(cx, |_, window, cx| {
                        crate::extensions::manual_actions::invoke_selected(
                            "clipboard",
                            action,
                            context,
                            Some(origin),
                            window,
                            cx,
                        );
                    });
                });
            }
            ClipboardItemAction::Edit => self.begin_edit(entry, window, cx),
            ClipboardItemAction::TogglePinned => {
                let worker = self.history_worker.clone();
                let panel = cx.entity().downgrade();
                let item_id = entry.id;
                let value = !entry.is_pinned;
                let operation_id = uuid::Uuid::new_v4();
                self.set_operation_status_for(
                    operation_id,
                    PanelStatus::Pending("正在更新置顶…"),
                    cx,
                );
                cx.spawn(async move |_, cx| {
                    let result = worker.set_pinned_async(item_id, value).await;
                    let _ = panel.update(cx, |panel, cx| {
                        match result {
                            Ok(_) => {
                                panel.set_operation_status_for(
                                    operation_id,
                                    PanelStatus::Notice(if value {
                                        "已置顶".into()
                                    } else {
                                        "已取消置顶".into()
                                    }),
                                    cx,
                                );
                                panel.refresh_entries(cx);
                            }
                            Err(error) => {
                                error!(%error, "failed to update clipboard pinned state");
                                panel.set_operation_status_for(
                                    operation_id,
                                    PanelStatus::Error(format!("置顶失败：{error}")),
                                    cx,
                                );
                            }
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
            ClipboardItemAction::ToggleFavorite => {
                let worker = self.history_worker.clone();
                let panel = cx.entity().downgrade();
                let item_id = entry.id;
                let value = !entry.is_favorite;
                let operation_id = uuid::Uuid::new_v4();
                self.set_operation_status_for(
                    operation_id,
                    PanelStatus::Pending("正在更新收藏…"),
                    cx,
                );
                cx.spawn(async move |_, cx| {
                    let result = worker.set_favorite_async(item_id, value).await;
                    let _ = panel.update(cx, |panel, cx| {
                        match result {
                            Ok(_) => {
                                panel.set_operation_status_for(
                                    operation_id,
                                    PanelStatus::Notice(if value {
                                        "已收藏".into()
                                    } else {
                                        "已取消收藏".into()
                                    }),
                                    cx,
                                );
                                panel.refresh_entries(cx);
                            }
                            Err(error) => {
                                error!(%error, "failed to update clipboard favorite state");
                                panel.set_operation_status_for(
                                    operation_id,
                                    PanelStatus::Error(format!("收藏失败：{error}")),
                                    cx,
                                );
                            }
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
            ClipboardItemAction::Delete => {
                let worker = self.history_worker.clone();
                let panel = cx.entity().downgrade();
                let item_id = entry.id;
                let operation_id = uuid::Uuid::new_v4();
                self.set_operation_status_for(operation_id, PanelStatus::Pending("正在删除…"), cx);
                cx.spawn(async move |_, cx| {
                    let result = worker
                        .soft_delete_async(item_id, nxtools_ipc::unix_time_ms())
                        .await;
                    let _ = panel.update(cx, |panel, cx| {
                        match result {
                            Ok(()) => {
                                panel.last_deleted_item_id = Some(item_id);
                                panel.set_operation_status_for(
                                    operation_id,
                                    PanelStatus::Notice("已删除，可在底部撤销".into()),
                                    cx,
                                );
                                panel.refresh_entries(cx);
                            }
                            Err(error) => {
                                error!(%error, "failed to delete clipboard item");
                                panel.set_operation_status_for(
                                    operation_id,
                                    PanelStatus::Error(format!("删除失败：{error}")),
                                    cx,
                                );
                            }
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
            ClipboardItemAction::ToggleCategorySource {
                category_id,
                remove,
            } => {
                let process_name = entry.source.process_name.trim().to_owned();
                if process_name.is_empty() {
                    self.set_operation_status(
                        PanelStatus::Error("该条目没有可用的来源程序".into()),
                        cx,
                    );
                    cx.notify();
                    return;
                }
                let worker = self.history_worker.clone();
                let panel = cx.entity().downgrade();
                let operation_id = uuid::Uuid::new_v4();
                self.set_operation_status_for(
                    operation_id,
                    PanelStatus::Pending("正在更新来源分类…"),
                    cx,
                );
                cx.spawn(async move |_, cx| {
                    let result = if remove {
                        worker
                            .remove_category_source_async(&category_id, &process_name)
                            .await
                    } else {
                        worker
                            .add_category_source_async(&category_id, &process_name)
                            .await
                    };
                    let _ = panel.update(cx, |panel, cx| {
                        match result {
                            Ok(sources) => {
                                panel.category_sources.insert(
                                    category_id.clone(),
                                    sources
                                        .into_iter()
                                        .map(|source| source.to_lowercase())
                                        .collect(),
                                );
                                if panel.active_category_id == category_id {
                                    panel.refresh_entries(cx);
                                }
                                panel.set_operation_status_for(
                                    operation_id,
                                    PanelStatus::Notice(if remove {
                                        "已从分类移除来源".into()
                                    } else {
                                        "已将来源加入分类".into()
                                    }),
                                    cx,
                                );
                            }
                            Err(error) => {
                                error!(%error, "failed to update clipboard category source");
                                panel.set_operation_status_for(
                                    operation_id,
                                    PanelStatus::Error(format!("更新来源分类失败：{error}")),
                                    cx,
                                );
                            }
                        }
                        cx.notify();
                    });
                })
                .detach();
            }
        }
        cx.notify();
    }

    fn begin_edit(&mut self, entry: &ClipboardItem, window: &mut Window, cx: &mut Context<Self>) {
        if self.summary_counts.contains_key(&entry.id) {
            self.load_full_item_for_management(entry.id, ClipboardItemAction::Edit, window, cx);
            return;
        }
        if entry.kind != ClipboardKind::Text {
            self.set_operation_status(PanelStatus::Error("只有文本条目可以编辑".into()), cx);
            cx.notify();
            return;
        }
        self.category_editor = None;
        self.edit_generation = self.edit_generation.wrapping_add(1);
        self.edit_save_pending = None;
        self.editing_item_id = Some(entry.id);
        self.edit_input.update(cx, |state, cx| {
            state.set_value(entry.text.clone(), window, cx)
        });
        self.prepare_markdown_edit(&entry.text, window, cx);
        cx.notify();
    }

    fn cancel_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.edit_generation = self.edit_generation.wrapping_add(1);
        self.edit_save_pending = None;
        self.full_item_request = None;
        self.editing_item_id = None;
        self.search_input.focus_handle(cx).focus(window, cx);
        cx.notify();
    }

    fn save_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(item_id) = self.editing_item_id else {
            return;
        };
        if self.edit_save_pending.is_some() {
            return;
        }
        if !self.edit_source_mode && self.edit_rich.read(cx).is_composing() {
            return;
        }
        if self.edit_source_mode
            && self.edit_input.update(cx, |state, cx| {
                state.marked_text_range(window, cx).is_some()
            })
        {
            return;
        }
        let text = self.markdown_edit_value(cx);
        if text == self.edit_original {
            self.cancel_edit(window, cx);
            return;
        }
        let worker = self.history_worker.clone();
        let panel = cx.entity().downgrade();
        let window_handle = window.window_handle();
        let operation_id = uuid::Uuid::new_v4();
        let edit_generation = self.edit_generation;
        self.edit_save_pending = Some(operation_id);
        let started = Instant::now();
        info!(event = "clipboard.edit.save.accepted", correlation_id = %operation_id, mode = if self.edit_source_mode { "source" } else { "rich" }, "clipboard edit save accepted");
        self.set_operation_status_for(operation_id, PanelStatus::Pending("正在保存文本…"), cx);
        cx.spawn(async move |_, cx| {
            let result = worker.update_text_async(item_id, &text).await;
            info!(event = "clipboard.edit.save.completed", correlation_id = %operation_id, success = result.is_ok(), elapsed_ms = started.elapsed().as_millis(), "clipboard edit save completed");
            let _ = panel.update(cx, |panel, cx| {
                if panel.edit_generation != edit_generation || panel.edit_save_pending != Some(operation_id) || panel.editing_item_id != Some(item_id) {
                    return;
                }
                panel.edit_save_pending = None;
                match result {
                    Ok(_) => {
                        // An accepted write may finish after the user has continued typing.
                        // Keep that newer draft open; only the submitted value is durable.
                        let draft_unchanged = panel.markdown_edit_value(cx) == text;
                        panel.edit_original = text.clone();
                        if !draft_unchanged {
                            panel.set_operation_status_for(operation_id, PanelStatus::Notice("已保存提交的文本，当前草稿仍有修改".into()), cx);
                            cx.notify();
                            return;
                        }
                        panel.editing_item_id = None;
                        panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Notice("文本已保存为纯文本".into()),
                            cx,
                        );
                        panel.refresh_entries(cx);
                        let panel_for_focus = cx.entity().downgrade();
                        cx.defer(move |cx| {
                            let _ = window_handle.update(cx, |_, window, app| {
                                let _ = panel_for_focus.update(app, |panel, cx| {
                                    panel.search_input.focus_handle(cx).focus(window, cx);
                                });
                            });
                        });
                    }
                    Err(error) => {
                        warn!(event = "clipboard.edit.save.failed", correlation_id = %operation_id, "clipboard text save failed");
                        panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Error(format!("保存失败：{error}")),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn restore_last_deleted(&mut self, cx: &mut Context<Self>) {
        let Some(item_id) = self.last_deleted_item_id else {
            return;
        };
        let worker = self.history_worker.clone();
        let panel = cx.entity().downgrade();
        let operation_id = uuid::Uuid::new_v4();
        self.set_operation_status_for(operation_id, PanelStatus::Pending("正在撤销删除…"), cx);
        cx.spawn(async move |_, cx| {
            let result = worker.restore_async(item_id).await;
            let _ = panel.update(cx, |panel, cx| {
                match result {
                    Ok(_) => {
                        panel.last_deleted_item_id = None;
                        panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Notice("已撤销删除".into()),
                            cx,
                        );
                        panel.refresh_entries(cx);
                    }
                    Err(error) => {
                        error!(%error, "failed to restore clipboard item");
                        panel.set_operation_status_for(
                            operation_id,
                            PanelStatus::Error(format!("撤销删除失败：{error}")),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn activate_entry(
        &mut self,
        entry: &ClipboardItem,
        action: ClipboardAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.activate_entry_correlated(entry, action, window, uuid::Uuid::new_v4(), cx);
    }

    fn activate_entry_correlated(
        &mut self,
        entry: &ClipboardItem,
        action: ClipboardAction,
        _window: &mut Window,
        action_operation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        if self.pending_action.is_some() || self.pending_action_started_at.is_some() {
            let operation_id = self
                .pending_action
                .map(|(id, _, _)| id)
                .or_else(|| self.pending_action_started_at.map(|(id, _)| id))
                .expect("pending clipboard action identity");
            self.set_operation_status_for(
                operation_id,
                PanelStatus::Pending("正在等待剪贴板操作结果…"),
                cx,
            );
            cx.notify();
            return;
        }
        self.dismiss_preview_for_input("item_action", cx);
        if action == ClipboardAction::WithoutLineBreaks && entry.kind != ClipboardKind::Text {
            self.set_operation_status(PanelStatus::Error("去换行仅适用于文本条目".into()), cx);
            cx.notify();
            return;
        }
        if action != ClipboardAction::CopyOnly && self.target.window_handle == 0 {
            self.set_operation_status(PanelStatus::Error("原目标窗口已经不可用".into()), cx);
            cx.notify();
            return;
        }
        let worker = self.history_worker.clone();
        let action_query_generation = self.query_dispatch_generation;
        let hydrate_full_item = self.summary_counts.contains_key(&entry.id);
        let application = self.application.clone();
        let panel = cx.entity().downgrade();
        let entry = entry.clone();
        let target = self.target.clone();
        self.set_operation_status_for(
            action_operation_id,
            PanelStatus::Pending(if action == ClipboardAction::CopyOnly {
                "正在复制…"
            } else {
                "正在粘贴…"
            }),
            cx,
        );
        self.pending_action = Some((action_operation_id, action, entry.id));
        self.pending_action_started_at = Some((action_operation_id, Instant::now()));
        info!(
            event = "clipboard.action.started",
            session_id = %self.session_id,
            correlation_id = %action_operation_id,
            operation_id = %action_operation_id,
            item_id = entry.id,
            ?action,
            "clipboard action preparation started"
        );
        let panel_for_timeout = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            cx.background_executor()
                .timer(CLIPBOARD_ACTION_TIMEOUT)
                .await;
            let _ = panel_for_timeout.update(cx, |panel, cx| {
                if panel
                    .pending_action_started_at
                    .is_some_and(|(operation_id, _)| operation_id == action_operation_id)
                {
                    let elapsed_ms = panel
                        .pending_action_started_at
                        .map(|(_, started_at)| started_at.elapsed().as_millis());
                    panel.pending_action = None;
                    panel.pending_action_started_at = None;
                    panel.set_operation_status_for(
                        action_operation_id,
                        PanelStatus::Error("剪贴板操作超时，请重试".into()),
                        cx,
                    );
                    warn!(
                        event = "clipboard.action.failed",
                        session_id = %panel.session_id,
                        correlation_id = %action_operation_id,
                        operation_id = %action_operation_id,
                        failure_kind = "timeout",
                        elapsed_ms = elapsed_ms.unwrap_or_default(),
                        "clipboard action timed out"
                    );
                    cx.notify();
                }
            });
        })
        .detach();
        cx.spawn(async move |_, cx| {
            let result = async {
                let (entry, native_formats) = if hydrate_full_item {
                    let (item, formats) = worker
                        .action_item_async(entry.id)
                        .await
                        .map_err(anyhow::Error::from)?;
                    (
                        item,
                        if matches!(
                            action,
                            ClipboardAction::OriginalPaste | ClipboardAction::CopyOnly
                        ) {
                            formats
                        } else {
                            Vec::new()
                        },
                    )
                } else {
                    let formats = if matches!(
                        action,
                        ClipboardAction::OriginalPaste | ClipboardAction::CopyOnly
                    ) {
                        worker
                            .native_formats_async(entry.id)
                            .await
                            .map_err(anyhow::Error::from)?
                    } else {
                        Vec::new()
                    };
                    (entry.clone(), formats)
                };
                let route_is_current =
                    panel
                        .update(cx, |panel, _| {
                            panel.pending_action == Some((action_operation_id, action, entry.id))
                                && panel.is_open
                                && panel.query_dispatch_generation == action_query_generation
                                && !panel.search_pending
                                && panel.pending_action_started_at.is_some_and(
                                    |(operation_id, _)| operation_id == action_operation_id,
                                )
                        })
                        .unwrap_or(false);
                if !route_is_current {
                    return Ok(None);
                }
                application
                    .update(cx, |application, _| {
                        application.send_clipboard_action_with_native_formats(
                            &entry,
                            &target,
                            action,
                            native_formats,
                            action_operation_id,
                        )
                    })
                    .map_err(|error| anyhow::anyhow!("clipboard application unavailable: {error}"))
                    .and_then(|result| result.map(Some))
            }
            .await;
            let _ = panel.update(cx, |panel, cx| {
                if panel
                    .pending_action_started_at
                    .is_none_or(|(operation_id, _)| operation_id != action_operation_id)
                {
                    return;
                }
                match result {
                    Ok(Some(command_id)) => {
                        debug_assert_eq!(command_id, action_operation_id);
                    }
                    Ok(None) => {
                        info!(event = "clipboard.action.cancelled", correlation_id = %action_operation_id,
                            session_id = %panel.session_id, reason = "route_superseded",
                            elapsed_ms = panel.pending_action_started_at.map(|(_, at)| at.elapsed().as_millis() as u64).unwrap_or_default());
                        panel.pending_action = None;
                        panel.pending_action_started_at = None;
                        panel.status = PanelStatus::Ready(String::new());
                    }
                    Err(error) => {
                        let elapsed_ms = panel
                            .pending_action_started_at
                            .map(|(_, started_at)| started_at.elapsed().as_millis())
                            .unwrap_or_default();
                        panel.pending_action = None;
                        panel.pending_action_started_at = None;
                        warn!(
                            event = "clipboard.action.failed",
                            correlation_id = %action_operation_id,
                            operation_id = %action_operation_id,
                            session_id = %panel.session_id,
                            item_id = entry.id,
                            ?action,
                            failure_kind = "preparation_or_dispatch",
                            error_present = true,
                            elapsed_ms,
                            "clipboard action failed before terminal result"
                        );
                        panel.set_operation_status_for(
                            action_operation_id,
                            PanelStatus::Error(format!("无法粘贴：{error}")),
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        })
        .detach();
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
        let elapsed_ms = self
            .pending_action_started_at
            .take()
            .map(|(_, started_at)| started_at.elapsed().as_millis());
        self.pending_action = None;
        if result.success {
            info!(
                event = "clipboard.action.completed",
                correlation_id = %correlation_id,
                session_id = %self.session_id,
                item_id,
                ?action,
                elapsed_ms = elapsed_ms.unwrap_or_default(),
                "clipboard action completed"
            );
            let worker = self.history_worker.clone();
            let panel = (action == ClipboardAction::CopyOnly).then(|| cx.entity().downgrade());
            cx.spawn(async move |_, cx| {
                let result = worker
                    .mark_used_async(item_id, nxtools_ipc::unix_time_ms())
                    .await;
                if let Err(error) = result {
                    warn!(%error, item_id, "clipboard action succeeded but usage could not be recorded");
                }
                if let Some(panel) = panel {
                    let _ = panel.update(cx, |panel, cx| panel.refresh_entries(cx));
                }
            })
            .detach();
            if action == ClipboardAction::CopyOnly {
                self.set_operation_status_for(
                    correlation_id,
                    PanelStatus::Notice("已复制".into()),
                    cx,
                );
                window.activate_window();
                cx.notify();
            } else if self.fixed_display {
                // Native paste already restored the target. Do not reactivate this panel.
                cx.notify();
            } else {
                self.close_with_reason(
                    window,
                    cx,
                    PopupCloseReason::PasteSuccess {
                        command_id: correlation_id,
                    },
                );
            }
        } else {
            warn!(
                event = "clipboard.action.failed",
                correlation_id = %correlation_id,
                session_id = %self.session_id,
                item_id,
                ?action,
                failure_kind = "platform_result",
                error_present = result.error.is_some(),
                native_error_code = ?result.native_error_code,
                elapsed_ms = elapsed_ms.unwrap_or_default(),
                "clipboard action failed"
            );
            let reason = result.error.unwrap_or_else(|| "Windows 未返回原因".into());
            self.set_operation_status_for(
                correlation_id,
                PanelStatus::Error(format!("粘贴失败：{}", reason)),
                cx,
            );
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
        self.set_operation_status(PanelStatus::Error(format!("隐藏面板失败：{reason}")), cx);
        window.activate_window();
        self.focus_search_input(window, cx);
        cx.notify();
    }

    pub(crate) fn handle_topmost_failure(
        &mut self,
        reason: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.set_operation_status(PanelStatus::Error(format!("面板置顶失败：{reason}")), cx);
        window.activate_window();
        cx.notify();
    }

    pub(crate) fn handle_foreground_changed(
        &mut self,
        popup_window_handle: i64,
        preview_window_handle: i64,
        foreground: &nxtools_protocol::ForegroundContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.fixed_display {
            if foreground.window_handle != popup_window_handle
                && foreground.window_handle != preview_window_handle
                && foreground.process_id != std::process::id()
                && foreground.window_handle != 0
                && !is_transient_foreground_host(&foreground.process_name)
            {
                self.target = foreground.clone();
                self.dismiss_preview_for_input("fixed_focus_left", cx);
            }
            return;
        }
        if crate::notifications::screen::owns_native_window(foreground.window_handle, cx) {
            return;
        }
        if components::native_context_menu::active(window, cx) {
            return;
        }
        // Cross-process foreground snapshots may precede GPUI activation delivery
        // or arrive after this panel regained focus. Recheck only this same session
        // next turn instead of hiding an actively used panel from a stale snapshot.
        if window.is_window_active() && foreground.window_handle != popup_window_handle {
            let panel = cx.entity().downgrade();
            let popup_window = window.window_handle();
            let session_id = self.session_id;
            let foreground = foreground.clone();
            cx.defer(move |cx| {
                let _ = popup_window.update(cx, |_, window, app| {
                    let _ = panel.update(app, |panel, cx| {
                        if panel.session_id == session_id && !window.is_window_active() {
                            panel.handle_foreground_changed(
                                popup_window_handle,
                                preview_window_handle,
                                &foreground,
                                window,
                                cx,
                            );
                        }
                    });
                });
            });
            return;
        }
        let dismiss = PopupDismissContext {
            popup_window_handle,
            preview_window_handle,
            current_process_id: std::process::id(),
            paste_pending: self.pending_action.is_some(),
            is_open: self.is_open,
        };
        if should_hide_for_foreground_change(
            dismiss,
            foreground.window_handle,
            foreground.process_id,
            &foreground.process_name,
            Instant::now() < self.hide_suppressed_until,
        ) {
            self.close_with_reason(
                window,
                cx,
                PopupCloseReason::Foreground {
                    window_handle: foreground.window_handle,
                    process_id: foreground.process_id,
                    process_name: foreground.process_name.clone(),
                },
            );
            return;
        }
        if foreground_kept_panel_without_focus(
            dismiss,
            foreground.window_handle,
            foreground.process_id,
            &foreground.process_name,
        ) {
            // A transient text-services host took the foreground while the user
            // keeps interacting with the panel. Re-assert topmost without
            // activation so the panel cannot sink below the new foreground.
            let application = self.application.clone();
            schedule_entity_update(
                application,
                move |application, _cx| {
                    application.reassert_popup_topmost(popup_window_handle);
                },
                cx,
            );
        }
    }

    pub(crate) fn handle_pointer_pressed(
        &mut self,
        press: PopupPointerPressContext,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.fixed_display {
            return;
        }
        if crate::notifications::screen::owns_native_window(press.pressed_root_window_handle, cx) {
            return;
        }
        if components::native_context_menu::active(window, cx) {
            return;
        }
        let dismiss = PopupDismissContext {
            popup_window_handle: press.popup_window_handle,
            preview_window_handle: press.preview_window_handle,
            current_process_id: std::process::id(),
            paste_pending: self.pending_action.is_some(),
            is_open: self.is_open,
        };
        if should_hide_for_pointer_press_with_session(
            dismiss,
            press.pressed_root_window_handle,
            press.pressed_process_id,
            press.popup_requested_at_unix_ms,
            press.popup_event_sequence,
            press.pressed_at_unix_ms,
            press.pressed_event_sequence,
        ) {
            self.close_with_reason(
                window,
                cx,
                PopupCloseReason::Pointer {
                    window_handle: press.pressed_window_handle,
                    root_window_handle: press.pressed_root_window_handle,
                    process_id: press.pressed_process_id,
                },
            );
        }
    }

    pub(crate) fn close_from_native_menu(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.fixed_display {
            self.close_with_reason(window, cx, PopupCloseReason::Escape);
        }
    }

    fn close_with_reason(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
        reason: PopupCloseReason,
    ) {
        if !self.is_open {
            return;
        }
        let Some(window_handle) = native_window_handle(window) else {
            warn!(
                session_id = %self.session_id,
                reason = reason.kind(),
                "clipboard.popup.close_failed"
            );
            self.set_operation_status(
                PanelStatus::Error("无法隐藏面板：窗口句柄不可用".into()),
                cx,
            );
            cx.notify();
            return;
        };
        let popup_window = window.window_handle();
        let session_id = self.session_id;
        let final_viewport = window.viewport_size();
        self.is_open = false;
        self.fixed_display = false;
        self.full_item_request = None;
        self.extension_feedback.clear();
        if let Some(center) = crate::notifications::current(cx) {
            cx.defer(move |cx| {
                center.update(cx, |center, cx| center.dismiss_window(popup_window, cx))
            });
        }
        window.close_all_dialogs(cx);
        components::project_menu::dismiss_window(window, cx);
        let application = self.application.clone();
        let panel = cx.entity().downgrade();
        schedule_entity_update(
            application,
            move |application, cx| {
                application.observe_panel_size(session_id, popup_window, final_viewport, cx);
                application.dismiss_preview(cx);
                let result =
                    application.hide_popup(popup_window, window_handle, session_id, reason);
                if let Err(error) = result {
                    let _ = panel.update(cx, |panel, cx| {
                        panel.is_open = true;
                        panel.set_operation_status(
                            PanelStatus::Error(format!("无法隐藏面板：{error}")),
                            cx,
                        );
                        cx.notify();
                    });
                    // Recover GPUI input focus only after both application and
                    // panel leases are released; a stale close cannot refocus.
                    let panel = panel.clone();
                    cx.defer(move |cx| {
                        let _ = popup_window.update(cx, |_, window, cx| {
                            let _ = panel.update(cx, |panel, cx| {
                                if panel.session_id == session_id && panel.is_open {
                                    panel.focus_search_input(window, cx);
                                }
                            });
                        });
                    });
                }
            },
            cx,
        );
    }
}

impl Render for ClipboardPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        self.theme = Theme::active(cx);
        if window.focused(cx).is_none() && self.search_input_ready(window, cx) {
            self.focus_search_input(window, cx);
        }
        let theme = self.theme;
        let modifier_actions = self.modifier_actions;
        let text_card_max_height = self.text_card_max_height;
        let entries_empty = self.entries.borrow().is_empty();
        let search_query = self.search_input.read(cx).value().trim().to_owned();
        let search_loading = self.search_pending;
        let page_loading = self.page_request_pending;
        let query_failed = self.query_error.is_some();
        let query_error_message = self
            .query_error
            .clone()
            .unwrap_or_else(|| "剪贴板查询失败".into());
        let summary = self
            .active_category()
            .map(|category| category.name.clone())
            .unwrap_or_else(|| "全部".into());
        let selected_entry_index = self.selected_entry_index;
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
        let category_source_choices = self.category_source_choices();
        let selected_category_sources = self
            .category_editor
            .as_ref()
            .map(|editor| editor.selected_sources.clone())
            .unwrap_or_default();
        let can_undo_delete = self.last_deleted_item_id.is_some();
        let list_width = panel_list_width(window.viewport_size().width.as_f32());
        if self.list_width > 0.0 && (self.list_width - list_width).abs() > 0.5 {
            self.list_scroll.reset(self.entries.borrow().len());
            self.list_is_scrolled = false;
        }
        self.list_width = list_width;
        // The shared result row supplies 7px on each side; image wells also
        // carry 4px inset each side. Keep the aspect-ratio budget aligned.
        let available_image_width = (list_width - 22.0).max(28.0);
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
        let category_source_panel = cx.entity().downgrade();
        let search_scope_panel = cx.entity().downgrade();
        let extended_search = self.search_scope == ClipboardSearchScope::Extended;
        let move_application = self.application.clone();
        let resize_application = self.application.clone();
        let extension_feedback = self.extension_feedback.clone();
        let capture_controls = self.render_capture_controls(cx);
        let preview_motion = Rc::clone(&self.preview_pointer);
        let content = div()
            .track_focus(&self.focus_scope)
            .relative()
            .size_full()
            .text_color(theme.text)
            .flex()
            .flex_col()
            .gap(px(0.0))
            .capture_any_mouse_down(|_, window, cx| {
                if components::native_context_menu::consume_source_press(window, cx) {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            })
            .capture_any_mouse_up(|_, window, cx| {
                if components::native_context_menu::consume_source_release(window, cx) {
                    window.prevent_default();
                    cx.stop_propagation();
                }
            })
            .on_mouse_move(move |event, window, _| {
                preview_motion.set(window.bounds().origin + event.position);
            })
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
                    .debug_selector(|| "clipboard-panel-header".into())
                    .mt(px(0.0))
                    .h(px(CONTROL_HEIGHT_INPUT))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .cursor_default()
                    .on_mouse_down(gpui_kit::MouseButton::Left, move |_event, window, cx| {
                        cx.stop_propagation();
                        #[cfg(test)]
                        efficiency_tests::record_header_drag_press();
                        let Some(handle) = native_window_handle(window) else {
                            return;
                        };
                        let _ = move_application.update(cx, |application, _cx| {
                            application.begin_window_move(handle);
                        });
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(8.0))
                            .child(
                                Icon::new(MaterialIcon::ContentPaste)
                                    .size(px(16.0))
                                    .text_color(theme.accent),
                            )
                            .child(
                                div()
                                    .text_size(px(TEXT_BODY))
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .child("剪贴板"),
                            ),
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
                                .h(px(NOTIFICATION_META_BLOCK_HEIGHT))
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
                    .when(page_loading, |header| {
                        header.child(gpui_kit::component::label::Label::new("正在加载更多…").text_size(px(TEXT_META)))
                    })
                    .child(capture_controls),
            ))
            .child(inset_panel_section(
                div()
                    .h(px(CONTROL_HEIGHT_ACTION))
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("clipboard-category-scroll")
                            .min_w_0()
                            .h(px(CONTROL_HEIGHT_ACTION))
                            .overflow_x_scroll()
                            .track_scroll(&category_scroll)
                            .on_scroll_wheel({
                                let scroll = category_scroll.clone();
                                move |event, window, app| {
                                    app.stop_propagation();
                                    let delta = event.delta.pixel_delta(px(48.0));
                                    let maximum = scroll.max_offset().x.as_f32().max(0.0);
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
                                    .h(px(CONTROL_HEIGHT_ACTION))
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
                                            window,
                                            cx,
                                        )
                                    })),
                            ),
                    )
                    .child(
                        focused_action(
                            "add-clipboard-category",
                            &window
                                .use_keyed_state("clipboard-category-add-action", cx, |_, cx| {
                                    cx.focus_handle().tab_stop(true)
                                })
                                .read(cx)
                                .clone(),
                            theme,
                            false,
                            window,
                            cx,
                            {
                                let add_panel = add_category_panel.clone();
                                move |_event, window, app| {
                                    let _ = add_panel.update(app, |panel, cx| {
                                        panel.begin_category_editor(None, window, cx);
                                    });
                                }
                            },
                        )
                        .debug_selector(|| "add-clipboard-category".into())
                        .size(px(CONTROL_HEIGHT_ACTION))
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
                        .active(move |style| style.bg(theme.surface_pressed))
                        .tooltip(|window, cx| Tooltip::new("新建分类").build(window, cx))
                        .child(Icon::new(MaterialIcon::Add).size(px(14.0))),
                    ),
            ))
            .child(inset_panel_section(
                div()
                    .h(px(24.0))
                    .mb(px(crate::ui::metrics::SPACING_8))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div().min_w_0().flex_1().child(
                        InputGroup::new("clipboard-search-group")
                            .small()
                            .input(Input::new(&self.search_input).aria_label("搜索剪贴板历史"))
                            .addon(
                                InputGroupAddon::new("clipboard-search-icon")
                                    .child(Icon::new(MaterialIcon::Search)),
                            )
                            .addon(
                                InputGroupAddon::new("clipboard-search-actions")
                                    .align(InputGroupAddonAlignment::InlineEnd)
                                    .child(
                                        InputGroupButton::new("search-scope-toggle")
                                            .xsmall()
                                            .debug_selector(|| {
                                                "clipboard-search-scope-control".into()
                                            })
                                            .icon(MaterialIcon::ManageSearch)
                                            .selected(extended_search)
                                            .tooltip(if extended_search {
                                                "关闭扩展搜索"
                                            } else {
                                                "扩展搜索：图片名、文件路径、来源进程和窗口标题"
                                            })
                                            .accessibility_label("搜索范围")
                                            .on_click(move |_event, _window, app| {
                                                let _ = search_scope_panel
                                                    .update(app, |panel, cx| {
                                                        panel.toggle_search_scope(cx)
                                                    });
                                            }),
                                    ),
                            ),
                        ),
                    )
                    .child(self.render_history_filter_button(cx)),
            ))
            .child(inset_panel_section(self.render_active_history_filters(cx)))
            .child(inset_panel_section(self.render_capture_failure()))
            .child(inset_panel_section(
                div()
                    .debug_selector(|| "clipboard-history-viewport".into())
                    .mb(px(PANEL_CONTENT_INSET))
                    .relative()
                    .flex_1()
                    .when(entries_empty && search_loading, |list| {
                        list.child(progress_state(
                            theme,
                            "clipboard-history-loading",
                            "正在查询剪贴板",
                            "搜索结果加载中…",
                        ))
                    })
                    .when(
                        entries_empty && !search_loading && query_failed,
                        move |list| {
                            list.child(div().id("clipboard-history-query-error")
                                .debug_selector(|| "clipboard-kit-query-error".into())
                                .w_full().max_h_full().overflow_y_scroll()
                                .child(Alert::error("clipboard-history-query-alert", query_error_message)
                                    .xsmall().title("查询失败")))
                        },
                    )
                    .when(entries_empty && !search_loading && !query_failed, |list| {
                        list.child(self.render_history_empty(&search_query, cx))
                    })
                    .when(!entries_empty, |container| {
                        let entries = Rc::clone(&self.entries);
                        let custom_categories = self
                            .categories
                            .iter()
                            .filter(|category| category.kind == ClipboardCategoryKind::Custom)
                            .cloned()
                            .collect::<Vec<_>>();
                        let category_sources = self.category_sources.clone();
                        let history_panel = cx.entity().downgrade();
                        let summary_counts = self.summary_counts.clone();
                        let open_targets = self.open_targets.clone();
                        let shell_application = self.application.clone();
                        let preview_session_id = self.session_id;
                        let preview_query_generation = self.query_dispatch_generation;
                        let preview_pointer = Rc::clone(&self.preview_pointer);
                        let item_feedback = extension_feedback.clone();
                        let image_resource_cache = Rc::clone(&self.image_resource_cache);
                        let thumbnails = self.list_thumbnails.clone();
                        let source_icon_colors = self.source_icon_colors.clone();
                        let theme = self.theme;
                        container.child(
                            list(self.list_scroll.clone(), move |index, _window, app| {
                                let Some(entry) = entries.borrow().get(index).cloned() else {
                                    return div().into_any_element();
                                };
                                let text_character_count = summary_counts.get(&entry.id).copied();
                                if text_character_count.is_some() && !open_targets.contains_key(&entry.id) {
                                    let panel = history_panel.clone();
                                    let item_id = entry.id;
                                    app.defer(move |cx| {
                                        let _ = panel.update(cx, |panel, cx| panel.request_open_targets(item_id, cx));
                                    });
                                }
                                if !entry.source.process_icon_path.is_empty() && !source_icon_colors.contains_key(&entry.source.process_icon_path) {
                                    let panel = history_panel.clone();
                                    let path = entry.source.process_icon_path.clone();
                                    app.defer(move |cx| { let _ = panel.update(cx, |panel, cx| panel.request_source_icon_colors(vec![path], cx)); });
                                }
                                let source_icon_color = source_icon_colors.get(&entry.source.process_icon_path).copied().flatten();
                                let resolved_targets = text_character_count.map(|_| open_targets.get(&entry.id).cloned().unwrap_or_default());
                                let list_image = if entry.kind == ClipboardKind::Image {
                                    if !thumbnails.contains_key(&entry.image_path) {
                                        let panel = history_panel.clone();
                                        let path = entry.image_path.clone();
                                        app.defer(move |cx| {
                                            let _ = panel.update(cx, |panel, cx| panel.request_visible_thumbnails(vec![path], cx));
                                        });
                                    }
                                    Some(thumbnails.get(&entry.image_path).cloned().flatten().unwrap_or_default())
                                } else { None };
                                for resource_path in [
                                    list_image.as_deref().unwrap_or_else(|| Path::new("")),
                                    Path::new(&entry.source.process_icon_path),
                                ]
                                .into_iter()
                                .filter(|path| !path.as_os_str().is_empty())
                                {
                                    if let Some(evicted) = image_resource_cache
                                        .borrow_mut()
                                        .touch(resource_path)
                                    {
                                        ImageSource::from(evicted).remove_asset(app);
                                    }
                                }
                                let feedback = item_feedback.get(&entry.id).cloned();
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
                                          _event: &gpui_kit::ClickEvent,
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
                                          _event: &gpui_kit::ClickEvent,
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
                                let open_panel = history_panel.clone();
                                let open_item_id = entry.id;
                                let on_open = Rc::new(
                                    move |target: nxtools_clipboard::ClipboardOpenTarget,
                                          _event: &gpui_kit::ClickEvent,
                                          window: &mut Window,
                                          app: &mut App| {
                                        app.stop_propagation();
                                        let _ = open_panel.update(app, |panel, cx| {
                                            panel.open_full_item_target(open_item_id, target, window, cx);
                                        });
                                    },
                                );
                                let preview_enabled = !is_editing_category;
                                let preview_application = shell_application.clone();
                                let preview_panel = history_panel.clone();
                                let preview_pointer = Rc::clone(&preview_pointer);
                                let on_preview = Rc::new(
                                    move |entered: bool,
                                          item: ClipboardItem,
                                          panel_bounds: Bounds<Pixels>,
                                          pointer_position: Point<Pixels>,
                                          panel_scale_factor: f32,
                                          app: &mut App| {
                                        if !preview_enabled {
                                            return;
                                        }
                                        // Activation/hit-test refresh can deliver the same coordinates.
                                        if entered && preview_pointer.replace(pointer_position) == pointer_position {
                                            return;
                                        }
                                        let application = preview_application.clone();
                                        let panel = preview_panel.clone();
                                        app.defer(move |cx| {
                                            if entered && components::native_context_menu::any_active(cx) {
                                                return;
                                            }
                                            let current = panel.update(cx, |panel, _| {
                                                panel.session_id == preview_session_id
                                                    && panel.query_dispatch_generation == preview_query_generation
                                                    && (!entered || (panel.hover_preview_ready()
                                                        && panel.entries.borrow().get(index).is_some_and(|entry| entry.id == item.id)))
                                            }).unwrap_or(false);
                                            if !current {
                                                return;
                                            }
                                            let _ = application.update(cx, |application, cx| {
                                                application.schedule_preview(
                                                    entered,
                                                    item,
                                                    panel_bounds,
                                                    pointer_position,
                                                    panel_scale_factor,
                                                    cx,
                                                );
                                            });
                                        });
                                    },
                                );
                                let extension_actions = crate::extensions::manual_actions::available_for("clipboard", Some(nxtools_extensions::InputSelection {
                                    kind: match entry.kind { nxtools_clipboard::ClipboardKind::Text => nxtools_extensions::InputKind::Text, nxtools_clipboard::ClipboardKind::Image => nxtools_extensions::InputKind::Image, nxtools_clipboard::ClipboardKind::Files => nxtools_extensions::InputKind::Files },
                                    count: if entry.kind == nxtools_clipboard::ClipboardKind::Files { entry.file_paths.len() } else { 1 },
                                    bytes: if entry.kind == nxtools_clipboard::ClipboardKind::Text { text_character_count.unwrap_or_else(|| entry.text.chars().count()) } else { entry.file_paths.iter().map(String::len).sum() },
                                }), app);
                                history_row(
                                    theme,
                                    entry,
                                    index,
                                    selected_entry_index == Some(index),
                                    available_image_width,
                                    modifier_actions,
                                    CategorySourceMenu {
                                        options: category_options,
                                        unavailable_hint: category_hint,
                                    },
                                    HistoryRowHandlers {
                                        source_icon_color,
                                        list_image,
                                        text_character_count,
                                        text_card_max_height,
                                        open_targets: resolved_targets,
                                        action: on_action,
                                        management: on_management,
                                        open: on_open,
                                        preview: on_preview,
                                        extension_actions,
                                        extension_feedback: feedback,
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
                                    Button::new("back-to-top")
                                        .small().outline().icon(MaterialIcon::ArrowUpward).label("回到开头")
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
                                    Button::new("undo-clipboard-delete")
                                        .small().outline().icon(MaterialIcon::Undo).label("撤销删除")
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
                        .debug_selector(|| "clipboard-text-editor".into())
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
                            Form::vertical().small().flex_1().min_h_0().child(
                                Field::new()
                                    .label("编辑文本")
                                    .gap_0()
                                    .child(self.render_markdown_editor(window, cx))
                                    .description("富文本保存可能规范化 Markdown 格式；原文模式保留文本格式。未修改或取消不会改写记录。"),
                            ),
                        )
                        .child(
                            div()
                                .debug_selector(|| "clipboard-text-editor-footer".into())
                                .flex_none()
                                .flex()
                                .flex_wrap()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .id("clipboard-text-editor-shortcuts")
                                        .w_full()
                                        .flex_none()
                                        .flex().flex_wrap().items_center().gap_2()
                                        .child(crate::ui::components::shortcut::hint("ctrl-enter", "保存"))
                                        .child(crate::ui::components::shortcut::hint("escape", "取消")),
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
                let source_panel = category_source_panel.clone();
                let selected_sources = selected_category_sources.clone();
                panel
                    .child(
                        div()
                            .id("clipboard-category-editor-backdrop")
                            .debug_selector(|| "clipboard-category-editor-backdrop".into())
                            .absolute()
                            .top_0()
                            .left_0()
                            .right_0()
                            .bottom_0()
                            .occlude()
                            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                cx.stop_propagation();
                            })
                            .on_click(|_event, _window, cx| {
                                cx.stop_propagation();
                            }),
                    )
                    .child(
                        div()
                            .id("clipboard-category-editor")
                            .debug_selector(|| "clipboard-category-editor".into())
                            .on_mouse_down(MouseButton::Left, |_event, _window, cx| {
                                cx.stop_propagation();
                            })
                            .on_click(|_event, _window, cx| {
                                cx.stop_propagation();
                            })
                            .absolute()
                            .top(px(CATEGORY_EDITOR_TOP))
                            .left_3()
                            .right_3()
                            .h(px(CATEGORY_EDITOR_HEIGHT))
                            .occlude()
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
                                    .h(px(LINE_HEIGHT_BODY))
                                    .flex_none()
                                    .text_size(px(TEXT_BODY))
                                    .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                    .child(category_editor_title),
                            )
                            .child(
                                Form::horizontal()
                                    .small()
                                    .label_width(px(44.0))
                                    .flex_none()
                                    .child(
                                        Field::new().label("名称").items_center().gap_0().child(
                                            div()
                                                .debug_selector(|| {
                                                    "clipboard-category-name-field".into()
                                                })
                                                .w_full()
                                                .child(
                                                    Input::new(&self.category_name_input)
                                                        .aria_label("分类名称"),
                                                ),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_none()
                                    .text_size(px(TEXT_BODY))
                                    .line_height(px(LINE_HEIGHT_BODY))
                                    .text_color(theme.text_muted)
                                    .child("选择来源应用后，新捕获的内容会自动进入此分类"),
                            )
                            .child(
                                div()
                                    .h(px(GROUP_HEADER_HEIGHT))
                                    .flex_none()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex_1()
                                            .text_size(px(TEXT_BODY))
                                            .line_height(px(LINE_HEIGHT_BODY))
                                            .font_weight(gpui_kit::FontWeight::SEMIBOLD)
                                            .child("自动归类来源应用"),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(TEXT_META))
                                            .text_color(theme.text_muted)
                                            .child(format!("已选 {} 个", selected_sources.len())),
                                    ),
                            )
                            .child(
                                div()
                                    .id("clipboard-category-source-list")
                                    .debug_selector(|| "clipboard-category-source-list".into())
                                    .max_h(px(CATEGORY_EDITOR_SOURCE_LIST_HEIGHT))
                                    .min_h_0()
                                    .flex_1()
                                    .overflow_y_scroll()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .when(category_source_choices.is_empty(), |list| {
                                        list.child(
                                            div()
                                                .h(px(28.0))
                                                .flex()
                                                .items_center()
                                                .text_size(px(TEXT_BODY))
                                                .line_height(px(LINE_HEIGHT_BODY))
                                                .text_color(theme.text_muted)
                                                .child("暂无已捕获的来源应用"),
                                        )
                                    })
                                    .children(category_source_choices.into_iter().enumerate().map(
                                        {
                                            let selected_sources = selected_sources.clone();
                                            let source_panel = source_panel.clone();
                                            move |(index, source)| {
                                                let selected = selected_sources
                                                    .contains(&source.to_lowercase());
                                                let process_name = source.clone();
                                                div().debug_selector(move || format!("clipboard-category-source-{index}"))
                                                    .h(px(28.0)).w_full().min_w_0().flex_none().flex().items_center()
                                                    .child(gpui_kit::component::checkbox::Checkbox::new(
                                                        gpui_kit::SharedString::from(format!("clipboard-category-source-{index}")))
                                                        .checked(selected).small().w_full()
                                                        .accessibility_label(source.clone()).tooltip(source.clone())
                                                .on_click({
                                                    let source_panel = source_panel.clone();
                                                    move |_event, _window, app| {
                                                        app.stop_propagation();
                                                        let _ = source_panel.update(
                                                            app,
                                                            |panel, cx| {
                                                                panel
                                                                    .toggle_category_editor_source(
                                                                        &process_name,
                                                                        cx,
                                                                    );
                                                            },
                                                        );
                                                    }
                                                })
                                                        .child(div().min_w_0().truncate().child(source)))
                                            }
                                        },
                                    )),
                            )
                            .child(
                                div()
                                    .debug_selector(|| "clipboard-category-editor-footer".into())
                                    .flex_none()
                                    .h(px(GROUP_HEADER_HEIGHT))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(div().flex_1())
                                    .child(dialog_button(
                                        theme,
                                        "取消",
                                        false,
                                        move |_event, window, app| {
                                            let _ =
                                                cancel_category_panel.update(app, |panel, cx| {
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
                    .text_size(px(TEXT_META))
                    .text_color(theme.text_muted)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        Icon::new(MaterialIcon::OpenInFull)
                            .size(px(14.0))
                            .text_color(theme.text_muted),
                    )
                    .on_mouse_down(gpui_kit::MouseButton::Left, move |_event, window, cx| {
                        cx.stop_propagation();
                        let Some(handle) = native_window_handle(window) else {
                            return;
                        };
                        let _ = resize_application.update(cx, |application, _cx| {
                            application.begin_window_resize(handle);
                        });
                    }),
            ));
        let content = content.when_some(crate::notifications::current(cx), |content, center| {
            content.child(crate::notifications::presentation::render(
                center, false, 92.0, window, cx,
            ))
        });
        div()
            .size_full()
            .capture_key_down(
                cx.listener(|panel, event, window, cx| panel.number_key_down(event, window, cx)),
            )
            .capture_key_up(
                cx.listener(|panel, event, window, cx| panel.number_key_up(event, window, cx)),
            )
            .child(panel_window_frame(theme, content))
            .child(self.render_search_input_fallback(cx))
            .into_any_element()
    }
}

fn panel_window_frame(theme: Theme, content: impl IntoElement) -> gpui_kit::AnyElement {
    div()
        .id("clipboard-panel-window")
        .size_full()
        .child(
            div()
                .id("clipboard-panel-shell")
                .size_full()
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PopupPointerPressContext {
    pub(crate) popup_window_handle: i64,
    pub(crate) preview_window_handle: i64,
    pub(crate) pressed_window_handle: i64,
    pub(crate) pressed_root_window_handle: i64,
    pub(crate) pressed_process_id: u32,
    pub(crate) popup_requested_at_unix_ms: i64,
    pub(crate) popup_event_sequence: u64,
    pub(crate) pressed_at_unix_ms: i64,
    pub(crate) pressed_event_sequence: u64,
}

fn should_hide_for_foreground_change(
    dismiss: PopupDismissContext,
    foreground_window_handle: i64,
    _foreground_process_id: u32,
    foreground_process_name: &str,
    within_open_grace: bool,
) -> bool {
    dismiss.is_open
        && !dismiss.paste_pending
        && !within_open_grace
        && dismiss.popup_window_handle != 0
        && foreground_window_handle != 0
        && foreground_window_handle != dismiss.popup_window_handle
        && foreground_window_handle != dismiss.preview_window_handle
        && !is_transient_foreground_host(foreground_process_name)
}

/// True when a foreground change kept the panel open but moved the OS focus to
/// another window: a transient system host or a pending paste. The panel then
/// needs a topmost re-assertion so it cannot sink behind the new foreground.
fn foreground_kept_panel_without_focus(
    dismiss: PopupDismissContext,
    foreground_window_handle: i64,
    foreground_process_id: u32,
    foreground_process_name: &str,
) -> bool {
    dismiss.is_open
        && !dismiss.paste_pending
        && foreground_window_handle != 0
        && foreground_process_id != dismiss.current_process_id
        && foreground_window_handle != dismiss.popup_window_handle
        && foreground_window_handle != dismiss.preview_window_handle
        && is_transient_foreground_host(foreground_process_name)
}

/// Windows text-services hosts transiently take the foreground while the user
/// types with an IME inside the panel's search box; treating them as a real
/// foreground change hid the panel mid-typing.
fn is_transient_foreground_host(process_name: &str) -> bool {
    const TRANSIENT_HOSTS: [&str; 2] = ["textinputhost", "ctfmon"];
    let name = process_name
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or(process_name);
    let name = name.strip_suffix(".exe").unwrap_or(name);
    TRANSIENT_HOSTS
        .iter()
        .any(|host| name.eq_ignore_ascii_case(host))
}

#[cfg(test)]
fn should_hide_for_pointer_press(
    dismiss: PopupDismissContext,
    pressed_root_window_handle: i64,
    pressed_process_id: u32,
) -> bool {
    should_hide_for_pointer_press_with_session(
        dismiss,
        pressed_root_window_handle,
        pressed_process_id,
        0,
        0,
        0,
        0,
    )
}

fn should_hide_for_pointer_press_with_session(
    dismiss: PopupDismissContext,
    pressed_root_window_handle: i64,
    _pressed_process_id: u32,
    popup_requested_at_unix_ms: i64,
    popup_event_sequence: u64,
    pressed_at_unix_ms: i64,
    pressed_event_sequence: u64,
) -> bool {
    // A pointer sampled before this popup session began must not dismiss the
    // newly opened popup merely because IPC delivered it afterward. Zero is
    // the compatibility value used by older PlatformHost peers.
    let sampled_before_popup = (popup_event_sequence != 0
        && pressed_event_sequence != 0
        && pressed_event_sequence <= popup_event_sequence)
        || (popup_requested_at_unix_ms != 0
            && pressed_at_unix_ms != 0
            && pressed_at_unix_ms < popup_requested_at_unix_ms);
    if sampled_before_popup {
        return false;
    }

    // Presses anywhere outside the panel and its preview dismiss the panel —
    // including presses inside THIS application's other windows.
    dismiss.is_open
        && !dismiss.paste_pending
        && dismiss.popup_window_handle != 0
        && pressed_root_window_handle != 0
        && pressed_root_window_handle != dismiss.popup_window_handle
        && pressed_root_window_handle != dismiss.preview_window_handle
}

fn modifier_from_modifiers(modifiers: gpui_kit::Modifiers) -> Option<ClipboardClickModifier> {
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
    modifiers: gpui_kit::Modifiers,
    actions: ClipboardModifierActions,
) -> Option<(ClipboardClickModifier, ClipboardClickAction)> {
    modifier_from_modifiers(modifiers).map(|modifier| (modifier, actions.action(modifier)))
}

fn action_from_modifiers(
    modifiers: gpui_kit::Modifiers,
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
    window: &mut Window,
    cx: &mut App,
) -> gpui_kit::AnyElement {
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
    let focus = window
        .use_keyed_state(
            gpui_kit::SharedString::from(format!("category-action-{category_id}")),
            cx,
            |_, cx| cx.focus_handle().tab_stop(true),
        )
        .read(cx)
        .clone();
    let click_panel = panel.clone();
    let tab = div()
        .id(gpui_kit::SharedString::from(format!(
            "category-{category_id}"
        )))
        .tooltip(move |window, cx| Tooltip::new(tooltip_label.clone()).build(window, cx))
        .w(px(tab_width))
        .flex_none()
        .child(
            Tab::new()
                .underline()
                .xsmall()
                .selected(selected)
                .aria_label(label.clone())
                .w_full()
                .px_2()
                .track_focus(&focus)
                .focus_visible(move |style| style.bg(theme.accent_soft))
                .on_click(move |_, window, app| {
                    app.stop_propagation();
                    focus.focus(window, app);
                    let category_id = click_category_id.clone();
                    let _ = click_panel.update(app, |panel, cx| {
                        panel.select_category(category_id, cx);
                    });
                })
                .on_drag(drag, |drag: &CategoryDrag, position, _window, cx| {
                    cx.new(|_| drag.clone().at(position))
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_1()
                        .child(Icon::new(icon).size(px(12.0)))
                        .child(label),
                ),
        )
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
        });
    let tab = if is_custom {
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
    };
    div()
        .w(px(tab_width))
        .h(px(CONTROL_HEIGHT_ACTION))
        .flex_none()
        .child(tab)
        .into_any_element()
}
fn dialog_button(
    _theme: Theme,
    label: &'static str,
    primary: bool,
    on_click: impl Fn(&gpui_kit::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    Button::new(gpui_kit::SharedString::from(format!(
        "clipboard-dialog-button-{label}"
    )))
    .label(label)
    .small()
    .when(primary, |button| button.primary())
    .when(!primary, |button| button.outline())
    .on_click(move |event, window, cx| {
        cx.stop_propagation();
        on_click(event, window, cx);
    })
}
pub(crate) fn panel_bounds(
    request: &ClipboardPanelRequest,
    panel_size: nxtools_core::ClipboardPanelSize,
    cx: &App,
) -> (Bounds<gpui_kit::Pixels>, Option<gpui_kit::DisplayId>) {
    let desired_size = size(px(panel_size.width as f32), px(panel_size.height as f32));
    let displays = cx.displays();
    let display = displays
        .iter()
        .find(|display| u64::from(display.id()) == u64::from(request.cursor_display_index))
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

fn clamp_pixels(
    value: gpui_kit::Pixels,
    minimum: gpui_kit::Pixels,
    maximum: gpui_kit::Pixels,
) -> gpui_kit::Pixels {
    if maximum < minimum || value < minimum {
        minimum
    } else if value > maximum {
        maximum
    } else {
        value
    }
}

#[cfg(test)]
mod polish_tests;
#[cfg(test)]
#[cfg(test)]
mod tests;

#[cfg(test)]
mod kit_migration_tests;

#[cfg(test)]
mod notification_tests;

#[cfg(test)]
mod efficiency_tests;

#[cfg(test)]
mod history_identity_tests;
