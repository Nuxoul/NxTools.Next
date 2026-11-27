use super::*;
use gpui_kit::base::{Root, RootPlugin as _};
use gpui_kit::{AppContext as _, InteractiveElement as _, TestAppContext};
use nxtools_protocol::window_material::MaterialFallbackReason;
use nxtools_protocol::{Message, PlatformCommand, PlatformCommandResult};
use uuid::Uuid;

fn facts(cx: &mut App, revision: u64) {
    system_facts(cx, revision, true);
}
/// `system`: Windows 11 system acrylic; otherwise the Windows 10 host backdrop.
fn system_facts(cx: &mut App, revision: u64, system: bool) {
    let facts = cx.default_global::<UiMaterialCapabilities>();
    facts.connected();
    let session = facts
        .snapshot()
        .map_or_else(Uuid::new_v4, |facts| facts.session_id);
    assert!(facts.apply(SystemMaterialCapabilitiesChanged {
        correlation_id: Uuid::new_v4(),
        session_id: session,
        revision,
        supported: true,
        transparency_enabled: true,
        high_contrast: false,
        energy_saver: false,
        remote_session: false,
        native_error_code: 0,
        system_backdrop: system,
        rounded_corners: system,
    }));
}
fn receipt(
    command: ApplyWindowMaterialCommand,
    effective: EffectiveWindowMaterial,
) -> AppliedWindowMaterial {
    AppliedWindowMaterial {
        session_id: command.session_id,
        facts_revision: command.facts_revision,
        window_handle: command.window_handle,
        generation: command.generation,
        style: command.style,
        surface: command.surface,
        dark: command.dark,
        effective,
        fallback_reason: if effective == EffectiveWindowMaterial::AcrylicOpaque {
            MaterialFallbackReason::Unsupported
        } else {
            MaterialFallbackReason::None
        },
        native_error_code: 0,
    }
}
fn setup(cx: &mut TestAppContext, style: nxtools_core::MaterialStyle) -> gpui_kit::AnyWindowHandle {
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    cx.update(|cx| {
        crate::theme::set_appearance_preferences(nxtools_core::ThemePreference::Dark, style, cx);
        facts(cx, 1);
    });
    let (_, surface) = cx.add_window_view(|window, cx| {
        let content = cx.new(|_| Content);
        Root::new(content, window, cx)
    });
    let handle = surface.update(|window, _| window.window_handle());
    cx.update(|cx| {
        runtime::set_test_window_handle(handle, 42, cx);
        native_appearance::observe_window(handle, 42, cx);
    });
    handle
}
/// The Root fill the material plugin paints; `None` keeps the official Root fill.
fn fill(handle: gpui_kit::AnyWindowHandle, expected: Option<Hsla>, cx: &mut TestAppContext) {
    handle
        .update(cx, |_, window, cx| {
            assert!(Root::read(window, cx).plugin::<MaterialRoot>().is_some());
            let mut surface = gpui_kit::div().id("material-style-probe");
            MaterialRoot.style(&mut surface, window, cx);
            assert_eq!(
                surface.style().background,
                expected
                    .and_then(|color| gpui_kit::StyleRefinement::default().bg(color).background)
            );
        })
        .expect("live material Root");
}
fn publish_effect(surface: MaterialSurface, effect: EffectiveWindowMaterial, cx: &mut App) {
    let command = command(42, surface, cx).expect("current host material command");
    publish(command, receipt(command, effect), cx);
}
fn background(cx: &mut TestAppContext) -> Option<WindowBackgroundAppearance> {
    cx.read(|cx| {
        cx.try_global::<Backgrounds>()
            .and_then(|backgrounds| backgrounds.0.get(&42).map(|known| known.appearance))
    })
}
struct Content;
impl Render for Content {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        gpui_kit::div()
    }
}

