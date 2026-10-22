//! Presentation-only download rate/ETA sampling; never changes transfer/checkpoint state.
use std::collections::VecDeque;
use std::time::Duration;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct ProgressDisplay {
    pub bytes_per_second: u64,
    pub remaining_seconds: Option<u64>,
}

#[derive(Default)]
pub(super) struct ProgressDisplaySampler {
    samples: VecDeque<(Duration, u64)>,
    displayed: ProgressDisplay,
    last_display_at: Option<Duration>,
    transferring: bool,
}

impl ProgressDisplaySampler {
    pub fn observe(
        &mut self,
        now: Duration,
        bytes: u64,
        total: Option<u64>,
        transferring: bool,
    ) -> ProgressDisplay {
        let reset = self.transferring != transferring
            || self
                .samples
                .back()
                .is_some_and(|(time, previous)| now < *time || bytes < *previous);
        self.transferring = transferring;
        if reset || !transferring {
            self.samples.clear();
            self.last_display_at = None;
            self.displayed = ProgressDisplay::default();
        }
        if !transferring {
            return self.displayed;
        }
        if self.samples.is_empty() {
            self.samples.push_back((now, bytes));
            self.last_display_at = Some(now);
            return self.displayed;
        }
        if self
            .last_display_at
            .is_some_and(|last| now.saturating_sub(last) < Duration::from_secs(1))
        {
            return self.displayed;
        }
        self.samples.push_back((now, bytes));
        while self.samples.len() > 2
            && self
                .samples
                .get(1)
                .is_some_and(|(time, _)| now.saturating_sub(*time) >= Duration::from_secs(5))
        {
            self.samples.pop_front();
        }
        let (first_time, first_bytes) = self
            .samples
            .front()
            .copied()
            .expect("nonempty sample window");
        let nanos = now.saturating_sub(first_time).as_nanos();
        let rate = u128::from(bytes.saturating_sub(first_bytes))
            .saturating_mul(1_000_000_000)
            .checked_div(nanos)
            .unwrap_or_default()
            .min(u128::from(u64::MAX)) as u64;
        self.displayed = ProgressDisplay {
            bytes_per_second: rate,
            remaining_seconds: total
                .filter(|total| *total > 0)
                .zip((rate > 0).then_some(rate))
                .map(|(total, rate)| total.saturating_sub(bytes) / rate),
        };
        self.last_display_at = Some(now);
        self.displayed
    }

    pub fn current(&self) -> ProgressDisplay {
        self.displayed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn displayed_rate_and_eta_update_once_per_second_and_use_recent_byte_window() {
        let mut sampler = ProgressDisplaySampler::default();
        assert_eq!(
            sampler.observe(Duration::ZERO, 0, Some(1000), true),
            ProgressDisplay::default()
        );
        let first = sampler.observe(Duration::from_secs(1), 100, Some(1000), true);
        assert_eq!(
            first,
            ProgressDisplay {
                bytes_per_second: 100,
                remaining_seconds: Some(9)
            }
        );
        assert_eq!(
            sampler.observe(Duration::from_millis(1500), 500, Some(1000), true),
            first
        );
        assert_eq!(
            sampler
                .observe(Duration::from_secs(2), 500, Some(1000), true)
                .bytes_per_second,
            250
        );
        assert_eq!(sampler.samples.len(), 3);
        for second in 3..=20 {
            sampler.observe(Duration::from_secs(second), second * 100, Some(5000), true);
        }
        assert!(sampler.samples.len() <= 6);
        assert_eq!(sampler.current().bytes_per_second, 100);
    }
    #[test]
    fn paused_restart_and_unknown_size_never_show_stale_or_fabricated_estimates() {
        let mut sampler = ProgressDisplaySampler::default();
        sampler.observe(Duration::ZERO, 100, None, true);
        assert_eq!(
            sampler
                .observe(Duration::from_secs(1), 200, None, true)
                .remaining_seconds,
            None
        );
        assert_eq!(
            sampler.observe(Duration::from_millis(1100), 200, None, false),
            ProgressDisplay::default()
        );
        assert_eq!(
            sampler.observe(Duration::from_secs(2), 0, Some(1000), true),
            ProgressDisplay::default()
        );
        assert_eq!(
            sampler
                .observe(Duration::from_secs(3), 100, Some(1000), true)
                .bytes_per_second,
            100
        );
        assert_eq!(
            sampler.observe(Duration::from_millis(3100), 0, Some(1000), true),
            ProgressDisplay::default()
        );
        assert_eq!(
            sampler
                .observe(Duration::from_secs(5), 0, Some(1000), true)
                .remaining_seconds,
            None
        );
    }
}
