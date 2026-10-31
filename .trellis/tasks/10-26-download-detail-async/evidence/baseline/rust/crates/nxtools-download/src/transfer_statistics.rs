//! One task's monotonic union of active payload workers. No SQL, logging or UI.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::DownloadTransferStatistics;

#[derive(Debug, Clone)]
pub(crate) struct TransferStatistics(Arc<Mutex<Measurement>>);

#[derive(Debug)]
struct Measurement {
    bytes: Option<u64>,
    elapsed: Duration,
    workers: u32,
    started: Option<Instant>,
}

impl TransferStatistics {
    pub(crate) fn new(saved: Option<DownloadTransferStatistics>) -> Self {
        Self(Arc::new(Mutex::new(Measurement {
            bytes: saved.map(|saved| saved.transferred_bytes),
            elapsed: saved.map_or(Duration::ZERO, |saved| {
                Duration::from_millis(saved.active_download_ms)
            }),
            workers: 0,
            started: None,
        })))
    }

    pub(crate) fn worker(&self) -> ActiveTransfer {
        self.begin_at(Instant::now());
        ActiveTransfer(self.clone())
    }

    fn begin_at(&self, now: Instant) {
        if let Ok(mut state) = self.0.lock() {
            if state.workers == 0 {
                state.started = Some(now);
            }
            state.workers = state.workers.saturating_add(1);
        }
    }

    fn end_at(&self, now: Instant) {
        if let Ok(mut state) = self.0.lock() {
            state.workers = state.workers.saturating_sub(1);
            if state.workers == 0
                && let Some(started) = state.started.take()
            {
                state.elapsed = state
                    .elapsed
                    .saturating_add(now.saturating_duration_since(started));
            }
        }
    }

    pub(crate) fn add_bytes(&self, bytes: u64) {
        if let Ok(mut state) = self.0.lock() {
            state.bytes = state.bytes.and_then(|total| total.checked_add(bytes));
        }
    }

    pub(crate) fn invalidate(&self) {
        if let Ok(mut state) = self.0.lock() {
            state.bytes = None;
        }
    }

    pub(crate) fn snapshot(&self) -> Option<DownloadTransferStatistics> {
        self.snapshot_at(Instant::now())
    }

    fn snapshot_at(&self, now: Instant) -> Option<DownloadTransferStatistics> {
        let state = self.0.lock().ok()?;
        let elapsed =
            state
                .elapsed
                .saturating_add(state.started.map_or(Duration::ZERO, |started| {
                    now.saturating_duration_since(started)
                }));
        // Round once after summing, so sub-ms workers don't each lose a tick.
        let active_download_ms = u64::try_from(elapsed.as_nanos().div_ceil(1_000_000)).ok()?;
        Some(DownloadTransferStatistics {
            transferred_bytes: state.bytes?,
            active_download_ms,
        })
    }
}

pub(crate) struct ActiveTransfer(TransferStatistics);

impl Drop for ActiveTransfer {
    fn drop(&mut self) {
        self.0.end_at(Instant::now());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn average_monotonic_worker_union_excludes_pause_wait_retry_backoff_and_mux() {
        let stats = TransferStatistics::new(Some(DownloadTransferStatistics::default()));
        let clock = Instant::now();
        stats.begin_at(clock);
        stats.add_bytes(100);
        stats.begin_at(clock + Duration::from_secs(1));
        stats.add_bytes(200);
        stats.end_at(clock + Duration::from_secs(2));
        stats.end_at(clock + Duration::from_secs(3));
        assert_eq!(
            stats.snapshot_at(clock + Duration::from_secs(86_400)),
            Some(DownloadTransferStatistics {
                transferred_bytes: 300,
                active_download_ms: 3_000
            })
        );
        stats.begin_at(clock + Duration::from_secs(86_400));
        stats.add_bytes(100); // retransmitted payload counts even if an artifact was discarded
        stats.end_at(clock + Duration::from_secs(86_401));
        let saved = stats
            .snapshot_at(clock + Duration::from_secs(172_800))
            .expect("reliable pair");
        assert_eq!(
            saved,
            DownloadTransferStatistics {
                transferred_bytes: 400,
                active_download_ms: 4_000
            }
        );
        let resumed = TransferStatistics::new(Some(saved));
        resumed.begin_at(clock + Duration::from_secs(172_800));
        resumed.add_bytes(100);
        resumed.end_at(clock + Duration::from_secs(172_801));
        assert_eq!(
            resumed.snapshot_at(clock + Duration::from_secs(259_200)),
            Some(DownloadTransferStatistics {
                transferred_bytes: 500,
                active_download_ms: 5_000
            })
        );
    }

    #[test]
    fn average_unknown_prefix_and_overflow_never_become_a_partial_average() {
        let clock = Instant::now();
        let legacy = TransferStatistics::new(None);
        legacy.begin_at(clock);
        legacy.add_bytes(100);
        legacy.end_at(clock + Duration::from_secs(1));
        assert_eq!(legacy.snapshot_at(clock), None);
        let overflow = TransferStatistics::new(Some(DownloadTransferStatistics {
            transferred_bytes: u64::MAX,
            active_download_ms: 1,
        }));
        overflow.add_bytes(1);
        assert_eq!(overflow.snapshot_at(clock), None);
    }

    #[test]
    fn average_worker_guard_closes_on_failure_and_snapshot_rate_is_bounded() {
        let stats = TransferStatistics::new(Some(DownloadTransferStatistics::default()));
        {
            let _worker = stats.worker();
            stats.add_bytes(32);
        }
        let stopped = stats.snapshot().expect("stopped measurement");
        assert_eq!(
            stats.snapshot_at(Instant::now() + Duration::from_secs(86_400)),
            Some(stopped)
        );
        assert_eq!(
            DownloadTransferStatistics {
                transferred_bytes: 1_048_576,
                active_download_ms: 2_000
            }
            .bytes_per_second(),
            Some(524_288)
        );
        assert_eq!(
            DownloadTransferStatistics {
                transferred_bytes: 1,
                active_download_ms: 0
            }
            .bytes_per_second(),
            None
        );
        assert_eq!(
            DownloadTransferStatistics {
                transferred_bytes: u64::MAX,
                active_download_ms: 1
            }
            .bytes_per_second(),
            None
        );
    }
}
