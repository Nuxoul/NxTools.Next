//! Effective material state for existing GPUI windows; no presentation operations.
use super::*;
use gpui_kit::{AnyWindowHandle, BorrowAppContext, Subscription};
use nxtools_ipc::{PlatformBridge, PlatformNotification};
use std::rc::Rc;

mod readiness;
pub(crate) use readiness::when_ready;
use readiness::{Readiness, disconnected, queue_readiness};
#[cfg(test)]
use readiness::{expire_readiness, when_ready_with_handle};
#[cfg(test)]
pub(crate) use readiness::{set_test_window_handle, test_window_handle};

struct RegisteredWindow {
    window: AnyWindowHandle,
    hwnd: i64,
    surface: MaterialSurface,
    preparation: Preparation,
    readiness: Option<Readiness>,
}
struct MaterialRuntime {
    platform: Rc<PlatformBridge>,
    windows: Vec<RegisteredWindow>,
    scheduled: bool,
    _subscriptions: Vec<Subscription>,
}
impl Global for MaterialRuntime {}

/// Whether the original owner already has a first-show completion for this window.
pub(crate) fn is_waiting(window: AnyWindowHandle, cx: &App) -> bool {
    cx.try_global::<MaterialRuntime>().is_some_and(|runtime| {
        runtime
            .windows
            .iter()
            .any(|record| record.window == window && record.readiness.is_some())
    })
}

fn install(platform: Rc<PlatformBridge>, cx: &mut App) {
    if cx.try_global::<MaterialRuntime>().is_some() {
        return;
    }
    cx.set_global(MaterialRuntime {
        platform,
        windows: Vec::new(),
        scheduled: false,
        _subscriptions: Vec::new(),
    });
    let subscriptions = vec![
        cx.observe_global::<UiMaterialCapabilities>(schedule),
        cx.observe_global::<crate::theme::UiThemeState>(schedule),
        cx.on_window_closed(|cx, closed| {
            cx.update_global::<MaterialRuntime, _>(|runtime, cx| {
                runtime.windows.retain_mut(|record| {
                    if record.window.window_id() != closed {
                        return true;
                    }
                    paint::retire(record.hwnd, cx);
                    record.preparation.retire(&runtime.platform);
                    false
                });
            });
        }),
    ];
    cx.global_mut::<MaterialRuntime>()._subscriptions = subscriptions;
}

/// Transfer a presentation owner's confirmed request/receipt without showing.
pub(crate) fn track_prepared(
    window: AnyWindowHandle,
    platform: Rc<PlatformBridge>,
    preparation: Preparation,
    cx: &mut App,
) -> bool {
    let Some(pending) = preparation.pending.as_ref() else {
        return false;
    };
    let Some(receipt) = pending
        .applied
        .filter(|receipt| receipt.matches(&pending.command))
    else {
        return false;
    };
    let current = preparation.receipt(cx).is_some();
    if !cx.windows().contains(&window) {
        return false;
    }
    paint::publish(pending.command, receipt, cx);
    install(platform, cx);
    let runtime = cx.global_mut::<MaterialRuntime>();
    if let Some(index) = runtime
        .windows
        .iter()
        .position(|record| record.window == window)
    {
        let mut previous = runtime.windows.remove(index);
        previous.preparation.retire(&runtime.platform);
    }
    runtime.windows.push(RegisteredWindow {
        window,
        hwnd: receipt.window_handle,
        surface: receipt.surface,
        preparation,
        readiness: None,
    });
    if !current {
        schedule(cx);
    }
    true
}

