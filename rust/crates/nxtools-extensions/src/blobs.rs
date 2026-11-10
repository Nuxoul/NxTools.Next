//! Invocation-bound temporary bytes; host paths never cross the script boundary.
//! Retired backing keeps byte/slot permits until the bounded IO cleanup actually succeeds.
use crate::InvocationIdentity;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex, OnceLock, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;

#[path = "blob_host_io.rs"]
mod host_io;
pub use host_io::HostBlobWriter;
#[path = "blob_recovery.rs"]
mod recovery;
#[cfg(test)]
#[path = "blob_shutdown_tests.rs"]
mod shutdown_tests;

pub const MAX_BLOB_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_RUN_BLOB_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_GLOBAL_BLOB_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_CHUNK_BYTES: usize = 64 * 1024;
const MAX_SLOTS: usize = 4;
const LEASE_AGE: Duration = Duration::from_secs(120);
const ACK_WAIT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum BlobError {
    #[error("invalid blob parameters")]
    InvalidParams,
    #[error("blob resource limit")]
    ResourceLimit,
    #[error("blob lease retired")]
    Stale,
    #[error("blob transfer invalid")]
    InvalidTransfer,
    #[error("blob IO failed")]
    Io,
}
impl BlobError {
    pub fn code(self) -> &'static str {
        match self {
            Self::InvalidParams => "invalid_params",
            Self::ResourceLimit => "resource_limit",
            Self::Stale => "blob_stale",
            Self::InvalidTransfer => "invalid_transfer",
            Self::Io => "blob_io_failed",
        }
    }
}
#[derive(Clone)]
pub struct BlobStore(Arc<Store>);
struct Store {
    bytes: Arc<Semaphore>,
    io: Arc<Semaphore>,
    slots: Arc<Semaphore>,
    root: Arc<Root>,
    runs: Mutex<Vec<Weak<Run>>>,
}
struct Cleanup {
    path: PathBuf,
    _bytes: OwnedSemaphorePermit,
    _slot: OwnedSemaphorePermit,
    _run_bytes: OwnedSemaphorePermit,
    _run_slot: OwnedSemaphorePermit,
    correlation_id: Uuid,
    run_id: Uuid,
    started: Instant,
    reported_pending: bool,
}
struct Root {
    path: PathBuf,
    pending: Mutex<Vec<Cleanup>>,
    cleanup_lock: Mutex<Option<recovery::OwnerLease>>,
    wake: Condvar,
    closing: AtomicBool,
    worker: Mutex<CleanupWorker>,
    joining: AtomicBool,
    completion: tokio::sync::watch::Sender<Option<Result<(), BlobError>>>,
    active: AtomicUsize,
    io: Arc<Semaphore>,
}
#[derive(Default)]
struct CleanupWorker {
    thread: Option<std::thread::JoinHandle<()>>,
    result: Option<Result<(), BlobError>>,
}
impl Drop for Store {
    fn drop(&mut self) {
        self.root.close();
    }
}
impl Root {
    fn close(&self) {
        let _pending = self
            .pending
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        self.closing.store(true, Ordering::Release);
        self.wake.notify_all();
    }
    fn ensure_worker(self: &Arc<Self>) -> Result<(), BlobError> {
        let mut worker = self.worker.lock().map_err(|_| BlobError::Io)?;
        if worker.thread.is_some() || worker.result.is_some() {
            return Ok(());
        }
        let root = self.clone();
        worker.thread = Some(
            std::thread::Builder::new()
                .name("extension-blob-cleanup".into())
                .spawn(move || {
                    let mut recovered = false;
                    loop {
                        let mut clean = false;
                        let permit = root.io.clone().try_acquire_owned().ok();
                        if permit.is_some() {
                            if !recovered {
                                recovery::recover(&root.path);
                                recovered = true;
                            }
                            clean = root.cleanup().is_ok();
                        }
                        let pending = root
                            .pending
                            .lock()
                            .unwrap_or_else(|poison| poison.into_inner());
                        if recovered
                            && clean
                            && root.closing.load(Ordering::Acquire)
                            && root.active.load(Ordering::Acquire) == 0
                            && pending.is_empty()
                        {
                            drop(pending);
                            // Admission is closed and no backing can create another file now.
                            if root.cleanup().is_ok() {
                                break;
                            }
                            continue;
                        }
                        drop(permit);
                        // Failed deletion and IO contention retain ownership and retry on this thread.
                        if recovered && clean && pending.is_empty() {
                            drop(root.wake.wait(pending));
                        } else {
                            drop(root.wake.wait_timeout(pending, Duration::from_millis(250)));
                        }
                    }
                })
                .map_err(|_| BlobError::Io)?,
        );
        Ok(())
    }
    fn join(self: &Arc<Self>) -> Result<(), BlobError> {
        self.ensure_worker()?;
        // Serialize all callers through the same join, including the complete-result handoff.
        let mut worker = self.worker.lock().map_err(|_| BlobError::Io)?;
        if let Some(thread) = worker.thread.take() {
            worker.result = Some(thread.join().map_err(|_| BlobError::Io));
        }
        worker.result.unwrap_or(Err(BlobError::Io))
    }
    fn cleanup(&self) -> Result<(), BlobError> {
        let mut lease = self.cleanup_lock.lock().map_err(|_| BlobError::Io)?;
        let pending = std::mem::take(&mut *self.pending.lock().map_err(|_| BlobError::Io)?);
        let mut retained = Vec::new();
        for mut item in pending {
            let result = (|| {
                if item.path.parent() != Some(self.path.as_path()) {
                    return Err(BlobError::Io);
                }
                match std::fs::symlink_metadata(&self.path) {
                    Ok(metadata) if metadata.is_dir() && !is_reparse(&metadata) => {}
                    Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                    _ => return Err(BlobError::Io),
                }
                match std::fs::symlink_metadata(&item.path) {
                    Ok(metadata) if metadata.is_file() && !is_reparse(&metadata) => {}
                    Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(()),
                    _ => return Err(BlobError::Io),
                }
                std::fs::remove_file(&item.path).map_err(|_| BlobError::Io)
            })();
            if result.is_err() {
                if !item.reported_pending {
                    tracing::warn!(event="extension.blob.cleanup_pending",correlation_id=%item.correlation_id,run_id=%item.run_id,reason="backing_delete_failed",elapsed_ms=item.started.elapsed().as_millis(),"temporary blob physical cleanup pending");
                    item.reported_pending = true;
                }
                retained.push(item);
            } else {
                tracing::info!(event="extension.blob.terminal",correlation_id=%item.correlation_id,run_id=%item.run_id,success=true,cleanup_complete=true,elapsed_ms=item.started.elapsed().as_millis(),"temporary blob backing cleanup completed");
            }
        }
        let failed = !retained.is_empty();
        self.pending
            .lock()
            .map_err(|_| BlobError::Io)?
            .extend(retained);
        if failed {
            return Err(BlobError::Io);
        }
        if self.active.load(Ordering::Acquire) == 0
            && self.pending.lock().map_err(|_| BlobError::Io)?.is_empty()
        {
            if lease.is_some() {
                match std::fs::remove_file(self.path.join(recovery::OWNER)) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(_) => return Err(BlobError::Io),
                }
                drop(lease.take());
            }
            match std::fs::remove_dir(&self.path) {
                Ok(()) => {}
                Err(failure) if matches!(failure.kind(), std::io::ErrorKind::NotFound) => {}
                Err(_) => return Err(BlobError::Io),
            }
        }
        Ok(())
    }
}
fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt as _;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
impl Default for BlobStore {
    fn default() -> Self {
        Self::new(std::env::temp_dir().join(format!("nxtools-extension-blobs-{}", Uuid::new_v4())))
    }
}
impl BlobStore {
    /// Composition supplies a fresh host-owned root, never an extension parameter.
    pub fn new(root: PathBuf) -> Self {
        let io = Arc::new(Semaphore::new(4));
        let store = Self(Arc::new(Store {
            bytes: Arc::new(Semaphore::new(MAX_GLOBAL_BLOB_BYTES)),
            io: io.clone(),
            slots: Arc::new(Semaphore::new(256)),
            runs: Mutex::new(Vec::new()),
            root: Arc::new(Root {
                path: root,
                pending: Mutex::new(Vec::new()),
                cleanup_lock: Mutex::new(None),
                wake: Condvar::new(),
                closing: AtomicBool::new(false),
                worker: Mutex::new(CleanupWorker::default()),
                joining: AtomicBool::new(false),
                completion: tokio::sync::watch::channel(None).0,
                active: AtomicUsize::new(0),
                io,
            }),
        }));
        // Startup recovery runs on the same finite IO owner, even when this instance has no blobs.
        let _ = store.0.root.ensure_worker();
        store
    }
    pub fn invocation(&self, identity: InvocationIdentity, session_id: Uuid) -> BlobRun {
        let mut runs = self
            .0
            .runs
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        runs.retain(|run| run.strong_count() != 0);
        let run = BlobRun(Arc::new(Run {
            store: self.clone(),
            identity,
            session_id,
            retired: AtomicBool::new(self.0.root.closing.load(Ordering::Acquire)),
            bytes: Arc::new(Semaphore::new(MAX_RUN_BLOB_BYTES)),
            slots: Arc::new(Semaphore::new(MAX_SLOTS)),
            in_flight: AtomicUsize::new(0),
            state: Mutex::new(RunState {
                blobs: HashMap::new(),
                downloads: HashMap::new(),
                transfers: 0,
                bytes: 0,
            }),
        }));
        runs.push(Arc::downgrade(&run.0));
        run
    }
    pub fn available_bytes(&self) -> usize {
        self.0.bytes.available_permits()
    }
    pub(crate) fn close(&self) {
        self.0.root.close();
        let mut runs = self
            .0
            .runs
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        for run in runs.drain(..).filter_map(|run| run.upgrade()) {
            BlobRun(run).retire();
        }
    }
    /// Close admission, retire live runs, and wait for the original physical cleanup owner.
    pub async fn shutdown(&self) -> Result<(), BlobError> {
        self.close();
        let root = self.0.root.clone();
        let mut completion = root.completion.subscribe();
        if !root.joining.swap(true, Ordering::AcqRel) {
            let correlation_id = Uuid::new_v4();
            let started = Instant::now();
            tracing::info!(event="extension.blob.shutdown_accepted", %correlation_id);
            // One shared join leaves blocking workers and all four IO permits available.
            tokio::task::spawn_blocking(move || {
                let result = root.join();
                tracing::info!(event="extension.blob.shutdown_terminal", %correlation_id, success=result.is_ok(), elapsed_ms=started.elapsed().as_millis());
                root.completion.send_replace(Some(result));
            });
        }
        loop {
            if let Some(result) = *completion.borrow_and_update() {
                return result;
            }
            completion.changed().await.map_err(|_| BlobError::Io)?;
        }
    }
    pub async fn cleanup(&self) -> Result<(), BlobError> {
        let permit = self
            .0
            .io
            .clone()
            .try_acquire_owned()
            .map_err(|_| BlobError::ResourceLimit)?;
        let root = self.0.root.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            root.cleanup()
        })
        .await
        .map_err(|_| BlobError::Io)?
    }
}
#[derive(Clone)]
pub struct BlobRun(Arc<Run>);
struct Run {
    store: BlobStore,
    identity: InvocationIdentity,
    session_id: Uuid,
    retired: AtomicBool,
    state: Mutex<RunState>,
    bytes: Arc<Semaphore>,
    slots: Arc<Semaphore>,
    in_flight: AtomicUsize,
}
struct IoLease(Arc<Run>);
impl Drop for IoLease {
    fn drop(&mut self) {
        self.0.in_flight.fetch_sub(1, Ordering::AcqRel);
    }
}
struct RunState {
    blobs: HashMap<Uuid, Arc<Backing>>,
    downloads: HashMap<Uuid, Download>,
    transfers: usize,
    bytes: usize,
}
struct Backing {
    id: Uuid,
    upload: Uuid,
    length: usize,
    media_type: String,
    sha256: String,
    host_owned: bool,
    completed: OnceLock<BlobDescriptor>,
    created: Instant,
    file: Mutex<BackingState>,
    bytes: Option<OwnedSemaphorePermit>,
    slot: Option<OwnedSemaphorePermit>,
    run_bytes: Option<OwnedSemaphorePermit>,
    run_slot: Option<OwnedSemaphorePermit>,
    root: Arc<Root>,
    path: PathBuf,
    created_file: AtomicBool,
    sealed: AtomicBool,
    correlation_id: Uuid,
    run_id: Uuid,
}
struct BackingState {
    file: Option<File>,
    offset: usize,
    sequence: u64,
    digest: Sha256,
    sealed: bool,
}
impl Drop for Backing {
    fn drop(&mut self) {
        let state = self
            .file
            .get_mut()
            .unwrap_or_else(|poison| poison.into_inner());
        drop(state.file.take());
        let mut pending = self
            .root
            .pending
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        if self.created_file.load(Ordering::Acquire)
            && let (Some(bytes), Some(slot), Some(run_bytes), Some(run_slot)) = (
                self.bytes.take(),
                self.slot.take(),
                self.run_bytes.take(),
                self.run_slot.take(),
            )
        {
            pending.push(Cleanup {
                path: self.path.clone(),
                _bytes: bytes,
                _slot: slot,
                _run_bytes: run_bytes,
                _run_slot: run_slot,
                correlation_id: self.correlation_id,
                run_id: self.run_id,
                started: self.created,
                reported_pending: false,
            });
        }
        self.root.active.fetch_sub(1, Ordering::AcqRel);
        self.root.wake.notify_all();
    }
}
struct Download {
    backing: Arc<Backing>,
    next: usize,
    end: usize,
    sequence: u64,
    pending: Option<(u64, usize)>,
    last_sent: Instant,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlobDescriptor {
    pub blob_id: Uuid,
    pub length: usize,
    pub media_type: String,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct BlobUpload {
    #[serde(flatten)]
    pub blob: BlobDescriptor,
    pub transfer_id: Uuid,
}
#[derive(Debug, Clone, Serialize)]
pub struct BlobRead {
    pub transfer_id: Uuid,
    pub length: usize,
}
#[derive(Debug, Clone)]
pub struct BlobChunk {
    pub transfer_id: Uuid,
    pub sequence: u64,
    pub offset: usize,
    pub data: String,
}
impl Backing {
    fn length(&self) -> usize {
        self.completed.get().map_or(self.length, |blob| blob.length)
    }
    fn descriptor(&self) -> BlobDescriptor {
        if let Some(blob) = self.completed.get() {
            return blob.clone();
        }
        BlobDescriptor {
            blob_id: self.id,
            length: self.length,
            media_type: self.media_type.clone(),
            sha256: self.sha256.clone(),
        }
    }
}
impl BlobRun {
    pub fn check(&self, current: &InvocationIdentity, session: Uuid) -> Result<(), BlobError> {
        if self.0.retired.load(Ordering::Acquire)
            || self.0.store.0.root.closing.load(Ordering::Acquire)
            || current != &self.0.identity
            || session != self.0.session_id
        {
            Err(BlobError::Stale)
        } else {
            Ok(())
        }
    }
    fn current(&self) -> Result<(), BlobError> {
        self.check(&self.0.identity, self.0.session_id)
    }
    async fn io<T: Send + 'static>(
        &self,
        operation: impl FnOnce() -> Result<T, BlobError> + Send + 'static,
    ) -> Result<T, BlobError> {
        self.current()?;
        let permit = self
            .0
            .store
            .0
            .io
            .clone()
            .try_acquire_owned()
            .map_err(|_| BlobError::ResourceLimit)?;
        self.0.in_flight.fetch_add(1, Ordering::AcqRel);
        let lease = IoLease(self.0.clone());
        let result = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let _lease = lease;
            if _lease.0.retired.load(Ordering::Acquire) {
                return Err(BlobError::Stale);
            }
            operation()
        })
        .await
        .map_err(|_| BlobError::Io)??;
        self.current()?;
        Ok(result)
    }
    fn backing(&self, id: Uuid) -> Result<Arc<Backing>, BlobError> {
        self.current()?;
        let backing = self
            .0
            .state
            .lock()
            .map_err(|_| BlobError::Io)?
            .blobs
            .get(&id)
            .cloned()
            .ok_or(BlobError::Stale)?;
        if backing.created.elapsed() > LEASE_AGE {
            return Err(BlobError::Stale);
        }
        Ok(backing)
    }
    pub fn retire(&self) {
        self.0.retired.store(true, Ordering::Release);
        if let Ok(mut state) = self.0.state.lock() {
            state.downloads.clear();
            state.blobs.clear();
            state.bytes = 0;
        }
    }
    pub fn quiescent(&self) -> bool {
        self.0.in_flight.load(Ordering::Acquire) == 0
            && self.0.state.lock().is_ok_and(|state| {
                state.downloads.is_empty()
                    && state
                        .blobs
                        .values()
                        .all(|backing| backing.sealed.load(Ordering::Acquire))
            })
    }
    pub fn ack_expired(&self) -> bool {
        self.0.state.lock().is_ok_and(|state| {
            state.downloads.values().any(|download| {
                download.pending.is_some() && download.last_sent.elapsed() >= ACK_WAIT
            })
        })
    }
    fn reserve_backing(
        &self,
        length: usize,
        media_type: String,
        sha256: String,
        host_owned: bool,
    ) -> Result<Arc<Backing>, BlobError> {
        self.current()?;
        if length > MAX_BLOB_BYTES
            || media_type.is_empty()
            || media_type.len() > 128
            || media_type.chars().any(char::is_control)
            || sha256.len() != 64
            || !sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(BlobError::InvalidParams);
        }
        let bytes = self
            .0
            .store
            .0
            .bytes
            .clone()
            .try_acquire_many_owned(length as u32)
            .map_err(|_| BlobError::ResourceLimit)?;
        let slot = self
            .0
            .store
            .0
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| BlobError::ResourceLimit)?;
        let id = Uuid::new_v4();
        let run_bytes = self
            .0
            .bytes
            .clone()
            .try_acquire_many_owned(length as u32)
            .map_err(|_| BlobError::ResourceLimit)?;
        let run_slot = self
            .0
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| BlobError::ResourceLimit)?;
        let upload = Uuid::new_v4();
        let root = self.0.store.0.root.clone();
        let path = root.path.join(id.to_string());
        root.ensure_worker()?;
        {
            let _pending = root.pending.lock().map_err(|_| BlobError::Io)?;
            self.current()?;
            root.active.fetch_add(1, Ordering::AcqRel);
        }
        let backing = Arc::new(Backing {
            id,
            upload,
            length,
            media_type,
            sha256,
            host_owned,
            completed: OnceLock::new(),
            created: Instant::now(),
            correlation_id: self.0.identity.correlation_id,
            run_id: self.0.identity.run_id,
            file: Mutex::new(BackingState {
                file: None,
                offset: 0,
                sequence: 0,
                digest: Sha256::new(),
                sealed: false,
            }),
            bytes: Some(bytes),
            slot: Some(slot),
            run_bytes: Some(run_bytes),
            run_slot: Some(run_slot),
            root: root.clone(),
            path: path.clone(),
            created_file: AtomicBool::new(false),
            sealed: AtomicBool::new(false),
        });
        {
            let mut state = self.0.state.lock().map_err(|_| BlobError::Io)?;
            self.current()?;
            if state.blobs.len() >= MAX_SLOTS
                || state.transfers >= MAX_SLOTS
                || state
                    .bytes
                    .checked_add(length)
                    .is_none_or(|total| total > MAX_RUN_BLOB_BYTES)
            {
                return Err(BlobError::ResourceLimit);
            }
            state.transfers += 1;
            state.bytes += length;
            state.blobs.insert(id, backing.clone());
        }
        Ok(backing)
    }
    async fn open_backing(&self, backing: Arc<Backing>) -> Result<(), BlobError> {
        let root = backing.root.clone();
        let path = backing.path.clone();
        let retained = backing;
        self.io(move || {
            let mut lease = root.cleanup_lock.lock().map_err(|_| BlobError::Io)?;
            if root.closing.load(Ordering::Acquire) {
                return Err(BlobError::Stale);
            }
            match std::fs::create_dir(&root.path) {
                Ok(()) => {}
                Err(failure) if failure.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => return Err(BlobError::Io),
            }
            let metadata = std::fs::symlink_metadata(&root.path).map_err(|_| BlobError::Io)?;
            if !metadata.is_dir() || is_reparse(&metadata) {
                return Err(BlobError::Io);
            }
            recovery::create_lease(&root.path, &mut lease)?;
            let file = OpenOptions::new()
                .create_new(true)
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|_| BlobError::Io)?;
            retained.created_file.store(true, Ordering::Release);
            retained.file.lock().map_err(|_| BlobError::Io)?.file = Some(file);
            Ok(())
        })
        .await
    }
    pub async fn begin(
        &self,
        length: usize,
        media_type: String,
        sha256: String,
    ) -> Result<BlobUpload, BlobError> {
        let backing = self.reserve_backing(length, media_type, sha256, false)?;
        let id = backing.id;
        let upload = backing.upload;
        let result = self.open_backing(backing.clone()).await;
        if let Err(failure) = result {
            let _ = self.release(id).await;
            return Err(failure);
        }
        tracing::info!(event="extension.blob.accepted",correlation_id=%self.0.identity.correlation_id,run_id=%self.0.identity.run_id,byte_count=length,"temporary blob admitted");
        Ok(BlobUpload {
            blob: backing.descriptor(),
            transfer_id: upload,
        })
    }
    pub async fn upload(
        &self,
        transfer_id: Uuid,
        sequence: u64,
        offset: usize,
        data: String,
    ) -> Result<usize, BlobError> {
        self.current()?;
        if data.len() > MAX_CHUNK_BYTES.div_ceil(3) * 4 {
            return Err(BlobError::InvalidTransfer);
        }
        let backing = self
            .0
            .state
            .lock()
            .map_err(|_| BlobError::Io)?
            .blobs
            .values()
            .find(|backing| backing.upload == transfer_id)
            .cloned()
            .ok_or(BlobError::Stale)?;
        if backing.created.elapsed() > LEASE_AGE {
            return Err(BlobError::Stale);
        }
        self.io(move || {
            let chunk = STANDARD
                .decode(data.as_bytes())
                .map_err(|_| BlobError::InvalidTransfer)?;
            if chunk.is_empty() || chunk.len() > MAX_CHUNK_BYTES {
                return Err(BlobError::InvalidTransfer);
            }
            let mut state = backing.file.lock().map_err(|_| BlobError::Io)?;
            if backing.host_owned
                || state.sealed
                || sequence != state.sequence + 1
                || offset != state.offset
                || offset
                    .checked_add(chunk.len())
                    .is_none_or(|end| end > backing.length)
            {
                return Err(BlobError::InvalidTransfer);
            }
            state
                .file
                .as_mut()
                .ok_or(BlobError::Io)?
                .write_all(&chunk)
                .map_err(|_| BlobError::Io)?;
            state.digest.update(&chunk);
            state.offset += chunk.len();
            state.sequence = sequence;
            Ok(state.offset)
        })
        .await
    }
    pub async fn seal(&self, id: Uuid) -> Result<BlobDescriptor, BlobError> {
        let backing = self.backing(id)?;
        self.io(move || {
            let mut state = backing.file.lock().map_err(|_| BlobError::Io)?;
            if backing.host_owned
                || state.sealed
                || state.offset != backing.length
                || format!("{:x}", state.digest.clone().finalize()) != backing.sha256
            {
                return Err(BlobError::InvalidTransfer);
            }
            state
                .file
                .as_mut()
                .ok_or(BlobError::Io)?
                .flush()
                .map_err(|_| BlobError::Io)?;
            state.sealed = true;
            backing.sealed.store(true, Ordering::Release);
            Ok(backing.descriptor())
        })
        .await
    }
    pub fn read(
        &self,
        id: Uuid,
        offset: usize,
        length: Option<usize>,
    ) -> Result<BlobRead, BlobError> {
        let backing = self.backing(id)?;
        let actual_length = backing.length();
        let length = length.unwrap_or_else(|| actual_length.saturating_sub(offset));
        if offset > actual_length
            || length > MAX_BLOB_BYTES
            || offset
                .checked_add(length)
                .is_none_or(|end| end > actual_length)
        {
            return Err(BlobError::InvalidParams);
        }
        if !backing.sealed.load(Ordering::Acquire) {
            return Err(BlobError::InvalidTransfer);
        }
        let mut state = self.0.state.lock().map_err(|_| BlobError::Io)?;
        self.current()?;
        if state.downloads.len() >= MAX_SLOTS || state.transfers >= MAX_SLOTS {
            return Err(BlobError::ResourceLimit);
        }
        state.transfers += 1;
        let transfer_id = Uuid::new_v4();
        if length > 0 {
            state.downloads.insert(
                transfer_id,
                Download {
                    backing,
                    next: offset,
                    end: offset + length,
                    sequence: 0,
                    pending: None,
                    last_sent: Instant::now(),
                },
            );
        }
        Ok(BlobRead {
            transfer_id,
            length,
        })
    }
    pub async fn next_chunk(&self, transfer_id: Uuid) -> Result<Option<BlobChunk>, BlobError> {
        self.current()?;
        let (backing, offset, length, sequence) = {
            let mut state = self.0.state.lock().map_err(|_| BlobError::Io)?;
            let Some(download) = state.downloads.get_mut(&transfer_id) else {
                return Ok(None);
            };
            if download.pending.is_some() {
                return Err(BlobError::InvalidTransfer);
            }
            if download.backing.created.elapsed() > LEASE_AGE {
                return Err(BlobError::Stale);
            }
            let length = (download.end - download.next).min(MAX_CHUNK_BYTES);
            let sequence = download.sequence + 1;
            download.pending = Some((sequence, download.next + length));
            download.last_sent = Instant::now();
            (download.backing.clone(), download.next, length, sequence)
        };
        let data = self
            .io(move || {
                let mut state = backing.file.lock().map_err(|_| BlobError::Io)?;
                let file = state.file.as_mut().ok_or(BlobError::Io)?;
                file.seek(SeekFrom::Start(offset as u64))
                    .map_err(|_| BlobError::Io)?;
                let mut bytes = vec![0; length];
                file.read_exact(&mut bytes).map_err(|_| BlobError::Io)?;
                Ok(STANDARD.encode(bytes))
            })
            .await?;
        Ok(Some(BlobChunk {
            transfer_id,
            sequence,
            offset,
            data,
        }))
    }
    pub fn acknowledge(
        &self,
        transfer_id: Uuid,
        sequence: u64,
        offset: usize,
    ) -> Result<bool, BlobError> {
        self.current()?;
        let mut state = self.0.state.lock().map_err(|_| BlobError::Io)?;
        let download = state
            .downloads
            .get_mut(&transfer_id)
            .ok_or(BlobError::Stale)?;
        if download.backing.created.elapsed() > LEASE_AGE {
            return Err(BlobError::Stale);
        }
        if download.pending != Some((sequence, offset)) || download.last_sent.elapsed() >= ACK_WAIT
        {
            return Err(BlobError::InvalidTransfer);
        }
        download.pending = None;
        download.sequence = sequence;
        download.next = offset;
        let more = offset != download.end;
        if !more {
            state.downloads.remove(&transfer_id);
        }
        Ok(more)
    }
    pub(crate) fn upload_id(&self, id: Uuid) -> Result<Option<Uuid>, BlobError> {
        self.current()?;
        let state = self.0.state.lock().map_err(|_| BlobError::Io)?;
        if state
            .downloads
            .values()
            .any(|download| download.backing.id == id)
        {
            return Err(BlobError::InvalidTransfer);
        }
        state
            .blobs
            .get(&id)
            .map(|backing| (!backing.host_owned).then_some(backing.upload))
            .ok_or(BlobError::Stale)
    }
    pub async fn release(&self, id: Uuid) -> Result<(), BlobError> {
        self.current()?;
        let backing = {
            let mut state = self.0.state.lock().map_err(|_| BlobError::Io)?;
            if state
                .downloads
                .values()
                .any(|download| download.backing.id == id)
            {
                return Err(BlobError::InvalidTransfer);
            }
            let backing = state.blobs.remove(&id).ok_or(BlobError::Stale)?;
            state.bytes -= backing.length;
            backing
        };
        drop(backing);
        self.0.store.cleanup().await
    }
}

