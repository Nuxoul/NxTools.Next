use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

use chrono::{Local, NaiveDateTime, TimeZone};
use gpui::{
    AnyElement, AnyWindowHandle, App, Context, Entity, EventEmitter, FontWeight, IntoElement,
    MouseButton, Render, Subscription, Task, Window, WindowBackgroundAppearance, WindowBounds,
    WindowControlArea, WindowDecorations, WindowKind, WindowOptions, div, prelude::*, px, size,
};
use gpui_component::button::{Button, ButtonCustomVariant, ButtonVariants};
use gpui_component::input::{Input, InputState};
use gpui_component::{Disableable, Icon, PixelsExt, Root, Sizable};
use nxtools_download::{
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadBatchAction, DownloadCommand,
    DownloadCommandEnvelope, DownloadCommandOutcome, DownloadDailySchedule, DownloadEngine,
    DownloadEngineTask, DownloadEvent, DownloadEventEnvelope, DownloadProbe, DownloadProgress,
    DownloadQuery, DownloadQueue, DownloadQueueAction, DownloadQueuePlacement, DownloadRequest,
    DownloadSchedule, DownloadSegment, DownloadSegmentState, DownloadSnapshot, DownloadStatus,
    DownloadSubmitMode, DownloadTaskAction, DownloadTaskDetail, DownloadTaskId, DownloadWeekday,
    RequestOrigin,
};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{
    ActivateWindowCommand, BeginFileDragCommand, OpenShellTargetCommand, PickDirectoryCommand,
    PlatformCommand, PlatformCommandResult, SetTaskbarProgressCommand, SetWindowBoundsCommand,
    SetWindowTopmostCommand, ShellOpenAction, TaskbarProgressState,
};
use tracing::{error, info, warn};

