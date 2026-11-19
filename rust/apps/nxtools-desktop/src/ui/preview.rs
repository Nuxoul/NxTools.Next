#[path = "preview_rich.rs"]
mod rich;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use gpui_kit::component::scroll::ScrollableElement;
use gpui_kit::{
    Animation, AnimationExt, Context, Div, ObjectFit, Render, WeakEntity, Window, div,
    ease_out_quint, img, prelude::*, px,
};
use nxtools_clipboard::{ClipboardItem, ClipboardKind};
use unicode_width::UnicodeWidthStr;

use crate::clipboard::ClipboardApplication;
use crate::theme::{Theme, UI_MONO_FONT_FAMILY};
use crate::ui::metrics::RADIUS_CONTROL;
use crate::ui::typography::{TEXT_BODY, TEXT_LABEL, active_ui_font_family};

const TEXT_LINE_HEIGHT: f32 = 20.0;
const META_LINE_HEIGHT: f32 = 16.0;
// Matches the rendered 1px border and 10px inner inset on both axes.
const WINDOW_HORIZONTAL_CHROME: f32 = 22.0;
const WINDOW_VERTICAL_CHROME: f32 = 22.0;
const CONTENT_META_GAP: f32 = 8.0;
const IMAGE_MAX_CONTENT_WIDTH: f32 = 440.0;
const IMAGE_MAX_CONTENT_HEIGHT: f32 = 340.0;
pub(crate) const PREVIEW_APPEAR_DURATION: Duration = Duration::from_millis(130);

pub(crate) fn preview_shell(theme: Theme) -> Div {
    div()
        .debug_selector(|| "preview-shell".into())
        .size_full()
        .overflow_hidden()
        .border_1()
        .border_color(theme.window_border)
        .bg(theme.canvas)
        .text_color(theme.text)
        .flex()
        .flex_col()
}

pub(crate) fn preview_media_surface(theme: Theme) -> Div {
    div()
        .debug_selector(|| "preview-media-surface".into())
        .overflow_hidden()
        .rounded(px(RADIUS_CONTROL))
        .bg(theme.surface)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PreviewLayout {
    pub window_width: f32,
    pub window_height: f32,
    pub body_height: f32,
    pub requires_vertical_scroll: bool,
    pub maximum_height: f32,
    pub image: Option<super::preview_image::ImageViewport>,
}

type MeasuredLayoutCallback = Box<dyn FnOnce(PreviewLayout, &mut gpui_kit::App)>;

pub(crate) struct ClipboardPreview {
    item: ClipboardItem,
    layout: PreviewLayout,
    theme: Theme,
    application: WeakEntity<ClipboardApplication>,
    appearance_generation: u64,
    rich: rich::PreparedText,
    measured_for: Option<(rich::ViewingMode, f32, gpui_kit::SharedString)>,
    measured_callback: Option<MeasuredLayoutCallback>,
    viewport_body_height: f32,
    text_scroll: gpui_kit::ScrollHandle,
    image_scroll: gpui_kit::ScrollHandle,
}

impl ClipboardPreview {
    pub(crate) fn new(
        item: ClipboardItem,
        layout: PreviewLayout,
        application: WeakEntity<ClipboardApplication>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            rich: rich::PreparedText::new(&item, rich::ViewingMode::Auto, window, cx),
            item,
            layout,
            theme: Theme::zed_dark(),
            application,
            appearance_generation: 0,
            measured_for: None,
            measured_callback: None,
            viewport_body_height: layout.body_height,
            text_scroll: gpui_kit::ScrollHandle::new(),
            image_scroll: gpui_kit::ScrollHandle::new(),
        }
    }

    pub(crate) fn set_item(
        &mut self,
        item: ClipboardItem,
        layout: PreviewLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.rich = rich::PreparedText::new(&item, rich::ViewingMode::Auto, window, cx);
        self.item = item;
        self.layout = layout;
        self.appearance_generation = self.appearance_generation.wrapping_add(1);
        self.measured_for = None;
        self.text_scroll
            .set_offset(gpui_kit::point(px(0.0), px(0.0)));
        self.image_scroll
            .set_offset(gpui_kit::point(px(0.0), px(0.0)));
        cx.notify();
    }
    pub(crate) fn on_measured_layout(
        &mut self,
        callback: impl FnOnce(PreviewLayout, &mut gpui_kit::App) + 'static,
    ) {
        self.measured_for = None;
        self.measured_callback = Some(Box::new(callback));
    }
    pub(crate) fn set_measured_layout(&mut self, layout: PreviewLayout, cx: &mut Context<Self>) {
        self.layout = layout;
        cx.notify();
    }
    pub(super) fn body_viewport_height(&self) -> f32 {
        self.viewport_body_height
    }
    fn measure_rendered_layout(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use gpui_kit::AvailableSpace;
        let width = self.layout.window_width - WINDOW_HORIZONTAL_CHROME;
        let family = active_ui_font_family(cx);
        if self.item.kind != ClipboardKind::Text
            || self.measured_for == Some((self.rich.mode, width, family.clone()))
        {
            return;
        }
        let natural = if !super::preview_text::within_budget(&self.item.text) {
            (self.layout.maximum_height - WINDOW_VERTICAL_CHROME - CONTENT_META_GAP - 24.0)
                .max(TEXT_LINE_HEIGHT)
        } else if let Some(state) = self.rich.markdown.as_ref().or(self.rich.code.as_ref()) {
            let mut body = div()
                .font_family(family.clone())
                .w(px(width))
                .child(state.clone())
                .into_any_element();
            body.layout_as_root(
                gpui_kit::size(
                    AvailableSpace::Definite(px(width)),
                    AvailableSpace::MinContent,
                ),
                window,
                cx,
            )
            .height
            .as_f32()
            .ceil()
            .max(TEXT_LINE_HEIGHT)
        } else {
            let source = self.literal_text();
            let run = gpui_kit::TextRun {
                len: source.len(),
                font: gpui_kit::font(UI_MONO_FONT_FAMILY),
                ..Default::default()
            };
            window
                .text_system()
                .shape_text(
                    source.into(),
                    px(TEXT_BODY),
                    &[run],
                    self.rich.code.is_none().then_some(px(width)),
                    None,
                )
                .map(|lines| {
                    lines
                        .iter()
                        .map(|line| line.size(px(TEXT_LINE_HEIGHT)).height.as_f32())
                        .sum::<f32>()
                })
                .unwrap_or(self.layout.body_height)
                .max(TEXT_LINE_HEIGHT)
        };
        self.measured_for = Some((self.rich.mode, width, family));
        let maximum = self.layout.maximum_height;
        self.layout = measured_layout(self.layout.window_width, natural, 24.0, maximum / 0.7);
        if let Some(callback) = self.measured_callback.take() {
            let layout = self.layout;
            cx.defer(move |cx| callback(layout, cx));
        }
    }
}

