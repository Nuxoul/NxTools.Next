use gpui_kit::assets::IconName;
use gpui_kit::component::{
    ActiveTheme as _, FocusableExt as _, Icon, Sizable as _, Size, TitleBar,
    button::Button,
    group_box::GroupBox,
    input::{Input, InputGroup, InputGroupAddon, InputState},
    list::ListItem,
    sidebar::{Sidebar, SidebarMenu, SidebarMenuItem},
};
use gpui_kit::{
    App, AppContext as _, Bounds, Context, Entity, Focusable as _, Hsla, IntoElement, Render,
    SharedString, Styled as _, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, div, prelude::*, px, size, transparent_black,
};

#[path = "benchmark.rs"]
mod benchmark;

const TITLE: &str = "NxTools — Native Material Preview (F2 / Esc)";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Surface {
    Workbench,
    Clipboard,
    Search,
}
impl Surface {
    fn dimensions(self) -> gpui_kit::Size<gpui_kit::Pixels> {
        match self {
            Self::Workbench => size(px(900.0), px(600.0)),
            Self::Clipboard => size(px(480.0), px(540.0)),
            Self::Search => size(px(620.0), px(60.0)),
        }
    }
    fn tag(self) -> &'static str {
        match self {
            Self::Workbench => "workbench",
            Self::Clipboard => "clipboard",
            Self::Search => "search",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Material {
    Solid,
    Acrylic,
}
impl Material {
    fn tag(self) -> &'static str {
        match self {
            Self::Solid => "solid",
            Self::Acrylic => "acrylic",
        }
    }
    fn fill(self, color: Hsla) -> Hsla {
        color.opacity(if self == Self::Acrylic { 0.78 } else { 1.0 })
    }
}
struct Options {
    surface: Surface,
    material: Material,
    dark: bool,
    benchmark: Option<benchmark::Scenario>,
    benchmark_seconds: u64,
}
impl Options {
    fn parse(arguments: &[String]) -> Result<Self, &'static str> {
        if arguments
            .iter()
            .any(|argument| argument.starts_with("--gpu-capture="))
        {
            return Err(
                "DWM material must be observed in the native window, not a GPU-only capture",
            );
        }
        let surface = match arguments
            .iter()
            .find_map(|argument| argument.strip_prefix("--surface="))
        {
            None | Some("workbench") => Surface::Workbench,
            Some("clipboard") => Surface::Clipboard,
            Some("search") => Surface::Search,
            Some(_) => return Err("unsupported native preview surface"),
        };
        let material = match arguments
            .iter()
            .find_map(|argument| argument.strip_prefix("--material="))
        {
            None | Some("acrylic") => Material::Acrylic,
            Some("solid") => Material::Solid,
            Some(_) => return Err("unsupported native preview material"),
        };
        let benchmark = arguments
            .iter()
            .find_map(|argument| argument.strip_prefix("--benchmark="))
            .map(benchmark::Scenario::parse)
            .transpose()?;
        let benchmark_seconds = arguments
            .iter()
            .find_map(|argument| argument.strip_prefix("--benchmark-seconds="))
            .map(|value| {
                value
                    .parse::<u64>()
                    .map_err(|_| "invalid benchmark duration")
            })
            .transpose()?
            .unwrap_or(15);
        if !(1..=120).contains(&benchmark_seconds) {
            return Err("benchmark duration must be 1 through 120 seconds");
        }
        Ok(Self {
            surface,
            material,
            dark: !arguments.iter().any(|argument| argument == "--light"),
            benchmark,
            benchmark_seconds,
        })
    }
}

