#![allow(unsafe_code)]

use std::ffi::c_void;
use std::fs::File;
use std::io::{BufReader, ErrorKind, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle};
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use windows_sys::Win32::Foundation::{
    CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_BROKEN_PIPE,
    ERROR_INSUFFICIENT_BUFFER, ERROR_IO_PENDING, ERROR_NO_DATA, ERROR_OPERATION_ABORTED,
    ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, ERROR_PIPE_NOT_CONNECTED, GetLastError,
    INVALID_HANDLE_VALUE, LocalFree, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, FILE_FLAG_OVERLAPPED, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
    ReadFile, WriteFile,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, GetNamedPipeServerProcessId,
    PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
    PIPE_WAIT, WaitNamedPipeW,
};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::{
    CreateEventW, GetCurrentProcess, OpenProcess, OpenProcessToken,
    PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW, SetEvent,
    WaitForMultipleObjects,
};

pub const INDEXER_PIPE_NAME: &str = r"\\.\pipe\nxtools-next-indexer-v1";
const PIPE_BUFFER_BYTES: u32 = 64 * 1024;
const SDDL_REVISION_1: u32 = 1;
// SYSTEM and Administrators: full access. Authenticated users: read/write/synchronize only.
const CLIENT_PIPE_ACCESS: u32 = 0x0012_019B;
const PIPE_SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;0x12019b;;;AU)";
#[cfg(test)]
static MISSING_PIPE_ATTEMPTS: std::sync::atomic::AtomicUsize =
    std::sync::atomic::AtomicUsize::new(0);

pub struct LocalPipeConnection {
    pub reader: BufReader<LocalPipeReader>,
    pub writer: LocalPipeWriter,
    pub client_process_id: Option<u32>,
    pub client_session_id: Option<u32>,
}

/// A single signalled kernel event accompanies the existing service lease flag.
#[derive(Clone)]
pub struct PipeRetirement(Arc<PipeRetirementInner>);

struct PipeRetirementInner {
    flag: Arc<AtomicBool>,
    event: OwnedHandle,
}

impl PipeRetirement {
    fn new(flag: Arc<AtomicBool>) -> std::io::Result<Self> {
        let event = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };
        if event.is_null() {
            return Err(std::io::Error::last_os_error());
        }
        Ok(Self(Arc::new(PipeRetirementInner {
            flag,
            event: OwnedHandle(event),
        })))
    }

    pub fn retire(&self) {
        self.0.flag.store(true, Ordering::Release);
        unsafe { SetEvent(self.0.event.0) };
    }

    fn is_retired(&self) -> bool {
        self.0.flag.load(Ordering::Acquire)
    }

    fn event(&self) -> *mut c_void {
        self.0.event.0
    }
}

impl LocalPipeConnection {
    pub fn into_retiring(
        mut self,
        retired: Arc<AtomicBool>,
        write_timeout: Duration,
    ) -> std::io::Result<(
        BufReader<LocalPipeReader>,
        impl Write + Send + 'static,
        PipeRetirement,
    )> {
        let signal = PipeRetirement::new(retired)?;
        self.reader.get_mut().retired = Some(signal.clone());
        Ok((
            self.reader,
            RetiringPipeWriter {
                file: self.writer,
                retired: signal.clone(),
                write_timeout,
            },
            signal,
        ))
    }
}

pub struct LocalPipeWriter {
    file: File,
}

impl AsRawHandle for LocalPipeWriter {
    fn as_raw_handle(&self) -> std::os::windows::io::RawHandle {
        self.file.as_raw_handle()
    }
}

impl Write for LocalPipeWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        overlapped_write(&self.file, bytes, None, None)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

struct RetiringPipeWriter {
    file: LocalPipeWriter,
    retired: PipeRetirement,
    write_timeout: Duration,
}

impl Write for RetiringPipeWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        write_pipe_frame(
            &mut self.file,
            bytes,
            self.write_timeout,
            Some(&self.retired),
        )?;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub struct LocalPipeReader {
    file: File,
    initial_frame_deadline: Option<Instant>,
    retired: Option<PipeRetirement>,
    #[cfg(test)]
    wait_wakeups: Option<Arc<std::sync::atomic::AtomicUsize>>,
}

impl LocalPipeReader {
    /// Bounds only the initial newline-framed handshake; trickle reads never extend it.
    pub fn set_initial_frame_timeout(&mut self, timeout: Duration) {
        self.initial_frame_deadline = Some(Instant::now() + timeout);
    }
}

impl Read for LocalPipeReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self
            .retired
            .as_ref()
            .is_some_and(PipeRetirement::is_retired)
        {
            return Ok(0);
        }
        if self
            .initial_frame_deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return Err(std::io::Error::new(
                ErrorKind::TimedOut,
                "local pipe handshake expired",
            ));
        }
        let read = match overlapped_read(
            &self.file,
            buffer,
            self.initial_frame_deadline,
            self.retired.as_ref(),
            #[cfg(test)]
            self.wait_wakeups.as_deref(),
        ) {
            Err(_)
                if self
                    .retired
                    .as_ref()
                    .is_some_and(PipeRetirement::is_retired) =>
            {
                return Ok(0);
            }
            Err(error) if matches!(error.raw_os_error(), Some(code) if matches!(code as u32, ERROR_BROKEN_PIPE | ERROR_NO_DATA | ERROR_PIPE_NOT_CONNECTED)) =>
            {
                return Ok(0);
            }
            result => result?,
        };
        if buffer[..read].contains(&b'\n') {
            self.initial_frame_deadline = None;
        }
        Ok(read)
    }
}

