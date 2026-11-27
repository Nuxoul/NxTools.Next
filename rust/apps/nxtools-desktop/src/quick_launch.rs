#[path = "quick_launch/archive_open.rs"]
mod archive_open;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::Read;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use chrono::{DateTime, Local};
use gpui_kit::base::Root;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{
    Animation, AnimationExt, AnyElement, AnyWindowHandle, App, Bounds, Context, Div, Entity,
    EntityInputHandler, EventEmitter, Focusable, HighlightStyle, Image, ImageFormat, ImageSource,
    IntoElement, ObjectFit, Pixels, Render, ScrollHandle, ScrollStrategy, Size, StyledText,
    Subscription, UniformListScrollHandle, WeakEntity, Window, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, div, ease_out_quint, img, point,
    prelude::*, px, size, uniform_list,
};
use nxtools_core::{
    AppConfig, ClipboardHotKey, ClipboardHotKeyModifier, ProcessBlacklistFeature,
    QuickLaunchSettings,
};
#[cfg(test)]
use nxtools_finder::query_requests_file_index;
use nxtools_finder::{
    ApplicationCatalogSnapshot, ApplicationSource, CatalogStatus, ExternalSearchEntry,
    FinderCatalog, QuickLaunchHistorySummary, RecentDirectory, SearchResult, SearchResultAction,
    query_requests_file_index_with_commands, scan_application_roots,
};
use nxtools_ipc::{PlatformBridge, PlatformBridgeError, PlatformNotification};
use nxtools_protocol::{
    ActivateWindowCommand, ConfigureQuickLaunchCommand, DirectoryNavigationHost,
    ElevatedDirectoryHelperRequest, ExtractShellIconCommand, IndexSearchBatch, IndexSearchContext,
    IndexSearchItem, IndexSearchRequest, IndexSettings, IndexStatus, IndexerCommand, IndexerEvent,
    InvokeShellActionCommand, LaunchProcessCommand, NavigateDirectoryCommand,
    OpenShellTargetCommand, PasteTextCommand, PickDirectoryCommand, PickProgramCommand,
    PlatformCommand, PlatformCommandResult, PlatformEvent, QueryShellActionsCommand,
    QueryShellSubmenuCommand, QuickLaunchKeyboardHealthChanged, QuickLaunchKeyboardHealthState,
    QuickLaunchPanelRequest, SetQuickLaunchHotKeyRecordingCommand, SetWindowBoundsCommand,
    SetWindowNoActivateCommand, SetWindowPositionCommand, SetWindowTopmostCommand,
    SetWindowVisibilityCommand, ShellActionItem, ShellActionMenu, ShellOpenAction,
};
use tracing::{error, info, warn};

use crate::icons::{MaterialIcon, shell_icon_path_is_renderable};
use crate::notifications::NotificationCenter;
use crate::runtime::{IndexerBridge, IndexerNotification};
use crate::theme::Theme;
use crate::ui::components::state::{empty_state, progress_state};
use crate::ui::metrics::{
    COMPACT_RESULT_ICON_RAIL_WIDTH, COMPACT_RESULT_ROW_HEIGHT, COMPACT_RESULT_TRAILING_RAIL_WIDTH,
    RADIUS_CONTROL, SPACING_4, SPACING_8, SPACING_12,
};
#[cfg(test)]
use crate::ui::typography::UI_FONT_FAMILY;
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL, TEXT_META,
};
use crate::ui::{
    PREVIEW_APPEAR_DURATION, native_window_handle, preview_media_surface, preview_shell,
};

mod file_dialog_search;
use file_dialog_search::FileDialogSearchSession;
mod installed_applications;

#[cfg(debug_assertions)]
pub(crate) mod visual_preview;

/// Count-only index status changes are file-churn telemetry: they coalesce
/// into at most one log record per interval while structural changes always
/// log immediately.
const INDEX_COUNT_STATUS_LOG_INTERVAL: Duration = Duration::from_secs(60);
const VALIDATION_INSTALL_DIRECTORY_NAME: &str = "NxTools.Next-Validation-20261004";

pub(crate) fn should_manage_indexer_service_for_executable(executable: &Path) -> bool {
    executable
        .parent()
        .and_then(Path::file_name)
        .map(|name| {
            !name
                .to_string_lossy()
                .eq_ignore_ascii_case(VALIDATION_INSTALL_DIRECTORY_NAME)
        })
        .unwrap_or(true)
}

fn should_manage_indexer_service() -> bool {
    std::env::current_exe()
        .map(|executable| should_manage_indexer_service_for_executable(&executable))
        .unwrap_or(true)
}

fn indexer_deployment_requires_protected_restart(result: &PlatformCommandResult) -> bool {
    result.success && matches!(result.data.as_deref(), Some("installed" | "upgraded"))
}

pub(crate) fn should_log_index_status_change(
    count_only: bool,
    last_logged: Option<Instant>,
    now: Instant,
) -> bool {
    !count_only
        || last_logged
            .map(|logged| now.duration_since(logged) >= INDEX_COUNT_STATUS_LOG_INTERVAL)
            .unwrap_or(true)
}

/// Indexed provider rows enter the Finder catalog ranking as external entries.
fn indexed_search_entries(items: &[IndexSearchItem]) -> Vec<ExternalSearchEntry> {
    items
        .iter()
        .map(|item| {
            let normalized_path = item.path.replace('/', "\\").to_lowercase();
            ExternalSearchEntry {
                id: format!("indexed:{normalized_path}"),
                name: item.name.clone(),
                target: item.path.clone(),
                source: if item.is_directory {
                    ApplicationSource::IndexedDirectory
                } else {
                    ApplicationSource::IndexedFile
                },
            }
        })
        .collect()
}

const PANEL_WIDTH: f32 = 620.0;
const LAUNCHER_LIST_MAX_HEIGHT: f32 = 470.0;
const LAUNCHER_RESULT_ROW_HEIGHT: f32 = 52.0;
const LAUNCHER_RESULT_ICON_SIZE: f32 = 32.0;
const LAUNCHER_APPLICATION_TITLE_SIZE: f32 = 16.0;
const LAUNCHER_FILE_TITLE_SIZE: f32 = 15.0;
const LAUNCHER_TITLE_LINE_HEIGHT: f32 = 20.0;
const LAUNCHER_SUBTITLE_LINE_HEIGHT: f32 = 24.0;
const LAUNCHER_STATE_MIN_HEIGHT: f32 = 96.0;
const LAUNCHER_STATUS_FOOTER_HEIGHT: f32 = 36.0;
const PANEL_HEIGHT: f32 = COLLAPSED_PANEL_HEIGHT + LAUNCHER_LIST_MAX_HEIGHT;
const SEARCH_HEADER_HEIGHT: f32 = 58.0;
const SEARCH_INPUT_TEXT_SIZE: f32 = 26.0;
const SEARCH_INPUT_LINE_HEIGHT: f32 = 34.0;
const COLLAPSED_PANEL_HEIGHT: f32 = SEARCH_HEADER_HEIGHT + 2.0;
const RESULT_LIMIT: usize = 200;
const RECENT_DIRECTORY_LIMIT: usize = 64;
const INDEX_CANDIDATE_LIMIT: usize = 200;
const ICON_PREFETCH_LIMIT: usize = 10;
const ICON_RESOURCE_CACHE_LIMIT: usize = 96;
const ICON_VISIBLE_OVERSCAN: usize = 2;

pub(crate) fn quick_launch_process_is_blacklisted(config: &AppConfig, process_name: &str) -> bool {
    config.process_is_blacklisted(process_name, ProcessBlacklistFeature::QuickLaunch)
}

const ICON_RETRY_DELAY: Duration = Duration::from_millis(500);
const REQUEST_MAX_AGE_MS: i64 = 1_000;
const AUTO_QUICK_SWITCH_REQUEST_MAX_AGE_MS: i64 = 3_000;
const OPEN_GRACE: Duration = Duration::from_millis(180);
const FAILED_NAVIGATION_FOREGROUND_GUARD: Duration = Duration::from_millis(750);
const RESULTS_APPEAR_DURATION: Duration = crate::ui::motion::ENTER_DURATION;
const INDEX_SEARCH_FAILURE_MESSAGE: &str = "最近修改查询超出安全处理预算，请缩小范围后重试";
const QUICK_LAUNCH_PREVIEW_HOVER_DELAY: Duration = Duration::from_millis(420);
const QUICK_LAUNCH_PREVIEW_SWITCH_DELAY: Duration = Duration::from_millis(110);
const QUICK_LAUNCH_PREVIEW_LEAVE_DELAY: Duration = Duration::from_millis(140);
const QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION: Duration = Duration::from_millis(90);
const QUICK_LAUNCH_EMPTY_RESULTS_LABEL: &str = "没有匹配项";
const QUICK_LAUNCH_ACTION_CONTEXT_HEIGHT: f32 = 24.0;
const QUICK_LAUNCH_RESULT_SHORTCUT_HINTS: [&str; 9] = [
    "Ctrl+1", "Ctrl+2", "Ctrl+3", "Ctrl+4", "Ctrl+5", "Ctrl+6", "Ctrl+7", "Ctrl+8", "Ctrl+9",
];
const QUICK_LAUNCH_PREVIEW_WIDTH: f32 = 400.0;
const QUICK_LAUNCH_PREVIEW_HEIGHT: f32 = 410.0;
const QUICK_LAUNCH_PREVIEW_HEADER_HEIGHT: f32 = 72.0;
const QUICK_LAUNCH_PREVIEW_FOOTER_HEIGHT: f32 = 30.0;
const QUICK_LAUNCH_PREVIEW_GAP: f32 = 10.0;
const QUICK_LAUNCH_TEXT_PREVIEW_LIMIT: usize = 96 * 1024;
const QUICK_LAUNCH_IMAGE_FILE_LIMIT: u64 = 24 * 1024 * 1024;
const QUICK_LAUNCH_IMAGE_PIXEL_LIMIT: u64 = 16_000_000;
const QUICK_LAUNCH_PREVIEW_STAGING_COORDINATE: i32 = -32_000;
const QUICK_LAUNCH_PANEL_STAGING_COORDINATE: i32 = -30_000;
mod actions;
mod extension_providers;
mod floating_assistant;
mod icon;
mod monitor_presentation;
mod panel;
mod result_motion;
mod retained_text;
use retained_text::QuickLaunchResults;
pub(crate) mod preview;
mod preview_presentation;
use preview_presentation::PreviewMaterialAction;
mod search;

use self::actions::*;
use self::icon::*;
use self::panel::*;
use self::preview::*;
use self::search::*;

#[derive(Clone)]
struct QuickLaunchWindow {
    window: AnyWindowHandle,
    panel: WeakEntity<QuickLaunchPanel>,
    native_window_handle: i64,
}

