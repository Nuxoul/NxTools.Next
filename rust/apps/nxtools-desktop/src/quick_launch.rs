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
use gpui::{
    Animation, AnimationExt, AnyElement, AnyWindowHandle, App, Bounds, Context, Div, Entity,
    EntityInputHandler, EventEmitter, Focusable, HighlightStyle, Image, ImageFormat, ImageSource,
    IntoElement, ObjectFit, Pixels, Render, ScrollHandle, ScrollStrategy, Size, StyledText,
    Subscription, UniformListScrollHandle, WeakEntity, Window, WindowBackgroundAppearance,
    WindowBounds, WindowDecorations, WindowKind, WindowOptions, div, ease_out_quint, img, point,
    prelude::*, px, size, uniform_list,
};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::scroll::ScrollableElement;
use gpui_component::{PixelsExt, Root};
use nxtools_core::{AppConfig, ClipboardHotKey, ClipboardHotKeyModifier, QuickLaunchSettings};
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
    QueryShellSubmenuCommand, QuickLaunchPanelRequest, SetQuickLaunchHotKeyRecordingCommand,
    SetWindowBoundsCommand, SetWindowNoActivateCommand, SetWindowPositionCommand,
    SetWindowTopmostCommand, SetWindowVisibilityCommand, ShellActionItem, ShellActionMenu,
    ShellOpenAction,
};
use tracing::{error, info, warn};

use crate::icons::{MaterialIcon, shell_icon_path_is_renderable};
use crate::notifications::NotificationCenter;
use crate::runtime::{IndexerBridge, IndexerNotification};
use crate::theme::{Theme, UI_FONT_FAMILY, UI_MONO_FONT_FAMILY};
use crate::ui::components::state::{empty_state, progress_state};
use crate::ui::metrics::{
    COMPACT_RESULT_ICON_RAIL_WIDTH, COMPACT_RESULT_ROW_HEIGHT, COMPACT_RESULT_TRAILING_RAIL_WIDTH,
    RADIUS_CONTROL, SPACING_2, SPACING_6, SPACING_8, SPACING_12,
};
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, TEXT_BODY, TEXT_LABEL, TEXT_META,
};
use crate::ui::{
    PREVIEW_APPEAR_DURATION, native_window_handle, preview_media_surface, preview_shell,
};

mod file_dialog_search;
use file_dialog_search::FileDialogSearchSession;

/// Count-only index status changes are file-churn telemetry: they coalesce
/// into at most one log record per interval while structural changes always
/// log immediately.
const INDEX_COUNT_STATUS_LOG_INTERVAL: Duration = Duration::from_secs(60);

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

