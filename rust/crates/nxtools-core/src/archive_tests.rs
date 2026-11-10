use crate::archive::ArchiveTempSettings;
use crate::{
    AppConfig, ArchiveCompressionLevel, ArchiveConflictPolicy, ArchiveCreateFormat,
    ArchiveDestination, ArchiveFilenameEncoding, ArchiveSettings, ArchiveShellIntegrationSettings,
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

struct ConfigFixture(PathBuf);

impl ConfigFixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("fixture clock")
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "nxtools-archive-config-{}-{nonce}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&directory).expect("isolated archive config directory");
        Self(directory)
    }

    fn path(&self) -> PathBuf {
        self.0.join("config.json")
    }
}

impl Drop for ConfigFixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn changed_settings() -> ArchiveSettings {
    ArchiveSettings {
        open_in_nxtools: false,
        remember_recent: true,
        recent_limit: 10,
        temp: ArchiveTempSettings::default(),
        preview: Default::default(),
        auto_enter_single_root: false,
        show_parent_row: false,
        show_grid_lines: true,
        full_row_select: false,
        single_click_open: true,
        show_tree: false,
        show_full_path: true,
        auto_expand_tree: false,
        escape_closes_window: true,
        open_after_extract: false,
        keep_extract_progress_open: true,
        smart_single_file_folder: true,
        keep_create_progress_open: true,
        extract_custom_destination: Some(r"C:\synthetic\output".into()),
        default_destination: ArchiveDestination::Smart,
        conflict_policy: ArchiveConflictPolicy::AutoRename,
        filename_encoding: ArchiveFilenameEncoding::ShiftJis,
        create_format: ArchiveCreateFormat::SevenZ,
        compression_level: ArchiveCompressionLevel::Maximum,
        create_volume_size: None,
        test_after_create: false,
        strip_single_root: true,
        exclude_patterns: vec!["*.tmp".into(), " keep-spaces.txt ".into()],
        copy_motw: false,
        shell_integration: ArchiveShellIntegrationSettings {
            classic_cascade: true,
            open_with: false,
            extensions: vec![".zip".into(), ".7z".into()],
            desired_revision: 4,
            ..Default::default()
        },
    }
}

fn assert_rejected_without_rewrite(path: &Path, bytes: &[u8]) {
    fs::write(path, bytes).expect("write invalid archive fixture");
    assert!(AppConfig::load_from(path).is_err());
    assert_eq!(fs::read(path).expect("bytes after failed load"), bytes);
    assert!(AppConfig::update_archive_settings(path, ArchiveSettings::default()).is_err());
    assert_eq!(fs::read(path).expect("bytes after failed update"), bytes);
}

#[test]
fn absent_archive_section_loads_exact_defaults_without_rewriting_legacy_bytes() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    let mut legacy = serde_json::to_value(AppConfig::default()).expect("default config JSON");
    legacy
        .as_object_mut()
        .expect("config object")
        .remove("archive");
    let bytes = serde_json::to_vec(&legacy).expect("legacy JSON");
    fs::write(&path, &bytes).expect("legacy config");

    let expected = ArchiveSettings {
        open_in_nxtools: true,
        remember_recent: true,
        recent_limit: 10,
        temp: ArchiveTempSettings::default(),
        preview: Default::default(),
        auto_enter_single_root: true,
        show_parent_row: true,
        show_grid_lines: false,
        full_row_select: true,
        single_click_open: false,
        show_tree: true,
        show_full_path: false,
        auto_expand_tree: true,
        escape_closes_window: false,
        open_after_extract: true,
        keep_extract_progress_open: false,
        smart_single_file_folder: false,
        keep_create_progress_open: false,
        extract_custom_destination: None,
        default_destination: ArchiveDestination::Ask,
        conflict_policy: ArchiveConflictPolicy::Ask,
        filename_encoding: ArchiveFilenameEncoding::Auto,
        create_format: ArchiveCreateFormat::Zip,
        compression_level: ArchiveCompressionLevel::Normal,
        create_volume_size: None,
        test_after_create: true,
        strip_single_root: false,
        exclude_patterns: vec!["Thumbs.db".into(), "desktop.ini".into()],
        copy_motw: true,
        shell_integration: ArchiveShellIntegrationSettings::default(),
    };
    assert_eq!(ArchiveSettings::default(), expected);
    assert_eq!(
        AppConfig::load_from(&path).expect("legacy load").archive,
        expected
    );
    assert_eq!(fs::read(&path).expect("legacy bytes after load"), bytes);
}

