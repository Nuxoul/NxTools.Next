// DASH media transfer runner: manifest plan compatibility, fragment
// transfers, concatenation finalize with stop directives and owned-artifact
// cleanup.
use super::*;

pub(crate) async fn run_dash_task(
    inner: Arc<DownloadEngineInner>,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    selection: &crate::DashDownloadRequest,
    http: &HttpDownloadClient,
) -> Result<(), DownloadEngineError> {
    let started_at = Instant::now();
    let stored_plan = inner.store.load_dash_transfer_plan(task_id)?;
    let manifest_source = if stored_plan.is_some() {
        "network_revalidation"
    } else {
        "network"
    };
    info!(
        event = "download.dash_manifest.started",
        %correlation_id,
        ?task_id,
        source = manifest_source,
        "DASH manifest resolution started"
    );
    let manifest = match http
        .load_dash_manifest(request, &selection.manifest_url)
        .await
    {
        Ok(manifest) => manifest,
        Err(error) => {
            warn!(
                event = "download.dash_manifest.failed",
                %correlation_id,
                ?task_id,
                source = manifest_source,
                error_kind = error.kind(),
                native_error_code = error.io_error().and_then(std::io::Error::raw_os_error),
                elapsed_ms = started_at.elapsed().as_millis(),
                "DASH manifest resolution failed"
            );
            return Err(error.into());
        }
    };
    info!(
        event = "download.dash_manifest.completed",
        %correlation_id,
        ?task_id,
        source = manifest_source,
        video_track_count = manifest
            .tracks
            .iter()
            .filter(|track| track.kind == crate::dash::DashTrackKind::Video)
            .count(),
        audio_track_count = manifest
            .tracks
            .iter()
            .filter(|track| track.kind == crate::dash::DashTrackKind::Audio)
            .count(),
        segment_count = manifest
            .tracks
            .iter()
            .map(|track| track.segments.len())
            .sum::<usize>(),
        elapsed_ms = started_at.elapsed().as_millis(),
        "DASH manifest resolved"
    );
    let mut selected_tracks = Vec::new();
    for (kind, identity) in [
        (
            crate::dash::DashTrackKind::Video,
            selection.video_representation_id.as_deref(),
        ),
        (
            crate::dash::DashTrackKind::Audio,
            selection.audio_representation_id.as_deref(),
        ),
    ] {
        let Some(identity) = identity else {
            continue;
        };
        let track = manifest
            .tracks
            .iter()
            .find(|track| track.kind == kind && track.representation_id == identity)
            .cloned()
            .ok_or(HttpDownloadError::InvalidDashManifest(
                "selected DASH Representation no longer exists",
            ))?;
        selected_tracks.push(track);
    }
    if selected_tracks.is_empty() {
        return Err(
            HttpDownloadError::InvalidDashManifest("DASH request has no selected track").into(),
        );
    }
    let selected_manifest = crate::dash::DashManifest {
        final_url: manifest.final_url.clone(),
        tracks: selected_tracks,
    };
    if let Some(stored) = stored_plan.as_ref() {
        if !dash_plan_is_compatible(stored, &selected_manifest) {
            return Err(HttpDownloadError::InvalidDashManifest(
                "DASH manifest changed incompatibly with the durable transfer plan",
            )
            .into());
        }
        inner
            .store
            .refresh_dash_transfer_plan_manifest(task_id, &selected_manifest)?;
    } else {
        inner
            .store
            .save_dash_transfer_plan(task_id, &selected_manifest)?;
        info!(
            event = "download.dash_transfer_plan.created",
            %correlation_id,
            ?task_id,
            track_count = selected_manifest.tracks.len(),
            segment_count = selected_manifest
                .tracks
                .iter()
                .map(|track| track.segments.len())
                .sum::<usize>(),
            "DASH transfer plan persisted"
        );
    }
    let selected_track_count = selected_manifest.tracks.len();
    let selected_segment_count = selected_manifest
        .tracks
        .iter()
        .map(|track| track.segments.len())
        .sum::<usize>();
    let fallback_file_name = url::Url::parse(&manifest.final_url)
        .ok()
        .and_then(|url| {
            Path::new(url.path())
                .file_stem()
                .map(|stem| format!("{}.mp4", stem.to_string_lossy()))
        })
        .unwrap_or_else(|| "video.mp4".into());
    let probe = DownloadProbe {
        final_url: manifest.final_url.clone(),
        file_name: Some(fallback_file_name),
        total_bytes: None,
        supports_ranges: false,
        content_type: Some("application/dash+xml".into()),
        validator: crate::ResourceValidator::default(),
        media: Some(crate::DownloadMediaProbe::Dash(manifest.probe())),
    };
    let (temporary_path, final_path) =
        HttpDownloadClient::artifact_paths(request, &probe, task_id)?;
    tokio::fs::create_dir_all(&request.save_directory).await?;
    if tokio::fs::try_exists(&final_path).await? {
        return Err(HttpDownloadError::DestinationExists(final_path).into());
    }
    let working_directory = dash_working_directory(request, task_id);
    tokio::fs::create_dir_all(&working_directory).await?;
    let control = {
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.probe = Some(probe);
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
        state.transfer_rate = Some(TransferRateSampler::new(0, Instant::now()));
        let control = state
            .control
            .clone()
            .ok_or(DownloadEngineError::MissingTransferControl(task_id))?;
        inner.persist(state)?;
        inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(state.task.clone()),
        });
        control
    };
    let mut resources = Vec::new();
    for track in &selected_manifest.tracks {
        if let Some(initialization_url) = track.initialization_url.as_ref() {
            resources.push((track.kind, None, initialization_url.clone()));
        }
        resources.extend(
            track
                .segments
                .iter()
                .map(|segment| (track.kind, Some(segment.index), segment.url.clone())),
        );
    }
    info!(
        event = "download.dash_transfer.started",
        %correlation_id,
        ?task_id,
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        track_count = selected_track_count,
        segment_count = selected_segment_count,
        requested_connections = request.connection_limit.unwrap_or(1),
        "DASH media transfer started"
    );

    let stored_fragments = stored_plan
        .as_ref()
        .map(|plan| {
            plan.fragments
                .iter()
                .map(|fragment| ((fragment.kind, fragment.segment_index), fragment.clone()))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_default();
    let mut adopted_fragments = 0_usize;
    let mut reset_fragments = 0_usize;
    let mut downloaded_bytes = 0_u64;
    let mut pending_fragments = VecDeque::new();
    for (index, (kind, segment_index, _)) in resources.iter().enumerate() {
        let staging_path = working_directory.join(format!("{index:08}.part"));
        let completed_path = working_directory.join(format!("{index:08}.fragment"));
        remove_async_file_if_exists(&staging_path).await?;
        match tokio::fs::metadata(&completed_path).await {
            Ok(metadata) if metadata.len() > 0 => {
                if stored_fragments
                    .get(&(*kind, *segment_index))
                    .is_none_or(|fragment| {
                        fragment.state != DownloadSegmentState::Completed
                            || fragment.downloaded_bytes != metadata.len()
                    })
                {
                    adopted_fragments = adopted_fragments.saturating_add(1);
                }
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    *kind,
                    *segment_index,
                    DownloadSegmentState::Completed,
                    metadata.len(),
                )?;
                downloaded_bytes = downloaded_bytes
                    .checked_add(metadata.len())
                    .ok_or(HttpDownloadError::LengthOverflow)?;
            }
            Ok(_) => {
                if stored_fragments
                    .get(&(*kind, *segment_index))
                    .is_some_and(|fragment| fragment.state == DownloadSegmentState::Completed)
                {
                    reset_fragments = reset_fragments.saturating_add(1);
                }
                remove_async_file_if_exists(&completed_path).await?;
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    *kind,
                    *segment_index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                pending_fragments.push_back((
                    index,
                    *kind,
                    *segment_index,
                    resources[index].2.clone(),
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if stored_fragments
                    .get(&(*kind, *segment_index))
                    .is_some_and(|fragment| fragment.state == DownloadSegmentState::Completed)
                {
                    reset_fragments = reset_fragments.saturating_add(1);
                }
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    *kind,
                    *segment_index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                pending_fragments.push_back((
                    index,
                    *kind,
                    *segment_index,
                    resources[index].2.clone(),
                ));
            }
            Err(error) => return Err(error.into()),
        }
    }
    if adopted_fragments > 0 || reset_fragments > 0 {
        info!(
            event = "download.dash_checkpoints.reconciled",
            %correlation_id,
            ?task_id,
            adopted_fragments,
            reset_fragments,
            fragment_count = resources.len(),
            "DASH fragment checkpoints reconciled with durable artifacts"
        );
    }
    let durable_completed_bytes = Arc::new(AtomicU64::new(downloaded_bytes));
    let active_progress = Arc::new(Mutex::new(HashMap::<u32, u64>::new()));
    let active_connections = Arc::new(AtomicU16::new(0));
    let max_connections = usize::from(request.connection_limit.unwrap_or(1).clamp(1, 64));
    let mut transfers = tokio::task::JoinSet::new();
    while transfers.len() < max_connections
        && let Some((index, kind, segment_index, resource_url)) = pending_fragments.pop_front()
    {
        inner.store.save_dash_fragment_checkpoint(
            task_id,
            kind,
            segment_index,
            DownloadSegmentState::Active,
            0,
        )?;
        spawn_dash_fragment_transfer(
            &mut transfers,
            &inner,
            http,
            task_id,
            correlation_id,
            request,
            &control,
            &working_directory,
            &durable_completed_bytes,
            &active_progress,
            &active_connections,
            index,
            kind,
            segment_index,
            resource_url,
        );
    }
    let mut paused = false;
    let mut cancelled = false;
    let mut first_error = None;
    while let Some(joined) = transfers.join_next().await {
        let (index, kind, segment_index, staging_path, completed_path, outcome) =
            joined.map_err(|error| DownloadEngineError::Runtime(error.to_string()))?;
        active_progress
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .remove(&(index as u32));
        match outcome {
            Ok(HlsSegmentDownloadOutcome::Completed {
                downloaded_bytes: fragment_bytes,
            }) => {
                tokio::fs::rename(&staging_path, &completed_path).await?;
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    kind,
                    segment_index,
                    DownloadSegmentState::Completed,
                    fragment_bytes,
                )?;
                downloaded_bytes = durable_completed_bytes
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                        current.checked_add(fragment_bytes)
                    })
                    .map(|previous| previous + fragment_bytes)
                    .map_err(|_| HttpDownloadError::LengthOverflow)?;
                let mut tasks = inner.lock_tasks()?;
                let state = tasks
                    .get_mut(&task_id)
                    .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
                state.transfer.downloaded_bytes = downloaded_bytes;
                inner.persist(state)?;
            }
            Ok(HlsSegmentDownloadOutcome::Paused) => {
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    kind,
                    segment_index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                paused = true;
            }
            Ok(HlsSegmentDownloadOutcome::Cancelled) => {
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    kind,
                    segment_index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                cancelled = true;
            }
            Err(error) => {
                inner.store.save_dash_fragment_checkpoint(
                    task_id,
                    kind,
                    segment_index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                first_error.get_or_insert(error);
            }
        }
        if !paused
            && !cancelled
            && first_error.is_none()
            && let Some((index, kind, segment_index, resource_url)) = pending_fragments.pop_front()
        {
            inner.store.save_dash_fragment_checkpoint(
                task_id,
                kind,
                segment_index,
                DownloadSegmentState::Active,
                0,
            )?;
            spawn_dash_fragment_transfer(
                &mut transfers,
                &inner,
                http,
                task_id,
                correlation_id,
                request,
                &control,
                &working_directory,
                &durable_completed_bytes,
                &active_progress,
                &active_connections,
                index,
                kind,
                segment_index,
                resource_url,
            );
        }
    }
    downloaded_bytes = durable_completed_bytes.load(Ordering::Acquire);
    if let Some(error) = first_error {
        return Err(error.into());
    }
    if !paused && !cancelled {
        match hls_stop_directive(&control) {
            Some(TransferDirective::Pause) => paused = true,
            Some(TransferDirective::Cancel) => cancelled = true,
            Some(TransferDirective::Running) | None => {}
        }
    }
    if paused {
        transition_dash_stopped(&inner, task_id, DownloadStatus::Paused, downloaded_bytes)?;
        info!(
            event = "download.dash_transfer.paused",
            %correlation_id,
            ?task_id,
            proxy_mode = proxy_mode_name(http.network_proxy_mode()),
            proxy_source = http.network_proxy_source(),
            downloaded_bytes,
            elapsed_ms = started_at.elapsed().as_millis(),
            "DASH media transfer paused"
        );
        return Ok(());
    }
    if cancelled {
        let _ = validate_dash_artifact(&working_directory, true)?;
        if let Err(error) = tokio::fs::remove_dir_all(&working_directory).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            return Err(error.into());
        }
        inner.store.clear_dash_transfer_plan(task_id)?;
        transition_dash_stopped(&inner, task_id, DownloadStatus::Cancelled, downloaded_bytes)?;
        info!(
            event = "download.dash_transfer.cancelled",
            %correlation_id,
            ?task_id,
            proxy_mode = proxy_mode_name(http.network_proxy_mode()),
            proxy_source = http.network_proxy_source(),
            downloaded_bytes,
            elapsed_ms = started_at.elapsed().as_millis(),
            "DASH media transfer cancelled"
        );
        return Ok(());
    }
    {
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.downloaded_bytes = downloaded_bytes;
        state
            .task
            .transition_to(DownloadStatus::Finalizing, now_unix_ms())?;
        inner.persist(state)?;
        inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(state.task.clone()),
        });
    }
    let operation = if selected_track_count == 2 {
        "ffmpeg_stream_copy_mux"
    } else {
        "single_track_concatenate"
    };
    let task_started_at = started_at;
    let finalize_started_at = Instant::now();
    let started_at = finalize_started_at;
    let mut ffmpeg_version_status = if selected_track_count == 2 {
        "not_started"
    } else {
        "not_applicable"
    };
    let mut ffmpeg_elapsed_ms = None;
    info!(
        event = "download.dash_finalize.started",
        %correlation_id,
        ?task_id,
        operation,
        "DASH finalization started"
    );
    let mut video_paths = Vec::new();
    let mut audio_paths = Vec::new();
    for (index, (kind, _, _)) in resources.iter().enumerate() {
        let path = working_directory.join(format!("{index:08}.fragment"));
        match kind {
            crate::dash::DashTrackKind::Video => video_paths.push(path),
            crate::dash::DashTrackKind::Audio => audio_paths.push(path),
        }
    }
    if selected_track_count == 1 {
        let paths = if video_paths.is_empty() {
            &audio_paths
        } else {
            &video_paths
        };
        match concatenate_dash_fragments(paths, &temporary_path, &control).await {
            Ok(crate::media::MediaProcessOutcome::Completed) => {}
            Ok(outcome) => {
                return complete_dash_finalize_stop(
                    &inner,
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    task_started_at,
                    downloaded_bytes,
                    outcome,
                    &temporary_path,
                    &working_directory,
                )
                .await;
            }
            Err(error) => {
                log_dash_finalize_failure(
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    &error,
                );
                return Err(error);
            }
        }
    } else {
        let video_track_path = working_directory.join("video.track");
        let audio_track_path = working_directory.join("audio.track");
        match concatenate_dash_fragments(&video_paths, &video_track_path, &control).await {
            Ok(crate::media::MediaProcessOutcome::Completed) => {}
            Ok(outcome) => {
                return complete_dash_finalize_stop(
                    &inner,
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    task_started_at,
                    downloaded_bytes,
                    outcome,
                    &temporary_path,
                    &working_directory,
                )
                .await;
            }
            Err(error) => {
                log_dash_finalize_failure(
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    &error,
                );
                return Err(error);
            }
        }
        match concatenate_dash_fragments(&audio_paths, &audio_track_path, &control).await {
            Ok(crate::media::MediaProcessOutcome::Completed) => {}
            Ok(outcome) => {
                return complete_dash_finalize_stop(
                    &inner,
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    task_started_at,
                    downloaded_bytes,
                    outcome,
                    &temporary_path,
                    &working_directory,
                )
                .await;
            }
            Err(error) => {
                log_dash_finalize_failure(
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    &error,
                );
                return Err(error);
            }
        }
        if let Err(error) = remove_async_file_if_exists(&temporary_path).await {
            let error = DownloadEngineError::Io(error);
            log_dash_finalize_failure(
                task_id,
                correlation_id,
                operation,
                ffmpeg_version_status,
                started_at,
                &error,
            );
            return Err(error);
        }
        let resolution_started = Instant::now();
        let configured_ffmpeg = inner
            .ffmpeg_executable
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .clone();
        let ffmpeg_resolution =
            crate::media::resolve_ffmpeg_executable(configured_ffmpeg.as_deref());
        let Some(ffmpeg_resolution) = ffmpeg_resolution else {
            warn!(
                event = "download.ffmpeg_resolution.completed",
                %correlation_id,
                ?task_id,
                source = "missing",
                available = false,
                elapsed_ms = resolution_started.elapsed().as_millis(),
                "FFmpeg executable resolution completed"
            );
            let error = DownloadEngineError::FfmpegUnavailable;
            log_dash_finalize_failure(
                task_id,
                correlation_id,
                operation,
                "unavailable",
                finalize_started_at,
                &error,
            );
            return Err(error);
        };
        info!(
            event = "download.ffmpeg_resolution.completed",
            %correlation_id,
            ?task_id,
            source = ffmpeg_resolution.source.as_str(),
            available = true,
            elapsed_ms = resolution_started.elapsed().as_millis(),
            "FFmpeg executable resolution completed"
        );
        let ffmpeg_started = Instant::now();
        match crate::media::detect_ffmpeg_version(&ffmpeg_resolution.executable, &control).await {
            Ok(crate::media::MediaProcessOutcome::Completed) => {}
            Ok(outcome) => {
                return complete_dash_finalize_stop(
                    &inner,
                    task_id,
                    correlation_id,
                    operation,
                    "interrupted",
                    started_at,
                    task_started_at,
                    downloaded_bytes,
                    outcome,
                    &temporary_path,
                    &working_directory,
                )
                .await;
            }
            Err(error) => {
                warn!(
                    event = "download.dash_finalize.failed",
                    %correlation_id,
                    ?task_id,
                    operation,
                    ffmpeg_version_status = "unavailable",
                    error_kind = error.kind(),
                    exit_code = error.exit_code(),
                    native_error_code = error.native_error_code(),
                    ffmpeg_elapsed_ms = ffmpeg_started.elapsed().as_millis(),
                    elapsed_ms = finalize_started_at.elapsed().as_millis(),
                    "FFmpeg detection failed"
                );
                return Err(DownloadEngineError::media_process(error));
            }
        }
        ffmpeg_version_status = "detected";
        match crate::media::mux_dash_tracks(
            &ffmpeg_resolution.executable,
            &video_track_path,
            &audio_track_path,
            &temporary_path,
            &control,
        )
        .await
        {
            Ok(crate::media::MediaProcessOutcome::Completed) => {
                ffmpeg_elapsed_ms = Some(ffmpeg_started.elapsed().as_millis());
            }
            Ok(outcome) => {
                return complete_dash_finalize_stop(
                    &inner,
                    task_id,
                    correlation_id,
                    operation,
                    ffmpeg_version_status,
                    started_at,
                    task_started_at,
                    downloaded_bytes,
                    outcome,
                    &temporary_path,
                    &working_directory,
                )
                .await;
            }
            Err(error) => {
                if let Err(cleanup_error) = remove_async_file_if_exists(&temporary_path).await {
                    let cleanup_error = DownloadEngineError::Io(cleanup_error);
                    log_dash_finalize_failure(
                        task_id,
                        correlation_id,
                        operation,
                        ffmpeg_version_status,
                        started_at,
                        &cleanup_error,
                    );
                    return Err(cleanup_error);
                }
                warn!(
                    event = "download.dash_finalize.failed",
                    %correlation_id,
                    ?task_id,
                    operation,
                    ffmpeg_version_status = "detected",
                    error_kind = error.kind(),
                    exit_code = error.exit_code(),
                    native_error_code = error.native_error_code(),
                    ffmpeg_elapsed_ms = ffmpeg_started.elapsed().as_millis(),
                    elapsed_ms = finalize_started_at.elapsed().as_millis(),
                    "FFmpeg stream-copy mux failed"
                );
                return Err(DownloadEngineError::media_process(error));
            }
        }
    }
    if let Some(directive) = hls_stop_directive(&control) {
        let outcome = match directive {
            TransferDirective::Pause => crate::media::MediaProcessOutcome::Paused,
            TransferDirective::Cancel => crate::media::MediaProcessOutcome::Cancelled,
            TransferDirective::Running => unreachable!("stop helper returned Running"),
        };
        return complete_dash_finalize_stop(
            &inner,
            task_id,
            correlation_id,
            operation,
            ffmpeg_version_status,
            started_at,
            task_started_at,
            downloaded_bytes,
            outcome,
            &temporary_path,
            &working_directory,
        )
        .await;
    }
    if let Err(error) = http
        .commit_prepared(PreparedDownload {
            temporary_path,
            final_path,
            downloaded_bytes,
        })
        .await
    {
        let error = DownloadEngineError::Http(error);
        log_dash_finalize_failure(
            task_id,
            correlation_id,
            operation,
            ffmpeg_version_status,
            started_at,
            &error,
        );
        return Err(error);
    }
    if let Err(error) = commit_completed_state(&inner, task_id) {
        log_dash_finalize_failure(
            task_id,
            correlation_id,
            operation,
            ffmpeg_version_status,
            started_at,
            &error,
        );
        return Err(error);
    }
    let directory_removed = match tokio::fs::remove_dir_all(&working_directory).await {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => true,
        Err(error) => {
            warn!(
                event = "download.dash_cleanup.failed",
                %correlation_id,
                ?task_id,
                reason = "completed",
                artifact_kind = "fragment_directory",
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "DASH task staging cleanup failed"
            );
            false
        }
    };
    if directory_removed && let Err(_error) = inner.store.clear_dash_transfer_plan(task_id) {
        warn!(
            event = "download.dash_cleanup.failed",
            %correlation_id,
            ?task_id,
            reason = "completed",
            artifact_kind = "durable_plan",
            error_kind = "storage",
            native_error_code = Option::<i32>::None,
            elapsed_ms = started_at.elapsed().as_millis(),
            "DASH durable plan cleanup failed after publication"
        );
    }
    info!(
        event = "download.dash_finalize.completed",
        %correlation_id,
        ?task_id,
        operation,
        outcome = "completed",
        ffmpeg_version_status,
        ffmpeg_elapsed_ms,
        exit_code = if ffmpeg_version_status == "detected" {
            Some(0_i32)
        } else {
            None
        },
        downloaded_bytes,
        elapsed_ms = finalize_started_at.elapsed().as_millis(),
        "DASH finalization completed"
    );
    info!(
        event = "download.dash_transfer.completed",
        %correlation_id,
        ?task_id,
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        track_count = selected_track_count,
        segment_count = selected_segment_count,
        downloaded_bytes,
        elapsed_ms = task_started_at.elapsed().as_millis(),
        "DASH media transfer completed"
    );
    Ok(())
}

