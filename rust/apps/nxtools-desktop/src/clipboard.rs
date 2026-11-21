use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::base::Root;
use gpui_kit::{
    AnyWindowHandle, AppContext, Bounds, Context, Entity, Global, Pixels, Point, Size, WeakEntity,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, point,
    px, size,
};
use nxtools_clipboard::{
    BUILTIN_ALL_CATEGORY_ID, ClipboardAction, ClipboardCapturePolicy, ClipboardCategory,
    ClipboardDatabaseBackupReport, ClipboardDatabaseCompactionReport, ClipboardHistoryError,
    ClipboardHistoryPage, ClipboardItem, ClipboardKind,
    ClipboardNativeFormat as StoredClipboardNativeFormat, ClipboardOpenTargetKind,
    ClipboardRetentionPolicy, ClipboardSearchScope, action_text_payload,
};
use nxtools_core::{
    AppConfig, ClipboardClickAction, ClipboardClickModifier, ClipboardHotKey,
    ClipboardHotKeyModifier, ClipboardImagePasteProfile, ClipboardSettings, LinkPanelPosition,
    ProcessBlacklistFeature, ProcessBlacklistSettings, clipboard_image_paste_profile,
};
use nxtools_ipc::{PlatformBridge, PlatformBridgeError, PlatformNotification};
use nxtools_protocol::{
    BeginWindowMoveCommand, BeginWindowResizeCommand, ClipboardArchivePickerMode, ClipboardCapture,
    ClipboardNativeFormat as ProtocolClipboardNativeFormat, ClipboardPanelRequest,
    ConfigureClipboardCommand, ForegroundContext, OpenShellTargetCommand, PasteFilesCommand,
    PasteImageCommand, PasteNativeFormatsCommand, PasteTextCommand,
    PickClipboardArchivePathCommand, PickClipboardStorageDirectoryCommand, PlatformCommand,
    PlatformEvent, SetClipboardHotKeyRecordingCommand, SetWindowBoundsCommand,
    SetWindowNoActivateCommand, SetWindowPositionCommand, SetWindowShadowCommand,
    SetWindowTopmostCommand, SetWindowVisibilityCommand, ShellOpenAction,
};
#[cfg(test)]
use sha2::{Digest, Sha256};
use tracing::{error, info, warn};

use crate::notifications::{NotificationAction, NotificationCenter, NotificationDetail};
use crate::popup_lifecycle::{PopupCloseReason, PopupLifecycle, TopmostResolution};
use crate::tools::link_detector::DetectedLinkKind;
use crate::tools::link_panel::LinkPanelRoute;
use crate::ui::metrics::SPACING_24;
use crate::ui::{
    ClipboardPanel, ClipboardPreview, PreviewLayout, native_window_handle, panel_bounds,
};

const HISTORY_PAGE_SIZE: usize = 80;
const MAX_PANEL_REQUEST_AGE_MS: i64 = 2_000;
const CF_UNICODETEXT: u32 = 13;
const CF_HDROP: u32 = 15;
const CF_DIB: u32 = 8;
const CF_DIBV5: u32 = 17;
const PREVIEW_HOVER_DELAY: Duration = Duration::from_millis(800);
const PREVIEW_LEAVE_DELAY: Duration = Duration::from_millis(90);
const PREVIEW_SCROLL_RESUME_DELAY: Duration = Duration::from_millis(260);
mod capture_pause;
mod currency_tool;
pub(crate) mod extension_adapter;
mod link_detection;
mod link_placement;
mod popup_activation;
pub(crate) use capture_pause::CapturePauseSnapshot;
pub(crate) use currency_tool::CurrencyStatus;
#[path = "clipboard_preview_presentation.rs"]
mod preview_presentation;
#[cfg(test)]
#[path = "clipboard_resize_tests.rs"]
mod resize_tests;
const PANEL_SIZE_SAVE_DELAY: Duration = Duration::from_millis(350);
const PREVIEW_STAGING_COORDINATE: i32 = -32_000;
const RETENTION_INTERVAL_MS: i64 = 60 * 60 * 1000;
const ARCHIVE_NOTIFICATION_ID: &str = "clipboard.archive";
const DATABASE_NOTIFICATION_ID: &str = "clipboard.database";
const STORAGE_NOTIFICATION_ID: &str = "clipboard.storage";

#[cfg(test)]
fn remove_managed_clipboard_asset(asset_directory: &Path, candidate: &str) -> bool {
    let Ok(asset_directory) = asset_directory.canonicalize() else {
        return false;
    };
    let Ok(candidate) = Path::new(candidate).canonicalize() else {
        return false;
    };
    if !candidate.is_file() || !candidate.starts_with(&asset_directory) {
        return false;
    }
    std::fs::remove_file(candidate).is_ok()
}

pub struct ClipboardApplicationGlobal {
    _application: Entity<ClipboardApplication>,
}

impl ClipboardApplicationGlobal {
    pub fn new(application: Entity<ClipboardApplication>) -> Self {
        Self {
            _application: application,
        }
    }
    pub(crate) fn application(&self) -> Entity<ClipboardApplication> {
        self._application.clone()
    }
    pub(crate) fn context_menu_platform(&self, cx: &gpui_kit::App) -> Rc<PlatformBridge> {
        self._application.read(cx).platform.clone()
    }
    pub(crate) fn context_menu_source_session(
        &self,
        source: AnyWindowHandle,
        cx: &gpui_kit::App,
    ) -> Option<uuid::Uuid> {
        self._application
            .read(cx)
            .popup
            .lifecycle
            .current()
            .filter(|popup| popup.window == source)
            .map(|popup| popup.session_id)
    }
}

impl Global for ClipboardApplicationGlobal {}

