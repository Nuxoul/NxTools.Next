// HLS media transfer runner: playlist planning/compatibility, per-segment
// workers with keyed retry, encryption handling and cancel-artifact cleanup.
use super::*;

pub(crate) async fn run_hls_task(
    inner: Arc<DownloadEngineInner>,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    playlist_url: &str,
    http: &HttpDownloadClient,
) -> Result<(), DownloadEngineError> {
    let started_at = Instant::now();
    let stored_plan = match inner.store.load_hls_transfer_plan(task_id) {
        Ok(plan) => plan,
        Err(error) => {
            info!(
                event = "download.hls_playlist.started",
                %correlation_id,
                ?task_id,
                source = "checkpoint",
                "HLS media playlist resolution started"
            );
            warn!(
                event = "download.hls_playlist.failed",
                %correlation_id,
                ?task_id,
                source = "checkpoint",
                error_kind = "storage",
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS media playlist checkpoint could not be loaded"
            );
            return Err(error.into());
        }
    };
    let playlist_source = if stored_plan.is_some() {
        "network_revalidation"
    } else {
        "network"
    };
    info!(
        event = "download.hls_playlist.started",
        %correlation_id,
        ?task_id,
        source = playlist_source,
        "HLS media playlist resolution started"
    );
    if stored_plan
        .as_ref()
        .is_some_and(|plan| plan.segments.is_empty())
    {
        warn!(
            event = "download.hls_playlist.failed",
            %correlation_id,
            ?task_id,
            source = playlist_source,
            error_kind = "invalid_checkpoint",
            elapsed_ms = started_at.elapsed().as_millis(),
            "persisted HLS media playlist checkpoint is empty"
        );
        return Err(HttpDownloadError::HlsResourceChanged.into());
    }
    let (playlist, server_observation) =
        match http.load_hls_media_playlist(request, playlist_url).await {
            Ok(result) => result,
            Err(error) => {
                warn!(
                    event = "download.hls_playlist.failed",
                    %correlation_id,
                    ?task_id,
                    source = playlist_source,
                    error_kind = error.kind(),
                    retryable = error.is_retryable_transfer(),
                    elapsed_ms = started_at.elapsed().as_millis(),
                    "HLS media playlist resolution failed"
                );
                return Err(error.into());
            }
        };
    let source = if let Some(plan) = stored_plan.as_ref() {
        if !hls_plan_is_compatible(plan, &playlist) {
            warn!(
                event = "download.hls_playlist.failed",
                %correlation_id,
                ?task_id,
                source = "network_revalidation",
                error_kind = "hls_resource_changed",
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS media playlist no longer matches the durable transfer plan"
            );
            return Err(HttpDownloadError::HlsResourceChanged.into());
        }
        if let Err(error) = inner
            .store
            .refresh_hls_transfer_plan_urls(task_id, &playlist)
        {
            warn!(
                event = "download.hls_playlist.failed",
                %correlation_id,
                ?task_id,
                source = "network_revalidation",
                error_kind = "storage",
                elapsed_ms = started_at.elapsed().as_millis(),
                "refreshed HLS media playlist URLs could not be persisted"
            );
            return Err(error.into());
        }
        "network_revalidation"
    } else {
        if let Err(error) = inner.store.save_hls_transfer_plan(task_id, &playlist) {
            warn!(
                event = "download.hls_playlist.failed",
                %correlation_id,
                ?task_id,
                source = "network",
                error_kind = "storage",
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS media transfer plan could not be persisted"
            );
            return Err(error.into());
        }
        info!(
            event = "download.hls_transfer_plan.created",
            %correlation_id,
            ?task_id,
            segment_count = playlist.segments.len(),
            "HLS media transfer plan persisted"
        );
        "network"
    };
    info!(
        event = "download.hls_playlist.completed",
        %correlation_id,
        ?task_id,
        source,
        segment_count = playlist.segments.len(),
        route_kind = server_observation.route_kind.diagnostic_name(),
        peer_present = server_observation.peer_ip.is_some(),
        response_ms = server_observation.response_ms,
        elapsed_ms = started_at.elapsed().as_millis(),
        "HLS media playlist resolved and persisted"
    );
    let encrypted_segment_count = playlist
        .segments
        .iter()
        .filter(|segment| {
            matches!(
                segment.encryption,
                crate::hls::HlsSegmentEncryption::Aes128 { .. }
            )
        })
        .count();
    let key_slot_count = playlist
        .segments
        .iter()
        .filter_map(|segment| match segment.encryption {
            crate::hls::HlsSegmentEncryption::Aes128 { key_slot, .. } => Some(key_slot),
            crate::hls::HlsSegmentEncryption::None => None,
        })
        .collect::<HashSet<_>>()
        .len();
    let explicit_iv_count = playlist
        .segments
        .iter()
        .filter(|segment| {
            matches!(
                segment.encryption,
                crate::hls::HlsSegmentEncryption::Aes128 {
                    explicit_iv: Some(_),
                    ..
                }
            )
        })
        .count();
    if encrypted_segment_count > 0 {
        info!(
            event = "download.hls_encryption.configured",
            %correlation_id,
            ?task_id,
            encrypted_segment_count,
            key_slot_count,
            explicit_iv_count,
            "HLS AES-128 transfer configuration accepted"
        );
    }
    let fallback_file_name = url::Url::parse(&playlist.final_url)
        .ok()
        .and_then(|url| {
            Path::new(url.path())
                .file_stem()
                .map(|stem| format!("{}.ts", stem.to_string_lossy()))
        })
        .unwrap_or_else(|| "video.ts".into());
    let mut probe = DownloadProbe {
        final_url: playlist.final_url.clone(),
        file_name: Some(fallback_file_name),
        total_bytes: None,
        supports_ranges: false,
        content_type: Some("application/vnd.apple.mpegurl".into()),
        validator: crate::ResourceValidator::default(),
        media: Some(crate::DownloadMediaProbe::Hls(crate::HlsProbe {
            is_master: false,
            variants: Vec::new(),
        })),
        resolution: None,
        server_observation: Some(server_observation),
    };
    let (derived_temporary, final_path) =
        HttpDownloadClient::artifact_paths(request, &probe, task_id)?;
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
    let working_directory = hls_working_directory(request, task_id);
    tokio::fs::create_dir_all(&working_directory).await?;
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
    info!(
        event = "download.hls_transfer.started",
        %correlation_id,
        ?task_id,
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        segment_count = playlist.segments.len(),
        requested_connections = request.connection_limit.unwrap_or(1),
        "HLS media transfer started"
    );

    let stored_segments = stored_plan
        .map(|plan| {
            plan.segments
                .into_iter()
                .map(|segment| (segment.media.index, segment))
                .collect::<HashMap<_, _>>()
        })
        .unwrap_or_else(|| {
            playlist
                .segments
                .iter()
                .map(|segment| {
                    (
                        segment.index,
                        crate::storage::StoredHlsSegment {
                            media: segment.clone(),
                            downloaded_bytes: 0,
                            state: DownloadSegmentState::Waiting,
                        },
                    )
                })
                .collect()
        });
    let mut completed_bytes = 0_u64;
    let mut completed_segments = 0_usize;
    let mut adopted_segments = 0_usize;
    let mut reset_segments = 0_usize;
    let mut pending_segments = VecDeque::new();
    for segment in &playlist.segments {
        let checkpoint = stored_segments.get(&segment.index).ok_or_else(|| {
            DownloadEngineError::Runtime(format!(
                "persisted HLS segment {} is missing",
                segment.index
            ))
        })?;
        let completed_path = working_directory.join(format!("{:08}.segment", segment.index));
        match tokio::fs::metadata(&completed_path).await {
            Ok(metadata)
                if metadata.len() > 0
                    && (checkpoint.state != DownloadSegmentState::Completed
                        || checkpoint.downloaded_bytes == metadata.len()) =>
            {
                completed_bytes = completed_bytes
                    .checked_add(metadata.len())
                    .ok_or(HttpDownloadError::LengthOverflow)?;
                completed_segments = completed_segments.saturating_add(1);
                if checkpoint.state != DownloadSegmentState::Completed
                    || checkpoint.downloaded_bytes != metadata.len()
                {
                    inner.store.save_hls_segment_checkpoint(
                        task_id,
                        segment.index,
                        DownloadSegmentState::Completed,
                        metadata.len(),
                    )?;
                    adopted_segments = adopted_segments.saturating_add(1);
                }
                continue;
            }
            Ok(_) => {
                tokio::fs::remove_file(&completed_path).await?;
                reset_segments = reset_segments.saturating_add(1);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        inner.store.save_hls_segment_checkpoint(
            task_id,
            segment.index,
            DownloadSegmentState::Waiting,
            0,
        )?;
        pending_segments.push_back(segment.clone());
    }
    if adopted_segments > 0 {
        // Crash-window adoption has bytes but no matching durable time.
        control.statistics.invalidate();
    }
    if adopted_segments > 0 || reset_segments > 0 {
        info!(
            event = "download.hls_checkpoints.reconciled",
            %correlation_id,
            ?task_id,
            adopted_segments,
            reset_segments,
            segment_count = playlist.segments.len(),
            "HLS segment checkpoints reconciled with durable artifacts"
        );
    }
    let durable_completed_bytes = Arc::new(AtomicU64::new(completed_bytes));
    let active_progress = Arc::new(Mutex::new(HashMap::<u32, u64>::new()));
    let active_connections = Arc::new(AtomicU16::new(0));
    let key_cache = Arc::new(
        playlist
            .segments
            .iter()
            .filter_map(|segment| match segment.encryption {
                crate::hls::HlsSegmentEncryption::Aes128 { key_slot, .. } => Some((
                    key_slot,
                    Arc::new(tokio::sync::Mutex::new(None::<[u8; 16]>)),
                )),
                crate::hls::HlsSegmentEncryption::None => None,
            })
            .collect::<HashMap<_, _>>(),
    );
    let max_connections = usize::from(request.connection_limit.unwrap_or(1).clamp(1, 64));
    let mut transfers = tokio::task::JoinSet::new();
    while transfers.len() < max_connections
        && let Some(segment) = pending_segments.pop_front()
    {
        inner.store.save_hls_segment_checkpoint(
            task_id,
            segment.index,
            DownloadSegmentState::Active,
            0,
        )?;
        spawn_hls_segment_transfer(
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
            &key_cache,
            segment,
        );
    }
    let mut paused = false;
    let mut cancelled = false;
    let mut first_error = None;
    while let Some(joined) = transfers.join_next().await {
        let (segment, staging_path, completed_path, outcome) =
            joined.map_err(|error| DownloadEngineError::Runtime(error.to_string()))?;
        active_progress
            .lock()
            .map_err(|_| DownloadEngineError::LockPoisoned)?
            .remove(&segment.index);
        match outcome {
            Ok(HlsSegmentDownloadOutcome::Completed { downloaded_bytes }) => {
                tokio::fs::rename(&staging_path, &completed_path).await?;
                inner.store.save_hls_segment_checkpoint(
                    task_id,
                    segment.index,
                    DownloadSegmentState::Completed,
                    downloaded_bytes,
                )?;
                completed_bytes = durable_completed_bytes
                    .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                        current.checked_add(downloaded_bytes)
                    })
                    .map(|previous| previous + downloaded_bytes)
                    .map_err(|_| HttpDownloadError::LengthOverflow)?;
                completed_segments = completed_segments.saturating_add(1);
                let mut tasks = inner.lock_tasks()?;
                let state = tasks
                    .get_mut(&task_id)
                    .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
                state.transfer.downloaded_bytes = completed_bytes;
                inner.persist(state)?;
            }
            Ok(HlsSegmentDownloadOutcome::Paused) => {
                inner.store.save_hls_segment_checkpoint(
                    task_id,
                    segment.index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                paused = true;
            }
            Ok(HlsSegmentDownloadOutcome::Cancelled) => {
                inner.store.save_hls_segment_checkpoint(
                    task_id,
                    segment.index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                cancelled = true;
            }
            Err(error) => {
                inner.store.save_hls_segment_checkpoint(
                    task_id,
                    segment.index,
                    DownloadSegmentState::Waiting,
                    0,
                )?;
                first_error.get_or_insert(error);
            }
        }
        if !paused
            && !cancelled
            && first_error.is_none()
            && let Some(next) = pending_segments.pop_front()
        {
            inner.store.save_hls_segment_checkpoint(
                task_id,
                next.index,
                DownloadSegmentState::Active,
                0,
            )?;
            spawn_hls_segment_transfer(
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
                &key_cache,
                next,
            );
        }
    }
    completed_bytes = durable_completed_bytes.load(Ordering::Acquire);
    if let Some(error) = first_error {
        return Err(error.into());
    }
    if !paused && !cancelled {
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        match hls_stop_directive(&control) {
            Some(TransferDirective::Pause) => paused = true,
            Some(TransferDirective::Cancel) => cancelled = true,
            Some(TransferDirective::Running) | None => {
                state.transfer.downloaded_bytes = completed_bytes;
                probe.total_bytes = Some(completed_bytes);
                state.transfer.probe = Some(probe);
                state
                    .task
                    .transition_to(DownloadStatus::Finalizing, now_unix_ms())?;
                inner.persist(state)?;
                inner.emit(DownloadEvent::TaskChanged {
                    task: Box::new(state.task.clone()),
                });
            }
        }
    }
    if !paused && !cancelled {
        let mut total = 0_u64;
        for segment in &playlist.segments {
            total += tokio::fs::metadata(
                working_directory.join(format!("{:08}.segment", segment.index)),
            )
            .await?
            .len();
        }
        let mut copied = 0_u64;
        publish_finishing_progress(
            &inner,
            task_id,
            DownloadFinishingProgress {
                stage: DownloadFinishingStage::Concatenating,
                completed: 0,
                total: Some(total),
            },
            true,
        );
        info!(event = "download.hls_finalize.stage", %correlation_id, ?task_id, stage = "concatenating", "HLS finalization stage changed");
        let mut output = tokio::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&temporary_path)
            .await?;
        let mut buffer = vec![0_u8; 1024 * 1024];
        'merge: for segment in &playlist.segments {
            let completed_path = working_directory.join(format!("{:08}.segment", segment.index));
            let mut input = tokio::fs::File::open(completed_path).await?;
            loop {
                if let Some(directive) = hls_stop_directive(&control) {
                    paused = directive == TransferDirective::Pause;
                    cancelled = directive == TransferDirective::Cancel;
                    break 'merge;
                }
                let count = input.read(&mut buffer).await?;
                if count == 0 {
                    break;
                }
                if let Some(directive) = hls_stop_directive(&control) {
                    paused = directive == TransferDirective::Pause;
                    cancelled = directive == TransferDirective::Cancel;
                    break 'merge;
                }
                output.write_all(&buffer[..count]).await?;
                copied += count as u64;
                publish_finishing_progress(
                    &inner,
                    task_id,
                    DownloadFinishingProgress {
                        stage: DownloadFinishingStage::Concatenating,
                        completed: copied,
                        total: Some(total),
                    },
                    false,
                );
                tokio::task::yield_now().await;
            }
        }
        if !paused && !cancelled {
            output.flush().await?;
            output.sync_all().await?;
            publish_finishing_progress(
                &inner,
                task_id,
                DownloadFinishingProgress {
                    stage: DownloadFinishingStage::Concatenating,
                    completed: copied,
                    total: Some(total),
                },
                true,
            );
        }
        drop(output);
        if let Some(directive) = hls_stop_directive(&control) {
            paused = directive == TransferDirective::Pause;
            cancelled = directive == TransferDirective::Cancel;
        }
    }
    if paused {
        remove_hls_cancel_artifact(
            &temporary_path,
            "merge_artifact",
            false,
            task_id,
            correlation_id,
            Instant::now(),
        )
        .await?;
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.downloaded_bytes = completed_bytes;
        state
            .task
            .transition_to(DownloadStatus::Paused, now_unix_ms())?;
        inner.persist(state)?;
        state.control = None;
        inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(state.task.clone()),
        });
        info!(
            event = "download.hls_transfer.paused",
            %correlation_id,
            ?task_id,
            proxy_mode = proxy_mode_name(http.network_proxy_mode()),
            proxy_source = http.network_proxy_source(),
            completed_segments,
            downloaded_bytes = completed_bytes,
            elapsed_ms = started_at.elapsed().as_millis(),
            "HLS media transfer paused"
        );
        return Ok(());
    }
    if cancelled {
        let _ = validate_hls_artifact(&working_directory, true)?;
        let _ = validate_hls_artifact(&temporary_path, false)?;
        let cleanup_started = Instant::now();
        let staging_cleanup = remove_hls_cancel_artifact(
            &working_directory,
            "segment_staging",
            true,
            task_id,
            correlation_id,
            cleanup_started,
        )
        .await;
        let merge_cleanup = remove_hls_cancel_artifact(
            &temporary_path,
            "merge_artifact",
            false,
            task_id,
            correlation_id,
            cleanup_started,
        )
        .await;
        staging_cleanup?;
        merge_cleanup?;
        inner.store.clear_hls_transfer_plan(task_id)?;
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.downloaded_bytes = completed_bytes;
        state
            .task
            .transition_to(DownloadStatus::Cancelled, now_unix_ms())?;
        inner.persist(state)?;
        state.control = None;
        inner.emit(DownloadEvent::TaskChanged {
            task: Box::new(state.task.clone()),
        });
        info!(
            event = "download.hls_transfer.cancelled",
            %correlation_id,
            ?task_id,
            proxy_mode = proxy_mode_name(http.network_proxy_mode()),
            proxy_source = http.network_proxy_source(),
            downloaded_bytes = completed_bytes,
            elapsed_ms = started_at.elapsed().as_millis(),
            "HLS media transfer cancelled"
        );
        return Ok(());
    }

    {
        let prepared_bytes = tokio::fs::metadata(&temporary_path).await?.len();
        let mut tasks = inner.lock_tasks()?;
        let state = tasks
            .get_mut(&task_id)
            .ok_or(DownloadEngineError::TaskNotFound(task_id))?;
        state.transfer.prepared_size_bytes = Some(prepared_bytes);
        inner.persist(state)?;
    }

    if let Some(directive) = hls_stop_directive(&control) {
        remove_hls_cancel_artifact(
            &temporary_path,
            "merge_artifact",
            false,
            task_id,
            correlation_id,
            Instant::now(),
        )
        .await?;
        let status = match directive {
            TransferDirective::Pause => DownloadStatus::Paused,
            TransferDirective::Cancel => {
                remove_hls_cancel_artifact(
                    &working_directory,
                    "segment_staging",
                    true,
                    task_id,
                    correlation_id,
                    Instant::now(),
                )
                .await?;
                inner.store.clear_hls_transfer_plan(task_id)?;
                DownloadStatus::Cancelled
            }
            TransferDirective::Running => unreachable!("stop helper returned Running"),
        };
        transition_transfer_to_terminal(&inner, task_id, status, completed_bytes)?;
        return Ok(());
    }
    publish_finishing_progress(
        &inner,
        task_id,
        DownloadFinishingProgress {
            stage: DownloadFinishingStage::Publishing,
            completed: 0,
            total: None,
        },
        true,
    );
    info!(event = "download.hls_finalize.stage", %correlation_id, ?task_id, stage = "publishing", "HLS finalization stage changed");
    http.commit_prepared(PreparedDownload {
        temporary_path,
        final_path,
        downloaded_bytes: completed_bytes,
    })
    .await?;
    commit_completed_state(&inner, task_id)?;
    let cleanup_started_at = Instant::now();
    if let Err(error) = tokio::fs::remove_dir_all(&working_directory).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        warn!(
            event = "download.hls_cleanup.failed",
            %correlation_id,
            ?task_id,
            reason = "completed",
            artifact_kind = "segment_directory",
            native_error_code = error.raw_os_error(),
            elapsed_ms = cleanup_started_at.elapsed().as_millis(),
            "HLS task staging cleanup failed"
        );
    }
    info!(
        event = "download.hls_transfer.completed",
        %correlation_id,
        ?task_id,
        proxy_mode = proxy_mode_name(http.network_proxy_mode()),
        proxy_source = http.network_proxy_source(),
        segment_count = playlist.segments.len(),
        downloaded_bytes = completed_bytes,
        elapsed_ms = started_at.elapsed().as_millis(),
        "HLS media transfer completed"
    );
    Ok(())
}

