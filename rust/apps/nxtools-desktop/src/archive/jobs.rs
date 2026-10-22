//! Application-only FIFO and presentation over the original native worker session.
use super::*;
use crate::archive::{
    draft::ModifyDraft,
    forms::{FormCompletion, OperationForm},
    progress::JobProgressView,
    queue::{JobQueue, Resources},
};
use nxtools_protocol::{PlatformCommand, PlatformEvent, archive_job::*, archive_picker::*};
use std::collections::VecDeque;

pub(super) struct FormSlot {
    pub view: WeakEntity<OperationForm>,
    pub window: AnyWindowHandle,
    pub generation: u64,
}
struct PendingPicker {
    form: Uuid,
    generation: u64,
    operation: Uuid,
}
pub(super) struct JobRecord {
    pub command: ArchiveJobCommand,
    pub completion: FormCompletion,
    pub state: String,
    pub progress: Option<ArchiveJobProgress>,
    pub terminal: Option<ArchiveJobTerminal>,
    pub prompt: Option<ArchiveJobPrompt>,
    pub window: Option<(AnyWindowHandle, WeakEntity<JobProgressView>)>,
    pub started: Instant,
    pub paused: bool,
    pub active_operation: Uuid,
    pub created: Option<ArchiveJobTerminal>,
    pub post_test: bool,
    pub edit_form: Option<Uuid>,
    pub edit_origin: Option<(Uuid, u64, Uuid)>,
    connection: Option<u64>,
    terminal_notified: bool,
}
#[derive(Default)]
pub(super) struct Jobs {
    pub forms: HashMap<Uuid, FormSlot>,
    pub edit_forms: HashMap<Uuid, (Uuid, u64, Uuid)>,
    pub records: HashMap<Uuid, JobRecord>,
    queue: JobQueue,
    receipts: HashMap<Uuid, (Uuid, Uuid)>,
    pickers: HashMap<Uuid, PendingPicker>,
    retired: VecDeque<Uuid>,
    defer_pump: bool,
    pub(super) readmissions: HashMap<Uuid, super::readmission::Readmission>,
    pub(super) attempts: HashMap<Uuid, Uuid>,
}
impl ArchiveApplication {
    pub(in crate::archive) fn submit_form(
        &mut self,
        form: Uuid,
        generation: u64,
        request: ArchiveJobRequest,
        completion: FormCompletion,
        cx: &mut Context<Self>,
    ) {
        self.submit_form_admitted(form, generation, request, completion, cx);
    }
    fn remember_accepted(
        &mut self,
        settings: nxtools_core::archive::ArchiveSettings,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self.config_path.clone() else {
            return;
        };
        let owner = cx.entity().downgrade();
        let background = cx.background_executor().clone();
        cx.spawn(async move |_, cx| {
            let result = background
                .spawn(async move {
                    nxtools_core::AppConfig::update_archive_settings(&path, settings)
                        .map(|config| config.archive)
                        .map_err(|_| ())
                })
                .await;
            let _ = owner.update(cx, |app, cx| match result {
                Ok(settings) => app.settings = settings,
                Err(()) => {
                    if let Some(center) = crate::notifications::current(cx) {
                        cx.defer(move |cx| {
                            center.update(cx, |center, cx| {
                                center.push(
                                    crate::notifications::AppNotification::new(
                                        "archive-default-save",
                                        crate::notifications::NotificationLevel::Error,
                                        "默认设置未保存",
                                        "已接受的归档任务继续；请检查配置后重新保存默认设置。",
                                    ),
                                    cx,
                                )
                            })
                        });
                    }
                }
            });
        })
        .detach();
    }
    fn form_error(&self, id: Uuid, message: String, cx: &mut Context<Self>) {
        if let Some(slot) = self.jobs.forms.get(&id) {
            let view = slot.view.clone();
            let window = slot.window;
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, cx| {
                    let _ = view.update(cx, |form, cx| form.fail_submit(message, window, cx));
                });
            });
        }
    }
    fn submit_form_admitted(
        &mut self,
        form: Uuid,
        generation: u64,
        request: ArchiveJobRequest,
        completion: FormCompletion,
        cx: &mut Context<Self>,
    ) {
        if !self
            .jobs
            .forms
            .get(&form)
            .is_some_and(|slot| slot.generation == generation && slot.view.upgrade().is_some())
        {
            return;
        }
        let mut completion = completion;
        let mut requests = vec![request];
        requests.append(&mut completion.additional_requests);
        let validated = self.validate_batch(&requests);
        let result = validated.and_then(|()| {
            self.jobs.defer_pump = true;
            let origin = self.jobs.edit_forms.get(&form).copied();
            let result = (|| {
                for request in requests {
                    let item = FormCompletion {
                        open_after: completion.open_after,
                        keep_open: completion.keep_open,
                        remember: None,
                        additional_requests: vec![],
                    };
                    let id = self.submit_job(request, item, cx)?;
                    if let Some(origin) = origin
                        && let Some(record) = self.jobs.records.get_mut(&id)
                    {
                        record.edit_origin = Some(origin);
                        record.edit_form = Some(form);
                    }
                }
                Ok(())
            })();
            self.jobs.defer_pump = false;
            self.pump_jobs(cx);
            result
        });
        match result {
            Ok(_) => {
                if let Some(settings) = completion.remember.take() {
                    self.remember_accepted(settings, cx);
                }
                if !self.jobs.edit_forms.contains_key(&form)
                    && let Some(slot) = self.jobs.forms.remove(&form)
                {
                    cx.defer(move |cx| {
                        let _ = slot.window.update(cx, |_, window, cx| {
                            let _ = slot.view.update(cx, |form, cx| form.close(window, cx));
                        });
                    });
                }
            }
            Err(message) => self.form_error(form, message, cx),
        }
    }
    pub(super) fn submit_job(
        &mut self,
        request: ArchiveJobRequest,
        completion: FormCompletion,
        cx: &mut Context<Self>,
    ) -> Result<Uuid, String> {
        request
            .validate()
            .map_err(|_| "参数与当前格式能力不一致，请检查来源、目标及选项。".to_owned())?;
        let id = Uuid::new_v4();
        let memory = request
            .limits
            .max_working_bytes
            .checked_add(request.limits.max_metadata_bytes)
            .and_then(|n| n.checked_add(64 << 20))
            .ok_or("内存预算无效")?;
        self.jobs
            .queue
            .enqueue(
                id,
                Resources {
                    sources: request.paths.clone(),
                    output: request.destination.clone(),
                    memory,
                },
            )
            .map_err(|_| "归档队列已满或资源预算不可用；请等待当前任务结束。".to_owned())?;
        self.jobs.records.insert(
            id,
            JobRecord {
                command: ArchiveJobCommand {
                    version: 1,
                    operation_id: id,
                    session_id: id,
                    generation: 1,
                    action: ArchiveJobAction::Start,
                    request: Some(request),
                    reply: None,
                },
                completion,
                state: "排队中".into(),
                progress: None,
                terminal: None,
                prompt: None,
                window: None,
                started: Instant::now(),
                paused: false,
                active_operation: id,
                created: None,
                edit_form: None,
                post_test: false,
                connection: None,
                edit_origin: None,
                terminal_notified: false,
            },
        );
        tracing::info!(event="archive.job.accepted",job_id=%id,queued=self.jobs.queue.queued_count(),"archive job accepted");
        self.show_job(id, cx);
        if !self.jobs.defer_pump {
            self.pump_jobs(cx);
        }
        Ok(id)
    }
    fn validate_batch(&self, requests: &[ArchiveJobRequest]) -> Result<(), String> {
        if !self.jobs.queue.can_fit(requests.len()) {
            return Err("队列容量不足，整批任务未提交。".into());
        }
        for request in requests {
            request
                .validate()
                .map_err(|_| "整批任务中有无效参数，未启动任何任务。".to_owned())?;
            if request
                .limits
                .max_working_bytes
                .saturating_add(request.limits.max_metadata_bytes)
                .saturating_add(64 << 20)
                > 1 << 30
            {
                return Err("任务内存预算超过原生上限。".into());
            }
        }
        Ok(())
    }
    pub(super) fn submit_batch(
        &mut self,
        requests: Vec<ArchiveJobRequest>,
        cx: &mut Context<Self>,
    ) -> Result<Vec<Uuid>, String> {
        self.validate_batch(&requests)?;
        self.jobs.defer_pump = true;
        let mut ids = vec![];
        for request in requests {
            ids.push(self.submit_job(
                request,
                FormCompletion {
                    open_after: false,
                    keep_open: false,
                    remember: None,
                    additional_requests: vec![],
                },
                cx,
            )?);
        }
        self.jobs.defer_pump = false;
        self.pump_jobs(cx);
        Ok(ids)
    }
    pub(super) fn pump_jobs(&mut self, cx: &mut Context<Self>) {
        while let Some(id) = self.jobs.queue.next() {
            let Some(record) = self.jobs.records.get(&id) else {
                continue;
            };
            let command = record.command.clone();
            let extract = command
                .request
                .as_ref()
                .is_some_and(|request| request.kind == ArchiveJobKind::Extract);
            if extract {
                self.start_readmission(id, cx);
            } else {
                self.send_job_attempt(id, command, cx);
            }
        }
    }
    pub(super) fn send_job_attempt(
        &mut self,
        id: Uuid,
        command: ArchiveJobCommand,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self.jobs.records.get_mut(&id) else {
            return;
        };
        record.state = "正在准入".into();
        record.active_operation = command.operation_id;
        let receipt = Uuid::new_v4();
        self.jobs
            .receipts
            .insert(receipt, (id, command.operation_id));
        let connection = record
            .connection
            .or_else(|| self.platform.connection_generation());
        let Some(connection) = connection else {
            self.jobs.receipts.remove(&receipt);
            self.attempt_local_failure(id, local_terminal("disconnected_before_start", true), cx);
            return;
        };
        record.connection = Some(connection);
        self.jobs.attempts.insert(command.operation_id, id);
        if self
            .platform
            .send_archive_job(command, receipt, connection)
            .is_err()
        {
            self.jobs.receipts.remove(&receipt);
            self.attempt_local_failure(id, local_terminal("not_submitted", true), cx);
        }
    }
    fn attempt_local_failure(
        &mut self,
        id: Uuid,
        terminal: ArchiveJobTerminal,
        cx: &mut Context<Self>,
    ) {
        if let Some(runtime) = self.jobs.readmissions.get_mut(&id) {
            runtime.last = Some(terminal.clone());
            if let Some(reply) = runtime.response.take() {
                let _ = reply.send(terminal);
            }
        } else {
            self.finish_attempt(id, terminal, cx);
        }
    }
    pub(crate) fn control_job(
        &mut self,
        id: Uuid,
        action: ArchiveJobAction,
        cx: &mut Context<Self>,
    ) {
        if action == ArchiveJobAction::Cancel && self.jobs.queue.cancel_queued(id) {
            let mut terminal = local_terminal("cancelled", true);
            terminal.outcome = ArchiveJobOutcome::Cancelled;
            self.job_terminal(id, terminal, cx);
            self.pump_jobs(cx);
            return;
        }
        let Some(record) = self
            .jobs
            .records
            .get_mut(&id)
            .filter(|r| r.terminal.is_none())
        else {
            return;
        };
        if action == ArchiveJobAction::Cancel
            && let Some(runtime) = self.jobs.readmissions.get(&id)
        {
            runtime.control.cancel();
        }
        if action == ArchiveJobAction::Cancel {
            record.state = "正在取消，等待原生清理".into();
            record.prompt = None;
        }
        let command = ArchiveJobCommand {
            operation_id: record.active_operation,
            action,
            request: None,
            reply: None,
            ..record.command.clone()
        };
        if let Some(connection) = record.connection {
            let _ = self
                .platform
                .send_archive_job(command, Uuid::new_v4(), connection);
        }
        self.refresh_job(id, cx);
    }
    pub(crate) fn reply_job(
        &mut self,
        id: Uuid,
        prompt: ArchiveJobPrompt,
        reply: ArchiveJobReply,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self
            .jobs
            .records
            .get_mut(&id)
            .filter(|r| r.terminal.is_none() && r.prompt.as_ref() == Some(&prompt))
        else {
            return;
        };
        if reply.ticket != prompt.ticket || reply.kind != prompt.kind {
            return;
        }
        if prompt.kind == ArchiveJobPromptKind::OutputBudget {
            let approved = reply.approved_limit.is_some();
            if self.reply_readmission(id, &prompt, approved) {
                if let Some(record) = self.jobs.records.get_mut(&id) {
                    record.prompt = None;
                }
                self.refresh_job(id, cx);
            }
            return;
        }
        record.prompt = None;
        let command = ArchiveJobCommand {
            action: ArchiveJobAction::Reply,
            operation_id: record.active_operation,
            request: None,
            reply: Some(reply),
            ..record.command.clone()
        };
        let sent = record.connection.is_some_and(|connection| {
            self.platform
                .send_archive_job(command, Uuid::new_v4(), connection)
                .is_ok()
        });
        if !sent {
            self.control_job(id, ArchiveJobAction::Cancel, cx)
        }
        self.refresh_job(id, cx);
    }
    pub(super) fn job_notification(
        &mut self,
        notification: &PlatformNotification,
        cx: &mut Context<Self>,
    ) -> bool {
        match notification {
            PlatformNotification::Event(PlatformEvent::ArchiveJob(event)) => {
                if event.validate().is_err() {
                    return true;
                }
                let id = self
                    .jobs
                    .attempts
                    .get(&event.operation_id)
                    .copied()
                    .unwrap_or(event.operation_id);
                let Some(record) = self.jobs.records.get_mut(&id).filter(|r| {
                    r.command.session_id == event.session_id
                        && r.command.generation == event.generation
                        && r.active_operation == event.operation_id
                        && r.terminal.is_none()
                }) else {
                    return true;
                };
                if event.version != 1 {
                    return true;
                }
                match event.kind {
                    ArchiveJobEventKind::Accepted => record.state = "已准入".into(),
                    ArchiveJobEventKind::Started => {
                        record.state = "运行中".into();
                        tracing::info!(event="archive.job.started",job_id=%id,"archive job started");
                    }
                    ArchiveJobEventKind::State => {
                        if let Some(state) = event.state {
                            record.paused = state == ArchiveJobState::Paused;
                            record.state = state_label(state).into();
                            tracing::info!(event="archive.job.state",job_id=%id,state=?state,"archive job state changed");
                        }
                    }
                    ArchiveJobEventKind::Progress => record.progress = event.progress.clone(),
                    ArchiveJobEventKind::Prompt => {
                        record.prompt = event.prompt.clone();
                        record.state = "等待当前任务确认".into();
                    }
                    ArchiveJobEventKind::Terminal => {
                        if let Some(terminal) = event.terminal.clone() {
                            self.jobs.attempts.remove(&event.operation_id);
                            if let Some(runtime) = self.jobs.readmissions.get_mut(&id) {
                                runtime.last = Some(terminal.clone());
                                if let Some(reply) = runtime.response.take() {
                                    let _ = reply.send(terminal);
                                }
                            } else {
                                self.finish_attempt(id, terminal, cx);
                                self.pump_jobs(cx);
                            }
                            return true;
                        }
                    }
                }
                self.refresh_job(id, cx);
                true
            }
            PlatformNotification::Event(PlatformEvent::ArchivePicker(result)) => {
                if let Some(pending) = self.jobs.pickers.remove(&result.operation_id) {
                    if pending.operation != result.operation_id
                        || pending.form != result.session_id
                        || pending.generation != result.generation
                    {
                        return true;
                    }
                    if let Some(slot) = self
                        .jobs
                        .forms
                        .get(&pending.form)
                        .filter(|s| s.generation == pending.generation)
                    {
                        let view = slot.view.clone();
                        let result = result.clone();
                        let window = slot.window;
                        cx.defer(move |cx| {
                            let _ = window.update(cx, |_, window, cx| {
                                let _ = view
                                    .update(cx, |form, cx| form.picker_result(result, window, cx));
                            });
                        });
                    }
                }
                true
            }
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => {
                if let Some((id, attempt)) = self.jobs.receipts.remove(correlation_id) {
                    if !self.jobs.records.get(&id).is_some_and(|record| {
                        record.terminal.is_none() && record.active_operation == attempt
                    }) {
                        return true;
                    }
                    if !result.success {
                        if matches!(
                            result.error.as_deref(),
                            Some(
                                "busy"
                                    | "invalid_request"
                                    | "incompatible_version"
                                    | "missing_capability"
                            )
                        ) {
                            self.attempt_local_failure(
                                id,
                                local_terminal("native_admission_rejected", true),
                                cx,
                            );
                            self.pump_jobs(cx);
                        } else if let Some(record) = self.jobs.records.get_mut(&id) {
                            record.state =
                                "提交回执未确认：正在取消并等待原生真实结果；槽位保留。".into();
                            self.control_job(id, ArchiveJobAction::Cancel, cx);
                        }
                    }
                    return true;
                }
                false
            }
            PlatformNotification::Disconnected { .. } => {
                self.jobs.pickers.clear();
                let ids = self
                    .jobs
                    .records
                    .iter()
                    .filter(|(_, r)| r.terminal.is_none())
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>();
                for id in ids {
                    if self.jobs.queue.cancel_queued(id) {
                        self.job_terminal(
                            id,
                            local_terminal("disconnected_before_start", true),
                            cx,
                        );
                    } else if let Some(record) = self.jobs.records.get_mut(&id) {
                        if let Some(runtime) = self.jobs.readmissions.get_mut(&id) {
                            runtime.control.cancel();
                            runtime.interaction.close();
                            let mut unknown = local_terminal("disconnected", false);
                            unknown.commit_state = Some("CommitOutcomeUnknown".into());
                            runtime.last = Some(unknown.clone());
                            if let Some(reply) = runtime.response.take() {
                                let _ = reply.send(unknown);
                            }
                        }
                        record.state = "连接断开：输出提交与原生清理状态未知；执行槽保留。".into();
                        record.prompt = None;
                        self.refresh_job(id, cx);
                    }
                }
                false
            }
            _ => false,
        }
    }
    pub(super) fn job_terminal(
        &mut self,
        id: Uuid,
        terminal: ArchiveJobTerminal,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self.jobs.records.get_mut(&id) else {
            return;
        };
        if record.terminal.is_some() {
            return;
        }
        record.prompt = None;
        record.state = outcome_label(terminal.outcome).into();
        if let Some(request) = record.command.request.as_mut()
            && let Some(create) = request.create.as_mut()
        {
            create.password = None;
        }
        let open_directory = (record.completion.open_after
            && terminal.outcome == ArchiveJobOutcome::Succeeded
            && terminal.cleanup_complete
            && terminal.files_committed > 0)
            .then(|| {
                record
                    .command
                    .request
                    .as_ref()
                    .and_then(|request| request.destination.clone())
            })
            .flatten();
        record.terminal = Some(terminal.clone());
        self.jobs.queue.finish(id, terminal.cleanup_complete);
        tracing::info!(event="archive.job.finished",job_id=%id,outcome=?terminal.outcome,files_committed=terminal.files_committed,files_skipped=terminal.files_skipped,elapsed_ms=terminal.elapsed_ms,logical_elapsed_ms=record.started.elapsed().as_millis() as u64,native_error_code=?terminal.native_error_code,cleanup_complete=terminal.cleanup_complete,"archive job terminal");
        let edit_origin = record.edit_origin;
        let edit_form = record.edit_form;
        if (terminal.outcome == ArchiveJobOutcome::Succeeded || terminal.files_committed > 0)
            && terminal.cleanup_complete
            && let Some((window_id, generation, snapshot)) = edit_origin
        {
            self.refresh_modified_browser(window_id, generation, snapshot, cx);
        }
        self.refresh_job(id, cx);
        if let Some(form) = edit_form {
            if terminal.outcome == ArchiveJobOutcome::Succeeded && terminal.cleanup_complete {
                self.jobs.edit_forms.remove(&form);
                if let Some(slot) = self.jobs.forms.remove(&form) {
                    cx.defer(move |cx| {
                        let _ = slot.window.update(cx, |_, window, cx| {
                            if slot
                                .view
                                .read_with(cx, |view, _| !view.retired && view.id == form)
                                .unwrap_or(false)
                            {
                                let _ = slot.view.update(cx, |view, cx| view.close(window, cx));
                            }
                        });
                    });
                }
            } else if terminal.cleanup_complete
                && terminal.files_committed == 0
                && matches!(
                    terminal.commit_state.as_deref(),
                    None | Some("Preparing" | "Verified" | "FailedBeforeCommit")
                )
                && terminal.journal.iter().all(|fact| {
                    fact.published != Some(true)
                        && matches!(
                            fact.state.as_str(),
                            "Preparing" | "Verified" | "FailedBeforeCommit"
                        )
                })
            {
                self.form_error(
                    form,
                    format!(
                        "编辑未完成：{}。原归档保留，草稿可检查后重试。",
                        terminal.code.as_deref().unwrap_or("原生结果未确认")
                    ),
                    cx,
                );
            } else {
                if let Some(slot) = self.jobs.forms.get(&form) {
                    let view = slot.view.clone();
                    let window = slot.window;
                    cx.defer(move |cx| { let _ = window.update(cx, |_, _, cx| {
                        let _ = view.update(cx, |view, cx| view.block_uncertain_retry("提交结果未知或已发布；不可盲目重试，请关闭草稿并刷新核对归档。".into(),cx));
                    }); });
                }
            }
        }
        self.notify_job(id, cx);
        if let Some(target) = open_directory {
            let platform = self.platform.clone();
            cx.defer(move |_| {
                let _ = platform.send_command_correlated(
                    PlatformCommand::OpenShellTarget(nxtools_protocol::OpenShellTargetCommand {
                        target,
                        action: nxtools_protocol::ShellOpenAction::Open,
                    }),
                    id,
                );
            });
        }
        self.jobs.retired.push_back(id);
        while self.jobs.retired.len() > 64 {
            if let Some(old) = self.jobs.retired.pop_front()
                && self
                    .jobs
                    .records
                    .get(&old)
                    .is_some_and(|r| r.window.is_none())
                && !self.jobs.queue.is_running(old)
            {
                self.jobs.records.remove(&old);
            }
        }
    }
    fn notify_job(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(record) = self
            .jobs
            .records
            .get_mut(&id)
            .filter(|r| !r.terminal_notified)
        else {
            return;
        };
        let Some(terminal) = record.terminal.as_ref() else {
            return;
        };
        record.terminal_notified = true;
        let title = outcome_label(terminal.outcome);
        let message = format!(
            "已提交 {} 个文件，跳过 {} 个。{}",
            terminal.files_committed,
            terminal.files_skipped,
            if !terminal.cleanup_complete {
                "原生清理未确认，任务槽仍保留。"
            } else {
                ""
            }
        );
        let level = match terminal.outcome {
            ArchiveJobOutcome::Succeeded => crate::notifications::NotificationLevel::Success,
            ArchiveJobOutcome::Failed | ArchiveJobOutcome::Rejected => {
                crate::notifications::NotificationLevel::Error
            }
            _ => crate::notifications::NotificationLevel::Info,
        };
        let owner = cx.entity().downgrade();
        let notification = crate::notifications::AppNotification::new(
            format!("archive-job-{id}"),
            level,
            title,
            message,
        )
        .actions(vec![crate::notifications::NotificationAction::new(
            "查看结果",
            move |_, cx| {
                let owner = owner.clone();
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| app.show_job(id, cx));
                });
            },
        )]);
        if let Some(center) = crate::notifications::current(cx) {
            cx.defer(move |cx| center.update(cx, |center, cx| center.push(notification, cx)));
        }
    }
    pub(crate) fn pick_for_form(
        &mut self,
        form: Uuid,
        generation: u64,
        purpose: ArchivePickerPurpose,
        location: (Option<String>, Option<String>),
        source: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) -> Option<Uuid> {
        if self
            .jobs
            .forms
            .get(&form)
            .is_none_or(|slot| slot.generation != generation)
            || self.jobs.pickers.values().any(|p| p.form == form)
        {
            return None;
        }
        let (current_directory, suggested_name) = location;
        let Ok(owner_window_handle) = source.update(cx, |_, window, _| {
            crate::ui::native_window_handle(window).unwrap_or(0)
        }) else {
            return None;
        };
        let operation = Uuid::new_v4();
        self.jobs.pickers.insert(
            operation,
            PendingPicker {
                form,
                generation,
                operation,
            },
        );
        let command = ArchivePickerCommand {
            version: 1,
            operation_id: operation,
            session_id: form,
            generation,
            owner_window_handle,
            purpose,
            current_directory,
            suggested_name,
        };
        if self
            .platform
            .send_command_with_id(PlatformCommand::ArchivePicker(command), operation)
            .is_err()
        {
            self.jobs.pickers.remove(&operation);
            self.form_error(form, "选择器不可用，请重试。".into(), cx);
            return None;
        }
        Some(operation)
    }
    pub(crate) fn retire_form(&mut self, id: Uuid) {
        self.jobs.forms.remove(&id);
        self.jobs.pickers.retain(|_, p| p.form != id);
        self.jobs.edit_forms.remove(&id);
    }
    pub(super) fn refresh_job(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(record) = self.jobs.records.get(&id) else {
            return;
        };
        let Some((window, view)) = record.window.clone() else {
            return;
        };
        let snapshot = crate::archive::progress::ProgressSnapshot {
            state: record.state.clone(),
            progress: record.progress.clone(),
            terminal: record.terminal.clone(),
            prompt: record.prompt.clone(),
            paused: record.paused,
        };
        let platform = self.platform.clone();
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                let _ = view.update(cx, |view, cx| view.apply(snapshot, window, cx));
                let _ = platform.send_command_correlated(
                    PlatformCommand::SetTaskbarProgress(crate::archive::progress::taskbar(
                        id,
                        window,
                        view.clone(),
                        cx,
                    )),
                    id,
                );
            });
        });
    }
}
pub(super) fn local_terminal(code: &str, cleanup_complete: bool) -> ArchiveJobTerminal {
    ArchiveJobTerminal {
        outcome: ArchiveJobOutcome::Failed,
        code: Some(code.into()),
        files_committed: 0,
        bytes_committed: 0,
        files_verified: 0,
        bytes_verified: 0,
        files_skipped: 0,
        cleanup_complete,
        commit_state: None,
        elapsed_ms: 0,
        native_error_code: None,
        journal: vec![],
        output_shortfall: None,
    }
}
fn state_label(state: ArchiveJobState) -> &'static str {
    match state {
        ArchiveJobState::Running => "运行中",
        ArchiveJobState::Paused => "已暂停",
        ArchiveJobState::AwaitingPassword => "等待密码",
        ArchiveJobState::AwaitingConflict => "等待同名处理",
        ArchiveJobState::AwaitingRisk => "等待风险确认",
        ArchiveJobState::Verifying => "正在验证",
        ArchiveJobState::Committing => "正在提交",
        ArchiveJobState::Cancelling => "正在取消",
    }
}
fn outcome_label(outcome: ArchiveJobOutcome) -> &'static str {
    match outcome {
        ArchiveJobOutcome::Succeeded => "归档操作完成",
        ArchiveJobOutcome::SucceededWithWarnings => "归档操作完成，有警告",
        ArchiveJobOutcome::Cancelled => "归档操作已取消",
        ArchiveJobOutcome::Failed => "归档操作失败",
        ArchiveJobOutcome::Rejected => "归档操作未获准入",
    }
}

