mod icon_colors;
pub(crate) type SourceIconColors = Vec<(PathBuf, Option<[u8; 3]>)>;
mod thumbnails;

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
#[cfg(test)]
use nxtools_clipboard::ClipboardHistoryPage;
use nxtools_clipboard::{
    CaptureOutcome, CaptureSource, ClipboardArchiveExportReport, ClipboardArchiveImportReport,
    ClipboardCategory, ClipboardDatabaseBackupReport, ClipboardDatabaseCompactionReport,
    ClipboardHistory, ClipboardHistoryCounts, ClipboardHistoryError, ClipboardNativeFormat,
    ClipboardPageCursor, ClipboardRetentionPolicy, ClipboardSearchScope,
    ClipboardStorageMigrationReport, FilesCapture, ImageCapture, TextCapture,
};
use nxtools_core::AppConfig;
use nxtools_protocol::ClipboardCapture;
use sha2::{Digest, Sha256};
use tracing::{error, info, warn};

const CHANNEL_CAPACITY: usize = 256;
const RETENTION_INTERVAL_MS: i64 = 60 * 60 * 1000;
const LEGACY_RESET_MARKER_FILE: &str = ".nxtools-clipboard-legacy-reset-v1";
const COMMIT_NOTIFICATION_CAPACITY: usize = 64;
const MAX_COMMIT_SUBSCRIPTIONS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClipboardCommitChange {
    Inserted,
    Updated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClipboardCommitCausality {
    pub root_operation_id: uuid::Uuid,
    pub origin_plugin: Option<String>,
    pub invoked_handlers: Vec<String>,
    pub depth: u8,
    pub derived_from: Option<i64>,
}

impl ClipboardCommitCausality {
    pub(crate) fn capture() -> Self {
        Self {
            root_operation_id: uuid::Uuid::new_v4(),
            origin_plugin: None,
            invoked_handlers: Vec::new(),
            depth: 0,
            derived_from: None,
        }
    }

    pub(crate) fn validate(&self) -> Result<()> {
        anyhow::ensure!(
            !self.root_operation_id.is_nil(),
            "invalid clipboard operation identity"
        );
        anyhow::ensure!(
            self.depth <= 4 && self.invoked_handlers.len() <= 4,
            "clipboard causality limit exceeded"
        );
        anyhow::ensure!(
            self.origin_plugin
                .as_ref()
                .is_none_or(|id| !id.is_empty() && id.len() <= 128),
            "invalid clipboard origin"
        );
        for (index, handler) in self.invoked_handlers.iter().enumerate() {
            anyhow::ensure!(
                !handler.is_empty() && handler.len() <= 256,
                "invalid clipboard handler identity"
            );
            anyhow::ensure!(
                !self.invoked_handlers[..index].contains(handler),
                "clipboard handler reentry"
            );
        }
        anyhow::ensure!(
            self.derived_from.is_none_or(|id| id > 0),
            "invalid source item identity"
        );
        Ok(())
    }
}

/// Metadata only: consuming this queue never consumes UI replies or exposes capture payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClipboardCommitted {
    pub item_id: i64,
    pub kind: nxtools_clipboard::ClipboardKind,
    pub change: ClipboardCommitChange,
    pub causality: ClipboardCommitCausality,
}

pub(crate) struct ClipboardCommitSubscription {
    pub receiver: Receiver<ClipboardCommitted>,
    dropped: Arc<AtomicU64>,
    contended: Arc<AtomicU64>,
    contended_at_subscription: u64,
}

impl ClipboardCommitSubscription {
    pub(crate) fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed).saturating_add(
            self.contended
                .load(Ordering::Relaxed)
                .saturating_sub(self.contended_at_subscription),
        )
    }
}

struct CommitSubscriber {
    sender: Sender<ClipboardCommitted>,
    dropped: Arc<AtomicU64>,
}

#[derive(Default)]
struct CommitNotifications {
    subscribers: Mutex<Vec<CommitSubscriber>>,
    contended: Arc<AtomicU64>,
}

