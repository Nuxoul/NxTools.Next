//! Dedicated bounded archive event delivery on the existing platform connection/router.
use super::*;
use nxtools_protocol::archive_read::{
    ARCHIVE_READ_EVENT_CAPACITY, ArchiveReadAction, ArchiveReadCommand, ArchiveReadEvent,
    ArchiveReadEventKind,
};
use std::collections::HashSet;

const READ_RESERVATION_CAPACITY: usize = 2;

pub(super) struct ReadReservation {
    operation: uuid::Uuid,
    session: uuid::Uuid,
    generation: u64,
    version: u32,
    start_id: uuid::Uuid,
    start_dequeued: bool,
    cancel_id: Option<uuid::Uuid>,
    cancel: Option<Envelope>,
    admitted: Instant,
}
impl ReadReservation {
    fn matches(&self, command: &ArchiveReadCommand) -> bool {
        self.operation == command.operation_id
            && self.session == command.session_id
            && self.generation == command.generation
            && self.version == command.version
    }
}
impl ActiveConnection {
    pub(super) fn note_archive_start(&mut self, id: uuid::Uuid) {
        if let Some(read) = self
            .archive_reads
            .iter_mut()
            .find(|read| read.start_id == id)
        {
            read.start_dequeued = true;
        }
    }
    pub(super) fn take_archive_cancel(&mut self) -> Option<Envelope> {
        let read = self
            .archive_reads
            .iter_mut()
            .find(|read| read.start_dequeued && read.cancel.is_some())?;
        let envelope = read.cancel.take()?;
        self.cancel_turn = false;
        tracing::info!(event="archive.read.cancel", correlation_id=%read.operation, connection_generation=self.generation,
            stage="dequeued", elapsed_ms=read.admitted.elapsed().as_millis() as u64, "archive cancellation delivery");
        Some(envelope)
    }
}
impl PlatformServerEndpoint {
    /// Only a matching remote terminal with actual cleanup can release the original slot.
    pub(super) fn observe_archive_cleanup(&self, notification: &PlatformNotification) {
        let PlatformNotification::Event(PlatformEvent::ArchiveRead(event)) = notification else {
            return;
        };
        if !matches!(event.version, 1 | 2)
            || event.kind != ArchiveReadEventKind::Terminal
            || event.batch.is_some()
            || !event.capabilities.is_empty()
            || !event
                .terminal
                .as_ref()
                .is_some_and(|terminal| terminal.cleanup_complete)
        {
            return;
        }
        if let Ok(mut router) = self.router.lock() {
            let cancelled = router.active.as_mut().and_then(|active| {
                let index = active.archive_reads.iter().position(|read| {
                    read.operation == event.operation_id
                        && read.session == event.session_id
                        && read.generation == event.generation
                        && read.version == event.version
                })?;
                active.archive_reads.remove(index).cancel_id
            });
            if let Some(id) = cancelled {
                router.pending.remove(&id);
                if let Ok(mut ids) = self.archive.command_ids.lock() {
                    ids.remove(&id);
                }
            }
        }
    }
}
#[derive(Debug, Clone)]
pub enum ArchiveReadNotification {
    Connected {
        connection_generation: u64,
    },
    Disconnected {
        connection_generation: u64,
    },
    Event {
        connection_generation: u64,
        event: ArchiveReadEvent,
    },
    Receipt {
        connection_generation: u64,
        command_id: uuid::Uuid,
        result: PlatformCommandResult,
    },
    Overflow,
}
pub struct ArchiveReadReceiver {
    receiver: Receiver<ArchiveReadNotification>,
    wake: Receiver<()>,
    overflow: Arc<AtomicBool>,
    epoch: Arc<Mutex<()>>,
    #[cfg(test)]
    recovery_hook: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    #[cfg(test)]
    drain_hook: Mutex<Option<Box<dyn FnOnce() + Send>>>,
}
impl ArchiveReadReceiver {
    pub async fn recv(&self) -> Result<ArchiveReadNotification, PlatformBridgeError> {
        loop {
            if self.recover_overflow()? {
                return Ok(ArchiveReadNotification::Overflow);
            }
            tokio::select! {
                biased;
                wake = self.wake.recv() => {
                    wake.map_err(|_| PlatformBridgeError::Disconnected)?;
                    // Re-enter the SAME synchronized boundary, never reset independently.
                }
                value = self.receiver.recv() => {
                    let value = value.map_err(|_| PlatformBridgeError::Disconnected)?;
                    if self.recover_overflow()? { return Ok(ArchiveReadNotification::Overflow); }
                    return Ok(value);
                }
            }
        }
    }
    fn recover_overflow(&self) -> Result<bool, PlatformBridgeError> {
        if !self.overflow.load(Ordering::Acquire) {
            return Ok(false);
        }
        #[cfg(test)]
        self.run_recovery_hook();
        let _epoch = self
            .epoch
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        if !self.overflow.swap(false, Ordering::AcqRel) {
            return Ok(false);
        }
        #[cfg(test)]
        if let Some(hook) = self.drain_hook.lock().expect("drain ordering hook").take() {
            hook();
        }
        while self.receiver.try_recv().is_ok() {}
        while self.wake.try_recv().is_ok() {}
        // Releasing epoch is the new-admission boundary. A later emit cannot be erased here.
        Ok(true)
    }
    #[cfg(test)]
    fn run_recovery_hook(&self) {
        let hook = self
            .recovery_hook
            .lock()
            .expect("test ordering hook")
            .take();
        if let Some(hook) = hook {
            hook();
        }
    }
    pub fn queued(&self) -> usize {
        self.receiver.len()
    }
}
pub(super) struct ArchiveTransport {
    sender: Sender<ArchiveReadNotification>,
    receiver: Mutex<Option<ArchiveReadReceiver>>,
    wake: Sender<()>,
    overflow: Arc<AtomicBool>,
    epoch: Arc<Mutex<()>>,
    command_ids: Mutex<HashSet<uuid::Uuid>>,
    pub(super) generation: AtomicU64,
    claimed: AtomicBool,
}
impl ArchiveTransport {
    pub(super) fn new() -> Self {
        let (sender, receiver) = async_channel::bounded(ARCHIVE_READ_EVENT_CAPACITY);
        let (wake, wake_receiver) = async_channel::bounded(1);
        let overflow = Arc::new(AtomicBool::new(false));
        let epoch = Arc::new(Mutex::new(()));
        Self {
            sender,
            receiver: Mutex::new(Some(ArchiveReadReceiver {
                receiver,
                wake: wake_receiver,
                overflow: overflow.clone(),
                epoch: epoch.clone(),
                #[cfg(test)]
                recovery_hook: Mutex::new(None),
                #[cfg(test)]
                drain_hook: Mutex::new(None),
            })),
            wake,
            overflow,
            epoch,
            command_ids: Mutex::new(HashSet::new()),
            generation: AtomicU64::new(0),
            claimed: AtomicBool::new(false),
        }
    }
    fn emit(&self, value: ArchiveReadNotification) {
        if !self.claimed.load(Ordering::Acquire) {
            return;
        }
        let Ok(_epoch) = self.epoch.lock() else {
            return;
        };
        if self.sender.try_send(value).is_err() {
            self.overflow.store(true, Ordering::Release);
            let _ = self.wake.try_send(());
        }
    }
    pub(super) fn intercept(&self, notification: &PlatformNotification) -> bool {
        let generation = self.generation.load(Ordering::Acquire);
        match notification {
            PlatformNotification::Connected => self.emit(ArchiveReadNotification::Connected {
                connection_generation: generation,
            }),
            PlatformNotification::Disconnected { generation, .. } => {
                if let Ok(mut ids) = self.command_ids.lock() {
                    ids.clear();
                }
                self.emit(ArchiveReadNotification::Disconnected {
                    connection_generation: *generation,
                });
            }
            PlatformNotification::Event(PlatformEvent::ArchiveRead(event)) => {
                self.emit(ArchiveReadNotification::Event {
                    connection_generation: generation,
                    event: event.clone(),
                });
                return true;
            }
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } if self
                .command_ids
                .lock()
                .ok()
                .is_some_and(|mut ids| ids.remove(correlation_id)) =>
            {
                self.emit(ArchiveReadNotification::Receipt {
                    connection_generation: generation,
                    command_id: *correlation_id,
                    result: result.clone(),
                });
                return true;
            }
            _ => {}
        }
        false
    }
}
impl PlatformBridge {
    /// Exactly one headless archive owner; never competes with the UI notification receiver.
    pub fn take_archive_read_receiver(&self) -> Result<ArchiveReadReceiver, PlatformBridgeError> {
        let receiver = self
            .archive
            .receiver
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?
            .take()
            .ok_or(PlatformBridgeError::DuplicateCommandId)?;
        self.archive.claimed.store(true, Ordering::Release);
        Ok(receiver)
    }
    pub fn connection_generation(&self) -> Option<u64> {
        self.router
            .lock()
            .ok()
            .and_then(|router| router.active.as_ref().map(|active| active.generation))
    }
    pub fn send_archive_read(
        &self,
        command: ArchiveReadCommand,
        command_id: uuid::Uuid,
        generation: u64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        let active = router
            .active
            .as_mut()
            .filter(|active| active.generation == generation)
            .ok_or(PlatformBridgeError::Disconnected)?;
        let start = matches!(
            command.action,
            ArchiveReadAction::Open | ArchiveReadAction::Test
        );
        let cancel = command.action == ArchiveReadAction::Cancel;
        if start {
            if active.archive_reads.len() >= READ_RESERVATION_CAPACITY
                || active
                    .archive_reads
                    .iter()
                    .any(|read| read.operation == command.operation_id)
            {
                return Err(PlatformBridgeError::Busy);
            }
        } else {
            let read = active
                .archive_reads
                .iter()
                .find(|read| read.matches(&command))
                .ok_or(PlatformBridgeError::Disconnected)?;
            if cancel && let Some(id) = read.cancel_id {
                return Ok(id);
            }
        }
        if router.pending.contains_key(&command_id) {
            return Err(PlatformBridgeError::DuplicateCommandId);
        }
        let mut ids = self
            .archive
            .command_ids
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        // Cancel owns one already-reserved slot/read, independent of ordinary receipt pressure.
        if !cancel && ids.len() >= 32 {
            return Err(PlatformBridgeError::Busy);
        }
        if !ids.insert(command_id) {
            return Err(PlatformBridgeError::DuplicateCommandId);
        }
        drop(ids);
        let browser = command.action == ArchiveReadAction::Open
            && command.limits.is_some_and(|limits| limits.browser_context);
        let wire = if browser {
            PlatformCommand::ArchiveBrowserRead(command.clone())
        } else {
            PlatformCommand::ArchiveRead(command.clone())
        };
        let mut envelope = Envelope::new(Message::PlatformCommand(wire));
        envelope.id = command_id;
        envelope.correlation_id = Some(command.operation_id);
        if cancel {
            router.pending.insert(
                command_id,
                PendingCommand {
                    generation,
                    accepted_at: Instant::now(),
                    timeout: super::PENDING_COMMAND_TIMEOUT,
                    summon_reply: None,
                },
            );
            let active = router.active.as_mut().expect("original connection locked");
            let read = active
                .archive_reads
                .iter_mut()
                .find(|read| read.matches(&command))
                .expect("reserved read");
            read.cancel_id = Some(command_id);
            read.cancel = Some(envelope);
            let _ = active.command_wake.try_send(());
            tracing::info!(event="archive.read.cancel", correlation_id=%command.operation_id, connection_generation=generation,
                stage="retained", "archive cancellation retained");
            return Ok(command_id);
        }
        let result = router.enqueue(envelope, None).map_err(|error| match error {
            QueueAdmissionError::Full => PlatformBridgeError::Busy,
            QueueAdmissionError::Disconnected | QueueAdmissionError::Closed => {
                PlatformBridgeError::Disconnected
            }
        });
        if result.is_ok() && start {
            router
                .active
                .as_mut()
                .expect("original connection locked")
                .archive_reads
                .push(ReadReservation {
                    operation: command.operation_id,
                    session: command.session_id,
                    generation: command.generation,
                    version: command.version,
                    start_id: command_id,
                    start_dequeued: false,
                    cancel_id: None,
                    cancel: None,
                    admitted: Instant::now(),
                });
        }
        if result.is_err()
            && let Ok(mut ids) = self.archive.command_ids.lock()
        {
            ids.remove(&command_id);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxtools_protocol::archive_read::{ARCHIVE_READ_VERSION, ArchiveReadAction};
    #[test]
    fn archive_send_is_bound_to_original_connection_before_pending_admission() {
        let (bridge, endpoint) = platform_bridge(8);
        endpoint.set_connected_for_contract_test(true);
        let original = bridge.connection_generation().expect("first connection");
        let _replacement = endpoint.open_connection_for_contract_test();
        let command = ArchiveReadCommand {
            version: ARCHIVE_READ_VERSION,
            operation_id: uuid::Uuid::new_v4(),
            session_id: uuid::Uuid::new_v4(),
            generation: 1,
            action: ArchiveReadAction::Open,
            path: Some("C:\\owned\\literal.zip".into()),
            codepage: 0,
            limits: None,
            sequence: None,
        };
        assert_eq!(
            bridge.send_archive_read(command, uuid::Uuid::new_v4(), original),
            Err(PlatformBridgeError::Disconnected)
        );
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        assert!(bridge.router.lock().expect("router").pending.is_empty());
    }
}

#[cfg(test)]
mod overflow_epoch_tests {
    use super::*;

    fn fill_old_epoch(transport: &ArchiveTransport) {
        for _ in 0..=ARCHIVE_READ_EVENT_CAPACITY {
            transport.emit(ArchiveReadNotification::Connected {
                connection_generation: 1,
            });
        }
    }
    #[tokio::test]
    async fn overflow_epoch_recovery_absorbs_emit_in_actual_reset_drain_window() {
        let (bridge, _endpoint) = platform_bridge(1);
        let receiver = bridge
            .take_archive_read_receiver()
            .expect("production receiver");
        fill_old_epoch(&bridge.archive);
        let producer = bridge.archive.clone();
        *receiver.recovery_hook.lock().expect("hook") = Some(Box::new(move || {
            producer.emit(ArchiveReadNotification::Connected {
                connection_generation: 1,
            });
        }));
        assert!(matches!(
            receiver.recv().await,
            Ok(ArchiveReadNotification::Overflow)
        ));
        assert_eq!(receiver.queued(), 0);
        bridge.archive.emit(ArchiveReadNotification::Connected {
            connection_generation: 2,
        });
        assert!(
            matches!(
                receiver.recv().await,
                Ok(ArchiveReadNotification::Connected {
                    connection_generation: 2
                })
            ),
            "same recovery epoch must not leave an old flag or wake to poison new admission"
        );
        fill_old_epoch(&bridge.archive);
        assert!(
            matches!(receiver.recv().await, Ok(ArchiveReadNotification::Overflow)),
            "a genuinely later overflow must still report"
        );
    }
}

#[cfg(test)]
mod waiting_epoch_tests {
    use super::*;
    use std::future::Future;
    #[tokio::test]
    async fn overflow_epoch_waiting_select_uses_same_recovery_and_preserves_new_overflow() {
        let (bridge, _endpoint) = platform_bridge(1);
        let receiver = bridge.take_archive_read_receiver().expect("receiver");
        let waiting = receiver.recv();
        tokio::pin!(waiting);
        assert!(matches!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(waiting.as_mut().poll(cx))).await,
            std::task::Poll::Pending
        ));
        for _ in 0..=ARCHIVE_READ_EVENT_CAPACITY {
            bridge.archive.emit(ArchiveReadNotification::Connected {
                connection_generation: 1,
            });
        }
        let producer = bridge.archive.clone();
        *receiver.recovery_hook.lock().expect("hook") = Some(Box::new(move || {
            producer.emit(ArchiveReadNotification::Connected {
                connection_generation: 1,
            })
        }));
        assert!(matches!(
            waiting.await,
            Ok(ArchiveReadNotification::Overflow)
        ));
        bridge.archive.emit(ArchiveReadNotification::Disconnected {
            connection_generation: 2,
        });
        assert!(matches!(
            receiver.recv().await,
            Ok(ArchiveReadNotification::Disconnected {
                connection_generation: 2
            })
        ));
        for _ in 0..=ARCHIVE_READ_EVENT_CAPACITY {
            bridge.archive.emit(ArchiveReadNotification::Connected {
                connection_generation: 3,
            });
        }
        assert!(matches!(
            receiver.recv().await,
            Ok(ArchiveReadNotification::Overflow)
        ));
    }
}

