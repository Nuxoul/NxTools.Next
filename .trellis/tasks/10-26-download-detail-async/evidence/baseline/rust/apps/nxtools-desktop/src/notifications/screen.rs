//! One passive screen popup over the existing NotificationCenter, never a second queue.
use super::*;
use gpui_kit::{
    AppContext, Bounds, IntoElement, Render, Subscription, WindowBackgroundAppearance,
    WindowBounds, WindowKind, WindowOptions, div, point, prelude::*, px, size,
};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{
    PlatformCommand, PrepareNotificationWindowCommand, SetWindowVisibilityCommand,
};
use std::time::Instant;
use uuid::Uuid;

struct ScreenGlobal(Entity<ScreenHost>);
impl Global for ScreenGlobal {}
pub(crate) fn attached(cx: &App) -> bool {
    cx.try_global::<ScreenGlobal>().is_some()
}
pub(crate) fn attach(
    center: Entity<NotificationCenter>,
    platform: Rc<PlatformBridge>,
    source: AnyWindowHandle,
    cx: &mut App,
) {
    let host = cx.new(|cx| ScreenHost::new(center, platform, source, cx));
    cx.set_global(ScreenGlobal(host.clone()));
    host.update(cx, |host, cx| host.sync(cx));
}
pub(crate) fn handle(notification: PlatformNotification, cx: &mut App) {
    let Some(host) = cx
        .try_global::<ScreenGlobal>()
        .map(|global| global.0.downgrade())
    else {
        return;
    };
    cx.defer(move |cx| {
        let _ = host.update(cx, |host, cx| host.handle(notification, cx));
    });
}

pub(crate) fn owns_native_window(handle: i64, cx: &App) -> bool {
    handle != 0
        && cx.try_global::<ScreenGlobal>().is_some_and(|global| {
            global
                .0
                .read(cx)
                .surface
                .as_ref()
                .is_some_and(|surface| surface.hwnd == handle)
        })
}

