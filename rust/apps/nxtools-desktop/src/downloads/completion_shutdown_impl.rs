// Original completion feedback, persistent intention commands and the single action countdown.

use super::*;

impl DownloadApplication {
    pub(crate) fn publish_terminal_notifications(
        &mut self,
        tasks: &[DownloadEngineTask],
        cx: &mut Context<Self>,
    ) {
        let completion_snapshot = self.engine.completion_snapshot().ok();
        if let Some(snapshot) = &completion_snapshot {
            self.completion_actions
                .register_completion_members(snapshot);
        }
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
            let attempt = completion_snapshot
                .as_ref()
                .and_then(|snapshot| {
                    snapshot
                        .tasks
                        .iter()
                        .find(|member| member.task_id == task.task.id())
                        .map(|member| member.attempt)
                })
                .unwrap_or(0);
            self.completion_actions.record_terminal_attempt(
                CompletionTerminal {
                    task_id: task.task.id(),
                    status,
                    directory: final_path
                        .and_then(|path| path.parent())
                        .map(Path::to_path_buf),
                    display_name,
                    error_message: task.error.as_ref().map(|error| error.message.clone()),
                },
                attempt,
            );
            queued_feedback = true;
        }
        if let Some(snapshot) = &completion_snapshot {
            self.completion_actions.settle_completion_members(snapshot);
        }
        self.evaluate_shutdown_snapshot(completion_snapshot.as_ref(), cx);
        if (queued_feedback
            || self.completion_actions.has_pending_feedback()
            || self.completion_actions.has_ready_group_feedback())
            && !self.completion_feedback_scheduled
        {
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
    }

    pub(crate) fn flush_completion_feedback(&mut self, cx: &mut Context<Self>) {
        self.completion_feedback_scheduled = false;
        let policy = self.config.download.open_directory_on_completion;
        let mut batches = self
            .completion_actions
            .take_ready_group_feedback(policy)
            .into_iter()
            .map(|(id, batch)| (batch, Some(id)))
            .collect::<Vec<_>>();
        if let Some(batch) = self
            .completion_actions
            .take_feedback_batch_for_directory_policy(policy)
        {
            batches.push((batch, None));
        }
        for (batch, group_id) in batches {
            self.dispatch_completion_feedback_batch(batch, group_id, cx);
        }
    }

    fn dispatch_completion_feedback_batch(
        &mut self,
        batch: CompletionFeedbackBatch,
        group_id: Option<uuid::Uuid>,
        cx: &mut Context<Self>,
    ) {
        let correlation_id = group_id.unwrap_or_else(uuid::Uuid::new_v4);
        let started_at = Instant::now();
        let sound_enabled =
            self.config.download.completion_sound_enabled && batch.completed_count > 0;
        let directory_enabled = !batch.directories.is_empty();
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
        if batch.completed_count + batch.failed_count > 0 {
            self.show_completion_batch_notification(&batch, correlation_id, cx);
        }

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
        let (level, title, message) = if batch.completed_count > 0 && batch.failed_count == 0 {
            let message = if batch.completed_count == 1 {
                batch
                    .single_display_name
                    .clone()
                    .unwrap_or_else(|| "下载文件已完成".into())
            } else {
                format!("{} 个下载任务已完成", batch.completed_count)
            };
            (
                crate::notifications::NotificationLevel::Success,
                "下载完成",
                message,
            )
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
            (
                crate::notifications::NotificationLevel::Error,
                "下载批次已结束",
                message,
            )
        };
        let notice = crate::notifications::AppNotification::new(id, level, title, message);
        let notifications = self.notifications.clone();
        cx.defer(move |cx| notifications.update(cx, |center, cx| center.push(notice, cx)));
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
        self.engine
            .task(task_id)
            .ok()
            .flatten()
            .is_some_and(|task| {
                task.task.completion_intention().action
                    == Some(nxtools_download::DownloadCompletionAction::Shutdown)
            })
    }

    pub(crate) fn evaluate_shutdown(
        &mut self,
        _tasks: &[DownloadEngineTask],
        cx: &mut Context<Self>,
    ) {
        let snapshot = self.engine.completion_snapshot();
        self.evaluate_shutdown_snapshot(snapshot.as_ref().ok(), cx);
    }