use crate::download_window_lifecycle::{
    DownloadWindowDirective, DownloadWindowLifecycle, DownloadWindowPhase,
};
use crate::icons::MaterialIcon;
use crate::notifications::NotificationCenter;
use crate::theme::Theme;
use crate::ui::native_window_handle;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DownloadApplicationEvent {
    DirectoryPicked(PathBuf),
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
) -> DownloadSubmitPresentationPlan {
    DownloadSubmitPresentationPlan {
        close_source_window: true,
        deferred_task_window: (mode == DownloadSubmitMode::StartNow && !delayed).then_some(task_id),
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
    show_completion_window: bool,
    pending_directory_picker: Option<uuid::Uuid>,
    notified_terminal_states: HashSet<(DownloadTaskId, DownloadStatus)>,
    last_event_sequence: u64,
    _event_task: Task<()>,
}

impl EventEmitter<DownloadApplicationEvent> for DownloadApplication {}

impl DownloadApplication {
    pub(crate) fn new(
        engine: DownloadEngine,
        platform: Rc<PlatformBridge>,
        notifications: Entity<NotificationCenter>,
        default_directory: PathBuf,
        show_completion_window: bool,
        cx: &mut Context<Self>,
    ) -> Self {
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
            show_completion_window,
            pending_directory_picker: None,
            notified_terminal_states,
            last_event_sequence: 0,
            _event_task: event_task,
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

    pub(crate) fn tasks(&self) -> &[DownloadEngineTask] {
        &self.tasks
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
        self.engine
            .handle_command(DownloadCommandEnvelope {
                protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                request_id: uuid::Uuid::new_v4(),
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

    pub(crate) fn request_directory(&mut self, current_directory: String, cx: &mut Context<Self>) {
        if self.pending_directory_picker.is_some() {
            return;
        }
        match self
            .platform
            .send_command(PlatformCommand::PickDirectory(PickDirectoryCommand {
                current_directory,
                title: "选择下载保存目录".into(),
            })) {
            Ok(command_id) => self.pending_directory_picker = Some(command_id),
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

    pub(crate) fn open_new_download_window(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = self.new_download_window {
            if window
                .update(cx, |_, window, _| {
                    window.activate_window();
                    window.refresh();
                })
                .is_ok()
            {
                return;
            }
            self.new_download_window = None;
        }
        let application = cx.entity();
        let default_directory = self.default_directory.clone();
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
                let view =
                    cx.new(|cx| NewDownloadView::new(application, default_directory, window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => self.new_download_window = Some(window.into()),
            Err(error) => {
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        "download-new-window",
                        "无法打开新建下载窗口",
                        error.to_string(),
                        cx,
                    );
                });
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
                let view = cx.new(|cx| DownloadScheduleView::new(application, window, cx));
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
        let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = notification
        else {
            return;
        };
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
        if self.pending_directory_picker != Some(correlation_id) {
            return;
        }
        self.pending_directory_picker = None;
        self.handle_directory_result(result, cx);
    }

    fn handle_directory_result(&mut self, result: PlatformCommandResult, cx: &mut Context<Self>) {
        if result.success {
            if let Some(path) = result.data.filter(|path| !path.trim().is_empty()) {
                cx.emit(DownloadApplicationEvent::DirectoryPicked(PathBuf::from(
                    path,
                )));
            }
        } else {
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
        }
        cx.notify();
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.refresh_snapshot(true, cx);
    }

    fn refresh_snapshot(&mut self, refresh_task_detail: bool, cx: &mut Context<Self>) {
        let Ok(snapshot) = load_download_snapshot(&self.engine) else {
            return;
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
        let admitted_progress_task = match &envelope.event {
            DownloadEvent::TaskChanged { task } => {
                let known_to_desktop = self
                    .tasks
                    .iter()
                    .any(|candidate| candidate.task.id() == task.id());
                should_open_admitted_progress_window(
                    known_to_desktop,
                    self.awaiting_delayed_progress.contains(&task.id()),
                    task.origin(),
                    task.status(),
                )
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let queue = application.read(cx).default_queue().cloned();
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

struct NewDownloadView {
    theme: Theme,
    application: Entity<DownloadApplication>,
    url: Entity<InputState>,
    directory: Entity<InputState>,
    file_name: Entity<InputState>,
    scheduled_at: Entity<InputState>,
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
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let url =
            cx.new(|cx| InputState::new(window, cx).placeholder("https://example.com/file.zip"));
        let directory = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value(default_directory.to_string_lossy().into_owned())
                .placeholder("保存目录")
        });
        let file_name =
            cx.new(|cx| InputState::new(window, cx).placeholder("可选；留空时使用服务器文件名"));
        let scheduled_at = cx.new(|cx| {
            InputState::new(window, cx).placeholder("可选；例如 2026-07-30 09:30（本地时间）")
        });
        let subscription = cx.subscribe_in(
            &application,
            window,
            |this, _application, event: &DownloadApplicationEvent, window, cx| {
                let DownloadApplicationEvent::DirectoryPicked(path) = event;
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
            scheduled_at,
            confirmed_request: None,
            probe: None,
            probing: false,
            submitting: false,
            probe_generation: 0,
            error: None,
            _subscription: subscription,
        }
    }

    fn request(&self, cx: &Context<Self>) -> Result<DownloadRequest, String> {
        let url = self.url.read(cx).value().trim().to_owned();
        let directory = self.directory.read(cx).value().trim().to_owned();
        let file_name = self.file_name.read(cx).value().trim().to_owned();
        let scheduled_at = self.scheduled_at.read(cx).value().trim().to_owned();
        if url.is_empty() || directory.is_empty() {
            return Err("下载地址和保存目录不能为空".into());
        }
        let scheduled_at_unix_ms = if scheduled_at.is_empty() {
            None
        } else {
            let local = NaiveDateTime::parse_from_str(&scheduled_at, "%Y-%m-%d %H:%M")
                .map_err(|_| "计划时间格式应为 YYYY-MM-DD HH:MM，例如 2026-07-30 09:30")?;
            let resolved = Local
                .from_local_datetime(&local)
                .single()
                .ok_or("计划时间在当前本地时区无效或存在歧义，请换一个时间")?;
            let unix_ms = resolved.timestamp_millis();
            if unix_ms <= Local::now().timestamp_millis() {
                return Err("计划时间必须晚于当前时间".into());
            }
            Some(unix_ms)
        };
        let request = DownloadRequest {
            source_url: url,
            save_directory: PathBuf::from(directory),
            file_name: (!file_name.is_empty()).then_some(file_name),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            scheduled_at_unix_ms,
        };
        request.validate().map_err(|error| error.to_string())?;
        Ok(request)
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
                    let plan = submit_presentation_plan(mode, task_id, delayed);
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
        let footer_actions = if confirmed {
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .child(
                    Button::new("download-refresh-probe")
                        .icon(Icon::new(MaterialIcon::ManageSearch))
                        .small()
                        .ghost()
                        .disabled(submitting)
                        .on_click(cx.listener(|this, _, window, cx| {
                            this.begin_probe(window, cx);
                        })),
                )
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
                                                                application.request_directory(
                                                                    current_directory,
                                                                    cx,
                                                                );
                                                            },
                                                        );
                                                    });
                                                }),
                                        ),
                                )
                                .child(download_input_field(theme, "文件名", &self.file_name))
                                .child(download_input_field(theme, "稍后下载", &self.scheduled_at)),
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
                    .h(px(44.0))
                    .px(px(16.0))
                    .border_t_1()
                    .border_color(theme.border_subtle)
                    .bg(theme.surface)
                    .flex()
                    .items_center()
                    .child(footer_actions)
                    .child(div().flex_1())
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
                    scheduled_at_unix_ms: None,
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
            submit_presentation_plan(DownloadSubmitMode::StartNow, task_id, false),
            DownloadSubmitPresentationPlan {
                close_source_window: true,
                deferred_task_window: Some(task_id),
            }
        );
    }

    #[test]
    fn queue_only_closes_the_add_window_without_opening_a_progress_window() {
        let task_id = DownloadTaskId::new();

        assert_eq!(
            submit_presentation_plan(DownloadSubmitMode::QueueOnly, task_id, false),
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
            submit_presentation_plan(DownloadSubmitMode::StartNow, task_id, true),
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
            scheduled_at_unix_ms: None,
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
