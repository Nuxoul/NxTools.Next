use gpui::{Rgba, rgb};

#[derive(Clone, Copy)]
pub struct Theme {
    pub canvas: Rgba,
    pub surface: Rgba,
    pub surface_raised: Rgba,
    pub border: Rgba,
    pub border_active: Rgba,
    pub text: Rgba,
    pub text_muted: Rgba,
    pub accent: Rgba,
    pub accent_soft: Rgba,
    pub success: Rgba,
    pub danger: Rgba,
}

impl Theme {
    pub fn zed_dark() -> Self {
        Self {
            canvas: rgb(0x181818),
            surface: rgb(0x1e1e1e),
            surface_raised: rgb(0x252525),
            border: rgb(0x303030),
            border_active: rgb(0x454545),
            text: rgb(0xd8d8d8),
            text_muted: rgb(0x8a8a8a),
            accent: rgb(0x7aa2f7),
            accent_soft: rgb(0x283653),
            success: rgb(0x7dcfff),
            danger: rgb(0xf7768e),
        }
    }
}