impl Render for ClipboardPreview {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        crate::ui::motion::observe_motion(window, cx);
        crate::theme::observe_theme(window, cx);
        self.theme = Theme::active(cx);
        let theme = self.theme;
        self.measure_rendered_layout(window, cx);
        self.viewport_body_height = (window.viewport_size().height.as_f32()
            - WINDOW_VERTICAL_CHROME
            - CONTENT_META_GAP
            - if self.item.kind == ClipboardKind::Text {
                24.0
            } else {
                meta_height(
                    &self.item,
                    self.layout.window_width - WINDOW_HORIZONTAL_CHROME,
                )
            })
        .max(TEXT_LINE_HEIGHT);
        let layout = self.layout;
        let application = self.application.clone();
        let appearance_generation = self.appearance_generation;
        let meta = preview_meta(&self.item);
        let content = match self.item.kind {
            ClipboardKind::Text => {
                if let Some(body) = self.rich_text_body(cx) {
                    body
                } else {
                    let body = div()
                        .w_full()
                        .h(px(self.body_viewport_height()))
                        .flex_none()
                        .overflow_hidden()
                        .font_family(UI_MONO_FONT_FAMILY)
                        .text_size(px(TEXT_BODY))
                        .line_height(px(TEXT_LINE_HEIGHT))
                        .whitespace_normal()
                        .child(self.literal_text());
                    body.overflow_y_scrollbar().into_any_element()
                }
            }
            ClipboardKind::Image => {
                let source = existing_image_source(&self.item.image_path);
                let image = layout.image.unwrap_or(super::preview_image::ImageViewport {
                    content_width: layout.window_width - WINDOW_HORIZONTAL_CHROME,
                    content_height: layout.body_height,
                    scroll_x: false,
                    scroll_y: false,
                });
                preview_media_surface(theme)
                    .id("clipboard-image-viewport")
                    .debug_selector(|| "clipboard-image-viewport".into())
                    .w_full()
                    .h(px(layout.body_height))
                    .flex_none()
                    .track_scroll(&self.image_scroll)
                    .when(image.scroll_x, |viewport| viewport.overflow_x_scroll())
                    .when(image.scroll_y, |viewport| viewport.overflow_y_scroll())
                    .child(
                        div()
                            .flex_none()
                            .debug_selector(|| "clipboard-image-content".into())
                            .w(px(image.content_width))
                            .h(px(image.content_height))
                            .when_some(source, |content, source| {
                                content
                                    .child(img(source).size_full().object_fit(ObjectFit::Contain))
                            }),
                    )
                    .when(!Path::new(&self.item.image_path).is_file(), |viewport| {
                        viewport.child("原图片文件已不存在")
                    })
                    .into_any_element()
            }
            ClipboardKind::Files => {
                let body = div()
                    .w_full()
                    .h(px(layout.body_height))
                    .flex_none()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .children(self.item.file_paths.iter().cloned().map(|path| {
                        div()
                            .font_family(UI_MONO_FONT_FAMILY)
                            .text_size(px(TEXT_BODY))
                            .line_height(px(TEXT_LINE_HEIGHT))
                            .whitespace_normal()
                            .child(path)
                    }));
                if layout.requires_vertical_scroll {
                    body.overflow_y_scrollbar().into_any_element()
                } else {
                    body.into_any_element()
                }
            }
        };

