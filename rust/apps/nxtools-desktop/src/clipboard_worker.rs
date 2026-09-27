use std::any::Any;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
    mpsc::{self, SyncSender},
};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use async_channel::{Receiver, Sender};
use nxtools_clipboard::{
    CaptureSource, ClipboardArchiveExportReport, ClipboardArchiveImportReport, ClipboardCategory,
    ClipboardDatabaseBackupReport, ClipboardDatabaseCompactionReport, ClipboardHistory,
    ClipboardHistoryCounts, ClipboardHistoryError, ClipboardHistoryPage, ClipboardNativeFormat,
    ClipboardPageCursor, ClipboardRetentionPolicy, ClipboardSearchScope,
    ClipboardStorageMigrationReport, FilesCapture, ImageCapture, TextCapture,
};
use nxtools_protocol::ClipboardCapture;
use sha2::{Digest, Sha256};
use tracing::{error, info, warn};

const CHANNEL_CAPACITY: usize = 256;
const RETENTION_INTERVAL_MS: i64 = 60 * 60 * 1000;
const LEGACY_RESET_MARKER_FILE: &str = ".nxtools-clipboard-legacy-reset-v1";

// The actor owns this state for its entire lifetime. The handle only mirrors the
// paths needed by callers to label maintenance operations and update configuration.
struct ClipboardWorkerState {
    history: ClipboardHistory,
    asset_directory: PathBuf,
    retention: ClipboardRetentionPolicy,
    last_cleanup_at_unix_ms: i64,
}

type HistoryOperation = Box<dyn FnOnce(&mut ClipboardWorkerState) -> Box<dyn Any + Send> + Send>;

#[derive(Clone)]
struct QueryCancellation {
    latest_generation: Arc<AtomicU64>,
    generation: u64,
}

/// Reply channel of a worker job. Blocking callers keep the `std::sync::mpsc` rendezvous while
/// async callers get an `async_channel` sender that the worker thread can fill without blocking.
enum JobReply {
    Blocking(SyncSender<Box<dyn Any + Send>>),
    Async(Sender<Box<dyn Any + Send>>),
}

impl JobReply {
    /// Delivers one job outcome from the worker thread. A closed receiver is not an error: the
    /// caller already gave up on the reply.
    fn deliver(self, reply: Box<dyn Any + Send>) {
        match self {
            Self::Blocking(sender) => {
                let _ = sender.send(reply);
            }
            Self::Async(sender) => {
                let _ = sender.try_send(reply);
            }
        }
    }
}

enum ClipboardWriteRequest {
    Job {
        operation: HistoryOperation,
        reply: JobReply,
        cancellation: Option<QueryCancellation>,
        cancelled_result: Option<Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>>,
    },
    Capture(ClipboardCapture),
    Cleanup {
        policy: ClipboardRetentionPolicy,
        now_unix_ms: i64,
    },
    SetRetention(ClipboardRetentionPolicy),
    Shutdown {
        reply: SyncSender<()>,
    },
}

#[derive(Debug, Clone)]
struct ClipboardStoragePaths {
    database_path: PathBuf,
    asset_directory: PathBuf,
}