impl CommitNotifications {
    fn publish(&self, committed: &ClipboardCommitted) {
        // Never wait for a subscriber or execute consumer code on the database worker.
        let Ok(mut subscribers) = self.subscribers.try_lock() else {
            self.contended.fetch_add(1, Ordering::Relaxed);
            return;
        };
        subscribers.retain(|subscriber| !subscriber.sender.is_closed());
        for subscriber in subscribers.iter() {
            if subscriber.sender.try_send(committed.clone()).is_err() {
                subscriber.dropped.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
}

// The actor owns this state for its entire lifetime. The handle only mirrors the
// paths needed by callers to label maintenance operations and update configuration.
struct ClipboardWorkerState {
    history: ClipboardHistory,
    asset_directory: PathBuf,
    retention: ClipboardRetentionPolicy,
    last_cleanup_at_unix_ms: i64,
    icon_colors: icon_colors::IconColorCache,
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
    Capture {
        capture: Box<ClipboardCapture>,
        causality: ClipboardCommitCausality,
        reply: Option<JobReply>,
    },
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
    commits: Arc<CommitNotifications>,
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
        let commits = Arc::new(CommitNotifications::default());
        let worker_commits = Arc::clone(&commits);
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
                                icon_colors: icon_colors::IconColorCache::default(),
                            },
                            receiver,
                            worker_commits,
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
            commits,
        })
    }

    pub(crate) fn database_path(&self) -> PathBuf {
        self.paths
            .lock()
            .map(|paths| paths.database_path.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().database_path.clone())
    }

    pub(crate) async fn source_icon_colors_async(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<SourceIconColors> {
        let (sender, receiver) = async_channel::bounded(1);
        self.sender.try_send(ClipboardWriteRequest::Job {
            operation: Box::new(move |state| {
                let started = std::time::Instant::now();
                let correlation_id = uuid::Uuid::new_v4();
                info!(event="clipboard.icon_palette.started", %correlation_id, count=paths.len().min(4));
                let mut results = SourceIconColors::new();
                let mut cache_hits = 0_usize;
                for path in paths.into_iter().take(4) {
                    let (color, cached) = state.icon_colors.get(&state.asset_directory, &path);
                    cache_hits += usize::from(cached);
                    results.push((path, color));
                }
                info!(event="clipboard.icon_palette.completed", %correlation_id, count=results.len(), cache_hits,
                    unavailable=results.iter().filter(|(_,color)| color.is_none()).count(), elapsed_ms=started.elapsed().as_millis());
                Box::new(results)
            }),
            reply: JobReply::Async(sender), cancellation: None, cancelled_result: None,
        }).map_err(|_| anyhow::anyhow!("icon palette worker unavailable"))?;
        receiver
            .recv()
            .await
            .context("icon palette reply unavailable")?
            .downcast::<SourceIconColors>()
            .map(|result| *result)
            .map_err(|_| anyhow::anyhow!("icon palette reply type mismatch"))
    }

    /// Disposable derivatives never supersede history queries or mutate original payloads.
    pub(crate) async fn list_thumbnails_async(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<Vec<(PathBuf, Option<PathBuf>)>> {
        let (reply_sender, reply_receiver) = async_channel::bounded(1);
        self.sender.try_send(ClipboardWriteRequest::Job {
            operation: Box::new(move |state| {
                let started = std::time::Instant::now();
                let correlation_id = uuid::Uuid::new_v4();
                let mut results = Vec::new();
                let mut failures = 0_usize;
                for path in paths.into_iter().take(4) {
                    let thumbnail = thumbnails::ensure_thumbnail(&state.asset_directory, &path).ok();
                    failures += usize::from(thumbnail.is_none());
                    results.push((path, thumbnail));
                }
                if failures > 0 {
                    warn!(event = "clipboard.thumbnail.batch_failed", %correlation_id,
                        requested = results.len(), failures, elapsed_ms = started.elapsed().as_millis(),
                        failure_kind = "source_or_cache", "list thumbnail batch completed with unavailable derivatives");
                }
                Box::new(results)
            }),
            reply: JobReply::Async(reply_sender), cancellation: None, cancelled_result: None,
        }).map_err(|_| anyhow::anyhow!("thumbnail worker unavailable"))?;
        let reply = reply_receiver
            .recv()
            .await
            .context("thumbnail reply unavailable")?;
        reply
            .downcast::<Vec<(PathBuf, Option<PathBuf>)>>()
            .map(|result| *result)
            .map_err(|_| anyhow::anyhow!("thumbnail reply type mismatch"))
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn query_summary_page_async(
        &self,
        query: &str,
        category_id: &str,
        scope: ClipboardSearchScope,
        offset: usize,
        limit: usize,
        known_total: Option<usize>,
        cursor: Option<ClipboardPageCursor>,
        filters: nxtools_clipboard::ClipboardHistoryFilter,
    ) -> Result<nxtools_clipboard::ClipboardSummaryPage, ClipboardHistoryError> {
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
            history.filtered_summary_page(
                &query,
                &category_id,
                scope,
                offset,
                limit,
                cursor,
                known_total,
                &filters,
            )
        })
        .await
    }

    pub(crate) async fn active_item_async(
        &self,
        item_id: i64,
    ) -> Result<nxtools_clipboard::ClipboardItem, ClipboardHistoryError> {
        self.async_request_inner(None, move |history| history.active_item(item_id))
            .await
    }

    pub(crate) async fn open_targets_async(
        &self,
        item_ids: Vec<i64>,
    ) -> Result<Vec<(i64, Vec<nxtools_clipboard::ClipboardOpenTarget>)>, ClipboardHistoryError>
    {
        self.async_request_inner(None, move |history| {
            item_ids
                .into_iter()
                .take(4)
                .map(|id| {
                    history
                        .active_item(id)
                        .map(|item| (id, nxtools_clipboard::resolve_open_targets(&item)))
                })
                .collect()
        })
        .await
    }

    pub(crate) async fn action_item_async(
        &self,
        item_id: i64,
    ) -> Result<(nxtools_clipboard::ClipboardItem, Vec<ClipboardNativeFormat>), ClipboardHistoryError>
    {
        self.async_request_inner(None, move |history| {
            history.active_item_with_native_formats(item_id)
        })
        .await
    }

    pub(crate) async fn summary_page_containing_item_async(
        &self,
        item_id: i64,
        limit: usize,
    ) -> Result<(usize, nxtools_clipboard::ClipboardSummaryPage), ClipboardHistoryError> {
        self.async_request_inner(None, move |history| {
            history.summary_page_containing_item(item_id, limit)
        })
        .await
    }

    /// Queues a category page query and awaits the worker reply without blocking this thread.
    #[cfg(test)]
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

    #[cfg(test)]
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

    pub(crate) fn subscribe_committed(&self) -> Result<ClipboardCommitSubscription> {
        let mut subscribers = self
            .commits
            .subscribers
            .lock()
            .map_err(|_| anyhow::anyhow!("clipboard subscription state unavailable"))?;
        subscribers.retain(|subscriber| !subscriber.sender.is_closed());
        anyhow::ensure!(
            subscribers.len() < MAX_COMMIT_SUBSCRIPTIONS,
            "clipboard subscription limit reached"
        );
        let (sender, receiver) = async_channel::bounded(COMMIT_NOTIFICATION_CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        subscribers.push(CommitSubscriber {
            sender,
            dropped: Arc::clone(&dropped),
        });
        Ok(ClipboardCommitSubscription {
            receiver,
            dropped,
            contended: Arc::clone(&self.commits.contended),
            contended_at_subscription: self.commits.contended.load(Ordering::Relaxed),
        })
    }

    pub(crate) fn enqueue_capture(&self, capture: ClipboardCapture) -> Result<()> {
        self.send_capture(capture, ClipboardCommitCausality::capture(), None)
    }

    fn send_capture(
        &self,
        capture: ClipboardCapture,
        causality: ClipboardCommitCausality,
        reply: Option<JobReply>,
    ) -> Result<()> {
        if let Err(error) = causality.validate() {
            remove_staging_capture(capture);
            return Err(error);
        }
        self.sender
            .try_send(ClipboardWriteRequest::Capture {
                capture: Box::new(capture),
                causality,
                reply,
            })
            .map_err(|error| {
                let full = error.is_full();
                if let ClipboardWriteRequest::Capture { capture, .. } = error.into_inner() {
                    remove_staging_capture(*capture);
                }
                anyhow::anyhow!(if full {
                    "clipboard history worker queue is full"
                } else {
                    "clipboard history worker is closed"
                })
            })
    }

    /// Reply is the real committed identity, not queue admission. Notifications are independent.
    pub(crate) async fn capture_committed_async(
        &self,
        capture: ClipboardCapture,
        causality: ClipboardCommitCausality,
    ) -> Result<Option<ClipboardCommitted>> {
        let (sender, receiver) = async_channel::bounded(1);
        self.send_capture(capture, causality, Some(JobReply::Async(sender)))?;
        let reply = receiver
            .recv()
            .await
            .context("clipboard capture reply closed")?;
        *reply
            .downcast::<Result<Option<ClipboardCommitted>>>()
            .map_err(|_| anyhow::anyhow!("clipboard capture reply type mismatch"))?
    }

    /// Extension reads do not supersede the UI's generation-scoped paging requests.
    pub(crate) async fn extension_query_async(
        &self,
        query: String,
        limit: usize,
    ) -> Result<Vec<nxtools_clipboard::ClipboardItem>, ClipboardHistoryError> {
        self.async_request_inner(None, move |history| {
            history.search_extended(&query, limit.clamp(1, 32))
        })
        .await
    }

    pub(crate) async fn extension_page_async(
        &self,
        query: String,
        cursor: Option<ClipboardPageCursor>,
        revision: Option<String>,
        limit: usize,
    ) -> Result<Option<(nxtools_clipboard::ClipboardHistoryPage, String)>, ClipboardHistoryError>
    {
        self.async_request_inner(None, move |history| {
            if let Some(revision) = revision.as_ref()
                && history.mutation_revision()? != *revision
            {
                return Ok(None);
            }
            history.extension_page(&query, cursor, limit).map(Some)
        })
        .await
    }

    pub(crate) async fn extension_get_async(
        &self,
        item_id: i64,
    ) -> Result<nxtools_clipboard::ClipboardItem, ClipboardHistoryError> {
        self.active_item_async(item_id).await
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

    /// Serializes the config commit with the worker's history switch, so captures
    /// cannot land in the destination until restart will also open that destination.
    pub(crate) fn migrate_storage(
        &self,
        destination_root: PathBuf,
        config_path: PathBuf,
    ) -> Result<(ClipboardStorageMigrationReport, AppConfig), String> {
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
            let mut paths = paths
                .lock()
                .map_err(|_| "clipboard worker path state is poisoned".to_owned())?;
            let storage_directory = destination_root.to_string_lossy().into_owned();
            let merged = AppConfig::update_section(&config_path, |config| {
                config.clipboard.storage_directory = storage_directory;
            })
            .map_err(|error| error.to_string())?;
            state.history = replacement;
            state.asset_directory = report.asset_root.clone();
            state.last_cleanup_at_unix_ms = 0;
            paths.database_path = report.database_path.clone();
            paths.asset_directory = report.asset_root.clone();
            Ok((report, merged))
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

fn run_worker(
    mut state: ClipboardWorkerState,
    receiver: Receiver<ClipboardWriteRequest>,
    commits: Arc<CommitNotifications>,
) {
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
            ClipboardWriteRequest::Capture {
                capture,
                causality,
                reply,
            } => {
                let captured_at = capture.captured_at_unix_ms;
                let started = std::time::Instant::now();
                info!(event = "clipboard.commit.started", operation_id = %causality.root_operation_id, "clipboard commit started");
                let result = (|| -> Result<Option<ClipboardCommitted>> {
                    if let Some(source_id) = causality.derived_from {
                        match state.history.active_item(source_id) {
                            Ok(source) => {
                                let same_payload = (source.kind
                                    == nxtools_clipboard::ClipboardKind::Text
                                    && capture.paths.is_empty()
                                    && capture.image.is_none()
                                    && capture.text.as_deref() == Some(source.text.as_str()))
                                    || (source.kind == nxtools_clipboard::ClipboardKind::Files
                                        && capture.paths == source.file_paths);
                                if same_payload {
                                    remove_staging_files(StagingFiles::from_capture(&capture));
                                    anyhow::bail!("derived content must not replace its source");
                                }
                            }
                            Err(error) => {
                                remove_staging_files(StagingFiles::from_capture(&capture));
                                return Err(error.into());
                            }
                        }
                    }
                    Ok(
                        persist_capture(&mut state.history, &state.asset_directory, *capture)?.map(
                            |(item_id, kind, change)| ClipboardCommitted {
                                item_id,
                                kind,
                                change,
                                causality: causality.clone(),
                            },
                        ),
                    )
                })();
                match &result {
                    Ok(Some(committed)) => {
                        info!(event = "clipboard.commit.completed", operation_id = %causality.root_operation_id, item_id = committed.item_id, kind = ?committed.kind, change = ?committed.change, elapsed_ms = started.elapsed().as_millis() as u64, "clipboard commit completed");
                        commits.publish(committed);
                    }
                    Ok(None) => {
                        info!(event = "clipboard.commit.ignored", operation_id = %causality.root_operation_id, elapsed_ms = started.elapsed().as_millis() as u64, reason = "empty", "clipboard commit ignored")
                    }
                    Err(_) => {
                        error!(event = "clipboard.commit.failed", operation_id = %causality.root_operation_id, error_kind = "capture_persist", elapsed_ms = started.elapsed().as_millis() as u64, "clipboard history worker failed to persist capture")
                    }
                }
                if let Some(reply) = reply {
                    reply.deliver(Box::new(result));
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
    if let Ok(mut subscribers) = commits.subscribers.lock() {
        for subscriber in subscribers.drain(..) {
            subscriber.sender.close();
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
) -> Result<Option<(i64, nxtools_clipboard::ClipboardKind, ClipboardCommitChange)>> {
    let staging = StagingFiles::from_capture(&capture);
    let has_user_content = !capture.paths.is_empty()
        || capture.image.is_some()
        || capture
            .text
            .as_deref()
            .is_some_and(|text| !text.trim().is_empty());
    if !has_user_content {
        remove_staging_files(staging);
        return Ok(None);
    }
    let mut created_assets = Vec::new();
    let result =
        (|| -> Result<Option<(i64, nxtools_clipboard::ClipboardKind, ClipboardCommitChange)>> {
            fs::create_dir_all(asset_directory)
                .with_context(|| "failed to create clipboard asset directory")?;
            fs::create_dir_all(asset_directory.join("native-formats"))
                .with_context(|| "failed to create native format asset directory")?;

            let image_path = capture
                .image
                .as_ref()
                .map(|image| -> Result<String> {
                    let digest = content_digest(Path::new(&image.staging_path))?;
                    promote_asset(
                        Path::new(&image.staging_path),
                        asset_directory.join(format!("{digest}.png")),
                        &mut created_assets,
                    )
                })
                .transpose()?;

            let source_icon_path = capture
                .source_icon_staging_path
                .as_deref()
                .map(|path| -> Result<String> {
                    let digest = content_digest(Path::new(path))?;
                    promote_asset(
                        Path::new(path),
                        asset_directory.join(format!("source-{digest}.png")),
                        &mut created_assets,
                    )
                })
                .transpose()?;

            let mut native_formats = Vec::with_capacity(capture.native_formats.len());
            for format in &capture.native_formats {
                let digest = content_digest(Path::new(&format.data_path))?;
                let destination = asset_directory
                    .join("native-formats")
                    .join(format!("{digest}.bin"));
                let data_path = promote_asset(
                    Path::new(&format.data_path),
                    destination,
                    &mut created_assets,
                )?;
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
            let outcome = if !capture.paths.is_empty() {
                history.capture_files(FilesCapture {
                    paths: capture.paths,
                    native_formats,
                    source,
                    captured_at_unix_ms: capture.captured_at_unix_ms,
                })?
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
                })?
            } else {
                history.capture(TextCapture {
                    text,
                    native_formats,
                    source,
                    captured_at_unix_ms: capture.captured_at_unix_ms,
                })?
            };
            Ok(match outcome {
                CaptureOutcome::Inserted(item) => {
                    Some((item.id, item.kind, ClipboardCommitChange::Inserted))
                }
                CaptureOutcome::Updated(item) => {
                    Some((item.id, item.kind, ClipboardCommitChange::Updated))
                }
                CaptureOutcome::IgnoredEmpty => None,
            })
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
    if fs::remove_file(&candidate).is_ok() {
        thumbnails::remove_thumbnail(&asset_directory, &candidate);
        true
    } else {
        false
    }
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

    fn fixture_capture(text: &str) -> ClipboardCapture {
        ClipboardCapture {
            text: Some(text.into()),
            image: None,
            paths: Vec::new(),
            native_formats: Vec::new(),
            source_icon_staging_path: None,
            source: nxtools_protocol::ForegroundContext {
                window_handle: 0,
                focus_window_handle: 0,
                process_id: 0,
                process_name: String::new(),
                window_title: String::new(),
            },
            captured_at_unix_ms: 1_001,
        }
    }

    #[tokio::test]
    async fn icon_palette_worker_bounds_batches_without_superseding_history_queries() {
        let directory = tempfile::tempdir().expect("owned icon palette fixture");
        let root = directory.path().join("assets");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            root.clone(),
            retention(),
            1_000,
        )
        .expect("worker");
        let paths = (0..6)
            .map(|key| {
                let path = root.join(format!("source-{key:064x}.png"));
                image::RgbaImage::from_pixel(16, 16, image::Rgba([20, 180, 70, 255]))
                    .save(&path)
                    .expect("synthetic source icon");
                path
            })
            .collect::<Vec<_>>();
        let generation = worker.query_generation.load(Ordering::Acquire);
        let colors = worker
            .source_icon_colors_async(paths.clone())
            .await
            .expect("bounded palette reply");
        assert_eq!(colors.len(), 4);
        for ((path, color), expected) in colors.iter().zip(&paths) {
            assert_eq!(path, expected);
            assert_eq!(*color, Some([20, 180, 70]));
        }
        assert_eq!(worker.query_generation.load(Ordering::Acquire), generation);
        assert_eq!(
            worker.counts_async().await.expect("history intact").total,
            0
        );
        worker.shutdown().expect("shutdown fixture");
    }

    #[tokio::test]
    async fn committed_capture_reply_matches_independent_success_and_duplicate_events() {
        let directory = tempfile::tempdir().expect("owned clipboard fixture");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("assets"),
            retention(),
            1_000,
        )
        .expect("worker");
        let first = worker.subscribe_committed().expect("first subscription");
        let second = worker
            .subscribe_committed()
            .expect("independent subscription");
        let causality = ClipboardCommitCausality::capture();
        let inserted = worker
            .capture_committed_async(fixture_capture("original"), causality.clone())
            .await
            .expect("commit")
            .expect("nonempty capture");
        assert_eq!(inserted.change, ClipboardCommitChange::Inserted);
        assert_eq!(inserted.kind, nxtools_clipboard::ClipboardKind::Text);
        assert_eq!(inserted.causality, causality);
        assert_eq!(first.receiver.try_recv().expect("first event"), inserted);
        assert_eq!(second.receiver.try_recv().expect("second event"), inserted);
        let updated = worker
            .capture_committed_async(
                fixture_capture("original"),
                ClipboardCommitCausality::capture(),
            )
            .await
            .expect("duplicate commit")
            .expect("duplicate is committed");
        assert_eq!(updated.item_id, inserted.item_id);
        assert_eq!(updated.change, ClipboardCommitChange::Updated);
        assert_eq!(worker.counts_async().await.expect("counts").total, 1);
        assert_eq!(first.receiver.try_recv().expect("duplicate event"), updated);
        assert_eq!(
            second
                .receiver
                .try_recv()
                .expect("independent duplicate event"),
            updated
        );
        let staged = directory.path().join("image.png");
        fs::write(&staged, b"owned synthetic image bytes").expect("stage image");
        let mut image = fixture_capture("");
        image.image = Some(nxtools_protocol::ClipboardImageData {
            staging_path: staged.to_string_lossy().into_owned(),
            width: 2,
            height: 3,
        });
        let committed_image = worker
            .capture_committed_async(image, ClipboardCommitCausality::capture())
            .await
            .expect("asset and database success")
            .expect("image item");
        assert_eq!(
            committed_image.kind,
            nxtools_clipboard::ClipboardKind::Image
        );
        assert_eq!(
            first
                .receiver
                .try_recv()
                .expect("image event after asset success"),
            committed_image
        );
        assert_eq!(
            second.receiver.try_recv().expect("independent image event"),
            committed_image
        );
        let item = worker
            .extension_get_async(committed_image.item_id)
            .await
            .expect("persisted image");
        assert_eq!(
            fs::read(&item.image_path).expect("promoted asset"),
            b"owned synthetic image bytes"
        );
        assert!(!staged.exists());
        assert_eq!(worker.counts_async().await.expect("counts").total, 2);
        worker.shutdown().expect("shutdown fixture");
    }

    #[tokio::test]
    async fn extension_and_derived_capture_resolve_full_active_source_by_id() {
        let directory = tempfile::tempdir().expect("owned clipboard fixture");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("assets"),
            retention(),
            1_000,
        )
        .expect("worker");
        let source_text = "原".repeat(512) + "tail";
        let source_id = worker
            .capture_committed_async(
                fixture_capture(&source_text),
                ClipboardCommitCausality::capture(),
            )
            .await
            .expect("source commit")
            .expect("source item")
            .item_id;
        for index in 0..500 {
            worker
                .capture_committed_async(
                    fixture_capture(&format!("newer item {index}")),
                    ClipboardCommitCausality::capture(),
                )
                .await
                .expect("newer commit")
                .expect("newer item");
        }
        let source = worker
            .extension_get_async(source_id)
            .await
            .expect("old source");
        assert_eq!(source.id, source_id);
        assert_eq!(source.text, source_text);
        let derived = ClipboardCommitCausality {
            derived_from: Some(source_id),
            ..ClipboardCommitCausality::capture()
        };
        let replacement = worker
            .capture_committed_async(fixture_capture(&source_text), derived.clone())
            .await
            .expect_err("derived capture cannot replace its source");
        assert_eq!(
            replacement.to_string(),
            "derived content must not replace its source"
        );
        let distinct = worker
            .capture_committed_async(fixture_capture("derived text"), derived.clone())
            .await
            .expect("derived commit")
            .expect("derived item");
        assert_ne!(distinct.item_id, source_id);

        worker
            .soft_delete_async(source_id, 1_002)
            .await
            .expect("soft delete");
        assert!(matches!(
            worker.extension_get_async(source_id).await,
            Err(ClipboardHistoryError::ItemNotFound(id)) if id == source_id
        ));
        let unavailable = worker
            .capture_committed_async(fixture_capture("unavailable source"), derived)
            .await
            .expect_err("deleted source cannot authorize a derived capture");
        assert!(matches!(
            unavailable.downcast_ref::<ClipboardHistoryError>(),
            Some(ClipboardHistoryError::ItemNotFound(id)) if *id == source_id
        ));
        worker.shutdown().expect("shutdown fixture");
    }

    #[tokio::test]
    async fn failed_database_or_required_asset_and_empty_capture_publish_no_commit() {
        let directory = tempfile::tempdir().expect("owned clipboard fixture");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("assets"),
            retention(),
            1_000,
        )
        .expect("worker");
        let events = worker.subscribe_committed().expect("subscription");
        let staged = directory.path().join("native.bin");
        fs::write(&staged, b"owned native bytes").expect("stage asset");
        let connection = rusqlite::Connection::open(worker.database_path()).expect("fixture SQL");
        connection.execute_batch("CREATE TRIGGER reject_capture BEFORE INSERT ON clipboard_items BEGIN SELECT RAISE(ABORT, 'fixture rejection'); END;")
            .expect("inject actual database commit failure");
        let mut capture = fixture_capture("rejected");
        capture
            .native_formats
            .push(nxtools_protocol::ClipboardNativeFormat {
                format_id: 13,
                format_name: "UnicodeText".into(),
                data_path: staged.to_string_lossy().into_owned(),
                size_bytes: 18,
                sort_order: 0,
            });
        assert!(
            worker
                .capture_committed_async(capture, ClipboardCommitCausality::capture())
                .await
                .is_err()
        );
        assert!(!staged.exists());
        assert_eq!(
            fs::read_dir(directory.path().join("assets/native-formats"))
                .expect("assets")
                .count(),
            0
        );
        connection
            .execute_batch("DROP TRIGGER reject_capture;")
            .expect("remove fixture failure");
        let mut missing = fixture_capture("must not degrade to text");
        missing.image = Some(nxtools_protocol::ClipboardImageData {
            staging_path: directory
                .path()
                .join("missing.png")
                .to_string_lossy()
                .into_owned(),
            width: 1,
            height: 1,
        });
        assert!(
            worker
                .capture_committed_async(missing, ClipboardCommitCausality::capture())
                .await
                .is_err()
        );
        assert!(
            worker
                .capture_committed_async(fixture_capture(" "), ClipboardCommitCausality::capture())
                .await
                .expect("empty is ignored")
                .is_none()
        );
        assert_eq!(worker.counts_async().await.expect("counts").total, 0);
        assert!(events.receiver.try_recv().is_err());
        worker.shutdown().expect("shutdown fixture");
    }

    #[tokio::test]
    async fn slow_commit_subscriber_is_bounded_without_blocking_capture_or_other_subscriber() {
        let directory = tempfile::tempdir().expect("owned clipboard fixture");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("assets"),
            retention(),
            1_000,
        )
        .expect("worker");
        let slow = worker.subscribe_committed().expect("slow subscription");
        let fast = worker.subscribe_committed().expect("fast subscription");
        for index in 0..COMMIT_NOTIFICATION_CAPACITY + 3 {
            let committed = worker
                .capture_committed_async(
                    fixture_capture(&format!("fixture-{index}")),
                    ClipboardCommitCausality::capture(),
                )
                .await
                .expect("unblocked commit")
                .expect("item");
            assert_eq!(
                fast.receiver.try_recv().expect("independent event"),
                committed
            );
        }
        assert_eq!(slow.receiver.len(), COMMIT_NOTIFICATION_CAPACITY);
        assert_eq!(slow.dropped_count(), 3);
        assert_eq!(fast.dropped_count(), 0);
        worker.shutdown().expect("shutdown fixture");
    }

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
    fn migration_config_failure_keeps_worker_on_source_and_restart_config_unchanged() {
        let directory = tempfile::tempdir().expect("synthetic storage roots");
        let source = directory.path().join("source");
        let destination = directory.path().join("destination");
        fs::create_dir_all(&source).expect("create source");
        let config_path = directory.path().join("config-is-directory");
        fs::create_dir_all(&config_path).expect("force config commit failure");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            source.join("clipboard.db"),
            source.join("clipboard-images"),
            retention(),
            1_000,
        )
        .expect("start source worker");
        let failure = worker
            .migrate_storage(destination.clone(), config_path)
            .expect_err("config commit must fail");
        assert!(!failure.is_empty());
        assert_eq!(worker.database_path(), source.join("clipboard.db"));
        let async_runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("create count-query runtime");
        assert_eq!(
            async_runtime
                .block_on(worker.counts_async())
                .expect("source counts after failed commit")
                .total,
            0
        );
        assert!(source.join("clipboard.db").exists());
        worker.shutdown().expect("flush worker");
        let reopened = ClipboardHistory::open(source.join("clipboard.db")).expect("source restart");
        assert_eq!(reopened.counts().expect("reopened source counts").total, 0);
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
