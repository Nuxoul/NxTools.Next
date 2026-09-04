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
    ExportBrowserExtensionCommand, ExtractShellIconCommand, OpenShellTargetCommand,
    PasteTextCommand, PickDirectoryCommand, PickProgramCommand, PlaceTaskbarProgressOverlayCommand,
    PlatformCommand, PlatformCommandResult, PlatformEvent, SetTaskbarProgressCommand,
    SetWindowBoundsCommand, SetWindowTopmostCommand, ShellOpenAction, ShutdownSystemCommand,
    TaskbarOverlayAnchor, TaskbarProgressState,
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

struct DownloadTaskWindow {
    window: AnyWindowHandle,
    view: Entity<DownloadTaskWindowView>,
    native_window_handle: i64,
    generation: uuid::Uuid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TaskWindowIdentity {
    native_window_handle: i64,
    generation: uuid::Uuid,
}

#[derive(Clone, Copy)]
enum TaskWindowActivation {
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

struct PendingTaskWindowActivation {
    task_id: DownloadTaskId,
    window: TaskWindowIdentity,
    activation: TaskWindowActivation,
    started_at: Instant,
}

struct PendingCompletionFeedback {
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

    fn import_browser_blob(
        &mut self,
        context: BrowserBlobDialogContext,
        save_directory: PathBuf,
        file_name: String,
        category_id: Option<String>,
        save_directory_explicit: bool,
        cx: &mut Context<Self>,
    ) -> Result<DownloadTaskId, String> {
        let outcome = self.execute_correlated(
            DownloadCommand::BrowserImportBlob {
                handoff_id: context.handoff_id,
                blob: context.blob,
                save_directory,
                file_name,
                category_id,
                save_directory_explicit,
            },
            context.correlation_id,
        );
        let task_id = match outcome {
            DownloadCommandOutcome::BrowserAccepted { task_id, .. } => task_id,
            DownloadCommandOutcome::Rejected { fault } => return Err(fault.message),
            _ => return Err("下载引擎返回了无效的 Blob 导入结果".into()),
        };
        self.refresh(cx);
        Ok(task_id)
    }

    fn discard_browser_blob(&self, context: &BrowserBlobDialogContext) {
        let started_at = Instant::now();
        let outcome = self.execute_correlated(
            DownloadCommand::BrowserDiscardBlob {
                handoff_id: context.handoff_id,
                blob_id: context.blob.id,
            },
            context.correlation_id,
        );
        match outcome {
            DownloadCommandOutcome::Accepted { .. } => {}
            DownloadCommandOutcome::Rejected { fault } => warn!(
                event = "download.browser_blob.cleanup_request_failed",
                correlation_id = %context.correlation_id,
                handoff_id = %context.handoff_id,
                blob_id = %context.blob.id,
                fault_code = ?fault.code,
                retryable = fault.retryable,
                elapsed_ms = started_at.elapsed().as_millis(),
                "GPUI could not complete browser Blob staging cleanup"
            ),
            _ => warn!(
                event = "download.browser_blob.cleanup_request_failed",
                correlation_id = %context.correlation_id,
                handoff_id = %context.handoff_id,
                blob_id = %context.blob.id,
                elapsed_ms = started_at.elapsed().as_millis(),
                "GPUI received an unexpected browser Blob cleanup outcome"
            ),
        }
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

    /// Publishes the export status into the settings view when it is open.
    /// Safe cross-entity write: the result arrives on the app's own update
    /// turn, never while the view itself is leased.
    fn publish_extension_export_status(&self, status: String, cx: &mut Context<Self>) {
        cx.emit(DownloadApplicationEvent::BrowserExtensionExportCompleted { status });
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

    pub(crate) fn request_browser_extension_export(
        &mut self,
        format: &'static str,
    ) -> Result<(), String> {
        if self.pending_extension_export.is_some() {
            return Err("已有导出任务在进行".into());
        }
        match self
            .platform
            .send_command(PlatformCommand::ExportBrowserExtension(
                ExportBrowserExtensionCommand {
                    format: format.into(),
                    destination: String::new(),
                },
            )) {
            Ok(command_id) => {
                self.pending_extension_export = Some(PendingExtensionExport {
                    format,
                    started_at: Instant::now(),
                });
                info!(
                    event = "download.browser_extension.export_accepted",
                    %command_id,
                    format,
                    "browser extension export accepted"
                );
                Ok(())
            }
            Err(error) => {
                warn!(
                    event = "download.browser_extension.export_rejected",
                    %error,
                    format,
                    "browser extension export could not be queued"
                );
                Err(format!("导出失败:{error}"))
            }
        }
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

    pub(crate) fn open_new_download_window(&mut self, cx: &mut Context<Self>) {
        let _ = self.open_new_download_window_with_request(None, None, cx);
    }

    fn open_browser_download_dialog(
        &mut self,
        dialog: BrowserDownloadDialogRequest,
        cx: &mut Context<Self>,
    ) {
        let correlation_id = dialog.correlation_id;
        let handoff_id = dialog.handoff_id;
        let source = match dialog.source {
            BrowserDownloadDialogSource::Download(request) => {
                NewDownloadInitialSource::Download(*request)
            }
            BrowserDownloadDialogSource::Blob {
                blob,
                save_directory,
            } => NewDownloadInitialSource::Blob(BrowserBlobDialogContext {
                correlation_id,
                handoff_id,
                blob,
                save_directory,
            }),
        };
        let result = self.open_new_download_window_with_request(
            Some(source),
            Some((correlation_id, handoff_id)),
            cx,
        );
        let _ = dialog.completion.send(result);
    }

    fn open_new_download_window_with_request(
        &mut self,
        initial_source: Option<NewDownloadInitialSource>,
        browser_context: Option<(uuid::Uuid, uuid::Uuid)>,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if let Some(window) = self.new_download_window {
            if initial_source.is_some() {
                if window.update(cx, |_, _, _| {}).is_ok() {
                    return Err("已有添加下载窗口，请先完成或关闭后重试".into());
                }
            } else if window
                .update(cx, |_, window, _| {
                    window.activate_window();
                    window.refresh();
                })
                .is_ok()
            {
                return Ok(());
            }
            self.new_download_window = None;
        }
        let application = cx.entity();
        let default_directory = self.default_directory.clone();
        let default_connection_limit = self.default_connection_limit();
        let default_adaptive_segment_policy = self.default_adaptive_segment_policy();
        let categories = self
            .config
            .download
            .categories
            .iter()
            .filter(|category| category.enabled)
            .cloned()
            .collect::<Vec<_>>();
        let auto_probe = matches!(initial_source, Some(NewDownloadInitialSource::Download(_)));
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_native_slot = Rc::clone(&native_slot);
        #[cfg(test)]
        let view_slot = Rc::new(RefCell::new(None));
        #[cfg(test)]
        let opened_view_slot = Rc::clone(&view_slot);
        let window_size = initial_new_download_window_size(initial_source.as_ref());
        let window_bounds = WindowBounds::centered(window_size, cx);
        let opened = cx.open_window(
            new_download_window_options(window_bounds),
            move |window, cx| {
                let blob_cleanup = match initial_source.as_ref() {
                    Some(NewDownloadInitialSource::Blob(context)) => Some(context.clone()),
                    _ => None,
                };
                if let Some(blob_context) = blob_cleanup {
                    let cleanup_application = application.clone();
                    window.on_window_should_close(cx, move |_, cx| {
                        cleanup_application
                            .read(cx)
                            .discard_browser_blob(&blob_context);
                        true
                    });
                }
                *opened_native_slot.borrow_mut() = native_window_handle(window).unwrap_or_default();
                let view = cx.new(|cx| {
                    NewDownloadView::new(
                        NewDownloadViewInit {
                            application,
                            default_directory,
                            default_connection_limit,
                            default_adaptive_segment_policy,
                            categories,
                            initial_source,
                        },
                        window,
                        cx,
                    )
                });
                #[cfg(test)]
                {
                    *opened_view_slot.borrow_mut() = Some(view.clone());
                }
                if auto_probe {
                    let view = view.clone();
                    let source_window = window.window_handle();
                    cx.defer(move |cx| {
                        let _ = source_window.update(cx, |_, window, cx| {
                            view.update(cx, |view, cx| view.begin_probe(window, cx));
                        });
                    });
                }
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                let native_window_handle = *native_slot.borrow();
                let _ = window.update(cx, |_, window, _| window.activate_window());
                let correlation_id = browser_context
                    .map(|(correlation_id, _)| correlation_id)
                    .unwrap_or_else(uuid::Uuid::new_v4);
                let topmost_result = self.platform.send_command_correlated(
                    PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                        window_handle: native_window_handle,
                        enabled: true,
                    }),
                    correlation_id,
                );
                let activation_result = self.platform.send_command_correlated(
                    PlatformCommand::ActivateWindow(ActivateWindowCommand {
                        window_handle: native_window_handle,
                    }),
                    correlation_id,
                );
                self.new_download_window = Some(window.into());
                #[cfg(test)]
                {
                    self.new_download_view = view_slot.borrow().clone();
                }
                if let Some((correlation_id, handoff_id)) = browser_context {
                    match (topmost_result, activation_result) {
                        (Ok(topmost_command_id), Ok(activation_command_id)) => info!(
                            event = "download.browser_dialog.foreground_requested",
                            %correlation_id,
                            %handoff_id,
                            %topmost_command_id,
                            %activation_command_id,
                            window_handle = native_window_handle,
                            "browser confirmation window opened and foreground commands queued"
                        ),
                        (topmost, activation) => warn!(
                            event = "download.browser_dialog.foreground_request_failed",
                            %correlation_id,
                            %handoff_id,
                            window_handle = native_window_handle,
                            topmost_accepted = topmost.is_ok(),
                            activation_accepted = activation.is_ok(),
                            "browser confirmation window opened but foreground commands were not fully queued"
                        ),
                    }
                } else {
                    if let Err(error) = topmost_result {
                        warn!(%error, window_handle = native_window_handle, "add-download window topmost request was not queued");
                    }
                    if let Err(error) = activation_result {
                        warn!(%error, window_handle = native_window_handle, "add-download window activation request was not queued");
                    }
                }
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                if let Some((correlation_id, handoff_id)) = browser_context {
                    warn!(
                        event = "download.browser_dialog.open_failed",
                        %correlation_id,
                        %handoff_id,
                        error = %error,
                        "browser confirmation window could not be opened"
                    );
                }
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "download-new-window",
                        "无法打开新建下载窗口",
                        error.to_string(),
                        cx,
                    );
                });
                Err(message)
            }
        }
    }

    pub(crate) fn open_schedule_window(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = self.schedule_window {
            if window
                .update(cx, |_, window, _| {
                    window.activate_window();
                    window.refresh();
                })
                .is_ok()
            {
                return;
            }
            self.schedule_window = None;
        }
        let application = cx.entity();
        let queue = self.default_queue().cloned();
        let window_size = size(px(430.0), px(326.0));
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
                app_id: Some("NxTools.DownloadSchedule".into()),
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
                            application.handle_native_schedule_window_close(closing_window);
                        });
                    });
                    true
                });
                let view = cx.new(|cx| DownloadScheduleView::new(application, queue, window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => self.schedule_window = Some(window.into()),
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "download-schedule-window",
                        "无法打开每日计划",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
    }

    fn close_schedule_window(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = self.schedule_window.take() {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        }
    }

    fn handle_native_schedule_window_close(&mut self, closing_window: AnyWindowHandle) {
        if self.schedule_window == Some(closing_window) {
            self.schedule_window = None;
        }
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

    pub(crate) fn request_task_window(&mut self, task_id: DownloadTaskId, cx: &mut Context<Self>) {
        let phase = self
            .task(task_id)
            .map_or(DownloadWindowPhase::Progress, |task| {
                download_window_phase(task.task.status())
            });
        info!(
            event = "download.task_window.reopen_requested",
            task_id = %task_id.as_uuid(),
            ?phase,
            reused = self.task_windows.contains_key(&task_id),
            "download task window requested"
        );
        let directive = if phase == DownloadWindowPhase::Progress {
            self.task_window_lifecycle.request_progress(task_id)
        } else {
            self.task_window_lifecycle.request_window(task_id, phase)
        };
        self.apply_task_window_directive(task_id, directive, cx);
    }

    fn apply_task_window_directive(
        &mut self,
        task_id: DownloadTaskId,
        directive: DownloadWindowDirective,
        cx: &mut Context<Self>,
    ) {
        match directive {
            DownloadWindowDirective::None => {}
            DownloadWindowDirective::Open(phase) => {
                self.open_task_window(task_id, phase, None, cx);
            }
            DownloadWindowDirective::OpenAndFocus(phase) => {
                self.open_task_window(task_id, phase, Some(TaskWindowActivation::Fresh(phase)), cx);
            }
            DownloadWindowDirective::FocusExisting(expected_phase) => {
                if !self.update_task_window(
                    task_id,
                    Some(TaskWindowActivation::Reused(expected_phase)),
                    None,
                    cx,
                ) {
                    self.clear_task_window_registration(task_id);
                    let retry = self
                        .task_window_lifecycle
                        .request_window(task_id, expected_phase);
                    self.apply_task_window_directive(task_id, retry, cx);
                }
            }
            DownloadWindowDirective::RefreshExisting => {
                if !self.update_task_window(task_id, None, None, cx) {
                    self.clear_task_window_registration(task_id);
                }
            }
            DownloadWindowDirective::TransformExisting(phase) => {
                if !self.update_task_window(task_id, None, Some(phase), cx) {
                    self.clear_task_window_registration(task_id);
                    let retry = self
                        .task_window_lifecycle
                        .reopen_terminal_fallback(task_id, phase);
                    self.apply_task_window_directive(task_id, retry, cx);
                }
            }
            DownloadWindowDirective::CloseExisting => {
                if let Some(entry) = self.clear_task_window_registration(task_id) {
                    let _ = entry
                        .window
                        .update(cx, |_, window, _| window.remove_window());
                }
            }
        }
    }

    fn clear_task_window_registration(
        &mut self,
        task_id: DownloadTaskId,
    ) -> Option<DownloadTaskWindow> {
        clear_pending_task_window_work(
            task_id,
            &mut self.pending_task_window_activations,
            &mut self.pending_task_window_replacements,
        );
        self.task_window_lifecycle.window_closed(task_id);
        self.task_windows.remove(&task_id)
    }

    /// Releases the keyed task-window registration after a native (non-app-driven)
    /// close. The window identity must still match so a replacement window opened
    /// in between is never deregistered by a stale close notification.
    fn handle_task_window_native_close(
        &mut self,
        task_id: DownloadTaskId,
        closing_window: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) {
        let Some(entry) = self.task_windows.get(&task_id) else {
            return;
        };
        if entry.window != closing_window {
            return;
        }
        info!(
            event = "download.task_window.native_close_accepted",
            task_id = %task_id.as_uuid(),
            window_generation = %entry.generation,
            "download task window closed natively; lifecycle registration released"
        );
        self.clear_task_window_registration(task_id);
        cx.notify();
    }

    /// Closes the keyed task window from inside the window itself (an action
    /// click). Clearing the registration in the same turn keeps the lifecycle
    /// authoritative; otherwise the next task event targets a destroyed window
    /// and the recovery path produces a visible remove/re-create flash.
    pub(crate) fn close_task_window(&mut self, task_id: DownloadTaskId, cx: &mut Context<Self>) {
        if let Some(entry) = self.clear_task_window_registration(task_id) {
            let _ = entry
                .window
                .update(cx, |_, window, _| window.remove_window());
        }
    }

    fn open_task_window(
        &mut self,
        task_id: DownloadTaskId,
        phase: DownloadWindowPhase,
        activation: Option<TaskWindowActivation>,
        cx: &mut Context<Self>,
    ) {
        let application = cx.entity();
        let snapshot = self.task(task_id).cloned();
        let progress = self.progress(task_id).cloned();
        let shutdown_armed = self.is_shutdown_armed(task_id);
        let window_size = download_task_window_size(phase);
        let window_bounds = WindowBounds::centered(window_size, cx);
        let window_generation = uuid::Uuid::new_v4();
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_native_slot = Rc::clone(&native_slot);
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        #[cfg(test)]
        let native_window_handle_override = self.task_window_native_handle_override;
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                titlebar: None,
                focus: activation.is_some(),
                show: true,
                kind: WindowKind::Normal,
                is_movable: true,
                is_resizable: false,
                is_minimizable: true,
                display_id: None,
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some(format!("NxTools.Download.{}", task_id.as_uuid())),
                window_min_size: Some(window_size),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                #[cfg(test)]
                let resolved_native_handle = native_window_handle_override
                    .unwrap_or_else(|| native_window_handle(window).unwrap_or_default());
                #[cfg(not(test))]
                let resolved_native_handle = native_window_handle(window).unwrap_or_default();
                *opened_native_slot.borrow_mut() = resolved_native_handle;
                // A native close (title bar X, Alt+F4, taskbar close) must release the keyed
                // lifecycle registration in the same turn, or a later task event targets a
                // destroyed window and produces a visible remove/re-create flash.
                let close_application = application.clone();
                let closing_window = window.window_handle();
                window.on_window_should_close(cx, move |_, cx| {
                    let application = close_application.clone();
                    cx.defer(move |cx| {
                        application.update(cx, |application, cx| {
                            application.handle_task_window_native_close(
                                task_id,
                                closing_window,
                                cx,
                            );
                        });
                    });
                    true
                });
                let view = cx.new(|_| {
                    DownloadTaskWindowView::new(
                        application,
                        task_id,
                        resolved_native_handle,
                        snapshot,
                        progress,
                        shutdown_armed,
                    )
                });
                *opened_view_slot.borrow_mut() = Some(view.clone());
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                let Some(view) = view_slot.borrow().clone() else {
                    self.task_window_lifecycle.window_closed(task_id);
                    return;
                };
                self.task_windows.insert(
                    task_id,
                    DownloadTaskWindow {
                        window: window.into(),
                        view,
                        native_window_handle: *native_slot.borrow(),
                        generation: window_generation,
                    },
                );
                if activation.is_none()
                    && matches!(
                        phase,
                        DownloadWindowPhase::Completed | DownloadWindowPhase::Failed
                    )
                {
                    raise_terminal_window_without_activation(&self.platform, *native_slot.borrow());
                }
                let _ = self.update_task_window(task_id, activation, None, cx);
            }
            Err(error) => {
                self.task_window_lifecycle.window_closed(task_id);
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("download-task-window-{task_id:?}"),
                        "无法打开下载进度窗口",
                        error.to_string(),
                        cx,
                    );
                });
            }
        }
    }

    fn update_task_window(
        &mut self,
        task_id: DownloadTaskId,
        activation: Option<TaskWindowActivation>,
        resize_for: Option<DownloadWindowPhase>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(entry) = self.task_windows.get(&task_id) else {
            return false;
        };
        let snapshot = self.task(task_id).cloned();
        let progress = self.progress(task_id).cloned();
        let shutdown_armed = self.is_shutdown_armed(task_id);
        let native_window_handle = entry.native_window_handle;
        let window_identity = TaskWindowIdentity {
            native_window_handle,
            generation: entry.generation,
        };
        let window_handle = entry.window;
        let view = entry.view.clone();
        let platform = Rc::clone(&self.platform);
        let taskbar_progress = snapshot
            .as_ref()
            .map(|task| taskbar_progress_command(native_window_handle, task, progress.as_ref()));
        view.update(cx, |view, cx| {
            view.replace_snapshot(snapshot, progress, shutdown_armed, cx);
        });
        let updated = window_handle
            .update(cx, move |_, window, _| {
                if let Some(phase) = resize_for {
                    let new_size = download_task_window_size(phase);
                    let bounds = window.bounds();
                    let scale = window.scale_factor();
                    if native_window_handle != 0 {
                        let _ = platform.send_command(PlatformCommand::SetWindowBounds(
                            centered_task_window_bounds_command(
                                native_window_handle,
                                bounds,
                                new_size,
                                scale,
                            ),
                        ));
                    } else {
                        window.resize(new_size);
                    }
                }
                if let Some(command) = taskbar_progress {
                    let _ = platform.send_command(PlatformCommand::SetTaskbarProgress(command));
                }
                if matches!(
                    resize_for,
                    Some(DownloadWindowPhase::Completed | DownloadWindowPhase::Failed)
                ) {
                    raise_terminal_window_without_activation(&platform, native_window_handle);
                }
                if activation.is_some() {
                    window.activate_window();
                }
                window.refresh();
            })
            .is_ok();
        if updated && let Some(activation) = activation {
            self.queue_task_window_activation(task_id, window_identity, activation, cx);
        }
        updated
    }

    fn queue_task_window_activation(
        &mut self,
        task_id: DownloadTaskId,
        window: TaskWindowIdentity,
        activation: TaskWindowActivation,
        cx: &mut Context<Self>,
    ) {
        let Some(command) = task_window_activation_command(window.native_window_handle) else {
            warn!(
                event = "download.task_window.activation_rejected",
                success = false,
                task_id = %task_id.as_uuid(),
                window_generation = %window.generation,
                reason = "native_window_unavailable",
                "download task window native activation was unavailable"
            );
            if activation.replace_on_failure() {
                self.schedule_task_window_replacement(
                    task_id,
                    window,
                    activation.phase(),
                    "native_window_unavailable",
                    cx,
                );
            }
            return;
        };
        if self
            .pending_task_window_activations
            .values()
            .any(|pending| pending.task_id == task_id && pending.window == window)
        {
            return;
        }
        let started_at = Instant::now();
        #[cfg(test)]
        let send_result = if self.task_window_native_handle_override.is_some() {
            self.task_window_activation_commands
                .borrow_mut()
                .push(command);
            Ok(uuid::Uuid::new_v4())
        } else {
            self.platform.send_command(command)
        };
        #[cfg(not(test))]
        let send_result = self.platform.send_command(command);
        match send_result {
            Ok(correlation_id) => {
                self.pending_task_window_activations.insert(
                    correlation_id,
                    PendingTaskWindowActivation {
                        task_id,
                        window,
                        activation,
                        started_at,
                    },
                );
                info!(
                    event = "download.task_window.activation_accepted",
                    accepted = true,
                    %correlation_id,
                    task_id = %task_id.as_uuid(),
                    window_handle = window.native_window_handle,
                    window_generation = %window.generation,
                    "download task window activation accepted"
                );
                cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(Duration::from_secs(5)).await;
                    let _ = this.update(cx, |application, cx| {
                        application.task_window_activation_timed_out(correlation_id, cx);
                    });
                })
                .detach();
            }
            Err(error) => {
                warn!(
                    event = "download.task_window.activation_rejected",
                    success = false,
                    task_id = %task_id.as_uuid(),
                    window_handle = window.native_window_handle,
                    window_generation = %window.generation,
                    %error,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "download task window activation was rejected"
                );
                if activation.replace_on_failure() {
                    self.schedule_task_window_replacement(
                        task_id,
                        window,
                        activation.phase(),
                        "platform_command_rejected",
                        cx,
                    );
                }
            }
        }
    }

    fn task_window_activation_timed_out(
        &mut self,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending_task_window_activations.remove(&correlation_id) else {
            return;
        };
        warn!(
            event = "download.task_window.activation_timed_out",
            success = false,
            %correlation_id,
            task_id = %pending.task_id.as_uuid(),
            window_handle = pending.window.native_window_handle,
            window_generation = %pending.window.generation,
            elapsed_ms = pending.started_at.elapsed().as_millis(),
            "download task window activation timed out"
        );
        if pending.activation.replace_on_failure() {
            self.schedule_task_window_replacement(
                pending.task_id,
                pending.window,
                pending.activation.phase(),
                "platform_command_timeout",
                cx,
            );
        }
    }

    fn schedule_task_window_replacement(
        &mut self,
        task_id: DownloadTaskId,
        window: TaskWindowIdentity,
        expected_phase: DownloadWindowPhase,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        if self.pending_task_window_replacements.contains_key(&task_id) {
            return;
        }
        self.pending_task_window_replacements
            .insert(task_id, window);
        info!(
            event = "download.task_window.replacement_scheduled",
            task_id = %task_id.as_uuid(),
            window_handle = window.native_window_handle,
            window_generation = %window.generation,
            ?expected_phase,
            reason,
            "download task window replacement scheduled"
        );
        let application = cx.entity();
        cx.defer(move |cx| {
            application.update(cx, |application, cx| {
                application.replace_task_window(task_id, expected_phase, reason, cx);
            });
        });
    }

    fn replace_task_window(
        &mut self,
        task_id: DownloadTaskId,
        expected_phase: DownloadWindowPhase,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some(stale_window) = self.pending_task_window_replacements.remove(&task_id) else {
            return;
        };
        let current_window = self
            .task_windows
            .get(&task_id)
            .map(|entry| TaskWindowIdentity {
                native_window_handle: entry.native_window_handle,
                generation: entry.generation,
            });
        if !replacement_targets_current_window(current_window, stale_window) {
            info!(
                event = "download.task_window.replacement_aborted",
                task_id = %task_id.as_uuid(),
                stale_window_handle = stale_window.native_window_handle,
                stale_window_generation = %stale_window.generation,
                ?current_window,
                ?expected_phase,
                reason = "window_generation_changed",
                "download task window replacement no longer targets the current window"
            );
            return;
        }
        self.pending_task_window_activations
            .retain(|_, pending| pending.task_id != task_id);
        if let Some(entry) = self.task_windows.remove(&task_id) {
            let _ = entry
                .window
                .update(cx, |_, window, _| window.remove_window());
        }
        self.task_window_lifecycle.window_closed(task_id);
        let Some(task) = self.task(task_id) else {
            warn!(
                event = "download.task_window.replacement_aborted",
                task_id = %task_id.as_uuid(),
                ?expected_phase,
                reason = "task_missing",
                "download task window replacement was aborted"
            );
            return;
        };
        let phase = download_window_phase(task.task.status());
        info!(
            event = "download.task_window.replacement_started",
            task_id = %task_id.as_uuid(),
            ?phase,
            reason,
            "download task window replacement started"
        );
        let directive = self.task_window_lifecycle.request_window(task_id, phase);
        self.apply_task_window_directive(task_id, directive, cx);
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

    fn publish_terminal_notifications(
        &mut self,
        tasks: &[DownloadEngineTask],
        cx: &mut Context<Self>,
    ) {
        let mut queued_feedback = false;
        for task in tasks {
            let status = task.task.status();
            if !matches!(status, DownloadStatus::Completed | DownloadStatus::Failed)
                || !self
                    .notified_terminal_states
                    .insert((task.task.id(), status))
            {
                continue;
            }
            let display_name = task
                .final_path
                .as_ref()
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .or(task.task.request().file_name.as_deref())
                .unwrap_or("下载任务")
                .to_owned();
            self.completion_actions.record_terminal(CompletionTerminal {
                task_id: task.task.id(),
                status,
                directory: task
                    .final_path
                    .as_ref()
                    .and_then(|path| path.parent())
                    .map(Path::to_path_buf),
                display_name,
                error_message: task.error.as_ref().map(|error| error.message.clone()),
            });
            queued_feedback = true;
        }
        if queued_feedback && !self.completion_feedback_scheduled {
            self.completion_feedback_scheduled = true;
            cx.spawn(async move |this, cx| {
                cx.background_executor()
                    .timer(COMPLETION_FEEDBACK_DELAY)
                    .await;
                let _ = this.update(cx, |application, cx| {
                    application.flush_completion_feedback(cx);
                });
            })
            .detach();
        }
        self.evaluate_shutdown(tasks, cx);
    }

    fn flush_completion_feedback(&mut self, cx: &mut Context<Self>) {
        self.completion_feedback_scheduled = false;
        let Some(batch) = self.completion_actions.take_feedback_batch() else {
            return;
        };
        let correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        let sound_enabled =
            self.config.download.completion_sound_enabled && batch.completed_count > 0;
        let directory_enabled =
            self.config.download.open_directory_on_completion && batch.completed_count > 0;
        info!(
            event = "download.completion_feedback.started",
            %correlation_id,
            completed_count = batch.completed_count,
            failed_count = batch.failed_count,
            directory_count = if directory_enabled { batch.directories.len() } else { 0 },
            suppressed_directory_count = if directory_enabled { batch.suppressed_directory_count } else { 0 },
            sound_enabled,
            directory_enabled,
            "coalesced download completion feedback started"
        );
        self.show_completion_batch_notification(&batch, correlation_id, cx);

        let mut operation = PendingCompletionFeedback {
            correlation_id,
            started_at,
            completed_count: batch.completed_count,
            failed_count: batch.failed_count,
            directory_count: if directory_enabled {
                batch.directories.len()
            } else {
                0
            },
            suppressed_directory_count: if directory_enabled {
                batch.suppressed_directory_count
            } else {
                0
            },
            sound_enabled,
            directory_enabled,
            pending_commands: HashSet::new(),
            failed_commands: 0,
            native_error_code: None,
        };
        let mut commands = Vec::new();
        if sound_enabled {
            commands.push(PlatformCommand::PlaySystemNotificationSound);
        }
        if directory_enabled {
            commands.extend(batch.directories.into_iter().map(|directory| {
                PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                    target: directory.to_string_lossy().into_owned(),
                    action: ShellOpenAction::Open,
                })
            }));
        }
        for command in commands {
            match self
                .platform
                .send_command_correlated(command, correlation_id)
            {
                Ok(command_id) => {
                    operation.pending_commands.insert(command_id);
                    self.completion_feedback_commands
                        .insert(command_id, correlation_id);
                }
                Err(error) => {
                    operation.failed_commands += 1;
                    warn!(
                        event = "download.completion_feedback.command_enqueue_failed",
                        %correlation_id,
                        error_kind = "platform_enqueue",
                        %error,
                        "completion feedback command could not be queued"
                    );
                }
            }
        }
        if operation.pending_commands.is_empty() {
            self.finish_completion_feedback(operation);
            return;
        }
        self.pending_completion_feedback
            .insert(correlation_id, operation);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(COMPLETION_PLATFORM_TIMEOUT)
                .await;
            let _ = this.update(cx, |application, _| {
                application.completion_feedback_timed_out(correlation_id);
            });
        })
        .detach();
    }

    fn show_completion_batch_notification(
        &self,
        batch: &CompletionFeedbackBatch,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        let id = format!("download-completion-{correlation_id}");
        if batch.completed_count > 0 && batch.failed_count == 0 {
            let message = if batch.completed_count == 1 {
                batch
                    .single_display_name
                    .clone()
                    .unwrap_or_else(|| "下载文件已完成".into())
            } else {
                format!("{} 个下载任务已完成", batch.completed_count)
            };
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_success(id, "下载完成", message, cx);
            });
        } else {
            let message = if batch.failed_count == 1 && batch.completed_count == 0 {
                batch
                    .single_error_message
                    .clone()
                    .unwrap_or_else(|| "下载任务失败，未返回详细原因".into())
            } else {
                format!(
                    "完成 {} 个，失败 {} 个",
                    batch.completed_count, batch.failed_count
                )
            };
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(id, "下载批次已结束", message, cx);
            });
        }
    }

    fn handle_completion_feedback_result(
        &mut self,
        operation_id: uuid::Uuid,
        command_id: uuid::Uuid,
        result: PlatformCommandResult,
        _cx: &mut Context<Self>,
    ) {
        let Some(operation) = self.pending_completion_feedback.get_mut(&operation_id) else {
            return;
        };
        operation.pending_commands.remove(&command_id);
        if !result.success {
            operation.failed_commands += 1;
            operation.native_error_code = operation.native_error_code.or(result.native_error_code);
        }
        if operation.pending_commands.is_empty()
            && let Some(operation) = self.pending_completion_feedback.remove(&operation_id)
        {
            self.finish_completion_feedback(operation);
        }
    }

    fn completion_feedback_timed_out(&mut self, operation_id: uuid::Uuid) {
        let Some(mut operation) = self.pending_completion_feedback.remove(&operation_id) else {
            return;
        };
        for command_id in &operation.pending_commands {
            self.completion_feedback_commands.remove(command_id);
        }
        operation.failed_commands += operation.pending_commands.len();
        operation.pending_commands.clear();
        self.finish_completion_feedback(operation);
    }

    fn finish_completion_feedback(&self, operation: PendingCompletionFeedback) {
        if operation.failed_commands == 0 {
            info!(
                event = "download.completion_feedback.completed",
                correlation_id = %operation.correlation_id,
                completed_count = operation.completed_count,
                failed_count = operation.failed_count,
                directory_count = operation.directory_count,
                suppressed_directory_count = operation.suppressed_directory_count,
                sound_enabled = operation.sound_enabled,
                directory_enabled = operation.directory_enabled,
                elapsed_ms = operation.started_at.elapsed().as_millis(),
                "coalesced download completion feedback completed"
            );
        } else {
            warn!(
                event = "download.completion_feedback.failed",
                correlation_id = %operation.correlation_id,
                completed_count = operation.completed_count,
                failed_count = operation.failed_count,
                directory_count = operation.directory_count,
                suppressed_directory_count = operation.suppressed_directory_count,
                sound_enabled = operation.sound_enabled,
                directory_enabled = operation.directory_enabled,
                failed_command_count = operation.failed_commands,
                native_error_code = operation.native_error_code,
                elapsed_ms = operation.started_at.elapsed().as_millis(),
                "coalesced download completion feedback failed"
            );
        }
    }

    fn is_shutdown_armed(&self, task_id: DownloadTaskId) -> bool {
        self.completion_actions.is_shutdown_armed(task_id)
    }

    fn toggle_shutdown_armed(&mut self, task_id: DownloadTaskId, cx: &mut Context<Self>) {
        if self.pending_shutdown_command.is_some() {
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();
        info!(
            event = "download.shutdown_arm.started",
            %correlation_id,
            task_id = %task_id.as_uuid(),
            persistence = "process_only",
            "task-local shutdown arm update started"
        );
        let eligible = self
            .engine
            .task(task_id)
            .ok()
            .flatten()
            .is_some_and(|task| {
                !matches!(
                    task.task.status(),
                    DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
                )
            });
        if !eligible {
            warn!(
                event = "download.shutdown_arm.failed",
                %correlation_id,
                task_id = %task_id.as_uuid(),
                failure_kind = "task_not_active",
                elapsed_ms = started_at.elapsed().as_millis(),
                "task-local shutdown arm update rejected"
            );
            return;
        }
        let armed = !self.completion_actions.is_shutdown_armed(task_id);
        self.completion_actions.set_shutdown_armed(task_id, armed);
        info!(
            event = "download.shutdown_arm.completed",
            %correlation_id,
            task_id = %task_id.as_uuid(),
            armed,
            persistence = "process_only",
            elapsed_ms = started_at.elapsed().as_millis(),
            "task-local shutdown arm update completed"
        );
        if let Some(entry) = self.task_windows.get(&task_id) {
            entry.view.update(cx, |view, cx| {
                view.shutdown_armed = armed;
                cx.notify();
            });
        }
        let tasks = self
            .tasks
            .iter()
            .map(|task| (task.task.id(), task.task.status()))
            .collect::<Vec<_>>();
        let transition = self
            .completion_actions
            .evaluate_shutdown(tasks, Instant::now());
        self.apply_shutdown_transition(transition, cx);
    }

    fn evaluate_shutdown(&mut self, tasks: &[DownloadEngineTask], cx: &mut Context<Self>) {
        let task_states = tasks
            .iter()
            .map(|task| (task.task.id(), task.task.status()))
            .collect::<Vec<_>>();
        let transition = self
            .completion_actions
            .evaluate_shutdown(task_states, Instant::now());
        self.apply_shutdown_transition(transition, cx);
    }

    fn apply_shutdown_transition(
        &mut self,
        transition: ShutdownTransition,
        cx: &mut Context<Self>,
    ) {
        match transition {
            ShutdownTransition::None => {}
            ShutdownTransition::Started(countdown) => {
                let application = cx.entity();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.open_shutdown_countdown(countdown, cx);
                    });
                });
            }
            ShutdownTransition::Cancelled(countdown, reason) => {
                let application = cx.entity();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.finish_shutdown_cancelled(countdown, reason, cx);
                    });
                });
            }
        }
    }

    fn open_shutdown_countdown(&mut self, countdown: ShutdownCountdown, cx: &mut Context<Self>) {
        if self.shutdown_window.is_some() {
            return;
        }
        info!(
            event = "download.shutdown_countdown.started",
            correlation_id = %countdown.correlation_id,
            countdown_seconds = countdown.duration.as_secs(),
            reason = "queue_completed",
            "download shutdown countdown started"
        );
        let application = cx.entity();
        let native_handle = Rc::new(RefCell::new(0_i64));
        let opened_native_handle = Rc::clone(&native_handle);
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let window_size = size(px(450.0), px(200.0));
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
                app_id: Some("NxTools.DownloadShutdown".into()),
                window_min_size: Some(window_size),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let close_application = application.clone();
                window.on_window_should_close(cx, move |_, cx| {
                    let accepted = close_application
                        .read(cx)
                        .can_native_close_shutdown_countdown(countdown.correlation_id);
                    if accepted {
                        let application = close_application.clone();
                        cx.defer(move |cx| {
                            application.update(cx, |application, _| {
                                application.finish_native_close_shutdown_countdown(
                                    countdown.correlation_id,
                                );
                            });
                        });
                    }
                    accepted
                });
                *opened_native_handle.borrow_mut() =
                    native_window_handle(window).unwrap_or_default();
                let view = cx.new(|_| {
                    DownloadShutdownCountdownView::new(
                        application,
                        countdown.correlation_id,
                        countdown.duration.as_secs(),
                    )
                });
                *opened_view_slot.borrow_mut() = Some(view.clone());
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        let Ok(window) = opened else {
            let _ = self.completion_actions.clear_countdown();
            warn!(
                event = "download.shutdown_countdown.failed",
                correlation_id = %countdown.correlation_id,
                failure_kind = "window_creation",
                native_error_code = Option::<i32>::None,
                elapsed_ms = countdown.started_at.elapsed().as_millis(),
                "download shutdown countdown window could not be created"
            );
            return;
        };
        let Some(view) = view_slot.borrow().clone() else {
            let _ = self.completion_actions.clear_countdown();
            let _ = window.update(cx, |_, window, _| window.remove_window());
            warn!(
                event = "download.shutdown_countdown.failed",
                correlation_id = %countdown.correlation_id,
                failure_kind = "window_view_missing",
                native_error_code = Option::<i32>::None,
                elapsed_ms = countdown.started_at.elapsed().as_millis(),
                "download shutdown countdown view was not attached"
            );
            return;
        };
        self.shutdown_window = Some(DownloadShutdownWindow {
            window: window.into(),
            view,
            correlation_id: countdown.correlation_id,
        });
        let native_handle = *native_handle.borrow();
        if native_handle != 0 {
            let _ = self.platform.send_command_correlated(
                PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                    window_handle: native_handle,
                    enabled: true,
                }),
                countdown.correlation_id,
            );
        }
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let Ok(active) = this.update(cx, |application, cx| {
                    application.tick_shutdown_countdown(countdown.correlation_id, cx)
                }) else {
                    return;
                };
                if !active {
                    return;
                }
            }
        })
        .detach();
    }

    fn tick_shutdown_countdown(
        &mut self,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(countdown) = self.completion_actions.active_countdown() else {
            return false;
        };
        if countdown.correlation_id != correlation_id || self.pending_shutdown_command.is_some() {
            return false;
        }
        let now = Instant::now();
        let remaining = countdown
            .deadline
            .saturating_duration_since(now)
            .as_secs()
            .saturating_add(u64::from(now < countdown.deadline));
        if let Some(window) = &self.shutdown_window {
            window.view.update(cx, |view, cx| {
                view.remaining_seconds = remaining;
                cx.notify();
            });
        }
        if now >= countdown.deadline {
            self.execute_shutdown(correlation_id, "countdown_elapsed", cx);
            return false;
        }
        true
    }

    fn cancel_shutdown_countdown(
        &mut self,
        correlation_id: uuid::Uuid,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        if self.pending_shutdown_command.is_some() {
            return;
        }
        let Some(countdown) = self.completion_actions.clear_countdown() else {
            return;
        };
        if countdown.correlation_id != correlation_id {
            return;
        }
        self.finish_shutdown_cancelled(countdown, reason, cx);
    }

    fn finish_shutdown_cancelled(
        &mut self,
        countdown: ShutdownCountdown,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        info!(
            event = "download.shutdown_countdown.cancelled",
            correlation_id = %countdown.correlation_id,
            reason,
            elapsed_ms = countdown.started_at.elapsed().as_millis(),
            "download shutdown countdown cancelled"
        );
        self.close_shutdown_window(countdown.correlation_id, cx);
    }

    fn can_native_close_shutdown_countdown(&self, correlation_id: uuid::Uuid) -> bool {
        self.pending_shutdown_command.is_none()
            && self
                .completion_actions
                .active_countdown()
                .is_none_or(|countdown| countdown.correlation_id == correlation_id)
    }

    fn finish_native_close_shutdown_countdown(&mut self, correlation_id: uuid::Uuid) {
        if !self.can_native_close_shutdown_countdown(correlation_id) {
            return;
        }
        let Some(countdown) = self.completion_actions.clear_countdown() else {
            if self
                .shutdown_window
                .as_ref()
                .is_some_and(|window| window.correlation_id == correlation_id)
            {
                self.shutdown_window.take();
            }
            return;
        };
        info!(
            event = "download.shutdown_countdown.cancelled",
            %correlation_id,
            reason = "window_closed",
            elapsed_ms = countdown.started_at.elapsed().as_millis(),
            "download shutdown countdown cancelled by native window close"
        );
        if self
            .shutdown_window
            .as_ref()
            .is_some_and(|window| window.correlation_id == correlation_id)
        {
            self.shutdown_window.take();
        }
    }

    fn execute_shutdown(
        &mut self,
        correlation_id: uuid::Uuid,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        if self.pending_shutdown_command.is_some() {
            return;
        }
        let Some(countdown) = self
            .completion_actions
            .take_countdown_for_execution(correlation_id)
        else {
            return;
        };
        if let Some(window) = &self.shutdown_window {
            window.view.update(cx, |view, cx| {
                view.executing = true;
                cx.notify();
            });
        }
        info!(
            event = "download.shutdown_countdown.execute_requested",
            %correlation_id,
            reason,
            "download shutdown command requested"
        );
        match self.platform.send_command_correlated(
            PlatformCommand::ShutdownSystem(ShutdownSystemCommand { delay_seconds: 1 }),
            correlation_id,
        ) {
            Ok(command_id) => {
                self.pending_shutdown_command = Some(PendingShutdownCommand {
                    command_id,
                    countdown,
                });
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(COMPLETION_PLATFORM_TIMEOUT)
                        .await;
                    let _ = this.update(cx, |application, cx| {
                        application.shutdown_command_timed_out(command_id, cx);
                    });
                })
                .detach();
            }
            Err(error) => {
                warn!(
                    event = "download.shutdown_countdown.failed",
                    %correlation_id,
                    failure_kind = "platform_enqueue",
                    native_error_code = Option::<i32>::None,
                    elapsed_ms = countdown.started_at.elapsed().as_millis(),
                    %error,
                    "download shutdown command could not be queued"
                );
                self.show_shutdown_failure("无法请求 Windows 关机", cx);
                self.close_shutdown_window(correlation_id, cx);
            }
        }
    }

    fn handle_shutdown_result(
        &mut self,
        command_id: uuid::Uuid,
        result: PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending_shutdown_command.take() else {
            return;
        };
        if pending.command_id != command_id {
            self.pending_shutdown_command = Some(pending);
            return;
        }
        if result.success {
            info!(
                event = "download.shutdown_countdown.completed",
                correlation_id = %pending.countdown.correlation_id,
                elapsed_ms = pending.countdown.started_at.elapsed().as_millis(),
                "Windows accepted the download shutdown request"
            );
        } else {
            warn!(
                event = "download.shutdown_countdown.failed",
                correlation_id = %pending.countdown.correlation_id,
                failure_kind = "native_rejected",
                native_error_code = result.native_error_code,
                elapsed_ms = pending.countdown.started_at.elapsed().as_millis(),
                "Windows rejected the download shutdown request"
            );
            self.show_shutdown_failure(
                result.error.as_deref().unwrap_or("Windows 拒绝了关机请求"),
                cx,
            );
        }
        self.close_shutdown_window(pending.countdown.correlation_id, cx);
    }

    fn shutdown_command_timed_out(&mut self, command_id: uuid::Uuid, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_shutdown_command.take() else {
            return;
        };
        if pending.command_id != command_id {
            self.pending_shutdown_command = Some(pending);
            return;
        }
        warn!(
            event = "download.shutdown_countdown.failed",
            correlation_id = %pending.countdown.correlation_id,
            failure_kind = "platform_timeout",
            native_error_code = Option::<i32>::None,
            elapsed_ms = pending.countdown.started_at.elapsed().as_millis(),
            "download shutdown command timed out"
        );
        self.show_shutdown_failure("Windows 关机请求超时", cx);
        self.close_shutdown_window(pending.countdown.correlation_id, cx);
    }

    fn show_shutdown_failure(&self, message: &str, cx: &mut Context<Self>) {
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_error(
                "download-shutdown-failed",
                "自动关机失败",
                message.to_owned(),
                cx,
            );
        });
    }

    fn close_shutdown_window(&mut self, correlation_id: uuid::Uuid, cx: &mut Context<Self>) {
        if self
            .shutdown_window
            .as_ref()
            .is_some_and(|window| window.correlation_id == correlation_id)
            && let Some(window) = self.shutdown_window.take()
        {
            let _ = window
                .window
                .update(cx, |_, window, _| window.remove_window());
        }
    }
}

