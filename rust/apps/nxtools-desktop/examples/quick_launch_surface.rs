//! Standalone Kit search-surface preview. No NxTools runtime or persisted state.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use std::borrow::Cow;

use gpui_kit::component::{
    ActiveTheme as _, FocusableExt as _, Icon, IconName, Sizable as _, Size, Theme, ThemeMode,
    ThemeRegistry,
    input::{Input, InputGroup, InputGroupAddon, InputState},
};
use gpui_kit::{
    App, AppContext as _, Bounds, BoxShadow, Context, Entity, Focusable as _, IntoElement, Render,
    Styled as _, Window, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div,
    point, prelude::*, px, rgba, size, transparent_black,
};

const WIDTH: f32 = 620.0;
const HEIGHT: f32 = 60.0;
const GUTTER: f32 = 16.0;
const RADIUS: f32 = 8.0;

fn initialize(cx: &mut App, dark: bool) {
    cx.text_system()
        .add_fonts(vec![Cow::Borrowed(include_bytes!(
            "../assets/fonts/SarasaUiSC-Regular.ttf"
        ))])
        .expect("register bundled Sarasa UI SC Regular");
    gpui_kit::init(cx);
    ThemeRegistry::global_mut(cx)
        .load_themes_from_str(include_str!("../assets/themes/catppuccin.json"))
        .expect("bundled official themes");
    let registry = ThemeRegistry::global(cx);
    let light = registry.themes()["Catppuccin Latte"].clone();
    let dark_theme = registry.themes()["Catppuccin Frappe"].clone();
    Theme::update(cx, |theme| {
        theme.light_theme = light;
        theme.dark_theme = dark_theme;
    });
    change_theme(dark, cx);
}

fn change_theme(dark: bool, cx: &mut App) {
    Theme::change(
        if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );
    Theme::update(cx, |theme| {
        theme.font_family = "Sarasa UI SC".into();
        theme.mono_font_family = "Consolas".into();
        theme.focus_ring = false;
    });
}

fn defer_transparent_root(window: &Window, cx: &mut App) {
    let handle = window.window_handle();
    cx.defer(move |cx| {
        let _ = handle.update(cx, |_, window, cx| {
            if let Some(root) = window.root::<gpui_kit::base::Root>().flatten() {
                root.update(cx, |root, cx| {
                    root.style().background = Some(transparent_black().into());
                    cx.notify();
                });
            }
        });
    });
}

struct SearchSurface {
    input: Entity<InputState>,
    dark: bool,
    shadow: bool,
}

impl SearchSurface {
    fn new(dark: bool, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("搜索应用、文件或文件夹"));
        input.focus_handle(cx).focus(window, cx);
        defer_transparent_root(window, cx);
        Self {
            input,
            dark,
            shadow: true,
        }
    }
}

impl Render for SearchSurface {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("surface-preview")
            .debug_selector(|| "surface-preview".into())
            .size_full()
            .p(px(GUTTER))
            .font_family("Sarasa UI SC")
            .capture_key_down(
                cx.listener(|this, event: &gpui_kit::KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "escape" => {
                            window.remove_window();
                            cx.stop_propagation();
                        }
                        "f2" if !event.is_held => {
                            this.dark = !this.dark;
                            let dark = this.dark;
                            let handle = window.window_handle();
                            cx.defer(move |cx| {
                                change_theme(dark, cx);
                                let _ = handle.update(cx, |_, window, cx| {
                                    defer_transparent_root(window, cx);
                                });
                            });
                            cx.stop_propagation();
                        }
                        _ => {}
                    }
                }),
            )
            .child(
                InputGroup::new("search-surface")
                    .aria_label("搜索面板示例")
                    .with_size(Size::Large)
                    .focus_ring(false)
                    .w(px(WIDTH))
                    .h(px(HEIGHT))
                    .rounded(px(RADIUS))
                    .bg(cx.theme().popover)
                    .shadow(if self.shadow {
                        vec![BoxShadow {
                            color: rgba(0x00000024).into(),
                            offset: point(px(0.0), px(3.0)),
                            blur_radius: px(8.0),
                            spread_radius: px(0.0),
                            inset: false,
                        }]
                    } else {
                        Vec::new()
                    })
                    .addon(
                        InputGroupAddon::new("search-icon")
                            .pl(px(16.0))
                            .child(Icon::new(IconName::Search).size(px(20.0))),
                    )
                    .input(
                        Input::new(&self.input)
                            .id("search-control")
                            .aria_label("速启搜索示例")
                            .h(px(58.0))
                            .pl(px(12.0))
                            .pr(px(16.0))
                            .py(px(0.0))
                            .text_size(px(20.0))
                            .line_height(px(28.0)),
                    ),
            )
    }
}

fn main() {
    let arguments: Vec<_> = std::env::args().collect();
    if arguments
        .iter()
        .any(|argument| argument == "--native-material-preview")
    {
        material::run(&arguments);
        return;
    }
    let dark = !arguments.iter().any(|argument| argument == "--light");
    let capture_directory = arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("--gpu-capture="))
        .map(std::path::PathBuf::from);
    let without_shadow = capture_directory.is_some()
        && arguments
            .iter()
            .any(|argument| argument == "--gpu-no-shadow");
    let capture_successful = std::rc::Rc::new(std::cell::Cell::new(capture_directory.is_none()));
    let capture_result = capture_successful.clone();
    println!("Kit surface preview: F2 changes theme; Escape closes. No search is executed.");
    let correlation = arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("--correlation="))
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
        .unwrap_or_else(uuid::Uuid::new_v4);
    let started = std::time::Instant::now();
    println!("ui.surface_preview.started surface=search correlation={correlation}");
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            initialize(cx, dark);
            cx.on_window_closed(move |cx, _| {
                if cx.windows().is_empty() {
                    println!("ui.surface_preview.finished surface=search correlation={correlation} elapsed_ms={}", started.elapsed().as_millis());
                    cx.quit();
                }
            })
            .detach();
            let (handle, _) = gpui_kit::open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(WIDTH + GUTTER * 2.0), px(HEIGHT + GUTTER * 2.0)),
                        cx,
                    ))),
                    titlebar: None,
                    kind: WindowKind::PopUp,
                    focus: capture_directory.is_none(),
                    is_resizable: false,
                    is_minimizable: false,
                    window_background: WindowBackgroundAppearance::Transparent,
                    app_id: Some("NxTools.SearchSurfacePreview".into()),
                    ..Default::default()
                },
                cx,
                move |window, cx| {
                    window.set_window_title("NxTools — Search Surface Preview (F2 / Esc)");
                    cx.new(|cx| {
                        let mut surface = SearchSurface::new(dark, window, cx);
                        surface.shadow = !without_shadow;
                        surface
                    })
                },
            )
            .expect("open isolated GPUI preview");
            if let Some(directory) = capture_directory {
                capture::schedule(handle, directory, without_shadow, capture_result, correlation, cx);
            } else {
                cx.activate(true);
            }
        });
    assert!(capture_successful.get(), "isolated GPU capture failed");
}

#[path = "quick_launch_surface/capture.rs"]
mod capture;

#[cfg(test)]
#[path = "quick_launch_surface/tests.rs"]
mod tests;

#[path = "quick_launch_surface/material.rs"]
mod material;
