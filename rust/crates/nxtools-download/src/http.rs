use std::path::PathBuf;
use std::time::{Duration, Instant};

use reqwest::header::{
    ACCEPT_ENCODING, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, COOKIE,
    ETAG, HeaderName, HeaderValue, IF_RANGE, LAST_MODIFIED, RANGE, REFERER,
};
use reqwest::{Client, Method, Response, StatusCode};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::{
    DownloadProbe, DownloadRateLimitError, DownloadRateLimiter, DownloadRequest,
    DownloadRequestError, DownloadSegment, DownloadTaskId, ResourceValidator, SegmentedArtifact,
    SegmentedArtifactError, is_safe_windows_file_name,
};

const PROBE_RANGE: &str = "bytes=0-0";
const DEFAULT_STALL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct HttpDownloadClient {
    client: Client,
    stall_timeout: Duration,
    #[cfg(test)]
    injected_artifact_fault: Option<InjectedArtifactFault>,
}

impl HttpDownloadClient {
    pub fn new() -> Result<Self, HttpDownloadError> {
        Self::with_stall_timeout(DEFAULT_STALL_TIMEOUT)
    }

    pub fn with_stall_timeout(stall_timeout: Duration) -> Result<Self, HttpDownloadError> {
        if stall_timeout.is_zero() {
            return Err(HttpDownloadError::InvalidStallTimeout);
        }
        let client = Client::builder()
            .user_agent(concat!("NxTools.Next/", env!("CARGO_PKG_VERSION")))
            .redirect(reqwest::redirect::Policy::limited(10))
            .read_timeout(stall_timeout)
            .connect_timeout(stall_timeout)
            .build()?;
        Ok(Self {
            client,
            stall_timeout,
            #[cfg(test)]
            injected_artifact_fault: None,
        })
    }

    pub async fn probe(
        &self,
        request: &DownloadRequest,
    ) -> Result<DownloadProbe, HttpDownloadError> {
        request.validate()?;

        let head = self.send(Method::HEAD, request, false).await;
        let head = match head {
            Ok(response) if response.status().is_success() => Some(response),
            Ok(_) | Err(_) => None,
        };

        let head_metadata = head.as_ref().map(ProbeMetadata::from_response);
        let needs_range_probe = head_metadata
            .as_ref()
            .is_none_or(|metadata| metadata.content_length.is_none() || metadata.advertises_ranges);
        let mut ranged_failure = None;
        let ranged = if needs_range_probe {
            let ranged = self.send(Method::GET, request, true).await;
            match ranged {
                Ok(response) if response.status().is_success() => Some(response),
                Ok(response) if head.is_none() => {
                    ranged_failure = Some(HttpDownloadError::Status(response.status().as_u16()));
                    None
                }
                Err(error) if head.is_none() => {
                    ranged_failure = Some(error);
                    None
                }
                Ok(_) | Err(_) => None,
            }
        } else {
            None
        };
        let fallback = if head.is_none() && ranged.is_none() {
            match self.send(Method::GET, request, false).await {
                Ok(response) if response.status().is_success() => Some(response),
                Ok(response) => {
                    ranged_failure = Some(HttpDownloadError::Status(response.status().as_u16()));
                    None
                }
                Err(error) => {
                    ranged_failure = Some(error);
                    None
                }
            }
        } else {
            None
        };
        if head.is_none() && ranged.is_none() && fallback.is_none() {
            return Err(ranged_failure
                .unwrap_or(HttpDownloadError::Status(StatusCode::BAD_GATEWAY.as_u16())));
        }
        let valid_range = ranged.as_ref().is_some_and(valid_probe_range);
        let range_metadata = ranged.as_ref().map(ProbeMetadata::from_response);
        let fallback_metadata = fallback.as_ref().map(ProbeMetadata::from_response);
        let head_metadata = head.as_ref().map(ProbeMetadata::from_response);
        let metadata = [head_metadata, range_metadata, fallback_metadata]
            .into_iter()
            .flatten()
            .reduce(ProbeMetadata::merge)
            .expect("probe metadata");
        let response = ranged
            .as_ref()
            .or(fallback.as_ref())
            .or(head.as_ref())
            .expect("probe response");

        Ok(DownloadProbe {
            final_url: response.url().to_string(),
            file_name: Some(resolve_file_name(
                metadata.content_disposition.as_deref(),
                response.url(),
            )),
            total_bytes: metadata.total_bytes(valid_range),
            supports_ranges: valid_range,
            content_type: metadata.content_type,
            validator: ResourceValidator {
                etag: metadata.etag,
                last_modified: metadata.last_modified,
            },
        })
    }

    pub async fn download<F>(
        &self,
        request: &DownloadRequest,
        probe: &DownloadProbe,
        task_id: DownloadTaskId,
        control: &DownloadTransferControl,
        mut on_progress: F,
    ) -> Result<DownloadTransferOutcome, HttpDownloadError>
    where
        F: FnMut(DownloadTransferProgress),
    {
        match self
            .prepare_download(request, probe, task_id, control, &mut on_progress)
            .await?
        {
            PreparedDownloadOutcome::Ready(ready) => {
                let final_path = ready.final_path.clone();
                let downloaded_bytes = ready.downloaded_bytes;
                self.commit_prepared(ready).await?;
                Ok(DownloadTransferOutcome::Completed {
                    final_path,
                    downloaded_bytes,
                })
            }
            PreparedDownloadOutcome::Paused {
                temporary_path,
                downloaded_bytes,
            } => Ok(DownloadTransferOutcome::Paused {
                temporary_path,
                downloaded_bytes,
            }),
            PreparedDownloadOutcome::Cancelled { downloaded_bytes } => {
                Ok(DownloadTransferOutcome::Cancelled { downloaded_bytes })
            }
        }
    }