#[test]
fn changed_archive_settings_save_and_reload_exactly() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    let config = AppConfig {
        archive: changed_settings(),
        ..AppConfig::default()
    };
    config.save_to(&path).expect("save changed archive presets");
    assert_eq!(
        AppConfig::load_from(&path).expect("reload presets").archive,
        config.archive
    );
}

#[test]
fn stale_archive_update_preserves_latest_unrelated_sections() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    AppConfig::default().save_to(&path).expect("seed config");
    let mut stale = AppConfig::load_from(&path).expect("stale archive owner");
    let latest = AppConfig::update_section(&path, |config| {
        config.system.close_to_tray = false;
        config.download.default_connection_limit = 24;
        config.clipboard.storage_directory = "latest-owned-root".into();
    })
    .expect("other owners persist changes");
    stale.archive = changed_settings();

    let merged = AppConfig::update_archive_settings(&path, stale.archive.clone())
        .expect("locked archive-only merge");
    let mut expected = latest;
    expected.archive = stale.archive;
    assert_eq!(merged, expected);
    assert_eq!(
        AppConfig::load_from(&path).expect("reload merged config"),
        expected
    );
}

#[test]
fn malformed_json_and_missing_update_targets_are_not_recreated() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    assert_rejected_without_rewrite(&path, b"{ \"archive\": ");
    fs::remove_file(&path).expect("remove update target");
    assert!(AppConfig::update_archive_settings(&path, changed_settings()).is_err());
    assert!(!path.exists(), "missing config must not be resurrected");
}

#[test]
fn present_invalid_archive_sections_and_missing_required_fields_are_rejected() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    let valid = serde_json::to_value(AppConfig::default()).expect("config JSON");
    for invalid in [Value::Null, json!({}), json!([]), json!("invalid")] {
        let mut config = valid.clone();
        config["archive"] = invalid;
        assert_rejected_without_rewrite(&path, &serde_json::to_vec(&config).expect("invalid JSON"));
    }
    for field in valid["archive"].as_object().expect("archive object").keys() {
        // Additive fields with documented legacy defaults may be absent.
        if [
            "open_in_nxtools",
            "shell_integration",
            "auto_enter_single_root",
            "show_parent_row",
            "show_grid_lines",
            "full_row_select",
            "single_click_open",
            "show_tree",
            "show_full_path",
            "auto_expand_tree",
            "escape_closes_window",
            "open_after_extract",
            "keep_extract_progress_open",
            "smart_single_file_folder",
            "keep_create_progress_open",
            "extract_custom_destination",
            "remember_recent",
            "recent_limit",
            "preview",
            "temp",
        ]
        .contains(&field.as_str())
        {
            continue;
        }
        let mut config = valid.clone();
        config["archive"]
            .as_object_mut()
            .expect("archive object")
            .remove(field);
        assert_rejected_without_rewrite(
            &path,
            &serde_json::to_vec(&config).expect("missing field JSON"),
        );
    }
    for (field, invalid) in [
        ("open_in_nxtools", Value::Null),
        ("default_destination", json!("elsewhere")),
        ("conflict_policy", json!("replace")),
        ("filename_encoding", json!("unknown")),
        ("create_format", json!("rar")),
        ("compression_level", json!(99)),
        ("test_after_create", json!("true")),
        ("strip_single_root", Value::Null),
        ("exclude_patterns", json!([123])),
        ("copy_motw", Value::Null),
        ("shell_integration", Value::Null),
        ("shell_integration", json!({"classic_cascade": true})),
        (
            "shell_integration",
            json!({"classic_cascade": false, "open_with": false, "extensions": [".exe"], "desired_revision": 0}),
        ),
    ] {
        let mut config = valid.clone();
        config["archive"][field] = invalid;
        assert_rejected_without_rewrite(
            &path,
            &serde_json::to_vec(&config).expect("invalid field JSON"),
        );
    }
}

