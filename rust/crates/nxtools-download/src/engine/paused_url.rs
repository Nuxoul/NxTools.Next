use super::*;
use crate::{DownloadPausedUrlReplacement, DownloadResolutionControl, DownloadResolutionOperation};

#[derive(Clone)]
pub(super) struct PausedUrlReplacement {
    operation: DownloadResolutionOperation,
    revision: u64,
    pub(super) active: bool,
}

impl EngineTaskState {
    pub(super) fn cancel_paused_url_candidate(&self) {
        if let Some(candidate) = &self.paused_url_replacement
            && candidate.active
        {
            candidate.operation.control.cancel();
        }
    }
}

impl DownloadEngine {
    pub fn paused_url_replacement_revision(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<u64, DownloadEngineError> {
        let tasks = self.inner.lock_tasks()?;
        let state = tasks
            .get(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        Ok(state
            .paused_url_replacement
            .as_ref()
            .map_or(0, |candidate| candidate.revision))
    }

    pub fn replace_paused_url_with_control(
        &self,
        replacement: DownloadPausedUrlReplacement,
        correlation_id: uuid::Uuid,
        control: DownloadResolutionControl,
    ) -> Result<(), DownloadEngineError> {
        let started = Instant::now();
        let task_id = replacement.task_id;
        let operation_id = replacement.operation_id;
        info!(
            event = "download.paused_url.requested",
            %correlation_id, %operation_id, ?task_id,
            generation = replacement.generation,
            "paused download source replacement requested"
        );
        let result = self.replace_paused_url_candidate(replacement, correlation_id, control);
        match &result {
            Ok(()) => info!(
                event = "download.paused_url.completed",
                %correlation_id, %operation_id, ?task_id,
                elapsed_ms = started.elapsed().as_millis(),
                "paused download source replacement committed"
            ),
            Err(error) => warn!(
                event = "download.paused_url.failed",
                %correlation_id, %operation_id, ?task_id,
                fault_code = ?error.fault().code,
                native_error_code = match error {
                    DownloadEngineError::Storage(DownloadStorageError::Sqlite(rusqlite::Error::SqliteFailure(native, _))) => Some(native.extended_code),
                    _ => error.io_error().and_then(std::io::Error::raw_os_error),
                },
                elapsed_ms = started.elapsed().as_millis(),
                "paused download source replacement did not commit"
            ),
        }
        result
    }

    fn replace_paused_url_candidate(
        &self,
        replacement: DownloadPausedUrlReplacement,
        correlation_id: uuid::Uuid,
        control: DownloadResolutionControl,
    ) -> Result<(), DownloadEngineError> {
        let task_id = replacement.task_id;
        let operation = DownloadResolutionOperation {
            operation_id: replacement.operation_id,
            correlation_id,
            session_generation: self.session_generation(),
            deadline: Instant::now() + control.timeout(),
            control,
        };
        operation.check()?;
        let (retained_request, candidate_request, plan) = {
            let _scheduler = self
                .inner
                .scheduler_gate
                .lock()
                .map_err(|_| DownloadEngineError::LockPoisoned)?;
            let mut tasks = self.inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            self.require_explicit_paused_source(state)?;
            let revision = state
                .paused_url_replacement
                .as_ref()
                .map_or(0, |candidate| candidate.revision);
            if replacement.generation != self.session_generation()
                || replacement.operation_id.is_nil()
                || replacement.expected_revision != revision
                || replacement.expected_source_url != state.task.request().source_url
                || replacement.source_url == replacement.expected_source_url
                || state
                    .paused_url_replacement
                    .as_ref()
                    .is_some_and(|candidate| {
                        candidate.active
                            || candidate.operation.operation_id == replacement.operation_id
                    })
            {
                return Err(DownloadEngineError::InvalidAction {
                    task_id,
                    action: "replace_paused_url",
                    status: state.task.status(),
                });
            }
            let retained = state.task.request().clone();
            let mut request = retained.clone();
            request.source_url = replacement.source_url;
            request.validate()?;
            let plan = self
                .inner
                .store
                .load_transfer_plan(task_id)?
                .ok_or(DownloadEngineError::ResourceChanged(task_id))?;
            let revision = revision
                .checked_add(1)
                .ok_or(DownloadEngineError::InvalidAction {
                    task_id,
                    action: "replace_paused_url",
                    status: state.task.status(),
                })?;
            state.paused_url_replacement = Some(PausedUrlReplacement {
                operation: operation.clone(),
                revision,
                active: true,
            });
            (retained, request, plan)
        };
        info!(
            event = "download.paused_url.started",
            %correlation_id, operation_id = %operation.operation_id, ?task_id,
            "paused download candidate probe started"
        );
        let result = (|| -> Result<(), DownloadEngineError> {
            let probe = self.probe_effective_blocking(
                candidate_request.clone(),
                correlation_id,
                Some(operation.clone()),
            )?;
            operation.check()?;
            if plan.validator().if_range().is_none()
                || probe.media.is_some()
                || !plan.supports_ranges()
                || !probe.supports_ranges
                || plan.total_bytes().is_none()
                || plan.total_bytes() != probe.total_bytes
                || plan.validator() != &probe.validator
            {
                return Err(DownloadEngineError::ResourceChanged(task_id));
            }
            let updated_plan = TransferPlan::new(
                plan.protocol(),
                probe.final_url.clone(),
                plan.total_bytes(),
                true,
                plan.validator().clone(),
                plan.segments().to_vec(),
            )?;
            {
                let _scheduler = self
                    .inner
                    .scheduler_gate
                    .lock()
                    .map_err(|_| DownloadEngineError::LockPoisoned)?;
                let mut tasks = self.inner.lock_tasks()?;
                let state = tasks
                    .get_mut(&task_id)
                    .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
                self.require_explicit_paused_source(state)?;
                if state.task.request() != &retained_request
                    || !state
                        .paused_url_replacement
                        .as_ref()
                        .is_some_and(|candidate| {
                            candidate.active
                                && candidate.operation.operation_id == operation.operation_id
                        })
                {
                    return Err(DownloadEngineError::InvalidAction {
                        task_id,
                        action: "commit_replaced_url",
                        status: state.task.status(),
                    });
                }
                operation.check()?;
                let mut committed = state.clone();
                committed
                    .task
                    .set_source_url(candidate_request.source_url, now_unix_ms());
                committed.transfer.probe = Some(probe);
                self.inner.store.save_paused_url_replacement(
                    &committed.task,
                    &committed.transfer,
                    &updated_plan,
                )?;
                *state = committed;
            }
            self.inner.emit_task(task_id);
            Ok(())
        })();
        if let Ok(mut tasks) = self.inner.lock_tasks()
            && let Some(state) = tasks.get_mut(&task_id)
            && let Some(candidate) = state.paused_url_replacement.as_mut()
            && candidate.operation.operation_id == operation.operation_id
        {
            candidate.active = false;
        }
        result
    }

    fn require_explicit_paused_source(
        &self,
        state: &EngineTaskState,
    ) -> Result<(), DownloadEngineError> {
        let task_id = state.task.id();
        if state.task.status() != DownloadStatus::Paused
            || state.control.is_some()
            || state.task.request().media.is_some()
            || self.inner.store.is_queue_task_eligible(task_id)?
            || !self.inner.store.is_queue_task_user_paused(task_id)?
        {
            return Err(DownloadEngineError::InvalidAction {
                task_id,
                action: "replace_paused_url",
                status: state.task.status(),
            });
        }
        Ok(())
    }

    pub(super) fn cancel_paused_url_replacement(
        &self,
        task_id: DownloadTaskId,
        operation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let tasks = self.inner.lock_tasks()?;
        let state = tasks
            .get(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        let candidate = state
            .paused_url_replacement
            .as_ref()
            .filter(|candidate| {
                candidate.active && candidate.operation.operation_id == operation_id
            })
            .ok_or(DownloadEngineError::InvalidAction {
                task_id,
                action: "cancel_replaced_url",
                status: state.task.status(),
            })?;
        candidate.operation.control.cancel();
        info!(
            event = "download.paused_url.cancel_requested",
            correlation_id = %candidate.operation.correlation_id, %operation_id, ?task_id,
            "paused download candidate cancellation requested"
        );
        Ok(())
    }
}