impl ArchiveApplication {
    pub(super) fn show_job(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(record) = self.jobs.records.get(&id) else {
            return;
        };
        if let Some((window, _)) = record.window.clone() {
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, _| window.activate_window());
            });
            return;
        }
        let keep_open = record.completion.keep_open || record.terminal.is_some();
        let owner = cx.entity().downgrade();
        let closing_owner = owner.clone();
        let capture = Rc::new(std::cell::RefCell::new(None));
        let captured = capture.clone();
        let bounds = Bounds::centered(None, size(px(560.0), px(300.0)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(480.0), px(260.0))),
                app_id: Some(format!("NxTools.Archive.Job.{id}")),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title("归档任务 — NxTools");
                let view = cx.new(|cx| JobProgressView::new(id, owner, keep_open, cx));
                *captured.borrow_mut() = Some(view.downgrade());
                window.on_window_should_close(cx, move |_, cx| {
                    let owner = closing_owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |app, cx| app.background_job(id, cx));
                    });
                    false
                });
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            },
        );
        if let (Ok(window), Some(view)) = (opened, capture.borrow_mut().take()) {
            if let Some(record) = self.jobs.records.get_mut(&id) {
                record.window = Some((window.into(), view));
            }
            self.refresh_job(id, cx);
        }
    }
    pub(crate) fn background_job(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(record) = self.jobs.records.get_mut(&id) else {
            return;
        };
        if let Some((window, _)) = record.window.take() {
            let platform = self.platform.clone();
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, _| {
                    let _ = platform.send_command_correlated(
                        PlatformCommand::SetTaskbarProgress(
                            nxtools_protocol::SetTaskbarProgressCommand {
                                window_handle: crate::ui::native_window_handle(window).unwrap_or(0),
                                state: nxtools_protocol::TaskbarProgressState::None,
                                completed: 0,
                                total: 0,
                            },
                        ),
                        id,
                    );
                });
                let _ = window.update(cx, |_, window, _| window.remove_window());
            });
        }
        if self
            .jobs
            .records
            .get(&id)
            .is_some_and(|record| record.terminal.is_none())
        {
            let owner = cx.entity().downgrade();
            if let Some(center) = crate::notifications::current(cx) {
                let notification = crate::notifications::AppNotification::new(
                    format!("archive-background-{id}"),
                    crate::notifications::NotificationLevel::Info,
                    "归档任务已转后台",
                    "关闭进度窗口不会取消任务。",
                )
                .actions(vec![crate::notifications::NotificationAction::new(
                    "恢复进度",
                    move |_, cx| {
                        let owner = owner.clone();
                        cx.defer(move |cx| {
                            let _ = owner.update(cx, |app, cx| app.show_job(id, cx));
                        });
                    },
                )]);
                cx.defer(move |cx| center.update(cx, |center, cx| center.push(notification, cx)));
            }
        }
    }
    pub(in crate::archive) fn open_create_form(
        &mut self,
        sources: Vec<crate::archive::draft::DraftSource>,
        cx: &mut Context<Self>,
    ) {
        self.open_form(Some(sources), None, None, cx);
    }
    pub(in crate::archive) fn open_modify_form(
        &mut self,
        draft: ModifyDraft,
        origin: (Uuid, u64, Uuid),
        cx: &mut Context<Self>,
    ) {
        self.open_form(None, None, Some((draft, origin)), cx);
    }
    pub(in crate::archive) fn open_extract_form(
        &mut self,
        path: PathBuf,
        selection: Option<crate::archive::draft::DraftSelection>,
        cx: &mut Context<Self>,
    ) {
        self.open_form(None, Some((path, selection)), None, cx);
    }
    fn open_form(
        &mut self,
        sources: Option<Vec<crate::archive::draft::DraftSource>>,
        extract: Option<(PathBuf, Option<crate::archive::draft::DraftSelection>)>,
        modify: Option<(ModifyDraft, (Uuid, u64, Uuid))>,
        cx: &mut Context<Self>,
    ) {
        let width = if sources.is_some() { 680.0 } else { 560.0 };
        let settings = self.settings.clone();
        let owner = cx.entity().downgrade();
        let closing_owner = owner.clone();
        let captured = Rc::new(std::cell::RefCell::new(None));
        let capture = captured.clone();
        let modify_origin = modify.as_ref().map(|(_, origin)| *origin);
        let bounds = Bounds::centered(None, size(px(width), px(600.0)), cx);
        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(480.0), px(360.0))),
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title(if sources.is_some() {
                    "新建压缩包 — NxTools"
                } else if modify.is_some() {
                    "编辑 ZIP — NxTools"
                } else {
                    "解压 — NxTools"
                });
                let view = cx.new(|cx| {
                    if let Some(sources) = sources {
                        OperationForm::create(settings, sources, owner, window, cx)
                    } else if let Some((draft, _)) = modify {
                        OperationForm::modify(settings, draft, owner, window, cx)
                    } else {
                        let (path, selection) = extract.expect("extract draft");
                        OperationForm::extract(settings, path, selection, owner, window, cx)
                    }
                });
                let (id, generation) = view.read_with(cx, |form, _| (form.id, form.generation));
                *capture.borrow_mut() = Some((id, generation, view.downgrade()));
                let closing_view = view.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    let _ = closing_view.update(cx, |form, cx| form.retire(window, cx));
                    let owner = closing_owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |app, _| app.retire_form(id));
                    });
                    true
                });
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            },
        );
        if let (Ok(window), Some((id, generation, view))) = (opened, captured.borrow_mut().take()) {
            self.jobs.forms.insert(
                id,
                FormSlot {
                    view,
                    window: window.into(),
                    generation,
                },
            );
            if let Some(origin) = modify_origin {
                self.jobs.edit_forms.insert(id, origin);
            }
        }
    }
}