        let preview = preview_shell(theme)
            .id("clipboard-preview-root")
            .debug_selector(|| "clipboard-preview-root".into())
            .p(px(10.0))
            .gap(px(CONTENT_META_GAP))
            .on_hover(move |hovered, _window, cx| {
                let application = application.clone();
                let hovered = *hovered;
                // A preview render/hover can run while ClipboardApplication is leased.
                // Defer even the fast path; never write back into its current lease.
                cx.defer(move |cx| {
                    let _ = application.update(cx, |application, cx| {
                        application.set_preview_pointer_inside(hovered, cx);
                    });
                });
            })
            .child(content)
            .when(!meta.is_empty(), |preview| {
                let text = self.item.kind == ClipboardKind::Text;
                preview.child(
                    div()
                        .debug_selector(|| "clipboard-preview-metadata".into())
                        .w_full()
                        .flex_none()
                        .min_h(px(if text { 24.0 } else { META_LINE_HEIGHT }))
                        .flex()
                        .items_center()
                        .gap_2()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .text_size(px(TEXT_LABEL))
                                .line_height(px(META_LINE_HEIGHT))
                                .text_color(theme.text_muted)
                                .when(text || self.item.kind == ClipboardKind::Image, |node| {
                                    node.truncate()
                                })
                                .child(meta),
                        )
                        .when(text, |row| row.child(self.text_controls(cx))),
                )
            });
        if crate::ui::motion::motion_allowed(cx) {
            preview
                .with_animation(
                    ("clipboard-preview-appear", appearance_generation),
                    Animation::new(PREVIEW_APPEAR_DURATION).with_easing(ease_out_quint()),
                    |preview, delta| {
                        let (opacity, vertical_offset) = preview_appearance(delta);
                        preview.opacity(opacity).relative().top(px(vertical_offset))
                    },
                )
                .into_any_element()
        } else {
            preview.into_any_element()
        }
    }
}

fn preview_appearance(delta: f32) -> (f32, f32) {
    (0.76 + 0.24 * delta, 4.0 * (1.0 - delta))
}

pub(crate) fn preview_layout(
    item: &ClipboardItem,
    working_width: f32,
    working_height: f32,
) -> PreviewLayout {
    if item.kind == ClipboardKind::Image
        && item.image_width > 0
        && item.image_height > 0
        && Path::new(&item.image_path).is_file()
    {
        return image_layout(item, working_width, working_height);
    }

    let body = preview_body(item);
    let width = text_width(&body, working_width);
    let content_width = (width - WINDOW_HORIZONTAL_CHROME).max(1.0);
    let kind = if item.kind == ClipboardKind::Text {
        super::preview_text::classify(&item.text)
    } else {
        super::preview_text::TextKind::Plain
    };
    let desired_body_height =
        (estimated_text_lines(&body, content_width) as f32 * TEXT_LINE_HEIGHT).max(match kind {
            super::preview_text::TextKind::Markdown => 180.0,
            super::preview_text::TextKind::Code(_) => 140.0,
            _ => TEXT_LINE_HEIGHT,
        });
    measured_layout(
        width,
        desired_body_height,
        meta_height(item, content_width),
        working_height,
    )
}

fn measured_layout(
    width: f32,
    desired_body_height: f32,
    meta_height: f32,
    working_height: f32,
) -> PreviewLayout {
    let meta_block_height = if meta_height > 0.0 {
        CONTENT_META_GAP + meta_height
    } else {
        0.0
    };
    let measured_height = desired_body_height + meta_block_height + WINDOW_VERTICAL_CHROME;
    let maximum_height = (working_height * 0.7).clamp(180.0, 560.0);
    let window_height = measured_height.clamp(76.0, maximum_height);
    let requires_vertical_scroll = measured_height > maximum_height;
    let body_height =
        (window_height - meta_block_height - WINDOW_VERTICAL_CHROME).max(TEXT_LINE_HEIGHT);
    PreviewLayout {
        window_width: width,
        maximum_height,
        window_height,
        body_height,
        requires_vertical_scroll,
        image: None,
    }
}

fn text_width(text: &str, working_width: f32) -> f32 {
    let length = text.trim().chars().count();
    let width = match length {
        0..=16 => 190.0,
        17..=36 => 240.0,
        37..=80 => 320.0,
        _ => 460.0_f32.min((working_width * 0.42).max(320.0)),
    };
    width.min((working_width - 32.0).max(190.0))
}

fn estimated_text_lines(text: &str, content_width: f32) -> usize {
    let columns = (content_width / 6.8).floor().max(1.0) as usize;
    text.lines()
        .map(|line| UnicodeWidthStr::width(line).max(1).div_ceil(columns))
        .sum::<usize>()
        .max(1)
}

fn meta_height(item: &ClipboardItem, content_width: f32) -> f32 {
    (estimated_text_lines(&preview_meta(item), content_width) as f32 * META_LINE_HEIGHT).max(
        if item.kind == ClipboardKind::Text {
            24.0
        } else {
            META_LINE_HEIGHT
        },
    )
}

pub(crate) fn preview_layout_with_image_mode(
    item: &ClipboardItem,
    available_width: f32,
    available_height: f32,
    dpi_scale: f32,
    mode: nxtools_core::ClipboardImageViewMode,
) -> PreviewLayout {
    if item.kind != ClipboardKind::Image
        || item.image_width == 0
        || item.image_height == 0
        || existing_image_source(&item.image_path).is_none()
    {
        return preview_layout(item, available_width, available_height);
    }
    let plan = super::preview_image::image_plan(
        item.image_width,
        item.image_height,
        available_width,
        available_height,
        dpi_scale,
        mode,
    );
    PreviewLayout {
        window_width: plan.window_width,
        window_height: plan.window_height,
        body_height: plan.viewport_height,
        maximum_height: available_height,
        requires_vertical_scroll: plan.image.scroll_y,
        image: Some(plan.image),
    }
}