fn operation_event() -> std::io::Result<OwnedHandle> {
    let event = unsafe { CreateEventW(ptr::null(), 1, 0, ptr::null()) };
    if event.is_null() {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(OwnedHandle(event))
    }
}

/// Never release the OVERLAPPED or caller buffer before cancellation completion is drained.
fn complete_io(
    file: &File,
    overlap: &mut OVERLAPPED,
    deadline: Option<Instant>,
    retired: Option<&PipeRetirement>,
) -> std::io::Result<u32> {
    let handles = [
        overlap.hEvent,
        retired.map_or(ptr::null_mut(), PipeRetirement::event),
    ];
    let wait_ms = deadline.map_or(u32::MAX, |end| {
        let remaining = end.saturating_duration_since(Instant::now());
        remaining.as_millis().min((u32::MAX - 1) as u128) as u32
    });
    let waited = unsafe {
        WaitForMultipleObjects(
            if retired.is_some() { 2 } else { 1 },
            handles.as_ptr(),
            0,
            wait_ms,
        )
    };
    if waited != WAIT_OBJECT_0 {
        let failure = if waited == WAIT_TIMEOUT {
            std::io::Error::new(ErrorKind::TimedOut, "local pipe operation expired")
        } else if retired.is_some() && waited == WAIT_OBJECT_0 + 1 {
            std::io::Error::new(ErrorKind::BrokenPipe, "local pipe session retired")
        } else {
            std::io::Error::last_os_error()
        };
        unsafe { CancelIoEx(file.as_raw_handle(), overlap) };
        let mut drained = 0;
        unsafe { GetOverlappedResult(file.as_raw_handle(), overlap, &mut drained, 1) };
        return Err(failure);
    }
    let mut completed = 0;
    if unsafe { GetOverlappedResult(file.as_raw_handle(), overlap, &mut completed, 0) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(completed)
}

fn overlapped_read(
    file: &File,
    buffer: &mut [u8],
    deadline: Option<Instant>,
    retired: Option<&PipeRetirement>,
    #[cfg(test)] wakeups: Option<&std::sync::atomic::AtomicUsize>,
) -> std::io::Result<usize> {
    let event = operation_event()?;
    let mut overlap: OVERLAPPED = unsafe { std::mem::zeroed() };
    overlap.hEvent = event.0;
    let mut count = 0;
    let result = unsafe {
        ReadFile(
            file.as_raw_handle(),
            buffer.as_mut_ptr().cast(),
            buffer.len().min(u32::MAX as usize) as u32,
            &mut count,
            &mut overlap,
        )
    };
    if result != 0 {
        return Ok(count as usize);
    }
    let error = unsafe { GetLastError() };
    if error != ERROR_IO_PENDING {
        return Err(std::io::Error::from_raw_os_error(error as i32));
    }
    let result = complete_io(file, &mut overlap, deadline, retired);
    #[cfg(test)]
    if let Some(wakeups) = wakeups {
        wakeups.fetch_add(1, Ordering::Relaxed);
    }
    result.map(|count| count as usize)
}

fn overlapped_write(
    file: &File,
    buffer: &[u8],
    deadline: Option<Instant>,
    retired: Option<&PipeRetirement>,
) -> std::io::Result<usize> {
    if buffer.is_empty() {
        return Ok(0);
    }
    if retired.is_some_and(PipeRetirement::is_retired) {
        return Err(std::io::Error::new(
            ErrorKind::BrokenPipe,
            "local pipe session retired",
        ));
    }
    let event = operation_event()?;
    let mut overlap: OVERLAPPED = unsafe { std::mem::zeroed() };
    overlap.hEvent = event.0;
    let mut count = 0;
    let result = unsafe {
        WriteFile(
            file.as_raw_handle(),
            buffer.as_ptr().cast(),
            buffer.len().min(u32::MAX as usize) as u32,
            &mut count,
            &mut overlap,
        )
    };
    if result != 0 {
        return Ok(count as usize);
    }
    let error = unsafe { GetLastError() };
    if error != ERROR_IO_PENDING {
        return Err(std::io::Error::from_raw_os_error(error as i32));
    }
    complete_io(file, &mut overlap, deadline, retired).map(|count| count as usize)
}

pub fn accept_local_indexer_pipe(pipe_name: &str) -> Result<LocalPipeConnection> {
    accept_pipe_with_sddl(pipe_name, PIPE_SDDL, "indexer")
}

pub fn accept_current_user_pipe(pipe_name: &str) -> Result<LocalPipeConnection> {
    let user_sid = current_user_sid_string()?;
    let sddl = format!("D:(A;;GA;;;SY)(A;;GA;;;{user_sid})");
    accept_pipe_with_sddl(pipe_name, &sddl, "current-user")
}

pub(crate) fn transient_accept_error(error: &std::io::Error) -> bool {
    matches!(error.raw_os_error(), Some(code) if matches!(code as u32,
        ERROR_NO_DATA | ERROR_PIPE_NOT_CONNECTED | ERROR_BROKEN_PIPE | ERROR_OPERATION_ABORTED))
}

fn accept_pipe_with_sddl(
    pipe_name: &str,
    pipe_sddl: &str,
    pipe_kind: &str,
) -> Result<LocalPipeConnection> {
    accept_pipe_with_sddl_after_create(pipe_name, pipe_sddl, pipe_kind, || {})
}

fn accept_pipe_with_sddl_after_create(
    pipe_name: &str,
    pipe_sddl: &str,
    pipe_kind: &str,
    after_create: impl FnOnce(),
) -> Result<LocalPipeConnection> {
    let pipe_name = wide(pipe_name);
    let sddl = wide(pipe_sddl);
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    let converted = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            ptr::null_mut(),
        )
    };
    if converted == 0 {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("failed to build the {pipe_kind} pipe security descriptor"));
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let mut after_create = Some(after_create);
    let mut backoff = Duration::from_millis(10);
    let mut last_log = Instant::now() - Duration::from_secs(1);
    loop {
        let handle = unsafe {
            CreateNamedPipeW(
                pipe_name.as_ptr(),
                PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_UNLIMITED_INSTANCES,
                PIPE_BUFFER_BYTES,
                PIPE_BUFFER_BYTES,
                500,
                &attributes,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            let error = std::io::Error::last_os_error();
            unsafe { LocalFree(descriptor.cast()) };
            return Err(error).with_context(|| format!("failed to create {pipe_kind} named pipe"));
        }
        if let Some(hook) = after_create.take() {
            hook();
        }
        let file = unsafe { File::from_raw_handle(handle) };
        let event = match operation_event() {
            Ok(event) => event,
            Err(error) => {
                unsafe { LocalFree(descriptor.cast()) };
                return Err(error)
                    .with_context(|| format!("failed to create {pipe_kind} accept event"));
            }
        };
        let mut overlap: OVERLAPPED = unsafe { std::mem::zeroed() };
        overlap.hEvent = event.0;
        let connected = unsafe { ConnectNamedPipe(handle, &mut overlap) };
        let error = if connected != 0 {
            0
        } else {
            unsafe { GetLastError() }
        };
        let outcome = if error == ERROR_IO_PENDING {
            complete_io(&file, &mut overlap, None, None).map(|_| ())
        } else if error == 0 || error == ERROR_PIPE_CONNECTED {
            Ok(())
        } else {
            Err(std::io::Error::from_raw_os_error(error as i32))
        };
        match outcome {
            Ok(()) => {
                unsafe { LocalFree(descriptor.cast()) };
                let identity = server_client_identity(handle);
                return connection_from_handle(file.into_raw_handle(), identity);
            }
            Err(error) if transient_accept_error(&error) => {
                if last_log.elapsed() >= Duration::from_secs(1) {
                    tracing::warn!(
                        event = "local_pipe.accept.transient",
                        pipe_kind,
                        native_code = error.raw_os_error(),
                        "transient pipe accept; recreating instance"
                    );
                    last_log = Instant::now();
                }
                drop(file);
                std::thread::sleep(backoff);
                backoff = (backoff * 2).min(Duration::from_millis(100));
            }
            Err(error) => {
                unsafe { LocalFree(descriptor.cast()) };
                return Err(error)
                    .with_context(|| format!("failed to accept {pipe_kind} pipe client"));
            }
        }
    }
}

