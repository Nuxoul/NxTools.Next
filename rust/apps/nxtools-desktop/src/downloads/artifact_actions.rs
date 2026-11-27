// Artifact Shell requests and their terminal lifecycle, native drag dispatch,
// and delete confirmation stay on the same DownloadApplication entity.
use super::*;

const DOWNLOAD_ARTIFACT_ACTION_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy)]
pub(super) struct PendingDownloadArtifactAction {
    correlation_id: uuid::Uuid,
    task_id: DownloadTaskId,
    action: ShellOpenAction,
    window: Option<TaskWindowIdentity>,
    source_page: bool,
    started_at: Instant,
}

#[derive(Clone, Copy)]
enum ArtifactActionFailure {
    QueueRejected,
    Timeout,
    Disconnected,
    Native,
}

impl ArtifactActionFailure {
    fn presentation(
        self,
        action: ShellOpenAction,
        source_page: bool,
    ) -> (&'static str, &'static str) {
        if source_page {
            return (
                "无法打开浏览器",
                match self {
                    Self::QueueRejected => "浏览器操作未能排入队列，请重试。",
                    Self::Timeout => "Windows 未能确认浏览器已打开，请重试。",
                    Self::Disconnected => "Windows 连接已断开，请重新点击在浏览器中打开。",
                    Self::Native => "Windows 未能打开浏览器，请重试。",
                },
            );
        }
        let reveal = action == ShellOpenAction::SelectInExplorer;
        let title = if reveal {
            "无法打开所在文件夹"
        } else {
            "无法执行文件操作"
        };
        let message = match (self, reveal) {
            (Self::QueueRejected, _) => "Windows 文件操作未能排入队列，请重试。",
            (Self::Timeout, true) => "未能确认资源管理器已打开目标文件夹或定位文件，请重试。",
            (Self::Timeout, false) => "Windows 未能确认文件操作，请重试。",
            (Self::Disconnected, true) => "Windows 连接已断开，请重新点击打开所在文件夹。",
            (Self::Disconnected, false) => "Windows 连接已断开，请重试文件操作。",
            (Self::Native, true) => "资源管理器未能确认目标文件夹或文件，请重试。",
            (Self::Native, false) => "Windows 未能完成文件操作，请重试。",
        };
        (title, message)
    }
}

fn show_artifact_action_failure(
    notifications: &Entity<NotificationCenter>,
    task_id: DownloadTaskId,
    action: ShellOpenAction,
    source_page: bool,
    failure: ArtifactActionFailure,
    cx: &mut Context<DownloadApplication>,
) {
    let (title, message) = failure.presentation(action, source_page);
    notifications.update(cx, |notifications, cx| {
        notifications.show_error(
            format!("download-artifact-open-{}", task_id.as_uuid()),
            title,
            message,
            cx,
        );
    });
}

pub(super) fn resolve_download_artifact_target(
    final_path: Option<&Path>,
    save_directory: &Path,
    action: ShellOpenAction,
) -> Result<(PathBuf, bool), String> {
    if let Some(path) = final_path
        .filter(|path| path.is_file() || (action != ShellOpenAction::OpenWith && path.is_dir()))
    {
        return Ok((path.to_path_buf(), false));
    }
    if action == ShellOpenAction::SelectInExplorer && save_directory.is_dir() {
        return Ok((save_directory.to_path_buf(), true));
    }
    Err("任务还没有可用的最终文件路径。".into())
}

