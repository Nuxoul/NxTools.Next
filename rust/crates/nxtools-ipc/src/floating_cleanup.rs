use super::*;
use nxtools_protocol::floating_assistant::{FloatingAssistantCommand, FloatingAssistantOperation};

impl PlatformBridge {
    /// Capture before preparing the renderer; later commands cannot cross a reconnect.
    pub fn floating_assistant_connection(&self) -> Option<u64> {
        self.router
            .lock()
            .ok()?
            .active
            .as_ref()
            .map(|active| active.generation)
    }

    /// One original-connection cleanup slot, independent of the controller's lifetime.
    /// Ordinary floating commands keep the existing bounded admission policy.
    pub fn send_floating_assistant(
        &self,
        command: FloatingAssistantCommand,
        receipt: uuid::Uuid,
        origin: u64,
    ) -> Result<uuid::Uuid, PlatformBridgeError> {
        let cleanup = matches!(
            command.operation,
            FloatingAssistantOperation::ReleaseDrop | FloatingAssistantOperation::Retire
        );
        let mut envelope = Envelope::new(Message::PlatformCommand(
            PlatformCommand::FloatingAssistant(command.clone()),
        ));
        envelope.id = receipt;
        envelope.correlation_id = command.drop_id.or(Some(receipt));
        let correlation = envelope.correlation_id.unwrap_or(receipt);
        let mut router = self
            .router
            .lock()
            .map_err(|_| PlatformBridgeError::Disconnected)?;
        if router.pending.contains_key(&receipt) {
            return Err(PlatformBridgeError::DuplicateCommandId);
        }
        let active = router
            .active
            .as_mut()
            .filter(|active| active.generation == origin)
            .ok_or(PlatformBridgeError::Disconnected)?;
        if cleanup && let Some(previous) = &active.floating_cleanup {
            let Message::PlatformCommand(PlatformCommand::FloatingAssistant(old)) =
                &previous.message
            else {
                unreachable!("floating cleanup slot contains only typed cleanup");
            };
            if old.session_id != command.session_id
                || old.generation != command.generation
                || old.window_handle != command.window_handle
                || old.source_window_handle != command.source_window_handle
                || old.source_process_id != command.source_process_id
            {
                return Err(PlatformBridgeError::Busy);
            }
            if command.operation == FloatingAssistantOperation::ReleaseDrop {
                if old.operation == FloatingAssistantOperation::Retire
                    || old.drop_id == command.drop_id
                {
                    return Ok(previous.id);
                }
                return Err(PlatformBridgeError::Busy);
            }
            // Retirement owns all of this session's staging, including an unsent release.
            active.floating_cleanup = Some(envelope);
            let _ = active.command_wake.try_send(());
            return Ok(receipt);
        }
        match router.enqueue(envelope.clone(), None) {
            Ok(id) => Ok(id),
            Err(QueueAdmissionError::Full) if cleanup => {
                let active = router
                    .active
                    .as_mut()
                    .expect("original connection under router lock");
                active.floating_cleanup = Some(envelope);
                let _ = active.command_wake.try_send(());
                info!(event="floating_assistant.cleanup.deferred", correlation_id=%correlation,
                    connection_generation=origin, "floating cleanup retained on original connection");
                Ok(receipt)
            }
            Err(QueueAdmissionError::Full) => Err(PlatformBridgeError::Busy),
            Err(_) => Err(PlatformBridgeError::Disconnected),
        }
    }
}