struct MaterialSurface {
    surface: Surface,
    material: Material,
    input: Entity<InputState>,
    dark: bool,
    selected: usize,
    page: usize,
    correlation: uuid::Uuid,
    benchmark: Option<benchmark::BenchmarkState>,
}
impl MaterialSurface {
    fn new(
        surface: Surface,
        material: Material,
        dark: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("搜索应用、文件或记录"));
        input.focus_handle(cx).focus(window, cx);
        super::defer_transparent_root(window, cx);
        Self {
            surface,
            material,
            input,
            dark,
            selected: 0,
            page: 0,
            correlation: uuid::Uuid::nil(),
            benchmark: None,
        }
    }
    fn toggle_theme(&mut self, window: &Window, cx: &mut Context<Self>) {
        self.dark = !self.dark;
        let dark = self.dark;
        let correlation = self.correlation;
        let handle = window.window_handle();
        cx.notify();
        cx.defer(move |cx| {
            super::change_theme(dark, cx);
            if handle
                .update(cx, |_, window, cx| {
                    super::defer_transparent_root(window, cx)
                })
                .is_ok()
            {
                println!(
                    "ui.material_preview.theme_requested correlation={correlation} dark={dark}"
                );
            }
        });
    }
    fn search(&self, large: bool, cx: &App) -> InputGroup {
        InputGroup::new("native-search")
            .aria_label("原生材质搜索框")
            .with_size(Size::Large)
            .focus_ring(false)
            .w_full()
            .h(px(if large { 60.0 } else { 44.0 }))
            .rounded(px(if large { 8.0 } else { 6.0 }))
            .bg(if large {
                self.material.fill(cx.theme().popover)
            } else {
                cx.theme().popover
            })
            .addon(
                InputGroupAddon::new("native-search-icon")
                    .pl(px(16.0))
                    .child(Icon::new(IconName::Search).size(px(20.0))),
            )
            .input(
                Input::new(&self.input)
                    .id("native-input")
                    .aria_label("原生主题输入")
                    .h(px(if large { 58.0 } else { 42.0 }))
                    .pl(px(12.0))
                    .pr(px(16.0))
                    .py(px(0.0))
                    .text_size(px(if large { 20.0 } else { 16.0 }))
                    .line_height(px(if large { 28.0 } else { 22.0 })),
            )
    }
    fn records(&self, clipboard: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let records = if clipboard {
            [
                ("周五前确认界面方案", "文本记录"),
                ("项目文档", "链接"),
                ("界面素材", "文件夹"),
            ]
        } else {
            [
                ("项目素材.zip", "下载中 · 128 MB / 256 MB"),
                ("使用说明.pdf", "已完成 · 12.4 MB"),
                ("界面资源.zip", "等待队列 · 36.8 MB"),
            ]
        };
        div()
            .flex()
            .flex_col()
            .gap_2()
            .children(
                records
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, detail))| {
                        ListItem::new(SharedString::from(format!("native-record-{index}")))
                            .accessibility_label(label)
                            .selected(self.selected == index)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected = index;
                                cx.notify();
                            }))
                            .child(
                                Icon::new(if clipboard {
                                    IconName::FileText
                                } else {
                                    IconName::Download
                                })
                                .size(px(20.0)),
                            )
                            .child(
                                div().flex().flex_col().gap_1().py_2().child(label).child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(detail),
                                ),
                            )
                    }),
            )
    }
    fn workbench(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut menu = SidebarMenu::new();
        for (index, (label, icon)) in [
            ("下载", IconName::Download),
            ("剪切板", IconName::Clipboard),
            ("外观", IconName::Settings),
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
        let body =
            if self.page == 2 {
                GroupBox::new()
                    .title("主题与配色")
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(if self.material == Material::Acrylic {
                                "磨砂亚克力主题 · 原生 DWM 候选"
                            } else {
                                "现有实底主题 · 对照"
                            })
                            .child("配色：Catppuccin · 字体：Sarasa UI SC")
                            .child(
                                Button::new("native-theme-button")
                                    .label("切换深浅色")
                                    .outline()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.toggle_theme(window, cx)
                                    })),
                            ),
                    )
                    .into_any_element()
            } else {
                GroupBox::new()
                    .title(if self.page == 1 {
                        "示例剪切板记录"
                    } else {
                        "示例下载任务"
                    })
                    .child(self.records(self.page == 1, cx))
                    .into_any_element()
            };
        div()
            .flex()
            .flex_1()
            .min_h_0()
            .child(
                Sidebar::new("native-sidebar")
                    .w(px(176.0))
                    .collapsible(false)
                    .bg(transparent_black())
                    .child(menu)
                    .footer(
                        div()
                            .p_3()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("原生 GPUI 预览"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap_5()
                    .p_5()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(div().text_xl().child(match self.page {
                                1 => "剪切板",
                                2 => "外观",
                                _ => "下载",
                            }))
                            .child(
                                Button::new("native-next-record")
                                    .label("选择下一条")
                                    .outline()
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.selected = (this.selected + 1) % 3;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(self.search(false, cx))
                    .child(body)
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("F2 切换深浅色 · Escape 关闭 · 不连接业务服务"),
                    ),
            )
    }
}
impl Render for MaterialSurface {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut root = div()
            .id("native-material-root")
            .size_full()
            .flex()
            .flex_col()
            .rounded(px(8.0))
            .text_color(cx.theme().foreground)
            .capture_key_down(
                cx.listener(|this, event: &gpui_kit::KeyDownEvent, window, cx| {
                    match event.keystroke.key.as_str() {
                        "f2" if this.benchmark.is_some() => {
                            cx.stop_propagation();
                        }
                        "escape" => {
                            window.remove_window();
                            cx.stop_propagation();
                        }
                        "f2" if !event.is_held => {
                            this.toggle_theme(window, cx);
                            cx.stop_propagation();
                        }
                        _ => {}
                    }
                }),
            );
        if self.surface == Surface::Search {
            return root.child(self.search(true, cx)).into_any_element();
        }
        root = root.bg(self.material.fill(cx.theme().background)).child(
            TitleBar::new()
                .bg(transparent_black())
                .child(if self.surface == Surface::Clipboard {
                    "剪切板 · 原生材质预览"
                } else {
                    "NxTools · 原生材质预览"
                }),
        );
        if self.benchmark.is_some() {
            return root
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_h_0()
                        .p_4()
                        .gap_4()
                        .child(self.search(false, cx))
                        .child(self.benchmark_list(cx)),
                )
                .into_any_element();
        }
        if self.surface == Surface::Workbench {
            root.child(self.workbench(cx)).into_any_element()
        } else {
            root.child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h_0()
                    .p_4()
                    .gap_4()
                    .child(self.search(false, cx))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("全部 · 文本 · 图片 · 文件"),
                    )
                    .child(self.records(true, cx))
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("F2 深浅色 · Escape 关闭 · 示例记录"),
                    ),
            )
            .into_any_element()
        }
    }
}

