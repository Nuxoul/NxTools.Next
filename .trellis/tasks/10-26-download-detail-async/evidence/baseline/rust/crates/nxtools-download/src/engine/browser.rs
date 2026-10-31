//! Browser admission, persistent handoff replay and private confirmation-window orchestration.
//! Blob publication and its database compensation stay in the same admission boundary.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use tracing::{debug, error, info, warn};

use super::{
    BrowserDownloadDialogRequest, BrowserDownloadDialogSource, DownloadEngine, DownloadEngineError,
    EngineTaskState, apply_organization_to_request, now_unix_ms,
};
use crate::storage::{BrowserHandoffSave, StoredTransferState};
use crate::{
    BrowserBatchSubmissionResult, BrowserBlob, BrowserDownloadSubmission, DownloadProbe,
    DownloadRequest, DownloadRequestError, DownloadStatus, DownloadStorageError,
    DownloadSubmitMode, DownloadTask, DownloadTaskId, MAX_BROWSER_BATCH_ITEMS, RequestOrigin,
};

pub(super) struct BrowserBlobImportRequest {
    pub(super) handoff_id: uuid::Uuid,
    pub(super) blob: BrowserBlob,
    pub(super) save_directory: PathBuf,
    pub(super) file_name: String,
    pub(super) category_id: Option<String>,
    pub(super) save_directory_explicit: bool,
}

impl DownloadEngine {
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

    fn resolve_existing_browser_handoff(
        &self,
        handoff_id: uuid::Uuid,
        request_fingerprint: &str,
        correlation_id: uuid::Uuid,
    ) -> Result<Option<DownloadTaskId>, DownloadEngineError> {
        let Some(existing) = self.inner.store.browser_handoff(handoff_id)? else {
            return Ok(None);
        };
        if existing
            .request_fingerprint
            .as_deref()
            .is_some_and(|fingerprint| fingerprint != request_fingerprint)
        {
            warn!(
                event = "download.browser_handoff.conflict",
                %correlation_id,
                %handoff_id,
                ?existing.task_id,
                "browser handoff replay conflicts with the accepted request"
            );
            return Err(DownloadEngineError::BrowserHandoffConflict(handoff_id));
        }
        if existing.request_fingerprint.is_none()
            && !self
                .inner
                .store
                .backfill_browser_handoff_fingerprint(handoff_id, request_fingerprint)?
        {
            let Some(refreshed) = self.inner.store.browser_handoff(handoff_id)? else {
                return Ok(None);
            };
            if refreshed
                .request_fingerprint
                .as_deref()
                .is_some_and(|fingerprint| fingerprint != request_fingerprint)
            {
                warn!(
                    event = "download.browser_handoff.conflict",
                    %correlation_id,
                    %handoff_id,
                    ?refreshed.task_id,
                    "concurrent legacy browser handoff replay claimed a different request"
                );
                return Err(DownloadEngineError::BrowserHandoffConflict(handoff_id));
            }
            return Ok(Some(refreshed.task_id));
        }
        Ok(Some(existing.task_id))
    }

