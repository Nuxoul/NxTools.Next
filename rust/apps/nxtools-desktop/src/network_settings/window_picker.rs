use super::window_picker_overlay::WindowPickerOverlay;
use super::*;
use nxtools_protocol::PlatformEvent;
use nxtools_protocol::process_window_pick::{
    ProcessWindowPickChanged, ProcessWindowPickCommand, WindowPickState,
};

pub(super) struct PendingWindowPick {
    pub id: uuid::Uuid,
    pub view: WeakEntity<NetworkSettingsView>,
    pub source: AnyWindowHandle,
    pub connection: u64,
    pub overlay: WindowPickerOverlay,
    pub started: Instant,
    pub frame_requested: bool,
    pub show_command: Option<uuid::Uuid>,
}

impl NetworkSettingsView {
    pub(super) fn toggle_process_window_pick(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.saving {
            return;
        }
        let application = self.application.clone();
        if let Some(id) = self.process_window_pick.take() {
            cx.defer(move |cx| {
                application.update(cx, |app, cx| {
                    app.cancel_process_window_pick(id, "cancelled", cx)
                })
            });
        } else {
            let id = uuid::Uuid::new_v4();
            self.process_window_pick = Some(id);
            self.error = None;
            let view = cx.entity().downgrade();
            let source = window.window_handle();
            let source_hwnd = crate::ui::native_window_handle(window).unwrap_or_default();
            cx.defer(move |cx| {
                application.update(cx, |app, cx| {
                    app.request_process_window_pick(id, view, source, source_hwnd, cx)
                })
            });
        }
        cx.notify();
    }

    pub(super) fn observe_picker_page(&mut self, page: usize, cx: &mut Context<Self>) {
        if page == 2 {
            return;
        }
        if let Some(id) = self.process_window_pick.take() {
            let application = self.application.clone();
            cx.defer(move |cx| {
                application.update(cx, |app, cx| {
                    app.cancel_process_window_pick(id, "page_changed", cx)
                })
            });
        }
    }
}

impl NetworkSettingsApplication {
    fn request_process_window_pick(
        &mut self,
        id: uuid::Uuid,
        view: WeakEntity<NetworkSettingsView>,
        source: AnyWindowHandle,
        source_hwnd: i64,
        cx: &mut Context<Self>,
    ) {
        let admitted = self.window == Some(source)
            && view
                .upgrade()
                .is_some_and(|view| view.read(cx).process_window_pick == Some(id));
        if !admitted {
            return;
        }
        let overlay = match WindowPickerOverlay::open(cx) {
            Ok(overlay) => overlay,
            Err(_) => {
                self.release_process_picker(
                    id,
                    view,
                    source,
                    None,
                    Some("无法创建窗口选取高亮，请重试。".into()),
                    cx,
                );
                return;
            }
        };
        self.begin_process_window_pick(id, view, source, source_hwnd, overlay, cx);
    }

