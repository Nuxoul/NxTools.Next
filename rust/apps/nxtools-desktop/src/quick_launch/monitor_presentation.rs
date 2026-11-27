//! First-frame and native readback sequence for the original Quick Launch popup.
use super::{QUICK_LAUNCH_PANEL_STAGING_COORDINATE, QuickLaunchApplication, QuickLaunchWindow};
use crate::ui::material::{Preparation, UiMaterialCapabilities};
use gpui_kit::{App, Context, Subscription};
use nxtools_ipc::PlatformBridge;
use nxtools_protocol::window_material::MaterialSurface;
use nxtools_protocol::{
    PlatformCommand, PlatformCommandResult, QueryWindowGeometryCommand, QuickLaunchPanelRequest,
    SetWindowBoundsCommand, SetWindowPositionCommand, SetWindowVisibilityCommand, WindowGeometry,
    WindowGeometryRect,
};
use std::time::{Duration, Instant};
use uuid::Uuid;

const PRESENTATION_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy)]
pub(super) struct PlacementTarget {
    work: WindowGeometryRect,
    display_id: Option<u64>,
    dpi: u32,
    client: (i32, i32),
}

impl PlacementTarget {
    pub(super) fn from_request(request: &QuickLaunchPanelRequest, cx: &App) -> Self {
        let placement = crate::ui::monitor_placement::cursor_monitor_placement(
            request.cursor_monitor,
            request.cursor_display_index,
            request.cursor_monitor_dpi,
            cx,
        );
        let (bounds, _) = super::panel::quick_launch_bounds(request, false, cx);
        let px = |value: gpui_kit::Pixels| (value.as_f32() * placement.scale).round() as i32;
        let (work, display_id, dpi) = if let Some(snapshot) = request.cursor_monitor {
            (
                WindowGeometryRect {
                    left: snapshot.work_left,
                    top: snapshot.work_top,
                    right: (i64::from(snapshot.work_left) + i64::from(snapshot.work_width)) as i32,
                    bottom: (i64::from(snapshot.work_top) + i64::from(snapshot.work_height)) as i32,
                },
                Some(snapshot.display_id),
                snapshot.dpi,
            )
        } else {
            let area = placement.work_area;
            (
                WindowGeometryRect {
                    left: px(area.left()),
                    top: px(area.top()),
                    right: px(area.right()),
                    bottom: px(area.bottom()),
                },
                placement.display_id.map(u64::from),
                request.cursor_monitor_dpi.max(96),
            )
        };
        Self {
            work,
            display_id,
            dpi,
            client: (px(bounds.size.width), px(bounds.size.height)),
        }
    }

    fn origin(self, outer: (i32, i32)) -> (i32, i32) {
        let work_size = dimensions(self.work);
        let free_x = (work_size.0 - outer.0).max(0);
        let free_y = (work_size.1 - outer.1).max(0);
        (
            self.work.left + free_x / 2,
            self.work.top + (work_size.1 / 4).min(free_y),
        )
    }

    fn matching_target(self, geometry: WindowGeometry) -> bool {
        !geometry.visible
            && geometry.display_id != 0
            && self.display_id.is_none_or(|id| id == geometry.display_id)
            && self.dpi == geometry.dpi
    }

    fn final_bounds(self, hwnd: i64, geometry: WindowGeometry) -> Option<SetWindowBoundsCommand> {
        if !self.matching_target(geometry) || dimensions(geometry.client) != self.client {
            return None;
        }
        let actual = dimensions(geometry.outer);
        let work = dimensions(self.work);
        let outer = (actual.0.min(work.0), actual.1.min(work.1));
        let client = (
            self.client.0 - (actual.0 - outer.0),
            self.client.1 - (actual.1 - outer.1),
        );
        let origin = self.origin(outer);
        (client.0 > 0 && client.1 > 0).then(|| bounds(hwnd, origin, client))
    }

    fn confirms(self, command: &SetWindowBoundsCommand, geometry: WindowGeometry) -> bool {
        let outer = geometry.outer;
        let actual = dimensions(outer);
        self.matching_target(geometry)
            && actual.0 > 0
            && actual.1 > 0
            && dimensions(geometry.client) == (command.width, command.height)
            && (outer.left, outer.top) == (command.x, command.y)
            && self.origin(actual) == (outer.left, outer.top)
            && outer.right <= self.work.right
            && outer.bottom <= self.work.bottom
    }
}

fn dimensions(rect: WindowGeometryRect) -> (i32, i32) {
    (
        rect.right.saturating_sub(rect.left),
        rect.bottom.saturating_sub(rect.top),
    )
}

fn bounds(hwnd: i64, (x, y): (i32, i32), (width, height): (i32, i32)) -> SetWindowBoundsCommand {
    SetWindowBoundsCommand {
        window_handle: hwnd,
        x,
        y,
        width,
        height,
    }
}

#[derive(Clone, Copy)]
pub(super) struct PopupResizeLimits {
    work: WindowGeometryRect,
    frame: (i32, i32),
}

