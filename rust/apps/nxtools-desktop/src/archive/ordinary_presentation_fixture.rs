//! Confirmed material for tests isolating Archive's existing presentation semantics.
use super::*;
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, EffectiveWindowMaterial, MaterialFallbackReason, MaterialSurface,
    WindowMaterialStyle,
};
use std::cell::Cell;

pub(super) fn install(platform: Rc<PlatformBridge>, cx: &mut App) {
    if cx
        .try_global::<crate::ui::material::UiMaterialCapabilities>()
        .copied()
        .and_then(crate::ui::material::UiMaterialCapabilities::snapshot)
        .is_none()
    {
        crate::archive::ordinary_material_test_support::facts(Uuid::new_v4(), 1, cx);
    }
    let next_hwnd = Cell::new(8000_i64);
    cx.observe_new::<gpui_kit::base::Root>(move |root, window, cx| {
        let view = root.view().clone();
        if view.clone().downcast::<BrowserHost>().is_err()
            && view
                .clone()
                .downcast::<crate::archive::forms::OperationForm>()
                .is_err()
            && view
                .downcast::<crate::archive::progress::JobProgressView>()
                .is_err()
        {
            return;
        }
        let Some(window) = window else {
            return;
        };
        let handle = window.window_handle();
        let hwnd = next_hwnd.get();
        next_hwnd.set(hwnd + 1);
        crate::ui::material::runtime::set_test_window_handle(handle, hwnd, cx);
        let command = crate::ui::material::command(hwnd, MaterialSurface::Backdrop, cx)
            .expect("synthetic current material facts");
        let receipt = AppliedWindowMaterial {
            session_id: command.session_id,
            facts_revision: command.facts_revision,
            window_handle: hwnd,
            generation: command.generation,
            style: command.style,
            surface: command.surface,
            dark: command.dark,
            effective: if command.style == WindowMaterialStyle::Solid {
                EffectiveWindowMaterial::Solid
            } else {
                EffectiveWindowMaterial::Acrylic
            },
            fallback_reason: MaterialFallbackReason::None,
            native_error_code: 0,
        };
        let preparation =
            crate::ui::material::Preparation::confirmed(Uuid::new_v4(), command, receipt, cx)
                .expect("coherent synthetic material preparation");
        // Registration changes no window/entity state; the original deferred barrier
        // checks out this receipt and retains its own reveal/activation callback.
        assert!(crate::ui::material::runtime::track_prepared(
            handle,
            platform.clone(),
            preparation,
            cx,
        ));
    })
    .detach();
}
