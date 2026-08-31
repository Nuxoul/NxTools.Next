use gpui::{Rgba, hsla, rgb};

pub(crate) use crate::ui::typography::{UI_FONT_FAMILY, UI_MONO_FONT_FAMILY};

#[derive(Clone, Copy)]
pub struct Theme {
    pub canvas: Rgba,
    pub surface: Rgba,
    pub surface_raised: Rgba,
    pub surface_hover: Rgba,
    #[allow(dead_code)] // Shared semantic token consumed as later surfaces migrate.
    pub surface_pressed: Rgba,
    pub border_subtle: Rgba,
    pub border: Rgba,
    pub border_active: Rgba,
    pub text: Rgba,
    pub text_secondary: Rgba,
    pub text_muted: Rgba,
    #[allow(dead_code)] // Shared semantic token consumed as later surfaces migrate.
    pub text_disabled: Rgba,
    pub accent: Rgba,
    pub accent_soft: Rgba,
    pub accent_text: Rgba,
    pub success: Rgba,
    #[allow(dead_code)] // Shared semantic token consumed as later surfaces migrate.
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
    #[allow(dead_code)] // Shared semantic token consumed as later surfaces migrate.
    pub focus_ring: Rgba,
}

impl Theme {
    pub fn zed_dark() -> Self {
        Self {
            canvas: rgb(0x1e1f22),
            surface: rgb(0x242529),
            surface_raised: rgb(0x2a2b2f),
            surface_hover: rgb(0x303238),
            surface_pressed: rgb(0x35373d),
            border_subtle: rgb(0x34363b),
            border: rgb(0x44474f),
            border_active: rgb(0x4791bf),
            text: rgb(0xe0e0e0),
            text_secondary: rgb(0xb8bac0),
            text_muted: rgb(0x858991),
            text_disabled: rgb(0x64676e),
            accent: rgb(0x4791bf),
            accent_soft: rgb(0x273744),
            accent_text: rgb(0xd6e8f5),
            success: rgb(0x6fbf8f),
            warning: rgb(0xd6a85f),
            danger: rgb(0xff6b78),
            danger_soft: rgb(0x35252a),
            image_well: rgb(0x191a1d),
            window_border: zed_hsla(225.0, 13.0, 12.0),
            element: zed_hsla(223.0, 13.0, 21.0),
            element_selected: zed_hsla(224.0, 11.3, 26.1),
            icon: zed_hsla(222.9, 9.9, 86.1),
            icon_muted: zed_hsla(220.0, 12.1, 66.1),
            placeholder: zed_hsla(220.0, 6.6, 44.5),
            focus_ring: rgb(0x4791bf),
        }
    }
}

fn zed_hsla(hue: f32, saturation: f32, lightness: f32) -> Rgba {
    hsla(hue / 360.0, saturation / 100.0, lightness / 100.0, 1.0).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_theme_matches_the_reference_palette() {
        let theme = Theme::zed_dark();
        assert_eq!(theme.canvas, rgb(0x1e1f22));
        assert_eq!(theme.surface, rgb(0x242529));
        assert_eq!(theme.surface_raised, rgb(0x2a2b2f));
        assert_eq!(theme.text, rgb(0xe0e0e0));
        assert_eq!(theme.accent, rgb(0x4791bf));
    }

    #[test]
    fn launcher_semantic_colors_follow_zeds_dark_roles() {
        let theme = Theme::zed_dark();

        assert_eq!(theme.window_border, zed_hsla(225.0, 13.0, 12.0));
        assert_eq!(theme.element, zed_hsla(223.0, 13.0, 21.0));
        assert_eq!(theme.element_selected, zed_hsla(224.0, 11.3, 26.1));
        assert_eq!(theme.icon_muted, zed_hsla(220.0, 12.1, 66.1));
    }

    #[test]
    fn popup_typography_uses_one_compact_scale() {
        // The legacy popup scale (13.5/13.0/10.5 plus the 4px compact radius)
        // retired into ui::typography and ui::metrics during the CL-02
        // migration; the shared ui-token-boundaries gate now rejects any
        // reintroduction.
        assert_eq!(super::super::ui::typography::TEXT_BODY, 12.0);
        assert_eq!(super::super::ui::typography::TEXT_META, 10.0);
    }
}