impl PopupResizeLimits {
    pub(super) fn constrain(self, origin: (i32, i32), desired: (i32, i32)) -> (i32, i32) {
        (
            desired
                .0
                .min((self.work.right - origin.0 - self.frame.0).max(1)),
            desired
                .1
                .min((self.work.bottom - origin.1 - self.frame.1).max(1)),
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PopupIdentity {
    session: Uuid,
    hwnd: i64,
    connection: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    Hide,
    StageMove,
    StageSize,
    StageCheck,
    StageShow,
    Frame,
    HideAfterFrame,
    TargetMove,
    TargetSize,
    Measure,
    Place,
    Verify,
    Show,
}

enum Progress {
    Send(Step),
    WaitForFrame,
    Shown((i32, i32)),
    Failed(&'static str),
}

pub(super) struct PopupPresentation {
    identity: PopupIdentity,
    target: PlacementTarget,
    op: Uuid,
    started: Instant,
    step: Step,
    receipt: Option<Uuid>,
    final_bounds: Option<SetWindowBoundsCommand>,
    resize_limits: Option<PopupResizeLimits>,
    material: Preparation,
    material_started: bool,
    waiting_show: Option<Step>,
    shown_origin: Option<(i32, i32)>,
    _material_observers: Vec<Subscription>,
}

impl PopupPresentation {
    fn new(identity: PopupIdentity, target: PlacementTarget) -> Self {
        Self {
            identity,
            target,
            op: Uuid::new_v4(),
            started: Instant::now(),
            step: Step::Hide,
            receipt: None,
            final_bounds: None,
            resize_limits: None,
            material: Preparation::default(),
            material_started: false,
            waiting_show: None,
            _material_observers: Vec::new(),
            shown_origin: None,
        }
    }

    fn command(&self, step: Step) -> PlatformCommand {
        let hwnd = self.identity.hwnd;
        let visibility = |visible| {
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: hwnd,
                visible,
            })
        };
        let position = |(x, y)| {
            PlatformCommand::SetWindowPosition(SetWindowPositionCommand {
                window_handle: hwnd,
                x,
                y,
            })
        };
        let offscreen = (
            QUICK_LAUNCH_PANEL_STAGING_COORDINATE,
            QUICK_LAUNCH_PANEL_STAGING_COORDINATE,
        );
        match step {
            Step::Hide | Step::HideAfterFrame => visibility(false),
            Step::StageMove => position(offscreen),
            Step::StageSize => {
                PlatformCommand::SetWindowBounds(bounds(hwnd, offscreen, self.target.client))
            }
            Step::StageCheck | Step::Measure | Step::Verify => {
                PlatformCommand::QueryWindowGeometry(QueryWindowGeometryCommand {
                    window_handle: hwnd,
                })
            }
            Step::StageShow | Step::Show => visibility(true),
            Step::TargetMove => position(self.target.origin(self.target.client)),
            Step::TargetSize => PlatformCommand::SetWindowBounds(bounds(
                hwnd,
                self.target.origin(self.target.client),
                self.target.client,
            )),
            Step::Place => PlatformCommand::SetWindowBounds(
                self.final_bounds.clone().expect("measured final bounds"),
            ),
            Step::Frame => unreachable!("GPUI frame is not a platform command"),
        }
    }

    fn send(&mut self, platform: &PlatformBridge, step: Step) -> Result<(), &'static str> {
        if platform.connection_generation() != Some(self.identity.connection) {
            return Err("disconnected");
        }
        self.step = step;
        let receipt = Uuid::new_v4();
        self.receipt = Some(receipt);
        platform
            .send_command_with_id_correlated(self.command(step), receipt, self.op)
            .map_err(|_| "not_submitted")?;
        Ok(())
    }

    fn result(
        &mut self,
        receipt: Uuid,
        current: Option<PopupIdentity>,
        result: &PlatformCommandResult,
    ) -> Option<Progress> {
        if self.receipt != Some(receipt) {
            return None;
        }
        self.receipt = None;
        if current != Some(self.identity) {
            return Some(Progress::Failed("stale"));
        }
        if !result.success {
            return Some(Progress::Failed(
                if result.error.as_deref() == Some("platform command timed out") {
                    "timeout"
                } else {
                    "native_failure"
                },
            ));
        }
        let geometry = || {
            result
                .data
                .as_deref()
                .and_then(|data| serde_json::from_str::<WindowGeometry>(data).ok())
        };
        let next = match self.step {
            Step::Hide => Progress::Send(Step::StageMove),
            // Hidden GPUI creation does not apply its requested client size on Windows.
            Step::StageMove => Progress::Send(Step::StageSize),
            Step::StageSize => Progress::Send(Step::StageCheck),
            Step::StageCheck if geometry().is_some_and(|g| !g.visible && g.display_id == 0) => {
                Progress::Send(Step::StageShow)
            }
            Step::StageCheck => Progress::Failed("offscreen_stage_unconfirmed"),
            Step::StageShow => {
                self.step = Step::Frame;
                Progress::WaitForFrame
            }
            Step::HideAfterFrame => Progress::Send(Step::TargetMove),
            // Move while hidden first: the next size command must use frame insets
            // sampled after Windows has switched this HWND to the target DPI.
            Step::TargetMove => Progress::Send(Step::TargetSize),
            Step::TargetSize => Progress::Send(Step::Measure),
            Step::Measure => {
                self.final_bounds =
                    geometry().and_then(|g| self.target.final_bounds(self.identity.hwnd, g));
                if self.final_bounds.is_some() {
                    Progress::Send(Step::Place)
                } else {
                    Progress::Failed("target_geometry_unconfirmed")
                }
            }
            Step::Place => Progress::Send(Step::Verify),
            Step::Verify => {
                let final_bounds = self.final_bounds.as_ref().expect("placed bounds");
                if let Some(geometry) =
                    geometry().filter(|g| self.target.confirms(final_bounds, *g))
                {
                    let outer = dimensions(geometry.outer);
                    let client = dimensions(geometry.client);
                    self.resize_limits = Some(PopupResizeLimits {
                        work: self.target.work,
                        frame: ((outer.0 - client.0).max(0), (outer.1 - client.1).max(0)),
                    });
                    Progress::Send(Step::Show)
                } else {
                    Progress::Failed("final_geometry_unconfirmed")
                }
            }
            Step::Show => {
                let placed = self.final_bounds.as_ref().expect("verified bounds");
                Progress::Shown((placed.x, placed.y))
            }
            Step::Frame => unreachable!("no native receipt while waiting for a frame"),
        };
        Some(next)
    }

    fn frame_is_current(&self, op: Uuid, current: Option<PopupIdentity>) -> bool {
        self.op == op && self.step == Step::Frame && current == Some(self.identity)
    }

    fn finish(self, outcome: &'static str, native_error_code: Option<i32>) {
        tracing::info!(event = "quick_launch.popup.presentation_finished", correlation_id = %self.op,
            session = %self.identity.session, step = ?self.step, outcome, native_error_code = ?native_error_code,
            elapsed_ms = self.started.elapsed().as_millis() as u64);
    }
}

impl QuickLaunchApplication {
    fn current_popup_presentation_identity(&self) -> Option<PopupIdentity> {
        if !self.feature_enabled() || self.active_summon.is_none() {
            return None;
        }
        let popup = self.popup.as_ref()?;
        (popup.native_window_handle != 0).then_some(PopupIdentity {
            session: self.popup_session_id,
            hwnd: popup.native_window_handle,
            connection: self.platform.connection_generation()?,
        })
    }

