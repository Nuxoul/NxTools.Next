use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, Days, Local, NaiveDateTime, NaiveTime, TimeZone, Timelike};
use nxtools_core::{DownloadSettings, NetworkSettings};
use nxtools_network::proxy_mode_name;
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, error, info, warn};

use crate::capacity::{StorageCapacityProbe, SystemStorageCapacityProbe};
use crate::http::{
    HlsKeyFetchOutcome, HlsSegmentDownloadOutcome, PreparedDownload, PreparedDownloadOutcome,
    TransferDirective, decrypt_hls_aes128_segment, hls_sequence_iv, is_disk_full_error,
    is_file_in_use_error,
};
use crate::storage::{BrowserHandoffSave, StoredTransferState};
use crate::{
    ArtifactAllocation, BrowserBatchSubmissionResult, BrowserBlob, BrowserDownloadSubmission,
    DOWNLOAD_COMMAND_PROTOCOL_VERSION, DownloadAggregateMetrics, DownloadBatchAction,
    DownloadBatchItemResult, DownloadCommand, DownloadCommandEnvelope, DownloadCommandOutcome,
    DownloadCommandResult, DownloadDailySchedule, DownloadEvent, DownloadEventEnvelope,
    DownloadEventSequencer, DownloadFault, DownloadFaultCode, DownloadHttpPolicy,
    DownloadHttpPolicyError, DownloadOrganizationPolicy, DownloadOrganizationPolicyError,
    DownloadProbe, DownloadProgress, DownloadQuery, DownloadQueue, DownloadQueueAction,
    DownloadQueuePlacement, DownloadRateLimitError, DownloadRateLimiter, DownloadRequest,
    DownloadRequestError, DownloadRequestMedia, DownloadSchedule, DownloadSegment,
    DownloadSegmentError, DownloadSegmentProgress, DownloadSegmentState,
    DownloadSegmentTransferControl, DownloadSegmentTransferOutcome, DownloadSnapshot,
    DownloadStatus, DownloadStorageCapacityError, DownloadStorageError, DownloadStorageRequirement,
    DownloadSubmitMode, DownloadTask, DownloadTaskAction, DownloadTaskDetail, DownloadTaskError,
    DownloadTaskId, DownloadTransferContext, DownloadTransferControl, DownloadWeekday,
    FfmpegInstallControl, FfmpegInstallOperation, HttpDownloadClient, HttpDownloadError,
    MAX_BROWSER_BATCH_ITEMS, NetworkPolicy, NetworkPolicyError, RequestOrigin, SegmentedArtifact,
    SqliteDownloadStore, TransferPlan, TransferPlanError, TransferProtocol,
};

static DOWNLOAD_RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();

#[derive(Clone)]
pub struct DownloadEngine {
    inner: Arc<DownloadEngineInner>,
}

struct DownloadEngineInner {
    http_template: HttpDownloadClient,
    network_policy: NetworkPolicy,
    download_http_policy: DownloadHttpPolicy,
    download_organization_policy: DownloadOrganizationPolicy,
    ffmpeg_executable: Mutex<Option<PathBuf>>,
    ffmpeg_install_active: AtomicBool,
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
    browser_dialog_sender: async_channel::Sender<BrowserDownloadDialogRequest>,
    browser_dialog_receiver: Mutex<Option<async_channel::Receiver<BrowserDownloadDialogRequest>>>,
    browser_dialog_handoffs: Mutex<VecDeque<uuid::Uuid>>,
    browser_dialog_available: AtomicBool,
    browser_blob_staging_root: Option<PathBuf>,
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
}

