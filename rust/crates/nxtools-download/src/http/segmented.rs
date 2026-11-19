use super::*;
use crate::artifact::SegmentWriter;
use crate::engine::segment_planner::{Assignment, Claim, SegmentPlanner, WorkerId};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};
use tokio::io::{AsyncReadExt, AsyncSeekExt};

const WRITE_BATCH: usize = 256 * 1024;
const FLUSH_INTERVAL: Duration = Duration::from_secs(1);
const SIGNIFICANT_DISK_WAIT: Duration = Duration::from_millis(50);
const DISK_PRESSURE_GRACE: Duration = Duration::from_secs(2);

pub(crate) struct SegmentReceiveState {
    planner: Mutex<SegmentPlanner>,
    best_speed: AtomicU64,
    last_disk_pressure: Mutex<Option<Instant>>,
    stopped: watch::Sender<bool>,
}
impl SegmentReceiveState {
    pub(crate) fn new(planner: SegmentPlanner) -> Self {
        let (stopped, _) = watch::channel(false);
        Self {
            planner: Mutex::new(planner),
            best_speed: AtomicU64::new(0),
            last_disk_pressure: Mutex::new(None),
            stopped,
        }
    }
    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, SegmentPlanner>, HttpDownloadError> {
        self.planner
            .lock()
            .map_err(|_| std::io::Error::other("segment planner lock poisoned").into())
    }
    pub(crate) fn stop(&self) {
        self.stopped.send_replace(true);
    }
    pub(crate) fn is_stopped(&self) -> bool {
        *self.stopped.borrow()
    }
    pub(crate) async fn wait_for_stop(&self) {
        let mut receiver = self.stopped.subscribe();
        if *receiver.borrow() {
            return;
        }
        let _ = receiver.changed().await;
    }
    fn note_speed(&self, received: u64, elapsed: Duration) {
        if received >= WRITE_BATCH as u64 && !elapsed.is_zero() {
            self.best_speed.fetch_max(
                (received as f64 / elapsed.as_secs_f64()) as u64,
                Ordering::Relaxed,
            );
        }
    }
    fn note_disk_wait(&self, elapsed: Duration) {
        if elapsed >= SIGNIFICANT_DISK_WAIT
            && let Ok(mut last) = self.last_disk_pressure.lock()
        {
            *last = Some(Instant::now());
        }
    }
    fn should_replace(
        &self,
        worker: WorkerId,
        at: u64,
        received: u64,
        elapsed: Duration,
        control: &DownloadSegmentTransferControl,
    ) -> Result<bool, HttpDownloadError> {
        if elapsed < Duration::from_secs(2)
            || self
                .last_disk_pressure
                .lock()
                .ok()
                .is_some_and(|last| last.is_some_and(|at| at.elapsed() < DISK_PRESSURE_GRACE))
            || control
                .task
                .global_rate_limiter
                .bytes_per_second()
                .is_some()
            || control.task.task_rate_limiter.bytes_per_second().is_some()
        {
            return Ok(false);
        }
        let speed = (received as f64 / elapsed.as_secs_f64()) as u64;
        let remaining = self.lock()?.room(worker, at).unwrap_or_default();
        Ok(
            speed.saturating_mul(4) < self.best_speed.load(Ordering::Relaxed)
                && remaining as f64 / speed.max(1) as f64 > 2.0,
        )
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum SegmentReceiveOutcome {
    Completed { end: u64 },
    Lost,
    Stopped,
    Paused,
    Cancelled,
    Rebalanced,
}

struct ClaimedWriter<'a> {
    writer: SegmentWriter,
    state: &'a SegmentReceiveState,
    control: &'a DownloadSegmentTransferControl,
    worker: WorkerId,
    index: u32,
    start: u64,
    position: u64,
    buffer: Vec<u8>,
    disk_wait: Duration,
}
impl ClaimedWriter<'_> {
    async fn flush<F>(&mut self, on_progress: &mut F) -> Result<Option<u64>, HttpDownloadError>
    where
        F: FnMut(DownloadSegmentTransferProgress),
    {
        let mut batch = std::mem::take(&mut self.buffer);
        let wrote = !batch.is_empty();
        if wrote {
            let started = Instant::now();
            self.writer.write_all(&batch).await?;
            // Commit follows the physical write; an IO failure never replays this batch.
            self.writer.flush().await?;
            let elapsed = started.elapsed();
            self.disk_wait += elapsed;
            self.state.note_disk_wait(elapsed);
            self.control.task.statistics.add_bytes(batch.len() as u64);
            self.position += batch.len() as u64;
        }
        batch.clear();
        self.buffer = batch;
        let (end, length) = {
            let mut planner = self.state.lock()?;
            let end = planner.commit(self.worker, self.position);
            let length = planner
                .segments()
                .iter()
                .find(|segment| segment.index == self.index)
                .map(|segment| segment.end - segment.start)
                .unwrap_or_default();
            (end, length)
        };
        if wrote {
            on_progress(DownloadSegmentTransferProgress {
                segment_index: self.index,
                downloaded_bytes: self.position - self.start,
                segment_bytes: length,
            });
        }
        Ok(end)
    }
    async fn sync(&mut self) -> Result<(), HttpDownloadError> {
        self.writer.sync().await?;
        self.state.lock()?.mark_synced(self.index, self.position);
        Ok(())
    }
}

