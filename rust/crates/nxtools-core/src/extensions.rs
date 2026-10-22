use crate::AppConfig;
use anyhow::Result;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

impl AppConfig {
    pub fn save_extension_runtime_defaults(
        path: &Path,
        defaults: BTreeMap<String, PathBuf>,
    ) -> Result<Self> {
        Self::try_update_section(path, move |config| {
            config.extensions.set_runtime_defaults(defaults)?;
            Ok(())
        })
    }

    pub fn set_extension_runtime_override(
        path: &Path,
        id: &str,
        executable: Option<PathBuf>,
    ) -> Result<Self> {
        Self::try_update_section(path, move |config| {
            config.extensions.set_runtime_override(id, executable)?;
            Ok(())
        })
    }

    pub fn save_extension_resolver_chain(
        path: &Path,
        host: String,
        route: Vec<nxtools_extensions::HandlerReference>,
    ) -> Result<Self> {
        Self::try_update_section(path, move |config| {
            config.extensions.set_resolver_chain(host, route)?;
            Ok(())
        })
    }
    pub fn set_extension_event(
        path: &Path,
        id: &str,
        handler: &str,
        enabled: bool,
    ) -> Result<Self> {
        Self::try_update_section(path, |config| {
            config.extensions.set_event(id, handler, enabled)?;
            Ok(())
        })
    }
    pub fn set_extension_provider(
        path: &Path,
        id: &str,
        handler: &str,
        enabled: bool,
    ) -> Result<Self> {
        Self::try_update_section(path, |config| {
            config.extensions.set_provider(id, handler, enabled)?;
            Ok(())
        })
    }

    pub fn import_extension_directory(
        path: &Path,
        directory: &Path,
        packages_root: &Path,
        replace: bool,
    ) -> Result<Self> {
        let preview = nxtools_extensions::ImportPreview::from_directory(directory)?;
        Self::import_extension_preview(
            path,
            preview,
            packages_root,
            directory.to_string_lossy().into_owned(),
            replace,
        )
    }

    pub fn import_extension_preview(
        path: &Path,
        preview: nxtools_extensions::ImportPreview,
        packages_root: &Path,
        source: String,
        replace: bool,
    ) -> Result<Self> {
        Self::try_update_section(path, move |config| {
            // Stage only after the locked reread accepted the existing config.
            // A published hash may already be shared by another importer/config;
            // never delete it on commit failure without an exclusive stage lease.
            let staged = preview.stage_into(packages_root)?;
            config.extensions.install(staged, source, replace)?;
            Ok(())
        })
    }

    /// Compare the reviewed installed identity inside the same locked reread as installation.
    pub fn import_extension_preview_checked(
        path: &Path,
        preview: nxtools_extensions::ImportPreview,
        packages_root: &Path,
        source: String,
        expected_previous: Option<String>,
    ) -> Result<Self> {
        Self::try_update_section(path, move |config| {
            let current = config
                .extensions
                .packages
                .iter()
                .find(|package| package.manifest.id == preview.manifest.id)
                .map(|package| package.content_hash.clone());
            anyhow::ensure!(
                current == expected_previous,
                "extension import candidate superseded"
            );
            let staged = preview.stage_into(packages_root)?;
            config
                .extensions
                .install(staged, source, expected_previous.is_some())?;
            Ok(())
        })
    }

    pub fn approve_extension(
        path: &Path,
        id: &str,
        content_hash: &str,
        runtime: nxtools_extensions::RuntimeIdentity,
    ) -> Result<Self> {
        Self::try_update_section(path, move |config| {
            config.extensions.approve(id, content_hash, runtime)?;
            Ok(())
        })
    }
    pub fn approve_extension_checked(
        path: &Path,
        id: &str,
        content_hash: &str,
        runtime: nxtools_extensions::RuntimeIdentity,
        expected_selected: Option<PathBuf>,
    ) -> Result<Self> {
        Self::try_update_section(path, move |config| {
            anyhow::ensure!(
                config.extensions.runtime_executable(id)? == expected_selected,
                "runtime selection changed during approval"
            );
            config.extensions.approve(id, content_hash, runtime)?;
            Ok(())
        })
    }