pub fn connect_local_indexer_pipe(
    pipe_name: &str,
    timeout: Duration,
) -> Result<LocalPipeConnection> {
    connect_local_pipe(pipe_name, timeout)
        .context("timed out connecting to the indexer service pipe")
}

pub fn connect_local_pipe(pipe_name: &str, timeout: Duration) -> Result<LocalPipeConnection> {
    let pipe_name = wide(pipe_name);
    #[cfg(test)]
    let count_missing_attempts = {
        let prefix = wide(r"\\.\pipe\nxtools-absent-");
        pipe_name.starts_with(&prefix[..prefix.len() - 1])
    };
    let deadline = Instant::now() + timeout;
    loop {
        #[cfg(test)]
        if count_missing_attempts {
            MISSING_PIPE_ATTEMPTS.fetch_add(1, Ordering::Relaxed);
        }
        let handle = unsafe {
            CreateFileW(
                pipe_name.as_ptr(),
                CLIENT_PIPE_ACCESS,
                0,
                ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED,
                ptr::null_mut(),
            )
        };
        if handle != INVALID_HANDLE_VALUE {
            return connection_from_handle(handle, None);
        }
        let error = std::io::Error::last_os_error();
        if Instant::now() >= deadline {
            return Err(error).context("timed out connecting to the local pipe");
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        let wait_ms = remaining.min(Duration::from_millis(250)).as_millis() as u32;
        let waited = unsafe { WaitNamedPipeW(pipe_name.as_ptr(), wait_ms.max(1)) };
        if waited == 0 {
            let wait_error = std::io::Error::last_os_error();
            if !matches!(wait_error.kind(), ErrorKind::NotFound | ErrorKind::TimedOut)
                && wait_error.raw_os_error() != Some(ERROR_PIPE_BUSY as i32)
            {
                return Err(wait_error).context("failed waiting for the local pipe");
            }
            if wait_error.kind() == ErrorKind::NotFound {
                std::thread::sleep(
                    Duration::from_millis(25)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
        }
    }
}

fn current_user_sid_string() -> Result<String> {
    process_user_sid_string(unsafe { GetCurrentProcess() })
}

fn process_user_sid_string(process: *mut c_void) -> Result<String> {
    let mut token = ptr::null_mut();
    let opened = unsafe { OpenProcessToken(process, TOKEN_QUERY, &raw mut token) };
    if opened == 0 {
        return Err(std::io::Error::last_os_error())
            .context("failed to open the current process token");
    }
    let token = OwnedHandle(token);
    let mut bytes = 0_u32;
    let queried =
        unsafe { GetTokenInformation(token.0, TokenUser, ptr::null_mut(), 0, &raw mut bytes) };
    if queried != 0 || bytes == 0 || unsafe { GetLastError() } != ERROR_INSUFFICIENT_BUFFER {
        bail!("Windows did not return the current user token size");
    }
    let mut buffer = vec![0_u8; bytes as usize];
    let queried = unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            bytes,
            &raw mut bytes,
        )
    };
    if queried == 0 {
        return Err(std::io::Error::last_os_error())
            .context("failed to query the current user token");
    }
    let token_user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let mut sid_text = ptr::null_mut();
    let converted = unsafe { ConvertSidToStringSidW(token_user.User.Sid, &raw mut sid_text) };
    if converted == 0 {
        return Err(std::io::Error::last_os_error())
            .context("failed to convert the current user SID");
    }
    let sid_text = LocalAllocation(sid_text.cast());
    let mut length = 0_usize;
    while length < 256 && unsafe { *sid_text.0.cast::<u16>().add(length) } != 0 {
        length += 1;
    }
    if length == 256 {
        bail!("Windows returned an unterminated current user SID");
    }
    String::from_utf16(unsafe { std::slice::from_raw_parts(sid_text.0.cast::<u16>(), length) })
        .context("Windows returned an invalid current user SID")
}

struct OwnedHandle(*mut c_void);
// A Windows kernel handle is shareable across threads; the owner closes it once.
unsafe impl Send for OwnedHandle {}
unsafe impl Sync for OwnedHandle {}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

struct LocalAllocation(*mut c_void);

impl Drop for LocalAllocation {
    fn drop(&mut self) {
        unsafe {
            LocalFree(self.0);
        }
    }
}

fn connection_from_handle(
    handle: *mut c_void,
    client_identity: Option<(u32, u32)>,
) -> Result<LocalPipeConnection> {
    if handle == INVALID_HANDLE_VALUE {
        bail!("cannot construct an indexer connection from an invalid handle");
    }
    let process = unsafe { GetCurrentProcess() };
    let mut writer_handle = ptr::null_mut();
    let duplicated = unsafe {
        DuplicateHandle(
            process,
            handle,
            process,
            &mut writer_handle,
            0,
            0,
            DUPLICATE_SAME_ACCESS,
        )
    };
    if duplicated == 0 {
        unsafe { drop(File::from_raw_handle(handle)) };
        return Err(std::io::Error::last_os_error())
            .context("failed to duplicate indexer pipe handle");
    }
    let reader = LocalPipeReader {
        file: unsafe { File::from_raw_handle(handle) },
        initial_frame_deadline: None,
        retired: None,
        #[cfg(test)]
        wait_wakeups: None,
    };
    let writer = LocalPipeWriter {
        file: unsafe { File::from_raw_handle(writer_handle) },
    };
    Ok(LocalPipeConnection {
        reader: BufReader::new(reader),
        writer,
        client_process_id: client_identity.map(|(process_id, _)| process_id),
        client_session_id: client_identity.map(|(_, session_id)| session_id),
    })
}

fn server_client_identity(handle: *mut c_void) -> Option<(u32, u32)> {
    let mut process_id = 0;
    let identified = unsafe { GetNamedPipeClientProcessId(handle, &raw mut process_id) };
    if identified == 0 || process_id == 0 {
        return None;
    }
    let mut session_id = 0;
    let resolved = unsafe { ProcessIdToSessionId(process_id, &raw mut session_id) };
    (resolved != 0).then_some((process_id, session_id))
}

/// Short secondary-client writes must not wait forever on a peer that stops reading.
pub(crate) fn write_desktop_frame(
    file: &mut LocalPipeWriter,
    bytes: &[u8],
    timeout: Duration,
) -> Result<()> {
    write_pipe_frame(file, bytes, timeout, None).context("desktop pipe write failed")
}

fn write_pipe_frame(
    file: &mut LocalPipeWriter,
    bytes: &[u8],
    timeout: Duration,
    retired: Option<&PipeRetirement>,
) -> std::io::Result<()> {
    let deadline = Instant::now() + timeout;
    let mut written = 0;
    while written < bytes.len() {
        if retired.is_some_and(PipeRetirement::is_retired) {
            return Err(std::io::Error::new(
                ErrorKind::BrokenPipe,
                "local pipe session retired",
            ));
        }
        if Instant::now() >= deadline {
            return Err(std::io::Error::new(
                ErrorKind::TimedOut,
                "local pipe write expired",
            ));
        }
        let count = overlapped_write(&file.file, &bytes[written..], Some(deadline), retired)?;
        if count == 0 {
            return Err(std::io::Error::new(
                ErrorKind::WriteZero,
                "local pipe write returned zero",
            ));
        }
        written += count;
    }
    Ok(())
}

/// Security descriptor lives until CreateNamedPipe returns; Windows copies it.
pub(crate) fn create_current_user_async_pipe(
    pipe_name: &str,
    first: bool,
) -> Result<tokio::net::windows::named_pipe::NamedPipeServer> {
    let sid = current_user_sid_string()?;
    let sddl = wide(&format!("D:(A;;GA;;;SY)(A;;GA;;;{sid})"));
    let mut descriptor: PSECURITY_DESCRIPTOR = ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &raw mut descriptor,
            ptr::null_mut(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error()).context("desktop pipe security failed");
    }
    let allocation = LocalAllocation(descriptor);
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: allocation.0,
        bInheritHandle: 0,
    };
    let result = unsafe {
        tokio::net::windows::named_pipe::ServerOptions::new()
            .first_pipe_instance(first)
            .reject_remote_clients(true)
            .create_with_security_attributes_raw(pipe_name, (&raw mut attributes).cast())
    };
    result.context("desktop pipe creation failed")
}

