use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, Days, Local, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use thiserror::Error;
use tracing::{info, warn};

use crate::capacity::{StorageCapacityProbe, SystemStorageCapacityProbe};
use crate::http::{
    PreparedDownload, PreparedDownloadOutcome, is_disk_full_error, is_file_in_use_error,
};
use crate::storage::StoredTransferState;
use crate::{
    ArtifactAllocation, DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadAggregateMetrics,
    DownloadBatchAction, DownloadBatchItemResult, DownloadCommand, DownloadCommandEnvelope,
    DownloadCommandOutcome, DownloadCommandResult, DownloadDailySchedule, DownloadEvent,
    DownloadEventEnvelope, DownloadEventSequencer, DownloadFault, DownloadFaultCode, DownloadProbe,
    DownloadProgress, DownloadQuery, DownloadQueue, DownloadQueueAction, DownloadQueuePlacement,
    DownloadRateLimitError, DownloadRateLimiter, DownloadRequest, DownloadRequestError,
    DownloadSchedule, DownloadSegment, DownloadSegmentError, DownloadSegmentProgress,
    DownloadSegmentState, DownloadSegmentTransferControl, DownloadSegmentTransferOutcome,
    DownloadSnapshot, DownloadStatus, DownloadStorageCapacityError, DownloadStorageError,
    DownloadStorageRequirement, DownloadSubmitMode, DownloadTask, DownloadTaskAction,
    DownloadTaskDetail, DownloadTaskError, DownloadTaskId, DownloadTransferContext,
    DownloadTransferControl, DownloadWeekday, HttpDownloadClient, HttpDownloadError, RequestOrigin,
    SegmentedArtifact, SqliteDownloadStore, TransferPlan, TransferPlanError, TransferProtocol,
};

static DOWNLOAD_RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

#[derive(Clone)]
pub struct DownloadEngine {
    inner: Arc<DownloadEngineInner>,
}

struct DownloadEngineInner {
    http: HttpDownloadClient,
    store: SqliteDownloadStore,
    global_rate_limiter: DownloadRateLimiter,
    capacity_probe: Arc<dyn StorageCapacityProbe>,
    tasks: Mutex<HashMap<DownloadTaskId, EngineTaskState>>,
    scheduler_gate: Mutex<()>,
    schedule_wake_generation: AtomicU64,
    daily_schedule_wake_generation: AtomicU64,
    event_sender: async_channel::Sender<DownloadEventEnvelope>,
    event_receiver: Mutex<Option<async_channel::Receiver<DownloadEventEnvelope>>>,
    event_subscribers: Mutex<Vec<async_channel::Sender<DownloadEventEnvelope>>>,
    event_sequencer: DownloadEventSequencer,
}

#[derive(Clone)]
struct EngineTaskState {
    task: DownloadTask,
    transfer: StoredTransferState,
    control: Option<DownloadTransferControl>,
    last_progress_event: Option<Instant>,
    transfer_rate: Option<TransferRateSampler>,
}

#[derive(Clone)]
struct TransferRateSampler {
    sampled_bytes: u64,
    sampled_at: Instant,
    last_rate: u64,
}

const ADAPTIVE_SAMPLE_COUNT: usize = 5;
const ADAPTIVE_STABILITY_PERCENT: u64 = 15;
const ADAPTIVE_MIN_PROPORTIONAL_GAIN_PERCENT: u64 = 80;
const MIB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdaptiveConnectionDecision {
    Hold,
    Grow,
    DisabledAfterRetry,
    DisabledForLowBenefit,
}

#[derive(Debug)]
struct AdaptiveSegmentController {
    max_segments: u16,
    samples: VecDeque<u64>,
    pending_evaluation: Option<(u64, u16, u16)>,
    disabled: bool,
}

impl AdaptiveSegmentController {
    fn new(max_segments: u16, enabled: bool) -> Self {
        Self {
            max_segments,
            samples: VecDeque::with_capacity(ADAPTIVE_SAMPLE_COUNT),
            pending_evaluation: None,
            disabled: !enabled || max_segments <= 1,
        }
    }

    fn observe(
        &mut self,
        bytes_per_second: u64,
        segment_count: u16,
        retry_observed: bool,
    ) -> AdaptiveConnectionDecision {
        if self.disabled {
            return AdaptiveConnectionDecision::Hold;
        }
        if retry_observed {
            self.disabled = true;
            self.samples.clear();
            self.pending_evaluation = None;
            return AdaptiveConnectionDecision::DisabledAfterRetry;
        }
        if bytes_per_second == 0 {
            return AdaptiveConnectionDecision::Hold;
        }
        if let Some((_, _, target_segments)) = self.pending_evaluation
            && segment_count < target_segments
        {
            self.samples.clear();
            return AdaptiveConnectionDecision::Hold;
        }
        self.samples.push_back(bytes_per_second);
        while self.samples.len() > ADAPTIVE_SAMPLE_COUNT {
            self.samples.pop_front();
        }
        if self.samples.len() < ADAPTIVE_SAMPLE_COUNT || !adaptive_samples_are_stable(&self.samples)
        {
            return AdaptiveConnectionDecision::Hold;
        }
        let average = self.samples.iter().copied().sum::<u64>()
            / u64::try_from(self.samples.len()).unwrap_or(1);
        if let Some((baseline_speed, baseline_segments, target_segments)) = self.pending_evaluation
        {
            let baseline_segments = baseline_segments.max(1);
            let added_segments = target_segments.saturating_sub(baseline_segments);
            let required_gain_percent = u128::from(added_segments)
                .saturating_mul(100)
                .saturating_mul(u128::from(ADAPTIVE_MIN_PROPORTIONAL_GAIN_PERCENT))
                / u128::from(baseline_segments)
                / 100;
            let required = u128::from(baseline_speed)
                .saturating_mul(100_u128.saturating_add(required_gain_percent))
                / 100;
            if u128::from(average) < required {
                self.disabled = true;
                self.samples.clear();
                self.pending_evaluation = None;
                return AdaptiveConnectionDecision::DisabledForLowBenefit;
            }
            self.pending_evaluation = None;
            self.samples.clear();
            return AdaptiveConnectionDecision::Hold;
        }
        if segment_count >= self.max_segments {
            return AdaptiveConnectionDecision::Hold;
        }
        let baseline_segments = segment_count.max(1);
        self.pending_evaluation = Some((average, baseline_segments, baseline_segments));
        self.samples.clear();
        AdaptiveConnectionDecision::Grow
    }

    fn confirm_growth_target(&mut self, target_segments: u16) {
        let Some((baseline_speed, baseline_segments, _)) = self.pending_evaluation else {
            return;
        };
        self.pending_evaluation = Some((
            baseline_speed,
            baseline_segments,
            target_segments.max(baseline_segments),
        ));
        self.samples.clear();
    }

    fn cancel_growth(&mut self) {
        self.pending_evaluation = None;
        self.samples.clear();
    }
}

fn adaptive_samples_are_stable(samples: &VecDeque<u64>) -> bool {
    let Some(minimum) = samples.iter().copied().min() else {
        return false;
    };
    let Some(maximum) = samples.iter().copied().max() else {
        return false;
    };
    let average = samples.iter().copied().sum::<u64>() / u64::try_from(samples.len()).unwrap_or(1);
    average > 0
        && maximum.saturating_sub(minimum).saturating_mul(100)
            <= average.saturating_mul(ADAPTIVE_STABILITY_PERCENT)
}

impl TransferRateSampler {
    const MIN_SAMPLE_INTERVAL: Duration = Duration::from_millis(500);

    fn new(downloaded_bytes: u64, sampled_at: Instant) -> Self {
        Self {
            sampled_bytes: downloaded_bytes,
            sampled_at,
            last_rate: 0,
        }
    }

