use super::*;
use std::time::{Duration, Instant};
use uuid::Uuid;

type Completion = Box<dyn FnOnce(Result<(), &'static str>, &mut App)>;
pub(super) struct Readiness {
    pub(super) token: Uuid,
    connection: Option<u64>,
    started: Instant,
    scheduled: bool,
    completion: Completion,
}

/// Notify the original presentation owner; never changes visibility, placement or focus.
/// Call outside a lease of the target window.
pub(crate) fn when_ready(
    window: AnyWindowHandle,
    platform: Rc<PlatformBridge>,
    surface: MaterialSurface,
    timeout: Duration,
    completion: impl FnOnce(Result<(), &'static str>, &mut App) + 'static,
    cx: &mut App,
) {
    let hwnd = window
        .update(cx, |_, window, _| crate::ui::native_window_handle(window))
        .ok()
        .flatten();
    #[cfg(test)]
    let hwnd = cx
        .try_global::<TestWindowHandles>()
        .and_then(|handles| handles.0.get(&window).copied())
        .or(hwnd);
    let Some(hwnd) = hwnd else {
        cx.defer(move |cx| completion(Err("material_window_handle"), cx));
        return;
    };
    when_ready_with_handle(window, platform, hwnd, surface, timeout, completion, cx);
}

pub(super) fn when_ready_with_handle(
    window: AnyWindowHandle,
    platform: Rc<PlatformBridge>,
    hwnd: i64,
    surface: MaterialSurface,
    timeout: Duration,
    completion: impl FnOnce(Result<(), &'static str>, &mut App) + 'static,
    cx: &mut App,
) {
    if !cx.windows().contains(&window) {
        return;
    }
    install(platform.clone(), cx);
    let preparation = take_preparation(window, cx).unwrap_or_default();
    let token = Uuid::new_v4();
    let readiness = Readiness {
        token,
        connection: platform.connection_generation(),
        started: Instant::now(),
        scheduled: false,
        completion: Box::new(completion),
    };
    tracing::info!(event = "product.material.readiness.started", correlation_id = %token, window_handle = hwnd);
    cx.global_mut::<MaterialRuntime>()
        .windows
        .push(RegisteredWindow {
            window,
            hwnd,
            surface,
            preparation,
            readiness: Some(readiness),
        });
    schedule(cx);
    cx.spawn(async move |cx| {
        cx.background_executor().timer(timeout).await;
        cx.update(move |cx| expire_readiness(window, token, cx));
    })
    .detach();
}

pub(super) fn queue_readiness(
    record: &mut RegisteredWindow,
    result: Result<(), &'static str>,
    cx: &mut App,
) {
    let Some(readiness) = record.readiness.as_mut() else {
        return;
    };
    if readiness.scheduled {
        return;
    }
    readiness.scheduled = true;
    let token = readiness.token;
    let window = record.window;
    cx.defer(move |cx| complete_readiness(window, token, result, cx));
}

pub(super) fn disconnected(record: &RegisteredWindow, platform: &PlatformBridge) -> bool {
    record.readiness.as_ref().is_some_and(|ready| {
        ready.connection.is_some() && ready.connection != platform.connection_generation()
    })
}

pub(super) fn expire_readiness(window: AnyWindowHandle, token: Uuid, cx: &mut App) {
    complete_readiness(window, token, Err("material_timeout"), cx);
}

fn complete_readiness(
    window: AnyWindowHandle,
    token: Uuid,
    mut result: Result<(), &'static str>,
    cx: &mut App,
) {
    let Some(runtime) = cx.try_global::<MaterialRuntime>() else {
        return;
    };
    let Some(index) = runtime.windows.iter().position(|record| {
        record.window == window
            && record
                .readiness
                .as_ref()
                .is_some_and(|ready| ready.token == token)
    }) else {
        return;
    };
    if result.is_ok() && disconnected(&runtime.windows[index], &runtime.platform) {
        result = Err("material_disconnected");
    }
    if result.is_ok() && runtime.windows[index].preparation.receipt(cx).is_none() {
        cx.global_mut::<MaterialRuntime>().windows[index]
            .readiness
            .as_mut()
            .expect("current callback")
            .scheduled = false;
        schedule(cx);
        return;
    }
    let readiness = if result.is_err() {
        let runtime = cx.global_mut::<MaterialRuntime>();
        let mut record = runtime.windows.remove(index);
        record.preparation.retire(&runtime.platform);
        record.readiness.take().expect("current callback")
    } else {
        cx.global_mut::<MaterialRuntime>().windows[index]
            .readiness
            .take()
            .expect("current callback")
    };
    tracing::info!(event = "product.material.readiness.completed", correlation_id = %readiness.token,
        success = result.is_ok(), reason = result.err(), elapsed_ms = readiness.started.elapsed().as_millis() as u64);
    (readiness.completion)(result, cx);
}

#[cfg(test)]
#[derive(Default)]
struct TestWindowHandles(std::collections::HashMap<AnyWindowHandle, i64>);
#[cfg(test)]
impl Global for TestWindowHandles {}

/// Explicit synthetic HWND for one TestWindow; production always reads its native handle.
#[cfg(test)]
pub(crate) fn set_test_window_handle(window: AnyWindowHandle, hwnd: i64, cx: &mut App) {
    assert_ne!(hwnd, 0);
    cx.default_global::<TestWindowHandles>()
        .0
        .insert(window, hwnd);
}

#[cfg(test)]
pub(crate) fn test_window_handle(window: AnyWindowHandle, cx: &App) -> Option<i64> {
    cx.try_global::<TestWindowHandles>()?
        .0
        .get(&window)
        .copied()
}