/// Actual kernel pipe PID, token SID, session and executable identity, in both directions.
/// The wire's client_name/capabilities are never authority. Only this exact running image
/// is admitted (protected redirect already forwards original argv to that image).
pub(crate) fn authorize_desktop_peer(
    handle: *mut c_void,
    server: bool,
) -> Result<(), nxtools_protocol::desktop_archive_intent::LaunchFailure> {
    use nxtools_protocol::desktop_archive_intent::LaunchFailure;
    let authorize = || -> Result<()> {
        let mut peer_pid = 0u32;
        let identified = unsafe {
            if server {
                GetNamedPipeClientProcessId(handle, &raw mut peer_pid)
            } else {
                GetNamedPipeServerProcessId(handle, &raw mut peer_pid)
            }
        };
        if identified == 0 || peer_pid == 0 {
            bail!("desktop peer PID unavailable");
        }
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, peer_pid) };
        if process.is_null() {
            bail!("desktop peer process unavailable");
        }
        let process = OwnedHandle(process);
        let mut peer_session = 0;
        let mut own_session = 0;
        if unsafe { ProcessIdToSessionId(peer_pid, &raw mut peer_session) } == 0
            || unsafe { ProcessIdToSessionId(std::process::id(), &raw mut own_session) } == 0
            || peer_session != own_session
            || process_user_sid_string(process.0)? != current_user_sid_string()?
        {
            bail!("desktop peer identity rejected");
        }
        let mut path = vec![0u16; 32768];
        let mut length = path.len() as u32;
        if unsafe { QueryFullProcessImageNameW(process.0, 0, path.as_mut_ptr(), &raw mut length) }
            == 0
        {
            bail!("desktop peer image unavailable");
        }
        let peer_path = String::from_utf16(&path[..length as usize])?;
        let own_path = std::env::current_exe()?;
        if !peer_path
            .trim_start_matches(r"\\?\")
            .replace('/', "\\")
            .eq_ignore_ascii_case(
                &own_path
                    .to_string_lossy()
                    .trim_start_matches(r"\\?\")
                    .replace('/', "\\"),
            )
        {
            bail!("desktop peer image rejected");
        }
        Ok(())
    };
    authorize().map_err(|_| LaunchFailure::Unauthorized)
}

