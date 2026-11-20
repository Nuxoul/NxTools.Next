use std::{
    cmp::Ordering,
    time::{Duration, Instant},
};

use gpui_kit::base::Root;
use gpui_kit::component::{
    ActiveTheme, Sizable, Size, TitleBar,
    button::{Button, ButtonCustomVariant, ButtonVariants},
    description_list::DescriptionList,
};
use gpui_kit::{
    Animation, AnimationExt, AnyElement, ClipboardItem, Context, Entity, FontWeight, Hsla,
    IntoElement, Render, Task, Window, WindowBounds, WindowKind, WindowOptions, div, prelude::*,
    px, relative, rgb, rgba, size,
};
use tracing::{info, warn};
use uuid::Uuid;

use super::MainWindow;
use crate::ui::{motion::motion_allowed, typography::UI_MONO_FONT_FAMILY};

mod emblem;
mod update_download;
use update_download::{Release, ReleaseCheck, checked_release};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const TARGET: &str = env!("NXTOOLS_BUILD_TARGET");
const COMPILER: &str = match option_env!("NXTOOLS_RUSTC") {
    Some(version) => version,
    None => "未记录",
};
const COMMIT: Option<&str> = option_env!("NXTOOLS_BUILD_COMMIT");
const RENDERER: &str = "GPUI GPU-Accelerated";
const GITHUB: &str = "https://github.com/Nuxoul/NxTools.Next";
const LATEST_RELEASE: &str = "https://api.github.com/repos/Nuxoul/NxTools.Next/releases/latest";

impl MainWindow {
    pub(super) fn open_about(&mut self, cx: &mut Context<Self>) {
        let id = Uuid::new_v4();
        let start = Instant::now();
        info!(event = "desktop.about.open.start", correlation_id = %id);
        if let Some(handle) = self.about_window
            && handle
                .update(cx, |_, window, _| window.activate_window())
                .is_ok()
        {
            info!(event = "desktop.about.open.reused", correlation_id = %id, elapsed_ms = start.elapsed().as_millis() as u64);
            return;
        }

        let downloads = self.downloads.clone();
        let dimensions = size(px(420.), px(480.));
        let bounds = WindowBounds::centered(dimensions, cx);
        let mut titlebar = TitleBar::title_bar_options();
        titlebar.title = Some("关于 NxTools".into());
        let result = cx.open_window(
            WindowOptions {
                titlebar: Some(titlebar),
                window_bounds: Some(bounds),
                window_min_size: Some(dimensions),
                kind: WindowKind::Dialog,
                is_resizable: false,
                is_minimizable: false,
                ..TitleBar::window_options()
            },
            move |window, cx| {
                let view = cx.new(|cx| {
                    let mut view = About::new(id, cx);
                    view.downloads = Some(downloads);
                    view
                });
                About::observe_close(&view, window, cx);
                cx.new(|cx| Root::new(view, window, cx))
            },
        );
        match result {
            Ok(window) => {
                self.about_window = Some(window.into());
                info!(event = "desktop.about.open.ready", correlation_id = %id, elapsed_ms = start.elapsed().as_millis() as u64);
            }
            Err(error) => {
                self.about_window = None;
                let native_code = error
                    .chain()
                    .find_map(|cause| cause.downcast_ref::<std::io::Error>()?.raw_os_error());
                warn!(event = "desktop.about.open.failed", correlation_id = %id, elapsed_ms = start.elapsed().as_millis() as u64, ?native_code);
            }
        }
    }
}

struct About {
    id: Uuid,
    opened: Instant,
    copied: bool,
    copy_timer: Option<Task<()>>,
    licenses: bool,
    animate_page: bool,
    update: UpdateStatus,
    update_task: Option<Task<()>>,
    update_request: Option<Uuid>,
    downloads: Option<gpui_kit::WeakEntity<crate::downloads::DownloadApplication>>,
    closed: bool,
    emblem: Entity<emblem::Emblem>,
}

