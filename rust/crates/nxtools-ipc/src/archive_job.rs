//! General jobs use ordinary notifications but reserve original-connection cancellation.
use super::*;
use nxtools_protocol::archive_job::{ArchiveJobAction, ArchiveJobCommand, ArchiveJobEventKind};

pub(super) struct JobReservation {
    operation: uuid::Uuid,
    session: uuid::Uuid,
    generation: u64,
    version: u32,
    start_id: uuid::Uuid,
    start_dequeued: bool,
    cancel_id: Option<uuid::Uuid>,
    cancel: Option<Envelope>,
}
impl JobReservation {
    fn matches(&self, c: &ArchiveJobCommand) -> bool {
        self.operation == c.operation_id
            && self.session == c.session_id
            && self.generation == c.generation
            && self.version == c.version
    }
}
impl ActiveConnection {
    pub(super) fn note_job_start(&mut self, id: uuid::Uuid) {
        if let Some(job) = self.archive_jobs.iter_mut().find(|j| j.start_id == id) {
            job.start_dequeued = true;
        }
    }
    pub(super) fn take_job_cancel(&mut self) -> Option<Envelope> {
        let job = self
            .archive_jobs
            .iter_mut()
            .find(|j| j.start_dequeued && j.cancel.is_some())?;
        let item = job.cancel.take();
        self.cancel_turn = false;
        item
    }
}
impl PlatformBridge {
    pub fn send_archive_job(
        &self,
        c: ArchiveJobCommand,
        command_id: uuid::Uuid,
        generation: u64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        c.validate().map_err(|_| PlatformBridgeError::Busy)?;
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        let active = router
            .active
            .as_ref()
            .filter(|a| a.generation == generation)
            .ok_or(PlatformBridgeError::Disconnected)?;
        let start = c.action == ArchiveJobAction::Start;
        let cancel = c.action == ArchiveJobAction::Cancel;
        if start {
            if active.archive_jobs.len() >= 2
                || active
                    .archive_jobs
                    .iter()
                    .any(|j| j.operation == c.operation_id)
            {
                return Err(PlatformBridgeError::Busy);
            }
        } else {
            let job = active
                .archive_jobs
                .iter()
                .find(|j| j.matches(&c))
                .ok_or(PlatformBridgeError::Disconnected)?;
            if cancel && let Some(id) = job.cancel_id {
                return Ok(id);
            }
        }
        if router.pending.contains_key(&command_id) {
            return Err(PlatformBridgeError::DuplicateCommandId);
        }
        if !cancel && router.pending.len() >= 32 {
            return Err(PlatformBridgeError::Busy);
        }
        let mut envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::ArchiveJob(
            Box::new(c.clone()),
        )));
        envelope.id = command_id;
        envelope.correlation_id = Some(c.operation_id);
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
            let job = active
                .archive_jobs
                .iter_mut()
                .find(|j| j.matches(&c))
                .expect("reserved original job");
            job.cancel_id = Some(command_id);
            job.cancel = Some(envelope);
            let _ = active.command_wake.try_send(());
            return Ok(command_id);
        }
        let result = router.enqueue(envelope, None).map_err(|e| match e {
            QueueAdmissionError::Full => PlatformBridgeError::Busy,
            _ => PlatformBridgeError::Disconnected,
        });
        if result.is_ok() && start {
            router
                .active
                .as_mut()
                .expect("original connection locked")
                .archive_jobs
                .push(JobReservation {
                    operation: c.operation_id,
                    session: c.session_id,
                    generation: c.generation,
                    version: c.version,
                    start_id: command_id,
                    start_dequeued: false,
                    cancel_id: None,
                    cancel: None,
                });
        }
        result
    }
}
impl PlatformServerEndpoint {
    pub(super) fn observe_job_cleanup(&self, notification: &PlatformNotification) {
        let PlatformNotification::Event(PlatformEvent::ArchiveJob(event)) = notification else {
            return;
        };
        if event.validate().is_err()
            || event.version != 1
            || event.kind != ArchiveJobEventKind::Terminal
            || !event.terminal.as_ref().is_some_and(|t| t.cleanup_complete)
            || event.progress.is_some()
            || event.prompt.is_some()
        {
            return;
        }
        if let Ok(mut router) = self.router.lock() {
            let cancel = router.active.as_mut().and_then(|active| {
                let index = active.archive_jobs.iter().position(|j| {
                    j.operation == event.operation_id
                        && j.session == event.session_id
                        && j.generation == event.generation
                        && j.version == event.version
                })?;
                active.archive_jobs.remove(index).cancel_id
            });
            if let Some(id) = cancel {
                router.pending.remove(&id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn start() -> ArchiveJobCommand {
        ArchiveJobCommand {
            version: 1,
            operation_id: uuid::Uuid::new_v4(),
            session_id: uuid::Uuid::new_v4(),
            generation: 1,
            action: ArchiveJobAction::Start,
            reply: None,
            request: Some(nxtools_protocol::archive_job::ArchiveJobRequest {
                kind: nxtools_protocol::archive_job::ArchiveJobKind::Test,
                paths: vec!["C:\\owned\\tiny.zip".into()],
                destination: None,
                codepage: 0,
                limits: Default::default(),
                extract: None,
                create: None,
                readmission: None,
                modify: None,
            }),
        }
    }
    #[test]
    fn cancellation_is_reserved_under_ordinary_pressure_and_original_generation_only() {
        let (bridge, endpoint) = platform_bridge(1);
        let receiver = endpoint.open_connection_for_contract_test();
        let generation = bridge.connection_generation().expect("connected");
        let command = start();
        let start_id = uuid::Uuid::new_v4();
        bridge
            .send_archive_job(command.clone(), start_id, generation)
            .expect("start");
        assert_eq!(receiver.try_recv().expect("start first").id, start_id);
        bridge
            .send_command(PlatformCommand::PlaySystemNotificationSound)
            .expect("fill ordinary queue");
        let cancel = ArchiveJobCommand {
            action: ArchiveJobAction::Cancel,
            request: None,
            ..command.clone()
        };
        let cancel_id = uuid::Uuid::new_v4();
        bridge
            .send_archive_job(cancel.clone(), cancel_id, generation)
            .expect("reserved cancel");
        assert_eq!(
            bridge
                .send_archive_job(cancel, uuid::Uuid::new_v4(), generation)
                .expect("coalesced"),
            cancel_id
        );
        let mut received = [
            receiver.try_recv().expect("fair command").id,
            receiver.try_recv().expect("fair cancel").id,
        ];
        received.sort();
        assert!(received.contains(&cancel_id));
        let _replacement = endpoint.open_connection_for_contract_test();
        assert!(matches!(
            bridge.send_archive_job(start(), uuid::Uuid::new_v4(), generation),
            Err(PlatformBridgeError::Disconnected)
        ));
    }
    #[test]
    fn job_reservation_releases_only_matching_actual_cleanup_not_false_terminal() {
        use nxtools_protocol::archive_job::*;
        let (bridge, endpoint) = platform_bridge(4);
        let _receiver = endpoint.open_connection_for_contract_test();
        let connection = bridge.connection_generation().expect("connected");
        let first = start();
        bridge
            .send_archive_job(first.clone(), uuid::Uuid::new_v4(), connection)
            .expect("first");
        bridge
            .send_archive_job(start(), uuid::Uuid::new_v4(), connection)
            .expect("second");
        let mut event = ArchiveJobEvent {
            version: 1,
            operation_id: first.operation_id,
            session_id: first.session_id,
            generation: first.generation,
            kind: ArchiveJobEventKind::Terminal,
            state: None,
            progress: None,
            prompt: None,
            terminal: Some(ArchiveJobTerminal {
                outcome: ArchiveJobOutcome::Cancelled,
                code: Some("cancelled".into()),
                files_committed: 0,
                bytes_committed: 0,
                files_verified: 0,
                bytes_verified: 0,
                files_skipped: 0,
                cleanup_complete: false,
                commit_state: None,
                elapsed_ms: 1,
                native_error_code: None,
                journal: vec![],
                output_shortfall: None,
                result_location: None,
                output_published: None,
            }),
        };
        endpoint.observe_job_cleanup(&PlatformNotification::Event(PlatformEvent::ArchiveJob(
            Box::new(event.clone()),
        )));
        assert!(matches!(
            bridge.send_archive_job(start(), uuid::Uuid::new_v4(), connection),
            Err(PlatformBridgeError::Busy)
        ));
        event.terminal.as_mut().expect("terminal").cleanup_complete = true;
        event.generation += 1;
        endpoint.observe_job_cleanup(&PlatformNotification::Event(PlatformEvent::ArchiveJob(
            Box::new(event.clone()),
        )));
        assert!(matches!(
            bridge.send_archive_job(start(), uuid::Uuid::new_v4(), connection),
            Err(PlatformBridgeError::Busy)
        ));
        event.generation = first.generation;
        endpoint.observe_job_cleanup(&PlatformNotification::Event(PlatformEvent::ArchiveJob(
            Box::new(event),
        )));
        bridge
            .send_archive_job(start(), uuid::Uuid::new_v4(), connection)
            .expect("only actual matching cleanup released");
    }
}
