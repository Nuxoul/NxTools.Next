#[cfg(test)]
use std::cell::Cell;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant};

use chrono::{
    Datelike, Duration as ChronoDuration, Local, NaiveDate, NaiveTime, TimeZone, Timelike,
};
use gpui_kit::base::Root;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState, Textarea, TextareaState};
use gpui_kit::component::popover::Popover;
use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::component::select::{Select, SelectState};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Disableable, Icon, IndexPath, Sizable};
use gpui_kit::{
    Anchor, AnyElement, AnyWindowHandle, App, Context, Entity, EventEmitter, Focusable, FontWeight,
    IntoElement, MouseButton, Render, ScrollHandle, Subscription, Task, Window,
    WindowBackgroundAppearance, WindowBounds, WindowDecorations, WindowKind, WindowOptions, div,
    prelude::*, px, size,
};
use nxtools_core::DownloadRowDoubleClickActionSettings;
use nxtools_core::{
    AppConfig, DownloadCategorySettings, DownloadDuplicateStrategySettings, DownloadSettings,
    DownloadTaskbarOverlayAnchor, DownloadTaskbarProgressMode,
    DownloadTaskbarProgressOverlaySettings, NetworkProxyMode,
};
use nxtools_download::{
    BrowserBlob, BrowserDownloadDialogRequest, BrowserDownloadDialogSource, BrowserHandoffState,
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadBatchAction, DownloadCommand,
    DownloadCommandEnvelope, DownloadCommandOutcome, DownloadDailySchedule, DownloadEngine,
    DownloadEngineTask, DownloadEvent, DownloadEventEnvelope, DownloadProbe, DownloadProgress,
    DownloadQuery, DownloadQueue, DownloadQueueAction, DownloadQueuePlacement, DownloadRequest,
    DownloadSchedule, DownloadSegment, DownloadSegmentState, DownloadSnapshot, DownloadStatus,
    DownloadSubmitMode, DownloadTaskAction, DownloadTaskDetail, DownloadTaskId, DownloadWeekday,
    FFMPEG_BTBN_DOWNLOAD_URL, FFMPEG_GYAN_DOWNLOAD_URL, FfmpegInstallControl, FfmpegInstallFailure,
    FfmpegInstallPhase, FfmpegInstallProgress, FfmpegInstallation, RequestOrigin,
    resolve_ffmpeg_executable,
};
#[cfg(test)]
use nxtools_download::{DownloadCertificatePolicy, FfmpegInstallSource};
use nxtools_ipc::{PlatformBridge, PlatformBridgeError, PlatformNotification};
use nxtools_protocol::{
    ActivateWindowCommand, BeginFileDragCommand, EnsureBrowserNativeMessagingCommand,
    ExportBrowserExtensionCommand, ExtractShellIconCommand, MoveWindowCenteredOverKeyCommand,
    OpenShellTargetCommand, PasteTextCommand, PickBrowserExtensionExportPathCommand,
    PickDirectoryCommand, PickProgramCommand, PlaceTaskbarProgressOverlayCommand, PlatformCommand,
    PlatformCommandResult, PlatformEvent, SetTaskbarProgressCommand,
    SetWindowBoundsCenteredCommand, SetWindowTopmostCommand, SetWindowVisibilityCommand,
    ShellOpenAction, ShutdownSystemCommand, StashWindowBoundsCommand, SuspendSystemCommand,
    TaskbarOverlayAnchor, TaskbarProgressState,
};
use tracing::{error, info, warn};

use crate::completion_actions::{
    COMPLETION_FEEDBACK_DELAY, COMPLETION_PLATFORM_TIMEOUT, CompletionActionCoordinator,
    CompletionFeedbackBatch, CompletionTerminal, ShutdownCountdown, ShutdownTransition,
    completion_action_label,
};
use crate::download_taskbar_overlay::{
    DownloadTaskbarOverlayModel, DownloadTaskbarOverlaySample, DownloadTaskbarOverlaySegmentSample,
    DownloadTaskbarOverlayView, aggregate_taskbar_progress,
};
use crate::download_window_lifecycle::{
    DownloadWindowDirective, DownloadWindowLifecycle, DownloadWindowPhase,
};
use crate::icons::{MaterialIcon, shell_icon_path_is_renderable};
use crate::notifications::NotificationCenter;
use crate::theme::Theme;
use crate::ui::components::action::{
    SegmentedOption, action_button, danger_hover_button_variant, icon_action, icon_action_button,
    icon_action_tooltip_surface, segmented_control,
};
use crate::ui::components::dialog::{DialogShellIds, dialog_shell};
use crate::ui::components::field::{field_shell, inline_error, path_picker_shell};
use crate::ui::components::settings::{
    SettingsShellIds, dialog_footer, settings_shell, settings_toggle,
};
use crate::ui::components::titlebar::client_titlebar;
use crate::ui::metrics::{
    CONTROL_HEIGHT_ACTION, CONTROL_HEIGHT_INPUT, RADIUS_CONTROL, RADIUS_FLOATING,
    SETTINGS_ROW_MIN_HEIGHT, SPACING_4, SPACING_8, SPACING_12, SPACING_16,
};
use crate::ui::native_window_handle;
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, LINE_HEIGHT_TITLE, TEXT_BODY, TEXT_LABEL, TEXT_META,
    TEXT_STATUS_GLYPH, TEXT_TITLE, UI_MONO_FONT_FAMILY,
};

pub(crate) fn protocol_restart_summary(
    error: &nxtools_download::DownloadFault,
) -> Option<&'static str> {
    if error.code != nxtools_download::DownloadFaultCode::Protocol {
        return None;
    }
    let message = error.message.to_ascii_lowercase();
    if message.contains("resource changed")
        || message.contains("validator changed")
        || message.contains("etag changed")
    {
        Some("远程文件已变化，请重新下载")
    } else if message.contains("ignored the requested byte range")
        || message.contains("range") && message.contains("http 200")
    {
        Some("服务器不支持断点续传，请重新开始")
    } else {
        None
    }
}

mod artifact_actions;
mod batch;
#[cfg(test)]
mod batch_tests;
mod browser_handoff_impl;
mod clipboard_prefill;
mod completion_intention;
mod completion_shutdown_impl;
#[cfg(test)]
#[path = "downloads/controlled_handoff_tests.rs"]
mod controlled_handoff_tests;
use self::completion_intention::{CompletionIntentionTarget, completion_intention_menu};
#[cfg(debug_assertions)]
pub(crate) mod completion_transition_preview;
mod conflict;
#[cfg(test)]
mod conflict_tests;
mod delete_confirmation;
mod detail_request;
#[cfg(test)]
pub(crate) mod detail_request_tests;
mod directory_picker;
#[cfg(test)]
mod directory_picker_tests;
#[cfg(debug_assertions)]
pub(crate) mod fixed_transition_preview;
mod floating_assistant;
#[cfg(test)]
mod grouped_completion_tests;
mod new_download_view;
mod paused_url;
#[cfg(test)]
mod polish_tests;
mod progress_display;
mod range_coverage;
mod schedule_view;
#[cfg(test)]
mod server_information_tests;
mod settings_view;
mod shutdown_window;
mod speed_limit_edit;
mod task_window;
mod task_windows_impl;
#[cfg(debug_assertions)]
pub(crate) mod visual_preview;

