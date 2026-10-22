//! Feature-owned extension seam. Shared composition supplies trust/current-run validation.
//! Existing worker/history/native copy owner remains authoritative; no capture listener is added.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use anyhow::{Result, bail, ensure};
use async_channel::{Receiver, Sender};
use gpui_kit::{App, Context, WeakEntity};
use nxtools_clipboard::{ClipboardItem, ClipboardKind};
use nxtools_protocol::{ClipboardCapture, ForegroundContext, PlatformCommandResult};
use uuid::Uuid;

use super::ClipboardApplication;
use crate::clipboard_worker::{
    ClipboardCaptureWorkerHandle, ClipboardCommitCausality, ClipboardCommitSubscription,
    ClipboardCommitted,
};

const MAX_CONTENT_BYTES: usize = 128 * 1024;
const MAX_PATHS: usize = 64;
const MAX_PATH_BYTES: usize = 4096;
const MAX_QUERY_BYTES: usize = 4096;
const MAX_PENDING_COPIES: usize = 8;
const COPY_DEADLINE: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClipboardExtensionContent {
    Text(String),
    Files(Vec<String>),
}

impl ClipboardExtensionContent {
    fn validate(&self) -> Result<()> {
        match self {
            Self::Text(text) => {
                ensure!(
                    !text.trim().is_empty() && !text.contains('\0'),
                    "invalid clipboard text"
                );
                ensure!(
                    text.len() <= MAX_CONTENT_BYTES,
                    "clipboard content budget exceeded"
                );
            }
            Self::Files(paths) => {
                ensure!(
                    !paths.is_empty() && paths.len() <= MAX_PATHS,
                    "invalid clipboard file count"
                );
                let mut bytes = 0_usize;
                for path in paths {
                    ensure!(
                        !path.trim().is_empty()
                            && !path.contains('\0')
                            && path.len() <= MAX_PATH_BYTES,
                        "invalid clipboard file reference"
                    );
                    bytes = bytes.saturating_add(path.len());
                }
                ensure!(
                    bytes <= MAX_CONTENT_BYTES,
                    "clipboard content budget exceeded"
                );
            }
        }
        Ok(())
    }

    fn capture(self, captured_at_unix_ms: i64) -> ClipboardCapture {
        let (text, paths) = match self {
            Self::Text(text) => (Some(text), Vec::new()),
            Self::Files(paths) => (None, paths),
        };
        ClipboardCapture {
            text,
            paths,
            image: None,
            native_formats: Vec::new(),
            source_icon_staging_path: None,
            source: ForegroundContext {
                window_handle: 0,
                focus_window_handle: 0,
                process_id: 0,
                process_name: String::new(),
                window_title: String::new(),
            },
            captured_at_unix_ms,
        }
    }
}