#[gpui_kit::test]
fn material_paint_is_published_before_presentation_runtime_handoff(cx: &mut TestAppContext) {
    let handle = setup(cx, nxtools_core::MaterialStyle::Acrylic);
    let layers = cx.read(crate::theme::material::layers);
    let fallback = layers.opaque;
    fill(handle, Some(fallback), cx);
    let (bridge, endpoint) = nxtools_ipc::platform_bridge(8);
    endpoint.set_connected_for_contract_test(true);
    let mut preparation = Preparation::default();
    cx.update(|cx| {
        assert!(
            !preparation
                .ensure(&bridge, 42, MaterialSurface::Backdrop, cx)
                .expect("enqueue current material preparation")
        )
    });
    let envelope = endpoint
        .try_receive_command_for_contract_test()
        .expect("queued material request");
    let Message::PlatformCommand(PlatformCommand::ApplyWindowMaterial(command)) = envelope.message
    else {
        panic!("material command")
    };
    assert_eq!(
        command.style,
        nxtools_protocol::window_material::WindowMaterialStyle::Acrylic
    );
    cx.update(|cx| {
        assert_eq!(effective(42, cx), None);
        let result = PlatformCommandResult {
            success: true,
            error: None,
            native_error_code: None,
            data: Some(
                serde_json::to_string(&receipt(command, EffectiveWindowMaterial::Acrylic))
                    .expect("serialize coherent material receipt"),
            ),
        };
        assert_eq!(preparation.accept(envelope.id, &result, cx), Some(Ok(true)));
        assert_eq!(effective(42, cx), Some(EffectiveWindowMaterial::Acrylic));
        assert!(
            runtime::receipt(handle, cx).is_none(),
            "original presenter still owns preparation"
        );
    });
    // Dark content layer: the default 50 % slider at 0.9, over the veil.
    assert!((layers.content.a - 0.45).abs() < 0.0001);
    fill(handle, Some(layers.backdrop()), cx);
    cx.update(|cx| facts(cx, 2));
    fill(handle, Some(fallback), cx);
    assert_eq!(cx.read(|cx| effective(42, cx)), None);
}

#[gpui_kit::test]
fn material_paint_layers_follow_the_surface_kind(cx: &mut TestAppContext) {
    let handle = setup(cx, nxtools_core::MaterialStyle::Acrylic);
    let clear = gpui_kit::transparent_black();
    let layers = cx.read(crate::theme::material::layers);
    for (surface, effect, expected) in [
        (
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::AcrylicOpaque,
            layers.opaque,
        ),
        (
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::Acrylic,
            layers.backdrop(),
        ),
        // Transient lists sit on the veiled system acrylic.
        (
            MaterialSurface::Transient,
            EffectiveWindowMaterial::Acrylic,
            layers.transient,
        ),
        (
            MaterialSurface::Transparent,
            EffectiveWindowMaterial::Transparent,
            clear,
        ),
    ] {
        cx.update(|cx| publish_effect(surface, effect, cx));
        fill(handle, Some(expected), cx);
    }
    // A layered window keeps the veil and paints the content layer in its own content area.
    cx.update(|cx| {
        publish_effect(
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::Acrylic,
            cx,
        );
        mark_layered(42, cx);
    });
    fill(handle, Some(layers.veil), cx);
    handle
        .update(cx, |_, window, cx| {
            assert_eq!(content_layer(window, cx), Some(layers.content));
        })
        .expect("live layered window");
    handle
        .update(cx, |_, window, _| window.remove_window())
        .expect("remove owned material window");
    cx.run_until_parked();
    assert_eq!(cx.read(|cx| effective(42, cx)), None);
    assert!(cx.read(|cx| !is_layered(42, cx)));
}

#[gpui_kit::test]
fn material_paint_solid_windows_render_opaque_and_return_to_transparent(cx: &mut TestAppContext) {
    let handle = setup(cx, nxtools_core::MaterialStyle::Solid);
    // Unknown surface: the window keeps the transparent background it was created with.
    fill(handle, None, cx);
    assert_eq!(background(cx), None);
    cx.update(|cx| {
        publish_effect(
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::Solid,
            cx,
        )
    });
    fill(handle, None, cx);
    assert_eq!(background(cx), Some(WindowBackgroundAppearance::Opaque));
    let pending = cx.update(|cx| {
        crate::theme::set_appearance_preferences(
            nxtools_core::ThemePreference::Dark,
            nxtools_core::MaterialStyle::Acrylic,
            cx,
        );
        command(42, MaterialSurface::Backdrop, cx).expect("frosted request")
    });
    // The next frame returns to Transparent. That write replaced the accent, so the frosted
    // request made before it is superseded and the material is applied again.
    cx.run_until_parked();
    assert_eq!(
        background(cx),
        Some(WindowBackgroundAppearance::Transparent)
    );
    assert!(cx.read(|cx| !is_current(&pending, cx)));
    let layers = cx.read(crate::theme::material::layers);
    fill(handle, Some(layers.opaque), cx);
    cx.update(|cx| {
        publish_effect(
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::Acrylic,
            cx,
        )
    });
    fill(handle, Some(layers.backdrop()), cx);
}

