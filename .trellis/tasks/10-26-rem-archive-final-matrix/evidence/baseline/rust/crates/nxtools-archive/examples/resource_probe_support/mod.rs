//! Test-tool ownership only. Never a replacement for the C# native Job owner.
mod control;
#[cfg(windows)]
pub mod product;
use control::read_bounded;
pub use control::{EngineMonitor, POLL, ProbeStatus};
use serde_json::json;
use std::{io, path::PathBuf, process::Child, sync::Arc, time::Duration};
use uuid::Uuid;

pub trait OwnedChild {
    fn exited(&mut self) -> io::Result<bool>;
    fn force(&mut self) -> io::Result<()>;
}
impl OwnedChild for Child {
    fn exited(&mut self) -> io::Result<bool> {
        self.try_wait().map(|value| value.is_some())
    }
    fn force(&mut self) -> io::Result<()> {
        self.kill()
    }
}
#[derive(Default)]
pub struct DrainReport {
    pub exited: bool,
    pub forced: bool,
    pub errors: Vec<(&'static str, Option<i32>)>,
}
/// Actual guard seam, also used by deterministic ordering tests. No unbounded Child::wait.
pub fn drain_owned_child(
    child: &mut impl OwnedChild,
    close_pipe: impl FnOnce(),
    attempts: usize,
    mut pause: impl FnMut(),
    mut expired: impl FnMut() -> bool,
    before_force: impl FnOnce(),
) -> DrainReport {
    close_pipe(); // EOF to the original session, never a new connection or stop receipt.
    let mut report = DrainReport::default();
    for _ in 0..=attempts {
        match child.exited() {
            Ok(true) => {
                report.exited = true;
                return report;
            }
            Ok(false) => {}
            Err(failure) => {
                if report.errors.is_empty() {
                    report
                        .errors
                        .push(("host_wait_failure", failure.raw_os_error()));
                }
            }
        }
        if expired() {
            break;
        }
        pause();
    }
    before_force();
    report.forced = true;
    if let Err(failure) = child.force() {
        report
            .errors
            .push(("host_force_failure", failure.raw_os_error()));
    }
    // Explicit finite final observation, not force receipt => cleanup.
    for _ in 0..50 {
        match child.exited() {
            Ok(true) => {
                report.exited = true;
                break;
            }
            Ok(false) => {}
            Err(failure) => {
                if report.errors.len() < 3 {
                    report
                        .errors
                        .push(("host_final_wait_failure", failure.raw_os_error()));
                }
            }
        }
        pause();
    }
    report
}

/// Declared AFTER Fixture: unwinding drops this runtime (and detached connection task)
/// BEFORE Fixture waits for the host. Aborting run_platform_server alone is insufficient.
#[cfg(windows)]
pub struct PipeRuntime {
    runtime: Option<tokio::runtime::Runtime>,
    status: Arc<ProbeStatus>,
}
#[cfg(windows)]
impl PipeRuntime {
    pub fn new(status: Arc<ProbeStatus>) -> io::Result<Self> {
        Ok(Self {
            runtime: Some(
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?,
            ),
            status,
        })
    }
    pub fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        self.runtime
            .as_ref()
            .expect("owned runtime")
            .block_on(future)
    }
}
#[cfg(windows)]
impl Drop for PipeRuntime {
    fn drop(&mut self) {
        drop(self.runtime.take()); // closes ORIGINAL pipe; permits C# await-using DrainAsync.
        self.status.fact("pipe_closed", json!(true));
    }
}