struct PendingPopupResize {
    command_id: uuid::Uuid,
    started_at: Instant,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QuickLaunchApplicationEvent {
    CustomCommandProgramPicked {
        correlation_id: uuid::Uuid,
        index: usize,
        path: String,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct QuickLaunchPerformanceSnapshot {
    pub last_local_search_us: Option<u64>,
    pub last_index_search_us: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QuickLaunchHotkeyLifecycle {
    Applied,
    WaitingForAdapter { status: String },
    WaitingForRecording { status: String },
    Recording,
    Applying { candidate: Option<String> },
    Failed { error: String },
}

impl QuickLaunchHotkeyLifecycle {
    pub(crate) fn owns_recording_session(&self) -> bool {
        matches!(self, Self::WaitingForRecording { .. } | Self::Recording)
    }
}

pub(crate) struct QuickLaunchApplication {
    catalog: Rc<RefCell<FinderCatalog>>,
    catalog_refresh_in_progress: bool,
    pending_installed_applications: Option<installed_applications::PendingInstalledApplications>,
    platform: Rc<PlatformBridge>,
    indexer: Rc<IndexerBridge>,
    config: AppConfig,
    config_path: PathBuf,
    log_dir: PathBuf,
    notifications: Entity<NotificationCenter>,
    popup: Option<QuickLaunchWindow>,
    popup_monitor: Option<nxtools_protocol::CursorMonitorSnapshot>,
    popup_presentation: Option<monitor_presentation::PopupPresentation>,
    preview: QuickLaunchPreviewSession,
    pending_configuration: HashSet<uuid::Uuid>,
    latest_configuration: Option<(uuid::Uuid, Instant)>,
    feature_save_error: Option<String>,
    feature_events_after: i64,
    pending_hotkey: Option<(uuid::Uuid, ClipboardHotKey)>,
    hotkey_error: Option<String>,
    keyboard_health: Option<(QuickLaunchKeyboardHealthState, i32)>,
    keyboard_health_revision: u64,
    keyboard_capability_unavailable: bool,
    keyboard_health_notified: bool,
    pending_keyboard_reenable: Option<(uuid::Uuid, Instant, u64)>,
    pending_recording: Option<(uuid::Uuid, bool)>,
    recording_active: bool,
    recording_desired: bool,
    pending_actions: HashMap<uuid::Uuid, PendingQuickLaunchAction>,
    pending_auto_quick_switches: HashMap<uuid::Uuid, (i64, String)>,
    file_dialog_search: FileDialogSearchSession,
    navigation_foreground_guard: Option<NavigationForegroundGuard>,
    popup_session_id: uuid::Uuid,
    active_summon: Option<(i64, i64)>,
    pending_shell_queries: HashMap<uuid::Uuid, PendingShellQuery>,
    pending_window_activation: HashMap<uuid::Uuid, PendingWindowActivation>,
    pending_panel_hides: HashMap<uuid::Uuid, PendingPanelHide>,
    pending_popup_resize: Option<PendingPopupResize>,
    popup_origin_device_pixels: Option<(i32, i32)>,
    icon_resources: IconResourceCache,
    pending_icons: HashMap<uuid::Uuid, PendingIconResource>,
    pending_icon_resources: HashMap<String, uuid::Uuid>,
    failed_icon_resources: HashMap<String, Instant>,
    indexer_service_bootstrap: IndexerServiceBootstrap,
    pending_exclusion_picker: Option<PendingIndexExclusionPicker>,
    pending_program_picker: Option<PendingProgramPicker>,
    elevated_helper_request: Option<ElevatedDirectoryHelperRequest>,
    pending_elevated_helper: Option<uuid::Uuid>,
    acknowledged_elevated_helper_pipe: Option<String>,
    last_sent_index_settings: Option<IndexSettings>,
    configuration_status: String,
    index_status: Option<IndexStatus>,
    index_search: IndexSearchSession,
    pending_index_search: Option<nxtools_protocol::IndexSearchCancel>,
    last_local_search_us: Option<u64>,
    last_count_status_log: Option<Instant>,
    extension_providers: extension_providers::ExtensionProviders,
}

impl EventEmitter<QuickLaunchApplicationEvent> for QuickLaunchApplication {}

#[derive(Clone, Copy)]
struct PendingIndexExclusionPicker {
    correlation_id: uuid::Uuid,
    started_at: Instant,
}

#[derive(Clone, Copy)]
struct PendingProgramPicker {
    correlation_id: uuid::Uuid,
    command_index: usize,
    started_at: Instant,
}
impl QuickLaunchApplication {
    pub(crate) fn extension_search(&self, query: &str) -> nxtools_extensions::HostReply {
        match self.catalog.borrow().search_with_packages(query,&[],&self.config.quick_launch.custom_commands,&self.config.extensions,32) {
            Ok(results)=>nxtools_extensions::HostReply{success:true,value:serde_json::Value::Array(results.into_iter().map(|result|serde_json::json!({"id":result.entry.id,"title":result.entry.name,"target":result.entry.target})).collect())},
            Err(_)=>nxtools_extensions::HostReply::error("search_failed"),
        }
    }
    pub(crate) fn extension_search_catalog(&self, query: &str) -> nxtools_extensions::HostReply {
        match self.catalog.borrow().search_with_packages(
            query,
            &[],
            &self.config.quick_launch.custom_commands,
            &self.config.extensions,
            33,
        ) {
            Ok(mut results) => {
                let truncated = results.len() > 32;
                results.truncate(32);
                nxtools_extensions::HostReply {
                    success: true,
                    value: serde_json::json!({"items":results.into_iter().map(|result|serde_json::json!({"id":result.entry.id,"title":result.entry.name,"target":result.entry.target})).collect::<Vec<_>>(),"scope":"local_catalog","sources":["catalog","custom_commands","declarative_extensions"],"excluded_sources":["indexed_files","script_providers"],"complete":!truncated,"truncated":truncated}),
                }
            }
            Err(_) => nxtools_extensions::HostReply::error("search_failed"),
        }
    }
    pub(crate) fn extension_set_query(
        &mut self,
        query: String,
        cancellation: tokio::sync::watch::Receiver<bool>,
        reply: async_channel::Sender<nxtools_extensions::HostReply>,
        cx: &mut Context<Self>,
    ) {
        if query.len() > 4096 || *cancellation.borrow() {
            let _ = reply.try_send(nxtools_extensions::HostReply::error("invalid_or_cancelled"));
            return;
        }
        let Some(popup) = self.popup.clone() else {
            let _ = reply.try_send(nxtools_extensions::HostReply::error("ui_unavailable"));
            return;
        };
        cx.defer(move |cx| {
            let result = popup
                .window
                .update(cx, |_, window, cx| {
                    if *cancellation.borrow() {
                        return false;
                    }
                    let input = popup
                        .panel
                        .update(cx, |panel, _| {
                            panel.is_open.then(|| panel.search_input.clone())
                        })
                        .ok()
                        .flatten();
                    if let Some(input) = input {
                        input.update(cx, |input, cx| input.set_value(query, window, cx));
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(false);
            let _ = reply.try_send(if result {
                nxtools_extensions::HostReply {
                    success: true,
                    value: serde_json::json!({}),
                }
            } else {
                nxtools_extensions::HostReply::error("ui_unavailable")
            });
        });
    }

    pub(crate) fn new(
        catalog: FinderCatalog,
        platform: Rc<PlatformBridge>,
        indexer: Rc<IndexerBridge>,
        config: AppConfig,
        config_path: PathBuf,
        log_dir: PathBuf,
        notifications: Entity<NotificationCenter>,
    ) -> Self {
        Self {
            catalog: Rc::new(RefCell::new(catalog)),
            catalog_refresh_in_progress: false,
            pending_installed_applications: None,
            platform,
            indexer,
            config,
            config_path,
            log_dir,
            notifications,
            popup: None,
            popup_monitor: None,
            popup_presentation: None,
            preview: QuickLaunchPreviewSession::default(),
            pending_configuration: HashSet::new(),
            latest_configuration: None,
            feature_save_error: None,
            feature_events_after: i64::MIN,
            pending_hotkey: None,
            hotkey_error: None,
            keyboard_health: None,
            keyboard_health_revision: 0,
            pending_keyboard_reenable: None,
            keyboard_capability_unavailable: false,
            keyboard_health_notified: false,
            pending_recording: None,
            recording_active: false,
            recording_desired: false,
            pending_actions: HashMap::new(),
            pending_auto_quick_switches: HashMap::new(),
            file_dialog_search: FileDialogSearchSession::default(),
            navigation_foreground_guard: None,
            popup_session_id: uuid::Uuid::new_v4(),
            active_summon: None,
            pending_shell_queries: HashMap::new(),
            pending_window_activation: HashMap::new(),
            pending_panel_hides: HashMap::new(),
            pending_popup_resize: None,
            popup_origin_device_pixels: None,
            icon_resources: IconResourceCache::bounded(ICON_RESOURCE_CACHE_LIMIT),
            pending_icons: HashMap::new(),
            pending_icon_resources: HashMap::new(),
            failed_icon_resources: HashMap::new(),
            indexer_service_bootstrap: IndexerServiceBootstrap::default(),
            pending_exclusion_picker: None,
            pending_program_picker: None,
            elevated_helper_request: None,
            pending_elevated_helper: None,
            acknowledged_elevated_helper_pipe: None,
            last_sent_index_settings: None,
            configuration_status: "等待 Windows 适配器".into(),
            index_status: None,
            index_search: IndexSearchSession::default(),
            pending_index_search: None,
            last_local_search_us: None,
            last_count_status_log: None,
            extension_providers: Default::default(),
        }
    }

    /// Rust foreground filtering consumes the same durable global rule snapshot.
    pub(crate) fn apply_live_blacklist(
        &mut self,
        settings: nxtools_core::ProcessBlacklistSettings,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        let started_at = std::time::Instant::now();
        self.config.set_process_blacklist_settings(settings);
        info!(event = "global.blacklist.quick_launch_updated", %correlation_id,
            rule_count = self.config.process_blacklist.entries().len(),
            elapsed_ms = started_at.elapsed().as_millis(), "quick launch live rules updated");
        let settings = self.config.quick_launch.clone();
        let _ = self.send_configuration_correlated(
            &settings,
            uuid::Uuid::new_v4(),
            Some(correlation_id),
        );
        cx.notify();
    }

    pub(crate) fn is_process_excluded(&self, process_name: &str) -> bool {
        quick_launch_process_is_blacklisted(&self.config, process_name)
    }

    /// Persists only the Quick Launch section, merged into the file as it is
    /// now, then refreshes the other cached sections from the merged result so
    /// this module never writes a stale copy of anything it does not own.
    fn save_quick_launch_section(&mut self) -> anyhow::Result<()> {
        let quick_launch = self.config.quick_launch.clone();
        let merged = AppConfig::update_quick_launch_settings(&self.config_path, quick_launch)?;
        self.config = merged;
        Ok(())
    }

    pub(crate) fn feature_enabled(&self) -> bool {
        self.config.quick_launch.enabled
    }

    pub(crate) fn feature_configuration_status(&self) -> &str {
        self.feature_save_error.as_deref().unwrap_or_else(|| {
            if self.latest_configuration.is_some() {
                "正在应用"
            } else {
                &self.configuration_status
            }
        })
    }

    pub(crate) fn set_feature_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.feature_enabled() == enabled {
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        info!(event = "quick_launch.enablement.accepted", %correlation_id, enabled);
        let mut settings = self.settings();
        settings.enabled = enabled;
        match AppConfig::update_quick_launch_settings(&self.config_path, settings.clone()) {
            Ok(merged) => self.config = merged,
            Err(error) => {
                self.feature_save_error = Some(format!("保存失败：{error}"));
                warn!(event = "quick_launch.enablement.failed", %correlation_id, enabled,
                    stage = "save", elapsed_ms = started.elapsed().as_millis());
                cx.notify();
                return;
            }
        }
        self.feature_save_error = None;
        self.feature_events_after = nxtools_ipc::unix_time_ms();
        self.pending_hotkey = None;
        if !enabled {
            self.retire_installed_applications("disabled");
            self.set_hotkey_recording(false);
            self.pending_auto_quick_switches.clear();
            self.pending_shell_queries.clear();
            self.pending_popup_resize = None;
            self.cancel_extension_providers();
            self.pending_panel_hides.clear();
            self.hide_popup(cx);
            if let Some(popup) = self.popup.take() {
                let _ = popup
                    .window
                    .update(cx, |_, window, _| window.remove_window());
            }
            self.popup_origin_device_pixels = None;
            if let Some(preview) = self.preview.window.take() {
                let _ = preview
                    .identity
                    .gpui
                    .update(cx, |_, window, _| window.remove_window());
            }
            self.retire_file_dialog_search(cx);
        }
        info!(event = "quick_launch.enablement.saved", %correlation_id, enabled,
            elapsed_ms = started.elapsed().as_millis());
        if self
            .send_configuration_for_with_id(&settings, correlation_id)
            .is_some()
        {
            self.schedule_feature_configuration_timeout(correlation_id, cx);
        }
        cx.notify();
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
                    owner.configuration_status = "应用失败：Windows 未确认".into();
                    warn!(event = "quick_launch.configuration.failed", correlation_id = %id,
                        stage = "timeout", elapsed_ms = started.elapsed().as_millis());
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn admits_feature_event(&self, event_unix_ms: i64) -> bool {
        self.feature_enabled() && event_unix_ms > self.feature_events_after
    }

    pub(crate) fn settings(&self) -> QuickLaunchSettings {
        self.config.quick_launch.clone()
    }

    pub(crate) fn extension_config_snapshot(&self) -> (AppConfig, PathBuf) {
        (self.config.clone(), self.config_path.clone())
    }

    pub(crate) fn refresh_extension_storage_metadata(
        &mut self,
        packages: &nxtools_extensions::PackageRegistry,
        cx: &mut Context<Self>,
    ) {
        if crate::extensions::service::refresh_storage_metadata(
            &mut self.config.extensions,
            packages,
        ) {
            // Metadata-only projection must not cancel current provider requests/results.
            cx.notify();
        }
    }
    pub(crate) fn apply_extension_snapshot(
        &mut self,
        packages: nxtools_extensions::PackageRegistry,
        cx: &mut Context<Self>,
    ) {
        self.cancel_extension_providers();
        let mut packages = packages;
        if let Some(current) = cx
            .try_global::<crate::extensions::ui_host::ExtensionServiceGlobal>()
            .and_then(|global| global.0.runtime.snapshot().ok())
        {
            crate::extensions::service::refresh_storage_metadata(&mut packages, &current);
        }
        self.config.extensions = packages;
        self.refresh_indexed_results(cx, true);
        cx.notify();
    }

    pub(crate) fn catalog_status(&self) -> CatalogStatus {
        self.catalog.borrow().status().clone()
    }

    pub(crate) fn index_status(&self) -> Option<IndexStatus> {
        self.index_status.clone()
    }

    pub(crate) fn indexer_connected(&self) -> bool {
        self.indexer.is_connected()
    }

    pub(crate) fn history_summary(&self) -> QuickLaunchHistorySummary {
        self.catalog.borrow().history_summary()
    }

    pub(crate) fn performance_snapshot(&self) -> QuickLaunchPerformanceSnapshot {
        QuickLaunchPerformanceSnapshot {
            last_local_search_us: self.last_local_search_us,
            last_index_search_us: self.index_search.last_completed_elapsed_us,
        }
    }

    #[cfg(test)]
    pub(crate) fn set_performance_snapshot_for_test(
        &mut self,
        last_local_search_us: Option<u64>,
        last_index_search_us: Option<u64>,
    ) {
        self.last_local_search_us = last_local_search_us;
        self.index_search.last_completed_elapsed_us = last_index_search_us;
    }

    pub(crate) fn log_directory(&self) -> &Path {
        &self.log_dir
    }

    pub(crate) fn clear_history(&mut self, cx: &mut Context<Self>) {
        let clear_result = self.catalog.borrow_mut().clear_history();
        match clear_result {
            Ok(()) => {
                self.index_search.invalidate();
                self.pending_icons.clear();
                self.pending_icon_resources.clear();
                let popup = self.popup.clone();
                let application = cx.entity().downgrade();
                let notifications = self.notifications.clone();
                cx.defer(move |cx| {
                    let reset = popup.as_ref().and_then(|popup| {
                        popup
                            .window
                            .update(cx, |_, window, app| {
                                popup.panel.update(app, |panel, cx| {
                                    panel.reset_after_history_clear(window, cx)
                                })
                            })
                            .ok()
                            .and_then(Result::ok)
                    });
                    if let Some((query, panel_generation)) = reset {
                        let popup = popup.clone();
                        let application = application.clone();
                        cx.defer(move |cx| {
                            let Some(application_entity) = application.upgrade() else {
                                return;
                            };
                            let update = application_entity.update(cx, |application, cx| {
                                if query.trim().is_empty() {
                                    QuickLaunchSearchUpdate {
                                        results: Vec::new(),
                                        pending: false,
                                    }
                                } else {
                                    application.search_and_request_icons(&query, cx)
                                }
                            });
                            let Some(popup) = popup else {
                                return;
                            };
                            cx.defer(move |cx| {
                                let _ = popup.window.update(cx, |_, _, app| {
                                    let _ = popup.panel.update(app, |panel, cx| {
                                        if panel.query_dispatch_generation != panel_generation
                                            || panel.recent_directory_mode
                                            || panel.search_input.read(cx).value().as_ref() != query
                                        {
                                            return;
                                        }
                                        panel.replace_results(update.results, cx);
                                        panel.search_pending = update.pending;
                                        cx.notify();
                                    });
                                });
                            });
                        });
                    }
                    notifications.update(cx, |notifications, cx| {
                        notifications.show_success(
                            "quick-launch-history",
                            "速启历史已清空",
                            "排序学习、关键词关联和最近目录已重置。",
                            cx,
                        );
                    });
                });
                cx.notify();
            }
            Err(error) => {
                error!(%error, "failed to clear quick-launch history");
                let notifications = self.notifications.clone();
                let message = error.to_string();
                cx.defer(move |cx| {
                    notifications.update(cx, |notifications, cx| {
                        notifications.show_error(
                            "quick-launch-history",
                            "无法清空速启历史",
                            message,
                            cx,
                        );
                    });
                });
            }
        }
    }

    pub(crate) fn open_log_directory(&self) {
        if let Err(error) =
            self.platform
                .send_command(PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                    target: self.log_dir.to_string_lossy().into_owned(),
                    action: ShellOpenAction::Open,
                }))
        {
            error!(%error, path = %self.log_dir.display(), "failed to queue log directory open");
        }
    }

    pub(crate) fn prepare_file_index(&mut self, cx: &mut Context<Self>) {
        if self.indexer.is_connected() {
            self.refresh_index_volumes(cx);
            return;
        }
        if !should_manage_indexer_service() {
            return;
        }
        self.ensure_indexer_service(false, cx);
    }

    fn ensure_indexer_service(&mut self, automatic: bool, cx: &mut Context<Self>) {
        if self.indexer_service_bootstrap.pending.is_some() {
            return;
        }
        let command_id = uuid::Uuid::new_v4();
        self.indexer_service_bootstrap.begin(command_id, automatic);
        match self
            .platform
            .send_command_with_id(PlatformCommand::EnsureIndexerService, command_id)
        {
            Ok(_) => {
                if !automatic {
                    self.notifications.update(cx, |notifications, cx| {
                        notifications.show_info(
                            "quick-launch-indexer-service",
                            "正在准备索引服务",
                            "首次安装或版本升级需要 Windows 授权；主程序仍以普通权限运行。",
                            cx,
                        );
                    });
                }
            }
            Err(error) => {
                self.indexer_service_bootstrap.finish(command_id);
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-indexer-service",
                        "无法准备索引服务",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
        cx.notify();
    }

    pub(crate) fn refresh_index_volumes(&mut self, cx: &mut Context<Self>) {
        match self
            .indexer
            .send_command(nxtools_protocol::IndexerCommand::RefreshVolumes)
        {
            Ok(_) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_info(
                        "quick-launch-volume-refresh",
                        "重建本地文件索引",
                        "正在重新发现 NTFS 卷并建立新的 MFT 快照",
                        cx,
                    );
                });
            }
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-volume-refresh",
                        "无法重建本地文件索引",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
    }

    pub(crate) fn set_indexed_volumes(
        &mut self,
        indexed_volumes: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        self.config.quick_launch.indexed_volumes = indexed_volumes;
        self.persist_and_sync_index_settings(cx);
    }

    pub(crate) fn set_auto_quick_switch(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.quick_launch.auto_quick_switch == enabled {
            return;
        }
        self.config.quick_launch.auto_quick_switch = enabled;
        self.persist_and_sync_quick_launch_behavior(cx);
    }

    pub(crate) fn set_file_dialog_search(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.quick_launch.file_dialog_search == enabled {
            return;
        }
        self.config.quick_launch.file_dialog_search = enabled;
        if !enabled {
            self.hide_file_dialog_search_window(cx);
        }
        self.persist_and_sync_quick_launch_behavior(cx);
    }

    pub(crate) fn add_custom_command(&mut self, cx: &mut Context<Self>) -> usize {
        let index = self.config.quick_launch.custom_commands.len();
        self.config
            .quick_launch
            .custom_commands
            .push(nxtools_core::CustomCommandSettings {
                enabled: false,
                title: "新命令".into(),
                ..nxtools_core::CustomCommandSettings::default()
            });
        self.persist_custom_commands(cx);
        index
    }

    pub(crate) fn update_custom_command(
        &mut self,
        index: usize,
        command: nxtools_core::CustomCommandSettings,
        cx: &mut Context<Self>,
    ) -> bool {
        let mut next = self.config.quick_launch.custom_commands.clone();
        let Some(slot) = next.get_mut(index) else {
            return false;
        };
        *slot = command;
        let mut settings = self.config.quick_launch.clone();
        settings.custom_commands = next.clone();
        settings.normalize();
        let validation = settings.validate_custom_commands().and_then(|()| {
            settings
                .custom_commands
                .iter()
                .filter(|command| command.enabled)
                .try_for_each(|command| {
                    nxtools_finder::validate_custom_command_template(&command.argument_template)
                        .map_err(|error| format!("命令“{}”：{error}", command.keyword))
                })
        });
        if let Err(error) = validation {
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-custom-command-validation",
                    "无法保存自定义命令",
                    error,
                    cx,
                );
            });
            return false;
        }
        self.config.quick_launch.custom_commands = settings.custom_commands;
        self.persist_custom_commands(cx)
    }

    pub(crate) fn remove_custom_command(&mut self, index: usize, cx: &mut Context<Self>) -> bool {
        if index >= self.config.quick_launch.custom_commands.len() {
            return false;
        }
        self.config.quick_launch.custom_commands.remove(index);
        self.persist_custom_commands(cx)
    }

    fn persist_custom_commands(&mut self, cx: &mut Context<Self>) -> bool {
        if let Err(error) = self.save_quick_launch_section() {
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-custom-command-save",
                    "无法保存自定义命令",
                    error.to_string(),
                    cx,
                );
            });
            return false;
        }
        self.index_search.invalidate();
        cx.notify();
        true
    }

