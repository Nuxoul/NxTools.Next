//! One admitted native virtual drag retains one exact cancellation on the ORIGINAL connection.
//! Reuses PlatformCommandReceiver fairness/wake and pending receipts; no second transport.
use super::*;
use nxtools_protocol::archive_virtual_drag::ArchiveVirtualDragCommand;

pub(super) struct DragReservation {
    operation: uuid::Uuid,
    session: uuid::Uuid,
    generation: u64,
    start_id: uuid::Uuid,
    start_dequeued: bool,
    cancel_id: Option<uuid::Uuid>,
    cancel: Option<Envelope>,
}
impl DragReservation {
    fn matches(&self, command: &ArchiveVirtualDragCommand) -> bool {
        self.operation == command.operation_id
            && self.session == command.session_id
            && self.generation == command.generation
    }
    pub(super) fn receipt(&self, id: uuid::Uuid) -> bool {
        self.start_id == id || self.cancel_id == Some(id)
    }
}
impl ActiveConnection {
    pub(super) fn note_drag_start(&mut self, id: uuid::Uuid) {
        if let Some(drag) = self
            .archive_drag
            .as_mut()
            .filter(|drag| drag.start_id == id)
        {
            drag.start_dequeued = true;
        }
    }
    pub(super) fn take_drag_cancel(&mut self) -> Option<Envelope> {
        let drag = self
            .archive_drag
            .as_mut()
            .filter(|drag| drag.start_dequeued)?;
        let cancel = drag.cancel.take()?;
        self.cancel_turn = false;
        Some(cancel)
    }
}
impl PlatformBridge {
    /// Retains the connection that admitted the ORIGINAL source read, not current UI state.
    /// Comparison and all admission changes share the existing router lock.
    pub fn send_archive_virtual_drag(
        &self,
        command: ArchiveVirtualDragCommand,
        command_id: uuid::Uuid,
        origin_connection: u64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let mut envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::ArchiveVirtualDrag(Box::new(command.clone())),
        ));
        envelope.id = command_id;
        envelope.correlation_id = Some(command.operation_id);
        self.send_virtual_drag(envelope, &command, Some(origin_connection))
    }
    pub(super) fn send_virtual_drag(
        &self,
        envelope: Envelope,
        command: &ArchiveVirtualDragCommand,
        origin_connection: Option<u64>,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        command.validate().map_err(|_| PlatformBridgeError::Busy)?;
        if envelope.correlation_id != Some(command.operation_id)
            || envelope.id == command.operation_id
        {
            return Err(PlatformBridgeError::Busy);
        }
        // Formal starts require caller-origin authority; the historical isolated spike is separate.
        // Destination location is a start of the same single native drag slot.
        let start = matches!(command.action.as_str(), "start" | "locate");
        if start
            && origin_connection.is_none()
            && matches!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(_))
            )
        {
            return Err(PlatformBridgeError::Disconnected);
        }
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        let active = router
            .active
            .as_ref()
            .filter(|active| origin_connection.is_none_or(|origin| active.generation == origin))
            .ok_or(PlatformBridgeError::Disconnected)?;
        let generation = active.generation;
        if start {
            if active.archive_drag.is_some() {
                return Err(PlatformBridgeError::Busy);
            }
        } else {
            let drag = active
                .archive_drag
                .as_ref()
                .filter(|drag| drag.matches(command))
                .ok_or(PlatformBridgeError::Disconnected)?;
            if let Some(id) = drag.cancel_id {
                return Ok(id);
            }
        }
        let id = envelope.id;
        if router.pending.contains_key(&id) {
            return Err(PlatformBridgeError::DuplicateCommandId);
        }
        if !start {
            router.pending.insert(
                id,
                PendingCommand {
                    generation,
                    accepted_at: Instant::now(),
                    timeout: Some(super::PENDING_COMMAND_TIMEOUT),
                    summon_reply: None,
                },
            );
            let active = router.active.as_mut().expect("original connection locked");
            let drag = active
                .archive_drag
                .as_mut()
                .expect("reserved original drag");
            drag.cancel_id = Some(id);
            drag.cancel = Some(envelope);
            let _ = active.command_wake.try_send(());
            return Ok(id);
        }
        let result = router
            .enqueue(envelope, None)
            .map_err(|failure| match failure {
                QueueAdmissionError::Full => PlatformBridgeError::Busy,
                _ => PlatformBridgeError::Disconnected,
            });
        if result.is_ok() {
            router
                .active
                .as_mut()
                .expect("original connection locked")
                .archive_drag = Some(DragReservation {
                operation: command.operation_id,
                session: command.session_id,
                generation: command.generation,
                start_id: id,
                start_dequeued: false,
                cancel_id: None,
                cancel: None,
            });
        }
        result
    }
}
impl PlatformServerEndpoint {
    pub(super) fn observe_virtual_drag_cleanup(&self, notification: &PlatformNotification) {
        let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = notification
        else {
            return;
        };
        if result.error.as_deref() == Some("virtual_cleanup_incomplete") {
            return;
        }
        if let Ok(mut router) = self.router.lock() {
            let drag = router.active.as_mut().and_then(|active| {
                if active
                    .archive_drag
                    .as_ref()
                    .is_some_and(|drag| drag.start_id == *correlation_id)
                {
                    active.archive_drag.take()
                } else {
                    None
                }
            });
            if let Some(drag) = drag {
                router.pending.remove(&drag.start_id);
                if let Some(id) = drag.cancel_id {
                    router.pending.remove(&id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxtools_protocol::archive_browser::*;
    fn start() -> ArchiveVirtualDragCommand {
        let operation = uuid::Uuid::new_v4();
        let session = uuid::Uuid::new_v4();
        ArchiveVirtualDragCommand {
            version: 1,
            authorization_id: None,
            operation_id: operation,
            session_id: session,
            generation: 7,
            action: "start".into(),
            source_window_handle: 55,
            items: Some(vec![
                nxtools_protocol::archive_virtual_drag::ArchiveVirtualDragItem {
                    name: "one.txt".into(),
                    size: Some(3),
                    directory: false,
                    entry: ArchiveEntryCommand {
                        version: 1,
                        operation_id: uuid::Uuid::new_v4(),
                        session_id: session,
                        generation: 7,
                        action: ArchiveEntryAction::Materialize,
                        path: Some(r"C:\owned\source.zip".into()),
                        codepage: 0,
                        ordinal: Some(0),
                        browser: Some(ArchiveBrowserContext {
                            snapshot_id: uuid::Uuid::new_v4(),
                            format: "Zip".into(),
                            comment: String::new(),
                            sevenz_edit: None,
                            tar_edit: None,
                            source: ArchiveSourceBinding {
                                native: "v1:owned".into(),
                                digest: [1; 32],
                                size: 99,
                                modified_unix_ns: 1,
                            },
                        }),
                        lease_id: None,
                        depth: 0,
                        copy_motw: Some(true),
                        password_interaction: false,
                        temp_policy: None,
                        preview_options: None,
                    },
                },
            ]),
        }
    }
    fn send(
        bridge: &PlatformBridge,
        command: ArchiveVirtualDragCommand,
        id: uuid::Uuid,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        bridge.send_archive_virtual_drag(
            command,
            id,
            bridge.connection_generation().expect("fixture connection"),
        )
    }
    #[test]
    fn reserved_virtual_drag_cancel_is_bounded_ordered_and_cleanup_scoped() {
        let (bridge, endpoint) = platform_bridge(1);
        endpoint.set_connected_for_contract_test(true);
        let command = start();
        let request = uuid::Uuid::new_v4();
        send(&bridge, command.clone(), request).expect("start");
        let cancel = ArchiveVirtualDragCommand {
            action: "cancel".into(),
            source_window_handle: 0,
            items: None,
            ..command.clone()
        };
        let cancel_id = uuid::Uuid::new_v4();
        send(&bridge, cancel.clone(), cancel_id).expect("cancel reserved before dequeue");
        assert_eq!(
            send(&bridge, cancel.clone(), uuid::Uuid::new_v4()).expect("dedup"),
            cancel_id
        );
        assert_eq!(
            endpoint
                .try_receive_command_for_contract_test()
                .expect("start precedes cancel")
                .id,
            request
        );
        bridge
            .send_command(PlatformCommand::PlaySystemNotificationSound)
            .expect("full ordinary queue");
        assert_eq!(
            endpoint
                .try_receive_command_for_contract_test()
                .expect("reserved cancel")
                .id,
            cancel_id
        );
        assert!(matches!(
            endpoint
                .try_receive_command_for_contract_test()
                .expect("fair ordinary command")
                .message,
            Message::PlatformCommand(PlatformCommand::PlaySystemNotificationSound)
        ));
        assert!(
            send(&bridge, start(), uuid::Uuid::new_v4()).is_err(),
            "cancel request is not actual cleanup"
        );
        let generation = endpoint.connection_generation.load(Ordering::Acquire);
        assert!(
            !endpoint
                .expire_pending_commands(generation, Instant::now() + Duration::from_secs(60))
                .iter()
                .any(|pending| pending.id == request),
            "native drag cannot become ordinary 10s timeout"
        );
        endpoint
            .send_notification_for_contract_test(PlatformNotification::CommandResult {
                correlation_id: cancel_id,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: None,
                },
            })
            .expect("cancel ACK");
        bridge
            .notifications
            .try_recv()
            .expect("consume ACK notification");
        assert!(send(&bridge, start(), uuid::Uuid::new_v4()).is_err());
        endpoint
            .send_notification_for_contract_test(PlatformNotification::CommandResult {
                correlation_id: request,
                result: PlatformCommandResult {
                    success: false,
                    error: Some("virtual_cleanup_incomplete".into()),
                    native_error_code: None,
                    data: Some(r#"{"cleanup_complete":false}"#.into()),
                },
            })
            .expect("incomplete cleanup");
        bridge
            .notifications
            .try_recv()
            .expect("consume incomplete notification");
        assert!(send(&bridge, start(), uuid::Uuid::new_v4()).is_err());
        endpoint
            .send_notification_for_contract_test(PlatformNotification::CommandResult {
                correlation_id: request,
                result: PlatformCommandResult {
                    success: true,
                    error: None,
                    native_error_code: None,
                    data: Some(r#"{"status":"cancelled"}"#.into()),
                },
            })
            .expect("actual terminal");
        bridge
            .notifications
            .try_recv()
            .expect("consume terminal notification");
        send(&bridge, start(), uuid::Uuid::new_v4()).expect("actual cleanup releases slot");
        endpoint.set_connected_for_contract_test(false);
        endpoint.set_connected_for_contract_test(true);
        assert!(
            send(&bridge, cancel, uuid::Uuid::new_v4()).is_err(),
            "old cancel cannot reach replaced generation"
        );
        send(&bridge, start(), uuid::Uuid::new_v4()).expect("new generation independently admits");
    }
    #[test]
    fn connection_origin_rejection_has_no_pending_or_reservation_and_fresh_origin_starts() {
        let (bridge, endpoint) = platform_bridge(8);
        let _old_receiver = endpoint.open_connection_for_contract_test();
        let origin = bridge
            .connection_generation()
            .expect("original read connection");
        let command = start();
        let receiver = endpoint.open_connection_for_contract_test();
        let stale_id = uuid::Uuid::new_v4();
        assert_eq!(
            bridge.notifications.len(),
            1,
            "old disconnect is still queued"
        );
        assert_eq!(
            bridge.send_archive_virtual_drag(command.clone(), stale_id, origin),
            Err(PlatformBridgeError::Disconnected)
        );
        assert!(receiver.try_recv().is_err());
        {
            let router = bridge.router.lock().expect("original router");
            assert!(router.pending.is_empty());
            assert!(
                router
                    .active
                    .as_ref()
                    .expect("replacement")
                    .archive_drag
                    .is_none()
            );
        }
        assert_eq!(
            bridge.send_command_with_id_correlated(
                PlatformCommand::ArchiveVirtualDrag(Box::new(command.clone())),
                stale_id,
                command.operation_id
            ),
            Err(PlatformBridgeError::Disconnected),
            "unbound formal start cannot bypass source authority"
        );
        let fresh_origin = bridge
            .connection_generation()
            .expect("fresh read connection");
        bridge
            .send_archive_virtual_drag(command.clone(), stale_id, fresh_origin)
            .expect("same receipt not leaked; fresh source admitted");
        assert_eq!(receiver.try_recv().expect("exactly one start").id, stale_id);
        let cancel = ArchiveVirtualDragCommand {
            action: "cancel".into(),
            source_window_handle: 0,
            items: None,
            ..command
        };
        assert_eq!(
            bridge.send_archive_virtual_drag(cancel, uuid::Uuid::new_v4(), origin),
            Err(PlatformBridgeError::Disconnected),
            "old source cannot cancel even matching fresh reservation identity"
        );
        assert!(receiver.try_recv().is_err());
        assert!(
            matches!(bridge.notifications.try_recv(),Ok(PlatformNotification::Disconnected {generation,..}) if generation==origin)
        );
    }
    #[test]
    fn tree_root_descriptor_uses_same_locked_origin_and_never_leaks_rejected_admission() {
        let (bridge, endpoint) = platform_bridge(8);
        let _old = endpoint.open_connection_for_contract_test();
        let origin = bridge
            .connection_generation()
            .expect("original source read");
        let receiver = endpoint.open_connection_for_contract_test();
        let fresh = bridge.connection_generation().expect("replacement");
        let mut command = start();
        let items = command.items.as_mut().expect("items");
        let mut wrapper = items[0].clone();
        wrapper.name = "Bundle".into();
        wrapper.directory = true;
        wrapper.size = None;
        wrapper.entry.ordinal = None;
        wrapper.entry.operation_id = uuid::Uuid::new_v4();
        items[0].name = "Bundle/one.txt".into();
        items.insert(0, wrapper);
        command.validate().expect("unchanged v1 descriptor wrapper");
        let receipt = uuid::Uuid::new_v4();
        assert_eq!(
            bridge.send_archive_virtual_drag(command.clone(), receipt, origin),
            Err(PlatformBridgeError::Disconnected)
        );
        assert!(receiver.try_recv().is_err());
        {
            let router = bridge.router.lock().expect("same admission lock");
            assert!(router.pending.is_empty());
            assert!(
                router
                    .active
                    .as_ref()
                    .expect("active")
                    .archive_drag
                    .is_none()
            );
        }
        bridge
            .send_archive_virtual_drag(command.clone(), receipt, fresh)
            .expect("fresh authority same receipt");
        let received = receiver.try_recv().expect("one frozen start");
        assert_eq!(received.id, receipt);
        let Message::PlatformCommand(PlatformCommand::ArchiveVirtualDrag(value)) = received.message
        else {
            panic!("formal start")
        };
        assert_eq!(*value, command);
        assert!(receiver.try_recv().is_err());
    }
}
