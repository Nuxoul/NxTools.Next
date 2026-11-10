//! Debug native rendering fixture; never connects platform/service or probes HTTP.
use super::*;

pub(crate) fn open_confirmation(cx: &mut App) -> anyhow::Result<()> {
    let engine = DownloadEngine::open_in_memory()?;
    let (platform, endpoint) = nxtools_ipc::platform_bridge(16);
    drop(endpoint);
    let notifications = cx.new(|_| NotificationCenter::default());
    let scratch = std::env::var_os("PI_SCRATCH_DIR")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("isolated preview requires session scratch"))?;
    let application = cx.new(|cx| {
        DownloadApplication::new(
            engine,
            Rc::new(platform),
            notifications,
            DownloadRuntimeContext {
                fallback_default_directory: scratch.clone(),
                config: AppConfig::default(),
                config_path: scratch.join("preview-unused-config.json"),
                log_directory: scratch.clone(),
                database_path: scratch.join("preview-unused-downloads.sqlite3"),
                ffmpeg_install_directory: scratch.join("preview-unused-ffmpeg"),
            },
            cx,
        )
    });
    if let Some(surface) = std::env::var_os("NXTOOLS_PREVIEW_AUXILIARY") {
        let surface = surface.to_string_lossy().into_owned();
        let window_size = match surface.as_str() {
            "schedule" => size(px(430.0), px(326.0)),
            "shutdown" => size(px(450.0), px(200.0)),
            _ => anyhow::bail!("unknown isolated auxiliary surface"),
        };
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::centered(window_size, cx)),
                titlebar: None,
                window_decorations: Some(WindowDecorations::Client),
                window_min_size: Some(window_size),
                is_resizable: false,
                ..Default::default()
            },
            move |window, cx| {
                window.set_window_title("NxTools Auxiliary Preview — synthetic");
                println!(
                    "ui.preview.auxiliary_ready hwnd={} scale={}",
                    native_window_handle(window).unwrap_or_default(),
                    window.scale_factor()
                );
                if surface == "schedule" {
                    let view = cx.new(|cx| {
                        let mut view = DownloadScheduleView::new(application, None, window, cx);
                        view.enabled_start = true;
                        view.enabled_stop = true;
                        view
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                } else {
                    let view = cx.new(|_| {
                        DownloadShutdownCountdownView::new(application, uuid::Uuid::new_v4(), 30)
                    });
                    cx.new(|cx| Root::new(view, window, cx))
                }
            },
        )?;
        return Ok(());
    }
    let transition = std::env::var_os("NXTOOLS_PREVIEW_FIXED_TRANSITION").is_some();
    let presentation = std::env::var_os("NXTOOLS_PREVIEW_FIXED_PRESENTATION").is_some();
    open_fixture(
        application.clone(),
        !transition && !presentation,
        false,
        "download_ready",
        cx,
    )?;
    if presentation {
        open_fixture(application, true, true, "fixed_target_ready", cx)?;
    }
    Ok(())
}

fn open_fixture(
    application: Entity<DownloadApplication>,
    confirmed: bool,
    hidden: bool,
    label: &'static str,
    cx: &mut App,
) -> anyhow::Result<()> {
    let bounds = WindowBounds::centered(new_download_window_size(confirmed), cx);
    let mut options = new_download_window_options(bounds);
    options.show = !hidden;
    options.focus = !hidden;
    cx.open_window(options, move |window, cx| {
        window.set_window_title("NxTools Download Preview — synthetic");
        let view = cx.new(|cx| {
            let mut view = NewDownloadView::new(
                NewDownloadViewInit {
                    application,
                    default_directory: PathBuf::from(r"D:\Preview"),
                    default_connection_limit: 8,
                    default_adaptive_segment_policy: Default::default(),
                    categories: Vec::new(),
                    initial_source: None,
                },
                window,
                cx,
            );
            view.url.update(cx, |input, cx| {
                input.set_value("https://example.test/preview.bin", window, cx)
            });
            view.file_name
                .update(cx, |input, cx| input.set_value("preview.bin", window, cx));
            if confirmed {
                view.probe = Some(DownloadProbe {
                    resolution: None,
                    server_observation: None,
                    final_url: "https://example.test/preview.bin".into(),
                    file_name: Some("preview.bin".into()),
                    total_bytes: Some(64 * 1024 * 1024),
                    supports_ranges: true,
                    content_type: Some("application/octet-stream".into()),
                    validator: Default::default(),
                    media: None,
                });
                view.confirmed_target_height = Some(px(NEW_DOWNLOAD_CONFIRMED_HEIGHT));
                view.confirmed_request = view.request(cx).ok();
            }
            view
        });
        let last_height = std::cell::Cell::new(0_i32);
        view.update(cx, |_, cx| {
            cx.observe_window_bounds(window, move |_, window, _| {
                let height = window.viewport_size().height.as_f32().round() as i32;
                if last_height.replace(height) != height {
                    println!("ui.preview.{label} viewport_height={height}");
                }
            })
            .detach();
        });
        println!(
            "ui.preview.{label} hwnd={} scale={}",
            native_window_handle(window).unwrap_or_default(),
            window.scale_factor()
        );
        cx.new(|cx| Root::new(view, window, cx))
    })?;
    Ok(())
}

