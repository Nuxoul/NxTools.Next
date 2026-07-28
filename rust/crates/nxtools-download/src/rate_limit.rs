use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use thiserror::Error;
use tokio::sync::watch;

const MAX_BURST_BYTES: u64 = 64 * 1024;
const BURST_WINDOW_DIVISOR: u64 = 10;

#[derive(Debug, Clone)]
pub struct DownloadRateLimiter {
    inner: Arc<RateLimiterInner>,
}

#[derive(Debug)]
struct RateLimiterInner {
    state: Mutex<RateLimiterState>,
    configured_bytes_per_second: AtomicU64,
    revision: watch::Sender<u64>,
}

#[derive(Debug)]
struct RateLimiterState {
    bytes_per_second: Option<u64>,
    available_tokens: f64,
    last_refill: Instant,
}

impl DownloadRateLimiter {
    pub fn new(bytes_per_second: Option<u64>) -> Result<Self, DownloadRateLimitError> {
        validate_limit(bytes_per_second)?;
        let (revision, _) = watch::channel(0);
        Ok(Self {
            inner: Arc::new(RateLimiterInner {
                state: Mutex::new(RateLimiterState::new(bytes_per_second)),
                configured_bytes_per_second: AtomicU64::new(bytes_per_second.unwrap_or_default()),
                revision,
            }),
        })
    }

    pub fn bytes_per_second(&self) -> Option<u64> {
        match self
            .inner
            .configured_bytes_per_second
            .load(Ordering::Acquire)
        {
            0 => None,
            bytes_per_second => Some(bytes_per_second),
        }
    }

    pub fn set_bytes_per_second(
        &self,
        bytes_per_second: Option<u64>,
    ) -> Result<(), DownloadRateLimitError> {
        validate_limit(bytes_per_second)?;
        {
            let mut state = self
                .inner
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if state.bytes_per_second == bytes_per_second {
                return Ok(());
            }
            *state = RateLimiterState::new(bytes_per_second);
        }
        self.inner
            .configured_bytes_per_second
            .store(bytes_per_second.unwrap_or_default(), Ordering::Release);
        self.inner.revision.send_modify(|revision| {
            *revision = revision.wrapping_add(1);
        });
        Ok(())
    }

    pub async fn acquire(&self, byte_count: u64) {
        if byte_count == 0
            || self
                .inner
                .configured_bytes_per_second
                .load(Ordering::Acquire)
                == 0
        {
            return;
        }
        let mut remaining = byte_count;
        let mut revisions = self.inner.revision.subscribe();
        while remaining > 0 {
            let wait = {
                let mut state = self
                    .inner
                    .state
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                let Some(bytes_per_second) = state.bytes_per_second else {
                    return;
                };
                state.refill(bytes_per_second);
                let quantum = remaining.min(burst_bytes(bytes_per_second));
                if state.available_tokens >= quantum as f64 {
                    state.available_tokens -= quantum as f64;
                    remaining -= quantum;
                    None
                } else {
                    let missing = quantum as f64 - state.available_tokens;
                    Some(Duration::from_secs_f64(missing / bytes_per_second as f64))
                }
            };
            let Some(wait) = wait else {
                continue;
            };
            tokio::select! {
                _ = tokio::time::sleep(wait) => {}
                changed = revisions.changed() => {
                    if changed.is_err() {
                        return;
                    }
                }
            }
        }
    }
}

impl RateLimiterState {
    fn new(bytes_per_second: Option<u64>) -> Self {
        Self {
            bytes_per_second,
            available_tokens: bytes_per_second.map_or(0.0, |limit| burst_bytes(limit) as f64),
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self, bytes_per_second: u64) {
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(self.last_refill);
        self.last_refill = now;
        self.available_tokens = (self.available_tokens
            + elapsed.as_secs_f64() * bytes_per_second as f64)
            .min(burst_bytes(bytes_per_second) as f64);
    }
}

fn burst_bytes(bytes_per_second: u64) -> u64 {
    (bytes_per_second / BURST_WINDOW_DIVISOR).clamp(1, MAX_BURST_BYTES)
}

fn validate_limit(bytes_per_second: Option<u64>) -> Result<(), DownloadRateLimitError> {
    if bytes_per_second == Some(0) {
        return Err(DownloadRateLimitError::ZeroLimit);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum DownloadRateLimitError {
    #[error("download speed limit must be greater than zero; use none for unlimited")]
    ZeroLimit,
}