#[derive(Clone)]
pub(crate) struct ClipboardCaptureWorkerHandle {
    sender: Sender<ClipboardWriteRequest>,
    paths: Arc<Mutex<ClipboardStoragePaths>>,
    query_generation: Arc<AtomicU64>,
    join: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl ClipboardCaptureWorkerHandle {
    pub(crate) fn spawn(
        database_path: impl AsRef<Path>,
        asset_directory: impl AsRef<Path>,
        retention: ClipboardRetentionPolicy,
        now_unix_ms: i64,
    ) -> Result<Self> {
        let database_path = database_path.as_ref().to_owned();
        let asset_directory = asset_directory.as_ref().to_owned();
        let storage_root = database_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_owned();
        let paths = Arc::new(Mutex::new(ClipboardStoragePaths {
            database_path: database_path.clone(),
            asset_directory: asset_directory.clone(),
        }));
        let (sender, receiver) = async_channel::bounded(CHANNEL_CAPACITY);
        let (startup_sender, startup_receiver) = mpsc::sync_channel(1);
        let worker_database_path = database_path.clone();
        let worker_asset_directory = asset_directory.clone();
        let worker_thread = thread::Builder::new()
            .name("clipboard-history-worker".into())
            .spawn(move || {
                let startup = (|| -> Result<ClipboardHistory> {
                    reset_legacy_clipboard_storage(&storage_root)?;
                    fs::create_dir_all(&worker_asset_directory)
                        .with_context(|| "failed to create clipboard asset directory")?;
                    ClipboardHistory::open(&worker_database_path)
                        .map_err(anyhow::Error::from)
                        .context("failed to open clipboard history")
                })();
                match startup {
                    Ok(mut history) => {
                        if let Err(error) = history.cleanup(retention, now_unix_ms) {
                            let _ = startup_sender.send(Err(error.to_string()));
                            return;
                        }
                        let _ = startup_sender.send(Ok(()));
                        run_worker(
                            ClipboardWorkerState {
                                history,
                                asset_directory: worker_asset_directory,
                                retention,
                                last_cleanup_at_unix_ms: now_unix_ms,
                            },
                            receiver,
                        );
                    }
                    Err(error) => {
                        let _ = startup_sender.send(Err(error.to_string()));
                    }
                }
            })
            .context("failed to start clipboard history worker")?;
        startup_receiver
            .recv()
            .context("clipboard history worker stopped during startup")?
            .map_err(anyhow::Error::msg)?;
        Ok(Self {
            sender,
            paths,
            query_generation: Arc::new(AtomicU64::new(0)),
            join: Arc::new(Mutex::new(Some(worker_thread))),
        })
    }

    pub(crate) fn database_path(&self) -> PathBuf {
        self.paths
            .lock()
            .map(|paths| paths.database_path.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().database_path.clone())
    }

    /// Queues a category page query and awaits the worker reply without blocking this thread.
    pub(crate) async fn query_category_page_async(
        &self,
        query: &str,
        category_id: &str,
        scope: ClipboardSearchScope,
        offset: usize,
        limit: usize,
        known_total: Option<usize>,
    ) -> Result<ClipboardHistoryPage, ClipboardHistoryError> {
        self.query_category_page_async_with_cursor(
            query,
            category_id,
            scope,
            offset,
            limit,
            None,
            known_total,
        )
        .await
    }

    /// Queues a page after a stable sort cursor, avoiding deep `OFFSET` scans.
    pub(crate) async fn query_category_page_after_async(
        &self,
        query: &str,
        category_id: &str,
        scope: ClipboardSearchScope,
        cursor: ClipboardPageCursor,
        limit: usize,
        known_total: Option<usize>,
    ) -> Result<ClipboardHistoryPage, ClipboardHistoryError> {
        self.query_category_page_async_with_cursor(
            query,
            category_id,
            scope,
            0,
            limit,
            Some(cursor),
            known_total,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn query_category_page_async_with_cursor(
        &self,
        query: &str,
        category_id: &str,
        scope: ClipboardSearchScope,
        offset: usize,
        limit: usize,
        cursor: Option<ClipboardPageCursor>,
        known_total: Option<usize>,
    ) -> Result<ClipboardHistoryPage, ClipboardHistoryError> {
        let generation = self
            .query_generation
            .fetch_add(1, Ordering::AcqRel)
            .saturating_add(1);
        let cancellation = QueryCancellation {
            latest_generation: Arc::clone(&self.query_generation),
            generation,
        };
        let query = query.to_owned();
        let category_id = category_id.to_owned();
        self.async_request_inner(Some(cancellation), move |history| {
            if let Some(cursor) = cursor {
                history.query_category_page_after(
                    &query,
                    &category_id,
                    scope,
                    cursor,
                    limit,
                    known_total,
                )
            } else {
                history.query_category_page_with_total(
                    &query,
                    &category_id,
                    scope,
                    offset,
                    limit,
                    known_total,
                )
            }
        })
        .await
    }

    /// Queues a counts query and awaits the worker reply without blocking this thread.
    pub(crate) async fn counts_async(
        &self,
    ) -> Result<ClipboardHistoryCounts, ClipboardHistoryError> {
        self.async_request(|history| history.counts()).await
    }

    /// Queues a category listing and awaits the worker reply without blocking this thread.
    pub(crate) async fn categories_async(
        &self,
    ) -> Result<Vec<ClipboardCategory>, ClipboardHistoryError> {
        self.async_request(|history| history.categories()).await
    }

    /// Queues a category source listing and awaits the worker reply without blocking this thread.
    pub(crate) async fn category_sources_async(
        &self,
        category_id: &str,
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        let category_id = category_id.to_owned();
        self.async_request(move |history| history.category_sources(&category_id))
            .await
    }
    pub(crate) async fn source_process_names_async(
        &self,
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        self.async_request(|history| history.source_process_names())
            .await
    }

    pub(crate) async fn create_category_async(
        &self,
        name: &str,
    ) -> Result<ClipboardCategory, ClipboardHistoryError> {
        let name = name.to_owned();
        self.async_request(move |history| history.create_category(&name))
            .await
    }

    pub(crate) async fn rename_category_async(
        &self,
        category_id: &str,
        name: &str,
    ) -> Result<ClipboardCategory, ClipboardHistoryError> {
        let category_id = category_id.to_owned();
        let name = name.to_owned();
        self.async_request(move |history| history.rename_category(&category_id, &name))
            .await
    }

    pub(crate) async fn delete_category_async(
        &self,
        category_id: &str,
    ) -> Result<(), ClipboardHistoryError> {
        let category_id = category_id.to_owned();
        self.async_request(move |history| history.delete_category(&category_id))
            .await
    }

    pub(crate) async fn reorder_categories_async(
        &self,
        ordered_ids: &[String],
    ) -> Result<Vec<ClipboardCategory>, ClipboardHistoryError> {
        let ordered_ids = ordered_ids.to_vec();
        self.async_request(move |history| history.reorder_categories(&ordered_ids))
            .await
    }

    pub(crate) async fn add_category_source_async(
        &self,
        category_id: &str,
        process_name: &str,
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        let category_id = category_id.to_owned();
        let process_name = process_name.to_owned();
        self.async_request(move |history| history.add_category_source(&category_id, &process_name))
            .await
    }

    pub(crate) async fn remove_category_source_async(
        &self,
        category_id: &str,
        process_name: &str,
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        let category_id = category_id.to_owned();
        let process_name = process_name.to_owned();
        self.async_request(move |history| {
            history.remove_category_source(&category_id, &process_name)
        })
        .await
    }

    pub(crate) async fn replace_category_sources_async(
        &self,
        category_id: &str,
        process_names: &[String],
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        let category_id = category_id.to_owned();
        let process_names = process_names.to_vec();
        self.async_request(move |history| {
            history.replace_category_sources(&category_id, &process_names)
        })
        .await
    }

    pub(crate) async fn set_favorite_async(
        &self,
        item_id: i64,
        value: bool,
    ) -> Result<nxtools_clipboard::ClipboardItem, ClipboardHistoryError> {
        self.async_request(move |history| history.set_favorite(item_id, value))
            .await
    }

    pub(crate) async fn set_pinned_async(
        &self,
        item_id: i64,
        value: bool,
    ) -> Result<nxtools_clipboard::ClipboardItem, ClipboardHistoryError> {
        self.async_request(move |history| history.set_pinned(item_id, value))
            .await
    }

    pub(crate) async fn soft_delete_async(
        &self,
        item_id: i64,
        deleted_at_unix_ms: i64,
    ) -> Result<(), ClipboardHistoryError> {
        self.async_request(move |history| history.soft_delete(item_id, deleted_at_unix_ms))
            .await
    }

    pub(crate) async fn restore_async(
        &self,
        item_id: i64,
    ) -> Result<nxtools_clipboard::ClipboardItem, ClipboardHistoryError> {
        self.async_request(move |history| history.restore(item_id))
            .await
    }

    pub(crate) async fn mark_used_async(
        &self,
        item_id: i64,
        used_at_unix_ms: i64,
    ) -> Result<nxtools_clipboard::ClipboardUsage, ClipboardHistoryError> {
        self.async_request(move |history| history.mark_used(item_id, used_at_unix_ms))
            .await
    }

    pub(crate) async fn update_text_async(
        &self,
        item_id: i64,
        text: &str,
    ) -> Result<nxtools_clipboard::ClipboardItem, ClipboardHistoryError> {
        let text = text.to_owned();
        self.async_request(move |history| history.update_text(item_id, &text))
            .await
    }

    pub(crate) async fn native_formats_async(
        &self,
        item_id: i64,
    ) -> Result<Vec<ClipboardNativeFormat>, ClipboardHistoryError> {
        self.async_request(move |history| history.native_formats(item_id))
            .await
    }

    pub(crate) fn enqueue_capture(&self, capture: ClipboardCapture) -> Result<()> {
        match self
            .sender
            .try_send(ClipboardWriteRequest::Capture(capture))
        {
            Ok(()) => Ok(()),
            Err(async_channel::TrySendError::Full(ClipboardWriteRequest::Capture(capture))) => {
                remove_staging_capture(capture);
                Err(anyhow::anyhow!("clipboard history worker queue is full"))
            }
            Err(async_channel::TrySendError::Closed(ClipboardWriteRequest::Capture(capture))) => {
                remove_staging_capture(capture);
                Err(anyhow::anyhow!("clipboard history worker is closed"))
            }
            Err(async_channel::TrySendError::Full(_)) => {
                Err(anyhow::anyhow!("clipboard history worker queue is full"))
            }
            Err(async_channel::TrySendError::Closed(_)) => {
                Err(anyhow::anyhow!("clipboard history worker is closed"))
            }
        }
    }

    pub(crate) fn enqueue_cleanup(
        &self,
        policy: ClipboardRetentionPolicy,
        now_unix_ms: i64,
    ) -> Result<()> {
        self.sender
            .try_send(ClipboardWriteRequest::Cleanup {
                policy,
                now_unix_ms,
            })
            .map_err(|error| {
                anyhow::anyhow!("clipboard history worker queue rejected cleanup: {error}")
            })
    }

    pub(crate) fn set_retention(&self, policy: ClipboardRetentionPolicy) -> Result<()> {
        self.sender
            .try_send(ClipboardWriteRequest::SetRetention(policy))
            .map_err(|error| {
                anyhow::anyhow!("clipboard history worker queue rejected retention: {error}")
            })
    }
    pub(crate) fn shutdown(self) -> Result<()> {
        let join_slot = Arc::clone(&self.join);
        let (reply_sender, reply_receiver) = mpsc::sync_channel(1);
        self.sender
            .send_blocking(ClipboardWriteRequest::Shutdown {
                reply: reply_sender,
            })
            .map_err(|error| {
                anyhow::anyhow!("clipboard history worker shutdown rejected: {error}")
            })?;
        reply_receiver.recv().map_err(|error| {
            anyhow::anyhow!("clipboard history worker shutdown reply closed: {error}")
        })?;
        let join = join_slot
            .lock()
            .map_err(|_| anyhow::anyhow!("clipboard history worker join state is poisoned"))?
            .take();
        if let Some(join) = join {
            join.join()
                .map_err(|_| anyhow::anyhow!("clipboard history worker thread panicked"))?;
        }
        Ok(())
    }

    /// Queues one history operation and awaits its reply on an `async_channel` receiver, so the
    /// caller never parks an executor thread on the worker's serialized database connection.
    async fn async_request<T, F>(&self, operation: F) -> Result<T, ClipboardHistoryError>
    where
        T: Send + 'static,
        F: FnOnce(&mut ClipboardHistory) -> Result<T, ClipboardHistoryError> + Send + 'static,
    {
        self.query_generation.fetch_add(1, Ordering::AcqRel);
        self.async_request_inner(None, operation).await
    }
    fn superseded_query_result<T: Send + 'static>() -> Box<dyn Any + Send> {
        Box::new(Err::<T, ClipboardHistoryError>(
            ClipboardHistoryError::QuerySuperseded,
        ))
    }

    async fn async_request_inner<T, F>(
        &self,
        cancellation: Option<QueryCancellation>,
        operation: F,
    ) -> Result<T, ClipboardHistoryError>
    where
        T: Send + 'static,
        F: FnOnce(&mut ClipboardHistory) -> Result<T, ClipboardHistoryError> + Send + 'static,
    {
        let (reply_sender, reply_receiver) = async_channel::bounded(1);
        self.sender
            .try_send(ClipboardWriteRequest::Job {
                operation: Box::new(move |state| Box::new(operation(&mut state.history))),
                reply: JobReply::Async(reply_sender),
                cancelled_result: cancellation.as_ref().map(|_| {
                    Box::new(Self::superseded_query_result::<T>)
                        as Box<dyn FnOnce() -> Box<dyn Any + Send> + Send>
                }),
                cancellation,
            })
            .map_err(|error| {
                worker_boundary_history_error(if error.is_full() {
                    ClipboardWorkerBoundaryError::QueueRejected
                } else {
                    ClipboardWorkerBoundaryError::WorkerClosed
                })
            })?;
        let reply = reply_receiver.recv().await.map_err(|_| {
            worker_boundary_history_error(ClipboardWorkerBoundaryError::WorkerClosed)
        })?;
        *reply
            .downcast::<Result<T, ClipboardHistoryError>>()
            .map_err(|_| {
                worker_boundary_history_error(ClipboardWorkerBoundaryError::ReplyTypeMismatch)
            })?
    }

    fn request_message<T, F>(&self, operation: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce(&mut ClipboardHistory) -> Result<T, String> + Send + 'static,
    {
        self.request_state_message(move |state| operation(&mut state.history))
    }

    fn request_state_message<T, F>(&self, operation: F) -> Result<T, String>
    where
        T: Send + 'static,
        F: FnOnce(&mut ClipboardWorkerState) -> Result<T, String> + Send + 'static,
    {
        self.query_generation.fetch_add(1, Ordering::AcqRel);
        let (reply_sender, reply_receiver) = mpsc::sync_channel(1);
        self.sender
            .try_send(ClipboardWriteRequest::Job {
                operation: Box::new(move |state| Box::new(operation(state))),
                reply: JobReply::Blocking(reply_sender),
                cancelled_result: None,
                cancellation: None,
            })
            .map_err(|error| format!("clipboard history worker queue rejected request: {error}"))?;
        let reply = reply_receiver
            .recv()
            .map_err(|error| format!("clipboard history worker reply closed: {error}"))?;
        *reply
            .downcast::<Result<T, String>>()
            .map_err(|_| "clipboard history worker reply type mismatch".to_owned())?
    }

    pub(crate) fn export_archive(
        &self,
        path: PathBuf,
    ) -> Result<ClipboardArchiveExportReport, String> {
        self.request_message(move |history| {
            history
                .export_archive(&path)
                .map_err(|error| error.to_string())
        })
    }

    pub(crate) fn import_archive(
        &self,
        path: PathBuf,
    ) -> Result<ClipboardArchiveImportReport, String> {
        self.request_state_message(move |state| {
            let asset_root = state.asset_directory.clone();
            state
                .history
                .import_archive(&path, &asset_root)
                .map_err(|error| error.to_string())
        })
    }

    pub(crate) fn create_database_backup(
        &self,
        backup_directory: PathBuf,
    ) -> Result<ClipboardDatabaseBackupReport, String> {
        self.request_message(move |history| {
            history
                .create_database_backup(&backup_directory)
                .map_err(|error| error.to_string())
        })
    }

    pub(crate) fn compact_database(
        &self,
        backup_directory: PathBuf,
    ) -> Result<ClipboardDatabaseCompactionReport, String> {
        self.request_message(move |history| {
            history
                .compact_database(&backup_directory)
                .map_err(|error| error.to_string())
        })
    }

    pub(crate) fn migrate_storage(
        &self,
        destination_root: PathBuf,
    ) -> Result<ClipboardStorageMigrationReport, String> {
        let paths = Arc::clone(&self.paths);
        self.request_state_message(move |state| {
            let source_asset_root = state.asset_directory.clone();
            let report = state
                .history
                .migrate_storage(&source_asset_root, &destination_root)
                .map_err(|error| error.to_string())?;
            write_reset_marker(&destination_root).map_err(|error| error.to_string())?;
            let replacement =
                ClipboardHistory::open(&report.database_path).map_err(|error| error.to_string())?;
            state.history = replacement;
            state.asset_directory = report.asset_root.clone();
            state.last_cleanup_at_unix_ms = 0;
            let mut paths = paths
                .lock()
                .map_err(|_| "clipboard worker path state is poisoned".to_owned())?;
            paths.database_path = report.database_path.clone();
            paths.asset_directory = report.asset_root.clone();
            Ok(report)
        })
    }
}

/// Boundary outcome of handing a job to the worker, kept separate from the result of the history
/// operation itself. `ClipboardHistoryError` stays the caller-visible error type and its signature
/// is unchanged, so the boundary kind travels inside a synthetic SQLite API-misuse error and is
/// recovered with [`worker_boundary_error`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClipboardWorkerBoundaryError {
    /// The bounded request queue is full; the worker never accepted the job.
    QueueRejected,
    /// The worker is gone, or its reply channel closed before the reply arrived.
    WorkerClosed,
    /// The worker replied with an unexpected payload type.
    ReplyTypeMismatch,
}

const WORKER_BOUNDARY_MARKER: &str = "nxtools-clipboard-worker-boundary";

impl ClipboardWorkerBoundaryError {
    fn marker(self) -> &'static str {
        match self {
            Self::QueueRejected => "queue-rejected",
            Self::WorkerClosed => "worker-closed",
            Self::ReplyTypeMismatch => "reply-type-mismatch",
        }
    }

    #[cfg(test)]
    fn from_marker(marker: &str) -> Option<Self> {
        match marker {
            "queue-rejected" => Some(Self::QueueRejected),
            "worker-closed" => Some(Self::WorkerClosed),
            "reply-type-mismatch" => Some(Self::ReplyTypeMismatch),
            _ => None,
        }
    }
}

fn worker_boundary_history_error(kind: ClipboardWorkerBoundaryError) -> ClipboardHistoryError {
    ClipboardHistoryError::Database(rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_MISUSE),
        Some(format!("{WORKER_BOUNDARY_MARKER}:{}", kind.marker())),
    ))
}

