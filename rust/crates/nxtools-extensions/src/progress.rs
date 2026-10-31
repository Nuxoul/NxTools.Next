//! Bounded, UI-only plugin-reported progress. Never changes invocation deadlines or results.
use crate::{InvocationError, InvocationIdentity};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgressFrame {
    pub protocol_version: u16,
    pub op: String,
    pub session_id: Uuid,
    pub run_id: Uuid,
    pub sequence: u64,
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub message: Option<String>,
}
impl ProgressFrame {
    pub(crate) fn validate(&self) -> Result<(), InvocationError> {
        const MAX_INTEGER: u64 = (1 << 53) - 1;
        let numbers = match (self.completed, self.total) {
            (None, None) => true,
            (Some(completed), Some(total)) => {
                total > 0 && completed <= total && total <= MAX_INTEGER
            }
            _ => false,
        };
        if self.protocol_version != 2
            || self.op != "progress"
            || self.sequence == 0
            || !numbers
            || self.message.as_ref().is_some_and(|text| {
                text.len() > 512
                    || text
                        .chars()
                        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\t'))
            })
        {
            return Err(InvocationError::InvalidFrame);
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvocationProgress {
    pub identity: InvocationIdentity,
    pub sequence: u64,
    pub completed: Option<u64>,
    pub total: Option<u64>,
    pub message: Option<String>,
}
impl InvocationProgress {
    pub fn percentage(&self) -> Option<f32> {
        self.completed
            .zip(self.total)
            .map(|(completed, total)| (completed as f64 / total as f64 * 100.0) as f32)
    }
}
struct Entry {
    latest: InvocationProgress,
    published: Option<InvocationProgress>,
    last_publish: Option<Instant>,
}
#[derive(Clone)]
pub(crate) struct ProgressHub {
    entries: Arc<Mutex<HashMap<Uuid, Entry>>>,
    changed: watch::Sender<u64>,
}
impl Default for ProgressHub {
    fn default() -> Self {
        Self {
            entries: Default::default(),
            changed: watch::channel(0).0,
        }
    }
}
impl ProgressHub {
    pub(crate) fn subscribe(&self) -> watch::Receiver<u64> {
        self.changed.subscribe()
    }
    pub(crate) fn get(&self, correlation: Uuid) -> Option<InvocationProgress> {
        self.entries
            .lock()
            .ok()?
            .values()
            .find(|entry| entry.latest.identity.correlation_id == correlation)?
            .published
            .clone()
    }
    pub(crate) fn report(
        &self,
        identity: &InvocationIdentity,
        frame: ProgressFrame,
    ) -> Result<(), InvocationError> {
        let mut entries = self.entries.lock().map_err(|_| InvocationError::Retired)?;
        if entries.len() >= 36 && !entries.contains_key(&identity.run_id) {
            return Err(InvocationError::ResultLimit);
        }
        let latest = InvocationProgress {
            identity: identity.clone(),
            sequence: frame.sequence,
            completed: frame.completed,
            total: frame.total,
            message: frame.message,
        };
        entries
            .entry(identity.run_id)
            .and_modify(|entry| entry.latest = latest.clone())
            .or_insert(Entry {
                latest,
                published: None,
                last_publish: None,
            });
        drop(entries);
        self.flush(identity.run_id);
        Ok(())
    }
    pub(crate) fn flush(&self, run: Uuid) {
        let publish = self.entries.lock().is_ok_and(|mut entries| {
            let Some(entry) = entries.get_mut(&run) else {
                return false;
            };
            if entry
                .last_publish
                .is_some_and(|last| last.elapsed() < Duration::from_millis(250))
                || entry.published.as_ref() == Some(&entry.latest)
            {
                return false;
            }
            entry.published = Some(entry.latest.clone());
            entry.last_publish = Some(Instant::now());
            true
        });
        if publish {
            self.changed
                .send_modify(|revision| *revision = revision.wrapping_add(1));
        }
    }
    pub(crate) fn retire(&self, run: Uuid) {
        if self
            .entries
            .lock()
            .is_ok_and(|mut entries| entries.remove(&run).is_some())
        {
            self.changed
                .send_modify(|revision| *revision = revision.wrapping_add(1));
        }
    }
}
