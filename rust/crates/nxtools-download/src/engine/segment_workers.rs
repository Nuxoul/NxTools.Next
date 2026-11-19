use super::segment_planner::{Assignment, SegmentPlanner, WorkerId};
use super::*;
use crate::http::segmented::{SegmentReceiveOutcome, SegmentReceiveState};

type SegmentProgress = Arc<Mutex<HashMap<u32, u64>>>;
type SegmentRanges = Arc<Mutex<HashMap<u32, (u64, u64)>>>;

struct SegmentRun {
    inner: Arc<DownloadEngineInner>,
    http: HttpDownloadClient,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: DownloadRequest,
    probe: DownloadProbe,
    control: DownloadTransferControl,
    artifact: SegmentedArtifact,
    state: SegmentReceiveState,
    progress: SegmentProgress,
    ranges: SegmentRanges,
    active: AtomicU16,
    ceiling: connection_ceiling::Ceiling,
    use_if_range: bool,
    // Serializes plan publication, not byte claims. The planner lock is never held across IO.
    persistence: tokio::sync::Mutex<()>,
}
impl SegmentRun {
    fn publish_progress(&self) -> Result<(), DownloadEngineError> {
        let aggregate = {
            let planner = self.state.lock()?;
            let mut progress = self
                .progress
                .lock()
                .map_err(|_| DownloadEngineError::LockPoisoned)?;
            let mut ranges = self
                .ranges
                .lock()
                .map_err(|_| DownloadEngineError::LockPoisoned)?;
            progress.clear();
            ranges.clear();
            for (index, start, length, written) in planner.progress_segments() {
                progress.insert(index, written);
                ranges.insert(index, (start, length));
            }
            progress.values().copied().sum()
        };
        publish_transfer_progress(
            &self.inner,
            self.task_id,
            aggregate,
            self.probe.total_bytes,
            self.active.load(Ordering::Acquire),
            Some(&self.progress),
            Some(&self.ranges),
        );
        Ok(())
    }
    fn save_plan(&self) -> Result<(), DownloadEngineError> {
        let segments = self.state.lock()?.checkpoints()?;
        let plan = TransferPlan::new(
            TransferProtocol::Http,
            self.probe.final_url.clone(),
            self.probe.total_bytes,
            true,
            self.probe.validator.clone(),
            segments,
        )?;
        self.inner.persist_transfer_plan_change(self.task_id, &plan)
    }
    async fn checkpoint(&self) -> Result<(), DownloadEngineError> {
        let _publication = self.persistence.lock().await;
        let positions = self
            .state
            .lock()?
            .segments()
            .iter()
            .map(|segment| (segment.index, segment.cursor))
            .collect::<Vec<_>>();
        // Every captured cursor follows a completed writer flush. Later writes are not checkpointed.
        self.artifact
            .sync_data()
            .await
            .map_err(HttpDownloadError::from)?;
        {
            let mut planner = self.state.lock()?;
            for (index, position) in positions {
                planner.mark_synced(index, position);
            }
        }
        self.save_plan()?;
        info!(event = "download.segment_checkpoint.persisted", correlation_id = %self.correlation_id,
            task_id = ?self.task_id, segment_count = self.state.lock()?.segments().len(),
            "synced segmented plan checkpoint persisted");
        Ok(())
    }
    async fn assign(
        &self,
        worker: WorkerId,
        after: Option<u64>,
    ) -> Result<Option<Assignment>, DownloadEngineError> {
        let _publication = self.persistence.lock().await;
        if self.state.is_stopped() || self.control.current_directive() != TransferDirective::Running
        {
            return Ok(None);
        }
        let assignment = self.state.lock()?.assign(worker, after);
        if let Some(assignment) = assignment {
            self.publish_progress()?;
            if assignment.split {
                self.save_plan()?;
                info!(event = "download.segment_split.persisted", correlation_id = %self.correlation_id,
                    task_id = ?self.task_id, worker, segment_index = assignment.index,
                    segment_count = self.state.lock()?.segments().len(),
                    "unclaimed tail persisted before its new request");
            }
            if let Some(previous) = assignment.preempted {
                info!(event = "download.segment_assignment.reassigned", correlation_id = %self.correlation_id,
                    task_id = ?self.task_id, worker, previous_worker = previous, segment_index = assignment.index,
                    "pending adjacent response assignment transferred");
            }
        }
        Ok(assignment)
    }
    async fn worker(&self, worker: WorkerId) -> Result<(), DownloadEngineError> {
        let mut after = None;
        let mut failures = 0_u32;
        let mut received = 0_u64;
        let mut at_last_failure = 0_u64;
        let started = Instant::now();
        loop {
            let Some(assignment) = self.assign(worker, after.take()).await? else {
                return Ok(());
            };
            let segment = DownloadSegment::new(
                assignment.index,
                assignment.from - assignment.written,
                assignment.to - 1,
                assignment.written,
                DownloadSegmentState::Active,
            )?;
            let control = DownloadSegmentTransferControl::new(self.control.clone());
            let mut transfer = crate::DownloadSegmentTransferRequest::new(
                DownloadTransferContext::new(self.task_id, self.correlation_id),
                &self.request,
                &self.probe,
                &segment,
                &self.artifact,
                &control,
            );
            if self.use_if_range {
                transfer = transfer.with_resume_validator();
            }
            let mut receiving = connection_ceiling::ReceivingLease::new(Arc::clone(&self.ceiling));
            let mut last_written = assignment.written;
            let mut progress_failure = None;
            self.active.fetch_add(1, Ordering::AcqRel);
            let result = self
                .http
                .download_planned_segment(transfer, assignment, &self.state, worker, |progress| {
                    // The first callback follows a payload claim; allocation/headers alone never reach it.
                    if progress.downloaded_bytes == assignment.written {
                        let receiving_width = match self.state.lock() {
                            Ok(planner) => {
                                u16::try_from(planner.receiving_workers()).unwrap_or(u16::MAX)
                            }
                            Err(error) => {
                                progress_failure = Some(error.into());
                                self.state.stop();
                                return;
                            }
                        };
                        receiving.progress(receiving_width);
                    }
                    received += progress.downloaded_bytes.saturating_sub(last_written);
                    last_written = progress.downloaded_bytes;
                    if let Err(error) = self.publish_progress() {
                        progress_failure = Some(error);
                        self.state.stop();
                    }
                })
                .await;
            self.active.fetch_sub(1, Ordering::AcqRel);
            drop(receiving);
            self.state.lock()?.release(worker);
            if let Some(error) = progress_failure {
                return Err(error);
            }
            match result {
                Ok(SegmentReceiveOutcome::Completed { end }) => {
                    failures = 0;
                    after = Some(end);
                }
                Ok(SegmentReceiveOutcome::Lost) => {}
                Ok(
                    SegmentReceiveOutcome::Stopped
                    | SegmentReceiveOutcome::Paused
                    | SegmentReceiveOutcome::Cancelled
                    | SegmentReceiveOutcome::Rebalanced,
                ) => return Ok(()),
                Err(error) => {
                    if error.is_fatal_segment() {
                        return Err(error.into());
                    }
                    if matches!(error, HttpDownloadError::SlowConnection) {
                        info!(event = "download.segment_connection.slow_replaced", correlation_id = %self.correlation_id,
                            task_id = ?self.task_id, worker, segment_index = assignment.index,
                            elapsed_ms = started.elapsed().as_millis(), "slow response replaced without consuming retry budget");
                        continue;
                    }
                    let (working, receiving) = {
                        let planner = self.state.lock()?;
                        (planner.active_workers(), planner.receiving_workers())
                    };
                    if let Some(width) = connection_ceiling::observe_run_refusal(
                        &self.ceiling,
                        &error,
                        working,
                        receiving,
                    ) {
                        let status = match &error {
                            HttpDownloadError::Throttled { status, .. }
                            | HttpDownloadError::Status(status) => Some(i64::from(*status)),
                            _ => error
                                .native_connection_refusal()
                                .and_then(std::io::Error::raw_os_error)
                                .map(i64::from),
                        };
                        info!(event = "download.connection_ceiling.learned", correlation_id = %self.correlation_id,
                            task_id = ?self.task_id, width, status, ttl_seconds = 600,
                            reason = "refused_extra_connection_with_working_peers",
                            elapsed_ms = started.elapsed().as_millis(), "host receiving width learned within captured network scope");
                    }
                    if error.is_refused_connection()
                        && working > 0
                        && working
                            >= usize::from(connection_ceiling::limit(&self.ceiling, u16::MAX))
                    {
                        info!(event = "download.segment_connection.retired", correlation_id = %self.correlation_id,
                            task_id = ?self.task_id, worker, reason = "learned_connection_ceiling",
                            "refused extra connection retired while peers continue");
                        return Ok(());
                    }
                    if error.is_dead_link() {
                        return if working > 0 {
                            Ok(())
                        } else {
                            Err(error.into())
                        };
                    }
                    if received > at_last_failure {
                        failures = 0;
                    }
                    at_last_failure = received;
                    failures += 1;
                    if failures > 5 {
                        info!(event = "download.segment_connection.retired", correlation_id = %self.correlation_id,
                            task_id = ?self.task_id, worker, failures, working_peers = working,
                            reason = "retry_budget_exhausted", "connection stopped after bounded no-progress failures");
                        return if working > 0 {
                            Ok(())
                        } else {
                            Err(error.into())
                        };
                    }
                    let delay = segment_retry_wait(failures, &error);
                    warn!(event = "download.segment_retry.scheduled", correlation_id = %self.correlation_id,
                        task_id = ?self.task_id, worker, segment_index = assignment.index,
                        failures, failed_attempt = failures, delay_ms = delay.as_millis(), error_kind = error.kind(),
                        "released segment scheduled for connection retry");
                    tokio::select! {
                        biased;
                        _ = self.state.wait_for_stop() => return Ok(()),
                        _ = self.control.wait_for_stop_or_timeout(delay) => {},
                    }
                }
            }
        }
    }
}