impl About {
    fn new(id: Uuid, cx: &mut Context<Self>) -> Self {
        Self {
            id,
            opened: Instant::now(),
            copied: false,
            copy_timer: None,
            licenses: false,
            animate_page: false,
            update: UpdateStatus::Idle,
            update_task: None,
            update_request: None,
            downloads: None,
            closed: false,
            emblem: cx.new(|_| emblem::Emblem::default()),
        }
    }

    fn copy(&mut self, cx: &mut Context<Self>) {
        let id = Uuid::new_v4();
        let start = Instant::now();
        info!(event = "desktop.about.copy.start", correlation_id = %id);
        cx.write_to_clipboard(ClipboardItem::new_string(format!(
            "NxTools v{VERSION}\nCommit: {}\n构建架构: {TARGET}\n渲染管线: {RENDERER}\n编译工具链: {COMPILER}",
            COMMIT.unwrap_or("未记录")
        )));
        info!(event = "desktop.about.copy.dispatched", correlation_id = %id, elapsed_ms = start.elapsed().as_millis() as u64);
        self.copied = true;
        let timer = cx.background_executor().timer(Duration::from_secs(2));
        self.copy_timer = Some(cx.spawn(async move |view, cx| {
            timer.await;
            let _ = view.update(cx, |view, cx| {
                view.copied = false;
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn check_updates(&mut self, cx: &mut Context<Self>) {
        if self.closed || self.update_request.is_some() {
            return;
        }
        let id = Uuid::new_v4();
        let started = Instant::now();
        self.update_request = Some(id);
        self.update = UpdateStatus::Checking;
        info!(event = "desktop.about.update.start", correlation_id = %id);
        let (mut tx, rx) = tokio::sync::oneshot::channel();
        let spawn = std::thread::Builder::new().name("nxtools-update-check".into()).spawn(move || {
            let result = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(runtime) => runtime.block_on(async {
                    tokio::select! {
                        _ = tx.closed() => None,
                        status = latest_release() => Some(status),
                    }
                }),
                Err(error) => Some(ReleaseCheck::status(UpdateStatus::Failed(UpdateFailure::Worker(error.raw_os_error())))),
            };
            if let Some(result) = result {
                info!(event = "desktop.about.update.finished", correlation_id = %id, elapsed_ms = started.elapsed().as_millis() as u64, outcome = result.status.outcome(), code = ?result.status.code());
                let _ = tx.send(result);
            } else {
                info!(event = "desktop.about.update.cancelled", correlation_id = %id, elapsed_ms = started.elapsed().as_millis() as u64);
            }
        });
        if let Err(error) = spawn {
            let native_code = error.raw_os_error();
            warn!(event = "desktop.about.update.spawn_failed", correlation_id = %id, elapsed_ms = started.elapsed().as_millis() as u64, ?native_code);
            self.update = UpdateStatus::Failed(UpdateFailure::Worker(native_code));
            self.update_request = None;
            cx.notify();
            return;
        }
        self.update = UpdateStatus::Checking;
        let minimum = cx.background_executor().timer(Duration::from_millis(1200));
        self.update_task = Some(cx.spawn(async move |view, cx| {
            minimum.await;
            let result = rx.await.unwrap_or_else(|_| {
                ReleaseCheck::status(UpdateStatus::Failed(UpdateFailure::Worker(None)))
            });
            let _ = view.update(cx, |view, cx| view.receive_update(id, started, result, cx));
        }));
        cx.notify();
    }

    fn show_licenses(&mut self, show: bool, cx: &mut Context<Self>) {
        self.licenses = show;
        self.animate_page = true;
        info!(event = "desktop.about.page.changed", correlation_id = %self.id, licenses = show);
        cx.notify();
    }

    fn overview(&self, palette: Palette, compact: bool, cx: &Context<Self>) -> AnyElement {
        let gap = px(if compact { 10. } else { 16. });
        let version = COMMIT.and_then(|commit| commit.get(..7)).map_or_else(
            || format!("v{VERSION}"),
            |commit| format!("v{VERSION} ({commit})"),
        );
        let hint = if self.copied {
            "已复制 ✓"
        } else {
            "复制"
        };
        let pill_border = if self.copied {
            palette.success.opacity(0.40)
        } else {
            palette.border
        };
        let copy = Button::new("about-copy")
            .accessibility_label(format!("{version}：{hint}版本诊断信息"))
            .with_size(Size::Medium)
            .rounded(px(999.))
            .px(px(12.))
            .h(px(30.))
            .custom(
                ButtonCustomVariant::new(cx)
                    .color(palette.card)
                    .foreground(palette.text)
                    .hover(palette.card_hover)
                    .active(palette.card_hover),
            )
            .border_1()
            .border_color(pill_border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(14.))
                    .text_size(px(12.))
                    .line_height(px(18.))
                    .child(div().font_family(UI_MONO_FONT_FAMILY).child(version))
                    .child(
                        div()
                            .text_color(if self.copied {
                                palette.success
                            } else {
                                palette.muted
                            })
                            .child(hint),
                    ),
            )
            .on_click(cx.listener(|view, _, _, cx| view.copy(cx)));
        let brand = div()
            .flex()
            .flex_col()
            .items_center()
            .flex_none()
            .gap(px(if compact { 4. } else { 10. }))
            .child(self.emblem.clone())
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.))
                    .child(
                        div()
                            .text_size(px(22.))
                            .line_height(px(28.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("NxTools"),
                    )
                    .child(
                        div()
                            .text_size(px(13.))
                            .line_height(px(18.))
                            .text_color(palette.secondary)
                            .child("下载、剪贴板与快速查找，一站即达"),
                    ),
            )
            .child(div().debug_selector(|| "about-version".into()).child(copy));

        let mut metadata = DescriptionList::horizontal()
            .bordered(false)
            .columns(1)
            .label_width(px(84.))
            .small();
        let compiler = COMPILER
            .split_once(" (")
            .map_or(COMPILER, |(version, _)| version);
        for (index, (key, value)) in [
            ("构建架构", TARGET),
            ("渲染管线", RENDERER),
            ("编译工具链", compiler),
        ]
        .into_iter()
        .enumerate()
        {
            let row = |color: Hsla| {
                div()
                    .text_color(color)
                    .text_size(px(12.))
                    .line_height(px(17.))
                    .py(px(4.))
                    .when(index != 2, |this| {
                        this.border_b_1().border_color(palette.border)
                    })
            };
            metadata = metadata.item(
                row(palette.secondary).child(key).into_any_element(),
                row(palette.text)
                    .font_family(UI_MONO_FONT_FAMILY)
                    .font_weight(FontWeight::MEDIUM)
                    .text_right()
                    .child(value)
                    .into_any_element(),
                1,
            );
        }
        let specifications = div()
            .debug_selector(|| "about-specifications".into())
            .w_full()
            .flex_none()
            .p(px(if compact { 12. } else { 16. }))
            .rounded(px(12.))
            .border_1()
            .border_color(palette.border)
            .bg(palette.card)
            .child(metadata);

        let actions = div().debug_selector(|| "about-actions".into()).w_full().flex().gap(px(10.))
            .child(action("about-updates", self.update.button_label(), palette, true, cx)
                .loading(matches!(self.update, UpdateStatus::Checking | UpdateStatus::Submitting))
                .on_click(cx.listener(|view, _, _, cx| view.check_updates(cx))))
            .child(action("about-licenses", "开源许可", palette, false, cx)
                .on_click(cx.listener(|view, _, _, cx| view.show_licenses(true, cx))))
            .child(action("about-github", "GitHub", palette, false, cx)
                .on_click(|_, _, cx| {
                    let id = Uuid::new_v4();
                    let start = Instant::now();
                    info!(event = "desktop.about.github.start", correlation_id = %id);
                    cx.open_url(GITHUB);
                    info!(event = "desktop.about.github.dispatched", correlation_id = %id, elapsed_ms = start.elapsed().as_millis() as u64);
                }));
        let footer = div()
            .w_full()
            .flex_none()
            .flex()
            .flex_col()
            .items_center()
            .child(actions)
            .child(
                div()
                    .debug_selector(|| "about-update-status".into())
                    .w_full()
                    .h(px(18.))
                    .text_center()
                    .text_size(px(11.))
                    .line_height(px(18.))
                    .text_color(if matches!(self.update, UpdateStatus::Current) {
                        palette.success
                    } else {
                        palette.secondary
                    })
                    .children(self.update.message()),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.))
                    .text_size(px(11.))
                    .line_height(px(16.))
                    .text_color(palette.secondary)
                    .child(
                        div()
                            .debug_selector(|| "about-author".into())
                            .font_weight(FontWeight::MEDIUM)
                            .child("作者 @Nuxoul"),
                    )
                    .child("·")
                    .child(
                        div()
                            .debug_selector(|| "about-copyright".into())
                            .child("© 2026 NxTools contributors"),
                    ),
            );
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(gap)
            .child(
                div()
                    .id("about-overview-scroll")
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .w_full()
                            .min_h(relative(1.))
                            .flex()
                            .flex_col()
                            .items_center()
                            .justify_between()
                            .gap(gap)
                            .child(brand)
                            .child(specifications),
                    ),
            )
            .child(footer)
            .into_any_element()
    }