    fn persist_and_sync_quick_launch_behavior(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.save_quick_launch_section() {
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-context-settings",
                    "无法保存文件对话框设置",
                    error.to_string(),
                    cx,
                );
            });
            return;
        }
        self.send_configuration();
        cx.notify();
    }

    pub(crate) fn remove_excluded_path(&mut self, path: &str, cx: &mut Context<Self>) {
        self.config
            .quick_launch
            .excluded_paths
            .retain(|candidate| !candidate.eq_ignore_ascii_case(path));
        self.persist_and_sync_index_settings(cx);
    }

    pub(crate) fn request_index_exclusion_directory(
        &mut self,
        source_window_handle: i64,
        cx: &mut Context<Self>,
    ) {
        if self.pending_exclusion_picker.is_some() {
            return;
        }
        let attempt_correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        let current_directory = self
            .config
            .quick_launch
            .excluded_paths
            .last()
            .cloned()
            .unwrap_or_default();
        match self
            .platform
            .send_command(PlatformCommand::PickDirectory(PickDirectoryCommand {
                current_directory,
                title: "选择要从速启搜索中排除的目录".into(),
                source_window_handle,
                source_process_id: std::process::id(),
            })) {
            Ok(command_id) => {
                info!(
                    event = "quick_launch.index_exclusion_picker.started",
                    correlation_id = %command_id,
                    "quick-launch index exclusion picker started"
                );
                self.pending_exclusion_picker = Some(PendingIndexExclusionPicker {
                    correlation_id: command_id,
                    started_at,
                });
            }
            Err(error) => {
                warn!(
                    event = "quick_launch.index_exclusion_picker.completed",
                    correlation_id = %attempt_correlation_id,
                    outcome = "rejected",
                    elapsed_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
                    native_error_code = ?Option::<i32>::None,
                    error_reported = true,
                    "quick-launch index exclusion picker could not start"
                );
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-index-exclusion-picker",
                        "无法打开目录选择器",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
        cx.notify();
    }

    pub(crate) fn request_custom_command_program(
        &mut self,
        index: usize,
        current_path: String,
        source_window_handle: i64,
        cx: &mut Context<Self>,
    ) -> Option<uuid::Uuid> {
        if self.pending_program_picker.is_some()
            || index >= self.config.quick_launch.custom_commands.len()
        {
            return None;
        }
        let attempt_correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        match self
            .platform
            .send_command(PlatformCommand::PickProgram(PickProgramCommand {
                current_path,
                source_window_handle,
                source_process_id: std::process::id(),
            })) {
            Ok(command_id) => {
                info!(
                    event = "quick_launch.custom_command_program_picker.started",
                    correlation_id = %command_id,
                    "quick-launch custom-command program picker started"
                );
                self.pending_program_picker = Some(PendingProgramPicker {
                    correlation_id: command_id,
                    command_index: index,
                    started_at,
                });
                cx.notify();
                return Some(command_id);
            }
            Err(error) => {
                warn!(
                    event = "quick_launch.custom_command_program_picker.completed",
                    correlation_id = %attempt_correlation_id,
                    outcome = "rejected",
                    elapsed_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
                    native_error_code = ?Option::<i32>::None,
                    error_reported = true,
                    "quick-launch custom-command program picker could not start"
                );
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-command-program-picker",
                        "无法打开程序选择器",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
        cx.notify();
        None
    }

    fn persist_and_sync_index_settings(&mut self, cx: &mut Context<Self>) {
        self.config.quick_launch.normalize();
        if let Err(error) = self.save_quick_launch_section() {
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-index-settings",
                    "无法保存索引设置",
                    error.to_string(),
                    cx,
                );
            });
            return;
        }
        self.last_sent_index_settings = None;
        self.sync_index_settings();
        self.index_search.invalidate();
        cx.notify();
    }

    fn sync_index_settings(&mut self) {
        if !self.indexer.is_connected() {
            return;
        }
        let settings = index_settings_from_quick_launch(&self.config.quick_launch);
        if self.last_sent_index_settings.as_ref() == Some(&settings) {
            return;
        }
        match self
            .indexer
            .send_command(IndexerCommand::UpdateSettings(settings.clone()))
        {
            Ok(_) => self.last_sent_index_settings = Some(settings),
            Err(error) => warn!(%error, "could not synchronize quick-launch index settings"),
        }
    }

    pub(crate) fn handle_indexer_notification(
        &mut self,
        notification: IndexerNotification,
        cx: &mut Context<Self>,
    ) {
        let finished_generation = match &notification {
            IndexerNotification::Event(IndexerEvent::SearchBatch(batch)) if batch.complete => {
                Some(batch.generation)
            }
            IndexerNotification::Event(IndexerEvent::SearchFailed(failure)) => {
                Some(failure.generation)
            }
            _ => None,
        };
        if self
            .pending_index_search
            .as_ref()
            .is_some_and(|pending| Some(pending.generation) == finished_generation)
        {
            self.pending_index_search = None;
        }
        if let IndexerNotification::Event(IndexerEvent::SearchFailed(failure)) = &notification {
            if self.handle_file_dialog_search_failure(failure.generation, cx) {
                return;
            }
            if failure.generation == self.index_search.generation {
                self.index_search.complete = true;
                self.index_search.items.clear();
                self.index_search.failed = true;
                let query = self.index_search.query.clone();
                let providers_pending = self.extension_search_in_progress(&query);
                if providers_pending && let Some(popup) = self.popup.clone() {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        if panel.is_open
                            && panel.action_mode.is_none()
                            && panel.search_input.read(cx).value().as_ref() == query
                        {
                            panel.set_error(INDEX_SEARCH_FAILURE_MESSAGE.into(), cx);
                            panel.search_pending = true;
                        }
                    });
                }
                self.refresh_indexed_results(cx, false);
            }
            return;
        }
        if let IndexerNotification::Event(IndexerEvent::SearchBatch(batch)) = &notification
            && self.handle_file_dialog_search_batch(batch, cx)
        {
            return;
        }
        if matches!(&notification, IndexerNotification::Disconnected) {
            self.pending_index_search = None;
            self.handle_file_dialog_search_disconnect(cx);
        }
        match &notification {
            IndexerNotification::Event(IndexerEvent::Status(status))
                if index_status_affects_presentation(self.index_status.as_ref(), status) =>
            {
                // Count-only updates arrive at journal-poll frequency on a
                // live system; they coalesce into rate-limited records while
                // phase, error and drive-structure changes always log.
                let count_only = index_status_count_only_change(self.index_status.as_ref(), status);
                if should_log_index_status_change(
                    count_only,
                    self.last_count_status_log,
                    Instant::now(),
                ) {
                    info!(
                        phase = ?status.phase,
                        indexed_items = status.indexed_items,
                        count_only,
                        "quick-launch index provider status changed"
                    );
                    if count_only {
                        self.last_count_status_log = Some(Instant::now());
                    }
                }
            }
            IndexerNotification::Event(IndexerEvent::Status(_)) => {}
            IndexerNotification::Event(IndexerEvent::SearchFailed(_)) => {}
            IndexerNotification::Event(IndexerEvent::SearchBatch(batch)) => {
                if batch.generation == self.index_search.generation
                    && (self.index_search.items.is_empty() || batch.complete)
                {
                    info!(
                        generation = batch.generation,
                        first = self.index_search.items.is_empty() && !batch.items.is_empty(),
                        complete = batch.complete,
                        item_count = batch.items.len(),
                        provider_elapsed_us = batch.elapsed_us,
                        "quick-launch index provider milestone received"
                    );
                }
            }
            IndexerNotification::Event(IndexerEvent::ElevatedDirectoryHelper(status)) => {
                if status.launched {
                    info!("elevated directory helper launch was accepted");
                } else {
                    warn!(
                        reason = status.error.as_deref().unwrap_or("unknown"),
                        "elevated directory helper launch failed"
                    );
                }
            }
            IndexerNotification::Disconnected => {
                warn!("quick-launch index provider disconnected")
            }
        }
        if matches!(
            &notification,
            IndexerNotification::Event(IndexerEvent::Status(_))
        ) {
            self.sync_index_settings();
            self.sync_elevated_directory_helper();
        } else if let IndexerNotification::Event(IndexerEvent::ElevatedDirectoryHelper(status)) =
            &notification
        {
            let is_current = elevated_helper_status_matches_request(
                self.elevated_helper_request.as_ref(),
                status,
            );
            if is_current {
                self.pending_elevated_helper = None;
                self.acknowledged_elevated_helper_pipe = Some(status.pipe_name.clone());
            }
        } else if matches!(&notification, IndexerNotification::Disconnected) {
            self.last_sent_index_settings = None;
            if self.pending_elevated_helper.is_some() {
                self.pending_elevated_helper = None;
                self.acknowledged_elevated_helper_pipe = None;
            }
        }
        let retry_current_query = match &notification {
            IndexerNotification::Event(IndexerEvent::Status(status)) => {
                should_retry_index_search_after_status(self.index_status.as_ref(), status)
            }
            _ => false,
        };
        let refresh_results = matches!(
            &notification,
            IndexerNotification::Event(IndexerEvent::SearchBatch(batch))
                if batch.generation == self.index_search.generation
                    && batch.complete
        );
        let indexer_disconnected = matches!(&notification, IndexerNotification::Disconnected);
        if apply_indexer_notification(&mut self.index_status, &mut self.index_search, notification)
        {
            if retry_current_query {
                self.retry_index_search_after_ready(cx);
                self.retry_file_dialog_search_after_ready(cx);
            }
            if refresh_results {
                self.refresh_indexed_results(cx, false);
            }
            if indexer_disconnected {
                self.refresh_results_after_indexer_disconnect(cx);
            }
            cx.notify();
        }
    }

    fn retry_index_search_after_ready(&mut self, cx: &mut Context<Self>) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let panel_state = popup
            .panel
            .update(cx, |panel, cx| {
                (
                    panel.is_open,
                    panel.action_mode.is_none(),
                    panel.search_input.read(cx).value().to_string(),
                )
            })
            .ok();
        let Some((true, true, query)) = panel_state else {
            return;
        };
        if query.trim().is_empty() {
            return;
        }
        info!(
            query_length = query.chars().count(),
            "retrying visible quick-launch query after indexer became ready"
        );
        let update = self.search_and_request_icons(&query, cx);
        let _ = popup.panel.update(cx, |panel, cx| {
            panel.search_pending = update.pending;
            if update.pending {
                cx.notify();
            } else {
                panel.replace_results(update.results, cx);
            }
        });
    }

    pub(crate) fn hotkey_lifecycle(&self) -> QuickLaunchHotkeyLifecycle {
        if let Some((_, hotkey)) = &self.pending_hotkey {
            return QuickLaunchHotkeyLifecycle::Applying {
                candidate: Some(hotkey.display.clone()),
            };
        }
        if let Some(error) = &self.hotkey_error {
            return QuickLaunchHotkeyLifecycle::Failed {
                error: error.clone(),
            };
        }
        if self.hotkey_recording_active() {
            return if self.hotkey_recording_ready() {
                QuickLaunchHotkeyLifecycle::Recording
            } else {
                QuickLaunchHotkeyLifecycle::WaitingForRecording {
                    status: self.configuration_status.clone(),
                }
            };
        }
        if !self.pending_configuration.is_empty() {
            return QuickLaunchHotkeyLifecycle::Applying { candidate: None };
        }
        if self.configuration_status.starts_with("等待") {
            return QuickLaunchHotkeyLifecycle::WaitingForAdapter {
                status: self.configuration_status.clone(),
            };
        }
        if self.configuration_status.starts_with("应用失败") {
            return QuickLaunchHotkeyLifecycle::Failed {
                error: self.configuration_status.clone(),
            };
        }
        QuickLaunchHotkeyLifecycle::Applied
    }

    pub(crate) fn keyboard_health_warning(&self) -> Option<(String, bool)> {
        let health = self.keyboard_health;
        let settings = &self.config.quick_launch;
        if !settings.enabled
            || !settings.hotkey.modifiers.is_empty()
            || !settings.hotkey.key.eq_ignore_ascii_case("Control")
            || self.hotkey_recording_active()
        {
            return None;
        }
        let reason = match health {
            Some((QuickLaunchKeyboardHealthState::InstallFailed, code)) => {
                format!("Windows 键盘钩子安装失败（系统错误码 {code}）。可尝试重新启用。")
            }
            Some((QuickLaunchKeyboardHealthState::ThreadExited, code)) => {
                format!("Windows 键盘钩子线程已退出（系统错误码 {code}）。可尝试重新启用。")
            }
            _ if self.keyboard_capability_unavailable => {
                "Windows 适配器未提供双击 Ctrl 热键能力。可尝试重新启用。".to_owned()
            }
            _ => return None,
        };
        Some((
            reason,
            self.pending_keyboard_reenable.is_some() || !self.pending_configuration.is_empty(),
        ))
    }

    pub(crate) fn reenable_keyboard_hotkey(&mut self, cx: &mut Context<Self>) {
        if self.keyboard_health_warning().is_none()
            || self.pending_keyboard_reenable.is_some()
            || !self.pending_configuration.is_empty()
            || !self.platform.is_connected()
        {
            return;
        }
        let id = uuid::Uuid::new_v4();
        let started = Instant::now();
        self.pending_keyboard_reenable = Some((id, started, self.keyboard_health_revision));
        info!(event = "quick_launch.keyboard_reenable.started", correlation_id = %id,
            "requested Quick Launch keyboard hook reinstallation");
        let settings = self.config.quick_launch.clone();
        if self.send_configuration_for_with_id(&settings, id).is_none() {
            self.pending_keyboard_reenable = None;
            warn!(event = "quick_launch.keyboard_reenable.completed", correlation_id = %id,
                success = false, failure_kind = "enqueue_failed", elapsed_ms = started.elapsed().as_millis() as u64,
                "Quick Launch keyboard re-enable was not queued");
        }
        cx.notify();
    }

    fn handle_keyboard_health(
        &mut self,
        event: QuickLaunchKeyboardHealthChanged,
        cx: &mut Context<Self>,
    ) {
        self.keyboard_health_revision = self.keyboard_health_revision.wrapping_add(1);
        let unavailable = event.state != QuickLaunchKeyboardHealthState::Reinstalled;
        self.keyboard_health = unavailable.then_some((event.state, event.native_error_code));
        if !unavailable {
            self.keyboard_capability_unavailable = false;
        }
        info!(event = "quick_launch.keyboard_health.changed", correlation_id = %event.correlation_id,
            state = ?event.state, native_error_code = event.native_error_code,
            "observed Quick Launch keyboard hook lifecycle");
        if !unavailable {
            self.keyboard_health_notified = false;
        }
        self.notify_keyboard_health_if_visible(cx);
        cx.notify();
    }

    fn notify_keyboard_health_if_visible(&mut self, cx: &mut Context<Self>) {
        if self.keyboard_health_notified {
            return;
        }
        let Some((reason, _)) = self.keyboard_health_warning() else {
            return;
        };
        self.keyboard_health_notified = true;
        let notifications = self.notifications.clone();
        cx.defer(move |cx| {
            notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-keyboard-health",
                    "双击 Ctrl 已失效",
                    reason,
                    cx,
                );
            });
        });
    }

    pub(crate) fn hotkey_recording_ready(&self) -> bool {
        self.recording_active && self.pending_recording.is_none()
    }

    pub(crate) fn hotkey_recording_active(&self) -> bool {
        self.recording_active || self.recording_desired || self.pending_recording.is_some()
    }

    pub(crate) fn set_hotkey_recording(&mut self, recording: bool) {
        if recording {
            self.hotkey_error = None;
        }
        self.recording_desired = recording;
        self.drive_hotkey_recording();
    }

    pub(crate) fn set_hotkey(&mut self, hotkey: ClipboardHotKey) {
        if self.pending_hotkey.is_some() || !self.hotkey_recording_ready() {
            return;
        }
        self.recording_desired = false;
        self.recording_active = false;
        self.apply_hotkey(hotkey);
    }

    pub(crate) fn hotkey_is_custom(&self) -> bool {
        let default = QuickLaunchSettings::default().hotkey;
        self.config.quick_launch.hotkey.modifiers != default.modifiers
            || !self
                .config
                .quick_launch
                .hotkey
                .key
                .eq_ignore_ascii_case(&default.key)
    }

    pub(crate) fn can_reset_hotkey(&self) -> bool {
        self.hotkey_is_custom()
            && self.platform.is_connected()
            && self.pending_configuration.is_empty()
            && !self.hotkey_recording_active()
    }

    pub(crate) fn reset_hotkey(&mut self) {
        if self.can_reset_hotkey() {
            self.apply_hotkey(QuickLaunchSettings::default().hotkey);
        }
    }

    fn apply_hotkey(&mut self, hotkey: ClipboardHotKey) {
        self.hotkey_error = None;
        let mut settings = self.config.quick_launch.clone();
        settings.hotkey = hotkey.clone();
        if let Some(command_id) = self.send_configuration_for(&settings) {
            self.pending_hotkey = Some((command_id, hotkey));
            info!(event = "quick_launch.hotkey.submitted", correlation_id = %command_id,
                "quick-launch binding submitted through original configuration owner");
        } else {
            self.hotkey_error = Some(self.configuration_status.clone());
        }
    }

    pub(crate) fn handle_platform_notification(
        &mut self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        match notification {
            PlatformNotification::Connected => {
                if let Some(popup) = &self.popup {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        panel.result_motion_available = true;
                        cx.notify();
                    });
                }
                self.pending_keyboard_reenable = None;
                self.keyboard_health = None;
                self.keyboard_capability_unavailable =
                    self.platform.quick_launch_panel_hotkey_available() == Some(false);
                self.keyboard_health_notified = false;
                self.notify_keyboard_health_if_visible(cx);
                let prepare_indexer = self.indexer_service_bootstrap.on_platform_connected();
                self.send_configuration();
                self.request_installed_applications(cx);
                if prepare_indexer && should_manage_indexer_service() {
                    self.ensure_indexer_service(true, cx);
                }
                if let Some((id, _)) = self.latest_configuration {
                    self.schedule_feature_configuration_timeout(id, cx);
                }
                cx.notify();
            }
            PlatformNotification::Disconnected {
                generation,
                pending_command_ids,
            } => {
                self.disconnect_popup_presentation(generation, cx);
                self.retire_installed_applications("disconnected");
                warn!(
                    event = "quick_launch.platform_disconnected",
                    generation,
                    pending_command_count = pending_command_ids.len(),
                    "quick-launch invalidated commands from the disconnected PlatformHost session"
                );
                if let Some(popup) = &self.popup {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        panel.result_motion_available = false;
                        panel.finish_result_entrances("disconnected", cx);
                        cx.notify();
                    });
                }
                self.pending_configuration.clear();
                if let Some((id, started)) = self.latest_configuration.take() {
                    warn!(event = "quick_launch.configuration.failed", correlation_id = %id,
                        stage = "disconnected", elapsed_ms = started.elapsed().as_millis());
                }
                self.pending_hotkey = None;
                if let Some((correlation_id, started, _revision)) =
                    self.pending_keyboard_reenable.take()
                {
                    warn!(event = "quick_launch.keyboard_reenable.completed", %correlation_id,
                        success = false, failure_kind = "disconnected",
                        elapsed_ms = started.elapsed().as_millis() as u64,
                        "Quick Launch keyboard re-enable ended with PlatformHost session");
                }
                self.keyboard_health = None;
                self.keyboard_capability_unavailable = false;
                self.keyboard_health_notified = false;
                self.pending_recording = None;
                self.recording_active = false;
                self.recording_desired = false;
                self.hotkey_error = Some("等待 Windows 适配器".into());
                let current_session_id = self.popup_session_id;
                let (disconnected_current_actions, stale_action_count) =
                    partition_actions_for_disconnect(
                        std::mem::take(&mut self.pending_actions),
                        current_session_id,
                    );
                if stale_action_count > 0 {
                    warn!(
                        event = "quick_launch.action.disconnected_stale_discarded",
                        generation,
                        stale_action_count,
                        "discarded accepted actions from an older popup session"
                    );
                }
                if !disconnected_current_actions.is_empty() {
                    let action_count = disconnected_current_actions.len();
                    let action_label = disconnected_current_actions
                        .first()
                        .map(|pending| quick_launch_action_label(&pending.action))
                        .unwrap_or("速启动作");
                    for pending in &disconnected_current_actions {
                        error!(
                            event = "quick_launch.action.completed",
                            correlation_id = %pending.correlation_id,
                            action = quick_launch_action_diagnostic_label(&pending.action),
                            trigger = pending.trigger.diagnostic_label(),
                            source = pending.source.label(),
                            outcome = "failure",
                            failure_kind = "platform_disconnected",
                            elapsed_ms = pending.started_at.elapsed().as_millis() as u64,
                            error_reported = true,
                            "quick-launch action failed because PlatformHost disconnected"
                        );
                    }
                    let notifications = self.notifications.clone();
                    let message = if action_count == 1 {
                        format!(
                            "{action_label}未完成，Windows 适配器已断开；重新呼出速启后可重试。"
                        )
                    } else {
                        format!(
                            "{action_count} 个速启动作未完成，Windows 适配器已断开；重新呼出速启后可重试。"
                        )
                    };
                    cx.defer(move |cx| {
                        notifications.update(cx, |notifications, cx| {
                            notifications.show_error(
                                "quick-launch-action-disconnected",
                                "速启动作未完成",
                                message,
                                cx,
                            );
                        });
                    });
                }
                self.pending_auto_quick_switches.clear();
                self.navigation_foreground_guard = None;
                self.pending_shell_queries.clear();
                self.pending_window_activation.clear();
                self.pending_panel_hides.clear();
                if let Some(pending) = self.pending_popup_resize.take() {
                    warn!(event = "quick_launch.popup.resize.completed", correlation_id = %pending.command_id,
                        outcome = "disconnected", elapsed_ms = pending.started_at.elapsed().as_millis() as u64,
                        "quick-launch content resize interrupted");
                }
                self.pending_icons.clear();
                self.pending_icon_resources.clear();
                self.pending_exclusion_picker = None;
                self.pending_program_picker = None;
                self.elevated_helper_request = None;
                self.pending_elevated_helper = None;
                self.acknowledged_elevated_helper_pipe = None;
                self.configuration_status = "等待 Windows 适配器".into();
                self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
                self.preview.hovered_entry_id = None;
                self.preview.pointer_inside = false;
                self.hide_quick_launch_preview_window(cx);
                cx.notify();
            }
            PlatformNotification::Event(event) => self.handle_platform_event(event, cx),
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => self.handle_command_result(correlation_id, result, cx),
        }
    }

    fn handle_platform_event(&mut self, event: PlatformEvent, cx: &mut Context<Self>) {
        match event {
            PlatformEvent::ElevatedDirectoryHelperRequested(request) => {
                self.elevated_helper_request = Some(request);
                self.pending_elevated_helper = None;
                self.acknowledged_elevated_helper_pipe = None;
                self.sync_elevated_directory_helper();
            }
            PlatformEvent::QuickLaunchKeyboardHealthChanged(event) => {
                self.handle_keyboard_health(event, cx);
            }
            PlatformEvent::QuickLaunchPanelRequested(request) => {
                let age = nxtools_ipc::unix_time_ms().saturating_sub(request.requested_at_unix_ms);
                if self.admits_feature_event(request.requested_at_unix_ms)
                    && age <= REQUEST_MAX_AGE_MS
                    && !self.hotkey_recording_active()
                    && !self.is_process_excluded(&request.target.process_name)
                {
                    self.toggle_panel(request, cx);
                }
            }
            PlatformEvent::QuickLaunchDirectoryContextResolved(resolution) => {
                self.apply_resolved_directory_context(resolution, cx);
            }
            PlatformEvent::QuickLaunchAutoSwitchRequested(request) => {
                self.handle_auto_quick_switch_request(request);
            }
            PlatformEvent::QuickLaunchFileDialogSearchHostChanged(host) => {
                self.handle_file_dialog_search_host(host, cx);
            }
            PlatformEvent::ForegroundChanged(foreground) => {
                let pending_navigation_owns_foreground =
                    self.pending_actions.values().any(|pending| {
                        pending_action_owns_foreground(
                            pending,
                            self.popup_session_id,
                            foreground.window_handle,
                        )
                    });
                if pending_navigation_owns_foreground
                    || navigation_guard_owns_foreground(
                        &mut self.navigation_foreground_guard,
                        self.popup_session_id,
                        foreground.window_handle,
                        Instant::now(),
                    )
                {
                    return;
                }
                let Some(popup) = self.popup.clone() else {
                    return;
                };
                let preview_window_handle = self
                    .preview
                    .window
                    .as_ref()
                    .map_or(0, |preview| preview.identity.native);
                let should_hide = popup
                    .window
                    .update(cx, |_, _window, app| {
                        popup.panel.update(app, |panel, _| {
                            panel.should_dismiss_for_foreground(
                                popup.native_window_handle,
                                preview_window_handle,
                                foreground.window_handle,
                            )
                        })
                    })
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or(false);
                if should_hide {
                    info!(
                        dismissal_source = "foreground_changed",
                        popup_window_handle = popup.native_window_handle,
                        preview_window_handle,
                        foreground_window_handle = foreground.window_handle,
                        foreground_process_id = foreground.process_id,
                        foreground_process_name = %foreground.process_name,
                        foreground_window_title = %foreground.window_title,
                        "quick-launch popup auto-dismissed"
                    );
                    self.hide_popup(cx);
                }
            }
            PlatformEvent::GlobalPointerPressed(pointer) => {
                let Some(popup) = self.popup.clone() else {
                    return;
                };
                if pointer.root_window_handle == popup.native_window_handle
                    && pointer.process_id == std::process::id()
                {
                    self.suspend_quick_launch_preview_for_scroll(cx);
                }
                let should_hide = popup
                    .window
                    .update(cx, |_, _window, app| {
                        popup.panel.update(app, |panel, _| {
                            panel.should_dismiss_for_pointer(
                                popup.native_window_handle,
                                pointer.root_window_handle,
                                pointer.process_id,
                            )
                        })
                    })
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or(false);
                if should_hide {
                    info!(
                        dismissal_source = "global_pointer_pressed",
                        popup_window_handle = popup.native_window_handle,
                        pointer_root_window_handle = pointer.root_window_handle,
                        pointer_process_id = pointer.process_id,
                        desktop_process_id = std::process::id(),
                        "quick-launch popup auto-dismissed"
                    );
                    self.hide_popup(cx);
                }
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
            | PlatformEvent::SystemMaterialCapabilitiesChanged(_)
            | PlatformEvent::ClipboardCaptured(_)
            | PlatformEvent::ClipboardPanelRequested(_)
            | PlatformEvent::TaskbarOverlayEditGestureFinished(_) => {}
        }
    }

    fn handle_auto_quick_switch_request(
        &mut self,
        request: nxtools_protocol::QuickLaunchAutoSwitchRequest,
    ) {
        if !self.admits_feature_event(request.requested_at_unix_ms) {
            return;
        }
        let Some(command) = auto_quick_switch_command(
            &self.config.quick_launch,
            &request,
            nxtools_ipc::unix_time_ms(),
        ) else {
            return;
        };
        match self
            .platform
            .send_command(PlatformCommand::NavigateDirectory(command))
        {
            Ok(correlation_id) => {
                self.pending_auto_quick_switches.insert(
                    correlation_id,
                    (
                        request.target.window_handle,
                        request.source_directory.clone(),
                    ),
                );
            }
            Err(error) => {
                warn!(
                    %error,
                    target_window_handle = request.target.window_handle,
                    source_directory_present = !request.source_directory.is_empty(),
                    "could not queue automatic file-dialog directory switch"
                );
            }
        }
    }

    fn sync_elevated_directory_helper(&mut self) {
        let Some(command) = elevated_directory_helper_command(
            self.elevated_helper_request.as_ref(),
            self.pending_elevated_helper.is_some(),
            self.acknowledged_elevated_helper_pipe.as_deref(),
            self.indexer.is_connected(),
        ) else {
            return;
        };
        match self.indexer.send_command(command) {
            Ok(command_id) => self.pending_elevated_helper = Some(command_id),
            Err(error) => {
                warn!(%error, "could not request the elevated directory helper");
            }
        }
    }

    fn apply_resolved_directory_context(
        &mut self,
        resolution: nxtools_protocol::QuickLaunchDirectoryContextResolution,
        cx: &mut Context<Self>,
    ) {
        if !directory_context_resolution_matches(
            self.active_summon,
            &resolution,
            nxtools_ipc::unix_time_ms(),
        ) {
            return;
        }
        let recent_directories = self
            .catalog
            .borrow()
            .recent_directories(RECENT_DIRECTORY_LIMIT);
        let context = resolution.directory_navigation_context;
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let applied = popup
            .panel
            .update(cx, |panel, cx| {
                if !panel.is_open || panel.target.window_handle != resolution.target_window_handle {
                    return false;
                }
                panel.navigation_context = Some(context.clone());
                panel.recent_directories = recent_directories;
                cx.notify();
                true
            })
            .unwrap_or(false);
        if applied {
            self.schedule_captured_directory_record(Some(&context), cx);
        }
    }

    fn resize_popup_content(&mut self, cx: &mut Context<Self>) {
        if self.pending_popup_resize.is_some() {
            return;
        }
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let Some((x, y)) = self.popup_origin_device_pixels else {
            return;
        };
        let command = popup
            .window
            .update(cx, |_, window, cx| {
                let panel = popup.panel.upgrade()?;
                let panel = panel.read(cx);
                if !panel.is_open {
                    return None;
                }
                let desired = panel.desired_content_size(window, cx);
                if window.bounds().size == desired {
                    return None;
                }
                let bounds = Bounds::new(window.bounds().origin, desired)
                    .to_device_pixels(window.scale_factor());
                Some(SetWindowBoundsCommand {
                    window_handle: popup.native_window_handle,
                    x,
                    y,
                    width: bounds.size.width.into(),
                    height: bounds.size.height.into(),
                })
            })
            .ok()
            .flatten();
        let Some(command) = command else {
            return;
        };
        let command_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        self.pending_popup_resize = Some(PendingPopupResize {
            command_id,
            started_at,
        });
        match self
            .platform
            .send_command_with_id(PlatformCommand::SetWindowBounds(command), command_id)
        {
            Ok(_) => info!(
                event = "quick_launch.popup.resize.accepted",
                correlation_id = %command_id,
                "quick-launch content resize accepted"
            ),
            Err(error) => {
                self.pending_popup_resize = None;
                warn!(
                    event = "quick_launch.popup.resize.rejected",
                    correlation_id = %command_id,
                    elapsed_ms = started_at.elapsed().as_millis() as u64,
                    %error,
                    "quick-launch content resize rejected"
                );
            }
        }
    }

    fn handle_command_result(
        &mut self,
        correlation_id: uuid::Uuid,
        result: PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        if self.installed_applications_result(correlation_id, &result, cx) {
            return;
        }
        if self.popup_presentation_result(correlation_id, &result, cx) {
            return;
        }
        if self
            .pending_popup_resize
            .as_ref()
            .is_some_and(|pending| pending.command_id == correlation_id)
        {
            let pending = self
                .pending_popup_resize
                .take()
                .expect("matching popup resize");
            info!(
                event = "quick_launch.popup.resize.completed",
                correlation_id = %correlation_id,
                success = result.success,
                elapsed_ms = pending.started_at.elapsed().as_millis() as u64,
                native_error_code = ?result.native_error_code,
                "quick-launch content resize completed"
            );
            if result.success {
                // A provider may have published again while Windows applied the
                // previous size. Re-read the live surface, never replay old bounds.
                self.resize_popup_content(cx);
            }
            return;
        }
        if let Some(pending) = self.pending_panel_hides.remove(&correlation_id) {
            let elapsed_ms = pending.requested_at.elapsed().as_millis() as u64;
            if result.success {
                info!(
                    event = "quick_launch.panel.hide_applied",
                    popup_window_handle = pending.popup_window_handle,
                    attempt = pending.attempt,
                    elapsed_ms,
                    "quick-launch panel hide confirmed by Windows"
                );
            } else if pending.attempt < PANEL_HIDE_RETRY_LIMIT {
                warn!(
                    event = "quick_launch.panel.hide_rejected",
                    popup_window_handle = pending.popup_window_handle,
                    reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                    "quick-launch panel hide rejected; retrying once"
                );
                self.resend_panel_hide(pending);
            } else {
                error!(
                    event = "quick_launch.panel.hide_failed",
                    popup_window_handle = pending.popup_window_handle,
                    attempt = pending.attempt,
                    reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                    "quick-launch panel hide failed after retry"
                );
            }
            return;
        }
        if self.handle_preview_material_result(correlation_id, &result, cx) {
            return;
        }
        if self.handle_quick_launch_preview_stage_result(
            correlation_id,
            result.success,
            result.error.as_deref(),
            cx,
        ) {
            return;
        }
        if let Some((target_window_handle, source_directory)) =
            self.pending_auto_quick_switches.remove(&correlation_id)
        {
            if result.success {
                self.update_file_dialog_search_directory(
                    target_window_handle,
                    source_directory,
                    cx,
                );
            } else {
                warn!(
                    reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                    "automatic file-dialog directory switch failed"
                );
            }
            return;
        }
        if let Some(pending) = self.pending_shell_queries.remove(&correlation_id) {
            let menu = if result.success {
                result.data.as_deref().and_then(|data| {
                    serde_json::from_str::<ShellActionMenu>(data)
                        .map_err(|error| {
                            warn!(%error, "could not decode Windows Shell action menu");
                            error
                        })
                        .ok()
                })
            } else {
                warn!(
                    reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                    "could not enumerate Windows Shell actions"
                );
                None
            };
            self.apply_shell_actions(
                pending.mode_id,
                pending.level_id,
                &pending.entry_id,
                menu,
                cx,
            );
            return;
        }
        if self
            .pending_exclusion_picker
            .is_some_and(|pending| pending.correlation_id == correlation_id)
        {
            let pending = self
                .pending_exclusion_picker
                .take()
                .expect("matching exclusion picker must exist");
            let elapsed_ms =
                u64::try_from(pending.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
            if result.success {
                let outcome = if result
                    .data
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|path| !path.is_empty())
                {
                    "selected"
                } else {
                    "cancelled"
                };
                info!(
                    event = "quick_launch.index_exclusion_picker.completed",
                    correlation_id = %pending.correlation_id,
                    outcome,
                    elapsed_ms,
                    native_error_code = ?result.native_error_code,
                    "quick-launch index exclusion picker completed"
                );
                if let Some(path) = result.data.as_deref().map(str::trim)
                    && !path.is_empty()
                    && !self
                        .config
                        .quick_launch
                        .excluded_paths
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(path))
                {
                    self.config.quick_launch.excluded_paths.push(path.into());
                    self.persist_and_sync_index_settings(cx);
                }
            } else {
                error!(
                    event = "quick_launch.index_exclusion_picker.completed",
                    correlation_id = %pending.correlation_id,
                    outcome = "failure",
                    elapsed_ms,
                    native_error_code = ?result.native_error_code,
                    error_reported = result.error.is_some(),
                    "quick-launch index exclusion picker failed"
                );
                let reason = result
                    .error
                    .unwrap_or_else(|| "Windows 未返回错误详情".into());
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-index-exclusion-picker",
                        "无法选择排除目录",
                        reason,
                        cx,
                    );
                });
            }
            cx.notify();
            return;
        }
        if self
            .pending_program_picker
            .as_ref()
            .is_some_and(|pending| pending.correlation_id == correlation_id)
        {
            let pending = self
                .pending_program_picker
                .take()
                .expect("matching program picker must exist");
            let elapsed_ms =
                u64::try_from(pending.started_at.elapsed().as_millis()).unwrap_or(u64::MAX);
            if result.success {
                let outcome = if result
                    .data
                    .as_deref()
                    .map(str::trim)
                    .is_some_and(|path| !path.is_empty())
                {
                    "selected"
                } else {
                    "cancelled"
                };
                info!(
                    event = "quick_launch.custom_command_program_picker.completed",
                    correlation_id = %pending.correlation_id,
                    outcome,
                    elapsed_ms,
                    native_error_code = ?result.native_error_code,
                    "quick-launch custom-command program picker completed"
                );
                if let Some(path) = result.data.filter(|path| !path.trim().is_empty()) {
                    cx.emit(QuickLaunchApplicationEvent::CustomCommandProgramPicked {
                        correlation_id: pending.correlation_id,
                        index: pending.command_index,
                        path,
                    });
                }
            } else {
                error!(
                    event = "quick_launch.custom_command_program_picker.completed",
                    correlation_id = %pending.correlation_id,
                    outcome = "failure",
                    elapsed_ms,
                    native_error_code = ?result.native_error_code,
                    error_reported = result.error.is_some(),
                    "quick-launch custom-command program picker failed"
                );
                let reason = result
                    .error
                    .unwrap_or_else(|| "Windows 未返回错误详情".into());
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "quick-launch-command-program-picker",
                        "无法选择命令程序",
                        reason,
                        cx,
                    );
                });
            }
            cx.notify();
            return;
        }
        if let Some(automatic) = self.indexer_service_bootstrap.finish(correlation_id) {
            if indexer_deployment_requires_protected_restart(&result) {
                match crate::launch_protected_desktop_after_current_exit() {
                    Ok(true) => {
                        info!("restarting the desktop from the protected service package");
                        cx.quit();
                        return;
                    }
                    Ok(false) => {}
                    Err(error) => {
                        warn!(
                            %error,
                            "could not restart the protected desktop after service installation"
                        );
                    }
                }
            }
            if automatic && result.success && result.data.as_deref() == Some("running") {
                return;
            }
            let reason = result
                .error
                .clone()
                .unwrap_or_else(|| "Windows 未返回错误详情".into());
            self.notifications.update(cx, move |notifications, cx| {
                if result.success {
                    notifications.show_success(
                        "quick-launch-indexer-service",
                        "索引服务已就绪",
                        "正在连接已授权版本的索引服务；现有快照由服务恢复。",
                        cx,
                    );
                } else {
                    notifications.show_error(
                        "quick-launch-indexer-service",
                        "索引服务暂不可用",
                        reason,
                        cx,
                    );
                }
            });
            cx.notify();
            return;
        }
        if let Some(pending) = self.pending_window_activation.remove(&correlation_id) {
            self.handle_window_activation_result(correlation_id, pending, &result, cx);
            return;
        }
        if let Some(pending) = self.pending_icons.remove(&correlation_id) {
            let PendingIconResource {
                resource_key,
                entry_ids,
            } = pending;
            self.pending_icon_resources.remove(&resource_key);
            let icon_path = result
                .success
                .then_some(result.data.as_deref())
                .flatten()
                .map(PathBuf::from)
                .filter(|path| shell_icon_path_is_renderable(path));
            if let Some(path) = icon_path {
                self.failed_icon_resources.remove(&resource_key);
                self.icon_resources.insert(resource_key, path.clone());
                let icons = entry_ids
                    .into_iter()
                    .map(|entry_id| (entry_id, path.clone()))
                    .collect::<Vec<_>>();
                self.deliver_file_dialog_search_icons(icons.clone(), cx);
                if let Some(popup) = self.popup.clone() {
                    let _ = popup
                        .panel
                        .update(cx, |panel, cx| panel.set_icons(icons, cx));
                }
            } else {
                self.record_icon_failure(resource_key, cx);
            }
            return;
        }
        if self
            .pending_keyboard_reenable
            .as_ref()
            .is_some_and(|(id, _, _)| *id == correlation_id)
        {
            let (_, started, revision) = self
                .pending_keyboard_reenable
                .take()
                .expect("matching retry");
            if revision == self.keyboard_health_revision {
                if result.success {
                    self.keyboard_health = None;
                    self.keyboard_capability_unavailable = false;
                    self.keyboard_health_notified = false;
                } else if let Some((_, code)) = &mut self.keyboard_health {
                    *code = result.native_error_code.unwrap_or(*code);
                }
            }
            info!(event = "quick_launch.keyboard_reenable.completed", %correlation_id,
                success = result.success, native_error_code = result.native_error_code,
                elapsed_ms = started.elapsed().as_millis() as u64,
                "matching Quick Launch keyboard re-enable result");
            cx.notify();
        }
        if self.pending_configuration.remove(&correlation_id)
            && self
                .latest_configuration
                .is_some_and(|(id, _)| id == correlation_id)
        {
            let (_, started) = self
                .latest_configuration
                .take()
                .expect("matching configuration");
            info!(event = "quick_launch.configuration.completed", %correlation_id,
                enabled = self.feature_enabled(), success = result.success,
                native_error_code = result.native_error_code, elapsed_ms = started.elapsed().as_millis());
            if result.success && self.keyboard_health.is_none() {
                self.keyboard_capability_unavailable = false;
                self.keyboard_health_notified = false;
            }
            self.configuration_status = if result.success {
                if self.feature_enabled() {
                    "已启用"
                } else {
                    "已停用"
                }
                .into()
            } else {
                format!(
                    "应用失败：{}",
                    result.error.as_deref().unwrap_or("未知错误")
                )
            };
            cx.notify();
        }
        if self
            .pending_recording
            .as_ref()
            .is_some_and(|(command_id, _)| *command_id == correlation_id)
        {
            let Some((_, recording)) = self.pending_recording.take() else {
                return;
            };
            self.recording_active = result.success && recording;
            if !result.success {
                self.recording_desired = false;
                self.configuration_status = format!(
                    "热键录制失败：{}",
                    result.error.as_deref().unwrap_or("未知错误")
                );
                self.hotkey_error = Some(self.configuration_status.clone());
            }
            self.drive_hotkey_recording();
            self.notify_keyboard_health_if_visible(cx);
            cx.notify();
        }
        if self
            .pending_hotkey
            .as_ref()
            .is_some_and(|(command_id, _)| *command_id == correlation_id)
        {
            let Some((_, hotkey)) = self.pending_hotkey.take() else {
                return;
            };
            info!(event = "quick_launch.hotkey.completed", %correlation_id,
                success = result.success, native_error_code = result.native_error_code,
                "quick-launch binding received matching Windows result");
            if result.success {
                self.config.quick_launch.hotkey = hotkey;
                if let Err(error) = self.save_quick_launch_section() {
                    self.configuration_status = format!("热键已生效，但保存失败：{error}");
                    self.hotkey_error = Some(self.configuration_status.clone());
                } else {
                    self.configuration_status = if self.feature_enabled() {
                        "已启用"
                    } else {
                        "已停用"
                    }
                    .into();
                    self.hotkey_error = None;
                }
            } else {
                self.hotkey_error = Some(format!(
                    "热键应用失败：{}",
                    result.error.as_deref().unwrap_or("未知错误")
                ));
            }
            self.notify_keyboard_health_if_visible(cx);
            cx.notify();
        }
        let Some(pending_action) = self.pending_actions.remove(&correlation_id) else {
            return;
        };
        let elapsed_ms = pending_action
            .started_at
            .elapsed()
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let action_kind = quick_launch_action_diagnostic_label(&pending_action.action);
        let action_correlation_id = pending_action.correlation_id;
        let trigger = pending_action.trigger.diagnostic_label();
        let source = pending_action.source.label();
        if result.success {
            info!(
                event = "quick_launch.action.completed",
                correlation_id = %action_correlation_id,
                action = action_kind,
                trigger,
                source,
                outcome = "success",
                elapsed_ms,
                native_error_code = ?result.native_error_code,
                "quick-launch action completed"
            );
        } else {
            error!(
                event = "quick_launch.action.completed",
                correlation_id = %action_correlation_id,
                action = action_kind,
                trigger,
                source,
                outcome = "failure",
                elapsed_ms,
                native_error_code = ?result.native_error_code,
                error_reported = result.error.is_some(),
                "quick-launch action failed"
            );
        }
        let targets_current_session =
            pending_action_targets_session(&pending_action, self.popup_session_id);
        if result.success {
            match action_success_policy(&pending_action.action) {
                ActionSuccessPolicy::RecordAndHide => {
                    let activated_at_unix_ms = nxtools_ipc::unix_time_ms();
                    {
                        let mut catalog = self.catalog.borrow_mut();
                        if let Err(error) = catalog.record_activation(
                            &pending_action.entry_id,
                            &pending_action.query,
                            activated_at_unix_ms,
                        ) {
                            warn!(%error, "failed to persist quick-launch activation");
                        }
                    }
                    if let Some(directory) = pending_action.directory_visit {
                        self.schedule_directory_record(
                            directory,
                            activated_at_unix_ms,
                            pending_action.popup_session_id,
                            cx,
                        );
                    }
                    if targets_current_session {
                        self.hide_popup(cx);
                    }
                }
                ActionSuccessPolicy::HideWithoutRecording => {
                    if targets_current_session {
                        self.hide_popup(cx);
                    }
                }
                ActionSuccessPolicy::KeepOpen => {
                    if targets_current_session && let Some(popup) = self.popup.clone() {
                        let message = format!("已复制路径：{}", pending_action.target);
                        let _ = popup.window.update(cx, |_, window, app| {
                            let _ = popup.panel.update(app, |panel, cx| {
                                panel.set_status(message, cx);
                                panel.focus_search(window, cx);
                            });
                        });
                    }
                }
            }
        } else {
            let reason = result
                .error
                .unwrap_or_else(|| "Windows 未返回错误详情".into());
            if targets_current_session && let Some(popup) = self.popup.clone() {
                if let QuickLaunchAction::SwitchDirectory { window_handle, .. } =
                    &pending_action.action
                {
                    self.navigation_foreground_guard = Some(NavigationForegroundGuard {
                        popup_session_id: self.popup_session_id,
                        window_handle: *window_handle,
                        expires_at: Instant::now() + FAILED_NAVIGATION_FOREGROUND_GUARD,
                    });
                }
                self.begin_window_activation(popup.native_window_handle, cx);
                let _ = popup.window.update(cx, |_, window, app| {
                    let _ = popup.panel.update(app, |panel, cx| {
                        panel.notify_operation(
                            crate::notifications::NotificationLevel::Error,
                            format!(
                                "{} {} 失败：{reason}",
                                quick_launch_action_label(&pending_action.action),
                                pending_action.display_name
                            ),
                            cx,
                        );
                        panel.focus_search(window, cx);
                    });
                });
            }
        }
    }

    fn handle_window_activation_result(
        &mut self,
        correlation_id: uuid::Uuid,
        pending: PendingWindowActivation,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        if self
            .popup
            .as_ref()
            .is_none_or(|popup| popup.native_window_handle != pending.window_handle)
        {
            return;
        }

        let (stage, attempt) = match pending.step {
            QuickLaunchWindowActivationStep::PreTopmost(attempt) => ("pre_topmost", attempt),
            QuickLaunchWindowActivationStep::Activate(attempt) => ("activate", attempt),
            QuickLaunchWindowActivationStep::ConfirmTopmost(attempt) => {
                ("confirm_topmost", attempt)
            }
        };
        info!(
            event = "quick_launch.window_activation.step_completed",
            %correlation_id,
            window_handle = pending.window_handle,
            stage,
            attempt,
            success = result.success,
            native_error_code = ?result.native_error_code,
            "quick-launch window activation step completed"
        );

        if should_focus_search_after_activation(pending.step, result.success)
            && let Some(popup) = self.popup.clone()
        {
            let _ = popup.window.update(cx, |_, window, app| {
                let _ = popup
                    .panel
                    .update(app, |panel, cx| panel.focus_search(window, cx));
            });
        }

        let Some(next) = next_window_activation_step(pending.step, result.success) else {
            match pending.step {
                QuickLaunchWindowActivationStep::Activate(_) => {
                    warn!(
                        %correlation_id,
                        window_handle = pending.window_handle,
                        reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                        native_error_code = ?result.native_error_code,
                        "quick-launch popup activation failed after bounded retries"
                    );
                    self.notify_window_activation_failure(cx);
                }
                QuickLaunchWindowActivationStep::ConfirmTopmost(_) if result.success => {
                    info!(
                        %correlation_id,
                        window_handle = pending.window_handle,
                        "quick-launch popup activation and topmost confirmed"
                    );
                }
                QuickLaunchWindowActivationStep::ConfirmTopmost(_) => {
                    warn!(
                        %correlation_id,
                        window_handle = pending.window_handle,
                        reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                        "quick-launch popup stayed open after bounded topmost retries were exhausted"
                    );
                }
                QuickLaunchWindowActivationStep::PreTopmost(_) => {}
            }
            return;
        };
        self.queue_window_activation_step(pending.window_handle, next, cx);
    }

    fn notify_window_activation_failure(&self, cx: &mut Context<Self>) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let _ = popup.window.update(cx, |_, _, app| {
            let _ = popup.panel.update(app, |panel, cx| {
                panel.notify_operation(
                    crate::notifications::NotificationLevel::Error,
                    "无法激活快速启动窗口，请点击搜索框后重试。".into(),
                    cx,
                );
            });
        });
    }

    fn begin_window_activation(&mut self, window_handle: i64, cx: &mut Context<Self>) {
        self.pending_window_activation.clear();
        if window_handle == 0 {
            warn!("quick-launch popup opened without a native Windows handle");
            return;
        }
        self.queue_window_activation_step(
            window_handle,
            QuickLaunchWindowActivationStep::PreTopmost(0),
            cx,
        );
    }

    fn queue_window_activation_step(
        &mut self,
        window_handle: i64,
        initial_step: QuickLaunchWindowActivationStep,
        cx: &mut Context<Self>,
    ) {
        let mut step = Some(initial_step);
        while let Some(current) = step {
            let command = match current {
                QuickLaunchWindowActivationStep::PreTopmost(_)
                | QuickLaunchWindowActivationStep::ConfirmTopmost(_) => {
                    PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                        window_handle,
                        enabled: true,
                    })
                }
                QuickLaunchWindowActivationStep::Activate(_) => {
                    PlatformCommand::ActivateWindow(ActivateWindowCommand { window_handle })
                }
            };
            match self.platform.send_command(command) {
                Ok(command_id) => {
                    self.pending_window_activation.insert(
                        command_id,
                        PendingWindowActivation {
                            window_handle,
                            step: current,
                        },
                    );
                    return;
                }
                Err(error) => {
                    warn!(
                        %error,
                        window_handle,
                        ?current,
                        "could not queue quick-launch window activation step"
                    );
                    step = next_window_activation_step(current, false);
                    if step.is_none()
                        && matches!(current, QuickLaunchWindowActivationStep::Activate(_))
                    {
                        self.notify_window_activation_failure(cx);
                    }
                }
            }
        }
    }

    fn drive_hotkey_recording(&mut self) {
        if self.pending_recording.is_some() || self.recording_active == self.recording_desired {
            return;
        }
        let recording = self.recording_desired;
        match self
            .platform
            .send_command(PlatformCommand::SetQuickLaunchHotKeyRecording(
                SetQuickLaunchHotKeyRecordingCommand { recording },
            )) {
            Ok(command_id) => {
                self.pending_recording = Some((command_id, recording));
                self.configuration_status = if recording {
                    "正在暂停速启热键".into()
                } else {
                    "正在恢复速启热键".into()
                };
            }
            Err(error) => {
                self.recording_active = false;
                self.recording_desired = false;
                self.configuration_status = format!("热键录制失败：{error}");
                self.hotkey_error = Some(self.configuration_status.clone());
            }
        }
    }

    fn send_configuration(&mut self) {
        let settings = self.config.quick_launch.clone();
        let _ = self.send_configuration_for(&settings);
    }

    fn send_configuration_for(&mut self, settings: &QuickLaunchSettings) -> Option<uuid::Uuid> {
        self.send_configuration_correlated(settings, uuid::Uuid::new_v4(), None)
    }

    fn send_configuration_for_with_id(
        &mut self,
        settings: &QuickLaunchSettings,
        command_id: uuid::Uuid,
    ) -> Option<uuid::Uuid> {
        self.send_configuration_correlated(settings, command_id, None)
    }

    fn send_configuration_correlated(
        &mut self,
        settings: &QuickLaunchSettings,
        command_id: uuid::Uuid,
        correlation_id: Option<uuid::Uuid>,
    ) -> Option<uuid::Uuid> {
        let command = ConfigureQuickLaunchCommand {
            enabled: settings.enabled,
            auto_quick_switch_enabled: settings.auto_quick_switch,
            file_dialog_search_enabled: settings.file_dialog_search,
            suppress_in_exclusive_fullscreen: self.config.suppress_panel_in_fullscreen(),
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
        };
        let started = Instant::now();
        if let Some((old_id, old_started)) = self.latest_configuration.take() {
            info!(event = "quick_launch.configuration.superseded", correlation_id = %old_id,
                elapsed_ms = old_started.elapsed().as_millis());
        }
        info!(event = "quick_launch.configuration.started", correlation_id = %correlation_id.unwrap_or(command_id),
            %command_id, enabled = settings.enabled);
        self.pending_configuration.insert(command_id);
        let queued = match correlation_id {
            Some(correlation_id) => self.platform.send_command_with_id_correlated(
                PlatformCommand::ConfigureQuickLaunch(command),
                command_id,
                correlation_id,
            ),
            None => self
                .platform
                .send_command_with_id(PlatformCommand::ConfigureQuickLaunch(command), command_id),
        };
        match queued {
            Ok(_) => {
                self.latest_configuration = Some((command_id, started));
                self.configuration_status = "正在应用".into();
                Some(command_id)
            }
            Err(error) => {
                warn!(event = "quick_launch.configuration.failed", correlation_id = %correlation_id.unwrap_or(command_id),
                    %command_id, enabled = settings.enabled, stage = "enqueue", elapsed_ms = started.elapsed().as_millis());
                self.pending_configuration.remove(&command_id);
                self.configuration_status = match error {
                    PlatformBridgeError::Disconnected => "等待 Windows 适配器".into(),
                    other => format!("应用失败：{other}"),
                };
                None
            }
        }
    }

    fn combined_results(&self, query: &str) -> Vec<SearchResult> {
        if query.trim().is_empty() {
            return Vec::new();
        }
        let indexed = if self.index_search.query == query {
            self.index_search.items.as_slice()
        } else {
            &[]
        };
        if recent_modified_filter(query).is_some() {
            return recent_modified_results(indexed);
        }
        let indexed = indexed_search_entries(indexed);
        let provisional = self.index_search.query == query && !self.index_search.complete;
        self.merge_extension_rows(
            query,
            self.unified_catalog_results(query, &indexed, provisional),
        )
    }

    /// One ranking owner for every launcher surface: local catalog sources and indexed rows
    /// are scored, de-duplicated and history-weighted together by the Finder catalog.
    fn unified_catalog_results(
        &self,
        query: &str,
        indexed: &[ExternalSearchEntry],
        provisional: bool,
    ) -> Vec<SearchResult> {
        let catalog = self.catalog.borrow();
        let results = if provisional {
            catalog.search_with_packages_normal(
                query,
                indexed,
                &self.config.quick_launch.custom_commands,
                &self.config.extensions,
                RESULT_LIMIT,
            )
        } else {
            catalog.search_with_packages(
                query,
                indexed,
                &self.config.quick_launch.custom_commands,
                &self.config.extensions,
                RESULT_LIMIT,
            )
        };
        results.unwrap_or_else(|error| {
            error!(%error, query_length = query.chars().count(), "quick-launch unified search failed");
            Vec::new()
        })
    }

    fn cancel_index_search(&mut self, reason: &'static str, cx: &mut Context<Self>) {
        let Some(cancel) = self.pending_index_search.take() else {
            return;
        };
        let indexer = Rc::clone(&self.indexer);
        info!(event = "quick_launch.search.cancel_requested", correlation_id = %cancel.request_id,
            generation = cancel.generation, reason, "indexed search retired locally");
        cx.spawn(async move |_, _| {
            let started = Instant::now();
            let queued = indexer.cancel_search(cancel).await.is_ok();
            info!(event = "quick_launch.search.cancel_dispatched", correlation_id = %cancel.request_id,
                generation = cancel.generation, reason, queued,
                elapsed_us = started.elapsed().as_micros() as u64, "indexed search cancellation dispatch ended");
        }).detach();
    }

    fn search_and_request_icons(
        &mut self,
        query: &str,
        cx: &mut Context<Self>,
    ) -> QuickLaunchSearchUpdate {
        if !self.feature_enabled() {
            return QuickLaunchSearchUpdate {
                results: Vec::new(),
                pending: false,
            };
        }
        let local_search_started = Instant::now();
        self.begin_extension_providers(query, cx);
        let recent_mode = recent_modified_filter(query).is_some();
        let local_normal_hit = if recent_mode {
            false
        } else {
            self.catalog
                .borrow()
                .has_local_normal_hit(
                    query,
                    &self.config.quick_launch.custom_commands,
                    &self.config.extensions,
                    RESULT_LIMIT,
                )
                .unwrap_or_else(|_| {
                    warn!(
                        reason = "local_search_failed",
                        query_length = query.chars().count(),
                        "quick-launch ordinary local search failed"
                    );
                    false
                })
        };
        let allow_homophone_fallback = !recent_mode && !local_normal_hit;
        let mut index_request = self.index_search.begin(query, INDEX_CANDIDATE_LIMIT);
        index_request.context.allow_homophone_fallback = allow_homophone_fallback;
        let generation = index_request.generation;
        let indexer_connected = self.indexer.is_connected();
        let index_available = index_status_has_searchable_snapshot(self.index_status.as_ref());
        let search_indexer = should_search_indexer(indexer_connected, index_available)
            && (recent_modified_filter(query).is_some()
                || query_requests_file_index_with_commands(
                    query,
                    &self.config.quick_launch.custom_commands,
                ));
        let search_queued = if query.trim().is_empty() || !search_indexer {
            self.cancel_index_search(
                if query.trim().is_empty() {
                    "cleared"
                } else {
                    "provider_not_requested"
                },
                cx,
            );
            false
        } else {
            let request_id = uuid::Uuid::new_v4();
            let previous = self.pending_index_search.take();
            self.pending_index_search = Some(nxtools_protocol::IndexSearchCancel {
                request_id,
                generation,
            });
            match self
                .indexer
                .send_command_with_id(IndexerCommand::Search(index_request), request_id)
            {
                Ok(_) => true,
                Err(error) => {
                    self.pending_index_search = previous;
                    self.cancel_index_search("dispatch_failed", cx);
                    warn!(%error, "could not queue indexed-file search");
                    false
                }
            }
        };
        self.index_search.complete = !search_queued;
        info!(
            generation,
            query_length = query.chars().count(),
            indexer_connected,
            index_available,
            search_queued,
            allow_homophone_fallback,
            "quick-launch query provider state"
        );
        self.last_local_search_us = Some(elapsed_microseconds(local_search_started));
        if search_queued || self.extension_search_in_progress(query) {
            return QuickLaunchSearchUpdate {
                results: Vec::new(),
                pending: true,
            };
        }
        let results = self.combined_results(query);
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        self.index_search.terminal_published = true;
        QuickLaunchSearchUpdate {
            results,
            pending: false,
        }
    }

    fn refresh_indexed_results(&mut self, cx: &mut Context<Self>, provider_changed: bool) {
        if !self.index_search.complete
            || self.extension_search_in_progress(&self.index_search.query)
            || (!provider_changed && self.index_search.terminal_published)
        {
            return;
        }
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let panel_state = popup
            .panel
            .update(cx, |panel, cx| {
                (
                    panel.is_open,
                    panel.action_mode.is_none(),
                    panel.search_input.read(cx).value().to_string(),
                )
            })
            .ok();
        let Some((true, true, query)) = panel_state else {
            return;
        };
        if query != self.index_search.query {
            return;
        }
        let local_search_started = Instant::now();
        let results = self.combined_results(&query);
        self.last_local_search_us = Some(elapsed_microseconds(local_search_started));
        let indexed_visible = results
            .iter()
            .filter(|result| {
                matches!(
                    result.entry.source,
                    ApplicationSource::IndexedDirectory | ApplicationSource::IndexedFile
                )
            })
            .count();
        let result_count = results.len();
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        let failed = self.index_search.failed;
        let published = popup
            .panel
            .update(cx, |panel, cx| {
                if !panel.is_open
                    || panel.action_mode.is_some()
                    || panel.search_input.read(cx).value().as_ref() != query
                {
                    return false;
                }
                panel.replace_results(results, cx);
                if failed {
                    panel.set_error(INDEX_SEARCH_FAILURE_MESSAGE.into(), cx);
                }
                true
            })
            .unwrap_or(false);
        if published {
            self.index_search.terminal_published = true;
            info!(
                generation = self.index_search.generation,
                complete = self.index_search.complete,
                indexed_candidates = self.index_search.items.len(),
                indexed_visible,
                result_count,
                "quick-launch merged provider results published"
            );
        }
    }

    fn refresh_results_after_indexer_disconnect(&mut self, cx: &mut Context<Self>) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let panel_state = popup
            .panel
            .update(cx, |panel, cx| {
                (
                    panel.is_open,
                    panel.action_mode.is_none(),
                    panel.search_input.read(cx).value().to_string(),
                )
            })
            .ok();
        let Some((true, true, query)) = panel_state else {
            return;
        };
        self.index_search.query = query;
        self.index_search.complete = true;
        self.refresh_indexed_results(cx, true);
    }

    fn request_catalog_refresh(&mut self, cx: &mut Context<Self>) {
        self.request_installed_applications(cx);
        if self.catalog_refresh_in_progress {
            return;
        }
        self.catalog_refresh_in_progress = true;
        let roots = self.catalog.borrow().roots().to_vec();
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let snapshot = background
                .spawn(async move { scan_application_roots(&roots) })
                .await;
            if let Some(this) = this.upgrade() {
                this.update(cx, |application, cx| {
                    application.finish_catalog_refresh(snapshot, cx);
                });
            }
        })
        .detach();
    }

    fn finish_catalog_refresh(
        &mut self,
        snapshot: ApplicationCatalogSnapshot,
        cx: &mut Context<Self>,
    ) {
        self.catalog_refresh_in_progress = false;
        if let Err(errors) = self.catalog.borrow_mut().apply_refresh_snapshot(snapshot) {
            let reason = errors
                .first()
                .cloned()
                .unwrap_or_else(|| "未知目录读取错误".into());
            warn!(error_count = errors.len(), %reason, "quick-launch catalog refresh rejected");
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "quick-launch-refresh",
                    "速启索引刷新失败",
                    format!("已保留上次可用结果：{reason}"),
                    cx,
                );
            });
            if let Some(popup) = self.popup.clone() {
                let _ = popup.panel.update(cx, |panel, cx| {
                    if panel.is_open {
                        panel.set_error(format!("索引刷新失败，已保留原结果：{reason}"), cx);
                    }
                });
            }
            cx.notify();
            return;
        }
        self.refresh_catalog_results(cx);
    }

    fn refresh_catalog_results(&mut self, cx: &mut Context<Self>) {
        if !self.feature_enabled() {
            return;
        }
        let Some(popup) = self.popup.clone() else {
            cx.notify();
            return;
        };
        let query = popup
            .panel
            .update(cx, |panel, cx| {
                quick_launch_catalog_refresh_query(
                    panel.is_open,
                    panel.action_mode.is_some(),
                    panel.search_input.read(cx).value().to_string(),
                )
            })
            .ok()
            .flatten();
        let Some(query) = query else {
            cx.notify();
            return;
        };
        if self.indexer.is_connected()
            && self.index_search.query == query
            && !self.index_search.complete
        {
            cx.notify();
            return;
        }
        let results = self.combined_results(&query);
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        let _ = popup
            .panel
            .update(cx, |panel, cx| panel.replace_results(results, cx));
        info!(
            indexed_items = self.catalog.borrow().status().indexed_items,
            "quick-launch application catalog refreshed"
        );
        cx.notify();
    }

    fn schedule_quick_launch_preview(
        &mut self,
        entered: bool,
        result: SearchResult,
        panel_bounds: Bounds<Pixels>,
        panel_scale_factor: f32,
        cx: &mut Context<Self>,
    ) {
        if !self.feature_enabled() || self.popup_presentation.is_some() {
            return;
        }
        if !quick_launch_result_capabilities(&result).preview {
            if entered {
                self.dismiss_quick_launch_preview(cx);
            }
            return;
        }
        let entry_id = result.entry.id.clone();
        if entered {
            self.preview.hovered_entry_id = Some(entry_id.clone());
            self.preview.pointer_inside = false;
            self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
            let candidate = QuickLaunchPreviewCandidate {
                result,
                panel_bounds,
                panel_scale_factor,
            };
            let switching_target = self
                .preview
                .pending_stage
                .as_ref()
                .is_some_and(|stage| stage.entry_id != entry_id)
                || quick_launch_preview_target_changed(
                    self.preview.visible,
                    self.preview.displayed_entry_id.as_deref(),
                    &entry_id,
                );
            if self.preview.visible {
                self.cancel_quick_launch_preview_exit(cx);
            }
            if quick_launch_preview_open_action(
                self.preview.window.is_some(),
                self.preview.visible,
                self.preview.displayed_entry_id.as_deref(),
                &entry_id,
            ) != QuickLaunchPreviewOpenAction::Ignore
            {
                self.schedule_quick_launch_preview_candidate(
                    candidate,
                    if switching_target {
                        QUICK_LAUNCH_PREVIEW_SWITCH_DELAY
                    } else {
                        QUICK_LAUNCH_PREVIEW_HOVER_DELAY
                    },
                    cx,
                );
            }
        } else if self.preview.hovered_entry_id.as_deref() == Some(entry_id.as_str()) {
            self.preview.hovered_entry_id = None;
            self.schedule_quick_launch_preview_hide(QUICK_LAUNCH_PREVIEW_LEAVE_DELAY, cx);
        }
    }

    fn schedule_quick_launch_preview_candidate(
        &mut self,
        candidate: QuickLaunchPreviewCandidate,
        delay: Duration,
        cx: &mut Context<Self>,
    ) {
        if !self.feature_enabled() {
            return;
        }
        let generation = self.preview.generation;
        let entry_id = candidate.result.entry.id.clone();
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            background.timer(delay).await;
            let should_load = this
                .update(cx, |application, _| {
                    application.preview.generation == generation
                        && application.preview.hovered_entry_id.as_deref()
                            == Some(entry_id.as_str())
                })
                .unwrap_or(false);
            if !should_load {
                return;
            }
            let result = candidate.result.clone();
            let payload = background
                .spawn(async move { load_quick_launch_preview(&result) })
                .await;
            let _ = this.update(cx, |application, cx| {
                if application.preview.generation != generation
                    || application.preview.hovered_entry_id.as_deref() != Some(entry_id.as_str())
                {
                    return;
                }
                if let Some(payload) = payload {
                    application.show_quick_launch_preview(payload, candidate, cx);
                } else {
                    application.hide_quick_launch_preview_window(cx);
                }
            });
        })
        .detach();
    }

    fn set_quick_launch_preview_pointer_inside(&mut self, inside: bool, cx: &mut Context<Self>) {
        self.preview.pointer_inside = inside;
        self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
        if inside {
            self.cancel_quick_launch_preview_exit(cx);
        } else if self.preview.hovered_entry_id.is_none() {
            self.schedule_quick_launch_preview_hide(QUICK_LAUNCH_PREVIEW_LEAVE_DELAY, cx);
        }
    }

    fn schedule_quick_launch_preview_hide(&mut self, delay: Duration, cx: &mut Context<Self>) {
        self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
        let generation = self.preview.generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(delay).await;
            let _ = this.update(cx, |application, cx| {
                if application.preview.generation == generation
                    && application.preview.hovered_entry_id.is_none()
                    && !application.preview.pointer_inside
                {
                    application.animate_quick_launch_preview_exit(cx);
                } else if application.preview.visible
                    && application.preview.hovered_entry_id.is_none()
                    && !application.preview.pointer_inside
                {
                    // A racing hover bumped the generation and cancelled this
                    // timer, but the replacement preview never actually took
                    // over (target switch rejected or staging dropped). Rearm
                    // once more so a visible-but-unhovered preview cannot stay
                    // on screen orphaned — the reported "preview never closes"
                    // symptom.
                    application.schedule_quick_launch_preview_hide(delay, cx);
                }
            });
        })
        .detach();
    }

    fn cancel_quick_launch_preview_exit(&mut self, cx: &mut Context<Self>) {
        self.preview.exit_generation = self.preview.exit_generation.wrapping_add(1).max(1);
        if let Some(preview) = self.preview.window.as_ref() {
            let _ = preview
                .view
                .update(cx, |preview, cx| preview.cancel_exit(cx));
        }
    }

    fn animate_quick_launch_preview_exit(&mut self, cx: &mut Context<Self>) {
        if !self.preview.visible {
            self.hide_quick_launch_preview_window(cx);
            return;
        }
        self.preview.exit_generation = self.preview.exit_generation.wrapping_add(1).max(1);
        let exit_generation = self.preview.exit_generation;
        let motion_started = self.preview.window.as_ref().is_some_and(|preview| {
            preview
                .view
                .update(cx, |preview, cx| preview.begin_exit(cx))
                .is_ok()
        });
        if !motion_started {
            self.hide_quick_launch_preview_window(cx);
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(QUICK_LAUNCH_PREVIEW_DISAPPEAR_DURATION)
                .await;
            let _ = this.update(cx, |application, cx| {
                if application.preview.exit_generation == exit_generation {
                    application.hide_quick_launch_preview_window(cx);
                }
            });
        })
        .detach();
    }

    fn suspend_quick_launch_preview_for_scroll(&mut self, cx: &mut Context<Self>) {
        self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
        self.preview.hovered_entry_id = None;
        self.preview.pointer_inside = false;
        self.hide_quick_launch_preview_window(cx);
    }

    fn dismiss_quick_launch_preview(&mut self, cx: &mut Context<Self>) {
        self.preview.generation = self.preview.generation.wrapping_add(1).max(1);
        self.preview.hovered_entry_id = None;
        self.preview.pointer_inside = false;
        self.hide_quick_launch_preview_window(cx);
    }

    fn hide_quick_launch_preview_window(&mut self, cx: &mut Context<Self>) {
        let native_may_be_visible =
            self.preview.visible || self.preview.material_lifecycle.is_some();
        self.preview.material_wait = None;
        if let Some((id, started)) = self.preview.material_lifecycle.take() {
            info!(event = "quick_launch.preview.prepare.retired", correlation_id = %id,
                elapsed_ms = started.elapsed().as_millis() as u64);
        }
        self.preview.material.retire(&self.platform);
        self.preview.exit_generation = self.preview.exit_generation.wrapping_add(1).max(1);
        self.preview.pending_stage = None;
        if let Some(image) = self.preview.image_asset.take() {
            ImageSource::from(image).remove_asset(cx);
        }
        if let Some(preview) = self.preview.window.as_ref() {
            let _ = preview
                .view
                .update(cx, |preview, cx| preview.clear_image_asset(cx));
        }
        let native_window_handle = self
            .preview
            .window
            .as_ref()
            .map_or(0, |preview| preview.identity.native);
        if native_may_be_visible
            && native_window_handle != 0
            && let Err(error) = self
                .platform
                .send_command(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand {
                        window_handle: native_window_handle,
                        visible: false,
                    },
                ))
        {
            warn!(%error, "failed to hide quick-launch preview");
        }
        self.preview.visible = false;
        self.preview.displayed_entry_id = None;
    }

    fn show_quick_launch_preview(
        &mut self,
        payload: QuickLaunchPreviewPayload,
        candidate: QuickLaunchPreviewCandidate,
        cx: &mut Context<Self>,
    ) {
        if !self.feature_enabled() || self.popup_presentation.is_some() {
            return;
        }
        let placement = crate::ui::monitor_placement::owner_monitor_placement(
            self.popup_monitor,
            self.popup.as_ref().map(|popup| popup.window),
            cx,
        );
        let screen = placement.work_area;
        let preview_size = size(
            px(QUICK_LAUNCH_PREVIEW_WIDTH.min(screen.size.width.as_f32())),
            px(quick_launch_preview_height(&payload).min(screen.size.height.as_f32())),
        );
        let bounds = quick_launch_preview_bounds(candidate.panel_bounds, preview_size, screen);
        self.preview.exit_generation = self.preview.exit_generation.wrapping_add(1).max(1);
        let action = quick_launch_preview_open_action(
            self.preview.window.is_some(),
            self.preview.visible,
            self.preview.displayed_entry_id.as_deref(),
            &payload.entry_id,
        );
        if action == QuickLaunchPreviewOpenAction::Ignore {
            self.cancel_quick_launch_preview_exit(cx);
            return;
        }
        if action == QuickLaunchPreviewOpenAction::Reuse {
            self.stage_reused_quick_launch_preview(payload, preview_size, bounds, candidate, cx);
            return;
        }

        let application = cx.entity().downgrade();
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_native_slot = Rc::clone(&native_slot);
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let displayed_entry_id = payload.entry_id.clone();
        let mut image_asset = quick_launch_preview_image_asset(&payload);
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
                app_id: Some("NxTools.QuickLaunchPreview".into()),
                window_min_size: Some(preview_size),
                window_decorations: Some(WindowDecorations::Client),
                app_owns_titlebar_drag: false,
                inactive_frame_interval: None,
                icon: None,
                tabbing_identifier: None,
                ..Default::default()
            },
            move |window, cx| {
                let window_handle = if cfg!(test) {
                    0
                } else {
                    native_window_handle(window).unwrap_or_default()
                };
                *opened_native_slot.borrow_mut() = window_handle;
                let view = cx.new(|_| QuickLaunchPreview::new(payload, application));
                *opened_view_slot.borrow_mut() = Some(view.downgrade());
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                let Some(view) = view_slot.borrow_mut().take() else {
                    let window: AnyWindowHandle = window.into();
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                    if let Some(image) = image_asset.take() {
                        ImageSource::from(image).remove_asset(cx);
                    }
                    return;
                };
                self.preview.window = Some(QuickLaunchPreviewWindow {
                    identity: QuickLaunchPreviewWindowIdentity {
                        gpui: window.into(),
                        native: *native_slot.borrow(),
                    },
                    view,
                });
                self.preview.image_asset = image_asset;
                self.preview.no_activate_ready = false;
                self.preview.restore_primary_focus = true;
                let window_identity = self
                    .preview
                    .window
                    .as_ref()
                    .expect("created preview")
                    .identity;
                if cfg!(test) && window_identity.native == 0 {
                    // TestWindow has no native visibility boundary.
                    self.preview.visible = true;
                    self.preview.displayed_entry_id = Some(displayed_entry_id);
                    self.restore_quick_launch_panel_keyboard_focus(cx);
                    self.preview.restore_primary_focus = false;
                } else {
                    self.preview.visible = false;
                    self.preview.displayed_entry_id = None;
                    self.begin_preview_material_presentation(cx);
                    let stage = QuickLaunchPreviewPendingStage {
                        command_id: uuid::Uuid::nil(),
                        phase: QuickLaunchPreviewStagePhase::InitialNoActivate,
                        window_identity,
                        generation: self.preview.generation,
                        entry_id: displayed_entry_id,
                        payload: None,
                        preview_size,
                        bounds,
                        panel_scale_factor: candidate.panel_scale_factor,
                    };
                    self.send_preview_stage(
                        stage,
                        PlatformCommand::SetWindowNoActivate(SetWindowNoActivateCommand {
                            window_handle: window_identity.native,
                            enabled: true,
                        }),
                        cx,
                    );
                }
            }
            Err(error) => {
                self.preview.visible = false;
                self.preview.displayed_entry_id = None;
                if let Some(image) = image_asset {
                    ImageSource::from(image).remove_asset(cx);
                }
                error!(%error, "failed to open quick-launch preview");
            }
        }
    }

    fn restore_quick_launch_panel_keyboard_focus(&self, cx: &mut Context<Self>) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let owner = cx.entity().downgrade();
        let platform = Rc::clone(&self.platform);
        let generation = self.preview.generation;
        cx.defer(move |app| {
            let current = owner.read_with(app, |owner, _| {
                owner.preview.visible && owner.preview.generation == generation
                    && owner.popup.as_ref().is_some_and(|current| {
                        current.window == popup.window
                            && current.native_window_handle == popup.native_window_handle
                    })
            }).unwrap_or(false);
            if !current { return; }
            if popup.native_window_handle != 0
                && let Err(error) = platform.send_command(PlatformCommand::ActivateWindow(
                    ActivateWindowCommand { window_handle: popup.native_window_handle }
                ))
            {
                warn!(%error, "failed to restore quick-launch keyboard target after preview creation");
            }
            let _ = popup.window.update(app, |_, window, app| {
                let _ = popup.panel.update(app, |panel, cx| panel.focus_search(window, cx));
            });
        });
    }

    fn stage_reused_quick_launch_preview(
        &mut self,
        payload: QuickLaunchPreviewPayload,
        preview_size: Size<Pixels>,
        bounds: Bounds<Pixels>,
        candidate: QuickLaunchPreviewCandidate,
        cx: &mut Context<Self>,
    ) {
        let Some(preview_window) = self.preview.window.as_ref() else {
            return;
        };
        let window_identity = preview_window.identity;
        let native_window_handle = window_identity.native;
        if native_window_handle == 0 {
            self.preview.window = None;
            self.show_quick_launch_preview(payload, candidate, cx);
            return;
        }
        self.begin_preview_material_presentation(cx);
        let entry_id = payload.entry_id.clone();
        let mut stage = QuickLaunchPreviewPendingStage {
            command_id: uuid::Uuid::nil(),
            phase: QuickLaunchPreviewStagePhase::MoveOffscreen,
            window_identity,
            generation: self.preview.generation,
            entry_id,
            payload: Some(payload),
            preview_size,
            bounds,
            panel_scale_factor: candidate.panel_scale_factor,
        };
        if !self.preview.no_activate_ready {
            stage.phase = QuickLaunchPreviewStagePhase::InitialNoActivate;
            self.send_preview_stage(
                stage,
                PlatformCommand::SetWindowNoActivate(SetWindowNoActivateCommand {
                    window_handle: native_window_handle,
                    enabled: true,
                }),
                cx,
            );
            return;
        }
        match self
            .platform
            .send_command(PlatformCommand::SetWindowPosition(
                SetWindowPositionCommand {
                    window_handle: native_window_handle,
                    x: QUICK_LAUNCH_PREVIEW_STAGING_COORDINATE,
                    y: QUICK_LAUNCH_PREVIEW_STAGING_COORDINATE,
                },
            )) {
            Ok(command_id) => {
                stage.command_id = command_id;
                self.preview.pending_stage = Some(stage);
                self.preview.displayed_entry_id = None;
            }
            Err(error) => {
                warn!(%error, "failed to stage quick-launch preview outside the display");
                self.hide_quick_launch_preview_window(cx);
            }
        }
    }

    fn handle_quick_launch_preview_stage_result(
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
        let stage = self
            .preview
            .pending_stage
            .take()
            .expect("matching quick-launch preview stage must exist");
        if self
            .preview
            .window
            .as_ref()
            .is_none_or(|preview| preview.identity != stage.window_identity)
        {
            return true;
        }
        if self.preview.generation != stage.generation
            || self.preview.hovered_entry_id.as_deref() != Some(stage.entry_id.as_str())
        {
            return true;
        }
        if stage.window_identity.native != 0 && !self.preview_stage_is_current(&stage, cx) {
            self.hide_quick_launch_preview_window(cx);
            return true;
        }
        if !success {
            warn!(
                %correlation_id,
                reason = error_message.unwrap_or("unknown Windows error"),
                phase = ?stage.phase,
                "quick-launch preview staging failed"
            );
            self.hide_quick_launch_preview_window(cx);
            return true;
        }
        match stage.phase {
            QuickLaunchPreviewStagePhase::InitialNoActivate => {
                self.preview.no_activate_ready = true;
                self.prepare_initial_preview_bounds(stage, cx);
            }
            QuickLaunchPreviewStagePhase::InitialBounds => {
                self.wait_for_preview_material(stage, PreviewMaterialAction::InitialFrame, cx);
            }
            QuickLaunchPreviewStagePhase::InitialShow => {
                self.finish_prepared_preview(stage, cx);
            }
            QuickLaunchPreviewStagePhase::MoveOffscreen if !self.preview.visible => {
                self.wait_for_preview_material(stage, PreviewMaterialAction::ShowOffscreen, cx);
            }
            QuickLaunchPreviewStagePhase::MoveOffscreen
            | QuickLaunchPreviewStagePhase::ShowOffscreen => {
                self.preview.visible = true;
                self.render_staged_quick_launch_preview(stage, cx);
            }
            QuickLaunchPreviewStagePhase::RevealOnscreen => {
                if cfg!(test) && stage.window_identity.native == 0 {
                    self.preview.visible = true;
                    self.preview.displayed_entry_id = Some(stage.entry_id);
                } else {
                    self.finish_prepared_preview(stage, cx);
                }
            }
        }
        true
    }

    fn render_staged_quick_launch_preview(
        &mut self,
        stage: QuickLaunchPreviewPendingStage,
        cx: &mut Context<Self>,
    ) {
        let Some(preview) = self.preview.window.clone() else {
            self.preview.visible = false;
            return;
        };
        let QuickLaunchPreviewPendingStage {
            window_identity,
            generation,
            entry_id,
            payload,
            preview_size,
            bounds,
            panel_scale_factor,
            ..
        } = stage;
        let Some(payload) = payload else {
            self.hide_quick_launch_preview_window(cx);
            return;
        };
        let image_asset = quick_launch_preview_image_asset(&payload);
        let view = preview.view.clone();
        let application = cx.entity().downgrade();
        let update = preview.identity.gpui.update(cx, move |_, window, app| {
            if view
                .update(app, |preview, cx| preview.set_payload(payload, cx))
                .is_err()
            {
                return;
            }
            window.on_next_frame(move |_, app| {
                app.defer(move |app| {
                    let _ = application.update(app, |application, cx| {
                        let stage = QuickLaunchPreviewPendingStage {
                            command_id: uuid::Uuid::nil(),
                            phase: QuickLaunchPreviewStagePhase::RevealOnscreen,
                            window_identity,
                            generation,
                            entry_id,
                            payload: None,
                            preview_size,
                            bounds,
                            panel_scale_factor,
                        };
                        application.wait_for_preview_material(
                            stage,
                            PreviewMaterialAction::RevealOnscreen,
                            cx,
                        );
                    });
                });
            });
            window.refresh();
        });
        if update.is_err() {
            self.preview.window = None;
            self.preview.visible = false;
            self.preview.displayed_entry_id = None;
            if let Some(image) = image_asset {
                ImageSource::from(image).remove_asset(cx);
            }
        } else {
            self.preview.image_asset = image_asset;
        }
    }

    fn request_icons(&mut self, results: &[SearchResult], cx: &mut Context<Self>) {
        let mut cached = Vec::new();
        for result in results {
            if !quick_launch_result_capabilities(result).shell_icon {
                continue;
            }
            let entry_id = result.entry.id.clone();
            let resource_key = icon_resource_key(result, 32);
            if let Some(path) = self.icon_resources.get(&resource_key) {
                cached.push((entry_id, path));
                continue;
            }
            if let Some(command_id) = self.pending_icon_resources.get(&resource_key).copied() {
                if let Some(pending) = self.pending_icons.get_mut(&command_id) {
                    pending.entry_ids.insert(entry_id);
                }
                continue;
            }
            if self
                .failed_icon_resources
                .get(&resource_key)
                .is_some_and(|failed_at| icon_retry_pending(*failed_at, Instant::now()))
            {
                continue;
            }
            self.failed_icon_resources.remove(&resource_key);
            match self
                .platform
                .send_command(PlatformCommand::ExtractShellIcon(ExtractShellIconCommand {
                    target: result.entry.target.clone(),
                    size: 32,
                })) {
                Ok(command_id) => {
                    self.pending_icon_resources
                        .insert(resource_key.clone(), command_id);
                    self.pending_icons.insert(
                        command_id,
                        PendingIconResource {
                            resource_key,
                            entry_ids: HashSet::from([entry_id]),
                        },
                    );
                }
                Err(error) => {
                    warn!(%error, "could not request a quick-launch icon");
                    self.record_icon_failure(resource_key, cx);
                }
            }
        }
        if !cached.is_empty() {
            self.deliver_file_dialog_search_icons(cached.clone(), cx);
        }
        if !cached.is_empty()
            && let Some(popup) = self.popup.clone()
        {
            cx.defer(move |cx| {
                let _ = popup
                    .panel
                    .update(cx, |panel, cx| panel.set_icons(cached, cx));
            });
        }
    }

    fn record_icon_failure(&mut self, resource_key: String, cx: &mut Context<Self>) {
        let failed_at = Instant::now();
        self.failed_icon_resources
            .insert(resource_key.clone(), failed_at);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(ICON_RETRY_DELAY).await;
            let Some(this) = this.upgrade() else {
                return;
            };
            this.update(cx, |application, cx| {
                if application
                    .failed_icon_resources
                    .get(&resource_key)
                    .is_none_or(|current| *current != failed_at)
                {
                    return;
                }
                application.failed_icon_resources.remove(&resource_key);
                if let Some(popup) = application.popup.clone() {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        panel.last_presented_icon_range = None;
                        cx.notify();
                    });
                }
            });
        })
        .detach();
    }

    fn query_shell_actions(
        &mut self,
        mode_id: uuid::Uuid,
        level_id: uuid::Uuid,
        entry_id: String,
        target: String,
        cx: &mut Context<Self>,
    ) {
        match self
            .platform
            .send_command(PlatformCommand::QueryShellActions(
                QueryShellActionsCommand { target },
            )) {
            Ok(command_id) => {
                self.pending_shell_queries.insert(
                    command_id,
                    PendingShellQuery {
                        mode_id,
                        level_id,
                        entry_id,
                    },
                );
            }
            Err(error) => {
                warn!(%error, "could not queue Windows Shell action enumeration");
                self.apply_shell_actions(mode_id, level_id, &entry_id, None, cx);
            }
        }
    }

    fn query_shell_submenu(
        &mut self,
        mode_id: uuid::Uuid,
        level: PendingShellSubmenuLevel,
        entry_id: String,
        cx: &mut Context<Self>,
    ) {
        let command = PlatformCommand::QueryShellSubmenu(QueryShellSubmenuCommand {
            session_id: level.session_id,
            submenu_id: level.submenu_id,
        });
        match self.platform.send_command(command) {
            Ok(command_id) => {
                self.pending_shell_queries.insert(
                    command_id,
                    PendingShellQuery {
                        mode_id,
                        level_id: level.level_id,
                        entry_id,
                    },
                );
            }
            Err(error) => {
                warn!(%error, "could not queue Windows Shell submenu enumeration");
                self.apply_shell_actions(mode_id, level.level_id, &entry_id, None, cx);
            }
        }
    }

    fn apply_shell_actions(
        &mut self,
        mode_id: uuid::Uuid,
        level_id: uuid::Uuid,
        entry_id: &str,
        menu: Option<ShellActionMenu>,
        cx: &mut Context<Self>,
    ) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let _ = popup.panel.update(cx, |panel, cx| {
            panel.apply_shell_actions(mode_id, level_id, entry_id, menu, cx);
        });
    }

    fn execute(
        &mut self,
        result: SearchResult,
        action: QuickLaunchAction,
        query: String,
        trigger: QuickLaunchActionTrigger,
        cx: &mut Context<Self>,
    ) {
        if !self.feature_enabled() {
            return;
        }
        let action_correlation_id = uuid::Uuid::new_v4();
        if result.entry.id.starts_with("extension-script:")
            && !self.extension_result_current(&result, &query)
        {
            return;
        }
        if result.entry.id.starts_with("extension-script:")
            && matches!(
                action,
                QuickLaunchAction::RunElevated | QuickLaunchAction::Shell { .. }
            )
        {
            return;
        }
        if result.entry.id.starts_with("extension-script:") {
            self.verify_and_execute_extension(result, action, query, trigger, cx);
            return;
        }
        if result.entry.id.starts_with("extension:") {
            let current = self
                .catalog
                .borrow()
                .search_with_packages_normal(
                    &query,
                    &[],
                    &self.config.quick_launch.custom_commands,
                    &self.config.extensions,
                    RESULT_LIMIT,
                )
                .is_ok_and(|rows| {
                    rows.iter().any(|row| {
                        row.entry.id == result.entry.id
                            && row.entry.target == result.entry.target
                            && row.action == result.action
                    })
                });
            if !current {
                warn!(event = "quick_launch.action.rejected", correlation_id=%action_correlation_id,
                    reason="extension_retired_or_changed", "retired extension action rejected");
                if let Some(popup) = self.popup.clone() {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        panel.notify_operation(
                            crate::notifications::NotificationLevel::Error,
                            "扩展已停用或更新，请重新搜索后再执行".into(),
                            cx,
                        )
                    });
                }
                return;
            }
        }
        self.execute_native_result(result, action, query, trigger, cx);
    }
    fn execute_native_result(
        &mut self,
        result: SearchResult,
        action: QuickLaunchAction,
        query: String,
        trigger: QuickLaunchActionTrigger,
        cx: &mut Context<Self>,
    ) {
        if !self.feature_enabled() {
            return;
        }
        if self.try_archive_open(&result, &action, &query, trigger, cx) {
            return;
        }
        let started_at = Instant::now();
        let action_correlation_id = uuid::Uuid::new_v4();
        let execute_session_id = self.popup_session_id;
        let owner_window_handle = self
            .popup
            .as_ref()
            .map_or(0, |popup| popup.native_window_handle);
        let (
            target_kind,
            target_root,
            target_library,
            target_length,
            target_segment_count,
            target_has_trailing_separator,
        ) = directory_target_diagnostic_fields(&result.entry.target);
        let directory_visit = match &action {
            QuickLaunchAction::SwitchDirectory {
                target_directory, ..
            } => Some(target_directory.clone()),
            QuickLaunchAction::Open
                if result.entry.source == ApplicationSource::IndexedDirectory =>
            {
                Some(result.entry.target.clone())
            }
            _ => None,
        };
        let Some(command) = platform_command_for_action(&result, &action, owner_window_handle)
        else {
            warn!(
                event = "quick_launch.action.rejected",
                correlation_id = %action_correlation_id,
                action = quick_launch_action_diagnostic_label(&action),
                trigger = trigger.diagnostic_label(),
                source = result.entry.source.label(),
                outcome = "failure",
                elapsed_ms = u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX),
                native_error_code = ?Option::<i32>::None,
                reason = "unsupported_action",
                "quick-launch action rejected before dispatch"
            );
            if let Some(popup) = self.popup.clone() {
                let _ = popup.panel.update(cx, |panel, cx| {
                    panel.notify_operation(
                        crate::notifications::NotificationLevel::Error,
                        "系统工具没有可定位的快捷方式".into(),
                        cx,
                    );
                });
            }
            return;
        };
        match self
            .platform
            .send_command_correlated(command, action_correlation_id)
        {
            Ok(command_id) => {
                info!(
                    event = "quick_launch.action.accepted",
                    correlation_id = %action_correlation_id,
                    action = quick_launch_action_diagnostic_label(&action),
                    trigger = trigger.diagnostic_label(),
                    source = result.entry.source.label(),
                    target_kind,
                    target_root = %target_root,
                    target_library = ?target_library,
                    target_length,
                    target_segment_count,
                    target_has_trailing_separator,
                    "quick-launch action accepted"
                );
                // Copy actions keep the current nested action mode available for
                // confirmation or retry; launch-like actions close on acceptance.
                if execute_session_id == self.popup_session_id
                    && action_success_policy(&action) != ActionSuccessPolicy::KeepOpen
                {
                    self.hide_popup(cx);
                }
                self.pending_actions.insert(
                    command_id,
                    PendingQuickLaunchAction {
                        correlation_id: action_correlation_id,
                        popup_session_id: self.popup_session_id,
                        entry_id: result.entry.id,
                        display_name: result.entry.name,
                        target: result.entry.target,
                        query,
                        action,
                        directory_visit,
                        trigger,
                        source: result.entry.source,
                        started_at,
                    },
                );
            }
            Err(error) => {
                warn!(
                    event = "quick_launch.action.rejected",
                    correlation_id = %action_correlation_id,
                    action = quick_launch_action_diagnostic_label(&action),
                    trigger = trigger.diagnostic_label(),
                    source = result.entry.source.label(),
                    outcome = "failure",
                    elapsed_ms = u64::try_from(started_at.elapsed().as_millis())
                        .unwrap_or(u64::MAX),
                    native_error_code = ?Option::<i32>::None,
                    reason = %error,
                    "quick-launch action rejected before dispatch"
                );
                if let Some(popup) = self.popup.clone() {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        panel.notify_operation(
                            crate::notifications::NotificationLevel::Error,
                            format!("Windows 适配器不可用：{error}"),
                            cx,
                        );
                    });
                }
            }
        }
    }

    fn toggle_panel(&mut self, request: QuickLaunchPanelRequest, cx: &mut Context<Self>) {
        if !self.feature_enabled() {
            return;
        }
        if let Some(popup) = self.popup.clone() {
            let (is_open, captured_window_handle) = popup
                .panel
                .update(cx, |panel, _| (panel.is_open, panel.target.window_handle))
                .unwrap_or((false, 0));
            if is_open {
                info!(
                    dismissal_source = "quick_launch_trigger_toggled",
                    popup_window_handle = popup.native_window_handle,
                    "quick-launch popup dismissed"
                );
                self.hide_popup_and_restore(captured_window_handle, cx);
            } else {
                self.show_existing_popup(popup, request, cx);
            }
            return;
        }
        self.open_popup(request, cx);
    }

    fn show_existing_popup(
        &mut self,
        popup: QuickLaunchWindow,
        request: QuickLaunchPanelRequest,
        cx: &mut Context<Self>,
    ) {
        let placement = monitor_presentation::PlacementTarget::from_request(&request, cx);
        self.popup_monitor = request.cursor_monitor;
        self.popup_session_id = uuid::Uuid::new_v4();
        self.active_summon = Some((request.target.window_handle, request.requested_at_unix_ms));
        self.navigation_foreground_guard = None;
        self.index_search.invalidate();
        let results = Vec::new();
        let recent_directories = if request.directory_navigation_context.is_some() {
            self.catalog
                .borrow()
                .recent_directories(RECENT_DIRECTORY_LIMIT)
        } else {
            Vec::new()
        };
        let captured_navigation_context = request.directory_navigation_context.clone();
        let (bounds, _) = quick_launch_bounds(&request, false, cx);
        let row_style = self.config.quick_launch.row_style;
        let result_entrance = self.config.quick_launch.result_entrance;
        let _ = popup.window.update(cx, |_, window, app| {
            window.resize(bounds.size);
            let _ = popup.panel.update(app, |panel, cx| {
                panel.prepare_for_show(
                    request.target,
                    request.directory_navigation_context,
                    recent_directories,
                    results,
                    window,
                    cx,
                );
                panel.row_style = row_style;
                panel.result_entrance = result_entrance;
            });
        });
        self.begin_popup_presentation(placement, cx);
        self.schedule_captured_directory_record(captured_navigation_context.as_ref(), cx);
        info!(
            window_handle = popup.native_window_handle,
            "quick-launch popup reused"
        );
        self.request_catalog_refresh(cx);
    }

    fn open_popup(&mut self, request: QuickLaunchPanelRequest, cx: &mut Context<Self>) {
        let placement = monitor_presentation::PlacementTarget::from_request(&request, cx);
        self.popup_monitor = request.cursor_monitor;
        self.popup_session_id = uuid::Uuid::new_v4();
        self.active_summon = Some((request.target.window_handle, request.requested_at_unix_ms));
        self.navigation_foreground_guard = None;
        self.index_search.invalidate();
        let results = Vec::new();
        let recent_directories = if request.directory_navigation_context.is_some() {
            self.catalog
                .borrow()
                .recent_directories(RECENT_DIRECTORY_LIMIT)
        } else {
            Vec::new()
        };
        let (bounds, display_id) = quick_launch_bounds(&request, false, cx);
        let staged_bounds = bounds;
        let application = cx.entity().downgrade();
        let target = request.target;
        let navigation_context = request.directory_navigation_context;
        let captured_navigation_context = navigation_context.clone();
        let panel_slot = Rc::new(RefCell::new(None));
        let native_slot = Rc::new(RefCell::new(0_i64));
        let row_style = self.config.quick_launch.row_style;
        let result_entrance = self.config.quick_launch.result_entrance;
        let opened_panel_slot = Rc::clone(&panel_slot);
        let opened_native_slot = Rc::clone(&native_slot);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(staged_bounds)),
                titlebar: None,
                focus: false,
                show: false,
                kind: WindowKind::PopUp,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                display_id,
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.QuickLaunch".into()),
                window_min_size: Some(size(px(1.0), px(1.0))),
                window_decorations: Some(WindowDecorations::Client),
                app_owns_titlebar_drag: false,
                inactive_frame_interval: None,
                icon: None,
                tabbing_identifier: None,
                ..Default::default()
            },
            move |window, cx| {
                *opened_native_slot.borrow_mut() = native_window_handle(window).unwrap_or_default();
                let panel = cx.new(|cx| {
                    let mut panel = QuickLaunchPanel::new(
                        results,
                        target,
                        navigation_context,
                        recent_directories,
                        application,
                        window,
                        cx,
                    );
                    panel.row_style = row_style;
                    panel.result_entrance = result_entrance;
                    panel
                });
                *opened_panel_slot.borrow_mut() = Some(panel.downgrade());
                cx.new(|cx| crate::ui::popup_surface::root(panel, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                let Some(panel) = panel_slot.borrow_mut().take() else {
                    let _ = window.update(cx, |_, window, _| window.remove_window());
                    return;
                };
                let popup = QuickLaunchWindow {
                    window: window.into(),
                    panel,
                    native_window_handle: *native_slot.borrow(),
                };
                self.popup = Some(popup);
                self.begin_popup_presentation(placement, cx);
                self.schedule_captured_directory_record(captured_navigation_context.as_ref(), cx);
                self.request_catalog_refresh(cx);
                info!("quick-launch popup opened");
            }
            Err(error) => error!(%error, "failed to open quick-launch popup"),
        }
    }

    fn schedule_captured_directory_record(
        &mut self,
        context: Option<&nxtools_protocol::DirectoryNavigationContext>,
        cx: &mut Context<Self>,
    ) {
        let Some(directory) = context
            .and_then(|context| context.current_directory.as_deref())
            .filter(|directory| Path::new(directory).is_absolute())
            .map(ToOwned::to_owned)
        else {
            return;
        };
        self.schedule_directory_record(
            directory,
            nxtools_ipc::unix_time_ms(),
            self.popup_session_id,
            cx,
        );
    }

    fn schedule_directory_record(
        &mut self,
        directory: String,
        visited_at_unix_ms: i64,
        popup_session_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        let history_path = self.catalog.borrow().history_path().to_path_buf();
        let background = cx.background_executor().clone();
        cx.spawn(async move |this, cx| {
            let recorded = background
                .spawn(async move {
                    FinderCatalog::persist_directory_visit(
                        history_path,
                        &directory,
                        visited_at_unix_ms,
                    )
                    .map_err(|error| (directory, error))
                })
                .await;
            let Some(this) = this.upgrade() else {
                return;
            };
            this.update(cx, |application, cx| {
                let recorded = match recorded {
                    Ok(Some(recorded)) => recorded,
                    Ok(None) => return,
                    Err((directory, error)) => {
                        warn!(%error, %directory, "failed to persist recent directory");
                        return;
                    }
                };
                let recent = {
                    let mut catalog = application.catalog.borrow_mut();
                    catalog.apply_recorded_directory_visit(recorded);
                    catalog.recent_directories(RECENT_DIRECTORY_LIMIT)
                };
                if application.popup_session_id != popup_session_id {
                    return;
                }
                let Some(popup) = application.popup.clone() else {
                    return;
                };
                let _ = popup.panel.update(cx, |panel, cx| {
                    if panel.is_open && panel.navigation_context.is_some() {
                        panel.recent_directories = recent;
                        cx.notify();
                    }
                });
            });
        })
        .detach();
    }

    pub(crate) fn hide_popup(&mut self, cx: &mut Context<Self>) {
        self.hide_popup_with_restore(None, cx);
    }

    fn hide_popup_and_restore(&mut self, captured_window_handle: i64, cx: &mut Context<Self>) {
        self.hide_popup_with_restore(Some(captured_window_handle), cx);
    }

    fn hide_popup_with_restore(
        &mut self,
        restore_window_handle: Option<i64>,
        cx: &mut Context<Self>,
    ) {
        self.retire_popup_presentation("hidden");
        self.popup_origin_device_pixels = None;
        self.index_search.invalidate();
        self.cancel_index_search("hidden", cx);
        self.popup_session_id = uuid::Uuid::new_v4();
        self.active_summon = None;
        self.navigation_foreground_guard = None;
        self.pending_window_activation.clear();
        self.dismiss_quick_launch_preview(cx);
        if !self.feature_enabled() {
            return;
        }
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let _ = popup.window.update(cx, |_, window, app| {
            let _ = popup.panel.update(app, |panel, cx| {
                panel.prepare_for_hide(window, cx);
            });
        });
        info!(
            event = "quick_launch.popup.hide_requested",
            popup_window_handle = popup.native_window_handle,
            restore_window_handle = ?restore_window_handle,
            "quick-launch hide commands queued"
        );
        let commands =
            quick_launch_dismiss_commands(popup.native_window_handle, restore_window_handle);
        for (index, command) in commands.into_iter().enumerate() {
            if index == 0 {
                // The visibility hide is the one command whose loss makes the
                // panel stick on screen forever. Send it correlated, track the
                // result and re-send once when it is lost or rejected.
                let guard = PendingPanelHide {
                    popup_window_handle: popup.native_window_handle,
                    attempt: 0,
                    session_id: self.popup_session_id,
                    requested_at: Instant::now(),
                };
                match self
                    .platform
                    .send_command_correlated(command, guard.correlation_id())
                {
                    Ok(command_id) => {
                        self.pending_panel_hides.insert(command_id, guard);
                        self.schedule_panel_hide_recheck(guard, cx);
                    }
                    Err(error) => {
                        error!(
                            event = "quick_launch.panel.hide_send_failed",
                            popup_window_handle = popup.native_window_handle,
                            %error,
                            "quick-launch hide command could not be queued"
                        );
                    }
                }
            } else {
                let _ = self.platform.send_command(command);
            }
        }
    }

    /// Re-sends the panel-hide visibility command for a guard whose original
    /// command was rejected or never confirmed. Capped at one retry.
    fn resend_panel_hide(&mut self, mut pending: PendingPanelHide) {
        pending.attempt += 1;
        pending.requested_at = Instant::now();
        let command = PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
            window_handle: pending.popup_window_handle,
            visible: false,
        });
        match self
            .platform
            .send_command_correlated(command, pending.correlation_id())
        {
            Ok(command_id) => {
                info!(
                    event = "quick_launch.panel.hide_retry",
                    popup_window_handle = pending.popup_window_handle,
                    attempt = pending.attempt,
                    "quick-launch panel hide re-sent"
                );
                self.pending_panel_hides.insert(command_id, pending);
            }
            Err(error) => {
                error!(
                    event = "quick_launch.panel.hide_send_failed",
                    popup_window_handle = pending.popup_window_handle,
                    attempt = pending.attempt,
                    %error,
                    "quick-launch panel hide retry could not be queued"
                );
            }
        }
    }

    /// The user report: the panel sometimes never disappears. A lost command
    /// result must not leave that state undetected, so one bounded recheck
    /// re-sends the hide when no confirmation arrived in time.
    fn schedule_panel_hide_recheck(&mut self, guard: PendingPanelHide, cx: &mut Context<Self>) {
        let session_id = guard.session_id;
        let popup_window_handle = guard.popup_window_handle;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(PANEL_HIDE_RECHECK_DELAY_MS))
                .await;
            let _ = this.update(cx, |this, _cx| {
                if this.popup_session_id != session_id {
                    return;
                }
                let unconfirmed = this
                    .pending_panel_hides
                    .values()
                    .any(|pending| pending.session_id == session_id);
                if !unconfirmed {
                    return;
                }
                warn!(
                    event = "quick_launch.panel.hide_unconfirmed",
                    popup_window_handle, "panel hide was not confirmed in time; re-sending"
                );
                let stale: Vec<uuid::Uuid> = this
                    .pending_panel_hides
                    .iter()
                    .filter(|(_, pending)| pending.session_id == session_id)
                    .map(|(id, _)| *id)
                    .collect();
                for id in stale {
                    if let Some(pending) = this.pending_panel_hides.remove(&id) {
                        if pending.attempt < PANEL_HIDE_RETRY_LIMIT {
                            this.resend_panel_hide(pending);
                        } else {
                            error!(
                                event = "quick_launch.panel.hide_failed",
                                popup_window_handle = pending.popup_window_handle,
                                attempt = pending.attempt,
                                "quick-launch panel hide failed after retry"
                            );
                        }
                    }
                }
            });
        })
        .detach();
    }
}

