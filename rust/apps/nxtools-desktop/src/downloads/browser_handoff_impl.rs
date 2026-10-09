// Browser handoff and the new-download entry surface: blob import and
// discard, extension export requests, the browser download confirmation
// dialog, the new-download window lifecycle and the schedule window.
// Split from downloads.rs — same type, separate impl block.

use super::*;

const FIXED_CONFIRMATION_IPC_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) struct PendingNewDownloadHandoff {
    pub(super) command_id: uuid::Uuid,
    started_at: Instant,
    source: AnyWindowHandle,
    source_view: gpui::WeakEntity<NewDownloadView>,
    generation: u64,
    target: AnyWindowHandle,
    target_view: Entity<NewDownloadView>,
    source_native: i64,
    target_native: i64,
}

impl DownloadApplication {
    fn fixed_source_is_current(
        &self,
        source: AnyWindowHandle,
        view: &gpui::WeakEntity<NewDownloadView>,
        generation: u64,
        cx: &App,
    ) -> bool {
        self.new_download_window == Some(source)
            && cx.windows().contains(&source)
            && view.upgrade().is_some_and(|view| {
                let view = view.read(cx);
                view.probe_generation == generation
                    && view.handoff_staged
                    && view.probe.is_some()
                    && view.confirmed_request.as_ref().is_some_and(|request| {
                        request.source_url == view.url.read(cx).value().trim()
                    })
            })
    }

    pub(crate) fn create_fixed_confirmation(
        &mut self,
        source: AnyWindowHandle,
        source_view: gpui::WeakEntity<NewDownloadView>,
        generation: u64,
        operation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        self.cleanup_new_download_handoff(cx);
        if self.pending_new_download_handoff.is_some()
            || !self.fixed_source_is_current(source, &source_view, generation, cx)
        {
            return;
        }
        let Some(view) = source_view.upgrade() else {
            return;
        };
        let window_size = new_download_window_size_for_probe(
            view.read(cx)
                .probe
                .as_ref()
                .expect("validated successful probe"),
        );
        let draft = view.read(cx).fixed_confirmation_draft(cx);
        let anchor = source.update(cx, |_, window, cx| {
            (
                native_window_handle(window).unwrap_or_default(),
                window.scale_factor(),
                window.display(cx).map(|display| display.id()),
            )
        });
        let Ok((source_native, scale, display_id)) = anchor else {
            return;
        };
        let view_slot = Rc::new(RefCell::new(None));
        let opened_view_slot = Rc::clone(&view_slot);
        let native_slot = Rc::new(RefCell::new(0_i64));
        let opened_native_slot = Rc::clone(&native_slot);
        let mut options = new_download_window_options(WindowBounds::centered(window_size, cx));
        options.show = false;
        options.focus = false;
        options.display_id = display_id;
        let opened = cx.open_window(options, move |window, cx| {
            *opened_native_slot.borrow_mut() = native_window_handle(window).unwrap_or_default();
            let view = cx.new(|cx| draft.hydrate(window, cx));
            *opened_view_slot.borrow_mut() = Some(view.clone());
            cx.new(|cx| Root::new(view, window, cx))
        });
        let target: AnyWindowHandle = match opened {
            Ok(window) => window.into(),
            Err(_) => {
                let _ = source_view.update(cx, |view, cx| {
                    view.handoff_pending = false;
                    view.error = Some("确认窗口未能打开，可重试打开确认；无需重新探测".into());
                    cx.notify();
                });
                warn!(event = "download.fixed_confirmation.failed", correlation_id = %operation_id, reason = "window_create", "fixed confirmation could not be created");
                return;
            }
        };
        let target_view = view_slot
            .borrow_mut()
            .take()
            .expect("created confirmation view");
        let target_native = *native_slot.borrow();
        #[cfg(test)]
        let (source_native, target_native) = self
            .new_download_native_handles_override
            .unwrap_or((source_native, target_native));
        if !self.fixed_source_is_current(source, &source_view, generation, cx) {
            let _ = target.update(cx, |_, window, _| window.remove_window());
            return;
        }
        // Use the operation ID as the command ID: send_command_with_id has no separate correlation field.
        self.pending_new_download_handoff = Some(PendingNewDownloadHandoff {
            command_id: operation_id,
            started_at: Instant::now(),
            source,
            source_view,
            generation,
            target,
            target_view,
            source_native,
            target_native,
        });
        info!(event = "download.fixed_confirmation.started", correlation_id = %operation_id, source_native, target_native, generation, "fixed confirmation handoff started");
        if source_native == 0 && target_native == 0 {
            self.finish_new_download_handoff(operation_id, true, "hwndless", cx);
            return;
        }
        let command = PlatformCommand::PresentFixedWindowOverAnchor(
            nxtools_protocol::PresentFixedWindowOverAnchorCommand {
                window_handle: target_native,
                anchor_window_handle: source_native,
                width: (window_size.width.as_f32() * scale).round() as i32,
                height: (window_size.height.as_f32() * scale).round() as i32,
            },
        );
        #[cfg(test)]
        if self.new_download_native_handles_override.is_some() {
            self.new_download_handoff_commands.push(command);
            self.schedule_fixed_confirmation_timeout(operation_id, cx);
            return;
        }
        if self
            .platform
            .send_command_with_id(command, operation_id)
            .is_err()
        {
            self.finish_new_download_handoff(operation_id, false, "dispatch_rejected", cx);
            return;
        }
        self.schedule_fixed_confirmation_timeout(operation_id, cx);
    }

