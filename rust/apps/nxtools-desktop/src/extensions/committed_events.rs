use crate::{
    clipboard_worker::{ClipboardCommitSubscription, ClipboardCommitted},
    extensions::service::ExtensionService,
};
use nxtools_download::{DownloadEvent, DownloadEventEnvelope, DownloadStatus, DownloadTaskId};
use nxtools_extensions::{Causality, EventGuard, InvokeRequest};
use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct EventControl {
    guard: Arc<Mutex<EventGuard>>,
    changed: tokio::sync::watch::Sender<u64>,
}
impl Default for EventControl {
    fn default() -> Self {
        Self {
            guard: Default::default(),
            changed: tokio::sync::watch::channel(0).0,
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

/// Bounded transition memory, seeded from existing persisted tasks (not replayed as created).
pub(crate) struct DownloadEventTransitions {
    states: HashMap<DownloadTaskId, DownloadStatus>,
    order: VecDeque<DownloadTaskId>,
    generation: u64,
    sequence: u64,
}
impl DownloadEventTransitions {
    pub(crate) fn new(engine: &nxtools_download::DownloadEngine) -> anyhow::Result<Self> {
        let mut this = Self {
            states: HashMap::new(),
            order: VecDeque::new(),
            generation: engine.session_generation(),
            sequence: 0,
        };
        for task in engine.tasks()?.into_iter().take(4096) {
            this.remember(task.task.id(), task.task.status());
        }
        Ok(this)
    }
    fn remember(&mut self, id: DownloadTaskId, status: DownloadStatus) {
        if !self.states.contains_key(&id) {
            self.order.push_back(id);
        }
        self.states.insert(id, status);
        while self.order.len() > 4096 {
            if let Some(old) = self.order.pop_front() {
                self.states.remove(&old);
            }
        }
    }
    pub(crate) fn accept(
        &mut self,
        event: &DownloadEventEnvelope,
    ) -> Option<(&'static str, Uuid, Causality, serde_json::Value)> {
        if event.generation != self.generation || event.sequence <= self.sequence {
            return None;
        }
        self.sequence = event.sequence;
        match &event.event {
            DownloadEvent::TaskRemoved { task_id } => {
                self.states.remove(task_id);
                None
            }
            DownloadEvent::TaskChanged { task } => {
                let prior = self.states.get(&task.id()).copied();
                let status = task.status();
                self.remember(task.id(), status);
                if prior == Some(status) {
                    return None;
                }
                let entry = match status {
                    DownloadStatus::Completed => "completed",
                    DownloadStatus::Failed => "failed",
                    _ if prior.is_none() => "created",
                    _ => return None,
                };
                // Host-created events do not include full private headers/cookies/request snapshots.
                let id = Uuid::new_v4();
                let causality = task
                    .resolution()
                    .and_then(|resolution| resolution.extension_causality.as_ref())
                    .map_or_else(
                        || Causality::root(id),
                        |origin| Causality {
                            root_id: origin.root_id,
                            origin_plugin: Some(origin.origin_plugin.clone()),
                            handlers: origin.handlers.clone(),
                        },
                    );
                Some((
                    entry,
                    id,
                    causality,
                    serde_json::json!({"task_id":task.id(),"status":status,"generation":event.generation,"sequence":event.sequence}),
                ))
            }
            _ => None,
        }
    }
}
pub(crate) fn attach(
    service: &ExtensionService,
    engine: &nxtools_download::DownloadEngine,
    clipboard: Option<ClipboardCommitSubscription>,
) -> anyhow::Result<Vec<tokio::task::AbortHandle>> {
    let events = engine.subscribe_events()?;
    let mut transitions = DownloadEventTransitions::new(engine)?;
    let runtime = service.runtime.clone();
    let control = service.events.clone();
    let download_control = control.clone();
    let task = service.executor().spawn(async move {
        while let Ok(event) = events.recv().await {
            if let Some((entry, id, causality, context)) = transitions.accept(&event) {
                dispatch(
                    runtime.clone(),
                    download_control.clone(),
                    ("downloads", entry),
                    id,
                    causality,
                    context,
                    event.generation,
                );
            }
        }
    });
    let mut tasks = vec![task.abort_handle()];
    if let Some(subscription) = clipboard {
        let runtime = service.runtime.clone();
        let task=service.executor().spawn(async move{let mut observed_drops=0;while let Ok(committed)=subscription.receiver.recv().await{
            let drops=subscription.dropped_count();if drops>observed_drops{tracing::warn!(event="extension.clipboard.commit_drops",count=drops-observed_drops,"clipboard commit subscriber observed bounded drops");observed_drops=drops;}
            let id=Uuid::new_v4();let causality=clipboard_causality(&committed);
            dispatch(runtime.clone(),control.clone(),("clipboard","committed"),id,causality,serde_json::json!({"item_id":committed.item_id,"kind":format!("{:?}",committed.kind).to_ascii_lowercase()}),0);
        }});
        tasks.push(task.abort_handle());
    }
    Ok(tasks)
}
fn clipboard_causality(committed: &ClipboardCommitted) -> Causality {
    Causality {
        root_id: committed.causality.root_operation_id,
        origin_plugin: committed.causality.origin_plugin.clone(),
        handlers: committed.causality.invoked_handlers.clone(),
    }
}
fn dispatch(
    runtime: nxtools_extensions::ExtensionRuntime,
    control: EventControl,
    registration: (&str, &str),
    id: Uuid,
    causality: Causality,
    context: serde_json::Value,
    generation: u64,
) {
    let (feature, entry) = registration;
    let Ok(registry) = runtime.snapshot() else {
        return;
    };
    for package in registry
        .packages
        .iter()
        .filter(|package| package.enabled && package.trust.is_some())
    {
        for handler in package.manifest.handlers.iter().filter(|handler| {
            handler.feature == feature
                && handler.entry == entry
                && package.enabled_events.contains(&handler.id)
        }) {
            let admitted = control
                .guard
                .lock()
                .map(|mut guard| {
                    guard.admit(
                        true,
                        id,
                        &package.manifest.id,
                        &handler.id,
                        &causality,
                        Instant::now(),
                    )
                })
                .ok();
            if !matches!(admitted, Some(Ok(_))) {
                continue;
            }
            let runtime = runtime.clone();
            let control = control.clone();
            let package = package.manifest.id.clone();
            let handler = handler.id.clone();
            let context = context.clone();
            let causality = causality.clone();
            tokio::spawn(async move {
                let (_cancel, cancellation) = tokio::sync::watch::channel(false);
                let result = runtime
                    .invoke(InvokeRequest {
                        package_id: package.clone(),
                        handler_id: handler.clone(),
                        correlation_id: causality.root_id,
                        generation,
                        context,
                        causality,
                        cancellation,
                        batches: None,
                    })
                    .await;
                let success = result
                    .as_ref()
                    .is_ok_and(|outcome| outcome.status != "error");
                control.terminal(&package, &handler, success);
                tracing::info!(event="extension.event.terminal",event_id=%id,success,"committed extension event terminated");
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_background_engine_commit_is_observed_once_without_private_request_or_progress() {
        let root = tempfile::tempdir().expect("fixture");
        let engine = nxtools_download::DownloadEngine::open(root.path().join("downloads.db"))
            .expect("engine");
        let events = engine.subscribe_events().expect("independent events");
        let mut transitions = DownloadEventTransitions::new(&engine).expect("seed");
        let request = nxtools_download::DownloadRequest {
            source_url: "https://fixture.test/file".into(),
            save_directory: root.path().to_owned(),
            save_directory_explicit: true,
            file_name: Some("file.bin".into()),
            headers: vec![nxtools_download::DownloadRequestHeader {
                name: "Authorization".into(),
                value: "private fixture".into(),
            }],
            cookie: None,
            referrer: None,
            certificate_policy: Default::default(),
            connection_limit: Some(1),
            speed_limit_bytes_per_second: None,
            expected_total_bytes: None,
            scheduled_at_unix_ms: None,
            adaptive_segment_policy: None,
            media: None,
            category_id: None,
            category_name: None,
            duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
        };
        let id = engine
            .enqueue(request, nxtools_download::RequestOrigin::Ui)
            .expect("durable acceptance");
        let event = events.recv_blocking().expect("actual commit event");
        let (entry, _, _, context) = transitions.accept(&event).expect("new committed event");
        assert_eq!(entry, "created");
        assert_eq!(context["task_id"], serde_json::to_value(id).expect("id"));
        assert!(!context.to_string().contains("private fixture"));
        assert!(
            transitions.accept(&event).is_none(),
            "same sequence cannot replay"
        );
        let mut seeded = DownloadEventTransitions::new(&engine).expect("restart seed");
        assert!(
            seeded.accept(&event).is_none(),
            "persisted tasks not replayed as newly created"
        );
    }
    #[test]
    fn circuit_state_notifies_only_pause_and_explicit_retry_without_event_replay() {
        let control = EventControl::default();
        let mut changes = control.subscribe();
        // Real admissions establish circuit identity; spacing avoids the debounce.
        let now = Instant::now();
        for index in 0..3 {
            let id = Uuid::new_v4();
            control
                .guard
                .lock()
                .expect("guard")
                .admit(
                    true,
                    id,
                    "fixture",
                    "event",
                    &Causality::root(id),
                    now + std::time::Duration::from_secs(index),
                )
                .expect("admit");
            control.terminal("fixture", "event", false);
        }
        assert!(control.is_paused("fixture", "event"));
        assert!(changes.has_changed().expect("change"));
        changes.borrow_and_update();
        control.terminal("fixture", "event", false);
        assert!(!changes.has_changed().expect("no repeated notice"));
        control.retry("fixture", "event").expect("retry");
        assert!(!control.is_paused("fixture", "event"));
        assert!(changes.has_changed().expect("restored notice"));
    }
}