fn image_layout(item: &ClipboardItem, working_width: f32, working_height: f32) -> PreviewLayout {
    let source_width = item.image_width.max(1) as f32;
    let source_height = item.image_height.max(1) as f32;
    let scale = (IMAGE_MAX_CONTENT_WIDTH / source_width)
        .min(IMAGE_MAX_CONTENT_HEIGHT / source_height)
        .min(1.0);
    let content_width = (source_width * scale).clamp(160.0, IMAGE_MAX_CONTENT_WIDTH);
    let desired_body_height = (source_height * scale).clamp(80.0, IMAGE_MAX_CONTENT_HEIGHT);
    let width = (content_width + WINDOW_HORIZONTAL_CHROME).min((working_width - 24.0).max(182.0));
    measured_layout(
        width,
        desired_body_height,
        meta_height(item, content_width),
        working_height,
    )
}

fn preview_kind_label(kind: ClipboardKind) -> &'static str {
    match kind {
        ClipboardKind::Text => "文本",
        ClipboardKind::Image => "图片",
        ClipboardKind::Files => "文件",
    }
}

fn preview_body(item: &ClipboardItem) -> String {
    let body = match item.kind {
        ClipboardKind::Text => item.text.trim().to_owned(),
        ClipboardKind::Image => "原图片文件已不存在".into(),
        ClipboardKind::Files => item.file_paths.join("\n"),
    };
    if body.is_empty() {
        "(空内容)".into()
    } else {
        body
    }
}

fn preview_meta(item: &ClipboardItem) -> String {
    let source = if item.source.process_name.trim().is_empty() {
        "未知来源"
    } else {
        item.source.process_name.trim()
    };
    let detail = match item.kind {
        ClipboardKind::Text => format!("{} 字符", item.text.chars().count()),
        ClipboardKind::Image => format!("{} × {}", item.image_width, item.image_height),
        ClipboardKind::Files => format!("{} 项", item.file_paths.len()),
    };
    format!(
        "{source}  ·  {}  ·  {detail}",
        preview_kind_label(item.kind)
    )
}

