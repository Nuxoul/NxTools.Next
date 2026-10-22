use crate::{InstalledPackage, PackageError, ScriptProgram};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

/// Identity of the exact installed interpreter/EXE, not a PATH alias or a claimed version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeIdentity {
    pub executable: PathBuf,
    pub sha256: String,
}

impl RuntimeIdentity {
    pub fn inspect(executable: &Path) -> Result<Self, PackageError> {
        crate::registry::validate_runtime_path(executable)?;
        let metadata = fs::symlink_metadata(executable).map_err(|_| PackageError::FileOperation)?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(PackageError::InvalidManifest);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(PackageError::InvalidManifest);
            }
        }
        let executable = fs::canonicalize(executable).map_err(|_| PackageError::FileOperation)?;
        let mut file = fs::File::open(&executable).map_err(|_| PackageError::FileOperation)?;
        if file
            .metadata()
            .map_err(|_| PackageError::FileOperation)?
            .len()
            > 512 * 1024 * 1024
        {
            return Err(PackageError::ManifestTooLarge);
        }
        let mut digest = Sha256::new();
        let mut buffer = [0u8; 64 * 1024];
        let mut total = 0u64;
        loop {
            let count = file
                .read(&mut buffer)
                .map_err(|_| PackageError::FileOperation)?;
            if count == 0 {
                break;
            }
            total += count as u64;
            if total > 512 * 1024 * 1024 {
                return Err(PackageError::ManifestTooLarge);
            }
            digest.update(&buffer[..count]);
        }
        Ok(Self {
            executable,
            sha256: format!("{:x}", digest.finalize()),
        })
    }

    pub(crate) fn validate(&self) -> Result<(), PackageError> {
        crate::registry::validate_runtime_path(&self.executable)?;
        if !valid_hash(&self.sha256) {
            return Err(PackageError::InvalidManifest);
        }
        Ok(())
    }

    pub fn verify(&self) -> Result<(), PackageError> {
        if Self::inspect(&self.executable)? != *self {
            return Err(PackageError::Untrusted);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrustApproval {
    pub content_hash: String,
    pub runtime: RuntimeIdentity,
}

impl InstalledPackage {
    pub fn verify_current_code(&self) -> Result<(), PackageError> {
        if self.manifest.runtime.is_none() {
            return Ok(());
        }
        let root = self
            .code_directory
            .as_ref()
            .ok_or(PackageError::CodeRequired)?;
        crate::bundle::verify_package(root, &self.manifest, &self.file_hashes)
    }

    pub fn verify_approval(&self) -> Result<(), PackageError> {
        let approval = self.trust.as_ref().ok_or(PackageError::Untrusted)?;
        if approval.content_hash != self.content_hash {
            return Err(PackageError::Untrusted);
        }
        if let Some(executable) = &self.runtime_override {
            if self
                .manifest
                .runtime
                .as_ref()
                .is_none_or(|runtime| runtime.program == ScriptProgram::Exe)
            {
                return Err(PackageError::Untrusted);
            }
            verify_selected_path(executable, &approval.runtime.executable)?;
        }
        self.verify_current_code()?;
        approval.runtime.verify()?;
        if self
            .manifest
            .runtime
            .as_ref()
            .is_some_and(|runtime| runtime.program == ScriptProgram::Exe)
        {
            let root = self
                .code_directory
                .as_ref()
                .ok_or(PackageError::CodeRequired)?;
            for handler in &self.manifest.handlers {
                if let crate::DeclarativeAction::Script { entry_file } = &handler.action {
                    let entry = fs::canonicalize(root.join(entry_file.replace('\\', "/")))
                        .map_err(|_| PackageError::FileOperation)?;
                    if entry != approval.runtime.executable {
                        return Err(PackageError::Untrusted);
                    }
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn verify_selected_path(selected: &Path, approved: &Path) -> Result<(), PackageError> {
    crate::registry::validate_runtime_path(selected)?;
    let metadata = fs::symlink_metadata(selected).map_err(|_| PackageError::FileOperation)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(PackageError::InvalidManifest);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(PackageError::InvalidManifest);
        }
    }
    let current = fs::canonicalize(selected).map_err(|_| PackageError::FileOperation)?;
    if current != approved {
        return Err(PackageError::Untrusted);
    }
    Ok(())
}

pub(crate) fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
