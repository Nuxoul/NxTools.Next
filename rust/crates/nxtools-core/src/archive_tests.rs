use crate::{
    AppConfig, ArchiveCompressionLevel, ArchiveConflictPolicy, ArchiveCreateFormat,
    ArchiveDestination, ArchiveFilenameEncoding, ArchiveSettings,
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
        default_destination: ArchiveDestination::Smart,
        conflict_policy: ArchiveConflictPolicy::AutoRename,
        filename_encoding: ArchiveFilenameEncoding::ShiftJis,
        create_format: ArchiveCreateFormat::SevenZ,
        compression_level: ArchiveCompressionLevel::Maximum,
        test_after_create: false,
        strip_single_root: true,
        exclude_patterns: vec!["*.tmp".into(), " keep-spaces.txt ".into()],
        copy_motw: false,
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
        default_destination: ArchiveDestination::Ask,
        conflict_policy: ArchiveConflictPolicy::Ask,
        filename_encoding: ArchiveFilenameEncoding::Auto,
        create_format: ArchiveCreateFormat::Zip,
        compression_level: ArchiveCompressionLevel::Normal,
        test_after_create: true,
        strip_single_root: false,
        exclude_patterns: vec!["Thumbs.db".into(), "desktop.ini".into()],
        copy_motw: true,
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
        if field == "open_in_nxtools" {
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