#[test]
fn archive_pattern_validation_is_bounded_and_never_normalizes_corrupt_values() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    AppConfig::default().save_to(&path).expect("seed config");
    let original = fs::read(&path).expect("original bytes");
    for patterns in [
        vec!["*.tmp".into(); 33],
        vec!["a".repeat(257)],
        vec!["bad\0pattern".into()],
        vec!["[".into()],
        vec!["../escape".into()],
        vec![r"folder\file".into()],
    ] {
        let settings = ArchiveSettings {
            exclude_patterns: patterns,
            ..ArchiveSettings::default()
        };
        assert!(settings.validate().is_err());
        let mut config = AppConfig {
            archive: settings.clone(),
            ..AppConfig::default()
        };
        config.normalize();
        assert_eq!(
            config.archive, settings,
            "normalize must not repair archive data"
        );
        assert!(AppConfig::update_archive_settings(&path, settings).is_err());
        assert!(config.save_to(&path).is_err());
        assert_eq!(fs::read(&path).expect("unchanged valid config"), original);
        let invalid = serde_json::to_vec(&config).expect("invalid persisted fixture");
        assert_rejected_without_rewrite(&path, &invalid);
        fs::write(&path, &original).expect("restore valid fixture");
    }
}

#[test]
fn archive_pattern_validation_accepts_exact_character_and_count_limits() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    AppConfig::default().save_to(&path).expect("seed config");
    for patterns in [
        Vec::new(),
        vec!["a".repeat(256); 32],
        vec!["界".repeat(256)],
    ] {
        let settings = ArchiveSettings {
            exclude_patterns: patterns,
            ..ArchiveSettings::default()
        };
        assert!(settings.validate().is_ok());
        let merged = AppConfig::update_archive_settings(&path, settings.clone())
            .expect("persist bounded patterns without alteration");
        assert_eq!(merged.archive, settings);
        assert_eq!(
            AppConfig::load_from(&path)
                .expect("reload bounded patterns")
                .archive,
            settings
        );
    }
}

#[test]
fn tar_preserves_format_independent_compression_preference() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    AppConfig::default().save_to(&path).expect("seed config");
    let settings = ArchiveSettings {
        create_format: ArchiveCreateFormat::Tar,
        compression_level: ArchiveCompressionLevel::Maximum,
        ..ArchiveSettings::default()
    };
    AppConfig::update_archive_settings(&path, settings.clone()).expect("persist TAR preference");
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("reload TAR preference")
            .archive,
        settings
    );
}

#[test]
fn single_stream_formats_reject_store_without_resetting_saved_preferences() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    AppConfig::default().save_to(&path).expect("seed config");
    let original = fs::read(&path).expect("original bytes");
    for create_format in [ArchiveCreateFormat::Gz, ArchiveCreateFormat::Xz] {
        let settings = ArchiveSettings {
            create_format,
            compression_level: ArchiveCompressionLevel::Store,
            ..ArchiveSettings::default()
        };
        assert!(settings.validate().is_err());
        assert!(AppConfig::update_archive_settings(&path, settings.clone()).is_err());
        assert_eq!(fs::read(&path).expect("unchanged valid config"), original);
        let mut config = AppConfig {
            archive: settings.clone(),
            ..AppConfig::default()
        };
        config.normalize();
        assert_eq!(config.archive, settings);
        assert_rejected_without_rewrite(
            &path,
            &serde_json::to_vec(&config).expect("invalid combo"),
        );
        fs::write(&path, &original).expect("restore valid fixture");
    }
}

#[test]
fn phase_d_internal_archive_open_default_is_explicit_and_legacy_compatible() {
    let defaults = serde_json::to_value(ArchiveSettings::default()).expect("archive defaults");
    assert_eq!(
        defaults["open_in_nxtools"],
        json!(true),
        "internal archive open defaults on"
    );
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    let mut legacy = serde_json::to_value(AppConfig::default()).expect("legacy config");
    legacy["archive"]
        .as_object_mut()
        .expect("section")
        .remove("open_in_nxtools");
    let bytes = serde_json::to_vec(&legacy).expect("legacy bytes");
    fs::write(&path, &bytes).expect("seed legacy");
    let loaded = serde_json::to_value(
        AppConfig::load_from(&path)
            .expect("legacy accepted")
            .archive,
    )
    .expect("loaded");
    assert_eq!(loaded["open_in_nxtools"], json!(true));
    assert_eq!(fs::read(path).expect("read only"), bytes);
}

