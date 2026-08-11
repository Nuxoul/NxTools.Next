use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use aes::Aes128;
use cbc::cipher::{BlockDecryptMut, KeyIvInit, generic_array::GenericArray};
use reqwest::header::{
    ACCEPT_ENCODING, CONTENT_DISPOSITION, CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, COOKIE,
    ETAG, HeaderName, HeaderValue, IF_RANGE, LAST_MODIFIED, RANGE, REFERER,
};
use reqwest::{Client, Method, Response, StatusCode};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::watch;
use tracing::{info, warn};

use crate::dash::{
    DashError, MAX_DASH_MANIFEST_BYTES, MAX_DASH_URL_BYTES, parse_dash_manifest,
    validate_dash_probe_size,
};
use crate::hls::{
    HlsMediaPlaylist, HlsProbeError, MAX_HLS_MANIFEST_BYTES, MAX_HLS_VARIANT_URL_BYTES,
    parse_hls_media_playlist, parse_hls_probe, validate_hls_probe_size,
};
use crate::{
    DownloadMediaProbe, DownloadProbe, DownloadRateLimitError, DownloadRateLimiter,
    DownloadRequest, DownloadRequestError, DownloadSegment, DownloadTaskId, ResourceValidator,
    SegmentedArtifact, SegmentedArtifactError, is_safe_windows_file_name,
};
use crate::{NetworkOperationSnapshot, NetworkPolicy, NetworkPolicyError};
use nxtools_core::{NetworkProxyMode, NetworkSettings};

const PROBE_RANGE: &str = "bytes=0-0";
const DEFAULT_STALL_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone)]
pub struct HttpDownloadClient {
    client: Client,
    stall_timeout: Duration,
    network_proxy_mode: NetworkProxyMode,
    network_proxy_source: &'static str,
    #[cfg(test)]
    injected_artifact_fault: Option<InjectedArtifactFault>,
}

impl HttpDownloadClient {
    pub fn new() -> Result<Self, HttpDownloadError> {
        Self::with_stall_timeout(DEFAULT_STALL_TIMEOUT)
    }

    pub fn with_stall_timeout(stall_timeout: Duration) -> Result<Self, HttpDownloadError> {
        let policy = NetworkPolicy::new(NetworkSettings::default())?;
        let snapshot = policy.snapshot()?;
        Self::with_stall_timeout_and_network(stall_timeout, &snapshot)
    }

    pub fn with_stall_timeout_and_network(
        stall_timeout: Duration,
        network: &NetworkOperationSnapshot,
    ) -> Result<Self, HttpDownloadError> {
        if stall_timeout.is_zero() {
            return Err(HttpDownloadError::InvalidStallTimeout);
        }
        let client = network.build_client(
            Client::builder()
                .user_agent(concat!("NxTools.Next/", env!("CARGO_PKG_VERSION")))
                .redirect(reqwest::redirect::Policy::limited(10))
                .read_timeout(stall_timeout)
                .connect_timeout(stall_timeout),
        )?;
        Ok(Self {
            client,
            stall_timeout,
            network_proxy_mode: network.mode(),
            network_proxy_source: network.source(),
            #[cfg(test)]
            injected_artifact_fault: None,
        })
    }

    pub const fn network_proxy_mode(&self) -> NetworkProxyMode {
        self.network_proxy_mode
    }