pub(crate) fn hls_plan_is_compatible(
    stored: &crate::storage::StoredHlsTransferPlan,
    current: &crate::hls::HlsMediaPlaylist,
) -> bool {
    stored_hls_plan_is_valid(stored)
        && stored.media_sequence == current.media_sequence
        && stored.segments.len() == current.segments.len()
        && stored
            .segments
            .iter()
            .zip(&current.segments)
            .all(|(stored, current)| {
                stored.media.index == current.index
                    && stored.media.sequence == current.sequence
                    && stored.media.duration_millis == current.duration_millis
                    && hls_encryption_is_compatible(&stored.media.encryption, &current.encryption)
            })
}

pub(crate) fn hls_encryption_is_compatible(
    stored: &crate::hls::HlsSegmentEncryption,
    current: &crate::hls::HlsSegmentEncryption,
) -> bool {
    match (stored, current) {
        (crate::hls::HlsSegmentEncryption::None, crate::hls::HlsSegmentEncryption::None) => true,
        (
            crate::hls::HlsSegmentEncryption::Aes128 {
                key_slot: stored_slot,
                explicit_iv: stored_iv,
                ..
            },
            crate::hls::HlsSegmentEncryption::Aes128 {
                key_slot: current_slot,
                explicit_iv: current_iv,
                ..
            },
        ) => stored_slot == current_slot && stored_iv == current_iv,
        _ => false,
    }
}