    pub fn revoke_extension(path: &Path, id: &str) -> Result<Self> {
        Self::try_update_section(path, |config| {
            config.extensions.revoke(id)?;
            Ok(())
        })
    }

    pub fn save_extension_settings(
        path: &Path,
        id: &str,
        settings: std::collections::BTreeMap<String, serde_json::Value>,
    ) -> Result<Self> {
        Self::try_update_section(path, move |config| {
            config.extensions.set_settings(id, settings)?;
            Ok(())
        })
    }
    pub fn import_extension_json(
        path: &Path,
        bytes: &[u8],
        source: String,
        replace: bool,
    ) -> Result<Self> {
        let preview = nxtools_extensions::ImportPreview::from_json(bytes)?;
        Self::try_update_section(path, move |config| {
            config.extensions.install(preview, source, replace)?;
            Ok(())
        })
    }

    pub fn set_extension_enabled(path: &Path, id: &str, enabled: bool) -> Result<Self> {
        Self::try_update_section(path, |config| {
            let manual = config
                .quick_launch
                .custom_commands
                .iter()
                .filter(|command| command.enabled)
                .map(|command| command.keyword.clone())
                .collect::<Vec<_>>();
            config.extensions.set_enabled(id, enabled, &manual)?;
            Ok(())
        })
    }

    pub fn set_extension_aliases(
        path: &Path,
        id: &str,
        handler: &str,
        aliases: Vec<String>,
    ) -> Result<Self> {
        Self::try_update_section(path, |config| {
            let manual = config
                .quick_launch
                .custom_commands
                .iter()
                .filter(|command| command.enabled)
                .map(|command| command.keyword.clone())
                .collect::<Vec<_>>();
            config
                .extensions
                .set_aliases(id, handler, aliases, &manual)?;
            Ok(())
        })
    }