    pub(super) fn begin_process_window_pick(
        &mut self,
        id: uuid::Uuid,
        view: WeakEntity<NetworkSettingsView>,
        source: AnyWindowHandle,
        source_hwnd: i64,
        overlay: WindowPickerOverlay,
        cx: &mut Context<Self>,
    ) {
        let connection = self
            .platform
            .as_ref()
            .and_then(|platform| platform.connection_generation());
        if self.pending_process_window_pick.is_some()
            || self.window != Some(source)
            || view
                .upgrade()
                .is_none_or(|view| view.read(cx).process_window_pick != Some(id))
            || source_hwnd == 0
            || overlay.native_window_handle == 0
            || connection.is_none()
        {
            let handle = overlay.window;
            cx.defer(move |cx| {
                let _ = handle.update(cx, |_, window, _| window.remove_window());
            });
            self.release_process_picker(
                id,
                view,
                source,
                None,
                Some("无法开始窗口选取，请确认 Windows 平台组件已连接后重试。".into()),
                cx,
            );
            return;
        }
        let command = ProcessWindowPickCommand::Start {
            session_id: id,
            source_window_handle: source_hwnd,
            overlay_window_handle: overlay.native_window_handle,
        };
        self.pending_process_window_pick = Some(PendingWindowPick {
            id,
            view,
            source,
            connection: connection.expect("connected picker admission"),
            overlay,
            started: Instant::now(),
            frame_requested: false,
            show_command: None,
        });
        info!(event = "global.blacklist.window_pick.accepted", correlation_id = %id);
        let sent = self.platform.as_ref().is_some_and(|platform| {
            platform
                .send_command_with_id_correlated(
                    PlatformCommand::ProcessWindowPick(command),
                    id,
                    id,
                )
                .is_ok()
        });
        if !sent {
            self.finish_process_window_pick(
                id,
                None,
                Some("无法开始窗口选取，请稍后重试。".into()),
                "dispatch_failed",
                cx,
            );
            return;
        }
        cx.spawn(async move |owner, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(125))
                .await;
            let _ = owner.update(cx, |app, cx| {
                app.cancel_process_window_pick(id, "timeout", cx)
            });
        })
        .detach();
    }

    pub(super) fn cancel_process_window_pick(
        &mut self,
        id: uuid::Uuid,
        reason: &'static str,
        cx: &mut Context<Self>,
    ) {
        let error = match reason {
            "timeout" => Some("窗口选取已超时，请重新选取。".to_owned()),
            "native_failed" | "show_dispatch_failed" | "invalid_viewport" => {
                Some("窗口选取失败，请检查平台组件或手动填写进程名。".to_owned())
            }
            _ => None,
        };
        let Some(pending) = self
            .pending_process_window_pick
            .as_ref()
            .filter(|pending| pending.id == id)
        else {
            return;
        };
        if let Some(platform) = self
            .platform
            .as_ref()
            .filter(|platform| platform.connection_generation() == Some(pending.connection))
        {
            let _ = platform.send_command_correlated(
                PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::Cancel {
                    session_id: id,
                }),
                id,
            );
        }
        self.finish_process_window_pick(id, None, error, reason, cx);
    }

    pub(super) fn handle_process_window_pick(
        &mut self,
        notification: &PlatformNotification,
        cx: &mut Context<Self>,
    ) -> bool {
        match notification {
            PlatformNotification::CommandResult {
                correlation_id,
                result,
            } => {
                let Some(pending) = self.pending_process_window_pick.as_ref() else {
                    return false;
                };
                if *correlation_id != pending.id && pending.show_command != Some(*correlation_id) {
                    return false;
                }
                if !result.success {
                    let id = pending.id;
                    warn!(event = "global.blacklist.window_pick.native_failed", correlation_id = %id, native_error_code = ?result.native_error_code);
                    self.cancel_process_window_pick(id, "native_failed", cx);
                }
                true
            }
            PlatformNotification::Event(PlatformEvent::ProcessWindowPickChanged(event)) => {
                self.process_window_pick_changed(event.clone(), cx);
                true
            }
            PlatformNotification::Disconnected { generation, .. } => {
                if let Some(id) = self
                    .pending_process_window_pick
                    .as_ref()
                    .filter(|pending| pending.connection == *generation)
                    .map(|pending| pending.id)
                {
                    self.finish_process_window_pick(
                        id,
                        None,
                        Some("Windows 平台组件已断开，请重新选取。".into()),
                        "disconnected",
                        cx,
                    );
                }
                false
            }
            _ => false,
        }
    }

    fn process_window_pick_changed(
        &mut self,
        event: ProcessWindowPickChanged,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self
            .pending_process_window_pick
            .as_ref()
            .filter(|pending| pending.id == event.session_id)
        else {
            return;
        };
        let current = self.window == Some(pending.source)
            && self
                .platform
                .as_ref()
                .and_then(|platform| platform.connection_generation())
                == Some(pending.connection)
            && pending
                .view
                .upgrade()
                .is_some_and(|view| view.read(cx).process_window_pick == Some(event.session_id));
        if !current {
            self.cancel_process_window_pick(event.session_id, "owner_retired", cx);
            return;
        }
        match event.state {
            WindowPickState::Hover => {
                let Some(viewport) = event.viewport.filter(|viewport| viewport.is_valid()) else {
                    self.cancel_process_window_pick(event.session_id, "invalid_viewport", cx);
                    return;
                };
                let pending = self
                    .pending_process_window_pick
                    .as_mut()
                    .expect("current picker");
                pending.overlay.view.update(cx, |overlay, cx| {
                    overlay.viewport = Some(viewport);
                    overlay.candidate = event.candidate.filter(|candidate| {
                        candidate.window_handle != 0
                            && candidate.process_id != 0
                            && candidate.width > 0
                            && candidate.height > 0
                            && ProcessBlacklistRule::new(&candidate.process_name, true, false)
                                .is_ok()
                    });
                    cx.notify();
                });
                if pending.frame_requested {
                    return;
                }
                pending.frame_requested = true;
                let id = pending.id;
                let owner = cx.entity().downgrade();
                let _ = pending.overlay.window.update(cx, |_, window, _| {
                    window.on_next_frame(move |_, cx| {
                        cx.defer(move |cx| {
                            let _ =
                                owner.update(cx, |app, cx| app.show_process_picker_overlay(id, cx));
                        });
                    });
                    window.refresh();
                });
            }
            WindowPickState::Selected => {
                let selected = event
                    .candidate
                    .filter(|candidate| {
                        candidate.window_handle != 0
                            && candidate.process_id != 0
                            && candidate.width > 0
                            && candidate.height > 0
                    })
                    .and_then(|candidate| {
                        ProcessBlacklistRule::new(&candidate.process_name, true, false)
                            .ok()
                            .map(|rule| format!("{}.exe", rule.process))
                    });
                let error = selected
                    .is_none()
                    .then(|| "无法读取所选窗口的进程名，请重新选取或手动填写。".to_owned());
                self.finish_process_window_pick(event.session_id, selected, error, "selected", cx);
            }
            WindowPickState::Cancelled => {
                self.finish_process_window_pick(event.session_id, None, None, "cancelled", cx)
            }
            WindowPickState::Failed => {
                warn!(event = "global.blacklist.window_pick.native_failed", correlation_id = %event.session_id, native_error_code = ?event.native_error_code);
                self.finish_process_window_pick(
                    event.session_id,
                    None,
                    Some("窗口选取失败，请检查目标窗口权限或手动填写进程名。".into()),
                    "native_failed",
                    cx,
                );
            }
        }
    }

    fn show_process_picker_overlay(&mut self, id: uuid::Uuid, cx: &mut Context<Self>) {
        let Some(pending) = self
            .pending_process_window_pick
            .as_mut()
            .filter(|pending| pending.id == id && pending.show_command.is_none())
        else {
            return;
        };
        if self.window != Some(pending.source)
            || pending
                .view
                .upgrade()
                .is_none_or(|view| view.read(cx).process_window_pick != Some(id))
            || self
                .platform
                .as_ref()
                .and_then(|platform| platform.connection_generation())
                != Some(pending.connection)
            || !cx.windows().contains(&pending.overlay.window)
        {
            self.cancel_process_window_pick(id, "owner_retired", cx);
            return;
        }
        let overlay = pending.overlay.window;
        let hwnd = pending.overlay.native_window_handle;
        if !crate::ui::material::runtime::receipt(overlay, cx).is_some_and(|receipt| {
            receipt.window_handle == hwnd
                && receipt.surface
                    == nxtools_protocol::window_material::MaterialSurface::Transparent
        }) {
            let platform = self.platform.clone().expect("current picker connection");
            let connection = pending.connection;
            let owner = cx.entity().downgrade();
            // Material admission may lease the overlay, so leave this owner callback first.
            cx.defer(move |cx| {
                let Some(owner) = owner.upgrade() else {
                    return;
                };
                let current = {
                    let app = owner.read(cx);
                    app.pending_process_window_pick
                        .as_ref()
                        .is_some_and(|pending| {
                            pending.id == id
                                && pending.overlay.window == overlay
                                && pending.overlay.native_window_handle == hwnd
                                && pending.connection == connection
                                && app.window == Some(pending.source)
                                && platform.connection_generation() == Some(connection)
                                && pending.view.upgrade().is_some_and(|view| {
                                    view.read(cx).process_window_pick == Some(id)
                                })
                        })
                };
                if !current
                    || !cx.windows().contains(&overlay)
                    || crate::ui::material::runtime::is_waiting(overlay, cx)
                {
                    return;
                }
                crate::ui::material::runtime::when_ready(
                    overlay,
                    platform,
                    nxtools_protocol::window_material::MaterialSurface::Transparent,
                    std::time::Duration::from_secs(8),
                    move |result, cx| {
                        owner.update(cx, |app, cx| {
                            if !app
                                .pending_process_window_pick
                                .as_ref()
                                .is_some_and(|pending| {
                                    pending.id == id
                                        && pending.overlay.window == overlay
                                        && pending.overlay.native_window_handle == hwnd
                                        && pending.connection == connection
                                })
                            {
                                return;
                            }
                            match result {
                                Ok(()) => app.show_process_picker_overlay(id, cx),
                                Err(_) => app.cancel_process_window_pick(id, "native_failed", cx),
                            }
                        });
                    },
                    cx,
                );
            });
            return;
        }
        let command_id = uuid::Uuid::new_v4();
        pending.show_command = Some(command_id);
        let sent = self
            .platform
            .as_ref()
            .filter(|platform| platform.connection_generation() == Some(pending.connection))
            .is_some_and(|platform| {
                platform
                    .send_command_with_id_correlated(
                        PlatformCommand::ProcessWindowPick(ProcessWindowPickCommand::ShowOverlay {
                            session_id: id,
                        }),
                        command_id,
                        id,
                    )
                    .is_ok()
            });
        if !sent {
            self.cancel_process_window_pick(id, "show_dispatch_failed", cx);
        }
    }

    fn finish_process_window_pick(
        &mut self,
        id: uuid::Uuid,
        selected: Option<String>,
        error: Option<String>,
        outcome: &'static str,
        cx: &mut Context<Self>,
    ) {
        if self
            .pending_process_window_pick
            .as_ref()
            .is_none_or(|pending| pending.id != id)
        {
            return;
        }
        let pending = self
            .pending_process_window_pick
            .take()
            .expect("matched picker");
        info!(event = "global.blacklist.window_pick.completed", correlation_id = %id, outcome, selected = selected.is_some(), elapsed_ms = pending.started.elapsed().as_millis() as u64);
        let overlay = pending.overlay.window;
        cx.defer(move |cx| {
            let _ = overlay.update(cx, |_, window, _| window.remove_window());
        });
        self.release_process_picker(id, pending.view, pending.source, selected, error, cx);
    }
    fn release_process_picker(
        &self,
        id: uuid::Uuid,
        view: WeakEntity<NetworkSettingsView>,
        source: AnyWindowHandle,
        selected: Option<String>,
        error: Option<String>,
        cx: &mut Context<Self>,
    ) {
        let owner = cx.entity().downgrade();
        let connection = self
            .platform
            .as_ref()
            .and_then(|platform| platform.connection_generation());
        cx.defer(move |cx| {
            let current = owner.upgrade().is_some_and(|owner| {
                let owner = owner.read(cx);
                owner.window == Some(source)
                    && owner
                        .platform
                        .as_ref()
                        .and_then(|platform| platform.connection_generation())
                        == connection
            });
            let _ = source.update(cx, |_, window, cx| {
                let _ = view.update(cx, |view, cx| {
                    if view.process_window_pick != Some(id) {
                        return;
                    }
                    view.process_window_pick = None;
                    if current && !view.saving {
                        view.error = error;
                        if let Some(process) = selected {
                            view.process_input.update(cx, |input, cx| {
                                input.set_value(process, window, cx);
                                input.focus(window, cx);
                            });
                            window.activate_window();
                        }
                    }
                    cx.notify();
                });
            });
        });
    }
}

#[cfg(test)]
#[path = "window_picker_material_tests.rs"]
mod material_tests;
