//! Bounded startup recovery under the original BlobStore cleanup and IO owner.
use super::*;
use std::path::Path;

pub(super) const OWNER: &str = ".owner";
const MAGIC: &[u8] = b"nxtools-extension-blobs\n";
const MAX_PARENT_ENTRIES: usize = 1024;
const MAX_RUNS: usize = 32;
const MAX_FILES: usize = 256;

pub(super) struct OwnerLease {
    file: File,
    ready: bool,
}

pub(super) fn create_lease(path: &Path, lease: &mut Option<OwnerLease>) -> Result<(), BlobError> {
    create_lease_with(path, lease, |file| {
        file.write_all(MAGIC).map_err(|_| BlobError::Io)?;
        file.sync_all().map_err(|_| BlobError::Io)
    })
}

pub(super) fn create_lease_with(
    path: &Path,
    lease: &mut Option<OwnerLease>,
    initialize: impl FnOnce(&mut File) -> Result<(), BlobError>,
) -> Result<(), BlobError> {
    if let Some(owner) = lease {
        return if owner.ready {
            Ok(())
        } else {
            Err(BlobError::Io)
        };
    }
    let file = OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(path.join(OWNER))
        .map_err(|_| BlobError::Io)?;
    // The original Root owns the created file before any fallible initialization step.
    let owner = lease.insert(OwnerLease { file, ready: false });
    fs2::FileExt::try_lock_exclusive(&owner.file).map_err(|_| BlobError::Io)?;
    initialize(&mut owner.file)?;
    owner.ready = true;
    Ok(())
}

fn canonical_uuid(value: &std::ffi::OsStr) -> bool {
    value
        .to_str()
        .is_some_and(|value| Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value))
}

fn recover_run(path: &Path) -> std::io::Result<bool> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || is_reparse(&metadata) {
        return Ok(false);
    }
    let marker = path.join(OWNER);
    let metadata = match std::fs::symlink_metadata(&marker) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error),
    };
    if !metadata.is_file() || is_reparse(&metadata) || metadata.len() != MAGIC.len() as u64 {
        return Ok(false);
    }
    let mut lease = OpenOptions::new().read(true).write(true).open(&marker)?;
    if fs2::FileExt::try_lock_exclusive(&lease).is_err() {
        return Ok(false);
    }
    let mut magic = [0; MAGIC.len()];
    lease.read_exact(&mut magic)?;
    if magic != MAGIC {
        return Ok(false);
    }
    // Validate the whole bounded direct-entry set before deleting any file. No recursive removal.
    let mut files = Vec::new();
    for (index, entry) in std::fs::read_dir(path)?.enumerate() {
        if index > MAX_FILES {
            return Ok(false);
        }
        let entry = entry?;
        let metadata = std::fs::symlink_metadata(entry.path())?;
        if !metadata.is_file() || is_reparse(&metadata) {
            return Ok(false);
        }
        if entry.file_name() == OWNER {
            continue;
        }
        if !canonical_uuid(&entry.file_name()) || files.len() == MAX_FILES {
            return Ok(false);
        }
        files.push(entry.path());
    }
    for file in files {
        std::fs::remove_file(file)?;
    }
    // Keep the marker and its lock on any failed backing delete, allowing a later startup retry.
    std::fs::remove_file(marker)?;
    drop(lease);
    std::fs::remove_dir(path)?;
    Ok(true)
}

pub(super) fn recover(current: &Path) {
    let Some(parent) = current.parent() else {
        return;
    };
    let Ok(entries) = std::fs::read_dir(parent) else {
        return;
    };
    let correlation_id = Uuid::new_v4();
    let started = Instant::now();
    tracing::info!(event = "extension.blob.recovery_accepted", %correlation_id);
    let mut candidates = 0;
    let mut recovered = 0;
    let mut failures = 0;
    let mut native_error_code = None;
    for entry in entries.take(MAX_PARENT_ENTRIES).flatten() {
        if entry.path() == current {
            continue;
        }
        let name = entry.file_name();
        let Some(suffix) = name
            .to_str()
            .and_then(|name| name.strip_prefix("nxtools-extension-blobs-"))
        else {
            continue;
        };
        if !canonical_uuid(std::ffi::OsStr::new(suffix)) {
            continue;
        }
        candidates += 1;
        match recover_run(&entry.path()) {
            Ok(true) => recovered += 1,
            Ok(false) => {}
            Err(error) => {
                failures += 1;
                native_error_code = error.raw_os_error();
            }
        }
        if candidates == MAX_RUNS {
            break;
        }
    }
    tracing::info!(event = "extension.blob.recovery_terminal", %correlation_id, candidates, recovered, failures, ?native_error_code, elapsed_ms = started.elapsed().as_millis(), "bounded temporary blob recovery completed");
}
