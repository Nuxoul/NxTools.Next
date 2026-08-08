use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use chrono::{
    Datelike, Duration as ChronoDuration, Local, NaiveDate, NaiveTime, TimeZone, Timelike,
};
use gpui::{
    AnyElement, AnyWindowHandle, App, Context, Corner, Entity, EventEmitter, FontWeight,
    IntoElement, MouseButton, Render, ScrollHandle, Subscription, Task, Window,
    WindowBackgroundAppearance, WindowBounds, WindowControlArea, WindowDecorations, WindowKind,
    WindowOptions, div, prelude::*, px, size,
};
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::popover::Popover;
use gpui_component::scroll::ScrollableElement;
use gpui_component::select::{Select, SelectState};
use gpui_component::{Disableable, Icon, IndexPath, PixelsExt, Root, Sizable};
use nxtools_core::{
    AppConfig, DownloadSettings, DownloadTaskbarOverlayAnchor, DownloadTaskbarProgressMode,
    DownloadTaskbarProgressOverlaySettings,
};
use nxtools_download::{
    BrowserDownloadDialogRequest, DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadBatchAction,
    DownloadCommand, DownloadCommandEnvelope, DownloadCommandOutcome, DownloadDailySchedule,
    DownloadEngine, DownloadEngineTask, DownloadEvent, DownloadEventEnvelope, DownloadProbe,
    DownloadProgress, DownloadQuery, DownloadQueue, DownloadQueueAction, DownloadQueuePlacement,
    DownloadRequest, DownloadSchedule, DownloadSegment, DownloadSegmentState, DownloadSnapshot,
    DownloadStatus, DownloadSubmitMode, DownloadTaskAction, DownloadTaskDetail, DownloadTaskId,
    DownloadWeekday, RequestOrigin,
};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{
    ActivateWindowCommand, BeginFileDragCommand, EnsureBrowserNativeMessagingCommand,
    OpenShellTargetCommand, PickDirectoryCommand, PlaceTaskbarProgressOverlayCommand,
    PlatformCommand, PlatformCommandResult, PlatformEvent, SetTaskbarProgressCommand,
    SetWindowBoundsCommand, SetWindowTopmostCommand, ShellOpenAction, TaskbarOverlayAnchor,
    TaskbarProgressState,
};
use tracing::{error, info, warn};

use crate::download_taskbar_overlay::{
    DownloadTaskbarOverlayModel, DownloadTaskbarOverlaySample, DownloadTaskbarOverlaySegmentSample,
    DownloadTaskbarOverlayView, aggregate_taskbar_progress,
};
use crate::download_window_lifecycle::{
    DownloadWindowDirective, DownloadWindowLifecycle, DownloadWindowPhase,
};
use crate::icons::MaterialIcon;
use crate::notifications::NotificationCenter;
use crate::theme::Theme;
use crate::ui::native_window_handle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DownloadApplicationEvent {
    DirectoryPicked {
        target: DownloadDirectoryTarget,
        path: PathBuf,
    },
    TaskbarOverlayEditChanged {
        settings: DownloadTaskbarProgressOverlaySettings,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DownloadDirectoryTarget {
    NewTask,
    Settings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownloadSettingsDraft {
    default_directory: PathBuf,
    default_connection_limit: u16,
    adaptive_segment_scaling_enabled: bool,
    adaptive_segment_max_segments: u16,
    adaptive_segment_min_mib: u16,
    max_concurrent_tasks: u16,
    global_speed_limit_bytes_per_second: Option<u64>,
    show_progress_window: bool,
    show_completion_window: bool,
    taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DownloadSettingsViewInitial {
    draft: DownloadSettingsDraft,
    config_path: String,
    log_directory: String,
    database_path: String,
}

pub(crate) struct DownloadRuntimeContext {
    pub fallback_default_directory: PathBuf,
    pub config: AppConfig,
    pub config_path: PathBuf,
    pub log_directory: PathBuf,
    pub database_path: PathBuf,
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
    schedule_window: Option<AnyWindowHandle>,
    settings_window: Option<AnyWindowHandle>,
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
    show_progress_window: bool,
    show_completion_window: bool,
    pending_directory_picker: Option<(uuid::Uuid, DownloadDirectoryTarget)>,
    notified_terminal_states: HashSet<(DownloadTaskId, DownloadStatus)>,
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
        } = runtime;
        config.download.normalize();
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
            schedule_window: None,
            settings_window: None,
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
            show_progress_window,
            show_completion_window,
            pending_directory_picker: None,
            notified_terminal_states,
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

    pub(crate) fn tasks(&self) -> &[DownloadEngineTask] {
        &self.tasks
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
            taskbar_progress_overlay: self.config.download.taskbar_progress_overlay.clone(),
        }
    }

    fn settings_view_initial(&self) -> DownloadSettingsViewInitial {
        DownloadSettingsViewInitial {
            draft: self.settings_draft(),
            config_path: self.config_path.display().to_string(),
            log_directory: self.log_directory.display().to_string(),
            database_path: self.database_path.display().to_string(),
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
                window_min_size: Some(download_task_window_size(DownloadWindowPhase::Completed)),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
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
        let result = self.open_new_download_window_with_request(
            Some(dialog.request),
            Some((correlation_id, handoff_id)),
            cx,
        );
        let _ = dialog.completion.send(result);
    }

    fn open_new_download_window_with_request(
        &mut self,
        initial_request: Option<DownloadRequest>,
        browser_context: Option<(uuid::Uuid, uuid::Uuid)>,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        if let Some(window) = self.new_download_window {
            if initial_request.is_some() {
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
        let auto_probe = initial_request.is_some();
        let window_size = new_download_window_size(false);
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
                app_id: Some("NxTools.NewDownload".into()),
                window_min_size: Some(window_size),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let view = cx.new(|cx| {
                    NewDownloadView::new(
                        application,
                        default_directory,
                        default_connection_limit,
                        default_adaptive_segment_policy,
                        initial_request,
                        window,
                        cx,
                    )
                });
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
                self.new_download_window = Some(window.into());
                if let Some((correlation_id, handoff_id)) = browser_context {
                    info!(
                        event = "download.browser_dialog.opened",
                        %correlation_id,
                        %handoff_id,
                        "browser confirmation window opened"
                    );
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
        let window_size = size(px(520.0), px(486.0));
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
                app_id: Some("NxTools.DownloadSettings".into()),
                window_min_size: Some(window_size),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
            },
            move |window, cx| {
                let close_application = application.clone();
                window.on_window_should_close(cx, move |_, cx| {
                    close_application.update(cx, |application, cx| {
                        application.handle_settings_window_closed(cx);
                    });
                    true
                });
                let view = cx.new(|cx| DownloadSettingsView::new(application, initial, window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => self.settings_window = Some(window.into()),
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
                window.on_window_should_close(cx, move |_, cx| {
                    let editing = close_application
                        .read(cx)
                        .taskbar_overlay_edit_draft
                        .is_some();
                    if editing {
                        close_application.update(cx, |application, cx| {
                            application.finish_taskbar_overlay_edit(
                                false,
                                uuid::Uuid::new_v4(),
                                cx,
                            );
                        });
                        return false;
                    }
                    close_application.update(cx, |application, _| {
                        application.taskbar_overlay_window = None;
                        application.taskbar_overlay_native_window_handle = 0;
                    });
                    true
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
        let failed = |message: String| {
            warn!(
                event = "download.settings.apply_failed",
                %correlation_id,
                elapsed_ms = started_at.elapsed().as_millis(),
                "download settings apply failed"
            );
            Err(message)
        };
        if draft.default_directory.exists() && !draft.default_directory.is_dir() {
            return failed("默认保存位置不是文件夹".into());
        }
        if let Err(error) = std::fs::create_dir_all(&draft.default_directory) {
            return failed(format!("无法创建默认保存目录：{error}"));
        }
        let write_probe = draft
            .default_directory
            .join(format!(".nxtools-write-probe-{correlation_id}"));
        if let Err(error) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&write_probe)
        {
            return failed(format!("默认保存目录不可写：{error}"));
        }
        let _ = std::fs::remove_file(&write_probe);

        let previous_concurrency = self
            .default_queue()
            .and_then(|queue| queue.max_concurrent_tasks)
            .unwrap_or(3);
        let previous_speed_limit = self.engine.global_speed_limit();
        let next_download_settings = DownloadSettings {
            default_directory: draft.default_directory.to_string_lossy().into_owned(),
            default_connection_limit: draft.default_connection_limit,
            adaptive_segment_scaling_enabled: draft.adaptive_segment_scaling_enabled,
            adaptive_segment_max_segments: draft.adaptive_segment_max_segments,
            adaptive_segment_min_mib: draft.adaptive_segment_min_mib,
            show_progress_window: draft.show_progress_window,
            show_completion_window: draft.show_completion_window,
            taskbar_progress_overlay: draft.taskbar_progress_overlay.clone(),
        };
        if let Err(error) =
            self.set_default_queue_concurrency(draft.max_concurrent_tasks, correlation_id)
        {
            return failed(error);
        }
        if let Err(error) =
            self.set_global_speed_limit(draft.global_speed_limit_bytes_per_second, correlation_id)
        {
            return match self.set_default_queue_concurrency(previous_concurrency, correlation_id) {
                Ok(()) => failed(error),
                Err(rollback_error) => {
                    failed(format!("{error}；同时无法回滚队列并发：{rollback_error}"))
                }
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
                    return failed(format!("无法保存下载设置：{error}{rollback}"));
                }
            };
        self.config = next_config;
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
                let view = cx
                    .new(|_| DownloadTaskWindowView::new(application, task_id, snapshot, progress));
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
            view.replace_snapshot(snapshot, progress, cx);
        });
        let updated = window_handle
            .update(cx, move |_, window, _| {
                if let Some(phase) = resize_for {
                    let new_size = download_task_window_size(phase);
                    let bounds = window.bounds();
                    let scale = window.scale_factor();
                    let centered_x = bounds.origin.x.as_f32()
                        + (bounds.size.width.as_f32() - new_size.width.as_f32()) / 2.0;
                    let centered_y = bounds.origin.y.as_f32()
                        + (bounds.size.height.as_f32() - new_size.height.as_f32()) / 2.0;
                    if native_window_handle != 0 {
                        let _ = platform.send_command(PlatformCommand::SetWindowBounds(
                            SetWindowBoundsCommand {
                                window_handle: native_window_handle,
                                x: (centered_x * scale).round() as i32,
                                y: (centered_y * scale).round() as i32,
                                width: (new_size.width.as_f32() * scale).round() as i32,
                                height: (new_size.height.as_f32() * scale).round() as i32,
                            },
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
        for task in tasks {
            let status = task.task.status();
            if !matches!(status, DownloadStatus::Completed | DownloadStatus::Failed)
                || !self
                    .notified_terminal_states
                    .insert((task.task.id(), status))
            {
                continue;
            }
            let id = format!("download-{}", task.task.id().as_uuid());
            match status {
                DownloadStatus::Completed => {
                    let message = task.final_path.as_ref().map_or_else(
                        || "下载文件已完成".into(),
                        |path| path.display().to_string(),
                    );
                    self.notifications.update(cx, |notifications, cx| {
                        notifications.show_success(id, "下载完成", message, cx);
                    });
                }
                DownloadStatus::Failed => {
                    let message = task.error.as_ref().map_or_else(
                        || "下载任务失败，未返回详细原因".into(),
                        |error| error.message.clone(),
                    );
                    self.notifications.update(cx, |notifications, cx| {
                        notifications.show_error(id, "下载失败", message, cx);
                    });
                }
                _ => {}
            }
        }
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

struct DownloadScheduleView {
    theme: Theme,
    application: Entity<DownloadApplication>,
    start_time: Entity<InputState>,
    stop_time: Entity<InputState>,
    days: HashSet<DownloadWeekday>,
    enabled_start: bool,
    enabled_stop: bool,
    queue_running: bool,
    saving: bool,
    error: Option<String>,
}

impl DownloadScheduleView {
    fn new(
        application: Entity<DownloadApplication>,
        queue: Option<DownloadQueue>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let daily = queue
            .as_ref()
            .and_then(|queue| queue.schedule.as_ref())
            .map(|schedule| schedule.daily.clone());
        let start_minute = daily
            .as_ref()
            .map_or(2 * 60 + 30, |daily| daily.start_minute);
        let stop_minute = daily
            .as_ref()
            .map_or(7 * 60 + 30, |daily| daily.stop_minute);
        let start_time = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(format_clock_minute(start_minute))
                .placeholder("HH:MM")
        });
        let stop_time = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(format_clock_minute(stop_minute))
                .placeholder("HH:MM")
        });
        let days = daily.as_ref().map_or_else(
            || DownloadWeekday::ALL.into_iter().collect(),
            |daily| daily.days.iter().copied().collect(),
        );
        Self {
            theme: Theme::zed_dark(),
            application,
            start_time,
            stop_time,
            days,
            enabled_start: daily.as_ref().is_some_and(|daily| daily.enabled_start),
            enabled_stop: daily.as_ref().is_some_and(|daily| daily.enabled_stop),
            queue_running: queue.is_none_or(|queue| queue.is_running),
            saving: false,
            error: None,
        }
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let schedule = match build_download_schedule(
            self.enabled_start,
            self.enabled_stop,
            &self.days,
            self.start_time.read(cx).value().trim(),
            self.stop_time.read(cx).value().trim(),
        ) {
            Ok(schedule) => schedule,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        self.saving = true;
        self.error = None;
        let result = self.application.update(cx, |application, cx| {
            application.set_default_queue_schedule(schedule, cx)
        });
        match result {
            Ok(()) => window.remove_window(),
            Err(error) => {
                self.saving = false;
                self.error = Some(error);
                cx.notify();
            }
        }
    }
}

impl Render for DownloadScheduleView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let mut weekdays = div().flex().items_center().gap(px(4.0));
        for (index, day) in DownloadWeekday::ALL.into_iter().enumerate() {
            let selected = self.days.contains(&day);
            weekdays = weekdays.child(
                div()
                    .id(("download-schedule-day", index))
                    .w(px(38.0))
                    .h(px(26.0))
                    .rounded(px(4.0))
                    .border_1()
                    .border_color(if selected {
                        theme.accent
                    } else {
                        theme.border_subtle
                    })
                    .bg(if selected {
                        theme.accent_soft
                    } else {
                        theme.canvas
                    })
                    .cursor_pointer()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(10.0))
                    .text_color(if selected {
                        theme.accent_text
                    } else {
                        theme.text_muted
                    })
                    .child(day.short_label())
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !this.days.insert(day) {
                            this.days.remove(&day);
                        }
                        this.error = None;
                        cx.notify();
                    })),
            );
        }
        let start_control = schedule_trigger_row(
            theme,
            "自动开始",
            "到点后启动等待队列",
            self.enabled_start,
            &self.start_time,
            "download-schedule-start-toggle",
            cx.listener(|this, _, _, cx| {
                this.enabled_start = !this.enabled_start;
                this.error = None;
                cx.notify();
            }),
        );
        let stop_control = schedule_trigger_row(
            theme,
            "自动停止",
            "到点后暂停活动任务",
            self.enabled_stop,
            &self.stop_time,
            "download-schedule-stop-toggle",
            cx.listener(|this, _, _, cx| {
                this.enabled_stop = !this.enabled_stop;
                this.error = None;
                cx.notify();
            }),
        );
        div()
            .size_full()
            .bg(theme.canvas)
            .text_color(theme.text)
            .flex()
            .flex_col()
            .child(
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
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("每日计划"),
                    )
                    .child(download_window_close_button(theme)),
            )
            .child(
                div()
                    .flex_1()
                    .px(px(14.0))
                    .py(px(10.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .h(px(32.0))
                            .px(px(9.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(theme.border_subtle)
                            .bg(theme.surface)
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(10.0))
                                    .text_color(theme.text_secondary)
                                    .child("默认下载队列"),
                            )
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .text_color(if self.queue_running {
                                        theme.success
                                    } else {
                                        theme.text_muted
                                    })
                                    .child(if self.queue_running {
                                        "运行中"
                                    } else {
                                        "计划暂停"
                                    }),
                            ),
                    )
                    .child(
                        div()
                            .h(px(44.0))
                            .px(px(9.0))
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(theme.border_subtle)
                            .bg(theme.surface)
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .w(px(82.0))
                                    .text_size(px(10.0))
                                    .text_color(theme.text_secondary)
                                    .child("生效星期"),
                            )
                            .child(weekdays),
                    )
                    .child(start_control)
                    .child(stop_control)
                    .when_some(self.error.clone(), |body, error| {
                        body.child(download_inline_error(theme, error))
                    }),
            )
            .child(
                div()
                    .h(px(44.0))
                    .px(px(14.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .flex_1()
                            .text_size(px(9.0))
                            .text_color(theme.text_muted)
                            .child("本地时间 · 支持跨午夜"),
                    )
                    .child(
                        Button::new("download-schedule-cancel")
                            .label("取消")
                            .small()
                            .ghost()
                            .disabled(self.saving)
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("download-schedule-save")
                            .label(if self.saving {
                                "正在应用…"
                            } else {
                                "应用"
                            })
                            .small()
                            .primary()
                            .disabled(self.saving)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save(window, cx);
                            })),
                    ),
            )
    }
}

fn schedule_trigger_row(
    theme: Theme,
    title: &'static str,
    description: &'static str,
    enabled: bool,
    input: &Entity<InputState>,
    id: &'static str,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .h(px(44.0))
        .px(px(9.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .items_center()
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(1.0))
                .child(div().text_size(px(10.0)).child(title))
                .child(
                    div()
                        .text_size(px(9.0))
                        .text_color(theme.text_muted)
                        .child(description),
                ),
        )
        .child(
            div()
                .id(id)
                .debug_selector(move || id.into())
                .h(px(24.0))
                .w(px(50.0))
                .mr(px(7.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(if enabled {
                    theme.accent
                } else {
                    theme.border_subtle
                })
                .bg(if enabled {
                    theme.accent_soft
                } else {
                    theme.canvas
                })
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(9.5))
                .text_color(if enabled {
                    theme.accent_text
                } else {
                    theme.text_muted
                })
                .child(if enabled { "开启" } else { "关闭" })
                .on_click(on_toggle),
        )
        .child(if enabled {
            div()
                .w(px(82.0))
                .h(px(28.0))
                .rounded(px(4.0))
                .border_1()
                .border_color(theme.border_subtle)
                .bg(theme.canvas)
                .px(px(6.0))
                .flex()
                .items_center()
                .child(
                    Input::new(input)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false),
                )
                .into_any_element()
        } else {
            div()
                .w(px(82.0))
                .text_size(px(9.5))
                .text_color(theme.text_muted)
                .text_center()
                .child("未启用")
                .into_any_element()
        })
        .into_any_element()
}

fn parse_clock_minute(value: &str) -> Result<u16, &'static str> {
    let Some((hours, minutes)) = value.split_once(':') else {
        return Err("请输入 HH:MM");
    };
    let hours = hours.parse::<u16>().map_err(|_| "请输入 HH:MM")?;
    let minutes = minutes.parse::<u16>().map_err(|_| "请输入 HH:MM")?;
    if hours >= 24 || minutes >= 60 {
        return Err("时间必须在 00:00 到 23:59 之间");
    }
    Ok(hours * 60 + minutes)
}

fn build_download_schedule(
    enabled_start: bool,
    enabled_stop: bool,
    days: &HashSet<DownloadWeekday>,
    start_time: &str,
    stop_time: &str,
) -> Result<Option<DownloadSchedule>, String> {
    if !enabled_start && !enabled_stop {
        return Ok(None);
    }
    if days.is_empty() {
        return Err("至少选择一个生效星期".into());
    }
    let start_minute = if enabled_start {
        parse_clock_minute(start_time).map_err(|error| format!("开始时间：{error}"))?
    } else {
        0
    };
    let stop_minute = if enabled_stop {
        parse_clock_minute(stop_time).map_err(|error| format!("停止时间：{error}"))?
    } else {
        0
    };
    Ok(Some(DownloadSchedule {
        daily: DownloadDailySchedule {
            days: DownloadWeekday::ALL
                .into_iter()
                .filter(|day| days.contains(day))
                .collect(),
            start_minute,
            stop_minute,
            enabled_start,
            enabled_stop,
        },
    }))
}

fn format_clock_minute(value: u16) -> String {
    format!("{:02}:{:02}", value / 60, value % 60)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DelayedStartSelection {
    date: NaiveDate,
    hour: u32,
    minute: u32,
}

impl DelayedStartSelection {
    fn resolve_unix_ms(self, now_unix_ms: i64) -> Result<i64, String> {
        let time = NaiveTime::from_hms_opt(self.hour, self.minute, 0)
            .ok_or("计划时间必须是 00:00 到 23:59")?;
        let resolved = Local
            .from_local_datetime(&self.date.and_time(time))
            .single()
            .ok_or("计划时间在当前本地时区无效或存在歧义，请换一个时间")?;
        let unix_ms = resolved.timestamp_millis();
        if unix_ms <= now_unix_ms {
            return Err("计划时间必须晚于当前时间".into());
        }
        Ok(unix_ms)
    }
}

fn resolve_scheduled_start(
    selection: Option<DelayedStartSelection>,
    now_unix_ms: i64,
) -> Result<Option<i64>, String> {
    selection
        .map(|selection| selection.resolve_unix_ms(now_unix_ms))
        .transpose()
}

fn read_delayed_start_draft(
    date: NaiveDate,
    hour: &Entity<SelectState<Vec<String>>>,
    minute: &Entity<SelectState<Vec<String>>>,
    cx: &App,
) -> Result<DelayedStartSelection, String> {
    let hour = hour
        .read(cx)
        .selected_value()
        .and_then(|value| value.parse().ok())
        .ok_or("请选择计划下载小时")?;
    let minute = minute
        .read(cx)
        .selected_value()
        .and_then(|value| value.parse().ok())
        .ok_or("请选择计划下载分钟")?;
    Ok(DelayedStartSelection { date, hour, minute })
}

fn calendar_month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1)
        .expect("a chrono date always has a valid first day")
}

