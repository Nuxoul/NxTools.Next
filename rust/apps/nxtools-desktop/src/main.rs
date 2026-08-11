mod assets;
mod clipboard;
mod download_taskbar_overlay;
mod download_window_lifecycle;
mod downloads;
mod icons;
mod main_window;
mod notifications;
mod platform_events;
mod popup_lifecycle;
mod quick_launch;
mod runtime;
mod shell;
mod theme;
mod ui;

use anyhow::{Context as _, Result};
use assets::NxToolsAssets;
use clipboard::{ClipboardApplication, ClipboardApplicationGlobal};
use downloads::{DownloadApplication, DownloadRuntimeContext};
use gpui::{
    App, AppContext, Application, TitlebarOptions, WindowBackgroundAppearance, WindowBounds,
    WindowDecorations, WindowKind, WindowOptions, px, size, transparent_black,
};
use gpui_component::{Root, Theme, ThemeMode};
use main_window::MainWindow;
use notifications::NotificationCenter;
use platform_events::{
    BrowserStartupWindow, PlatformEventDispatcher, PlatformEventDispatcherGlobal,
};
use quick_launch::QuickLaunchApplication;
use runtime::DesktopRuntime;
use sha2::{Digest, Sha256};
use std::backtrace::Backtrace;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::Once;
use std::time::Instant;
use tracing::{error, info};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopLaunchMode {
    Normal,
    BrowserDialog { correlation_id: Uuid },
    Background,
}

impl DesktopLaunchMode {
    fn from_arguments(arguments: impl IntoIterator<Item = String>) -> Result<Self> {
        let arguments = arguments.into_iter().collect::<Vec<_>>();
        let background = arguments.iter().any(|argument| argument == "--background");
        let browser_dialog = arguments
            .iter()
            .any(|argument| argument == "--browser-dialog");
        if background && browser_dialog {
            anyhow::bail!("desktop startup modes --background and --browser-dialog conflict");
        }
        if background {
            return Ok(Self::Background);
        }
        if !browser_dialog {
            return Ok(Self::Normal);
        }
        let correlation_id = arguments
            .windows(2)
            .find(|pair| pair[0] == "--browser-correlation-id")
            .context("--browser-dialog requires --browser-correlation-id")?
            .get(1)
            .context("--browser-dialog requires a correlation ID value")?
            .parse::<Uuid>()
            .context("--browser-correlation-id must be a UUID")?;
        Ok(Self::BrowserDialog { correlation_id })
    }

    const fn main_window_focus(self) -> bool {
        !matches!(self, Self::BrowserDialog { .. })
    }

    const fn main_window_show(self) -> bool {
        !matches!(self, Self::BrowserDialog { .. })
    }

    const fn correlation_id(self) -> Option<Uuid> {
        match self {
            Self::BrowserDialog { correlation_id } => Some(correlation_id),
            Self::Normal | Self::Background => None,
        }
    }
}