use self::artifact_actions::PendingDownloadArtifactAction;
#[cfg(test)]
use self::artifact_actions::resolve_download_artifact_target;
use self::batch::{BatchConfirmation, PendingDownloadBatch};
use self::delete_confirmation::*;
pub(crate) use self::detail_request::DownloadDetailScope;
use self::directory_picker::*;
use self::new_download_view::*;
use self::schedule_view::*;
use self::settings_view::*;
use self::shutdown_window::*;
use self::task_window::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DownloadApplicationEvent {
    SelectBatchFailures {
        task_ids: Vec<DownloadTaskId>,
    },
    FfmpegExecutablePicked {
        path: PathBuf,
        picker_origin: Option<FfmpegPickerOrigin>,
    },
    FfmpegInstallChanged {
        state: FfmpegInstallViewState,
    },
    TaskbarOverlayEditChanged {
        settings: DownloadTaskbarProgressOverlaySettings,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FfmpegInstallViewState {
    Idle,
    Installing {
        phase: FfmpegInstallPhase,
        percent: u8,
        downloaded_bytes: u64,
        total_bytes: Option<u64>,
        cancelling: bool,
    },
    Completed,
    Cancelled,
    Failed(String),
}

impl FfmpegInstallViewState {
    fn is_active(&self) -> bool {
        matches!(self, Self::Installing { .. })
    }
}

#[derive(Debug, Clone)]
struct PendingFfmpegInstall {
    correlation_id: uuid::Uuid,
    started_at: Instant,
    network_proxy_mode: NetworkProxyMode,
    network_proxy_source: &'static str,
    control: PendingFfmpegInstallControl,
}

#[derive(Debug, Clone)]
enum PendingFfmpegInstallControl {
    Core(FfmpegInstallControl),
    #[cfg(test)]
    Test(Rc<Cell<bool>>),
}

impl PendingFfmpegInstallControl {
    fn cancel(&self) {
        match self {
            Self::Core(control) => control.cancel(),
            #[cfg(test)]
            Self::Test(cancelled) => cancelled.set(true),
        }
    }
}

#[cfg(test)]
struct FfmpegInstallOperationOverride {
    correlation_id: uuid::Uuid,
    progress: async_channel::Receiver<FfmpegInstallProgress>,
    completion: async_channel::Receiver<Result<FfmpegInstallation, FfmpegInstallFailure>>,
    cancelled: Rc<Cell<bool>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DownloadDirectoryTarget {
    NewTask,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownloadSettingsDraft {
    baseline: Option<DownloadSettings>,
    default_directory: PathBuf,
    chrome_web_store_extension_id: String,
    automatic_categories_enabled: bool,
    categories: Vec<DownloadCategorySettings>,
    duplicate_strategy: DownloadDuplicateStrategySettings,
    double_click_action: DownloadRowDoubleClickActionSettings,
    task_list_sort: Option<String>,
    task_list_sort_descending: Option<bool>,
    ffmpeg_executable_path: String,
    default_request_headers: String,
    default_cookie: String,
    accept_invalid_certificates: bool,
    default_connection_limit: u16,
    max_concurrent_tasks: u16,
    max_concurrent_tasks_baseline: Option<u16>,
    global_speed_limit_edited: bool,
    global_speed_limit_bytes_per_second: Option<u64>,
    background_speed_limit: nxtools_core::BackgroundDownloadLimitSettings,
    show_progress_window: bool,
    show_completion_window: bool,
    completion_sound_enabled: bool,
    open_directory_on_completion: bool,
    task_removal_deletes_file: bool,
    taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownloadSettingsViewInitial {
    draft: DownloadSettingsDraft,
    config_path: String,
    log_directory: String,
    database_path: String,
    ffmpeg_install_state: FfmpegInstallViewState,
    completion_queues: Vec<DownloadQueue>,
}

const FFMPEG_DOWNLOAD_PAGE_TIMEOUT: Duration = Duration::from_secs(15);
const DOWNLOAD_SETTINGS_PATH_ACTION_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadSettingsPathKind {
    Config,
    Logs,
    Database,
}

#[derive(Debug, Clone, Copy)]
struct DownloadSettingsPathPresentation {
    key: &'static str,
    display_name: &'static str,
    value_selector: &'static str,
    value_tooltip_selector: &'static str,
    copy_button_id: &'static str,
    open_button_id: &'static str,
    copy_tooltip: &'static str,
    open_tooltip: &'static str,
    copy_tooltip_selector: &'static str,
    open_tooltip_selector: &'static str,
    shell_action: ShellOpenAction,
}

impl DownloadSettingsPathKind {
    const fn presentation(self) -> DownloadSettingsPathPresentation {
        match self {
            Self::Config => DownloadSettingsPathPresentation {
                key: "config",
                display_name: "配置",
                value_selector: "download-settings-path-config-value",
                value_tooltip_selector: "download-settings-path-config-value-tooltip",
                copy_button_id: "download-settings-path-config-copy-button",
                open_button_id: "download-settings-path-config-open-button",
                copy_tooltip: "复制配置路径",
                open_tooltip: "在资源管理器中显示配置文件",
                copy_tooltip_selector: "download-settings-path-config-copy-tooltip",
                open_tooltip_selector: "download-settings-path-config-open-tooltip",
                shell_action: ShellOpenAction::SelectInExplorer,
            },
            Self::Logs => DownloadSettingsPathPresentation {
                key: "logs",
                display_name: "日志",
                value_selector: "download-settings-path-logs-value",
                value_tooltip_selector: "download-settings-path-logs-value-tooltip",
                copy_button_id: "download-settings-path-logs-copy-button",
                open_button_id: "download-settings-path-logs-open-button",
                copy_tooltip: "复制日志路径",
                open_tooltip: "打开日志目录",
                copy_tooltip_selector: "download-settings-path-logs-copy-tooltip",
                open_tooltip_selector: "download-settings-path-logs-open-tooltip",
                shell_action: ShellOpenAction::Open,
            },
            Self::Database => DownloadSettingsPathPresentation {
                key: "database",
                display_name: "数据库",
                value_selector: "download-settings-path-database-value",
                value_tooltip_selector: "download-settings-path-database-value-tooltip",
                copy_button_id: "download-settings-path-database-copy-button",
                open_button_id: "download-settings-path-database-open-button",
                copy_tooltip: "复制数据库路径",
                open_tooltip: "在资源管理器中显示数据库文件",
                copy_tooltip_selector: "download-settings-path-database-copy-tooltip",
                open_tooltip_selector: "download-settings-path-database-open-tooltip",
                shell_action: ShellOpenAction::SelectInExplorer,
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DownloadSettingsPathAction {
    Copy,
    Open,
}

impl DownloadSettingsPathAction {
    const fn name(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Open => "open",
        }
    }
}

#[derive(Clone)]
struct PendingExtensionExport {
    command_id: uuid::Uuid,
    operation_id: uuid::Uuid,
    format: &'static str,
    started_at: Instant,
    selection_origin: Option<(AnyWindowHandle, gpui_kit::WeakEntity<DownloadSettingsView>)>,
}

#[derive(Debug, Clone, Copy)]
struct PendingDownloadSettingsPathAction {
    correlation_id: uuid::Uuid,
    started_at: Instant,
    path_kind: DownloadSettingsPathKind,
    action: DownloadSettingsPathAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FfmpegDownloadSource {
    Gyan,
    Btbn,
}

impl FfmpegDownloadSource {
    const fn name(self) -> &'static str {
        match self {
            Self::Gyan => "gyan",
            Self::Btbn => "btbn",
        }
    }

    const fn url(self) -> &'static str {
        match self {
            Self::Gyan => FFMPEG_GYAN_DOWNLOAD_URL,
            Self::Btbn => FFMPEG_BTBN_DOWNLOAD_URL,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FfmpegPlatformOperationKind {
    Picker,
    DownloadPage(FfmpegDownloadSource),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FfmpegPickerOrigin {
    view_id: gpui_kit::EntityId,
    edit_generation: u64,
}

#[derive(Debug, Clone, Copy)]
struct PendingFfmpegPlatformOperation {
    correlation_id: uuid::Uuid,
    started_at: Instant,
    kind: FfmpegPlatformOperationKind,
    picker_origin: Option<FfmpegPickerOrigin>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FfmpegDiscoveryState {
    Checking,
    Found(nxtools_download::FfmpegResolutionSource),
    Missing,
}

pub(crate) struct DownloadRuntimeContext {
    pub fallback_default_directory: PathBuf,
    pub config: AppConfig,
    pub config_path: PathBuf,
    pub log_directory: PathBuf,
    pub database_path: PathBuf,
    pub ffmpeg_install_directory: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DownloadSubmitPresentationPlan {
    close_source_window: bool,
    deferred_task_window: Option<DownloadTaskId>,
}

fn submit_presentation_plan(
    mode: DownloadSubmitMode,
    task_id: DownloadTaskId,
    delayed: bool,
    show_progress_window: bool,
) -> DownloadSubmitPresentationPlan {
    DownloadSubmitPresentationPlan {
        close_source_window: true,
        deferred_task_window: (show_progress_window
            && mode == DownloadSubmitMode::StartNow
            && !delayed)
            .then_some(task_id),
    }
}

fn should_open_admitted_progress_window(
    known_to_desktop: bool,
    awaiting_delayed_progress: bool,
    origin: RequestOrigin,
    status: DownloadStatus,
) -> bool {
    (awaiting_delayed_progress || (!known_to_desktop && origin == RequestOrigin::Cli))
        && is_active_download_progress(status)
}

fn is_active_download_progress(status: DownloadStatus) -> bool {
    matches!(
        status,
        DownloadStatus::Probing
            | DownloadStatus::Waiting
            | DownloadStatus::Downloading
            | DownloadStatus::Finalizing
            | DownloadStatus::Verifying
    )
}

#[derive(Clone, Copy)]
enum ControlledProgressPlacement {
    Background,
    Centered,
    Capturing {
        command_id: uuid::Uuid,
        source: AnyWindowHandle,
        started_at: Instant,
    },
    Captured(uuid::Uuid),
}

struct PendingControlledProgress {
    correlation_id: uuid::Uuid,
    placement: ControlledProgressPlacement,
}

fn refresh_immediately_after_remove() -> bool {
    false
}

fn merge_confirmed_download_request(
    mut confirmed: DownloadRequest,
    current: DownloadRequest,
) -> Option<DownloadRequest> {
    if current.source_url != confirmed.source_url {
        return None;
    }
    confirmed.save_directory = current.save_directory;
    if current.file_name.is_some() {
        confirmed.file_name = current.file_name;
    }
    confirmed.connection_limit = current.connection_limit;
    confirmed.adaptive_segment_policy = current.adaptive_segment_policy;
    confirmed.scheduled_at_unix_ms = current.scheduled_at_unix_ms;
    confirmed.category_id = current.category_id;
    confirmed.category_name = current.category_name;
    confirmed.duplicate_strategy = current.duplicate_strategy;
    Some(confirmed)
}

pub(crate) struct DownloadTaskWindow {
    window: AnyWindowHandle,
    view: Entity<DownloadTaskWindowView>,
    native_window_handle: i64,
    generation: uuid::Uuid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TaskWindowIdentity {
    native_window_handle: i64,
    generation: uuid::Uuid,
}

#[derive(Clone, Copy)]
pub(crate) enum TaskWindowActivation {
    Fresh(DownloadWindowPhase),
    Reused(DownloadWindowPhase),
}

impl TaskWindowActivation {
    fn phase(self) -> DownloadWindowPhase {
        match self {
            Self::Fresh(phase) | Self::Reused(phase) => phase,
        }
    }

    fn replace_on_failure(self) -> bool {
        matches!(self, Self::Reused(_))
    }
}

pub(crate) struct PendingTaskWindowActivation {
    task_id: DownloadTaskId,
    window: TaskWindowIdentity,
    activation: TaskWindowActivation,
    started_at: Instant,
}

pub(crate) struct PendingCompletionFeedback {
    correlation_id: uuid::Uuid,
    started_at: Instant,
    completed_count: usize,
    failed_count: usize,
    directory_count: usize,
    suppressed_directory_count: usize,
    sound_enabled: bool,
    directory_enabled: bool,
    pending_commands: HashSet<uuid::Uuid>,
    failed_commands: usize,
    native_error_code: Option<i32>,
}

struct PendingShutdownCommand {
    command_id: uuid::Uuid,
    countdown: ShutdownCountdown,
}

struct PendingDownloadTaskIcon {
    resource_key: String,
    task_ids: HashSet<DownloadTaskId>,
    resource_scope: &'static str,
    started_at: Instant,
}

struct DownloadTaskIconFailure {
    attempts: u8,
    retry_token: Option<uuid::Uuid>,
}

const DOWNLOAD_TASK_ICON_SIZE: u32 = 48;
const DOWNLOAD_TASK_ICON_MAX_IN_FLIGHT: usize = 8;
const DOWNLOAD_TASK_ICON_RETRY_DELAY: Duration = Duration::from_secs(5);
const DOWNLOAD_TASK_ICON_MAX_ATTEMPTS: u8 = 3;
const DOWNLOAD_TASK_ICON_STA_RETRY_DELAY: Duration = Duration::from_secs(1);

fn download_task_icon_type_resource(target: &str) -> (String, String) {
    let extension = Path::new(target)
        .extension()
        .and_then(|extension| extension.to_str())
        .filter(|extension| {
            extension.len() <= 16
                && !extension.is_empty()
                && extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
        })
        .map(str::to_ascii_lowercase);
    let suffix =
        extension.map_or_else(|| "generic".to_owned(), |extension| format!(".{extension}"));
    (
        format!("{DOWNLOAD_TASK_ICON_SIZE}:type:{suffix}"),
        format!("nxtools-file-type:{suffix}"),
    )
}

fn download_task_icon_target(task: &DownloadEngineTask) -> Option<String> {
    if task.task.status() == DownloadStatus::Completed
        && let Some(path) = task.final_path.as_ref()
    {
        return Some(path.to_string_lossy().into_owned());
    }
    task.task
        .request()
        .file_name
        .clone()
        .or_else(|| task.probe.as_ref()?.file_name.clone())
        .or_else(|| {
            task.final_path
                .as_ref()?
                .file_name()?
                .to_str()
                .map(str::to_owned)
        })
}

fn download_task_icon_resource_key(task: &DownloadEngineTask) -> Option<String> {
    let target = download_task_icon_target(task)?;
    if task.task.status() == DownloadStatus::Completed && task.final_path.is_some() {
        return Some(format!(
            "{DOWNLOAD_TASK_ICON_SIZE}:target:{}",
            target.replace('/', "\\").to_lowercase()
        ));
    }
    Some(download_task_icon_type_resource(&target).0)
}

struct DownloadShutdownWindow {
    window: AnyWindowHandle,
    view: Entity<DownloadShutdownCountdownView>,
    correlation_id: uuid::Uuid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskbarOverlayNativeCloseIntent {
    FinishEdit {
        edit_correlation_id: Option<uuid::Uuid>,
    },
    ClosePassive,
}

pub(crate) struct DownloadApplication {
    engine: DownloadEngine,
    platform: Rc<PlatformBridge>,
    notifications: Entity<NotificationCenter>,
    default_directory: PathBuf,
    tasks: Rc<Vec<DownloadEngineTask>>,
    task_detail: Option<DownloadTaskDetail>,
    task_detail_error: Option<(DownloadTaskId, String)>,
    detail_requests: detail_request::DetailRequests,
    snapshot_error: Option<String>,
    queues: Vec<DownloadQueue>,
    latest_progress: HashMap<DownloadTaskId, DownloadProgress>,
    task_icons: HashMap<DownloadTaskId, PathBuf>,
    task_icon_resource_assignments: HashMap<DownloadTaskId, String>,
    task_icon_resources: HashMap<String, PathBuf>,
    pending_task_icons: HashMap<uuid::Uuid, PendingDownloadTaskIcon>,
    failed_task_icon_resources: HashMap<String, DownloadTaskIconFailure>,
    pending_task_icon_resources: HashMap<String, uuid::Uuid>,
    awaiting_delayed_progress: HashSet<DownloadTaskId>,
    pending_controlled_progress: HashMap<DownloadTaskId, PendingControlledProgress>,
    task_window_lifecycle: DownloadWindowLifecycle,
    task_windows: HashMap<DownloadTaskId, DownloadTaskWindow>,
    pending_task_window_activations: HashMap<uuid::Uuid, PendingTaskWindowActivation>,
    pending_task_window_replacements: HashMap<DownloadTaskId, TaskWindowIdentity>,
    #[cfg(test)]
    task_window_native_handle_override: Option<i64>,
    #[cfg(test)]
    task_window_activation_commands: Rc<RefCell<Vec<PlatformCommand>>>,
    #[allow(
        dead_code,
        reason = "retained for the Phase 3 task context-menu delete action"
    )]
    delete_confirmation_windows: HashMap<DownloadTaskId, AnyWindowHandle>,
    pending_batch: Option<PendingDownloadBatch>,
    pending_batch_confirmation: Option<DownloadTaskId>,
    destination_conflicts: HashSet<DownloadTaskId>,
    new_download_window: Option<AnyWindowHandle>,
    #[cfg(test)]
    new_download_view: Option<Entity<NewDownloadView>>,
    pending_new_download_handoff: Option<browser_handoff_impl::PendingNewDownloadHandoff>,
    pending_new_download_clipboard: Option<clipboard_prefill::PendingDownloadClipboard>,
    _new_download_close_subscription: Subscription,
    #[cfg(test)]
    new_download_native_handles_override: Option<(i64, i64)>,
    #[cfg(test)]
    new_download_handoff_commands: Vec<PlatformCommand>,
    schedule_window: Option<AnyWindowHandle>,
    settings_window: Option<AnyWindowHandle>,
    #[cfg(test)]
    settings_view: Option<Entity<DownloadSettingsView>>,
    taskbar_overlay_window: Option<AnyWindowHandle>,
    taskbar_overlay_native_window_handle: i64,
    taskbar_overlay_correlation_id: uuid::Uuid,
    taskbar_overlay_edit_draft: Option<DownloadTaskbarProgressOverlaySettings>,
    taskbar_overlay_edit_correlation_id: Option<uuid::Uuid>,
    taskbar_overlay_edit_gesture_active: bool,
    pending_taskbar_overlay_gestures: HashSet<uuid::Uuid>,
    config: AppConfig,
    foreground_process_id: Option<u32>,
    pending_artifact_actions: HashMap<uuid::Uuid, PendingDownloadArtifactAction>,
    config_path: PathBuf,
    log_directory: PathBuf,
    database_path: PathBuf,
    ffmpeg_install_directory: PathBuf,
    show_progress_window: bool,
    show_completion_window: bool,
    pending_directory_picker: Option<PendingDownloadDirectoryPicker>,
    pending_settings_path_actions: HashMap<uuid::Uuid, PendingDownloadSettingsPathAction>,
    pending_extension_export: Option<PendingExtensionExport>,
    pending_browser_registration: Option<(uuid::Uuid, Instant)>,
    pending_browser_store_page: Option<(uuid::Uuid, Instant)>,
    pending_ffmpeg_operations: HashMap<uuid::Uuid, PendingFfmpegPlatformOperation>,
    ffmpeg_install_state: FfmpegInstallViewState,
    pending_ffmpeg_install: Option<PendingFfmpegInstall>,
    #[cfg(test)]
    ffmpeg_install_operation_override: Option<FfmpegInstallOperationOverride>,
    #[cfg(test)]
    ffmpeg_platform_commands: Rc<RefCell<Vec<(uuid::Uuid, PlatformCommand)>>>,
    notified_terminal_states: HashSet<(DownloadTaskId, DownloadStatus)>,
    completion_actions: CompletionActionCoordinator,
    completion_feedback_scheduled: bool,
    pending_completion_feedback: HashMap<uuid::Uuid, PendingCompletionFeedback>,
    completion_feedback_commands: HashMap<uuid::Uuid, uuid::Uuid>,
    shutdown_window: Option<DownloadShutdownWindow>,
    pending_shutdown_command: Option<PendingShutdownCommand>,
    event_generation: u64,
    last_event_sequence: u64,
    _event_task: Task<()>,
    _browser_dialog_task: Task<()>,
}

impl EventEmitter<DownloadApplicationEvent> for DownloadApplication {}

impl DownloadApplication {
    pub(crate) fn new(
        engine: DownloadEngine,
        platform: Rc<PlatformBridge>,
        notifications: Entity<NotificationCenter>,
        runtime: DownloadRuntimeContext,
        cx: &mut Context<Self>,
    ) -> Self {
        let DownloadRuntimeContext {
            fallback_default_directory,
            mut config,
            config_path,
            log_directory,
            database_path,
            ffmpeg_install_directory,
        } = runtime;
        config.download.normalize();
        if let Err(error) = engine.update_download_http_policy(&config.download) {
            warn!(
                error_kind = "download_http_policy",
                %error,
                "could not apply configured download HTTP policy"
            );
        }
        if let Err(error) = engine.update_download_organization_policy(&config.download) {
            warn!(
                error_kind = "download_organization_policy",
                %error,
                "could not apply configured download organization policy"
            );
        }
        let configured_ffmpeg = (!config.download.ffmpeg_executable_path.is_empty())
            .then(|| PathBuf::from(&config.download.ffmpeg_executable_path));
        if let Err(error) = engine.set_ffmpeg_executable_path(configured_ffmpeg) {
            warn!(%error, "could not apply configured FFmpeg executable to download engine");
        }
        let default_directory = if config.download.default_directory.is_empty() {
            fallback_default_directory
        } else {
            PathBuf::from(&config.download.default_directory)
        };
        let show_progress_window = config.download.show_progress_window;
        let show_completion_window = config.download.show_completion_window;
        let (tasks, queues, event_generation, event_sequence) =
            match load_download_snapshot(&engine) {
                Ok(snapshot) => match download_snapshot_tasks(&snapshot) {
                    Ok(tasks) => (
                        tasks,
                        snapshot.queues,
                        snapshot.generation,
                        snapshot.sequence,
                    ),
                    Err(error) => {
                        warn!(%error, "could not project the compact download snapshot");
                        (
                            Vec::new(),
                            snapshot.queues,
                            snapshot.generation,
                            snapshot.sequence,
                        )
                    }
                },
                Err(error) => {
                    warn!(%error, "could not load the download snapshot for the desktop");
                    (Vec::new(), Vec::new(), 0, 0)
                }
            };
        let mut task_window_lifecycle = DownloadWindowLifecycle::default();
        let awaiting_delayed_progress = tasks
            .iter()
            .filter(|task| task.task.origin() == RequestOrigin::Ui && task.scheduled)
            .map(|task| task.task.id())
            .collect();
        let mut notified_terminal_states = HashSet::new();
        for task in &tasks {
            let task_id = task.task.id();
            let status = task.task.status();
            task_window_lifecycle.restore_existing(task_id, status);
            if matches!(status, DownloadStatus::Completed | DownloadStatus::Failed) {
                notified_terminal_states.insert((task_id, status));
            }
        }
        let _ = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Ui,
            command: DownloadCommand::Subscribe {
                after_sequence: Some(event_sequence),
                generation: (event_generation != 0).then_some(event_generation),
            },
        });
        let receiver = engine.take_event_receiver().unwrap_or_else(|error| {
            warn!(%error, "could not attach the desktop download event subscriber");
            async_channel::bounded(1).1
        });
        let event_task = cx.spawn(async move |this, cx| {
            while let Ok(event) = receiver.recv().await {
                let Some(this) = this.upgrade() else {
                    return;
                };
                this.update(cx, |application, cx| {
                    application.handle_engine_event(event, cx);
                });
            }
        });
        let browser_dialog_receiver =
            engine
                .take_browser_dialog_receiver()
                .unwrap_or_else(|error| {
                    warn!(%error, "could not attach the desktop browser-dialog subscriber");
                    async_channel::bounded(1).1
                });
        let browser_dialog_task = cx.spawn(async move |this, cx| {
            while let Ok(request) = browser_dialog_receiver.recv().await {
                let Some(this) = this.upgrade() else {
                    return;
                };
                this.update(cx, |application, cx| {
                    application.open_browser_download_dialog(request, cx);
                });
            }
        });
        let completion_actions = engine
            .completion_snapshot()
            .map(|snapshot| CompletionActionCoordinator::from_snapshot(&snapshot))
            .unwrap_or_default();
        Self {
            engine,
            platform,
            notifications,
            default_directory,
            tasks: Rc::new(tasks),
            task_detail: None,
            task_detail_error: None,
            detail_requests: detail_request::DetailRequests::default(),
            snapshot_error: None,
            queues,
            latest_progress: HashMap::new(),
            task_icons: HashMap::new(),
            task_icon_resource_assignments: HashMap::new(),
            task_icon_resources: HashMap::new(),
            pending_task_icons: HashMap::new(),
            pending_task_icon_resources: HashMap::new(),
            failed_task_icon_resources: HashMap::new(),
            awaiting_delayed_progress,
            pending_controlled_progress: HashMap::new(),
            task_window_lifecycle,
            task_windows: HashMap::new(),
            pending_task_window_activations: HashMap::new(),
            pending_task_window_replacements: HashMap::new(),
            #[cfg(test)]
            task_window_native_handle_override: None,
            #[cfg(test)]
            task_window_activation_commands: Rc::new(RefCell::new(Vec::new())),
            delete_confirmation_windows: HashMap::new(),
            pending_batch: None,
            pending_batch_confirmation: None,
            destination_conflicts: HashSet::new(),
            new_download_window: None,
            #[cfg(test)]
            new_download_view: None,
            pending_new_download_handoff: None,
            pending_new_download_clipboard: None,
            _new_download_close_subscription: {
                let application = cx.entity().downgrade();
                cx.on_window_closed(move |cx, _| {
                    let application = application.clone();
                    cx.defer(move |cx| {
                        let _ = application.update(cx, |application, cx| {
                            application.cleanup_new_download_handoff(cx);
                            application.cleanup_directory_picker(cx);
                        });
                    });
                })
            },
            #[cfg(test)]
            new_download_native_handles_override: None,
            #[cfg(test)]
            new_download_handoff_commands: Vec::new(),
            schedule_window: None,
            settings_window: None,
            #[cfg(test)]
            settings_view: None,
            taskbar_overlay_window: None,
            taskbar_overlay_native_window_handle: 0,
            taskbar_overlay_correlation_id: uuid::Uuid::new_v4(),
            taskbar_overlay_edit_draft: None,
            taskbar_overlay_edit_correlation_id: None,
            taskbar_overlay_edit_gesture_active: false,
            pending_taskbar_overlay_gestures: HashSet::new(),
            config,
            foreground_process_id: None,
            pending_artifact_actions: HashMap::new(),
            config_path,
            log_directory,
            database_path,
            ffmpeg_install_directory,
            show_progress_window,
            show_completion_window,
            pending_directory_picker: None,
            pending_settings_path_actions: HashMap::new(),
            pending_extension_export: None,
            pending_browser_registration: None,
            pending_browser_store_page: None,
            pending_ffmpeg_operations: HashMap::new(),
            ffmpeg_install_state: FfmpegInstallViewState::Idle,
            pending_ffmpeg_install: None,
            #[cfg(test)]
            ffmpeg_install_operation_override: None,
            #[cfg(test)]
            ffmpeg_platform_commands: Rc::new(RefCell::new(Vec::new())),
            notified_terminal_states,
            completion_actions,
            completion_feedback_scheduled: false,
            pending_completion_feedback: HashMap::new(),
            completion_feedback_commands: HashMap::new(),
            shutdown_window: None,
            pending_shutdown_command: None,
            event_generation,
            last_event_sequence: event_sequence,
            _event_task: event_task,
            _browser_dialog_task: browser_dialog_task,
        }
    }

    #[cfg(test)]
    pub(crate) fn configure_task_window_activation_test(
        &mut self,
        native_window_handle: i64,
    ) -> Rc<RefCell<Vec<PlatformCommand>>> {
        self.task_window_native_handle_override = Some(native_window_handle);
        self.task_window_activation_commands.borrow_mut().clear();
        Rc::clone(&self.task_window_activation_commands)
    }

    #[cfg(test)]
    pub(crate) fn replace_tasks_for_test(&mut self, tasks: Vec<DownloadEngineTask>) {
        self.tasks = Rc::new(tasks);
    }

    #[cfg(test)]
    pub(crate) fn replace_progress_for_test(&mut self, progress: Vec<DownloadProgress>) {
        self.latest_progress = progress
            .into_iter()
            .map(|progress| (progress.task_id, progress))
            .collect();
    }

    #[cfg(test)]
    pub(crate) fn clear_task_detail_for_test(&mut self, cx: &mut Context<Self>) {
        self.task_detail = None;
        self.task_detail_error = None;
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn replace_task_icon_for_test(
        &mut self,
        task_id: DownloadTaskId,
        path: PathBuf,
        cx: &mut Context<Self>,
    ) {
        self.task_icons.insert(task_id, path);
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn replace_task_detail_error_for_test(
        &mut self,
        task_id: DownloadTaskId,
        error: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.task_detail = None;
        self.task_detail_error = Some((task_id, error.into()));
        cx.notify();
    }

    pub(crate) fn tasks(&self) -> &[DownloadEngineTask] {
        &self.tasks
    }

    /// Shares the authoritative immutable snapshot with deferred row composition.
    pub(crate) fn task_snapshot(&self) -> Rc<Vec<DownloadEngineTask>> {
        Rc::clone(&self.tasks)
    }

    pub(crate) fn task_removal_deletes_file(&self) -> bool {
        self.config.download.task_removal_deletes_file
    }

    pub(crate) fn task_icon_path(&self, task_id: DownloadTaskId) -> Option<&Path> {
        #[cfg(test)]
        crate::main_window::projection_tests::observe_icon_read();
        self.task_icons.get(&task_id).map(PathBuf::as_path)
    }

    pub(crate) fn taskbar_overlay_model(&self) -> DownloadTaskbarOverlayModel {
        let editing = self.taskbar_overlay_edit_draft.is_some();
        let settings = self
            .taskbar_overlay_edit_draft
            .as_ref()
            .unwrap_or(&self.config.download.taskbar_progress_overlay);
        if !settings.enabled && !editing {
            return DownloadTaskbarOverlayModel::Hidden;
        }
        aggregate_taskbar_progress(
            settings,
            editing,
            self.tasks.iter().map(|task| {
                let progress = self.latest_progress.get(&task.task.id());
                DownloadTaskbarOverlaySample {
                    status: task.task.status(),
                    downloaded_bytes: progress
                        .map_or(task.downloaded_bytes, |progress| progress.downloaded_bytes),
                    total_bytes: progress
                        .and_then(|progress| progress.total_bytes)
                        .or(task.total_bytes),
                    bytes_per_second: progress.map_or(0, |progress| progress.bytes_per_second),
                    active_connections: progress.map_or(0, |progress| progress.active_connections),
                    segments: progress
                        .map(|progress| {
                            progress
                                .segments
                                .iter()
                                .map(|segment| DownloadTaskbarOverlaySegmentSample {
                                    downloaded_bytes: segment.downloaded_bytes,
                                    total_bytes: segment.total_bytes,
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                }
            }),
        )
    }

    pub(crate) fn taskbar_overlay_diagnostic_correlation_id(&self) -> uuid::Uuid {
        self.taskbar_overlay_edit_correlation_id
            .unwrap_or(self.taskbar_overlay_correlation_id)
    }

    pub(crate) fn set_taskbar_overlay_edit_gesture_active(&mut self, active: bool) {
        self.taskbar_overlay_edit_gesture_active = active;
    }

    pub(crate) fn taskbar_overlay_edit_gesture_active(&self) -> bool {
        self.taskbar_overlay_edit_gesture_active
    }

    pub(crate) fn register_taskbar_overlay_edit_gesture(&mut self, command_id: uuid::Uuid) {
        self.pending_taskbar_overlay_gestures.insert(command_id);
    }

    pub(crate) fn default_queue_order(&self) -> Vec<DownloadTaskId> {
        self.engine.queue_order("default").unwrap_or_else(|error| {
            warn!(%error, "could not load the default download queue order");
            Vec::new()
        })
    }

    pub(crate) fn task_detail(&self, task_id: DownloadTaskId) -> Option<&DownloadTaskDetail> {
        self.task_detail
            .as_ref()
            .filter(|detail| detail.task.task.id() == task_id)
    }

    pub(crate) fn task_detail_error(&self, task_id: DownloadTaskId) -> Option<&str> {
        self.task_detail_error
            .as_ref()
            .filter(|(failed_task_id, _)| *failed_task_id == task_id)
            .map(|(_, error)| error.as_str())
    }

    pub(crate) fn snapshot_error(&self) -> Option<&str> {
        self.snapshot_error.as_deref()
    }

    fn default_queue(&self) -> Option<&DownloadQueue> {
        self.queues.iter().find(|queue| queue.id == "default")
    }

    fn settings_draft(&self) -> DownloadSettingsDraft {
        DownloadSettingsDraft {
            baseline: Some(DownloadSettings {
                default_directory: self.default_directory.to_string_lossy().into_owned(),
                ..self.config.download.clone()
            }),
            default_directory: self.default_directory.clone(),
            chrome_web_store_extension_id: self
                .config
                .download
                .chrome_web_store_extension_id
                .clone(),
            automatic_categories_enabled: self.config.download.automatic_categories_enabled,
            categories: self.config.download.categories.clone(),
            duplicate_strategy: self.config.download.duplicate_strategy,
            double_click_action: self.config.download.double_click_action,
            task_list_sort: self.config.download.task_list_sort.clone(),
            task_list_sort_descending: self.config.download.task_list_sort_descending,
            ffmpeg_executable_path: self.config.download.ffmpeg_executable_path.clone(),
            default_request_headers: self.config.download.default_request_headers.clone(),
            default_cookie: self.config.download.default_cookie.clone(),
            accept_invalid_certificates: self.config.download.accept_invalid_certificates,
            default_connection_limit: self.config.download.default_connection_limit,
            max_concurrent_tasks: self
                .default_queue()
                .and_then(|queue| queue.max_concurrent_tasks)
                .unwrap_or(3),
            max_concurrent_tasks_baseline: Some(
                self.default_queue()
                    .and_then(|queue| queue.max_concurrent_tasks)
                    .unwrap_or(3),
            ),
            global_speed_limit_edited: true,
            global_speed_limit_bytes_per_second: self.engine.global_speed_limit(),
            background_speed_limit: self.config.download.background_speed_limit.clone(),
            show_progress_window: self.show_progress_window,
            show_completion_window: self.show_completion_window,
            completion_sound_enabled: self.config.download.completion_sound_enabled,
            open_directory_on_completion: self.config.download.open_directory_on_completion,
            task_removal_deletes_file: self.config.download.task_removal_deletes_file,
            taskbar_progress_overlay: self.config.download.taskbar_progress_overlay.clone(),
        }
    }

    fn settings_view_initial(&self) -> DownloadSettingsViewInitial {
        DownloadSettingsViewInitial {
            draft: self.settings_draft(),
            config_path: self.config_path.display().to_string(),
            log_directory: self.log_directory.display().to_string(),
            database_path: self.database_path.display().to_string(),
            ffmpeg_install_state: self.ffmpeg_install_state.clone(),
            completion_queues: self
                .engine
                .completion_snapshot()
                .map(|snapshot| snapshot.queues)
                .unwrap_or_default(),
        }
    }

    fn default_connection_limit(&self) -> u16 {
        self.config.download.default_connection_limit
    }

    fn default_adaptive_segment_policy(&self) -> nxtools_download::DownloadAdaptiveSegmentPolicy {
        nxtools_download::DownloadAdaptiveSegmentPolicy {
            enabled: self.config.download.adaptive_segment_scaling_enabled,
            max_segments: self.config.download.adaptive_segment_max_segments,
            min_segment_mib: self.config.download.adaptive_segment_min_mib,
        }
    }

    fn task(&self, task_id: DownloadTaskId) -> Option<&DownloadEngineTask> {
        self.tasks
            .iter()
            .find(|candidate| candidate.task.id() == task_id)
    }

    pub(crate) fn progress(&self, task_id: DownloadTaskId) -> Option<&DownloadProgress> {
        #[cfg(test)]
        crate::main_window::projection_tests::observe_progress_read();
        self.latest_progress.get(&task_id)
    }

    pub(crate) fn submit_installer_download(
        &mut self,
        source_url: String,
        file_name: String,
        expected_total_bytes: Option<u64>,
        idempotency_key: uuid::Uuid,
        cx: &mut Context<Self>,
    ) -> Result<DownloadTaskId, String> {
        if let Some(existing) = self.tasks.iter().find(|entry| {
            let request = entry.task.request();
            request.source_url == source_url
                && request.file_name.as_deref() == Some(file_name.as_str())
        }) {
            return Ok(existing.task.id());
        }
        let request = DownloadRequest {
            source_url,
            save_directory: self.default_directory.clone(),
            save_directory_explicit: false,
            file_name: Some(file_name),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            source_page: None,
            certificate_policy: Default::default(),
            connection_limit: Some(self.default_connection_limit()),
            speed_limit_bytes_per_second: None,
            expected_total_bytes,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: Some(self.default_adaptive_segment_policy()),
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        self.submit_with_idempotency(
            request,
            DownloadSubmitMode::StartNow,
            Some(idempotency_key),
            cx,
        )
    }

    pub(crate) fn submit_with_idempotency(
        &mut self,
        request: DownloadRequest,
        mode: DownloadSubmitMode,
        idempotency_key: Option<uuid::Uuid>,
        cx: &mut Context<Self>,
    ) -> Result<DownloadTaskId, String> {
        self.submit_with_resolution_options(request, mode, idempotency_key, None, cx)
    }
    pub(crate) fn submit_with_resolution_options(
        &mut self,
        request: DownloadRequest,
        mode: DownloadSubmitMode,
        idempotency_key: Option<uuid::Uuid>,
        resolution: Option<nxtools_download::DownloadResolutionOptions>,
        cx: &mut Context<Self>,
    ) -> Result<DownloadTaskId, String> {
        let delayed_progress =
            mode == DownloadSubmitMode::StartNow && request.scheduled_at_unix_ms.is_some();
        let command = if let Some(resolution) = resolution {
            DownloadCommand::SubmitResolved {
                request,
                mode,
                idempotency_key,
                resolution,
            }
        } else {
            DownloadCommand::Submit {
                request,
                mode,
                idempotency_key,
            }
        };
        let outcome = self.execute(command);
        let task_id = match outcome {
            DownloadCommandOutcome::Accepted {
                task_id: Some(task_id),
            }
            | DownloadCommandOutcome::IdempotentAccepted { task_id, .. } => task_id,
            DownloadCommandOutcome::Rejected { fault } => return Err(fault.message),
            _ => return Err("下载引擎返回了无效的提交结果".into()),
        };
        if delayed_progress {
            self.awaiting_delayed_progress.insert(task_id);
        }
        self.refresh(cx);
        info!(
            task_id = %task_id.as_uuid(),
            ?mode,
            idempotency_key = ?idempotency_key,
            "download task submission accepted"
        );
        Ok(task_id)
    }

    pub(crate) fn pause(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if let Err(error) = self.execute_task_action(task_id, DownloadTaskAction::Pause) {
            return self.action_error("无法暂停下载", error, cx);
        }
        Ok(())
    }

    pub(crate) fn resume(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if let Err(error) = self.execute_task_action(task_id, DownloadTaskAction::Resume) {
            return self.action_error("无法继续下载", error, cx);
        }
        self.refresh(cx);
        self.present_explicit_task_start(task_id, false, cx);
        Ok(())
    }
    pub(crate) fn retry(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if let Err(error) = self.execute_task_action(task_id, DownloadTaskAction::Retry) {
            return self.action_error("无法重试下载", error.to_string(), cx);
        }
        self.refresh(cx);
        self.present_explicit_task_start(task_id, false, cx);
        Ok(())
    }

    fn present_explicit_task_start(
        &mut self,
        task_id: DownloadTaskId,
        background: bool,
        cx: &mut Context<Self>,
    ) {
        self.awaiting_delayed_progress.remove(&task_id);
        if !self.show_progress_window
            || self.pending_controlled_progress.contains_key(&task_id)
            || self
                .task(task_id)
                .is_none_or(|task| task.task.status() == DownloadStatus::Cancelled)
        {
            return;
        }
        if background {
            if self
                .task(task_id)
                .is_some_and(|task| is_active_download_progress(task.task.status()))
            {
                let directive = self
                    .task_window_lifecycle
                    .request_background_progress(task_id);
                self.apply_task_window_directive(task_id, directive, cx);
            }
        } else {
            self.request_task_window(task_id, cx);
        }
    }

    pub(crate) fn redownload(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) -> Result<DownloadTaskId, String> {
        let replacement = match self.execute_task_action(task_id, DownloadTaskAction::Redownload) {
            Ok(replacement) => replacement,
            Err(error) => {
                let _ = self.action_error("无法重新下载", error.clone(), cx);
                return Err(error);
            }
        };
        self.refresh(cx);
        Ok(replacement)
    }

    pub(crate) fn cancel(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if let Err(error) = self.execute_task_action(task_id, DownloadTaskAction::Cancel) {
            return self.action_error("无法取消下载", error, cx);
        }
        Ok(())
    }

    #[allow(
        dead_code,
        reason = "retained for the Phase 3 task context-menu remove action"
    )]
    pub(crate) fn remove(
        &mut self,
        task_id: DownloadTaskId,
        delete_artifact: bool,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if let Err(error) =
            self.execute_task_action(task_id, DownloadTaskAction::Remove { delete_artifact })
        {
            return self.action_error("无法移除下载任务", error, cx);
        }
        self.notified_terminal_states
            .retain(|(candidate, _)| *candidate != task_id);
        if refresh_immediately_after_remove() {
            self.refresh(cx);
        }
        Ok(())
    }

    pub(crate) fn move_queue_tasks(
        &mut self,
        task_ids: Vec<DownloadTaskId>,
        placement: DownloadQueuePlacement,
        cx: &mut Context<Self>,
    ) {
        match self.execute(DownloadCommand::Queue {
            action: DownloadQueueAction::Move {
                queue_id: "default".into(),
                task_ids,
                placement,
            },
        }) {
            DownloadCommandOutcome::QueueChanged { .. } => self.refresh(cx),
            DownloadCommandOutcome::Rejected { fault } => {
                let _ = self.action_error("无法调整任务顺序", fault.message, cx);
            }
            _ => {
                let _ = self.action_error(
                    "无法调整任务顺序",
                    "下载引擎返回了无效的队列操作结果".into(),
                    cx,
                );
            }
        }
    }

    fn set_default_queue_schedule(
        &mut self,
        schedule: Option<DownloadSchedule>,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        match self.execute(DownloadCommand::Queue {
            action: DownloadQueueAction::SetSchedule {
                queue_id: "default".into(),
                schedule,
            },
        }) {
            DownloadCommandOutcome::QueueChanged { .. } => {
                self.refresh(cx);
                Ok(())
            }
            DownloadCommandOutcome::Rejected { fault } => Err(fault.message),
            _ => Err("下载引擎返回了无效的队列操作结果".into()),
        }
    }

    fn action_error(
        &mut self,
        title: &'static str,
        message: String,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_error(
                format!("download-task-action-{}", uuid::Uuid::new_v4()),
                title,
                message.clone(),
                cx,
            );
        });
        Err(message)
    }

    fn execute(&self, command: DownloadCommand) -> DownloadCommandOutcome {
        self.execute_correlated(command, uuid::Uuid::new_v4())
    }

    fn execute_correlated(
        &self,
        command: DownloadCommand,
        correlation_id: uuid::Uuid,
    ) -> DownloadCommandOutcome {
        self.engine
            .handle_command(DownloadCommandEnvelope {
                protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                request_id: correlation_id,
                origin: RequestOrigin::Ui,
                command,
            })
            .outcome
    }

    fn execute_task_action(
        &self,
        task_id: DownloadTaskId,
        action: DownloadTaskAction,
    ) -> Result<DownloadTaskId, String> {
        match self.execute(DownloadCommand::Task { task_id, action }) {
            DownloadCommandOutcome::Accepted {
                task_id: Some(result_task_id),
            } => Ok(result_task_id),
            DownloadCommandOutcome::Accepted { task_id: None } => {
                Err("下载引擎没有返回任务 ID".into())
            }
            DownloadCommandOutcome::Rejected { fault } => Err(fault.message),
            _ => Err("下载引擎返回了无效的任务操作结果".into()),
        }
    }

    /// Copies the source URLs of the given tasks to the clipboard (one per
    /// line) through the platform paste channel with paste disabled.
    pub(crate) fn copy_task_urls(&mut self, task_ids: &[DownloadTaskId], cx: &mut Context<Self>) {
        let mut urls = Vec::new();
        for task_id in task_ids {
            if let Some(task) = self.task(*task_id) {
                urls.push(task.task.request().source_url.clone());
            }
        }
        if urls.is_empty() {
            return;
        }
        let payload = urls.join(
            "
",
        );
        match self
            .platform
            .send_command(PlatformCommand::PasteText(PasteTextCommand {
                text: payload,
                paste_after_write: false,
                target_window_handle: 0,
                target_focus_window_handle: 0,
            })) {
            Ok(_) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_success(
                        format!("download-urls-copied-{}", uuid::Uuid::new_v4()),
                        "链接已复制",
                        format!("已复制 {} 条下载链接", urls.len()),
                        cx,
                    );
                });
            }
            Err(error) => {
                warn!(event = "download.urls.copy_failed", %error, "could not copy download urls");
            }
        }
    }

    pub(crate) fn row_double_click_action(&self) -> DownloadRowDoubleClickActionSettings {
        self.config.download.double_click_action
    }

    pub(crate) fn platform_bridge(&self) -> Rc<PlatformBridge> {
        Rc::clone(&self.platform)
    }

    pub(crate) fn task_list_sort(&self) -> (Option<String>, Option<bool>) {
        (
            self.config.download.task_list_sort.clone(),
            self.config.download.task_list_sort_descending,
        )
    }

    /// Persists the task-list sort preference. Sort changes are frequent, so
    /// this only rewrites the two option fields through the standard config
    /// update helper and keeps the in-memory copy in sync.
    pub(crate) fn set_task_list_sort(
        &mut self,
        key: Option<String>,
        descending: Option<bool>,
        cx: &mut Context<Self>,
    ) {
        if self.config.download.task_list_sort == key
            && self.config.download.task_list_sort_descending == descending
        {
            return;
        }
        self.config.download.task_list_sort = key.clone();
        self.config.download.task_list_sort_descending = descending;
        let mut next = self.config.download.clone();
        next.task_list_sort = key;
        next.task_list_sort_descending = descending;
        match AppConfig::update_download_settings(&self.config_path, next) {
            Ok(_) => info!(event = "download.sort_preference_saved"),
            Err(error) => warn!(
                event = "download.sort_preference_save_failed",
                %error,
                "could not persist the task-list sort preference"
            ),
        }
        cx.notify();
    }

    fn request_ffmpeg_executable(
        &mut self,
        current_path: String,
        source_window_handle: i64,
        picker_origin: Option<FfmpegPickerOrigin>,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_ffmpeg_operations
            .values()
            .any(|pending| pending.kind == FfmpegPlatformOperationKind::Picker)
        {
            return;
        }
        self.begin_ffmpeg_platform_operation(
            PlatformCommand::PickProgram(PickProgramCommand {
                current_path,
                source_window_handle,
                source_process_id: std::process::id(),
            }),
            FfmpegPlatformOperationKind::Picker,
            None,
            picker_origin,
            cx,
        );
    }

    fn start_ffmpeg_install(&mut self, cx: &mut Context<Self>) {
        if self.pending_ffmpeg_install.is_some() {
            return;
        }
        let preserve_executable = (!self.config.download.ffmpeg_executable_path.is_empty())
            .then(|| PathBuf::from(&self.config.download.ffmpeg_executable_path));
        #[cfg(test)]
        let correlation_id = self
            .ffmpeg_install_operation_override
            .as_ref()
            .map_or_else(uuid::Uuid::new_v4, |operation| operation.correlation_id);
        #[cfg(not(test))]
        let correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        #[cfg(test)]
        let operation_result =
            if let Some(operation) = self.ffmpeg_install_operation_override.take() {
                Ok((
                    operation.correlation_id,
                    NetworkProxyMode::Direct,
                    "none",
                    PendingFfmpegInstallControl::Test(operation.cancelled),
                    operation.progress,
                    operation.completion,
                ))
            } else {
                self.engine
                    .start_ffmpeg_install(
                        correlation_id,
                        self.ffmpeg_install_directory.clone(),
                        preserve_executable,
                    )
                    .map(|operation| {
                        (
                            operation.correlation_id,
                            operation.network_proxy_mode,
                            operation.network_proxy_source,
                            PendingFfmpegInstallControl::Core(operation.control),
                            operation.progress,
                            operation.completion,
                        )
                    })
            };
        #[cfg(not(test))]
        let operation_result = self
            .engine
            .start_ffmpeg_install(
                correlation_id,
                self.ffmpeg_install_directory.clone(),
                preserve_executable,
            )
            .map(|operation| {
                (
                    operation.correlation_id,
                    operation.network_proxy_mode,
                    operation.network_proxy_source,
                    PendingFfmpegInstallControl::Core(operation.control),
                    operation.progress,
                    operation.completion,
                )
            });
        let (
            correlation_id,
            network_proxy_mode,
            network_proxy_source,
            control,
            progress,
            completion,
        ) = match operation_result {
            Ok(operation) => operation,
            Err(error) => {
                let message = format!("无法启动 FFmpeg 自动下载：{error}");
                warn!(
                    event = "download.ffmpeg_install.failed",
                    %correlation_id,
                    proxy_mode = "unavailable",
                    proxy_source = "policy",
                    phase = "connecting",
                    failure_kind = "start_rejected",
                    downloaded_bytes = 0,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "managed FFmpeg installation start was rejected"
                );
                self.set_ffmpeg_install_state(FfmpegInstallViewState::Failed(message.clone()), cx);
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("ffmpeg-install-{correlation_id}"),
                        "无法自动下载 FFmpeg",
                        message,
                        cx,
                    );
                });
                return;
            }
        };
        self.pending_ffmpeg_install = Some(PendingFfmpegInstall {
            correlation_id,
            started_at,
            network_proxy_mode,
            network_proxy_source,
            control,
        });
        self.set_ffmpeg_install_state(
            FfmpegInstallViewState::Installing {
                phase: FfmpegInstallPhase::Connecting,
                percent: 0,
                downloaded_bytes: 0,
                total_bytes: None,
                cancelling: false,
            },
            cx,
        );
        cx.spawn(async move |this, cx| {
            while let Ok(update) = progress.recv().await {
                let Some(this) = this.upgrade() else {
                    return;
                };
                this.update(cx, |application, cx| {
                    application.handle_ffmpeg_install_progress(correlation_id, update, cx);
                });
            }
            let Ok(result) = completion.recv().await else {
                return;
            };
            let Some(this) = this.upgrade() else {
                return;
            };
            this.update(cx, |application, cx| {
                application.finish_ffmpeg_install(correlation_id, result, cx);
            });
        })
        .detach();
    }

    fn cancel_ffmpeg_install(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_ffmpeg_install.as_ref() else {
            return;
        };
        pending.control.cancel();
        if let FfmpegInstallViewState::Installing {
            phase,
            percent,
            downloaded_bytes,
            total_bytes,
            ..
        } = self.ffmpeg_install_state.clone()
        {
            self.set_ffmpeg_install_state(
                FfmpegInstallViewState::Installing {
                    phase,
                    percent,
                    downloaded_bytes,
                    total_bytes,
                    cancelling: true,
                },
                cx,
            );
        }
    }

    fn handle_ffmpeg_install_progress(
        &mut self,
        correlation_id: uuid::Uuid,
        progress: FfmpegInstallProgress,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending_ffmpeg_install.as_ref() else {
            return;
        };
        if pending.correlation_id != correlation_id {
            return;
        }
        let cancelling = matches!(
            self.ffmpeg_install_state,
            FfmpegInstallViewState::Installing {
                cancelling: true,
                ..
            }
        );
        self.set_ffmpeg_install_state(
            FfmpegInstallViewState::Installing {
                phase: progress.phase,
                percent: progress.percent,
                downloaded_bytes: progress.downloaded_bytes,
                total_bytes: progress.total_bytes,
                cancelling,
            },
            cx,
        );
    }

    fn finish_ffmpeg_install(
        &mut self,
        correlation_id: uuid::Uuid,
        result: Result<FfmpegInstallation, FfmpegInstallFailure>,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending_ffmpeg_install.take() else {
            return;
        };
        if pending.correlation_id != correlation_id {
            self.pending_ffmpeg_install = Some(pending);
            return;
        }
        let elapsed_ms = pending.started_at.elapsed().as_millis();
        let proxy_mode = nxtools_network::proxy_mode_name(pending.network_proxy_mode);
        let proxy_source = pending.network_proxy_source;
        match result {
            Ok(installation) => {
                if let Err(message) = self.commit_ffmpeg_installation(correlation_id, &installation)
                {
                    warn!(
                        event = "download.ffmpeg_install.failed",
                        %correlation_id,
                        proxy_mode,
                        proxy_source,
                        phase = "configuring",
                        failure_kind = "configuration",
                        downloaded_bytes = installation.downloaded_bytes,
                        elapsed_ms,
                        "managed FFmpeg installation could not be configured"
                    );
                    self.set_ffmpeg_install_state(
                        FfmpegInstallViewState::Failed(message.clone()),
                        cx,
                    );
                    self.notifications.update(cx, |notifications, cx| {
                        notifications.show_error(
                            format!("ffmpeg-install-{correlation_id}"),
                            "FFmpeg 自动安装失败",
                            message,
                            cx,
                        );
                    });
                    return;
                }
                info!(
                    event = "download.ffmpeg_install.completed",
                    %correlation_id,
                    proxy_mode,
                    proxy_source,
                    source = installation.source.as_str(),
                    downloaded_bytes = installation.downloaded_bytes,
                    elapsed_ms,
                    "managed FFmpeg installation completed and was configured"
                );
                self.set_ffmpeg_install_state(FfmpegInstallViewState::Completed, cx);
                cx.emit(DownloadApplicationEvent::FfmpegExecutablePicked {
                    path: installation.executable.clone(),
                    picker_origin: None,
                });
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_success(
                        format!("ffmpeg-install-{correlation_id}"),
                        "FFmpeg 已安装",
                        "已下载到用户目录并自动配置，DASH 双轨任务可以直接合并。",
                        cx,
                    );
                });
            }
            Err(error) if error.is_cancelled() => {
                let downloaded_bytes = error.downloaded_bytes();
                info!(
                    event = "download.ffmpeg_install.cancelled",
                    %correlation_id,
                    proxy_mode,
                    proxy_source,
                    phase = error.phase().as_str(),
                    downloaded_bytes,
                    elapsed_ms,
                    "managed FFmpeg installation cancelled"
                );
                self.set_ffmpeg_install_state(FfmpegInstallViewState::Cancelled, cx);
            }
            Err(error) => {
                let message = error.user_message().to_owned();
                let downloaded_bytes = error.downloaded_bytes();
                warn!(
                    event = "download.ffmpeg_install.failed",
                    %correlation_id,
                    proxy_mode,
                    proxy_source,
                    phase = error.phase().as_str(),
                    failure_kind = error.failure_kind(),
                    native_error_code = error.native_error_code(),
                    http_status = error.http_status(),
                    downloaded_bytes,
                    elapsed_ms,
                    "managed FFmpeg installation failed"
                );
                self.set_ffmpeg_install_state(FfmpegInstallViewState::Failed(message.clone()), cx);
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("ffmpeg-install-{correlation_id}"),
                        "FFmpeg 自动安装失败",
                        message,
                        cx,
                    );
                });
            }
        }
    }

    fn commit_ffmpeg_installation(
        &mut self,
        correlation_id: uuid::Uuid,
        installation: &FfmpegInstallation,
    ) -> Result<(), String> {
        let previous = (!self.config.download.ffmpeg_executable_path.is_empty())
            .then(|| PathBuf::from(&self.config.download.ffmpeg_executable_path));
        if let Err(error) = self
            .engine
            .set_ffmpeg_executable_path(Some(installation.executable.clone()))
        {
            self.discard_unconfigured_ffmpeg(correlation_id, &installation.executable);
            return Err(format!("FFmpeg 已下载，但无法同步到下载引擎：{error}"));
        }
        let mut settings = self.config.download.clone();
        settings.ffmpeg_executable_path = installation.executable.to_string_lossy().into_owned();
        match AppConfig::update_download_settings(&self.config_path, settings) {
            Ok(config) => {
                self.config = config;
                Ok(())
            }
            Err(error) => match self.engine.set_ffmpeg_executable_path(previous) {
                Ok(()) => {
                    self.discard_unconfigured_ffmpeg(correlation_id, &installation.executable);
                    Err(format!("FFmpeg 已下载，但无法保存自动配置：{error}"))
                }
                Err(rollback_error) => {
                    warn!(
                        event = "download.ffmpeg_install.rollback_failed",
                        %correlation_id,
                        operation = "engine_configuration",
                        "managed FFmpeg configuration rollback failed; installed artifact remains owned"
                    );
                    Err(format!(
                        "FFmpeg 已下载，但无法保存配置，且下载引擎回滚失败：{rollback_error}"
                    ))
                }
            },
        }
    }

    fn discard_unconfigured_ffmpeg(&self, correlation_id: uuid::Uuid, executable: &Path) {
        let started_at = Instant::now();
        let Some(directory) = executable.parent() else {
            return;
        };
        let is_owned = directory.parent() == Some(self.ffmpeg_install_directory.as_path())
            && directory
                .file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("managed-"));
        if !is_owned {
            return;
        }
        if let Err(error) = std::fs::remove_dir_all(directory) {
            warn!(
                event = "download.ffmpeg_install.cleanup_failed",
                %correlation_id,
                artifact_kind = "unconfigured_install",
                reason = "configuration_failed",
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "unconfigured managed FFmpeg installation remains owned for later cleanup"
            );
        }
    }

    fn set_ffmpeg_install_state(&mut self, state: FfmpegInstallViewState, cx: &mut Context<Self>) {
        self.ffmpeg_install_state = state.clone();
        cx.emit(DownloadApplicationEvent::FfmpegInstallChanged { state });
        cx.notify();
    }

    fn perform_download_settings_path_action(
        &mut self,
        path_kind: DownloadSettingsPathKind,
        action: DownloadSettingsPathAction,
        cx: &mut Context<Self>,
    ) {
        let presentation = path_kind.presentation();
        let target = match path_kind {
            DownloadSettingsPathKind::Config => self.config_path.to_string_lossy().into_owned(),
            DownloadSettingsPathKind::Logs => self.log_directory.to_string_lossy().into_owned(),
            DownloadSettingsPathKind::Database => self.database_path.to_string_lossy().into_owned(),
        };
        let command = match action {
            DownloadSettingsPathAction::Copy => PlatformCommand::PasteText(PasteTextCommand {
                text: target,
                paste_after_write: false,
                target_window_handle: 0,
                target_focus_window_handle: 0,
            }),
            DownloadSettingsPathAction::Open => {
                PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                    target,
                    action: presentation.shell_action,
                })
            }
        };
        let correlation_id = uuid::Uuid::new_v4();
        let pending = PendingDownloadSettingsPathAction {
            correlation_id,
            started_at: Instant::now(),
            path_kind,
            action,
        };
        info!(
            event = "download.settings_path_action.started",
            %correlation_id,
            path_kind = presentation.key,
            action = action.name(),
            "download settings path action started"
        );
        match self
            .platform
            .send_command_correlated(command, correlation_id)
        {
            Ok(command_id) => {
                self.pending_settings_path_actions
                    .insert(command_id, pending);
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(DOWNLOAD_SETTINGS_PATH_ACTION_TIMEOUT)
                        .await;
                    let _ = this.update(cx, |application, cx| {
                        application.download_settings_path_action_timed_out(command_id, cx);
                    });
                })
                .detach();
            }
            Err(error) => {
                let failure_kind = match error {
                    PlatformBridgeError::Disconnected => "platform_disconnected",
                    PlatformBridgeError::Busy => "queue_busy",
                    PlatformBridgeError::DuplicateCommandId => "duplicate_command_id",
                };
                self.finish_download_settings_path_action_failure(pending, failure_kind, None, cx);
            }
        }
        cx.notify();
    }

    fn download_settings_path_action_timed_out(
        &mut self,
        command_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending_settings_path_actions.remove(&command_id) else {
            return;
        };
        self.finish_download_settings_path_action_failure(pending, "timeout", None, cx);
    }

    fn handle_download_settings_path_action_result(
        &mut self,
        pending: PendingDownloadSettingsPathAction,
        result: PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        let presentation = pending.path_kind.presentation();
        if !result.success {
            self.finish_download_settings_path_action_failure(
                pending,
                "platform",
                result.native_error_code,
                cx,
            );
            return;
        }
        info!(
            event = "download.settings_path_action.completed",
            correlation_id = %pending.correlation_id,
            path_kind = presentation.key,
            action = pending.action.name(),
            elapsed_ms = pending.started_at.elapsed().as_millis(),
            "download settings path action completed"
        );
        if pending.action == DownloadSettingsPathAction::Copy {
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_success(
                    format!("download-settings-path-{}", pending.correlation_id),
                    "路径已复制",
                    format!("{}路径已写入剪贴板", presentation.display_name),
                    cx,
                );
            });
        }
        cx.notify();
    }

    fn finish_download_settings_path_action_failure(
        &mut self,
        pending: PendingDownloadSettingsPathAction,
        failure_kind: &'static str,
        native_error_code: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        let presentation = pending.path_kind.presentation();
        warn!(
            event = "download.settings_path_action.failed",
            correlation_id = %pending.correlation_id,
            path_kind = presentation.key,
            action = pending.action.name(),
            failure_kind,
            native_error_code,
            elapsed_ms = pending.started_at.elapsed().as_millis(),
            "download settings path action failed"
        );
        let (title, message) = match (pending.action, failure_kind) {
            (DownloadSettingsPathAction::Copy, "timeout") => (
                "无法复制诊断路径",
                "Windows 未在 15 秒内确认；操作可能仍会完成。",
            ),
            (DownloadSettingsPathAction::Open, "timeout") => (
                "无法打开诊断路径",
                "Windows 未在 15 秒内确认；操作可能仍会完成。",
            ),
            (DownloadSettingsPathAction::Copy, _) => {
                ("无法复制诊断路径", "Windows 剪贴板暂时不可用，请稍后重试。")
            }
            (DownloadSettingsPathAction::Open, _) => (
                "无法打开诊断路径",
                "Windows 无法打开该位置，请确认目标仍然存在。",
            ),
        };
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_error(
                format!("download-settings-path-{}", pending.correlation_id),
                title,
                message,
                cx,
            );
        });
        cx.notify();
    }

    fn open_ffmpeg_download_page(&mut self, source: FfmpegDownloadSource, cx: &mut Context<Self>) {
        self.begin_ffmpeg_platform_operation(
            PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: source.url().into(),
                action: ShellOpenAction::Open,
            }),
            FfmpegPlatformOperationKind::DownloadPage(source),
            Some(FFMPEG_DOWNLOAD_PAGE_TIMEOUT),
            None,
            cx,
        );
    }

    fn begin_ffmpeg_platform_operation(
        &mut self,
        command: PlatformCommand,
        kind: FfmpegPlatformOperationKind,
        timeout: Option<Duration>,
        picker_origin: Option<FfmpegPickerOrigin>,
        cx: &mut Context<Self>,
    ) {
        let correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        match kind {
            FfmpegPlatformOperationKind::Picker => info!(
                event = "download.ffmpeg_picker.started",
                %correlation_id,
                "FFmpeg executable picker started"
            ),
            FfmpegPlatformOperationKind::DownloadPage(source) => info!(
                event = "download.ffmpeg_download_page.started",
                %correlation_id,
                source = source.name(),
                "public FFmpeg download page open started"
            ),
        }
        #[cfg(test)]
        let send_result = {
            let command_id = uuid::Uuid::new_v4();
            self.ffmpeg_platform_commands
                .borrow_mut()
                .push((command_id, command));
            Ok::<uuid::Uuid, nxtools_ipc::PlatformBridgeError>(command_id)
        };
        #[cfg(not(test))]
        let send_result = self
            .platform
            .send_command_correlated(command, correlation_id);
        match send_result {
            Ok(command_id) => {
                self.pending_ffmpeg_operations.insert(
                    command_id,
                    PendingFfmpegPlatformOperation {
                        correlation_id,
                        started_at,
                        kind,
                        picker_origin,
                    },
                );
                if let Some(timeout) = timeout {
                    cx.spawn(async move |this, cx| {
                        cx.background_executor().timer(timeout).await;
                        let _ = this.update(cx, |application, cx| {
                            application.ffmpeg_platform_operation_timed_out(command_id, cx);
                        });
                    })
                    .detach();
                }
            }
            Err(error) => {
                self.finish_ffmpeg_platform_operation_failure(
                    PendingFfmpegPlatformOperation {
                        correlation_id,
                        started_at,
                        kind,
                        picker_origin,
                    },
                    None,
                    error.to_string(),
                    cx,
                );
            }
        }
        cx.notify();
    }

    fn ffmpeg_platform_operation_timed_out(
        &mut self,
        command_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending_ffmpeg_operations.remove(&command_id) else {
            return;
        };
        self.finish_ffmpeg_platform_operation_failure(
            pending,
            None,
            "Windows 适配器未在时限内返回结果".into(),
            cx,
        );
    }

    fn finish_ffmpeg_platform_operation_failure(
        &mut self,
        pending: PendingFfmpegPlatformOperation,
        native_error_code: Option<i32>,
        user_message: String,
        cx: &mut Context<Self>,
    ) {
        let native_error_present = native_error_code.is_some();
        match pending.kind {
            FfmpegPlatformOperationKind::Picker => {
                warn!(
                    event = "download.ffmpeg_picker.failed",
                    correlation_id = %pending.correlation_id,
                    native_error_present,
                    native_error_code,
                    elapsed_ms = pending.started_at.elapsed().as_millis(),
                    "FFmpeg executable picker failed"
                );
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("download-ffmpeg-picker-{}", pending.correlation_id),
                        "无法选择 FFmpeg",
                        user_message,
                        cx,
                    );
                });
            }
            FfmpegPlatformOperationKind::DownloadPage(source) => {
                warn!(
                    event = "download.ffmpeg_download_page.failed",
                    correlation_id = %pending.correlation_id,
                    source = source.name(),
                    native_error_present,
                    native_error_code,
                    elapsed_ms = pending.started_at.elapsed().as_millis(),
                    "public FFmpeg download page could not be opened"
                );
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!(
                            "download-ffmpeg-page-{}-{}",
                            source.name(),
                            pending.correlation_id
                        ),
                        "无法打开 FFmpeg 下载链接",
                        user_message,
                        cx,
                    );
                });
            }
        }
        cx.notify();
    }

    pub(crate) fn open_settings_window(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = self.settings_window {
            if window
                .update(cx, |_, window, _| {
                    window.activate_window();
                    window.refresh();
                })
                .is_ok()
            {
                return;
            }
            self.handle_settings_window_closed(cx);
        }
        let application = cx.entity();
        let initial = self.settings_view_initial();
        let window_options = download_settings_window_options(cx);
        #[cfg(test)]
        let view_slot = Rc::new(RefCell::new(None));
        #[cfg(test)]
        let opened_view_slot = Rc::clone(&view_slot);
        let opened = cx.open_window(window_options, move |window, cx| {
            let close_application = application.clone();
            let closing_window = window.window_handle();
            window.on_window_should_close(cx, move |_, cx| {
                let application = close_application.clone();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.handle_native_settings_window_closed(closing_window, cx);
                    });
                });
                true
            });
            let view = cx.new(|cx| DownloadSettingsView::new(application, initial, window, cx));
            #[cfg(test)]
            {
                *opened_view_slot.borrow_mut() = Some(view.clone());
            }
            cx.new(|cx| Root::new(view, window, cx))
        });
        match opened {
            Ok(window) => {
                self.settings_window = Some(window.into());
                #[cfg(test)]
                {
                    self.settings_view = view_slot.borrow().clone();
                }
            }
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("download-settings-window-{}", uuid::Uuid::new_v4()),
                        "无法打开下载设置",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
    }

    pub(crate) fn open_taskbar_overlay(&mut self, cx: &mut Context<Self>) {
        if self.taskbar_overlay_window.is_some() {
            return;
        }
        let application = cx.entity();
        let platform = Rc::clone(&self.platform);
        let placement = self.config.download.taskbar_progress_overlay.clone();
        let initial_model = self.taskbar_overlay_model();
        let visible = initial_model.is_visible();
        let overlay_correlation_id = self.taskbar_overlay_diagnostic_correlation_id();
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_native_slot = Rc::clone(&native_slot);
        let window_size = size(px(placement.width as f32), px(placement.height as f32));
        let window_bounds = WindowBounds::centered(window_size, cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                titlebar: None,
                focus: false,
                show: false,
                kind: WindowKind::PopUp,
                is_movable: true,
                is_resizable: true,
                is_minimizable: false,
                display_id: None,
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.DownloadTaskbarProgress".into()),
                window_min_size: Some(size(px(72.0), px(6.0))),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
                ..Default::default()
            },
            move |window, cx| {
                let close_application = application.clone();
                let closing_window = window.window_handle();
                window.on_window_should_close(cx, move |_, cx| {
                    let intent = close_application
                        .read(cx)
                        .taskbar_overlay_native_close_intent(closing_window);
                    if let Some(intent) = intent {
                        let application = close_application.clone();
                        cx.defer(move |cx| {
                            application.update(cx, |application, cx| {
                                application.handle_native_taskbar_overlay_close(
                                    closing_window,
                                    intent,
                                    cx,
                                );
                            });
                        });
                    }
                    !matches!(
                        intent,
                        Some(TaskbarOverlayNativeCloseIntent::FinishEdit { .. })
                    )
                });
                let window_handle = if cfg!(test) {
                    0
                } else {
                    crate::ui::native_window_handle(window).unwrap_or_default()
                };
                *opened_native_slot.borrow_mut() = window_handle;
                if window_handle != 0 {
                    let command = taskbar_overlay_placement_command(
                        window_handle,
                        &placement,
                        visible,
                        false,
                    );
                    match platform.send_command_correlated(
                        PlatformCommand::PlaceTaskbarProgressOverlay(command),
                        overlay_correlation_id,
                    ) {
                        Ok(correlation_id) => info!(
                            event = "download.taskbar_overlay.accepted",
                            %correlation_id,
                            window_handle,
                            "taskbar overlay placement accepted"
                        ),
                        Err(error) => warn!(
                            event = "download.taskbar_overlay.accept_failed",
                            %error,
                            window_handle,
                            "taskbar overlay placement could not be queued"
                        ),
                    }
                }
                let view = cx.new(|cx| {
                    DownloadTaskbarOverlayView::new(
                        application.clone(),
                        Rc::clone(&platform),
                        window_handle,
                        initial_model,
                        window,
                        cx,
                    )
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                self.taskbar_overlay_native_window_handle = *native_slot.borrow();
                self.taskbar_overlay_window = Some(window.into());
            }
            Err(error) => warn!(
                event = "download.taskbar_overlay.open_failed",
                %error,
                "could not open the taskbar progress overlay"
            ),
        }
    }

    fn taskbar_overlay_native_close_intent(
        &self,
        closing_window: AnyWindowHandle,
    ) -> Option<TaskbarOverlayNativeCloseIntent> {
        if self.taskbar_overlay_window != Some(closing_window) {
            return None;
        }
        Some(if self.taskbar_overlay_edit_draft.is_some() {
            TaskbarOverlayNativeCloseIntent::FinishEdit {
                edit_correlation_id: self.taskbar_overlay_edit_correlation_id,
            }
        } else {
            TaskbarOverlayNativeCloseIntent::ClosePassive
        })
    }

    fn handle_native_taskbar_overlay_close(
        &mut self,
        closing_window: AnyWindowHandle,
        intent: TaskbarOverlayNativeCloseIntent,
        cx: &mut Context<Self>,
    ) {
        if self.taskbar_overlay_window != Some(closing_window) {
            return;
        }
        match intent {
            TaskbarOverlayNativeCloseIntent::FinishEdit {
                edit_correlation_id,
            } if self.taskbar_overlay_edit_draft.is_some()
                && self.taskbar_overlay_edit_correlation_id == edit_correlation_id =>
            {
                self.finish_taskbar_overlay_edit(
                    false,
                    edit_correlation_id.unwrap_or(self.taskbar_overlay_correlation_id),
                    cx,
                );
            }
            TaskbarOverlayNativeCloseIntent::ClosePassive
                if self.taskbar_overlay_edit_draft.is_none() =>
            {
                self.taskbar_overlay_window = None;
                self.taskbar_overlay_native_window_handle = 0;
            }
            _ => {}
        }
    }

    fn apply_taskbar_overlay_placement(
        &mut self,
        correlation_id: uuid::Uuid,
    ) -> Result<uuid::Uuid, String> {
        let window_handle = self.taskbar_overlay_native_window_handle;
        if window_handle == 0 {
            return Err("任务栏下载进度窗口尚未就绪".into());
        }
        self.taskbar_overlay_correlation_id = correlation_id;
        self.platform
            .send_command_correlated(
                PlatformCommand::PlaceTaskbarProgressOverlay(taskbar_overlay_placement_command(
                    window_handle,
                    &self.config.download.taskbar_progress_overlay,
                    self.taskbar_overlay_model().is_visible(),
                    false,
                )),
                correlation_id,
            )
            .map_err(|error| format!("无法应用任务栏进度条位置：{error}"))
    }

    fn begin_taskbar_overlay_edit(
        &mut self,
        mut settings: DownloadTaskbarProgressOverlaySettings,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        settings.enabled = true;
        self.taskbar_overlay_edit_draft = Some(settings.clone());
        self.taskbar_overlay_edit_correlation_id = Some(correlation_id);
        self.taskbar_overlay_edit_gesture_active = false;
        self.pending_taskbar_overlay_gestures.clear();
        let window_handle = self.taskbar_overlay_native_window_handle;
        if window_handle == 0 {
            self.taskbar_overlay_edit_draft = None;
            self.taskbar_overlay_edit_correlation_id = None;
            return Err("任务栏覆盖层窗口尚未就绪".into());
        }
        self.platform
            .send_command_correlated(
                PlatformCommand::PlaceTaskbarProgressOverlay(taskbar_overlay_placement_command(
                    window_handle,
                    &settings,
                    true,
                    true,
                )),
                correlation_id,
            )
            .map_err(|error| {
                self.taskbar_overlay_edit_draft = None;
                self.taskbar_overlay_edit_correlation_id = None;
                format!("无法进入任务栏覆盖层编辑模式：{error}")
            })?;
        info!(
            event = "download.taskbar_overlay.edit_started",
            %correlation_id,
            width = settings.width,
            height = settings.height,
            "taskbar overlay edit mode started"
        );
        cx.emit(DownloadApplicationEvent::TaskbarOverlayEditChanged { settings });
        cx.notify();
        Ok(())
    }

    fn update_taskbar_overlay_edit(
        &mut self,
        mut settings: DownloadTaskbarProgressOverlaySettings,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if self.taskbar_overlay_edit_draft.is_none() {
            return Ok(());
        }
        let correlation_id = self
            .taskbar_overlay_edit_correlation_id
            .unwrap_or(correlation_id);
        settings.enabled = true;
        let mut wrapper = DownloadSettings {
            taskbar_progress_overlay: settings,
            ..DownloadSettings::default()
        };
        wrapper.normalize();
        let settings = wrapper.taskbar_progress_overlay;
        let window_handle = self.taskbar_overlay_native_window_handle;
        if window_handle == 0 {
            return Err("任务栏覆盖层窗口尚未就绪".into());
        }
        self.platform
            .send_command_correlated(
                PlatformCommand::PlaceTaskbarProgressOverlay(taskbar_overlay_placement_command(
                    window_handle,
                    &settings,
                    true,
                    true,
                )),
                correlation_id,
            )
            .map_err(|error| format!("无法刷新任务栏覆盖层编辑预览：{error}"))?;
        self.taskbar_overlay_edit_draft = Some(settings.clone());
        info!(
            event = "download.taskbar_overlay.edit_preview_changed",
            %correlation_id,
            progress_mode = ?settings.progress_mode,
            show_progress = settings.show_progress,
            show_speed = settings.show_speed,
            show_remaining_time = settings.show_remaining_time,
            show_active_tasks = settings.show_active_tasks,
            show_active_connections = settings.show_active_connections,
            show_transferred_bytes = settings.show_transferred_bytes,
            "taskbar overlay edit preview changed"
        );
        cx.emit(DownloadApplicationEvent::TaskbarOverlayEditChanged { settings });
        cx.notify();
        Ok(())
    }

    pub(crate) fn record_taskbar_overlay_edit_bounds(
        &mut self,
        mut settings: DownloadTaskbarProgressOverlaySettings,
        _correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        if self.taskbar_overlay_edit_draft.is_none() {
            return;
        }
        let raw_settings = settings.clone();
        let mut wrapper = DownloadSettings {
            taskbar_progress_overlay: settings.clone(),
            ..DownloadSettings::default()
        };
        wrapper.normalize();
        settings = wrapper.taskbar_progress_overlay;
        if settings != raw_settings && self.taskbar_overlay_native_window_handle != 0 {
            let correlation_id = self.taskbar_overlay_diagnostic_correlation_id();
            if let Err(error) = self.platform.send_command_correlated(
                PlatformCommand::PlaceTaskbarProgressOverlay(taskbar_overlay_placement_command(
                    self.taskbar_overlay_native_window_handle,
                    &settings,
                    true,
                    true,
                )),
                correlation_id,
            ) {
                warn!(
                    event = "download.taskbar_overlay.edit_normalize_failed",
                    %correlation_id,
                    %error,
                    "normalized taskbar overlay edit bounds could not be applied"
                );
            }
        }
        if self.taskbar_overlay_edit_draft.as_ref() == Some(&settings) {
            return;
        }
        self.taskbar_overlay_edit_draft = Some(settings.clone());
        cx.emit(DownloadApplicationEvent::TaskbarOverlayEditChanged { settings });
        cx.notify();
    }

    fn finish_taskbar_overlay_edit(
        &mut self,
        committed: bool,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        if self.taskbar_overlay_edit_draft.take().is_none() {
            return;
        }
        self.taskbar_overlay_edit_gesture_active = false;
        self.pending_taskbar_overlay_gestures.clear();
        let correlation_id = self
            .taskbar_overlay_edit_correlation_id
            .take()
            .unwrap_or(correlation_id);
        self.taskbar_overlay_correlation_id = correlation_id;
        let settings = &self.config.download.taskbar_progress_overlay;
        let visible = self.taskbar_overlay_model().is_visible();
        if self.taskbar_overlay_native_window_handle != 0
            && let Err(error) = self.platform.send_command_correlated(
                PlatformCommand::PlaceTaskbarProgressOverlay(taskbar_overlay_placement_command(
                    self.taskbar_overlay_native_window_handle,
                    settings,
                    visible,
                    false,
                )),
                correlation_id,
            )
        {
            warn!(
                event = "download.taskbar_overlay.edit_finish_failed",
                %correlation_id,
                %error,
                committed,
                "taskbar overlay could not leave edit mode"
            );
        } else {
            info!(
                event = "download.taskbar_overlay.edit_finished",
                %correlation_id,
                committed,
                "taskbar overlay edit mode finished"
            );
        }
        cx.notify();
    }

    fn handle_native_settings_window_closed(
        &mut self,
        window: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        if self.settings_window == Some(window) {
            self.handle_settings_window_closed(cx);
        } else if self
            .pending_directory_picker
            .as_ref()
            .is_some_and(|pending| pending.request.window == window)
        {
            self.retire_directory_picker("window_closed", cx);
        }
    }

    fn handle_settings_window_closed(&mut self, cx: &mut Context<Self>) {
        self.retire_extension_export_selection(cx);
        self.pending_ffmpeg_operations.retain(|_, pending| {
            if pending.kind != FfmpegPlatformOperationKind::Picker {
                return true;
            }
            info!(
                event = "download.ffmpeg_picker.retired",
                correlation_id = %pending.correlation_id,
                reason = "window_closed",
                elapsed_ms = pending.started_at.elapsed().as_millis(),
                "FFmpeg executable selection retired"
            );
            false
        });
        if self
            .pending_directory_picker
            .as_ref()
            .is_some_and(|pending| Some(pending.request.window) == self.settings_window)
        {
            self.retire_directory_picker("window_closed", cx);
        }
        self.settings_window = None;
        #[cfg(test)]
        {
            self.settings_view = None;
        }
        if self.taskbar_overlay_edit_draft.is_some() {
            self.finish_taskbar_overlay_edit(false, uuid::Uuid::new_v4(), cx);
        }
    }

    fn apply_settings(
        &mut self,
        draft: DownloadSettingsDraft,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let started_at = Instant::now();
        info!(
            event = "download.settings.apply_started",
            %correlation_id,
            "download settings apply started"
        );
        let failed = |error_kind: &'static str, message: String| {
            warn!(
                event = "download.settings.apply_failed",
                %correlation_id,
                error_kind,
                elapsed_ms = started_at.elapsed().as_millis(),
                "download settings apply failed"
            );
            Err(message)
        };
        if !nxtools_core::is_valid_chrome_web_store_extension_id(
            &draft.chrome_web_store_extension_id,
        ) {
            return failed(
                "validation",
                "Chrome 商店扩展 ID 必须留空或为 32 位 a–p 小写字母".into(),
            );
        }
        let browser_identity_changed = self.config.download.chrome_web_store_extension_id
            != draft.chrome_web_store_extension_id;
        if draft.default_directory.exists() && !draft.default_directory.is_dir() {
            return failed("validation", "默认保存位置不是文件夹".into());
        }
        if let Err(error) = std::fs::create_dir_all(&draft.default_directory) {
            return failed("filesystem", format!("无法创建默认保存目录：{error}"));
        }
        let write_probe = draft
            .default_directory
            .join(format!(".nxtools-write-probe-{correlation_id}"));
        if let Err(error) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&write_probe)
        {
            return failed("filesystem", format!("默认保存目录不可写：{error}"));
        }
        let _ = std::fs::remove_file(&write_probe);

        let previous_concurrency = match self.execute_correlated(
            DownloadCommand::Snapshot {
                query: DownloadQuery::default(),
            },
            correlation_id,
        ) {
            DownloadCommandOutcome::Snapshot(snapshot) => snapshot
                .queues
                .iter()
                .find(|queue| queue.id == "default")
                .and_then(|queue| queue.max_concurrent_tasks)
                .unwrap_or(3),
            DownloadCommandOutcome::Rejected { fault } => return failed("queue", fault.message),
            _ => return failed("queue", "下载引擎返回了无效的队列状态".into()),
        };
        let previous_speed_limit = self.engine.global_speed_limit();
        let next_download_settings = DownloadSettings {
            default_directory: draft.default_directory.to_string_lossy().into_owned(),
            chrome_web_store_extension_id: draft.chrome_web_store_extension_id.clone(),
            automatic_categories_enabled: draft.automatic_categories_enabled,
            categories: draft.categories.clone(),
            duplicate_strategy: draft.duplicate_strategy,
            double_click_action: draft.double_click_action,
            task_list_sort: draft.task_list_sort.clone(),
            task_list_sort_descending: draft.task_list_sort_descending,
            ffmpeg_executable_path: draft.ffmpeg_executable_path.clone(),
            default_request_headers: draft.default_request_headers.clone(),
            default_cookie: draft.default_cookie.clone(),
            accept_invalid_certificates: draft.accept_invalid_certificates,
            default_connection_limit: draft.default_connection_limit,
            adaptive_segment_scaling_enabled: self.config.download.adaptive_segment_scaling_enabled,
            adaptive_segment_max_segments: self.config.download.adaptive_segment_max_segments,
            adaptive_segment_min_mib: self.config.download.adaptive_segment_min_mib,
            show_progress_window: draft.show_progress_window,
            show_completion_window: draft.show_completion_window,
            completion_sound_enabled: draft.completion_sound_enabled,
            open_directory_on_completion: draft.open_directory_on_completion,
            task_removal_deletes_file: draft.task_removal_deletes_file,
            taskbar_progress_overlay: draft.taskbar_progress_overlay.clone(),
            background_speed_limit: draft.background_speed_limit.clone(),
        };
        if let Err(error) =
            nxtools_download::DownloadHttpPolicy::from_download_settings(&next_download_settings)
        {
            return failed(
                "validation",
                format!("请求 Header / Cookie 设置无效：{error}"),
            );
        }
        if let Err(error) = nxtools_download::DownloadOrganizationPolicy::from_download_settings(
            &next_download_settings,
        ) {
            return failed("validation", format!("下载分类设置无效：{error}"));
        }
        let change_concurrency = draft
            .max_concurrent_tasks_baseline
            .is_none_or(|baseline| baseline != draft.max_concurrent_tasks);
        let change_speed_limit = draft.global_speed_limit_edited;
        if change_concurrency
            && let Err(error) =
                self.set_default_queue_concurrency(draft.max_concurrent_tasks, correlation_id)
        {
            return failed("queue", error);
        }
        if change_speed_limit
            && let Err(error) = self
                .set_global_speed_limit(draft.global_speed_limit_bytes_per_second, correlation_id)
        {
            if change_concurrency
                && let Err(rollback_error) =
                    self.set_default_queue_concurrency(previous_concurrency, correlation_id)
            {
                return failed(
                    "rollback",
                    format!("{error}；同时无法回滚队列并发：{rollback_error}"),
                );
            }
            return failed("speed_limit", error);
        }

        let baseline = draft.baseline.as_ref().unwrap_or(&self.config.download);
        let next_config = match AppConfig::merge_download_settings(
            &self.config_path,
            baseline,
            next_download_settings,
        ) {
            Ok(config) => config,
            Err(error) => {
                let mut rollback_errors = Vec::new();
                if change_concurrency
                    && let Err(rollback_error) =
                        self.set_default_queue_concurrency(previous_concurrency, correlation_id)
                {
                    rollback_errors.push(format!("队列并发：{rollback_error}"));
                }
                if change_speed_limit
                    && let Err(rollback_error) =
                        self.set_global_speed_limit(previous_speed_limit, correlation_id)
                {
                    rollback_errors.push(format!("全局限速：{rollback_error}"));
                }
                let rollback = if rollback_errors.is_empty() {
                    String::new()
                } else {
                    format!("；同时无法完整回滚 {}", rollback_errors.join("、"))
                };
                return failed(
                    "persistence",
                    format!("无法保存下载设置：{error}{rollback}"),
                );
            }
        };
        self.config = next_config;
        if browser_identity_changed
            || !self
                .config
                .download
                .chrome_web_store_extension_id
                .is_empty()
        {
            self.request_browser_registration(correlation_id, cx);
        }
        self.apply_background_speed_policy(correlation_id)?;
        if let Err(error) = self
            .engine
            .update_download_http_policy(&self.config.download)
        {
            return failed(
                "runtime_sync",
                format!("下载设置已保存，但无法同步请求策略：{error}"),
            );
        }
        if let Err(error) = self
            .engine
            .update_download_organization_policy(&self.config.download)
        {
            return failed(
                "runtime_sync",
                format!("下载设置已保存，但无法同步分类策略：{error}"),
            );
        }
        let configured_ffmpeg = (!self.config.download.ffmpeg_executable_path.is_empty())
            .then(|| PathBuf::from(&self.config.download.ffmpeg_executable_path));
        if let Err(error) = self.engine.set_ffmpeg_executable_path(configured_ffmpeg) {
            return failed(
                "runtime_sync",
                format!("FFmpeg 路径已保存，但无法同步到下载引擎：{error}"),
            );
        }
        if !self.config.download.default_directory.is_empty() {
            self.default_directory = PathBuf::from(&self.config.download.default_directory);
        }
        self.show_progress_window = self.config.download.show_progress_window;
        self.show_completion_window = self.config.download.show_completion_window;
        let was_editing_taskbar_overlay = self.taskbar_overlay_edit_draft.take().is_some();
        let edit_correlation_id = self.taskbar_overlay_edit_correlation_id.take();
        match self.apply_taskbar_overlay_placement(correlation_id) {
            Ok(overlay_correlation_id) => info!(
                event = "download.taskbar_overlay.accepted",
                correlation_id = %overlay_correlation_id,
                settings_correlation_id = %correlation_id,
                "updated taskbar overlay placement accepted"
            ),
            Err(error) => warn!(
                event = "download.taskbar_overlay.apply_failed",
                %correlation_id,
                %error,
                "download settings saved but taskbar overlay placement failed"
            ),
        }
        if was_editing_taskbar_overlay {
            let correlation_id = edit_correlation_id.unwrap_or(correlation_id);
            info!(
                event = "download.taskbar_overlay.edit_finished",
                %correlation_id,
                committed = true,
                "taskbar overlay edit mode committed with download settings"
            );
        }
        self.refresh(cx);
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_success(
                format!("download-settings-saved-{correlation_id}"),
                "下载设置已保存",
                "新任务将使用更新后的下载策略。",
                cx,
            );
        });
        info!(
            event = "download.settings.apply_succeeded",
            %correlation_id,
            elapsed_ms = started_at.elapsed().as_millis(),
            "download settings applied"
        );
        Ok(())
    }

    fn set_default_queue_concurrency(
        &self,
        max_concurrent_tasks: u16,
        correlation_id: uuid::Uuid,
    ) -> Result<(), String> {
        match self.execute_correlated(
            DownloadCommand::Queue {
                action: DownloadQueueAction::SetMaxConcurrent {
                    queue_id: "default".into(),
                    max_concurrent_tasks,
                },
            },
            correlation_id,
        ) {
            DownloadCommandOutcome::QueueChanged { .. } => Ok(()),
            DownloadCommandOutcome::Rejected { fault } => Err(fault.message),
            _ => Err("下载引擎返回了无效的队列设置结果".into()),
        }
    }

    fn set_global_speed_limit(
        &self,
        bytes_per_second: Option<u64>,
        correlation_id: uuid::Uuid,
    ) -> Result<(), String> {
        match self.execute_correlated(
            DownloadCommand::SetGlobalSpeedLimit { bytes_per_second },
            correlation_id,
        ) {
            DownloadCommandOutcome::Accepted { task_id: None } => Ok(()),
            DownloadCommandOutcome::Rejected { fault } => Err(fault.message),
            _ => Err("下载引擎返回了无效的全局限速设置结果".into()),
        }
    }

    fn apply_background_speed_policy(&self, correlation_id: uuid::Uuid) -> Result<(), String> {
        let policy = &self.config.download.background_speed_limit;
        let cap = (policy.enabled
            && self
                .foreground_process_id
                .is_some_and(|process_id| process_id != std::process::id()))
        .then_some(policy.bytes_per_second);
        self.engine
            .set_runtime_speed_cap(cap, correlation_id)
            .map_err(|_| "下载设置已保存，但临时限速未能应用，请重试。".into())
    }

    fn defer_effective_limit_display(&self, cx: &mut Context<Self>) {
        let limit = self.engine.effective_global_speed_limit();
        let views = self
            .task_windows
            .values()
            .map(|entry| entry.view.downgrade())
            .collect::<Vec<_>>();
        cx.defer(move |cx| {
            for view in views {
                let _ = view.update(cx, |view, cx| {
                    if view.effective_global_speed_limit != limit {
                        view.effective_global_speed_limit = limit;
                        cx.notify();
                    }
                });
            }
        });
    }

    pub(crate) fn handle_platform_notification(
        &mut self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        if self.handle_new_download_clipboard(&notification, cx) {
            return;
        }
        if self.handle_browser_store_page(&notification, cx) {
            return;
        }
        if let PlatformNotification::Event(PlatformEvent::ForegroundChanged(foreground)) =
            &notification
        {
            let previous_limit = self.engine.effective_global_speed_limit();
            self.foreground_process_id = (foreground.window_handle != 0
                && foreground.process_id != 0)
                .then_some(foreground.process_id);
            if self
                .apply_background_speed_policy(uuid::Uuid::new_v4())
                .is_err()
            {
                warn!(
                    event = "download.background_limit.apply_failed",
                    failure_kind = "runtime_policy",
                    "foreground limit policy could not be applied"
                );
            }
            if previous_limit != self.engine.effective_global_speed_limit() {
                self.defer_effective_limit_display(cx);
            }
            return;
        }
        if matches!(&notification, PlatformNotification::Connected) {
            self.request_task_icons(cx);
            let correlation_id = uuid::Uuid::new_v4();
            self.request_browser_registration(correlation_id, cx);
            match self.apply_taskbar_overlay_placement(correlation_id) {
                Ok(command_id) => info!(
                    event = "download.taskbar_overlay.reconnect_restore_accepted",
                    %correlation_id,
                    %command_id,
                    window_handle = self.taskbar_overlay_native_window_handle,
                    "saved taskbar overlay placement queued after PlatformHost connected"
                ),
                Err(error) => warn!(
                    event = "download.taskbar_overlay.reconnect_restore_failed",
                    %correlation_id,
                    %error,
                    window_handle = self.taskbar_overlay_native_window_handle,
                    "saved taskbar overlay placement could not be restored after PlatformHost connected"
                ),
            }
            return;
        }
        if let PlatformNotification::Disconnected {
            generation,
            pending_command_ids,
        } = &notification
        {
            self.foreground_process_id = None;
            let _ = self.apply_background_speed_policy(uuid::Uuid::new_v4());
            self.defer_effective_limit_display(cx);
            warn!(
                event = "downloads.platform_disconnected",
                generation,
                pending_command_count = pending_command_ids.len(),
                failed_icon_resource_count = self.failed_task_icon_resources.len(),
                "downloads invalidated commands from the disconnected PlatformHost session"
            );
            if let Some(pending) = self.pending_new_download_handoff.as_ref() {
                self.finish_new_download_handoff(pending.command_id, false, "disconnected", cx);
            }
            let pending_captures = self
                .pending_controlled_progress
                .values()
                .filter_map(|pending| match pending.placement {
                    ControlledProgressPlacement::Capturing { command_id, .. } => Some(command_id),
                    _ => None,
                })
                .collect::<Vec<_>>();
            for command_id in pending_captures {
                self.finish_controlled_progress_anchor(command_id, false, "disconnected", None, cx);
            }
            self.disconnect_artifact_actions(cx);
            self.pending_task_icons.clear();
            self.pending_task_icon_resources.clear();
            self.failed_task_icon_resources.clear();
            self.pending_task_window_activations.clear();
            self.pending_taskbar_overlay_gestures.clear();
            self.retire_directory_picker("disconnected", cx);
            self.pending_settings_path_actions.clear();
            self.finish_browser_registration(false, None, "disconnected", cx);
            if let Some(pending) = self.pending_extension_export.take() {
                let elapsed = pending.started_at.elapsed().as_millis();
                self.publish_extension_export_status(
                    pending.operation_id,
                    "导出失败:Windows 连接已断开".into(),
                    cx,
                );
                warn!(
                    event = "download.browser_extension.export_failed",
                    command_id = %pending.command_id,
                    format = pending.format,
                    failure_kind = "platform_disconnected",
                    elapsed_ms = elapsed,
                    "browser extension export was invalidated by PlatformHost disconnect"
                );
            }
            self.pending_extension_export = None;
            self.pending_ffmpeg_operations.clear();
            self.completion_feedback_commands.clear();
            self.disconnect_completion_action(cx);
            self.taskbar_overlay_edit_gesture_active = false;
            cx.notify();
            return;
        }
        if let PlatformNotification::Event(PlatformEvent::TaskbarOverlayEditGestureFinished(
            gesture,
        )) = &notification
        {
            if gesture.window_handle == self.taskbar_overlay_native_window_handle {
                self.taskbar_overlay_edit_gesture_active = false;
                self.pending_taskbar_overlay_gestures.clear();
                cx.notify();
            }
            return;
        }
        let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = notification
        else {
            return;
        };
        if self.finish_controlled_progress_anchor(
            correlation_id,
            result.success,
            "native_result",
            result.native_error_code,
            cx,
        ) {
            return;
        }
        if self
            .pending_new_download_handoff
            .as_ref()
            .is_some_and(|pending| pending.command_id == correlation_id)
        {
            let application = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = application.update(cx, |application, cx| {
                    application.finish_new_download_handoff(
                        correlation_id,
                        result.success,
                        "native_result",
                        cx,
                    );
                });
            });
            return;
        }
        if self.handle_artifact_action_result(correlation_id, &result, cx) {
            return;
        }
        if let Some(pending) = self.pending_task_icons.remove(&correlation_id) {
            let native_error_code = result.native_error_code;
            let sta_unavailable = result.error.as_deref() == Some("shell_icon_sta_unavailable");
            let error_present = result.error.is_some();
            self.pending_task_icon_resources
                .remove(&pending.resource_key);
            let icon_path = result
                .success
                .then_some(result.data.as_deref())
                .flatten()
                .map(PathBuf::from)
                .filter(|path| shell_icon_path_is_renderable(path));
            if let Some(path) = icon_path {
                self.failed_task_icon_resources
                    .remove(&pending.resource_key);
                let resource_key = pending.resource_key;
                self.task_icon_resources
                    .insert(resource_key.clone(), path.clone());
                for task_id in pending.task_ids {
                    if self.tasks.iter().any(|task| task.task.id() == task_id)
                        && self.task_icon_resource_assignments.get(&task_id) == Some(&resource_key)
                    {
                        self.task_icons.insert(task_id, path.clone());
                    }
                }
                info!(
                    event = "download.list.icon_completed",
                    %correlation_id,
                    success = true,
                    resource_scope = pending.resource_scope,
                    elapsed_ms = pending.started_at.elapsed().as_millis(),
                    "download task Shell icon request completed"
                );
            } else {
                if pending.resource_scope == "target" && native_error_code == Some(2) {
                    self.failed_task_icon_resources.insert(
                        pending.resource_key.clone(),
                        DownloadTaskIconFailure {
                            attempts: DOWNLOAD_TASK_ICON_MAX_ATTEMPTS,
                            retry_token: None,
                        },
                    );
                } else {
                    self.record_task_icon_failure(
                        pending.resource_key.clone(),
                        sta_unavailable,
                        cx,
                    );
                }
                let failure = &self.failed_task_icon_resources[&pending.resource_key];
                warn!(
                    event = "download.list.icon_failed",
                    %correlation_id,
                    success = false,
                    resource_scope = pending.resource_scope,
                    elapsed_ms = pending.started_at.elapsed().as_millis(),
                    ?native_error_code,
                    error_present,
                    attempts = failure.attempts,
                    retry_scheduled = failure.retry_token.is_some(),
                    "download task Shell icon request failed"
                );
            }
            self.request_task_icons(cx);
            cx.notify();
            return;
        }
        if let Some(operation_id) = self.completion_feedback_commands.remove(&correlation_id) {
            self.handle_completion_feedback_result(operation_id, correlation_id, result, cx);
            return;
        }
        if self
            .pending_shutdown_command
            .as_ref()
            .is_some_and(|pending| pending.command_id == correlation_id)
        {
            self.handle_shutdown_result(correlation_id, result, cx);
            return;
        }
        if self
            .pending_taskbar_overlay_gestures
            .remove(&correlation_id)
        {
            if !result.success {
                self.taskbar_overlay_edit_gesture_active = false;
                cx.notify();
            }
            return;
        }
        if let Some(pending) = self.pending_task_window_activations.remove(&correlation_id) {
            if result.success {
                info!(
                    event = "download.task_window.activation_completed",
                    success = true,
                    %correlation_id,
                    task_id = %pending.task_id.as_uuid(),
                    window_handle = pending.window.native_window_handle,
                    window_generation = %pending.window.generation,
                    elapsed_ms = pending.started_at.elapsed().as_millis(),
                    "download task window activation completed"
                );
            } else {
                warn!(
                    event = "download.task_window.activation_failed",
                    success = false,
                    %correlation_id,
                    task_id = %pending.task_id.as_uuid(),
                    window_handle = pending.window.native_window_handle,
                    window_generation = %pending.window.generation,
                    elapsed_ms = pending.started_at.elapsed().as_millis(),
                    "download task window activation failed"
                );
                if pending.activation.replace_on_failure() {
                    self.schedule_task_window_replacement(
                        pending.task_id,
                        pending.window,
                        pending.activation.phase(),
                        "platform_activation_failed",
                        cx,
                    );
                }
            }
            return;
        }
        if let Some(pending) = self.pending_settings_path_actions.remove(&correlation_id) {
            self.handle_download_settings_path_action_result(pending, result, cx);
            return;
        }
        if self
            .pending_browser_registration
            .is_some_and(|(id, _)| id == correlation_id)
        {
            self.finish_browser_registration(
                result.success,
                result.native_error_code,
                "native_result",
                cx,
            );
            return;
        }
        if self.handle_browser_extension_export_result(correlation_id, &result, cx) {
            return;
        }
        if let Some(pending) = self.pending_ffmpeg_operations.remove(&correlation_id) {
            if !result.success {
                let reason = result
                    .error
                    .unwrap_or_else(|| "Windows 未返回错误详情".into());
                self.finish_ffmpeg_platform_operation_failure(
                    pending,
                    result.native_error_code,
                    reason,
                    cx,
                );
                return;
            }
            match pending.kind {
                FfmpegPlatformOperationKind::Picker => {
                    if let Some(path) = result.data.filter(|path| !path.trim().is_empty()) {
                        info!(
                            event = "download.ffmpeg_picker.succeeded",
                            correlation_id = %pending.correlation_id,
                            elapsed_ms = pending.started_at.elapsed().as_millis(),
                            "FFmpeg executable picker completed"
                        );
                        cx.emit(DownloadApplicationEvent::FfmpegExecutablePicked {
                            path: PathBuf::from(path),
                            picker_origin: pending.picker_origin,
                        });
                    } else {
                        info!(
                            event = "download.ffmpeg_picker.cancelled",
                            correlation_id = %pending.correlation_id,
                            elapsed_ms = pending.started_at.elapsed().as_millis(),
                            "FFmpeg executable picker completed without a selection"
                        );
                    }
                }
                FfmpegPlatformOperationKind::DownloadPage(source) => {
                    info!(
                        event = "download.ffmpeg_download_page.completed",
                        correlation_id = %pending.correlation_id,
                        source = source.name(),
                        elapsed_ms = pending.started_at.elapsed().as_millis(),
                        "public FFmpeg download page opened"
                    );
                }
            }
            cx.notify();
            return;
        }
        self.handle_directory_result(correlation_id, result, cx);
    }

    fn handle_directory_result(
        &mut self,
        correlation_id: uuid::Uuid,
        result: PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        self.receive_directory_result(correlation_id, result, cx);
    }
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.refresh_snapshot(true, cx);
    }

    fn record_task_icon_failure(
        &mut self,
        resource_key: String,
        sta_unavailable: bool,
        cx: &mut Context<Self>,
    ) {
        let attempts = if sta_unavailable {
            self.failed_task_icon_resources
                .get(&resource_key)
                .map_or(0, |failure| failure.attempts)
        } else {
            self.failed_task_icon_resources
                .get(&resource_key)
                .map_or(1, |failure| failure.attempts + 1)
                .min(DOWNLOAD_TASK_ICON_MAX_ATTEMPTS)
        };
        let retry_token = (attempts < DOWNLOAD_TASK_ICON_MAX_ATTEMPTS).then(uuid::Uuid::new_v4);
        self.failed_task_icon_resources.insert(
            resource_key.clone(),
            DownloadTaskIconFailure {
                attempts,
                retry_token,
            },
        );
        let Some(retry_token) = retry_token else {
            return;
        };
        let retry_delay = if sta_unavailable {
            DOWNLOAD_TASK_ICON_STA_RETRY_DELAY
        } else {
            DOWNLOAD_TASK_ICON_RETRY_DELAY * (1 << (attempts - 1))
        };
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(retry_delay).await;
            let Some(this) = this.upgrade() else {
                return;
            };
            this.update(cx, |application, cx| {
                let Some(failure) = application
                    .failed_task_icon_resources
                    .get_mut(&resource_key)
                else {
                    return;
                };
                if failure.retry_token != Some(retry_token) {
                    return;
                }
                failure.retry_token = None;
                application.request_task_icons(cx);
                cx.notify();
            });
        })
        .detach();
    }

    fn request_task_icons(&mut self, cx: &mut Context<Self>) {
        let live_task_ids = self
            .tasks
            .iter()
            .map(|task| task.task.id())
            .collect::<HashSet<_>>();
        self.task_icons
            .retain(|task_id, _| live_task_ids.contains(task_id));
        self.task_icon_resource_assignments
            .retain(|task_id, _| live_task_ids.contains(task_id));

        let descriptors = self
            .tasks
            .iter()
            .filter_map(|task| {
                let resource_key = download_task_icon_resource_key(task)?;
                let target = download_task_icon_target(task)?;
                let (resource_key, target) = if resource_key.ends_with(":type:generic")
                    || (resource_key.contains(":target:")
                        && self
                            .failed_task_icon_resources
                            .get(&resource_key)
                            .is_some_and(|failure| {
                                failure.attempts >= DOWNLOAD_TASK_ICON_MAX_ATTEMPTS
                            })) {
                    download_task_icon_type_resource(&target)
                } else {
                    (resource_key, target)
                };
                Some((task.task.id(), resource_key, target))
            })
            .collect::<Vec<_>>();
        let live_resource_keys = self
            .tasks
            .iter()
            .filter_map(download_task_icon_resource_key)
            .chain(
                descriptors
                    .iter()
                    .map(|(_, resource_key, _)| resource_key.clone()),
            )
            .collect::<HashSet<_>>();
        self.task_icon_resources
            .retain(|resource_key, _| live_resource_keys.contains(resource_key));
        self.failed_task_icon_resources
            .retain(|resource_key, _| live_resource_keys.contains(resource_key));
        if !self.platform.is_connected() {
            return;
        }
        let mut in_flight = self.pending_task_icons.len();
        for (task_id, resource_key, target) in descriptors {
            let resource_scope = if resource_key.contains(":target:") {
                "target"
            } else {
                "file_type"
            };
            if self
                .task_icon_resource_assignments
                .insert(task_id, resource_key.clone())
                .is_some_and(|previous| previous != resource_key)
            {
                self.task_icons.remove(&task_id);
            }
            if let Some(path) = self.task_icon_resources.get(&resource_key) {
                self.task_icons.insert(task_id, path.clone());
                continue;
            }
            if let Some(command_id) = self.pending_task_icon_resources.get(&resource_key).copied() {
                if let Some(pending) = self.pending_task_icons.get_mut(&command_id) {
                    pending.task_ids.insert(task_id);
                }
                continue;
            }
            if self
                .failed_task_icon_resources
                .get(&resource_key)
                .is_some_and(|failure| {
                    failure.attempts >= DOWNLOAD_TASK_ICON_MAX_ATTEMPTS
                        || failure.retry_token.is_some()
                })
            {
                continue;
            }
            if in_flight >= DOWNLOAD_TASK_ICON_MAX_IN_FLIGHT {
                continue;
            }
            let command_id = uuid::Uuid::new_v4();
            self.pending_task_icon_resources
                .insert(resource_key.clone(), command_id);
            self.pending_task_icons.insert(
                command_id,
                PendingDownloadTaskIcon {
                    resource_key: resource_key.clone(),
                    task_ids: HashSet::from([task_id]),
                    resource_scope,
                    started_at: Instant::now(),
                },
            );
            match self.platform.send_command_with_id(
                PlatformCommand::ExtractShellIcon(ExtractShellIconCommand {
                    target,
                    size: DOWNLOAD_TASK_ICON_SIZE,
                }),
                command_id,
            ) {
                Ok(_) => {
                    in_flight += 1;
                    info!(
                        event = "download.list.icon_accepted",
                        correlation_id = %command_id,
                        resource_scope,
                        "download task Shell icon request accepted"
                    );
                }
                Err(_) => {
                    self.pending_task_icons.remove(&command_id);
                    self.pending_task_icon_resources.remove(&resource_key);
                    self.record_task_icon_failure(resource_key.clone(), false, cx);
                    let failure = &self.failed_task_icon_resources[&resource_key];
                    warn!(
                        event = "download.list.icon_request_failed",
                        correlation_id = %command_id,
                        resource_scope,
                        attempts = failure.attempts,
                        retry_scheduled = failure.retry_token.is_some(),
                        failure_kind = "transport",
                        "download task Shell icon request could not be queued"
                    );
                }
            }
        }
    }

    fn bounded_snapshot_error(error: &str) -> String {
        const MAX_ERROR_CHARS: usize = 180;
        let message = error.chars().take(MAX_ERROR_CHARS).collect::<String>();
        if error.chars().count() > MAX_ERROR_CHARS {
            format!("下载列表刷新失败：{message}…")
        } else {
            format!("下载列表刷新失败：{message}")
        }
    }

    fn refresh_snapshot(&mut self, refresh_task_detail: bool, cx: &mut Context<Self>) {
        let snapshot = match load_download_snapshot(&self.engine) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                let message = Self::bounded_snapshot_error(&error);
                warn!(
                    event = "download.desktop.snapshot_refresh_failed",
                    error_kind = "snapshot",
                    error_chars = error.chars().count(),
                    "desktop download snapshot refresh failed"
                );
                self.snapshot_error = Some(message);
                cx.notify();
                return;
            }
        };
        if self.event_generation != 0 && self.event_generation != snapshot.generation {
            info!(
                previous_generation = self.event_generation,
                generation = snapshot.generation,
                "download desktop accepted a new engine session snapshot"
            );
            self.last_event_sequence = 0;
        }
        self.event_generation = snapshot.generation;
        self.snapshot_error = None;
        let tasks = match download_snapshot_tasks(&snapshot) {
            Ok(tasks) => tasks,
            Err(error) => {
                warn!(%error, "could not project the compact download snapshot");
                self.snapshot_error = Some(Self::bounded_snapshot_error(&error));
                cx.notify();
                return;
            }
        };
        self.latest_progress.clear();
        self.last_event_sequence = snapshot.sequence;
        self.observe_destination_conflicts(&tasks);
        self.publish_terminal_notifications(&tasks, cx);
        self.tasks = Rc::new(tasks);
        self.queues = snapshot.queues;
        self.request_task_icons(cx);
        if refresh_task_detail {
            self.refresh_selected_task_detail(cx);
        }
        let task_states = self
            .tasks
            .iter()
            .map(|task| (task.task.id(), task.task.status()))
            .collect::<Vec<_>>();
        let mut unseen_progress = self
            .pending_controlled_progress
            .keys()
            .copied()
            .collect::<HashSet<_>>();
        for (task_id, status) in task_states {
            if unseen_progress.remove(&task_id) {
                self.reconcile_controlled_progress(task_id, status, cx);
            }
            let mut directive = self.task_window_lifecycle.task_changed(task_id, status);
            if !self.show_completion_window
                && matches!(
                    directive,
                    DownloadWindowDirective::Open(DownloadWindowPhase::Completed)
                )
            {
                self.task_window_lifecycle.window_closed(task_id);
                directive = DownloadWindowDirective::None;
            }
            self.apply_task_window_directive(task_id, directive, cx);
        }
        for task_id in unseen_progress {
            self.retire_controlled_progress(task_id, "task_removed", cx);
        }
        cx.notify();
    }
    fn refresh_open_task_window_for_progress(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) {
        if self.task_windows.contains_key(&task_id) {
            self.apply_task_window_directive(task_id, DownloadWindowDirective::RefreshExisting, cx);
        }
    }

    fn handle_engine_event(&mut self, envelope: DownloadEventEnvelope, cx: &mut Context<Self>) {
        if envelope.generation != 0
            && self.event_generation != 0
            && envelope.generation != self.event_generation
        {
            warn!(
                expected_generation = self.event_generation,
                observed_generation = envelope.generation,
                "stale download event generation detected; refreshing the full task snapshot"
            );
            self.refresh_snapshot(true, cx);
            return;
        }
        if self.event_generation == 0 && envelope.generation != 0 {
            self.event_generation = envelope.generation;
            self.last_event_sequence = 0;
        }
        let sequence = envelope.sequence;
        let detail_rejected =
            if let DownloadEvent::CommandRejected { request_id, .. } = &envelope.event {
                self.claim_detail_rejection(*request_id, cx)
            } else {
                false
            };
        if self.last_event_sequence != 0 && sequence <= self.last_event_sequence {
            warn!(
                event = "download.desktop.stale_event_dropped",
                expected_after_sequence = self.last_event_sequence,
                observed_sequence = sequence,
                "stale download event was rejected before applying state"
            );
            return;
        }
        let sequence_gap =
            self.last_event_sequence != 0 && sequence != self.last_event_sequence + 1;
        if sequence_gap {
            warn!(
                expected_sequence = self.last_event_sequence + 1,
                observed_sequence = sequence,
                "download event gap detected; refreshing the full task snapshot"
            );
        }
        self.last_event_sequence = sequence;
        // Read-only TaskDetail rejection is owned by its correlated async reply.
        // It cannot trigger a query -> rejection -> snapshot -> query loop.
        if detail_rejected {
            if sequence_gap {
                self.refresh_snapshot(false, cx);
            }
            return;
        }
        let progress_event = matches!(&envelope.event, DownloadEvent::TaskProgress { .. });
        if sequence_gap && progress_event {
            self.refresh_snapshot(true, cx);
            return;
        }
        let task_event_diagnostic = match &envelope.event {
            DownloadEvent::TaskChanged { task } => Some((task.id(), task.origin(), task.status())),
            _ => None,
        };
        let admitted_progress_task = match &envelope.event {
            DownloadEvent::TaskChanged { task } => {
                let known_to_desktop = self
                    .tasks
                    .iter()
                    .any(|candidate| candidate.task.id() == task.id());
                (self.show_progress_window
                    && should_open_admitted_progress_window(
                        known_to_desktop,
                        self.awaiting_delayed_progress.contains(&task.id()),
                        task.origin(),
                        task.status(),
                    ))
                .then_some(task.id())
            }
            _ => None,
        };
        let progress_task_id = match &envelope.event {
            DownloadEvent::TaskProgress { progress } => Some(progress.task_id),
            _ => None,
        };
        match envelope.event {
            DownloadEvent::TaskProgress { progress } => {
                if let Some(detail) = self.task_detail.as_mut()
                    && detail.task.task.id() == progress.task_id
                {
                    apply_progress_to_task_detail(detail, &progress);
                }
                self.latest_progress.insert(progress.task_id, progress);
            }
            DownloadEvent::TaskRemoved { task_id } => {
                self.latest_progress.remove(&task_id);
                self.awaiting_delayed_progress.remove(&task_id);
                self.retire_controlled_progress(task_id, "task_removed", cx);
                let directive = self.task_window_lifecycle.task_removed(task_id);
                self.apply_task_window_directive(task_id, directive, cx);
            }
            DownloadEvent::TaskChanged { task } => {
                if matches!(
                    task.status(),
                    DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
                ) {
                    self.awaiting_delayed_progress.remove(&task.id());
                }
            }
            DownloadEvent::CommandRejected { .. } | DownloadEvent::SnapshotRequired { .. } => {}
        }
        if !sequence_gap && let Some(task_id) = progress_task_id {
            self.refresh_open_task_window_for_progress(task_id, cx);
            cx.notify();
            return;
        }
        self.refresh_snapshot(true, cx);
        if let Some((task_id, origin, status)) = task_event_diagnostic {
            info!(
                event = "download.desktop.task_event_applied",
                sequence,
                task_id = %task_id.as_uuid(),
                ?origin,
                ?status,
                task_count = self.tasks.len(),
                "download task state applied to the desktop snapshot"
            );
        }
        if let Some(task_id) = admitted_progress_task {
            let still_active = self
                .task(task_id)
                .is_some_and(|task| is_active_download_progress(task.task.status()));
            if still_active {
                let use_background_policy = task_event_diagnostic
                    .as_ref()
                    .is_some_and(|(_, origin, _)| *origin == RequestOrigin::Cli)
                    || self.awaiting_delayed_progress.contains(&task_id);
                self.awaiting_delayed_progress.remove(&task_id);
                let directive = if use_background_policy {
                    self.task_window_lifecycle
                        .request_background_progress(task_id)
                } else {
                    self.task_window_lifecycle.request_progress(task_id)
                };
                self.apply_task_window_directive(task_id, directive, cx);
            }
        }
    }
}

mod window_support;
pub(crate) use window_support::*;

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "downloads/new_download_link_tests.rs"]
mod new_download_link_tests;

#[cfg(test)]
mod installer_download_tests;