fn shift_calendar_month(month: NaiveDate, delta: i32) -> NaiveDate {
    let total_months = month.year() * 12 + month.month0() as i32 + delta;
    let year = total_months.div_euclid(12);
    let month = total_months.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(year, month, 1).expect("normalized year and month are valid")
}

fn delayed_start_calendar_days(month: NaiveDate) -> Vec<NaiveDate> {
    let grid_start = month - ChronoDuration::days(month.weekday().num_days_from_sunday() as i64);
    (0..42)
        .map(|offset| grid_start + ChronoDuration::days(offset))
        .collect()
}

fn delayed_start_calendar_title(month: NaiveDate) -> String {
    format!("{}年{}月", month.year(), month.month())
}

struct DownloadSettingsInput<'a> {
    default_directory: &'a str,
    connection_limit: &'a str,
    adaptive_segment_scaling_enabled: bool,
    adaptive_segment_max_segments: &'a str,
    adaptive_segment_min_mib: &'a str,
    max_concurrent_tasks: &'a str,
    speed_limit_kib: &'a str,
    show_progress_window: bool,
    show_completion_window: bool,
    taskbar_overlay_enabled: bool,
    taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor,
    taskbar_overlay_offset_x: &'a str,
    taskbar_overlay_offset_y: &'a str,
    taskbar_overlay_width: &'a str,
    taskbar_overlay_height: &'a str,
    taskbar_overlay_progress_mode: DownloadTaskbarProgressMode,
    taskbar_overlay_show_progress: bool,
    taskbar_overlay_show_speed: bool,
    taskbar_overlay_show_remaining_time: bool,
    taskbar_overlay_show_active_tasks: bool,
    taskbar_overlay_show_active_connections: bool,
    taskbar_overlay_show_transferred_bytes: bool,
}