#[test]
fn phase_f_shell_integration_desired_intent_defaults_off_and_validates() {
    let defaults = ArchiveSettings::default();
    assert_eq!(
        defaults.shell_integration,
        ArchiveShellIntegrationSettings::default()
    );
    assert!(!defaults.shell_integration.classic_cascade && !defaults.shell_integration.open_with);
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    let mut legacy = serde_json::to_value(AppConfig::default()).expect("legacy config");
    legacy["archive"]
        .as_object_mut()
        .expect("section")
        .remove("shell_integration");
    let bytes = serde_json::to_vec(&legacy).expect("legacy bytes");
    fs::write(&path, &bytes).expect("seed legacy");
    let loaded = AppConfig::load_from(&path)
        .expect("legacy accepted")
        .archive;
    assert_eq!(
        loaded.shell_integration,
        ArchiveShellIntegrationSettings::default()
    );
    assert_eq!(
        fs::read(&path).expect("read only"),
        bytes,
        "load never rewrites"
    );

    let desired = ArchiveSettings {
        shell_integration: ArchiveShellIntegrationSettings {
            classic_cascade: true,
            open_with: true,
            extensions: vec![".zip".into()],
            desired_revision: 1,
            ..Default::default()
        },
        ..ArchiveSettings::default()
    };
    let merged = AppConfig::update_archive_settings(&path, desired.clone()).expect("save desired");
    assert_eq!(merged.archive.shell_integration, desired.shell_integration);
    for invalid in [
        ArchiveShellIntegrationSettings {
            extensions: vec![".exe".into()],
            ..Default::default()
        },
        ArchiveShellIntegrationSettings {
            extensions: vec![".zip".into(), ".zip".into()],
            ..Default::default()
        },
        ArchiveShellIntegrationSettings {
            desired_revision: -1,
            ..Default::default()
        },
    ] {
        let settings = ArchiveSettings {
            shell_integration: invalid,
            ..ArchiveSettings::default()
        };
        assert!(settings.validate().is_err());
    }
    let draft = ArchiveSettings {
        shell_integration: ArchiveShellIntegrationSettings {
            classic_cascade: true,
            ..Default::default()
        },
        ..ArchiveSettings::default()
    };
    assert!(
        draft.validate().is_ok(),
        "an enabled draft may be saved before choosing extensions"
    );
    let mut unknown = serde_json::to_value(&desired).expect("desired json");
    unknown["shell_integration"]["applied"] = json!(true);
    assert!(
        serde_json::from_value::<ArchiveSettings>(unknown).is_err(),
        "observed applied facts cannot be smuggled into saved desired settings"
    );
}

#[test]
fn ordinary_archive_preferences_default_migrate_and_persist_without_rewriting_on_error() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    let fields = [
        ("auto_enter_single_root", true),
        ("show_parent_row", true),
        ("show_grid_lines", false),
        ("full_row_select", true),
        ("single_click_open", false),
        ("show_tree", true),
        ("show_full_path", false),
        ("auto_expand_tree", true),
        ("escape_closes_window", false),
        ("open_after_extract", true),
        ("keep_extract_progress_open", false),
        ("smart_single_file_folder", false),
        ("keep_create_progress_open", false),
    ];
    let defaults = serde_json::to_value(ArchiveSettings::default()).expect("defaults");
    let mut legacy = serde_json::to_value(AppConfig::default()).expect("legacy config");
    for (field, value) in fields {
        assert_eq!(
            defaults[field],
            json!(value),
            "explicit default for {field}"
        );
        legacy["archive"]
            .as_object_mut()
            .expect("archive")
            .remove(field);
    }
    let bytes = serde_json::to_vec(&legacy).expect("legacy bytes");
    fs::write(&path, &bytes).expect("legacy fixture");
    let loaded = AppConfig::load_from(&path).expect("additive migration");
    assert_eq!(loaded.archive, ArchiveSettings::default());
    assert_eq!(fs::read(&path).expect("read only migration"), bytes);
    let mut changed = serde_json::to_value(&loaded.archive).expect("settings");
    for (field, value) in fields {
        changed[field] = json!(!value);
    }
    let changed: ArchiveSettings = serde_json::from_value(changed).expect("opposite values");
    let merged = AppConfig::update_archive_settings(&path, changed.clone()).expect("real save");
    assert_eq!(merged.archive, changed);
    assert_eq!(
        AppConfig::load_from(&path).expect("reopen").archive,
        changed
    );
    let valid = serde_json::to_value(&merged).expect("saved config");
    for (field, _) in fields {
        let mut invalid = valid.clone();
        invalid["archive"][field] = Value::Null;
        assert_rejected_without_rewrite(&path, &serde_json::to_vec(&invalid).expect("bad bool"));
    }
}

