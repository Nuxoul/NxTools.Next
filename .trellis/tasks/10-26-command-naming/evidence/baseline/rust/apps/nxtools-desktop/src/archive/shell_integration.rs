//! P206 explicit Apply/Revert/Status request owner. Saving the desired intent never applies it;
//! only the correlated native observed result updates the displayed applied state.
use super::*;
use nxtools_protocol::archive_shell_integration::{
    ArchiveShellIntegrationAction, ArchiveShellIntegrationCommand, ArchiveShellIntegrationResult,
    VERSION,
};

pub(crate) type IntegrationReply = Rc<dyn Fn(IntegrationOutcome, &mut App)>;

/// Typed outcome delivered on a later application turn to the requesting view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IntegrationOutcome {
    Observed(ArchiveShellIntegrationResult),
    DefaultAppsOpened,
    /// The request never reached the native owner or its reply was malformed/failed.
    NotDelivered(&'static str),
}

pub(super) struct PendingIntegration {
    action: Option<ArchiveShellIntegrationAction>,
    started: Instant,
    reply: IntegrationReply,
}

/// Requests one explicit integration action through the existing archive owner.
pub(crate) fn request(
    action: ArchiveShellIntegrationAction,
    desired: nxtools_core::archive::ArchiveShellIntegrationSettings,
    reply: impl Fn(IntegrationOutcome, &mut App) + 'static,
    cx: &mut App,
) {
    let reply: IntegrationReply = Rc::new(reply);
    let Some(owner) = cx.try_global::<ArchiveGlobal>().map(|g| g.0.downgrade()) else {
        cx.defer(move |cx| reply(IntegrationOutcome::NotDelivered("owner_unavailable"), cx));
        return;
    };
    cx.defer(move |cx| {
        let failed = reply.clone();
        if owner
            .update(cx, |app, cx| {
                app.request_integration(action, desired, reply, cx)
            })
            .is_err()
        {
            failed(IntegrationOutcome::NotDelivered("owner_unavailable"), cx);
        }
    });
}

pub(crate) fn open_default_apps(
    reply: impl Fn(IntegrationOutcome, &mut App) + 'static,
    cx: &mut App,
) {
    let reply: IntegrationReply = Rc::new(reply);
    let Some(owner) = cx
        .try_global::<ArchiveGlobal>()
        .map(|global| global.0.downgrade())
    else {
        cx.defer(move |cx| reply(IntegrationOutcome::NotDelivered("owner_unavailable"), cx));
        return;
    };
    cx.defer(move |cx| {
        let fallback = reply.clone();
        if owner
            .update(cx, |app, cx| app.request_default_apps(reply, cx))
            .is_err()
        {
            fallback(IntegrationOutcome::NotDelivered("owner_unavailable"), cx);
        }
    });
}