    fn license_view(&self, palette: Palette, cx: &Context<Self>) -> AnyElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap(px(16.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.))
                    .child(
                        div()
                            .text_size(px(22.))
                            .line_height(px(28.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child("开源许可"),
                    )
                    .child(
                        action("about-back", "返回", palette, false, cx)
                            .flex_none()
                            .on_click(cx.listener(|view, _, _, cx| view.show_licenses(false, cx))),
                    ),
            )
            .child(
                div()
                    .id("about-license-list")
                    .debug_selector(|| "about-license-list".into())
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(16.))
                    .rounded(px(12.))
                    .border_1()
                    .border_color(palette.border)
                    .bg(palette.card)
                    .children(
                        [
                            ("NxTools", "MIT 或 Apache-2.0"),
                            ("GPUI / GPUI Kit", "Apache-2.0"),
                            ("Google Material Symbols", "Apache-2.0"),
                            ("Sarasa UI SC", "SIL OFL-1.1"),
                            ("Tokio / Serde", "MIT / MIT 或 Apache-2.0"),
                            ("reqwest / image", "MIT 或 Apache-2.0"),
                        ]
                        .map(|(name, license)| {
                            div()
                                .flex()
                                .justify_between()
                                .gap(px(8.))
                                .py(px(8.))
                                .border_b_1()
                                .border_color(palette.border)
                                .text_size(px(12.))
                                .child(name)
                                .child(div().text_color(palette.secondary).child(license))
                        }),
                    )
                    .child(
                        div()
                            .mt(px(12.))
                            .text_size(px(11.))
                            .line_height(px(17.))
                            .text_color(palette.secondary)
                            .child("感谢这些开源项目。以下为 NxTools 随附的许可文本。"),
                    )
                    .children(
                        [
                            include_str!("../../../../../LICENSE-MIT"),
                            include_str!("../../../../../LICENSE-APACHE"),
                            crate::assets::SARASA_FONT_LICENSE,
                        ]
                        .map(|license| {
                            div()
                                .mt(px(12.))
                                .text_size(px(11.))
                                .line_height(px(17.))
                                .text_color(palette.secondary)
                                .child(license)
                        }),
                    ),
            )
            .into_any_element()
    }
}