    pub async fn download_segment<F>(
        &self,
        transfer: DownloadSegmentTransferRequest<'_>,
        mut on_progress: F,
    ) -> Result<DownloadSegmentTransferOutcome, HttpDownloadError>
    where
        F: FnMut(DownloadSegmentTransferProgress),
    {
        let DownloadSegmentTransferRequest {
            context,
            request,
            probe,
            segment,
            artifact,
            control,
            use_if_range,
        } = transfer;
        let started = Instant::now();
        info!(
            event = "download.segment_transfer.started",
            correlation_id = %context.correlation_id,
            task_id = ?context.task_id,
            segment_index = segment.index(),
            resumed_bytes = segment.downloaded_bytes(),
            conditional_resume = use_if_range,
            "segment transfer started"
        );
        let result = async {
            request.validate()?;
            let total_bytes = probe
                .total_bytes
                .ok_or(HttpDownloadError::SegmentedTransferNeedsKnownLength)?;
            let requested_start = segment
                .start_byte()
                .checked_add(segment.downloaded_bytes())
                .ok_or(HttpDownloadError::LengthOverflow)?;
            if requested_start > segment.end_byte() {
                return Ok(DownloadSegmentTransferOutcome::Completed {
                    downloaded_bytes: segment.downloaded_bytes(),
                });
            }

            let mut directives = control.subscribe_task();
            let mut rebalances = control.subscribe_rebalance();
            match *directives.borrow() {
                TransferDirective::Running => {}
                TransferDirective::Pause => {
                    return Ok(DownloadSegmentTransferOutcome::Paused {
                        downloaded_bytes: segment.downloaded_bytes(),
                    });
                }
                TransferDirective::Cancel => {
                    return Ok(DownloadSegmentTransferOutcome::Cancelled {
                        downloaded_bytes: segment.downloaded_bytes(),
                    });
                }
            }
            if *rebalances.borrow() {
                return Ok(DownloadSegmentTransferOutcome::Rebalanced {
                    downloaded_bytes: segment.downloaded_bytes(),
                });
            }
            let response_request = self.send_range(
                request,
                &probe.final_url,
                requested_start,
                segment.end_byte(),
                &probe.validator,
                use_if_range,
            );
            tokio::pin!(response_request);
            let mut response = tokio::select! {
                biased;
                changed = directives.changed() => {
                    let _ = changed;
                    let directive = *directives.borrow();
                    match directive {
                        TransferDirective::Running => response_request.await?,
                        TransferDirective::Pause => {
                            return Ok(DownloadSegmentTransferOutcome::Paused {
                                downloaded_bytes: segment.downloaded_bytes(),
                            });
                        }
                        TransferDirective::Cancel => {
                            return Ok(DownloadSegmentTransferOutcome::Cancelled {
                                downloaded_bytes: segment.downloaded_bytes(),
                            });
                        }
                    }
                }
                changed = rebalances.changed() => {
                    let _ = changed;
                    return Ok(DownloadSegmentTransferOutcome::Rebalanced {
                        downloaded_bytes: segment.downloaded_bytes(),
                    });
                }
                result = &mut response_request => result?,
                _ = tokio::time::sleep(self.stall_timeout) => {
                    return Err(HttpDownloadError::Stalled {
                        timeout_ms: duration_millis(self.stall_timeout),
                    });
                }
            };
            validate_segment_response(
                &response,
                requested_start,
                segment.end_byte(),
                total_bytes,
                &probe.validator,
                use_if_range,
            )?;
            info!(
                event = "download.segment_response.accepted",
                correlation_id = %context.correlation_id,
                task_id = ?context.task_id,
                segment_index = segment.index(),
                response_status = response.status().as_u16(),
                requested_start,
                requested_end = segment.end_byte(),
                validator_policy = if use_if_range { "resume_strict" } else { "fresh_range_only" },
                "segment response passed range validation"
            );

            let mut writer = artifact.open_segment(segment).await?;
            let mut downloaded_bytes = segment.downloaded_bytes();
            loop {
                let directive = *directives.borrow();
                match directive {
                    TransferDirective::Running => {}
                    TransferDirective::Pause => {
                        writer.sync().await?;
                        return Ok(DownloadSegmentTransferOutcome::Paused { downloaded_bytes });
                    }
                    TransferDirective::Cancel => {
                        writer.sync().await?;
                        return Ok(DownloadSegmentTransferOutcome::Cancelled { downloaded_bytes });
                    }
                }
                tokio::select! {
                    biased;
                    changed = directives.changed() => {
                        if changed.is_err() {
                            continue;
                        }
                    }
                    changed = rebalances.changed() => {
                        let _ = changed;
                        writer.sync().await?;
                        return Ok(DownloadSegmentTransferOutcome::Rebalanced {
                            downloaded_bytes,
                        });
                    }
                    _ = tokio::time::sleep(self.stall_timeout) => {
                        return Err(HttpDownloadError::Stalled {
                            timeout_ms: duration_millis(self.stall_timeout),
                        });
                    }
                    chunk = response.chunk() => {
                        let Some(chunk) = chunk.map_err(|error| self.client_error(error))? else {
                            break;
                        };
                        let bandwidth = control.acquire_bandwidth(chunk.len() as u64);
                        tokio::pin!(bandwidth);
                        tokio::select! {
                            biased;
                            changed = directives.changed() => {
                                let _ = changed;
                                let directive = *directives.borrow();
                                match directive {
                                    TransferDirective::Running => bandwidth.await,
                                    TransferDirective::Pause => {
                                        writer.sync().await?;
                                        return Ok(DownloadSegmentTransferOutcome::Paused {
                                            downloaded_bytes,
                                        });
                                    }
                                    TransferDirective::Cancel => {
                                        writer.sync().await?;
                                        return Ok(DownloadSegmentTransferOutcome::Cancelled {
                                            downloaded_bytes,
                                        });
                                    }
                                }
                            }
                            changed = rebalances.changed() => {
                                let _ = changed;
                                writer.sync().await?;
                                return Ok(DownloadSegmentTransferOutcome::Rebalanced {
                                    downloaded_bytes,
                                });
                            }
                            () = &mut bandwidth => {}
                        }
                        writer.write_all(&chunk).await?;
                        downloaded_bytes = downloaded_bytes
                            .checked_add(chunk.len() as u64)
                            .ok_or(HttpDownloadError::LengthOverflow)?;
                        on_progress(DownloadSegmentTransferProgress {
                            segment_index: segment.index(),
                            downloaded_bytes,
                            segment_bytes: segment.len(),
                        });
                    }
                }
            }
            if downloaded_bytes != segment.len() {
                return Err(HttpDownloadError::LengthMismatch {
                    expected_bytes: segment.len(),
                    downloaded_bytes,
                });
            }
            writer.sync().await?;
            Ok(DownloadSegmentTransferOutcome::Completed { downloaded_bytes })
        }
        .await;

        match &result {
            Ok(outcome) => info!(
                event = "download.segment_transfer.completed",
                correlation_id = %context.correlation_id,
                task_id = ?context.task_id,
                segment_index = segment.index(),
                outcome = outcome.name(),
                downloaded_bytes = outcome.downloaded_bytes(),
                elapsed_ms = started.elapsed().as_millis(),
                "segment transfer completed"
            ),
            Err(error) => warn!(
                event = "download.segment_transfer.failed",
                correlation_id = %context.correlation_id,
                task_id = ?context.task_id,
                segment_index = segment.index(),
                error_kind = error.kind(),
                error = %error,
                requested_start = segment.start_byte().saturating_add(segment.downloaded_bytes()),
                requested_end = segment.end_byte(),
                conditional_resume = use_if_range,
                elapsed_ms = started.elapsed().as_millis(),
                "segment transfer failed"
            ),
        }
        result
    }