const PANEL_WIDTH: f32 = 620.0;
const PANEL_HEIGHT: f32 = 410.0;
const SEARCH_HEADER_HEIGHT: f32 = 42.0;
const COLLAPSED_PANEL_HEIGHT: f32 = SEARCH_HEADER_HEIGHT + 2.0;
const RESULT_LIMIT: usize = 200;
const RECENT_DIRECTORY_LIMIT: usize = 64;
const INDEX_CANDIDATE_LIMIT: usize = 200;
const ICON_PREFETCH_LIMIT: usize = 10;
const ICON_VISIBLE_OVERSCAN: usize = 2;
const ICON_RESOURCE_CACHE_LIMIT: usize = 1_024;
const ICON_RETRY_DELAY: Duration = Duration::from_millis(500);
const REQUEST_MAX_AGE_MS: i64 = 1_000;
const AUTO_QUICK_SWITCH_REQUEST_MAX_AGE_MS: i64 = 3_000;
const OPEN_GRACE: Duration = Duration::from_millis(180);
const FAILED_NAVIGATION_FOREGROUND_GUARD: Duration = Duration::from_millis(750);
const RESULTS_APPEAR_DURATION: Duration = Duration::from_millis(80);
const SEARCH_INPUT_DEBOUNCE: Duration = Duration::from_millis(65);
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
mod actions;
mod icon;
mod panel;
mod preview;
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
    platform: Rc<PlatformBridge>,
    indexer: Rc<IndexerBridge>,
    config: AppConfig,
    config_path: PathBuf,
    log_dir: PathBuf,
    notifications: Entity<NotificationCenter>,
    popup: Option<QuickLaunchWindow>,
    preview: QuickLaunchPreviewSession,
    pending_configuration: HashSet<uuid::Uuid>,
    pending_hotkey: Option<(uuid::Uuid, ClipboardHotKey)>,
    hotkey_error: Option<String>,
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
    last_local_search_us: Option<u64>,
    last_count_status_log: Option<Instant>,
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
            platform,
            indexer,
            config,
            config_path,
            log_dir,
            notifications,
            popup: None,
            preview: QuickLaunchPreviewSession::default(),
            pending_configuration: HashSet::new(),
            pending_hotkey: None,
            hotkey_error: None,
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
            last_local_search_us: None,
            last_count_status_log: None,
        }
    }

    pub(crate) fn settings(&self) -> QuickLaunchSettings {
        self.config.quick_launch.clone()
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
                let panel = self.popup.as_ref().map(|popup| popup.panel.clone());
                let notifications = self.notifications.clone();
                cx.defer(move |cx| {
                    if let Some(panel) = panel {
                        let _ = panel.update(cx, |panel, cx| {
                            panel.recent_directories.clear();
                            if panel.recent_directory_mode {
                                panel.recent_directory_mode = false;
                                panel.results.clear();
                                panel.selected = 0;
                            }
                            cx.notify();
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
        self.ensure_indexer_service(false, cx);
    }

    fn ensure_indexer_service(&mut self, automatic: bool, cx: &mut Context<Self>) {
        if self.indexer_service_bootstrap.pending.is_some() {
            return;
        }
        match self
            .platform
            .send_command(PlatformCommand::EnsureIndexerService)
        {
            Ok(command_id) => {
                self.indexer_service_bootstrap.begin(command_id, automatic);
                if !automatic {
                    self.notifications.update(cx, |notifications, cx| {
                        notifications.show_info(
                            "quick-launch-indexer-service",
                            "正在准备索引服务",
                            "首次安装会显示一次 Windows 授权窗口，主程序仍以普通权限运行。",
                            cx,
                        );
                    });
                }
            }
            Err(error) => {
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
        if let Err(error) = self.config.save_to(&self.config_path) {
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
        if let Err(error) = self.config.save_to(&self.config_path) {
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

    pub(crate) fn request_index_exclusion_directory(&mut self, cx: &mut Context<Self>) {
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
        if let Err(error) = self.config.save_to(&self.config_path) {
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
        if let IndexerNotification::Event(IndexerEvent::SearchBatch(batch)) = &notification
            && self.handle_file_dialog_search_batch(batch, cx)
        {
            return;
        }
        if matches!(&notification, IndexerNotification::Disconnected) {
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
            IndexerNotification::Event(IndexerEvent::SearchBatch(batch)) => info!(
                batch_generation = batch.generation,
                active_generation = self.index_search.generation,
                item_count = batch.items.len(),
                complete = batch.complete,
                accepted = batch.generation == self.index_search.generation,
                "quick-launch index provider batch received"
            ),
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
                if batch.generation == self.index_search.generation && batch.complete
        );
        let indexer_disconnected = matches!(&notification, IndexerNotification::Disconnected);
        if apply_indexer_notification(&mut self.index_status, &mut self.index_search, notification)
        {
            if retry_current_query {
                self.retry_index_search_after_ready(cx);
            }
            if refresh_results {
                self.refresh_indexed_results(cx);
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
        self.hotkey_error = None;
        let mut settings = self.config.quick_launch.clone();
        settings.hotkey = hotkey.clone();
        if let Some(command_id) = self.send_configuration_for(&settings) {
            self.pending_hotkey = Some((command_id, hotkey));
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
                let prepare_indexer = self.indexer_service_bootstrap.on_platform_connected();
                self.send_configuration();
                if prepare_indexer {
                    self.ensure_indexer_service(true, cx);
                }
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
            PlatformEvent::QuickLaunchPanelRequested(request) => {
                let age = nxtools_ipc::unix_time_ms().saturating_sub(request.requested_at_unix_ms);
                if age <= REQUEST_MAX_AGE_MS && !self.hotkey_recording_active() {
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
            PlatformEvent::TrayActionRequested(_)
            | PlatformEvent::ClipboardCaptured(_)
            | PlatformEvent::ClipboardPanelRequested(_)
            | PlatformEvent::TaskbarOverlayEditGestureFinished(_) => {}
        }
    }

    fn handle_auto_quick_switch_request(
        &mut self,
        request: nxtools_protocol::QuickLaunchAutoSwitchRequest,
    ) {
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
                    source_directory = %request.source_directory,
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

    fn handle_command_result(
        &mut self,
        correlation_id: uuid::Uuid,
        result: PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
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
            if result.success && result.data.as_deref() == Some("installed") {
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
                        "正在连接服务并建立 NTFS 文件索引。",
                        cx,
                    );
                } else {
                    notifications.show_error(
                        "quick-launch-indexer-service",
                        "索引服务未安装",
                        reason,
                        cx,
                    );
                }
            });
            cx.notify();
            return;
        }
        if let Some(pending) = self.pending_window_activation.remove(&correlation_id) {
            self.handle_window_activation_result(pending, &result, cx);
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
                if let Some(popup) = self.popup.clone() {
                    let _ = popup.panel.update(cx, |panel, cx| {
                        panel.set_icons(
                            entry_ids
                                .into_iter()
                                .map(|entry_id| (entry_id, path.clone())),
                            cx,
                        );
                    });
                }
            } else {
                self.record_icon_failure(resource_key, cx);
            }
            return;
        }
        if self.pending_configuration.remove(&correlation_id) {
            self.configuration_status = if result.success {
                "已启用".into()
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
            if result.success {
                self.config.quick_launch.hotkey = hotkey;
                if let Err(error) = self.config.save_to(&self.config_path) {
                    self.configuration_status = format!("热键已生效，但保存失败：{error}");
                    self.hotkey_error = Some(self.configuration_status.clone());
                } else {
                    self.configuration_status = "已启用".into();
                    self.hotkey_error = None;
                }
            } else {
                self.hotkey_error = Some(format!(
                    "热键应用失败：{}",
                    result.error.as_deref().unwrap_or("未知错误")
                ));
            }
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
                self.begin_window_activation(popup.native_window_handle);
                let _ = popup.window.update(cx, |_, window, app| {
                    let _ = popup.panel.update(app, |panel, cx| {
                        panel.set_error(
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
            if result.success {
                info!(
                    window_handle = pending.window_handle,
                    "quick-launch popup activation and topmost confirmed"
                );
            } else {
                warn!(
                    window_handle = pending.window_handle,
                    reason = result.error.as_deref().unwrap_or("unknown Windows error"),
                    "quick-launch popup stayed open after bounded topmost retries were exhausted"
                );
            }
            return;
        };
        self.queue_window_activation_step(pending.window_handle, next);
    }

    fn begin_window_activation(&mut self, window_handle: i64) {
        self.pending_window_activation.clear();
        if window_handle == 0 {
            warn!("quick-launch popup opened without a native Windows handle");
            return;
        }
        self.queue_window_activation_step(
            window_handle,
            QuickLaunchWindowActivationStep::PreTopmost(0),
        );
    }

    fn queue_window_activation_step(
        &mut self,
        window_handle: i64,
        initial_step: QuickLaunchWindowActivationStep,
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
        let command = ConfigureQuickLaunchCommand {
            enabled: settings.enabled,
            auto_quick_switch_enabled: settings.auto_quick_switch,
            file_dialog_search_enabled: settings.file_dialog_search,
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
        match self
            .platform
            .send_command(PlatformCommand::ConfigureQuickLaunch(command))
        {
            Ok(command_id) => {
                self.pending_configuration.insert(command_id);
                self.configuration_status = "正在应用".into();
                Some(command_id)
            }
            Err(PlatformBridgeError::Disconnected) => {
                self.configuration_status = "等待 Windows 适配器".into();
                None
            }
            Err(error) => {
                self.configuration_status = format!("应用失败：{error}");
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
        let indexed = indexed
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
            .collect::<Vec<_>>();
        self.catalog
            .borrow()
            .search_with_sources(
                query,
                &indexed,
                &self.config.quick_launch.custom_commands,
                RESULT_LIMIT,
            )
            .unwrap_or_else(|error| {
                error!(%error, %query, "quick-launch unified search failed");
                Vec::new()
            })
    }

    fn search_and_request_icons(
        &mut self,
        query: &str,
        cx: &mut Context<Self>,
    ) -> QuickLaunchSearchUpdate {
        let index_request = self.index_search.begin(query, INDEX_CANDIDATE_LIMIT);
        let generation = index_request.generation;
        let indexer_connected = self.indexer.is_connected();
        let index_available = index_status_has_searchable_snapshot(self.index_status.as_ref());
        let search_indexer = should_search_indexer(indexer_connected, index_available)
            && query_requests_file_index_with_commands(
                query,
                &self.config.quick_launch.custom_commands,
            );
        let search_queued = if query.trim().is_empty() || !search_indexer {
            false
        } else {
            match self
                .indexer
                .send_command(IndexerCommand::Search(index_request))
            {
                Ok(_) => true,
                Err(error) => {
                    warn!(%error, "could not queue indexed-file search");
                    false
                }
            }
        };
        info!(
            generation,
            query_length = query.chars().count(),
            indexer_connected,
            index_available,
            search_queued,
            "quick-launch query provider state"
        );
        if !should_publish_immediate_results(search_indexer, search_queued) {
            return QuickLaunchSearchUpdate {
                results: Vec::new(),
                pending: true,
            };
        }
        let local_search_started = Instant::now();
        let results = self.combined_results(query);
        self.last_local_search_us = Some(elapsed_microseconds(local_search_started));
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        // Local rows publish immediately; `pending` keeps the bounded "正在搜索"
        // hint alive until the complete index batch replaces this list.
        QuickLaunchSearchUpdate {
            results,
            pending: search_update_pending(search_queued),
        }
    }

    fn refresh_indexed_results(&mut self, cx: &mut Context<Self>) {
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
        info!(
            generation = self.index_search.generation,
            indexed_candidates = self.index_search.items.len(),
            indexed_visible,
            result_count = results.len(),
            "quick-launch merged provider results published"
        );
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        let _ = popup
            .panel
            .update(cx, |panel, cx| panel.replace_results(results, cx));
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
        let results = self.combined_results(&query);
        self.request_icons(&results[..results.len().min(ICON_PREFETCH_LIMIT)], cx);
        let _ = popup
            .panel
            .update(cx, |panel, cx| panel.replace_results(results, cx));
    }

    fn request_catalog_refresh(&mut self, cx: &mut Context<Self>) {
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
                let _ = this.update(cx, |application, cx| {
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
        if self.preview.visible
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
        let display = cx
            .displays()
            .into_iter()
            .find(|display| display.bounds().contains(&candidate.panel_bounds.origin))
            .or_else(|| cx.primary_display());
        let screen = display
            .as_ref()
            .map(|display| display.bounds())
            .unwrap_or_else(|| Bounds::new(point(px(0.0), px(0.0)), size(px(1920.0), px(1080.0))));
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
        let platform = Rc::clone(&self.platform);
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_native_slot = Rc::clone(&native_slot);
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let displayed_entry_id = payload.entry_id.clone();
        let mut image_asset = quick_launch_preview_image_asset(&payload);
        let panel_scale_factor = candidate.panel_scale_factor;
        let device_x = (bounds.origin.x.as_f32() * panel_scale_factor).round() as i32;
        let device_y = (bounds.origin.y.as_f32() * panel_scale_factor).round() as i32;
        let device_width = (preview_size.width.as_f32() * panel_scale_factor)
            .round()
            .max(1.0) as i32;
        let device_height = (preview_size.height.as_f32() * panel_scale_factor)
            .round()
            .max(1.0) as i32;
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
                display_id: display.as_ref().map(|display| display.id()),
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("NxTools.QuickLaunchPreview".into()),
                window_min_size: Some(preview_size),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let window_handle = if cfg!(test) {
                    0
                } else {
                    native_window_handle(window).unwrap_or_default()
                };
                *opened_native_slot.borrow_mut() = window_handle;
                if window_handle != 0 {
                    let _ = platform.send_command(PlatformCommand::SetWindowNoActivate(
                        SetWindowNoActivateCommand {
                            window_handle,
                            enabled: true,
                        },
                    ));
                    let _ = platform.send_command(PlatformCommand::SetWindowBounds(
                        SetWindowBoundsCommand {
                            window_handle,
                            x: device_x,
                            y: device_y,
                            width: device_width,
                            height: device_height,
                        },
                    ));
                    let _ = platform.send_command(PlatformCommand::SetWindowTopmost(
                        SetWindowTopmostCommand {
                            window_handle,
                            enabled: true,
                        },
                    ));
                    let _ = platform.send_command(PlatformCommand::SetWindowVisibility(
                        SetWindowVisibilityCommand {
                            window_handle,
                            visible: true,
                        },
                    ));
                }
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
                self.preview.visible = true;
                self.preview.displayed_entry_id = Some(displayed_entry_id);
                self.preview.image_asset = image_asset;
                self.restore_quick_launch_panel_keyboard_focus(cx);
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
        if popup.native_window_handle != 0
            && let Err(error) =
                self.platform
                    .send_command(PlatformCommand::ActivateWindow(ActivateWindowCommand {
                        window_handle: popup.native_window_handle,
                    }))
        {
            warn!(%error, "failed to restore quick-launch keyboard target after preview creation");
        }
        let _ = popup.window.update(cx, |_, window, app| {
            let _ = popup
                .panel
                .update(app, |panel, cx| panel.focus_search(window, cx));
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
        let mut stage = self
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
            QuickLaunchPreviewStagePhase::MoveOffscreen if !self.preview.visible => {
                match self
                    .platform
                    .send_command(PlatformCommand::SetWindowVisibility(
                        SetWindowVisibilityCommand {
                            window_handle: stage.window_identity.native,
                            visible: true,
                        },
                    )) {
                    Ok(command_id) => {
                        stage.command_id = command_id;
                        stage.phase = QuickLaunchPreviewStagePhase::ShowOffscreen;
                        self.preview.pending_stage = Some(stage);
                        self.preview.visible = true;
                    }
                    Err(error) => {
                        warn!(%error, "failed to show staged quick-launch preview");
                        self.hide_quick_launch_preview_window(cx);
                    }
                }
            }
            QuickLaunchPreviewStagePhase::MoveOffscreen
            | QuickLaunchPreviewStagePhase::ShowOffscreen => {
                self.preview.visible = true;
                self.render_staged_quick_launch_preview(stage, cx);
            }
            QuickLaunchPreviewStagePhase::RevealOnscreen => {
                let _ = self
                    .platform
                    .send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                        window_handle: stage.window_identity.native,
                        enabled: true,
                    }));
                self.preview.visible = true;
                self.preview.displayed_entry_id = Some(stage.entry_id);
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
                let _ = application.update(app, |application, cx| {
                    if application
                        .preview
                        .window
                        .as_ref()
                        .is_none_or(|preview| preview.identity != window_identity)
                        || application.preview.generation != generation
                        || application.preview.hovered_entry_id.as_deref()
                            != Some(entry_id.as_str())
                    {
                        return;
                    }
                    let x = (bounds.origin.x.as_f32() * panel_scale_factor).round() as i32;
                    let y = (bounds.origin.y.as_f32() * panel_scale_factor).round() as i32;
                    match application
                        .platform
                        .send_command(PlatformCommand::SetWindowPosition(
                            SetWindowPositionCommand {
                                window_handle: window_identity.native,
                                x,
                                y,
                            },
                        )) {
                        Ok(command_id) => {
                            application.preview.pending_stage =
                                Some(QuickLaunchPreviewPendingStage {
                                    command_id,
                                    phase: QuickLaunchPreviewStagePhase::RevealOnscreen,
                                    window_identity,
                                    generation,
                                    entry_id,
                                    payload: None,
                                    preview_size,
                                    bounds,
                                    panel_scale_factor,
                                });
                        }
                        Err(error) => {
                            warn!(%error, "failed to reveal refreshed quick-launch preview");
                            application.hide_quick_launch_preview_window(cx);
                        }
                    }
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
                    warn!(%error, %resource_key, "could not request a quick-launch icon");
                    self.record_icon_failure(resource_key, cx);
                }
            }
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
            let _ = this.update(cx, |application, cx| {
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
        let started_at = Instant::now();
        let action_correlation_id = uuid::Uuid::new_v4();
        let execute_session_id = self.popup_session_id;
        let owner_window_handle = self
            .popup
            .as_ref()
            .map_or(0, |popup| popup.native_window_handle);
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
                    panel.set_error("系统工具没有可定位的快捷方式".into(), cx);
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
                    "quick-launch action accepted"
                );
                // Hide immediately after dispatch: waiting for the completion
                // result made Enter feel slow on slow shell launches (measured
                // 1-2s). The completion handler still records activation and
                // shows errors — the panel just isn't visible for it.
                if execute_session_id == self.popup_session_id {
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
                        panel.set_error(format!("Windows 适配器不可用：{error}"), cx);
                    });
                }
            }
        }
    }

    fn toggle_panel(&mut self, request: QuickLaunchPanelRequest, cx: &mut Context<Self>) {
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
        let scale = request.cursor_monitor_dpi.max(96) as f32 / 96.0;
        let _ = self
            .platform
            .send_command(PlatformCommand::SetWindowPosition(
                SetWindowPositionCommand {
                    window_handle: popup.native_window_handle,
                    x: (bounds.origin.x.as_f32() * scale).round() as i32,
                    y: (bounds.origin.y.as_f32() * scale).round() as i32,
                },
            ));
        let _ = self
            .platform
            .send_command(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle: popup.native_window_handle,
                    visible: true,
                },
            ));
        let _ = popup.window.update(cx, |_, window, app| {
            window.resize(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT)));
            let _ = popup.panel.update(app, |panel, cx| {
                panel.prepare_for_show(
                    request.target,
                    request.directory_navigation_context,
                    recent_directories,
                    results,
                    window,
                    cx,
                );
            });
        });
        self.begin_window_activation(popup.native_window_handle);
        self.schedule_captured_directory_record(captured_navigation_context.as_ref(), cx);
        info!(
            window_handle = popup.native_window_handle,
            "quick-launch popup reused"
        );
        self.request_catalog_refresh(cx);
    }

    fn open_popup(&mut self, request: QuickLaunchPanelRequest, cx: &mut Context<Self>) {
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
        let application = cx.entity().downgrade();
        let target = request.target;
        let navigation_context = request.directory_navigation_context;
        let captured_navigation_context = navigation_context.clone();
        let panel_slot = Rc::new(RefCell::new(None));
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_panel_slot = Rc::clone(&panel_slot);
        let opened_native_slot = Rc::clone(&native_slot);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: None,
                focus: false,
                show: true,
                kind: WindowKind::PopUp,
                is_movable: false,
                is_resizable: false,
                is_minimizable: false,
                display_id,
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("NxTools.QuickLaunch".into()),
                window_min_size: Some(size(px(PANEL_WIDTH), px(COLLAPSED_PANEL_HEIGHT))),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                *opened_native_slot.borrow_mut() = native_window_handle(window).unwrap_or_default();
                let panel = cx.new(|cx| {
                    QuickLaunchPanel::new(
                        results,
                        target,
                        navigation_context,
                        recent_directories,
                        application,
                        window,
                        cx,
                    )
                });
                *opened_panel_slot.borrow_mut() = Some(panel.downgrade());
                cx.new(|cx| Root::new(panel, window, cx))
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
                let native_window_handle = popup.native_window_handle;
                self.popup = Some(popup);
                self.begin_window_activation(native_window_handle);
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
            let _ = this.update(cx, |application, cx| {
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
        self.popup_session_id = uuid::Uuid::new_v4();
        self.active_summon = None;
        self.navigation_foreground_guard = None;
        self.pending_window_activation.clear();
        self.dismiss_quick_launch_preview(cx);
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let _ = popup.panel.update(cx, |panel, cx| {
            panel.is_open = false;
            cx.notify();
        });
        info!(
            event = "quick_launch.popup.hide_requested",
            popup_window_handle = popup.native_window_handle,
            restore_window_handle = ?restore_window_handle,
            "quick-launch hide commands queued"
        );
        let commands = quick_launch_dismiss_commands(
            popup.native_window_handle,
            restore_window_handle,
        );
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
                        self.pending_panel_hides
                            .insert(command_id, guard);
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
    fn schedule_panel_hide_recheck(
        &mut self,
        guard: PendingPanelHide,
        cx: &mut Context<Self>,
    ) {
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
                    popup_window_handle,
                    "panel hide was not confirmed in time; re-sending"
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
        IndexerNotification::Disconnected => {
            *index_status = None;
            index_search.invalidate();
            true
        }
    }
}
#[cfg(test)]
mod tests;
