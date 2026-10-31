//! Host-owned durable extension data. This is not encrypted credential storage or an OS sandbox.
//! One finite serial queue per owner; at most four workers in this process. Filesystem locks
//! always follow global quota/catalog -> namespace order, including across owner instances.
use crate::{InvocationIdentity, strict_json::StrictValue};
use fs2::FileExt;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::Instant,
};
use tokio::sync::{oneshot, watch};
use uuid::Uuid;

pub const MAX_NAMESPACE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_GLOBAL_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_VALUE_BYTES: usize = 64 * 1024;
pub const MAX_KEYS: usize = 256;
pub const MAX_NAMESPACES: usize = 1024;
const MAX_METADATA_BYTES: usize = 1024 * 1024;
const MAX_REVISION: u64 = (1_u64 << 53) - 1;
static WORKERS: AtomicUsize = AtomicUsize::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum StorageError {
    #[error("invalid storage parameters")]
    InvalidParams,
    #[error("storage resource limit")]
    ResourceLimit,
    #[error("storage revision conflict")]
    Conflict,
    #[error("storage is corrupt; data was retained")]
    Corrupt,
    #[error("storage IO failed")]
    Io,
    #[error("storage operation cancelled before commit")]
    Cancelled,
    #[error("storage namespace retired or not owned by this installation")]
    Stale,
    #[error("storage owner unavailable")]
    Unavailable,
    #[error("storage physical cleanup is pending")]
    CleanupPending,
    #[error("storage cursor belongs to another invocation or query")]
    InvalidCursor,
    #[error("storage cursor revision is stale")]
    CursorStale,
}
impl StorageError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidParams => "invalid_params",
            Self::ResourceLimit => "resource_limit",
            Self::Conflict => "storage_conflict",
            Self::Corrupt => "storage_corrupt",
            Self::Io => "storage_io_failed",
            Self::Cancelled => "cancelled",
            Self::Stale => "storage_stale",
            Self::Unavailable => "capability_unavailable",
            Self::CleanupPending => "storage_cleanup_pending",
            Self::InvalidCursor => "invalid_cursor",
            Self::CursorStale => "cursor_stale",
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoredValue {
    pub found: bool,
    pub value: Value,
    /// The namespace revision observed atomically with this value, for optional CAS.
    pub revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageMutation {
    pub revision: u64,
    pub cleanup_pending: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StorageKey {
    pub key: String,
    pub revision: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StoragePage {
    pub items: Vec<StorageKey>,
    pub revision: u64,
    pub has_more: bool,
    pub next_cursor: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedStorage {
    pub namespace: Uuid,
    pub revision: u64,
    pub bytes: usize,
    pub cleanup_pending: bool,
}

/// Host-only management projection, never exposed by the script API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetainedStorageEntry {
    pub package: String,
    pub storage: RetainedStorage,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RetainedStoragePage {
    pub items: Vec<RetainedStorageEntry>,
    pub next_after: Option<Uuid>,
}

/// Host composition implements the original durable configuration transaction. Called only
/// on a bounded background worker; scripts cannot supply a namespace or storage path.
pub trait StorageHost: Send + Sync {
    fn owner(&self) -> StorageOwner;
    fn bind(
        &self,
        reviewed: &crate::InstalledPackage,
        cancelled: &(dyn Fn() -> bool + Sync),
    ) -> Result<crate::InstalledPackage, StorageError>;
    /// Synchronous background-only boundary. Composition may serialize its original
    /// config publication through this exact durable bind and runtime metadata application.
    /// `apply` must not perform IO, async work or call back into the host.
    fn bind_and_apply(
        &self,
        reviewed: &crate::InstalledPackage,
        cancelled: &(dyn Fn() -> bool + Sync),
        apply: &dyn Fn(&crate::InstalledPackage) -> Result<(), StorageError>,
    ) -> Result<crate::InstalledPackage, StorageError> {
        let bound = self.bind(reviewed, cancelled)?;
        apply(&bound)?;
        Ok(bound)
    }
}

type Job = Box<dyn FnOnce(&mut Worker) + Send>;
#[derive(Clone)]
pub struct StorageOwner {
    queue: mpsc::SyncSender<Job>,
}
#[derive(Clone)]
pub struct StorageNamespace {
    owner: StorageOwner,
    namespace: Uuid,
    identity: InvocationIdentity,
    cancellation: watch::Receiver<bool>,
}
struct Worker {
    root: PathBuf,
    cursors: crate::PageCursors,
}
impl Drop for Worker {
    fn drop(&mut self) {
        WORKERS.fetch_sub(1, Ordering::AcqRel);
    }
}
#[derive(Clone)]
struct Request {
    identity: InvocationIdentity,
    cancellation: watch::Receiver<bool>,
}
impl Request {
    fn check<T>(
        &self,
        reply: &oneshot::Sender<Result<T, StorageError>>,
    ) -> Result<(), StorageError> {
        if *self.cancellation.borrow() || reply.is_closed() {
            Err(StorageError::Cancelled)
        } else {
            Ok(())
        }
    }
}
impl StorageOwner {
    /// Does no caller-thread filesystem IO. The root is supplied by the host composition,
    /// outside immutable package staging; scripts never receive it or choose a namespace.
    pub fn new(root: PathBuf) -> Result<Self, StorageError> {
        if !root.is_absolute() {
            return Err(StorageError::InvalidParams);
        }
        WORKERS
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
                (count < 4).then_some(count + 1)
            })
            .map_err(|_| StorageError::ResourceLimit)?;
        let (queue, receiver) = mpsc::sync_channel::<Job>(32);
        let worker = Worker {
            root,
            cursors: Default::default(),
        };
        std::thread::Builder::new()
            .name("extension-storage-io".into())
            .spawn(move || {
                let mut worker = worker;
                while let Ok(job) = receiver.recv() {
                    job(&mut worker);
                }
            })
            .map_err(|_| StorageError::Unavailable)?;
        Ok(Self { queue })
    }
    async fn submit<T: Send + 'static>(
        &self,
        request: Option<Request>,
        operation: impl FnOnce(
            &mut Worker,
            &oneshot::Sender<Result<T, StorageError>>,
        ) -> Result<T, StorageError>
        + Send
        + 'static,
    ) -> Result<T, StorageError> {
        let (reply, result) = oneshot::channel();
        let started = Instant::now();
        if let Some(request) = &request {
            request.check(&reply)?;
        }
        let accepted_identity = request.as_ref().map(|request| request.identity.clone());
        let job: Job = Box::new(move |worker| {
            let identity = request.as_ref().map(|request| &request.identity);
            let span = tracing::info_span!("extension.storage.operation", correlation_id = ?identity.map(|id| id.correlation_id), run_id = ?identity.map(|id| id.run_id));
            let _entered = span.enter();
            tracing::info!(event = "extension.storage.started");
            let value = match request
                .as_ref()
                .map(|request| request.check(&reply))
                .transpose()
            {
                Ok(_) if !reply.is_closed() => operation(worker, &reply),
                _ => Err(StorageError::Cancelled),
            };
            tracing::info!(
                event = "extension.storage.terminal",
                reason = value.as_ref().err().map_or("success", |error| error.code()),
                elapsed_ms = started.elapsed().as_millis() as u64
            );
            let _ = reply.send(value);
        });
        self.queue.try_send(job).map_err(|error| match error {
            mpsc::TrySendError::Full(_) => StorageError::ResourceLimit,
            mpsc::TrySendError::Disconnected(_) => StorageError::Unavailable,
        })?;
        tracing::info!(event="extension.storage.accepted",correlation_id=?accepted_identity.as_ref().map(|id|id.correlation_id),run_id=?accepted_identity.as_ref().map(|id|id.run_id));
        result.await.map_err(|_| StorageError::Unavailable)?
    }
    /// Pre-reserves the retained/orphan metadata slot before Core may publish this UUID.
    pub async fn reserve_namespace(
        &self,
        namespace: Uuid,
        package: String,
    ) -> Result<(), StorageError> {
        if namespace.is_nil() || !crate::valid_id(&package) {
            return Err(StorageError::InvalidParams);
        }
        self.submit(None, move |worker, reply| {
            let _global = worker.global_lock()?;
            let mut catalog = worker.catalog()?;
            if let Some(record) = catalog.namespaces.get(&namespace) {
                return if record.package == package && record.state == NamespaceState::Active {
                    Ok(())
                } else {
                    Err(StorageError::Stale)
                };
            }
            if catalog.namespaces.len() >= MAX_NAMESPACES {
                return Err(StorageError::ResourceLimit);
            }
            if reply.is_closed() {
                return Err(StorageError::Cancelled);
            }
            catalog.namespaces.insert(
                namespace,
                Record {
                    package,
                    state: NamespaceState::Active,
                    retirement: None,
                    file: None,
                    bytes: 0,
                    revision: 0,
                },
            );
            worker.save_catalog(&catalog)
        })
        .await
    }
    /// Background Core transaction entry; pre-reserves the orphan slot before pointer commit.
    pub fn reserve_namespace_blocking(
        &self,
        namespace: Uuid,
        package: &str,
    ) -> Result<(), StorageError> {
        if namespace.is_nil() || !crate::valid_id(package) {
            return Err(StorageError::InvalidParams);
        }
        let package = package.to_owned();
        self.management_blocking(move |worker| {
            let _global = worker.global_lock()?;
            let mut catalog = worker.catalog()?;
            if let Some(record) = catalog.namespaces.get(&namespace) {
                return if record.package == package && record.state == NamespaceState::Active {
                    Ok(())
                } else {
                    Err(StorageError::Stale)
                };
            }
            if catalog.namespaces.len() >= MAX_NAMESPACES {
                return Err(StorageError::ResourceLimit);
            }
            catalog.namespaces.insert(
                namespace,
                Record {
                    package,
                    state: NamespaceState::Active,
                    retirement: None,
                    file: None,
                    bytes: 0,
                    revision: 0,
                },
            );
            worker.save_catalog(&catalog)
        })
    }
    /// Only for an existing background Core config transaction, never a render/async reader.
    fn management_blocking<T: Send + 'static>(
        &self,
        operation: impl FnOnce(&mut Worker) -> Result<T, StorageError> + Send + 'static,
    ) -> Result<T, StorageError> {
        let (reply, result) = mpsc::sync_channel(1);
        let span = tracing::Span::current();
        let started = Instant::now();
        tracing::info!(event = "extension.storage.management_accepted");
        self.queue
            .try_send(Box::new(move |worker| {
                let _entered = span.enter();
                tracing::info!(event = "extension.storage.management_started");
                let value = operation(worker);
                tracing::info!(
                    event = "extension.storage.management_terminal",
                    reason = value.as_ref().err().map_or("success", |error| error.code()),
                    elapsed_ms = started.elapsed().as_millis() as u64
                );
                let _ = reply.send(value);
            }))
            .map_err(|error| {
                let reason = match error {
                    mpsc::TrySendError::Full(_) => StorageError::ResourceLimit,
                    mpsc::TrySendError::Disconnected(_) => StorageError::Unavailable,
                };
                tracing::info!(
                    event = "extension.storage.management_terminal",
                    reason = reason.code(),
                    elapsed_ms = started.elapsed().as_millis() as u64
                );
                reason
            })?;
        result.recv().map_err(|_| StorageError::Unavailable)?
    }
    /// Call while Core holds its original config lock and has validated the reviewed target.
    /// This records intent only: current invocations can still finish until the config commits.
    pub fn prepare_retirement_blocking(
        &self,
        namespace: Uuid,
        package: &str,
        clear: bool,
    ) -> Result<(), StorageError> {
        let package = package.to_owned();
        self.management_blocking(move |worker| {
            let _global = worker.global_lock()?;
            let mut catalog = worker.catalog()?;
            catalog.active(namespace, &package)?;
            let _namespace = worker.namespace_lock(namespace)?;
            catalog
                .namespaces
                .get_mut(&namespace)
                .ok_or(StorageError::Stale)?
                .retirement = Some(clear);
            worker.save_catalog(&catalog)?;
            tracing::info!(event = "extension.storage.retirement_prepared", clear);
            Ok(())
        })
    }
    /// Core must hold its original config lock throughout preparation, pointer commit and
    /// reconciliation. The durable intent quiesces old handles across independent owners.
    pub fn prepare_restore_blocking(
        &self,
        package: &str,
        reviewed: &RetainedStorage,
        destination: Option<Uuid>,
        restored: Uuid,
    ) -> Result<(), StorageError> {
        if !crate::valid_id(package) || restored.is_nil() {
            return Err(StorageError::InvalidParams);
        }
        let package = package.to_owned();
        let reviewed = reviewed.clone();
        self.management_blocking(move |worker| {
            let _global = worker.global_lock()?;
            let mut catalog = worker.catalog()?;
            if catalog.restore.is_some() {
                return Err(StorageError::CleanupPending);
            }
            if catalog.namespaces.contains_key(&restored) || destination == Some(reviewed.namespace)
            {
                return Err(StorageError::Stale);
            }
            if catalog.namespaces.len() >= MAX_NAMESPACES {
                return Err(StorageError::ResourceLimit);
            }
            let source = catalog.reviewed_retained(&package, &reviewed)?;
            if source.state != NamespaceState::Retained {
                return Err(StorageError::Stale);
            }
            let _source = worker.namespace_lock(reviewed.namespace)?;
            worker.document(source)?;
            let _destination = if let Some(destination) = destination {
                let record = catalog.active(destination, &package)?;
                let lock = worker.namespace_lock(destination)?;
                if !worker.document(record)?.values.is_empty() {
                    return Err(StorageError::Conflict);
                }
                Some(lock)
            } else {
                None
            };
            catalog.namespaces.insert(
                restored,
                Record::empty(package.clone(), NamespaceState::RestorePending),
            );
            catalog.restore = Some(RestoreIntent {
                package,
                source: reviewed.namespace,
                revision: reviewed.revision,
                bytes: reviewed.bytes,
                destination,
                restored,
            });
            worker.save_catalog(&catalog)?;
            tracing::info!(event = "extension.storage.restore_prepared");
            Ok(())
        })
    }

    /// Explicit retained clear/retry, while Core holds its original lock around a fresh
    /// registry review. No config pointer may reference the selected retained generation.
    pub fn clear_retained_blocking(
        &self,
        package: &str,
        reviewed: &RetainedStorage,
        registry: &crate::PackageRegistry,
    ) -> Result<bool, StorageError> {
        registry.validate().map_err(|_| StorageError::Corrupt)?;
        let registry = registry.clone();
        let package = package.to_owned();
        let reviewed = reviewed.clone();
        self.management_blocking(move |worker| {
            let _global = worker.global_lock()?;
            let mut catalog = worker.catalog()?;
            if catalog.restore.is_some() {
                return Err(StorageError::CleanupPending);
            }
            for current in &registry.packages {
                if let Some(namespace) = current.data_uuid {
                    catalog
                        .active(namespace, &current.manifest.id)
                        .map_err(|_| StorageError::Corrupt)?;
                    if namespace == reviewed.namespace {
                        return Err(StorageError::Stale);
                    }
                }
            }
            let record = catalog.reviewed_retained(&package, &reviewed)?;
            let _namespace = worker.namespace_lock(reviewed.namespace)?;
            if record.state == NamespaceState::Retained {
                worker.document(record)?;
            }
            catalog
                .namespaces
                .get_mut(&reviewed.namespace)
                .ok_or(StorageError::Stale)?
                .state = NamespaceState::ClearPending;
            worker.save_catalog(&catalog)?;
            drop(_namespace);
            tracing::info!(event = "extension.storage.retained_clear_committed");
            // Durable deletion authorization succeeded. Subsequent physical failure is
            // pending, not a rollback or permission to release the reserved bytes/slot.
            Ok(worker.clear_retired(&mut catalog).unwrap_or(true))
        })
    }

    /// The caller MUST hold Core's original config lock around the freshly loaded registry
    /// and this entire call. Never reconcile/delete against a cached publication snapshot.
    pub fn reconcile_current_registry_blocking(
        &self,
        registry: &crate::PackageRegistry,
    ) -> Result<bool, StorageError> {
        registry.validate().map_err(|_| StorageError::Corrupt)?;
        let pointers: BTreeMap<_, _> = registry
            .packages
            .iter()
            .filter_map(|package| {
                package
                    .data_uuid
                    .map(|id| (id, package.manifest.id.clone()))
            })
            .collect();
        self.management_blocking(move |worker| {
            let _global = worker.global_lock()?;
            let mut catalog = worker.catalog()?;
            if let Some(intent) = &catalog.restore {
                let _source = worker.namespace_lock(intent.source)?;
                worker.document(
                    catalog
                        .namespaces
                        .get(&intent.source)
                        .ok_or(StorageError::Corrupt)?,
                )?;
                if let Some(destination) = intent.destination {
                    let _destination = worker.namespace_lock(destination)?;
                    if !worker
                        .document(
                            catalog
                                .namespaces
                                .get(&destination)
                                .ok_or(StorageError::Corrupt)?,
                        )?
                        .values
                        .is_empty()
                    {
                        return Err(StorageError::Corrupt);
                    }
                }
            }
            catalog.resolve_restore(&pointers)?;
            // Ambiguous/missing/corrupt metadata is never permission to erase or resurrect.
            for (namespace, package) in &pointers {
                if !catalog.namespaces.get(namespace).is_some_and(|record| {
                    record.package == *package && record.state == NamespaceState::Active
                }) {
                    return Err(StorageError::Corrupt);
                }
            }
            for (namespace, record) in &mut catalog.namespaces {
                if pointers.contains_key(namespace) {
                    record.retirement = None;
                } else if record.state == NamespaceState::Active {
                    record.state = if record.retirement == Some(true) {
                        NamespaceState::ClearPending
                    } else {
                        NamespaceState::Retained
                    };
                    record.retirement = None;
                }
            }
            // Publish logical retirement before physical cleanup. Per-namespace locks below
            // drain actual operations, including jobs whose async caller was already dropped.
            worker.save_catalog(&catalog)?;
            let pending = worker.clear_retired(&mut catalog)?;
            tracing::info!(
                event = "extension.storage.reconciled",
                namespaces = catalog.namespaces.len(),
                cleanup_pending = pending
            );
            Ok(pending)
        })
    }
    /// Read at most sixteen management records. The continuation is only an ordering key,
    /// not authority: every later mutation rechecks the exact record and current registry.
    pub fn retained_page_blocking(
        &self,
        package: Option<String>,
        after: Option<Uuid>,
    ) -> Result<RetainedStoragePage, StorageError> {
        if package
            .as_ref()
            .is_some_and(|package| !crate::valid_id(package))
        {
            return Err(StorageError::InvalidParams);
        }
        self.management_blocking(move |worker| {
            let _global = worker.global_lock()?;
            let catalog = worker.catalog()?;
            let mut records = catalog.namespaces.iter().filter(|(namespace, record)| {
                after.is_none_or(|after| **namespace > after)
                    && package
                        .as_ref()
                        .is_none_or(|package| &record.package == package)
                    && matches!(
                        record.state,
                        NamespaceState::Retained | NamespaceState::ClearPending
                    )
            });
            let items = records
                .by_ref()
                .take(16)
                .map(|(namespace, record)| RetainedStorageEntry {
                    package: record.package.clone(),
                    storage: RetainedStorage {
                        namespace: *namespace,
                        revision: record.revision,
                        bytes: record.bytes,
                        cleanup_pending: record.state == NamespaceState::ClearPending,
                    },
                })
                .collect::<Vec<_>>();
            let next_after = records
                .next()
                .and_then(|_| items.last().map(|item| item.storage.namespace));
            Ok(RetainedStoragePage { items, next_after })
        })
    }
    /// Bounded UI-only metadata, not diagnostic payload or a script-accessible namespace list.
    pub fn retained_metadata_blocking(
        &self,
        package: &str,
    ) -> Result<Vec<RetainedStorage>, StorageError> {
        let package = package.to_owned();
        self.management_blocking(move |worker| {
            let _global = worker.global_lock()?;
            let catalog = worker.catalog()?;
            Ok(catalog
                .namespaces
                .iter()
                .filter(|(_, record)| {
                    record.package == package
                        && matches!(
                            record.state,
                            NamespaceState::Retained | NamespaceState::ClearPending
                        )
                })
                .map(|(namespace, record)| RetainedStorage {
                    namespace: *namespace,
                    revision: record.revision,
                    bytes: record.bytes,
                    cleanup_pending: record.state == NamespaceState::ClearPending,
                })
                .collect())
        })
    }
    /// An already published config pointer must resolve, never create a replacement record.
    pub fn verify_namespace_blocking(
        &self,
        namespace: Uuid,
        package: &str,
    ) -> Result<(), StorageError> {
        let package = package.to_owned();
        self.management_blocking(move |worker| {
            let _global = worker.global_lock()?;
            let catalog = worker.catalog()?;
            let record = catalog.active(namespace, &package)?;
            let _namespace = worker.namespace_lock(namespace)?;
            worker.document(record)?;
            Ok(())
        })
    }
    pub fn namespace(
        &self,
        namespace: Uuid,
        identity: InvocationIdentity,
        cancellation: watch::Receiver<bool>,
    ) -> StorageNamespace {
        StorageNamespace {
            owner: self.clone(),
            namespace,
            identity,
            cancellation,
        }
    }
}
impl StorageNamespace {
    fn request(&self) -> Request {
        Request {
            identity: self.identity.clone(),
            cancellation: self.cancellation.clone(),
        }
    }
    pub async fn get(&self, key: String) -> Result<StoredValue, StorageError> {
        validate_key(&key)?;
        let namespace = self.namespace;
        let request = self.request();
        self.owner
            .submit(Some(request.clone()), move |worker, reply| {
                let _global = worker.global_lock()?;
                let catalog = worker.catalog()?;
                let record = catalog.active(namespace, &request.identity.package_id)?;
                let _namespace = worker.namespace_lock(namespace)?;
                request.check(reply)?;
                let document = worker.document(record)?;
                Ok(StoredValue {
                    found: document.values.contains_key(&key),
                    value: document
                        .values
                        .get(&key)
                        .map_or(Value::Null, |entry| entry.value.clone()),
                    revision: record.revision,
                })
            })
            .await
    }
    pub async fn list(
        &self,
        prefix: String,
        limit: usize,
        cursor: Option<Uuid>,
    ) -> Result<StoragePage, StorageError> {
        if prefix.len() > 128 || prefix.chars().any(char::is_control) || !(1..=32).contains(&limit)
        {
            return Err(StorageError::InvalidParams);
        }
        let namespace = self.namespace;
        let request = self.request();
        self.owner
            .submit(Some(request.clone()), move |worker, reply| {
                let _global = worker.global_lock()?;
                let catalog = worker.catalog()?;
                let record = catalog.active(namespace, &request.identity.package_id)?;
                let _namespace = worker.namespace_lock(namespace)?;
                request.check(reply)?;
                let query =
                    serde_json::json!({"namespace":namespace,"prefix":prefix,"limit":limit});
                let after = if let Some(cursor) = cursor {
                    let state = worker
                        .cursors
                        .read(cursor, &request.identity, "storage.list", &query)
                        .map_err(|_| StorageError::InvalidCursor)?;
                    if state["revision"].as_u64() != Some(record.revision) {
                        return Err(StorageError::CursorStale);
                    }
                    Some(
                        state["after"]
                            .as_str()
                            .ok_or(StorageError::InvalidCursor)?
                            .to_owned(),
                    )
                } else {
                    None
                };
                let document = worker.document(record)?;
                let mut matching = document.values.iter().filter(|(key, _)| {
                    key.starts_with(&prefix) && after.as_ref().is_none_or(|after| *key > after)
                });
                let items: Vec<_> = matching
                    .by_ref()
                    .take(limit)
                    .map(|(key, entry)| StorageKey {
                        key: key.clone(),
                        revision: entry.revision,
                    })
                    .collect();
                let has_more = matching.next().is_some();
                let next_cursor = if has_more {
                    let after = &items.last().ok_or(StorageError::Corrupt)?.key;
                    Some(
                        worker
                            .cursors
                            .remember(
                                &request.identity,
                                "storage.list",
                                query,
                                serde_json::json!({"revision":record.revision,"after":after}),
                            )
                            .map_err(|_| StorageError::ResourceLimit)?,
                    )
                } else {
                    None
                };
                let page = StoragePage {
                    items,
                    revision: record.revision,
                    has_more,
                    next_cursor,
                };
                if serde_json::to_vec(&page)
                    .map_err(|_| StorageError::Corrupt)?
                    .len()
                    > 128 * 1024
                {
                    return Err(StorageError::ResourceLimit);
                }
                Ok(page)
            })
            .await
    }
    pub async fn set(
        &self,
        key: String,
        value: Value,
        expected_revision: Option<u64>,
    ) -> Result<StorageMutation, StorageError> {
        validate_key(&key)?;
        validate_value(&value)?;
        self.mutate(key, Some(value), expected_revision).await
    }
    pub async fn delete(
        &self,
        key: String,
        expected_revision: Option<u64>,
    ) -> Result<StorageMutation, StorageError> {
        validate_key(&key)?;
        self.mutate(key, None, expected_revision).await
    }
    async fn mutate(
        &self,
        key: String,
        value: Option<Value>,
        expected_revision: Option<u64>,
    ) -> Result<StorageMutation, StorageError> {
        let namespace = self.namespace;
        let request = self.request();
        self.owner
            .submit(Some(request.clone()), move |worker, reply| {
                let _global = worker.global_lock()?;
                let mut catalog = worker.catalog()?;
                let previous = catalog
                    .active(namespace, &request.identity.package_id)?
                    .clone();
                let _namespace = worker.namespace_lock(namespace)?;
                request.check(reply)?;
                if expected_revision.is_some_and(|expected| expected != previous.revision) {
                    return Err(StorageError::Conflict);
                }
                let mut document = worker.document(&previous)?;
                let revision = previous
                    .revision
                    .checked_add(1)
                    .filter(|revision| *revision <= MAX_REVISION)
                    .ok_or(StorageError::ResourceLimit)?;
                if let Some(value) = value {
                    document.values.insert(key, Entry { value, revision });
                } else {
                    document.values.remove(&key);
                }
                document.revision = revision;
                if document.values.len() > MAX_KEYS {
                    return Err(StorageError::ResourceLimit);
                }
                let bytes =
                    serde_json::to_vec(&document).map_err(|_| StorageError::InvalidParams)?;
                if bytes.len() > MAX_NAMESPACE_BYTES
                    || catalog.total_bytes() - previous.bytes + bytes.len() > MAX_GLOBAL_BYTES
                {
                    return Err(StorageError::ResourceLimit);
                }
                request.check(reply)?;
                // One persistent physical-copy reservation, including crash/failed-delete recovery.
                // No new copy may be allocated while its predecessor cannot actually be removed.
                worker.cleanup_pending(&mut catalog)?;
                let file = Uuid::new_v4();
                catalog.pending = Some(file);
                worker.save_catalog(&catalog)?;
                let target = worker.data_path(file);
                let prepare = (|| {
                    request.check(reply)?;
                    write_new(&target, &bytes)?;
                    request.check(reply)?;
                    let record = catalog
                        .namespaces
                        .get_mut(&namespace)
                        .ok_or(StorageError::Stale)?;
                    record.file = Some(file);
                    record.bytes = bytes.len();
                    record.revision = revision;
                    catalog.pending = previous.file;
                    worker.save_catalog(&catalog)
                })();
                if let Err(error) = prepare {
                    // Catalog commit is the single source of truth. Reload rather than assume rollback.
                    if let Ok(mut durable) = worker.catalog() {
                        let _ = worker.cleanup_pending(&mut durable);
                    }
                    return Err(error);
                }
                tracing::info!(
                    event = "extension.storage.committed",
                    revision,
                    keys = document.values.len(),
                    bytes = bytes.len()
                );
                let cleanup_pending = worker.cleanup_pending(&mut catalog).is_err();
                if cleanup_pending {
                    tracing::warn!(
                        event = "extension.storage.cleanup_pending",
                        reason = "physical_copy_retained"
                    );
                }
                Ok(StorageMutation {
                    revision,
                    cleanup_pending,
                })
            })
            .await
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum NamespaceState {
    Active,
    Retained,
    ClearPending,
    RestorePending,
    CleanupOnly,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    package: String,
    state: NamespaceState,
    #[serde(default)]
    retirement: Option<bool>,
    file: Option<Uuid>,
    bytes: usize,
    revision: u64,
}
impl Record {
    fn empty(package: String, state: NamespaceState) -> Self {
        Self {
            package,
            state,
            retirement: None,
            file: None,
            bytes: 0,
            revision: 0,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RestoreIntent {
    package: String,
    source: Uuid,
    revision: u64,
    bytes: usize,
    destination: Option<Uuid>,
    restored: Uuid,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Catalog {
    version: u16,
    namespaces: BTreeMap<Uuid, Record>,
    pending: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    restore: Option<RestoreIntent>,
}
impl Default for Catalog {
    fn default() -> Self {
        Self {
            version: 1,
            namespaces: BTreeMap::new(),
            pending: None,
            restore: None,
        }
    }
}
impl Catalog {
    fn reviewed_retained(
        &self,
        package: &str,
        reviewed: &RetainedStorage,
    ) -> Result<&Record, StorageError> {
        self.namespaces
            .get(&reviewed.namespace)
            .filter(|record| {
                record.package == package
                    && matches!(
                        record.state,
                        NamespaceState::Retained | NamespaceState::ClearPending
                    )
                    && record.revision == reviewed.revision
                    && record.bytes == reviewed.bytes
                    && (record.state == NamespaceState::ClearPending) == reviewed.cleanup_pending
            })
            .ok_or(StorageError::Stale)
    }
    fn resolve_restore(&mut self, pointers: &BTreeMap<Uuid, String>) -> Result<(), StorageError> {
        let Some(intent) = self.restore.clone() else {
            return Ok(());
        };
        let current = pointers
            .iter()
            .find(|(_, package)| **package == intent.package)
            .map(|(id, _)| *id);
        if current == Some(intent.restored) {
            let mut source = self
                .namespaces
                .get(&intent.source)
                .ok_or(StorageError::Corrupt)?
                .clone();
            source.state = NamespaceState::Active;
            source.retirement = None;
            self.namespaces.insert(intent.restored, source);
            self.namespaces.insert(
                intent.source,
                Record::empty(intent.package.clone(), NamespaceState::CleanupOnly),
            );
            if let Some(destination) = intent.destination {
                self.namespaces
                    .get_mut(&destination)
                    .ok_or(StorageError::Corrupt)?
                    .state = NamespaceState::CleanupOnly;
            }
        } else if current == intent.destination || current.is_none() {
            self.namespaces.insert(
                intent.restored,
                Record::empty(intent.package, NamespaceState::CleanupOnly),
            );
        } else {
            return Err(StorageError::Corrupt);
        }
        self.restore = None;
        Ok(())
    }
    fn total_bytes(&self) -> usize {
        self.namespaces.values().map(|record| record.bytes).sum()
    }
    fn active(&self, namespace: Uuid, package: &str) -> Result<&Record, StorageError> {
        if self
            .restore
            .as_ref()
            .is_some_and(|intent| intent.destination == Some(namespace))
        {
            return Err(StorageError::Stale);
        }
        self.namespaces
            .get(&namespace)
            .filter(|record| record.state == NamespaceState::Active && record.package == package)
            .ok_or(StorageError::Stale)
    }
    fn validate(&self) -> Result<(), StorageError> {
        if self.version != 1 || self.namespaces.len() > MAX_NAMESPACES {
            return Err(StorageError::Corrupt);
        }
        if let Some(intent) = &self.restore {
            let source = self
                .namespaces
                .get(&intent.source)
                .ok_or(StorageError::Corrupt)?;
            let restored = self
                .namespaces
                .get(&intent.restored)
                .ok_or(StorageError::Corrupt)?;
            if intent.source == intent.restored
                || intent.destination == Some(intent.source)
                || intent.destination == Some(intent.restored)
                || source.package != intent.package
                || source.state != NamespaceState::Retained
                || source.revision != intent.revision
                || source.bytes != intent.bytes
                || restored.package != intent.package
                || restored.state != NamespaceState::RestorePending
                || intent.destination.is_some_and(|id| {
                    !self.namespaces.get(&id).is_some_and(|record| {
                        record.package == intent.package && record.state == NamespaceState::Active
                    })
                })
            {
                return Err(StorageError::Corrupt);
            }
        }
        let mut files = std::collections::HashSet::new();
        for (namespace, record) in &self.namespaces {
            if namespace.is_nil()
                || !crate::valid_id(&record.package)
                || record.bytes > MAX_NAMESPACE_BYTES
                || record.revision > MAX_REVISION
                || record.file.is_none() != (record.bytes == 0 && record.revision == 0)
                || (record.state == NamespaceState::CleanupOnly && record.retirement.is_some())
                || (record.state == NamespaceState::RestorePending
                    && (record.file.is_some() || record.retirement.is_some()))
                || (record.state == NamespaceState::RestorePending
                    && self
                        .restore
                        .as_ref()
                        .is_none_or(|intent| intent.restored != *namespace))
                || record
                    .file
                    .is_some_and(|file| file.is_nil() || !files.insert(file))
            {
                return Err(StorageError::Corrupt);
            }
        }
        if self.total_bytes() > MAX_GLOBAL_BYTES
            || self
                .pending
                .is_some_and(|file| file.is_nil() || files.contains(&file))
        {
            return Err(StorageError::Corrupt);
        }
        Ok(())
    }
}
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    version: u16,
    revision: u64,
    values: BTreeMap<String, Entry>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    value: Value,
    revision: u64,
}

impl Worker {
    fn global_lock(&self) -> Result<File, StorageError> {
        fs::create_dir_all(&self.root).map_err(io_error)?;
        safe_directory(&self.root)?;
        lock_file(&self.root.join("quota.lock"))
    }
    fn namespace_lock(&self, namespace: Uuid) -> Result<File, StorageError> {
        lock_file(&self.root.join(format!("{namespace}.lock")))
    }
    fn data_path(&self, file: Uuid) -> PathBuf {
        self.root.join(format!("{file}.json"))
    }
    fn catalog(&self) -> Result<Catalog, StorageError> {
        safe_directory(&self.root)?;
        let path = self.root.join("catalog.json");
        let catalog = match safe_regular(&path) {
            Ok(None) => {
                // Only a new, empty managed root may initialize. An interrupted creation,
                // missing marker/catalog, or stray backing must never become an empty store.
                let sentinel = self.root.join("initialized");
                if safe_regular(&sentinel)?.is_some() {
                    return Err(StorageError::Corrupt);
                }
                for entry in fs::read_dir(&self.root).map_err(io_error)? {
                    if entry.map_err(io_error)?.file_name() != "quota.lock" {
                        return Err(StorageError::Corrupt);
                    }
                }
                let catalog = Catalog::default();
                self.save_catalog(&catalog)?;
                write_new(&sentinel, b"1")?;
                catalog
            }
            Ok(Some(_)) => {
                let marker = self.root.join("initialized");
                if safe_regular(&marker)?.is_none_or(|metadata| metadata.len() != 1)
                    || fs::read(&marker).map_err(io_error)? != b"1"
                {
                    return Err(StorageError::Corrupt);
                }
                read_json(&path, MAX_METADATA_BYTES)?
            }
            Err(error) => return Err(error),
        };
        catalog.validate()?;
        Ok(catalog)
    }
    fn save_catalog(&self, catalog: &Catalog) -> Result<(), StorageError> {
        catalog.validate()?;
        let bytes = serde_json::to_vec(catalog).map_err(|_| StorageError::Corrupt)?;
        if bytes.len() > MAX_METADATA_BYTES {
            return Err(StorageError::ResourceLimit);
        }
        safe_directory(&self.root)?;
        let temporary = self.root.join("catalog.tmp");
        if safe_regular(&temporary)?.is_some() {
            fs::remove_file(&temporary).map_err(io_error)?;
        }
        write_new(&temporary, &bytes)?;
        let target = self.root.join("catalog.json");
        safe_regular(&target)?;
        if let Err(error) = fs::rename(&temporary, &target) {
            let _ = fs::remove_file(&temporary);
            return Err(io_error(error));
        }
        Ok(())
    }
    fn document(&self, record: &Record) -> Result<Document, StorageError> {
        let Some(file) = record.file else {
            return Ok(Document {
                version: 1,
                ..Document::default()
            });
        };
        let path = self.data_path(file);
        let metadata = safe_regular(&path)?.ok_or(StorageError::Corrupt)?;
        if metadata.len() != record.bytes as u64 {
            return Err(StorageError::Corrupt);
        }
        let document: Document = read_json(&path, MAX_NAMESPACE_BYTES)?;
        if document.version != 1
            || document.revision != record.revision
            || document.values.len() > MAX_KEYS
            || document.values.iter().any(|(key, entry)| {
                validate_key(key).is_err()
                    || validate_value(&entry.value).is_err()
                    || entry.revision == 0
                    || entry.revision > document.revision
            })
        {
            return Err(StorageError::Corrupt);
        }
        Ok(document)
    }
    fn clear_retired(&self, catalog: &mut Catalog) -> Result<bool, StorageError> {
        let mut pending = self.cleanup_pending(catalog).is_err();
        let namespaces: Vec<_> = catalog
            .namespaces
            .iter()
            .filter(|(_, record)| {
                matches!(
                    record.state,
                    NamespaceState::ClearPending | NamespaceState::CleanupOnly
                )
            })
            .map(|(namespace, _)| *namespace)
            .collect();
        for namespace in namespaces {
            let _namespace = self.namespace_lock(namespace)?;
            let record = catalog
                .namespaces
                .get(&namespace)
                .ok_or(StorageError::Corrupt)?;
            if let Some(file) = record.file {
                let path = self.data_path(file);
                // ClearPending is durable deletion authority. A missing managed file may
                // mean deletion succeeded before a crash/metadata-rename failure.
                if safe_regular(&path)?.is_some() {
                    self.document(record)?;
                    if fs::remove_file(path).map_err(io_error).is_err() {
                        pending = true;
                        continue;
                    }
                }
            }
            // All operations acquire the global lock first, so no competing namespace
            // waiter can exist here. Close the actual lock handle before deleting it.
            drop(_namespace);
            let lock_path = self.root.join(format!("{namespace}.lock"));
            if safe_regular(&lock_path)?.is_some()
                && fs::remove_file(lock_path).map_err(io_error).is_err()
            {
                pending = true;
                continue;
            }
            // Successful physical deletion plus failed metadata commit stays reserved;
            // ClearPending recovery may observe both managed files already absent.
            catalog.namespaces.remove(&namespace);
            self.save_catalog(catalog)?;
        }
        Ok(pending)
    }
    fn cleanup_pending(&self, catalog: &mut Catalog) -> Result<(), StorageError> {
        if let Some(file) = catalog.pending {
            safe_directory(&self.root)?;
            let path = self.data_path(file);
            if let Some(metadata) = safe_regular(&path)? {
                if metadata.len() > MAX_NAMESPACE_BYTES as u64 {
                    return Err(StorageError::Corrupt);
                }
                fs::remove_file(&path).map_err(|error| {
                    let _ = io_error(error);
                    StorageError::CleanupPending
                })?;
            }
            let mut next = Catalog {
                version: catalog.version,
                namespaces: catalog.namespaces.clone(),
                pending: None,
                restore: catalog.restore.clone(),
            };
            self.save_catalog(&next)?;
            std::mem::swap(catalog, &mut next);
        }
        Ok(())
    }
}
pub(crate) fn validate_key(key: &str) -> Result<(), StorageError> {
    if key.is_empty() || key.len() > 128 || key.chars().any(char::is_control) {
        Err(StorageError::InvalidParams)
    } else {
        Ok(())
    }
}
pub(crate) fn validate_value(value: &Value) -> Result<(), StorageError> {
    fn depth(value: &Value, level: usize) -> bool {
        if level > 16 {
            return false;
        }
        match value {
            Value::Array(items) => items.iter().all(|item| depth(item, level + 1)),
            Value::Object(items) => items.values().all(|item| depth(item, level + 1)),
            _ => true,
        }
    }
    if !depth(value, 0)
        || serde_json::to_vec(value)
            .map_err(|_| StorageError::InvalidParams)?
            .len()
            > MAX_VALUE_BYTES
    {
        Err(StorageError::InvalidParams)
    } else {
        Ok(())
    }
}
fn io_error(error: std::io::Error) -> StorageError {
    tracing::warn!(
        event = "extension.storage.io_failed",
        native_code = error.raw_os_error(),
        reason = "filesystem_operation_failed"
    );
    StorageError::Io
}
fn is_reparse(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
fn safe_directory(path: &Path) -> Result<(), StorageError> {
    let metadata = fs::symlink_metadata(path).map_err(io_error)?;
    if !metadata.is_dir() || is_reparse(&metadata) {
        return Err(StorageError::Corrupt);
    }
    Ok(())
}
fn safe_regular(path: &Path) -> Result<Option<fs::Metadata>, StorageError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !is_reparse(&metadata) => Ok(Some(metadata)),
        Ok(_) => Err(StorageError::Corrupt),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io_error(error)),
    }
}
fn lock_file(path: &Path) -> Result<File, StorageError> {
    safe_regular(path)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)
        .map_err(io_error)?;
    file.lock_exclusive().map_err(io_error)?;
    Ok(file)
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<(), StorageError> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(io_error)?;
    file.write_all(bytes).map_err(io_error)?;
    file.sync_all().map_err(io_error)?;
    Ok(())
}
fn read_json<T: DeserializeOwned>(path: &Path, limit: usize) -> Result<T, StorageError> {
    let metadata = safe_regular(path)?.ok_or(StorageError::Corrupt)?;
    if metadata.len() > limit as u64 {
        return Err(StorageError::Corrupt);
    }
    let file = File::open(path).map_err(io_error)?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(io_error)?;
    if bytes.len() > limit || bytes.len() as u64 != metadata.len() {
        return Err(StorageError::Corrupt);
    }
    let StrictValue(value) = serde_json::from_slice(&bytes).map_err(|_| StorageError::Corrupt)?;
    serde_json::from_value(value).map_err(|_| StorageError::Corrupt)
}
