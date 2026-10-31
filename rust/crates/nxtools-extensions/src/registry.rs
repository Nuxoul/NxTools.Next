use crate::{Handler, ImportPreview, MAX_PACKAGES, Manifest, PackageError, valid_text};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstalledPackage {
    pub manifest: Manifest,
    pub content_hash: String,
    pub source: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub aliases: BTreeMap<String, Vec<String>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub file_hashes: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_directory: Option<std::path::PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trust: Option<crate::TrustApproval>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_override: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub settings: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub enabled_events: HashSet<String>,
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub enabled_providers: HashSet<String>,
    /// Host-owned installation data identity; never part of manifest/code authorization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data_uuid: Option<uuid::Uuid>,
}

impl InstalledPackage {
    pub fn keywords<'a>(&'a self, handler: &'a Handler) -> &'a [String] {
        self.aliases
            .get(&handler.id)
            .map_or(handler.keywords.as_slice(), Vec::as_slice)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageRegistry {
    #[serde(default)]
    pub packages: Vec<InstalledPackage>,
    /// Once seeded, an empty catalog is an explicit user choice, not a request to reseed.
    #[serde(default)]
    pub bundled_defaults_initialized: bool,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resolver_preferences: BTreeMap<String, crate::HandlerReference>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub resolver_chains: BTreeMap<String, Vec<crate::HandlerReference>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub runtime_defaults: BTreeMap<String, PathBuf>,
}

impl PackageRegistry {
    /// Resolve configuration without filesystem probing. Legacy approvals remain a
    /// fallback only while neither an override nor a program default is configured.
    pub fn runtime_executable(&self, id: &str) -> Result<Option<PathBuf>, PackageError> {
        self.validate()?;
        let package = self
            .packages
            .iter()
            .find(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        self.selected_runtime(package)
    }

    fn selected_runtime(
        &self,
        package: &InstalledPackage,
    ) -> Result<Option<PathBuf>, PackageError> {
        let Some(runtime) = &package.manifest.runtime else {
            return Ok(None);
        };
        if runtime.program == crate::ScriptProgram::Exe {
            let root = package
                .code_directory
                .as_ref()
                .ok_or(PackageError::CodeRequired)?;
            let mut selected = None;
            for handler in &package.manifest.handlers {
                if let crate::DeclarativeAction::Script { entry_file } = &handler.action {
                    let entry = root.join(entry_file.replace('\\', "/"));
                    if selected.as_ref().is_some_and(|previous| previous != &entry) {
                        return Err(PackageError::InvalidManifest);
                    }
                    selected = Some(entry);
                }
            }
            return selected.ok_or(PackageError::InvalidManifest).map(Some);
        }
        Ok(package
            .runtime_override
            .as_ref()
            .or_else(|| self.runtime_defaults.get(runtime.program.key()))
            .cloned()
            .or_else(|| {
                package
                    .trust
                    .as_ref()
                    .map(|approval| approval.runtime.executable.clone())
            }))
    }

    pub fn set_runtime_defaults(
        &mut self,
        defaults: BTreeMap<String, PathBuf>,
    ) -> Result<(), PackageError> {
        self.validate()?;
        let mut candidate = self.clone();
        candidate.runtime_defaults = defaults;
        candidate.validate()?;
        self.publish_runtime_configuration(candidate)
    }

    pub fn set_runtime_override(
        &mut self,
        id: &str,
        executable: Option<PathBuf>,
    ) -> Result<(), PackageError> {
        self.validate()?;
        let mut candidate = self.clone();
        let package = candidate
            .packages
            .iter_mut()
            .find(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        package.runtime_override = executable;
        candidate.validate()?;
        self.publish_runtime_configuration(candidate)
    }

    fn publish_runtime_configuration(&mut self, mut candidate: Self) -> Result<(), PackageError> {
        for package in &self.packages {
            let next = candidate
                .packages
                .iter()
                .find(|next| next.manifest.id == package.manifest.id)
                .ok_or(PackageError::NotInstalled)?;
            if !same_runtime_selection(
                self.selected_runtime(package)?.as_deref(),
                candidate.selected_runtime(next)?.as_deref(),
            ) {
                candidate.revoke(&package.manifest.id)?;
            }
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    /// Filesystem identity verification for background enable/execute/admission owners,
    /// never rendering or ordinary settings Save.
    pub fn verify_runtime_selection(&self, id: &str) -> Result<(), PackageError> {
        let package = self
            .packages
            .iter()
            .find(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        let approval = package.trust.as_ref().ok_or(PackageError::Untrusted)?;
        let selected = self
            .selected_runtime(package)?
            .ok_or(PackageError::Untrusted)?;
        crate::trust::verify_selected_path(&selected, &approval.runtime.executable)
    }

    pub fn approve(
        &mut self,
        id: &str,
        expected_content_hash: &str,
        runtime: crate::RuntimeIdentity,
    ) -> Result<(), PackageError> {
        self.validate()?;
        let mut candidate = self.clone();
        let index = candidate
            .packages
            .iter()
            .position(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        let package = &candidate.packages[index];
        let program = package
            .manifest
            .runtime
            .as_ref()
            .ok_or(PackageError::Untrusted)?
            .program;
        if package.content_hash != expected_content_hash {
            return Err(PackageError::Untrusted);
        }
        package.verify_current_code()?;
        runtime.verify()?;
        let configured = program == crate::ScriptProgram::Exe
            || package.runtime_override.is_some()
            || candidate.runtime_defaults.contains_key(program.key());
        if configured {
            let selected = candidate
                .selected_runtime(package)?
                .ok_or(PackageError::Untrusted)?;
            crate::trust::verify_selected_path(&selected, &runtime.executable)?;
        } else {
            // Compatibility with the original approval picker: its chosen executable
            // becomes explicit configuration, rather than silently changing a default.
            candidate.packages[index].runtime_override = Some(runtime.executable.clone());
        }
        if !same_runtime_selection(
            self.selected_runtime(&self.packages[index])?.as_deref(),
            Some(&runtime.executable),
        ) {
            candidate.revoke(id)?;
        }
        candidate.packages[index].trust = Some(crate::TrustApproval {
            content_hash: expected_content_hash.into(),
            runtime,
        });
        candidate.packages[index].verify_approval()?;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    pub fn revoke(&mut self, id: &str) -> Result<(), PackageError> {
        let package = self
            .packages
            .iter_mut()
            .find(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        package.enabled = false;
        package.enabled_events.clear();
        package.enabled_providers.clear();
        package.trust = None;
        Ok(())
    }

    pub fn set_settings(
        &mut self,
        id: &str,
        settings: BTreeMap<String, serde_json::Value>,
    ) -> Result<(), PackageError> {
        let mut candidate = self.clone();
        let package = candidate
            .packages
            .iter_mut()
            .find(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        if !package.manifest.settings_ready(&settings) {
            return Err(PackageError::InvalidManifest);
        }
        package.settings = settings;
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub fn set_event(
        &mut self,
        id: &str,
        handler: &str,
        enabled: bool,
    ) -> Result<(), PackageError> {
        let mut candidate = self.clone();
        let package = candidate
            .packages
            .iter_mut()
            .find(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        if !package.manifest.handlers.iter().any(|entry| {
            entry.id == handler
                && matches!(
                    entry.entry.as_str(),
                    "created" | "completed" | "failed" | "committed"
                )
        }) {
            return Err(PackageError::InvalidManifest);
        }
        if enabled {
            package.enabled_events.insert(handler.into());
        } else {
            package.enabled_events.remove(handler);
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub fn set_provider(
        &mut self,
        id: &str,
        handler: &str,
        enabled: bool,
    ) -> Result<(), PackageError> {
        let mut candidate = self.clone();
        let package = candidate
            .packages
            .iter_mut()
            .find(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        if !package.manifest.handlers.iter().any(|entry| {
            entry.id == handler && entry.feature == "quick_launch" && entry.entry == "provider"
        }) {
            return Err(PackageError::InvalidManifest);
        }
        if enabled {
            package.enabled_providers.insert(handler.into());
        } else {
            package.enabled_providers.remove(handler);
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }
    pub fn set_aliases(
        &mut self,
        id: &str,
        handler: &str,
        aliases: Vec<String>,
        manual: &[String],
    ) -> Result<(), PackageError> {
        self.validate()?;
        let mut candidate = self.clone();
        let package = candidate
            .packages
            .iter_mut()
            .find(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        if !package
            .manifest
            .handlers
            .iter()
            .any(|entry| entry.id == handler && entry.feature == "quick_launch")
        {
            return Err(PackageError::InvalidManifest);
        }
        package.aliases.insert(handler.into(), aliases);
        candidate.validate()?;
        candidate.check_keyword_conflicts(id, manual)?;
        *self = candidate;
        Ok(())
    }

    pub fn install(
        &mut self,
        preview: ImportPreview,
        source: String,
        replace: bool,
    ) -> Result<(), PackageError> {
        self.validate()?;
        preview.validate_preview()?;
        let file_hashes = crate::bundle::file_hashes(&preview.code_files);
        if preview.content_hash != crate::bundle::combined_hash(&preview.manifest, &file_hashes)?
            || source.len() > 32_768
            || source.contains('\0')
        {
            return Err(PackageError::InvalidManifest);
        }
        let existing = self
            .packages
            .iter()
            .position(|package| package.manifest.id == preview.manifest.id);
        if existing.is_some() && !replace {
            return Err(PackageError::AlreadyInstalled);
        }
        if existing.is_none() && self.packages.len() >= MAX_PACKAGES {
            return Err(PackageError::PackageLimit);
        }
        let aliases = existing
            .map(|index| {
                self.packages[index]
                    .aliases
                    .iter()
                    .filter(|(handler_id, _)| {
                        preview.manifest.handlers.iter().any(|handler| {
                            &handler.id == *handler_id && handler.feature == "quick_launch"
                        })
                    })
                    .map(|(id, aliases)| (id.clone(), aliases.clone()))
                    .collect()
            })
            .unwrap_or_default();
        if preview.manifest.runtime.is_some() || preview.code_directory.is_some() {
            if !preview.staged {
                return Err(PackageError::CodeRequired);
            }
            let directory = preview
                .code_directory
                .as_ref()
                .ok_or(PackageError::CodeRequired)?;
            crate::bundle::verify_package(directory, &preview.manifest, &file_hashes)?;
        }
        let settings = preview
            .manifest
            .settings
            .iter()
            .filter_map(|setting| {
                setting
                    .default_value()
                    .map(|value| (setting.key.clone(), value))
            })
            .collect();
        let package = InstalledPackage {
            manifest: preview.manifest,
            content_hash: preview.content_hash,
            source,
            enabled: false,
            aliases,
            file_hashes,
            code_directory: preview.code_directory,
            trust: None,
            runtime_override: None,
            settings,
            enabled_events: HashSet::new(),
            enabled_providers: HashSet::new(),
            data_uuid: existing.and_then(|index| self.packages[index].data_uuid),
        };
        let mut candidate = self.clone();
        if let Some(index) = existing {
            candidate.packages[index] = package;
        } else {
            candidate.packages.push(package);
        }
        candidate.validate()?;
        *self = candidate;
        Ok(())
    }

    pub fn set_enabled(
        &mut self,
        id: &str,
        enabled: bool,
        manual_keywords: &[String],
    ) -> Result<(), PackageError> {
        self.validate()?;
        let index = self
            .packages
            .iter()
            .position(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        if enabled {
            if self.packages[index].manifest.runtime.is_some() {
                self.packages[index].verify_approval()?;
                self.verify_runtime_selection(id)?;
            }
            self.check_keyword_conflicts(id, manual_keywords)?;
        }
        self.packages[index].enabled = enabled;
        Ok(())
    }

    fn check_keyword_conflicts(&self, id: &str, manual: &[String]) -> Result<(), PackageError> {
        let package = self
            .packages
            .iter()
            .find(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        let mut occupied = manual
            .iter()
            .map(|keyword| keyword.to_lowercase())
            .collect::<HashSet<_>>();
        for other in self
            .packages
            .iter()
            .filter(|other| other.enabled && other.manifest.id != id)
        {
            for handler in other
                .manifest
                .handlers
                .iter()
                .filter(|handler| handler.feature == "quick_launch")
            {
                occupied.extend(
                    other
                        .keywords(handler)
                        .iter()
                        .map(|keyword| keyword.to_lowercase()),
                );
            }
        }
        for handler in package
            .manifest
            .handlers
            .iter()
            .filter(|handler| handler.feature == "quick_launch")
        {
            for keyword in package.keywords(handler) {
                if occupied.contains(&keyword.to_lowercase()) {
                    return Err(PackageError::KeywordConflict {
                        keyword: keyword.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn validate_manual_keywords(&self, manual: &[String]) -> Result<(), PackageError> {
        for package in self.packages.iter().filter(|package| package.enabled) {
            self.check_keyword_conflicts(&package.manifest.id, manual)?;
        }
        Ok(())
    }

    pub fn remove(&mut self, id: &str) -> Result<(), PackageError> {
        let index = self
            .packages
            .iter()
            .position(|package| package.manifest.id == id)
            .ok_or(PackageError::NotInstalled)?;
        self.packages.remove(index);
        self.resolver_preferences
            .retain(|_, handler| handler.package_id != id);
        self.resolver_chains
            .retain(|_, chain| !chain.iter().any(|handler| handler.package_id == id));
        Ok(())
    }

    pub fn validate(&self) -> Result<(), PackageError> {
        self.validate_resolver_routes()?;
        for (program, executable) in &self.runtime_defaults {
            if !matches!(program.as_str(), "python" | "node" | "powershell" | "pwsh") {
                return Err(PackageError::InvalidManifest);
            }
            validate_runtime_path(executable)?;
        }
        if self.packages.len() > MAX_PACKAGES {
            return Err(PackageError::PackageLimit);
        }
        let mut ids = HashSet::new();
        let mut enabled_keywords = HashSet::new();
        let mut data_ids = HashSet::new();
        for package in &self.packages {
            package.manifest.validate()?;
            if package
                .data_uuid
                .is_some_and(|id| id.is_nil() || !data_ids.insert(id))
            {
                return Err(PackageError::InvalidManifest);
            }
            if let Some(executable) = &package.runtime_override {
                if !package
                    .manifest
                    .runtime
                    .as_ref()
                    .is_some_and(|runtime| runtime.program != crate::ScriptProgram::Exe)
                {
                    return Err(PackageError::InvalidManifest);
                }
                validate_runtime_path(executable)?;
            }
            if !ids.insert(&package.manifest.id)
                || package.content_hash
                    != crate::bundle::combined_hash(&package.manifest, &package.file_hashes)?
                || package.source.len() > 32_768
                || package.source.contains('\0')
                || package.aliases.len() > package.manifest.handlers.len()
                || package.aliases.keys().any(|id| {
                    !package
                        .manifest
                        .handlers
                        .iter()
                        .any(|handler| &handler.id == id && handler.feature == "quick_launch")
                })
            {
                return Err(PackageError::InvalidManifest);
            }
            if package.manifest.runtime.is_some() {
                if !package
                    .code_directory
                    .as_ref()
                    .is_some_and(|path| path.is_absolute())
                    || package
                        .file_hashes
                        .values()
                        .any(|hash| !crate::trust::valid_hash(hash))
                {
                    return Err(PackageError::InvalidManifest);
                }
                if let Some(approval) = &package.trust {
                    approval.runtime.validate()?;
                    if approval.content_hash != package.content_hash {
                        return Err(PackageError::Untrusted);
                    }
                }
                if package.enabled && package.trust.is_none() {
                    return Err(PackageError::Untrusted);
                }
            } else if !package.file_hashes.is_empty()
                || package.trust.is_some()
                || package
                    .code_directory
                    .as_ref()
                    .is_some_and(|path| !path.is_absolute())
            {
                return Err(PackageError::InvalidManifest);
            }
            if package.settings.len() > package.manifest.settings.len()
                || package.settings.iter().any(|(key, value)| {
                    !package
                        .manifest
                        .settings
                        .iter()
                        .any(|setting| &setting.key == key && setting.accepts_stored(value))
                })
            {
                return Err(PackageError::InvalidManifest);
            }
            if package.enabled_events.iter().any(|id| {
                !package.manifest.handlers.iter().any(|handler| {
                    &handler.id == id
                        && matches!(
                            handler.entry.as_str(),
                            "created" | "completed" | "failed" | "committed"
                        )
                })
            }) {
                return Err(PackageError::InvalidManifest);
            }
            if package.enabled_providers.iter().any(|id| {
                !package.manifest.handlers.iter().any(|handler| {
                    &handler.id == id
                        && handler.feature == "quick_launch"
                        && handler.entry == "provider"
                })
            }) {
                return Err(PackageError::InvalidManifest);
            }
            let mut package_keywords = HashSet::new();
            for handler in package
                .manifest
                .handlers
                .iter()
                .filter(|handler| handler.feature == "quick_launch")
            {
                let aliases = package.keywords(handler);
                if (aliases.is_empty() && handler.entry == "command") || aliases.len() > 32 {
                    return Err(PackageError::InvalidManifest);
                }
                for keyword in aliases {
                    if !valid_text(keyword, 64)
                        || keyword.chars().any(char::is_whitespace)
                        || !package_keywords.insert(keyword.to_lowercase())
                    {
                        return Err(PackageError::InvalidManifest);
                    }
                    if package.enabled && !enabled_keywords.insert(keyword.to_lowercase()) {
                        return Err(PackageError::KeywordConflict {
                            keyword: keyword.clone(),
                        });
                    }
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn validate_runtime_path(path: &Path) -> Result<(), PackageError> {
    let text = path.to_str().ok_or(PackageError::InvalidManifest)?;
    if !path.is_absolute()
        || text.is_empty()
        || text.len() > 32_768
        || text.chars().any(char::is_control)
    {
        return Err(PackageError::InvalidManifest);
    }
    Ok(())
}

/// Compare filesystem-free selections, including ordinary vs canonical Windows
/// path spelling. Save never needs to open an interpreter that may be absent.
pub(crate) fn same_runtime_selection(left: Option<&Path>, right: Option<&Path>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => runtime_path_key(left) == runtime_path_key(right),
        (None, None) => true,
        _ => false,
    }
}

fn runtime_path_key(path: &Path) -> PathBuf {
    #[cfg(windows)]
    let spelling = {
        let text = path.to_string_lossy().replace('/', "\\");
        PathBuf::from(text.strip_prefix("\\\\?\\").unwrap_or(&text).to_lowercase())
    };
    #[cfg(not(windows))]
    let spelling = path.to_path_buf();
    let mut normalized = PathBuf::new();
    for component in spelling.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}
