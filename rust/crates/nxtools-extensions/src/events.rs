use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Causality {
    pub root_id: Uuid,
    #[serde(default)]
    pub origin_plugin: Option<String>,
    #[serde(default)]
    pub handlers: Vec<String>,
}
impl Causality {
    pub fn root(root_id: Uuid) -> Self {
        Self {
            root_id,
            origin_plugin: None,
            handlers: Vec::new(),
        }
    }
    pub fn enter(&self, package: &str, handler: &str) -> Result<Self, EventAdmissionError> {
        let key = format!("{package}/{handler}");
        if self.root_id.is_nil() || self.handlers.len() >= 4 || self.handlers.contains(&key) {
            return Err(EventAdmissionError::Reentry);
        }
        let mut child = self.clone();
        child.origin_plugin = Some(package.into());
        child.handlers.push(key);
        Ok(child)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventAdmissionError {
    Disabled,
    Reentry,
    Duplicate,
    CircuitOpen,
    RateLimited,
    QueueFull,
}
#[derive(Default)]
struct Circuit {
    failures: VecDeque<Instant>,
    paused: bool,
    last_admitted: Option<Instant>,
}
#[derive(Default)]
pub struct EventGuard {
    seen: HashSet<(Uuid, String)>,
    order: VecDeque<(Uuid, String)>,
    circuits: HashMap<String, Circuit>,
    pending: usize,
}
impl EventGuard {
    pub fn admit(
        &mut self,
        enabled: bool,
        event_id: Uuid,
        package: &str,
        handler: &str,
        causality: &Causality,
        now: Instant,
    ) -> Result<Causality, EventAdmissionError> {
        if !enabled {
            return Err(EventAdmissionError::Disabled);
        }
        let key = format!("{package}/{handler}");
        let child = causality.enter(package, handler)?;
        if self.seen.contains(&(event_id, key.clone())) {
            return Err(EventAdmissionError::Duplicate);
        }
        if self.pending >= 32 {
            return Err(EventAdmissionError::QueueFull);
        }
        if !self.circuits.contains_key(&key) && self.circuits.len() >= 25600 {
            return Err(EventAdmissionError::QueueFull);
        }
        let circuit = self.circuits.entry(key.clone()).or_default();
        if circuit.paused {
            return Err(EventAdmissionError::CircuitOpen);
        }
        if circuit
            .last_admitted
            .is_some_and(|last| now.saturating_duration_since(last) < Duration::from_millis(150))
        {
            return Err(EventAdmissionError::RateLimited);
        }
        circuit.last_admitted = Some(now);
        self.pending += 1;
        self.seen.insert((event_id, key.clone()));
        self.order.push_back((event_id, key));
        while self.order.len() > 4096 {
            if let Some(old) = self.order.pop_front() {
                self.seen.remove(&old);
            }
        }
        Ok(child)
    }
    pub fn terminal(&mut self, package: &str, handler: &str, success: bool, now: Instant) {
        self.pending = self.pending.saturating_sub(1);
        let Some(circuit) = self.circuits.get_mut(&format!("{package}/{handler}")) else {
            return;
        };
        while circuit
            .failures
            .front()
            .is_some_and(|time| now.saturating_duration_since(*time) > Duration::from_secs(60))
        {
            circuit.failures.pop_front();
        }
        if !success {
            circuit.failures.push_back(now);
            if circuit.failures.len() >= 3 {
                circuit.paused = true;
            }
        }
    }
    pub fn is_paused(&self, package: &str, handler: &str) -> bool {
        self.circuits
            .get(&format!("{package}/{handler}"))
            .is_some_and(|circuit| circuit.paused)
    }
    pub fn retry(&mut self, package: &str, handler: &str) {
        if let Some(circuit) = self.circuits.get_mut(&format!("{package}/{handler}")) {
            circuit.failures.clear();
            circuit.paused = false;
            circuit.last_admitted = None;
        }
    }
}
