//! Original application job projection and read-only revalidation, never a scheduler.
use super::*;
use crate::archive::batch::{BatchMember, BatchRecord, BatchSnapshot, MemberState};
use nxtools_protocol::archive_space::{ArchiveSpaceCommand, ArchiveSpaceObservation};
pub(super) struct JobObservation {
    pub command: ArchiveSpaceCommand,
    pub job: Option<(Uuid, Uuid, bool)>, // Logical member, concrete native attempt, explicit recheck.
    pub response: Option<async_channel::Sender<Result<ArchiveSpaceObservation, String>>>,
    pub connection: u64,
    pub _deadline: Task<()>,
}
fn retry_shape(record: &JobRecord) -> bool {
    record.command.request.as_ref().is_some_and(|r| {
        r.paths.len() == 1
            && matches!(r.kind, ArchiveJobKind::Test | ArchiveJobKind::Extract)
            && r.extract
                .as_ref()
                .is_none_or(|e| e.selection.is_none() && e.reclamation.is_none())
    })
}
fn retry_facts(record: &JobRecord) -> bool {
    retry_shape(record)
        && record.retry_baseline.is_some()
        && !record.retry_pending
        && record.terminal.as_ref().is_some_and(|r| {
            matches!(
                r.outcome,
                ArchiveJobOutcome::Failed | ArchiveJobOutcome::Rejected
            ) && r.cleanup_complete
                && r.files_committed == 0
                && r.output_published == Some(false)
                && r.result_location.is_none()
                && matches!(
                    r.commit_state.as_deref(),
                    None | Some("Preparing" | "Verified" | "FailedBeforeCommit")
                )
                && r.journal.iter().all(|j| {
                    j.published == Some(false)
                        && matches!(
                            j.state.as_str(),
                            "Preparing" | "Verified" | "FailedBeforeCommit"
                        )
                })
                && matches!(
                    r.code.as_deref(),
                    Some(
                        "io_failure"
                            | "not_submitted"
                            | "disconnected_before_start"
                            | "source_busy"
                            | "target_busy"
                            | "target_denied"
                    )
                )
        })
}
impl ArchiveApplication {
    pub(in crate::archive) fn bind_batch(
        &mut self,
        members: Vec<Uuid>,
        refusal: Option<(usize, String)>,
        cx: &mut Context<Self>,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let requested = refusal.as_ref().map_or(members.len(), |(count, _)| *count);
        for member in &members {
            if let Some(record) = self.jobs.records.get_mut(member) {
                record.batch = Some(id);
            }
        }
        self.jobs.batches.insert(
            id,
            BatchRecord {
                members: members.clone(),
                requested,
                refusal: refusal.map(|(_, message)| message),
                pending_forms: Vec::new(),
                terminal_logged: false,
                cancelled_unstarted: 0,
            },
        );
        self.jobs.batch_order.push_back(id);
        tracing::info!(event="archive.batch.accepted",batch_id=%id,requested,accepted=members.len(),"archive batch bound to original logical jobs");
        if let Some(member) = members.first() {
            self.refresh_batch_views(*member, cx);
        }
        self.prune_batches();
        id
    }
    pub(in crate::archive) fn batch_snapshot_for(&self, id: Uuid) -> Option<BatchSnapshot> {
        let batch_id = self.jobs.records.get(&id)?.batch?;
        let batch = self.jobs.batches.get(&batch_id)?;
        let members = batch
            .members
            .iter()
            .filter_map(|id| {
                let record = self.jobs.records.get(id)?;
                let terminal = record.terminal.as_ref();
                let state = if let Some(result) = terminal {
                    match result.outcome {
                        ArchiveJobOutcome::Succeeded => MemberState::Succeeded,
                        ArchiveJobOutcome::SucceededWithWarnings => MemberState::Warnings,
                        ArchiveJobOutcome::Cancelled => MemberState::Cancelled,
                        _ => MemberState::Failed,
                    }
                } else if record.prompt.is_some() {
                    MemberState::Waiting
                } else if record.paused {
                    MemberState::Paused
                } else if !self.jobs.queue.is_running(*id) {
                    MemberState::Queued
                } else if !record.ever_started {
                    MemberState::Admitting
                } else {
                    MemberState::Running
                };
                let label = record
                    .command
                    .request
                    .as_ref()
                    .and_then(|r| r.paths.first())
                    .map_or_else(|| "归档任务".into(), |p| p.clone());
                let detail = terminal.map_or_else(
                    || record.state.clone(),
                    |result| {
                        format!(
                            "{}；Windows {}；提交文件 {}，跳过 {}；{}{}",
                            result.code.as_deref().unwrap_or("无错误代码"),
                            result
                                .native_error_code
                                .map_or_else(|| "未知".into(), |n| n.to_string()),
                            result.files_committed,
                            result.files_skipped,
                            result.commit_state.as_deref().unwrap_or("无提交状态"),
                            if retry_facts(record) {
                                "；可重验后重试"
                            } else {
                                ""
                            }
                        )
                    },
                );
                Some(BatchMember {
                    id: *id,
                    label,
                    state,
                    never_started: !record.ever_started,
                    cleanup_unknown: terminal.is_some_and(|r| !r.cleanup_complete),
                    published: terminal.is_some_and(|r| {
                        r.output_published == Some(true)
                            || r.files_committed > 0
                            || r.result_location.is_some()
                            || r.journal.iter().any(|j| j.published == Some(true))
                    }),
                    retry: retry_facts(record),
                    retry_pending: record.retry_pending,
                    detail,
                })
            })
            .collect::<Vec<_>>();
        Some(BatchSnapshot {
            id: batch_id,
            not_admitted: batch.requested.saturating_sub(
                batch.members.len() + batch.pending_forms.len() + batch.cancelled_unstarted,
            ),
            pending_forms: batch.pending_forms.len(),
            cancelled_unstarted: batch.cancelled_unstarted,
            members,
            refusal: batch.refusal.clone(),
        })
    }
    pub(in crate::archive) fn refresh_batch_views(&self, id: Uuid, cx: &mut Context<Self>) {
        let Some(snapshot) = self.batch_snapshot_for(id) else {
            return;
        };
        for member in &snapshot.members {
            if let Some((_, view)) = self
                .jobs
                .records
                .get(&member.id)
                .and_then(|r| r.window.clone())
            {
                let snapshot = snapshot.clone();
                cx.defer(move |cx| {
                    let _ = view.update(cx, |view, cx| view.apply_batch(Some(snapshot), cx));
                });
            }
        }
    }
    pub(in crate::archive) fn prune_batches(&mut self) {
        let gone = self
            .jobs
            .batch_order
            .iter()
            .copied()
            .filter(|id| {
                self.jobs.batches.get(id).is_none_or(|batch| {
                    batch
                        .members
                        .iter()
                        .any(|id| !self.jobs.records.contains_key(id))
                })
            })
            .collect::<Vec<_>>();
        let mut gone = gone;
        let mut retained = self.jobs.batches.len().saturating_sub(gone.len());
        for id in &self.jobs.batch_order {
            if retained <= 64 {
                break;
            }
            if gone.contains(id) {
                continue;
            }
            if self.jobs.batches.get(id).is_some_and(|batch| {
                batch.pending_forms.is_empty()
                    && batch.members.iter().all(|id| {
                        self.jobs.records.get(id).is_none_or(|r| {
                            r.terminal.as_ref().is_some_and(|t| t.cleanup_complete)
                                && !r.retry_pending
                                && !self.jobs.queue.is_running(*id)
                        })
                    })
            }) {
                gone.push(*id);
                retained -= 1;
            }
        }
        for id in gone {
            if let Some(batch) = self.jobs.batches.remove(&id) {
                for member in batch.members {
                    if let Some(record) = self.jobs.records.get_mut(&member) {
                        record.batch = None;
                    }
                }
            }
            self.jobs.batch_order.retain(|old| *old != id);
        }
    }
    pub(super) fn batch_retirement_held(&self, id: Uuid) -> bool {
        self.jobs
            .records
            .get(&id)
            .and_then(|r| r.batch)
            .and_then(|batch| self.jobs.batches.get(&batch))
            .is_some_and(|batch| {
                batch.members.iter().any(|id| {
                    self.jobs.records.get(id).is_some_and(|r| {
                        !batch.pending_forms.is_empty()
                            || r.terminal.as_ref().is_none_or(|t| !t.cleanup_complete)
                            || r.retry_pending
                            || self.jobs.queue.is_running(*id)
                    })
                })
            })
    }
    pub(in crate::archive) fn bind_confirmation_forms(
        &mut self,
        forms: Vec<Uuid>,
        cx: &mut Context<Self>,
    ) {
        if forms.len() < 2 {
            return;
        }
        let batch = Uuid::new_v4();
        let requested = forms.len();
        self.jobs.batches.insert(
            batch,
            BatchRecord {
                members: Vec::new(),
                requested,
                refusal: None,
                pending_forms: forms.clone(),
                cancelled_unstarted: 0,
                terminal_logged: false,
            },
        );
        self.jobs.batch_order.push_back(batch);
        for form in forms {
            self.jobs.confirmation_batches.insert(form, batch);
        }
        tracing::info!(event="archive.batch.accepted",batch_id=%batch,requested,accepted=0,"archive confirmation batch awaits explicit forms");
        self.prune_batches();
        let _ = cx;
    }
    pub(super) fn attach_confirmation_jobs(
        &mut self,
        batch: Uuid,
        form: Uuid,
        members: &[Uuid],
        cx: &mut Context<Self>,
    ) {
        if let Some(record) = self.jobs.batches.get_mut(&batch) {
            record.pending_forms.retain(|id| *id != form);
            record.members.extend_from_slice(members);
        }
        for id in members {
            if let Some(record) = self.jobs.records.get_mut(id) {
                record.batch = Some(batch);
            }
        }
        if let Some(id) = members.first() {
            self.refresh_batch_views(*id, cx);
        }
    }
    pub(super) fn cancel_confirmation_member(&mut self, form: Uuid) {
        if let Some(batch) = self.jobs.confirmation_batches.remove(&form)
            && let Some(record) = self.jobs.batches.get_mut(&batch)
        {
            record.pending_forms.retain(|id| *id != form);
            record.cancelled_unstarted += 1;
        }
    }
    pub(in crate::archive) fn refresh_all_batch_views(&self, cx: &mut Context<Self>) {
        for batch in self.jobs.batches.values() {
            if let Some(id) = batch.members.first() {
                self.refresh_batch_views(*id, cx);
            }
        }
    }
    pub(in crate::archive) fn retry_job(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if self.jobs.records.get(&id).is_some_and(retry_facts) {
            self.observe_job(id, true, cx);
        }
    }
    pub(super) fn retire_job_observations(&mut self, id: Uuid) {
        self.jobs
            .observations
            .retain(|_, pending| pending.job.is_none_or(|(job, _, _)| job != id));
        if let Some(record) = self.jobs.records.get_mut(&id) {
            record.retry_pending = false;
        }
    }
    pub(super) fn observe_job(&mut self, id: Uuid, retry: bool, cx: &mut Context<Self>) {
        if !retry
            && (!self.jobs.queue.is_running(id)
                || self
                    .jobs
                    .observations
                    .values()
                    .filter(|p| p.job.is_some())
                    .count()
                    >= 2)
        {
            return;
        }
        if !retry
            && self.jobs.records.get(&id).is_none_or(|r| {
                r.batch.is_none()
                    || r.retry_baseline.is_some()
                    || r.terminal.is_some()
                    || r.active_operation != r.command.operation_id
                    || self.jobs.attempts.get(&r.active_operation) != Some(&id)
            })
        {
            return;
        }
        let Some(record) = self
            .jobs
            .records
            .get(&id)
            .filter(|r| retry_shape(r) && (!retry || retry_facts(r)))
        else {
            return;
        };
        let attempt = record.active_operation;
        let request = record.command.request.as_ref().expect("qualified request");
        let source = request.paths[0].clone();
        let destination = request
            .destination
            .clone()
            .or_else(|| {
                std::path::Path::new(&source)
                    .parent()
                    .map(|p| p.to_string_lossy().into_owned())
            })
            .unwrap_or_default();
        let baseline = retry.then_some(record.retry_baseline.as_ref()).flatten();
        let command = ArchiveSpaceCommand {
            version: 2,
            operation_id: Uuid::new_v4(),
            session_id: id,
            generation: record.command.generation,
            source,
            destination,
            expected_source: baseline
                .and_then(|o| o.source_set.as_ref().map(|s| s.identity.clone())),
            expected_target: baseline.map(|o| o.target_identity.clone()),
        };
        if retry {
            self.jobs
                .records
                .get_mut(&id)
                .expect("record")
                .retry_pending = true;
        }
        self.start_job_observation(command, Some((id, attempt, retry)), None, cx);
        self.refresh_batch_views(id, cx);
    }
    pub(in crate::archive) fn start_job_observation(
        &mut self,
        command: ArchiveSpaceCommand,
        job: Option<(Uuid, Uuid, bool)>,
        response: Option<async_channel::Sender<Result<ArchiveSpaceObservation, String>>>,
        cx: &mut Context<Self>,
    ) {
        let Some(connection) = self.platform.connection_generation() else {
            if let Some(sender) = response {
                let _ = sender.try_send(Err("disconnected".into()));
            }
            if let Some((id, _, true)) = job
                && let Some(record) = self.jobs.records.get_mut(&id)
            {
                record.retry_pending = false;
            }
            return;
        };
        let id = command.operation_id;
        if command.validate().is_err() || self.jobs.observations.len() >= 32 {
            if let Some(sender) = response {
                let _ = sender.try_send(Err("observation_busy".into()));
            }
            if let Some((id, _, true)) = job
                && let Some(record) = self.jobs.records.get_mut(&id)
            {
                record.retry_pending = false;
            }
            return;
        }
        let deadline = cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(7))
                .await;
            let _ = owner.update(cx, |app, cx| {
                app.job_observation_result(
                    id,
                    &nxtools_protocol::PlatformCommandResult {
                        success: false,
                        error: Some("observation_timeout".into()),
                        native_error_code: None,
                        data: None,
                    },
                    cx,
                );
            });
        });
        self.jobs.observations.insert(
            id,
            JobObservation {
                command: command.clone(),
                job,
                response,
                connection,
                _deadline: deadline,
            },
        );
        if self
            .platform
            .send_command_with_id_correlated(PlatformCommand::ArchiveSpace(command), id, id)
            .is_err()
        {
            self.job_observation_result(
                id,
                &nxtools_protocol::PlatformCommandResult {
                    success: false,
                    error: Some("not_submitted".into()),
                    native_error_code: None,
                    data: None,
                },
                cx,
            );
        }
    }
    pub(in crate::archive) fn job_observation_result(
        &mut self,
        id: Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pending) = self.jobs.observations.remove(&id) else {
            return false;
        };
        let observed = result
            .data
            .as_deref()
            .filter(|data| data.len() <= 2 << 20)
            .and_then(|data| serde_json::from_str::<ArchiveSpaceObservation>(data).ok())
            .filter(|o| {
                result.success
                    && self.platform.connection_generation() == Some(pending.connection)
                    && o.validate(&pending.command).is_ok()
                    && o.source_matches
                    && o.source_set.is_some()
            })
            .ok_or_else(|| {
                result
                    .error
                    .clone()
                    .unwrap_or_else(|| "invalid_observation".into())
            });
        if let Some(sender) = pending.response {
            let _ = sender.try_send(observed);
            return true;
        }
        let Some((job, attempt, retry)) = pending.job else {
            return true;
        };
        let native_live = self.jobs.attempts.get(&attempt) == Some(&job);
        let Some(record) = self.jobs.records.get_mut(&job).filter(|r| {
            r.command.generation == pending.command.generation
                && r.active_operation == attempt
                && (retry || (native_live && r.command.operation_id == attempt))
        }) else {
            return true;
        };
        if !retry {
            if record.terminal.is_none() && record.retry_baseline.is_none() {
                record.retry_baseline = observed.ok();
            }
            self.refresh_batch_views(job, cx);
            return true;
        }
        record.retry_pending = false;
        let current = observed.ok().filter(|o| {
            record.retry_baseline.as_ref().is_some_and(|prior| {
                prior.source_set == o.source_set && prior.target_identity == o.target_identity
            })
        });
        if current.is_none() || !retry_facts(record) {
            record.control_status =
                Some("来源/目标已变化或重验未完成；未重试，请重新确认原操作。".into());
            self.refresh_job(job, cx);
            self.prune_retired_jobs();
            return true;
        }
        let request = record.command.request.as_ref().expect("retry shape");
        let memory = request
            .limits
            .max_working_bytes
            .saturating_add(request.limits.max_metadata_bytes)
            .saturating_add(64 << 20);
        if self
            .jobs
            .queue
            .enqueue(
                job,
                Resources {
                    sources: request.paths.clone(),
                    output: request.destination.clone(),
                    memory,
                },
            )
            .is_err()
        {
            record.control_status = Some("原队列容量不足；未提交重试。".into());
            self.refresh_job(job, cx);
            self.prune_retired_jobs();
            return true;
        }
        record.command.operation_id = Uuid::new_v4();
        record.command.generation = record.command.generation.wrapping_add(1).max(1);
        record.active_operation = record.command.operation_id;
        record.terminal = None;
        record.progress = None;
        record.prompt = None;
        record.paused = false;
        record.controls.clear();
        record.control_status = None;
        record.connection = None;
        record.terminal_notified = false;
        record.state = "排队中（显式重试）".into();
        self.jobs.retired.retain(|old| *old != job);
        if let Some(batch) = record.batch.and_then(|id| self.jobs.batches.get_mut(&id)) {
            batch.terminal_logged = false;
        }
        tracing::info!(event="archive.batch.retry_accepted",job_id=%job,attempt=%record.active_operation,generation=record.command.generation,batch_id=?record.batch,"explicit retry revalidated and reentered original queue");
        self.refresh_job(job, cx);
        self.pump_jobs(cx);
        true
    }
}