    pub(super) fn submit_browser_correlated(
        &self,
        handoff_id: uuid::Uuid,
        request: DownloadRequest,
        mode: DownloadSubmitMode,
        correlation_id: uuid::Uuid,
    ) -> Result<(DownloadTaskId, bool), DownloadEngineError> {
        self.submit_browser_with_resolution(
            handoff_id,
            request,
            mode,
            correlation_id,
            crate::DownloadResolutionOptions {
                operation_id: correlation_id,
                mode: crate::DownloadResolutionMode::Resolve,
            },
            crate::DownloadResolutionControl::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn submit_browser_with_resolution(
        &self,
        handoff_id: uuid::Uuid,
        mut request: DownloadRequest,
        mode: DownloadSubmitMode,
        correlation_id: uuid::Uuid,
        resolution: crate::DownloadResolutionOptions,
        control: crate::DownloadResolutionControl,
    ) -> Result<(DownloadTaskId, bool), DownloadEngineError> {
        let request_fingerprint = crate::storage::request_fingerprint(&request)?;
        if let Some(task_id) =
            self.resolve_existing_browser_handoff(handoff_id, &request_fingerprint, correlation_id)?
        {
            info!(
                event = "download.browser_handoff.duplicate_accepted",
                %correlation_id,
                %handoff_id,
                ?task_id,
                "duplicate browser handoff resolved before request replay"
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
        if let Some(task_id) =
            self.resolve_existing_browser_handoff(handoff_id, &request_fingerprint, correlation_id)?
        {
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
        let resolution_acceptance = self.lock_resolution_acceptance(&prepared)?;
        let now = now_unix_ms();
        let mut task = DownloadTask::new(request, RequestOrigin::Browser, now);
        task.set_resolution(prepared.provenance.clone());
        let transfer = StoredTransferState::default();
        let delayed_until_due = task
            .request()
            .scheduled_at_unix_ms
            .is_some_and(|scheduled_at| scheduled_at > now);
        let saved = self
            .inner
            .store
            .save_browser_task_with_transfer_and_queue(
                handoff_id,
                &request_fingerprint,
                &task,
                &transfer,
                "default",
                mode == DownloadSubmitMode::StartNow && !delayed_until_due,
            )
            .map_err(|error| match error {
                DownloadStorageError::BrowserHandoffConflict { .. } => {
                    DownloadEngineError::BrowserHandoffConflict(handoff_id)
                }
                error => error.into(),
            })?;
        drop(resolution_acceptance);
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

    pub(super) fn submit_browser_batch_correlated(
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

    pub(super) fn import_browser_blob_correlated(
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
        let request_fingerprint = crate::storage::request_fingerprint(&request)?;
        if let Some(task_id) =
            self.resolve_existing_browser_handoff(handoff_id, &request_fingerprint, correlation_id)?
        {
            info!(
                event = "download.browser_blob.duplicate_accepted",
                %correlation_id,
                %handoff_id,
                ?task_id,
                "duplicate browser Blob import resolved before staging access"
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
        request.validate()?;
        let _scheduler = self
            .inner
            .scheduler_gate
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?;
        if let Some(task_id) =
            self.resolve_existing_browser_handoff(handoff_id, &request_fingerprint, correlation_id)?
        {
            info!(
                event = "download.browser_blob.duplicate_accepted",
                %correlation_id,
                %handoff_id,
                ?task_id,
                "duplicate browser Blob import resolved after staging checks"
            );
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
                resolution: None,
            }),
            downloaded_bytes: blob.size,
            error: None,
            temporary_path: None,
            final_path: Some(final_path.clone()),
            final_committed: true,
        };
        let saved = persist_browser_blob_with_compensation(&staged_path, &final_path, || {
            self.inner.store.save_browser_task_with_transfer_and_queue(
                handoff_id,
                &request_fingerprint,
                &task,
                &transfer,
                "default",
                false,
            )
        });
        if let Err(error) = saved {
            match error {
                BrowserBlobPersistenceError::Persistence(
                    DownloadStorageError::BrowserHandoffConflict {
                        handoff_id: conflict,
                    },
                ) if conflict == handoff_id => {
                    return Err(DownloadEngineError::BrowserHandoffConflict(handoff_id));
                }
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

    pub(super) fn discard_browser_blob(
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

    pub(super) fn open_browser_dialog_correlated(
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

    pub(super) fn open_browser_blob_dialog_correlated(
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
pub(super) enum BrowserBlobPersistenceError {
    Persistence(DownloadStorageError),
    Rollback {
        persistence: DownloadStorageError,
        rollback: Box<DownloadEngineError>,
    },
}

pub(super) fn persist_browser_blob_with_compensation(
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