impl DownloadApplication {
    fn final_artifact_path(&self, task_id: DownloadTaskId) -> Result<PathBuf, String> {
        let detail = self
            .task_detail
            .as_ref()
            .filter(|detail| detail.task.task.id() == task_id)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| load_download_task_detail(&self.engine, task_id));
        detail?
            .task
            .final_path
            .clone()
            .ok_or_else(|| "任务还没有可用的最终文件路径。".into())
    }

    pub(crate) fn open_task_artifact(
        &mut self,
        task_id: DownloadTaskId,
        action: ShellOpenAction,
        cx: &mut Context<Self>,
    ) {
        self.open_task_artifact_with_window(task_id, action, None, cx);
    }

    pub(crate) fn open_task_artifact_from_task_window(
        &mut self,
        task_id: DownloadTaskId,
        action: ShellOpenAction,
        window: TaskWindowIdentity,
        cx: &mut Context<Self>,
    ) {
        self.open_task_artifact_with_window(task_id, action, Some(window), cx);
    }

    pub(crate) fn open_task_source_in_browser(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_artifact_actions
            .values()
            .any(|pending| pending.task_id == task_id)
        {
            return;
        }
        let Some(task) = self.task(task_id) else {
            return;
        };
        let target = task.task.request().source_url.clone();
        let pending = PendingDownloadArtifactAction {
            correlation_id: uuid::Uuid::new_v4(),
            task_id,
            action: ShellOpenAction::Open,
            window: None,
            source_page: true,
            started_at: Instant::now(),
        };
        self.dispatch_task_shell_action(pending, target, cx);
    }

    fn open_task_artifact_with_window(
        &mut self,
        task_id: DownloadTaskId,
        action: ShellOpenAction,
        window: Option<TaskWindowIdentity>,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_artifact_actions
            .values()
            .any(|pending| pending.task_id == task_id)
        {
            info!(
                event = "download.artifact.shell_action_coalesced",
                task_id = %task_id.as_uuid(),
                ?action,
                "duplicate download artifact action ignored while a result is pending"
            );
            return;
        }
        let correlation_id = uuid::Uuid::new_v4();
        let started_at = Instant::now();

        let detail = self
            .task_detail
            .as_ref()
            .filter(|detail| detail.task.task.id() == task_id)
            .cloned()
            .map(Ok)
            .unwrap_or_else(|| load_download_task_detail(&self.engine, task_id));
        let (target, used_save_directory_fallback) = match detail.and_then(|detail| {
            resolve_download_artifact_target(
                detail
                    .task
                    .final_path
                    .as_deref()
                    .filter(|path| path.exists())
                    .or_else(|| {
                        (action == ShellOpenAction::SelectInExplorer
                            && detail.task.task.status() != DownloadStatus::Completed)
                            .then_some(detail.task.temporary_path.as_deref())
                            .flatten()
                    }),
                &detail.task.task.request().save_directory,
                action,
            )
        }) {
            Ok(target) => target,
            Err(error) => {
                warn!(
                    event = "download.artifact.shell_action_failed",
                    %correlation_id,
                    task_id = %task_id.as_uuid(),
                    ?action,
                    failure_kind = "target_unavailable",
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "download artifact target could not be resolved"
                );
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("download-artifact-missing-{}", task_id.as_uuid()),
                        "无法打开下载文件",
                        error,
                        cx,
                    );
                });
                return;
            }
        };
        let path = target.to_string_lossy().into_owned();
        info!(
            event = "download.artifact.reveal_target_resolved",
            %correlation_id,
            task_id = %task_id.as_uuid(),
            used_save_directory_fallback,
            target_is_directory = target.is_dir(),
            "download reveal target resolved"
        );

        let pending = PendingDownloadArtifactAction {
            correlation_id,
            task_id,
            action,
            window,
            source_page: false,
            started_at,
        };
        self.dispatch_task_shell_action(pending, path, cx);
    }

    fn dispatch_task_shell_action(
        &mut self,
        pending: PendingDownloadArtifactAction,
        path: String,
        cx: &mut Context<Self>,
    ) {
        let PendingDownloadArtifactAction {
            correlation_id,
            task_id,
            action,
            window,
            source_page,
            started_at,
        } = pending;
        let command_id = correlation_id;
        self.pending_artifact_actions.insert(command_id, pending);
        match self.platform.send_command_with_id(
            PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                target: path,
                action,
            }),
            command_id,
        ) {
            Ok(command_id) => {
                info!(
                    event = "download.artifact.shell_action_accepted",
                    %correlation_id,
                    %command_id,
                    task_id = %task_id.as_uuid(),
                    ?action,
                    from_task_window = window.is_some(),
                    source_page,
                    "download artifact Shell action accepted"
                );
                if action != ShellOpenAction::OpenWith {
                    cx.spawn(async move |this, cx| {
                        cx.background_executor()
                            .timer(DOWNLOAD_ARTIFACT_ACTION_TIMEOUT)
                            .await;
                        let _ = this.update(cx, |application, cx| {
                            application.artifact_action_timed_out(command_id, cx);
                        });
                    })
                    .detach();
                }
            }
            Err(error) => {
                self.pending_artifact_actions.remove(&command_id);
                warn!(
                    event = "download.artifact.shell_action_failed",
                    %correlation_id,
                    task_id = %task_id.as_uuid(),
                    ?action,
                    failure_kind = match &error {
                        PlatformBridgeError::Disconnected => "platform_disconnected",
                        PlatformBridgeError::Busy => "queue_busy",
                        PlatformBridgeError::DuplicateCommandId => "duplicate_command_id",
                    },
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "download artifact Shell action was not accepted"
                );
                show_artifact_action_failure(
                    &self.notifications,
                    task_id,
                    action,
                    source_page,
                    ArtifactActionFailure::QueueRejected,
                    cx,
                );
            }
        }
    }

    pub(super) fn artifact_action_timed_out(
        &mut self,
        command_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending_artifact_actions.remove(&command_id) else {
            return;
        };
        warn!(
            event = "download.artifact.shell_action_failed",
            correlation_id = %pending.correlation_id,
            %command_id,
            task_id = %pending.task_id.as_uuid(),
            ?pending.action,
            failure_kind = "timeout",
            elapsed_ms = pending.started_at.elapsed().as_millis(),
            "download artifact Shell action timed out"
        );
        show_artifact_action_failure(
            &self.notifications,
            pending.task_id,
            pending.action,
            pending.source_page,
            ArtifactActionFailure::Timeout,
            cx,
        );
    }

    pub(super) fn disconnect_artifact_actions(&mut self, cx: &mut Context<Self>) {
        for (command_id, pending) in self.pending_artifact_actions.drain() {
            warn!(
                event = "download.artifact.shell_action_failed",
                correlation_id = %pending.correlation_id,
                %command_id,
                task_id = %pending.task_id.as_uuid(),
                ?pending.action,
                failure_kind = "platform_disconnected",
                elapsed_ms = pending.started_at.elapsed().as_millis(),
                "download artifact Shell action was invalidated by PlatformHost disconnect"
            );
            show_artifact_action_failure(
                &self.notifications,
                pending.task_id,
                pending.action,
                pending.source_page,
                ArtifactActionFailure::Disconnected,
                cx,
            );
        }
    }

    pub(super) fn handle_artifact_action_result(
        &mut self,
        correlation_id: uuid::Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pending) = self.pending_artifact_actions.remove(&correlation_id) else {
            return false;
        };
        if pending.action == ShellOpenAction::OpenWith
            && !result.success
            && result.error.is_none()
            && result.data.as_deref() == Some("cancelled")
        {
            info!(event = "download.artifact.shell_action_cancelled", correlation_id = %pending.correlation_id,
                command_id = %correlation_id, task_id = %pending.task_id.as_uuid(), elapsed_ms = pending.started_at.elapsed().as_millis(),
                "Open With chooser cancelled; task window retained");
            return true;
        }
        if result.success {
            if let Some(window) = pending.window {
                if self.close_task_window_if_identity(pending.task_id, window, cx) {
                    info!(
                        event = "download.artifact.task_window_closed",
                        correlation_id = %pending.correlation_id,
                        command_id = %correlation_id,
                        task_id = %pending.task_id.as_uuid(),
                        window_handle = window.native_window_handle,
                        window_generation = %window.generation,
                        "download artifact action completed and task window closed"
                    );
                } else {
                    info!(
                        event = "download.artifact.task_window_close_suppressed",
                        correlation_id = %pending.correlation_id,
                        command_id = %correlation_id,
                        task_id = %pending.task_id.as_uuid(),
                        window_handle = window.native_window_handle,
                        window_generation = %window.generation,
                        reason = "window_generation_changed",
                        "download artifact action completed but its task window was no longer current"
                    );
                }
            }
            info!(
                event = "download.artifact.shell_action_completed",
                correlation_id = %pending.correlation_id,
                command_id = %correlation_id,
                task_id = %pending.task_id.as_uuid(),
                ?pending.action,
                elapsed_ms = pending.started_at.elapsed().as_millis(),
                "download artifact Shell action completed"
            );
        } else {
            warn!(
                event = "download.artifact.shell_action_failed",
                correlation_id = %pending.correlation_id,
                command_id = %correlation_id,
                task_id = %pending.task_id.as_uuid(),
                ?pending.action,
                native_error_code = ?result.native_error_code,
                elapsed_ms = pending.started_at.elapsed().as_millis(),
                "download artifact Shell action failed"
            );
            show_artifact_action_failure(
                &self.notifications,
                pending.task_id,
                pending.action,
                pending.source_page,
                ArtifactActionFailure::Native,
                cx,
            );
        }
        true
    }

    pub(super) fn begin_task_file_drag(
        &mut self,
        task_id: DownloadTaskId,
        source_window_handle: i64,
        cx: &mut Context<Self>,
    ) {
        let Ok(path) = self
            .final_artifact_path(task_id)
            .map(|path| path.to_string_lossy().into_owned())
        else {
            return;
        };
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
                    "download file drag requested"
                );
            }
            Err(error) => {
                warn!(
                    task_id = %task_id.as_uuid(),
                    source_window_handle,
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

    pub(crate) fn request_delete_confirmation(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) {
        if self.batch_busy() {
            return;
        }
        // The user-configurable contract for "删除文件": when the file delete
        // is disabled, the menu item degenerates to a list-only removal.
        if !self.config.download.task_removal_deletes_file {
            let _ = self.remove(task_id, false, cx);
            return;
        }
        let path = self
            .task(task_id)
            .and_then(|task| task.final_path.as_ref())
            .map_or_else(|| "最终文件".into(), |path| path.display().to_string());
        self.open_delete_confirmation(task_id, path, None, cx);
    }

    pub(super) fn open_delete_confirmation(
        &mut self,
        task_id: DownloadTaskId,
        path: String,
        batch: Option<BatchConfirmation>,
        cx: &mut Context<Self>,
    ) {
        if let Some(window) = self.delete_confirmation_windows.get(&task_id).cloned() {
            if batch.is_none() && cx.windows().contains(&window) {
                self.prepare_delete_material(task_id, window, false, cx);
                return;
            }
            let _ = window.update(cx, |_, window, _| window.remove_window());
            self.delete_confirmation_windows.remove(&task_id);
        }
        let application = cx.entity();
        let is_batch = batch.is_some();
        let window_size = size(px(380.0), px(178.0));
        let window_bounds = WindowBounds::centered(window_size, cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                titlebar: None,
                focus: false,
                show: false,
                kind: WindowKind::Normal,
                is_movable: true,
                is_resizable: false,
                is_minimizable: false,
                display_id: None,
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.Download.DeleteConfirmation".into()),
                window_min_size: Some(window_size),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
                ..Default::default()
            },
            move |window, cx| {
                let close_application = application.clone();
                let closing_window = window.window_handle();
                window.on_window_should_close(cx, move |_, cx| {
                    let application = close_application.clone();
                    cx.defer(move |cx| {
                        application.update(cx, |application, cx| {
                            application
                                .handle_native_delete_confirmation_close(task_id, closing_window);
                            cx.notify();
                        });
                    });
                    true
                });
                let view = cx.new(|_| DeleteDownloadView {
                    theme: Theme::zed_dark(),
                    application,
                    task_id,
                    path,
                    batch,
                });
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match opened {
            Ok(window) => {
                self.delete_confirmation_windows
                    .insert(task_id, window.into());
                self.prepare_delete_material(task_id, window.into(), true, cx);
                if is_batch {
                    self.pending_batch_confirmation = Some(task_id);
                    cx.notify();
                }
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

    pub(super) fn close_delete_confirmation(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) {
        if let Some(window) = self.delete_confirmation_windows.remove(&task_id) {
            if self.pending_batch_confirmation == Some(task_id) {
                self.pending_batch_confirmation = None;
                cx.notify();
            }
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
            if self.pending_batch_confirmation == Some(task_id) {
                self.pending_batch_confirmation = None;
            }
        }
    }

    pub(super) fn confirm_delete_artifact(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) {
        if self.batch_busy() {
            return;
        }
        if self.remove(task_id, true, cx).is_ok() {
            self.close_delete_confirmation(task_id, cx);
        }
    }
}

#[cfg(test)]
mod feedback_tests {
    use super::*;
    #[test]
    fn round2_list_artifact_failure_copy_never_claims_a_completion_window() {
        for failure in [
            ArtifactActionFailure::QueueRejected,
            ArtifactActionFailure::Timeout,
            ArtifactActionFailure::Disconnected,
            ArtifactActionFailure::Native,
        ] {
            let (_, message) = failure.presentation(ShellOpenAction::SelectInExplorer, false);
            assert!(
                !message.contains("完成窗口") && !message.contains("前台"),
                "list error must describe target operation, not unrelated window/focus: {message}"
            );
        }
    }
}
