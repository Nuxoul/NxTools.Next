use super::*;
use gpui_kit::TestAppContext;
use uuid::Uuid;

#[gpui_kit::test]
fn installer_download_reuses_original_task_after_defaults_change(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let root = tempfile::tempdir().expect("owned installer fixture");
    let database = root.path().join("downloads.sqlite3");
    {
        let store = nxtools_download::SqliteDownloadStore::open(&database).expect("fixture store");
        store
            .set_queue_running("default", false)
            .expect("pause before opening engine");
    }
    let engine = DownloadEngine::open(&database).expect("fixture engine");
    let (platform, _endpoint) = nxtools_ipc::platform_bridge(16);
    let notifications = cx.update(|cx| cx.new(|_| NotificationCenter::default()));
    let mut config = AppConfig::default();
    config.download.automatic_categories_enabled = false;
    config.download.show_progress_window = false;
    config.download.show_completion_window = false;
    let application = cx.update(|cx| {
        cx.new(|cx| {
            DownloadApplication::new(
                engine,
                Rc::new(platform),
                notifications,
                DownloadRuntimeContext {
                    fallback_default_directory: root.path().to_path_buf(),
                    config,
                    config_path: root.path().join("config.json"),
                    log_directory: root.path().to_path_buf(),
                    database_path: database,
                    ffmpeg_install_directory: root.path().join("ffmpeg"),
                },
                cx,
            )
        })
    });
    let key = Uuid::new_v4();
    cx.update(|cx| {
        application.update(cx, |application, cx| {
            let first = application
                .submit_installer_download(
                    "https://example.invalid/setup.exe".into(),
                    "NxTools.Next-setup-0.7.10.exe".into(),
                    Some(24),
                    key,
                    cx,
                )
                .expect("initial durable admission");
            application.default_directory = root.path().join("new-default");
            application.config.download.default_connection_limit = 8;
            application.config.download.adaptive_segment_max_segments = 32;
            let repeated = application
                .submit_installer_download(
                    "https://example.invalid/setup.exe".into(),
                    "NxTools.Next-setup-0.7.10.exe".into(),
                    Some(24),
                    key,
                    cx,
                )
                .expect("existing installer remains accepted after preferences change");
            assert_eq!(first, repeated);
            assert_eq!(application.tasks().len(), 1);
            let original = application
                .engine
                .task(first)
                .expect("durable lookup")
                .expect("original task");
            assert_eq!(original.task.request().save_directory, root.path());
            assert_eq!(original.task.request().connection_limit, Some(16));
            assert!(application.tasks()[0].queue_eligible);
        })
    });
    drop(application);
    cx.update(|_| {});
    cx.run_until_parked();
    root.close()
        .expect("release fixture database and owned files");
}
