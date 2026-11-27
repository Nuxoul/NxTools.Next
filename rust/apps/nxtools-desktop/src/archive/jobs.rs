//! Application-only FIFO and presentation over the original native worker session.
use super::*;
use crate::archive::{
    draft::ModifyDraft,
    forms::{FormCompletion, OperationForm},
    progress::{JobControls, JobProgressView, StopQuestion},
    queue::{JobQueue, Resources},
};
use nxtools_protocol::{PlatformCommand, PlatformEvent, archive_job::*, archive_picker::*};
use std::collections::VecDeque;
#[path = "batch_jobs.rs"]
mod batch_jobs;
#[path = "job_activation.rs"]
mod job_activation;
#[path = "ordinary_material.rs"]
mod ordinary_material;
use job_activation::{JobActivation, JobPresentation};

pub(super) struct FormSlot {
    pub view: WeakEntity<OperationForm>,
    pub window: AnyWindowHandle,
    pub generation: u64,
}
struct ReclamationContinuation {
    destination: Option<PathBuf>,
}
type ExtractForm = (
    PathBuf,
    Option<crate::archive::draft::DraftSelection>,
    Option<nxtools_core::archive::ArchiveDestination>,
    Option<ReclamationContinuation>,
);
struct PendingPicker {
    form: Uuid,
    generation: u64,
    operation: Uuid,
    started: Instant,
    connection: u64,
    target: Option<(PathBuf, ArchiveBrowserContext, String)>,
}
struct PendingJobControl {
    receipt: Uuid,
    action: ArchiveJobAction,
    attempt: Uuid,
    generation: u64,
    connection: u64,
    accepted: bool,
    started: Instant,
    deadline: Option<Task<()>>,
}
struct StopConfirmation {
    token: Uuid,
    attempt: Uuid,
    generation: u64,
    connection: u64,
    pause_receipt: Option<Uuid>,
    acquired_pause: bool,
    declined: bool,
    started: Instant,
}
pub(super) struct JobRecord {
    pub command: ArchiveJobCommand,
    pub completion: FormCompletion,
    pub state: String,
    pub progress: Option<ArchiveJobProgress>,
    progress_elapsed_ms: u64,
    pub terminal: Option<ArchiveJobTerminal>,
    pub prompt: Option<ArchiveJobPrompt>,
    pub window: Option<(AnyWindowHandle, WeakEntity<JobProgressView>)>,
    pub(super) placement: crate::ui::monitor_placement::MonitorPlacement,
    pub started: Instant,
    pub paused: bool,
    pub active_operation: Uuid,
    pub created: Option<ArchiveJobTerminal>,
    pub post_test: bool,
    pub edit_form: Option<Uuid>,
    pub edit_origin: Option<(Uuid, u64, Uuid)>,
    connection: Option<u64>,
    terminal_notified: bool,
    pub(super) location_status: Option<String>,
    pub(super) location_pending: Option<Uuid>,
    pub(super) location_deadline: Option<Task<()>>,
    pub(super) auto_location_waiting: bool,
    controls: Vec<PendingJobControl>,
    control_status: Option<String>,
    worker_state: Option<ArchiveJobState>,
    stop_confirmation: Option<StopConfirmation>,
    seen_prompts: std::collections::HashSet<Uuid>,
    pub batch: Option<Uuid>,
    pub ever_started: bool,
    pub retry_baseline: Option<nxtools_protocol::archive_space::ArchiveSpaceObservation>,
    pub retry_pending: bool,
}
impl JobRecord {
    fn pause_allowed(&self) -> bool {
        self.connection.is_some()
            && self.terminal.is_none()
            && self.prompt.is_none()
            && matches!(
                self.worker_state,
                Some(ArchiveJobState::Running | ArchiveJobState::Paused)
            )
            && !self.progress.as_ref().is_some_and(|progress| {
                matches!(
                    progress.phase.as_str(),
                    "edit_verifying" | "edit_committing"
                )
            })
            && self.controls.is_empty()
    }
    fn stop_allowed(&self) -> bool {
        self.terminal.is_none()
            && (self.connection.is_some() || self.state == "排队中")
            && self.worker_state != Some(ArchiveJobState::Cancelling)
            && !self
                .controls
                .iter()
                .any(|control| control.action == ArchiveJobAction::Cancel)
    }
    pub(super) fn can_continue_reclamation(&self) -> bool {
        self.ever_started
            && self.terminal.as_ref().is_some_and(|terminal| {
                terminal.cleanup_complete
                    && matches!(
                        terminal.outcome,
                        ArchiveJobOutcome::Cancelled | ArchiveJobOutcome::Failed
                    )
            })
            && self.command.request.as_ref().is_some_and(|request| {
                request.kind == ArchiveJobKind::Extract
                    && request.paths.len() == 1
                    && request
                        .extract
                        .as_ref()
                        .is_some_and(|extract| extract.reclamation.is_some())
            })
    }
}
#[derive(Clone, Copy)]
pub(super) struct EditFormOrigin {
    pub(super) browser: (Uuid, u64, Uuid),
    pub(super) connection: u64,
}
#[derive(Default)]
pub(super) struct Jobs {
    pub forms: HashMap<Uuid, FormSlot>,
    pub edit_forms: HashMap<Uuid, EditFormOrigin>,
    pub records: HashMap<Uuid, JobRecord>,
    queue: JobQueue,
    receipts: HashMap<Uuid, (Uuid, Uuid)>,
    pickers: HashMap<Uuid, PendingPicker>,
    retired: VecDeque<Uuid>,
    defer_pump: bool,
    defer_presentation: bool,
    activations: HashMap<Uuid, JobActivation>,
    material_pending: HashMap<Uuid, AnyWindowHandle>,
    #[cfg(test)]
    activation_test_hwnd: Option<i64>,
    pub(super) readmissions: HashMap<Uuid, super::readmission::Readmission>,
    pub(super) attempts: HashMap<Uuid, Uuid>,
    pub(super) spaces: HashMap<Uuid, super::space_requests::Pending>,
    pub(super) recoveries: HashMap<Uuid, super::recovery_requests::Pending>,
    pub(super) clipboards: HashMap<Uuid, super::clipboard_requests::Pending>,
    pub(super) create_plans: HashMap<Uuid, super::create_output_plan::Pending>,
    pub(super) drag_progress: Option<super::drag_progress::DragPresentation>,
    pub batches: HashMap<Uuid, crate::archive::batch::BatchRecord>,
    batch_order: VecDeque<Uuid>,
    observations: HashMap<Uuid, batch_jobs::JobObservation>,
    confirmation_batches: HashMap<Uuid, Uuid>,
}
impl ArchiveApplication {
    fn restore_prompt_surface(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(record) = self.jobs.records.get(&id).filter(|_| {
            self.jobs
                .records
                .get(&self.job_surface_id(id))
                .is_some_and(|r| r.window.is_none())
        }) else {
            return;
        };
        let Some(prompt) = record.prompt.as_ref() else {
            return;
        };
        let ticket = prompt.ticket.clone();
        let attempt = record.active_operation;
        let generation = record.command.generation;
        let connection = record.connection;
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| {
                if app.jobs.records.get(&id).is_some_and(|record| {
                    record.terminal.is_none()
                        && record.active_operation == attempt
                        && record.command.generation == generation
                        && connection.is_some() && record.connection == connection
                        && app.jobs.records.get(&app.job_surface_id(id)).is_some_and(|r| r.window.is_none())
                        && record.prompt.as_ref().is_some_and(|prompt| prompt.ticket == ticket)
                }) {
                    tracing::info!(event="archive.job.prompt_restored",job_id=%id,%attempt,generation,"current background decision restored");
                    app.show_job(id, cx);
                }
            });
        });
    }
    pub(crate) fn request_job_stop(
        &mut self,
        id: Uuid,
        attempt: Uuid,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self.jobs.records.get(&id).filter(|record| {
            record.active_operation == attempt
                && record.command.generation == generation
                && record.stop_allowed()
                && record.stop_confirmation.is_none()
        }) else {
            return;
        };
        if record.state == "排队中" {
            self.control_job(id, ArchiveJobAction::Cancel, cx);
            return;
        }
        let Some(connection) = record.connection else {
            return;
        };
        let pause = record.pause_allowed() && !record.paused;
        let token = Uuid::new_v4();
        self.jobs
            .records
            .get_mut(&id)
            .expect("current job")
            .stop_confirmation = Some(StopConfirmation {
            token,
            attempt,
            generation,
            connection,
            pause_receipt: None,
            acquired_pause: false,
            declined: false,
            started: Instant::now(),
        });
        tracing::info!(event="archive.job.stop_question",job_id=%id,%attempt,generation,correlation_id=%token,temporary_pause=pause,"stop confirmation accepted");
        if pause {
            self.control_job(id, ArchiveJobAction::Pause, cx);
            let record = self.jobs.records.get_mut(&id).expect("current job");
            let receipt = record
                .controls
                .iter()
                .find(|control| control.action == ArchiveJobAction::Pause)
                .map(|control| control.receipt);
            if let Some(stop) = &mut record.stop_confirmation {
                stop.pause_receipt = receipt;
            }
        }
        self.refresh_job(id, cx);
    }
    pub(crate) fn answer_job_stop(
        &mut self,
        id: Uuid,
        token: Uuid,
        accepted: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self
            .jobs
            .records
            .get_mut(&id)
            .filter(|record| record.terminal.is_none())
        else {
            return;
        };
        let Some(stop) = record.stop_confirmation.as_mut().filter(|stop| {
            stop.token == token
                && stop.attempt == record.active_operation
                && stop.generation == record.command.generation
                && Some(stop.connection) == record.connection
                && !stop.declined
        }) else {
            return;
        };
        if accepted && stop.pause_receipt.is_some() {
            return;
        }
        tracing::info!(event="archive.job.stop_answered",job_id=%id,attempt=%stop.attempt,generation=stop.generation,correlation_id=%token,accepted,elapsed_ms=stop.started.elapsed().as_millis(),"stop confirmation answered");
        if accepted {
            record.stop_confirmation = None;
            self.control_job(id, ArchiveJobAction::Cancel, cx);
        } else {
            stop.declined = true;
            self.restore_declined_stop(id, cx);
            self.refresh_job(id, cx);
        }
    }
    fn restore_declined_stop(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let Some(record) = self.jobs.records.get(&id) else {
            return;
        };
        let Some(stop) = record
            .stop_confirmation
            .as_ref()
            .filter(|stop| stop.declined && stop.pause_receipt.is_none())
        else {
            return;
        };
        let restore = stop.acquired_pause
            && record.paused
            && record.terminal.is_none()
            && stop.attempt == record.active_operation
            && stop.generation == record.command.generation
            && Some(stop.connection) == record.connection
            && !record
                .progress
                .as_ref()
                .is_some_and(|p| matches!(p.phase.as_str(), "edit_verifying" | "edit_committing"))
            && record.worker_state == Some(ArchiveJobState::Paused);
        if let Some(record) = self.jobs.records.get_mut(&id) {
            record.stop_confirmation = None;
        }
        if restore {
            self.control_job_admitted(id, ArchiveJobAction::Resume, true, cx);
        }
        self.refresh_job(id, cx);
    }
    pub(in crate::archive) fn submit_form(
        &mut self,
        form: Uuid,
        generation: u64,
        request: ArchiveJobRequest,
        completion: FormCompletion,
        cx: &mut Context<Self>,
    ) {
        let Some((request, completion)) =
            self.plan_create_outputs(form, generation, request, completion, cx)
        else {
            return;
        };
        self.submit_form_admitted(form, generation, request, completion, cx);
    }
    fn remember_accepted(
        &mut self,
        settings: nxtools_core::archive::ArchiveSettings,
        kind: ArchiveJobKind,
        baseline: Option<nxtools_core::archive::ArchiveSettings>,
        cx: &mut Context<Self>,
    ) {
        let Some(path) = self.config_path.clone() else {
            return;
        };
        let owner = cx.entity().downgrade();
        let background = cx.background_executor().clone();
        #[cfg(test)]
        let publication_barrier = self.remember_publication_barrier.take();
        let correlation_id = Uuid::new_v4();
        let started = Instant::now();
        tracing::info!(event="archive.preferences.save_started", %correlation_id, "archive preferences save accepted");
        cx.spawn(async move |_, cx| {
            let result = background
                .spawn(async move {
                    super::save_settings(&path, |current| match kind {
                        ArchiveJobKind::Create => {
                            if let Some(baseline) = &baseline {
                                current.merge_create_preferences(baseline, &settings);
                            } else {
                                let previous = current.clone();
                                current.merge_create_preferences(&previous, &settings);
                            }
                        }
                        ArchiveJobKind::Extract => {
                            if let Some(baseline) = &baseline {
                                current.merge_extract_preferences(baseline, &settings);
                            } else {
                                let previous = current.clone();
                                current.merge_extract_preferences(&previous, &settings);
                            }
                        }
                        _ => {}
                    })
                    .map_err(|_| ())
                })
                .await;
            #[cfg(test)]
            if let Some(barrier) = publication_barrier {
                let _ = barrier.recv().await;
            }
            tracing::info!(event="archive.preferences.save_finished", %correlation_id, success=result.is_ok(), elapsed_ms=started.elapsed().as_millis() as u64, "archive preferences save finished");
            let _ = owner.update(cx, |app, cx| match result {
                Ok(saved) => app.publish_saved_settings(saved, cx),
                Err(()) => {
                    if let Some(center) = crate::notifications::current(cx) {
                        cx.defer(move |cx| {
                            center.update(cx, |center, cx| {
                                center.push(
                                    crate::notifications::AppNotification::new(
                                        "archive-default-save",
                                        crate::notifications::NotificationLevel::Error,
                                        "默认设置未保存",
                                        "已接受的压缩包任务继续；请检查配置后重新保存默认设置。",
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
    pub(super) fn submit_form_admitted(
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
        if let Some(selection) = request
            .extract
            .as_ref()
            .and_then(|extract| extract.selection.as_ref())
        {
            let current = self.windows.values().any(|slot| {
                request
                    .paths
                    .first()
                    .is_some_and(|path| std::path::Path::new(path) == slot.path.as_path())
                    && slot.browser.as_ref() == Some(&selection.browser)
                    && slot
                        .view
                        .read_with(cx, |view, _| {
                            view.generation == slot.generation
                                && matches!(view.status, BrowserStatus::Ready)
                                && view.model.as_ref().is_some_and(|model| {
                                    model.snapshot == selection.browser.snapshot_id
                                })
                                && self.platform.connection_generation()
                                    == Some(view.preview_connection())
                        })
                        .unwrap_or(false)
            });
            if !current {
                self.form_error(
                    form,
                    "源已变化或选定快照已失效；请刷新后重新选择解压项目。".into(),
                    cx,
                );
                return;
            }
        }
        // Every edit form is presentation, not authority after refresh/source failure.
        if request.modify.is_some() {
            let current = self
                .jobs
                .edit_forms
                .get(&form)
                .is_some_and(|origin| self.edit_request_current(&request, *origin, cx));
            if !current {
                self.form_error(
                    form,
                    "源已变化或旧快照不能编辑；请刷新后重新打开编辑表单。".into(),
                    cx,
                );
                return;
            }
        }
        let mut completion = completion;
        let mut requests = vec![request];
        let kind = requests[0].kind;
        requests.append(&mut completion.additional_requests);
        let validated = self.validate_batch(&requests);
        let result = validated.and_then(|()| {
            self.jobs.defer_pump = true;
            self.jobs.defer_presentation = true;
            let origin = self.jobs.edit_forms.get(&form).copied();
            let requested = requests.len();
            let mut batch_members = Vec::new();
            let result = (|| {
                for request in requests {
                    let item = FormCompletion {
                        open_after: completion.open_after,
                        keep_open: completion.keep_open,
                        remember: None,
                        remember_baseline: None,
                        additional_requests: vec![],
                    };
                    let id = self.submit_job(request, item, cx)?;
                    batch_members.push(id);
                    if let Some(origin) = origin
                        && let Some(record) = self.jobs.records.get_mut(&id)
                    {
                        record.edit_origin = Some(origin.browser);
                        record.edit_form = Some(form);
                    }
                }
                Ok(())
            })();
            if let Some(batch) = self.jobs.confirmation_batches.remove(&form) {
                self.attach_confirmation_jobs(batch, form, &batch_members, cx);
            } else if requested > 1 && !batch_members.is_empty() {
                self.bind_batch(
                    batch_members.clone(),
                    result
                        .as_ref()
                        .err()
                        .map(|message: &String| (requested, message.clone())),
                    cx,
                );
            }
            self.jobs.defer_pump = false;
            self.jobs.defer_presentation = false;
            if let Some(id) = batch_members.first() {
                self.show_job(*id, cx);
            }
            self.pump_jobs(cx);
            result
        });
        match result {
            Ok(_) => {
                self.retire_form_space(form);
                if let Some(settings) = completion.remember.take() {
                    self.remember_accepted(settings, kind, completion.remember_baseline.take(), cx);
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
    pub(super) fn edit_request_current(
        &self,
        request: &ArchiveJobRequest,
        origin: EditFormOrigin,
        cx: &App,
    ) -> bool {
        let Some(modify) = request.modify.as_ref() else {
            return false;
        };
        let (id, generation, snapshot) = origin.browser;
        self.platform.connection_generation() == Some(origin.connection)
            && self.windows.get(&id).is_some_and(|slot| {
                slot.generation == generation
                    && request
                        .paths
                        .first()
                        .is_some_and(|path| std::path::Path::new(path) == slot.path.as_path())
                    && slot.browser.as_ref() == Some(&modify.browser)
                    && modify.browser.snapshot_id == snapshot
                    && slot
                        .view
                        .read_with(cx, |view, _| {
                            view.generation == generation
                                && view.preview_connection() == origin.connection
                                && view.operation_refusal(modify.operation()).is_none()
                                && view.edit_members_current(
                                    modify.action,
                                    &modify.ordinals,
                                    modify.value.as_deref(),
                                    modify.source_prefix.as_deref(),
                                )
                                && view.model.as_ref().is_some_and(|model| {
                                    model.snapshot == snapshot
                                        && (modify.action != ArchiveJobModifyAction::Add
                                            || model.folder
                                                == modify
                                                    .value
                                                    .as_deref()
                                                    .unwrap_or("")
                                                    .trim_end_matches('/'))
                                })
                                && view.edit_source.as_ref() == Some(&modify.browser.source)
                        })
                        .unwrap_or(false)
            })
    }

    pub(super) fn submit_drop_add(
        &mut self,
        request: ArchiveJobRequest,
        origin: EditFormOrigin,
        cx: &mut Context<Self>,
    ) -> Result<Uuid, String> {
        self.validate_batch(std::slice::from_ref(&request))?;
        self.jobs.defer_pump = true;
        self.jobs.defer_presentation = true;
        let result = self.submit_job(request, FormCompletion::default(), cx);
        if let Ok(id) = result.as_ref()
            && let Some(record) = self.jobs.records.get_mut(id)
        {
            record.edit_origin = Some(origin.browser);
            record.connection = Some(origin.connection);
        }
        self.jobs.defer_pump = false;
        self.jobs.defer_presentation = false;
        if let Ok(id) = result.as_ref() {
            self.show_job(*id, cx);
            self.pump_jobs(cx);
        }
        result
    }

    pub(super) fn submit_job(
        &mut self,
        request: ArchiveJobRequest,
        completion: FormCompletion,
        cx: &mut Context<Self>,
    ) -> Result<Uuid, String> {
        self.submit_job_with_id(Uuid::new_v4(), request, completion, cx)
    }

    pub(super) fn submit_job_with_id(
        &mut self,
        id: Uuid,
        request: ArchiveJobRequest,
        completion: FormCompletion,
        cx: &mut Context<Self>,
    ) -> Result<Uuid, String> {
        if id.is_nil() || self.jobs.records.contains_key(&id) {
            return Err("压缩包任务标识已使用，请重新发起操作。".into());
        }
        request
            .validate()
            .map_err(|_| "参数与当前格式能力不一致，请检查来源、目标及选项。".to_owned())?;
        let reclamation = request
            .extract
            .as_ref()
            .and_then(|extract| extract.reclamation);
        let reclamation_source = reclamation.and_then(|_| {
            request
                .paths
                .first()
                .map(|path| PathBuf::from(path.as_str()))
        });
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
            .map_err(|_| "压缩包队列已满或资源预算不可用；请等待当前任务结束。".to_owned())?;
        let placement = crate::ui::monitor_placement::active_monitor_placement(cx);
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
                    batch_id: None,
                },
                completion,
                state: "排队中".into(),
                progress: None,
                progress_elapsed_ms: 0,
                terminal: None,
                prompt: None,
                window: None,
                placement,
                started: cx.background_executor().now(),
                paused: false,
                active_operation: id,
                created: None,
                edit_form: None,
                post_test: false,
                connection: None,
                edit_origin: None,
                terminal_notified: false,
                location_status: None,
                location_pending: None,
                location_deadline: None,
                auto_location_waiting: false,
                controls: Vec::new(),
                control_status: None,
                worker_state: None,
                stop_confirmation: None,
                seen_prompts: Default::default(),
                batch: None,
                ever_started: false,
                retry_baseline: None,
                retry_pending: false,
            },
        );
        if let Some(source) = reclamation_source {
            self.retire_reclamation_source(&source, cx);
        }
        if let Some(intent) = reclamation {
            tracing::info!(event="archive.reclamation.accepted",job_id=%id,recovery_id=%intent.recovery_id,mode=?intent.mode,queued=self.jobs.queue.queued_count(),"archive reclamation job accepted");
        } else {
            tracing::info!(event="archive.job.accepted",job_id=%id,queued=self.jobs.queue.queued_count(),"archive job accepted");
        }
        if !self.jobs.defer_presentation {
            self.show_job(id, cx);
        }
        if !self.jobs.defer_pump {
            self.pump_jobs(cx);
        }
        Ok(id)
    }
    fn retire_reclamation_source(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        let Some(browser_id) = self.browser_id(path) else {
            return;
        };
        self.cancel_browser(browser_id, cx);
        let Some(slot) = self.windows.get_mut(&browser_id) else {
            return;
        };
        slot.browser = None;
        slot.generation = slot.generation.wrapping_add(1).max(1);
        let generation = slot.generation;
        let view = slot.view.clone();
        cx.defer(move |cx| {
            let _ = view.update(cx, |view, cx| {
                if view.generation > generation {
                    return;
                }
                view.generation = generation;
                view.read_id = None;
                view.open_pending = false;
                view.invalidate_drag();
                view.edit_source = None;
                view.status = BrowserStatus::Stale(
                    "空间回收解压已获准入；旧浏览快照已停用，不能在原件数据变化后继续使用。".into(),
                );
                cx.notify();
            });
        });
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
        self.jobs.defer_presentation = true;
        let mut ids = vec![];
        let requested = requests.len();
        let result = (|| {
            for request in requests {
                let extract = request.kind == ArchiveJobKind::Extract;
                ids.push(self.submit_job(
                    request,
                    FormCompletion {
                        open_after: extract && self.settings.open_after_extract,
                        keep_open: extract && self.settings.keep_extract_progress_open,
                        remember: None,
                        remember_baseline: None,
                        additional_requests: vec![],
                    },
                    cx,
                )?);
            }
            Ok(())
        })();
        if requested > 1 {
            self.bind_batch(
                ids.clone(),
                result
                    .as_ref()
                    .err()
                    .map(|message: &String| (requested, message.clone())),
                cx,
            );
        }
        self.jobs.defer_pump = false;
        self.jobs.defer_presentation = false;
        if let Some(id) = ids.first() {
            self.show_job(*id, cx);
        }
        self.pump_jobs(cx);
        result.map(|()| ids)
    }
    pub(super) fn pump_jobs(&mut self, cx: &mut Context<Self>) {
        while let Some(id) = self.jobs.queue.next() {
            let Some(record) = self.jobs.records.get(&id) else {
                continue;
            };
            let command = record.command.clone();
            let ordinary_extract = command.request.as_ref().is_some_and(|request| {
                request.kind == ArchiveJobKind::Extract
                    && request
                        .extract
                        .as_ref()
                        .is_none_or(|extract| extract.reclamation.is_none())
            });
            if ordinary_extract {
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
        let stale_drop = self.jobs.records.get(&id).is_some_and(|record| {
            let Some(browser) = record.edit_origin.filter(|_| record.edit_form.is_none()) else {
                return false;
            };
            let Some(request) = command.request.as_ref().filter(|request| {
                request
                    .modify
                    .as_ref()
                    .is_some_and(|modify| modify.action == ArchiveJobModifyAction::Add)
            }) else {
                return false;
            };
            record.connection.is_none_or(|connection| {
                !self.tab_is_active(browser.0, cx)
                    || !self.edit_request_current(
                        request,
                        EditFormOrigin {
                            browser,
                            connection,
                        },
                        cx,
                    )
            })
        });
        if stale_drop {
            self.attempt_local_failure(id, local_terminal("stale_add_target", true), cx);
            return;
        }
        self.retire_job_observations(id); // Old-attempt observations cannot cross dispatch.
        let Some(record) = self.jobs.records.get_mut(&id) else {
            return;
        };
        record.controls.clear();
        record.control_status = None;
        record.worker_state = None;
        record.stop_confirmation = None;
        record.seen_prompts.clear();
        record.state = "正在准入".into();
        record.active_operation = command.operation_id;
        record.progress = None;
        record.prompt = None;
        record.paused = false;
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
        } else {
            let owner = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = owner.update(cx, |app, cx| app.observe_job(id, false, cx));
            });
        }
    }
    fn attempt_local_failure(
        &mut self,
        id: Uuid,
        terminal: ArchiveJobTerminal,
        cx: &mut Context<Self>,
    ) {
        self.retire_job_observations(id);
        if let Some(record) = self.jobs.records.get(&id) {
            self.jobs.attempts.remove(&record.active_operation);
        }
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
        self.control_job_admitted(id, action, false, cx);
    }
    fn control_job_admitted(
        &mut self,
        id: Uuid,
        action: ArchiveJobAction,
        restoring_pause: bool,
        cx: &mut Context<Self>,
    ) {
        if !matches!(
            action,
            ArchiveJobAction::Pause | ArchiveJobAction::Resume | ArchiveJobAction::Cancel
        ) {
            return;
        }
        if action == ArchiveJobAction::Cancel && self.jobs.queue.cancel_queued(id) {
            let mut terminal = local_terminal("cancelled", true);
            terminal.outcome = ArchiveJobOutcome::Cancelled;
            self.job_terminal(id, terminal, cx);
            self.pump_jobs(cx);
            return;
        }
        if action == ArchiveJobAction::Cancel
            && let Some(runtime) = self.jobs.readmissions.get(&id)
        {
            runtime.control.cancel();
        }
        let Some(record) = self
            .jobs
            .records
            .get_mut(&id)
            .filter(|record| record.terminal.is_none())
        else {
            return;
        };
        let restore_allowed = restoring_pause
            && action == ArchiveJobAction::Resume
            && record.paused
            && record.worker_state == Some(ArchiveJobState::Paused)
            && record.controls.is_empty()
            && !record
                .progress
                .as_ref()
                .is_some_and(|p| matches!(p.phase.as_str(), "edit_verifying" | "edit_committing"));
        if !record.stop_allowed()
            || action != ArchiveJobAction::Cancel
                && (!(record.pause_allowed() || restore_allowed)
                    || (action == ArchiveJobAction::Pause) == record.paused)
        {
            self.refresh_job(id, cx);
            return;
        }
        if record.controls.iter().any(|pending| {
            pending.action == ArchiveJobAction::Cancel || action != ArchiveJobAction::Cancel
        }) {
            return;
        }
        let receipt = Uuid::new_v4();
        let Some(connection) = record.connection else {
            record.control_status = Some("控制请求未提交：连接不可用，请重试。".into());
            self.refresh_job(id, cx);
            return;
        };
        let command = ArchiveJobCommand {
            operation_id: record.active_operation,
            action,
            request: None,
            reply: None,
            ..record.command.clone()
        };
        record.control_status = None;
        record.controls.push(PendingJobControl {
            receipt,
            action,
            attempt: record.active_operation,
            generation: record.command.generation,
            connection,
            accepted: false,
            started: Instant::now(),
            deadline: None,
        });
        tracing::info!(event="archive.job.control_requested", job_id=%id,correlation_id=%receipt,attempt=%command.operation_id,generation=command.generation,?action,"archive job control requested");
        let receipt = match self.platform.send_archive_job(command, receipt, connection) {
            Ok(actual_receipt) => {
                if actual_receipt != receipt
                    && let Some(pending) = self.jobs.records.get_mut(&id).and_then(|record| {
                        record
                            .controls
                            .iter_mut()
                            .find(|pending| pending.receipt == receipt)
                    })
                {
                    pending.receipt = actual_receipt;
                }
                actual_receipt
            }
            Err(_) => {
                self.finish_job_control(id, receipt, "not_submitted", None, cx);
                return;
            }
        };
        if action == ArchiveJobAction::Cancel
            && let Some(record) = self.jobs.records.get_mut(&id)
        {
            record.prompt = None;
        }
        self.arm_job_control(id, receipt, false, cx);
        self.refresh_job(id, cx);
    }
    fn arm_job_control(&mut self, id: Uuid, receipt: Uuid, accepted: bool, cx: &mut Context<Self>) {
        let timeout = if accepted { 10 } else { 5 };
        let deadline = cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(timeout))
                .await;
            let _ = owner.update(cx, |app, cx| {
                app.finish_job_control(
                    id,
                    receipt,
                    if accepted {
                        "state_timeout"
                    } else {
                        "receipt_timeout"
                    },
                    None,
                    cx,
                );
            });
        });
        if let Some(pending) = self.jobs.records.get_mut(&id).and_then(|record| {
            record
                .controls
                .iter_mut()
                .find(|pending| pending.receipt == receipt)
        }) {
            pending.deadline = Some(deadline);
        }
    }
    fn finish_job_control(
        &mut self,
        id: Uuid,
        receipt: Uuid,
        reason: &'static str,
        native_error: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        let Some(record) = self.jobs.records.get_mut(&id) else {
            return;
        };
        let Some(index) = record
            .controls
            .iter()
            .position(|pending| pending.receipt == receipt)
        else {
            return;
        };
        let pending = record.controls.remove(index);
        if record.active_operation != pending.attempt
            || record.command.generation != pending.generation
            || record.connection != Some(pending.connection)
        {
            return;
        }
        if let Some(stop) = &mut record.stop_confirmation
            && stop.pause_receipt == Some(receipt)
        {
            stop.pause_receipt = None;
            stop.acquired_pause = reason == "observed" && record.paused;
        }
        record.control_status = match reason {
            "observed" | "terminal" => None,
            "not_submitted" => Some("控制请求未提交：队列繁忙或连接不可用，请重试。".into()),
            "native_refused" => Some("原生未接受控制请求；任务实际状态未改变，可以重试。".into()),
            "disconnected" => Some("连接断开，控制结果未知；等待连接恢复后核对任务状态。".into()),
            "receipt_timeout" => {
                Some("控制回执超时，结果未知；请核对实际状态后重试，不会自动重复发送。".into())
            }
            _ => {
                Some("控制已接受但实际状态未确认；请核对实际状态后重试，不会自动重复发送。".into())
            }
        };
        tracing::info!(event="archive.job.control_finished",job_id=%id,correlation_id=%receipt,attempt=%pending.attempt,generation=pending.generation,action=?pending.action,reason,native_error_code=?native_error,elapsed_ms=pending.started.elapsed().as_millis(),"archive control UI wait retired; native slot unchanged");
        self.refresh_job(id, cx);
        self.restore_declined_stop(id, cx);
    }
    fn job_control_result(
        &mut self,
        receipt: Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(id) = self
            .jobs
            .records
            .iter()
            .find(|(_, record)| {
                record
                    .controls
                    .iter()
                    .any(|pending| pending.receipt == receipt)
            })
            .map(|(id, _)| *id)
        else {
            return false;
        };
        if !result.success {
            self.finish_job_control(id, receipt, "native_refused", result.native_error_code, cx);
            return true;
        }
        let record = self
            .jobs
            .records
            .get_mut(&id)
            .expect("matched control owner");
        let pending = record
            .controls
            .iter_mut()
            .find(|pending| pending.receipt == receipt)
            .expect("matched control receipt");
        if record.active_operation != pending.attempt
            || record.command.generation != pending.generation
            || record.connection != Some(pending.connection)
        {
            return true;
        }
        if !pending.accepted {
            pending.accepted = true;
            record.control_status = Some("控制请求已接受；仍在等待任务报告真实状态。".into());
            tracing::info!(event="archive.job.control_accepted",job_id=%id,correlation_id=%receipt,attempt=%pending.attempt,generation=pending.generation,action=?pending.action,"control acknowledgement is not worker state");
            let stop_paused = pending.action == ArchiveJobAction::Pause
                && record.paused
                && record
                    .stop_confirmation
                    .as_ref()
                    .is_some_and(|stop| stop.pause_receipt == Some(receipt));
            if stop_paused {
                self.finish_job_control(id, receipt, "observed", None, cx);
            } else {
                self.arm_job_control(id, receipt, true, cx);
                self.refresh_job(id, cx);
            }
        }
        true
    }
    pub(crate) fn reply_job(
        &mut self,
        id: Uuid,
        prompt: ArchiveJobPrompt,
        reply: ArchiveJobReply,
        cx: &mut Context<Self>,
    ) {
        let password_reusable = self.batch_password_eligible(id);
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
        if reply.remember_batch_password
            && (!password_reusable
                || reply.kind != ArchiveJobPromptKind::Password
                || reply.password.is_none())
        {
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
                        && (r.connection.is_some() || event.kind == ArchiveJobEventKind::Terminal)
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
                        record.worker_state = Some(ArchiveJobState::Running);
                        record.ever_started = true;
                        tracing::info!(event="archive.job.started",job_id=%id,"archive job started");
                    }
                    ArchiveJobEventKind::State => {
                        if let Some(state) = event.state {
                            record.paused = state == ArchiveJobState::Paused;
                            record.worker_state = Some(state);
                            if matches!(
                                state,
                                ArchiveJobState::Committing | ArchiveJobState::Cancelling
                            ) {
                                record.stop_confirmation = None;
                            }
                            record.state = state_label(state).into();
                            tracing::info!(event="archive.job.state",job_id=%id,state=?state,"archive job state changed");
                        }
                    }
                    ArchiveJobEventKind::Progress => {
                        if let Some(next) = &event.progress {
                            if record.progress.as_ref().is_some_and(|old| {
                                match (old.sequence, next.sequence) {
                                    (Some(old), Some(next)) => next <= old,
                                    (Some(_), None) => true,
                                    (None, Some(_)) => false,
                                    (None, None) => {
                                        old.phase == next.phase
                                            && (next.bytes_done < old.bytes_done
                                                || next.files_done < old.files_done)
                                    }
                                }
                            }) {
                                return true;
                            }
                            if next.phase == "edit_committing"
                                && record
                                    .progress
                                    .as_ref()
                                    .is_none_or(|old| old.phase != next.phase)
                            {
                                record.stop_confirmation = None;
                            }
                            if record
                                .progress
                                .as_ref()
                                .is_none_or(|old| old.phase != next.phase)
                            {
                                tracing::info!(event="archive.job.phase",job_id=%id,attempt=%event.operation_id,generation=event.generation,phase=next.phase.as_str(),"archive job phase changed");
                            }
                            record.progress = Some(next.clone());
                            record.progress_elapsed_ms =
                                cx.background_executor()
                                    .now()
                                    .saturating_duration_since(record.started)
                                    .as_millis()
                                    .min(u64::MAX as u128) as u64;
                        }
                    }
                    ArchiveJobEventKind::Prompt => {
                        let Some(prompt) = &event.prompt else {
                            return true;
                        };
                        if !record.seen_prompts.insert(prompt.ticket.prompt_id) {
                            return true;
                        }
                        if let Some(stop) = &mut record.stop_confirmation {
                            stop.declined = true;
                        }
                        record.prompt = event.prompt.clone();
                        record.state = "等待当前任务确认".into();
                    }
                    ArchiveJobEventKind::Terminal => {
                        record.stop_confirmation = None;
                        if let Some(terminal) = event.terminal.clone() {
                            let pending = record
                                .controls
                                .iter()
                                .map(|control| control.receipt)
                                .collect::<Vec<_>>();
                            // Close first-baseline acquisition at native terminal, before logical Finished.
                            self.retire_job_observations(id);
                            for receipt in pending {
                                self.finish_job_control(id, receipt, "terminal", None, cx);
                            }
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
                let observed = self.jobs.records[&id]
                    .controls
                    .iter()
                    .filter(|control| {
                        event.kind == ArchiveJobEventKind::State
                            && (control.accepted
                                || !self.jobs.records[&id]
                                    .stop_confirmation
                                    .as_ref()
                                    .is_some_and(|stop| {
                                        stop.pause_receipt == Some(control.receipt)
                                    }))
                            && matches!(
                                (control.action, event.state),
                                (ArchiveJobAction::Pause, Some(ArchiveJobState::Paused))
                                    | (ArchiveJobAction::Resume, Some(ArchiveJobState::Running))
                                    | (ArchiveJobAction::Cancel, Some(ArchiveJobState::Cancelling))
                            )
                    })
                    .map(|control| control.receipt)
                    .collect::<Vec<_>>();
                for receipt in observed {
                    self.finish_job_control(id, receipt, "observed", None, cx);
                }
                if event.kind == ArchiveJobEventKind::Prompt {
                    self.restore_declined_stop(id, cx);
                    self.restore_prompt_surface(id, cx);
                }
                self.refresh_job(id, cx);
                true
            }
            PlatformNotification::Event(PlatformEvent::ArchivePicker(result)) => {
                if self
                    .jobs
                    .pickers
                    .get(&result.operation_id)
                    .is_some_and(|pending| {
                        pending.operation == result.operation_id
                            && pending.form == result.session_id
                            && pending.generation == result.generation
                    })
                {
                    let pending = self
                        .jobs
                        .pickers
                        .remove(&result.operation_id)
                        .expect("matching picker");
                    tracing::info!(event="archive.picker.finished",correlation_id=%pending.operation,generation=pending.generation,cancelled=result.cancelled,success=!result.cancelled && result.code.is_none(),native_error_code=?result.native_error_code,elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive picker terminal");
                    if let Some(slot) = self
                        .jobs
                        .forms
                        .get(&pending.form)
                        .filter(|s| s.generation == pending.generation)
                    {
                        let view = slot.view.clone();
                        let result = result.clone();
                        let window = slot.window;
                        let owner = cx.entity().downgrade();
                        cx.defer(move |cx| {
                            let current = owner
                                .read_with(cx, |app, cx| {
                                    app.platform.connection_generation() == Some(pending.connection)
                                        && (pending.target.is_none()
                                            || app.intake_target_current(
                                                pending.form,
                                                &pending.target,
                                                cx,
                                            ))
                                })
                                .unwrap_or(false);
                            let _ = window.update(cx, |_, window, cx| {
                                let _ = view.update(cx, |form, cx| {
                                    if current {
                                        form.picker_result(result, window, cx);
                                    } else {
                                        form.picker_receipt_failure(
                                            pending.operation,
                                            "连接或原 Add 目标已变化；未添加任何来源。",
                                            cx,
                                        );
                                    }
                                });
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
                if self.job_activation_result(*correlation_id, result, cx) {
                    return true;
                }
                if self.create_output_result(*correlation_id, result, cx) {
                    return true;
                }
                if self.clipboard_result(*correlation_id, result, cx) {
                    return true;
                }
                if self.jobs.pickers.contains_key(correlation_id) {
                    if result.success {
                        return true;
                    }
                    let pending = self
                        .jobs
                        .pickers
                        .remove(correlation_id)
                        .expect("matching picker receipt");
                    tracing::info!(event="archive.picker.finished",correlation_id=%pending.operation,generation=pending.generation,success=false,reason=if result.error.as_deref()==Some("platform command timed out") {"timeout"} else {"native_rejected"},native_error_code=?result.native_error_code,elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive picker receipt failed");
                    if let Some(slot) = self
                        .jobs
                        .forms
                        .get(&pending.form)
                        .filter(|slot| slot.generation == pending.generation)
                    {
                        let view = slot.view.clone();
                        let operation = pending.operation;
                        let message =
                            if result.error.as_deref() == Some("platform command timed out") {
                                "选择器等待超时，请重试。"
                            } else {
                                "选择器请求被拒绝，请重试。"
                            };
                        cx.defer(move |cx| {
                            let _ = view.update(cx, |form, cx| {
                                form.picker_receipt_failure(operation, message, cx)
                            });
                        });
                    }
                    return true;
                }
                if self.job_control_result(*correlation_id, result, cx) {
                    return true;
                }
                if self.drag_presentation_result(*correlation_id, result, cx)
                    || self.job_location_result(*correlation_id, result, cx)
                {
                    return true;
                }
                if self.job_observation_result(*correlation_id, result, cx) {
                    return true;
                }
                if self.recovery_result(*correlation_id, result, cx)
                    || self.space_result(*correlation_id, result, cx)
                {
                    return true;
                }
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
            PlatformNotification::Disconnected { generation, .. } => {
                self.job_activations_disconnected(*generation);
                self.clipboard_disconnected(*generation, cx);
                self.create_output_disconnected(*generation, cx);
                self.space_disconnected(cx);
                self.recovery_disconnected(cx);
                let forms = self
                    .jobs
                    .forms
                    .values()
                    .map(|slot| slot.view.clone())
                    .collect::<Vec<_>>();
                for view in forms {
                    cx.defer(move |cx| {
                        let _ = view.update(cx, |form, cx| {
                            form.space_disconnected(cx);
                            form.picker_disconnected(cx);
                        });
                    });
                }
                for pending in self.jobs.pickers.drain().map(|(_, pending)| pending) {
                    tracing::info!(event="archive.picker.finished",correlation_id=%pending.operation,generation=pending.generation,success=false,reason="disconnected",elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive picker connection retired");
                }
                let ids = self
                    .jobs
                    .records
                    .iter()
                    .filter(|(_, record)| {
                        record.terminal.is_none() && record.connection == Some(*generation)
                    })
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>();
                for id in ids {
                    self.retire_job_observations(id);
                    let pending = self.jobs.records[&id]
                        .controls
                        .iter()
                        .map(|control| control.receipt)
                        .collect::<Vec<_>>();
                    for receipt in pending {
                        self.finish_job_control(id, receipt, "disconnected", None, cx);
                    }
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
                        record.connection = None;
                        record.stop_confirmation = None;
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
        mut terminal: ArchiveJobTerminal,
        cx: &mut Context<Self>,
    ) {
        self.retire_job_observations(id);
        let Some(record) = self.jobs.records.get_mut(&id) else {
            return;
        };
        if record.terminal.is_some() {
            return;
        }
        let connection = self.platform.connection_generation();
        let current_terminal = connection.is_some()
            && (record.connection == connection
                || record.connection.is_none() && !self.jobs.queue.is_running(id));
        if let Some(created) = record.created.take() {
            terminal.files_committed = created.files_committed;
            terminal.bytes_committed = created.bytes_committed;
            terminal.files_verified = created.files_verified;
            terminal.bytes_verified = created.bytes_verified;
            terminal.files_skipped = created.files_skipped;
            terminal.commit_state = created.commit_state;
            terminal.journal = created.journal;
            terminal.output_published = created.output_published;
            terminal.result_location = created.result_location;
            terminal.cleanup_complete &= created.cleanup_complete;
            terminal.elapsed_ms = terminal.elapsed_ms.saturating_add(created.elapsed_ms);
        }
        record.controls.clear();
        record.stop_confirmation = None;
        record.control_status = None;
        record.prompt = None;
        record.state = outcome_label(terminal.outcome).into();
        if let Some(request) = record.command.request.as_mut()
            && let Some(create) = request.create.as_mut()
        {
            create.password = None;
        }
        let open_directory = record.completion.open_after
            && record
                .command
                .request
                .as_ref()
                .is_some_and(|request| request.kind == ArchiveJobKind::Extract)
            && terminal.outcome == ArchiveJobOutcome::Succeeded
            && terminal.cleanup_complete
            && terminal.result_location.is_some();
        record.terminal = Some(terminal.clone());
        self.jobs.queue.finish(id, terminal.cleanup_complete);
        tracing::info!(event="archive.job.finished",job_id=%id,outcome=?terminal.outcome,files_committed=terminal.files_committed,files_skipped=terminal.files_skipped,elapsed_ms=terminal.elapsed_ms,logical_elapsed_ms=record.started.elapsed().as_millis() as u64,native_error_code=?terminal.native_error_code,cleanup_complete=terminal.cleanup_complete,"archive job terminal");
        let edit_origin = record.edit_origin;
        let edit_form = record.edit_form;
        let edit_intent = record
            .command
            .request
            .as_ref()
            .and_then(|request| request.modify.clone());
        let add = edit_intent
            .as_ref()
            .is_some_and(|intent| intent.action == ArchiveJobModifyAction::Add);
        if terminal.files_committed > 0
            && terminal.cleanup_complete
            && (!add
                || terminal.output_published == Some(true)
                    && terminal.commit_state.as_deref() == Some("Committed"))
            && let Some((window_id, generation, snapshot)) = edit_origin
        {
            self.refresh_modified_browser(
                window_id,
                generation,
                snapshot,
                edit_intent,
                terminal.add_result.clone(),
                cx,
            );
        }
        self.refresh_job(id, cx);
        if let Some(form) = edit_form {
            if matches!(
                terminal.outcome,
                ArchiveJobOutcome::Succeeded | ArchiveJobOutcome::SucceededWithWarnings
            ) && terminal.cleanup_complete
            {
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
                && (!add || terminal.output_published == Some(false))
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
                if add {
                    if let Some(slot) = self.jobs.forms.get(&form) {
                        let view = slot.view.clone();
                        cx.defer(move |cx| {
                            let _ = view.update(cx, |view, cx| view.release_failed_submit(cx));
                        });
                    }
                } else {
                    self.form_error(
                        form,
                        format!(
                            "编辑未完成：{}。原压缩包保留，草稿可检查后重试。",
                            terminal.code.as_deref().unwrap_or("原生结果未确认")
                        ),
                        cx,
                    );
                }
            } else {
                if let Some(slot) = self.jobs.forms.get(&form) {
                    let view = slot.view.clone();
                    let window = slot.window;
                    cx.defer(move |cx| { let _ = window.update(cx, |_, _, cx| {
                        let _ = view.update(cx, |view, cx| view.block_uncertain_retry("提交结果未知或已发布；不可盲目重试，请关闭草稿并刷新核对压缩包。".into(),cx));
                    }); });
                }
            }
        }
        self.notify_job(id, cx);
        self.present_job_terminal(id, current_terminal, cx);
        if open_directory {
            self.locate_job_result(id, cx);
        }
        self.jobs.retired.push_back(id);
        self.prune_retired_jobs();
    }
    fn prune_retired_jobs(&mut self) {
        self.prune_batches();
        // One bounded pass: protection postpones retirement, never discards its tracking ID.
        let candidates = self.jobs.retired.len();
        for _ in 0..candidates {
            if self.jobs.retired.len() <= 64 {
                break;
            }
            let Some(old) = self.jobs.retired.pop_front() else {
                break;
            };
            let reclaimable = self.jobs.records.get(&old).is_none_or(|record| {
                record.window.is_none()
                    && record.terminal.as_ref().is_some_and(|t| t.cleanup_complete)
                    && !record.retry_pending
                    && record.location_pending.is_none()
                    && !record.auto_location_waiting
                    && !self.jobs.queue.is_running(old)
                    && !self.batch_retirement_held(old)
            });
            if reclaimable {
                self.jobs.records.remove(&old);
                self.prune_batches();
            } else {
                self.jobs.retired.push_back(old);
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
        let reclamation = record
            .command
            .request
            .as_ref()
            .and_then(|request| request.extract.as_ref())
            .and_then(|extract| extract.reclamation)
            .map(|intent| intent.mode);
        let Some(terminal) = record.terminal.as_ref() else {
            return;
        };
        record.terminal_notified = true;
        let failed = matches!(
            terminal.outcome,
            ArchiveJobOutcome::Failed | ArchiveJobOutcome::Rejected
        );
        let request = record.command.request.as_ref();
        let title = reclamation.map_or_else(
            || {
                if failed {
                    format!(
                        "{}{}",
                        job_action_label(request),
                        if terminal.outcome == ArchiveJobOutcome::Rejected {
                            "未开始"
                        } else {
                            "失败"
                        }
                    )
                } else {
                    outcome_label(terminal.outcome).to_owned()
                }
            },
            |mode| {
                let action = if mode == ArchiveReclamationMode::Continue {
                    "继续空间回收解压"
                } else {
                    "空间回收解压"
                };
                format!(
                    "{action}{}",
                    match terminal.outcome {
                        ArchiveJobOutcome::Succeeded => "完成",
                        ArchiveJobOutcome::SucceededWithWarnings => "完成，有警告",
                        ArchiveJobOutcome::Cancelled => "已取消",
                        ArchiveJobOutcome::Failed => "失败",
                        ArchiveJobOutcome::Rejected => "未获准入",
                    }
                )
            },
        );
        let cleanup = if !terminal.cleanup_complete {
            "原生清理未确认，任务槽仍保留。"
        } else {
            ""
        };
        let message = if reclamation.is_some() {
            let authority = match terminal.outcome {
                ArchiveJobOutcome::Succeeded | ArchiveJobOutcome::SucceededWithWarnings => {
                    "原压缩包是已消耗外壳，不是完整可用压缩包；请保留恢复记录和输出，不能恢复完整原压缩包。"
                }
                ArchiveJobOutcome::Cancelled => {
                    "已释放的数据不会恢复；请保留原压缩包、恢复记录和输出，用“继续空间回收解压”处理剩余文件。"
                }
                ArchiveJobOutcome::Failed => {
                    "原压缩包可能已是已消耗外壳；请保留原压缩包、恢复记录和输出，只能继续剩余解压。"
                }
                ArchiveJobOutcome::Rejected => {
                    "本次未开始新的释放；若已有恢复标记，请从“继续空间回收解压”进入，不要使用普通重试。"
                }
            };
            format!(
                "已提交 {} 个文件，跳过 {} 个。{authority}{cleanup}",
                terminal.files_committed, terminal.files_skipped
            )
        } else if failed {
            format!("{}{cleanup}", job_failure_message(request, terminal))
        } else {
            format!(
                "已提交 {} 个文件，跳过 {} 个。{cleanup}",
                terminal.files_committed, terminal.files_skipped
            )
        };
        let level = match terminal.outcome {
            ArchiveJobOutcome::Succeeded => crate::notifications::NotificationLevel::Success,
            ArchiveJobOutcome::Failed | ArchiveJobOutcome::Rejected => {
                crate::notifications::NotificationLevel::Error
            }
            _ => crate::notifications::NotificationLevel::Info,
        };
        let owner = cx.entity().downgrade();
        let mut notification = crate::notifications::AppNotification::new(
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
        if let Some(window) = record
            .edit_origin
            .and_then(|(id, _, _)| self.windows.get(&id).map(|slot| slot.window))
            .or_else(|| record.window.as_ref().map(|(window, _)| *window))
        {
            notification = notification.in_window(window);
        }
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
        let connection = self.platform.connection_generation()?;
        if purpose == ArchivePickerPurpose::Folders
            && self.platform.archive_multi_folder_picker_available() == Some(false)
        {
            self.form_error(
                form,
                "当前平台组件不支持多文件夹选择，请更新后重试。".into(),
                cx,
            );
            return None;
        }
        let target = self
            .jobs
            .forms
            .get(&form)?
            .view
            .read_with(cx, |form, _| form.intake_target())
            .ok()?;
        if target.is_some() && !self.intake_target_current(form, &target, cx) {
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
                started: Instant::now(),
                connection,
                target,
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
            .send_command_with_id_correlated(
                PlatformCommand::ArchivePicker(command),
                operation,
                operation,
            )
            .is_err()
        {
            if let Some(pending) = self.jobs.pickers.remove(&operation) {
                tracing::info!(event="archive.picker.finished",correlation_id=%operation,generation,success=false,reason="not_submitted",elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive picker not submitted");
            }
            self.form_error(form, "选择器不可用，请重试。".into(), cx);
            return None;
        }
        tracing::info!(event="archive.picker.started",correlation_id=%operation,generation,purpose=?purpose,"archive picker request accepted");
        Some(operation)
    }
    pub(crate) fn retire_form(&mut self, id: Uuid) {
        self.cancel_confirmation_member(id);
        self.retire_form_clipboard(id);
        self.retire_create_output_plan(id);
        self.retire_form_space(id);
        self.retire_recovery(id);
        self.jobs.forms.remove(&id);
        self.jobs.pickers.retain(|_, pending| {
            if pending.form == id {
                tracing::info!(event="archive.picker.finished",correlation_id=%pending.operation,generation=pending.generation,success=false,reason="form_closed",elapsed_ms=pending.started.elapsed().as_millis() as u64,"archive picker form retired");
                false
            } else {
                true
            }
        });
        self.jobs.edit_forms.remove(&id);
        self.prune_retired_jobs();
    }
    pub(super) fn refresh_job(&mut self, id: Uuid, cx: &mut Context<Self>) {
        self.refresh_batch_views(id, cx);
        let surface_id = self.job_surface_id(id);
        if !self.job_is_selected(id) {
            return;
        }
        let Some(record) = self.jobs.records.get(&id) else {
            return;
        };
        let Some((window, view)) = self
            .jobs
            .records
            .get(&surface_id)
            .and_then(|r| r.window.clone())
        else {
            return;
        };
        let snapshot = crate::archive::progress::ProgressSnapshot {
            id,
            retry: batch_jobs::retry_facts(record),
            retry_pending: record.retry_pending,
            operation: record.active_operation,
            generation: record.command.generation,
            continuation: record
                .can_continue_reclamation()
                .then_some((record.active_operation, record.command.generation)),
            state: record.state.clone(),
            kind: record
                .command
                .request
                .as_ref()
                .map_or(ArchiveJobKind::Test, |request| request.kind),
            edit_format: record
                .command
                .request
                .as_ref()
                .and_then(|request| request.modify.as_ref())
                .map(|modify| modify.browser.format.clone()),
            reclamation: record
                .command
                .request
                .as_ref()
                .and_then(|request| request.extract.as_ref())
                .and_then(|extract| extract.reclamation)
                .map(|intent| intent.mode),
            elapsed_ms: cx
                .background_executor()
                .now()
                .saturating_duration_since(record.started)
                .as_millis()
                .min(u64::MAX as u128) as u64,
            progress_elapsed_ms: record.progress_elapsed_ms,
            progress: record.progress.clone(),
            terminal: record.terminal.clone(),
            prompt: record.prompt.clone(),
            paused: record.paused,
            controls: JobControls {
                pause: record.pause_allowed() && record.stop_confirmation.is_none(),
                stop: record.stop_allowed() && record.stop_confirmation.is_none(),
            },
            stop_question: record
                .stop_confirmation
                .as_ref()
                .filter(|stop| !stop.declined)
                .map(|stop| StopQuestion {
                    token: stop.token,
                    waiting: stop.pause_receipt.is_some(),
                }),
            location_status: record.location_status.clone(),
            pause_requested: record
                .controls
                .iter()
                .find_map(|pending| match pending.action {
                    ArchiveJobAction::Pause => Some(true),
                    ArchiveJobAction::Resume => Some(false),
                    _ => None,
                }),
            cancel_requested: record
                .controls
                .iter()
                .any(|pending| pending.action == ArchiveJobAction::Cancel)
                || record.state.starts_with("正在取消"),
            control_status: record.control_status.clone(),
            batch: self.batch_snapshot_for(id),
        };
        let platform = self.platform.clone();
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            if !owner
                .read_with(cx, |app, _| app.job_is_selected(id))
                .unwrap_or(false)
            {
                return;
            }
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
        journal_summary: None,
        output_shortfall: None,
        result_location: None,
        add_result: None,
        output_published: (cleanup_complete
            && matches!(code, "not_submitted" | "disconnected_before_start"))
        .then_some(false),
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
fn job_action_label(request: Option<&ArchiveJobRequest>) -> &'static str {
    match request.map(|request| request.kind) {
        Some(ArchiveJobKind::Extract) => "解压文件",
        Some(ArchiveJobKind::Create) => "创建压缩包",
        Some(ArchiveJobKind::Test) => "测试压缩包",
        Some(ArchiveJobKind::Modify) => match request
            .and_then(|request| request.modify.as_ref())
            .map(|modify| modify.action)
        {
            Some(ArchiveJobModifyAction::Add) => "添加文件",
            Some(ArchiveJobModifyAction::Delete) => "删除压缩包项目",
            Some(ArchiveJobModifyAction::Rename) => "重命名压缩包项目",
            Some(ArchiveJobModifyAction::NewDirectory) => "新建压缩包目录",
            Some(ArchiveJobModifyAction::ReplaceContent) => "替换文件内容",
            Some(ArchiveJobModifyAction::Comment) => "修改压缩包注释",
            None => "修改压缩包",
        },
        None => "压缩包操作",
    }
}

fn job_failure_message(
    request: Option<&ArchiveJobRequest>,
    terminal: &ArchiveJobTerminal,
) -> String {
    let reason = match (terminal.code.as_deref(), terminal.native_error_code) {
        (Some("stale_add_target"), _) => "原压缩包、目录或连接已变化，请在目标目录重新拖入文件。",
        (Some("source_changed" | "target_changed"), _) => {
            "来源或目标在操作期间已变化，请重新打开压缩包后再操作。"
        }
        (Some("unsafe_path"), _) => "来源或压缩包成员的路径未通过安全校验，无法继续写入。",
        (Some("output_collision" | "target_exists" | "source_output_collision"), _) => {
            "目标名称或文件类型发生冲突，无法写入；请重新选择同名处理方式。"
        }
        (Some("password_required"), _) => "此压缩包需要密码，请输入密码后再操作。",
        (Some("wrong_password"), _) => "密码不正确，请确认密码后再操作。",
        (Some("authentication_or_data_error"), _) => {
            "密码验证或加密数据校验失败，请确认密码与压缩包完整性。"
        }
        (Some("corrupt_archive"), _) => "压缩包数据校验失败，无法完成操作。",
        (Some("resource_limit"), _) => "本次操作超出允许的资源上限，已停止处理。",
        (
            Some(
                "unsupported_operation"
                | "unsupported_format"
                | "unsupported_encryption"
                | "worker_capability_mismatch",
            ),
            _,
        ) => "此压缩包的格式或结构不支持当前操作。",
        (
            Some(
                "stale_connection" | "disconnected_before_start" | "disconnected" | "not_submitted",
            ),
            _,
        ) => "本地处理程序连接已中断，任务无法继续；请恢复连接后重新操作。",
        (_, Some(32 | 33)) => "相关文件正被其他程序占用，请关闭占用后重新操作。",
        (_, Some(5)) => "没有访问相关文件的权限，无法完成操作。",
        (_, Some(112)) => "目标磁盘空间不足，无法完成写入。",
        (Some("source_unavailable"), _) => "无法读取所选来源，文件可能已移动、被占用或不可访问。",
        (Some("timeout"), _) => "处理程序未在规定时间内完成；请等待清理结束后再操作。",
        (Some("io_failure" | "worker_failure"), _) => "读取来源或写入候选文件失败，操作已停止。",
        _ => "处理程序未能完成本次操作；任务结果中保留了具体诊断。",
    };
    let published = terminal.output_published == Some(true)
        || terminal
            .journal
            .iter()
            .any(|entry| entry.published == Some(true));
    let publication = if terminal.commit_state.as_deref() == Some("CommitOutcomeUnknown") {
        "写入结果尚未确认，请检查结果后再操作。"
    } else if published {
        "已有输出发布，请确认结果后再操作。"
    } else if terminal.output_published == Some(false)
        && request.is_some_and(|request| request.kind == ArchiveJobKind::Modify)
    {
        "原压缩包未被替换。"
    } else {
        ""
    };
    format!("{reason}{publication}")
}

fn outcome_label(outcome: ArchiveJobOutcome) -> &'static str {
    match outcome {
        ArchiveJobOutcome::Succeeded => "压缩包操作完成",
        ArchiveJobOutcome::SucceededWithWarnings => "压缩包操作完成，有警告",
        ArchiveJobOutcome::Cancelled => "压缩包操作已取消",
        ArchiveJobOutcome::Failed => "压缩包操作失败",
        ArchiveJobOutcome::Rejected => "压缩包操作未获准入",
    }
}

impl ArchiveApplication {
    fn job_surface_id(&self, id: Uuid) -> Uuid {
        self.jobs
            .records
            .get(&id)
            .and_then(|r| r.batch)
            .and_then(|batch| self.jobs.batches.get(&batch))
            .and_then(|batch| batch.members.first().copied())
            .unwrap_or(id)
    }
    fn job_is_selected(&self, id: Uuid) -> bool {
        self.jobs
            .records
            .get(&id)
            .and_then(|r| r.batch)
            .and_then(|batch| self.jobs.batches.get(&batch))
            .is_none_or(|batch| batch.selected == Some(id))
    }
    pub(in crate::archive) fn show_job(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let surface_id = self.job_surface_id(id);
        if let Some(batch) = self
            .jobs
            .records
            .get(&id)
            .and_then(|r| r.batch)
            .and_then(|batch| self.jobs.batches.get_mut(&batch))
        {
            batch.selected = Some(id);
        }
        let Some(record) = self.jobs.records.get(&id) else {
            return;
        };
        if let Some((window, view)) = self
            .jobs
            .records
            .get(&surface_id)
            .and_then(|r| r.window.clone())
        {
            self.refresh_job(id, cx);
            let owner = cx.entity().downgrade();
            cx.defer(move |cx| {
                if !owner
                    .read_with(cx, |app, _| {
                        app.job_is_selected(id) && app.job_window_is_current(surface_id, window)
                    })
                    .unwrap_or(false)
                {
                    return;
                }
                let _ = window.update(cx, |_, window, cx| {
                    let _ = view.update(cx, |view, cx| view.show_detail(window, cx));
                });
            });
            if self.jobs.material_pending.get(&surface_id) == Some(&window) {
                return;
            }
            self.activate_job(id, JobPresentation::Reuse, cx);
            return;
        }
        let keep_open = record.completion.keep_open || record.terminal.is_some();
        let owner = cx.entity().downgrade();
        let closing_owner = owner.clone();
        let capture = Rc::new(std::cell::RefCell::new(None));
        let captured = capture.clone();
        let placement = record.placement;
        let bounds = placement.centered(size(px(560.0), px(260.0)));
        let opened = cx.open_window(
            WindowOptions {
                show: false,
                focus: false,
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                display_id: placement.display_id,
                window_min_size: Some(size(px(480.0), px(260.0))),
                app_id: Some(format!("NxTools.Archive.Job.{id}")),
                ..crate::ui::components::titlebar::caption_window_options()
            },
            move |window, cx| {
                window.set_window_title("压缩包任务 — NxTools");
                let view = cx.new(|cx| JobProgressView::new(id, owner, keep_open, cx));
                *captured.borrow_mut() = Some(view.downgrade());
                let closing_window = window.window_handle();
                window.on_window_should_close(cx, move |_, cx| {
                    let owner = closing_owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |app, cx| {
                            if app.job_window_is_current(surface_id, closing_window) {
                                app.background_job(surface_id, cx);
                            }
                        });
                    });
                    false
                });
                cx.new(|cx| gpui_kit::base::Root::new(view, window, cx))
            },
        );
        if let (Ok(window), Some(view)) = (opened, capture.borrow_mut().take()) {
            if let Some(record) = self.jobs.records.get_mut(&surface_id) {
                record.window = Some((window.into(), view));
            }
            self.refresh_job(id, cx);
            self.prepare_job_material(surface_id, window.into(), cx);
        }
    }
    pub(crate) fn background_job(&mut self, id: Uuid, cx: &mut Context<Self>) {
        let surface_id = self.job_surface_id(id);
        let members = self
            .jobs
            .records
            .get(&id)
            .and_then(|r| r.batch)
            .and_then(|batch| self.jobs.batches.get(&batch))
            .map(|batch| batch.members.clone())
            .unwrap_or_else(|| vec![id]);
        let id = self
            .jobs
            .records
            .get(&id)
            .and_then(|r| r.batch)
            .and_then(|batch| self.jobs.batches.get(&batch))
            .and_then(|batch| batch.selected)
            .unwrap_or(id);
        if let Some(token) = self
            .jobs
            .records
            .get(&id)
            .and_then(|record| record.stop_confirmation.as_ref().map(|stop| stop.token))
        {
            self.answer_job_stop(id, token, false, cx);
        }
        if let Some((window, view)) = self
            .jobs
            .records
            .get_mut(&surface_id)
            .and_then(|record| record.window.take())
        {
            self.jobs.material_pending.remove(&surface_id);
            self.retire_job_activation(surface_id, "retired");
            tracing::info!(event="archive.job.presentation_closed", job_id=%id, "archive progress closed without cancellation");
            let platform = self.platform.clone();
            cx.defer(move |cx| {
                let _ = view.update(cx, |view, _| view.retire_display());
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
        self.prune_retired_jobs();
        for member in members {
            self.restore_prompt_surface(member, cx);
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
                    "压缩包任务已转后台",
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
    pub(in crate::archive) fn open_create_paths(
        &mut self,
        paths: Vec<String>,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty()
            || paths.len() > 32
            || paths
                .iter()
                .any(|path| !nxtools_protocol::archive_job::valid_job_path(path))
        {
            return;
        }
        let Some(id) = self.open_form(Some(vec![]), None, None, cx) else {
            return;
        };
        let view = self.jobs.forms[&id].view.clone();
        cx.defer(move |cx| {
            let _ = view.update(cx, |form, cx| {
                if !form.retired {
                    form.enumerate_sources(paths, cx);
                }
            });
        });
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
        self.open_form(None, Some((path, selection, None, None)), None, cx);
    }
    /// The receiver-created folder is filled by the original ordinary extraction job.
    pub(in crate::archive) fn extract_dropped(
        &mut self,
        drop: crate::archive::browser::LocatedDrop,
        cx: &mut Context<Self>,
    ) {
        let operation = drop.operation;
        let selected_count = drop
            .selection
            .as_ref()
            .map(|selection| selection.ordinals.len());
        let mut draft = crate::archive::draft::ExtractDraft::new(&self.settings, drop.source, None);
        draft.destination = drop.destination;
        draft.strategy = nxtools_core::archive::ArchiveDestination::Here;
        let submitted = draft.request(&self.settings).and_then(|mut request| {
            request.codepage = drop.codepage;
            request.extract.as_mut().expect("extract options").selection = drop.selection;
            request
                .validate()
                .map_err(|_| "拖出选区已失效或超过任务传输限制，请刷新后重试。".to_owned())?;
            self.submit_job_with_id(
                operation,
                request,
                FormCompletion {
                    keep_open: self.settings.keep_extract_progress_open,
                    ..FormCompletion::default()
                },
                cx,
            )
        });
        tracing::info!(
            event = "archive.drop_extract.submitted",
            correlation_id = %operation,
            job_id = ?submitted.as_ref().ok(),
            selected_count,
            success = submitted.is_ok(),
            "archive drop extraction"
        );
        if let Err(message) = submitted
            && let Some(center) = crate::notifications::current(cx)
        {
            let notification = crate::notifications::AppNotification::new(
                format!("archive-drop-{}", Uuid::new_v4()),
                crate::notifications::NotificationLevel::Error,
                "拖出解压未开始",
                message,
            );
            cx.defer(move |cx| center.update(cx, |center, cx| center.push(notification, cx)));
        }
    }
    pub(in crate::archive) fn open_reclamation_continue_form(
        &mut self,
        path: PathBuf,
        destination: Option<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        if path.as_os_str().is_empty() {
            return;
        }
        self.open_form(
            None,
            Some((
                path,
                None,
                None,
                Some(ReclamationContinuation { destination }),
            )),
            None,
            cx,
        );
    }
    pub(in crate::archive) fn open_extract_confirmation(
        &mut self,
        path: PathBuf,
        strategy: nxtools_core::archive::ArchiveDestination,
        cx: &mut Context<Self>,
    ) {
        self.open_form(None, Some((path, None, Some(strategy), None)), None, cx);
    }
    fn open_form(
        &mut self,
        sources: Option<Vec<crate::archive::draft::DraftSource>>,
        extract: Option<ExtractForm>,
        modify: Option<(ModifyDraft, (Uuid, u64, Uuid))>,
        cx: &mut Context<Self>,
    ) -> Option<Uuid> {
        let width = if sources.is_some() { 680.0 } else { 560.0 };
        let settings = self.settings.clone();
        let owner = cx.entity().downgrade();
        let closing_owner = owner.clone();
        let captured = Rc::new(std::cell::RefCell::new(None));
        let capture = captured.clone();
        let reclamation_continue = extract
            .as_ref()
            .is_some_and(|(_, _, _, continuation)| continuation.is_some());
        let modify_origin = if let Some((_, origin)) = &modify {
            Some(EditFormOrigin {
                browser: *origin,
                connection: self.platform.connection_generation()?,
            })
        } else {
            None
        };
        let modify_objects = modify
            .as_ref()
            .and_then(|(draft, (id, generation, snapshot))| {
                self.windows
                    .get(id)
                    .filter(|slot| slot.generation == *generation)
                    .and_then(|slot| {
                        slot.view
                            .read_with(cx, |view, _| {
                                let Some(model) = view
                                    .model
                                    .as_ref()
                                    .filter(|model| model.snapshot == *snapshot)
                                else {
                                    return Vec::new();
                                };
                                let mut bytes = 0usize;
                                draft
                                    .source_prefix
                                    .iter()
                                    .cloned()
                                    .chain(
                                        model
                                            .entries
                                            .iter()
                                            .filter(|entry| draft.ordinals.contains(&entry.ordinal))
                                            .map(|entry| entry.name.clone()),
                                    )
                                    .take(32)
                                    .take_while(|name| {
                                        bytes = bytes.saturating_add(name.len());
                                        bytes <= 8192
                                    })
                                    .collect::<Vec<_>>()
                            })
                            .ok()
                    })
            })
            .unwrap_or_default();
        let placement = modify
            .as_ref()
            .and_then(|(_, (id, _, _))| self.windows.get(id).map(|slot| slot.window))
            .map(|owner| {
                crate::ui::monitor_placement::owner_monitor_placement(None, Some(owner), cx)
            })
            .unwrap_or_else(|| crate::ui::monitor_placement::active_monitor_placement(cx));
        let bounds = placement.centered(size(px(width), px(600.0)));
        let opened = cx.open_window(
            WindowOptions {
                show: false,
                focus: false,
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                display_id: placement.display_id,
                window_min_size: Some(size(px(480.0), px(360.0))),
                ..crate::ui::components::titlebar::caption_window_options()
            },
            move |window, cx| {
                window.set_window_title(if sources.is_some() {
                    "新建压缩包 — NxTools"
                } else if let Some((draft, _)) = &modify {
                    match draft.browser.format.as_str() {
                        "SevenZ" => "编辑 7z — NxTools",
                        "Tar" | "TarGz" | "TarXz" => "编辑 TAR — NxTools",
                        _ => "编辑 ZIP — NxTools",
                    }
                } else if reclamation_continue {
                    "继续空间回收解压 — NxTools"
                } else {
                    "解压 — NxTools"
                });
                let view = cx.new(|cx| {
                    if let Some(sources) = sources {
                        OperationForm::create(settings, sources, owner, window, cx)
                    } else if let Some((draft, _)) = modify {
                        let mut form = OperationForm::modify(settings, draft, owner, window, cx);
                        form.set_modify_objects(modify_objects);
                        form
                    } else {
                        let (path, selection, strategy, continuation) =
                            extract.expect("extract draft");
                        if let Some(continuation) = continuation {
                            OperationForm::continue_reclamation(
                                settings,
                                path,
                                continuation.destination,
                                owner,
                                window,
                                cx,
                            )
                        } else {
                            OperationForm::extract_with_strategy(
                                settings,
                                (path, selection, strategy),
                                owner,
                                window,
                                cx,
                            )
                        }
                    }
                });
                let (id, generation) = view.read_with(cx, |form, _| (form.id, form.generation));
                *capture.borrow_mut() = Some((id, generation, view.downgrade()));
                let closing_view = view.downgrade();
                window.on_window_should_close(cx, move |window, cx| {
                    let _ = closing_view.update(cx, |form, cx| form.retire(window, cx));
                    let owner = closing_owner.clone();
                    cx.defer(move |cx| {
                        let _ = owner.update(cx, |app, cx| {
                            app.retire_form(id);
                            app.refresh_all_batch_views(cx);
                        });
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
            self.prepare_form_material(id, generation, window.into(), cx);
            if let Some(origin) = modify_origin {
                self.jobs.edit_forms.insert(id, origin);
            }
            Some(id)
        } else {
            None
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
        target: (Option<String>, Option<String>),
        codepage: u32,
        cx: &mut Context<Self>,
    ) {
        let (value, source_prefix) = target;
        let (id, generation, snapshot) = origin;
        let Some(slot) = self.windows.get(&id).filter(|s| {
            s.generation == generation
                && s.view
                    .read_with(cx, |view, _| {
                        view.operation_refusal(action.operation(value.as_deref()))
                            .is_none()
                            && view.edit_members_current(
                                action,
                                &ordinals,
                                value.as_deref(),
                                source_prefix.as_deref(),
                            )
                            && view.model.as_ref().is_some_and(|m| m.snapshot == snapshot)
                    })
                    .unwrap_or(false)
        }) else {
            return;
        };
        let Some(browser) = slot.browser.as_ref().filter(|b| {
            b.snapshot_id == snapshot
                && matches!(
                    b.format.as_str(),
                    "Zip" | "SevenZ" | "Tar" | "TarGz" | "TarXz"
                )
        }) else {
            return;
        };
        if action == ArchiveJobModifyAction::ReplaceContent
            && (!matches!(
                browser.format.as_str(),
                "SevenZ" | "Tar" | "TarGz" | "TarXz"
            ) || ordinals.len() != 1
                || value.is_some()
                || !slot
                    .view
                    .read_with(cx, |view, _| {
                        view.model.as_ref().is_some_and(|model| {
                            model.entries.iter().any(|entry| {
                                entry.ordinal == ordinals[0]
                                    && entry.kind == "File"
                                    && entry.warning_codes.is_empty()
                            })
                        })
                    })
                    .unwrap_or(false))
        {
            return;
        }
        let draft = ModifyDraft {
            source: slot.path.clone(),
            browser: browser.clone(),
            action,
            ordinals,
            value,
            source_prefix,
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
        intent: Option<nxtools_protocol::archive_job::ArchiveJobModify>,
        add_result: Option<ArchiveJobAddResult>,
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
        if let Some(intent) = intent.as_ref()
            && intent.action == ArchiveJobModifyAction::Add
            && (slot.browser.as_ref() != Some(&intent.browser)
                || !slot
                    .view
                    .read_with(cx, |view, _| {
                        view.edit_source.as_ref() == Some(&intent.browser.source)
                            && view.model.as_ref().is_some_and(|model| {
                                model.snapshot == snapshot
                                    && model.folder
                                        == intent
                                            .value
                                            .as_deref()
                                            .unwrap_or("")
                                            .trim_end_matches('/')
                            })
                    })
                    .unwrap_or(false))
        {
            return;
        }
        let codepage = slot
            .view
            .read_with(cx, |view, _| view.codepage)
            .unwrap_or(0);
        if let Some(intent) = intent {
            let view = slot.view.clone();
            let next_generation = generation.wrapping_add(1).max(1);
            cx.defer(move |cx| {
                let _ = view.update(cx, |view, _| {
                    if intent.action == ArchiveJobModifyAction::Add {
                        view.prepare_add_refresh(&intent, add_result.as_ref(), next_generation);
                    } else {
                        view.prepare_edit_refresh(&intent, next_generation);
                    }
                });
            });
        }
        self.refresh_browser(id, codepage, cx);
    }
}
impl ArchiveApplication {
    pub(crate) fn keep_job_open(&mut self, id: Uuid, value: bool, cx: &mut Context<Self>) {
        let Some(record) = self.jobs.records.get_mut(&id) else {
            return;
        };
        record.completion.keep_open = value;
        let create = record
            .command
            .request
            .as_ref()
            .is_some_and(|request| request.kind == ArchiveJobKind::Create);
        let Some(path) = self.config_path.clone() else {
            return;
        };
        let owner = cx.entity().downgrade();
        let background = cx.background_executor().clone();
        let correlation_id = Uuid::new_v4();
        let started = Instant::now();
        tracing::info!(event="archive.preferences.save_started", %correlation_id, job_id=%id, "progress preference save accepted");
        cx.spawn(async move |_, cx| {
            let saved = background.spawn(async move {
                super::save_settings(&path, |settings| {
                    if create { settings.keep_create_progress_open = value; }
                    else { settings.keep_extract_progress_open = value; }
                }).map_err(|_| ())
            }).await;
            tracing::info!(event="archive.preferences.save_finished", %correlation_id, job_id=%id, success=saved.is_ok(), elapsed_ms=started.elapsed().as_millis() as u64, "progress preference save finished");
            let _ = owner.update(cx, |app, cx| match saved {
                Ok(saved) => app.publish_saved_settings(saved, cx),
                Err(()) => {
                    if let Some(center) = crate::notifications::current(cx) {
                        cx.defer(move |cx| { center.update(cx, |center, cx| center.push(
                            crate::notifications::AppNotification::new("archive-progress-preference", crate::notifications::NotificationLevel::Error,
                                "窗口偏好未保存", "当前任务保持所选行为；重启后仍使用原配置，请检查配置后重试。"), cx)); });
                    }
                }
            });
        }).detach();
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
            let Some(target) = terminal
                .result_location
                .as_ref()
                .filter(|location| {
                    terminal.output_published == Some(true)
                        && terminal.commit_state.as_deref() == Some("Committed")
                        && valid_job_path(&location.target)
                })
                .map(|location| location.target.clone())
            else {
                terminal.outcome = ArchiveJobOutcome::Failed;
                terminal.code = Some("created_result_location_unavailable".into());
                return None;
            };
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
            record.state = "压缩包已提交，正在创建后测试".into();
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
