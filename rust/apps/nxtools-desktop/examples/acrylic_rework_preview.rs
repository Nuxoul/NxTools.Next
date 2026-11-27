//! Native preview of the reworked frosted theme: DWM system backdrops, a neutral material
//! palette with one content layer, DWM rounding with a self-drawn contour, Opaque solid
//! windows and a cloaked first frame. No NxTools runtime or persisted state is involved.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
use std::{borrow::Cow, path::PathBuf, time::Duration, time::Instant};

use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, FocusableExt as _, Icon, Sizable as _, Size, Theme, ThemeMode, ThemeRegistry,
    TitleBar,
    button::{Button, ButtonVariants as _},
    group_box::{GroupBox, GroupBoxVariants as _},
    input::{Input, InputGroup, InputGroupAddon, InputState},
    list::ListItem,
    menu::{DropdownMenu as _, PopupMenuItem},
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
    slider::{Slider, SliderEvent, SliderState},
};
use gpui_kit::{
    App, AppContext as _, Bounds, Context, Decorations, Entity, Focusable as _, Global,
    IntoElement, Render, SharedString, StyleRefinement, Styled as _, Subscription, Window,
    WindowBounds, WindowKind, WindowOptions, div, prelude::*, px, size, transparent_black,
};

#[path = "acrylic_rework_preview/native.rs"]
mod native;
#[path = "acrylic_rework_preview/palette.rs"]
mod palette;

use native::{Mode, Role};
use palette::Layers;

/// Diagnostics go to stdout and to `preview.log` beside the captures, because the windows
/// subsystem build has no console on a test machine.
macro_rules! report {
    ($($arg:tt)*) => {
        crate::log_line(format!($($arg)*))
    };
}

static LOG_PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

pub(crate) fn log_line(line: String) {
    println!("{line}");
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

const RADIUS: f32 = 8.0;
const REVEAL_TIMEOUT: Duration = Duration::from_millis(250);
const HINTS: &str =
    "F2 深浅色 · F3 材质 · F4 剪切板 · F5 搜索 · F6 Win10 模糊/亚克力 · F9 截取 GPU 帧 · Esc 关闭";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Surface {
    Workbench,
    Clipboard,
    Search,
}

impl Surface {
    fn tag(self) -> &'static str {
        match self {
            Surface::Workbench => "workbench",
            Surface::Clipboard => "clipboard",
            Surface::Search => "search",
        }
    }

    fn role(self) -> Role {
        match self {
            Surface::Workbench => Role::Persistent,
            Surface::Clipboard | Surface::Search => Role::Transient,
        }
    }

    fn size(self) -> gpui_kit::Size<gpui_kit::Pixels> {
        match self {
            Surface::Workbench => size(px(960.0), px(640.0)),
            Surface::Clipboard => size(px(480.0), px(540.0)),
            Surface::Search => size(px(620.0), px(392.0)),
        }
    }
}

/// Preview-wide state; every window observes it and re-applies its native material.
#[derive(Clone, Copy, Debug)]
struct PreviewState {
    mode: Mode,
    dark: bool,
    content_percent: f32,
    /// Windows 10 accent fallback: F6 switches between blur (3) and acrylic (4).
    accent_blur: native::AccentBlur,
}

impl Global for PreviewState {}

impl PreviewState {
    fn layers(&self, cx: &App) -> Layers {
        if self.mode.frosted() {
            palette::frosted_layers(self.dark, self.content_percent)
        } else {
            palette::solid_layers(&cx.theme().colors)
        }
    }
}

struct Launch {
    correlation: uuid::Uuid,
    capture_directory: PathBuf,
    /// Automatic GPU readback after reveal, for scripted verification.
    capture_after: Option<Duration>,
    /// Windows 11 DWM rounding, or square Windows 10 corners.
    corners: native::Corners,
    /// Build 22621+ has DWM system backdrops; older systems fall back to Accent4.
    system_backdrops: bool,
}

impl Global for Launch {}

fn apply_theme(state: PreviewState, cx: &mut App) {
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
        theme.mono_font_family = "Consolas".into();
        theme.focus_ring = false;
        theme.window_radius = px(RADIUS);
        // The contour is drawn by the root element itself, never by the official frame.
        theme.colors.window_border.a = 0.0;
        if state.mode.frosted() {
            palette::apply_frosted(&mut theme.colors, state.dark);
        }
    });
}