#[test]
fn shell_settings_legacy_defaults_and_strict_choices_are_persisted() {
    let old =
        json!({"classic_cascade":false,"open_with":false,"extensions":[],"desired_revision":0});
    let legacy: ArchiveShellIntegrationSettings =
        serde_json::from_value(old.clone()).expect("legacy fields");
    let encoded = serde_json::to_value(&legacy).expect("settings JSON");
    assert_eq!(encoded["double_click_action"], json!("open"));
    assert_eq!(encoded["menu_layout"], json!("cascade"));
    assert_eq!(
        encoded["menu_commands"],
        json!([
            "open",
            "extract_smart",
            "extract_here",
            "extract_name",
            "test",
            "create_zip",
            "create_7z",
            "create"
        ])
    );
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    AppConfig::default().save_to(&path).expect("seed");
    let mut updated = encoded.clone();
    updated["double_click_action"] = json!("smart");
    updated["menu_layout"] = json!("flat");
    updated["menu_commands"] = json!(["extract_dialog", "create"]);
    let integration = serde_json::from_value(updated).expect("supported choices");
    let settings = ArchiveSettings {
        shell_integration: integration,
        ..Default::default()
    };
    AppConfig::update_archive_settings(&path, settings.clone()).expect("save choices");
    assert_eq!(
        AppConfig::load_from(&path).expect("reopen").archive,
        settings
    );
    let bytes = fs::read(&path).expect("before invalid");
    for (field, invalid) in [
        ("double_click_action", json!("admin")),
        ("menu_layout", json!("windows11")),
        ("menu_commands", json!(["open", "open"])),
        ("menu_commands", json!(["delete_source"])),
        ("menu_commands", Value::Null),
    ] {
        let mut value = encoded.clone();
        value[field] = invalid;
        if let Ok(integration) = serde_json::from_value::<ArchiveShellIntegrationSettings>(value) {
            assert!(
                AppConfig::update_archive_settings(
                    &path,
                    ArchiveSettings {
                        shell_integration: integration,
                        ..Default::default()
                    }
                )
                .is_err()
            );
        }
        assert_eq!(fs::read(&path).expect("unchanged"), bytes);
    }
}

#[test]
fn ui_remaining_recent_settings_have_real_legacy_defaults_and_strict_bounds() {
    let mut legacy = serde_json::to_value(ArchiveSettings::default()).expect("legacy settings");
    legacy
        .as_object_mut()
        .expect("object")
        .remove("remember_recent");
    legacy
        .as_object_mut()
        .expect("object")
        .remove("recent_limit");
    let loaded: ArchiveSettings = serde_json::from_value(legacy.clone()).expect("legacy config");
    let value = serde_json::to_value(loaded).expect("current config");
    assert_eq!(value["remember_recent"], json!(true));
    assert_eq!(value["recent_limit"], json!(10));
    for (field, invalid) in [
        ("remember_recent", Value::Null),
        ("recent_limit", json!(31)),
        ("recent_limit", json!(-1)),
        ("recent_limit", Value::Null),
    ] {
        let mut candidate = legacy.clone();
        candidate[field] = invalid;
        if let Ok(settings) = serde_json::from_value::<ArchiveSettings>(candidate) {
            assert!(settings.validate().is_err(), "invalid {field} must reject");
        }
    }
}

#[test]
fn ui_remaining_temp_settings_save_reopen_and_invalid_values_do_not_rewrite() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    AppConfig::default().save_to(&path).expect("seed");
    let mut settings = ArchiveSettings {
        temp: ArchiveTempSettings {
            base_directory: r"C:\owned\temporary".into(),
            retention_hours: 6,
            cleanup_on_child_exit: true,
        },
        ..Default::default()
    };
    AppConfig::update_archive_settings(&path, settings.clone()).expect("temp save");
    assert_eq!(
        AppConfig::load_from(&path).expect("reopen").archive.temp,
        settings.temp
    );
    let bytes = fs::read(&path).expect("before invalid");
    for invalid in [
        r"relative",
        r"\\server\share",
        r"\\?\C:\owned",
        r"C:\owned\..\other",
        r"C:\owned\bad.",
        r"C:\\",
        r"C:\owned\\child",
        r"C:\owned\CON.txt",
    ] {
        settings.temp.base_directory = invalid.into();
        assert!(AppConfig::update_archive_settings(&path, settings.clone()).is_err());
        assert_eq!(fs::read(&path).expect("preserved"), bytes);
    }
    settings.temp.base_directory.clear();
    settings.temp.retention_hours = 0;
    assert!(settings.validate().is_err());
    let mut old = serde_json::to_value(ArchiveSettings::default()).expect("legacy");
    old.as_object_mut().expect("object").remove("temp");
    assert_eq!(
        serde_json::from_value::<ArchiveSettings>(old)
            .expect("legacy load")
            .temp,
        ArchiveTempSettings::default()
    );
}

