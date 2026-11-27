//! Reassert selected native policy after the framework's OS-appearance write.
use super::*;
use gpui_kit::{AnyWindowHandle, BorrowAppContext, Subscription};

struct ObservedWindow {
    window: AnyWindowHandle,
    hwnd: i64,
    minimum_generation: u64,
    _appearance: Subscription,
}
struct NativeAppearance {
    windows: Vec<ObservedWindow>,
    _closed: Subscription,
}
impl Global for NativeAppearance {}

pub(super) fn current(command: &ApplyWindowMaterialCommand, cx: &App) -> bool {
    cx.try_global::<NativeAppearance>()
        .and_then(|state| {
            state
                .windows
                .iter()
                .find(|record| record.hwnd == command.window_handle)
        })
        .is_none_or(|record| command.generation >= record.minimum_generation)
}

pub(super) fn observe(hwnd: i64, cx: &mut App) {
    if is_observed(hwnd, cx) {
        return;
    }
    cx.defer(move |cx| {
        if is_observed(hwnd, cx) {
            return;
        }
        for handle in cx.windows() {
            let matches = handle
                .update(cx, |_, window, _| {
                    crate::ui::native_window_handle(window) == Some(hwnd)
                })
                .unwrap_or(false);
            if matches {
                observe_window(handle, hwnd, cx);
                return;
            }
        }
    });
}

pub(super) fn observe_window(handle: AnyWindowHandle, hwnd: i64, cx: &mut App) {
    if is_observed(hwnd, cx) {
        return;
    }
    let subscription = handle.update(cx, |_, window, _| {
        window.observe_window_appearance(move |_, cx| {
            cx.defer(move |cx| invalidate(hwnd, cx));
        })
    });
    let Ok(subscription) = subscription else {
        return;
    };
    if cx.try_global::<NativeAppearance>().is_none() {
        let closed = cx.on_window_closed(|cx, closed| {
            cx.update_global::<NativeAppearance, _>(|state, cx| {
                state.windows.retain(|record| {
                    if record.window.window_id() != closed {
                        return true;
                    }
                    paint::retire(record.hwnd, cx);
                    false
                });
            });
        });
        cx.set_global(NativeAppearance {
            windows: Vec::new(),
            _closed: closed,
        });
    }
    cx.global_mut::<NativeAppearance>()
        .windows
        .push(ObservedWindow {
            window: handle,
            hwnd,
            minimum_generation: 0,
            _appearance: subscription,
        });
}

pub(super) fn is_observed(hwnd: i64, cx: &App) -> bool {
    cx.try_global::<NativeAppearance>()
        .is_some_and(|state| state.windows.iter().any(|record| record.hwnd == hwnd))
}

pub(super) fn invalidate(hwnd: i64, cx: &mut App) {
    let minimum_generation = cx
        .try_global::<MaterialGeneration>()
        .map_or(1, |generation| {
            generation
                .0
                .checked_add(1)
                .expect("material request generation exhausted")
        });
    let Some(state) = cx.try_global::<NativeAppearance>() else {
        return;
    };
    let Some(index) = state.windows.iter().position(|record| record.hwnd == hwnd) else {
        return;
    };
    cx.global_mut::<NativeAppearance>().windows[index].minimum_generation = minimum_generation;
    if let Some(facts) = cx.try_global::<UiMaterialCapabilities>().copied() {
        cx.set_global(facts);
    }
}