fn update_state(cx: &mut App, retheme: bool, edit: impl FnOnce(&mut PreviewState)) {
    let mut state = *cx.global::<PreviewState>();
    edit(&mut state);
    cx.set_global(state);
    if retheme {
        apply_theme(state, cx);
        let correlation = cx.global::<Launch>().correlation;
        report!(
            "ui.acrylic_preview.state correlation={correlation} mode={} dark={} accent_blur={}",
            state.mode.tag(),
            state.dark,
            state.accent_blur.label()
        );
    }
}

fn initialize(cx: &mut App, state: PreviewState) {
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
    let dark = registry.themes()["Catppuccin Frappe"].clone();
    Theme::update(cx, |theme| {
        theme.light_theme = light;
        theme.dark_theme = dark;
    });
    cx.set_global(state);
    apply_theme(state, cx);
}

/// Opens a surface hidden and cloaked, applies its material, then shows it. The view
/// uncloaks after its first frame has been presented (or after `REVEAL_TIMEOUT`).
fn open_surface(surface: Surface, cx: &mut App) {
    let opened = Instant::now();
    let correlation = cx.global::<Launch>().correlation;
    let bounds = Bounds::centered(None, surface.size(), cx);
    let options = match surface {
        Surface::Workbench => WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            show: false,
            focus: true,
            is_resizable: true,
            is_minimizable: true,
            window_min_size: Some(size(px(720.0), px(480.0))),
            app_id: Some("NxTools.AcrylicReworkPreview".into()),
            ..TitleBar::window_options()
        },
        Surface::Clipboard | Surface::Search => WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: None,
            kind: WindowKind::PopUp,
            show: false,
            focus: true,
            is_resizable: false,
            is_minimizable: false,
            app_id: Some("NxTools.AcrylicReworkPreview".into()),
            ..Default::default()
        },
    };
    let opened_window = gpui_kit::open_window(options, cx, move |window, cx| {
        window.set_window_title(&format!("NxTools — 磨砂主题预览 · {}", surface.tag()));
        let hwnd = native::hwnd(window);
        let cloak_hr = hwnd.map_or(-1, |hwnd| native::set_cloaked(hwnd, true));
        let state = *cx.global::<PreviewState>();
        let corners = cx.global::<Launch>().corners;
        let receipt = native::apply(
            window,
            surface.role(),
            state.mode,
            state.dark,
            corners,
            state.accent_blur,
        );
        report!(
            "ui.acrylic_preview.opened correlation={correlation} surface={} mode={} cloak_hr={cloak_hr} {}",
            surface.tag(),
            state.mode.tag(),
            native::describe(receipt)
        );
        transparent_root(window, cx);
        let applied = (state.mode, state.dark, state.accent_blur);
        cx.new(|cx| PreviewView::new(surface, opened, applied, window, cx))
    });
    match opened_window {
        Ok((handle, _)) => {
            let _ = handle.update(cx, |_, window, _| window.activate_window());
        }
        Err(error) => report!(
            "ui.acrylic_preview.open_failed correlation={correlation} surface={} error={error}",
            surface.tag()
        ),
    }
}

