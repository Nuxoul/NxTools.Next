//! Bounded observation of existing native fixture paint; never schedules frames.
use super::*;

struct Cadence {
    started: Instant,
    frames_us: Vec<u64>,
    milestones_us: Vec<(&'static str, u64)>,
}
impl Global for Cadence {}

pub(super) fn init(cx: &mut App) {
    if std::env::var("NXTOOLS_PREVIEW_NOTIFICATION_CADENCE").as_deref() == Ok("1") {
        cx.set_global(Cadence {
            started: Instant::now(),
            frames_us: Vec::with_capacity(512),
            milestones_us: Vec::with_capacity(8),
        });
    }
}

pub(super) fn paint(cx: &mut App) {
    if cx.try_global::<Cadence>().is_some() {
        let cadence = cx.global_mut::<Cadence>();
        if cadence.frames_us.len() < 512 {
            cadence
                .frames_us
                .push(cadence.started.elapsed().as_micros() as u64);
        }
    }
}

pub(super) fn mark(stage: &'static str, cx: &mut App) {
    if cx.try_global::<Cadence>().is_some() {
        let cadence = cx.global_mut::<Cadence>();
        if cadence.milestones_us.len() < 8 {
            cadence
                .milestones_us
                .push((stage, cadence.started.elapsed().as_micros() as u64));
        }
    }
}

pub(super) fn snapshot(cx: &App) -> Option<serde_json::Value> {
    let cadence = cx.try_global::<Cadence>()?;
    Some(
        serde_json::json!({"frames_us": cadence.frames_us, "milestones_us": cadence.milestones_us, "bounded": true,
        "observer_requests_frames": false, "elapsed_us": cadence.started.elapsed().as_micros() as u64}),
    )
}

pub(super) fn options(mut options: WindowOptions, cx: &App) -> WindowOptions {
    if cx.try_global::<Cadence>().is_some()
        && std::env::var("NXTOOLS_PREVIEW_NOTIFICATION_LEGACY_CAP").as_deref() == Ok("1")
    {
        options.inactive_frame_interval = Some(Duration::from_micros(33_333));
    }
    options
}
