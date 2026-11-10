use super::*;
use gpui_kit::base::motion::{Easing, MotionTransform, Timing};
use nxtools_core::QuickLaunchResultEntrance;

struct ResultTween {
    started_at: Instant,
    mode: QuickLaunchResultEntrance,
    timing: Timing,
}

impl ResultTween {
    fn sample(&self, now: Instant) -> MotionTransform {
        let progress = self
            .timing
            .sample(now.saturating_duration_since(self.started_at))
            .directed_progress;
        let remaining = 1.0 - progress;
        MotionTransform {
            translation: match self.mode {
                QuickLaunchResultEntrance::SlideUp => point(px(0.0), px(8.0 * remaining)),
                QuickLaunchResultEntrance::SlideFromRight => point(px(12.0 * remaining), px(0.0)),
                _ => point(px(0.0), px(0.0)),
            },
            opacity: progress,
            ..MotionTransform::identity()
        }
    }
}

/// Query-owned admission records outlive virtual rows; paint never starts a Tween.
#[derive(Default)]
pub(crate) struct ResultEntrances {
    generation: u64,
    correlation_id: uuid::Uuid,
    seen: HashSet<String>,
    active: HashMap<String, ResultTween>,
}

impl ResultEntrances {
    pub(crate) fn begin(&mut self, generation: u64, now: Instant) {
        if self.generation == generation {
            return;
        }
        self.finish("superseded", now);
        self.generation = generation;
        self.correlation_id = uuid::Uuid::new_v4();
        self.seen.clear();
    }

    pub(crate) fn admit(
        &mut self,
        generation: u64,
        identities: impl IntoIterator<Item = (String, bool)>,
        mode: QuickLaunchResultEntrance,
        allowed: bool,
        now: Instant,
    ) {
        if generation != self.generation {
            return;
        }
        if !allowed || mode == QuickLaunchResultEntrance::None {
            self.finish("suppressed", now);
        }
        let mut current = HashSet::new();
        let mut count = 0;
        for (id, visible) in identities {
            current.insert(id.clone());
            if self.seen.insert(id.clone())
                && visible
                && allowed
                && mode != QuickLaunchResultEntrance::None
            {
                self.active.insert(
                    id,
                    ResultTween {
                        started_at: now,
                        mode,
                        timing: Timing::new(RESULTS_APPEAR_DURATION).ease(Easing::EaseOut),
                    },
                );
                count += 1;
            }
        }
        let previous = self.active.len();
        let elapsed_ms = self
            .active
            .values()
            .map(|tween| now.saturating_duration_since(tween.started_at).as_millis() as u64)
            .max()
            .unwrap_or_default();
        self.active.retain(|id, _| current.contains(id));
        let retired = previous - self.active.len();
        if retired > 0 {
            info!(event = "quick_launch.results.entrance.completed",
                correlation_id = %self.correlation_id, generation, count = retired,
                category = "retired", elapsed_ms, "quick-launch result entrances completed");
        }
        if count > 0 {
            info!(event = "quick_launch.results.entrance.started",
                correlation_id = %self.correlation_id, generation, count,
                category = ?mode, "quick-launch result entrances started");
        }
    }

    pub(crate) fn advance(&mut self, now: Instant) {
        let previous = self.active.len();
        let elapsed_ms = self
            .active
            .values()
            .map(|tween| now.saturating_duration_since(tween.started_at).as_millis() as u64)
            .max()
            .unwrap_or_default();
        self.active.retain(|_, tween| {
            !tween
                .timing
                .sample(now.saturating_duration_since(tween.started_at))
                .finished
        });
        let count = previous - self.active.len();
        if count > 0 {
            info!(event = "quick_launch.results.entrance.completed",
                correlation_id = %self.correlation_id, generation = self.generation, count,
                elapsed_ms,
                category = "finished", "quick-launch result entrances completed");
        }
    }

    pub(crate) fn finish(&mut self, category: &'static str, now: Instant) {
        if self.active.is_empty() {
            return;
        }
        let count = self.active.len();
        let elapsed_ms = self
            .active
            .values()
            .map(|tween| now.saturating_duration_since(tween.started_at).as_millis() as u64)
            .max()
            .unwrap_or_default();
        self.active.clear();
        info!(event = "quick_launch.results.entrance.completed",
            correlation_id = %self.correlation_id, generation = self.generation, count,
            category, elapsed_ms, "quick-launch result entrances completed");
    }

