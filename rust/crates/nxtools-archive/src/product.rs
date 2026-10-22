//! Headless product read owner. No GPUI, native handles or process control.
use nxtools_ipc::{
    PlatformBridge, PlatformBridgeError,
    archive_read::{ArchiveReadNotification, ArchiveReadReceiver},
};
use nxtools_protocol::archive_read::*;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex, Weak},
    time::Instant,
};
use uuid::Uuid;

pub struct ArchiveReadClient {
    shared: Arc<Shared>,
    receiver: ArchiveReadReceiver,
}
struct Shared {
    bridge: Arc<PlatformBridge>,
    state: Mutex<State>,
}
struct State {
    active: HashMap<Uuid, Pending>,
    retired: HashSet<Uuid>,
    recent: VecDeque<Uuid>,
}
struct Pending {
    command: ArchiveReadCommand,
    connection: u64,
    command_ids: HashSet<Uuid>,
    accepted: bool,
    negotiated: bool,
    snapshot: Option<Uuid>,
    format: Option<String>,
    next_sequence: u64,
    entries: u64,
    retained: u64,
    waiting_ack: Option<u64>,
    cancelled: bool,
    started: Instant,
    mailbox: Arc<Mutex<Mailbox>>,
}
struct Mailbox {
    batch: Option<ArchiveReadBatch>,
    terminal: Option<ArchiveReadTerminal>,
}
pub struct ArchiveReadOperation {
    id: Uuid,
    shared: Weak<Shared>,
    mailbox: Arc<Mutex<Mailbox>>,
}
impl ArchiveReadOperation {
    pub fn operation_id(&self) -> Uuid {
        self.id
    }
    /// Move one batch, never clone a whole archive. Ack only after the consumer retains it.
    pub fn take_batch(&self) -> Option<ArchiveReadBatch> {
        self.mailbox.lock().ok()?.batch.take()
    }
    pub fn terminal(&self) -> Option<ArchiveReadTerminal> {
        self.mailbox.lock().ok()?.terminal.clone()
    }
    pub fn acknowledge(&self, sequence: u64) -> Result<(), ArchiveReadError> {
        self.shared
            .upgrade()
            .ok_or(ArchiveReadError::Disconnected)?
            .control(self.id, Some(sequence))
    }
    pub fn cancel(&self) -> Result<(), ArchiveReadError> {
        self.shared
            .upgrade()
            .ok_or(ArchiveReadError::Disconnected)?
            .control(self.id, None)
    }
}
impl Drop for ArchiveReadOperation {
    fn drop(&mut self) {
        let _ = self.cancel();
    }
}
impl ArchiveReadClient {
    pub fn new(bridge: Arc<PlatformBridge>) -> Result<Self, ArchiveReadError> {
        let receiver = bridge.take_archive_read_receiver().map_err(map_bridge)?;
        Ok(Self {
            shared: Arc::new(Shared {
                bridge,
                state: Mutex::new(State {
                    active: HashMap::new(),
                    retired: HashSet::new(),
                    recent: VecDeque::new(),
                }),
            }),
            receiver,
        })
    }
    pub fn start(
        &self,
        command: ArchiveReadCommand,
    ) -> Result<ArchiveReadOperation, ArchiveReadError> {
        command.validate()?;
        if !matches!(
            command.action,
            ArchiveReadAction::Open | ArchiveReadAction::Test
        ) {
            return Err(ArchiveReadError::InvalidRequest);
        }
        let connection = self
            .shared
            .bridge
            .connection_generation()
            .ok_or(ArchiveReadError::Disconnected)?;
        let mailbox = Arc::new(Mutex::new(Mailbox {
            batch: None,
            terminal: None,
        }));
        let id = command.operation_id;
        let command_id = Uuid::new_v4();
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| ArchiveReadError::IoFailure)?;
        if state.active.contains_key(&id) || state.retired.contains(&id) || state.active.len() >= 2
        {
            return Err(ArchiveReadError::Busy);
        }
        state.active.insert(
            id,
            Pending {
                command: command.clone(),
                connection,
                command_ids: HashSet::from([command_id]),
                accepted: false,
                negotiated: false,
                snapshot: None,
                format: None,
                next_sequence: 0,
                entries: 0,
                retained: 0,
                waiting_ack: None,
                cancelled: false,
                started: Instant::now(),
                mailbox: mailbox.clone(),
            },
        );
        if let Err(error) = self
            .shared
            .bridge
            .send_archive_read(command, command_id, connection)
        {
            state.active.remove(&id);
            return Err(map_bridge(error));
        }
        Ok(ArchiveReadOperation {
            id,
            shared: Arc::downgrade(&self.shared),
            mailbox,
        })
    }
    /// Drive on a background task, not the UI/app thread. One notification at a time.
    pub async fn poll(&self) -> Result<(), ArchiveReadError> {
        let notification = self.receiver.recv().await.map_err(map_bridge)?;
        self.apply(notification)
    }
    pub fn queued_events(&self) -> usize {
        self.receiver.queued()
    }
    fn apply(&self, notification: ArchiveReadNotification) -> Result<(), ArchiveReadError> {
        let mut state = self
            .shared
            .state
            .lock()
            .map_err(|_| ArchiveReadError::IoFailure)?;
        match notification {
            ArchiveReadNotification::Connected { .. } => {}
            ArchiveReadNotification::Disconnected {
                connection_generation,
            } => {
                let ids = state
                    .active
                    .iter()
                    .filter(|(_, p)| p.connection == connection_generation)
                    .map(|(id, _)| *id)
                    .collect::<Vec<_>>();
                for id in ids {
                    finish_local(&mut state, id, ArchiveReadError::Disconnected);
                }
            }
            ArchiveReadNotification::Overflow => {
                let ids = state.active.keys().copied().collect::<Vec<_>>();
                for id in ids {
                    if let Some(pending) = state.active.get(&id) {
                        let _ = self.shared.send_control(pending, None);
                    }
                    finish_local(&mut state, id, ArchiveReadError::ConsumerQueueFull);
                }
            }
            ArchiveReadNotification::Receipt {
                connection_generation,
                command_id,
                result,
            } => {
                let id = state
                    .active
                    .iter()
                    .find(|(_, p)| {
                        p.connection == connection_generation && p.command_ids.contains(&command_id)
                    })
                    .map(|(id, _)| *id);
                if let Some(id) = id {
                    let pending = state.active.get_mut(&id).expect("found pending owner");
                    pending.command_ids.remove(&command_id);
                    let receipt = result
                        .data
                        .as_deref()
                        .and_then(|v| serde_json::from_str::<ArchiveReadReceipt>(v).ok());
                    if receipt.as_ref().is_none_or(|r| {
                        r.accepted != result.success || r.accepted && r.code.is_some()
                    }) {
                        let _ = self.shared.send_control(pending, None);
                        finish_local(&mut state, id, ArchiveReadError::ProtocolViolation);
                    }
                    // A typed rejected start has its own unique terminal. Do not invent a second one.
                }
            }
            ArchiveReadNotification::Event {
                connection_generation,
                event,
            } => {
                let Some(pending) = state.active.get_mut(&event.operation_id) else {
                    return Ok(());
                };
                if pending.connection != connection_generation
                    || event.session_id != pending.command.session_id
                    || event.generation != pending.command.generation
                {
                    return Ok(());
                }
                let id = event.operation_id;
                let result = apply_event(pending, event);
                match result {
                    Ok(Some(terminal)) => {
                        let pending = state.active.remove(&id).expect("current operation");
                        if let Ok(mut mailbox) = pending.mailbox.lock() {
                            mailbox.batch = None;
                            mailbox.terminal = Some(terminal.clone());
                        }
                        diagnostic(&pending, "terminal", terminal.code);
                        retire(&mut state, id);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        let _ = self.shared.send_control(pending, None);
                        finish_local(&mut state, id, error);
                    }
                }
            }
        }
        Ok(())
    }
}
impl Drop for ArchiveReadClient {
    fn drop(&mut self) {
        if let Ok(mut state) = self.shared.state.lock() {
            let ids = state.active.keys().copied().collect::<Vec<_>>();
            for id in ids {
                if let Some(pending) = state.active.get(&id) {
                    let _ = self.shared.send_control(pending, None);
                }
                finish_local(&mut state, id, ArchiveReadError::Cancelled);
            }
        }
    }
}
impl Shared {
    fn control(&self, id: Uuid, sequence: Option<u64>) -> Result<(), ArchiveReadError> {
        let mut state = self.state.lock().map_err(|_| ArchiveReadError::IoFailure)?;
        let Some(pending) = state.active.get_mut(&id) else {
            return Ok(());
        };
        if self.bridge.connection_generation() != Some(pending.connection) {
            return Err(ArchiveReadError::Disconnected);
        }
        if let Some(sequence) = sequence {
            if pending.waiting_ack != Some(sequence) {
                return Err(ArchiveReadError::ProtocolViolation);
            }
            if pending
                .mailbox
                .lock()
                .map_err(|_| ArchiveReadError::IoFailure)?
                .batch
                .is_some()
            {
                return Err(ArchiveReadError::Busy);
            }
        } else if pending.cancelled {
            return Ok(());
        }
        let command_id = self.send_control(pending, sequence)?;
        pending.command_ids.insert(command_id);
        if sequence.is_some() {
            pending.waiting_ack = None;
        } else {
            pending.cancelled = true;
        }
        Ok(())
    }
    fn send_control(
        &self,
        pending: &Pending,
        sequence: Option<u64>,
    ) -> Result<Uuid, ArchiveReadError> {
        let command = ArchiveReadCommand {
            version: pending.command.version,
            operation_id: pending.command.operation_id,
            session_id: pending.command.session_id,
            generation: pending.command.generation,
            action: if sequence.is_some() {
                ArchiveReadAction::Ack
            } else {
                ArchiveReadAction::Cancel
            },
            path: None,
            codepage: 0,
            limits: None,
            sequence,
        };
        self.bridge
            .send_archive_read(command, Uuid::new_v4(), pending.connection)
            .map_err(map_bridge)
    }
}
fn apply_event(
    p: &mut Pending,
    event: ArchiveReadEvent,
) -> Result<Option<ArchiveReadTerminal>, ArchiveReadError> {
    if event.version != p.command.version {
        return Err(ArchiveReadError::IncompatibleVersion);
    }
    let no_payload = event.batch.is_none() && event.terminal.is_none();
    match event.kind {
        ArchiveReadEventKind::Accepted
            if no_payload && event.capabilities.is_empty() && !p.accepted =>
        {
            p.accepted = true;
            diagnostic(p, "accepted", None);
        }
        ArchiveReadEventKind::Negotiated if no_payload && p.accepted && !p.negotiated => {
            if event.capabilities.len() > 16
                || archive_read_capabilities(p.command.version)
                    .iter()
                    .any(|c| !event.capabilities.iter().any(|v| v == c))
            {
                return Err(ArchiveReadError::MissingCapability);
            }
            p.negotiated = true;
            diagnostic(p, "negotiated", None);
        }
        ArchiveReadEventKind::Batch
            if p.negotiated
                && event.terminal.is_none()
                && event.capabilities.is_empty()
                && p.command.action == ArchiveReadAction::Open =>
        {
            let batch = event.batch.ok_or(ArchiveReadError::ProtocolViolation)?;
            let bytes = batch.validate_for(p.command.version)? as u64;
            if p.waiting_ack.is_some()
                || batch.sequence != p.next_sequence
                || p.snapshot.is_some_and(|id| id != batch.snapshot_id)
                || p.format
                    .as_ref()
                    .is_some_and(|format| format != &batch.format)
                || batch
                    .entries
                    .iter()
                    .enumerate()
                    .any(|(i, e)| e.ordinal != p.entries + i as u64)
            {
                return Err(ArchiveReadError::ProtocolViolation);
            }
            let limits = p.command.limits.unwrap_or_default();
            if p.entries + batch.entries.len() as u64 > limits.max_entries
                || p.retained + bytes > limits.max_retained_bytes
            {
                return Err(ArchiveReadError::MetadataBudget);
            }
            let mut mailbox = p.mailbox.lock().map_err(|_| ArchiveReadError::IoFailure)?;
            if mailbox.batch.is_some() {
                return Err(ArchiveReadError::ConsumerQueueFull);
            }
            p.snapshot = Some(batch.snapshot_id);
            p.format = Some(batch.format.clone());
            p.entries += batch.entries.len() as u64;
            p.retained += bytes;
            p.waiting_ack = Some(batch.sequence);
            p.next_sequence += 1;
            mailbox.batch = Some(batch);
        }
        ArchiveReadEventKind::Terminal
            if event.batch.is_none() && event.capabilities.is_empty() =>
        {
            let terminal = event.terminal.ok_or(ArchiveReadError::ProtocolViolation)?;
            if let Some(browser) = &terminal.browser {
                browser.validate()?;
                // Context is accepted only on the explicit capability-bound browser read.
                if p.command.version != ARCHIVE_READ_LARGE_VERSION
                    || !p.command.limits.unwrap_or_default().browser_context
                    || p.command.action != ArchiveReadAction::Open
                    || terminal.status != ArchiveReadStatus::Succeeded
                    || p.snapshot.is_some_and(|id| id != browser.snapshot_id)
                    || p.format
                        .as_ref()
                        .is_some_and(|format| format != &browser.format)
                {
                    return Err(ArchiveReadError::ProtocolViolation);
                }
                let bytes = serde_json::to_vec(browser)
                    .map_err(|_| ArchiveReadError::ProtocolViolation)?
                    .len() as u64;
                if p.retained + bytes > p.command.limits.unwrap_or_default().max_retained_bytes {
                    return Err(ArchiveReadError::MetadataBudget);
                }
                p.retained += bytes;
            }
            if terminal.status == ArchiveReadStatus::Succeeded {
                if !p.negotiated
                    || terminal.code.is_some()
                    || p.command.limits.unwrap_or_default().browser_context
                        && p.command.action == ArchiveReadAction::Open
                        && terminal.browser.is_none()
                    || !terminal.cleanup_complete
                    || p.waiting_ack.is_some()
                    || terminal.entry_count != p.entries
                    || terminal.batch_count != p.next_sequence
                {
                    return Err(ArchiveReadError::ProtocolViolation);
                }
                if p.command.version == ARCHIVE_READ_LARGE_VERSION {
                    let stats = terminal
                        .transfer
                        .as_ref()
                        .ok_or(ArchiveReadError::ProtocolViolation)?;
                    let limits = p.command.limits.unwrap_or_default();
                    if stats.private_bytes > limits.max_transport_bytes
                        || stats.received_bytes > limits.max_retained_bytes
                        || stats.received_bytes < p.retained
                        || stats.max_batch_entries > ARCHIVE_READ_LARGE_BATCH_ENTRIES as u64
                        || stats.max_batch_bytes > ARCHIVE_READ_LARGE_BATCH_BYTES as u64
                        || stats.max_inflight_batches > 1
                    {
                        return Err(ArchiveReadError::MetadataBudget);
                    }
                }
            } else if terminal.code.is_none() {
                return Err(ArchiveReadError::ProtocolViolation);
            }
            return Ok(Some(terminal));
        }
        _ => return Err(ArchiveReadError::ProtocolViolation),
    }
    Ok(None)
}
fn retire(state: &mut State, id: Uuid) {
    state.retired.insert(id);
    state.recent.push_back(id);
    if state.recent.len() > 4096
        && let Some(old) = state.recent.pop_front()
    {
        state.retired.remove(&old);
    }
}
fn finish_local(state: &mut State, id: Uuid, code: ArchiveReadError) {
    if let Some(p) = state.active.remove(&id) {
        let status = if code == ArchiveReadError::Disconnected {
            ArchiveReadStatus::Disconnected
        } else if code == ArchiveReadError::Cancelled {
            ArchiveReadStatus::Cancelled
        } else {
            ArchiveReadStatus::Failed
        };
        if let Ok(mut mailbox) = p.mailbox.lock() {
            mailbox.batch = None;
            mailbox.terminal = Some(ArchiveReadTerminal {
                status,
                code: Some(code),
                files_verified: 0,
                bytes_verified: 0,
                entry_count: p.entries,
                batch_count: p.next_sequence,
                elapsed_ms: p.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                native_error_code: None,
                cleanup_complete: false,
                transfer: None,
                browser: None,
            });
        }
        diagnostic(&p, "terminal", Some(code));
        retire(state, id);
    }
}
fn diagnostic(p: &Pending, stage: &'static str, code: Option<ArchiveReadError>) {
    tracing::info!(event="archive.read.lifecycle",correlation_id=%p.command.operation_id,stage,?code,elapsed_ms=p.started.elapsed().as_millis() as u64,entries=p.entries,"archive read lifecycle");
}
fn map_bridge(error: PlatformBridgeError) -> ArchiveReadError {
    match error {
        PlatformBridgeError::Busy | PlatformBridgeError::DuplicateCommandId => {
            ArchiveReadError::Busy
        }
        PlatformBridgeError::Disconnected => ArchiveReadError::Disconnected,
    }
}
