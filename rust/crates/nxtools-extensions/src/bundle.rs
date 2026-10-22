use crate::{ImportPreview, MAX_MANIFEST_BYTES, Manifest, PackageError};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) const MAX_CODE_BYTES: u64 = 256 * 1024 * 1024;

impl ImportPreview {
    pub fn from_directory(root: &Path) -> Result<Self, PackageError> {
        reject_link(root)?;
        let bytes = read_regular(&root.join("extension.json"), MAX_MANIFEST_BYTES as u64)?;
        let manifest = crate::strict_json::manifest(&bytes)?;
        manifest.validate()?;
        let mut code_files = BTreeMap::new();
        let mut total = 0;
        for relative in &manifest.files {
            let path = checked_child(root, relative)?;
            let bytes = read_regular(&path, MAX_CODE_BYTES.saturating_sub(total))?;
            total += bytes.len() as u64;
            if total > MAX_CODE_BYTES {
                return Err(PackageError::ManifestTooLarge);
            }
            code_files.insert(relative.clone(), bytes);
        }
        let content_hash = combined_hash(&manifest, &file_hashes(&code_files))?;
        Ok(Self {
            manifest,
            content_hash,
            code_files,
            code_directory: None,
            staged: false,
        })
    }

    /// Write a content-addressed snapshot. A failed stage cannot change the registry.
    pub fn stage_into(&self, packages_root: &Path) -> Result<Self, PackageError> {
        self.validate_preview()?;
        let manifest_bytes =
            serde_json::to_vec(&self.manifest).map_err(|_| PackageError::InvalidManifest)?;
        if manifest_bytes.len() > MAX_MANIFEST_BYTES {
            return Err(PackageError::ManifestTooLarge);
        }
        fs::create_dir_all(packages_root).map_err(|_| PackageError::FileOperation)?;
        reject_link(packages_root)?;
        let packages_root =
            fs::canonicalize(packages_root).map_err(|_| PackageError::FileOperation)?;
        let owner_root = packages_root.join(&self.manifest.id);
        fs::create_dir_all(&owner_root).map_err(|_| PackageError::FileOperation)?;
        reject_link(&owner_root)?;
        let destination = owner_root.join(&self.content_hash);
        if destination.exists() {
            verify_package(&destination, &self.manifest, &file_hashes(&self.code_files))?;
        } else {
            let staging = owner_root.join(format!(
                ".stage-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |v| v.as_nanos())
            ));
            fs::create_dir(&staging).map_err(|_| PackageError::FileOperation)?;
            let result = (|| {
                for (relative, bytes) in &self.code_files {
                    let target = staging.join(relative.replace('\\', "/"));
                    if let Some(parent) = target.parent() {
                        fs::create_dir_all(parent).map_err(|_| PackageError::FileOperation)?;
                    }
                    let mut file = fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(&target)
                        .map_err(|_| PackageError::FileOperation)?;
                    file.write_all(bytes)
                        .and_then(|_| file.sync_all())
                        .map_err(|_| PackageError::FileOperation)?;
                }
                let mut manifest = fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(staging.join("extension.json"))
                    .map_err(|_| PackageError::FileOperation)?;
                manifest
                    .write_all(&manifest_bytes)
                    .and_then(|_| manifest.sync_all())
                    .map_err(|_| PackageError::FileOperation)?;
                drop(manifest);
                fs::rename(&staging, &destination).map_err(|_| PackageError::FileOperation)
            })();
            if result.is_err() {
                let _ = fs::remove_dir_all(&staging);
            }
            result?;
        }
        let mut staged = self.clone();
        staged.code_directory = Some(destination);
        staged.staged = true;
        Ok(staged)
    }