    pub(crate) async fn prepare_download<F>(
        &self,
        request: &DownloadRequest,
        probe: &DownloadProbe,
        task_id: DownloadTaskId,
        control: &DownloadTransferControl,
        mut on_progress: F,
    ) -> Result<PreparedDownloadOutcome, HttpDownloadError>
    where
        F: FnMut(DownloadTransferProgress),
    {
        request.validate()?;
        let (temporary_path, final_path) = Self::artifact_paths(request, probe, task_id)?;
        #[cfg(test)]
        if let Some(fault) = self.injected_artifact_fault {
            return Err(HttpDownloadError::Artifact(SegmentedArtifactError::Io(
                fault.into_io_error(),
            )));
        }
        tokio::fs::create_dir_all(&request.save_directory).await?;

        if tokio::fs::try_exists(&final_path).await? {
            return Err(HttpDownloadError::DestinationExists(final_path));
        }
        let mut output = tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary_path)
            .await?;

        let mut directives = control.subscribe();
        let response_request = self.send(Method::GET, request, false);
        tokio::pin!(response_request);
        let mut response = tokio::select! {
            biased;
            changed = directives.changed() => {
                let _ = changed;
                let directive = *directives.borrow();
                match directive {
                    TransferDirective::Running => response_request.await?,
                    TransferDirective::Pause => {
                        sync_output(&mut output).await?;
                        return Ok(PreparedDownloadOutcome::Paused {
                            temporary_path,
                            downloaded_bytes: 0,
                        });
                    }
                    TransferDirective::Cancel => {
                        drop(output);
                        remove_file_if_exists(&temporary_path).await?;
                        return Ok(PreparedDownloadOutcome::Cancelled {
                            downloaded_bytes: 0,
                        });
                    }
                }
            }
            result = &mut response_request => result?,
            _ = tokio::time::sleep(self.stall_timeout) => {
                return Err(HttpDownloadError::Stalled {
                    timeout_ms: duration_millis(self.stall_timeout),
                });
            }
        };
        if !response.status().is_success() {
            return Err(HttpDownloadError::Status(response.status().as_u16()));
        }
        let expected_bytes = probe.total_bytes.or(response.content_length());
        let mut downloaded_bytes = 0_u64;
        loop {
            let directive = *directives.borrow();
            match directive {
                TransferDirective::Running => {}
                TransferDirective::Pause => {
                    sync_output(&mut output).await?;
                    return Ok(PreparedDownloadOutcome::Paused {
                        temporary_path,
                        downloaded_bytes,
                    });
                }
                TransferDirective::Cancel => {
                    drop(output);
                    remove_file_if_exists(&temporary_path).await?;
                    return Ok(PreparedDownloadOutcome::Cancelled { downloaded_bytes });
                }
            }

            tokio::select! {
                biased;
                changed = directives.changed() => {
                    if changed.is_err() {
                        continue;
                    }
                }
                _ = tokio::time::sleep(self.stall_timeout) => {
                    return Err(HttpDownloadError::Stalled {
                        timeout_ms: duration_millis(self.stall_timeout),
                    });
                }
                chunk = response.chunk() => {
                    let Some(chunk) = chunk.map_err(|error| self.client_error(error))? else {
                        break;
                    };
                    let bandwidth = control.acquire_bandwidth(chunk.len() as u64);
                    tokio::pin!(bandwidth);
                    tokio::select! {
                            biased;
                            changed = directives.changed() => {
                                let _ = changed;
                                let directive = *directives.borrow();
                                match directive {
                                TransferDirective::Running => bandwidth.await,
                                TransferDirective::Pause => {
                                    sync_output(&mut output).await?;
                                    return Ok(PreparedDownloadOutcome::Paused {
                                        temporary_path,
                                        downloaded_bytes,
                                    });
                                }
                                TransferDirective::Cancel => {
                                    drop(output);
                                    remove_file_if_exists(&temporary_path).await?;
                                    return Ok(PreparedDownloadOutcome::Cancelled {
                                        downloaded_bytes,
                                    });
                                }
                            }
                        }
                        () = &mut bandwidth => {}
                    }
                    output.write_all(&chunk).await?;
                    downloaded_bytes = downloaded_bytes
                        .checked_add(chunk.len() as u64)
                        .ok_or(HttpDownloadError::LengthOverflow)?;
                    on_progress(DownloadTransferProgress {
                        downloaded_bytes,
                        total_bytes: expected_bytes,
                    });
                }
            }
        }

