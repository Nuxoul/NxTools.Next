//! Debug owned-HWND fixture for the real task window phase geometry.
use super::*;

pub(crate) fn open(cx: &mut App) -> anyhow::Result<()> {
    let scratch = PathBuf::from(
        std::env::var_os("PI_SCRATCH_DIR").ok_or_else(|| anyhow::anyhow!("scratch required"))?,
    );
    let engine = DownloadEngine::open_in_memory()?;
    let request = DownloadRequest {
        source_url: "https://example.test/task-fixture.bin".into(),
        save_directory: scratch.clone(),
        save_directory_explicit: false,
        file_name: Some("task-fixture.bin".into()),
        headers: Vec::new(),
        cookie: None,
        referrer: None,
        certificate_policy: Default::default(),
        connection_limit: Some(1),
        speed_limit_bytes_per_second: None,
        expected_total_bytes: Some(4096),
        scheduled_at_unix_ms: None,
        adaptive_segment_policy: None,
        media: None,
        category_id: None,
        category_name: None,
        duplicate_strategy: nxtools_download::DownloadDuplicateStrategy::UseDefault,
    };
    let task_id = engine.enqueue(request, RequestOrigin::Internal)?;
    let snapshot = Some(
        load_download_task_detail(&engine, task_id)
            .map_err(anyhow::Error::msg)?
            .task,
    );
    let (platform, endpoint) = nxtools_ipc::platform_bridge(16);
    drop(endpoint);
    let platform = Rc::new(platform);
    let notifications = cx.new(|_| NotificationCenter::default());
    let application = cx.new(|cx| {
        DownloadApplication::new(
            engine,
            platform.clone(),
            notifications,
            DownloadRuntimeContext {
                fallback_default_directory: scratch.clone(),
                config: AppConfig::default(),
                config_path: scratch.join("completion-preview-unused.json"),
                log_directory: scratch.clone(),
                database_path: scratch.join("completion-preview-unused.sqlite3"),
                ffmpeg_install_directory: scratch.join("completion-preview-unused-tools"),
            },
            cx,
        )
    });
    let window_size = download_task_window_size(DownloadWindowPhase::Progress);
    cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::centered(window_size, cx)),
            titlebar: None,
            focus: true,
            show: true,
            kind: WindowKind::Normal,
            is_movable: true,
            is_resizable: false,
            is_minimizable: true,
            display_id: None,
            window_background: WindowBackgroundAppearance::Opaque,
            app_id: Some("NxTools.TaskTransition.Preview".into()),
            window_min_size: Some(task_window_minimum_size()),
            window_decorations: Some(WindowDecorations::Client),
            tabbing_identifier: None,
        },
        move |window, cx| {
            let native = native_window_handle(window).unwrap_or_default();
            let view = cx.new(|_| {
                DownloadTaskWindowView::new(
                    application,
                    platform,
                    task_id,
                    TaskWindowIdentity {
                        native_window_handle: native,
                        generation: uuid::Uuid::new_v4(),
                    },
                    snapshot,
                    None,
                    false,
                )
            });
            println!(
                "ui.preview.completion_source hwnd={native} scale={}",
                window.scale_factor()
            );
            cx.new(|cx| Root::new(view, window, cx))
        },
    )?;
    Ok(())
}
