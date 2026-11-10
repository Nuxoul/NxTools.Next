//! Directory-picker identity on the existing DownloadApplication and draft views.
use super::*;

#[derive(Default)]
pub(super) struct DownloadDirectoryDraft {
    revision: u64,
    operation: Option<uuid::Uuid>,
}

impl DownloadDirectoryDraft {
    pub(super) fn edited(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.operation = None;
    }

    pub(super) fn begin(&mut self) -> Option<(uuid::Uuid, u64)> {
        if self.operation.is_some() {
            return None;
        }
        let id = uuid::Uuid::new_v4();
        self.operation = Some(id);
        Some((id, self.revision))
    }

    fn matches(&self, request: &DownloadDirectoryPickerRequest) -> bool {
        self.operation == Some(request.command_id) && self.revision == request.revision
    }

    fn release(&mut self, command_id: uuid::Uuid) {
        if self.operation == Some(command_id) {
            self.operation = None;
        }
    }
}

#[derive(Clone)]
pub(super) enum DownloadDirectoryOwner {
    Settings(gpui_kit::WeakEntity<DownloadSettingsView>),
    NewTask(gpui_kit::WeakEntity<NewDownloadView>),
}

#[derive(Clone)]
pub(super) struct DownloadDirectoryPickerRequest {
    pub(super) command_id: uuid::Uuid,
    pub(super) window: AnyWindowHandle,
    pub(super) owner: DownloadDirectoryOwner,
    pub(super) revision: u64,
    pub(super) current_directory: String,
}

pub(super) struct PendingDownloadDirectoryPicker {
    pub(super) request: DownloadDirectoryPickerRequest,
    started_at: Instant,
    result_received: bool,
}

impl DownloadDirectoryPickerRequest {
    fn target(&self) -> DownloadDirectoryTarget {
        match self.owner {
            DownloadDirectoryOwner::Settings(_) => DownloadDirectoryTarget::Settings,
            DownloadDirectoryOwner::NewTask(_) => DownloadDirectoryTarget::NewTask,
        }
    }

    fn is_current(&self, cx: &App) -> bool {
        if !cx.windows().contains(&self.window) {
            return false;
        }
        match &self.owner {
            DownloadDirectoryOwner::Settings(view) => view.upgrade().is_some_and(|view| {
                let view = view.read(cx);
                view.directory_picker.matches(self)
                    && view.directory.read(cx).value().as_str() == self.current_directory
                    && !view.saving
            }),
            DownloadDirectoryOwner::NewTask(view) => view.upgrade().is_some_and(|view| {
                let view = view.read(cx);
                view.directory_picker.matches(self)
                    && view.directory.read(cx).value().as_str() == self.current_directory
                    && !view.submitting
                    && !view.handoff_pending
                    && !view.handoff_staged
            }),
        }
    }

    fn release(&self, cx: &mut App) {
        match &self.owner {
            DownloadDirectoryOwner::Settings(view) => {
                let _ = view.update(cx, |view, _| view.directory_picker.release(self.command_id));
            }
            DownloadDirectoryOwner::NewTask(view) => {
                let _ = view.update(cx, |view, _| view.directory_picker.release(self.command_id));
            }
        }
    }

    fn apply(&self, path: String, window: &mut Window, cx: &mut App) -> bool {
        if window.window_handle() != self.window {
            return false;
        }
        match &self.owner {
            DownloadDirectoryOwner::Settings(view) => view
                .update(cx, |view, cx| {
                    if !view.directory_picker.matches(self)
                        || view.saving
                        || view.directory.read(cx).value().as_str() != self.current_directory
                    {
                        return false;
                    }
                    view.directory_picker.edited();
                    view.directory
                        .update(cx, |input, cx| input.set_value(path, window, cx));
                    view.error = None;
                    #[cfg(test)]
                    {
                        view.category_presentation_revision =
                            view.category_presentation_revision.wrapping_add(1);
                    }
                    cx.notify();
                    true
                })
                .unwrap_or(false),
            DownloadDirectoryOwner::NewTask(view) => view
                .update(cx, |view, cx| {
                    if !view.directory_picker.matches(self)
                        || view.submitting
                        || view.handoff_pending
                        || view.handoff_staged
                        || view.directory.read(cx).value().as_str() != self.current_directory
                    {
                        return false;
                    }
                    view.directory_picker.edited();
                    view.directory
                        .update(cx, |input, cx| input.set_value(path, window, cx));
                    view.save_directory_explicit = true;
                    view.error = None;
                    cx.notify();
                    true
                })
                .unwrap_or(false),
        }
    }
}