struct BrowserBlobImportRequest {
    handoff_id: uuid::Uuid,
    blob: BrowserBlob,
    save_directory: PathBuf,
    file_name: String,
    category_id: Option<String>,
    save_directory_explicit: bool,
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

/// Progress events are coalesced to at most 10 Hz per task (the plan's
/// 5-10 Hz band); terminal states always publish immediately.
const PROGRESS_EVENT_MIN_INTERVAL: Duration = Duration::from_millis(100);
const ADAPTIVE_SAMPLE_COUNT: usize = 5;
const ADAPTIVE_STABILITY_PERCENT: u64 = 15;
const ADAPTIVE_MIN_PROPORTIONAL_GAIN_PERCENT: u64 = 80;
/// A transfer whose average speed stays below this share of the baseline for
/// a full stability window shrinks back one segment at a time, down to the
/// initial segment count.
const ADAPTIVE_SHRINK_SPEED_SHARE_PERCENT: u64 = 60;
const MIB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdaptiveConnectionDecision {
    Hold,
    Grow,
    Shrink,
    DisabledAfterRetry,
    DisabledForLowBenefit,
}

#[derive(Debug)]
struct AdaptiveSegmentController {
    max_segments: u16,
    samples: VecDeque<u64>,
    pending_evaluation: Option<(u64, u16, u16)>,
    disabled: bool,
    /// Baseline speed recorded right before the last growth batch; shrink
    /// compares the current average against it.
    growth_baseline: Option<(u64, u16)>,
}

impl AdaptiveSegmentController {
    fn new(max_segments: u16, enabled: bool) -> Self {
        Self {
            max_segments,
            samples: VecDeque::with_capacity(ADAPTIVE_SAMPLE_COUNT),
            pending_evaluation: None,
            disabled: !enabled || max_segments <= 1,
            growth_baseline: None,
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
        self.growth_baseline = Some((average, baseline_segments));
        self.pending_evaluation = Some((average, baseline_segments, baseline_segments));
        self.samples.clear();
        AdaptiveConnectionDecision::Grow
    }

    /// Shrink fires when the transfer has grown beyond its initial segment
    /// count and a full stability window now averages below the configured
    /// share of the pre-growth baseline: the extra segments stopped paying
    /// for themselves, so shed one.
    fn observe_shrink(
        &mut self,
        bytes_per_second: u64,
        segment_count: u16,
        initial_segment_count: u16,
    ) -> AdaptiveConnectionDecision {
        if self.disabled || segment_count <= initial_segment_count || segment_count <= 1 {
            return AdaptiveConnectionDecision::Hold;
        }
        let Some((baseline_speed, baseline_segments)) = self.growth_baseline else {
            return AdaptiveConnectionDecision::Hold;
        };
        if baseline_speed == 0 {
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
        self.samples.clear();
        let baseline_segments = u128::from(baseline_segments.max(1));
        let grown_segments = u128::from(segment_count.max(1));
        // The baseline earned by baseline_segments is the reference; the
        // per-segment fair share decides whether the extra segments still
        // carry meaningful traffic.
        let expected = u128::from(baseline_speed)
            .saturating_mul(grown_segments)
            .saturating_div(baseline_segments)
            .saturating_mul(u128::from(ADAPTIVE_SHRINK_SPEED_SHARE_PERCENT))
            / 100;
        if u128::from(average) < expected {
            AdaptiveConnectionDecision::Shrink
        } else {
            AdaptiveConnectionDecision::Hold
        }
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

    fn record_completed_growth(&mut self, grown_segments: u16) {
        if let Some((baseline_speed, baseline_segments)) = self.growth_baseline
            && grown_segments > baseline_segments
        {
            self.growth_baseline = Some((baseline_speed, grown_segments.min(self.max_segments)));
        }
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
        let (browser_dialog_sender, browser_dialog_receiver) = async_channel::bounded(16);
        let engine = Self {
            inner: Arc::new(DownloadEngineInner {
                http_template: http,
                network_policy,
                download_http_policy,
                download_organization_policy: DownloadOrganizationPolicy::from_download_settings(
                    &DownloadSettings::default(),
                )?,
                ffmpeg_executable: Mutex::new(ffmpeg_executable),
                ffmpeg_install_active: AtomicBool::new(false),
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
                browser_dialog_sender,
                browser_dialog_receiver: Mutex::new(Some(browser_dialog_receiver)),
                browser_dialog_handoffs: Mutex::new(VecDeque::new()),
                browser_dialog_available: AtomicBool::new(false),
                browser_blob_staging_root,
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

    /// Attaches the one in-process GPUI consumer for browser confirmation-window requests.
    /// This channel is intentionally separate from serializable download events because the
    /// request can contain credentials and request headers.
    pub fn take_browser_dialog_receiver(
        &self,
    ) -> Result<async_channel::Receiver<BrowserDownloadDialogRequest>, DownloadEngineError> {
        let receiver = self
            .inner
            .browser_dialog_receiver
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .take()
            .ok_or(DownloadEngineError::BrowserDialogSubscriberAlreadyTaken)?;
        self.inner
            .browser_dialog_available
            .store(true, Ordering::Release);
        Ok(receiver)
    }

    pub fn global_speed_limit(&self) -> Option<u64> {
        self.inner.global_rate_limiter.bytes_per_second()
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
        let runtime = download_runtime()?;
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
        runtime.spawn(async move {
            let result = crate::ffmpeg_install::run_ffmpeg_install(
                correlation_id,
                install_root,
                preserve_executable,
                control,
                progress_sender,
                network,
            )
            .await;
            inner.ffmpeg_install_active.store(false, Ordering::Release);
            let _ = completion_sender.send(result).await;
        });
        Ok(FfmpegInstallOperation {
            correlation_id,
            network_proxy_mode,
            network_proxy_source,
            control: public_control,
            progress,
            completion,
        })
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
            DownloadCommand::Submit { request, mode } => self
                .submit_with_mode_correlated(request, envelope.origin, mode, request_id)
                .map(|task_id| DownloadCommandOutcome::Accepted {
                    task_id: Some(task_id),
                }),
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
        mut request: DownloadRequest,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadCommandOutcome, DownloadEngineError> {
        let started = Instant::now();
        let policy_result = self
            .inner
            .download_http_policy
            .apply_to_request(&mut request);
        let certificate_policy = request.certificate_policy;
        let client = policy_result
            .map_err(DownloadEngineError::from)
            .and_then(|()| self.inner.http_for_operation(certificate_policy));
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
            "download resource probe started"
        );
        let result = (|| {
            request.validate()?;
            let client = client?;
            let (sender, receiver) = std::sync::mpsc::sync_channel(1);
            download_runtime()?.spawn(async move {
                let _ = sender.send(client.probe(&request).await);
            });
            receiver
                .recv()
                .map_err(|error| DownloadEngineError::Runtime(error.to_string()))?
                .map_err(DownloadEngineError::from)
        })();
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
                info!(
                    event = "download.probe.completed",
                    %correlation_id,
                    proxy_mode,
                    proxy_source,
                    certificate_policy = certificate_policy_name(certificate_policy),
                    media_kind,
                    variant_count,
                    elapsed_ms = started.elapsed().as_millis(),
                    "download resource probe completed"
                );
                Ok(DownloadCommandOutcome::Probe(probe))
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
    ) -> Result<(), DownloadEngineError> {
        match action {
            DownloadTaskAction::Pause => self.pause(task_id),
            DownloadTaskAction::Resume => self.resume_correlated(task_id, correlation_id),
            DownloadTaskAction::Cancel => self.cancel_correlated(task_id, correlation_id),
            DownloadTaskAction::SetSpeedLimit { bytes_per_second } => {
                self.set_task_speed_limit_correlated(task_id, bytes_per_second, correlation_id)
            }
            DownloadTaskAction::Remove { delete_artifact } => {
                self.remove_correlated(task_id, delete_artifact, correlation_id)
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
        mut request: DownloadRequest,
        origin: RequestOrigin,
        mode: DownloadSubmitMode,
        correlation_id: uuid::Uuid,
    ) -> Result<DownloadTaskId, DownloadEngineError> {
        self.inner
            .download_http_policy
            .apply_to_request(&mut request)?;
        request.validate()?;
        self.resolve_missing_file_name_before_acceptance(&mut request, correlation_id)?;
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        let mut tasks = self.inner.lock_tasks()?;
        apply_organization_to_request(&self.inner, &mut request, &tasks, correlation_id)?;
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
        self.inner.store.save_task_with_transfer_and_queue(
            &task,
            &transfer,
            "default",
            mode == DownloadSubmitMode::StartNow && !delayed_until_due,
        )?;
        tasks.insert(
            task_id,
            EngineTaskState {
                task,
                transfer,
                control,
                last_progress_event: None,
                transfer_rate: None,
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
        Ok(task_id)
    }

    fn resolve_missing_file_name_before_acceptance(
        &self,
        request: &mut DownloadRequest,
        correlation_id: uuid::Uuid,
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
            download_runtime()?.spawn(async move {
                let _ = sender.send(client.probe(&probe_request).await);
            });
            receiver
                .recv()
                .map_err(|error| DownloadEngineError::Runtime(error.to_string()))?
                .map_err(DownloadEngineError::from)
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

    fn submit_browser_correlated(
        &self,
        handoff_id: uuid::Uuid,
        mut request: DownloadRequest,
        mode: DownloadSubmitMode,
        correlation_id: uuid::Uuid,
    ) -> Result<(DownloadTaskId, bool), DownloadEngineError> {
        if let Some(task_id) = self.inner.store.browser_handoff_task(handoff_id)? {
            info!(
                event = "download.browser_handoff.duplicate_accepted",
                %correlation_id,
                %handoff_id,
                ?task_id,
                "duplicate browser handoff resolved before request replay"
            );
            return Ok((task_id, true));
        }
        self.inner
            .download_http_policy
            .apply_to_request(&mut request)?;
        request.validate()?;
        self.resolve_missing_file_name_before_acceptance(&mut request, correlation_id)?;
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if let Some(task_id) = self.inner.store.browser_handoff_task(handoff_id)? {
            info!(
                event = "download.browser_handoff.duplicate_accepted",
                %correlation_id,
                %handoff_id,
                ?task_id,
                "duplicate browser handoff resolved to the existing task"
            );
            return Ok((task_id, true));
        }
        let mut tasks = self.inner.lock_tasks()?;
        apply_organization_to_request(&self.inner, &mut request, &tasks, correlation_id)?;
        request.validate()?;
        let now = now_unix_ms();
        let task = DownloadTask::new(request, RequestOrigin::Browser, now);
        let transfer = StoredTransferState::default();
        let delayed_until_due = task
            .request()
            .scheduled_at_unix_ms
            .is_some_and(|scheduled_at| scheduled_at > now);
        let saved = self.inner.store.save_browser_task_with_transfer_and_queue(
            handoff_id,
            &task,
            &transfer,
            "default",
            mode == DownloadSubmitMode::StartNow && !delayed_until_due,
        )?;
        let task_id = match saved {
            crate::storage::BrowserHandoffSave::Inserted(task_id) => task_id,
            crate::storage::BrowserHandoffSave::Duplicate(task_id) => {
                drop(tasks);
                info!(
                    event = "download.browser_handoff.duplicate_accepted",
                    %correlation_id,
                    %handoff_id,
                    ?task_id,
                    "duplicate browser handoff resolved to the existing task"
                );
                return Ok((task_id, true));
            }
        };

        tasks.insert(
            task_id,
            EngineTaskState {
                task,
                transfer,
                control: None,
                last_progress_event: None,
                transfer_rate: None,
            },
        );
        drop(tasks);
        self.inner.emit_task(task_id);
        if delayed_until_due {
            self.refresh_delayed_schedule_wake()?;
        } else if mode == DownloadSubmitMode::StartNow
            && let Err(error) = self.schedule_queue_locked("default", correlation_id)
        {
            warn!(
                event = "download.browser_handoff.admission_deferred",
                %correlation_id,
                %handoff_id,
                ?task_id,
                error = %error,
                "accepted browser task remains eligible for a later queue refill"
            );
        }
        info!(
            event = "download.browser_handoff.accepted",
            %correlation_id,
            %handoff_id,
            ?task_id,
            "browser handoff accepted"
        );
        Ok((task_id, false))
    }

    fn submit_browser_batch_correlated(
        &self,
        submissions: Vec<BrowserDownloadSubmission>,
        correlation_id: uuid::Uuid,
    ) -> Result<Vec<BrowserBatchSubmissionResult>, DownloadEngineError> {
        if submissions.is_empty() || submissions.len() > MAX_BROWSER_BATCH_ITEMS {
            return Err(DownloadEngineError::InvalidBrowserBatchSize {
                count: submissions.len(),
                maximum: MAX_BROWSER_BATCH_ITEMS,
            });
        }
        let mut handoff_ids = HashSet::with_capacity(submissions.len());
        for submission in &submissions {
            if !handoff_ids.insert(submission.handoff_id) {
                return Err(DownloadEngineError::DuplicateBrowserBatchHandoff);
            }
            submission.request.validate()?;
        }

        let started_at = Instant::now();
        info!(
            event = "download.browser_batch.started",
            %correlation_id,
            item_count = submissions.len(),
            "browser batch submission started"
        );
        let mut accepted_count = 0_usize;
        let mut duplicate_count = 0_usize;
        let results = submissions
            .into_iter()
            .map(|submission| {
                let handoff_id = submission.handoff_id;
                match self.submit_browser_correlated(
                    handoff_id,
                    submission.request,
                    DownloadSubmitMode::QueueOnly,
                    correlation_id,
                ) {
                    Ok((task_id, duplicate)) => {
                        accepted_count += 1;
                        duplicate_count += usize::from(duplicate);
                        BrowserBatchSubmissionResult {
                            handoff_id,
                            task_id: Some(task_id),
                            duplicate,
                            fault: None,
                        }
                    }
                    Err(error) => BrowserBatchSubmissionResult {
                        handoff_id,
                        task_id: None,
                        duplicate: false,
                        fault: Some(error.fault()),
                    },
                }
            })
            .collect::<Vec<_>>();
        info!(
            event = "download.browser_batch.completed",
            %correlation_id,
            item_count = results.len(),
            accepted_count,
            duplicate_count,
            rejected_count = results.len().saturating_sub(accepted_count),
            elapsed_ms = started_at.elapsed().as_millis() as u64,
            "browser batch submission completed"
        );
        Ok(results)
    }

    fn import_browser_blob_correlated(
        &self,
        import: BrowserBlobImportRequest,
        correlation_id: uuid::Uuid,
    ) -> Result<(DownloadTaskId, bool), DownloadEngineError> {
        let BrowserBlobImportRequest {
            handoff_id,
            blob,
            save_directory,
            file_name,
            category_id,
            save_directory_explicit,
        } = import;
        let started_at = Instant::now();
        if let Some(task_id) = self.inner.store.browser_handoff_task(handoff_id)? {
            info!(
                event = "download.browser_blob.duplicate_accepted",
                %correlation_id,
                %handoff_id,
                ?task_id,
                "duplicate browser Blob import resolved to the existing task"
            );
            return Ok((task_id, true));
        }
        let staging_root = self
            .inner
            .browser_blob_staging_root
            .as_ref()
            .ok_or_else(|| {
                DownloadEngineError::Runtime("browser Blob staging is unavailable".into())
            })?;
        if blob.size == 0 || blob.file_name.trim().is_empty() || file_name.trim().is_empty() {
            return Err(DownloadRequestError::InvalidFileName.into());
        }
        let staged_path = staging_root.join(format!("{}.blob", blob.id));
        let observed_size = std::fs::metadata(&staged_path)
            .map_err(|error| DownloadEngineError::Runtime(error.to_string()))?
            .len();
        if observed_size != blob.size {
            return Err(DownloadEngineError::Runtime(
                "staged browser Blob size does not match its metadata".into(),
            ));
        }
        let source_url = format!("https://browser-blob.invalid/{}", blob.id);
        let mut request = DownloadRequest {
            source_url: source_url.clone(),
            save_directory,
            save_directory_explicit,
            file_name: Some(file_name.clone()),
            headers: Vec::new(),
            cookie: None,
            referrer: None,
            certificate_policy: crate::DownloadCertificatePolicy::Strict,
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: Some(blob.size),
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id,
            category_name: None,
            duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
        };
        request.validate()?;
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if let Some(task_id) = self.inner.store.browser_handoff_task(handoff_id)? {
            return Ok((task_id, true));
        }
        let mut tasks = self.inner.lock_tasks()?;
        apply_organization_to_request(&self.inner, &mut request, &tasks, correlation_id)?;
        request.validate()?;
        let resolved_file_name = request
            .file_name
            .clone()
            .ok_or(DownloadRequestError::InvalidFileName)?;
        let final_path = request.save_directory.join(&resolved_file_name);
        import_blob_artifact(&staged_path, &final_path)?;
        let now = now_unix_ms();
        let mut task = DownloadTask::new(request, RequestOrigin::Browser, now);
        for status in [
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
            DownloadStatus::Finalizing,
            DownloadStatus::Completed,
        ] {
            task.transition_to(status, now)?;
        }
        let transfer = StoredTransferState {
            probe: Some(DownloadProbe {
                final_url: source_url,
                file_name: Some(resolved_file_name),
                total_bytes: Some(blob.size),
                supports_ranges: false,
                content_type: blob.media_type,
                validator: crate::ResourceValidator::default(),
                media: None,
            }),
            downloaded_bytes: blob.size,
            error: None,
            temporary_path: None,
            final_path: Some(final_path.clone()),
            final_committed: true,
        };
        let saved = persist_browser_blob_with_compensation(&staged_path, &final_path, || {
            self.inner.store.save_browser_task_with_transfer_and_queue(
                handoff_id, &task, &transfer, "default", false,
            )
        });
        if let Err(error) = saved {
            match error {
                BrowserBlobPersistenceError::Persistence(error) => {
                    warn!(
                        event = "download.browser_blob.persistence_rejected",
                        %correlation_id,
                        %handoff_id,
                        blob_id = %blob.id,
                        elapsed_ms = started_at.elapsed().as_millis(),
                        "browser Blob database save failed and the artifact was restored to staging"
                    );
                    return Err(error.into());
                }
                BrowserBlobPersistenceError::Rollback {
                    persistence,
                    rollback,
                } => {
                    error!(
                        event = "download.browser_blob.rollback_failed",
                        %correlation_id,
                        %handoff_id,
                        blob_id = %blob.id,
                        elapsed_ms = started_at.elapsed().as_millis(),
                        error = %rollback,
                        "browser Blob database rejection could not restore the staged artifact"
                    );
                    return Err(DownloadEngineError::Runtime(format!(
                        "browser Blob database save failed ({persistence}); artifact rollback also failed ({rollback})"
                    )));
                }
            }
        }
        let task_id = task.id();
        tasks.insert(
            task_id,
            EngineTaskState {
                task,
                transfer,
                control: None,
                last_progress_event: None,
                transfer_rate: None,
            },
        );
        drop(tasks);
        self.inner.emit_task(task_id);
        info!(
            event = "download.browser_blob.imported",
            %correlation_id,
            %handoff_id,
            ?task_id,
            size = blob.size,
            elapsed_ms = started_at.elapsed().as_millis(),
            "browser Blob imported as a completed download artifact"
        );
        Ok((task_id, false))
    }

    fn discard_browser_blob(
        &self,
        handoff_id: uuid::Uuid,
        blob_id: uuid::Uuid,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
        let started_at = Instant::now();
        let staging_root = self
            .inner
            .browser_blob_staging_root
            .as_ref()
            .ok_or_else(|| {
                DownloadEngineError::Runtime("browser Blob staging is unavailable".into())
            })?;
        let staged_path = staging_root.join(format!("{blob_id}.blob"));
        match std::fs::remove_file(staged_path) {
            Ok(()) => info!(
                event = "download.browser_blob.discarded",
                %correlation_id,
                %handoff_id,
                %blob_id,
                elapsed_ms = started_at.elapsed().as_millis(),
                "uncommitted browser Blob staging was discarded"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => info!(
                event = "download.browser_blob.cleanup_completed",
                %correlation_id,
                %handoff_id,
                %blob_id,
                elapsed_ms = started_at.elapsed().as_millis(),
                "browser Blob staging cleanup completed with no remaining artifact"
            ),
            Err(error) => {
                warn!(
                    event = "download.browser_blob.discard_failed",
                    %correlation_id,
                    %handoff_id,
                    %blob_id,
                    native_error_code = error.raw_os_error().unwrap_or_default(),
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "browser Blob staging cleanup failed"
                );
                return Err(DownloadEngineError::Runtime(error.to_string()));
            }
        }
        Ok(())
    }

    fn open_browser_dialog_correlated(
        &self,
        handoff_id: uuid::Uuid,
        request: DownloadRequest,
        correlation_id: uuid::Uuid,
    ) -> Result<bool, DownloadEngineError> {
        request.validate()?;
        self.open_browser_dialog_source_correlated(
            handoff_id,
            BrowserDownloadDialogSource::Download(Box::new(request)),
            correlation_id,
        )
    }

    fn open_browser_blob_dialog_correlated(
        &self,
        handoff_id: uuid::Uuid,
        blob: BrowserBlob,
        save_directory: PathBuf,
        correlation_id: uuid::Uuid,
    ) -> Result<bool, DownloadEngineError> {
        if self
            .inner
            .browser_dialog_handoffs
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .contains(&handoff_id)
        {
            info!(
                event = "download.browser_dialog.duplicate_accepted",
                %correlation_id,
                %handoff_id,
                "duplicate browser Blob confirmation-window request was suppressed"
            );
            return Ok(true);
        }
        let staging_root = self
            .inner
            .browser_blob_staging_root
            .as_ref()
            .ok_or_else(|| {
                DownloadEngineError::Runtime("browser Blob staging is unavailable".into())
            })?;
        let staged_path = staging_root.join(format!("{}.blob", blob.id));
        if blob.size == 0
            || blob.file_name.trim().is_empty()
            || std::fs::metadata(staged_path)
                .map_err(|error| DownloadEngineError::Runtime(error.to_string()))?
                .len()
                != blob.size
        {
            return Err(DownloadEngineError::Runtime(
                "staged browser Blob is missing or incomplete".into(),
            ));
        }
        self.open_browser_dialog_source_correlated(
            handoff_id,
            BrowserDownloadDialogSource::Blob {
                blob,
                save_directory,
            },
            correlation_id,
        )
    }

    fn open_browser_dialog_source_correlated(
        &self,
        handoff_id: uuid::Uuid,
        source: BrowserDownloadDialogSource,
        correlation_id: uuid::Uuid,
    ) -> Result<bool, DownloadEngineError> {
        if !self.inner.browser_dialog_available.load(Ordering::Acquire) {
            debug!(
                event = "download.browser_dialog.consumer_not_ready",
                %correlation_id,
                %handoff_id,
                "browser confirmation window is waiting for the GPUI consumer"
            );
            return Err(DownloadEngineError::BrowserDialogConsumerUnavailable);
        }

        let mut handoffs = self
            .inner
            .browser_dialog_handoffs
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if handoffs.contains(&handoff_id) {
            info!(
                event = "download.browser_dialog.duplicate_accepted",
                %correlation_id,
                %handoff_id,
                "duplicate browser confirmation-window request was suppressed"
            );
            return Ok(true);
        }
        let (completion, completed) = std::sync::mpsc::sync_channel(1);
        self.inner
            .browser_dialog_sender
            .try_send(BrowserDownloadDialogRequest {
                correlation_id,
                handoff_id,
                source,
                completion,
            })
            .map_err(|_| DownloadEngineError::BrowserDialogUnavailable)?;
        match completed.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {}
            Ok(Err(_))
            | Err(std::sync::mpsc::RecvTimeoutError::Timeout)
            | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                warn!(
                    event = "download.browser_dialog.open_unconfirmed",
                    %correlation_id,
                    %handoff_id,
                    "browser confirmation window was not confirmed by GPUI"
                );
                return Err(DownloadEngineError::BrowserDialogUnavailable);
            }
        }
        handoffs.push_back(handoff_id);
        while handoffs.len() > 4096 {
            handoffs.pop_front();
        }
        info!(
            event = "download.browser_dialog.accepted",
            %correlation_id,
            %handoff_id,
            "browser confirmation-window request queued for GPUI"
        );
        Ok(false)
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
                task: Box::new(state.task.clone()),
            });
            drop(tasks);
            drop(_scheduler);
            self.refresh_delayed_schedule_wake()?;
            return Ok(());
        }
        let dash_finalizing = state.task.status() == DownloadStatus::Finalizing
            && matches!(
                state.task.request().media.as_ref(),
                Some(DownloadRequestMedia::Dash(_))
            );
        if !matches!(
            state.task.status(),
            DownloadStatus::Probing | DownloadStatus::Waiting | DownloadStatus::Downloading
        ) && !dash_finalizing
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
            DownloadStatus::Finalizing
                if matches!(
                    state.task.request().media.as_ref(),
                    Some(DownloadRequestMedia::Dash(_))
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
        self.remove_correlated(task_id, delete_artifact, uuid::Uuid::new_v4())
    }

    fn remove_correlated(
        &self,
        task_id: DownloadTaskId,
        delete_artifact: bool,
        correlation_id: uuid::Uuid,
    ) -> Result<(), DownloadEngineError> {
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
            snapshot_from_state(state)
        };

        if matches!(
            snapshot.task.request().media.as_ref(),
            Some(DownloadRequestMedia::Hls(_))
        ) {
            info!(
                event = "download.hls_cleanup.started",
                %correlation_id,
                ?task_id,
                reason = "removed",
                "HLS task-owned artifact cleanup started"
            );
            remove_hls_owned_artifacts(
                snapshot.task.request(),
                task_id,
                snapshot.temporary_path.as_deref(),
                correlation_id,
                "removed",
                started_at,
            )?;
            info!(
                event = "download.hls_cleanup.completed",
                %correlation_id,
                ?task_id,
                reason = "removed",
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
                reason = "removed",
                "DASH task-owned artifact cleanup started"
            );
            remove_dash_owned_artifacts(
                snapshot.task.request(),
                task_id,
                snapshot.temporary_path.as_deref(),
                correlation_id,
                "removed",
                started_at,
            )?;
            info!(
                event = "download.dash_cleanup.completed",
                %correlation_id,
                ?task_id,
                reason = "removed",
                elapsed_ms = started_at.elapsed().as_millis(),
                "DASH task-owned artifact cleanup completed"
            );
        } else if let Some(temporary_path) = snapshot.temporary_path.as_deref() {
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
    fn http_for_operation(
        &self,
        certificate_policy: crate::DownloadCertificatePolicy,
    ) -> Result<HttpDownloadClient, DownloadEngineError> {
        let snapshot = self.network_policy.snapshot()?;
        Ok(self
            .http_template
            .for_network_snapshot(&snapshot, certificate_policy)?)
    }

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

fn spawn_download_worker(
    inner: Arc<DownloadEngineInner>,
    queue_id: String,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
) -> Result<(), DownloadEngineError> {
    download_runtime()?.spawn(async move {
        let started_at = Instant::now();
        let certificate_policy = inner
            .lock_tasks()
            .ok()
            .and_then(|tasks| {
                tasks
                    .get(&task_id)
                    .map(|state| state.task.request().certificate_policy)
            })
            .unwrap_or_default();
        let http = inner.http_for_operation(certificate_policy);
        let (proxy_mode, proxy_source) =
            http.as_ref().map_or(("unavailable", "policy"), |client| {
                (
                    proxy_mode_name(client.network_proxy_mode()),
                    client.network_proxy_source(),
                )
            });
        let diagnostics = DownloadAttemptDiagnostics {
            proxy_mode,
            proxy_source,
            certificate_policy,
        };
        info!(
            event = "download.task.attempt_started",
            %correlation_id,
            ?task_id,
            proxy_mode,
            proxy_source,
            certificate_policy = certificate_policy_name(certificate_policy),
            "download task attempt started"
        );
        let result = match http {
            Ok(http) => run_task(Arc::clone(&inner), task_id, correlation_id, http).await,
            Err(error) => Err(error),
        };
        match result {
            Ok(()) => {
                let outcome = inner
                    .lock_tasks()
                    .ok()
                    .and_then(|tasks| tasks.get(&task_id).map(|state| state.task.status()))
                    .map(download_terminal_name)
                    .unwrap_or("unknown");
                info!(
                    event = "download.task.attempt_completed",
                    %correlation_id,
                    ?task_id,
                    proxy_mode = diagnostics.proxy_mode,
                    proxy_source = diagnostics.proxy_source,
                    certificate_policy = certificate_policy_name(certificate_policy),
                    outcome,
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "download task attempt reached its non-failure terminal"
                );
            }
            Err(error) => {
                record_failure(
                    &inner,
                    task_id,
                    correlation_id,
                    started_at.elapsed(),
                    diagnostics,
                    error,
                );
            }
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
    http: HttpDownloadClient,
) -> Result<(), DownloadEngineError> {
    // `spawn_download_worker` freezes one route before entering this attempt. Every probe,
    // fragment and retry below shares this client and cannot re-snapshot a settings change.
    let request = {
        let tasks = inner.lock_tasks()?;
        tasks
            .get(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?
            .task
            .request()
            .clone()
    };
    if let Some(DownloadRequestMedia::Hls(hls)) = request.media.as_ref() {
        return run_hls_task(
            Arc::clone(&inner),
            task_id,
            correlation_id,
            &request,
            &hls.playlist_url,
            &http,
        )
        .await;
    }
    if let Some(DownloadRequestMedia::Dash(dash)) = request.media.as_ref() {
        return run_dash_task(
            Arc::clone(&inner),
            task_id,
            correlation_id,
            &request,
            dash,
            &http,
        )
        .await;
    }
    let probe_started_at = Instant::now();
    info!(
        event = "download.probe.started",
        %correlation_id,
        ?task_id,
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        certificate_policy = certificate_policy_name(http.certificate_policy()),
        requested_connections = request.connection_limit.unwrap_or(1),
        "download resource probe started"
    );
    let probe = match http.probe(&request).await {
        Ok(probe) => probe,
        Err(error) => {
            warn!(
                event = "download.probe.failed",
                %correlation_id,
                ?task_id,
                proxy_mode = proxy_mode_name(http.network_proxy_mode()),
                proxy_source = http.network_proxy_source(),
                certificate_policy = certificate_policy_name(http.certificate_policy()),
                error_kind = error.kind(),
                native_error_code = error.io_error().and_then(std::io::Error::raw_os_error),
                elapsed_ms = probe_started_at.elapsed().as_millis(),
                "download resource probe failed"
            );
            return Err(error.into());
        }
    };
    let media_kind = match probe.media.as_ref() {
        Some(crate::DownloadMediaProbe::Hls(_)) => "hls",
        Some(crate::DownloadMediaProbe::Dash(_)) => "dash",
        None => "ordinary",
    };
    let (variant_count, video_track_count, audio_track_count) = match probe.media.as_ref() {
        Some(crate::DownloadMediaProbe::Hls(hls)) => (hls.variants.len(), 0, 0),
        Some(crate::DownloadMediaProbe::Dash(dash)) => {
            (0, dash.video_tracks.len(), dash.audio_tracks.len())
        }
        None => (0, 0, 0),
    };
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
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        certificate_policy = certificate_policy_name(http.certificate_policy()),
        media_kind,
        variant_count,
        video_track_count,
        audio_track_count,
        total_bytes = probe.total_bytes.unwrap_or_default(),
        total_bytes_known = probe.total_bytes.is_some(),
        supports_ranges = probe.supports_ranges,
        validator_kind,
        elapsed_ms = probe_started_at.elapsed().as_millis(),
        "download resource probe completed"
    );
    let automatic_media = match probe.media.as_ref() {
        Some(crate::DownloadMediaProbe::Hls(hls)) => {
            Some(DownloadRequestMedia::Hls(crate::HlsDownloadRequest {
                playlist_url: if hls.is_master {
                    hls.variants
                        .first()
                        .map(|variant| variant.playlist_url.clone())
                        .ok_or(HttpDownloadError::InvalidHlsManifest(
                            "HLS master playlist has no variant",
                        ))?
                } else {
                    probe.final_url.clone()
                },
            }))
        }
        Some(crate::DownloadMediaProbe::Dash(dash)) => {
            let video_representation_id = dash
                .video_tracks
                .first()
                .map(|track| track.representation_id.clone());
            let audio_representation_id = dash
                .audio_tracks
                .first()
                .map(|track| track.representation_id.clone());
            Some(DownloadRequestMedia::Dash(crate::DashDownloadRequest {
                manifest_url: probe.final_url.clone(),
                video_representation_id,
                audio_representation_id,
            }))
        }
        None => None,
    };
    if let Some(media) = automatic_media {
        let automatic_file_name = automatic_media_file_name(
            &media,
            request.file_name.as_deref(),
            probe.file_name.as_deref(),
        );
        let selected_request = {
            let mut tasks = inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            state.task.set_media(media.clone(), now_unix_ms());
            if let Some(file_name) = automatic_file_name {
                state.task.set_file_name(file_name, now_unix_ms());
            }
            inner.persist(state)?;
            state.task.request().clone()
        };
        match media {
            DownloadRequestMedia::Hls(hls) => {
                return run_hls_task(
                    Arc::clone(&inner),
                    task_id,
                    correlation_id,
                    &selected_request,
                    &hls.playlist_url,
                    &http,
                )
                .await;
            }
            DownloadRequestMedia::Dash(dash) => {
                return run_dash_task(
                    Arc::clone(&inner),
                    task_id,
                    correlation_id,
                    &selected_request,
                    &dash,
                    &http,
                )
                .await;
            }
        }
    }
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
            task: Box::new(state.task.clone()),
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
            &http,
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
        &http,
        "probe_single_stream",
    )
    .await
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

async fn run_hls_task(
    inner: Arc<DownloadEngineInner>,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    playlist_url: &str,
    http: &HttpDownloadClient,
) -> Result<(), DownloadEngineError> {
    let started_at = Instant::now();
    let stored_plan = match inner.store.load_hls_transfer_plan(task_id) {
        Ok(plan) => plan,
        Err(error) => {
            info!(
                event = "download.hls_playlist.started",
                %correlation_id,
                ?task_id,
                source = "checkpoint",
                "HLS media playlist resolution started"
            );
            warn!(
                event = "download.hls_playlist.failed",
                %correlation_id,
                ?task_id,
                source = "checkpoint",
                error_kind = "storage",
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS media playlist checkpoint could not be loaded"
            );
            return Err(error.into());
        }
    };
    let playlist_source = if stored_plan.is_some() {
        "network_revalidation"
    } else {
        "network"
    };
    info!(
        event = "download.hls_playlist.started",
        %correlation_id,
        ?task_id,
        source = playlist_source,
        "HLS media playlist resolution started"
    );
    if stored_plan
        .as_ref()
        .is_some_and(|plan| plan.segments.is_empty())
    {
        warn!(
            event = "download.hls_playlist.failed",
            %correlation_id,
            ?task_id,
            source = playlist_source,
            error_kind = "invalid_checkpoint",
            elapsed_ms = started_at.elapsed().as_millis(),
            "persisted HLS media playlist checkpoint is empty"
        );
        return Err(HttpDownloadError::HlsResourceChanged.into());
    }
    let playlist = match http.load_hls_media_playlist(request, playlist_url).await {
        Ok(playlist) => playlist,
        Err(error) => {
            warn!(
                event = "download.hls_playlist.failed",
                %correlation_id,
                ?task_id,
                source = playlist_source,
                error_kind = error.kind(),
                retryable = error.is_retryable_transfer(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS media playlist resolution failed"
            );
            return Err(error.into());
        }
    };
    let source = if let Some(plan) = stored_plan.as_ref() {
        if !hls_plan_is_compatible(plan, &playlist) {
            warn!(
                event = "download.hls_playlist.failed",
                %correlation_id,
                ?task_id,
                source = "network_revalidation",
                error_kind = "hls_resource_changed",
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS media playlist no longer matches the durable transfer plan"
            );
            return Err(HttpDownloadError::HlsResourceChanged.into());
        }
        if let Err(error) = inner
            .store
            .refresh_hls_transfer_plan_urls(task_id, &playlist)
        {
            warn!(
                event = "download.hls_playlist.failed",
                %correlation_id,
                ?task_id,
                source = "network_revalidation",
                error_kind = "storage",
                elapsed_ms = started_at.elapsed().as_millis(),
                "refreshed HLS media playlist URLs could not be persisted"
            );
            return Err(error.into());
        }
        "network_revalidation"
    } else {
        if let Err(error) = inner.store.save_hls_transfer_plan(task_id, &playlist) {
            warn!(
                event = "download.hls_playlist.failed",
                %correlation_id,
                ?task_id,
                source = "network",
                error_kind = "storage",
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS media transfer plan could not be persisted"
            );
            return Err(error.into());
        }
        info!(
            event = "download.hls_transfer_plan.created",
            %correlation_id,
            ?task_id,
            segment_count = playlist.segments.len(),
            "HLS media transfer plan persisted"
        );
        "network"
    };
    info!(
        event = "download.hls_playlist.completed",
        %correlation_id,
        ?task_id,
        source,
        segment_count = playlist.segments.len(),
        elapsed_ms = started_at.elapsed().as_millis(),
        "HLS media playlist resolved and persisted"
    );
    let encrypted_segment_count = playlist
        .segments
        .iter()
        .filter(|segment| {
            matches!(
                segment.encryption,
                crate::hls::HlsSegmentEncryption::Aes128 { .. }
            )
        })
        .count();
    let key_slot_count = playlist
        .segments
        .iter()
        .filter_map(|segment| match segment.encryption {
            crate::hls::HlsSegmentEncryption::Aes128 { key_slot, .. } => Some(key_slot),
            crate::hls::HlsSegmentEncryption::None => None,
        })
        .collect::<HashSet<_>>()
        .len();
    let explicit_iv_count = playlist
        .segments
        .iter()
        .filter(|segment| {
            matches!(
                segment.encryption,
                crate::hls::HlsSegmentEncryption::Aes128 {
                    explicit_iv: Some(_),
                    ..
                }
            )
        })
        .count();
    if encrypted_segment_count > 0 {
        info!(
            event = "download.hls_encryption.configured",
            %correlation_id,
            ?task_id,
            encrypted_segment_count,
            key_slot_count,
            explicit_iv_count,
            "HLS AES-128 transfer configuration accepted"
        );
    }
    let fallback_file_name = url::Url::parse(&playlist.final_url)
        .ok()
        .and_then(|url| {
            Path::new(url.path())
                .file_stem()
                .map(|stem| format!("{}.ts", stem.to_string_lossy()))
        })
        .unwrap_or_else(|| "video.ts".into());
    let mut probe = DownloadProbe {
        final_url: playlist.final_url.clone(),
        file_name: Some(fallback_file_name),
        total_bytes: None,
        supports_ranges: false,
        content_type: Some("application/vnd.apple.mpegurl".into()),
        validator: crate::ResourceValidator::default(),
        media: Some(crate::DownloadMediaProbe::Hls(crate::HlsProbe {
            is_master: false,
            variants: Vec::new(),
        })),
    };
    let (temporary_path, final_path) =
        HttpDownloadClient::artifact_paths(request, &probe, task_id)?;
    tokio::fs::create_dir_all(&request.save_directory).await?;
    if tokio::fs::try_exists(&final_path).await? {
        return Err(HttpDownloadError::DestinationExists(final_path).into());
    }
    let working_directory = hls_working_directory(request, task_id);
    tokio::fs::create_dir_all(&working_directory).await?;
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
        state.transfer_rate = Some(TransferRateSampler::new(0, Instant::now()));
        let control = state
            .control
            .clone()
            .ok_or(DownloadEngineError::MissingTransferControl(task_id))?;
        inner.persist(state)?;
        inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(state.task.clone()),
        });
        control
    };
    info!(
        event = "download.hls_transfer.started",
        %correlation_id,
        ?task_id,
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        segment_count = playlist.segments.len(),
        requested_connections = request.connection_limit.unwrap_or(1),
        "HLS media transfer started"
    );

    let stored_segments = stored_plan
        .map(|plan| {
            plan.segments
                .into_iter()
                .map(|segment| (segment.media.index, segment))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_else(|| {
            playlist
                .segments
                .iter()
                .map(|segment| {
                    (
                        segment.index,
                        crate::storage::StoredHlsSegment {
                            media: segment.clone(),
                            downloaded_bytes: 0,
                            state: DownloadSegmentState::Waiting,
                        },
                    )
                })
                .collect()
        });
    let mut completed_bytes = 0_u64;
    let mut completed_segments = 0_usize;
    let mut adopted_segments = 0_usize;
    let mut reset_segments = 0_usize;
    let mut pending_segments = VecDeque::new();
    for segment in &playlist.segments {
        let checkpoint = stored_segments.get(&segment.index).ok_or_else(|| {
            DownloadEngineError::Runtime(format!(
                "persisted HLS segment {} is missing",
                segment.index
            ))
        })?;
        let completed_path = working_directory.join(format!("{:08}.segment", segment.index));
        match tokio::fs::metadata(&completed_path).await {
            Ok(metadata)
                if metadata.len() > 0
                    && (checkpoint.state != DownloadSegmentState::Completed
                        || checkpoint.downloaded_bytes == metadata.len()) =>
            {
                completed_bytes = completed_bytes
                    .checked_add(metadata.len())
                    .ok_or(HttpDownloadError::LengthOverflow)?;
                completed_segments = completed_segments.saturating_add(1);
                if checkpoint.state != DownloadSegmentState::Completed
                    || checkpoint.downloaded_bytes != metadata.len()
                {
                    inner.store.save_hls_segment_checkpoint(
                        task_id,
                        segment.index,
                        DownloadSegmentState::Completed,
                        metadata.len(),
                    )?;
                    adopted_segments = adopted_segments.saturating_add(1);
                }
                continue;
            }
            Ok(_) => {
                tokio::fs::remove_file(&completed_path).await?;
                reset_segments = reset_segments.saturating_add(1);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        inner.store.save_hls_segment_checkpoint(
            task_id,
            segment.index,
            DownloadSegmentState::Waiting,
            0,
        )?;
        pending_segments.push_back(segment.clone());
    }
    if adopted_segments > 0 || reset_segments > 0 {
        info!(
            event = "download.hls_checkpoints.reconciled",
            %correlation_id,
            ?task_id,
            adopted_segments,
            reset_segments,
            segment_count = playlist.segments.len(),
            "HLS segment checkpoints reconciled with durable artifacts"
        );
    }
    let durable_completed_bytes = Arc::new(AtomicU64::new(completed_bytes));
    let active_progress = Arc::new(Mutex::new(HashMap::<u32, u64>::new()));
    let active_connections = Arc::new(AtomicU16::new(0));
    let key_cache = Arc::new(
        playlist
            .segments
            .iter()
            .filter_map(|segment| match segment.encryption {
                crate::hls::HlsSegmentEncryption::Aes128 { key_slot, .. } => Some((
                    key_slot,
                    Arc::new(tokio::sync::Mutex::new(None::<[u8; 16]>)),
                )),
                crate::hls::HlsSegmentEncryption::None => None,
            })
            .collect::<HashMap<_, _>>(),
    );
    let max_connections = usize::from(request.connection_limit.unwrap_or(1).clamp(1, 64));
    let mut transfers = tokio::task::JoinSet::new();
    while transfers.len() < max_connections
        && let Some(segment) = pending_segments.pop_front()
    {
        inner.store.save_hls_segment_checkpoint(
            task_id,
            segment.index,
            DownloadSegmentState::Active,
            0,
        )?;
        spawn_hls_segment_transfer(
            &mut transfers,
            &inner,
            http,
            task_id,
            correlation_id,
            request,
            &control,
            &working_directory,
            &durable_completed_bytes,
            &active_progress,
            &active_connections,
            &key_cache,
            segment,
        );
    }
    let mut paused = false;
    let mut cancelled = false;
    let mut first_error = None;
    while let Some(joined) = transfers.join_next().await {
        let (segment, staging_path, completed_path, outcome) =
            joined.map_err(|error| DownloadEngineError::Runtime(error.to_string()))?;
        active_progress
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .remove(&segment.index);
        match outcome {
            Ok(HlsSegmentDownloadOutcome::Completed { downloaded_bytes }) => {
                tokio::fs::rename(&staging_path, &completed_path).await?;
                inner.store.save_hls_segment_checkpoint(
                    task_id,
                    segment.index,
                    DownloadSegmentState::Completed,
                    downloaded_bytes,
                )?;
                completed_bytes = durable_completed_bytes
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                        current.checked_add(downloaded_bytes)
                    })
                    .map(|previous| previous + downloaded_bytes)
                    .map_err(|_| HttpDownloadError::LengthOverflow)?;
                completed_segments = completed_segments.saturating_add(1);
                let mut tasks = inner.lock_tasks()?;
                let state = tasks
                    .get_mut(&task_id)
                    .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
                state.transfer.downloaded_bytes = completed_bytes;
                inner.persist(state)?;
            }
            Ok(HlsSegmentDownloadOutcome::Paused) => {
                inner.store.save_hls_segment_checkpoint(
                    task_id,
                    segment.index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                paused = true;
            }
            Ok(HlsSegmentDownloadOutcome::Cancelled) => {
                inner.store.save_hls_segment_checkpoint(
                    task_id,
                    segment.index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                cancelled = true;
            }
            Err(error) => {
                inner.store.save_hls_segment_checkpoint(
                    task_id,
                    segment.index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                first_error.get_or_insert(error);
            }
        }
        if !paused
            && !cancelled
            && first_error.is_none()
            && let Some(next) = pending_segments.pop_front()
        {
            inner.store.save_hls_segment_checkpoint(
                task_id,
                next.index,
                DownloadSegmentState::Active,
                0,
            )?;
            spawn_hls_segment_transfer(
                &mut transfers,
                &inner,
                http,
                task_id,
                correlation_id,
                request,
                &control,
                &working_directory,
                &durable_completed_bytes,
                &active_progress,
                &active_connections,
                &key_cache,
                next,
            );
        }
    }
    completed_bytes = durable_completed_bytes.load(Ordering::Acquire);
    if let Some(error) = first_error {
        return Err(error.into());
    }
    if !paused && !cancelled {
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        match hls_stop_directive(&control) {
            Some(TransferDirective::Pause) => paused = true,
            Some(TransferDirective::Cancel) => cancelled = true,
            Some(TransferDirective::Running) | None => {
                state.transfer.downloaded_bytes = completed_bytes;
                probe.total_bytes = Some(completed_bytes);
                state.transfer.probe = Some(probe);
                state
                    .task
                    .transition_to(DownloadStatus::Finalizing, now_unix_ms())?;
                inner.persist(state)?;
                inner.emit(DownloadEvent::TaskChanged {
                    task: Box::new(state.task.clone()),
                });
            }
        }
    }
    if paused {
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.downloaded_bytes = completed_bytes;
        state
            .task
            .transition_to(DownloadStatus::Paused, now_unix_ms())?;
        state.control = None;
        inner.persist(state)?;
        inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(state.task.clone()),
        });
        info!(
            event = "download.hls_transfer.paused",
            %correlation_id,
            ?task_id,
            proxy_mode = proxy_mode_name(http.network_proxy_mode()),
            proxy_source = http.network_proxy_source(),
            completed_segments,
            downloaded_bytes = completed_bytes,
            elapsed_ms = started_at.elapsed().as_millis(),
            "HLS media transfer paused"
        );
        return Ok(());
    }
    if cancelled {
        let cleanup_started = Instant::now();
        let staging_cleanup = remove_hls_cancel_artifact(
            &working_directory,
            "segment_staging",
            true,
            task_id,
            correlation_id,
            cleanup_started,
        )
        .await;
        let merge_cleanup = remove_hls_cancel_artifact(
            &temporary_path,
            "merge_artifact",
            false,
            task_id,
            correlation_id,
            cleanup_started,
        )
        .await;
        staging_cleanup?;
        merge_cleanup?;
        inner.store.clear_hls_transfer_plan(task_id)?;
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.downloaded_bytes = completed_bytes;
        state
            .task
            .transition_to(DownloadStatus::Cancelled, now_unix_ms())?;
        state.control = None;
        inner.persist(state)?;
        inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(state.task.clone()),
        });
        info!(
            event = "download.hls_transfer.cancelled",
            %correlation_id,
            ?task_id,
            proxy_mode = proxy_mode_name(http.network_proxy_mode()),
            proxy_source = http.network_proxy_source(),
            downloaded_bytes = completed_bytes,
            elapsed_ms = started_at.elapsed().as_millis(),
            "HLS media transfer cancelled"
        );
        return Ok(());
    }

    let mut output = tokio::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&temporary_path)
        .await?;
    for segment in &playlist.segments {
        let completed_path = working_directory.join(format!("{:08}.segment", segment.index));
        let mut input = tokio::fs::File::open(completed_path).await?;
        tokio::io::copy(&mut input, &mut output).await?;
    }
    output.flush().await?;
    output.sync_all().await?;
    drop(output);
    http.commit_prepared(PreparedDownload {
        temporary_path,
        final_path,
        downloaded_bytes: completed_bytes,
    })
    .await?;
    commit_completed_state(&inner, task_id)?;
    let cleanup_started_at = Instant::now();
    if let Err(error) = tokio::fs::remove_dir_all(&working_directory).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        warn!(
            event = "download.hls_cleanup.failed",
            %correlation_id,
            ?task_id,
            reason = "completed",
            artifact_kind = "segment_directory",
            native_error_code = error.raw_os_error(),
            elapsed_ms = cleanup_started_at.elapsed().as_millis(),
            "HLS task staging cleanup failed"
        );
    }
    info!(
        event = "download.hls_transfer.completed",
        %correlation_id,
        ?task_id,
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        segment_count = playlist.segments.len(),
        downloaded_bytes = completed_bytes,
        elapsed_ms = started_at.elapsed().as_millis(),
        "HLS media transfer completed"
    );
    Ok(())
}

async fn run_dash_task(
    inner: Arc<DownloadEngineInner>,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    selection: &crate::DashDownloadRequest,
    http: &HttpDownloadClient,
) -> Result<(), DownloadEngineError> {
    let started_at = Instant::now();
    let stored_plan = inner.store.load_dash_transfer_plan(task_id)?;
    let manifest_source = if stored_plan.is_some() {
        "network_revalidation"
    } else {
        "network"
    };
    info!(
        event = "download.dash_manifest.started",
        %correlation_id,
        ?task_id,
        source = manifest_source,
        "DASH manifest resolution started"
    );
    let manifest = match http
        .load_dash_manifest(request, &selection.manifest_url)
        .await
    {
        Ok(manifest) => manifest,
        Err(error) => {
            warn!(
                event = "download.dash_manifest.failed",
                %correlation_id,
                ?task_id,
                source = manifest_source,
                error_kind = error.kind(),
                native_error_code = error.io_error().and_then(std::io::Error::raw_os_error),
                elapsed_ms = started_at.elapsed().as_millis(),
                "DASH manifest resolution failed"
            );
            return Err(error.into());
        }
    };
    info!(
        event = "download.dash_manifest.completed",
        %correlation_id,
        ?task_id,
        source = manifest_source,
        video_track_count = manifest
            .tracks
            .iter()
            .filter(|track| track.kind == crate::dash::DashTrackKind::Video)
            .count(),
        audio_track_count = manifest
            .tracks
            .iter()
            .filter(|track| track.kind == crate::dash::DashTrackKind::Audio)
            .count(),
        segment_count = manifest
            .tracks
            .iter()
            .map(|track| track.segments.len())
            .sum::<usize>(),
        elapsed_ms = started_at.elapsed().as_millis(),
        "DASH manifest resolved"
    );
    let mut selected_tracks = Vec::new();
    for (kind, identity) in [
        (
            crate::dash::DashTrackKind::Video,
            selection.video_representation_id.as_deref(),
        ),
        (
            crate::dash::DashTrackKind::Audio,
            selection.audio_representation_id.as_deref(),
        ),
    ] {
        let Some(identity) = identity else {
            continue;
        };
        let track = manifest
            .tracks
            .iter()
            .find(|track| track.kind == kind && track.representation_id == identity)
            .cloned()
            .ok_or(HttpDownloadError::InvalidDashManifest(
                "selected DASH Representation no longer exists",
            ))?;
        selected_tracks.push(track);
    }
    if selected_tracks.is_empty() {
        return Err(
            HttpDownloadError::InvalidDashManifest("DASH request has no selected track").into(),
        );
    }
    let selected_manifest = crate::dash::DashManifest {
        final_url: manifest.final_url.clone(),
        tracks: selected_tracks,
    };
    if let Some(stored) = stored_plan.as_ref() {
        if !dash_plan_is_compatible(stored, &selected_manifest) {
            return Err(HttpDownloadError::InvalidDashManifest(
                "DASH manifest changed incompatibly with the durable transfer plan",
            )
            .into());
        }
        inner
            .store
            .refresh_dash_transfer_plan_manifest(task_id, &selected_manifest)?;
    } else {
        inner
            .store
            .save_dash_transfer_plan(task_id, &selected_manifest)?;
        info!(
            event = "download.dash_transfer_plan.created",
            %correlation_id,
            ?task_id,
            track_count = selected_manifest.tracks.len(),
            segment_count = selected_manifest
                .tracks
                .iter()
                .map(|track| track.segments.len())
                .sum::<usize>(),
            "DASH transfer plan persisted"
        );
    }
    let selected_track_count = selected_manifest.tracks.len();
    let selected_segment_count = selected_manifest
        .tracks
        .iter()
        .map(|track| track.segments.len())
        .sum::<usize>();
    let fallback_file_name = url::Url::parse(&manifest.final_url)
        .ok()
        .and_then(|url| {
            Path::new(url.path())
                .file_stem()
                .map(|stem| format!("{}.mp4", stem.to_string_lossy()))
        })
        .unwrap_or_else(|| "video.mp4".into());
    let probe = DownloadProbe {
        final_url: manifest.final_url.clone(),
        file_name: Some(fallback_file_name),
        total_bytes: None,
        supports_ranges: false,
        content_type: Some("application/dash+xml".into()),
        validator: crate::ResourceValidator::default(),
        media: Some(crate::DownloadMediaProbe::Dash(manifest.probe())),
    };
    let (temporary_path, final_path) =
        HttpDownloadClient::artifact_paths(request, &probe, task_id)?;
    tokio::fs::create_dir_all(&request.save_directory).await?;
    if tokio::fs::try_exists(&final_path).await? {
        return Err(HttpDownloadError::DestinationExists(final_path).into());
    }
    let working_directory = dash_working_directory(request, task_id);
    tokio::fs::create_dir_all(&working_directory).await?;
    let control = {
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.probe = Some(probe);
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
        state.transfer_rate = Some(TransferRateSampler::new(0, Instant::now()));
        let control = state
            .control
            .clone()
            .ok_or(DownloadEngineError::MissingTransferControl(task_id))?;
        inner.persist(state)?;
        inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(state.task.clone()),
        });
        control
    };
    let mut resources = Vec::new();
    for track in &selected_manifest.tracks {
        if let Some(initialization_url) = track.initialization_url.as_ref() {
            resources.push((track.kind, None, initialization_url.clone()));
        }
        resources.extend(
            track
                .segments
                .iter()
                .map(|segment| (track.kind, Some(segment.index), segment.url.clone())),
        );
    }
    info!(
        event = "download.dash_transfer.started",
        %correlation_id,
        ?task_id,
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        track_count = selected_track_count,
        segment_count = selected_segment_count,
        requested_connections = request.connection_limit.unwrap_or(1),
        "DASH media transfer started"
    );

    let stored_fragments = stored_plan
        .as_ref()
        .map(|plan| {
            plan.fragments
                .iter()
                .map(|fragment| ((fragment.kind, fragment.segment_index), fragment.clone()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let mut adopted_fragments = 0_usize;
    let mut reset_fragments = 0_usize;
    let mut downloaded_bytes = 0_u64;
    let mut pending_fragments = VecDeque::new();
    for (index, (kind, segment_index, _)) in resources.iter().enumerate() {
        let staging_path = working_directory.join(format!("{index:08}.part"));
        let completed_path = working_directory.join(format!("{index:08}.fragment"));
        remove_async_file_if_exists(&staging_path).await?;
        match tokio::fs::metadata(&completed_path).await {
            Ok(metadata) if metadata.len() > 0 => {
                if stored_fragments
                    .get(&(*kind, *segment_index))
                    .is_none_or(|fragment| {
                        fragment.state != DownloadSegmentState::Completed
                            || fragment.downloaded_bytes != metadata.len()
                    })
                {
                    adopted_fragments = adopted_fragments.saturating_add(1);
                }
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    *kind,
                    *segment_index,
                    DownloadSegmentState::Completed,
                    metadata.len(),
                )?;
                downloaded_bytes = downloaded_bytes
                    .checked_add(metadata.len())
                    .ok_or(HttpDownloadError::LengthOverflow)?;
            }
            Ok(_) => {
                if stored_fragments
                    .get(&(*kind, *segment_index))
                    .is_some_and(|fragment| fragment.state == DownloadSegmentState::Completed)
                {
                    reset_fragments = reset_fragments.saturating_add(1);
                }
                remove_async_file_if_exists(&completed_path).await?;
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    *kind,
                    *segment_index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                pending_fragments.push_back((
                    index,
                    *kind,
                    *segment_index,
                    resources[index].2.clone(),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if stored_fragments
                    .get(&(*kind, *segment_index))
                    .is_some_and(|fragment| fragment.state == DownloadSegmentState::Completed)
                {
                    reset_fragments = reset_fragments.saturating_add(1);
                }
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    *kind,
                    *segment_index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                pending_fragments.push_back((
                    index,
                    *kind,
                    *segment_index,
                    resources[index].2.clone(),
                ));
            }
            Err(error) => return Err(error.into()),
        }
    }
    if adopted_fragments > 0 || reset_fragments > 0 {
        info!(
            event = "download.dash_checkpoints.reconciled",
            %correlation_id,
            ?task_id,
            adopted_fragments,
            reset_fragments,
            fragment_count = resources.len(),
            "DASH fragment checkpoints reconciled with durable artifacts"
        );
    }
    let durable_completed_bytes = Arc::new(AtomicU64::new(downloaded_bytes));
    let active_progress = Arc::new(Mutex::new(HashMap::<u32, u64>::new()));
    let active_connections = Arc::new(AtomicU16::new(0));
    let max_connections = usize::from(request.connection_limit.unwrap_or(1).clamp(1, 64));
    let mut transfers = tokio::task::JoinSet::new();
    while transfers.len() < max_connections
        && let Some((index, kind, segment_index, resource_url)) = pending_fragments.pop_front()
    {
        inner.store.save_dash_fragment_checkpoint(
            task_id,
            kind,
            segment_index,
            DownloadSegmentState::Active,
            0,
        )?;
        spawn_dash_fragment_transfer(
            &mut transfers,
            &inner,
            http,
            task_id,
            correlation_id,
            request,
            &control,
            &working_directory,
            &durable_completed_bytes,
            &active_progress,
            &active_connections,
            index,
            kind,
            segment_index,
            resource_url,
        );
    }
    let mut paused = false;
    let mut cancelled = false;
    let mut first_error = None;
    while let Some(joined) = transfers.join_next().await {
        let (index, kind, segment_index, staging_path, completed_path, outcome) =
            joined.map_err(|error| DownloadEngineError::Runtime(error.to_string()))?;
        active_progress
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .remove(&(index as u32));
        match outcome {
            Ok(HlsSegmentDownloadOutcome::Completed {
                downloaded_bytes: fragment_bytes,
            }) => {
                tokio::fs::rename(&staging_path, &completed_path).await?;
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    kind,
                    segment_index,
                    DownloadSegmentState::Completed,
                    fragment_bytes,
                )?;
                downloaded_bytes = durable_completed_bytes
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                        current.checked_add(fragment_bytes)
                    })
                    .map(|previous| previous + fragment_bytes)
                    .map_err(|_| HttpDownloadError::LengthOverflow)?;
                let mut tasks = inner.lock_tasks()?;
                let state = tasks
                    .get_mut(&task_id)
                    .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
                state.transfer.downloaded_bytes = downloaded_bytes;
                inner.persist(state)?;
            }
            Ok(HlsSegmentDownloadOutcome::Paused) => {
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    kind,
                    segment_index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                paused = true;
            }
            Ok(HlsSegmentDownloadOutcome::Cancelled) => {
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    kind,
                    segment_index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                cancelled = true;
            }
            Err(error) => {
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    kind,
                    segment_index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                first_error.get_or_insert(error);
            }
        }
        if !paused
            && !cancelled
            && first_error.is_none()
            && let Some((index, kind, segment_index, resource_url)) = pending_fragments.pop_front()
        {
            inner.store.save_dash_fragment_checkpoint(
                task_id,
                kind,
                segment_index,
                DownloadSegmentState::Active,
                0,
            )?;
            spawn_dash_fragment_transfer(
                &mut transfers,
                &inner,
                http,
                task_id,
                correlation_id,
                request,
                &control,
                &working_directory,
                &durable_completed_bytes,
                &active_progress,
                &active_connections,
                index,
                kind,
                segment_index,
                resource_url,
            );
        }
    }
    downloaded_bytes = durable_completed_bytes.load(Ordering::Acquire);
    if let Some(error) = first_error {
        return Err(error.into());
    }
    if !paused && !cancelled {
        match hls_stop_directive(&control) {
            Some(TransferDirective::Pause) => paused = true,
            Some(TransferDirective::Cancel) => cancelled = true,
            Some(TransferDirective::Running) | None => {}
        }
    }
    if paused {
        transition_dash_stopped(&inner, task_id, DownloadStatus::Paused, downloaded_bytes)?;
        info!(
            event = "download.dash_transfer.paused",
            %correlation_id,
            ?task_id,
            proxy_mode = proxy_mode_name(http.network_proxy_mode()),
            proxy_source = http.network_proxy_source(),
            downloaded_bytes,
            elapsed_ms = started_at.elapsed().as_millis(),
            "DASH media transfer paused"
        );
        return Ok(());
    }
    if cancelled {
        if let Err(error) = tokio::fs::remove_dir_all(&working_directory).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.into());
        }
        inner.store.clear_dash_transfer_plan(task_id)?;
        transition_dash_stopped(&inner, task_id, DownloadStatus::Cancelled, downloaded_bytes)?;
        info!(
            event = "download.dash_transfer.cancelled",
            %correlation_id,
            ?task_id,
            proxy_mode = proxy_mode_name(http.network_proxy_mode()),
            proxy_source = http.network_proxy_source(),
            downloaded_bytes,
            elapsed_ms = started_at.elapsed().as_millis(),
            "DASH media transfer cancelled"
        );
        return Ok(());
    }
    {
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.downloaded_bytes = downloaded_bytes;
        state
            .task
            .transition_to(DownloadStatus::Finalizing, now_unix_ms())?;
        inner.persist(state)?;
        inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(state.task.clone()),
        });
    }
    let operation = if selected_track_count == 2 {
        "ffmpeg_stream_copy_mux"
    } else {
        "single_track_concatenate"
    };
    let task_started_at = started_at;
    let finalize_started_at = Instant::now();
    let started_at = finalize_started_at;
    let mut ffmpeg_version_status = if selected_track_count == 2 {
        "not_started"
    } else {
        "not_applicable"
    };
    let mut ffmpeg_elapsed_ms = None;
    info!(
        event = "download.dash_finalize.started",
        %correlation_id,
        ?task_id,
        operation,
        "DASH finalization started"
    );
    let mut video_paths = Vec::new();
    let mut audio_paths = Vec::new();
    for (index, (kind, _, _)) in resources.iter().enumerate() {
        let path = working_directory.join(format!("{index:08}.fragment"));
        match kind {
            crate::dash::DashTrackKind::Video => video_paths.push(path),
            crate::dash::DashTrackKind::Audio => audio_paths.push(path),
        }
    }
    if selected_track_count == 1 {
        let paths = if video_paths.is_empty() {
            &audio_paths
        } else {
            &video_paths
        };
        match concatenate_dash_fragments(paths, &temporary_path, &control).await {
            Ok(crate::media::MediaProcessOutcome::Completed) => {}
            Ok(outcome) => {
                return complete_dash_finalize_stop(
                    &inner,
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    task_started_at,
                    downloaded_bytes,
                    outcome,
                    &temporary_path,
                    &working_directory,
                )
                .await;
            }
            Err(error) => {
                log_dash_finalize_failure(
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    &error,
                );
                return Err(error);
            }
        }
    } else {
        let video_track_path = working_directory.join("video.track");
        let audio_track_path = working_directory.join("audio.track");
        match concatenate_dash_fragments(&video_paths, &video_track_path, &control).await {
            Ok(crate::media::MediaProcessOutcome::Completed) => {}
            Ok(outcome) => {
                return complete_dash_finalize_stop(
                    &inner,
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    task_started_at,
                    downloaded_bytes,
                    outcome,
                    &temporary_path,
                    &working_directory,
                )
                .await;
            }
            Err(error) => {
                log_dash_finalize_failure(
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    &error,
                );
                return Err(error);
            }
        }
        match concatenate_dash_fragments(&audio_paths, &audio_track_path, &control).await {
            Ok(crate::media::MediaProcessOutcome::Completed) => {}
            Ok(outcome) => {
                return complete_dash_finalize_stop(
                    &inner,
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    task_started_at,
                    downloaded_bytes,
                    outcome,
                    &temporary_path,
                    &working_directory,
                )
                .await;
            }
            Err(error) => {
                log_dash_finalize_failure(
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    &error,
                );
                return Err(error);
            }
        }
        if let Err(error) = remove_async_file_if_exists(&temporary_path).await {
            let error = DownloadEngineError::Io(error);
            log_dash_finalize_failure(
                task_id,
                correlation_id,
                operation,
                ffmpeg_version_status,
                started_at,
                &error,
            );
            return Err(error);
        }
        let resolution_started = Instant::now();
        let configured_ffmpeg = inner
            .ffmpeg_executable
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .clone();
        let ffmpeg_resolution =
            crate::media::resolve_ffmpeg_executable(configured_ffmpeg.as_deref());
        let Some(ffmpeg_resolution) = ffmpeg_resolution else {
            warn!(
                event = "download.ffmpeg_resolution.completed",
                %correlation_id,
                ?task_id,
                source = "missing",
                available = false,
                elapsed_ms = resolution_started.elapsed().as_millis(),
                "FFmpeg executable resolution completed"
            );
            let error = DownloadEngineError::FfmpegUnavailable;
            log_dash_finalize_failure(
                task_id,
                correlation_id,
                operation,
                "unavailable",
                finalize_started_at,
                &error,
            );
            return Err(error);
        };
        info!(
            event = "download.ffmpeg_resolution.completed",
            %correlation_id,
            ?task_id,
            source = ffmpeg_resolution.source.as_str(),
            available = true,
            elapsed_ms = resolution_started.elapsed().as_millis(),
            "FFmpeg executable resolution completed"
        );
        let ffmpeg_started = Instant::now();
        match crate::media::detect_ffmpeg_version(&ffmpeg_resolution.executable, &control).await {
            Ok(crate::media::MediaProcessOutcome::Completed) => {}
            Ok(outcome) => {
                return complete_dash_finalize_stop(
                    &inner,
                    task_id,
                    correlation_id,
                    operation,
                    "interrupted",
                    started_at,
                    task_started_at,
                    downloaded_bytes,
                    outcome,
                    &temporary_path,
                    &working_directory,
                )
                .await;
            }
            Err(error) => {
                warn!(
                    event = "download.dash_finalize.failed",
                    %correlation_id,
                    ?task_id,
                    operation,
                    ffmpeg_version_status = "unavailable",
                    error_kind = error.kind(),
                    exit_code = error.exit_code(),
                    native_error_code = error.native_error_code(),
                    ffmpeg_elapsed_ms = ffmpeg_started.elapsed().as_millis(),
                    elapsed_ms = finalize_started_at.elapsed().as_millis(),
                    "FFmpeg detection failed"
                );
                return Err(DownloadEngineError::media_process(error));
            }
        }
        ffmpeg_version_status = "detected";
        match crate::media::mux_dash_tracks(
            &ffmpeg_resolution.executable,
            &video_track_path,
            &audio_track_path,
            &temporary_path,
            &control,
        )
        .await
        {
            Ok(crate::media::MediaProcessOutcome::Completed) => {
                ffmpeg_elapsed_ms = Some(ffmpeg_started.elapsed().as_millis());
            }
            Ok(outcome) => {
                return complete_dash_finalize_stop(
                    &inner,
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    task_started_at,
                    downloaded_bytes,
                    outcome,
                    &temporary_path,
                    &working_directory,
                )
                .await;
            }
            Err(error) => {
                if let Err(cleanup_error) = remove_async_file_if_exists(&temporary_path).await {
                    let cleanup_error = DownloadEngineError::Io(cleanup_error);
                    log_dash_finalize_failure(
                        task_id,
                        correlation_id,
                        operation,
                        ffmpeg_version_status,
                        started_at,
                        &cleanup_error,
                    );
                    return Err(cleanup_error);
                }
                warn!(
                    event = "download.dash_finalize.failed",
                    %correlation_id,
                    ?task_id,
                    operation,
                    ffmpeg_version_status = "detected",
                    error_kind = error.kind(),
                    exit_code = error.exit_code(),
                    native_error_code = error.native_error_code(),
                    ffmpeg_elapsed_ms = ffmpeg_started.elapsed().as_millis(),
                    elapsed_ms = finalize_started_at.elapsed().as_millis(),
                    "FFmpeg stream-copy mux failed"
                );
                return Err(DownloadEngineError::media_process(error));
            }
        }
    }
    if let Some(directive) = hls_stop_directive(&control) {
        let outcome = match directive {
            TransferDirective::Pause => crate::media::MediaProcessOutcome::Paused,
            TransferDirective::Cancel => crate::media::MediaProcessOutcome::Cancelled,
            TransferDirective::Running => unreachable!("stop helper returned Running"),
        };
        return complete_dash_finalize_stop(
            &inner,
            task_id,
            correlation_id,
            operation,
            ffmpeg_version_status,
            started_at,
            task_started_at,
            downloaded_bytes,
            outcome,
            &temporary_path,
            &working_directory,
        )
        .await;
    }
    if let Err(error) = http
        .commit_prepared(PreparedDownload {
            temporary_path,
            final_path,
            downloaded_bytes,
        })
        .await
    {
        let error = DownloadEngineError::Http(error);
        log_dash_finalize_failure(
            task_id,
            correlation_id,
            operation,
            ffmpeg_version_status,
            started_at,
            &error,
        );
        return Err(error);
    }
    if let Err(error) = commit_completed_state(&inner, task_id) {
        log_dash_finalize_failure(
            task_id,
            correlation_id,
            operation,
            ffmpeg_version_status,
            started_at,
            &error,
        );
        return Err(error);
    }
    let directory_removed = match tokio::fs::remove_dir_all(&working_directory).await {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(error) => {
            warn!(
                event = "download.dash_cleanup.failed",
                %correlation_id,
                ?task_id,
                reason = "completed",
                artifact_kind = "fragment_directory",
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "DASH task staging cleanup failed"
            );
            false
        }
    };
    if directory_removed && let Err(_error) = inner.store.clear_dash_transfer_plan(task_id) {
        warn!(
            event = "download.dash_cleanup.failed",
            %correlation_id,
            ?task_id,
            reason = "completed",
            artifact_kind = "durable_plan",
            error_kind = "storage",
            native_error_code = Option::<i32>::None,
            elapsed_ms = started_at.elapsed().as_millis(),
            "DASH durable plan cleanup failed after publication"
        );
    }
    info!(
        event = "download.dash_finalize.completed",
        %correlation_id,
        ?task_id,
        operation,
        outcome = "completed",
        ffmpeg_version_status,
        ffmpeg_elapsed_ms,
        exit_code = if ffmpeg_version_status == "detected" {
            Some(0_i32)
        } else {
            None
        },
        downloaded_bytes,
        elapsed_ms = finalize_started_at.elapsed().as_millis(),
        "DASH finalization completed"
    );
    info!(
        event = "download.dash_transfer.completed",
        %correlation_id,
        ?task_id,
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        track_count = selected_track_count,
        segment_count = selected_segment_count,
        downloaded_bytes,
        elapsed_ms = task_started_at.elapsed().as_millis(),
        "DASH media transfer completed"
    );
    Ok(())
}

fn dash_plan_is_compatible(
    stored: &crate::storage::StoredDashTransferPlan,
    current: &crate::dash::DashManifest,
) -> bool {
    if stored.manifest.tracks.len() != current.tracks.len() {
        return false;
    }
    let tracks_match =
        stored
            .manifest
            .tracks
            .iter()
            .zip(&current.tracks)
            .all(|(stored, current)| {
                stored.kind == current.kind
                    && stored.representation_id == current.representation_id
                    && stored.bandwidth == current.bandwidth
                    && stored.resolution == current.resolution
                    && stored.codecs == current.codecs
                    && stored.mime_type == current.mime_type
                    && stored.language == current.language
                    && stored.initialization_url.is_some() == current.initialization_url.is_some()
                    && stored.segments.len() == current.segments.len()
                    && stored
                        .segments
                        .iter()
                        .zip(&current.segments)
                        .all(|(stored, current)| {
                            stored.index == current.index
                                && stored.number == current.number
                                && stored.time == current.time
                                && stored.duration_millis == current.duration_millis
                                && stored.duration_units == current.duration_units
                                && stored.timescale == current.timescale
                        })
            });
    if !tracks_match {
        return false;
    }
    let expected_fragment_count = current
        .tracks
        .iter()
        .map(|track| {
            track
                .segments
                .len()
                .saturating_add(usize::from(track.initialization_url.is_some()))
        })
        .sum::<usize>();
    stored.fragments.len() == expected_fragment_count
        && current.tracks.iter().all(|track| {
            let initialization_matches = track.initialization_url.is_none()
                || stored.fragments.iter().any(|fragment| {
                    fragment.kind == track.kind && fragment.segment_index.is_none()
                });
            initialization_matches
                && track.segments.iter().all(|segment| {
                    stored.fragments.iter().any(|fragment| {
                        fragment.kind == track.kind && fragment.segment_index == Some(segment.index)
                    })
                })
        })
}

async fn remove_async_file_if_exists(path: &Path) -> Result<(), std::io::Error> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

async fn concatenate_dash_fragments(
    fragments: &[PathBuf],
    output_path: &Path,
    control: &DownloadTransferControl,
) -> Result<crate::media::MediaProcessOutcome, DownloadEngineError> {
    if fragments.is_empty() {
        return Err(HttpDownloadError::InvalidDashManifest(
            "selected DASH track has no durable fragments",
        )
        .into());
    }
    let mut output = tokio::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output_path)
        .await?;
    for fragment in fragments {
        let mut input = tokio::fs::File::open(fragment).await?;
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            if let Some(directive) = hls_stop_directive(control) {
                drop(output);
                remove_async_file_if_exists(output_path).await?;
                return Ok(match directive {
                    TransferDirective::Pause => crate::media::MediaProcessOutcome::Paused,
                    TransferDirective::Cancel => crate::media::MediaProcessOutcome::Cancelled,
                    TransferDirective::Running => unreachable!("stop helper returned Running"),
                });
            }
            let read = input.read(&mut buffer).await?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read]).await?;
            tokio::task::yield_now().await;
        }
    }
    output.flush().await?;
    output.sync_all().await?;
    Ok(crate::media::MediaProcessOutcome::Completed)
}

#[allow(clippy::too_many_arguments)]
async fn complete_dash_finalize_stop(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    operation: &'static str,
    ffmpeg_version_status: &'static str,
    finalize_started_at: Instant,
    task_started_at: Instant,
    downloaded_bytes: u64,
    outcome: crate::media::MediaProcessOutcome,
    temporary_path: &Path,
    working_directory: &Path,
) -> Result<(), DownloadEngineError> {
    let stop_result = async {
        remove_async_file_if_exists(temporary_path).await?;
        let result = match outcome {
            crate::media::MediaProcessOutcome::Paused => (DownloadStatus::Paused, "paused"),
            crate::media::MediaProcessOutcome::Cancelled => {
                match tokio::fs::remove_dir_all(working_directory).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(DownloadEngineError::Io(error)),
                }
                inner.store.clear_dash_transfer_plan(task_id)?;
                (DownloadStatus::Cancelled, "cancelled")
            }
            crate::media::MediaProcessOutcome::Completed => {
                unreachable!("completed finalization is not a stop outcome")
            }
        };
        transition_dash_stopped(inner, task_id, result.0, downloaded_bytes)?;
        Ok::<_, DownloadEngineError>(result)
    }
    .await;
    let (status, outcome_name) = match stop_result {
        Ok(result) => result,
        Err(error) => {
            log_dash_finalize_failure(
                task_id,
                correlation_id,
                operation,
                ffmpeg_version_status,
                finalize_started_at,
                &error,
            );
            return Err(error);
        }
    };
    info!(
        event = "download.dash_finalize.completed",
        %correlation_id,
        ?task_id,
        operation,
        outcome = outcome_name,
        ffmpeg_version_status,
        exit_code = Option::<i32>::None,
        downloaded_bytes,
        elapsed_ms = finalize_started_at.elapsed().as_millis(),
        "DASH finalization stopped by task control"
    );
    info!(
        event = if status == DownloadStatus::Paused {
            "download.dash_transfer.paused"
        } else {
            "download.dash_transfer.cancelled"
        },
        %correlation_id,
        ?task_id,
        downloaded_bytes,
        elapsed_ms = task_started_at.elapsed().as_millis(),
        "DASH media transfer stopped during finalization"
    );
    Ok(())
}

fn log_dash_finalize_failure(
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    operation: &'static str,
    ffmpeg_version_status: &'static str,
    started_at: Instant,
    error: &DownloadEngineError,
) {
    warn!(
        event = "download.dash_finalize.failed",
        %correlation_id,
        ?task_id,
        operation,
        ffmpeg_version_status,
        exit_code = Option::<i32>::None,
        error_kind = match error {
            DownloadEngineError::Http(_) => "artifact_commit",
            DownloadEngineError::Io(_) => "filesystem",
            DownloadEngineError::Storage(_) => "storage",
            DownloadEngineError::Task(_) => "state",
            _ => "finalize",
        },
        native_error_code = error.native_os_error(),
        elapsed_ms = started_at.elapsed().as_millis(),
        "DASH finalization failed"
    );
}

fn transition_dash_stopped(
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
        task: Box::new(state.task.clone()),
    });
    Ok(())
}

fn hls_plan_is_compatible(
    stored: &crate::storage::StoredHlsTransferPlan,
    current: &crate::hls::HlsMediaPlaylist,
) -> bool {
    stored_hls_plan_is_valid(stored)
        && stored.media_sequence == current.media_sequence
        && stored.segments.len() == current.segments.len()
        && stored
            .segments
            .iter()
            .zip(&current.segments)
            .all(|(stored, current)| {
                stored.media.index == current.index
                    && stored.media.sequence == current.sequence
                    && stored.media.duration_millis == current.duration_millis
                    && hls_encryption_is_compatible(&stored.media.encryption, &current.encryption)
            })
}

fn hls_encryption_is_compatible(
    stored: &crate::hls::HlsSegmentEncryption,
    current: &crate::hls::HlsSegmentEncryption,
) -> bool {
    match (stored, current) {
        (crate::hls::HlsSegmentEncryption::None, crate::hls::HlsSegmentEncryption::None) => true,
        (
            crate::hls::HlsSegmentEncryption::Aes128 {
                key_slot: stored_slot,
                explicit_iv: stored_iv,
                ..
            },
            crate::hls::HlsSegmentEncryption::Aes128 {
                key_slot: current_slot,
                explicit_iv: current_iv,
                ..
            },
        ) => stored_slot == current_slot && stored_iv == current_iv,
        _ => false,
    }
}

fn stored_hls_plan_is_valid(plan: &crate::storage::StoredHlsTransferPlan) -> bool {
    if plan.segments.is_empty()
        || plan.segments.len() > crate::hls::MAX_HLS_MEDIA_SEGMENTS
        || plan.final_url.len() > crate::MAX_HLS_VARIANT_URL_BYTES
        || url::Url::parse(&plan.final_url)
            .ok()
            .is_none_or(|url| !matches!(url.scheme(), "http" | "https"))
    {
        return false;
    }
    let mut aggregate_url_bytes = 0_usize;
    let mut key_slots = HashMap::<u32, (&str, Option<[u8; 16]>)>::new();
    plan.segments
        .iter()
        .enumerate()
        .all(|(expected_index, segment)| {
            aggregate_url_bytes = aggregate_url_bytes.saturating_add(segment.media.url.len());
            let encryption_is_valid = match &segment.media.encryption {
                crate::hls::HlsSegmentEncryption::None => true,
                crate::hls::HlsSegmentEncryption::Aes128 {
                    key_url,
                    key_slot,
                    explicit_iv,
                } => {
                    aggregate_url_bytes = aggregate_url_bytes.saturating_add(key_url.len());
                    if let Some((known_url, known_iv)) = key_slots.get(key_slot) {
                        *known_url == key_url && *known_iv == *explicit_iv
                    } else {
                        key_slots.insert(*key_slot, (key_url, *explicit_iv));
                        key_url.len() <= crate::MAX_HLS_VARIANT_URL_BYTES
                            && url::Url::parse(key_url)
                                .ok()
                                .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
                    }
                }
            };
            encryption_is_valid
                && usize::try_from(segment.media.index).ok() == Some(expected_index)
                && plan
                    .media_sequence
                    .checked_add(u64::from(segment.media.index))
                    == Some(segment.media.sequence)
                && segment.media.url.len() <= crate::MAX_HLS_VARIANT_URL_BYTES
                && aggregate_url_bytes <= crate::hls::MAX_HLS_MEDIA_URL_BYTES
                && url::Url::parse(&segment.media.url)
                    .ok()
                    .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
        })
}

fn hls_stop_directive(control: &DownloadTransferControl) -> Option<TransferDirective> {
    let directive = control.current_directive();
    (directive != TransferDirective::Running).then_some(directive)
}

async fn remove_hls_cancel_artifact(
    path: &Path,
    artifact_kind: &'static str,
    directory: bool,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    started_at: Instant,
) -> Result<(), std::io::Error> {
    let result = if directory {
        tokio::fs::remove_dir_all(path).await
    } else {
        tokio::fs::remove_file(path).await
    };
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            warn!(
                event = "download.hls_cleanup.failed",
                %correlation_id,
                ?task_id,
                reason = "cancelled",
                artifact_kind,
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS cancellation cleanup failed"
            );
            Err(error)
        }
    }
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