#[derive(Clone, PartialEq)]
struct Projection {
    items: Vec<(SharedString, u64, bool)>,
    position: NotificationPosition,
    source: AnyWindowHandle,
}
#[derive(Clone, Copy, PartialEq)]
enum Phase {
    Prepare,
    Adjust,
    Frame,
    Reveal,
    Hide,
}
struct Pending {
    id: Uuid,
    phase: Phase,
    started: Instant,
}
struct Surface {
    window: AnyWindowHandle,
    view: Entity<ScreenView>,
    hwnd: i64,
}
struct ScreenHost {
    center: Entity<NotificationCenter>,
    platform: Rc<PlatformBridge>,
    default_source: AnyWindowHandle,
    surface: Option<Surface>,
    projection: Option<Projection>,
    pending: Option<Pending>,
    dirty: bool,
    visible: bool,
    measured_height: Option<f32>,
    queued_height: Option<f32>,
    _center: Subscription,
}
impl ScreenHost {
    fn new(
        center: Entity<NotificationCenter>,
        platform: Rc<PlatformBridge>,
        source: AnyWindowHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        let subscription = cx.observe(&center, |_, _, cx| {
            let host = cx.entity().downgrade();
            cx.defer(move |cx| {
                let _ = host.update(cx, |host, cx| host.changed(cx));
            });
        });
        Self {
            center,
            platform,
            default_source: source,
            surface: None,
            projection: None,
            pending: None,
            dirty: false,
            visible: false,
            measured_height: None,
            queued_height: None,
            _center: subscription,
        }
    }
    fn snapshot(&self, cx: &App) -> Option<Projection> {
        let center = self.center.read(cx);
        let items = center.items().iter().collect::<Vec<_>>();
        let items = items
            .into_iter()
            .filter(|item| {
                item.target
                    .is_none_or(|source| cx.windows().contains(&source))
            })
            .collect::<Vec<_>>();
        let first = items.first()?;
        Some(Projection {
            items: items
                .iter()
                .map(|item| (item.id.clone(), item.generation(), item.exiting))
                .collect(),
            position: center.position(),
            source: first.target.unwrap_or(self.default_source),
        })
    }
    fn changed(&mut self, cx: &mut Context<Self>) {
        let next = self.snapshot(cx);
        if next != self.projection
            && self.visible
            && self
                .pending
                .as_ref()
                .is_none_or(|pending| pending.phase != Phase::Hide)
            && next
                .as_ref()
                .zip(self.projection.as_ref())
                .is_some_and(|(next, old)| {
                    next.position == old.position && next.source == old.source
                })
        {
            // Exit and content updates repaint the live HWND. Native hide/reprepare/reveal
            // here restarts animations and produces the installed flicker.
            self.projection = next;
            self.dirty = false;
            if let Some(surface) = self.surface.as_ref() {
                let view = surface.view.clone();
                let surface_window = surface.window;
                cx.defer(move |cx| {
                    let _ = surface_window.update(cx, |_, window, cx| {
                        view.update(cx, |_, cx| cx.notify());
                        window.refresh();
                    });
                });
            }
            return;
        }
        let next = self.snapshot(cx);
        if next == self.projection {
            return;
        }
        self.dirty = true;
        self.measured_height = None;
        if self.pending.is_none() {
            self.sync(cx);
        }
    }
    fn sync(&mut self, cx: &mut Context<Self>) {
        if self.pending.is_some() {
            self.dirty = true;
            return;
        }
        self.dirty = false;
        let next = self.snapshot(cx);
        if next.is_none() {
            self.projection = None;
            if self.visible {
                self.send_visibility(false, cx);
            }
            return;
        }
        let next = next.expect("nonempty projection");
        if self.visible {
            self.dirty = true;
            self.send_visibility(false, cx);
            return;
        }
        if self.surface.is_none() {
            match open_surface(
                self.center.clone(),
                self.default_source,
                next.source,
                cx.entity().downgrade(),
                cx,
            ) {
                Ok(surface) => self.surface = Some(surface),
                Err(_) => {
                    tracing::warn!(
                        event = "ui.notification.surface.failed",
                        reason = "create",
                        "notification popup creation failed"
                    );
                    return;
                }
            }
        }
        let surface = self.surface.as_ref().expect("created surface");
        let hwnd = surface.hwnd;
        let source = next.source;
        let source_hwnd = source
            .update(cx, |_, window, _| crate::ui::native_window_handle(window))
            .ok()
            .flatten();
        #[cfg(test)]
        let source_hwnd = source_hwnd.or(Some(-1));
        let Some(source_hwnd) = source_hwnd else {
            self.retire("source_unavailable", cx);
            return;
        };
        let height = self.measured_height.unwrap_or(
            24.0 + next.items.len() as f32 * 112.0
                + next.items.len().saturating_sub(1) as f32 * 8.0,
        );
        let command =
            PlatformCommand::PrepareNotificationWindow(PrepareNotificationWindowCommand {
                window_handle: hwnd,
                source_window_handle: source_hwnd,
                source_process_id: std::process::id(),
                position: position_name(next.position).into(),
                width: 360,
                height: height.ceil() as i32,
                keep_visible: false,
            });
        self.projection = Some(next);
        self.visible = false; // Prepare requires the target to be natively hidden.
        self.send(command, Phase::Prepare, cx);
    }
    fn send(&mut self, command: PlatformCommand, phase: Phase, cx: &mut Context<Self>) {
        let id = Uuid::new_v4();
        self.pending = Some(Pending {
            id,
            phase,
            started: Instant::now(),
        });
        if self.platform.send_command_with_id(command, id).is_err() {
            self.retire("dispatch", cx);
            return;
        }
        tracing::info!(event="ui.notification.surface.accepted",correlation_id=%id,phase=phase_name(phase),"notification popup operation admitted");
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(Duration::from_secs(5)).await;
            let _ = this.update(cx, |host, cx| {
                if host
                    .pending
                    .as_ref()
                    .is_some_and(|pending| pending.id == id)
                {
                    host.retire("timeout", cx);
                }
            });
        })
        .detach();
    }
    fn send_visibility(&mut self, visible: bool, cx: &mut Context<Self>) {
        let Some(surface) = self.surface.as_ref() else {
            return;
        };
        self.send(
            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                window_handle: surface.hwnd,
                visible,
            }),
            if visible { Phase::Reveal } else { Phase::Hide },
            cx,
        );
    }
    fn handle(&mut self, notification: PlatformNotification, cx: &mut Context<Self>) {
        if matches!(notification, PlatformNotification::Disconnected { .. }) {
            self.retire("disconnected", cx);
            return;
        }
        if matches!(notification, PlatformNotification::Connected) {
            self.sync(cx);
            return;
        }
        let PlatformNotification::CommandResult {
            correlation_id,
            result,
        } = notification
        else {
            return;
        };
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.id != correlation_id)
        {
            return;
        }
        let pending = self.pending.take().expect("matched pending");
        tracing::info!(event="ui.notification.surface.terminal",correlation_id=%correlation_id,phase=phase_name(pending.phase),success=result.success,native_error_code=?result.native_error_code,elapsed_ms=pending.started.elapsed().as_millis(),"notification popup operation terminated");
        if !result.success {
            self.retire("native_failed", cx);
            return;
        }
        if self.dirty || self.snapshot(cx) != self.projection {
            self.visible = match pending.phase {
                Phase::Hide | Phase::Prepare => false,
                Phase::Reveal => true,
                _ => self.visible,
            };
            self.sync(cx);
            return;
        }
        match pending.phase {
            Phase::Prepare | Phase::Adjust => {
                let placement = result
                    .data
                    .as_deref()
                    .and_then(|data| serde_json::from_str::<Placement>(data).ok())
                    .filter(|p| p.width > 0 && p.height > 0 && (48..=960).contains(&p.dpi));
                let Some(placement) = placement else {
                    self.retire("invalid_readback", cx);
                    return;
                };
                self.pending = Some(Pending {
                    id: correlation_id,
                    phase: Phase::Frame,
                    started: pending.started,
                });
                if let Some(surface) = self.surface.as_ref() {
                    let view = surface.view.clone();
                    let surface_window = surface.window;
                    let host = cx.entity().downgrade();
                    cx.defer(move |cx| {
                        let _ = surface_window.update(cx, |_, window, cx| {
                            view.update(cx, |view, cx| {
                                view.expected_frame = Some((correlation_id, placement));
                                cx.notify();
                            });
                            // Hidden Windows HWNDs have no ordinary frame loop. Build the
                            // first scene explicitly after the host lease, before reveal.
                            window.refresh();
                            window.draw(cx).clear(cx);
                            if view.read(cx).expected_frame.is_none() {
                                cx.defer(move |cx| {
                                    let _ = host.update(cx, |host, cx| {
                                        host.frame_ready(correlation_id, cx)
                                    });
                                });
                            }
                        });
                    });
                }
            }
            Phase::Reveal => self.visible = true,
            Phase::Hide => self.visible = false,
            Phase::Frame => {}
        }
    }
    fn frame_ready(&mut self, id: Uuid, cx: &mut Context<Self>) {
        if self
            .pending
            .as_ref()
            .is_none_or(|pending| pending.id != id || pending.phase != Phase::Frame)
        {
            return;
        }
        self.pending = None;
        if self.dirty || self.snapshot(cx) != self.projection {
            self.sync(cx);
        } else if !self.visible {
            self.send_visibility(true, cx);
        } else if let Some(height) = self.queued_height.take() {
            self.measured(height, cx);
        }
    }
    fn measured(&mut self, height: f32, cx: &mut Context<Self>) {
        let height = height.ceil().clamp(78.0, 400.0);
        if self.visible {
            if self.pending.is_some() {
                self.queued_height = Some(height);
                return;
            }
            if self
                .measured_height
                .is_some_and(|old| (old - height).abs() < 1.0)
            {
                return;
            }
            self.measured_height = Some(height);
            let Some(projection) = self.projection.as_ref() else {
                return;
            };
            let Some(surface) = self.surface.as_ref() else {
                return;
            };
            let source_hwnd = projection
                .source
                .update(cx, |_, window, _| crate::ui::native_window_handle(window))
                .ok()
                .flatten();
            #[cfg(test)]
            let source_hwnd = source_hwnd.or(Some(-1));
            let Some(source_hwnd) = source_hwnd else {
                self.retire("source_unavailable", cx);
                return;
            };
            self.send(
                PlatformCommand::PrepareNotificationWindow(PrepareNotificationWindowCommand {
                    window_handle: surface.hwnd,
                    source_window_handle: source_hwnd,
                    source_process_id: std::process::id(),
                    position: position_name(projection.position).into(),
                    width: 360,
                    height: height as i32,
                    keep_visible: true,
                }),
                Phase::Adjust,
                cx,
            );
            return;
        }
        if self.projection.is_none()
            || self
                .measured_height
                .is_some_and(|old| (old - height).abs() < 1.0)
        {
            return;
        }
        self.measured_height = Some(height);
        self.dirty = true;
        if self.pending.is_none() {
            self.sync(cx);
        }
    }
    fn retire(&mut self, reason: &'static str, cx: &mut Context<Self>) {
        self.pending = None;
        self.visible = false;
        self.projection = None;
        self.dirty = false;
        self.measured_height = None;
        self.queued_height = None;
        if let Some(surface) = self.surface.take() {
            // Destroy on failure: a late native reveal cannot resurrect an old HWND.
            cx.defer(move |cx| {
                let _ = surface
                    .window
                    .update(cx, |_, window, _| window.remove_window());
            });
        }
        tracing::warn!(
            event = "ui.notification.surface.failed",
            reason,
            "notification popup retired"
        );
    }
}
#[derive(Clone, Copy, serde::Deserialize)]
struct Placement {
    width: i32,
    height: i32,
    dpi: u32,
}
fn position_name(position: NotificationPosition) -> &'static str {
    match position {
        NotificationPosition::TopLeft => "top_left",
        NotificationPosition::TopCenter => "top_center",
        NotificationPosition::TopRight => "top_right",
        NotificationPosition::BottomLeft => "bottom_left",
        NotificationPosition::BottomCenter => "bottom_center",
        NotificationPosition::BottomRight => "bottom_right",
    }
}
fn phase_name(phase: Phase) -> &'static str {
    match phase {
        Phase::Prepare => "prepare",
        Phase::Adjust => "adjust",
        Phase::Frame => "frame",
        Phase::Reveal => "reveal",
        Phase::Hide => "hide",
    }
}
struct ScreenView {
    center: Entity<NotificationCenter>,
    source: AnyWindowHandle,
    host: WeakEntity<ScreenHost>,
    expected_frame: Option<(Uuid, Placement)>,
}
impl Render for ScreenView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::theme::observe_theme(window, cx);
        if let Some((_, placement)) = self.expected_frame {
            let scale = window.scale_factor();
            let viewport = window.viewport_size();
            if (viewport.width.as_f32() * scale - placement.width as f32).abs() <= 1.0
                && (viewport.height.as_f32() * scale - placement.height as f32).abs() <= 1.0
            {
                self.expected_frame = None;
                // Scene readiness is consumed by the caller after draw returns.
            }
        }
        let host = self.host.clone();
        div()
            .size_full()
            .relative()
            .child(super::presentation::render_screen(
                self.center.clone(),
                self.source,
                Rc::new(move |height, cx| {
                    let _ = host.update(cx, |host, cx| host.measured(height, cx));
                }),
                window,
                cx,
            ))
            .children({
                #[cfg(debug_assertions)]
                let observer = Some(
                    gpui_kit::canvas(|_, _, _| {}, |_, _, _, cx| preview_cadence::paint(cx))
                        .absolute()
                        .size_full(),
                );
                #[cfg(not(debug_assertions))]
                let observer: Option<gpui_kit::Div> = None;
                observer
            })
    }
}
fn notification_window_options(display_id: Option<gpui_kit::DisplayId>) -> WindowOptions {
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(0.0), px(0.0)),
            size(px(360.0), px(376.0)),
        ))),
        titlebar: None,
        kind: WindowKind::PopUp,
        show: false,
        focus: false,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        display_id,
        window_background: WindowBackgroundAppearance::Transparent,
        // This HWND is deliberately never active; do not cap its one-shot Toast animations to 30fps.
        inactive_frame_interval: None,
        ..Default::default()
    }
}