impl CommandRouter {
    /// Called after an original dequeue; no retry task, alternate reader or new queue.
    pub(super) fn flush_floating_cleanup(&mut self) {
        let Some(envelope) = self
            .active
            .as_mut()
            .and_then(|active| active.floating_cleanup.take())
        else {
            return;
        };
        let receipt = envelope.id;
        let correlation = envelope.correlation_id.unwrap_or(receipt);
        match self.enqueue(envelope.clone(), None) {
            Ok(_) => info!(event="floating_assistant.cleanup.resumed", correlation_id=%correlation,
                "floating cleanup admitted after original queue capacity returned"),
            Err(QueueAdmissionError::Full) => {
                if let Some(active) = &mut self.active {
                    active.floating_cleanup = Some(envelope);
                }
            }
            // Original connection retirement transfers physical cleanup to its C# EOF owner.
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(operation: FloatingAssistantOperation) -> FloatingAssistantCommand {
        FloatingAssistantCommand {
            version: 1,
            session_id: uuid::Uuid::new_v4(),
            generation: 1,
            window_handle: -40,
            source_window_handle: -39,
            source_process_id: 1,
            operation,
            x: 40,
            y: 180,
            size: 72,
            opacity: 48,
            click_through: false,
            position_locked: false,
            drop_id: None,
            edge_hidden: false,
        }
    }

    #[test]
    fn floating_image_picker_waits_for_selection_or_cancel_without_machine_timeout() {
        for data in [
            r#"{"selected_path":"C:/fixture/image.png"}"#,
            r#"{"selected_path":null}"#,
        ] {
            let (bridge, endpoint) = platform_bridge(1);
            let receiver = endpoint.open_connection_for_contract_test();
            let origin = bridge
                .floating_assistant_connection()
                .expect("original picker connection");
            let id = uuid::Uuid::new_v4();
            let picker = command(FloatingAssistantOperation::PickImage);
            bridge
                .send_command_with_id(PlatformCommand::FloatingAssistant(picker.clone()), id)
                .expect("admit caller-owned image picker");
            let envelope = receiver.try_recv().expect("original image picker dispatch");
            assert_eq!(envelope.id, id);
            assert_eq!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::FloatingAssistant(picker))
            );
            let accepted = endpoint.router.lock().expect("router").pending[&id].accepted_at;
            for seconds in [11, 121, 601] {
                assert!(
                    endpoint
                        .expire_pending_commands(origin, accepted + Duration::from_secs(seconds))
                        .is_empty(),
                    "waiting for image selection must not produce a timeout failure"
                );
            }
            assert!(bridge.notification_receiver().try_recv().is_err());
            assert!(endpoint.complete_command(origin, id).is_some());
            endpoint
                .send_notification_for_contract_test(PlatformNotification::CommandResult {
                    correlation_id: id,
                    result: PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some(data.into()),
                    },
                })
                .expect("publish delayed picker result");
            assert!(matches!(
                bridge.notification_receiver().try_recv(),
                Ok(PlatformNotification::CommandResult { correlation_id, result })
                    if correlation_id == id && result.success && result.data.as_deref() == Some(data)
            ));
            assert!(endpoint.complete_command(origin, id).is_none());
            assert!(!endpoint.command_pending_for_contract_test(id));
        }
    }

    #[test]
    fn floating_image_picker_disconnect_retires_pending_and_rejects_late_results() {
        let (bridge, endpoint) = platform_bridge(1);
        let receiver = endpoint.open_connection_for_contract_test();
        let origin = bridge
            .floating_assistant_connection()
            .expect("original picker connection");
        let id = uuid::Uuid::new_v4();
        bridge
            .send_command_with_id(
                PlatformCommand::FloatingAssistant(command(FloatingAssistantOperation::PickImage)),
                id,
            )
            .expect("admit original image picker");
        assert_eq!(receiver.try_recv().expect("picker dispatch").id, id);
        endpoint.set_connected_for_contract_test(false);
        assert!(!endpoint.command_pending_for_contract_test(id));
        assert!(matches!(
            bridge.notification_receiver().try_recv(),
            Ok(PlatformNotification::Disconnected { generation, pending_command_ids })
                if generation == origin && pending_command_ids == vec![id]
        ));
        let replacement = endpoint.open_connection_for_contract_test();
        let current = bridge
            .floating_assistant_connection()
            .expect("replacement picker connection");
        bridge
            .send_command_with_id(
                PlatformCommand::FloatingAssistant(command(FloatingAssistantOperation::PickImage)),
                id,
            )
            .expect("admit replacement picker with retired caller ID");
        assert_eq!(replacement.try_recv().expect("replacement dispatch").id, id);
        assert!(endpoint.complete_command(origin, id).is_none());
        assert!(endpoint.command_pending_for_contract_test(id));
        assert!(endpoint.complete_command(current, id).is_some());
        assert!(endpoint.complete_command(current, id).is_none());
    }

    #[test]
    fn floating_cleanup_is_one_original_slot_and_retire_covers_release() {
        let (bridge, endpoint) = platform_bridge(1);
        let receiver = endpoint.open_connection_for_contract_test();
        let origin = bridge
            .floating_assistant_connection()
            .expect("original connection");
        let mut value = command(FloatingAssistantOperation::Update);
        bridge
            .send_command(PlatformCommand::FloatingAssistant(value.clone()))
            .expect("fill ordinary queue");
        value.operation = FloatingAssistantOperation::ReleaseDrop;
        value.drop_id = Some(uuid::Uuid::new_v4());
        let release = uuid::Uuid::new_v4();
        assert_eq!(
            bridge.send_floating_assistant(value.clone(), release, origin),
            Ok(release)
        );
        assert_eq!(
            bridge.send_floating_assistant(value.clone(), uuid::Uuid::new_v4(), origin),
            Ok(release)
        );
        let mut other = value.clone();
        other.drop_id = Some(uuid::Uuid::new_v4());
        assert_eq!(
            bridge.send_floating_assistant(other, uuid::Uuid::new_v4(), origin),
            Err(PlatformBridgeError::Busy)
        );
        let mut stale = value.clone();
        stale.generation += 1;
        assert_eq!(
            bridge.send_floating_assistant(stale, uuid::Uuid::new_v4(), origin),
            Err(PlatformBridgeError::Busy)
        );
        value.operation = FloatingAssistantOperation::Retire;
        value.drop_id = None;
        let retire = uuid::Uuid::new_v4();
        assert_eq!(
            bridge.send_floating_assistant(value, retire, origin),
            Ok(retire)
        );
        assert!(!endpoint.command_pending_for_contract_test(release));
        drop(bridge);
        receiver.try_recv().expect("original queued update");
        assert_eq!(receiver.try_recv().expect("retained retirement").id, retire);
        assert!(endpoint.command_pending_for_contract_test(retire));
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn floating_cleanup_cannot_replay_or_authorize_replacement_connection() {
        let (bridge, endpoint) = platform_bridge(1);
        let old = endpoint.open_connection_for_contract_test();
        let origin = bridge
            .floating_assistant_connection()
            .expect("original connection");
        let mut value = command(FloatingAssistantOperation::Update);
        bridge
            .send_command(PlatformCommand::FloatingAssistant(value.clone()))
            .expect("fill old ordinary queue");
        value.operation = FloatingAssistantOperation::Retire;
        bridge
            .send_floating_assistant(value.clone(), uuid::Uuid::new_v4(), origin)
            .expect("retain original retirement");
        let current = endpoint.open_connection_for_contract_test();
        assert_eq!(
            bridge.send_floating_assistant(value, uuid::Uuid::new_v4(), origin),
            Err(PlatformBridgeError::Disconnected)
        );
        assert!(current.try_recv().is_err());
        old.try_recv()
            .expect("old queue retains only its ordinary update");
        assert!(old.try_recv().is_err());
        assert!(current.try_recv().is_err());
    }
    #[test]
    fn floating_retire_waits_for_original_terminal_not_generic_timeout() {
        let (bridge, endpoint) = platform_bridge(1);
        let receiver = endpoint.open_connection_for_contract_test();
        let origin = bridge
            .floating_assistant_connection()
            .expect("original connection");
        let command = command(FloatingAssistantOperation::Retire);
        let id = uuid::Uuid::new_v4();
        bridge
            .send_floating_assistant(command, id, origin)
            .expect("admit original retirement");
        receiver.try_recv().expect("original retirement dispatch");
        let expired =
            endpoint.expire_pending_commands(origin, Instant::now() + Duration::from_secs(60));
        assert!(
            expired.is_empty(),
            "retirement must await original native terminal or disconnect"
        );
        assert!(endpoint.command_pending_for_contract_test(id));
        assert!(endpoint.complete_command(origin, id).is_some());
    }
}