#[allow(clippy::too_many_arguments)]
fn spawn_dash_fragment_transfer(
    transfers: &mut tokio::task::JoinSet<DashFragmentTransferResult>,
    inner: &Arc<DownloadEngineInner>,
    http: &HttpDownloadClient,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    control: &DownloadTransferControl,
    working_directory: &Path,
    durable_completed_bytes: &Arc<AtomicU64>,
    active_progress: &Arc<Mutex<HashMap<u32, u64>>>,
    active_connections: &Arc<AtomicU16>,
    index: usize,
    kind: crate::dash::DashTrackKind,
    segment_index: Option<u32>,
    resource_url: String,
) {
    let http = http.clone();
    let request = request.clone();
    let control = control.clone();
    let progress_inner = Arc::clone(inner);
    let durable_completed_bytes = Arc::clone(durable_completed_bytes);
    let active_progress = Arc::clone(active_progress);
    let active_connections = Arc::clone(active_connections);
    let staging_path = working_directory.join(format!("{index:08}.part"));
    let completed_path = working_directory.join(format!("{index:08}.fragment"));
    let progress_key = u32::try_from(index).unwrap_or(u32::MAX);
    active_connections.fetch_add(1, Ordering::AcqRel);
    transfers.spawn(async move {
        let mut attempt = 1_u32;
        let outcome = loop {
            let callback_inner = Arc::clone(&progress_inner);
            let callback_progress = Arc::clone(&active_progress);
            let callback_completed = Arc::clone(&durable_completed_bytes);
            let callback_active = Arc::clone(&active_connections);
            let outcome = http
                .download_hls_segment(
                    &request,
                    &resource_url,
                    &staging_path,
                    &control,
                    move |fragment_bytes| {
                        let active_bytes = callback_progress
                            .lock()
                            .map(|mut values| {
                                values.insert(progress_key, fragment_bytes);
                                values.values().copied().sum::<u64>()
                            })
                            .unwrap_or_default();
                        publish_transfer_progress(
                            &callback_inner,
                            task_id,
                            callback_completed
                                .load(Ordering::Acquire)
                                .saturating_add(active_bytes),
                            None,
                            callback_active.load(Ordering::Acquire),
                            None,
                            None,
                        );
                    },
                )
                .await;
            match outcome {
                Err(error) if should_retry_transfer(attempt, &error) => {
                    if let Ok(mut values) = active_progress.lock() {
                        values.insert(progress_key, 0);
                    }
                    if let Err(cleanup_error) = remove_async_file_if_exists(&staging_path).await {
                        break Err(cleanup_error.into());
                    }
                    let delay = transfer_retry_delay(attempt);
                    warn!(
                        event = "download.dash_fragment_retry.scheduled",
                        %correlation_id,
                        ?task_id,
                        fragment_index = index,
                        failed_attempt = attempt,
                        next_attempt = attempt + 1,
                        delay_ms = delay.as_millis(),
                        error_kind = error.kind(),
                        "transient DASH fragment failure scheduled for retry"
                    );
                    attempt += 1;
                    let _ = control.wait_for_stop_or_timeout(delay).await;
                }
                terminal => {
                    if !matches!(terminal, Ok(HlsSegmentDownloadOutcome::Completed { .. }))
                        && let Err(cleanup_error) = remove_async_file_if_exists(&staging_path).await
                    {
                        break Err(cleanup_error.into());
                    }
                    break terminal;
                }
            }
        };
        active_connections.fetch_sub(1, Ordering::AcqRel);
        (
            index,
            kind,
            segment_index,
            staging_path,
            completed_path,
            outcome,
        )
    });
}

