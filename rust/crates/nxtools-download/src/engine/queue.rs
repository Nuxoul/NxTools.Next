//! Durable queue order, bounded worker admission and generation-scoped schedule wakes.
//! All mutations use the engine's existing scheduler gate and store-before-memory ordering.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use chrono::{DateTime, Datelike, Days, Local, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use tracing::{info, warn};

use super::{
    DownloadEngine, DownloadEngineError, DownloadScheduleMoment, download_runtime,
    is_active_download_status, now_unix_ms, spawn_download_worker,
};
use crate::{
    DownloadCommandOutcome, DownloadDailySchedule, DownloadEvent, DownloadQueue,
    DownloadQueueAction, DownloadQueuePlacement, DownloadRateLimiter, DownloadSchedule,
    DownloadStatus, DownloadStorageError, DownloadTaskId, DownloadTransferControl, DownloadWeekday,
};

impl DownloadScheduleMoment {
    pub(super) fn now_local() -> Self {
        let now = Local::now();
        Self {
            weekday: weekday_from_chrono(now.weekday()),
            minute_of_day: u16::try_from(now.hour() * 60 + now.minute())
                .expect("local clock minute is within a day"),
        }
    }
}

/// Returns whether a daily start/stop window contains a local weekday and minute. The start
/// endpoint is inclusive and the stop endpoint is exclusive, so an automatic stop configured for
/// 07:30 is applied when the local clock reaches 07:30.
pub fn daily_schedule_window_contains(
    schedule: &DownloadDailySchedule,
    moment: DownloadScheduleMoment,
) -> bool {
    if !schedule.enabled_start || !schedule.enabled_stop {
        return false;
    }
    if schedule.start_minute <= schedule.stop_minute {
        return schedule.days.contains(&moment.weekday)
            && moment.minute_of_day >= schedule.start_minute
            && moment.minute_of_day < schedule.stop_minute;
    }
    if moment.minute_of_day >= schedule.start_minute {
        return schedule.days.contains(&moment.weekday);
    }
    let previous_day = previous_weekday(moment.weekday);
    schedule.days.contains(&previous_day) && moment.minute_of_day < schedule.stop_minute
}

impl DownloadEngine {
    pub fn queue_order(&self, queue_id: &str) -> Result<Vec<DownloadTaskId>, DownloadEngineError> {
        self.inner
            .store
            .load_queue_order(queue_id)
            .map_err(Into::into)
    }

