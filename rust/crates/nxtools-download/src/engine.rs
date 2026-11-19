use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use nxtools_core::{DownloadSettings, NetworkSettings};
use nxtools_network::proxy_mode_name;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn};

use crate::capacity::{StorageCapacityProbe, SystemStorageCapacityProbe};
use crate::http::{
    HlsKeyFetchOutcome, HlsSegmentDownloadOutcome, PreparedDownload, PreparedDownloadOutcome,
    TransferDirective, decrypt_hls_aes128_segment, hls_sequence_iv, is_disk_full_error,
    is_file_in_use_error,
};
use crate::storage::{
    IdempotentSubmissionSave, QueueAdmission, RedownloadOperationSave, StoredTransferState,
};
use crate::{
    ArtifactAllocation, BrowserBlob, DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadBatchAction,
    DownloadBatchItemResult, DownloadCommand, DownloadCommandEnvelope, DownloadCommandOutcome,
    DownloadCommandResult, DownloadEvent, DownloadEventEnvelope, DownloadEventSequencer,
    DownloadFault, DownloadFaultCode, DownloadFinishingProgress, DownloadFinishingStage,
    DownloadHttpPolicy, DownloadHttpPolicyError, DownloadOrganizationPolicy,
    DownloadOrganizationPolicyError, DownloadProbe, DownloadProgress, DownloadRateLimitError,
    DownloadRateLimiter, DownloadRequest, DownloadRequestError, DownloadRequestMedia,
    DownloadSegment, DownloadSegmentError, DownloadSegmentProgress, DownloadSegmentState,
    DownloadSegmentTransferControl, DownloadStatus, DownloadStorageCapacityError,
    DownloadStorageError, DownloadStorageRequirement, DownloadSubmitMode, DownloadTask,
    DownloadTaskAction, DownloadTaskError, DownloadTaskId, DownloadTransferContext,
    DownloadTransferControl, DownloadWeekday, FfmpegInstallControl, FfmpegInstallOperation,
    HttpDownloadClient, HttpDownloadError, NetworkPolicy, NetworkPolicyError, RequestOrigin,
    SegmentedArtifact, SqliteDownloadStore, TransferPlan, TransferPlanError, TransferProtocol,
};

static DOWNLOAD_RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

#[derive(Clone)]
pub struct DownloadEngine {
    inner: Arc<DownloadEngineInner>,
}

#[derive(Clone)]
pub struct WeakDownloadEngine(std::sync::Weak<DownloadEngineInner>);
impl WeakDownloadEngine {
    pub fn upgrade(&self) -> Option<DownloadEngine> {
        self.0.upgrade().map(|inner| DownloadEngine { inner })
    }
}
impl DownloadEngine {
    pub fn downgrade(&self) -> WeakDownloadEngine {
        WeakDownloadEngine(Arc::downgrade(&self.inner))
    }
}

type OperationHttpClients = HashMap<(u64, String, bool), (HttpDownloadClient, Instant)>;

pub(crate) struct DownloadEngineInner {
    http_template: HttpDownloadClient,
    http_clients: Mutex<OperationHttpClients>,
    connection_ceilings: Mutex<connection_ceiling::ConnectionCeilings>,
    network_policy: NetworkPolicy,
    download_http_policy: DownloadHttpPolicy,
    download_organization_policy: DownloadOrganizationPolicy,
    ffmpeg_executable: Mutex<Option<PathBuf>>,
    ffmpeg_install_active: AtomicBool,
    store: SqliteDownloadStore,
    global_rate_limiter: DownloadRateLimiter,
    global_speed_limits: Mutex<GlobalSpeedLimits>,
    capacity_probe: Arc<dyn StorageCapacityProbe>,
    tasks: Mutex<HashMap<DownloadTaskId, EngineTaskState>>,
    resolution: Mutex<resolution::ResolutionState>,
    scheduler_gate: Mutex<HashMap<String, u64>>,
    draining: AtomicBool,
    work_revision: AtomicU64,
    workers: Mutex<tokio::task::JoinSet<()>>,
    worker_failure: AtomicBool,
    schedule_wake_generation: AtomicU64,
    daily_schedule_wake_generation: AtomicU64,
    event_sender: async_channel::Sender<DownloadEventEnvelope>,
    primary_state_event_lost: AtomicBool,
    event_receiver: Mutex<Option<async_channel::Receiver<DownloadEventEnvelope>>>,
    event_subscribers: Mutex<Vec<async_channel::Sender<DownloadEventEnvelope>>>,
    event_sequencer: DownloadEventSequencer,
    session_generation: u64,
    browser_dialog_sender: async_channel::Sender<BrowserDownloadDialogRequest>,
    browser_dialog_receiver: Mutex<Option<async_channel::Receiver<BrowserDownloadDialogRequest>>>,
    browser_dialog_handoffs: Mutex<VecDeque<uuid::Uuid>>,
    browser_dialog_available: AtomicBool,
    browser_blob_staging_root: Option<PathBuf>,
}

/// Desired manual state is durable; the foreground policy is session-only.
/// Both setters hold the same gate through persistence and limiter publication.
struct GlobalSpeedLimits {
    manual: Option<u64>,
    runtime_cap: Option<u64>,
}

impl GlobalSpeedLimits {
    fn effective(&self) -> Option<u64> {
        match (self.manual, self.runtime_cap) {
            (Some(manual), Some(runtime)) => Some(manual.min(runtime)),
            (manual, runtime) => manual.or(runtime),
        }
    }
}

