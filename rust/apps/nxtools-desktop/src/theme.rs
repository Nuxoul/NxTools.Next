use gpui::{App, Rgba, rgb, transparent_black};

pub(crate) use crate::ui::typography::{UI_FONT_FAMILY, UI_MONO_FONT_FAMILY};

#[derive(Clone, Copy)]
pub struct Theme {
    pub canvas: Rgba,
    pub surface: Rgba,
    pub surface_raised: Rgba,
    pub surface_hover: Rgba,
    pub surface_pressed: Rgba,
    pub border_subtle: Rgba,
    pub border: Rgba,
    pub border_active: Rgba,
    pub text: Rgba,
    pub text_secondary: Rgba,
    pub text_muted: Rgba,
    #[allow(dead_code)] // Later module actions have not all migrated disabled presentation.
    pub text_disabled: Rgba,
    pub accent: Rgba,
    pub accent_hover: Rgba,
    pub accent_soft: Rgba,
    pub accent_text: Rgba,
    pub success: Rgba,
    #[allow(dead_code)] // Warning surfaces retain their existing owner-specific presentation.
    pub warning: Rgba,
    pub danger: Rgba,
    pub danger_soft: Rgba,
    pub image_well: Rgba,
    pub window_border: Rgba,
    pub element: Rgba,
    pub element_selected: Rgba,
    pub icon: Rgba,
    pub icon_muted: Rgba,
    pub placeholder: Rgba,
    pub focus_ring: Rgba,
}

impl Theme {
    // Retain the constructor name for existing consumers; the approved product
    // brief, not historical Zed measurements, now owns these semantic roles.
    pub fn zed_dark() -> Self {
        let canvas = rgb(0x18181b);
        let surface = rgb(0x1f1f23);
        let raised = rgb(0x27272a);
        let accent = rgb(0x38bdf8);
        let accent_soft = tint(canvas, accent, 0.12);
        Self {
            canvas,
            surface,
            surface_raised: raised,
            surface_hover: raised,
            surface_pressed: tint(raised, accent, 0.12),
            border_subtle: raised,
            border: tint(raised, rgb(0xa1a1aa), 0.18),
            border_active: accent,
            text: rgb(0xf4f4f5),
            text_secondary: rgb(0xa1a1aa),
            text_muted: rgb(0x71717a),
            text_disabled: tint(surface, rgb(0x71717a), 0.72),
            accent,
            accent_hover: rgb(0x7dd3fc),
            accent_soft,
            accent_text: rgb(0x38bdf8),
            success: rgb(0x6fbf8f),
            warning: rgb(0xd6a85f),
            danger: rgb(0xff6b78),
            danger_soft: tint(surface, rgb(0xff6b78), 0.14),
            image_well: rgb(0x121215),
            window_border: raised,
            element: surface,
            element_selected: tint(raised, accent, 0.14),
            icon: rgb(0xf4f4f5),
            icon_muted: rgb(0xa1a1aa),
            placeholder: rgb(0x71717a),
            focus_ring: accent,
        }
    }
}

fn tint(base: Rgba, foreground: Rgba, amount: f32) -> Rgba {
    Rgba {
        r: base.r + (foreground.r - base.r) * amount,
        g: base.g + (foreground.g - base.g) * amount,
        b: base.b + (foreground.b - base.b) * amount,
        a: 1.0,
    }
}