pub(crate) fn stored_hls_plan_is_valid(plan: &crate::storage::StoredHlsTransferPlan) -> bool {
    if plan.segments.is_empty()
        || plan.segments.len() > crate::hls::MAX_HLS_MEDIA_SEGMENTS
        || plan.final_url.len() > crate::MAX_HLS_VARIANT_URL_BYTES
        || url::Url::parse(&plan.final_url)
            .ok()
            .is_none_or(|url| !matches!(url.scheme(), "http" | "https"))
    {
        return false;
    }
    let mut aggregate_url_bytes = 0_usize;
    let mut key_slots = HashMap::<u32, (&str, Option<[u8; 16]>)>::new();
    plan.segments
        .iter()
        .enumerate()
        .all(|(expected_index, segment)| {
            aggregate_url_bytes = aggregate_url_bytes.saturating_add(segment.media.url.len());
            let encryption_is_valid = match &segment.media.encryption {
                crate::hls::HlsSegmentEncryption::None => true,
                crate::hls::HlsSegmentEncryption::Aes128 {
                    key_url,
                    key_slot,
                    explicit_iv,
                } => {
                    aggregate_url_bytes = aggregate_url_bytes.saturating_add(key_url.len());
                    if let Some((known_url, known_iv)) = key_slots.get(key_slot) {
                        *known_url == key_url && *known_iv == *explicit_iv
                    } else {
                        key_slots.insert(*key_slot, (key_url, *explicit_iv));
                        key_url.len() <= crate::MAX_HLS_VARIANT_URL_BYTES
                            && url::Url::parse(key_url)
                                .ok()
                                .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
                    }
                }
            };
            encryption_is_valid
                && usize::try_from(segment.media.index).ok() == Some(expected_index)
                && plan
                    .media_sequence
                    .checked_add(u64::from(segment.media.index))
                    == Some(segment.media.sequence)
                && segment.media.url.len() <= crate::MAX_HLS_VARIANT_URL_BYTES
                && aggregate_url_bytes <= crate::hls::MAX_HLS_MEDIA_URL_BYTES
                && url::Url::parse(&segment.media.url)
                    .ok()
                    .is_some_and(|url| matches!(url.scheme(), "http" | "https"))
        })
}