pub(super) fn run(arguments: &[String]) {
    let options = Options::parse(arguments).expect("valid isolated native material options");
    let correlation = arguments
        .iter()
        .find_map(|argument| argument.strip_prefix("--correlation="))
        .and_then(|value| uuid::Uuid::parse_str(value).ok())
        .unwrap_or_else(uuid::Uuid::new_v4);
    let started = std::time::Instant::now();
    let surface = options.surface;
    let material = options.material;
    println!(
        "ui.material_preview.started surface={} material={} correlation={correlation}",
        surface.tag(),
        material.tag()
    );
    gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(move |cx| {
        super::initialize(cx, options.dark);
        cx.on_window_closed(move |cx, _| {
            if cx.windows().is_empty() {
                println!("ui.material_preview.finished surface={} material={} correlation={correlation} elapsed_ms={}", surface.tag(), material.tag(), started.elapsed().as_millis());
                cx.quit();
            }
        }).detach();
        let surfaces: Vec<_> = if options.benchmark == Some(benchmark::Scenario::Multiwindow) {
            vec![Surface::Workbench, Surface::Clipboard, Surface::Search]
        } else { vec![surface] };
        let group_origin = Bounds::centered(None, size(px(1404.0), px(684.0)), cx).origin;
        for (index, surface) in surfaces.into_iter().enumerate() {
            let bounds = if options.benchmark == Some(benchmark::Scenario::Multiwindow) {
                let offset = match surface {
                    Surface::Workbench => gpui_kit::point(px(0.0), px(0.0)),
                    Surface::Clipboard => gpui_kit::point(px(924.0), px(0.0)),
                    Surface::Search => gpui_kit::point(px(0.0), px(624.0)),
                };
                Bounds::new(group_origin + offset, surface.dimensions())
            } else { Bounds::centered(None, surface.dimensions(), cx) };
            gpui_kit::open_window(WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: if surface == Surface::Search { None } else { Some(TitleBar::title_bar_options()) },
                kind: if surface == Surface::Search { WindowKind::PopUp } else { WindowKind::Normal },
                focus: index == 0,
                is_resizable: false,
                is_minimizable: false,
                window_background: WindowBackgroundAppearance::Transparent,
                app_id: Some("NxTools.NativeMaterialPreview".into()),
                ..TitleBar::window_options()
            }, cx, move |window, cx| {
                window.set_window_title(TITLE);
                cx.new(|cx| {
                    let mut view = MaterialSurface::new(surface, material, options.dark, window, cx);
                    view.correlation = correlation;
                    if let Some(scenario) = options.benchmark {
                        view.benchmark = Some(benchmark::BenchmarkState::new());
                        if index == 0 { view.arm_benchmark(scenario, options.benchmark_seconds, window, cx); }
                    }
                    view
                })
            }).expect("open actual isolated GPUI material window");
        }
        cx.activate(true);
    });
}

#[cfg(test)]
#[path = "material_tests.rs"]
mod tests;
