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
use gpui::{
    AnyElement, AnyWindowHandle, App, Context, Corner, Entity, EventEmitter, Focusable, FontWeight,
    IntoElement, MouseButton, Render, Rgba, ScrollHandle, Subscription, Task, Window,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowDecorations, WindowKind,
    WindowOptions, div, prelude::*, px, size,
};
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::popover::Popover;
use gpui_component::scroll::ScrollableElement;
use gpui_component::select::{Select, SelectState};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Disableable, Icon, IndexPath, PixelsExt, Root, Sizable};
use nxtools_core::DownloadRowDoubleClickActionSettings;
use nxtools_core::{
    AppConfig, DownloadCategorySettings, DownloadDuplicateStrategySettings, DownloadSettings,
    DownloadTaskbarOverlayAnchor, DownloadTaskbarProgressMode,
    DownloadTaskbarProgressOverlaySettings, NetworkProxyMode,
};
use nxtools_download::{
    BrowserBlob, BrowserDownloadDialogRequest, BrowserDownloadDialogSource,
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
    OpenShellTargetCommand, PasteTextCommand, PickDirectoryCommand, PickProgramCommand,
    PlaceTaskbarProgressOverlayCommand, PlatformCommand, PlatformCommandResult, PlatformEvent,
    SetTaskbarProgressCommand, SetWindowBoundsCenteredCommand, SetWindowTopmostCommand,
    ShellOpenAction, ShutdownSystemCommand, StashWindowBoundsCommand, TaskbarOverlayAnchor,
    TaskbarProgressState,
};
use tracing::{error, info, warn};

use crate::completion_actions::{
    COMPLETION_FEEDBACK_DELAY, COMPLETION_PLATFORM_TIMEOUT, CompletionActionCoordinator,
    CompletionFeedbackBatch, CompletionTerminal, ShutdownCountdown, ShutdownTransition,
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
    icon_action_tooltip_surface, primary_button_variant, segmented_control,
};
use crate::ui::components::dialog::{DialogShellIds, dialog_shell};
use crate::ui::components::field::{
    field_shell, inline_error, labeled_control, labeled_field, multiline_field_shell,
    path_picker_shell,
};
use crate::ui::components::list::{compact_list_row, radio_list_row};
use crate::ui::components::popover::{list_box, popover_shell};
use crate::ui::components::settings::{
    SettingsShellIds, dialog_footer, settings_shell, settings_toggle,
};
use crate::ui::components::titlebar::client_titlebar;
use crate::ui::metrics::{
    BADGE_HEIGHT, CONTROL_HEIGHT_ACTION, CONTROL_HEIGHT_INPUT, PILL_PADDING_X,
    PROGRESS_TRACK_HEIGHT, RADIUS_CONTROL, RADIUS_FLOATING, SETTINGS_ROW_MIN_HEIGHT, SPACING_2,
    SPACING_4, SPACING_6, SPACING_8, SPACING_12, SPACING_16,
};
use crate::ui::native_window_handle;
use crate::ui::typography::{
    LINE_HEIGHT_BODY, LINE_HEIGHT_LABEL, LINE_HEIGHT_META, LINE_HEIGHT_TITLE, TEXT_BODY,
    TEXT_LABEL, TEXT_META, TEXT_STATUS_GLYPH, TEXT_TITLE, UI_MONO_FONT_FAMILY,
};

mod delete_confirmation;
mod browser_handoff_impl;
mod completion_shutdown_impl;
mod task_windows_impl;
mod new_download_view;
mod schedule_view;
mod settings_view;
mod shutdown_window;
mod task_window;