    pub const fn network_proxy_source(&self) -> &'static str {
        self.network_proxy_source
    }

    pub(crate) fn for_network_snapshot(
        &self,
        network: &NetworkOperationSnapshot,
    ) -> Result<Self, HttpDownloadError> {
        #[allow(unused_mut)]
        let mut rebuilt = Self::with_stall_timeout_and_network(self.stall_timeout, network)?;
        #[cfg(test)]
        {
            rebuilt.injected_artifact_fault = self.injected_artifact_fault;
        }
        Ok(rebuilt)
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
        let probed_url = response.url().clone();
        let (final_url, media) = if is_hls_candidate(&probed_url, metadata.content_type.as_deref())
        {
            let (manifest_url, hls) = self.probe_hls_manifest(request).await?;
            (manifest_url, Some(DownloadMediaProbe::Hls(hls)))
        } else if is_dash_candidate(&probed_url, metadata.content_type.as_deref()) {
            let (manifest_url, dash) = self.probe_dash_manifest(request).await?;
            (manifest_url, Some(DownloadMediaProbe::Dash(dash)))
        } else {
            (probed_url, None)
        };

        let probe = DownloadProbe {
            final_url: final_url.to_string(),
            file_name: Some(resolve_file_name(
                metadata.content_disposition.as_deref(),
                &final_url,
            )),
            total_bytes: metadata.total_bytes(valid_range),
            supports_ranges: valid_range,
            content_type: metadata.content_type,
            validator: ResourceValidator {
                etag: metadata.etag,
                last_modified: metadata.last_modified,
            },
            media,
        };
        match probe.media.as_ref() {
            Some(DownloadMediaProbe::Hls(_)) => {
                validate_hls_probe_size(&probe).map_err(map_hls_probe_error)?;
            }
            Some(DownloadMediaProbe::Dash(_)) => {
                validate_dash_probe_size(&probe).map_err(map_dash_error)?;
            }
            None => {}
        }
        Ok(probe)
    }

    async fn probe_dash_manifest(
        &self,
        request: &DownloadRequest,
    ) -> Result<(url::Url, crate::DashProbe), HttpDownloadError> {
        let mut response = self.send(Method::GET, request, false).await?;
        if !response.status().is_success() {
            return Err(HttpDownloadError::Status(response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_DASH_MANIFEST_BYTES as u64)
        {
            return Err(HttpDownloadError::DashManifestTooLarge);
        }
        let final_url = response.url().clone();
        if final_url.as_str().len() > MAX_DASH_URL_BYTES {
            return Err(HttpDownloadError::DashMetadataTooLarge);
        }
        let mut manifest = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(4096)
                .min(MAX_DASH_MANIFEST_BYTES as u64) as usize,
        );
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| self.client_error(error))?
        {
            if manifest.len().saturating_add(chunk.len()) > MAX_DASH_MANIFEST_BYTES {
                return Err(HttpDownloadError::DashManifestTooLarge);
            }
            manifest.extend_from_slice(&chunk);
        }
        let dash = parse_dash_manifest(&final_url, &manifest)
            .map_err(map_dash_error)?
            .probe();
        Ok((final_url, dash))
    }

    pub(crate) async fn load_dash_manifest(
        &self,
        request: &DownloadRequest,
        manifest_url: &str,
    ) -> Result<crate::dash::DashManifest, HttpDownloadError> {
        let mut manifest_request = request.clone();
        manifest_request.source_url = manifest_url.to_owned();
        manifest_request.media = None;
        manifest_request.validate()?;
        let mut response = self.send(Method::GET, &manifest_request, false).await?;
        if !response.status().is_success() {
            return Err(HttpDownloadError::Status(response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_DASH_MANIFEST_BYTES as u64)
        {
            return Err(HttpDownloadError::DashManifestTooLarge);
        }
        let final_url = response.url().clone();
        if final_url.as_str().len() > MAX_DASH_URL_BYTES {
            return Err(HttpDownloadError::DashMetadataTooLarge);
        }
        let mut manifest = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(4096)
                .min(MAX_DASH_MANIFEST_BYTES as u64) as usize,
        );
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| self.client_error(error))?
        {
            if manifest.len().saturating_add(chunk.len()) > MAX_DASH_MANIFEST_BYTES {
                return Err(HttpDownloadError::DashManifestTooLarge);
            }
            manifest.extend_from_slice(&chunk);
        }
        parse_dash_manifest(&final_url, &manifest).map_err(map_dash_error)
    }

    async fn probe_hls_manifest(
        &self,
        request: &DownloadRequest,
    ) -> Result<(url::Url, crate::HlsProbe), HttpDownloadError> {
        let mut response = self.send(Method::GET, request, false).await?;
        if !response.status().is_success() {
            return Err(HttpDownloadError::Status(response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_HLS_MANIFEST_BYTES as u64)
        {
            return Err(HttpDownloadError::HlsManifestTooLarge);
        }
        let final_url = response.url().clone();
        if final_url.as_str().len() > MAX_HLS_VARIANT_URL_BYTES {
            return Err(HttpDownloadError::HlsMetadataTooLarge);
        }
        let mut manifest = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(4096)
                .min(MAX_HLS_MANIFEST_BYTES as u64) as usize,
        );
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| self.client_error(error))?
        {
            if manifest.len().saturating_add(chunk.len()) > MAX_HLS_MANIFEST_BYTES {
                return Err(HttpDownloadError::HlsManifestTooLarge);
            }
            manifest.extend_from_slice(&chunk);
        }
        let hls = parse_hls_probe(&final_url, &manifest).map_err(map_hls_probe_error)?;
        Ok((final_url, hls))
    }

    pub(crate) async fn load_hls_media_playlist(
        &self,
        request: &DownloadRequest,
        playlist_url: &str,
    ) -> Result<HlsMediaPlaylist, HttpDownloadError> {
        let mut playlist_request = request.clone();
        playlist_request.source_url = playlist_url.to_owned();
        playlist_request.media = None;
        playlist_request.validate()?;
        let mut response = self.send(Method::GET, &playlist_request, false).await?;
        if !response.status().is_success() {
            return Err(HttpDownloadError::Status(response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_HLS_MANIFEST_BYTES as u64)
        {
            return Err(HttpDownloadError::HlsManifestTooLarge);
        }
        let final_url = response.url().clone();
        if final_url.as_str().len() > MAX_HLS_VARIANT_URL_BYTES {
            return Err(HttpDownloadError::HlsMetadataTooLarge);
        }
        let mut manifest = Vec::with_capacity(
            response
                .content_length()
                .unwrap_or(4096)
                .min(MAX_HLS_MANIFEST_BYTES as u64) as usize,
        );
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|error| self.client_error(error))?
        {
            if manifest.len().saturating_add(chunk.len()) > MAX_HLS_MANIFEST_BYTES {
                return Err(HttpDownloadError::HlsManifestTooLarge);
            }
            manifest.extend_from_slice(&chunk);
        }
        parse_hls_media_playlist(&final_url, &manifest).map_err(map_hls_probe_error)
    }

    pub(crate) async fn download_hls_segment<F>(
        &self,
        request: &DownloadRequest,
        segment_url: &str,
        staging_path: &PathBuf,
        control: &DownloadTransferControl,
        mut on_progress: F,
    ) -> Result<HlsSegmentDownloadOutcome, HttpDownloadError>
    where
        F: FnMut(u64),
    {
        let mut segment_request = request.clone();
        segment_request.source_url = segment_url.to_owned();
        segment_request.media = None;
        segment_request.validate()?;
        remove_file_if_exists(staging_path).await?;
        let mut directives = control.subscribe();
        match *directives.borrow() {
            TransferDirective::Running => {}
            TransferDirective::Pause => return Ok(HlsSegmentDownloadOutcome::Paused),
            TransferDirective::Cancel => return Ok(HlsSegmentDownloadOutcome::Cancelled),
        }
        let response_request = self.send(Method::GET, &segment_request, false);
        tokio::pin!(response_request);
        let mut response = tokio::select! {
            biased;
            changed = directives.changed() => {
                let _ = changed;
                return Ok(match *directives.borrow() {
                    TransferDirective::Cancel => HlsSegmentDownloadOutcome::Cancelled,
                    TransferDirective::Pause | TransferDirective::Running => HlsSegmentDownloadOutcome::Paused,
                });
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
        let mut output = tokio::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(staging_path)
            .await?;
        let mut downloaded_bytes = 0_u64;
        loop {
            let next_chunk = response.chunk();
            tokio::pin!(next_chunk);
            let chunk = tokio::select! {
                biased;
                changed = directives.changed() => {
                    let _ = changed;
                    drop(output);
                    remove_file_if_exists(staging_path).await?;
                    return Ok(match *directives.borrow() {
                        TransferDirective::Cancel => HlsSegmentDownloadOutcome::Cancelled,
                        TransferDirective::Pause | TransferDirective::Running => HlsSegmentDownloadOutcome::Paused,
                    });
                }
                result = &mut next_chunk => result.map_err(|error| self.client_error(error))?,
                _ = tokio::time::sleep(self.stall_timeout) => {
                    drop(output);
                    remove_file_if_exists(staging_path).await?;
                    return Err(HttpDownloadError::Stalled {
                        timeout_ms: duration_millis(self.stall_timeout),
                    });
                }
            };
            let Some(chunk) = chunk else {
                break;
            };
            let chunk_len =
                u64::try_from(chunk.len()).map_err(|_| HttpDownloadError::LengthOverflow)?;
            let bandwidth = control.acquire_bandwidth(chunk_len);
            tokio::pin!(bandwidth);
            tokio::select! {
                biased;
                changed = directives.changed() => {
                    let _ = changed;
                    drop(output);
                    remove_file_if_exists(staging_path).await?;
                    return Ok(match *directives.borrow() {
                        TransferDirective::Cancel => HlsSegmentDownloadOutcome::Cancelled,
                        TransferDirective::Pause | TransferDirective::Running => {
                            HlsSegmentDownloadOutcome::Paused
                        }
                    });
                }
                () = &mut bandwidth => {}
            }
            output.write_all(&chunk).await?;
            downloaded_bytes = downloaded_bytes
                .checked_add(chunk_len)
                .ok_or(HttpDownloadError::LengthOverflow)?;
            on_progress(downloaded_bytes);
        }
        if downloaded_bytes == 0 {
            drop(output);
            remove_file_if_exists(staging_path).await?;
            return Err(HttpDownloadError::EmptyHlsSegment);
        }
        sync_output(&mut output).await?;
        Ok(HlsSegmentDownloadOutcome::Completed { downloaded_bytes })
    }

    pub(crate) async fn fetch_hls_aes128_key(
        &self,
        request: &DownloadRequest,
        key_url: &str,
        control: &DownloadTransferControl,
    ) -> Result<HlsKeyFetchOutcome, HttpDownloadError> {
        let mut key_request = request.clone();
        key_request.source_url = key_url.to_owned();
        key_request.media = None;
        key_request.validate()?;
        let mut directives = control.subscribe();
        match *directives.borrow() {
            TransferDirective::Running => {}
            TransferDirective::Pause => return Ok(HlsKeyFetchOutcome::Paused),
            TransferDirective::Cancel => return Ok(HlsKeyFetchOutcome::Cancelled),
        }
        let response_request = self.send(Method::GET, &key_request, false);
        tokio::pin!(response_request);
        let mut response = tokio::select! {
            biased;
            changed = directives.changed() => {
                let _ = changed;
                return Ok(match *directives.borrow() {
                    TransferDirective::Cancel => HlsKeyFetchOutcome::Cancelled,
                    TransferDirective::Pause | TransferDirective::Running => HlsKeyFetchOutcome::Paused,
                });
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
        if response.content_length().is_some_and(|length| length != 16) {
            return Err(HttpDownloadError::InvalidHlsKeyLength {
                actual_bytes: response.content_length().unwrap_or_default(),
            });
        }
        let mut key = Vec::with_capacity(16);
        loop {
            let next_chunk = response.chunk();
            tokio::pin!(next_chunk);
            let chunk = tokio::select! {
                biased;
                changed = directives.changed() => {
                    let _ = changed;
                    return Ok(match *directives.borrow() {
                        TransferDirective::Cancel => HlsKeyFetchOutcome::Cancelled,
                        TransferDirective::Pause | TransferDirective::Running => HlsKeyFetchOutcome::Paused,
                    });
                }
                result = &mut next_chunk => result.map_err(|error| self.client_error(error))?,
                _ = tokio::time::sleep(self.stall_timeout) => {
                    return Err(HttpDownloadError::Stalled {
                        timeout_ms: duration_millis(self.stall_timeout),
                    });
                }
            };
            let Some(chunk) = chunk else {
                break;
            };
            if key.len().saturating_add(chunk.len()) > 16 {
                return Err(HttpDownloadError::InvalidHlsKeyLength {
                    actual_bytes: u64::try_from(key.len().saturating_add(chunk.len()))
                        .unwrap_or(u64::MAX),
                });
            }
            let chunk_len =
                u64::try_from(chunk.len()).map_err(|_| HttpDownloadError::LengthOverflow)?;
            let bandwidth = control.acquire_bandwidth(chunk_len);
            tokio::pin!(bandwidth);
            tokio::select! {
                biased;
                changed = directives.changed() => {
                    let _ = changed;
                    return Ok(match *directives.borrow() {
                        TransferDirective::Cancel => HlsKeyFetchOutcome::Cancelled,
                        TransferDirective::Pause | TransferDirective::Running => HlsKeyFetchOutcome::Paused,
                    });
                }
                () = &mut bandwidth => {}
            }
            key.extend_from_slice(&chunk);
        }
        let actual_bytes = u64::try_from(key.len()).unwrap_or(u64::MAX);
        let key = <[u8; 16]>::try_from(key)
            .map_err(|_| HttpDownloadError::InvalidHlsKeyLength { actual_bytes })?;
        Ok(HlsKeyFetchOutcome::Loaded(key))
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
        let mut directives = control.subscribe();
        let initial_directive = *directives.borrow_and_update();
        request.validate()?;
        let (temporary_path, final_path) = Self::artifact_paths(request, probe, task_id)?;
        match initial_directive {
            TransferDirective::Running => {}
            TransferDirective::Pause => {
                return Ok(PreparedDownloadOutcome::Paused {
                    temporary_path,
                    downloaded_bytes: 0,
                });
            }
            TransferDirective::Cancel => {
                remove_file_if_exists(&temporary_path).await?;
                return Ok(PreparedDownloadOutcome::Cancelled {
                    downloaded_bytes: 0,
                });
            }
        }
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

        let prepared_directive = *directives.borrow_and_update();
        match prepared_directive {
            TransferDirective::Running => {}
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HlsSegmentDownloadOutcome {
    Completed { downloaded_bytes: u64 },
    Paused,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HlsKeyFetchOutcome {
    Loaded([u8; 16]),
    Paused,
    Cancelled,
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

pub(crate) const fn hls_sequence_iv(sequence: u64) -> [u8; 16] {
    let mut iv = [0_u8; 16];
    let sequence = sequence.to_be_bytes();
    let mut index = 0;
    while index < sequence.len() {
        iv[8 + index] = sequence[index];
        index += 1;
    }
    iv
}

pub(crate) async fn decrypt_hls_aes128_segment(
    ciphertext_path: &Path,
    plaintext_path: &Path,
    key: [u8; 16],
    iv: [u8; 16],
    control: &DownloadTransferControl,
) -> Result<HlsSegmentDownloadOutcome, HttpDownloadError> {
    remove_file_if_exists(&plaintext_path.to_path_buf()).await?;
    let ciphertext_path = ciphertext_path.to_path_buf();
    let plaintext_path = plaintext_path.to_path_buf();
    let cleanup_plaintext_path = plaintext_path.clone();
    let control = control.clone();
    let result = tokio::task::spawn_blocking(move || {
        use std::io::{Read, Write};

        let ciphertext_bytes = std::fs::metadata(&ciphertext_path)?.len();
        if ciphertext_bytes == 0 || ciphertext_bytes % 16 != 0 {
            return Err(HttpDownloadError::InvalidHlsCiphertextLength {
                actual_bytes: ciphertext_bytes,
            });
        }
        let mut input = std::fs::File::open(&ciphertext_path)?;
        let mut output = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&plaintext_path)?;
        let mut decryptor = cbc::Decryptor::<Aes128>::new((&key).into(), (&iv).into());
        let block_count = ciphertext_bytes / 16;
        let mut plaintext_bytes = 0_u64;
        for block_index in 0..block_count {
            match control.current_directive() {
                TransferDirective::Running => {}
                directive => {
                    drop(output);
                    std::fs::remove_file(&plaintext_path)?;
                    return Ok(match directive {
                        TransferDirective::Cancel => HlsSegmentDownloadOutcome::Cancelled,
                        TransferDirective::Pause | TransferDirective::Running => {
                            HlsSegmentDownloadOutcome::Paused
                        }
                    });
                }
            }
            let mut raw_block = [0_u8; 16];
            input.read_exact(&mut raw_block)?;
            let mut block = GenericArray::clone_from_slice(&raw_block);
            decryptor.decrypt_block_mut(&mut block);
            if block_index + 1 == block_count {
                let padding = usize::from(block[15]);
                if padding == 0
                    || padding > 16
                    || !block[16 - padding..]
                        .iter()
                        .all(|byte| usize::from(*byte) == padding)
                {
                    return Err(HttpDownloadError::InvalidHlsPadding);
                }
                output.write_all(&block[..16 - padding])?;
                plaintext_bytes = plaintext_bytes
                    .checked_add(u64::try_from(16 - padding).unwrap_or_default())
                    .ok_or(HttpDownloadError::LengthOverflow)?;
            } else {
                output.write_all(&block)?;
                plaintext_bytes = plaintext_bytes
                    .checked_add(16)
                    .ok_or(HttpDownloadError::LengthOverflow)?;
            }
        }
        if plaintext_bytes == 0 {
            return Err(HttpDownloadError::EmptyHlsSegment);
        }
        output.flush()?;
        output.sync_all()?;
        Ok(HlsSegmentDownloadOutcome::Completed {
            downloaded_bytes: plaintext_bytes,
        })
    })
    .await
    .map_err(|error| HttpDownloadError::HlsDecryptionWorker(error.to_string()));
    let result = match result {
        Ok(result) => result,
        Err(error) => Err(error),
    };
    if result.is_err() {
        remove_file_if_exists(&cleanup_plaintext_path).await?;
    }
    result
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

    pub(crate) fn subscribe(&self) -> watch::Receiver<TransferDirective> {
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

    pub(crate) async fn wait_for_stop_or_timeout(
        &self,
        duration: std::time::Duration,
    ) -> Option<TransferDirective> {
        let mut directives = self.subscribe();
        let current = *directives.borrow();
        if current != TransferDirective::Running {
            return Some(current);
        }
        tokio::select! {
            _ = tokio::time::sleep(duration) => None,
            changed = directives.changed() => {
                if changed.is_ok() {
                    let directive = *directives.borrow();
                    (directive != TransferDirective::Running).then_some(directive)
                } else {
                    None
                }
            }
        }
    }

    pub(crate) fn current_directive(&self) -> TransferDirective {
        *self.directive.borrow()
    }
}

impl Default for DownloadTransferControl {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferDirective {
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

    #[tokio::test]
    async fn aes128_cbc_segment_uses_sequence_iv_and_removes_pkcs7_padding() {
        let directory = tempfile::tempdir().expect("create AES fixture directory");
        let ciphertext_path = directory.path().join("segment.encrypted.part");
        let plaintext_path = directory.path().join("segment.part");
        tokio::fs::write(
            &ciphertext_path,
            [
                0x66, 0xde, 0x9a, 0x0c, 0x54, 0x6f, 0xfc, 0xb6, 0x3e, 0x5e, 0x74, 0x80, 0x80, 0x1e,
                0x42, 0x98,
            ],
        )
        .await
        .expect("write independent ciphertext fixture");
        let key = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];

        let outcome = decrypt_hls_aes128_segment(
            &ciphertext_path,
            &plaintext_path,
            key,
            hls_sequence_iv(7_794),
            &DownloadTransferControl::new(),
        )
        .await
        .expect("decrypt AES-128 segment");

        assert_eq!(
            outcome,
            HlsSegmentDownloadOutcome::Completed {
                downloaded_bytes: 14
            }
        );
        assert_eq!(
            tokio::fs::read(&plaintext_path)
                .await
                .expect("read decrypted segment"),
            b"encrypted-one|"
        );
    }

    #[tokio::test]
    async fn aes128_cbc_rejects_an_all_padding_segment_and_cleans_plaintext() {
        let directory = tempfile::tempdir().expect("create empty AES fixture directory");
        let ciphertext_path = directory.path().join("empty.encrypted.part");
        let plaintext_path = directory.path().join("empty.part");
        tokio::fs::write(
            &ciphertext_path,
            [
                0xed, 0x75, 0x55, 0xc6, 0x22, 0x21, 0xcc, 0x68, 0xd2, 0x7e, 0xaa, 0xee, 0x41, 0x3e,
                0xbc, 0xca,
            ],
        )
        .await
        .expect("write independently encrypted all-padding fixture");

        let error = decrypt_hls_aes128_segment(
            &ciphertext_path,
            &plaintext_path,
            [
                0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
                0x0e, 0x0f,
            ],
            hls_sequence_iv(7_794),
            &DownloadTransferControl::new(),
        )
        .await
        .expect_err("reject an all-padding media segment");

        assert!(matches!(error, HttpDownloadError::EmptyHlsSegment));
        assert!(!plaintext_path.exists());
    }

    #[tokio::test]
    async fn aes128_cbc_observes_pause_during_local_decryption_and_cleans_plaintext() {
        let directory = tempfile::tempdir().expect("create paused AES fixture directory");
        let ciphertext_path = directory.path().join("paused.encrypted.part");
        let plaintext_path = directory.path().join("paused.part");
        tokio::fs::write(&ciphertext_path, vec![0_u8; 2 * 1024 * 1024])
            .await
            .expect("write paused ciphertext fixture");
        let control = DownloadTransferControl::new();
        let worker_control = control.clone();
        let worker_ciphertext = ciphertext_path.clone();
        let worker_plaintext = plaintext_path.clone();
        let decrypt = tokio::spawn(async move {
            decrypt_hls_aes128_segment(
                &worker_ciphertext,
                &worker_plaintext,
                [0_u8; 16],
                [0_u8; 16],
                &worker_control,
            )
            .await
        });
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        loop {
            if tokio::fs::metadata(&plaintext_path)
                .await
                .is_ok_and(|metadata| metadata.len() > 0)
            {
                break;
            }
            assert!(
                std::time::Instant::now() < deadline && !decrypt.is_finished(),
                "local AES decryption did not enter its real file-write boundary"
            );
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        control.pause();
        let outcome = decrypt
            .await
            .expect("join local AES decrypt worker")
            .expect("pause local AES decryption");

        assert_eq!(outcome, HlsSegmentDownloadOutcome::Paused);
        assert!(!plaintext_path.exists());
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

fn is_hls_candidate(url: &url::Url, content_type: Option<&str>) -> bool {
    let path_is_hls = url.path().to_ascii_lowercase().ends_with(".m3u8");
    let content_type_is_hls = content_type
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .is_some_and(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "application/vnd.apple.mpegurl"
                    | "application/x-mpegurl"
                    | "application/mpegurl"
                    | "audio/mpegurl"
                    | "audio/x-mpegurl"
            )
        });
    path_is_hls || content_type_is_hls
}

fn is_dash_candidate(url: &url::Url, content_type: Option<&str>) -> bool {
    let path_is_dash = url.path().to_ascii_lowercase().ends_with(".mpd");
    let content_type_is_dash = content_type
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .is_some_and(|value| value.eq_ignore_ascii_case("application/dash+xml"));
    path_is_dash || content_type_is_dash
}

fn map_hls_probe_error(error: HlsProbeError) -> HttpDownloadError {
    match error {
        HlsProbeError::InvalidManifest(reason) => HttpDownloadError::InvalidHlsManifest(reason),
        HlsProbeError::TooManyVariants => HttpDownloadError::TooManyHlsVariants,
        HlsProbeError::MetadataTooLarge => HttpDownloadError::HlsMetadataTooLarge,
        HlsProbeError::TooManyMediaSegments => HttpDownloadError::TooManyHlsMediaSegments,
        HlsProbeError::UnsupportedFeature(feature) => {
            HttpDownloadError::UnsupportedHlsFeature(feature)
        }
    }
}

fn map_dash_error(error: DashError) -> HttpDownloadError {
    match error {
        DashError::Invalid(reason) => HttpDownloadError::InvalidDashManifest(reason),
        DashError::Unsupported(feature) => HttpDownloadError::UnsupportedDashFeature(feature),
        DashError::TooManyRepresentations => HttpDownloadError::TooManyDashRepresentations,
        DashError::TooManySegments => HttpDownloadError::TooManyDashSegments,
        DashError::MetadataTooLarge => HttpDownloadError::DashMetadataTooLarge,
    }
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
    #[error(transparent)]
    NetworkPolicy(#[from] NetworkPolicyError),
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
    #[error("HLS manifest exceeds the 2 MiB probe limit")]
    HlsManifestTooLarge,
    #[error("invalid HLS manifest: {0}")]
    InvalidHlsManifest(&'static str),
    #[error("HLS manifest contains more than 32 variants")]
    TooManyHlsVariants,
    #[error("HLS presentation metadata exceeds its bounded field limits")]
    HlsMetadataTooLarge,
    #[error("HLS media playlist contains too many segments")]
    TooManyHlsMediaSegments,
    #[error("HLS feature is not supported in this transfer slice: {0}")]
    UnsupportedHlsFeature(&'static str),
    #[error("HLS media segment is empty")]
    EmptyHlsSegment,
    #[error("HLS AES-128 key has {actual_bytes} bytes instead of exactly 16")]
    InvalidHlsKeyLength { actual_bytes: u64 },
    #[error("HLS AES-128 ciphertext length {actual_bytes} is not a positive block multiple")]
    InvalidHlsCiphertextLength { actual_bytes: u64 },
    #[error("HLS AES-128 segment has invalid PKCS#7 padding")]
    InvalidHlsPadding,
    #[error("HLS AES-128 decryption worker failed: {0}")]
    HlsDecryptionWorker(String),
    #[error("DASH MPD exceeds the 2 MiB probe limit")]
    DashManifestTooLarge,
    #[error("invalid DASH MPD: {0}")]
    InvalidDashManifest(&'static str),
    #[error("DASH MPD contains more than 32 Representations")]
    TooManyDashRepresentations,
    #[error("DASH MPD expands to too many Segments")]
    TooManyDashSegments,
    #[error("DASH presentation metadata exceeds its bounded limits")]
    DashMetadataTooLarge,
    #[error("DASH feature is not supported in this transfer slice: {0}")]
    UnsupportedDashFeature(&'static str),
    #[error("HLS media playlist changed since the durable transfer plan was created")]
    HlsResourceChanged,
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
    pub(crate) fn allows_fresh_segmented_restart_after_resume(&self) -> bool {
        matches!(self, Self::RangeNotHonored { status: 200 })
    }

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
            Self::NetworkPolicy(_) => false,
            Self::Status(status) | Self::RangeNotHonored { status } => {
                matches!(*status, 408 | 425 | 429 | 500..=599)
            }
            Self::LengthMismatch { .. } | Self::ResourceValidatorMismatch { .. } => true,
            Self::InvalidHlsKeyLength { .. }
            | Self::InvalidHlsCiphertextLength { .. }
            | Self::InvalidHlsPadding => true,
            Self::Stalled { .. } => true,
            Self::InvalidRequest(_)
            | Self::MissingFileName
            | Self::DestinationExists(_)
            | Self::InsufficientDiskSpace { .. }
            | Self::LengthOverflow
            | Self::InvalidStallTimeout
            | Self::HlsManifestTooLarge
            | Self::InvalidHlsManifest(_)
            | Self::TooManyHlsVariants
            | Self::HlsMetadataTooLarge
            | Self::TooManyHlsMediaSegments
            | Self::UnsupportedHlsFeature(_)
            | Self::EmptyHlsSegment
            | Self::HlsDecryptionWorker(_)
            | Self::HlsResourceChanged
            | Self::DashManifestTooLarge
            | Self::InvalidDashManifest(_)
            | Self::TooManyDashRepresentations
            | Self::TooManyDashSegments
            | Self::DashMetadataTooLarge
            | Self::UnsupportedDashFeature(_)
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
            Self::NetworkPolicy(_) => "network_policy",
            Self::Status(_) => "status",
            Self::MissingFileName => "missing_file_name",
            Self::DestinationExists(_) => "destination_exists",
            Self::InsufficientDiskSpace { .. } => "disk_full",
            Self::LengthOverflow => "length_overflow",
            Self::InvalidStallTimeout => "invalid_stall_timeout",
            Self::Stalled { .. } => "stalled",
            Self::HlsManifestTooLarge => "hls_manifest_too_large",
            Self::InvalidHlsManifest(_) => "invalid_hls_manifest",
            Self::TooManyHlsVariants => "too_many_hls_variants",
            Self::HlsMetadataTooLarge => "hls_metadata_too_large",
            Self::TooManyHlsMediaSegments => "too_many_hls_media_segments",
            Self::UnsupportedHlsFeature(_) => "unsupported_hls_feature",
            Self::EmptyHlsSegment => "empty_hls_segment",
            Self::InvalidHlsKeyLength { .. } => "hls_key_invalid_length",
            Self::InvalidHlsCiphertextLength { .. } => "hls_ciphertext_invalid_length",
            Self::InvalidHlsPadding => "hls_padding_invalid",
            Self::HlsDecryptionWorker(_) => "hls_decryption_worker",
            Self::HlsResourceChanged => "hls_resource_changed",
            Self::DashManifestTooLarge => "dash_manifest_too_large",
            Self::InvalidDashManifest(_) => "dash_manifest_invalid",
            Self::TooManyDashRepresentations => "dash_too_many_representations",
            Self::TooManyDashSegments => "dash_too_many_segments",
            Self::DashMetadataTooLarge => "dash_metadata_too_large",
            Self::UnsupportedDashFeature(_) => "dash_feature_unsupported",
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
