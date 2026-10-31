use crate::{ArchiveError, Result};
use std::{
    sync::{Arc, Condvar, Mutex},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchiveProgress {
    pub bytes: u64,
    pub files: u64,
    pub total_bytes: Option<u64>,
}
#[derive(Clone, Copy)]
struct ExtractionProgress {
    total_bytes: Option<u64>,
    files_total: u64,
    member_done: Option<u64>,
    member_total: Option<u64>,
}

pub(crate) struct ProgressSnapshot {
    pub progress: ArchiveProgress,
    pub phase: &'static str,
    pub current_member: Option<String>,
    pub files_total: Option<u64>,
    pub current_bytes_done: Option<u64>,
    pub current_bytes_total: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CancellationBoundary {
    Checkpoint,
    PausedCheckpoint,
    NativeReply,
    MetadataCredit,
}
/// Fixed cancellation facts; all elapsed values share this control's monotonic origin.
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub(crate) struct CancellationObservation {
    received_us: Option<u64>,
    observed_us: Option<u64>,
    boundary: Option<CancellationBoundary>,
}
struct State {
    cancelled: bool,
    paused: bool,
    latest: Option<ArchiveProgress>,
    last_publish: Option<Instant>,
    progress_phase: &'static str,
    current_member: Option<String>,
    extraction: Option<ExtractionProgress>,
    cancel_hooks: Vec<std::sync::Weak<dyn Fn() + Send + Sync>>,
    source_deletion: crate::source_deletion::Evidence,
    cancellation: CancellationObservation,
    #[cfg(test)]
    pause_waiters: usize,
}
struct Shared {
    state: Mutex<State>,
    wake: Condvar,
    started: Instant,
}

/// Cloneable cooperative control. No worker/thread spawn, callback, or unbounded progress queue.
#[derive(Clone)]
pub struct OperationControl {
    id: Uuid,
    shared: Arc<Shared>,
}
impl Default for OperationControl {
    fn default() -> Self {
        Self::with_operation_id(Uuid::new_v4())
    }
}
impl OperationControl {
    pub fn with_operation_id(id: Uuid) -> Self {
        Self {
            id,
            shared: Arc::new(Shared {
                state: Mutex::new(State {
                    cancelled: false,
                    paused: false,
                    latest: None,
                    current_member: None,
                    extraction: None,
                    last_publish: None,
                    progress_phase: "running",
                    cancel_hooks: Vec::new(),
                    source_deletion: Default::default(),
                    cancellation: CancellationObservation {
                        received_us: None,
                        observed_us: None,
                        boundary: None,
                    },
                    #[cfg(test)]
                    pause_waiters: 0,
                }),
                wake: Condvar::new(),
                started: Instant::now(),
            }),
        }
    }
    pub fn operation_id(&self) -> Uuid {
        self.id
    }
    /// An actual retry has a fresh identity but retains the logical operation's cancel/lifetime.
    pub(crate) fn attempt(&self) -> Self {
        Self {
            id: Uuid::new_v4(),
            shared: self.shared.clone(),
        }
    }
    /// Consumes lifecycle evidence once. This alone never authorizes a native path operation.
    pub fn source_delete_eligibility(
        &self,
        consent: bool,
    ) -> crate::source_deletion::SourceDeleteEligibility {
        let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
        let cancelled = state.cancelled;
        state.source_deletion.take(self.id, consent, cancelled)
    }
    pub(crate) fn source_delete_begin(&self) -> crate::source_deletion::Lifecycle {
        self.shared
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .source_deletion
            .begin()
    }
    pub(crate) fn source_delete_finish(
        &self,
        lifecycle: &crate::source_deletion::Lifecycle,
        reason: &'static str,
        sources: Vec<Uuid>,
    ) {
        self.shared
            .state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .source_deletion
            .finish(lifecycle, reason, sources);
    }
    fn elapsed_us(&self) -> u64 {
        u64::try_from(self.shared.started.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
    pub(crate) fn receive_cancel(&self) {
        if let Ok(mut state) = self.shared.state.lock() {
            state
                .cancellation
                .received_us
                .get_or_insert_with(|| self.elapsed_us());
        }
        self.cancel();
    }
    pub(crate) fn observe_cancelled(&self, boundary: CancellationBoundary) {
        if let Ok(mut state) = self.shared.state.lock() {
            self.record_observed(&mut state, boundary);
        }
    }
    fn record_observed(&self, state: &mut State, boundary: CancellationBoundary) {
        if state.cancelled && state.cancellation.observed_us.is_none() {
            state.cancellation.observed_us = Some(self.elapsed_us());
            state.cancellation.boundary = Some(boundary);
        }
    }
    pub(crate) fn cancellation_observation(&self) -> Option<CancellationObservation> {
        self.shared
            .state
            .lock()
            .ok()
            .and_then(|state| state.cancelled.then_some(state.cancellation))
    }
    pub fn cancel(&self) {
        let hooks = {
            let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
            state.cancelled = true;
            self.shared.wake.notify_all();
            state
                .cancel_hooks
                .iter()
                .filter_map(std::sync::Weak::upgrade)
                .collect::<Vec<_>>()
        };
        for hook in hooks {
            hook();
        }
    }
    pub fn set_paused(&self, paused: bool) {
        let mut state = self.shared.state.lock().unwrap_or_else(|e| e.into_inner());
        state.paused = paused;
        self.shared.wake.notify_all();
    }
    pub fn checkpoint(&self) -> Result<()> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| ArchiveError::IoFailure)?;
        let mut waited = false;
        while state.paused && !state.cancelled {
            waited = true;
            #[cfg(test)]
            {
                state.pause_waiters += 1;
                self.shared.wake.notify_all();
            }
            state = self
                .shared
                .wake
                .wait(state)
                .map_err(|_| ArchiveError::IoFailure)?;
            #[cfg(test)]
            {
                state.pause_waiters -= 1;
            }
        }
        if state.cancelled {
            self.record_observed(
                &mut state,
                if waited {
                    CancellationBoundary::PausedCheckpoint
                } else {
                    CancellationBoundary::Checkpoint
                },
            );
            Err(ArchiveError::Cancelled)
        } else {
            Ok(())
        }
    }
    pub(crate) fn on_cancel(
        &self,
        hook: Arc<dyn Fn() + Send + Sync>,
    ) -> Result<Arc<dyn Fn() + Send + Sync>> {
        let cancelled = {
            let mut state = self
                .shared
                .state
                .lock()
                .map_err(|_| ArchiveError::IoFailure)?;
            state.cancel_hooks.retain(|hook| hook.strong_count() != 0);
            if state.cancel_hooks.len() >= 32 {
                return Err(ArchiveError::Busy);
            }
            state.cancel_hooks.push(Arc::downgrade(&hook));
            state.cancelled
        };
        if cancelled {
            hook();
        }
        Ok(hook)
    }

    pub(crate) fn set_current_member(&self, value: &str) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.current_member =
                (!value.is_empty() && value.len() <= 32767 && !value.chars().any(char::is_control))
                    .then(|| value.to_owned());
        }
    }
    pub(crate) fn configure_extraction_progress(&self, total_bytes: Option<u64>, files_total: u64) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.extraction = Some(ExtractionProgress {
                total_bytes,
                files_total,
                member_done: None,
                member_total: None,
            });
            state.current_member = None;
            state.latest = Some(ArchiveProgress {
                bytes: 0,
                files: 0,
                total_bytes,
            });
        }
    }
    pub(crate) fn begin_extracted_member(&self, member: &str, total: Option<u64>) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.current_member = (!member.is_empty()
                && member.len() <= 32767
                && !member.chars().any(char::is_control))
            .then(|| member.to_owned());
            let has_member = state.current_member.is_some();
            if let Some(extraction) = &mut state.extraction {
                extraction.member_done = has_member.then_some(0);
                extraction.member_total = if has_member { total } else { None };
            }
        }
    }
    pub(crate) fn clear_extracted_member(&self) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.current_member = None;
            if let Some(extraction) = &mut state.extraction {
                extraction.member_done = None;
                extraction.member_total = None;
            }
        }
    }
    pub(crate) fn publish_extraction_progress(
        &self,
        mut progress: ArchiveProgress,
        member_done: u64,
    ) {
        if let Ok(mut state) = self.shared.state.lock()
            && let Some(extraction) = &mut state.extraction
        {
            progress.total_bytes = extraction.total_bytes;
            if extraction.member_done.is_some() {
                extraction.member_done = Some(member_done);
                if extraction
                    .member_total
                    .is_some_and(|total| member_done > total)
                {
                    extraction.member_total = None;
                }
            }
            state.latest = Some(progress);
            return;
        }
        // Creation, modification and reclamation still use the original coalesced counter.
        self.publish(progress);
    }
    pub(crate) fn progress_snapshot(&self) -> Option<ProgressSnapshot> {
        let state = self.shared.state.lock().ok()?;
        let progress = state.latest?;
        let extraction = state.extraction;
        Some(ProgressSnapshot {
            progress,
            phase: state.progress_phase,
            current_member: state.current_member.clone(),
            files_total: extraction.map(|value| value.files_total),
            current_bytes_done: extraction.and_then(|value| value.member_done),
            current_bytes_total: extraction.and_then(|value| value.member_total),
        })
    }
    pub fn latest_progress(&self) -> Option<ArchiveProgress> {
        self.shared.state.lock().ok()?.latest
    }
    pub(crate) fn set_progress_phase(&self, phase: &'static str) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.progress_phase = phase;
            state.latest = None;
            state.extraction = None;
            state.current_member = None;
            state.last_publish = None;
        }
    }
    pub(crate) fn publish(&self, progress: ArchiveProgress) {
        if let Ok(mut state) = self.shared.state.lock() {
            let now = Instant::now();
            if state
                .last_publish
                .is_none_or(|last| now.duration_since(last) >= Duration::from_millis(100))
            {
                state.latest = Some(progress);
                state.last_publish = Some(now);
            }
        }
    }
    pub(crate) fn finish_progress(&self, progress: ArchiveProgress) {
        if let Ok(mut state) = self.shared.state.lock() {
            state.latest = Some(progress);
        }
    }
}