    pub fn remove_extension(path: &Path, id: &str) -> Result<Self> {
        Self::try_update_section(path, |config| {
            config.extensions.remove(id)?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CustomCommandSettings, DownloadSettings};
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    const PACKAGE: &[u8] = br#"{"schema_version":1,"id":"example.web","name":"Web","version":"1.0.0","handlers":[{"id":"web","title":"Web","feature":"quick_launch","entry":"command","keywords":["web"],"action":{"type":"website","url_template":"https://example.test/?q={query}"}}]}"#;

    #[test]
    fn w6_import_enable_reopen_preserves_other_sections_and_failed_update_bytes() {
        let root = std::env::temp_dir().join(format!(
            "nxtools-w6-core-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp directory");
        let path = root.join("config.json");
        let mut initial = AppConfig {
            download: DownloadSettings {
                default_connection_limit: 24,
                ..Default::default()
            },
            ..Default::default()
        };
        initial
            .quick_launch
            .custom_commands
            .push(CustomCommandSettings {
                keyword: "manual".into(),
                program: "manual.exe".into(),
                ..Default::default()
            });
        initial.save_to(&path).expect("seed config");
        let imported =
            AppConfig::import_extension_json(&path, PACKAGE, "fixture.json".into(), false)
                .expect("import");
        assert!(!imported.extensions.packages[0].enabled);
        assert_eq!(imported.download.default_connection_limit, 24);
        assert_eq!(imported.quick_launch.custom_commands[0].keyword, "manual");
        AppConfig::set_extension_enabled(&path, "example.web", true).expect("enable");
        assert!(
            AppConfig::load_from(&path)
                .expect("reopen")
                .extensions
                .packages[0]
                .enabled
        );
        let good = fs::read(&path).expect("prior bytes");
        assert!(
            AppConfig::import_extension_json(&path, b"broken", "fixture".into(), true).is_err()
        );
        assert_eq!(fs::read(&path).expect("failed update bytes"), good);
        AppConfig::set_extension_enabled(&path, "example.web", false).expect("disable");
        let mut current = AppConfig::load_from(&path).expect("latest");
        current.quick_launch.custom_commands[0].keyword = "WEB".into();
        current.save_to(&path).expect("new manual keyword");
        let before_conflict = fs::read(&path).expect("conflict bytes");
        assert!(AppConfig::set_extension_enabled(&path, "example.web", true).is_err());
        assert_eq!(fs::read(&path).expect("after conflict"), before_conflict);
        AppConfig::set_extension_aliases(&path, "example.web", "web", vec!["lookup".into()])
            .expect("resolve keyword");
        AppConfig::set_extension_enabled(&path, "example.web", true).expect("enable alias");
        let restored = AppConfig::load_from(&path).expect("alias reload");
        assert_eq!(restored.extensions.packages[0].aliases["web"], ["lookup"]);
        let mut conflicting = restored.quick_launch.clone();
        conflicting.custom_commands[0].keyword = "LOOKUP".into();
        let before_manual = fs::read(&path).expect("before manual conflict");
        assert!(
            AppConfig::update_quick_launch_settings(&path, conflicting).is_err(),
            "manual rename cannot steal package keyword"
        );
        assert_eq!(
            fs::read(&path).expect("manual conflict bytes"),
            before_manual
        );
        let good = fs::read(&path).expect("before corrupt");
        fs::write(&path, b"broken").expect("corrupt fixture");
        assert!(AppConfig::set_extension_enabled(&path, "example.web", false).is_err());
        assert_eq!(fs::read(&path).expect("unchanged corrupt"), b"broken");
        fs::write(&path, good).expect("restore fixture");
        AppConfig::remove_extension(&path, "example.web").expect("remove");
        assert!(
            AppConfig::load_from(&path)
                .expect("removed reload")
                .extensions
                .packages
                .is_empty()
        );
        fs::remove_dir_all(root).expect("remove fixture");
    }

    #[test]
    fn w7_staged_import_trust_settings_and_revoke_are_atomic_and_preserve_other_sections() {
        let root = std::env::temp_dir().join(format!(
            "nxtools-w7-core-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let source = root.join("source");
        fs::create_dir_all(&source).expect("fixture source");
        let manifest = br#"{"schema_version":1,"id":"example.script","name":"Script","version":"1","runtime":{"program":"python"},"files":["entry.py"],"settings":[{"key":"message","title":"Message","type":"text","default":"initial"},{"key":"token","title":"Token","type":"password"}],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]}"#;
        fs::write(source.join("extension.json"), manifest).expect("manifest");
        fs::write(source.join("entry.py"), b"fixture code; never executed").expect("code");
        let runtime_file = root.join("runtime.exe");
        fs::write(&runtime_file, b"fixture runtime; never executed").expect("runtime");
        let runtime =
            nxtools_extensions::RuntimeIdentity::inspect(&runtime_file).expect("runtime identity");
        let path = root.join("config.json");
        let initial = AppConfig {
            download: DownloadSettings {
                default_connection_limit: 19,
                ..Default::default()
            },
            ..Default::default()
        };
        initial.save_to(&path).expect("seed config");
        let imported =
            AppConfig::import_extension_directory(&path, &source, &root.join("packages"), false)
                .expect("staged import");
        let hash = imported.extensions.packages[0].content_hash.clone();
        assert!(
            imported.extensions.packages[0]
                .code_directory
                .as_ref()
                .expect("staged path")
                .starts_with(fs::canonicalize(root.join("packages")).expect("packages"))
        );
        assert_eq!(imported.download.default_connection_limit, 19);
        AppConfig::approve_extension(&path, "example.script", &hash, runtime.clone())
            .expect("approval");
        let approved = AppConfig::load_from(&path).expect("reopen");
        approved.extensions.packages[0]
            .verify_approval()
            .expect("persisted approval");
        let before = fs::read(&path).expect("before failed settings");
        let mut settings = approved.extensions.packages[0].settings.clone();
        settings.insert(
            "token".into(),
            serde_json::Value::String("do-not-persist".into()),
        );
        assert!(AppConfig::save_extension_settings(&path, "example.script", settings).is_err());
        assert_eq!(fs::read(&path).expect("after failed settings"), before);
        let enabled = AppConfig::set_extension_enabled(&path, "example.script", true)
            .expect("explicit enable after approval");
        assert!(enabled.extensions.packages[0].enabled);
        assert!(
            AppConfig::load_from(&path)
                .expect("enabled config reopen")
                .extensions
                .packages[0]
                .enabled
        );
        let mut settings = approved.extensions.packages[0].settings.clone();
        settings.insert("message".into(), serde_json::Value::String("saved".into()));
        AppConfig::save_extension_settings(&path, "example.script", settings)
            .expect("nonsecret save");
        assert_eq!(
            AppConfig::load_from(&path)
                .expect("settings reload")
                .extensions
                .packages[0]
                .settings["message"],
            "saved"
        );
        let revoked = AppConfig::revoke_extension(&path, "example.script").expect("revoke");
        assert!(revoked.extensions.packages[0].trust.is_none());
        assert!(!revoked.extensions.packages[0].enabled);
        assert_eq!(revoked.download.default_connection_limit, 19);
        fs::remove_dir_all(root).expect("remove owned fixture");
    }

    #[test]
    fn runtime_configuration_reopens_preserves_other_owners_and_rejects_without_writing() {
        let root = std::env::temp_dir().join(format!(
            "nxtools-runtime-config-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        let source = root.join("source");
        fs::create_dir_all(&source).expect("source");
        fs::write(source.join("extension.json"), br#"{"schema_version":1,"id":"example.runtime","name":"Runtime","version":"1","runtime":{"program":"python"},"files":["entry.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]}"#).expect("manifest");
        fs::write(source.join("entry.py"), b"never executed").expect("code");
        let path = root.join("config.json");
        AppConfig::default().save_to(&path).expect("seed");
        AppConfig::import_extension_directory(&path, &source, &root.join("packages"), false)
            .expect("import");
        let mut other_owner = AppConfig::load_from(&path).expect("other owner");
        other_owner.download.default_connection_limit = 23;
        other_owner
            .quick_launch
            .custom_commands
            .push(CustomCommandSettings {
                keyword: "keep".into(),
                program: "manual.exe".into(),
                ..Default::default()
            });
        other_owner.save_to(&path).expect("other owner writes");
        let inherited = root.join("missing-python.exe");
        let pinned = root.join("missing-pinned.exe");
        AppConfig::save_extension_runtime_defaults(
            &path,
            BTreeMap::from([("python".into(), inherited.clone())]),
        )
        .expect("save missing runtime configuration");
        AppConfig::set_extension_runtime_override(&path, "example.runtime", Some(pinned.clone()))
            .expect("pin");
        let reopened = AppConfig::load_from(&path).expect("reopen");
        assert_eq!(reopened.extensions.runtime_defaults["python"], inherited);
        assert_eq!(
            reopened
                .extensions
                .runtime_executable("example.runtime")
                .expect("selected"),
            Some(pinned)
        );
        assert_eq!(reopened.download.default_connection_limit, 23);
        assert_eq!(reopened.quick_launch.custom_commands[0].keyword, "keep");
        let before = fs::read(&path).expect("before invalid");
        assert!(
            AppConfig::save_extension_runtime_defaults(
                &path,
                BTreeMap::from([("exe".into(), root.join("invalid.exe"))])
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).expect("after invalid default"), before);
        assert!(
            AppConfig::set_extension_runtime_override(
                &path,
                "example.runtime",
                Some("relative.exe".into())
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).expect("after invalid override"), before);
        assert!(AppConfig::set_extension_runtime_override(&path, "not.installed", None).is_err());
        assert_eq!(fs::read(&path).expect("after missing package"), before);
        AppConfig::set_extension_runtime_override(&path, "example.runtime", None).expect("inherit");
        assert_eq!(
            AppConfig::load_from(&path)
                .expect("inherit reopen")
                .extensions
                .runtime_executable("example.runtime")
                .expect("inherited"),
            Some(inherited)
        );
        let selected = root.join("selected.exe");
        fs::write(&selected, b"synthetic runtime, never executed").expect("runtime");
        let identity = nxtools_extensions::RuntimeIdentity::inspect(&selected).expect("identity");
        let saved = AppConfig::save_extension_runtime_defaults(
            &path,
            BTreeMap::from([("python".into(), selected)]),
        )
        .expect("installed default");
        let hash = saved.extensions.packages[0].content_hash.clone();
        let current_selection = saved
            .extensions
            .runtime_executable("example.runtime")
            .expect("selection");
        let before_approval = fs::read(&path).expect("before stale approval");
        assert!(
            AppConfig::approve_extension_checked(
                &path,
                "example.runtime",
                &hash,
                identity.clone(),
                None
            )
            .is_err()
        );
        assert_eq!(
            fs::read(&path).expect("no stale trust write"),
            before_approval
        );
        AppConfig::approve_extension_checked(
            &path,
            "example.runtime",
            &hash,
            identity,
            current_selection,
        )
        .expect("approve unchanged selection");
        AppConfig::set_extension_enabled(&path, "example.runtime", true).expect("enable");
        let changed = AppConfig::save_extension_runtime_defaults(
            &path,
            BTreeMap::from([("python".into(), root.join("changed.exe"))]),
        )
        .expect("change default");
        assert!(changed.extensions.packages[0].trust.is_none());
        assert!(!changed.extensions.packages[0].enabled);
        let reopened = AppConfig::load_from(&path).expect("revocation reopen");
        assert!(reopened.extensions.packages[0].trust.is_none());
        assert!(!reopened.extensions.packages[0].enabled);
        assert_eq!(reopened.download.default_connection_limit, 23);
        assert_eq!(reopened.quick_launch.custom_commands[0].keyword, "keep");
        fs::write(&path, b"broken").expect("corrupt");
        assert!(AppConfig::save_extension_runtime_defaults(&path, BTreeMap::new()).is_err());
        assert_eq!(fs::read(&path).expect("corrupt preserved"), b"broken");
        fs::remove_file(&path).expect("remove config");
        assert!(AppConfig::save_extension_runtime_defaults(&path, BTreeMap::new()).is_err());
        assert!(!path.exists());
        fs::remove_dir_all(root).expect("owned fixture cleanup");
    }

    // Stored ZIP fixtures are written directly so Core needs no production ZIP dependency.
    fn zip_fixture(path: &Path, manifest: &[u8], code: Option<&[u8]>) {
        fn crc(bytes: &[u8]) -> u32 {
            let mut crc = !0u32;
            for byte in bytes {
                crc ^= u32::from(*byte);
                for _ in 0..8 {
                    crc = (crc >> 1) ^ (0xedb88320 & 0u32.wrapping_sub(crc & 1));
                }
            }
            !crc
        }
        let mut entries = vec![("extension.json", manifest)];
        if let Some(code) = code {
            entries.push(("entry.py", code));
        }
        let mut bytes = Vec::new();
        let mut central = Vec::new();
        for (name, data) in &entries {
            let offset = bytes.len() as u32;
            bytes.extend(0x04034b50u32.to_le_bytes());
            for v in [20u16, 0x800, 0, 0, 0] {
                bytes.extend(v.to_le_bytes());
            }
            for v in [crc(data), data.len() as u32, data.len() as u32] {
                bytes.extend(v.to_le_bytes());
            }
            bytes.extend((name.len() as u16).to_le_bytes());
            bytes.extend(0u16.to_le_bytes());
            bytes.extend(name.as_bytes());
            bytes.extend(*data);
            central.extend(0x02014b50u32.to_le_bytes());
            for v in [0x314u16, 20, 0x800, 0, 0, 0] {
                central.extend(v.to_le_bytes());
            }
            for v in [crc(data), data.len() as u32, data.len() as u32] {
                central.extend(v.to_le_bytes());
            }
            for v in [name.len() as u16, 0, 0, 0, 0] {
                central.extend(v.to_le_bytes());
            }
            central.extend((0o100644u32 << 16).to_le_bytes());
            central.extend(offset.to_le_bytes());
            central.extend(name.as_bytes());
        }
        let offset = bytes.len() as u32;
        let size = central.len() as u32;
        bytes.extend(central);
        bytes.extend(0x06054b50u32.to_le_bytes());
        for v in [0u16, 0, entries.len() as u16, entries.len() as u16] {
            bytes.extend(v.to_le_bytes());
        }
        bytes.extend(size.to_le_bytes());
        bytes.extend(offset.to_le_bytes());
        bytes.extend(0u16.to_le_bytes());
        fs::write(path, bytes).expect("owned ZIP fixture");
    }

    #[test]
    fn zip_preview_confirm_reopen_source_removal_and_failed_replacement_keep_last_good() {
        let root = std::env::temp_dir().join(format!(
            "nxtools-zip-core-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("owned fixture");
        let path = root.join("config.json");
        let zip = root.join("source.zip");
        let packages = root.join("extensions");
        AppConfig {
            download: DownloadSettings {
                default_connection_limit: 21,
                ..Default::default()
            },
            ..Default::default()
        }
        .save_to(&path)
        .expect("seed");
        let original = fs::read(&path).expect("original bytes");
        zip_fixture(&zip, PACKAGE, None);
        let preview = nxtools_extensions::ImportPreview::from_archive(&zip).expect("ZIP preview");
        assert!(!packages.exists(), "preview/cancel has no live snapshot");
        assert_eq!(fs::read(&path).expect("preview config"), original);
        fs::remove_file(&zip).expect("source gone before confirm");
        let imported = AppConfig::import_extension_preview(
            &path,
            preview,
            &packages,
            "owned.zip".into(),
            false,
        )
        .expect("confirm");
        let snapshot = imported.extensions.packages[0]
            .code_directory
            .clone()
            .expect("managed manifest");
        let manifest_bytes = fs::read(snapshot.join("extension.json")).expect("stored manifest");
        let enabled = AppConfig::set_extension_enabled(&path, "example.web", true)
            .expect("ordinary explicit enable");
        let prior = fs::read(&path).expect("last good config");
        let reopened = AppConfig::load_from(&path).expect("reopen");
        assert_eq!(reopened.extensions, enabled.extensions);
        assert_eq!(reopened.download.default_connection_limit, 21);
        zip_fixture(
            &zip,
            &String::from_utf8_lossy(PACKAGE)
                .replace("1.0.0", "2.0.0")
                .into_bytes(),
            None,
        );
        let replacement =
            nxtools_extensions::ImportPreview::from_archive(&zip).expect("replacement");
        assert!(
            AppConfig::import_extension_preview(
                &path,
                replacement.clone(),
                &packages,
                "owned.zip".into(),
                false
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).expect("failed replace config"), prior);
        assert_eq!(
            fs::read(snapshot.join("extension.json")).expect("last good snapshot"),
            manifest_bytes
        );
        // An owned read handle without FILE_SHARE_DELETE makes the actual final
        // config rename fail on Windows, after registry validation and staging.
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let blocker = fs::OpenOptions::new()
                .read(true)
                .share_mode(3)
                .open(&path)
                .expect("block config replacement");
            assert!(
                AppConfig::import_extension_preview(
                    &path,
                    replacement,
                    &packages,
                    "owned.zip".into(),
                    true
                )
                .is_err()
            );
            drop(blocker);
            assert_eq!(
                fs::read(&path).expect("commit failure preserves bytes"),
                prior
            );
            assert_eq!(
                AppConfig::load_from(&path)
                    .expect("failed commit reopen")
                    .extensions,
                enabled.extensions
            );
        }
        assert_eq!(
            fs::read(snapshot.join("extension.json")).expect("old disk retained"),
            manifest_bytes
        );
        fs::write(&zip, b"invalid archive").expect("invalid fixture");
        assert!(nxtools_extensions::ImportPreview::from_archive(&zip).is_err());
        assert_eq!(fs::read(&path).expect("invalid no mutation"), prior);
        let script = br#"{"schema_version":1,"id":"example.zip-script","name":"ZIP Script","version":"1","runtime":{"program":"python"},"files":["entry.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]}"#;
        zip_fixture(&zip, script, Some(b"synthetic never executed"));
        let preview =
            nxtools_extensions::ImportPreview::from_archive(&zip).expect("script preview");
        let installed = AppConfig::import_extension_preview(
            &path,
            preview,
            &packages,
            "owned.zip".into(),
            false,
        )
        .expect("script confirm");
        fs::remove_file(&zip).expect("delete original archive");
        let reopened = AppConfig::load_from(&path).expect("script reopen");
        assert_eq!(reopened.extensions, installed.extensions);
        let script = &reopened.extensions.packages[1];
        script
            .verify_current_code()
            .expect("snapshot independent of archive");
        assert!(!script.enabled && script.trust.is_none());
        assert_eq!(
            fs::read(
                script
                    .code_directory
                    .as_ref()
                    .expect("script snapshot")
                    .join("entry.py")
            )
            .expect("installed code"),
            b"synthetic never executed"
        );
        fs::remove_dir_all(root).expect("owned cleanup");
    }

    #[test]
    fn checked_import_compares_fresh_disk_identity_before_staging() {
        let root = std::env::temp_dir().join(format!(
            "nxtools-checked-import-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("owned");
        let path = root.join("config.json");
        let packages = root.join("extensions");
        AppConfig::default().save_to(&path).expect("config");
        let first = nxtools_extensions::ImportPreview::from_json(PACKAGE).expect("first");
        let reviewed = first.content_hash.clone();
        AppConfig::import_extension_preview_checked(
            &path,
            first.clone(),
            &packages,
            "first.zip".into(),
            None,
        )
        .expect("first install");
        let changed = String::from_utf8(PACKAGE.to_vec())
            .expect("UTF8")
            .replace("1.0.0", "2.0.0");
        let newer =
            nxtools_extensions::ImportPreview::from_json(changed.as_bytes()).expect("newer");
        let newer_hash = newer.content_hash.clone();
        AppConfig::import_extension_preview(&path, newer, &packages, "other writer".into(), true)
            .expect("other writer");
        let bytes = fs::read(&path).expect("current bytes");
        assert!(
            AppConfig::import_extension_preview_checked(
                &path,
                first.clone(),
                &packages,
                "stale.zip".into(),
                Some(reviewed)
            )
            .is_err()
        );
        assert!(
            AppConfig::import_extension_preview_checked(
                &path,
                first,
                &packages,
                "stale absent.zip".into(),
                None
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).expect("unchanged"), bytes);
        assert_eq!(
            AppConfig::load_from(&path)
                .expect("reopen")
                .extensions
                .packages[0]
                .content_hash,
            newer_hash
        );
        fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn w6_legacy_config_defaults_to_no_extensions() {
        let mut json = serde_json::to_value(AppConfig::default()).expect("old configuration");
        json.as_object_mut().expect("object").remove("extensions");
        let restored: AppConfig = serde_json::from_value(json).expect("legacy decode");
        assert!(restored.extensions.packages.is_empty());
    }
}