    pub(super) fn begin_popup_presentation(
        &mut self,
        target: PlacementTarget,
        cx: &mut Context<Self>,
    ) {
        self.retire_popup_presentation("superseded");
        self.popup_origin_device_pixels = None;
        self.pending_popup_resize = None;
        self.pending_window_activation.clear();
        self.pending_panel_hides.clear();
        let Some(identity) = self.current_popup_presentation_identity() else {
            self.hide_popup(cx);
            return;
        };
        let mut pending = PopupPresentation::new(identity, target);
        if let Some(popup) = self.popup.as_ref()
            && let Some(preparation) =
                crate::ui::material::runtime::take_preparation(popup.window, cx)
        {
            pending.material = preparation;
        }
        let op = pending.op;
        pending._material_observers = vec![
            cx.observe_global::<UiMaterialCapabilities>(move |app, cx| {
                app.defer_popup_material(op, cx);
            }),
            cx.observe_global::<crate::theme::UiThemeState>(move |app, cx| {
                app.defer_popup_material(op, cx);
            }),
        ];
        self.popup_presentation = Some(pending);
        tracing::info!(event = "quick_launch.popup.presentation_accepted", correlation_id = %op, session = %identity.session);
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| {
                if app.popup_presentation.as_ref().is_some_and(|pending| pending.op == op) {
                    tracing::info!(event = "quick_launch.popup.presentation_started", correlation_id = %op);
                    app.send_popup_presentation_step(Step::Hide, cx);
                }
            });
        });
        cx.spawn(async move |owner, cx| {
            cx.background_executor().timer(PRESENTATION_TIMEOUT).await;
            let _ = owner.update(cx, |app, cx| {
                if app
                    .popup_presentation
                    .as_ref()
                    .is_some_and(|pending| pending.op == op)
                {
                    app.fail_popup_presentation("timeout", None, cx);
                }
            });
        })
        .detach();
    }

    fn send_popup_presentation_step(&mut self, step: Step, cx: &mut Context<Self>) {
        let current = self.current_popup_presentation_identity();
        let Some(pending) = &mut self.popup_presentation else {
            return;
        };
        if current != Some(pending.identity) {
            self.fail_popup_presentation("stale", None, cx);
            return;
        }
        if pending.started.elapsed() >= PRESENTATION_TIMEOUT {
            self.fail_popup_presentation("timeout", None, cx);
            return;
        }
        if matches!(step, Step::StageShow | Step::Show) {
            pending.material_started = true;
            pending.waiting_show = Some(step);
            match pending.material.ensure(
                &self.platform,
                pending.identity.hwnd,
                MaterialSurface::Transient,
                cx,
            ) {
                Ok(true) => pending.waiting_show = None,
                Ok(false) => return,
                Err(outcome) => {
                    self.fail_popup_presentation(outcome, None, cx);
                    return;
                }
            }
        }
        if let Err(outcome) = pending.send(&self.platform, step) {
            self.fail_popup_presentation(outcome, None, cx);
        }
    }

    fn defer_popup_material(&self, op: Uuid, cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| app.advance_popup_material(op, cx));
        });
    }

    fn advance_popup_material(&mut self, op: Uuid, cx: &mut Context<Self>) {
        let current = self.current_popup_presentation_identity();
        let Some(pending) = self
            .popup_presentation
            .as_mut()
            .filter(|pending| pending.op == op)
        else {
            return;
        };
        if current != Some(pending.identity) {
            self.fail_popup_presentation("stale", None, cx);
            return;
        }
        if pending.started.elapsed() >= PRESENTATION_TIMEOUT {
            self.fail_popup_presentation("timeout", None, cx);
            return;
        }
        if !pending.material_started {
            return;
        }
        match pending.material.ensure(
            &self.platform,
            pending.identity.hwnd,
            MaterialSurface::Transient,
            cx,
        ) {
            Ok(true) => {
                if let Some(step) = pending.waiting_show.take() {
                    self.send_popup_presentation_step(step, cx);
                } else if let Some(origin) = pending.shown_origin.take() {
                    let hwnd = pending.identity.hwnd;
                    let resize_limits = pending.resize_limits;
                    self.handoff_popup_material(cx);
                    self.popup_origin_device_pixels = Some(origin);
                    self.publish_popup_resize_limits(resize_limits, origin, cx);
                    self.begin_window_activation(hwnd, cx);
                }
            }
            Ok(false) => {}
            Err(outcome) => self.fail_popup_presentation(outcome, None, cx),
        }
    }

    pub(super) fn popup_presentation_result(
        &mut self,
        receipt: Uuid,
        result: &PlatformCommandResult,
        cx: &mut Context<Self>,
    ) -> bool {
        let current = self.current_popup_presentation_identity();
        let Some(pending) = &mut self.popup_presentation else {
            return false;
        };
        if let Some(material) = pending.material.accept(receipt, result, cx) {
            let op = pending.op;
            if current != Some(pending.identity) {
                self.fail_popup_presentation("stale", None, cx);
            } else if let Err(outcome) = material {
                self.fail_popup_presentation(outcome, result.native_error_code, cx);
            } else {
                // Neither a native receipt nor a global observer may re-enter a leased view.
                // Re-read the latest facts/style on the original owner's next turn.
                self.defer_popup_material(op, cx);
            }
            return true;
        }
        let Some(progress) = pending.result(receipt, current, result) else {
            return false;
        };
        let op = pending.op;
        match progress {
            Progress::Send(step) => self.send_popup_presentation_step(step, cx),
            Progress::WaitForFrame => self.wait_for_popup_frame(op, cx),
            Progress::Shown(origin) => {
                pending.shown_origin = Some(origin);
                self.defer_popup_material(op, cx);
            }
            Progress::Failed(outcome) => {
                self.fail_popup_presentation(outcome, result.native_error_code, cx)
            }
        }
        true
    }

    fn wait_for_popup_frame(&mut self, op: Uuid, cx: &mut Context<Self>) {
        let owner = cx.entity().downgrade();
        let Some(QuickLaunchWindow { window, .. }) = self.popup.clone() else {
            return;
        };
        let scheduled = window.update(cx, |_, window, _| {
            window.on_next_frame(move |_, cx| {
                // GPUI invokes the frame callback with the Window leased. Do not
                // call the application or native effects through that lease.
                cx.defer(move |cx| {
                    let _ = owner.update(cx, |app, cx| {
                        let current = app.current_popup_presentation_identity();
                        if app
                            .popup_presentation
                            .as_ref()
                            .is_some_and(|pending| pending.frame_is_current(op, current))
                        {
                            app.send_popup_presentation_step(Step::HideAfterFrame, cx);
                        }
                    });
                });
            });
            window.refresh();
        });
        if scheduled.is_err() {
            self.fail_popup_presentation("window_retired", None, cx);
        }
    }

    fn handoff_popup_material(&mut self, cx: &mut Context<Self>) {
        let Some(mut pending) = self.popup_presentation.take() else {
            return;
        };
        let Some(popup) = self.popup.clone() else {
            pending.material.retire(&self.platform);
            return;
        };
        let material = std::mem::take(&mut pending.material);
        let identity = pending.identity;
        let platform = self.platform.clone();
        let owner = cx.entity().downgrade();
        pending.finish("shown", None);
        cx.defer(move |cx| {
            let current = owner
                .read_with(cx, |app, _| {
                    app.popup_session_id == identity.session
                        && app.popup_presentation.is_none()
                        && app
                            .popup
                            .as_ref()
                            .is_some_and(|current| current.window == popup.window)
                })
                .unwrap_or(false);
            if current {
                crate::ui::material::runtime::track_prepared(popup.window, platform, material, cx);
            }
        });
    }

    pub(super) fn retire_popup_presentation(&mut self, outcome: &'static str) {
        if let Some(mut pending) = self.popup_presentation.take() {
            pending.material.retire(&self.platform);
            pending.finish(outcome, None);
        }
    }

    pub(super) fn disconnect_popup_presentation(
        &mut self,
        generation: u64,
        cx: &mut Context<Self>,
    ) {
        if self
            .popup_presentation
            .as_ref()
            .is_some_and(|pending| pending.identity.connection == generation)
        {
            self.fail_popup_presentation("disconnected", None, cx);
        }
    }

    fn fail_popup_presentation(
        &mut self,
        outcome: &'static str,
        native_error_code: Option<i32>,
        cx: &mut Context<Self>,
    ) {
        let Some(mut pending) = self.popup_presentation.take() else {
            return;
        };
        let identity = pending.identity;
        pending.material.retire(&self.platform);
        pending.finish(outcome, native_error_code);
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            let _ = owner.update(cx, |app, cx| {
                if app.feature_enabled()
                    && app.popup_session_id == identity.session
                    && app
                        .popup
                        .as_ref()
                        .is_some_and(|popup| popup.native_window_handle == identity.hwnd)
                {
                    app.hide_popup(cx);
                }
            });
        });
    }
}