#[gpui_kit::test]
fn material_paint_rejects_malformed_and_stale_projection(cx: &mut TestAppContext) {
    let handle = setup(cx, nxtools_core::MaterialStyle::Acrylic);
    cx.update(|cx| {
        let command = command(42, MaterialSurface::Backdrop, cx).expect("current backdrop command");
        let mut malformed = receipt(command, EffectiveWindowMaterial::Acrylic);
        malformed.window_handle = 43;
        publish(command, malformed, cx);
        assert_eq!(effective(42, cx), None);
        facts(cx, 2);
        publish(
            command,
            receipt(command, EffectiveWindowMaterial::Acrylic),
            cx,
        );
        assert_eq!(effective(42, cx), None);
    });
    let layers = cx.read(crate::theme::material::layers);
    fill(handle, Some(layers.opaque), cx);
}

fn grain_of(handle: gpui_kit::AnyWindowHandle, cx: &mut TestAppContext) -> Option<f32> {
    handle
        .update(cx, |_, window, cx| grain(window, cx))
        .expect("live material Root")
}

#[gpui_kit::test]
fn material_paint_grains_only_the_live_windows_10_blur(cx: &mut TestAppContext) {
    let handle = setup(cx, nxtools_core::MaterialStyle::Acrylic);
    // System acrylic carries its own noise.
    cx.update(|cx| {
        publish_effect(
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::Acrylic,
            cx,
        )
    });
    assert_eq!(grain_of(handle, cx), None);
    cx.update(|cx| system_facts(cx, 2, false));
    // No confirmed blur yet: the opaque fallback carries no grain.
    assert_eq!(grain_of(handle, cx), None);
    let dark_grain = crate::theme::material::BLUR_GRAIN[1];
    for surface in [MaterialSurface::Backdrop, MaterialSurface::Transient] {
        cx.update(|cx| publish_effect(surface, EffectiveWindowMaterial::Acrylic, cx));
        assert_eq!(grain_of(handle, cx), Some(dark_grain));
    }
    cx.update(|cx| {
        publish_effect(
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::AcrylicOpaque,
            cx,
        )
    });
    assert_eq!(grain_of(handle, cx), None);
    cx.update(|cx| {
        publish_effect(
            MaterialSurface::Transparent,
            EffectiveWindowMaterial::Transparent,
            cx,
        )
    });
    assert_eq!(grain_of(handle, cx), None);
    cx.update(|cx| {
        publish_effect(
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::Acrylic,
            cx,
        );
        crate::theme::set_appearance_preferences(
            nxtools_core::ThemePreference::Light,
            nxtools_core::MaterialStyle::Acrylic,
            cx,
        );
        publish_effect(
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::Acrylic,
            cx,
        );
    });
    assert_eq!(
        grain_of(handle, cx),
        Some(crate::theme::material::BLUR_GRAIN[0])
    );
    cx.update(|cx| {
        crate::theme::set_appearance_preferences(
            nxtools_core::ThemePreference::Light,
            nxtools_core::MaterialStyle::Solid,
            cx,
        )
    });
    assert_eq!(grain_of(handle, cx), None);
}

#[test]
fn material_paint_grain_tile_is_fine_balanced_and_bounded() {
    let amplitude = crate::theme::material::BLUR_GRAIN[1];
    let pixels = grain_pixels(amplitude);
    assert_eq!(pixels.dimensions(), (GRAIN_TILE, GRAIN_TILE));
    let ceiling = (amplitude * 255.0).ceil() as u8;
    let (mut lighten, mut darken, mut net) = (0u32, 0u32, 0i64);
    for pixel in pixels.pixels() {
        let [level, _, _, alpha] = pixel.0;
        assert!(alpha <= ceiling, "grain alpha {alpha} above {ceiling}");
        assert!(level == 0 || level == 255);
        if alpha > 0 {
            if level == 255 {
                lighten += 1;
                net += i64::from(alpha);
            } else {
                darken += 1;
                net -= i64::from(alpha);
            }
        }
    }
    let count = f64::from(GRAIN_TILE * GRAIN_TILE);
    // Both directions appear about equally and the tile neither lightens nor darkens overall.
    assert!((f64::from(lighten) / count - f64::from(darken) / count).abs() < 0.02);
    assert!((net as f64 / count).abs() < 0.05, "net shift {net}");
    // The same seed gives the same texture in every window.
    assert_eq!(grain_pixels(amplitude), pixels);
}

