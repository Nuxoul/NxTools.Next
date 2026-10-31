//! Desktop composition only: original Core transactions and original service publication lock.
use nxtools_core::{AppConfig, ConfigExtensionStorage};
use nxtools_extensions::{
    InstalledPackage, PackageRegistry,
    storage::{StorageError, StorageHost, StorageOwner},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(super) struct DesktopStorage {
    pub(super) owner: StorageOwner,
    path: PathBuf,
    core: ConfigExtensionStorage,
    publication: Arc<Mutex<()>>,
    reconciled: Mutex<Option<bool>>,
    changed: tokio::sync::watch::Sender<u64>,
}
impl DesktopStorage {
    pub(super) fn new(
        path: PathBuf,
        root: PathBuf,
        publication: Arc<Mutex<()>>,
    ) -> Result<Self, StorageError> {
        let owner = StorageOwner::new(root)?;
        Ok(Self {
            core: ConfigExtensionStorage::new(path.clone(), owner.clone()),
            owner,
            path,
            publication,
            reconciled: Mutex::new(None),
            changed: tokio::sync::watch::channel(0).0,
        })
    }
    pub(super) fn subscribe(&self) -> tokio::sync::watch::Receiver<u64> {
        self.changed.subscribe()
    }
    fn reconcile_locked(&self) -> Result<bool, StorageError> {
        let mut ready = self
            .reconciled
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        if let Some(pending) = *ready {
            return Ok(pending);
        }
        let pending = AppConfig::reconcile_extension_storage(&self.path, &self.owner)
            .map_err(|_| StorageError::Io)?;
        *ready = Some(pending);
        Ok(pending)
    }
    pub(super) fn ensure_reconciled(&self) -> Result<bool, StorageError> {
        let _publication = self
            .publication
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        self.reconcile_locked()
    }
}
impl StorageHost for DesktopStorage {
    fn owner(&self) -> StorageOwner {
        self.owner.clone()
    }
    fn bind(
        &self,
        reviewed: &InstalledPackage,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<InstalledPackage, StorageError> {
        let _publication = self
            .publication
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        self.reconcile_locked()?;
        self.core.bind(reviewed, cancelled)
    }
    fn bind_and_apply(
        &self,
        reviewed: &InstalledPackage,
        cancelled: &(dyn Fn() -> bool + Sync),
        apply: &dyn Fn(&InstalledPackage) -> Result<(), StorageError>,
    ) -> Result<InstalledPackage, StorageError> {
        // Physical bounded worker owns this guard until BOTH durable Core commit and exact
        // runtime application finish. Never hold a GPUI entity lease or await under it.
        let _publication = self
            .publication
            .lock()
            .map_err(|_| StorageError::Unavailable)?;
        self.reconcile_locked()?;
        let bound = self.core.bind(reviewed, cancelled)?;
        apply(&bound)?;
        if reviewed != &bound {
            self.changed
                .send_modify(|revision| *revision = revision.wrapping_add(1));
        }
        Ok(bound)
    }
}

/// Update only exact None->committed UUID metadata; never import ordinary identity changes.
/// This is a read-only cache projection, NOT an execution/publication equality exemption.
pub(crate) fn refresh_storage_metadata(
    cached: &mut PackageRegistry,
    current: &PackageRegistry,
) -> bool {
    let mut changed = false;
    for package in &mut cached.packages {
        if package.data_uuid.is_some() {
            continue;
        }
        let Some(bound) = current
            .packages
            .iter()
            .find(|bound| bound.manifest.id == package.manifest.id)
        else {
            continue;
        };
        if bound.data_uuid.is_none_or(|id| id.is_nil()) {
            continue;
        }
        let mut expected = package.clone();
        expected.data_uuid = bound.data_uuid;
        if &expected == bound {
            *package = expected;
            changed = true;
        }
    }
    changed
}