pub(crate) fn hls_stop_directive(control: &DownloadTransferControl) -> Option<TransferDirective> {
    let directive = control.current_directive();
    (directive != TransferDirective::Running).then_some(directive)
}

pub(crate) async fn remove_hls_cancel_artifact(
    path: &Path,
    artifact_kind: &'static str,
    directory: bool,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    started_at: Instant,
) -> Result<(), std::io::Error> {
    let _ = validate_hls_artifact(path, directory)?;
    let result = if directory {
        tokio::fs::remove_dir_all(path).await
    } else {
        tokio::fs::remove_file(path).await
    };
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            warn!(
                event = "download.hls_cleanup.failed",
                %correlation_id,
                ?task_id,
                reason = "cancelled",
                artifact_kind,
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS cancellation cleanup failed"
            );
            Err(error)
        }
    }
}

pub(crate) async fn cleanup_hls_segment_attempt_files(
    staging_path: &Path,
    ciphertext_path: &Path,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    segment_index: u32,
    reason: &'static str,
) -> Result<(), HttpDownloadError> {
    let _ = validate_hls_artifact(staging_path, false)?;
    let _ = validate_hls_artifact(ciphertext_path, false)?;
    let started_at = Instant::now();
    let mut first_error = None;
    for (path, artifact_kind) in [
        (staging_path, "segment_staging"),
        (ciphertext_path, "segment_ciphertext"),
    ] {
        if let Err(error) = tokio::fs::remove_file(path).await
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                event = "download.hls_cleanup.failed",
                %correlation_id,
                ?task_id,
                segment_index,
                reason,
                artifact_kind,
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS segment staging cleanup failed"
            );
            first_error.get_or_insert(error);
        }
    }
    match first_error {
        Some(error) => Err(error.into()),
        None => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn download_hls_segment_attempt<F>(
    http: &HttpDownloadClient,
    request: &DownloadRequest,
    segment: &crate::hls::HlsMediaSegment,
    staging_path: &PathBuf,
    ciphertext_path: &PathBuf,
    control: &DownloadTransferControl,
    key_cache: &HlsKeyCache,
    on_progress: F,
) -> Result<HlsSegmentDownloadOutcome, HttpDownloadError>
where
    F: FnMut(u64),
{
    let crate::hls::HlsSegmentEncryption::Aes128 {
        key_url,
        key_slot,
        explicit_iv,
    } = &segment.encryption
    else {
        return http
            .download_hls_segment(request, &segment.url, staging_path, control, on_progress)
            .await;
    };
    let cache = key_cache
        .get(key_slot)
        .ok_or(HttpDownloadError::HlsResourceChanged)?;
    let key = {
        let mut cached = cache.lock().await;
        if let Some(key) = *cached {
            key
        } else {
            match http.fetch_hls_aes128_key(request, key_url, control).await? {
                HlsKeyFetchOutcome::Loaded(key) => {
                    *cached = Some(key);
                    key
                }
                HlsKeyFetchOutcome::Paused => return Ok(HlsSegmentDownloadOutcome::Paused),
                HlsKeyFetchOutcome::Cancelled => {
                    return Ok(HlsSegmentDownloadOutcome::Cancelled);
                }
            }
        }
    };
    let downloaded = http
        .download_hls_segment(request, &segment.url, ciphertext_path, control, on_progress)
        .await?;
    if !matches!(downloaded, HlsSegmentDownloadOutcome::Completed { .. }) {
        return Ok(downloaded);
    }
    let iv = explicit_iv.unwrap_or_else(|| hls_sequence_iv(segment.sequence));
    let decrypted =
        decrypt_hls_aes128_segment(ciphertext_path, staging_path, key, iv, control).await;
    match tokio::fs::remove_file(ciphertext_path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    decrypted
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn spawn_hls_segment_transfer(
    transfers: &mut tokio::task::JoinSet<HlsSegmentTransferResult>,
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
    key_cache: &HlsKeyCache,
    segment: crate::hls::HlsMediaSegment,
) {
    let http = http.clone();
    let request = request.clone();
    let control = control.clone();
    let progress_inner = Arc::clone(inner);
    let durable_completed_bytes = Arc::clone(durable_completed_bytes);
    let active_progress = Arc::clone(active_progress);
    let active_connections = Arc::clone(active_connections);
    let key_cache = Arc::clone(key_cache);
    let staging_path = working_directory.join(format!("{:08}.part", segment.index));
    let ciphertext_path = working_directory.join(format!("{:08}.encrypted.part", segment.index));
    let completed_path = working_directory.join(format!("{:08}.segment", segment.index));
    active_connections.fetch_add(1, Ordering::AcqRel);
    transfers.spawn(async move {
        let mut attempt = 1_u32;
        let outcome = loop {
            let callback_inner = Arc::clone(&progress_inner);
            let callback_progress = Arc::clone(&active_progress);
            let callback_completed = Arc::clone(&durable_completed_bytes);
            let callback_active = Arc::clone(&active_connections);
            let outcome = download_hls_segment_attempt(
                &http,
                &request,
                &segment,
                &staging_path,
                &ciphertext_path,
                &control,
                &key_cache,
                move |segment_bytes| {
                    let active_bytes = callback_progress
                        .lock()
                        .map(|mut values| {
                            values.insert(segment.index, segment_bytes);
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
                        values.insert(segment.index, 0);
                    }
                    let delay = transfer_retry_delay(attempt);
                    if let Err(cleanup_error) = cleanup_hls_segment_attempt_files(
                        &staging_path,
                        &ciphertext_path,
                        task_id,
                        correlation_id,
                        segment.index,
                        "segment_retry",
                    )
                    .await
                    {
                        break Err(cleanup_error);
                    }
                    warn!(
                        event = "download.hls_segment_retry.scheduled",
                        %correlation_id,
                        ?task_id,
                        segment_index = segment.index,
                        failed_attempt = attempt,
                        next_attempt = attempt + 1,
                        delay_ms = delay.as_millis(),
                        error_kind = error.kind(),
                        "transient HLS segment failure scheduled for retry"
                    );
                    attempt += 1;
                    let _ = control.wait_for_stop_or_timeout(delay).await;
                }
                terminal => {
                    if !matches!(terminal, Ok(HlsSegmentDownloadOutcome::Completed { .. }))
                        && let Err(cleanup_error) = cleanup_hls_segment_attempt_files(
                            &staging_path,
                            &ciphertext_path,
                            task_id,
                            correlation_id,
                            segment.index,
                            "segment_terminal",
                        )
                        .await
                    {
                        break Err(cleanup_error);
                    }
                    break terminal;
                }
            }
        };
        active_connections.fetch_sub(1, Ordering::AcqRel);
        (segment, staging_path, completed_path, outcome)
    });
}

pub(crate) fn hls_working_directory(request: &DownloadRequest, task_id: DownloadTaskId) -> PathBuf {
    request
        .save_directory
        .join(format!(".nxtools-hls-{}", task_id.as_uuid()))
}

fn validate_hls_artifact(path: &Path, directory: bool) -> Result<bool, std::io::Error> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if metadata.is_dir() != directory {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "HLS cleanup artifact has an unexpected type",
        ));
    }
    if metadata.permissions().readonly() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "HLS cleanup artifact is read-only",
        ));
    }
    Ok(true)
}