fn parse_download_settings_draft(
    input: DownloadSettingsInput<'_>,
) -> Result<DownloadSettingsDraft, String> {
    let DownloadSettingsInput {
        default_directory,
        connection_limit,
        adaptive_segment_scaling_enabled,
        adaptive_segment_max_segments,
        adaptive_segment_min_mib,
        max_concurrent_tasks,
        speed_limit_kib,
        show_progress_window,
        show_completion_window,
        taskbar_overlay_enabled,
        taskbar_overlay_anchor,
        taskbar_overlay_offset_x,
        taskbar_overlay_offset_y,
        taskbar_overlay_width,
        taskbar_overlay_height,
        taskbar_overlay_progress_mode,
        taskbar_overlay_show_progress,
        taskbar_overlay_show_speed,
        taskbar_overlay_show_remaining_time,
        taskbar_overlay_show_active_tasks,
        taskbar_overlay_show_active_connections,
        taskbar_overlay_show_transferred_bytes,
    } = input;
    let default_directory = default_directory.trim();
    if default_directory.is_empty() {
        return Err("请选择默认保存目录".into());
    }
    let default_connection_limit = connection_limit
        .trim()
        .parse::<u16>()
        .map_err(|_| "连接上限必须是 1 到 64 的整数")?;
    if !(1..=64).contains(&default_connection_limit) {
        return Err("连接上限必须在 1 到 64 之间".into());
    }
    let adaptive_segment_max_segments = adaptive_segment_max_segments
        .trim()
        .parse::<u16>()
        .map_err(|_| "动态分段上限必须是整数")?;
    if !(default_connection_limit..=256).contains(&adaptive_segment_max_segments) {
        return Err("动态分段上限必须在连接数和 256 之间".into());
    }
    let adaptive_segment_min_mib = adaptive_segment_min_mib
        .trim()
        .parse::<u16>()
        .map_err(|_| "最小可拆分大小必须是 1 到 100 MiB 的整数")?;
    if !(1..=100).contains(&adaptive_segment_min_mib) {
        return Err("最小可拆分大小必须在 1 到 100 MiB 之间".into());
    }
    let max_concurrent_tasks = max_concurrent_tasks
        .trim()
        .parse::<u16>()
        .map_err(|_| "同时下载数必须是 1 到 64 的整数")?;
    if !(1..=64).contains(&max_concurrent_tasks) {
        return Err("同时下载数必须在 1 到 64 之间".into());
    }
    let speed_limit_kib = speed_limit_kib
        .trim()
        .parse::<u64>()
        .map_err(|_| "全局限速必须是非负整数")?;
    let global_speed_limit_bytes_per_second = if speed_limit_kib == 0 {
        None
    } else {
        Some(
            speed_limit_kib
                .checked_mul(1024)
                .ok_or("全局限速数值过大")?,
        )
    };
    let taskbar_overlay_offset_x = taskbar_overlay_offset_x
        .trim()
        .parse::<i32>()
        .map_err(|_| "任务栏 X 偏移必须是整数")?;
    let taskbar_overlay_offset_y = taskbar_overlay_offset_y
        .trim()
        .parse::<i32>()
        .map_err(|_| "任务栏 Y 偏移必须是整数")?;
    if !(-4096..=4096).contains(&taskbar_overlay_offset_x)
        || !(-4096..=4096).contains(&taskbar_overlay_offset_y)
    {
        return Err("任务栏偏移必须在 -4096 到 4096 之间".into());
    }
    let taskbar_overlay_width = taskbar_overlay_width
        .trim()
        .parse::<u32>()
        .map_err(|_| "任务栏覆盖层宽度必须是整数")?;
    let mut taskbar_overlay_height = taskbar_overlay_height
        .trim()
        .parse::<u32>()
        .map_err(|_| "任务栏覆盖层高度必须是整数")?;
    if !(72..=640).contains(&taskbar_overlay_width) {
        return Err("任务栏覆盖层宽度必须在 72 到 640 之间".into());
    }
    if !(6..=72).contains(&taskbar_overlay_height) {
        return Err("任务栏覆盖层高度必须在 6 到 72 之间".into());
    }
    if !taskbar_overlay_show_progress
        && !taskbar_overlay_show_speed
        && !taskbar_overlay_show_remaining_time
        && !taskbar_overlay_show_active_tasks
        && !taskbar_overlay_show_active_connections
        && !taskbar_overlay_show_transferred_bytes
    {
        return Err("任务栏覆盖层至少要显示一项信息".into());
    }
    if taskbar_overlay_show_speed
        || taskbar_overlay_show_remaining_time
        || taskbar_overlay_show_active_tasks
        || taskbar_overlay_show_active_connections
        || taskbar_overlay_show_transferred_bytes
    {
        taskbar_overlay_height = taskbar_overlay_height.max(20);
    }
    Ok(DownloadSettingsDraft {
        default_directory: PathBuf::from(default_directory),
        default_connection_limit,
        adaptive_segment_scaling_enabled,
        adaptive_segment_max_segments,
        adaptive_segment_min_mib,
        max_concurrent_tasks,
        global_speed_limit_bytes_per_second,
        show_progress_window,
        show_completion_window,
        taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings {
            enabled: taskbar_overlay_enabled,
            anchor: taskbar_overlay_anchor,
            offset_x: taskbar_overlay_offset_x,
            offset_y: taskbar_overlay_offset_y,
            width: taskbar_overlay_width,
            height: taskbar_overlay_height,
            progress_mode: taskbar_overlay_progress_mode,
            show_progress: taskbar_overlay_show_progress,
            show_speed: taskbar_overlay_show_speed,
            show_remaining_time: taskbar_overlay_show_remaining_time,
            show_active_tasks: taskbar_overlay_show_active_tasks,
            show_active_connections: taskbar_overlay_show_active_connections,
            show_transferred_bytes: taskbar_overlay_show_transferred_bytes,
        },
    })
}