fn apply_indexer_notification(
    index_status: &mut Option<IndexStatus>,
    index_search: &mut IndexSearchSession,
    notification: IndexerNotification,
) -> bool {
    match notification {
        IndexerNotification::Event(IndexerEvent::Status(status)) => {
            let changed = index_status_affects_presentation(index_status.as_ref(), &status);
            *index_status = Some(status);
            changed
        }
        IndexerNotification::Event(IndexerEvent::SearchBatch(batch)) => index_search.apply(batch),
        IndexerNotification::Event(IndexerEvent::ElevatedDirectoryHelper(_)) => false,
        IndexerNotification::Event(IndexerEvent::SearchFailed(_)) => false,
        IndexerNotification::Disconnected => {
            *index_status = None;
            index_search.invalidate();
            true
        }
    }
}
#[cfg(test)]
mod polish_tests;
#[cfg(test)]
mod retained_text_tests;
#[cfg(test)]
mod tests;

impl QuickLaunchApplication {
    pub(crate) fn set_result_entrance(
        &mut self,
        mode: nxtools_core::QuickLaunchResultEntrance,
        cx: &mut Context<Self>,
    ) {
        let correlation_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        info!(event = "quick_launch.result_entrance.save_started", %correlation_id,
            category = ?mode, "quick-launch result entrance save started");
        match AppConfig::update_section(&self.config_path, |config| {
            config.quick_launch.result_entrance = mode;
        }) {
            Ok(merged) => {
                self.config = merged;
                if let Some(popup) = self.popup.clone() {
                    cx.defer(move |cx| {
                        let _ = popup.panel.update(cx, |panel, cx| {
                            panel.result_entrance = mode;
                            panel.finish_result_entrances("setting", cx);
                            cx.notify();
                        });
                    });
                }
                info!(event = "quick_launch.result_entrance.saved", %correlation_id,
                    category = ?mode, elapsed_ms = started.elapsed().as_millis() as u64,
                    "quick-launch result entrance saved");
            }
            Err(_) => {
                warn!(event = "quick_launch.result_entrance.save_failed", %correlation_id,
                    category = "config_commit", elapsed_ms = started.elapsed().as_millis() as u64,
                    "quick-launch result entrance save failed");
                let notices = self.notifications.clone();
                cx.defer(move |cx| {
                    notices.update(cx, |notices, cx| {
                        notices.show_error(
                            "quick-launch-result-entrance",
                            "速启结果动画",
                            "保存失败，仍使用原来的结果动画。",
                            cx,
                        )
                    });
                });
            }
        }
        cx.notify();
    }