const DOWNLOAD_SETTINGS_WINDOW_WIDTH: f32 = 984.0;
const DOWNLOAD_SETTINGS_WINDOW_HEIGHT: f32 = 650.0;
const DOWNLOAD_REQUEST_HEADER_EDITOR_HEIGHT: f32 = 120.0;

fn download_settings_window_options(cx: &App) -> WindowOptions {
    // Match the reference settings workspace: a stable navigation rail plus a content column
    // wide enough for two compact settings rows without relying on child min-width to size it.
    let window_size = size(
        px(DOWNLOAD_SETTINGS_WINDOW_WIDTH),
        px(DOWNLOAD_SETTINGS_WINDOW_HEIGHT),
    );
    WindowOptions {
        window_bounds: Some(WindowBounds::centered(window_size, cx)),
        titlebar: None,
        focus: true,
        show: true,
        kind: WindowKind::Normal,
        is_movable: true,
        is_resizable: false,
        is_minimizable: false,
        display_id: None,
        window_background: WindowBackgroundAppearance::Opaque,
        app_id: Some("NxTools.DownloadSettings".into()),
        window_min_size: Some(window_size),
        window_decorations: Some(WindowDecorations::Client),
        tabbing_identifier: None,
    }
}

fn task_window_activation_command(window_handle: i64) -> Option<PlatformCommand> {
    (window_handle != 0).then_some(PlatformCommand::ActivateWindow(ActivateWindowCommand {
        window_handle,
    }))
}

fn replacement_targets_current_window(
    current: Option<TaskWindowIdentity>,
    stale: TaskWindowIdentity,
) -> bool {
    current == Some(stale)
}

fn clear_pending_task_window_work(
    task_id: DownloadTaskId,
    activations: &mut HashMap<uuid::Uuid, PendingTaskWindowActivation>,
    replacements: &mut HashMap<DownloadTaskId, TaskWindowIdentity>,
) {
    activations.retain(|_, pending| pending.task_id != task_id);
    replacements.remove(&task_id);
}

fn download_window_phase(status: DownloadStatus) -> DownloadWindowPhase {
    match status {
        DownloadStatus::Completed => DownloadWindowPhase::Completed,
        DownloadStatus::Failed => DownloadWindowPhase::Failed,
        _ => DownloadWindowPhase::Progress,
    }
}

fn raise_terminal_window_without_activation(platform: &PlatformBridge, window_handle: i64) {
    if window_handle == 0 {
        return;
    }
    let _ = platform.send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
        window_handle,
        enabled: true,
    }));
}

fn load_download_snapshot(engine: &DownloadEngine) -> Result<DownloadSnapshot, String> {
    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::Snapshot {
            query: DownloadQuery::default(),
        },
    });
    match result.outcome {
        DownloadCommandOutcome::Snapshot(snapshot) => Ok(snapshot),
        DownloadCommandOutcome::Rejected { fault } => Err(fault.message),
        _ => Err("下载引擎返回了无效的快照结果".into()),
    }
}

fn load_download_task_detail(
    engine: &DownloadEngine,
    task_id: DownloadTaskId,
) -> Result<DownloadTaskDetail, String> {
    let result = engine.handle_command(DownloadCommandEnvelope {
        protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
        request_id: uuid::Uuid::new_v4(),
        origin: RequestOrigin::Ui,
        command: DownloadCommand::TaskDetail { task_id },
    });
    match result.outcome {
        DownloadCommandOutcome::TaskDetail(detail) => Ok(*detail),
        DownloadCommandOutcome::Rejected { fault } => Err(fault.message),
        _ => Err("下载引擎返回了无效的任务详情结果".into()),
    }
}