impl Drop for About {
    fn drop(&mut self) {
        info!(event = "desktop.about.closed", correlation_id = %self.id, elapsed_ms = self.opened.elapsed().as_millis() as u64);
    }
}

impl Render for About {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let palette = Palette::new(cx.theme().mode.is_dark());
        let compact = window.viewport_size().height < px(464.);
        let page = if self.licenses {
            self.license_view(palette, cx)
        } else {
            self.overview(palette, compact, cx)
        };
        let page = if self.animate_page && motion_allowed(cx) {
            let animation_id = if self.licenses {
                "about-licenses-in"
            } else {
                "about-overview-in"
            };
            div()
                .size_full()
                .child(page)
                .with_animation(
                    animation_id,
                    Animation::new(Duration::from_millis(200)),
                    |view, progress| view.opacity(progress),
                )
                .into_any_element()
        } else {
            page
        };
        div()
            .relative()
            .size_full()
            .bg(palette.canvas)
            .text_color(palette.text)
            .child(
                div()
                    .size_full()
                    .px(px(32.))
                    .pt(px(36.))
                    .pb(px(if compact { 20. } else { 32. }))
                    .child(page),
            )
            .child(
                div().absolute().top_0().left_0().right_0().child(
                    TitleBar::new()
                        .bg(gpui_kit::transparent_black())
                        .border_b_0(),
                ),
            )
    }
}