pub struct ClipboardApplication {
    history_worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    platform: Rc<PlatformBridge>,
    config: AppConfig,
    config_path: PathBuf,
    notifications: Entity<NotificationCenter>,
    downloads: Option<WeakEntity<crate::downloads::DownloadApplication>>,
    last_cleanup_at_unix_ms: i64,
    pending_configuration: HashSet<uuid::Uuid>,
    latest_configuration: Option<(uuid::Uuid, Instant)>,
    feature_save_error: Option<String>,
    feature_events_after: i64,
    pending_blacklist_configuration: HashMap<uuid::Uuid, (uuid::Uuid, std::time::Instant)>,
    deferred_blacklist_correlation: Option<uuid::Uuid>,
    pending_hotkey_configuration: Option<(uuid::Uuid, ClipboardHotKey)>,
    pending_hotkey_recording: Option<(uuid::Uuid, bool)>,
    hotkey_recording_active: bool,
    hotkey_recording_desired: bool,
    clipboard_configuration_dirty: bool,
    clipboard_configuration_status: String,
    pending_archive_picker: Option<(uuid::Uuid, ClipboardArchiveOperation)>,
    pending_storage_picker: Option<uuid::Uuid>,
    management_operation_active: bool,
    deferred_captures: Vec<ClipboardCapture>,
    capture_suppression: Option<ClipboardCaptureSuppression>,
    capture_pause: capture_pause::CapturePauseState,
    popup: PopupSession,
    preview: PreviewSession,
    currency: currency_tool::CurrencyState,
    link_detection: link_detection::LinkDetectionState,
    link_panel: crate::tools::link_panel::LinkPanelSession,
    pending_link_placement: Option<link_placement::PendingLinkPlacement>,
    popup_monitor: Option<nxtools_protocol::CursorMonitorSnapshot>,
    pending_link_panel_actions: HashMap<uuid::Uuid, PendingLinkPanelAction>,
    pending_extension_copies: HashMap<uuid::Uuid, extension_adapter::PendingExtensionCopy>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardArchiveOperation {
    Export,
    Import,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardDatabaseOperation {
    Backup,
    Compact,
}

enum ClipboardDatabaseOperationResult {
    Backup(ClipboardDatabaseBackupReport),
    Compact(ClipboardDatabaseCompactionReport),
}

#[derive(Clone)]
struct PopupWindow {
    window: AnyWindowHandle,
    panel: WeakEntity<ClipboardPanel>,
    native_window_handle: i64,
    session_id: uuid::Uuid,
    summon_requested_at_unix_ms: i64,
    summon_event_sequence: u64,
}

struct PendingPopupVisibility {
    window: AnyWindowHandle,
    native_window_handle: i64,
    visible: bool,
    session_id: uuid::Uuid,
    close_reason: Option<PopupCloseReason>,
    started_at: Instant,
}

#[derive(Default)]
struct PopupSession {
    lifecycle: PopupLifecycle<PopupWindow>,
    pending_visibility: HashMap<uuid::Uuid, PendingPopupVisibility>,
    pending_activation: Option<popup_activation::PendingPopupActivation>,
    load_generation: u64,
    pending_load_generation: Option<u64>,
    // Armed only after native reveal confirmation, never for hidden creation bounds.
    size_tracking_session: Option<uuid::Uuid>,
    pending_size: Option<(uuid::Uuid, nxtools_core::ClipboardPanelSize)>,
    size_revision: u64,
    size_save_active: bool,
}

#[derive(Debug)]
struct ClipboardPopupSnapshot {
    page: ClipboardHistoryPage,
    summary_counts: HashMap<i64, usize>,
    categories: Vec<ClipboardCategory>,
    category_sources: HashMap<String, HashSet<String>>,
    source_process_names: Vec<String>,
}
#[derive(Default)]
struct PreviewSession {
    window: Option<PreviewWindow>,
    native_window_handle: i64,
    visible: bool,
    displayed_item_id: Option<i64>,
    generation: u64,
    scroll_generation: u64,
    hovered_item_id: Option<i64>,
    pointer_inside: bool,
    scroll_suppressed: bool,
    candidate: Option<PreviewCandidate>,
    pending_stage: Option<PendingPreviewStage>,
    timing: Option<PreviewTiming>,
}

struct PreviewTiming {
    correlation_id: uuid::Uuid,
    started_at: Instant,
    native_started_at: Option<Instant>,
    step_started_at: Option<Instant>,
}

#[derive(Clone)]
struct PreviewCandidate {
    item: ClipboardItem,
    panel_bounds: Bounds<Pixels>,
    pointer_position: Point<Pixels>,
    panel_scale_factor: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewStagePhase {
    MeasureInitial,
    MeasureReuse,
    MoveOffscreen,
    ReuseBounds,
    ShowOffscreen,
    ReuseNoActivate,
    InitialNoActivate,
    InitialBounds,
    InitialTopmost,
    InitialShow,
}

struct PendingPreviewStage {
    command_id: uuid::Uuid,
    native_window_handle: i64,
    phase: PreviewStagePhase,
    generation: u64,
    item: ClipboardItem,
    layout: PreviewLayout,
    preview_size: Size<Pixels>,
    bounds: Bounds<Pixels>,
    panel_scale_factor: f32,
    content_measured: bool,
}

#[derive(Debug, Clone)]
struct ClipboardCaptureSuppression {
    since_unix_ms: i64,
    until_unix_ms: i64,
    action: ClipboardAction,
}

#[derive(Debug)]
struct PendingLinkPanelAction {
    correlation_id: uuid::Uuid,
    target_kind: DetectedLinkKind,
    action: ShellOpenAction,
    value_length: usize,
    started_at: Instant,
}

struct PreviewWindow {
    window: AnyWindowHandle,
    view: WeakEntity<ClipboardPreview>,
}

fn resolve_pending_hotkey(
    pending: &mut Option<(uuid::Uuid, ClipboardHotKey)>,
    correlation_id: uuid::Uuid,
    success: bool,
) -> Option<Result<ClipboardHotKey, ()>> {
    if pending
        .as_ref()
        .is_none_or(|(command_id, _)| *command_id != correlation_id)
    {
        return None;
    }
    let (_, hotkey) = pending.take().expect("matching pending hotkey must exist");
    Some(if success { Ok(hotkey) } else { Err(()) })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PreviewOpenAction {
    Ignore,
    Reuse,
    Create,
}

fn preview_open_action(
    window_exists: bool,
    visible: bool,
    displayed_item_id: Option<i64>,
    requested_item_id: i64,
) -> PreviewOpenAction {
    if !window_exists {
        PreviewOpenAction::Create
    } else if visible && displayed_item_id == Some(requested_item_id) {
        PreviewOpenAction::Ignore
    } else {
        PreviewOpenAction::Reuse
    }
}

fn should_request_popup_topmost(window_opened: bool, native_window_handle: i64) -> bool {
    window_opened && native_window_handle != 0
}

fn preview_bounds(
    panel_bounds: Bounds<Pixels>,
    preview_size: Size<Pixels>,
    screen: Bounds<Pixels>,
    pointer_position: Point<Pixels>,
) -> Bounds<Pixels> {
    let gap = px(10.0);
    let x = if panel_bounds.right() + gap + preview_size.width <= screen.right() {
        panel_bounds.right() + gap
    } else {
        (panel_bounds.origin.x - gap - preview_size.width).max(screen.origin.x)
    };
    let max_y = (screen.bottom() - preview_size.height).max(screen.origin.y);
    let preferred_y = pointer_position.y - preview_size.height / 3.0;
    let y = preferred_y.max(screen.origin.y).min(max_y);
    let max_x = (screen.right() - preview_size.width).max(screen.origin.x);
    Bounds::new(point(x.clamp(screen.origin.x, max_x), y), preview_size)
}

fn preview_device_origin(bounds: Bounds<Pixels>, scale_factor: f32) -> (i32, i32) {
    let device_bounds = bounds.to_device_pixels(scale_factor);
    (device_bounds.origin.x.into(), device_bounds.origin.y.into())
}

async fn load_clipboard_popup_snapshot(
    worker: &crate::clipboard_worker::ClipboardCaptureWorkerHandle,
    query: &str,
    category_id: &str,
    scope: ClipboardSearchScope,
) -> Result<ClipboardPopupSnapshot, ClipboardHistoryError> {
    let page = worker
        .query_summary_page_async(
            query,
            category_id,
            scope,
            0,
            HISTORY_PAGE_SIZE,
            None,
            None,
            Default::default(),
        )
        .await?;
    let summary_counts = page
        .items
        .iter()
        .map(|row| (row.item.id, row.text_character_count))
        .collect();
    let page = ClipboardHistoryPage {
        items: page.items.into_iter().map(|row| row.item).collect(),
        total: page.total,
        has_more: page.has_more,
    };
    let categories = worker.categories_async().await?;
    let mut category_sources = HashMap::new();
    for category in categories
        .iter()
        .filter(|category| category.kind == nxtools_clipboard::ClipboardCategoryKind::Custom)
    {
        let sources = worker.category_sources_async(&category.id).await?;
        category_sources.insert(
            category.id.clone(),
            sources
                .into_iter()
                .map(|source| source.to_lowercase())
                .collect(),
        );
    }
    let source_process_names = worker.source_process_names_async().await?;
    Ok(ClipboardPopupSnapshot {
        page,
        summary_counts,
        categories,
        category_sources,
        source_process_names,
    })
}
fn clamp_link_panel_pixels(value: Pixels, minimum: Pixels, maximum: Pixels) -> Pixels {
    if maximum < minimum || value < minimum {
        minimum
    } else if value > maximum {
        maximum
    } else {
        value
    }
}

fn link_panel_bounds(
    position: LinkPanelPosition,
    panel_size: Size<Pixels>,
    screen: Bounds<Pixels>,
) -> Bounds<Pixels> {
    let panel_size = size(
        panel_size.width.min(screen.size.width),
        panel_size.height.min(screen.size.height),
    );
    let inset = px(SPACING_24);
    let x = match position {
        LinkPanelPosition::TopLeft => screen.left() + inset,
        LinkPanelPosition::Center => screen.left() + (screen.size.width - panel_size.width) / 2.0,
        LinkPanelPosition::TopRight => screen.right() - panel_size.width - inset,
    };
    let y = match position {
        LinkPanelPosition::Center => screen.top() + (screen.size.height - panel_size.height) / 2.0,
        LinkPanelPosition::TopLeft | LinkPanelPosition::TopRight => screen.top() + inset,
    };
    Bounds::new(
        point(
            clamp_link_panel_pixels(x, screen.left(), screen.right() - panel_size.width),
            clamp_link_panel_pixels(y, screen.top(), screen.bottom() - panel_size.height),
        ),
        panel_size,
    )
}
fn clipboard_capture_has_content(capture: &ClipboardCapture) -> bool {
    !capture.paths.is_empty()
        || capture.image.is_some()
        || capture
            .text
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty())
}

fn remove_clipboard_capture_staging(capture: &ClipboardCapture) {
    if let Some(image) = &capture.image {
        let _ = std::fs::remove_file(&image.staging_path);
    }
    for format in &capture.native_formats {
        let _ = std::fs::remove_file(&format.data_path);
    }
    if let Some(path) = &capture.source_icon_staging_path {
        let _ = std::fs::remove_file(path);
    }
}
impl ClipboardApplication {
    #[allow(clippy::too_many_arguments, clippy::let_and_return)]
    pub fn new(
        history_worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
        platform: Rc<PlatformBridge>,
        config: AppConfig,
        config_path: PathBuf,
        notifications: Entity<NotificationCenter>,
        _cx: &mut Context<Self>,
    ) -> Self {
        let application = Self {
            history_worker,
            platform,
            config,
            config_path,
            notifications,
            downloads: None,
            last_cleanup_at_unix_ms: 0,
            pending_configuration: HashSet::new(),
            latest_configuration: None,
            feature_save_error: None,
            feature_events_after: i64::MIN,
            pending_blacklist_configuration: HashMap::new(),
            deferred_blacklist_correlation: None,
            pending_hotkey_configuration: None,
            pending_hotkey_recording: None,
            hotkey_recording_active: false,
            hotkey_recording_desired: false,
            clipboard_configuration_dirty: false,
            clipboard_configuration_status: "等待 Windows 适配器".into(),
            pending_archive_picker: None,
            pending_storage_picker: None,
            management_operation_active: false,
            deferred_captures: Vec::new(),
            capture_suppression: None,
            capture_pause: capture_pause::CapturePauseState::default(),
            popup: PopupSession::default(),
            preview: PreviewSession::default(),
            currency: currency_tool::CurrencyState::default(),
            link_detection: link_detection::LinkDetectionState::default(),
            link_panel: crate::tools::link_panel::LinkPanelSession::default(),
            pending_link_placement: None,
            popup_monitor: None,
            pending_link_panel_actions: HashMap::new(),
            pending_extension_copies: HashMap::new(),
        };
        application
    }

    pub(crate) fn bind_downloads(
        &mut self,
        downloads: WeakEntity<crate::downloads::DownloadApplication>,
    ) {
        self.downloads = Some(downloads);
    }

    pub(crate) fn take_link_panel_download_owner(
        &mut self,
        panel: AnyWindowHandle,
    ) -> Option<WeakEntity<crate::downloads::DownloadApplication>> {
        if !self.forget_link_panel_if_current(panel) {
            return None;
        }
        self.downloads.clone()
    }

    pub(crate) fn clipboard_settings(&self) -> ClipboardSettings {
        self.config.clipboard.clone()
    }

    /// Publishes already-durable global rules; this consumer never writes settings.
    pub(crate) fn apply_live_blacklist(
        &mut self,
        settings: ProcessBlacklistSettings,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        let started_at = std::time::Instant::now();
        self.config.set_process_blacklist_settings(settings);
        self.invalidate_link_detection();
        self.link_panel.last_fingerprint = None;
        info!(event = "global.blacklist.clipboard_updated", %correlation_id,
            rule_count = self.config.process_blacklist.entries().len(),
            excluded_count = self.config.clipboard.excluded_processes.len(),
            elapsed_ms = started_at.elapsed().as_millis(), "clipboard live rules updated");
        if self.pending_hotkey_configuration.is_some() {
            self.clipboard_configuration_dirty = true;
            self.deferred_blacklist_correlation = Some(correlation_id);
        } else {
            let settings = self.config.clipboard.clone();
            if let Some(command_id) =
                self.send_clipboard_configuration_correlated(&settings, Some(correlation_id))
            {
                self.schedule_blacklist_configuration_timeout(command_id, cx);
            }
        }
        cx.notify();
    }

    fn schedule_blacklist_configuration_timeout(
        &self,
        command_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(5))
                .await;
            let _ = owner.update(cx, |owner, cx| {
                if let Some((correlation_id, started_at)) =
                    owner.pending_blacklist_configuration.remove(&command_id)
                {
                    warn!(event = "global.blacklist.clipboard_native_failed", %correlation_id,
                        failure_kind = "timeout", elapsed_ms = started_at.elapsed().as_millis(),
                        "Windows did not confirm clipboard rules");
                    owner.clipboard_configuration_status =
                        "规则已保存，Windows 尚未确认；请重启后重试。".into();
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn abandon_blacklist_configuration(&mut self) {
        for (_, (correlation_id, started_at)) in self.pending_blacklist_configuration.drain() {
            warn!(event = "global.blacklist.clipboard_native_failed", %correlation_id,
                failure_kind = "connection_reset", elapsed_ms = started_at.elapsed().as_millis(),
                "clipboard rules await platform reconnection");
        }
    }

    pub(crate) fn clipboard_configuration_status(&self) -> &str {
        &self.clipboard_configuration_status
    }

    pub(crate) fn feature_configuration_status(&self) -> &str {
        self.feature_save_error.as_deref().unwrap_or_else(|| {
            if self.latest_configuration.is_some() {
                "正在应用"
            } else {
                &self.clipboard_configuration_status
            }
        })
    }

    pub(crate) fn set_feature_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.clipboard.enabled == enabled {
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        info!(event = "clipboard.enablement.accepted", %correlation_id, enabled);
        let mut settings = self.clipboard_settings();
        settings.enabled = enabled;
        if let Err(error) = self.save_clipboard_settings(settings) {
            self.feature_save_error = Some(format!("保存失败：{error}"));
            warn!(event = "clipboard.enablement.failed", %correlation_id, enabled,
                stage = "save", elapsed_ms = started.elapsed().as_millis());
            cx.notify();
            return;
        }
        self.feature_save_error = None;
        self.feature_events_after = nxtools_ipc::unix_time_ms();
        self.pending_hotkey_configuration = None;
        if !enabled {
            self.set_clipboard_hotkey_recording(false);
            self.retire_feature_surfaces(cx);
        }
        info!(event = "clipboard.enablement.saved", %correlation_id, enabled,
            elapsed_ms = started.elapsed().as_millis());
        let settings = self.clipboard_settings();
        if let Some(id) = self.queue_clipboard_configuration(&settings, correlation_id, None) {
            self.schedule_feature_configuration_timeout(id, cx);
        }
        cx.notify();
    }

    fn retire_feature_surfaces(&mut self, cx: &mut Context<Self>) {
        self.retire_link_placement("disabled", cx);
        self.popup.load_generation = self.popup.load_generation.wrapping_add(1).max(1);
        self.popup.pending_load_generation = None;
        self.popup.size_tracking_session = None;
        self.abandon_pending_popup_visibility("disabled");
        let previous = self.popup.lifecycle.reset_pending();
        let current = self.popup.lifecycle.take_current();
        self.dismiss_preview(cx);
        let preview = self.preview.window.take().map(|preview| preview.window);
        self.preview.native_window_handle = 0;
        self.preview.displayed_item_id = None;
        self.preview.scroll_generation = self.preview.scroll_generation.wrapping_add(1);
        self.preview.scroll_suppressed = false;
        let link = self.link_panel.window.take();
        self.forget_link_panel();
        self.link_panel.native_window_handle = 0;
        self.link_panel.last_fingerprint = None;
        self.link_panel.last_panel_copied = None;
        self.abandon_pending_link_panel_actions("disabled");
        self.invalidate_link_detection();
        self.invalidate_currency_capture();
        for capture in self.deferred_captures.drain(..) {
            remove_clipboard_capture_staging(&capture);
        }
        self.capture_suppression = None;
        for window in current
            .into_iter()
            .chain(previous)
            .map(|popup| popup.window)
            .chain(preview)
            .chain(link)
        {
            let _ = window.update(cx, |_, window, cx| {
                crate::ui::components::native_context_menu::dismiss_window(window, cx);
                window.remove_window();
            });
        }
    }

    fn schedule_feature_configuration_timeout(&self, id: uuid::Uuid, cx: &mut Context<Self>) {
        cx.spawn(async move |owner, cx| {
            cx.background_executor().timer(Duration::from_secs(5)).await;
            let _ = owner.update(cx, |owner, cx| {
                if owner
                    .latest_configuration
                    .is_some_and(|(current, _)| current == id)
                {
                    let (_, started) = owner
                        .latest_configuration
                        .take()
                        .expect("matching configuration");
                    owner.pending_configuration.remove(&id);
                    owner.clipboard_configuration_status = "应用失败：Windows 未确认".into();
                    warn!(event = "clipboard.configuration.failed", correlation_id = %id,
                        stage = "timeout", elapsed_ms = started.elapsed().as_millis());
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn admits_feature_event(&self, event_unix_ms: i64) -> bool {
        self.config.clipboard.enabled && event_unix_ms > self.feature_events_after
    }

    pub(crate) fn clipboard_management_busy(&self) -> bool {
        self.pending_archive_picker.is_some()
            || self.pending_storage_picker.is_some()
            || self.management_operation_active
    }

    fn notify_progress(
        &self,
        id: &'static str,
        title: &'static str,
        message: impl Into<gpui_kit::SharedString>,
        cx: &mut Context<Self>,
    ) {
        let message = message.into();
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_progress(id, title, message, cx);
        });
    }

    fn notify_info(
        &self,
        id: &'static str,
        title: &'static str,
        message: impl Into<gpui_kit::SharedString>,
        cx: &mut Context<Self>,
    ) {
        let message = message.into();
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_info(id, title, message, cx);
        });
    }

    fn notify_success(
        &self,
        id: &'static str,
        title: &'static str,
        message: impl Into<gpui_kit::SharedString>,
        cx: &mut Context<Self>,
    ) {
        let message = message.into();
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_success(id, title, message, cx);
        });
    }

    fn notify_success_with_details(
        &self,
        id: &'static str,
        title: &'static str,
        message: impl Into<gpui_kit::SharedString>,
        details: Vec<NotificationDetail>,
        action: Option<NotificationAction>,
        cx: &mut Context<Self>,
    ) {
        let message = message.into();
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_success_with_details(id, title, message, details, action, cx);
        });
    }

    fn notify_error(
        &self,
        id: &'static str,
        title: &'static str,
        message: impl Into<gpui_kit::SharedString>,
        cx: &mut Context<Self>,
    ) {
        let message = message.into();
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_error(id, title, message, cx);
        });
    }

    pub(crate) fn clipboard_storage_directory(&self) -> String {
        let database_path = self.history_worker.database_path();
        database_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_string_lossy()
            .into_owned()
    }
    pub(crate) fn clipboard_worker_handle(
        &self,
    ) -> crate::clipboard_worker::ClipboardCaptureWorkerHandle {
        self.history_worker.clone()
    }

    pub(crate) fn request_clipboard_export(
        &mut self,
        source_window_handle: i64,
        cx: &mut Context<Self>,
    ) {
        self.request_clipboard_archive_path(
            ClipboardArchiveOperation::Export,
            source_window_handle,
            cx,
        );
    }

    pub(crate) fn request_clipboard_import(
        &mut self,
        source_window_handle: i64,
        cx: &mut Context<Self>,
    ) {
        self.request_clipboard_archive_path(
            ClipboardArchiveOperation::Import,
            source_window_handle,
            cx,
        );
    }

    pub(crate) fn request_clipboard_database_backup(&mut self, cx: &mut Context<Self>) {
        self.start_clipboard_database_operation(ClipboardDatabaseOperation::Backup, cx);
    }

    pub(crate) fn request_clipboard_database_compaction(&mut self, cx: &mut Context<Self>) {
        self.start_clipboard_database_operation(ClipboardDatabaseOperation::Compact, cx);
    }

    pub(crate) fn request_clipboard_storage_migration(
        &mut self,
        source_window_handle: i64,
        cx: &mut Context<Self>,
    ) {
        if self.clipboard_management_busy() {
            self.notify_info(
                STORAGE_NOTIFICATION_ID,
                "存储迁移",
                "已有剪贴板数据操作正在进行",
                cx,
            );
            return;
        }
        let current_directory = self.clipboard_storage_directory();
        match self
            .platform
            .send_command(PlatformCommand::PickClipboardStorageDirectory(
                PickClipboardStorageDirectoryCommand {
                    current_directory,
                    source_window_handle,
                    source_process_id: std::process::id(),
                },
            )) {
            Ok(command_id) => {
                self.pending_storage_picker = Some(command_id);
            }
            Err(error) => {
                self.notify_error(
                    STORAGE_NOTIFICATION_ID,
                    "存储迁移",
                    format!("无法打开目录选择器：{error}"),
                    cx,
                );
            }
        }
    }

    fn request_clipboard_archive_path(
        &mut self,
        operation: ClipboardArchiveOperation,
        source_window_handle: i64,
        cx: &mut Context<Self>,
    ) {
        if self.clipboard_management_busy() {
            self.notify_info(
                ARCHIVE_NOTIFICATION_ID,
                "历史压缩包",
                "已有剪贴板数据操作正在进行",
                cx,
            );
            return;
        }
        let mode = match operation {
            ClipboardArchiveOperation::Export => ClipboardArchivePickerMode::Export,
            ClipboardArchiveOperation::Import => ClipboardArchivePickerMode::Import,
        };
        match self
            .platform
            .send_command(PlatformCommand::PickClipboardArchivePath(
                PickClipboardArchivePathCommand {
                    mode,
                    suggested_file_name: "NxTools-clipboard.nxtclip".into(),
                    source_window_handle,
                    source_process_id: std::process::id(),
                },
            )) {
            Ok(command_id) => {
                self.pending_archive_picker = Some((command_id, operation));
            }
            Err(error) => {
                self.notify_error(
                    ARCHIVE_NOTIFICATION_ID,
                    "历史压缩包",
                    format!("无法打开文件选择器：{error}"),
                    cx,
                );
            }
        }
    }

    fn start_clipboard_archive_operation(
        &mut self,
        operation: ClipboardArchiveOperation,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let history_worker = self.history_worker.clone();
        self.management_operation_active = true;
        self.notify_progress(
            ARCHIVE_NOTIFICATION_ID,
            "历史压缩包",
            match operation {
                ClipboardArchiveOperation::Export => "正在导出便携压缩包…",
                ClipboardArchiveOperation::Import => "正在导入并合并历史…",
            },
            cx,
        );
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    match operation {
                        ClipboardArchiveOperation::Export => {
                            let file_name = path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_else(|| path.to_string_lossy().into_owned());
                            history_worker
                                .export_archive(path)
                                .map(move |report| {
                                    format!(
                                        "已导出 {} 条记录、{} 个资源 · {}",
                                        report.item_count, report.asset_count, file_name
                                    )
                                })
                                .map_err(anyhow::Error::msg)
                        }
                        ClipboardArchiveOperation::Import => history_worker
                            .import_archive(path)
                            .map(|report| {
                                format!(
                                    "导入完成：新增 {}，合并 {}，跳过 {}",
                                    report.inserted, report.updated, report.skipped
                                )
                            })
                            .map_err(anyhow::Error::msg),
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(message) => {
                        this.notify_success(
                            ARCHIVE_NOTIFICATION_ID,
                            "历史压缩包",
                            message.clone(),
                            cx,
                        );
                    }
                    Err(error) => {
                        let message = format!("数据操作失败：{error}");
                        this.notify_error(
                            ARCHIVE_NOTIFICATION_ID,
                            "历史压缩包",
                            message.clone(),
                            cx,
                        );
                    }
                }
                this.finish_clipboard_management_operation(cx);
            });
        })
        .detach();
    }

    fn start_clipboard_database_operation(
        &mut self,
        operation: ClipboardDatabaseOperation,
        cx: &mut Context<Self>,
    ) {
        if self.clipboard_management_busy() {
            self.notify_info(
                DATABASE_NOTIFICATION_ID,
                "数据库维护",
                "已有剪贴板数据操作正在进行",
                cx,
            );
            return;
        }
        let history_worker = self.history_worker.clone();
        let database_path = history_worker.database_path();
        let backup_directory = database_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        self.management_operation_active = true;
        self.notify_progress(
            DATABASE_NOTIFICATION_ID,
            "数据库维护",
            match operation {
                ClipboardDatabaseOperation::Backup => "正在创建在线备份…",
                ClipboardDatabaseOperation::Compact => "正在备份并压缩数据库…",
            },
            cx,
        );
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    match operation {
                        ClipboardDatabaseOperation::Backup => history_worker
                            .create_database_backup(backup_directory)
                            .map(ClipboardDatabaseOperationResult::Backup)
                            .map_err(anyhow::Error::msg),
                        ClipboardDatabaseOperation::Compact => history_worker
                            .compact_database(backup_directory)
                            .map(ClipboardDatabaseOperationResult::Compact)
                            .map_err(anyhow::Error::msg),
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(result) => {
                        let (title, message, backup_path) = match result {
                            ClipboardDatabaseOperationResult::Backup(report) => (
                                "数据库备份",
                                format!("已创建可恢复备份 · {} KB", report.size_bytes / 1024),
                                report.backup_path,
                            ),
                            ClipboardDatabaseOperationResult::Compact(report) => (
                                "数据库压缩",
                                format!(
                                    "压缩完成 · 释放 {} KB · 已创建恢复点",
                                    report.before_bytes.saturating_sub(report.after_bytes) / 1024
                                ),
                                report.backup_path,
                            ),
                        };
                        let backup_directory = backup_path
                            .parent()
                            .unwrap_or_else(|| Path::new("."))
                            .to_string_lossy()
                            .into_owned();
                        let open_target = backup_directory.clone();
                        let platform = this.platform.clone();
                        let action = NotificationAction::new("打开目录", move |_window, _cx| {
                            if let Err(error) = platform.send_command(
                                PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                                    target: open_target.clone(),
                                    action: ShellOpenAction::Open,
                                }),
                            ) {
                                error!(%error, target = %open_target, "failed to open backup directory");
                            }
                        });
                        this.notify_success_with_details(
                            DATABASE_NOTIFICATION_ID,
                            title,
                            message,
                            vec![NotificationDetail::new("保存位置", backup_directory)],
                            Some(action),
                            cx,
                        );
                    }
                    Err(error) => {
                        let message = format!("数据库维护失败：{error}");
                        this.notify_error(
                            DATABASE_NOTIFICATION_ID,
                            "数据库维护",
                            message.clone(),
                            cx,
                        );
                    }
                }
                this.finish_clipboard_management_operation(cx);
            });
        })
        .detach();
    }

    fn start_clipboard_storage_migration(&mut self, destination: PathBuf, cx: &mut Context<Self>) {
        let history_worker = self.history_worker.clone();
        let config_path = self.config_path.clone();
        self.management_operation_active = true;
        self.notify_progress(
            STORAGE_NOTIFICATION_ID,
            "存储迁移",
            format!("正在迁移到 {}", destination.display()),
            cx,
        );
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    history_worker
                        .migrate_storage(destination, config_path)
                        .map_err(anyhow::Error::msg)
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok((report, merged)) => {
                        this.refresh_committed_migration_config(merged);
                        let message = format!(
                            "迁移完成：{} 条记录、{} 个资源；原目录已保留 · {}",
                            report.item_count,
                            report.asset_count,
                            report.database_path.display()
                        );
                        this.notify_success(STORAGE_NOTIFICATION_ID, "存储迁移", message, cx);
                    }
                    Err(error) => {
                        let message = format!("存储迁移失败：{error}");
                        this.notify_error(STORAGE_NOTIFICATION_ID, "存储迁移", message.clone(), cx);
                    }
                }
                this.finish_clipboard_management_operation(cx);
            });
        })
        .detach();
    }

    fn refresh_committed_migration_config(&mut self, committed: AppConfig) {
        // A settings save can land after the worker's commit but before this
        // result callback. Never publish that earlier snapshot over newer edits.
        let latest_size = self.config.clipboard_panel;
        self.config = AppConfig::load_from(&self.config_path).unwrap_or(committed);
        self.config.clipboard_panel = latest_size;
    }

    fn finish_clipboard_management_operation(&mut self, cx: &mut Context<Self>) {
        self.management_operation_active = false;
        for capture in std::mem::take(&mut self.deferred_captures) {
            self.handle_platform_event(PlatformEvent::ClipboardCaptured(capture), cx);
        }
        cx.notify();
    }

    pub(crate) fn clipboard_hotkey_update_pending(&self) -> bool {
        self.pending_hotkey_configuration.is_some()
    }

    pub(crate) fn clipboard_hotkey_recording_ready(&self) -> bool {
        self.hotkey_recording_active && self.pending_hotkey_recording.is_none()
    }

    pub(crate) fn clipboard_hotkey_recording_active(&self) -> bool {
        self.hotkey_recording_active
            || self.hotkey_recording_desired
            || self.pending_hotkey_recording.is_some()
    }

    pub(crate) fn link_detector_enabled(&self) -> bool {
        self.config.clipboard.link_detector_enabled
    }

    /// Queue detection away from GPUI; only the current capture may present
    /// results through the original reusable panel and auto-hide lifecycle.
    pub(crate) fn handle_link_detector_capture(&mut self, text: &str, cx: &mut Context<Self>) {
        if !self.config.clipboard.enabled || !self.config.clipboard.link_detector_enabled {
            return;
        }
        // One capture is delivered on TWO platform connections ~40ms apart;
        // without this guard the second delivery raced the just-opened
        // panel and gpui killed it with "window not found".
        if let Some((copied, at_ms)) = &self.link_panel.last_panel_copied
            && copied == text
            && nxtools_ipc::unix_time_ms() - at_ms < 1_000
        {
            info!(
                event = "tools.link_detector.echo_ignored",
                "ignored the panel's own clipboard write"
            );
            return;
        }
        let fingerprint = (self.config.clipboard.link_detector_enabled, text.to_owned());
        if self.link_panel.last_fingerprint.as_ref() == Some(&fingerprint)
            && nxtools_ipc::unix_time_ms() - self.link_panel.last_fingerprint_at_unix_ms < 500
        {
            return;
        }
        self.link_panel.last_fingerprint = Some(fingerprint);
        self.link_panel.last_fingerprint_at_unix_ms = nxtools_ipc::unix_time_ms();
        self.queue_link_detection(text, cx);
    }

    /// Records the text the panel is about to write so the resulting echo
    /// capture does not reopen the panel (clicks looked like no-ops).
    pub(crate) fn note_panel_copied(&mut self, value: String) {
        self.link_panel.last_panel_copied = Some((value, nxtools_ipc::unix_time_ms()));
    }

    /// Clears panel session state after the window removed itself. The
    /// window-side close must not bounce through close_link_panel: updating
    /// this same window re-entrantly errors and left the panel on screen.
    pub(crate) fn forget_link_panel(&mut self) {
        self.link_panel.window = None;
        self.link_panel.generation += 1;
    }

    /// A deferred callback may retire only the panel window that accepted its action.
    pub(crate) fn forget_link_panel_if_current(&mut self, window: AnyWindowHandle) -> bool {
        if self.link_panel.window != Some(window) {
            return false;
        }
        self.forget_link_panel();
        true
    }

    /// Snapshot of the link panel user settings for the settings window.
    pub(crate) fn link_panel_settings(&self) -> (bool, u64, LinkPanelPosition) {
        (
            self.config.clipboard.link_detector_enabled,
            self.config
                .clipboard
                .link_panel_auto_hide_ms
                .saturating_div(1_000),
            self.config.clipboard.link_panel_position,
        )
    }

    pub(crate) fn show_link_panel(
        &mut self,
        links: Vec<crate::tools::link_panel::DetectedLinkRow>,
        cx: &mut Context<Self>,
    ) {
        if !self.config.clipboard.enabled {
            return;
        }
        let application = cx.entity().downgrade();
        let platform = Rc::clone(&self.platform);
        let link_panel_auto_hide_ms = self.config.clipboard.link_panel_auto_hide_ms;
        let link_panel_position = self.config.clipboard.link_panel_position;
        let panel_size = size(
            px(crate::tools::link_panel::LINK_PANEL_WIDTH),
            px(crate::tools::link_panel::link_panel_height(
                links.len(),
                false,
            )),
        );
        let placement = crate::ui::monitor_placement::cursor_monitor_placement(
            self.link_panel.monitor,
            0,
            96,
            cx,
        );
        let bounds = link_panel_bounds(link_panel_position, panel_size, placement.work_area);
        if let Some(existing) = self.link_panel.window {
            if existing
                .update(cx, |root, window, cx| {
                    let panel = root.downcast::<Root>().ok().and_then(|root| {
                        root.read(cx)
                            .view()
                            .clone()
                            .downcast::<crate::tools::link_panel::LinkPanelView>()
                            .ok()
                    });
                    if let Some(view) = panel {
                        view.update(cx, |view, cx| {
                            view.auto_hide_ms = link_panel_auto_hide_ms;
                            view.swap_links(links.clone(), existing, cx);
                            cx.notify();
                        });
                        true
                    } else {
                        window.remove_window();
                        false
                    }
                })
                .unwrap_or(false)
            {
                let generation = self.link_panel.generation.wrapping_add(1);
                self.link_panel.generation = generation;
                let _ = existing.update(cx, |_, window, _| window.resize(bounds.size));
                return;
            }
            let _ = existing.update(cx, |_, window, _| window.remove_window());
            self.link_panel.window = None;
        }
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_native_slot = Rc::clone(&native_slot);
        let any_handle_slot = Rc::new(RefCell::new(None));
        let opened_any_handle_slot = Rc::clone(&any_handle_slot);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: false,
                show: false,
                kind: WindowKind::PopUp,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                display_id: placement.display_id,
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.LinkPanel".into()),
                window_min_size: None,
                window_decorations: Some(WindowDecorations::Client),
                app_owns_titlebar_drag: false,
                inactive_frame_interval: None,
                icon: None,
                tabbing_identifier: None,
            },
            move |window, cx| {
                let handle = native_window_handle(window).unwrap_or_default();
                *opened_native_slot.borrow_mut() = handle;
                *opened_any_handle_slot.borrow_mut() = Some(window.window_handle());
                if handle != 0 {
                    // The system drop shadow reads as a dirty smudge behind a
                    // small floating hint card; render it flat.
                    let _ = platform.send_command(PlatformCommand::SetWindowShadow(
                        SetWindowShadowCommand {
                            window_handle: handle,
                            enabled: false,
                        },
                    ));
                }
                let window_handle = window.window_handle();
                let panel = cx.new(|cx| {
                    let mut view = crate::tools::link_panel::LinkPanelView {
                        links,
                        application,
                        expanded: false,
                        auto_hide_ms: link_panel_auto_hide_ms,
                        window_handle: Some(window_handle),
                        auto_hide_task: None,
                    };
                    view.arm_auto_hide(window_handle, cx);
                    view
                });
                cx.new(|cx| Root::new(panel, window, cx))
            },
        );
        let window = match opened {
            Ok(window) => window,
            Err(error) => {
                error!(%error, "failed to open the link detector panel window");
                return;
            }
        };
        self.link_panel.native_window_handle = *native_slot.borrow();
        #[cfg(test)]
        if let Some(native) = self.link_panel.test_native_window_handle {
            self.link_panel.native_window_handle = native;
        }
        self.link_panel.window = Some(window.into());
        let generation = self.link_panel.generation.wrapping_add(1);
        self.link_panel.generation = generation;
    }

    pub(crate) fn set_link_detector_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.config.clipboard.link_detector_enabled = enabled;
        self.invalidate_link_detection();
        self.link_panel.last_fingerprint = None;
        self.persist_clipboard_config(cx);
    }

    pub(crate) fn set_link_panel_auto_hide_ms(&mut self, ms: u64, cx: &mut Context<Self>) {
        self.config.clipboard.link_panel_auto_hide_ms = ms.clamp(2_000, 60_000);
        self.persist_clipboard_config(cx);
    }

    pub(crate) fn set_link_panel_position(
        &mut self,
        position: LinkPanelPosition,
        cx: &mut Context<Self>,
    ) {
        self.config.clipboard.link_panel_position = position;
        self.persist_clipboard_config(cx);
    }

    pub(crate) fn dispatch_link_panel_route(
        &mut self,
        route: LinkPanelRoute,
        target: &str,
        target_kind: DetectedLinkKind,
        value_length: usize,
        cx: &mut gpui_kit::App,
    ) {
        if !self.config.clipboard.enabled {
            return;
        }
        if route == LinkPanelRoute::Copy {
            let correlation_id = uuid::Uuid::new_v4();
            let started_at = Instant::now();
            info!(event = "tools.link_panel.copy_started", %correlation_id,
                target_kind = target_kind.label(), value_length,
                "ordinary link panel copy accepted");
            self.note_panel_copied(target.to_owned());
            cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string(target.to_owned()));
            info!(event = "tools.link_panel.copy_completed", %correlation_id,
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                "ordinary link panel copy completed");
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        let Some(command) =
            crate::tools::link_panel::link_panel_shell_command(route, target, target_kind)
        else {
            warn!(
                event = "tools.link_panel.route_rejected",
                %correlation_id,
                target_kind = target_kind.label(),
                ?route,
                value_length,
                reason = "target_kind_mismatch",
                "link panel refused an incompatible route"
            );
            return;
        };
        let action = match route {
            LinkPanelRoute::Browser => ShellOpenAction::Open,
            LinkPanelRoute::OpenShell { action } => action,
            LinkPanelRoute::Copy => unreachable!("copy routes return before shell dispatch"),
        };
        info!(
            event = "tools.link_panel.route_started",
            %correlation_id,
            target_kind = target_kind.label(),
            ?route,
            value_length,
            "link panel open requested"
        );
        // Browser and local targets share the existing C# Shell boundary. This
        // keeps native launch/codes out of Rust and makes disconnected dispatch
        // a typed rejection rather than an untestable cmd/browser side effect.
        match self
            .platform
            .send_command_correlated(command, correlation_id)
        {
            Ok(command_id) => {
                info!(
                    event = "tools.link_panel.shell_accepted",
                    %correlation_id,
                    %command_id,
                    target_kind = target_kind.label(),
                    ?action,
                    value_length,
                    "link panel shell route accepted"
                );
                self.pending_link_panel_actions.insert(
                    command_id,
                    PendingLinkPanelAction {
                        correlation_id,
                        target_kind,
                        action,
                        value_length,
                        started_at,
                    },
                );
            }
            Err(error) => warn!(
                event = "tools.link_panel.shell_rejected",
                %correlation_id,
                target_kind = target_kind.label(),
                ?action,
                value_length,
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                accepted = false,
                reason = %error,
                "link panel shell route was not accepted"
            ),
        }
    }

    fn abandon_pending_link_panel_actions(&mut self, terminal: &'static str) {
        for (command_id, pending) in self.pending_link_panel_actions.drain() {
            warn!(
                event = "tools.link_panel.shell_failed",
                %command_id,
                %pending.correlation_id,
                target_kind = pending.target_kind.label(),
                ?pending.action,
                value_length = pending.value_length,
                elapsed_ms = pending.started_at.elapsed().as_millis() as u64,
                accepted = false,
                reason = terminal,
                "link panel shell route ended before a native result"
            );
        }
    }

    fn persist_clipboard_config(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.save_clipboard_section() {
            error!(%error, "could not persist clipboard tool preferences");
            return;
        }
        info!(event = "tools.clipboard_preferences_saved");
        cx.notify();
    }

    pub(crate) fn set_clipboard_hotkey_recording(&mut self, recording: bool) {
        self.hotkey_recording_desired = recording;
        self.drive_clipboard_hotkey_recording();
    }

    pub(crate) fn set_clipboard_hotkey(&mut self, hotkey: ClipboardHotKey) {
        if self.pending_hotkey_configuration.is_some() {
            self.clipboard_configuration_status = "上一热键仍在应用".into();
            return;
        }
        if !self.clipboard_hotkey_recording_ready() {
            self.clipboard_configuration_status = "热键录制尚未就绪".into();
            return;
        }
        self.hotkey_recording_desired = false;
        self.hotkey_recording_active = false;
        self.apply_clipboard_hotkey(hotkey);
    }

    pub(crate) fn clipboard_hotkey_is_custom(&self) -> bool {
        let default = ClipboardHotKey::default();
        self.config.clipboard.hotkey.modifiers != default.modifiers
            || !self
                .config
                .clipboard
                .hotkey
                .key
                .eq_ignore_ascii_case(&default.key)
    }

    pub(crate) fn can_reset_clipboard_hotkey(&self) -> bool {
        self.clipboard_hotkey_is_custom()
            && self.platform.is_connected()
            && self.pending_configuration.is_empty()
            && !self.clipboard_hotkey_recording_active()
    }

    pub(crate) fn reset_clipboard_hotkey(&mut self) {
        if self.can_reset_clipboard_hotkey() {
            self.apply_clipboard_hotkey(ClipboardHotKey::default());
        }
    }

    fn apply_clipboard_hotkey(&mut self, hotkey: ClipboardHotKey) {
        let mut candidate = self.config.clipboard.clone();
        candidate.hotkey = hotkey.clone();
        if let Some(command_id) = self.send_clipboard_configuration_for(&candidate) {
            self.pending_hotkey_configuration = Some((command_id, hotkey));
            info!(event = "clipboard.hotkey.submitted", correlation_id = %command_id,
                "clipboard binding submitted through original configuration owner");
        }
    }

    pub(crate) fn set_clipboard_modifier_action(
        &mut self,
        modifier: ClipboardClickModifier,
        action: ClipboardClickAction,
    ) {
        self.config.clipboard.modifier_actions.set(modifier, action);
        if self.persist_clipboard_configuration() {
            info!(?modifier, ?action, "clipboard modifier action changed");
        }
    }

    fn drive_clipboard_hotkey_recording(&mut self) {
        if self.pending_hotkey_recording.is_some()
            || self.hotkey_recording_active == self.hotkey_recording_desired
        {
            return;
        }
        let recording = self.hotkey_recording_desired;
        match self
            .platform
            .send_command(PlatformCommand::SetClipboardHotKeyRecording(
                SetClipboardHotKeyRecordingCommand { recording },
            )) {
            Ok(command_id) => {
                self.pending_hotkey_recording = Some((command_id, recording));
                self.clipboard_configuration_status = if recording {
                    "正在暂停全局热键".into()
                } else {
                    "正在恢复全局热键".into()
                };
            }
            Err(error) => {
                self.hotkey_recording_desired = false;
                self.hotkey_recording_active = false;
                self.clipboard_configuration_status = format!("热键录制失败：{error}");
            }
        }
    }

    pub(crate) fn set_clipboard_retention(&mut self, max_items: usize, retention_days: u32) {
        self.config.clipboard.max_items = max_items;
        self.config.clipboard.retention_days = retention_days;
        let _ = self.persist_clipboard_configuration();
        let _ = self.history_worker.set_retention(ClipboardRetentionPolicy {
            max_items: self.config.clipboard.max_items,
            retention_days: self.config.clipboard.retention_days,
        });
        let _ = self.history_worker.enqueue_cleanup(
            ClipboardRetentionPolicy {
                max_items: self.config.clipboard.max_items,
                retention_days: self.config.clipboard.retention_days,
            },
            nxtools_ipc::unix_time_ms(),
        );
    }

    pub(crate) fn set_clipboard_capture_limits(
        &mut self,
        max_text_bytes: u64,
        max_image_bytes: u64,
        max_image_pixels: u64,
    ) {
        self.config.clipboard.max_text_bytes = max_text_bytes;
        self.config.clipboard.max_image_bytes = max_image_bytes;
        self.config.clipboard.max_image_pixels = max_image_pixels;
        self.config.clipboard.normalize();
        let _ = self.persist_clipboard_configuration();
        if self.pending_hotkey_configuration.is_some() {
            self.clipboard_configuration_dirty = true;
            self.clipboard_configuration_status = "等待热键确认后应用".into();
        } else {
            self.send_clipboard_configuration();
        }
    }

    fn persist_clipboard_configuration(&mut self) -> bool {
        match self.save_clipboard_section() {
            Ok(()) => true,
            Err(error) => {
                self.clipboard_configuration_status = format!("保存失败：{error}");
                error!(%error, "failed to persist clipboard configuration");
                false
            }
        }
    }

    /// Persist ordinary settings only; resize persistence independently owns panel size.
    /// Preserve the latest disk storage root owned by the migration worker.
    fn save_clipboard_section(&mut self) -> anyhow::Result<()> {
        self.save_clipboard_settings(self.config.clipboard.clone())
    }

    fn save_clipboard_settings(&mut self, mut clipboard: ClipboardSettings) -> anyhow::Result<()> {
        let merged = AppConfig::update_section(&self.config_path, |config| {
            // Only the worker migration transaction owns the storage root.
            clipboard.storage_directory = config.clipboard.storage_directory.clone();
            config.clipboard = clipboard;
        })?;
        let latest_size = self.config.clipboard_panel;
        self.config = merged;
        self.config.clipboard_panel = latest_size;
        Ok(())
    }

    fn send_clipboard_configuration(&mut self) {
        let settings = self.config.clipboard.clone();
        let _ = self.send_clipboard_configuration_for(&settings);
    }

    fn send_clipboard_configuration_for(
        &mut self,
        settings: &ClipboardSettings,
    ) -> Option<uuid::Uuid> {
        self.send_clipboard_configuration_correlated(settings, None)
    }

    fn send_clipboard_configuration_correlated(
        &mut self,
        settings: &ClipboardSettings,
        correlation_id: Option<uuid::Uuid>,
    ) -> Option<uuid::Uuid> {
        self.queue_clipboard_configuration(settings, uuid::Uuid::new_v4(), correlation_id)
    }

    fn queue_clipboard_configuration(
        &mut self,
        settings: &ClipboardSettings,
        command_id: uuid::Uuid,
        correlation_id: Option<uuid::Uuid>,
    ) -> Option<uuid::Uuid> {
        let command = PlatformCommand::ConfigureClipboard(ConfigureClipboardCommand {
            enabled: settings.enabled,
            hotkey_modifiers: settings
                .hotkey
                .modifiers
                .iter()
                .map(|modifier| match modifier {
                    ClipboardHotKeyModifier::Control => "control",
                    ClipboardHotKeyModifier::Alt => "alt",
                    ClipboardHotKeyModifier::Shift => "shift",
                    ClipboardHotKeyModifier::Win => "win",
                })
                .map(ToOwned::to_owned)
                .collect(),
            hotkey_key: settings.hotkey.key.clone(),
            hotkey_display: settings.hotkey.display.clone(),
            excluded_processes: settings.excluded_processes.clone(),
            suppress_in_fullscreen: self.config.suppress_panel_in_fullscreen(),
            max_text_bytes: settings.max_text_bytes,
            max_image_bytes: settings.max_image_bytes,
            max_image_pixels: settings.max_image_pixels,
        });
        let started = Instant::now();
        if let Some((old_id, old_started)) = self.latest_configuration.take() {
            info!(event = "clipboard.configuration.superseded", correlation_id = %old_id,
                elapsed_ms = old_started.elapsed().as_millis());
        }
        info!(event = "clipboard.configuration.started", correlation_id = %correlation_id.unwrap_or(command_id),
            %command_id, enabled = settings.enabled);
        let queued = match correlation_id {
            Some(correlation_id) => {
                self.platform
                    .send_command_with_id_correlated(command, command_id, correlation_id)
            }
            None => self.platform.send_command_with_id(command, command_id),
        };
        match queued {
            Ok(_) => {
                self.latest_configuration = Some((command_id, started));
                self.pending_configuration.insert(command_id);
                if let Some(correlation_id) = correlation_id {
                    self.pending_blacklist_configuration
                        .insert(command_id, (correlation_id, std::time::Instant::now()));
                    info!(event = "global.blacklist.clipboard_native_started", %correlation_id, %command_id,
                        excluded_count = settings.excluded_processes.len(), "clipboard rules queued for Windows");
                }
                self.clipboard_configuration_status = "正在应用".into();
                Some(command_id)
            }
            Err(PlatformBridgeError::Disconnected) => {
                warn!(event = "clipboard.configuration.failed", correlation_id = %correlation_id.unwrap_or(command_id),
                    %command_id, stage = "disconnected", elapsed_ms = started.elapsed().as_millis());
                if let Some(correlation_id) = correlation_id {
                    warn!(event = "global.blacklist.clipboard_native_failed", %correlation_id,
                        failure_kind = "disconnected", elapsed_ms = 0, "clipboard rules await platform connection");
                }
                self.clipboard_configuration_status = "等待 Windows 适配器".into();
                None
            }
            Err(error) => {
                warn!(event = "clipboard.configuration.failed", correlation_id = %correlation_id.unwrap_or(command_id),
                    %command_id, stage = "enqueue", elapsed_ms = started.elapsed().as_millis());
                if let Some(correlation_id) = correlation_id {
                    warn!(event = "global.blacklist.clipboard_native_failed", %correlation_id,
                        failure_kind = "enqueue", elapsed_ms = 0, "clipboard rules could not be queued");
                }
                self.clipboard_configuration_status = format!("应用失败：{error}");
                None
            }
        }
    }

    fn apply_retention(&mut self, now_unix_ms: i64) {
        let policy = ClipboardRetentionPolicy {
            max_items: self.config.clipboard.max_items,
            retention_days: self.config.clipboard.retention_days,
        };
        match self.history_worker.enqueue_cleanup(policy, now_unix_ms) {
            Ok(()) => self.last_cleanup_at_unix_ms = now_unix_ms,
            Err(error) => error!(%error, "failed to enqueue clipboard retention"),
        }
    }

    fn capture_policy(&self) -> ClipboardCapturePolicy {
        ClipboardCapturePolicy {
            enabled: self.config.clipboard.enabled,
            capture_images: self.config.clipboard.capture_images,
            capture_files: self.config.clipboard.capture_files,
        }
    }

    pub(crate) fn is_process_excluded(&self, process_name: &str) -> bool {
        self.config
            .process_is_blacklisted(process_name, ProcessBlacklistFeature::Clipboard)
    }

    fn arm_capture_suppression(&mut self, action: ClipboardAction) {
        let since_unix_ms = nxtools_ipc::unix_time_ms();
        self.capture_suppression = Some(ClipboardCaptureSuppression {
            since_unix_ms,
            until_unix_ms: since_unix_ms.saturating_add(750),
            action,
        });
    }

    fn suppress_action_capture(&mut self, capture: &ClipboardCapture) -> bool {
        let Some(suppression) = self.capture_suppression.as_ref() else {
            return false;
        };
        let now = nxtools_ipc::unix_time_ms();
        if now > suppression.until_unix_ms {
            self.capture_suppression = None;
            return false;
        }
        if capture.captured_at_unix_ms < suppression.since_unix_ms {
            return false;
        }
        let action = suppression.action;
        self.capture_suppression = None;
        info!(
            ?action,
            "clipboard capture matched a recent history action write"
        );
        true
    }

    pub(crate) fn send_clipboard_action_with_native_formats(
        &mut self,
        entry: &ClipboardItem,
        target: &ForegroundContext,
        action: ClipboardAction,
        native_formats: Vec<StoredClipboardNativeFormat>,
        command_id: uuid::Uuid,
    ) -> anyhow::Result<uuid::Uuid> {
        anyhow::ensure!(
            self.config.clipboard.enabled,
            "clipboard feature is disabled"
        );
        let command = command_for_action(entry, target, action, native_formats)?;
        self.arm_capture_suppression(action);
        match self.platform.send_command_with_id(command, command_id) {
            Ok(accepted_id) => {
                info!(
                    event = "clipboard.action.accepted",
                    operation_id = %command_id,
                    command_id = %accepted_id,
                    item_id = entry.id,
                    ?action,
                    "clipboard action command sent"
                );
                Ok(accepted_id)
            }
            Err(error) => {
                self.capture_suppression = None;
                Err(error.into())
            }
        }
    }
    pub(crate) fn observe_panel_size(
        &mut self,
        session_id: uuid::Uuid,
        window: AnyWindowHandle,
        viewport: Size<Pixels>,
        cx: &mut Context<Self>,
    ) {
        if self.popup.size_tracking_session != Some(session_id)
            || self
                .popup
                .lifecycle
                .current()
                .is_none_or(|popup| popup.session_id != session_id || popup.window != window)
        {
            return;
        }
        let width = viewport.width.as_f32();
        let height = viewport.height.as_f32();
        if !width.is_finite() || !height.is_finite() || width < 280.0 || height < 280.0 {
            return;
        }
        let panel =
            nxtools_core::ClipboardPanelSize::clamped(width.round() as u32, height.round() as u32);
        if self.config.clipboard_panel == panel {
            return;
        }
        // Memory intent is immediate: reopen need not wait for durable IO.
        self.config.clipboard_panel = panel;
        self.popup.pending_size = Some((session_id, panel));
        self.popup.size_revision = self.popup.size_revision.wrapping_add(1);
        if self.popup.size_save_active {
            return;
        }
        self.popup.size_save_active = true;
        let config_path = self.config_path.clone();
        cx.spawn(async move |this, cx| {
            loop {
                let Ok(revision) = this.update(cx, |app, _| app.popup.size_revision) else {
                    return;
                };
                cx.background_executor().timer(PANEL_SIZE_SAVE_DELAY).await;
                let Ok(pending) = this.update(cx, |app, _| {
                    if app.popup.size_revision != revision { return None; }
                    app.popup.pending_size.map(|pending| (revision, pending))
                }) else { return; };
                let Some((revision, (session_id, panel))) = pending else { continue; };
                let started_at = Instant::now();
                info!(event = "clipboard.panel_size.save_started", %session_id, revision,
                    width = panel.width, height = panel.height);
                let path = config_path.clone();
                let result = cx.background_executor().spawn(async move {
                    AppConfig::update_section(&path, |config| config.clipboard_panel = panel)
                }).await;
                let Ok(done) = this.update(cx, |app, cx| {
                    match result {
                        Ok(_) => info!(event = "clipboard.panel_size.save_completed", %session_id,
                            revision, elapsed_ms = started_at.elapsed().as_millis() as u64),
                        Err(_) => {
                            warn!(event = "clipboard.panel_size.save_failed", %session_id, revision,
                                reason = "config_commit", elapsed_ms = started_at.elapsed().as_millis() as u64);
                            let notifications = app.notifications.downgrade();
                            cx.defer(move |cx| {
                                let _ = notifications.update(cx, |notifications, cx| {
                                    notifications.show_error("clipboard.panel_size", "剪贴板窗口尺寸保存失败",
                                        "本次窗口尺寸仍可使用，但重启后可能恢复旧尺寸；请检查配置文件后重新调整窗口。", cx);
                                });
                            });
                        }
                    }
                    // Never replace a newer in-memory size/settings snapshot with this merge.
                    if app.popup.size_revision == revision {
                        app.popup.pending_size = None;
                        app.popup.size_save_active = false;
                        true
                    } else { false }
                }) else { return; };
                if done { return; }
            }
        }).detach();
    }

    pub(crate) fn hide_popup(
        &mut self,
        popup_window: AnyWindowHandle,
        native_window_handle: i64,
        session_id: uuid::Uuid,
        reason: PopupCloseReason,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        self.retire_popup_activation("hide_requested");
        self.popup
            .pending_visibility
            .retain(|_, pending| pending.session_id != session_id || !pending.visible);
        let started_at = Instant::now();
        let command_id = match self.platform.send_command_correlated(
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: native_window_handle,
                visible: false,
            }),
            session_id,
        ) {
            Ok(command_id) => command_id,
            Err(error) => {
                warn!(
                    event = "clipboard.popup.close_request_failed",
                    %session_id,
                    reason = reason.kind(),
                    details = ?reason,
                    window_handle = native_window_handle,
                    elapsed_ms = started_at.elapsed().as_millis() as u64,
                    %error,
                );
                return Err(error);
            }
        };
        if self.popup.size_tracking_session == Some(session_id) {
            self.popup.size_tracking_session = None;
        }
        self.popup.pending_visibility.insert(
            command_id,
            PendingPopupVisibility {
                window: popup_window,
                visible: false,
                native_window_handle,
                session_id,
                close_reason: Some(reason.clone()),
                started_at,
            },
        );
        info!(
            event = "clipboard.popup.close_requested",
            %session_id,
            %command_id,
            reason = reason.kind(),
            details = ?reason,
            window_handle = native_window_handle,
        );
        Ok(command_id)
    }

    fn abandon_pending_popup_visibility(&mut self, terminal: &'static str) {
        self.retire_popup_activation(terminal);
        for (_, pending) in self.popup.pending_visibility.drain() {
            let Some(reason) = pending.close_reason else {
                continue;
            };
            warn!(
                event = "clipboard.popup.close_failed",
                %pending.session_id,
                reason = reason.kind(),
                details = ?reason,
                terminal,
                elapsed_ms = pending.started_at.elapsed().as_millis() as u64,
                error = terminal,
                "clipboard popup close ended before native visibility result"
            );
        }
    }
    /// host took the OS focus. SetWindowPos runs with SWP_NOACTIVATE, so the
    /// re-assertion never steals focus back from the user's window.
    pub(crate) fn reassert_popup_topmost(&self, native_window_handle: i64) {
        if native_window_handle == 0 {
            return;
        }
        if let Err(error) = self
            .platform
            .send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                window_handle: native_window_handle,
                enabled: true,
            }))
        {
            error!(%error, window_handle = native_window_handle, "failed to re-assert clipboard popup topmost");
        }
    }

    pub(crate) fn begin_window_resize(&self, window_handle: i64) {
        if let Err(error) = self
            .platform
            .send_command(PlatformCommand::BeginWindowResize(
                BeginWindowResizeCommand { window_handle },
            ))
        {
            error!(%error, window_handle, "failed to request clipboard popup resize");
        }
    }

    pub(crate) fn begin_window_move(&self, window_handle: i64) {
        if let Err(error) =
            self.platform
                .send_command(PlatformCommand::BeginWindowMove(BeginWindowMoveCommand {
                    window_handle,
                }))
        {
            error!(%error, window_handle, "failed to request clipboard popup move");
        }
    }

    pub(crate) fn open_quick_target(&self, target: &nxtools_clipboard::ClipboardOpenTarget) {
        let action = match target.kind {
            ClipboardOpenTargetKind::File => ShellOpenAction::SelectInExplorer,
            ClipboardOpenTargetKind::RegistryKey => ShellOpenAction::RegistryKey,
            ClipboardOpenTargetKind::SystemTool => ShellOpenAction::SystemTool,
            ClipboardOpenTargetKind::Web | ClipboardOpenTargetKind::Directory => {
                ShellOpenAction::Open
            }
        };
        self.open_typed_shell_target(&target.value, action);
    }

    fn open_typed_shell_target(&self, target: &str, action: ShellOpenAction) {
        if let Err(error) =
            self.platform
                .send_command(PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                    target: target.to_owned(),
                    action,
                }))
        {
            error!(%error, target, "failed to request shell target open");
        }
    }

    pub(crate) fn schedule_preview(
        &mut self,
        entered: bool,
        item: ClipboardItem,
        panel_bounds: Bounds<Pixels>,
        pointer_position: Point<Pixels>,
        panel_scale_factor: f32,
        cx: &mut Context<Self>,
    ) {
        if !self.config.clipboard.enabled {
            return;
        }
        if crate::ui::components::native_context_menu::any_active(cx) {
            self.dismiss_preview(cx);
            return;
        }
        if entered {
            // Mouse movement inside the same row must not restart the original hover clock.
            if self.preview.hovered_item_id == Some(item.id) {
                return;
            }
            if preview_open_action(
                self.preview.window.is_some(),
                self.preview.visible,
                self.preview.displayed_item_id,
                item.id,
            ) == PreviewOpenAction::Ignore
            {
                return;
            }
            self.preview.hovered_item_id = Some(item.id);
            self.preview.pointer_inside = false;
            self.preview.generation = self.preview.generation.wrapping_add(1);
            let candidate = PreviewCandidate {
                item,
                panel_bounds,
                pointer_position,
                panel_scale_factor,
            };
            self.preview.candidate = Some(candidate.clone());
            if !self.preview.scroll_suppressed {
                self.schedule_preview_candidate(candidate, cx);
            }
        } else if self.preview.hovered_item_id == Some(item.id) {
            self.preview.hovered_item_id = None;
            self.preview.candidate = None;
            self.schedule_preview_hide(PREVIEW_LEAVE_DELAY, cx);
        }
    }

    fn schedule_preview_candidate(&mut self, candidate: PreviewCandidate, cx: &mut Context<Self>) {
        if !self.config.clipboard.enabled {
            return;
        }
        let generation = self.preview.generation;
        self.preview.timing = Some(PreviewTiming {
            correlation_id: uuid::Uuid::new_v4(),
            started_at: Instant::now(),
            native_started_at: None,
            step_started_at: None,
        });
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PREVIEW_HOVER_DELAY).await;
            let worker = this.update(cx, |this, _| {
                (this.preview.generation == generation && !this.preview.scroll_suppressed
                    && this.preview.hovered_item_id == Some(candidate.item.id))
                    .then(|| this.history_worker.clone())
            }).ok().flatten();
            let Some(worker) = worker else { return; };
            let Ok(item) = worker.active_item_async(candidate.item.id).await else { return; };
            let candidate = PreviewCandidate { item, ..candidate };
            let _ = this.update(cx, |this, cx| {
                if this.preview.generation == generation
                    && !this.preview.scroll_suppressed
                    && this.preview.hovered_item_id == Some(candidate.item.id)
                {
                    if let Some(timing) = this.preview.timing.as_ref() {
                        info!(event = "clipboard.preview.hover_ready", correlation_id = %timing.correlation_id,
                            generation, hover_intent_ms = PREVIEW_HOVER_DELAY.as_millis() as u64,
                            elapsed_ms = timing.started_at.elapsed().as_millis() as u64);
                    }
                    this.show_preview(candidate, cx);
                }
            });
        })
        .detach();
    }

    pub(crate) fn suspend_preview_for_scroll(&mut self, cx: &mut Context<Self>) {
        if self.preview.scroll_suppressed {
            return;
        }
        self.preview.scroll_suppressed = true;
        self.preview.scroll_generation = self.preview.scroll_generation.wrapping_add(1);
        self.preview.generation = self.preview.generation.wrapping_add(1);
        let scroll_generation = self.preview.scroll_generation;
        self.hide_preview_window();

        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(PREVIEW_SCROLL_RESUME_DELAY)
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.preview.scroll_generation != scroll_generation {
                    return;
                }
                this.preview.scroll_suppressed = false;
                if let Some(candidate) = this.preview.candidate.clone() {
                    this.preview.generation = this.preview.generation.wrapping_add(1);
                    this.schedule_preview_candidate(candidate, cx);
                }
            });
        })
        .detach();
    }

    pub(crate) fn set_preview_pointer_inside(&mut self, inside: bool, cx: &mut Context<Self>) {
        self.preview.pointer_inside = inside;
        self.preview.generation = self.preview.generation.wrapping_add(1);
        if !inside && self.preview.hovered_item_id.is_none() {
            self.schedule_preview_hide(PREVIEW_LEAVE_DELAY, cx);
        }
    }

    pub(crate) fn dismiss_preview_for_interaction(
        &mut self,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        let active = self.preview.candidate.is_some()
            || self.preview.hovered_item_id.is_some()
            || self.preview.pending_stage.is_some()
            || self.preview.visible;
        if active {
            info!(
                event = "clipboard.preview.interaction_cancelled",
                reason,
                generation = self.preview.generation
            );
        }
        self.dismiss_preview(cx);
    }

    #[cfg(test)]
    pub(crate) fn has_pending_preview_for_test(&self) -> bool {
        self.preview.candidate.is_some()
            || self.preview.hovered_item_id.is_some()
            || self.preview.pending_stage.is_some()
            || self.preview.visible
    }

    #[cfg(test)]
    pub(crate) fn preview_hover_for_test(&self) -> (u64, Option<i64>) {
        (self.preview.generation, self.preview.hovered_item_id)
    }

    pub(crate) fn dismiss_preview(&mut self, _cx: &mut Context<Self>) {
        self.preview.hovered_item_id = None;
        self.preview.candidate = None;
        self.preview.pointer_inside = false;
        self.preview.generation = self.preview.generation.wrapping_add(1);
        self.hide_preview_window();
    }

    fn hide_preview_window(&mut self) {
        if let Some(timing) = self.preview.timing.take()
            && timing.native_started_at.is_some()
        {
            info!(event = "clipboard.preview.lifecycle_ended", correlation_id = %timing.correlation_id,
                elapsed_ms = timing.started_at.elapsed().as_millis() as u64);
        }
        self.preview.pending_stage = None;
        if self.preview.visible
            && self.preview.native_window_handle != 0
            && let Err(error) = self
                .platform
                .send_command(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand {
                        window_handle: self.preview.native_window_handle,
                        visible: false,
                    },
                ))
        {
            error!(%error, "failed to hide clipboard preview");
        }
        self.preview.visible = false;
    }

    fn schedule_preview_hide(&mut self, delay: Duration, cx: &mut Context<Self>) {
        self.preview.generation = self.preview.generation.wrapping_add(1);
        let generation = self.preview.generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |this, cx| {
                if this.preview.generation == generation
                    && this.preview.hovered_item_id.is_none()
                    && !this.preview.pointer_inside
                {
                    this.dismiss_preview(cx);
                } else if this.preview.visible
                    && this.preview.hovered_item_id.is_none()
                    && !this.preview.pointer_inside
                {
                    // A racing hover bumped the generation and cancelled this
                    // timer, but the replacement preview never became visible
                    // (the target switched away or staging was dropped). The
                    // window would otherwise stay on screen orphaned until an
                    // unrelated hover event arrived — the reported "preview
                    // never auto-closes" symptom. Re-arm once more so the
                    // visible-but-unhovered window always terminates.
                    this.schedule_preview_hide(PREVIEW_LEAVE_DELAY, cx);
                }
            });
        })
        .detach();
    }

    fn stage_reused_preview(&mut self, mut stage: PendingPreviewStage) {
        let native_window_handle = self.preview.native_window_handle;
        match self
            .platform
            .send_command(PlatformCommand::SetWindowPosition(
                SetWindowPositionCommand {
                    window_handle: native_window_handle,
                    x: PREVIEW_STAGING_COORDINATE,
                    y: PREVIEW_STAGING_COORDINATE,
                },
            )) {
            Ok(command_id) => {
                stage.command_id = command_id;
                self.preview.pending_stage = Some(stage);
                self.preview.displayed_item_id = None;
            }
            Err(error) => {
                error!(%error, "failed to stage clipboard preview outside the display");
                self.hide_preview_window();
            }
        }
    }

    fn handle_preview_stage_result(
        &mut self,
        correlation_id: uuid::Uuid,
        success: bool,
        error_message: Option<&str>,
        cx: &mut Context<Self>,
    ) -> bool {
        if self
            .preview
            .pending_stage
            .as_ref()
            .is_none_or(|stage| stage.command_id != correlation_id)
        {
            return false;
        }
        let mut stage = self
            .preview
            .pending_stage
            .take()
            .expect("matching preview stage must exist");
        if stage.native_window_handle != self.preview.native_window_handle {
            return true;
        }
        if !success {
            error!(
                %correlation_id,
                has_native_error = error_message.is_some(),
                phase = ?stage.phase,
                "failed to prepare clipboard preview"
            );
            self.hide_preview_window();
            return true;
        }
        if !preview_callback_is_current(
            self.preview.generation,
            stage.generation,
            self.preview.hovered_item_id,
            stage.item.id,
            self.preview.scroll_suppressed,
            self.preview.native_window_handle,
            stage.native_window_handle,
        ) {
            if matches!(
                stage.phase,
                PreviewStagePhase::InitialNoActivate
                    | PreviewStagePhase::InitialBounds
                    | PreviewStagePhase::InitialTopmost
                    | PreviewStagePhase::InitialShow
            ) {
                self.hide_preview_window();
            }
            return true;
        }

        match stage.phase {
            PreviewStagePhase::ReuseNoActivate => {
                stage.phase = PreviewStagePhase::MoveOffscreen;
                self.stage_reused_preview(stage);
            }
            PreviewStagePhase::MeasureInitial | PreviewStagePhase::MeasureReuse => return true,
            PreviewStagePhase::InitialNoActivate
            | PreviewStagePhase::InitialBounds
            | PreviewStagePhase::InitialTopmost
            | PreviewStagePhase::InitialShow => {
                self.advance_initial_preview_presentation(stage, cx);
            }
            PreviewStagePhase::MoveOffscreen => {
                if stage.content_measured {
                    stage.phase = PreviewStagePhase::ReuseBounds;
                    self.queue_reused_preview_bounds(stage, cx);
                } else {
                    self.begin_preview_content_measurement(stage, false, cx);
                }
            }
            PreviewStagePhase::ReuseBounds if !self.preview.visible => {
                match self
                    .platform
                    .send_command(PlatformCommand::SetWindowVisibility(
                        SetWindowVisibilityCommand {
                            window_handle: self.preview.native_window_handle,
                            visible: true,
                        },
                    )) {
                    Ok(command_id) => {
                        stage.command_id = command_id;
                        stage.phase = PreviewStagePhase::ShowOffscreen;
                        self.preview.pending_stage = Some(stage);
                        // Treat a successfully queued show as visible immediately so a pointer
                        // leave can enqueue a matching hide behind it before confirmation arrives.
                        self.preview.visible = true;
                    }
                    Err(error) => {
                        error!(%error, "failed to show staged clipboard preview");
                        self.hide_preview_window();
                    }
                }
            }
            PreviewStagePhase::ReuseBounds | PreviewStagePhase::ShowOffscreen => {
                self.preview.visible = true;
                self.render_staged_preview(stage, cx);
            }
        }
        true
    }

    fn render_staged_preview(&mut self, stage: PendingPreviewStage, cx: &mut Context<Self>) {
        let Some(preview) = self.preview.window.as_ref() else {
            self.preview.visible = false;
            self.preview.displayed_item_id = None;
            return;
        };
        let view = preview.view.clone();
        let requested_item_id = stage.item.id;
        let generation = stage.generation;
        let native_window_handle = self.preview.native_window_handle;
        let application = cx.entity().downgrade();
        let platform = Rc::clone(&self.platform);
        let (x, y) = preview_device_origin(stage.bounds, stage.panel_scale_factor);
        let update_result = preview.window.update(cx, move |_, window, app| {
            // C# already acknowledged the final offscreen bounds; no competing
            // asynchronous GPUI native resize before the replacement frame.
            if view
                .update(app, |preview, cx| {
                    if stage.item.kind != ClipboardKind::Text { preview.set_item(stage.item, stage.layout, window, cx); }
                    else { preview.set_measured_layout(stage.layout, cx); }
                })
                .is_err()
            {
                return;
            }
            window.on_next_frame(move |_, app| {
                let _ = application.update(app, |application, _cx| {
                    if !preview_callback_is_current(
                        application.preview.generation,
                        generation,
                        application.preview.hovered_item_id,
                        requested_item_id,
                        application.preview.scroll_suppressed,
                        application.preview.native_window_handle,
                        native_window_handle,
                    ) {
                        return;
                    }
                    if let Err(error) = platform.send_command(PlatformCommand::SetWindowPosition(
                        SetWindowPositionCommand {
                            window_handle: native_window_handle,
                            x,
                            y,
                        },
                    )) {
                        error!(%error, "failed to reveal refreshed clipboard preview");
                        application.hide_preview_window();
                        return;
                    }
                    if let Err(error) = platform.send_command(PlatformCommand::SetWindowTopmost(
                        SetWindowTopmostCommand {
                            window_handle: native_window_handle,
                            enabled: true,
                        },
                    )) {
                        error!(%error, "failed to keep refreshed clipboard preview topmost");
                    }
                    application.preview.visible = true;
                    application.preview.displayed_item_id = Some(requested_item_id);
                    if let Some(timing) = application.preview.timing.take() {
                        info!(event = "clipboard.preview.reuse_reveal_queued", correlation_id = %timing.correlation_id,
                            generation, elapsed_ms = timing.started_at.elapsed().as_millis() as u64,
                            native_elapsed_ms = ?timing.native_started_at.map(|started| started.elapsed().as_millis() as u64));
                    }
                });
            });
            window.refresh();
        });
        if update_result.is_err() {
            self.preview.window = None;
            self.preview.native_window_handle = 0;
            self.preview.visible = false;
            self.preview.displayed_item_id = None;
        }
    }

    fn show_preview(&mut self, candidate: PreviewCandidate, cx: &mut Context<Self>) {
        if crate::ui::components::native_context_menu::any_active(cx) {
            self.dismiss_preview(cx);
            return;
        }
        if let Some(timing) = self.preview.timing.as_mut() {
            timing.native_started_at = Some(Instant::now());
        }
        let PreviewCandidate {
            item,
            panel_bounds,
            pointer_position,
            panel_scale_factor,
        } = candidate;
        let placement = crate::ui::monitor_placement::owner_monitor_placement(
            self.popup_monitor,
            self.popup.lifecycle.current().map(|popup| popup.window),
            cx,
        );
        let screen = placement.work_area;
        let adjacent_width = ((screen.right() - panel_bounds.right()).as_f32() - 10.0)
            .max((panel_bounds.left() - screen.left()).as_f32() - 10.0)
            .clamp(64.0, screen.size.width.as_f32().max(64.0));
        let layout = crate::ui::preview_layout_with_image_mode(
            &item,
            if item.kind == ClipboardKind::Image {
                adjacent_width
            } else {
                screen.size.width.as_f32()
            },
            screen.size.height.as_f32(),
            panel_scale_factor,
            self.config.clipboard.image_view_mode,
        );
        let preview_size = size(px(layout.window_width), px(layout.window_height));
        let bounds = preview_bounds(panel_bounds, preview_size, screen, pointer_position);

        if self.preview.window.is_some() && self.preview.native_window_handle != 0 {
            let stage = PendingPreviewStage {
                command_id: uuid::Uuid::nil(),
                native_window_handle: self.preview.native_window_handle,
                phase: PreviewStagePhase::MoveOffscreen,
                generation: self.preview.generation,
                item,
                layout,
                preview_size,
                bounds,
                panel_scale_factor,
                content_measured: false,
            };
            self.begin_reused_preview_presentation(stage, cx);
            return;
        }
        self.preview.window = None;
        self.preview.native_window_handle = 0;
        self.preview.visible = false;
        self.preview.displayed_item_id = None;

        let application = cx.entity().downgrade();
        let native_window_slot = Rc::new(RefCell::new(None));
        let opened_native_window_slot = Rc::clone(&native_window_slot);
        let preview_view_slot = Rc::new(RefCell::new(None));
        let opened_preview_view_slot = Rc::clone(&preview_view_slot);
        let initial_stage = PendingPreviewStage {
            command_id: uuid::Uuid::nil(),
            native_window_handle: 0,
            phase: PreviewStagePhase::InitialNoActivate,
            generation: self.preview.generation,
            item: item.clone(),
            layout,
            preview_size,
            bounds,
            panel_scale_factor,
            content_measured: false,
        };
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: false,
                show: false,
                kind: WindowKind::PopUp,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                display_id: placement.display_id,
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.ClipboardPreview".into()),
                window_min_size: None,
                window_decorations: Some(WindowDecorations::Client),
                app_owns_titlebar_drag: false,
                inactive_frame_interval: None,
                icon: None,
                tabbing_identifier: None,
            },
            move |window, cx| {
                let preview_window_handle = native_window_handle(window);
                *opened_native_window_slot.borrow_mut() = preview_window_handle;
                let preview =
                    cx.new(|cx| ClipboardPreview::new(item, layout, application, window, cx));
                *opened_preview_view_slot.borrow_mut() = Some(preview.downgrade());
                cx.new(|cx| Root::new(preview, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                self.preview.native_window_handle =
                    native_window_slot.borrow_mut().take().unwrap_or(0);
                if let Some(view) = preview_view_slot.borrow_mut().take() {
                    self.preview.window = Some(PreviewWindow {
                        window: window.into(),
                        view,
                    });
                    self.begin_preview_content_measurement(initial_stage, true, cx);
                } else {
                    let window: AnyWindowHandle = window.into();
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                    self.preview.native_window_handle = 0;
                    error!("clipboard preview opened without a view handle");
                }
            }
            Err(error) => {
                self.preview.native_window_handle = 0;
                self.preview.visible = false;
                self.preview.displayed_item_id = None;
                error!(%error, "failed to open clipboard preview");
            }
        }
    }
}