fn existing_image_source(path: &str) -> Option<PathBuf> {
    let path = PathBuf::from(path);
    path.is_file().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxtools_clipboard::CaptureSource;

    fn image_item(width: u32, height: u32) -> ClipboardItem {
        ClipboardItem {
            id: 1,
            kind: ClipboardKind::Image,
            text: String::new(),
            image_path: "C:/image.png".into(),
            image_width: width,
            image_height: height,
            file_paths: Vec::new(),
            source: CaptureSource {
                window_handle: 1,
                process_id: 2,
                process_name: "fixture".into(),
                process_icon_path: String::new(),
                window_title: String::new(),
            },
            captured_at_unix_ms: 1,
            last_used_at_unix_ms: 1,
            copy_count: 1,
            is_favorite: false,
            is_pinned: false,
            deleted_at_unix_ms: None,
        }
    }

    fn text_item(text: &str) -> ClipboardItem {
        let mut item = image_item(0, 0);
        item.kind = ClipboardKind::Text;
        item.text = text.into();
        item.image_path.clear();
        item
    }

    #[gpui_kit::test]
    fn image_modes_real_viewport_scrolls_to_end_and_fit_resets_offset(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        let directory = tempfile::tempdir().expect("temporary image");
        let path = directory.path().join("long.png");
        image::RgbaImage::from_pixel(60, 1200, image::Rgba([40, 120, 180, 255]))
            .save(&path)
            .expect("PNG");
        let mut item = image_item(60, 1200);
        item.image_path = path.to_string_lossy().into_owned();
        let layout = preview_layout_with_image_mode(
            &item,
            320.,
            300.,
            1.,
            nxtools_core::ClipboardImageViewMode::Auto,
        );
        assert!(layout.image.expect("image geometry").scroll_y);
        let original = item.clone();
        let (view, surface) = cx.add_window_view(|window, cx| {
            ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx)
        });
        surface.simulate_resize(gpui_kit::size(
            px(layout.window_width),
            px(layout.window_height),
        ));
        surface.refresh().expect("real image viewport");
        view.update_in(surface, |view, _, cx| {
            view.image_scroll
                .set_offset(gpui_kit::point(px(0.), px(-10000.)));
            cx.notify();
        });
        surface.refresh().expect("scroll final image region");
        let viewport = surface
            .debug_bounds("clipboard-image-viewport")
            .expect("viewport");
        let content = surface
            .debug_bounds("clipboard-image-content")
            .expect("content");
        assert!(
            content.bottom() <= viewport.bottom() + px(1.),
            "last image region reachable {content:?} {viewport:?}"
        );
        let fit = preview_layout_with_image_mode(
            &original,
            320.,
            300.,
            1.,
            nxtools_core::ClipboardImageViewMode::Fit,
        );
        view.update_in(surface, |view, window, cx| {
            view.set_item(original, fit, window, cx)
        });
        surface.simulate_resize(gpui_kit::size(px(fit.window_width), px(fit.window_height)));
        surface.refresh().expect("fit with same preview entity");
        view.read_with(surface, |view, _| {
            assert_eq!(view.image_scroll.offset(), gpui_kit::point(px(0.), px(0.)));
            assert!(!view.layout.image.expect("fit").scroll_y);
        });
        let viewport = surface
            .debug_bounds("clipboard-image-viewport")
            .expect("fit viewport");
        let content = surface
            .debug_bounds("clipboard-image-content")
            .expect("fit content");
        assert!(
            content.bottom() <= viewport.bottom() + px(1.)
                && content.right() <= viewport.right() + px(1.)
        );
    }

    #[gpui_kit::test]
    fn auxiliary_clipboard_preview_rethemes_without_replacing_content(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let item = text_item("合成只读内容");
        let layout = preview_layout(&item, 1920.0, 1080.0);
        let (view, window) = cx.add_window_view(|window, cx| {
            ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx)
        });
        window.simulate_resize(gpui_kit::size(
            px(layout.window_width),
            px(layout.window_height),
        ));
        window.refresh().expect("dark production preview");
        let bounds = window
            .debug_bounds("clipboard-preview-root")
            .expect("preview root");
        view.update_in(window, |_, _, cx| {
            crate::theme::set_theme_preference(nxtools_core::ThemePreference::Light, cx);
        });
        window.run_until_parked();
        window.refresh().expect("light production preview");
        assert!(window.update(|window, _| {
            window
                .painted_quads()
                .iter()
                .any(|quad| quad.background == gpui_kit::solid_background(Theme::light().canvas))
        }));
        assert_eq!(window.debug_bounds("clipboard-preview-root"), Some(bounds));
        assert_eq!(
            view.read_with(window, |view, _| view.item.text.clone()),
            "合成只读内容"
        );
        assert_eq!(
            view.read_with(window, |view, _| view.appearance_generation),
            0
        );
    }

    #[test]
    fn long_cjk_preview_uses_reference_width_and_never_underallocates_body() {
        let text = "重新装备时基础属性存在Roll值，玩家可以使用特定材料对装备进行重铸操作，重铸会重置装备的Roll值。".repeat(5);
        let layout = preview_layout(&text_item(&text), 1920.0, 1080.0);

        assert_eq!(layout.window_width, 460.0);
        assert!(layout.body_height >= 160.0);
        assert!(!layout.requires_vertical_scroll);
        assert!(layout.window_height >= layout.body_height + 40.0);
    }

    #[test]
    fn oversized_text_is_bounded_to_seventy_percent_and_scrolls() {
        let item = text_item(&"很长的中文预览内容".repeat(600));
        let layout = preview_layout(&item, 1920.0, 720.0);

        assert_eq!(layout.window_height, 504.0);
        assert!(layout.requires_vertical_scroll);
    }

    #[test]
    fn short_text_uses_minimum_height_slack_above_the_bottom_metadata() {
        let mut item = text_item("1234");
        item.source.process_name = "N".into();
        let layout = preview_layout(&item, 1920.0, 1080.0);
        let content_width = layout.window_width - WINDOW_HORIZONTAL_CHROME;
        let metadata = meta_height(&item, content_width);

        assert_eq!(layout.window_height, 76.0);
        assert_eq!(
            layout.body_height + CONTENT_META_GAP + metadata + WINDOW_VERTICAL_CHROME,
            layout.window_height,
        );
    }

    #[test]
    fn image_preview_preserves_reference_aspect_ratio_and_does_not_upscale() {
        let wide = image_layout(&image_item(400, 120), 1920.0, 1080.0);
        let tall = image_layout(&image_item(120, 400), 1920.0, 1080.0);

        assert_eq!(wide.body_height, 120.0);
        assert_eq!(tall.body_height, 340.0);
    }

    #[test]
    fn missing_image_is_not_sent_to_the_gpui_asset_loader() {
        assert!(existing_image_source("Z:/definitely-missing/nxtools-image.png").is_none());
        let layout = preview_layout(&image_item(4_000, 3_000), 1920.0, 1080.0);
        assert_eq!(layout.window_width, 190.0);
        assert!(layout.window_height < 120.0);
    }

    #[test]
    fn preview_appearance_is_short_generation_scoped_and_never_repeats() {
        assert_eq!(
            PREVIEW_APPEAR_DURATION,
            std::time::Duration::from_millis(130)
        );
        assert_eq!(preview_appearance(0.0), (0.76, 4.0));
        assert_eq!(preview_appearance(1.0), (1.0, 0.0));

        let source = include_str!("preview.rs");
        let render = source
            .split("impl Render for ClipboardPreview")
            .nth(1)
            .expect("preview render implementation");
        assert!(render.contains("(\"clipboard-preview-appear\", appearance_generation)"));
        assert!(render.contains(".with_animation("));
        assert!(!render.contains(".repeat()"));

        let outer_shell = render
            .split(".id(\"clipboard-preview-root\")")
            .nth(1)
            .and_then(|source| source.split(".with_animation(").next())
            .expect("preview outer shell source");
        assert!(
            !outer_shell.contains(".rounded("),
            "a rounded transparent preview root exposes the rectangular HWND corners"
        );
    }

    #[test]
    fn clipboard_preview_uses_the_shared_neutral_shell_and_media_surface() {
        let source = include_str!("preview.rs");
        let shell = source
            .split("pub(crate) fn preview_shell(")
            .nth(1)
            .and_then(|source| source.split("pub(crate) fn preview_media_surface(").next())
            .expect("shared preview shell");
        let media = source
            .split("pub(crate) fn preview_media_surface(")
            .nth(1)
            .and_then(|source| source.split("pub(crate) struct PreviewLayout").next())
            .expect("shared preview media surface");
        let render = source
            .split("impl Render for ClipboardPreview")
            .nth(1)
            .expect("clipboard preview renderer");

        assert!(shell.contains(".border_color(theme.window_border)"));
        assert!(!media.contains(".items_center()"));
        assert!(!media.contains(".justify_center()"));
        assert!(render.contains("preview_shell(theme)"));
        assert!(render.contains("preview_media_surface(theme)"));
    }

    #[gpui_kit::test]
    fn round2_text_preview_body_matches_real_font_wrapping(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        cx.update(crate::theme::apply_component_theme);
        let mut item = text_item(&"i".repeat(180));
        item.source.process_name = "fixture".into();
        let layout = preview_layout(&item, 1920.0, 1080.0);
        let body = preview_body(&item);
        let (_, window) = cx.add_window_view(move |window, cx| {
            ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx)
        });
        window.simulate_resize(gpui_kit::size(
            px(layout.window_width),
            px(layout.window_height),
        ));
        window.refresh().expect("actual preview render");
        let measured = window.update(|window, _| {
            let run = gpui_kit::TextRun {
                len: body.len(),
                font: gpui_kit::font(UI_MONO_FONT_FAMILY),
                ..Default::default()
            };
            window
                .text_system()
                .shape_text(
                    body.into(),
                    px(TEXT_BODY),
                    &[run],
                    Some(px(layout.window_width - WINDOW_HORIZONTAL_CHROME)),
                    None,
                )
                .expect("real production font wrapping")
                .iter()
                .map(|line| line.size(px(TEXT_LINE_HEIGHT)).height.as_f32())
                .sum::<f32>()
        });
        assert!(
            layout.body_height - measured <= TEXT_LINE_HEIGHT,
            "excess body slack: allocated={}, actual={measured}",
            layout.body_height
        );
        assert!(layout.body_height >= measured, "text must not be clipped");
    }

    struct HeightFeedbackMeasure {
        view: gpui_kit::Entity<ClipboardPreview>,
        height: std::rc::Rc<std::cell::Cell<f32>>,
    }
    impl Render for HeightFeedbackMeasure {
        fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            use gpui_kit::AvailableSpace;
            let (state, width) = {
                let view = self.view.read(cx);
                (
                    view.rich.markdown.clone().expect("Markdown"),
                    view.layout.window_width - WINDOW_HORIZONTAL_CHROME,
                )
            };
            let mut body = div()
                .font_family(active_ui_font_family(cx))
                .w(px(width))
                .child(state)
                .into_any_element();
            self.height.set(
                body.layout_as_root(
                    gpui_kit::size(
                        AvailableSpace::Definite(px(width)),
                        AvailableSpace::MinContent,
                    ),
                    window,
                    cx,
                )
                .height
                .as_f32(),
            );
            div().size_full().child(self.view.clone())
        }
    }

    #[gpui_kit::test]
    fn height_feedback_preview_rich_layout_fits_without_language_control(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use gpui_kit::AppContext as _;
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let source = "价格：[链接](https://example.invalid/product-12345678)\n\n第二段 **加粗**\n\n第三段\n\n第四段\n\n最后一行";
        let item = text_item(source);
        let layout = preview_layout(&item, 1920.0, 1080.0);
        let height = std::rc::Rc::new(std::cell::Cell::new(0.0));
        let measured = height.clone();
        let retained = std::rc::Rc::new(std::cell::RefCell::new(None));
        let slot = retained.clone();
        let (_, window) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx)
            });
            *slot.borrow_mut() = Some(view.clone());
            HeightFeedbackMeasure {
                view,
                height: measured,
            }
        });
        let view = retained.borrow().clone().expect("preview");
        window.simulate_resize(gpui_kit::size(
            px(layout.window_width),
            px(layout.window_height),
        ));
        window.refresh().expect("preview initial render");
        let natural = px(height.get());
        assert!(
            natural > px(0.0),
            "real official layout, not an unmeasured zero"
        );
        let actual = view.read_with(window, |view, _| view.layout);
        assert!(
            px(actual.body_height) >= natural,
            "allocated {} but official Markdown needs {natural:?}",
            actual.body_height
        );
        let nodes = window.update(|window, _| gpui_kit::base::test_support::snapshots(window));
        assert!(
            !nodes.iter().any(|node| node
                .path()
                .iter()
                .any(|id| id == &gpui_kit::ElementId::from("clipboard-preview-language"))),
            "no language/Markdown button"
        );
    }

    #[gpui_kit::test]
    fn rich_preview_production_markdown_has_official_rendered_body_and_preserves_source(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let source = "# Heading\n\n**Strong** and `code`.\n\n| A | B |\n| --- | --- |\n| 1 | 2 |";
        let item = text_item(source);
        let layout = preview_layout(&item, 1920.0, 1080.0);
        let (view, window) = cx.add_window_view(|window, cx| {
            ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx)
        });
        window.simulate_resize(gpui_kit::size(
            px(layout.window_width),
            px(layout.window_height),
        ));
        window.refresh().expect("production rich preview");
        assert!(
            window.debug_bounds("clipboard-preview-markdown").is_some(),
            "actual Markdown descendant, not literal text"
        );
        let markdown = view.read_with(window, |view, _| {
            view.rich.markdown.clone().expect("actual official state")
        });
        for _ in 0..200 {
            window.run_until_parked();
            if markdown.read_with(window, |state, _| !state.rendered_text().is_empty()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        markdown.read_with(window, |state, _| {
            let text = state.rendered_text();
            assert!(text.as_str().contains("Heading") && text.as_str().contains("Strong"));
            assert!(
                !text.as_str().contains("**Strong**"),
                "assert actual parsed result, not a wrapper selector"
            );
        });
        assert_eq!(
            view.read_with(window, |view, _| view.item.text.clone()),
            source
        );
    }

    #[gpui_kit::test]
    fn feedback_preview_defaults_to_markdown_without_control_click(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let source = "_强调_ 与 `代码`\n\n1. 第一项\n2. 第二项";
        let item = text_item(source);
        let layout = preview_layout(&item, 1920.0, 1080.0);
        let (view, window) = cx.add_window_view(|window, cx| {
            ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx)
        });
        window.simulate_resize(gpui_kit::size(
            px(layout.window_width),
            px(layout.window_height),
        ));
        window.refresh().expect("initial preview");
        let state = view.read_with(window, |view, _| {
            view.rich
                .markdown
                .clone()
                .expect("default Markdown, not classifier-gated literal")
        });
        for _ in 0..200 {
            window.run_until_parked();
            if state.read_with(window, |state, _| !state.rendered_text().is_empty()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        state.read_with(window, |state, _| {
            let rendered = state.rendered_text();
            assert!(rendered.as_str().contains("强调") && rendered.as_str().contains("第一项"));
            assert!(!rendered.as_str().contains("_强调_") && !rendered.as_str().contains("`代码`"));
        });
        assert_eq!(
            view.read_with(window, |view, _| view.item.text.clone()),
            source
        );
    }

    #[gpui_kit::test]
    fn rich_preview_production_code_uses_official_editor_without_changing_payload(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let source = "fn main() { println!(\"unchanged\"); }";
        let item = text_item(source);
        let layout = preview_layout(&item, 1920.0, 1080.0);
        let (view, window) = cx.add_window_view(|window, cx| {
            ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx)
        });
        window.simulate_resize(gpui_kit::size(
            px(layout.window_width),
            px(layout.window_height),
        ));
        window.refresh().expect("official readonly code");
        assert!(
            window.debug_bounds("clipboard-preview-code").is_some(),
            "real code editor descendant"
        );
        assert_eq!(
            view.read_with(window, |view, _| view.item.text.clone()),
            source
        );
        view.update_in(window, |view, window, cx| {
            let state = view.rich.code.clone().expect("official editor state");
            let editor = state.read(cx).editor.clone();
            assert_eq!(state.read(cx).language(), Some("rust"));
            editor.update(cx, |state, cx| {
                assert!(state.is_readonly());
                let original = state.source();
                use gpui_kit::EntityInputHandler as _;
                state.replace_text_in_range(None, "MUTATION", window, cx);
                assert_eq!(
                    state.source(),
                    original,
                    "readonly boundary rejects editing"
                );
            });
            let mut highlighter = gpui_kit::component::highlighter::SyntaxHighlighter::new("rust");
            assert!(highlighter.update(
                None,
                &gpui_kit::component::input::Rope::from(source),
                Some(Duration::from_millis(50))
            ));
            let styles = highlighter.styles(
                &(0..source.len()),
                gpui_kit::component::Theme::global(cx)
                    .highlight_theme
                    .as_ref(),
            );
            assert!(
                styles
                    .iter()
                    .any(|(_, style)| style.color.is_some() || style.font_weight.is_some()),
                "real grammar produces syntax styles"
            );
        });
    }

    #[gpui_kit::test]
    fn rich_preview_source_toggle_and_reused_item_stay_local(cx: &mut gpui_kit::TestAppContext) {
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let source = "fn main() { println!(\"keep bytes\"); }";
        let item = text_item(source);
        let layout = preview_layout(&item, 1920.0, 1080.0);
        let (view, window) = cx.add_window_view(|window, cx| {
            ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx)
        });
        window.simulate_resize(gpui_kit::size(
            px(layout.window_width),
            px(layout.window_height),
        ));
        window.refresh().expect("actual controls");
        let source_button = window
            .debug_bounds("clipboard-preview-source")
            .expect("official source action");
        window.simulate_click(source_button.center(), Default::default());
        window.run_until_parked();
        view.read_with(window, |view, _| {
            assert_eq!(view.rich.mode, rich::ViewingMode::Source);
            assert_eq!(view.literal_text(), source);
            assert!(view.rich.code.is_none());
        });
        view.update_in(window, |view, window, cx| {
            view.change_viewing_mode(rich::ViewingMode::Auto, window, cx)
        });
        view.read_with(window, |view, cx| {
            assert_eq!(
                view.rich
                    .code
                    .as_ref()
                    .expect("automatic editor")
                    .read(cx)
                    .language(),
                Some("rust")
            );
            assert_eq!(view.item.text, source);
        });
        let next = text_item("ordinary text remains unchanged");
        let next_layout = preview_layout(&next, 1920.0, 1080.0);
        view.update_in(window, |view, window, cx| {
            view.set_item(next, next_layout, window, cx)
        });
        view.read_with(window, |view, _| {
            assert_eq!(view.rich.mode, rich::ViewingMode::Auto);
            assert!(view.rich.code.is_none() && view.rich.markdown.is_some());
        });
    }

    #[gpui_kit::test]
    fn rich_preview_resources_never_reach_http_and_long_markdown_scrolls(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let requests = Arc::new(AtomicUsize::new(0));
        let counted = requests.clone();
        cx.update(|cx| {
            cx.set_http_client(gpui_kit::http_client::FakeHttpClient::create(move |_| {
                counted.fetch_add(1, Ordering::SeqCst);
                async { Err(anyhow::anyhow!("unexpected fixture resource request")) }
            }))
        });
        let source = format!(
            "# Title\n\n![remote](https://example.invalid/private)\n![local](file:///C:/private.png)\n\n<img src='https://example.invalid/html'>\n\n{}\nLAST",
            "paragraph **strong**\n\n".repeat(80)
        );
        let item = text_item(&source);
        let layout = preview_layout(&item, 1920.0, 720.0);
        let (view, window) = cx.add_window_view(|window, cx| {
            ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx)
        });
        window.simulate_resize(gpui_kit::size(
            px(layout.window_width),
            px(layout.window_height),
        ));
        let state = view.read_with(window, |view, _| {
            view.rich.markdown.clone().expect("safe official Markdown")
        });
        for _ in 0..200 {
            window.run_until_parked();
            window.refresh().expect("rich production frame");
            if state.read_with(window, |state, _| {
                state.rendered_text().as_str().contains("LAST")
            }) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        view.update_in(window, |view, window, cx| {
            assert!(state.read(cx).rendered_text().contains("LAST"));
            view.text_scroll
                .set_offset(gpui_kit::point(px(0.), px(-100_000.)));
            cx.notify();
            window.refresh();
        });
        for _ in 0..4 {
            window.refresh().expect("scroll to last paragraph");
            window.run_until_parked();
        }
        view.read_with(window, |view, cx| {
            let viewport = view.text_scroll.bounds();
            let document = state.read(cx).bounds();
            assert!(view.text_scroll.offset().y < px(0.));
            assert!(
                document.bottom() <= viewport.bottom() + px(1.),
                "document final line is reachable: {document:?} vs {viewport:?}"
            );
        });
        assert_eq!(
            requests.load(Ordering::SeqCst),
            0,
            "document resources must never reach GPUI HTTP loader"
        );
        let body = window
            .debug_bounds("clipboard-preview-markdown")
            .expect("body");
        let footer = window
            .debug_bounds("clipboard-preview-metadata")
            .expect("footer");
        assert!(body.bottom() <= footer.top() && footer.bottom() <= px(layout.window_height));
        assert_eq!(
            view.read_with(window, |view, _| view.item.text.clone()),
            source
        );
    }

    #[gpui_kit::test]
    fn rich_preview_single_source_control_keeps_original_entity(cx: &mut gpui_kit::TestAppContext) {
        use gpui_kit::AppContext as _;
        cx.update(gpui_kit::init);
        cx.executor().allow_parking();
        let source = "fn main() { println!(\"source\"); }";
        let item = text_item(source);
        let layout = preview_layout(&item, 1920.0, 1080.0);
        let retained = std::rc::Rc::new(std::cell::RefCell::new(None));
        let slot = retained.clone();
        let (_, window) = cx.add_window_view(|window, cx| {
            let view = cx.new(|cx| {
                ClipboardPreview::new(item, layout, WeakEntity::new_invalid(), window, cx)
            });
            *slot.borrow_mut() = Some(view.clone());
            gpui_kit::base::Root::new(view, window, cx)
        });
        let view = retained.borrow().clone().expect("production view");
        let identity = view.entity_id();
        window.simulate_resize(gpui_kit::size(
            px(layout.window_width),
            px(layout.window_height),
        ));
        window.refresh().expect("preview controls");
        for expected in [rich::ViewingMode::Source, rich::ViewingMode::Auto] {
            let trigger = window
                .debug_bounds("clipboard-preview-source")
                .expect("only source toggle");
            window.simulate_click(trigger.center(), Default::default());
            window.run_until_parked();
            window.refresh().expect("source/render toggle");
            view.read_with(window, |view, _| {
                assert_eq!(view.rich.mode, expected);
                assert_eq!(view.item.text, source);
            });
        }
        assert_eq!(view.entity_id(), identity);
    }

    #[gpui_kit::test]
    fn rich_preview_automatic_languages_have_actual_grammar_and_styles(
        cx: &mut gpui_kit::TestAppContext,
    ) {
        cx.update(gpui_kit::init);
        rich::ensure_preview_grammars();
        let samples = [
            ("rust", "fn main() { let x = 1; }"),
            ("python", "def main():\n    return 1"),
            ("javascript", "const x = () => 1;"),
            ("typescript", "interface User { name: string; }"),
            ("tsx", "const view = <div>Hello</div>;"),
            ("c", "int main(void) { return 0; }"),
            ("cpp", "int main() { std::cout << 1; }"),
            ("csharp", "public class A { public int X = 1; }"),
            ("java", "public class A { int x = 1; }"),
            ("go", "package main\nfunc main() {}"),
            ("json", "{\"enabled\":true}"),
            ("sql", "SELECT name FROM users;"),
            ("bash", "#!/bin/bash\necho \"hello\""),
            ("html", "<div class=\"x\">Hello</div>"),
            ("css", ".x { color: red; }"),
        ];
        cx.read(|cx| {
            let mut missing_styles = Vec::new();
            for (language, source) in samples {
                let mut highlighter =
                    gpui_kit::component::highlighter::SyntaxHighlighter::new(language);
                assert_eq!(
                    highlighter.language().as_ref(),
                    language,
                    "selected grammar available"
                );
                assert!(
                    highlighter.update(
                        None,
                        &gpui_kit::component::input::Rope::from(source),
                        Some(Duration::from_millis(100))
                    ),
                    "bounded parse {language}"
                );
                assert!(
                    highlighter.tree().is_some(),
                    "actual parser tree {language}"
                );
                let styles = highlighter.styles(
                    &(0..source.len()),
                    gpui_kit::component::Theme::global(cx)
                        .highlight_theme
                        .as_ref(),
                );
                if !styles
                    .iter()
                    .any(|(_, style)| style.color.is_some() || style.font_weight.is_some())
                {
                    missing_styles.push(language);
                }
            }
            assert!(
                missing_styles.is_empty(),
                "actual highlight styles missing: {missing_styles:?}"
            );
        });
    }
}
