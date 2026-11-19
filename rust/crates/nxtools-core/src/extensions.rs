use crate::AppConfig;
use anyhow::Result;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Original Core config boundary composed with the host-owned bounded data owner.
#[derive(Clone)]
pub struct ConfigExtensionStorage {
    path: PathBuf,
    owner: nxtools_extensions::storage::StorageOwner,
}
impl ConfigExtensionStorage {
    pub fn new(path: PathBuf, owner: nxtools_extensions::storage::StorageOwner) -> Self {
        Self { path, owner }
    }
}
impl nxtools_extensions::storage::StorageHost for ConfigExtensionStorage {
    fn owner(&self) -> nxtools_extensions::storage::StorageOwner {
        self.owner.clone()
    }
    fn bind(
        &self,
        reviewed: &nxtools_extensions::InstalledPackage,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> std::result::Result<
        nxtools_extensions::InstalledPackage,
        nxtools_extensions::storage::StorageError,
    > {
        use nxtools_extensions::storage::StorageError;
        let result = AppConfig::with_config_lock(&self.path, || {
            anyhow::ensure!(!cancelled(), StorageError::Cancelled);
            let mut config = AppConfig::load_from(&self.path)?;
            let current = config
                .extensions
                .packages
                .iter()
                .find(|package| package.manifest.id == reviewed.manifest.id)
                .ok_or(StorageError::Stale)?;
            anyhow::ensure!(current == reviewed, StorageError::Stale);
            anyhow::ensure!(
                current.enabled
                    && current
                        .manifest
                        .runtime
                        .as_ref()
                        .is_some_and(|runtime| runtime.protocol_version == 2),
                StorageError::Stale
            );
            current.verify_approval().map_err(|_| StorageError::Stale)?;
            config
                .extensions
                .verify_runtime_selection(&current.manifest.id)
                .map_err(|_| StorageError::Stale)?;
            anyhow::ensure!(!cancelled(), StorageError::Cancelled);
            if let Some(namespace) = current.data_uuid {
                self.owner
                    .verify_namespace_blocking(namespace, &current.manifest.id)?;
                anyhow::ensure!(!cancelled(), StorageError::Cancelled);
                return Ok(current.clone());
            }
            let namespace = nxtools_extensions::InstallationDataId::new_v4();
            self.owner
                .reserve_namespace_blocking(namespace, &current.manifest.id)?;
            anyhow::ensure!(!cancelled(), StorageError::Cancelled);
            let package = config
                .extensions
                .packages
                .iter_mut()
                .find(|package| package.manifest.id == reviewed.manifest.id)
                .ok_or(StorageError::Stale)?;
            package.data_uuid = Some(namespace);
            let bound = package.clone();
            config.normalize();
            config.save_to(&self.path)?;
            tracing::info!(event = "extension.storage.identity_committed");
            Ok(bound)
        });
        result.map_err(|error| {
            error
                .downcast_ref::<StorageError>()
                .copied()
                .unwrap_or(StorageError::Io)
        })
    }
}

impl AppConfig {
    /// Reserve durable data metadata before publishing its pointer under the original lock.
    /// Compare the whole reviewed package; failed reservation/commit cannot expose data.
    pub fn allocate_extension_data_checked(
        path: &Path,
        reviewed: &nxtools_extensions::InstalledPackage,
        namespace: nxtools_extensions::InstallationDataId,
        storage: &nxtools_extensions::storage::StorageOwner,
    ) -> Result<Self> {
        anyhow::ensure!(!namespace.is_nil(), "invalid extension data identity");
        Self::try_update_section(path, |config| {
            let package = config
                .extensions
                .packages
                .iter_mut()
                .find(|package| package.manifest.id == reviewed.manifest.id)
                .ok_or_else(|| anyhow::anyhow!("extension data target unavailable"))?;
            anyhow::ensure!(
                package == reviewed,
                "extension data allocation review is stale"
            );
            anyhow::ensure!(
                package.data_uuid.is_none(),
                "extension data identity already allocated"
            );
            storage.reserve_namespace_blocking(namespace, &reviewed.manifest.id)?;
            package.data_uuid = Some(namespace);
            config.extensions.validate()?;
            Ok(())
        })
    }

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
            if current.as_deref() == Some(preview.content_hash.as_str()) {
                return Err(nxtools_extensions::PackageError::AlreadyInstalled.into());
            }
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