pub(crate) fn remove_hls_owned_artifacts(
    request: &DownloadRequest,
    task_id: DownloadTaskId,
    temporary_path: Option<&Path>,
    correlation_id: uuid::Uuid,
    reason: &'static str,
    started_at: Instant,
) -> Result<(), std::io::Error> {
    let working_directory = hls_working_directory(request, task_id);
    let _ = validate_hls_artifact(&working_directory, true)?;
    if let Some(path) = temporary_path {
        let _ = validate_hls_artifact(path, false)?;
        remove_hls_artifact(
            path,
            "merge_temporary",
            false,
            task_id,
            correlation_id,
            reason,
            started_at,
        )?;
    }
    remove_hls_artifact(
        &working_directory,
        "segment_directory",
        true,
        task_id,
        correlation_id,
        reason,
        started_at,
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn remove_hls_artifact(
    path: &Path,
    artifact_kind: &'static str,
    directory: bool,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    reason: &'static str,
    started_at: Instant,
) -> Result<(), std::io::Error> {
    let result = if directory {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    match result {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            warn!(
                event = "download.hls_cleanup.failed",
                %correlation_id,
                ?task_id,
                reason,
                artifact_kind,
                native_error_code = error.raw_os_error(),
                elapsed_ms = started_at.elapsed().as_millis(),
                "HLS task-owned artifact cleanup failed"
            );
            Err(error)
        }
    }
}