#[cfg(test)]
#[path = "job_tests.rs"]
mod tests;

impl ArchiveApplication {
    pub(crate) fn extract_browser(
        &mut self,
        id: Uuid,
        generation: u64,
        snapshot: Uuid,
        ordinals: Option<Vec<u64>>,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self
            .windows
            .get(&id)
            .filter(|slot| slot.generation == generation)
        else {
            return;
        };
        let Some(browser) = slot
            .browser
            .clone()
            .filter(|browser| browser.snapshot_id == snapshot)
        else {
            return;
        };
        let path = slot.path.clone();
        let selection =
            ordinals.map(|ordinals| crate::archive::draft::DraftSelection { browser, ordinals });
        self.open_extract_form(path, selection, cx);
    }
    pub(crate) fn test_browser(&mut self, id: Uuid, generation: u64, cx: &mut Context<Self>) {
        let Some(slot) = self
            .windows
            .get(&id)
            .filter(|slot| slot.generation == generation && slot.browser.is_some())
        else {
            return;
        };
        let request = ArchiveJobRequest {
            kind: ArchiveJobKind::Test,
            paths: vec![slot.path.to_string_lossy().into_owned()],
            destination: None,
            codepage: default_codepage(self.settings.filename_encoding),
            limits: Default::default(),
            extract: None,
            create: None,
            readmission: None,
            modify: None,
        };
        let _ = self.submit_job(request, FormCompletion::default(), cx);
    }
}