        if let Some(expected_bytes) = expected_bytes
            && downloaded_bytes != expected_bytes
        {
            return Err(HttpDownloadError::LengthMismatch {
                expected_bytes,
                downloaded_bytes,
            });
        }

        sync_output(&mut output).await?;
        drop(output);
        Ok(PreparedDownloadOutcome::Ready(PreparedDownload {
            temporary_path,
            final_path,
            downloaded_bytes,
        }))
    }

    #[cfg(test)]
    pub(crate) fn with_injected_artifact_fault(mut self, fault: InjectedArtifactFault) -> Self {
        self.injected_artifact_fault = Some(fault);
        self
    }

    pub(crate) async fn commit_prepared(
        &self,
        prepared: PreparedDownload,
    ) -> Result<(), HttpDownloadError> {
        tokio::fs::rename(prepared.temporary_path, prepared.final_path).await?;
        Ok(())
    }

    pub(crate) fn artifact_paths(
        request: &DownloadRequest,
        probe: &DownloadProbe,
        task_id: DownloadTaskId,
    ) -> Result<(PathBuf, PathBuf), HttpDownloadError> {
        let file_name = request
            .file_name
            .as_deref()
            .or(probe.file_name.as_deref())
            .filter(|file_name| is_safe_windows_file_name(file_name))
            .ok_or(HttpDownloadError::MissingFileName)?;
        Ok((
            request
                .save_directory
                .join(format!(".{file_name}.{}.nxtools-part", task_id.as_uuid())),
            request.save_directory.join(file_name),
        ))
    }

    async fn send(
        &self,
        method: Method,
        request: &DownloadRequest,
        range_probe: bool,
    ) -> Result<Response, HttpDownloadError> {
        let mut builder = self.client.request(method, &request.source_url);
        for header in &request.headers {
            let name = HeaderName::from_bytes(header.name.as_bytes())
                .map_err(|_| HttpDownloadError::InvalidHeaderName(header.name.clone()))?;
            let value = HeaderValue::from_str(&header.value)
                .map_err(|_| HttpDownloadError::InvalidHeaderValue(header.name.clone()))?;
            builder = builder.header(name, value);
        }
        if let Some(cookie) = request.cookie.as_deref() {
            builder = builder.header(COOKIE, cookie);
        }
        if let Some(referrer) = request.referrer.as_deref() {
            builder = builder.header(REFERER, referrer);
        }
        if range_probe {
            builder = builder.header(RANGE, PROBE_RANGE);
        }
        builder
            .send()
            .await
            .map_err(|error| self.client_error(error))
    }

    async fn send_range(
        &self,
        request: &DownloadRequest,
        final_url: &str,
        start_byte: u64,
        end_byte: u64,
        validator: &ResourceValidator,
        use_if_range: bool,
    ) -> Result<Response, HttpDownloadError> {
        let mut builder = self.client.get(final_url);
        for header in &request.headers {
            let name = HeaderName::from_bytes(header.name.as_bytes())
                .map_err(|_| HttpDownloadError::InvalidHeaderName(header.name.clone()))?;
            let value = HeaderValue::from_str(&header.value)
                .map_err(|_| HttpDownloadError::InvalidHeaderValue(header.name.clone()))?;
            builder = builder.header(name, value);
        }
        if let Some(cookie) = request.cookie.as_deref() {
            builder = builder.header(COOKIE, cookie);
        }
        if let Some(referrer) = request.referrer.as_deref() {
            builder = builder.header(REFERER, referrer);
        }
        builder = builder
            .header(ACCEPT_ENCODING, "identity")
            .header(RANGE, format!("bytes={start_byte}-{end_byte}"));
        if use_if_range
            && let Some(if_range) =
                strong_etag(validator.etag.as_deref()).or(validator.last_modified.as_deref())
        {
            builder = builder.header(IF_RANGE, if_range);
        }
        builder
            .send()
            .await
            .map_err(|error| self.client_error(error))
    }

    fn client_error(&self, error: reqwest::Error) -> HttpDownloadError {
        if error.is_timeout() {
            HttpDownloadError::Stalled {
                timeout_ms: duration_millis(self.stall_timeout),
            }
        } else {
            HttpDownloadError::Client(error)
        }
    }
}