#[test]
fn remembered_extract_custom_destination_is_additive_validated_and_field_merged() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    let baseline = ArchiveSettings::default();
    let mut legacy = serde_json::to_value(AppConfig::default()).expect("legacy config");
    legacy["archive"]
        .as_object_mut()
        .expect("archive")
        .remove("extract_custom_destination");
    fs::write(&path, serde_json::to_vec(&legacy).expect("encode")).expect("seed");
    assert!(
        AppConfig::load_from(&path)
            .expect("old additive field")
            .archive
            .extract_custom_destination
            .is_none()
    );
    AppConfig::update_section(&path, |config| {
        config.archive.copy_motw = false;
        config.archive.show_tree = false;
    })
    .expect("concurrent setting");
    let mut desired = baseline.clone();
    desired.keep_extract_progress_open = true;
    desired.extract_custom_destination = Some(r"C:\synthetic\remembered".into());
    AppConfig::update_section(&path, |config| {
        config
            .archive
            .merge_extract_preferences(&baseline, &desired)
    })
    .expect("original field transaction");
    let saved = AppConfig::load_from(&path).expect("reload").archive;
    assert!(
        !saved.copy_motw && !saved.show_tree,
        "untouched fields retain disk latest"
    );
    assert!(saved.keep_extract_progress_open);
    assert_eq!(
        saved.extract_custom_destination,
        desired.extract_custom_destination
    );
    let mut invalid = serde_json::to_value(AppConfig::default()).expect("config");
    invalid["archive"]["extract_custom_destination"] = json!("relative/path");
    assert_rejected_without_rewrite(&path, &serde_json::to_vec(&invalid).expect("invalid"));
}

#[test]
fn preview_defaults_are_additive_without_opening_write_and_bounds_are_validated() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    let mut legacy = serde_json::to_value(AppConfig::default()).expect("config");
    legacy["archive"]
        .as_object_mut()
        .expect("archive")
        .remove("preview");
    let bytes = serde_json::to_vec(&legacy).expect("legacy JSON");
    fs::write(&path, &bytes).expect("legacy config");
    let loaded = AppConfig::load_from(&path).expect("legacy preview defaults");
    assert_eq!(fs::read(&path).expect("unchanged bytes"), bytes);
    let expected = json!({
        "text_limit_kib": 1024, "text_max_lines": 20000,
        "image_limit_mib": 32, "image_max_megapixels": 16,
        "image_decode_mib": 512, "timeout_seconds": 120
    });
    let mut value = serde_json::to_value(loaded).expect("loaded config");
    assert_eq!(value["archive"]["preview"], expected);
    for (field, minimum, maximum) in [
        ("text_limit_kib", 1, 8192),
        ("text_max_lines", 1, 100000),
        ("image_limit_mib", 1, 128),
        ("image_max_megapixels", 1, 64),
        ("image_decode_mib", 32, 1024),
        ("timeout_seconds", 5, 120),
    ] {
        for valid in [minimum, maximum] {
            value["archive"]["preview"] = expected.clone();
            value["archive"]["preview"][field] = json!(valid);
            let config: AppConfig = serde_json::from_value(value.clone()).expect("typed bound");
            assert!(config.archive.validate().is_ok(), "{field}={valid}");
        }
        for invalid in [minimum - 1, maximum + 1] {
            value["archive"]["preview"] = expected.clone();
            value["archive"]["preview"][field] = json!(invalid);
            assert_rejected_without_rewrite(&path, &serde_json::to_vec(&value).expect("invalid"));
        }
    }
    for invalid in [
        Value::Null,
        json!({"unknown": 1}),
        json!([]),
        json!("invalid"),
    ] {
        value["archive"]["preview"] = invalid;
        assert_rejected_without_rewrite(&path, &serde_json::to_vec(&value).expect("invalid shape"));
    }
}

