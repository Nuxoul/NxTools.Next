use super::*;
use nxtools_protocol::installed_applications::InstalledApplications;

pub(super) struct PendingInstalledApplications {
    id: uuid::Uuid,
    connection: u64,
    started: Instant,
}

impl QuickLaunchApplication {
    pub(super) fn request_installed_applications(&mut self, cx: &mut Context<Self>) {
        if !self.feature_enabled() || self.pending_installed_applications.is_some() {
            return;
        }
        let Some(connection) = self.platform.connection_generation() else {
            return;
        };
        let id = uuid::Uuid::new_v4();
        self.pending_installed_applications = Some(PendingInstalledApplications {
            id,
            connection,
            started: Instant::now(),
        });
        info!(event = "quick_launch.applications.accepted", correlation_id = %id, connection);
        if self
            .platform
            .send_command_with_id(PlatformCommand::QueryInstalledApplications, id)
            .is_err()
        {
            self.retire_installed_applications("dispatch_failed");
            return;
        }
        cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(15))
                .await;
            let _ = owner.update(cx, |owner, _| {
                if owner
                    .pending_installed_applications
                    .as_ref()
                    .is_some_and(|pending| pending.id == id)
                {
                    owner.retire_installed_applications("timeout");
                }
            });
        })
        .detach();
    }

    pub(super) fn retire_installed_applications(&mut self, reason: &'static str) {
        if let Some(pending) = self.pending_installed_applications.take() {
            warn!(event = "quick_launch.applications.completed", correlation_id = %pending.id,
                success = false, reason, elapsed_ms = pending.started.elapsed().as_millis() as u64);
        }
    }

    pub(super) fn installed_applications_result(
        &mut self,
        id: uuid::Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        if self
            .pending_installed_applications
            .as_ref()
            .is_none_or(|pending| pending.id != id)
        {
            return false;
        }
        let pending = self
            .pending_installed_applications
            .take()
            .expect("matching installed application request");
        let current = self.feature_enabled()
            && self.platform.connection_generation() == Some(pending.connection);
        let snapshot = current
            .then_some(result)
            .filter(|result| result.success)
            .and_then(|result| result.data.as_deref())
            .and_then(|data| serde_json::from_str::<InstalledApplications>(data).ok());
        let applied = snapshot.is_some_and(|snapshot| {
            self.catalog
                .borrow_mut()
                .replace_installed_applications(
                    snapshot
                        .applications
                        .into_iter()
                        .map(|application| nxtools_finder::InstalledApplication {
                            name: application.name,
                            target: application.target,
                        })
                        .collect(),
                )
                .is_ok()
        });
        let stage = if !current {
            "retired"
        } else if !result.success {
            "native_failed"
        } else if applied {
            "applied"
        } else {
            "snapshot_rejected"
        };
        info!(event = "quick_launch.applications.completed", correlation_id = %id,
            success = applied, current, stage, native_error_code = ?result.native_error_code,
            elapsed_ms = pending.started.elapsed().as_millis() as u64);
        if applied {
            self.refresh_catalog_results(cx);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;

    fn snapshot(name: &str) -> PlatformCommandResult {
        PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some(serde_json::json!({"applications": [{"name": name, "target": "shell:AppsFolder\\Future.Tool_fixture!App"}]}).to_string()),
        }
    }

    fn receive(endpoint: &nxtools_ipc::PlatformServerEndpoint) -> uuid::Uuid {
        let envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("native discovery command");
        assert!(matches!(
            envelope.message,
            nxtools_protocol::Message::PlatformCommand(PlatformCommand::QueryInstalledApplications)
        ));
        envelope.id
    }

    #[gpui_kit::test]
    fn windows_apps_refresh_coalesces_and_preserves_last_good_until_successful_empty(
        cx: &mut TestAppContext,
    ) {
        let (_temp, application, endpoint) =
            super::super::tests::quick_launch_application_fixture(cx);
        application.update(cx, |app, cx| {
            app.request_installed_applications(cx);
            app.request_installed_applications(cx);
        });
        let id = receive(&endpoint);
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        application.update(cx, |app, cx| {
            app.handle_command_result(id, snapshot("未来系统工具"), cx);
            assert_eq!(
                app.catalog
                    .borrow()
                    .search("未来系统工具", 10)
                    .expect("search")
                    .len(),
                1
            );
            assert!(app.popup.is_none(), "discovery must not open a panel");
            app.request_installed_applications(cx);
        });
        let failed_id = receive(&endpoint);
        application.update(cx, |app, cx| {
            let mut failed = snapshot("ignored");
            failed.success = false;
            app.handle_command_result(failed_id, failed, cx);
            assert_eq!(
                app.catalog
                    .borrow()
                    .search("未来系统工具", 10)
                    .expect("last good")
                    .len(),
                1
            );
            app.request_installed_applications(cx);
        });
        let empty_id = receive(&endpoint);
        application.update(cx, |app, cx| {
            let mut empty = snapshot("ignored");
            empty.data = Some(r#"{"applications":[]}"#.into());
            app.handle_command_result(empty_id, empty, cx);
            assert!(
                app.catalog
                    .borrow()
                    .search("未来系统工具", 10)
                    .expect("removed")
                    .is_empty()
            );
            assert!(app.popup.is_none());
        });
    }

    #[gpui_kit::test]
    fn windows_apps_disconnected_disabled_and_retired_receipts_cannot_publish(
        cx: &mut TestAppContext,
    ) {
        let (_temp, application, endpoint) =
            super::super::tests::quick_launch_application_fixture(cx);
        application.update(cx, |app, cx| app.request_installed_applications(cx));
        let old_id = receive(&endpoint);
        endpoint.set_connected_for_contract_test(false);
        endpoint.set_connected_for_contract_test(true);
        application.update(cx, |app, cx| {
            app.handle_command_result(old_id, snapshot("旧连接应用"), cx);
            assert!(
                app.catalog
                    .borrow()
                    .search("旧连接应用", 10)
                    .expect("search")
                    .is_empty()
            );
            app.request_installed_applications(cx);
        });
        let disabled_id = receive(&endpoint);
        application.update(cx, |app, cx| {
            app.set_feature_enabled(false, cx);
            assert!(app.pending_installed_applications.is_none());
            app.handle_command_result(disabled_id, snapshot("禁用后应用"), cx);
            assert!(
                app.catalog
                    .borrow()
                    .search("禁用后应用", 10)
                    .expect("search")
                    .is_empty()
            );
            app.config.quick_launch.enabled = true;
            app.request_installed_applications(cx);
        });
        // Configuration belongs to the original total-switch owner.
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            if matches!(
                envelope.message,
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::QueryInstalledApplications
                )
            ) {
                application.update(cx, |app, cx| {
                    app.retire_installed_applications("timeout");
                    app.handle_command_result(envelope.id, snapshot("超时应用"), cx);
                    assert!(
                        app.catalog
                            .borrow()
                            .search("超时应用", 10)
                            .expect("search")
                            .is_empty()
                    );
                });
                return;
            }
        }
        panic!("new native discovery request");
    }

    #[test]
    fn windows_apps_open_uses_shell_namespace_without_file_capabilities() {
        let directory = tempfile::tempdir().expect("catalog fixture");
        let mut catalog =
            FinderCatalog::open(directory.path().join("history.db"), &[]).expect("catalog");
        catalog
            .replace_installed_applications(vec![nxtools_finder::InstalledApplication {
                name: "Calculator".into(),
                target: "shell:AppsFolder\\Microsoft.WindowsCalculator_8wekyb3d8bbwe!App".into(),
            }])
            .expect("snapshot");
        let result = catalog.search("计算器", 10).expect("search").remove(0);
        assert!(
            matches!(platform_command_for_action(&result, &QuickLaunchAction::Open, 0), Some(PlatformCommand::OpenShellTarget(command)) if command.action == ShellOpenAction::Open)
        );
        for action in [
            QuickLaunchAction::Reveal,
            QuickLaunchAction::RunElevated,
            QuickLaunchAction::CopyPath,
            QuickLaunchAction::Shell {
                session_id: "fixture".into(),
                command_id: 1,
            },
        ] {
            assert!(!quick_launch_action_available(&result, &action));
            assert!(platform_command_for_action(&result, &action, 0).is_none());
        }
        let capabilities = quick_launch_result_capabilities(&result);
        assert!(!capabilities.preview);
        assert!(!capabilities.shell_actions);
        assert!(!capabilities.shell_icon);
    }

    #[gpui_kit::test]
    fn windows_apps_receipt_refreshes_live_query_without_reopening_closed_panel(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (_temp, application, endpoint) =
            super::super::tests::quick_launch_application_fixture(cx);
        let (panel, window) = super::super::tests::quick_launch_panel_fixture(cx, Vec::new());
        let handle = window.update(|window, _| window.window_handle());
        application.update(window, |app, cx| {
            app.popup = Some(QuickLaunchWindow {
                window: handle,
                panel: panel.downgrade(),
                native_window_handle: 1,
            });
            app.request_installed_applications(cx);
        });
        let id = receive(&endpoint);
        panel.update_in(window, |panel, window, cx| {
            panel.is_open = true;
            panel
                .search_input
                .update(cx, |input, cx| input.set_value("未来", window, cx));
        });
        application.update(window, |app, cx| {
            app.handle_command_result(id, snapshot("未来系统工具"), cx)
        });
        panel.read_with(window, |panel, cx| {
            assert!(panel.is_open);
            assert_eq!(panel.search_input.read(cx).value().as_ref(), "未来");
            assert_eq!(panel.results.len(), 1);
            assert_eq!(panel.results[0].entry.name, "未来系统工具");
            assert_eq!(panel.results[0].activation_count, 0);
        });
        application.update(window, |app, cx| app.request_installed_applications(cx));
        let id = receive(&endpoint);
        panel.update_in(window, |panel, _, _| panel.is_open = false);
        application.update(window, |app, cx| {
            app.handle_command_result(id, snapshot("关闭后应用"), cx)
        });
        panel.read_with(window, |panel, _| {
            assert!(!panel.is_open);
            assert_eq!(panel.results[0].entry.name, "未来系统工具");
        });
    }

    #[gpui_kit::test]
    fn windows_apps_timeout_retires_the_actual_pending_request(cx: &mut TestAppContext) {
        let (_temp, application, endpoint) =
            super::super::tests::quick_launch_application_fixture(cx);
        application.update(cx, |app, cx| app.request_installed_applications(cx));
        let id = receive(&endpoint);
        cx.executor().run_until_parked();
        cx.executor().advance_clock(Duration::from_secs(16));
        cx.executor().run_until_parked();
        application.update(cx, |app, cx| {
            assert!(app.pending_installed_applications.is_none());
            app.handle_command_result(id, snapshot("超时应用"), cx);
            assert!(
                app.catalog
                    .borrow()
                    .search("超时应用", 10)
                    .expect("search")
                    .is_empty()
            );
        });
    }
}