#[derive(Debug)]
pub(crate) struct PreparedDownload {
    pub temporary_path: PathBuf,
    pub final_path: PathBuf,
    pub downloaded_bytes: u64,
}

#[derive(Debug)]
pub(crate) enum PreparedDownloadOutcome {
    Ready(PreparedDownload),
    Paused {
        temporary_path: PathBuf,
        downloaded_bytes: u64,
    },
    Cancelled {
        downloaded_bytes: u64,
    },
}

async fn sync_output(output: &mut tokio::fs::File) -> Result<(), std::io::Error> {
    output.flush().await?;
    output.sync_all().await
}

async fn remove_file_if_exists(path: &PathBuf) -> Result<(), std::io::Error> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[derive(Debug, Clone)]
pub struct DownloadTransferControl {
    directive: watch::Sender<TransferDirective>,
    global_rate_limiter: DownloadRateLimiter,
    task_rate_limiter: DownloadRateLimiter,
}

#[derive(Debug, Clone)]
pub struct DownloadSegmentTransferControl {
    task: DownloadTransferControl,
    rebalance: watch::Sender<bool>,
}

impl DownloadSegmentTransferControl {
    pub fn new(task: DownloadTransferControl) -> Self {
        let (rebalance, _) = watch::channel(false);
        Self { task, rebalance }
    }

    pub fn request_rebalance(&self) {
        self.rebalance.send_replace(true);
    }

    fn subscribe_task(&self) -> watch::Receiver<TransferDirective> {
        self.task.subscribe()
    }

    fn subscribe_rebalance(&self) -> watch::Receiver<bool> {
        self.rebalance.subscribe()
    }

    async fn acquire_bandwidth(&self, byte_count: u64) {
        self.task.acquire_bandwidth(byte_count).await;
    }
}

impl DownloadTransferControl {
    pub fn new() -> Self {
        Self::with_rate_limiters(
            DownloadRateLimiter::new(None).expect("unlimited rate is valid"),
            DownloadRateLimiter::new(None).expect("unlimited rate is valid"),
        )
    }

    pub(crate) fn with_rate_limiters(
        global_rate_limiter: DownloadRateLimiter,
        task_rate_limiter: DownloadRateLimiter,
    ) -> Self {
        let (directive, _) = watch::channel(TransferDirective::Running);
        Self {
            directive,
            global_rate_limiter,
            task_rate_limiter,
        }
    }

    pub fn pause(&self) {
        self.directive.send_replace(TransferDirective::Pause);
    }

    pub fn cancel(&self) {
        self.directive.send_replace(TransferDirective::Cancel);
    }

    fn subscribe(&self) -> watch::Receiver<TransferDirective> {
        self.directive.subscribe()
    }

    pub(crate) fn set_task_speed_limit(
        &self,
        bytes_per_second: Option<u64>,
    ) -> Result<(), DownloadRateLimitError> {
        self.task_rate_limiter
            .set_bytes_per_second(bytes_per_second)
    }

    async fn acquire_bandwidth(&self, byte_count: u64) {
        tokio::join!(
            self.global_rate_limiter.acquire(byte_count),
            self.task_rate_limiter.acquire(byte_count)
        );
    }

    pub(crate) async fn wait_for_stop_or_timeout(&self, duration: std::time::Duration) -> bool {
        let mut directives = self.subscribe();
        if *directives.borrow() != TransferDirective::Running {
            return true;
        }
        tokio::select! {
            _ = tokio::time::sleep(duration) => false,
            changed = directives.changed() => {
                changed.is_ok() && *directives.borrow() != TransferDirective::Running
            }
        }
    }
}

