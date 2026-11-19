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
    let owner = Arc::clone(&inner);
    owner.spawn_owned_worker(async move {
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
    })
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
            .map(|segment| (segment.index(), (segment.start_byte(), segment.len())))
            .collect::<HashMap<_, _>>(),
    ));
    let first_error = super::segment_workers::run(
        inner,
        http,
        task_id,
        correlation_id,
        request,
        probe,
        control,
        &artifact,
        &segments,
        &progress_by_segment,
        &segment_lengths,
        use_if_range,
    )
    .await?;
    let cancelled = control.current_directive() == TransferDirective::Cancel;
    let paused = control.current_directive() == TransferDirective::Pause;

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
