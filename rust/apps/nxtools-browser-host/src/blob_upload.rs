use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use tracing::{info, warn};
use uuid::Uuid;

pub(crate) const MAX_BLOB_UPLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;
pub(crate) const MAX_BLOB_CHUNK_BYTES: u64 = 1024 * 1024;
pub(crate) const MAX_PENDING_BLOB_UPLOADS: usize = 4;
const MAX_ACTIVE_BLOB_UPLOADS: usize = 2;
const MAX_BLOB_CONNECTIONS: usize = 8;
const MAX_HTTP_HEADER_BYTES: usize = 8 * 1024;
const CAPABILITY_TTL: Duration = Duration::from_secs(2 * 60);

#[derive(Debug, Clone)]
pub(crate) struct BlobUploadCapability {
    pub id: Uuid,
    pub path: String,
    pub expires_after_ms: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct BlobCommitCandidate {
    pub path: PathBuf,
    pub registered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UploadState {
    Pending,
    Uploading,
    Receiving,
    Complete,
}

#[derive(Debug)]
struct PendingUpload {
    token: String,
    correlation_id: Uuid,
    handoff_id: Uuid,
    expected_size: u64,
    started_at: Instant,
    expires_at: Instant,
    state: UploadState,
    received: u64,
}

#[derive(Debug, Default)]
struct UploadRegistry {
    pending: HashMap<Uuid, PendingUpload>,
    active: usize,
}

pub(crate) struct BlobUploadService {
    address: SocketAddr,
    staging_root: PathBuf,
    registry: Arc<Mutex<UploadRegistry>>,
    stop: Arc<std::sync::atomic::AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl BlobUploadService {
    pub(crate) fn start(staging_root: impl AsRef<Path>) -> Result<Self> {
        let staging_root = staging_root.as_ref().to_path_buf();
        fs::create_dir_all(&staging_root)
            .with_context(|| format!("failed to create {}", staging_root.display()))?;
        cleanup_stale_staging_files(&staging_root);
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .context("failed to bind the bounded Blob upload listener")?;
        listener
            .set_nonblocking(true)
            .context("failed to configure the Blob upload listener")?;
        let address = listener.local_addr()?;
        let registry = Arc::new(Mutex::new(UploadRegistry::default()));
        let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_registry = Arc::clone(&registry);
        let worker_stop = Arc::clone(&stop);
        let worker_root = staging_root.clone();
        let connections = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let worker_connections = Arc::clone(&connections);
        let worker = std::thread::Builder::new()
            .name("nxtools-browser-blob-upload".into())
            .spawn(move || {
                while !worker_stop.load(std::sync::atomic::Ordering::Acquire) {
                    match listener.accept() {
                        Ok((stream, peer)) if peer.ip().is_loopback() => {
                            if worker_connections
                                .fetch_update(
                                    std::sync::atomic::Ordering::AcqRel,
                                    std::sync::atomic::Ordering::Acquire,
                                    |count| (count < MAX_BLOB_CONNECTIONS).then_some(count + 1),
                                )
                                .is_err()
                            {
                                continue;
                            }
                            let registry = Arc::clone(&worker_registry);
                            let staging_root = worker_root.clone();
                            let connections = Arc::clone(&worker_connections);
                            let spawn_result = std::thread::Builder::new()
                                .name("nxtools-browser-blob-connection".into())
                                .spawn(move || {
                                    handle_connection(stream, &staging_root, &registry);
                                    connections.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
                                });
                            if spawn_result.is_err() {
                                worker_connections
                                    .fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
                            }
                        }
                        Ok((_stream, _peer)) => {}
                        Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(15));
                        }
                        Err(_) => break,
                    }
                }
            })
            .context("failed to start the Blob upload listener")?;
        Ok(Self {
            address,
            staging_root,
            registry,
            stop,
            worker: Some(worker),
        })
    }

    pub(crate) const fn address(&self) -> SocketAddr {
        self.address
    }

    pub(crate) fn prepare(
        &self,
        expected_size: u64,
        correlation_id: Uuid,
        handoff_id: Uuid,
    ) -> Result<BlobUploadCapability> {
        if expected_size == 0 || expected_size > MAX_BLOB_UPLOAD_BYTES {
            bail!("Blob size is outside the bounded upload limit");
        }
        let now = Instant::now();
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| anyhow::anyhow!("Blob upload registry is unavailable"))?;
        reap_expired_uploads(&mut registry, &self.staging_root, now);
        if registry.pending.len() >= MAX_PENDING_BLOB_UPLOADS {
            bail!("too many Blob uploads are pending");
        }
        let id = Uuid::new_v4();
        let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        registry.pending.insert(
            id,
            PendingUpload {
                token: token.clone(),
                correlation_id,
                handoff_id,
                expected_size,
                started_at: now,
                expires_at: now + CAPABILITY_TTL,
                state: UploadState::Pending,
                received: 0,
            },
        );
        Ok(BlobUploadCapability {
            id,
            path: format!("/nxtools/blob/{id}/{token}"),
            expires_after_ms: CAPABILITY_TTL.as_millis() as u64,
        })
    }

