use super::*;
use nxtools_protocol::{CursorMonitorSnapshot, PlatformCommandResult};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Monitor,
    Hide,
    NoActivate,
    Bounds,
    Topmost,
    Stage,
    Frame,
    HideAfterFrame,
    TargetMonitor,
    FinalBounds,
    Reveal,
}

pub(super) struct PendingLinkPlacement {
    operation: uuid::Uuid,
    command: uuid::Uuid,
    generation: u64,
    pause_generation: u64,
    started: Instant,
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
            phase: Phase::Monitor,
            links,
            monitor: None,
            window: None,
            native: 0,
            bounds: (0, 0, 0, 0),
        });
        info!(event = "tools.link_panel.presentation_accepted", correlation_id = %operation);
        self.send_link_placement_command(PlatformCommand::QueryCursorMonitor, cx);
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
                pending.phase = Phase::Stage;
                let native = pending.native;
                self.pending_link_placement = Some(pending);
                self.send_link_placement_command(
                    PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                        window_handle: native,
                        visible: true,
                    }),
                    cx,
                );
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
                pending.phase = Phase::Reveal;
                let native = pending.native;
                self.pending_link_placement = Some(pending);
                self.send_link_placement_command(
                    PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                        window_handle: native,
                        visible: true,
                    }),
                    cx,
                );
            }
            Phase::Reveal => {
                info!(event = "tools.link_panel.presentation_completed", correlation_id = %pending.operation,
                    success = true, elapsed_ms = pending.started.elapsed().as_millis() as u64);
            }
            Phase::Frame => self.pending_link_placement = Some(pending),
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