/// The official Root surface stays transparent; the preview root paints fill and contour.
fn transparent_root(window: &Window, cx: &mut App) {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reveal {
    Pending,
    Armed,
    Shown,
}

struct PreviewView {
    surface: Surface,
    input: Entity<InputState>,
    content: Entity<SliderState>,
    page: usize,
    selected: usize,
    opened: Instant,
    reveal: Reveal,
    applied: (Mode, bool, native::AccentBlur),
    _subscriptions: Vec<Subscription>,
}

impl PreviewView {
    fn new(
        surface: Surface,
        opened: Instant,
        applied: (Mode, bool, native::AccentBlur),
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let placeholder = match surface {
            Surface::Search => "搜索应用、文件或文件夹",
            Surface::Clipboard => "搜索剪切板记录",
            Surface::Workbench => "搜索任务、记录或设置",
        };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        input.focus_handle(cx).focus(window, cx);
        let percent = cx.global::<PreviewState>().content_percent;
        let content = cx.new(|_| {
            SliderState::new()
                .min(palette::MIN_CONTENT_PERCENT)
                .max(100.0)
                .step(1.0)
                .default_value(percent)
        });
        let subscriptions = vec![
            cx.subscribe(&content, |_, state, _: &SliderEvent, cx| {
                let percent = state.read(cx).value().start().round();
                update_state(cx, false, |preview| preview.content_percent = percent);
            }),
            cx.observe_global_in::<PreviewState>(window, |this, window, cx| {
                this.sync_material(window, cx)
            }),
        ];
        Self {
            surface,
            input,
            content,
            page: 0,
            selected: 0,
            opened,
            reveal: Reveal::Pending,
            applied,
            _subscriptions: subscriptions,
        }
    }

    fn sync_material(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let state = *cx.global::<PreviewState>();
        let applied = (state.mode, state.dark, state.accent_blur);
        if applied != self.applied {
            self.applied = applied;
            let corners = cx.global::<Launch>().corners;
            let receipt = native::apply(
                window,
                self.surface.role(),
                state.mode,
                state.dark,
                corners,
                state.accent_blur,
            );
            let correlation = cx.global::<Launch>().correlation;
            report!(
                "ui.acrylic_preview.material_applied correlation={correlation} surface={} mode={} dark={} {}",
                self.surface.tag(),
                state.mode.tag(),
                state.dark,
                native::describe(receipt)
            );
        }
        cx.notify();
    }

    fn arm_reveal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.reveal != Reveal::Pending {
            return;
        }
        self.reveal = Reveal::Armed;
        let view = cx.entity().downgrade();
        // Runs at the start of the frame after the one being drawn now, i.e. once the first
        // frame has been presented to DirectComposition.
        window.on_next_frame(move |window, cx| {
            let _ = view.update(cx, |this, cx| this.uncloak("frame", window, cx));
        });
        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(REVEAL_TIMEOUT).await;
            let _ = this.update_in(cx, |this, window, cx| this.uncloak("timeout", window, cx));
        })
        .detach();
    }

    fn uncloak(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.reveal == Reveal::Shown {
            return;
        }
        self.reveal = Reveal::Shown;
        let hr = native::hwnd(window).map_or(-1, |hwnd| native::set_cloaked(hwnd, false));
        let correlation = cx.global::<Launch>().correlation;
        report!(
            "ui.acrylic_preview.uncloaked correlation={correlation} surface={} path={path} hr={hr} elapsed_ms={}",
            self.surface.tag(),
            self.opened.elapsed().as_millis()
        );
        if let Some(delay) = cx.global::<Launch>().capture_after {
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(delay).await;
                let _ = this.update_in(cx, |this, window, cx| this.capture(window, cx));
            })
            .detach();
        }
    }

    fn on_key(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        match key {
            "escape" => window.remove_window(),
            "f2" => update_state(cx, true, |state| state.dark = !state.dark),
            "f3" => {
                let backdrops = cx.global::<Launch>().system_backdrops;
                update_state(cx, true, |state| state.mode = state.mode.next(backdrops))
            }
            "f4" => cx.defer(|cx| open_surface(Surface::Clipboard, cx)),
            "f5" => cx.defer(|cx| open_surface(Surface::Search, cx)),
            "f6" => update_state(cx, true, |state| {
                state.accent_blur = state.accent_blur.toggled()
            }),
            "f9" => self.capture(window, cx),
            _ => return false,
        }
        true
    }

    /// GPU readback of the composed GPUI frame (before DWM adds the backdrop). Premultiplied
    /// alpha at named points tells which regions really let the system material through.
    fn capture(&self, window: &mut Window, cx: &mut Context<Self>) {
        let launch = cx.global::<Launch>();
        let correlation = launch.correlation;
        let state = *cx.global::<PreviewState>();
        let name = format!(
            "{}-{}-{}",
            self.surface.tag(),
            state.mode.tag(),
            if state.dark { "dark" } else { "light" }
        );
        let path = launch.capture_directory.join(format!("{name}.png"));
        let image = match window.render_to_image() {
            Ok(image) => image,
            Err(error) => {
                report!(
                    "ui.acrylic_preview.capture_failed correlation={correlation} stage=readback error={error}"
                );
                return;
            }
        };
        let scale = window.scale_factor();
        let (width, height) = image.dimensions();
        let at = |x: f32, y: f32| {
            let x = ((x * scale) as u32).min(width - 1);
            let y = ((y * scale) as u32).min(height - 1);
            image.get_pixel(x, y).0
        };
        let logical_width = width as f32 / scale;
        let logical_height = height as f32 / scale;
        let samples = [
            ("title_bar", at(logical_width * 0.5, 14.0)),
            ("sidebar", at(90.0, logical_height * 0.5)),
            ("content", at(logical_width * 0.62, logical_height - 40.0)),
            ("corner_0", image.get_pixel(0, 0).0),
            ("corner_2", image.get_pixel(2, 2).0),
            ("edge_mid_left", image.get_pixel(0, height / 2).0),
            ("edge_mid_top", image.get_pixel(width / 2, 0).0),
        ];
        let saved = std::fs::create_dir_all(&launch.capture_directory)
            .map_err(|error| error.to_string())
            .and_then(|_| image.save(&path).map_err(|error| error.to_string()));
        let rendered: Vec<String> = samples
            .iter()
            .map(|(label, [r, g, b, a])| format!("{label}=rgba({r},{g},{b},{a})"))
            .collect();
        report!(
            "ui.acrylic_preview.capture correlation={correlation} surface={} mode={} dark={} scale={scale} size={width}x{height} saved={} path={} {}",
            self.surface.tag(),
            state.mode.tag(),
            state.dark,
            saved.is_ok(),
            path.display(),
            rendered.join(" ")
        );
    }

    fn frame(
        &self,
        window: &Window,
        layers: &Layers,
        cx: &App,
    ) -> gpui_kit::Stateful<gpui_kit::Div> {
        let decorations = window.window_decorations();
        let tiled = matches!(decorations, Decorations::Client { tiling } if tiling.is_tiled());
        let client = matches!(decorations, Decorations::Client { .. });
        let mut root = div()
            .id("acrylic-preview-root")
            .flex()
            .flex_col()
            .overflow_hidden()
            .font_family("Sarasa UI SC")
            .text_color(cx.theme().foreground)
            .bg(layers.window);
        // A client-decorated frame reserves a 1 DIP ring; the root covers it so fill and
        // contour stay one element out to the window edge.
        root = if client && !tiled {
            let edge = px(-1.0);
            root.absolute()
                .top(edge)
                .left(edge)
                .right(edge)
                .bottom(edge)
        } else {
            root.size_full()
        };
        // Windows 10 keeps the 1px contour on a square window.
        if !tiled {
            root = root.border_1().border_color(layers.contour);
            if cx.global::<Launch>().corners == native::Corners::System {
                root = root.rounded(px(RADIUS));
            }
        }
        root
    }

    fn search_input(&self, layers: &Layers, large: bool, surface_fill: bool) -> InputGroup {
        InputGroup::new("acrylic-search")
            .aria_label("搜索")
            .with_size(Size::Large)
            .focus_ring(false)
            .w_full()
            .h(px(if large { 60.0 } else { 40.0 }))
            .rounded(px(if large { 0.0 } else { 6.0 }))
            .when(large, |group| group.border_0())
            .bg(if surface_fill {
                transparent_black()
            } else {
                layers.input
            })
            .addon(
                InputGroupAddon::new("acrylic-search-icon")
                    .pl(px(if large { 18.0 } else { 12.0 }))
                    .child(Icon::new(IconName::Search).size(px(if large { 20.0 } else { 16.0 }))),
            )
            .input(
                Input::new(&self.input)
                    .id("acrylic-search-input")
                    .aria_label("搜索输入")
                    .h(px(if large { 58.0 } else { 38.0 }))
                    .pl(px(10.0))
                    .pr(px(16.0))
                    .py(px(0.0))
                    .text_size(px(if large { 20.0 } else { 14.0 }))
                    .line_height(px(if large { 28.0 } else { 20.0 })),
            )
    }

    fn records(&self, kind: usize, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let records: [(&str, &str, IconName); 4] = match kind {
            1 => [
                ("周五前确认界面方案", "文本 · 2 分钟前", IconName::FileText),
                (
                    "https://github.com/longbridge/gpui-component",
                    "链接 · 10 分钟前",
                    IconName::Globe,
                ),
                ("界面素材", "文件夹 · 今天 09:12", IconName::Folder),
                ("会议纪要：材质与圆角", "文本 · 昨天", IconName::FileText),
            ],
            2 => [
                (
                    "Visual Studio Code",
                    "C:\\Program Files\\Microsoft VS Code",
                    IconName::SquareTerminal,
                ),
                ("设置", "系统应用", IconName::Settings),
                (
                    "项目文档.md",
                    "D:\\Code\\NxTools.Next-main\\docs",
                    IconName::FileText,
                ),
                (
                    "下载",
                    "C:\\Users\\Administrator\\Downloads",
                    IconName::Folder,
                ),
            ],
            _ => [
                (
                    "项目素材.zip",
                    "下载中 · 128 MB / 256 MB · 8.2 MB/s",
                    IconName::File,
                ),
                ("使用说明.pdf", "已完成 · 12.4 MB", IconName::FileText),
                ("界面资源.zip", "等待队列 · 36.8 MB", IconName::File),
                ("安装包.msi", "已暂停 · 64 MB / 210 MB", IconName::File),
            ],
        };
        let muted = cx.theme().muted_foreground;
        div()
            .flex()
            .flex_col()
            .gap_1()
            .children(
                records
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, detail, icon))| {
                        ListItem::new(SharedString::from(format!("acrylic-record-{kind}-{index}")))
                            .accessibility_label(label)
                            .selected(self.selected == index)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected = index;
                                cx.notify();
                            }))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_3()
                                    .py_1()
                                    .child(Icon::new(icon).size(px(18.0)))
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .min_w_0()
                                            .child(div().truncate().child(label))
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(muted)
                                                    .truncate()
                                                    .child(detail),
                                            ),
                                    ),
                            )
                    }),
            )
    }

    fn card(&self, layers: &Layers, title: &'static str) -> GroupBox {
        GroupBox::new().fill().title(title).content_style(
            StyleRefinement::default()
                .border_1()
                .border_color(layers.card_stroke),
        )
    }

    fn appearance_page(
        &self,
        state: PreviewState,
        layers: &Layers,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let system_backdrops = cx.global::<Launch>().system_backdrops;
        let mut modes = div().flex().flex_wrap().gap_2();
        for &mode in Mode::available(system_backdrops) {
            let button = Button::new(SharedString::from(format!("acrylic-mode-{}", mode.tag())))
                .label(mode.label_for(system_backdrops))
                .on_click(cx.listener(move |_, _, _, cx| {
                    update_state(cx, true, |state| state.mode = mode)
                }));
            modes = modes.child(if mode == state.mode {
                button.primary()
            } else {
                button.outline()
            });
        }
        let percent = self.content.read(cx).value().start().round();
        let muted = cx.theme().muted_foreground;
        div()
            .flex()
            .flex_col()
            .gap_4()
            .child(
                self.card(layers, "材质").child(modes).child(
                    div()
                        .text_sm()
                        .text_color(muted)
                        .child("常驻窗口：Mica / Mica Alt；瞬态浮层（F4/F5）始终使用亚克力。"),
                ),
            )
            .child(
                self.card(layers, "内容层")
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .child(div().w(px(96.0)).child("不透明度"))
                            .child(
                                Slider::new(&self.content)
                                    .accessibility_label("内容层不透明度")
                                    .disabled(!state.mode.frosted())
                                    .flex_1(),
                            )
                            .child(
                                div()
                                    .w(px(44.0))
                                    .text_right()
                                    .child(format!("{percent:.0}%")),
                            ),
                    )
                    .child(div().text_sm().text_color(muted).child(format!(
                        "当前 alpha {:.2}（深色为浅色的 0.9 倍）；标题栏与侧栏始终透明。",
                        palette::content_alpha(percent, state.dark)
                    ))),
            )
            .child(
                self.card(layers, "深浅色").child(
                    div().flex().gap_2().child(
                        Button::new("acrylic-toggle-dark")
                            .label(if state.dark {
                                "切换到浅色"
                            } else {
                                "切换到深色"
                            })
                            .outline()
                            .on_click(cx.listener(|_, _, _, cx| {
                                update_state(cx, true, |state| state.dark = !state.dark)
                            })),
                    ),
                ),
            )
    }

    fn workbench(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = *cx.global::<PreviewState>();
        let layers = state.layers(cx);
        let launch = cx.global::<Launch>();
        let accent_effective =
            state.mode == Mode::Accent || (state.mode.frosted() && !launch.system_backdrops);
        let material_label = if accent_effective {
            state.accent_blur.label()
        } else {
            state.mode.label_for(launch.system_backdrops)
        };
        let corner_tag = match launch.corners {
            native::Corners::System => "圆角 Win11 系统",
            native::Corners::Square => "方角（Win10）",
        };
        let mut menu = SidebarMenu::new();
        for (index, (label, icon)) in [
            ("下载", IconName::File),
            ("剪切板", IconName::Copy),
            ("外观", IconName::Palette),
        ]
        .into_iter()
        .enumerate()
        {
            menu = menu.child(
                SidebarMenuItem::new(label)
                    .icon(icon)
                    .active(self.page == index)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.page = index;
                        cx.notify();
                    })),
            );
        }
        let title = ["下载", "剪切板", "外观"][self.page];
        let body = match self.page {
            2 => self.appearance_page(state, &layers, cx).into_any_element(),
            page => self
                .card(&layers, if page == 1 { "最近记录" } else { "任务" })
                .child(self.records(page, cx))
                .into_any_element(),
        };
        let muted = cx.theme().muted_foreground;
        let frosted = state.mode.frosted();
        let content = div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w_0()
            .gap_4()
            .p_5()
            .bg(layers.content)
            .when(frosted, |panel| {
                panel
                    .rounded_tl(px(RADIUS))
                    .border_t_1()
                    .border_l_1()
                    .border_color(layers.card_stroke)
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xl().child(title))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new("acrylic-open-clipboard")
                                    .label("剪切板浮层")
                                    .outline()
                                    .on_click(|_, _, cx| {
                                        cx.defer(|cx| open_surface(Surface::Clipboard, cx))
                                    }),
                            )
                            .child(
                                Button::new("acrylic-more")
                                    .label("更多")
                                    .outline()
                                    .dropdown_caret(true)
                                    .dropdown_menu(|menu, _, _| {
                                        menu.item(PopupMenuItem::new("打开搜索浮层").on_click(
                                            |_, _, cx| {
                                                cx.defer(|cx| open_surface(Surface::Search, cx))
                                            },
                                        ))
                                        .item(PopupMenuItem::new("切换深浅色").on_click(
                                            |_, _, cx| {
                                                update_state(cx, true, |state| {
                                                    state.dark = !state.dark
                                                })
                                            },
                                        ))
                                        .separator()
                                        .item(
                                            PopupMenuItem::new("下一种材质").on_click(
                                                |_, _, cx| {
                                                    let backdrops =
                                                        cx.global::<Launch>().system_backdrops;
                                                    update_state(cx, true, |state| {
                                                        state.mode = state.mode.next(backdrops)
                                                    })
                                                },
                                            ),
                                        )
                                    }),
                            ),
                    ),
            )
            .child(self.search_input(&layers, false, false))
            .child(body)
            .child(div().flex_1())
            .child(div().text_sm().text_color(muted).child(HINTS));
        self.frame(window, &layers, cx)
            .child(
                TitleBar::new().bg(transparent_black()).child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .child("NxTools · 磨砂主题预览")
                        .child(div().text_sm().text_color(muted).child(format!(
                            "{} · {} · {}",
                            material_label,
                            if state.dark { "深色" } else { "浅色" },
                            corner_tag
                        ))),
                ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        Sidebar::new("acrylic-sidebar")
                            .w(px(200.0))
                            .collapsible(false)
                            .bg(transparent_black())
                            .border_0()
                            .child(menu),
                    )
                    .child(content),
            )
    }

    fn clipboard(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = *cx.global::<PreviewState>();
        let layers = state.layers(cx);
        let muted = cx.theme().muted_foreground;
        self.frame(window, &layers, cx).child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_h_0()
                .gap_3()
                .p_4()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .child(div().text_lg().child("剪切板"))
                        .child(
                            div()
                                .text_sm()
                                .text_color(muted)
                                .child(state.mode.transient_label()),
                        ),
                )
                .child(self.search_input(&layers, false, false))
                .child(
                    div()
                        .text_sm()
                        .text_color(muted)
                        .child("全部 · 文本 · 图片 · 文件"),
                )
                .child(self.records(1, cx))
                .child(div().flex_1())
                .child(
                    div()
                        .text_sm()
                        .text_color(muted)
                        .child("F2 深浅色 · F3 材质 · Esc 关闭"),
                ),
        )
    }

    fn search(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = *cx.global::<PreviewState>();
        let layers = state.layers(cx);
        let muted = cx.theme().muted_foreground;
        let divider = cx.theme().border;
        self.frame(window, &layers, cx)
            .child(self.search_input(&layers, true, true))
            .child(div().h(px(1.0)).bg(divider))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .gap_1()
                    .child(self.records(2, cx))
                    .child(div().flex_1())
                    .child(
                        div()
                            .px_2()
                            .text_xs()
                            .text_color(muted)
                            .child("Enter 打开 · Esc 关闭 · F3 材质"),
                    ),
            )
    }
}