pub struct Fixture {
    pub root: PathBuf,
    pub host: Option<Child>,
    pub server: Option<tokio::task::JoinHandle<()>>,
    pub sources: Vec<(Uuid, PathBuf)>,
    pub status: Arc<ProbeStatus>,
    pub created: bool,
    finished: bool,
}
impl Fixture {
    pub fn new(root: PathBuf, status: Arc<ProbeStatus>) -> Self {
        Self {
            root,
            host: None,
            server: None,
            sources: Vec::new(),
            status,
            created: false,
            finished: false,
        }
    }
    pub fn shutdown(&mut self) -> bool {
        if self.finished {
            return self.status.document()["cleanup"]["confirmed"] == true;
        }
        if std::thread::panicking() {
            self.status.error("probe_panic", None);
            self.status.request_stop("probe_failure");
            self.status.operation(json!({"status":"Failed","reason":"probe_panic","operation_id":self.status.document()["operation_id"]}));
        }
        self.status.poll_stop();
        self.status.phase("Draining");
        let mut forced = false;
        let mut host_exited = true;
        if let Some(mut host) = self.host.take() {
            let server = self.server.take();
            let host_id = host.id();
            let attempts = self
                .status
                .cooperation_remaining()
                .as_millis()
                .div_ceil(20)
                .min(500) as usize;
            let host_deadline = std::time::Instant::now() + Duration::from_secs(10);
            let report = drain_owned_child(
                &mut host,
                || {
                    if let Some(server) = server {
                        server.abort();
                    }
                },
                attempts.max(1),
                || {
                    self.status.poll_stop();
                    std::thread::sleep(Duration::from_millis(20));
                },
                || {
                    self.status.cooperation_remaining().is_zero()
                        || std::time::Instant::now() >= host_deadline
                },
                || {
                    self.status.phase("Forced");
                    self.status.fact("forced_target", json!({"pid":host_id,"role":"isolated_csharp_host","reason":"cooperation_deadline"}));
                },
            );
            forced = report.forced;
            host_exited = report.exited;
            for (reason, native) in report.errors {
                self.status.error(reason, native);
            }
            if forced {
                self.status.error("cooperation_deadline", None);
                self.status.fact("forced_target", json!({"pid":host.id(),"role":"isolated_csharp_host","reason":"cooperation_deadline"}));
            }
        } else if let Some(server) = self.server.take() {
            server.abort();
        }
        self.status.fact("host_exited", json!(host_exited));
        let pipe_closed = self.status.document()["cleanup"]["pipe_closed"] == true || !self.created;
        let mut remote = true;
        let mut source_released = true;
        for (id, source) in &self.sources {
            // A local Cancelled/receipt never participates in this proof. CaptureTerminal is
            // written by the SAME C# session only after the original runner's exited cleanup.
            let clean = match read_bounded(&self.root.join(format!("terminal-{id}.json"))) {
                Ok(bytes) => match serde_json::from_slice::<
                    nxtools_protocol::archive_read::ArchiveReadTerminal,
                >(&bytes)
                {
                    Ok(terminal) => terminal.cleanup_complete,
                    Err(_) => {
                        self.status.error("remote_terminal_invalid", None);
                        false
                    }
                },
                Err(failure) => {
                    self.status
                        .error("remote_terminal_read_failure", failure.raw_os_error());
                    false
                }
            };
            remote &= clean;
            if !clean {
                self.status.error("remote_cleanup_unconfirmed", None);
            }
            match std::fs::OpenOptions::new().write(true).open(source) {
                Ok(handle) => drop(handle),
                Err(failure) => {
                    source_released = false;
                    self.status
                        .error("source_release_unconfirmed", failure.raw_os_error());
                }
            }
        }
        self.status.fact("remote_cleanup_complete", json!(remote));
        self.status.fact("source_released", json!(source_released));
        // Without remote cleanup and host exit, even owned partials/helpers are retained.
        let safe = !forced && host_exited && pipe_closed && remote && source_released;
        let package_removed = if safe && self.created {
            self.remove_known_package()
        } else {
            !self.created
        };
        self.status.fact("package_removed", json!(package_removed));
        self.status
            .fact("helper_released", json!(safe && package_removed));
        self.status.finish(safe && package_removed, forced);
        self.finished = true;
        safe && package_removed
    }
    fn remove_known_package(&self) -> bool {
        // This package is read-only: helper + known marker + exact operation terminal records.
        // Unknown files/subdirectories are evidence, NEVER swept recursively.
        let mut known = vec![
            self.root.join("NxTools.ArchiveWorker.exe"),
            self.root.join("capture-terminal"),
        ];
        for (id, _) in &self.sources {
            known.push(self.root.join(format!("terminal-{id}.json")));
            known.push(self.root.join(format!("terminal-{id}.json.tmp")));
        }
        let entries = match std::fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(failure) => {
                self.status
                    .error("package_observe_failure", failure.raw_os_error());
                return false;
            }
        };
        for entry in entries {
            let Ok(entry) = entry else {
                self.status.error("package_observe_failure", None);
                return false;
            };
            if !known.contains(&entry.path())
                || !entry
                    .file_type()
                    .is_ok_and(|kind| kind.is_file() && !kind.is_symlink())
            {
                self.status.error("unknown_package_entry", None);
                return false;
            }
        }
        for path in known {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(failure) if failure.kind() == io::ErrorKind::NotFound => {}
                Err(failure) => {
                    self.status
                        .error("package_file_release_failure", failure.raw_os_error());
                    return false;
                }
            }
        }
        match std::fs::remove_dir(&self.root) {
            Ok(()) => true,
            Err(failure) => {
                self.status
                    .error("package_remove_failure", failure.raw_os_error());
                false
            }
        }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests;