pub(super) fn segment_retry_wait(failures: u32, error: &HttpDownloadError) -> Duration {
    match error {
        HttpDownloadError::Throttled {
            retry_after: Some(after),
            ..
        } => (*after).min(Duration::from_secs(300)),
        _ if failures <= 1 => Duration::ZERO,
        _ => Duration::from_secs((1_u64 << (failures - 2).min(10)).min(30)),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run(
    inner: &Arc<DownloadEngineInner>,
    http: &HttpDownloadClient,
    task_id: DownloadTaskId,
    correlation_id: uuid::Uuid,
    request: &DownloadRequest,
    probe: &DownloadProbe,
    control: &DownloadTransferControl,
    artifact: &SegmentedArtifact,
    segments: &[DownloadSegment],
    progress: &SegmentProgress,
    ranges: &SegmentRanges,
    use_if_range: bool,
) -> Result<Option<HttpDownloadError>, DownloadEngineError> {
    let mut next_start = 0;
    for segment in segments {
        if segment.start_byte() != next_start {
            return Err(DownloadEngineError::IncompleteSegmentLayout(task_id));
        }
        next_start = segment.end_byte() + 1;
    }
    if Some(next_start) != probe.total_bytes {
        return Err(DownloadEngineError::IncompleteSegmentLayout(task_id));
    }
    let preempt_after =
        Duration::from_millis(probe.server_observation.as_ref().map_or(20, |observation| {
            observation.response_ms.saturating_mul(2).max(20)
        }));
    let run = Arc::new(SegmentRun {
        inner: Arc::clone(inner),
        http: http.clone(),
        task_id,
        correlation_id,
        request: request.clone(),
        probe: probe.clone(),
        control: control.clone(),
        artifact: artifact.clone(),
        state: SegmentReceiveState::new(SegmentPlanner::new(segments, preempt_after)),
        progress: Arc::clone(progress),
        ranges: Arc::clone(ranges),
        active: AtomicU16::new(0),
        ceiling: inner.connection_ceiling(http, &probe.final_url),
        use_if_range,
        persistence: tokio::sync::Mutex::new(()),
    });
    let requested = request.connection_limit.unwrap_or(1).clamp(1, 64);
    let useful = u16::try_from(next_start.div_ceil(MIN_INITIAL_SEGMENT_BYTES))
        .unwrap_or(u16::MAX)
        .max(1);
    let width = connection_ceiling::limit(&run.ceiling, requested).min(useful);
    info!(event = "download.segment_workers.started", %correlation_id, ?task_id, width,
        "connection workers admitted for segmented transfer");
    let mut transfers = tokio::task::JoinSet::new();
    let mut next_worker = 0_u32;
    for _ in 0..width {
        spawn(&mut transfers, &run, next_worker);
        next_worker += 1;
    }
    let mut checkpoint_tick = tokio::time::interval(Duration::from_secs(2));
    checkpoint_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    checkpoint_tick.tick().await;
    let mut directives = control.subscribe();
    let mut first_fatal = None;
    let mut last_error = None;
    while !transfers.is_empty() {
        tokio::select! {
            biased;
            _ = directives.changed(), if !run.state.is_stopped() => { run.state.stop(); },
            _ = checkpoint_tick.tick(), if !run.state.is_stopped() => {
                if let Err(error) = run.checkpoint().await {
                    first_fatal = Some(error);
                    run.state.stop();
                }
            },
            joined = transfers.join_next() => {
                match joined {
                    Some(Ok(Err(DownloadEngineError::Http(error)))) if !error.is_fatal_segment() => { last_error = Some(error); },
                    Some(Ok(Err(error))) => { first_fatal.get_or_insert(error); run.state.stop(); },
                    Some(Err(error)) => { first_fatal.get_or_insert(DownloadEngineError::Runtime(error.to_string())); run.state.stop(); },
                    _ => {},
                }
                let complete = match run.state.lock() {
                    Ok(planner) => planner.is_complete(),
                    Err(error) => { first_fatal.get_or_insert(error.into()); run.state.stop(); false },
                };
                if complete { run.state.stop(); }
                if transfers.is_empty() && !complete && !run.state.is_stopped()
                    && control.current_directive() == TransferDirective::Running && last_error.is_none() {
                    spawn(&mut transfers, &run, next_worker);
                    next_worker += 1;
                }
            }
        }
    }
    // No future/physical writer remains when this final checkpoint and publication are attempted.
    run.checkpoint().await?;
    run.publish_progress()?;
    if let Some(error) = first_fatal {
        return match error {
            DownloadEngineError::Http(error) => Ok(Some(error)),
            other => Err(other),
        };
    }
    if run.state.lock()?.is_complete() {
        return Ok(None);
    }
    Ok(last_error)
}
fn spawn(
    transfers: &mut tokio::task::JoinSet<Result<(), DownloadEngineError>>,
    run: &Arc<SegmentRun>,
    worker: WorkerId,
) {
    let run = Arc::clone(run);
    transfers.spawn(async move { run.worker(worker).await });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn segment_retry_wait_uses_immediate_first_retry_exponential_delay_and_server_cap() {
        let error = HttpDownloadError::Status(500);
        let waits: Vec<_> = (1..=8)
            .map(|failures| segment_retry_wait(failures, &error).as_secs())
            .collect();
        assert_eq!(waits, [0, 1, 2, 4, 8, 16, 30, 30]);
        assert_eq!(
            segment_retry_wait(
                1,
                &HttpDownloadError::Throttled {
                    status: 503,
                    retry_after: Some(Duration::from_secs(900)),
                }
            ),
            Duration::from_secs(300)
        );
        assert_eq!(
            segment_retry_wait(
                2,
                &HttpDownloadError::Throttled {
                    status: 429,
                    retry_after: Some(Duration::from_secs(7)),
                }
            ),
            Duration::from_secs(7)
        );
    }
}