impl Render for PreviewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.arm_reveal(window, cx);
        let keys = cx.listener(|this, event: &gpui_kit::KeyDownEvent, window, cx| {
            if !event.is_held && this.on_key(event.keystroke.key.as_str(), window, cx) {
                cx.stop_propagation();
            }
        });
        let body = match self.surface {
            Surface::Workbench => self.workbench(window, cx).into_any_element(),
            Surface::Clipboard => self.clipboard(window, cx).into_any_element(),
            Surface::Search => self.search(window, cx).into_any_element(),
        };
        div()
            .id("acrylic-preview")
            .relative()
            .size_full()
            .capture_key_down(keys)
            .child(body)
    }
}

fn argument<'a>(arguments: &'a [String], name: &str) -> Option<&'a str> {
    arguments
        .iter()
        .find_map(|argument| argument.strip_prefix(name)?.strip_prefix('='))
}

fn main() {
    let arguments: Vec<String> = std::env::args().collect();
    let mode = argument(&arguments, "--mode")
        .and_then(Mode::parse)
        .unwrap_or(Mode::Mica);
    let state = PreviewState {
        mode,
        dark: !arguments.iter().any(|argument| argument == "--light"),
        content_percent: palette::DEFAULT_CONTENT_PERCENT,
        accent_blur: native::AccentBlur::Blur,
    };
    let correlation = argument(&arguments, "--correlation")
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
        .unwrap_or_else(uuid::Uuid::new_v4);
    let capture_directory = argument(&arguments, "--capture-dir")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("nxtools-acrylic-preview"));
    if std::fs::create_dir_all(&capture_directory).is_ok() {
        let _ = LOG_PATH.set(capture_directory.join("preview.log"));
    }
    let extra: Vec<Surface> = argument(&arguments, "--open")
        .map(|list| {
            list.split(',')
                .filter_map(|name| match name {
                    "clipboard" => Some(Surface::Clipboard),
                    "search" => Some(Surface::Search),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default();
    let exit_after = argument(&arguments, "--exit-after-ms")
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis);
    // `--legacy-corners` shows the Windows 10 presentation on Windows 11: square corners and
    // only the solid and frosted (accent) modes.
    let legacy = arguments
        .iter()
        .any(|argument| argument == "--legacy-corners");
    let corners = native::Corners::detect(legacy);
    let system_backdrops = !legacy && native::windows_build() >= native::SYSTEM_BACKDROP_BUILD;
    let state = PreviewState {
        mode: mode.effective(system_backdrops),
        ..state
    };
    let capture_after = argument(&arguments, "--capture-after-ms")
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_millis);
    let started = Instant::now();
    report!(
        "ui.acrylic_preview.started correlation={correlation} mode={} dark={} build={}",
        state.mode.tag(),
        state.dark,
        native::windows_build()
    );
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx| {
            cx.set_global(Launch {
                correlation,
                capture_directory,
                capture_after,
                corners,
                system_backdrops,
            });
            initialize(cx, state);
            cx.on_window_closed(move |cx, _| {
                if cx.windows().is_empty() {
                    report!(
                        "ui.acrylic_preview.finished correlation={correlation} elapsed_ms={}",
                        started.elapsed().as_millis()
                    );
                    cx.quit();
                }
            })
            .detach();
            open_surface(Surface::Workbench, cx);
            for surface in extra {
                open_surface(surface, cx);
            }
            if let Some(delay) = exit_after {
                cx.spawn(async move |cx| {
                    cx.background_executor().timer(delay).await;
                    cx.update(|cx| cx.quit());
                })
                .detach();
            }
            cx.activate(true);
        });
}
