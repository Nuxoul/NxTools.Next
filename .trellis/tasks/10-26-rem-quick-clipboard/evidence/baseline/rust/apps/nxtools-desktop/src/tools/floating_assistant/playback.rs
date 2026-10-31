//! One monotonic playback clock; only its owning controller schedules returned wakeups.
use std::time::{Duration, Instant};

const MIN_FRAME_TIME: Duration = Duration::from_millis(1);
const ZERO_DELAY_FALLBACK: Duration = Duration::from_millis(100);
const MIB_PER_SECOND: f64 = 1_048_576.0;

/// `downloading_tasks` counts only tasks currently Downloading, not queued/paused/completed.
/// A Downloading task with zero aggregate throughput deliberately pauses playback.
pub(crate) fn download_speed_factor(
    sync_enabled: bool,
    downloading_tasks: usize,
    downloading_bytes_per_second: u64,
) -> f64 {
    if !sync_enabled || downloading_tasks == 0 {
        1.0
    } else if downloading_bytes_per_second == 0 {
        0.0
    } else {
        (downloading_bytes_per_second as f64 / MIB_PER_SECOND).clamp(0.25, 4.0)
    }
}

/// Frame indexes are read from the same frame vector supplied to `new` (or `reset`).
/// The source clock is advanced only when visible, enabled, and motion is allowed.
/// There is no task, frame queue, platform dependency, or implicit polling here.
#[derive(Debug)]
pub(crate) struct Playback {
    ends: Vec<f64>,
    cycle_secs: f64,
    source_secs: f64,
    last_sample: Instant,
    last_presented: Instant,
    displayed: usize,
    min_display_time: Duration,
    factor: f64,
    enabled: bool,
    visible: bool,
    motion_allowed: bool,
}

impl Playback {
    pub(crate) fn new(durations: &[Duration], fps: u32, now: Instant) -> Self {
        let mut ends = Vec::with_capacity(durations.len());
        let mut cycle_secs = 0.0;
        for duration in durations {
            cycle_secs += if duration.is_zero() {
                ZERO_DELAY_FALLBACK
            } else {
                *duration
            }
            .as_secs_f64()
            .max(MIN_FRAME_TIME.as_secs_f64());
            ends.push(cycle_secs);
        }
        Self {
            ends,
            cycle_secs,
            source_secs: 0.0,
            last_sample: now,
            last_presented: now,
            displayed: 0,
            min_display_time: Duration::from_secs_f64(1.0 / f64::from(fps.clamp(1, 60))),
            factor: 1.0,
            enabled: true,
            visible: true,
            motion_allowed: true,
        }
    }

    fn running(&self) -> bool {
        self.ends.len() > 1
            && self.enabled
            && self.visible
            && self.motion_allowed
            && self.factor > 0.0
    }

    fn advance(&mut self, now: Instant) {
        if self.running() {
            let elapsed = now
                .saturating_duration_since(self.last_sample)
                .as_secs_f64();
            // Remainder keeps the source clock bounded even after long sleep/catch-up.
            self.source_secs = (self.source_secs + elapsed * self.factor) % self.cycle_secs;
        }
        self.last_sample = now;
    }

    /// Update speed at the transition instant so the prior rate is accounted for exactly once.
    pub(crate) fn set_download_speed(
        &mut self,
        sync_enabled: bool,
        downloading_tasks: usize,
        downloading_bytes_per_second: u64,
        now: Instant,
    ) {
        self.advance(now);
        self.factor = download_speed_factor(
            sync_enabled,
            downloading_tasks,
            downloading_bytes_per_second,
        );
    }

    /// A hidden/disabled/reduced surface freezes its playhead, with no scheduled wakeup.
    pub(crate) fn set_activity(
        &mut self,
        enabled: bool,
        visible: bool,
        motion_allowed: bool,
        now: Instant,
    ) {
        self.advance(now);
        self.enabled = enabled;
        self.visible = visible;
        self.motion_allowed = motion_allowed;
    }

    pub(crate) fn set_fps(&mut self, fps: u32, now: Instant) {
        self.advance(now);
        self.min_display_time = Duration::from_secs_f64(1.0 / f64::from(fps.clamp(1, 60)));
    }

    /// Start a replacement asset at frame zero without a retired asset's timer or playhead.
    pub(crate) fn reset(&mut self, durations: &[Duration], fps: u32, now: Instant) {
        let enabled = self.enabled;
        let visible = self.visible;
        let motion_allowed = self.motion_allowed;
        let factor = self.factor;
        *self = Self::new(durations, fps, now);
        self.enabled = enabled;
        self.visible = visible;
        self.motion_allowed = motion_allowed;
        self.factor = factor;
    }

    fn current_index(&self) -> usize {
        self.ends
            .partition_point(|end| *end <= self.source_secs)
            .min(self.ends.len().saturating_sub(1))
    }

    /// Call on a scheduled wake. Returns only a changed frame; elapsed frames are skipped.
    pub(crate) fn poll(&mut self, now: Instant) -> Option<usize> {
        self.advance(now);
        if !self.running()
            || now.saturating_duration_since(self.last_presented) < self.min_display_time
        {
            return None;
        }
        let due = self.current_index();
        if due == self.displayed {
            return None;
        }
        self.displayed = due;
        self.last_presented = now;
        Some(due)
    }

    pub(crate) fn displayed(&self) -> usize {
        self.displayed
    }