    pub(super) fn run_queue_action(
        &self,
        action: DownloadQueueAction,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadCommandOutcome, DownloadEngineError> {
        let queue = match action {
            DownloadQueueAction::Move {
                queue_id,
                task_ids,
                placement,
            } => {
                let _scheduler = self
                    .inner
                    .scheduler_gate
                    .lock()
                    .map_err(|_| DownloadEngineError::LockPoisoned)?;
                let mut order = self.inner.store.load_queue_order(&queue_id)?;
                // Manual ordering covers every task in the queue (completed,
                // failed, paused, active): the user-arranged order is a
                // persisted preference, not an admission decision. Gating it
                // on admission eligibility made moves on finished tasks fail
                // silently, so the order never stuck across restarts.
                let known: HashSet<_> = order.iter().copied().collect();
                for task_id in &task_ids {
                    if !known.contains(task_id) {
                        return Err(DownloadEngineError::InvalidQueueMove(*task_id));
                    }
                }
                move_queue_items(&mut order, &task_ids, placement);
                self.inner.store.replace_queue_order(&queue_id, &order)?;
                info!(
                    event = "download.queue.reordered",
                    %correlation_id,
                    queue_id,
                    placement = ?placement,
                    target_count = task_ids.len(),
                    "download queue order changed"
                );
                self.queue(&queue_id)?
            }
            DownloadQueueAction::SetCompletionIntention {
                queue_id,
                intention,
                baseline,
            } => {
                let started = Instant::now();
                let _scheduler = self
                    .inner
                    .scheduler_gate
                    .lock()
                    .map_err(|_| DownloadEngineError::LockPoisoned)?;
                let result = self
                    .inner
                    .store
                    .set_queue_completion_intention(&queue_id, intention, baseline);
                match result {
                    Ok(queue) => {
                        info!(event = "download.queue.intention_accepted", %correlation_id,
                            generation = self.inner.session_generation, queue_id,
                            action = ?queue.completion_intention.action, open_directory = queue.completion_intention.open_directory,
                            elapsed_ms = started.elapsed().as_millis(), "queue intention persisted");
                        queue
                    }
                    Err(error) => {
                        warn!(event = "download.queue.intention_failed", %correlation_id,
                            generation = self.inner.session_generation, queue_id,
                            elapsed_ms = started.elapsed().as_millis(), "queue intention not persisted");
                        return Err(error.into());
                    }
                }
            }
            DownloadQueueAction::AssignTasks { queue_id, task_ids } => {
                let mut scheduler = self
                    .inner
                    .scheduler_gate
                    .lock()
                    .map_err(|_| DownloadEngineError::LockPoisoned)?;
                self.inner.check_admission()?;
                if !task_ids.is_empty() {
                    self.inner.next_work_revision()?;
                }
                {
                    let tasks = self.inner.lock_tasks()?;
                    let mut sources = HashSet::new();
                    for id in &task_ids {
                        let state = tasks
                            .get(id)
                            .ok_or(DownloadEngineError::TaskNotFound(*id))?;
                        if is_active_download_status(state.task.status()) {
                            return Err(DownloadEngineError::InvalidAction {
                                task_id: *id,
                                action: "assign_queue",
                                status: state.task.status(),
                            });
                        }
                        let source = self.inner.store.queue_id_for_task(*id)?;
                        if source != queue_id {
                            sources.insert(source);
                        }
                    }
                    let moved = self.inner.store.assign_queue_tasks(&queue_id, &task_ids)?;
                    if moved != 0 {
                        let revision = self.inner.bump_work_revision();
                        for source in sources {
                            scheduler.insert(source, revision);
                        }
                    }
                }
                let queue = self.queue(&queue_id)?;
                if let Err(error) = self.schedule_queue_locked(&queue_id, correlation_id) {
                    warn!(event = "download.queue.refill_deferred", %correlation_id,
                        queue_id, error = %error, "assigned queue remains eligible for refill");
                }
                queue
            }
            DownloadQueueAction::SetMaxConcurrent {
                queue_id,
                max_concurrent_tasks,
            } => {
                if max_concurrent_tasks == 0 {
                    return Err(DownloadEngineError::InvalidQueueConcurrency);
                }
                let queue = self
                    .inner
                    .store
                    .set_queue_max_concurrent(&queue_id, max_concurrent_tasks)?;
                if let Err(error) = self.schedule_queue(&queue_id, correlation_id) {
                    warn!(
                        event = "download.queue.refill_deferred",
                        %correlation_id,
                        queue_id,
                        error = %error,
                        "queue concurrency was committed; refill remains eligible for retry"
                    );
                }
                info!(
                    event = "download.queue.concurrency_changed",
                    %correlation_id,
                    queue_id,
                    max_concurrent_tasks,
                    "download queue concurrency changed"
                );
                queue
            }
            DownloadQueueAction::SetSchedule { queue_id, schedule } => {
                self.set_queue_schedule_correlated(&queue_id, schedule, correlation_id)?
            }
        };
        let sequence = self.inner.event_sequencer.current();
        self.inner.emit(DownloadEvent::SnapshotRequired {
            expected_sequence: sequence,
            observed_sequence: sequence,
            expected_generation: Some(self.inner.session_generation),
            observed_generation: Some(self.inner.session_generation),
        });
        Ok(DownloadCommandOutcome::QueueChanged { queue })
    }

    fn queue(&self, queue_id: &str) -> Result<DownloadQueue, DownloadEngineError> {
        self.inner
            .store
            .load_queues()?
            .into_iter()
            .find(|queue| queue.id == queue_id)
            .ok_or_else(|| DownloadStorageError::QueueNotFound(queue_id.to_owned()).into())
    }

    pub fn set_queue_schedule(
        &self,
        queue_id: &str,
        schedule: Option<DownloadSchedule>,
    ) -> Result<DownloadQueue, DownloadEngineError> {
        self.set_queue_schedule_correlated(queue_id, schedule, uuid::Uuid::new_v4())
    }

    fn set_queue_schedule_correlated(
        &self,
        queue_id: &str,
        schedule: Option<DownloadSchedule>,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadQueue, DownloadEngineError> {
        let started_at = Instant::now();
        if let Some(daily) = schedule.as_ref().map(|schedule| &schedule.daily) {
            daily.validate().map_err(DownloadEngineError::Schedule)?;
        }
        let queue = self
            .inner
            .store
            .set_queue_schedule(queue_id, schedule.as_ref())?;
        info!(
            event = "download.schedule.daily_changed",
            %correlation_id,
            queue_id,
            enabled = queue.schedule.is_some(),
            "daily queue schedule changed"
        );
        if let Err(error) = self.process_daily_schedules_at_correlated(
            DownloadScheduleMoment::now_local(),
            correlation_id,
        ) {
            warn!(
                event = "download.schedule.daily_change_failed",
                %correlation_id,
                queue_id,
                committed = true,
                elapsed_ms = started_at.elapsed().as_millis(),
                error = %error,
                "daily queue schedule was persisted but runtime reconciliation failed"
            );
            return Err(error);
        }
        let queue = match self.queue(queue_id) {
            Ok(queue) => queue,
            Err(error) => {
                warn!(
                    event = "download.schedule.daily_change_failed",
                    %correlation_id,
                    queue_id,
                    stage = "reload_queue",
                    committed = true,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    error = %error,
                    "daily queue schedule was persisted but its resulting snapshot could not be loaded"
                );
                return Err(error);
            }
        };
        info!(
            event = "download.schedule.daily_change_completed",
            %correlation_id,
            queue_id,
            elapsed_ms = started_at.elapsed().as_millis(),
            "daily queue schedule change completed"
        );
        Ok(queue)
    }

    /// Reconciles all persisted daily queue schedules at a supplied local wall-clock moment.
    /// The public seam makes cross-midnight behavior deterministic in contract tests.
    pub fn process_daily_schedules_at(
        &self,
        moment: DownloadScheduleMoment,
    ) -> Result<usize, DownloadEngineError> {
        self.process_daily_schedules_at_correlated(moment, uuid::Uuid::new_v4())
    }

    fn process_daily_schedules_at_correlated(
        &self,
        moment: DownloadScheduleMoment,
        correlation_id: uuid::Uuid,
    ) -> Result<usize, DownloadEngineError> {
        let started_at = Instant::now();
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if self.inner.draining.load(Ordering::SeqCst) {
            return Ok(0);
        }
        let mut changed_count = 0_usize;
        for queue in self.inner.store.load_queues()? {
            let Some(daily) = queue.schedule.as_ref() else {
                continue;
            };
            let daily = &daily.daily;
            daily.validate().map_err(DownloadEngineError::Schedule)?;
            let desired_running = daily_schedule_desired_running(daily, moment);
            let Some(desired_running) = desired_running else {
                continue;
            };
            if desired_running == queue.is_running {
                if desired_running {
                    self.start_daily_queue_locked(&queue.id, correlation_id)?;
                }
                continue;
            }
            self.inner
                .store
                .set_queue_running(&queue.id, desired_running)?;
            if desired_running {
                self.start_daily_queue_locked(&queue.id, correlation_id)?;
            } else {
                self.pause_daily_queue_locked(&queue.id)?;
            }
            changed_count = changed_count.saturating_add(1);
            info!(
                event = "download.schedule.daily_queue_transition",
                %correlation_id,
                queue_id = queue.id,
                running = desired_running,
                minute_of_day = moment.minute_of_day,
                weekday = ?moment.weekday,
                "daily queue scheduling state changed"
            );
        }
        drop(_scheduler);
        self.refresh_daily_schedule_wake()?;
        info!(
            event = "download.schedule.daily_reconcile_completed",
            %correlation_id,
            changed_count,
            elapsed_ms = started_at.elapsed().as_millis(),
            "daily queue schedule reconciliation completed"
        );
        Ok(changed_count)
    }

    fn activate_daily_queue_locked(&self, queue_id: &str) -> Result<(), DownloadEngineError> {
        let queue_task_ids = self.inner.store.load_queue_order(queue_id)?;
        let tasks = self.inner.lock_tasks()?;
        for task_id in queue_task_ids {
            let Some(state) = tasks.get(&task_id) else {
                continue;
            };
            if !matches!(
                state.task.status(),
                DownloadStatus::Created | DownloadStatus::Paused
            ) || self.inner.store.is_queue_task_user_paused(task_id)?
            {
                continue;
            }
            if self
                .inner
                .store
                .is_controlled_browser_handoff_held_task(task_id)?
            {
                continue;
            }
            self.inner.store.set_queue_task_eligible(task_id, true)?;
        }
        Ok(())
    }

    fn start_daily_queue_locked(
        &self,
        queue_id: &str,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        self.activate_daily_queue_locked(queue_id)?;
        if let Err(error) = self.schedule_queue_locked(queue_id, correlation_id) {
            warn!(
                event = "download.schedule.daily_start_admission_deferred",
                %correlation_id,
                queue_id,
                error = %error,
                "open daily queue retains eligible work for a later refill"
            );
        }
        Ok(())
    }

    fn pause_daily_queue_locked(&self, queue_id: &str) -> Result<(), DownloadEngineError> {
        let queue_task_ids = self.inner.store.load_queue_order(queue_id)?;
        let tasks = self.inner.lock_tasks()?;
        for task_id in queue_task_ids {
            let Some(state) = tasks.get(&task_id) else {
                continue;
            };
            if !is_active_download_status(state.task.status()) {
                continue;
            }
            self.inner.store.set_queue_task_eligible(task_id, false)?;
            state
                .control
                .as_ref()
                .ok_or(DownloadEngineError::MissingTransferControl(task_id))?
                .pause();
        }
        Ok(())
    }

    pub(super) fn refresh_daily_schedule_wake(&self) -> Result<(), DownloadEngineError> {
        let queues = self.inner.store.load_queues()?;
        let generation = self
            .inner
            .daily_schedule_wake_generation
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        let now = Local::now();
        let Some(next_trigger) = next_daily_schedule_trigger(&queues, now) else {
            return Ok(());
        };
        let wait_ms = next_trigger
            .timestamp_millis()
            .saturating_sub(now.timestamp_millis())
            .max(1);
        let engine = self.clone();
        download_runtime()?.spawn(async move {
            tokio::time::sleep(Duration::from_millis(
                u64::try_from(wait_ms).unwrap_or(60_000),
            ))
            .await;
            if engine
                .inner
                .daily_schedule_wake_generation
                .load(Ordering::SeqCst)
                != generation
            {
                return;
            }
            let correlation_id = uuid::Uuid::new_v4();
            let [scheduled_moment, current_moment] =
                daily_schedule_wake_moments(next_trigger, Local::now());
            let scheduled_result =
                engine.process_daily_schedules_at_correlated(scheduled_moment, correlation_id);
            let current_result =
                engine.process_daily_schedules_at_correlated(current_moment, correlation_id);
            if let Err(error) = scheduled_result.and(current_result) {
                warn!(
                    event = "download.schedule.daily_wake_failed",
                    %correlation_id,
                    error = %error,
                    "daily queue schedule wake failed"
                );
            }
        });
        Ok(())
    }

    pub(super) fn schedule_queue(
        &self,
        queue_id: &str,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        self.schedule_queue_locked(queue_id, correlation_id)
    }

    pub(super) fn schedule_queue_locked(
        &self,
        queue_id: &str,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let queue = self.queue(queue_id)?;
        if self.inner.draining.load(Ordering::SeqCst) {
            return Ok(());
        }
        if !queue.is_running {
            return Ok(());
        }
        let limit = usize::from(queue.max_concurrent_tasks.unwrap_or(3).max(1));
        let queue_task_ids = self
            .inner
            .store
            .load_queue_order(queue_id)?
            .into_iter()
            .collect::<HashSet<_>>();
        let active_count = {
            let tasks = self.inner.lock_tasks()?;
            tasks
                .values()
                .filter(|state| {
                    queue_task_ids.contains(&state.task.id())
                        && is_active_download_status(state.task.status())
                })
                .count()
        };
        let available_slots = limit.saturating_sub(active_count);
        if available_slots == 0 {
            return Ok(());
        }
        download_runtime()?;

        let candidates = self.inner.store.load_eligible_queue_order(queue_id)?;
        let mut admitted = 0_usize;
        for task_id in candidates {
            if admitted >= available_slots {
                break;
            }
            let restored_segment_bytes =
                self.inner.store.load_transfer_plan(task_id)?.map(|plan| {
                    plan.segments()
                        .iter()
                        .map(crate::DownloadSegment::downloaded_bytes)
                        .sum()
                });
            let changed_task = {
                let mut tasks = self.inner.lock_tasks()?;
                let Some(state) = tasks.get_mut(&task_id) else {
                    continue;
                };
                if !matches!(
                    state.task.status(),
                    DownloadStatus::Created | DownloadStatus::Paused | DownloadStatus::Failed
                ) {
                    continue;
                }
                let mut committed = state.clone();
                self.inner.next_work_revision()?;
                if committed.task.completion_attempt() == 0 {
                    committed.task.set_completion_attempt(1);
                }
                let next = if committed.task.status() == DownloadStatus::Created {
                    DownloadStatus::Probing
                } else {
                    DownloadStatus::Waiting
                };
                committed.task.transition_to(next, now_unix_ms())?;
                if committed.transfer.prepared_size_bytes.is_none() {
                    committed.transfer.downloaded_bytes = restored_segment_bytes.unwrap_or(0);
                }
                committed.transfer.error = None;
                committed.control = Some(DownloadTransferControl::with_rate_limiters(
                    self.inner.global_rate_limiter.clone(),
                    DownloadRateLimiter::new(
                        committed.task.request().speed_limit_bytes_per_second,
                    )?,
                ));
                if let Some(control) = committed.control.as_mut() {
                    control.statistics = crate::transfer_statistics::TransferStatistics::new(
                        committed.task.transfer_statistics(),
                    );
                }
                committed.last_progress_event = None;
                self.inner.store.save_task_with_transfer_and_eligibility(
                    &committed.task,
                    &committed.transfer,
                    false,
                )?;
                self.inner.bump_work_revision();
                *state = committed;
                state.task.clone()
            };
            self.inner.emit(DownloadEvent::TaskChanged {
                task: Box::new(changed_task),
            });
            spawn_download_worker(
                Arc::clone(&self.inner),
                queue_id.to_owned(),
                task_id,
                correlation_id,
            )?;
            admitted += 1;
        }
        info!(
            event = "download.queue.admitted",
            %correlation_id,
            queue_id,
            admitted,
            active_before = active_count,
            max_concurrent_tasks = limit,
            "eligible download tasks admitted from queue"
        );
        Ok(())
    }

    /// Releases delayed submissions whose absolute schedule has elapsed.
    ///
    /// This is a public engine boundary so the scheduler can be tested deterministically; the
    /// normal runtime wake-up invokes the same method without polling from the UI.
    pub fn process_due_schedules_at(&self, now_unix_ms: i64) -> Result<usize, DownloadEngineError> {
        self.process_due_schedules_at_correlated(now_unix_ms, uuid::Uuid::new_v4())
    }

    fn process_due_schedules_at_correlated(
        &self,
        now_unix_ms: i64,
        correlation_id: uuid::Uuid,
    ) -> Result<usize, DownloadEngineError> {
        let started_at = Instant::now();
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        let mut released_task_ids = Vec::new();
        if self.inner.draining.load(Ordering::SeqCst) {
            return Ok(0);
        }
        {
            let mut tasks = self.inner.lock_tasks()?;
            for (task_id, state) in tasks.iter_mut() {
                let Some(scheduled_at) = state.task.request().scheduled_at_unix_ms else {
                    continue;
                };
                if scheduled_at > now_unix_ms || state.task.status() != DownloadStatus::Created {
                    continue;
                }
                if self.inner.store.is_queue_task_eligible(*task_id)? {
                    continue;
                }
                if self
                    .inner
                    .store
                    .is_controlled_browser_handoff_held_task(*task_id)?
                {
                    continue;
                }
                if self
                    .inner
                    .store
                    .controlled_browser_handoff_start_now(*task_id)?
                    == Some(false)
                {
                    continue;
                }
                let mut committed = state.clone();
                committed.task.clear_scheduled_start(now_unix_ms);
                self.inner.store.save_task_with_transfer_and_eligibility(
                    &committed.task,
                    &committed.transfer,
                    true,
                )?;
                *state = committed;
                released_task_ids.push(*task_id);
            }
        }
        for task_id in &released_task_ids {
            self.inner.emit_task(*task_id);
            info!(
                event = "download.schedule.delayed_released",
                %correlation_id,
                ?task_id,
                "delayed download became queue-eligible"
            );
        }
        for queue in self.inner.store.load_queues()? {
            if let Err(error) = self.schedule_queue_locked(&queue.id, correlation_id) {
                warn!(event = "download.schedule.delayed_admission_deferred", %correlation_id,
                    queue_id = queue.id, released_count = released_task_ids.len(),
                    error = %error, "due delayed downloads remain eligible for later refill");
            }
        }
        drop(_scheduler);
        self.refresh_delayed_schedule_wake()?;
        info!(
            event = "download.schedule.delayed_reconcile_completed",
            %correlation_id,
            released_count = released_task_ids.len(),
            elapsed_ms = started_at.elapsed().as_millis(),
            "delayed download reconciliation completed"
        );
        Ok(released_task_ids.len())
    }

    pub(super) fn refresh_delayed_schedule_wake(&self) -> Result<(), DownloadEngineError> {
        let now = now_unix_ms();
        let next_due = self
            .inner
            .lock_tasks()?
            .values()
            .filter_map(|state| state.task.request().scheduled_at_unix_ms)
            .filter(|scheduled_at| *scheduled_at > now)
            .min();
        let generation = self
            .inner
            .schedule_wake_generation
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        let Some(next_due) = next_due else {
            return Ok(());
        };
        let wait_ms = u64::try_from(next_due.saturating_sub(now))
            .unwrap_or_default()
            .max(1);
        let engine = self.clone();
        download_runtime()?.spawn(async move {
            tokio::time::sleep(Duration::from_millis(wait_ms)).await;
            if engine.inner.schedule_wake_generation.load(Ordering::SeqCst) != generation {
                return;
            }
            let correlation_id = uuid::Uuid::new_v4();
            if let Err(error) =
                engine.process_due_schedules_at_correlated(now_unix_ms(), correlation_id)
            {
                warn!(
                    event = "download.schedule.delayed_wake_failed",
                    %correlation_id,
                    error = %error,
                    "delayed schedule wake failed"
                );
            }
        });
        Ok(())
    }
}

fn move_queue_items(
    order: &mut Vec<DownloadTaskId>,
    task_ids: &[DownloadTaskId],
    placement: DownloadQueuePlacement,
) {
    let selected = task_ids.iter().copied().collect::<HashSet<_>>();
    match placement {
        DownloadQueuePlacement::Top | DownloadQueuePlacement::Bottom => {
            let (mut moved, mut retained): (Vec<_>, Vec<_>) = order
                .iter()
                .copied()
                .partition(|task_id| selected.contains(task_id));
            if placement == DownloadQueuePlacement::Top {
                moved.append(&mut retained);
                *order = moved;
            } else {
                retained.append(&mut moved);
                *order = retained;
            }
        }
        DownloadQueuePlacement::Up => {
            for index in 1..order.len() {
                if selected.contains(&order[index]) && !selected.contains(&order[index - 1]) {
                    order.swap(index - 1, index);
                }
            }
        }
        DownloadQueuePlacement::Down => {
            for index in (0..order.len().saturating_sub(1)).rev() {
                if selected.contains(&order[index]) && !selected.contains(&order[index + 1]) {
                    order.swap(index, index + 1);
                }
            }
        }
    }
}

pub(super) fn daily_schedule_desired_running(
    schedule: &DownloadDailySchedule,
    moment: DownloadScheduleMoment,
) -> Option<bool> {
    if schedule.enabled_start && schedule.enabled_stop {
        return Some(daily_schedule_window_contains(schedule, moment));
    }
    if !schedule.days.contains(&moment.weekday) {
        return None;
    }
    if schedule.enabled_start && moment.minute_of_day == schedule.start_minute {
        return Some(true);
    }
    if schedule.enabled_stop && moment.minute_of_day == schedule.stop_minute {
        return Some(false);
    }
    None
}

pub(super) fn next_daily_schedule_trigger(
    queues: &[DownloadQueue],
    now: DateTime<Local>,
) -> Option<DateTime<Local>> {
    queues
        .iter()
        .filter_map(|queue| queue.schedule.as_ref())
        .flat_map(|schedule| {
            let daily = &schedule.daily;
            [
                daily.enabled_start.then_some((daily.start_minute, false)),
                daily.enabled_stop.then_some((daily.stop_minute, true)),
            ]
            .into_iter()
            .flatten()
            .flat_map(move |(minute, is_stop)| {
                (0_u64..=7).filter_map(move |days_ahead| {
                    let date = now.date_naive().checked_add_days(Days::new(days_ahead))?;
                    let weekday = weekday_from_chrono(date.weekday());
                    let schedule_day =
                        if is_stop && daily.enabled_start && daily.start_minute > daily.stop_minute
                        {
                            previous_weekday(weekday)
                        } else {
                            weekday
                        };
                    if !daily.days.contains(&schedule_day) {
                        return None;
                    }
                    let time =
                        NaiveTime::from_hms_opt(u32::from(minute / 60), u32::from(minute % 60), 0)?;
                    let local = Local
                        .from_local_datetime(&NaiveDateTime::new(date, time))
                        .earliest()?;
                    (local > now).then_some(local)
                })
            })
        })
        .min()
}

pub(super) fn daily_schedule_wake_moments(
    scheduled: DateTime<Local>,
    current: DateTime<Local>,
) -> [DownloadScheduleMoment; 2] {
    [scheduled, current].map(|moment| DownloadScheduleMoment {
        weekday: weekday_from_chrono(moment.weekday()),
        minute_of_day: u16::try_from(moment.hour() * 60 + moment.minute())
            .expect("local clock minute is within a day"),
    })
}

fn weekday_from_chrono(weekday: chrono::Weekday) -> DownloadWeekday {
    match weekday {
        chrono::Weekday::Mon => DownloadWeekday::Monday,
        chrono::Weekday::Tue => DownloadWeekday::Tuesday,
        chrono::Weekday::Wed => DownloadWeekday::Wednesday,
        chrono::Weekday::Thu => DownloadWeekday::Thursday,
        chrono::Weekday::Fri => DownloadWeekday::Friday,
        chrono::Weekday::Sat => DownloadWeekday::Saturday,
        chrono::Weekday::Sun => DownloadWeekday::Sunday,
    }
}

fn previous_weekday(weekday: DownloadWeekday) -> DownloadWeekday {
    match weekday {
        DownloadWeekday::Monday => DownloadWeekday::Sunday,
        DownloadWeekday::Tuesday => DownloadWeekday::Monday,
        DownloadWeekday::Wednesday => DownloadWeekday::Tuesday,
        DownloadWeekday::Thursday => DownloadWeekday::Wednesday,
        DownloadWeekday::Friday => DownloadWeekday::Thursday,
        DownloadWeekday::Saturday => DownloadWeekday::Friday,
        DownloadWeekday::Sunday => DownloadWeekday::Saturday,
    }
}
