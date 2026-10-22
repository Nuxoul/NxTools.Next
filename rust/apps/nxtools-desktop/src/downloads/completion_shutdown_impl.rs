// Batch completion feedback and the post-download shutdown flow:
// terminal notifications, completion open/reveal feedback tracking,
// shutdown arming and evaluation, the countdown window and the actual
// system shutdown execution. Split from downloads.rs — same type,
// separate impl block.

use super::*;

impl DownloadApplication {
    pub(crate) fn publish_terminal_notifications(
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
            let detail = task
                .has_final_artifact
                .then(|| load_download_task_detail(&self.engine, task.task.id()).ok())
                .flatten();
            let final_path = detail
                .as_ref()
                .and_then(|detail| detail.task.final_path.as_ref());
            let display_name = final_path
                .and_then(|path| path.file_name())
                .and_then(|name| name.to_str())
                .or(task.task.request().file_name.as_deref())
                .unwrap_or("下载任务")
                .to_owned();
            self.completion_actions.record_terminal(CompletionTerminal {
                task_id: task.task.id(),
                status,
                directory: final_path
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

    pub(crate) fn flush_completion_feedback(&mut self, cx: &mut Context<Self>) {
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

    pub(crate) fn show_completion_batch_notification(
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

    pub(crate) fn handle_completion_feedback_result(
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

    pub(crate) fn completion_feedback_timed_out(&mut self, operation_id: uuid::Uuid) {
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

    pub(crate) fn finish_completion_feedback(&self, operation: PendingCompletionFeedback) {
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

    pub(crate) fn is_shutdown_armed(&self, task_id: DownloadTaskId) -> bool {
        self.completion_actions.is_shutdown_armed(task_id)
    }

    pub(crate) fn toggle_shutdown_armed(
        &mut self,
        task_id: DownloadTaskId,
        cx: &mut Context<Self>,
    ) {
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

    pub(crate) fn evaluate_shutdown(
        &mut self,
        tasks: &[DownloadEngineTask],
        cx: &mut Context<Self>,
    ) {
        let task_states = tasks
            .iter()
            .map(|task| (task.task.id(), task.task.status()))
            .collect::<Vec<_>>();
        let transition = self
            .completion_actions
            .evaluate_shutdown(task_states, Instant::now());
        self.apply_shutdown_transition(transition, cx);
    }

    pub(crate) fn apply_shutdown_transition(
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

    pub(crate) fn open_shutdown_countdown(
        &mut self,
        countdown: ShutdownCountdown,
        cx: &mut Context<Self>,
    ) {
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
                ..Default::default()
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

    pub(crate) fn tick_shutdown_countdown(
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

    pub(crate) fn cancel_shutdown_countdown(
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

    pub(crate) fn finish_shutdown_cancelled(
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

    pub(crate) fn can_native_close_shutdown_countdown(&self, correlation_id: uuid::Uuid) -> bool {
        self.pending_shutdown_command.is_none()
            && self
                .completion_actions
                .active_countdown()
                .is_none_or(|countdown| countdown.correlation_id == correlation_id)
    }

    pub(crate) fn finish_native_close_shutdown_countdown(&mut self, correlation_id: uuid::Uuid) {
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

    pub(crate) fn execute_shutdown(
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

    pub(crate) fn handle_shutdown_result(
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

    pub(crate) fn shutdown_command_timed_out(
        &mut self,
        command_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
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

    pub(crate) fn show_shutdown_failure(&self, message: &str, cx: &mut Context<Self>) {
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_error(
                "download-shutdown-failed",
                "自动关机失败",
                message.to_owned(),
                cx,
            );
        });
    }

    pub(crate) fn close_shutdown_window(
        &mut self,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
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
