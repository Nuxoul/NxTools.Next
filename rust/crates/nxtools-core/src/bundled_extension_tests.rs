use crate::{AppConfig, AppPaths};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

struct UserDirectory(PathBuf);
impl UserDirectory {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "nxtools-bundled-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("isolated user directory");
        Self(root)
    }
    fn paths(&self) -> AppPaths {
        AppPaths {
            config_dir: self.0.clone(),
            data_dir: self.0.join("data"),
            cache_dir: self.0.join("cache"),
            log_dir: self.0.join("logs"),
        }
    }
}
impl Drop for UserDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
const IDS: [&str; 3] = [
    "example.github-public-proxy",
    "example.download-link-list",
    "example.clipboard-text-tools",
];

#[test]
fn new_computer_load_or_create_contains_three_built_in_extensions_without_import() {
    let directory = UserDirectory::new();
    let paths = directory.paths();
    assert!(
        AppConfig::default().extensions.packages.is_empty(),
        "plain defaults remain side-effect free"
    );
    let config =
        AppConfig::load_or_create(&paths).expect("first-run production config initialization");
    assert_eq!(
        config.extensions.packages.len(),
        3,
        "a new installation must expose all three built-in extensions without manual ZIP import"
    );
    for id in IDS {
        let package = config
            .extensions
            .packages
            .iter()
            .find(|package| package.manifest.id == id)
            .expect("built-in catalog entry");
        assert!(!package.enabled && package.trust.is_none());
        assert!(package.enabled_events.is_empty() && package.enabled_providers.is_empty());
        assert!(package.runtime_override.is_none());
        package
            .verify_current_code()
            .expect("immutable built-in snapshot");
        assert!(
            package
                .code_directory
                .as_ref()
                .expect("staged code")
                .starts_with(
                    fs::canonicalize(directory.0.join("extensions")).expect("managed root")
                )
        );
    }
    let proxy = config
        .extensions
        .packages
        .iter()
        .find(|package| package.manifest.id == IDS[0])
        .expect("proxy");
    assert_eq!(
        proxy.settings.get("third_party_opt_in"),
        Some(&serde_json::json!(false))
    );
    let bytes = fs::read(paths.config_file()).expect("first committed config");
    let reopened = AppConfig::load_or_create(&paths).expect("repeat startup");
    assert_eq!(reopened.extensions, config.extensions);
    assert_eq!(
        bytes,
        fs::read(paths.config_file()).expect("unchanged config")
    );
}