impl ArchiveApplication {
    pub(crate) fn modify_browser(
        &mut self,
        origin: (Uuid, u64, Uuid),
        action: ArchiveJobModifyAction,
        ordinals: Vec<u64>,
        value: Option<String>,
        codepage: u32,
        cx: &mut Context<Self>,
    ) {
        let (id, generation, snapshot) = origin;
        let Some(slot) = self.windows.get(&id).filter(|s| {
            s.generation == generation
                && s.view
                    .read_with(cx, |view, _| {
                        view.edit_reason().is_none()
                            && view.model.as_ref().is_some_and(|m| m.snapshot == snapshot)
                    })
                    .unwrap_or(false)
        }) else {
            return;
        };
        let Some(browser) = slot
            .browser
            .as_ref()
            .filter(|b| b.snapshot_id == snapshot && b.format == "Zip")
        else {
            return;
        };
        let draft = ModifyDraft {
            source: slot.path.clone(),
            browser: browser.clone(),
            action,
            ordinals,
            value,
            added: Vec::new(),
            codepage,
        };
        self.open_modify_form(draft, (id, generation, snapshot), cx);
    }
    fn refresh_modified_browser(
        &mut self,
        id: Uuid,
        generation: u64,
        snapshot: Uuid,
        cx: &mut Context<Self>,
    ) {
        let Some(slot) = self.windows.get(&id).filter(|s| {
            s.generation == generation
                && s.browser
                    .as_ref()
                    .is_some_and(|b| b.snapshot_id == snapshot)
        }) else {
            return;
        };
        let codepage = slot
            .view
            .read_with(cx, |view, _| view.codepage)
            .unwrap_or(0);
        self.refresh_browser(id, codepage, cx);
    }
}
impl ArchiveApplication {
    pub(crate) fn keep_job_open(&mut self, id: Uuid, value: bool) {
        if let Some(record) = self.jobs.records.get_mut(&id) {
            record.completion.keep_open = value;
        }
    }
}