impl ArchiveApplication {
    fn request_default_apps(&mut self, reply: IntegrationReply, cx: &mut Context<Self>) {
        if self.integrations.len() >= 4 {
            cx.defer(move |cx| reply(IntegrationOutcome::NotDelivered("busy"), cx));
            return;
        }
        let correlation_id = Uuid::new_v4();
        self.integrations.insert(
            correlation_id,
            PendingIntegration {
                action: None,
                started: Instant::now(),
                reply: reply.clone(),
            },
        );
        tracing::info!(event="archive.default_apps.started",%correlation_id,"default applications settings requested");
        if self
            .platform
            .send_command_with_id(
                nxtools_protocol::PlatformCommand::OpenDefaultAppsSettings(
                    nxtools_protocol::OpenDefaultAppsSettingsCommand {},
                ),
                correlation_id,
            )
            .is_err()
        {
            self.integrations.remove(&correlation_id);
            tracing::warn!(event="archive.default_apps.finished",%correlation_id,outcome="not_submitted","default applications settings request failed");
            cx.defer(move |cx| reply(IntegrationOutcome::NotDelivered("disconnected"), cx));
        }
    }
    pub(crate) fn request_integration(
        &mut self,
        action: ArchiveShellIntegrationAction,
        desired: nxtools_core::archive::ArchiveShellIntegrationSettings,
        reply: IntegrationReply,
        cx: &mut Context<Self>,
    ) {
        let command = ArchiveShellIntegrationCommand {
            version: VERSION,
            action,
            desired_revision: desired.desired_revision,
            classic_cascade: desired.classic_cascade,
            open_with: desired.open_with,
            extensions: desired.extensions,
            double_click_action: desired.double_click_action.wire_name().into(),
            menu_layout: desired.menu_layout.wire_name().into(),
            menu_commands: desired
                .menu_commands
                .iter()
                .map(|command| command.wire_name().to_owned())
                .collect(),
        };
        let correlation_id = Uuid::new_v4();
        let finish = |reply: IntegrationReply, outcome, cx: &mut Context<Self>| {
            cx.defer(move |cx| reply(outcome, cx));
        };
        if command.validate().is_err() {
            tracing::warn!(event = "archive.shell_integration.request_rejected", %correlation_id, action = ?action, reason = "invalid_request", "archive integration request rejected before native delivery");
            finish(
                reply,
                IntegrationOutcome::NotDelivered("invalid_request"),
                cx,
            );
            return;
        }
        if self.integrations.len() >= 4 {
            finish(reply, IntegrationOutcome::NotDelivered("busy"), cx);
            return;
        }
        self.integrations.insert(
            correlation_id,
            PendingIntegration {
                action: Some(action),
                started: Instant::now(),
                reply: reply.clone(),
            },
        );
        if self
            .platform
            .send_command_with_id(
                nxtools_protocol::PlatformCommand::ArchiveShellIntegration(command),
                correlation_id,
            )
            .is_err()
        {
            self.integrations.remove(&correlation_id);
            tracing::warn!(event = "archive.shell_integration.request_finished", %correlation_id, action = ?action, outcome = "not_submitted", "archive integration request not submitted");
            finish(reply, IntegrationOutcome::NotDelivered("disconnected"), cx);
            return;
        }
        tracing::info!(event = "archive.shell_integration.request_started", %correlation_id, action = ?action, desired_revision = desired.desired_revision, "archive integration request accepted");
    }

    /// Returns true when the result belonged to an integration request.
    pub(super) fn integration_result(
        &mut self,
        correlation_id: Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(pending) = self.integrations.remove(&correlation_id) else {
            return false;
        };
        let outcome =
            if !result.success {
                IntegrationOutcome::NotDelivered("native_command_failed")
            } else if pending.action.is_none() {
                if result.data.as_deref() == Some("opened") {
                    IntegrationOutcome::DefaultAppsOpened
                } else {
                    IntegrationOutcome::NotDelivered("invalid_response")
                }
            } else {
                match result.data.as_deref().and_then(|data| {
                    serde_json::from_str::<ArchiveShellIntegrationResult>(data).ok()
                }) {
                    Some(observed) if observed.validate().is_ok() => {
                        IntegrationOutcome::Observed(observed)
                    }
                    _ => IntegrationOutcome::NotDelivered("invalid_response"),
                }
            };
        let state = match &outcome {
            IntegrationOutcome::Observed(observed) => format!("{:?}", observed.state),
            IntegrationOutcome::NotDelivered(reason) => (*reason).to_owned(),
            IntegrationOutcome::DefaultAppsOpened => "default_apps_opened".into(),
        };
        tracing::info!(event = "archive.shell_integration.request_finished", %correlation_id, action = ?pending.action, outcome = %state, native_error_code = ?result.native_error_code, elapsed_ms = pending.started.elapsed().as_millis() as u64, "archive integration request finished");
        let reply = pending.reply;
        cx.defer(move |cx| reply(outcome, cx));
        true
    }

