use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    AnyWindowHandle, AppContext, Bounds, Context, Entity, Global, Pixels, Point, Size, WeakEntity,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, point,
    px, size,
};
use gpui_component::{PixelsExt, Root};
use nxtools_clipboard::{
    BUILTIN_ALL_CATEGORY_ID, CaptureSource, ClipboardAction, ClipboardCapturePolicy,
    ClipboardCategory, ClipboardDatabaseBackupReport, ClipboardDatabaseCompactionReport,
    ClipboardHistory, ClipboardHistoryCounts, ClipboardHistoryError, ClipboardHistoryPage,
    ClipboardItem, ClipboardKind, ClipboardNativeFormat as StoredClipboardNativeFormat,
    ClipboardOpenTargetKind, ClipboardRetentionPolicy, ClipboardSearchScope, FilesCapture,
    ImageCapture, TextCapture, action_text_payload,
};
use nxtools_core::{
    AppConfig, ClipboardClickAction, ClipboardClickModifier, ClipboardHotKey,
    ClipboardHotKeyModifier, ClipboardImagePasteProfile, ClipboardPanelSize, ClipboardSettings,
    clipboard_image_paste_profile,
};
use nxtools_ipc::{PlatformBridge, PlatformBridgeError, PlatformNotification};
use nxtools_protocol::{
    BeginWindowMoveCommand, BeginWindowResizeCommand, ClipboardArchivePickerMode, ClipboardCapture,
    ClipboardNativeFormat as ProtocolClipboardNativeFormat, ClipboardPanelRequest,
    ConfigureClipboardCommand, ForegroundContext, OpenShellTargetCommand, PasteFilesCommand,
    PasteImageCommand, PasteNativeFormatsCommand, PasteTextCommand,
    PickClipboardArchivePathCommand, PickClipboardStorageDirectoryCommand, PlatformCommand,
    PlatformEvent, SetClipboardHotKeyRecordingCommand, SetWindowPositionCommand,
    SetWindowTopmostCommand, SetWindowVisibilityCommand, ShellOpenAction,
};
use sha2::{Digest, Sha256};
use tracing::{error, info, warn};

