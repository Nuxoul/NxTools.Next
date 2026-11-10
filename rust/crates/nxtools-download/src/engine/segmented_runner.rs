// Task dispatch and range-based transfer: the worker spawn, single-stream
// runner, segmented runner with dynamic rebalance, initial/resume plan
// construction, checkpointing and the shared retry policy.
use super::*;

pub(crate) fn spawn_download_worker(
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

pub(crate) async fn run_task(
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
    let prepared = {
        let tasks = inner.lock_tasks()?;
        let state = tasks
            .get(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.prepared_size_bytes.map(|size| {
            (
                size,
                state.transfer.temporary_path.clone(),
                state.transfer.final_path.clone(),
                state.transfer.downloaded_bytes,
                state.control.clone(),
            )
        })
    };
    if let Some((size, source, target, downloaded_bytes, control)) = prepared {
        let source = source.ok_or(DownloadEngineError::MissingTemporaryPath(task_id))?;
        let target = target.ok_or(DownloadEngineError::MissingTemporaryPath(task_id))?;
        let control = control.ok_or(DownloadEngineError::MissingTransferControl(task_id))?;
        let observed = tokio::fs::metadata(&source).await?.len();
        if observed != size {
            return Err(HttpDownloadError::LengthMismatch {
                expected_bytes: size,
                downloaded_bytes: observed,
            }
            .into());
        }
        if let Some(stop) = hls_stop_directive(&control) {
            if stop == TransferDirective::Cancel {
                match request.media.as_ref() {
                    Some(DownloadRequestMedia::Hls(_)) => {
                        remove_hls_owned_artifacts(
                            &request,
                            task_id,
                            Some(&source),
                            correlation_id,
                            "cancelled",
                            Instant::now(),
                        )?;
                        inner.store.clear_hls_transfer_plan(task_id)?;
                    }
                    Some(DownloadRequestMedia::Dash(_)) => {
                        remove_dash_owned_artifacts(
                            &request,
                            task_id,
                            Some(&source),
                            correlation_id,
                            "cancelled",
                            Instant::now(),
                        )?;
                        inner.store.clear_dash_transfer_plan(task_id)?;
                    }
                    None => remove_path_if_exists(&source)?,
                }
            }
            let mut tasks = inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            state.task.transition_to(
                if stop == TransferDirective::Pause {
                    DownloadStatus::Paused
                } else {
                    DownloadStatus::Cancelled
                },
                now_unix_ms(),
            )?;
            if stop == TransferDirective::Cancel {
                state.transfer.prepared_size_bytes = None;
            }
            state.control = None;
            inner.persist(state)?;
            let task = Box::new(state.task.clone());
            drop(tasks);
            inner.emit(DownloadEvent::TaskChanged { task });
            return Ok(());
        }
        {
            let mut tasks = inner.lock_tasks()?;
            let state = tasks
                .get_mut(&task_id)
                .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
            state
                .task
                .transition_to(DownloadStatus::Downloading, now_unix_ms())?;
            state
                .task
                .transition_to(DownloadStatus::Finalizing, now_unix_ms())?;
            inner.persist(state)?;
            let task = Box::new(state.task.clone());
            drop(tasks);
            inner.emit(DownloadEvent::TaskChanged { task });
        }
        http.commit_prepared(PreparedDownload {
            temporary_path: source,
            final_path: target,
            downloaded_bytes,
        })
        .await?;
        commit_completed_state(&inner, task_id)?;
        if let Some(media) = request.media.as_ref() {
            let (working, clear) = match media {
                DownloadRequestMedia::Hls(_) => (hls_working_directory(&request, task_id), true),
                DownloadRequestMedia::Dash(_) => (dash_working_directory(&request, task_id), false),
            };
            let removed = match tokio::fs::remove_dir_all(&working).await {
                Ok(()) => true,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
                Err(error) => {
                    warn!(event = "download.prepared_cleanup.failed", %correlation_id, ?task_id,
                    native_error_code = error.raw_os_error(), "prepared media staging cleanup failed");
                    false
                }
            };
            if removed {
                let plan = if clear {
                    inner.store.clear_hls_transfer_plan(task_id)
                } else {
                    inner.store.clear_dash_transfer_plan(task_id)
                };
                if let Err(_error) = plan {
                    warn!(event = "download.prepared_cleanup.failed", %correlation_id, ?task_id,
                        error_kind = "storage", "prepared media plan cleanup failed");
                }
            }
        }
        info!(event = "download.prepared_commit.completed", %correlation_id, ?task_id,
            prepared_bytes = size, "retained prepared artifact published");
        return Ok(());
    }

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
        route_kind = probe.server_observation.as_ref().map(|value| value.route_kind.diagnostic_name()),
        peer_present = probe.server_observation.as_ref().is_some_and(|value| value.peer_ip.is_some()),
        response_ms = probe.server_observation.as_ref().map(|value| value.response_ms),
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
    let (derived_temporary, final_path) =
        HttpDownloadClient::artifact_paths(&request, &probe, task_id)?;
    let temporary_path = inner
        .lock_tasks()?
        .get(&task_id)
        .ok_or(DownloadEngineError::TaskNotFound(task_id))?
        .transfer
        .temporary_path
        .clone()
        .unwrap_or(derived_temporary);
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
        let task = Box::new(state.task.clone());
        drop(tasks);
        inner.emit(DownloadEvent::TaskChanged { task });
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

#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_single_stream_task(
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
    let existing_temporary = inner
        .lock_tasks()?
        .get(&task_id)
        .ok_or(DownloadEngineError::TaskNotFound(task_id))?
        .transfer
        .temporary_path
        .clone();
    let mut attempt = 1_u32;
    let outcome = loop {
        let progress_inner = Arc::clone(inner);
        let outcome = http
            .prepare_download(
                request,
                probe,
                task_id,
                control,
                existing_temporary.as_deref(),
                move |progress| {
                    publish_transfer_progress(
                        &progress_inner,
                        task_id,
                        progress.downloaded_bytes,
                        progress.total_bytes,
                        1,
                        None,
                        None,
                    );
                },
            )
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
                state.transfer.prepared_size_bytes = Some(ready.downloaded_bytes);
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
            inner.persist(state)?;
            state.control = None;
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
            inner.persist(state)?;
            state.control = None;
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

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_segment_transfer(
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

pub(crate) fn select_rebalance_candidate(
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

fn select_shrink_candidate(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    worker_controls: &HashMap<u32, DownloadSegmentTransferControl>,
    waiting: &VecDeque<DownloadSegment>,
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
                .unwrap_or(segment.downloaded_bytes());
            let remaining = segment.len().saturating_sub(observed);
            (remaining >= minimum_remaining_bytes
                && waiting.iter().any(|neighbour| {
                    (segment.end_byte().saturating_add(1) == neighbour.start_byte()
                        && neighbour.downloaded_bytes() == 0)
                        || (neighbour.end_byte().saturating_add(1) == segment.start_byte()
                            && observed == 0)
                }))
            .then_some((segment.index(), remaining))
        })
        .max_by_key(|(_, remaining)| *remaining))
}

fn shrink_eligible(
    requested: bool,
    outcome: &Result<DownloadSegmentTransferOutcome, HttpDownloadError>,
    stopping: bool,
    segment_count: usize,
    initial_count: usize,
) -> bool {
    requested
        && matches!(
            outcome,
            Ok(DownloadSegmentTransferOutcome::Rebalanced { .. })
        )
        && !stopping
        && segment_count > initial_count
}

#[cfg(test)]
mod shrink_fixture {
    use super::*;

    pub(super) fn synced_worker_identity_survives_pending_clear_but_not_stops_or_completion() {
        let acknowledged = Ok(DownloadSegmentTransferOutcome::Rebalanced {
            downloaded_bytes: 0,
        });
        assert!(shrink_eligible(true, &acknowledged, false, 3, 2));
        assert!(!shrink_eligible(false, &acknowledged, false, 3, 2));
        assert!(!shrink_eligible(true, &acknowledged, true, 3, 2));
        assert!(!shrink_eligible(true, &acknowledged, false, 2, 2));
        assert!(!shrink_eligible(
            true,
            &Ok(DownloadSegmentTransferOutcome::Completed {
                downloaded_bytes: 1
            }),
            false,
            3,
            2
        ));
    }

    // This is a localhost integration gate, colocated to keep the sample seam
    // cfg(test)-only. Only sampling is synthetic: Range I/O, writer sync,
    // worker joins, plan replacement, pause/reopen and publication are real.
    pub(super) async fn sampled_runner_shrinks_a_grown_saved_plan_and_resumes_exact_artifact() {
        for grow_first in [true, false] {
            let server = HeldRangeServer::start();
            let directory = tempfile::tempdir().expect("shrink directory");
            let database = directory.path().join("downloads.sqlite3");
            let engine = DownloadEngine::open(&database).expect("open shrink engine");
            let request = DownloadRequest {
                source_url: server.url.clone(),
                save_directory: directory.path().to_owned(),
                save_directory_explicit: true,
                file_name: Some("shrink.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                certificate_policy: Default::default(),
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: Some(crate::DownloadAdaptiveSegmentPolicy {
                    enabled: true,
                    max_segments: if grow_first { 3 } else { 2 },
                    min_segment_mib: 1,
                }),
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
            };
            let task_id = engine
                .enqueue(request.clone(), RequestOrigin::Internal)
                .expect("enqueue");
            let http = HttpDownloadClient::new().expect("HTTP client");
            let probe = http.probe(&request).await.expect("localhost probe");
            let (temporary, final_path) =
                HttpDownloadClient::artifact_paths(&request, &probe, task_id)
                    .expect("artifact paths");
            SegmentedArtifact::create(&temporary, 5 * MIB)
                .await
                .expect("saved artifact");
            let plan = TransferPlan::new(
                TransferProtocol::Http,
                server.url.clone(),
                Some(5 * MIB),
                true,
                probe.validator.clone(),
                vec![
                    DownloadSegment::new(0, 0, 4 * MIB - 1, 0, DownloadSegmentState::Waiting)
                        .expect("saved left range"),
                    DownloadSegment::new(1, 4 * MIB, 5 * MIB - 1, 0, DownloadSegmentState::Waiting)
                        .expect("saved right range"),
                ],
            )
            .expect("grown saved plan");
            engine
                .inner
                .store
                .save_transfer_plan(task_id, &plan)
                .expect("save grown plan");
            let control = DownloadTransferControl::new();
            {
                let mut tasks = engine.inner.lock_tasks().expect("lock fixture task state");
                let state = tasks.get_mut(&task_id).expect("fixture task exists");
                for status in [
                    DownloadStatus::Probing,
                    DownloadStatus::Waiting,
                    DownloadStatus::Downloading,
                ] {
                    state
                        .task
                        .transition_to(status, now_unix_ms())
                        .expect("fixture state transition");
                }
                state.control = Some(control.clone());
                state.transfer.probe = Some(probe.clone());
                state.transfer.temporary_path = Some(temporary.clone());
                state.transfer.final_path = Some(final_path.clone());
                engine
                    .inner
                    .persist(state)
                    .expect("persist fixture attempt");
            }
            let (samples, sample_receiver) = async_channel::bounded(1);
            let worker_inner = engine.inner.clone();
            let worker_control = control.clone();
            let worker = tokio::spawn(async move {
                run_segmented_task_with_samples(
                    &worker_inner,
                    task_id,
                    uuid::Uuid::new_v4(),
                    &request,
                    &probe,
                    &worker_control,
                    &http,
                    temporary,
                    final_path,
                    Some(sample_receiver),
                )
                .await
            });
            wait_for_bytes(&engine, task_id, 4096).await;
            for _ in 0..ADAPTIVE_SAMPLE_COUNT {
                samples
                    .send(100_000)
                    .await
                    .expect("runner accepts baseline sample");
            }
            let grown_count = if grow_first { 3 } else { 2 };
            wait_for_plan_count(&engine, task_id, grown_count).await;
            if grow_first {
                // The restarted reduced worker writes another real prefix before
                // slow samples ask it to sync and absorb its queued right tail.
                wait_for_bytes(&engine, task_id, 8192).await;
            }
            for _ in 0..ADAPTIVE_SAMPLE_COUNT - 1 {
                samples
                    .send(10_000)
                    .await
                    .expect("runner accepts slow sample");
            }
            // Fence the fourth sample by waiting for the one-slot channel drain.
            tokio::time::timeout(Duration::from_secs(3), async {
                while !samples.is_empty() {
                    tokio::task::yield_now().await;
                }
            })
            .await
            .expect("runner consumed fourth slow sample");
            assert_eq!(
                engine
                    .inner
                    .store
                    .load_transfer_plan(task_id)
                    .expect("read durable plan")
                    .expect("durable plan exists")
                    .segments()
                    .len(),
                grown_count,
                "four slow samples must not become a five-sample window"
            );
            samples
                .send(10_000)
                .await
                .expect("runner accepts fifth slow sample");
            wait_for_plan_count(&engine, task_id, grown_count - 1).await;
            let shrunk = engine
                .inner
                .store
                .load_transfer_plan(task_id)
                .expect("read shrunk plan")
                .expect("shrunk plan exists");
            let surviving = shrunk
                .segments()
                .iter()
                .find(|segment| segment.index() == 0)
                .expect("real worker continues synced range");
            assert_eq!(surviving.start_byte(), 0);
            assert_eq!(
                surviving.end_byte(),
                if grow_first { 4 * MIB - 1 } else { 5 * MIB - 1 }
            );
            assert!(
                surviving.downloaded_bytes() >= if grow_first { 8192 } else { 4096 },
                "the joined writer's real prefix must survive the durable merge"
            );
            assert!(
                shrunk
                    .segments()
                    .iter()
                    .filter(|segment| segment.state == DownloadSegmentState::Active)
                    .count()
                    <= 1
            );
            let counts = engine
                .inner
                .store
                .load_task_timeline(task_id, 100)
                .expect("read plan history")
                .into_iter()
                .rev() // Storage returns newest first; assert chronological decrease.
                .filter_map(|entry| match entry.event {
                    crate::DownloadTaskTimelineEvent::TransferPlanChanged { segment_count } => {
                        Some(segment_count)
                    }
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert!(
                counts
                    .windows(2)
                    .any(|pair| pair[0] as usize == grown_count
                        && pair[1] as usize == grown_count - 1),
                "SQLite must record the shrink, not just an in-memory decision: {counts:?}"
            );
            control.pause();
            tokio::time::timeout(Duration::from_secs(3), worker)
                .await
                .expect("pause bounded runner")
                .expect("join runner")
                .expect("pause runner");
            assert_eq!(
                engine
                    .task(task_id)
                    .expect("read paused task")
                    .expect("paused task exists")
                    .task
                    .status(),
                DownloadStatus::Paused
            );
            assert!(!directory.path().join("shrink.bin").exists());
            let durable = engine
                .inner
                .store
                .load_transfer_plan(task_id)
                .expect("read durable paused plan")
                .expect("paused plan exists");
            let prefix = durable.segments()[0].downloaded_bytes() as usize;
            assert!(prefix > 0);
            drop(engine);
            let reopened_store = SqliteDownloadStore::open(&database).expect("independent reopen");
            assert_eq!(
                reopened_store
                    .load_transfer_plan(task_id)
                    .expect("read reopened plan")
                    .expect("reopened plan exists")
                    .segments(),
                durable.segments()
            );
            let reopened = DownloadEngine::open(&database).expect("reopen engine");
            server.release.store(true, Ordering::Release);
            reopened.resume(task_id).expect("resume shrunk checkpoint");
            let completed = reopened
                .wait_for_terminal(task_id, Duration::from_secs(5))
                .await
                .expect("resume reaches terminal state");
            assert_eq!(
                completed.task.status(),
                DownloadStatus::Completed,
                "{:?}",
                completed.error
            );
            assert_eq!(
                std::fs::read(directory.path().join("shrink.bin"))
                    .expect("read exact final artifact"),
                server.body.as_ref().clone()
            );
            assert!(
                server
                    .ranges
                    .lock()
                    .expect("lock observed ranges")
                    .iter()
                    .any(|&(start, _)| start == prefix),
                "resumed request must continue at the synced prefix"
            );
            assert_eq!(
                reopened_store
                    .load_transfer_plan(task_id)
                    .expect("read completed plan")
                    .expect("completed plan exists")
                    .segments()
                    .len(),
                grown_count - 1,
                "shrink completion must not fall through to split/re-grow"
            );
        }
    }

    async fn wait_for_bytes(engine: &DownloadEngine, task_id: DownloadTaskId, bytes: u64) {
        tokio::time::timeout(Duration::from_secs(3), async {
            // Progress events can be coalesced after a worker restart. Inspect
            // the real prefix rather than requiring another UI-rate event.
            while {
                use std::io::{Read, Seek};
                let path = engine
                    .task(task_id)
                    .expect("read active task")
                    .expect("active task exists")
                    .temporary_path
                    .expect("temporary artifact is registered");
                let mut byte = [0_u8; 1];
                !std::fs::File::open(path).is_ok_and(|mut file| {
                    file.seek(std::io::SeekFrom::Start(bytes - 1)).is_ok()
                        && file.read_exact(&mut byte).is_ok()
                        && byte[0] == ((bytes - 1) % 251) as u8
                })
            } {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .expect("real Range writer made progress");
    }

    async fn wait_for_plan_count(engine: &DownloadEngine, task_id: DownloadTaskId, count: usize) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while engine
                .inner
                .store
                .load_transfer_plan(task_id)
                .expect("read runner plan")
                .expect("runner plan exists")
                .segments()
                .len()
                != count
            {
                tokio::time::sleep(Duration::from_millis(2)).await;
            }
        })
        .await
        .expect("actual runner persisted the expected plan size");
    }

    struct HeldRangeServer {
        url: String,
        body: Arc<Vec<u8>>,
        ranges: Arc<Mutex<Vec<(usize, usize)>>>,
        release: Arc<AtomicBool>,
        shutdown: Arc<AtomicBool>,
        listener: Option<std::thread::JoinHandle<()>>,
    }

    impl HeldRangeServer {
        fn start() -> Self {
            let listener =
                std::net::TcpListener::bind(("127.0.0.1", 0)).expect("bind held fixture");
            listener
                .set_nonblocking(true)
                .expect("nonblocking listener");
            let url = format!(
                "http://{}/shrink.bin",
                listener.local_addr().expect("fixture address")
            );
            let body = Arc::new(
                (0..5 * MIB)
                    .map(|offset| (offset % 251) as u8)
                    .collect::<Vec<_>>(),
            );
            let ranges = Arc::new(Mutex::new(Vec::new()));
            let release = Arc::new(AtomicBool::new(false));
            let shutdown = Arc::new(AtomicBool::new(false));
            let (worker_body, worker_ranges, worker_release, worker_shutdown) = (
                body.clone(),
                ranges.clone(),
                release.clone(),
                shutdown.clone(),
            );
            let listener = std::thread::spawn(move || {
                let mut connections = Vec::new();
                while !worker_shutdown.load(Ordering::Acquire) {
                    match listener.accept() {
                        Ok((mut stream, _)) => {
                            let (body, ranges, release, shutdown) = (
                                worker_body.clone(),
                                worker_ranges.clone(),
                                worker_release.clone(),
                                worker_shutdown.clone(),
                            );
                            connections.push(std::thread::spawn(move || {
                                use std::io::{Read, Write};
                                // Winsock accepted sockets inherit the listener's
                                // nonblocking mode; the connection worker is blocking.
                                stream.set_nonblocking(false).expect("blocking accepted socket");
                                stream.set_read_timeout(Some(Duration::from_secs(3))).expect("bounded fixture read");
                                stream.set_write_timeout(Some(Duration::from_secs(3))).expect("bounded fixture write");
                                stream.set_nodelay(true).expect("disable fixture Nagle");
                                let mut head = Vec::new();
                                let mut buffer = [0_u8; 2048];
                                while head.len() < 16 * 1024 && !head.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                                    let Ok(read) = stream.read(&mut buffer) else { return; };
                                    if read == 0 { return; }
                                    head.extend_from_slice(&buffer[..read]);
                                }
                                let head = String::from_utf8(head).expect("fixture ASCII request header");
                                if head.starts_with("HEAD ") {
                                    let response = format!("HTTP/1.1 200 OK\r\nAccept-Ranges: bytes\r\nETag: \"shrink\"\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
                                    let _ = stream.write_all(response.as_bytes());
                                    let _ = stream.shutdown(std::net::Shutdown::Write);
                                    return;
                                }
                                let range = head.lines().find_map(|line| {
                                    let (name, value) = line.split_once(':')?;
                                    if !name.eq_ignore_ascii_case("range") { return None; }
                                    let (start, end) = value.trim().strip_prefix("bytes=")?.split_once('-')?;
                                    Some((start.parse::<usize>().ok()?, end.parse::<usize>().ok()?))
                                });
                                let Some((start, end)) = range else {
                                    return;
                                };
                                assert!(start <= end && end < body.len());
                                let response = format!("HTTP/1.1 206 Partial Content\r\nETag: \"shrink\"\r\nContent-Range: bytes {start}-{end}/{}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len(), end - start + 1);
                                if stream.write_all(response.as_bytes()).is_err() { return; }
                                let first_end = (start + 4095).min(end);
                                if stream.write_all(&body[start..=first_end]).is_err() { return; }
                                let _ = stream.flush();
                                if (start, end) == (0, 0) { return; }
                                ranges.lock().expect("lock request record").push((start, end));
                                let deadline = Instant::now() + Duration::from_secs(15);
                                while !release.load(Ordering::Acquire) && !shutdown.load(Ordering::Acquire) && Instant::now() < deadline {
                                    std::thread::sleep(Duration::from_millis(2));
                                }
                                if !shutdown.load(Ordering::Acquire) && first_end < end {
                                    let _ = stream.write_all(&body[first_end + 1..=end]);
                                }
                            }));
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(2))
                        }
                        Err(error) => panic!("localhost accept: {error}"),
                    }
                }
                for connection in connections {
                    connection.join().expect("join held fixture connection");
                }
            });
            Self {
                url,
                body,
                ranges,
                release,
                shutdown,
                listener: Some(listener),
            }
        }
    }

    impl Drop for HeldRangeServer {
        fn drop(&mut self) {
            self.shutdown.store(true, Ordering::Release);
            self.listener
                .take()
                .expect("fixture listener handle")
                .join()
                .expect("join listener");
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn run_segmented_task(
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
    run_segmented_task_with_samples(
        inner,
        task_id,
        correlation_id,
        request,
        probe,
        control,
        http,
        temporary_path,
        final_path,
        #[cfg(test)]
        None,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn run_segmented_task_with_samples(
    inner: &Arc<DownloadEngineInner>,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    probe: &DownloadProbe,
    control: &DownloadTransferControl,
    http: &HttpDownloadClient,
    temporary_path: PathBuf,
    final_path: PathBuf,
    #[cfg(test)] samples: Option<async_channel::Receiver<u64>>,
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
    // The restored layout may already contain adaptive tails. Its current
    // count is not the original floor; derive that from the request's initial
    // connection layout so saved grown plans can shed queued ranges.
    let initial_segment_count =
        u16::try_from(create_initial_segments(total_bytes, Some(max_connections))?.len())
            .unwrap_or(u16::MAX);
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
            injected_speed = async {
                #[cfg(test)]
                if let Some(samples) = samples.as_ref() {
                    return samples.recv().await.ok();
                }
                adaptive_tick.tick().await;
                None::<u64>
            }, if !paused && !cancelled && first_error.is_none() => {
                let now = Instant::now();
                let downloaded = progress_by_segment
                    .lock()
                    .map_err(|_| DownloadEngineError::LockPoisoned)?
                    .values()
                    .copied()
                    .sum::<u64>();
                let elapsed = now.duration_since(sampled_at).as_secs_f64().max(0.001);
                let speed = injected_speed.unwrap_or_else(|| {
                    (downloaded.saturating_sub(sampled_bytes) as f64 / elapsed)
                        .min(u64::MAX as f64) as u64
                });
                sampled_at = now;
                sampled_bytes = downloaded;
                let current_segments = u16::try_from(segment_count).unwrap_or(u16::MAX);
                let decision = adaptive.observe(
                    speed,
                    current_segments,
                    initial_segment_count,
                    retry_observed.load(Ordering::Acquire),
                );
                match decision {
                    AdaptiveConnectionDecision::Grow if pending_rebalance.is_none() && pending_shrink.is_none() => {
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
                    AdaptiveConnectionDecision::Shrink if pending_shrink.is_none() && pending_rebalance.is_none() => {
                        // Only shed a worker adjacent to a queued Waiting range.
                        // Sync its writer before editing the persisted plan.
                        if let Some((candidate_index, _remaining)) = select_shrink_candidate(
                            inner, task_id, &worker_controls, &waiting_segments,
                            &progress_by_segment, adaptive_min_remaining_bytes,
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
                    AdaptiveConnectionDecision::Shrink => {}
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
        let skip_shrink = !shrink_eligible(
            shrink_requested,
            &outcome,
            paused || cancelled || first_error.is_some(),
            segment_count,
            usize::from(initial_segment_count),
        );
        let mut shrink_merged = false;
        let mut shrink_waiting = None;
        if shrink_requested && !skip_shrink {
            let downloaded_bytes = match &outcome {
                Ok(DownloadSegmentTransferOutcome::Rebalanced { downloaded_bytes }) => {
                    *downloaded_bytes
                }
                _ => unreachable!("shrink only follows a synced worker rebalance"),
            };
            shrink_waiting = Some(checkpoint_segment(
                inner,
                task_id,
                correlation_id,
                &segment,
                downloaded_bytes,
                DownloadSegmentState::Waiting,
            )?);
        }
        if let Some(waiting_checkpoint) = shrink_waiting.as_ref() {
            let Some(plan) = inner.store.load_transfer_plan(task_id)? else {
                return Err(DownloadEngineError::EmptySegmentLayout(task_id));
            };
            if let Some(cancelled) = plan
                .segments()
                .iter()
                .find(|candidate| candidate.index() == segment.index())
                && cancelled.state == DownloadSegmentState::Waiting
                && let Some(neighbour) = waiting_segments.iter().find(|candidate| {
                    (cancelled.downloaded_bytes() == 0
                        && candidate.end_byte().saturating_add(1) == cancelled.start_byte())
                        || (candidate.downloaded_bytes() == 0
                            && cancelled.end_byte().saturating_add(1) == candidate.start_byte())
                })
            {
                let (shed_index, surviving_index) = if cancelled.downloaded_bytes() == 0 {
                    (segment.index(), neighbour.index())
                } else {
                    (neighbour.index(), segment.index())
                };
                let neighbour_index = neighbour.index();
                let mut merged_plan = plan.clone();
                if merged_plan
                    .merge_waiting_segment(shed_index, surviving_index)
                    .is_ok()
                {
                    inner.persist_transfer_plan_change(task_id, &merged_plan)?;
                    waiting_segments.retain(|candidate| candidate.index() != neighbour_index);
                    let merged = merged_plan
                        .segments()
                        .iter()
                        .find(|candidate| candidate.index() == surviving_index)
                        .ok_or(DownloadEngineError::IncompleteSegmentLayout(task_id))?
                        .clone();
                    progress_by_segment
                        .lock()
                        .map_err(|_| DownloadEngineError::LockPoisoned)?
                        .remove(&shed_index);
                    segment_lengths
                        .lock()
                        .map_err(|_| DownloadEngineError::LockPoisoned)?
                        .remove(&shed_index);
                    segment_lengths
                        .lock()
                        .map_err(|_| DownloadEngineError::LockPoisoned)?
                        .insert(surviving_index, merged.len());
                    progress_by_segment
                        .lock()
                        .map_err(|_| DownloadEngineError::LockPoisoned)?
                        .insert(surviving_index, merged.downloaded_bytes());
                    waiting_segments.push_back(merged);
                    segment_count = segment_count.saturating_sub(1);
                    shrink_merged = true;
                    adaptive_growth_remaining = 0;
                    adaptive.cancel_growth();
                    adaptive.disabled = true;
                    info!(
                        event = "download.adaptive_segments.shrink_merged",
                        %correlation_id, ?task_id,
                        cancelled_segment_index = shed_index,
                        merged_segment_index = surviving_index,
                        current_segments = segment_count,
                        "synced adaptive shrink merged a waiting range"
                    );
                }
            }
            if !shrink_merged {
                waiting_segments.push_back(waiting_checkpoint.clone());
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
            Ok(DownloadSegmentTransferOutcome::Rebalanced { .. }) if shrink_waiting.is_some() => {
                // Synced Waiting checkpoint was already merged or queued above.
            }
            Ok(DownloadSegmentTransferOutcome::Rebalanced { downloaded_bytes })
                if shrink_requested =>
            {
                let waiting = checkpoint_segment(
                    inner,
                    task_id,
                    correlation_id,
                    &segment,
                    downloaded_bytes,
                    DownloadSegmentState::Waiting,
                )?;
                if !paused && !cancelled && first_error.is_none() {
                    waiting_segments.push_back(waiting);
                }
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
            && pending_shrink.is_none()
            && !shrink_requested
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
        state.transfer.prepared_size_bytes = Some(total_bytes);
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

pub(crate) fn transfer_retry_delay(failed_attempt: u32) -> Duration {
    TRANSFER_RETRY_BASE_DELAY.saturating_mul(1 << failed_attempt.saturating_sub(1).min(6))
}

pub(crate) fn should_retry_transfer(failed_attempt: u32, error: &HttpDownloadError) -> bool {
    failed_attempt < MAX_TRANSFER_ATTEMPTS && error.is_retryable_transfer()
}

pub(crate) fn create_initial_segments(
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

pub(crate) fn checkpoint_segment(
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

pub(crate) fn validate_resumed_plan(
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

#[cfg(test)]
mod shrink_regression {
    #[test]
    fn synced_worker_identity_survives_pending_clear_but_not_stops_or_completion() {
        super::shrink_fixture::synced_worker_identity_survives_pending_clear_but_not_stops_or_completion();
    }

    #[tokio::test]
    async fn sampled_runner_shrinks_a_grown_saved_plan_and_resumes_exact_artifact() {
        super::shrink_fixture::sampled_runner_shrinks_a_grown_saved_plan_and_resumes_exact_artifact().await;
    }
}
