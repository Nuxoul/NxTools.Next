//! Windows 10 corner lab: compares square, `SetWindowRgn`-clipped and self-drawn rounded
//! windows in solid and frosted (accent blur) styles, and what a frosted window shows while it
//! is dragged (blur, an opaque accent tint, or a fully solid body). Two windows (main and
//! secondary) let a frosted window be dragged over another; a handle starts the drag the way
//! the product main window does (SC_MOVE) instead of through the caption hit test.
//! No NxTools runtime or persisted state is involved.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use std::{borrow::Cow, path::PathBuf};

use gpui_kit::component::{
    ActiveTheme as _, Theme, ThemeMode, ThemeRegistry, TitleBar,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{
    App, AppContext as _, Bounds, Context, Decorations, Global, Hsla, IntoElement, MouseButton,
    Render, Rgba, SharedString, Styled as _, Subscription, Window, WindowBounds, WindowOptions,
    div, prelude::*, px, size, transparent_black,
};

#[path = "win10_corner_lab/native.rs"]
mod native;
#[path = "win10_corner_lab/shadow.rs"]
mod shadow;
#[path = "win10_corner_lab/host.rs"]
mod host;

use native::{Backdrop, Corners, DragMode, Throttle};

const RADIUS: f32 = 8.0;
const MARGIN: f32 = 16.0;
/// Product Windows 10 values: chrome veil and content layer alpha, `[light, dark]`.
const VEIL: [f32; 2] = [0.70, 0.77];
const CONTENT: [f32; 2] = [0.50, 0.45];

static LOG_PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

static LOG_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(crate) fn log_line(line: String) {
    let _gate = LOG_GATE.lock();
    if let Some(path) = LOG_PATH.get()
        && let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
    {
        use std::io::Write as _;
        let _ = writeln!(file, "{line}");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Surface {
    Main,
    Secondary,
}

impl Surface {
    fn tag(self) -> &'static str {
        match self {
            Surface::Main => "main",
            Surface::Secondary => "secondary",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LabState {
    frosted: bool,
    backdrop: Backdrop,
    corners: Corners,
    drag: DragMode,
    drag_hide_shadow: bool,
    throttle: Throttle,
    dark: bool,
}

impl Global for LabState {}

fn apply_theme(state: LabState, cx: &mut App) {
    Theme::change(
        if state.dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        },
        None,
        cx,
    );
    Theme::update(cx, |theme| {
        theme.font_family = "Sarasa UI SC".into();
        theme.focus_ring = false;
        theme.window_radius = px(0.0);
        theme.colors.window_border.a = 0.0;
    });
}

fn update(cx: &mut App, edit: impl FnOnce(&mut LabState)) {
    let mut state = *cx.global::<LabState>();
    edit(&mut state);
    cx.set_global(state);
    apply_theme(state, cx);
    log_line(format!(
        "lab.state frosted={} backdrop={} corners={} drag={} throttle={} drag_hide_shadow={} dark={}",
        state.frosted,
        state.backdrop.tag(),
        state.corners.tag(),
        state.drag.tag(),
        state.throttle.tag(),
        state.drag_hide_shadow,
        state.dark
    ));
}

fn with_alpha(color: Hsla, alpha: f32) -> Hsla {
    Hsla { a: alpha, ..color }
}

fn mix_white(color: Hsla, amount: f32) -> Hsla {
    let rgb = color.to_rgb();
    let lerp = |value: f32| value + (1.0 - value) * amount;
    Rgba {
        r: lerp(rgb.r),
        g: lerp(rgb.g),
        b: lerp(rgb.b),
        a: 1.0,
    }
    .into()
}

/// Opaque canvas as 0xAABBGGRR for the drag-time accent tint.
fn abgr(color: Hsla) -> u32 {
    let rgb = color.to_rgb();
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u32;
    0xFF00_0000 | (channel(rgb.b) << 16) | (channel(rgb.g) << 8) | channel(rgb.r)
}

fn native_config(state: LabState, surface: Surface, cx: &App) -> native::Config {
    let canvas = cx.theme().background;
    native::Config {
        frosted: state.frosted,
        backdrop: state.backdrop,
        corners: state.corners,
        drag: state.drag,
        drag_hide_shadow: state.drag_hide_shadow,
        throttle: state.throttle,
        tint: abgr(canvas),
        radius_dip: RADIUS,
        margin_dip: MARGIN,
        resizable: surface == Surface::Main || surface == Surface::Secondary,
        dark: state.dark,
    }
}

fn open_surface(surface: Surface, cx: &mut App) {
    let window_size = match surface {
        Surface::Main => size(px(900.0), px(600.0)),
        Surface::Secondary => size(px(640.0), px(440.0)),
    };
    let mut bounds = Bounds::centered(None, window_size, cx);
    if surface == Surface::Secondary {
        bounds.origin.x += px(160.0);
        bounds.origin.y += px(90.0);
    }
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        is_resizable: true,
        is_minimizable: true,
        window_min_size: Some(size(px(420.0), px(300.0))),
        app_id: Some("NxTools.Win10CornerLab".into()),
        ..TitleBar::window_options()
    };
    let opened = gpui_kit::open_window(options, cx, move |window, cx| {
        window.set_window_title(&format!("NxTools Win10 圆角实验 · {}", surface.tag()));
        let state = *cx.global::<LabState>();
        if let Some(hwnd) = native::hwnd(window) {
            native::install(hwnd, native_config(state, surface, cx));
        }
        let receipt = native::apply(window, native_config(state, surface, cx));
        log_line(format!("lab.opened surface={} {receipt}", surface.tag()));
        clear_root(window, cx);
        cx.new(|cx| LabView::new(surface, state, window, cx))
    });
    if let Err(error) = opened {
        log_line(format!("lab.open_failed surface={} error={error}", surface.tag()));
    }
}

/// The official Root surface stays transparent; the lab paints body, fill and contour.
fn clear_root(window: &Window, cx: &mut App) {
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

struct LabView {
    surface: Surface,
    applied: LabState,
    /// A solid drag is in progress: the body paints opaque until the move loop ends.
    solid_drag: bool,
    _subscription: Subscription,
}

impl LabView {
    fn new(surface: Surface, state: LabState, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe_global_in::<LabState>(window, |this, window, cx| {
            let state = *cx.global::<LabState>();
            if state != this.applied {
                this.applied = state;
                let receipt = native::apply(window, native_config(state, this.surface, cx));
                log_line(format!(
                    "lab.applied surface={} {receipt}",
                    this.surface.tag()
                ));
            }
            cx.notify();
        });
        // The move loop runs inside DefWindowProc; GPUI's move-loop timer runs this task and
        // paints, so the switch lands within the loop instead of after it.
        if let Some(hwnd) = native::hwnd(window) {
            let events = native::drag_events(hwnd);
            cx.spawn_in(window, async move |view, cx| {
                while let Ok(active) = events.recv().await {
                    let applied = view.update_in(cx, |view, window, cx| {
                        view.set_solid_drag(active, window, cx)
                    });
                    if applied.is_err() {
                        break;
                    }
                }
            })
            .detach();
        }
        Self {
            surface,
            applied: state,
            solid_drag: false,
            _subscription: subscription,
        }
    }

    fn set_solid_drag(&mut self, active: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.solid_drag == active {
            return;
        }
        self.solid_drag = active;
        let started = std::time::Instant::now();
        let accent_ok = native::set_drag_body(window, active);
        log_line(format!(
            "lab.drag.body surface={} solid={active} accent_ok={accent_ok} apply_us={}",
            self.surface.tag(),
            started.elapsed().as_micros()
        ));
        cx.notify();
    }

    fn on_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match key {
            "escape" => window.remove_window(),
            "f2" => update(cx, |state| state.dark = !state.dark),
            "f3" => update(cx, |state| state.frosted = !state.frosted),
            "f5" => update(cx, |state| {
                state.backdrop = match state.backdrop {
                    Backdrop::Accent => Backdrop::Host,
                    Backdrop::Host => Backdrop::Accent,
                }
            }),
            "f4" => cx.defer(|cx| open_surface(Surface::Secondary, cx)),
            "f7" => update(cx, |state| state.corners = state.corners.next()),
            "f8" => update(cx, |state| state.drag = state.drag.next()),
            "f6" => update(cx, |state| state.throttle = state.throttle.next()),
            "f9" => update(cx, |state| state.drag_hide_shadow = !state.drag_hide_shadow),
            _ => return false,
        }
        true
    }

    fn option(
        &self,
        id: &'static str,
        label: &'static str,
        active: bool,
        cx: &mut Context<Self>,
        edit: fn(&mut LabState),
    ) -> Button {
        let button = Button::new(id)
            .label(label)
            .on_click(cx.listener(move |_, _, _, cx| update(cx, edit)));
        if active { button.primary() } else { button.outline() }
    }

    fn controls(&self, state: LabState, muted: Hsla, cx: &mut Context<Self>) -> impl IntoElement {
        let row = |title: &'static str| {
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(div().w(px(96.0)).text_color(muted).child(title))
        };
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                row("风格 F3")
                    .child(self.option("lab-solid", "实底", !state.frosted, cx, |s| {
                        s.frosted = false
                    }))
                    .child(self.option("lab-frosted", "磨砂", state.frosted, cx, |s| {
                        s.frosted = true
                    })),
            )
            .child(
                row("磨砂来源 F5")
                    .child(self.option(
                        "lab-backdrop-host",
                        Backdrop::Host.label(),
                        state.backdrop == Backdrop::Host,
                        cx,
                        |s| s.backdrop = Backdrop::Host,
                    ))
                    .child(self.option(
                        "lab-backdrop-accent",
                        Backdrop::Accent.label(),
                        state.backdrop == Backdrop::Accent,
                        cx,
                        |s| s.backdrop = Backdrop::Accent,
                    )),
            )
            .child(
                row("圆角 F7")
                    .child(self.option(
                        "lab-square",
                        Corners::Square.label(),
                        state.corners == Corners::Square,
                        cx,
                        |s| s.corners = Corners::Square,
                    ))
                    .child(self.option(
                        "lab-region",
                        Corners::Region.label(),
                        state.corners == Corners::Region,
                        cx,
                        |s| s.corners = Corners::Region,
                    ))
                    .child(self.option(
                        "lab-margin",
                        Corners::Margin.label(),
                        state.corners == Corners::Margin,
                        cx,
                        |s| s.corners = Corners::Margin,
                    ))
                    .child(self.option(
                        "lab-region-shadow",
                        Corners::RegionShadow.label(),
                        state.corners == Corners::RegionShadow,
                        cx,
                        |s| s.corners = Corners::RegionShadow,
                    )),
            )
            .child(
                row("拖动 F8")
                    .child(self.option(
                        "lab-drag-blur",
                        "拖动时保持模糊",
                        state.drag == DragMode::Blur,
                        cx,
                        |s| s.drag = DragMode::Blur,
                    ))
                    .child(self.option(
                        "lab-drag-tint",
                        "拖动时强调色实色",
                        state.drag == DragMode::Tint,
                        cx,
                        |s| s.drag = DragMode::Tint,
                    ))
                    .child(self.option(
                        "lab-drag-solid",
                        "拖动时整窗实底",
                        state.drag == DragMode::Solid,
                        cx,
                        |s| s.drag = DragMode::Solid,
                    )),
            )
            .child(
                row("阴影 F9")
                    .child(self.option(
                        "lab-shadow-follow",
                        "拖动时阴影跟随",
                        !state.drag_hide_shadow,
                        cx,
                        |s| s.drag_hide_shadow = false,
                    ))
                    .child(self.option(
                        "lab-shadow-hide",
                        "拖动时隐藏阴影",
                        state.drag_hide_shadow,
                        cx,
                        |s| s.drag_hide_shadow = true,
                    )),
            )
            .child(
                row("节流 F6")
                    .child(self.option(
                        "lab-throttle-off",
                        Throttle::Off.label(),
                        state.throttle == Throttle::Off,
                        cx,
                        |s| s.throttle = Throttle::Off,
                    ))
                    .child(self.option(
                        "lab-throttle-frame",
                        Throttle::Frame.label(),
                        state.throttle == Throttle::Frame,
                        cx,
                        |s| s.throttle = Throttle::Frame,
                    ))
                    .child(self.option(
                        "lab-throttle-frame2",
                        Throttle::TwoFrames.label(),
                        state.throttle == Throttle::TwoFrames,
                        cx,
                        |s| s.throttle = Throttle::TwoFrames,
                    )),
            )
            .child(
                row("其他").child(
                    Button::new("lab-open-secondary")
                        .label("再开一个二级窗口 F4")
                        .outline()
                        .on_click(|_, _, cx| cx.defer(|cx| open_surface(Surface::Secondary, cx))),
                ),
            )
    }

    fn checklist(&self, muted: Hsla) -> impl IntoElement {
        let lines = [
            "看四个角：② ③ ④ 下是否还露出方形的模糊或阴影；② ④ 的圆弧边缘是否有锯齿。",
            "看阴影：① 系统阴影；② 无阴影；③ 窗口内自绘阴影；④ 独立阴影窗口（拖动时阴影是否跟手）。",
            "看文字：实底 ① ② ④ 有 ClearType 彩边，③ 是灰度抗锯齿，比较清晰度。",
            "拖动：磨砂下用 F5 在「宿主背景（Terminal 方式）」与「强调色模糊（现状）」之间切换，各拖一次，比较是否跟手。",
            "宿主背景下再看：窗口后面的模糊是否正常、拖动时背景是否实时更新、失去焦点时是否还是磨砂。",
            "每种方式再按住下方“主窗口方式”手柄拖一次，对比标题栏拖动与主窗口的 SC_MOVE 拖动。",
            "③ 下窗口外圈 16px 是缩放边，鼠标放上去应出现调整大小的光标。",
        ];
        div()
            .flex()
            .flex_col()
            .gap_1()
            .text_sm()
            .text_color(muted)
            .children(lines.into_iter().map(|line| div().child(line)))
    }
}

impl Render for LabView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = *cx.global::<LabState>();
        let theme = cx.theme();
        let dark = state.dark;
        let canvas = theme.background;
        let foreground = theme.foreground;
        let muted = theme.muted_foreground;
        let border = theme.border;
        let raised = mix_white(canvas, if dark { 0.10 } else { 0.70 });
        // A solid drag paints the solid palette until the move loop ends.
        let (body_fill, sidebar_fill, content_fill) = if state.frosted && !self.solid_drag {
            (
                with_alpha(canvas, VEIL[usize::from(dark)]),
                transparent_black(),
                with_alpha(raised, CONTENT[usize::from(dark)]),
            )
        } else {
            (canvas, theme.sidebar, canvas)
        };
        let contour = if dark {
            with_alpha(gpui_kit::white(), 0.10)
        } else {
            with_alpha(gpui_kit::black(), 0.12)
        };
        let decorations = window.window_decorations();
        let tiled = matches!(decorations, Decorations::Client { tiling } if tiling.is_tiled());
        let maximized = window.is_maximized();
        let shaped = !tiled && !maximized;
        let rounded = shaped && state.corners != Corners::Square;
        let margin = shaped && state.corners == Corners::Margin;

        let keys = cx.listener(|this, event: &gpui_kit::KeyDownEvent, window, cx| {
            if !event.is_held && this.on_key(event.keystroke.key.as_str(), window, cx) {
                cx.stop_propagation();
            }
        });
        let build = native::windows_build();
        let status = format!(
            "Windows build {build} · {} · {} · 拖动{} · {} · {} · {}",
            if state.frosted { state.backdrop.label() } else { "实底" },
            state.corners.label(),
            state.drag.label(),
            state.throttle.label(),
            if state.drag_hide_shadow { "拖动隐藏阴影" } else { "阴影跟随" },
            if dark { "深色" } else { "浅色" }
        );
        let log = LOG_PATH
            .get()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        let content = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap_4()
            .p_5()
            .bg(content_fill)
            .when(state.frosted && !self.solid_drag, |panel| {
                panel.rounded_tl(px(RADIUS)).border_t_1().border_l_1().border_color(border)
            })
            .child(div().text_xl().child(match self.surface {
                Surface::Main => "主窗口",
                Surface::Secondary => "二级窗口",
            }))
            .child(div().text_sm().text_color(muted).child(status))
            .child(
                div()
                    .id("lab-system-move")
                    .px_3()
                    .py_2()
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(border)
                    .bg(with_alpha(foreground, 0.06))
                    .cursor_move()
                    .child("按住这里拖动（主窗口方式：SC_MOVE）")
                    .on_mouse_down(MouseButton::Left, |_, window, cx| {
                        if let Some(hwnd) = native::hwnd(window) {
                            native::begin_system_move(hwnd);
                        }
                        cx.stop_propagation();
                    }),
            )
            .when(self.surface == Surface::Main, |panel| {
                panel.child(self.controls(state, muted, cx))
            })
            .child(self.checklist(muted))
            .child(div().flex_1())
            .child(
                div()
                    .text_xs()
                    .text_color(muted)
                    .child(format!("F2 深浅色 · Esc 关闭 · 日志 {log}")),
            );
        let sidebar = div()
            .w(px(160.0))
            .flex_none()
            .p_3()
            .flex()
            .flex_col()
            .gap_1()
            .bg(sidebar_fill)
            .children(["下载", "速启", "剪贴板", "扩展"].into_iter().enumerate().map(
                |(index, label)| {
                    div()
                        .px_3()
                        .py_2()
                        .rounded(px(6.0))
                        .when(index == 0, |item| item.bg(with_alpha(foreground, 0.08)))
                        .text_color(if index == 0 { foreground } else { muted })
                        .child(label)
                },
            ));
        let mut body = div()
            .flex()
            .flex_col()
            .size_full()
            .overflow_hidden()
            .font_family("Sarasa UI SC")
            .text_color(foreground)
            .bg(body_fill)
            .child(
                TitleBar::new().bg(transparent_black()).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child(SharedString::from("NxTools · Win10 圆角实验")),
                ),
            )
            .child(div().flex().flex_1().min_h_0().child(sidebar).child(content));
        if shaped {
            body = body.border_1().border_color(contour);
        }
        if rounded {
            body = body.rounded(px(RADIUS));
        }
        if margin {
            body = body.shadow(vec![
                gpui_kit::BoxShadow::new(px(0.0), px(6.0), gpui_kit::black().opacity(0.26))
                    .blur_radius(px(14.0)),
                gpui_kit::BoxShadow::new(px(0.0), px(0.0), gpui_kit::black().opacity(0.12))
                    .blur_radius(px(2.0)),
            ]);
        }
        // A client-decorated frame reserves a 1 DIP ring; the outer layer covers it so the
        // window edge, margin and body line up with the native window rectangle.
        let client = matches!(decorations, Decorations::Client { .. });
        let mut outer = div()
            .id("lab-root")
            .capture_key_down(keys)
            .flex()
            .bg(transparent_black())
            .when(margin, |outer| outer.p(px(MARGIN)));
        outer = if client && !tiled {
            let edge = px(-1.0);
            outer.absolute().top(edge).left(edge).right(edge).bottom(edge)
        } else {
            outer.size_full()
        };
        div().relative().size_full().child(outer.child(body))
    }
}

