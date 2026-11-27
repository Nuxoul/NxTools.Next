//! Material submission ordering on the original connection and command receiver.
use crate::*;
use std::collections::VecDeque;
use uuid::Uuid;

#[derive(Default)]
pub(super) struct MaterialCommands {
    active: Option<Uuid>,
    waiting: VecDeque<Envelope>,
    unavailable: bool,
}

impl ActiveConnection {
    pub(super) fn take_material(&mut self) -> Option<Envelope> {
        if self.material.active.is_some() || self.material.unavailable {
            return None;
        }
        let envelope = self.material.waiting.pop_front()?;
        self.material.active = Some(envelope.id);
        self.cancel_turn = true;
        info!(event = "platform.material.dispatched", command_id = %envelope.id,
            connection_generation = self.generation, waiting_count = self.material.waiting.len());
        Some(envelope)
    }
}

impl CommandRouter {
    pub(super) fn enqueue_material(
        &mut self,
        envelope: Envelope,
        summon_reply: Option<Sender<PlatformCommandResult>>,
    ) -> std::result::Result<Uuid, QueueAdmissionError> {
        let active = self
            .active
            .as_mut()
            .ok_or(QueueAdmissionError::Disconnected)?;
        if active.material.unavailable {
            return Err(QueueAdmissionError::Closed);
        }
        if active.material.waiting.len() + usize::from(active.material.active.is_some())
            >= self.capacity
        {
            return Err(QueueAdmissionError::Full);
        }
        let id = envelope.id;
        self.pending.insert(
            id,
            PendingCommand {
                generation: active.generation,
                accepted_at: Instant::now(),
                timeout: Some(PENDING_COMMAND_TIMEOUT),
                summon_reply,
            },
        );
        active.material.waiting.push_back(envelope);
        let _ = active.command_wake.try_send(());
        info!(event = "platform.material.queued", command_id = %id,
            connection_generation = active.generation, waiting_count = active.material.waiting.len());
        Ok(id)
    }

    fn fail_waiting_material(&mut self) -> Vec<PendingTerminal> {
        let Some(active) = self.active.as_mut() else {
            return Vec::new();
        };
        active.material.unavailable = true;
        active
            .material
            .waiting
            .drain(..)
            .filter_map(|envelope| {
                self.pending
                    .remove(&envelope.id)
                    .map(|pending| PendingTerminal {
                        id: envelope.id,
                        summon_reply: pending.summon_reply,
                    })
            })
            .collect()
    }

    pub(super) fn expire_material(&mut self, expired: &mut Vec<PendingTerminal>) {
        let Some(active) = self.active.as_mut() else {
            return;
        };
        active
            .material
            .waiting
            .retain(|envelope| !expired.iter().any(|pending| pending.id == envelope.id));
        if active
            .material
            .active
            .is_some_and(|id| expired.iter().any(|pending| pending.id == id))
        {
            // An IPC deadline does not prove that the native worker has released its slot.
            expired.extend(self.fail_waiting_material());
        }
    }

    fn finish_material(
        &mut self,
        generation: u64,
        id: Uuid,
        result: &PlatformCommandResult,
    ) -> Vec<PendingTerminal> {
        let Some(active) = self
            .active
            .as_mut()
            .filter(|active| active.generation == generation)
        else {
            return Vec::new();
        };
        if active.material.active != Some(id) {
            return Vec::new();
        }
        active.material.active = None;
        if !result.success && result.error.as_deref() == Some("window_material_unavailable") {
            return self.fail_waiting_material();
        }
        // An expired native operation must not reopen this connection's material lane.
        if !active.material.unavailable {
            let _ = active.command_wake.try_send(());
        }
        Vec::new()
    }
}

impl PlatformBridge {
    /// Retire an owner-cancelled request only while it is still local to the original queue.
    /// A dispatched operation retains its identity/slot until its actual terminal or disconnect.
    pub fn retire_queued_window_material(&self, id: Uuid) -> bool {
        let Ok(mut router) = self.router.lock() else {
            return false;
        };
        let Some(active) = router.active.as_mut() else {
            return false;
        };
        let Some(index) = active
            .material
            .waiting
            .iter()
            .position(|envelope| envelope.id == id)
        else {
            return false;
        };
        active.material.waiting.remove(index);
        router.pending.remove(&id);
        info!(event = "platform.material.retired", command_id = %id);
        true
    }
}

