use super::material::{HOST_ACRYLIC, card_stroke, composite, content_alpha, host_acrylic, layers};
use super::*;
use nxtools_core::{MaterialStyle, MaterialTuning, ThemePreference};

fn close(actual: f32, expected: f32) -> bool {
    (actual - expected).abs() < 0.0001
}

fn rgb_hex(color: gpui_kit::Hsla) -> u32 {
    let rgba = color.to_rgb();
    let channel = |value: f32| (value * 255.0).round() as u32;
    (channel(rgba.r) << 16) | (channel(rgba.g) << 8) | channel(rgba.b)
}

fn report_corners(rounded: bool, cx: &mut App) {
    let mut facts = crate::ui::material::UiMaterialCapabilities::default();
    facts.connected();
    assert!(facts.apply(
        nxtools_protocol::window_material::SystemMaterialCapabilitiesChanged {
            correlation_id: uuid::Uuid::new_v4(),
            session_id: uuid::Uuid::new_v4(),
            revision: 1,
            supported: true,
            transparency_enabled: true,
            high_contrast: false,
            energy_saver: false,
            remote_session: false,
            native_error_code: 0,
            system_backdrop: rounded,
            rounded_corners: rounded,
        }
    ));
    cx.set_global(facts);
}

#[gpui_kit::test]
fn frosted_style_tints_layers_with_the_scheme_and_solid_restores_it(
    cx: &mut gpui_kit::TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(apply_component_theme);
    for preference in ThemePreference::ALL {
        cx.update(|cx| set_appearance_preferences(preference, MaterialStyle::Solid, cx));
        let original = cx.read(|cx| gpui_kit::component::Theme::global(cx).clone());
        cx.update(|cx| set_appearance_preferences(preference, MaterialStyle::Acrylic, cx));
        cx.read(|cx| {
            let theme = gpui_kit::component::Theme::global(cx);
            let colors = &theme.colors;
            let dark = theme.mode.is_dark();
            // Chrome shows the veiled material.
            for clear in [
                colors.background,
                colors.title_bar,
                colors.sidebar,
                colors.status_bar,
                colors.list,
                colors.table,
                colors.window_border,
            ] {
                assert_eq!(clear.a, 0.0, "{preference:?}");
            }
            // Menus and dialogs are the scheme's opaque raised surface.
            assert_eq!(colors.popover.a, 1.0);
            assert_eq!(rgb_hex(colors.popover), rgb_hex(original.colors.popover));
            assert!(close(colors.group_box.a, if dark { 0.05 } else { 0.72 }));
            // Body text, accent and selection keep the scheme's hues; supporting text moves
            // part-way toward the body ink so it holds up over a translucent veil.
            assert_eq!(colors.foreground, original.colors.foreground);
            let toward_body = |frosted: gpui_kit::Hsla, solid: gpui_kit::Hsla| {
                let (f, s, b) = (frosted.to_rgb(), solid.to_rgb(), colors.foreground.to_rgb());
                (f.r - b.r).abs() + (f.g - b.g).abs() + (f.b - b.b).abs()
                    < (s.r - b.r).abs() + (s.g - b.g).abs() + (s.b - b.b).abs()
                    || solid == colors.foreground
            };
            assert!(toward_body(
                colors.muted_foreground,
                original.colors.muted_foreground
            ));
            assert!(toward_body(
                colors.tab_foreground,
                original.colors.tab_foreground
            ));
            assert_eq!(colors.primary, original.colors.primary);
            assert_eq!(
                (
                    colors.list_active.h,
                    colors.list_active.s,
                    colors.list_active.l
                ),
                (
                    original.colors.primary.h,
                    original.colors.primary.s,
                    original.colors.primary.l
                )
            );
            assert!(close(colors.list_active.a, if dark { 0.22 } else { 0.14 }));
            // The layers carry the scheme canvas.
            let layers = layers(cx);
            assert_eq!(rgb_hex(layers.opaque), rgb_hex(original.colors.background));
            assert_eq!(rgb_hex(layers.veil), rgb_hex(original.colors.background));
            assert_eq!(layers.opaque.a, 1.0);
            assert_eq!(theme.font_family, original.font_family);
            assert_eq!(theme.font_size, original.font_size);
            assert_eq!(overlay_background(cx), theme.tokens.popover.background);
            assert!(close(crate::theme::material::contour(cx).a, 0.10));
        });
        cx.update(|cx| set_appearance_preferences(preference, MaterialStyle::Solid, cx));
        cx.read(|cx| {
            let restored = gpui_kit::component::Theme::global(cx);
            assert_eq!(restored.colors, original.colors);
            assert_eq!(restored.font_family, original.font_family);
            assert_eq!(
                overlay_background(cx),
                restored.tokens.background.background
            );
            assert_eq!(
                crate::theme::material::contour(cx),
                restored.colors.window_border
            );
        });
    }
}