fn preview_callback_is_current(
    current_generation: u64,
    expected_generation: u64,
    current_hovered_item: Option<i64>,
    expected_item: i64,
    scroll_suppressed: bool,
    current_native_window: i64,
    expected_native_window: i64,
) -> bool {
    current_generation == expected_generation
        && current_hovered_item == Some(expected_item)
        && !scroll_suppressed
        && current_native_window == expected_native_window
}

fn command_for_action(
    entry: &ClipboardItem,
    target: &ForegroundContext,
    action: ClipboardAction,
    native_formats: Vec<StoredClipboardNativeFormat>,
) -> anyhow::Result<PlatformCommand> {
    if action == ClipboardAction::WithoutLineBreaks && entry.kind != ClipboardKind::Text {
        anyhow::bail!("去换行仅适用于文本条目");
    }
    let paste_after_write = action != ClipboardAction::CopyOnly;
    let image_profile = (entry.kind == ClipboardKind::Image)
        .then(|| clipboard_image_paste_profile(&target.process_name));
    let native_formats = if entry.kind == ClipboardKind::Image {
        native_formats
            .into_iter()
            .filter(native_format_is_available)
            .collect()
    } else {
        native_formats
    };
    if native_formats_restore_primary_kind(entry.kind, &native_formats) {
        return Ok(PlatformCommand::PasteNativeFormats(
            PasteNativeFormatsCommand {
                formats: native_formats
                    .into_iter()
                    .filter(|format| {
                        image_profile != Some(ClipboardImagePasteProfile::NativeImage)
                            || format.format_id != 15
                    })
                    .map(|format| ProtocolClipboardNativeFormat {
                        format_id: format.format_id,
                        format_name: format.format_name,
                        data_path: format.data_path,
                        size_bytes: format.size_bytes,
                        sort_order: format.sort_order,
                    })
                    .collect(),
                image_path_fallback: (image_profile
                    == Some(ClipboardImagePasteProfile::NativeImageWithFileDrop))
                .then(|| entry.image_path.clone()),
                paste_after_write,
                target_window_handle: target.window_handle,
                target_focus_window_handle: target.focus_window_handle,
            },
        ));
    }
    if let Some(text) = action_text_payload(entry, action) {
        return Ok(PlatformCommand::PasteText(PasteTextCommand {
            text,
            paste_after_write,
            target_window_handle: target.window_handle,
            target_focus_window_handle: target.focus_window_handle,
        }));
    }
    Ok(match entry.kind {
        ClipboardKind::Text => PlatformCommand::PasteText(PasteTextCommand {
            text: entry.text.clone(),
            paste_after_write,
            target_window_handle: target.window_handle,
            target_focus_window_handle: target.focus_window_handle,
        }),
        ClipboardKind::Files => PlatformCommand::PasteFiles(PasteFilesCommand {
            paths: entry.file_paths.clone(),
            paste_after_write,
            target_window_handle: target.window_handle,
            target_focus_window_handle: target.focus_window_handle,
        }),
        ClipboardKind::Image => PlatformCommand::PasteImage(PasteImageCommand {
            image_path: entry.image_path.clone(),
            include_file_drop_fallback: image_profile
                == Some(ClipboardImagePasteProfile::NativeImageWithFileDrop),
            paste_after_write,
            target_window_handle: target.window_handle,
            target_focus_window_handle: target.focus_window_handle,
        }),
    })
}

