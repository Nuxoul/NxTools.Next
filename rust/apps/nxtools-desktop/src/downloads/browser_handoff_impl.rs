// Browser handoff and the new-download entry surface: blob import and
// discard, extension export requests, the browser download confirmation
// dialog, the new-download window lifecycle and the schedule window.
// Split from downloads.rs — same type, separate impl block.

use super::*;

const FIXED_CONFIRMATION_IPC_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) const CHROME_STORE_EXTENSION_ID: &str = "eldfplghejhopcmgfkacimanhaojakph";

pub(super) struct PendingNewDownloadHandoff {
    pub(super) command_id: uuid::Uuid,
    started_at: Instant,
    source: AnyWindowHandle,
    source_view: gpui_kit::WeakEntity<NewDownloadView>,
    generation: u64,
    pub(super) target: AnyWindowHandle,
    target_view: Entity<NewDownloadView>,
    source_native: i64,
    target_native: i64,
}

impl DownloadApplication {
    fn fixed_source_is_current(
        &self,
        source: AnyWindowHandle,
        view: &gpui_kit::WeakEntity<NewDownloadView>,
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
        source_view: gpui_kit::WeakEntity<NewDownloadView>,
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
                if let Some(context) = source_view
                    .upgrade()
                    .and_then(|view| view.read(cx).controlled_handoff.clone())
                    && context.claim_abort_for_window(source)
                {
                    self.abort_controlled_browser_handoff(
                        &context,
                        "confirmation_window_create_failed",
                        cx,
                    );
                }
                let _ = source_view.update(cx, |view, cx| {
                    view.handoff_pending = false;
                    view.error = Some(if view.controlled_handoff.is_some() {
                        "确认窗口未能打开，浏览器接管已结束；请从浏览器重新发起".into()
                    } else {
                        "确认窗口未能打开，可重试打开确认；无需重新探测".into()
                    });
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
        let mut ready = success && current && cx.windows().contains(&pending.target);
        if ready {
            ready = pending
                .target_view
                .read(cx)
                .transfer_controlled_handoff_owner(pending.source, pending.target);
        }
        info!(event = "download.fixed_confirmation.finished", correlation_id = %command_id,
            source_native = pending.source_native, target_native = pending.target_native, generation = pending.generation,
            success = ready, reason, elapsed_ms = pending.started_at.elapsed().as_millis(), "fixed confirmation handoff finished");
        if !ready {
            if let Some(context) = pending
                .source_view
                .upgrade()
                .and_then(|view| view.read(cx).controlled_handoff.clone())
                && context.claim_abort_for_window(pending.source)
            {
                self.abort_controlled_browser_handoff(&context, "confirmation_window_failed", cx);
            }
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
                    view.error = Some(if view.controlled_handoff.is_some() {
                        "确认窗口未能打开，浏览器接管已结束；请从浏览器重新发起".into()
                    } else {
                        "确认窗口未能打开，可重试打开确认；无需重新探测".into()
                    });
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
                    view.file_name.focus_handle(cx).focus(window, cx);
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

    pub(crate) fn controlled_browser_handoff_status(
        &self,
        context: &ControlledBrowserDialogContext,
    ) -> Result<(BrowserHandoffState, Option<DownloadTaskId>), String> {
        match self.execute_correlated(
            DownloadCommand::BrowserHandoffStatus {
                handoff_id: context.handoff_id,
            },
            context.correlation_id,
        ) {
            DownloadCommandOutcome::BrowserHandoff {
                handoff_id,
                state,
                task_id,
            } if handoff_id == context.handoff_id => Ok((state, task_id)),
            DownloadCommandOutcome::BrowserHandoff { .. } => {
                Err("下载引擎返回了不匹配的浏览器接管身份".into())
            }
            DownloadCommandOutcome::Rejected { fault } => Err(fault.message),
            _ => Err("下载引擎返回了无效的浏览器接管状态".into()),
        }
    }

    pub(crate) fn accept_controlled_browser_handoff(
        &mut self,
        context: &ControlledBrowserDialogContext,
        request: DownloadRequest,
        mode: DownloadSubmitMode,
        resolution: nxtools_download::DownloadResolutionOptions,
        cx: &mut Context<Self>,
    ) -> Result<DownloadTaskId, String> {
        match self.controlled_browser_handoff_status(context)? {
            (BrowserHandoffState::Pending, None) => {}
            (BrowserHandoffState::Held, Some(task_id)) => {
                self.refresh(cx);
                return Ok(task_id);
            }
            (BrowserHandoffState::Aborted, _) => {
                context.claim_abort();
                return Err("浏览器接管已取消，不能再确认此下载".into());
            }
            (BrowserHandoffState::Expired, _) => {
                context.claim_abort();
                return Err("浏览器接管确认已过期，请从浏览器重新发起".into());
            }
            (BrowserHandoffState::Missing, _) => {
                context.claim_abort();
                return Err("浏览器接管记录不存在，请从浏览器重新发起".into());
            }
            (BrowserHandoffState::Committed, _) => {
                context.claim_abort();
                return Err("浏览器接管已由原操作完成，不能重复确认".into());
            }
            _ => return Err("浏览器接管状态与任务身份不一致".into()),
        }
        let outcome = self.execute_correlated(
            DownloadCommand::BrowserAcceptHandoff {
                handoff_id: context.handoff_id,
                request,
                mode,
                resolution,
            },
            context.correlation_id,
        );
        let task_id = match outcome {
            DownloadCommandOutcome::BrowserHandoff {
                handoff_id,
                state: BrowserHandoffState::Held,
                task_id: Some(task_id),
            } if handoff_id == context.handoff_id => task_id,
            DownloadCommandOutcome::BrowserHandoff {
                handoff_id,
                state: BrowserHandoffState::Aborted,
                ..
            } if handoff_id == context.handoff_id => {
                context.claim_abort();
                return Err("浏览器接管已取消，不能再确认此下载".into());
            }
            DownloadCommandOutcome::BrowserHandoff {
                handoff_id,
                state: BrowserHandoffState::Expired,
                ..
            } if handoff_id == context.handoff_id => {
                context.claim_abort();
                return Err("浏览器接管确认已过期，请从浏览器重新发起".into());
            }
            DownloadCommandOutcome::BrowserHandoff {
                handoff_id,
                state: BrowserHandoffState::Missing,
                ..
            } if handoff_id == context.handoff_id => {
                context.claim_abort();
                return Err("浏览器接管记录不存在，请从浏览器重新发起".into());
            }
            DownloadCommandOutcome::BrowserHandoff {
                handoff_id,
                state: BrowserHandoffState::Committed,
                ..
            } if handoff_id == context.handoff_id => {
                context.claim_abort();
                return Err("浏览器接管已由原操作完成，不能重复确认".into());
            }
            DownloadCommandOutcome::BrowserHandoff {
                handoff_id,
                state: BrowserHandoffState::Pending,
                ..
            } if handoff_id == context.handoff_id => {
                return Err("浏览器接管尚未完成持久化，请重试确认".into());
            }
            DownloadCommandOutcome::BrowserHandoff { .. } => {
                return Err("下载引擎返回了不匹配的浏览器接管身份".into());
            }
            DownloadCommandOutcome::Rejected { fault } => return Err(fault.message),
            _ => return Err("下载引擎返回了无效的浏览器接管结果".into()),
        };
        self.refresh(cx);
        info!(
            event = "download.browser_handoff.held",
            correlation_id = %context.correlation_id,
            handoff_id = %context.handoff_id,
            task_id = %task_id.as_uuid(),
            ?mode,
            "browser handoff accepted into held state"
        );
        Ok(task_id)
    }

    pub(crate) fn abort_controlled_browser_handoff(
        &mut self,
        context: &ControlledBrowserDialogContext,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        let outcome = self.execute_correlated(
            DownloadCommand::BrowserAbortHandoff {
                handoff_id: context.handoff_id,
            },
            context.correlation_id,
        );
        match outcome {
            DownloadCommandOutcome::BrowserHandoff {
                handoff_id,
                state,
                task_id,
            } if handoff_id == context.handoff_id => {
                if task_id.is_some() {
                    self.refresh(cx);
                }
                info!(
                    event = "download.browser_handoff.desktop_terminal",
                    correlation_id = %context.correlation_id,
                    handoff_id = %context.handoff_id,
                    ?state,
                    reason,
                    "desktop browser handoff ownership ended"
                );
            }
            DownloadCommandOutcome::Rejected { fault } => warn!(
                event = "download.browser_handoff.desktop_abort_failed",
                correlation_id = %context.correlation_id,
                handoff_id = %context.handoff_id,
                fault_code = ?fault.code,
                retryable = fault.retryable,
                reason,
                "desktop could not abort browser handoff"
            ),
            _ => warn!(
                event = "download.browser_handoff.desktop_abort_failed",
                correlation_id = %context.correlation_id,
                handoff_id = %context.handoff_id,
                reason,
                "desktop received an invalid browser handoff abort result"
            ),
        }
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

    /// The shared screen center survives closing the settings window during export.
    pub(crate) fn publish_extension_export_status(
        &self,
        operation_id: uuid::Uuid,
        status: String,
        cx: &mut Context<Self>,
    ) {
        let center = self.notifications.clone();
        let level = if status.starts_with("导出失败") {
            crate::notifications::NotificationLevel::Error
        } else if status.starts_with("已取消") {
            crate::notifications::NotificationLevel::Info
        } else {
            crate::notifications::NotificationLevel::Success
        };
        let notification = crate::notifications::AppNotification::new(
            format!("download-browser-export-{operation_id}"),
            level,
            "浏览器扩展导出",
            status,
        );
        cx.defer(move |cx| center.update(cx, |center, cx| center.push(notification, cx)));
    }

    pub(crate) fn request_browser_extension_export(
        &mut self,
        format: &'static str,
        operation_id: uuid::Uuid,
        source: AnyWindowHandle,
        owner: gpui_kit::WeakEntity<DownloadSettingsView>,
        source_window_handle: i64,
    ) -> Result<(), String> {
        if self.pending_extension_export.is_some() {
            return Err("已有导出任务在进行".into());
        }
        self.pending_extension_export = Some(PendingExtensionExport {
            command_id: operation_id,
            operation_id,
            format,
            started_at: Instant::now(),
            selection_origin: Some((source, owner)),
        });
        match self.platform.send_command_with_id_correlated(
            PlatformCommand::PickBrowserExtensionExportPath(
                PickBrowserExtensionExportPathCommand {
                    format: format.into(),
                    source_window_handle,
                    source_process_id: std::process::id(),
                },
            ),
            operation_id,
            operation_id,
        ) {
            Ok(command_id) => {
                info!(event = "download.browser_extension.selection_accepted", %operation_id,
                    %command_id, format, "browser extension destination selection accepted");
                Ok(())
            }
            Err(error) => {
                self.pending_extension_export = None;
                warn!(event = "download.browser_extension.export_rejected", %operation_id,
                    format, "browser extension destination selection could not be queued");
                Err(format!("导出失败:{error}"))
            }
        }
    }

    fn dismiss_extension_export_status(&self, operation_id: uuid::Uuid, cx: &mut Context<Self>) {
        let center = self.notifications.clone();
        let id = format!("download-browser-export-{operation_id}");
        cx.defer(move |cx| center.update(cx, |center, cx| center.dismiss_current(id, cx)));
    }

    pub(super) fn retire_extension_export_selection(&mut self, cx: &mut Context<Self>) {
        if self
            .pending_extension_export
            .as_ref()
            .is_some_and(|pending| pending.selection_origin.is_some())
        {
            let pending = self
                .pending_extension_export
                .take()
                .expect("selection pending");
            self.dismiss_extension_export_status(pending.operation_id, cx);
            info!(event = "download.browser_extension.selection_retired", operation_id = %pending.operation_id,
                elapsed_ms = pending.started_at.elapsed().as_millis(), reason = "window_closed",
                "browser extension destination owner retired");
        }
    }

    pub(super) fn handle_browser_extension_export_result(
        &mut self,
        command_id: uuid::Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        if self
            .pending_extension_export
            .as_ref()
            .is_none_or(|pending| pending.command_id != command_id)
        {
            return false;
        }
        let mut pending = self
            .pending_extension_export
            .take()
            .expect("matching export command");
        let elapsed_ms = pending.started_at.elapsed().as_millis();
        if let Some((source, owner)) = pending.selection_origin.take() {
            if !cx.windows().contains(&source)
                || owner.upgrade().is_none()
                || self.settings_window != Some(source)
            {
                self.dismiss_extension_export_status(pending.operation_id, cx);
                info!(event = "download.browser_extension.selection_retired", operation_id = %pending.operation_id,
                    elapsed_ms, reason = "origin_retired", "browser extension destination owner retired");
                return true;
            }
            let destination = result.data.as_deref().unwrap_or_default();
            if result.success && destination.is_empty() {
                self.dismiss_extension_export_status(pending.operation_id, cx);
                info!(event = "download.browser_extension.export_cancelled", operation_id = %pending.operation_id,
                    elapsed_ms, "browser extension destination selection cancelled");
                return true;
            }
            if result.success {
                let export_id = uuid::Uuid::new_v4();
                pending.command_id = export_id;
                let operation_id = pending.operation_id;
                let format = pending.format;
                self.pending_extension_export = Some(pending);
                if self
                    .platform
                    .send_command_with_id_correlated(
                        PlatformCommand::ExportBrowserExtension(ExportBrowserExtensionCommand {
                            format: format.into(),
                            destination: destination.into(),
                        }),
                        export_id,
                        operation_id,
                    )
                    .is_err()
                {
                    self.pending_extension_export = None;
                    self.publish_extension_export_status(
                        operation_id,
                        "导出失败:无法发送导出请求".into(),
                        cx,
                    );
                    warn!(event = "download.browser_extension.export_failed", %operation_id, elapsed_ms,
                        failure_kind = "dispatch_rejected", "browser extension export could not be queued");
                } else {
                    info!(event = "download.browser_extension.export_started", %operation_id,
                        command_id = %export_id, format, "browser extension export started");
                }
                return true;
            }
        }
        if result.success {
            let file_name = result
                .data
                .as_deref()
                .and_then(|path| std::path::Path::new(path).file_name())
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            self.publish_extension_export_status(
                pending.operation_id,
                format!("已导出:{file_name}"),
                cx,
            );
            info!(event = "download.browser_extension.exported", operation_id = %pending.operation_id,
                format = pending.format, elapsed_ms, "browser extension export completed");
        } else {
            let reason = result.error.as_deref().unwrap_or("Windows 未返回错误详情");
            self.publish_extension_export_status(
                pending.operation_id,
                format!("导出失败:{reason}"),
                cx,
            );
            warn!(event = "download.browser_extension.export_failed", operation_id = %pending.operation_id,
                format = pending.format, native_error_code = ?result.native_error_code,
                elapsed_ms, "browser extension export failed");
        }
        cx.notify();
        true
    }

    pub(crate) fn admits_link_download(url: &str) -> bool {
        clipboard_prefill::valid_download_url(url)
    }

    pub(crate) fn open_link_download(
        &mut self,
        url: String,
        correlation_id: uuid::Uuid,
        started_at: Instant,
        cx: &mut Context<Self>,
    ) {
        let outcome = if !Self::admits_link_download(&url) {
            "invalid_target"
        } else if self
            .open_new_download_window_with_request(
                Some(NewDownloadInitialSource::Url {
                    url,
                    correlation_id,
                }),
                None,
                cx,
            )
            .is_ok()
        {
            "intake_ready"
        } else {
            "open_failed"
        };
        info!(event = "tools.link_panel.download_finished", %correlation_id, outcome,
            elapsed_ms = started_at.elapsed().as_millis() as u64,
            "explicit link download intake finished");
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
            BrowserDownloadDialogSource::ControlledDownload(request) => {
                NewDownloadInitialSource::ControlledDownload {
                    request: *request,
                    context: ControlledBrowserDialogContext::new(correlation_id, handoff_id),
                }
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
        let (explicit_url, intake_correlation) = match initial_source.as_ref() {
            Some(NewDownloadInitialSource::Url {
                url,
                correlation_id,
            }) => (Some(url.clone()), Some(*correlation_id)),
            _ => (None, None),
        };
        if let Some(window) = self.new_download_window {
            if initial_source.is_some() && explicit_url.is_none() {
                if window.update(cx, |_, _, _| {}).is_ok() {
                    return Err("已有添加下载窗口，请先完成或关闭后重试".into());
                }
            } else if window
                .update(cx, |root, native_window, cx| {
                    native_window.refresh();
                    if let Some(url) = explicit_url.clone() {
                        let view = root.downcast::<Root>().ok().and_then(|root| {
                            root.read(cx)
                                .view()
                                .clone()
                                .downcast::<NewDownloadView>()
                                .ok()
                        });
                        if let Some(view) = view {
                            cx.defer(move |cx| {
                                let _ = window.update(cx, |_, window, cx| {
                                    let current =
                                        view.read(cx).application.read(cx).new_download_window;
                                    if current != Some(window.window_handle()) {
                                        return;
                                    }
                                    view.update(cx, |view, cx| {
                                        let prefilled = view.prefill_clipboard_url(url, window, cx);
                                        view.focus_url_input(window, cx);
                                        if prefilled {
                                            view.begin_probe(window, cx);
                                        }
                                    });
                                });
                            });
                        }
                    }
                })
                .is_ok()
            {
                if explicit_url.is_some() {
                    self.retire_new_download_clipboard("explicit_url");
                }
                let native = window
                    .update(cx, |_, window, _| {
                        native_window_handle(window).unwrap_or_default()
                    })
                    .unwrap_or_default();
                let _ = self.platform.send_command_correlated(
                    PlatformCommand::ActivateWindow(ActivateWindowCommand {
                        window_handle: native,
                    }),
                    intake_correlation.unwrap_or_else(uuid::Uuid::new_v4),
                );
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
        let auto_probe = matches!(
            initial_source.as_ref(),
            Some(
                NewDownloadInitialSource::Url { .. }
                    | NewDownloadInitialSource::Download(_)
                    | NewDownloadInitialSource::ControlledDownload { .. }
            )
        );
        let prefill_clipboard = initial_source.is_none();
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
                let clipboard_application = application.downgrade();
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
                if prefill_clipboard {
                    let clipboard_view = view.downgrade();
                    let clipboard_window = window.window_handle();
                    cx.defer(move |cx| {
                        let _ = clipboard_application.update(cx, |app, cx| {
                            app.request_new_download_clipboard(
                                clipboard_window,
                                clipboard_view,
                                cx,
                            );
                        });
                    });
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
                    .or(intake_correlation)
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
                ..Default::default()
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

impl DownloadApplication {
    pub(crate) fn request_browser_registration(
        &mut self,
        correlation_id: uuid::Uuid,
        cx: &mut Context<Self>,
    ) {
        let configured = &self.config.download.chrome_web_store_extension_id;
        let chromium_extension_id = if configured.is_empty() {
            CHROME_STORE_EXTENSION_ID.to_owned()
        } else {
            configured.clone()
        };
        let command_id = correlation_id;
        if let Some((retired_id, started)) = self.pending_browser_registration.take() {
            info!(event = "download.browser_native_messaging.registration_retired", command_id = %retired_id,
                reason = "superseded", elapsed_ms = started.elapsed().as_millis(), "old registration ownership retired");
        }
        self.pending_browser_registration = Some((command_id, Instant::now()));
        match self.platform.send_command_with_id(
            PlatformCommand::EnsureBrowserNativeMessaging(EnsureBrowserNativeMessagingCommand {
                enabled: true,
                chromium_extension_id,
                firefox_extension_id: "nxtools-download-interceptor@nxtools.local".into(),
            }),
            command_id,
        ) {
            Ok(_) => {
                info!(event = "download.browser_native_messaging.registration_requested",
                    %correlation_id, %command_id, "exact browser registration queued");
                cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(Duration::from_secs(15))
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        if this
                            .pending_browser_registration
                            .is_some_and(|(id, _)| id == command_id)
                        {
                            this.finish_browser_registration(false, None, "timeout", cx);
                        }
                    });
                })
                .detach();
            }
            Err(_) => self.finish_browser_registration(false, None, "queue_rejected", cx),
        }
    }

    pub(super) fn finish_browser_registration(
        &mut self,
        success: bool,
        native_error_code: Option<i32>,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some((command_id, started)) = self.pending_browser_registration.take() else {
            return;
        };
        info!(event = "download.browser_native_messaging.registration_completed", %command_id,
            success, reason, ?native_error_code, elapsed_ms = started.elapsed().as_millis(),
            "browser registration terminal result");
        if !success {
            let center = self.notifications.clone();
            cx.defer(move |cx| {
                center.update(cx, |center, cx| {
                    center.show_error(
                        "browser-native-registration",
                        "浏览器扩展连接配置未应用",
                        "已保存的设置仍保留。请重新保存下载设置或重启 NxTools 后重试连接。",
                        cx,
                    );
                })
            });
        }
    }
}

pub(super) const FIREFOX_STORE_URL: &str =
    "https://addons.mozilla.org/zh-CN/firefox/addon/nxtools-download-interceptor/";
impl DownloadApplication {
    pub(crate) fn open_chrome_store(&mut self, cx: &mut Context<Self>) {
        self.open_browser_store(
            "chrome",
            format!("https://chromewebstore.google.com/detail/nxtools-download-intercep/{CHROME_STORE_EXTENSION_ID}"),
            cx,
        );
    }

    pub(crate) fn open_firefox_store(&mut self, cx: &mut Context<Self>) {
        self.open_browser_store("firefox", FIREFOX_STORE_URL.into(), cx);
    }

    fn open_browser_store(&mut self, store: &'static str, target: String, cx: &mut Context<Self>) {
        if self.pending_browser_store_page.is_some() {
            return;
        }
        let id = uuid::Uuid::new_v4();
        self.pending_browser_store_page = Some((id, Instant::now()));
        info!(event="download.browser_store.open_started", correlation_id=%id, store);
        if self
            .platform
            .send_command_with_id(
                PlatformCommand::OpenShellTarget(OpenShellTargetCommand {
                    target,
                    action: ShellOpenAction::Open,
                }),
                id,
            )
            .is_err()
        {
            self.finish_browser_store_page(false, None, cx);
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(10))
                .await;
            let _ = this.update(cx, |owner, cx| {
                if owner
                    .pending_browser_store_page
                    .is_some_and(|(current, _)| current == id)
                {
                    owner.finish_browser_store_page(false, None, cx);
                }
            });
        })
        .detach();
    }
    pub(super) fn handle_browser_store_page(
        &mut self,
        notification: &PlatformNotification,
        cx: &mut Context<Self>,
    ) -> bool {
        match notification {
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } if self
                .pending_browser_store_page
                .is_some_and(|(id, _)| id == *correlation_id) =>
            {
                self.finish_browser_store_page(result.success, result.native_error_code, cx);
                true
            }
            PlatformNotification::Disconnected { .. } => {
                if self.pending_browser_store_page.is_some() {
                    self.finish_browser_store_page(false, None, cx);
                }
                false
            }
            _ => false,
        }
    }
    fn finish_browser_store_page(
        &mut self,
        success: bool,
        native_error_code: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        let Some((id, started)) = self.pending_browser_store_page.take() else {
            return;
        };
        info!(event="download.browser_store.open_terminal", correlation_id=%id, success, ?native_error_code, elapsed_ms=started.elapsed().as_millis());
        if !success {
            let center = self.notifications.clone();
            cx.defer(move |cx| {
                center.update(cx, |center, cx| {
                    center.show_error(
                        format!("browser-store-{id}"),
                        "浏览器扩展",
                        "无法打开扩展商店页面，请检查默认浏览器后重试。",
                        cx,
                    )
                })
            });
        }
    }
}