fn run(instance_wait: std::time::Duration, launch_mode: DesktopLaunchMode) -> Result<()> {
    let mut runtime = DesktopRuntime::initialize(instance_wait)?;
    install_panic_logging();
    let platform_bridge = runtime
        .platform_bridge
        .take()
        .context("desktop runtime did not provide a platform bridge")?;
    let indexer_bridge = runtime
        .indexer_bridge
        .take()
        .context("desktop runtime did not provide an indexer bridge")?;
    let clipboard_history = runtime
        .clipboard_history
        .take()
        .context("desktop runtime did not provide clipboard history")?;
    let clipboard_asset_dir = runtime
        .clipboard_asset_dir
        .take()
        .context("desktop runtime did not provide a clipboard image directory")?;
    let finder_catalog = runtime
        .finder_catalog
        .take()
        .context("desktop runtime did not provide a quick-launch catalog")?;
    let download_engine = runtime
        .download_engine
        .take()
        .context("desktop runtime did not provide a download engine")?;
    let download_default_directory = runtime.download_default_directory.clone();
    let download_database_path = runtime.download_database_path.clone();
    let ffmpeg_install_directory = runtime.ffmpeg_install_directory.clone();
    let app_config = runtime.config.clone();
    let config_path = runtime.config_path.clone();
    let log_dir = runtime.log_dir.clone();
    info!(pipe_name = %runtime.config.platform_pipe_name, "starting GPUI application");

    Application::new()
        .with_assets(NxToolsAssets)
        .run(move |cx: &mut App| {
            gpui_component::init(cx);
            Theme::change(ThemeMode::Dark, None, cx);
            let clear = transparent_black();
            let visual_theme = crate::theme::Theme::zed_dark();
            let component_theme = Theme::global_mut(cx);
            component_theme.colors.background = clear;
            component_theme.colors.window_border = clear;
            component_theme.colors.foreground = visual_theme.text.into();
            component_theme.colors.muted_foreground = visual_theme.placeholder.into();
            component_theme.colors.caret = visual_theme.accent.into();
            component_theme.colors.selection = visual_theme.accent_soft.into();
            component_theme.colors.border = visual_theme.border.into();
            component_theme.colors.input = visual_theme.border.into();
            component_theme.colors.ring = visual_theme.border_active.into();
            component_theme.colors.popover = visual_theme.surface_raised.into();
            component_theme.colors.popover_foreground = visual_theme.text.into();
            component_theme.colors.accent = visual_theme.element_selected.into();
            component_theme.colors.accent_foreground = visual_theme.text.into();
            component_theme.colors.muted = visual_theme.element.into();
            component_theme.colors.list = visual_theme.canvas.into();
            component_theme.colors.list_active = visual_theme.element_selected.into();
            component_theme.colors.list_active_border = visual_theme.border_subtle.into();
            component_theme.colors.list_hover = visual_theme.surface_hover.into();
            component_theme.colors.scrollbar = visual_theme.canvas.into();
            component_theme.colors.scrollbar_thumb = visual_theme.border.into();
            component_theme.colors.scrollbar_thumb_hover = visual_theme.text_muted.into();

            let notifications = cx.new(|_| NotificationCenter::default());
            let platform_bridge = Rc::new(platform_bridge);
            let indexer_bridge = Rc::new(indexer_bridge);
            let quick_launch = cx.new(|_| {
                QuickLaunchApplication::new(
                    finder_catalog,
                    Rc::clone(&platform_bridge),
                    Rc::clone(&indexer_bridge),
                    app_config.clone(),
                    config_path.clone(),
                    log_dir.clone(),
                    notifications.clone(),
                )
            });
            let indexer_events = indexer_bridge.event_receiver();
            let quick_launch_for_indexer = quick_launch.downgrade();
            cx.spawn(async move |cx| {
                while let Ok(event) = indexer_events.recv().await {
                    if quick_launch_for_indexer
                        .update(cx, |application, cx| {
                            application.handle_indexer_notification(event, cx);
                        })
                        .is_err()
                    {
                        return;
                    }
                }
            })
            .detach();
            let clipboard = cx.new(|cx| {
                ClipboardApplication::new(
                    clipboard_history,
                    clipboard_asset_dir,
                    Rc::clone(&platform_bridge),
                    app_config.clone(),
                    config_path.clone(),
                    notifications.clone(),
                    cx,
                )
            });
            let downloads = cx.new(|cx| {
                DownloadApplication::new(
                    download_engine,
                    Rc::clone(&platform_bridge),
                    notifications.clone(),
                    DownloadRuntimeContext {
                        fallback_default_directory: download_default_directory,
                        config: app_config.clone(),
                        config_path: config_path.clone(),
                        log_directory: log_dir.clone(),
                        database_path: download_database_path,
                        ffmpeg_install_directory,
                    },
                    cx,
                )
            });
            downloads.update(cx, |downloads, cx| {
                downloads.open_taskbar_overlay(cx);
            });
            cx.set_global(ClipboardApplicationGlobal::new(clipboard.clone()));

            let clipboard_for_window = clipboard.downgrade();
            let quick_launch_for_window = quick_launch.downgrade();
            let downloads_for_window = downloads.downgrade();
            let startup_started_at = Instant::now();
            let window = cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::centered(size(px(1060.0), px(500.0)), cx)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("NxTools".into()),
                        appears_transparent: true,
                        traffic_light_position: None,
                    }),
                    focus: launch_mode.main_window_focus(),
                    show: launch_mode.main_window_show(),
                    kind: WindowKind::Normal,
                    is_movable: true,
                    is_resizable: true,
                    is_minimizable: true,
                    display_id: None,
                    window_background: WindowBackgroundAppearance::Opaque,
                    app_id: Some("NxTools.Next".into()),
                    window_min_size: Some(size(px(920.0), px(480.0))),
                    window_decorations: Some(WindowDecorations::Client),
                    tabbing_identifier: None,
                },
                move |window, cx| {
                    let main_window = cx.new(|cx| {
                        MainWindow::new(
                            clipboard_for_window,
                            quick_launch_for_window,
                            downloads_for_window,
                            notifications.clone(),
                            window,
                            cx,
                        )
                    });
                    cx.new(|cx| Root::new(main_window, window, cx))
                },
            );
            let browser_startup_window = match (&window, launch_mode.correlation_id()) {
                (Ok(window), Some(correlation_id)) => {
                    match window.update(cx, |_, window, _| crate::ui::native_window_handle(window))
                    {
                        Ok(Some(window_handle)) if window_handle != 0 => {
                            Some(BrowserStartupWindow::new(
                                window_handle,
                                correlation_id,
                                startup_started_at,
                            ))
                        }
                        _ => {
                            error!(
                                event = "download.browser_startup.main_window_handle_failed",
                                success = false,
                                %correlation_id,
                                elapsed_ms = startup_started_at.elapsed().as_millis(),
                                "desktop could not resolve its hidden browser-startup main window"
                            );
                            None
                        }
                    }
                }
                _ => None,
            };
            if let Err(error) = window {
                tracing::error!(%error, "failed to open main window");
                cx.quit();
                return;
            }
            let dispatcher = cx.new(|cx| {
                PlatformEventDispatcher::new(
                    Rc::clone(&platform_bridge),
                    browser_startup_window,
                    clipboard.downgrade(),
                    quick_launch,
                    downloads,
                    cx,
                )
            });
            cx.set_global(PlatformEventDispatcherGlobal::new(dispatcher));
        });

    drop(runtime);
    Ok(())
}