fn action(
    id: &'static str,
    label: &'static str,
    palette: Palette,
    primary: bool,
    cx: &Context<About>,
) -> Button {
    let fill = if primary {
        palette.accent_fill
    } else {
        palette.button
    };
    let text = if primary {
        palette.accent_text
    } else {
        palette.button_text
    };
    Button::new(id)
        .label(label)
        .with_size(Size::Medium)
        .flex_1()
        .min_w_0()
        .h(px(32.))
        .px(px(6.))
        .rounded(px(8.))
        .font_weight(FontWeight::MEDIUM)
        .text_size(px(12.))
        .custom(
            ButtonCustomVariant::new(cx)
                .color(fill)
                .foreground(text)
                .hover(if primary {
                    palette.accent_hover
                } else {
                    palette.card_hover
                })
                .active(palette.card_hover),
        )
        .border_1()
        .border_color(if primary {
            palette.accent_border
        } else {
            palette.border
        })
}

#[derive(Clone, Copy)]
struct Palette {
    canvas: Hsla,
    card: Hsla,
    card_hover: Hsla,
    border: Hsla,
    text: Hsla,
    secondary: Hsla,
    muted: Hsla,
    success: Hsla,
    button: Hsla,
    button_text: Hsla,
    accent_fill: Hsla,
    accent_text: Hsla,
    accent_border: Hsla,
    accent_hover: Hsla,
}

