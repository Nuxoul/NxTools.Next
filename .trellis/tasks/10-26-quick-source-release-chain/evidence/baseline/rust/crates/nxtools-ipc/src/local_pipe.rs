#![allow(unsafe_code)]

use std::ffi::c_void;
use std::fs::File;
use std::io::{BufReader, ErrorKind, Read};
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::ptr;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use windows_sys::Win32::Foundation::{
    CloseHandle, DUPLICATE_SAME_ACCESS, DuplicateHandle, ERROR_BROKEN_PIPE,
    ERROR_INSUFFICIENT_BUFFER, ERROR_NO_DATA, ERROR_PIPE_CONNECTED, ERROR_PIPE_NOT_CONNECTED,
    GetLastError, INVALID_HANDLE_VALUE, LocalFree,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
    TokenUser,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, FILE_ATTRIBUTE_NORMAL, OPEN_EXISTING, PIPE_ACCESS_DUPLEX,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, GetNamedPipeClientProcessId, GetNamedPipeServerProcessId,
    PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
    PIPE_WAIT, PeekNamedPipe, WaitNamedPipeW,
};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::{
    GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION,
    QueryFullProcessImageNameW,
};

pub const INDEXER_PIPE_NAME: &str = r"\\.\pipe\nxtools-next-indexer-v1";
const PIPE_BUFFER_BYTES: u32 = 64 * 1024;
const SDDL_REVISION_1: u32 = 1;
// SYSTEM and Administrators: full access. Authenticated users: read/write/synchronize only.
const CLIENT_PIPE_ACCESS: u32 = 0x0012_019B;
const PIPE_SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;0x12019b;;;AU)";

pub struct LocalPipeConnection {
    pub reader: BufReader<LocalPipeReader>,
    pub writer: File,
    pub client_process_id: Option<u32>,
    pub client_session_id: Option<u32>,
}

pub struct LocalPipeReader {
    file: File,
    initial_frame_deadline: Option<Instant>,
}

impl LocalPipeReader {
    /// Bounds an untrusted initial newline-framed handshake, not a healthy
    /// long-lived session. Partial trickle reads do not reset this deadline.
    pub fn set_initial_frame_timeout(&mut self, timeout: Duration) {
        self.initial_frame_deadline = Some(Instant::now() + timeout);
    }
}

impl Read for LocalPipeReader {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let mut idle_wait = Duration::from_millis(1);
        loop {
            if self
                .initial_frame_deadline
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                return Err(std::io::Error::new(
                    ErrorKind::TimedOut,
                    "local pipe handshake expired",
                ));
            }
            let mut available = 0_u32;
            let peeked = unsafe {
                PeekNamedPipe(
                    self.file.as_raw_handle(),
                    ptr::null_mut(),
                    0,
                    ptr::null_mut(),
                    &raw mut available,
                    ptr::null_mut(),
                )
            };
            if peeked == 0 {
                let error = unsafe { GetLastError() };
                if matches!(
                    error,
                    ERROR_BROKEN_PIPE | ERROR_NO_DATA | ERROR_PIPE_NOT_CONNECTED
                ) {
                    return Ok(0);
                }
                return Err(std::io::Error::from_raw_os_error(error as i32));
            }
            if available == 0 {
                std::thread::sleep(idle_wait);
                idle_wait = (idle_wait * 2).min(Duration::from_millis(8));
                continue;
            }
            let bytes = (available as usize).min(buffer.len());
            let read = self.file.read(&mut buffer[..bytes])?;
            if buffer[..read].contains(&b'\n') {
                self.initial_frame_deadline = None;
            }
            return Ok(read);
        }
    }
}

pub fn accept_local_indexer_pipe(pipe_name: &str) -> Result<LocalPipeConnection> {
    accept_pipe_with_sddl(pipe_name, PIPE_SDDL, "indexer")
}

pub fn accept_current_user_pipe(pipe_name: &str) -> Result<LocalPipeConnection> {
    let user_sid = current_user_sid_string()?;
    let sddl = format!("D:(A;;GA;;;SY)(A;;GA;;;{user_sid})");
    accept_pipe_with_sddl(pipe_name, &sddl, "current-user")
}

fn accept_pipe_with_sddl(
    pipe_name: &str,
    pipe_sddl: &str,
    pipe_kind: &str,
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
    let handle = unsafe {
        CreateNamedPipeW(
            pipe_name.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            PIPE_BUFFER_BYTES,
            PIPE_BUFFER_BYTES,
            500,
            &attributes,
        )
    };
    unsafe { LocalFree(descriptor.cast()) };
    if handle == INVALID_HANDLE_VALUE {
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("failed to create {pipe_kind} named pipe"));
    }
    let connected = unsafe { ConnectNamedPipe(handle, ptr::null_mut()) };
    if connected == 0 && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED {
        unsafe { drop(File::from_raw_handle(handle)) };
        return Err(std::io::Error::last_os_error())
            .with_context(|| format!("failed to accept {pipe_kind} pipe client"));
    }
    let identity = server_client_identity(handle);
    connection_from_handle(handle, identity)
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
    let deadline = Instant::now() + timeout;
    loop {
        let handle = unsafe {
            CreateFileW(
                pipe_name.as_ptr(),
                CLIENT_PIPE_ACCESS,
                0,
                ptr::null(),
                OPEN_EXISTING,
                FILE_ATTRIBUTE_NORMAL,
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
                && wait_error.raw_os_error() != Some(231)
            {
                return Err(wait_error).context("failed waiting for the indexer service pipe");
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
    };
    let writer = unsafe { File::from_raw_handle(writer_handle) };
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
pub(crate) fn write_desktop_frame(file: &mut File, bytes: &[u8], timeout: Duration) -> Result<()> {
    use std::io::Write;
    use windows_sys::Win32::System::Pipes::{PIPE_NOWAIT, SetNamedPipeHandleState};
    let mode = PIPE_READMODE_BYTE | PIPE_NOWAIT;
    if unsafe { SetNamedPipeHandleState(file.as_raw_handle(), &mode, ptr::null(), ptr::null()) }
        == 0
    {
        return Err(std::io::Error::last_os_error()).context("desktop pipe write mode failed");
    }
    let deadline = Instant::now() + timeout;
    let result = (|| -> Result<()> {
        let mut written = 0;
        while written < bytes.len() {
            if Instant::now() >= deadline {
                bail!("desktop pipe write expired");
            }
            match file.write(&bytes[written..]) {
                Ok(0) => std::thread::sleep(Duration::from_millis(2)),
                Ok(count) => written += count,
                Err(error)
                    if error.kind() == ErrorKind::WouldBlock
                        || error.raw_os_error() == Some(232) =>
                {
                    std::thread::sleep(Duration::from_millis(2))
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    })();
    let mode = PIPE_READMODE_BYTE | PIPE_WAIT;
    if unsafe { SetNamedPipeHandleState(file.as_raw_handle(), &mode, ptr::null(), ptr::null()) }
        == 0
    {
        return Err(std::io::Error::last_os_error()).context("desktop pipe restore mode failed");
    }
    result
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
}
