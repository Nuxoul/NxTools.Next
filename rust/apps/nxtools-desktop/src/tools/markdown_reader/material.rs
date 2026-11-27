//! Readiness joins the original reader reveal and Explorer foreground owners.
use super::*;

pub(super) fn prepare(handle: AnyWindowHandle, platform: Rc<PlatformBridge>, cx: &mut App) {
    cx.defer(move |cx| {
        crate::ui::material::runtime::when_ready(
            handle,
            platform,
            nxtools_protocol::window_material::MaterialSurface::Backdrop,
            Duration::from_secs(8),
            move |result, cx| finish(handle, result, cx),
            cx,
        );
    });
}

pub(super) fn present(handle: AnyWindowHandle, operation: Uuid, external: bool, cx: &mut App) {
    with_reader(handle, cx, move |reader, window, _| {
        reader.present_when_ready(operation, external, window);
    });
}

fn with_reader(
    handle: AnyWindowHandle,
    cx: &mut App,
    callback: impl FnOnce(&mut Reader, &mut Window, &mut Context<Reader>),
) {
    if cx.try_global::<ReaderWindow>().and_then(|state| state.0) != Some(handle) {
        return;
    }
    let _ = handle.update(cx, |root, window, cx| {
        let Ok(root) = root.downcast::<gpui_kit::base::Root>() else {
            return;
        };
        let Ok(reader) = root.read(cx).view().clone().downcast::<Reader>() else {
            return;
        };
        reader.update(cx, |reader, cx| callback(reader, window, cx));
    });
}

fn finish(handle: AnyWindowHandle, result: Result<(), &'static str>, cx: &mut App) {
    with_reader(handle, cx, move |reader, window, cx| {
        if !reader.material_pending {
            return;
        }
        reader.material_pending = false;
        let presentation = reader.material_presentation.take();
        if let Err(reason) = result {
            tracing::warn!(event = "markdown.reader.material_failed", correlation_id = ?presentation.map(|(operation, _)| operation), reason);
            reader.request_close(None, window, cx);
            return;
        }
        // Consumes GPUI's retained creation placement; no second positioning owner.
        window.activate_window();
        if let Some((operation, true)) = presentation {
            reader.request_foreground(operation, window);
        }
    });
}

impl Reader {
    pub(super) fn present_when_ready(
        &mut self,
        operation: Uuid,
        external: bool,
        window: &mut Window,
    ) {
        if self.material_pending {
            self.material_presentation = Some((operation, external));
        } else if external {
            self.request_foreground(operation, window);
        } else {
            window.activate_window();
        }
    }
}
