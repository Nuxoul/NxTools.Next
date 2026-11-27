use super::*;
use crate::ui::material::runtime as material_runtime;
use nxtools_protocol::window_material::MaterialSurface;
use nxtools_protocol::{CursorMonitorSnapshot, PlatformCommandResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Monitor,
    Hide,
    NoActivate,
    Bounds,
    Topmost,
    MaterialStage,
    Stage,
    Frame,
    HideAfterFrame,
    TargetMonitor,
    FinalBounds,
    MaterialReveal,
    Reveal,
    ExpandedBounds,
}

pub(super) struct PendingLinkPlacement {
    operation: uuid::Uuid,
    command: uuid::Uuid,
    generation: u64,
    pause_generation: u64,
    started: Instant,
    connection: Option<u64>,
    phase: Phase,
    links: Vec<crate::tools::link_panel::DetectedLinkRow>,
    monitor: Option<CursorMonitorSnapshot>,
    window: Option<AnyWindowHandle>,
    native: i64,
    bounds: (i32, i32, i32, i32),
}

impl ClipboardApplication {
    pub(super) fn request_link_panel(
        &mut self,
        links: Vec<crate::tools::link_panel::DetectedLinkRow>,
        cx: &mut Context<Self>,
    ) {
        self.retire_link_placement("superseded", cx);
        if !self.config.clipboard.enabled || !self.config.clipboard.link_detector_enabled {
            return;
        }
        let operation = uuid::Uuid::new_v4();
        self.pending_link_placement = Some(PendingLinkPlacement {
            operation,
            command: uuid::Uuid::nil(),
            generation: self.link_panel.generation,
            pause_generation: self.capture_pause.generation,
            started: Instant::now(),
            connection: self.platform.connection_generation(),
            phase: Phase::Monitor,
            links,
            monitor: None,
            window: None,
            native: 0,
            bounds: (0, 0, 0, 0),
        });
        info!(event = "tools.link_panel.presentation_accepted", correlation_id = %operation);
        self.send_link_placement_command(PlatformCommand::QueryCursorMonitor, cx);
        self.arm_link_placement_timeout(operation, cx);
    }