    pub(crate) fn validate_preview(&self) -> Result<(), PackageError> {
        self.manifest.validate()?;
        let total = self
            .code_files
            .values()
            .try_fold(0u64, |total, bytes| total.checked_add(bytes.len() as u64))
            .ok_or(PackageError::ManifestTooLarge)?;
        if total > MAX_CODE_BYTES {
            return Err(PackageError::ManifestTooLarge);
        }
        if self
            .code_files
            .keys()
            .any(|path| !crate::script::relative_code_path(path))
            || self.content_hash != combined_hash(&self.manifest, &file_hashes(&self.code_files))?
        {
            return Err(PackageError::InvalidManifest);
        }
        if self.manifest.runtime.is_none() && !self.code_files.is_empty() {
            return Err(PackageError::InvalidManifest);
        }
        Ok(())
    }
}

pub(crate) fn file_hashes(files: &BTreeMap<String, Vec<u8>>) -> BTreeMap<String, String> {
    files
        .iter()
        .map(|(name, bytes)| (name.clone(), format!("{:x}", Sha256::digest(bytes))))
        .collect()
}

pub(crate) fn combined_hash(
    manifest: &Manifest,
    files: &BTreeMap<String, String>,
) -> Result<String, PackageError> {
    if manifest.runtime.is_none() {
        return manifest.content_hash();
    }
    if files.len() != manifest.files.len()
        || manifest.files.iter().any(|file| !files.contains_key(file))
    {
        return Err(PackageError::CodeRequired);
    }
    let mut digest = Sha256::new();
    digest.update(serde_json::to_vec(manifest).map_err(|_| PackageError::InvalidManifest)?);
    for (name, hash) in files {
        digest.update((name.len() as u64).to_le_bytes());
        digest.update(name.as_bytes());
        digest.update(hash.as_bytes());
    }
    Ok(format!("{digest:x}", digest = digest.finalize()))
}

pub(crate) fn verify_files(
    root: &Path,
    hashes: &BTreeMap<String, String>,
) -> Result<(), PackageError> {
    let mut total = 0;
    for (relative, expected) in hashes {
        let bytes = read_regular(
            &checked_child(root, relative)?,
            MAX_CODE_BYTES.saturating_sub(total),
        )?;
        total += bytes.len() as u64;
        if format!("{:x}", Sha256::digest(bytes)) != *expected {
            return Err(PackageError::Untrusted);
        }
    }
    Ok(())
}

pub(crate) fn verify_package(
    root: &Path,
    manifest: &Manifest,
    hashes: &BTreeMap<String, String>,
) -> Result<(), PackageError> {
    reject_link(root)?;
    let bytes = read_regular(&root.join("extension.json"), MAX_MANIFEST_BYTES as u64)?;
    let current = crate::strict_json::manifest(&bytes).map_err(|_| PackageError::Untrusted)?;
    if current != *manifest {
        return Err(PackageError::Untrusted);
    }
    verify_files(root, hashes)
}

fn checked_child(root: &Path, relative: &str) -> Result<PathBuf, PackageError> {
    if !crate::script::relative_code_path(relative) {
        return Err(PackageError::InvalidManifest);
    }
    reject_link(root)?;
    let mut path = root.to_owned();
    for part in relative.split(['/', '\\']) {
        path.push(part);
        reject_link(&path)?;
    }
    Ok(path)
}

pub(crate) fn reject_link(path: &Path) -> Result<(), PackageError> {
    let metadata = fs::symlink_metadata(path).map_err(|_| PackageError::FileOperation)?;
    if metadata.file_type().is_symlink() {
        return Err(PackageError::InvalidManifest);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if metadata.file_attributes() & 0x400 != 0 {
            return Err(PackageError::InvalidManifest);
        }
    }
    Ok(())
}

fn read_regular(path: &Path, limit: u64) -> Result<Vec<u8>, PackageError> {
    reject_link(path)?;
    let file = fs::File::open(path).map_err(|_| PackageError::FileOperation)?;
    let metadata = file.metadata().map_err(|_| PackageError::FileOperation)?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(PackageError::ManifestTooLarge);
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| PackageError::FileOperation)?;
    if bytes.len() as u64 > limit {
        return Err(PackageError::ManifestTooLarge);
    }
    Ok(bytes)
}
