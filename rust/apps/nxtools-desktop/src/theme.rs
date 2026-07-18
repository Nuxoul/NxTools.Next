use gpui::{Rgba, rgb};

#[derive(Clone, Copy)]
pub struct Theme {
    pub canvas: Rgba,
    pub surface: Rgba,
    pub surface_raised: Rgba,
    pub surface_hover: Rgba,
    pub border_subtle: Rgba,
    pub border: Rgba,
    pub border_active: Rgba,
    pub text: Rgba,
    pub text_secondary: Rgba,
    pub text_muted: Rgba,
    pub accent: Rgba,
    pub accent_soft: Rgba,
    pub accent_text: Rgba,
    pub danger: Rgba,
    pub image_well: Rgba,
}

impl Theme {
    pub fn zed_dark() -> Self {
        Self {
            canvas: rgb(0x232423),
            surface: rgb(0x292929),
            surface_raised: rgb(0x303130),
            surface_hover: rgb(0x343534),
            border_subtle: rgb(0x363836),
            border: rgb(0x454745),
            border_active: rgb(0x56615b),
            text: rgb(0xf2f1ea),
            text_secondary: rgb(0xa9aea8),
            text_muted: rgb(0x929a94),
            accent: rgb(0x38d6c7),
            accent_soft: rgb(0x305c54),
            accent_text: rgb(0xddfbf7),
            danger: rgb(0xff6b78),
            image_well: rgb(0x202623),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_theme_matches_the_reference_palette() {
        let theme = Theme::zed_dark();
        assert_eq!(theme.canvas, rgb(0x232423));
        assert_eq!(theme.surface, rgb(0x292929));
        assert_eq!(theme.border, rgb(0x454745));
        assert_eq!(theme.text, rgb(0xf2f1ea));
        assert_eq!(theme.accent, rgb(0x38d6c7));
    }
}