    pub(crate) fn commit_candidate(
        &self,
        id: Uuid,
        expected_size: u64,
        correlation_id: Uuid,
        handoff_id: Uuid,
    ) -> Result<BlobCommitCandidate> {
        let now = Instant::now();
        let mut registry = self
            .registry
            .lock()
            .map_err(|_| anyhow::anyhow!("Blob upload registry is unavailable"))?;
        reap_expired_uploads(&mut registry, &self.staging_root, now);
        let registered = if let Some(upload) = registry.pending.get(&id) {
            if upload.state != UploadState::Complete
                || upload.expected_size != expected_size
                || upload.correlation_id != correlation_id
                || upload.handoff_id != handoff_id
            {
                bail!("Blob upload is incomplete or does not match its commit identity");
            }
            true
        } else {
            false
        };
        let path = completed_path(&self.staging_root, id);
        if let Ok(metadata) = fs::metadata(&path)
            && metadata.len() != expected_size
        {
            bail!("staged Blob size does not match its commit metadata");
        }
        Ok(BlobCommitCandidate { path, registered })
    }

    pub(crate) fn forget(&self, id: Uuid) {
        let Ok(mut registry) = self.registry.lock() else {
            return;
        };
        if let Some(upload) = registry.pending.remove(&id)
            && matches!(
                upload.state,
                UploadState::Uploading | UploadState::Receiving
            )
        {
            registry.active = registry.active.saturating_sub(1);
        }
    }
}

