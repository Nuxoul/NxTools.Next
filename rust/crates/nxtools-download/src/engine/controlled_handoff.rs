//! Durable browser ownership transfer between pending confirmation, held task admission and activation.

use std::sync::atomic::Ordering;
use std::time::Instant;

use tracing::{info, warn};

use super::{
    BrowserDownloadDialogSource, DownloadEngine, DownloadEngineError, EngineTaskState,
    apply_organization_to_request, now_unix_ms,
};
use crate::storage::{ControlledBrowserHandoff, ControlledHandoffAcceptSave, StoredTransferState};
use crate::{
    BrowserHandoffState, DownloadCommandOutcome, DownloadRequest, DownloadResolutionControl,
    DownloadResolutionMode, DownloadResolutionOptions, DownloadStatus, DownloadSubmitMode,
    DownloadTask, DownloadTaskId, RequestOrigin,
};

impl DownloadEngine {
    pub(super) fn prepare_browser_handoff_correlated(
        &self,
        handoff_id: uuid::Uuid,
        request: DownloadRequest,
        open_dialog: bool,
        mode: DownloadSubmitMode,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadCommandOutcome, DownloadEngineError> {
        request.validate()?;
        let fingerprint = super::resolution::resolution_resource_fingerprint(&request)?;
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if open_dialog && !self.inner.browser_dialog_available.load(Ordering::Acquire) {
            let existing = self.inner.store.controlled_browser_handoff_status(
                handoff_id,
                now_unix_ms(),
                false,
            )?;
            if matches!(
                existing.record.state,
                BrowserHandoffState::Missing | BrowserHandoffState::Pending
            ) {
                return Err(DownloadEngineError::BrowserDialogConsumerUnavailable);
            }
            if existing
                .record
                .request_fingerprint
                .as_deref()
                .is_some_and(|stored| stored != fingerprint)
            {
                return Err(DownloadEngineError::BrowserHandoffConflict(handoff_id));
            }
            return Ok(handoff_outcome(&existing.record));
        }
        let prepared = self.inner.store.prepare_controlled_browser_handoff(
            handoff_id,
            &fingerprint,
            now_unix_ms(),
        )?;
        if prepared.expired {
            log_handoff_transition(
                handoff_id,
                BrowserHandoffState::Expired,
                None,
                Instant::now(),
            );
        }
        if !prepared.inserted
            && (!open_dialog || prepared.record.state != BrowserHandoffState::Pending)
        {
            return Ok(handoff_outcome(&prepared.record));
        }
        if prepared.inserted {
            log_handoff_transition(
                handoff_id,
                BrowserHandoffState::Pending,
                None,
                Instant::now(),
            );
        }
        drop(_scheduler);

        if open_dialog {
            let opened = self.open_browser_dialog_source_correlated(
                handoff_id,
                BrowserDownloadDialogSource::ControlledDownload(Box::new(request)),
                correlation_id,
                None,
            );
            if let Err(error) = opened {
                let aborted = self.inner.store.abort_controlled_browser_handoff(
                    handoff_id,
                    None,
                    now_unix_ms(),
                )?;
                if aborted.changed {
                    log_handoff_transition(
                        handoff_id,
                        BrowserHandoffState::Aborted,
                        None,
                        Instant::now(),
                    );
                }
                return Err(error);
            }
            let current = self.inner.store.controlled_browser_handoff_status(
                handoff_id,
                now_unix_ms(),
                false,
            )?;
            if current.expired {
                log_handoff_transition(
                    handoff_id,
                    BrowserHandoffState::Expired,
                    None,
                    Instant::now(),
                );
            }
            return Ok(handoff_outcome(&current.record));
        }

        self.accept_browser_handoff_correlated(
            handoff_id,
            request,
            mode,
            DownloadResolutionOptions {
                operation_id: handoff_id,
                mode: DownloadResolutionMode::Resolve,
            },
            correlation_id,
        )
    }

    pub(super) fn browser_handoff_status_correlated(
        &self,
        handoff_id: uuid::Uuid,
    ) -> Result<DownloadCommandOutcome, DownloadEngineError> {
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        let status =
            self.inner
                .store
                .controlled_browser_handoff_status(handoff_id, now_unix_ms(), true)?;
        if status.expired {
            log_handoff_transition(
                handoff_id,
                BrowserHandoffState::Expired,
                None,
                Instant::now(),
            );
        }
        Ok(handoff_outcome(&status.record))
    }

    pub(super) fn accept_browser_handoff_correlated(
        &self,
        handoff_id: uuid::Uuid,
        mut request: DownloadRequest,
        mode: DownloadSubmitMode,
        resolution: DownloadResolutionOptions,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadCommandOutcome, DownloadEngineError> {
        let initial =
            self.inner
                .store
                .controlled_browser_handoff_status(handoff_id, now_unix_ms(), false)?;
        if initial.expired {
            log_handoff_transition(
                handoff_id,
                BrowserHandoffState::Expired,
                None,
                Instant::now(),
            );
        }
        if initial.record.state != BrowserHandoffState::Pending {
            return Ok(handoff_outcome(&initial.record));
        }
        let identity = super::resolution::resolution_resource_fingerprint(&request)?;
        if initial.record.request_fingerprint.as_deref() != Some(identity.as_str()) {
            return Err(DownloadEngineError::BrowserHandoffConflict(handoff_id));
        }

        let prepared = self.prepare_request_resolution(
            request,
            resolution,
            DownloadResolutionControl::default(),
            correlation_id,
        )?;
        request = prepared.request.clone();
        self.resolve_missing_file_name_with_operation(
            &mut request,
            correlation_id,
            Some(prepared.operation.clone()),
        )?;
        self.check_request_resolution(&prepared)?;

        let started_at = Instant::now();
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        self.inner.check_admission()?;
        self.inner.next_work_revision()?;
        let current =
            self.inner
                .store
                .controlled_browser_handoff_status(handoff_id, now_unix_ms(), false)?;
        if current.expired {
            log_handoff_transition(handoff_id, BrowserHandoffState::Expired, None, started_at);
        }
        if current.record.state != BrowserHandoffState::Pending {
            return Ok(handoff_outcome(&current.record));
        }

        let mut tasks = self.inner.lock_tasks()?;
        apply_organization_to_request(&self.inner, &mut request, &tasks, correlation_id)?;
        request.validate()?;
        let resolution_acceptance = self.lock_resolution_acceptance(&prepared)?;
        let now = now_unix_ms();
        let mut task = DownloadTask::new(request, RequestOrigin::Browser, now);
        task.set_resolution(prepared.provenance.clone());
        let task_id = task.id();
        let transfer = StoredTransferState::default();
        let saved = self.inner.store.accept_controlled_browser_handoff(
            handoff_id,
            &task,
            &transfer,
            "default",
            mode == DownloadSubmitMode::StartNow,
            now,
        )?;
        drop(resolution_acceptance);
        match saved {
            ControlledHandoffAcceptSave::Existing(record) => {
                drop(tasks);
                Ok(handoff_outcome(&record))
            }
            ControlledHandoffAcceptSave::Inserted(saved_task_id) => {
                debug_assert_eq!(saved_task_id, task_id);
                let accepted_revision = self.inner.bump_work_revision();
                tasks.insert(
                    task_id,
                    EngineTaskState {
                        task,
                        transfer,
                        control: None,
                        last_progress_event: None,
                        transfer_rate: None,
                        paused_url_replacement: None,
                        completion_batch_id: None,
                        accepted_revision,
                    },
                );
                drop(tasks);
                self.inner.emit_task(task_id);
                log_handoff_transition(
                    handoff_id,
                    BrowserHandoffState::Held,
                    Some(task_id),
                    started_at,
                );
                Ok(DownloadCommandOutcome::BrowserHandoff {
                    handoff_id,
                    state: BrowserHandoffState::Held,
                    task_id: Some(task_id),
                })
            }
        }
    }

    pub(super) fn commit_browser_handoff_correlated(
        &self,
        handoff_id: uuid::Uuid,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadCommandOutcome, DownloadEngineError> {
        let started_at = Instant::now();
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        let current =
            self.inner
                .store
                .controlled_browser_handoff_status(handoff_id, now_unix_ms(), false)?;
        if let Some(stored_task_id) = current.record.task_id
            && stored_task_id != task_id
        {
            return Err(DownloadEngineError::BrowserHandoffTaskMismatch {
                handoff_id,
                expected: stored_task_id,
                actual: task_id,
            });
        }
        if current.record.state != BrowserHandoffState::Held {
            return Ok(handoff_outcome(&current.record));
        }
        let now = now_unix_ms();
        let mut task = self
            .inner
            .lock_tasks()?
            .get(&task_id)
            .map(|state| state.task.clone())
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        let schedule_due = task
            .request()
            .scheduled_at_unix_ms
            .is_none_or(|scheduled_at| scheduled_at <= now);
        let clear_due_schedule = current.record.start_now == Some(true)
            && task.request().scheduled_at_unix_ms.is_some()
            && schedule_due;
        if clear_due_schedule {
            task.clear_scheduled_start(now);
        }
        let committed = self.inner.store.commit_controlled_browser_handoff(
            handoff_id,
            &task,
            schedule_due,
            now,
        )?;
        if !committed.changed {
            return Ok(handoff_outcome(&committed.record));
        }
        if clear_due_schedule {
            let mut tasks = self.inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            state.task = task.clone();
        }
        self.inner.emit_task(task_id);
        log_handoff_transition(
            handoff_id,
            BrowserHandoffState::Committed,
            Some(task_id),
            started_at,
        );
        drop(_scheduler);
        if committed.queue_eligible {
            if let Err(error) = self.schedule_task_queue(task_id, correlation_id) {
                warn!(
                    event = "download.browser_handoff.activation_deferred",
                    correlation_id = %handoff_id,
                    %handoff_id,
                    ?task_id,
                    fault_code = ?error.fault().code,
                    "committed browser handoff remains eligible for a later queue refill"
                );
            }
        } else if task.request().scheduled_at_unix_ms.is_some() {
            self.refresh_delayed_schedule_wake()?;
        }
        Ok(handoff_outcome(&committed.record))
    }

    pub(super) fn abort_browser_handoff_correlated(
        &self,
        handoff_id: uuid::Uuid,
    ) -> Result<DownloadCommandOutcome, DownloadEngineError> {
        let started_at = Instant::now();
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        let current =
            self.inner
                .store
                .controlled_browser_handoff_status(handoff_id, now_unix_ms(), false)?;
        let mut cancelled_task = None;
        if current.record.state == BrowserHandoffState::Held {
            let task_id = current
                .record
                .task_id
                .ok_or(DownloadEngineError::BrowserHandoffMissingTask(handoff_id))?;
            let mut task = self
                .inner
                .lock_tasks()?
                .get(&task_id)
                .map(|state| state.task.clone())
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            task.transition_to(DownloadStatus::Cancelled, now_unix_ms())?;
            cancelled_task = Some(task);
        }
        let aborted = self.inner.store.abort_controlled_browser_handoff(
            handoff_id,
            cancelled_task.as_ref(),
            now_unix_ms(),
        )?;
        if aborted.expired {
            log_handoff_transition(handoff_id, BrowserHandoffState::Expired, None, started_at);
        }
        if let Some(task_id) = aborted.cancelled_task_id {
            let task =
                cancelled_task.ok_or(DownloadEngineError::BrowserHandoffMissingTask(handoff_id))?;
            let mut tasks = self.inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            state.task = task;
            state.control = None;
            drop(tasks);
            self.inner.bump_work_revision();
            self.inner.emit_task(task_id);
        }
        if aborted.changed {
            log_handoff_transition(
                handoff_id,
                BrowserHandoffState::Aborted,
                aborted.record.task_id,
                started_at,
            );
        }
        Ok(handoff_outcome(&aborted.record))
    }

    pub(super) fn ensure_task_not_held(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<(), DownloadEngineError> {
        if self
            .inner
            .store
            .is_controlled_browser_handoff_held_task(task_id)?
        {
            Err(DownloadEngineError::BrowserHandoffHeld(task_id))
        } else {
            Ok(())
        }
    }
}

fn handoff_outcome(record: &ControlledBrowserHandoff) -> DownloadCommandOutcome {
    DownloadCommandOutcome::BrowserHandoff {
        handoff_id: record.handoff_id,
        state: record.state,
        task_id: record.task_id,
    }
}

fn log_handoff_transition(
    handoff_id: uuid::Uuid,
    state: BrowserHandoffState,
    task_id: Option<DownloadTaskId>,
    started_at: Instant,
) {
    info!(
        event = "download.browser_handoff.lifecycle",
        correlation_id = %handoff_id,
        %handoff_id,
        ?task_id,
        state = ?state,
        elapsed_ms = started_at.elapsed().as_millis(),
        "controlled browser handoff state changed"
    );
}