/// Production startup and visual fixtures use one component palette mapping.
/// Transparent window backgrounds preserve the existing native alpha contract.
pub(crate) fn apply_component_theme(cx: &mut App) {
    use crate::ui::metrics::{RADIUS_CONTROL, RADIUS_DIALOG};
    use crate::ui::typography::{TEXT_BODY, TEXT_LABEL};
    use gpui::px;
    use gpui_component::{Theme as ComponentTheme, ThemeMode};

    ComponentTheme::change(ThemeMode::Dark, None, cx);
    let visual = Theme::zed_dark();
    let component = ComponentTheme::global_mut(cx);
    component.font_family = UI_FONT_FAMILY.into();
    component.mono_font_family = UI_MONO_FONT_FAMILY.into();
    component.font_size = px(TEXT_BODY);
    component.mono_font_size = px(TEXT_LABEL);
    component.radius = px(RADIUS_CONTROL);
    component.radius_lg = px(RADIUS_DIALOG);
    component.colors.background = transparent_black();
    component.colors.window_border = transparent_black();
    component.colors.foreground = visual.text.into();
    component.colors.muted_foreground = visual.placeholder.into();
    component.colors.caret = visual.accent.into();
    component.colors.selection = visual.accent_soft.into();
    component.colors.border = visual.border.into();
    component.colors.input = visual.border.into();
    component.colors.ring = visual.focus_ring.into();
    component.colors.popover = visual.surface_raised.into();
    component.colors.popover_foreground = visual.text.into();
    component.colors.primary = visual.accent.into();
    component.colors.primary_hover = visual.accent_hover.into();
    component.colors.primary_active = visual.accent.into();
    component.colors.primary_foreground = visual.image_well.into();
    component.colors.secondary = visual.surface_raised.into();
    component.colors.secondary_hover = visual.surface_hover.into();
    component.colors.secondary_active = visual.surface_pressed.into();
    component.colors.secondary_foreground = visual.text.into();
    component.colors.accent = visual.element_selected.into();
    component.colors.accent_foreground = visual.text.into();
    component.colors.muted = visual.element.into();
    component.colors.list = visual.canvas.into();
    component.colors.list_active = visual.element_selected.into();
    component.colors.list_active_border = visual.border_subtle.into();
    component.colors.list_hover = visual.surface_hover.into();
    component.colors.scrollbar = visual.canvas.into();
    component.colors.scrollbar_thumb = visual.border.into();
    component.colors.scrollbar_thumb_hover = visual.text_muted.into();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foundation_palette_matches_the_approved_product_brief() {
        let theme = Theme::zed_dark();
        assert_eq!(theme.canvas, rgb(0x18181b));
        assert_eq!(theme.surface, rgb(0x1f1f23));
        assert_eq!(theme.surface_raised, rgb(0x27272a));
        assert_eq!(theme.image_well, rgb(0x121215));
        assert_eq!(theme.border_subtle, rgb(0x27272a));
        assert_eq!(theme.text, rgb(0xf4f4f5));
        assert_eq!(theme.text_secondary, rgb(0xa1a1aa));
        assert_eq!(theme.text_muted, rgb(0x71717a));
        assert_eq!(theme.accent, rgb(0x38bdf8));
        assert_eq!(theme.accent_hover, rgb(0x7dd3fc));
        assert_eq!(theme.focus_ring, theme.accent);
    }

    #[test]
    fn selection_and_pressed_roles_remain_distinct_from_neutral_hover() {
        let theme = Theme::zed_dark();
        assert_ne!(theme.element_selected, theme.accent_soft);
        assert_ne!(theme.element_selected, theme.surface_hover);
        assert_ne!(theme.surface_pressed, theme.surface_hover);
        assert_eq!(theme.icon_muted, theme.text_secondary);
    }

    #[gpui::test]
    fn foundation_component_theme_uses_the_same_palette(cx: &mut gpui::TestAppContext) {
        cx.update(gpui_component::init);
        cx.update(apply_component_theme);
        cx.read(|cx| {
            let component = gpui_component::Theme::global(cx);
            let visual = Theme::zed_dark();
            assert_eq!(component.colors.ring, visual.focus_ring.into());
            assert_eq!(component.colors.primary, visual.accent.into());
            assert_ne!(
                component.colors.primary_active, component.colors.primary_hover,
                "primary pressed state must remain distinct from hover"
            );
            assert_eq!(
                component.colors.primary_foreground,
                visual.image_well.into()
            );
            assert_eq!(component.font_family, UI_FONT_FAMILY);
            assert_eq!(
                component.radius,
                gpui::px(crate::ui::metrics::RADIUS_CONTROL)
            );
        });
    }

    #[test]
    fn popup_typography_uses_one_compact_scale() {
        assert_eq!(super::super::ui::typography::TEXT_BODY, 12.0);
        assert_eq!(super::super::ui::typography::TEXT_META, 10.0);
    }
}