fn main() {
    let log_directory = std::env::temp_dir().join("nxtools-win10-corner-lab");
    if std::fs::create_dir_all(&log_directory).is_ok() {
        let _ = LOG_PATH.set(log_directory.join("lab.log"));
    }
    log_line(format!(
        "lab.started build={} pid={}",
        native::windows_build(),
        std::process::id()
    ));
    // Optional start state: --solid, --dark, --drag=blur|tint|solid, --throttle=off|frame|frame2,
    // --drag-hide-shadow,
    // --corners=square|region|margin|region-shadow.
    let arguments: Vec<String> = std::env::args().collect();
    let flag = |name: &str| arguments.iter().any(|argument| argument == name);
    let corners = arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("--corners="))
        .and_then(|tag| Corners::ALL.into_iter().find(|corners| corners.tag() == tag))
        .unwrap_or(Corners::RegionShadow);
    let state = LabState {
        frosted: !flag("--solid"),
        backdrop: if flag("--backdrop=accent") {
            Backdrop::Accent
        } else {
            Backdrop::Host
        },
        corners,
        drag: arguments
            .iter()
            .find_map(|argument| argument.strip_prefix("--drag="))
            .and_then(|tag| DragMode::ALL.into_iter().find(|mode| mode.tag() == tag))
            .unwrap_or(DragMode::Blur),
        throttle: arguments
            .iter()
            .find_map(|argument| argument.strip_prefix("--throttle="))
            .and_then(|tag| Throttle::ALL.into_iter().find(|mode| mode.tag() == tag))
            .unwrap_or(Throttle::Off),
        drag_hide_shadow: flag("--drag-hide-shadow"),
        dark: flag("--dark"),
    };
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            cx.text_system()
                .add_fonts(vec![Cow::Borrowed(include_bytes!(
                    "../assets/fonts/SarasaUiSC-Regular.ttf"
                ))])
                .expect("register bundled Sarasa UI SC Regular");
            gpui_kit::init(cx);
            ThemeRegistry::global_mut(cx)
                .load_themes_from_str(include_str!("../assets/themes/nxtools.json"))
                .expect("bundled NxTools schemes");
            let registry = ThemeRegistry::global(cx);
            let light = registry.themes()["默认浅色"].clone();
            let dark = registry.themes()["默认深色"].clone();
            Theme::update(cx, |theme| {
                theme.light_theme = light;
                theme.dark_theme = dark;
            });
            cx.set_global(state);
            apply_theme(state, cx);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    log_line("lab.finished".into());
                    cx.quit();
                }
            })
            .detach();
            open_surface(Surface::Main, cx);
            open_surface(Surface::Secondary, cx);
            cx.activate(true);
        });
}
