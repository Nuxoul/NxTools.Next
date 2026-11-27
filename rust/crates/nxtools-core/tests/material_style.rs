use nxtools_core::{AppConfig, MaterialStyle, MaterialTuning, SystemSettings, ThemePreference};

#[test]
fn material_style_legacy_default_and_all_palette_font_roundtrips_are_independent() {
    for value in [serde_json::json!({}), serde_json::json!({"system": {}})] {
        let mut value = value;
        value["platform_pipe_name"] = "synthetic".into();
        value["theme"] = "system".into();
        let config: AppConfig = serde_json::from_value(value).expect("legacy config");
        assert_eq!(config.system.material_style, MaterialStyle::Solid);
    }
    let styles: Vec<&str> = MaterialStyle::ALL
        .iter()
        .map(|style| style.as_str())
        .collect();
    assert_eq!(styles, ["solid", "acrylic"]);
    // Configs saved with the former Mica styles read as acrylic.
    for legacy in ["mica", "mica_alt"] {
        let settings: SystemSettings =
            serde_json::from_value(serde_json::json!({ "material_style": legacy }))
                .expect("former Mica style");
        assert_eq!(settings.material_style, MaterialStyle::Acrylic);
    }
    assert_eq!(MaterialStyle::Acrylic.label_for(true), "亚克力");
    assert_eq!(MaterialStyle::Acrylic.label_for(false), "磨砂");
    assert_eq!(MaterialStyle::Solid.label_for(false), "实底");
    for theme in ThemePreference::ALL {
        for material_style in MaterialStyle::ALL {
            let settings = SystemSettings {
                theme,
                material_style,
                ui_font_family: "思源黑体 CN".into(),
                ..Default::default()
            };
            let json = serde_json::to_value(&settings).expect("serialize");
            assert_eq!(json["material_style"], material_style.as_str());
            let restored: SystemSettings = serde_json::from_value(json).expect("restore");
            assert_eq!(restored, settings);
        }
    }
}

#[test]
fn material_style_save_merges_only_changed_system_leaves_in_original_global_transaction() {
    let root = std::env::temp_dir().join(format!("nxtools-material-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("temporary config");
    let path = root.join("config.json");
    let initial = AppConfig::default();
    initial.save_to(&path).expect("initial");
    let baseline = initial.system.clone();
    let mut latest = initial.clone();
    latest.system.theme = ThemePreference::SolarizedLight;
    latest.system.ui_font_family = "Microsoft YaHei UI".into();
    latest.system.notification_duration_seconds = 17;
    latest.system.close_to_tray = false;
    latest.markdown_reader.font_size = 24;
    latest.save_to(&path).expect("concurrent update");
    let desired = SystemSettings {
        material_style: MaterialStyle::Acrylic,
        ..baseline.clone()
    };
    let merged = AppConfig::merge_global_settings_with_blacklist(
        &path,
        initial.network.clone(),
        &baseline,
        desired,
        initial.process_blacklist.clone(),
    )
    .expect("merge original global transaction");
    let mut expected = latest.system;
    expected.material_style = MaterialStyle::Acrylic;
    assert_eq!(merged.system, expected);
    assert_eq!(merged.markdown_reader.font_size, 24);
    assert_eq!(
        AppConfig::load_from(&path).expect("durable").system,
        expected
    );
    // An unrelated stale palette edit preserves the disk-latest material and font.
    let desired = SystemSettings {
        theme: ThemePreference::GruvboxDark,
        ..baseline.clone()
    };
    let merged = AppConfig::merge_global_settings_with_blacklist(
        &path,
        initial.network.clone(),
        &baseline,
        desired,
        initial.process_blacklist.clone(),
    )
    .expect("merge palette only");
    assert_eq!(merged.system.material_style, MaterialStyle::Acrylic);
    assert_eq!(merged.system.theme, ThemePreference::GruvboxDark);
    assert_eq!(merged.system.ui_font_family, "Microsoft YaHei UI");
    std::fs::write(&path, b"{broken").expect("inject parse failure");
    assert!(
        AppConfig::merge_global_settings_with_blacklist(
            &path,
            initial.network.clone(),
            &baseline,
            initial.system.clone(),
            initial.process_blacklist.clone(),
        )
        .is_err()
    );
    assert_eq!(std::fs::read(&path).expect("retained bytes"), b"{broken");
    std::fs::remove_file(&path).expect("inject missing target");
    assert!(
        AppConfig::merge_global_settings_with_blacklist(
            &path,
            initial.network,
            &baseline,
            initial.system,
            initial.process_blacklist,
        )
        .is_err()
    );
    assert!(!path.exists());
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn material_tuning_defaults_clamps_and_merges_as_an_independent_system_leaf() {
    let legacy: SystemSettings = serde_json::from_value(serde_json::json!({})).expect("legacy");
    assert_eq!(legacy.material_tuning, MaterialTuning::default());
    assert_eq!(legacy.material_tuning.content_percent, 50);
    for (raw, expected) in [
        (0, MaterialTuning::MIN_PERCENT),
        (4000, MaterialTuning::MAX_PERCENT),
        (73, 73),
    ] {
        let clamped: SystemSettings = serde_json::from_value(serde_json::json!({
            "material_tuning": {"content_percent": raw}
        }))
        .expect("out-of-range values are clamped, not rejected");
        assert_eq!(clamped.material_tuning.content_percent, expected);
    }
    // Tuning saved by an earlier build keeps only the content layer at its default.
    let earlier: SystemSettings = serde_json::from_value(serde_json::json!({
        "material_tuning": {"window_percent": 80, "surface_percent": 90}
    }))
    .expect("earlier tuning");
    assert_eq!(earlier.material_tuning, MaterialTuning::default());

    let root = std::env::temp_dir().join(format!("nxtools-tuning-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("temporary config");
    let path = root.join("config.json");
    let initial = AppConfig::default();
    initial.save_to(&path).expect("initial");
    let baseline = initial.system.clone();
    let mut latest = initial.clone();
    latest.system.theme = ThemePreference::SolarizedLight;
    latest.save_to(&path).expect("concurrent update");
    let desired = SystemSettings {
        material_tuning: MaterialTuning {
            content_percent: 65,
        },
        ..baseline.clone()
    };
    let merged = AppConfig::merge_global_settings_with_blacklist(
        &path,
        initial.network.clone(),
        &baseline,
        desired.clone(),
        initial.process_blacklist.clone(),
    )
    .expect("merge tuning");
    assert_eq!(merged.system.material_tuning, desired.material_tuning);
    assert_eq!(merged.system.theme, ThemePreference::SolarizedLight);
    assert_eq!(
        AppConfig::load_from(&path)
            .expect("durable")
            .system
            .material_tuning,
        desired.material_tuning
    );
    std::fs::remove_dir_all(root).expect("cleanup");
}