use crate::notifications::{NotificationAction, NotificationCenter, NotificationDetail};
use crate::popup_lifecycle::{PopupLifecycle, TopmostResolution};
use crate::ui::{
    ClipboardPanel, ClipboardPreview, PreviewLayout, native_window_handle, panel_bounds,
    preview_layout,
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
const PREVIEW_STAGING_COORDINATE: i32 = -32_000;
const RETENTION_INTERVAL_MS: i64 = 60 * 60 * 1000;
const ARCHIVE_NOTIFICATION_ID: &str = "clipboard.archive";
const DATABASE_NOTIFICATION_ID: &str = "clipboard.database";
const STORAGE_NOTIFICATION_ID: &str = "clipboard.storage";

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

fn map_source(source: ForegroundContext, process_icon_path: String) -> CaptureSource {
    CaptureSource {
        window_handle: source.window_handle,
        process_id: source.process_id,
        process_name: source.process_name,
        process_icon_path,
        window_title: source.window_title,
    }
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
}

impl Global for ClipboardApplicationGlobal {}

pub struct ClipboardApplication {
    history: Rc<RefCell<ClipboardHistory>>,
    clipboard_asset_dir: PathBuf,
    platform: Rc<PlatformBridge>,
    config: AppConfig,
    config_path: PathBuf,
    notifications: Entity<NotificationCenter>,
    panel_size_generation: u64,
    last_cleanup_at_unix_ms: i64,
    pending_configuration: HashSet<uuid::Uuid>,
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
    popup: PopupSession,
    preview: PreviewSession,
    link_panel: crate::tools::link_panel::LinkPanelSession,
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
}

#[derive(Default)]
struct PopupSession {
    lifecycle: PopupLifecycle<PopupWindow>,
    pending_visibility: HashMap<uuid::Uuid, AnyWindowHandle>,
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
    MoveOffscreen,
    ShowOffscreen,
}

struct PendingPreviewStage {
    command_id: uuid::Uuid,
    phase: PreviewStagePhase,
    generation: u64,
    item: ClipboardItem,
    layout: PreviewLayout,
    preview_size: Size<Pixels>,
    bounds: Bounds<Pixels>,
    panel_scale_factor: f32,
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
    Bounds::new(point(x, y), preview_size)
}

fn preview_device_origin(bounds: Bounds<Pixels>, scale_factor: f32) -> (i32, i32) {
    let device_bounds = bounds.to_device_pixels(scale_factor);
    (device_bounds.origin.x.into(), device_bounds.origin.y.into())
}

impl ClipboardApplication {
    pub fn new(
        history: ClipboardHistory,
        clipboard_asset_dir: PathBuf,
        platform: Rc<PlatformBridge>,
        config: AppConfig,
        config_path: PathBuf,
        notifications: Entity<NotificationCenter>,
        _cx: &mut Context<Self>,
    ) -> Self {
        let mut application = Self {
            history: Rc::new(RefCell::new(history)),
            clipboard_asset_dir,
            platform,
            config,
            config_path,
            notifications,
            panel_size_generation: 0,
            last_cleanup_at_unix_ms: 0,
            pending_configuration: HashSet::new(),
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
            popup: PopupSession::default(),
            preview: PreviewSession::default(),
            link_panel: crate::tools::link_panel::LinkPanelSession::default(),
        };
        application.apply_retention(nxtools_ipc::unix_time_ms());
        application
    }

    pub(crate) fn query_entries_page(
        &self,
        query: &str,
        category_id: &str,
        search_scope: ClipboardSearchScope,
        offset: usize,
    ) -> Result<ClipboardHistoryPage, ClipboardHistoryError> {
        self.history.borrow().query_category_page(
            query,
            category_id,
            search_scope,
            offset,
            HISTORY_PAGE_SIZE,
        )
    }

    pub(crate) fn clipboard_counts(&self) -> Result<ClipboardHistoryCounts, ClipboardHistoryError> {
        self.history.borrow().counts()
    }

    pub(crate) fn clipboard_settings(&self) -> ClipboardSettings {
        self.config.clipboard.clone()
    }

    pub(crate) fn clipboard_configuration_status(&self) -> &str {
        &self.clipboard_configuration_status
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
        message: impl Into<gpui::SharedString>,
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
        message: impl Into<gpui::SharedString>,
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
        message: impl Into<gpui::SharedString>,
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
        message: impl Into<gpui::SharedString>,
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
        message: impl Into<gpui::SharedString>,
        cx: &mut Context<Self>,
    ) {
        let message = message.into();
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_error(id, title, message, cx);
        });
    }

    pub(crate) fn clipboard_storage_directory(&self) -> String {
        self.history
            .borrow()
            .database_path()
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_string_lossy()
            .into_owned()
    }

    pub(crate) fn request_clipboard_export(&mut self, cx: &mut Context<Self>) {
        self.request_clipboard_archive_path(ClipboardArchiveOperation::Export, cx);
    }

    pub(crate) fn request_clipboard_import(&mut self, cx: &mut Context<Self>) {
        self.request_clipboard_archive_path(ClipboardArchiveOperation::Import, cx);
    }

    pub(crate) fn request_clipboard_database_backup(&mut self, cx: &mut Context<Self>) {
        self.start_clipboard_database_operation(ClipboardDatabaseOperation::Backup, cx);
    }

    pub(crate) fn request_clipboard_database_compaction(&mut self, cx: &mut Context<Self>) {
        self.start_clipboard_database_operation(ClipboardDatabaseOperation::Compact, cx);
    }

    pub(crate) fn request_clipboard_storage_migration(&mut self, cx: &mut Context<Self>) {
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
                PickClipboardStorageDirectoryCommand { current_directory },
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
        cx: &mut Context<Self>,
    ) {
        if self.clipboard_management_busy() {
            self.notify_info(
                ARCHIVE_NOTIFICATION_ID,
                "历史归档",
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
                },
            )) {
            Ok(command_id) => {
                self.pending_archive_picker = Some((command_id, operation));
            }
            Err(error) => {
                self.notify_error(
                    ARCHIVE_NOTIFICATION_ID,
                    "历史归档",
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
        let database_path = self.history.borrow().database_path().to_owned();
        let asset_root = self.clipboard_asset_dir.clone();
        self.management_operation_active = true;
        self.notify_progress(
            ARCHIVE_NOTIFICATION_ID,
            "历史归档",
            match operation {
                ClipboardArchiveOperation::Export => "正在导出便携归档…",
                ClipboardArchiveOperation::Import => "正在导入并合并历史…",
            },
            cx,
        );
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let mut history = ClipboardHistory::open(&database_path)?;
                    match operation {
                        ClipboardArchiveOperation::Export => history
                            .export_archive(&path)
                            .map(|report| {
                                let file_name = path
                                    .file_name()
                                    .map(|name| name.to_string_lossy())
                                    .unwrap_or_else(|| path.to_string_lossy());
                                format!(
                                    "已导出 {} 条记录、{} 个资源 · {}",
                                    report.item_count, report.asset_count, file_name
                                )
                            })
                            .map_err(anyhow::Error::from),
                        ClipboardArchiveOperation::Import => history
                            .import_archive(&path, &asset_root)
                            .map(|report| {
                                format!(
                                    "导入完成：新增 {}，合并 {}，跳过 {}",
                                    report.inserted, report.updated, report.skipped
                                )
                            })
                            .map_err(anyhow::Error::from),
                    }
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok(message) => {
                        this.notify_success(
                            ARCHIVE_NOTIFICATION_ID,
                            "历史归档",
                            message.clone(),
                            cx,
                        );
                    }
                    Err(error) => {
                        let message = format!("数据操作失败：{error}");
                        this.notify_error(ARCHIVE_NOTIFICATION_ID, "历史归档", message.clone(), cx);
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
        let database_path = self.history.borrow().database_path().to_owned();
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
                    let mut history = ClipboardHistory::open(&database_path)?;
                    match operation {
                        ClipboardDatabaseOperation::Backup => history
                            .create_database_backup(&backup_directory)
                            .map(ClipboardDatabaseOperationResult::Backup)
                            .map_err(anyhow::Error::from),
                        ClipboardDatabaseOperation::Compact => history
                            .compact_database(&backup_directory)
                            .map(ClipboardDatabaseOperationResult::Compact)
                            .map_err(anyhow::Error::from),
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
        let database_path = self.history.borrow().database_path().to_owned();
        let source_asset_root = self.clipboard_asset_dir.clone();
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
                    let source = ClipboardHistory::open(&database_path)?;
                    let report = source.migrate_storage(&source_asset_root, &destination)?;
                    let history = ClipboardHistory::open(&report.database_path)?;
                    Ok::<_, anyhow::Error>((history, report))
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                match result {
                    Ok((history, report)) => {
                        let mut candidate = this.config.clone();
                        candidate.clipboard.storage_directory = report
                            .database_path
                            .parent()
                            .unwrap_or_else(|| Path::new("."))
                            .to_string_lossy()
                            .into_owned();
                        match candidate.save_to(&this.config_path) {
                            Ok(()) => {
                                this.config = candidate;
                                this.history = Rc::new(RefCell::new(history));
                                this.clipboard_asset_dir = report.asset_root.clone();
                                let message = format!(
                                    "迁移完成：{} 条记录、{} 个资源；原目录已保留 · {}",
                                    report.item_count,
                                    report.asset_count,
                                    report.database_path.display()
                                );
                                this.notify_success(
                                    STORAGE_NOTIFICATION_ID,
                                    "存储迁移",
                                    message.clone(),
                                    cx,
                                );
                            }
                            Err(error) => {
                                let message = format!(
                                    "迁移数据已生成，但配置保存失败，仍使用原目录：{error}"
                                );
                                this.notify_error(
                                    STORAGE_NOTIFICATION_ID,
                                    "存储迁移",
                                    message.clone(),
                                    cx,
                                );
                            }
                        }
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

    /// 链接识别: after each text capture, extract links and show the floating
    /// panel near the pointer. Reuses the preview window lifecycle pattern:
    /// show at computed bounds, one auto-hide timer, generation-guarded.
    pub(crate) fn handle_link_detector_capture(&mut self, text: &str, cx: &mut Context<Self>) {
        if !self.config.clipboard.link_detector_enabled {
            return;
        }
        let links = crate::tools::link_detector::detect_links(text);
        if links.is_empty() {
            return;
        }
        info!(
            event = "tools.link_detector.matches_found",
            count = links.len(),
            "link detector recognized clipboard targets"
        );
        self.show_link_panel(links, cx);
    }

    pub(crate) fn close_link_panel(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = self.link_panel.window.take() {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        }
        self.link_panel.generation += 1;
    }

    pub(crate) fn show_link_panel(&mut self, links: Vec<crate::tools::link_panel::DetectedLinkRow>, cx: &mut Context<Self>) {
        let application = cx.entity().downgrade();
        let panel_size = size(
            px(crate::tools::link_panel::LINK_PANEL_WIDTH),
            px(crate::tools::link_panel::link_panel_height(links.len())),
        );
        let bounds = Bounds::new(point(px(120.0), px(120.0)), panel_size);
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
                display_id: None,
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("NxTools.LinkPanel".into()),
                window_min_size: None,
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let native = native_window_handle(window);
                let panel = cx.new(|_| crate::tools::link_panel::LinkPanelView {
                    links,
                    application,
                    expanded: false,
                });
                let _ = native;
                cx.new(|cx| Root::new(panel, window, cx))
            },
        );
        if let Ok(window) = opened {
            self.link_panel.window = Some(window.into());
            // Auto-hide: the panel always dismisses after the dwell time; a
            // fresh capture replaces the window wholesale.
            let generation = self.link_panel.generation.wrapping_add(1);
            self.link_panel.generation = generation;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(Duration::from_millis(
                        crate::tools::link_panel::LINK_PANEL_AUTO_HIDE_MS,
                    ))
                    .await;
                let _ = this.update(cx, |this, cx| {
                    if this.link_panel.generation != generation {
                        return;
                    }
                    if let Some(window) = this.link_panel.window.take() {
                        let _ = window.update(cx, |_, window, _| window.remove_window());
                    }
                });
            })
            .detach();
        }
    }

    pub(crate) fn set_link_detector_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        if self.config.clipboard.link_detector_enabled == enabled {
            return;
        }
        self.config.clipboard.link_detector_enabled = enabled;
        if let Err(error) = self.config.save_to(&self.config_path) {
            error!(%error, "could not persist the link detector preference");
            return;
        }
        info!(event = "tools.link_detector_preference_saved", enabled);
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
        let mut candidate = self.config.clipboard.clone();
        candidate.hotkey = hotkey.clone();
        if let Some(command_id) = self.send_clipboard_configuration_for(&candidate) {
            self.pending_hotkey_configuration = Some((command_id, hotkey));
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
        self.config.clipboard.normalize();
        let _ = self.persist_clipboard_configuration();
        self.apply_retention(nxtools_ipc::unix_time_ms());
    }

    pub(crate) fn set_clipboard_excluded_processes(&mut self, text: &str) {
        self.config.clipboard.excluded_processes = text
            .split([';', ',', '\n', '\r'])
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect();
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
        if let Err(error) = self.config.save_to(&self.config_path) {
            self.clipboard_configuration_status = format!("保存失败：{error}");
            error!(%error, "failed to persist clipboard configuration");
            false
        } else {
            true
        }
    }

    fn send_clipboard_configuration(&mut self) {
        let settings = self.config.clipboard.clone();
        let _ = self.send_clipboard_configuration_for(&settings);
    }

    fn send_clipboard_configuration_for(
        &mut self,
        settings: &ClipboardSettings,
    ) -> Option<uuid::Uuid> {
        match self
            .platform
            .send_command(PlatformCommand::ConfigureClipboard(
                ConfigureClipboardCommand {
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
                },
            )) {
            Ok(command_id) => {
                self.pending_configuration.insert(command_id);
                self.clipboard_configuration_status = "正在应用".into();
                Some(command_id)
            }
            Err(PlatformBridgeError::Disconnected) => {
                self.clipboard_configuration_status = "等待 Windows 适配器".into();
                None
            }
            Err(error) => {
                self.clipboard_configuration_status = format!("应用失败：{error}");
                None
            }
        }
    }

    fn apply_retention(&mut self, now_unix_ms: i64) {
        let settings = &self.config.clipboard;
        match self.history.borrow_mut().cleanup(
            ClipboardRetentionPolicy {
                max_items: settings.max_items,
                retention_days: settings.retention_days,
            },
            now_unix_ms,
        ) {
            Ok(report) => {
                for path in &report.purged_asset_paths {
                    remove_managed_clipboard_asset(&self.clipboard_asset_dir, path);
                }
                self.last_cleanup_at_unix_ms = now_unix_ms;
                if report.soft_deleted > 0 || report.purged > 0 {
                    info!(
                        soft_deleted = report.soft_deleted,
                        purged = report.purged,
                        "clipboard retention applied"
                    );
                }
            }
            Err(error) => error!(%error, "failed to apply clipboard retention"),
        }
    }

    fn capture_policy(&self) -> ClipboardCapturePolicy {
        ClipboardCapturePolicy {
            enabled: self.config.clipboard.enabled,
            capture_images: self.config.clipboard.capture_images,
            capture_files: self.config.clipboard.capture_files,
        }
    }

    fn is_process_excluded(&self, process_name: &str) -> bool {
        fn normalized_process_name(value: &str) -> String {
            let value = value.trim().to_ascii_lowercase();
            value.strip_suffix(".exe").unwrap_or(&value).to_owned()
        }

        let normalized = normalized_process_name(process_name);
        self.config
            .clipboard
            .excluded_processes
            .iter()
            .any(|excluded| normalized_process_name(excluded) == normalized)
    }

    pub(crate) fn record_panel_size(&mut self, width: f32, height: f32, cx: &mut Context<Self>) {
        let size = ClipboardPanelSize::clamped(
            width.round().max(0.0) as u32,
            height.round().max(0.0) as u32,
        );
        if self.config.clipboard_panel == size {
            return;
        }
        self.config.clipboard_panel = size;
        self.panel_size_generation = self.panel_size_generation.wrapping_add(1);
        let generation = self.panel_size_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(250))
                .await;
            let _ = this.update(cx, |this, _cx| {
                if this.panel_size_generation != generation {
                    return;
                }
                if let Err(error) = this.config.save_to(&this.config_path) {
                    error!(%error, "failed to persist clipboard panel size");
                } else {
                    info!(
                        width = this.config.clipboard_panel.width,
                        height = this.config.clipboard_panel.height,
                        "clipboard panel size persisted"
                    );
                }
            });
        })
        .detach();
    }

    pub(crate) fn categories(&self) -> Result<Vec<ClipboardCategory>, ClipboardHistoryError> {
        self.history.borrow().categories()
    }

    pub(crate) fn create_category(
        &self,
        name: &str,
    ) -> Result<ClipboardCategory, ClipboardHistoryError> {
        self.history.borrow_mut().create_category(name)
    }

    pub(crate) fn rename_category(
        &self,
        category_id: &str,
        name: &str,
    ) -> Result<ClipboardCategory, ClipboardHistoryError> {
        self.history.borrow_mut().rename_category(category_id, name)
    }

    pub(crate) fn delete_category(&self, category_id: &str) -> Result<(), ClipboardHistoryError> {
        self.history.borrow_mut().delete_category(category_id)
    }

    pub(crate) fn reorder_categories(
        &self,
        ordered_ids: &[String],
    ) -> Result<Vec<ClipboardCategory>, ClipboardHistoryError> {
        self.history.borrow_mut().reorder_categories(ordered_ids)
    }

    pub(crate) fn add_category_source(
        &self,
        category_id: &str,
        process_name: &str,
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        self.history
            .borrow_mut()
            .add_category_source(category_id, process_name)
    }

    pub(crate) fn category_sources(
        &self,
        category_id: &str,
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        self.history.borrow().category_sources(category_id)
    }

    pub(crate) fn remove_category_source(
        &self,
        category_id: &str,
        process_name: &str,
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        self.history
            .borrow_mut()
            .remove_category_source(category_id, process_name)
    }

    pub(crate) fn set_item_favorite(
        &self,
        item_id: i64,
        value: bool,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.history.borrow_mut().set_favorite(item_id, value)
    }

    pub(crate) fn set_item_pinned(
        &self,
        item_id: i64,
        value: bool,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.history.borrow_mut().set_pinned(item_id, value)
    }

    pub(crate) fn delete_item(&self, item_id: i64) -> Result<(), ClipboardHistoryError> {
        self.history
            .borrow_mut()
            .soft_delete(item_id, nxtools_ipc::unix_time_ms())
    }

    pub(crate) fn restore_item(
        &self,
        item_id: i64,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.history.borrow_mut().restore(item_id)
    }

    pub(crate) fn mark_item_used(
        &self,
        item_id: i64,
    ) -> Result<nxtools_clipboard::ClipboardUsage, ClipboardHistoryError> {
        self.history
            .borrow_mut()
            .mark_used(item_id, nxtools_ipc::unix_time_ms())
    }

    pub(crate) fn update_item_text(
        &self,
        item_id: i64,
        text: &str,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.history.borrow_mut().update_text(item_id, text)
    }

    pub(crate) fn request_clipboard_action(
        &self,
        entry: &ClipboardItem,
        target: &ForegroundContext,
        action: ClipboardAction,
    ) -> anyhow::Result<uuid::Uuid> {
        let native_formats = if matches!(
            action,
            ClipboardAction::OriginalPaste | ClipboardAction::CopyOnly
        ) {
            self.history.borrow().native_formats(entry.id)?
        } else {
            Vec::new()
        };
        let command = command_for_action(entry, target, action, native_formats)?;
        let command_id = self.platform.send_command(command)?;
        info!(item_id = entry.id, ?action, %command_id, target = %target.process_name, "clipboard action command sent");
        Ok(command_id)
    }

    pub(crate) fn hide_popup(
        &mut self,
        popup_window: AnyWindowHandle,
        native_window_handle: i64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let command_id = self
            .platform
            .send_command(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle: native_window_handle,
                    visible: false,
                },
            ))?;
        self.popup
            .pending_visibility
            .insert(command_id, popup_window);
        info!(%command_id, window_handle = native_window_handle, "clipboard popup hide command sent");
        Ok(command_id)
    }

    /// Re-asserts the open panel's topmost band after a transient foreground
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
        if entered {
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
        let generation = self.preview.generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(PREVIEW_HOVER_DELAY).await;
            let _ = this.update(cx, |this, cx| {
                if this.preview.generation == generation
                    && !this.preview.scroll_suppressed
                    && this.preview.hovered_item_id == Some(candidate.item.id)
                {
                    this.show_preview(candidate, cx);
                }
            });
        })
        .detach();
    }

    pub(crate) fn suspend_preview_for_scroll(&mut self, cx: &mut Context<Self>) {
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

    pub(crate) fn dismiss_preview(&mut self, _cx: &mut Context<Self>) {
        self.preview.hovered_item_id = None;
        self.preview.candidate = None;
        self.preview.pointer_inside = false;
        self.preview.generation = self.preview.generation.wrapping_add(1);
        self.hide_preview_window();
    }

    fn hide_preview_window(&mut self) {
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
        if !success {
            error!(
                %correlation_id,
                reason = error_message.unwrap_or("unknown Windows error"),
                phase = ?stage.phase,
                "failed to prepare clipboard preview"
            );
            self.hide_preview_window();
            return true;
        }
        if self.preview.generation != stage.generation
            || self.preview.hovered_item_id != Some(stage.item.id)
            || self.preview.scroll_suppressed
        {
            self.hide_preview_window();
            return true;
        }

        match stage.phase {
            PreviewStagePhase::MoveOffscreen if !self.preview.visible => {
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
            PreviewStagePhase::MoveOffscreen | PreviewStagePhase::ShowOffscreen => {
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
            window.resize(stage.preview_size);
            if view
                .update(app, |preview, cx| {
                    preview.set_item(stage.item, stage.layout, cx)
                })
                .is_err()
            {
                return;
            }
            window.on_next_frame(move |_, app| {
                let _ = application.update(app, |application, _cx| {
                    if application.preview.generation != generation
                        || application.preview.hovered_item_id != Some(requested_item_id)
                        || application.preview.scroll_suppressed
                        || application.preview.native_window_handle != native_window_handle
                    {
                        application.hide_preview_window();
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
        let PreviewCandidate {
            item,
            panel_bounds,
            pointer_position,
            panel_scale_factor,
        } = candidate;
        let display = cx
            .displays()
            .into_iter()
            .find(|display| display.bounds().contains(&panel_bounds.origin))
            .or_else(|| cx.primary_display());
        let screen = display
            .as_ref()
            .map(|display| display.bounds())
            .unwrap_or_else(|| Bounds::new(point(px(0.0), px(0.0)), size(px(1920.0), px(1080.0))));
        let layout = preview_layout(
            &item,
            screen.size.width.as_f32(),
            screen.size.height.as_f32(),
        );
        let preview_size = size(px(layout.window_width), px(layout.window_height));
        let bounds = preview_bounds(panel_bounds, preview_size, screen, pointer_position);

        if self.preview.window.is_some() && self.preview.native_window_handle != 0 {
            let stage = PendingPreviewStage {
                command_id: uuid::Uuid::nil(),
                phase: PreviewStagePhase::MoveOffscreen,
                generation: self.preview.generation,
                item,
                layout,
                preview_size,
                bounds,
                panel_scale_factor,
            };
            self.stage_reused_preview(stage);
            return;
        }
        self.preview.window = None;
        self.preview.native_window_handle = 0;
        self.preview.visible = false;
        self.preview.displayed_item_id = None;

        let application = cx.entity().downgrade();
        let platform = Rc::clone(&self.platform);
        let native_window_slot = Rc::new(RefCell::new(None));
        let opened_native_window_slot = Rc::clone(&native_window_slot);
        let preview_view_slot = Rc::new(RefCell::new(None));
        let opened_preview_view_slot = Rc::clone(&preview_view_slot);
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
                display_id: display.as_ref().map(|display| display.id()),
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.ClipboardPreview".into()),
                window_min_size: None,
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let preview_window_handle = native_window_handle(window);
                *opened_native_window_slot.borrow_mut() = preview_window_handle;
                if let Some(window_handle) = preview_window_handle {
                    let _ = platform.send_command(PlatformCommand::SetWindowTopmost(
                        SetWindowTopmostCommand {
                            window_handle,
                            enabled: true,
                        },
                    ));
                }
                let preview = cx.new(|_cx| ClipboardPreview::new(item, layout, application));
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
                    self.preview.visible = true;
                    self.preview.displayed_item_id = self.preview.hovered_item_id;
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
    pub(crate) fn handle_platform_notification(
        &mut self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        match notification {
            PlatformNotification::Connected => {
                // A reconnect invalidates every command that was pending on the previous pipe.
                // Keep only values already confirmed and persisted by the Windows adapter.
                self.pending_configuration.clear();
                self.pending_hotkey_configuration = None;
                self.pending_hotkey_recording = None;
                if self.pending_archive_picker.take().is_some() {
                    self.notify_error(
                        ARCHIVE_NOTIFICATION_ID,
                        "历史归档",
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
                cx.notify();
            }
            PlatformNotification::Event(event) => self.handle_platform_event(event, cx),
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => {
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
                        self.notify_error(ARCHIVE_NOTIFICATION_ID, "历史归档", message, cx);
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
                    let hotkey_result = resolve_pending_hotkey(
                        &mut self.pending_hotkey_configuration,
                        correlation_id,
                        result.success,
                    );
                    if result.success {
                        if let Some(Ok(hotkey)) = hotkey_result {
                            self.config.clipboard.hotkey = hotkey;
                            if self.persist_clipboard_configuration() {
                                self.clipboard_configuration_status = "已应用".into();
                            }
                        } else if self.pending_hotkey_configuration.is_none() {
                            self.clipboard_configuration_status = "已应用".into();
                        }
                    } else if self.pending_hotkey_configuration.is_none() {
                        self.clipboard_configuration_status = format!(
                            "应用失败：{}",
                            result.error.as_deref().unwrap_or("未知 Windows 错误")
                        );
                    }
                    if self.pending_hotkey_configuration.is_none()
                        && self.clipboard_configuration_dirty
                    {
                        self.clipboard_configuration_dirty = false;
                        self.send_clipboard_configuration();
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
                if let Some(pending_window) = self.popup.pending_visibility.remove(&correlation_id)
                {
                    if result.success {
                        info!(%correlation_id, "clipboard popup hidden without closing its GPUI window");
                    } else {
                        let reason = result
                            .error
                            .as_deref()
                            .unwrap_or("unknown Windows error")
                            .to_owned();
                        error!(
                            %correlation_id,
                            %reason,
                            "failed to hide clipboard popup"
                        );
                        if let Some(popup) = self
                            .popup
                            .lifecycle
                            .current()
                            .filter(|popup| popup.window == pending_window)
                            .cloned()
                        {
                            let _ = popup.window.update(cx, |_, window, app| {
                                let _ = popup.panel.update(app, |panel, cx| {
                                    panel.handle_hide_failure(reason, window, cx);
                                });
                            });
                        }
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
                    match resolution {
                        TopmostResolution::Confirmed { previous } => {
                            info!(%correlation_id, "clipboard window is topmost");
                            if let Some(previous) = previous {
                                let _ = previous
                                    .window
                                    .update(cx, |_, window, _| window.remove_window());
                            }
                        }
                        TopmostResolution::Rejected { previous } => {
                            warn!(
                                %correlation_id,
                                reason = %topmost_reason,
                                "clipboard window remains open because topmost is nonfatal"
                            );
                            if let Some(previous) = previous {
                                let _ = previous
                                    .window
                                    .update(cx, |_, window, _| window.remove_window());
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
                if self.management_operation_active {
                    self.deferred_captures.push(capture);
                    info!(
                        queued = self.deferred_captures.len(),
                        "clipboard capture deferred during storage operation"
                    );
                    return;
                }
                let captured_at_unix_ms = capture.captured_at_unix_ms;
                if self.is_process_excluded(&capture.source.process_name) {
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
                let native_formats = self.ingest_native_formats(capture.native_formats);
                let process_icon_path = capture
                    .source_icon_staging_path
                    .as_deref()
                    .and_then(|path| self.store_source_icon(path).ok())
                    .unwrap_or_default();
                let source = map_source(capture.source, process_icon_path);
                if !capture.paths.is_empty() {
                    if let Some(image) = capture.image {
                        let _ = std::fs::remove_file(image.staging_path);
                    }
                    if let Err(error) = self.history.borrow_mut().capture_files(FilesCapture {
                        paths: capture.paths,
                        native_formats,
                        source,
                        captured_at_unix_ms: capture.captured_at_unix_ms,
                    }) {
                        error!(%error, "failed to persist clipboard files capture");
                    }
                } else if let Some(image) = capture.image {
                    match self.store_captured_image(&image.staging_path) {
                        Ok(image_path) => {
                            if let Err(error) =
                                self.history.borrow_mut().capture_image(ImageCapture {
                                    image_path,
                                    width: image.width,
                                    height: image.height,
                                    native_formats,
                                    source,
                                    captured_at_unix_ms: capture.captured_at_unix_ms,
                                })
                            {
                                error!(%error, "failed to persist clipboard image capture");
                            }
                        }
                        Err(error) => error!(%error, "failed to ingest staged clipboard image"),
                    }
                } else if let Some(text) = capture.text
                    && !text.is_empty()
                {
                    if let Err(error) = self.history.borrow_mut().capture(TextCapture {
                        text: text.clone(),
                        native_formats,
                        source,
                        captured_at_unix_ms: capture.captured_at_unix_ms,
                    }) {
                        error!(%error, "failed to persist clipboard capture");
                    }
                    // 链接识别 runs on every text capture while enabled; a
                    // capture without recognized targets is a no-op.
                    self.handle_link_detector_capture(&text, cx);
                }
                if captured_at_unix_ms.saturating_sub(self.last_cleanup_at_unix_ms)
                    >= RETENTION_INTERVAL_MS
                {
                    self.apply_retention(captured_at_unix_ms);
                }
            }
            PlatformEvent::ClipboardPanelRequested(request) => {
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
                            popup.native_window_handle,
                            preview_window_handle,
                            pointer.root_window_handle,
                            pointer.process_id,
                            window,
                            cx,
                        );
                    });
                });
            }
            PlatformEvent::TrayActionRequested(_)
            | PlatformEvent::QuickLaunchPanelRequested(_)
            | PlatformEvent::QuickLaunchDirectoryContextResolved(_)
            | PlatformEvent::QuickLaunchAutoSwitchRequested(_)
            | PlatformEvent::QuickLaunchFileDialogSearchHostChanged(_)
            | PlatformEvent::TaskbarOverlayEditGestureFinished(_)
            | PlatformEvent::ElevatedDirectoryHelperRequested(_) => {}
        }
    }

    fn store_captured_image(&self, staging_path: &str) -> std::io::Result<String> {
        self.store_captured_asset(staging_path, "", true)
    }

    fn ingest_native_formats(
        &self,
        formats: Vec<ProtocolClipboardNativeFormat>,
    ) -> Vec<StoredClipboardNativeFormat> {
        formats
            .into_iter()
            .filter_map(|format| match self.store_native_format(&format.data_path) {
                Ok(data_path) => Some(StoredClipboardNativeFormat {
                    format_id: format.format_id,
                    format_name: format.format_name,
                    data_path,
                    size_bytes: format.size_bytes,
                    sort_order: format.sort_order,
                }),
                Err(error) => {
                    error!(
                        %error,
                        format_id = format.format_id,
                        size_bytes = format.size_bytes,
                        "failed to ingest staged native clipboard format"
                    );
                    let _ = std::fs::remove_file(&format.data_path);
                    None
                }
            })
            .collect()
    }

    fn store_native_format(&self, staging_path: &str) -> std::io::Result<String> {
        let staging = std::path::Path::new(staging_path);
        let fingerprint = content_fingerprint(std::fs::File::open(staging)?)?;
        let format_dir = self.clipboard_asset_dir.join("native-formats");
        std::fs::create_dir_all(&format_dir)?;
        let destination = format_dir.join(format!("{fingerprint}.bin"));
        if !destination.exists() {
            std::fs::copy(staging, &destination)?;
        }
        let _ = std::fs::remove_file(staging);
        Ok(destination.to_string_lossy().into_owned())
    }

    fn store_source_icon(&self, staging_path: &str) -> std::io::Result<String> {
        self.store_captured_asset(staging_path, "source-", false)
    }

    fn store_captured_asset(
        &self,
        staging_path: &str,
        file_name_prefix: &str,
        remove_source: bool,
    ) -> std::io::Result<String> {
        let staging = std::path::Path::new(staging_path);
        let fingerprint = content_fingerprint(std::fs::File::open(staging)?)?;
        let destination = self
            .clipboard_asset_dir
            .join(format!("{file_name_prefix}{fingerprint}.png"));
        if !destination.exists() {
            std::fs::copy(staging, &destination)?;
        }
        if remove_source {
            let _ = std::fs::remove_file(staging);
        }
        Ok(destination.to_string_lossy().into_owned())
    }

    fn show_panel(&mut self, request: ClipboardPanelRequest, cx: &mut Context<Self>) {
        if self.popup.lifecycle.replacement_pending() {
            info!(
                "ignored clipboard panel request while a replacement is awaiting topmost confirmation"
            );
            return;
        }

        let page = self
            .query_entries_page(
                "",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                0,
            )
            .unwrap_or_else(|error| {
                error!(%error, "failed to load clipboard popup history");
                ClipboardHistoryPage {
                    items: Vec::new(),
                    total: 0,
                    has_more: false,
                }
            });
        let categories = self.categories().unwrap_or_else(|error| {
            error!(%error, "failed to load clipboard popup categories");
            Vec::new()
        });
        let panel_size = self.config.clipboard_panel;
        let modifier_actions = self.config.clipboard.modifier_actions;
        let (bounds, display_id) = panel_bounds(&request, panel_size, cx);
        let application = cx.entity().downgrade();
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
                show: true,
                kind: WindowKind::PopUp,
                is_movable: true,
                is_resizable: true,
                is_minimizable: false,
                display_id,
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.Clipboard".into()),
                window_min_size: Some(size(px(280.0), px(280.0))),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let native_window = native_window_handle(window);
                *opened_native_window_slot.borrow_mut() = native_window;
                let panel = cx.new(|cx| {
                    ClipboardPanel::new(
                        page,
                        categories,
                        modifier_actions,
                        target,
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
                };
                let focus_window = popup.window;
                let topmost_result = if should_request_popup_topmost(true, native_window_handle) {
                    self.platform
                        .send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                            window_handle: native_window_handle,
                            enabled: true,
                        }))
                        .map_err(|error| error.to_string())
                } else {
                    Err("GPUI did not expose a native Windows handle".to_owned())
                };
                let Ok(topmost_command) = topmost_result else {
                    let reason =
                        topmost_result.expect_err("topmost result was checked as an error");
                    error!(%reason, "clipboard popup topmost request could not be created");
                    if let Some(previous) = self
                        .popup
                        .lifecycle
                        .replace_without_topmost_confirmation(popup)
                    {
                        let _ = previous
                            .window
                            .update(cx, |_, window, _| window.remove_window());
                    }
                    if let Some(current) = self.popup.lifecycle.current().cloned() {
                        let _ = current.window.update(cx, |_, window, app| {
                            let _ = current.panel.update(app, |panel, cx| {
                                panel.handle_topmost_failure(reason, window, cx);
                            });
                        });
                    }
                    return;
                };
                self.popup.lifecycle.stage(popup, topmost_command);
                info!(
                    target = %request.target.process_name,
                    x = request.cursor_x,
                    y = request.cursor_y,
                    width = panel_size.width,
                    height = panel_size.height,
                    "clipboard popup opened"
                );
                // The ctor focuses the search input while the native window is
                // still being shown; the OS can hand focus back to the caller
                // after the window appears, leaving typed text going nowhere.
                // Re-focus one frame later once the window is actually on
                // screen so typing lands in the search box by default.
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

    fn fixture_text() -> ClipboardItem {
        ClipboardItem {
            id: 1,
            kind: ClipboardKind::Text,
            text: "first\r\nsecond".into(),
            image_path: String::new(),
            image_width: 0,
            image_height: 0,
            file_paths: Vec::new(),
            source: CaptureSource {
                window_handle: 1,
                process_id: 2,
                process_name: "browser".into(),
                process_icon_path: String::new(),
                window_title: "source".into(),
            },
            captured_at_unix_ms: 10,
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
    fn image_original_uses_dib_bundle_and_shift_rejects_non_text() {
        let mut image = fixture_text();
        image.kind = ClipboardKind::Image;
        image.text.clear();
        image.image_path = "C:/images/capture.png".into();
        image.image_width = 640;
        image.image_height = 480;
        let dib = StoredClipboardNativeFormat {
            format_id: 8,
            format_name: String::new(),
            data_path: "C:/formats/image-dib.bin".into(),
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
        let formats = vec![
            StoredClipboardNativeFormat {
                format_id: 8,
                format_name: String::new(),
                data_path: "C:/formats/image-dib.bin".into(),
                size_bytes: 1024,
                sort_order: 0,
            },
            StoredClipboardNativeFormat {
                format_id: 15,
                format_name: String::new(),
                data_path: "C:/formats/image-drop.bin".into(),
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
}
