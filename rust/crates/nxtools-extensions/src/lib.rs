//! Host-neutral package identity and invocation contracts for NxTools extensions.
//! Parsing and preview never execute an action or start an interpreter.

mod archive;
mod broker;
mod bundle;
mod bundled;
pub mod command_template;
mod credential_transport;
mod events;
mod invocation;
mod registry;
mod resolver_routing;
mod result_merge;
pub use resolver_routing::{HandlerReference, ResolverCycleGuard, ResolverRoutingError};
mod runtime;
mod script;
mod settings_transaction;
mod strict_json;
mod trust;
pub use broker::{BrokerClient, BrokerError, BrokerEvent, BrokerRun, BrokerStart};
pub use bundled::bundled_previews;
pub use credential_transport::BrokerSecretStore;
pub use events::{Causality, EventAdmissionError, EventGuard};
pub use invocation::{
    ExtensionEffect, Invocation, InvocationError, InvocationIdentity, InvocationNotice,
    ProviderItem,
};
pub use registry::{InstalledPackage, PackageRegistry};
pub use result_merge::{MergedRow, ResultMerge};
pub use runtime::{
    ExtensionRuntime, FeatureAdapter, HostContext, HostFuture, HostReply, InvokeOutcome,
    InvokeRequest, RuntimeError,
};
pub use script::{ScriptProgram, ScriptRuntime, Setting, SettingType, UrlMatch};
pub use settings_transaction::{
    SecretFuture, SecretStore, SecretStoreError, SettingsDraft, SettingsSaveError, save_settings,
};
pub use trust::{RuntimeIdentity, TrustApproval};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
pub const MAX_PACKAGES: usize = 256;
pub const MAX_HANDLERS: usize = 100;
pub const MAX_ITEMS: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema_version: u16,
    pub id: String,
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub permissions: Vec<PermissionDeclaration>,
    pub handlers: Vec<Handler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<ScriptRuntime>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub settings: Vec<Setting>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionDeclaration {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Handler {
    pub id: String,
    pub title: String,
    pub feature: String,
    pub entry: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub url_matches: Vec<UrlMatch>,
    #[serde(default)]
    pub keywords: Vec<String>,
    pub action: DeclarativeAction,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum DeclarativeAction {
    Website {
        url_template: String,
    },
    Program {
        executable: String,
        #[serde(default)]
        argument_template: String,
        #[serde(default)]
        working_directory: String,
    },
    Folder {
        path: String,
    },
    Items {
        items: Vec<FixedItem>,
    },
    Script {
        entry_file: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedItem {
    pub id: String,
    pub title: String,
    pub action: Box<DeclarativeAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPreview {
    pub manifest: Manifest,
    pub content_hash: String,
    pub code_files: std::collections::BTreeMap<String, Vec<u8>>,
    pub(crate) staged: bool,
    pub code_directory: Option<std::path::PathBuf>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PackageError {
    #[error("extension ZIP is corrupt or uses an unsupported format")]
    ArchiveInvalid,
    #[error("extension ZIP contains unsafe or ambiguous entries")]
    ArchiveUnsafe,
    #[error("extension ZIP exceeds an archive resource limit")]
    ArchiveTooLarge,
    #[error("extension manifest exceeds the byte limit")]
    ManifestTooLarge,
    #[error("unsupported extension schema version")]
    UnsupportedVersion,
    #[error("invalid extension manifest")]
    InvalidManifest,
    #[error("extension package already exists; replacement must be explicit")]
    AlreadyInstalled,
    #[error("extension package is not installed")]
    NotInstalled,
    #[error("extension package limit reached")]
    PackageLimit,
    #[error("extension keyword conflicts: {keyword}")]
    KeywordConflict { keyword: String },
    #[error("script package requires verified code files")]
    CodeRequired,
    #[error("script package has not been explicitly trusted")]
    Untrusted,
    #[error("script package file operation failed")]
    FileOperation,
}

impl ImportPreview {
    pub fn from_json(bytes: &[u8]) -> Result<Self, PackageError> {
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(PackageError::ManifestTooLarge);
        }
        let manifest = strict_json::manifest(bytes)?;
        if manifest.runtime.is_some() {
            return Err(PackageError::CodeRequired);
        }
        manifest.validate()?;
        let content_hash = manifest.content_hash()?;
        Ok(Self {
            manifest,
            content_hash,
            code_files: Default::default(),
            code_directory: None,
            staged: false,
        })
    }
}

impl Manifest {
    pub fn validate(&self) -> Result<(), PackageError> {
        if self.schema_version != 1 {
            return Err(PackageError::UnsupportedVersion);
        }
        script::validate_script_manifest(self)?;
        if !valid_id(&self.id)
            || !valid_text(&self.name, 256)
            || !valid_text(&self.version, 64)
            || self.description.len() > 4096
            || self.author.len() > 256
            || self.handlers.is_empty()
            || self.handlers.len() > MAX_HANDLERS
            || self.permissions.len() > 64
        {
            return Err(PackageError::InvalidManifest);
        }
        let mut ids = HashSet::new();
        let mut keywords = HashSet::new();
        for handler in &self.handlers {
            if !valid_id(&handler.id)
                || !ids.insert(&handler.id)
                || !valid_text(&handler.title, 256)
                || !matches!(
                    (handler.feature.as_str(), handler.entry.as_str()),
                    ("quick_launch", "command")
                        | ("quick_launch", "provider")
                        | ("tools", "action")
                        | ("downloads", "resolve")
                        | ("downloads", "action")
                        | ("downloads", "created")
                        | ("downloads", "completed")
                        | ("downloads", "failed")
                        | ("clipboard", "action")
                        | ("clipboard", "committed")
                )
                || handler.keywords.len() > 32
                || (handler.feature == "quick_launch"
                    && handler.entry == "command"
                    && handler.keywords.is_empty())
            {
                return Err(PackageError::InvalidManifest);
            }
            for keyword in &handler.keywords {
                if !valid_text(keyword, 64)
                    || keyword.chars().any(char::is_whitespace)
                    || !keywords.insert(keyword.to_lowercase())
                {
                    return Err(PackageError::InvalidManifest);
                }
            }
            handler.action.validate(false)?;
            if let DeclarativeAction::Script { entry_file } = &handler.action {
                if self.runtime.is_none() || !self.files.contains(entry_file) {
                    return Err(PackageError::InvalidManifest);
                }
            } else if handler.entry != "command" && handler.feature != "tools" {
                return Err(PackageError::InvalidManifest);
            }
            if handler.url_matches.len() > 32
                || handler.url_matches.iter().any(|rule| {
                    rule.host.is_empty()
                        || rule.host.len() > 253
                        || rule.host.contains(['/', ':', '*', '\0'])
                        || rule.path.len() > 2048
                })
            {
                return Err(PackageError::InvalidManifest);
            }
        }
        let mut permissions = HashSet::new();
        for permission in &self.permissions {
            if !valid_id(&permission.id)
                || !valid_text(&permission.reason, 1024)
                || !permissions.insert(&permission.id)
            {
                return Err(PackageError::InvalidManifest);
            }
        }
        Ok(())
    }

    fn content_hash(&self) -> Result<String, PackageError> {
        let canonical = serde_json::to_vec(self).map_err(|_| PackageError::InvalidManifest)?;
        Ok(format!("{:x}", Sha256::digest(canonical)))
    }
}

impl DeclarativeAction {
    fn validate(&self, nested: bool) -> Result<(), PackageError> {
        let valid = match self {
            Self::Website { url_template } => {
                if !valid_text(url_template, 32_768) {
                    false
                } else {
                    url::Url::parse(&url_template.replace("{query}", "words")).is_ok_and(|url| {
                        matches!(url.scheme(), "http" | "https")
                            && url.host_str().is_some()
                            && url.username().is_empty()
                            && url.password().is_none()
                    })
                }
            }
            Self::Program {
                executable,
                argument_template,
                working_directory,
            } => {
                valid_text(executable, 32_768)
                    && argument_template.len() <= 32_768
                    && working_directory.len() <= 32_768
                    && !argument_template.contains('\0')
                    && !working_directory.contains('\0')
                    && command_template::validate_template(argument_template).is_ok()
            }
            Self::Script { entry_file } => !nested && script::relative_code_path(entry_file),
            Self::Folder { path } => valid_text(path, 32_768),
            Self::Items { items } => {
                if nested || items.is_empty() || items.len() > MAX_ITEMS {
                    false
                } else {
                    let mut ids = HashSet::new();
                    for item in items {
                        if !valid_id(&item.id)
                            || !ids.insert(&item.id)
                            || !valid_text(&item.title, 256)
                        {
                            return Err(PackageError::InvalidManifest);
                        }
                        item.action.validate(true)?;
                    }
                    true
                }
            }
        };
        if valid {
            Ok(())
        } else {
            Err(PackageError::InvalidManifest)
        }
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('.')
        && !value.ends_with('.')
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
}

fn valid_text(value: &str, max_bytes: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max_bytes && !value.chars().any(char::is_control)
}
