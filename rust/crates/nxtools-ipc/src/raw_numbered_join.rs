//! Ordinary numbered join uses the original router, with one physical-operation reservation.
use super::*;
use nxtools_protocol::raw_numbered_join::{RawJoinAction, RawNumberedJoinCommand};

pub(super) struct JoinReservation {
    command: RawNumberedJoinCommand,
    start_id: uuid::Uuid,
    dequeued: bool,
    cancel_id: Option<uuid::Uuid>,
    cancel: Option<Envelope>,
}
impl JoinReservation {
    fn matches(&self, c: &RawNumberedJoinCommand) -> bool {
        self.command.operation_id == c.operation_id
            && self.command.session_id == c.session_id
            && self.command.generation == c.generation
            && self.command.version == c.version
    }
}
impl ActiveConnection {
    pub(super) fn note_join_start(&mut self, id: uuid::Uuid) {
        if let Some(join) = self.raw_join.as_mut().filter(|j| j.start_id == id) {
            join.dequeued = true;
        }
    }
    pub(super) fn take_join_cancel(&mut self) -> Option<Envelope> {
        let join = self
            .raw_join
            .as_mut()
            .filter(|j| j.dequeued && j.cancel.is_some())?;
        self.cancel_turn = false;
        join.cancel.take()
    }
}
impl PlatformBridge {
    pub fn send_raw_numbered_join(
        &self,
        c: RawNumberedJoinCommand,
        id: uuid::Uuid,
        origin: u64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        c.validate().map_err(|_| PlatformBridgeError::Busy)?;
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        let active = router
            .active
            .as_ref()
            .filter(|a| a.generation == origin)
            .ok_or(PlatformBridgeError::Disconnected)?;
        let cancel = c.action == RawJoinAction::Cancel;
        if cancel {
            let reserved = active
                .raw_join
                .as_ref()
                .filter(|j| j.matches(&c))
                .ok_or(PlatformBridgeError::Disconnected)?;
            if let Some(existing) = reserved.cancel_id {
                return Ok(existing);
            }
        } else if active.raw_join.is_some() || router.pending.len() >= 32 {
            return Err(PlatformBridgeError::Busy);
        }
        if router.pending.contains_key(&id) {
            return Err(PlatformBridgeError::DuplicateCommandId);
        }
        let mut envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::RawNumberedJoin(c.clone()),
        ));
        envelope.id = id;
        envelope.correlation_id = Some(c.operation_id);
        if cancel {
            router.pending.insert(
                id,
                PendingCommand {
                    generation: origin,
                    accepted_at: Instant::now(),
                    timeout: Some(PENDING_COMMAND_TIMEOUT),
                    summon_reply: None,
                },
            );
            let active = router.active.as_mut().expect("original connection locked");
            let reserved = active.raw_join.as_mut().expect("reserved operation");
            reserved.cancel_id = Some(id);
            reserved.cancel = Some(envelope);
            let _ = active.command_wake.try_send(());
            return Ok(id);
        }
        router.enqueue(envelope, None).map_err(|e| match e {
            QueueAdmissionError::Full => PlatformBridgeError::Busy,
            _ => PlatformBridgeError::Disconnected,
        })?;
        router
            .active
            .as_mut()
            .expect("original connection locked")
            .raw_join = Some(JoinReservation {
            command: c,
            start_id: id,
            dequeued: false,
            cancel_id: None,
            cancel: None,
        });
        Ok(id)
    }
}
impl PlatformServerEndpoint {
    pub(super) fn observe_join_cleanup(&self, notification: &PlatformNotification) {
        let PlatformNotification::CommandResult { correlation_id, .. } = notification else {
            return;
        };
        if let Ok(mut router) = self.router.lock() {
            let retired = router.active.as_mut().and_then(|active| {
                if active
                    .raw_join
                    .as_ref()
                    .is_some_and(|j| j.start_id == *correlation_id)
                {
                    active.raw_join.take()
                } else {
                    None
                }
            });
            if let Some(join) = retired {
                router.pending.remove(&join.start_id);
                if let Some(cancel) = join.cancel_id {
                    router.pending.remove(&cancel);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn command() -> RawNumberedJoinCommand {
        RawNumberedJoinCommand {
            version: 1,
            operation_id: uuid::Uuid::new_v4(),
            session_id: uuid::Uuid::new_v4(),
            generation: 1,
            action: RawJoinAction::Inspect,
            selected_path: Some(r"C:\owned\ordinary.003".into()),
            preview_id: None,
            output_path: None,
        }
    }
    #[test]
    fn ref18_original_router_reserves_cancel_and_waits_for_actual_terminal() {
        let (bridge, endpoint) = platform_bridge(1);
        let receiver = endpoint.open_connection_for_contract_test();
        let origin = bridge.connection_generation().expect("original connection");
        let c = command();
        let id = uuid::Uuid::new_v4();
        bridge
            .send_raw_numbered_join(c.clone(), id, origin)
            .expect("real operation admission");
        assert_eq!(receiver.try_recv().expect("start before cancel").id, id);
        bridge
            .send_command(PlatformCommand::PlaySystemNotificationSound)
            .expect("ordinary queue full");
        let cancel = RawNumberedJoinCommand {
            action: RawJoinAction::Cancel,
            selected_path: None,
            ..c.clone()
        };
        let cancel_id = uuid::Uuid::new_v4();
        bridge
            .send_raw_numbered_join(cancel.clone(), cancel_id, origin)
            .expect("reserved cancellation");
        assert_eq!(
            bridge
                .send_raw_numbered_join(cancel, uuid::Uuid::new_v4(), origin)
                .expect("coalesced"),
            cancel_id
        );
        let a = receiver.try_recv().expect("fair delivery");
        let b = receiver.try_recv().expect("fair delivery");
        assert!(a.id == cancel_id || b.id == cancel_id);
        endpoint
            .send_notification_for_contract_test(PlatformNotification::CommandResult {
                correlation_id: cancel_id,
                result: failed_platform_result("cancel acknowledged"),
            })
            .expect("cancel receipt");
        bridge
            .notification_receiver()
            .try_recv()
            .expect("original consumer observes cancel receipt");
        assert!(matches!(
            bridge.send_raw_numbered_join(command(), uuid::Uuid::new_v4(), origin),
            Err(PlatformBridgeError::Busy)
        ));
        let accepted = bridge.router.lock().expect("router");
        assert!(
            accepted
                .pending
                .get(&id)
                .expect("long operation pending")
                .timeout
                .is_none()
        );
        drop(accepted);
        endpoint
            .send_notification_for_contract_test(PlatformNotification::CommandResult {
                correlation_id: id,
                result: failed_platform_result("cancelled after cleanup"),
            })
            .expect("actual terminal");
        bridge
            .send_raw_numbered_join(command(), uuid::Uuid::new_v4(), origin)
            .expect("released after terminal");
        let _replacement = endpoint.open_connection_for_contract_test();
        assert!(matches!(
            bridge.send_raw_numbered_join(command(), uuid::Uuid::new_v4(), origin),
            Err(PlatformBridgeError::Disconnected)
        ));
    }
}