    fn evaluate_shutdown_snapshot(
        &mut self,
        snapshot: Option<&nxtools_download::DownloadCompletionSnapshot>,
        cx: &mut Context<Self>,
    ) {
        match snapshot {
            Some(snapshot) => {
                let transition = self.completion_actions.evaluate_completion_in_slot(
                    snapshot,
                    Instant::now(),
                    self.pending_shutdown_command.is_none(),
                );
                self.apply_shutdown_transition(transition, cx);
            }
            None => {
                if let Some(countdown) = self.completion_actions.clear_countdown() {
                    self.finish_shutdown_cancelled(countdown, "snapshot_unavailable", cx);
                }
            }
        }
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
        if self.completion_actions.active_countdown() != Some(countdown) {
            return;
        }
        if self.shutdown_window.is_some() {
            return;
        }
        info!(
            event = "download.shutdown_countdown.started",
            correlation_id = %countdown.correlation_id,
            action = ?countdown.action,
            generation = countdown.generation,
            work_revision = countdown.work_revision,
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
                focus: false,
                show: false,
                kind: WindowKind::Normal,
                is_movable: true,
                is_resizable: false,
                is_minimizable: false,
                display_id: None,
                window_background: WindowBackgroundAppearance::Transparent,
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
                    let mut view = DownloadShutdownCountdownView::new(
                        application,
                        countdown.correlation_id,
                        countdown.duration.as_secs(),
                    );
                    view.action = countdown.action;
                    view
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
            material_ready: false,
        });
        let native_handle = *native_handle.borrow();
        let material_window: AnyWindowHandle = window.into();
        self.after_download_material(
            material_window,
            nxtools_protocol::window_material::MaterialSurface::Backdrop,
            DOWNLOAD_MATERIAL_TIMEOUT,
            move |app, result, cx| {
                if app.completion_actions.active_countdown() != Some(countdown)
                    || !cx.windows().contains(&material_window)
                    || app.shutdown_window.as_ref().is_none_or(|entry| {
                        entry.window != material_window
                            || entry.correlation_id != countdown.correlation_id
                    })
                {
                    return;
                }
                if result.is_err() || Instant::now() >= countdown.deadline {
                    app.cancel_shutdown_countdown(
                        countdown.correlation_id,
                        "material_not_ready",
                        cx,
                    );
                    app.download_material_failed(
                        "shutdown",
                        result.err().unwrap_or("countdown_elapsed"),
                        cx,
                    );
                    return;
                }
                app.shutdown_window
                    .as_mut()
                    .expect("current countdown")
                    .material_ready = true;
                let _ = material_window.update(cx, |_, window, _| window.activate_window());
                if native_handle != 0 {
                    let _ = app.platform.send_command_correlated(
                        PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                            window_handle: native_handle,
                            enabled: true,
                        }),
                        countdown.correlation_id,
                    );
                }
            },
            cx,
        );
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
        if self
            .shutdown_window
            .as_ref()
            .is_some_and(|entry| !entry.material_ready)
        {
            if now >= countdown.deadline {
                self.cancel_shutdown_countdown(correlation_id, "material_not_ready", cx);
                return false;
            }
            return true;
        }
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
        let Some(countdown) = self.completion_actions.cancel_countdown(correlation_id) else {
            return;
        };
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
        if self
            .completion_actions
            .active_countdown()
            .is_none_or(|countdown| countdown.correlation_id != correlation_id)
        {
            return;
        }
        self.evaluate_shutdown(&[], cx);
        if self
            .completion_actions
            .active_countdown()
            .is_none_or(|countdown| countdown.correlation_id != correlation_id)
        {
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
            action = ?countdown.action,
            generation = countdown.generation,
            work_revision = countdown.work_revision,
            reason,
            "download shutdown command requested"
        );
        let command = match countdown.action {
            nxtools_download::DownloadCompletionAction::Shutdown => {
                PlatformCommand::ShutdownSystem(ShutdownSystemCommand { delay_seconds: 1 })
            }
            nxtools_download::DownloadCompletionAction::Sleep => {
                PlatformCommand::SuspendSystem(SuspendSystemCommand { hibernate: false })
            }
            nxtools_download::DownloadCompletionAction::Hibernate => {
                PlatformCommand::SuspendSystem(SuspendSystemCommand { hibernate: true })
            }
            nxtools_download::DownloadCompletionAction::ExitApplication => {
                self.close_shutdown_window(correlation_id, cx);
                info!(event="download.completion_exit.close_requested",%correlation_id,generation=countdown.generation,elapsed_ms=countdown.started_at.elapsed().as_millis(),"normal application close requested");
                if let Some(correlation) =
                    cx.try_global::<crate::completion_actions::CompletionExitCorrelation>()
                    && let Ok(mut current) = correlation.0.lock()
                {
                    *current = Some(correlation_id);
                }
                cx.quit();
                return;
            }
        };
        match self
            .platform
            .send_command_correlated(command, correlation_id)
        {
            Ok(command_id) => {
                self.pending_shutdown_command = Some(PendingShutdownCommand {
                    command_id,
                    countdown,
                });
                if countdown.action == nxtools_download::DownloadCompletionAction::Shutdown {
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
                self.show_shutdown_failure("无法提交完成动作请求", cx);
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
                action = ?pending.countdown.action,
                "Windows accepted the completion action request"
            );
        } else {
            warn!(
                event = "download.shutdown_countdown.failed",
                correlation_id = %pending.countdown.correlation_id,
                failure_kind = "native_rejected",
                native_error_code = result.native_error_code,
                action = ?pending.countdown.action,
                elapsed_ms = pending.countdown.started_at.elapsed().as_millis(),
                "Windows rejected the completion action request"
            );
            let detail = result
                .error
                .as_deref()
                .unwrap_or("Windows 拒绝了完成动作请求");
            let message = result.native_error_code.map_or_else(
                || detail.to_owned(),
                |code| format!("{detail}（Windows 错误 {code}）"),
            );
            self.show_shutdown_failure(&message, cx);
        }
        self.close_shutdown_window(pending.countdown.correlation_id, cx);
        self.evaluate_shutdown(&[], cx);
    }

    pub(crate) fn shutdown_command_timed_out(
        &mut self,
        command_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_shutdown_command
            .as_ref()
            .is_none_or(|pending| {
                pending.command_id != command_id
                    || pending.countdown.action
                        != nxtools_download::DownloadCompletionAction::Shutdown
            })
        {
            return;
        }
        let Some(pending) = self.pending_shutdown_command.take() else {
            return;
        };
        warn!(
            event = "download.shutdown_countdown.failed",
            correlation_id = %pending.countdown.correlation_id,
            failure_kind = "platform_timeout",
            native_error_code = Option::<i32>::None,
            elapsed_ms = pending.countdown.started_at.elapsed().as_millis(),
            "download shutdown command timed out"
        );
        self.show_shutdown_failure("Windows 完成动作请求超时", cx);
        self.close_shutdown_window(pending.countdown.correlation_id, cx);
        self.evaluate_shutdown(&[], cx);
    }

    pub(crate) fn disconnect_completion_action(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_shutdown_command.take() else {
            return;
        };
        warn!(
            event = "download.shutdown_countdown.failed",
            correlation_id = %pending.countdown.correlation_id,
            command_id = %pending.command_id,
            generation = pending.countdown.generation,
            action = ?pending.countdown.action,
            failure_kind = "platform_disconnected",
            native_error_code = Option::<i32>::None,
            elapsed_ms = pending.countdown.started_at.elapsed().as_millis(),
            "completion action outcome is unconfirmed after PlatformHost disconnect"
        );
        self.close_shutdown_window(pending.countdown.correlation_id, cx);
        let notifications = self.notifications.clone();
        cx.defer(move |cx| {
            notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    "download-shutdown-failed",
                    "完成动作结果未确认",
                    "Windows 连接已断开，未确认完成动作结果",
                    cx,
                );
            });
        });
    }

    pub(crate) fn show_shutdown_failure(&self, message: &str, cx: &mut Context<Self>) {
        self.notifications.update(cx, |notifications, cx| {
            notifications.show_error(
                "download-shutdown-failed",
                "完成动作失败",
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
