//! Programmatic search prefill through the same summon/query owner as the keyboard entry.
use super::*;
impl QuickLaunchApplication {
    pub(crate) fn show_floating_search(
        &mut self,
        text: String,
        request: QuickLaunchPanelRequest,
        cx: &mut Context<Self>,
    ) {
        if !self.admits_feature_event(request.requested_at_unix_ms)
            || text.len() > 196_608
            || text.contains('\0')
        {
            return;
        }
        if let Some(popup) = self.popup.clone() {
            self.show_existing_popup(popup, request, cx);
        } else {
            self.open_popup(request, cx);
        }
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let session = self.popup_session_id;
        // Capture the original panel generation; a newer edit or summon must win.
        let Some(panel) = popup.panel.upgrade() else {
            return;
        };
        let query_generation = panel.read(cx).query_dispatch_generation;
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            if owner
                .upgrade()
                .is_none_or(|owner| owner.read(cx).popup_session_id != session)
            {
                return;
            }
            let _ = popup.window.update(cx, |_, window, cx| {
                let input = popup.panel.upgrade().and_then(|panel| {
                    let panel = panel.read(cx);
                    (panel.is_open && panel.query_dispatch_generation == query_generation)
                        .then(|| panel.search_input.clone())
                        .filter(|input| input.read(cx).value().is_empty())
                });
                if let Some(input) = input {
                    input.update(cx, |input, cx| input.set_value(text, window, cx));
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::TestAppContext;

    fn request(target: i64, cursor_x: i32) -> QuickLaunchPanelRequest {
        QuickLaunchPanelRequest {
            target: nxtools_protocol::ForegroundContext {
                window_handle: target,
                focus_window_handle: target,
                process_id: 1,
                process_name: "fixture".into(),
                window_title: String::new(),
            },
            directory_navigation_context: None,
            cursor_x,
            cursor_y: 100,
            cursor_display_index: 0,
            cursor_monitor_dpi: 96,
            cursor_monitor: None,
            requested_at_unix_ms: 1_700_000_000_000,
            correlation_id: None,
        }
    }

    fn query(application: &Entity<QuickLaunchApplication>, cx: &TestAppContext) -> String {
        cx.read(|cx| {
            let popup = application.read(cx).popup.clone().expect("original popup");
            let panel = popup.panel.upgrade().expect("original panel");
            panel.read(cx).search_input.read(cx).value().to_string()
        })
    }

    fn seed_native_popup(application: &Entity<QuickLaunchApplication>, cx: &mut TestAppContext) {
        let owner = application.downgrade();
        let slot = Rc::new(RefCell::new(None));
        let capture = slot.clone();
        let _ = cx.add_window_view(move |window, cx| {
            let panel = cx.new(|cx| {
                QuickLaunchPanel::new(
                    vec![],
                    request(91, 200).target,
                    None,
                    vec![],
                    owner,
                    window,
                    cx,
                )
            });
            *capture.borrow_mut() = Some(QuickLaunchWindow {
                window: window.window_handle(),
                panel: panel.downgrade(),
                native_window_handle: 9001,
            });
            Root::new(panel, window, cx)
        });
        application.update(cx, |app, _| app.popup = slot.borrow_mut().take());
    }

    #[gpui_kit::test]
    fn feature_disable_retires_quick_launch_and_rejects_queued_summons(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (temp, application, endpoint) =
            super::super::tests::quick_launch_application_fixture(cx);
        let path = temp.path().join("config.json");
        let mut summon = request(91, 200);
        summon.requested_at_unix_ms = nxtools_ipc::unix_time_ms();
        application.update(cx, |app, cx| {
            app.show_floating_search("retire me".into(), summon.clone(), cx);
        });
        assert!(!cx.windows().is_empty());
        // An unwritable target exposes the real section-save failure without changing the live popup.
        std::fs::remove_file(&path).expect("remove fixture config");
        std::fs::create_dir(&path).expect("create unwritable config target");
        application.update(cx, |app, cx| {
            let session = app.popup_session_id;
            app.set_feature_enabled(false, cx);
            assert!(app.settings().enabled);
            assert_eq!(app.popup_session_id, session);
            assert!(app.feature_configuration_status().starts_with("保存失败"));
        });
        std::fs::remove_dir(&path).expect("remove unwritable target");
        AppConfig::default()
            .save_to(&path)
            .expect("restore fixture config");
        application.update(cx, |app, cx| {
            app.set_feature_enabled(false, cx);
            app.show_floating_search("disabled".into(), summon.clone(), cx);
            app.handle_platform_event(PlatformEvent::QuickLaunchPanelRequested(summon.clone()), cx);
            assert!(app.popup.is_none());
            assert!(app.pending_window_activation.is_empty());
            assert!(app.active_summon.is_none());
            app.set_feature_enabled(true, cx);
            app.show_floating_search("stale".into(), summon.clone(), cx);
            app.handle_platform_event(PlatformEvent::QuickLaunchPanelRequested(summon), cx);
            assert!(app.popup.is_none());
        });
        cx.run_until_parked();
        assert!(cx.windows().is_empty());
        let applied = std::iter::from_fn(|| endpoint.try_receive_command_for_contract_test().ok())
            .filter_map(|envelope| match envelope.message {
                nxtools_protocol::Message::PlatformCommand(
                    PlatformCommand::ConfigureQuickLaunch(command),
                ) => Some(command.enabled),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(applied, [false, true]);
    }

    #[gpui_kit::test]
    fn floating_search_uses_original_popup_input_target_and_reuses_window(cx: &mut TestAppContext) {
        cx.update(gpui_kit::init);
        let (_temporary, application, endpoint) =
            super::super::tests::quick_launch_application_fixture(cx);
        seed_native_popup(&application, cx);
        application.update(cx, |app, cx| {
            app.show_floating_search("first".into(), request(91, 200), cx)
        });
        cx.run_until_parked();
        assert_eq!(query(&application, cx), "first");
        let original = application.read_with(cx, |app, _| app.popup.clone().expect("opened popup"));
        let original_input = original
            .panel
            .upgrade()
            .expect("panel")
            .read_with(cx, |panel, _| panel.search_input.entity_id());
        application.update(cx, |app, cx| {
            app.show_floating_search("second".into(), request(92, 300), cx)
        });
        cx.run_until_parked();
        assert_eq!(query(&application, cx), "second");
        application.read_with(cx, |app, cx| {
            let popup = app.popup.as_ref().expect("reused popup");
            assert_eq!(popup.native_window_handle, original.native_window_handle);
            assert!(
                popup.panel.upgrade().expect("panel").entity_id()
                    == original
                        .panel
                        .upgrade()
                        .expect("original panel")
                        .entity_id()
            );
            let panel = popup.panel.upgrade().expect("panel");
            let panel = panel.read(cx);
            assert_eq!(panel.search_input.entity_id(), original_input);
            assert_eq!(panel.target.window_handle, 92);
            assert_eq!(app.active_summon.map(|(target, _)| target), Some(92));
        });
        assert!(
            endpoint.try_receive_command_for_contract_test().is_ok(),
            "production native queue must be used"
        );
    }

    #[gpui_kit::test]
    fn floating_search_deferred_prefill_retires_on_new_session_or_live_edit(
        cx: &mut TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let (_temporary, application, _endpoint) =
            super::super::tests::quick_launch_application_fixture(cx);
        seed_native_popup(&application, cx);
        application.update(cx, |app, cx| {
            app.show_floating_search("stale".into(), request(91, 100), cx);
            app.show_floating_search("latest".into(), request(92, 200), cx);
        });
        cx.run_until_parked();
        assert_eq!(query(&application, cx), "latest");
        application.update(cx, |app, cx| {
            app.show_floating_search("deferred".into(), request(93, 300), cx)
        });
        let popup = application.read_with(cx, |app, _| app.popup.clone().expect("popup"));
        let panel = popup.panel.upgrade().expect("panel");
        let input = panel.read_with(cx, |panel, _| panel.search_input.clone());
        popup
            .window
            .update(cx, |_, window, cx| {
                input.update(cx, |input, cx| input.set_value("live edit", window, cx));
            })
            .expect("edit original input");
        cx.run_until_parked();
        assert_eq!(query(&application, cx), "live edit");
        application.update(cx, |app, cx| {
            app.show_floating_search("retired".into(), request(94, 400), cx);
            app.hide_popup(cx);
        });
        cx.run_until_parked();
        application.read_with(cx, |app, cx| {
            let panel = app
                .popup
                .as_ref()
                .expect("retained popup")
                .panel
                .upgrade()
                .expect("panel");
            assert!(
                !panel.read(cx).is_open,
                "hidden session cannot be reopened by stale prefill"
            );
        });
    }
}
