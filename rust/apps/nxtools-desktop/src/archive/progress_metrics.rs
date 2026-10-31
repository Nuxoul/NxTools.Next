//! Bounded event-driven presentation samples. No task, timer, IPC or execution ownership.
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Default)]
pub(super) struct RateSamples {
    scope: Option<(Uuid, u64, String)>,
    samples: VecDeque<(u64, u64)>,
    last_observed: Option<(u64, u64)>,
}
impl RateSamples {
    pub(super) fn clear(&mut self) {
        self.scope = None;
        self.samples.clear();
        self.last_observed = None;
    }
    pub(super) fn observe(&mut self, scope: (Uuid, u64, String), now: u64, bytes: u64) {
        if self.scope.as_ref() != Some(&scope)
            || self
                .last_observed
                .is_some_and(|(time, done)| now < time || bytes < done)
        {
            self.clear();
            self.scope = Some(scope);
        }
        self.last_observed = Some((now, bytes));
        if self
            .samples
            .back()
            .is_some_and(|(time, _)| now.saturating_sub(*time) < 100)
        {
            return;
        }
        self.samples.push_back((now, bytes));
        while self.samples.len() > 64
            || self
                .samples
                .front()
                .is_some_and(|(time, _)| now.saturating_sub(*time) > 5_000)
        {
            self.samples.pop_front();
        }
    }
    pub(super) fn expire(&mut self, now: u64) {
        while self
            .samples
            .front()
            .is_some_and(|(time, _)| now.saturating_sub(*time) > 5_000)
        {
            self.samples.pop_front();
        }
    }
    pub(super) fn rate(&self) -> Option<f64> {
        let &(now, done) = self.samples.back()?;
        let &(start, before) = self.samples.front()?;
        let elapsed = now.checked_sub(start)?;
        // A gap is not evidence of a current rate. Repeated equal counters also suppress ETA.
        if elapsed < 1_000 || self.samples.len() < 2 {
            return None;
        }
        let previous = self.samples.get(self.samples.len() - 2)?;
        if done == previous.1 {
            return Some(0.0);
        }
        Some(done.checked_sub(before)? as f64 * 1_000.0 / elapsed as f64)
    }
    pub(super) fn eta(&self, done: u64, total: Option<u64>) -> Option<u64> {
        let total = total.filter(|total| *total > done)?;
        let rate = self.rate().filter(|rate| *rate >= 1.0)?;
        Some(((total - done) as f64 / rate).ceil().min(u64::MAX as f64) as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope() -> (Uuid, u64, String) {
        (Uuid::nil(), 1, "running".into())
    }
    #[test]
    fn hz2_rate_insufficient_window_coalescing_zero_and_unknown_totals() {
        let mut rate = RateSamples::default();
        rate.observe(scope(), 0, 0);
        rate.observe(scope(), 999, 999);
        assert_eq!(rate.rate(), None);
        rate.observe(scope(), 1_100, 1_100);
        assert_eq!(rate.rate(), Some(1_000.));
        assert_eq!(rate.eta(1_100, Some(2_100)), Some(1));
        assert_eq!(rate.eta(1_100, None), None);
        assert_eq!(rate.eta(1_100, Some(0)), None);
        rate.observe(scope(), 1_200, 1_100);
        assert_eq!(rate.rate(), Some(0.));
        assert_eq!(rate.eta(1_100, Some(2_100)), None);
        for time in 1_201..20_000 {
            rate.observe(scope(), time, time);
        }
        assert!(rate.samples.len() <= 51);
        assert!(
            rate.samples.back().expect("last sample").0
                - rate.samples.front().expect("first sample").0
                <= 5_000
        );
    }
    #[test]
    fn hz2_rate_attempt_stage_reset_backward_clock_and_counter_reset() {
        let mut rate = RateSamples::default();
        for (time, bytes) in [(0, 0), (1000, 1000)] {
            rate.observe(scope(), time, bytes);
        }
        assert!(rate.rate().is_some());
        rate.observe((Uuid::new_v4(), 1, "running".into()), 2000, 2000);
        assert_eq!(rate.rate(), None);
        rate.observe(scope(), 3000, 3000);
        rate.observe(scope(), 4000, 4000);
        rate.observe(scope(), 3500, 3500);
        assert_eq!(rate.rate(), None);
        rate.observe(scope(), 4500, 1);
        assert_eq!(rate.rate(), None);
        rate.clear();
        assert!(rate.samples.is_empty() && rate.scope.is_none());
    }
}