#[gpui_kit::test]
fn window_radius_follows_system_rounding_for_every_style(cx: &mut gpui_kit::TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(apply_component_theme);
    let radius = |cx: &mut gpui_kit::TestAppContext| {
        cx.read(|cx| gpui_kit::component::Theme::global(cx).window_radius)
    };
    // Unknown until PlatformHost reports facts: square.
    assert_eq!(radius(cx), gpui_kit::px(0.0));
    for style in MaterialStyle::ALL {
        cx.update(|cx| {
            set_appearance_preferences(ThemePreference::Dark, style, cx);
            report_corners(true, cx);
        });
        assert_eq!(
            radius(cx),
            gpui_kit::px(material::WINDOW_RADIUS),
            "{style:?}"
        );
        // Windows 10: rounded by a window region, with the same 8 DIP contour.
        cx.update(|cx| report_corners(false, cx));
        assert_eq!(
            radius(cx),
            gpui_kit::px(material::WINDOW_RADIUS),
            "{style:?}"
        );
    }
}

#[test]
fn content_layer_follows_the_slider_and_runs_lighter_on_dark() {
    for (percent, light) in [(50, 0.50), (73, 0.73), (100, 1.0)] {
        let tuning = MaterialTuning {
            content_percent: percent,
        };
        assert!(close(content_alpha(tuning, false), light));
        assert!(close(content_alpha(tuning, true), light * 0.9));
    }
    // Below the minimum the content layer stays at the minimum.
    assert!(close(
        content_alpha(
            MaterialTuning {
                content_percent: 10
            },
            false
        ),
        0.50
    ));
    assert!(close(card_stroke(false).a, 0.06));
    assert!(close(card_stroke(true).a, 0.18));
}

#[test]
fn windows_10_host_acrylic_lets_more_wallpaper_through_than_xaml_defaults() {
    for (dark, (luminosity, tint)) in [false, true].into_iter().zip(HOST_ACRYLIC) {
        // XAML window acrylic uses 85 % (light) / 96 % (dark): nearly the tint alone.
        assert!(
            luminosity >= 50 && luminosity < if dark { 96 } else { 85 },
            "dark {dark}"
        );
        assert!(tint <= 20, "dark {dark}");
    }
}

#[gpui_kit::test]
fn windows_10_host_acrylic_keeps_every_text_role_readable_over_any_desktop(
    cx: &mut gpui_kit::TestAppContext,
) {
    fn luminance(color: gpui_kit::Hsla) -> f32 {
        let color = color.to_rgb();
        let linear = |value: f32| {
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
    }
    cx.update(gpui_kit::init);
    cx.update(apply_component_theme);
    let contrast = |a: gpui_kit::Hsla, b: gpui_kit::Hsla| {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    };
    // The luminosity layer gives the blurred desktop the tint's brightness at its opacity and
    // keeps the desktop's hue; for grey desktops, black to white, that is exactly a mix.
    let desktops: Vec<gpui_kit::Hsla> = [0x000000, 0x404040, 0x808080, 0xC0C0C0, 0xFFFFFF]
        .into_iter()
        .map(|color| gpui_kit::rgb(color).into())
        .collect();
    // The NxTools schemes; third-party palettes keep their own published contrast.
    for preference in [
        ThemePreference::Light,
        ThemePreference::Dark,
        ThemePreference::MistLight,
        ThemePreference::MistDark,
        ThemePreference::PineLight,
        ThemePreference::PineDark,
        ThemePreference::SandLight,
        ThemePreference::SandDark,
    ] {
        cx.update(|cx| set_appearance_preferences(preference, MaterialStyle::Acrylic, cx));
        cx.read(|cx| {
            let colors = gpui_kit::component::Theme::global(cx).colors;
            let layers = layers(cx);
            let (_, luminosity, _) = host_acrylic(cx);
            let luminosity = f32::from(luminosity) / 100.0;
            let roles = [
                ("body", colors.foreground, 7.0),
                ("secondary", colors.tab_foreground, 4.5),
                ("muted", colors.muted_foreground, 4.5),
            ];
            for desktop in &desktops {
                let acrylic = composite(
                    gpui_kit::Hsla {
                        a: luminosity,
                        ..layers.opaque
                    },
                    *desktop,
                );
                // The clearest surface: the veil alone (chrome, transient panels, slider minimum).
                let surface = composite(layers.veil, acrylic);
                for (role, text, floor) in roles {
                    let ratio = contrast(text, surface);
                    assert!(
                        ratio >= floor,
                        "{preference:?} {role} over {desktop:?}: {ratio:.2} < {floor}"
                    );
                }
            }
        });
    }
}