struct DownloadSettingsView {
    theme: Theme,
    application: Entity<DownloadApplication>,
    directory: Entity<InputState>,
    connection_limit: Entity<InputState>,
    adaptive_segment_max_segments: Entity<InputState>,
    adaptive_segment_min_mib: Entity<InputState>,
    concurrent_tasks: Entity<InputState>,
    speed_limit_kib: Entity<InputState>,
    taskbar_overlay_offset_x: Entity<InputState>,
    taskbar_overlay_offset_y: Entity<InputState>,
    taskbar_overlay_width: Entity<InputState>,
    taskbar_overlay_height: Entity<InputState>,
    scroll: ScrollHandle,
    show_progress_window: bool,
    show_completion_window: bool,
    adaptive_segment_scaling_enabled: bool,
    taskbar_overlay_enabled: bool,
    taskbar_overlay_anchor: DownloadTaskbarOverlayAnchor,
    taskbar_overlay_progress_mode: DownloadTaskbarProgressMode,
    taskbar_overlay_show_progress: bool,
    taskbar_overlay_show_speed: bool,
    taskbar_overlay_show_remaining_time: bool,
    taskbar_overlay_show_active_tasks: bool,
    taskbar_overlay_show_active_connections: bool,
    taskbar_overlay_show_transferred_bytes: bool,
    taskbar_overlay_editing: bool,
    config_path: String,
    log_directory: String,
    database_path: String,
    saving: bool,
    error: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl DownloadSettingsView {
    fn new(
        application: Entity<DownloadApplication>,
        initial: DownloadSettingsViewInitial,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let DownloadSettingsViewInitial {
            draft: snapshot,
            config_path,
            log_directory,
            database_path,
        } = initial;
        let directory = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.default_directory.to_string_lossy().into_owned())
                .placeholder("选择默认保存目录")
        });
        let connection_limit = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.default_connection_limit.to_string())
                .placeholder("1-64")
        });
        let adaptive_segment_max_segments = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.adaptive_segment_max_segments.to_string())
                .placeholder("8-256")
        });
        let adaptive_segment_min_mib = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.adaptive_segment_min_mib.to_string())
                .placeholder("1-100")
        });
        let concurrent_tasks = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.max_concurrent_tasks.to_string())
                .placeholder("1-64")
        });
        let speed_limit_kib = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(
                    snapshot
                        .global_speed_limit_bytes_per_second
                        .map_or(0, |value| value / 1024)
                        .to_string(),
                )
                .placeholder("0")
        });
        let taskbar_overlay_offset_x = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.taskbar_progress_overlay.offset_x.to_string())
                .placeholder("12")
        });
        let taskbar_overlay_offset_y = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.taskbar_progress_overlay.offset_y.to_string())
                .placeholder("0")
        });
        let taskbar_overlay_width = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.taskbar_progress_overlay.width.to_string())
                .placeholder("132")
        });
        let taskbar_overlay_height = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(snapshot.taskbar_progress_overlay.height.to_string())
                .placeholder("8")
        });
        let subscription = cx.subscribe_in(
            &application,
            window,
            |this, _application, event: &DownloadApplicationEvent, window, cx| {
                match event {
                    DownloadApplicationEvent::DirectoryPicked { target, path } => {
                        if *target != DownloadDirectoryTarget::Settings {
                            return;
                        }
                        this.directory.update(cx, |input, cx| {
                            input.set_value(path.to_string_lossy().into_owned(), window, cx);
                        });
                        this.error = None;
                    }
                    DownloadApplicationEvent::TaskbarOverlayEditChanged { settings } => {
                        this.taskbar_overlay_offset_x.update(cx, |input, cx| {
                            input.set_value(settings.offset_x.to_string(), window, cx);
                        });
                        this.taskbar_overlay_offset_y.update(cx, |input, cx| {
                            input.set_value(settings.offset_y.to_string(), window, cx);
                        });
                        this.taskbar_overlay_width.update(cx, |input, cx| {
                            input.set_value(settings.width.to_string(), window, cx);
                        });
                        this.taskbar_overlay_height.update(cx, |input, cx| {
                            input.set_value(settings.height.to_string(), window, cx);
                        });
                        this.taskbar_overlay_editing = true;
                    }
                }
                cx.notify();
            },
        );
        let offset_x_subscription = cx.subscribe_in(
            &taskbar_overlay_offset_x,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.sync_taskbar_overlay_edit(cx);
                }
            },
        );
        let offset_y_subscription = cx.subscribe_in(
            &taskbar_overlay_offset_y,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.sync_taskbar_overlay_edit(cx);
                }
            },
        );
        let width_subscription = cx.subscribe_in(
            &taskbar_overlay_width,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.sync_taskbar_overlay_edit(cx);
                }
            },
        );
        let height_subscription = cx.subscribe_in(
            &taskbar_overlay_height,
            window,
            |this, _, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
                    this.sync_taskbar_overlay_edit(cx);
                }
            },
        );
        Self {
            theme: Theme::zed_dark(),
            application,
            directory,
            connection_limit,
            adaptive_segment_max_segments,
            adaptive_segment_min_mib,
            concurrent_tasks,
            speed_limit_kib,
            taskbar_overlay_offset_x,
            taskbar_overlay_offset_y,
            taskbar_overlay_width,
            taskbar_overlay_height,
            scroll: ScrollHandle::new(),
            show_progress_window: snapshot.show_progress_window,
            show_completion_window: snapshot.show_completion_window,
            adaptive_segment_scaling_enabled: snapshot.adaptive_segment_scaling_enabled,
            taskbar_overlay_enabled: snapshot.taskbar_progress_overlay.enabled,
            taskbar_overlay_anchor: snapshot.taskbar_progress_overlay.anchor,
            taskbar_overlay_progress_mode: snapshot.taskbar_progress_overlay.progress_mode,
            taskbar_overlay_show_progress: snapshot.taskbar_progress_overlay.show_progress,
            taskbar_overlay_show_speed: snapshot.taskbar_progress_overlay.show_speed,
            taskbar_overlay_show_remaining_time: snapshot
                .taskbar_progress_overlay
                .show_remaining_time,
            taskbar_overlay_show_active_tasks: snapshot.taskbar_progress_overlay.show_active_tasks,
            taskbar_overlay_show_active_connections: snapshot
                .taskbar_progress_overlay
                .show_active_connections,
            taskbar_overlay_show_transferred_bytes: snapshot
                .taskbar_progress_overlay
                .show_transferred_bytes,
            taskbar_overlay_editing: false,
            config_path,
            log_directory,
            database_path,
            saving: false,
            error: None,
            _subscriptions: vec![
                subscription,
                offset_x_subscription,
                offset_y_subscription,
                width_subscription,
                height_subscription,
            ],
        }
    }

    fn current_draft(&self, cx: &App) -> Result<DownloadSettingsDraft, String> {
        parse_download_settings_draft(DownloadSettingsInput {
            default_directory: &self.directory.read(cx).value(),
            connection_limit: &self.connection_limit.read(cx).value(),
            adaptive_segment_scaling_enabled: self.adaptive_segment_scaling_enabled,
            adaptive_segment_max_segments: &self.adaptive_segment_max_segments.read(cx).value(),
            adaptive_segment_min_mib: &self.adaptive_segment_min_mib.read(cx).value(),
            max_concurrent_tasks: &self.concurrent_tasks.read(cx).value(),
            speed_limit_kib: &self.speed_limit_kib.read(cx).value(),
            show_progress_window: self.show_progress_window,
            show_completion_window: self.show_completion_window,
            taskbar_overlay_enabled: self.taskbar_overlay_enabled,
            taskbar_overlay_anchor: self.taskbar_overlay_anchor,
            taskbar_overlay_offset_x: &self.taskbar_overlay_offset_x.read(cx).value(),
            taskbar_overlay_offset_y: &self.taskbar_overlay_offset_y.read(cx).value(),
            taskbar_overlay_width: &self.taskbar_overlay_width.read(cx).value(),
            taskbar_overlay_height: &self.taskbar_overlay_height.read(cx).value(),
            taskbar_overlay_progress_mode: self.taskbar_overlay_progress_mode,
            taskbar_overlay_show_progress: self.taskbar_overlay_show_progress,
            taskbar_overlay_show_speed: self.taskbar_overlay_show_speed,
            taskbar_overlay_show_remaining_time: self.taskbar_overlay_show_remaining_time,
            taskbar_overlay_show_active_tasks: self.taskbar_overlay_show_active_tasks,
            taskbar_overlay_show_active_connections: self.taskbar_overlay_show_active_connections,
            taskbar_overlay_show_transferred_bytes: self.taskbar_overlay_show_transferred_bytes,
        })
    }

    fn toggle_taskbar_overlay_edit(&mut self, cx: &mut Context<Self>) {
        let correlation_id = uuid::Uuid::new_v4();
        if self.taskbar_overlay_editing {
            let application = self.application.clone();
            cx.defer(move |cx| {
                application.update(cx, |application, cx| {
                    application.finish_taskbar_overlay_edit(false, correlation_id, cx);
                });
            });
            self.taskbar_overlay_editing = false;
            cx.notify();
            return;
        }
        let settings = match self.current_draft(cx) {
            Ok(draft) => draft.taskbar_progress_overlay,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        let application = self.application.clone();
        let view = cx.entity().downgrade();
        cx.defer(move |cx| {
            let result = application.update(cx, |application, cx| {
                application.begin_taskbar_overlay_edit(settings, correlation_id, cx)
            });
            let _ = view.update(cx, |view, cx| match result {
                Ok(()) => {
                    view.taskbar_overlay_editing = true;
                    view.error = None;
                    cx.notify();
                }
                Err(error) => {
                    view.error = Some(error);
                    cx.notify();
                }
            });
        });
    }

    fn sync_taskbar_overlay_edit(&mut self, cx: &mut Context<Self>) {
        if !self.taskbar_overlay_editing {
            return;
        }
        let settings = match self.current_draft(cx) {
            Ok(draft) => draft.taskbar_progress_overlay,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        let application = self.application.clone();
        let view = cx.entity().downgrade();
        let correlation_id = uuid::Uuid::new_v4();
        cx.defer(move |cx| {
            let result = application.update(cx, |application, cx| {
                application.update_taskbar_overlay_edit(settings, correlation_id, cx)
            });
            let _ = view.update(cx, |view, cx| match result {
                Ok(()) => {
                    view.error = None;
                    cx.notify();
                }
                Err(error) => {
                    view.error = Some(error);
                    cx.notify();
                }
            });
        });
    }

    fn save(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.saving {
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let draft = match self.current_draft(cx) {
            Ok(draft) => draft,
            Err(error) => {
                warn!(
                    event = "download.settings.validation_failed",
                    %correlation_id,
                    "download settings validation failed"
                );
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        self.saving = true;
        self.error = None;
        let application = self.application.clone();
        let source_window = window.window_handle();
        let view = cx.entity().downgrade();
        cx.defer(move |cx| {
            let result = application.update(cx, |application, cx| {
                application.apply_settings(draft, correlation_id, cx)
            });
            match result {
                Ok(()) => {
                    let _ = source_window.update(cx, |_, window, _| window.remove_window());
                }
                Err(error) => {
                    let _ = view.update(cx, |view, cx| {
                        view.saving = false;
                        view.error = Some(error);
                        cx.notify();
                    });
                }
            }
        });
        cx.notify();
    }
}

impl Render for DownloadSettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let scroll = self.scroll.clone();
        let current_directory = self.directory.read(cx).value().to_string();
        let application = self.application.clone();
        let directory_control = div()
            .w(px(274.0))
            .flex()
            .items_center()
            .gap(px(5.0))
            .child(download_settings_input(theme, &self.directory, px(236.0)))
            .child(
                Button::new("download-settings-pick-directory")
                    .icon(Icon::new(MaterialIcon::Folder))
                    .xsmall()
                    .compact()
                    .ghost()
                    .on_click(move |_, _, cx| {
                        let application = application.clone();
                        let current_directory = current_directory.clone();
                        cx.defer(move |cx| {
                            application.update(cx, |application, cx| {
                                application.request_settings_directory(current_directory, cx);
                            });
                        });
                    }),
            )
            .into_any_element();
        let connection_control =
            download_settings_input(theme, &self.connection_limit, px(88.0)).into_any_element();
        let adaptive_max_control =
            download_settings_input(theme, &self.adaptive_segment_max_segments, px(88.0))
                .into_any_element();
        let adaptive_min_control =
            download_settings_input(theme, &self.adaptive_segment_min_mib, px(88.0))
                .into_any_element();
        let adaptive_toggle = download_settings_toggle(
            theme,
            "download-settings-adaptive-segments",
            self.adaptive_segment_scaling_enabled,
            cx.listener(|this, _, _, cx| {
                this.adaptive_segment_scaling_enabled = !this.adaptive_segment_scaling_enabled;
                this.error = None;
                cx.notify();
            }),
        );
        let concurrent_control =
            download_settings_input(theme, &self.concurrent_tasks, px(88.0)).into_any_element();
        let speed_control =
            download_settings_input(theme, &self.speed_limit_kib, px(88.0)).into_any_element();
        let progress_toggle = download_settings_toggle(
            theme,
            "download-settings-progress-window",
            self.show_progress_window,
            cx.listener(|this, _, _, cx| {
                this.show_progress_window = !this.show_progress_window;
                this.error = None;
                cx.notify();
            }),
        );
        let completion_toggle = download_settings_toggle(
            theme,
            "download-settings-completion-window",
            self.show_completion_window,
            cx.listener(|this, _, _, cx| {
                this.show_completion_window = !this.show_completion_window;
                this.error = None;
                cx.notify();
            }),
        );
        let taskbar_overlay_toggle = download_settings_toggle(
            theme,
            "download-settings-taskbar-overlay",
            self.taskbar_overlay_enabled,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_enabled = !this.taskbar_overlay_enabled;
                this.error = None;
                cx.notify();
            }),
        );
        let taskbar_anchor = download_settings_anchor(
            theme,
            self.taskbar_overlay_anchor,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_anchor = DownloadTaskbarOverlayAnchor::Left;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_anchor = DownloadTaskbarOverlayAnchor::Right;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_offset_x =
            download_settings_input(theme, &self.taskbar_overlay_offset_x, px(88.0))
                .into_any_element();
        let taskbar_offset_y =
            download_settings_input(theme, &self.taskbar_overlay_offset_y, px(88.0))
                .into_any_element();
        let taskbar_width = download_settings_input(theme, &self.taskbar_overlay_width, px(88.0))
            .into_any_element();
        let taskbar_height = download_settings_input(theme, &self.taskbar_overlay_height, px(88.0))
            .into_any_element();
        let taskbar_progress_mode = download_settings_progress_mode(
            theme,
            self.taskbar_overlay_progress_mode,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_progress_mode = DownloadTaskbarProgressMode::Aggregate;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_progress_mode = DownloadTaskbarProgressMode::PerTask;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_progress_mode = DownloadTaskbarProgressMode::PerSegment;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_progress_info = download_settings_toggle(
            theme,
            "download-settings-taskbar-show-progress",
            self.taskbar_overlay_show_progress,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_progress = !this.taskbar_overlay_show_progress;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_speed_info = download_settings_toggle(
            theme,
            "download-settings-taskbar-show-speed",
            self.taskbar_overlay_show_speed,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_speed = !this.taskbar_overlay_show_speed;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_remaining_info = download_settings_toggle(
            theme,
            "download-settings-taskbar-show-remaining",
            self.taskbar_overlay_show_remaining_time,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_remaining_time =
                    !this.taskbar_overlay_show_remaining_time;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_task_count_info = download_settings_toggle(
            theme,
            "download-settings-taskbar-show-tasks",
            self.taskbar_overlay_show_active_tasks,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_active_tasks = !this.taskbar_overlay_show_active_tasks;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_connection_info = download_settings_toggle(
            theme,
            "download-settings-taskbar-show-connections",
            self.taskbar_overlay_show_active_connections,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_active_connections =
                    !this.taskbar_overlay_show_active_connections;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_bytes_info = download_settings_toggle(
            theme,
            "download-settings-taskbar-show-bytes",
            self.taskbar_overlay_show_transferred_bytes,
            cx.listener(|this, _, _, cx| {
                this.taskbar_overlay_show_transferred_bytes =
                    !this.taskbar_overlay_show_transferred_bytes;
                this.error = None;
                this.sync_taskbar_overlay_edit(cx);
                cx.notify();
            }),
        );
        let taskbar_edit_control = Button::new("download-settings-taskbar-edit")
            .icon(Icon::new(if self.taskbar_overlay_editing {
                MaterialIcon::Check
            } else {
                MaterialIcon::Edit
            }))
            .label(if self.taskbar_overlay_editing {
                "结束预览"
            } else {
                "编辑布局"
            })
            .small()
            .custom(download_primary_button_variant(theme, cx))
            .on_click(cx.listener(|this, _, _, cx| {
                this.toggle_taskbar_overlay_edit(cx);
            }))
            .into_any_element();
        div()
            .size_full()
            .bg(theme.canvas)
            .text_color(theme.text)
            .flex()
            .flex_col()
            .child(download_settings_titlebar(
                theme,
                self.application.clone(),
                self.taskbar_overlay_editing,
            ))
            .child(
                div()
                    .id("download-settings-viewport")
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .id("download-settings-scroll")
                            .debug_selector(|| "download-settings-scroll".into())
                            .size_full()
                            .overflow_y_scroll()
                            .track_scroll(&scroll)
                            .px(px(14.0))
                            .py(px(10.0))
                            .flex()
                            .flex_col()
                            .gap(px(9.0))
                            .child(download_settings_section(
                                theme,
                                "保存",
                                vec![download_settings_row(
                                    theme,
                                    "默认保存目录",
                                    "新建任务自动使用，可在单个任务中覆盖",
                                    directory_control,
                                )],
                            ))
                            .child(download_settings_section(
                                theme,
                                "传输",
                                vec![
                                    download_settings_row(
                                        theme,
                                        "连接上限",
                                        "新任务从开始即使用的最大并行连接数，范围 1–64",
                                        connection_control,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "动态分段",
                                        "速度稳定时拆分长尾；连接数仍保持在上限内",
                                        adaptive_toggle,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "分段上限",
                                        "动态拆分后的最大分段数，范围为连接数到 256",
                                        adaptive_max_control,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "最小长尾",
                                        "剩余大小达到此值才拆分，单位 MiB，范围 1–100",
                                        adaptive_min_control,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "同时下载",
                                        "默认队列允许同时运行的任务数，范围 1–64",
                                        concurrent_control,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "全局限速",
                                        "单位 KiB/s；填 0 表示不限速",
                                        speed_control,
                                    ),
                                ],
                            ))
                            .child(download_settings_section(
                                theme,
                                "窗口",
                                vec![
                                    download_settings_row(
                                        theme,
                                        "自动显示进度",
                                        "任务开始后显示独立进度窗口",
                                        progress_toggle,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "显示完成窗口",
                                        "进度窗已关闭时被动显示完成窗口",
                                        completion_toggle,
                                    ),
                                ],
                            ))
                            .child(download_settings_section(
                                theme,
                                "任务栏进度",
                                vec![
                                    download_settings_row(
                                        theme,
                                        "任务栏内进度",
                                        "活动下载存在时在任务栏上沿显示总体进度",
                                        taskbar_overlay_toggle,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "布局预览",
                                        "无任务也显示模拟数据；可直接拖动和缩放",
                                        taskbar_edit_control,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "锚点",
                                        "从任务栏左侧或右侧计算 X 偏移",
                                        taskbar_anchor,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "X 偏移",
                                        "逻辑像素；正值向任务栏内部移动",
                                        taskbar_offset_x,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "Y 偏移",
                                        "逻辑像素；0 表示紧贴任务栏上沿",
                                        taskbar_offset_y,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "宽度",
                                        "逻辑像素，范围 72–640",
                                        taskbar_width,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "高度",
                                        "逻辑像素，范围 6–72",
                                        taskbar_height,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "进度模式",
                                        "总进度、按任务或按当前任务分段显示",
                                        taskbar_progress_mode,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "进度条",
                                        "显示所选模式的进度槽",
                                        taskbar_progress_info,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "速度",
                                        "显示当前合计下载速度",
                                        taskbar_speed_info,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "剩余时间",
                                        "按当前合计速度估算",
                                        taskbar_remaining_info,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "活动任务数",
                                        "显示当前正在处理的任务数量",
                                        taskbar_task_count_info,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "活动连接数",
                                        "显示所有活动任务的连接合计",
                                        taskbar_connection_info,
                                    ),
                                    download_settings_row(
                                        theme,
                                        "已下载 / 总大小",
                                        "显示活动任务的合计字节数",
                                        taskbar_bytes_info,
                                    ),
                                ],
                            ))
                            .child(download_settings_section(
                                theme,
                                "诊断",
                                vec![
                                    download_settings_path_row(
                                        theme,
                                        "配置",
                                        self.config_path.clone(),
                                    ),
                                    download_settings_path_row(
                                        theme,
                                        "日志",
                                        self.log_directory.clone(),
                                    ),
                                    download_settings_path_row(
                                        theme,
                                        "数据",
                                        self.database_path.clone(),
                                    ),
                                ],
                            ))
                            .when_some(self.error.clone(), |body, error| {
                                body.child(download_inline_error(theme, error))
                            }),
                    )
                    .vertical_scrollbar(&self.scroll),
            )
            .child(
                div()
                    .h(px(44.0))
                    .px(px(14.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(6.0))
                    .child(
                        div()
                            .debug_selector(|| "download-settings-cancel".into())
                            .child(
                                Button::new("download-settings-cancel-button")
                                    .label("取消")
                                    .small()
                                    .ghost()
                                    .disabled(self.saving)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if this.taskbar_overlay_editing {
                                            let application = this.application.clone();
                                            let correlation_id = uuid::Uuid::new_v4();
                                            cx.defer(move |cx| {
                                                application.update(cx, |application, cx| {
                                                    application.finish_taskbar_overlay_edit(
                                                        false,
                                                        correlation_id,
                                                        cx,
                                                    );
                                                });
                                            });
                                            this.taskbar_overlay_editing = false;
                                        }
                                        window.remove_window();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .debug_selector(|| "download-settings-apply".into())
                            .child(
                                Button::new("download-settings-apply-button")
                                    .icon(Icon::new(MaterialIcon::Check))
                                    .label(if self.saving {
                                        "正在保存…"
                                    } else {
                                        "应用"
                                    })
                                    .small()
                                    .custom(download_primary_button_variant(theme, cx))
                                    .disabled(self.saving)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.save(window, cx);
                                    })),
                            ),
                    ),
            )
    }
}

fn download_settings_input(
    theme: Theme,
    input: &Entity<InputState>,
    width: gpui::Pixels,
) -> impl IntoElement {
    div()
        .w(width)
        .h(px(28.0))
        .rounded(px(4.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.canvas)
        .px(px(6.0))
        .flex()
        .items_center()
        .child(
            Input::new(input)
                .appearance(false)
                .bordered(false)
                .focus_bordered(false),
        )
}

fn download_settings_toggle(
    theme: Theme,
    id: &'static str,
    enabled: bool,
    on_toggle: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .id(id)
        .debug_selector(move || id.into())
        .w(px(46.0))
        .h(px(22.0))
        .p(px(2.0))
        .rounded(px(11.0))
        .border_1()
        .border_color(if enabled {
            theme.border_active
        } else {
            theme.border_subtle
        })
        .bg(if enabled {
            theme.accent_soft
        } else {
            theme.canvas
        })
        .cursor_pointer()
        .flex()
        .when(enabled, |toggle| toggle.justify_end())
        .on_click(on_toggle)
        .child(div().size(px(16.0)).rounded(px(8.0)).bg(if enabled {
            theme.accent
        } else {
            theme.text_muted
        }))
        .into_any_element()
}

type DownloadSettingsClickHandler = Box<dyn Fn(&gpui::ClickEvent, &mut Window, &mut App)>;

fn download_settings_anchor(
    theme: Theme,
    selected: DownloadTaskbarOverlayAnchor,
    on_left: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_right: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let option = |id: &'static str,
                  label: &'static str,
                  active: bool,
                  on_click: DownloadSettingsClickHandler| {
        div()
            .id(id)
            .debug_selector(move || id.into())
            .w(px(42.0))
            .h(px(24.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(if active {
                theme.border_active
            } else {
                theme.border_subtle
            })
            .bg(if active {
                theme.element_selected
            } else {
                theme.canvas
            })
            .text_size(px(10.0))
            .text_color(if active {
                theme.text
            } else {
                theme.text_secondary
            })
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .hover(move |style| style.bg(theme.surface_hover))
            .on_click(on_click)
            .child(label)
    };
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(option(
            "download-settings-taskbar-left",
            "靠左",
            selected == DownloadTaskbarOverlayAnchor::Left,
            Box::new(on_left),
        ))
        .child(option(
            "download-settings-taskbar-right",
            "靠右",
            selected == DownloadTaskbarOverlayAnchor::Right,
            Box::new(on_right),
        ))
        .into_any_element()
}

fn download_settings_progress_mode(
    theme: Theme,
    selected: DownloadTaskbarProgressMode,
    on_aggregate: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_task: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
    on_segment: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let option = |id: &'static str,
                  label: &'static str,
                  active: bool,
                  on_click: DownloadSettingsClickHandler| {
        div()
            .id(id)
            .h(px(24.0))
            .px(px(7.0))
            .rounded(px(4.0))
            .border_1()
            .border_color(if active {
                theme.border_active
            } else {
                theme.border_subtle
            })
            .bg(if active {
                theme.element_selected
            } else {
                theme.canvas
            })
            .text_size(px(9.5))
            .text_color(if active {
                theme.text
            } else {
                theme.text_secondary
            })
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .hover(move |style| style.bg(theme.surface_hover))
            .on_click(on_click)
            .child(label)
    };
    div()
        .flex()
        .items_center()
        .gap(px(4.0))
        .child(option(
            "download-settings-taskbar-mode-aggregate",
            "总进度",
            selected == DownloadTaskbarProgressMode::Aggregate,
            Box::new(on_aggregate),
        ))
        .child(option(
            "download-settings-taskbar-mode-task",
            "按任务",
            selected == DownloadTaskbarProgressMode::PerTask,
            Box::new(on_task),
        ))
        .child(option(
            "download-settings-taskbar-mode-segment",
            "按分段",
            selected == DownloadTaskbarProgressMode::PerSegment,
            Box::new(on_segment),
        ))
        .into_any_element()
}

fn download_settings_section(
    theme: Theme,
    title: &'static str,
    rows: Vec<AnyElement>,
) -> AnyElement {
    div()
        .w_full()
        .rounded(px(5.0))
        .border_1()
        .border_color(theme.border_subtle)
        .bg(theme.surface)
        .flex()
        .flex_col()
        .child(
            div()
                .h(px(26.0))
                .px(px(9.0))
                .border_b_1()
                .border_color(theme.border_subtle)
                .flex()
                .items_center()
                .text_size(px(9.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.text_secondary)
                .child(title),
        )
        .children(rows)
        .into_any_element()
}

fn download_settings_row(
    theme: Theme,
    title: &'static str,
    description: &'static str,
    control: AnyElement,
) -> AnyElement {
    div()
        .min_h(px(42.0))
        .px(px(9.0))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(
            div()
                .min_w_0()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(div().text_size(px(10.5)).child(title))
                .child(
                    div()
                        .text_size(px(8.8))
                        .text_color(theme.text_muted)
                        .child(description),
                ),
        )
        .child(control)
        .into_any_element()
}

fn download_settings_path_row(theme: Theme, label: &'static str, path: String) -> AnyElement {
    div()
        .h(px(30.0))
        .px(px(9.0))
        .border_b_1()
        .border_color(theme.border_subtle)
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .w(px(36.0))
                .text_size(px(9.0))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .min_w_0()
                .flex_1()
                .truncate()
                .text_size(px(9.0))
                .text_color(theme.text_secondary)
                .child(path),
        )
        .into_any_element()
}

struct NewDownloadView {
    theme: Theme,
    application: Entity<DownloadApplication>,
    url: Entity<InputState>,
    directory: Entity<InputState>,
    file_name: Entity<InputState>,
    scheduled_date: NaiveDate,
    scheduled_month: NaiveDate,
    scheduled_hour: Entity<SelectState<Vec<String>>>,
    scheduled_minute: Entity<SelectState<Vec<String>>>,
    schedule_popover_open: bool,
    committed_schedule: Option<DelayedStartSelection>,
    default_connection_limit: u16,
    default_adaptive_segment_policy: nxtools_download::DownloadAdaptiveSegmentPolicy,
    external_request: Option<DownloadRequest>,
    confirmed_request: Option<DownloadRequest>,
    probe: Option<DownloadProbe>,
    probing: bool,
    submitting: bool,
    probe_generation: u64,
    error: Option<String>,
    _subscription: Subscription,
}

impl NewDownloadView {
    fn new(
        application: Entity<DownloadApplication>,
        default_directory: PathBuf,
        default_connection_limit: u16,
        default_adaptive_segment_policy: nxtools_download::DownloadAdaptiveSegmentPolicy,
        initial_request: Option<DownloadRequest>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let initial_url = initial_request
            .as_ref()
            .map_or_else(String::new, |request| request.source_url.clone());
        let initial_directory = initial_request.as_ref().map_or_else(
            || default_directory.to_string_lossy().into_owned(),
            |request| request.save_directory.to_string_lossy().into_owned(),
        );
        let initial_file_name = initial_request
            .as_ref()
            .and_then(|request| request.file_name.clone())
            .unwrap_or_default();
        let url = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_url)
                .placeholder("https://example.com/file.zip")
        });
        let directory = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_directory)
                .placeholder("保存目录")
        });
        let file_name = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(initial_file_name)
                .placeholder("可选；留空时使用服务器文件名")
        });
        let default_scheduled_at = Local::now() + ChronoDuration::minutes(10);
        let scheduled_date = default_scheduled_at.date_naive();
        let scheduled_month = calendar_month_start(scheduled_date);
        let scheduled_hour = cx.new(|cx| {
            SelectState::new(
                (0..24).map(|hour| format!("{hour:02}")).collect(),
                Some(IndexPath::default().row(default_scheduled_at.hour() as usize)),
                window,
                cx,
            )
        });
        let scheduled_minute = cx.new(|cx| {
            SelectState::new(
                (0..60).map(|minute| format!("{minute:02}")).collect(),
                Some(IndexPath::default().row(default_scheduled_at.minute() as usize)),
                window,
                cx,
            )
        });
        let subscription = cx.subscribe_in(
            &application,
            window,
            |this, _application, event: &DownloadApplicationEvent, window, cx| {
                let DownloadApplicationEvent::DirectoryPicked { target, path } = event else {
                    return;
                };
                if *target != DownloadDirectoryTarget::NewTask {
                    return;
                }
                this.directory.update(cx, |input, cx| {
                    input.set_value(path.to_string_lossy().into_owned(), window, cx);
                });
                this.error = None;
                cx.notify();
            },
        );
        Self {
            theme: Theme::zed_dark(),
            application,
            url,
            directory,
            file_name,
            scheduled_date,
            scheduled_month,
            scheduled_hour,
            scheduled_minute,
            schedule_popover_open: false,
            committed_schedule: None,
            default_connection_limit,
            default_adaptive_segment_policy,
            external_request: initial_request,
            confirmed_request: None,
            probe: None,
            probing: false,
            submitting: false,
            probe_generation: 0,
            error: None,
            _subscription: subscription,
        }
    }

    fn request(&self, cx: &App) -> Result<DownloadRequest, String> {
        let url = self.url.read(cx).value().trim().to_owned();
        let directory = self.directory.read(cx).value().trim().to_owned();
        let file_name = self.file_name.read(cx).value().trim().to_owned();
        if url.is_empty() || directory.is_empty() {
            return Err("下载地址和保存目录不能为空".into());
        }
        let scheduled_at_unix_ms =
            resolve_scheduled_start(self.committed_schedule, Local::now().timestamp_millis())?;
        let mut request = self.external_request.clone().unwrap_or(DownloadRequest {
            source_url: String::new(),
            save_directory: PathBuf::new(),
            file_name: None,
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            connection_limit: Some(self.default_connection_limit),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: Some(self.default_adaptive_segment_policy),
        });
        request.source_url = url;
        request.save_directory = PathBuf::from(directory);
        request.file_name = (!file_name.is_empty()).then_some(file_name);
        request.connection_limit = Some(self.default_connection_limit);
        request.adaptive_segment_policy = Some(self.default_adaptive_segment_policy);
        request.scheduled_at_unix_ms = scheduled_at_unix_ms;
        request.validate().map_err(|error| error.to_string())?;
        Ok(request)
    }

    fn reset_schedule_draft(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let default = Local::now() + ChronoDuration::minutes(10);
        let selection = self.committed_schedule.unwrap_or(DelayedStartSelection {
            date: default.date_naive(),
            hour: default.hour(),
            minute: default.minute(),
        });
        self.scheduled_date = selection.date;
        self.scheduled_month = calendar_month_start(selection.date);
        self.scheduled_hour.update(cx, |hour, cx| {
            hour.set_selected_index(
                Some(IndexPath::default().row(selection.hour as usize)),
                window,
                cx,
            );
        });
        self.scheduled_minute.update(cx, |minute, cx| {
            minute.set_selected_index(
                Some(IndexPath::default().row(selection.minute as usize)),
                window,
                cx,
            );
        });
    }

    fn begin_probe(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let request = match self.request(cx) {
            Ok(request) => request,
            Err(error) => {
                self.error = Some(error);
                self.confirmed_request = None;
                self.probe = None;
                cx.notify();
                return;
            }
        };
        self.probe_generation = self.probe_generation.wrapping_add(1).max(1);
        let generation = self.probe_generation;
        self.probing = true;
        self.error = None;
        self.confirmed_request = None;
        self.probe = None;
        window.resize(new_download_window_size(false));
        info!(generation, "download link probe scheduled");
        let engine = self.application.read(cx).engine.clone();
        let background = cx.background_executor().clone();
        let source_window = window.window_handle();
        cx.spawn(async move |this, cx| {
            let request_for_probe = request.clone();
            let outcome = background
                .spawn(async move {
                    engine
                        .handle_command(DownloadCommandEnvelope {
                            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                            request_id: uuid::Uuid::new_v4(),
                            origin: RequestOrigin::Ui,
                            command: DownloadCommand::Probe {
                                request: request_for_probe,
                            },
                        })
                        .outcome
                })
                .await;
            let applied = this.update(cx, |this, cx| {
                if this.probe_generation != generation {
                    return None;
                }
                this.probing = false;
                let mut resolved_file_name = None;
                let confirmed = match outcome {
                    DownloadCommandOutcome::Probe(probe) => {
                        info!(
                            generation,
                            file_name = ?probe.file_name,
                            total_bytes = ?probe.total_bytes,
                            supports_ranges = probe.supports_ranges,
                            "download link probe completed"
                        );
                        let mut confirmed = request;
                        if confirmed.file_name.is_none() {
                            confirmed.file_name = probe.file_name.clone();
                            resolved_file_name = probe.file_name.clone();
                        }
                        this.confirmed_request = Some(confirmed);
                        this.probe = Some(probe);
                        this.error = None;
                        true
                    }
                    DownloadCommandOutcome::Rejected { fault } => {
                        warn!(
                            generation,
                            error = %fault.message,
                            "download link probe rejected"
                        );
                        this.error = Some(fault.message);
                        false
                    }
                    _ => {
                        this.error = Some("下载引擎返回了无效的探测结果".into());
                        false
                    }
                };
                cx.notify();
                Some((this.file_name.clone(), resolved_file_name, confirmed))
            });
            if let Ok(Some((file_name, resolved_file_name, confirmed))) = applied {
                let _ = source_window.update(cx, move |_, window, cx| {
                    if let Some(resolved_file_name) = resolved_file_name {
                        file_name.update(cx, |input, cx| {
                            input.set_value(resolved_file_name, window, cx);
                        });
                    }
                    window.resize(new_download_window_size(confirmed));
                });
            }
        })
        .detach();
        cx.notify();
    }

    fn submit(&mut self, mode: DownloadSubmitMode, window: &mut Window, cx: &mut Context<Self>) {
        if self.submitting {
            return;
        }
        let current = match self.request(cx) {
            Ok(request) => request,
            Err(error) => {
                self.error = Some(error);
                cx.notify();
                return;
            }
        };
        let Some(confirmed) = self.confirmed_request.clone() else {
            self.error = Some("请先探测链接并确认文件信息".into());
            cx.notify();
            return;
        };
        let Some(request) = merge_confirmed_download_request(confirmed, current) else {
            self.confirmed_request = None;
            self.probe = None;
            self.error = Some("下载地址已变更，请重新探测".into());
            cx.notify();
            return;
        };
        if request.save_directory.exists() && !request.save_directory.is_dir() {
            self.error = Some("保存位置不是文件夹，请重新选择".into());
            cx.notify();
            return;
        }
        if let Some(file_name) = request.file_name.as_deref() {
            let destination = request.save_directory.join(file_name);
            if destination.exists() {
                self.error = Some(format!(
                    "目标文件已存在：{}。请修改文件名或保存目录。",
                    destination.display()
                ));
                cx.notify();
                return;
            }
        }
        self.submitting = true;
        self.error = None;
        cx.notify();

        let application = self.application.clone();
        let source_window = window.window_handle();
        let view = cx.entity().downgrade();
        info!(?mode, "scheduling download task submission");
        cx.defer(move |cx| {
            let delayed = request.scheduled_at_unix_ms.is_some();
            let result =
                application.update(cx, |application, cx| application.submit(request, mode, cx));
            match result {
                Ok(task_id) => {
                    let show_progress_window = application.read(cx).show_progress_window;
                    let plan =
                        submit_presentation_plan(mode, task_id, delayed, show_progress_window);
                    if plan.close_source_window {
                        let _ = source_window.update(cx, |_, window, _| window.remove_window());
                    }
                    if let Some(task_id) = plan.deferred_task_window {
                        let application = application.clone();
                        cx.defer(move |cx| {
                            info!(
                                task_id = %task_id.as_uuid(),
                                "opening deferred download progress window"
                            );
                            application.update(cx, |application, cx| {
                                application.request_task_window(task_id, cx);
                            });
                        });
                    }
                }
                Err(message) => {
                    error!(error = %message, ?mode, "download task submission rejected");
                    let _ = view.update(cx, |view, cx| {
                        view.submitting = false;
                        view.error = Some(message);
                        cx.notify();
                    });
                }
            }
        });
    }
}