#[derive(Debug)]
pub struct BrowserDownloadDialogRequest {
    pub correlation_id: uuid::Uuid,
    pub handoff_id: uuid::Uuid,
    pub source: BrowserDownloadDialogSource,
    pub completion: std::sync::mpsc::SyncSender<Result<(), String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BrowserDownloadDialogSource {
    Download(Box<DownloadRequest>),
    Blob {
        blob: BrowserBlob,
        save_directory: PathBuf,
    },
    ControlledDownload(Box<DownloadRequest>),
}

#[derive(Clone)]
struct EngineTaskState {
    task: DownloadTask,
    transfer: StoredTransferState,
    control: Option<DownloadTransferControl>,
    last_progress_event: Option<Instant>,
    transfer_rate: Option<TransferRateSampler>,
    paused_url_replacement: Option<paused_url::PausedUrlReplacement>,
    accepted_revision: u64,
    completion_batch_id: Option<uuid::Uuid>,
}

impl EngineTaskState {
    fn capture_transfer_statistics(&mut self) {
        if let Some(control) = &self.control {
            self.task
                .set_transfer_statistics(control.statistics.snapshot());
        }
    }
}

#[derive(Clone)]
struct TransferRateSampler {
    sampled_bytes: u64,
    sampled_at: Instant,
    last_rate: u64,
}

/// Progress events are coalesced to at most 10 Hz per task (the plan's
/// 5-10 Hz band); terminal states always publish immediately.
const PROGRESS_EVENT_MIN_INTERVAL: Duration = Duration::from_millis(100);
/// How long a built HTTP client (with its warm connection pool) is reused for
/// probes and transfers before the system proxy is re-read.
const HTTP_CLIENT_REUSE_TTL: Duration = Duration::from_secs(30);

mod browser;
mod connection_ceiling;
mod controlled_handoff;
mod dash_runner;
mod hls_runner;
mod paused_url;
mod projection;
mod queue;
mod resolution;
pub(crate) mod segment_planner;
mod segment_workers;
mod segmented_runner;

#[cfg(test)]
mod connection_experiments;

#[cfg(test)]
mod fault_recovery;

use browser::BrowserBlobImportRequest;
use projection::snapshot_from_state;
pub use queue::daily_schedule_window_contains;

pub(crate) use dash_runner::*;
pub(crate) use hls_runner::*;
pub(crate) use segmented_runner::*;

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
    #[serde(default)]
    pub has_final_artifact: bool,
    #[serde(default)]
    pub scheduled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadCompletionSnapshot {
    pub generation: u64,
    pub work_revision: u64,
    pub queue_invalidations: HashMap<String, u64>,
    pub tasks: Vec<DownloadCompletionTask>,
    pub queues: Vec<crate::DownloadQueue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadCompletionTask {
    pub task_id: DownloadTaskId,
    pub status: DownloadStatus,
    pub attempt: u64,
    pub accepted_revision: u64,
    pub queue_id: String,
    pub intention: crate::DownloadCompletionIntention,
    pub directory: Option<PathBuf>,
    pub completion_batch_id: Option<uuid::Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadScheduleMoment {
    pub weekday: DownloadWeekday,
    pub minute_of_day: u16,
}

impl DownloadEngine {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DownloadEngineError> {
        Self::open_with_network(path, NetworkSettings::default())
    }

    pub fn open_with_network(
        path: impl AsRef<Path>,
        network_settings: NetworkSettings,
    ) -> Result<Self, DownloadEngineError> {
        Self::open_with_network_policy(path, NetworkPolicy::new(network_settings)?)
    }

    pub fn open_with_network_policy(
        path: impl AsRef<Path>,
        network_policy: NetworkPolicy,
    ) -> Result<Self, DownloadEngineError> {
        Self::open_with_policies(
            path,
            network_policy,
            DownloadHttpPolicy::from_download_settings(&DownloadSettings::default())?,
        )
    }

    pub fn open_with_policies(
        path: impl AsRef<Path>,
        network_policy: NetworkPolicy,
        download_http_policy: DownloadHttpPolicy,
    ) -> Result<Self, DownloadEngineError> {
        let path = path.as_ref();
        let staging_root = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("browser-blob-staging");
        let store = SqliteDownloadStore::open(path)?;
        Self::from_store_with_http_and_capacity_and_staging_root(
            store,
            HttpDownloadClient::new()?,
            Arc::new(SystemStorageCapacityProbe),
            Some(staging_root),
            None,
            network_policy,
            download_http_policy,
        )
    }

    pub fn open_in_memory() -> Result<Self, DownloadEngineError> {
        Self::from_store(SqliteDownloadStore::open_in_memory()?)
    }

    #[doc(hidden)]
    pub fn open_in_memory_with_policies(
        network_policy: NetworkPolicy,
        download_http_policy: DownloadHttpPolicy,
    ) -> Result<Self, DownloadEngineError> {
        Self::from_store_with_http_and_capacity_and_staging_root(
            SqliteDownloadStore::open_in_memory()?,
            HttpDownloadClient::new()?,
            Arc::new(SystemStorageCapacityProbe),
            None,
            None,
            network_policy,
            download_http_policy,
        )
    }

    pub fn update_download_http_policy(
        &self,
        settings: &DownloadSettings,
    ) -> Result<(), DownloadEngineError> {
        self.inner
            .download_http_policy
            .update_from_download_settings(settings)?;
        Ok(())
    }

    pub fn update_download_organization_policy(
        &self,
        settings: &DownloadSettings,
    ) -> Result<(), DownloadEngineError> {
        self.inner
            .download_organization_policy
            .update_from_download_settings(settings)?;
        Ok(())
    }

    #[doc(hidden)]
    pub fn open_in_memory_with_network_policy(
        network_policy: NetworkPolicy,
    ) -> Result<Self, DownloadEngineError> {
        Self::from_store_with_http_and_capacity_and_staging_root(
            SqliteDownloadStore::open_in_memory()?,
            HttpDownloadClient::new()?,
            Arc::new(SystemStorageCapacityProbe),
            None,
            None,
            network_policy,
            DownloadHttpPolicy::from_download_settings(&DownloadSettings::default())?,
        )
    }

    #[doc(hidden)]
    pub fn open_in_memory_with_ffmpeg(
        executable: impl AsRef<Path>,
    ) -> Result<Self, DownloadEngineError> {
        Self::from_store_with_http_and_capacity_and_staging_root(
            SqliteDownloadStore::open_in_memory()?,
            HttpDownloadClient::new()?,
            Arc::new(SystemStorageCapacityProbe),
            None,
            Some(executable.as_ref().to_owned()),
            NetworkPolicy::new(NetworkSettings::default())?,
            DownloadHttpPolicy::from_download_settings(&DownloadSettings::default())?,
        )
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
        Self::from_store_with_http_and_capacity_and_staging_root(
            store,
            http,
            capacity_probe,
            None,
            None,
            NetworkPolicy::new(NetworkSettings::default())?,
            DownloadHttpPolicy::from_download_settings(&DownloadSettings::default())?,
        )
    }

    fn from_store_with_http_and_capacity_and_staging_root(
        store: SqliteDownloadStore,
        http: HttpDownloadClient,
        capacity_probe: Arc<dyn StorageCapacityProbe>,
        browser_blob_staging_root: Option<PathBuf>,
        ffmpeg_executable: Option<PathBuf>,
        network_policy: NetworkPolicy,
        download_http_policy: DownloadHttpPolicy,
    ) -> Result<Self, DownloadEngineError> {
        let persisted_tasks = store.load_tasks()?;
        let pending_redownload_operations = store.load_redownload_operations()?;
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
                    paused_url_replacement: None,
                    completion_batch_id: None,
                    accepted_revision: 0,
                },
            );
        }
        let (event_sender, event_receiver) = async_channel::bounded(256);
        let (browser_dialog_sender, browser_dialog_receiver) = async_channel::bounded(16);
        let engine = Self {
            inner: Arc::new(DownloadEngineInner {
                http_template: http,
                connection_ceilings: Mutex::new(connection_ceiling::ConnectionCeilings::default()),
                http_clients: Mutex::new(HashMap::new()),
                network_policy,
                download_http_policy,
                download_organization_policy: DownloadOrganizationPolicy::from_download_settings(
                    &DownloadSettings::default(),
                )?,
                ffmpeg_executable: Mutex::new(ffmpeg_executable),
                ffmpeg_install_active: AtomicBool::new(false),
                store,
                global_rate_limiter: DownloadRateLimiter::new(global_speed_limit)?,
                global_speed_limits: Mutex::new(GlobalSpeedLimits {
                    manual: global_speed_limit,
                    runtime_cap: None,
                }),
                capacity_probe,
                tasks: Mutex::new(tasks),
                resolution: Mutex::new(resolution::ResolutionState::default()),
                scheduler_gate: Mutex::new(HashMap::new()),
                draining: AtomicBool::new(false),
                work_revision: AtomicU64::new(0),
                workers: Mutex::new(tokio::task::JoinSet::new()),
                worker_failure: AtomicBool::new(false),
                schedule_wake_generation: AtomicU64::new(0),
                daily_schedule_wake_generation: AtomicU64::new(0),
                event_sender,
                primary_state_event_lost: AtomicBool::new(false),
                event_receiver: Mutex::new(Some(event_receiver)),
                event_subscribers: Mutex::new(Vec::new()),
                event_sequencer: DownloadEventSequencer::new(0),
                session_generation: new_engine_session_generation(),
                browser_dialog_sender,
                browser_dialog_receiver: Mutex::new(Some(browser_dialog_receiver)),
                browser_dialog_handoffs: Mutex::new(VecDeque::new()),
                browser_dialog_available: AtomicBool::new(false),
                browser_blob_staging_root,
            }),
        };
        engine.reconcile_pending_redownload_operations(
            &pending_redownload_operations,
            uuid::Uuid::new_v4(),
        )?;
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
        if let Some(root) = &engine.inner.browser_blob_staging_root {
            browser::cleanup_pending_browser_blobs(root, correlation_id);
        }
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
    pub fn session_generation(&self) -> u64 {
        self.inner.session_generation
    }

    /// One serialized view of accepted queue membership and task state.
    pub fn completion_snapshot(&self) -> Result<DownloadCompletionSnapshot, DownloadEngineError> {
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        let tasks = self.inner.lock_tasks()?;
        let queues = self.inner.store.load_queues()?;
        let mut members = Vec::new();
        for queue in &queues {
            for task_id in self.inner.store.load_queue_order(&queue.id)? {
                if let Some(state) = tasks.get(&task_id) {
                    // Retry admission precedes dispatch when its queue is stopped.
                    let status = if state.task.status() == DownloadStatus::Failed
                        && self.inner.store.is_queue_task_eligible(task_id)?
                    {
                        DownloadStatus::Waiting
                    } else {
                        state.task.status()
                    };
                    members.push(DownloadCompletionTask {
                        task_id,
                        status,
                        attempt: state.task.completion_attempt(),
                        completion_batch_id: state.completion_batch_id,
                        accepted_revision: state.accepted_revision,
                        queue_id: queue.id.clone(),
                        intention: state.task.completion_intention(),
                        directory: if state.task.status() == DownloadStatus::Completed
                            && state.transfer.final_committed
                        {
                            state
                                .transfer
                                .final_path
                                .as_ref()
                                .and_then(|path| path.parent().map(Path::to_path_buf))
                        } else {
                            None
                        },
                    });
                }
            }
        }
        Ok(DownloadCompletionSnapshot {
            generation: self.inner.session_generation,
            work_revision: self.inner.work_revision.load(Ordering::SeqCst),
            queue_invalidations: _scheduler.clone(),
            tasks: members,
            queues,
        })
    }

    /// Close new admissions, request resumable pauses and await all owned transfer workers.
    pub fn shutdown_and_wait(&self) -> Result<(), DownloadEngineError> {
        self.shutdown_and_wait_correlated(uuid::Uuid::new_v4())
    }

    pub fn shutdown_and_wait_correlated(
        &self,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let started = Instant::now();
        {
            let _scheduler = self
                .inner
                .scheduler_gate
                .lock()
                .map_err(|_| DownloadEngineError::LockPoisoned)?;
            self.inner.draining.store(true, Ordering::SeqCst);
            self.inner
                .schedule_wake_generation
                .fetch_add(1, Ordering::SeqCst);
            self.inner
                .daily_schedule_wake_generation
                .fetch_add(1, Ordering::SeqCst);
            let tasks = self.inner.lock_tasks()?;
            for state in tasks.values() {
                state.cancel_paused_url_candidate();
                if let Some(control) = &state.control {
                    control.pause();
                }
            }
        }
        info!(event = "download.engine.drain_started", %correlation_id,
            generation = self.inner.session_generation, "download worker drain started");
        let mut handles = std::mem::take(
            &mut *self
                .inner
                .workers
                .lock()
                .map_err(|_| DownloadEngineError::LockPoisoned)?,
        );
        let count = handles.len();
        let (sender, receiver) = std::sync::mpsc::sync_channel(1);
        download_runtime()?.spawn(async move {
            let mut failed = false;
            while let Some(result) = handles.join_next().await {
                failed |= result.is_err();
            }
            let _ = sender.send(failed);
        });
        let failed = receiver
            .recv()
            .map_err(|error| DownloadEngineError::Runtime(error.to_string()))?;
        if failed || self.inner.worker_failure.load(Ordering::SeqCst) {
            self.inner.worker_failure.store(true, Ordering::SeqCst);
            warn!(event = "download.engine.drain_failed", %correlation_id,
                generation = self.inner.session_generation, workers = count,
                elapsed_ms = started.elapsed().as_millis(), "owned download worker failed; all handles retired");
            return Err(DownloadEngineError::Runtime(
                "owned download worker failed".into(),
            ));
        }
        info!(event = "download.engine.drain_completed", %correlation_id,
            generation = self.inner.session_generation, workers = count,
            elapsed_ms = started.elapsed().as_millis(), "download workers drained");
        Ok(())
    }

    pub fn event_sequence(&self) -> u64 {
        self.inner.event_sequencer.current()
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

    /// Returns the persisted desired manual limit, not the temporary policy ceiling.
    /// `None` means unlimited. Clones of this engine share the same desired state.
    pub fn global_speed_limit(&self) -> Option<u64> {
        self.inner
            .global_speed_limits
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .manual
    }

    /// Returns the actual shared transfer limiter ceiling: the minimum enabled
    /// manual global limit and runtime cap. Per-task limiters remain independent.
    pub fn effective_global_speed_limit(&self) -> Option<u64> {
        self.inner.global_rate_limiter.bytes_per_second()
    }

    /// Applies a session-only global ceiling without changing SQLite or task caps.
    /// The caller owns foreground policy and ordering: pass `Some(positive_bytes)`
    /// while another application is active, and `None` on disable/return to NxTools.
    /// `Some(0)` is rejected; `None` removes only the policy, restoring the latest
    /// desired manual limit. Engine reopen starts with no runtime cap.
    ///
    /// This updates the existing shared limiter in place, waking active waiters
    /// when the effective ceiling changes. Identical policy calls are silent;
    /// changed calls emit correlated `runtime_started`/`runtime_changed` diagnostics
    /// with numeric limits and elapsed time only. No download task event is emitted.
    pub fn set_runtime_speed_cap(
        &self,
        bytes_per_second: Option<u64>,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let started_at = Instant::now();
        if bytes_per_second == Some(0) {
            warn!(
                event = "download.speed_limit.runtime_rejected",
                %correlation_id,
                reason = "zero_limit",
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                "temporary download speed cap rejected"
            );
            return Err(DownloadRateLimitError::ZeroLimit.into());
        }
        let mut limits = self
            .inner
            .global_speed_limits
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if limits.runtime_cap == bytes_per_second {
            return Ok(());
        }
        info!(
            event = "download.speed_limit.runtime_started",
            %correlation_id,
            enabled = bytes_per_second.is_some(),
            "temporary download speed cap update started"
        );
        let updated = GlobalSpeedLimits {
            manual: limits.manual,
            runtime_cap: bytes_per_second,
        };
        let effective = updated.effective();
        self.inner
            .global_rate_limiter
            .set_bytes_per_second(effective)?;
        *limits = updated;
        info!(
            event = "download.speed_limit.runtime_changed",
            %correlation_id,
            enabled = bytes_per_second.is_some(),
            bytes_per_second = bytes_per_second.unwrap_or_default(),
            effective_enabled = effective.is_some(),
            effective_bytes_per_second = effective.unwrap_or_default(),
            elapsed_ms = started_at.elapsed().as_millis() as u64,
            "temporary download speed cap applied"
        );
        Ok(())
    }

    pub fn set_ffmpeg_executable_path(
        &self,
        executable: Option<PathBuf>,
    ) -> Result<(), DownloadEngineError> {
        *self
            .inner
            .ffmpeg_executable
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)? = executable;
        Ok(())
    }

    pub fn ffmpeg_resolution(
        &self,
    ) -> Result<Option<crate::FfmpegResolution>, DownloadEngineError> {
        let configured = self
            .inner
            .ffmpeg_executable
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .clone();
        Ok(crate::media::resolve_ffmpeg_executable(
            configured.as_deref(),
        ))
    }

    pub fn start_ffmpeg_install(
        &self,
        correlation_id: uuid::Uuid,
        install_root: PathBuf,
        preserve_executable: Option<PathBuf>,
    ) -> Result<FfmpegInstallOperation, DownloadEngineError> {
        let network = self.inner.network_policy.snapshot()?;
        let network_proxy_mode = network.mode();
        let network_proxy_source = network.source();
        info!(
            event = "download.ffmpeg_install.started",
            %correlation_id,
            proxy_mode = proxy_mode_name(network_proxy_mode),
            proxy_source = network_proxy_source,
            source_count = crate::FfmpegInstallSource::ALL.len(),
            "managed FFmpeg installation started"
        );
        self.inner
            .ffmpeg_install_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| DownloadEngineError::FfmpegInstallAlreadyActive)?;
        let control = DownloadTransferControl::new();
        let public_control = FfmpegInstallControl::new(control.clone());
        let (progress_sender, progress) = async_channel::bounded(32);
        let (completion_sender, completion) = async_channel::bounded(1);
        let inner = self.inner.clone();
        let cancel_control = control.clone();
        if let Err(error) = self.inner.spawn_owned_worker(async move {
            let result = {
                let future = crate::ffmpeg_install::run_ffmpeg_install(
                    correlation_id,
                    install_root,
                    preserve_executable,
                    control,
                    progress_sender,
                    network,
                );
                tokio::pin!(future);
                loop {
                    tokio::select! {
                        result = &mut future => break result,
                        () = tokio::time::sleep(Duration::from_millis(10)) => {
                            if inner.draining.load(Ordering::SeqCst) { cancel_control.cancel(); }
                        }
                    }
                }
            };
            inner.ffmpeg_install_active.store(false, Ordering::Release);
            let _ = completion_sender.send(result).await;
        }) {
            self.inner
                .ffmpeg_install_active
                .store(false, Ordering::Release);
            return Err(error);
        }
        Ok(FfmpegInstallOperation {
            correlation_id,
            network_proxy_mode,
            network_proxy_source,
            control: public_control,
            progress,
            completion,
        })
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
        let mut limits = self
            .inner
            .global_speed_limits
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if limits.manual == bytes_per_second {
            return Ok(());
        }
        // Commit desired intent before publishing it; policy never enters storage.
        self.inner.store.save_global_speed_limit(bytes_per_second)?;
        let updated = GlobalSpeedLimits {
            manual: bytes_per_second,
            runtime_cap: limits.runtime_cap,
        };
        let effective = updated.effective();
        self.inner
            .global_rate_limiter
            .set_bytes_per_second(effective)?;
        *limits = updated;
        info!(
            event = "download.speed_limit.global_changed",
            %correlation_id,
            enabled = bytes_per_second.is_some(),
            bytes_per_second = bytes_per_second.unwrap_or_default(),
            effective_enabled = effective.is_some(),
            effective_bytes_per_second = effective.unwrap_or_default(),
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
            self.inner.persist(&mut committed)?;
            if let Some(control) = committed.control.as_ref() {
                control.set_task_speed_limit(bytes_per_second)?;
            }
            *state = committed;
            state.task.clone()
        };
        self.inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(changed_task),
        });
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

    fn set_task_completion_intention(
        &self,
        task_id: DownloadTaskId,
        intention: crate::DownloadCompletionIntention,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let started_at = Instant::now();
        info!(event = "download.completion_intention.started", %correlation_id,
            task_id = %task_id.as_uuid(), generation = self.inner.session_generation,
            action = ?intention.action, open_directory = intention.open_directory,
            "task completion intention update started");
        let saved = (|| {
            let mut tasks = self.inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            if matches!(
                state.task.status(),
                DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
            ) {
                return Err(DownloadEngineError::InvalidAction {
                    task_id,
                    action: "set_completion_intention",
                    status: state.task.status(),
                });
            }
            let mut task = state.task.clone();
            task.set_completion_intention(intention);
            self.inner.store.save_task(&task)?;
            state.task = task.clone();
            Ok::<_, DownloadEngineError>(task)
        })();
        match saved {
            Ok(task) => {
                self.inner.emit(DownloadEvent::TaskChanged {
                    task: Box::new(task),
                });
                info!(event = "download.completion_intention.accepted", %correlation_id,
                    task_id = %task_id.as_uuid(), generation = self.inner.session_generation,
                    action = ?intention.action, open_directory = intention.open_directory,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "task completion intention persisted");
                Ok(())
            }
            Err(error) => {
                warn!(event = "download.completion_intention.failed", %correlation_id,
                    task_id = %task_id.as_uuid(), generation = self.inner.session_generation,
                    fault_code = ?error.fault().code, native_error_code = error.native_os_error(),
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "task completion intention was not persisted");
                Err(error)
            }
        }
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
            DownloadCommand::Probe { request } => self.probe_blocking(request, request_id),
            DownloadCommand::ProbeResolved {
                request,
                resolution,
            } => self
                .probe_resolved_correlated(
                    request,
                    resolution,
                    crate::DownloadResolutionControl::default(),
                    request_id,
                )
                .map(DownloadCommandOutcome::Probe),
            DownloadCommand::SubmitResolved {
                request,
                mode,
                idempotency_key,
                resolution,
            } => self
                .submit_resolved_correlated(
                    request,
                    envelope.origin,
                    mode,
                    idempotency_key,
                    request_id,
                    resolution,
                    crate::DownloadResolutionControl::default(),
                )
                .map(|(task_id, duplicate)| {
                    if idempotency_key.is_some() {
                        DownloadCommandOutcome::IdempotentAccepted { task_id, duplicate }
                    } else {
                        DownloadCommandOutcome::Accepted {
                            task_id: Some(task_id),
                        }
                    }
                }),
            DownloadCommand::BrowserSubmitResolved {
                handoff_id,
                request,
                mode,
                resolution,
            } => {
                self.submit_browser_with_resolution(
                    handoff_id,
                    request,
                    mode,
                    request_id,
                    resolution,
                    crate::DownloadResolutionControl::default(),
                )
                .map(|(task_id, duplicate)| {
                    DownloadCommandOutcome::BrowserAccepted { task_id, duplicate }
                })
            }
            DownloadCommand::BrowserPrepareHandoff {
                handoff_id,
                request,
                open_dialog,
                mode,
            } => self.prepare_browser_handoff_correlated(
                handoff_id,
                request,
                open_dialog,
                mode,
                handoff_id,
            ),
            DownloadCommand::BrowserAcceptHandoff {
                handoff_id,
                request,
                mode,
                resolution,
            } => self.accept_browser_handoff_correlated(
                handoff_id, request, mode, resolution, handoff_id,
            ),
            DownloadCommand::BrowserHandoffStatus { handoff_id } => {
                self.browser_handoff_status_correlated(handoff_id)
            }
            DownloadCommand::BrowserCommitHandoff {
                handoff_id,
                task_id,
            } => self.commit_browser_handoff_correlated(handoff_id, task_id, handoff_id),
            DownloadCommand::BrowserAbortHandoff { handoff_id } => {
                self.abort_browser_handoff_correlated(handoff_id)
            }
            DownloadCommand::Submit {
                request,
                mode,
                idempotency_key,
            } => match idempotency_key {
                None => self
                    .submit_with_mode_correlated(request, envelope.origin, mode, request_id)
                    .map(|task_id| DownloadCommandOutcome::Accepted {
                        task_id: Some(task_id),
                    }),
                Some(idempotency_key) => match self.submit_with_mode_idempotent_correlated(
                    request,
                    envelope.origin,
                    mode,
                    Some(idempotency_key),
                    request_id,
                ) {
                    Ok((task_id, duplicate)) => {
                        Ok(DownloadCommandOutcome::IdempotentAccepted { task_id, duplicate })
                    }
                    Err(error) => {
                        let fault = error.fault();
                        warn!(
                            event = "download.submission.failed",
                            %request_id,
                            %idempotency_key,
                            fault_code = ?fault.code,
                            retryable = fault.retryable,
                            "idempotent submission failed"
                        );
                        Err(error)
                    }
                },
            },
            DownloadCommand::BrowserSubmit {
                handoff_id,
                request,
                mode,
            } => self
                .submit_browser_correlated(handoff_id, request, mode, request_id)
                .map(
                    |(task_id, duplicate)| DownloadCommandOutcome::BrowserAccepted {
                        task_id,
                        duplicate,
                    },
                ),
            DownloadCommand::BrowserBatchSubmit { submissions } => self
                .submit_browser_batch_correlated(submissions, request_id)
                .map(|results| DownloadCommandOutcome::BrowserBatchAccepted { results }),
            DownloadCommand::BrowserOpenDialog {
                handoff_id,
                request,
            } => self
                .open_browser_dialog_correlated(handoff_id, request, request_id)
                .map(|duplicate| DownloadCommandOutcome::BrowserDialogAccepted {
                    handoff_id,
                    duplicate,
                }),
            DownloadCommand::BrowserOpenBlobDialog {
                handoff_id,
                blob,
                save_directory,
            } => self
                .open_browser_blob_dialog_correlated(handoff_id, blob, save_directory, request_id)
                .map(|duplicate| DownloadCommandOutcome::BrowserDialogAccepted {
                    handoff_id,
                    duplicate,
                }),
            DownloadCommand::BrowserImportBlob {
                handoff_id,
                blob,
                save_directory,
                file_name,
                category_id,
                save_directory_explicit,
            } => {
                self.import_browser_blob_correlated(
                    BrowserBlobImportRequest {
                        handoff_id,
                        blob,
                        save_directory,
                        file_name,
                        category_id,
                        save_directory_explicit,
                    },
                    request_id,
                )
                .map(|(task_id, duplicate)| {
                    DownloadCommandOutcome::BrowserAccepted { task_id, duplicate }
                })
            }
            DownloadCommand::BrowserDiscardBlob {
                handoff_id,
                blob_id,
            } => self
                .discard_browser_blob(handoff_id, blob_id, request_id)
                .map(|()| DownloadCommandOutcome::Accepted { task_id: None }),
            DownloadCommand::ReplacePausedUrl(replacement) => {
                let task_id = replacement.task_id;
                self.replace_paused_url_with_control(
                    replacement,
                    request_id,
                    crate::DownloadResolutionControl::default(),
                )
                .map(|()| DownloadCommandOutcome::Accepted {
                    task_id: Some(task_id),
                })
            }
            DownloadCommand::CancelPausedUrlReplacement {
                task_id,
                operation_id,
            } => self
                .cancel_paused_url_replacement(task_id, operation_id)
                .map(|()| DownloadCommandOutcome::Accepted {
                    task_id: Some(task_id),
                }),
            DownloadCommand::Task { task_id, action } => self
                .run_task_action(task_id, action, request_id)
                .map(|result_task_id| DownloadCommandOutcome::Accepted {
                    task_id: Some(result_task_id),
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
            DownloadCommand::Subscribe {
                after_sequence,
                generation,
            } => {
                let observed_sequence = self.inner.event_sequencer.current();
                let observed_generation = self.inner.session_generation;
                let generation_mismatch =
                    generation.is_some_and(|value| value != observed_generation);
                let sequence_gap = after_sequence.is_some_and(|value| value != observed_sequence);
                let snapshot_required = generation_mismatch || sequence_gap;
                if snapshot_required {
                    self.inner.emit(DownloadEvent::SnapshotRequired {
                        expected_sequence: if generation_mismatch {
                            1
                        } else {
                            after_sequence.unwrap_or_default().saturating_add(1)
                        },
                        observed_sequence,
                        expected_generation: Some(observed_generation),
                        observed_generation: generation,
                    });
                }
                if generation.is_some() {
                    Ok(DownloadCommandOutcome::Subscribed {
                        generation: observed_generation,
                        sequence: observed_sequence,
                        snapshot_required,
                    })
                } else {
                    Ok(DownloadCommandOutcome::Accepted { task_id: None })
                }
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
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadCommandOutcome, DownloadEngineError> {
        self.probe_with_resolution(
            request,
            crate::DownloadResolutionOptions {
                operation_id: correlation_id,
                mode: crate::DownloadResolutionMode::Resolve,
            },
            crate::DownloadResolutionControl::default(),
        )
        .map(DownloadCommandOutcome::Probe)
    }

    fn probe_effective_blocking(
        &self,
        request: DownloadRequest,
        correlation_id: uuid::Uuid,
        operation: Option<crate::DownloadResolutionOperation>,
    ) -> Result<DownloadProbe, DownloadEngineError> {
        let started = Instant::now();
        let certificate_policy = request.certificate_policy;
        let client_started = Instant::now();
        let client = self.inner.http_for_operation_traced(certificate_policy);
        let client_ms = client_started.elapsed().as_millis() as u64;
        let client_reused = client.as_ref().is_ok_and(|(_, reused)| *reused);
        let (proxy_mode, proxy_source) =
            client
                .as_ref()
                .map_or(("unavailable", "policy"), |(client, _)| {
                    (
                        proxy_mode_name(client.network_proxy_mode()),
                        client.network_proxy_source(),
                    )
                });
        info!(
            event = "download.probe.started",
            %correlation_id,
            proxy_mode,
            proxy_source,
            certificate_policy = certificate_policy_name(certificate_policy),
            "download resource probe started"
        );
        let mut stage_timings = None;
        let result =
            (|| {
                request.validate()?;
                let client = client?.0;
                let (sender, receiver) = std::sync::mpsc::sync_channel(1);
                let inner = Arc::clone(&self.inner);
                self.inner.spawn_owned_worker(async move {
                    let result = {
                        let future = client.probe_timed(&request);
                        tokio::pin!(future);
                        loop {
                            tokio::select! {
                                result = &mut future => break result.map_err(DownloadEngineError::from),
                                () = tokio::time::sleep(Duration::from_millis(10)) => {
                                    if inner.draining.load(Ordering::SeqCst) { break Err(DownloadEngineError::Draining); }
                                    if let Some(operation) = &operation
                                        && let Err(error) = operation.check() { break Err(error.into()); }
                                }
                            }
                        }
                    };
                    let _ = sender.send(result);
                })?;
                receiver
                    .recv()
                    .map_err(|error| DownloadEngineError::Runtime(error.to_string()))?
            })()
            .map(|(probe, timings)| {
                stage_timings = Some(timings);
                probe
            });
        match result {
            Ok(probe) => {
                let (media_kind, variant_count) = match probe.media.as_ref() {
                    Some(crate::DownloadMediaProbe::Hls(hls)) => ("hls", hls.variants.len()),
                    Some(crate::DownloadMediaProbe::Dash(dash)) => (
                        "dash",
                        dash.video_tracks
                            .len()
                            .saturating_add(dash.audio_tracks.len()),
                    ),
                    None => ("http", 0),
                };
                let timings = stage_timings.unwrap_or_default();
                info!(
                    event = "download.probe.completed",
                    %correlation_id,
                    proxy_mode,
                    proxy_source,
                    certificate_policy = certificate_policy_name(certificate_policy),
                    route_kind = probe.server_observation.as_ref().map(|value| value.route_kind.diagnostic_name()),
                    peer_present = probe.server_observation.as_ref().is_some_and(|value| value.peer_ip.is_some()),
                    response_ms = probe.server_observation.as_ref().map(|value| value.response_ms),
                    media_kind,
                    variant_count,
                    client_reused,
                    client_ms,
                    head_ms = timings.head_ms,
                    range_ms = timings.range_ms,
                    fallback_ms = timings.fallback_ms,
                    manifest_ms = timings.manifest_ms,
                    head_companion_skipped = timings.head_companion_skipped,
                    range_companion_skipped = timings.range_companion_skipped,
                    elapsed_ms = started.elapsed().as_millis(),
                    "download resource probe completed"
                );
                Ok(probe)
            }
            Err(error) => {
                warn!(
                    event = "download.probe.failed",
                    %correlation_id,
                    proxy_mode,
                    proxy_source,
                    certificate_policy = certificate_policy_name(certificate_policy),
                    error_kind = probe_error_kind(&error),
                    native_error_code = error
                        .io_error()
                        .and_then(std::io::Error::raw_os_error),
                    elapsed_ms = started.elapsed().as_millis(),
                    "download resource probe failed"
                );
                Err(error)
            }
        }
    }

    fn run_task_action(
        &self,
        task_id: DownloadTaskId,
        action: DownloadTaskAction,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        self.ensure_task_not_held(task_id)?;
        match action {
            DownloadTaskAction::Pause => {
                self.pause(task_id)?;
                Ok(task_id)
            }
            DownloadTaskAction::Resume => {
                self.resume_correlated(task_id, correlation_id)?;
                Ok(task_id)
            }
            DownloadTaskAction::Retry => self.retry_correlated(task_id, correlation_id),
            DownloadTaskAction::RetryWithAutoRename => {
                self.retry_with_auto_rename(task_id, correlation_id)
            }
            DownloadTaskAction::Redownload => self.redownload_correlated(task_id, correlation_id),
            DownloadTaskAction::Cancel => {
                self.cancel_correlated(task_id, correlation_id)?;
                Ok(task_id)
            }
            DownloadTaskAction::SetSpeedLimit { bytes_per_second } => {
                self.set_task_speed_limit_correlated(task_id, bytes_per_second, correlation_id)?;
                Ok(task_id)
            }
            DownloadTaskAction::SetCompletionIntention { intention } => {
                self.set_task_completion_intention(task_id, intention, correlation_id)?;
                Ok(task_id)
            }
            DownloadTaskAction::Remove { delete_artifact } => {
                self.remove_correlated(task_id, delete_artifact, correlation_id)?;
                Ok(task_id)
            }
        }
    }

    fn run_batch_action(
        &self,
        task_ids: Vec<DownloadTaskId>,
        action: DownloadBatchAction,
        correlation_id: uuid::Uuid,
    ) -> DownloadCommandOutcome {
        let started_at = Instant::now();
        info!(event = "download.batch.started", %correlation_id, ?action,
            target_count = task_ids.len(), "download batch command started");
        let mut seen = HashSet::new();
        let mut results = Vec::new();
        for task_id in task_ids.into_iter().filter(|task_id| seen.insert(*task_id)) {
            let task_action = match action {
                DownloadBatchAction::Pause => DownloadTaskAction::Pause,
                DownloadBatchAction::Resume => DownloadTaskAction::Resume,
                DownloadBatchAction::Retry => DownloadTaskAction::Retry,
                DownloadBatchAction::Redownload => DownloadTaskAction::Redownload,
                DownloadBatchAction::Cancel => DownloadTaskAction::Cancel,
                DownloadBatchAction::Remove => DownloadTaskAction::Remove {
                    delete_artifact: false,
                },
                DownloadBatchAction::RemoveWithArtifact => DownloadTaskAction::Remove {
                    delete_artifact: true,
                },
            };
            let (result_task_id, fault) =
                match self.run_task_action(task_id, task_action, correlation_id) {
                    Ok(result_task_id) => (result_task_id, None),
                    Err(error) => (task_id, Some(error.fault())),
                };
            results.push(DownloadBatchItemResult {
                task_id: result_task_id,
                fault,
            });
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
        self.submit_with_mode_idempotent_correlated(
            request,
            origin,
            mode,
            None,
            uuid::Uuid::new_v4(),
        )
        .map(|(task_id, _)| task_id)
    }

    pub fn submit_idempotent(
        &self,
        idempotency_key: uuid::Uuid,
        request: DownloadRequest,
        origin: RequestOrigin,
        mode: DownloadSubmitMode,
    ) -> Result<(DownloadTaskId, bool), DownloadEngineError> {
        self.submit_with_mode_idempotent_correlated(
            request,
            origin,
            mode,
            Some(idempotency_key),
            uuid::Uuid::new_v4(),
        )
    }

    fn submit_with_mode_correlated(
        &self,
        request: DownloadRequest,
        origin: RequestOrigin,
        mode: DownloadSubmitMode,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        self.submit_with_mode_idempotent_correlated(request, origin, mode, None, correlation_id)
            .map(|(task_id, _)| task_id)
    }

    fn resolve_existing_idempotent_submission(
        &self,
        origin: RequestOrigin,
        idempotency_key: uuid::Uuid,
        request_fingerprint: &str,
        correlation_id: uuid::Uuid,
    ) -> Result<Option<DownloadTaskId>, DownloadEngineError> {
        let Some(existing) = self
            .inner
            .store
            .submission_receipt(origin, idempotency_key)?
        else {
            return Ok(None);
        };
        if existing.request_fingerprint != request_fingerprint {
            warn!(
                event = "download.submission.conflict",
                %correlation_id,
                %idempotency_key,
                ?existing.task_id,
                "idempotent submission conflicts with the accepted request"
            );
            return Err(DownloadEngineError::IdempotencyConflict(idempotency_key));
        }
        Ok(Some(existing.task_id))
    }

    fn submit_with_mode_idempotent_correlated(
        &self,
        request: DownloadRequest,
        origin: RequestOrigin,
        mode: DownloadSubmitMode,
        idempotency_key: Option<uuid::Uuid>,
        correlation_id: uuid::Uuid,
    ) -> Result<(DownloadTaskId, bool), DownloadEngineError> {
        self.submit_resolved_correlated(
            request,
            origin,
            mode,
            idempotency_key,
            correlation_id,
            crate::DownloadResolutionOptions {
                operation_id: correlation_id,
                mode: crate::DownloadResolutionMode::Resolve,
            },
            crate::DownloadResolutionControl::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn submit_with_resolution(
        &self,
        request: DownloadRequest,
        origin: RequestOrigin,
        mode: DownloadSubmitMode,
        idempotency_key: Option<uuid::Uuid>,
        resolution: crate::DownloadResolutionOptions,
        control: crate::DownloadResolutionControl,
    ) -> Result<(DownloadTaskId, bool), DownloadEngineError> {
        let correlation_id = resolution.operation_id;
        self.submit_resolved_correlated(
            request,
            origin,
            mode,
            idempotency_key,
            correlation_id,
            resolution,
            control,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn submit_resolved_correlated(
        &self,
        mut request: DownloadRequest,
        origin: RequestOrigin,
        mode: DownloadSubmitMode,
        idempotency_key: Option<uuid::Uuid>,
        correlation_id: uuid::Uuid,
        resolution: crate::DownloadResolutionOptions,
        control: crate::DownloadResolutionControl,
    ) -> Result<(DownloadTaskId, bool), DownloadEngineError> {
        let request_fingerprint = idempotency_key
            .map(|_| crate::storage::request_fingerprint(&request))
            .transpose()?;
        if let Some(idempotency_key) = idempotency_key
            && let Some(task_id) = self.resolve_existing_idempotent_submission(
                origin,
                idempotency_key,
                request_fingerprint
                    .as_deref()
                    .expect("idempotent submission has a request fingerprint"),
                correlation_id,
            )?
        {
            info!(
                event = "download.submission.duplicate",
                %correlation_id,
                %idempotency_key,
                ?task_id,
                "idempotent submission replay resolved before request acceptance"
            );
            return Ok((task_id, true));
        }
        let prepared =
            self.prepare_request_resolution(request, resolution, control, correlation_id)?;
        request = prepared.request.clone();
        self.resolve_missing_file_name_with_operation(
            &mut request,
            correlation_id,
            Some(prepared.operation.clone()),
        )?;
        self.check_request_resolution(&prepared)?;
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if let Some(idempotency_key) = idempotency_key
            && let Some(task_id) = self.resolve_existing_idempotent_submission(
                origin,
                idempotency_key,
                request_fingerprint
                    .as_deref()
                    .expect("idempotent submission has a request fingerprint"),
                correlation_id,
            )?
        {
            info!(
                event = "download.submission.duplicate",
                %correlation_id,
                %idempotency_key,
                ?task_id,
                "idempotent submission replay resolved at acceptance boundary"
            );
            return Ok((task_id, true));
        }
        let mut tasks = self.inner.lock_tasks()?;
        self.inner.check_admission()?;
        self.inner.next_work_revision()?;
        apply_organization_to_request(&self.inner, &mut request, &tasks, correlation_id)?;
        request.validate()?;
        let resolution_acceptance = self.lock_resolution_acceptance(&prepared)?;
        let now = now_unix_ms();
        let delayed_until_due = request
            .scheduled_at_unix_ms
            .is_some_and(|scheduled_at| scheduled_at > now);
        let scheduled_at_unix_ms = request.scheduled_at_unix_ms;
        let mut task = DownloadTask::new(request, origin, now);
        task.set_resolution(prepared.provenance.clone());
        let task_id = task.id();
        let transfer = StoredTransferState::default();
        let is_eligible = mode == DownloadSubmitMode::StartNow && !delayed_until_due;
        if let Some(idempotency_key) = idempotency_key {
            let saved = self
                .inner
                .store
                .save_idempotent_task_with_transfer_and_queue(
                    origin,
                    idempotency_key,
                    request_fingerprint
                        .as_deref()
                        .expect("idempotent submission has a request fingerprint"),
                    &task,
                    &transfer,
                    QueueAdmission {
                        id: "default",
                        eligible: is_eligible,
                    },
                )
                .map_err(|error| match error {
                    DownloadStorageError::IdempotencyConflict { .. } => {
                        DownloadEngineError::IdempotencyConflict(idempotency_key)
                    }
                    error => error.into(),
                })?;
            if let IdempotentSubmissionSave::Duplicate(task_id) = saved {
                drop(tasks);
                info!(
                    event = "download.submission.duplicate",
                    %correlation_id,
                    %idempotency_key,
                    ?task_id,
                    "idempotent submission replay resolved by durable receipt"
                );
                return Ok((task_id, true));
            }
        } else {
            self.inner.store.save_task_with_transfer_and_queue(
                &task,
                &transfer,
                "default",
                is_eligible,
            )?;
        }
        let accepted_revision = self.inner.bump_work_revision();
        drop(resolution_acceptance);
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
        if let Some(idempotency_key) = idempotency_key {
            info!(
                event = "download.submission.accepted",
                %correlation_id,
                %idempotency_key,
                ?task_id,
                "idempotent submission accepted"
            );
        }
        Ok((task_id, false))
    }

    fn resolve_missing_file_name_before_acceptance(
        &self,
        request: &mut DownloadRequest,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        self.resolve_missing_file_name_with_operation(request, correlation_id, None)
    }

    fn resolve_missing_file_name_with_operation(
        &self,
        request: &mut DownloadRequest,
        correlation_id: uuid::Uuid,
        operation: Option<crate::DownloadResolutionOperation>,
    ) -> Result<(), DownloadEngineError> {
        if request.file_name.is_some() {
            return Ok(());
        }
        let started_at = Instant::now();
        let certificate_policy = request.certificate_policy;
        let client = self.inner.http_for_operation(certificate_policy);
        let (proxy_mode, proxy_source) =
            client.as_ref().map_or(("unavailable", "policy"), |client| {
                (
                    proxy_mode_name(client.network_proxy_mode()),
                    client.network_proxy_source(),
                )
            });
        info!(
            event = "download.probe.started",
            %correlation_id,
            proxy_mode,
            proxy_source,
            certificate_policy = certificate_policy_name(certificate_policy),
            purpose = "acceptance_file_name",
            "download acceptance file-name probe started"
        );
        let probe_request = request.clone();
        let result = (|| {
            let client = client?;
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            let inner = Arc::clone(&self.inner);
            self.inner.spawn_owned_worker(async move {
                    let result = {
                        let future = client.probe(&probe_request);
                        tokio::pin!(future);
                        loop {
                            tokio::select! {
                                result = &mut future => break result.map_err(DownloadEngineError::from),
                                () = tokio::time::sleep(Duration::from_millis(10)) => {
                                    if inner.draining.load(Ordering::SeqCst) { break Err(DownloadEngineError::Draining); }
                                    if let Some(operation) = &operation
                                        && let Err(error) = operation.check() { break Err(error.into()); }
                                }
                            }
                        }
                    };
                    let _ = sender.send(result);
                })?;
            receiver
                .recv()
                .map_err(|error| DownloadEngineError::Runtime(error.to_string()))?
        })();
        let file_name = match result.and_then(|probe| {
            probe
                .file_name
                .ok_or(DownloadRequestError::InvalidFileName.into())
        }) {
            Ok(file_name) => file_name,
            Err(error) => {
                warn!(
                    event = "download.probe.failed",
                    %correlation_id,
                    proxy_mode,
                    proxy_source,
                    certificate_policy = certificate_policy_name(certificate_policy),
                    error_kind = probe_error_kind(&error),
                    native_error_code = error.io_error().and_then(std::io::Error::raw_os_error),
                    elapsed_ms = started_at.elapsed().as_millis(),
                    purpose = "acceptance_file_name",
                    "download acceptance file-name probe failed"
                );
                return Err(error);
            }
        };
        request.file_name = Some(file_name);
        info!(
            event = "download.probe.completed",
            %correlation_id,
            proxy_mode,
            proxy_source,
            certificate_policy = certificate_policy_name(certificate_policy),
            elapsed_ms = started_at.elapsed().as_millis(),
            purpose = "acceptance_file_name",
            "download acceptance file-name probe completed"
        );
        Ok(())
    }

    pub fn pause(&self, task_id: DownloadTaskId) -> Result<(), DownloadEngineError> {
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        self.ensure_task_not_held(task_id)?;
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
            let task = Box::new(state.task.clone());
            drop(tasks);
            self.inner.emit(DownloadEvent::TaskChanged { task });
            drop(_scheduler);
            self.refresh_delayed_schedule_wake()?;
            return Ok(());
        }
        let media_finalizing = state.task.status() == DownloadStatus::Finalizing
            && matches!(
                state.task.request().media.as_ref(),
                Some(DownloadRequestMedia::Dash(_) | DownloadRequestMedia::Hls(_))
            );
        if !matches!(
            state.task.status(),
            DownloadStatus::Probing | DownloadStatus::Waiting | DownloadStatus::Downloading
        ) && !media_finalizing
        {
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

    fn retry_with_auto_rename(
        &self,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        let started = Instant::now();
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        let mut tasks = self.inner.lock_tasks()?;
        let state = tasks
            .get(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        if state.task.status() != DownloadStatus::Failed
            || state.transfer.final_committed
            || state.transfer.error.as_ref().map(|fault| fault.code)
                != Some(DownloadFaultCode::DestinationExists)
        {
            return Err(DownloadEngineError::InvalidAction {
                task_id,
                action: "retry_with_auto_rename",
                status: state.task.status(),
            });
        }
        let mut candidate = state.task.request().clone();
        let old_path = candidate.save_directory.join(
            candidate
                .file_name
                .as_deref()
                .ok_or(DownloadRequestError::InvalidFileName)?,
        );
        candidate.category_id = None;
        candidate.save_directory_explicit = true;
        candidate.duplicate_strategy = crate::DownloadDuplicateStrategy::AddNumbered;
        self.inner
            .download_organization_policy
            .apply_to_request(&mut candidate, |path| {
                windows_target_key(path) == windows_target_key(&old_path)
                    || tasks.iter().any(|(id, other)| {
                        *id != task_id
                            && !other.transfer.final_committed
                            && other
                                .task
                                .request()
                                .file_name
                                .as_deref()
                                .is_some_and(|name| {
                                    windows_target_key(
                                        &other.task.request().save_directory.join(name),
                                    ) == windows_target_key(path)
                                })
                    })
            })?;
        let new_name = candidate
            .file_name
            .ok_or(DownloadRequestError::InvalidFileName)?;
        self.inner.check_admission()?;
        self.inner.next_work_revision()?;
        let mut updated = state.clone();
        let next = updated
            .task
            .completion_attempt()
            .checked_add(1)
            .ok_or(DownloadEngineError::CompletionOverflow)?;
        updated.task.set_completion_attempt(next);
        let updated_at = now_unix_ms();
        updated.task.set_file_name(new_name.clone(), updated_at);
        updated.task.clear_scheduled_start(updated_at);
        updated.transfer.final_path = Some(updated.task.request().save_directory.join(new_name));
        self.inner.store.save_task_with_transfer_and_admission(
            &updated.task,
            &updated.transfer,
            true,
            false,
        )?;
        updated.accepted_revision = self.inner.bump_work_revision();
        tasks.insert(task_id, updated.clone());
        drop(tasks);
        drop(_scheduler);
        self.inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(updated.task),
        });
        if let Err(error) = self
            .refresh_delayed_schedule_wake()
            .and_then(|()| self.schedule_task_queue(task_id, correlation_id))
        {
            warn!(event = "download.queue.resume_deferred", %correlation_id, ?task_id,
                error = %error, "resume intent was committed and remains eligible for a later refill");
        }
        info!(event = "download.retry_auto_rename.accepted", %correlation_id, ?task_id,
            elapsed_ms = started.elapsed().as_millis(), "conflicted task destination renumbered");
        Ok(task_id)
    }

    fn retry_correlated(
        &self,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        let already_admitted = {
            let tasks = self.inner.lock_tasks()?;
            let state = tasks
                .get(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            match state.task.status() {
                DownloadStatus::Failed => {
                    let fault = state
                        .transfer
                        .error
                        .as_ref()
                        .ok_or(DownloadEngineError::RetryNotAllowed(task_id))?;
                    if !fault.retryable {
                        return Err(DownloadEngineError::RetryNotAllowed(task_id));
                    }
                    self.inner.store.is_queue_task_eligible(task_id)?
                }
                DownloadStatus::Waiting
                | DownloadStatus::Probing
                | DownloadStatus::Downloading
                | DownloadStatus::Finalizing
                | DownloadStatus::Verifying => true,
                status => {
                    return Err(DownloadEngineError::InvalidAction {
                        task_id,
                        action: "retry",
                        status,
                    });
                }
            }
        };
        if already_admitted {
            info!(
                event = "download.retry.duplicate",
                %correlation_id,
                ?task_id,
                "retry request coalesced with an existing admission"
            );
            return Ok(task_id);
        }
        self.resume_eligible_correlated(task_id, correlation_id)?;
        info!(
            event = "download.retry.accepted",
            %correlation_id,
            ?task_id,
            "retry request accepted with durable continuation state"
        );
        Ok(task_id)
    }

    fn create_redownload_replacement(
        &self,
        old: &DownloadEngineTask,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        if let Some((_, replacement_task_id)) = self
            .inner
            .store
            .load_redownload_operations()?
            .into_iter()
            .find(|(old_task_id, _)| *old_task_id == old.task.id())
        {
            return Ok(replacement_task_id);
        }
        let mut request = old.task.request().clone();
        if old.task.resolution().is_none() {
            self.inner
                .download_http_policy
                .apply_to_request(&mut request)?;
        }
        request.validate()?;
        self.resolve_missing_file_name_before_acceptance(&mut request, correlation_id)?;
        self.inner.check_admission()?;
        self.inner.next_work_revision()?;
        let mut tasks = self.inner.lock_tasks()?;
        apply_organization_to_request(&self.inner, &mut request, &tasks, correlation_id)?;
        request.validate()?;
        let mut task = DownloadTask::new(request, old.task.origin(), now_unix_ms());
        task.set_resolution(old.task.resolution().cloned());
        let task_id = task.id();
        let transfer = StoredTransferState::default();
        let saved = self
            .inner
            .store
            .save_redownload_task_with_transfer_and_queue(
                old.task.id(),
                &task,
                &transfer,
                "default",
                false,
            )?;
        match saved {
            RedownloadOperationSave::Existing(existing_task_id) => {
                drop(tasks);
                Ok(existing_task_id)
            }
            RedownloadOperationSave::Inserted(_) => {
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
                        accepted_revision: self.inner.next_work_revision()?,
                    },
                );
                self.inner.bump_work_revision();
                drop(tasks);
                self.inner.emit_task(task_id);
                Ok(task_id)
            }
        }
    }

    fn reconcile_pending_redownload_operations(
        &self,
        operations: &[(DownloadTaskId, DownloadTaskId)],
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        for &(old_task_id, replacement_task_id) in operations {
            let (old, replacement_exists) = {
                let tasks = self.inner.lock_tasks()?;
                (
                    tasks.get(&old_task_id).map(snapshot_from_state),
                    tasks.contains_key(&replacement_task_id),
                )
            };
            let Some(old) = old else {
                warn!(
                    event = "download.redownload.reconciliation_missing_old_task",
                    %correlation_id,
                    ?old_task_id,
                    ?replacement_task_id,
                    "pending redownload marker has no original task"
                );
                continue;
            };
            if !replacement_exists {
                warn!(
                    event = "download.redownload.reconciliation_missing_replacement",
                    %correlation_id,
                    ?old_task_id,
                    ?replacement_task_id,
                    "pending redownload marker has no replacement task"
                );
                continue;
            }
            if let Err(error) = self.cleanup_owned_artifacts(
                &old,
                correlation_id,
                "startup_reconcile",
                Instant::now(),
            ) {
                warn!(
                    event = "download.redownload.reconciliation_deferred",
                    %correlation_id,
                    ?old_task_id,
                    ?replacement_task_id,
                    error = %error,
                    "pending redownload remains durable after cleanup failure"
                );
                continue;
            }
            if !self
                .inner
                .store
                .complete_redownload_operation(old_task_id, replacement_task_id)?
            {
                continue;
            }
            self.inner.lock_tasks()?.remove(&old_task_id);
            self.inner.emit(DownloadEvent::TaskRemoved {
                task_id: old_task_id,
            });
            if let Err(error) = self.resume_correlated(replacement_task_id, correlation_id) {
                warn!(
                    event = "download.redownload.reconciliation_admission_deferred",
                    %correlation_id,
                    ?old_task_id,
                    ?replacement_task_id,
                    error = %error,
                    "reconciled replacement remains eligible for queue refill"
                );
            }
            info!(
                event = "download.redownload.reconciled",
                %correlation_id,
                ?old_task_id,
                ?replacement_task_id,
                "pending redownload replacement reconciled"
            );
        }
        Ok(())
    }

    fn redownload_correlated(
        &self,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        let old = self
            .task(task_id)?
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        if !matches!(
            old.task.status(),
            DownloadStatus::Completed | DownloadStatus::Failed | DownloadStatus::Cancelled
        ) {
            return Err(DownloadEngineError::InvalidAction {
                task_id,
                action: "redownload",
                status: old.task.status(),
            });
        }

        let new_task_id = self.create_redownload_replacement(&old, correlation_id)?;
        if let Err(cleanup_error) =
            self.cleanup_owned_artifacts(&old, correlation_id, "redownload", Instant::now())
        {
            warn!(
                event = "download.redownload.old_task_cleanup_failed",
                %correlation_id,
                ?task_id,
                ?new_task_id,
                error = %cleanup_error,
                "redownload remains durable for cleanup retry"
            );
            return Err(DownloadEngineError::RedownloadCleanupFailed {
                old_task_id: task_id,
                new_task_id,
            });
        }
        if !self
            .inner
            .store
            .complete_redownload_operation(task_id, new_task_id)?
        {
            return Err(DownloadEngineError::TaskNotFound(task_id));
        }
        self.inner.lock_tasks()?.remove(&task_id);
        self.inner.emit(DownloadEvent::TaskRemoved { task_id });

        drop(_scheduler);
        if let Err(error) = self.resume_correlated(new_task_id, correlation_id) {
            warn!(
                event = "download.redownload.admission_deferred",
                %correlation_id,
                ?task_id,
                ?new_task_id,
                error = %error,
                "redownload was accepted but replacement admission was deferred"
            );
        }
        info!(
            event = "download.redownload.accepted",
            %correlation_id,
            ?task_id,
            ?new_task_id,
            "redownload accepted as a new task"
        );
        Ok(new_task_id)
    }
    fn resume_correlated(
        &self,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let status = self
            .inner
            .lock_tasks()?
            .get(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?
            .task
            .status();
        if status == DownloadStatus::Failed {
            self.retry_correlated(task_id, correlation_id).map(|_| ())
        } else {
            self.resume_eligible_correlated(task_id, correlation_id)
        }
    }

    fn resume_eligible_correlated(
        &self,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        self.ensure_task_not_held(task_id)?;
        self.inner.check_admission()?;
        self.inner.next_work_revision()?;
        let newly_admitted;
        {
            let mut tasks = self.inner.lock_tasks()?;

            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            if state
                .paused_url_replacement
                .as_ref()
                .is_some_and(|operation| operation.active)
            {
                return Err(DownloadEngineError::InvalidAction {
                    task_id,
                    action: "resume_while_replacing_url",
                    status: state.task.status(),
                });
            }
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
            newly_admitted = !self.inner.store.is_queue_task_eligible(task_id)?;
            let mut committed = state.clone();
            if committed.task.status() == DownloadStatus::Failed && newly_admitted {
                let next = committed
                    .task
                    .completion_attempt()
                    .checked_add(1)
                    .ok_or(DownloadEngineError::CompletionOverflow)?;
                committed.task.set_completion_attempt(next);
            }
            if committed.task.request().scheduled_at_unix_ms.is_some() {
                committed.task.clear_scheduled_start(now_unix_ms());
                self.inner.store.save_task_with_transfer_and_admission(
                    &committed.task,
                    &committed.transfer,
                    true,
                    false,
                )?;
                *state = committed;
            } else if committed.task.completion_attempt() != state.task.completion_attempt() {
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
            if newly_admitted {
                let revision = self.inner.bump_work_revision();
                if state.task.status() == DownloadStatus::Failed {
                    state.accepted_revision = revision;
                }
            }
        }
        drop(_scheduler);
        if newly_admitted {
            self.inner.emit_task(task_id);
        }
        self.refresh_delayed_schedule_wake()?;
        if let Err(error) = self.schedule_task_queue(task_id, correlation_id) {
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

    fn schedule_task_queue(
        &self,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        for queue in self.inner.store.load_queues()? {
            if self
                .inner
                .store
                .load_queue_order(&queue.id)?
                .contains(&task_id)
            {
                return self.schedule_queue(&queue.id, correlation_id);
            }
        }
        Err(DownloadStorageError::TaskNotFound(task_id).into())
    }

    pub fn cancel(&self, task_id: DownloadTaskId) -> Result<(), DownloadEngineError> {
        self.cancel_correlated(task_id, uuid::Uuid::new_v4())
    }

    fn cancel_correlated(
        &self,
        task_id: DownloadTaskId,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let started_at = Instant::now();
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        self.ensure_task_not_held(task_id)?;
        self.inner.next_work_revision()?;
        self.inner.store.set_queue_task_eligible(task_id, false)?;
        let mut tasks = self.inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.cancel_paused_url_candidate();
        let changed = match state.task.status() {
            DownloadStatus::Probing | DownloadStatus::Waiting | DownloadStatus::Downloading => {
                state
                    .control
                    .as_ref()
                    .ok_or(DownloadEngineError::MissingTransferControl(task_id))?
                    .cancel();
                false
            }
            DownloadStatus::Finalizing
                if matches!(
                    state.task.request().media.as_ref(),
                    Some(DownloadRequestMedia::Dash(_) | DownloadRequestMedia::Hls(_))
                ) =>
            {
                state
                    .control
                    .as_ref()
                    .ok_or(DownloadEngineError::MissingTransferControl(task_id))?
                    .cancel();
                false
            }
            DownloadStatus::Created | DownloadStatus::Paused | DownloadStatus::Failed => {
                if matches!(
                    state.task.request().media.as_ref(),
                    Some(DownloadRequestMedia::Hls(_))
                ) {
                    info!(
                        event = "download.hls_cleanup.started",
                        %correlation_id,
                        ?task_id,
                        reason = "cancelled",
                        "HLS task-owned artifact cleanup started"
                    );
                    remove_hls_owned_artifacts(
                        state.task.request(),
                        task_id,
                        state.transfer.temporary_path.as_deref(),
                        correlation_id,
                        "cancelled",
                        started_at,
                    )?;
                    self.inner.store.clear_hls_transfer_plan(task_id)?;
                    info!(
                        event = "download.hls_cleanup.completed",
                        %correlation_id,
                        ?task_id,
                        reason = "cancelled",
                        elapsed_ms = started_at.elapsed().as_millis(),
                        "HLS task-owned artifact cleanup completed"
                    );
                } else if matches!(
                    state.task.request().media.as_ref(),
                    Some(DownloadRequestMedia::Dash(_))
                ) {
                    info!(
                        event = "download.dash_cleanup.started",
                        %correlation_id,
                        ?task_id,
                        reason = "cancelled",
                        "DASH task-owned artifact cleanup started"
                    );
                    remove_dash_owned_artifacts(
                        state.task.request(),
                        task_id,
                        state.transfer.temporary_path.as_deref(),
                        correlation_id,
                        "cancelled",
                        started_at,
                    )?;
                    self.inner.store.clear_dash_transfer_plan(task_id)?;
                    info!(
                        event = "download.dash_cleanup.completed",
                        %correlation_id,
                        ?task_id,
                        reason = "cancelled",
                        elapsed_ms = started_at.elapsed().as_millis(),
                        "DASH task-owned artifact cleanup completed"
                    );
                } else {
                    remove_task_temporary_artifact(state);
                }
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
        self.inner.bump_work_revision();
        drop(tasks);
        if changed {
            self.inner.emit_task(task_id);
        }
        Ok(())
    }

    fn cleanup_owned_artifacts(
        &self,
        snapshot: &DownloadEngineTask,
        correlation_id: uuid::Uuid,
        reason: &'static str,
        started_at: Instant,
    ) -> Result<(), DownloadEngineError> {
        let task_id = snapshot.task.id();
        if matches!(
            snapshot.task.request().media.as_ref(),
            Some(DownloadRequestMedia::Hls(_))
        ) {
            info!(
                event = "download.hls_cleanup.started",
                %correlation_id,
                ?task_id,
                reason,
                "HLS task-owned artifact cleanup started"
            );
            remove_hls_owned_artifacts(
                snapshot.task.request(),
                task_id,
                snapshot.temporary_path.as_deref(),
                correlation_id,
                reason,
                started_at,
            )?;
            info!(
                event = "download.hls_cleanup.completed",
                %correlation_id,
                ?task_id,
                reason,
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS task-owned artifact cleanup completed"
            );
        } else if matches!(
            snapshot.task.request().media.as_ref(),
            Some(DownloadRequestMedia::Dash(_))
        ) {
            info!(
                event = "download.dash_cleanup.started",
                %correlation_id,
                ?task_id,
                reason,
                "DASH task-owned artifact cleanup started"
            );
            remove_dash_owned_artifacts(
                snapshot.task.request(),
                task_id,
                snapshot.temporary_path.as_deref(),
                correlation_id,
                reason,
                started_at,
            )?;
            info!(
                event = "download.dash_cleanup.completed",
                %correlation_id,
                ?task_id,
                reason,
                elapsed_ms = started_at.elapsed().as_millis(),
                "DASH task-owned artifact cleanup completed"
            );
        } else if let Some(temporary_path) = snapshot.temporary_path.as_deref() {
            remove_path_if_exists(temporary_path)?;
        }
        Ok(())
    }

    pub fn remove(
        &self,
        task_id: DownloadTaskId,
        delete_artifact: bool,
    ) -> Result<(), DownloadEngineError> {
        self.remove_correlated(task_id, delete_artifact, uuid::Uuid::new_v4())
    }

    fn remove_correlated(
        &self,
        task_id: DownloadTaskId,
        delete_artifact: bool,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let mut _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        self.ensure_task_not_held(task_id)?;
        self.inner.next_work_revision()?;
        let queue_id = self.inner.store.queue_id_for_task(task_id)?;
        let started_at = Instant::now();
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
            state.cancel_paused_url_candidate();
            snapshot_from_state(state)
        };

        self.cleanup_owned_artifacts(&snapshot, correlation_id, "removed", started_at)?;
        if delete_artifact && let Some(final_path) = snapshot.final_path.as_deref() {
            remove_path_if_exists(final_path)?;
        }
        if !self.inner.store.remove_task(task_id)? {
            return Err(DownloadEngineError::TaskNotFound(task_id));
        }
        let revision = self.inner.bump_work_revision();
        _scheduler.insert(queue_id, revision);
        self.inner.lock_tasks()?.remove(&task_id);
        self.inner.emit(DownloadEvent::TaskRemoved { task_id });
        Ok(())
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
    fn connection_ceiling(
        &self,
        http: &HttpDownloadClient,
        final_url: &str,
    ) -> connection_ceiling::Ceiling {
        self.connection_ceilings
            .lock()
            .expect("connection observations")
            .for_resource(http.connection_scope(), final_url)
    }

    fn spawn_owned_worker(
        self: &Arc<Self>,
        work: impl std::future::Future<Output = ()> + Send + 'static,
    ) -> Result<(), DownloadEngineError> {
        let runtime = download_runtime()?;
        let mut workers = self
            .workers
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        self.check_admission()?;
        while let Some(result) = workers.try_join_next() {
            if result.is_err() {
                self.worker_failure.store(true, Ordering::SeqCst);
            }
        }
        workers.spawn_on(work, runtime.handle());
        Ok(())
    }

    fn check_admission(&self) -> Result<(), DownloadEngineError> {
        if self.draining.load(Ordering::SeqCst) {
            Err(DownloadEngineError::Draining)
        } else {
            Ok(())
        }
    }

    fn next_work_revision(&self) -> Result<u64, DownloadEngineError> {
        self.work_revision
            .load(Ordering::SeqCst)
            .checked_add(1)
            .ok_or(DownloadEngineError::CompletionOverflow)
    }

    fn bump_work_revision(&self) -> u64 {
        self.work_revision.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn http_for_operation(
        &self,
        certificate_policy: crate::DownloadCertificatePolicy,
    ) -> Result<HttpDownloadClient, DownloadEngineError> {
        self.http_for_operation_traced(certificate_policy)
            .map(|(client, _)| client)
    }

    /// Returns the operation client and whether it came from the reuse cache.
    ///
    /// Building a `reqwest::Client` re-reads the system proxy and TLS roots and
    /// starts with an empty connection pool, so every probe used to pay a full
    /// TCP+TLS handshake. Clients are reused per (policy generation, route,
    /// certificate policy). A policy update bumps the generation, so a changed
    /// proxy setting takes effect on the next operation; the TTL only bounds
    /// how long a changed *system* proxy (which raises no signal) can be
    /// ignored.
    fn http_for_operation_traced(
        &self,
        certificate_policy: crate::DownloadCertificatePolicy,
    ) -> Result<(HttpDownloadClient, bool), DownloadEngineError> {
        // Read the generation before the snapshot: if an update lands in
        // between, the client is stored under the older generation and is
        // never served again, instead of an old client masquerading as new.
        let generation = self.network_policy.generation();
        let snapshot = self.network_policy.snapshot()?;
        let key = (
            generation,
            snapshot.route_key(),
            certificate_policy == crate::DownloadCertificatePolicy::AcceptInvalid,
        );
        if let Ok(cache) = self.http_clients.lock()
            && let Some((client, built_at)) = cache.get(&key)
            && built_at.elapsed() < HTTP_CLIENT_REUSE_TTL
        {
            return Ok((client.clone(), true));
        }
        let client = self
            .http_template
            .for_network_snapshot(&snapshot, certificate_policy)?
            .with_policy_generation(generation);
        if let Ok(mut cache) = self.http_clients.lock() {
            cache.retain(|(cached_generation, _, _), (_, built_at)| {
                *cached_generation == generation && built_at.elapsed() < HTTP_CLIENT_REUSE_TTL
            });
            cache.insert(key, (client.clone(), Instant::now()));
        }
        Ok((client, false))
    }

    fn lock_tasks(
        &self,
    ) -> Result<MutexGuard<'_, HashMap<DownloadTaskId, EngineTaskState>>, DownloadEngineError> {
        self.tasks
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)
    }

    fn persist(&self, state: &mut EngineTaskState) -> Result<(), DownloadEngineError> {
        let mut task = state.task.clone();
        if let Some(control) = &state.control {
            task.set_transfer_statistics(control.statistics.snapshot());
        }
        self.store.save_task_with_transfer(&task, &state.transfer)?;
        state.task = task;
        Ok(())
    }

    fn emit(&self, event: DownloadEvent) {
        if self.event_sender.len() < 256
            && self.primary_state_event_lost.swap(false, Ordering::AcqRel)
        {
            let observed_sequence = self.event_sequencer.current();
            let recovery = self.event_sequencer.next_with_generation(
                self.session_generation,
                now_unix_ms(),
                DownloadEvent::SnapshotRequired {
                    expected_sequence: observed_sequence,
                    observed_sequence: observed_sequence.saturating_add(1),
                    expected_generation: Some(self.session_generation),
                    observed_generation: Some(self.session_generation),
                },
            );
            if recovery.is_ok_and(|recovery| self.event_sender.try_send(recovery).is_err()) {
                self.primary_state_event_lost.store(true, Ordering::Release);
            }
        }
        let Ok(envelope) = self.event_sequencer.next_with_generation(
            self.session_generation,
            now_unix_ms(),
            event,
        ) else {
            return;
        };
        if matches!(envelope.event, DownloadEvent::TaskProgress { .. }) {
            if self.event_sender.len() < 192 {
                let _ = self.event_sender.try_send(envelope.clone());
            }
        } else if self.event_sender.try_send(envelope.clone()).is_err() {
            self.primary_state_event_lost.store(true, Ordering::Release);
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
            self.emit(DownloadEvent::TaskChanged {
                task: Box::new(task),
            });
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

const fn download_terminal_name(status: DownloadStatus) -> &'static str {
    match status {
        DownloadStatus::Completed => "completed",
        DownloadStatus::Paused => "paused",
        DownloadStatus::Cancelled => "cancelled",
        DownloadStatus::Failed => "failed",
        DownloadStatus::Created
        | DownloadStatus::Probing
        | DownloadStatus::Waiting
        | DownloadStatus::Downloading
        | DownloadStatus::Finalizing
        | DownloadStatus::Verifying => "non_terminal",
    }
}

const fn certificate_policy_name(policy: crate::DownloadCertificatePolicy) -> &'static str {
    match policy {
        crate::DownloadCertificatePolicy::Strict => "strict",
        crate::DownloadCertificatePolicy::AcceptInvalid => "accept_invalid",
    }
}

#[derive(Debug, Clone, Copy)]
struct DownloadAttemptDiagnostics {
    proxy_mode: &'static str,
    proxy_source: &'static str,
    certificate_policy: crate::DownloadCertificatePolicy,
}

fn automatic_media_file_name(
    media: &DownloadRequestMedia,
    requested: Option<&str>,
    probed: Option<&str>,
) -> Option<String> {
    let (manifest_extension, output_extension) = match media {
        DownloadRequestMedia::Hls(_) => ("m3u8", "ts"),
        DownloadRequestMedia::Dash(_) => ("mpd", "mp4"),
    };
    let candidate = requested.or(probed)?;
    let mut path = PathBuf::from(candidate);
    if path
        .extension()
        .is_none_or(|extension| extension.eq_ignore_ascii_case(manifest_extension))
    {
        path.set_extension(output_extension);
    } else {
        return None;
    }
    Some(
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("video.{output_extension}")),
    )
}

type HlsSegmentTransferResult = (
    crate::hls::HlsMediaSegment,
    PathBuf,
    PathBuf,
    Result<HlsSegmentDownloadOutcome, HttpDownloadError>,
);

type DashFragmentTransferResult = (
    usize,
    crate::dash::DashTrackKind,
    Option<u32>,
    PathBuf,
    PathBuf,
    Result<HlsSegmentDownloadOutcome, HttpDownloadError>,
);

type HlsKeyCache = Arc<HashMap<u32, Arc<tokio::sync::Mutex<Option<[u8; 16]>>>>>;

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

const MIN_INITIAL_SEGMENT_BYTES: u64 = 64 * 1024;
const MAX_TRANSFER_ATTEMPTS: u32 = 3;
const TRANSFER_RETRY_BASE_DELAY: Duration = Duration::from_millis(100);

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
    inner.persist(state)?;
    state.control = None;
    let task = Box::new(state.task.clone());
    drop(tasks);
    inner.emit(DownloadEvent::TaskChanged { task });
    Ok(())
}

fn publish_transfer_progress(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    downloaded_bytes: u64,
    total_bytes: Option<u64>,
    active_connections: u16,
    progress_by_segment: Option<&Mutex<HashMap<u32, u64>>>,
    segment_lengths: Option<&Mutex<HashMap<u32, (u64, u64)>>>,
) {
    if let Ok(mut tasks) = inner.tasks.lock()
        && let Some(state) = tasks.get_mut(&task_id)
    {
        state.transfer.downloaded_bytes = state.transfer.downloaded_bytes.max(downloaded_bytes);
        let now = Instant::now();
        let should_publish = state
            .last_progress_event
            .is_none_or(|last| now.duration_since(last) >= PROGRESS_EVENT_MIN_INTERVAL)
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
            drop(tasks);
            inner.emit(DownloadEvent::TaskProgress {
                progress: DownloadProgress {
                    task_id,
                    downloaded_bytes: published_downloaded_bytes,
                    total_bytes,
                    bytes_per_second,
                    active_connections,
                    segments,
                    finishing: None,
                },
            });
        }
    }
}
fn publish_finishing_progress(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    finishing: DownloadFinishingProgress,
    force: bool,
) {
    if let Ok(mut tasks) = inner.tasks.lock()
        && let Some(state) = tasks.get_mut(&task_id)
    {
        let now = Instant::now();
        if force
            || state
                .last_progress_event
                .is_none_or(|last| now.duration_since(last) >= PROGRESS_EVENT_MIN_INTERVAL)
        {
            state.last_progress_event = Some(now);
            let progress = DownloadProgress {
                task_id,
                downloaded_bytes: state.transfer.downloaded_bytes,
                total_bytes: state
                    .transfer
                    .probe
                    .as_ref()
                    .and_then(|probe| probe.total_bytes),
                bytes_per_second: 0,
                active_connections: 0,
                segments: Vec::new(),
                finishing: Some(finishing),
            };
            drop(tasks);
            inner.emit(DownloadEvent::TaskProgress { progress });
        }
    }
}

fn download_segment_progress_snapshot(
    values: &HashMap<u32, u64>,
    lengths: Option<&HashMap<u32, (u64, u64)>>,
) -> Vec<DownloadSegmentProgress> {
    let mut segments = values
        .iter()
        .map(
            |(segment_index, downloaded_bytes)| DownloadSegmentProgress {
                segment_index: *segment_index,
                start_byte: lengths
                    .and_then(|lengths| lengths.get(segment_index).map(|range| range.0)),
                downloaded_bytes: *downloaded_bytes,
                total_bytes: lengths
                    .and_then(|lengths| lengths.get(segment_index).map(|range| range.1)),
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
    committed.capture_transfer_statistics();
    committed.control = None;
    inner.persist(&mut committed)?;
    *state = committed;
    let task = Box::new(state.task.clone());
    drop(tasks);
    inner.emit(DownloadEvent::TaskChanged { task });
    Ok(())
}

fn record_failure(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    elapsed: Duration,
    diagnostics: DownloadAttemptDiagnostics,
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
        proxy_mode = diagnostics.proxy_mode,
        proxy_source = diagnostics.proxy_source,
        certificate_policy = certificate_policy_name(diagnostics.certificate_policy),
        fault_code = ?fault.code,
        retryable = fault.retryable,
        native_error = error.native_os_error().unwrap_or_default(),
        elapsed_ms = elapsed.as_millis(),
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
    if let Err(storage_error) = inner.persist(state) {
        // Preserve the existing best-effort failure publication policy, but
        // never advertise a reliable measurement that wasn't committed.
        state.task.set_transfer_statistics(None);
        let sqlite_error = match &storage_error {
            DownloadEngineError::Storage(DownloadStorageError::Sqlite(
                rusqlite::Error::SqliteFailure(native, _),
            )) => Some(native.extended_code),
            _ => None,
        };
        warn!(event = "download.statistics.commit_failed", %correlation_id, ?task_id,
            native_error = storage_error.native_os_error().unwrap_or_default(),
            sqlite_extended_code = ?sqlite_error,
            "terminal transfer statistics could not be committed");
    }
    state.control = None;
    let task = Box::new(state.task.clone());
    drop(tasks);
    inner.emit(DownloadEvent::TaskChanged { task });
}

fn target_reserved_by_tasks(
    tasks: &HashMap<DownloadTaskId, EngineTaskState>,
    candidate: &Path,
) -> bool {
    let candidate = windows_target_key(candidate);
    tasks.values().any(|state| {
        if state.transfer.final_committed {
            return false;
        }
        let Some(file_name) = state.task.request().file_name.as_deref() else {
            return false;
        };
        windows_target_key(&state.task.request().save_directory.join(file_name)) == candidate
    })
}

fn windows_target_key(path: &Path) -> String {
    let resolved = path.parent().map_or_else(
        || path.to_owned(),
        |parent| {
            std::fs::canonicalize(parent)
                .unwrap_or_else(|_| parent.to_owned())
                .join(path.file_name().unwrap_or_default())
        },
    );
    crate::organization::windows_path_key(&resolved)
}

fn apply_organization_to_request(
    inner: &DownloadEngineInner,
    request: &mut DownloadRequest,
    tasks: &HashMap<DownloadTaskId, EngineTaskState>,
    correlation_id: uuid::Uuid,
) -> Result<(), DownloadOrganizationPolicyError> {
    let started_at = Instant::now();
    let category_was_explicit = request.category_id.is_some();
    let original_file_name = request.file_name.clone();
    if let Err(error) = inner
        .download_organization_policy
        .apply_to_request(request, |candidate| {
            target_reserved_by_tasks(tasks, candidate)
        })
    {
        let failure_kind = match &error {
            DownloadOrganizationPolicyError::TargetOccupied => "target_occupied",
            DownloadOrganizationPolicyError::NumberedTargetExhausted => "numbering_exhausted",
            DownloadOrganizationPolicyError::LockPoisoned => "lock",
            DownloadOrganizationPolicyError::TooManyRules
            | DownloadOrganizationPolicyError::InvalidCategoryIdentity
            | DownloadOrganizationPolicyError::InvalidCategoryDirectory
            | DownloadOrganizationPolicyError::CategoryUnavailable
            | DownloadOrganizationPolicyError::CategoryTextTooLarge => "invalid_policy",
            DownloadOrganizationPolicyError::Io(_) => "filesystem",
        };
        warn!(
            event = "download.organization.rejected",
            %correlation_id,
            failure_kind,
            duplicate_strategy = duplicate_strategy_name(request.duplicate_strategy),
            native_error_code = organization_native_error(&error),
            elapsed_ms = started_at.elapsed().as_millis(),
            "download organization resolution rejected the task"
        );
        return Err(error);
    }
    let classification_source = if request.category_id.is_none() {
        "unclassified"
    } else if category_was_explicit {
        "explicit"
    } else {
        "automatic"
    };
    let conflict_outcome =
        if original_file_name.is_some() && original_file_name != request.file_name {
            "numbered"
        } else {
            "available"
        };
    info!(
        event = "download.organization.resolved",
        %correlation_id,
        classification_source,
        category_id = request.category_id.as_deref().unwrap_or("none"),
        duplicate_strategy = duplicate_strategy_name(request.duplicate_strategy),
        conflict_outcome,
        elapsed_ms = started_at.elapsed().as_millis(),
        "download organization resolution completed"
    );
    Ok(())
}

const fn duplicate_strategy_name(strategy: crate::DownloadDuplicateStrategy) -> &'static str {
    match strategy {
        crate::DownloadDuplicateStrategy::UseDefault => "use_default",
        crate::DownloadDuplicateStrategy::AddNumbered => "add_numbered",
        crate::DownloadDuplicateStrategy::Reject => "reject",
    }
}

fn organization_native_error(error: &DownloadOrganizationPolicyError) -> Option<i32> {
    match error {
        DownloadOrganizationPolicyError::Io(error) => error.raw_os_error(),
        _ => None,
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

fn probe_error_kind(error: &DownloadEngineError) -> &'static str {
    match error {
        DownloadEngineError::InvalidRequest(_) => "invalid_request",
        DownloadEngineError::Http(error) => error.kind(),
        DownloadEngineError::Runtime(_) => "runtime_unavailable",
        _ => "internal",
    }
}

fn new_engine_session_generation() -> u64 {
    let generation = uuid::Uuid::new_v4().as_u128() as u64;
    if generation == 0 { 1 } else { generation }
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
    Resolution(#[from] crate::DownloadResolutionError),
    #[error(transparent)]
    Task(#[from] DownloadTaskError),
    #[error(transparent)]
    Storage(#[from] DownloadStorageError),
    #[error(transparent)]
    Http(#[from] HttpDownloadError),
    #[error(transparent)]
    NetworkPolicy(#[from] NetworkPolicyError),
    #[error(transparent)]
    DownloadHttpPolicy(#[from] DownloadHttpPolicyError),
    #[error(transparent)]
    DownloadOrganizationPolicy(#[from] DownloadOrganizationPolicyError),
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
    #[error("FFmpeg media process failed: {kind}")]
    MediaProcess {
        kind: &'static str,
        native_error_code: Option<i32>,
    },
    #[error("未找到 FFmpeg；请打开“下载设置 → 媒体工具”指定 ffmpeg.exe，或使用其中的公开下载链接")]
    FfmpegUnavailable,
    #[error("FFmpeg automatic installation is already running")]
    FfmpegInstallAlreadyActive,
    #[error("download task does not exist: {0:?}")]
    TaskNotFound(DownloadTaskId),
    #[error("download task has no active transfer control: {0:?}")]
    MissingTransferControl(DownloadTaskId),
    #[error("download task has no temporary artifact path: {0:?}")]
    MissingTemporaryPath(DownloadTaskId),
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
    #[error("download task cannot be retried because its last failure is not retryable: {0:?}")]
    RetryNotAllowed(DownloadTaskId),
    #[error(
        "redownload could not clean the original task {old_task_id:?}; replacement {new_task_id:?} was rolled back"
    )]
    RedownloadCleanupFailed {
        old_task_id: DownloadTaskId,
        new_task_id: DownloadTaskId,
    },
    #[error(
        "redownload rollback failed for original task {old_task_id:?} and replacement {new_task_id:?}"
    )]
    RedownloadRollbackFailed {
        old_task_id: DownloadTaskId,
        new_task_id: DownloadTaskId,
    },
    #[error("download queue concurrency must be at least one")]
    InvalidQueueConcurrency,
    #[error("download engine is draining")]
    Draining,
    #[error("download completion authority counter overflow")]
    CompletionOverflow,
    #[error("browser batch must contain between 1 and {maximum} items, got {count}")]
    InvalidBrowserBatchSize { count: usize, maximum: usize },
    #[error("browser batch contains a duplicate handoff ID")]
    DuplicateBrowserBatchHandoff,
    #[error("idempotency key {0} conflicts with a previously accepted request")]
    IdempotencyConflict(uuid::Uuid),
    #[error("browser handoff {0} conflicts with a previously accepted request")]
    BrowserHandoffConflict(uuid::Uuid),
    #[error("browser handoff {handoff_id} expects task {expected:?}, not {actual:?}")]
    BrowserHandoffTaskMismatch {
        handoff_id: uuid::Uuid,
        expected: DownloadTaskId,
        actual: DownloadTaskId,
    },
    #[error("browser handoff {0} has no durable task identity")]
    BrowserHandoffMissingTask(uuid::Uuid),
    #[error("download task remains held for browser ownership transfer: {0:?}")]
    BrowserHandoffHeld(DownloadTaskId),
    #[error("download task is not eligible for queue reordering: {0:?}")]
    InvalidQueueMove(DownloadTaskId),
    #[error("download engine task lock is poisoned")]
    LockPoisoned,
    #[error("the download engine event subscriber is already attached")]
    EventSubscriberAlreadyTaken,
    #[error("the browser confirmation-window subscriber is already attached")]
    BrowserDialogSubscriberAlreadyTaken,
    #[error("the browser confirmation-window consumer is not attached yet")]
    BrowserDialogConsumerUnavailable,
    #[error("the browser confirmation window is unavailable")]
    BrowserDialogUnavailable,
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
            Self::Resolution(crate::DownloadResolutionError::Cancelled) => {
                (DownloadFaultCode::Cancelled, false)
            }
            Self::Resolution(crate::DownloadResolutionError::InvalidResult) => {
                (DownloadFaultCode::InvalidRequest, false)
            }
            Self::Resolution(_) => (DownloadFaultCode::ResolutionRequired, true),
            Self::InvalidRequest(_)
            | Self::InvalidBrowserBatchSize { .. }
            | Self::DuplicateBrowserBatchHandoff
            | Self::IdempotencyConflict(_)
            | Self::BrowserHandoffConflict(_) => (DownloadFaultCode::InvalidRequest, false),
            Self::Task(_)
            | Self::Segment(_)
            | Self::TransferPlan(_)
            | Self::RateLimit(_)
            | Self::Schedule(_)
            | Self::TaskNotFound(_)
            | Self::MissingTransferControl(_)
            | Self::MissingTemporaryPath(_)
            | Self::MissingSegmentedLength(_)
            | Self::EmptySegmentLayout(_)
            | Self::IncompleteSegmentLayout(_)
            | Self::FfmpegInstallAlreadyActive
            | Self::RetryNotAllowed(_)
            | Self::InvalidQueueConcurrency
            | Self::InvalidQueueMove(_)
            | Self::BrowserHandoffTaskMismatch { .. }
            | Self::BrowserHandoffMissingTask(_)
            | Self::BrowserHandoffHeld(_)
            | Self::Draining
            | Self::CompletionOverflow
            | Self::InvalidAction { .. } => (DownloadFaultCode::InvalidState, false),
            Self::RedownloadCleanupFailed { .. } => (DownloadFaultCode::Storage, true),
            Self::RedownloadRollbackFailed { .. } => (DownloadFaultCode::Internal, false),
            Self::MediaProcess { .. } | Self::FfmpegUnavailable => {
                (DownloadFaultCode::Unavailable, false)
            }
            Self::StorageCapacity(_) => (DownloadFaultCode::Storage, false),
            Self::Storage(_) | Self::LockPoisoned | Self::Io(_) => {
                (DownloadFaultCode::Storage, true)
            }
            Self::EventSubscriberAlreadyTaken | Self::BrowserDialogSubscriberAlreadyTaken => {
                (DownloadFaultCode::InvalidState, false)
            }
            Self::BrowserDialogConsumerUnavailable => (DownloadFaultCode::Unavailable, true),
            Self::BrowserDialogUnavailable => (DownloadFaultCode::Unavailable, false),
            Self::Http(HttpDownloadError::InsufficientDiskSpace { .. }) => {
                (DownloadFaultCode::DiskFull, false)
            }
            Self::Http(HttpDownloadError::DestinationExists(_)) => {
                (DownloadFaultCode::DestinationExists, false)
            }
            Self::Http(HttpDownloadError::UnexpectedWebPage) => {
                (DownloadFaultCode::UnexpectedWebPage, false)
            }
            Self::Http(error) if error.is_retryable_transfer() => {
                (DownloadFaultCode::Network, true)
            }
            Self::ResourceChanged(_) => (DownloadFaultCode::Protocol, false),
            Self::Http(_) => (DownloadFaultCode::Protocol, false),
            Self::NetworkPolicy(_) => (DownloadFaultCode::Unavailable, false),
            Self::DownloadHttpPolicy(_) => (DownloadFaultCode::InvalidRequest, false),
            Self::DownloadOrganizationPolicy(
                DownloadOrganizationPolicyError::TargetOccupied
                | DownloadOrganizationPolicyError::NumberedTargetExhausted
                | DownloadOrganizationPolicyError::TooManyRules
                | DownloadOrganizationPolicyError::InvalidCategoryIdentity
                | DownloadOrganizationPolicyError::InvalidCategoryDirectory
                | DownloadOrganizationPolicyError::CategoryUnavailable
                | DownloadOrganizationPolicyError::CategoryTextTooLarge,
            ) => (DownloadFaultCode::InvalidRequest, false),
            Self::DownloadOrganizationPolicy(
                DownloadOrganizationPolicyError::LockPoisoned
                | DownloadOrganizationPolicyError::Io(_),
            ) => (DownloadFaultCode::Storage, true),
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
            Self::DownloadOrganizationPolicy(DownloadOrganizationPolicyError::Io(error)) => {
                Some(error)
            }
            _ => None,
        }
    }

    fn native_os_error(&self) -> Option<i32> {
        self.io_error()
            .and_then(std::io::Error::raw_os_error)
            .or(match self {
                Self::MediaProcess {
                    native_error_code, ..
                } => *native_error_code,
                _ => None,
            })
    }

    fn media_process(error: crate::media::MediaProcessError) -> Self {
        Self::MediaProcess {
            kind: error.kind(),
            native_error_code: error.native_error_code(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::browser::{BrowserBlobPersistenceError, persist_browser_blob_with_compensation};
    use super::queue::{
        daily_schedule_desired_running, daily_schedule_wake_moments, next_daily_schedule_trigger,
    };
    use super::*;
    use crate::http::InjectedArtifactFault;
    use crate::{
        DownloadDailySchedule, DownloadQuery, DownloadQueue, DownloadSchedule, DownloadSnapshot,
        DownloadTaskDetail, ResourceValidator, SegmentedArtifactError,
    };
    use chrono::{Datelike, Local, TimeZone, Timelike};
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn configured_ffmpeg_is_applied_to_the_live_engine_without_restart() {
        let directory = tempfile::tempdir().expect("create configured FFmpeg directory");
        let executable = directory.path().join("ffmpeg-configured.exe");
        std::fs::write(&executable, b"fixture").expect("write FFmpeg fixture");
        let engine = DownloadEngine::open_in_memory().expect("open download engine");

        engine
            .set_ffmpeg_executable_path(Some(executable.clone()))
            .expect("apply configured FFmpeg executable");

        let resolution = engine
            .ffmpeg_resolution()
            .expect("resolve configured FFmpeg")
            .expect("configured FFmpeg should be available");
        assert_eq!(resolution.executable, executable);
        assert_eq!(resolution.source, crate::FfmpegResolutionSource::Configured);
    }

    #[tokio::test]
    async fn dash_concatenation_pause_interrupts_real_file_io_and_removes_partial_output() {
        let directory = tempfile::tempdir().expect("create DASH concat test directory");
        let fragment = directory.path().join("fragment.m4s");
        let output = directory.path().join("output.part");
        let file = std::fs::File::create(&fragment).expect("create sparse fragment");
        file.set_len(64 * 1024 * 1024)
            .expect("size sparse fragment");
        let control = DownloadTransferControl::new();
        let worker_control = control.clone();
        let worker_fragment = fragment.clone();
        let worker_output = output.clone();
        let worker = tokio::spawn(async move {
            concatenate_dash_fragments(
                &[worker_fragment],
                &worker_output,
                &worker_control,
                0,
                64 * 1024 * 1024,
                |_, _| {},
            )
            .await
        });
        let deadline = Instant::now() + Duration::from_secs(2);
        while !std::fs::metadata(&output).is_ok_and(|metadata| metadata.len() > 0) {
            assert!(
                Instant::now() < deadline,
                "DASH concat did not start writing"
            );
            tokio::task::yield_now().await;
        }
        control.pause();
        let outcome = worker
            .await
            .expect("join DASH concat worker")
            .expect("interrupt DASH concat");
        assert_eq!(outcome, crate::media::MediaProcessOutcome::Paused);
        assert!(!output.exists());
    }

    struct FixedStorageCapacity(u64);

    impl StorageCapacityProbe for FixedStorageCapacity {
        fn available_space(&self, _path: &Path) -> Option<u64> {
            Some(self.0)
        }
    }

    #[test]
    fn browser_blob_database_failure_restores_the_staged_artifact() {
        let directory = tempfile::tempdir().expect("create Blob rollback directory");
        let database = directory.path().join("downloads.sqlite3");
        let staging = directory.path().join("browser-blob-staging");
        let destination = directory.path().join("downloads");
        std::fs::create_dir_all(&staging).expect("create Blob staging root");
        let blob_id = uuid::Uuid::new_v4();
        let staged = staging.join(format!("{blob_id}.blob"));
        std::fs::write(&staged, b"rollback").expect("stage rollback fixture");
        let engine = DownloadEngine::open(database).expect("open download engine");
        engine
            .inner
            .store
            .inject_browser_handoff_insert_failure()
            .expect("inject browser handoff failure");

        let result = engine.import_browser_blob_correlated(
            BrowserBlobImportRequest {
                handoff_id: uuid::Uuid::new_v4(),
                blob: BrowserBlob {
                    id: blob_id,
                    file_name: "rollback.bin".into(),
                    media_type: Some("application/octet-stream".into()),
                    size: 8,
                },
                save_directory: destination.clone(),
                file_name: "rollback.bin".into(),
                category_id: None,
                save_directory_explicit: false,
            },
            uuid::Uuid::new_v4(),
        );

        assert!(result.is_err());
        assert_eq!(
            std::fs::read(&staged).expect("read restored staging"),
            b"rollback"
        );
        assert!(!destination.join("rollback.bin").exists());
        assert!(
            engine
                .command_snapshot(DownloadQuery::default())
                .expect("read rollback snapshot")
                .tasks
                .is_empty()
        );
    }

    #[test]
    fn browser_blob_rollback_failure_is_a_typed_terminal_error() {
        let directory = tempfile::tempdir().expect("create rollback failure directory");
        let staged = directory.path().join("staged.blob");
        let final_path = directory.path().join("final.bin");
        std::fs::create_dir(&staged).expect("block the rollback destination with a directory");
        std::fs::write(&final_path, b"committed").expect("create final artifact fixture");

        let result = persist_browser_blob_with_compensation(&staged, &final_path, || {
            Err(DownloadStorageError::LockPoisoned)
        });

        assert!(matches!(
            result,
            Err(BrowserBlobPersistenceError::Rollback { .. })
        ));
        assert!(final_path.exists());
    }

    #[test]
    fn browser_blob_discard_failure_is_returned_from_the_real_file_boundary() {
        let directory = tempfile::tempdir().expect("create discard failure directory");
        let database = directory.path().join("downloads.sqlite3");
        let staging = directory.path().join("browser-blob-staging");
        std::fs::create_dir_all(&staging).expect("create Blob staging root");
        let blob_id = uuid::Uuid::new_v4();
        let blocked = staging.join(format!("{blob_id}.blob"));
        std::fs::create_dir(&blocked).expect("block file removal with a directory");
        let engine = DownloadEngine::open(database).expect("open download engine");

        let result =
            engine.discard_browser_blob(uuid::Uuid::new_v4(), blob_id, uuid::Uuid::new_v4());

        assert!(result.is_err());
        assert!(blocked.is_dir());
    }

    #[test]
    fn browser_reported_size_is_visible_before_the_http_probe() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/browser-size.bin".into(),
                    save_directory: std::env::temp_dir(),
                    save_directory_explicit: false,
                    file_name: Some("browser-size.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    source_page: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(1),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: Some(12_345),
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Browser,
            )
            .expect("enqueue browser task");

        let task = engine
            .task(task_id)
            .expect("read browser task")
            .expect("browser task exists");
        assert_eq!(task.total_bytes, Some(12_345));
    }

    #[test]
    fn operation_client_is_reused_until_the_network_policy_changes() {
        use nxtools_core::{NetworkProxyMode, NetworkProxySettings, NetworkSettings};

        let policy = NetworkPolicy::new(NetworkSettings::default()).expect("direct policy");
        let engine = DownloadEngine::open_in_memory_with_network_policy(policy.clone())
            .expect("open engine");
        let strict = crate::DownloadCertificatePolicy::Strict;

        let (_, first_reused) = engine
            .inner
            .http_for_operation_traced(strict)
            .expect("first client");
        let (_, second_reused) = engine
            .inner
            .http_for_operation_traced(strict)
            .expect("second client");
        assert!(!first_reused, "the first operation builds a client");
        assert!(second_reused, "an unchanged policy reuses the warm client");

        policy
            .update(NetworkSettings {
                proxy: NetworkProxySettings {
                    mode: NetworkProxyMode::ManualHttp,
                    host: "127.0.0.1".into(),
                    port: 3128,
                },
            })
            .expect("switch to a manual proxy");
        let (rebuilt, rebuilt_reused) = engine
            .inner
            .http_for_operation_traced(strict)
            .expect("client after the policy change");
        assert!(
            !rebuilt_reused,
            "a policy update must not be served a client built for the old route"
        );
        assert_eq!(
            rebuilt.network_proxy_mode(),
            NetworkProxyMode::ManualHttp,
            "the rebuilt client must follow the new route"
        );

        // A same-value update still bumps the generation and rebuilds once;
        // that is conservative, never stale.
        policy
            .update(NetworkSettings::default())
            .expect("switch back to direct");
        let (back, back_reused) = engine
            .inner
            .http_for_operation_traced(strict)
            .expect("client after switching back");
        assert!(!back_reused);
        assert_eq!(back.network_proxy_mode(), NetworkProxyMode::Direct);
    }

    #[test]
    fn browser_dialog_keeps_explicit_context_raw_until_one_final_acceptance_snapshot() {
        let policy_a = DownloadHttpPolicy::from_download_settings(&DownloadSettings {
            default_request_headers: "X-NxTools-Generation: a".into(),
            default_cookie: "generation=a".into(),
            ..DownloadSettings::default()
        })
        .expect("create initial browser-dialog policy");
        let engine = DownloadEngine::open_in_memory_with_policies(
            NetworkPolicy::new(NetworkSettings::default()).expect("create network policy"),
            policy_a,
        )
        .expect("open browser-dialog engine");
        let dialogs = engine
            .take_browser_dialog_receiver()
            .expect("attach dialog consumer");
        let command_engine = engine.clone();
        let handoff_id = uuid::Uuid::new_v4();
        let dialog_thread = std::thread::spawn(move || {
            command_engine.handle_command(DownloadCommandEnvelope {
                protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
                request_id: uuid::Uuid::new_v4(),
                origin: RequestOrigin::Browser,
                command: DownloadCommand::BrowserOpenDialog {
                    handoff_id,
                    request: DownloadRequest {
                        source_url: "https://example.test/browser.bin".into(),
                        save_directory: std::env::temp_dir(),
                        save_directory_explicit: false,
                        file_name: Some("browser.bin".into()),
                        headers: vec![crate::DownloadRequestHeader {
                            name: "X-NxTools-Explicit".into(),
                            value: "browser".into(),
                        }],
                        cookie: None,
                        referrer: None,
                        source_page: None,
                        certificate_policy: Default::default(),
                        connection_limit: Some(1),
                        speed_limit_bytes_per_second: None,
                        expected_total_bytes: None,
                        scheduled_at_unix_ms: None,
                        adaptive_segment_policy: None,
                        media: None,
                        category_id: None,
                        category_name: None,
                        duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
                    },
                },
            })
        });
        let dialog = dialogs.recv_blocking().expect("receive browser dialog");
        let BrowserDownloadDialogSource::Download(raw_request) = dialog.source else {
            panic!("expected ordinary browser request");
        };
        assert!(raw_request.cookie.is_none());
        assert!(
            raw_request
                .headers
                .iter()
                .all(|header| !header.name.eq_ignore_ascii_case("x-nxtools-generation"))
        );
        dialog
            .completion
            .send(Ok(()))
            .expect("confirm browser dialog");
        assert!(matches!(
            dialog_thread.join().expect("join dialog command").outcome,
            DownloadCommandOutcome::BrowserDialogAccepted { .. }
        ));

        engine
            .update_download_http_policy(&DownloadSettings {
                default_request_headers: "X-NxTools-Generation: b".into(),
                default_cookie: "generation=b".into(),
                accept_invalid_certificates: true,
                ..DownloadSettings::default()
            })
            .expect("publish final acceptance policy");
        let accepted = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Browser,
            command: DownloadCommand::Submit {
                request: *raw_request,
                mode: DownloadSubmitMode::QueueOnly,
                idempotency_key: None,
            },
        });
        let DownloadCommandOutcome::Accepted { task_id, .. } = accepted.outcome else {
            panic!("browser confirmation must create a task");
        };
        let task_id = task_id.expect("new browser confirmation creates a task identity");
        let accepted = engine
            .task(task_id)
            .expect("read accepted browser task")
            .expect("browser task exists");
        assert_eq!(
            accepted.task.request().cookie.as_deref(),
            Some("generation=b")
        );
        assert_eq!(
            accepted.task.request().certificate_policy,
            crate::DownloadCertificatePolicy::AcceptInvalid
        );
        assert!(accepted.task.request().headers.iter().any(|header| {
            header.name.eq_ignore_ascii_case("x-nxtools-generation") && header.value == "b"
        }));
        assert!(accepted.task.request().headers.iter().any(|header| {
            header.name.eq_ignore_ascii_case("x-nxtools-explicit") && header.value == "browser"
        }));
    }

    #[test]
    fn task_detail_command_returns_only_the_selected_tasks_segments_and_bounded_timeline() {
        let engine = DownloadEngine::open_in_memory().expect("open engine");
        let task_id = engine
            .enqueue(
                DownloadRequest {
                    source_url: "https://example.test/detail.bin".into(),
                    save_directory: std::env::temp_dir(),
                    save_directory_explicit: false,
                    file_name: Some("detail.bin".into()),
                    headers: vec![crate::DownloadRequestHeader {
                        name: "Authorization".into(),
                        value: "secret-value-must-not-enter-the-timeline".into(),
                    }],
                    cookie: Some("private-cookie".into()),
                    referrer: None,
                    source_page: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(2),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
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
                    save_directory_explicit: false,
                    file_name: Some("rebalanced.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    source_page: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(2),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
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
                    save_directory_explicit: false,
                    file_name: Some("progress.bin".into()),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    source_page: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(2),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: None,
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )
            .expect("enqueue task");
        while events.try_recv().is_ok() {}
        let segment_progress = Mutex::new(HashMap::from([(0, 5), (1, 7)]));
        let segment_lengths = Mutex::new(HashMap::from([(0, (0, 10)), (1, (10, 10))]));

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
            completion_intention: Default::default(),
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
        let conflict = DownloadEngineError::Http(HttpDownloadError::DestinationExists(
            PathBuf::from("existing.bin"),
        ))
        .fault();
        assert_eq!(conflict.code, DownloadFaultCode::DestinationExists);
        assert!(!conflict.retryable);
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
                save_directory_explicit: false,
                file_name: Some("must-not-exist.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                source_page: None,
                certificate_policy: Default::default(),
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
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
            save_directory_explicit: false,
            file_name: Some("must-not-exist.bin".into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            source_page: None,
            certificate_policy: Default::default(),
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
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
                save_directory_explicit: false,
                file_name: Some("final.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                source_page: None,
                certificate_policy: Default::default(),
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
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
                media: None,
                resolution: None,
                server_observation: None,
            }),
            downloaded_bytes: 16,
            error: None,
            prepared_size_bytes: None,
            temporary_path: Some(directory.path().join(".final.part")),
            final_path: Some(directory.path().join("final.bin")),
            final_committed: false,
        };
        engine
            .inner
            .store
            .save_task_with_transfer(&task, &transfer)
            .expect("persist finalizing state");
        let saved_statistics = task.transfer_statistics();
        let control = DownloadTransferControl::new();
        control.statistics.add_bytes(100);
        engine.inner.tasks.lock().expect("lock tasks").insert(
            task_id,
            EngineTaskState {
                task,
                transfer,
                control: Some(control),
                last_progress_event: None,
                transfer_rate: None,
                paused_url_replacement: None,
                accepted_revision: 0,
                completion_batch_id: None,
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
        assert_eq!(
            snapshot.task.transfer_statistics(),
            saved_statistics,
            "failed terminal commit must not publish a measured pair ahead of SQLite"
        );
        assert_eq!(
            engine
                .inner
                .store
                .load_task(task_id)
                .expect("durable failure task")
                .expect("task")
                .transfer_statistics(),
            saved_statistics
        );
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
                    finishing: None,
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

    fn action_envelope(
        task_id: DownloadTaskId,
        action: DownloadTaskAction,
    ) -> DownloadCommandEnvelope {
        DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Internal,
            command: DownloadCommand::Task { task_id, action },
        }
    }

    fn batch_envelope(
        task_ids: Vec<DownloadTaskId>,
        action: DownloadBatchAction,
    ) -> DownloadCommandEnvelope {
        DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Internal,
            command: DownloadCommand::Batch { task_ids, action },
        }
    }

    fn action_request(save_directory: &Path, file_name: &str) -> DownloadRequest {
        DownloadRequest {
            source_url: "https://example.test/action.bin".into(),
            save_directory: save_directory.to_owned(),
            save_directory_explicit: false,
            file_name: Some(file_name.into()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            source_page: None,
            certificate_policy: Default::default(),
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
        }
    }

    fn action_plan() -> TransferPlan {
        TransferPlan::new(
            TransferProtocol::Http,
            "https://example.test/action.bin".into(),
            Some(16),
            true,
            ResourceValidator::default(),
            vec![
                DownloadSegment::new(0, 0, 15, 4, DownloadSegmentState::Completed)
                    .expect("action checkpoint segment"),
            ],
        )
        .expect("action transfer plan")
    }

    fn seed_failed_action_task(
        engine: &DownloadEngine,
        request: DownloadRequest,
        fault: DownloadFault,
        temporary_path: Option<PathBuf>,
    ) -> (DownloadTaskId, TransferPlan) {
        let task_id = engine
            .enqueue(request, RequestOrigin::Internal)
            .expect("enqueue action fixture");
        let plan = action_plan();
        {
            let mut tasks = engine.inner.lock_tasks().expect("lock action tasks");
            let state = tasks.get_mut(&task_id).expect("action task state");
            let base = state.task.created_at_unix_ms().saturating_add(1);
            state
                .task
                .transition_to(DownloadStatus::Probing, base)
                .expect("enter probing action state");
            state
                .task
                .transition_to(DownloadStatus::Failed, base.saturating_add(1))
                .expect("enter failed action state");
            state.transfer.downloaded_bytes = 4;
            state.transfer.error = Some(fault);
            state.transfer.temporary_path = temporary_path;
            engine.inner.persist(state).expect("persist failed action");
        }
        engine
            .inner
            .store
            .save_transfer_plan(task_id, &plan)
            .expect("persist action checkpoint");
        (task_id, plan)
    }

    #[test]
    fn failed_uncommitted_task_reserves_name_for_numbered_and_reject_submissions() {
        let directory = tempfile::tempdir().expect("create failed-name directory");
        let engine = DownloadEngine::open_in_memory().expect("open failed-name engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop queue");
        let (failed_id, _) = seed_failed_action_task(
            &engine,
            action_request(directory.path(), "reserved.bin"),
            DownloadFault {
                code: DownloadFaultCode::Protocol,
                message: "nonretryable fixture".into(),
                retryable: false,
            },
            None,
        );
        let mut numbered = action_request(directory.path(), "reserved.bin");
        numbered.duplicate_strategy = crate::DownloadDuplicateStrategy::AddNumbered;
        let numbered_id = engine
            .enqueue(numbered, RequestOrigin::Internal)
            .expect("numbered admission");
        assert_ne!(numbered_id, failed_id);
        assert_eq!(
            engine
                .task(numbered_id)
                .expect("read numbered task")
                .expect("numbered task")
                .task
                .request()
                .file_name
                .as_deref(),
            Some("reserved (1).bin")
        );

        let mut rejected = action_request(directory.path(), "reserved.bin");
        rejected.duplicate_strategy = crate::DownloadDuplicateStrategy::Reject;
        assert!(matches!(
            engine.enqueue(rejected, RequestOrigin::Internal),
            Err(DownloadEngineError::DownloadOrganizationPolicy(
                DownloadOrganizationPolicyError::TargetOccupied
            ))
        ));
    }

    #[test]
    fn cancelled_uncommitted_task_reserves_name_for_redownload_and_reject() {
        let directory = tempfile::tempdir().expect("create cancelled-name directory");
        let engine = DownloadEngine::open_in_memory().expect("open cancelled-name engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop queue");
        let task_id = engine
            .enqueue(
                action_request(directory.path(), "cancelled.bin"),
                RequestOrigin::Internal,
            )
            .expect("enqueue original");
        engine.cancel(task_id).expect("cancel original");
        let mut rejected = action_request(directory.path(), "cancelled.bin");
        rejected.duplicate_strategy = crate::DownloadDuplicateStrategy::Reject;
        assert!(matches!(
            engine.enqueue(rejected, RequestOrigin::Internal),
            Err(DownloadEngineError::DownloadOrganizationPolicy(
                DownloadOrganizationPolicyError::TargetOccupied
            ))
        ));
        let result =
            engine.handle_command(action_envelope(task_id, DownloadTaskAction::Redownload));
        let DownloadCommandOutcome::Accepted {
            task_id: Some(new_id),
        } = result.outcome
        else {
            panic!("cancelled task can redownload");
        };
        assert_ne!(task_id, new_id);
        assert_eq!(
            engine
                .task(new_id)
                .expect("read replacement")
                .expect("replacement")
                .task
                .request()
                .file_name
                .as_deref(),
            Some("cancelled (1).bin")
        );
    }

    #[test]
    fn auto_rename_retry_keeps_task_checkpoint_and_staged_bytes() {
        let directory = tempfile::tempdir().expect("create conflict retry directory");
        let database = directory.path().join("downloads.sqlite3");
        let staged = directory.path().join(".conflict.part");
        let destination = directory.path().join("conflict.bin");
        std::fs::write(&staged, b"checkpoint bytes").expect("stage checkpoint");
        let engine = DownloadEngine::open(&database).expect("open engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop queue");
        let (task_id, plan) = seed_failed_action_task(
            &engine,
            action_request(directory.path(), "conflict.bin"),
            DownloadFault {
                code: DownloadFaultCode::DestinationExists,
                message: "destination already exists".into(),
                retryable: false,
            },
            Some(staged.clone()),
        );
        std::fs::write(&destination, b"external bytes").expect("create conflict after admission");
        let request_id = uuid::Uuid::new_v4();
        let result = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id,
            origin: RequestOrigin::Ui,
            command: DownloadCommand::Task {
                task_id,
                action: DownloadTaskAction::RetryWithAutoRename,
            },
        });
        assert_eq!(result.request_id, request_id);
        assert!(
            matches!(result.outcome, DownloadCommandOutcome::Accepted { task_id: Some(id) } if id == task_id)
        );
        let restored = DownloadEngine::open(&database).expect("reopen renamed task");
        let task = restored
            .task(task_id)
            .expect("read task")
            .expect("task retained");
        assert_eq!(
            task.task.request().file_name.as_deref(),
            Some("conflict (1).bin")
        );
        assert_eq!(task.temporary_path.as_deref(), Some(staged.as_path()));
        assert_eq!(
            std::fs::read(staged).expect("checkpoint bytes retained"),
            b"checkpoint bytes"
        );
        assert_eq!(
            std::fs::read(destination).expect("original destination retained"),
            b"external bytes"
        );
        assert_eq!(
            restored
                .inner
                .store
                .load_transfer_plan(task_id)
                .expect("load plan")
                .expect("plan retained"),
            plan
        );
        assert_eq!(restored.tasks().expect("list tasks").len(), 1);
    }

    #[test]
    fn auto_rename_retry_admission_failure_rolls_back_name_transfer_and_events() {
        for scheduled in [false, true] {
            let directory = tempfile::tempdir().expect("isolated atomic rename fixture");
            let database = directory.path().join("downloads.sqlite3");
            let staged = directory.path().join(".conflict.part");
            std::fs::write(&staged, b"prepared bytes").expect("retained prepared output");
            let engine = DownloadEngine::open(&database).expect("open atomic rename engine");
            engine
                .inner
                .store
                .set_queue_running("default", false)
                .expect("stop queue");
            let mut request = action_request(directory.path(), "conflict.bin");
            if scheduled {
                request.scheduled_at_unix_ms = Some(now_unix_ms() + 3_600_000);
            }
            let (task_id, plan) = seed_failed_action_task(
                &engine,
                request,
                DownloadFault {
                    code: DownloadFaultCode::DestinationExists,
                    message: "destination already exists".into(),
                    retryable: false,
                },
                Some(staged.clone()),
            );
            {
                let mut tasks = engine.inner.lock_tasks().expect("lock prepared fixture");
                let state = tasks.get_mut(&task_id).expect("failed task");
                state.transfer.prepared_size_bytes = Some(14);
                state.transfer.final_path = Some(directory.path().join("conflict.bin"));
                engine.inner.persist(state).expect("persist readiness");
            }
            engine
                .inner
                .store
                .set_queue_task_admission(task_id, false, true)
                .expect("paused admission");
            let before = engine
                .task(task_id)
                .expect("snapshot before")
                .expect("task before");
            let events = engine.take_event_receiver().expect("event stream");
            while events.try_recv().is_ok() {}
            let connection = rusqlite::Connection::open(&database).expect("synthetic database");
            connection.execute_batch(&format!(
                "CREATE TRIGGER reject_numbered_retry_admission BEFORE UPDATE ON download_queue_items \
                 WHEN NEW.task_id = '{}' AND NEW.is_eligible = 1 \
                 BEGIN SELECT RAISE(ABORT, 'injected admission failure'); END;", task_id.as_uuid()
            )).expect("reject actual admission write");
            for _ in 0..2 {
                let result = engine.handle_command(action_envelope(
                    task_id,
                    DownloadTaskAction::RetryWithAutoRename,
                ));
                assert!(
                    matches!(result.outcome, DownloadCommandOutcome::Rejected { fault }
                    if fault.code == DownloadFaultCode::Storage)
                );
                assert_eq!(
                    engine
                        .task(task_id)
                        .expect("unchanged live task")
                        .expect("same task"),
                    before,
                    "a rejected command must not publish a numbered name or path"
                );
                assert_eq!(
                    engine
                        .inner
                        .store
                        .load_task(task_id)
                        .expect("persisted task"),
                    Some(before.task.clone())
                );
                let transfer = engine
                    .inner
                    .store
                    .load_transfer_state(task_id)
                    .expect("persisted transfer");
                assert_eq!(transfer.final_path, before.final_path);
                assert_eq!(transfer.temporary_path.as_deref(), Some(staged.as_path()));
                assert_eq!(transfer.prepared_size_bytes, Some(14));
                assert_eq!(transfer.error, before.error);
                let admission: (bool, bool) = connection.query_row(
                    "SELECT is_eligible, is_user_paused FROM download_queue_items WHERE task_id=?1",
                    [task_id.as_uuid().to_string()], |row| Ok((row.get(0)?, row.get(1)?))
                ).expect("unchanged admission");
                assert_eq!(admission, (false, true));
                let rejection = events.try_recv().expect("command rejection event");
                assert!(
                    matches!(rejection.event, DownloadEvent::CommandRejected { request_id, .. }
                    if request_id == result.request_id)
                );
                assert!(
                    events.try_recv().is_err(),
                    "rejected transaction must not broadcast a task change"
                );
            }
            connection
                .execute_batch("DROP TRIGGER reject_numbered_retry_admission;")
                .expect("allow admission");
            let result = engine.handle_command(action_envelope(
                task_id,
                DownloadTaskAction::RetryWithAutoRename,
            ));
            assert!(
                matches!(result.outcome, DownloadCommandOutcome::Accepted { task_id: Some(id) } if id == task_id)
            );
            let renamed = engine
                .task(task_id)
                .expect("accepted task")
                .expect("same identity");
            assert_eq!(
                renamed.task.request().file_name.as_deref(),
                Some("conflict (1).bin")
            );
            assert_eq!(renamed.task.request().scheduled_at_unix_ms, None);
            assert!(renamed.queue_eligible);
            assert_eq!(
                engine
                    .inner
                    .store
                    .load_transfer_plan(task_id)
                    .expect("checkpoint"),
                Some(plan)
            );
            assert_eq!(
                std::fs::read(&staged).expect("retained output"),
                b"prepared bytes"
            );
            let restored = DownloadEngine::open(&database).expect("reopen committed retry");
            assert_eq!(
                restored
                    .task(task_id)
                    .expect("restored task")
                    .expect("durable task"),
                renamed
            );
        }
    }

    #[tokio::test]
    async fn preallocated_incomplete_conflict_never_publishes_as_prepared() {
        let directory = tempfile::tempdir().expect("create partial directory");
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("reserve offline loopback URL");
        let address = listener.local_addr().expect("read offline address");
        drop(listener);
        let engine = DownloadEngine::open_in_memory().expect("open partial engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop queue");
        let mut request = action_request(directory.path(), "partial.bin");
        request.source_url = format!("http://{address}/partial.bin");
        let original = engine
            .enqueue(request, RequestOrigin::Internal)
            .expect("enqueue partial");
        let staging = directory
            .path()
            .join(".partial.bin.checkpoint.nxtools-part");
        let mut partial = [0_u8; 16];
        partial[..4].copy_from_slice(b"part");
        std::fs::write(&staging, partial).expect("preallocate partial artifact");
        {
            let mut tasks = engine.inner.lock_tasks().expect("lock fixture");
            let state = tasks.get_mut(&original).expect("original state");
            state
                .task
                .transition_to(DownloadStatus::Probing, now_unix_ms())
                .expect("probe");
            state
                .task
                .transition_to(DownloadStatus::Failed, now_unix_ms())
                .expect("fail");
            state.transfer.error = Some(DownloadFault {
                code: DownloadFaultCode::DestinationExists,
                message: "pre-transfer conflict".into(),
                retryable: false,
            });
            state.transfer.temporary_path = Some(staging.clone());
            state.transfer.downloaded_bytes = 4;
            engine
                .inner
                .persist(state)
                .expect("persist incomplete state");
        }
        std::fs::write(directory.path().join("partial.bin"), b"external").expect("create conflict");
        engine
            .inner
            .store
            .set_queue_running("default", true)
            .expect("start queue");
        let result = engine.handle_command(action_envelope(
            original,
            DownloadTaskAction::RetryWithAutoRename,
        ));
        assert!(
            matches!(result.outcome, DownloadCommandOutcome::Accepted { task_id: Some(id) } if id == original)
        );
        let failed = engine
            .wait_for_terminal(original, Duration::from_secs(8))
            .await
            .expect("offline task terminates");
        assert_eq!(failed.task.status(), DownloadStatus::Failed);
        assert!(!directory.path().join("partial (1).bin").exists());
        assert_eq!(
            std::fs::read(staging).expect("partial artifact retained"),
            partial
        );
    }

    #[tokio::test]
    async fn completed_transfer_conflict_auto_renames_without_refetching_offline_source() {
        let directory = tempfile::tempdir().expect("create publication fixture");
        let database = directory.path().join("downloads.sqlite3");
        let destination = directory.path().join("fixture.bin");
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind loopback fixture");
        let address = listener.local_addr().expect("fixture address");
        listener.set_nonblocking(true).expect("bounded accept");
        let conflict_path = destination.clone();
        let server = std::thread::spawn(move || {
            let deadline = Instant::now() + Duration::from_secs(10);
            let mut transfer_gets = 0;
            loop {
                assert!(Instant::now() < deadline, "fixture transfer did not arrive");
                let (mut stream, _) = match listener.accept() {
                    Ok(connection) => connection,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        std::thread::sleep(Duration::from_millis(2));
                        continue;
                    }
                    Err(error) => panic!("accept fixture: {error}"),
                };
                stream
                    .set_nonblocking(false)
                    .expect("blocking fixture connection");
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .expect("bounded fixture read");
                let mut request = [0_u8; 4096];
                let count = stream.read(&mut request).expect("read fixture request");
                let request = String::from_utf8_lossy(&request[..count]);
                let is_head = request.starts_with("HEAD ");
                let is_range = request.to_ascii_lowercase().contains("\r\nrange:");
                if !is_head && !is_range {
                    transfer_gets += 1;
                    std::fs::write(&conflict_path, b"external owner")
                        .expect("create last-minute conflict");
                }
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 16\r\nConnection: close\r\n\r\n",
                    )
                    .expect("write response");
                if !is_head {
                    stream
                        .write_all(b"0123456789abcdef")
                        .expect("write fixture bytes");
                }
                if transfer_gets == 1 {
                    return transfer_gets;
                }
            }
        });
        let engine = DownloadEngine::open(&database).expect("open download engine");
        let mut request = action_request(directory.path(), "fixture.bin");
        request.source_url = format!("http://{address}/fixture.bin");
        let task_id = engine
            .submit(request, RequestOrigin::Internal)
            .expect("submit real transfer");
        let failed = engine
            .wait_for_terminal(task_id, Duration::from_secs(12))
            .await
            .expect("wait for publication conflict");
        assert_eq!(failed.task.status(), DownloadStatus::Failed);
        assert_eq!(
            failed.error.as_ref().map(|fault| fault.code),
            Some(DownloadFaultCode::DestinationExists)
        );
        let staged = failed.temporary_path.expect("preserved prepared artifact");
        assert_eq!(
            std::fs::read(&staged).expect("read prepared artifact"),
            b"0123456789abcdef"
        );
        assert_eq!(server.join().expect("join offline fixture"), 1);
        drop(engine);
        let engine = DownloadEngine::open(&database).expect("reopen failed transfer");
        let result = engine.handle_command(action_envelope(
            task_id,
            DownloadTaskAction::RetryWithAutoRename,
        ));
        assert!(
            matches!(result.outcome, DownloadCommandOutcome::Accepted { task_id: Some(id) } if id == task_id)
        );
        let completed = engine
            .wait_for_terminal(task_id, Duration::from_secs(5))
            .await
            .expect("complete without source");
        assert_eq!(completed.task.status(), DownloadStatus::Completed);
        assert_eq!(
            std::fs::read(directory.path().join("fixture (1).bin")).expect("read numbered output"),
            b"0123456789abcdef"
        );
        assert_eq!(
            std::fs::read(destination).expect("read original owner"),
            b"external owner"
        );
        assert!(!staged.exists(), "staged artifact consumed on commit");
    }

    #[test]
    fn background_batch_uses_the_original_correlated_command() {
        let directory = tempfile::tempdir().expect("create batch directory");
        let engine = DownloadEngine::open_in_memory().expect("open batch engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop queue");
        let task_id = engine
            .enqueue(
                action_request(directory.path(), "batch.bin"),
                RequestOrigin::Internal,
            )
            .expect("enqueue task");
        let receiver = engine
            .request_batch_background(
                vec![task_id],
                DownloadBatchAction::Remove,
                uuid::Uuid::new_v4(),
            )
            .expect("dispatch batch");
        assert!(
            matches!(receiver.recv_blocking().expect("receive outcome"), DownloadCommandOutcome::Batch { results } if results.len() == 1 && results[0].task_id == task_id && results[0].fault.is_none())
        );
        assert!(engine.task(task_id).expect("read removed task").is_none());
    }

    fn seed_completed_action_task(
        engine: &DownloadEngine,
        request: DownloadRequest,
        final_path: PathBuf,
    ) -> DownloadTaskId {
        let task_id = engine
            .enqueue(request, RequestOrigin::Internal)
            .expect("enqueue completed action fixture");
        std::fs::write(&final_path, b"old artifact").expect("write completed action artifact");
        {
            let mut tasks = engine
                .inner
                .lock_tasks()
                .expect("lock completed action tasks");
            let state = tasks.get_mut(&task_id).expect("completed action state");
            let base = state.task.created_at_unix_ms().saturating_add(1);
            state
                .task
                .transition_to(DownloadStatus::Probing, base)
                .expect("enter completed probing state");
            state
                .task
                .transition_to(DownloadStatus::Waiting, base.saturating_add(1))
                .expect("enter completed waiting state");
            state
                .task
                .transition_to(DownloadStatus::Downloading, base.saturating_add(2))
                .expect("enter completed downloading state");
            state
                .task
                .transition_to(DownloadStatus::Finalizing, base.saturating_add(3))
                .expect("enter completed finalizing state");
            state
                .task
                .transition_to(DownloadStatus::Verifying, base.saturating_add(4))
                .expect("enter completed verifying state");
            state
                .task
                .transition_to(DownloadStatus::Completed, base.saturating_add(5))
                .expect("enter completed state");
            state.transfer.final_path = Some(final_path);
            state.transfer.final_committed = true;
            engine
                .inner
                .persist(state)
                .expect("persist completed action");
        }
        task_id
    }

    #[test]
    fn batch_remove_without_event_consumer_returns_and_requests_snapshot() {
        let directory = tempfile::tempdir().expect("batch backpressure directory");
        let engine = DownloadEngine::open_in_memory().expect("batch backpressure engine");
        let events = engine
            .take_event_receiver()
            .expect("primary event receiver");
        let mut task_ids = Vec::new();
        for index in 0..300 {
            task_ids.push(
                engine
                    .enqueue(
                        action_request(directory.path(), &format!("batch-{index}.bin")),
                        RequestOrigin::Internal,
                    )
                    .expect("enqueue dormant batch task"),
            );
            while events.try_recv().is_ok() {}
        }
        let worker_engine = engine.clone();
        let (completed, completion) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            let result =
                worker_engine.handle_command(batch_envelope(task_ids, DownloadBatchAction::Remove));
            let _ = completed.send(result);
        });
        let result = completion.recv_timeout(Duration::from_secs(3));
        if result.is_err() {
            events.close();
        }
        worker
            .join()
            .expect("batch worker exits after receiver close");
        let result = result.expect("300 removals must return without a primary consumer");
        assert!(
            matches!(result.outcome, DownloadCommandOutcome::Batch { results }
            if results.len() == 300 && results.iter().all(|result| result.fault.is_none()))
        );
        assert!(engine.tasks().expect("authoritative tasks").is_empty());
        while events.try_recv().is_ok() {}
        engine
            .enqueue(
                action_request(directory.path(), "after-batch.bin"),
                RequestOrigin::Internal,
            )
            .expect("next state event after consumer frees capacity");
        let snapshot = events
            .try_recv()
            .expect("snapshot recovery precedes next state");
        assert!(matches!(
            snapshot.event,
            DownloadEvent::SnapshotRequired { .. }
        ));
        let next = events.try_recv().expect("next authoritative state event");
        assert_eq!(next.generation, snapshot.generation);
        assert!(next.sequence > snapshot.sequence);
        assert!(matches!(next.event, DownloadEvent::TaskChanged { .. }));
    }

    #[test]
    fn retry_preserves_identity_checkpoint_and_coalesces_after_engine_restart() {
        let directory = tempfile::tempdir().expect("create retry action directory");
        let database = directory.path().join("downloads.sqlite3");
        let request = action_request(&directory.path().join("artifacts"), "retry.bin");
        let task_id;
        let plan;
        {
            let engine = DownloadEngine::open(&database).expect("open retry action engine");
            engine
                .inner
                .store
                .set_queue_running("default", false)
                .expect("stop retry fixture queue");
            (task_id, plan) = seed_failed_action_task(
                &engine,
                request,
                DownloadFault {
                    code: DownloadFaultCode::Network,
                    message: "temporary network failure".into(),
                    retryable: true,
                },
                None,
            );
        }

        let engine = DownloadEngine::open(&database).expect("reopen retry action engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop reopened retry fixture queue");
        let restored = engine
            .task(task_id)
            .expect("read restored retry task")
            .expect("restored retry task exists");
        assert_eq!(
            restored.error.as_ref().map(|fault| fault.retryable),
            Some(true)
        );
        assert_eq!(restored.downloaded_bytes, 4);

        for _ in 0..2 {
            let result = engine.handle_command(action_envelope(task_id, DownloadTaskAction::Retry));
            assert!(matches!(
                result.outcome,
                DownloadCommandOutcome::Accepted { task_id: Some(id) } if id == task_id
            ));
        }
        let restored_plan = engine
            .inner
            .store
            .load_transfer_plan(task_id)
            .expect("load retained retry plan")
            .expect("retry plan remains durable");
        assert_eq!(
            restored_plan.segments()[0].downloaded_bytes(),
            plan.segments()[0].downloaded_bytes()
        );
        assert_eq!(engine.tasks().expect("list coalesced retry tasks").len(), 1);
    }

    #[test]
    fn retry_rejects_non_retryable_fault_without_touching_checkpoint() {
        let directory = tempfile::tempdir().expect("create non-retryable action directory");
        let engine = DownloadEngine::open_in_memory().expect("open non-retryable action engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop non-retryable fixture queue");
        let (task_id, plan) = seed_failed_action_task(
            &engine,
            action_request(directory.path(), "protocol.bin"),
            DownloadFault {
                code: DownloadFaultCode::Protocol,
                message: "resource changed".into(),
                retryable: false,
            },
            None,
        );
        let result = engine.handle_command(action_envelope(task_id, DownloadTaskAction::Retry));
        let DownloadCommandOutcome::Rejected { fault } = result.outcome else {
            panic!("non-retryable retry must be rejected");
        };
        assert_eq!(fault.code, DownloadFaultCode::InvalidState);
        assert!(!fault.retryable);
        let alias = engine.handle_command(action_envelope(task_id, DownloadTaskAction::Resume));
        assert!(matches!(
            alias.outcome,
            DownloadCommandOutcome::Rejected { .. }
        ));
        assert_eq!(
            engine
                .inner
                .store
                .load_transfer_plan(task_id)
                .expect("load non-retryable checkpoint")
                .expect("checkpoint remains")
                .segments()[0]
                .downloaded_bytes(),
            plan.segments()[0].downloaded_bytes()
        );
    }

    #[test]
    fn redownload_rejects_non_terminal_tasks_before_acceptance() {
        let directory = tempfile::tempdir().expect("create non-terminal redownload directory");
        let engine = DownloadEngine::open_in_memory().expect("open non-terminal redownload engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop non-terminal redownload fixture queue");
        let task_id = engine
            .enqueue(
                action_request(directory.path(), "created.bin"),
                RequestOrigin::Internal,
            )
            .expect("enqueue created redownload task");

        let result =
            engine.handle_command(action_envelope(task_id, DownloadTaskAction::Redownload));
        assert!(matches!(
            result.outcome,
            DownloadCommandOutcome::Rejected { .. }
        ));
        assert!(engine.task(task_id).expect("read created task").is_some());
        assert_eq!(engine.tasks().expect("list created task").len(), 1);
    }

    #[test]
    fn redownload_creates_numbered_replacement_after_acceptance() {
        let directory = tempfile::tempdir().expect("create redownload directory");
        let engine = DownloadEngine::open_in_memory().expect("open redownload engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop redownload fixture queue");
        engine
            .update_download_organization_policy(&nxtools_core::DownloadSettings {
                duplicate_strategy: nxtools_core::DownloadDuplicateStrategySettings::AddNumbered,
                ..nxtools_core::DownloadSettings::default()
            })
            .expect("apply numbered duplicate strategy");
        let final_path = directory.path().join("artifact.bin");
        let old_id = seed_completed_action_task(
            &engine,
            action_request(directory.path(), "artifact.bin"),
            final_path.clone(),
        );

        let result = engine.handle_command(action_envelope(old_id, DownloadTaskAction::Redownload));
        let DownloadCommandOutcome::Accepted {
            task_id: Some(new_id),
        } = result.outcome
        else {
            panic!("redownload must return a replacement task");
        };
        assert_ne!(new_id, old_id);
        assert!(
            engine
                .task(old_id)
                .expect("read removed old task")
                .is_none()
        );
        let replacement = engine
            .task(new_id)
            .expect("read replacement task")
            .expect("replacement task exists");
        assert_eq!(
            replacement.task.request().file_name.as_deref(),
            Some("artifact (1).bin")
        );
        assert!(
            final_path.exists(),
            "redownload keeps the completed old artifact"
        );
    }

    #[test]
    fn redownload_acceptance_failure_keeps_the_old_task_and_artifact() {
        let directory = tempfile::tempdir().expect("create failed redownload directory");
        let engine = DownloadEngine::open_in_memory().expect("open failed redownload engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop failed redownload fixture queue");
        engine
            .update_download_organization_policy(&nxtools_core::DownloadSettings {
                duplicate_strategy: nxtools_core::DownloadDuplicateStrategySettings::Reject,
                ..nxtools_core::DownloadSettings::default()
            })
            .expect("apply rejecting duplicate strategy");
        let final_path = directory.path().join("occupied.bin");
        let old_id = seed_completed_action_task(
            &engine,
            action_request(directory.path(), "occupied.bin"),
            final_path.clone(),
        );

        let result = engine.handle_command(action_envelope(old_id, DownloadTaskAction::Redownload));
        assert!(matches!(
            result.outcome,
            DownloadCommandOutcome::Rejected { .. }
        ));
        assert!(
            engine
                .task(old_id)
                .expect("read retained old task")
                .is_some()
        );
        assert!(final_path.exists());
        assert_eq!(engine.tasks().expect("list retained task").len(), 1);
    }

    #[test]
    fn redownload_batch_reports_partial_success_and_returns_new_task_id() {
        let directory = tempfile::tempdir().expect("create batch redownload directory");
        let engine = DownloadEngine::open_in_memory().expect("open batch redownload engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop batch redownload fixture queue");
        let old_id = seed_completed_action_task(
            &engine,
            action_request(directory.path(), "batch.bin"),
            directory.path().join("batch.bin"),
        );
        let missing_id = DownloadTaskId::new();
        let result = engine.handle_command(batch_envelope(
            vec![old_id, missing_id],
            DownloadBatchAction::Redownload,
        ));
        let DownloadCommandOutcome::Batch { results } = result.outcome else {
            panic!("redownload batch outcome expected");
        };
        assert_eq!(results.len(), 2);
        assert!(results[0].fault.is_none());
        assert_ne!(results[0].task_id, old_id);
        assert!(results[1].fault.is_some());
        assert_eq!(results[1].task_id, missing_id);
    }

    #[test]
    fn redownload_artifact_cleanup_failure_retains_retryable_operation() {
        let directory = tempfile::tempdir().expect("create retryable redownload directory");
        let engine = DownloadEngine::open_in_memory().expect("open retryable redownload engine");
        engine
            .inner
            .store
            .set_queue_running("default", false)
            .expect("stop retryable redownload fixture queue");
        let temporary_path = directory.path().join("broken.part");
        std::fs::create_dir_all(&temporary_path).expect("create obstructing temporary artifact");
        let (old_id, _) = seed_failed_action_task(
            &engine,
            action_request(directory.path(), "rollback.bin"),
            DownloadFault {
                code: DownloadFaultCode::Network,
                message: "temporary failure".into(),
                retryable: true,
            },
            Some(temporary_path.clone()),
        );

        let result = engine.handle_command(action_envelope(old_id, DownloadTaskAction::Redownload));
        assert!(matches!(
            result.outcome,
            DownloadCommandOutcome::Rejected { .. }
        ));
        assert!(
            engine
                .task(old_id)
                .expect("read retained retryable task")
                .is_some()
        );
        assert!(temporary_path.is_dir());
        assert_eq!(engine.tasks().expect("list retryable tasks").len(), 2);
        assert_eq!(
            engine
                .inner
                .store
                .load_redownload_operations()
                .expect("load pending redownload operation")
                .len(),
            1
        );
        let replacement_id = engine
            .tasks()
            .expect("list pending replacement")
            .into_iter()
            .find(|task| task.task.id() != old_id)
            .expect("pending replacement task")
            .task
            .id();
        std::fs::remove_dir_all(&temporary_path).expect("remove cleanup obstruction");
        let replay = engine.handle_command(action_envelope(old_id, DownloadTaskAction::Redownload));
        assert!(matches!(
            replay.outcome,
            DownloadCommandOutcome::Accepted {
                task_id: Some(task_id)
            } if task_id == replacement_id
        ));
        assert_eq!(engine.tasks().expect("list replayed tasks").len(), 1);
        assert!(
            engine
                .inner
                .store
                .load_redownload_operations()
                .expect("load completed redownload operation")
                .is_empty()
        );
    }

    #[test]
    fn pending_redownload_operation_reconciles_after_restart_without_duplicate_tasks() {
        let directory = tempfile::tempdir().expect("create restart redownload directory");
        let database = directory.path().join("downloads.sqlite3");
        let final_path = directory.path().join("restart.bin");
        let (old_id, replacement_id) = {
            let engine = DownloadEngine::open(&database).expect("open restart redownload engine");
            engine
                .inner
                .store
                .set_queue_running("default", false)
                .expect("stop restart redownload queue");
            let old_id = seed_completed_action_task(
                &engine,
                action_request(directory.path(), "restart.bin"),
                final_path.clone(),
            );
            let old = engine
                .task(old_id)
                .expect("read restart redownload task")
                .expect("restart redownload task exists");
            let replacement_id = engine
                .create_redownload_replacement(&old, uuid::Uuid::new_v4())
                .expect("persist replacement before simulated crash");
            assert_eq!(engine.tasks().expect("list pre-crash tasks").len(), 2);
            (old_id, replacement_id)
        };

        let reopened = DownloadEngine::open(&database).expect("reopen restart redownload engine");
        assert!(
            reopened
                .task(old_id)
                .expect("read reconciled old task")
                .is_none()
        );
        assert!(
            reopened
                .task(replacement_id)
                .expect("read reconciled replacement")
                .is_some()
        );
        assert_eq!(reopened.tasks().expect("list reconciled tasks").len(), 1);
        assert!(final_path.exists(), "redownload preserves the old artifact");
        assert!(
            reopened
                .inner
                .store
                .load_redownload_operations()
                .expect("load reconciled markers")
                .is_empty()
        );
    }

    #[test]
    fn media_cleanup_preflights_both_owned_targets_before_deleting_either() {
        let directory = tempfile::tempdir().expect("create media cleanup directory");
        let request = action_request(directory.path(), "partial.bin");
        let hls_task_id = DownloadTaskId::new();
        let hls_working = hls_working_directory(&request, hls_task_id);
        let hls_temporary = directory.path().join("hls-merge.part");
        std::fs::create_dir_all(&hls_working).expect("create HLS working directory");
        std::fs::write(hls_working.join("segment"), b"segment").expect("write HLS segment");
        std::fs::create_dir_all(&hls_temporary).expect("obstruct HLS temporary artifact");
        assert!(
            remove_hls_owned_artifacts(
                &request,
                hls_task_id,
                Some(&hls_temporary),
                uuid::Uuid::new_v4(),
                "test",
                Instant::now(),
            )
            .is_err()
        );
        assert!(hls_working.exists());
        assert!(hls_temporary.is_dir());

        let dash_task_id = DownloadTaskId::new();
        let dash_working = dash_working_directory(&request, dash_task_id);
        let dash_temporary = directory.path().join("dash-merge.part");
        std::fs::create_dir_all(&dash_working).expect("create DASH working directory");
        std::fs::write(dash_working.join("fragment"), b"fragment").expect("write DASH fragment");
        std::fs::create_dir_all(&dash_temporary).expect("obstruct DASH temporary artifact");
        assert!(
            remove_dash_owned_artifacts(
                &request,
                dash_task_id,
                Some(&dash_temporary),
                uuid::Uuid::new_v4(),
                "test",
                Instant::now(),
            )
            .is_err()
        );
        assert!(dash_working.exists());
        assert!(dash_temporary.is_dir());
    }

    const SNAPSHOT_BENCHMARK_ITERATIONS: usize = 7;

    fn snapshot_benchmark_request(directory: &Path, index: usize) -> DownloadRequest {
        DownloadRequest {
            source_url: format!("https://snapshot.example.test/files/{index}.bin"),
            save_directory: directory.to_owned(),
            save_directory_explicit: false,
            file_name: Some(format!("snapshot-{index:05}.bin")),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            source_page: None,
            certificate_policy: Default::default(),
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: Some(1024),
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
        }
    }

    fn seed_snapshot_benchmark_engine(task_count: usize) -> (DownloadEngine, Vec<DownloadTaskId>) {
        let directory = tempfile::tempdir().expect("create snapshot benchmark directory");
        let engine = DownloadEngine::open_in_memory().expect("open snapshot benchmark engine");
        let mut task_ids = Vec::with_capacity(task_count);
        let mut tasks = engine
            .inner
            .lock_tasks()
            .expect("lock snapshot benchmark tasks");
        for index in 0..task_count {
            let task = DownloadTask::new(
                snapshot_benchmark_request(directory.path(), index),
                RequestOrigin::Internal,
                1_000 + index as i64,
            );
            let task_id = task.id();
            engine
                .inner
                .store
                .save_task(&task)
                .expect("save snapshot benchmark task");
            tasks.insert(
                task_id,
                EngineTaskState {
                    task,
                    transfer: StoredTransferState::default(),
                    control: None,
                    last_progress_event: None,
                    transfer_rate: None,
                    paused_url_replacement: None,
                    accepted_revision: 0,
                    completion_batch_id: None,
                },
            );
            task_ids.push(task_id);
        }
        drop(tasks);
        engine
            .inner
            .store
            .replace_queue_order("default", &task_ids)
            .expect("save snapshot benchmark queue order");
        (engine, task_ids)
    }

    fn benchmark_snapshot(engine: &DownloadEngine, compact: bool) -> DownloadSnapshot {
        let result = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Internal,
            command: DownloadCommand::Snapshot {
                query: DownloadQuery {
                    compact,
                    ..DownloadQuery::default()
                },
            },
        });
        let DownloadCommandOutcome::Snapshot(snapshot) = result.outcome else {
            panic!("snapshot benchmark command must return a snapshot");
        };
        snapshot
    }

    fn benchmark_detail(engine: &DownloadEngine, task_id: DownloadTaskId) -> DownloadTaskDetail {
        let result = engine.handle_command(DownloadCommandEnvelope {
            protocol_version: DOWNLOAD_COMMAND_PROTOCOL_VERSION,
            request_id: uuid::Uuid::new_v4(),
            origin: RequestOrigin::Internal,
            command: DownloadCommand::TaskDetail { task_id },
        });
        let DownloadCommandOutcome::TaskDetail(detail) = result.outcome else {
            panic!("task detail benchmark command must return detail");
        };
        *detail
    }

    fn benchmark_median_p95(samples: &mut [u128]) -> (u128, u128) {
        samples.sort_unstable();
        let median = samples[samples.len() / 2];
        let p95_index = ((samples.len() * 95).div_ceil(100)).saturating_sub(1);
        (median, samples[p95_index])
    }

    fn measure_snapshot_fixture(task_count: usize, compact: bool) {
        let (engine, task_ids) = seed_snapshot_benchmark_engine(task_count);
        let warm_snapshot = benchmark_snapshot(&engine, compact);
        if compact {
            assert_eq!(warm_snapshot.task_summaries.len(), task_count);
            assert!(warm_snapshot.tasks.is_empty());
            assert!(warm_snapshot.task_details.is_empty());
        } else {
            assert_eq!(warm_snapshot.tasks.len(), task_count);
            assert_eq!(warm_snapshot.task_details.len(), task_count);
        }
        let warm_detail = benchmark_detail(&engine, task_ids[task_count / 2]);
        assert_eq!(warm_detail.task.task.id(), task_ids[task_count / 2]);

        let mut command_samples = Vec::with_capacity(SNAPSHOT_BENCHMARK_ITERATIONS);
        let mut serialization_samples = Vec::with_capacity(SNAPSHOT_BENCHMARK_ITERATIONS);
        let mut snapshot_json_bytes = 0_usize;
        for _ in 0..SNAPSHOT_BENCHMARK_ITERATIONS {
            let started = Instant::now();
            let snapshot = benchmark_snapshot(&engine, compact);
            command_samples.push(started.elapsed().as_micros());
            let started = Instant::now();
            snapshot_json_bytes = serde_json::to_vec(&snapshot)
                .expect("serialize snapshot benchmark payload")
                .len();
            serialization_samples.push(started.elapsed().as_micros());
        }

        let mut detail_command_samples = Vec::with_capacity(SNAPSHOT_BENCHMARK_ITERATIONS);
        let mut detail_serialization_samples = Vec::with_capacity(SNAPSHOT_BENCHMARK_ITERATIONS);
        let mut detail_json_bytes = 0_usize;
        for _ in 0..SNAPSHOT_BENCHMARK_ITERATIONS {
            let started = Instant::now();
            let detail = benchmark_detail(&engine, task_ids[task_count / 2]);
            detail_command_samples.push(started.elapsed().as_micros());
            let started = Instant::now();
            detail_json_bytes = serde_json::to_vec(&detail)
                .expect("serialize detail benchmark payload")
                .len();
            detail_serialization_samples.push(started.elapsed().as_micros());
        }

        let mut sqlite_samples = Vec::with_capacity(SNAPSHOT_BENCHMARK_ITERATIONS);
        for _ in 0..SNAPSHOT_BENCHMARK_ITERATIONS {
            let started = Instant::now();
            let tasks = engine
                .inner
                .store
                .load_tasks()
                .expect("load SQLite benchmark tasks");
            let queue = engine
                .inner
                .store
                .load_queue_order("default")
                .expect("load SQLite benchmark queue order");
            assert_eq!(tasks.len(), task_count);
            assert_eq!(queue.len(), task_count);
            sqlite_samples.push(started.elapsed().as_micros());
        }

        let (command_median, command_p95) = benchmark_median_p95(&mut command_samples);
        let (serialization_median, serialization_p95) =
            benchmark_median_p95(&mut serialization_samples);
        let (detail_command_median, detail_command_p95) =
            benchmark_median_p95(&mut detail_command_samples);
        let (detail_serialization_median, detail_serialization_p95) =
            benchmark_median_p95(&mut detail_serialization_samples);
        let (sqlite_median, sqlite_p95) = benchmark_median_p95(&mut sqlite_samples);
        let projection = if compact { "compact" } else { "legacy" };
        println!(
            "snapshot_benchmark projection={projection} task_count={task_count} snapshot_json_bytes={snapshot_json_bytes} snapshot_command_us_median={command_median} snapshot_command_us_p95={command_p95} snapshot_serialize_us_median={serialization_median} snapshot_serialize_us_p95={serialization_p95} detail_json_bytes={detail_json_bytes} detail_command_us_median={detail_command_median} detail_command_us_p95={detail_command_p95} detail_serialize_us_median={detail_serialization_median} detail_serialize_us_p95={detail_serialization_p95} sqlite_tasks_queue_us_median={sqlite_median} sqlite_tasks_queue_us_p95={sqlite_p95}"
        );
    }

    #[test]
    fn snapshot_detail_baseline_uses_the_current_full_projection_at_1000_tasks() {
        measure_snapshot_fixture(1_000, false);
    }

    #[test]
    fn snapshot_detail_baseline_uses_the_current_full_projection_at_10000_tasks() {
        measure_snapshot_fixture(10_000, false);
    }
    #[test]
    fn snapshot_detail_compact_projection_at_1000_tasks() {
        measure_snapshot_fixture(1_000, true);
    }

    #[test]
    fn snapshot_detail_compact_projection_at_10000_tasks() {
        measure_snapshot_fixture(10_000, true);
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

#[cfg(test)]
#[test]
fn transfer_retry_policy_never_schedules_a_fourth_attempt() {
    let transient = HttpDownloadError::Status(503);

    assert!(should_retry_transfer(1, &transient));
    assert!(should_retry_transfer(2, &transient));
    assert!(!should_retry_transfer(3, &transient));
    assert!(!should_retry_transfer(1, &HttpDownloadError::Status(404)));
}

#[test]
fn hls_finalization_gate_observes_a_stop_directive_after_the_last_worker() {
    let control = DownloadTransferControl::new();
    assert_eq!(hls_stop_directive(&control), None);
    control.cancel();
    assert_eq!(
        hls_stop_directive(&control),
        Some(TransferDirective::Cancel)
    );
}

#[cfg(test)]
mod b5_drain_tests {
    use super::*;

    #[test]
    fn b5_shutdown_joins_remaining_workers_and_reports_worker_panic() {
        let engine = DownloadEngine::open_in_memory().expect("engine");
        let runtime = download_runtime().expect("original runtime");
        let (finished, observed) = std::sync::mpsc::sync_channel(1);
        let mut workers = engine.inner.workers.lock().expect("owned workers");
        workers.spawn_on(
            async { panic!("injected owned worker failure") },
            runtime.handle(),
        );
        workers.spawn_on(
            async move {
                tokio::time::sleep(Duration::from_millis(30)).await;
                finished.send(()).expect("worker exit marker");
            },
            runtime.handle(),
        );
        drop(workers);
        let result = engine.shutdown_and_wait();
        observed
            .try_recv()
            .expect("all owned workers exit before drain returns");
        assert!(
            result.is_err(),
            "worker panic must not be reported as successful resource drain"
        );
    }
}