fn install_panic_logging() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let default_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            let backtrace = Backtrace::force_capture();
            error!(
                panic = %panic_info,
                backtrace = %backtrace,
                "unhandled desktop panic"
            );
            default_hook(panic_info);
        }));
    });
}

fn main() {
    let instance_wait = protected_restart_instance_wait();
    let launch_mode = match DesktopLaunchMode::from_arguments(std::env::args()) {
        Ok(launch_mode) => launch_mode,
        Err(error) => {
            eprintln!("NxTools could not parse its startup intent: {error:#}");
            std::process::exit(2);
        }
    };
    match relaunch_protected_desktop_when_current() {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => eprintln!("NxTools could not use the protected desktop copy: {error:#}"),
    }
    let result = match launch_mode {
        DesktopLaunchMode::Background => runtime::run_background_download_host(instance_wait)
            .context("background download host startup failed"),
        DesktopLaunchMode::Normal | DesktopLaunchMode::BrowserDialog { .. } => {
            run(instance_wait, launch_mode).context("desktop startup failed")
        }
    };
    if let Err(error) = result {
        eprintln!("NxTools could not start: {error:#}");
        std::process::exit(1);
    }
}

fn protected_restart_instance_wait() -> std::time::Duration {
    let mut arguments = std::env::args();
    while let Some(argument) = arguments.next() {
        if argument == "--wait-for-instance-release-ms"
            && let Some(milliseconds) = arguments.next().and_then(|value| value.parse::<u64>().ok())
        {
            return std::time::Duration::from_millis(milliseconds.min(15_000));
        }
    }
    std::time::Duration::ZERO
}

fn relaunch_protected_desktop_when_current() -> Result<bool> {
    let current = std::env::current_exe().context("failed to locate desktop executable")?;
    let Some(protected) = protected_desktop_path() else {
        return Ok(false);
    };
    if paths_equal(&current, &protected)
        || !protected_package_matches_release(&current, &protected)?
    {
        return Ok(false);
    }
    Command::new(&protected)
        .args(std::env::args_os().skip(1))
        .spawn()
        .with_context(|| format!("failed to start protected desktop {}", protected.display()))?;
    Ok(true)
}

pub(crate) fn launch_protected_desktop_after_current_exit() -> Result<bool> {
    let current = std::env::current_exe().context("failed to locate desktop executable")?;
    let Some(protected) = protected_desktop_path() else {
        return Ok(false);
    };
    if paths_equal(&current, &protected)
        || !protected_package_matches_release(&current, &protected)?
    {
        return Ok(false);
    }
    Command::new(&protected)
        .args(["--wait-for-instance-release-ms", "10000"])
        .spawn()
        .with_context(|| {
            format!(
                "failed to restart from protected desktop {}",
                protected.display()
            )
        })?;
    Ok(true)
}