/// An item-bound reference, not an unrestricted filesystem path or inline image/native bytes.
/// Resolution/read leasing belongs to parent runtime integration; no image-read API is claimed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClipboardExtensionImageReference {
    pub item_id: i64,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClipboardExtensionSnapshotContent {
    Text(String),
    Files(Vec<String>),
    Image(ClipboardExtensionImageReference),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClipboardExtensionSnapshot {
    pub item_id: i64,
    pub content: ClipboardExtensionSnapshotContent,
}

impl ClipboardExtensionSnapshot {
    /// The parent supplies its immutable current selection, never a raw panel/entity handle.
    pub(crate) fn selected_item(item: &ClipboardItem) -> Result<Self> {
        ensure!(
            item.id > 0 && item.deleted_at_unix_ms.is_none(),
            "clipboard item unavailable"
        );
        let content = match item.kind {
            ClipboardKind::Text => {
                ClipboardExtensionContent::Text(item.text.clone()).validate()?;
                ClipboardExtensionSnapshotContent::Text(item.text.clone())
            }
            ClipboardKind::Files => {
                ClipboardExtensionContent::Files(item.file_paths.clone()).validate()?;
                ClipboardExtensionSnapshotContent::Files(item.file_paths.clone())
            }
            ClipboardKind::Image => {
                ClipboardExtensionSnapshotContent::Image(ClipboardExtensionImageReference {
                    item_id: item.id,
                    width: item.image_width,
                    height: item.image_height,
                })
            }
        };
        Ok(Self {
            item_id: item.id,
            content,
        })
    }

    fn payload_bytes(&self) -> usize {
        match &self.content {
            ClipboardExtensionSnapshotContent::Text(text) => text.len(),
            ClipboardExtensionSnapshotContent::Files(paths) => paths.iter().map(String::len).sum(),
            ClipboardExtensionSnapshotContent::Image(_) => 32,
        }
    }
}

#[derive(Clone)]
pub(crate) struct ClipboardExtensionAdapter {
    worker: ClipboardCaptureWorkerHandle,
}

impl ClipboardExtensionAdapter {
    pub(crate) fn new(worker: ClipboardCaptureWorkerHandle) -> Self {
        Self { worker }
    }

    pub(crate) fn subscribe_committed(&self) -> Result<ClipboardCommitSubscription> {
        self.worker.subscribe_committed()
    }

    pub(crate) async fn get(&self, item_id: i64) -> Result<ClipboardExtensionSnapshot> {
        ensure!(item_id > 0, "invalid clipboard item identity");
        let item = self.worker.extension_get_async(item_id).await?;
        ClipboardExtensionSnapshot::selected_item(&item)
    }

    /// Independent read lane: does not cancel or replace the popup's paging generation.
    /// Reject oversized results explicitly, never truncate private content as if complete.
    pub(crate) async fn query(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ClipboardExtensionSnapshot>> {
        ensure!(
            query.len() <= MAX_QUERY_BYTES && !query.contains('\0'),
            "invalid clipboard query"
        );
        ensure!((1..=32).contains(&limit), "invalid clipboard query limit");
        let items = self
            .worker
            .extension_query_async(query.to_owned(), limit)
            .await?;
        let snapshots = items
            .iter()
            .map(ClipboardExtensionSnapshot::selected_item)
            .collect::<Result<Vec<_>>>()?;
        ensure!(
            snapshots
                .iter()
                .map(ClipboardExtensionSnapshot::payload_bytes)
                .sum::<usize>()
                <= MAX_CONTENT_BYTES,
            "clipboard result budget exceeded"
        );
        Ok(snapshots)
    }

    /// Create/revive content through the sole worker. Source presence is revalidated on that
    /// worker before persistence; source payload is never edited, deleted or marked used.
    /// The supplied causality is host-owned, not copied unchecked from plugin JSON.
    pub(crate) async fn create_derived(
        &self,
        source_item_id: i64,
        content: ClipboardExtensionContent,
        mut causality: ClipboardCommitCausality,
    ) -> Result<ClipboardCommitted> {
        content.validate()?;
        ensure!(
            source_item_id > 0 && causality.origin_plugin.is_some(),
            "invalid derived clipboard origin"
        );
        causality.derived_from = Some(source_item_id);
        causality.validate()?;
        self.worker
            .capture_committed_async(content.capture(nxtools_ipc::unix_time_ms()), causality)
            .await?
            .ok_or_else(|| anyhow::anyhow!("derived clipboard content was empty"))
    }

    /// All native dispatch occurs on a later application turn. The receiver returns Windows
    /// completion (not queue admission), and cancellation cannot undo an already applied copy.
    pub(crate) fn copy_deferred(
        application: WeakEntity<ClipboardApplication>,
        content: ClipboardExtensionContent,
        causality: ClipboardCommitCausality,
        cancelled: Arc<AtomicBool>,
        cx: &mut App,
    ) -> Receiver<Result<ClipboardExtensionCopyCompleted>> {
        let (reply, receiver) = async_channel::bounded(1);
        cx.defer(move |cx| {
            let reply_on_missing = reply.clone();
            if application
                .update(cx, move |app, cx| {
                    app.start_extension_copy(content, causality, cancelled, reply, cx);
                })
                .is_err()
            {
                // No consumer callback runs under either the original or application lease.
                cx.defer(move |_| {
                    let _ =
                        reply_on_missing.try_send(Err(anyhow::anyhow!("clipboard UI unavailable")));
                });
            }
        });
        receiver
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClipboardExtensionCopyCompleted {
    pub operation_id: Uuid,
    pub root_operation_id: Uuid,
    pub native_error_code: Option<i32>,
}

pub(super) struct PendingExtensionCopy {
    reply: Sender<Result<ClipboardExtensionCopyCompleted>>,
    cancelled: Arc<AtomicBool>,
    started_at: Instant,
    root_operation_id: Uuid,
    _watchdog: Option<gpui_kit::Task<()>>,
}

impl ClipboardApplication {
    #[cfg(test)]
    pub(crate) fn extension_adapter(&self) -> ClipboardExtensionAdapter {
        ClipboardExtensionAdapter::new(self.history_worker.clone())
    }

    fn start_extension_copy(
        &mut self,
        content: ClipboardExtensionContent,
        causality: ClipboardCommitCausality,
        cancelled: Arc<AtomicBool>,
        reply: Sender<Result<ClipboardExtensionCopyCompleted>>,
        cx: &mut Context<Self>,
    ) {
        let operation_id = Uuid::new_v4();
        let root_operation_id = causality.root_operation_id;
        tracing::info!(event = "clipboard.extension_copy.started", %operation_id, %root_operation_id, "extension copy started");
        let result = (|| -> Result<()> {
            content.validate()?;
            causality.validate()?;
            ensure!(
                causality.origin_plugin.is_some(),
                "invalid clipboard copy origin"
            );
            ensure!(
                !cancelled.load(Ordering::Acquire),
                "clipboard operation cancelled"
            );
            ensure!(
                !self.management_operation_active,
                "clipboard maintenance active"
            );
            ensure!(
                self.pending_extension_copies.len() < MAX_PENDING_COPIES,
                "clipboard copy limit reached"
            );
            ensure!(
                !self.pending_extension_copies.contains_key(&operation_id),
                "clipboard operation already pending"
            );
            let capture = content.capture(nxtools_ipc::unix_time_ms());
            let (kind, text, paths) = if capture.paths.is_empty() {
                (
                    ClipboardKind::Text,
                    capture.text.unwrap_or_default(),
                    Vec::new(),
                )
            } else {
                (ClipboardKind::Files, String::new(), capture.paths)
            };
            let entry = ClipboardItem {
                id: 0,
                kind,
                text,
                file_paths: paths,
                image_path: String::new(),
                image_width: 0,
                image_height: 0,
                source: nxtools_clipboard::CaptureSource {
                    window_handle: 0,
                    process_id: 0,
                    process_name: String::new(),
                    process_icon_path: String::new(),
                    window_title: String::new(),
                },
                captured_at_unix_ms: capture.captured_at_unix_ms,
                last_used_at_unix_ms: capture.captured_at_unix_ms,
                copy_count: 0,
                is_favorite: false,
                is_pinned: false,
                deleted_at_unix_ms: None,
            };
            let target = capture.source;
            // Pending ownership precedes native dispatch, just as for the ordinary popup action.
            self.pending_extension_copies.insert(
                operation_id,
                PendingExtensionCopy {
                    reply: reply.clone(),
                    cancelled: Arc::clone(&cancelled),
                    started_at: Instant::now(),
                    root_operation_id,
                    _watchdog: None,
                },
            );
            if self
                .send_clipboard_action_with_native_formats(
                    &entry,
                    &target,
                    nxtools_clipboard::ClipboardAction::CopyOnly,
                    Vec::new(),
                    operation_id,
                )
                .is_err()
            {
                self.pending_extension_copies.remove(&operation_id);
                bail!("clipboard native dispatch failed");
            }
            Ok(())
        })();
        if let Err(error) = result {
            tracing::warn!(event = "clipboard.extension_copy.rejected", %operation_id, reason = "validation_or_dispatch", "extension copy rejected");
            cx.defer(move |_| {
                let _ = reply.try_send(Err(error));
            });
            return;
        }
        let watchdog = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPY_DEADLINE).await;
            let _ = this.update(cx, |app, cx| {
                if let Some(pending) = app.pending_extension_copies.remove(&operation_id) {
                    tracing::warn!(event = "clipboard.extension_copy.failed", %operation_id, reason = "timeout", elapsed_ms = pending.started_at.elapsed().as_millis() as u64, "extension copy expired");
                    cx.defer(move |_| { let _ = pending.reply.try_send(Err(anyhow::anyhow!("clipboard copy timed out"))); });
                }
            });
        });
        if let Some(pending) = self.pending_extension_copies.get_mut(&operation_id) {
            pending._watchdog = Some(watchdog);
        }
    }

    pub(super) fn complete_extension_copy(
        &mut self,
        operation_id: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pending) = self.pending_extension_copies.remove(&operation_id) else {
            return false;
        };
        let cancelled = pending.cancelled.load(Ordering::Acquire);
        tracing::info!(event = "clipboard.extension_copy.completed", %operation_id, root_operation_id = %pending.root_operation_id, success = result.success, cancelled, native_error_code = ?result.native_error_code, elapsed_ms = pending.started_at.elapsed().as_millis() as u64, "extension copy terminal");
        let terminal = if cancelled {
            Err(anyhow::anyhow!("clipboard operation cancelled"))
        } else if result.success {
            Ok(ClipboardExtensionCopyCompleted {
                operation_id,
                root_operation_id: pending.root_operation_id,
                native_error_code: result.native_error_code,
            })
        } else {
            Err(anyhow::anyhow!("clipboard native copy failed"))
        };
        cx.defer(move |_| {
            let _ = pending.reply.try_send(terminal);
        });
        true
    }

    pub(super) fn abandon_extension_copies(&mut self, cx: &mut Context<Self>) {
        for (operation_id, pending) in self.pending_extension_copies.drain() {
            tracing::warn!(event = "clipboard.extension_copy.failed", %operation_id, reason = "disconnected", "extension copy invalidated");
            cx.defer(move |_| {
                let _ = pending
                    .reply
                    .try_send(Err(anyhow::anyhow!("clipboard platform disconnected")));
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clipboard_worker::ClipboardCommitChange;

    #[tokio::test]
    async fn derived_entry_uses_real_worker_preserves_original_and_carries_causality() {
        let directory = tempfile::tempdir().expect("owned clipboard fixture");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("assets"),
            nxtools_clipboard::ClipboardRetentionPolicy {
                max_items: 1000,
                retention_days: 30,
            },
            nxtools_ipc::unix_time_ms(),
        )
        .expect("fixture worker");
        let adapter = ClipboardExtensionAdapter::new(worker.clone());
        let events = adapter.subscribe_committed().expect("subscription");
        let original = worker
            .capture_committed_async(
                ClipboardExtensionContent::Text("original fixture".into())
                    .capture(nxtools_ipc::unix_time_ms()),
                ClipboardCommitCausality::capture(),
            )
            .await
            .expect("original persisted")
            .expect("original item");
        events.receiver.try_recv().expect("original event");
        let before = adapter
            .get(original.item_id)
            .await
            .expect("original snapshot");
        let mut cause = ClipboardCommitCausality::capture();
        cause.origin_plugin = Some("fixture.plugin".into());
        cause.invoked_handlers = vec!["fixture.plugin/transform".into()];
        cause.depth = 1;
        let connection = rusqlite::Connection::open(worker.database_path())
            .expect("owned database failure fixture");
        connection.execute_batch("CREATE TRIGGER reject_derived BEFORE INSERT ON clipboard_items BEGIN SELECT RAISE(ABORT, 'fixture rejection'); END;").expect("inject persistence failure");
        assert!(
            adapter
                .create_derived(
                    original.item_id,
                    ClipboardExtensionContent::Text("failed derived fixture".into()),
                    cause.clone()
                )
                .await
                .is_err()
        );
        assert!(
            events.receiver.try_recv().is_err(),
            "failed derived persistence emits no commit"
        );
        assert_eq!(
            adapter
                .get(original.item_id)
                .await
                .expect("original after database failure"),
            before
        );
        connection
            .execute_batch("DROP TRIGGER reject_derived;")
            .expect("clear fixture failure");
        let derived = adapter
            .create_derived(
                original.item_id,
                ClipboardExtensionContent::Text("derived fixture".into()),
                cause.clone(),
            )
            .await
            .expect("derived persisted");
        assert_ne!(derived.item_id, original.item_id);
        assert_eq!(derived.change, ClipboardCommitChange::Inserted);
        assert_eq!(derived.causality.derived_from, Some(original.item_id));
        assert_eq!(derived.causality.origin_plugin, cause.origin_plugin);
        assert_eq!(derived.causality.invoked_handlers, cause.invoked_handlers);
        assert_eq!(events.receiver.try_recv().expect("derived event"), derived);
        assert_eq!(
            adapter
                .get(original.item_id)
                .await
                .expect("original remains"),
            before
        );
        let duplicate = adapter
            .create_derived(
                original.item_id,
                ClipboardExtensionContent::Text("derived fixture".into()),
                cause.clone(),
            )
            .await
            .expect("derived duplicate");
        assert_eq!(duplicate.item_id, derived.item_id);
        assert_eq!(duplicate.change, ClipboardCommitChange::Updated);
        assert_eq!(
            events.receiver.try_recv().expect("duplicate event"),
            duplicate
        );
        assert!(
            adapter
                .create_derived(
                    original.item_id,
                    ClipboardExtensionContent::Text("original fixture".into()),
                    cause.clone()
                )
                .await
                .is_err()
        );
        assert_eq!(
            adapter
                .get(original.item_id)
                .await
                .expect("original still preserved"),
            before
        );
        let files = adapter
            .create_derived(
                original.item_id,
                ClipboardExtensionContent::Files(vec!["C:\\owned-fixture\\result.txt".into()]),
                cause.clone(),
            )
            .await
            .expect("derived file references");
        assert_eq!(files.kind, ClipboardKind::Files);
        assert!(matches!(
            adapter
                .get(files.item_id)
                .await
                .expect("file snapshot")
                .content,
            ClipboardExtensionSnapshotContent::Files(_)
        ));
        assert_eq!(events.receiver.try_recv().expect("files commit"), files);
        assert_eq!(
            adapter
                .query("fixture", 32)
                .await
                .expect("real query")
                .len(),
            3
        );
        assert!(
            adapter
                .create_derived(
                    i64::MAX,
                    ClipboardExtensionContent::Text("missing source".into()),
                    cause.clone()
                )
                .await
                .is_err()
        );
        assert!(
            adapter
                .create_derived(
                    original.item_id,
                    ClipboardExtensionContent::Text(" ".into()),
                    cause.clone()
                )
                .await
                .is_err()
        );
        cause
            .invoked_handlers
            .push("fixture.plugin/transform".into());
        assert!(
            adapter
                .create_derived(
                    original.item_id,
                    ClipboardExtensionContent::Text("recursive fixture".into()),
                    cause
                )
                .await
                .is_err()
        );
        assert!(events.receiver.try_recv().is_err());
        assert_eq!(worker.counts_async().await.expect("counts").total, 3);
        worker.shutdown().expect("shutdown fixture");
    }

    #[gpui_kit::test]
    fn replacement_copy_defers_dispatch_and_terminal_delivery_without_touching_history(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::AppContext;
        use std::rc::Rc;
        let directory = tempfile::tempdir().expect("owned clipboard UI fixture");
        let worker = ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("clipboard.db"),
            directory.path().join("assets"),
            nxtools_clipboard::ClipboardRetentionPolicy {
                max_items: 1000,
                retention_days: 30,
            },
            nxtools_ipc::unix_time_ms(),
        )
        .expect("fixture worker");
        let (platform, endpoint) = nxtools_ipc::platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let application = cx.update(|cx| {
            let notifications = cx.new(|_| crate::notifications::NotificationCenter::default());
            cx.new(|cx| {
                ClipboardApplication::new(
                    worker.clone(),
                    Rc::new(platform),
                    nxtools_core::AppConfig::default(),
                    directory.path().join("config.json"),
                    notifications,
                    cx,
                )
            })
        });
        let _adapter = cx.read(|cx| application.read(cx).extension_adapter());
        let mut causality = ClipboardCommitCausality::capture();
        causality.origin_plugin = Some("fixture.plugin".into());
        let root_operation_id = causality.root_operation_id;
        // Deliberately call from inside the destination application's current lease. Removing
        // the first defer would reenter this entity before the callback releases it.
        let reply = application.update(cx, |app, cx| {
            let reply = ClipboardExtensionAdapter::copy_deferred(
                cx.entity().downgrade(),
                ClipboardExtensionContent::Text("replacement fixture".into()),
                causality,
                Arc::new(AtomicBool::new(false)),
                cx,
            );
            assert!(app.pending_extension_copies.is_empty());
            assert!(endpoint.try_receive_command_for_contract_test().is_err());
            reply
        });
        cx.run_until_parked();
        let envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("deferred native command");
        assert!(matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(
                nxtools_protocol::PlatformCommand::PasteText(nxtools_protocol::PasteTextCommand {
                    paste_after_write: false,
                    ..
                })
            )
        ));
        assert!(reply.try_recv().is_err(), "admission is not native success");
        application.update(cx, |app, cx| {
            app.handle_platform_notification(
                nxtools_ipc::PlatformNotification::CommandResult {
                    correlation_id: envelope.id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: None,
                    },
                },
                cx,
            );
            assert!(
                reply.try_recv().is_err(),
                "no terminal callback under application lease"
            );
            assert!(app.pending_extension_copies.is_empty());
        });
        cx.run_until_parked();
        let terminal = reply
            .try_recv()
            .expect("deferred completion")
            .expect("native success");
        assert_eq!(terminal.operation_id, envelope.id);
        assert_eq!(terminal.root_operation_id, root_operation_id);
        let cancelled = Arc::new(AtomicBool::new(true));
        let mut cancelled_cause = ClipboardCommitCausality::capture();
        cancelled_cause.origin_plugin = Some("fixture.plugin".into());
        let cancelled_reply = cx.update(|cx| {
            ClipboardExtensionAdapter::copy_deferred(
                application.downgrade(),
                ClipboardExtensionContent::Files(vec!["C:\\owned-fixture\\cancelled.txt".into()]),
                cancelled_cause,
                cancelled,
                cx,
            )
        });
        cx.run_until_parked();
        assert!(
            cancelled_reply
                .try_recv()
                .expect("cancel terminal")
                .is_err()
        );
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "cancel before dispatch sends no native command"
        );
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("fixture runtime");
        assert_eq!(
            runtime
                .block_on(worker.counts_async())
                .expect("history counts")
                .total,
            0
        );
        worker.shutdown().expect("shutdown fixture");
    }
}
