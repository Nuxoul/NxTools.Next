#![allow(unsafe_code)]

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use std::{fs::File, io::Read};

use nxtools_protocol::{
    ElevatedDirectoryHelperRequest, ElevatedDirectoryHelperStatus, Envelope, IndexerCommand,
    IndexerEvent, Message,
};
use sha2::{Digest, Sha256};
use windows_sys::Win32::Foundation::{CloseHandle, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, REG_SZ, RegCloseKey, RegOpenKeyExW, RegQueryValueExW,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;
const PIPE_NAME_PREFIX: &str = "nxtools-next-elevated-nav-";
const AUTHENTICATION_TOKEN_HEX_LENGTH: usize = 64;
const BROKER_TIMEOUT: Duration = Duration::from_secs(3);
const BROKER_POLL_INTERVAL: Duration = Duration::from_millis(20);
const SERVICE_REGISTRY_KEY: &str = r"SYSTEM\CurrentControlSet\Services\NxToolsIndexerV1";
const DESKTOP_HASH_VALUE: &str = "NxToolsDesktopSha256";
const DESKTOP_PATH_VALUE: &str = "NxToolsDesktopPath";
const PLATFORM_HOST_HASH_VALUE: &str = "NxToolsPlatformHostSha256";

pub fn authorized_packaged_desktop(process_id: u32) -> bool {
    let Some(expected_path) = read_service_registry_string(DESKTOP_PATH_VALUE) else {
        return false;
    };
    query_process_path(process_id).is_some_and(|client_path| {
        packaged_desktop_path_matches(Path::new(&expected_path), &client_path)
            && registered_hash_matches(DESKTOP_HASH_VALUE, &client_path)
    })
}

pub fn handle_service_command(
    request: &Envelope,
    client_process_id: u32,
    session_id: u32,
    service_path: &Path,
) -> Option<Vec<Envelope>> {
    let Message::IndexerCommand(IndexerCommand::EnsureElevatedDirectoryHelper(helper)) =
        &request.message
    else {
        return None;
    };
    let status = match require_packaged_desktop(client_process_id)
        .and_then(|()| validate_request(helper))
        .and_then(|()| packaged_platform_host_path(service_path))
        .and_then(|platform_host| launch_broker(&platform_host, session_id, helper))
    {
        Ok(()) => ElevatedDirectoryHelperStatus {
            pipe_name: helper.pipe_name.clone(),
            launched: true,
            error: None,
        },
        Err(error) => ElevatedDirectoryHelperStatus {
            pipe_name: helper.pipe_name.clone(),
            launched: false,
            error: Some(error),
        },
    };
    Some(vec![Envelope::reply_to(
        request,
        Message::IndexerEvent(IndexerEvent::ElevatedDirectoryHelper(status)),
    )])
}

fn require_packaged_desktop(process_id: u32) -> Result<(), String> {
    authorized_packaged_desktop(process_id)
        .then_some(())
        .ok_or_else(|| {
            "the elevated directory helper request did not come from the packaged desktop".into()
        })
}

fn validate_request(request: &ElevatedDirectoryHelperRequest) -> Result<(), String> {
    let valid_pipe_name = request.pipe_name.starts_with(PIPE_NAME_PREFIX)
        && request.pipe_name.len() <= 96
        && request
            .pipe_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-');
    if !valid_pipe_name {
        return Err("the elevated helper pipe name is invalid".into());
    }
    if request.authentication_token.len() != AUTHENTICATION_TOKEN_HEX_LENGTH
        || !request
            .authentication_token
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("the elevated helper authentication token is invalid".into());
    }
    Ok(())
}

fn packaged_platform_host_path(service_path: &Path) -> Result<PathBuf, String> {
    let directory = service_path
        .parent()
        .ok_or_else(|| "the indexer service directory is unavailable".to_owned())?;
    let platform_host = directory.join("NxTools.PlatformHost.exe");
    if !platform_host.is_file() {
        return Err("NxTools.PlatformHost.exe is not packaged beside the indexer service".into());
    }
    if !registered_hash_matches(PLATFORM_HOST_HASH_VALUE, &platform_host) {
        return Err(
            "NxTools.PlatformHost.exe does not match the administrator-approved package hash"
                .into(),
        );
    }
    Ok(platform_host)
}

fn registered_hash_matches(value_name: &str, path: &Path) -> bool {
    let Some(expected) = read_service_registry_string(value_name) else {
        return false;
    };
    file_sha256(path).is_some_and(|actual| actual.eq_ignore_ascii_case(expected.trim()))
}

fn file_sha256(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(format!("{:X}", hasher.finalize()))
}

