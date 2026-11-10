//! On-demand intake on the original application; no clipboard capture engine or job queue.
use super::*;
use nxtools_protocol::{PlatformCommand, PlatformCommandResult, archive_clipboard::*};

type Target = (PathBuf, ArchiveBrowserContext, String);
pub(super) struct Pending {
    command: ArchiveClipboardCommand,
    connection: u64,
    target: Option<Target>,
    started: Instant,
    _deadline: Task<()>,
}
impl ArchiveApplication {
    pub(super) fn intake_target_current(
        &self,
        form: Uuid,
        target: &Option<Target>,
        cx: &App,
    ) -> bool {
        let Some((path, browser, folder)) = target else {
            return !self.jobs.edit_forms.contains_key(&form);
        };
        self.jobs.edit_forms.get(&form).is_some_and(|origin| {
            let (id, generation, snapshot) = &origin.browser;
            self.platform.connection_generation() == Some(origin.connection)
                && self.windows.get(id).is_some_and(|slot| {
                    slot.generation == *generation
                        && slot.path == *path
                        && slot.browser.as_ref() == Some(browser)
                        && *snapshot == browser.snapshot_id
                        && slot
                            .view
                            .read_with(cx, |view, _| {
                                view.edit_reason().is_none()
                                    && view.edit_source.as_ref() == Some(&browser.source)
                                    && view.model.as_ref().is_some_and(|model| {
                                        model.snapshot == *snapshot && model.folder == *folder
                                    })
                            })
                            .unwrap_or(false)
                })
        })
    }
    pub(in crate::archive) fn read_files_for_form(
        &mut self,
        command: ArchiveClipboardCommand,
        cx: &mut Context<Self>,
    ) {
        let form = command.session_id;
        let Some(slot) = self
            .jobs
            .forms
            .get(&form)
            .filter(|slot| slot.generation == command.generation)
        else {
            return;
        };
        let Some(target) = slot
            .view
            .read_with(cx, |form, _| {
                form.clipboard_current(&command)
                    .then(|| form.intake_target())
            })
            .ok()
            .flatten()
        else {
            return;
        };
        let connection = self.platform.connection_generation();
        if command.validate().is_err()
            || connection.is_none()
            || !self.intake_target_current(form, &target, cx)
            || self.jobs.clipboards.len() >= 8
            || self
                .jobs
                .clipboards
                .values()
                .any(|pending| pending.command.session_id == form)
        {
            self.deliver_clipboard(
                command,
                target,
                connection,
                Err("原目标已变化或文件读取不可用，请重试。".into()),
                cx,
            );
            return;
        }
        let operation = command.operation_id;
        let deadline = cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(7))
                .await;
            let _ = owner.update(cx, |app, cx| {
                app.clipboard_result(
                    operation,
                    &PlatformCommandResult {
                        success: false,
                        error: Some("clipboard_timeout".into()),
                        native_error_code: None,
                        data: None,
                    },
                    cx,
                );
            });
        });
        self.jobs.clipboards.insert(
            operation,
            Pending {
                command: command.clone(),
                connection: connection.expect("connected"),
                target,
                started: Instant::now(),
                _deadline: deadline,
            },
        );
        tracing::info!(event="archive.clipboard.accepted",correlation_id=%operation,session_id=%form,generation=command.generation,"file-list read accepted");
        if self
            .platform
            .send_command_with_id_correlated(
                PlatformCommand::ReadClipboardFileDrop(command),
                operation,
                operation,
            )
            .is_err()
        {
            self.clipboard_result(
                operation,
                &PlatformCommandResult {
                    success: false,
                    error: Some("disconnected".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        }
    }
    pub(super) fn clipboard_result(
        &mut self,
        operation: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pending) = self.jobs.clipboards.remove(&operation) else {
            return false;
        };
        let response = result
            .data
            .as_deref()
            .filter(|data| data.len() <= 900_000)
            .and_then(|data| serde_json::from_str::<ArchiveClipboardResult>(data).ok())
            .filter(|response| response.validate(&pending.command).is_ok());
        let outcome = if self.platform.connection_generation() != Some(pending.connection)
            || !self.intake_target_current(pending.command.session_id, &pending.target, cx)
        {
            Err("连接或原 Add 目标已变化；未添加任何来源。".into())
        } else if result.success {
            response
                .filter(|response| response.code.is_none())
                .map(|response| response.paths)
                .ok_or_else(|| "剪贴板文件列表无效或超过预算；未添加任何来源。".into())
        } else {
            Err(match result.error.as_deref() {
                Some("clipboard_timeout" | "platform command timed out") => {
                    "文件读取超时，请重试。"
                }
                Some("clipboard_limit") => "剪贴板文件列表超过上限；未添加任何来源。",
                Some("invalid_clipboard_data") => "剪贴板文件列表无效；未添加任何来源。",
                _ => "剪贴板文件读取失败，请重试。",
            }
            .into())
        };
        tracing::info!(event="archive.clipboard.finished",correlation_id=%operation,session_id=%pending.command.session_id,success=outcome.is_ok(),native_error_code=?result.native_error_code,elapsed_ms=pending.started.elapsed().as_millis() as u64,"file-list read finished");
        self.deliver_clipboard(
            pending.command,
            pending.target,
            Some(pending.connection),
            outcome,
            cx,
        );
        true
    }
    fn deliver_clipboard(
        &self,
        command: ArchiveClipboardCommand,
        target: Option<Target>,
        connection: Option<u64>,
        result: Result<Vec<String>, String>,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self
            .jobs
            .forms
            .get(&command.session_id)
            .filter(|slot| slot.generation == command.generation)
        else {
            return;
        };
        let view = slot.view.clone();
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let current = owner
                .read_with(cx, |app, cx| {
                    connection.is_some()
                        && app.platform.connection_generation() == connection
                        && app.intake_target_current(command.session_id, &target, cx)
                })
                .unwrap_or(false);
            let result = if current {
                result
            } else {
                Err("连接或原 Add 目标已变化；未添加任何来源。".into())
            };
            let _ = view.update(cx, |form, cx| form.clipboard_result(&command, result, cx));
        });
    }
    pub(super) fn clipboard_disconnected(&mut self, generation: u64, cx: &mut Context<Self>) {
        let operations = self
            .jobs
            .clipboards
            .iter()
            .filter(|(_, pending)| pending.connection == generation)
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for operation in operations {
            self.clipboard_result(
                operation,
                &PlatformCommandResult {
                    success: false,
                    error: Some("disconnected".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        }
    }
    pub(super) fn retire_form_clipboard(&mut self, form: Uuid) {
        self.jobs.clipboards.retain(|operation, pending| {
            if pending.command.session_id != form { return true; }
            tracing::info!(event="archive.clipboard.finished",correlation_id=%operation,session_id=%form,success=false,reason="form_retired",elapsed_ms=pending.started.elapsed().as_millis() as u64,"file-list read retired");
            false
        });
    }
}