impl Palette {
    fn new(dark: bool) -> Self {
        if dark {
            Self {
                canvas: rgb(0x0D0E12).into(),
                card: rgba(0x161821CC).into(),
                card_hover: rgb(0x242836).into(),
                border: rgba(0xFFFFFF14).into(),
                text: rgb(0xF3F4F6).into(),
                secondary: rgb(0x9CA3AF).into(),
                muted: rgb(0x818A99).into(),
                success: rgb(0x34D399).into(),
                button: rgba(0xFFFFFF08).into(),
                button_text: rgb(0xD1D5DB).into(),
                accent_fill: rgba(0x3B82F626).into(),
                accent_text: rgb(0x93C5FD).into(),
                accent_border: rgba(0x60A5FA40).into(),
                accent_hover: rgba(0x3B82F644).into(),
            }
        } else {
            Self {
                canvas: rgb(0xF8FAFC).into(),
                card: rgba(0xFFFFFFD9).into(),
                card_hover: rgb(0xEDF2F8).into(),
                border: rgba(0x0F172A14).into(),
                text: rgb(0x0F172A).into(),
                secondary: rgb(0x475569).into(),
                muted: rgb(0x64748B).into(),
                success: rgb(0x087451).into(),
                button: rgba(0x0F172A06).into(),
                button_text: rgb(0x334155).into(),
                accent_fill: rgba(0x3B82F61C).into(),
                accent_text: rgb(0x174BA0).into(),
                accent_border: rgba(0x2563EB40).into(),
                accent_hover: rgba(0x3B82F633).into(),
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum UpdateFailure {
    Worker(Option<i32>),
    Network,
    Http(u16),
    InvalidRelease,
    InstallerUnavailable,
    Download,
}

#[derive(Clone, Copy)]
enum UpdateStatus {
    Idle,
    Checking,
    Current,
    Newer,
    Submitting,
    Queued,
    Unavailable,
    Failed(UpdateFailure),
}

impl UpdateStatus {
    fn button_label(self) -> &'static str {
        if matches!(self, Self::Checking | Self::Submitting) {
            "检查中"
        } else {
            "检查更新"
        }
    }

    fn message(self) -> Option<&'static str> {
        match self {
            Self::Idle | Self::Checking | Self::Submitting => None,
            Self::Current => Some("已是最新版本"),
            Self::Newer => Some("发现新版本"),
            Self::Queued => Some("安装包已加入下载器"),
            Self::Failed(UpdateFailure::InstallerUnavailable) => Some("新版本暂无适用的安装包"),
            Self::Failed(UpdateFailure::Download) => Some("无法加入下载器，请稍后重试"),
            Self::Unavailable => Some("暂无可查询的公开版本"),
            Self::Failed(_) => Some("无法确认最新版本，请稍后重试"),
        }
    }

    fn outcome(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Checking => "checking",
            Self::Current => "current",
            Self::Newer => "newer",
            Self::Submitting => "submitting",
            Self::Queued => "queued",
            Self::Unavailable => "unavailable",
            Self::Failed(UpdateFailure::Worker(_)) => "worker",
            Self::Failed(UpdateFailure::Network) => "network",
            Self::Failed(UpdateFailure::Http(_)) => "http",
            Self::Failed(UpdateFailure::InvalidRelease) => "invalid_release",
            Self::Failed(UpdateFailure::InstallerUnavailable) => "installer_unavailable",
            Self::Failed(UpdateFailure::Download) => "download_rejected",
        }
    }

    fn code(self) -> Option<i32> {
        match self {
            Self::Failed(UpdateFailure::Worker(code)) => code,
            Self::Failed(UpdateFailure::Http(code)) => Some(i32::from(code)),
            _ => None,
        }
    }
}

async fn latest_release() -> ReleaseCheck {
    async fn request() -> Result<ReleaseCheck, UpdateFailure> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(6))
            .user_agent("NxTools")
            .build()
            .map_err(|_| UpdateFailure::Network)?;
        let mut response = client
            .get(LATEST_RELEASE)
            .send()
            .await
            .map_err(|_| UpdateFailure::Network)?;
        match response.status() {
            reqwest::StatusCode::NOT_FOUND => {
                return Ok(ReleaseCheck::status(UpdateStatus::Unavailable));
            }
            status if !status.is_success() => return Err(UpdateFailure::Http(status.as_u16())),
            _ => {}
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| UpdateFailure::Network)? {
            if body.len() + chunk.len() > 256 * 1024 {
                return Err(UpdateFailure::InvalidRelease);
            }
            body.extend_from_slice(&chunk);
        }
        let release: Release =
            serde_json::from_slice(&body).map_err(|_| UpdateFailure::InvalidRelease)?;
        checked_release(VERSION, TARGET, release)
    }
    request()
        .await
        .unwrap_or_else(|failure| ReleaseCheck::status(UpdateStatus::Failed(failure)))
}

fn compare_release(current: &str, latest: &str) -> Option<UpdateStatus> {
    fn version(tag: &str) -> Option<[u32; 3]> {
        let mut parts = tag.strip_prefix('v').unwrap_or(tag).split('.');
        let parsed = [
            parts.next()?.parse().ok()?,
            parts.next()?.parse().ok()?,
            parts.next()?.parse().ok()?,
        ];
        parts.next().is_none().then_some(parsed)
    }
    Some(match version(current)?.cmp(&version(latest)?) {
        Ordering::Less => UpdateStatus::Newer,
        _ => UpdateStatus::Current,
    })
}

#[cfg(test)]
mod tests;