impl QuickLaunchApplication {
    fn publish_popup_resize_limits(
        &self,
        limits: Option<PopupResizeLimits>,
        origin: (i32, i32),
        cx: &mut Context<Self>,
    ) {
        let Some(popup) = self.popup.clone() else {
            return;
        };
        let identity = self.current_popup_presentation_identity();
        let owner = cx.entity().downgrade();
        cx.defer(move |cx| {
            if !owner
                .read_with(cx, |app, _| {
                    app.current_popup_presentation_identity() == identity
                })
                .unwrap_or(false)
            {
                return;
            }
            let _ = popup.panel.update(cx, |panel, cx| {
                panel.native_resize_limits = limits.map(|limits| (limits, origin));
                cx.notify();
            });
            let _ = owner.update(cx, |app, cx| {
                if app.current_popup_presentation_identity() == identity {
                    app.resize_popup_content(cx);
                }
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxtools_protocol::{Envelope, Message};

    const HWND: i64 = 987;
    const DISPLAY: u64 = 0xABCD;

    fn target(dpi: u32) -> PlacementTarget {
        PlacementTarget {
            work: WindowGeometryRect {
                left: -1800,
                top: 76,
                right: -600,
                bottom: 976,
            },
            display_id: Some(DISPLAY),
            dpi,
            client: (640 * dpi as i32 / 96, 64 * dpi as i32 / 96),
        }
    }

    fn geometry(target: PlacementTarget, x: i32, y: i32, display_id: u64) -> WindowGeometry {
        let (width, height) = target.client;
        WindowGeometry {
            outer: WindowGeometryRect {
                left: x,
                top: y,
                right: x + width + 20,
                bottom: y + height + 12,
            },
            client: WindowGeometryRect {
                left: x + 10,
                top: y + 6,
                right: x + 10 + width,
                bottom: y + 6 + height,
            },
            display_id,
            dpi: target.dpi,
            visible: false,
        }
    }

    fn ok(geometry: Option<WindowGeometry>) -> PlatformCommandResult {
        PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: geometry.map(|g| serde_json::to_string(&g).expect("fixture geometry")),
        }
    }

    fn send(
        pending: &mut PopupPresentation,
        platform: &PlatformBridge,
        endpoint: &nxtools_ipc::PlatformServerEndpoint,
        step: Step,
    ) -> Envelope {
        pending
            .send(platform, step)
            .expect("production bridge admission");
        let envelope = endpoint
            .try_receive_command_for_contract_test()
            .expect("native command");
        assert_eq!(pending.receipt, Some(envelope.id));
        assert_eq!(envelope.correlation_id, Some(pending.op));
        assert!(endpoint.command_pending_for_contract_test(envelope.id));
        envelope
    }

    #[test]
    fn r1_popup_presentation_confirms_outer_geometry_before_final_show() {
        for dpi in [96, 120, 144] {
            let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
            endpoint.set_connected_for_contract_test(true);
            let identity = PopupIdentity {
                session: Uuid::new_v4(),
                hwnd: HWND,
                connection: platform.connection_generation().expect("connection"),
            };
            let target = target(dpi);
            let mut pending = PopupPresentation::new(identity, target);
            let mut next = Step::Hide;
            for (current, expected) in [
                (Step::Hide, Step::StageMove),
                (Step::StageMove, Step::StageSize),
                (Step::StageSize, Step::StageCheck),
                (Step::StageCheck, Step::StageShow),
                (Step::HideAfterFrame, Step::TargetMove),
                (Step::TargetMove, Step::TargetSize),
                (Step::TargetSize, Step::Measure),
                (Step::Measure, Step::Place),
                (Step::Place, Step::Verify),
                (Step::Verify, Step::Show),
            ] {
                if current == Step::HideAfterFrame {
                    let stage_show = send(&mut pending, &platform, &endpoint, next);
                    assert!(matches!(
                        stage_show.message,
                        Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                            SetWindowVisibilityCommand { visible: true, .. }
                        ))
                    ));
                    assert!(matches!(
                        pending.result(stage_show.id, Some(identity), &ok(None)),
                        Some(Progress::WaitForFrame)
                    ));
                    assert!(pending.frame_is_current(pending.op, Some(identity)));
                    assert!(endpoint.try_receive_command_for_contract_test().is_err());
                    next = Step::HideAfterFrame;
                }
                assert_eq!(next, current);
                let command = send(&mut pending, &platform, &endpoint, current);
                let (width, height) = target.client;
                let x = -1800 + (1200 - (width + 20)) / 2;
                let y = 76 + 900 / 4;
                if current == Step::Place {
                    assert!(
                        matches!(command.message, Message::PlatformCommand(PlatformCommand::SetWindowBounds(ref b)) if (b.window_handle, b.x, b.y, b.width, b.height) == (HWND, x, y, width, height))
                    );
                }
                let data = match current {
                    Step::StageCheck => Some(geometry(target, -30_000, -30_000, 0)),
                    Step::Measure => Some(geometry(target, x + 10, y, DISPLAY)),
                    Step::Verify => Some(geometry(target, x, y, DISPLAY)),
                    _ => None,
                };
                let Progress::Send(step) = pending
                    .result(command.id, Some(identity), &ok(data))
                    .expect("current result")
                else {
                    panic!("unexpected terminal at {current:?}")
                };
                assert_eq!(step, expected);
                next = step;
            }
            let limits = pending.resize_limits.expect("confirmed outer frame limits");
            let placed = pending.final_bounds.as_ref().expect("final rectangle");
            let expanded = limits.constrain(
                (placed.x, placed.y),
                (target.client.0, 530 * dpi as i32 / 96),
            );
            assert!(placed.y + expanded.1 + 12 <= target.work.bottom);
            assert_eq!(
                expanded.1,
                (530 * dpi as i32 / 96).min(target.work.bottom - placed.y - 12)
            );
            let show = send(&mut pending, &platform, &endpoint, next);
            assert!(matches!(
                show.message,
                Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand { visible: true, .. }
                ))
            ));
            assert!(matches!(
                pending.result(show.id, Some(identity), &ok(None)),
                Some(Progress::Shown((_, 301)))
            ));
        }
    }