#[test]
fn preview_changed_leaf_merge_preserves_latest_fields_and_other_sections() {
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    AppConfig::default().save_to(&path).expect("seed");
    let baseline = AppConfig::load_from(&path)
        .expect("baseline")
        .archive
        .preview;
    let mut desired = baseline;
    desired.text_limit_kib = 2048;
    desired.timeout_seconds = 60;
    AppConfig::update_section(&path, |config| {
        config.archive.preview.image_limit_mib = 64;
        config.archive.preview.text_max_lines = 40_000;
        config.system.close_to_tray = false;
    })
    .expect("concurrent owner");
    let merged = AppConfig::update_section(&path, |config| {
        config.archive.preview.merge_changed(&baseline, &desired);
    })
    .expect("original locked section update");
    assert_eq!(merged.archive.preview.text_limit_kib, 2048);
    assert_eq!(merged.archive.preview.timeout_seconds, 60);
    assert_eq!(merged.archive.preview.image_limit_mib, 64);
    assert_eq!(merged.archive.preview.text_max_lines, 40_000);
    assert!(!merged.system.close_to_tray);
    assert_eq!(AppConfig::load_from(&path).expect("reload"), merged);
}

#[test]
fn ref17_size_units_legacy_mib_and_load_are_lossless_without_writes() {
    use crate::archive::{ArchiveVolumeSize, ArchiveVolumeUnit};
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    let mut json = serde_json::to_value(AppConfig::default()).expect("configuration");
    assert!(json["archive"].get("create_volume_size").is_none());
    json["archive"]["create_volume_size"] = json!({"value":128});
    let encoded = serde_json::to_vec(&json).expect("legacy integer MiB");
    fs::write(&path, &encoded).expect("legacy fixture");
    let loaded = AppConfig::load_from(&path).expect("absent unit is MiB");
    assert_eq!(
        loaded
            .archive
            .create_volume_size
            .expect("saved size")
            .bytes()
            .expect("bytes"),
        134_217_728
    );
    assert_eq!(fs::read(&path).expect("untouched bytes"), encoded);
    for (value, unit, bytes) in [
        (64, ArchiveVolumeUnit::KiB, 65_536),
        (1536, ArchiveVolumeUnit::KiB, 1_572_864),
        (2, ArchiveVolumeUnit::GiB, 2_147_483_648),
    ] {
        let desired = ArchiveVolumeSize { value, unit };
        let saved = AppConfig::update_section(&path, |config| {
            config.archive.create_volume_size = Some(desired)
        })
        .expect("original section transaction");
        assert_eq!(saved.archive.create_volume_size, Some(desired));
        let reopened = AppConfig::load_from(&path).expect("reopen");
        assert_eq!(
            reopened
                .archive
                .create_volume_size
                .expect("size")
                .bytes()
                .expect("checked bytes"),
            bytes
        );
    }
    for unit in ArchiveVolumeUnit::ALL {
        assert!(
            ArchiveVolumeSize {
                value: u64::MAX,
                unit
            }
            .bytes()
            .is_err()
        );
        assert!(ArchiveVolumeSize { value: 0, unit }.bytes().is_err());
    }
}

#[test]
fn ref17_size_is_one_changed_leaf_in_original_section_transaction() {
    use crate::archive::{ArchiveVolumeSize, ArchiveVolumeUnit};
    let fixture = ConfigFixture::new();
    let path = fixture.path();
    AppConfig::default().save_to(&path).expect("seed");
    let baseline = ArchiveSettings::default();
    let mut desired = baseline.clone();
    desired.keep_create_progress_open = true;
    let exact = Some(ArchiveVolumeSize {
        value: 1536,
        unit: ArchiveVolumeUnit::KiB,
    });
    AppConfig::update_section(&path, |config| {
        config.archive.create_volume_size = exact;
        config.archive.test_after_create = false;
    })
    .expect("concurrent save");
    AppConfig::update_section(&path, |config| {
        config.archive.merge_create_preferences(&baseline, &desired)
    })
    .expect("changed leaves");
    let reopened = AppConfig::load_from(&path).expect("merged reopen");
    assert_eq!(reopened.archive.create_volume_size, exact);
    assert!(!reopened.archive.test_after_create && reopened.archive.keep_create_progress_open);
}
