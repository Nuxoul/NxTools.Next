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
                    final_url: "https://example.test/preview.bin".into(),
                    file_name: Some("preview.bin".into()),
                    total_bytes: Some(64 * 1024 * 1024),
                    supports_ranges: true,
                    content_type: Some("application/octet-stream".into()),
                    validator: Default::default(),
                    media: None,
                });
                view.confirmed_target_height = Some(px(278.0));
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
