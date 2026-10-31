use super::*;

fn fixture(label: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    let directory = std::env::temp_dir().join(format!(
        "nxtools-floating-assistant-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("fixture clock")
            .as_nanos()
    ));
    fs::create_dir_all(&directory).expect("create fixture directory");
    let path = directory.join("config.json");
    (directory, path)
}

#[test]
fn floating_assistant_legacy_and_partial_sections_default_safely() {
    let legacy: AppConfig =
        serde_json::from_str(r#"{"theme":"dark","platform_pipe_name":"legacy-pipe"}"#)
            .expect("legacy config");
    let defaults = FloatingAssistantSettings::default();
    assert_eq!(legacy.floating_assistant, defaults);
    assert!(!defaults.enabled);
    assert_eq!(defaults.image_path, None);
    assert_eq!((defaults.size, defaults.frame_rate), (72, 24));
    assert!(!defaults.synchronize_download_speed);
    assert_eq!((defaults.base_opacity, defaults.hover_opacity), (48, 96));
    assert_eq!(defaults.edge_snap_distance, 24);
    assert!(defaults.auto_hide_at_edge);
    assert!(defaults.hide_in_fullscreen);
    assert!(!defaults.position_locked);
    assert!(!defaults.click_through);
    assert_eq!((defaults.position.x, defaults.position.y), (40, 180));
    assert_eq!(defaults.position.monitor_id, None);
    assert_eq!(defaults.position.dpi, None);
    assert_eq!(defaults.left_click_action, ClickAction::OpenMain);
    assert_eq!(defaults.right_click_action, ClickAction::AddDownload);
    assert_eq!(defaults.text_drop_action, TextDropAction::Automatic);
    assert_eq!(defaults.image_drop_action, ImageDropAction::Automatic);

    let partial: AppConfig = serde_json::from_str(
        r#"{"theme":"dark","platform_pipe_name":"legacy-pipe","floating_assistant":{"enabled":true,"position":{"x":-40}}}"#,
    )
    .expect("partial config");
    assert!(partial.floating_assistant.enabled);
    assert_eq!(partial.floating_assistant.position.x, -40);
    assert_eq!(partial.floating_assistant.position.y, 180);
    assert_eq!(partial.floating_assistant.frame_rate, 24);
}

#[test]
fn floating_assistant_all_action_enums_roundtrip_as_snake_case() {
    for (action, wire) in [
        (ClickAction::None, "none"),
        (ClickAction::OpenMain, "open_main"),
        (ClickAction::AddDownload, "add_download"),
    ] {
        assert_eq!(
            serde_json::to_string(&action).expect("encode click"),
            format!("\"{wire}\"")
        );
        assert_eq!(
            serde_json::from_str::<ClickAction>(&format!("\"{wire}\"")).expect("decode click"),
            action
        );
    }
    for (action, wire) in [
        (TextDropAction::None, "none"),
        (TextDropAction::Automatic, "automatic"),
        (TextDropAction::AddDownload, "add_download"),
        (TextDropAction::QuickLaunchSearch, "quick_launch_search"),
    ] {
        assert_eq!(
            serde_json::to_string(&action).expect("encode text"),
            format!("\"{wire}\"")
        );
        assert_eq!(
            serde_json::from_str::<TextDropAction>(&format!("\"{wire}\"")).expect("decode text"),
            action
        );
    }
    for (action, wire) in [
        (ImageDropAction::None, "none"),
        (ImageDropAction::Automatic, "automatic"),
        (ImageDropAction::AddDownload, "add_download"),
        (ImageDropAction::WriteClipboard, "write_clipboard"),
    ] {
        assert_eq!(
            serde_json::to_string(&action).expect("encode image"),
            format!("\"{wire}\"")
        );
        assert_eq!(
            serde_json::from_str::<ImageDropAction>(&format!("\"{wire}\"")).expect("decode image"),
            action
        );
    }
    let mut config = AppConfig::default();
    config.floating_assistant.left_click_action = ClickAction::None;
    config.floating_assistant.right_click_action = ClickAction::OpenMain;
    config.floating_assistant.text_drop_action = TextDropAction::QuickLaunchSearch;
    config.floating_assistant.image_drop_action = ImageDropAction::WriteClipboard;
    let encoded = serde_json::to_vec(&config).expect("encode config");
    assert_eq!(
        serde_json::from_slice::<AppConfig>(&encoded).expect("decode config"),
        config
    );
}

#[test]
fn floating_assistant_normalizes_ranges_path_and_monitor_without_losing_position() {
    let mut settings = FloatingAssistantSettings {
        size: 0,
        frame_rate: u32::MAX,
        base_opacity: 101,
        hover_opacity: 1,
        edge_snap_distance: 0,
        image_path: Some("  C:\\fixture\\ball.gif  ".into()),
        position: FloatingAssistantPosition {
            x: -1234,
            y: 5432,
            monitor_id: Some(" display-one ".into()),
            dpi: Some(0),
        },
        ..Default::default()
    };
    settings.normalize();
    assert_eq!((settings.size, settings.frame_rate), (20, 60));
    assert_eq!((settings.base_opacity, settings.hover_opacity), (100, 100));
    assert_eq!(settings.edge_snap_distance, 4);
    assert_eq!(
        settings.image_path.as_deref(),
        Some("C:\\fixture\\ball.gif")
    );
    assert_eq!((settings.position.x, settings.position.y), (-1234, 5432));
    assert_eq!(settings.position.monitor_id.as_deref(), Some("display-one"));
    assert_eq!(settings.position.dpi, None);
    let once = settings.clone();
    settings.normalize();
    assert_eq!(settings, once);

    settings.size = 999;
    settings.frame_rate = 0;
    settings.base_opacity = 0;
    settings.hover_opacity = 20;
    settings.edge_snap_distance = 999;
    settings.image_path = Some("a".repeat(4097));
    settings.position.monitor_id = Some("bad\0monitor".into());
    settings.position.dpi = Some(961);
    settings.normalize();
    assert_eq!((settings.size, settings.frame_rate), (300, 1));
    assert_eq!((settings.base_opacity, settings.hover_opacity), (10, 20));
    assert_eq!(settings.edge_snap_distance, 120);
    assert_eq!(settings.image_path, None);
    assert_eq!(settings.position.monitor_id, None);
    assert_eq!(settings.position.dpi, None);
    settings.image_path = Some("bad\0image".into());
    settings.normalize();
    assert_eq!(settings.image_path, None);
    settings.image_path = Some("   ".into());
    settings.normalize();
    assert_eq!(settings.image_path, None);
}

#[test]
fn floating_assistant_locked_section_update_merges_disk_latest_and_normalizes() {
    let (directory, path) = fixture("merge");
    AppConfig::default().save_to(&path).expect("seed config");
    let stale = AppConfig::load_from(&path).expect("cache at startup");
    AppConfig::update_section(&path, |config| {
        config.download.default_connection_limit = 24;
        config.clipboard.storage_directory = "synthetic-latest-root".into();
        config.system.theme = ThemePreference::Light;
    })
    .expect("other owners update");
    let mut settings = stale.floating_assistant;
    settings.enabled = true;
    settings.frame_rate = 100;
    settings.image_path = Some("  C:\\fixture\\chosen.png  ".into());
    settings.position = FloatingAssistantPosition {
        x: 250,
        y: -35,
        monitor_id: Some("display-one".into()),
        dpi: Some(144),
    };
    let merged =
        AppConfig::update_floating_assistant_settings(&path, settings).expect("feature save");
    assert_eq!(merged, AppConfig::load_from(&path).expect("durable reload"));
    assert!(merged.floating_assistant.enabled);
    assert_eq!(merged.floating_assistant.frame_rate, 60);
    assert_eq!(
        merged.floating_assistant.image_path.as_deref(),
        Some("C:\\fixture\\chosen.png")
    );
    assert_eq!(merged.floating_assistant.position.dpi, Some(144));
    assert_eq!(merged.download.default_connection_limit, 24);
    assert_eq!(merged.clipboard.storage_directory, "synthetic-latest-root");
    assert_eq!(merged.system.theme, ThemePreference::Light);
    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn floating_assistant_missing_and_corrupt_section_updates_do_not_resurrect_config() {
    let (directory, path) = fixture("failure");
    let requested = FloatingAssistantSettings {
        enabled: true,
        ..Default::default()
    };
    assert!(AppConfig::update_floating_assistant_settings(&path, requested.clone()).is_err());
    assert!(!path.exists(), "missing config must stay missing");
    fs::write(&path, b"{broken").expect("corrupt fixture");
    assert!(AppConfig::update_floating_assistant_settings(&path, requested).is_err());
    assert_eq!(fs::read(&path).expect("read corrupt fixture"), b"{broken");
    fs::remove_dir_all(directory).expect("cleanup");
}

#[test]
fn floating_assistant_reset_image_only_preserves_other_preferences() {
    let (directory, path) = fixture("image-reset");
    let settings = FloatingAssistantSettings {
        enabled: true,
        image_path: Some("C:\\fixture\\skin.gif".into()),
        frame_rate: 48,
        click_through: true,
        ..Default::default()
    };
    AppConfig::default().save_to(&path).expect("seed");
    AppConfig::update_floating_assistant_settings(&path, settings).expect("select image");
    let mut next = AppConfig::load_from(&path)
        .expect("reload")
        .floating_assistant;
    next.image_path = None;
    let saved = AppConfig::update_floating_assistant_settings(&path, next).expect("default image");
    assert_eq!(saved.floating_assistant.image_path, None);
    assert!(saved.floating_assistant.enabled);
    assert!(saved.floating_assistant.click_through);
    assert_eq!(saved.floating_assistant.frame_rate, 48);
    fs::remove_dir_all(directory).expect("cleanup");
}