impl Default for DownloadTransferControl {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransferDirective {
    Running,
    Pause,
    Cancel,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum InjectedArtifactFault {
    PermissionDenied,
    DiskFull,
    FileInUse,
}

#[cfg(test)]
impl InjectedArtifactFault {
    fn into_io_error(self) -> std::io::Error {
        match self {
            Self::PermissionDenied => std::io::Error::from(std::io::ErrorKind::PermissionDenied),
            Self::DiskFull => std::io::Error::from_raw_os_error(112),
            Self::FileInUse => std::io::Error::from_raw_os_error(32),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadTransferProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadSegmentTransferProgress {
    pub segment_index: u32,
    pub downloaded_bytes: u64,
    pub segment_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadSegmentTransferOutcome {
    Completed { downloaded_bytes: u64 },
    Paused { downloaded_bytes: u64 },
    Cancelled { downloaded_bytes: u64 },
    Rebalanced { downloaded_bytes: u64 },
}

impl DownloadSegmentTransferOutcome {
    pub const fn downloaded_bytes(self) -> u64 {
        match self {
            Self::Completed { downloaded_bytes }
            | Self::Paused { downloaded_bytes }
            | Self::Cancelled { downloaded_bytes }
            | Self::Rebalanced { downloaded_bytes } => downloaded_bytes,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Completed { .. } => "completed",
            Self::Paused { .. } => "paused",
            Self::Cancelled { .. } => "cancelled",
            Self::Rebalanced { .. } => "rebalanced",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DownloadTransferContext {
    pub task_id: DownloadTaskId,
    pub correlation_id: uuid::Uuid,
}

#[derive(Debug, Clone, Copy)]
pub struct DownloadSegmentTransferRequest<'a> {
    pub context: DownloadTransferContext,
    pub request: &'a DownloadRequest,
    pub probe: &'a DownloadProbe,
    pub segment: &'a DownloadSegment,
    pub artifact: &'a SegmentedArtifact,
    pub control: &'a DownloadSegmentTransferControl,
    use_if_range: bool,
}

impl<'a> DownloadSegmentTransferRequest<'a> {
    pub const fn new(
        context: DownloadTransferContext,
        request: &'a DownloadRequest,
        probe: &'a DownloadProbe,
        segment: &'a DownloadSegment,
        artifact: &'a SegmentedArtifact,
        control: &'a DownloadSegmentTransferControl,
    ) -> Self {
        Self {
            context,
            request,
            probe,
            segment,
            artifact,
            control,
            use_if_range: false,
        }
    }

    #[must_use]
    pub const fn with_resume_validator(mut self) -> Self {
        self.use_if_range = true;
        self
    }
}

impl DownloadTransferContext {
    pub const fn new(task_id: DownloadTaskId, correlation_id: uuid::Uuid) -> Self {
        Self {
            task_id,
            correlation_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownloadTransferOutcome {
    Completed {
        final_path: PathBuf,
        downloaded_bytes: u64,
    },
    Paused {
        temporary_path: PathBuf,
        downloaded_bytes: u64,
    },
    Cancelled {
        downloaded_bytes: u64,
    },
}

#[derive(Debug, Clone)]
struct ProbeMetadata {
    content_length: Option<u64>,
    content_range_total: Option<u64>,
    content_type: Option<String>,
    content_disposition: Option<String>,
    etag: Option<String>,
    last_modified: Option<String>,
    advertises_ranges: bool,
}

impl ProbeMetadata {
    fn from_response(response: &Response) -> Self {
        let headers = response.headers();
        Self {
            content_length: header_u64(headers.get(CONTENT_LENGTH)),
            content_range_total: headers
                .get(CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                .and_then(content_range_total),
            content_type: header_string(headers.get(CONTENT_TYPE)),
            content_disposition: header_string(headers.get(CONTENT_DISPOSITION)),
            etag: header_string(headers.get(ETAG)),
            last_modified: header_string(headers.get(LAST_MODIFIED)),
            advertises_ranges: headers
                .get(reqwest::header::ACCEPT_RANGES)
                .and_then(|value| value.to_str().ok())
                .is_some_and(|value| value.eq_ignore_ascii_case("bytes")),
        }
    }

    fn merge(self, fallback: Self) -> Self {
        Self {
            content_length: self.content_length.or(fallback.content_length),
            content_range_total: self.content_range_total.or(fallback.content_range_total),
            content_type: self.content_type.or(fallback.content_type),
            content_disposition: self.content_disposition.or(fallback.content_disposition),
            etag: self.etag.or(fallback.etag),
            last_modified: self.last_modified.or(fallback.last_modified),
            advertises_ranges: self.advertises_ranges || fallback.advertises_ranges,
        }
    }

    fn total_bytes(&self, valid_range: bool) -> Option<u64> {
        if valid_range {
            self.content_range_total.or(self.content_length)
        } else {
            self.content_length
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn final_artifact_is_absent_until_the_commit_rename() {
        let directory = tempfile::tempdir().expect("create commit directory");
        let temporary_path = directory.path().join(".fixture.part");
        let final_path = directory.path().join("fixture.bin");
        tokio::fs::write(&temporary_path, b"complete bytes")
            .await
            .expect("write synchronized transfer fixture");
        let output = tokio::fs::OpenOptions::new()
            .write(true)
            .open(&temporary_path)
            .await
            .expect("open temporary artifact");
        output.sync_all().await.expect("sync temporary artifact");

        assert!(temporary_path.exists());
        assert!(!final_path.exists());

        let client = HttpDownloadClient::new().expect("create HTTP client");
        client
            .commit_prepared(PreparedDownload {
                temporary_path: temporary_path.clone(),
                final_path: final_path.clone(),
                downloaded_bytes: 14,
            })
            .await
            .expect("commit prepared artifact");

        assert!(!temporary_path.exists());
        assert_eq!(
            std::fs::read(final_path).expect("read committed artifact"),
            b"complete bytes"
        );
    }

    #[test]
    fn validator_mismatch_is_retried_before_failing_the_task() {
        let error = HttpDownloadError::ResourceValidatorMismatch {
            validator: "etag",
            expected: "\"stable-replica\"".into(),
            actual: Some("\"stale-replica\"".into()),
        };

        assert!(
            error.is_retryable_transfer(),
            "one inconsistent CDN replica must be retried without accepting its bytes"
        );
    }
}

fn valid_probe_range(response: &Response) -> bool {
    if response.status() != StatusCode::PARTIAL_CONTENT {
        return false;
    }
    let Some(value) = response
        .headers()
        .get(CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    parse_content_range(value)
        .is_some_and(|(start, end, total)| start == 0 && end == 0 && total > end)
}

fn validate_segment_response(
    response: &Response,
    expected_start: u64,
    expected_end: u64,
    expected_total: u64,
    expected_validator: &ResourceValidator,
    require_validator_match: bool,
) -> Result<(), HttpDownloadError> {
    if response.status() != StatusCode::PARTIAL_CONTENT {
        return Err(HttpDownloadError::RangeNotHonored {
            status: response.status().as_u16(),
        });
    }
    let raw_content_range = response
        .headers()
        .get(CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .ok_or(HttpDownloadError::MissingContentRange)?;
    let (actual_start, actual_end, actual_total) = parse_content_range(raw_content_range)
        .ok_or_else(|| HttpDownloadError::MalformedContentRange(raw_content_range.to_owned()))?;
    if (actual_start, actual_end, actual_total) != (expected_start, expected_end, expected_total) {
        return Err(HttpDownloadError::ContentRangeMismatch {
            expected_start,
            expected_end,
            expected_total,
            actual_start,
            actual_end,
            actual_total,
        });
    }
    let expected_length = expected_end - expected_start + 1;
    if let Some(actual_length) = response.content_length()
        && actual_length != expected_length
    {
        return Err(HttpDownloadError::ContentLengthMismatch {
            expected_bytes: expected_length,
            actual_bytes: actual_length,
        });
    }
    if require_validator_match {
        validate_response_validator(response, expected_validator)?;
    }
    Ok(())
}

fn validate_response_validator(
    response: &Response,
    expected: &ResourceValidator,
) -> Result<(), HttpDownloadError> {
    if let Some(expected_etag) = expected.etag.as_deref() {
        let actual = header_string(response.headers().get(ETAG));
        if actual.as_deref() != Some(expected_etag) {
            return Err(HttpDownloadError::ResourceValidatorMismatch {
                validator: "etag",
                expected: expected_etag.to_owned(),
                actual,
            });
        }
    } else if let Some(expected_last_modified) = expected.last_modified.as_deref() {
        let actual = header_string(response.headers().get(LAST_MODIFIED));
        if actual.as_deref() != Some(expected_last_modified) {
            return Err(HttpDownloadError::ResourceValidatorMismatch {
                validator: "last_modified",
                expected: expected_last_modified.to_owned(),
                actual,
            });
        }
    }
    Ok(())
}

fn strong_etag(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim_start().starts_with("W/"))
}

fn duration_millis(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn parse_content_range(value: &str) -> Option<(u64, u64, u64)> {
    let value = value.strip_prefix("bytes ")?;
    let (range, total) = value.split_once('/')?;
    let (start, end) = range.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?, total.parse().ok()?))
}

fn content_range_total(value: &str) -> Option<u64> {
    parse_content_range(value).map(|(_, _, total)| total)
}

fn header_u64(value: Option<&HeaderValue>) -> Option<u64> {
    value?.to_str().ok()?.parse().ok()
}

fn header_string(value: Option<&HeaderValue>) -> Option<String> {
    value?.to_str().ok().map(ToOwned::to_owned)
}

fn resolve_file_name(content_disposition: Option<&str>, final_url: &reqwest::Url) -> String {
    let candidate = content_disposition
        .and_then(content_disposition_file_name)
        .or_else(|| {
            final_url
                .path_segments()?
                .next_back()
                .map(ToOwned::to_owned)
        })
        .filter(|name| is_safe_windows_file_name(name));
    candidate.unwrap_or_else(|| "download.bin".to_owned())
}

fn content_disposition_file_name(value: &str) -> Option<String> {
    value.split(';').skip(1).find_map(|parameter| {
        let (name, value) = parameter.trim().split_once('=')?;
        if !name.trim().eq_ignore_ascii_case("filename") {
            return None;
        }
        Some(value.trim().trim_matches('"').to_owned())
    })
}

#[derive(Debug, Error)]
pub enum HttpDownloadError {
    #[error(transparent)]
    InvalidRequest(#[from] DownloadRequestError),
    #[error(transparent)]
    Client(#[from] reqwest::Error),
    #[error("HTTP probe returned status {0}")]
    Status(u16),
    #[error("download request does not contain a safe file name")]
    MissingFileName,
    #[error("download destination already exists: {0}")]
    DestinationExists(PathBuf),
    #[error(
        "insufficient disk space: {available_bytes} bytes available, {required_bytes} bytes required"
    )]
    InsufficientDiskSpace {
        available_bytes: u64,
        required_bytes: u64,
    },
    #[error("download length overflowed u64")]
    LengthOverflow,
    #[error("HTTP stall timeout must be greater than zero")]
    InvalidStallTimeout,
    #[error("HTTP response made no progress for {timeout_ms} ms")]
    Stalled { timeout_ms: u64 },
    #[error(
        "download ended after {downloaded_bytes} bytes, expected exactly {expected_bytes} bytes"
    )]
    LengthMismatch {
        expected_bytes: u64,
        downloaded_bytes: u64,
    },
    #[error("segmented HTTP transfer requires a known resource length")]
    SegmentedTransferNeedsKnownLength,
    #[error("server ignored the requested byte range and returned HTTP {status}")]
    RangeNotHonored { status: u16 },
    #[error("partial response did not include Content-Range")]
    MissingContentRange,
    #[error("partial response contained malformed Content-Range: {0}")]
    MalformedContentRange(String),
    #[error(
        "partial response range mismatch: expected bytes {expected_start}-{expected_end}/{expected_total}, got {actual_start}-{actual_end}/{actual_total}"
    )]
    ContentRangeMismatch {
        expected_start: u64,
        expected_end: u64,
        expected_total: u64,
        actual_start: u64,
        actual_end: u64,
        actual_total: u64,
    },
    #[error(
        "partial response length mismatch: expected {expected_bytes} bytes, got {actual_bytes}"
    )]
    ContentLengthMismatch {
        expected_bytes: u64,
        actual_bytes: u64,
    },
    #[error("partial response {validator} changed: expected {expected}, got {actual:?}")]
    ResourceValidatorMismatch {
        validator: &'static str,
        expected: String,
        actual: Option<String>,
    },
    #[error("invalid HTTP header name: {0}")]
    InvalidHeaderName(String),
    #[error("invalid HTTP header value for {0}")]
    InvalidHeaderValue(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Artifact(#[from] SegmentedArtifactError),
}

impl HttpDownloadError {
    pub(crate) fn allows_clean_single_stream_fallback(&self) -> bool {
        matches!(
            self,
            Self::RangeNotHonored { status } if (200..300).contains(status)
        ) || matches!(
            self,
            Self::MissingContentRange
                | Self::MalformedContentRange(_)
                | Self::ContentRangeMismatch { .. }
                | Self::ContentLengthMismatch { .. }
                | Self::ResourceValidatorMismatch { .. }
        )
    }

    pub(crate) fn is_retryable_transfer(&self) -> bool {
        match self {
            Self::Client(error) => {
                error.is_timeout() || error.is_connect() || error.is_body() || error.is_request()
            }
            Self::Status(status) | Self::RangeNotHonored { status } => {
                matches!(*status, 408 | 425 | 429 | 500..=599)
            }
            Self::LengthMismatch { .. } | Self::ResourceValidatorMismatch { .. } => true,
            Self::Stalled { .. } => true,
            Self::InvalidRequest(_)
            | Self::MissingFileName
            | Self::DestinationExists(_)
            | Self::InsufficientDiskSpace { .. }
            | Self::LengthOverflow
            | Self::InvalidStallTimeout
            | Self::SegmentedTransferNeedsKnownLength
            | Self::MissingContentRange
            | Self::MalformedContentRange(_)
            | Self::ContentRangeMismatch { .. }
            | Self::ContentLengthMismatch { .. }
            | Self::InvalidHeaderName(_)
            | Self::InvalidHeaderValue(_)
            | Self::Io(_)
            | Self::Artifact(_) => false,
        }
    }

    pub(crate) fn kind(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_) => "invalid_request",
            Self::Client(_) => "client",
            Self::Status(_) => "status",
            Self::MissingFileName => "missing_file_name",
            Self::DestinationExists(_) => "destination_exists",
            Self::InsufficientDiskSpace { .. } => "disk_full",
            Self::LengthOverflow => "length_overflow",
            Self::InvalidStallTimeout => "invalid_stall_timeout",
            Self::Stalled { .. } => "stalled",
            Self::LengthMismatch { .. } => "length_mismatch",
            Self::SegmentedTransferNeedsKnownLength => "unknown_length",
            Self::RangeNotHonored { .. } => "range_not_honored",
            Self::MissingContentRange => "missing_content_range",
            Self::MalformedContentRange(_) => "malformed_content_range",
            Self::ContentRangeMismatch { .. } => "content_range_mismatch",
            Self::ContentLengthMismatch { .. } => "content_length_mismatch",
            Self::ResourceValidatorMismatch { .. } => "resource_validator_mismatch",
            Self::InvalidHeaderName(_) => "invalid_header_name",
            Self::InvalidHeaderValue(_) => "invalid_header_value",
            Self::Io(_) | Self::Artifact(_) => {
                self.io_error().map(storage_io_kind).unwrap_or("artifact")
            }
        }
    }

    pub(crate) fn io_error(&self) -> Option<&std::io::Error> {
        match self {
            Self::Io(error) | Self::Artifact(SegmentedArtifactError::Io(error)) => Some(error),
            _ => None,
        }
    }
}

fn storage_io_kind(error: &std::io::Error) -> &'static str {
    if error.kind() == std::io::ErrorKind::PermissionDenied {
        return "permission";
    }
    if is_disk_full_error(error) {
        return "disk_full";
    }
    if is_file_in_use_error(error) {
        return "file_in_use";
    }
    "io"
}

pub(crate) fn is_disk_full_error(error: &std::io::Error) -> bool {
    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(39 | 112))
    }
    #[cfg(not(windows))]
    {
        error.raw_os_error() == Some(28)
    }
}

pub(crate) fn is_file_in_use_error(error: &std::io::Error) -> bool {
    #[cfg(windows)]
    {
        matches!(error.raw_os_error(), Some(32 | 33))
    }
    #[cfg(not(windows))]
    {
        false
    }
}
