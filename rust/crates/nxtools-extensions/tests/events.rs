use nxtools_extensions::{Causality, EventAdmissionError, EventGuard};
use std::time::{Duration, Instant};
use uuid::Uuid;
#[test]
fn events_default_off_deduplicate_reentry_and_pause_repeated_failures() {
    let now = Instant::now();
    let event = Uuid::new_v4();
    let root = Causality::root(event);
    let mut guard = EventGuard::default();
    assert_eq!(
        guard.admit(false, event, "fixture.plugin", "event", &root, now),
        Err(EventAdmissionError::Disabled)
    );
    let child = guard
        .admit(true, event, "fixture.plugin", "event", &root, now)
        .expect("admit committed event");
    assert_eq!(
        guard.admit(true, event, "fixture.plugin", "event", &root, now),
        Err(EventAdmissionError::Duplicate)
    );
    assert_eq!(
        guard.admit(true, Uuid::new_v4(), "fixture.plugin", "event", &child, now),
        Err(EventAdmissionError::Reentry)
    );
    guard.terminal("fixture.plugin", "event", false, now);
    for index in 1..3 {
        let time = now + Duration::from_secs(index);
        guard
            .admit(true, Uuid::new_v4(), "fixture.plugin", "event", &root, time)
            .expect("bounded retry before circuit");
        guard.terminal("fixture.plugin", "event", false, time);
    }
    assert_eq!(
        guard.admit(
            true,
            Uuid::new_v4(),
            "fixture.plugin",
            "event",
            &root,
            now + Duration::from_secs(3)
        ),
        Err(EventAdmissionError::CircuitOpen)
    );
    assert!(guard.is_paused("fixture.plugin", "event"));
    guard.retry("fixture.plugin", "event");
    assert!(!guard.is_paused("fixture.plugin", "event"));
    assert!(
        guard
            .admit(
                true,
                Uuid::new_v4(),
                "fixture.plugin",
                "event",
                &root,
                now + Duration::from_secs(4)
            )
            .is_ok()
    );
}
#[test]
fn causality_depth_and_admission_queue_are_bounded() {
    let root = Causality::root(Uuid::new_v4());
    let mut causality = root.clone();
    for index in 0..4 {
        causality = causality
            .enter("fixture.plugin", &format!("handler-{index}"))
            .expect("allowed depth");
    }
    assert_eq!(
        causality.enter("fixture.plugin", "overflow"),
        Err(EventAdmissionError::Reentry)
    );
    let mut guard = EventGuard::default();
    let now = Instant::now();
    for index in 0..32 {
        guard
            .admit(
                true,
                Uuid::new_v4(),
                "fixture.plugin",
                &format!("handler-{index}"),
                &root,
                now,
            )
            .expect("bounded queue");
    }
    assert_eq!(
        guard.admit(
            true,
            Uuid::new_v4(),
            "fixture.plugin",
            "overflow",
            &root,
            now
        ),
        Err(EventAdmissionError::QueueFull)
    );
}