/// Recovers the worker boundary kind from an error produced by the async request path. Failures
/// raised by the history operation itself return `None`.
#[cfg(test)]
pub(crate) fn worker_boundary_error(
    error: &ClipboardHistoryError,
) -> Option<ClipboardWorkerBoundaryError> {
    let ClipboardHistoryError::Database(rusqlite::Error::SqliteFailure(native, Some(message))) =
        error
    else {
        return None;
    };
    if native.code != rusqlite::ffi::ErrorCode::ApiMisuse {
        return None;
    }
    message
        .strip_prefix(WORKER_BOUNDARY_MARKER)?
        .strip_prefix(':')
        .and_then(ClipboardWorkerBoundaryError::from_marker)
}

fn run_worker(mut state: ClipboardWorkerState, receiver: Receiver<ClipboardWriteRequest>) {
    while let Ok(request) = receiver.recv_blocking() {
        let should_stop = match request {
            ClipboardWriteRequest::Job {
                operation,
                reply,
                cancellation,
                cancelled_result,
            } => {
                if cancellation.is_some_and(|cancellation| {
                    cancellation.latest_generation.load(Ordering::Acquire)
                        != cancellation.generation
                }) {
                    reply.deliver(cancelled_result
                        .expect("cancellable query supplies a superseded result")(
                    ));
                } else {
                    reply.deliver(operation(&mut state));
                }
                false
            }
            ClipboardWriteRequest::Capture(capture) => {
                let captured_at = capture.captured_at_unix_ms;
                if let Err(_error) =
                    persist_capture(&mut state.history, &state.asset_directory, capture)
                {
                    error!(
                        error_kind = "capture_persist",
                        "clipboard history worker failed to persist capture"
                    );
                }
                if captured_at.saturating_sub(state.last_cleanup_at_unix_ms)
                    >= RETENTION_INTERVAL_MS
                {
                    let policy = state.retention;
                    apply_cleanup(&mut state, policy, captured_at);
                }
                false
            }
            ClipboardWriteRequest::Cleanup {
                policy,
                now_unix_ms,
            } => {
                state.retention = policy;
                apply_cleanup(&mut state, policy, now_unix_ms);
                false
            }
            ClipboardWriteRequest::SetRetention(policy) => {
                state.retention = policy;
                info!(
                    max_items = policy.max_items,
                    retention_days = policy.retention_days,
                    "clipboard history worker retention policy updated"
                );
                false
            }
            ClipboardWriteRequest::Shutdown { reply } => {
                let _ = reply.send(());
                true
            }
        };
        if should_stop {
            break;
        }
    }
}