impl PlatformServerEndpoint {
    fn material_terminal(
        &self,
        generation: u64,
        id: Uuid,
        result: &PlatformCommandResult,
    ) -> Vec<PendingTerminal> {
        self.router
            .lock()
            .map(|mut router| router.finish_material(generation, id, result))
            .unwrap_or_default()
    }

    pub(super) async fn route_command_result(
        &self,
        generation: u64,
        id: Uuid,
        result: PlatformCommandResult,
    ) -> Result<()> {
        let failed = self.material_terminal(generation, id, &result);
        if let Some(pending) = self.complete_command(generation, id) {
            if let Some(reply) = pending.summon_reply {
                let _ = reply.send(result).await;
            } else {
                self.send_notification(PlatformNotification::CommandResult {
                    correlation_id: id,
                    result,
                })
                .await?;
            }
        } else {
            warn!(generation, command_id = %id, "ignored platform command result for a stale or unknown command");
        }
        self.send_failed_results(failed, "window_material_unavailable")
            .await
    }

    /// Feed an actual terminal through the production pending/material ownership seam.
    #[doc(hidden)]
    pub fn route_command_result_for_contract_test(
        &self,
        id: Uuid,
        result: PlatformCommandResult,
    ) -> Result<(), String> {
        let generation = self
            .router
            .lock()
            .map_err(|_| "router unavailable")?
            .active
            .as_ref()
            .ok_or("disconnected")?
            .generation;
        let failed = self.material_terminal(generation, id, &result);
        if let Some(pending) = self.complete_command(generation, id) {
            if let Some(reply) = pending.summon_reply {
                reply.try_send(result).map_err(|error| error.to_string())?;
            } else {
                self.send_notification_for_contract_test(PlatformNotification::CommandResult {
                    correlation_id: id,
                    result,
                })?;
            }
        }
        for pending in failed {
            let result = failed_platform_result("window_material_unavailable");
            if let Some(reply) = pending.summon_reply {
                reply.try_send(result).map_err(|error| error.to_string())?;
            } else {
                self.send_notification_for_contract_test(PlatformNotification::CommandResult {
                    correlation_id: pending.id,
                    result,
                })?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxtools_protocol::window_material::{
        ApplyWindowMaterialCommand, MaterialSurface, WindowMaterialStyle,
    };

    fn material(window_handle: i64) -> PlatformCommand {
        PlatformCommand::ApplyWindowMaterial(ApplyWindowMaterialCommand {
            session_id: Uuid::new_v4(),
            facts_revision: 1,
            window_handle,
            generation: 1,
            style: WindowMaterialStyle::Acrylic,
            surface: MaterialSurface::Backdrop,
            dark: true,
        })
    }
    fn success() -> PlatformCommandResult {
        PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: None,
        }
    }

    #[test]
    fn material_submissions_wait_for_terminal_without_blocking_ordinary_commands() {
        let (bridge, endpoint) = platform_bridge(8);
        let receiver = endpoint.open_connection_for_contract_test();
        let first = bridge.send_command(material(42)).expect("first material");
        let second = bridge.send_command(material(43)).expect("second material");
        assert_eq!(receiver.try_recv().expect("first dispatch").id, first);
        assert!(
            receiver.try_recv().is_err(),
            "only one native material operation may be in flight"
        );
        let ordinary = bridge
            .send_command(PlatformCommand::EnsureIndexerService)
            .expect("synthetic ordinary command");
        assert_eq!(receiver.try_recv().expect("ordinary dispatch").id, ordinary);
        assert!(endpoint.command_pending_for_contract_test(second));
        endpoint
            .route_command_result_for_contract_test(Uuid::new_v4(), success())
            .expect("unknown result");
        assert!(receiver.try_recv().is_err());
        endpoint
            .route_command_result_for_contract_test(first, success())
            .expect("first terminal");
        assert_eq!(receiver.try_recv().expect("second dispatch").id, second);
        endpoint
            .route_command_result_for_contract_test(first, success())
            .expect("duplicate terminal");
        assert!(receiver.try_recv().is_err());
        assert!(
            matches!(bridge.notification_receiver().try_recv(), Ok(PlatformNotification::CommandResult { correlation_id, .. }) if correlation_id == first)
        );
        assert!(
            bridge.notification_receiver().try_recv().is_err(),
            "one original terminal only"
        );
    }

    #[test]
    fn material_queue_owner_retirement_is_bounded_and_does_not_release_active_native_work() {
        let (bridge, endpoint) = platform_bridge(2);
        let receiver = endpoint.open_connection_for_contract_test();
        let first = bridge.send_command(material(42)).expect("first");
        let stale = bridge.send_command(material(43)).expect("queued");
        assert_eq!(
            bridge.send_command(material(44)),
            Err(PlatformBridgeError::Busy)
        );
        assert_eq!(receiver.try_recv().expect("active").id, first);
        assert!(!bridge.retire_queued_window_material(first));
        assert!(bridge.retire_queued_window_material(stale));
        assert!(!endpoint.command_pending_for_contract_test(stale));
        let latest = bridge
            .send_command(material(43))
            .expect("latest owner generation");
        assert!(receiver.try_recv().is_err());
        endpoint
            .route_command_result_for_contract_test(
                first,
                failed_platform_result("stale_material_facts"),
            )
            .expect("stale native terminal");
        assert_eq!(receiver.try_recv().expect("latest dispatch").id, latest);
    }

    #[test]
    fn material_native_unavailable_fails_waiters_without_replacement_workers() {
        let (bridge, endpoint) = platform_bridge(8);
        let receiver = endpoint.open_connection_for_contract_test();
        let first = bridge.send_command(material(42)).expect("first");
        let second = bridge.send_command(material(43)).expect("second");
        assert_eq!(receiver.try_recv().expect("active").id, first);
        endpoint
            .route_command_result_for_contract_test(
                first,
                failed_platform_result("window_material_unavailable"),
            )
            .expect("unavailable terminal");
        for id in [first, second] {
            assert!(
                matches!(bridge.notification_receiver().try_recv(), Ok(PlatformNotification::CommandResult { correlation_id, result }) if correlation_id == id && !result.success && result.error.as_deref() == Some("window_material_unavailable"))
            );
            assert!(!endpoint.command_pending_for_contract_test(id));
        }
        assert!(receiver.try_recv().is_err());
        assert_eq!(
            bridge.send_command(material(44)),
            Err(PlatformBridgeError::Disconnected)
        );
        let ordinary = bridge
            .send_command(PlatformCommand::EnsureIndexerService)
            .expect("ordinary remains responsive");
        assert_eq!(receiver.try_recv().expect("ordinary").id, ordinary);
    }

    #[tokio::test]
    async fn material_ipc_deadline_never_releases_an_unconfirmed_native_slot() {
        let (bridge, endpoint) = platform_bridge(8);
        let receiver = endpoint.open_connection_for_contract_test();
        let generation = bridge.connection_generation().expect("connection");
        let first = bridge.send_command(material(42)).expect("first");
        let accepted = endpoint.router.lock().expect("router").pending[&first].accepted_at;
        let second = bridge.send_command(material(43)).expect("second");
        assert_eq!(receiver.try_recv().expect("active").id, first);
        let expired =
            endpoint.expire_pending_commands(generation, accepted + Duration::from_secs(11));
        assert_eq!(expired.len(), 2);
        endpoint
            .send_timeout_results(expired)
            .await
            .expect("timeouts");
        assert!(receiver.try_recv().is_err());
        endpoint
            .route_command_result_for_contract_test(first, success())
            .expect("late native terminal");
        assert!(receiver.try_recv().is_err());
        assert!(!endpoint.command_pending_for_contract_test(second));
        assert_eq!(
            bridge.send_command(material(44)),
            Err(PlatformBridgeError::Disconnected)
        );
    }

    #[test]
    fn material_queued_deadline_and_disconnect_retire_exact_original_ids() {
        let (bridge, endpoint) = platform_bridge(8);
        let receiver = endpoint.open_connection_for_contract_test();
        let first = bridge.send_command(material(42)).expect("first");
        let second = bridge.send_command(material(43)).expect("second");
        assert_eq!(receiver.try_recv().expect("active").id, first);
        endpoint.set_connected_for_contract_test(false);
        let Ok(PlatformNotification::Disconnected {
            pending_command_ids,
            ..
        }) = bridge.notification_receiver().try_recv()
        else {
            panic!("disconnect");
        };
        assert_eq!(pending_command_ids.len(), 2);
        assert!(pending_command_ids.contains(&first) && pending_command_ids.contains(&second));
        let replacement = endpoint.open_connection_for_contract_test();
        let latest = bridge.send_command(material(44)).expect("replacement");
        assert_eq!(
            replacement.try_recv().expect("replacement active").id,
            latest
        );
        endpoint
            .route_command_result_for_contract_test(first, success())
            .expect("retired result");
        assert!(replacement.try_recv().is_err());
        assert!(endpoint.command_pending_for_contract_test(latest));
    }

    #[test]
    fn material_expired_queued_request_is_not_sent_after_active_terminal() {
        let (bridge, endpoint) = platform_bridge(8);
        let receiver = endpoint.open_connection_for_contract_test();
        let first = bridge.send_command(material(42)).expect("first");
        let second = bridge.send_command(material(43)).expect("second");
        assert_eq!(receiver.try_recv().expect("active").id, first);
        let generation = bridge.connection_generation().expect("connection");
        let now = Instant::now();
        endpoint
            .router
            .lock()
            .expect("router")
            .pending
            .get_mut(&second)
            .expect("queued pending")
            .accepted_at = now - Duration::from_secs(11);
        let expired = endpoint.expire_pending_commands(generation, now);
        assert_eq!(expired.len(), 1);
        assert_eq!(expired[0].id, second);
        endpoint
            .route_command_result_for_contract_test(first, success())
            .expect("active terminal");
        assert!(
            receiver.try_recv().is_err(),
            "expired queued HWND must never reach native code"
        );
        let next = bridge
            .send_command(material(44))
            .expect("lane stays available");
        assert_eq!(receiver.try_recv().expect("next").id, next);
    }
}

#[cfg(test)]
mod routing_tests {
    use super::*;
    use nxtools_protocol::window_material::{
        ApplyWindowMaterialCommand, MaterialSurface, WindowMaterialStyle,
    };

    #[tokio::test]
    async fn material_production_terminal_wakes_next_without_accepting_replaced_generation() {
        let (bridge, endpoint) = platform_bridge(8);
        let original = endpoint.open_connection_for_contract_test();
        let old_generation = bridge.connection_generation().expect("original generation");
        let id = Uuid::new_v4();
        let command = PlatformCommand::ApplyWindowMaterial(ApplyWindowMaterialCommand {
            session_id: Uuid::new_v4(),
            facts_revision: 1,
            window_handle: 42,
            generation: 1,
            style: WindowMaterialStyle::Acrylic,
            surface: MaterialSurface::Backdrop,
            dark: true,
        });
        bridge
            .send_command_with_id(command.clone(), id)
            .expect("original");
        assert_eq!(original.try_recv().expect("original dispatch").id, id);
        let replacement = endpoint.open_connection_for_contract_test();
        let generation = bridge
            .connection_generation()
            .expect("replacement generation");
        let _ = bridge
            .notification_receiver()
            .try_recv()
            .expect("replacement retires old pending");
        bridge
            .send_command_with_id(command.clone(), id)
            .expect("same ID new generation");
        assert_eq!(replacement.try_recv().expect("replacement dispatch").id, id);
        let next = bridge.send_command(command).expect("replacement waiting");
        let result = PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: None,
        };
        endpoint
            .route_command_result(old_generation, id, result.clone())
            .await
            .expect("old result");
        assert!(replacement.try_recv().is_err());
        assert!(endpoint.command_pending_for_contract_test(id));
        endpoint
            .route_command_result(generation, id, result)
            .await
            .expect("current result");
        assert_eq!(replacement.recv().await.expect("terminal wake").id, next);
        assert!(
            matches!(bridge.notification_receiver().try_recv(), Ok(PlatformNotification::CommandResult { correlation_id, .. }) if correlation_id == id)
        );
        endpoint
            .route_command_result(
                generation,
                next,
                failed_platform_result("window_material_unavailable"),
            )
            .await
            .expect("bounded native unavailable");
        assert!(!endpoint.command_pending_for_contract_test(next));
    }
}