impl DownloadApplication {
    pub(super) fn request_directory(
        &mut self,
        request: DownloadDirectoryPickerRequest,
        cx: &mut Context<Self>,
    ) {
        self.cleanup_directory_picker(cx);
        if !request.is_current(cx) || self.pending_directory_picker.is_some() {
            let reason = if self.pending_directory_picker.is_some() {
                "already_pending"
            } else {
                "origin_retired"
            };
            info!(event = "download.directory_picker.retired", command_id = %request.command_id,
                reason, "directory picker request was not admitted");
            cx.defer(move |cx| request.release(cx));
            return;
        }
        let command_id = request.command_id;
        let target = request.target();
        let source_window_handle = request
            .window
            .update(cx, |_, window, _| {
                crate::ui::native_window_handle(window).unwrap_or_default()
            })
            .unwrap_or_default();
        let command = PlatformCommand::PickDirectory(PickDirectoryCommand {
            current_directory: request.current_directory.clone(),
            title: "选择下载保存目录".into(),
            source_window_handle,
            source_process_id: std::process::id(),
        });
        self.pending_directory_picker = Some(PendingDownloadDirectoryPicker {
            request,
            started_at: Instant::now(),
            result_received: false,
        });
        match self
            .platform
            .send_command_with_id_correlated(command, command_id, command_id)
        {
            Ok(_) => info!(event = "download.directory_picker.accepted", %command_id, ?target,
                "download directory picker accepted"),
            Err(error) => {
                self.retire_directory_picker("dispatch_rejected", cx);
                self.notifications.update(cx, |notifications, cx| {
                    notifications.show_error(
                        format!("download-directory-picker-{command_id}"),
                        "无法打开目录选择器",
                        error.to_string(),
                        cx,
                    )
                });
            }
        }
        cx.notify();
    }

    pub(super) fn cleanup_directory_picker(&mut self, cx: &mut Context<Self>) {
        if self
            .pending_directory_picker
            .as_ref()
            .is_some_and(|pending| !pending.request.is_current(cx))
        {
            self.retire_directory_picker("origin_or_draft_retired", cx);
        }
    }

    pub(super) fn retire_directory_picker(&mut self, reason: &'static str, cx: &mut Context<Self>) {
        if let Some(pending) = self.pending_directory_picker.take() {
            info!(event = "download.directory_picker.retired", command_id = %pending.request.command_id,
                target = ?pending.request.target(), elapsed_ms = pending.started_at.elapsed().as_millis(),
                reason, "download directory picker logically retired");
            cx.defer(move |cx| pending.request.release(cx));
        }
    }

    pub(super) fn receive_directory_result(
        &mut self,
        command_id: uuid::Uuid,
        result: PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending_directory_picker.as_ref() else {
            return;
        };
        if pending.request.command_id != command_id || pending.result_received {
            return;
        }
        if !pending.request.is_current(cx) {
            self.retire_directory_picker("origin_or_draft_retired", cx);
            return;
        }
        let pending = self
            .pending_directory_picker
            .as_mut()
            .expect("matched original picker");
        pending.result_received = true;
        let application = cx.entity().downgrade();
        // Results can precede queued InputEvent::Change in this same app update.
        // A second turn lets those edits settle before the final origin/revision check.
        cx.defer(move |cx| {
            cx.defer(move |cx| {
                let _ = application.update(cx, |application, cx| {
                    application.finish_directory_result(command_id, result, cx)
                });
            })
        });
    }

    fn finish_directory_result(
        &mut self,
        command_id: uuid::Uuid,
        result: PlatformCommandResult,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_directory_picker
            .as_ref()
            .is_none_or(|pending| pending.request.command_id != command_id)
        {
            return;
        }
        self.cleanup_directory_picker(cx);
        let Some(pending) = self.pending_directory_picker.take() else {
            return;
        };
        let target = pending.request.target();
        let elapsed_ms = pending.started_at.elapsed().as_millis();
        if result.success {
            if let Some(path) = result.data.filter(|path| !path.trim().is_empty()) {
                let applied = pending
                    .request
                    .window
                    .update(cx, |_, window, cx| pending.request.apply(path, window, cx))
                    .unwrap_or(false);
                info!(event = "download.directory_picker.completed", %command_id, ?target, elapsed_ms,
                    applied, "download directory picker result application finished");
            } else {
                info!(event = "download.directory_picker.cancelled", %command_id, ?target, elapsed_ms,
                    "download directory picker completed without a selection");
            }
        } else {
            let native_error_present = result.error.is_some();
            warn!(event = "download.directory_picker.failed", %command_id, ?target, elapsed_ms,
                native_error_code = result.native_error_code, native_error_present, "download directory picker failed");
            self.notifications.update(cx, |notifications, cx| {
                notifications.show_error(
                    format!("download-directory-picker-{command_id}"),
                    "无法选择下载目录",
                    result
                        .error
                        .unwrap_or_else(|| "Windows 未返回错误详情".into()),
                    cx,
                )
            });
        }
        pending.request.release(cx);
        cx.notify();
    }
}