/// Keeps a long-lived window that is already on screen in step with the effective material.
/// The window shows its opaque foundation until the first confirmed receipt; style, palette and
/// capability changes re-apply it through the same reconcile turn. No visibility or placement.
pub(crate) fn watch(
    window: AnyWindowHandle,
    platform: Rc<PlatformBridge>,
    hwnd: i64,
    surface: MaterialSurface,
    cx: &mut App,
) {
    if hwnd == 0 || !cx.windows().contains(&window) {
        return;
    }
    install(platform, cx);
    let runtime = cx.global_mut::<MaterialRuntime>();
    if runtime.windows.iter().any(|record| record.window == window) {
        return;
    }
    runtime.windows.push(RegisteredWindow {
        window,
        hwnd,
        surface,
        preparation: Preparation::default(),
        readiness: None,
    });
    schedule(cx);
}

/// Reuse presentation temporarily owns the same request, including an in-flight update.
pub(crate) fn take_preparation(window: AnyWindowHandle, cx: &mut App) -> Option<Preparation> {
    let runtime = cx.try_global::<MaterialRuntime>()?;
    let index = runtime
        .windows
        .iter()
        .position(|record| record.window == window)?;
    Some(
        cx.global_mut::<MaterialRuntime>()
            .windows
            .remove(index)
            .preparation,
    )
}

pub(crate) fn receipt(window: AnyWindowHandle, cx: &App) -> Option<AppliedWindowMaterial> {
    cx.try_global::<MaterialRuntime>()?
        .windows
        .iter()
        .find(|record| record.window == window)?
        .preparation
        .receipt(cx)
}

fn schedule(cx: &mut App) {
    let Some(runtime) = cx.try_global::<MaterialRuntime>() else {
        return;
    };
    if runtime.scheduled {
        return;
    }
    cx.global_mut::<MaterialRuntime>().scheduled = true;
    cx.defer(reconcile);
}
fn reconcile(cx: &mut App) {
    let live = cx.windows();
    cx.update_global::<MaterialRuntime, _>(|runtime, cx| {
        runtime.scheduled = false;
        runtime.windows.retain_mut(|record| {
            if !live.contains(&record.window) {
                record.preparation.retire(&runtime.platform);
                return false;
            }
            if disconnected(record, &runtime.platform) {
                queue_readiness(record, Err("material_disconnected"), cx);
                return true;
            }
            match record
                .preparation
                .ensure(&runtime.platform, record.hwnd, record.surface, cx)
            {
                Ok(true) => queue_readiness(record, Ok(()), cx),
                Ok(false) => {}
                Err(reason) => {
                    tracing::warn!(
                        event = "product.material.runtime.failed",
                        window_handle = record.hwnd,
                        reason
                    );
                    queue_readiness(record, Err(reason), cx);
                }
            }
            true
        });
    });
    cx.refresh_windows();
}

/// The original dispatcher owns result delivery; this module never claims a receiver.
pub(crate) fn handle_notification(notification: &PlatformNotification, cx: &mut App) -> bool {
    let PlatformNotification::CommandResult {
        correlation_id,
        result,
    } = notification
    else {
        return false;
    };
    if cx.try_global::<MaterialRuntime>().is_none() {
        return false;
    }
    let owned = cx.update_global::<MaterialRuntime, _>(|runtime, cx| {
        for record in &mut runtime.windows {
            let Some(accepted) = record.preparation.accept(*correlation_id, result, cx) else { continue; };
            match accepted {
                Ok(true) => {
                    if let Some(receipt) = record.preparation.receipt(cx) {
                        tracing::info!(event = "product.material.runtime.completed", command_id = %correlation_id,
                            window_handle = record.hwnd, effective = ?receipt.effective,
                            fallback_reason = ?receipt.fallback_reason, native_error_code = receipt.native_error_code);
                    }
                }
                Ok(false) => {}
                Err(reason) => {
                    tracing::warn!(event = "product.material.runtime.failed", command_id = %correlation_id,
                        window_handle = record.hwnd, reason, native_error_code = result.native_error_code);
                    queue_readiness(record, Err(reason), cx);
                }
            }
            return true;
        }
        false
    });
    if owned {
        schedule(cx);
    }
    owned
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod readiness_tests;