fn read_service_registry_string(value_name: &str) -> Option<String> {
    let key_path = wide_null(SERVICE_REGISTRY_KEY);
    let mut key: HKEY = std::ptr::null_mut();
    if unsafe {
        RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            key_path.as_ptr(),
            0,
            KEY_QUERY_VALUE,
            &raw mut key,
        )
    } != ERROR_SUCCESS
    {
        return None;
    }
    let value_name = wide_null(value_name);
    let mut value_type = 0_u32;
    let mut byte_count = 0_u32;
    let measured = unsafe {
        RegQueryValueExW(
            key,
            value_name.as_ptr(),
            std::ptr::null(),
            &raw mut value_type,
            std::ptr::null_mut(),
            &raw mut byte_count,
        )
    };
    if measured != ERROR_SUCCESS || value_type != REG_SZ || !(2..=512).contains(&byte_count) {
        unsafe {
            RegCloseKey(key);
        }
        return None;
    }
    let mut buffer = vec![0_u16; usize::try_from(byte_count).ok()?.div_ceil(2)];
    let loaded = unsafe {
        RegQueryValueExW(
            key,
            value_name.as_ptr(),
            std::ptr::null(),
            &raw mut value_type,
            buffer.as_mut_ptr().cast(),
            &raw mut byte_count,
        )
    };
    unsafe {
        RegCloseKey(key);
    }
    if loaded != ERROR_SUCCESS || value_type != REG_SZ {
        return None;
    }
    let length = buffer
        .iter()
        .position(|unit| *unit == 0)
        .unwrap_or(buffer.len());
    String::from_utf16(&buffer[..length]).ok()
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn launch_broker(
    platform_host: &Path,
    session_id: u32,
    request: &ElevatedDirectoryHelperRequest,
) -> Result<(), String> {
    let mut child = Command::new(platform_host)
        .arg("--launch-elevated-directory-helper")
        .arg(session_id.to_string())
        .arg(&request.pipe_name)
        .arg(&request.authentication_token)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|error| format!("failed to start the packaged helper broker: {error}"))?;
    let deadline = Instant::now() + BROKER_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => {
                return Err(format!(
                    "the packaged helper broker exited with code {}",
                    status.code().unwrap_or(-1)
                ));
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(BROKER_POLL_INTERVAL),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("the packaged helper broker timed out".into());
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "failed while waiting for the packaged helper broker: {error}"
                ));
            }
        }
    }
}

fn packaged_desktop_path_matches(expected_path: &Path, client_path: &Path) -> bool {
    client_path
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("NxTools.exe"))
        && paths_equal(expected_path, client_path)
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    normalize_path(left) == normalize_path(right)
}

fn normalize_path(path: &Path) -> String {
    let value = path.to_string_lossy().replace('/', "\\");
    value
        .strip_prefix(r"\\?\")
        .unwrap_or(&value)
        .trim_end_matches('\\')
        .to_ascii_lowercase()
}

fn query_process_path(process_id: u32) -> Option<PathBuf> {
    let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, process_id) };
    if process.is_null() {
        return None;
    }
    let mut buffer = vec![0_u16; 32_768];
    let mut length = buffer.len() as u32;
    let queried =
        unsafe { QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &raw mut length) };
    unsafe {
        CloseHandle(process);
    }
    (queried != 0 && length > 0).then(|| {
        PathBuf::from(String::from_utf16_lossy(
            &buffer[..usize::try_from(length).unwrap_or(0)],
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_packaged_desktop_is_an_authorized_service_client() {
        let desktop = Path::new(r"C:\Program Files\NxTools\NxTools.exe");
        assert!(packaged_desktop_path_matches(
            desktop,
            Path::new(r"\\?\C:\Program Files\NxTools\NxTools.exe")
        ));
        assert!(!packaged_desktop_path_matches(
            desktop,
            Path::new(r"C:\Temp\NxTools.exe")
        ));
        assert!(!packaged_desktop_path_matches(
            desktop,
            Path::new(r"C:\Program Files\NxTools\Other.exe")
        ));
    }

    #[test]
    fn helper_rendezvous_rejects_paths_and_short_tokens() {
        let valid = ElevatedDirectoryHelperRequest {
            pipe_name: format!("{PIPE_NAME_PREFIX}0123456789abcdef"),
            authentication_token: "ab".repeat(32),
        };
        assert!(validate_request(&valid).is_ok());
        assert!(
            validate_request(&ElevatedDirectoryHelperRequest {
                pipe_name: r"nxtools-next-elevated-nav-..\other".into(),
                ..valid.clone()
            })
            .is_err()
        );
        assert!(
            validate_request(&ElevatedDirectoryHelperRequest {
                authentication_token: "short".into(),
                ..valid
            })
            .is_err()
        );
    }

    #[test]
    fn invalid_helper_commands_return_a_correlated_failure_without_launching() {
        let request = Envelope::new(Message::IndexerCommand(
            IndexerCommand::EnsureElevatedDirectoryHelper(ElevatedDirectoryHelperRequest {
                pipe_name: r"nxtools-next-elevated-nav-..\other".into(),
                authentication_token: "ab".repeat(32),
            }),
        ));
        let responses = handle_service_command(
            &request,
            std::process::id(),
            1,
            Path::new(r"C:\Missing\NxTools.Indexer.exe"),
        )
        .expect("helper command is intercepted");
        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].correlation_id, Some(request.id));
        assert!(matches!(
            &responses[0].message,
            Message::IndexerEvent(IndexerEvent::ElevatedDirectoryHelper(status))
                if !status.launched && status.error.is_some()
        ));
    }
}