    pub(crate) fn sample(&self, id: &str, now: Instant) -> MotionTransform {
        self.active
            .get(id)
            .map(|tween| tween.sample(now))
            .unwrap_or_default()
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.active.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quick_launch_result_tween_modes_are_finite_ease_out_values() {
        for mode in QuickLaunchResultEntrance::ALL {
            let now = Instant::now();
            let mut state = ResultEntrances::default();
            state.begin(1, now);
            state.admit(1, [("row".into(), true)], mode, true, now);
            let first = state.sample("row", now);
            let middle = state.sample("row", now + RESULTS_APPEAR_DURATION / 2);
            if mode == QuickLaunchResultEntrance::None {
                assert_eq!(first, MotionTransform::identity());
                assert!(!state.is_active());
            } else {
                assert_eq!(first.opacity, 0.0);
                assert!(middle.opacity > 0.5 && middle.opacity < 1.0, "ease-out");
                assert_eq!(
                    first.translation.x,
                    px(if mode == QuickLaunchResultEntrance::SlideFromRight {
                        12.0
                    } else {
                        0.0
                    })
                );
                assert_eq!(
                    first.translation.y,
                    px(if mode == QuickLaunchResultEntrance::SlideUp {
                        8.0
                    } else {
                        0.0
                    })
                );
            }
            state.advance(now + RESULTS_APPEAR_DURATION);
            assert_eq!(
                state.sample("row", now + RESULTS_APPEAR_DURATION),
                MotionTransform::identity()
            );
            assert!(!state.is_active());
        }
    }

    #[test]
    fn quick_launch_result_tween_delayed_identity_reorder_and_virtual_scroll_do_not_replay() {
        let now = Instant::now();
        let mut state = ResultEntrances::default();
        state.begin(1, now);
        state.admit(
            1,
            [("early".into(), true), ("offscreen".into(), false)],
            QuickLaunchResultEntrance::SlideUp,
            true,
            now,
        );
        state.advance(now + RESULTS_APPEAR_DURATION);
        let later = now + RESULTS_APPEAR_DURATION * 2;
        state.admit(
            1,
            [
                ("late".into(), true),
                ("early".into(), true),
                ("offscreen".into(), true),
            ],
            QuickLaunchResultEntrance::SlideUp,
            true,
            later,
        );
        assert_eq!(state.sample("early", later), MotionTransform::identity());
        assert_eq!(
            state.sample("offscreen", later),
            MotionTransform::identity()
        );
        assert_eq!(state.sample("late", later).opacity, 0.0);
        let started = state.active["late"].started_at;
        state.admit(
            1,
            [("early".into(), true), ("late".into(), true)],
            QuickLaunchResultEntrance::SlideUp,
            true,
            later + Duration::from_millis(20),
        );
        assert_eq!(state.active["late"].started_at, started);
        state.advance(later + RESULTS_APPEAR_DURATION);
        state.admit(
            1,
            [("late".into(), true)],
            QuickLaunchResultEntrance::SlideUp,
            true,
            later + RESULTS_APPEAR_DURATION,
        );
        assert!(!state.is_active());
        state.begin(2, later);
        state.admit(
            1,
            [("stale".into(), true)],
            QuickLaunchResultEntrance::SlideUp,
            true,
            later,
        );
        assert!(
            state.seen.is_empty(),
            "stale admission cannot affect the new generation"
        );
        state.admit(
            2,
            [("early".into(), true)],
            QuickLaunchResultEntrance::SlideUp,
            true,
            later,
        );
        assert!(
            state.is_active(),
            "same identity gets one entrance in the new query"
        );
    }

    #[test]
    fn quick_launch_result_tween_suppression_and_retirement_finish_without_rearming() {
        let now = Instant::now();
        for category in [
            "reduced",
            "unknown",
            "disabled",
            "hide",
            "close",
            "disconnected",
        ] {
            let mut state = ResultEntrances::default();
            state.begin(1, now);
            state.admit(
                1,
                [("row".into(), true)],
                QuickLaunchResultEntrance::SlideUp,
                true,
                now,
            );
            state.finish(category, now);
            assert_eq!(state.sample("row", now), MotionTransform::identity());
            state.admit(
                1,
                [("row".into(), true)],
                QuickLaunchResultEntrance::SlideUp,
                true,
                now,
            );
            assert!(
                !state.is_active(),
                "{category} cannot restart an admitted identity"
            );
            state.admit(
                1,
                [("suppressed".into(), true)],
                QuickLaunchResultEntrance::SlideUp,
                false,
                now,
            );
            state.admit(
                1,
                [("suppressed".into(), true)],
                QuickLaunchResultEntrance::SlideUp,
                true,
                now,
            );
            assert!(!state.is_active(), "suppressed admission is final");
        }
    }
}
