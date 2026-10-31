//! One styled, independently owned GPUI menu session. C# only prepares native placement.
use super::{
    project_menu::{Handler, MenuItem},
    styled_menu_surface::{self, Surface},
};
use gpui_kit::{
    AnyElement, AnyWindowHandle, App, ClickEvent, FocusHandle, Global, Pixels, Point, Window,
    canvas, div, point, prelude::*, px,
};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use nxtools_protocol::{
    ActivateWindowCommand, CancelContextMenuCommand, PlatformCommand, PlatformEvent,
    SetWindowVisibilityCommand, ShowContextMenuCommand, StyledMenuPlacement,
};
use std::{
    cell::Cell,
    collections::HashMap,
    rc::{Rc, Weak},
    time::Instant,
};
use uuid::Uuid;

#[derive(Default)]
struct NativeMenuState {
    pending: Option<Pending>,
    source_dismiss: Option<(AnyWindowHandle, Instant)>,
}
impl Global for NativeMenuState {}
struct Pending {
    id: Uuid,
    window: AnyWindowHandle,
    command_id: Uuid,
    source_hwnd: i64,
    source_session: Option<Uuid>,
    trigger: Weak<Cell<bool>>,
    callbacks: HashMap<u32, Handler>,
    previous_focus: Option<FocusHandle>,
    platform: Rc<PlatformBridge>,
    surface: Option<Surface>,
    pointer_inside_source: bool,
    source_bounds: gpui_kit::Bounds<gpui_kit::DevicePixels>,
    pointer: bool,
    ready: bool,
    step: u8,
    started: Instant,
}
pub(super) fn init(cx: &mut App) {
    cx.set_global(NativeMenuState::default());
}
pub(crate) fn active(window: &Window, cx: &App) -> bool {
    cx.try_global::<NativeMenuState>()
        .and_then(|state| state.pending.as_ref())
        .is_some_and(|pending| pending.window == window.window_handle())
}
pub(crate) fn any_active(cx: &App) -> bool {
    cx.try_global::<NativeMenuState>()
        .is_some_and(|state| state.pending.is_some())
}
pub(crate) fn consume_source_press(window: &Window, cx: &mut App) -> bool {
    let source = window.window_handle();
    let pending = cx
        .try_global::<NativeMenuState>()
        .and_then(|state| state.pending.as_ref())
        .filter(|pending| pending.window == source)
        .map(|pending| pending.id);
    if let Some(id) = pending {
        finish(id, None, false, "source_pointer", cx);
        return true;
    }
    cx.try_global::<NativeMenuState>()
        .and_then(|state| state.source_dismiss.as_ref())
        .is_some_and(|(window, started)| {
            *window == source && started.elapsed() < std::time::Duration::from_millis(500)
        })
}
pub(crate) fn consume_source_release(window: &Window, cx: &mut App) -> bool {
    let source = window.window_handle();
    if cx
        .try_global::<NativeMenuState>()
        .and_then(|state| state.source_dismiss.as_ref())
        .is_some_and(|(window, started)| {
            *window == source && started.elapsed() < std::time::Duration::from_millis(500)
        })
    {
        cx.global_mut::<NativeMenuState>().source_dismiss = None;
        return true;
    }
    false
}
pub(super) fn surface_session(window: &Window, cx: &App) -> Option<Uuid> {
    cx.try_global::<NativeMenuState>()
        .and_then(|state| state.pending.as_ref())
        .filter(|pending| {
            pending
                .surface
                .as_ref()
                .is_some_and(|surface| surface.window == window.window_handle())
        })
        .map(|pending| pending.id)
}
pub(super) fn mark_pointer_position(
    id: Uuid,
    position: Point<Pixels>,
    window: &Window,
    cx: &mut App,
) {
    let screen = window.bounds().origin + position;
    let screen = point(
        gpui_kit::DevicePixels((screen.x.as_f32() * window.scale_factor()).round() as i32),
        gpui_kit::DevicePixels((screen.y.as_f32() * window.scale_factor()).round() as i32),
    );
    if let Some(pending) = cx
        .global_mut::<NativeMenuState>()
        .pending
        .as_mut()
        .filter(|pending| pending.id == id)
    {
        pending.pointer = true;
        pending.pointer_inside_source = pending.source_bounds.contains(&screen);
    }
}
pub(super) fn mark_keyboard(id: Uuid, cx: &mut App) {
    if let Some(pending) = cx
        .global_mut::<NativeMenuState>()
        .pending
        .as_mut()
        .filter(|pending| pending.id == id)
    {
        pending.pointer = false;
    }
}
fn bind_branch(
    items: &mut [MenuItem],
    id: Uuid,
    next: &mut u32,
    callbacks: &mut HashMap<u32, Handler>,
    ancestor_enabled: bool,
) {
    for item in items {
        let enabled = ancestor_enabled && item.eligible();
        item.disabled = !enabled;
        bind_branch(&mut item.children, id, next, callbacks, enabled);
        if item.separator || !item.children.is_empty() {
            item.handler = None;
            continue;
        }
        *next += 1;
        let leaf = *next;
        let handler = item.handler.take();
        if enabled && let Some(handler) = handler {
            callbacks.insert(leaf, handler);
            item.handler = Some(Rc::new(move |_, _, cx| {
                // The official menu entity is still leased. Never call source business code here.
                cx.defer(move |cx| finish(id, Some(leaf), false, "selected", cx));
            }));
        }
    }
}
fn report_failure(
    source: AnyWindowHandle,
    source_session: Option<Uuid>,
    message: &'static str,
    cx: &mut App,
) {
    if let Some(application) = cx
        .try_global::<crate::clipboard::ClipboardApplicationGlobal>()
        .map(|global| global.application())
    {
        cx.defer(move |cx| {
            application.update(cx, |app, cx| {
                app.context_menu_failed(source, source_session, message, cx)
            })
        });
    }
}
pub(super) fn open(
    items: Vec<MenuItem>,
    trigger: Weak<Cell<bool>>,
    position: Point<Pixels>,
    previous_focus: Option<FocusHandle>,
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let Some(global) = cx.try_global::<crate::clipboard::ClipboardApplicationGlobal>() else {
        return false;
    };
    let platform = global.context_menu_platform(cx);
    let source_session = global.context_menu_source_session(window.window_handle(), cx);
    open_with_source(
        items,
        trigger,
        position,
        previous_focus,
        (platform, source_session),
        window,
        cx,
    )
}
fn open_with_source(
    mut items: Vec<MenuItem>,
    trigger: Weak<Cell<bool>>,
    position: Point<Pixels>,
    previous_focus: Option<FocusHandle>,
    (platform, source_session): (Rc<PlatformBridge>, Option<Uuid>),
    window: &mut Window,
    cx: &mut App,
) -> bool {
    let Some(hwnd) = crate::ui::native_window_handle(window) else {
        report_failure(
            window.window_handle(),
            source_session,
            "菜单窗口当前不可用，请重试。",
            cx,
        );
        return true;
    };
    if cx.try_global::<NativeMenuState>().is_none() {
        init(cx);
    }
    dismiss_all(cx);
    if items.is_empty() {
        return true;
    }
    let id = Uuid::new_v4();
    let mut callbacks = HashMap::new();
    bind_branch(&mut items, id, &mut 0, &mut callbacks, true);
    // Install the source guard BEFORE opening a native window or crossing IPC.
    cx.global_mut::<NativeMenuState>().pending = Some(Pending {
        command_id: id,
        id,
        window: window.window_handle(),
        source_hwnd: hwnd,
        source_session,
        trigger,
        callbacks,
        previous_focus,
        platform: platform.clone(),
        surface: None,
        pointer: false,
        pointer_inside_source: false,
        source_bounds: window.bounds().to_device_pixels(window.scale_factor()),
        ready: false,
        step: 0,
        started: Instant::now(),
    });
    if let Some(application) = cx
        .try_global::<crate::clipboard::ClipboardApplicationGlobal>()
        .map(|global| global.application())
    {
        cx.defer(move |cx| {
            application.update(cx, |owner, cx| {
                owner.dismiss_preview_for_interaction("context_menu", cx)
            })
        });
    }
    tracing::info!(event="ui.menu.styled.accepted", correlation_id=%id, "styled context menu accepted");
    let surface = match styled_menu_surface::open(id, items, window, cx) {
        Ok(surface) => surface,
        Err(_) => {
            fail(id, "菜单窗口创建失败，请重试。", cx);
            return true;
        }
    };
    let popup_hwnd = surface.hwnd;
    cx.global_mut::<NativeMenuState>()
        .pending
        .as_mut()
        .expect("installed menu")
        .surface = Some(surface);
    let scale = window.scale_factor();
    let command = PlatformCommand::ShowContextMenu(ShowContextMenuCommand {
        source_window_handle: hwnd,
        source_process_id: std::process::id(),
        x: (position.x.as_f32() * scale).round() as i32,
        y: (position.y.as_f32() * scale).round() as i32,
        popup_window_handle: popup_hwnd,
        items: vec![],
    });
    if platform.send_command_with_id(command, id).is_err() {
        fail(id, "菜单暂时不可用，请重试。", cx);
    } else {
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(3))
                .await;
            cx.update(move |cx| {
                if cx
                    .try_global::<NativeMenuState>()
                    .and_then(|state| state.pending.as_ref())
                    .is_some_and(|p| p.id == id && !p.ready)
                {
                    fail(id, "菜单准备超时，已取消。", cx);
                }
            });
        })
        .detach();
    }
    true
}
fn fail(id: Uuid, message: &'static str, cx: &mut App) {
    let source = cx
        .try_global::<NativeMenuState>()
        .and_then(|state| state.pending.as_ref())
        .filter(|pending| pending.id == id)
        .map(|pending| (pending.window, pending.source_session));
    finish(id, None, false, "failed", cx);
    let source_session = source.and_then(|(_, session)| session);
    if let Some((source, _)) = source {
        report_failure(source, source_session, message, cx);
    }
}
pub(crate) fn dismiss_window(window: &Window, cx: &mut App) {
    if active(window, cx) {
        dismiss_all(cx);
    }
}
fn dismiss_all(cx: &mut App) {
    if let Some(id) = cx
        .try_global::<NativeMenuState>()
        .and_then(|state| state.pending.as_ref())
        .map(|pending| pending.id)
    {
        finish(id, None, false, "host_retired", cx);
    }
}
/// First matching terminal wins. Retire before all deferred popup/source/entity effects.
pub(super) fn finish(
    id: Uuid,
    selected: Option<u32>,
    outside: bool,
    reason: &'static str,
    cx: &mut App,
) {
    if cx
        .try_global::<NativeMenuState>()
        .and_then(|state| state.pending.as_ref())
        .is_none_or(|pending| pending.id != id)
    {
        return;
    }
    let pending = cx
        .global_mut::<NativeMenuState>()
        .pending
        .take()
        .expect("matching menu");
    let reason = if outside
        && reason == "deactivated"
        && pending
            .window
            .update(cx, |_, window, _| window.is_window_active())
            .unwrap_or(false)
    {
        "source_pointer"
    } else {
        reason
    };
    let reason = if selected.is_none()
        && pending.pointer
        && pending.pointer_inside_source
        && reason == "escape"
    {
        "source_overlay"
    } else {
        reason
    };
    let outside = outside && !matches!(reason, "source_pointer" | "source_overlay");
    if reason == "source_pointer" {
        cx.global_mut::<NativeMenuState>().source_dismiss = Some((pending.window, Instant::now()));
    }
    let outside = outside || (selected.is_none() && pending.pointer && reason == "escape");
    let restore = !outside
        && matches!(
            reason,
            "escape"
                | "selected"
                | "failed"
                | "trigger_retired"
                | "source_pointer"
                | "source_overlay"
        );
    let _ = pending
        .platform
        .send_command(PlatformCommand::CancelContextMenu(
            CancelContextMenuCommand { session_id: id },
        ));
    tracing::info!(event="ui.menu.styled.terminal", correlation_id=%id, reason, elapsed_ms=pending.started.elapsed().as_millis(), "styled context menu ended");
    cx.defer(move |cx| {
        if let Some(surface) = pending.surface {
            let _ = surface
                .window
                .update(cx, |_, window, _| window.remove_window());
        }
        let _ = pending.window.update(cx, |_, window, cx| {
            let live = pending.trigger.upgrade().is_some_and(|token| token.get());
            let session_current = cx
                .try_global::<crate::clipboard::ClipboardApplicationGlobal>()
                .is_none_or(|global| {
                    global.context_menu_source_session(pending.window, cx) == pending.source_session
                });
            if !session_current
                || cx.global::<NativeMenuState>().pending.is_some()
                || (!live && reason != "trigger_retired")
            {
                return;
            }
            if outside {
                if let Some(application) = cx
                    .try_global::<crate::clipboard::ClipboardApplicationGlobal>()
                    .map(|global| global.application())
                {
                    cx.defer(move |cx| {
                        application.update(cx, |app, cx| {
                            app.context_menu_outside(pending.window, pending.source_session, cx)
                        })
                    });
                }
                return;
            }
            if restore {
                let _ = pending
                    .platform
                    .send_command(PlatformCommand::ActivateWindow(ActivateWindowCommand {
                        window_handle: pending.source_hwnd,
                    }));
                if let Some(focus) = pending.previous_focus {
                    focus.focus(window, cx);
                }
            }
            if let Some(handler) = selected
                .and_then(|leaf| pending.callbacks.get(&leaf))
                .cloned()
            {
                let trigger = pending.trigger.clone();
                let source = pending.window;
                let session = pending.source_session;
                window.defer(cx, move |window, cx| {
                    let session_current = cx
                        .try_global::<crate::clipboard::ClipboardApplicationGlobal>()
                        .is_none_or(|global| {
                            global.context_menu_source_session(source, cx) == session
                        });
                    if session_current
                        && trigger.upgrade().is_some_and(|token| token.get())
                        && !active(window, cx)
                    {
                        handler(&ClickEvent::default(), window, cx);
                    }
                });
            }
        });
    });
}
pub(crate) fn handle_notification(notification: &PlatformNotification, cx: &mut App) {
    let Some(pending) = cx
        .try_global::<NativeMenuState>()
        .and_then(|state| state.pending.as_ref())
    else {
        return;
    };
    let id = pending.id;
    match notification {
        PlatformNotification::Connected | PlatformNotification::Disconnected { .. } => {
            dismiss_all(cx)
        }
        PlatformNotification::Event(PlatformEvent::GlobalPointerPressed(press))
            if pending.ready =>
        {
            let popup = pending.surface.as_ref().map(|surface| surface.hwnd);
            if Some(press.window_handle) != popup && Some(press.root_window_handle) != popup {
                let sampled = press.sampled_at_unix_ms;
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |time| time.as_millis() as i64);
                if sampled > 0
                    && now.saturating_sub(sampled) < pending.started.elapsed().as_millis() as i64
                {
                    let inside_source = press.window_handle == pending.source_hwnd
                        || press.root_window_handle == pending.source_hwnd;
                    cx.defer(move |cx| {
                        finish(
                            id,
                            None,
                            !inside_source,
                            if inside_source {
                                "source_pointer"
                            } else {
                                "outside_pointer"
                            },
                            cx,
                        )
                    });
                }
            }
        }
        PlatformNotification::CommandResult {
            correlation_id,
            result,
        } if *correlation_id == pending.command_id && !pending.ready => {
            let step = pending.step;
            if !result.success {
                tracing::warn!(event="ui.menu.styled.native_failed", correlation_id=%id, step, native_error_code=result.native_error_code, "styled menu presentation failed");
                fail(id, "菜单显示失败，请重试。", cx);
                return;
            }
            if step == 2 {
                let pending = cx
                    .global_mut::<NativeMenuState>()
                    .pending
                    .as_mut()
                    .expect("matching menu");
                pending.ready = true;
                tracing::info!(event="ui.menu.styled.ready", correlation_id=%id, elapsed_ms=pending.started.elapsed().as_millis(), "styled menu visible and active");
                return;
            }
            let placement = if step == 0 {
                let placement = result
                    .data
                    .as_deref()
                    .and_then(|data| serde_json::from_str::<StyledMenuPlacement>(data).ok());
                let Some(placement) = placement.filter(|p| {
                    p.width > 0
                        && p.height > 0
                        && p.anchor_x >= 0
                        && p.anchor_y >= 0
                        && p.anchor_x < p.width
                        && p.anchor_y < p.height
                }) else {
                    fail(id, "菜单位置准备失败，请重试。", cx);
                    return;
                };
                Some(placement)
            } else {
                None
            };
            // Unique phase IDs reject stale/duplicate results; session ID remains the diagnostic owner.
            let command_id = Uuid::new_v4();
            let pending = cx
                .global_mut::<NativeMenuState>()
                .pending
                .as_mut()
                .expect("matching menu");
            pending.step = step + 1;
            pending.command_id = command_id;
            cx.defer(move |cx| {
                let Some(pending) = cx
                    .global::<NativeMenuState>()
                    .pending
                    .as_ref()
                    .filter(|p| p.id == id && p.step == step + 1)
                else {
                    return;
                };
                if !pending.trigger.upgrade().is_some_and(|token| token.get()) {
                    finish(id, None, false, "trigger_retired", cx);
                    return;
                }
                let Some(surface) = pending.surface.as_ref() else {
                    return;
                };
                let handle = surface.window;
                let view = surface.view.clone();
                let hwnd = surface.hwnd;
                let platform = pending.platform.clone();
                let _ = handle.update(cx, |_, window, cx| {
                    if let Some(placement) = placement {
                        let scale = window.scale_factor();
                        let viewport = window.viewport_size();
                        if (viewport.width.as_f32() * scale - placement.width as f32).abs() > 1.0
                            || (viewport.height.as_f32() * scale - placement.height as f32).abs()
                                > 1.0
                        {
                            cx.defer(move |cx| fail(id, "菜单窗口尺寸尚未就绪，请重试。", cx));
                            return;
                        }
                        view.update(cx, |view, cx| {
                            view.place(
                                point(
                                    px(placement.anchor_x as f32 / scale),
                                    px(placement.anchor_y as f32 / scale),
                                ),
                                window,
                                cx,
                            )
                        });
                    }
                    let show = move |cx: &mut App| {
                        let Some(pending) = cx
                            .try_global::<NativeMenuState>()
                            .and_then(|state| state.pending.as_ref())
                            .filter(|pending| pending.id == id && pending.command_id == command_id)
                        else {
                            return;
                        };
                        if !pending.trigger.upgrade().is_some_and(|token| token.get()) {
                            finish(id, None, false, "trigger_retired", cx);
                            return;
                        }
                        if cx
                            .try_global::<crate::clipboard::ClipboardApplicationGlobal>()
                            .is_some_and(|global| {
                                global.context_menu_source_session(pending.window, cx)
                                    != pending.source_session
                            })
                        {
                            finish(id, None, false, "host_retired", cx);
                            return;
                        }
                        let command = if step == 0 {
                            PlatformCommand::SetWindowVisibility(SetWindowVisibilityCommand {
                                window_handle: hwnd,
                                visible: true,
                            })
                        } else {
                            PlatformCommand::ActivateWindow(ActivateWindowCommand {
                                window_handle: hwnd,
                            })
                        };
                        if platform.send_command_with_id(command, command_id).is_err() {
                            fail(id, "菜单显示失败，请重试。", cx);
                        }
                    };
                    if step == 0 {
                        // Hidden Windows HWNDs do not deliver the ordinary next-frame callback.
                        // Build the confirmed-size scene after the source/view leases, then reveal
                        // on another app turn; never show first or treat a timeout as readiness.
                        window.refresh();
                        window.draw(cx).clear(cx);
                        cx.defer(show);
                    } else {
                        show(cx);
                    }
                });
            });
        }
        _ => {}
    }
}
pub(super) fn prepare(window: &Window, cx: &mut App) {
    if let Some(pending) = cx
        .try_global::<NativeMenuState>()
        .and_then(|state| state.pending.as_ref())
        && pending.window == window.window_handle()
        && let Some(token) = pending.trigger.upgrade()
    {
        token.set(false);
    }
}
pub(super) fn guard(window: &Window, cx: &App) -> AnyElement {
    let Some((id, token)) = cx
        .try_global::<NativeMenuState>()
        .and_then(|state| state.pending.as_ref())
        .filter(|pending| pending.window == window.window_handle())
        .map(|pending| (pending.id, pending.trigger.clone()))
    else {
        return div().into_any_element();
    };
    canvas(
        |_, _, _| (),
        move |_, (), window, cx| {
            if !token.upgrade().is_some_and(|token| token.get()) {
                window.defer(cx, move |_, cx| {
                    finish(id, None, false, "trigger_retired", cx)
                });
            }
        },
    )
    .absolute()
    .size_0()
    .into_any_element()
}