impl Drop for BlobUploadService {
    fn drop(&mut self) {
        self.stop.store(true, std::sync::atomic::Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn handle_connection(
    mut stream: TcpStream,
    staging_root: &Path,
    registry: &Arc<Mutex<UploadRegistry>>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(5)));
    let result = receive_upload(&mut stream, staging_root, registry);
    let (status, reason) = match result {
        Ok(()) => (204, "No Content"),
        Err(UploadFailure::NotFound) => (404, "Not Found"),
        Err(UploadFailure::Busy) => (429, "Too Many Requests"),
        Err(UploadFailure::Invalid) => (400, "Bad Request"),
        Err(UploadFailure::Io) => (500, "Internal Server Error"),
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-store\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UploadFailure {
    Invalid,
    NotFound,
    Busy,
    Io,
}

fn receive_upload(
    stream: &mut TcpStream,
    staging_root: &Path,
    registry: &Arc<Mutex<UploadRegistry>>,
) -> Result<(), UploadFailure> {
    let header = read_header(stream)?;
    let header_text = std::str::from_utf8(&header).map_err(|_| UploadFailure::Invalid)?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines.next().ok_or(UploadFailure::Invalid)?;
    let mut request = request_line.split_whitespace();
    if request.next() != Some("POST") {
        return Err(UploadFailure::NotFound);
    }
    let path = request.next().ok_or(UploadFailure::Invalid)?;
    if request.next() != Some("HTTP/1.1") || request.next().is_some() {
        return Err(UploadFailure::Invalid);
    }
    let content_length = lines
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<u64>().ok())
                .flatten()
        })
        .ok_or(UploadFailure::Invalid)?;
    let (id, token, offset) = parse_capability_path(path).ok_or(UploadFailure::NotFound)?;
    {
        let mut state = registry.lock().map_err(|_| UploadFailure::Io)?;
        reap_expired_uploads(&mut state, staging_root, Instant::now());
    }
    let expected_size = match claim_upload(registry, id, token, offset, content_length) {
        Ok(expected_size) => expected_size,
        Err(error) => {
            drain_small_rejected_body(stream, content_length);
            return Err(error);
        }
    };
    let temporary = staging_root.join(format!("{id}.uploading"));
    let completed = completed_path(staging_root, id);
    let write_result = write_chunk_body(stream, &temporary, offset, content_length);
    let mut state = registry.lock().map_err(|_| UploadFailure::Io)?;
    if let Err(failure) = write_result {
        drain_claimed_body(stream, failure.unread_bytes);
        state.active = state.active.saturating_sub(1);
        let upload = state.pending.remove(&id).ok_or(UploadFailure::Io)?;
        warn!(
            event = "browser_native_host.blob_upload_failed",
            correlation_id = %upload.correlation_id,
            handoff_id = %upload.handoff_id,
            upload_id = %id,
            received_bytes = upload.received,
            failure_kind = failure.kind,
            native_error_code = failure.native_error_code,
            elapsed_ms = upload.started_at.elapsed().as_millis(),
            "bounded browser Blob upload failed"
        );
        cleanup_upload_paths(staging_root, id, &upload, "write_failed");
        return Err(UploadFailure::Io);
    }
    let received = offset + content_length;
    let complete = received == expected_size;
    if complete && let Err(error) = fs::rename(&temporary, &completed) {
        state.active = state.active.saturating_sub(1);
        let upload = state.pending.remove(&id).ok_or(UploadFailure::Io)?;
        warn!(
            event = "browser_native_host.blob_upload_failed",
            correlation_id = %upload.correlation_id,
            handoff_id = %upload.handoff_id,
            upload_id = %id,
            received_bytes = received,
            failure_kind = "staging_rename",
            native_error_code = error.raw_os_error().unwrap_or_default(),
            elapsed_ms = upload.started_at.elapsed().as_millis(),
            "completed browser Blob could not enter staging"
        );
        cleanup_upload_paths(staging_root, id, &upload, "rename_failed");
        return Err(UploadFailure::Io);
    }
    let (correlation_id, handoff_id, started_at) = {
        let upload = state.pending.get_mut(&id).ok_or(UploadFailure::Io)?;
        upload.received = received;
        upload.expires_at = Instant::now() + CAPABILITY_TTL;
        upload.state = if complete {
            UploadState::Complete
        } else {
            UploadState::Uploading
        };
        (upload.correlation_id, upload.handoff_id, upload.started_at)
    };
    if complete {
        state.active = state.active.saturating_sub(1);
        info!(
            event = "browser_native_host.blob_upload_completed",
            %correlation_id,
            %handoff_id,
            upload_id = %id,
            size = received,
            elapsed_ms = started_at.elapsed().as_millis(),
            "bounded browser Blob upload completed"
        );
    }
    Ok(())
}

fn read_header(stream: &mut TcpStream) -> Result<Vec<u8>, UploadFailure> {
    let mut header = Vec::with_capacity(512);
    let mut byte = [0_u8; 1];
    while header.len() < MAX_HTTP_HEADER_BYTES {
        stream
            .read_exact(&mut byte)
            .map_err(|_| UploadFailure::Invalid)?;
        header.push(byte[0]);
        if header.ends_with(b"\r\n\r\n") {
            return Ok(header);
        }
    }
    Err(UploadFailure::Invalid)
}