    #[test]
    fn r1_popup_presentation_rejects_stale_frame_geometry_and_disconnect() {
        let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
        endpoint.set_connected_for_contract_test(true);
        let identity = PopupIdentity {
            session: Uuid::new_v4(),
            hwnd: HWND,
            connection: platform.connection_generation().expect("connection"),
        };
        let replaced = PopupIdentity {
            session: Uuid::new_v4(),
            ..identity
        };
        let target = target(144);
        let mut pending = PopupPresentation::new(identity, target);
        let probe = send(&mut pending, &platform, &endpoint, Step::StageCheck);
        assert!(matches!(
            pending.result(
                probe.id,
                Some(identity),
                &ok(Some(geometry(target, -30_000, -30_000, DISPLAY)))
            ),
            Some(Progress::Failed("offscreen_stage_unconfirmed"))
        ));
        let staged = send(&mut pending, &platform, &endpoint, Step::StageShow);
        assert!(matches!(
            pending.result(staged.id, Some(identity), &ok(None)),
            Some(Progress::WaitForFrame)
        ));
        assert!(!pending.frame_is_current(pending.op, Some(replaced)));
        assert!(!pending.frame_is_current(pending.op, None));
        let measure = send(&mut pending, &platform, &endpoint, Step::Measure);
        assert!(
            pending
                .result(staged.id, Some(identity), &ok(None))
                .is_none()
        );
        assert!(matches!(
            pending.result(
                measure.id,
                Some(replaced),
                &ok(Some(geometry(target, -1700, 301, DISPLAY)))
            ),
            Some(Progress::Failed("stale"))
        ));
        let measure = send(&mut pending, &platform, &endpoint, Step::Measure);
        assert!(matches!(
            pending.result(
                measure.id,
                Some(identity),
                &ok(Some(geometry(target, -1700, 301, DISPLAY + 1)))
            ),
            Some(Progress::Failed("target_geometry_unconfirmed"))
        ));
        endpoint.set_connected_for_contract_test(true);
        assert_eq!(pending.send(&platform, Step::Show), Err("disconnected"));
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
    }