use self::delete_confirmation::*;
use self::new_download_view::*;
use self::schedule_view::*;
use self::settings_view::*;
use self::shutdown_window::*;
use self::task_window::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DownloadApplicationEvent {
    DirectoryPicked {
        target: DownloadDirectoryTarget,
        path: PathBuf,
    },
    FfmpegExecutablePicked {
        path: PathBuf,
    },
    FfmpegInstallChanged {
        state: FfmpegInstallViewState,
    },
    BrowserExtensionExportCompleted {
        status: String,
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
    default_directory: PathBuf,
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
    adaptive_segment_scaling_enabled: bool,
    adaptive_segment_max_segments: u16,
    adaptive_segment_min_mib: u16,
    max_concurrent_tasks: u16,
    global_speed_limit_bytes_per_second: Option<u64>,
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
}

const FFMPEG_PICKER_TIMEOUT: Duration = Duration::from_secs(10 * 60);
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

#[derive(Debug, Clone, Copy)]
struct PendingExtensionExport {
    format: &'static str,
    started_at: Instant,
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

#[derive(Debug, Clone, Copy)]
struct PendingFfmpegPlatformOperation {
    correlation_id: uuid::Uuid,
    started_at: Instant,
    kind: FfmpegPlatformOperationKind,
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
        && matches!(
            status,
            DownloadStatus::Probing
                | DownloadStatus::Waiting
                | DownloadStatus::Downloading
                | DownloadStatus::Finalizing
                | DownloadStatus::Verifying
        )
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

const DOWNLOAD_TASK_ICON_SIZE: u32 = 24;

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
    let path = Path::new(&target);
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())?
        .trim()
        .to_ascii_lowercase();
    (!extension.is_empty()).then(|| format!("{DOWNLOAD_TASK_ICON_SIZE}:type:.{extension}"))
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
    tasks: Vec<DownloadEngineTask>,
    task_detail: Option<DownloadTaskDetail>,
    task_detail_error: Option<(DownloadTaskId, String)>,
    queues: Vec<DownloadQueue>,
    latest_progress: HashMap<DownloadTaskId, DownloadProgress>,
    task_icons: HashMap<DownloadTaskId, PathBuf>,
    task_icon_resource_assignments: HashMap<DownloadTaskId, String>,
    task_icon_resources: HashMap<String, PathBuf>,
    pending_task_icons: HashMap<uuid::Uuid, PendingDownloadTaskIcon>,
    pending_task_icon_resources: HashMap<String, uuid::Uuid>,
    failed_task_icon_resources: HashSet<String>,
    awaiting_delayed_progress: HashSet<DownloadTaskId>,
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
    new_download_window: Option<AnyWindowHandle>,
    #[cfg(test)]
    new_download_view: Option<Entity<NewDownloadView>>,
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
    config_path: PathBuf,
    log_directory: PathBuf,
    database_path: PathBuf,
    ffmpeg_install_directory: PathBuf,
    show_progress_window: bool,
    show_completion_window: bool,
    pending_directory_picker: Option<(uuid::Uuid, DownloadDirectoryTarget)>,
    pending_settings_path_actions: HashMap<uuid::Uuid, PendingDownloadSettingsPathAction>,
    pending_extension_export: Option<PendingExtensionExport>,
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
        let (tasks, queues) = match load_download_snapshot(&engine) {
            Ok(snapshot) => (snapshot.task_details, snapshot.queues),
            Err(error) => {
                warn!(%error, "could not load the download snapshot for the desktop");
                (Vec::new(), Vec::new())
            }
        };
        let mut task_window_lifecycle = DownloadWindowLifecycle::default();
        let awaiting_delayed_progress = tasks
            .iter()
            .filter(|task| {
                task.task.origin() == RequestOrigin::Ui
                    && task.task.request().scheduled_at_unix_ms.is_some()
            })
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
                after_sequence: None,
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
                if this
                    .update(cx, |application, cx| {
                        application.handle_engine_event(event, cx);
                    })
                    .is_err()
                {
                    return;
                }
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
                if this
                    .update(cx, |application, cx| {
                        application.open_browser_download_dialog(request, cx);
                    })
                    .is_err()
                {
                    return;
                }
            }
        });
        Self {
            engine,
            platform,
            notifications,
            default_directory,
            tasks,
            task_detail: None,
            task_detail_error: None,
            queues,
            latest_progress: HashMap::new(),
            task_icons: HashMap::new(),
            task_icon_resource_assignments: HashMap::new(),
            task_icon_resources: HashMap::new(),
            pending_task_icons: HashMap::new(),
            pending_task_icon_resources: HashMap::new(),
            failed_task_icon_resources: HashSet::new(),
            awaiting_delayed_progress,
            task_window_lifecycle,
            task_windows: HashMap::new(),
            pending_task_window_activations: HashMap::new(),
            pending_task_window_replacements: HashMap::new(),
            #[cfg(test)]
            task_window_native_handle_override: None,
            #[cfg(test)]
            task_window_activation_commands: Rc::new(RefCell::new(Vec::new())),
            delete_confirmation_windows: HashMap::new(),
            new_download_window: None,
            #[cfg(test)]
            new_download_view: None,
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
            config_path,
            log_directory,
            database_path,
            ffmpeg_install_directory,
            show_progress_window,
            show_completion_window,
            pending_directory_picker: None,
            pending_settings_path_actions: HashMap::new(),
            pending_extension_export: None,
            pending_ffmpeg_operations: HashMap::new(),
            ffmpeg_install_state: FfmpegInstallViewState::Idle,
            pending_ffmpeg_install: None,
            #[cfg(test)]
            ffmpeg_install_operation_override: None,
            #[cfg(test)]
            ffmpeg_platform_commands: Rc::new(RefCell::new(Vec::new())),
            notified_terminal_states,
            completion_actions: CompletionActionCoordinator::default(),
            completion_feedback_scheduled: false,
            pending_completion_feedback: HashMap::new(),
            completion_feedback_commands: HashMap::new(),
            shutdown_window: None,
            pending_shutdown_command: None,
            last_event_sequence: 0,
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
        self.tasks = tasks;
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

    pub(crate) fn task_icon_path(&self, task_id: DownloadTaskId) -> Option<&Path> {
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

    pub(crate) fn request_task_detail(&mut self, task_id: DownloadTaskId, cx: &mut Context<Self>) {
        if self.task_detail(task_id).is_some() {
            return;
        }
        match load_download_task_detail(&self.engine, task_id) {
            Ok(detail) => {
                self.task_detail = Some(detail);
                self.task_detail_error = None;
            }
            Err(error) => {
                warn!(
                    task_id = %task_id.as_uuid(),
                    "could not load selected download task detail"
                );
                self.task_detail = None;
                self.task_detail_error = Some((task_id, error));
            }
        }
        cx.notify();
    }

    fn default_queue(&self) -> Option<&DownloadQueue> {
        self.queues.iter().find(|queue| queue.id == "default")
    }

    fn settings_draft(&self) -> DownloadSettingsDraft {
        DownloadSettingsDraft {
            default_directory: self.default_directory.clone(),
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
            adaptive_segment_scaling_enabled: self.config.download.adaptive_segment_scaling_enabled,
            adaptive_segment_max_segments: self.config.download.adaptive_segment_max_segments,
            adaptive_segment_min_mib: self.config.download.adaptive_segment_min_mib,
            max_concurrent_tasks: self
                .default_queue()
                .and_then(|queue| queue.max_concurrent_tasks)
                .unwrap_or(3),
            global_speed_limit_bytes_per_second: self.engine.global_speed_limit(),
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
        self.latest_progress.get(&task_id)
    }

    pub(crate) fn submit(
        &mut self,
        request: DownloadRequest,
        mode: DownloadSubmitMode,
        cx: &mut Context<Self>,
    ) -> Result<DownloadTaskId, String> {
        let delayed_progress =
            mode == DownloadSubmitMode::StartNow && request.scheduled_at_unix_ms.is_some();
        let outcome = self.execute(DownloadCommand::Submit { request, mode });
        let task_id = match outcome {
            DownloadCommandOutcome::Accepted {
                task_id: Some(task_id),
            } => task_id,
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
        Ok(())
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

    pub(crate) fn batch_action(
        &mut self,
        task_ids: Vec<DownloadTaskId>,
        action: DownloadBatchAction,
        cx: &mut Context<Self>,
    ) {
        let outcome = self.execute(DownloadCommand::Batch { task_ids, action });
        match outcome {
            DownloadCommandOutcome::Batch { results } => {
                let failures = results
                    .iter()
                    .filter_map(|result| result.fault.as_ref())
                    .collect::<Vec<_>>();
                if let Some(first) = failures.first() {
                    self.notifications.update(cx, |notifications, cx| {
                        notifications.show_error(
                            "download-batch-action",
                            format!("有 {} 个任务操作失败", failures.len()),
                            first.message.clone(),
                            cx,
                        );
                    });
                }
                self.refresh(cx);
            }
            DownloadCommandOutcome::Rejected { fault } => {
                let _ = self.action_error("批量操作失败", fault.message, cx);
            }
            _ => {
                let _ = self.action_error(
                    "批量操作失败",
                    "下载引擎返回了无效的批量操作结果".into(),
                    cx,
                );
            }
        }
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
            DownloadCommandOutcome::Rejected { fault } => {
                self.action_error("无法保存每日计划", fault.message, cx)
            }
            _ => self.action_error(
                "无法保存每日计划",
                "下载引擎返回了无效的队列操作结果".into(),
                cx,
            ),
        }
    }

    pub(crate) fn open_task_artifact(
        &mut self,
        task_id: DownloadTaskId,
        action: ShellOpenAction,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self
            .task(task_id)
            .and_then(|task| task.final_path.as_ref())
            .map(|path| path.to_string_lossy().into_owned())
        else {
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    format!("download-artifact-missing-{}", task_id.as_uuid()),
                    "无法打开下载文件",
                    "任务还没有可用的最终文件路径。",
                    cx,
                );
            });
            return;
        };
        match self
            .platform
            .send_command(PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: path,
                action,
            })) {
            Ok(correlation_id) => {
                info!(
                    event = "download.artifact.shell_action_accepted",
                    %correlation_id,
                    task_id = %task_id.as_uuid(),
                    ?action,
                    "download artifact Shell action accepted"
                );
            }
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("download-artifact-open-{}", task_id.as_uuid()),
                        "无法执行文件操作",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
    }

    fn begin_task_file_drag(
        &mut self,
        task_id: DownloadTaskId,
        source_window_handle: i64,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self
            .task(task_id)
            .and_then(|task| task.final_path.as_ref())
            .map(|path| path.to_string_lossy().into_owned())
        else {
            return;
        };
        let file_name = PathBuf::from(&path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned());
        match self
            .platform
            .send_command(PlatformCommand::BeginFileDrag(BeginFileDragCommand {
                path,
                source_window_handle,
            })) {
            Ok(correlation_id) => {
                info!(
                    task_id = %task_id.as_uuid(),
                    %correlation_id,
                    source_window_handle,
                    ?file_name,
                    "download file drag requested"
                );
            }
            Err(error) => {
                warn!(
                    task_id = %task_id.as_uuid(),
                    source_window_handle,
                    ?file_name,
                    %error,
                    "download file drag request rejected"
                );
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("download-file-drag-{}", task_id.as_uuid()),
                        "无法开始拖拽文件",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
    }

    #[allow(
        dead_code,
        reason = "retained for the Phase 3 task context-menu delete action"
    )]
    pub(crate) fn request_delete_confirmation(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) {
        // The user-configurable contract for "删除文件": when the file delete
        // is disabled, the menu item degenerates to a list-only removal.
        if !self.config.download.task_removal_deletes_file {
            let _ = self.remove(task_id, false, cx);
            return;
        }
        if let Some(window) = self.delete_confirmation_windows.get(&task_id).cloned() {
            if window
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
            {
                return;
            }
            self.delete_confirmation_windows.remove(&task_id);
        }
        let application = cx.entity();
        let path = self
            .task(task_id)
            .and_then(|task| task.final_path.as_ref())
            .map_or_else(|| "最终文件".into(), |path| path.display().to_string());
        let window_size = size(px(380.0), px(178.0));
        let window_bounds = WindowBounds::centered(window_size, cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                titlebar: None,
                focus: true,
                show: true,
                kind: WindowKind::Normal,
                is_movable: true,
                is_resizable: false,
                is_minimizable: false,
                display_id: None,
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some("NxTools.Download.DeleteConfirmation".into()),
                window_min_size: Some(window_size),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let close_application = application.clone();
                let closing_window = window.window_handle();
                window.on_window_should_close(cx, move |_, cx| {
                    let application = close_application.clone();
                    cx.defer(move |cx| {
                        application.update(cx, |application, _| {
                            application
                                .handle_native_delete_confirmation_close(task_id, closing_window);
                        });
                    });
                    true
                });
                let view = cx.new(|_| DeleteDownloadView {
                    theme: Theme::zed_dark(),
                    application,
                    task_id,
                    path,
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                self.delete_confirmation_windows
                    .insert(task_id, window.into());
            }
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("download-delete-confirm-{}", task_id.as_uuid()),
                        "无法打开删除确认窗口",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
    }

    fn close_delete_confirmation(&mut self, task_id: DownloadTaskId, cx: &mut Context<Self>) {
        if let Some(window) = self.delete_confirmation_windows.remove(&task_id) {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        }
    }

    fn handle_native_delete_confirmation_close(
        &mut self,
        task_id: DownloadTaskId,
        closing_window: AnyWindowHandle,
    ) {
        if self.delete_confirmation_windows.get(&task_id) == Some(&closing_window) {
            self.delete_confirmation_windows.remove(&task_id);
        }
    }

    fn confirm_delete_artifact(&mut self, task_id: DownloadTaskId, cx: &mut Context<Self>) {
        if self.remove(task_id, true, cx).is_ok() {
            self.close_delete_confirmation(task_id, cx);
        }
    }

    fn action_error(
        &mut self,
        title: &'static str,
        message: String,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_error("download-task-action", title, message.clone(), cx);
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
    ) -> Result<(), String> {
        match self.execute(DownloadCommand::Task { task_id, action }) {
            DownloadCommandOutcome::Accepted { .. } => Ok(()),
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

    fn request_directory(
        &mut self,
        current_directory: String,
        target: DownloadDirectoryTarget,
        cx: &mut Context<Self>,
    ) {
        if self.pending_directory_picker.is_some() {
            return;
        }
        match self
            .platform
            .send_command(PlatformCommand::PickDirectory(PickDirectoryCommand {
                current_directory,
                title: "选择下载保存目录".into(),
            })) {
            Ok(command_id) => {
                self.pending_directory_picker = Some((command_id, target));
                info!(
                    event = "download.directory_picker.accepted",
                    %command_id,
                    ?target,
                    "download directory picker accepted"
                );
            }
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "download-directory-picker",
                        "无法打开目录选择器",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
        cx.notify();
    }

    pub(crate) fn request_new_download_directory(
        &mut self,
        current_directory: String,
        cx: &mut Context<Self>,
    ) {
        self.request_directory(current_directory, DownloadDirectoryTarget::NewTask, cx);
    }

    fn request_settings_directory(&mut self, current_directory: String, cx: &mut Context<Self>) {
        self.request_directory(current_directory, DownloadDirectoryTarget::Settings, cx);
    }

    fn request_ffmpeg_executable(&mut self, current_path: String, cx: &mut Context<Self>) {
        if self
            .pending_ffmpeg_operations
            .values()
            .any(|pending| pending.kind == FfmpegPlatformOperationKind::Picker)
        {
            return;
        }
        self.begin_ffmpeg_platform_operation(
            PlatformCommand::PickProgram(PickProgramCommand { current_path }),
            FfmpegPlatformOperationKind::Picker,
            FFMPEG_PICKER_TIMEOUT,
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
                        "ffmpeg-install-start",
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
                if this
                    .update(cx, |application, cx| {
                        application.handle_ffmpeg_install_progress(correlation_id, update, cx);
                    })
                    .is_err()
                {
                    return;
                }
            }
            let Ok(result) = completion.recv().await else {
                return;
            };
            let Some(this) = this.upgrade() else {
                return;
            };
            let _ = this.update(cx, |application, cx| {
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
                            "ffmpeg-install-failed",
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
                });
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_success(
                        "ffmpeg-install-completed",
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
                        "ffmpeg-install-failed",
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
                    format!("download-settings-path-{}-copy", presentation.key),
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
                format!(
                    "download-settings-path-{}-{}-failed",
                    presentation.key,
                    pending.action.name()
                ),
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
            FFMPEG_DOWNLOAD_PAGE_TIMEOUT,
            cx,
        );
    }

    fn begin_ffmpeg_platform_operation(
        &mut self,
        command: PlatformCommand,
        kind: FfmpegPlatformOperationKind,
        timeout: Duration,
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
                    },
                );
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(timeout).await;
                    let _ = this.update(cx, |application, cx| {
                        application.ffmpeg_platform_operation_timed_out(command_id, cx);
                    });
                })
                .detach();
            }
            Err(error) => {
                self.finish_ffmpeg_platform_operation_failure(
                    PendingFfmpegPlatformOperation {
                        correlation_id,
                        started_at,
                        kind,
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
                        "download-ffmpeg-picker",
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
                        format!("download-ffmpeg-page-{}", source.name()),
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
            self.settings_window = None;
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
            window.on_window_should_close(cx, move |_, cx| {
                close_application.update(cx, |application, cx| {
                    application.handle_settings_window_closed(cx);
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
                        "download-settings-window",
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

    fn handle_settings_window_closed(&mut self, cx: &mut Context<Self>) {
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

        let previous_concurrency = self
            .default_queue()
            .and_then(|queue| queue.max_concurrent_tasks)
            .unwrap_or(3);
        let previous_speed_limit = self.engine.global_speed_limit();
        let next_download_settings = DownloadSettings {
            default_directory: draft.default_directory.to_string_lossy().into_owned(),
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
            adaptive_segment_scaling_enabled: draft.adaptive_segment_scaling_enabled,
            adaptive_segment_max_segments: draft.adaptive_segment_max_segments,
            adaptive_segment_min_mib: draft.adaptive_segment_min_mib,
            show_progress_window: draft.show_progress_window,
            show_completion_window: draft.show_completion_window,
            completion_sound_enabled: draft.completion_sound_enabled,
            open_directory_on_completion: draft.open_directory_on_completion,
            task_removal_deletes_file: draft.task_removal_deletes_file,
            taskbar_progress_overlay: draft.taskbar_progress_overlay.clone(),
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
        if let Err(error) =
            self.set_default_queue_concurrency(draft.max_concurrent_tasks, correlation_id)
        {
            return failed("queue", error);
        }
        if let Err(error) =
            self.set_global_speed_limit(draft.global_speed_limit_bytes_per_second, correlation_id)
        {
            return match self.set_default_queue_concurrency(previous_concurrency, correlation_id) {
                Ok(()) => failed("speed_limit", error),
                Err(rollback_error) => failed(
                    "rollback",
                    format!("{error}；同时无法回滚队列并发：{rollback_error}"),
                ),
            };
        }

        let next_config =
            match AppConfig::update_download_settings(&self.config_path, next_download_settings) {
                Ok(config) => config,
                Err(error) => {
                    let mut rollback_errors = Vec::new();
                    if let Err(rollback_error) =
                        self.set_default_queue_concurrency(previous_concurrency, correlation_id)
                    {
                        rollback_errors.push(format!("队列并发：{rollback_error}"));
                    }
                    if let Err(rollback_error) =
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
        let configured_ffmpeg = (!draft.ffmpeg_executable_path.is_empty())
            .then(|| PathBuf::from(&draft.ffmpeg_executable_path));
        if let Err(error) = self.engine.set_ffmpeg_executable_path(configured_ffmpeg) {
            return failed(
                "runtime_sync",
                format!("FFmpeg 路径已保存，但无法同步到下载引擎：{error}"),
            );
        }
        self.default_directory = draft.default_directory;
        self.show_progress_window = draft.show_progress_window;
        self.show_completion_window = draft.show_completion_window;
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
                "download-settings-saved",
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













    pub(crate) fn handle_platform_notification(
        &mut self,
        notification: PlatformNotification,
        cx: &mut Context<Self>,
    ) {
        if matches!(&notification, PlatformNotification::Connected) {
            self.failed_task_icon_resources.clear();
            self.request_task_icons();
            let correlation_id = uuid::Uuid::new_v4();
            match self.platform.send_command_correlated(
                PlatformCommand::EnsureBrowserNativeMessaging(
                    EnsureBrowserNativeMessagingCommand {
                        enabled: true,
                        chromium_extension_id: "haohhnpjfggpdadhcbjakjpkmkljkpif".into(),
                        firefox_extension_id: "nxtools-download-interceptor@nxtools.local".into(),
                    },
                ),
                correlation_id,
            ) {
                Ok(command_id) => info!(
                    event = "download.browser_native_messaging.registration_requested",
                    %correlation_id,
                    %command_id,
                    "browser native messaging registration queued after PlatformHost connected"
                ),
                Err(error) => warn!(
                    event = "download.browser_native_messaging.registration_request_failed",
                    %correlation_id,
                    %error,
                    "browser native messaging registration could not be queued"
                ),
            }
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
        if let Some(pending) = self.pending_task_icons.remove(&correlation_id) {
            let native_error_code = result.native_error_code;
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
                warn!(
                    event = "download.list.icon_failed",
                    %correlation_id,
                    success = false,
                    resource_scope = pending.resource_scope,
                    elapsed_ms = pending.started_at.elapsed().as_millis(),
                    ?native_error_code,
                    error_present,
                    "download task Shell icon request failed"
                );
                self.failed_task_icon_resources.insert(pending.resource_key);
            }
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
        if let Some(pending) = self.pending_extension_export.take() {
            let elapsed = pending.started_at.elapsed().as_millis();
            if result.success {
                let cancelled = result
                    .data
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or_default()
                    .is_empty();
                if cancelled {
                    info!(
                        event = "download.browser_extension.export_cancelled",
                        format = pending.format,
                        elapsed_ms = elapsed,
                        "browser extension export cancelled in the save dialog"
                    );
                    self.publish_extension_export_status("已取消导出浏览器扩展".into(), cx);
                    return;
                }
                let file_name = result
                    .data
                    .as_deref()
                    .map(|path| {
                        std::path::Path::new(path)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.to_owned())
                    })
                    .unwrap_or_default();
                let status = format!("已导出:{file_name}");
                self.publish_extension_export_status(status, cx);
                info!(
                    event = "download.browser_extension.exported",
                    format = pending.format,
                    elapsed_ms = elapsed,
                    "browser extension export completed"
                );
            } else {
                let reason = result
                    .error
                    .clone()
                    .unwrap_or_else(|| "Windows 未返回错误详情".into());
                let status = format!("导出失败:{reason}");
                self.publish_extension_export_status(status, cx);
                warn!(
                    event = "download.browser_extension.export_failed",
                    format = pending.format,
                    reason,
                    elapsed_ms = elapsed,
                    "browser extension export failed"
                );
            }
            cx.notify();
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
        let Some((pending_id, target)) = self.pending_directory_picker else {
            return;
        };
        if pending_id != correlation_id {
            return;
        }
        self.pending_directory_picker = None;
        self.handle_directory_result(correlation_id, target, result, cx);
    }

    fn handle_directory_result(
        &mut self,
        correlation_id: uuid::Uuid,
        target: DownloadDirectoryTarget,
        result: PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        if result.success {
            if let Some(path) = result.data.filter(|path| !path.trim().is_empty()) {
                info!(
                    event = "download.directory_picker.succeeded",
                    %correlation_id,
                    ?target,
                    "download directory picker completed"
                );
                cx.emit(DownloadApplicationEvent::DirectoryPicked {
                    target,
                    path: PathBuf::from(path),
                });
            } else {
                info!(
                    event = "download.directory_picker.cancelled",
                    %correlation_id,
                    ?target,
                    "download directory picker completed without a selection"
                );
            }
        } else {
            let native_error_present = result.error.is_some();
            let reason = result
                .error
                .unwrap_or_else(|| "Windows 未返回错误详情".into());
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "download-directory-picker",
                    "无法选择下载目录",
                    reason,
                    cx,
                );
            });
            warn!(
                event = "download.directory_picker.failed",
                %correlation_id,
                ?target,
                native_error_present,
                "download directory picker failed"
            );
        }
        cx.notify();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.refresh_snapshot(true, cx);
    }

    fn request_task_icons(&mut self) {
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
                Some((
                    task.task.id(),
                    download_task_icon_resource_key(task)?,
                    download_task_icon_target(task)?,
                ))
            })
            .collect::<Vec<_>>();
        let live_resource_keys = descriptors
            .iter()
            .map(|(_, resource_key, _)| resource_key.clone())
            .collect::<HashSet<_>>();
        self.task_icon_resources
            .retain(|resource_key, _| live_resource_keys.contains(resource_key));
        self.failed_task_icon_resources
            .retain(|resource_key| live_resource_keys.contains(resource_key));
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
            if self.failed_task_icon_resources.contains(&resource_key) {
                continue;
            }
            match self
                .platform
                .send_command(PlatformCommand::ExtractShellIcon(ExtractShellIconCommand {
                    target,
                    size: DOWNLOAD_TASK_ICON_SIZE,
                })) {
                Ok(command_id) => {
                    self.pending_task_icon_resources
                        .insert(resource_key.clone(), command_id);
                    self.pending_task_icons.insert(
                        command_id,
                        PendingDownloadTaskIcon {
                            resource_key,
                            task_ids: HashSet::from([task_id]),
                            resource_scope,
                            started_at: Instant::now(),
                        },
                    );
                    info!(
                        event = "download.list.icon_accepted",
                        correlation_id = %command_id,
                        resource_scope,
                        "download task Shell icon request accepted"
                    );
                }
                Err(error) => {
                    self.failed_task_icon_resources.insert(resource_key);
                    warn!(
                        event = "download.list.icon_request_failed",
                        %error,
                        "download task Shell icon request could not be queued"
                    );
                }
            }
        }
    }

    fn refresh_snapshot(&mut self, refresh_task_detail: bool, cx: &mut Context<Self>) {
        let snapshot = match load_download_snapshot(&self.engine) {
            Ok(snapshot) => snapshot,
            Err(error) => {
                warn!(
                    event = "download.desktop.snapshot_refresh_failed",
                    %error,
                    "desktop download snapshot refresh failed"
                );
                return;
            }
        };
        let tasks = snapshot.task_details;
        self.publish_terminal_notifications(&tasks, cx);
        self.tasks = tasks;
        self.queues = snapshot.queues;
        self.request_task_icons();
        if refresh_task_detail
            && let Some(task_id) = self
                .task_detail
                .as_ref()
                .map(|detail| detail.task.task.id())
        {
            if self.tasks.iter().any(|task| task.task.id() == task_id) {
                match load_download_task_detail(&self.engine, task_id) {
                    Ok(detail) => {
                        self.task_detail = Some(detail);
                        self.task_detail_error = None;
                    }
                    Err(error) => {
                        warn!(
                            task_id = %task_id.as_uuid(),
                            "could not refresh selected download task detail"
                        );
                        self.task_detail_error = Some((task_id, error));
                    }
                }
            } else {
                self.task_detail = None;
                self.task_detail_error = None;
            }
        }
        let task_states = self
            .tasks
            .iter()
            .map(|task| (task.task.id(), task.task.status()))
            .collect::<Vec<_>>();
        for (task_id, status) in task_states {
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
        cx.notify();
    }

    fn handle_engine_event(&mut self, envelope: DownloadEventEnvelope, cx: &mut Context<Self>) {
        let sequence = envelope.sequence;
        if self.last_event_sequence != 0 && sequence != self.last_event_sequence + 1 {
            warn!(
                expected_sequence = self.last_event_sequence + 1,
                observed_sequence = sequence,
                "download event gap detected; refreshing the full task snapshot"
            );
        }
        self.last_event_sequence = sequence;
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
        let is_progress_event = matches!(&envelope.event, DownloadEvent::TaskProgress { .. });
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
        self.refresh_snapshot(!is_progress_event, cx);
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
            let still_active = self.task(task_id).is_some_and(|task| {
                matches!(
                    task.task.status(),
                    DownloadStatus::Probing
                        | DownloadStatus::Waiting
                        | DownloadStatus::Downloading
                        | DownloadStatus::Finalizing
                        | DownloadStatus::Verifying
                )
            });
            if still_active {
                self.awaiting_delayed_progress.remove(&task_id);
                let directive = self.task_window_lifecycle.request_progress(task_id);
                self.apply_task_window_directive(task_id, directive, cx);
            }
        }
    }
}

mod window_support;
pub(crate) use window_support::*;

#[cfg(test)]
mod tests;