#[cfg(test)]
mod concurrent_epoch_tests {
    use super::*;
    use std::sync::Barrier;
    #[tokio::test]
    async fn overflow_epoch_producer_cannot_cross_locked_reset_drain_boundary() {
        let (bridge, _endpoint) = platform_bridge(1);
        let receiver = bridge.take_archive_read_receiver().expect("receiver");
        for _ in 0..=ARCHIVE_READ_EVENT_CAPACITY {
            bridge.archive.emit(ArchiveReadNotification::Connected {
                connection_generation: 1,
            });
        }
        let entered = Arc::new(Barrier::new(2));
        let attempted = Arc::new(Barrier::new(2));
        let producer = bridge.archive.clone();
        let producer_entered = entered.clone();
        let producer_attempted = attempted.clone();
        let thread = std::thread::spawn(move || {
            producer_entered.wait();
            assert!(
                matches!(
                    producer.epoch.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ),
                "recovery must still own the epoch across reset and drain"
            );
            producer_attempted.wait();
            producer.emit(ArchiveReadNotification::Connected {
                connection_generation: 2,
            });
        });
        *receiver.drain_hook.lock().expect("hook") = Some(Box::new(move || {
            entered.wait();
            attempted.wait();
        }));
        assert!(matches!(
            receiver.recv().await,
            Ok(ArchiveReadNotification::Overflow)
        ));
        thread
            .join()
            .expect("producer completes without lock/await deadlock");
        assert!(matches!(
            receiver.recv().await,
            Ok(ArchiveReadNotification::Connected {
                connection_generation: 2
            })
        ));
        for _ in 0..=ARCHIVE_READ_EVENT_CAPACITY {
            bridge.archive.emit(ArchiveReadNotification::Connected {
                connection_generation: 3,
            });
        }
        assert!(matches!(
            receiver.recv().await,
            Ok(ArchiveReadNotification::Overflow)
        ));
    }
}