impl Render for NewDownloadView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let current_directory = self.directory.read(cx).value().to_string();
        let application = self.application.clone();
        let probe = self.probe.clone();
        let probing = self.probing;
        let submitting = self.submitting;
        let confirmed = probe.is_some();
        let schedule_label = if let Some(selection) = self.committed_schedule {
            format!(
                "{} {:02}:{:02}",
                selection.date.format("%m-%d"),
                selection.hour,
                selection.minute
            )
        } else {
            "稍后下载".into()
        };
        let schedule_view = cx.entity().downgrade();
        let schedule_date = self.scheduled_date;
        let schedule_month = self.scheduled_month;
        let schedule_today = Local::now().date_naive();
        let schedule_hour = self.scheduled_hour.clone();
        let schedule_minute = self.scheduled_minute.clone();
        let schedule_calendar_view = schedule_view.clone();
        let schedule_clear_view = schedule_view.clone();
        let schedule_confirm_view = schedule_view.clone();
        let schedule_confirm_hour = schedule_hour.clone();
        let schedule_confirm_minute = schedule_minute.clone();
        let schedule_primary_variant = download_primary_button_variant(theme, cx);
        let schedule_control = div()
            .id("download-new-schedule-control")
            .debug_selector(|| "download-new-schedule-control".into())
            .flex()
            .items_center()
            .child(
                Popover::new("download-new-schedule-popover")
                    .anchor(Corner::BottomLeft)
                    .open(self.schedule_popover_open)
                    .on_open_change(move |open, window, cx| {
                        let _ = schedule_view.update(cx, |view, cx| {
                            view.schedule_popover_open = *open;
                            if *open {
                                view.reset_schedule_draft(window, cx);
                            }
                            cx.notify();
                        });
                    })
                    .trigger(
                        Button::new("download-new-schedule-trigger")
                            .icon(Icon::new(MaterialIcon::CalendarMonth))
                            .label(schedule_label)
                            .small()
                            .outline(),
                    )
                    .content(move |_, _, _| {
                        let schedule_clear_view = schedule_clear_view.clone();
                        let schedule_confirm_view = schedule_confirm_view.clone();
                        let schedule_calendar_view = schedule_calendar_view.clone();
                        let schedule_confirm_hour = schedule_confirm_hour.clone();
                        let schedule_confirm_minute = schedule_confirm_minute.clone();
                        div()
                            .id("download-new-schedule-popover-content")
                            .debug_selector(|| "download-new-schedule-popover-content".into())
                            .w(px(286.0))
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .child(delayed_start_calendar(
                                theme,
                                schedule_month,
                                schedule_date,
                                schedule_today,
                                schedule_calendar_view,
                            ))
                            .child(
                                div()
                                    .pt(px(6.0))
                                    .border_t_1()
                                    .border_color(theme.border_subtle)
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .child(
                                        div()
                                            .mr(px(4.0))
                                            .text_size(px(9.5))
                                            .text_color(theme.text_secondary)
                                            .child("时间"),
                                    )
                                    .child(div().w(px(66.0)).child(
                                        Select::new(&schedule_hour).menu_width(px(66.0)).small(),
                                    ))
                                    .child(
                                        div()
                                            .text_size(px(11.0))
                                            .text_color(theme.text_muted)
                                            .child(":"),
                                    )
                                    .child(div().w(px(66.0)).child(
                                        Select::new(&schedule_minute).menu_width(px(66.0)).small(),
                                    ))
                                    .child(div().flex_1())
                                    .child(
                                        Button::new("download-new-schedule-clear")
                                            .label("清除")
                                            .small()
                                            .ghost()
                                            .on_click(move |_, _, cx| {
                                                info!("download delayed-start selection cleared");
                                                let _ =
                                                    schedule_clear_view.update(cx, |view, cx| {
                                                        view.committed_schedule = None;
                                                        view.schedule_popover_open = false;
                                                        view.error = None;
                                                        cx.notify();
                                                    });
                                            }),
                                    )
                                    .child(
                                        div()
                                            .debug_selector(|| {
                                                "download-new-schedule-confirm".into()
                                            })
                                            .child(
                                                Button::new("download-new-schedule-confirm-button")
                                                    .label("确定")
                                                    .small()
                                                    .custom(schedule_primary_variant)
                                                    .on_click(move |_, _, cx| {
                                                        let selection = read_delayed_start_draft(
                                                            schedule_date,
                                                            &schedule_confirm_hour,
                                                            &schedule_confirm_minute,
                                                            cx,
                                                        );
                                                        let _ = schedule_confirm_view.update(
                                                            cx,
                                                            |view, cx| {
                                                                let selection = match selection {
                                                                    Ok(selection) => selection,
                                                                    Err(error) => {
                                                                        view.error = Some(error);
                                                                        cx.notify();
                                                                        return;
                                                                    }
                                                                };
                                                                info!(
                                                                    "download delayed-start selection committed"
                                                                );
                                                                view.committed_schedule =
                                                                    Some(selection);
                                                                view.schedule_popover_open = false;
                                                                view.error = None;
                                                                cx.notify();
                                                            },
                                                        );
                                                    }),
                                            ),
                                    ),
                            )
                    }),
            );
        let footer_actions = if confirmed {
            Button::new("download-refresh-probe")
                .icon(Icon::new(MaterialIcon::ManageSearch))
                .small()
                .ghost()
                .disabled(submitting)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.begin_probe(window, cx);
                }))
                .into_any_element()
        } else {
            Button::new("download-probe")
                .icon(Icon::new(if probing {
                    MaterialIcon::ProgressActivity
                } else {
                    MaterialIcon::ManageSearch
                }))
                .label(if probing { "正在探测…" } else { "继续" })
                .small()
                .custom(download_primary_button_variant(theme, cx))
                .disabled(probing || submitting)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.begin_probe(window, cx);
                }))
                .into_any_element()
        };
        let footer_leading_actions = if confirmed {
            div()
                .debug_selector(|| "download-new-footer-leading-actions".into())
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(schedule_control)
                .child(footer_actions)
                .into_any_element()
        } else {
            footer_actions
        };
        let footer_submission_actions = confirmed.then(|| {
            div()
                .debug_selector(|| "download-new-footer-submission-actions".into())
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    Button::new("download-add-to-queue")
                        .label(if submitting {
                            "正在添加…"
                        } else {
                            "添加"
                        })
                        .small()
                        .ghost()
                        .disabled(submitting)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.submit(DownloadSubmitMode::QueueOnly, window, cx);
                        })),
                )
                .child(
                    Button::new("download-start-now")
                        .icon(Icon::new(MaterialIcon::Download))
                        .label(if submitting {
                            "正在添加…"
                        } else {
                            "下载"
                        })
                        .small()
                        .custom(download_primary_button_variant(theme, cx))
                        .disabled(submitting)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.submit(DownloadSubmitMode::StartNow, window, cx);
                        })),
                )
                .into_any_element()
        });
        let body = if let Some(probe) = probe {
            let size = probe
                .total_bytes
                .map(format_download_size)
                .unwrap_or_else(|| "大小未知".into());
            div()
                .flex_1()
                .px(px(16.0))
                .py(px(10.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(download_input_field(theme, "下载地址", &self.url))
                .child(
                    div()
                        .flex()
                        .items_start()
                        .gap(px(12.0))
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(7.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_end()
                                        .gap(px(6.0))
                                        .child(div().min_w_0().flex_1().child(
                                            download_input_field(
                                                theme,
                                                "保存目录",
                                                &self.directory,
                                            ),
                                        ))
                                        .child(
                                            Button::new("download-pick-directory")
                                                .icon(Icon::new(MaterialIcon::Folder))
                                                .small()
                                                .ghost()
                                                .on_click(move |_, _, cx| {
                                                    let application = application.clone();
                                                    let current_directory =
                                                        current_directory.clone();
                                                    cx.defer(move |cx| {
                                                        application.update(
                                                            cx,
                                                            |application, cx| {
                                                                application
                                                                    .request_new_download_directory(
                                                                        current_directory,
                                                                        cx,
                                                                    );
                                                            },
                                                        );
                                                    });
                                                }),
                                        ),
                                )
                                .child(download_input_field(theme, "文件名", &self.file_name)),
                        )
                        .child(
                            div()
                                .w(px(118.0))
                                .h(px(82.0))
                                .pl(px(12.0))
                                .border_l_1()
                                .border_color(theme.border_subtle)
                                .flex()
                                .flex_col()
                                .justify_center()
                                .gap(px(5.0))
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap(px(6.0))
                                        .child(
                                            Icon::new(MaterialIcon::Check)
                                                .size(px(14.0))
                                                .text_color(theme.success),
                                        )
                                        .child(
                                            div()
                                                .text_size(px(9.5))
                                                .font_weight(FontWeight::MEDIUM)
                                                .text_color(theme.success)
                                                .child("链接可用"),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .child(size),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.0))
                                        .text_color(if probe.supports_ranges {
                                            theme.success
                                        } else {
                                            theme.text_muted
                                        })
                                        .child(if probe.supports_ranges {
                                            "支持断点续传"
                                        } else {
                                            "单连接传输"
                                        }),
                                ),
                        ),
                )
                .when_some(self.error.clone(), |body, error| {
                    body.child(download_inline_error(theme, error))
                })
                .into_any_element()
        } else {
            div()
                .flex_1()
                .px(px(16.0))
                .py(px(10.0))
                .flex()
                .flex_col()
                .gap(px(8.0))
                .child(download_input_field(theme, "下载地址", &self.url))
                .child(
                    div()
                        .h(px(18.0))
                        .flex()
                        .items_center()
                        .gap(px(6.0))
                        .child(
                            Icon::new(if probing {
                                MaterialIcon::ProgressActivity
                            } else {
                                MaterialIcon::Info
                            })
                            .size(px(13.0))
                            .text_color(if probing {
                                theme.accent
                            } else {
                                theme.icon_muted
                            }),
                        )
                        .child(
                            div()
                                .text_size(px(9.5))
                                .text_color(if probing {
                                    theme.text_secondary
                                } else {
                                    theme.text_muted
                                })
                                .child(if probing {
                                    "正在读取远端文件名、大小与续传能力…"
                                } else {
                                    "输入 HTTP/HTTPS 链接，继续后自动获取文件信息"
                                }),
                        ),
                )
                .when_some(self.error.clone(), |body, error| {
                    body.child(download_inline_error(theme, error))
                })
                .into_any_element()
        };
        div()
            .id("download-new-root")
            .debug_selector(|| "download-new-root".into())
            .size_full()
            .bg(theme.canvas)
            .text_color(theme.text)
            .flex()
            .flex_col()
            .child(
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
                            .flex()
                            .items_center()
                            .gap(px(7.0))
                            .text_size(px(12.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(
                                Icon::new(MaterialIcon::Download)
                                    .size(px(15.0))
                                    .text_color(theme.accent),
                            )
                            .child("新建下载"),
                    )
                    .child(download_window_close_button(theme)),
            )
            .child(body)
            .child(
                div()
                    .id("download-new-footer")
                    .debug_selector(|| "download-new-footer".into())
                    .h(px(44.0))
                    .px(px(16.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .child(footer_leading_actions)
                    .child(div().flex_1())
                    .when_some(footer_submission_actions, |footer, actions| {
                        footer.child(actions).child(div().w(px(6.0)))
                    })
                    .child(
                        Button::new("download-new-cancel")
                            .label("取消")
                            .small()
                            .ghost()
                            .disabled(submitting)
                            .on_click(|_, window, _| window.remove_window()),
                    ),
            )
    }
}

fn delayed_start_calendar(
    theme: Theme,
    month: NaiveDate,
    selected_date: NaiveDate,
    today: NaiveDate,
    view: gpui::WeakEntity<NewDownloadView>,
) -> AnyElement {
    let previous_view = view.clone();
    let next_view = view.clone();
    let weekdays = ["日", "一", "二", "三", "四", "五", "六"];
    let days = delayed_start_calendar_days(month);
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(2.0))
        .child(
            div()
                .h(px(22.0))
                .flex()
                .items_center()
                .child(
                    Button::new("download-new-schedule-previous-month")
                        .icon(Icon::new(MaterialIcon::ChevronLeft))
                        .xsmall()
                        .ghost()
                        .on_click(move |_, _, cx| {
                            let _ = previous_view.update(cx, |view, cx| {
                                view.scheduled_month =
                                    shift_calendar_month(view.scheduled_month, -1);
                                cx.notify();
                            });
                        }),
                )
                .child(
                    div()
                        .debug_selector(|| "download-new-schedule-month-title".into())
                        .flex_1()
                        .text_center()
                        .text_size(px(11.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(theme.text)
                        .child(delayed_start_calendar_title(month)),
                )
                .child(
                    Button::new("download-new-schedule-next-month")
                        .icon(Icon::new(MaterialIcon::ChevronRight))
                        .xsmall()
                        .ghost()
                        .on_click(move |_, _, cx| {
                            let _ = next_view.update(cx, |view, cx| {
                                view.scheduled_month =
                                    shift_calendar_month(view.scheduled_month, 1);
                                cx.notify();
                            });
                        }),
                ),
        )
        .child(
            div()
                .h(px(16.0))
                .flex()
                .children(weekdays.into_iter().map(|weekday| {
                    div()
                        .w(px(40.0))
                        .h_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(8.5))
                        .text_color(theme.text_muted)
                        .child(weekday)
                })),
        )
        .children((0..6).map(|row| {
            div()
                .h(px(22.0))
                .flex()
                .children(days[row * 7..row * 7 + 7].iter().copied().map(|date| {
                    let disabled = date < today;
                    let selected = date == selected_date;
                    let current_month = date.month() == month.month();
                    let is_today = date == today;
                    let day_view = view.clone();
                    div()
                        .id(("download-new-schedule-day", date.num_days_from_ce() as u64))
                        .w(px(40.0))
                        .h(px(22.0))
                        .rounded(px(5.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(9.0))
                        .text_color(if selected {
                            theme.accent_text
                        } else if current_month {
                            theme.text_secondary
                        } else {
                            theme.text_muted
                        })
                        .when(disabled, |cell| cell.opacity(0.32))
                        .when(is_today && !selected, |cell| {
                            cell.border_1().border_color(theme.border_active)
                        })
                        .when(selected, |cell| cell.bg(theme.accent))
                        .when(!disabled, |cell| {
                            cell.cursor_pointer()
                                .hover(|cell| {
                                    if selected {
                                        cell.bg(theme.accent)
                                    } else {
                                        cell.bg(theme.surface_hover)
                                    }
                                })
                                .on_click(move |_, _, cx| {
                                    let _ = day_view.update(cx, |view, cx| {
                                        view.scheduled_date = date;
                                        view.scheduled_month = calendar_month_start(date);
                                        cx.notify();
                                    });
                                })
                        })
                        .child(date.day().to_string())
                }))
        }))
        .into_any_element()
}

fn download_inline_error(theme: Theme, error: String) -> impl IntoElement {
    div()
        .max_w_full()
        .text_size(px(9.5))
        .text_color(theme.danger)
        .whitespace_normal()
        .line_clamp(2)
        .child(error)
}

fn format_download_size(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;
    let bytes = bytes as f64;
    if bytes >= GIB {
        format!("{:.2} GB", bytes / GIB)
    } else if bytes >= MIB {
        format!("{:.1} MB", bytes / MIB)
    } else if bytes >= KIB {
        format!("{:.1} KB", bytes / KIB)
    } else {
        format!("{bytes:.0} B")
    }
}

fn new_download_window_size(confirmed: bool) -> gpui::Size<gpui::Pixels> {
    if confirmed {
        size(px(500.0), px(278.0))
    } else {
        size(px(500.0), px(176.0))
    }
}

fn download_task_window_size(phase: DownloadWindowPhase) -> gpui::Size<gpui::Pixels> {
    match phase {
        DownloadWindowPhase::Progress => size(px(440.0), px(176.0)),
        DownloadWindowPhase::Failed => size(px(440.0), px(196.0)),
        DownloadWindowPhase::Completed => size(px(440.0), px(180.0)),
    }
}

#[allow(
    dead_code,
    reason = "retained for the Phase 3 task context-menu delete action"
)]
struct DeleteDownloadView {
    theme: Theme,
    application: Entity<DownloadApplication>,
    task_id: DownloadTaskId,
    path: String,
}