    /// Explicit pointer-switch restore. Never merge values or reactivate a retained UUID.
    /// Only call from the existing bounded management worker, not a GPUI/async reader.
    pub fn restore_extension_data_checked(
        path: &Path,
        reviewed: &nxtools_extensions::InstalledPackage,
        retained: &nxtools_extensions::storage::RetainedStorage,
        storage: &nxtools_extensions::storage::StorageOwner,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(Self, bool)> {
        use nxtools_extensions::storage::StorageError;
        Self::with_config_lock(path, || {
            anyhow::ensure!(!cancelled(), StorageError::Cancelled);
            let mut config = Self::load_from(path)?;
            let current = config
                .extensions
                .packages
                .iter()
                .find(|package| package.manifest.id == reviewed.manifest.id)
                .ok_or(StorageError::Stale)?;
            anyhow::ensure!(current == reviewed, StorageError::Stale);
            anyhow::ensure!(
                current
                    .manifest
                    .runtime
                    .as_ref()
                    .is_some_and(|runtime| runtime.protocol_version == 2),
                StorageError::Stale
            );
            current.verify_approval().map_err(|_| StorageError::Stale)?;
            config
                .extensions
                .verify_runtime_selection(&current.manifest.id)
                .map_err(|_| StorageError::Stale)?;
            storage.reconcile_current_registry_blocking(&config.extensions)?;
            let restored = nxtools_extensions::InstallationDataId::new_v4();
            storage.prepare_restore_blocking(
                &current.manifest.id,
                retained,
                current.data_uuid,
                restored,
            )?;
            let commit = (|| -> Result<()> {
                anyhow::ensure!(!cancelled(), StorageError::Cancelled);
                reviewed
                    .verify_approval()
                    .map_err(|_| StorageError::Stale)?;
                config
                    .extensions
                    .verify_runtime_selection(&reviewed.manifest.id)
                    .map_err(|_| StorageError::Stale)?;
                config
                    .extensions
                    .packages
                    .iter_mut()
                    .find(|package| package.manifest.id == reviewed.manifest.id)
                    .ok_or(StorageError::Stale)?
                    .data_uuid = Some(restored);
                config.normalize();
                anyhow::ensure!(!cancelled(), StorageError::Cancelled);
                config.save_to(path)
            })();
            if let Err(error) = commit {
                // Original lock is still held. Read durable config, never infer rollback
                // from an in-memory draft; a failed reconciliation keeps the intent.
                if let Ok(durable) = Self::load_from(path) {
                    let _ = storage.reconcile_current_registry_blocking(&durable.extensions);
                }
                return Err(error);
            }
            tracing::info!(event = "extension.storage.restore_pointer_committed");
            let pending = storage
                .reconcile_current_registry_blocking(&config.extensions)
                .unwrap_or_else(|_| {
                    tracing::warn!(
                        event = "extension.storage.cleanup_pending",
                        reason = "restore_postcommit_reconciliation_failed"
                    );
                    true
                });
            Ok((config, pending))
        })
    }

    /// Host management reads only a finite metadata page, under the same fresh config lock
    /// as destructive review. Reading never reconciles/deletes data or rewrites config.
    pub fn review_retained_extension_data_checked(
        path: &Path,
        reviewed: &nxtools_extensions::PackageRegistry,
        storage: &nxtools_extensions::storage::StorageOwner,
        package: Option<String>,
        after: Option<nxtools_extensions::InstallationDataId>,
    ) -> Result<nxtools_extensions::storage::RetainedStoragePage> {
        Self::with_config_lock(path, || {
            let config = Self::load_from(path)?;
            anyhow::ensure!(
                &config.extensions == reviewed,
                nxtools_extensions::storage::StorageError::Stale
            );
            Ok(storage.retained_page_blocking(package, after)?)
        })
    }

    /// Clear only the exact retained generation confirmed against a fresh registry. This
    /// never rewrites configuration, credentials, code staging or unrelated user data.
    pub fn clear_retained_extension_data_checked(
        path: &Path,
        id: &str,
        reviewed: &nxtools_extensions::PackageRegistry,
        retained: &nxtools_extensions::storage::RetainedStorage,
        storage: &nxtools_extensions::storage::StorageOwner,
    ) -> Result<bool> {
        Self::with_config_lock(path, || {
            let config = Self::load_from(path)?;
            anyhow::ensure!(
                &config.extensions == reviewed,
                nxtools_extensions::storage::StorageError::Stale
            );
            Ok(storage.clear_retained_blocking(id, retained, &config.extensions)?)
        })
    }

    /// Hold the original config lock throughout review -> durable retirement intent ->
    /// config commit -> fresh pointer reconciliation. Postcommit cleanup cannot undo removal.
    pub fn remove_extension_data_checked(
        path: &Path,
        id: &str,
        reviewed: &nxtools_extensions::PackageRegistry,
        storage: &nxtools_extensions::storage::StorageOwner,
        clear: bool,
    ) -> Result<(Self, bool)> {
        Self::with_config_lock(path, || {
            let mut config = Self::load_from(path)?;
            anyhow::ensure!(
                &config.extensions == reviewed,
                "extension removal review is stale"
            );
            let namespace = config
                .extensions
                .packages
                .iter()
                .find(|package| package.manifest.id == id)
                .ok_or_else(|| anyhow::anyhow!("extension removal target unavailable"))?
                .data_uuid;
            if let Some(namespace) = namespace {
                storage.prepare_retirement_blocking(namespace, id, clear)?;
            }
            config.extensions.remove(id)?;
            config.normalize();
            config.save_to(path)?;
            let cleanup_pending = storage
                .reconcile_current_registry_blocking(&config.extensions)
                .unwrap_or_else(|_| {
                    tracing::warn!(
                        event = "extension.storage.cleanup_pending",
                        reason = "postcommit_reconciliation_failed"
                    );
                    true
                });
            Ok((config, cleanup_pending))
        })
    }

    /// Explicit host retry of already authorized ClearPending/CleanupOnly obligations.
    /// Fresh registry review is mandatory; retained generations are never authorized here.
    pub fn retry_extension_storage_cleanup_checked(
        path: &Path,
        reviewed: &nxtools_extensions::PackageRegistry,
        storage: &nxtools_extensions::storage::StorageOwner,
    ) -> Result<bool> {
        Self::with_config_lock(path, || {
            let config = Self::load_from(path)?;
            anyhow::ensure!(
                &config.extensions == reviewed,
                nxtools_extensions::storage::StorageError::Stale
            );
            Ok(storage.reconcile_current_registry_blocking(&config.extensions)?)
        })
    }

    /// Startup/retry reconciliation never consumes a cached registry and never rewrites config.
    pub fn reconcile_extension_storage(
        path: &Path,
        storage: &nxtools_extensions::storage::StorageOwner,
    ) -> Result<bool> {
        Self::with_config_lock(path, || {
            let config = Self::load_from(path)?;
            Ok(storage.reconcile_current_registry_blocking(&config.extensions)?)
        })
    }

    /// Compare reviewed extension state inside the original locked reread. Other sections
    /// may change independently; a changed package/chain/settings review must be renewed.
    pub fn remove_extension_checked(
        path: &Path,
        id: &str,
        reviewed: &nxtools_extensions::PackageRegistry,
    ) -> Result<Self> {
        Self::try_update_section(path, |config| {
            anyhow::ensure!(
                &config.extensions == reviewed,
                "extension removal review is stale"
            );
            config.extensions.remove(id)?;
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
    fn remediation_reviewed_removal_rejects_changed_extension_config_but_preserves_other_sections()
    {
        let root = std::env::temp_dir().join(format!(
            "nxtools-removal-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("owned directory");
        let path = root.join("config.json");
        let mut config = AppConfig::default();
        config
            .extensions
            .install(
                nxtools_extensions::ImportPreview::from_json(PACKAGE).expect("package"),
                "fixture".into(),
                false,
            )
            .expect("install");
        config.save_to(&path).expect("save");
        let reviewed = config.extensions.clone();
        AppConfig::set_extension_aliases(&path, "example.web", "web", vec!["changed".into()])
            .expect("concurrent extension change");
        let changed = fs::read(&path).expect("changed bytes");
        let outcome = AppConfig::remove_extension_checked(&path, "example.web", &reviewed);
        assert!(
            outcome.is_err(),
            "stale reviewed removal must not delete newer configuration"
        );
        assert_eq!(fs::read(&path).expect("stale no write"), changed);
        let mut current = AppConfig::load_from(&path).expect("current");
        let reviewed = current.extensions.clone();
        current.system.start_on_login_silently = true;
        current.save_to(&path).expect("other section change");
        let removed = AppConfig::remove_extension_checked(&path, "example.web", &reviewed)
            .expect("current removal");
        assert!(removed.extensions.packages.is_empty());
        assert!(removed.system.start_on_login_silently);
        fs::remove_dir_all(root).expect("owned cleanup");
    }
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
    fn checked_import_repeated_package_preserves_enabled_config() {
        let root = std::env::temp_dir().join(format!(
            "nxtools-identical-import-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("owned root");
        let path = root.join("config.json");
        let packages = root.join("extensions");
        AppConfig::default().save_to(&path).expect("config");
        let preview = nxtools_extensions::ImportPreview::from_json(PACKAGE).expect("preview");
        let mut installed = AppConfig::import_extension_preview_checked(
            &path,
            preview.clone(),
            &packages,
            "original.zip".into(),
            None,
        )
        .expect("install");
        installed
            .extensions
            .set_enabled(&preview.manifest.id, true, &[])
            .expect("enable");
        installed.save_to(&path).expect("save enabled");
        let before = fs::read(&path).expect("config bytes");
        let reviewed = Some(preview.content_hash.clone());
        let _ = AppConfig::import_extension_preview_checked(
            &path,
            preview,
            &packages,
            "duplicate.zip".into(),
            reviewed,
        );
        let after = fs::read(&path).expect("config bytes");
        fs::remove_dir_all(root).expect("cleanup");
        assert!(
            after == before,
            "identical import must not reset the configuration"
        );
    }

    #[test]
    fn expansion_settings_locked_save_rechecks_readiness_and_preserves_disk_bytes() {
        let root = std::env::temp_dir().join(format!(
            "nxtools-p1-core-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        fs::create_dir(&root).expect("owned root");
        let path = root.join("config.json");
        let mut initial = AppConfig::default();
        initial.download.default_connection_limit = 23;
        initial.save_to(&path).expect("seed");
        let manifest = serde_json::json!({"schema_version":1,"id":"fixture.validation","name":"Validation","version":"1",
            "settings":[{"key":"name","title":"Name","type":"text","validation":{"required":true,"min_length":2}}],
            "handlers":[{"id":"web","title":"Web","feature":"tools","entry":"action","action":{"type":"website","url_template":"https://fixture.test"}}]});
        AppConfig::import_extension_json(
            &path,
            &serde_json::to_vec(&manifest).expect("manifest"),
            "owned".into(),
            false,
        )
        .expect("required-unconfigured import");
        let before = fs::read(&path).expect("before");
        for values in [
            std::collections::BTreeMap::new(),
            std::collections::BTreeMap::from([("name".into(), serde_json::json!("x"))]),
        ] {
            assert!(
                AppConfig::save_extension_settings(&path, "fixture.validation", values).is_err()
            );
            assert_eq!(fs::read(&path).expect("failed save unchanged"), before);
        }
        AppConfig::save_extension_settings(
            &path,
            "fixture.validation",
            std::collections::BTreeMap::from([("name".into(), serde_json::json!("ready"))]),
        )
        .expect("valid save");
        let current = AppConfig::load_from(&path).expect("reopen");
        assert_eq!(current.download.default_connection_limit, 23);
        assert_eq!(current.extensions.packages[0].settings["name"], "ready");
        fs::remove_dir_all(root).expect("owned cleanup");
    }

    #[test]
    fn expansion_storage_binding_rechecks_trust_and_never_recreates_missing_namespace() {
        use nxtools_extensions::storage::{StorageHost, StorageOwner};
        let root = std::env::temp_dir().join(format!(
            "nxtools-p3-binding-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir(&root).expect("owned root");
        let source = root.join("source");
        fs::create_dir(&source).expect("source");
        let manifest = serde_json::json!({"schema_version":1,"id":"fixture.binding","name":"Binding","version":"1","runtime":{"program":"python","protocol_version":2},"files":["entry.py"],"handlers":[{"id":"run","title":"Run","feature":"tools","entry":"action","action":{"type":"script","entry_file":"entry.py"}}]});
        fs::write(
            source.join("extension.json"),
            serde_json::to_vec(&manifest).expect("json"),
        )
        .expect("manifest");
        fs::write(source.join("entry.py"), b"not executed").expect("code");
        let executable = root.join("fixture.exe");
        fs::write(&executable, b"synthetic interpreter; never executed").expect("interpreter");
        let preview = nxtools_extensions::ImportPreview::from_directory(&source)
            .expect("preview")
            .stage_into(&root.join("packages"))
            .expect("stage");
        let hash = preview.content_hash.clone();
        let mut config = AppConfig::default();
        config.download.default_connection_limit = 23;
        config
            .extensions
            .install(preview, "owned".into(), false)
            .expect("install");
        let path = root.join("config.json");
        config.save_to(&path).expect("seed");
        let owner = StorageOwner::new(root.join("data")).expect("owner");
        let host = super::ConfigExtensionStorage::new(path.clone(), owner.clone());
        let reviewed = config.extensions.packages[0].clone();
        let before = fs::read(&path).expect("before");
        assert!(host.bind(&reviewed, &|| false).is_err());
        assert_eq!(fs::read(&path).expect("untrusted unchanged"), before);
        config
            .extensions
            .approve(
                "fixture.binding",
                &hash,
                nxtools_extensions::RuntimeIdentity::inspect(&executable).expect("identity"),
            )
            .expect("approve");
        config
            .extensions
            .set_enabled("fixture.binding", true, &[])
            .expect("enable");
        config.save_to(&path).expect("approved");
        let reviewed = config.extensions.packages[0].clone();
        assert!(host.bind(&reviewed, &|| true).is_err());
        let config_before = fs::read(&path).expect("before blocked binding");
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path.with_extension("json.lock"))
            .expect("original config lock");
        fs2::FileExt::lock_exclusive(&lock).expect("hold lock");
        let cancelled = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let task_cancelled = cancelled.clone();
        let task_host = host.clone();
        let task_reviewed = reviewed.clone();
        let (send, receive) = std::sync::mpsc::channel();
        let task = std::thread::spawn(move || {
            let result = task_host.bind(&task_reviewed, &|| {
                task_cancelled.load(std::sync::atomic::Ordering::Acquire)
            });
            send.send(result).expect("test receiver");
        });
        assert!(
            receive
                .recv_timeout(std::time::Duration::from_millis(30))
                .is_err(),
            "actual config lock must block the transaction"
        );
        cancelled.store(true, std::sync::atomic::Ordering::Release);
        fs2::FileExt::unlock(&lock).expect("release config lock");
        assert_eq!(
            receive
                .recv_timeout(std::time::Duration::from_secs(5))
                .expect("cancelled bind returns"),
            Err(nxtools_extensions::storage::StorageError::Cancelled)
        );
        task.join().expect("binding worker");
        assert_eq!(
            fs::read(&path).expect("cancel did not publish"),
            config_before
        );
        assert!(
            !root.join("data").exists(),
            "cancel before admission cannot allocate data"
        );
        let bound = host
            .bind(&reviewed, &|| false)
            .expect("lazy locked binding");
        assert!(bound.data_uuid.is_some());
        assert_eq!(
            AppConfig::load_from(&path)
                .expect("reopen")
                .download
                .default_connection_limit,
            23
        );
        assert_eq!(host.bind(&bound, &|| false).expect("same binding"), bound);
        assert!(
            host.bind(&reviewed, &|| false).is_err(),
            "old full review is stale"
        );
        fs::write(
            root.join("data").join("catalog.json"),
            br#"{"version":1,"namespaces":{},"pending":null}"#,
        )
        .expect("simulate lost namespace metadata");
        assert!(
            host.bind(&bound, &|| false).is_err(),
            "never recreate a missing referenced namespace"
        );
        drop(host);
        drop(owner);
        fs::remove_dir_all(root).expect("owned cleanup");
    }

    #[test]
    fn expansion_storage_data_identity_is_lazy_upgrade_stable_and_reimport_fresh() {
        let root = std::env::temp_dir().join(format!(
            "nxtools-p3-core-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir(&root).expect("owned root");
        let path = root.join("config.json");
        let mut initial = AppConfig::default();
        initial.download.default_connection_limit = 23;
        initial.save_to(&path).expect("seed");
        let imported = AppConfig::import_extension_json(&path, PACKAGE, "owned".into(), false)
            .expect("import");
        let before = fs::read(&path).expect("legacy bytes");
        assert_eq!(imported.extensions.packages[0].data_uuid, None);
        assert!(
            !String::from_utf8(before.clone())
                .expect("json")
                .contains("data_uuid")
        );
        AppConfig::load_from(&path).expect("side-effect-free load");
        assert_eq!(fs::read(&path).expect("load did not write"), before);
        let reviewed = imported.extensions.packages[0].clone();
        let namespace = nxtools_extensions::InstallationDataId::new_v4();
        let blocked = root.join("blocked-data");
        fs::write(&blocked, b"not a directory").expect("owned blocked root");
        let unavailable =
            nxtools_extensions::storage::StorageOwner::new(blocked).expect("lazy owner");
        assert!(
            AppConfig::allocate_extension_data_checked(&path, &reviewed, namespace, &unavailable)
                .is_err()
        );
        assert_eq!(
            fs::read(&path).expect("failed reservation did not publish pointer"),
            before
        );
        drop(unavailable);
        let owner =
            nxtools_extensions::storage::StorageOwner::new(root.join("data")).expect("owner");
        let allocated =
            AppConfig::allocate_extension_data_checked(&path, &reviewed, namespace, &owner)
                .expect("locked allocation after durable reservation");
        assert_eq!(allocated.extensions.packages[0].data_uuid, Some(namespace));
        assert_eq!(
            allocated.extensions.packages[0].content_hash,
            reviewed.content_hash
        );
        assert_eq!(allocated.download.default_connection_limit, 23);
        let allocated_bytes = fs::read(&path).expect("allocated bytes");
        assert!(
            AppConfig::allocate_extension_data_checked(
                &path,
                &reviewed,
                nxtools_extensions::InstallationDataId::new_v4(),
                &owner,
            )
            .is_err()
        );
        assert_eq!(
            fs::read(&path).expect("stale did not write"),
            allocated_bytes
        );
        let upgraded = AppConfig::import_extension_json(&path, PACKAGE, "upgrade".into(), true)
            .expect("explicit replacement");
        assert_eq!(upgraded.extensions.packages[0].data_uuid, Some(namespace));
        assert!(!upgraded.extensions.packages[0].enabled);
        AppConfig::remove_extension_checked(&path, "example.web", &upgraded.extensions)
            .expect("checked remove");
        let reimported = AppConfig::import_extension_json(&path, PACKAGE, "fresh".into(), false)
            .expect("reimport");
        assert_eq!(reimported.extensions.packages[0].data_uuid, None);
        let fresh = nxtools_extensions::InstallationDataId::new_v4();
        assert_ne!(fresh, namespace);
        let allocated = AppConfig::allocate_extension_data_checked(
            &path,
            &reimported.extensions.packages[0],
            fresh,
            &owner,
        )
        .expect("fresh allocation");
        assert_eq!(allocated.extensions.packages[0].data_uuid, Some(fresh));
        fs::remove_dir_all(root).expect("owned cleanup");
    }

    #[cfg(windows)]
    #[test]
    fn expansion_storage_remove_commit_failure_preserves_registration_and_default_retains() {
        use std::os::windows::fs::OpenOptionsExt as _;
        let root = std::env::temp_dir().join(format!(
            "nxtools-p3-remove-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ));
        fs::create_dir(&root).expect("owned root");
        let path = root.join("config.json");
        AppConfig::default().save_to(&path).expect("seed");
        let imported = AppConfig::import_extension_json(&path, PACKAGE, "owned".into(), false)
            .expect("import");
        let owner =
            nxtools_extensions::storage::StorageOwner::new(root.join("data")).expect("owner");
        let namespace = nxtools_extensions::InstallationDataId::new_v4();
        owner
            .reserve_namespace_blocking(namespace, "example.web")
            .expect("durable reserve");
        let allocated = AppConfig::allocate_extension_data_checked(
            &path,
            &imported.extensions.packages[0],
            namespace,
            &owner,
        )
        .expect("allocate");
        let before = fs::read(&path).expect("before");
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .open(&path)
            .expect("deny config rename");
        assert!(
            AppConfig::remove_extension_data_checked(
                &path,
                "example.web",
                &allocated.extensions,
                &owner,
                true
            )
            .is_err()
        );
        assert_eq!(fs::read(&path).expect("config retained"), before);
        assert!(
            owner
                .retained_metadata_blocking("example.web")
                .expect("not removed")
                .is_empty()
        );
        drop(held);
        let (removed, cleanup_pending) = AppConfig::remove_extension_data_checked(
            &path,
            "example.web",
            &allocated.extensions,
            &owner,
            false,
        )
        .expect("default retains after actual commit");
        assert!(removed.extensions.packages.is_empty());
        assert!(!cleanup_pending);
        assert_eq!(
            owner
                .retained_metadata_blocking("example.web")
                .expect("retained")
                .len(),
            1
        );
        fs::remove_dir_all(root).expect("owned cleanup");
    }

    #[test]
    fn w6_legacy_config_defaults_to_no_extensions() {
        let mut json = serde_json::to_value(AppConfig::default()).expect("old configuration");
        json.as_object_mut().expect("object").remove("extensions");
        let restored: AppConfig = serde_json::from_value(json).expect("legacy decode");
        assert!(restored.extensions.packages.is_empty());
    }
}
