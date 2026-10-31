//! Pure image view geometry. All inputs/outputs are DIP except source pixels and the device cap.
use nxtools_core::ClipboardImageViewMode;
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ImageViewport {
    pub content_width: f32,
    pub content_height: f32,
    pub scroll_x: bool,
    pub scroll_y: bool,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ImagePlan {
    pub window_width: f32,
    pub window_height: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub image: ImageViewport,
}
const HORIZONTAL_CHROME: f32 = 22.0;
const VERTICAL_CHROME: f32 = 46.0;
pub(crate) fn image_plan(
    width: u32,
    height: u32,
    available_width: f32,
    available_height: f32,
    dpi_scale: f32,
    mode: ClipboardImageViewMode,
) -> ImagePlan {
    let dpi = if dpi_scale.is_finite() && dpi_scale > 0.0 {
        dpi_scale
    } else {
        1.0
    };
    let source_width = width.max(1) as f32 / dpi;
    let source_height = height.max(1) as f32 / dpi;
    let mut maximum_width = available_width.max(HORIZONTAL_CHROME + 1.0);
    let mut maximum_height = available_height.max(VERTICAL_CHROME + 1.0);
    let cap = match mode {
        ClipboardImageViewMode::Light => Some(800.0 / dpi),
        ClipboardImageViewMode::Mini => Some(500.0 / dpi),
        ClipboardImageViewMode::Compact => Some(300.0 / dpi),
        _ => None,
    };
    if let Some(cap) = cap {
        // Cap the short window axis only; the long axis keeps its available budget.
        if source_width >= source_height {
            maximum_height = maximum_height.min(cap);
        } else {
            maximum_width = maximum_width.min(cap);
        }
    }
    let body_width = (maximum_width - HORIZONTAL_CHROME).max(1.0);
    let body_height = (maximum_height - VERTICAL_CHROME).max(1.0);
    let scale_x = body_width / source_width;
    let scale_y = body_height / source_height;
    let fit = scale_x.min(scale_y).min(1.0);
    let disproportion = (source_width / source_height) / (body_width / body_height);
    let scale = match mode {
        ClipboardImageViewMode::Panorama => 1.0,
        ClipboardImageViewMode::Fit => fit,
        _ if !(0.4..=2.5).contains(&disproportion) => scale_x.max(scale_y).min(1.0),
        _ => fit,
    };
    let content_width = source_width * scale;
    let content_height = source_height * scale;
    let viewport_width = content_width.min(body_width).max(1.0);
    let viewport_height = content_height.min(body_height).max(1.0);
    ImagePlan {
        window_width: viewport_width + HORIZONTAL_CHROME,
        window_height: viewport_height + VERTICAL_CHROME,
        viewport_width,
        viewport_height,
        image: ImageViewport {
            content_width,
            content_height,
            scroll_x: content_width > viewport_width + 0.1,
            scroll_y: content_height > viewport_height + 0.1,
        },
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn image_modes_panorama_is_one_source_pixel_per_device_pixel() {
        for dpi in [1.0, 1.25, 1.5] {
            let plan = image_plan(
                2400,
                1800,
                900.,
                760.,
                dpi,
                ClipboardImageViewMode::Panorama,
            );
            assert!((plan.image.content_width * dpi - 2400.).abs() < 0.01);
            assert!((plan.image.content_height * dpi - 1800.).abs() < 0.01);
            assert!(plan.image.scroll_x && plan.image.scroll_y);
        }
    }
    #[test]
    fn image_modes_fit_is_complete_and_auto_long_images_scroll_on_only_overflow_axis() {
        for (width, height) in [(4000, 3000), (200, 10000), (10000, 200), (32, 32)] {
            let fit = image_plan(width, height, 850., 700., 1.25, ClipboardImageViewMode::Fit);
            assert!(!fit.image.scroll_x && !fit.image.scroll_y);
            assert!(
                fit.image.content_width <= fit.viewport_width + 0.01
                    && fit.image.content_height <= fit.viewport_height + 0.01
            );
            assert!(
                (fit.image.content_width / fit.image.content_height - width as f32 / height as f32)
                    .abs()
                    < 0.01
            );
        }
        let tall = image_plan(1200, 20000, 850., 700., 1., ClipboardImageViewMode::Auto);
        assert!(tall.image.scroll_y && !tall.image.scroll_x);
        let wide = image_plan(20000, 1200, 850., 700., 1., ClipboardImageViewMode::Auto);
        assert!(wide.image.scroll_x && !wide.image.scroll_y);
    }
    #[test]
    fn image_modes_short_edge_caps_whole_window_in_device_pixels_and_no_small_image_upscale() {
        for dpi in [1., 1.25, 1.5] {
            for (mode, cap) in [
                (ClipboardImageViewMode::Light, 800.),
                (ClipboardImageViewMode::Mini, 500.),
                (ClipboardImageViewMode::Compact, 300.),
            ] {
                for (width, height) in [(8000, 6000), (6000, 8000), (24000, 1000), (1000, 24000)] {
                    let plan = image_plan(width, height, 1600., 1200., dpi, mode);
                    assert!(plan.window_width.min(plan.window_height) * dpi <= cap + 0.1);
                    assert!(plan.window_width <= 1600. && plan.window_height <= 1200.);
                }
            }
        }
        for mode in ClipboardImageViewMode::ALL {
            if mode != ClipboardImageViewMode::Panorama {
                let plan = image_plan(32, 16, 900., 760., 1.5, mode);
                assert!(
                    plan.image.content_width * 1.5 <= 32.01
                        && plan.image.content_height * 1.5 <= 16.01
                );
            }
        }
    }
}