impl Render for DeleteDownloadView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let task_id = self.task_id;
        let application = self.application.clone();
        let path = self.path.clone();
        div()
            .size_full()
            .bg(theme.canvas)
            .text_color(theme.text)
            .flex()
            .flex_col()
            .child(download_window_titlebar(theme, "删除本地文件"))
            .child(
                div()
                    .flex_1()
                    .px(px(16.0))
                    .py(px(12.0))
                    .flex()
                    .flex_col()
                    .gap(px(5.0))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .font_weight(FontWeight::MEDIUM)
                            .child("将同时移除任务记录，且无法撤销。"),
                    )
                    .child(
                        div()
                            .text_size(px(9.5))
                            .text_color(theme.text_muted)
                            .truncate()
                            .child(path),
                    ),
            )
            .child(
                div()
                    .h(px(48.0))
                    .px(px(14.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(6.0))
                    .child(
                        Button::new("download-delete-cancel")
                            .label("取消")
                            .small()
                            .ghost()
                            .on_click(|_, window, _| window.remove_window()),
                    )
                    .child(
                        Button::new("download-delete-confirm")
                            .icon(Icon::new(MaterialIcon::Delete))
                            .label("删除文件")
                            .small()
                            .danger()
                            .on_click(move |_, window, cx| {
                                let result = application.update(cx, |application, cx| {
                                    application.remove(task_id, true, cx)
                                });
                                if result.is_ok() {
                                    window.remove_window();
                                }
                            }),
                    ),
            )
    }
}