fn open_surface(
    center: Entity<NotificationCenter>,
    default_source: AnyWindowHandle,
    source: AnyWindowHandle,
    host: WeakEntity<ScreenHost>,
    cx: &mut App,
) -> anyhow::Result<Surface> {
    let display_id = source
        .update(cx, |_, window, cx| {
            window.display(cx).map(|display| display.id())
        })
        .ok()
        .flatten();
    let mut view = None;
    let mut hwnd: Option<i64> = None;
    let options = notification_window_options(display_id);
    #[cfg(debug_assertions)]
    let options = preview_cadence::options(options, cx);
    let window: AnyWindowHandle = cx
        .open_window(options, |window, cx| {
            window.set_window_title("NxTools.Notification");
            hwnd = crate::ui::native_window_handle(window);
            #[cfg(test)]
            if hwnd.is_none() {
                hwnd = Some(-2);
            }
            let content = cx.new(|_| ScreenView {
                center,
                source: default_source,
                host,
                expected_frame: None,
            });
            view = Some(content.clone());
            cx.new(|cx| {
                gpui_kit::base::Root::new(content, window, cx).bg(gpui_kit::transparent_black())
            })
        })?
        .into();
    match (view, hwnd) {
        (Some(view), Some(hwnd)) => Ok(Surface { window, view, hwnd }),
        _ => {
            let _: anyhow::Result<()> = window.update(cx, |_, window, _| window.remove_window());
            anyhow::bail!("notification HWND unavailable")
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(debug_assertions)]
pub(crate) mod preview;

#[cfg(debug_assertions)]
mod preview_cadence;