fn parse_capability_path(path: &str) -> Option<(Uuid, &str, u64)> {
    let (route, query) = path.split_once('?')?;
    let offset = query.strip_prefix("offset=")?.parse::<u64>().ok()?;
    let segments = route.split('/').collect::<Vec<_>>();
    if segments.len() != 5 || segments[1] != "nxtools" || segments[2] != "blob" {
        return None;
    }
    let id = Uuid::parse_str(segments[3]).ok()?;
    let token = segments[4];
    (token.len() == 64 && token.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then_some((id, token, offset))
}

fn claim_upload(
    registry: &Arc<Mutex<UploadRegistry>>,
    id: Uuid,
    token: &str,
    offset: u64,
    content_length: u64,
) -> Result<u64, UploadFailure> {
    let mut registry = registry.lock().map_err(|_| UploadFailure::Io)?;
    let (pending, expected_size) = {
        let upload = registry.pending.get(&id).ok_or(UploadFailure::NotFound)?;
        if !matches!(upload.state, UploadState::Pending | UploadState::Uploading)
            || upload.expires_at <= Instant::now()
            || upload.token.as_bytes() != token.as_bytes()
        {
            return Err(UploadFailure::NotFound);
        }
        if content_length == 0
            || content_length > MAX_BLOB_CHUNK_BYTES
            || offset != upload.received
            || offset.saturating_add(content_length) > upload.expected_size
        {
            return Err(UploadFailure::Invalid);
        }
        (upload.state == UploadState::Pending, upload.expected_size)
    };
    if pending {
        if registry.active >= MAX_ACTIVE_BLOB_UPLOADS {
            return Err(UploadFailure::Busy);
        }
        registry.active += 1;
    }
    let upload = registry
        .pending
        .get_mut(&id)
        .ok_or(UploadFailure::NotFound)?;
    upload.state = UploadState::Receiving;
    upload.expires_at = Instant::now() + CAPABILITY_TTL;
    Ok(expected_size)
}

fn write_chunk_body(
    stream: &mut TcpStream,
    path: &Path,
    offset: u64,
    chunk_size: u64,
) -> Result<(), BlobBodyFailure> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|error| BlobBodyFailure::new("staging_open", error, chunk_size))?;
    file.set_len(offset)
        .map_err(|error| BlobBodyFailure::new("staging_truncate", error, chunk_size))?;
    file.seek(SeekFrom::Start(offset))
        .map_err(|error| BlobBodyFailure::new("staging_seek", error, chunk_size))?;
    let mut remaining = chunk_size;
    let mut buffer = [0_u8; 64 * 1024];
    while remaining > 0 {
        let read_limit = usize::try_from(remaining.min(buffer.len() as u64))
            .expect("read limit is bounded by the in-memory buffer");
        let read = stream
            .read(&mut buffer[..read_limit])
            .map_err(|error| BlobBodyFailure::new("socket_read", error, remaining))?;
        if read == 0 {
            return Err(BlobBodyFailure {
                kind: "unexpected_eof",
                native_error_code: 0,
                unread_bytes: remaining,
            });
        }
        file.write_all(&buffer[..read]).map_err(|error| {
            BlobBodyFailure::new(
                "staging_write",
                error,
                remaining.saturating_sub(read as u64),
            )
        })?;
        remaining -= read as u64;
    }
    file.flush()
        .map_err(|error| BlobBodyFailure::new("staging_flush", error, 0))?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BlobBodyFailure {
    kind: &'static str,
    native_error_code: i32,
    unread_bytes: u64,
}

impl BlobBodyFailure {
    fn new(kind: &'static str, error: std::io::Error, unread_bytes: u64) -> Self {
        Self {
            kind,
            native_error_code: error.raw_os_error().unwrap_or_default(),
            unread_bytes,
        }
    }
}

fn drain_claimed_body(stream: &mut TcpStream, unread_bytes: u64) {
    if unread_bytes > MAX_BLOB_CHUNK_BYTES {
        return;
    }
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let mut remaining = unread_bytes;
    let mut buffer = [0_u8; 16 * 1024];
    while remaining > 0 {
        let read_limit = usize::try_from(remaining.min(buffer.len() as u64))
            .expect("claimed drain limit is bounded by the in-memory buffer");
        match stream.read(&mut buffer[..read_limit]) {
            Ok(0) | Err(_) => return,
            Ok(read) => remaining -= read as u64,
        }
    }
}

fn drain_small_rejected_body(stream: &mut TcpStream, content_length: u64) {
    if content_length > 64 * 1024 {
        return;
    }
    let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
    let mut remaining = content_length;
    let mut buffer = [0_u8; 4096];
    while remaining > 0 {
        let read_limit = usize::try_from(remaining.min(buffer.len() as u64))
            .expect("drain limit is bounded by the in-memory buffer");
        match stream.read(&mut buffer[..read_limit]) {
            Ok(0) | Err(_) => return,
            Ok(read) => remaining -= read as u64,
        }
    }
}

fn completed_path(staging_root: &Path, id: Uuid) -> PathBuf {
    staging_root.join(format!("{id}.blob"))
}

fn cleanup_upload_paths(
    staging_root: &Path,
    id: Uuid,
    upload: &PendingUpload,
    reason: &'static str,
) -> usize {
    let paths = [
        (staging_root.join(format!("{id}.uploading")), "uploading"),
        (completed_path(staging_root, id), "completed"),
    ];
    let mut failures = 0;
    for (path, artifact_kind) in paths {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                failures += 1;
                warn!(
                    event = "browser_native_host.blob_staging_cleanup_failed",
                    correlation_id = %upload.correlation_id,
                    handoff_id = %upload.handoff_id,
                    upload_id = %id,
                    reason,
                    artifact_kind,
                    native_error_code = error.raw_os_error().unwrap_or_default(),
                    elapsed_ms = upload.started_at.elapsed().as_millis(),
                    "browser Blob data-plane staging cleanup failed"
                );
            }
        }
    }
    failures
}