pub(crate) fn dash_plan_is_compatible(
    stored: &crate::storage::StoredDashTransferPlan,
    current: &crate::dash::DashManifest,
) -> bool {
    if stored.manifest.tracks.len() != current.tracks.len() {
        return false;
    }
    let tracks_match =
        stored
            .manifest
            .tracks
            .iter()
            .zip(&current.tracks)
            .all(|(stored, current)| {
                stored.kind == current.kind
                    && stored.representation_id == current.representation_id
                    && stored.bandwidth == current.bandwidth
                    && stored.resolution == current.resolution
                    && stored.codecs == current.codecs
                    && stored.mime_type == current.mime_type
                    && stored.language == current.language
                    && stored.initialization_url.is_some() == current.initialization_url.is_some()
                    && stored.segments.len() == current.segments.len()
                    && stored
                        .segments
                        .iter()
                        .zip(&current.segments)
                        .all(|(stored, current)| {
                            stored.index == current.index
                                && stored.number == current.number
                                && stored.time == current.time
                                && stored.duration_millis == current.duration_millis
                                && stored.duration_units == current.duration_units
                                && stored.timescale == current.timescale
                        })
            });
    if !tracks_match {
        return false;
    }
    let expected_fragment_count = current
        .tracks
        .iter()
        .map(|track| {
            track
                .segments
                .len()
                .saturating_add(usize::from(track.initialization_url.is_some()))
        })
        .sum::<usize>();
    stored.fragments.len() == expected_fragment_count
        && current.tracks.iter().all(|track| {
            let initialization_matches = track.initialization_url.is_none()
                || stored.fragments.iter().any(|fragment| {
                    fragment.kind == track.kind && fragment.segment_index.is_none()
                });
            initialization_matches
                && track.segments.iter().all(|segment| {
                    stored.fragments.iter().any(|fragment| {
                        fragment.kind == track.kind && fragment.segment_index == Some(segment.index)
                    })
                })
        })
}