fn native_format_is_available(format: &StoredClipboardNativeFormat) -> bool {
    std::fs::metadata(&format.data_path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.len() == format.size_bytes)
}

fn native_formats_restore_primary_kind(
    kind: ClipboardKind,
    formats: &[StoredClipboardNativeFormat],
) -> bool {
    match kind {
        ClipboardKind::Text => formats
            .iter()
            .any(|format| format.format_id == CF_UNICODETEXT),
        ClipboardKind::Files => formats.iter().any(|format| format.format_id == CF_HDROP),
        ClipboardKind::Image => formats
            .iter()
            .any(|format| matches!(format.format_id, CF_DIB | CF_DIBV5)),
    }
}

impl ClipboardApplication {
    pub(crate) fn context_menu_failed(
        &mut self,
        source: AnyWindowHandle,
        source_session: Option<uuid::Uuid>,
        message: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some(popup) = self
            .popup
            .lifecycle
            .current()
            .filter(|popup| popup.window == source && Some(popup.session_id) == source_session)
            .cloned()
        else {
            return;
        };
        cx.defer(move |cx| {
            let _ = popup.panel.update(cx, |panel, cx| {
                panel.extension_start_failed(popup.session_id, message, cx)
            });
        });
    }
    pub(crate) fn context_menu_outside(
        &mut self,
        source: AnyWindowHandle,
        source_session: Option<uuid::Uuid>,
        cx: &mut Context<Self>,
    ) {
        let Some(popup) = self
            .popup
            .lifecycle
            .current()
            .filter(|popup| popup.window == source && Some(popup.session_id) == source_session)
            .cloned()
        else {
            return;
        };
        cx.defer(move |cx| {
            let _ = popup.window.update(cx, |_, window, cx| {
                let _ = popup
                    .panel
                    .update(cx, |panel, cx| panel.close_from_native_menu(window, cx));
            });
        });
    }
    pub(crate) fn handle_platform_notification(
        &mut self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        if crate::ui::components::native_context_menu::any_active(cx) {
            self.dismiss_preview(cx);
        }
        match notification {
            PlatformNotification::Connected => {
                self.reapply_capture_pause(cx);
                self.abandon_extension_copies(cx);
                // A reconnect invalidates every command that was pending on the previous pipe.
                self.pending_configuration.clear();
                self.latest_configuration = None;
                self.abandon_blacklist_configuration();
                self.deferred_blacklist_correlation = None;
                self.abandon_pending_link_panel_actions("connection_reset");
                self.retire_link_placement("connection_reset", cx);
                self.popup.load_generation = self.popup.load_generation.wrapping_add(1).max(1);
                self.popup.pending_load_generation = None;
                self.abandon_pending_popup_visibility("connection_reset");
                if let Some(previous) = self.popup.lifecycle.reset_pending() {
                    let _ = previous
                        .window
                        .update(cx, |_, window, _| window.remove_window());
                }
                self.preview.pending_stage = None;
                self.preview.generation = self.preview.generation.wrapping_add(1);
                if let Some(current) = self.popup.lifecycle.current().cloned() {
                    if let Ok(command_id) =
                        self.platform
                            .send_command(PlatformCommand::SetWindowTopmost(
                                SetWindowTopmostCommand {
                                    window_handle: current.native_window_handle,
                                    enabled: true,
                                },
                            ))
                    {
                        self.popup.lifecycle.replace_pending_topmost(command_id);
                    }
                    let panel = current.panel.clone();
                    cx.defer(move |cx| {
                        let _ = panel.update(cx, |panel, cx| {
                            panel.handle_platform_reconnected(cx);
                        });
                    });
                }
                self.pending_hotkey_configuration = None;
                self.pending_hotkey_recording = None;
                if self.pending_archive_picker.take().is_some() {
                    self.notify_error(
                        ARCHIVE_NOTIFICATION_ID,
                        "历史压缩包",
                        "Windows 文件选择器连接已重置",
                        cx,
                    );
                }
                if self.pending_storage_picker.take().is_some() {
                    self.notify_error(
                        STORAGE_NOTIFICATION_ID,
                        "存储迁移",
                        "Windows 目录选择器连接已重置",
                        cx,
                    );
                }
                self.hotkey_recording_active = false;
                self.hotkey_recording_desired = false;
                self.clipboard_configuration_dirty = false;
                self.send_clipboard_configuration();
                if let Some((id, _)) = self.latest_configuration {
                    self.schedule_feature_configuration_timeout(id, cx);
                }
                cx.notify();
            }
            PlatformNotification::Disconnected {
                generation,
                pending_command_ids,
            } => {
                self.disconnect_capture_pause(cx);
                self.abandon_extension_copies(cx);
                warn!(
                    event = "clipboard.platform_disconnected",
                    generation,
                    pending_command_count = pending_command_ids.len(),
                    "clipboard invalidated commands from the disconnected PlatformHost session"
                );
                self.pending_configuration.clear();
                if let Some((id, started)) = self.latest_configuration.take() {
                    warn!(event = "clipboard.configuration.failed", correlation_id = %id,
                        stage = "disconnected", elapsed_ms = started.elapsed().as_millis());
                }
                self.abandon_blacklist_configuration();
                self.abandon_pending_link_panel_actions("disconnected");
                self.retire_link_placement("disconnected", cx);
                self.popup.load_generation = self.popup.load_generation.wrapping_add(1).max(1);
                self.popup.pending_load_generation = None;
                self.abandon_pending_popup_visibility("disconnected");
                if let Some(previous) = self.popup.lifecycle.reset_pending() {
                    let _ = previous
                        .window
                        .update(cx, |_, window, _| window.remove_window());
                }
                self.preview.pending_stage = None;
                self.preview.generation = self.preview.generation.wrapping_add(1);
                if let Some(current) = self.popup.lifecycle.current().cloned() {
                    let _ = current.window.update(cx, |_, _window, app| {
                        let _ = current.panel.update(app, |panel, cx| {
                            panel.handle_platform_disconnected(cx);
                        });
                    });
                }
                self.pending_hotkey_recording = None;
                self.hotkey_recording_active = false;
                self.hotkey_recording_desired = false;
                self.clipboard_configuration_dirty = false;
                self.clipboard_configuration_status = "等待 Windows 适配器".into();
                if self.pending_archive_picker.take().is_some() {
                    self.notify_error(
                        ARCHIVE_NOTIFICATION_ID,
                        "历史压缩包",
                        "历史压缩包选择已因 Windows 连接断开而取消",
                        cx,
                    );
                }
                if self.pending_storage_picker.take().is_some() {
                    self.notify_error(
                        STORAGE_NOTIFICATION_ID,
                        "存储迁移",
                        "存储目录选择已因 Windows 连接断开而取消",
                        cx,
                    );
                }
                self.hotkey_recording_active = false;
                cx.notify();
            }
            PlatformNotification::Event(event) => self.handle_platform_event(event, cx),
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => {
                if self.finish_link_placement(correlation_id, &result, cx) {
                    return;
                }
                if self.finish_popup_activation(
                    correlation_id,
                    result.success,
                    result.native_error_code,
                    "native_result",
                    cx,
                ) {
                    return;
                }
                if self.handle_capture_pause_result(correlation_id, &result, cx) {
                    return;
                }
                if self.complete_extension_copy(correlation_id, &result, cx) {
                    return;
                }
                if let Some(pending) = self.pending_link_panel_actions.remove(&correlation_id) {
                    let elapsed_ms = pending.started_at.elapsed().as_millis() as u64;
                    if result.success {
                        info!(
                            event = "tools.link_panel.shell_completed",
                            correlation_id = %pending.correlation_id,
                            command_id = %correlation_id,
                            target_kind = pending.target_kind.label(),
                            ?pending.action,
                            value_length = pending.value_length,
                            elapsed_ms,
                            accepted = true,
                            "link panel shell route completed"
                        );
                    } else {
                        warn!(
                            event = "tools.link_panel.shell_failed",
                            correlation_id = %pending.correlation_id,
                            command_id = %correlation_id,
                            target_kind = pending.target_kind.label(),
                            ?pending.action,
                            value_length = pending.value_length,
                            elapsed_ms,
                            accepted = false,
                            native_error_code = ?result.native_error_code,
                            "link panel shell route failed"
                        );
                    }
                    return;
                }

                if self.pending_storage_picker == Some(correlation_id) {
                    self.pending_storage_picker = None;
                    if !result.success {
                        let message = format!(
                            "目录选择失败：{}",
                            result.error.as_deref().unwrap_or("未知 Windows 错误")
                        );
                        self.notify_error(STORAGE_NOTIFICATION_ID, "存储迁移", message, cx);
                    } else if let Some(path) = result.data.filter(|path| !path.is_empty()) {
                        self.start_clipboard_storage_migration(PathBuf::from(path), cx);
                    }
                    cx.notify();
                    return;
                }
                if self
                    .pending_archive_picker
                    .is_some_and(|(command_id, _)| command_id == correlation_id)
                {
                    let (_, operation) = self
                        .pending_archive_picker
                        .take()
                        .expect("matching archive picker command must exist");
                    if !result.success {
                        let message = format!(
                            "文件选择失败：{}",
                            result.error.as_deref().unwrap_or("未知 Windows 错误")
                        );
                        self.notify_error(ARCHIVE_NOTIFICATION_ID, "历史压缩包", message, cx);
                    } else if let Some(path) = result.data.filter(|path| !path.is_empty()) {
                        self.start_clipboard_archive_operation(operation, PathBuf::from(path), cx);
                    }
                    cx.notify();
                    return;
                }
                if self
                    .pending_hotkey_recording
                    .is_some_and(|(command_id, _)| command_id == correlation_id)
                {
                    let (_, requested) = self
                        .pending_hotkey_recording
                        .take()
                        .expect("matching recording command must exist");
                    if result.success {
                        self.hotkey_recording_active = requested;
                        self.clipboard_configuration_status = if requested {
                            "请按新的组合键".into()
                        } else {
                            "已恢复".into()
                        };
                        self.drive_clipboard_hotkey_recording();
                    } else {
                        self.hotkey_recording_active = false;
                        self.hotkey_recording_desired = false;
                        self.clipboard_configuration_status = format!(
                            "热键录制失败：{}",
                            result.error.as_deref().unwrap_or("未知 Windows 错误")
                        );
                    }
                    cx.notify();
                    return;
                }
                if self.pending_configuration.remove(&correlation_id) {
                    let latest = self
                        .latest_configuration
                        .is_some_and(|(id, _)| id == correlation_id);
                    if latest {
                        let (_, started) = self
                            .latest_configuration
                            .take()
                            .expect("matching configuration");
                        info!(event = "clipboard.configuration.completed", %correlation_id,
                            enabled = self.config.clipboard.enabled, success = result.success,
                            native_error_code = result.native_error_code, elapsed_ms = started.elapsed().as_millis());
                    }
                    if let Some((operation_id, started_at)) =
                        self.pending_blacklist_configuration.remove(&correlation_id)
                    {
                        info!(event = "global.blacklist.clipboard_native_completed", correlation_id = %operation_id,
                            success = result.success, native_error_code = result.native_error_code,
                            elapsed_ms = started_at.elapsed().as_millis(), "Windows clipboard rules terminal result");
                    }
                    let hotkey_result = resolve_pending_hotkey(
                        &mut self.pending_hotkey_configuration,
                        correlation_id,
                        result.success,
                    );
                    if hotkey_result.is_some() {
                        info!(event = "clipboard.hotkey.completed", %correlation_id,
                            success = result.success, native_error_code = result.native_error_code,
                            "clipboard binding received matching Windows result");
                    }
                    if result.success {
                        if let Some(Ok(hotkey)) = hotkey_result {
                            self.config.clipboard.hotkey = hotkey;
                            if self.persist_clipboard_configuration() {
                                self.clipboard_configuration_status =
                                    if self.config.clipboard.enabled {
                                        "已启用"
                                    } else {
                                        "已停用"
                                    }
                                    .into();
                            }
                        } else if latest && self.pending_hotkey_configuration.is_none() {
                            self.clipboard_configuration_status =
                                if self.config.clipboard.enabled {
                                    "已启用"
                                } else {
                                    "已停用"
                                }
                                .into();
                        }
                    } else if latest && self.pending_hotkey_configuration.is_none() {
                        self.clipboard_configuration_status = format!(
                            "应用失败：{}",
                            result.error.as_deref().unwrap_or("未知 Windows 错误")
                        );
                    }
                    if self.pending_hotkey_configuration.is_none()
                        && self.clipboard_configuration_dirty
                    {
                        self.clipboard_configuration_dirty = false;
                        if let Some(operation_id) = self.deferred_blacklist_correlation.take() {
                            let settings = self.config.clipboard.clone();
                            if let Some(command_id) = self.send_clipboard_configuration_correlated(
                                &settings,
                                Some(operation_id),
                            ) {
                                self.schedule_blacklist_configuration_timeout(command_id, cx);
                            }
                        } else {
                            self.send_clipboard_configuration();
                        }
                    }
                    cx.notify();
                    return;
                }
                if self.handle_preview_stage_result(
                    correlation_id,
                    result.success,
                    result.error.as_deref(),
                    cx,
                ) {
                    return;
                }
                if let Some(pending) = self.popup.pending_visibility.remove(&correlation_id) {
                    let Some(popup) = self
                        .popup
                        .lifecycle
                        .current()
                        .filter(|popup| {
                            popup.window == pending.window
                                && popup.session_id == pending.session_id
                                && popup.native_window_handle == pending.native_window_handle
                        })
                        .cloned()
                    else {
                        warn!(
                            event = "clipboard.popup.visibility_result_stale",
                            window_handle = pending.native_window_handle,
                            %pending.session_id,
                            visible = pending.visible,
                            details = ?pending.close_reason,
                            elapsed_ms = pending.started_at.elapsed().as_millis() as u64,
                            result_success = result.success,
                            native_error_code = ?result.native_error_code,
                        );
                        return;
                    };
                    let elapsed_ms = pending.started_at.elapsed().as_millis() as u64;
                    if result.success {
                        if pending.visible {
                            self.popup.size_tracking_session = Some(popup.session_id);
                            let _ = popup.window.update(cx, |_, window, app| {
                                let _ = popup.panel.update(app, |panel, _cx| {
                                    panel.enable_size_tracking(window);
                                    // Native activation completes before the deferred search focus.
                                });
                            });
                            self.request_popup_activation(popup.clone(), cx);
                            info!(%correlation_id, %pending.session_id, elapsed_ms, "clipboard popup revealed after topmost confirmation");
                        } else {
                            if let Some(reason) = pending.close_reason.as_ref() {
                                info!(
                                    event = "clipboard.popup.close_completed",
                                    %correlation_id,
                                    %pending.session_id,
                                    reason = reason.kind(),
                                    details = ?reason,
                                    window_handle = popup.native_window_handle,
                                    elapsed_ms,
                                );
                            }
                        }
                    } else {
                        let reason = result
                            .error
                            .as_deref()
                            .unwrap_or("unknown Windows error")
                            .to_owned();
                        if let Some(close_reason) = pending.close_reason.as_ref() {
                            warn!(
                                event = "clipboard.popup.close_failed",
                                %correlation_id,
                                %pending.session_id,
                                reason = close_reason.kind(),
                                details = ?close_reason,
                                window_handle = popup.native_window_handle,
                                elapsed_ms,
                                error = %reason,
                                native_error_code = ?result.native_error_code,
                            );
                        }
                        error!(%correlation_id, %reason, "failed to change clipboard popup visibility");
                        if !pending.visible {
                            self.popup.size_tracking_session = Some(popup.session_id);
                        }
                        let _ = popup.window.update(cx, |_, window, app| {
                            let _ = popup.panel.update(app, |panel, cx| {
                                if pending.visible {
                                    panel.handle_topmost_failure(reason, window, cx);
                                } else {
                                    panel.handle_hide_failure(reason, window, cx);
                                }
                            });
                        });
                    }
                    return;
                }
                let topmost_reason = result
                    .error
                    .as_deref()
                    .unwrap_or("unknown Windows error")
                    .to_owned();
                if let Some(resolution) = self
                    .popup
                    .lifecycle
                    .resolve_topmost(correlation_id, result.success)
                {
                    let (previous, topmost_failed) = match resolution {
                        TopmostResolution::Confirmed { previous } => {
                            info!(%correlation_id, "clipboard window is topmost");
                            (previous, false)
                        }
                        TopmostResolution::Rejected { previous } => {
                            warn!(
                                %correlation_id,
                                reason = %topmost_reason,
                                "clipboard window topmost failed; revealing fallback window"
                            );
                            (previous, true)
                        }
                    };
                    if let Some(previous) = previous {
                        let _ = previous
                            .window
                            .update(cx, |_, window, _| window.remove_window());
                    }
                    if let Some(popup) = self.popup.lifecycle.current().cloned() {
                        match self
                            .platform
                            .send_command(PlatformCommand::SetWindowVisibility(
                                SetWindowVisibilityCommand {
                                    window_handle: popup.native_window_handle,
                                    visible: true,
                                },
                            )) {
                            Ok(command_id) => {
                                self.popup.pending_visibility.insert(
                                    command_id,
                                    PendingPopupVisibility {
                                        window: popup.window,
                                        visible: true,
                                        native_window_handle: popup.native_window_handle,
                                        session_id: popup.session_id,
                                        close_reason: None,
                                        started_at: Instant::now(),
                                    },
                                );
                                if topmost_failed {
                                    let _ = popup.window.update(cx, |_, window, app| {
                                        let _ = popup.panel.update(app, |panel, cx| {
                                            panel.handle_topmost_failure(
                                                topmost_reason.clone(),
                                                window,
                                                cx,
                                            );
                                        });
                                    });
                                }
                            }
                            Err(error) => {
                                error!(%error, "failed to reveal clipboard popup after topmost result");
                                let _ = popup.window.update(cx, |_, window, app| {
                                    let _ = popup.panel.update(app, |panel, cx| {
                                        panel.handle_topmost_failure(error.to_string(), window, cx);
                                    });
                                });
                            }
                        }
                    }
                    return;
                }
                let Some(popup) = self.popup.lifecycle.current().cloned() else {
                    return;
                };
                let _ = popup.window.update(cx, |_, window, app| {
                    let _ = popup.panel.update(app, |panel, cx| {
                        panel.handle_command_result(correlation_id, result, window, cx);
                    });
                });
            }
        }
    }

