//! Material readiness retains the original raw-join window and close/cancel owner.
use super::*;

pub(super) fn prepare(window: AnyWindowHandle, platform: Rc<PlatformBridge>, cx: &mut App) {
    let Some(Target(view, current)) = cx.try_global::<Target>().cloned() else {
        return;
    };
    if current != window {
        return;
    }
    let Ok(session) = view.read_with(cx, |view, _| view.session) else {
        return;
    };
    cx.defer(move |cx| {
        crate::ui::material::runtime::when_ready(
            window,
            platform,
            nxtools_protocol::window_material::MaterialSurface::Backdrop,
            std::time::Duration::from_secs(8),
            move |result, cx| finish(window, view, session, result, cx),
            cx,
        );
    });
}

fn finish(
    window: AnyWindowHandle,
    view: WeakEntity<JoinWindow>,
    session: Uuid,
    result: Result<(), &'static str>,
    cx: &mut App,
) {
    if cx
        .try_global::<Target>()
        .is_none_or(|target| target.1 != window || target.0 != view)
    {
        return;
    }
    let _ = window.update(cx, |_, window, cx| {
        let _ = view.update(cx, |view, cx| {
            if view.session != session || !view.material_pending { return; }
            view.material_pending = false;
            if let Err(reason) = result {
                tracing::warn!(event="tools.raw_numbered_join.material_failed",correlation_id=%session,reason);
                view.close(window, cx);
            } else {
                window.activate_window();
                window.refresh();
            }
        });
    });
}
