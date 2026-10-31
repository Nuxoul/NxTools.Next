use nxtools_extensions::{Causality, EventGuard};
use std::{collections::VecDeque, sync::{Arc, Mutex}, time::Instant};
use uuid::Uuid;
#[derive(Clone)]
pub(crate) struct EventControl {
    guard: Arc<Mutex<EventGuard>>,
    changed: tokio::sync::watch::Sender<u64>,
    history: Arc<Mutex<VecDeque<super::event_history::EventTerminal>>>,
}
impl Default for EventControl {
    fn default() -> Self {
        Self {
            guard: Default::default(),
            changed: tokio::sync::watch::channel(0).0,
            history: Default::default(),
        }
    }
}
impl EventControl {
    pub(crate) fn subscribe(&self) -> tokio::sync::watch::Receiver<u64> {
        self.changed.subscribe()
    }
    pub(crate) fn is_paused(&self, package: &str, handler: &str) -> bool {
        self.guard
            .lock()
            .map_or(true, |guard| guard.is_paused(package, handler))
    }
    pub(crate) fn retry(&self, package: &str, handler: &str) -> anyhow::Result<()> {
        self.guard
            .lock()
            .map_err(|_| anyhow::anyhow!("event control unavailable"))?
            .retry(package, handler);
        self.changed
            .send_modify(|revision| *revision = revision.wrapping_add(1));
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn pause_for_test(&self, package: &str, handler: &str) {
        let now = Instant::now();
        for index in 0..3 {
            let id = Uuid::new_v4();
            self.guard
                .lock()
                .expect("guard")
                .admit(
                    true,
                    id,
                    package,
                    handler,
                    &Causality::root(id),
                    now + std::time::Duration::from_secs(index),
                )
                .expect("admit");
            self.terminal(package, handler, false);
        }
    }
    pub(crate) fn recent(&self) -> Vec<super::event_history::EventTerminal> {
        self.history
            .lock()
            .map(|history| history.iter().rev().cloned().collect())
            .unwrap_or_default()
    }
    pub(crate) fn latest(
        &self,
        package: &str,
        handler: &str,
    ) -> Option<super::event_history::EventTerminal> {
        self.history
            .lock()
            .ok()?
            .iter()
            .rev()
            .find(|item| item.package_id == package && item.handler_id == handler)
            .cloned()
    }
    fn record(&self, terminal: super::event_history::EventTerminal) {
        let Ok(mut history) = self.history.lock() else {
            return;
        };
        if history.iter().any(|item| {
            item.event_id == terminal.event_id
                && item.package_id == terminal.package_id
                && item.handler_id == terminal.handler_id
        }) {
            return;
        }
        if let Ok(mut guard) = self.guard.lock() {
            let before = guard.is_paused(&terminal.package_id, &terminal.handler_id);
            guard.terminal(
                &terminal.package_id,
                &terminal.handler_id,
                terminal.outcome.success(),
                Instant::now(),
            );
            if !before && guard.is_paused(&terminal.package_id, &terminal.handler_id) {
                tracing::warn!(event="extension.event.paused", correlation_id=%terminal.correlation_id, "automatic handler paused after repeated failures");
            }
        }
        history.push_back(terminal);
        while history.len() > 64 {
            history.pop_front();
        }
        drop(history);
        self.changed
            .send_modify(|revision| *revision = revision.wrapping_add(1));
    }
    #[cfg(test)]
    fn terminal(&self, package: &str, handler: &str, success: bool) {
        if let Ok(mut guard) = self.guard.lock() {
            let before = guard.is_paused(package, handler);
            guard.terminal(package, handler, success, Instant::now());
            if !before && guard.is_paused(package, handler) {
                self.changed
                    .send_modify(|revision| *revision = revision.wrapping_add(1));
                tracing::warn!(
                    event = "extension.event.paused",
                    "automatic handler paused after repeated failures"
                );
            }
        }
    }
}

#[cfg(test)]
mod compile_closeout_tests {
    use super::*;
    use crate::extensions::event_history::{EventOutcome, EventTerminal};
    use nxtools_extensions::RuntimeError;

    #[test]
    fn compile_closeout_unsupported_transport_is_unavailable_failure_without_replay() {
        let control = EventControl::default();
        let now = Instant::now();
        for index in 0..3 {
            let id = Uuid::new_v4();
            control
                .guard
                .lock()
                .expect("original event guard")
                .admit(
                    true,
                    id,
                    "fixture.transport",
                    "event",
                    &Causality::root(id),
                    now + std::time::Duration::from_secs(index),
                )
                .expect("admit a distinct synthetic event");
            let terminal = EventTerminal {
                event_id: id,
                correlation_id: id,
                package_id: "fixture.transport".into(),
                handler_id: "event".into(),
                at_unix_ms: index as i64,
                elapsed_ms: 0,
                outcome: EventOutcome::from_result(&Err(RuntimeError::UnsupportedTransport)),
            };
            control.record(terminal.clone());
            control.record(terminal);
            let observed = control
                .latest("fixture.transport", "event")
                .expect("original terminal history");
            assert_eq!(observed.outcome, EventOutcome::Unavailable);
            assert!(!observed.outcome.success());
        }
        assert_eq!(
            control.recent().len(),
            3,
            "duplicate terminals stay deduplicated"
        );
        assert!(control.is_paused("fixture.transport", "event"));
        control
            .retry("fixture.transport", "event")
            .expect("explicit circuit retry");
        assert!(!control.is_paused("fixture.transport", "event"));
        assert_eq!(control.recent().len(), 3, "retry never replays old events");
    }
}
