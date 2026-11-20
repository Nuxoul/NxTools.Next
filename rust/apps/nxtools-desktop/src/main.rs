#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// GPUI derive output resolves through the Kit, without a direct GPUI dependency.
extern crate gpui_kit as gpui;

mod archive;
#[cfg(test)]
mod archive_component_spike;
mod archive_launch;
mod assets;
mod clipboard;
mod clipboard_worker;
mod completion_actions;
#[cfg(test)]
#[path = "../../../crates/nxtools-download/tests/support/b4_url.rs"]
mod completion_http_fixture;
mod data_directory;
mod download_taskbar_overlay;
mod download_window_lifecycle;
mod downloads;
mod extensions;
mod icons;
#[cfg(windows)]
mod install_archive_integration;
mod main_window;
mod markdown_launch;
mod network_settings;
mod notifications;
mod platform_events;
mod popup_lifecycle;
mod quick_launch;
mod runtime;
mod shell;
mod startup_failure;
mod theme;
#[cfg(test)]
mod theme_graphite_tests;
mod tools;
mod ui;
#[cfg(test)]
mod windows_icon;

use anyhow::{Context as _, Result};
use assets::NxToolsAssets;
use clipboard::{ClipboardApplication, ClipboardApplicationGlobal};
use downloads::{DownloadApplication, DownloadRuntimeContext};
use gpui_kit::base::Root;
use gpui_kit::{
    App, AppContext, TitlebarOptions, WindowBackgroundAppearance, WindowBounds, WindowDecorations,
    WindowKind, WindowOptions, px, size,
};
use main_window::{MainWindow, MainWindowShellContext};
use network_settings::NetworkSettingsApplication;
use notifications::NotificationCenter;
use nxtools_core::{AppPaths, SingleInstanceGuard, SystemSettings};
use nxtools_protocol::desktop_markdown_intent::{
    DesktopMarkdownIntent, LaunchFailure as MarkdownLaunchFailure,
};
use platform_events::{
    BrowserStartupWindow, PlatformApplicationTargets, PlatformEventDispatcher,
    PlatformEventDispatcherGlobal,
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
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopLaunchMode {
    Normal,
    SilentStart,
    BrowserDialog { correlation_id: Uuid },
    Background,
}

const MAIN_WINDOW_LOGICAL_WIDTH: f32 = 1060.0;
const MAIN_WINDOW_LOGICAL_HEIGHT: f32 = 650.0;

fn main_window_logical_size() -> gpui_kit::Size<gpui_kit::Pixels> {
    size(
        px(MAIN_WINDOW_LOGICAL_WIDTH),
        px(MAIN_WINDOW_LOGICAL_HEIGHT),
    )
}

struct MainWindowPlacement {
    bounds: WindowBounds,
    display_id: Option<gpui_kit::DisplayId>,
}

fn main_window_placement_for_display(
    display: Option<(gpui_kit::DisplayId, gpui_kit::Bounds<gpui_kit::Pixels>)>,
) -> MainWindowPlacement {
    let window_size = main_window_logical_size();
    match display {
        Some((display_id, display_bounds)) => MainWindowPlacement {
            bounds: WindowBounds::Windowed(gpui_kit::Bounds::centered_at(
                display_bounds.center(),
                window_size,
            )),
            display_id: Some(display_id),
        },
        None => MainWindowPlacement {
            bounds: WindowBounds::Windowed(gpui_kit::Bounds::new(
                gpui_kit::point(px(0.0), px(0.0)),
                window_size,
            )),
            display_id: None,
        },
    }
}

fn main_window_placement(cx: &App) -> MainWindowPlacement {
    let display = cx
        .primary_display()
        .map(|display| (display.id(), display.bounds()));
    main_window_placement_for_display(display)
}

impl DesktopLaunchMode {
    fn from_arguments<S: AsRef<std::ffi::OsStr>>(
        arguments: impl IntoIterator<Item = S>,
    ) -> Result<Self> {
        let arguments = arguments
            .into_iter()
            .map(|argument| argument.as_ref().to_os_string())
            .collect::<Vec<_>>();
        let background = arguments.iter().any(|argument| argument == "--background");
        let silent_start = arguments
            .iter()
            .any(|argument| argument == "--silent-start");
        let browser_dialog = arguments
            .iter()
            .any(|argument| argument == "--browser-dialog");
        if [background, browser_dialog, silent_start]
            .into_iter()
            .filter(|enabled| *enabled)
            .count()
            > 1
        {
            anyhow::bail!("desktop startup modes --background and --browser-dialog conflict");
        }
        if background {
            return Ok(Self::Background);
        }
        if silent_start {
            return Ok(Self::SilentStart);
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
            .to_str()
            .context("--browser-correlation-id must be Unicode")?
            .parse::<Uuid>()
            .context("--browser-correlation-id must be a UUID")?;
        Ok(Self::BrowserDialog { correlation_id })
    }

    const fn main_window_focus(self) -> bool {
        !matches!(self, Self::BrowserDialog { .. } | Self::SilentStart)
    }

    const fn main_window_show(self) -> bool {
        !matches!(self, Self::BrowserDialog { .. } | Self::SilentStart)
    }

    const fn should_summon_running_instance(self) -> bool {
        !matches!(self, Self::SilentStart)
    }

    const fn startup_failure_is_interactive(self) -> bool {
        !matches!(self, Self::Background | Self::SilentStart)
    }

    const fn correlation_id(self) -> Option<Uuid> {
        match self {
            Self::BrowserDialog { correlation_id } => Some(correlation_id),
            Self::Normal | Self::SilentStart | Self::Background => None,
        }
    }
}

fn startup_main_window_presentation(
    mode: DesktopLaunchMode,
    archive_origin: bool,
    markdown_origin: bool,
) -> (bool, bool) {
    if markdown_origin {
        (false, false)
    } else {
        archive_launch::main_window_presentation(mode, archive_origin)
    }
}

fn run(
    instance_wait: std::time::Duration,
    launch_mode: DesktopLaunchMode,
    initial_archive: Option<nxtools_protocol::desktop_archive_intent::DesktopArchiveIntent>,
    initial_markdown: Option<DesktopMarkdownIntent>,
) -> Result<()> {
    let markdown_origin = initial_markdown.is_some();
    let (main_focus, main_show) =
        startup_main_window_presentation(launch_mode, initial_archive.is_some(), markdown_origin);
    let offer_integration_setup = main_show && matches!(launch_mode, DesktopLaunchMode::Normal);
    let mut runtime = DesktopRuntime::initialize(instance_wait, initial_archive, initial_markdown)?;
    install_panic_logging();
    let platform_bridge = runtime
        .platform_bridge
        .take()
        .context("desktop runtime did not provide a platform bridge")?;
    let indexer_bridge = runtime
        .indexer_bridge
        .take()
        .context("desktop runtime did not provide an indexer bridge")?;
    let clipboard_worker = runtime
        .clipboard_worker
        .take()
        .context("desktop runtime did not provide clipboard history worker")?;
    let archive_launches = runtime
        .archive_launches
        .take()
        .context("desktop archive ingress unavailable")?;
    let markdown_launches = runtime
        .markdown_launches
        .take()
        .context("desktop Markdown ingress unavailable")?;
    let finder_catalog = runtime
        .finder_catalog
        .take()
        .context("desktop runtime did not provide a quick-launch catalog")?;
    let download_engine = runtime
        .download_engine
        .take()
        .context("desktop runtime did not provide a download engine")?;
    let download_shutdown = download_engine.clone();
    let download_default_directory = runtime.download_default_directory.clone();
    let download_database_path = runtime.download_database_path.clone();
    let ffmpeg_install_directory = runtime.ffmpeg_install_directory.clone();
    let network_policy = runtime.network_policy.clone();
    let app_config = runtime.config.clone();
    let config_path = runtime.config_path.clone();
    let archive_ui_state_path = nxtools_core::AppPaths::discover()?
        .data_dir
        .join("archive-ui-state.json");
    let log_dir = runtime.log_dir.clone();
    let extension_service = runtime.extension_service.clone();
    let extension_ui_calls = runtime.extension_ui_calls.take();
    info!(pipe_name = %runtime.config.platform_pipe_name, "starting GPUI application");

    let worker_shutdown = clipboard_worker.clone();
    let completion_exit_correlation = std::sync::Arc::new(std::sync::Mutex::new(None));
    let exit_correlation_for_application = completion_exit_correlation.clone();
    let migration_slot = data_directory::MigrationSlot::default();
    let migration_for_ui = migration_slot.clone();
    let migration_result = data_directory::take_result(&AppPaths::discover()?);
    gpui_kit::application()
        .with_assets(NxToolsAssets)
        .run(move |cx: &mut App| {
            cx.set_global(migration_for_ui);
            cx.set_global(crate::completion_actions::CompletionExitCorrelation(
                exit_correlation_for_application,
            ));
            gpui_kit::init(cx);
            crate::ui::markdown_document::init(cx);
            //Unknown native animation preference is conservative until PlatformHost's snapshot.
            cx.set_reduce_motion(true);
            crate::theme::apply_component_theme(cx);
            crate::theme::set_theme_preference(app_config.system.theme, cx);
            crate::theme::set_ui_font_family(&app_config.system.ui_font_family, cx);
            crate::tools::markdown_reader::install_settings(&app_config, config_path.clone(), cx);

            let notifications = cx.new(|_| NotificationCenter::default());
            crate::notifications::register(&notifications, cx);
            notifications.update(cx, |center, cx| {
                center.set_position(app_config.system.notification_position, cx);
                center.set_duration_seconds(app_config.system.notification_duration_seconds, cx);
            });
            if let Some(success) = migration_result {
                notifications.update(cx, |center, cx| {
                    if success {
                        center.show_success("data-directory-migrated", "软件数据迁移完成", "已使用新数据目录；原目录保留为备份。", cx);
                    } else {
                        center.show_error("data-directory-migration-failed", "软件数据未能迁移", "仍使用原数据目录，原数据完整保留。请检查目标目录权限、可用空间后重试。", cx);
                    }
                });
            }
            let platform_bridge = Rc::new(platform_bridge);
            notifications.update(cx, |center, _| {
                center.bind_copy_platform(Rc::clone(&platform_bridge))
            });
            crate::archive::register(
                Rc::clone(&platform_bridge),
                app_config.archive.clone(),
                archive_ui_state_path,
                cx,
            );
            crate::archive::set_config_path(config_path.clone(), cx);
            crate::archive_launch::start(archive_launches, Rc::clone(&platform_bridge), cx);
            cx.set_global(crate::extensions::ui_host::ExtensionServiceGlobal(
                extension_service.clone(),
            ));
            // Close-to-tray is an active platform capability, not merely a persisted preference.
            // Keep it disabled until PlatformHost confirms that the tray icon and startup adapter
            // were configured, otherwise a failed tray creation could hide the only UI surface.
            let system_settings = Rc::new(std::cell::RefCell::new(SystemSettings {
                start_on_login_silently: false,
                close_to_tray: false,
                theme: app_config.system.theme,
                ui_font_family: app_config.system.ui_font_family.clone(),
                notification_position: app_config.system.notification_position,
                notification_duration_seconds: app_config.system.notification_duration_seconds,
            }));
            let network_settings = cx.new(|_| {
                NetworkSettingsApplication::new(
                    network_policy.clone(),
                    app_config.clone(),
                    config_path.clone(),
                    notifications.clone(),
                    Some(Rc::clone(&platform_bridge)),
                    Rc::clone(&system_settings),
                    matches!(launch_mode, DesktopLaunchMode::SilentStart),
                )
            });
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
                    clipboard_worker,
                    Rc::clone(&platform_bridge),
                    app_config.clone(),
                    config_path.clone(),
                    notifications.clone(),
                    cx,
                )
            });
            clipboard.update(cx, |owner, cx| {
                owner.bind_currency_network(network_policy.clone(), cx)
            });
            network_settings.update(cx, |owner, _| {
                owner.bind_clipboard(clipboard.downgrade());
                owner.bind_quick_launch(quick_launch.downgrade());
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
            clipboard.update(cx, |clipboard, _| {
                clipboard.bind_downloads(downloads.downgrade());
            });
            downloads.update(cx, |downloads, cx| {
                downloads.open_taskbar_overlay(cx);
            });
            cx.set_global(ClipboardApplicationGlobal::new(clipboard.clone()));
            if let Some(receiver) = extension_ui_calls {
                crate::extensions::ui_host::start_ui_host(receiver, cx);
            }

            let clipboard_for_window = clipboard.downgrade();
            let quick_launch_for_window = quick_launch.downgrade();
            let downloads_for_window = downloads.downgrade();
            let network_settings_for_window = Some(network_settings.clone());
            let platform_bridge_for_window = Rc::clone(&platform_bridge);
            let close_platform = Rc::clone(&platform_bridge);
            let close_behavior = Rc::clone(&system_settings);
            let startup_started_at = Instant::now();
            let screen_notifications = notifications.clone();
            let placement = main_window_placement(cx);
            let window = cx.open_window(
                WindowOptions {
                    window_bounds: Some(placement.bounds),
                    titlebar: Some(TitlebarOptions {
                        title: Some("NxTools".into()),
                        appears_transparent: true,
                        traffic_light_position: None,
                    }),
                    focus: main_focus,
                    show: main_show,
                    kind: WindowKind::Normal,
                    is_movable: true,
                    is_resizable: true,
                    is_minimizable: true,
                    display_id: placement.display_id,
                    window_background: WindowBackgroundAppearance::Opaque,
                    app_id: Some("NxTools.Next".into()),
                    window_min_size: Some(size(px(920.0), px(480.0))),
                    window_decorations: Some(WindowDecorations::Client),
                    app_owns_titlebar_drag: false,
                    inactive_frame_interval: None,
                    icon: None,
                    tabbing_identifier: None,
                },
                move |window, cx| {
                    let close_platform = Rc::clone(&close_platform);
                    let close_behavior = Rc::clone(&close_behavior);
                    let native_window_handle =
                        crate::ui::native_window_handle(window).unwrap_or_default();
                    window.on_window_should_close(cx, move |_, _| {
                        if !close_behavior.borrow().close_to_tray || native_window_handle == 0 {
                            return true;
                        }
                        let hidden = close_platform.send_command(
                            nxtools_protocol::PlatformCommand::SetWindowVisibility(
                                nxtools_protocol::SetWindowVisibilityCommand {
                                    window_handle: native_window_handle,
                                    visible: false,
                                },
                            ),
                        );
                        hidden.is_err()
                    });
                    let main_window = cx.new(|cx| {
                        MainWindow::new(
                            clipboard_for_window,
                            quick_launch_for_window,
                            downloads_for_window,
                            MainWindowShellContext {
                                settings: network_settings_for_window,
                                platform: Some(Rc::clone(&platform_bridge_for_window)),
                                system_settings: Rc::clone(&system_settings),
                            },
                            notifications.clone(),
                            window,
                            cx,
                        )
                    });
                    if offer_integration_setup {
                        let main = main_window.clone();
                        let source = window.window_handle();
                        cx.defer(move |cx| {
                            let _ = source.update(cx, |_, window, cx| {
                                main.update(cx, |main, cx| {
                                    main.maybe_show_integration_setup(window, cx);
                                });
                            });
                        });
                    }
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
            let main_window_handle = window
                .as_ref()
                .ok()
                .and_then(|window| {
                    window
                        .update(cx, |_, window, _| crate::ui::native_window_handle(window))
                        .ok()
                        .flatten()
                })
                .unwrap_or_default();
            let main_source: Option<gpui_kit::AnyWindowHandle> =
                window.as_ref().ok().map(|source| (*source).into());
            let prepared_notifications = screen_notifications.clone();
            let prepared_platform = Rc::clone(&platform_bridge);
            let prepared_downloads = downloads.downgrade();
            let prepared_quick_launch = quick_launch.downgrade();
            let prepared_config = app_config.clone();
            let prepared_config_path = config_path.clone();
            let publish_main = move |cx: &mut App| {
                let Some(source) = main_source else {
                    return;
                };
                if !cx.windows().contains(&source) {
                    return;
                }
                crate::notifications::screen::attach(
                    prepared_notifications.clone(),
                    Rc::clone(&prepared_platform),
                    source,
                    cx,
                );
                if main_window_handle != 0
                    && let Ok(paths) = AppPaths::discover()
                {
                    SingleInstanceGuard::record_main_window_handle(
                        &paths,
                        "desktop",
                        main_window_handle,
                    );
                }
                let assistant = cx.new(|cx| {
                    crate::tools::floating_assistant::FloatingAssistantApplication::new(
                        Rc::clone(&prepared_platform),
                        prepared_config,
                        prepared_config_path,
                        prepared_notifications,
                        source,
                        prepared_downloads,
                        prepared_quick_launch,
                        cx,
                    )
                });
                crate::tools::floating_assistant::register(assistant.clone(), cx);
                cx.defer(move |cx| assistant.update(cx, |assistant, cx| assistant.start(cx)));
            };
            let main_preparation = if !main_show {
                let preparation =
                    platform_events::MainWindowPreparation::new(main_window_handle, publish_main);
                Some(if markdown_origin {
                    preparation.keep_application_on_failure()
                } else {
                    preparation
                })
            } else {
                cx.defer(publish_main);
                None
            };
            let dispatcher = cx.new(|cx| {
                PlatformEventDispatcher::new(
                    Rc::clone(&platform_bridge),
                    browser_startup_window,
                    main_window_handle,
                    matches!(launch_mode, DesktopLaunchMode::SilentStart),
                    PlatformApplicationTargets {
                        clipboard: clipboard.downgrade(),
                        quick_launch,
                        downloads,
                        global_settings: network_settings,
                    },
                    main_preparation,
                    cx,
                )
            });
            cx.set_global(PlatformEventDispatcherGlobal::new(dispatcher));
            crate::markdown_launch::start(markdown_launches, Rc::clone(&platform_bridge), cx);
        });
    let migration_correlation = migration_slot
        .0
        .lock()
        .ok()
        .and_then(|slot| slot.as_ref().map(|migration| migration.correlation_id));
    let shutdown_correlation_id = completion_exit_correlation
        .lock()
        .ok()
        .and_then(|mut correlation| correlation.take())
        .or(migration_correlation)
        .unwrap_or_else(Uuid::new_v4);
    let shutdown_started = Instant::now();
    info!(event="desktop.shutdown.started",correlation_id=%shutdown_correlation_id,"normal application resource drain started");
    let download_drain =
        runtime.shutdown_downloads_and_wait(&download_shutdown, shutdown_correlation_id);
    let extension_shutdown = runtime.extension_service.shutdown_and_wait();
    let clipboard_shutdown = worker_shutdown.shutdown();
    if download_drain.is_err() || extension_shutdown.is_err() || clipboard_shutdown.is_err() {
        error!(event="desktop.shutdown.failed",correlation_id=%shutdown_correlation_id,elapsed_ms=shutdown_started.elapsed().as_millis(),"normal feature resource drain failed");
    }
    let resource_drain = download_drain
        .and(extension_shutdown)
        .and(clipboard_shutdown);
    if resource_drain.is_ok() {
        info!(event="desktop.shutdown.feature_resources_drained",correlation_id=%shutdown_correlation_id,elapsed_ms=shutdown_started.elapsed().as_millis(),"download, extension and clipboard resources drained");
    }

    drop(download_shutdown);
    let migration = migration_slot
        .0
        .lock()
        .ok()
        .and_then(|mut slot| slot.take());
    if let Some(migration) = migration {
        let success = resource_drain.is_ok() && migration.execute().is_ok();
        if data_directory::save_result(&migration.source, success).is_err() {
            warn!(event = "data_directory.result_write.failed", correlation_id = %migration.correlation_id, "migration result notification could not be persisted");
        }
        let correlation_id = migration.correlation_id;
        drop(migration);
        data_directory::restart()?;
        info!(event = "data_directory.restart.started", %correlation_id, success, "desktop restart requested after migration");
    } else {
        resource_drain.context("failed to drain feature resources during shutdown")?;
    }
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
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    #[cfg(windows)]
    if arguments
        .first()
        .is_some_and(|arg| arg == install_archive_integration::ARGUMENT)
    {
        std::process::exit(install_archive_integration::run(&arguments));
    }
    let cwd = std::env::current_dir().ok();
    let markdown_intent = match markdown_launch::parse_arguments(
        arguments.iter().cloned(),
        cwd.as_deref().unwrap_or(Path::new("")),
    ) {
        Ok(intent) => intent,
        Err(failure) => {
            eprintln!("NxTools could not parse Markdown startup arguments ({failure}).");
            std::process::exit(2);
        }
    };
    #[cfg(not(debug_assertions))]
    if std::env::args_os().any(|argument| argument == "--ui-preview") {
        eprintln!("Isolated UI preview is available only in Debug builds.");
        std::process::exit(2);
    }
    #[cfg(debug_assertions)]
    match main_window::visual_preview::requested(
        &std::env::args_os()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>(),
    ) {
        Ok(true) => {
            main_window::visual_preview::run();
            return;
        }
        Ok(false) => {}
        Err(_) => {
            eprintln!("Invalid isolated UI preview arguments.");
            std::process::exit(2);
        }
    }
    let instance_wait = protected_restart_instance_wait();
    let launch_mode = match DesktopLaunchMode::from_arguments(std::env::args_os()) {
        Ok(launch_mode) => launch_mode,
        Err(error) => {
            eprintln!("NxTools could not parse its startup intent: {error:#}");
            std::process::exit(2);
        }
    };
    let archive_intent = if markdown_intent.is_some() {
        None
    } else {
        match cwd.and_then(|cwd| archive_launch::parse_arguments(arguments, &cwd).ok()) {
            Some(intent) => intent,
            None => {
                eprintln!("NxTools could not parse archive startup arguments (InvalidRequest).");
                std::process::exit(2);
            }
        }
    };
    match relaunch_protected_desktop_when_current(markdown_intent.as_ref()) {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => eprintln!("NxTools could not use the protected desktop copy: {error:#}"),
    }
    let ordinary = || {
        if launch_mode.should_summon_running_instance() {
            match runtime::summon_running_desktop() {
                Ok(forwarded) => Ok(forwarded),
                Err(error) => {
                    eprintln!("NxTools could not summon the running instance: {error:#}");
                    Ok(false)
                }
            }
        } else {
            match runtime::desktop_instance_is_running() {
                Ok(running) => Ok(running),
                Err(error) => {
                    eprintln!("NxTools could not check the existing desktop instance: {error:#}");
                    Ok(false)
                }
            }
        }
    };
    let forwarded = markdown_launch::route_before_startup(
        markdown_intent.as_ref(),
        runtime::forward_running_markdown,
        || {
            archive_launch::route_before_startup(
                archive_intent.as_ref(),
                runtime::forward_running_archive,
                ordinary,
            )
        },
    );
    match forwarded {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            if let Some(intent) = &markdown_intent {
                report_markdown_startup_failure(
                    intent.request_id,
                    &error,
                    MarkdownLaunchFailure::Disconnected,
                );
                std::process::exit(1);
            }
            let failure = error
                .downcast_ref::<nxtools_protocol::desktop_archive_intent::LaunchFailure>()
                .copied()
                .unwrap_or(nxtools_protocol::desktop_archive_intent::LaunchFailure::Disconnected);
            eprintln!("NxTools archive handoff failed ({failure}).");
            std::process::exit(1);
        }
    }
    let result = match launch_mode {
        DesktopLaunchMode::Background => runtime::run_background_download_host(instance_wait)
            .context("background download host startup failed"),
        DesktopLaunchMode::Normal
        | DesktopLaunchMode::SilentStart
        | DesktopLaunchMode::BrowserDialog { .. } => run(
            instance_wait,
            launch_mode,
            archive_intent.clone(),
            markdown_intent.clone(),
        )
        .context("desktop startup failed"),
    };
    if let Err(error) = result {
        if let Some(intent) = &markdown_intent {
            match markdown_launch::recover_instance_race(
                &error,
                intent,
                runtime::forward_running_markdown,
            ) {
                Ok(true) => return,
                Ok(false) => report_markdown_startup_failure(
                    intent.request_id,
                    &error,
                    MarkdownLaunchFailure::OwnerUnavailable,
                ),
                Err(error) => report_markdown_startup_failure(
                    intent.request_id,
                    &error,
                    MarkdownLaunchFailure::Disconnected,
                ),
            }
            std::process::exit(1);
        }
        match archive_launch::recover_instance_race(
            &error,
            archive_intent.as_ref(),
            runtime::forward_running_archive,
        ) {
            Ok(true) => return,
            Ok(false) => {}
            Err(error) => {
                let failure = error
                    .downcast_ref::<nxtools_protocol::desktop_archive_intent::LaunchFailure>()
                    .copied()
                    .unwrap_or(
                        nxtools_protocol::desktop_archive_intent::LaunchFailure::Disconnected,
                    );
                eprintln!("NxTools archive handoff after startup race failed ({failure}).");
                std::process::exit(1);
            }
        }
        let log_dir = AppPaths::discover().ok().map(|paths| paths.log_dir);
        let platform_host = std::env::current_exe().ok().and_then(|executable| {
            let host = executable.parent()?.join("NxTools.PlatformHost.exe");
            host.exists().then_some(host)
        });
        let interactive = launch_mode.startup_failure_is_interactive();
        startup_failure::report(
            log_dir.as_deref(),
            platform_host.as_deref(),
            &error,
            interactive,
        );
        std::process::exit(1);
    }
}

fn report_markdown_startup_failure(
    correlation_id: Uuid,
    error: &anyhow::Error,
    default: MarkdownLaunchFailure,
) {
    let failure = error
        .downcast_ref::<MarkdownLaunchFailure>()
        .copied()
        .unwrap_or(default);
    let log_dir = AppPaths::discover().ok().map(|paths| paths.log_dir);
    let report = anyhow::anyhow!("Markdown startup failed ({failure}, operation {correlation_id})");
    startup_failure::report(log_dir.as_deref(), None, &report, false);
}

fn protected_restart_instance_wait() -> std::time::Duration {
    let mut arguments = std::env::args_os();
    while let Some(argument) = arguments.next() {
        if argument == "--wait-for-instance-release-ms"
            && let Some(milliseconds) = arguments
                .next()
                .and_then(|value| value.to_str().and_then(|value| value.parse::<u64>().ok()))
        {
            return std::time::Duration::from_millis(milliseconds.min(15_000));
        }
    }
    std::time::Duration::ZERO
}

fn relaunch_protected_desktop_when_current(
    markdown_intent: Option<&DesktopMarkdownIntent>,
) -> Result<bool> {
    let current = std::env::current_exe().context("failed to locate desktop executable")?;
    let Some(protected) = protected_desktop_path() else {
        return Ok(false);
    };
    if paths_equal(&current, &protected)
        || !protected_package_matches_release(&current, &protected)?
    {
        return Ok(false);
    }
    let mut command = Command::new(&protected);
    if let Some(intent) = markdown_intent {
        command
            .arg(markdown_launch::OPEN_ARGUMENT)
            .arg(&intent.path)
            .arg(markdown_launch::CORRELATION_ARGUMENT)
            .arg(intent.request_id.to_string());
    } else {
        command.args(std::env::args_os().skip(1));
    }
    command
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
    for name in [
        "NxTools.Indexer.exe",
        "NxTools.PlatformHost.exe",
        "NxTools.ArchiveWorker.exe",
        "NxTools.ShellSelectionHelper.exe",
        "NxTools.ShellSelectionHelper.dll",
        "NxTools.ShellSelectionHelper.deps.json",
        "NxTools.ShellSelectionHelper.runtimeconfig.json",
    ] {
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
    use gpui_kit::TestAppContext;

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
    fn silent_start_creates_the_product_runtime_without_showing_or_focusing_main() {
        let silent = DesktopLaunchMode::from_arguments([
            "NxTools.exe".to_string(),
            "--silent-start".to_string(),
        ])
        .expect("silent startup intent");

        assert_eq!(silent, DesktopLaunchMode::SilentStart);
        assert!(!silent.main_window_focus());
        assert!(!silent.main_window_show());
    }

    #[test]
    fn silent_start_never_summons_or_surfaces_an_interactive_failure() {
        let silent = DesktopLaunchMode::SilentStart;
        let normal = DesktopLaunchMode::Normal;

        assert!(!silent.should_summon_running_instance());
        assert!(normal.should_summon_running_instance());
        assert!(!silent.startup_failure_is_interactive());
        assert!(normal.startup_failure_is_interactive());
    }

    #[test]
    fn main_window_size_remains_logical_at_common_boot_dpi_scales() {
        let expected = size(px(1060.0), px(650.0));
        for dpi_scale in [1.0_f32, 1.25, 1.5] {
            assert_eq!(
                main_window_logical_size(),
                expected,
                "initial size must not be pre-scaled at {dpi_scale:.2}x DPI"
            );
        }
    }

    #[gpui_kit::test]
    fn initial_window_placement_binds_bounds_to_the_native_display(cx: &mut TestAppContext) {
        let display_id = cx.update(|app| {
            app.primary_display()
                .expect("test platform primary display")
                .id()
        });
        for dpi_scale in [1.0_f32, 1.25, 1.5] {
            let logical_display_bounds = gpui_kit::Bounds::new(
                gpui_kit::point(px(0.0), px(0.0)),
                size(px(2560.0 / dpi_scale), px(1440.0 / dpi_scale)),
            );
            let placement =
                main_window_placement_for_display(Some((display_id, logical_display_bounds)));
            let bounds = placement.bounds.get_bounds();

            assert_eq!(placement.display_id, Some(display_id));
            assert_eq!(bounds.size, main_window_logical_size());
            assert_eq!(bounds.center().x * dpi_scale, px(1280.0));
            assert_eq!(bounds.center().y * dpi_scale, px(720.0));
        }

        let fallback = main_window_placement_for_display(None);
        assert_eq!(fallback.display_id, None);
        assert_eq!(
            fallback.bounds.get_bounds().size,
            main_window_logical_size()
        );
        assert_eq!(
            fallback.bounds.get_bounds().origin,
            gpui_kit::point(px(0.0), px(0.0))
        );
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
            (
                "NxTools.ArchiveWorker.exe",
                b"current archive member".as_slice(),
            ),
            (
                "NxTools.ShellSelectionHelper.exe",
                b"current archive member".as_slice(),
            ),
            (
                "NxTools.ShellSelectionHelper.dll",
                b"current archive member".as_slice(),
            ),
            (
                "NxTools.ShellSelectionHelper.deps.json",
                b"current archive member".as_slice(),
            ),
            (
                "NxTools.ShellSelectionHelper.runtimeconfig.json",
                b"current archive member".as_slice(),
            ),
        ] {
            std::fs::write(release_dir.join(name), bytes).expect("release binary");
            std::fs::write(protected_dir.join(name), bytes).expect("protected binary");
        }
        let release = release_dir.join("NxTools.exe");
        let protected = protected_dir.join("NxTools.exe");
        for name in [
            "NxTools.ArchiveWorker.exe",
            "NxTools.ShellSelectionHelper.exe",
            "NxTools.ShellSelectionHelper.dll",
            "NxTools.ShellSelectionHelper.deps.json",
            "NxTools.ShellSelectionHelper.runtimeconfig.json",
        ] {
            std::fs::remove_file(protected_dir.join(name)).expect("remove protected member");
            assert!(
                !protected_package_matches_release(&release, &protected)
                    .expect("missing archive release member rejects redirect")
            );
            std::fs::write(release_dir.join(name), b"current archive member")
                .expect("release archive member");
            std::fs::write(protected_dir.join(name), b"old archive member")
                .expect("old protected archive member");
            assert!(
                !protected_package_matches_release(&release, &protected)
                    .expect("mixed archive release member rejects redirect")
            );
            std::fs::write(protected_dir.join(name), b"current archive member")
                .expect("matching archive member");
        }
        assert!(protected_package_matches_release(&release, &protected).expect("complete package"));
        std::fs::write(protected_dir.join("NxTools.Indexer.exe"), b"older indexer")
            .expect("replace protected indexer");
        assert!(
            !protected_package_matches_release(&release, &protected)
                .expect("compare different packages")
        );
    }
}