#[cfg(test)]
pub(super) fn test_callback_count(mut items: Vec<MenuItem>) -> usize {
    let mut callbacks = HashMap::new();
    bind_branch(&mut items, Uuid::new_v4(), &mut 0, &mut callbacks, true);
    callbacks.len()
}
#[cfg(test)]
pub(super) fn test_terminal_is_once_deferred(cx: &mut gpui_kit::TestAppContext) {
    use gpui_kit::{Context, Render, TestAppContext};
    struct Fixture;
    impl Render for Fixture {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }
    cx.update(gpui_kit::init);
    let (_, surface) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| Fixture);
        gpui_kit::base::Root::new(view, window, cx)
    });
    let (platform, endpoint) = nxtools_ipc::platform_bridge(16);
    endpoint.set_connected_for_contract_test(true);
    let platform = Rc::new(platform);
    let calls = Rc::new(Cell::new(0));
    for (cancel, stale) in [(false, false), (true, false), (false, true)] {
        let id = Uuid::new_v4();
        let alive = Rc::new(Cell::new(!stale));
        let callback_calls = calls.clone();
        let callback: Handler = Rc::new(move |_, window, cx| {
            assert!(!active(window, cx));
            callback_calls.set(callback_calls.get() + 1);
        });
        let handle = surface.update(|window, _| window.window_handle());
        surface.update(|_, cx| {
            cx.set_global(NativeMenuState {
                pending: Some(Pending {
                    id,
                    window: handle,
                    command_id: id,
                    source_hwnd: 1,
                    source_session: None,
                    trigger: Rc::downgrade(&alive),
                    callbacks: HashMap::from([(7, callback)]),
                    previous_focus: None,
                    platform: platform.clone(),
                    surface: None,
                    pointer: false,
                    pointer_inside_source: false,
                    source_bounds: Default::default(),
                    ready: true,
                    step: 2,
                    started: Instant::now(),
                }),
                source_dismiss: None,
            });
            if cancel {
                finish(id, None, false, "host_retired", cx);
            }
            finish(id, Some(7), false, "selected", cx);
            finish(id, Some(7), false, "selected", cx);
            assert_eq!(
                calls.get(),
                usize::from(cancel || stale),
                "callback must be deferred"
            );
        });
        surface.run_until_parked();
    }
    assert_eq!(
        calls.get(),
        1,
        "duplicate/cancelled/stale trigger cannot execute"
    );
    let _: &mut TestAppContext = cx;
}

#[cfg(test)]
#[path = "native_context_menu_tests.rs"]
mod tests;

#[cfg(debug_assertions)]
#[path = "native_context_menu_preview.rs"]
mod preview;
#[cfg(debug_assertions)]
pub(crate) use preview::open as open_isolated_preview;

#[cfg(test)]
pub(crate) fn install_pending_for_test(
    window: AnyWindowHandle,
    platform: Rc<PlatformBridge>,
    cx: &mut App,
) -> Rc<Cell<bool>> {
    let token = Rc::new(Cell::new(true));
    let id = Uuid::new_v4();
    cx.set_global(NativeMenuState {
        pending: Some(Pending {
            id,
            window,
            command_id: id,
            source_hwnd: 1,
            source_session: None,
            trigger: Rc::downgrade(&token),
            callbacks: HashMap::new(),
            previous_focus: None,
            platform,
            surface: None,
            pointer: false,
            pointer_inside_source: false,
            source_bounds: Default::default(),
            ready: true,
            step: 2,
            started: Instant::now(),
        }),
        source_dismiss: None,
    });
    token
}