#[test]
fn legacy_config_gets_bundles_once_preserving_existing_package_and_sections() {
    let directory = UserDirectory::new();
    let paths = directory.paths();
    let mut config = AppConfig::default();
    config.download.default_connection_limit = 24;
    config.theme = crate::ThemePreference::Light;
    let existing=nxtools_extensions::ImportPreview::from_json(br#"{"schema_version":1,"id":"example.clipboard-text-tools","name":"User package","version":"9.0.0","handlers":[{"id":"open","title":"User action","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://example.invalid/"}}]}"#).expect("user package");
    config
        .extensions
        .install(existing, "user-import".into(), false)
        .expect("user install");
    config
        .extensions
        .set_enabled(IDS[2], true, &[])
        .expect("enabled user declaration");
    let expected = config.extensions.packages[0].clone();
    config.save_to(&paths.config_file()).expect("legacy config");
    let mut legacy: serde_json::Value =
        serde_json::from_slice(&fs::read(paths.config_file()).expect("legacy bytes"))
            .expect("legacy json");
    legacy["extensions"]
        .as_object_mut()
        .expect("registry")
        .remove("bundled_defaults_initialized");
    fs::write(
        paths.config_file(),
        serde_json::to_vec(&legacy).expect("old schema"),
    )
    .expect("old registry schema");
    let migrated = AppConfig::load_or_create(&paths).expect("legacy bootstrap");
    assert_eq!(migrated.extensions.packages.len(), 3);
    assert_eq!(
        migrated
            .extensions
            .packages
            .iter()
            .find(|package| package.manifest.id == IDS[2]),
        Some(&expected)
    );
    assert_eq!(migrated.download.default_connection_limit, 24);
    assert_eq!(migrated.theme, crate::ThemePreference::Light);
    assert!(migrated.extensions.bundled_defaults_initialized);
}

#[test]
fn removing_every_bundle_is_authoritative_and_does_not_reseed() {
    let directory = UserDirectory::new();
    let paths = directory.paths();
    AppConfig::load_or_create(&paths).expect("bootstrap");
    for id in IDS {
        AppConfig::remove_extension(&paths.config_file(), id).expect("remove bundle");
    }
    let bytes = fs::read(paths.config_file()).expect("empty committed catalog");
    let config = AppConfig::load_or_create(&paths).expect("reopen empty catalog");
    assert!(config.extensions.packages.is_empty());
    assert!(config.extensions.bundled_defaults_initialized);
    assert_eq!(bytes, fs::read(paths.config_file()).expect("no rewrite"));
}

#[test]
fn corrupt_config_and_failed_stage_leave_existing_configuration_bytes_untouched() {
    let directory = UserDirectory::new();
    let paths = directory.paths();
    fs::write(paths.config_file(), b"not config").expect("corrupt fixture");
    assert!(AppConfig::load_or_create(&paths).is_err());
    assert_eq!(
        fs::read(paths.config_file()).expect("corrupt retained"),
        b"not config"
    );
    assert!(!directory.0.join("extensions").exists());
    AppConfig::default()
        .save_to(&paths.config_file())
        .expect("legacy configuration");
    fs::write(directory.0.join("extensions"), b"block directory").expect("stage failure fixture");
    let bytes = fs::read(paths.config_file()).expect("prior config");
    assert!(AppConfig::load_or_create(&paths).is_err());
    assert_eq!(
        bytes,
        fs::read(paths.config_file()).expect("last good retained")
    );
    assert!(
        !AppConfig::load_from(&paths.config_file())
            .expect("still parseable")
            .extensions
            .bundled_defaults_initialized
    );
    fs::remove_file(directory.0.join("extensions")).expect("remove owned blocker");
    assert_eq!(
        AppConfig::load_or_create(&paths)
            .expect("retry initialization")
            .extensions
            .packages
            .len(),
        3
    );
}

#[test]
fn concurrent_new_user_initialization_commits_one_identical_catalog() {
    let directory = UserDirectory::new();
    let paths = directory.paths();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let workers = (0..2)
        .map(|_| {
            let paths = paths.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                AppConfig::load_or_create(&paths).expect("serialized bootstrap")
            })
        })
        .collect::<Vec<_>>();
    let configs = workers
        .into_iter()
        .map(|worker| worker.join().expect("startup worker"))
        .collect::<Vec<_>>();
    assert_eq!(configs[0].extensions, configs[1].extensions);
    assert_eq!(
        AppConfig::load_from(&paths.config_file())
            .expect("committed snapshot")
            .extensions
            .packages
            .len(),
        3
    );
}

#[test]
fn initialized_existing_bundle_settings_and_trust_are_not_reset() {
    let directory = UserDirectory::new();
    let paths = directory.paths();
    let mut config = AppConfig::load_or_create(&paths).expect("bootstrap");
    let current_exe = std::env::current_exe().expect("owned test executable");
    let runtime = nxtools_extensions::RuntimeIdentity::inspect(&current_exe)
        .expect("runtime fixture identity only");
    let hash = config.extensions.packages[0].content_hash.clone();
    config
        .extensions
        .approve(IDS[0], &hash, runtime)
        .expect("explicit fixture approval");
    config
        .extensions
        .set_enabled(IDS[0], true, &[])
        .expect("explicit fixture enable");
    config.extensions.packages[0]
        .settings
        .insert("third_party_opt_in".into(), serde_json::json!(true));
    config
        .save_to(&paths.config_file())
        .expect("configured catalog");
    let bytes = fs::read(paths.config_file()).expect("configured bytes");
    let reopened = AppConfig::load_or_create(&paths).expect("startup after user choices");
    assert_eq!(reopened.extensions, config.extensions);
    assert_eq!(
        fs::read(paths.config_file()).expect("choices not rewritten"),
        bytes
    );
}