type HlsKeyCache = Arc<HashMap<u32, Arc<tokio::sync::Mutex<Option<[u8; 16]>>>>>;

async fn cleanup_hls_segment_attempt_files(
    staging_path: &Path,
    ciphertext_path: &Path,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    segment_index: u32,
    reason: &'static str,
) -> Result<(), HttpDownloadError> {
    let started_at = Instant::now();
    let mut first_error = None;
    for (path, artifact_kind) in [
        (staging_path, "segment_staging"),
        (ciphertext_path, "segment_ciphertext"),
    ] {
        if let Err(error) = tokio::fs::remove_file(path).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                event = "download.hls_cleanup.failed",
                %correlation_id,
                ?task_id,
                segment_index,
                reason,
                artifact_kind,
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS segment staging cleanup failed"
            );
            first_error.get_or_insert(error);
        }
    }
    match first_error {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn download_hls_segment_attempt<F>(
    http: &HttpDownloadClient,
    request: &DownloadRequest,
    segment: &crate::hls::HlsMediaSegment,
    staging_path: &PathBuf,
    ciphertext_path: &PathBuf,
    control: &DownloadTransferControl,
    key_cache: &HlsKeyCache,
    on_progress: F,
) -> Result<HlsSegmentDownloadOutcome, HttpDownloadError>
where
    F: FnMut(u64),
{
    let crate::hls::HlsSegmentEncryption::Aes128 {
        key_url,
        key_slot,
        explicit_iv,
    } = &segment.encryption
    else {
        return http
            .download_hls_segment(request, &segment.url, staging_path, control, on_progress)
            .await;
    };
    let cache = key_cache
        .get(key_slot)
        .ok_or(HttpDownloadError::HlsResourceChanged)?;
    let key = {
        let mut cached = cache.lock().await;
        if let Some(key) = *cached {
            key
        } else {
            match http.fetch_hls_aes128_key(request, key_url, control).await? {
                HlsKeyFetchOutcome::Loaded(key) => {
                    *cached = Some(key);
                    key
                }
                HlsKeyFetchOutcome::Paused => return Ok(HlsSegmentDownloadOutcome::Paused),
                HlsKeyFetchOutcome::Cancelled => {
                    return Ok(HlsSegmentDownloadOutcome::Cancelled);
                }
            }
        }
    };
    let downloaded = http
        .download_hls_segment(request, &segment.url, ciphertext_path, control, on_progress)
        .await?;
    if !matches!(downloaded, HlsSegmentDownloadOutcome::Completed { .. }) {
        return Ok(downloaded);
    }
    let iv = explicit_iv.unwrap_or_else(|| hls_sequence_iv(segment.sequence));
    let decrypted =
        decrypt_hls_aes128_segment(ciphertext_path, staging_path, key, iv, control).await;
    match tokio::fs::remove_file(ciphertext_path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    decrypted
}

#[allow(clippy::too_many_arguments)]
fn spawn_hls_segment_transfer(
    transfers: &mut tokio::task::JoinSet<HlsSegmentTransferResult>,
    inner: &Arc<DownloadEngineInner>,
    http: &HttpDownloadClient,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    control: &DownloadTransferControl,
    working_directory: &Path,
    durable_completed_bytes: &Arc<AtomicU64>,
    active_progress: &Arc<Mutex<HashMap<u32, u64>>>,
    active_connections: &Arc<AtomicU16>,
    key_cache: &HlsKeyCache,
    segment: crate::hls::HlsMediaSegment,
) {
    let http = http.clone();
    let request = request.clone();
    let control = control.clone();
    let progress_inner = Arc::clone(inner);
    let durable_completed_bytes = Arc::clone(durable_completed_bytes);
    let active_progress = Arc::clone(active_progress);
    let active_connections = Arc::clone(active_connections);
    let key_cache = Arc::clone(key_cache);
    let staging_path = working_directory.join(format!("{:08}.part", segment.index));
    let ciphertext_path = working_directory.join(format!("{:08}.encrypted.part", segment.index));
    let completed_path = working_directory.join(format!("{:08}.segment", segment.index));
    active_connections.fetch_add(1, Ordering::AcqRel);
    transfers.spawn(async move {
        let mut attempt = 1_u32;
        let outcome = loop {
            let callback_inner = Arc::clone(&progress_inner);
            let callback_progress = Arc::clone(&active_progress);
            let callback_completed = Arc::clone(&durable_completed_bytes);
            let callback_active = Arc::clone(&active_connections);
            let outcome = download_hls_segment_attempt(
                &http,
                &request,
                &segment,
                &staging_path,
                &ciphertext_path,
                &control,
                &key_cache,
                move |segment_bytes| {
                    let active_bytes = callback_progress
                        .lock()
                        .map(|mut values| {
                            values.insert(segment.index, segment_bytes);
                            values.values().copied().sum::<u64>()
                        })
                        .unwrap_or_default();
                    publish_transfer_progress(
                        &callback_inner,
                        task_id,
                        callback_completed
                            .load(Ordering::Acquire)
                            .saturating_add(active_bytes),
                        None,
                        callback_active.load(Ordering::Acquire),
                        None,
                        None,
                    );
                },
            )
            .await;
            match outcome {
                Err(error) if should_retry_transfer(attempt, &error) => {
                    if let Ok(mut values) = active_progress.lock() {
                        values.insert(segment.index, 0);
                    }
                    let delay = transfer_retry_delay(attempt);
                    if let Err(cleanup_error) = cleanup_hls_segment_attempt_files(
                        &staging_path,
                        &ciphertext_path,
                        task_id,
                        correlation_id,
                        segment.index,
                        "segment_retry",
                    )
                    .await
                    {
                        break Err(cleanup_error);
                    }
                    warn!(
                        event = "download.hls_segment_retry.scheduled",
                        %correlation_id,
                        ?task_id,
                        segment_index = segment.index,
                        failed_attempt = attempt,
                        next_attempt = attempt + 1,
                        delay_ms = delay.as_millis(),
                        error_kind = error.kind(),
                        "transient HLS segment failure scheduled for retry"
                    );
                    attempt += 1;
                    let _ = control.wait_for_stop_or_timeout(delay).await;
                }
                terminal => {
                    if !matches!(terminal, Ok(HlsSegmentDownloadOutcome::Completed { .. }))
                        && let Err(cleanup_error) = cleanup_hls_segment_attempt_files(
                            &staging_path,
                            &ciphertext_path,
                            task_id,
                            correlation_id,
                            segment.index,
                            "segment_terminal",
                        )
                        .await
                    {
                        break Err(cleanup_error);
                    }
                    break terminal;
                }
            }
        };
        active_connections.fetch_sub(1, Ordering::AcqRel);
        (segment, staging_path, completed_path, outcome)
    });
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

#[allow(clippy::too_many_arguments)]
async fn run_single_stream_task(
    inner: &Arc<DownloadEngineInner>,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    probe: &DownloadProbe,
    control: &DownloadTransferControl,
    http: &HttpDownloadClient,
    reason: &'static str,
) -> Result<(), DownloadEngineError> {
    let started_at = Instant::now();
    info!(
        event = "download.single_stream.started",
        %correlation_id,
        ?task_id,
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        reason,
        expected_bytes = probe.total_bytes.unwrap_or_default(),
        "single-stream transfer started"
    );
    let mut attempt = 1_u32;
    let outcome = loop {
        let progress_inner = Arc::clone(inner);
        let outcome = http
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
            .await;
        match outcome {
            Err(error) if should_retry_transfer(attempt, &error) => {
                let delay = transfer_retry_delay(attempt);
                warn!(
                    event = "download.single_stream_retry.scheduled",
                    %correlation_id,
                    ?task_id,
                    reason,
                    failed_attempt = attempt,
                    next_attempt = attempt + 1,
                    delay_ms = delay.as_millis(),
                    error_kind = error.kind(),
                    "transient single-stream failure scheduled for retry"
                );
                attempt += 1;
                if let Some(directive) = control.wait_for_stop_or_timeout(delay).await {
                    let (downloaded_bytes, temporary_path) = {
                        let tasks = inner.lock_tasks()?;
                        let state = tasks
                            .get(&task_id)
                            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
                        (
                            state.transfer.downloaded_bytes,
                            state
                                .transfer
                                .temporary_path
                                .clone()
                                .ok_or(DownloadEngineError::MissingTemporaryPath(task_id))?,
                        )
                    };
                    break match directive {
                        TransferDirective::Pause => Ok(PreparedDownloadOutcome::Paused {
                            temporary_path,
                            downloaded_bytes,
                        }),
                        TransferDirective::Cancel => {
                            remove_path_if_exists(&temporary_path)?;
                            Ok(PreparedDownloadOutcome::Cancelled { downloaded_bytes })
                        }
                        TransferDirective::Running => {
                            unreachable!("wait_for_stop_or_timeout only returns stop directives")
                        }
                    };
                }
            }
            terminal => break terminal,
        }
    }?;

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
                    task: Box::new(state.task.clone()),
                });
            }
            http.commit_prepared(ready).await?;
            commit_completed_state(inner, task_id)?;
            info!(
                event = "download.single_stream.completed",
                %correlation_id,
                ?task_id,
                proxy_mode = proxy_mode_name(http.network_proxy_mode()),
                proxy_source = http.network_proxy_source(),
                reason,
                downloaded_bytes,
                elapsed_ms = started_at.elapsed().as_millis(),
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
                task: Box::new(state.task.clone()),
            });
            info!(
                event = "download.single_stream.paused",
                %correlation_id,
                ?task_id,
                proxy_mode = proxy_mode_name(http.network_proxy_mode()),
                proxy_source = http.network_proxy_source(),
                reason,
                downloaded_bytes,
                elapsed_ms = started_at.elapsed().as_millis(),
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
                task: Box::new(state.task.clone()),
            });
            info!(
                event = "download.single_stream.cancelled",
                %correlation_id,
                ?task_id,
                proxy_mode = proxy_mode_name(http.network_proxy_mode()),
                proxy_source = http.network_proxy_source(),
                reason,
                downloaded_bytes,
                elapsed_ms = started_at.elapsed().as_millis(),
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
    http: &HttpDownloadClient,
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
    let http = http.clone();
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
                Err(error) if should_retry_transfer(attempt, &error) => {
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
                    let delay = transfer_retry_delay(attempt);
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
                    let _stop_directive = task_control.wait_for_stop_or_timeout(delay).await;
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
    http: &HttpDownloadClient,
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
            http,
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
    let mut pending_shrink: Option<u32> = None;
    let mut adaptive_growth_remaining = 0_usize;
    let initial_segment_count = u16::try_from(segments.len()).unwrap_or(u16::MAX);
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
                let observed = adaptive.observe(
                    speed,
                    current_segments,
                    retry_observed.load(Ordering::Acquire),
                );
                let decision = if matches!(observed, AdaptiveConnectionDecision::Hold) {
                    adaptive.observe_shrink(
                        speed,
                        current_segments,
                        initial_segment_count,
                    )
                } else {
                    observed
                };
                match decision {
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
                    AdaptiveConnectionDecision::Shrink => {
                        // Shed one segment: pick the one with the most
                        // remaining bytes, cancel its worker, checkpoint it
                        // back to Waiting, merge its untouched range into the
                        // neighbouring Waiting segment and respawn that
                        // neighbour. segment_count is decremented when the
                        // joined worker is reaped below.
                        if let Some((candidate_index, _remaining)) = select_rebalance_candidate(
                            inner,
                            task_id,
                            &worker_controls,
                            &progress_by_segment,
                            adaptive_min_remaining_bytes,
                        )? && let Some(candidate_control) = worker_controls.get(&candidate_index)
                        {
                            candidate_control.request_rebalance();
                            pending_shrink = Some(candidate_index);
                            info!(
                                event = "download.adaptive_segments.shrink_requested",
                                %correlation_id,
                                ?task_id,
                                segment_index = candidate_index,
                                sampled_bytes_per_second = speed,
                                current_segments,
                                initial_segments = initial_segment_count,
                                "stable slow transfer shed one adaptive segment"
                            );
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
        let shrink_requested = pending_shrink == Some(segment.index());
        if shrink_requested {
            pending_shrink = None;
        }
        let skip_shrink = pending_shrink.is_none()
            || paused
            || cancelled
            || first_error.is_some()
            || !u16::try_from(segment_count)
                .map(|count| count > initial_segment_count)
                .unwrap_or(false);
        // Adaptive shrink: the shed worker exits through its cancel path with
        // its remaining range checkpointed back to Waiting; fold that range
        // into the neighbouring Waiting segment and respawn the neighbour.
        if shrink_requested && !skip_shrink {
            let Some(plan) = inner.store.load_transfer_plan(task_id)? else {
                info!(
                    event = "download.adaptive_segments.shrink_missing_plan",
                    %correlation_id,
                    ?task_id,
                    "adaptive shrink skipped: transfer plan disappeared"
                );
                // The shed worker simply exits; the segment stays as waiting
                // state and the queue refills it on the next run.
                continue;
            };
            let cancelled_segment = plan
                .segments()
                .iter()
                .find(|candidate| candidate.index() == segment.index())
                .cloned();
            if let Some(cancelled) = cancelled_segment
                && cancelled.state == DownloadSegmentState::Waiting
            {
                let neighbour = plan
                    .segments()
                    .iter()
                    .filter(|candidate| {
                        candidate.index() != segment.index()
                            && candidate.state == DownloadSegmentState::Waiting
                            && (candidate.end_byte().saturating_add(1) == cancelled.start_byte()
                                || cancelled.end_byte().saturating_add(1) == candidate.start_byte())
                    })
                    .max_by_key(|candidate| candidate.len())
                    .cloned();
                if let Some(neighbour) = neighbour {
                    let Some(mut merge_plan) = inner.store.load_transfer_plan(task_id)? else {
                        info!(
                            event = "download.adaptive_segments.shrink_missing_plan",
                            %correlation_id,
                            ?task_id,
                            "adaptive shrink merge skipped: transfer plan disappeared"
                        );
                        continue;
                    };
                    let neighbour_control = worker_controls.get(&neighbour.index()).cloned();
                    match merge_plan.merge_waiting_segment(segment.index(), neighbour.index()) {
                        Ok(()) => {
                            inner.persist_transfer_plan_change(task_id, &merge_plan)?;
                            segment_count = segment_count.saturating_sub(1);
                            info!(
                                event = "download.adaptive_segments.shrink_merged",
                                %correlation_id,
                                ?task_id,
                                cancelled_segment_index = segment.index(),
                                merged_segment_index = neighbour.index(),
                                current_segments = segment_count,
                                "adaptive shrink merged the shed range into its neighbour"
                            );
                            if let Some(neighbour_control) = neighbour_control {
                                neighbour_control.request_rebalance();
                            }
                            adaptive.record_completed_growth(
                                u16::try_from(segment_count).unwrap_or(u16::MAX),
                            );
                        }
                        Err(error) => {
                            info!(
                                event = "download.adaptive_segments.shrink_merge_failed",
                                %correlation_id,
                                ?task_id,
                                error = %error,
                                "adaptive shrink could not merge; keeping the segment"
                            );
                        }
                    }
                }
            }
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
                                    http,
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
                                http,
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
                http,
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
        if use_if_range && error.allows_fresh_segmented_restart_after_resume() {
            warn!(
                event = "download.segment_resume.fresh_restart",
                %correlation_id,
                ?task_id,
                reason = error.kind(),
                segment_count = final_plan.segments().len(),
                discarded_durable_bytes = durable_downloaded_bytes,
                discarded_observed_bytes = observed_downloaded_bytes,
                "conditional Range resume was rejected; discarding the old segmented artifact and retrying with fresh Range requests"
            );
            inner.store.clear_transfer_plan(task_id)?;
            remove_path_if_exists(&temporary_path)?;
            {
                let mut tasks = inner.lock_tasks()?;
                let state = tasks
                    .get_mut(&task_id)
                    .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
                state.transfer.downloaded_bytes = 0;
                inner.persist(state)?;
            }
            return Box::pin(run_segmented_task(
                inner,
                task_id,
                correlation_id,
                request,
                probe,
                control,
                http,
                temporary_path,
                final_path,
            ))
            .await;
        }
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
                http,
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
            task: Box::new(state.task.clone()),
        });
    }
    http.commit_prepared(PreparedDownload {
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
const MAX_TRANSFER_ATTEMPTS: u32 = 3;
const TRANSFER_RETRY_BASE_DELAY: Duration = Duration::from_millis(100);

fn transfer_retry_delay(failed_attempt: u32) -> Duration {
    TRANSFER_RETRY_BASE_DELAY.saturating_mul(1 << failed_attempt.saturating_sub(1).min(6))
}

fn should_retry_transfer(failed_attempt: u32, error: &HttpDownloadError) -> bool {
    failed_attempt < MAX_TRANSFER_ATTEMPTS && error.is_retryable_transfer()
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
        task: Box::new(state.task.clone()),
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
        task: Box::new(state.task.clone()),
    });
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
    state.control = None;
    let _ = inner.persist(state);
    inner.emit(DownloadEvent::TaskChanged {
        task: Box::new(state.task.clone()),
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
            .and_then(|probe| probe.total_bytes)
            .or(state.task.request().expected_total_bytes),
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

fn target_reserved_by_tasks(
    tasks: &HashMap<DownloadTaskId, EngineTaskState>,
    candidate: &Path,
) -> bool {
    let candidate = windows_target_key(candidate);
    tasks.values().any(|state| {
        if state.task.status().is_terminal() {
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

fn hls_working_directory(request: &DownloadRequest, task_id: DownloadTaskId) -> PathBuf {
    request
        .save_directory
        .join(format!(".nxtools-hls-{}", task_id.as_uuid()))
}

fn dash_working_directory(request: &DownloadRequest, task_id: DownloadTaskId) -> PathBuf {
    request
        .save_directory
        .join(format!(".nxtools-dash-{}", task_id.as_uuid()))
}

fn remove_dash_owned_artifacts(
    request: &DownloadRequest,
    task_id: DownloadTaskId,
    temporary_path: Option<&Path>,
    correlation_id: uuid::Uuid,
    reason: &'static str,
    started_at: Instant,
) -> Result<(), std::io::Error> {
    let working_directory = dash_working_directory(request, task_id);
    let directory_result = match std::fs::remove_dir_all(&working_directory) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            warn!(
                event = "download.dash_cleanup.failed",
                %correlation_id,
                ?task_id,
                reason,
                artifact_kind = "fragment_directory",
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "DASH task-owned artifact cleanup failed"
            );
            Err(error)
        }
    };
    let temporary_result = temporary_path.map_or(Ok(()), |path| match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            warn!(
                event = "download.dash_cleanup.failed",
                %correlation_id,
                ?task_id,
                reason,
                artifact_kind = "mux_temporary",
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "DASH task-owned artifact cleanup failed"
            );
            Err(error)
        }
    });
    directory_result?;
    temporary_result
}

fn remove_hls_owned_artifacts(
    request: &DownloadRequest,
    task_id: DownloadTaskId,
    temporary_path: Option<&Path>,
    correlation_id: uuid::Uuid,
    reason: &'static str,
    started_at: Instant,
) -> Result<(), std::io::Error> {
    let working_directory = hls_working_directory(request, task_id);
    let segment_cleanup = remove_hls_artifact(
        &working_directory,
        "segment_directory",
        true,
        task_id,
        correlation_id,
        reason,
        started_at,
    );
    let merge_cleanup = temporary_path.map_or(Ok(()), |path| {
        remove_hls_artifact(
            path,
            "merge_temporary",
            false,
            task_id,
            correlation_id,
            reason,
            started_at,
        )
    });
    segment_cleanup?;
    merge_cleanup
}

#[allow(clippy::too_many_arguments)]
fn remove_hls_artifact(
    path: &Path,
    artifact_kind: &'static str,
    directory: bool,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    reason: &'static str,
    started_at: Instant,
) -> Result<(), std::io::Error> {
    let result = if directory {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            warn!(
                event = "download.hls_cleanup.failed",
                %correlation_id,
                ?task_id,
                reason,
                artifact_kind,
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS task-owned artifact cleanup failed"
            );
            Err(error)
        }
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

fn probe_error_kind(error: &DownloadEngineError) -> &'static str {
    match error {
        DownloadEngineError::InvalidRequest(_) => "invalid_request",
        DownloadEngineError::Http(error) => error.kind(),
        DownloadEngineError::Runtime(_) => "runtime_unavailable",
        _ => "internal",
    }
}

fn import_blob_artifact(source: &Path, destination: &Path) -> Result<(), DownloadEngineError> {
    match std::fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(rename_error) => {
            let mut source_file = std::fs::File::open(source)
                .map_err(|error| DownloadEngineError::Runtime(error.to_string()))?;
            let mut destination_file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .map_err(|error| DownloadEngineError::Runtime(error.to_string()))?;
            if let Err(error) = std::io::copy(&mut source_file, &mut destination_file)
                .and_then(|_| destination_file.sync_all())
            {
                let _ = std::fs::remove_file(destination);
                return Err(DownloadEngineError::Runtime(format!(
                    "could not import browser Blob after rename failed ({rename_error}): {error}"
                )));
            }
            if let Err(error) = std::fs::remove_file(source) {
                let _ = std::fs::remove_file(destination);
                return Err(DownloadEngineError::Runtime(format!(
                    "could not remove staged browser Blob after copy: {error}"
                )));
            }
            Ok(())
        }
    }
}

#[derive(Debug)]
enum BrowserBlobPersistenceError {
    Persistence(DownloadStorageError),
    Rollback {
        persistence: DownloadStorageError,
        rollback: Box<DownloadEngineError>,
    },
}

fn persist_browser_blob_with_compensation(
    staged_path: &Path,
    final_path: &Path,
    persist: impl FnOnce() -> Result<BrowserHandoffSave, DownloadStorageError>,
) -> Result<BrowserHandoffSave, BrowserBlobPersistenceError> {
    match persist() {
        Ok(saved) => Ok(saved),
        Err(persistence) => match import_blob_artifact(final_path, staged_path) {
            Ok(()) => Err(BrowserBlobPersistenceError::Persistence(persistence)),
            Err(rollback) => Err(BrowserBlobPersistenceError::Rollback {
                persistence,
                rollback: Box::new(rollback),
            }),
        },
    }
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
    #[error("download queue concurrency must be at least one")]
    InvalidQueueConcurrency,
    #[error("browser batch must contain between 1 and {maximum} items, got {count}")]
    InvalidBrowserBatchSize { count: usize, maximum: usize },
    #[error("browser batch contains a duplicate handoff ID")]
    DuplicateBrowserBatchHandoff,
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
            Self::InvalidRequest(_)
            | Self::InvalidBrowserBatchSize { .. }
            | Self::DuplicateBrowserBatchHandoff => (DownloadFaultCode::InvalidRequest, false),
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
            | Self::InvalidQueueConcurrency
            | Self::InvalidQueueMove(_)
            | Self::InvalidAction { .. } => (DownloadFaultCode::InvalidState, false),
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
                (DownloadFaultCode::Storage, false)
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
    use super::*;
    use crate::http::InjectedArtifactFault;
    use crate::{ResourceValidator, SegmentedArtifactError};
    use std::io::{Read, Write};
    use std::net::TcpListener;

    #[test]
    fn configured_ffmpeg_is_applied_to_the_live_engine_without_restart() {
        let directory = tempfile::tempdir().expect("create configured FFmpeg directory");
        let executable = directory.path().join("ffmpeg-configured.exe");
        std::fs::write(&executable, b"fixture").expect("write configured FFmpeg executable");
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
            concatenate_dash_fragments(&[worker_fragment], &worker_output, &worker_control).await
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
                save_directory_explicit: false,
                file_name: Some("must-not-exist.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
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