fn wide(value: &str) -> Vec<u16> {
    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, Write};

    use super::*;

    #[test]
    fn measure_ten_seconds_of_idle_pipe_reader_wakeups() {
        let name = format!(r"\\.\pipe\nxtools-idle-measure-{}", uuid::Uuid::new_v4());
        let server_name = name.clone();
        let wakeups = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let measured_wakeups = Arc::clone(&wakeups);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let mut connection = accept_current_user_pipe(&server_name).expect("accept fixture");
            connection.reader.get_mut().wait_wakeups = Some(measured_wakeups);
            ready_tx.send(()).expect("ready");
            let mut bytes = [0; 64];
            connection.reader.read(&mut bytes).expect("idle EOF")
        });
        let client = connect_local_pipe(&name, Duration::from_secs(2)).expect("connect fixture");
        ready_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("ready");
        std::thread::sleep(Duration::from_secs(10));
        let wakeups = wakeups.load(Ordering::Relaxed);
        println!("idle_reader_10s_wakeups={wakeups}");
        drop(client);
        assert_eq!(server.join().expect("idle worker"), 0);
    }

    #[test]
    fn missing_pipe_attempts_are_bounded_by_remaining_deadline() {
        let name = format!(r"\\.\pipe\nxtools-absent-{}", uuid::Uuid::new_v4());
        MISSING_PIPE_ATTEMPTS.store(0, Ordering::Relaxed);
        let began = Instant::now();
        assert!(connect_local_pipe(&name, Duration::from_millis(180)).is_err());
        let attempts = MISSING_PIPE_ATTEMPTS.load(Ordering::Relaxed);
        assert!(began.elapsed() < Duration::from_millis(500));
        assert!(
            attempts <= 10,
            "absent pipe caused {attempts} native retries"
        );
    }

    #[test]
    fn disconnected_before_connect_named_pipe_does_not_end_admission() {
        let name = format!(r"\\.\pipe\nxtools-preaccept-{}", uuid::Uuid::new_v4());
        let client_name = name.clone();
        let client = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(25));
            let mut connection = connect_local_pipe(&client_name, Duration::from_secs(2))
                .expect("healthy second client");
            connection.writer.write_all(b"hello\n").expect("hello");
        });
        let user_sid = current_user_sid_string().expect("test SID");
        let sddl = format!("D:(A;;GA;;;SY)(A;;GA;;;{user_sid})");
        let preconnect = name.clone();
        let mut connection =
            accept_pipe_with_sddl_after_create(&name, &sddl, "current-user", move || {
                let name = wide(&preconnect);
                let handle = unsafe {
                    CreateFileW(
                        name.as_ptr(),
                        CLIENT_PIPE_ACCESS,
                        0,
                        ptr::null(),
                        OPEN_EXISTING,
                        FILE_ATTRIBUTE_NORMAL | FILE_FLAG_OVERLAPPED,
                        ptr::null_mut(),
                    )
                };
                assert_ne!(handle, INVALID_HANDLE_VALUE, "preconnect fixture");
                unsafe { drop(File::from_raw_handle(handle)) };
            })
            .expect("transient client must not end admission");
        let mut line = String::new();
        connection
            .reader
            .read_line(&mut line)
            .expect("second client's frame");
        assert_eq!(line, "hello\n");
        client.join().expect("second client");
    }

    #[tokio::test]
    async fn tokio_accept_recreates_after_a_closed_preconnected_client() {
        let name = format!(r"\\.\pipe\nxtools-tokio-accept-{}", uuid::Uuid::new_v4());
        let server = create_current_user_async_pipe(&name, true).expect("first instance");
        let client = connect_local_pipe(&name, Duration::from_secs(2)).expect("preconnect");
        drop(client);
        let outcome = tokio::time::timeout(Duration::from_secs(2), server.connect())
            .await
            .expect("preconnected accept bound");
        if let Err(error) = outcome {
            assert!(
                transient_accept_error(&error),
                "unexpected native accept code: {:?}",
                error.raw_os_error()
            );
        }
        assert!(transient_accept_error(&std::io::Error::from_raw_os_error(
            ERROR_NO_DATA as i32
        )));
        drop(server);

        let mut next = create_current_user_async_pipe(&name, false).expect("replacement instance");
        let client_name = name.clone();
        let healthy = tokio::task::spawn_blocking(move || {
            connect_local_pipe(&client_name, Duration::from_secs(2)).expect("healthy client")
        });
        tokio::time::timeout(Duration::from_secs(2), next.connect())
            .await
            .expect("healthy accept bound")
            .expect("healthy Tokio accept");
        let mut client = healthy.await.expect("client worker");
        client.writer.write_all(b"healthy\n").expect("frame");
        let mut line = String::new();
        tokio::io::AsyncBufReadExt::read_line(&mut tokio::io::BufReader::new(&mut next), &mut line)
            .await
            .expect("next client's frame");
        assert_eq!(line, "healthy\n");
    }

    #[test]
    fn idle_reader_has_no_periodic_wakeups() {
        let name = format!(r"\\.\pipe\nxtools-idle-regression-{}", uuid::Uuid::new_v4());
        let server_name = name.clone();
        let wakeups = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let measured_wakeups = Arc::clone(&wakeups);
        let (started, ready) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let mut connection = accept_current_user_pipe(&server_name).expect("accept fixture");
            connection.reader.get_mut().wait_wakeups = Some(measured_wakeups);
            started.send(()).expect("ready");
            let mut buffer = [0; 4];
            connection.reader.read(&mut buffer).expect("EOF")
        });
        let client = connect_local_pipe(&name, Duration::from_secs(2)).expect("connect fixture");
        ready
            .recv_timeout(Duration::from_secs(2))
            .expect("reader entered");
        std::thread::sleep(Duration::from_millis(200));
        let wakes = wakeups.load(Ordering::Relaxed);
        drop(client);
        assert_eq!(server.join().expect("reader worker"), 0);
        assert_eq!(wakes, 0, "idle reader woke {wakes} times");
    }

    #[test]
    fn authenticated_local_client_can_use_the_service_pipe_duplex() {
        let pipe_name = format!(r"\\.\pipe\nxtools-next-test-{}", uuid::Uuid::new_v4());
        let server_name = pipe_name.clone();
        let server = std::thread::spawn(move || {
            let mut connection =
                accept_local_indexer_pipe(&server_name).expect("accept test client");
            assert_eq!(connection.client_process_id, Some(std::process::id()));
            assert!(connection.client_session_id.is_some());
            let mut request = String::new();
            connection
                .reader
                .read_line(&mut request)
                .expect("read request");
            assert_eq!(request, "hello\n");
            connection
                .writer
                .write_all(b"ready\n")
                .expect("write response");
            connection.writer.flush().expect("flush response");
        });

        let mut client = connect_local_indexer_pipe(&pipe_name, Duration::from_secs(2))
            .expect("connect test client");
        client.writer.write_all(b"hello\n").expect("write request");
        client.writer.flush().expect("flush request");
        let mut response = String::new();
        client
            .reader
            .read_line(&mut response)
            .expect("read response");
        assert_eq!(response, "ready\n");
        server.join().expect("join pipe server");
    }

    #[test]
    fn current_user_client_can_use_a_current_user_only_pipe() {
        let pipe_name = format!(r"\\.\pipe\nxtools-next-user-test-{}", uuid::Uuid::new_v4());
        let server_name = pipe_name.clone();
        let server = std::thread::spawn(move || {
            let mut connection =
                accept_current_user_pipe(&server_name).expect("accept current-user client");
            let mut request = String::new();
            connection.reader.read_line(&mut request).expect("request");
            assert_eq!(request, "same-user\n");
            connection
                .writer
                .write_all(b"accepted\n")
                .expect("response");
            connection.writer.flush().expect("flush response");
        });

        let mut client =
            connect_local_pipe(&pipe_name, Duration::from_secs(2)).expect("connect current user");
        client.writer.write_all(b"same-user\n").expect("request");
        client.writer.flush().expect("flush request");
        let mut response = String::new();
        client.reader.read_line(&mut response).expect("response");
        assert_eq!(response, "accepted\n");
        server.join().expect("join server");
    }

    #[test]
    fn a_waiting_reader_does_not_block_responses_on_the_duplicated_writer() {
        let pipe_name = format!(r"\\.\pipe\nxtools-next-test-{}", uuid::Uuid::new_v4());
        let server_name = pipe_name.clone();
        let server = std::thread::spawn(move || {
            let connection = accept_local_indexer_pipe(&server_name).expect("accept test client");
            let mut reader = connection.reader;
            let mut writer = connection.writer;
            let (requests, received) = std::sync::mpsc::sync_channel(2);
            let reader_thread = std::thread::spawn(move || {
                for _ in 0..2 {
                    let mut request = String::new();
                    reader.read_line(&mut request).expect("read request");
                    requests.send(request).expect("publish request");
                }
            });

            assert_eq!(received.recv().expect("first request"), "hello\n");
            // The reader thread is already waiting for the second request here. The response
            // must still be writable through the duplicated handle.
            writer.write_all(b"ready\n").expect("write first response");
            writer.flush().expect("flush first response");
            assert_eq!(received.recv().expect("second request"), "ping\n");
            writer.write_all(b"pong\n").expect("write second response");
            writer.flush().expect("flush second response");
            reader_thread.join().expect("join reader thread");
        });

        let mut client = connect_local_indexer_pipe(&pipe_name, Duration::from_secs(2))
            .expect("connect test client");
        client.writer.write_all(b"hello\n").expect("write hello");
        client.writer.flush().expect("flush hello");
        let mut ready = String::new();
        client.reader.read_line(&mut ready).expect("read ready");
        assert_eq!(ready, "ready\n");
        client.writer.write_all(b"ping\n").expect("write ping");
        client.writer.flush().expect("flush ping");
        let mut pong = String::new();
        client.reader.read_line(&mut pong).expect("read pong");
        assert_eq!(pong, "pong\n");
        server.join().expect("join pipe server");
    }

    #[test]
    fn partial_initial_frame_expires_without_resetting_deadline() {
        let name = format!(r"\\.\pipe\nxtools-handshake-{}", uuid::Uuid::new_v4());
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            let mut connection =
                accept_current_user_pipe(&server_name).expect("accept synthetic client");
            connection
                .reader
                .get_mut()
                .set_initial_frame_timeout(Duration::from_millis(80));
            let mut frame = String::new();
            let failure = connection
                .reader
                .read_line(&mut frame)
                .expect_err("incomplete Hello must expire");
            assert_eq!(failure.kind(), ErrorKind::TimedOut);
        });
        let mut client =
            connect_local_pipe(&name, Duration::from_secs(2)).expect("connect fixture");
        client
            .writer
            .write_all(b"partial hello")
            .expect("partial frame");
        client.writer.flush().expect("flush");
        server.join().expect("bounded handshake thread exits");
    }

    #[test]
    fn complete_initial_frame_disarms_deadline_for_normal_session() {
        let name = format!(r"\\.\pipe\nxtools-handshake-done-{}", uuid::Uuid::new_v4());
        let server_name = name.clone();
        let server = std::thread::spawn(move || {
            let mut connection = accept_current_user_pipe(&server_name).expect("accept fixture");
            connection
                .reader
                .get_mut()
                .set_initial_frame_timeout(Duration::from_millis(100));
            let mut hello = String::new();
            connection
                .reader
                .read_line(&mut hello)
                .expect("hello frame");
            assert_eq!(hello, "hello\n");
            std::thread::sleep(Duration::from_millis(150));
            let mut next = String::new();
            connection
                .reader
                .read_line(&mut next)
                .expect("session remains readable");
            assert_eq!(next, "ping\n");
        });
        let mut client =
            connect_local_pipe(&name, Duration::from_secs(2)).expect("connect fixture");
        client.writer.write_all(b"hello\n").expect("hello");
        client.writer.flush().expect("flush hello");
        std::thread::sleep(Duration::from_millis(180));
        client.writer.write_all(b"ping\n").expect("session ping");
        client.writer.flush().expect("flush ping");
        server.join().expect("join fixture");
    }

    #[test]
    fn m4_retirement_releases_an_idle_pipe_reader_while_peer_remains_connected() {
        let name = format!(r"\\.\pipe\nxtools-m4-idle-{}", uuid::Uuid::new_v4());
        let server_name = name.clone();
        let retired = Arc::new(AtomicBool::new(false));
        let reader_retired = Arc::clone(&retired);
        let (started, ready) = std::sync::mpsc::channel();
        let (finished, done) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let connection = accept_current_user_pipe(&server_name).expect("accept fixture");
            let (mut reader, _writer, signal) = connection
                .into_retiring(reader_retired, Duration::from_millis(150))
                .expect("retirement event");
            started.send(signal).expect("reader ready");
            let mut frame = String::new();
            let read = reader.read_line(&mut frame).expect("retired read");
            finished.send(read).expect("reader finished");
        });
        let client = connect_local_pipe(&name, Duration::from_secs(2)).expect("connect fixture");
        let signal = ready
            .recv_timeout(Duration::from_secs(2))
            .expect("reader entered");
        signal.retire();
        let outcome = done.recv_timeout(Duration::from_millis(500));
        drop(client);
        server.join().expect("reader worker");
        assert_eq!(outcome.expect("idle retirement bound"), 0);
    }

    #[test]
    fn m4_nonreading_pipe_write_expires_and_releases_the_endpoint() {
        let name = format!(r"\\.\pipe\nxtools-m4-deadline-{}", uuid::Uuid::new_v4());
        let server_name = name.clone();
        let (finished, done) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let connection = accept_current_user_pipe(&server_name).expect("accept fixture");
            let (_reader, mut writer, _signal) = connection
                .into_retiring(Arc::new(AtomicBool::new(false)), Duration::from_millis(100))
                .expect("retirement event");
            let started = Instant::now();
            let failure = writer
                .write_all(&vec![b'x'; 1024 * 1024])
                .expect_err("nonreader fills bounded pipe buffer");
            finished
                .send((failure.kind(), started.elapsed()))
                .expect("write terminal");
        });
        let client = connect_local_pipe(&name, Duration::from_secs(2)).expect("connect fixture");
        let outcome = done.recv_timeout(Duration::from_millis(750));
        drop(client);
        server.join().expect("writer worker");
        let (kind, elapsed) = outcome.expect("native write deadline bound");
        assert_eq!(kind, ErrorKind::TimedOut);
        assert!(elapsed < Duration::from_millis(750));
    }

    #[test]
    fn m4_retirement_interrupts_a_nonreading_write_before_its_deadline() {
        let name = format!(r"\\.\pipe\nxtools-m4-retire-write-{}", uuid::Uuid::new_v4());
        let server_name = name.clone();
        let retired = Arc::new(AtomicBool::new(false));
        let writer_retired = Arc::clone(&retired);
        let (started, ready) = std::sync::mpsc::channel();
        let (finished, done) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let connection = accept_current_user_pipe(&server_name).expect("accept fixture");
            let (_reader, mut writer, signal) = connection
                .into_retiring(writer_retired, Duration::from_secs(5))
                .expect("retirement event");
            started.send(signal).expect("writer started");
            let failure = writer
                .write_all(&vec![b'x'; 1024 * 1024])
                .expect_err("retired write");
            finished.send(failure.kind()).expect("write terminal");
        });
        let client = connect_local_pipe(&name, Duration::from_secs(2)).expect("connect fixture");
        let signal = ready
            .recv_timeout(Duration::from_secs(2))
            .expect("writer entered");
        std::thread::sleep(Duration::from_millis(30));
        signal.retire();
        let outcome = done.recv_timeout(Duration::from_millis(500));
        drop(client);
        server.join().expect("writer worker");
        assert_eq!(
            outcome.expect("write retirement bound"),
            ErrorKind::BrokenPipe
        );
    }

    #[test]
    fn closed_peer_returns_eof_without_waiting_for_handshake_deadline() {
        let name = format!(r"\\.\pipe\nxtools-eof-{}", uuid::Uuid::new_v4());
        let server_name = name.clone();
        let (started, ready) = std::sync::mpsc::channel();
        let (finished, done) = std::sync::mpsc::channel();
        let server = std::thread::spawn(move || {
            let mut connection = accept_current_user_pipe(&server_name).expect("accept fixture");
            connection
                .reader
                .get_mut()
                .set_initial_frame_timeout(Duration::from_secs(2));
            started.send(()).expect("reader ready");
            let result = connection.reader.read(&mut [0_u8; 1]);
            finished.send(result).expect("read completed");
        });
        let client = connect_local_pipe(&name, Duration::from_secs(2)).expect("connect fixture");
        ready
            .recv_timeout(Duration::from_secs(2))
            .expect("accepted reader");
        drop(client);
        let outcome = done.recv_timeout(Duration::from_millis(600));
        server.join().expect("bounded reader worker");
        assert_eq!(
            outcome
                .expect("peer close wakes reader")
                .expect("native EOF"),
            0
        );
    }
}
