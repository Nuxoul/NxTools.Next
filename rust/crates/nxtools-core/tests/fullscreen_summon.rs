use std::{fs, path::PathBuf, time::SystemTime};

use nxtools_core::{AppConfig, ProcessBlacklistSettings};
use serde_json::{Value, json};

fn fixture(name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("fixture clock")
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "nxtools-fullscreen-{name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).expect("fixture directory");
    directory.join("config.json")
}

#[test]
fn fullscreen_summon_legacy_choice_migrates_without_writing_on_load() {
    let path = fixture("legacy");
    for legacy in [false, true] {
        for present_blacklist in [false, true] {
            let mut value = serde_json::to_value(AppConfig::default()).expect("default config");
            value["quick_launch"]["suppress_in_exclusive_fullscreen"] = json!(legacy);
            if present_blacklist {
                value["process_blacklist"]
                    .as_object_mut()
                    .expect("blacklist")
                    .remove("suppress_in_fullscreen");
            } else {
                value
                    .as_object_mut()
                    .expect("config")
                    .remove("process_blacklist");
            }
            let bytes = serde_json::to_vec(&value).expect("legacy bytes");
            fs::write(&path, &bytes).expect("legacy fixture");
            let loaded = AppConfig::load_from(&path).expect("legacy load");
            let normalized = serde_json::to_value(loaded).expect("normalized config");
            assert_eq!(
                normalized["process_blacklist"]["suppress_in_fullscreen"],
                json!(legacy)
            );
            assert_eq!(fs::read(&path).expect("read unchanged config"), bytes);
        }
    }
    fs::remove_dir_all(path.parent().expect("fixture parent")).expect("cleanup");
}

#[test]
fn fullscreen_summon_global_choice_survives_section_and_rule_updates() {
    let path = fixture("global");
    for global in [false, true] {
        let mut value = serde_json::to_value(AppConfig::default()).expect("default config");
        value["quick_launch"]["suppress_in_exclusive_fullscreen"] = json!(!global);
        value["process_blacklist"]["suppress_in_fullscreen"] = json!(global);
        fs::write(&path, serde_json::to_vec(&value).expect("global bytes")).expect("seed");
        let loaded = AppConfig::load_from(&path).expect("global load");
        let loaded_value: Value = serde_json::to_value(&loaded).expect("loaded value");
        assert_eq!(
            loaded_value["process_blacklist"]["suppress_in_fullscreen"],
            json!(global)
        );
        AppConfig::update_quick_launch_settings(&path, loaded.quick_launch).expect("sibling save");
        AppConfig::update_process_blacklist(&path, ProcessBlacklistSettings::from_entries(vec![]))
            .expect("clear rules without changing fullscreen policy");
        let saved = AppConfig::load_from(&path).expect("reload");
        assert!(saved.process_blacklist.entries().is_empty());
        assert_eq!(
            serde_json::to_value(saved).expect("saved value")["process_blacklist"]["suppress_in_fullscreen"],
            json!(global)
        );
    }
    fs::remove_dir_all(path.parent().expect("fixture parent")).expect("cleanup");
}