    pub(crate) fn set_row_style(
        &mut self,
        style: nxtools_core::QuickLaunchRowStyle,
        cx: &mut Context<Self>,
    ) {
        let correlation_id = uuid::Uuid::new_v4();
        let started = Instant::now();
        info!(event="quick_launch.row_style_save_started", %correlation_id, ?style);
        match AppConfig::update_section(&self.config_path, |config| {
            config.quick_launch.row_style = style
        }) {
            Ok(merged) => {
                self.config = merged;
                if let Some(popup) = self.popup.clone() {
                    cx.defer(move |cx| {
                        let _ = popup.panel.update(cx, |panel, cx| {
                            panel.row_style = style;
                            panel.dismiss_preview(cx);
                            cx.notify();
                        });
                    });
                }
                info!(event="quick_launch.row_style_saved", %correlation_id, ?style, elapsed_ms=started.elapsed().as_millis());
            }
            Err(_) => {
                warn!(event="quick_launch.row_style_save_failed", %correlation_id, reason="config_commit",elapsed_ms=started.elapsed().as_millis());
                let notices = self.notifications.clone();
                cx.defer(move |cx| {
                    notices.update(cx, |notices, cx| {
                        notices.show_error(
                            "quick-launch-row-style",
                            "速启列表设置",
                            "保存失败，仍使用原来的列表样式。",
                            cx,
                        )
                    })
                });
            }
        }
        cx.notify();
    }
}
