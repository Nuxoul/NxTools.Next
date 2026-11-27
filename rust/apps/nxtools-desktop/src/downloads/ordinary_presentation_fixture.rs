//! Qualified material receipts for fixtures isolating Download presentation semantics.
use super::*;
use nxtools_protocol::window_material::{
    AppliedWindowMaterial, EffectiveWindowMaterial, MaterialFallbackReason, MaterialSurface,
    SystemMaterialCapabilitiesChanged, WindowMaterialStyle,
};

struct Installed;
impl gpui_kit::Global for Installed {}

/// Opt in before constructing the original application; material-boundary tests use `new` directly.
pub(crate) fn confirmed_application(
    engine: DownloadEngine,
    platform: Rc<PlatformBridge>,
    notifications: Entity<NotificationCenter>,
    runtime: DownloadRuntimeContext,
    cx: &mut Context<DownloadApplication>,
) -> DownloadApplication {
    install(platform.clone(), cx);
    DownloadApplication::new(engine, platform, notifications, runtime, cx)
}

fn install(platform: Rc<PlatformBridge>, cx: &mut App) {
    if cx.try_global::<Installed>().is_some() {
        return;
    }
    cx.set_global(Installed);
    let mut facts = cx
        .try_global::<crate::ui::material::UiMaterialCapabilities>()
        .copied()
        .unwrap_or_default();
    if facts.snapshot().is_none() {
        facts.connected();
        assert!(facts.apply(SystemMaterialCapabilitiesChanged {
            correlation_id: uuid::Uuid::new_v4(),
            session_id: uuid::Uuid::new_v4(),
            revision: 1,
            supported: true,
            transparency_enabled: true,
            high_contrast: false,
            energy_saver: false,
            remote_session: false,
            native_error_code: 0,
            system_backdrop: true,
            rounded_corners: true,
        }));
        cx.set_global(facts);
    }
    let next_hwnd = Cell::new(9000_i64);
    cx.observe_new::<Root>(move |root, window, cx| {
        let view = root.view().clone();
        let surface = if view
            .clone()
            .downcast::<DownloadTaskbarOverlayView>()
            .is_ok()
        {
            MaterialSurface::Transparent
        } else if view.clone().downcast::<NewDownloadView>().is_ok()
            || view.clone().downcast::<DownloadSettingsView>().is_ok()
            || view.clone().downcast::<DownloadScheduleView>().is_ok()
            || view.clone().downcast::<DeleteDownloadView>().is_ok()
            || view.clone().downcast::<DownloadTaskWindowView>().is_ok()
            || view.downcast::<DownloadShutdownCountdownView>().is_ok()
        {
            MaterialSurface::Backdrop
        } else {
            return;
        };
        let Some(window) = window else {
            return;
        };
        let handle = window.window_handle();
        let hwnd = next_hwnd.get();
        next_hwnd.set(hwnd + 1);
        crate::ui::material::runtime::set_test_window_handle(handle, hwnd, cx);
        let command = crate::ui::material::command(hwnd, surface, cx)
            .expect("synthetic current Download material facts");
        let receipt = AppliedWindowMaterial {
            session_id: command.session_id,
            facts_revision: command.facts_revision,
            window_handle: command.window_handle,
            generation: command.generation,
            style: command.style,
            surface: command.surface,
            dark: command.dark,
            effective: if surface == MaterialSurface::Transparent {
                EffectiveWindowMaterial::Transparent
            } else if command.style == WindowMaterialStyle::Solid {
                EffectiveWindowMaterial::Solid
            } else {
                EffectiveWindowMaterial::Acrylic
            },
            fallback_reason: MaterialFallbackReason::None,
            native_error_code: 0,
        };
        let preparation =
            crate::ui::material::Preparation::confirmed(uuid::Uuid::new_v4(), command, receipt, cx)
                .expect("coherent synthetic Download material preparation");
        // The original deferred readiness owner takes this same runtime record.
        assert!(crate::ui::material::runtime::track_prepared(
            handle,
            platform.clone(),
            preparation,
            cx,
        ));
    })
    .detach();
}

#[gpui_kit::test]
fn material_fixture_qualifies_only_download_roots(cx: &mut gpui_kit::TestAppContext) {
    cx.update(gpui_kit::init);
    let (app, directory) =
        super::tests::download_application_for_window_test(cx, "material-fixture");
    app.update(cx, |app, cx| {
        app.open_settings_window(cx);
        app.open_taskbar_overlay(cx);
    });
    let (_, unrelated) = cx.add_window_view(|window, cx| {
        let view = cx.new(|_| UnrelatedView);
        Root::new(view, window, cx)
    });
    let unrelated_handle = unrelated.update(|window, _| window.window_handle());
    cx.run_until_parked();
    app.read_with(cx, |app, cx| {
        let settings = crate::ui::material::runtime::receipt(
            app.settings_window.expect("qualified settings"),
            cx,
        )
        .expect("retained ordinary receipt");
        let overlay = crate::ui::material::runtime::receipt(
            app.taskbar_overlay_window.expect("qualified taskbar"),
            cx,
        )
        .expect("retained transparent receipt");
        assert_eq!(settings.surface, MaterialSurface::Backdrop);
        assert_eq!(overlay.surface, MaterialSurface::Transparent);
        assert_eq!(overlay.effective, EffectiveWindowMaterial::Transparent);
        assert_ne!(settings.window_handle, overlay.window_handle);
        assert!(crate::ui::material::runtime::receipt(unrelated_handle, cx).is_none());
    });
    std::fs::remove_dir_all(directory).expect("remove Download material fixture");
}

struct UnrelatedView;
impl Render for UnrelatedView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
