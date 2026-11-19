use nxtools_core::{AppConfig, DownloadSettings, default_download_request_headers};

const EXPECTED_HEADERS: &str =
    "User-Agent: Mozilla/5.0 (Windows NT 6.1; Trident/7.0; rv:11.0) like Gecko";

#[test]
fn b4_download_defaults_and_legacy_load_are_effective_and_read_only() {
    assert_eq!(default_download_request_headers(), EXPECTED_HEADERS);
    assert_eq!(
        DownloadSettings::default().default_request_headers,
        EXPECTED_HEADERS
    );
    for headers in [None, Some(""), Some(" \r\n\t "), Some("X-Custom: retained")] {
        let directory = tempfile::tempdir().expect("owned configuration directory");
        let path = directory.path().join("config.json");
        let mut raw = serde_json::to_value(AppConfig::default()).expect("configuration");
        if let Some(headers) = headers {
            raw["download"]["default_request_headers"] = serde_json::json!(headers);
        } else {
            raw["download"]
                .as_object_mut()
                .expect("download section")
                .remove("default_request_headers");
        }
        let before = serde_json::to_vec_pretty(&raw).expect("configuration bytes");
        std::fs::write(&path, &before).expect("owned file");
        let loaded = AppConfig::load_from(&path).expect("original load");
        assert_eq!(
            loaded.download.default_request_headers,
            if headers == Some("X-Custom: retained") {
                "X-Custom: retained"
            } else {
                EXPECTED_HEADERS
            }
        );
        assert_eq!(std::fs::read(&path).expect("load is read only"), before);
    }
}

#[test]
fn b4_download_merge_uses_open_baseline_and_original_locked_writer() {
    let directory = tempfile::tempdir().expect("owned configuration directory");
    let path = directory.path().join("config.json");
    let mut initial = AppConfig::default();
    initial.download.default_request_headers = "User-Agent: initial-custom".into();
    initial.download.background_speed_limit.enabled = true;
    initial.download.background_speed_limit.bytes_per_second = 1537;
    initial.save_to(&path).expect("initial file");
    let baseline = AppConfig::load_from(&path).expect("open baseline").download;
    let concurrent = AppConfig::update_section(&path, |config| {
        config.download.default_cookie = "concurrent=1".into();
        config.download.task_list_sort = Some("name".into());
        config.download.taskbar_progress_overlay.offset_y = 48;
        config.download.background_speed_limit.enabled = false;
        config.download.show_completion_window = false;
    })
    .expect("original concurrent writer");
    let mut desired = baseline.clone();
    desired.default_request_headers.clear();
    desired.taskbar_progress_overlay.offset_x = 24;
    let merged = AppConfig::merge_download_settings(&path, &baseline, desired)
        .expect("original locked merge");
    let mut expected = concurrent;
    expected.download.default_request_headers = EXPECTED_HEADERS.into();
    expected.download.taskbar_progress_overlay.offset_x = 24;
    assert_eq!(merged.download, expected.download);
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("saved configuration")
            .download,
        expected.download
    );
    assert_eq!(
        merged.download.background_speed_limit.bytes_per_second,
        1537
    );
    std::fs::write(&path, b"{invalid-owned-config").expect("owned failure fixture");
    assert!(
        AppConfig::merge_download_settings(&path, &baseline, DownloadSettings::default()).is_err()
    );
    assert_eq!(
        std::fs::read(&path).expect("failed merge does not publish"),
        b"{invalid-owned-config"
    );
}

#[test]
fn increasing_connections_preserves_latest_legacy_adaptive_settings() {
    let directory = tempfile::tempdir().expect("owned configuration directory");
    let path = directory.path().join("config.json");
    let mut config = AppConfig::default();
    config.download.default_connection_limit = 8;
    config.download.adaptive_segment_max_segments = 16;
    config.save_to(&path).expect("initial file");
    let baseline = AppConfig::load_from(&path).expect("open baseline").download;
    AppConfig::update_section(&path, |config| {
        config.download.adaptive_segment_scaling_enabled = false;
        config.download.adaptive_segment_max_segments = 64;
        config.download.adaptive_segment_min_mib = 9;
    })
    .expect("latest file");
    let mut desired = baseline.clone();
    desired.default_connection_limit = 32;
    AppConfig::merge_download_settings(&path, &baseline, desired).expect("apply connections");
    let saved = AppConfig::load_from(&path).expect("reopen").download;
    assert_eq!(saved.default_connection_limit, 32);
    assert_eq!(saved.adaptive_segment_max_segments, 64);
    assert_eq!(saved.adaptive_segment_min_mib, 9);
    assert!(!saved.adaptive_segment_scaling_enabled);
}