    fn sample(&mut self, downloaded_bytes: u64, sampled_at: Instant) -> u64 {
        if downloaded_bytes < self.sampled_bytes {
            *self = Self::new(downloaded_bytes, sampled_at);
            return 0;
        }
        let elapsed = sampled_at.saturating_duration_since(self.sampled_at);
        if elapsed < Self::MIN_SAMPLE_INTERVAL {
            return self.last_rate;
        }
        let elapsed_nanos = elapsed.as_nanos();
        let downloaded_delta = downloaded_bytes.saturating_sub(self.sampled_bytes) as u128;
        let raw_rate = downloaded_delta
            .saturating_mul(1_000_000_000)
            .checked_div(elapsed_nanos)
            .unwrap_or_default()
            .min(u64::MAX as u128) as u64;
        let bytes_per_second = if self.last_rate == 0 {
            raw_rate
        } else {
            self.last_rate.saturating_mul(3).saturating_add(raw_rate) / 4
        };
        self.sampled_bytes = downloaded_bytes;
        self.sampled_at = sampled_at;
        self.last_rate = bytes_per_second;
        bytes_per_second
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DownloadEngineTask {
    pub task: DownloadTask,
    pub probe: Option<DownloadProbe>,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub error: Option<DownloadFault>,
    pub final_path: Option<PathBuf>,
    pub temporary_path: Option<PathBuf>,
    #[serde(default)]
    pub queue_eligible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadScheduleMoment {
    pub weekday: DownloadWeekday,
    pub minute_of_day: u16,
}

impl DownloadScheduleMoment {
    fn now_local() -> Self {
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
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DownloadEngineError> {
        let store = SqliteDownloadStore::open(path)?;
        Self::from_store(store)
    }

    pub fn open_in_memory() -> Result<Self, DownloadEngineError> {
        Self::from_store(SqliteDownloadStore::open_in_memory()?)
    }

    fn from_store(store: SqliteDownloadStore) -> Result<Self, DownloadEngineError> {
        Self::from_store_with_http(store, HttpDownloadClient::new()?)
    }

    fn from_store_with_http(
        store: SqliteDownloadStore,
        http: HttpDownloadClient,
    ) -> Result<Self, DownloadEngineError> {
        Self::from_store_with_http_and_capacity(store, http, Arc::new(SystemStorageCapacityProbe))
    }

    fn from_store_with_http_and_capacity(
        store: SqliteDownloadStore,
        http: HttpDownloadClient,
        capacity_probe: Arc<dyn StorageCapacityProbe>,
    ) -> Result<Self, DownloadEngineError> {
        let persisted_tasks = store.load_tasks()?;
        let reconciliation_id = uuid::Uuid::new_v4();
        for task in persisted_tasks.iter().rev() {
            match store.is_queue_task_eligible(task.id()) {
                Ok(_) => {}
                Err(DownloadStorageError::TaskNotFound(_)) => {
                    store.append_to_queue("default", task.id(), false)?;
                    warn!(
                        event = "download.queue.orphan_reconciled",
                        correlation_id = %reconciliation_id,
                        task_id = ?task.id(),
                        "persisted task without queue membership was attached to the default queue"
                    );
                }
                Err(error) => return Err(error.into()),
            }
        }
        let interrupted_task_ids = persisted_tasks
            .into_iter()
            .filter(|task| task.status().was_interrupted())
            .map(|task| task.id())
            .collect::<Vec<_>>();
        let recovered_count = store.recover_interrupted_tasks(now_unix_ms())?;
        if recovered_count > 0 {
            let correlation_id = uuid::Uuid::new_v4();
            for task_id in &interrupted_task_ids {
                info!(
                    event = "download.recovery.interrupted_task_paused",
                    %correlation_id,
                    ?task_id,
                    "interrupted task and active segments recovered as paused"
                );
            }
            info!(
                event = "download.recovery.completed",
                %correlation_id,
                recovered_count,
                "download startup recovery completed"
            );
        }
        let global_speed_limit = store.load_global_speed_limit()?;
        let mut tasks = HashMap::new();
        for task in store.load_tasks()? {
            let transfer = store.load_transfer_state(task.id())?;
            tasks.insert(
                task.id(),
                EngineTaskState {
                    task,
                    transfer,
                    control: None,
                    last_progress_event: None,
                    transfer_rate: None,
                },
            );
        }
        let (event_sender, event_receiver) = async_channel::bounded(256);
        let engine = Self {
            inner: Arc::new(DownloadEngineInner {
                http,
                store,
                global_rate_limiter: DownloadRateLimiter::new(global_speed_limit)?,
                capacity_probe,
                tasks: Mutex::new(tasks),
                scheduler_gate: Mutex::new(()),
                schedule_wake_generation: AtomicU64::new(0),
                daily_schedule_wake_generation: AtomicU64::new(0),
                event_sender,
                event_receiver: Mutex::new(Some(event_receiver)),
                event_subscribers: Mutex::new(Vec::new()),
                event_sequencer: DownloadEventSequencer::new(0),
            }),
        };
        if let Err(error) = engine.process_due_schedules_at(now_unix_ms()) {
            warn!(
                event = "download.schedule.startup_due_release_failed",
                error = %error,
                "due delayed downloads remain durable for the next scheduler wake"
            );
        }
        let correlation_id = uuid::Uuid::new_v4();
        for queue in engine.inner.store.load_queues()? {
            if let Err(error) = engine.schedule_queue(&queue.id, correlation_id) {
                warn!(
                    event = "download.queue.startup_refill_failed",
                    %correlation_id,
                    queue_id = queue.id,
                    error = %error,
                    "eligible queued tasks remain durable for a later refill attempt"
                );
            }
        }
        if let Err(error) = engine.process_daily_schedules_at(DownloadScheduleMoment::now_local()) {
            warn!(
                event = "download.schedule.daily_startup_reconcile_failed",
                error = %error,
                "daily queue schedule remains durable for the next wake"
            );
        }
        engine.refresh_delayed_schedule_wake()?;
        engine.refresh_daily_schedule_wake()?;
        Ok(engine)
    }

    pub fn take_event_receiver(
        &self,
    ) -> Result<async_channel::Receiver<DownloadEventEnvelope>, DownloadEngineError> {
        self.inner
            .event_receiver
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .take()
            .ok_or(DownloadEngineError::EventSubscriberAlreadyTaken)
    }

    /// Attaches an additional bounded event subscriber without taking ownership of the
    /// desktop application's primary event queue.
    pub fn subscribe_events(
        &self,
    ) -> Result<async_channel::Receiver<DownloadEventEnvelope>, DownloadEngineError> {
        let (sender, receiver) = async_channel::bounded(256);
        self.inner
            .event_subscribers
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .push(sender);
        Ok(receiver)
    }

    pub fn global_speed_limit(&self) -> Option<u64> {
        self.inner.global_rate_limiter.bytes_per_second()
    }

    pub fn queue_order(&self, queue_id: &str) -> Result<Vec<DownloadTaskId>, DownloadEngineError> {
        self.inner
            .store
            .load_queue_order(queue_id)
            .map_err(Into::into)
    }

    pub fn set_global_speed_limit(
        &self,
        bytes_per_second: Option<u64>,
    ) -> Result<(), DownloadEngineError> {
        self.set_global_speed_limit_correlated(bytes_per_second, uuid::Uuid::new_v4())
    }

    fn set_global_speed_limit_correlated(
        &self,
        bytes_per_second: Option<u64>,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        DownloadRateLimiter::new(bytes_per_second)?;
        self.inner.store.save_global_speed_limit(bytes_per_second)?;
        self.inner
            .global_rate_limiter
            .set_bytes_per_second(bytes_per_second)?;
        info!(
            event = "download.speed_limit.global_changed",
            %correlation_id,
            enabled = bytes_per_second.is_some(),
            bytes_per_second = bytes_per_second.unwrap_or_default(),
            "global download speed limit changed"
        );
        Ok(())
    }

    pub fn set_task_speed_limit(
        &self,
        task_id: DownloadTaskId,
        bytes_per_second: Option<u64>,
    ) -> Result<(), DownloadEngineError> {
        self.set_task_speed_limit_correlated(task_id, bytes_per_second, uuid::Uuid::new_v4())
    }

    fn set_task_speed_limit_correlated(
        &self,
        task_id: DownloadTaskId,
        bytes_per_second: Option<u64>,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        DownloadRateLimiter::new(bytes_per_second)?;
        let changed_task = {
            let mut tasks = self.inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            let mut committed = state.clone();
            committed
                .task
                .set_speed_limit(bytes_per_second, now_unix_ms());
            self.inner.persist(&committed)?;
            if let Some(control) = committed.control.as_ref() {
                control.set_task_speed_limit(bytes_per_second)?;
            }
            *state = committed;
            state.task.clone()
        };
        self.inner
            .emit(DownloadEvent::TaskChanged { task: changed_task });
        info!(
            event = "download.speed_limit.task_changed",
            %correlation_id,
            ?task_id,
            enabled = bytes_per_second.is_some(),
            bytes_per_second = bytes_per_second.unwrap_or_default(),
            "task download speed limit changed"
        );
        Ok(())
    }

    pub fn handle_command(&self, envelope: DownloadCommandEnvelope) -> DownloadCommandResult {
        let request_id = envelope.request_id;
        if envelope.protocol_version != DOWNLOAD_COMMAND_PROTOCOL_VERSION {
            return self.rejected(
                request_id,
                DownloadFault {
                    code: DownloadFaultCode::Protocol,
                    message: format!(
                        "unsupported download command protocol version {}; expected {}",
                        envelope.protocol_version, DOWNLOAD_COMMAND_PROTOCOL_VERSION
                    ),
                    retryable: false,
                },
            );
        }

        let outcome = match envelope.command {
            DownloadCommand::Probe { request } => self.probe_blocking(request),
            DownloadCommand::Submit { request, mode } => self
                .submit_with_mode_correlated(request, envelope.origin, mode, request_id)
                .map(|task_id| DownloadCommandOutcome::Accepted {
                    task_id: Some(task_id),
                }),
            DownloadCommand::Task { task_id, action } => self
                .run_task_action(task_id, action, request_id)
                .map(|()| DownloadCommandOutcome::Accepted {
                    task_id: Some(task_id),
                }),
            DownloadCommand::Batch { task_ids, action } => {
                Ok(self.run_batch_action(task_ids, action, request_id))
            }
            DownloadCommand::Queue { action } => self.run_queue_action(action, request_id),
            DownloadCommand::SetGlobalSpeedLimit { bytes_per_second } => self
                .set_global_speed_limit_correlated(bytes_per_second, request_id)
                .map(|()| DownloadCommandOutcome::Accepted { task_id: None }),
            DownloadCommand::Snapshot { query } => self
                .command_snapshot(query)
                .map(DownloadCommandOutcome::Snapshot),
            DownloadCommand::TaskDetail { task_id } => self
                .command_task_detail(task_id, request_id)
                .map(Box::new)
                .map(DownloadCommandOutcome::TaskDetail),
            DownloadCommand::Subscribe { after_sequence } => {
                if let Some(after_sequence) = after_sequence {
                    let observed_sequence = self.inner.event_sequencer.current();
                    if after_sequence != observed_sequence {
                        self.inner.emit(DownloadEvent::SnapshotRequired {
                            expected_sequence: after_sequence.saturating_add(1),
                            observed_sequence,
                        });
                    }
                }
                Ok(DownloadCommandOutcome::Accepted { task_id: None })
            }
        };

        match outcome {
            Ok(outcome) => DownloadCommandResult {
                request_id,
                outcome,
            },
            Err(error) => self.rejected(request_id, error.fault()),
        }
    }

    fn probe_blocking(
        &self,
        request: DownloadRequest,
    ) -> Result<DownloadCommandOutcome, DownloadEngineError> {
        request.validate()?;
        let client = self.inner.http.clone();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        download_runtime()?.spawn(async move {
            let _ = sender.send(client.probe(&request).await);
        });
        let probe = receiver
            .recv()
            .map_err(|error| DownloadEngineError::Runtime(error.to_string()))??;
        Ok(DownloadCommandOutcome::Probe(probe))
    }

    fn run_task_action(
        &self,
        task_id: DownloadTaskId,
        action: DownloadTaskAction,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        match action {
            DownloadTaskAction::Pause => self.pause(task_id),
            DownloadTaskAction::Resume => self.resume_correlated(task_id, correlation_id),
            DownloadTaskAction::Cancel => self.cancel(task_id),
            DownloadTaskAction::SetSpeedLimit { bytes_per_second } => {
                self.set_task_speed_limit_correlated(task_id, bytes_per_second, correlation_id)
            }
            DownloadTaskAction::Remove { delete_artifact } => self.remove(task_id, delete_artifact),
        }
    }

    fn run_batch_action(
        &self,
        task_ids: Vec<DownloadTaskId>,
        action: DownloadBatchAction,
        correlation_id: uuid::Uuid,
    ) -> DownloadCommandOutcome {
        let started_at = Instant::now();
        let mut seen = HashSet::new();
        let mut results = Vec::new();
        for task_id in task_ids.into_iter().filter(|task_id| seen.insert(*task_id)) {
            let task_action = match action {
                DownloadBatchAction::Pause => DownloadTaskAction::Pause,
                DownloadBatchAction::Resume => DownloadTaskAction::Resume,
                DownloadBatchAction::Cancel => DownloadTaskAction::Cancel,
                DownloadBatchAction::Remove => DownloadTaskAction::Remove {
                    delete_artifact: false,
                },
                DownloadBatchAction::RemoveWithArtifact => DownloadTaskAction::Remove {
                    delete_artifact: true,
                },
            };
            let fault = self
                .run_task_action(task_id, task_action, correlation_id)
                .err()
                .map(|error| error.fault());
            info!(
                event = "download.batch.target_completed",
                %correlation_id,
                ?task_id,
                action = ?action,
                accepted = fault.is_none(),
                fault_code = ?fault.as_ref().map(|fault| fault.code),
                "download batch target completed"
            );
            results.push(DownloadBatchItemResult { task_id, fault });
        }
        info!(
            event = "download.batch.completed",
            %correlation_id,
            action = ?action,
            target_count = results.len(),
            rejected_count = results.iter().filter(|result| result.fault.is_some()).count(),
            elapsed_ms = started_at.elapsed().as_millis(),
            "download batch command completed"
        );
        DownloadCommandOutcome::Batch { results }
    }

    fn run_queue_action(
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
                let eligible = self
                    .inner
                    .store
                    .load_eligible_queue_order(&queue_id)?
                    .into_iter()
                    .collect::<HashSet<_>>();
                for task_id in &task_ids {
                    if !eligible.contains(task_id) {
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
                return self
                    .set_queue_schedule_correlated(&queue_id, schedule, correlation_id)
                    .map(|queue| DownloadCommandOutcome::QueueChanged { queue });
            }
        };
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

    fn refresh_daily_schedule_wake(&self) -> Result<(), DownloadEngineError> {
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

    fn schedule_queue(
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

    fn schedule_queue_locked(
        &self,
        queue_id: &str,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let queue = self.queue(queue_id)?;
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
                let next = if committed.task.status() == DownloadStatus::Created {
                    DownloadStatus::Probing
                } else {
                    DownloadStatus::Waiting
                };
                committed.task.transition_to(next, now_unix_ms())?;
                committed.transfer.downloaded_bytes = restored_segment_bytes.unwrap_or(0);
                committed.transfer.error = None;
                committed.control = Some(DownloadTransferControl::with_rate_limiters(
                    self.inner.global_rate_limiter.clone(),
                    DownloadRateLimiter::new(
                        committed.task.request().speed_limit_bytes_per_second,
                    )?,
                ));
                committed.last_progress_event = None;
                self.inner.store.save_task_with_transfer_and_eligibility(
                    &committed.task,
                    &committed.transfer,
                    false,
                )?;
                *state = committed;
                state.task.clone()
            };
            self.inner
                .emit(DownloadEvent::TaskChanged { task: changed_task });
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

    fn command_snapshot(
        &self,
        query: DownloadQuery,
    ) -> Result<DownloadSnapshot, DownloadEngineError> {
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|search| !search.is_empty())
            .map(str::to_lowercase);
        let limit = query.limit.unwrap_or(u32::MAX) as usize;
        let task_details = self
            .tasks()?
            .into_iter()
            .filter(|snapshot| {
                (query.statuses.is_empty() || query.statuses.contains(&snapshot.task.status()))
                    && search.as_ref().is_none_or(|search| {
                        snapshot
                            .task
                            .request()
                            .source_url
                            .to_lowercase()
                            .contains(search)
                            || snapshot.final_path.as_ref().is_some_and(|path| {
                                path.to_string_lossy().to_lowercase().contains(search)
                            })
                    })
            })
            .take(limit)
            .collect::<Vec<_>>();
        let tasks = task_details
            .iter()
            .map(|snapshot| snapshot.task.clone())
            .collect();
        Ok(DownloadSnapshot {
            generation: 1,
            sequence: self.inner.event_sequencer.current(),
            tasks,
            queues: self.inner.store.load_queues()?,
            aggregate_metrics: DownloadAggregateMetrics::default(),
            task_details,
        })
    }

    fn command_task_detail(
        &self,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadTaskDetail, DownloadEngineError> {
        let started_at = Instant::now();
        info!(
            event = "download.detail.requested",
            %correlation_id,
            task_id = %task_id.as_uuid(),
            "download task detail requested"
        );
        let result: Result<DownloadTaskDetail, DownloadEngineError> = (|| {
            let task = self
                .task(task_id)?
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            let segments = self
                .inner
                .store
                .load_transfer_plan(task_id)?
                .map(|plan| plan.segments().to_vec())
                .unwrap_or_default();
            let timeline = self.inner.store.load_task_timeline(task_id, 300)?;
            Ok(DownloadTaskDetail {
                task,
                segments,
                timeline,
            })
        })();
        match result {
            Ok(detail) => {
                info!(
                    event = "download.detail.loaded",
                    %correlation_id,
                    task_id = %task_id.as_uuid(),
                    segment_count = detail.segments.len(),
                    timeline_count = detail.timeline.len(),
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "download task detail loaded"
                );
                Ok(detail)
            }
            Err(error) => {
                let fault = error.fault();
                warn!(
                    event = "download.detail.failed",
                    %correlation_id,
                    task_id = %task_id.as_uuid(),
                    fault_code = ?fault.code,
                    retryable = fault.retryable,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "download task detail failed"
                );
                Err(error)
            }
        }
    }

    fn rejected(&self, request_id: uuid::Uuid, fault: DownloadFault) -> DownloadCommandResult {
        self.inner.emit(DownloadEvent::CommandRejected {
            request_id,
            fault: fault.clone(),
        });
        DownloadCommandResult {
            request_id,
            outcome: DownloadCommandOutcome::Rejected { fault },
        }
    }

    pub fn submit(
        &self,
        request: DownloadRequest,
        origin: RequestOrigin,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        self.submit_with_mode(request, origin, DownloadSubmitMode::StartNow)
    }

    pub fn enqueue(
        &self,
        request: DownloadRequest,
        origin: RequestOrigin,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        self.submit_with_mode(request, origin, DownloadSubmitMode::QueueOnly)
    }

    fn submit_with_mode(
        &self,
        request: DownloadRequest,
        origin: RequestOrigin,
        mode: DownloadSubmitMode,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        self.submit_with_mode_correlated(request, origin, mode, uuid::Uuid::new_v4())
    }

    fn submit_with_mode_correlated(
        &self,
        request: DownloadRequest,
        origin: RequestOrigin,
        mode: DownloadSubmitMode,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        request.validate()?;
        let now = now_unix_ms();
        let delayed_until_due = request
            .scheduled_at_unix_ms
            .is_some_and(|scheduled_at| scheduled_at > now);
        let scheduled_at_unix_ms = request.scheduled_at_unix_ms;
        let task = DownloadTask::new(request, origin, now);
        let task_id = task.id();
        let transfer = StoredTransferState::default();
        let control = None;
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        self.inner.store.save_task_with_transfer_and_queue(
            &task,
            &transfer,
            "default",
            mode == DownloadSubmitMode::StartNow && !delayed_until_due,
        )?;
        self.inner.lock_tasks()?.insert(
            task_id,
            EngineTaskState {
                task,
                transfer,
                control,
                last_progress_event: None,
                transfer_rate: None,
            },
        );
        self.inner.emit_task(task_id);

        if delayed_until_due {
            info!(
                event = "download.schedule.delayed_accepted",
                %correlation_id,
                ?task_id,
                due_at_unix_ms = scheduled_at_unix_ms.unwrap_or_default(),
                "delayed download accepted and held outside the queue"
            );
            self.refresh_delayed_schedule_wake()?;
        } else if mode == DownloadSubmitMode::StartNow
            && let Err(error) = self.schedule_queue_locked("default", correlation_id)
        {
            warn!(
                event = "download.queue.admission_deferred",
                %correlation_id,
                ?task_id,
                error = %error,
                "accepted task remains eligible for a later queue refill"
            );
        }
        Ok(task_id)
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
        if !released_task_ids.is_empty()
            && let Err(error) = self.schedule_queue_locked("default", correlation_id)
        {
            warn!(
                event = "download.schedule.delayed_admission_deferred",
                %correlation_id,
                released_count = released_task_ids.len(),
                error = %error,
                "due delayed downloads remain eligible for a later queue refill"
            );
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

    fn refresh_delayed_schedule_wake(&self) -> Result<(), DownloadEngineError> {
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

    pub fn pause(&self, task_id: DownloadTaskId) -> Result<(), DownloadEngineError> {
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        let mut tasks = self.inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        if matches!(
            state.task.status(),
            DownloadStatus::Created | DownloadStatus::Paused | DownloadStatus::Failed
        ) && (self.inner.store.is_queue_task_eligible(task_id)?
            || state.task.request().scheduled_at_unix_ms.is_some())
        {
            let mut committed = state.clone();
            if committed.task.status() == DownloadStatus::Created {
                committed
                    .task
                    .transition_to(DownloadStatus::Paused, now_unix_ms())?;
            }
            committed.task.clear_scheduled_start(now_unix_ms());
            self.inner.store.save_task_with_transfer_and_admission(
                &committed.task,
                &committed.transfer,
                false,
                true,
            )?;
            *state = committed;
            self.inner.emit(DownloadEvent::TaskChanged {
                task: state.task.clone(),
            });
            drop(tasks);
            drop(_scheduler);
            self.refresh_delayed_schedule_wake()?;
            return Ok(());
        }
        if !matches!(
            state.task.status(),
            DownloadStatus::Probing | DownloadStatus::Waiting | DownloadStatus::Downloading
        ) {
            return Err(DownloadEngineError::InvalidAction {
                task_id,
                action: "pause",
                status: state.task.status(),
            });
        }
        state
            .control
            .as_ref()
            .ok_or(DownloadEngineError::MissingTransferControl(task_id))?
            .pause();
        self.inner
            .store
            .set_queue_task_admission(task_id, false, true)?;
        Ok(())
    }

    pub fn resume(&self, task_id: DownloadTaskId) -> Result<(), DownloadEngineError> {
        self.resume_correlated(task_id, uuid::Uuid::new_v4())
    }

    fn resume_correlated(
        &self,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        {
            let mut tasks = self.inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            if !matches!(
                state.task.status(),
                DownloadStatus::Created | DownloadStatus::Paused | DownloadStatus::Failed
            ) {
                return Err(DownloadEngineError::InvalidAction {
                    task_id,
                    action: "resume",
                    status: state.task.status(),
                });
            }
            if state.task.request().scheduled_at_unix_ms.is_some() {
                let mut committed = state.clone();
                committed.task.clear_scheduled_start(now_unix_ms());
                self.inner.store.save_task_with_transfer_and_admission(
                    &committed.task,
                    &committed.transfer,
                    true,
                    false,
                )?;
                *state = committed;
            } else {
                self.inner
                    .store
                    .set_queue_task_admission(task_id, true, false)?;
            }
        }
        self.refresh_delayed_schedule_wake()?;
        if let Err(error) = self.schedule_queue("default", correlation_id) {
            warn!(
                event = "download.queue.resume_deferred",
                %correlation_id,
                ?task_id,
                error = %error,
                "resume intent was committed and remains eligible for a later refill"
            );
        }
        Ok(())
    }

    pub fn cancel(&self, task_id: DownloadTaskId) -> Result<(), DownloadEngineError> {
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        self.inner.store.set_queue_task_eligible(task_id, false)?;
        let mut tasks = self.inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        let changed = match state.task.status() {
            DownloadStatus::Probing | DownloadStatus::Waiting | DownloadStatus::Downloading => {
                state
                    .control
                    .as_ref()
                    .ok_or(DownloadEngineError::MissingTransferControl(task_id))?
                    .cancel();
                false
            }
            DownloadStatus::Created | DownloadStatus::Paused | DownloadStatus::Failed => {
                remove_task_temporary_artifact(state);
                state.transfer.temporary_path = None;
                state
                    .task
                    .transition_to(DownloadStatus::Cancelled, now_unix_ms())?;
                state.control = None;
                self.inner.persist(state)?;
                true
            }
            status => {
                return Err(DownloadEngineError::InvalidAction {
                    task_id,
                    action: "cancel",
                    status,
                });
            }
        };
        drop(tasks);
        if changed {
            self.inner.emit_task(task_id);
        }
        Ok(())
    }

    pub fn remove(
        &self,
        task_id: DownloadTaskId,
        delete_artifact: bool,
    ) -> Result<(), DownloadEngineError> {
        let snapshot = {
            let tasks = self.inner.lock_tasks()?;
            let state = tasks
                .get(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            if matches!(
                state.task.status(),
                DownloadStatus::Probing
                    | DownloadStatus::Waiting
                    | DownloadStatus::Downloading
                    | DownloadStatus::Finalizing
                    | DownloadStatus::Verifying
            ) {
                return Err(DownloadEngineError::InvalidAction {
                    task_id,
                    action: "remove",
                    status: state.task.status(),
                });
            }
            snapshot_from_state(state)
        };

        if let Some(temporary_path) = snapshot.temporary_path.as_deref() {
            remove_path_if_exists(temporary_path)?;
        }
        if delete_artifact && let Some(final_path) = snapshot.final_path.as_deref() {
            remove_path_if_exists(final_path)?;
        }
        if !self.inner.store.remove_task(task_id)? {
            return Err(DownloadEngineError::TaskNotFound(task_id));
        }
        self.inner.lock_tasks()?.remove(&task_id);
        self.inner.emit(DownloadEvent::TaskRemoved { task_id });
        Ok(())
    }

    pub fn task(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<Option<DownloadEngineTask>, DownloadEngineError> {
        let mut snapshot = self
            .inner
            .lock_tasks()?
            .get(&task_id)
            .map(snapshot_from_state);
        if let Some(snapshot) = snapshot.as_mut() {
            snapshot.queue_eligible = match self.inner.store.is_queue_task_eligible(task_id) {
                Ok(eligible) => eligible,
                Err(DownloadStorageError::TaskNotFound(_)) => false,
                Err(error) => return Err(error.into()),
            };
        }
        Ok(snapshot)
    }

    pub fn tasks(&self) -> Result<Vec<DownloadEngineTask>, DownloadEngineError> {
        let queue_order = self.inner.store.load_queue_order("default")?;
        let queue_positions = queue_order
            .into_iter()
            .enumerate()
            .map(|(position, task_id)| (task_id, position))
            .collect::<HashMap<_, _>>();
        let mut eligible = HashSet::new();
        for queue in self.inner.store.load_queues()? {
            eligible.extend(self.inner.store.load_eligible_queue_order(&queue.id)?);
        }
        let mut snapshots = self
            .inner
            .lock_tasks()?
            .values()
            .map(snapshot_from_state)
            .collect::<Vec<_>>();
        for snapshot in &mut snapshots {
            snapshot.queue_eligible = eligible.contains(&snapshot.task.id());
        }
        snapshots.sort_by_key(|snapshot| {
            (
                queue_positions
                    .get(&snapshot.task.id())
                    .copied()
                    .unwrap_or(usize::MAX),
                std::cmp::Reverse(snapshot.task.created_at_unix_ms()),
            )
        });
        Ok(snapshots)
    }

    pub async fn wait_for_terminal(
        &self,
        task_id: DownloadTaskId,
        timeout: Duration,
    ) -> Result<DownloadEngineTask, DownloadEngineError> {
        let deadline = Instant::now() + timeout;
        loop {
            let task = self
                .task(task_id)?
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            if matches!(
                task.task.status(),
                DownloadStatus::Completed
                    | DownloadStatus::Failed
                    | DownloadStatus::Paused
                    | DownloadStatus::Cancelled
            ) {
                return Ok(task);
            }
            if Instant::now() >= deadline {
                return Err(DownloadEngineError::WaitTimeout(task_id));
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}

impl DownloadEngineInner {
    fn lock_tasks(
        &self,
    ) -> Result<MutexGuard<'_, HashMap<DownloadTaskId, EngineTaskState>>, DownloadEngineError> {
        self.tasks
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)
    }

    fn persist(&self, state: &EngineTaskState) -> Result<(), DownloadEngineError> {
        self.store
            .save_task_with_transfer(&state.task, &state.transfer)?;
        Ok(())
    }

    fn emit(&self, event: DownloadEvent) {
        let Ok(envelope) = self.event_sequencer.next(now_unix_ms(), event) else {
            return;
        };
        if matches!(envelope.event, DownloadEvent::TaskProgress { .. }) {
            if self.event_sender.len() < 192 {
                let _ = self.event_sender.try_send(envelope.clone());
            }
        } else {
            // State events are rare and authoritative. Backpressure is preferable to either
            // dropping them or moving an unbounded backlog into spawned tasks.
            let _ = self.event_sender.send_blocking(envelope.clone());
        }
        if let Ok(mut subscribers) = self.event_subscribers.lock() {
            subscribers.retain(|subscriber| {
                if matches!(envelope.event, DownloadEvent::TaskProgress { .. }) {
                    subscriber.len() >= 192 || subscriber.try_send(envelope.clone()).is_ok()
                } else {
                    // Secondary observers must never backpressure the authoritative engine. A
                    // stalled IPC watch client is disconnected once its bounded queue is full.
                    subscriber.try_send(envelope.clone()).is_ok()
                }
            });
        }
    }

    fn emit_task(&self, task_id: DownloadTaskId) {
        let task = self
            .tasks
            .lock()
            .ok()
            .and_then(|tasks| tasks.get(&task_id).map(|state| state.task.clone()));
        if let Some(task) = task {
            self.emit(DownloadEvent::TaskChanged { task });
        }
    }

    fn persist_transfer_plan_change(
        &self,
        task_id: DownloadTaskId,
        plan: &TransferPlan,
    ) -> Result<(), DownloadEngineError> {
        self.store.save_transfer_plan(task_id, plan)?;
        self.emit_task(task_id);
        Ok(())
    }
}

fn is_active_download_status(status: DownloadStatus) -> bool {
    matches!(
        status,
        DownloadStatus::Probing
            | DownloadStatus::Waiting
            | DownloadStatus::Downloading
            | DownloadStatus::Finalizing
            | DownloadStatus::Verifying
    )
}

fn spawn_download_worker(
    inner: Arc<DownloadEngineInner>,
    queue_id: String,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
) -> Result<(), DownloadEngineError> {
    download_runtime()?.spawn(async move {
        if let Err(error) = run_task(Arc::clone(&inner), task_id, correlation_id).await {
            record_failure(&inner, task_id, correlation_id, error);
        }
        let engine = DownloadEngine {
            inner: Arc::clone(&inner),
        };
        if let Err(error) = engine.schedule_queue(&queue_id, correlation_id) {
            warn!(
                event = "download.queue.refill_failed",
                %correlation_id,
                ?task_id,
                queue_id,
                error = %error,
                "download queue could not refill a released slot"
            );
        }
    });
    Ok(())
}

async fn run_task(
    inner: Arc<DownloadEngineInner>,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
) -> Result<(), DownloadEngineError> {
    let request = {
        let tasks = inner.lock_tasks()?;
        tasks
            .get(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?
            .task
            .request()
            .clone()
    };
    let probe = inner.http.probe(&request).await?;
    let resume_plan_present = inner.store.load_transfer_plan(task_id)?.is_some();
    let validator_kind = if probe.validator.etag.is_some() {
        "etag"
    } else if probe.validator.last_modified.is_some() {
        "last_modified"
    } else {
        "none"
    };
    info!(
        event = "download.probe.completed",
        %correlation_id,
        ?task_id,
        total_bytes = probe.total_bytes.unwrap_or_default(),
        total_bytes_known = probe.total_bytes.is_some(),
        supports_ranges = probe.supports_ranges,
        validator_kind,
        resume_plan_present,
        requested_connections = request.connection_limit.unwrap_or(1),
        "download resource probe completed"
    );
    let (temporary_path, final_path) =
        HttpDownloadClient::artifact_paths(&request, &probe, task_id)?;
    tokio::fs::create_dir_all(&request.save_directory).await?;
    if tokio::fs::try_exists(&final_path).await? {
        return Err(HttpDownloadError::DestinationExists(final_path).into());
    }
    preflight_storage_capacity(
        &inner,
        task_id,
        correlation_id,
        &request.save_directory,
        &probe,
    )
    .await?;
    let control = {
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.probe = Some(probe.clone());
        state.transfer.temporary_path = Some(temporary_path.clone());
        state.transfer.final_path = Some(final_path.clone());
        state.transfer.final_committed = false;
        if state.task.status() == DownloadStatus::Probing {
            state
                .task
                .transition_to(DownloadStatus::Waiting, now_unix_ms())?;
        }
        state
            .task
            .transition_to(DownloadStatus::Downloading, now_unix_ms())?;
        state.last_progress_event = None;
        state.transfer_rate = Some(TransferRateSampler::new(
            state.transfer.downloaded_bytes,
            Instant::now(),
        ));
        let control = state
            .control
            .clone()
            .ok_or(DownloadEngineError::MissingTransferControl(task_id))?;
        inner.persist(state)?;
        inner.emit(DownloadEvent::TaskChanged {
            task: state.task.clone(),
        });
        control
    };

    if probe.supports_ranges && probe.total_bytes.is_some_and(|total| total > 0) {
        return run_segmented_task(
            &inner,
            task_id,
            correlation_id,
            &request,
            &probe,
            &control,
            temporary_path,
            final_path,
        )
        .await;
    }

    run_single_stream_task(
        &inner,
        task_id,
        correlation_id,
        &request,
        &probe,
        &control,
        "probe_single_stream",
    )
    .await
}

async fn preflight_storage_capacity(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    save_directory: &Path,
    probe: &DownloadProbe,
) -> Result<(), DownloadEngineError> {
    let Some(total_bytes) = probe.total_bytes else {
        info!(
            event = "download.storage_preflight.skipped",
            %correlation_id,
            ?task_id,
            reason = "unknown_length",
            "storage capacity preflight skipped"
        );
        return Ok(());
    };
    let durable_downloaded_bytes = if probe.supports_ranges {
        inner
            .store
            .load_transfer_plan(task_id)?
            .map(|plan| {
                plan.segments()
                    .iter()
                    .map(DownloadSegment::downloaded_bytes)
                    .sum()
            })
            .unwrap_or_default()
    } else {
        0
    };
    let requirement =
        DownloadStorageRequirement::for_transfer(total_bytes, durable_downloaded_bytes)?;
    if requirement.required_bytes() == 0 {
        info!(
            event = "download.storage_preflight.accepted",
            %correlation_id,
            ?task_id,
            remaining_bytes = 0_u64,
            reserve_bytes = 0_u64,
            required_bytes = 0_u64,
            available_bytes = 0_u64,
            "completed artifact needs no additional capacity"
        );
        return Ok(());
    }

    let probe_path = save_directory.to_path_buf();
    let capacity_probe = Arc::clone(&inner.capacity_probe);
    let available =
        tokio::task::spawn_blocking(move || capacity_probe.available_space(&probe_path))
            .await
            .map_err(|error| DownloadEngineError::Runtime(error.to_string()))?;
    let Some(available_bytes) = available else {
        warn!(
            event = "download.storage_preflight.skipped",
            %correlation_id,
            ?task_id,
            reason = "volume_not_found",
            remaining_bytes = requirement.remaining_bytes(),
            reserve_bytes = requirement.reserve_bytes(),
            required_bytes = requirement.required_bytes(),
            "destination volume was not identified; real writes remain authoritative"
        );
        return Ok(());
    };

    if let Err(DownloadStorageCapacityError::Insufficient {
        available_bytes,
        required_bytes,
    }) = requirement.ensure_available(available_bytes)
    {
        warn!(
            event = "download.storage_preflight.failed",
            %correlation_id,
            ?task_id,
            remaining_bytes = requirement.remaining_bytes(),
            reserve_bytes = requirement.reserve_bytes(),
            required_bytes,
            available_bytes,
            error_kind = "disk_full",
            "destination volume does not have the required free space"
        );
        return Err(HttpDownloadError::InsufficientDiskSpace {
            available_bytes,
            required_bytes,
        }
        .into());
    }

    info!(
        event = "download.storage_preflight.accepted",
        %correlation_id,
        ?task_id,
        remaining_bytes = requirement.remaining_bytes(),
        reserve_bytes = requirement.reserve_bytes(),
        required_bytes = requirement.required_bytes(),
        available_bytes,
        "destination volume has sufficient free space"
    );
    Ok(())
}

async fn run_single_stream_task(
    inner: &Arc<DownloadEngineInner>,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    probe: &DownloadProbe,
    control: &DownloadTransferControl,
    reason: &'static str,
) -> Result<(), DownloadEngineError> {
    info!(
        event = "download.single_stream.started",
        %correlation_id,
        ?task_id,
        reason,
        expected_bytes = probe.total_bytes.unwrap_or_default(),
        "single-stream transfer started"
    );
    let progress_inner = Arc::clone(inner);
    let outcome = inner
        .http
        .prepare_download(request, probe, task_id, control, move |progress| {
            publish_transfer_progress(
                &progress_inner,
                task_id,
                progress.downloaded_bytes,
                progress.total_bytes,
                1,
                None,
                None,
            );
        })
        .await?;

    match outcome {
        PreparedDownloadOutcome::Ready(ready) => {
            let downloaded_bytes = ready.downloaded_bytes;
            {
                let mut tasks = inner.lock_tasks()?;
                let state = tasks
                    .get_mut(&task_id)
                    .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
                state.transfer.downloaded_bytes = ready.downloaded_bytes;
                state
                    .task
                    .transition_to(DownloadStatus::Finalizing, now_unix_ms())?;
                inner.persist(state)?;
                inner.emit(DownloadEvent::TaskChanged {
                    task: state.task.clone(),
                });
            }
            inner.http.commit_prepared(ready).await?;
            commit_completed_state(inner, task_id)?;
            info!(
                event = "download.single_stream.completed",
                %correlation_id,
                ?task_id,
                reason,
                downloaded_bytes,
                "single-stream transfer completed"
            );
        }
        PreparedDownloadOutcome::Paused {
            downloaded_bytes, ..
        } => {
            let mut tasks = inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            state.transfer.downloaded_bytes = downloaded_bytes;
            state
                .task
                .transition_to(DownloadStatus::Paused, now_unix_ms())?;
            state.control = None;
            inner.persist(state)?;
            inner.emit(DownloadEvent::TaskChanged {
                task: state.task.clone(),
            });
            info!(
                event = "download.single_stream.paused",
                %correlation_id,
                ?task_id,
                reason,
                downloaded_bytes,
                "single-stream transfer paused"
            );
        }
        PreparedDownloadOutcome::Cancelled { downloaded_bytes } => {
            let mut tasks = inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            state.transfer.downloaded_bytes = downloaded_bytes;
            state
                .task
                .transition_to(DownloadStatus::Cancelled, now_unix_ms())?;
            state.control = None;
            inner.persist(state)?;
            inner.emit(DownloadEvent::TaskChanged {
                task: state.task.clone(),
            });
            info!(
                event = "download.single_stream.cancelled",
                %correlation_id,
                ?task_id,
                reason,
                downloaded_bytes,
                "single-stream transfer cancelled"
            );
        }
    }
    Ok(())
}

type SegmentTransferResult = (
    DownloadSegment,
    Result<DownloadSegmentTransferOutcome, HttpDownloadError>,
);

#[allow(clippy::too_many_arguments)]
fn spawn_segment_transfer(
    transfers: &mut tokio::task::JoinSet<SegmentTransferResult>,
    inner: &Arc<DownloadEngineInner>,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    probe: &DownloadProbe,
    task_control: &DownloadTransferControl,
    artifact: &SegmentedArtifact,
    progress_by_segment: &Arc<Mutex<HashMap<u32, u64>>>,
    segment_lengths: &Arc<Mutex<HashMap<u32, u64>>>,
    active_connections: &Arc<AtomicU16>,
    retry_observed: &Arc<AtomicBool>,
    total_bytes: u64,
    use_if_range: bool,
    segment: DownloadSegment,
) -> DownloadSegmentTransferControl {
    let http = inner.http.clone();
    let request = request.clone();
    let probe = probe.clone();
    let artifact = artifact.clone();
    let task_control = task_control.clone();
    let segment_control = DownloadSegmentTransferControl::new(task_control.clone());
    let worker_control = segment_control.clone();
    let progress_inner = Arc::clone(inner);
    let progress_by_segment = Arc::clone(progress_by_segment);
    let segment_lengths = Arc::clone(segment_lengths);
    let active_connections = Arc::clone(active_connections);
    let retry_observed = Arc::clone(retry_observed);
    active_connections.fetch_add(1, Ordering::AcqRel);
    transfers.spawn(async move {
        let segment_index = segment.index();
        let mut attempt = 1_u32;
        let outcome = loop {
            let attempt_progress_inner = Arc::clone(&progress_inner);
            let attempt_progress_by_segment = Arc::clone(&progress_by_segment);
            let attempt_segment_lengths = Arc::clone(&segment_lengths);
            let attempt_active_connections = Arc::clone(&active_connections);
            let mut transfer = crate::DownloadSegmentTransferRequest::new(
                DownloadTransferContext::new(task_id, correlation_id),
                &request,
                &probe,
                &segment,
                &artifact,
                &segment_control,
            );
            if use_if_range {
                transfer = transfer.with_resume_validator();
            }
            let outcome = http
                .download_segment(transfer, move |progress| {
                    let aggregate = attempt_progress_by_segment
                        .lock()
                        .map(|mut values| {
                            values.insert(segment_index, progress.downloaded_bytes);
                            values.values().copied().sum()
                        })
                        .unwrap_or_default();
                    publish_transfer_progress(
                        &attempt_progress_inner,
                        task_id,
                        aggregate,
                        Some(total_bytes),
                        attempt_active_connections.load(Ordering::Acquire),
                        Some(&attempt_progress_by_segment),
                        Some(&attempt_segment_lengths),
                    );
                })
                .await;
            match outcome {
                Err(error)
                    if attempt < MAX_SEGMENT_TRANSFER_ATTEMPTS && error.is_retryable_transfer() =>
                {
                    retry_observed.store(true, Ordering::Release);
                    let aggregate = progress_by_segment
                        .lock()
                        .map(|mut values| {
                            values.insert(segment_index, segment.downloaded_bytes());
                            values.values().copied().sum()
                        })
                        .unwrap_or_default();
                    publish_transfer_progress(
                        &progress_inner,
                        task_id,
                        aggregate,
                        Some(total_bytes),
                        active_connections.load(Ordering::Acquire),
                        Some(progress_by_segment.as_ref()),
                        Some(segment_lengths.as_ref()),
                    );
                    let delay = segment_retry_delay(attempt);
                    warn!(
                        event = "download.segment_retry.scheduled",
                        %correlation_id,
                        ?task_id,
                        segment_index,
                        failed_attempt = attempt,
                        next_attempt = attempt + 1,
                        delay_ms = delay.as_millis(),
                        error_kind = error.kind(),
                        "transient segment failure scheduled for retry"
                    );
                    attempt += 1;
                    let _stopped = task_control.wait_for_stop_or_timeout(delay).await;
                }
                terminal => break terminal,
            }
        };
        active_connections.fetch_sub(1, Ordering::AcqRel);
        (segment, outcome)
    });
    worker_control
}

fn select_rebalance_candidate(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    worker_controls: &HashMap<u32, DownloadSegmentTransferControl>,
    progress_by_segment: &Arc<Mutex<HashMap<u32, u64>>>,
    minimum_remaining_bytes: u64,
) -> Result<Option<(u32, u64)>, DownloadEngineError> {
    let plan = inner
        .store
        .load_transfer_plan(task_id)?
        .ok_or(DownloadEngineError::EmptySegmentLayout(task_id))?;
    let progress = progress_by_segment
        .lock()
        .map_err(|_| DownloadEngineError::LockPoisoned)?;
    Ok(plan
        .segments()
        .iter()
        .filter(|segment| worker_controls.contains_key(&segment.index()))
        .filter_map(|segment| {
            let observed = progress
                .get(&segment.index())
                .copied()
                .unwrap_or_else(|| segment.downloaded_bytes());
            let remaining = segment.len().saturating_sub(observed);
            (remaining >= minimum_remaining_bytes).then_some((segment.index(), remaining))
        })
        .max_by_key(|(_, remaining)| *remaining))
}

#[allow(clippy::too_many_arguments)]
async fn run_segmented_task(
    inner: &Arc<DownloadEngineInner>,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    probe: &DownloadProbe,
    control: &DownloadTransferControl,
    temporary_path: PathBuf,
    final_path: PathBuf,
) -> Result<(), DownloadEngineError> {
    let total_bytes = probe
        .total_bytes
        .ok_or(DownloadEngineError::MissingSegmentedLength(task_id))?;
    tokio::fs::create_dir_all(&request.save_directory).await?;
    if tokio::fs::try_exists(&final_path).await? {
        return Err(HttpDownloadError::DestinationExists(final_path).into());
    }

    let (segments, artifact, use_if_range) = match inner.store.load_transfer_plan(task_id)? {
        Some(plan) => {
            validate_resumed_plan(task_id, &plan, probe)?;
            if plan.segments().is_empty() {
                return Err(DownloadEngineError::EmptySegmentLayout(task_id));
            }
            let use_if_range = plan
                .segments()
                .iter()
                .any(|segment| segment.downloaded_bytes() > 0);
            let mut restored = Vec::with_capacity(plan.segments().len());
            for segment in plan.segments() {
                let state = if segment.state == DownloadSegmentState::Completed {
                    DownloadSegmentState::Completed
                } else {
                    DownloadSegmentState::Waiting
                };
                let restored_segment = DownloadSegment::new(
                    segment.index(),
                    segment.start_byte(),
                    segment.end_byte(),
                    segment.downloaded_bytes(),
                    state,
                )?;
                if state == DownloadSegmentState::Waiting {
                    inner
                        .store
                        .save_segment_checkpoint(task_id, &restored_segment)?;
                    info!(
                        event = "download.segment_checkpoint.restored",
                        %correlation_id,
                        ?task_id,
                        segment_index = restored_segment.index(),
                        downloaded_bytes = restored_segment.downloaded_bytes(),
                        "persisted segment checkpoint restored to the waiting queue"
                    );
                }
                restored.push(restored_segment);
            }
            let artifact = SegmentedArtifact::open_existing(&temporary_path, total_bytes)
                .await
                .map_err(HttpDownloadError::from)?;
            (restored, artifact, use_if_range)
        }
        None => {
            let max_connections = request.connection_limit.unwrap_or(1).clamp(1, 64);
            let segments = create_initial_segments(total_bytes, Some(max_connections))?;
            let artifact = SegmentedArtifact::create(&temporary_path, total_bytes)
                .await
                .map_err(HttpDownloadError::from)?;
            match artifact.allocation() {
                ArtifactAllocation::Sparse => info!(
                    event = "download.artifact.sparse_enabled",
                    %correlation_id,
                    ?task_id,
                    total_bytes,
                    "segmented temporary artifact created with sparse allocation"
                ),
                ArtifactAllocation::Normal => warn!(
                    event = "download.artifact.sparse_fallback",
                    %correlation_id,
                    ?task_id,
                    total_bytes,
                    reason = "unsupported_or_unavailable",
                    "segmented temporary artifact uses ordinary allocation"
                ),
            }
            let plan = TransferPlan::new(
                TransferProtocol::Http,
                probe.final_url.clone(),
                Some(total_bytes),
                true,
                probe.validator.clone(),
                segments.clone(),
            )?;
            inner.store.save_transfer_plan(task_id, &plan)?;
            info!(
                event = "download.transfer_plan.created",
                %correlation_id,
                ?task_id,
                segment_count = segments.len(),
                total_bytes,
                "segmented transfer plan persisted"
            );
            (segments, artifact, false)
        }
    };

    let progress_by_segment = Arc::new(Mutex::new(
        segments
            .iter()
            .map(|segment| (segment.index(), segment.downloaded_bytes()))
            .collect::<HashMap<_, _>>(),
    ));
    let segment_lengths = Arc::new(Mutex::new(
        segments
            .iter()
            .map(|segment| (segment.index(), segment.len()))
            .collect::<HashMap<_, _>>(),
    ));
    let runnable = segments
        .iter()
        .filter(|segment| segment.state != DownloadSegmentState::Completed)
        .cloned()
        .collect::<Vec<_>>();
    let active_connections = Arc::new(AtomicU16::new(0));
    let retry_observed = Arc::new(AtomicBool::new(false));
    let mut transfers = tokio::task::JoinSet::new();
    let mut worker_controls = HashMap::new();
    let max_connections = request.connection_limit.unwrap_or(1).clamp(1, 64);
    let mut waiting_segments = VecDeque::new();
    for segment in runnable {
        if worker_controls.len() >= usize::from(max_connections) {
            let waiting = if segment.state == DownloadSegmentState::Waiting {
                segment
            } else {
                checkpoint_segment(
                    inner,
                    task_id,
                    correlation_id,
                    &segment,
                    segment.downloaded_bytes(),
                    DownloadSegmentState::Waiting,
                )?
            };
            waiting_segments.push_back(waiting);
            continue;
        }
        let active = if segment.state == DownloadSegmentState::Active {
            segment
        } else {
            checkpoint_segment(
                inner,
                task_id,
                correlation_id,
                &segment,
                segment.downloaded_bytes(),
                DownloadSegmentState::Active,
            )?
        };
        let segment_index = active.index();
        let worker_control = spawn_segment_transfer(
            &mut transfers,
            inner,
            task_id,
            correlation_id,
            request,
            probe,
            control,
            &artifact,
            &progress_by_segment,
            &segment_lengths,
            &active_connections,
            &retry_observed,
            total_bytes,
            use_if_range,
            active,
        );
        worker_controls.insert(segment_index, worker_control);
    }

    let mut first_error = None;
    let mut paused = false;
    let mut cancelled = false;
    let mut pending_rebalance = None;
    let mut adaptive_growth_remaining = 0_usize;
    let target_connections = usize::from(max_connections);
    let mut segment_count = segments.len();
    let adaptive_policy = request.adaptive_segment_policy.unwrap_or_default();
    let adaptive_max_segments = usize::from(adaptive_policy.max_segments).max(segment_count);
    let adaptive_min_remaining_bytes =
        u64::from(adaptive_policy.min_segment_mib).saturating_mul(MIB);
    let mut adaptive = AdaptiveSegmentController::new(
        u16::try_from(adaptive_max_segments).unwrap_or(u16::MAX),
        adaptive_policy.enabled,
    );
    let mut adaptive_tick = tokio::time::interval(Duration::from_secs(1));
    adaptive_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    adaptive_tick.tick().await;
    let mut sampled_at = Instant::now();
    let mut sampled_bytes = progress_by_segment
        .lock()
        .map_err(|_| DownloadEngineError::LockPoisoned)?
        .values()
        .copied()
        .sum::<u64>();
    loop {
        let joined = tokio::select! {
            joined = transfers.join_next() => {
                let Some(joined) = joined else {
                    break;
                };
                joined
            }
            _ = adaptive_tick.tick(), if !paused && !cancelled && first_error.is_none() => {
                let now = Instant::now();
                let downloaded = progress_by_segment
                    .lock()
                    .map_err(|_| DownloadEngineError::LockPoisoned)?
                    .values()
                    .copied()
                    .sum::<u64>();
                let elapsed = now.duration_since(sampled_at).as_secs_f64().max(0.001);
                let speed = (downloaded.saturating_sub(sampled_bytes) as f64 / elapsed)
                    .min(u64::MAX as f64) as u64;
                sampled_at = now;
                sampled_bytes = downloaded;
                let current_segments = u16::try_from(segment_count).unwrap_or(u16::MAX);
                match adaptive.observe(
                    speed,
                    current_segments,
                    retry_observed.load(Ordering::Acquire),
                ) {
                    AdaptiveConnectionDecision::Grow if pending_rebalance.is_none() => {
                        adaptive_growth_remaining =
                            adaptive_max_segments.saturating_sub(segment_count).min(4);
                        adaptive.confirm_growth_target(
                            u16::try_from(
                                segment_count.saturating_add(adaptive_growth_remaining),
                            )
                            .unwrap_or(u16::MAX),
                        );
                        if let Some((candidate_index, remaining_bytes)) =
                            select_rebalance_candidate(
                                inner,
                                task_id,
                                &worker_controls,
                                &progress_by_segment,
                                adaptive_min_remaining_bytes,
                            )?
                            && let Some(candidate_control) = worker_controls.get(&candidate_index)
                        {
                            candidate_control.request_rebalance();
                            pending_rebalance = Some(candidate_index);
                            adaptive_growth_remaining =
                                adaptive_growth_remaining.saturating_sub(1);
                            info!(
                                event = "download.adaptive_segments.growth_requested",
                                %correlation_id,
                                ?task_id,
                                segment_index = candidate_index,
                                remaining_bytes,
                                sampled_bytes_per_second = speed,
                                current_segments,
                                target_segments = segment_count
                                    .saturating_add(adaptive_growth_remaining)
                                    .saturating_add(1),
                                max_segments = adaptive_max_segments,
                                max_connections,
                                "stable transfer speed allowed one adaptive segment split"
                            );
                        } else {
                            adaptive_growth_remaining = 0;
                            adaptive.cancel_growth();
                        }
                    }
                    AdaptiveConnectionDecision::DisabledAfterRetry => {
                        adaptive_growth_remaining = 0;
                        warn!(
                            event = "download.adaptive_segments.disabled_after_retry",
                            %correlation_id,
                            ?task_id,
                            current_segments,
                            "automatic segment splitting disabled after a segment network retry"
                        );
                    }
                    AdaptiveConnectionDecision::DisabledForLowBenefit => {
                        adaptive_growth_remaining = 0;
                        info!(
                            event = "download.adaptive_segments.disabled_low_benefit",
                            %correlation_id,
                            ?task_id,
                            current_segments,
                            sampled_bytes_per_second = speed,
                            "automatic segment splitting stopped because the added segments had insufficient benefit"
                        );
                    }
                    AdaptiveConnectionDecision::Grow => adaptive.cancel_growth(),
                    AdaptiveConnectionDecision::Hold => {}
                }
                continue;
            }
        };
        let (segment, outcome) =
            joined.map_err(|error| DownloadEngineError::Runtime(error.to_string()))?;
        worker_controls.remove(&segment.index());
        if pending_rebalance == Some(segment.index()) {
            pending_rebalance = None;
        }
        let mut released_slot = false;
        match outcome {
            Ok(DownloadSegmentTransferOutcome::Completed { downloaded_bytes }) => {
                checkpoint_segment(
                    inner,
                    task_id,
                    correlation_id,
                    &segment,
                    downloaded_bytes,
                    DownloadSegmentState::Completed,
                )?;
                released_slot = true;
            }
            Ok(DownloadSegmentTransferOutcome::Paused { downloaded_bytes }) => {
                checkpoint_segment(
                    inner,
                    task_id,
                    correlation_id,
                    &segment,
                    downloaded_bytes,
                    DownloadSegmentState::Waiting,
                )?;
                paused = true;
            }
            Ok(DownloadSegmentTransferOutcome::Cancelled { .. }) => {
                cancelled = true;
            }
            Ok(DownloadSegmentTransferOutcome::Rebalanced { downloaded_bytes }) => {
                let waiting = checkpoint_segment(
                    inner,
                    task_id,
                    correlation_id,
                    &segment,
                    downloaded_bytes,
                    DownloadSegmentState::Waiting,
                )?;
                if !paused && !cancelled && first_error.is_none() {
                    let mut plan = inner
                        .store
                        .load_transfer_plan(task_id)?
                        .ok_or(DownloadEngineError::EmptySegmentLayout(task_id))?;
                    let new_segment_index = plan
                        .segments()
                        .iter()
                        .map(DownloadSegment::index)
                        .max()
                        .unwrap_or_default()
                        .checked_add(1)
                        .ok_or_else(|| {
                            DownloadEngineError::Runtime(
                                "download segment index overflowed during rebalance".to_owned(),
                            )
                        })?;
                    match plan.split_segment_after_checkpoint(
                        segment.index(),
                        new_segment_index,
                        MIN_DYNAMIC_SEGMENT_BYTES,
                    ) {
                        Ok(()) => {
                            let reduced = plan
                                .segments()
                                .iter()
                                .find(|candidate| candidate.index() == segment.index())
                                .cloned()
                                .ok_or(DownloadEngineError::IncompleteSegmentLayout(task_id))?;
                            let added = plan
                                .segments()
                                .iter()
                                .find(|candidate| candidate.index() == new_segment_index)
                                .cloned()
                                .ok_or(DownloadEngineError::IncompleteSegmentLayout(task_id))?;
                            inner.persist_transfer_plan_change(task_id, &plan)?;
                            segment_count = segment_count.saturating_add(1);
                            info!(
                                event = "download.segment_split.persisted",
                                %correlation_id,
                                ?task_id,
                                source_segment_index = segment.index(),
                                new_segment_index,
                                reduced_start = reduced.start_byte(),
                                reduced_end = reduced.end_byte(),
                                new_start = added.start_byte(),
                                new_end = added.end_byte(),
                                "durable slow-segment split persisted"
                            );
                            for waiting_segment in [reduced, added] {
                                progress_by_segment
                                    .lock()
                                    .map_err(|_| DownloadEngineError::LockPoisoned)?
                                    .insert(
                                        waiting_segment.index(),
                                        waiting_segment.downloaded_bytes(),
                                    );
                                segment_lengths
                                    .lock()
                                    .map_err(|_| DownloadEngineError::LockPoisoned)?
                                    .insert(waiting_segment.index(), waiting_segment.len());
                                if worker_controls.len() >= usize::from(max_connections) {
                                    waiting_segments.push_back(waiting_segment);
                                    continue;
                                }
                                let active = checkpoint_segment(
                                    inner,
                                    task_id,
                                    correlation_id,
                                    &waiting_segment,
                                    waiting_segment.downloaded_bytes(),
                                    DownloadSegmentState::Active,
                                )?;
                                let active_index = active.index();
                                let worker_control = spawn_segment_transfer(
                                    &mut transfers,
                                    inner,
                                    task_id,
                                    correlation_id,
                                    request,
                                    probe,
                                    control,
                                    &artifact,
                                    &progress_by_segment,
                                    &segment_lengths,
                                    &active_connections,
                                    &retry_observed,
                                    total_bytes,
                                    use_if_range,
                                    active,
                                );
                                worker_controls.insert(active_index, worker_control);
                            }
                        }
                        Err(TransferPlanError::InsufficientSplitTail { .. }) => {
                            let active = DownloadSegment::new(
                                waiting.index(),
                                waiting.start_byte(),
                                waiting.end_byte(),
                                waiting.downloaded_bytes(),
                                DownloadSegmentState::Active,
                            )?;
                            inner.store.save_segment_checkpoint(task_id, &active)?;
                            let active_index = active.index();
                            let worker_control = spawn_segment_transfer(
                                &mut transfers,
                                inner,
                                task_id,
                                correlation_id,
                                request,
                                probe,
                                control,
                                &artifact,
                                &progress_by_segment,
                                &segment_lengths,
                                &active_connections,
                                &retry_observed,
                                total_bytes,
                                use_if_range,
                                active,
                            );
                            worker_controls.insert(active_index, worker_control);
                        }
                        Err(error) => return Err(error.into()),
                    }
                }
            }
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            }
        }

        if retry_observed.load(Ordering::Acquire) {
            adaptive_growth_remaining = 0;
        }
        while worker_controls.len() < target_connections
            && !paused
            && !cancelled
            && first_error.is_none()
            && let Some(segment) = waiting_segments.pop_front()
        {
            let active = checkpoint_segment(
                inner,
                task_id,
                correlation_id,
                &segment,
                segment.downloaded_bytes(),
                DownloadSegmentState::Active,
            )?;
            let segment_index = active.index();
            let worker_control = spawn_segment_transfer(
                &mut transfers,
                inner,
                task_id,
                correlation_id,
                request,
                probe,
                control,
                &artifact,
                &progress_by_segment,
                &segment_lengths,
                &active_connections,
                &retry_observed,
                total_bytes,
                use_if_range,
                active,
            );
            worker_controls.insert(segment_index, worker_control);
        }
        let continuing_adaptive_growth = adaptive_growth_remaining > 0;
        let can_split = adaptive_policy.enabled && segment_count < adaptive_max_segments;
        let should_rebalance = can_split
            && (released_slot && worker_controls.len() < target_connections
                || continuing_adaptive_growth);
        if should_rebalance
            && !paused
            && !cancelled
            && first_error.is_none()
            && pending_rebalance.is_none()
        {
            let minimum_remaining_bytes = adaptive_min_remaining_bytes;
            if let Some((candidate_index, remaining_bytes)) = select_rebalance_candidate(
                inner,
                task_id,
                &worker_controls,
                &progress_by_segment,
                minimum_remaining_bytes,
            )? && let Some(candidate_control) = worker_controls.get(&candidate_index)
            {
                candidate_control.request_rebalance();
                pending_rebalance = Some(candidate_index);
                if continuing_adaptive_growth {
                    adaptive_growth_remaining = adaptive_growth_remaining.saturating_sub(1);
                }
                info!(
                    event = "download.segment_rebalance.requested",
                    %correlation_id,
                    ?task_id,
                    segment_index = candidate_index,
                    remaining_bytes,
                    active_connections = worker_controls.len(),
                    target_connections,
                    adaptive_growth = continuing_adaptive_growth,
                    "connection target assigned to the largest unwritten tail"
                );
            } else if continuing_adaptive_growth {
                adaptive_growth_remaining = 0;
                adaptive.cancel_growth();
            }
        }
    }

    let observed_downloaded_bytes = progress_by_segment
        .lock()
        .map_err(|_| DownloadEngineError::LockPoisoned)?
        .values()
        .copied()
        .sum::<u64>();
    let final_plan = inner
        .store
        .load_transfer_plan(task_id)?
        .ok_or(DownloadEngineError::EmptySegmentLayout(task_id))?;
    let durable_downloaded_bytes = final_plan
        .segments()
        .iter()
        .map(DownloadSegment::downloaded_bytes)
        .sum();

    if cancelled {
        remove_path_if_exists(&temporary_path)?;
        transition_transfer_to_terminal(
            inner,
            task_id,
            DownloadStatus::Cancelled,
            durable_downloaded_bytes,
        )?;
        return Ok(());
    }
    if paused {
        transition_transfer_to_terminal(
            inner,
            task_id,
            DownloadStatus::Paused,
            durable_downloaded_bytes,
        )?;
        return Ok(());
    }
    if let Some(error) = first_error {
        if !use_if_range && error.allows_clean_single_stream_fallback() {
            warn!(
                event = "download.segment_transfer.fallback",
                %correlation_id,
                ?task_id,
                reason = error.kind(),
                segment_count = final_plan.segments().len(),
                discarded_downloaded_bytes = observed_downloaded_bytes,
                "range transfers rejected for a fresh task; discarding segmented work and restarting as a single stream"
            );
            inner.store.clear_transfer_plan(task_id)?;
            remove_path_if_exists(&temporary_path)?;
            let mut fallback_probe = probe.clone();
            fallback_probe.supports_ranges = false;
            {
                let mut tasks = inner.lock_tasks()?;
                let state = tasks
                    .get_mut(&task_id)
                    .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
                state.transfer.probe = Some(fallback_probe.clone());
                state.transfer.downloaded_bytes = 0;
                inner.persist(state)?;
            }
            return run_single_stream_task(
                inner,
                task_id,
                correlation_id,
                request,
                &fallback_probe,
                control,
                "range_validation_fallback",
            )
            .await;
        }
        {
            let mut tasks = inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            // Progress callbacks are intentionally optimistic. On failure, publish and persist
            // only bytes covered by a durable segment checkpoint so a later resume never appears
            // to move backwards.
            state.transfer.downloaded_bytes = durable_downloaded_bytes;
            inner.persist(state)?;
        }
        return Err(error.into());
    }

    if final_plan.segments().iter().any(|segment| {
        segment.state != DownloadSegmentState::Completed
            || segment.downloaded_bytes() != segment.len()
    }) || durable_downloaded_bytes != total_bytes
    {
        return Err(DownloadEngineError::IncompleteSegmentLayout(task_id));
    }

    {
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.downloaded_bytes = total_bytes;
        state
            .task
            .transition_to(DownloadStatus::Finalizing, now_unix_ms())?;
        inner.persist(state)?;
        inner.emit(DownloadEvent::TaskChanged {
            task: state.task.clone(),
        });
    }
    inner
        .http
        .commit_prepared(PreparedDownload {
            temporary_path,
            final_path,
            downloaded_bytes: total_bytes,
        })
        .await?;
    commit_completed_state(inner, task_id)?;
    Ok(())
}

const MIN_INITIAL_SEGMENT_BYTES: u64 = 64 * 1024;
const MIN_DYNAMIC_SEGMENT_BYTES: u64 = 32 * 1024;
const MAX_SEGMENT_TRANSFER_ATTEMPTS: u32 = 3;
const SEGMENT_RETRY_BASE_DELAY: Duration = Duration::from_millis(100);

fn segment_retry_delay(failed_attempt: u32) -> Duration {
    SEGMENT_RETRY_BASE_DELAY.saturating_mul(1 << failed_attempt.saturating_sub(1).min(6))
}

fn create_initial_segments(
    total_bytes: u64,
    connection_limit: Option<u16>,
) -> Result<Vec<DownloadSegment>, DownloadSegmentError> {
    let requested = u64::from(connection_limit.unwrap_or(1).clamp(1, 64));
    let useful = total_bytes.div_ceil(MIN_INITIAL_SEGMENT_BYTES).max(1);
    let segment_count = requested.min(useful);
    let base_length = total_bytes / segment_count;
    let remainder = total_bytes % segment_count;
    let mut start_byte = 0_u64;
    let mut segments = Vec::with_capacity(segment_count as usize);
    for index in 0..segment_count {
        let length = base_length + u64::from(index < remainder);
        let end_byte = start_byte + length - 1;
        segments.push(DownloadSegment::new(
            index as u32,
            start_byte,
            end_byte,
            0,
            DownloadSegmentState::Active,
        )?);
        start_byte = end_byte + 1;
    }
    Ok(segments)
}

fn checkpoint_segment(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    segment: &DownloadSegment,
    downloaded_bytes: u64,
    state: DownloadSegmentState,
) -> Result<DownloadSegment, DownloadEngineError> {
    let checkpoint = DownloadSegment::new(
        segment.index(),
        segment.start_byte(),
        segment.end_byte(),
        downloaded_bytes,
        state,
    )?;
    inner.store.save_segment_checkpoint(task_id, &checkpoint)?;
    info!(
        event = "download.segment_checkpoint.persisted",
        %correlation_id,
        ?task_id,
        segment_index = checkpoint.index(),
        downloaded_bytes,
        state = ?state,
        "segment checkpoint persisted after durable write"
    );
    Ok(checkpoint)
}

fn validate_resumed_plan(
    task_id: DownloadTaskId,
    plan: &TransferPlan,
    probe: &DownloadProbe,
) -> Result<(), DownloadEngineError> {
    if plan.protocol() != TransferProtocol::Http
        || !plan.supports_ranges()
        || plan.total_bytes() != probe.total_bytes
        || plan.validator() != &probe.validator
    {
        return Err(DownloadEngineError::ResourceChanged(task_id));
    }
    Ok(())
}

fn transition_transfer_to_terminal(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    status: DownloadStatus,
    downloaded_bytes: u64,
) -> Result<(), DownloadEngineError> {
    let mut tasks = inner.lock_tasks()?;
    let state = tasks
        .get_mut(&task_id)
        .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
    state.transfer.downloaded_bytes = downloaded_bytes;
    state.task.transition_to(status, now_unix_ms())?;
    state.control = None;
    inner.persist(state)?;
    inner.emit(DownloadEvent::TaskChanged {
        task: state.task.clone(),
    });
    Ok(())
}

fn publish_transfer_progress(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    active_connections: u16,
    progress_by_segment: Option<&Mutex<HashMap<u32, u64>>>,
    segment_lengths: Option<&Mutex<HashMap<u32, u64>>>,
) {
    if let Ok(mut tasks) = inner.tasks.lock()
        && let Some(state) = tasks.get_mut(&task_id)
    {
        state.transfer.downloaded_bytes = state.transfer.downloaded_bytes.max(downloaded_bytes);
        let now = Instant::now();
        let should_publish = state
            .last_progress_event
            .is_none_or(|last| now.duration_since(last) >= Duration::from_millis(80))
            || total_bytes == Some(downloaded_bytes);
        if should_publish {
            state.last_progress_event = Some(now);
            let segments = progress_by_segment
                .and_then(|values| values.lock().ok())
                .map(|values| {
                    let lengths = segment_lengths.and_then(|lengths| lengths.lock().ok());
                    download_segment_progress_snapshot(&values, lengths.as_deref())
                })
                .unwrap_or_default();
            let published_downloaded_bytes = if progress_by_segment.is_some() {
                segments
                    .iter()
                    .map(|segment| segment.downloaded_bytes)
                    .sum()
            } else {
                downloaded_bytes
            };
            state.transfer.downloaded_bytes = published_downloaded_bytes;
            let bytes_per_second = state
                .transfer_rate
                .get_or_insert_with(|| TransferRateSampler::new(published_downloaded_bytes, now))
                .sample(published_downloaded_bytes, now);
            inner.emit(DownloadEvent::TaskProgress {
                progress: DownloadProgress {
                    task_id,
                    downloaded_bytes: published_downloaded_bytes,
                    total_bytes,
                    bytes_per_second,
                    active_connections,
                    segments,
                },
            });
        }
    }
}

fn download_segment_progress_snapshot(
    values: &HashMap<u32, u64>,
    lengths: Option<&HashMap<u32, u64>>,
) -> Vec<DownloadSegmentProgress> {
    let mut segments = values
        .iter()
        .map(
            |(segment_index, downloaded_bytes)| DownloadSegmentProgress {
                segment_index: *segment_index,
                downloaded_bytes: *downloaded_bytes,
                total_bytes: lengths.and_then(|lengths| lengths.get(segment_index).copied()),
            },
        )
        .collect::<Vec<_>>();
    segments.sort_by_key(|segment| segment.segment_index);
    segments
}

fn commit_completed_state(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
) -> Result<(), DownloadEngineError> {
    let mut tasks = inner.lock_tasks()?;
    let state = tasks
        .get_mut(&task_id)
        .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
    let mut committed = state.clone();
    committed
        .task
        .transition_to(DownloadStatus::Completed, now_unix_ms())?;
    committed.transfer.final_committed = true;
    committed.control = None;
    inner.persist(&committed)?;
    *state = committed;
    inner.emit(DownloadEvent::TaskChanged {
        task: state.task.clone(),
    });
    Ok(())
}

fn record_failure(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    error: DownloadEngineError,
) {
    let Ok(mut tasks) = inner.tasks.lock() else {
        return;
    };
    let Some(state) = tasks.get_mut(&task_id) else {
        return;
    };
    let fault = error.fault();
    warn!(
        event = "download.task.failed",
        %correlation_id,
        ?task_id,
        fault_code = ?fault.code,
        retryable = fault.retryable,
        native_error = error.native_os_error().unwrap_or_default(),
        "download task reached a terminal failure"
    );
    state.transfer.error = Some(fault);
    if state
        .task
        .transition_to(DownloadStatus::Failed, now_unix_ms())
        .is_err()
    {
        return;
    }
    state.control = None;
    let _ = inner.persist(state);
    inner.emit(DownloadEvent::TaskChanged {
        task: state.task.clone(),
    });
}

fn snapshot_from_state(state: &EngineTaskState) -> DownloadEngineTask {
    let file_name = state.task.request().file_name.as_deref().or_else(|| {
        state
            .transfer
            .probe
            .as_ref()
            .and_then(|probe| probe.file_name.as_deref())
    });
    DownloadEngineTask {
        task: state.task.clone(),
        probe: state.transfer.probe.clone(),
        downloaded_bytes: state.transfer.downloaded_bytes,
        total_bytes: state
            .transfer
            .probe
            .as_ref()
            .and_then(|probe| probe.total_bytes),
        error: state.transfer.error.clone(),
        final_path: state
            .transfer
            .final_path
            .clone()
            .or_else(|| file_name.map(|name| state.task.request().save_directory.join(name))),
        temporary_path: state.transfer.temporary_path.clone(),
        queue_eligible: false,
    }
}

fn remove_task_temporary_artifact(state: &EngineTaskState) {
    if let Some(path) = state.transfer.temporary_path.as_deref() {
        let _ = remove_path_if_exists(path);
    }
}

fn remove_path_if_exists(path: &Path) -> Result<(), std::io::Error> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
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

fn daily_schedule_desired_running(
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

fn next_daily_schedule_trigger(
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

fn daily_schedule_wake_moments(
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

fn download_runtime() -> Result<&'static tokio::runtime::Runtime, DownloadEngineError> {
    DOWNLOAD_RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_name("nxtools-download")
                .enable_all()
                .build()
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(|message| DownloadEngineError::Runtime(message.clone()))
}

fn now_unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

#[derive(Debug, Error)]
pub enum DownloadEngineError {
    #[error(transparent)]
    InvalidRequest(#[from] DownloadRequestError),
    #[error(transparent)]
    Task(#[from] DownloadTaskError),
    #[error(transparent)]
    Storage(#[from] DownloadStorageError),
    #[error(transparent)]
    Http(#[from] HttpDownloadError),
    #[error(transparent)]
    Segment(#[from] DownloadSegmentError),
    #[error(transparent)]
    TransferPlan(#[from] TransferPlanError),
    #[error(transparent)]
    RateLimit(#[from] DownloadRateLimitError),
    #[error(transparent)]
    StorageCapacity(#[from] DownloadStorageCapacityError),
    #[error(transparent)]
    Schedule(#[from] crate::DownloadScheduleError),
    #[error("download task does not exist: {0:?}")]
    TaskNotFound(DownloadTaskId),
    #[error("download task has no active transfer control: {0:?}")]
    MissingTransferControl(DownloadTaskId),
    #[error("download task has no known length for segmented transfer: {0:?}")]
    MissingSegmentedLength(DownloadTaskId),
    #[error("download task has an empty persisted segment plan: {0:?}")]
    EmptySegmentLayout(DownloadTaskId),
    #[error("download task has an incomplete persisted segment plan: {0:?}")]
    IncompleteSegmentLayout(DownloadTaskId),
    #[error("download resource changed while task {0:?} was paused")]
    ResourceChanged(DownloadTaskId),
    #[error("cannot {action} download task {task_id:?} while it is {status:?}")]
    InvalidAction {
        task_id: DownloadTaskId,
        action: &'static str,
        status: DownloadStatus,
    },
    #[error("download queue concurrency must be at least one")]
    InvalidQueueConcurrency,
    #[error("download task is not eligible for queue reordering: {0:?}")]
    InvalidQueueMove(DownloadTaskId),
    #[error("download engine task lock is poisoned")]
    LockPoisoned,
    #[error("the download engine event subscriber is already attached")]
    EventSubscriberAlreadyTaken,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("download runtime could not start: {0}")]
    Runtime(String),
    #[error("timed out waiting for download task: {0:?}")]
    WaitTimeout(DownloadTaskId),
}

impl DownloadEngineError {
    fn fault(&self) -> DownloadFault {
        if let Some(error) = self.io_error() {
            let code = if error.kind() == std::io::ErrorKind::PermissionDenied {
                DownloadFaultCode::Permission
            } else if is_disk_full_error(error) {
                DownloadFaultCode::DiskFull
            } else if is_file_in_use_error(error) {
                DownloadFaultCode::FileInUse
            } else {
                DownloadFaultCode::Storage
            };
            return DownloadFault {
                code,
                message: self.to_string(),
                retryable: code == DownloadFaultCode::Storage,
            };
        }
        let (code, retryable) = match self {
            Self::InvalidRequest(_) => (DownloadFaultCode::InvalidRequest, false),
            Self::Task(_)
            | Self::Segment(_)
            | Self::TransferPlan(_)
            | Self::RateLimit(_)
            | Self::Schedule(_)
            | Self::TaskNotFound(_)
            | Self::MissingTransferControl(_)
            | Self::MissingSegmentedLength(_)
            | Self::EmptySegmentLayout(_)
            | Self::IncompleteSegmentLayout(_)
            | Self::InvalidQueueConcurrency
            | Self::InvalidQueueMove(_)
            | Self::InvalidAction { .. } => (DownloadFaultCode::InvalidState, false),
            Self::StorageCapacity(_) => (DownloadFaultCode::Storage, false),
            Self::Storage(_) | Self::LockPoisoned | Self::Io(_) => {
                (DownloadFaultCode::Storage, true)
            }
            Self::EventSubscriberAlreadyTaken => (DownloadFaultCode::InvalidState, false),
            Self::Http(HttpDownloadError::InsufficientDiskSpace { .. }) => {
                (DownloadFaultCode::DiskFull, false)
            }
            Self::Http(HttpDownloadError::DestinationExists(_)) => {
                (DownloadFaultCode::Storage, false)
            }
            Self::Http(error) if error.is_retryable_transfer() => {
                (DownloadFaultCode::Network, true)
            }
            Self::ResourceChanged(_) => (DownloadFaultCode::Protocol, false),
            Self::Http(_) => (DownloadFaultCode::Protocol, false),
            Self::Runtime(_) | Self::WaitTimeout(_) => (DownloadFaultCode::Unavailable, true),
        };
        DownloadFault {
            code,
            message: self.to_string(),
            retryable,
        }
    }

    fn io_error(&self) -> Option<&std::io::Error> {
        match self {
            Self::Io(error) => Some(error),
            Self::Http(error) => error.io_error(),
            _ => None,
        }
    }

    fn native_os_error(&self) -> Option<i32> {
        self.io_error().and_then(std::io::Error::raw_os_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::InjectedArtifactFault;
    use crate::{ResourceValidator, SegmentedArtifactError};
    use std::io::{Read, Write};
    use std::net::TcpListener;

    struct FixedStorageCapacity(u64);

    impl StorageCapacityProbe for FixedStorageCapacity {
        fn available_space(&self, _path: &Path) -> Option<u64> {
            Some(self.0)
        }
    }

    #[test]
    fn adaptive_segments_grow_only_after_stable_speed_samples() {
        let mut controller = AdaptiveSegmentController::new(8, true);

        for speed in [10_000, 10_400, 9_800, 10_200] {
            assert_eq!(
                controller.observe(speed, 4, false),
                AdaptiveConnectionDecision::Hold
            );
        }
        assert_eq!(
            controller.observe(10_100, 4, false),
            AdaptiveConnectionDecision::Grow
        );
        controller.confirm_growth_target(8);
    }

    #[test]
    fn adaptive_segments_use_only_post_growth_samples_for_the_benefit_window() {
        let mut controller = AdaptiveSegmentController::new(8, true);
        for speed in [10_000, 10_100, 9_900, 10_000, 10_050] {
            let _ = controller.observe(speed, 4, false);
        }
        controller.confirm_growth_target(8);

        for speed in [10_100, 10_200, 10_000, 10_150, 10_100] {
            assert_eq!(
                controller.observe(speed, 5, false),
                AdaptiveConnectionDecision::Hold
            );
        }
        for speed in [10_100, 10_200, 10_000, 10_150] {
            assert_eq!(
                controller.observe(speed, 8, false),
                AdaptiveConnectionDecision::Hold
            );
        }
        assert_eq!(
            controller.observe(10_100, 8, false),
            AdaptiveConnectionDecision::DisabledForLowBenefit
        );
        assert_eq!(
            controller.observe(40_000, 8, false),
            AdaptiveConnectionDecision::Hold
        );
    }

    #[test]
    fn adaptive_segments_stop_after_a_segment_network_retry() {
        let mut controller = AdaptiveSegmentController::new(8, true);

        assert_eq!(
            controller.observe(10_000, 4, true),
            AdaptiveConnectionDecision::DisabledAfterRetry
        );
        for _ in 0..5 {
            assert_eq!(
                controller.observe(20_000, 4, false),
                AdaptiveConnectionDecision::Hold
            );
        }
    }

    #[test]
    fn adaptive_segments_respect_the_user_disable_switch() {
        let mut controller = AdaptiveSegmentController::new(32, false);

        for _ in 0..10 {
            assert_eq!(
                controller.observe(20_000, 8, false),
                AdaptiveConnectionDecision::Hold
            );
        }
    }

    #[test]
    fn task_detail_command_returns_only_the_selected_tasks_segments_and_bounded_timeline() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/detail.bin".into(),
                    save_directory: std::env::temp_dir(),
                    file_name: Some("detail.bin".into()),
                    headers: vec![crate::DownloadRequestHeader {
                        name: "Authorization".into(),
                        value: "secret-value-must-not-enter-the-timeline".into(),
                    }],
                    cookie: Some("private-cookie".into()),
                    referrer: None,
                    connection_limit: Some(2),
                    speed_limit_bytes_per_second: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue task");
        let plan = TransferPlan::new(
            TransferProtocol::Http,
            "https://cdn.example.test/detail.bin".into(),
            Some(16),
            true,
            ResourceValidator {
                etag: Some("\"fixture\"".into()),
                last_modified: None,
            },
            vec![
                DownloadSegment::new(0, 0, 7, 4, DownloadSegmentState::Active)
                    .expect("first segment"),
                DownloadSegment::new(1, 8, 15, 8, DownloadSegmentState::Completed)
                    .expect("second segment"),
            ],
        )
        .expect("transfer plan");
        engine
            .inner
            .store
            .save_transfer_plan(task_id, &plan)
            .expect("save transfer plan");

        let result = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Ui,
            command: DownloadCommand::TaskDetail { task_id },
        });
        let DownloadCommandOutcome::TaskDetail(detail) = result.outcome else {
            panic!("task detail command must return a typed detail snapshot");
        };

        assert_eq!(detail.task.task.id(), task_id);
        assert_eq!(detail.segments, plan.segments());
        assert!(detail.timeline.iter().any(|entry| matches!(
            entry.event,
            crate::DownloadTaskTimelineEvent::TransferPlanChanged { segment_count: 2 }
        )));
        let timeline_json = serde_json::to_string(&detail.timeline).expect("serialize timeline");
        assert!(!timeline_json.contains("secret-value"));
        assert!(!timeline_json.contains("private-cookie"));
        assert!(detail.timeline.len() <= 300);
    }

    #[test]
    fn persisted_transfer_plan_change_emits_a_coarse_task_refresh_event() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let events = engine.take_event_receiver().expect("take event receiver");
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/rebalanced.bin".into(),
                    save_directory: std::env::temp_dir(),
                    file_name: Some("rebalanced.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    connection_limit: Some(2),
                    speed_limit_bytes_per_second: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue task");
        while events.try_recv().is_ok() {}
        let plan = TransferPlan::new(
            TransferProtocol::Http,
            "https://example.test/rebalanced.bin".into(),
            Some(32),
            true,
            ResourceValidator::default(),
            vec![
                DownloadSegment::new(0, 0, 23, 8, DownloadSegmentState::Active)
                    .expect("first segment"),
                DownloadSegment::new(1, 24, 31, 0, DownloadSegmentState::Waiting)
                    .expect("new segment"),
            ],
        )
        .expect("transfer plan");

        engine
            .inner
            .persist_transfer_plan_change(task_id, &plan)
            .expect("persist plan change");

        let event = events.recv_blocking().expect("task refresh event");
        assert!(matches!(
            event.event,
            DownloadEvent::TaskChanged { task } if task.id() == task_id
        ));
    }

    #[test]
    fn segmented_progress_total_and_segment_snapshot_are_published_atomically() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let events = engine.take_event_receiver().expect("take event receiver");
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/progress.bin".into(),
                    save_directory: std::env::temp_dir(),
                    file_name: Some("progress.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    connection_limit: Some(2),
                    speed_limit_bytes_per_second: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue task");
        while events.try_recv().is_ok() {}
        let segment_progress = Mutex::new(HashMap::from([(0, 5), (1, 7)]));
        let segment_lengths = Mutex::new(HashMap::from([(0, 10), (1, 10)]));

        publish_transfer_progress(
            &engine.inner,
            task_id,
            5,
            Some(20),
            2,
            Some(&segment_progress),
            Some(&segment_lengths),
        );

        let event = events.recv_blocking().expect("progress event");
        let DownloadEvent::TaskProgress { progress } = event.event else {
            panic!("expected a task progress event");
        };
        assert_eq!(progress.downloaded_bytes, 12);
        assert_eq!(
            progress
                .segments
                .iter()
                .map(|segment| segment.downloaded_bytes)
                .sum::<u64>(),
            progress.downloaded_bytes
        );
        assert_eq!(
            progress
                .segments
                .iter()
                .map(|segment| segment.total_bytes)
                .collect::<Vec<_>>(),
            vec![Some(10), Some(10)]
        );
    }

    #[test]
    fn daily_scheduler_arms_the_nearest_enabled_trigger_instead_of_polling() {
        let now = Local
            .with_ymd_and_hms(2026, 7, 31, 23, 0, 0)
            .earliest()
            .expect("valid local fixture time");
        let queue = DownloadQueue {
            id: "default".into(),
            name: "默认队列".into(),
            position: 0,
            max_concurrent_tasks: Some(3),
            connection_limit: None,
            speed_limit_bytes_per_second: None,
            default_save_directory: None,
            schedule: Some(DownloadSchedule {
                daily: DownloadDailySchedule {
                    days: vec![DownloadWeekday::Friday],
                    start_minute: 22 * 60 + 30,
                    stop_minute: 2 * 60 + 15,
                    enabled_start: true,
                    enabled_stop: true,
                },
            }),
            is_running: false,
        };

        let next = next_daily_schedule_trigger(&[queue], now).expect("next trigger");

        assert_eq!(next.weekday(), chrono::Weekday::Sat);
        assert_eq!(next.hour(), 2);
        assert_eq!(next.minute(), 15);
    }

    #[test]
    fn late_daily_wake_retains_the_scheduled_moment_for_independent_triggers() {
        let scheduled = Local
            .with_ymd_and_hms(2026, 7, 31, 9, 0, 0)
            .earliest()
            .expect("valid scheduled local time");
        let current = Local
            .with_ymd_and_hms(2026, 7, 31, 9, 12, 0)
            .earliest()
            .expect("valid delayed local time");
        let [scheduled_moment, current_moment] = daily_schedule_wake_moments(scheduled, current);
        let start_only = DownloadDailySchedule {
            days: vec![DownloadWeekday::Friday],
            start_minute: 9 * 60,
            stop_minute: 0,
            enabled_start: true,
            enabled_stop: false,
        };

        assert_eq!(
            daily_schedule_desired_running(&start_only, scheduled_moment),
            Some(true)
        );
        assert_eq!(
            daily_schedule_desired_running(&start_only, current_moment),
            None
        );
    }

    #[test]
    fn io_faults_preserve_permission_disk_full_and_file_in_use_categories() {
        let permission = DownloadEngineError::Http(HttpDownloadError::Io(std::io::Error::from(
            std::io::ErrorKind::PermissionDenied,
        )))
        .fault();
        assert_eq!(permission.code, DownloadFaultCode::Permission);
        assert!(!permission.retryable);

        let disk_full = DownloadEngineError::Http(HttpDownloadError::Io(
            std::io::Error::from_raw_os_error(112),
        ))
        .fault();
        assert_eq!(disk_full.code, DownloadFaultCode::DiskFull);
        assert!(!disk_full.retryable);

        let wrapped_disk_full = DownloadEngineError::Http(HttpDownloadError::Artifact(
            SegmentedArtifactError::Io(std::io::Error::from_raw_os_error(112)),
        ))
        .fault();
        assert_eq!(wrapped_disk_full.code, DownloadFaultCode::DiskFull);
        assert!(!wrapped_disk_full.retryable);

        let file_in_use = DownloadEngineError::Http(HttpDownloadError::Artifact(
            SegmentedArtifactError::Io(std::io::Error::from_raw_os_error(32)),
        ))
        .fault();
        assert_eq!(file_in_use.code, DownloadFaultCode::FileInUse);
        assert!(!file_in_use.retryable);
    }

    #[tokio::test]
    async fn artifact_boundary_faults_fail_the_real_engine_without_publishing_a_file() {
        for (injected_fault, expected_code) in [
            (
                InjectedArtifactFault::PermissionDenied,
                DownloadFaultCode::Permission,
            ),
            (InjectedArtifactFault::DiskFull, DownloadFaultCode::DiskFull),
            (
                InjectedArtifactFault::FileInUse,
                DownloadFaultCode::FileInUse,
            ),
        ] {
            let (source_url, server) = one_head_response_server();
            let directory = tempfile::tempdir().expect("create fault directory");
            let destination = directory.path().join("artifacts");
            let http = HttpDownloadClient::new()
                .expect("create HTTP client")
                .with_injected_artifact_fault(injected_fault);
            let engine = DownloadEngine::from_store_with_http(
                SqliteDownloadStore::open_in_memory().expect("create store"),
                http,
            )
            .expect("create faulting engine");
            let request = DownloadRequest {
                source_url,
                save_directory: destination.clone(),
                file_name: Some("must-not-exist.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
            };

            let task_id = engine
                .submit(request, RequestOrigin::Internal)
                .expect("submit faulting task");
            let failed = engine
                .wait_for_terminal(task_id, Duration::from_secs(2))
                .await
                .expect("wait for injected failure");

            assert_eq!(failed.task.status(), DownloadStatus::Failed);
            assert_eq!(
                failed.error.as_ref().map(|fault| fault.code),
                Some(expected_code)
            );
            assert!(!destination.join("must-not-exist.bin").exists());
            server.join().expect("join HTTP fixture");
        }
    }

    #[tokio::test]
    async fn insufficient_capacity_fails_before_artifact_or_get_request() {
        let (source_url, server) = one_head_response_server();
        let directory = tempfile::tempdir().expect("create capacity directory");
        let destination = directory.path().join("artifacts");
        let engine = DownloadEngine::from_store_with_http_and_capacity(
            SqliteDownloadStore::open_in_memory().expect("create store"),
            HttpDownloadClient::new().expect("create HTTP client"),
            Arc::new(FixedStorageCapacity(0)),
        )
        .expect("create capacity-limited engine");
        let request = DownloadRequest {
            source_url,
            save_directory: destination.clone(),
            file_name: Some("must-not-exist.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
        };

        let task_id = engine
            .submit(request, RequestOrigin::Internal)
            .expect("submit capacity-limited task");
        let failed = engine
            .wait_for_terminal(task_id, Duration::from_secs(2))
            .await
            .expect("wait for capacity failure");

        assert_eq!(failed.task.status(), DownloadStatus::Failed);
        assert_eq!(
            failed.error.as_ref().map(|fault| fault.code),
            Some(DownloadFaultCode::DiskFull)
        );
        assert!(!destination.join("must-not-exist.bin").exists());
        server.join().expect("join HTTP fixture");
    }

    #[test]
    fn failed_completion_transaction_never_leaks_completed_into_memory() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let directory = tempfile::tempdir().expect("create artifact directory");
        let mut task = DownloadTask::new(
            DownloadRequest {
                source_url: "https://example.test/final.bin".into(),
                save_directory: directory.path().to_path_buf(),
                file_name: Some("final.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
            },
            RequestOrigin::Internal,
            1_000,
        );
        for status in [
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
            DownloadStatus::Finalizing,
        ] {
            task.transition_to(status, 1_100)
                .expect("advance task to finalizing");
        }
        let task_id = task.id();
        let transfer = StoredTransferState {
            probe: Some(DownloadProbe {
                final_url: task.request().source_url.clone(),
                file_name: Some("final.bin".into()),
                total_bytes: Some(16),
                supports_ranges: false,
                content_type: None,
                validator: crate::ResourceValidator::default(),
            }),
            downloaded_bytes: 16,
            error: None,
            temporary_path: Some(directory.path().join(".final.part")),
            final_path: Some(directory.path().join("final.bin")),
            final_committed: false,
        };
        engine
            .inner
            .store
            .save_task_with_transfer(&task, &transfer)
            .expect("persist finalizing state");
        engine.inner.tasks.lock().expect("lock tasks").insert(
            task_id,
            EngineTaskState {
                task,
                transfer,
                control: None,
                last_progress_event: None,
                transfer_rate: None,
            },
        );
        engine
            .inner
            .store
            .inject_artifact_update_failure()
            .expect("inject database failure");

        assert!(commit_completed_state(&engine.inner, task_id).is_err());
        let snapshot = engine
            .task(task_id)
            .expect("read task")
            .expect("task exists");
        assert_eq!(snapshot.task.status(), DownloadStatus::Finalizing);
        assert!(
            !engine
                .inner
                .store
                .load_transfer_state(task_id)
                .expect("load transfer")
                .final_committed
        );
    }

    #[test]
    fn progress_pressure_keeps_the_queue_bounded_and_preserves_state_order() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let receiver = engine
            .take_event_receiver()
            .expect("attach event subscriber");
        let task_id = DownloadTaskId::new();
        for downloaded_bytes in 0..400 {
            engine.inner.emit(DownloadEvent::TaskProgress {
                progress: DownloadProgress {
                    task_id,
                    downloaded_bytes,
                    total_bytes: Some(400),
                    bytes_per_second: 0,
                    active_connections: 1,
                    segments: Vec::new(),
                },
            });
        }
        engine.inner.emit(DownloadEvent::TaskRemoved { task_id });

        assert!(receiver.len() <= 193);
        let events = std::iter::from_fn(|| receiver.try_recv().ok()).collect::<Vec<_>>();
        assert!(
            events
                .windows(2)
                .all(|pair| pair[0].sequence < pair[1].sequence)
        );
        assert!(matches!(
            events.last().map(|event| &event.event),
            Some(DownloadEvent::TaskRemoved { task_id: removed }) if *removed == task_id
        ));
    }

    #[test]
    fn additional_event_subscribers_receive_the_same_authoritative_state_event() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let desktop = engine
            .take_event_receiver()
            .expect("attach desktop subscriber");
        let cli = engine.subscribe_events().expect("attach CLI subscriber");
        let task_id = DownloadTaskId::new();

        engine.inner.emit(DownloadEvent::TaskRemoved { task_id });

        let desktop_event = desktop.try_recv().expect("desktop event");
        let cli_event = cli.try_recv().expect("CLI event");
        assert_eq!(desktop_event, cli_event);
    }

    #[test]
    fn a_stalled_secondary_subscriber_cannot_block_authoritative_state_changes() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let primary = engine
            .take_event_receiver()
            .expect("attach primary event drain");
        std::thread::spawn(move || while primary.recv_blocking().is_ok() {});
        let _stalled = engine
            .subscribe_events()
            .expect("attach stalled subscriber");
        let (completed, completion) = std::sync::mpsc::channel();
        let emitting_engine = engine.clone();
        std::thread::spawn(move || {
            for _ in 0..300 {
                emitting_engine.inner.emit(DownloadEvent::TaskRemoved {
                    task_id: DownloadTaskId::new(),
                });
            }
            let _ = completed.send(());
        });

        completion
            .recv_timeout(Duration::from_secs(1))
            .expect("stalled secondary subscriber must be disconnected, not block the engine");
    }

    fn one_head_response_server() -> (String, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind HTTP fixture");
        let address = listener.local_addr().expect("read HTTP fixture address");
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept HTTP probe");
            let mut request = [0_u8; 2048];
            let _ = stream.read(&mut request);
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\n")
                .expect("write HEAD response");
        });
        (format!("http://{address}/fixture.bin"), server)
    }
}
#[test]
fn transfer_rate_sampler_reports_nonzero_rate_after_the_publish_interval() {
    let started_at = Instant::now();
    let mut sampler = TransferRateSampler::new(0, started_at);

    assert_eq!(
        sampler.sample(524_288, started_at + Duration::from_millis(100)),
        0
    );
    assert_eq!(
        sampler.sample(2_621_440, started_at + Duration::from_millis(500)),
        5_242_880
    );
    assert_eq!(
        sampler.sample(2_883_584, started_at + Duration::from_millis(600)),
        5_242_880
    );
}

#[test]
fn transfer_rate_sampler_smooths_short_lived_network_jitter() {
    let started_at = Instant::now();
    let mut sampler = TransferRateSampler::new(0, started_at);

    assert_eq!(
        sampler.sample(5_242_880, started_at + Duration::from_millis(500)),
        10_485_760
    );
    assert_eq!(
        sampler.sample(6_291_456, started_at + Duration::from_millis(1_000)),
        8_388_608
    );
}
