use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    DownloadQueue, DownloadRequest, DownloadSchedule, DownloadStatus, DownloadTask, DownloadTaskId,
    RequestOrigin, ResourceValidator,
};

pub const DOWNLOAD_COMMAND_PROTOCOL_VERSION: u16 = 3;

pub const MAX_BROWSER_BATCH_ITEMS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserBlob {
    pub id: Uuid,
    pub file_name: String,
    pub media_type: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserDownloadSubmission {
    pub handoff_id: Uuid,
    pub request: DownloadRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserBatchSubmissionResult {
    pub handoff_id: Uuid,
    pub task_id: Option<DownloadTaskId>,
    pub duplicate: bool,
    pub fault: Option<DownloadFault>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadCommandEnvelope {
    pub protocol_version: u16,
    pub request_id: Uuid,
    pub origin: RequestOrigin,
    #[serde(flatten)]
    pub command: DownloadCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "command", content = "data", rename_all = "snake_case")]
pub enum DownloadCommand {
    Probe {
        request: DownloadRequest,
    },
    Submit {
        request: DownloadRequest,
        #[serde(default)]
        mode: DownloadSubmitMode,
    },
    BrowserSubmit {
        handoff_id: Uuid,
        request: DownloadRequest,
        #[serde(default)]
        mode: DownloadSubmitMode,
    },
    BrowserBatchSubmit {
        submissions: Vec<BrowserDownloadSubmission>,
    },
    BrowserOpenDialog {
        handoff_id: Uuid,
        request: DownloadRequest,
    },
    BrowserOpenBlobDialog {
        handoff_id: Uuid,
        blob: BrowserBlob,
        save_directory: std::path::PathBuf,
    },
    BrowserImportBlob {
        handoff_id: Uuid,
        blob: BrowserBlob,
        save_directory: std::path::PathBuf,
        file_name: String,
    },
    BrowserDiscardBlob {
        handoff_id: Uuid,
        blob_id: Uuid,
    },
    Task {
        task_id: DownloadTaskId,
        action: DownloadTaskAction,
    },
    Batch {
        task_ids: Vec<DownloadTaskId>,
        action: DownloadBatchAction,
    },
    Queue {
        action: DownloadQueueAction,
    },
    SetGlobalSpeedLimit {
        bytes_per_second: Option<u64>,
    },
    Snapshot {
        query: DownloadQuery,
    },
    TaskDetail {
        task_id: DownloadTaskId,
    },
    Subscribe {
        after_sequence: Option<u64>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadSubmitMode {
    QueueOnly,
    #[default]
    StartNow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "data", rename_all = "snake_case")]
pub enum DownloadTaskAction {
    Pause,
    Resume,
    Cancel,
    SetSpeedLimit { bytes_per_second: Option<u64> },
    Remove { delete_artifact: bool },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadBatchAction {
    Pause,
    Resume,
    Cancel,
    Remove,
    RemoveWithArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "action", content = "data", rename_all = "snake_case")]
pub enum DownloadQueueAction {
    Move {
        queue_id: String,
        task_ids: Vec<DownloadTaskId>,
        placement: DownloadQueuePlacement,
    },
    SetMaxConcurrent {
        queue_id: String,
        max_concurrent_tasks: u16,
    },
    SetSchedule {
        queue_id: String,
        schedule: Option<DownloadSchedule>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadQueuePlacement {
    Top,
    Up,
    Down,
    Bottom,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DownloadQuery {
    #[serde(default)]
    pub statuses: Vec<DownloadStatus>,
    pub search: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadCommandResult {
    pub request_id: Uuid,
    #[serde(flatten)]
    pub outcome: DownloadCommandOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", content = "data", rename_all = "snake_case")]
pub enum DownloadCommandOutcome {
    Accepted {
        task_id: Option<DownloadTaskId>,
    },
    BrowserAccepted {
        task_id: DownloadTaskId,
        duplicate: bool,
    },
    BrowserBatchAccepted {
        results: Vec<BrowserBatchSubmissionResult>,
    },
    BrowserDialogAccepted {
        handoff_id: Uuid,
        duplicate: bool,
    },
    Batch {
        results: Vec<DownloadBatchItemResult>,
    },
    QueueChanged {
        queue: DownloadQueue,
    },
    Probe(DownloadProbe),
    Snapshot(DownloadSnapshot),
    TaskDetail(Box<DownloadTaskDetail>),
    Rejected {
        fault: DownloadFault,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadBatchItemResult {
    pub task_id: DownloadTaskId,
    pub fault: Option<DownloadFault>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadProbe {
    pub final_url: String,
    pub file_name: Option<String>,
    pub total_bytes: Option<u64>,
    pub supports_ranges: bool,
    pub content_type: Option<String>,
    pub validator: ResourceValidator,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadSnapshot {
    pub generation: u64,
    pub sequence: u64,
    pub tasks: Vec<DownloadTask>,
    pub queues: Vec<DownloadQueue>,
    pub aggregate_metrics: DownloadAggregateMetrics,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub task_details: Vec<crate::DownloadEngineTask>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadTaskDetail {
    pub task: crate::DownloadEngineTask,
    pub segments: Vec<crate::DownloadSegment>,
    pub timeline: Vec<DownloadTaskTimelineEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadTaskTimelineEntry {
    pub sequence: u64,
    pub occurred_at_unix_ms: i64,
    pub event: DownloadTaskTimelineEvent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum DownloadTaskTimelineEvent {
    TaskCreated,
    StatusChanged { status: DownloadStatus },
    TransferPlanChanged { segment_count: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct DownloadAggregateMetrics {
    pub active_tasks: u32,
    pub downloaded_bytes_per_second: u64,
    pub disk_write_bytes_per_second: u64,
    pub active_connections: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadEventEnvelope {
    pub sequence: u64,
    pub emitted_at_unix_ms: i64,
    #[serde(flatten)]
    pub event: DownloadEvent,
}

#[derive(Debug)]
pub struct DownloadEventSequencer {
    sequence: AtomicU64,
}

impl DownloadEventSequencer {
    pub const fn new(after_sequence: u64) -> Self {
        Self {
            sequence: AtomicU64::new(after_sequence),
        }
    }

    pub fn next(
        &self,
        emitted_at_unix_ms: i64,
        event: DownloadEvent,
    ) -> Result<DownloadEventEnvelope, DownloadEventSequenceError> {
        let previous = self
            .sequence
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |sequence| {
                sequence.checked_add(1)
            })
            .map_err(|_| DownloadEventSequenceError::Exhausted)?;
        Ok(DownloadEventEnvelope {
            sequence: previous + 1,
            emitted_at_unix_ms,
            event,
        })
    }

    pub fn current(&self) -> u64 {
        self.sequence.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DownloadEventSequenceError {
    #[error("download event sequence is exhausted")]
    Exhausted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", content = "data", rename_all = "snake_case")]
pub enum DownloadEvent {
    TaskChanged {
        task: DownloadTask,
    },
    TaskRemoved {
        task_id: DownloadTaskId,
    },
    TaskProgress {
        progress: DownloadProgress,
    },
    CommandRejected {
        request_id: Uuid,
        fault: DownloadFault,
    },
    SnapshotRequired {
        expected_sequence: u64,
        observed_sequence: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub task_id: DownloadTaskId,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub bytes_per_second: u64,
    pub active_connections: u16,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub segments: Vec<DownloadSegmentProgress>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadSegmentProgress {
    pub segment_index: u32,
    pub downloaded_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadFault {
    pub code: DownloadFaultCode,
    pub message: String,
    pub retryable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DownloadFaultCode {
    InvalidRequest,
    NotFound,
    InvalidState,
    Storage,
    Network,
    Protocol,
    Permission,
    DiskFull,
    FileInUse,
    Cancelled,
    Unavailable,
    Internal,
}