struct DownloadTaskWindowView {
    theme: Theme,
    application: Entity<DownloadApplication>,
    task_id: DownloadTaskId,
    snapshot: Option<DownloadEngineTask>,
    progress: Option<DownloadProgress>,
}

impl DownloadTaskWindowView {
    fn new(
        application: Entity<DownloadApplication>,
        task_id: DownloadTaskId,
        snapshot: Option<DownloadEngineTask>,
        progress: Option<DownloadProgress>,
    ) -> Self {
        Self {
            theme: Theme::zed_dark(),
            application,
            task_id,
            snapshot,
            progress,
        }
    }

    fn replace_snapshot(
        &mut self,
        snapshot: Option<DownloadEngineTask>,
        progress: Option<DownloadProgress>,
        cx: &mut Context<Self>,
    ) {
        self.snapshot = snapshot;
        self.progress = progress;
        cx.notify();
    }
}

impl Render for DownloadTaskWindowView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let application = self.application.clone();
        let task_id = self.task_id;
        let snapshot = self.snapshot.clone();
        let progress_event = self.progress.clone();
        let Some(task) = snapshot else {
            return div()
                .size_full()
                .bg(theme.canvas)
                .text_color(theme.text)
                .flex()
                .flex_col()
                .child(download_window_titlebar(theme, "下载任务"))
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(px(11.0))
                        .text_color(theme.text_muted)
                        .child("任务已被移除"),
                )
                .into_any_element();
        };

        let status = task.task.status();
        let name = task
            .final_path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .or(task.task.request().file_name.as_deref())
            .or_else(|| {
                task.probe
                    .as_ref()
                    .and_then(|probe| probe.file_name.as_deref())
            })
            .unwrap_or("正在解析文件名")
            .to_owned();
        let final_path = task.final_path.as_ref().map_or_else(
            || task.task.request().save_directory.display().to_string(),
            |path| path.display().to_string(),
        );

        if status == DownloadStatus::Completed {
            let open_application = application.clone();
            let reveal_application = application.clone();
            let drag_application = application.clone();
            return div()
                .size_full()
                .bg(theme.canvas)
                .text_color(theme.text)
                .flex()
                .flex_col()
                .child(download_window_titlebar(theme, "下载完成"))
                .child(
                    div()
                        .flex_1()
                        .px(px(16.0))
                        .py(px(10.0))
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(
                            div()
                                .size(px(38.0))
                                .rounded(px(6.0))
                                .bg(theme.accent_soft)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new(MaterialIcon::Check)
                                        .size(px(19.0))
                                        .text_color(theme.success),
                                ),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex_1()
                                .flex()
                                .flex_col()
                                .gap(px(3.0))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .truncate()
                                        .child(name),
                                )
                                .child(
                                    div()
                                        .text_size(px(10.0))
                                        .text_color(theme.text_muted)
                                        .child(
                                            task.total_bytes
                                                .map(format_download_size)
                                                .unwrap_or_else(|| {
                                                    format_download_size(task.downloaded_bytes)
                                                }),
                                        ),
                                )
                                .child(
                                    div()
                                        .text_size(px(9.5))
                                        .text_color(theme.text_muted)
                                        .truncate()
                                        .child(final_path),
                                ),
                        ),
                )
                .child(
                    div()
                        .h(px(44.0))
                        .px(px(14.0))
                        .border_t_1()
                        .border_color(theme.border_subtle)
                        .bg(theme.surface)
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(6.0))
                        .child(
                            div()
                                .id("download-complete-drag")
                                .h(px(28.0))
                                .px(px(8.0))
                                .rounded(px(4.0))
                                .border_1()
                                .border_color(theme.border_subtle)
                                .bg(theme.surface_raised)
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .gap(px(5.0))
                                .text_size(px(10.0))
                                .hover(move |style| {
                                    style.bg(theme.surface_hover).border_color(theme.border)
                                })
                                .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                                    let source_window_handle =
                                        native_window_handle(window).unwrap_or_default();
                                    drag_application.update(cx, |application, cx| {
                                        application.begin_task_file_drag(
                                            task_id,
                                            source_window_handle,
                                            cx,
                                        );
                                    });
                                    cx.stop_propagation();
                                })
                                .child(
                                    Icon::new(MaterialIcon::OpenInFull)
                                        .size(px(14.0))
                                        .text_color(theme.icon_muted),
                                )
                                .child("拖拽文件"),
                        )
                        .child(
                            Button::new("download-complete-reveal")
                                .icon(Icon::new(MaterialIcon::Folder))
                                .label("打开所在目录")
                                .small()
                                .ghost()
                                .on_click(move |_, _, cx| {
                                    let application = reveal_application.clone();
                                    cx.defer(move |cx| {
                                        application.update(cx, |application, cx| {
                                            application.open_task_artifact(
                                                task_id,
                                                ShellOpenAction::SelectInExplorer,
                                                cx,
                                            );
                                        });
                                    });
                                }),
                        )
                        .child(
                            Button::new("download-complete-open")
                                .icon(Icon::new(MaterialIcon::OpenInNew))
                                .label("打开文件")
                                .small()
                                .custom(download_primary_button_variant(theme, cx))
                                .on_click(move |_, _, cx| {
                                    let application = open_application.clone();
                                    cx.defer(move |cx| {
                                        application.update(cx, |application, cx| {
                                            application.open_task_artifact(
                                                task_id,
                                                ShellOpenAction::Open,
                                                cx,
                                            );
                                        });
                                    });
                                }),
                        ),
                )
                .into_any_element();
        }

        if status == DownloadStatus::Failed {
            let cancel_application = application.clone();
            let retry_application = application.clone();
            let error = task.error.as_ref().map_or_else(
                || "下载任务失败，未返回更多信息。".into(),
                |error| error.message.clone(),
            );
            return div()
                .size_full()
                .bg(theme.canvas)
                .text_color(theme.text)
                .flex()
                .flex_col()
                .child(download_window_titlebar(theme, "下载失败"))
                .child(
                    div()
                        .flex_1()
                        .px(px(16.0))
                        .py(px(10.0))
                        .flex()
                        .flex_col()
                        .gap(px(8.0))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(8.0))
                                .child(
                                    Icon::new(MaterialIcon::Download)
                                        .size(px(17.0))
                                        .text_color(theme.danger),
                                )
                                .child(
                                    div()
                                        .min_w_0()
                                        .flex_1()
                                        .text_size(px(11.5))
                                        .font_weight(FontWeight::MEDIUM)
                                        .truncate()
                                        .child(name),
                                ),
                        )
                        .child(
                            div()
                                .w_full()
                                .px(px(9.0))
                                .py(px(7.0))
                                .rounded(px(5.0))
                                .bg(theme.danger_soft)
                                .text_size(px(9.5))
                                .text_color(theme.danger)
                                .truncate()
                                .child(error),
                        )
                        .child(
                            div()
                                .text_size(px(9.0))
                                .text_color(theme.text_muted)
                                .truncate()
                                .child(final_path),
                        ),
                )
                .child(
                    div()
                        .h(px(44.0))
                        .px(px(14.0))
                        .border_t_1()
                        .border_color(theme.border_subtle)
                        .bg(theme.surface)
                        .flex()
                        .items_center()
                        .justify_end()
                        .gap(px(6.0))
                        .child(
                            Button::new("download-window-failed-cancel")
                                .label("取消任务")
                                .small()
                                .ghost()
                                .on_click(move |_, _, cx| {
                                    let application = cancel_application.clone();
                                    cx.defer(move |cx| {
                                        application.update(cx, |application, cx| {
                                            let _ = application.cancel(task_id, cx);
                                        });
                                    });
                                }),
                        )
                        .child(
                            Button::new("download-window-failed-retry")
                                .icon(Icon::new(MaterialIcon::Download))
                                .label("重试")
                                .small()
                                .custom(download_primary_button_variant(theme, cx))
                                .on_click(move |_, _, cx| {
                                    let application = retry_application.clone();
                                    cx.defer(move |cx| {
                                        application.update(cx, |application, cx| {
                                            let _ = application.resume(task_id, cx);
                                        });
                                    });
                                }),
                        ),
                )
                .into_any_element();
        }

        let total = task
            .total_bytes
            .or_else(|| task.probe.as_ref().and_then(|probe| probe.total_bytes));
        let downloaded = progress_event
            .as_ref()
            .map_or(task.downloaded_bytes, |progress| progress.downloaded_bytes);
        let ratio = total
            .filter(|total| *total > 0)
            .map(|total| (downloaded as f32 / total as f32).clamp(0.0, 1.0));
        let speed = progress_event
            .as_ref()
            .map_or(0, |progress| progress.bytes_per_second);
        let connections = progress_event
            .as_ref()
            .map_or(0, |progress| progress.active_connections);
        let progress_text = total.map_or_else(
            || format_download_size(downloaded),
            |total| {
                format!(
                    "{} / {}",
                    format_download_size(downloaded),
                    format_download_size(total)
                )
            },
        );
        let eta = total
            .zip((speed > 0).then_some(speed))
            .map(|(total, speed)| total.saturating_sub(downloaded) / speed)
            .map(format_download_eta)
            .unwrap_or_else(|| "—".into());
        let (status_label, status_color) = download_task_status(status);
        let progress_badge = ratio
            .map(|ratio| format!("{:.0}%", ratio * 100.0))
            .unwrap_or_else(|| status_label.to_owned());
        let action_application = application.clone();
        let secondary_application = application.clone();
        let actions = match status {
            DownloadStatus::Created | DownloadStatus::Paused | DownloadStatus::Failed => div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    Button::new("download-window-cancel")
                        .label("取消任务")
                        .small()
                        .ghost()
                        .on_click(move |_, _, cx| {
                            let application = secondary_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    let _ = application.cancel(task_id, cx);
                                });
                            });
                        }),
                )
                .child(
                    Button::new("download-window-resume")
                        .icon(Icon::new(MaterialIcon::Download))
                        .label(if status == DownloadStatus::Failed {
                            "重试"
                        } else {
                            "继续"
                        })
                        .small()
                        .custom(download_primary_button_variant(theme, cx))
                        .on_click(move |_, _, cx| {
                            let application = action_application.clone();
                            cx.defer(move |cx| {
                                application.update(cx, |application, cx| {
                                    let _ = application.resume(task_id, cx);
                                });
                            });
                        }),
                )
                .into_any_element(),
            DownloadStatus::Probing | DownloadStatus::Waiting | DownloadStatus::Downloading => {
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(
                        Button::new("download-window-cancel-active")
                            .label("取消")
                            .small()
                            .ghost()
                            .on_click(move |_, _, cx| {
                                let application = secondary_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        let _ = application.cancel(task_id, cx);
                                    });
                                });
                            }),
                    )
                    .child(
                        Button::new("download-window-pause")
                            .label("暂停")
                            .small()
                            .custom(download_primary_button_variant(theme, cx))
                            .on_click(move |_, _, cx| {
                                let application = action_application.clone();
                                cx.defer(move |cx| {
                                    application.update(cx, |application, cx| {
                                        let _ = application.pause(task_id, cx);
                                    });
                                });
                            }),
                    )
                    .into_any_element()
            }
            _ => div().into_any_element(),
        };

        div()
            .size_full()
            .bg(theme.canvas)
            .text_color(theme.text)
            .flex()
            .flex_col()
            .child(download_window_titlebar(theme, status_label))
            .child(
                div()
                    .flex_1()
                    .px(px(16.0))
                    .py(px(8.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(9.0))
                            .child(
                                Icon::new(MaterialIcon::Download)
                                    .size(px(16.0))
                                    .text_color(status_color),
                            )
                            .child(
                                div()
                                    .min_w_0()
                                    .flex_1()
                                    .text_size(px(11.5))
                                    .font_weight(FontWeight::MEDIUM)
                                    .truncate()
                                    .child(name),
                            )
                            .child(
                                div()
                                    .text_size(px(9.5))
                                    .text_color(status_color)
                                    .child(progress_badge),
                            ),
                    )
                    .child(
                        div()
                            .w_full()
                            .h(px(5.0))
                            .rounded(px(4.0))
                            .bg(theme.surface_raised)
                            .when_some(ratio, |track, ratio| {
                                track.child(
                                    div()
                                        .h_full()
                                        .w(px(408.0 * ratio))
                                        .rounded(px(4.0))
                                        .bg(status_color),
                                )
                            }),
                    )
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_start()
                            .gap(px(12.0))
                            .child(download_window_metric(theme, "已下载", progress_text))
                            .child(download_window_metric(
                                theme,
                                "速度",
                                format!("{} /s", format_download_size(speed)),
                            ))
                            .child(download_window_metric(theme, "剩余", eta))
                            .child(download_window_metric(
                                theme,
                                "连接",
                                connections.to_string(),
                            )),
                    )
                    .child(
                        div()
                            .text_size(px(8.5))
                            .text_color(theme.text_muted)
                            .truncate()
                            .child(final_path),
                    ),
            )
            .child(
                div()
                    .h(px(42.0))
                    .px(px(14.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .justify_end()
                    .child(actions),
            )
            .into_any_element()
    }
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
                .text_size(px(8.5))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .text_size(px(10.0))
                .font_weight(FontWeight::MEDIUM)
                .truncate()
                .child(value),
        )
}