fn reap_expired_uploads(registry: &mut UploadRegistry, staging_root: &Path, now: Instant) {
    let expired = registry
        .pending
        .iter()
        .filter_map(|(id, upload)| (upload.expires_at <= now).then_some(*id))
        .collect::<Vec<_>>();
    for id in expired {
        let Some(upload) = registry.pending.remove(&id) else {
            continue;
        };
        if matches!(
            upload.state,
            UploadState::Uploading | UploadState::Receiving
        ) {
            registry.active = registry.active.saturating_sub(1);
        }
        let cleanup_failures = cleanup_upload_paths(staging_root, id, &upload, "expired");
        warn!(
            event = "browser_native_host.blob_upload_expired",
            correlation_id = %upload.correlation_id,
            handoff_id = %upload.handoff_id,
            upload_id = %id,
            received_bytes = upload.received,
            cleanup_failures,
            elapsed_ms = upload.started_at.elapsed().as_millis(),
            "idle browser Blob upload expired and released its registry capacity"
        );
    }
}

fn cleanup_stale_staging_files(staging_root: &Path) {
    let stale_before = std::time::SystemTime::now()
        .checked_sub(Duration::from_secs(60 * 60))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let Ok(entries) = fs::read_dir(staging_root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let supported = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension == "blob" || extension == "uploading");
        if !supported {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| modified < stale_before);
        if stale
            && let Err(error) = fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            warn!(
                event = "browser_native_host.blob_stale_cleanup_failed",
                artifact_kind = path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .unwrap_or("unknown"),
                native_error_code = error.raw_os_error().unwrap_or_default(),
                "stale browser Blob staging cleanup failed"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::path::PathBuf;

    use super::BlobUploadService;

    fn prepare(service: &BlobUploadService, size: u64) -> super::BlobUploadCapability {
        service
            .prepare(size, uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
            .expect("prepare Blob upload")
    }

    #[test]
    fn one_time_capability_streams_exact_blob_bytes_to_disk() {
        let directory = unique_test_directory();
        std::fs::create_dir_all(&directory).expect("create Blob staging directory");
        let service = BlobUploadService::start(&directory).expect("start Blob upload service");
        let capability = prepare(&service, 5);

        let mut stream = TcpStream::connect(service.address()).expect("connect loopback upload");
        write!(
            stream,
            "POST {}?offset=0 HTTP/1.1\r\nHost: {}\r\nContent-Length: 5\r\nContent-Type: text/plain;charset=UTF-8\r\nConnection: close\r\n\r\nhello",
            capability.path,
            service.address()
        )
        .expect("write Blob upload");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("read upload response");
        assert!(response.starts_with("HTTP/1.1 204"), "{response}");

        let staged = super::completed_path(&directory, capability.id);
        assert_eq!(std::fs::read(staged).expect("read staged Blob"), b"hello");
        std::fs::remove_dir_all(directory).expect("remove Blob staging fixture");
    }

    #[test]
    fn capability_rejects_wrong_size_and_cannot_be_reused() {
        let directory = unique_test_directory();
        std::fs::create_dir_all(&directory).expect("create Blob staging directory");
        let service = BlobUploadService::start(&directory).expect("start Blob upload service");
        let capability = prepare(&service, 5);

        let invalid = post_blob(&service, &capability.path, 0, b"longer");
        assert!(invalid.starts_with("HTTP/1.1 400"), "{invalid}");
        let accepted = post_blob(&service, &capability.path, 0, b"hello");
        assert!(accepted.starts_with("HTTP/1.1 204"), "{accepted}");
        let reused = post_blob(&service, &capability.path, 0, b"hello");
        assert!(reused.starts_with("HTTP/1.1 404"), "{reused}");
        let staged = super::completed_path(&directory, capability.id);
        std::fs::remove_file(staged).expect("remove staged Blob fixture");
        std::fs::remove_dir_all(directory).expect("remove Blob staging fixture");
    }

    #[test]
    fn prepare_rejects_empty_and_over_limit_blobs() {
        let directory = unique_test_directory();
        let service = BlobUploadService::start(&directory).expect("start Blob upload service");
        assert!(
            service
                .prepare(0, uuid::Uuid::new_v4(), uuid::Uuid::new_v4())
                .is_err()
        );
        assert!(
            service
                .prepare(
                    super::MAX_BLOB_UPLOAD_BYTES + 1,
                    uuid::Uuid::new_v4(),
                    uuid::Uuid::new_v4(),
                )
                .is_err()
        );
        std::fs::remove_dir_all(directory).expect("remove Blob staging fixture");
    }

    #[test]
    fn sequential_chunks_require_the_authoritative_offset() {
        let directory = unique_test_directory();
        let service = BlobUploadService::start(&directory).expect("start Blob upload service");
        let capability = prepare(&service, 6);
        let first = post_blob(&service, &capability.path, 0, b"abc");
        assert!(first.starts_with("HTTP/1.1 204"), "{first}");
        let skipped = post_blob(&service, &capability.path, 4, b"de");
        assert!(skipped.starts_with("HTTP/1.1 400"), "{skipped}");
        let second = post_blob(&service, &capability.path, 3, b"def");
        assert!(second.starts_with("HTTP/1.1 204"), "{second}");
        let staged = super::completed_path(&directory, capability.id);
        assert_eq!(
            std::fs::read(&staged).expect("read chunked Blob"),
            b"abcdef"
        );
        std::fs::remove_file(staged).expect("remove staged Blob fixture");
        std::fs::remove_dir_all(directory).expect("remove Blob staging fixture");
    }

    #[test]
    fn expired_partial_upload_releases_capacity_and_removes_staging() {
        let directory = unique_test_directory();
        let service = BlobUploadService::start(&directory).expect("start Blob upload service");
        let capability = prepare(&service, 6);
        let first = post_blob(&service, &capability.path, 0, b"abc");
        assert!(first.starts_with("HTTP/1.1 204"), "{first}");
        let temporary = directory.join(format!("{}.uploading", capability.id));
        assert!(temporary.exists());
        {
            let mut registry = service.registry.lock().expect("lock upload registry");
            let upload = registry
                .pending
                .get_mut(&capability.id)
                .expect("partial upload remains registered");
            upload.expires_at = std::time::Instant::now() - std::time::Duration::from_secs(1);
            assert_eq!(registry.active, 1);
        }

        let replacement = prepare(&service, 1);
        let registry = service.registry.lock().expect("lock reaped registry");
        assert_eq!(registry.active, 0);
        assert!(!registry.pending.contains_key(&capability.id));
        assert!(registry.pending.contains_key(&replacement.id));
        drop(registry);
        assert!(!temporary.exists());
        std::fs::remove_dir_all(directory).expect("remove Blob staging fixture");
    }

    #[test]
    fn completed_commit_identity_survives_registry_forget_for_replay() {
        let directory = unique_test_directory();
        let service = BlobUploadService::start(&directory).expect("start Blob upload service");
        let correlation_id = uuid::Uuid::new_v4();
        let handoff_id = uuid::Uuid::new_v4();
        let capability = service
            .prepare(5, correlation_id, handoff_id)
            .expect("prepare identified Blob upload");
        let accepted = post_blob(&service, &capability.path, 0, b"hello");
        assert!(accepted.starts_with("HTTP/1.1 204"), "{accepted}");
        let first = service
            .commit_candidate(capability.id, 5, correlation_id, handoff_id)
            .expect("resolve registered commit");
        assert!(first.registered);
        service.forget(capability.id);
        let replay = service
            .commit_candidate(capability.id, 5, correlation_id, handoff_id)
            .expect("resolve replay after registry forget");
        assert!(!replay.registered);
        assert_eq!(first.path, replay.path);
        std::fs::remove_file(replay.path).expect("remove replay fixture");
        std::fs::remove_dir_all(directory).expect("remove Blob staging fixture");
    }

    #[test]
    fn claiming_a_near_expiry_chunk_renews_the_receiving_lease() {
        let directory = unique_test_directory();
        let service = BlobUploadService::start(&directory).expect("start Blob upload service");
        let capability = prepare(&service, 1);
        {
            let mut registry = service.registry.lock().expect("lock upload registry");
            registry
                .pending
                .get_mut(&capability.id)
                .expect("prepared upload exists")
                .expires_at = std::time::Instant::now() + std::time::Duration::from_millis(1);
        }
        let token = capability
            .path
            .rsplit('/')
            .next()
            .expect("capability token");
        let claimed_at = std::time::Instant::now();
        super::claim_upload(&service.registry, capability.id, token, 0, 1)
            .expect("claim near-expiry chunk");
        let registry = service.registry.lock().expect("lock renewed registry");
        let upload = registry
            .pending
            .get(&capability.id)
            .expect("claimed upload remains registered");
        assert_eq!(upload.state, super::UploadState::Receiving);
        assert!(
            upload.expires_at
                >= claimed_at + super::CAPABILITY_TTL - std::time::Duration::from_millis(10)
        );
        drop(registry);
        service.forget(capability.id);
        std::fs::remove_dir_all(directory).expect("remove Blob staging fixture");
    }

    #[test]
    fn staging_rename_failure_releases_upload_state() {
        let directory = unique_test_directory();
        let service = BlobUploadService::start(&directory).expect("start Blob upload service");
        let capability = prepare(&service, 1);
        let completed = super::completed_path(&directory, capability.id);
        std::fs::create_dir(&completed).expect("reserve completed path as a directory");

        let response = post_blob(&service, &capability.path, 0, b"x");
        assert!(response.starts_with("HTTP/1.1 500"), "{response}");
        let registry = service.registry.lock().expect("lock failed registry");
        assert_eq!(registry.active, 0);
        assert!(!registry.pending.contains_key(&capability.id));
        drop(registry);
        assert!(
            !directory
                .join(format!("{}.uploading", capability.id))
                .exists()
        );
        std::fs::remove_dir_all(directory).expect("remove Blob staging fixture");
    }

    #[test]
    fn staging_open_failure_preserves_native_failure_and_releases_state() {
        let directory = unique_test_directory();
        let service = BlobUploadService::start(&directory).expect("start Blob upload service");
        let capability = prepare(&service, 1);
        let temporary = directory.join(format!("{}.uploading", capability.id));
        std::fs::create_dir(&temporary).expect("block staging open with a directory");

        let response = post_blob(&service, &capability.path, 0, b"x");
        assert!(response.starts_with("HTTP/1.1 500"), "{response}");
        let registry = service.registry.lock().expect("lock failed registry");
        assert_eq!(registry.active, 0);
        assert!(!registry.pending.contains_key(&capability.id));
        drop(registry);
        assert!(temporary.is_dir());
        std::fs::remove_dir_all(directory).expect("remove Blob staging fixture");
    }

    #[test]
    fn expired_upload_reports_cleanup_failure_without_retaining_capacity() {
        let directory = unique_test_directory();
        let service = BlobUploadService::start(&directory).expect("start Blob upload service");
        let capability = prepare(&service, 2);
        let first = post_blob(&service, &capability.path, 0, b"x");
        assert!(first.starts_with("HTTP/1.1 204"), "{first}");
        let temporary = directory.join(format!("{}.uploading", capability.id));
        std::fs::remove_file(&temporary).expect("replace partial file with blocked directory");
        std::fs::create_dir(&temporary).expect("block expiry cleanup with a directory");
        {
            let mut registry = service.registry.lock().expect("lock upload registry");
            registry
                .pending
                .get_mut(&capability.id)
                .expect("partial upload remains registered")
                .expires_at = std::time::Instant::now() - std::time::Duration::from_secs(1);
        }

        let replacement = prepare(&service, 1);
        let registry = service.registry.lock().expect("lock reaped registry");
        assert_eq!(registry.active, 0);
        assert!(!registry.pending.contains_key(&capability.id));
        assert!(registry.pending.contains_key(&replacement.id));
        drop(registry);
        assert!(temporary.is_dir());
        std::fs::remove_dir_all(directory).expect("remove Blob staging fixture");
    }

    fn post_blob(service: &BlobUploadService, path: &str, offset: u64, body: &[u8]) -> String {
        let mut stream = TcpStream::connect(service.address()).expect("connect loopback upload");
        write!(
            stream,
            "POST {path}?offset={offset} HTTP/1.1\r\nHost: {}\r\nContent-Length: {}\r\nContent-Type: text/plain;charset=UTF-8\r\nConnection: close\r\n\r\n",
            service.address(),
            body.len()
        )
        .expect("write Blob request headers");
        stream.write_all(body).expect("write Blob request body");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("read Blob response");
        response
    }

    fn unique_test_directory() -> PathBuf {
        std::env::temp_dir().join(format!(
            "nxtools-browser-blob-test-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ))
    }
}