impl HttpDownloadClient {
    pub(crate) async fn download_planned_segment<F>(
        &self,
        transfer: DownloadSegmentTransferRequest<'_>,
        assignment: Assignment,
        state: &SegmentReceiveState,
        worker: WorkerId,
        mut on_progress: F,
    ) -> Result<SegmentReceiveOutcome, HttpDownloadError>
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
        info!(event = "download.segment_transfer.started", correlation_id = %context.correlation_id,
            task_id = ?context.task_id, segment_index = assignment.index, worker,
            resumed_bytes = assignment.written, conditional_resume = use_if_range,
            "segment response started");
        let result = async {
            request.validate()?;
            if state.is_stopped() { return Ok(SegmentReceiveOutcome::Stopped); }
            match control.task.current_directive() {
                TransferDirective::Running => {},
                TransferDirective::Pause => return Ok(SegmentReceiveOutcome::Paused),
                TransferDirective::Cancel => return Ok(SegmentReceiveOutcome::Cancelled),
            }
            if *control.rebalance.borrow() { return Ok(SegmentReceiveOutcome::Rebalanced); }
            let _active_transfer = control.task.statistics.worker();
            let total = probe.total_bytes.ok_or(HttpDownloadError::SegmentedTransferNeedsKnownLength)?;
            let overlap = assignment.written.min(64);
            let requested_start = assignment.from - overlap;
            let mut directives = control.subscribe_task();
            let mut rebalances = control.subscribe_rebalance();
            let response_request = self.send_range(request, &probe.final_url, requested_start,
                assignment.to - 1, &probe.validator, use_if_range);
            tokio::pin!(response_request);
            let mut response = tokio::select! {
                biased;
                _ = state.wait_for_stop() => return Ok(SegmentReceiveOutcome::Stopped),
                _ = directives.changed() => return Ok(match control.task.current_directive() {
                    TransferDirective::Cancel => SegmentReceiveOutcome::Cancelled,
                    _ => SegmentReceiveOutcome::Paused,
                }),
                _ = rebalances.changed() => return Ok(SegmentReceiveOutcome::Rebalanced),
                result = &mut response_request => result?,
                _ = tokio::time::sleep(self.stall_timeout) => return Err(HttpDownloadError::Stalled { timeout_ms: duration_millis(self.stall_timeout) }),
            };
            let mut content_guard = web_page::ContentGuard::new(request, probe, &response, requested_start == 0)?;
            let mut validation_pending = content_guard.checking();
            if !validation_pending {
                validate_segment_response(&response, requested_start, assignment.to - 1, total, &probe.validator, use_if_range)?;
            }
            let mut disk_overlap = vec![0_u8; overlap as usize];
            if overlap > 0 {
                let mut file = tokio::fs::File::open(artifact.path()).await?;
                file.seek(std::io::SeekFrom::Start(requested_start)).await?;
                file.read_exact(&mut disk_overlap).await?;
            }
            let mut sink = ClaimedWriter {
                writer: artifact.open_segment(segment).await?, state, control, worker, index: assignment.index,
                start: segment.start_byte(), position: assignment.from, buffer: Vec::with_capacity(WRITE_BATCH),
                disk_wait: Duration::ZERO,
            };
            let mut verified = 0_usize;
            let mut received = 0_u64;
            let response_started = Instant::now();
            let mut checked_at = response_started;
            let mut flushed_at = response_started;
            let mut limiter_wait = Duration::ZERO;
            let mut flush_tick = tokio::time::interval(FLUSH_INTERVAL);
            flush_tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            flush_tick.tick().await;
            let mut last_body_progress = Instant::now();
            let receive_result = async {
                loop {
                    if state.is_stopped() { return Ok(SegmentReceiveOutcome::Stopped); }
                    match control.task.current_directive() {
                        TransferDirective::Running => {},
                        TransferDirective::Pause => return Ok(SegmentReceiveOutcome::Paused),
                        TransferDirective::Cancel => return Ok(SegmentReceiveOutcome::Cancelled),
                    }
                    tokio::select! {
                        biased;
                        _ = state.wait_for_stop() => return Ok(SegmentReceiveOutcome::Stopped),
                        _ = directives.changed() => continue,
                        _ = rebalances.changed() => return Ok(SegmentReceiveOutcome::Rebalanced),
                        _ = flush_tick.tick() => {
                            let previous_disk_wait = sink.disk_wait;
                            if let Some(end) = sink.flush(&mut on_progress).await? {
                                return Ok(SegmentReceiveOutcome::Completed { end });
                            }
                            last_body_progress += sink.disk_wait - previous_disk_wait;
                            flushed_at = Instant::now();
                        }
                        _ = tokio::time::sleep_until(tokio::time::Instant::from_std(last_body_progress + self.stall_timeout)) => {
                            return Err(HttpDownloadError::Stalled { timeout_ms: duration_millis(self.stall_timeout) });
                        }
                        chunk = response.chunk() => {
                            let chunk = chunk.map_err(|error| self.client_error(error))?;
                            let ended = chunk.is_none();
                            let bytes = chunk.as_deref().unwrap_or_default();
                            let (prefix, consumed) = content_guard.accept(bytes, ended)?;
                            if validation_pending && !content_guard.checking() {
                                validate_segment_response(&response, requested_start, assignment.to - 1, total, &probe.validator, use_if_range)?;
                                validation_pending = false;
                            }
                            last_body_progress = Instant::now();
                            for mut bytes in [prefix.as_slice(), &bytes[consumed..]] {
                            if bytes.is_empty() { continue; }
                            if verified < disk_overlap.len() {
                                let count = bytes.len().min(disk_overlap.len() - verified);
                                if bytes[..count] != disk_overlap[verified..verified + count] {
                                    return Err(HttpDownloadError::FileChanged);
                                }
                                verified += count;
                                bytes = &bytes[count..];
                                if bytes.is_empty() { continue; }
                            }
                            let limiter_started = Instant::now();
                            let bandwidth = control.acquire_bandwidth(bytes.len() as u64);
                            tokio::pin!(bandwidth);
                            tokio::select! {
                                biased;
                                _ = state.wait_for_stop() => return Ok(SegmentReceiveOutcome::Stopped),
                                _ = directives.changed() => continue,
                                _ = rebalances.changed() => return Ok(SegmentReceiveOutcome::Rebalanced),
                                () = &mut bandwidth => {},
                            }
                            let waited = limiter_started.elapsed();
                            limiter_wait += waited;
                            last_body_progress += waited;
                            let at = sink.position + sink.buffer.len() as u64;
                            let (count, full) = {
                                let mut planner = state.lock()?;
                                let Claim::Granted(count) = planner.claim(worker, at, bytes.len() as u64) else {
                                    return Ok(SegmentReceiveOutcome::Lost);
                                };
                                (count as usize, planner.room(worker, at + count) == Some(0))
                            };
                            sink.buffer.extend_from_slice(&bytes[..count]);
                            if received == 0 && count > 0 {
                                on_progress(DownloadSegmentTransferProgress {
                                    segment_index: assignment.index, downloaded_bytes: sink.position - sink.start,
                                    segment_bytes: segment.len(),
                                });
                            }
                            received += count as u64;
                            if checked_at.elapsed() >= Duration::from_millis(500) {
                                checked_at = Instant::now();
                                state.note_speed(received, response_started.elapsed());
                                if state.should_replace(worker, at + count as u64, received,
                                    response_started.elapsed().saturating_sub(sink.disk_wait + limiter_wait), control)? {
                                    return Err(HttpDownloadError::SlowConnection);
                                }
                            }
                            if sink.buffer.len() >= WRITE_BATCH || full || flushed_at.elapsed() >= FLUSH_INTERVAL {
                                let previous_disk_wait = sink.disk_wait;
                                if let Some(end) = sink.flush(&mut on_progress).await? {
                                    return Ok(SegmentReceiveOutcome::Completed { end });
                                }
                                last_body_progress += sink.disk_wait - previous_disk_wait;
                                flushed_at = Instant::now();
                            }
                            }
                            if ended { return Err(HttpDownloadError::ShortBody); }
                        }
                    }
                }
            }.await;
            let completed = sink.flush(&mut on_progress).await?;
            sink.sync().await?;
            state.note_speed(received, response_started.elapsed());
            // A truncated response is irrelevant once all bytes still owned by this worker are in.
            if let Some(end) = completed
                && receive_result.as_ref().is_err_and(|error| !error.is_fatal_segment())
            {
                return Ok(SegmentReceiveOutcome::Completed { end });
            }
            receive_result
        }.await;
        match &result {
            Ok(outcome) => {
                info!(event = "download.segment_transfer.completed", correlation_id = %context.correlation_id,
                task_id = ?context.task_id, segment_index = assignment.index, worker, outcome = ?outcome,
                elapsed_ms = started.elapsed().as_millis(), "segment response retired after writer sync")
            }
            Err(error) => {
                warn!(event = "download.segment_transfer.failed", correlation_id = %context.correlation_id,
                task_id = ?context.task_id, segment_index = assignment.index, worker, error_kind = error.kind(),
                response_status = match error {
                    HttpDownloadError::Status(status) | HttpDownloadError::Throttled { status, .. }
                        | HttpDownloadError::RangeNotHonored { status } => Some(*status),
                    _ => None,
                },
                native_error_code = error.io_error().or_else(|| error.native_connection_refusal())
                    .and_then(std::io::Error::raw_os_error),
                elapsed_ms = started.elapsed().as_millis(), "segment response failed after writer retirement")
            }
        }
        result
    }
}

#[cfg(test)]
#[path = "segmented_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "disk_pressure_tests.rs"]
mod disk_pressure_tests;