pub(crate) fn gallery_owner(
    scratch: &Path,
    notifications: Entity<NotificationCenter>,
    populated: bool,
    cx: &mut App,
) -> anyhow::Result<Entity<DownloadApplication>> {
    let engine = DownloadEngine::open_in_memory()?;
    if populated {
        for index in 0..4 {
            engine.enqueue(
                DownloadRequest {
                    source_url: format!("https://synthetic.invalid/{index}"),
                    save_directory: scratch.into(),
                    save_directory_explicit: false,
                    file_name: Some(format!("合成任务{index} · 长中文文件名称用于检查裁剪.bin")),
                    headers: Vec::new(),
                    cookie: None,
                    referrer: None,
                    certificate_policy: Default::default(),
                    connection_limit: Some(4),
                    speed_limit_bytes_per_second: None,
                    expected_total_bytes: Some(64 * 1024 * 1024),
                    scheduled_at_unix_ms: None,
                    adaptive_segment_policy: None,
                    media: None,
                    category_id: None,
                    category_name: None,
                    duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
                },
                RequestOrigin::Internal,
            )?;
        }
    }
    let (platform, endpoint) = nxtools_ipc::platform_bridge(8);
    drop(endpoint);
    let mut config = AppConfig::default();
    config.download.ffmpeg_executable_path = scratch
        .join("missing-fixture-ffmpeg.exe")
        .to_string_lossy()
        .into_owned();
    Ok(cx.new(|cx| {
        DownloadApplication::new(
            engine,
            Rc::new(platform),
            notifications,
            DownloadRuntimeContext {
                fallback_default_directory: scratch.into(),
                config,
                config_path: scratch.join("unused-download-config.json"),
                log_directory: scratch.into(),
                database_path: scratch.join("unused-download.db"),
                ffmpeg_install_directory: scratch.join("unused-ffmpeg"),
            },
            cx,
        )
    }))
}
pub(crate) fn open_gallery(
    application: Entity<DownloadApplication>,
    cx: &mut App,
) -> anyhow::Result<()> {
    use crate::main_window::visual_preview::final_gallery::{options, ready};
    for (mode, page) in [
        ("download-settings-general", DownloadSettingsPage::General),
        ("download-settings-request", DownloadSettingsPage::Request),
        ("download-settings-transfer", DownloadSettingsPage::Transfer),
        ("download-settings-window", DownloadSettingsPage::Window),
        (
            "download-settings-diagnostics",
            DownloadSettingsPage::Diagnostics,
        ),
        ("download-settings-error", DownloadSettingsPage::Request),
        ("download-settings-busy", DownloadSettingsPage::General),
    ] {
        let initial = application.read(cx).settings_view_initial();
        let owner = application.clone();
        cx.open_window(options(984.0, 650.0), move |window, cx| {
            let view = cx.new(|cx| {
                let mut view = DownloadSettingsView::new(owner, initial, window, cx);
                view.page = page;
                if mode.ends_with("error") {
                    view.error = Some("合成长错误，保留完整失败内容和重试。".repeat(20));
                }
                if mode.ends_with("busy") {
                    view.saving = true;
                }
                view.default_request_headers.update(cx, |input, cx| {
                    input.set_value("X-Synthetic: value\n".repeat(30), window, cx)
                });
                view
            });
            ready(mode, 984.0, 650.0, window);
            cx.new(|cx| Root::new(view, window, cx))
        })?;
    }
    let owner = application.clone();
    cx.open_window(options(430.0, 326.0), move |window, cx| {
        let view = cx.new(|cx| {
            let mut view = DownloadScheduleView::new(owner, None, window, cx);
            view.enabled_start = true;
            view.enabled_stop = true;
            view
        });
        ready("download-schedule", 430.0, 326.0, window);
        cx.new(|cx| Root::new(view, window, cx))
    })?;
    let task = application.read(cx).tasks().first().cloned();
    if let Some(task) = task {
        let id = task.task.id();
        let owner = application.clone();
        cx.open_window(options(380.0, 178.0), move |window, cx| {
            let view = cx.new(|_| DeleteDownloadView {
                theme: Theme::zed_dark(),
                application: owner,
                task_id: id,
                path: "仅合成路径 · 不会删除任何文件".repeat(5),
                batch: None,
            });
            ready("download-delete", 380.0, 178.0, window);
            cx.new(|cx| Root::new(view, window, cx))
        })?;
        for (mode, snapshot) in [
            ("download-task-queued", Some(task)),
            ("download-task-removed", None),
        ] {
            let owner = application.clone();
            let platform = application.read(cx).platform.clone();
            cx.open_window(options(440.0, 184.0), move |window, cx| {
                let view = cx.new(|_| {
                    DownloadTaskWindowView::new(
                        owner,
                        platform,
                        id,
                        TaskWindowIdentity {
                            native_window_handle: 0,
                            generation: uuid::Uuid::new_v4(),
                        },
                        snapshot,
                        None,
                        false,
                    )
                });
                ready(mode, 440.0, 184.0, window);
                cx.new(|cx| Root::new(view, window, cx))
            })?;
        }
    }
    Ok(())
}