    pub(super) fn integrations_disconnected(&mut self, cx: &mut Context<Self>) {
        for (_, pending) in self.integrations.drain() {
            let reply = pending.reply;
            cx.defer(move |cx| reply(IntegrationOutcome::NotDelivered("disconnected"), cx));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;
    use nxtools_core::archive::ArchiveShellIntegrationSettings;
    use nxtools_protocol::archive_shell_integration::ArchiveShellIntegrationState;
    use nxtools_protocol::{Message, PlatformCommand, PlatformCommandResult};
    use std::cell::RefCell;

    fn reply(data: &str, id: Uuid) -> PlatformNotification {
        PlatformNotification::CommandResult {
            correlation_id: id,
            result: PlatformCommandResult {
                success: true,
                error: None,
                native_error_code: None,
                data: Some(data.into()),
            },
        }
    }

    #[gpui_kit::test]
    fn explicit_request_sends_once_and_reports_only_observed_native_state(cx: &mut TestAppContext) {
        let (owner, endpoint) = super::super::tests::setup(cx);
        let seen = Rc::new(RefCell::new(Vec::<IntegrationOutcome>::new()));
        let record = |seen: &Rc<RefCell<Vec<IntegrationOutcome>>>| -> IntegrationReply {
            let seen = seen.clone();
            Rc::new(move |outcome, _| seen.borrow_mut().push(outcome))
        };
        let desired = ArchiveShellIntegrationSettings {
            classic_cascade: true,
            open_with: false,
            extensions: vec![".zip".into(), ".7z".into()],
            desired_revision: 7,
            double_click_action: nxtools_core::archive::ArchiveDoubleClickAction::Smart,
            menu_layout: nxtools_core::archive::ArchiveMenuLayout::Flat,
            menu_commands: vec![
                nxtools_core::archive::ArchiveMenuCommand::Test,
                nxtools_core::archive::ArchiveMenuCommand::Create,
            ],
        };
        owner.update(cx, |app, cx| {
            app.request_integration(
                ArchiveShellIntegrationAction::Apply,
                desired.clone(),
                record(&seen),
                cx,
            )
        });
        cx.run_until_parked();
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("one explicit native request");
        let Message::PlatformCommand(PlatformCommand::ArchiveShellIntegration(wire)) =
            &command.message
        else {
            panic!("typed archive shell integration command");
        };
        assert_eq!(wire.action, ArchiveShellIntegrationAction::Apply);
        assert_eq!(wire.desired_revision, 7);
        assert_eq!(wire.extensions, vec![".zip".to_owned(), ".7z".to_owned()]);
        assert_eq!(wire.double_click_action, "smart");
        assert_eq!(wire.menu_layout, "flat");
        assert_eq!(wire.menu_commands, vec!["test", "create"]);
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        assert!(seen.borrow().is_empty(), "no state before the native reply");

        owner.update(cx, |app, cx| {
            app.notification(
                reply(
                    r#"{"version":1,"state":"disabled","desired_revision":7,"applied_revision":null,"applied_values":0,"removed":0,"restored":0,"preserved":0,"failure":"apply_not_authorized"}"#,
                    command.id,
                ),
                cx,
            )
        });
        cx.run_until_parked();
        assert!(matches!(
            seen.borrow().as_slice(),
            [IntegrationOutcome::Observed(result)]
                if result.state == ArchiveShellIntegrationState::Disabled
                    && result.applied_revision.is_none()
        ));
        owner.update(cx, |app, cx| app.notification(reply("{}", command.id), cx));
        cx.run_until_parked();
        assert_eq!(seen.borrow().len(), 1, "late duplicate reply is ignored");

        // A forged applied state without an applied revision is not displayed as applied.
        owner.update(cx, |app, cx| {
            app.request_integration(
                ArchiveShellIntegrationAction::Status,
                desired.clone(),
                record(&seen),
                cx,
            )
        });
        let status = endpoint
            .try_receive_command_for_contract_test()
            .expect("status request");
        owner.update(cx, |app, cx| {
            app.notification(
                reply(
                    r#"{"version":1,"state":"applied","desired_revision":7,"applied_revision":null,"applied_values":3,"removed":0,"restored":0,"preserved":0,"failure":null}"#,
                    status.id,
                ),
                cx,
            )
        });
        cx.run_until_parked();
        assert_eq!(
            seen.borrow().last(),
            Some(&IntegrationOutcome::NotDelivered("invalid_response"))
        );

        // Invalid desired intent never reaches native code.
        owner.update(cx, |app, cx| {
            app.request_integration(
                ArchiveShellIntegrationAction::Apply,
                ArchiveShellIntegrationSettings {
                    classic_cascade: true,
                    ..Default::default()
                },
                record(&seen),
                cx,
            )
        });
        cx.run_until_parked();
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        assert_eq!(
            seen.borrow().last(),
            Some(&IntegrationOutcome::NotDelivered("invalid_request"))
        );

        // Disconnect retires pending requests with an explicit non-applied outcome.
        owner.update(cx, |app, cx| {
            app.request_integration(
                ArchiveShellIntegrationAction::Revert,
                desired,
                record(&seen),
                cx,
            )
        });
        let revert = endpoint
            .try_receive_command_for_contract_test()
            .expect("revert request");
        owner.update(cx, |app, cx| {
            app.notification(
                PlatformNotification::Disconnected {
                    generation: 1,
                    pending_command_ids: vec![revert.id],
                },
                cx,
            )
        });
        cx.run_until_parked();
        assert_eq!(
            seen.borrow().last(),
            Some(&IntegrationOutcome::NotDelivered("disconnected"))
        );
        assert!(owner.read_with(cx, |app, _| app.integrations.is_empty()));
    }
}

#[cfg(test)]
mod settings_completion_tests {
    use super::*;
    #[gpui_kit::test]
    fn shell_settings_default_apps_uses_original_pending_and_terminal_boundary(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        let (owner, endpoint) = super::super::tests::setup(cx);
        let outcomes = Rc::new(std::cell::RefCell::new(Vec::new()));
        let seen = outcomes.clone();
        owner.update(cx, |app, cx| {
            app.request_default_apps(
                Rc::new(move |outcome, _| seen.borrow_mut().push(outcome)),
                cx,
            )
        });
        let command = endpoint
            .try_receive_command_for_contract_test()
            .expect("typed request");
        assert!(matches!(
            command.message,
            nxtools_protocol::Message::PlatformCommand(
                nxtools_protocol::PlatformCommand::OpenDefaultAppsSettings(_)
            )
        ));
        assert!(owner.read_with(cx, |app, _| app.integrations.contains_key(&command.id)));
        assert!(outcomes.borrow().is_empty());
        owner.update(cx, |app, cx| {
            app.notification(
                nxtools_ipc::PlatformNotification::CommandResult {
                    correlation_id: command.id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: false,
                        error: Some("fixed fixture failure".into()),
                        native_error_code: Some(5),
                        data: None,
                    },
                },
                cx,
            )
        });
        cx.run_until_parked();
        assert_eq!(
            *outcomes.borrow(),
            vec![IntegrationOutcome::NotDelivered("native_command_failed")]
        );
        assert!(owner.read_with(cx, |app, _| app.integrations.is_empty()));
        owner.update(cx, |app, cx| {
            app.notification(
                nxtools_ipc::PlatformNotification::CommandResult {
                    correlation_id: command.id,
                    result: nxtools_protocol::PlatformCommandResult {
                        success: true,
                        error: None,
                        native_error_code: None,
                        data: Some("opened".into()),
                    },
                },
                cx,
            )
        });
        cx.run_until_parked();
        assert_eq!(
            outcomes.borrow().len(),
            1,
            "late result cannot resurrect completed request"
        );
    }
}