    fn schedule_fixed_confirmation_timeout(&self, command_id: uuid::Uuid, cx: &mut Context<Self>) {
        cx.spawn(async move |application, cx| {
            cx.background_executor()
                .timer(FIXED_CONFIRMATION_IPC_TIMEOUT)
                .await;
            let _ = application.update(cx, |application, cx| {
                application.finish_new_download_handoff(command_id, false, "ipc_timeout", cx);
            });
        })
        .detach();
    }

    pub(crate) fn cleanup_new_download_handoff(&mut self, cx: &mut Context<Self>) {
        if let Some(pending) = self.pending_new_download_handoff.as_ref()
            && (!self.fixed_source_is_current(
                pending.source,
                &pending.source_view,
                pending.generation,
                cx,
            ) || !cx.windows().contains(&pending.target))
        {
            self.finish_new_download_handoff(pending.command_id, false, "retired", cx);
        }
        if self
            .new_download_window
            .is_some_and(|window| !cx.windows().contains(&window))
        {
            self.new_download_window = None;
            #[cfg(test)]
            {
                self.new_download_view = None;
            }
        }
    }

    pub(crate) fn finish_new_download_handoff(
        &mut self,
        command_id: uuid::Uuid,
        success: bool,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_new_download_handoff
            .as_ref()
            .is_none_or(|pending| pending.command_id != command_id)
        {
            return;
        }
        let pending = self
            .pending_new_download_handoff
            .take()
            .expect("matched handoff");
        let current = self.fixed_source_is_current(
            pending.source,
            &pending.source_view,
            pending.generation,
            cx,
        );
        let ready = success && current && cx.windows().contains(&pending.target);
        info!(event = "download.fixed_confirmation.finished", correlation_id = %command_id,
            source_native = pending.source_native, target_native = pending.target_native, generation = pending.generation,
            success = ready, reason, elapsed_ms = pending.started_at.elapsed().as_millis(), "fixed confirmation handoff finished");
        if !ready {
            let _ = pending
                .target
                .update(cx, |_, window, _| window.remove_window());
            if self.new_download_window == Some(pending.source)
                && cx.windows().contains(&pending.source)
                && pending
                    .source_view
                    .upgrade()
                    .is_some_and(|view| view.read(cx).probe_generation == pending.generation)
            {
                let _ = pending.source_view.update(cx, |view, cx| {
                    view.handoff_pending = false;
                    view.error = Some("确认窗口未能打开，可重试打开确认；无需重新探测".into());
                    cx.notify();
                });
            }
            return;
        }
        self.new_download_window = Some(pending.target);
        #[cfg(test)]
        {
            self.new_download_view = Some(pending.target_view.clone());
        }
        let _ = pending
            .source
            .update(cx, |_, window, _| window.remove_window());
        let target = pending.target;
        let view = pending.target_view.downgrade();
        cx.defer(move |cx| {
            let _ = target.update(cx, |_, window, cx| {
                let _ = view.update(cx, |view, cx| {
                    view.file_name.focus_handle(cx).focus(window);
                    cx.notify();
                });
            });
        });
        // Never call GPUI activate_window: hidden WindowsWindow retains initial_placement,
        // and its activate() replays stale constructor bounds through SetWindowPlacement.
        if pending.target_native != 0 {
            let _ = self.platform.send_command_correlated(
                PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                    window_handle: pending.target_native,
                    enabled: true,
                }),
                command_id,
            );
            let _ = self.platform.send_command_correlated(
                PlatformCommand::ActivateWindow(ActivateWindowCommand {
                    window_handle: pending.target_native,
                }),
                command_id,
            );
        }
    }

    pub(crate) fn import_browser_blob(
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

    pub(crate) fn discard_browser_blob(&self, context: &BrowserBlobDialogContext) {
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

    /// Publishes the export status into the settings view when it is open.
    /// Safe cross-entity write: the result arrives on the app's own update
    /// turn, never while the view itself is leased.
    pub(crate) fn publish_extension_export_status(&self, status: String, cx: &mut Context<Self>) {
        cx.emit(DownloadApplicationEvent::BrowserExtensionExportCompleted { status });
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
                    command_id,
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

    pub(crate) fn open_new_download_window(&mut self, cx: &mut Context<Self>) {
        let _ = self.open_new_download_window_with_request(None, None, cx);
    }

    pub(crate) fn open_browser_download_dialog(
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

    pub(crate) fn open_new_download_window_with_request(
        &mut self,
        initial_source: Option<NewDownloadInitialSource>,
        browser_context: Option<(uuid::Uuid, uuid::Uuid)>,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        self.cleanup_new_download_handoff(cx);
        if let Some(window) = self.new_download_window {
            if initial_source.is_some() {
                if window.update(cx, |_, _, _| {}).is_ok() {
                    return Err("已有添加下载窗口，请先完成或关闭后重试".into());
                }
            } else if window.update(cx, |_, window, _| window.refresh()).is_ok() {
                let native = window
                    .update(cx, |_, window, _| {
                        native_window_handle(window).unwrap_or_default()
                    })
                    .unwrap_or_default();
                let _ = self.platform.send_command(PlatformCommand::ActivateWindow(
                    ActivateWindowCommand {
                        window_handle: native,
                    },
                ));
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
                // Focus the URL input one frame after the dialog is actually
                // on screen; the constructor races the native first show.
                let focus_view = view.downgrade();
                let focus_window = window.window_handle();
                cx.defer(move |cx| {
                    let _ = focus_window.update(cx, |_, window, cx| {
                        if let Some(view) = focus_view.upgrade() {
                            view.update(cx, |view, cx| view.focus_url_input(window, cx));
                        }
                    });
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

    pub(crate) fn close_schedule_window(&mut self, cx: &mut Context<Self>) {
        if let Some(window) = self.schedule_window.take() {
            let _ = window.update(cx, |_, window, _| window.remove_window());
        }
    }

    pub(crate) fn handle_native_schedule_window_close(&mut self, closing_window: AnyWindowHandle) {
        if self.schedule_window == Some(closing_window) {
            self.schedule_window = None;
        }
    }
}