    /// Next wall-clock opportunity for a changed frame, or None (static/inactive/paused).
    /// If the display cap masks a source frame, the wake is deferred rather than queued.
    pub(crate) fn next_wake(&self, now: Instant) -> Option<Instant> {
        if !self.running() {
            return None;
        }
        let elapsed = now
            .saturating_duration_since(self.last_sample)
            .as_secs_f64();
        let position = (self.source_secs + elapsed * self.factor) % self.cycle_secs;
        let due = self.ends.partition_point(|end| *end <= position);
        // A render-cap skip may leave the displayed index behind the playhead;
        // repeated overdue probes then need only one capped wake, not a frame queue.
        let cap_deadline = self.last_presented + self.min_display_time;
        if due != self.displayed {
            // Source has already passed the displayed frame. Do not spin if cap expired.
            return Some(cap_deadline.max(now + MIN_FRAME_TIME));
        }
        let boundary = self.ends[due];
        let until_source = (boundary - position) / self.factor;
        let frame_deadline = now + Duration::from_secs_f64(until_source.max(0.0));
        Some(frame_deadline.max(cap_deadline).max(now + MIN_FRAME_TIME))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: Instant, millis: u64) -> Instant {
        start + Duration::from_millis(millis)
    }

    #[test]
    fn download_mapping_has_baseline_pause_and_clamped_extremes() {
        assert_eq!(download_speed_factor(false, 1, 0), 1.0);
        assert_eq!(download_speed_factor(true, 0, 0), 1.0);
        assert_eq!(download_speed_factor(true, 1, 0), 0.0);
        assert_eq!(download_speed_factor(true, 1, 1), 0.25);
        assert_eq!(download_speed_factor(true, 2, 262_144), 0.25);
        assert_eq!(download_speed_factor(true, 1, 1_048_576), 1.0);
        assert_eq!(download_speed_factor(true, 1, 4_194_304), 4.0);
        assert_eq!(download_speed_factor(true, 1, u64::MAX), 4.0);
    }

    #[test]
    fn gif_source_timing_and_025_1_4x_rates() {
        let start = Instant::now();
        let durations = [Duration::from_millis(100), Duration::from_millis(200)];
        for (rate, expected) in [(262_144, 400), (1_048_576, 100), (4_194_304, 25)] {
            let mut clock = Playback::new(&durations, 60, start);
            clock.set_download_speed(true, 1, rate, start);
            assert_eq!(clock.poll(at(start, expected - 1)), None);
            assert_eq!(clock.poll(at(start, expected)), Some(1));
        }
        let clock = Playback::new(&durations, 60, start);
        assert_eq!(clock.next_wake(start), Some(at(start, 100)));
    }

    #[test]
    fn pause_and_speed_changes_preserve_fractional_time() {
        let start = Instant::now();
        let mut clock = Playback::new(&[Duration::from_millis(100); 2], 60, start);
        clock.set_download_speed(true, 1, 0, at(start, 50));
        assert_eq!(clock.next_wake(at(start, 5_000)), None);
        assert_eq!(clock.poll(at(start, 5_000)), None);
        clock.set_download_speed(true, 1, 262_144, at(start, 5_000));
        assert_eq!(clock.poll(at(start, 5_199)), None);
        assert_eq!(clock.poll(at(start, 5_200)), Some(1));
        clock.set_download_speed(false, 1, 0, at(start, 5_210));
        assert_eq!(clock.poll(at(start, 5_280)), None);
        assert_eq!(clock.poll(at(start, 5_310)), Some(0));
    }

    #[test]
    fn inactive_and_static_have_no_timer_and_resume_without_catchup() {
        let start = Instant::now();
        let mut static_clock = Playback::new(&[Duration::from_millis(100)], 60, start);
        assert_eq!(static_clock.next_wake(start), None);
        assert_eq!(static_clock.poll(at(start, 500)), None);
        let mut clock = Playback::new(&[Duration::from_millis(100); 2], 60, start);
        clock.set_activity(true, false, true, at(start, 50));
        assert_eq!(clock.next_wake(at(start, 5_000)), None);
        clock.set_activity(true, true, false, at(start, 5_000));
        assert_eq!(clock.next_wake(at(start, 5_000)), None);
        clock.set_activity(false, true, true, at(start, 5_000));
        assert_eq!(clock.next_wake(at(start, 5_000)), None);
        clock.set_activity(true, true, true, at(start, 5_000));
        assert_eq!(clock.poll(at(start, 5_049)), None);
        assert_eq!(clock.poll(at(start, 5_050)), Some(1));
        clock.reset(&[Duration::from_millis(200)], 60, at(start, 5_060));
        assert_eq!(clock.displayed(), 0);
        assert_eq!(clock.next_wake(at(start, 5_060)), None);
    }

    #[test]
    fn cap_limits_60fps_even_at_four_x_and_skips_elapsed_frames() {
        let start = Instant::now();
        let durations = [Duration::from_millis(20); 8];
        let mut clock = Playback::new(&durations, 60, start);
        clock.set_download_speed(true, 1, 4_194_304, start);
        assert_eq!(clock.poll(at(start, 5)), None);
        assert_eq!(clock.poll(at(start, 17)), Some(3));
        assert_eq!(clock.poll(at(start, 20)), None);
        assert!(
            clock.next_wake(at(start, 20)).expect("wake")
                >= at(start, 17) + Duration::from_secs_f64(1.0 / 60.0)
        );
        assert_eq!(clock.poll(at(start, 34)), Some(6));
        clock.set_fps(1, at(start, 34));
        assert_eq!(clock.poll(at(start, 200)), None);
    }
}