#[cfg(test)]
mod cancellation_tests {
    use super::*;
    #[test]
    fn paused_checkpoint_is_really_waiting_then_cancel_wakes_without_resume() {
        let control = OperationControl::default();
        control.set_paused(true);
        let work = control.clone();
        let (send, receive) = std::sync::mpsc::channel();
        let thread = std::thread::spawn(move || {
            send.send(work.checkpoint())
                .expect("report actual checkpoint")
        });
        let state = control.shared.state.lock().expect("control state");
        let (state, deadline) = control
            .shared
            .wake
            .wait_timeout_while(state, Duration::from_secs(2), |state| {
                state.pause_waiters == 0
            })
            .expect("real pause wait barrier");
        assert!(!deadline.timed_out() && state.paused && !state.cancelled);
        assert!(state.cancellation.observed_us.is_none());
        drop(state);
        let before = Instant::now();
        control.cancel();
        assert_eq!(
            receive
                .recv_timeout(Duration::from_millis(200))
                .expect("bounded cooperative wake"),
            Err(ArchiveError::Cancelled)
        );
        let elapsed = before.elapsed();
        thread.join().expect("owned checkpoint thread");
        let state = control.shared.state.lock().expect("facts");
        assert!(state.paused, "no resume substituted for cancellation");
        assert_eq!(
            state.cancellation.boundary,
            Some(CancellationBoundary::PausedCheckpoint)
        );
        assert!(
            state.cancellation.received_us.is_none(),
            "local cancel is not worker input reception"
        );
        assert!(elapsed < Duration::from_millis(200));
        eprintln!(
            "CANCEL_BOUNDARY paused_checkpoint caller_elapsed_us={}",
            elapsed.as_micros()
        );
    }
    #[test]
    fn receive_and_observe_are_first_wins_not_acknowledgements_or_payloads() {
        let control = OperationControl::default();
        assert!(control.cancellation_observation().is_none());
        control.receive_cancel();
        let received = control.cancellation_observation().expect("received");
        assert!(received.received_us.is_some() && received.observed_us.is_none());
        assert_eq!(control.checkpoint(), Err(ArchiveError::Cancelled));
        let observed = control.cancellation_observation().expect("observed");
        control.receive_cancel();
        control.observe_cancelled(CancellationBoundary::NativeReply);
        let duplicate = control.cancellation_observation().expect("stable");
        assert_eq!(observed.received_us, duplicate.received_us);
        assert_eq!(observed.observed_us, duplicate.observed_us);
        assert_eq!(duplicate.boundary, Some(CancellationBoundary::Checkpoint));
        assert!(observed.observed_us >= observed.received_us);
    }
}
