use std::cell::RefCell;

use gpui_kit::component::highlighter::{HighlightTheme, SyntaxHighlighter};
use gpui_kit::component::{ActiveTheme as _, Rope};
use gpui_kit::{
    Bounds, DevicePixels, HeadlessAppContext, HeadlessAtlas, Hsla, Pixels, PlatformAtlas,
    PlatformHeadlessRenderer, Scene, TestAppContext, point,
};

use super::*;

// Observe the glyph colors submitted by the real reader render, independently
// of a compositor or a screenshot backend.
type Glyphs = Rc<RefCell<Vec<(gpui_kit::Point<gpui_kit::ScaledPixels>, Hsla)>>>;
struct GlyphRecorder {
    glyphs: Glyphs,
    atlas: Arc<HeadlessAtlas>,
}

impl PlatformHeadlessRenderer for GlyphRecorder {
    fn render_scene_to_image(
        &mut self,
        scene: &Scene,
        size: gpui_kit::Size<DevicePixels>,
    ) -> gpui_kit::Result<image::RgbaImage> {
        self.render_scene(scene, size)?;
        Ok(image::RgbaImage::new(0, 0))
    }

    fn render_scene(
        &mut self,
        scene: &Scene,
        _: gpui_kit::Size<DevicePixels>,
    ) -> gpui_kit::Result<()> {
        let mut glyphs = self.glyphs.borrow_mut();
        glyphs.clear();
        glyphs.extend(
            scene
                .monochrome_sprites
                .iter()
                .map(|glyph| (glyph.bounds.center(), glyph.color)),
        );
        glyphs.extend(
            scene
                .subpixel_sprites
                .iter()
                .map(|glyph| (glyph.bounds.center(), glyph.color)),
        );
        Ok(())
    }

    fn sprite_atlas(&self) -> Arc<dyn PlatformAtlas> {
        self.atlas.clone()
    }
}

fn colors_in(glyphs: &Glyphs, bounds: Bounds<Pixels>, scale: f32) -> Vec<Hsla> {
    let mut colors = Vec::new();
    for (glyph, color) in glyphs.borrow().iter() {
        let logical = point(
            px(f64::from(glyph.x) as f32 / scale),
            px(f64::from(glyph.y) as f32 / scale),
        );
        if bounds.contains(&logical) && !colors.contains(color) {
            colors.push(*color);
        }
    }
    colors
}

fn syntax_styles(
    source: &str,
    language: &str,
    theme: &HighlightTheme,
) -> Vec<(Range<usize>, Hsla)> {
    let rope = Rope::from(source);
    let mut highlighter = SyntaxHighlighter::new(language);
    highlighter.update(None, &rope, None);
    highlighter
        .styles(&(0..source.len()), theme)
        .into_iter()
        .filter_map(|(range, style)| style.color.map(|color| (range, color)))
        .collect()
}