    fn handle_platform_event(&mut self, event: PlatformEvent, cx: &mut Context<Self>) {
        match event {
            PlatformEvent::ClipboardCaptured(capture) => {
                if !self.admits_feature_event(capture.captured_at_unix_ms) {
                    remove_clipboard_capture_staging(&capture);
                    return;
                }
                self.invalidate_currency_capture();
                if self.capture_is_paused(&capture) {
                    self.invalidate_link_detection();
                    remove_clipboard_capture_staging(&capture);
                    return;
                }
                if self.management_operation_active {
                    self.invalidate_link_detection();
                    self.deferred_captures.push(capture);
                    info!(
                        queued = self.deferred_captures.len(),
                        "clipboard capture deferred during storage operation"
                    );
                    return;
                }
                if !clipboard_capture_has_content(&capture) {
                    self.invalidate_link_detection();
                    remove_clipboard_capture_staging(&capture);
                    info!(
                        event = "clipboard.capture_skipped",
                        reason = "no_content",
                        "ignored clipboard capture without user content"
                    );
                    return;
                }
                if self.suppress_action_capture(&capture) {
                    self.invalidate_link_detection();
                    remove_clipboard_capture_staging(&capture);
                    info!(
                        event = "clipboard.capture_skipped",
                        reason = "action_write",
                        "ignored clipboard capture produced by a clipboard history action"
                    );
                    return;
                }
                let captured_at_unix_ms = capture.captured_at_unix_ms;
                if self.is_process_excluded(&capture.source.process_name) {
                    self.invalidate_link_detection();
                    if let Some(image) = capture.image {
                        let _ = std::fs::remove_file(image.staging_path);
                    }
                    for format in capture.native_formats {
                        let _ = std::fs::remove_file(format.data_path);
                    }
                    if let Some(path) = capture.source_icon_staging_path {
                        let _ = std::fs::remove_file(path);
                    }
                    info!(source = %capture.source.process_name, "clipboard capture excluded by Rust policy");
                    return;
                }
                let kind = if !capture.paths.is_empty() {
                    ClipboardKind::Files
                } else if capture.image.is_some() {
                    ClipboardKind::Image
                } else {
                    ClipboardKind::Text
                };
                if !self.capture_policy().allows(kind) {
                    self.invalidate_link_detection();
                    if let Some(image) = capture.image {
                        let _ = std::fs::remove_file(image.staging_path);
                    }
                    for format in capture.native_formats {
                        let _ = std::fs::remove_file(format.data_path);
                    }
                    if let Some(path) = capture.source_icon_staging_path {
                        let _ = std::fs::remove_file(path);
                    }
                    info!(?kind, "clipboard capture ignored by policy");
                    return;
                }
                let link_text = capture.text.clone();
                if let Err(error) = self.history_worker.enqueue_capture(capture) {
                    error!(%error, "failed to enqueue clipboard capture for worker");
                }
                if let Some(text) = link_text.filter(|text| !text.is_empty()) {
                    self.handle_link_detector_capture(&text, cx);
                    self.handle_currency_capture(&text, cx);
                } else {
                    self.invalidate_link_detection();
                }
                if captured_at_unix_ms.saturating_sub(self.last_cleanup_at_unix_ms)
                    >= RETENTION_INTERVAL_MS
                {
                    self.apply_retention(captured_at_unix_ms);
                }
            }
            PlatformEvent::ClipboardPanelRequested(request) => {
                if !self.admits_feature_event(request.requested_at_unix_ms) {
                    return;
                }
                if self.management_operation_active {
                    info!("ignored clipboard panel request during storage operation");
                    return;
                }
                if self.clipboard_hotkey_recording_active() {
                    info!("ignored clipboard panel request while recording a new hotkey");
                    return;
                }
                let request_age =
                    nxtools_ipc::unix_time_ms().saturating_sub(request.requested_at_unix_ms);
                if request_age <= MAX_PANEL_REQUEST_AGE_MS {
                    self.show_panel(request, cx);
                } else {
                    info!(request_age, "ignored stale clipboard panel request");
                }
            }
            PlatformEvent::ForegroundChanged(foreground) => {
                let Some(popup) = self.popup.lifecycle.current().cloned() else {
                    return;
                };
                let preview_window_handle = self.preview.native_window_handle;
                let _ = popup.window.update(cx, |_, window, app| {
                    let _ = popup.panel.update(app, |panel, cx| {
                        panel.handle_foreground_changed(
                            popup.native_window_handle,
                            preview_window_handle,
                            &foreground,
                            window,
                            cx,
                        );
                    });
                });
            }
            PlatformEvent::GlobalPointerPressed(pointer) => {
                let Some(popup) = self.popup.lifecycle.current().cloned() else {
                    return;
                };
                let preview_window_handle = self.preview.native_window_handle;
                let _ = popup.window.update(cx, |_, window, app| {
                    let _ = popup.panel.update(app, |panel, cx| {
                        panel.handle_pointer_pressed(
                            crate::ui::PopupPointerPressContext {
                                popup_window_handle: popup.native_window_handle,
                                preview_window_handle,
                                pressed_window_handle: pointer.window_handle,
                                pressed_root_window_handle: pointer.root_window_handle,
                                pressed_process_id: pointer.process_id,
                                popup_requested_at_unix_ms: popup.summon_requested_at_unix_ms,
                                popup_event_sequence: popup.summon_event_sequence,
                                pressed_at_unix_ms: pointer.sampled_at_unix_ms,
                                pressed_event_sequence: pointer.event_sequence,
                            },
                            window,
                            cx,
                        );
                    });
                });
            }
            PlatformEvent::ArchiveRead(_)
            | PlatformEvent::ArchivePassword(_)
            | PlatformEvent::ArchiveJob(_)
            | PlatformEvent::ArchiveVirtualDragProgress(_)
            | PlatformEvent::ArchivePicker(_)
            | PlatformEvent::FloatingAssistant(_)
            | PlatformEvent::ProcessWindowPickChanged(_)
            | PlatformEvent::TrayActionRequested(_)
            | PlatformEvent::SystemMotionPreferenceChanged(_)
            | PlatformEvent::SystemThemePreferenceChanged(_)
            | PlatformEvent::QuickLaunchPanelRequested(_)
            | PlatformEvent::QuickLaunchKeyboardHealthChanged(_)
            | PlatformEvent::QuickLaunchDirectoryContextResolved(_)
            | PlatformEvent::QuickLaunchAutoSwitchRequested(_)
            | PlatformEvent::QuickLaunchFileDialogSearchHostChanged(_)
            | PlatformEvent::TaskbarOverlayEditGestureFinished(_)
            | PlatformEvent::ElevatedDirectoryHelperRequested(_) => {}
        }
    }

