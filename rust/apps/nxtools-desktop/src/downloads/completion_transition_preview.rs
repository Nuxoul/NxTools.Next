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
        source_page: None,
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
    let mut snapshot = Some(
        load_download_task_detail(&engine, task_id)
            .map_err(anyhow::Error::msg)?
            .task,
    );
    let layout_page = std::env::var("NXTOOLS_PREVIEW_PROGRESS_PAGE")
        .ok()
        .map(|page| page.parse::<usize>().unwrap_or(3));
    let progress = layout_page.map(|_| {
        let snapshot = snapshot.as_mut().expect("synthetic task");
        let at = snapshot.task.updated_at_unix_ms();
        for status in [
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
        ] {
            snapshot
                .task
                .transition_to(status, at)
                .expect("synthetic progress");
        }
        let total = 208 * 1024 * 1024;
        let per_segment = total / 16;
        let written = per_segment / 3;
        snapshot.total_bytes = Some(total);
        snapshot.downloaded_bytes = written * 16;
        snapshot.final_path = Some(scratch.join("download-1.1.0-portable.zip"));
        DownloadProgress {
            task_id,
            downloaded_bytes: written * 16,
            total_bytes: Some(total),
            bytes_per_second: 5 * 1024 * 1024,
            active_connections: 8,
            segments: (0..16)
                .map(|index| nxtools_download::DownloadSegmentProgress {
                    segment_index: index,
                    start_byte: Some(u64::from(index) * per_segment),
                    total_bytes: Some(per_segment),
                    downloaded_bytes: written,
                })
                .collect(),
            finishing: None,
        }
    });
    if std::env::var("NXTOOLS_PREVIEW_PROGRESS_THEME").is_ok_and(|mode| mode == "light") {
        gpui_kit::component::Theme::change(gpui_kit::component::ThemeMode::Light, None, cx);
    }
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
    let window_size = download_task_window_size_for_details(
        DownloadWindowPhase::Progress,
        layout_page.is_some_and(|page| page < 3),
    );
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
            ..Default::default()
        },
        move |window, cx| {
            let native = native_window_handle(window).unwrap_or_default();
            let view = cx.new(|_| {
                let mut view = DownloadTaskWindowView::new(
                    application,
                    platform,
                    task_id,
                    TaskWindowIdentity {
                        native_window_handle: native,
                        generation: uuid::Uuid::new_v4(),
                    },
                    snapshot,
                    progress.clone(),
                    false,
                );
                if let Some(page) = layout_page {
                    view.details_tab = page.min(2);
                    view.details_expanded = page < 3;
                    let total = 208 * 1024 * 1024;
                    let bytes = total / 16 / 3 * 16;
                    view.display_sampler =
                        super::progress_display::ProgressDisplaySampler::default();
                    view.display_sampler.observe(
                        Duration::ZERO,
                        bytes - 5 * 1024 * 1024,
                        Some(total),
                        true,
                    );
                    view.display_sampler
                        .observe(Duration::from_secs(1), bytes, Some(total), true);
                }
                view
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