#[gpui_kit::test]
fn markdown_reader_paints_syntax_colors_in_all_modes_for_both_themes(_: &mut TestAppContext) {
    const CODE: &str = "let greeting = \"sample-text\"; // a comment";
    let markdown = format!(
        "# 网络调参\n\n**拥塞窗口** 和 `RTT`\n\n```rust\n{CODE}\n```\n\n```unknown-reader-lang\n{CODE}\n```\n\n```\n{CODE}\n```\n"
    );
    let directory = tempfile::tempdir().expect("temporary Markdown fixture directory");
    let path = directory.path().join("中文高亮.md");
    std::fs::write(&path, &markdown).expect("write Markdown color fixture");

    let glyphs = Glyphs::default();
    let sink = glyphs.clone();
    let platform = gpui_kit::platform::current_platform(false);
    let mut cx =
        HeadlessAppContext::with_platform(platform.text_system(), Arc::new(()), move || {
            Ok(Some(Box::new(GlyphRecorder {
                glyphs: sink.clone(),
                atlas: Arc::new(HeadlessAtlas::default()),
            })))
        });
    cx.update(gpui_kit::init);
    cx.update(crate::theme::apply_component_theme);
    let window = cx
        .open_window(size(px(1100.), px(760.)), |window, cx| {
            cx.new(|cx| Reader::new(None, window, cx))
        })
        .expect("off-screen reader");
    cx.update_window(window.into(), |root, _, cx| {
        let reader = root.downcast::<Reader>().expect("reader entity");
        let input = document::read(path).expect("read syntax fixture");
        assert!(reader.update(cx, |reader, cx| reader.add_document(input, cx)));
    })
    .expect("load real reader document");
    cx.run_until_parked();

    for preference in [
        nxtools_core::ThemePreference::Light,
        nxtools_core::ThemePreference::Dark,
    ] {
        cx.update(|cx| crate::theme::set_theme_preference(preference, cx));
        let (rust_colors, markdown_styles) = cx.update(|cx| {
            let theme = &cx.theme().highlight_theme;
            let rust_styles = syntax_styles(CODE, "rust", theme);
            let colors = ["let", "sample-text", "a comment"].map(|token| {
                let start = CODE.find(token).expect("token in fixture");
                rust_styles
                    .iter()
                    .find(|(range, _)| range.contains(&start))
                    .expect("registered Rust grammar colored the token")
                    .1
            });
            (colors, syntax_styles(&markdown, "markdown", theme))
        });
        assert!(
            rust_colors[0] != rust_colors[1]
                && rust_colors[0] != rust_colors[2]
                && rust_colors[1] != rust_colors[2]
        );

        for mode in [edit::Mode::Preview, edit::Mode::Code, edit::Mode::Split] {
            cx.update_window(window.into(), |root, native, cx| {
                let reader = root.downcast::<Reader>().expect("reader entity");
                reader.update(cx, |reader, cx| reader.set_mode(mode, native, cx));
            })
            .expect("switch reader mode");
            cx.run_until_parked();
            let (fences, scale) = cx
                .update_window(window.into(), |root, native, cx| {
                    native.draw(cx).clear(cx);
                    let reader = root.downcast::<Reader>().expect("reader entity");
                    let tab = reader.read(cx).active_tab().expect("open document");
                    (
                        [2, 3, 4]
                            .map(|index| tab.view.read(cx).list_state().bounds_for_item(index)),
                        native.scale_factor(),
                    )
                })
                .expect("render and measure positioned fences");
            // This calls the recorder with the actual scene just painted above.
            let _ = cx
                .capture_screenshot(window.into())
                .expect("capture recorded render scene");
            if mode != edit::Mode::Code {
                let fence_colors = fences.map(|bounds| {
                    let mut bounds = bounds.expect("fixture fence is in the viewport");
                    // Start below the compact overlay's top edge, inside the code line.
                    bounds.origin.y += px(20.);
                    bounds.size.height -= px(20.);
                    colors_in(&glyphs, bounds, scale)
                });
                assert!(
                    rust_colors
                        .iter()
                        .all(|color| fence_colors[0].contains(color)),
                    "{preference:?} {mode:?} glyphs: {fence_colors:?}, expected {rust_colors:?}"
                );
                for plain in &fence_colors[1..] {
                    assert_eq!(
                        plain.len(),
                        1,
                        "unsupported/plain fences should paint readable monochrome text: {plain:?}"
                    );
                }
            }
            if mode != edit::Mode::Preview {
                let right = if mode == edit::Mode::Split {
                    315.
                } else {
                    1000.
                };
                // Left gutter and title-bar text are not syntax glyphs.
                let source_colors = colors_in(
                    &glyphs,
                    Bounds::new(point(px(55.), px(132.)), size(px(right - 55.), px(450.))),
                    scale,
                );
                let matches = source_colors
                    .iter()
                    .filter(|color| markdown_styles.iter().any(|(_, syntax)| syntax == *color))
                    .count();
                assert!(
                    matches >= 2,
                    "{preference:?} {mode:?} did not paint distinct Markdown syntax colors: {source_colors:?}"
                );
            }
        }
    }
}