    fn show_panel(&mut self, request: ClipboardPanelRequest, cx: &mut Context<Self>) {
        if !self.config.clipboard.enabled {
            return;
        }
        if self.popup.lifecycle.replacement_pending() {
            info!(
                "ignored clipboard panel request while a replacement is awaiting topmost confirmation"
            );
            return;
        }
        self.popup.load_generation = self.popup.load_generation.wrapping_add(1).max(1);
        let generation = self.popup.load_generation;
        self.popup.pending_load_generation = Some(generation);
        let worker = self.history_worker.clone();
        let application = cx.entity().downgrade();
        cx.spawn(async move |_, cx| {
            let result = load_clipboard_popup_snapshot(
                &worker,
                "",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
            )
            .await;
            let _ = application.update(cx, move |application, cx| {
                application.finish_show_panel(generation, request, result, cx);
            });
        })
        .detach();
    }

    fn finish_show_panel(
        &mut self,
        generation: u64,
        request: ClipboardPanelRequest,
        result: Result<ClipboardPopupSnapshot, ClipboardHistoryError>,
        cx: &mut Context<Self>,
    ) {
        if !self.config.clipboard.enabled || self.popup.pending_load_generation != Some(generation)
        {
            return;
        }
        self.popup.pending_load_generation = None;
        let (snapshot, initial_error) = match result {
            Ok(snapshot) => (snapshot, None),
            Err(error) => {
                error!(%error, %generation, "failed to load clipboard popup snapshot");
                (
                    ClipboardPopupSnapshot {
                        page: ClipboardHistoryPage {
                            items: Vec::new(),
                            total: 0,
                            has_more: false,
                        },
                        summary_counts: HashMap::new(),
                        categories: Vec::new(),
                        category_sources: HashMap::new(),
                        source_process_names: Vec::new(),
                    },
                    Some(format!("剪贴板历史加载失败：{error}")),
                )
            }
        };
        self.open_panel(request, snapshot, initial_error, cx);
    }