pub(crate) async fn remove_async_file_if_exists(path: &Path) -> Result<(), std::io::Error> {
    match tokio::fs::remove_file(path).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

pub(crate) async fn concatenate_dash_fragments(
    fragments: &[PathBuf],
    output_path: &Path,
    control: &DownloadTransferControl,
) -> Result<crate::media::MediaProcessOutcome, DownloadEngineError> {
    if fragments.is_empty() {
        return Err(HttpDownloadError::InvalidDashManifest(
            "selected DASH track has no durable fragments",
        )
        .into());
    }
    let mut output = tokio::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(output_path)
        .await?;
    for fragment in fragments {
        let mut input = tokio::fs::File::open(fragment).await?;
        // Every tokio::fs read/write hops to the blocking pool; a 64 KiB
        // buffer made finalize-merge crawl on large media. 1 MiB keeps the
        // same per-chunk stop-directive responsiveness at 1/16th of the hops.
        let mut buffer = vec![0_u8; 1024 * 1024];
        loop {
            if let Some(directive) = hls_stop_directive(control) {
                drop(output);
                remove_async_file_if_exists(output_path).await?;
                return Ok(match directive {
                    TransferDirective::Pause => crate::media::MediaProcessOutcome::Paused,
                    TransferDirective::Cancel => crate::media::MediaProcessOutcome::Cancelled,
                    TransferDirective::Running => unreachable!("stop helper returned Running"),
                });
            }
            let read = input.read(&mut buffer).await?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read]).await?;
            tokio::task::yield_now().await;
        }
    }
    output.flush().await?;
    output.sync_all().await?;
    Ok(crate::media::MediaProcessOutcome::Completed)
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn complete_dash_finalize_stop(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    operation: &'static str,
    ffmpeg_version_status: &'static str,
    finalize_started_at: Instant,
    task_started_at: Instant,
    downloaded_bytes: u64,
    outcome: crate::media::MediaProcessOutcome,
    temporary_path: &Path,
    working_directory: &Path,
) -> Result<(), DownloadEngineError> {
    let stop_result = async {
        remove_async_file_if_exists(temporary_path).await?;
        let result = match outcome {
            crate::media::MediaProcessOutcome::Paused => (DownloadStatus::Paused, "paused"),
            crate::media::MediaProcessOutcome::Cancelled => {
                match tokio::fs::remove_dir_all(working_directory).await {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(DownloadEngineError::Io(error)),
                }
                inner.store.clear_dash_transfer_plan(task_id)?;
                (DownloadStatus::Cancelled, "cancelled")
            }
            crate::media::MediaProcessOutcome::Completed => {
                unreachable!("completed finalization is not a stop outcome")
            }
        };
        transition_dash_stopped(inner, task_id, result.0, downloaded_bytes)?;
        Ok::<_, DownloadEngineError>(result)
    }
    .await;
    let (status, outcome_name) = match stop_result {
        Ok(result) => result,
        Err(error) => {
            log_dash_finalize_failure(
                task_id,
                correlation_id,
                operation,
                ffmpeg_version_status,
                finalize_started_at,
                &error,
            );
            return Err(error);
        }
    };
    info!(
        event = "download.dash_finalize.completed",
        %correlation_id,
        ?task_id,
        operation,
        outcome = outcome_name,
        ffmpeg_version_status,
        exit_code = Option::<i32>::None,
        downloaded_bytes,
        elapsed_ms = finalize_started_at.elapsed().as_millis(),
        "DASH finalization stopped by task control"
    );
    info!(
        event = if status == DownloadStatus::Paused {
            "download.dash_transfer.paused"
        } else {
            "download.dash_transfer.cancelled"
        },
        %correlation_id,
        ?task_id,
        downloaded_bytes,
        elapsed_ms = task_started_at.elapsed().as_millis(),
        "DASH media transfer stopped during finalization"
    );
    Ok(())
}