fn download_window_titlebar(theme: Theme, title: &'static str) -> impl IntoElement {
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
                .text_size(px(12.0))
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
                .text_size(px(12.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child("下载设置"),
        )
        .child(
            div()
                .id("download-settings-close")
                .w(px(38.0))
                .h_full()
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_center()
                .hover(move |style| style.bg(theme.danger).text_color(gpui::white()))
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_click(move |_, window, cx| {
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
                })
                .child(Icon::new(MaterialIcon::Close).size(px(15.0))),
        )
}

fn download_primary_button_variant(theme: Theme, cx: &gpui::App) -> ButtonCustomVariant {
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

fn download_input_field(
    theme: Theme,
    label: &'static str,
    input: &Entity<InputState>,
) -> impl IntoElement {
    div()
        .w_full()
        .flex()
        .flex_col()
        .gap(px(3.0))
        .child(
            div()
                .text_size(px(9.5))
                .text_color(theme.text_muted)
                .child(label),
        )
        .child(
            div()
                .h(px(30.0))
                .rounded(px(5.0))
                .border_1()
                .border_color(theme.border)
                .bg(theme.surface_raised)
                .px(px(7.0))
                .flex()
                .items_center()
                .child(
                    Input::new(input)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false),
                ),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, Modifiers, TestAppContext};

    #[test]
    fn live_progress_updates_the_cached_selected_segment_without_requerying_sqlite() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/live.bin".into(),
                    save_directory: std::env::temp_dir(),
                    file_name: Some("live.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    connection_limit: Some(2),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
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
        let (_root, window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                NewDownloadView::new(
                    application_for_window,
                    default_directory,
                    default_connection_limit,
                    default_adaptive_segment_policy,
                    None,
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
            file_name: Some("file.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            connection_limit: Some(default_connection_limit),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
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
            file_name: Some("private.bin".into()),
            headers: vec![nxtools_download::DownloadRequestHeader {
                name: "Authorization".into(),
                value: "Bearer secret".into(),
            }],
            cookie: Some("session=secret".into()),
            referrer: Some("https://example.test/page".into()),
            connection_limit: Some(4),
            speed_limit_bytes_per_second: Some(512_000),
            expected_total_bytes: Some(8_192),
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
        };
        let expected = initial.clone();
        let default_directory = directory.clone();
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let (_root, _window) = cx.add_window_view(move |window, cx| {
            let view = cx.new(|cx| {
                NewDownloadView::new(
                    application_for_window,
                    default_directory,
                    4,
                    nxtools_download::DownloadAdaptiveSegmentPolicy::default(),
                    Some(initial),
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

    fn download_application_for_window_test(
        cx: &mut TestAppContext,
        label: &str,
    ) -> (Entity<DownloadApplication>, PathBuf) {
        let directory = unique_download_settings_test_directory(label);
        std::fs::create_dir_all(&directory).expect("create download window test directory");
        let engine = DownloadEngine::open_in_memory().expect("open download window test engine");
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
                        config: AppConfig::default(),
                        config_path: directory.join("config.json"),
                        log_directory: directory.clone(),
                        database_path: directory.join("downloads.sqlite3"),
                    },
                    cx,
                )
            })
        });
        (application, directory)
    }

    #[gpui::test]
    fn schedule_window_opens_without_reentering_download_application(cx: &mut TestAppContext) {
        cx.update(gpui_component::init);
        let (application, directory) = download_application_for_window_test(cx, "schedule-window");

        cx.update(|cx| {
            application.update(cx, |application, cx| {
                application.open_schedule_window(cx);
            });
        });

        assert!(cx.read(|cx| application.read(cx).schedule_window.is_some()));
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

        assert!(cx.read(|cx| application.read(cx).settings_window.is_some()));
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
    fn settings_window_routes_picker_applies_and_cancels_without_cross_module_loss(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_component::init);
        let directory = unique_download_settings_test_directory("window");
        let selected_directory = directory.join("selected");
        std::fs::create_dir_all(&selected_directory).expect("create selected directory");
        let config_path = directory.join("config.json");
        let mut config = AppConfig::default();
        config.quick_launch.enabled = false;
        config.save_to(&config_path).expect("save settings fixture");
        let engine = DownloadEngine::open_in_memory().expect("open settings engine");
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
        let completion_toggle = window
            .debug_bounds("download-settings-completion-window")
            .expect("completion setting toggle");
        window.simulate_click(completion_toggle.center(), Modifiers::default());
        window.update(|_, cx| {
            settings.update(cx, |settings, cx| {
                settings.scroll.scroll_to_bottom();
                cx.notify();
            });
        });
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
        assert!(!persisted.download.show_completion_window);
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
                        config: AppConfig::default(),
                        config_path: missing_config,
                        log_directory: directory.clone(),
                        database_path: directory.join("downloads.sqlite3"),
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
                        default_connection_limit: 12,
                        adaptive_segment_scaling_enabled: true,
                        adaptive_segment_max_segments: 32,
                        adaptive_segment_min_mib: 3,
                        max_concurrent_tasks: 7,
                        global_speed_limit_bytes_per_second: Some(1024 * 1024),
                        show_progress_window: false,
                        show_completion_window: false,
                        taskbar_progress_overlay: DownloadTaskbarProgressOverlaySettings::default(),
                    },
                    uuid::Uuid::new_v4(),
                    cx,
                )
            })
        });
        assert!(result.is_err());
        assert_eq!(engine_for_assertion.global_speed_limit(), None);
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
        let source = include_str!("downloads.rs");
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
            (440.0, 176.0)
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
            (500.0, 176.0)
        );
        assert_eq!(
            (confirmed.width.as_f32(), confirmed.height.as_f32()),
            (500.0, 278.0)
        );
    }

    #[test]
    fn confirmed_probe_keeps_destination_and_filename_editable() {
        let confirmed = DownloadRequest {
            source_url: "https://example.com/original".into(),
            save_directory: PathBuf::from(r"C:\Downloads"),
            file_name: Some("server-name.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
        };
        let current = DownloadRequest {
            save_directory: PathBuf::from(r"D:\Archive"),
            file_name: Some("renamed.bin".into()),
            ..confirmed.clone()
        };

        let merged = merge_confirmed_download_request(confirmed, current)
            .expect("destination-only edits do not invalidate a network probe");

        assert_eq!(merged.save_directory, PathBuf::from(r"D:\Archive"));
        assert_eq!(merged.file_name.as_deref(), Some("renamed.bin"));
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
            begin_probe.contains("Some((this.file_name.clone()")
                && begin_probe.contains("file_name.update"),
            "the resolved server filename must be written into the editable file-name field"
        );
        assert!(
            begin_probe.contains("input.set_value"),
            "the file-name input must visibly update when probing completes"
        );
    }
}
