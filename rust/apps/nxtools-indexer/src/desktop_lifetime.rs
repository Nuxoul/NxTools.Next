//! Desktop process-object leases for the demand-started Windows service.
#![allow(unsafe_code)] // Owns and closes only the process handles opened below.

use std::collections::HashMap;
use std::io;
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{
    CloseHandle, HANDLE, WAIT_FAILED, WAIT_OBJECT_0, WAIT_TIMEOUT,
};
use windows_sys::Win32::System::Threading::{OpenProcess, WaitForSingleObject};
const PROCESS_SYNCHRONIZE: u32 = 0x0010_0000;

struct ProcessHandle(HANDLE);
// The handle is immutable after opening; polling/closing is serialized by the owner's mutex.
unsafe impl Send for ProcessHandle {}

impl ProcessHandle {
    fn open(pid: u32) -> io::Result<Self> {
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        if handle.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Self(handle))
        }
    }

    fn alive(&self) -> io::Result<bool> {
        match unsafe { WaitForSingleObject(self.0, 0) } {
            WAIT_TIMEOUT => Ok(true),
            WAIT_OBJECT_0 => Ok(false),
            WAIT_FAILED => Err(io::Error::last_os_error()),
            _ => Err(io::Error::other("unexpected process wait result")),
        }
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.0) };
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub(crate) enum StopReason {
    NoOwner,
    LastOwnerExited,
}

pub(crate) struct DesktopLifetime {
    owners: HashMap<u32, ProcessHandle>,
    ever_admitted: bool,
    startup_deadline: Instant,
}

impl DesktopLifetime {
    pub(crate) fn new(now: Instant, startup_wait: Duration) -> Self {
        Self {
            owners: HashMap::new(),
            ever_admitted: false,
            startup_deadline: now + startup_wait,
        }
    }

    // Called only after native PID/session/path approval and a deadline-bounded Hello.
    pub(crate) fn admit(&mut self, pid: u32) -> io::Result<bool> {
        if let Some(handle) = self.owners.get(&pid) {
            if handle.alive()? {
                return Ok(false);
            }
            self.owners.remove(&pid);
        }
        let handle = ProcessHandle::open(pid)?;
        if !handle.alive()? {
            return Err(io::Error::other("approved desktop exited before admission"));
        }
        self.owners.insert(pid, handle);
        self.ever_admitted = true;
        Ok(true)
    }

    pub(crate) fn poll(&mut self, now: Instant) -> io::Result<Option<StopReason>> {
        let mut exited = Vec::new();
        for (pid, handle) in &self.owners {
            if !handle.alive()? {
                exited.push(*pid);
            }
        }
        for pid in exited {
            self.owners.remove(&pid);
        }
        if self.ever_admitted && self.owners.is_empty() {
            Ok(Some(StopReason::LastOwnerExited))
        } else if !self.ever_admitted && now >= self.startup_deadline {
            Ok(Some(StopReason::NoOwner))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn count(&self) -> usize {
        self.owners.len()
    }

    pub(crate) fn clear(&mut self) {
        self.owners.clear();
    }
}

#[cfg(test)]
#[path = "desktop_lifetime_tests.rs"]
mod tests;
