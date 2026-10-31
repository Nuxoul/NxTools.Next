//! Bounded application admission only; original C# runner owns processes/cleanup.
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;
const MAX_RUNNING: usize = 2;
const MAX_QUEUED: usize = 32;
const MAX_MEMORY: u64 = 2 << 30;
#[derive(Clone)]
pub(super) struct Resources {
    pub sources: Vec<String>,
    pub output: Option<String>,
    pub memory: u64,
}
#[derive(Debug, PartialEq, Eq)]
pub(super) enum AdmissionError {
    Duplicate,
    Full,
    Memory,
}
#[derive(Default)]
pub(super) struct JobQueue {
    queued: VecDeque<(Uuid, Resources)>,
    running: HashMap<Uuid, Resources>,
}
fn key(value: &str) -> String {
    value
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase()
}
fn overlap(a: &str, b: &str) -> bool {
    let a = key(a);
    let b = key(b);
    a == b || a.starts_with(&(b.clone() + "/")) || b.starts_with(&(a + "/"))
}
impl JobQueue {
    pub fn can_fit(&self, count: usize) -> bool {
        count > 0 && self.queued.len().saturating_add(count) <= MAX_QUEUED
    }
    pub fn enqueue(&mut self, id: Uuid, resources: Resources) -> Result<(), AdmissionError> {
        if self.running.contains_key(&id) || self.queued.iter().any(|(prior, _)| *prior == id) {
            return Err(AdmissionError::Duplicate);
        }
        if self.queued.len() >= MAX_QUEUED {
            return Err(AdmissionError::Full);
        }
        if resources.memory == 0 || resources.memory > 1 << 30 {
            return Err(AdmissionError::Memory);
        }
        self.queued.push_back((id, resources));
        Ok(())
    }
    pub fn next(&mut self) -> Option<Uuid> {
        let (_, next) = self.queued.front()?;
        if self.running.len() >= MAX_RUNNING
            || self.running.values().map(|r| r.memory).sum::<u64>() + next.memory > MAX_MEMORY
        {
            return None;
        }
        if self.running.values().any(|running| {
            running
                .sources
                .iter()
                .any(|a| next.sources.iter().any(|b| overlap(a, b)))
                || running
                    .output
                    .as_ref()
                    .is_some_and(|a| next.output.as_ref().is_some_and(|b| overlap(a, b)))
                || running
                    .output
                    .as_ref()
                    .is_some_and(|a| next.sources.iter().any(|b| overlap(a, b)))
                || next
                    .output
                    .as_ref()
                    .is_some_and(|a| running.sources.iter().any(|b| overlap(a, b)))
        }) {
            return None;
        }
        let (id, resources) = self.queued.pop_front().expect("observed head");
        self.running.insert(id, resources);
        Some(id)
    }
    pub fn cancel_queued(&mut self, id: Uuid) -> bool {
        let before = self.queued.len();
        self.queued.retain(|(prior, _)| *prior != id);
        before != self.queued.len()
    }
    pub fn finish(&mut self, id: Uuid, cleanup_complete: bool) -> bool {
        cleanup_complete && self.running.remove(&id).is_some()
    }
    pub fn is_running(&self, id: Uuid) -> bool {
        self.running.contains_key(&id)
    }
    pub fn queued_count(&self) -> usize {
        self.queued.len()
    }
}
#[cfg(test)]
#[path = "queue_tests.rs"]
mod tests;