fn protected_desktop_path() -> Option<PathBuf> {
    std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .or_else(|| Some(PathBuf::from(r"C:\ProgramData")))
        .map(|root| {
            root.join("NxTools")
                .join("NxTools.Next")
                .join("Service")
                .join("NxTools.exe")
        })
}

fn protected_package_matches_release(release: &Path, protected: &Path) -> Result<bool> {
    if !matching_files(release, protected)? {
        return Ok(false);
    }
    let Some(release_directory) = release.parent() else {
        return Ok(false);
    };
    let Some(protected_directory) = protected.parent() else {
        return Ok(false);
    };
    for name in ["NxTools.Indexer.exe", "NxTools.PlatformHost.exe"] {
        if !matching_files(
            &release_directory.join(name),
            &protected_directory.join(name),
        )? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn matching_files(left: &Path, right: &Path) -> Result<bool> {
    if !left.is_file() || !right.is_file() {
        return Ok(false);
    }
    Ok(file_sha256(left)? == file_sha256(right)?)
}

fn file_sha256(path: &Path) -> Result<[u8; 32]> {
    let mut file =
        File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .with_context(|| format!("failed to read {}", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('/', "\\")
        .eq_ignore_ascii_case(
            &right
                .to_string_lossy()
                .trim_start_matches(r"\\?\")
                .replace('/', "\\"),
        )
}

#[cfg(test)]
mod protected_desktop_tests {
    use super::*;

    #[test]
    fn browser_dialog_launch_minimizes_main_window_without_claiming_focus() {
        let correlation_id = uuid::Uuid::new_v4();
        let launch_mode = DesktopLaunchMode::from_arguments([
            "NxTools.exe".to_string(),
            "--browser-dialog".to_string(),
            "--browser-correlation-id".to_string(),
            correlation_id.to_string(),
        ])
        .expect("valid browser startup intent");

        assert_eq!(
            launch_mode,
            DesktopLaunchMode::BrowserDialog { correlation_id }
        );
        assert!(!launch_mode.main_window_focus());
        assert!(!launch_mode.main_window_show());
        assert_eq!(launch_mode.correlation_id(), Some(correlation_id));
    }

    #[test]
    fn ordinary_and_background_launches_keep_their_existing_presentation() {
        let normal = DesktopLaunchMode::from_arguments(["NxTools.exe".to_string()])
            .expect("normal startup intent");
        let background = DesktopLaunchMode::from_arguments([
            "NxTools.exe".to_string(),
            "--background".to_string(),
        ])
        .expect("background startup intent");

        assert_eq!(normal, DesktopLaunchMode::Normal);
        assert!(normal.main_window_focus());
        assert!(normal.main_window_show());
        assert_eq!(background, DesktopLaunchMode::Background);
    }

    #[test]
    fn browser_dialog_launch_rejects_a_missing_or_malformed_correlation_id() {
        assert!(
            DesktopLaunchMode::from_arguments([
                "NxTools.exe".to_string(),
                "--browser-dialog".to_string(),
            ])
            .is_err()
        );
        assert!(
            DesktopLaunchMode::from_arguments([
                "NxTools.exe".to_string(),
                "--browser-dialog".to_string(),
                "--browser-correlation-id".to_string(),
                "not-a-uuid".to_string(),
            ])
            .is_err()
        );
    }

    #[test]
    fn only_an_identical_protected_package_replaces_the_release_launcher() {
        let temp = tempfile::tempdir().expect("temporary package copies");
        let release_dir = temp.path().join("release");
        let protected_dir = temp.path().join("protected");
        std::fs::create_dir_all(&release_dir).expect("release directory");
        std::fs::create_dir_all(&protected_dir).expect("protected directory");
        for (name, bytes) in [
            ("NxTools.exe", b"approved desktop".as_slice()),
            ("NxTools.Indexer.exe", b"approved indexer".as_slice()),
            (
                "NxTools.PlatformHost.exe",
                b"approved platform host".as_slice(),
            ),
        ] {
            std::fs::write(release_dir.join(name), bytes).expect("release binary");
            std::fs::write(protected_dir.join(name), bytes).expect("protected binary");
        }
        let release = release_dir.join("NxTools.exe");
        let protected = protected_dir.join("NxTools.exe");
        assert!(
            protected_package_matches_release(&release, &protected)
                .expect("compare identical packages")
        );
        std::fs::write(protected_dir.join("NxTools.Indexer.exe"), b"older indexer")
            .expect("replace protected indexer");
        assert!(
            !protected_package_matches_release(&release, &protected)
                .expect("compare different packages")
        );
    }
}