    fn open_panel(
        &mut self,
        request: ClipboardPanelRequest,
        snapshot: ClipboardPopupSnapshot,
        initial_error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let panel_size = self.config.clipboard_panel;
        let modifier_actions = self.config.clipboard.modifier_actions;
        let text_card_max_height = self.config.clipboard.text_card_max_height;
        let (bounds, display_id) = panel_bounds(&request, panel_size, cx);
        let session_id = uuid::Uuid::new_v4();
        let application = cx.entity().downgrade();
        let worker = self.history_worker.clone();
        let capture_pause = self.capture_pause_snapshot();
        let target = request.target.clone();
        let panel_slot = Rc::new(RefCell::new(None));
        let native_window_slot = Rc::new(RefCell::new(None));
        let opened_panel_slot = Rc::clone(&panel_slot);
        let opened_native_window_slot = Rc::clone(&native_window_slot);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: true,
                show: false,
                kind: WindowKind::PopUp,
                is_movable: true,
                is_resizable: true,
                is_minimizable: false,
                display_id,
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.Clipboard".into()),
                window_min_size: Some(size(px(280.0), px(280.0))),
                window_decorations: Some(WindowDecorations::Client),
                app_owns_titlebar_drag: false,
                inactive_frame_interval: None,
                icon: None,
                tabbing_identifier: None,
            },
            move |window, cx| {
                let native_window = native_window_handle(window);
                *opened_native_window_slot.borrow_mut() = native_window;
                let panel = cx.new(|cx| {
                    ClipboardPanel::new(
                        snapshot.page,
                        snapshot.categories,
                        snapshot.category_sources,
                        snapshot.source_process_names,
                        modifier_actions,
                        target,
                        worker,
                        application,
                        session_id,
                        initial_error,
                        window,
                        cx,
                    )
                    .with_summary_counts(snapshot.summary_counts)
                    .with_text_card_max_height(text_card_max_height)
                    .with_capture_pause(capture_pause)
                });
                *opened_panel_slot.borrow_mut() = Some(panel.downgrade());
                cx.new(|cx| Root::new(panel, window, cx))
            },
        );

        match opened {
            Ok(window) => {
                let Some(panel) = panel_slot.borrow_mut().take() else {
                    error!("clipboard popup opened without a panel entity");
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                    return;
                };
                let native_window_handle = native_window_slot.borrow_mut().take().unwrap_or(0);
                let panel_for_focus = panel.clone();
                let popup = PopupWindow {
                    window: window.into(),
                    panel: panel.clone(),
                    native_window_handle,
                    session_id,
                    summon_requested_at_unix_ms: request.requested_at_unix_ms,
                    summon_event_sequence: request.event_sequence,
                };
                let focus_window = popup.window;
                let topmost_result = if should_request_popup_topmost(true, native_window_handle) {
                    let scale = request
                        .cursor_monitor
                        .map_or(request.cursor_monitor_dpi, |monitor| monitor.dpi)
                        .max(96) as f32
                        / 96.0;
                    let bounds_result = self
                        .platform
                        .send_command(PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
                            window_handle: native_window_handle,
                            x: (bounds.origin.x.as_f32() * scale).round() as i32,
                            y: (bounds.origin.y.as_f32() * scale).round() as i32,
                            width: (bounds.size.width.as_f32() * scale).round() as i32,
                            height: (bounds.size.height.as_f32() * scale).round() as i32,
                            corner_radius: None,
                        }))
                        .map_err(|error| format!("failed to size clipboard popup: {error}"));
                    match bounds_result {
                        Ok(bounds_command) => self
                            .platform
                            .send_command(PlatformCommand::SetWindowTopmost(
                                SetWindowTopmostCommand {
                                    window_handle: native_window_handle,
                                    enabled: true,
                                },
                            ))
                            .map_err(|error| error.to_string())
                            .inspect(|_| {
                                info!(
                                    %bounds_command,
                                    width = bounds.size.width.as_f32(),
                                    height = bounds.size.height.as_f32(),
                                    "clipboard popup native bounds command sent"
                                );
                            }),
                        Err(error) => Err(error),
                    }
                } else {
                    Err("GPUI did not expose a native Windows handle".to_owned())
                };
                let Ok(topmost_command) = topmost_result else {
                    let reason =
                        topmost_result.expect_err("topmost result was checked as an error");
                    error!(%reason, %session_id, "clipboard popup initialization command could not be created");
                    let previous = self
                        .popup
                        .lifecycle
                        .replace_without_topmost_confirmation(popup);
                    if let Some(failed) = self.popup.lifecycle.take_current() {
                        let _ = failed
                            .window
                            .update(cx, |_, window, _| window.remove_window());
                    }
                    if let Some(previous) = previous {
                        self.popup
                            .lifecycle
                            .replace_without_topmost_confirmation(previous);
                    }
                    return;
                };
                self.popup.lifecycle.stage(popup, topmost_command);
                self.popup_monitor = request.cursor_monitor;
                self.retire_popup_activation("popup_replaced");
                self.popup.size_tracking_session = None;
                info!(
                    %session_id,
                    target = %request.target.process_name,
                    requested_at_unix_ms = request.requested_at_unix_ms,
                    event_sequence = request.event_sequence,
                    x = request.cursor_x,
                    y = request.cursor_y,
                    width = panel_size.width,
                    height = panel_size.height,
                    "clipboard.popup.opened"
                );
                cx.defer(move |cx| {
                    let _ = focus_window.update(cx, |_, window, app| {
                        let _ = panel_for_focus.update(app, |panel, cx| {
                            panel.focus_search_input(window, cx);
                        });
                    });
                });
            }
            Err(error) => error!(%error, "failed to open clipboard popup"),
        }
    }
}