#[cfg(test)]
mod lifecycle_tests {
    use super::*;
    #[tokio::test]
    async fn expansion_blob_cancelled_waiter_keeps_actual_io_and_backing_reservations() {
        let root = tempfile::tempdir().expect("owned delayed IO root");
        let store = BlobStore::new(root.path().join("backing"));
        let identity = InvocationIdentity {
            run_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            generation: 1,
            package_id: "fixture.io".into(),
            handler_id: "run".into(),
            content_hash: "a".repeat(64),
        };
        let run = store.invocation(identity, Uuid::new_v4());
        let upload = run
            .begin(
                3,
                "text/plain".into(),
                format!("{:x}", Sha256::digest(b"abc")),
            )
            .await
            .expect("begin");
        let backing = run
            .backing(upload.blob.blob_id)
            .expect("owned backing lease");
        let (entered, arrival) = tokio::sync::oneshot::channel();
        let (release, wait) = std::sync::mpsc::channel();
        let retained = run.clone();
        let task = tokio::spawn(async move {
            retained
                .io(move || {
                    let _file = backing.file.lock().expect("actual file mutex");
                    entered.send(()).expect("entered");
                    wait.recv().expect("release actual blocking IO");
                    Ok(())
                })
                .await
        });
        arrival.await.expect("blocking owner entered");
        assert!(!run.quiescent(), "must not block waiting for the IO mutex");
        run.retire();
        task.abort();
        let _ = task.await;
        store.cleanup().await.expect("no finished backing yet");
        assert_eq!(
            store.available_bytes(),
            MAX_GLOBAL_BLOB_BYTES - 3,
            "cancel is not IO completion"
        );
        assert!(!run.quiescent());
        release.send(()).expect("finish actual IO");
        tokio::time::timeout(Duration::from_secs(3), async {
            while store.available_bytes() != MAX_GLOBAL_BLOB_BYTES {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("late last-lease drop triggers physical cleanup");
        assert!(run.quiescent());
    }
    #[tokio::test]
    async fn expansion_blob_expired_download_and_stale_ack_remain_unavailable() {
        let root = tempfile::tempdir().expect("owned expiry root");
        let store = BlobStore::new(root.path().join("backing"));
        let identity = InvocationIdentity {
            run_id: Uuid::new_v4(),
            correlation_id: Uuid::new_v4(),
            generation: 1,
            package_id: "fixture.expiry".into(),
            handler_id: "run".into(),
            content_hash: "a".repeat(64),
        };
        let run = store.invocation(identity, Uuid::new_v4());
        let upload = run
            .begin(
                3,
                "text/plain".into(),
                format!("{:x}", Sha256::digest(b"abc")),
            )
            .await
            .expect("begin");
        run.upload(upload.transfer_id, 1, 0, STANDARD.encode(b"abc"))
            .await
            .expect("upload");
        run.seal(upload.blob.blob_id).await.expect("seal");
        let span = run.read(upload.blob.blob_id, 0, None).expect("download");
        run.next_chunk(span.transfer_id).await.expect("one block");
        run.0
            .state
            .lock()
            .expect("state")
            .downloads
            .get_mut(&span.transfer_id)
            .expect("download")
            .last_sent = Instant::now() - ACK_WAIT;
        assert!(run.ack_expired());
        assert_eq!(
            run.acknowledge(span.transfer_id, 1, 3),
            Err(BlobError::InvalidTransfer)
        );
        {
            let mut state = run.0.state.lock().expect("state");
            state.downloads.clear();
            Arc::get_mut(state.blobs.get_mut(&upload.blob.blob_id).expect("backing"))
                .expect("only owned backing")
                .created = Instant::now() - LEASE_AGE - Duration::from_secs(1);
        }
        assert!(matches!(
            run.read(upload.blob.blob_id, 0, None),
            Err(BlobError::Stale)
        ));
        assert!(matches!(
            run.seal(upload.blob.blob_id).await,
            Err(BlobError::Stale)
        ));
        run.retire();
        store.cleanup().await.expect("actual expiry cleanup");
        assert_eq!(store.available_bytes(), MAX_GLOBAL_BLOB_BYTES);
    }
}
