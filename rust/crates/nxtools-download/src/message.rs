use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use crate::{
    DownloadQueue, DownloadRequest, DownloadStatus, DownloadTask, DownloadTaskId, RequestOrigin,
    ResourceValidator,
};

pub const DOWNLOAD_COMMAND_PROTOCOL_VERSION: u16 = 1;

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
    Task {
        task_id: DownloadTaskId,
        action: DownloadTaskAction,
    },
    SetGlobalSpeedLimit {
        bytes_per_second: Option<u64>,
    },
    Snapshot {
        query: DownloadQuery,
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
    Accepted { task_id: Option<DownloadTaskId> },
    Probe(DownloadProbe),
    Snapshot(DownloadSnapshot),
    Rejected { fault: DownloadFault },
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
