//! Original-connection bounded password preparation/revocation, never a secret store/transport.
use super::*;
use nxtools_protocol::archive_browser::{ArchiveEntryAction, ArchiveEntryCommand};

pub(super) struct AuthorizationReservation {
    operation: uuid::Uuid,
    session: uuid::Uuid,
    generation: u64,
    start: uuid::Uuid,
    dequeued: bool,
    revoke_id: Option<uuid::Uuid>,
    revoke: Option<Envelope>,
    finished: bool,
    revoke_acked: bool,
}
impl ActiveConnection {
    pub(super) fn note_authorization_start(&mut self, id: uuid::Uuid) {
        for grant in &mut self.archive_authorizations {
            if grant.start == id {
                grant.dequeued = true;
            }
        }
    }
    pub(super) fn take_authorization_revoke(&mut self) -> Option<Envelope> {
        let grant = self
            .archive_authorizations
            .iter_mut()
            .find(|grant| grant.dequeued && grant.revoke.is_some())?;
        self.cancel_turn = false;
        grant.revoke.take()
    }
    pub(super) fn holds_authorization_receipt(&self, id: uuid::Uuid) -> bool {
        self.archive_authorizations
            .iter()
            .any(|grant| grant.start == id || grant.revoke_id == Some(id))
    }
}
impl PlatformBridge {
    pub fn send_archive_authorization(
        &self,
        command: ArchiveEntryCommand,
        receipt: uuid::Uuid,
        origin: u64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        command.validate().map_err(|_| PlatformBridgeError::Busy)?;
        if !matches!(
            command.action,
            ArchiveEntryAction::Authorize | ArchiveEntryAction::Revoke
        ) || receipt == command.operation_id
        {
            return Err(PlatformBridgeError::Busy);
        }
        let mut envelope = Envelope::new(Message::PlatformCommand(PlatformCommand::ArchiveEntry(
            command.clone(),
        )));
        envelope.id = receipt;
        envelope.correlation_id = Some(command.operation_id);
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        let active = router
            .active
            .as_ref()
            .filter(|active| active.generation == origin)
            .ok_or(PlatformBridgeError::Disconnected)?;
        let start = command.action == ArchiveEntryAction::Authorize;
        if start {
            if active.archive_authorizations.len() >= 8
                || active
                    .archive_authorizations
                    .iter()
                    .any(|grant| grant.operation == command.operation_id)
            {
                return Err(PlatformBridgeError::Busy);
            }
        } else {
            let grant = active
                .archive_authorizations
                .iter()
                .find(|grant| {
                    grant.operation == command.operation_id
                        && grant.session == command.session_id
                        && grant.generation == command.generation
                })
                .ok_or(PlatformBridgeError::Disconnected)?;
            if let Some(id) = grant.revoke_id {
                return Ok(id);
            }
        }
        if router.pending.contains_key(&receipt) {
            return Err(PlatformBridgeError::DuplicateCommandId);
        }
        if !start {
            router.pending.insert(
                receipt,
                PendingCommand {
                    generation: origin,
                    accepted_at: Instant::now(),
                    timeout: Some(super::PENDING_COMMAND_TIMEOUT),
                    summon_reply: None,
                },
            );
            let active = router.active.as_mut().expect("locked connection");
            let grant = active
                .archive_authorizations
                .iter_mut()
                .find(|grant| grant.operation == command.operation_id)
                .expect("original reservation");
            grant.revoke_id = Some(receipt);
            grant.revoke = Some(envelope);
            let _ = active.command_wake.try_send(());
            return Ok(receipt);
        }
        router
            .enqueue(envelope, None)
            .map_err(|error| match error {
                QueueAdmissionError::Full => PlatformBridgeError::Busy,
                _ => PlatformBridgeError::Disconnected,
            })?;
        router
            .active
            .as_mut()
            .expect("locked connection")
            .archive_authorizations
            .push(AuthorizationReservation {
                operation: command.operation_id,
                session: command.session_id,
                generation: command.generation,
                start: receipt,
                dequeued: false,
                revoke_id: None,
                revoke: None,
                finished: false,
                revoke_acked: false,
            });
        Ok(receipt)
    }
    /// Secret reply stays on its admitted preparation's ORIGINAL connection, including reconnect gaps.
    pub fn send_archive_authorization_password(
        &self,
        reply: nxtools_protocol::archive_password::ArchivePasswordReply,
        origin: u64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        let active = router
            .active
            .as_ref()
            .filter(|active| active.generation == origin)
            .ok_or(PlatformBridgeError::Disconnected)?;
        if !active.archive_authorizations.iter().any(|grant| {
            grant.operation == reply.prompt.operation_id
                && grant.session == reply.prompt.session_id
                && grant.generation == reply.prompt.generation
                && grant.revoke_id.is_none()
        }) {
            return Err(PlatformBridgeError::Disconnected);
        }
        let operation = reply.prompt.operation_id;
        let mut envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::ArchivePasswordReply(reply),
        ));
        envelope.correlation_id = Some(operation);
        router.enqueue(envelope, None).map_err(|error| match error {
            QueueAdmissionError::Full => PlatformBridgeError::Busy,
            _ => PlatformBridgeError::Disconnected,
        })
    }
}
impl PlatformServerEndpoint {
    pub(super) fn observe_authorization_cleanup(&self, notification: &PlatformNotification) {
        let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = notification
        else {
            return;
        };
        if matches!(
            result.error.as_deref(),
            Some("cleanup_incomplete" | "virtual_cleanup_incomplete")
        ) {
            return;
        }
        if let Ok(mut router) = self.router.lock() {
            if let Some(active) = router.active.as_mut() {
                for grant in &mut active.archive_authorizations {
                    if grant.start == *correlation_id {
                        grant.finished = true;
                    }
                    if grant.revoke_id == Some(*correlation_id) {
                        grant.revoke_acked = true;
                    }
                }
            }
            let index = router.active.as_ref().and_then(|active| {
                active.archive_authorizations.iter().position(|grant| {
                    grant.finished
                        && (grant.revoke_acked || grant.start == *correlation_id && !result.success)
                })
            });
            if let Some(index) = index {
                let grant = router
                    .active
                    .as_mut()
                    .expect("original connection")
                    .archive_authorizations
                    .remove(index);
                router.pending.remove(&grant.start);
                if let Some(id) = grant.revoke_id {
                    router.pending.remove(&id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn prepare() -> ArchiveEntryCommand {
        use nxtools_protocol::archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding};
        ArchiveEntryCommand {
            version: 1,
            operation_id: uuid::Uuid::new_v4(),
            session_id: uuid::Uuid::new_v4(),
            generation: 7,
            action: ArchiveEntryAction::Authorize,
            path: Some(r"C:\owned\source.zip".into()),
            codepage: 0,
            ordinal: None,
            browser: Some(ArchiveBrowserContext {
                snapshot_id: uuid::Uuid::new_v4(),
                format: "Zip".into(),
                comment: String::new(),
                sevenz_edit: None,
                tar_edit: None,
                source: ArchiveSourceBinding {
                    native: "v1:owned".into(),
                    digest: [1; 32],
                    size: 1,
                    modified_unix_ns: 1,
                },
            }),
            lease_id: None,
            depth: 0,
            copy_motw: None,
            password_interaction: true,
            temp_policy: None,
            preview_options: None,
        }
    }
    #[test]
    fn authorization_revoke_is_reserved_ordered_and_ack_does_not_replace_cleanup() {
        let (bridge, endpoint) = platform_bridge(1);
        endpoint.set_connected_for_contract_test(true);
        let origin = bridge.connection_generation().expect("origin");
        let command = prepare();
        let receipt = uuid::Uuid::new_v4();
        bridge
            .send_archive_authorization(command.clone(), receipt, origin)
            .expect("prepare");
        let revoke = ArchiveEntryCommand {
            action: ArchiveEntryAction::Revoke,
            path: None,
            browser: None,
            codepage: 0,
            ordinal: None,
            lease_id: Some(command.operation_id),
            password_interaction: false,
            temp_policy: None,
            preview_options: None,
            copy_motw: None,
            depth: 0,
            ..command.clone()
        };
        let revoke_id = uuid::Uuid::new_v4();
        bridge
            .send_archive_authorization(revoke.clone(), revoke_id, origin)
            .expect("revoke retained despitefullqueue");
        assert_eq!(
            endpoint
                .try_receive_command_for_contract_test()
                .expect("startfirst")
                .id,
            receipt
        );
        bridge
            .send_command(PlatformCommand::PlaySystemNotificationSound)
            .expect("ordinaryfull");
        assert_eq!(
            endpoint
                .try_receive_command_for_contract_test()
                .expect("revoke")
                .id,
            revoke_id
        );
        endpoint
            .send_notification_for_contract_test(PlatformNotification::CommandResult {
                correlation_id: revoke_id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            })
            .expect("ACK");
        assert_eq!(
            bridge
                .router
                .lock()
                .expect("router")
                .active
                .as_ref()
                .expect("active")
                .archive_authorizations
                .len(),
            1,
            "ACKnotcleanup"
        );
        bridge.notifications.try_recv().expect("ACK");
        endpoint
            .send_notification_for_contract_test(PlatformNotification::CommandResult {
                correlation_id: receipt,
                result: PlatformCommandResult {
                    success: false,
                    error: Some("cancelled".into()),
                    native_error_code: None,
                    data: None,
                },
            })
            .expect("actualterminal");
        assert!(
            bridge
                .router
                .lock()
                .expect("router")
                .active
                .as_ref()
                .expect("active")
                .archive_authorizations
                .is_empty()
        );
        let receiver = endpoint.open_connection_for_contract_test();
        assert_eq!(
            bridge.send_archive_authorization(command, uuid::Uuid::new_v4(), origin),
            Err(PlatformBridgeError::Disconnected)
        );
        assert_eq!(
            bridge.send_archive_authorization(revoke, uuid::Uuid::new_v4(), origin),
            Err(PlatformBridgeError::Disconnected)
        );
        assert!(receiver.try_recv().is_err());
    }
}
