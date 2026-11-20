//! Foreground receipts for existing archive job and batch surfaces.
use super::*;
use nxtools_protocol::{ActivateWindowCommand, PlatformCommandResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum JobPresentation {
    Show,
    Reuse,
    Completion,
}

pub(super) struct JobActivation {
    receipt: Uuid,
    job: Uuid,
    attempt: Uuid,
    generation: u64,
    window: AnyWindowHandle,
    hwnd: Option<i64>,
    connection: Option<u64>,
    presentation: JobPresentation,
    started: Instant,
}

impl JobActivation {
    fn finish(self, surface: Uuid, outcome: &'static str, native_error_code: Option<i32>) {
        tracing::info!(event="archive.job.activation_finished", correlation_id=%self.receipt, job_id=%self.job,
            surface_id=%surface, attempt=%self.attempt, generation=self.generation,
            presentation=?self.presentation, outcome, native_error_code=?native_error_code,
            elapsed_ms=self.started.elapsed().as_millis() as u64, "archive progress activation terminal");
    }
}

impl ArchiveApplication {
    pub(super) fn job_window_is_current(&self, surface: Uuid, window: AnyWindowHandle) -> bool {
        self.jobs
            .records
            .get(&surface)
            .and_then(|record| record.window.as_ref())
            .is_some_and(|(current, _)| *current == window)
    }

    fn job_activation_is_current(&self, surface: Uuid, pending: &JobActivation) -> bool {
        self.job_window_is_current(surface, pending.window)
            && self.job_surface_id(pending.job) == surface
            && self.job_is_selected(pending.job)
            && self.jobs.records.get(&pending.job).is_some_and(|record| {
                record.active_operation == pending.attempt
                    && record.command.generation == pending.generation
            })
    }

    pub(super) fn activate_job(
        &mut self,
        job: Uuid,
        presentation: JobPresentation,
        cx: &mut Context<Self>,
    ) {
        let surface = self.job_surface_id(job);
        let Some((window, _)) = self
            .jobs
            .records
            .get(&surface)
            .and_then(|r| r.window.as_ref())
        else {
            return;
        };
        let record = &self.jobs.records[&job];
        let pending = JobActivation {
            receipt: Uuid::new_v4(),
            job,
            attempt: record.active_operation,
            generation: record.command.generation,
            window: *window,
            hwnd: None,
            connection: self.platform.connection_generation(),
            presentation,
            started: Instant::now(),
        };
        let receipt = pending.receipt;
        tracing::info!(event="archive.job.activation_accepted", correlation_id=%receipt, job_id=%job,
            surface_id=%surface, attempt=%pending.attempt, generation=pending.generation,
            ?presentation, "archive progress activation accepted");
        if let Some(previous) = self.jobs.activations.insert(surface, pending) {
            previous.finish(surface, "superseded", None);
        }
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.send_job_activation(surface, receipt, cx));
        });
    }

    fn send_job_activation(&mut self, surface: Uuid, receipt: Uuid, cx: &mut Context<Self>) {
        let Some(pending) = self
            .jobs
            .activations
            .get(&surface)
            .filter(|p| p.receipt == receipt)
        else {
            return;
        };
        if !self.job_activation_is_current(surface, pending) {
            self.retire_job_activation(surface, "stale");
            return;
        }
        if pending.connection.is_none()
            || pending.connection != self.platform.connection_generation()
        {
            self.retire_job_activation(surface, "disconnected");
            return;
        }
        let window = pending.window;
        let needs_visibility = pending.presentation == JobPresentation::Completion;
        #[cfg(test)]
        let test_hwnd = self.jobs.activation_test_hwnd;
        let hwnd = window.update(cx, |_, window, _| {
            if needs_visibility && !window.is_visible() {
                return Err("not_visible");
            }
            let hwnd = crate::ui::native_window_handle(window);
            #[cfg(test)]
            let hwnd = hwnd.or(test_hwnd);
            hwnd.ok_or("hwnd_unavailable")
        });
        let window_handle = match hwnd {
            Ok(Ok(hwnd)) => hwnd,
            Ok(Err(outcome)) => {
                self.retire_job_activation(surface, outcome);
                return;
            }
            Err(_) => {
                self.retire_job_activation(surface, "retired");
                return;
            }
        };
        self.jobs
            .activations
            .get_mut(&surface)
            .expect("current activation")
            .hwnd = Some(window_handle);
        tracing::info!(event="archive.job.activation_started", correlation_id=%receipt,
            surface_id=%surface, "archive progress activation sent to native foreground owner");
        if self
            .platform
            .send_command_with_id(
                PlatformCommand::ActivateWindow(ActivateWindowCommand { window_handle }),
                receipt,
            )
            .is_err()
        {
            self.retire_job_activation(surface, "not_submitted");
        }
    }

    pub(super) fn job_activation_result(
        &mut self,
        receipt: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((&surface, _)) = self
            .jobs
            .activations
            .iter()
            .find(|(_, p)| p.receipt == receipt)
        else {
            return false;
        };
        let pending = self
            .jobs
            .activations
            .remove(&surface)
            .expect("matching activation");
        #[cfg(test)]
        let test_hwnd = self.jobs.activation_test_hwnd;
        let current = self.job_activation_is_current(surface, &pending)
            && pending.connection == self.platform.connection_generation()
            && pending.hwnd.is_some_and(|hwnd| {
                pending
                    .window
                    .update(cx, |_, window, _| {
                        let actual = crate::ui::native_window_handle(window);
                        #[cfg(test)]
                        let actual = actual.or(test_hwnd);
                        actual == Some(hwnd)
                    })
                    .unwrap_or(false)
            });
        let (outcome, native_error_code) = if !current {
            ("stale", None)
        } else if result.success {
            ("confirmed", result.native_error_code)
        } else if result.error.as_deref() == Some("platform command timed out") {
            ("timeout", result.native_error_code)
        } else {
            ("native_failure", result.native_error_code)
        };
        pending.finish(surface, outcome, native_error_code);
        true
    }

    pub(super) fn retire_job_activation(&mut self, surface: Uuid, outcome: &'static str) {
        if let Some(pending) = self.jobs.activations.remove(&surface) {
            pending.finish(surface, outcome, None);
        }
    }

    pub(super) fn job_activations_disconnected(&mut self, generation: u64) {
        let surfaces = self
            .jobs
            .activations
            .iter()
            .filter_map(|(&surface, pending)| {
                (pending.connection == Some(generation)).then_some(surface)
            })
            .collect::<Vec<_>>();
        for surface in surfaces {
            self.retire_job_activation(surface, "disconnected");
        }
    }

    pub(super) fn present_job_terminal(&mut self, id: Uuid, current: bool, cx: &mut Context<Self>) {
        let record = &self.jobs.records[&id];
        let batch = record.batch;
        let was_finished = batch
            .and_then(|id| self.jobs.batches.get(&id))
            .is_some_and(|batch| batch.terminal_logged);
        let auto_close = batch.is_none()
            && !record.completion.keep_open
            && record
                .terminal
                .as_ref()
                .is_some_and(|t| t.outcome == ArchiveJobOutcome::Succeeded);
        self.record_batch_terminal(id);
        if auto_close {
            self.retire_job_activation(id, "auto_close");
        } else if current {
            let visible_job = if let Some(batch) = batch {
                self.jobs
                    .batches
                    .get(&batch)
                    .filter(|batch| batch.terminal_logged && !was_finished)
                    .and_then(|batch| batch.selected)
            } else {
                Some(id)
            };
            if let Some(job) = visible_job {
                self.activate_job(job, JobPresentation::Completion, cx);
            }
        }
    }
}