impl ArchiveApplication {
    pub(super) fn record_batch_terminal(&mut self, id: Uuid) {
        let Some(snapshot) = self.batch_snapshot_for(id) else {
            return;
        };
        let Some(record) = self.jobs.records.get(&id) else {
            return;
        };
        tracing::info!(event="archive.batch.member_finished",batch_id=%snapshot.id,job_id=%id,attempt=%record.active_operation,generation=record.command.generation,elapsed_ms=record.terminal.as_ref().map_or(0,|t|t.elapsed_ms),native_error_code=?record.terminal.as_ref().and_then(|t|t.native_error_code),"archive batch logical member terminal observed");
        let all_terminal = snapshot.pending_forms == 0
            && snapshot.members.iter().all(|m| {
                matches!(
                    m.state,
                    MemberState::Succeeded
                        | MemberState::Warnings
                        | MemberState::Failed
                        | MemberState::Cancelled
                )
            });
        if all_terminal
            && let Some(batch) = self.jobs.batches.get_mut(&snapshot.id)
            && !batch.terminal_logged
        {
            batch.terminal_logged = true;
            let count = |state| snapshot.members.iter().filter(|m| m.state == state).count();
            tracing::info!(event="archive.batch.finished",batch_id=%snapshot.id,jobs=snapshot.members.len(),succeeded=count(MemberState::Succeeded),warnings=count(MemberState::Warnings),failed=count(MemberState::Failed),cancelled=count(MemberState::Cancelled)+snapshot.cancelled_unstarted,not_started=snapshot.not_admitted+snapshot.cancelled_unstarted+snapshot.members.iter().filter(|m|m.never_started).count(),cleanup_unknown=snapshot.members.iter().filter(|m|m.cleanup_unknown).count(),"archive batch terminal counts, not aggregate success");
        }
    }
}