impl ArchiveApplication {
    fn finish_attempt(
        &mut self,
        id: Uuid,
        mut terminal: ArchiveJobTerminal,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self.jobs.records.get_mut(&id) else {
            return;
        };
        if record.terminal.is_some() {
            return;
        }
        if record.post_test {
            if let Some(created) = record.created.take() {
                terminal.files_committed = created.files_committed;
                terminal.bytes_committed = created.bytes_committed;
                terminal.commit_state = created.commit_state;
                terminal.journal = created.journal;
                terminal.elapsed_ms = terminal.elapsed_ms.saturating_add(created.elapsed_ms);
            }
            self.job_terminal(id, terminal, cx);
            return;
        }
        let test_request = record.command.request.as_ref().and_then(|request| {
            let create = request.create.as_ref()?;
            if !create.test_after_create
                || terminal.outcome != ArchiveJobOutcome::Succeeded
                || !terminal.cleanup_complete
            {
                return None;
            }
            let output_name =
                if create.volume_bytes.is_some() && create.format == ArchiveJobFormat::SevenZ {
                    format!("{}.001", create.name)
                } else {
                    create.name.clone()
                };
            let target = PathBuf::from(request.destination.as_ref()?)
                .join(output_name)
                .to_string_lossy()
                .into_owned();
            Some(ArchiveJobRequest {
                kind: ArchiveJobKind::Test,
                paths: vec![target],
                destination: None,
                codepage: request.codepage,
                limits: request.limits.clone(),
                extract: None,
                create: None,
                readmission: None,
                modify: None,
            })
        });
        if let Some(request) = test_request {
            record.created = Some(terminal);
            record.post_test = true;
            record.state = "归档已提交，正在创建后测试".into();
            record.prompt = None;
            let command = ArchiveJobCommand {
                operation_id: Uuid::new_v4(),
                request: Some(request),
                ..record.command.clone()
            };
            self.send_job_attempt(id, command, cx);
        } else {
            self.job_terminal(id, terminal, cx);
        }
    }
}