fn apply_cleanup(
    state: &mut ClipboardWorkerState,
    policy: ClipboardRetentionPolicy,
    now_unix_ms: i64,
) {
    match state.history.cleanup(policy, now_unix_ms) {
        Ok(report) => {
            for path in report.purged_asset_paths {
                if !remove_managed_asset(&state.asset_directory, &path) {
                    warn!("clipboard cleanup could not remove managed asset");
                }
            }
            state.last_cleanup_at_unix_ms = now_unix_ms;
        }
        Err(_error) => {
            error!(
                error_kind = "retention",
                "clipboard history worker failed to run retention cleanup"
            );
        }
    }
}

fn persist_capture(
    history: &mut ClipboardHistory,
    asset_directory: &Path,
    capture: ClipboardCapture,
) -> Result<()> {
    let staging = StagingFiles::from_capture(&capture);
    let has_user_content = !capture.paths.is_empty()
        || capture.image.is_some()
        || capture
            .text
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty());
    if !has_user_content {
        remove_staging_files(staging);
        return Ok(());
    }
    let mut created_assets = Vec::new();
    let result = (|| -> Result<()> {
        fs::create_dir_all(asset_directory)
            .with_context(|| "failed to create clipboard asset directory")?;
        fs::create_dir_all(asset_directory.join("native-formats"))
            .with_context(|| "failed to create native format asset directory")?;

        let image_path = capture.image.as_ref().and_then(|image| {
            let digest = match content_digest(Path::new(&image.staging_path)) {
                Ok(digest) => digest,
                Err(_error) => {
                    warn!(
                        error_kind = "image_stage_hash",
                        "clipboard image staging file could not be hashed"
                    );
                    return None;
                }
            };
            match promote_asset(
                Path::new(&image.staging_path),
                asset_directory.join(format!("{digest}.png")),
                &mut created_assets,
            ) {
                Ok(path) => Some(path),
                Err(_error) => {
                    warn!(
                        error_kind = "image_stage_promote",
                        "clipboard image staging file could not be promoted"
                    );
                    None
                }
            }
        });

        let source_icon_path = capture
            .source_icon_staging_path
            .as_deref()
            .and_then(|path| {
                let digest = match content_digest(Path::new(path)) {
                    Ok(digest) => digest,
                    Err(_error) => {
                        warn!(
                            error_kind = "source_icon_stage_hash",
                            "clipboard source icon staging file could not be hashed"
                        );
                        return None;
                    }
                };
                match promote_asset(
                    Path::new(path),
                    asset_directory.join(format!("source-{digest}.png")),
                    &mut created_assets,
                ) {
                    Ok(path) => Some(path),
                    Err(_error) => {
                        warn!(
                            error_kind = "source_icon_stage_promote",
                            "clipboard source icon staging file could not be promoted"
                        );
                        None
                    }
                }
            });

        let mut native_formats = Vec::with_capacity(capture.native_formats.len());
        for format in &capture.native_formats {
            let digest = match content_digest(Path::new(&format.data_path)) {
                Ok(digest) => digest,
                Err(_error) => {
                    warn!(
                        format_id = format.format_id,
                        error_kind = "native_format_stage_hash",
                        "clipboard native format staging file could not be hashed"
                    );
                    continue;
                }
            };
            let destination = asset_directory
                .join("native-formats")
                .join(format!("{digest}.bin"));
            let data_path = match promote_asset(
                Path::new(&format.data_path),
                destination,
                &mut created_assets,
            ) {
                Ok(path) => path,
                Err(_error) => {
                    warn!(
                        format_id = format.format_id,
                        error_kind = "native_format_stage_promote",
                        "clipboard native format staging file could not be promoted"
                    );
                    continue;
                }
            };
            native_formats.push(nxtools_clipboard::ClipboardNativeFormat {
                format_id: format.format_id,
                format_name: format.format_name.clone(),
                data_path,
                size_bytes: format.size_bytes,
                sort_order: format.sort_order,
            });
        }

        let source = CaptureSource {
            window_handle: capture.source.window_handle,
            process_id: capture.source.process_id,
            process_name: capture.source.process_name.clone(),
            process_icon_path: source_icon_path.unwrap_or_default(),
            window_title: capture.source.window_title.clone(),
        };
        let text = capture.text.unwrap_or_default();
        if !capture.paths.is_empty()
            && let Some(image_path) = image_path.as_deref()
            && created_assets.iter().any(|asset| asset == image_path)
        {
            let _ = remove_managed_asset(asset_directory, image_path);
            created_assets.retain(|asset| asset != image_path);
        }
        if !capture.paths.is_empty() {
            history.capture_files(FilesCapture {
                paths: capture.paths,
                native_formats,
                source,
                captured_at_unix_ms: capture.captured_at_unix_ms,
            })?;
        } else if let Some(image) = capture.image
            && let Some(image_path) = image_path
        {
            history.capture_image(ImageCapture {
                image_path,
                width: image.width,
                height: image.height,
                native_formats,
                source,
                captured_at_unix_ms: capture.captured_at_unix_ms,
            })?;
        } else {
            history.capture(TextCapture {
                text,
                native_formats,
                source,
                captured_at_unix_ms: capture.captured_at_unix_ms,
            })?;
        }
        Ok(())
    })();

    remove_staging_files(staging);
    if result.is_err() {
        for path in created_assets {
            let _ = remove_managed_asset(asset_directory, &path);
        }
    }
    result
}