pub(crate) fn log_dash_finalize_failure(
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    operation: &'static str,
    ffmpeg_version_status: &'static str,
    started_at: Instant,
    error: &DownloadEngineError,
) {
    warn!(
        event = "download.dash_finalize.failed",
        %correlation_id,
        ?task_id,
        operation,
        ffmpeg_version_status,
        exit_code = Option::<i32>::None,
        error_kind = match error {
            DownloadEngineError::Http(_) => "artifact_commit",
            DownloadEngineError::Io(_) => "filesystem",
            DownloadEngineError::Storage(_) => "storage",
            DownloadEngineError::Task(_) => "state",
            _ => "finalize",
        },
        native_error_code = error.native_os_error(),
        elapsed_ms = started_at.elapsed().as_millis(),
        "DASH finalization failed"
    );
}

pub(crate) fn transition_dash_stopped(
    inner: &DownloadEngineInner,
    task_id: DownloadTaskId,
    status: DownloadStatus,
    downloaded_bytes: u64,
) -> Result<(), DownloadEngineError> {
    let mut tasks = inner.lock_tasks()?;
    let state = tasks
        .get_mut(&task_id)
        .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
    state.transfer.downloaded_bytes = downloaded_bytes;
    state.task.transition_to(status, now_unix_ms())?;
    state.control = None;
    inner.persist(state)?;
    inner.emit(DownloadEvent::TaskChanged {
        task: Box::new(state.task.clone()),
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_dash_fragment_transfer(
    transfers: &mut tokio::task::JoinSet<DashFragmentTransferResult>,
    inner: &Arc<DownloadEngineInner>,
    http: &HttpDownloadClient,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    control: &DownloadTransferControl,
    working_directory: &Path,
    durable_completed_bytes: &Arc<AtomicU64>,
    active_progress: &Arc<Mutex<HashMap<u32, u64>>>,
    active_connections: &Arc<AtomicU16>,
    index: usize,
    kind: crate::dash::DashTrackKind,
    segment_index: Option<u32>,
    resource_url: String,
) {
    let http = http.clone();
    let request = request.clone();
    let control = control.clone();
    let progress_inner = Arc::clone(inner);
    let durable_completed_bytes = Arc::clone(durable_completed_bytes);
    let active_progress = Arc::clone(active_progress);
    let active_connections = Arc::clone(active_connections);
    let staging_path = working_directory.join(format!("{index:08}.part"));
    let completed_path = working_directory.join(format!("{index:08}.fragment"));
    let progress_key = u32::try_from(index).unwrap_or(u32::MAX);
    active_connections.fetch_add(1, Ordering::AcqRel);
    transfers.spawn(async move {
        let mut attempt = 1_u32;
        let outcome = loop {
            let callback_inner = Arc::clone(&progress_inner);
            let callback_progress = Arc::clone(&active_progress);
            let callback_completed = Arc::clone(&durable_completed_bytes);
            let callback_active = Arc::clone(&active_connections);
            let outcome = http
                .download_hls_segment(
                    &request,
                    &resource_url,
                    &staging_path,
                    &control,
                    move |fragment_bytes| {
                        let active_bytes = callback_progress
                            .lock()
                            .map(|mut values| {
                                values.insert(progress_key, fragment_bytes);
                                values.values().copied().sum::<u64>()
                            })
                            .unwrap_or_default();
                        publish_transfer_progress(
                            &callback_inner,
                            task_id,
                            callback_completed
                                .load(Ordering::Acquire)
                                .saturating_add(active_bytes),
                            None,
                            callback_active.load(Ordering::Acquire),
                            None,
                            None,
                        );
                    },
                )
                .await;
            match outcome {
                Err(error) if should_retry_transfer(attempt, &error) => {
                    if let Ok(mut values) = active_progress.lock() {
                        values.insert(progress_key, 0);
                    }
                    if let Err(cleanup_error) = remove_async_file_if_exists(&staging_path).await {
                        break Err(cleanup_error.into());
                    }
                    let delay = transfer_retry_delay(attempt);
                    warn!(
                        event = "download.dash_fragment_retry.scheduled",
                        %correlation_id,
                        ?task_id,
                        fragment_index = index,
                        failed_attempt = attempt,
                        next_attempt = attempt + 1,
                        delay_ms = delay.as_millis(),
                        error_kind = error.kind(),
                        "transient DASH fragment failure scheduled for retry"
                    );
                    attempt += 1;
                    let _ = control.wait_for_stop_or_timeout(delay).await;
                }
                terminal => {
                    if !matches!(terminal, Ok(HlsSegmentDownloadOutcome::Completed { .. }))
                        && let Err(cleanup_error) = remove_async_file_if_exists(&staging_path).await
                    {
                        break Err(cleanup_error.into());
                    }
                    break terminal;
                }
            }
        };
        active_connections.fetch_sub(1, Ordering::AcqRel);
        (
            index,
            kind,
            segment_index,
            staging_path,
            completed_path,
            outcome,
        )
    });
}

pub(crate) fn dash_working_directory(
    request: &DownloadRequest,
    task_id: DownloadTaskId,
) -> PathBuf {
    request
        .save_directory
        .join(format!(".nxtools-dash-{}", task_id.as_uuid()))
}

fn validate_dash_artifact(path: &Path, directory: bool) -> Result<bool, std::io::Error> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if metadata.is_dir() != directory {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "DASH cleanup artifact has an unexpected type",
        ));
    }
    if metadata.permissions().readonly() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "DASH cleanup artifact is read-only",
        ));
    }
    Ok(true)
}

pub(crate) fn remove_dash_owned_artifacts(
    request: &DownloadRequest,
    task_id: DownloadTaskId,
    temporary_path: Option<&Path>,
    correlation_id: uuid::Uuid,
    reason: &'static str,
    started_at: Instant,
) -> Result<(), std::io::Error> {
    let working_directory = dash_working_directory(request, task_id);
    let _ = validate_dash_artifact(&working_directory, true)?;
    if let Some(path) = temporary_path {
        let _ = validate_dash_artifact(path, false)?;
    }
    if let Some(path) = temporary_path {
        match std::fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                warn!(
                    event = "download.dash_cleanup.failed",
                    %correlation_id,
                    ?task_id,
                    reason,
                    artifact_kind = "mux_temporary",
                    native_error_code = error.raw_os_error(),
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "DASH task-owned artifact cleanup failed"
                );
                return Err(error);
            }
        }
    }
    match std::fs::remove_dir_all(&working_directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            warn!(
                event = "download.dash_cleanup.failed",
                %correlation_id,
                ?task_id,
                reason,
                artifact_kind = "fragment_directory",
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "DASH task-owned artifact cleanup failed"
            );
            return Err(error);
        }
    }
    Ok(())
}