/// The acrylic the active scheme asks for: its canvas through the Fluent luminosity recipe.
fn acrylic(cx: &mut TestAppContext) -> super::super::win10::Recipe {
    cx.read(|cx| {
        let (tint, luminosity_percent, tint_percent) = crate::theme::material::host_acrylic(cx);
        let canvas = crate::theme::material::layers(cx).opaque.to_rgb();
        let channel = |value: f32| (value * 255.0).round() as u32;
        assert_eq!(
            tint,
            (channel(canvas.r) << 16) | (channel(canvas.g) << 8) | channel(canvas.b),
            "the scheme canvas tints the acrylic"
        );
        super::super::win10::Recipe {
            tint,
            luminosity_percent,
            tint_percent,
            luminosity_mode: super::super::win10::BLEND_COLOR,
            tint_mode: super::super::win10::BLEND_LUMINOSITY,
        }
    })
}

#[gpui_kit::test]
fn material_paint_windows_10_rounds_framed_windows_and_hosts_only_live_frosting(
    cx: &mut TestAppContext,
) {
    use super::super::win10::Wanted;
    let handle = setup(cx, nxtools_core::MaterialStyle::Acrylic);
    let shape = |effect: Option<EffectiveWindowMaterial>, cx: &mut TestAppContext| {
        handle
            .update(cx, |_, window, cx| {
                // Render records the surface kind the shape follows.
                let mut surface = gpui_kit::div().id("material-shape-probe");
                MaterialRoot.style(&mut surface, window, cx);
                win10_shape(42, effect, cx)
            })
            .expect("live material Root")
    };
    // Windows 11 22H2: DWM rounds and system acrylic frosts; nothing native to add.
    cx.update(|cx| {
        publish_effect(
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::Acrylic,
            cx,
        )
    });
    assert_eq!(
        shape(Some(EffectiveWindowMaterial::Acrylic), cx),
        Wanted {
            round: false,
            acrylic: None,
            dark: true
        }
    );
    cx.update(|cx| system_facts(cx, 2, false));
    for surface in [MaterialSurface::Backdrop, MaterialSurface::Transient] {
        cx.update(|cx| publish_effect(surface, EffectiveWindowMaterial::Acrylic, cx));
        assert_eq!(
            shape(Some(EffectiveWindowMaterial::Acrylic), cx),
            Wanted {
                round: true,
                acrylic: Some(acrylic(cx)),
                dark: true
            },
            "{surface:?}"
        );
    }
    // The opaque fallback keeps the corners but shows no host backdrop.
    cx.update(|cx| {
        publish_effect(
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::AcrylicOpaque,
            cx,
        )
    });
    assert_eq!(
        shape(Some(EffectiveWindowMaterial::AcrylicOpaque), cx),
        Wanted {
            round: true,
            acrylic: None,
            dark: true
        }
    );
    // Solid windows are rounded too.
    cx.update(|cx| {
        crate::theme::set_appearance_preferences(
            nxtools_core::ThemePreference::Light,
            nxtools_core::MaterialStyle::Solid,
            cx,
        );
        publish_effect(
            MaterialSurface::Backdrop,
            EffectiveWindowMaterial::Solid,
            cx,
        );
    });
    assert_eq!(
        shape(Some(EffectiveWindowMaterial::Solid), cx),
        Wanted {
            round: true,
            acrylic: None,
            dark: false
        }
    );
    // Self-drawn overlays keep their own shape.
    cx.update(|cx| {
        publish_effect(
            MaterialSurface::Transparent,
            EffectiveWindowMaterial::Transparent,
            cx,
        )
    });
    assert_eq!(
        shape(Some(EffectiveWindowMaterial::Transparent), cx),
        Wanted {
            round: false,
            acrylic: None,
            dark: false
        }
    );
}
