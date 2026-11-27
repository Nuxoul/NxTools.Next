//! Material readiness belongs to the primary popup's existing presentation session.
use super::*;
use crate::ui::material::UiMaterialCapabilities;
use nxtools_protocol::window_material::{AppliedWindowMaterial, MaterialSurface};

pub(super) struct PendingPopupMaterial {
    popup: PopupWindow,
    token: uuid::Uuid,
    topmost: Option<Option<String>>,
    frame_requested: Option<AppliedWindowMaterial>,
    frame_ready: Option<AppliedWindowMaterial>,
    reveal_command: Option<uuid::Uuid>,
    started: Instant,
}

impl ClipboardApplication {
    pub(super) fn current_popup_material_receipt(
        &self,
        cx: &gpui_kit::App,
    ) -> Option<AppliedWindowMaterial> {
        let popup = self.popup.lifecycle.current()?;
        self.popup
            .material
            .receipt(cx)
            .or_else(|| crate::ui::material::runtime::receipt(popup.window, cx))
            .filter(|receipt| receipt.window_handle == popup.native_window_handle)
    }

    pub(super) fn begin_popup_material(&mut self, popup: PopupWindow, cx: &mut Context<Self>) {
        self.retire_popup_material("superseded");
        if let Some(material) = crate::ui::material::runtime::take_preparation(popup.window, cx) {
            self.popup.material = material;
        }
        if !self.popup.material_observers_installed {
            self.popup.material_observers_installed = true;
            cx.observe_global::<UiMaterialCapabilities>(|_, cx| {
                Self::defer_popup_material(cx);
            })
            .detach();
            cx.observe_global::<crate::theme::UiThemeState>(|_, cx| {
                Self::defer_popup_material(cx);
            })
            .detach();
        }
        let token = uuid::Uuid::new_v4();
        info!(event="clipboard.popup.material_started", correlation_id=%token,
            session_id=%popup.session_id, window_handle=popup.native_window_handle);
        self.popup.pending_material = Some(PendingPopupMaterial {
            popup,
            token,
            topmost: None,
            frame_requested: None,
            frame_ready: None,
            reveal_command: None,
            started: Instant::now(),
        });
        self.drive_popup_material(cx);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_secs(20))
                .await;
            let _ = this.update(cx, |this, cx| this.expire_popup_material(token, cx));
        })
        .detach();
    }

    fn defer_popup_material(cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |owner, cx| owner.drive_popup_material(cx));
        });
    }

    fn popup_material_is_current(&self, pending: &PendingPopupMaterial) -> bool {
        self.config.clipboard.enabled
            && self.popup.lifecycle.current().is_some_and(|current| {
                current.window == pending.popup.window
                    && current.session_id == pending.popup.session_id
                    && current.native_window_handle == pending.popup.native_window_handle
            })
    }

    pub(super) fn retire_popup_material(&mut self, reason: &'static str) {
        self.popup.material.retire(&self.platform);
        if let Some(pending) = self.popup.pending_material.take() {
            if let Some(id) = pending.reveal_command {
                self.popup.pending_visibility.remove(&id);
            }
            // Visibility remains owned by the original popup command path. An already
            // submitted show is followed by hide, never by a material fallback activation.
            if let Err(error) = self
                .platform
                .send_command(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand {
                        window_handle: pending.popup.native_window_handle,
                        visible: false,
                    },
                ))
            {
                warn!(event="clipboard.popup.material_hide_failed", correlation_id=%pending.token,
                    session_id=%pending.popup.session_id, %error);
            }
            info!(event="clipboard.popup.material_cancelled", correlation_id=%pending.token,
                session_id=%pending.popup.session_id, reason,
                elapsed_ms=pending.started.elapsed().as_millis() as u64);
        }
    }

    fn expire_popup_material(&mut self, token: uuid::Uuid, cx: &mut Context<Self>) {
        if self
            .popup
            .pending_material
            .as_ref()
            .is_some_and(|pending| pending.token == token)
        {
            self.retire_popup_material("timeout");
            if let Some(previous) = self.popup.lifecycle.reset_pending() {
                cx.defer(move |cx| {
                    let _ = previous
                        .window
                        .update(cx, |_, window, _| window.remove_window());
                });
            }
        }
    }

    pub(super) fn finish_popup_material(
        &mut self,
        id: uuid::Uuid,
        result: &nxtools_protocol::PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(outcome) = self.popup.material.accept(id, result, cx) else {
            return false;
        };
        match outcome {
            Ok(_) => self.drive_popup_material(cx),
            Err(reason) => self.retire_popup_material(reason),
        }
        true
    }

    pub(super) fn popup_material_topmost(
        &mut self,
        reason: Option<String>,
        cx: &mut Context<Self>,
    ) {
        if let Some(pending) = self.popup.pending_material.as_mut() {
            pending.topmost = Some(reason);
        }
        self.drive_popup_material(cx);
    }

    pub(super) fn drive_popup_material(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.popup.pending_material.as_ref() else {
            return;
        };
        if !self.popup_material_is_current(pending) {
            self.retire_popup_material("stale_popup");
            return;
        }
        let native = pending.popup.native_window_handle;
        let prepared =
            match self
                .popup
                .material
                .ensure(&self.platform, native, MaterialSurface::Transient, cx)
            {
                Ok(ready) => ready,
                Err(reason) => {
                    self.retire_popup_material(reason);
                    return;
                }
            };
        let pending = self
            .popup
            .pending_material
            .as_mut()
            .expect("current preparation");
        if !prepared {
            pending.frame_ready = None;
            if let Some(id) = pending.reveal_command.take() {
                self.popup.pending_visibility.remove(&id);
                if self
                    .platform
                    .send_command(PlatformCommand::SetWindowVisibility(
                        SetWindowVisibilityCommand {
                            window_handle: native,
                            visible: false,
                        },
                    ))
                    .is_err()
                {
                    self.retire_popup_material("superseded_reveal_hide_enqueue");
                }
            }
            return;
        }
        let receipt = self.popup.material.receipt(cx).expect("qualified material");
        if pending.topmost.is_none() || pending.reveal_command.is_some() {
            return;
        }
        if pending.frame_ready != Some(receipt) {
            if pending.frame_requested == Some(receipt) {
                return;
            }
            pending.frame_requested = Some(receipt);
            let token = pending.token;
            let popup = pending.popup.clone();
            let owner = cx.entity().downgrade();
            cx.defer(move |cx| {
                let current = owner
                    .read_with(cx, |owner, cx| {
                        owner
                            .popup
                            .pending_material
                            .as_ref()
                            .is_some_and(|pending| {
                                pending.token == token
                                    && pending.frame_requested == Some(receipt)
                                    && owner.popup_material_is_current(pending)
                                    && owner.popup.material.receipt(cx) == Some(receipt)
                            })
                    })
                    .unwrap_or(false);
                if !current {
                    return;
                }
                if popup
                    .window
                    .update(cx, |_, window, cx| {
                        // Hidden Windows HWNDs cannot rely on an ordinary frame callback.
                        window.refresh();
                        window.draw(cx).clear(cx);
                    })
                    .is_err()
                {
                    let _ =
                        owner.update(cx, |owner, _| owner.retire_popup_material("window_closed"));
                    return;
                }
                // Requalify on the next application turn, outside the window lease.
                cx.spawn(async move |cx| {
                    let _ = owner.update(cx, |owner, cx| {
                        owner.finish_popup_material_frame(token, receipt, cx);
                    });
                })
                .detach();
            });
            return;
        }
        let id = uuid::Uuid::new_v4();
        pending.reveal_command = Some(id);
        self.popup.pending_visibility.insert(
            id,
            PendingPopupVisibility {
                window: pending.popup.window,
                native_window_handle: native,
                visible: true,
                session_id: pending.popup.session_id,
                close_reason: None,
                started_at: Instant::now(),
            },
        );
        if self
            .platform
            .send_command_with_id(
                PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                    window_handle: native,
                    visible: true,
                }),
                id,
            )
            .is_err()
        {
            self.retire_popup_material("reveal_enqueue");
        }
    }

    fn finish_popup_material_frame(
        &mut self,
        token: uuid::Uuid,
        receipt: AppliedWindowMaterial,
        cx: &mut Context<Self>,
    ) {
        if let Some(pending) = self.popup.pending_material.as_mut()
            && pending.token == token
            && pending.frame_requested == Some(receipt)
            && self.popup.material.receipt(cx) == Some(receipt)
        {
            if !cx.windows().contains(&pending.popup.window) {
                self.retire_popup_material("window_closed");
                return;
            }
            pending.frame_requested = None;
            pending.frame_ready = Some(receipt);
            self.drive_popup_material(cx);
        }
    }

    pub(super) fn complete_popup_material_reveal(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.popup.pending_material.take() else {
            return;
        };
        info!(event="clipboard.popup.material_ready", correlation_id=%pending.token,
            session_id=%pending.popup.session_id, elapsed_ms=pending.started.elapsed().as_millis() as u64);
        let material = std::mem::take(&mut self.popup.material);
        let platform = self.platform.clone();
        let owner = cx.entity().downgrade();
        let popup = pending.popup;
        let fallback = pending.topmost.flatten();
        cx.defer(move |cx| {
            let current = owner
                .read_with(cx, |owner, _| {
                    owner.popup.pending_material.is_none()
                        && owner.popup.lifecycle.current().is_some_and(|current| {
                            current.window == popup.window && current.session_id == popup.session_id
                        })
                })
                .unwrap_or(false);
            if !current {
                return;
            }
            crate::ui::material::runtime::track_prepared(popup.window, platform, material, cx);
            if let Some(reason) = fallback
                && crate::ui::material::runtime::receipt(popup.window, cx).is_some()
            {
                let visible = owner
                    .read_with(cx, |owner, _| {
                        owner.popup.size_tracking_session == Some(popup.session_id)
                    })
                    .unwrap_or(false);
                if visible {
                    let _ = popup.window.update(cx, |_, window, cx| {
                        let _ = popup.panel.update(cx, |panel, cx| {
                            panel.handle_topmost_failure(reason, window, cx)
                        });
                    });
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::material::Preparation;
    use gpui_kit::{TestAppContext, VisualTestContext, Window, WindowVisibility};
    use nxtools_protocol::window_material::{
        ApplyWindowMaterialCommand, EffectiveWindowMaterial, MaterialFallbackReason,
        SystemMaterialCapabilitiesChanged, WindowMaterialStyle,
    };
    use nxtools_protocol::{ActivateWindowCommand, Message, PlatformCommandResult};

    struct Fixture {
        app: Entity<ClipboardApplication>,
        endpoint: nxtools_ipc::PlatformServerEndpoint,
        worker: crate::clipboard_worker::ClipboardCaptureWorkerHandle,
        popup: PopupWindow,
        topmost: uuid::Uuid,
        _directory: tempfile::TempDir,
    }

    fn facts(cx: &mut gpui_kit::App, revision: u64) {
        let state = cx.default_global::<UiMaterialCapabilities>();
        state.connected();
        let session_id = state
            .snapshot()
            .map_or_else(uuid::Uuid::new_v4, |f| f.session_id);
        assert!(state.apply(SystemMaterialCapabilitiesChanged {
            correlation_id: uuid::Uuid::new_v4(),
            session_id,
            revision,
            supported: true,
            transparency_enabled: true,
            high_contrast: false,
            energy_saver: false,
            remote_session: false,
            native_error_code: 0,
            system_backdrop: true,
            rounded_corners: true,
        }));
    }

    fn fixture(cx: &mut TestAppContext) -> Fixture {
        fixture_with_placement(cx, false)
    }

    fn fixture_with_placement(cx: &mut TestAppContext, hidden_first_show: bool) -> Fixture {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        cx.executor().allow_parking();
        let directory = tempfile::tempdir().expect("synthetic primary popup");
        let worker = crate::clipboard_worker::ClipboardCaptureWorkerHandle::spawn(
            directory.path().join("history.db"),
            directory.path().join("assets"),
            ClipboardRetentionPolicy {
                max_items: 100,
                retention_days: 30,
            },
            1000,
        )
        .expect("temporary history worker");
        let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
        endpoint.set_connected_for_contract_test(true);
        let app = cx.update(|cx| {
            let notifications = cx.new(|_| NotificationCenter::default());
            cx.new(|cx| {
                ClipboardApplication::new(
                    worker.clone(),
                    Rc::new(platform),
                    AppConfig::default(),
                    directory.path().join("config.json"),
                    notifications,
                    cx,
                )
            })
        });
        let session_id = uuid::Uuid::new_v4();
        let panel_slot = Rc::new(RefCell::new(None));
        let slot = panel_slot.clone();
        let owner = app.downgrade();
        let history = worker.clone();
        let root = move |window: &mut Window, cx: &mut Context<Root>| {
            if hidden_first_show {
                crate::ui::material::runtime::set_test_window_handle(
                    window.window_handle(),
                    42,
                    cx,
                );
            }
            let panel = cx.new(|cx| {
                ClipboardPanel::new(
                    ClipboardHistoryPage {
                        items: Vec::new(),
                        total: 0,
                        has_more: false,
                    },
                    Vec::new(),
                    HashMap::new(),
                    Vec::new(),
                    ClipboardSettings::default().modifier_actions,
                    ForegroundContext {
                        window_handle: 73,
                        focus_window_handle: 73,
                        process_id: 7,
                        process_name: "fixture".into(),
                        window_title: String::new(),
                    },
                    history,
                    owner,
                    session_id,
                    None,
                    window,
                    cx,
                )
            });
            *slot.borrow_mut() = Some(panel.downgrade());
            if hidden_first_show {
                crate::ui::popup_surface::root(panel, window, cx)
            } else {
                Root::new(panel, window, cx)
            }
        };
        let window = if hidden_first_show {
            let window = cx.update(|cx| {
                cx.open_window(
                    WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(Bounds {
                            origin: point(px(0.0), px(0.0)),
                            size: size(px(300.0), px(440.0)),
                        })),
                        show: false,
                        kind: WindowKind::PopUp,
                        window_background: WindowBackgroundAppearance::Transparent,
                        window_decorations: Some(WindowDecorations::Client),
                        ..Default::default()
                    },
                    |window, cx| cx.new(|cx| root(window, cx)),
                )
                .expect("hidden clipboard window")
            });
            // TestWindow has no HWND and initializes its visibility as Visible.
            cx.simulate_window_visibility_change(window.into(), WindowVisibility::Hidden);
            window
        } else {
            cx.add_window(root)
        };
        let popup = PopupWindow {
            window: window.into(),
            panel: panel_slot.borrow_mut().take().expect("retained panel"),
            native_window_handle: 42,
            session_id,
            summon_requested_at_unix_ms: 1000,
            summon_event_sequence: 1,
        };
        let topmost = app.update(cx, |app, cx| {
            // The production constructor gets its HWND from GPUI; substitute only
            // the initial native placement here, and keep the real receipt/reveal owner.
            let topmost = if hidden_first_show {
                app.platform
                    .send_command(PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
                        window_handle: popup.native_window_handle,
                        x: 0,
                        y: 0,
                        width: 600,
                        height: 880,
                    }))
                    .expect("queue synthetic native bounds");
                app.platform
                    .send_command(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                        window_handle: popup.native_window_handle,
                        enabled: true,
                    }))
                    .expect("queue synthetic native topmost")
            } else {
                uuid::Uuid::new_v4()
            };
            app.popup.lifecycle.stage(popup.clone(), topmost);
            app.begin_popup_material(popup.clone(), cx);
            topmost
        });
        Fixture {
            app,
            endpoint,
            worker,
            popup,
            topmost,
            _directory: directory,
        }
    }

    fn success() -> PlatformCommandResult {
        PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: None,
        }
    }
    fn material_result(command: ApplyWindowMaterialCommand) -> PlatformCommandResult {
        let receipt = AppliedWindowMaterial {
            session_id: command.session_id,
            facts_revision: command.facts_revision,
            window_handle: command.window_handle,
            generation: command.generation,
            style: command.style,
            surface: command.surface,
            dark: command.dark,
            effective: match command.style {
                WindowMaterialStyle::Solid => EffectiveWindowMaterial::Solid,
                _ => EffectiveWindowMaterial::Acrylic,
            },
            fallback_reason: MaterialFallbackReason::None,
            native_error_code: 0,
        };
        PlatformCommandResult {
            data: Some(serde_json::to_string(&receipt).expect("receipt")),
            ..success()
        }
    }
    fn deliver(
        f: &Fixture,
        id: uuid::Uuid,
        result: PlatformCommandResult,
        cx: &mut TestAppContext,
    ) {
        f.app.update(cx, |app, cx| {
            app.handle_platform_notification(
                PlatformNotification::CommandResult {
                    correlation_id: id,
                    result,
                },
                cx,
            )
        });
    }
    fn next_material(f: &Fixture) -> (uuid::Uuid, ApplyWindowMaterialCommand) {
        let envelope = f
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("material command");
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) =
            envelope.message
        else {
            panic!("primary material, not reveal");
        };
        assert_eq!(command.surface, MaterialSurface::Transient);
        (envelope.id, command)
    }
    fn frame(f: &Fixture, cx: &mut TestAppContext) {
        cx.run_until_parked();
        let mut surface = VisualTestContext::from_window(f.popup.window, cx);
        surface.refresh().expect("primary hidden frame");
        surface.update(|window, cx| {
            window.simulate_next_frame(cx);
        });
        surface.run_until_parked();
    }

    fn acknowledge(
        f: &Fixture,
        id: uuid::Uuid,
        result: PlatformCommandResult,
        cx: &mut TestAppContext,
    ) {
        assert!(f.endpoint.command_pending_for_contract_test(id));
        f.endpoint
            .route_command_result_for_contract_test(id, result)
            .expect("route native terminal");
        f.app.update(cx, |app, cx| {
            let notification = app
                .platform
                .notification_receiver()
                .try_recv()
                .expect("routed native terminal");
            assert!(matches!(
                &notification,
                PlatformNotification::CommandResult { correlation_id, .. } if *correlation_id == id
            ));
            app.handle_platform_notification(notification, cx);
        });
    }

    /// Whether the hidden frame still carries the opaque foundation painted until a backdrop is live.
    fn hidden_frame_has_opaque_foundation(f: &Fixture, cx: &mut TestAppContext) -> bool {
        f.popup
            .window
            .update(cx, |_, window, _| {
                assert_eq!(window.visibility(), WindowVisibility::Hidden);
                let scale = window.scale_factor();
                let viewport = window.viewport_size();
                window.painted_quads().iter().any(|quad| {
                    quad.bounds.size == viewport.scale(scale)
                        && quad
                            .background
                            .as_solid()
                            .is_some_and(|tint| tint.to_rgb().a == 1.0)
                })
            })
            .expect("original hidden popup")
    }

    #[gpui_kit::test]
    fn primary_hidden_first_show_draws_and_reveals_without_a_platform_frame(
        cx: &mut TestAppContext,
    ) {
        let f = fixture_with_placement(cx, true);
        cx.update(|cx| {
            crate::theme::set_appearance_preferences(
                nxtools_core::ThemePreference::Dark,
                nxtools_core::MaterialStyle::Acrylic,
                cx,
            );
        });
        let bounds = f
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("bounds");
        assert!(matches!(
            bounds.message,
            Message::PlatformCommand(PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
                window_handle: 42,
                width: 600,
                height: 880,
                ..
            }))
        ));
        acknowledge(&f, bounds.id, success(), cx);
        let topmost = f
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("topmost");
        assert_eq!(topmost.id, f.topmost);
        assert!(matches!(
            topmost.message,
            Message::PlatformCommand(PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                window_handle: 42,
                enabled: true
            }))
        ));
        acknowledge(&f, topmost.id, success(), cx);
        assert!(f.endpoint.try_receive_command_for_contract_test().is_err());

        cx.update(|cx| facts(cx, 1));
        cx.run_until_parked();
        let (id, command) = next_material(&f);
        assert_eq!(command.window_handle, 42);
        assert_eq!(command.style, WindowMaterialStyle::Acrylic);
        assert!(hidden_frame_has_opaque_foundation(&f, cx));
        assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
        acknowledge(&f, id, material_result(command), cx);

        // A hidden HWND does not receive the ordinary frame callback. Only drain
        // application work; do not deliver a frame or advance the timeout clock.
        cx.run_until_parked();
        assert!(f.app.read_with(cx, |app, cx| {
            app.current_popup_material_receipt(cx)
                .is_some_and(|receipt| receipt.matches(&command))
        }));
        let show = f.endpoint.try_receive_command_for_contract_test().expect(
            "qualified hidden popup must enqueue its original reveal without a platform frame",
        );
        assert!(matches!(
            show.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle: 42,
                    visible: true
                }
            ))
        ));
        // The transient popup's lists sit directly on the acrylic: no opaque foundation remains.
        assert!(!hidden_frame_has_opaque_foundation(&f, cx));
        assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
        acknowledge(&f, show.id, success(), cx);
        let activation = f
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("original activation");
        assert!(matches!(
            activation.message,
            Message::PlatformCommand(PlatformCommand::ActivateWindow(ActivateWindowCommand {
                window_handle: 42
            }))
        ));
        acknowledge(&f, activation.id, success(), cx);
        f.worker.shutdown().expect("temporary worker stopped");
    }

    #[gpui_kit::test]
    fn primary_topmost_alone_waits_for_material_then_frame_and_original_activation(
        cx: &mut TestAppContext,
    ) {
        let f = fixture(cx);
        deliver(&f, f.topmost, success(), cx);
        assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
        cx.update(|cx| facts(cx, 1));
        cx.run_until_parked();
        let (id, command) = next_material(&f);
        deliver(&f, uuid::Uuid::new_v4(), material_result(command), cx);
        assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
        f.endpoint
            .route_command_result_for_contract_test(id, material_result(command))
            .expect("material terminal");
        deliver(&f, id, material_result(command), cx);
        frame(&f, cx);
        let show = f
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("original reveal");
        assert!(matches!(
            show.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle: 42,
                    visible: true
                }
            ))
        ));
        deliver(&f, show.id, success(), cx);
        let activation = f
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("original activation");
        assert!(matches!(
            activation.message,
            Message::PlatformCommand(PlatformCommand::ActivateWindow(_))
        ));
        let focus_before = cx.update(|cx| {
            f.popup
                .window
                .update(cx, |_, window, cx| window.focused(cx))
                .expect("original focus owner")
        });
        assert!(
            focus_before.is_some(),
            "original search focus remains mounted"
        );
        deliver(&f, activation.id, success(), cx);
        cx.run_until_parked();
        let focus_after = cx.update(|cx| {
            f.popup
                .window
                .update(cx, |_, window, cx| window.focused(cx))
                .expect("retained focus owner")
        });
        assert_eq!(focus_after, focus_before);
        f.app.update(cx, |app, cx| {
            let current = app.popup.lifecycle.current().expect("same primary");
            assert_eq!(current.window, f.popup.window);
            assert_eq!(current.panel, f.popup.panel);
            assert_eq!(current.session_id, f.popup.session_id);
            assert_eq!(app.popup.size_tracking_session, Some(f.popup.session_id));
            assert!(
                app.popup.material.receipt(cx).is_none(),
                "runtime owns the confirmed record"
            );
            assert!(
                app.current_popup_material_receipt(cx).is_some(),
                "original visibility owner can read runtime material"
            );
        });
        f.worker.shutdown().expect("temporary worker stopped");
    }

    #[gpui_kit::test]
    fn primary_superseded_facts_and_deferred_frame_cannot_reveal_old_material(
        cx: &mut TestAppContext,
    ) {
        let f = fixture(cx);
        cx.update(|cx| facts(cx, 1));
        cx.run_until_parked();
        let (old_id, old) = next_material(&f);
        deliver(&f, f.topmost, success(), cx);
        cx.update(|cx| facts(cx, 2));
        cx.run_until_parked();
        f.endpoint
            .route_command_result_for_contract_test(old_id, material_result(old))
            .expect("old native terminal");
        deliver(&f, old_id, material_result(old), cx);
        let (id, current) = next_material(&f);
        assert_eq!(current.facts_revision, 2);
        assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
        f.endpoint
            .route_command_result_for_contract_test(id, material_result(current))
            .expect("current terminal");
        deliver(&f, id, material_result(current), cx);
        cx.update(|cx| {
            let state = cx.default_global::<crate::theme::UiThemeState>();
            state.material_style = nxtools_core::MaterialStyle::Acrylic;
            state.preference = nxtools_core::ThemePreference::Light;
        });
        frame(&f, cx);
        let (_, latest) = next_material(&f);
        assert_eq!(latest.style, WindowMaterialStyle::Acrylic);
        assert!(!latest.dark);
        assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
        f.worker.shutdown().expect("temporary worker stopped");
    }

    #[gpui_kit::test]
    fn primary_retirement_removes_queued_material_and_late_topmost_cannot_revive(
        cx: &mut TestAppContext,
    ) {
        let f = fixture(cx);
        f.app
            .update(cx, |app, _| app.retire_popup_material("fixture_reset"));
        while f.endpoint.try_receive_command_for_contract_test().is_ok() {}
        cx.update(|cx| facts(cx, 1));
        let mut blocker = Preparation::default();
        f.app.update(cx, |app, cx| {
            assert!(
                !blocker
                    .ensure(&app.platform, 99, MaterialSurface::Transient, cx)
                    .expect("blocking material")
            );
        });
        let active = f
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("active material");
        for boundary in 0..3 {
            let topmost = uuid::Uuid::new_v4();
            f.app.update(cx, |app, cx| {
                app.popup.lifecycle.replace_pending_topmost(topmost);
                app.begin_popup_material(f.popup.clone(), cx);
                match boundary {
                    0 => {
                        app.hide_popup(
                            f.popup.window,
                            42,
                            f.popup.session_id,
                            PopupCloseReason::Escape,
                        )
                        .expect("dismiss primary");
                    }
                    1 => {
                        let token = app
                            .popup
                            .pending_material
                            .as_ref()
                            .expect("pending preparation")
                            .token;
                        app.expire_popup_material(token, cx);
                    }
                    _ => app.handle_platform_notification(
                        PlatformNotification::Disconnected {
                            generation: 1,
                            pending_command_ids: Vec::new(),
                        },
                        cx,
                    ),
                }
            });
            deliver(&f, topmost, success(), cx);
            frame(&f, cx);
            while let Ok(envelope) = f.endpoint.try_receive_command_for_contract_test() {
                assert!(!matches!(
                    envelope.message,
                    Message::PlatformCommand(
                        PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                            visible: true,
                            ..
                        }) | PlatformCommand::ApplyWindowMaterial(_)
                            | PlatformCommand::ActivateWindow(_)
                    )
                ));
            }
            f.app.update(cx, |app, _| {
                assert!(app.popup.pending_material.is_none());
                assert_eq!(
                    app.popup
                        .lifecycle
                        .current()
                        .expect("retained primary")
                        .window,
                    f.popup.window
                );
            });
        }
        f.endpoint
            .route_command_result_for_contract_test(active.id, success())
            .expect("blocking terminal");
        assert!(
            f.endpoint.try_receive_command_for_contract_test().is_err(),
            "retired material never enters native lane"
        );
        f.worker.shutdown().expect("temporary worker stopped");
    }

    #[gpui_kit::test]
    fn primary_invalid_material_receipt_stays_hidden_and_late_success_is_retired(
        cx: &mut TestAppContext,
    ) {
        let f = fixture(cx);
        cx.update(|cx| facts(cx, 1));
        cx.run_until_parked();
        let (id, command) = next_material(&f);
        deliver(&f, f.topmost, success(), cx);
        let wrong = material_result(ApplyWindowMaterialCommand {
            window_handle: 43,
            ..command
        });
        f.endpoint
            .route_command_result_for_contract_test(id, wrong.clone())
            .expect("invalid terminal");
        deliver(&f, id, wrong, cx);
        deliver(&f, id, material_result(command), cx);
        frame(&f, cx);
        while let Ok(envelope) = f.endpoint.try_receive_command_for_contract_test() {
            assert!(matches!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand { visible: false, .. }
                ))
            ));
        }
        f.app
            .update(cx, |app, _| assert!(app.popup.pending_material.is_none()));
        f.worker.shutdown().expect("temporary worker stopped");
    }

    #[gpui_kit::test]
    fn primary_topmost_failure_fallback_requires_qualified_material(cx: &mut TestAppContext) {
        let f = fixture(cx);
        deliver(
            &f,
            f.topmost,
            PlatformCommandResult {
                success: false,
                error: Some("synthetic topmost failure".into()),
                ..success()
            },
            cx,
        );
        frame(&f, cx);
        assert!(f.endpoint.try_receive_command_for_contract_test().is_err());
        cx.update(|cx| facts(cx, 1));
        cx.run_until_parked();
        let (id, command) = next_material(&f);
        f.endpoint
            .route_command_result_for_contract_test(id, material_result(command))
            .expect("material terminal");
        deliver(&f, id, material_result(command), cx);
        frame(&f, cx);
        let show = f
            .endpoint
            .try_receive_command_for_contract_test()
            .expect("qualified fallback reveal");
        assert!(matches!(
            show.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand { visible: true, .. }
            ))
        ));
        f.worker.shutdown().expect("temporary worker stopped");
    }

    #[gpui_kit::test]
    fn primary_topmost_timeout_cancels_even_with_qualified_material(cx: &mut TestAppContext) {
        let f = fixture(cx);
        cx.update(|cx| facts(cx, 1));
        cx.run_until_parked();
        let (id, command) = next_material(&f);
        f.endpoint
            .route_command_result_for_contract_test(id, material_result(command))
            .expect("material terminal");
        deliver(&f, id, material_result(command), cx);
        deliver(
            &f,
            f.topmost,
            PlatformCommandResult {
                success: false,
                error: Some("platform command timed out".into()),
                ..success()
            },
            cx,
        );
        deliver(&f, f.topmost, success(), cx);
        frame(&f, cx);
        while let Ok(envelope) = f.endpoint.try_receive_command_for_contract_test() {
            assert!(matches!(
                envelope.message,
                Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand { visible: false, .. }
                ))
            ));
        }
        f.app
            .update(cx, |app, _| assert!(app.popup.pending_material.is_none()));
        f.worker.shutdown().expect("temporary worker stopped");
    }
}
