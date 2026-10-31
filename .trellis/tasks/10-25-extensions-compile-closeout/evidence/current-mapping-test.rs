#[path = "current-event-history.rs"]
mod event_history;
#[test]
fn actual_unsupported_transport_mapping_is_unavailable_not_success() {
    let observed = event_history::EventOutcome::from_result(
        &Err(nxtools_extensions::RuntimeError::UnsupportedTransport),
    );
    assert_eq!(observed, event_history::EventOutcome::Unavailable);
    assert!(!observed.success());
}