fn apply_progress_to_task_detail(detail: &mut DownloadTaskDetail, progress: &DownloadProgress) {
    detail.task.downloaded_bytes = progress.downloaded_bytes;
    detail.task.total_bytes = progress.total_bytes;
    for segment_progress in &progress.segments {
        let Some(position) = detail
            .segments
            .iter()
            .position(|segment| segment.index() == segment_progress.segment_index)
        else {
            continue;
        };
        let segment = &detail.segments[position];
        let downloaded_bytes = segment_progress.downloaded_bytes.min(segment.len());
        let state = if downloaded_bytes == segment.len() {
            DownloadSegmentState::Completed
        } else if downloaded_bytes > segment.downloaded_bytes() {
            DownloadSegmentState::Active
        } else {
            segment.state
        };
        if let Ok(updated) = DownloadSegment::new(
            segment.index(),
            segment.start_byte(),
            segment.end_byte(),
            downloaded_bytes,
            state,
        ) {
            detail.segments[position] = updated;
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletedTaskWindowAction {
    Drag,
    Reveal,
    Open,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompletedTaskWindowEmphasis {
    Primary,
    Outline,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CompletedTaskWindowActionSpec {
    button_id: &'static str,
    role_selector: &'static str,
    icon: MaterialIcon,
    label: Option<&'static str>,
    emphasis: CompletedTaskWindowEmphasis,
    tooltip: Option<&'static str>,
    tooltip_selector: Option<&'static str>,
}

impl CompletedTaskWindowAction {
    #[cfg(test)]
    const ALL: [Self; 3] = [Self::Drag, Self::Reveal, Self::Open];

    fn spec(self) -> CompletedTaskWindowActionSpec {
        match self {
            Self::Drag => CompletedTaskWindowActionSpec {
                button_id: "download-complete-drag",
                role_selector: "download-complete-secondary-drag-action",
                icon: MaterialIcon::OpenInFull,
                label: None,
                emphasis: CompletedTaskWindowEmphasis::Outline,
                tooltip: Some("拖拽文件"),
                tooltip_selector: Some("download-complete-drag-tooltip"),
            },
            Self::Reveal => CompletedTaskWindowActionSpec {
                button_id: "download-complete-reveal",
                role_selector: "download-complete-secondary-reveal-action",
                icon: MaterialIcon::Folder,
                label: Some("打开所在目录"),
                emphasis: CompletedTaskWindowEmphasis::Outline,
                tooltip: None,
                tooltip_selector: None,
            },
            Self::Open => CompletedTaskWindowActionSpec {
                button_id: "download-complete-open",
                role_selector: "download-complete-primary-open-action",
                icon: MaterialIcon::OpenInNew,
                label: Some("打开文件"),
                emphasis: CompletedTaskWindowEmphasis::Primary,
                tooltip: None,
                tooltip_selector: None,
            },
        }
    }
}

fn completed_task_window_action(
    action: CompletedTaskWindowAction,
    theme: Theme,
    cx: &App,
) -> Button {
    let spec = action.spec();
    let button = match spec.label {
        Some(label) => {
            action_button(spec.button_id, label).icon(Icon::new(spec.icon).size(px(14.0)))
        }
        None => icon_action_button(spec.button_id, spec.icon),
    };
    let button = match spec.emphasis {
        CompletedTaskWindowEmphasis::Primary => button.custom(primary_button_variant(theme, cx)),
        CompletedTaskWindowEmphasis::Outline => button.outline(),
    };
    button.debug_selector(move || spec.role_selector.into())
}

fn download_window_metric(theme: Theme, label: &'static str, value: String) -> impl IntoElement {
    div()
        .min_w_0()
        .flex_1()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .text_size(px(TEXT_LABEL))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .text_size(px(TEXT_META))
                .font_weight(FontWeight::MEDIUM)
                .truncate()
                .child(value),
        )
}

pub(crate) fn download_window_titlebar(theme: Theme, title: &'static str) -> impl IntoElement {
    div()
        .h(px(34.0))
        .pl(px(14.0))
        .border_b_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .window_control_area(WindowControlArea::Drag)
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .text_size(px(TEXT_BODY))
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(download_window_close_button(theme))
}

fn download_settings_titlebar(
    theme: Theme,
    application: Entity<DownloadApplication>,
    editing: bool,
) -> impl IntoElement {
    client_titlebar(
        theme,
        "download-settings-titlebar",
        "download-settings-close",
        "下载设置",
        move |_, window, cx| {
            if editing {
                let application = application.clone();
                let correlation_id = uuid::Uuid::new_v4();
                cx.defer(move |cx| {
                    application.update(cx, |application, cx| {
                        application.finish_taskbar_overlay_edit(false, correlation_id, cx);
                    });
                });
            }
            window.remove_window();
        },
    )
}

pub(crate) fn download_primary_button_variant(theme: Theme, cx: &gpui::App) -> ButtonCustomVariant {
    ButtonCustomVariant::new(cx)
        .color(theme.accent.into())
        .foreground(theme.accent_text.into())
        .border(theme.border_active.into())
        .hover(theme.border_active.into())
        .active(theme.accent.into())
}

fn download_window_close_button(theme: Theme) -> impl IntoElement {
    div()
        .id("download-window-close")
        .w(px(38.0))
        .h_full()
        .cursor_pointer()
        .flex()
        .items_center()
        .justify_center()
        .hover(move |style| style.bg(theme.danger).text_color(gpui::white()))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(|_, window, _| window.remove_window())
        .child(Icon::new(MaterialIcon::Close).size(px(15.0)))
}

fn download_task_status(status: DownloadStatus) -> (&'static str, gpui::Rgba) {
    let theme = Theme::zed_dark();
    match status {
        DownloadStatus::Created => ("等待开始", theme.text_muted),
        DownloadStatus::Probing => ("正在探测", theme.accent),
        DownloadStatus::Waiting => ("等待下载", theme.accent),
        DownloadStatus::Downloading => ("正在下载", theme.accent),
        DownloadStatus::Paused => ("已暂停", theme.text_secondary),
        DownloadStatus::Finalizing => ("正在落盘", theme.accent),
        DownloadStatus::Verifying => ("正在校验", theme.accent),
        DownloadStatus::Completed => ("下载完成", theme.success),
        DownloadStatus::Failed => ("下载失败", theme.danger),
        DownloadStatus::Cancelled => ("已取消", theme.text_muted),
    }
}

fn format_download_eta(seconds: u64) -> String {
    if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            seconds / 60 % 60,
            seconds % 60
        )
    } else {
        format!("{:02}:{:02}", seconds / 60, seconds % 60)
    }
}

fn taskbar_progress_command(
    window_handle: i64,
    task: &DownloadEngineTask,
    progress: Option<&DownloadProgress>,
) -> SetTaskbarProgressCommand {
    let downloaded = progress.map_or(task.downloaded_bytes, |progress| progress.downloaded_bytes);
    let total = progress
        .and_then(|progress| progress.total_bytes)
        .or(task.total_bytes);
    let status = task.task.status();
    let state = match status {
        DownloadStatus::Created | DownloadStatus::Probing | DownloadStatus::Waiting
            if total.is_none() =>
        {
            TaskbarProgressState::Indeterminate
        }
        DownloadStatus::Downloading
        | DownloadStatus::Finalizing
        | DownloadStatus::Verifying
        | DownloadStatus::Probing
        | DownloadStatus::Waiting => {
            if total.is_some() {
                TaskbarProgressState::Normal
            } else {
                TaskbarProgressState::Indeterminate
            }
        }
        DownloadStatus::Paused => TaskbarProgressState::Paused,
        DownloadStatus::Failed => TaskbarProgressState::Error,
        DownloadStatus::Completed | DownloadStatus::Cancelled => TaskbarProgressState::None,
        DownloadStatus::Created => TaskbarProgressState::Indeterminate,
    };
    let (completed, total) = match state {
        TaskbarProgressState::Normal | TaskbarProgressState::Paused => {
            let total = total.unwrap_or(1).max(1);
            (downloaded.min(total), total)
        }
        TaskbarProgressState::Error => {
            let total = total.unwrap_or(1).max(1);
            (downloaded.min(total).max(1), total)
        }
        TaskbarProgressState::None | TaskbarProgressState::Indeterminate => (0, 0),
    };
    SetTaskbarProgressCommand {
        window_handle,
        state,
        completed,
        total,
    }
}

fn taskbar_overlay_placement_command(
    window_handle: i64,
    settings: &DownloadTaskbarProgressOverlaySettings,
    visible: bool,
    interactive: bool,
) -> PlaceTaskbarProgressOverlayCommand {
    PlaceTaskbarProgressOverlayCommand {
        window_handle,
        enabled: settings.enabled,
        visible,
        anchor: match settings.anchor {
            DownloadTaskbarOverlayAnchor::Left => TaskbarOverlayAnchor::Left,
            DownloadTaskbarOverlayAnchor::Right => TaskbarOverlayAnchor::Right,
        },
        offset_x: settings.offset_x,
        offset_y: settings.offset_y,
        width: i32::try_from(settings.width).unwrap_or(640),
        height: i32::try_from(settings.height).unwrap_or(72),
        interactive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_search_matches_name_directory_and_extension_without_unbounded_metadata() {
        let category = DownloadCategorySettings {
            id: "archives".into(),
            name: "压缩包".into(),
            enabled: true,
            save_directory: r"Downloads\Archive".into(),
            extensions: vec!["zip".into(), "7z".into()],
            url_patterns: Vec::new(),
        };
        assert!(category_matches_query(&category, "压缩"));
        assert!(category_matches_query(&category, "archive"));
        assert!(category_matches_query(&category, "7z"));
        assert!(!category_matches_query(&category, "video"));
    }

    #[test]
    fn category_keyboard_highlight_is_visually_distinct_from_committed_selection() {
        let theme = Theme::zed_dark();

        assert_eq!(
            category_choice_background(theme, true, false),
            theme.accent_soft
        );
        assert_eq!(
            category_choice_background(theme, false, true),
            theme.element_selected
        );
        assert_ne!(
            category_choice_background(theme, true, false),
            category_choice_background(theme, false, true),
            "the committed selection and the single keyboard highlight need distinct surfaces"
        );
    }

    use gpui::{AppContext, Modifiers, TestAppContext, VisualTestContext};

    #[test]
    fn multiline_header_placeholder_stays_on_one_line_for_windows_directwrite() {
        assert!(!DEFAULT_REQUEST_HEADERS_PLACEHOLDER.contains(['\r', '\n']));
    }

    #[gpui::test]
    fn download_task_icons_use_the_typed_shell_boundary_and_publish_by_task_id(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory, platform_endpoint) =
            download_application_for_window_test_with_platform(cx, "task-shell-icons");
        platform_endpoint.set_connected_for_contract_test(true);
        let (task_id, sibling_task_id) = cx.update(|cx| {
            application.update(cx, |application, cx| {
                let task_id = application
                    .engine
                    .enqueue(
                        DownloadRequest {
                            source_url: "https://example.test/archive.zip".into(),
                            save_directory: directory.clone(),
                            save_directory_explicit: false,
                            file_name: Some("archive.zip".into()),
                            headers: Vec::new(),
                            cookie: None,
                            referrer: None,
                            certificate_policy: Default::default(),
                            connection_limit: Some(1),
                            speed_limit_bytes_per_second: None,
                            expected_total_bytes: None,
                            scheduled_at_unix_ms: None,
                            adaptive_segment_policy: None,
                            media: None,
                            category_id: None,
                            category_name: None,
                            duplicate_strategy:
                                nxtools_download::DownloadDuplicateStrategy::UseDefault,
                        },
                        RequestOrigin::Internal,
                    )
                    .expect("enqueue icon fixture task");
                let sibling_task_id = application
                    .engine
                    .enqueue(
                        DownloadRequest {
                            source_url: "https://example.test/second.zip".into(),
                            save_directory: directory.clone(),
                            save_directory_explicit: false,
                            file_name: Some("second.zip".into()),
                            headers: Vec::new(),
                            cookie: None,
                            referrer: None,
                            certificate_policy: Default::default(),
                            connection_limit: Some(1),
                            speed_limit_bytes_per_second: None,
                            expected_total_bytes: None,
                            scheduled_at_unix_ms: None,
                            adaptive_segment_policy: None,
                            media: None,
                            category_id: None,
                            category_name: None,
                            duplicate_strategy:
                                nxtools_download::DownloadDuplicateStrategy::UseDefault,
                        },
                        RequestOrigin::Internal,
                    )
                    .expect("enqueue sibling icon fixture task");
                application.refresh_snapshot(false, cx);
                (task_id, sibling_task_id)
            })
        });

        let envelope = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("typed Shell icon command");
        match &envelope.message {
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::ExtractShellIcon(
                ExtractShellIconCommand { target, size },
            )) => {
                assert_eq!(target, "archive.zip");
                assert_eq!(*size, DOWNLOAD_TASK_ICON_SIZE);
            }
            message => panic!("unexpected platform message: {message:?}"),
        }
        assert!(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .is_err(),
            "tasks with the same extension must share one bounded Shell request"
        );
        let icon_path = directory.join("archive-icon.png");
        std::fs::write(&icon_path, crate::icons::ONE_PIXEL_PNG).expect("write Shell icon result");
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: envelope.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(icon_path.to_string_lossy().into_owned()),
                        },
                    },
                    cx,
                );
            });
        });
        assert_eq!(
            cx.read(|cx| {
                application
                    .read(cx)
                    .task_icon_path(task_id)
                    .map(Path::to_path_buf)
            }),
            Some(icon_path.clone())
        );
        assert_eq!(
            cx.read(|cx| {
                application
                    .read(cx)
                    .task_icon_path(sibling_task_id)
                    .map(Path::to_path_buf)
            }),
            Some(icon_path)
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn live_progress_updates_the_cached_selected_segment_without_requerying_sqlite() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/live.bin".into(),
                    save_directory: std::env::temp_dir(),
                    save_directory_explicit: false,
                    file_name: Some("live.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(2),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue task");
        let task = engine
            .task(task_id)
            .expect("read task")
            .expect("task exists");
        let mut detail = DownloadTaskDetail {
            task,
            segments: vec![
                DownloadSegment::new(0, 0, 9, 0, DownloadSegmentState::Waiting).expect("segment"),
            ],
            timeline: Vec::new(),
        };
        let progress = DownloadProgress {
            task_id,
            downloaded_bytes: 6,
            total_bytes: Some(10),
            bytes_per_second: 1_024,
            active_connections: 1,
            segments: vec![nxtools_download::DownloadSegmentProgress {
                segment_index: 0,
                downloaded_bytes: 6,
                total_bytes: Some(10),
            }],
        };

        apply_progress_to_task_detail(&mut detail, &progress);

        assert_eq!(detail.task.downloaded_bytes, 6);
        assert_eq!(detail.segments[0].downloaded_bytes(), 6);
        assert_eq!(detail.segments[0].state, DownloadSegmentState::Active);
    }

    #[test]
    fn start_now_closes_the_add_window_before_scheduling_the_progress_window() {
        let task_id = DownloadTaskId::new();

        assert_eq!(
            submit_presentation_plan(DownloadSubmitMode::StartNow, task_id, false, true),
            DownloadSubmitPresentationPlan {
                close_source_window: true,
                deferred_task_window: Some(task_id),
            }
        );
    }

    #[test]
    fn disabled_automatic_progress_window_still_closes_the_add_window() {
        let task_id = DownloadTaskId::new();

        assert_eq!(
            submit_presentation_plan(DownloadSubmitMode::StartNow, task_id, false, false),
            DownloadSubmitPresentationPlan {
                close_source_window: true,
                deferred_task_window: None,
            }
        );
    }

    #[test]
    fn download_settings_validation_uses_real_engine_ranges() {
        let draft = parse_download_settings_draft(DownloadSettingsInput {
            default_directory: r"C:\Downloads",
            connection_limit: "8",
            adaptive_segment_scaling_enabled: true,
            adaptive_segment_max_segments: "32",
            adaptive_segment_min_mib: "3",
            max_concurrent_tasks: "3",
            speed_limit_kib: "2048",
            show_progress_window: true,
            show_completion_window: false,
            completion_sound_enabled: true,
            open_directory_on_completion: false,
            taskbar_overlay_enabled: true,
            taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor::Right,
            taskbar_overlay_offset_x: "12",
            taskbar_overlay_offset_y: "0",
            taskbar_overlay_width: "132",
            taskbar_overlay_height: "8",
            taskbar_overlay_progress_mode: DownloadTaskbarProgressMode::PerTask,
            taskbar_overlay_show_progress: true,
            taskbar_overlay_show_speed: false,
            taskbar_overlay_show_remaining_time: false,
            taskbar_overlay_show_active_tasks: false,
            taskbar_overlay_show_active_connections: false,
            taskbar_overlay_show_transferred_bytes: false,
        })
        .expect("valid download settings");

        assert_eq!(draft.default_connection_limit, 8);
        assert_eq!(draft.max_concurrent_tasks, 3);
        assert_eq!(
            draft.global_speed_limit_bytes_per_second,
            Some(2 * 1024 * 1024)
        );
        assert!(draft.show_progress_window);
        assert!(!draft.show_completion_window);
        assert!(draft.completion_sound_enabled);
        assert!(!draft.open_directory_on_completion);
        assert_eq!(draft.taskbar_progress_overlay.width, 132);
        assert_eq!(draft.taskbar_progress_overlay.height, 8);
        assert_eq!(
            draft.taskbar_progress_overlay.progress_mode,
            DownloadTaskbarProgressMode::PerTask
        );
        assert!(
            parse_download_settings_draft(DownloadSettingsInput {
                default_directory: r"C:\Downloads",
                connection_limit: "65",
                adaptive_segment_scaling_enabled: true,
                adaptive_segment_max_segments: "65",
                adaptive_segment_min_mib: "3",
                max_concurrent_tasks: "3",
                speed_limit_kib: "0",
                show_progress_window: true,
                show_completion_window: true,
                completion_sound_enabled: true,
                open_directory_on_completion: false,
                taskbar_overlay_enabled: true,
                taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor::Right,
                taskbar_overlay_offset_x: "12",
                taskbar_overlay_offset_y: "0",
                taskbar_overlay_width: "132",
                taskbar_overlay_height: "8",
                taskbar_overlay_progress_mode: DownloadTaskbarProgressMode::PerTask,
                taskbar_overlay_show_progress: true,
                taskbar_overlay_show_speed: false,
                taskbar_overlay_show_remaining_time: false,
                taskbar_overlay_show_active_tasks: false,
                taskbar_overlay_show_active_connections: false,
                taskbar_overlay_show_transferred_bytes: false,
            })
            .is_err()
        );
        assert!(
            parse_download_settings_draft(DownloadSettingsInput {
                default_directory: r"C:\Downloads",
                connection_limit: "8",
                adaptive_segment_scaling_enabled: true,
                adaptive_segment_max_segments: "32",
                adaptive_segment_min_mib: "3",
                max_concurrent_tasks: "0",
                speed_limit_kib: "0",
                show_progress_window: true,
                show_completion_window: true,
                completion_sound_enabled: true,
                open_directory_on_completion: false,
                taskbar_overlay_enabled: true,
                taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor::Right,
                taskbar_overlay_offset_x: "12",
                taskbar_overlay_offset_y: "0",
                taskbar_overlay_width: "132",
                taskbar_overlay_height: "8",
                taskbar_overlay_progress_mode: DownloadTaskbarProgressMode::PerTask,
                taskbar_overlay_show_progress: true,
                taskbar_overlay_show_speed: false,
                taskbar_overlay_show_remaining_time: false,
                taskbar_overlay_show_active_tasks: false,
                taskbar_overlay_show_active_connections: false,
                taskbar_overlay_show_transferred_bytes: false,
            })
            .is_err()
        );
        assert!(
            parse_download_settings_draft(DownloadSettingsInput {
                default_directory: r"C:\Downloads",
                connection_limit: "8",
                adaptive_segment_scaling_enabled: true,
                adaptive_segment_max_segments: "7",
                adaptive_segment_min_mib: "3",
                max_concurrent_tasks: "3",
                speed_limit_kib: "0",
                show_progress_window: true,
                show_completion_window: true,
                completion_sound_enabled: true,
                open_directory_on_completion: false,
                taskbar_overlay_enabled: true,
                taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor::Right,
                taskbar_overlay_offset_x: "12",
                taskbar_overlay_offset_y: "0",
                taskbar_overlay_width: "132",
                taskbar_overlay_height: "8",
                taskbar_overlay_progress_mode: DownloadTaskbarProgressMode::PerTask,
                taskbar_overlay_show_progress: true,
                taskbar_overlay_show_speed: false,
                taskbar_overlay_show_remaining_time: false,
                taskbar_overlay_show_active_tasks: false,
                taskbar_overlay_show_active_connections: false,
                taskbar_overlay_show_transferred_bytes: false,
            })
            .is_err(),
            "adaptive segment ceiling cannot be below the initial connection count"
        );
        assert!(
            parse_download_settings_draft(DownloadSettingsInput {
                default_directory: r"C:\Downloads",
                connection_limit: "8",
                adaptive_segment_scaling_enabled: true,
                adaptive_segment_max_segments: "32",
                adaptive_segment_min_mib: "0",
                max_concurrent_tasks: "3",
                speed_limit_kib: "0",
                show_progress_window: true,
                show_completion_window: true,
                completion_sound_enabled: true,
                open_directory_on_completion: false,
                taskbar_overlay_enabled: true,
                taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor::Right,
                taskbar_overlay_offset_x: "12",
                taskbar_overlay_offset_y: "0",
                taskbar_overlay_width: "132",
                taskbar_overlay_height: "8",
                taskbar_overlay_progress_mode: DownloadTaskbarProgressMode::PerTask,
                taskbar_overlay_show_progress: true,
                taskbar_overlay_show_speed: false,
                taskbar_overlay_show_remaining_time: false,
                taskbar_overlay_show_active_tasks: false,
                taskbar_overlay_show_active_connections: false,
                taskbar_overlay_show_transferred_bytes: false,
            })
            .is_err(),
            "adaptive split tail threshold cannot be zero"
        );
    }

    fn unique_download_settings_test_directory(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "nxtools-download-settings-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ))
    }

    #[gpui::test]
    fn taskbar_overlay_window_opens_without_reentering_download_application(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) = download_application_for_window_test(cx, "taskbar-overlay");

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_taskbar_overlay(cx);
            });
        });

        assert!(cx.read(|cx| {
            application
                .read(cx)
                .taskbar_overlay_window
                .as_ref()
                .is_some()
        }));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn stale_native_taskbar_overlay_close_cannot_cancel_the_current_edit(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "taskbar-overlay-stale-close");
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_taskbar_overlay(cx);
                application.open_schedule_window(cx);
            });
        });
        cx.run_until_parked();
        let (overlay_window, stale_window) = cx.read(|cx| {
            let application = application.read(cx);
            (
                application
                    .taskbar_overlay_window
                    .expect("current taskbar overlay window"),
                application
                    .schedule_window
                    .expect("stale comparison window"),
            )
        });
        let original_correlation_id = uuid::Uuid::new_v4();
        let replacement_correlation_id = uuid::Uuid::new_v4();
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.taskbar_overlay_edit_draft =
                    Some(DownloadTaskbarProgressOverlaySettings::default());
                application.taskbar_overlay_edit_correlation_id = Some(original_correlation_id);
                assert_eq!(
                    application.taskbar_overlay_native_close_intent(stale_window),
                    None
                );
                let stale_intent = application
                    .taskbar_overlay_native_close_intent(overlay_window)
                    .expect("current edit close intent");
                application.handle_native_taskbar_overlay_close(stale_window, stale_intent, cx);
                assert!(application.taskbar_overlay_edit_draft.is_some());

                application.taskbar_overlay_edit_correlation_id = Some(replacement_correlation_id);
                application.handle_native_taskbar_overlay_close(overlay_window, stale_intent, cx);
                assert!(application.taskbar_overlay_edit_draft.is_some());
                assert_eq!(
                    application.taskbar_overlay_edit_correlation_id,
                    Some(replacement_correlation_id)
                );

                let current_intent = application
                    .taskbar_overlay_native_close_intent(overlay_window)
                    .expect("replacement edit close intent");
                application.handle_native_taskbar_overlay_close(overlay_window, current_intent, cx);
                assert!(application.taskbar_overlay_edit_draft.is_none());
                assert!(application.taskbar_overlay_edit_correlation_id.is_none());
                assert!(application.taskbar_overlay_window == Some(overlay_window));
                application.close_schedule_window(cx);
            });
        });
        let _ = overlay_window.update(cx, |_, window, _| window.remove_window());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn production_taskbar_overlay_keeps_information_geometry_stable_in_edit_mode(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "taskbar-overlay-geometry");
        let settings = DownloadTaskbarProgressOverlaySettings {
            width: 320,
            height: 32,
            progress_mode: DownloadTaskbarProgressMode::Aggregate,
            show_speed: true,
            show_remaining_time: true,
            show_active_tasks: true,
            show_active_connections: true,
            show_transferred_bytes: true,
            ..Default::default()
        };
        let preview_samples = [
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Downloading,
                downloaded_bytes: 314 * 1024 * 1024,
                total_bytes: Some(1024 * 1024 * 1024),
                bytes_per_second: 10 * 1024 * 1024,
                active_connections: 4,
                segments: Vec::new(),
            },
            DownloadTaskbarOverlaySample {
                status: DownloadStatus::Downloading,
                downloaded_bytes: 420 * 1024 * 1024,
                total_bytes: Some(1024 * 1024 * 1024),
                bytes_per_second: 14 * 1024 * 1024,
                active_connections: 4,
                segments: Vec::new(),
            },
        ];
        let normal_model = aggregate_taskbar_progress(&settings, false, preview_samples);
        let platform = cx.read(|cx| Rc::clone(&application.read(cx).platform));
        let application_for_window = application.clone();
        let (_overlay, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(320.0), px(32.0)));
            DownloadTaskbarOverlayView::new(
                application_for_window,
                platform,
                0,
                normal_model,
                window,
                cx,
            )
        });
        window.refresh().expect("render normal taskbar overlay");
        window.run_until_parked();
        let normal_content = window
            .debug_bounds("download-taskbar-overlay-content")
            .expect("normal overlay content frame");
        let normal_metrics = window
            .debug_bounds("download-taskbar-overlay-metrics")
            .expect("normal overlay metric row");
        let normal_progress = window
            .debug_bounds("download-taskbar-overlay-progress")
            .expect("normal overlay progress lanes");
        let normal_transferred = window
            .debug_bounds("download-taskbar-overlay-transferred")
            .expect("normal overlay transferred-byte metric");
        assert_eq!(normal_metrics.size.height, px(LINE_HEIGHT_LABEL));
        assert!(normal_transferred.right() <= normal_metrics.right());
        assert!(
            window
                .debug_bounds("download-taskbar-overlay-edit-border")
                .is_none()
        );
        assert!(
            window
                .debug_bounds("download-taskbar-overlay-resize-handle")
                .is_none()
        );

        window.update(|_, cx| {
            application.update(cx, |application, cx| {
                application.taskbar_overlay_edit_draft = Some(settings);
                cx.notify();
            });
        });
        window.run_until_parked();
        window.refresh().expect("render editable taskbar overlay");
        let edit_content = window
            .debug_bounds("download-taskbar-overlay-content")
            .expect("editable overlay content frame");
        let edit_metrics = window
            .debug_bounds("download-taskbar-overlay-metrics")
            .expect("editable overlay metric row");
        let edit_progress = window
            .debug_bounds("download-taskbar-overlay-progress")
            .expect("editable overlay progress lanes");
        let edit_transferred = window
            .debug_bounds("download-taskbar-overlay-transferred")
            .expect("editable overlay transferred-byte metric");
        assert_eq!(edit_content, normal_content);
        assert_eq!(edit_metrics, normal_metrics);
        assert_eq!(edit_progress, normal_progress);
        assert_eq!(edit_transferred, normal_transferred);
        assert!(
            window
                .debug_bounds("download-taskbar-overlay-edit-border")
                .is_some()
        );
        assert!(
            window
                .debug_bounds("download-taskbar-overlay-resize-handle")
                .is_some()
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn new_download_window_opens_without_reentering_download_application(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "new-download-window");

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_new_download_window(cx);
            });
        });

        assert!(cx.read(|cx| { application.read(cx).new_download_window.as_ref().is_some() }));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn production_new_download_category_form_and_bounded_menu_remain_readable(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "production-category-menu");
        let categories = (0..32)
            .map(|index| DownloadCategorySettings {
                id: format!("category-{index}"),
                name: format!("分类 {index:02}"),
                enabled: true,
                save_directory: format!(
                    r"C:\Users\Administrator\Downloads\Category-{index:02}\A-Long-Readable-Directory"
                ),
                extensions: vec![format!("ext{index}")],
                url_patterns: Vec::new(),
            })
            .collect();
        let (completion, completed) = std::sync::mpsc::sync_channel(1);
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.config.download.categories = categories;
                application.open_browser_download_dialog(
                    BrowserDownloadDialogRequest {
                        correlation_id: uuid::Uuid::new_v4(),
                        handoff_id: uuid::Uuid::new_v4(),
                        source: BrowserDownloadDialogSource::Blob {
                            blob: BrowserBlob {
                                id: uuid::Uuid::new_v4(),
                                file_name: "captured.webm".into(),
                                media_type: Some("video/webm".into()),
                                size: 4096,
                            },
                            save_directory: directory.clone(),
                        },
                        completion,
                    },
                    cx,
                );
            });
        });
        assert_eq!(completed.recv().expect("browser dialog completion"), Ok(()));
        cx.run_until_parked();

        let new_download_window = cx
            .read(|cx| application.read(cx).new_download_window)
            .expect("production new-download window");
        let actual_size = new_download_window
            .update(cx, |_, window, _| window.bounds().size)
            .expect("read production new-download bounds");
        assert_eq!(actual_size, new_download_window_size(true));
        let mut window = VisualTestContext::from_window(new_download_window, cx);
        window.run_until_parked();
        window
            .refresh()
            .expect("render production new-download form");

        let file_name_field = window
            .debug_bounds("download-new-file-name-field")
            .expect("production file-name field");
        let category_field = window
            .debug_bounds("download-new-category-field")
            .expect("production category field");
        assert!(
            (file_name_field.top().as_f32() - category_field.top().as_f32()).abs() <= 1.0
                && (file_name_field.bottom().as_f32() - category_field.bottom().as_f32()).abs()
                    <= 1.0
                && (file_name_field.size.width.as_f32() - category_field.size.width.as_f32()).abs()
                    <= 2.0,
            "production file-name and category fields must share one equal-width form row"
        );
        let url_label = window
            .debug_bounds("download-new-url-field-label")
            .expect("production URL label");
        let url_control = window
            .debug_bounds("download-new-url-field")
            .expect("production URL control");
        let form_grid = window
            .debug_bounds("download-new-form-grid")
            .expect("production new-download form grid");
        let directory_label = window
            .debug_bounds("download-new-directory-field-label")
            .expect("production directory label");
        let directory_row = window
            .debug_bounds("download-new-directory-control")
            .expect("production directory path row");
        let directory_control = window
            .debug_bounds("download-new-directory-input")
            .expect("production directory control");
        let directory_picker = window
            .debug_bounds("download-new-directory-picker")
            .expect("production directory picker");
        let file_name_label = window
            .debug_bounds("download-new-file-name-control-label")
            .expect("production file-name label");
        let file_name_control = window
            .debug_bounds("download-new-file-name-control")
            .expect("production file-name control");
        let category_label = window
            .debug_bounds("download-new-category-control-label")
            .expect("production category label");
        let category_trigger = window
            .debug_bounds("download-new-category-trigger")
            .expect("production category trigger");
        for (name, label) in [
            ("URL", url_label),
            ("directory", directory_label),
            ("file name", file_name_label),
            ("category", category_label),
        ] {
            assert_eq!(
                label.size.height,
                px(LINE_HEIGHT_LABEL),
                "{name} label must use the shared Label line height"
            );
        }
        for (name, control) in [
            ("URL", url_control),
            ("directory", directory_control),
            ("file name", file_name_control),
            ("category", category_trigger),
        ] {
            assert_eq!(
                control.size.height,
                px(CONTROL_HEIGHT_INPUT),
                "{name} control must use the shared 32 px input tier"
            );
        }
        assert_eq!(directory_picker.size, gpui::size(px(28.0), px(28.0)));
        assert!(
            (url_control.left().as_f32() - (form_grid.left().as_f32() + 16.0)).abs() <= 1.0
                && (url_control.right().as_f32() - (form_grid.right().as_f32() - 16.0)).abs()
                    <= 1.0,
            "URL control must fill the padded form width"
        );
        assert!(
            (directory_row.left().as_f32() - file_name_field.left().as_f32()).abs() <= 1.0
                && (directory_row.right().as_f32() - category_field.right().as_f32()).abs() <= 1.0,
            "directory path row must fill the file/category form column"
        );
        assert!(
            (file_name_label.top().as_f32() - category_label.top().as_f32()).abs() <= 1.0
                && (file_name_control.top().as_f32() - category_trigger.top().as_f32()).abs()
                    <= 1.0,
            "file-name and category visible labels and controls must share baselines"
        );
        window.simulate_click(category_trigger.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render production category menu");
        let popover = window
            .debug_bounds("download-new-category-popover-content")
            .expect("bounded production category menu");
        assert!(
            popover.size.height.as_f32() <= 248.0,
            "32 categories must stay in a compact bounded menu, got {}px",
            popover.size.height.as_f32()
        );
        let search = window
            .debug_bounds("download-new-category-search")
            .expect("bounded category menu search field");
        let list = window
            .debug_bounds("download-new-category-list")
            .expect("shared bounded category ListBox");
        assert!(search.top() >= popover.top() && search.bottom() < list.top());
        assert!(
            list.bottom() <= popover.bottom(),
            "ListBox must stay inside the shared Popover: list={list:?}, popover={popover:?}"
        );
        assert!(
            window
                .debug_bounds("download-new-category-next-page")
                .is_none(),
            "the searchable ListBox must replace the seven-page control"
        );
        assert!(
            cx.read(|cx| {
                application
                    .read(cx)
                    .new_download_view
                    .as_ref()
                    .expect("production new-download view")
                    .read(cx)
                    .category_scroll
                    .max_offset()
                    .height
                    > px(0.0)
            }),
            "all 32 categories must share one genuinely scrollable ListBox"
        );
        let first_directory = window
            .debug_bounds("download-new-category-option-directory-0")
            .expect("first category directory summary");
        let first_category = window
            .debug_bounds("download-new-category-option-0")
            .expect("first category option");
        let first_title = window
            .debug_bounds("download-new-category-option-title-0")
            .expect("first category name line");
        assert_eq!(first_title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(first_directory.size.height, px(LINE_HEIGHT_LABEL));
        let category_content_left = first_category.left().as_f32() + SPACING_8;
        let category_content_right = first_category.right().as_f32() - SPACING_8;
        assert!(
            first_title.top() >= first_category.top()
                && first_title.bottom() <= first_directory.top()
                && first_directory.bottom() <= first_category.bottom()
                && (first_title.left().as_f32() - category_content_left).abs() <= 1.0
                && (first_title.right().as_f32() - category_content_right).abs() <= 1.0
                && (first_directory.left().as_f32() - category_content_left).abs() <= 1.0
                && (first_directory.right().as_f32() - category_content_right).abs() <= 1.0,
            "category name and directory must remain two bounded rows"
        );
        let executor = cx.executor();
        window.simulate_mouse_move(first_category.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(600));
        window.run_until_parked();
        window
            .refresh()
            .expect("render complete category-directory tooltip");
        assert!(
            window
                .debug_bounds("download-new-category-option-tooltip-0")
                .is_some(),
            "the complete category directory must remain readable from a stable tooltip"
        );
        let titlebar = window
            .debug_bounds("download-new-titlebar")
            .expect("production new-download titlebar");
        let outside_popover = gpui::point(titlebar.left() + px(24.0), titlebar.center().y);
        assert!(
            !popover.contains(&outside_popover),
            "the outside-click fixture must remain outside the production Popover"
        );
        window.simulate_click(outside_popover, Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("dismiss category menu through overlay");
        assert!(
            !cx.read(|cx| {
                application
                    .read(cx)
                    .new_download_view
                    .as_ref()
                    .expect("production new-download view")
                    .read(cx)
                    .category_popover_open
            }),
            "clicking outside must close the controlled category Popover"
        );
        window.update(|window, cx| {
            let view = application
                .read(cx)
                .new_download_view
                .as_ref()
                .expect("production new-download view");
            assert!(view.read(cx).file_name.focus_handle(cx).is_focused(window));
        });
        window.simulate_click(category_trigger.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("reopen category menu after overlay close");
        window.simulate_keystrokes("escape");
        assert!(
            !cx.read(|cx| {
                application
                    .read(cx)
                    .new_download_view
                    .as_ref()
                    .expect("production new-download view")
                    .read(cx)
                    .category_popover_open
            }),
            "Escape must close the controlled category Popover"
        );
        window.update(|window, cx| {
            let view = application
                .read(cx)
                .new_download_view
                .as_ref()
                .expect("production new-download view");
            assert!(view.read(cx).file_name.focus_handle(cx).is_focused(window));
        });
        window.simulate_click(category_trigger.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("reopen searchable category menu");
        window.simulate_input("分类 3");
        window.run_until_parked();
        window.refresh().expect("filter category ListBox");
        let last_category = window
            .debug_bounds("download-new-category-option-31")
            .expect("keyboard search reaches the final category");
        assert!(
            last_category.top() >= list.top() && last_category.bottom() <= list.bottom(),
            "the filtered final category must remain inside the ListBox: last={last_category:?}, list={list:?}"
        );
        window.simulate_keystrokes("down enter");
        window.run_until_parked();
        window
            .refresh()
            .expect("select searched production category");
        assert_eq!(
            cx.read(|cx| {
                application
                    .read(cx)
                    .new_download_view
                    .as_ref()
                    .expect("production new-download view")
                    .read(cx)
                    .selected_category_id
                    .clone()
            }),
            Some("category-31".into()),
            "keyboard search and Enter must select the final production category"
        );
        assert!(
            !cx.read(|cx| {
                application
                    .read(cx)
                    .new_download_view
                    .as_ref()
                    .expect("production new-download view")
                    .read(cx)
                    .category_popover_open
            }),
            "selection must close the controlled Popover"
        );
        window.update(|window, cx| {
            let view = application
                .read(cx)
                .new_download_view
                .as_ref()
                .expect("production new-download view");
            assert!(view.read(cx).file_name.focus_handle(cx).is_focused(window));
        });

        let cancel = window
            .debug_bounds("download-new-cancel-action")
            .expect("production new-download cancel action");
        window.simulate_click(cancel.center(), Modifiers::default());
        window.run_until_parked();
        assert!(
            new_download_window.update(cx, |_, _, _| {}).is_err(),
            "clicking the production cancel action must close the native GPUI window"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn production_delayed_start_uses_shared_popover_and_preserves_draft_lifecycle(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "production-delayed-start-popover");
        let (completion, completed) = std::sync::mpsc::sync_channel(1);
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_browser_download_dialog(
                    BrowserDownloadDialogRequest {
                        correlation_id: uuid::Uuid::new_v4(),
                        handoff_id: uuid::Uuid::new_v4(),
                        source: BrowserDownloadDialogSource::Blob {
                            blob: BrowserBlob {
                                id: uuid::Uuid::new_v4(),
                                file_name: "scheduled.webm".into(),
                                media_type: Some("video/webm".into()),
                                size: 4096,
                            },
                            save_directory: directory.clone(),
                        },
                        completion,
                    },
                    cx,
                );
            });
        });
        assert_eq!(completed.recv().expect("browser dialog completion"), Ok(()));
        cx.run_until_parked();

        let new_download_window = cx
            .read(|cx| application.read(cx).new_download_window)
            .expect("production new-download window");
        let view = cx
            .read(|cx| application.read(cx).new_download_view.clone())
            .expect("production new-download view");
        let mut window = VisualTestContext::from_window(new_download_window, cx);
        window.run_until_parked();
        window
            .refresh()
            .expect("render production delayed-start form");

        let schedule = window
            .debug_bounds("download-new-schedule-control")
            .expect("production delayed-start action");
        window.simulate_click(schedule.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render shared schedule Popover");

        let shell = window
            .debug_bounds("download-new-schedule-popover-shell")
            .expect("shared schedule Popover shell");
        let content = window
            .debug_bounds("download-new-schedule-popover-content")
            .expect("schedule Popover content");
        assert!(
            content.left() >= shell.left()
                && content.right() <= shell.right()
                && content.top() >= shell.top()
                && content.bottom() <= shell.bottom(),
            "schedule content must remain inside the shared Popover shell"
        );
        assert!(
            shell.size.width.as_f32() <= 304.0 && shell.size.height.as_f32() <= 240.0,
            "schedule Popover must stay bounded at 304x240, got {:?}",
            shell.size
        );
        for (name, selector) in [
            (
                "previous month",
                "download-new-schedule-previous-month-action",
            ),
            ("next month", "download-new-schedule-next-month-action"),
            ("clear", "download-new-schedule-clear-action"),
            ("confirm", "download-new-schedule-confirm"),
        ] {
            let action = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing shared schedule {name} action"));
            assert!(
                (28.0..=30.0).contains(&action.size.height.as_f32()),
                "schedule {name} must use the shared 28px tier, got {}px",
                action.size.height.as_f32()
            );
        }
        let initial_month = window.read(|cx| view.read(cx).scheduled_month);
        let next_month = window
            .debug_bounds("download-new-schedule-next-month-action")
            .expect("production next-month action");
        window.simulate_click(next_month.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("navigate to the next schedule month");
        assert_eq!(
            window.read(|cx| view.read(cx).scheduled_month),
            shift_calendar_month(initial_month, 1),
            "the production next-month action must update the local schedule draft"
        );
        let previous_month = window
            .debug_bounds("download-new-schedule-previous-month-action")
            .expect("production previous-month action");
        window.simulate_click(previous_month.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("navigate back to the original schedule month");
        assert_eq!(
            window.read(|cx| view.read(cx).scheduled_month),
            initial_month,
            "the production previous-month action must restore the local schedule month"
        );

        let confirm = window
            .debug_bounds("download-new-schedule-confirm")
            .expect("schedule confirm action");
        window.simulate_click(confirm.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("commit production delayed start");
        let committed = window
            .read(|cx| view.read(cx).committed_schedule)
            .expect("confirm must commit the production delayed start");
        assert!(!window.read(|cx| view.read(cx).schedule_popover_open));

        window.simulate_click(schedule.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("reopen production delayed start");
        assert_eq!(
            window.read(|cx| view.read(cx).scheduled_date),
            committed.date,
            "opening must snapshot the committed date back into the local draft"
        );
        let draft_date = committed.date + ChronoDuration::days(1);
        let draft_day = window
            .debug_bounds("download-new-schedule-next-day")
            .expect("next production schedule day");
        window.simulate_click(draft_day.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("render modified production schedule draft");
        assert_eq!(window.read(|cx| view.read(cx).scheduled_date), draft_date);
        window.simulate_click(schedule.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("dismiss schedule draft through trigger toggle");
        assert!(!window.read(|cx| view.read(cx).schedule_popover_open));
        assert_eq!(
            window.read(|cx| view.read(cx).committed_schedule),
            Some(committed),
            "trigger-toggle dismissal must not commit the modified draft"
        );

        window.simulate_click(schedule.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("reopen after trigger-toggle dismissal");
        assert_eq!(
            window.read(|cx| view.read(cx).scheduled_date),
            committed.date,
            "reopening after trigger dismissal must restore the committed draft snapshot"
        );
        let draft_day = window
            .debug_bounds("download-new-schedule-next-day")
            .expect("next production schedule day after reopening");
        window.simulate_click(draft_day.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("modify the reopened schedule draft");
        assert_eq!(window.read(|cx| view.read(cx).scheduled_date), draft_date);
        let titlebar = window
            .debug_bounds("download-new-titlebar")
            .expect("production new-download titlebar");
        let outside_popover = gpui::point(titlebar.right() - px(80.0), titlebar.center().y);
        assert!(
            !shell.contains(&outside_popover),
            "the outside-click fixture must remain outside the production schedule Popover"
        );
        window.simulate_click(outside_popover, Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("dismiss production schedule draft");
        assert!(!window.read(|cx| view.read(cx).schedule_popover_open));
        assert_eq!(
            window.read(|cx| view.read(cx).committed_schedule),
            Some(committed),
            "outside dismissal must preserve the last committed schedule"
        );

        window.simulate_click(schedule.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("reopen schedule before clearing");
        let clear = window
            .debug_bounds("download-new-schedule-clear-action")
            .expect("schedule clear action");
        window.simulate_click(clear.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("clear production delayed start");
        assert_eq!(window.read(|cx| view.read(cx).committed_schedule), None);
        assert!(!window.read(|cx| view.read(cx).schedule_popover_open));

        let cancel = window
            .debug_bounds("download-new-cancel-action")
            .expect("production new-download cancel action");
        window.simulate_click(cancel.center(), Modifiers::default());
        window.run_until_parked();
        assert!(new_download_window.update(cx, |_, _, _| {}).is_err());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn browser_download_window_starts_at_compact_probe_height() {
        let request = DownloadRequest {
            source_url: "https://example.test/browser.bin".into(),
            save_directory: PathBuf::from(r"C:\Downloads"),
            save_directory_explicit: false,
            file_name: Some("browser.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(4),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        assert_eq!(
            initial_new_download_window_size(Some(&NewDownloadInitialSource::Download(request))),
            new_download_window_size(false),
            "an unconfirmed browser handoff must open at the compact probe height"
        );
    }

    #[gpui::test]
    fn browser_probe_window_options_allow_the_compact_native_minimum(cx: &mut TestAppContext) {
        let compact = new_download_window_size(false);
        let options =
            cx.update(|cx| new_download_window_options(WindowBounds::centered(compact, cx)));
        let initial = options
            .window_bounds
            .expect("initial native window bounds")
            .get_bounds()
            .size;
        let minimum = options.window_min_size.expect("native minimum window size");

        assert_eq!(initial.width.as_f32(), compact.width.as_f32());
        assert_eq!(initial.height.as_f32(), compact.height.as_f32());
        assert_eq!(minimum.width.as_f32(), compact.width.as_f32());
        assert_eq!(minimum.height.as_f32(), compact.height.as_f32());
    }

    #[gpui::test]
    fn browser_download_window_shrinks_to_compact_height_while_probe_is_unconfirmed(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "browser-probe-window-height");
        let request = DownloadRequest {
            source_url: "http://127.0.0.1:9/browser.bin".into(),
            save_directory: directory.clone(),
            save_directory_explicit: false,
            file_name: Some("browser.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(4),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application
                    .open_new_download_window_with_request(
                        Some(NewDownloadInitialSource::Download(request)),
                        None,
                        cx,
                    )
                    .expect("open browser download window");
            });
        });
        cx.run_until_parked();

        let window = cx.read(|cx| {
            application
                .read(cx)
                .new_download_window
                .expect("browser download window")
        });
        let height = cx.update(|cx| {
            window
                .update(cx, |_, window, _| window.bounds().size.height.as_f32())
                .expect("read browser download window bounds")
        });
        // The auto-probe of the browser handoff URL fails fast (nothing listens
        // on port 9), which surfaces the inline error strip. The dialog must
        // grow by exactly the error reserve instead of squeezing the footer.
        let compact = new_download_window_size(false).height.as_f32();
        assert_eq!(
            height,
            compact + f32::from(NEW_DOWNLOAD_ERROR_RESERVE_HEIGHT),
            "a failed browser probe must reserve the inline error row, not the confirmed form"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn production_probe_summary_uses_narrow_inspector_and_bounded_stage_geometry(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "new-download-probe-summary");

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_new_download_window(cx);
            });
        });
        cx.run_until_parked();

        let new_download_window = cx
            .read(|cx| application.read(cx).new_download_window)
            .expect("production new-download window");
        let compact_size = new_download_window
            .update(cx, |_, window, _| window.bounds().size)
            .expect("read compact new-download bounds");
        assert_eq!(compact_size, new_download_window_size(false));

        let mut window = VisualTestContext::from_window(new_download_window, cx);
        window
            .refresh()
            .expect("render compact new-download probe stage");
        let guidance = window
            .debug_bounds("download-new-probe-guidance")
            .expect("compact inline probe guidance");
        let footer = window
            .debug_bounds("download-new-footer")
            .expect("compact new-download footer");
        assert_eq!(guidance.size.height, px(LINE_HEIGHT_BODY));
        assert!(guidance.bottom() <= footer.top());
        assert!(
            window
                .debug_bounds("download-new-probe-inspector")
                .is_none(),
            "the narrow probe inspector must not reserve space before confirmation"
        );

        let view = cx
            .read(|cx| application.read(cx).new_download_view.clone())
            .expect("production new-download view");
        let probe = DownloadProbe {
            final_url: "https://example.test/archive.bin".into(),
            file_name: Some("archive.bin".into()),
            total_bytes: Some(64 * 1024 * 1024),
            supports_ranges: true,
            content_type: Some("application/octet-stream".into()),
            validator: nxtools_download::ResourceValidator::default(),
            media: None,
        };
        let confirmed_probe_size = new_download_window_size_for_probe(&probe);
        new_download_window
            .update(cx, move |_, native_window, cx| {
                view.update(cx, |view, cx| {
                    view.probe = Some(probe);
                    cx.notify();
                });
                native_window.resize(confirmed_probe_size);
            })
            .expect("confirm production probe stage");
        window.run_until_parked();
        window.refresh().expect("render confirmed probe inspector");

        let confirmed_size = new_download_window
            .update(cx, |_, native_window, _| native_window.bounds().size)
            .expect("read confirmed new-download bounds");
        assert_eq!(confirmed_size, new_download_window_size(true));
        assert_eq!(
            confirmed_size.width.as_f32(),
            compact_size.width.as_f32(),
            "probe confirmation must not change the dialog width"
        );
        assert_eq!(
            confirmed_size.height.as_f32() - compact_size.height.as_f32(),
            102.0,
            "probe confirmation must add only the destination form height"
        );

        let form_grid = window
            .debug_bounds("download-new-form-grid")
            .expect("confirmed new-download form grid");
        let directory_label = window
            .debug_bounds("download-new-directory-field-label")
            .expect("confirmed directory label");
        let directory_row = window
            .debug_bounds("download-new-directory-control")
            .expect("confirmed directory path row");
        let file_name_field = window
            .debug_bounds("download-new-file-name-field")
            .expect("confirmed file-name field");
        let inspector = window
            .debug_bounds("download-new-probe-inspector")
            .expect("confirmed narrow probe inspector");
        assert!(
            inspector.size.width.as_f32() <= 120.0
                && inspector.size.width.as_f32() < form_grid.size.width.as_f32() * 0.25,
            "probe summary must remain a narrow inspector, got {}px inside {}px",
            inspector.size.width.as_f32(),
            form_grid.size.width.as_f32()
        );
        assert!(
            inspector.left() > directory_row.right()
                && inspector.top() <= directory_label.top()
                && inspector.bottom() >= file_name_field.bottom(),
            "probe inspector must occupy the right rail beside the destination fields"
        );
        for (selector, expected_height) in [
            ("download-new-probe-status", LINE_HEIGHT_LABEL),
            ("download-new-probe-size", LINE_HEIGHT_BODY),
            ("download-new-probe-capability", LINE_HEIGHT_META),
        ] {
            let row = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing probe inspector row {selector}"));
            assert_eq!(row.size.height, px(expected_height));
            assert!(
                row.left() >= inspector.left()
                    && row.right() <= inspector.right()
                    && row.top() >= inspector.top()
                    && row.bottom() <= inspector.bottom(),
                "{selector} must stay inside the narrow inspector"
            );
        }
        let confirmed_footer = window
            .debug_bounds("download-new-footer")
            .expect("confirmed new-download footer");
        assert!(inspector.bottom() <= confirmed_footer.top());

        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn probe_error_grows_the_unconfirmed_dialog_and_keeps_the_footer_position(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "new-download-error-reserve");

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_new_download_window(cx);
            });
        });
        cx.run_until_parked();

        let new_download_window = cx
            .read(|cx| application.read(cx).new_download_window)
            .expect("production new-download window");
        let mut window = VisualTestContext::from_window(new_download_window, cx);
        window.refresh().expect("render compact dialog");

        let footer_before = window
            .debug_bounds("download-new-footer")
            .expect("compact footer");
        let view = cx
            .read(|cx| application.read(cx).new_download_view.clone())
            .expect("production new-download view");
        new_download_window
            .update(cx, move |_, window, cx| {
                // An invalid URL is the same begin_probe failure path the user
                // hits; it sets the error and reserves the row.
                view.update(cx, |view, cx| {
                    view.url.update(cx, |input, cx| {
                        input.set_value("not a valid url", window, cx);
                    });
                    view.begin_probe(window, cx);
                });
            })
            .expect("run failing probe");
        window.run_until_parked();
        window.refresh().expect("render error reserve");

        let base = new_download_window_size(false);
        let error_size = new_download_window
            .update(cx, |_, native_window, _| native_window.bounds().size)
            .expect("read error reserve bounds");
        assert_eq!(
            error_size,
            size(base.width, base.height + NEW_DOWNLOAD_ERROR_RESERVE_HEIGHT),
            "a probe error must grow the fixed-height dialog instead of squeezing the footer"
        );
        let footer_after = window
            .debug_bounds("download-new-footer")
            .expect("footer with error");
        assert_eq!(
            footer_after.size, footer_before.size,
            "footer geometry must not change when an error appears"
        );
        assert!(
            window.debug_bounds("download-inline-error").is_some(),
            "the inline error must render inside the grown dialog"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn new_download_footer_maps_exactly_one_action_to_primary_emphasis() {
        let actions = [
            NewDownloadFooterAction::Schedule,
            NewDownloadFooterAction::QueueOnly,
            NewDownloadFooterAction::StartNow,
            NewDownloadFooterAction::Cancel,
        ];
        let primary = actions
            .iter()
            .copied()
            .filter(|action| action.emphasis() == NewDownloadFooterEmphasis::Primary)
            .collect::<Vec<_>>();

        assert_eq!(primary, vec![NewDownloadFooterAction::StartNow]);
        for action in actions {
            let selector = action.role_selector();
            assert_eq!(
                selector.contains("primary"),
                action.emphasis() == NewDownloadFooterEmphasis::Primary,
                "the observable footer role must be derived from the actual emphasis mapping"
            );
        }
    }

    #[gpui::test]
    fn confirmed_new_download_schedule_and_actions_stay_inside_the_footer(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "new-download-layout");
        let application_for_window = application.clone();
        let default_directory = directory.clone();
        let default_connection_limit =
            cx.read(|cx| application.read(cx).default_connection_limit());
        let default_adaptive_segment_policy =
            cx.read(|cx| application.read(cx).default_adaptive_segment_policy());
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let mut categories = DownloadSettings::default().categories;
        categories[0].save_directory =
            r"C:\Users\Administrator\Downloads\Archives\Compressed\LongCategoryDirectory".into();
        let (_root, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                NewDownloadView::new(
                    NewDownloadViewInit {
                        application: application_for_window,
                        default_directory,
                        default_connection_limit,
                        default_adaptive_segment_policy,
                        categories,
                        initial_source: None,
                    },
                    window,
                    cx,
                )
            });
            *opened_view_slot.borrow_mut() = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = view_slot
            .borrow()
            .clone()
            .expect("new download view mounted");
        window
            .refresh()
            .expect("render initial new download window");
        let request = DownloadRequest {
            source_url: "https://example.test/file.bin".into(),
            save_directory: directory.clone(),
            save_directory_explicit: false,
            file_name: Some("file.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(default_connection_limit),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        window.update(|window, cx| {
            let url = view.read(cx).url.clone();
            url.update(cx, |input, cx| {
                input.set_value("https://example.test/file.bin", window, cx);
            });
            view.update(cx, |view, cx| {
                view.confirmed_request = Some(request);
                view.probe = Some(DownloadProbe {
                    final_url: "https://example.test/file.bin".into(),
                    file_name: Some("file.bin".into()),
                    total_bytes: Some(1024),
                    supports_ranges: true,
                    content_type: Some("application/octet-stream".into()),
                    validator: nxtools_download::ResourceValidator::default(),
                    media: None,
                });
                cx.notify();
            });
            window.resize(new_download_window_size(true));
        });
        window
            .refresh()
            .expect("render confirmed new download window");
        window.run_until_parked();
        window
            .refresh()
            .expect("render resized confirmed new download window");

        let file_name_field = window
            .debug_bounds("download-new-file-name-field")
            .expect("new download file-name field");
        let category_field = window
            .debug_bounds("download-new-category-field")
            .expect("new download category field");
        let category_trigger = window
            .debug_bounds("download-new-category-trigger")
            .expect("new download category trigger");
        assert!(
            (file_name_field.top().as_f32() - category_field.top().as_f32()).abs() <= 1.0
                && (file_name_field.bottom().as_f32() - category_field.bottom().as_f32()).abs()
                    <= 1.0,
            "file name and category must share one form-row baseline"
        );
        let field_width_ratio =
            category_field.size.width.as_f32() / file_name_field.size.width.as_f32();
        assert!(
            (0.85..=1.15).contains(&field_width_ratio),
            "category is still disproportionately narrow: file={}px category={}px",
            file_name_field.size.width.as_f32(),
            category_field.size.width.as_f32()
        );
        assert!(
            category_trigger.size.width.as_f32() >= category_field.size.width.as_f32() - 2.0,
            "category trigger must fill its field instead of remaining a tiny button"
        );

        let category = window
            .debug_bounds("download-new-category-control")
            .expect("new download category control");
        window.simulate_click(category.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render category popover");
        let category_popover = window
            .debug_bounds("download-new-category-popover-content")
            .expect("category popover content");
        assert!(
            category_popover.size.width.as_f32() >= 240.0,
            "category popover remains too narrow at {}px",
            category_popover.size.width.as_f32()
        );
        let category_directory = window
            .debug_bounds("download-new-category-option-directory-0")
            .expect("category directory summary row");
        assert!(
            category_directory.size.height.as_f32() >= 9.0,
            "long category directories must retain a readable summary line"
        );
        let first_category = window
            .debug_bounds("download-new-category-option-0")
            .expect("first explicit category option");
        window.simulate_click(first_category.center(), Modifiers::default());
        window.run_until_parked();
        window
            .refresh()
            .expect("render explicit category selection");
        let selected_category_id = window.read(|cx| {
            view.read(cx)
                .categories
                .first()
                .expect("default category exists")
                .id
                .clone()
        });
        assert_eq!(
            window.read(|cx| view.read(cx).selected_category_id.clone()),
            Some(selected_category_id.clone())
        );

        let schedule = window
            .debug_bounds("download-new-schedule-control")
            .expect("schedule control");
        let footer = window
            .debug_bounds("download-new-footer")
            .expect("new download footer");
        let root = window
            .debug_bounds("download-new-root")
            .expect("new download root");
        assert!(
            schedule.top() >= footer.top() && schedule.bottom() <= footer.bottom(),
            "the schedule control must stay inside the fixed footer"
        );
        assert!(
            footer.bottom() <= root.bottom(),
            "the footer must stay inside the new download window"
        );
        let leading_actions = window
            .debug_bounds("download-new-footer-leading-actions")
            .expect("leading footer actions");
        let submission_actions = window
            .debug_bounds("download-new-footer-submission-actions")
            .expect("submission footer actions");
        assert!(
            leading_actions.left() >= footer.left()
                && leading_actions.right() < submission_actions.left()
                && submission_actions.right() <= footer.right(),
            "secondary controls must stay left while submit controls form one right-side group"
        );
        for (name, selector, role_selector) in [
            (
                "schedule",
                "download-new-schedule-control",
                "download-new-footer-secondary-schedule-action",
            ),
            (
                "queue",
                "download-add-to-queue",
                "download-new-footer-secondary-queue-action",
            ),
            (
                "download",
                "download-start-now",
                "download-new-footer-primary-action",
            ),
            (
                "cancel",
                "download-new-cancel-action",
                "download-new-footer-secondary-cancel-action",
            ),
        ] {
            let action = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing {name} footer action"));
            let role = window
                .debug_bounds(role_selector)
                .unwrap_or_else(|| panic!("missing {name} footer action role"));
            assert_eq!(
                action.size.height,
                px(CONTROL_HEIGHT_ACTION),
                "{name} footer action must use the exact shared action tier"
            );
            assert_eq!(role, action, "{name} role must identify the visible action");
        }
        let refresh = window
            .debug_bounds("download-new-refresh-action")
            .expect("refresh footer utility");
        assert_eq!(
            refresh.size,
            size(px(CONTROL_HEIGHT_ACTION), px(CONTROL_HEIGHT_ACTION)),
            "refresh must remain a square shared action"
        );
        window.simulate_click(schedule.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render opened schedule popover");

        let popup = window
            .debug_bounds("download-new-schedule-popover-content")
            .expect("date and time popover");
        let root_after_open = window
            .debug_bounds("download-new-root")
            .expect("new download root after opening schedule");
        let footer_after_open = window
            .debug_bounds("download-new-footer")
            .expect("new download footer after opening schedule");
        assert_eq!(
            root_after_open, root,
            "opening the date-time picker must not resize the download window"
        );
        assert_eq!(
            footer_after_open, footer,
            "opening the date-time picker must not move or resize the footer"
        );
        assert!(
            popup.left() >= root.left()
                && popup.right() <= root.right()
                && popup.top() >= root.top()
                && popup.bottom() <= root.bottom(),
            "the date-time picker must snap inside the download window"
        );
        let confirm = window
            .debug_bounds("download-new-schedule-confirm")
            .expect("schedule confirm action");
        window.simulate_click(confirm.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render committed schedule");
        window.run_until_parked();
        window.refresh().expect("render closed schedule popover");
        let committed = window
            .read(|cx| view.read(cx).committed_schedule)
            .expect("confirming the date-time picker must commit the delayed start");
        assert!(
            !window.read(|cx| view.read(cx).schedule_popover_open),
            "confirming the date-time picker must close the popover"
        );

        let schedule = window
            .debug_bounds("download-new-schedule-control")
            .expect("committed schedule control");
        window.simulate_click(schedule.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("reopen schedule popover");
        window.update(|_, cx| {
            view.update(cx, |view, cx| {
                view.scheduled_date = committed.date + ChronoDuration::days(1);
                cx.notify();
            });
        });
        window.refresh().expect("render modified schedule draft");
        window.simulate_click(
            gpui::point(root.left() + px(3.0), root.top() + px(17.0)),
            Modifiers::default(),
        );
        window.run_until_parked();
        window.refresh().expect("dismiss modified schedule draft");
        assert!(
            !window.read(|cx| view.read(cx).schedule_popover_open),
            "clicking outside must close the date-time picker"
        );
        assert_eq!(
            window.read(|cx| view.read(cx).committed_schedule),
            Some(committed),
            "clicking outside must discard the modified draft"
        );
        let request = window
            .read(|cx| view.read(cx).request(cx))
            .expect("request after discarding a modified schedule draft");
        assert_eq!(
            request.scheduled_at_unix_ms,
            Some(
                committed
                    .resolve_unix_ms(Local::now().timestamp_millis())
                    .expect("committed schedule remains in the future")
            ),
            "submission must continue to use the last confirmed schedule"
        );
        assert_eq!(request.category_id, Some(selected_category_id));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn confirmed_hls_master_submits_the_selected_quality_as_typed_media(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "new-download-hls-quality");
        let application_for_window = application.clone();
        let default_directory = directory.clone();
        let default_connection_limit =
            cx.read(|cx| application.read(cx).default_connection_limit());
        let default_adaptive_segment_policy =
            cx.read(|cx| application.read(cx).default_adaptive_segment_policy());
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let (_root, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                NewDownloadView::new(
                    NewDownloadViewInit {
                        application: application_for_window,
                        default_directory,
                        default_connection_limit,
                        default_adaptive_segment_policy,
                        categories: DownloadSettings::default().categories,
                        initial_source: None,
                    },
                    window,
                    cx,
                )
            });
            *opened_view_slot.borrow_mut() = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = view_slot
            .borrow()
            .clone()
            .expect("new download view mounted");
        let request = DownloadRequest {
            source_url: "https://example.test/master.m3u8".into(),
            save_directory: directory.clone(),
            save_directory_explicit: false,
            file_name: Some("master.m3u8".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(default_connection_limit),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        window.update(|window, cx| {
            let url = view.read(cx).url.clone();
            url.update(cx, |input, cx| {
                input.set_value("https://example.test/master.m3u8", window, cx);
            });
            view.update(cx, |view, cx| {
                view.confirmed_request = Some(request);
                view.probe = Some(DownloadProbe {
                    final_url: "https://example.test/master.m3u8".into(),
                    file_name: Some("master.m3u8".into()),
                    total_bytes: Some(512),
                    supports_ranges: false,
                    content_type: Some("application/vnd.apple.mpegurl".into()),
                    validator: nxtools_download::ResourceValidator::default(),
                    media: Some(nxtools_download::DownloadMediaProbe::Hls(
                        nxtools_download::HlsProbe {
                            is_master: true,
                            variants: vec![
                                nxtools_download::HlsVariantProbe {
                                    playlist_url: "https://example.test/high.m3u8".into(),
                                    bandwidth: 4_200_000,
                                    average_bandwidth: Some(3_900_000),
                                    resolution: Some((1920, 1080)),
                                    frame_rate_milli: Some(60_000),
                                    codecs: "avc1.640028,mp4a.40.2".into(),
                                    audio_group: Some("stereo".into()),
                                    subtitles_group: None,
                                },
                                nxtools_download::HlsVariantProbe {
                                    playlist_url: "https://example.test/low.m3u8".into(),
                                    bandwidth: 800_000,
                                    average_bandwidth: Some(700_000),
                                    resolution: Some((640, 360)),
                                    frame_rate_milli: Some(29_970),
                                    codecs: "avc1.4d401e,mp4a.40.2".into(),
                                    audio_group: Some("stereo".into()),
                                    subtitles_group: None,
                                },
                            ],
                        },
                    )),
                });
                cx.notify();
            });
        });
        window.refresh().expect("render HLS quality choices");
        window.run_until_parked();
        window.refresh().expect("refresh HLS quality choices");

        assert!(window.debug_bounds("download-hls-quality-0").is_some());
        assert!(window.debug_bounds("download-hls-quality-1").is_some());
        let first_row = window
            .debug_bounds("download-hls-quality-0")
            .expect("first HLS quality row");
        let first_radio = window
            .debug_bounds("download-hls-quality-0-radio")
            .expect("first HLS radio control");
        let first_title = window
            .debug_bounds("download-hls-quality-0-title")
            .expect("first HLS quality title");
        let first_detail = window
            .debug_bounds("download-hls-quality-0-detail")
            .expect("first HLS quality detail");
        assert!(first_row.contains(&first_radio.origin));
        assert!(first_row.contains(&first_radio.bottom_right()));
        assert!(first_row.contains(&first_title.origin));
        assert!(first_row.contains(&first_detail.bottom_right()));
        assert_eq!(first_row.size.height, px(SETTINGS_ROW_MIN_HEIGHT));
        assert_eq!(first_radio.size, size(px(14.0), px(14.0)));
        assert_eq!(first_title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(first_detail.size.height, px(LINE_HEIGHT_LABEL));
        assert_eq!(first_title.left(), first_detail.left());
        assert!(first_title.bottom() <= first_detail.top());
        let first_selected_dot = window
            .debug_bounds("download-hls-quality-0-radio-selected")
            .expect("selected HLS radio dot");
        assert_eq!(first_selected_dot.size, size(px(SPACING_6), px(SPACING_6)));
        assert!(
            first_radio.contains(&first_selected_dot.origin)
                && first_radio.contains(&first_selected_dot.bottom_right()),
            "the selected concrete quality must show a filled radio marker"
        );
        assert!(
            window
                .debug_bounds("download-hls-quality-1-radio-selected")
                .is_none(),
            "an unselected concrete quality must remain visually distinct"
        );
        assert!(
            window
                .debug_bounds("download-hls-quality-1-radio-unselected")
                .is_some()
        );
        let lower_quality = window
            .debug_bounds("download-hls-quality-1")
            .expect("lower HLS quality row");
        window.simulate_click(lower_quality.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render changed HLS selection");
        assert_eq!(
            window.read(|cx| view.read(cx).selected_hls_variant),
            Some(1)
        );
        assert!(
            window
                .debug_bounds("download-hls-quality-1-radio-selected")
                .is_some()
        );
        assert!(
            window
                .debug_bounds("download-hls-quality-0-radio-unselected")
                .is_some(),
            "the previous concrete quality must render its current unselected state"
        );
        let add_to_queue = window
            .debug_bounds("download-add-to-queue")
            .expect("enabled HLS queue button");
        window.simulate_click(add_to_queue.center(), Modifiers::default());
        window.run_until_parked();
        cx.run_until_parked();

        let tasks = cx.read(|cx| {
            application
                .read(cx)
                .engine
                .tasks()
                .expect("read submitted tasks")
        });
        assert_eq!(tasks.len(), 1);
        assert_eq!(
            tasks[0].task.request().media,
            Some(nxtools_download::DownloadRequestMedia::Hls(
                nxtools_download::HlsDownloadRequest {
                    playlist_url: "https://example.test/low.m3u8".into(),
                }
            ))
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn hls_variant_without_resolution_is_not_presented_as_automatic_quality() {
        let variant = nxtools_download::HlsVariantProbe {
            playlist_url: "https://example.test/gear4/prog_index.m3u8".into(),
            bandwidth: 1_927_833,
            average_bandwidth: None,
            resolution: None,
            frame_rate_milli: None,
            codecs: "mp4a.40.2, avc1.4d401f".into(),
            audio_group: None,
            subtitles_group: None,
        };

        assert_eq!(hls_variant_primary_label(&variant), "视频流");
        assert_eq!(
            hls_variant_detail_label(&variant),
            "mp4a.40.2, avc1.4d401f · 1.93 Mbps"
        );
    }

    #[test]
    fn media_probe_window_height_tracks_the_visible_choice_rows() {
        let dash_probe = DownloadProbe {
            final_url: "https://example.test/manifest.mpd".into(),
            file_name: Some("manifest.mpd".into()),
            total_bytes: None,
            supports_ranges: false,
            content_type: Some("application/dash+xml".into()),
            validator: nxtools_download::ResourceValidator::default(),
            media: Some(nxtools_download::DownloadMediaProbe::Dash(
                nxtools_download::DashProbe {
                    video_tracks: vec![nxtools_download::DashTrackProbe {
                        representation_id: "video".into(),
                        bandwidth: 5_100_000,
                        resolution: Some((1920, 1080)),
                        codecs: "avc1.64002a".into(),
                        mime_type: "video/mp4".into(),
                        language: None,
                        segment_count: 30,
                        has_initialization: true,
                    }],
                    audio_tracks: vec![nxtools_download::DashTrackProbe {
                        representation_id: "audio".into(),
                        bandwidth: 130_000,
                        resolution: None,
                        codecs: "mp4a.40.2".into(),
                        mime_type: "audio/mp4".into(),
                        language: None,
                        segment_count: 30,
                        has_initialization: true,
                    }],
                },
            )),
        };

        assert_eq!(
            new_download_window_size_for_probe(&dash_probe)
                .height
                .as_f32(),
            333.0,
            "a one-video/one-audio DASH choice should use one visible row, not the maximum scroller height"
        );

        let hls_variant = nxtools_download::HlsVariantProbe {
            playlist_url: "https://example.test/video.m3u8".into(),
            bandwidth: 1_000_000,
            average_bandwidth: None,
            resolution: None,
            frame_rate_milli: None,
            codecs: "mp4a.40.2, avc1.4d401e".into(),
            audio_group: None,
            subtitles_group: None,
        };
        let hls_probe = DownloadProbe {
            final_url: "https://example.test/master.m3u8".into(),
            file_name: Some("master.m3u8".into()),
            total_bytes: None,
            supports_ranges: false,
            content_type: Some("application/vnd.apple.mpegurl".into()),
            validator: nxtools_download::ResourceValidator::default(),
            media: Some(nxtools_download::DownloadMediaProbe::Hls(
                nxtools_download::HlsProbe {
                    is_master: true,
                    variants: vec![hls_variant; 5],
                },
            )),
        };
        assert_eq!(
            new_download_window_size_for_probe(&hls_probe)
                .height
                .as_f32(),
            402.0,
            "five HLS choices should use the bounded three-row viewport without extra footer whitespace"
        );
    }

    #[gpui::test]
    fn dash_probe_renders_track_choices_and_builds_a_typed_selection(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "new-download-dash-tracks");
        let application_for_window = application.clone();
        let default_directory = directory.clone();
        let default_connection_limit =
            cx.read(|cx| application.read(cx).default_connection_limit());
        let default_adaptive_segment_policy =
            cx.read(|cx| application.read(cx).default_adaptive_segment_policy());
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let (_root, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                NewDownloadView::new(
                    NewDownloadViewInit {
                        application: application_for_window,
                        default_directory,
                        default_connection_limit,
                        default_adaptive_segment_policy,
                        categories: DownloadSettings::default().categories,
                        initial_source: None,
                    },
                    window,
                    cx,
                )
            });
            *opened_view_slot.borrow_mut() = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = view_slot
            .borrow()
            .clone()
            .expect("new download view mounted");
        let track = |representation_id: &str, resolution| nxtools_download::DashTrackProbe {
            representation_id: representation_id.into(),
            bandwidth: 2_000_000,
            resolution,
            codecs: "avc1.640028".into(),
            mime_type: "video/mp4".into(),
            language: None,
            segment_count: 2,
            has_initialization: true,
        };
        let dash = nxtools_download::DashProbe {
            video_tracks: vec![
                track("v1080", Some((1920, 1080))),
                track("v720", Some((1280, 720))),
            ],
            audio_tracks: vec![nxtools_download::DashTrackProbe {
                representation_id: "a128".into(),
                bandwidth: 128_000,
                resolution: None,
                codecs: "mp4a.40.2".into(),
                mime_type: "audio/mp4".into(),
                language: Some("zh".into()),
                segment_count: 2,
                has_initialization: true,
            }],
        };
        let request = DownloadRequest {
            source_url: "https://example.test/manifest.mpd".into(),
            save_directory: directory.clone(),
            save_directory_explicit: false,
            file_name: Some("manifest.mp4".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(default_connection_limit),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        window.update(|window, cx| {
            let url = view.read(cx).url.clone();
            url.update(cx, |input, cx| {
                input.set_value("https://example.test/manifest.mpd", window, cx);
            });
            window.resize(size(px(500.0), px(430.0)));
            view.update(cx, |view, cx| {
                view.confirmed_request = Some(request);
                view.probe = Some(DownloadProbe {
                    final_url: "https://example.test/manifest.mpd".into(),
                    file_name: Some("manifest.mpd".into()),
                    total_bytes: None,
                    supports_ranges: false,
                    content_type: Some("application/dash+xml".into()),
                    validator: nxtools_download::ResourceValidator::default(),
                    media: Some(nxtools_download::DownloadMediaProbe::Dash(dash)),
                });
                view.selected_dash_video_track = Some(0);
                view.selected_dash_audio_track = Some(0);
                cx.notify();
            });
        });
        window.refresh().expect("render DASH track choices");
        let first_video_row = window
            .debug_bounds("download-dash-video-track-0")
            .expect("first DASH video track row");
        let first_video_radio = window
            .debug_bounds("download-dash-video-track-0-radio")
            .expect("first DASH video radio control");
        let first_video_title = window
            .debug_bounds("download-dash-video-track-0-title")
            .expect("first DASH video title");
        let first_video_detail = window
            .debug_bounds("download-dash-video-track-0-detail")
            .expect("first DASH video detail");
        assert!(first_video_row.contains(&first_video_radio.origin));
        assert!(first_video_row.contains(&first_video_detail.bottom_right()));
        assert_eq!(first_video_row.size.height, px(SETTINGS_ROW_MIN_HEIGHT));
        assert_eq!(first_video_radio.size, size(px(14.0), px(14.0)));
        assert_eq!(first_video_title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(first_video_detail.size.height, px(LINE_HEIGHT_LABEL));
        assert_eq!(first_video_title.left(), first_video_detail.left());
        assert!(first_video_title.bottom() <= first_video_detail.top());
        let first_video_dot = window
            .debug_bounds("download-dash-video-track-0-radio-selected")
            .expect("selected DASH video radio dot");
        assert_eq!(first_video_dot.size, size(px(SPACING_6), px(SPACING_6)));
        assert!(
            window
                .debug_bounds("download-dash-video-track-1-radio-selected")
                .is_none()
        );
        assert!(
            window
                .debug_bounds("download-dash-video-track-1-radio-unselected")
                .is_some()
        );
        let lower_video = window
            .debug_bounds("download-dash-video-track-1")
            .expect("lower DASH video track row");
        assert!(window.debug_bounds("download-dash-audio-track-0").is_some());
        window.simulate_click(lower_video.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render changed DASH selection");
        assert_eq!(
            window.read(|cx| view.read(cx).selected_dash_video_track),
            Some(1)
        );
        assert!(
            window
                .debug_bounds("download-dash-video-track-1-radio-selected")
                .is_some()
        );
        assert!(
            window
                .debug_bounds("download-dash-video-track-0-radio-unselected")
                .is_some()
        );
        let audio = window
            .debug_bounds("download-dash-audio-track-0")
            .expect("selected DASH audio track row");
        window.simulate_click(audio.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render deselected DASH audio");
        assert_eq!(
            window.read(|cx| view.read(cx).selected_dash_audio_track),
            None
        );
        assert!(
            window
                .debug_bounds("download-dash-audio-track-0-radio-unselected")
                .is_some(),
            "cancelling the optional audio track must render an empty radio"
        );
        let media = window.read(|cx| {
            let view = view.read(cx);
            selected_download_request_media(
                view.probe.as_ref(),
                view.selected_hls_variant,
                view.selected_dash_video_track,
                view.selected_dash_audio_track,
            )
            .expect("build typed DASH request")
        });
        assert_eq!(
            media,
            Some(nxtools_download::DownloadRequestMedia::Dash(
                nxtools_download::DashDownloadRequest {
                    manifest_url: "https://example.test/manifest.mpd".into(),
                    video_representation_id: Some("v720".into()),
                    audio_representation_id: None,
                }
            ))
        );
        let add_to_queue = window
            .debug_bounds("download-add-to-queue")
            .expect("enabled DASH queue button");
        window.simulate_click(add_to_queue.center(), Modifiers::default());
        window.run_until_parked();
        cx.run_until_parked();
        let tasks = cx.read(|cx| {
            application
                .read(cx)
                .engine
                .tasks()
                .expect("read submitted DASH tasks")
        });
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].task.request().media, media);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn browser_prefilled_download_window_preserves_private_request_context(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "browser-prefilled-download");
        let application_for_window = application.clone();
        let initial = DownloadRequest {
            source_url: "https://example.test/private.bin".into(),
            save_directory: directory.clone(),
            save_directory_explicit: false,
            file_name: Some("private.bin".into()),
            headers: vec![nxtools_download::DownloadRequestHeader {
                name: "Authorization".into(),
                value: "Bearer secret".into(),
            }],
            cookie: Some("session=secret".into()),
            referrer: Some("https://example.test/page".into()),
            certificate_policy: Default::default(),
            connection_limit: Some(4),
            speed_limit_bytes_per_second: Some(512_000),
            expected_total_bytes: Some(8_192),
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        let expected = initial.clone();
        let default_directory = directory.clone();
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let (_root, _window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                NewDownloadView::new(
                    NewDownloadViewInit {
                        application: application_for_window,
                        default_directory,
                        default_connection_limit: 4,
                        default_adaptive_segment_policy:
                            nxtools_download::DownloadAdaptiveSegmentPolicy::default(),
                        categories: DownloadSettings::default().categories,
                        initial_source: Some(NewDownloadInitialSource::Download(initial)),
                    },
                    window,
                    cx,
                )
            });
            *opened_view_slot.borrow_mut() = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = view_slot
            .borrow()
            .clone()
            .expect("browser-prefilled view mounted");
        let reconstructed = cx.read(|cx| view.read(cx).request(cx).expect("rebuild request"));

        assert_eq!(reconstructed.source_url, expected.source_url);
        assert_eq!(reconstructed.save_directory, expected.save_directory);
        assert_eq!(reconstructed.file_name, expected.file_name);
        assert_eq!(reconstructed.headers, expected.headers);
        assert_eq!(reconstructed.cookie, expected.cookie);
        assert_eq!(reconstructed.referrer, expected.referrer);
        assert_eq!(
            reconstructed.expected_total_bytes,
            expected.expected_total_bytes
        );
        assert_eq!(
            reconstructed.speed_limit_bytes_per_second,
            expected.speed_limit_bytes_per_second
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn browser_blob_window_is_prefilled_from_staged_metadata_without_network_probe(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "browser-prefilled-blob");
        let application_for_window = application.clone();
        let default_directory = directory.clone();
        let blob_id = uuid::Uuid::new_v4();
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let (_root, _window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                NewDownloadView::new(
                    NewDownloadViewInit {
                        application: application_for_window,
                        default_directory: default_directory.clone(),
                        default_connection_limit: 4,
                        default_adaptive_segment_policy:
                            nxtools_download::DownloadAdaptiveSegmentPolicy::default(),
                        categories: DownloadSettings::default().categories,
                        initial_source: Some(NewDownloadInitialSource::Blob(
                            BrowserBlobDialogContext {
                                correlation_id: uuid::Uuid::new_v4(),
                                handoff_id: uuid::Uuid::new_v4(),
                                blob: BrowserBlob {
                                    id: blob_id,
                                    file_name: "captured.webm".into(),
                                    media_type: Some("video/webm".into()),
                                    size: 4096,
                                },
                                save_directory: default_directory,
                            },
                        )),
                    },
                    window,
                    cx,
                )
            });
            *opened_view_slot.borrow_mut() = Some(view.clone());
            Root::new(view, window, cx)
        });
        let view = view_slot.borrow().clone().expect("Blob view mounted");
        cx.read(|cx| {
            let view = view.read(cx);
            assert_eq!(
                view.external_blob.as_ref().map(|value| value.blob.id),
                Some(blob_id)
            );
            assert_eq!(view.file_name.read(cx).value(), "captured.webm");
            assert_eq!(
                view.directory.read(cx).value(),
                directory.to_string_lossy().as_ref()
            );
            let probe = view.probe.as_ref().expect("Blob metadata is confirmed");
            assert_eq!(probe.total_bytes, Some(4096));
            assert_eq!(probe.content_type.as_deref(), Some("video/webm"));
            assert!(!view.probing);
        });
        let _ = std::fs::remove_dir_all(directory);
    }

    fn download_application_for_window_test(
        cx: &mut TestAppContext,
        label: &str,
    ) -> (Entity<DownloadApplication>, PathBuf) {
        let (application, directory, _platform_endpoint) =
            download_application_for_window_test_with_platform(cx, label);
        (application, directory)
    }

    fn assert_shared_dialog_shell(
        window: &mut VisualTestContext,
        shell_selector: &'static str,
        titlebar_selector: &'static str,
        body_selector: &'static str,
        footer_selector: &'static str,
    ) {
        let shell = window
            .debug_bounds(shell_selector)
            .expect("shared dialog shell");
        let titlebar = window
            .debug_bounds(titlebar_selector)
            .expect("shared dialog titlebar");
        let body = window
            .debug_bounds(body_selector)
            .expect("shared dialog body");
        let footer = window
            .debug_bounds(footer_selector)
            .expect("shared dialog footer");
        assert_eq!(
            titlebar.size.height,
            px(crate::ui::metrics::TITLEBAR_HEIGHT)
        );
        assert_eq!(
            footer.size.height,
            px(crate::ui::metrics::SETTINGS_FOOTER_HEIGHT)
        );
        assert_eq!(titlebar.bottom(), body.top());
        assert_eq!(body.bottom(), footer.top());
        assert_eq!(footer.bottom(), shell.bottom());
    }

    fn download_application_for_window_test_with_platform(
        cx: &mut TestAppContext,
        label: &str,
    ) -> (
        Entity<DownloadApplication>,
        PathBuf,
        nxtools_ipc::PlatformServerEndpoint,
    ) {
        let directory = unique_download_settings_test_directory(label);
        std::fs::create_dir_all(&directory).expect("create download window test directory");
        let engine = DownloadEngine::open_in_memory().expect("open download window test engine");
        let (platform, platform_endpoint) = nxtools_ipc::platform_bridge(16);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let application = cx.update(|cx| {
            cx.new(|cx| {
                DownloadApplication::new(
                    engine,
                    Rc::new(platform),
                    notifications,
                    DownloadRuntimeContext {
                        fallback_default_directory: directory.clone(),
                        config: AppConfig::default(),
                        config_path: directory.join("config.json"),
                        log_directory: directory.clone(),
                        database_path: directory.join("downloads.sqlite3"),
                        ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        (application, directory, platform_endpoint)
    }

    #[gpui::test]
    fn schedule_dialog_uses_shared_shell_and_preserves_cancel_close_apply_lifecycle(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) = download_application_for_window_test(cx, "schedule-window");

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_schedule_window(cx);
            });
        });
        cx.run_until_parked();

        let window_handle = cx
            .read(|cx| application.read(cx).schedule_window)
            .expect("production schedule window");
        let mut window = VisualTestContext::from_window(window_handle, cx);
        window.refresh().expect("render production schedule dialog");
        assert_shared_dialog_shell(
            &mut window,
            "download-schedule-dialog-shell",
            "download-schedule-dialog-titlebar",
            "download-schedule-dialog-body",
            "download-schedule-dialog-footer",
        );
        let cancel = window
            .debug_bounds("download-schedule-secondary-cancel-action")
            .expect("schedule cancel secondary action");
        let apply = window
            .debug_bounds("download-schedule-primary-apply-action")
            .expect("schedule apply primary action");
        assert_eq!(cancel.size.height, px(CONTROL_HEIGHT_ACTION));
        assert_eq!(apply.size.height, px(CONTROL_HEIGHT_ACTION));
        assert!(cancel.right() <= apply.left());
        window.simulate_click(cancel.center(), Modifiers::default());
        window.run_until_parked();
        assert!(window_handle.update(cx, |_, _, _| ()).is_err());
        assert!(cx.read(|cx| application.read(cx).schedule_window.is_none()));

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_schedule_window(cx);
            });
        });
        cx.run_until_parked();
        let native_close_handle = cx
            .read(|cx| application.read(cx).schedule_window)
            .expect("reopened schedule window for native close");
        let mut native_close_window = VisualTestContext::from_window(native_close_handle, cx);
        native_close_window
            .refresh()
            .expect("render schedule draft before native close");
        let start_toggle = native_close_window
            .debug_bounds("download-schedule-start-toggle")
            .expect("schedule start toggle");
        native_close_window.simulate_click(start_toggle.center(), Modifiers::default());
        native_close_window.run_until_parked();
        assert!(native_close_window.simulate_close());
        native_close_window.run_until_parked();
        assert!(cx.read(|cx| {
            let application = application.read(cx);
            application.schedule_window.is_none()
                && application
                    .default_queue()
                    .is_none_or(|queue| queue.schedule.is_none())
        }));

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_schedule_window(cx);
            });
        });
        cx.run_until_parked();
        let apply_handle = cx
            .read(|cx| application.read(cx).schedule_window)
            .expect("reopened schedule window for apply");
        let mut apply_window = VisualTestContext::from_window(apply_handle, cx);
        apply_window
            .refresh()
            .expect("render schedule apply dialog");
        let start_toggle = apply_window
            .debug_bounds("download-schedule-start-toggle")
            .expect("schedule start toggle after discarded draft");
        apply_window.simulate_click(start_toggle.center(), Modifiers::default());
        apply_window.run_until_parked();
        let apply = apply_window
            .debug_bounds("download-schedule-primary-apply-action")
            .expect("schedule apply action after discarded draft");
        apply_window.simulate_click(apply.center(), Modifiers::default());
        apply_window.run_until_parked();
        assert!(apply_handle.update(cx, |_, _, _| ()).is_err());
        assert!(cx.read(|cx| {
            let application = application.read(cx);
            application.schedule_window.is_none()
                && application.default_queue().is_some_and(|queue| {
                    queue
                        .schedule
                        .as_ref()
                        .is_some_and(|schedule| schedule.daily.enabled_start)
                })
        }));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn delete_dialog_uses_shared_danger_semantics_and_closes_only_after_a_decision(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "delete-dialog-shell");
        let task_id = cx.read(|cx| {
            application
                .read(cx)
                .engine
                .enqueue(
                    DownloadRequest {
                        source_url: "https://example.test/delete-dialog.bin".into(),
                        save_directory: directory.clone(),
                        save_directory_explicit: false,
                        file_name: Some("delete-dialog.bin".into()),
                        headers: Vec::new(),
                        cookie: None,
                        referrer: None,
                        certificate_policy: Default::default(),
                        connection_limit: Some(1),
                        speed_limit_bytes_per_second: None,
                        expected_total_bytes: None,
                        scheduled_at_unix_ms: None,
                        adaptive_segment_policy: None,
                        media: None,
                        category_id: None,
                        category_name: None,
                        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                    },
                    RequestOrigin::Internal,
                )
                .expect("enqueue delete-dialog fixture")
        });
        cx.read(|cx| {
            application
                .read(cx)
                .engine
                .cancel(task_id)
                .expect("cancel delete-dialog fixture")
        });
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.refresh(cx);
                application.request_delete_confirmation(task_id, cx);
            });
        });
        cx.run_until_parked();

        let first_window_handle = cx
            .read(|cx| {
                application
                    .read(cx)
                    .delete_confirmation_windows
                    .get(&task_id)
                    .copied()
            })
            .expect("production delete dialog");
        let mut first_window = VisualTestContext::from_window(first_window_handle, cx);
        first_window
            .refresh()
            .expect("render production delete dialog");
        assert_shared_dialog_shell(
            &mut first_window,
            "download-delete-dialog-shell",
            "download-delete-dialog-titlebar",
            "download-delete-dialog-body",
            "download-delete-dialog-footer",
        );
        let cancel = first_window
            .debug_bounds("download-delete-secondary-cancel-action")
            .expect("delete cancel secondary action");
        let confirm = first_window
            .debug_bounds("download-delete-danger-confirm-action")
            .expect("delete danger confirmation action");
        assert_eq!(cancel.size.height, px(CONTROL_HEIGHT_ACTION));
        assert_eq!(confirm.size.height, px(CONTROL_HEIGHT_ACTION));
        assert!(cancel.right() <= confirm.left());
        first_window.simulate_click(cancel.center(), Modifiers::default());
        first_window.run_until_parked();
        assert!(first_window_handle.update(cx, |_, _, _| ()).is_err());
        assert!(cx.read(|cx| {
            let application = application.read(cx);
            !application
                .delete_confirmation_windows
                .contains_key(&task_id)
                && application.task(task_id).is_some()
        }));

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.request_delete_confirmation(task_id, cx);
            });
        });
        cx.run_until_parked();
        let native_close_handle = cx
            .read(|cx| {
                application
                    .read(cx)
                    .delete_confirmation_windows
                    .get(&task_id)
                    .copied()
            })
            .expect("reopened delete dialog for native close");
        let mut native_close_window = VisualTestContext::from_window(native_close_handle, cx);
        native_close_window
            .refresh()
            .expect("render delete dialog before native close");
        assert!(native_close_window.simulate_close());
        native_close_window.run_until_parked();
        assert!(cx.read(|cx| {
            let application = application.read(cx);
            !application
                .delete_confirmation_windows
                .contains_key(&task_id)
                && application.task(task_id).is_some()
        }));

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.request_delete_confirmation(task_id, cx);
            });
        });
        cx.run_until_parked();
        let confirm_window_handle = cx
            .read(|cx| {
                application
                    .read(cx)
                    .delete_confirmation_windows
                    .get(&task_id)
                    .copied()
            })
            .expect("reopened production delete dialog for confirmation");
        let mut confirm_window = VisualTestContext::from_window(confirm_window_handle, cx);
        confirm_window
            .refresh()
            .expect("render confirmed delete dialog");
        let confirm = confirm_window
            .debug_bounds("download-delete-danger-confirm-action")
            .expect("reopened delete confirmation action");
        confirm_window.simulate_click(confirm.center(), Modifiers::default());
        confirm_window.run_until_parked();
        assert!(confirm_window_handle.update(cx, |_, _, _| ()).is_err());
        assert!(cx.read(|cx| {
            let application = application.read(cx);
            !application
                .delete_confirmation_windows
                .contains_key(&task_id)
                && application.task(task_id).is_none()
        }));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn settings_general_page_mounts_the_double_click_action_control(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "download-double-click-control");

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_settings_window(cx);
            });
        });
        cx.run_until_parked();

        let view = cx
            .read(|cx| application.read(cx).settings_view.clone())
            .expect("settings view handle");
        let carries_state = view.update(cx, |view, _| {
            view.double_click_action == DownloadRowDoubleClickActionSettings::OpenFile
        });
        assert!(
            carries_state,
            "settings view must carry the double-click state"
        );
        let general_source = include_str!("downloads/settings_view.rs");
        let general_page = general_source
            .split("DownloadSettingsPage::General => div()")
            .nth(1)
            .and_then(|source| {
                source
                    .split("DownloadSettingsPage::Request => div()")
                    .next()
            })
            .expect("general page render source");
        assert!(
            general_page.contains("double_click_action,"),
            "the list double-click control must render on the general page"
        );
        assert!(
            general_source.contains("download-settings-double-click-action"),
            "the segmented control helper must expose stable debug ids"
        );
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn download_settings_window_opens_without_reentering_download_application(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "download-settings-window");

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_settings_window(cx);
            });
        });
        cx.run_until_parked();

        let settings_window = cx
            .read(|cx| application.read(cx).settings_window)
            .expect("production settings window");
        let actual_size = settings_window
            .update(cx, |_, window, _| window.bounds().size)
            .expect("read production settings window bounds");
        assert_eq!(actual_size.width, px(DOWNLOAD_SETTINGS_WINDOW_WIDTH));
        assert_eq!(actual_size.height, px(DOWNLOAD_SETTINGS_WINDOW_HEIGHT));
        cx.read(|cx| {
            let options = download_settings_window_options(cx);
            assert!(!options.is_resizable);
            assert_eq!(
                options.window_min_size,
                Some(size(
                    px(DOWNLOAD_SETTINGS_WINDOW_WIDTH),
                    px(DOWNLOAD_SETTINGS_WINDOW_HEIGHT)
                ))
            );
        });
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn completion_feedback_crosses_the_bounded_platform_transport_once_per_batch(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let directory = unique_download_settings_test_directory("completion-feedback");
        std::fs::create_dir_all(&directory).expect("create completion feedback directory");
        let mut config = AppConfig::default();
        config.download.completion_sound_enabled = true;
        config.download.open_directory_on_completion = true;
        let engine = DownloadEngine::open_in_memory().expect("open completion feedback engine");
        let (platform, platform_endpoint) = nxtools_ipc::platform_bridge(16);
        platform_endpoint.set_connected_for_contract_test(true);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let application = cx.update(|cx| {
            cx.new(|cx| {
                DownloadApplication::new(
                    engine,
                    Rc::new(platform),
                    notifications,
                    DownloadRuntimeContext {
                        fallback_default_directory: directory.clone(),
                        config,
                        config_path: directory.join("config.json"),
                        log_directory: directory.clone(),
                        database_path: directory.join("downloads.sqlite3"),
                        ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                for index in 0..5 {
                    application
                        .completion_actions
                        .record_terminal(CompletionTerminal {
                            task_id: DownloadTaskId::new(),
                            status: DownloadStatus::Completed,
                            directory: Some(directory.join(format!("batch-{index}"))),
                            display_name: format!("batch-{index}.bin"),
                            error_message: None,
                        });
                }
                application.flush_completion_feedback(cx);
            });
        });

        let mut envelopes = Vec::new();
        for _ in 0..4 {
            envelopes.push(
                platform_endpoint
                    .try_receive_command_for_contract_test()
                    .expect("bounded completion command"),
            );
        }
        assert!(matches!(
            &envelopes[0].message,
            nxtools_protocol::Message::PlatformCommand(
                PlatformCommand::PlaySystemNotificationSound
            )
        ));
        assert_eq!(
            envelopes
                .iter()
                .filter(|envelope| matches!(
                    &envelope.message,
                    nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(_))
                ))
                .count(),
            3
        );
        let operation_id = envelopes[0]
            .correlation_id
            .expect("shared feedback correlation");
        assert!(
            envelopes
                .iter()
                .all(|envelope| envelope.correlation_id == Some(operation_id))
        );
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                for envelope in envelopes {
                    application.handle_platform_notification(
                        PlatformNotification::CommandResult {
                            correlation_id: envelope.id,
                            result: PlatformCommandResult {
                                success: true,
                                error: None,
                                native_error_code: None,
                                data: None,
                            },
                        },
                        cx,
                    );
                }
                assert!(application.pending_completion_feedback.is_empty());
                assert!(application.completion_feedback_commands.is_empty());
            });
        });
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn completed_task_window_maps_exactly_one_action_to_primary_emphasis() {
        assert_eq!(CompletedTaskWindowAction::ALL.len(), 3);
        assert_eq!(
            CompletedTaskWindowAction::ALL
                .into_iter()
                .filter(|action| { action.spec().emphasis == CompletedTaskWindowEmphasis::Primary })
                .collect::<Vec<_>>(),
            vec![CompletedTaskWindowAction::Open]
        );
        assert!(
            CompletedTaskWindowAction::ALL
                .into_iter()
                .filter(|action| { action.spec().emphasis == CompletedTaskWindowEmphasis::Outline })
                .all(|action| matches!(
                    action,
                    CompletedTaskWindowAction::Drag | CompletedTaskWindowAction::Reveal
                ))
        );
    }

    #[gpui::test]
    fn production_task_window_reuses_one_skeleton_and_generation_across_phases(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory, platform_endpoint) =
            download_application_for_window_test_with_platform(cx, "task-window-shared-skeleton");
        platform_endpoint.set_connected_for_contract_test(true);
        let task_id = cx.read(|cx| {
            application
                .read(cx)
                .engine
                .enqueue(
                    DownloadRequest {
                        source_url: "https://example.test/shared-skeleton.bin".into(),
                        save_directory: directory.clone(),
                        save_directory_explicit: false,
                        file_name: Some("shared-skeleton.bin".into()),
                        headers: Vec::new(),
                        cookie: None,
                        referrer: None,
                        certificate_policy: Default::default(),
                        connection_limit: Some(1),
                        speed_limit_bytes_per_second: None,
                        expected_total_bytes: Some(4096),
                        scheduled_at_unix_ms: None,
                        adaptive_segment_policy: None,
                        media: None,
                        category_id: None,
                        category_name: None,
                        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                    },
                    RequestOrigin::Internal,
                )
                .expect("enqueue shared task-window fixture")
        });
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.refresh(cx);
                let _ = application.configure_task_window_activation_test(42);
                application.request_task_window(task_id, cx);
            });
        });
        cx.run_until_parked();

        let (task_window_handle, generation, base_snapshot) = cx.read(|cx| {
            let application = application.read(cx);
            let entry = application
                .task_windows
                .get(&task_id)
                .expect("production keyed task window");
            assert_eq!(entry.native_window_handle, 42);
            (
                entry.window,
                entry.generation,
                application.task(task_id).cloned().expect("task snapshot"),
            )
        });
        let (progress_bounds, progress_scale) = task_window_handle
            .update(cx, |_, window, _| (window.bounds(), window.scale_factor()))
            .expect("read progress window geometry");
        let mut window = VisualTestContext::from_window(task_window_handle, cx);
        window.refresh().expect("render progress task skeleton");

        let shell = window
            .debug_bounds("download-task-window-shell")
            .expect("shared task-window shell");
        let titlebar = window
            .debug_bounds("download-task-window-titlebar")
            .expect("shared task-window titlebar");
        let body = window
            .debug_bounds("download-task-window-body")
            .expect("shared task-window body");
        let footer = window
            .debug_bounds("download-task-window-footer")
            .expect("shared task-window footer");
        let progress_phase = window
            .debug_bounds("download-task-window-phase-progress")
            .expect("progress phase body");
        assert_eq!(titlebar.size.height, px(34.0));
        assert_eq!(footer.size.height, px(44.0));
        assert_eq!(progress_phase, body);
        assert_eq!(titlebar.bottom(), body.top());
        assert_eq!(body.bottom(), footer.top());
        assert_eq!(footer.bottom(), shell.bottom());
        while platform_endpoint
            .try_receive_command_for_contract_test()
            .is_ok()
        {}

        let mut failed_snapshot = base_snapshot.clone();
        failed_snapshot
            .task
            .transition_to(DownloadStatus::Probing, 2)
            .expect("fixture enters probing");
        failed_snapshot
            .task
            .transition_to(DownloadStatus::Failed, 3)
            .expect("fixture enters failed");
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.replace_tasks_for_test(vec![failed_snapshot]);
                let directive = application
                    .task_window_lifecycle
                    .task_changed(task_id, DownloadStatus::Failed);
                assert_eq!(
                    directive,
                    DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Failed)
                );
                application.apply_task_window_directive(task_id, directive, cx);
            });
        });
        cx.run_until_parked();
        window.refresh().expect("render failed task skeleton");
        let failed_resize = std::iter::from_fn(|| {
            platform_endpoint
                .try_receive_command_for_contract_test()
                .ok()
        })
        .find_map(|envelope| match envelope.message {
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::SetWindowBounds(
                command,
            )) => Some(command),
            _ => None,
        })
        .expect("failed phase crosses the native resize boundary");
        assert_eq!(
            failed_resize,
            centered_task_window_bounds_command(
                42,
                progress_bounds,
                download_task_window_size(DownloadWindowPhase::Failed),
                progress_scale,
            )
        );
        assert_eq!(
            window
                .debug_bounds("download-task-window-phase-failed")
                .expect("failed phase body"),
            window
                .debug_bounds("download-task-window-body")
                .expect("failed shared body")
        );

        let mut completed_snapshot = base_snapshot;
        for (status, timestamp) in [
            (DownloadStatus::Probing, 2),
            (DownloadStatus::Waiting, 3),
            (DownloadStatus::Downloading, 4),
            (DownloadStatus::Finalizing, 5),
            (DownloadStatus::Completed, 6),
        ] {
            completed_snapshot
                .task
                .transition_to(status, timestamp)
                .expect("fixture advances to completion");
        }
        completed_snapshot.total_bytes = Some(4096);
        completed_snapshot.downloaded_bytes = 4096;
        completed_snapshot.final_path = Some(directory.join("shared-skeleton.bin"));
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.replace_tasks_for_test(vec![completed_snapshot]);
                let directive = application
                    .task_window_lifecycle
                    .task_changed(task_id, DownloadStatus::Completed);
                assert_eq!(
                    directive,
                    DownloadWindowDirective::TransformExisting(DownloadWindowPhase::Completed)
                );
                application.apply_task_window_directive(task_id, directive, cx);
            });
        });
        cx.run_until_parked();
        window.refresh().expect("render completed task skeleton");
        assert_eq!(
            window
                .debug_bounds("download-task-window-phase-completed")
                .expect("completed phase body"),
            window
                .debug_bounds("download-task-window-body")
                .expect("completed shared body")
        );
        let drag_action = window
            .debug_bounds("download-complete-secondary-drag-action")
            .expect("completed drag secondary action");
        let reveal_action = window
            .debug_bounds("download-complete-secondary-reveal-action")
            .expect("completed reveal secondary action");
        let open_action = window
            .debug_bounds("download-complete-primary-open-action")
            .expect("completed open primary action");
        for action in [drag_action, reveal_action, open_action] {
            assert_eq!(action.size.height, px(CONTROL_HEIGHT_ACTION));
        }
        assert_eq!(drag_action.size.width, px(CONTROL_HEIGHT_ACTION));
        assert!(drag_action.right() <= reveal_action.left());
        assert!(reveal_action.right() <= open_action.left());
        assert!(
            window
                .debug_bounds("download-complete-delete-action")
                .is_none(),
            "delete must not enter the completed-window action path"
        );
        let executor = cx.executor();
        window.simulate_mouse_move(drag_action.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(600));
        window.run_until_parked();
        window
            .refresh()
            .expect("render completed drag action tooltip");
        assert!(
            window
                .debug_bounds("download-complete-drag-tooltip")
                .is_some(),
            "the completed icon-only drag action must expose its Chinese tooltip"
        );
        while platform_endpoint
            .try_receive_command_for_contract_test()
            .is_ok()
        {}

        window.simulate_mouse_down(
            drag_action.center(),
            MouseButton::Left,
            Modifiers::default(),
        );
        window.run_until_parked();
        let drag_command = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("completed drag platform command");
        let nxtools_protocol::Message::PlatformCommand(PlatformCommand::BeginFileDrag(command)) =
            drag_command.message
        else {
            panic!("completed drag must cross the typed native file-drag boundary");
        };
        assert_eq!(
            command.path,
            directory.join("shared-skeleton.bin").display().to_string()
        );
        assert_eq!(command.source_window_handle, 42);
        assert!(
            cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
            "the native drag gesture must not close the completed window"
        );

        window.simulate_click(reveal_action.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .expect("completed reveal platform command")
                .message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
                OpenShellTargetCommand {
                    target: directory.join("shared-skeleton.bin").display().to_string(),
                    action: ShellOpenAction::SelectInExplorer,
                }
            ))
        );
        assert!(
            !cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
            "opening the artifact from the completed window must close the terminal window"
        );
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                let directive = application
                    .task_window_lifecycle
                    .request_window(task_id, DownloadWindowPhase::Completed);
                assert_eq!(
                    directive,
                    DownloadWindowDirective::OpenAndFocus(DownloadWindowPhase::Completed),
                    "after the completed window closes, a reopen request must open one fresh window"
                );
                application.apply_task_window_directive(task_id, directive, cx);
            });
        });
        cx.run_until_parked();
        let reopened_handle = cx.read(|cx| {
            application
                .read(cx)
                .task_windows
                .get(&task_id)
                .map(|entry| {
                    assert_ne!(
                        entry.generation, generation,
                        "the artifact-open close must release the keyed registration so a reopen creates a fresh window"
                    );
                    entry.window
                })
                .expect("reopened completed task window")
        });
        let mut window = VisualTestContext::from_window(reopened_handle, cx);
        window.refresh().expect("render reopened completed window");
        while platform_endpoint
            .try_receive_command_for_contract_test()
            .is_ok()
        {}
        window.simulate_click(open_action.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            platform_endpoint
                .try_receive_command_for_contract_test()
                .expect("completed open platform command")
                .message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
                OpenShellTargetCommand {
                    target: directory.join("shared-skeleton.bin").display().to_string(),
                    action: ShellOpenAction::Open,
                }
            ))
        );
        assert!(
            !cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
            "the Open action must also close the completed window"
        );

        let _ = task_window_handle.update(cx, |_, window, _| window.remove_window());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn task_window_arms_and_cancels_the_real_shutdown_countdown(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "shutdown-countdown-window");
        let task_id = cx.read(|cx| {
            application
                .read(cx)
                .engine
                .enqueue(
                    DownloadRequest {
                        source_url: "https://example.test/shutdown.bin".into(),
                        save_directory: directory.clone(),
                        save_directory_explicit: false,
                        file_name: Some("shutdown.bin".into()),
                        headers: Vec::new(),
                        cookie: None,
                        referrer: None,
                        certificate_policy: Default::default(),
                        connection_limit: Some(1),
                        speed_limit_bytes_per_second: None,
                        expected_total_bytes: None,
                        scheduled_at_unix_ms: None,
                        adaptive_segment_policy: None,
                        media: None,
                        category_id: None,
                        category_name: None,
                        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                    },
                    RequestOrigin::Internal,
                )
                .expect("enqueue shutdown fixture")
        });
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.refresh(cx);
                application.open_task_window(task_id, DownloadWindowPhase::Progress, None, cx);
            });
        });
        cx.run_until_parked();
        let task_window = cx
            .read(|cx| {
                application
                    .read(cx)
                    .task_windows
                    .get(&task_id)
                    .map(|entry| entry.window)
            })
            .expect("production task window");
        let mut task_window = VisualTestContext::from_window(task_window, cx);
        task_window
            .refresh()
            .expect("render production task window");
        let arm = task_window
            .debug_bounds("download-window-shutdown-arm")
            .expect("task-local shutdown arm");
        task_window.simulate_click(arm.center(), Modifiers::default());
        task_window.run_until_parked();
        assert!(cx.read(|cx| application.read(cx).is_shutdown_armed(task_id)));

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                let transition = application
                    .completion_actions
                    .evaluate_shutdown([(task_id, DownloadStatus::Completed)], Instant::now());
                application.apply_shutdown_transition(transition, cx);
            });
        });
        cx.run_until_parked();
        let shutdown_window_handle = cx
            .read(|cx| {
                application
                    .read(cx)
                    .shutdown_window
                    .as_ref()
                    .map(|window| window.window)
            })
            .expect("production shutdown countdown window");
        let mut shutdown_window = VisualTestContext::from_window(shutdown_window_handle, cx);
        shutdown_window
            .refresh()
            .expect("render production shutdown countdown");
        assert_shared_dialog_shell(
            &mut shutdown_window,
            "download-shutdown-dialog-shell",
            "download-shutdown-dialog-titlebar",
            "download-shutdown-dialog-body",
            "download-shutdown-dialog-footer",
        );
        let cancel = shutdown_window
            .debug_bounds("download-shutdown-secondary-cancel-action")
            .expect("shutdown cancel secondary action");
        let shutdown_now = shutdown_window
            .debug_bounds("download-shutdown-danger-now-action")
            .expect("shutdown danger action");
        assert_eq!(cancel.size.height, px(CONTROL_HEIGHT_ACTION));
        assert_eq!(shutdown_now.size.height, px(CONTROL_HEIGHT_ACTION));
        assert!(shutdown_now.right() <= cancel.left());
        shutdown_window.simulate_click(cancel.center(), Modifiers::default());
        shutdown_window.run_until_parked();
        assert!(cx.read(|cx| {
            let application = application.read(cx);
            application.completion_actions.active_countdown().is_none()
                && application.shutdown_window.is_none()
                && application.pending_shutdown_command.is_none()
        }));
        assert!(shutdown_window_handle.update(cx, |_, _, _| ()).is_err());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn a_native_task_window_close_releases_the_keyed_lifecycle_registration(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "native-close-lifecycle");
        let task_id = cx.read(|cx| {
            application
                .read(cx)
                .engine
                .enqueue(
                    DownloadRequest {
                        source_url: "https://example.test/native-close.bin".into(),
                        save_directory: directory.clone(),
                        save_directory_explicit: false,
                        file_name: Some("native-close.bin".into()),
                        headers: Vec::new(),
                        cookie: None,
                        referrer: None,
                        certificate_policy: Default::default(),
                        connection_limit: Some(1),
                        speed_limit_bytes_per_second: None,
                        expected_total_bytes: None,
                        scheduled_at_unix_ms: None,
                        adaptive_segment_policy: None,
                        media: None,
                        category_id: None,
                        category_name: None,
                        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                    },
                    RequestOrigin::Internal,
                )
                .expect("enqueue native-close fixture")
        });
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.refresh(cx);
                application.open_task_window(task_id, DownloadWindowPhase::Progress, None, cx);
            });
        });
        cx.run_until_parked();
        let task_window = cx
            .read(|cx| {
                application
                    .read(cx)
                    .task_windows
                    .get(&task_id)
                    .map(|entry| entry.window)
            })
            .expect("production task window");
        assert!(
            task_window.update(cx, |_, _, _| ()).is_ok(),
            "the keyed task window is alive before the native close"
        );

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.handle_task_window_native_close(task_id, task_window, cx);
            });
            // GPUI consumes the WM_CLOSE that triggered the handler and destroys
            // the platform window; mirror that in the test harness.
            let _ = task_window.update(cx, |_, window, _| window.remove_window());
        });
        cx.run_until_parked();
        assert!(
            !cx.read(|cx| application.read(cx).task_windows.contains_key(&task_id)),
            "the native close must release the keyed registration"
        );
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                // The next progress event arrives after the user closed the window:
                // the lifecycle must open one fresh window instead of targeting the
                // destroyed one and replacing it (the visible flash).
                let directive = application
                    .task_window_lifecycle
                    .task_changed(task_id, DownloadStatus::Downloading);
                assert_eq!(
                    directive,
                    DownloadWindowDirective::None,
                    "a task event after a native close must not resurrect the progress window"
                );
                application.apply_task_window_directive(task_id, directive, cx);
            });
        });
        assert!(
            task_window.update(cx, |_, _, _| ()).is_err(),
            "the natively closed window stays gone"
        );
        assert!(cx.read(|cx| application.read(cx).task_windows.is_empty()));

        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn native_shutdown_window_close_cancels_the_countdown(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "shutdown-native-close");
        let task_id = DownloadTaskId::new();
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application
                    .completion_actions
                    .set_shutdown_armed(task_id, true);
                let transition = application
                    .completion_actions
                    .evaluate_shutdown([(task_id, DownloadStatus::Completed)], Instant::now());
                application.apply_shutdown_transition(transition, cx);
            });
        });
        cx.run_until_parked();
        let window_handle = cx
            .read(|cx| {
                application
                    .read(cx)
                    .shutdown_window
                    .as_ref()
                    .map(|window| window.window)
            })
            .expect("production shutdown countdown window");
        let mut window = VisualTestContext::from_window(window_handle, cx);
        assert!(
            window.simulate_close(),
            "native close is accepted after disarming"
        );
        window.run_until_parked();
        assert!(cx.read(|cx| {
            let application = application.read(cx);
            application.completion_actions.active_countdown().is_none()
                && application.shutdown_window.is_none()
                && application.pending_shutdown_command.is_none()
        }));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn stale_shutdown_arm_click_cannot_arm_a_task_that_already_terminated(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "shutdown-stale-arm");
        let task_id = cx.read(|cx| {
            application
                .read(cx)
                .engine
                .enqueue(
                    DownloadRequest {
                        source_url: "https://example.test/stale-arm.bin".into(),
                        save_directory: directory.clone(),
                        save_directory_explicit: false,
                        file_name: Some("stale-arm.bin".into()),
                        headers: Vec::new(),
                        cookie: None,
                        referrer: None,
                        certificate_policy: Default::default(),
                        connection_limit: Some(1),
                        speed_limit_bytes_per_second: None,
                        expected_total_bytes: None,
                        scheduled_at_unix_ms: None,
                        adaptive_segment_policy: None,
                        media: None,
                        category_id: None,
                        category_name: None,
                        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                    },
                    RequestOrigin::Internal,
                )
                .expect("enqueue stale arm fixture")
        });
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.refresh(cx);
                application.open_task_window(task_id, DownloadWindowPhase::Progress, None, cx);
            });
        });
        cx.run_until_parked();
        let task_window = cx
            .read(|cx| {
                application
                    .read(cx)
                    .task_windows
                    .get(&task_id)
                    .map(|entry| entry.window)
            })
            .expect("production task window");
        let mut task_window = VisualTestContext::from_window(task_window, cx);
        task_window
            .refresh()
            .expect("render task window before terminal");
        let arm = task_window
            .debug_bounds("download-window-shutdown-arm")
            .expect("stale shutdown arm hit target");

        cx.read(|cx| {
            application
                .read(cx)
                .engine
                .cancel(task_id)
                .expect("terminate task before deferred arm click");
        });
        task_window.simulate_click(arm.center(), Modifiers::default());
        task_window.run_until_parked();
        assert!(cx.read(|cx| {
            let application = application.read(cx);
            !application.is_shutdown_armed(task_id)
                && application.completion_actions.active_countdown().is_none()
                && application.shutdown_window.is_none()
        }));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn shutdown_now_crosses_the_typed_platform_boundary_and_closes_on_success(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let directory = unique_download_settings_test_directory("shutdown-platform-boundary");
        std::fs::create_dir_all(&directory).expect("create shutdown boundary directory");
        let engine = DownloadEngine::open_in_memory().expect("open shutdown boundary engine");
        let (platform, platform_endpoint) = nxtools_ipc::platform_bridge(8);
        platform_endpoint.set_connected_for_contract_test(true);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let application = cx.update(|cx| {
            cx.new(|cx| {
                DownloadApplication::new(
                    engine,
                    Rc::new(platform),
                    notifications,
                    DownloadRuntimeContext {
                        fallback_default_directory: directory.clone(),
                        config: AppConfig::default(),
                        config_path: directory.join("config.json"),
                        log_directory: directory.clone(),
                        database_path: directory.join("downloads.sqlite3"),
                        ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        let task_id = DownloadTaskId::new();
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application
                    .completion_actions
                    .set_shutdown_armed(task_id, true);
                let transition = application
                    .completion_actions
                    .evaluate_shutdown([(task_id, DownloadStatus::Completed)], Instant::now());
                application.apply_shutdown_transition(transition, cx);
            });
        });
        cx.run_until_parked();
        let window_handle = cx
            .read(|cx| {
                application
                    .read(cx)
                    .shutdown_window
                    .as_ref()
                    .map(|window| window.window)
            })
            .expect("production shutdown countdown window");
        let mut window = VisualTestContext::from_window(window_handle, cx);
        window.refresh().expect("render shutdown countdown window");
        let shutdown_now = window
            .debug_bounds("download-shutdown-danger-now-action")
            .expect("immediate shutdown action");
        window.simulate_click(shutdown_now.center(), Modifiers::default());
        window.run_until_parked();

        let envelope = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("typed shutdown command");
        assert!(matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::ShutdownSystem(
                ShutdownSystemCommand { delay_seconds: 1 }
            ))
        ));
        let operation_id = envelope
            .correlation_id
            .expect("shutdown operation correlation");
        assert_eq!(
            cx.read(|cx| {
                application
                    .read(cx)
                    .pending_shutdown_command
                    .as_ref()
                    .map(|pending| pending.countdown.correlation_id)
            }),
            Some(operation_id)
        );
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: envelope.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: None,
                        },
                    },
                    cx,
                );
            });
        });
        assert!(cx.read(|cx| {
            let application = application.read(cx);
            application.pending_shutdown_command.is_none()
                && application.shutdown_window.is_none()
                && application.completion_actions.active_countdown().is_none()
        }));
        assert!(window_handle.update(cx, |_, _, _| ()).is_err());
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn closing_download_settings_always_leaves_taskbar_overlay_edit_mode(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "settings-edit-close");
        let correlation_id = uuid::Uuid::new_v4();

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.taskbar_overlay_edit_draft =
                    Some(DownloadTaskbarProgressOverlaySettings::default());
                application.taskbar_overlay_edit_correlation_id = Some(correlation_id);
                application.handle_settings_window_closed(cx);
            });
        });

        assert!(cx.read(|cx| {
            let application = application.read(cx);
            application.settings_window.is_none()
                && application.taskbar_overlay_edit_draft.is_none()
                && application.taskbar_overlay_edit_correlation_id.is_none()
        }));
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn native_move_size_end_closes_only_the_matching_taskbar_edit_gesture(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "taskbar-edit-native-finish");
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.taskbar_overlay_native_window_handle = 42;
                let failed_command_id = uuid::Uuid::new_v4();
                application.taskbar_overlay_edit_gesture_active = true;
                application.register_taskbar_overlay_edit_gesture(failed_command_id);
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: failed_command_id,
                        result: PlatformCommandResult {
                            success: false,
                            error: Some("native move rejected".into()),
                            native_error_code: None,
                            data: None,
                        },
                    },
                    cx,
                );
                assert!(!application.taskbar_overlay_edit_gesture_active);

                application.taskbar_overlay_edit_gesture_active = true;
                application.handle_platform_notification(
                    PlatformNotification::Event(PlatformEvent::TaskbarOverlayEditGestureFinished(
                        nxtools_protocol::TaskbarOverlayEditGesture { window_handle: 7 },
                    )),
                    cx,
                );
                assert!(application.taskbar_overlay_edit_gesture_active);
                application.handle_platform_notification(
                    PlatformNotification::Event(PlatformEvent::TaskbarOverlayEditGestureFinished(
                        nxtools_protocol::TaskbarOverlayEditGesture { window_handle: 42 },
                    )),
                    cx,
                );
                assert!(!application.taskbar_overlay_edit_gesture_active);
            });
        });
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn ffmpeg_platform_timeout_releases_picker_and_page_operations(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "ffmpeg-platform-timeout");
        let (picker_id, page_id) = cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.request_ffmpeg_executable(String::new(), cx);
                application.open_ffmpeg_download_page(FfmpegDownloadSource::Gyan, cx);
                let commands = application.ffmpeg_platform_commands.borrow();
                (commands[0].0, commands[1].0)
            })
        });

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.ffmpeg_platform_operation_timed_out(picker_id, cx);
                application.ffmpeg_platform_operation_timed_out(page_id, cx);
                assert!(application.pending_ffmpeg_operations.is_empty());
                application.request_ffmpeg_executable(String::new(), cx);
                assert_eq!(application.ffmpeg_platform_commands.borrow().len(), 3);
            });
        });
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn settings_auto_download_commits_managed_ffmpeg_without_manual_save(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "ffmpeg-auto-install");
        let config_path = directory.join("config.json");
        AppConfig::default()
            .save_to(&config_path)
            .expect("save FFmpeg auto-install config fixture");
        let managed_directory = directory
            .join("tools")
            .join("ffmpeg")
            .join("managed-fixture");
        std::fs::create_dir_all(&managed_directory).expect("create managed FFmpeg fixture");
        let executable = managed_directory.join("ffmpeg.exe");
        std::fs::write(&executable, b"fixture").expect("write managed FFmpeg fixture");
        let (progress_sender, progress) = async_channel::bounded(4);
        let (completion_sender, completion) = async_channel::bounded(1);
        let correlation_id = uuid::Uuid::new_v4();
        cx.update(|cx| {
            application.update(cx, |application, _| {
                application.ffmpeg_install_operation_override =
                    Some(FfmpegInstallOperationOverride {
                        correlation_id,
                        progress,
                        completion,
                        cancelled: Rc::new(Cell::new(false)),
                    });
            });
        });
        let application_for_window = application.clone();
        let initial = cx.read(|cx| application.read(cx).settings_view_initial());
        let (settings, window) = cx.add_window_view(move |window, cx| {
            DownloadSettingsView::new(application_for_window, initial, window, cx)
        });
        window
            .refresh()
            .expect("render FFmpeg auto-install settings");
        window.run_until_parked();
        let transfer_navigation = window
            .debug_bounds("download-settings-nav-transfer")
            .expect("transfer settings navigation");
        window.simulate_click(transfer_navigation.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render FFmpeg install section");
        let install = window
            .debug_bounds("download-settings-ffmpeg-install")
            .expect("FFmpeg auto-download button");
        window.simulate_click(install.center(), Modifiers::default());
        window.run_until_parked();
        assert!(
            window
                .debug_bounds("download-settings-ffmpeg-cancel")
                .is_some()
        );
        progress_sender
            .try_send(FfmpegInstallProgress {
                phase: FfmpegInstallPhase::Downloading,
                percent: 48,
                downloaded_bytes: 48,
                total_bytes: Some(100),
            })
            .expect("send FFmpeg install progress");
        drop(progress_sender);
        completion_sender
            .try_send(Ok(FfmpegInstallation {
                executable: executable.clone(),
                source: FfmpegInstallSource::Gyan,
                downloaded_bytes: 100,
            }))
            .expect("complete FFmpeg install");
        drop(completion_sender);
        window.run_until_parked();

        let persisted = AppConfig::load_from(&config_path).expect("load automatic FFmpeg config");
        assert_eq!(
            persisted.download.ffmpeg_executable_path,
            executable.display().to_string()
        );
        assert_eq!(
            window.read(|cx| {
                settings
                    .read(cx)
                    .ffmpeg_executable
                    .read(cx)
                    .value()
                    .to_string()
            }),
            executable.display().to_string()
        );
        cx.read(|cx| {
            assert_eq!(
                application
                    .read(cx)
                    .engine
                    .ffmpeg_resolution()
                    .expect("resolve managed FFmpeg")
                    .expect("managed FFmpeg should be configured")
                    .executable,
                executable
            );
        });
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn settings_media_tools_gives_the_ffmpeg_path_a_full_width_row(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "ffmpeg-settings-layout");
        let application_for_window = application.clone();
        let initial = cx.read(|cx| application.read(cx).settings_view_initial());
        let (_settings, window) = cx.add_window_view(move |window, cx| {
            window.resize(size(px(984.0), px(650.0)));
            DownloadSettingsView::new(application_for_window, initial, window, cx)
        });
        window.refresh().expect("render FFmpeg settings layout");
        let transfer_navigation = window
            .debug_bounds("download-settings-nav-transfer")
            .expect("transfer settings navigation");
        window.simulate_click(transfer_navigation.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render transfer settings layout");

        let section = window
            .debug_bounds("download-settings-media-tools-section")
            .expect("media tools section");
        let control = window
            .debug_bounds("download-settings-ffmpeg-control")
            .expect("FFmpeg path control");
        let width_ratio = control.size.width.as_f32() / section.size.width.as_f32();
        assert!(
            width_ratio >= 0.9,
            "the FFmpeg path control is still cramped at {}px inside a {}px section ({width_ratio:.2})",
            control.size.width.as_f32(),
            section.size.width.as_f32()
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn settings_sections_share_one_content_width_and_diagnostics_are_not_half_width(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let (application, directory, platform_endpoint) =
            download_application_for_window_test_with_platform(cx, "settings-section-widths");
        platform_endpoint.set_connected_for_contract_test(true);
        let diagnostic_root = directory
            .join("diagnostics-path-that-is-deliberately-long-for-the-production-settings-window")
            .join("runtime-owned-locations");
        let log_directory = diagnostic_root.join("logs");
        std::fs::create_dir_all(&log_directory).expect("create long diagnostics log directory");
        let config_path = diagnostic_root.join("nxtools-production-configuration.json");
        let database_path = diagnostic_root.join("downloads-production-database.sqlite3");
        AppConfig::default()
            .save_to(&config_path)
            .expect("save production settings layout config fixture");
        std::fs::write(&database_path, b"diagnostics database fixture")
            .expect("save production settings database fixture");
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.config_path = config_path.clone();
                application.log_directory = log_directory.clone();
                application.database_path = database_path.clone();
                application.open_settings_window(cx);
            });
        });
        cx.run_until_parked();
        let settings_window = cx
            .read(|cx| application.read(cx).settings_window)
            .expect("production settings window");
        let mut window = VisualTestContext::from_window(settings_window, cx);
        window.run_until_parked();
        window.refresh().expect("render download settings widths");
        let settings_view = cx
            .read(|cx| application.read(cx).settings_view.clone())
            .expect("production settings view");

        let shell = window
            .debug_bounds("download-settings-shell")
            .expect("shared download settings shell");
        let shell_content = window
            .debug_bounds("download-settings-shell-content")
            .expect("shared download settings content");
        let shell_footer = window
            .debug_bounds("download-settings-shell-footer")
            .expect("shared download settings footer");
        let titlebar = window
            .debug_bounds("download-settings-titlebar")
            .expect("shared download settings titlebar");
        assert_eq!(shell.size.width, px(DOWNLOAD_SETTINGS_WINDOW_WIDTH));
        assert_eq!(shell.size.height, px(DOWNLOAD_SETTINGS_WINDOW_HEIGHT));
        assert_eq!(
            titlebar.size.height,
            px(crate::ui::metrics::TITLEBAR_HEIGHT)
        );
        assert_eq!(shell_footer.size.height, px(48.0));
        assert_eq!(shell_content.bottom(), shell_footer.top());
        assert_eq!(shell_footer.bottom(), shell.bottom());

        let navigation_title = window
            .debug_bounds("download-settings-nav-general-title")
            .expect("general navigation title");
        let navigation_description = window
            .debug_bounds("download-settings-nav-general-description")
            .expect("general navigation description");
        assert_eq!(navigation_title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(navigation_description.size.height, px(LINE_HEIGHT_LABEL));

        let row_title = window
            .debug_bounds("download-settings-row-默认保存目录-title")
            .expect("default directory row title");
        let row_description = window
            .debug_bounds("download-settings-row-默认保存目录-description")
            .expect("default directory row description");
        assert_eq!(row_title.size.height, px(LINE_HEIGHT_BODY));
        assert_eq!(row_description.size.height, px(LINE_HEIGHT_LABEL));

        for (name, selector) in [
            ("cancel", "download-settings-cancel"),
            ("apply", "download-settings-apply"),
        ] {
            let action = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing {name} settings action"));
            assert_eq!(
                action.size.height,
                px(28.0),
                "{name} must use the shared 28px action tier"
            );
        }

        let save = window
            .debug_bounds("download-settings-section-保存")
            .expect("save section");
        let expected_left = save.origin.x.as_f32();
        let expected_width = save.size.width.as_f32();
        macro_rules! assert_section {
            ($name:literal, $selector:literal) => {{
                let bounds = window
                    .debug_bounds($selector)
                    .unwrap_or_else(|| panic!("missing {} section", $name));
                assert!(
                    (bounds.origin.x.as_f32() - expected_left).abs() <= 1.0,
                    "{} starts at {}px instead of {expected_left}px",
                    $name,
                    bounds.origin.x.as_f32()
                );
                assert!(
                    (bounds.size.width.as_f32() - expected_width).abs() <= 1.0,
                    "{} is {}px wide instead of the shared {expected_width}px content width",
                    $name,
                    bounds.size.width.as_f32()
                );
            }};
        }
        macro_rules! open_page {
            ($selector:literal) => {{
                let navigation = window
                    .debug_bounds($selector)
                    .expect("settings page navigation");
                window.simulate_click(navigation.center(), Modifiers::default());
                window.run_until_parked();
                window.refresh().expect("render selected settings page");
                assert_eq!(
                    window
                        .debug_bounds("download-settings-shell-footer")
                        .expect("stable settings footer"),
                    shell_footer,
                    "switching settings pages must not move the shared footer"
                );
            }};
        }
        assert_section!("分类与重名", "download-settings-section-分类与重名");
        assert!(
            window
                .debug_bounds("download-settings-section-传输")
                .is_none(),
            "inactive pages must not remain in one oversized scrolling column"
        );
        assert!(
            expected_width >= 760.0,
            "settings content remains unexpectedly narrow at {expected_width}px"
        );

        let initial_automatic_categories =
            cx.read(|cx| settings_view.read(cx).automatic_categories_enabled);
        let automatic_categories = window
            .debug_bounds("download-settings-automatic-categories")
            .expect("automatic categories draft control");
        window.simulate_click(automatic_categories.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            cx.read(|cx| settings_view.read(cx).automatic_categories_enabled),
            !initial_automatic_categories,
            "the production settings draft must change before page navigation"
        );

        open_page!("download-settings-nav-request");
        assert_section!("请求上下文", "download-settings-section-请求上下文");
        assert_section!("证书校验", "download-settings-section-证书校验");
        let request_form = window
            .debug_bounds("download-settings-request-context-form")
            .expect("request-context form");
        let header_field = window
            .debug_bounds("download-settings-default-headers-field")
            .expect("full-width Header editor");
        let header_label = window
            .debug_bounds("download-settings-default-headers-field-label")
            .expect("Header editor label");
        let cookie_field = window
            .debug_bounds("download-settings-default-cookie-field")
            .expect("full-width Cookie field");
        let cookie_label = window
            .debug_bounds("download-settings-default-cookie-field-label")
            .expect("Cookie field label");
        let certificate_section = window
            .debug_bounds("download-settings-section-证书校验")
            .expect("certificate policy section");
        let certificate_toggle = window
            .debug_bounds("download-settings-accept-invalid-certificates")
            .expect("certificate policy toggle");
        assert_eq!(header_field.size.height, px(120.0));
        assert_eq!(cookie_field.size.height, px(CONTROL_HEIGHT_INPUT));
        assert_eq!(header_field.origin.x, cookie_field.origin.x);
        assert_eq!(header_field.size.width, cookie_field.size.width);
        assert_eq!(header_label.origin.x, cookie_label.origin.x);
        assert!(request_form.contains(&header_field.origin));
        assert!(request_form.contains(&header_field.bottom_right()));
        assert!(request_form.contains(&cookie_field.origin));
        assert!(request_form.contains(&cookie_field.bottom_right()));
        assert!(certificate_section.bottom() <= shell_footer.top());

        let long_header = (0..48)
            .map(|index| format!("X-NxTools-{index}: bounded-value-{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let (header_input, cookie_input) = cx.read(|cx| {
            let settings = settings_view.read(cx);
            (
                settings.default_request_headers.clone(),
                settings.default_cookie.clone(),
            )
        });
        window.update(|window, cx| {
            header_input.update(cx, |input, cx| {
                input.set_value(&long_header, window, cx);
            });
            cookie_input.update(cx, |input, cx| {
                input.set_value("session=bounded", window, cx);
            });
        });
        window.run_until_parked();
        window.refresh().expect("render long request context");
        assert_eq!(
            window
                .debug_bounds("download-settings-default-headers-field")
                .expect("stable Header editor after long input"),
            header_field,
            "long Header content must scroll inside a fixed editor instead of growing the page"
        );
        assert_eq!(
            window
                .debug_bounds("download-settings-shell-footer")
                .expect("fixed footer after long Header input"),
            shell_footer,
            "long Header content must not move or resize the settings footer"
        );
        assert_eq!(
            window
                .debug_bounds("download-settings-section-证书校验")
                .expect("stable certificate section after long Header input"),
            certificate_section,
            "long Header content must not move the separate certificate policy section"
        );
        assert!(
            cx.read(|cx| {
                settings_view
                    .read(cx)
                    .default_request_headers
                    .read(cx)
                    .value()
                    .to_string()
            }) == long_header,
            "the fixed editor must preserve the complete Header draft"
        );
        window.simulate_click(certificate_toggle.center(), Modifiers::default());
        window.run_until_parked();
        assert!(
            cx.read(|cx| settings_view.read(cx).accept_invalid_certificates),
            "the production certificate toggle must update the same request-settings draft"
        );
        open_page!("download-settings-nav-transfer");
        assert_section!("传输", "download-settings-section-传输");
        assert_section!("媒体工具", "download-settings-section-媒体工具");
        let media_rows = [
            "download-settings-stacked-row-FFmpeg 可执行文件",
            "download-settings-stacked-row-自动安装",
            "download-settings-stacked-row-公开下载来源",
        ]
        .map(|selector| {
            window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing FFmpeg sequence row {selector}"))
        });
        assert!(
            media_rows.windows(2).all(|rows| {
                (rows[0].origin.x.as_f32() - rows[1].origin.x.as_f32()).abs() <= 1.0
                    && (rows[0].size.width.as_f32() - rows[1].size.width.as_f32()).abs() <= 1.0
                    && rows[1].origin.y.as_f32() > rows[0].origin.y.as_f32()
            }),
            "FFmpeg setup must render as three ordered full-width rows"
        );
        assert!(
            media_rows
                .iter()
                .all(|row| row.size.height.as_f32() <= 72.0),
            "FFmpeg setup rows must stay on the dense 72 px sequence tier"
        );
        let ffmpeg_path_control = window
            .debug_bounds("download-settings-ffmpeg-control")
            .expect("full-width FFmpeg path control");
        let ffmpeg_install_actions = window
            .debug_bounds("download-settings-ffmpeg-install-actions")
            .expect("full-width FFmpeg install action rail");
        let ffmpeg_source_actions = window
            .debug_bounds("download-settings-ffmpeg-source-actions")
            .expect("full-width FFmpeg source action rail");
        assert!(
            [
                ffmpeg_path_control,
                ffmpeg_install_actions,
                ffmpeg_source_actions
            ]
            .windows(2)
            .all(|controls| {
                (controls[0].origin.x.as_f32() - controls[1].origin.x.as_f32()).abs() <= 1.0
                    && (controls[0].size.width.as_f32() - controls[1].size.width.as_f32()).abs()
                        <= 1.0
            }),
            "path, managed install and public sources must share one full-width left control rail"
        );
        assert!(
            ffmpeg_path_control.size.width.as_f32()
                >= media_rows[0].size.width.as_f32() - SPACING_8 * 2.0 - 2.0,
            "the executable path must consume the full sequence-row content width"
        );
        let install_button = window
            .debug_bounds("download-settings-ffmpeg-install")
            .expect("primary managed-install action");
        let gyan_button = window
            .debug_bounds("download-settings-ffmpeg-gyan")
            .expect("quiet Gyan fallback action");
        assert!(
            (install_button.origin.x.as_f32() - ffmpeg_install_actions.origin.x.as_f32()).abs()
                <= 1.0
                && (gyan_button.origin.x.as_f32() - ffmpeg_source_actions.origin.x.as_f32()).abs()
                    <= 1.0,
            "managed install and public fallbacks must begin at the shared left rail"
        );
        window.update(|_, cx| {
            application.update(cx, |application, cx| {
                application.set_ffmpeg_install_state(
                    FfmpegInstallViewState::Installing {
                        phase: FfmpegInstallPhase::Downloading,
                        percent: 48,
                        downloaded_bytes: 48,
                        total_bytes: Some(100),
                        cancelling: false,
                    },
                    cx,
                );
            });
        });
        window.run_until_parked();
        window
            .refresh()
            .expect("render active FFmpeg installation in production settings");
        let installing_media_rows = [
            "download-settings-stacked-row-FFmpeg 可执行文件",
            "download-settings-stacked-row-自动安装",
            "download-settings-stacked-row-公开下载来源",
        ]
        .map(|selector| {
            window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing active FFmpeg sequence row {selector}"))
        });
        assert_eq!(
            installing_media_rows, media_rows,
            "install progress and Cancel must not move or resize the FFmpeg sequence rows"
        );
        assert_eq!(
            window
                .debug_bounds("download-settings-shell-footer")
                .expect("fixed footer during FFmpeg installation"),
            shell_footer,
            "install progress and Cancel must not move the fixed settings footer"
        );
        let active_cancel = window
            .debug_bounds("download-settings-ffmpeg-cancel")
            .expect("quiet Cancel action during FFmpeg installation");
        assert!(
            installing_media_rows[1].contains(&active_cancel.origin)
                && installing_media_rows[1].contains(&active_cancel.bottom_right()),
            "the active Cancel action must remain inside the automatic-install row"
        );
        window.update(|_, cx| {
            application.update(cx, |application, cx| {
                application.set_ffmpeg_install_state(FfmpegInstallViewState::Idle, cx);
            });
        });
        window.run_until_parked();
        let first_transfer_cell = window
            .debug_bounds("download-settings-grid-传输-0")
            .expect("first transfer grid cell");
        let second_transfer_cell = window
            .debug_bounds("download-settings-grid-传输-1")
            .expect("second transfer grid cell");
        assert!(
            (first_transfer_cell.origin.y.as_f32() - second_transfer_cell.origin.y.as_f32()).abs()
                <= 1.0,
            "the first two transfer settings are not in the same grid row"
        );
        assert!(
            (first_transfer_cell.size.width.as_f32() - second_transfer_cell.size.width.as_f32())
                .abs()
                <= 1.0
                && (first_transfer_cell.size.width.as_f32()
                    + second_transfer_cell.size.width.as_f32()
                    + 3.0
                    - expected_width)
                    .abs()
                    <= 2.0,
            "transfer grid cells must equally fill the {expected_width}px section after its two 1px borders and 1px center seam; actual widths were {}px and {}px",
            first_transfer_cell.size.width.as_f32(),
            second_transfer_cell.size.width.as_f32()
        );
        let transfer_section = window
            .debug_bounds("download-settings-section-传输")
            .expect("continuous transfer settings section");
        let transfer_matrix_selectors = [
            (
                "download-settings-matrix-传输-row-0",
                "download-settings-matrix-传输-cell-0-left",
                "download-settings-matrix-传输-seam-0",
                "download-settings-matrix-传输-cell-0-right",
            ),
            (
                "download-settings-matrix-传输-row-1",
                "download-settings-matrix-传输-cell-1-left",
                "download-settings-matrix-传输-seam-1",
                "download-settings-matrix-传输-cell-1-right",
            ),
            (
                "download-settings-matrix-传输-row-2",
                "download-settings-matrix-传输-cell-2-left",
                "download-settings-matrix-传输-seam-2",
                "download-settings-matrix-传输-cell-2-right",
            ),
        ];
        let mut transfer_rows = Vec::new();
        for (row_index, (row_selector, left_selector, seam_selector, right_selector)) in
            transfer_matrix_selectors.into_iter().enumerate()
        {
            let row = window
                .debug_bounds(row_selector)
                .unwrap_or_else(|| panic!("missing transfer matrix row {row_index}"));
            let left = window
                .debug_bounds(left_selector)
                .unwrap_or_else(|| panic!("missing left transfer cell {row_index}"));
            let seam = window
                .debug_bounds(seam_selector)
                .unwrap_or_else(|| panic!("missing transfer seam {row_index}"));
            let right = window
                .debug_bounds(right_selector)
                .unwrap_or_else(|| panic!("missing right transfer cell {row_index}"));
            assert!(
                (left.origin.y.as_f32() - right.origin.y.as_f32()).abs() <= 1.0
                    && (left.size.height.as_f32() - right.size.height.as_f32()).abs() <= 1.0,
                "transfer matrix row {row_index} must share one vertical baseline"
            );
            assert!(
                (seam.size.width.as_f32() - 1.0).abs() <= 0.5
                    && (left.origin.x.as_f32() + left.size.width.as_f32() - seam.origin.x.as_f32())
                        .abs()
                        <= 1.0
                    && (seam.origin.x.as_f32() + seam.size.width.as_f32()
                        - right.origin.x.as_f32())
                    .abs()
                        <= 1.0,
                "transfer matrix row {row_index} must use one 1 px center seam without card gaps"
            );
            assert!(
                (left.size.width.as_f32() - right.size.width.as_f32()).abs() <= 2.0,
                "transfer matrix row {row_index} must split the parent into equal cells"
            );
            assert!(
                left.origin.x.as_f32() >= transfer_section.origin.x.as_f32()
                    && right.origin.x.as_f32() + right.size.width.as_f32()
                        <= transfer_section.origin.x.as_f32()
                            + transfer_section.size.width.as_f32()
                            + 1.0,
                "transfer matrix row {row_index} must stay inside the section width"
            );
            transfer_rows.push(row);
        }
        assert!(
            transfer_rows
                .windows(2)
                .all(
                    |rows| (rows[0].origin.x.as_f32() - rows[1].origin.x.as_f32()).abs() <= 1.0
                        && (rows[0].size.width.as_f32() - rows[1].size.width.as_f32()).abs() <= 1.0
                ),
            "all transfer matrix rows must share one parent-width geometry"
        );
        let left_control_edges = [
            "download-settings-connection-limit-field",
            "download-settings-adaptive-segments",
            "download-settings-adaptive-min-field",
        ]
        .map(|selector| {
            let bounds = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing transfer control {selector}"));
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        let right_control_edges = [
            "download-settings-concurrent-tasks-field",
            "download-settings-adaptive-max-field",
            "download-settings-speed-limit-field",
        ]
        .map(|selector| {
            let bounds = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing transfer control {selector}"));
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        assert!(
            left_control_edges
                .windows(2)
                .all(|edges| (edges[0] - edges[1]).abs() <= 1.0)
                && right_control_edges
                    .windows(2)
                    .all(|edges| (edges[0] - edges[1]).abs() <= 1.0),
            "transfer controls must align to stable left and right trailing rails"
        );
        open_page!("download-settings-nav-window");
        assert_section!("任务窗口", "download-settings-section-任务窗口");
        assert_section!("任务栏进度", "download-settings-section-任务栏进度");
        let window_matrix_sections = [
            ("任务窗口", "download-settings-section-任务窗口", 2_usize),
            (
                "任务栏进度",
                "download-settings-section-任务栏进度",
                7_usize,
            ),
        ];
        for (title, section_selector, row_count) in window_matrix_sections {
            let section = window
                .debug_bounds(section_selector)
                .unwrap_or_else(|| panic!("missing {title} matrix section"));
            for row_index in 0..row_count {
                let (row_selector, left_selector, seam_selector, right_selector) = match title {
                    "任务窗口" => [
                        (
                            "download-settings-matrix-任务窗口-row-0",
                            "download-settings-matrix-任务窗口-cell-0-left",
                            "download-settings-matrix-任务窗口-seam-0",
                            "download-settings-matrix-任务窗口-cell-0-right",
                        ),
                        (
                            "download-settings-matrix-任务窗口-row-1",
                            "download-settings-matrix-任务窗口-cell-1-left",
                            "download-settings-matrix-任务窗口-seam-1",
                            "download-settings-matrix-任务窗口-cell-1-right",
                        ),
                    ][row_index],
                    "任务栏进度" => [
                        (
                            "download-settings-matrix-任务栏进度-row-0",
                            "download-settings-matrix-任务栏进度-cell-0-left",
                            "download-settings-matrix-任务栏进度-seam-0",
                            "download-settings-matrix-任务栏进度-cell-0-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-1",
                            "download-settings-matrix-任务栏进度-cell-1-left",
                            "download-settings-matrix-任务栏进度-seam-1",
                            "download-settings-matrix-任务栏进度-cell-1-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-2",
                            "download-settings-matrix-任务栏进度-cell-2-left",
                            "download-settings-matrix-任务栏进度-seam-2",
                            "download-settings-matrix-任务栏进度-cell-2-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-3",
                            "download-settings-matrix-任务栏进度-cell-3-left",
                            "download-settings-matrix-任务栏进度-seam-3",
                            "download-settings-matrix-任务栏进度-cell-3-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-4",
                            "download-settings-matrix-任务栏进度-cell-4-left",
                            "download-settings-matrix-任务栏进度-seam-4",
                            "download-settings-matrix-任务栏进度-cell-4-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-5",
                            "download-settings-matrix-任务栏进度-cell-5-left",
                            "download-settings-matrix-任务栏进度-seam-5",
                            "download-settings-matrix-任务栏进度-cell-5-right",
                        ),
                        (
                            "download-settings-matrix-任务栏进度-row-6",
                            "download-settings-matrix-任务栏进度-cell-6-left",
                            "download-settings-matrix-任务栏进度-seam-6",
                            "download-settings-matrix-任务栏进度-cell-6-right",
                        ),
                    ][row_index],
                    _ => unreachable!(),
                };
                let row = window
                    .debug_bounds(row_selector)
                    .expect("window matrix row");
                let left = window
                    .debug_bounds(left_selector)
                    .expect("window matrix left cell");
                let seam = window
                    .debug_bounds(seam_selector)
                    .expect("window matrix seam");
                let right = window
                    .debug_bounds(right_selector)
                    .expect("window matrix right cell");
                assert_eq!(left.origin.y, right.origin.y);
                assert_eq!(left.size.height, right.size.height);
                assert!((left.size.width.as_f32() - right.size.width.as_f32()).abs() <= 2.0);
                assert!((seam.size.width.as_f32() - 1.0).abs() <= 0.5);
                assert!(section.contains(&row.origin));
                assert!(section.contains(&row.bottom_right()));
            }
        }
        let task_window_left_edges = [
            "download-settings-progress-window",
            "download-settings-completion-sound",
        ]
        .map(|selector| {
            let bounds = window.debug_bounds(selector).expect("task-window control");
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        let task_window_right_edges = [
            "download-settings-completion-window",
            "download-settings-completion-directory",
        ]
        .map(|selector| {
            let bounds = window.debug_bounds(selector).expect("task-window control");
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        let taskbar_left_edges = [
            "download-settings-taskbar-overlay",
            "download-settings-taskbar-anchor",
            "download-settings-taskbar-x-field",
            "download-settings-taskbar-width-field",
            "download-settings-taskbar-show-progress",
            "download-settings-taskbar-show-remaining",
            "download-settings-taskbar-show-tasks",
        ]
        .map(|selector| {
            let bounds = window.debug_bounds(selector).expect("taskbar left control");
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        let taskbar_right_edges = [
            "download-settings-taskbar-edit",
            "download-settings-taskbar-progress-mode",
            "download-settings-taskbar-y-field",
            "download-settings-taskbar-height-field",
            "download-settings-taskbar-show-speed",
            "download-settings-taskbar-show-bytes",
            "download-settings-taskbar-show-connections",
        ]
        .map(|selector| {
            let bounds = window
                .debug_bounds(selector)
                .expect("taskbar right control");
            bounds.origin.x.as_f32() + bounds.size.width.as_f32()
        });
        for edges in [
            task_window_left_edges.as_slice(),
            task_window_right_edges.as_slice(),
            taskbar_left_edges.as_slice(),
            taskbar_right_edges.as_slice(),
        ] {
            assert!(
                edges
                    .windows(2)
                    .all(|pair| (pair[0] - pair[1]).abs() <= 1.0),
                "window and taskbar controls must align to stable trailing rails"
            );
        }
        open_page!("download-settings-nav-diagnostics");
        assert_section!("诊断", "download-settings-section-诊断");

        for (label, row_selector, value_selector, actions_selector, copy_selector, open_selector) in [
            (
                "配置",
                "download-settings-path-配置",
                "download-settings-path-config-value",
                "download-settings-path-config-actions",
                "download-settings-path-config-copy",
                "download-settings-path-config-open",
            ),
            (
                "日志",
                "download-settings-path-日志",
                "download-settings-path-logs-value",
                "download-settings-path-logs-actions",
                "download-settings-path-logs-copy",
                "download-settings-path-logs-open",
            ),
            (
                "数据库",
                "download-settings-path-数据库",
                "download-settings-path-database-value",
                "download-settings-path-database-actions",
                "download-settings-path-database-copy",
                "download-settings-path-database-open",
            ),
        ] {
            let row = window
                .debug_bounds(row_selector)
                .unwrap_or_else(|| panic!("missing {label} diagnostic path"));
            assert!(
                row.size.width.as_f32() >= expected_width - 2.0,
                "{label} diagnostic row is only {}px inside a {expected_width}px section",
                row.size.width.as_f32()
            );
            let value = window
                .debug_bounds(value_selector)
                .unwrap_or_else(|| panic!("missing {label} diagnostic path value"));
            let actions = window
                .debug_bounds(actions_selector)
                .unwrap_or_else(|| panic!("missing {label} diagnostic action rail"));
            assert!(
                value.size.width.as_f32() >= expected_width * 0.7,
                "{label} long path is still constrained to a narrow column"
            );
            assert!(row.contains(&value.origin) && row.contains(&actions.bottom_right()));
            for (action_name, selector) in [("复制", copy_selector), ("打开", open_selector)] {
                let action = window
                    .debug_bounds(selector)
                    .unwrap_or_else(|| panic!("missing {label} {action_name} action"));
                assert_eq!(
                    action.size,
                    size(px(28.0), px(28.0)),
                    "{label} {action_name} must use the shared square action tier"
                );
                assert!(
                    actions.contains(&action.origin) && actions.contains(&action.bottom_right())
                );
            }
        }

        let executor = cx.executor();
        let config_value = window
            .debug_bounds("download-settings-path-config-value")
            .expect("long config path value");
        window.simulate_mouse_move(config_value.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(600));
        window.run_until_parked();
        window
            .refresh()
            .expect("render complete diagnostic path tooltip");
        assert!(
            window
                .debug_bounds("download-settings-path-config-value-tooltip")
                .is_some(),
            "the complete long path must remain readable from the production tooltip"
        );
        for (selector, tooltip_selector) in [
            (
                "download-settings-path-config-copy",
                "download-settings-path-config-copy-tooltip",
            ),
            (
                "download-settings-path-config-open",
                "download-settings-path-config-open-tooltip",
            ),
        ] {
            let action = window
                .debug_bounds(selector)
                .expect("diagnostic action with tooltip");
            window.simulate_mouse_move(action.center(), None, Modifiers::default());
            executor.advance_clock(Duration::from_millis(600));
            window.run_until_parked();
            window.refresh().expect("render diagnostic action tooltip");
            assert!(
                window.debug_bounds(tooltip_selector).is_some(),
                "diagnostic icon actions must expose bounded Chinese tooltips"
            );
        }

        for (selector, expected_path) in [
            (
                "download-settings-path-config-copy",
                config_path.to_string_lossy().into_owned(),
            ),
            (
                "download-settings-path-logs-copy",
                log_directory.to_string_lossy().into_owned(),
            ),
            (
                "download-settings-path-database-copy",
                database_path.to_string_lossy().into_owned(),
            ),
        ] {
            let action = window
                .debug_bounds(selector)
                .expect("diagnostic copy action");
            window.simulate_click(action.center(), Modifiers::default());
            window.run_until_parked();
            let envelope = platform_endpoint
                .try_receive_command_for_contract_test()
                .expect("copy-only platform command");
            assert!(
                envelope.correlation_id.is_some(),
                "diagnostic path actions must keep one operation correlation across PlatformHost"
            );
            assert_eq!(
                envelope.message,
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::PasteText(
                    nxtools_protocol::PasteTextCommand {
                        text: expected_path,
                        paste_after_write: false,
                        target_window_handle: 0,
                        target_focus_window_handle: 0,
                    }
                ))
            );
            window.update(|_, cx| {
                application.update(cx, |application, cx| {
                    application.handle_platform_notification(
                        PlatformNotification::CommandResult {
                            correlation_id: envelope.id,
                            result: PlatformCommandResult {
                                success: true,
                                error: None,
                                native_error_code: None,
                                data: None,
                            },
                        },
                        cx,
                    );
                });
            });
        }
        let notifications = cx.read(|cx| application.read(cx).notifications.clone());
        assert!(cx.read(|cx| {
            notifications
                .read(cx)
                .items()
                .iter()
                .any(|item| item.title.as_ref() == "路径已复制")
        }));

        for (selector, target, action, success) in [
            (
                "download-settings-path-config-open",
                config_path.to_string_lossy().into_owned(),
                ShellOpenAction::SelectInExplorer,
                true,
            ),
            (
                "download-settings-path-logs-open",
                log_directory.to_string_lossy().into_owned(),
                ShellOpenAction::Open,
                true,
            ),
            (
                "download-settings-path-database-open",
                database_path.to_string_lossy().into_owned(),
                ShellOpenAction::SelectInExplorer,
                false,
            ),
        ] {
            let action_bounds = window
                .debug_bounds(selector)
                .expect("diagnostic open action");
            window.simulate_click(action_bounds.center(), Modifiers::default());
            window.run_until_parked();
            let envelope = platform_endpoint
                .try_receive_command_for_contract_test()
                .expect("shell path platform command");
            assert!(
                envelope.correlation_id.is_some(),
                "diagnostic path actions must keep one operation correlation across PlatformHost"
            );
            assert_eq!(
                envelope.message,
                nxtools_protocol::Message::PlatformCommand(PlatformCommand::OpenShellTarget(
                    OpenShellTargetCommand { target, action }
                ))
            );
            window.update(|_, cx| {
                application.update(cx, |application, cx| {
                    application.handle_platform_notification(
                        PlatformNotification::CommandResult {
                            correlation_id: envelope.id,
                            result: PlatformCommandResult {
                                success,
                                error: (!success)
                                    .then(|| "sensitive path must not be logged".into()),
                                native_error_code: (!success).then_some(2),
                                data: None,
                            },
                        },
                        cx,
                    );
                });
            });
        }
        assert!(cx.read(|cx| {
            notifications
                .read(cx)
                .items()
                .iter()
                .any(|item| item.title.as_ref() == "无法打开诊断路径")
        }));

        let timeout_action = window
            .debug_bounds("download-settings-path-config-copy")
            .expect("diagnostic copy timeout action");
        window.simulate_click(timeout_action.center(), Modifiers::default());
        window.run_until_parked();
        let timed_out_envelope = platform_endpoint
            .try_receive_command_for_contract_test()
            .expect("copy command that will time out");
        executor.advance_clock(DOWNLOAD_SETTINGS_PATH_ACTION_TIMEOUT);
        window.run_until_parked();
        assert!(cx.read(|cx| {
            notifications
                .read(cx)
                .items()
                .iter()
                .any(|item| item.title.as_ref() == "无法复制诊断路径")
        }));
        window.update(|_, cx| {
            application.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: timed_out_envelope.id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: None,
                        },
                    },
                    cx,
                );
            });
        });
        assert!(cx.read(|cx| {
            notifications
                .read(cx)
                .items()
                .iter()
                .all(|item| item.title.as_ref() != "路径已复制")
        }));

        open_page!("download-settings-nav-general");
        assert_eq!(
            cx.read(|cx| settings_view.read(cx).automatic_categories_enabled),
            !initial_automatic_categories,
            "switching through all five production pages must preserve the local draft"
        );

        let apply = window
            .debug_bounds("download-settings-apply")
            .expect("production settings Apply action");
        window.simulate_click(apply.center(), Modifiers::default());
        window.run_until_parked();
        let persisted =
            AppConfig::load_from(&config_path).expect("reload request-context production settings");
        assert!(
            persisted.download.default_request_headers == long_header,
            "Apply must preserve the complete Header draft without printing its value"
        );
        assert!(
            persisted.download.default_cookie == "session=bounded",
            "Apply must preserve the Cookie draft without printing its value"
        );
        assert!(persisted.download.accept_invalid_certificates);

        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn category_settings_use_a_readable_list_and_a_labeled_full_width_directory_editor(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let executor = cx.executor();
        let (application, directory) =
            download_application_for_window_test(cx, "category-settings-layout");
        let config_path = directory.join("config.json");
        let persisted_before_actions = AppConfig::default();
        persisted_before_actions
            .save_to(&config_path)
            .expect("save category action persistence fixture");
        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_settings_window(cx);
            });
        });
        cx.run_until_parked();
        let settings_window = cx
            .read(|cx| application.read(cx).settings_window)
            .expect("production settings window");
        let mut window = VisualTestContext::from_window(settings_window, cx);
        window.run_until_parked();
        window.refresh().expect("render category settings");
        let settings_view = cx
            .read(|cx| application.read(cx).settings_view.clone())
            .expect("production settings view");
        let initial_category_ids = cx.read(|cx| {
            settings_view
                .read(cx)
                .categories
                .iter()
                .map(|category| category.id.clone())
                .collect::<Vec<_>>()
        });

        let category_item = window
            .debug_bounds("download-settings-category-0")
            .expect("first category item");
        assert_eq!(
            category_item.size.height,
            px(crate::ui::metrics::SETTINGS_ROW_MIN_HEIGHT),
            "category master rows must use the shared compact-list height"
        );
        let category_master = window
            .debug_bounds("download-settings-category-master")
            .expect("category master list");
        let first_marker = window
            .debug_bounds("download-settings-category-0-marker")
            .expect("selected category edge");
        assert_eq!(first_marker.size.width, px(2.0));
        assert!(category_master.contains(&category_item.origin));
        assert!(category_master.contains(&category_item.bottom_right()));
        assert!(category_item.contains(&first_marker.origin));
        assert!(category_item.contains(&first_marker.bottom_right()));

        let name_label = window
            .debug_bounds("download-settings-category-name-field-label")
            .expect("category name label");
        let directory_label = window
            .debug_bounds("download-settings-category-directory-field-label")
            .expect("category directory label");
        let extensions_label = window
            .debug_bounds("download-settings-category-extensions-field-label")
            .expect("category extensions label");
        let url_label = window
            .debug_bounds("download-settings-category-url-field-label")
            .expect("category URL label");
        assert_eq!(name_label.origin.y, directory_label.origin.y);
        assert_eq!(extensions_label.origin.y, url_label.origin.y);

        let name_cell = window
            .debug_bounds("download-settings-category-name-cell")
            .expect("category name cell");
        let directory_cell = window
            .debug_bounds("download-settings-category-directory-cell")
            .expect("category directory cell");
        let extensions_cell = window
            .debug_bounds("download-settings-category-extensions-cell")
            .expect("category extensions cell");
        let url_cell = window
            .debug_bounds("download-settings-category-url-cell")
            .expect("category URL cell");
        assert_eq!(name_cell.origin.y, directory_cell.origin.y);
        assert_eq!(extensions_cell.origin.y, url_cell.origin.y);
        assert!((name_cell.size.width.as_f32() - directory_cell.size.width.as_f32()).abs() <= 1.0);
        assert!((extensions_cell.size.width.as_f32() - url_cell.size.width.as_f32()).abs() <= 1.0);

        let name_field = window
            .debug_bounds("download-settings-category-name-field")
            .expect("labeled category name field");
        let directory_field = window
            .debug_bounds("download-settings-category-directory-field")
            .expect("category directory field");
        let extensions_field = window
            .debug_bounds("download-settings-category-extensions-field")
            .expect("category extensions field");
        let url_field = window
            .debug_bounds("download-settings-category-url-field")
            .expect("category URL field");
        for field in [name_field, directory_field, extensions_field, url_field] {
            assert_eq!(
                field.size.height,
                px(CONTROL_HEIGHT_INPUT),
                "all category editor fields must use the shared 32px input tier"
            );
        }
        for (cell, description_selector) in [
            (
                directory_cell,
                "download-settings-category-directory-field-description",
            ),
            (
                extensions_cell,
                "download-settings-category-extensions-field-description",
            ),
            (url_cell, "download-settings-category-url-field-description"),
        ] {
            let description = window
                .debug_bounds(description_selector)
                .unwrap_or_else(|| panic!("missing {description_selector}"));
            assert!(cell.contains(&description.origin));
            assert!(cell.contains(&description.bottom_right()));
        }

        let initial_presentation_revision =
            cx.read(|cx| settings_view.read(cx).category_presentation_revision);
        let presentation_root = r"C:\Users\Administrator\Downloads\NxTools-Categories";
        let long_relative_directory =
            r"Compressed\A-Long-Readable-Category-Destination-For-Tooltip";
        let (directory_input, category_name_input, category_directory_input) = cx.read(|cx| {
            let settings = settings_view.read(cx);
            let category = settings.categories.first().expect("first category editor");
            (
                settings.directory.clone(),
                category.name.clone(),
                category.save_directory.clone(),
            )
        });
        window.update(|window, cx| {
            directory_input.update(cx, |input, cx| {
                input.set_value(presentation_root, window, cx);
            });
            category_name_input.update(cx, |input, cx| {
                input.set_value("压缩与归档文件", window, cx);
            });
            category_directory_input.update(cx, |input, cx| {
                input.set_value(long_relative_directory, window, cx);
            });
        });
        window.run_until_parked();
        window
            .refresh()
            .expect("render edited category presentation");
        cx.read(|cx| {
            let settings = settings_view.read(cx);
            assert!(
                settings.category_presentation_revision >= initial_presentation_revision + 3,
                "default directory, category name and category directory changes must each invalidate the master presentation"
            );
            let presentation = settings.categories[0].presentation(presentation_root, cx);
            assert_eq!(presentation.name, "压缩与归档文件");
            assert_eq!(
                presentation.directory,
                PathBuf::from(presentation_root)
                    .join(long_relative_directory)
                    .display()
                    .to_string(),
                "relative category destinations must resolve against the current default directory"
            );
        });

        let edited_category_item = window
            .debug_bounds("download-settings-category-0")
            .expect("edited category master row");
        window.simulate_mouse_move(edited_category_item.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(600));
        window.run_until_parked();
        window.refresh().expect("render category directory tooltip");
        assert!(
            window
                .debug_bounds("download-settings-category-0-tooltip")
                .is_some(),
            "the production master row must expose the complete resolved destination through a stable tooltip"
        );

        let category_heading = window
            .debug_bounds("download-settings-category-heading")
            .expect("selected-category heading");
        let category_heading_title = window
            .debug_bounds("download-settings-category-heading-title")
            .expect("selected-category heading title");
        let enabled_toggle = window
            .debug_bounds("download-settings-category-enabled-button")
            .expect("category enabled toggle");
        assert!(category_heading.contains(&category_heading_title.origin));
        assert!(category_heading.contains(&category_heading_title.bottom_right()));
        assert!(category_heading.contains(&enabled_toggle.origin));
        assert!(category_heading.contains(&enabled_toggle.bottom_right()));

        let action_toolbar = window
            .debug_bounds("download-settings-category-action-toolbar")
            .expect("category action toolbar");
        let order_group = window
            .debug_bounds("download-settings-category-order-group")
            .expect("category order group");
        // The action buttons are gpui-component Buttons whose element ids are
        // not exposed as debug selectors; their wrapper controls own the
        // stable debug selectors and wrap each button exactly.
        let move_up_button = window
            .debug_bounds("download-settings-category-up")
            .expect("category move-up button");
        let move_down_button = window
            .debug_bounds("download-settings-category-down")
            .expect("category move-down button");
        let delete_button = window
            .debug_bounds("download-settings-category-delete")
            .expect("category delete button");
        for action in [move_up_button, move_down_button, delete_button] {
            assert_eq!(
                action.size.height,
                px(crate::ui::metrics::CONTROL_HEIGHT_ACTION),
                "category tools must use the shared 28px action tier"
            );
            assert!(action_toolbar.contains(&action.origin));
            assert!(action_toolbar.contains(&action.bottom_right()));
        }
        assert_eq!(move_up_button.origin.y, move_down_button.origin.y);
        assert!(order_group.contains(&move_up_button.origin));
        assert!(order_group.contains(&move_up_button.bottom_right()));
        assert!(order_group.contains(&move_down_button.origin));
        assert!(order_group.contains(&move_down_button.bottom_right()));
        assert!(
            delete_button.origin.x >= order_group.bottom_right().x + px(SPACING_12),
            "destructive category action must be spatially separated from ordering tools"
        );

        let action_invocations_before_disabled_click =
            cx.read(|cx| settings_view.read(cx).category_action_invocations);
        window.simulate_click(move_up_button.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            cx.read(|cx| settings_view.read(cx).category_action_invocations),
            action_invocations_before_disabled_click,
            "the edge-disabled move-up button must suppress its callback, not merely rely on a callback guard"
        );
        assert_eq!(
            cx.read(|cx| {
                settings_view
                    .read(cx)
                    .categories
                    .iter()
                    .map(|category| category.id.clone())
                    .collect::<Vec<_>>()
            }),
            initial_category_ids,
            "the first category's disabled move-up action must not mutate draft order"
        );

        window.simulate_click(enabled_toggle.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render disabled category row");
        assert!(!cx.read(|cx| settings_view.read(cx).categories[0].enabled));
        let disabled_row = window
            .debug_bounds("download-settings-category-0")
            .expect("disabled category row remains rendered");
        let disabled_title = window
            .debug_bounds("download-settings-category-0-title")
            .expect("disabled category title remains readable");
        let disabled_detail = window
            .debug_bounds("download-settings-category-0-detail")
            .expect("disabled category directory remains readable");
        assert!(disabled_row.contains(&disabled_title.origin));
        assert!(disabled_row.contains(&disabled_title.bottom_right()));
        assert!(disabled_row.contains(&disabled_detail.origin));
        assert!(disabled_row.contains(&disabled_detail.bottom_right()));

        let second_category = window
            .debug_bounds("download-settings-category-1")
            .expect("second category item");
        window.simulate_click(second_category.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render second category editor");
        // Selection moved to the clicked category (asserted through the state
        // below). Note: gpui 0.2.2 double-buffers debug_bounds across frames
        // (mem::swap), so absence assertions on removed elements are
        // unreliable; the selected edge is asserted on the clicked row only.
        assert_eq!(
            cx.read(|cx| settings_view.read(cx).selected_category),
            Some(1)
        );
        assert!(
            window
                .debug_bounds("download-settings-category-1-marker")
                .is_some(),
            "the clicked master row must own the selected edge"
        );
        assert!(
            window
                .debug_bounds("download-settings-category-directory-field")
                .is_some(),
            "switching categories must keep the selected category's directory editor visible"
        );

        let move_down = window
            .debug_bounds("download-settings-category-down-control")
            .expect("category move-down action");
        window.simulate_click(move_down.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render category after moving down");
        assert_eq!(
            cx.read(|cx| {
                settings_view
                    .read(cx)
                    .categories
                    .iter()
                    .map(|category| category.id.clone())
                    .collect::<Vec<_>>()
            }),
            [
                initial_category_ids[0].clone(),
                initial_category_ids[2].clone(),
                initial_category_ids[1].clone(),
            ]
            .into_iter()
            .chain(initial_category_ids.iter().skip(3).cloned())
            .collect::<Vec<_>>(),
            "clicking move-down must update the production settings draft order"
        );

        let move_up = window
            .debug_bounds("download-settings-category-up-control")
            .expect("category move-up action");
        window.simulate_click(move_up.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render category after moving up");
        assert_eq!(
            cx.read(|cx| {
                settings_view
                    .read(cx)
                    .categories
                    .iter()
                    .map(|category| category.id.clone())
                    .collect::<Vec<_>>()
            }),
            initial_category_ids,
            "clicking move-up must restore the production settings draft order"
        );

        let deleted_category_id = initial_category_ids[1].clone();
        let delete = window
            .debug_bounds("download-settings-category-delete-control")
            .expect("category delete action");
        let delete_button_before_hover = window
            .debug_bounds("download-settings-category-delete")
            .expect("category delete button before hover");
        window.simulate_mouse_move(
            delete_button_before_hover.center(),
            None,
            Modifiers::default(),
        );
        window.run_until_parked();
        window.refresh().expect("render category delete hover");
        assert_eq!(
            window
                .debug_bounds("download-settings-category-delete")
                .expect("category delete button after hover"),
            delete_button_before_hover,
            "danger hover must not move or resize the category delete action"
        );
        window.simulate_click(delete.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render category after delete");
        cx.read(|cx| {
            let settings = settings_view.read(cx);
            assert_eq!(settings.categories.len(), initial_category_ids.len() - 1);
            assert!(
                settings
                    .categories
                    .iter()
                    .all(|category| category.id != deleted_category_id),
                "clicking delete must remove the selected category from the production draft"
            );
        });
        assert_eq!(
            AppConfig::load_from(&config_path)
                .expect("reload category settings before the Apply boundary"),
            persisted_before_actions,
            "category state, order and deletion actions must remain local draft changes until Apply"
        );
        let expected_categories_after_apply = cx.read(|cx| {
            settings_view
                .read(cx)
                .categories
                .iter()
                .map(|category| category.settings(cx))
                .collect::<Vec<_>>()
        });

        for (name, selector) in [
            ("directory picker", "download-settings-pick-directory"),
            ("category add", "download-settings-category-add-button"),
            ("category move up", "download-settings-category-up"),
            ("category move down", "download-settings-category-down"),
            ("category delete", "download-settings-category-delete"),
            ("settings cancel", "download-settings-cancel-button"),
            ("settings apply", "download-settings-apply-button"),
        ] {
            let bounds = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing {name} control"));
            assert_eq!(
                bounds.size.height,
                px(crate::ui::metrics::CONTROL_HEIGHT_ACTION),
                "{name} must use the shared action tier"
            );
        }
        assert_eq!(
            window
                .debug_bounds("download-settings-category-enabled-button")
                .expect("category enabled toggle")
                .size
                .height,
            px(crate::ui::metrics::CONTROL_HEIGHT_DEFAULT),
            "settings toggles keep their compact shared tier"
        );

        let directory_picker = window
            .debug_bounds("download-settings-pick-directory")
            .expect("directory picker button");
        window.simulate_mouse_move(directory_picker.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(600));
        window.run_until_parked();
        window.refresh().expect("render directory picker tooltip");
        assert!(
            window
                .debug_bounds("download-settings-pick-directory-tooltip")
                .is_some(),
            "the production directory picker exposes its Chinese tooltip"
        );

        let transfer_navigation = window
            .debug_bounds("download-settings-nav-transfer")
            .expect("transfer navigation");
        window.simulate_click(transfer_navigation.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render transfer action sizes");
        for (name, selector) in [
            ("FFmpeg picker", "download-settings-pick-ffmpeg-button"),
            (
                "FFmpeg auto-download",
                "download-settings-ffmpeg-install-button",
            ),
            ("Gyan source", "download-settings-ffmpeg-gyan-button"),
            ("BtbN source", "download-settings-ffmpeg-btbn-button"),
        ] {
            let action = window
                .debug_bounds(selector)
                .unwrap_or_else(|| panic!("missing {name} action"));
            assert_eq!(
                action.size.height,
                px(crate::ui::metrics::CONTROL_HEIGHT_ACTION),
                "{name} must use the shared action tier"
            );
        }
        let ffmpeg_picker = window
            .debug_bounds("download-settings-pick-ffmpeg-button")
            .expect("FFmpeg picker button");
        window.simulate_mouse_move(ffmpeg_picker.center(), None, Modifiers::default());
        executor.advance_clock(Duration::from_millis(600));
        window.run_until_parked();
        window.refresh().expect("render FFmpeg picker tooltip");
        assert!(
            window
                .debug_bounds("download-settings-pick-ffmpeg-tooltip")
                .is_some(),
            "the production FFmpeg picker exposes its Chinese tooltip"
        );

        let apply = window
            .debug_bounds("download-settings-apply")
            .expect("settings Apply action after category draft edits");
        window.simulate_click(apply.center(), Modifiers::default());
        window.run_until_parked();
        assert_eq!(
            AppConfig::load_from(&config_path)
                .expect("reload category settings after the Apply boundary")
                .download
                .categories,
            expected_categories_after_apply,
            "Apply must persist the exact enabled/order/deletion state from the category draft"
        );

        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn settings_auto_download_cancel_click_reaches_the_operation_control(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) =
            download_application_for_window_test(cx, "ffmpeg-auto-install-cancel");
        AppConfig::default()
            .save_to(&directory.join("config.json"))
            .expect("save FFmpeg cancel config fixture");
        let (progress_sender, progress) = async_channel::bounded(1);
        let (completion_sender, completion) = async_channel::bounded(1);
        let cancelled = Rc::new(Cell::new(false));
        let cancellation_assertion = Rc::clone(&cancelled);
        cx.update(|cx| {
            application.update(cx, |application, _| {
                application.ffmpeg_install_operation_override =
                    Some(FfmpegInstallOperationOverride {
                        correlation_id: uuid::Uuid::new_v4(),
                        progress,
                        completion,
                        cancelled,
                    });
            });
        });
        let application_for_window = application.clone();
        let initial = cx.read(|cx| application.read(cx).settings_view_initial());
        let (_settings, window) = cx.add_window_view(move |window, cx| {
            DownloadSettingsView::new(application_for_window, initial, window, cx)
        });
        window.refresh().expect("render FFmpeg cancel settings");
        window.run_until_parked();
        let transfer_navigation = window
            .debug_bounds("download-settings-nav-transfer")
            .expect("transfer settings navigation");
        window.simulate_click(transfer_navigation.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render FFmpeg cancel section");
        let install = window
            .debug_bounds("download-settings-ffmpeg-install")
            .expect("FFmpeg auto-download button");
        window.simulate_click(install.center(), Modifiers::default());
        window.run_until_parked();
        let cancel = window
            .debug_bounds("download-settings-ffmpeg-cancel")
            .expect("FFmpeg auto-download cancel button");
        window.simulate_click(cancel.center(), Modifiers::default());
        window.run_until_parked();

        assert!(cancellation_assertion.get());
        cx.read(|cx| {
            assert!(matches!(
                application.read(cx).ffmpeg_install_state,
                FfmpegInstallViewState::Installing {
                    cancelling: true,
                    ..
                }
            ));
        });
        drop(progress_sender);
        drop(completion_sender);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[gpui::test]
    fn settings_window_routes_picker_applies_and_cancels_without_cross_module_loss(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let directory = unique_download_settings_test_directory("window");
        let selected_directory = directory.join("selected");
        std::fs::create_dir_all(&selected_directory).expect("create selected directory");
        let ffmpeg_executable = directory.join("ffmpeg-user-selected.exe");
        std::fs::write(&ffmpeg_executable, b"fixture").expect("write FFmpeg picker fixture");
        let config_path = directory.join("config.json");
        let mut config = AppConfig::default();
        config.quick_launch.enabled = false;
        config.save_to(&config_path).expect("save settings fixture");
        let engine = DownloadEngine::open_in_memory().expect("open settings engine");
        let engine_for_assertion = engine.clone();
        let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let application = cx.update(|cx| {
            cx.new(|cx| {
                DownloadApplication::new(
                    engine,
                    Rc::new(platform),
                    notifications,
                    DownloadRuntimeContext {
                        fallback_default_directory: directory.clone(),
                        config,
                        config_path: config_path.clone(),
                        log_directory: directory.clone(),
                        database_path: directory.join("downloads.sqlite3"),
                        ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        let application_for_window = application.clone();
        let settings_initial = cx.read(|cx| application.read(cx).settings_view_initial());
        let (settings, window) = cx.add_window_view(move |window, cx| {
            DownloadSettingsView::new(application_for_window, settings_initial, window, cx)
        });
        window.refresh().expect("render download settings window");
        window.run_until_parked();
        assert!(window.debug_bounds("download-settings-scroll").is_some());
        window.update(|window, cx| {
            settings.update(cx, |settings, cx| {
                settings.default_request_headers.update(cx, |input, cx| {
                    input.set_value(
                        "X-NxTools-Default: desktop\nReferer: https://example.test/",
                        window,
                        cx,
                    );
                });
                settings.default_cookie.update(cx, |input, cx| {
                    input.set_value("session=desktop", window, cx);
                });
            });
        });
        let request_navigation = window
            .debug_bounds("download-settings-nav-request")
            .expect("request settings navigation");
        window.simulate_click(request_navigation.center(), Modifiers::default());
        window.run_until_parked();
        let certificate_toggle = window
            .debug_bounds("download-settings-accept-invalid-certificates")
            .expect("certificate policy toggle");
        window.simulate_click(certificate_toggle.center(), Modifiers::default());
        let general_navigation = window
            .debug_bounds("download-settings-nav-general")
            .expect("general settings navigation");
        window.simulate_click(general_navigation.center(), Modifiers::default());
        window.run_until_parked();
        let automatic_categories = window
            .debug_bounds("download-settings-automatic-categories")
            .expect("automatic category toggle");
        window.simulate_click(automatic_categories.center(), Modifiers::default());
        let reject_duplicates = window
            .debug_bounds("download-settings-duplicate-reject")
            .expect("reject duplicate strategy");
        window.simulate_click(reject_duplicates.center(), Modifiers::default());
        let first_category_enabled = window
            .debug_bounds("download-settings-category-enabled-button")
            .expect("selected category enable control");
        window.simulate_click(first_category_enabled.center(), Modifiers::default());
        let initial_category_count = window.read(|cx| settings.read(cx).categories.len());
        let add_category = window
            .debug_bounds("download-settings-category-add")
            .expect("add category control");
        window.simulate_click(add_category.center(), Modifiers::default());
        window.run_until_parked();
        let custom_category_id = window.read(|cx| {
            let settings = settings.read(cx);
            assert_eq!(settings.categories.len(), initial_category_count + 1);
            settings.categories.last().expect("new category").id.clone()
        });
        window.update(|window, cx| {
            settings.update(cx, |settings, cx| {
                let category = settings.categories.last().expect("new category editor");
                category
                    .name
                    .update(cx, |input, cx| input.set_value("镜像包", window, cx));
                category
                    .save_directory
                    .update(cx, |input, cx| input.set_value("Mirrors", window, cx));
                category
                    .extensions
                    .update(cx, |input, cx| input.set_value("img, iso", window, cx));
                category.url_patterns.update(cx, |input, cx| {
                    input.set_value("https://mirror.example/*", window, cx)
                });
            });
        });
        let original_directory =
            window.read(|cx| settings.read(cx).directory.read(cx).value().to_string());
        window.update(|_, cx| {
            application.update(cx, |_, cx| {
                cx.emit(DownloadApplicationEvent::DirectoryPicked {
                    target: DownloadDirectoryTarget::NewTask,
                    path: directory.join("wrong-target"),
                });
            });
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| settings.read(cx).directory.read(cx).value().to_string()),
            original_directory,
            "a new-task picker result must not mutate the settings draft"
        );
        window.update(|_, cx| {
            application.update(cx, |_, cx| {
                cx.emit(DownloadApplicationEvent::DirectoryPicked {
                    target: DownloadDirectoryTarget::Settings,
                    path: selected_directory.clone(),
                });
            });
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| settings.read(cx).directory.read(cx).value().to_string()),
            selected_directory.display().to_string()
        );
        let transfer_navigation = window
            .debug_bounds("download-settings-nav-transfer")
            .expect("transfer settings navigation");
        window.simulate_click(transfer_navigation.center(), Modifiers::default());
        window.run_until_parked();
        window.refresh().expect("render media tools section");
        let ffmpeg_picker = window
            .debug_bounds("download-settings-pick-ffmpeg")
            .expect("FFmpeg executable picker button");
        window.simulate_click(ffmpeg_picker.center(), Modifiers::default());
        window.run_until_parked();
        let picker_command_id = window.read(|cx| {
            let application = application.read(cx);
            let commands = application.ffmpeg_platform_commands.borrow();
            let (command_id, command) = commands.last().expect("FFmpeg picker command");
            assert!(matches!(command, PlatformCommand::PickProgram(_)));
            *command_id
        });
        window.update(|_, cx| {
            application.update(cx, |application, cx| {
                application.handle_platform_notification(
                    PlatformNotification::CommandResult {
                        correlation_id: picker_command_id,
                        result: PlatformCommandResult {
                            success: true,
                            error: None,
                            native_error_code: None,
                            data: Some(ffmpeg_executable.display().to_string()),
                        },
                    },
                    cx,
                );
            });
        });
        window.run_until_parked();
        assert_eq!(
            window.read(|cx| {
                settings
                    .read(cx)
                    .ffmpeg_executable
                    .read(cx)
                    .value()
                    .to_string()
            }),
            ffmpeg_executable.display().to_string()
        );
        for (button, expected_source, expected_url) in [
            (
                "download-settings-ffmpeg-gyan",
                FfmpegDownloadSource::Gyan,
                FFMPEG_GYAN_DOWNLOAD_URL,
            ),
            (
                "download-settings-ffmpeg-btbn",
                FfmpegDownloadSource::Btbn,
                FFMPEG_BTBN_DOWNLOAD_URL,
            ),
        ] {
            let bounds = window
                .debug_bounds(button)
                .expect("public FFmpeg download button");
            window.simulate_click(bounds.center(), Modifiers::default());
            window.run_until_parked();
            let command_id = window.read(|cx| {
                let application = application.read(cx);
                let commands = application.ffmpeg_platform_commands.borrow();
                let (command_id, command) = commands.last().expect("FFmpeg page command");
                assert_eq!(
                    command,
                    &PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                        target: expected_url.into(),
                        action: ShellOpenAction::Open,
                    })
                );
                assert_eq!(
                    application
                        .pending_ffmpeg_operations
                        .get(command_id)
                        .map(|pending| pending.kind),
                    Some(FfmpegPlatformOperationKind::DownloadPage(expected_source))
                );
                *command_id
            });
            window.update(|_, cx| {
                application.update(cx, |application, cx| {
                    application.handle_platform_notification(
                        PlatformNotification::CommandResult {
                            correlation_id: command_id,
                            result: PlatformCommandResult {
                                success: true,
                                error: None,
                                native_error_code: None,
                                data: None,
                            },
                        },
                        cx,
                    );
                });
            });
        }
        let window_navigation = window
            .debug_bounds("download-settings-nav-window")
            .expect("window settings navigation");
        window.simulate_click(window_navigation.center(), Modifiers::default());
        window.run_until_parked();
        window.update(|_, cx| {
            settings.update(cx, |settings, cx| {
                settings.scroll.scroll_to_bottom();
                cx.notify();
            });
        });
        window.run_until_parked();
        let completion_toggle = window
            .debug_bounds("download-settings-completion-window")
            .expect("completion setting toggle");
        window.simulate_click(completion_toggle.center(), Modifiers::default());
        let sound_toggle = window
            .debug_bounds("download-settings-completion-sound")
            .expect("completion sound toggle");
        window.simulate_click(sound_toggle.center(), Modifiers::default());
        let directory_toggle = window
            .debug_bounds("download-settings-completion-directory")
            .expect("completion directory toggle");
        window.simulate_click(directory_toggle.center(), Modifiers::default());
        window.run_until_parked();
        let taskbar_overlay_toggle = window
            .debug_bounds("download-settings-taskbar-overlay")
            .expect("taskbar overlay setting toggle");
        window.simulate_click(taskbar_overlay_toggle.center(), Modifiers::default());
        let taskbar_left = window
            .debug_bounds("download-settings-taskbar-left")
            .expect("taskbar overlay left anchor");
        window.simulate_click(taskbar_left.center(), Modifiers::default());
        let apply = window
            .debug_bounds("download-settings-apply")
            .expect("settings apply button");
        window.simulate_click(apply.center(), Modifiers::default());
        window.run_until_parked();

        let persisted = AppConfig::load_from(&config_path).expect("reload applied settings");
        assert_eq!(
            persisted.download.default_directory,
            selected_directory.display().to_string()
        );
        assert_eq!(
            persisted.download.ffmpeg_executable_path,
            ffmpeg_executable.display().to_string()
        );
        assert!(
            persisted.download.default_request_headers
                == "X-NxTools-Default: desktop\nReferer: https://example.test/",
            "applied Header settings must round-trip without printing their value"
        );
        assert!(
            persisted.download.default_cookie == "session=desktop",
            "applied Cookie settings must round-trip without printing their value"
        );
        assert!(persisted.download.accept_invalid_certificates);
        assert!(persisted.download.automatic_categories_enabled);
        assert_eq!(
            persisted.download.duplicate_strategy,
            DownloadDuplicateStrategySettings::Reject
        );
        assert!(!persisted.download.categories[0].enabled);
        let custom_category = persisted
            .download
            .categories
            .iter()
            .find(|category| category.id == custom_category_id)
            .expect("custom category persisted");
        assert_eq!(custom_category.name, "镜像包");
        assert_eq!(custom_category.save_directory, "Mirrors");
        assert_eq!(custom_category.extensions, ["img", "iso"]);
        assert_eq!(custom_category.url_patterns, ["https://mirror.example/*"]);
        assert_eq!(
            engine_for_assertion
                .ffmpeg_resolution()
                .expect("resolve applied FFmpeg")
                .expect("applied FFmpeg should be available")
                .executable,
            ffmpeg_executable
        );
        assert!(!persisted.download.show_completion_window);
        assert!(!persisted.download.completion_sound_enabled);
        assert!(persisted.download.open_directory_on_completion);
        assert!(!persisted.download.taskbar_progress_overlay.enabled);
        assert_eq!(
            persisted.download.taskbar_progress_overlay.anchor,
            DownloadTaskbarOverlayAnchor::Left
        );
        assert!(
            !persisted.quick_launch.enabled,
            "applying download settings must preserve current quick-launch settings"
        );

        let application_for_cancel = application.clone();
        let cancel_initial = cx.read(|cx| application.read(cx).settings_view_initial());
        let (_cancel_view, cancel_window) = cx.add_window_view(move |window, cx| {
            DownloadSettingsView::new(application_for_cancel, cancel_initial, window, cx)
        });
        cancel_window.read(|cx| {
            let view = _cancel_view.read(cx);
            assert!(
                view.default_request_headers.read(cx).value()
                    == "X-NxTools-Default: desktop\nReferer: https://example.test/",
                "reopened Header settings must match without printing their value"
            );
            assert!(
                view.default_cookie.read(cx).value() == "session=desktop",
                "reopened Cookie settings must match without printing their value"
            );
            assert!(view.accept_invalid_certificates);
            assert!(view.automatic_categories_enabled);
            assert_eq!(
                view.duplicate_strategy,
                DownloadDuplicateStrategySettings::Reject
            );
            assert!(!view.categories[0].enabled);
            assert!(!view.completion_sound_enabled);
            assert!(view.open_directory_on_completion);
        });
        cancel_window
            .refresh()
            .expect("render cancelled settings draft");
        let window_navigation = cancel_window
            .debug_bounds("download-settings-nav-window")
            .expect("window settings navigation");
        cancel_window.simulate_click(window_navigation.center(), Modifiers::default());
        cancel_window.run_until_parked();
        let progress_toggle = cancel_window
            .debug_bounds("download-settings-progress-window")
            .expect("progress setting toggle");
        cancel_window.simulate_click(progress_toggle.center(), Modifiers::default());
        let cancel = cancel_window
            .debug_bounds("download-settings-cancel")
            .expect("settings cancel button");
        cancel_window.simulate_click(cancel.center(), Modifiers::default());
        cancel_window.run_until_parked();
        assert!(
            AppConfig::load_from(&config_path)
                .expect("reload cancelled settings")
                .download
                .show_progress_window,
            "cancel must discard the local settings draft"
        );
        std::fs::remove_dir_all(directory).expect("remove settings fixture");
    }

    #[gpui::test]
    fn settings_persistence_failure_rolls_back_engine_values(cx: &mut TestAppContext) {
        let directory = unique_download_settings_test_directory("rollback");
        std::fs::create_dir_all(&directory).expect("create rollback directory");
        let missing_config = directory.join("missing").join("config.json");
        let engine = DownloadEngine::open_in_memory().expect("open rollback engine");
        let engine_for_assertion = engine.clone();
        let mut initial_config = AppConfig::default();
        initial_config.download.default_request_headers = "X-NxTools-Policy: persisted".into();
        initial_config.download.default_cookie = "persisted=1".into();
        let (platform, _platform_endpoint) = nxtools_ipc::platform_bridge(16);
        let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
        let application = cx.update(|cx| {
            cx.new(|cx| {
                DownloadApplication::new(
                    engine,
                    Rc::new(platform),
                    notifications,
                    DownloadRuntimeContext {
                        fallback_default_directory: directory.clone(),
                        config: initial_config,
                        config_path: missing_config,
                        log_directory: directory.clone(),
                        database_path: directory.join("downloads.sqlite3"),
                        ffmpeg_install_directory: directory.join("tools").join("ffmpeg"),
                    },
                    cx,
                )
            })
        });
        let result = cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.apply_settings(
                    DownloadSettingsDraft {
                        default_directory: directory.clone(),
                        automatic_categories_enabled: true,
                        categories: DownloadSettings::default().categories,
                        duplicate_strategy: DownloadDuplicateStrategySettings::AddNumbered,
                        double_click_action: DownloadRowDoubleClickActionSettings::OpenFile,
                        task_list_sort: None,
                        task_list_sort_descending: None,
                        ffmpeg_executable_path: String::new(),
                        default_request_headers: "X-NxTools-Policy: tentative".into(),
                        default_cookie: "tentative=1".into(),
                        accept_invalid_certificates: true,
                        default_connection_limit: 12,
                        adaptive_segment_scaling_enabled: true,
                        adaptive_segment_max_segments: 32,
                        adaptive_segment_min_mib: 3,
                        max_concurrent_tasks: 7,
                        global_speed_limit_bytes_per_second: Some(1024 * 1024),
                        show_progress_window: false,
                        show_completion_window: false,
                        completion_sound_enabled: true,
                        open_directory_on_completion: false,
                        taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings::default(),
                    },
                    uuid::Uuid::new_v4(),
                    cx,
                )
            })
        });
        assert!(result.is_err());
        assert_eq!(engine_for_assertion.global_speed_limit(), None);
        let task_id = engine_for_assertion
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/file.bin".into(),
                    save_directory: directory.clone(),
                    save_directory_explicit: false,
                    file_name: Some("policy.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("accept request after persistence failure");
        let accepted = engine_for_assertion
            .task(task_id)
            .expect("query accepted policy task")
            .expect("policy task exists");
        assert!(
            accepted.task.request().cookie.as_deref() == Some("persisted=1"),
            "rollback must keep the prior Cookie without printing its value"
        );
        assert_eq!(
            accepted.task.request().certificate_policy,
            DownloadCertificatePolicy::Strict
        );
        assert!(accepted.task.request().headers.iter().any(|header| {
            header.name.eq_ignore_ascii_case("x-nxtools-policy") && header.value == "persisted"
        }));
        let snapshot = engine_for_assertion.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Internal,
            command: DownloadCommand::Snapshot {
                query: DownloadQuery::default(),
            },
        });
        let DownloadCommandOutcome::Snapshot(snapshot) = snapshot.outcome else {
            panic!("expected rollback snapshot");
        };
        assert_eq!(
            snapshot
                .queues
                .iter()
                .find(|queue| queue.id == "default")
                .and_then(|queue| queue.max_concurrent_tasks),
            Some(3)
        );
        std::fs::remove_dir_all(directory).expect("remove rollback fixture");
    }

    #[test]
    fn queue_only_closes_the_add_window_without_opening_a_progress_window() {
        let task_id = DownloadTaskId::new();

        assert_eq!(
            submit_presentation_plan(DownloadSubmitMode::QueueOnly, task_id, false, true),
            DownloadSubmitPresentationPlan {
                close_source_window: true,
                deferred_task_window: None,
            }
        );
    }

    #[test]
    fn delayed_submission_closes_the_add_window_without_opening_progress_early() {
        let task_id = DownloadTaskId::new();

        assert_eq!(
            submit_presentation_plan(DownloadSubmitMode::StartNow, task_id, true, true),
            DownloadSubmitPresentationPlan {
                close_source_window: true,
                deferred_task_window: None,
            }
        );
    }

    #[test]
    fn daily_schedule_draft_preserves_independent_triggers_and_weekday_order() {
        let days = [DownloadWeekday::Friday, DownloadWeekday::Monday]
            .into_iter()
            .collect();
        let schedule = build_download_schedule(true, false, &days, "22:30", "invalid")
            .expect("build start-only schedule")
            .expect("enabled schedule");
        assert!(schedule.daily.enabled_start);
        assert!(!schedule.daily.enabled_stop);
        assert_eq!(schedule.daily.start_minute, 22 * 60 + 30);
        assert_eq!(
            schedule.daily.days,
            vec![DownloadWeekday::Monday, DownloadWeekday::Friday]
        );
    }

    #[test]
    fn daily_schedule_draft_rejects_missing_days_and_invalid_enabled_time() {
        let no_days = HashSet::new();
        assert_eq!(
            build_download_schedule(true, true, &no_days, "09:00", "17:00"),
            Err("至少选择一个生效星期".into())
        );
        let weekdays = [DownloadWeekday::Monday].into_iter().collect();
        assert_eq!(
            build_download_schedule(false, true, &weekdays, "ignored", "24:00"),
            Err("停止时间：时间必须在 00:00 到 23:59 之间".into())
        );
        assert_eq!(
            build_download_schedule(false, false, &no_days, "ignored", "ignored"),
            Ok(None)
        );
    }

    #[test]
    fn optional_delayed_start_uses_selected_local_date_and_time() {
        let now = Local::now();
        assert_eq!(
            resolve_scheduled_start(None, now.timestamp_millis()),
            Ok(None),
            "no committed selection means an immediate download"
        );

        let future = now + ChronoDuration::days(2);
        let scheduled = resolve_scheduled_start(
            Some(DelayedStartSelection {
                date: future.date_naive(),
                hour: future.hour(),
                minute: future.minute(),
            }),
            now.timestamp_millis(),
        )
        .expect("future delayed start")
        .expect("selected date creates a delayed start");
        assert!(scheduled > now.timestamp_millis());

        assert_eq!(
            resolve_scheduled_start(
                Some(DelayedStartSelection {
                    date: (now - ChronoDuration::days(1)).date_naive(),
                    hour: 12,
                    minute: 0,
                }),
                now.timestamp_millis(),
            ),
            Err("计划时间必须晚于当前时间".into())
        );
    }

    #[test]
    fn delayed_start_calendar_uses_chinese_year_month_and_a_stable_sunday_grid() {
        let july = NaiveDate::from_ymd_opt(2026, 7, 1).expect("valid month");
        let days = delayed_start_calendar_days(july);

        assert_eq!(delayed_start_calendar_title(july), "2026年7月");
        assert_eq!(days.len(), 42);
        assert_eq!(days.first().copied(), NaiveDate::from_ymd_opt(2026, 6, 28));
        assert_eq!(days.last().copied(), NaiveDate::from_ymd_opt(2026, 8, 8));
        assert_eq!(
            shift_calendar_month(july, -7),
            NaiveDate::from_ymd_opt(2025, 12, 1).expect("valid previous month")
        );
        assert_eq!(
            shift_calendar_month(july, 6),
            NaiveDate::from_ymd_opt(2027, 1, 1).expect("valid next month")
        );
    }

    #[test]
    fn a_new_active_cli_task_opens_one_desktop_progress_window() {
        for status in [
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
            DownloadStatus::Finalizing,
            DownloadStatus::Verifying,
        ] {
            assert!(should_open_admitted_progress_window(
                false,
                false,
                RequestOrigin::Cli,
                status
            ));
        }
        assert!(!should_open_admitted_progress_window(
            true,
            false,
            RequestOrigin::Cli,
            DownloadStatus::Downloading
        ));
        assert!(!should_open_admitted_progress_window(
            false,
            false,
            RequestOrigin::Cli,
            DownloadStatus::Created
        ));
        assert!(!should_open_admitted_progress_window(
            false,
            false,
            RequestOrigin::Ui,
            DownloadStatus::Downloading
        ));
        assert!(should_open_admitted_progress_window(
            true,
            true,
            RequestOrigin::Ui,
            DownloadStatus::Downloading
        ));
    }

    #[test]
    fn removal_waits_for_the_single_task_removed_refresh() {
        assert!(
            !refresh_immediately_after_remove(),
            "the initiating action and TaskRemoved must not both rebuild the list"
        );
    }

    #[test]
    fn add_window_submission_never_opens_a_gpui_window_while_the_view_is_leased() {
        let source = include_str!("downloads.rs");
        let application_submit = source
            .split("    pub(crate) fn submit(")
            .nth(1)
            .and_then(|body| body.split("    pub(crate) fn pause(").next())
            .expect("DownloadApplication::submit body");
        let view_submit = source
            .split("    fn submit(&mut self, mode:")
            .nth(1)
            .and_then(|body| body.split("\nimpl Render for NewDownloadView").next())
            .expect("NewDownloadView::submit body");

        assert!(
            !application_submit.contains("request_task_window("),
            "application submission must remain presentation-free"
        );
        assert!(
            view_submit.contains("cx.defer(move |cx|"),
            "cross-entity submission must leave the current GPUI entity callback"
        );
        let close = view_submit
            .find("source_window.update")
            .expect("source add window is closed after acceptance");
        let deferred_open = view_submit[close..]
            .find("cx.defer(move |cx|")
            .expect("task window opening is scheduled after source close");
        assert!(deferred_open > 0);
    }

    #[test]
    fn task_window_render_never_reads_the_parent_download_application() {
        let source = include_str!("downloads/task_window.rs");
        let task_window_render = source
            .split("impl Render for DownloadTaskWindowView")
            .nth(1)
            .and_then(|body| body.split("\nfn download_window_titlebar").next())
            .expect("DownloadTaskWindowView render body");

        assert!(
            !task_window_render.contains(".application.read(cx)"),
            "task windows opened or refreshed by DownloadApplication must render from pushed snapshots"
        );
    }

    #[test]
    fn task_windows_use_compact_content_driven_phase_sizes() {
        let progress = download_task_window_size(DownloadWindowPhase::Progress);
        let failed = download_task_window_size(DownloadWindowPhase::Failed);
        let completed = download_task_window_size(DownloadWindowPhase::Completed);

        assert_eq!(
            (progress.width.as_f32(), progress.height.as_f32()),
            (440.0, 184.0)
        );
        assert_eq!(
            (failed.width.as_f32(), failed.height.as_f32()),
            (440.0, 196.0)
        );
        assert_eq!(
            (completed.width.as_f32(), completed.height.as_f32()),
            (440.0, 180.0)
        );
    }

    #[test]
    fn task_window_phase_resize_preserves_the_physical_window_center() {
        let scale_factor = 1.5;
        let progress_size = download_task_window_size(DownloadWindowPhase::Progress);
        let progress_bounds = gpui::Bounds::new(gpui::point(px(100.0), px(200.0)), progress_size);
        let expected_physical_center = (480.0, 438.0);

        for phase in [DownloadWindowPhase::Failed, DownloadWindowPhase::Completed] {
            let command = centered_task_window_bounds_command(
                42,
                progress_bounds,
                download_task_window_size(phase),
                scale_factor,
            );
            assert_eq!(command.window_handle, 42);
            assert_eq!(
                (
                    f64::from(command.x) + f64::from(command.width) / 2.0,
                    f64::from(command.y) + f64::from(command.height) / 2.0,
                ),
                expected_physical_center
            );
        }
    }

    #[test]
    fn reopening_a_task_window_uses_the_native_restore_and_foreground_boundary() {
        let source = include_str!("downloads.rs");
        let update_task_window = source
            .split("    fn update_task_window(")
            .nth(1)
            .and_then(|body| {
                body.split("    pub(crate) fn handle_platform_notification")
                    .next()
            })
            .expect("DownloadApplication::update_task_window body");

        assert!(
            update_task_window.contains("queue_task_window_activation"),
            "native activation must be tracked for bounded terminal diagnostics"
        );
        assert_eq!(task_window_activation_command(0), None);
        assert!(matches!(
            task_window_activation_command(42),
            Some(PlatformCommand::ActivateWindow(ActivateWindowCommand {
                window_handle: 42
            }))
        ));
    }

    #[test]
    fn a_late_activation_result_cannot_replace_a_new_window_generation() {
        let stale = TaskWindowIdentity {
            native_window_handle: 42,
            generation: uuid::Uuid::new_v4(),
        };
        let replacement = TaskWindowIdentity {
            native_window_handle: 42,
            generation: uuid::Uuid::new_v4(),
        };

        assert!(replacement_targets_current_window(Some(stale), stale));
        assert!(
            !replacement_targets_current_window(Some(replacement), stale),
            "even a recycled HWND must not let an old activation result remove the new window"
        );
        assert!(!replacement_targets_current_window(None, stale));
    }

    #[test]
    fn stale_window_cleanup_discards_only_that_tasks_inflight_activation() {
        let stale_task = DownloadTaskId::new();
        let other_task = DownloadTaskId::new();
        let stale_window = TaskWindowIdentity {
            native_window_handle: 42,
            generation: uuid::Uuid::new_v4(),
        };
        let other_window = TaskWindowIdentity {
            native_window_handle: 84,
            generation: uuid::Uuid::new_v4(),
        };
        let stale_activation = uuid::Uuid::new_v4();
        let other_activation = uuid::Uuid::new_v4();
        let mut activations = HashMap::from([
            (
                stale_activation,
                PendingTaskWindowActivation {
                    task_id: stale_task,
                    window: stale_window,
                    activation: TaskWindowActivation::Reused(DownloadWindowPhase::Progress),
                    started_at: Instant::now(),
                },
            ),
            (
                other_activation,
                PendingTaskWindowActivation {
                    task_id: other_task,
                    window: other_window,
                    activation: TaskWindowActivation::Reused(DownloadWindowPhase::Progress),
                    started_at: Instant::now(),
                },
            ),
        ]);
        let mut replacements =
            HashMap::from([(stale_task, stale_window), (other_task, other_window)]);

        clear_pending_task_window_work(stale_task, &mut activations, &mut replacements);

        assert!(!activations.contains_key(&stale_activation));
        assert!(!replacements.contains_key(&stale_task));
        assert!(activations.contains_key(&other_activation));
        assert_eq!(replacements.get(&other_task), Some(&other_window));
    }

    #[test]
    fn explicit_terminal_reopen_is_activating_while_automatic_fallback_is_passive() {
        let task_id = DownloadTaskId::new();
        let mut lifecycle = DownloadWindowLifecycle::default();

        assert_eq!(
            lifecycle.request_window(task_id, DownloadWindowPhase::Completed),
            DownloadWindowDirective::OpenAndFocus(DownloadWindowPhase::Completed)
        );
        lifecycle.window_closed(task_id);
        assert_eq!(
            lifecycle.reopen_terminal_fallback(task_id, DownloadWindowPhase::Completed),
            DownloadWindowDirective::Open(DownloadWindowPhase::Completed)
        );
        assert!(
            !TaskWindowActivation::Fresh(DownloadWindowPhase::Completed).replace_on_failure(),
            "a fresh window must not loop through replacements when native activation is rejected"
        );
        assert!(
            TaskWindowActivation::Reused(DownloadWindowPhase::Completed).replace_on_failure(),
            "a rejected reused-window activation must fall back to one guarded replacement"
        );
    }

    #[test]
    fn new_download_window_expands_only_after_probe_confirmation() {
        let initial = new_download_window_size(false);
        let confirmed = new_download_window_size(true);

        assert_eq!(
            (initial.width.as_f32(), initial.height.as_f32()),
            (560.0, 176.0)
        );
        assert_eq!(
            (confirmed.width.as_f32(), confirmed.height.as_f32()),
            (560.0, 278.0)
        );
    }

    #[test]
    fn confirmed_probe_keeps_destination_and_filename_editable() {
        let confirmed = DownloadRequest {
            source_url: "https://example.com/original".into(),
            save_directory: PathBuf::from(r"C:\Downloads"),
            save_directory_explicit: false,
            file_name: Some("server-name.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        let current = DownloadRequest {
            save_directory: PathBuf::from(r"D:\Archive"),
            file_name: Some("renamed.bin".into()),
            category_id: Some("documents".into()),
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::Reject,
            ..confirmed.clone()
        };

        let merged = merge_confirmed_download_request(confirmed, current)
            .expect("destination-only edits do not invalidate a network probe");

        assert_eq!(merged.save_directory, PathBuf::from(r"D:\Archive"));
        assert_eq!(merged.file_name.as_deref(), Some("renamed.bin"));
        assert_eq!(merged.category_id.as_deref(), Some("documents"));
        assert_eq!(
            merged.duplicate_strategy,
            nxtools_download::DownloadDuplicateStrategy::Reject
        );
    }

    #[test]
    fn download_titlebars_close_through_an_explicit_gpui_action() {
        let source = include_str!("downloads.rs");
        let helper_marker = ["fn download_window_", "close_button"].concat();
        let close_button = source
            .split(&helper_marker)
            .nth(1)
            .and_then(|body| body.split("\nfn download_task_status").next())
            .expect("shared download close button");

        assert!(
            close_button.contains(".on_click(|_, window, _| window.remove_window())"),
            "custom client titlebars must close explicitly instead of relying only on native hit testing"
        );
    }

    #[test]
    fn probe_completion_hydrates_the_visible_file_name_input() {
        let source = include_str!("downloads.rs");
        let begin_probe = source
            .split("    fn begin_probe(")
            .nth(1)
            .and_then(|body| body.split("\n    fn submit(").next())
            .expect("NewDownloadView::begin_probe body");

        assert!(
            begin_probe.contains("this.file_name.clone(),")
                && begin_probe.contains("resolved_file_name,")
                && begin_probe.contains("file_name.update"),
            "the resolved server filename must be written into the editable file-name field"
        );
        assert!(
            begin_probe.contains("input.set_value"),
            "the file-name input must visibly update when probing completes"
        );
    }

    #[test]
    fn hls_probe_resolves_manifest_name_to_transport_stream_artifact() {
        let probe = DownloadProbe {
            final_url: "https://example.test/high/index.m3u8".into(),
            file_name: Some("index.m3u8".into()),
            total_bytes: None,
            supports_ranges: false,
            content_type: Some("application/vnd.apple.mpegurl".into()),
            validator: nxtools_download::ResourceValidator::default(),
            media: Some(nxtools_download::DownloadMediaProbe::Hls(
                nxtools_download::HlsProbe {
                    is_master: false,
                    variants: Vec::new(),
                },
            )),
        };

        assert_eq!(
            resolved_probe_file_name(&probe).as_deref(),
            Some("index.ts")
        );
        assert!(should_replace_media_manifest_name(
            Some("index.m3u8"),
            &probe
        ));
        assert!(
            !should_replace_media_manifest_name(Some("my-custom-name.m3u8"), &probe),
            "a user-edited manifest-looking name must remain user-owned"
        );
    }
}