    #[gpui_kit::test]
    fn r1_popup_presentation_real_hide_retires_a_delayed_native_receipt(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use crate::notifications::NotificationCenter;
        use gpui_kit::AppContext as _;
        use gpui_kit::base::Root;
        use std::{cell::RefCell, rc::Rc};

        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let tmp = tempfile::tempdir().expect("isolated Quick Launch state");
        let catalog = nxtools_finder::FinderCatalog::open(tmp.path().join("history.db"), &[])
            .expect("fixture catalog");
        let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
        endpoint.set_connected_for_contract_test(true);
        let (indexer, _indexer_endpoint) = crate::runtime::indexer_bridge(8);
        let notifications = cx.new(|_| NotificationCenter::default());
        let app = cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                nxtools_core::AppConfig::default(),
                tmp.path().join("config.json"),
                tmp.path().join("logs"),
                notifications,
            )
        });
        let slot = Rc::new(RefCell::new(None));
        let capture = slot.clone();
        let owner = app.downgrade();
        let _ = cx.add_window_view(move |window, cx| {
            let panel = cx.new(|cx| {
                super::super::panel::QuickLaunchPanel::new(
                    vec![],
                    nxtools_protocol::ForegroundContext {
                        window_handle: 1,
                        focus_window_handle: 1,
                        process_id: 1,
                        process_name: "fixture".into(),
                        window_title: "fixture".into(),
                    },
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
                native_window_handle: HWND,
            });
            Root::new(panel, window, cx)
        });
        let popup = slot.borrow().as_ref().expect("actual panel").clone();
        popup
            .window
            .update(cx, |_, window, cx| {
                popup
                    .panel
                    .update(cx, |panel, cx| {
                        panel.results_visible = true;
                        panel.native_resize_limits = Some((
                            PopupResizeLimits {
                                work: WindowGeometryRect {
                                    left: 0,
                                    top: 0,
                                    right: 800,
                                    bottom: 200,
                                },
                                frame: (20, 12),
                            },
                            (0, 64),
                        ));
                        let desired = panel.desired_content_size(window, cx);
                        assert_eq!(
                            (desired.height.as_f32() * window.scale_factor()).round() as i32,
                            124
                        );
                        panel.native_resize_limits = None;
                        panel.results_visible = false;
                    })
                    .expect("actual panel resize policy");
            })
            .expect("actual popup window");
        app.update(cx, |app, cx| {
            app.popup = slot.borrow_mut().take();
            app.active_summon = Some((1, 1));
            app.begin_popup_presentation(target(120), cx);
            assert!(app.popup_presentation.is_some());
            assert!(
                endpoint.try_receive_command_for_contract_test().is_err(),
                "first effect is deferred"
            );
        });
        cx.run_until_parked();
        let before = endpoint
            .try_receive_command_for_contract_test()
            .expect("staged hide command");
        assert!(matches!(
            before.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle: HWND,
                    visible: false
                }
            ))
        ));
        app.update(cx, |app, cx| {
            app.hide_popup(cx);
            assert!(app.popup_presentation.is_none());
            app.handle_platform_notification(
                nxtools_ipc::PlatformNotification::CommandResult {
                    correlation_id: before.id,
                    result: ok(None),
                },
                cx,
            );
            assert!(app.pending_window_activation.is_empty());
        });
        cx.run_until_parked();
        while let Ok(command) = endpoint.try_receive_command_for_contract_test() {
            assert!(!matches!(
                command.message,
                Message::PlatformCommand(
                    PlatformCommand::SetWindowPosition(_)
                        | PlatformCommand::ActivateWindow(_)
                        | PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                            visible: true,
                            ..
                        })
                )
            ));
        }
    }

    fn material_facts() -> nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged {
        nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged {
            correlation_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            revision: 1,
            supported: true,
            transparency_enabled: true,
            high_contrast: false,
            energy_saver: false,
            remote_session: false,
            native_error_code: 0,
            system_backdrop: true,
            rounded_corners: true,
        }
    }

    fn apply_facts(
        cx: &mut App,
        facts: nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged,
    ) {
        let mut state = cx
            .try_global::<UiMaterialCapabilities>()
            .copied()
            .unwrap_or_default();
        state.connected();
        assert!(state.apply(facts));
        cx.set_global(state);
    }

    fn publish_facts(
        cx: &mut gpui_kit::TestAppContext,
        facts: nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged,
    ) {
        cx.update(|cx| apply_facts(cx, facts));
    }

    fn material_ok(
        command: nxtools_protocol::window_material::ApplyWindowMaterialCommand,
    ) -> PlatformCommandResult {
        use nxtools_protocol::window_material::{
            AppliedWindowMaterial, EffectiveWindowMaterial, MaterialFallbackReason,
            WindowMaterialStyle,
        };
        let applied = AppliedWindowMaterial {
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
            data: Some(serde_json::to_string(&applied).expect("qualified material receipt")),
            ..ok(None)
        }
    }

    fn material_fixture(
        cx: &mut gpui_kit::TestAppContext,
    ) -> (
        tempfile::TempDir,
        gpui_kit::Entity<QuickLaunchApplication>,
        nxtools_ipc::PlatformServerEndpoint,
    ) {
        use gpui_kit::AppContext as _;
        use std::{cell::RefCell, rc::Rc};
        cx.update(gpui_kit::init);
        cx.update(|cx| cx.set_global(UiMaterialCapabilities::default()));
        cx.executor().allow_parking();
        let tmp = tempfile::tempdir().expect("isolated primary popup");
        let catalog = nxtools_finder::FinderCatalog::open(tmp.path().join("history.db"), &[])
            .expect("fixture catalog");
        let (platform, endpoint) = nxtools_ipc::platform_bridge(32);
        endpoint.set_connected_for_contract_test(true);
        let (indexer, _endpoint) = crate::runtime::indexer_bridge(8);
        let notifications = cx.new(|_| crate::notifications::NotificationCenter::default());
        let app = cx.new(|_| {
            QuickLaunchApplication::new(
                catalog,
                Rc::new(platform),
                Rc::new(indexer),
                nxtools_core::AppConfig::default(),
                tmp.path().join("config.json"),
                tmp.path().join("logs"),
                notifications,
            )
        });
        let slot = Rc::new(RefCell::new(None));
        let capture = slot.clone();
        let owner = app.downgrade();
        let _ = cx.add_window_view(move |window, cx| {
            let panel = cx.new(|cx| {
                super::super::panel::QuickLaunchPanel::new(
                    vec![],
                    nxtools_protocol::ForegroundContext {
                        window_handle: 123,
                        focus_window_handle: 124,
                        process_id: 1,
                        process_name: "fixture".into(),
                        window_title: "fixture".into(),
                    },
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
                native_window_handle: HWND,
            });
            gpui_kit::base::Root::new(panel, window, cx)
        });
        app.update(cx, |app, cx| {
            app.popup = slot.borrow_mut().take();
            app.active_summon = Some((1, 1));
            app.begin_popup_presentation(target(120), cx);
        });
        cx.run_until_parked();
        (tmp, app, endpoint)
    }

    fn deliver(
        app: &gpui_kit::Entity<QuickLaunchApplication>,
        cx: &mut gpui_kit::TestAppContext,
        id: Uuid,
        result: PlatformCommandResult,
    ) {
        app.update(cx, |app, cx| {
            app.handle_platform_notification(
                nxtools_ipc::PlatformNotification::CommandResult {
                    correlation_id: id,
                    result,
                },
                cx,
            )
        });
    }

    fn route(
        app: &gpui_kit::Entity<QuickLaunchApplication>,
        endpoint: &nxtools_ipc::PlatformServerEndpoint,
        cx: &mut gpui_kit::TestAppContext,
        id: Uuid,
        result: PlatformCommandResult,
    ) {
        route_then(app, endpoint, cx, id, result, |_, _| {});
    }

    fn route_then(
        app: &gpui_kit::Entity<QuickLaunchApplication>,
        endpoint: &nxtools_ipc::PlatformServerEndpoint,
        cx: &mut gpui_kit::TestAppContext,
        id: Uuid,
        result: PlatformCommandResult,
        after: impl FnOnce(&mut QuickLaunchApplication, &mut Context<QuickLaunchApplication>),
    ) {
        endpoint
            .route_command_result_for_contract_test(id, result)
            .expect("native terminal");
        app.update(cx, |app, cx| {
            let notification = app
                .platform
                .notification_receiver()
                .try_recv()
                .expect("original result notification");
            app.handle_platform_notification(notification, cx);
            after(app, cx);
        });
    }

    fn reach_material_barrier(
        app: &gpui_kit::Entity<QuickLaunchApplication>,
        endpoint: &nxtools_ipc::PlatformServerEndpoint,
        cx: &mut gpui_kit::TestAppContext,
    ) {
        for step in [
            Step::Hide,
            Step::StageMove,
            Step::StageSize,
            Step::StageCheck,
        ] {
            let command = endpoint
                .try_receive_command_for_contract_test()
                .expect("staging command");
            assert!(!matches!(
                command.message,
                Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                    SetWindowVisibilityCommand { visible: true, .. }
                ))
            ));
            let data =
                (step == Step::StageCheck).then(|| geometry(target(120), -30_000, -30_000, 0));
            route(app, endpoint, cx, command.id, ok(data));
            cx.run_until_parked();
        }
    }

    #[gpui_kit::test]
    fn stage3_primary_popup_waits_for_facts_and_receipt_before_first_native_show(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        let (_tmp, app, endpoint) = material_fixture(cx);
        reach_material_barrier(&app, &endpoint, cx);
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "no facts, no show"
        );
        publish_facts(cx, material_facts());
        cx.run_until_parked();
        let material = endpoint
            .try_receive_command_for_contract_test()
            .expect("material before show");
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) =
            material.message
        else {
            panic!("first native show requires material preparation");
        };
        assert_eq!(command.window_handle, HWND, "never the source/foreign HWND");
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
        deliver(&app, cx, Uuid::new_v4(), material_ok(command));
        cx.run_until_parked();
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "unowned result cannot advance"
        );
        route_then(
            &app,
            &endpoint,
            cx,
            material.id,
            material_ok(command),
            |_, _| {
                assert!(
                    endpoint.try_receive_command_for_contract_test().is_err(),
                    "receipt continuation is deferred"
                );
            },
        );
        cx.run_until_parked();
        let show = endpoint
            .try_receive_command_for_contract_test()
            .expect("qualified offscreen show");
        assert!(matches!(
            show.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle: HWND,
                    visible: true
                }
            ))
        ));
        deliver(&app, cx, material.id, material_ok(command));
        cx.run_until_parked();
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "receipt advances exactly once"
        );
    }

    #[gpui_kit::test]
    fn stage3_primary_popup_latest_facts_retire_unsent_material(cx: &mut gpui_kit::TestAppContext) {
        let (_tmp, app, endpoint) = material_fixture(cx);
        let facts = material_facts();
        publish_facts(cx, facts);
        reach_material_barrier(&app, &endpoint, cx);
        let original = app.read_with(cx, |app, _| {
            let pending = app
                .popup_presentation
                .as_ref()
                .expect("pending presentation");
            (pending.op, pending.started, pending.identity)
        });
        publish_facts(
            cx,
            nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged {
                revision: 2,
                transparency_enabled: false,
                ..facts
            },
        );
        cx.run_until_parked();
        let material = endpoint
            .try_receive_command_for_contract_test()
            .expect("latest material only");
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) =
            material.message
        else {
            panic!("facts must not reveal the popup");
        };
        assert_eq!(command.facts_revision, 2);
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "superseded queued request retired"
        );
        app.read_with(cx, |app, _| {
            let pending = app.popup_presentation.as_ref().expect("same presentation");
            assert_eq!((pending.op, pending.started, pending.identity), original);
        });
        let old = nxtools_protocol::window_material::ApplyWindowMaterialCommand {
            facts_revision: 1,
            ..command
        };
        deliver(&app, cx, material.id, material_ok(old));
        cx.run_until_parked();
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            assert!(!matches!(
                envelope.message,
                Message::PlatformCommand(
                    PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                        visible: true,
                        ..
                    }) | PlatformCommand::ActivateWindow(_)
                )
            ));
        }
    }

    #[gpui_kit::test]
    fn stage3_primary_popup_terminal_paths_retire_queued_material_and_late_results(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        for outcome in ["timeout", "dismiss", "disconnect"] {
            let (_tmp, app, endpoint) = material_fixture(cx);
            let facts = material_facts();
            publish_facts(cx, facts);
            reach_material_barrier(&app, &endpoint, cx);
            let generation = app.read_with(cx, |app, _| {
                app.platform.connection_generation().expect("connected")
            });
            if outcome == "timeout" {
                cx.executor().advance_clock(PRESENTATION_TIMEOUT);
                cx.run_until_parked();
            } else {
                app.update(cx, |app, cx| match outcome {
                    "dismiss" => app.hide_popup(cx),
                    _ => app.handle_platform_notification(
                        nxtools_ipc::PlatformNotification::Disconnected {
                            generation,
                            pending_command_ids: vec![],
                        },
                        cx,
                    ),
                });
            }
            publish_facts(
                cx,
                nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged {
                    revision: 2,
                    ..facts
                },
            );
            cx.run_until_parked();
            app.read_with(cx, |app, _| {
                assert!(app.popup_presentation.is_none());
                assert!(app.pending_window_activation.is_empty());
            });
            while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
                assert!(
                    !matches!(
                        envelope.message,
                        Message::PlatformCommand(
                            PlatformCommand::ApplyWindowMaterial(_)
                                | PlatformCommand::ActivateWindow(_)
                                | PlatformCommand::SetWindowVisibility(
                                    SetWindowVisibilityCommand { visible: true, .. }
                                )
                        )
                    ),
                    "{outcome} must retire unsent material and never show"
                );
            }
        }
    }

    #[gpui_kit::test]
    fn stage3_primary_popup_rechecks_receipt_after_deferred_facts_and_theme_changes(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        let (_tmp, app, endpoint) = material_fixture(cx);
        let facts = material_facts();
        publish_facts(cx, facts);
        reach_material_barrier(&app, &endpoint, cx);
        let first = endpoint
            .try_receive_command_for_contract_test()
            .expect("first material");
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(old)) = first.message
        else {
            panic!("material barrier");
        };
        route_then(&app, &endpoint, cx, first.id, material_ok(old), |_, cx| {
            apply_facts(
                cx,
                nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged {
                    revision: 2,
                    ..facts
                },
            );
            let mut theme = cx
                .try_global::<crate::theme::UiThemeState>()
                .copied()
                .unwrap_or_default();
            theme.material_style = nxtools_core::MaterialStyle::Acrylic;
            theme.preference = nxtools_core::ThemePreference::Light;
            cx.set_global(theme);
        });
        cx.run_until_parked();
        let latest = endpoint
            .try_receive_command_for_contract_test()
            .expect("latest material before show");
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(current)) =
            latest.message
        else {
            panic!("an accepted but outdated receipt cannot reveal");
        };
        assert_eq!(current.facts_revision, 2);
        assert_eq!(
            current.style,
            nxtools_protocol::window_material::WindowMaterialStyle::Acrylic
        );
        assert!(!current.dark);
        assert!(current.generation > old.generation);
        deliver(&app, cx, first.id, material_ok(old));
        cx.run_until_parked();
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "late old result cannot advance"
        );
        route(&app, &endpoint, cx, latest.id, material_ok(current));
        cx.run_until_parked();
        let show = endpoint
            .try_receive_command_for_contract_test()
            .expect("latest receipt reveals once");
        assert!(matches!(
            show.message,
            Message::PlatformCommand(PlatformCommand::SetWindowVisibility(
                SetWindowVisibilityCommand {
                    window_handle: HWND,
                    visible: true
                }
            ))
        ));
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
    }

    #[gpui_kit::test]
    fn stage3_primary_popup_native_stale_facts_waits_without_retry_loop(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        let (_tmp, app, endpoint) = material_fixture(cx);
        let facts = material_facts();
        publish_facts(cx, facts);
        reach_material_barrier(&app, &endpoint, cx);
        let first = endpoint
            .try_receive_command_for_contract_test()
            .expect("first material");
        route(
            &app,
            &endpoint,
            cx,
            first.id,
            PlatformCommandResult {
                success: false,
                error: Some("stale_material_facts".into()),
                ..ok(None)
            },
        );
        cx.run_until_parked();
        assert!(
            endpoint.try_receive_command_for_contract_test().is_err(),
            "same facts cannot retry or show"
        );
        publish_facts(
            cx,
            nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged {
                revision: 2,
                ..facts
            },
        );
        cx.run_until_parked();
        let latest = endpoint
            .try_receive_command_for_contract_test()
            .expect("new facts resume");
        assert!(matches!(
            latest.message,
            Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(
                nxtools_protocol::window_material::ApplyWindowMaterialCommand {
                    facts_revision: 2,
                    ..
                }
            ))
        ));
        assert!(endpoint.try_receive_command_for_contract_test().is_err());
    }

    #[gpui_kit::test]
    fn stage3_primary_popup_dismiss_before_material_continuation_cannot_show(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        let (_tmp, app, endpoint) = material_fixture(cx);
        publish_facts(cx, material_facts());
        reach_material_barrier(&app, &endpoint, cx);
        let material = endpoint
            .try_receive_command_for_contract_test()
            .expect("material request");
        let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) =
            material.message
        else {
            panic!("material barrier");
        };
        route_then(
            &app,
            &endpoint,
            cx,
            material.id,
            material_ok(command),
            |app, cx| {
                app.hide_popup(cx);
            },
        );
        deliver(&app, cx, material.id, material_ok(command));
        cx.run_until_parked();
        app.read_with(cx, |app, _| {
            assert!(app.popup_presentation.is_none());
            assert!(app.pending_window_activation.is_empty());
        });
        while let Ok(envelope) = endpoint.try_receive_command_for_contract_test() {
            assert!(!matches!(
                envelope.message,
                Message::PlatformCommand(
                    PlatformCommand::ApplyWindowMaterial(_)
                        | PlatformCommand::ActivateWindow(_)
                        | PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                            visible: true,
                            ..
                        })
                )
            ));
        }
    }
}