#[derive(Debug)]
struct StagingFiles {
    image: Option<PathBuf>,
    native_formats: Vec<PathBuf>,
    source_icon: Option<PathBuf>,
}

impl StagingFiles {
    fn from_capture(capture: &ClipboardCapture) -> Self {
        Self {
            image: capture
                .image
                .as_ref()
                .map(|image| PathBuf::from(&image.staging_path)),
            native_formats: capture
                .native_formats
                .iter()
                .map(|format| PathBuf::from(&format.data_path))
                .collect(),
            source_icon: capture
                .source_icon_staging_path
                .as_deref()
                .map(PathBuf::from),
        }
    }
}

fn remove_staging_capture(capture: ClipboardCapture) {
    remove_staging_files(StagingFiles::from_capture(&capture));
}

fn remove_staging_files(staging: StagingFiles) {
    for path in staging
        .image
        .into_iter()
        .chain(staging.native_formats)
        .chain(staging.source_icon)
    {
        let _ = fs::remove_file(path);
    }
}

fn content_digest(path: &Path) -> io::Result<String> {
    let mut input = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn promote_asset(
    source: &Path,
    destination: PathBuf,
    created_assets: &mut Vec<String>,
) -> Result<String> {
    if !source.is_file() {
        anyhow::bail!(
            "clipboard staging file does not exist: {}",
            source.display()
        );
    }
    if destination.exists() && !destination.is_file() {
        anyhow::bail!(
            "clipboard asset destination is not a file: {}",
            destination.display()
        );
    }
    let destination_preexisted = destination.is_file();
    let parent = destination
        .parent()
        .context("clipboard asset destination has no parent")?;
    fs::create_dir_all(parent)?;
    let temporary = parent.join(format!(".nxtools-asset-{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| -> Result<()> {
        let mut input = File::open(source)?;
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = input.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read])?;
        }
        output.sync_all()?;
        drop(output);
        if destination_preexisted {
            fs::remove_file(&temporary)?;
        } else if let Err(error) = fs::rename(&temporary, &destination) {
            if destination.exists() {
                let _ = fs::remove_file(&temporary);
            } else {
                return Err(error.into());
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    let path = destination.to_string_lossy().into_owned();
    if !destination_preexisted {
        created_assets.push(path.clone());
    }
    Ok(path)
}

fn remove_managed_asset(asset_directory: &Path, candidate: &str) -> bool {
    let Ok(asset_directory) = asset_directory.canonicalize() else {
        return false;
    };
    let Ok(candidate) = Path::new(candidate).canonicalize() else {
        return false;
    };
    if !candidate.is_file() || !candidate.starts_with(&asset_directory) {
        return false;
    }
    fs::remove_file(candidate).is_ok()
}

fn reset_legacy_clipboard_storage(storage_root: &Path) -> Result<bool> {
    fs::create_dir_all(storage_root).with_context(|| "failed to create clipboard storage root")?;
    let marker = storage_root.join(LEGACY_RESET_MARKER_FILE);
    if marker.is_file() {
        return Ok(false);
    }

    for file_name in ["clipboard.db", "clipboard.db-wal", "clipboard.db-shm"] {
        remove_file_if_present(&storage_root.join(file_name))?;
    }
    for directory_name in ["clipboard-images", "native-formats"] {
        remove_directory_if_present(&storage_root.join(directory_name))?;
    }

    let created = write_reset_marker(storage_root)?;
    if created {
        info!(storage_root = %storage_root.display(), "clipboard legacy storage reset completed");
    }
    Ok(created)
}

fn write_reset_marker(storage_root: &Path) -> Result<bool> {
    fs::create_dir_all(storage_root).with_context(|| "failed to create clipboard storage root")?;
    let marker = storage_root.join(LEGACY_RESET_MARKER_FILE);
    if marker.is_file() {
        return Ok(false);
    }
    let temporary = storage_root.join(format!(
        ".{LEGACY_RESET_MARKER_FILE}.{}.tmp",
        uuid::Uuid::new_v4()
    ));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(b"version=1\nclipboard_storage_reset=legacy-data\n")?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, &marker)?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(true)
}

fn remove_file_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_directory_if_present(path: &Path) -> io::Result<()> {
    match fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    fn retention() -> ClipboardRetentionPolicy {
        ClipboardRetentionPolicy {
            max_items: 1000,
            retention_days: 30,
        }
    }

    #[test]
    fn startup_creates_one_worker_owned_database_and_builtin_categories() {
        let directory = tempfile::tempdir().expect("clipboard worker directory");
        let database_path = directory.path().join("clipboard.db");
        let asset_directory = directory.path().join("clipboard-images");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            &database_path,
            &asset_directory,
            retention(),
            1_000,
        )
        .expect("start clipboard worker");

        assert!(database_path.is_file());
        assert!(asset_directory.is_dir());
        assert!(directory.path().join(LEGACY_RESET_MARKER_FILE).is_file());
        let async_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build worker test runtime");
        assert_eq!(
            async_runtime
                .block_on(worker.counts_async())
                .expect("worker counts")
                .total,
            0
        );
        assert_eq!(
            async_runtime
                .block_on(worker.categories_async())
                .expect("worker categories")
                .len(),
            5
        );
    }

    #[test]
    fn shutdown_barrier_flushes_accepted_requests_before_returning() {
        let directory = tempfile::tempdir().expect("clipboard shutdown directory");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("clipboard-images"),
            retention(),
            1_000,
        )
        .expect("start clipboard worker");
        worker
            .set_retention(retention())
            .expect("enqueue retention before shutdown");
        worker.shutdown().expect("flush clipboard worker");
    }

    #[test]
    fn legacy_reset_is_scoped_and_runs_only_once() {
        let directory = tempfile::tempdir().expect("clipboard reset directory");
        let root = directory.path();
        fs::write(root.join("clipboard.db"), b"legacy db").expect("legacy db");
        fs::write(root.join("clipboard.db-wal"), b"legacy wal").expect("legacy wal");
        fs::write(root.join("clipboard.db-shm"), b"legacy shm").expect("legacy shm");
        fs::create_dir_all(root.join("clipboard-images/native-formats")).expect("legacy assets");
        fs::write(root.join("clipboard-images/old.png"), b"old image").expect("old image");
        fs::create_dir_all(root.join("native-formats")).expect("legacy flat assets");
        fs::write(root.join("native-formats/old.bin"), b"old native").expect("old native");
        fs::write(root.join("config.json"), b"keep config").expect("config");
        fs::write(root.join("downloads.sqlite3"), b"keep downloads").expect("downloads");

        assert!(reset_legacy_clipboard_storage(root).expect("reset legacy storage"));
        assert!(root.join(LEGACY_RESET_MARKER_FILE).is_file());
        assert!(!root.join("clipboard.db").exists());
        assert!(!root.join("clipboard.db-wal").exists());
        assert!(!root.join("clipboard.db-shm").exists());
        assert!(!root.join("clipboard-images").exists());
        assert!(!root.join("native-formats").exists());
        assert_eq!(
            fs::read(root.join("config.json")).expect("config after reset"),
            b"keep config"
        );
        assert_eq!(
            fs::read(root.join("downloads.sqlite3")).expect("downloads after reset"),
            b"keep downloads"
        );

        fs::write(root.join("clipboard.db"), b"new db").expect("new db");
        fs::create_dir_all(root.join("clipboard-images")).expect("new assets");
        fs::write(root.join("clipboard-images/new.png"), b"new image").expect("new image");
        assert!(!reset_legacy_clipboard_storage(root).expect("second reset"));
        assert_eq!(
            fs::read(root.join("clipboard.db")).expect("db after marker"),
            b"new db"
        );
        assert_eq!(
            fs::read(root.join("clipboard-images/new.png")).expect("asset after marker"),
            b"new image"
        );
    }

    #[test]
    fn bounded_queue_rejects_after_capacity_without_unbounded_growth() {
        let directory = tempfile::tempdir().expect("clipboard queue directory");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("clipboard-images"),
            retention(),
            1_000,
        )
        .expect("start clipboard worker");
        let (started_sender, started_receiver) = mpsc::sync_channel(1);
        let (release_sender, release_receiver) = mpsc::sync_channel(1);
        let blocked_worker = worker.clone();
        let request_thread = thread::spawn(move || {
            let _: std::result::Result<(), String> =
                blocked_worker.request_state_message(move |_| {
                    started_sender.send(()).expect("notify blocked worker");
                    release_receiver.recv().expect("release blocked worker");
                    Ok(())
                });
        });
        started_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("worker should enter blocking request");

        for _ in 0..CHANNEL_CAPACITY {
            worker
                .sender
                .try_send(ClipboardWriteRequest::SetRetention(retention()))
                .expect("queue should accept its configured capacity");
        }
        assert!(
            worker
                .sender
                .try_send(ClipboardWriteRequest::SetRetention(retention()))
                .is_err()
        );

        release_sender.send(()).expect("release worker request");
        request_thread.join().expect("blocked request thread");
    }

    #[tokio::test]
    async fn superseded_queries_are_skipped_before_database_execution() {
        let directory = tempfile::tempdir().expect("clipboard supersession directory");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("clipboard-images"),
            retention(),
            1_000,
        )
        .expect("start clipboard worker");
        let (started_sender, started_receiver) = mpsc::sync_channel(1);
        let (release_sender, release_receiver) = mpsc::sync_channel(1);
        let blocked_worker = worker.clone();
        let blocker = thread::spawn(move || {
            let result: Result<(), String> = blocked_worker.request_state_message(move |_| {
                started_sender.send(()).expect("notify blocked worker");
                release_receiver.recv().expect("release blocked worker");
                Ok(())
            });
            result.expect("blocked worker request");
        });
        started_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("worker should enter blocking request");

        let first_worker = worker.clone();
        let first = tokio::spawn(async move {
            first_worker
                .query_category_page_async(
                    "",
                    nxtools_clipboard::BUILTIN_ALL_CATEGORY_ID,
                    ClipboardSearchScope::Content,
                    0,
                    20,
                    None,
                )
                .await
        });
        for _ in 0..1_000 {
            if !worker.sender.is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(!worker.sender.is_empty(), "first query was not queued");

        let second_worker = worker.clone();
        let second = tokio::spawn(async move {
            second_worker
                .query_category_page_async(
                    "",
                    nxtools_clipboard::BUILTIN_ALL_CATEGORY_ID,
                    ClipboardSearchScope::Content,
                    0,
                    20,
                    None,
                )
                .await
        });
        for _ in 0..1_000 {
            if worker.sender.len() >= 2 {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(worker.sender.len() >= 2, "second query was not queued");

        release_sender.send(()).expect("release blocked worker");
        blocker.join().expect("join blocked worker");
        assert!(matches!(
            first.await.expect("join first query"),
            Err(ClipboardHistoryError::QuerySuperseded)
        ));
        assert!(second.await.expect("join second query").is_ok());
        worker.shutdown().expect("flush clipboard worker");
    }

    #[tokio::test]
    async fn async_queries_deliver_worker_replies_without_a_blocking_receive() {
        let directory = tempfile::tempdir().expect("clipboard async directory");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("clipboard-images"),
            retention(),
            1_000,
        )
        .expect("start clipboard worker");

        let counts = worker.counts_async().await.expect("async worker counts");
        assert_eq!(counts.total, 0);
        let categories = worker
            .categories_async()
            .await
            .expect("async worker categories");
        assert_eq!(categories.len(), 5);
        let page = worker
            .query_category_page_async(
                "",
                nxtools_clipboard::BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                0,
                20,
                None,
            )
            .await
            .expect("async worker page");
        assert!(page.items.is_empty());
        assert_eq!(page.total, 0);
        assert!(!page.has_more);
        let sources = worker
            .category_sources_async(nxtools_clipboard::BUILTIN_ALL_CATEGORY_ID)
            .await
            .expect("async worker category sources");
        assert!(sources.is_empty());

        worker.shutdown().expect("flush clipboard worker");
    }

    #[tokio::test]
    async fn async_query_surfaces_history_errors_without_a_boundary_marker() {
        let directory = tempfile::tempdir().expect("clipboard async error directory");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("clipboard-images"),
            retention(),
            1_000,
        )
        .expect("start clipboard worker");

        let error = worker
            .category_sources_async("missing-category")
            .await
            .expect_err("unknown category must fail the async query");
        assert_eq!(worker_boundary_error(&error), None);
        assert!(matches!(
            error,
            ClipboardHistoryError::CategoryNotFound(ref category_id)
                if category_id == "missing-category"
        ));

        worker.shutdown().expect("flush clipboard worker");
    }

    #[tokio::test]
    async fn async_query_marks_a_full_queue_as_a_distinct_boundary_error() {
        let directory = tempfile::tempdir().expect("clipboard async queue directory");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("clipboard-images"),
            retention(),
            1_000,
        )
        .expect("start clipboard worker");
        let (started_sender, started_receiver) = mpsc::sync_channel(1);
        let (release_sender, release_receiver) = mpsc::sync_channel(1);
        let blocked_worker = worker.clone();
        let request_thread = thread::spawn(move || {
            let _: std::result::Result<(), String> =
                blocked_worker.request_state_message(move |_| {
                    started_sender.send(()).expect("notify blocked worker");
                    release_receiver.recv().expect("release blocked worker");
                    Ok(())
                });
        });
        started_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("worker should enter blocking request");

        for _ in 0..CHANNEL_CAPACITY {
            worker
                .sender
                .try_send(ClipboardWriteRequest::SetRetention(retention()))
                .expect("queue should accept its configured capacity");
        }

        let error = worker
            .counts_async()
            .await
            .expect_err("a full queue must reject the async request");
        assert_eq!(
            worker_boundary_error(&error),
            Some(ClipboardWorkerBoundaryError::QueueRejected)
        );

        release_sender.send(()).expect("release worker request");
        request_thread.join().expect("blocked request thread");
    }

    #[test]
    fn worker_boundary_errors_stay_distinct_from_history_errors() {
        for kind in [
            ClipboardWorkerBoundaryError::QueueRejected,
            ClipboardWorkerBoundaryError::WorkerClosed,
            ClipboardWorkerBoundaryError::ReplyTypeMismatch,
        ] {
            let error = worker_boundary_history_error(kind);
            assert_eq!(worker_boundary_error(&error), Some(kind));
        }
        assert_eq!(
            worker_boundary_error(&ClipboardHistoryError::EmptyTextEdit),
            None
        );
        assert_eq!(
            worker_boundary_error(&ClipboardHistoryError::CategoryNotFound("missing".into())),
            None
        );
    }
}
