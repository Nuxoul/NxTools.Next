// Browser handoff and the new-download entry surface: blob import and
// discard, extension export requests, the browser download confirmation
// dialog, the new-download window lifecycle and the schedule window.
// Split from downloads.rs — same type, separate impl block.

use super::*;

impl DownloadApplication {
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