    pub(crate) fn request_link_panel_expansion(
        &mut self,
        source: AnyWindowHandle,
        total_rows: usize,
        cx: &mut Context<Self>,
    ) {
        if self.link_panel.window != Some(source)
            || self.pending_link_placement.is_some()
            || self.link_panel.native_window_handle == 0
            || !self.config.clipboard.enabled
            || !self.config.clipboard.link_detector_enabled
            || self.capture_pause.blocks(nxtools_ipc::unix_time_ms())
        {
            return;
        }
        let placement = crate::ui::monitor_placement::cursor_monitor_placement(
            self.link_panel.monitor,
            0,
            96,
            cx,
        );
        let bounds = link_panel_bounds(
            self.config.clipboard.link_panel_position,
            size(
                px(crate::tools::link_panel::LINK_PANEL_WIDTH),
                px(crate::tools::link_panel::link_panel_height(
                    total_rows, true,
                )),
            ),
            placement.work_area,
        )
        .to_device_pixels(placement.scale);
        let operation = uuid::Uuid::new_v4();
        let native = self.link_panel.native_window_handle;
        self.pending_link_placement = Some(PendingLinkPlacement {
            operation,
            command: uuid::Uuid::nil(),
            generation: self.link_panel.generation,
            pause_generation: self.capture_pause.generation,
            started: Instant::now(),
            connection: self.platform.connection_generation(),
            phase: Phase::ExpandedBounds,
            links: Vec::new(),
            monitor: self.link_panel.monitor,
            window: Some(source),
            native,
            bounds: (
                bounds.origin.x.into(),
                bounds.origin.y.into(),
                bounds.size.width.into(),
                bounds.size.height.into(),
            ),
        });
        info!(event = "tools.link_panel.presentation_accepted", correlation_id = %operation, reason = "expand");
        self.send_link_placement_command(
            PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
                window_handle: native,
                x: bounds.origin.x.into(),
                y: bounds.origin.y.into(),
                width: bounds.size.width.into(),
                height: bounds.size.height.into(),
            }),
            cx,
        );
        self.arm_link_placement_timeout(operation, cx);
    }

    fn arm_link_placement_timeout(&self, operation: uuid::Uuid, cx: &mut Context<Self>) {
        cx.spawn(async move |owner, cx| {
            cx.background_executor().timer(Duration::from_secs(3)).await;
            let _ = owner.update(cx, |owner, cx| {
                if owner
                    .pending_link_placement
                    .as_ref()
                    .is_some_and(|pending| pending.operation == operation)
                {
                    owner.retire_link_placement("timeout", cx);
                }
            });
        })
        .detach();
    }

    fn link_placement_is_current(&self, pending: &PendingLinkPlacement) -> bool {
        self.config.clipboard.enabled
            && self.config.clipboard.link_detector_enabled
            && !self.capture_pause.blocks(nxtools_ipc::unix_time_ms())
            && pending.pause_generation == self.capture_pause.generation
            && pending.generation == self.link_panel.generation
            && pending.started.elapsed() < Duration::from_secs(3)
            && pending.connection == self.platform.connection_generation()
            && (pending.native == 0 || pending.native == self.link_panel.native_window_handle)
            && pending
                .window
                .is_none_or(|window| self.link_panel.window == Some(window))
    }

    fn send_link_placement_command(&mut self, command: PlatformCommand, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_link_placement.as_mut() else {
            return;
        };
        let id = uuid::Uuid::new_v4();
        pending.command = id;
        let operation = pending.operation;
        info!(event = "tools.link_panel.presentation_stage", correlation_id = %operation, command_id = %id, phase = ?pending.phase);
        if self
            .platform
            .send_command_with_id_correlated(command, id, operation)
            .is_err()
        {
            self.retire_link_placement("dispatch_failed", cx);
        }
    }

    pub(super) fn finish_link_placement(
        &mut self,
        id: uuid::Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        if self
            .pending_link_placement
            .as_ref()
            .is_none_or(|pending| pending.command != id)
        {
            return false;
        }
        let mut pending = self
            .pending_link_placement
            .take()
            .expect("matching link placement");
        if !result.success || !self.link_placement_is_current(&pending) {
            warn!(event = "tools.link_panel.presentation_failed", correlation_id = %pending.operation,
                native_error_code = result.native_error_code, elapsed_ms = pending.started.elapsed().as_millis() as u64);
            self.pending_link_placement = Some(pending);
            self.retire_link_placement("native_failed_or_retired", cx);
            return true;
        }
        match pending.phase {
            Phase::Monitor => {
                let monitor = result
                    .data
                    .as_deref()
                    .and_then(|data| serde_json::from_str::<CursorMonitorSnapshot>(data).ok())
                    .filter(|monitor| {
                        monitor.work_width > 0 && monitor.work_height > 0 && monitor.dpi >= 96
                    });
                let Some(monitor) = monitor else {
                    self.pending_link_placement = Some(pending);
                    self.retire_link_placement("monitor_unavailable", cx);
                    return true;
                };
                pending.monitor = Some(monitor);
                if let Some(window) = self.link_panel.window
                    && self.link_panel.native_window_handle != 0
                    && window.update(cx, |_, _, _| ()).is_ok()
                {
                    pending.phase = Phase::Hide;
                    pending.window = Some(window);
                    pending.native = self.link_panel.native_window_handle;
                    let native = pending.native;
                    self.pending_link_placement = Some(pending);
                    self.send_link_placement_command(
                        PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                            window_handle: native,
                            visible: false,
                        }),
                        cx,
                    );
                } else {
                    self.prepare_link_placement(pending, cx);
                }
            }
            Phase::Hide => self.prepare_link_placement(pending, cx),
            Phase::NoActivate => {
                pending.phase = Phase::Bounds;
                let (_, _, width, height) = pending.bounds;
                let native = pending.native;
                self.pending_link_placement = Some(pending);
                self.send_link_placement_command(
                    PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
                        window_handle: native,
                        x: PREVIEW_STAGING_COORDINATE,
                        y: PREVIEW_STAGING_COORDINATE,
                        width,
                        height,
                    }),
                    cx,
                );
            }
            Phase::Bounds => {
                pending.phase = Phase::Topmost;
                let native = pending.native;
                self.pending_link_placement = Some(pending);
                self.send_link_placement_command(
                    PlatformCommand::SetWindowTopmost(SetWindowTopmostCommand {
                        window_handle: native,
                        enabled: true,
                    }),
                    cx,
                );
            }
            Phase::Topmost => {
                pending.phase = Phase::MaterialStage;
                pending.command = uuid::Uuid::nil();
                self.pending_link_placement = Some(pending);
                self.wait_link_material(cx);
            }
            Phase::Stage => {
                pending.phase = Phase::Frame;
                let Some(window) = pending.window else {
                    return true;
                };
                let operation = pending.operation;
                self.pending_link_placement = Some(pending);
                let owner = cx.entity().downgrade();
                if window
                    .update(cx, |_, window, _| {
                        window.on_next_frame(move |_, cx| {
                            cx.defer(move |cx| {
                                let _ = owner.update(cx, |owner, cx| {
                                    owner.reveal_link_placement(operation, cx)
                                });
                            });
                        });
                        window.refresh();
                    })
                    .is_err()
                {
                    self.retire_link_placement("window_closed", cx);
                }
            }
            Phase::HideAfterFrame => {
                pending.phase = Phase::TargetMonitor;
                let native = pending.native;
                let (x, y, _, _) = pending.bounds;
                self.pending_link_placement = Some(pending);
                self.send_link_placement_command(
                    PlatformCommand::SetWindowPosition(SetWindowPositionCommand {
                        window_handle: native,
                        x,
                        y,
                    }),
                    cx,
                );
            }
            Phase::TargetMonitor => {
                pending.phase = Phase::FinalBounds;
                let native = pending.native;
                let (x, y, width, height) = pending.bounds;
                self.pending_link_placement = Some(pending);
                self.send_link_placement_command(
                    PlatformCommand::SetWindowBounds(SetWindowBoundsCommand {
                        window_handle: native,
                        x,
                        y,
                        width,
                        height,
                    }),
                    cx,
                );
            }
            Phase::FinalBounds => {
                pending.phase = Phase::MaterialReveal;
                pending.command = uuid::Uuid::nil();
                self.pending_link_placement = Some(pending);
                self.wait_link_material(cx);
            }
            Phase::Reveal | Phase::ExpandedBounds => {
                info!(event = "tools.link_panel.presentation_completed", correlation_id = %pending.operation,
                    success = true, elapsed_ms = pending.started.elapsed().as_millis() as u64);
            }
            Phase::Frame | Phase::MaterialStage | Phase::MaterialReveal => {
                self.pending_link_placement = Some(pending);
            }
        }
        true
    }

    fn prepare_link_placement(
        &mut self,
        mut pending: PendingLinkPlacement,
        cx: &mut Context<Self>,
    ) {
        let monitor = pending.monitor.expect("monitor query completed");
        self.link_panel.monitor = Some(monitor);
        let panel_size = size(
            px(crate::tools::link_panel::LINK_PANEL_WIDTH),
            px(crate::tools::link_panel::link_panel_height(
                pending.links.len(),
                false,
            )),
        );
        let placement = crate::ui::monitor_placement::MonitorPlacement::from_snapshot(monitor, cx);
        let bounds = link_panel_bounds(
            self.config.clipboard.link_panel_position,
            panel_size,
            placement.work_area,
        );
        let device = bounds.to_device_pixels(placement.scale);
        pending.bounds = (
            device.origin.x.into(),
            device.origin.y.into(),
            device.size.width.into(),
            device.size.height.into(),
        );
        self.show_link_panel(std::mem::take(&mut pending.links), cx);
        pending.generation = self.link_panel.generation;
        pending.window = self.link_panel.window;
        pending.native = self.link_panel.native_window_handle;
        pending.phase = Phase::NoActivate;
        let native = pending.native;
        self.pending_link_placement = Some(pending);
        if native == 0 {
            self.retire_link_placement("native_window_unavailable", cx);
            return;
        }
        self.send_link_placement_command(
            PlatformCommand::SetWindowNoActivate(SetWindowNoActivateCommand {
                window_handle: native,
                enabled: true,
            }),
            cx,
        );
    }

    fn wait_link_material(&mut self, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_link_placement.as_ref() else {
            return;
        };
        let Some(window) = pending.window else {
            self.retire_link_placement("window_closed", cx);
            return;
        };
        if !self.link_placement_is_current(pending) || !cx.windows().contains(&window) {
            self.retire_link_placement("retired_before_material", cx);
            return;
        }
        let operation = pending.operation;
        let phase = pending.phase;
        let native = pending.native;
        if material_runtime::receipt(window, cx)
            .is_some_and(|receipt| receipt.window_handle == native)
        {
            self.finish_link_material(operation, window, native, phase, Ok(()), cx);
            return;
        }
        let started = pending.started;
        let platform = Rc::clone(&self.platform);
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            if !owner
                .read_with(cx, |owner, cx| {
                    owner
                        .pending_link_placement
                        .as_ref()
                        .is_some_and(|pending| {
                            pending.operation == operation
                                && pending.phase == phase
                                && pending.window == Some(window)
                                && pending.native == native
                                && owner.link_placement_is_current(pending)
                                && cx.windows().contains(&window)
                        })
                })
                .unwrap_or(false)
            {
                return;
            }
            let timeout = Duration::from_secs(3).saturating_sub(started.elapsed());
            material_runtime::when_ready(
                window,
                platform,
                MaterialSurface::Transient,
                timeout,
                move |result, cx| {
                    let _ = owner.update(cx, |owner, cx| {
                        owner.finish_link_material(operation, window, native, phase, result, cx);
                    });
                },
                cx,
            );
        });
    }

    fn finish_link_material(
        &mut self,
        operation: uuid::Uuid,
        window: AnyWindowHandle,
        native: i64,
        phase: Phase,
        result: Result<(), &'static str>,
        cx: &mut Context<Self>,
    ) {
        let Some(pending) = self.pending_link_placement.as_ref().filter(|pending| {
            pending.operation == operation
                && pending.window == Some(window)
                && pending.native == native
                && pending.phase == phase
        }) else {
            return;
        };
        if let Err(reason) = result {
            self.retire_link_placement(reason, cx);
            return;
        }
        if !self.link_placement_is_current(pending) || !cx.windows().contains(&window) {
            self.retire_link_placement("retired_before_material_reveal", cx);
            return;
        }
        if material_runtime::receipt(window, cx)
            .is_none_or(|receipt| receipt.window_handle != native)
        {
            self.wait_link_material(cx);
            return;
        }
        self.pending_link_placement
            .as_mut()
            .expect("qualified link placement")
            .phase = match phase {
            Phase::MaterialStage => Phase::Stage,
            Phase::MaterialReveal => Phase::Reveal,
            _ => return,
        };
        self.send_link_placement_command(
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: native,
                visible: true,
            }),
            cx,
        );
    }

    fn reveal_link_placement(&mut self, operation: uuid::Uuid, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_link_placement.as_ref() else {
            return;
        };
        if pending.operation != operation || pending.phase != Phase::Frame {
            return;
        }
        if !self.link_placement_is_current(pending) {
            self.retire_link_placement("retired_before_reveal", cx);
            return;
        }
        let native = pending.native;
        self.pending_link_placement
            .as_mut()
            .expect("current placement")
            .phase = Phase::HideAfterFrame;
        self.send_link_placement_command(
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: native,
                visible: false,
            }),
            cx,
        );
    }

    pub(super) fn retire_link_placement(&mut self, reason: &'static str, cx: &mut Context<Self>) {
        let Some(pending) = self.pending_link_placement.take() else {
            return;
        };
        info!(event = "tools.link_panel.presentation_retired", correlation_id = %pending.operation, reason,
            elapsed_ms = pending.started.elapsed().as_millis() as u64);
        if let Some(window) = pending
            .window
            .filter(|window| self.link_panel.window == Some(*window))
        {
            if let Some(mut preparation) = material_runtime::take_preparation(window, cx) {
                preparation.retire(&self.platform);
            }
            self.link_panel.window = None;
            self.link_panel.native_window_handle = 0;
            self.link_panel.generation = self.link_panel.generation.wrapping_add(1);
            cx.defer(move |cx| {
                let _ = window.update(cx, |_, window, _| window.remove_window());
            });
        }
    }
}

#[cfg(test)]
#[path = "link_placement_tests.rs"]
mod tests;
