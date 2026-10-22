// Task-window lifecycle and placement for DownloadApplication: the keyed
// second-level window per task (open, transform between phases, native-close
// sync, activation queueing, replacement) plus the native position stash
// handoff. Split from downloads.rs — same type, separate impl block.

use super::*;

impl DownloadApplication {
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
    pub(crate) fn apply_task_window_directive(
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
    pub(crate) fn clear_task_window_registration(
        &mut self,
        task_id: DownloadTaskId,
    ) -> Option<DownloadTaskWindow> {
        clear_pending_task_window_work(
            task_id,
            &mut self.pending_task_window_activations,
            &mut self.pending_task_window_replacements,
        );
        self.task_window_lifecycle.window_closed(task_id);
        let entry = self.task_windows.remove(&task_id);
        // Preserve the real closing position so a later reopen for the same
        // task inherits it. GPUI's cached origin does not follow user drags,
        // so the geometry is captured natively before the window is destroyed.
        if let Some(entry) = entry.as_ref() {
            self.stash_task_window_anchor(task_id, entry.native_window_handle);
        }
        entry
    }
    /// Captures a window's real native rectangle under the task's correlation
    /// key so the task window opened next for this task can inherit that
    /// position. Cheap, bounded, and ordered through the shared command pipe.
    pub(crate) fn stash_task_window_anchor(
        &self,
        task_id: DownloadTaskId,
        native_window_handle: i64,
    ) {
        if native_window_handle == 0 {
            return;
        }
        let _ = self
            .platform
            .send_command(PlatformCommand::StashWindowBounds(
                StashWindowBoundsCommand {
                    window_handle: native_window_handle,
                    key: task_id.as_uuid().to_string(),
                },
            ));
    }
    /// Releases the keyed task-window registration after a native (non-app-driven)
    /// close. The window identity must still match so a replacement window opened
    /// in between is never deregistered by a stale close notification.
    pub(crate) fn handle_task_window_native_close(
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
    pub(crate) fn close_task_window_if_identity(
        &mut self,
        task_id: DownloadTaskId,
        expected: TaskWindowIdentity,
        cx: &mut Context<Self>,
    ) -> bool {
        let matches = self.task_windows.get(&task_id).is_some_and(|entry| {
            entry.native_window_handle == expected.native_window_handle
                && entry.generation == expected.generation
        });
        if !matches {
            return false;
        }
        self.close_task_window(task_id, cx);
        true
    }
    pub(crate) fn open_task_window(
        &mut self,
        task_id: DownloadTaskId,
        phase: DownloadWindowPhase,
        activation: Option<TaskWindowActivation>,
        cx: &mut Context<Self>,
    ) {
        let application = cx.entity();
        let snapshot = self.task(task_id).cloned().map(|snapshot| {
            if snapshot.has_final_artifact {
                load_download_task_detail(&self.engine, task_id)
                    .map(|detail| detail.task)
                    .unwrap_or(snapshot)
            } else {
                snapshot
            }
        });
        let progress = self.progress(task_id).cloned();
        let shutdown_armed = self.is_shutdown_armed(task_id);
        let effective_global_speed_limit = self.engine.effective_global_speed_limit();
        let window_size = download_task_window_size(phase);
        let window_bounds = WindowBounds::centered(window_size, cx);
        let window_generation = uuid::Uuid::new_v4();
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_native_slot = Rc::clone(&native_slot);
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let scale_slot = Rc::new(RefCell::new(1.0_f32));
        let opened_scale_slot = Rc::clone(&scale_slot);
        let platform_for_view = Rc::clone(&self.platform);
        #[cfg(test)]
        let native_window_handle_override = self.task_window_native_handle_override;
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(window_bounds),
                titlebar: None,
                focus: false,
                show: false,
                kind: WindowKind::Normal,
                is_movable: true,
                is_resizable: false,
                is_minimizable: true,
                display_id: None,
                window_background: WindowBackgroundAppearance::Opaque,
                app_id: Some(format!("NxTools.Download.{}", task_id.as_uuid())),
                window_min_size: Some(task_window_minimum_size()),
                window_decorations: Some(WindowDecorations::Client),
                tabbing_identifier: None,
                ..Default::default()
            },
            move |window, cx| {
                #[cfg(test)]
                let resolved_native_handle = native_window_handle_override
                    .unwrap_or_else(|| native_window_handle(window).unwrap_or_default());
                #[cfg(not(test))]
                let resolved_native_handle = native_window_handle(window).unwrap_or_default();
                *opened_native_slot.borrow_mut() = resolved_native_handle;
                *opened_scale_slot.borrow_mut() = window.scale_factor();
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
                    let mut view = DownloadTaskWindowView::new(
                        application,
                        platform_for_view,
                        task_id,
                        TaskWindowIdentity {
                            native_window_handle: resolved_native_handle,
                            generation: window_generation,
                        },
                        snapshot,
                        progress,
                        shutdown_armed,
                    );
                    view.effective_global_speed_limit = effective_global_speed_limit;
                    view
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
                // Inherit the stashed predecessor position (the replaced dialog
                // or the last closed window for this task) in the same command
                // batch; a stash miss keeps the centered creation position.
                let native_handle = *native_slot.borrow();
                if native_handle != 0 {
                    let scale = *scale_slot.borrow();
                    let _ = self
                        .platform
                        .send_command(PlatformCommand::MoveWindowCenteredOverKey(
                            MoveWindowCenteredOverKeyCommand {
                                window_handle: native_handle,
                                key: task_id.as_uuid().to_string(),
                                width: (window_size.width.as_f32() * scale).round() as i32,
                                height: (window_size.height.as_f32() * scale).round() as i32,
                            },
                        ));
                }
                let _ = self
                    .platform
                    .send_command(PlatformCommand::SetWindowVisibility(
                        SetWindowVisibilityCommand {
                            window_handle: *native_slot.borrow(),
                            visible: true,
                        },
                    ));
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
    pub(crate) fn update_task_window(
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
        let effective_global_speed_limit = self.engine.effective_global_speed_limit();
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
        let reset_details_expanded = matches!(resize_for, Some(DownloadWindowPhase::Progress));
        let mut resize_size = None;
        view.update(cx, |view, cx| {
            let previous_size = view.snapshot.as_ref().map(|task| {
                download_task_window_size_for_details(
                    download_window_phase(task.task.status()),
                    view.details_expanded,
                )
            });
            view.replace_snapshot(snapshot, progress, shutdown_armed, cx);
            view.effective_global_speed_limit = effective_global_speed_limit;
            if reset_details_expanded {
                view.reset_details_expanded();
            }
            if let Some(phase) = resize_for {
                let next_size = download_task_window_size_for_details(phase, view.details_expanded);
                if previous_size != Some(next_size) {
                    resize_size = Some(next_size);
                }
            }
        });
        let updated = window_handle
            .update(cx, move |_, window, _| {
                if let Some(new_size) = resize_size {
                    let scale = window.scale_factor();
                    if native_window_handle != 0 {
                        // GPUI's cached window origin never follows a user drag, so the
                        // centered geometry is computed from the native rectangle in C#.
                        let _ = platform.send_command(PlatformCommand::SetWindowBoundsCentered(
                            SetWindowBoundsCenteredCommand {
                                window_handle: native_window_handle,
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
    pub(crate) fn queue_task_window_activation(
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
    pub(crate) fn task_window_activation_timed_out(
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
    pub(crate) fn schedule_task_window_replacement(
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
    pub(crate) fn replace_task_window(
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
}