impl ClipboardApplication {
    pub(crate) fn set_text_card_max_height(&mut self, height: u32, cx: &mut Context<Self>) {
        if !(nxtools_core::CLIPBOARD_TEXT_CARD_MIN_HEIGHT
            ..=nxtools_core::CLIPBOARD_TEXT_CARD_MAX_HEIGHT)
            .contains(&height)
            || self.config.clipboard.text_card_max_height == height
        {
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        info!(event="clipboard.text_card.height_save_started", %correlation_id, height);
        match AppConfig::update_section(&self.config_path, |config| {
            config.clipboard.text_card_max_height = height;
        }) {
            Ok(merged) => {
                let panel_size = self.config.clipboard_panel;
                self.config = merged;
                self.config.clipboard_panel = panel_size;
                let popup = self.popup.lifecycle.current().cloned();
                cx.defer(move |cx| {
                    if let Some(popup) = popup {
                        let _ = popup.panel.update(cx, |panel, cx| {
                            panel.set_text_card_max_height(height, cx);
                        });
                    }
                });
                info!(event="clipboard.text_card.height_saved", %correlation_id, height, elapsed_ms=started.elapsed().as_millis());
            }
            Err(_) => {
                warn!(event="clipboard.text_card.height_save_failed", %correlation_id, reason="config_commit", elapsed_ms=started.elapsed().as_millis());
                self.notify_error(
                    "clipboard-text-card-height",
                    "文本卡最大高度",
                    "保存失败，仍使用原来的高度。请检查配置文件是否可写。",
                    cx,
                );
            }
        }
        cx.notify();
    }

    pub(crate) fn set_image_view_mode(
        &mut self,
        mode: nxtools_core::ClipboardImageViewMode,
        cx: &mut Context<Self>,
    ) {
        let correlation_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        info!(event="clipboard.preview.mode_save_started", %correlation_id, ?mode);
        match AppConfig::update_section(&self.config_path, |config| {
            config.clipboard.image_view_mode = mode
        }) {
            Ok(merged) => {
                let panel = self.config.clipboard_panel;
                self.config = merged;
                self.config.clipboard_panel = panel;
                self.dismiss_preview(cx);
                info!(event="clipboard.preview.mode_saved", %correlation_id, ?mode, elapsed_ms=started.elapsed().as_millis());
            }
            Err(_) => {
                warn!(event="clipboard.preview.mode_save_failed", %correlation_id, reason="config_commit", elapsed_ms=started.elapsed().as_millis());
                self.notify_error(
                    "clipboard-image-view-mode",
                    "图片预览设置",
                    "保存失败，仍使用原来的视图模式。请检查配置文件是否可写。",
                    cx,
                );
            }
        }
        cx.notify();
    }
}

#[cfg(test)]
fn content_fingerprint(mut reader: impl Read) -> std::io::Result<String> {
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod asset_tests {
    use super::*;

    #[test]
    fn managed_asset_removal_never_deletes_files_outside_the_clipboard_store() {
        let directory = tempfile::tempdir().expect("temp directory");
        let managed = directory.path().join("clipboard-assets");
        std::fs::create_dir_all(&managed).expect("create managed directory");
        let managed_file = managed.join("capture.png");
        let external_file = directory.path().join("user-document.png");
        std::fs::write(&managed_file, b"managed").expect("write managed fixture");
        std::fs::write(&external_file, b"external").expect("write external fixture");

        assert!(!remove_managed_clipboard_asset(
            &managed,
            &external_file.to_string_lossy()
        ));
        assert!(external_file.exists());
        assert!(remove_managed_clipboard_asset(
            &managed,
            &managed_file.to_string_lossy()
        ));
        assert!(!managed_file.exists());
    }

    #[test]
    fn hotkey_is_committed_only_after_the_matching_platform_confirmation() {
        let older_command = uuid::Uuid::new_v4();
        let current_command = uuid::Uuid::new_v4();
        let candidate = ClipboardHotKey::default();
        let mut pending = Some((current_command, candidate.clone()));

        assert!(resolve_pending_hotkey(&mut pending, older_command, true).is_none());
        assert!(pending.is_some());
        assert_eq!(
            resolve_pending_hotkey(&mut pending, current_command, false),
            Some(Err(()))
        );
        assert!(pending.is_none());

        pending = Some((current_command, candidate.clone()));
        assert_eq!(
            resolve_pending_hotkey(&mut pending, current_command, true),
            Some(Ok(candidate))
        );
        assert!(pending.is_none());
    }

    #[test]
    fn first_clipboard_preview_is_hidden_and_uses_confirmed_passive_presentation() {
        let source = include_str!("clipboard.rs");
        let open = source
            .split("fn show_preview(")
            .nth(1)
            .expect("production preview creation")
            .split("\n}\n")
            .next()
            .expect("preview body");
        assert!(
            open.contains("show: false"),
            "first preview must start hidden"
        );
        assert!(
            open.contains("begin_preview_content_measurement"),
            "first hidden preview must measure before entering confirmed native chain"
        );
        let presentation = include_str!("clipboard_preview_presentation.rs");
        assert!(presentation.contains("stage.phase = PreviewStagePhase::InitialNoActivate"));
        assert!(presentation.contains("self.begin_initial_preview_presentation(stage, cx)"));
        assert!(
            !open.contains("self.preview.visible = true"),
            "queue/create is not native shown confirmation"
        );
    }
    #[test]
    fn preview_window_is_reused_instead_of_recreated_for_hover_changes() {
        assert_eq!(
            preview_open_action(true, true, Some(7), 7),
            PreviewOpenAction::Ignore
        );
        assert_eq!(
            preview_open_action(true, true, Some(7), 8),
            PreviewOpenAction::Reuse
        );
        assert_eq!(
            preview_open_action(true, false, Some(7), 7),
            PreviewOpenAction::Reuse
        );
        assert_eq!(
            preview_open_action(false, false, None, 7),
            PreviewOpenAction::Create
        );
    }

    #[test]
    fn reused_preview_is_staged_offscreen_until_the_replacement_frame_is_committed() {
        let source = include_str!("clipboard.rs");
        let stage_start = source
            .find("fn stage_reused_preview")
            .expect("clipboard preview staging implementation");
        let stage_end = source[stage_start..]
            .find("fn handle_preview_stage_result")
            .expect("end of clipboard preview staging implementation");
        let stage_source = &source[stage_start..stage_start + stage_end];
        let stage = stage_source
            .find("PREVIEW_STAGING_COORDINATE")
            .expect("the old native preview surface must move outside every display");
        assert!(!stage_source[..stage].contains("visible: false"));

        let result_start = stage_start + stage_end;
        let result_end = source[result_start..]
            .find("fn render_staged_preview")
            .expect("end of clipboard preview stage-result implementation");
        let result_source = &source[result_start..result_start + result_end];
        let show_offscreen = result_source
            .find("visible: true")
            .expect("a previously hidden preview must be shown while it is off-screen");
        let render_after_show = result_source
            .find("self.render_staged_preview")
            .expect("rendering must start only after off-screen visibility is confirmed");
        assert!(show_offscreen < render_after_show);

        let render_start = result_start + result_end;
        let render_end = source[render_start..]
            .find("fn show_preview")
            .expect("end of staged preview rendering implementation");
        let render_source = &source[render_start..render_start + render_end];
        let update = render_source
            .find("preview.set_item")
            .expect("the reused preview entity must receive the requested item");
        let after_frame = render_source
            .find("window.on_next_frame")
            .expect("the native surface must wait for a freshly submitted GPUI frame");
        let reveal = render_source[after_frame..]
            .find("PlatformCommand::SetWindowPosition")
            .map(|offset| after_frame + offset)
            .expect("the refreshed preview must return on-screen after frame submission");

        assert!(update < after_frame);
        assert!(after_frame < reveal);
    }

    #[test]
    fn topmost_is_requested_only_after_the_native_popup_has_opened() {
        assert!(!should_request_popup_topmost(false, 88));
        assert!(!should_request_popup_topmost(true, 0));
        assert!(should_request_popup_topmost(true, 88));
    }

    #[test]
    fn preview_is_placed_beside_panel_without_covering_history() {
        let panel = Bounds::new(point(px(13.0), px(29.0)), size(px(300.0), px(440.0)));
        let screen = Bounds::new(point(px(0.0), px(0.0)), size(px(1920.0), px(1080.0)));
        let preview = preview_bounds(
            panel,
            size(px(360.0), px(420.0)),
            screen,
            point(px(150.0), px(169.0)),
        );

        assert_eq!(preview.origin.x, px(323.0));
        assert!(preview.origin.x >= panel.right());
        assert_eq!(preview.origin.y, panel.origin.y);
    }

    #[test]
    fn preview_device_position_uses_panel_monitor_scale() {
        let preview = Bounds::new(point(px(321.0), px(29.0)), size(px(360.0), px(420.0)));

        assert_eq!(preview_device_origin(preview, 1.5), (482, 44));
    }

    #[test]
    fn link_panel_positions_stay_inside_the_selected_display() {
        let screen = Bounds::new(point(px(0.0), px(0.0)), size(px(1920.0), px(1080.0)));
        let panel_size = size(px(420.0), px(200.0));

        let left = link_panel_bounds(LinkPanelPosition::TopLeft, panel_size, screen);
        let center = link_panel_bounds(LinkPanelPosition::Center, panel_size, screen);
        let right = link_panel_bounds(LinkPanelPosition::TopRight, panel_size, screen);

        assert_eq!(left.origin, point(px(24.0), px(24.0)));
        assert_eq!(center.origin, point(px(750.0), px(440.0)));
        assert_eq!(right.origin, point(px(1476.0), px(24.0)));

        let small_screen = Bounds::new(point(px(0.0), px(0.0)), size(px(800.0), px(600.0)));
        for position in LinkPanelPosition::ALL {
            let bounds = link_panel_bounds(position, panel_size, small_screen);
            assert!(bounds.left() >= small_screen.left());
            assert!(bounds.top() >= small_screen.top());
            assert!(bounds.right() <= small_screen.right());
            assert!(bounds.bottom() <= small_screen.bottom());
        }
    }

    #[test]
    fn content_fingerprint_depends_on_bytes_instead_of_staging_path() {
        let first = content_fingerprint(std::io::Cursor::new(b"same image bytes"))
            .expect("fingerprint first image");
        let second = content_fingerprint(std::io::Cursor::new(b"same image bytes"))
            .expect("fingerprint second image");
        let different = content_fingerprint(std::io::Cursor::new(b"different image bytes"))
            .expect("fingerprint different image");

        assert_eq!(first, second);
        assert_ne!(first, different);
    }

    fn fixture_target() -> ForegroundContext {
        ForegroundContext {
            window_handle: 42,
            focus_window_handle: 43,
            process_id: 7,
            process_name: "editor".into(),
            window_title: "draft".into(),
        }
    }

    #[gpui_kit::test]
    fn feature_disable_retires_clipboard_loads_and_preserves_timed_capture_pause(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use nxtools_protocol::Message;
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let temp = tempfile::tempdir().expect("feature enablement fixture");
        let config_path = temp.path().join("config.json");
        let config = AppConfig::default();
        config.save_to(&config_path).expect("seed enabled config");
        let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
            temp.path().join("history.db"),
            temp.path().join("assets"),
            ClipboardRetentionPolicy {
                max_items: 100,
                retention_days: 30,
            },
            1_000,
        )
        .expect("start fixture capture worker");
        let (platform, endpoint) = nxtools_ipc::platform_bridge(16);
        endpoint.set_connected_for_contract_test(true);
        let application = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            cx.new(|cx| {
                ClipboardApplication::new(
                    worker.clone(),
                    Rc::new(platform),
                    config,
                    config_path.clone(),
                    notifications,
                    cx,
                )
            })
        });
        let request = ClipboardPanelRequest {
            target: fixture_target(),
            cursor_x: 0,
            cursor_y: 0,
            cursor_display_index: 0,
            cursor_monitor_dpi: 96,
            cursor_monitor: None,
            requested_at_unix_ms: nxtools_ipc::unix_time_ms(),
            event_sequence: 0,
        };
        let staging = temp.path().join("staged-icon");
        std::fs::write(&staging, b"synthetic capture staging").expect("seed staged capture");
        application.update(cx, |app, cx| {
            app.handle_platform_event(
                PlatformEvent::ClipboardCaptured(ClipboardCapture {
                    text: Some("retained history".into()),
                    image: None,
                    paths: Vec::new(),
                    native_formats: Vec::new(),
                    source_icon_staging_path: None,
                    source: fixture_target(),
                    captured_at_unix_ms: request.requested_at_unix_ms,
                }),
                cx,
            );
            app.handle_platform_event(PlatformEvent::ClipboardPanelRequested(request.clone()), cx);
            assert!(app.popup.pending_load_generation.is_some());
            app.management_operation_active = true;
            app.handle_platform_event(
                PlatformEvent::ClipboardCaptured(ClipboardCapture {
                    text: Some("deferred capture".into()),
                    image: None,
                    paths: Vec::new(),
                    native_formats: Vec::new(),
                    source_icon_staging_path: Some(staging.to_string_lossy().into_owned()),
                    source: fixture_target(),
                    captured_at_unix_ms: request.requested_at_unix_ms,
                }),
                cx,
            );
            assert_eq!(app.deferred_captures.len(), 1);
            app.set_capture_pause(true, true, cx);
            let pause = app.capture_pause_snapshot();
            let generation = app.capture_pause.generation;

            app.set_feature_enabled(false, cx);
            assert!(app.popup.pending_load_generation.is_none());
            assert!(app.deferred_captures.is_empty());
            assert!(!staging.exists());
            app.finish_clipboard_management_operation(cx);
            app.handle_platform_event(PlatformEvent::ClipboardPanelRequested(request.clone()), cx);
            assert!(app.popup.pending_load_generation.is_none());
            let failed_attempt = app.latest_configuration.expect("disable configuration").0;
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: failed_attempt,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: false,
                        error: Some("fixture native rejection".into()),
                        native_error_code: Some(5),
                        data: None,
                    },
                },
                cx,
            );
            assert!(!app.clipboard_settings().enabled);
            assert!(
                app.feature_configuration_status()
                    .contains("fixture native rejection")
            );
            app.handle_platform_notification(
                PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: Vec::new(),
                },
                cx,
            );
            assert!(app.feature_configuration_status().starts_with("等待"));
            app.handle_platform_notification(PlatformNotification::Connected, cx);
            assert_eq!(app.feature_configuration_status(), "正在应用");
            let superseded = app.latest_configuration.expect("reconnect configuration").0;
            app.set_feature_enabled(true, cx);
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: superseded,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                },
                cx,
            );
            assert_eq!(app.feature_configuration_status(), "正在应用");
            app.handle_platform_event(PlatformEvent::ClipboardPanelRequested(request), cx);
            assert!(app.popup.pending_load_generation.is_none());
            assert_eq!(app.capture_pause_snapshot(), pause);
            assert_eq!(app.capture_pause.generation, generation);
        });
        cx.run_until_parked();
        assert!(cx.windows().is_empty());
        let commands = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
            .map(|envelope| envelope.message)
            .collect::<Vec<_>>();
        assert!(matches!(commands.as_slice(), [
            Message::PlatformCommand(PlatformCommand::SetClipboardCapturePaused(pause)),
            Message::PlatformCommand(PlatformCommand::ConfigureClipboard(off)),
            Message::PlatformCommand(PlatformCommand::SetClipboardCapturePaused(reconnect_pause)),
            Message::PlatformCommand(PlatformCommand::ConfigureClipboard(reconnect_off)),
            Message::PlatformCommand(PlatformCommand::ConfigureClipboard(on)),
        ] if pause.paused && reconnect_pause.paused && !off.enabled && !reconnect_off.enabled && on.enabled));
        worker.shutdown().expect("drain admitted history");
        let history = nxtools_clipboard::ClipboardHistory::open(temp.path().join("history.db"))
            .expect("reopen retained history");
        assert_eq!(
            history.counts().expect("retained history counts").text,
            1,
            "only the admitted history survives"
        );
    }

    fn fixture_text() -> ClipboardItem {
        ClipboardItem {
            id: 1,
            kind: ClipboardKind::Text,
            text: "first\r\nsecond".into(),
            image_path: String::new(),
            image_width: 0,
            image_height: 0,
            file_paths: Vec::new(),
            source: nxtools_clipboard::CaptureSource {
                window_handle: 1,
                process_id: 2,
                process_name: "browser".into(),
                process_icon_path: String::new(),
                window_title: "source".into(),
            },
            captured_at_unix_ms: 10,
            last_used_at_unix_ms: 10,
            copy_count: 1,
            is_favorite: false,
            is_pinned: false,
            deleted_at_unix_ms: None,
        }
    }

    fn unicode_native_format() -> StoredClipboardNativeFormat {
        StoredClipboardNativeFormat {
            format_id: 13,
            format_name: String::new(),
            data_path: "C:/formats/unicode.bin".into(),
            size_bytes: 28,
            sort_order: 0,
        }
    }

    #[test]
    fn four_clipboard_actions_build_the_expected_platform_outcomes() {
        let entry = fixture_text();
        let target = fixture_target();

        let original = command_for_action(
            &entry,
            &target,
            ClipboardAction::OriginalPaste,
            vec![unicode_native_format()],
        )
        .expect("original action");
        let copy = command_for_action(
            &entry,
            &target,
            ClipboardAction::CopyOnly,
            vec![unicode_native_format()],
        )
        .expect("copy action");
        let plain = command_for_action(&entry, &target, ClipboardAction::PlainText, Vec::new())
            .expect("plain action");
        let without_breaks = command_for_action(
            &entry,
            &target,
            ClipboardAction::WithoutLineBreaks,
            Vec::new(),
        )
        .expect("without line breaks action");

        let fixture_commands =
            include_str!("../../../../tests/fixtures/clipboard_action_commands.jsonl")
                .lines()
                .map(|line| {
                    nxtools_protocol::Envelope::decode_line(line)
                        .expect("decode shared action fixture")
                })
                .map(|envelope| match envelope.message {
                    nxtools_protocol::Message::PlatformCommand(command) => command,
                    message => panic!("unexpected shared fixture message: {message:?}"),
                })
                .collect::<Vec<_>>();
        assert_eq!(
            vec![
                original.clone(),
                copy.clone(),
                plain.clone(),
                without_breaks.clone()
            ],
            fixture_commands
        );

        assert!(matches!(
            original,
            PlatformCommand::PasteNativeFormats(PasteNativeFormatsCommand {
                paste_after_write: true,
                ..
            })
        ));
        assert!(matches!(
            copy,
            PlatformCommand::PasteNativeFormats(PasteNativeFormatsCommand {
                paste_after_write: false,
                ..
            })
        ));
        assert!(matches!(
            plain,
            PlatformCommand::PasteText(PasteTextCommand {
                ref text,
                paste_after_write: true,
                ..
            }) if text == "first\r\nsecond"
        ));
        assert!(matches!(
            without_breaks,
            PlatformCommand::PasteText(PasteTextCommand {

                ref text,
                paste_after_write: true,
                ..
            }) if text == "firstsecond"
        ));
    }
    #[test]
    fn plain_text_image_action_suppression_matches_the_written_path() {
        let mut entry = fixture_text();
        entry.kind = ClipboardKind::Image;
        entry.text.clear();
        entry.image_path = "C:/clipboard-images/example.png".into();

        let text = action_text_payload(&entry, ClipboardAction::PlainText)
            .expect("image plain-text action must produce a path");
        assert_eq!(entry.kind, ClipboardKind::Image);
        assert_eq!(text, "C:/clipboard-images/example.png");
    }

    #[test]
    fn image_original_uses_dib_bundle_and_shift_rejects_non_text() {
        let mut image = fixture_text();
        image.kind = ClipboardKind::Image;
        image.text.clear();
        image.image_path = "C:/images/capture.png".into();
        image.image_width = 640;
        image.image_height = 480;
        let directory = tempfile::tempdir().expect("native format fixture directory");
        let dib_path = directory.path().join("image-dib.bin");
        std::fs::write(&dib_path, vec![0_u8; 1024]).expect("write dib fixture");
        let dib = StoredClipboardNativeFormat {
            format_id: 8,
            format_name: String::new(),
            data_path: dib_path.to_string_lossy().into_owned(),
            size_bytes: 1024,
            sort_order: 0,
        };
        let mut file_drop_target = fixture_target();
        file_drop_target.process_name = "WindowsTerminal.exe".into();

        assert!(matches!(
            command_for_action(
                &image,
                &file_drop_target,
                ClipboardAction::OriginalPaste,
                vec![dib]
            )
            .expect("image original action"),
            PlatformCommand::PasteNativeFormats(PasteNativeFormatsCommand {
                image_path_fallback: Some(ref path),
                ..
            }) if path == "C:/images/capture.png"
        ));
        assert!(
            command_for_action(
                &image,
                &fixture_target(),
                ClipboardAction::WithoutLineBreaks,
                Vec::new()
            )
            .is_err()
        );
    }

    #[test]
    fn image_paste_to_zed_does_not_offer_the_same_image_as_a_file_drop() {
        let mut image = fixture_text();
        image.kind = ClipboardKind::Image;
        image.text.clear();
        image.image_path = "C:/images/capture.png".into();
        image.image_width = 640;
        image.image_height = 480;
        let mut target = fixture_target();
        target.process_name = "zed.exe".into();
        let directory = tempfile::tempdir().expect("native format fixture directory");
        let dib_path = directory.path().join("image-dib.bin");
        let drop_path = directory.path().join("image-drop.bin");
        std::fs::write(&dib_path, vec![0_u8; 1024]).expect("write dib fixture");
        std::fs::write(&drop_path, vec![0_u8; 256]).expect("write file-drop fixture");
        let formats = vec![
            StoredClipboardNativeFormat {
                format_id: 8,
                format_name: String::new(),
                data_path: dib_path.to_string_lossy().into_owned(),
                size_bytes: 1024,
                sort_order: 0,
            },
            StoredClipboardNativeFormat {
                format_id: 15,
                format_name: String::new(),
                data_path: drop_path.to_string_lossy().into_owned(),
                size_bytes: 256,
                sort_order: 1,
            },
        ];

        let command = command_for_action(&image, &target, ClipboardAction::OriginalPaste, formats)
            .expect("image original action");

        assert!(matches!(
            command,
            PlatformCommand::PasteNativeFormats(PasteNativeFormatsCommand {
                ref formats,
                image_path_fallback: None,
                ..
            }) if formats.iter().all(|format| format.format_id != 15)
        ));
    }
    #[test]
    fn image_original_falls_back_when_stored_native_format_is_missing() {
        let mut image = fixture_text();
        image.kind = ClipboardKind::Image;
        image.text.clear();
        image.image_path = "C:/images/capture.png".into();
        image.image_width = 640;
        image.image_height = 480;
        let command = command_for_action(
            &image,
            &fixture_target(),
            ClipboardAction::OriginalPaste,
            vec![StoredClipboardNativeFormat {
                format_id: 8,
                format_name: String::new(),
                data_path: "C:/missing/image-dib.bin".into(),
                size_bytes: 1024,
                sort_order: 0,
            }],
        )
        .expect("image fallback action");

        assert!(matches!(
            command,
            PlatformCommand::PasteImage(PasteImageCommand {
                ref image_path,
                paste_after_write: true,
                ..
            }) if image_path == "C:/images/capture.png"
        ));
    }
    #[test]
    fn clipboard_popup_is_hidden_until_topmost_confirmation() {
        let source = include_str!("clipboard.rs");
        let show_panel = source
            .split("fn show_panel(")
            .nth(1)
            .and_then(|source| source.split("fn command_for_action(").next())
            .expect("clipboard popup show implementation");
        let hidden = show_panel.find("show: false").expect("popup starts hidden");
        let bounds = show_panel
            .find("SetWindowBounds")
            .expect("popup applies native bounds");
        let topmost = show_panel
            .find("SetWindowTopmost")
            .expect("popup requests topmost");
        assert!(hidden < bounds);
        assert!(bounds < topmost);
        assert!(source.contains("SetWindowVisibilityCommand"));
    }
    #[test]
    fn stale_preview_callbacks_are_ignored_without_hiding_the_current_preview() {
        assert!(!preview_callback_is_current(
            2,
            1,
            Some(20),
            10,
            false,
            88,
            88
        ));
        assert!(!preview_callback_is_current(
            2,
            2,
            Some(20),
            20,
            false,
            88,
            77
        ));
        assert!(!preview_callback_is_current(
            2,
            2,
            Some(20),
            20,
            true,
            88,
            88
        ));
        assert!(preview_callback_is_current(
            2,
            2,
            Some(20),
            20,
            false,
            88,
            88
        ));
    }

    #[gpui_kit::test]
    fn settings_save_during_migration_result_delay_preserves_committed_destination(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        let temporary = tempfile::tempdir().expect("create synthetic migration directory");
        let source = temporary.path().join("source");
        let destination = temporary.path().join("destination");
        std::fs::create_dir_all(&source).expect("create source store");
        let config_path = temporary.path().join("config.json");
        let mut cached = AppConfig::default();
        cached.clipboard.storage_directory = source.to_string_lossy().into_owned();
        cached.save_to(&config_path).expect("seed source config");
        let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
            source.join("clipboard.db"),
            source.join("assets"),
            ClipboardRetentionPolicy {
                max_items: 1000,
                retention_days: 30,
            },
            1000,
        )
        .expect("spawn source worker");
        let shutdown = worker.clone();
        let (platform, _) = nxtools_ipc::platform_bridge(8);
        let application = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            cx.new(|cx| {
                ClipboardApplication::new(
                    worker,
                    Rc::new(platform),
                    cached,
                    config_path.clone(),
                    notifications,
                    cx,
                )
            })
        });
        let live_worker = cx.read(|cx| application.read(cx).history_worker.clone());
        // Real worker commits, but deliberately do not apply its UI result yet.
        let (_, committed) = live_worker
            .migrate_storage(destination.clone(), config_path.clone())
            .expect("worker commits destination before UI result");
        application.update(cx, |app, _| {
            app.config.clipboard.link_panel_copy_only = true;
            app.save_clipboard_section()
                .expect("save setting during result delay");
        });
        let disk = AppConfig::load_from(&config_path).expect("reload latest persisted config");
        assert_eq!(
            disk.clipboard.storage_directory,
            destination.to_string_lossy()
        );
        assert!(disk.clipboard.link_panel_copy_only);
        assert_eq!(
            live_worker.database_path(),
            destination.join("clipboard.db")
        );
        application.update(cx, |app, _| {
            app.refresh_committed_migration_config(committed);
            assert!(app.config.clipboard.link_panel_copy_only);
        });
        shutdown.shutdown().expect("flush synthetic worker");
    }
    #[gpui_kit::test]
    fn ref01_row_uses_saved_settings_at_activation_and_stale_close_preserves_reopened_panel(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use crate::tools::link_detector::DetectedLink;
        use gpui_kit::VisualTestContext;
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        cx.executor().allow_parking();
        let temporary = tempfile::tempdir().expect("synthetic link state");
        let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
            temporary.path().join("history.db"),
            temporary.path().join("assets"),
            ClipboardRetentionPolicy {
                max_items: 1000,
                retention_days: 30,
            },
            1000,
        )
        .expect("synthetic history worker");
        let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
        endpoint.set_connected_for_contract_test(true);
        let config_path = temporary.path().join("config.json");
        let mut config = AppConfig::default();
        config.clipboard.link_panel_copy_only = true;
        config.clipboard.link_panel_auto_open_browser = false;
        config
            .save_to(&config_path)
            .expect("seed saved clipboard settings");
        let application = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            cx.new(|cx| {
                ClipboardApplication::new(
                    worker.clone(),
                    Rc::new(platform),
                    config,
                    config_path.clone(),
                    notifications,
                    cx,
                )
            })
        });
        let link = |target: &str| {
            vec![DetectedLink {
                display: "synthetic web target".into(),
                copy_value: target.into(),
                kind: DetectedLinkKind::Web,
            }]
        };
        application.update(cx, |app, cx| {
            app.show_link_panel(link("https://first.invalid"), cx)
        });
        let first = *cx.windows().last().expect("first panel");
        application.update(cx, |app, cx| {
            app.show_link_panel(link("https://second.invalid"), cx)
        });
        assert_eq!(
            cx.windows(),
            vec![first],
            "restage original panel and timer"
        );
        application.update(cx, |app, _| {
            app.config.clipboard.link_panel_copy_only = false;
            app.config.clipboard.link_panel_auto_open_browser = true;
            app.save_clipboard_section().expect("persist current route");
        });
        let saved = AppConfig::load_from(&config_path).expect("read durable route");
        assert!(
            !saved.clipboard.link_panel_copy_only && saved.clipboard.link_panel_auto_open_browser
        );
        let mut surface = VisualTestContext::from_window(first, cx);
        surface.refresh().expect("restaged production row");
        let row = surface
            .debug_bounds("link-panel-name-0")
            .expect("real row target");
        surface.simulate_click(row.center(), Default::default());
        surface.run_until_parked();
        assert!(cx.windows().is_empty());
        let commands = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
            .filter_map(|envelope| match envelope.message {
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
                    command,
                )) => Some(command),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            commands,
            vec![OpenShellTargetCommand {
                target: "https://second.invalid".into(),
                action: ShellOpenAction::Open,
            }],
            "current persisted settings win over capture-time settings"
        );
        application.update(cx, |app, cx| {
            app.show_link_panel(link("https://third.invalid"), cx)
        });
        let third = *cx.windows().last().expect("replacement panel");
        assert_ne!(first, third);
        application.update(cx, |app, _| {
            assert!(
                !app.forget_link_panel_if_current(first),
                "old callback cannot retire replacement"
            );
            assert_eq!(app.link_panel.window, Some(third));
            app.config.clipboard.link_panel_auto_open_browser = false;
            app.save_clipboard_section()
                .expect("persist disabled auto-open");
        });
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("untouched".into()));
        let mut surface = VisualTestContext::from_window(third, cx);
        surface.refresh().expect("replacement production row");
        let row = surface
            .debug_bounds("link-panel-name-0")
            .expect("replacement row");
        surface.simulate_click(row.center(), Default::default());
        surface.run_until_parked();
        assert_eq!(
            cx.read_from_clipboard()
                .expect("clipboard remains unchanged")
                .text()
                .as_deref(),
            Some("untouched")
        );
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("row open ignores legacy browser setting");
        assert_eq!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
                OpenShellTargetCommand {
                    target: "https://third.invalid".into(),
                    action: ShellOpenAction::Open,
                }
            ))
        );
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        // Close one actual row, reopen before its deferred owner callback, then
        // let the stale callback run while a replacement panel is live.
        application.update(cx, |app, cx| {
            app.show_link_panel(link("https://retired.invalid"), cx)
        });
        let retired = *cx.windows().last().expect("retired panel");
        let mut retired_surface = VisualTestContext::from_window(retired, cx);
        retired_surface.refresh().expect("retired row");
        let retired_row = retired_surface
            .debug_bounds("link-panel-name-0")
            .expect("retired row hit target");
        cx.write_to_clipboard(gpui_kit::ClipboardItem::new_string("before-stale".into()));
        let reopener = application.clone();
        let retired_id = retired.window_id();
        let replacement_opened = Rc::new(std::cell::Cell::new(false));
        let opened = Rc::clone(&replacement_opened);
        let _reopen_on_close = cx.update(|cx| {
            cx.on_window_closed(move |cx, closed| {
                if closed == retired_id {
                    assert_eq!(
                        cx.read_from_clipboard()
                            .and_then(|item| item.text())
                            .as_deref(),
                        Some("before-stale"),
                        "replacement is established before deferred row copying"
                    );
                    reopener.update(cx, |app, cx| {
                        app.show_link_panel(link("https://current.invalid"), cx)
                    });
                    opened.set(true);
                }
            })
        });
        retired_surface.simulate_click(retired_row.center(), Default::default());
        assert!(
            replacement_opened.get(),
            "actual close opens the replacement"
        );
        let current = *cx.windows().last().expect("current panel");
        assert_ne!(retired, current);
        cx.run_until_parked();
        assert_eq!(
            cx.windows(),
            vec![current],
            "stale action keeps current window"
        );
        assert_eq!(
            cx.read_from_clipboard()
                .expect("clipboard after stale action")
                .text()
                .as_deref(),
            Some("before-stale"),
            "stale row action cannot overwrite clipboard"
        );
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        current
            .update(cx, |_, window, _| window.remove_window())
            .expect("close current fixture");
        worker.shutdown().expect("flush synthetic history");
    }
}

#[cfg(test)]
#[path = "clipboard_link_copy_tests.rs"]
mod link_copy_tests;
