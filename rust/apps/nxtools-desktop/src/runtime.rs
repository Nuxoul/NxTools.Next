use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result};
use nxtools_clipboard::ClipboardHistory;
use nxtools_core::{AppConfig, AppPaths, LogGuard, SingleInstanceGuard, init_file_logging};
use tracing::{error, info, warn};

pub struct DesktopRuntime {
    pub config: AppConfig,
    pub platform_bridge: Option<nxtools_ipc::PlatformBridge>,
    pub clipboard_history: Option<ClipboardHistory>,
    _instance: SingleInstanceGuard,
    _log_guard: LogGuard,
    _platform_host: Option<PlatformHostSupervisor>,
}

impl DesktopRuntime {
    pub fn initialize() -> Result<Self> {
        let paths = AppPaths::discover()?;
        let instance = SingleInstanceGuard::acquire(&paths, "desktop")?;
        let log_guard = init_file_logging(&paths, "desktop")?;
        let config = AppConfig::load_or_create(&paths)?;
        let clipboard_history = ClipboardHistory::open(paths.data_dir.join("clipboard.db"))
            .context("failed to open clipboard history")?;
        let (platform_bridge, platform_server) = nxtools_ipc::platform_bridge(256);
        start_platform_server(config.platform_pipe_name.clone(), platform_server)?;
        let platform_host = PlatformHostSupervisor::launch_if_packaged()?;
        info!("desktop runtime initialized");

        Ok(Self {
            config,
            platform_bridge: Some(platform_bridge),
            clipboard_history: Some(clipboard_history),
            _instance: instance,
            _log_guard: log_guard,
            _platform_host: platform_host,
        })
    }
}

fn start_platform_server(
    pipe_name: String,
    platform_server: nxtools_ipc::PlatformServerEndpoint,
) -> Result<()> {
    thread::Builder::new()
        .name("platform-ipc".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            match runtime {
                Ok(runtime) => {
                    if let Err(error) = runtime.block_on(nxtools_ipc::run_platform_server(
                        &pipe_name,
                        platform_server,
                    )) {
                        error!(%error, "platform IPC server stopped");
                    }
                }
                Err(error) => error!(%error, "failed to create IPC runtime"),
            }
        })
        .context("failed to start platform IPC thread")?;
    Ok(())
}

struct PlatformHostSupervisor {
    stop: Sender<()>,
    thread: Option<JoinHandle<()>>,
}

impl PlatformHostSupervisor {
    fn launch_if_packaged() -> Result<Option<Self>> {
        let executable = std::env::current_exe().context("failed to locate desktop executable")?;
        let Some(directory) = executable.parent() else {
            return Ok(None);
        };
        let host_path = directory.join("NxTools.PlatformHost.exe");
        if !host_path.exists() {
            info!(path = %host_path.display(), "packaged PlatformHost not present; development launcher owns it");
            return Ok(None);
        }

        let (stop, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name("platform-host-supervisor".into())
            .spawn(move || supervise_platform_host(host_path, receiver))
            .context("failed to start PlatformHost supervisor")?;
        Ok(Some(Self {
            stop,
            thread: Some(thread),
        }))
    }
}

impl Drop for PlatformHostSupervisor {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            warn!("PlatformHost supervisor panicked while stopping");
        }
    }
}

fn supervise_platform_host(host_path: PathBuf, receiver: Receiver<()>) {
    loop {
        let mut child = match spawn_platform_host(&host_path) {
            Ok(child) => child,
            Err(error) => {
                error!(%error, path = %host_path.display(), "failed to launch PlatformHost");
                if receiver.recv_timeout(Duration::from_secs(2)) != Err(RecvTimeoutError::Timeout) {
                    return;
                }
                continue;
            }
        };
        info!(pid = child.id(), "PlatformHost launched");

        loop {
            match receiver.recv_timeout(Duration::from_secs(1)) {
                Ok(()) | Err(RecvTimeoutError::Disconnected) => {
                    stop_child(&mut child);
                    return;
                }
                Err(RecvTimeoutError::Timeout) => match child.try_wait() {
                    Ok(Some(status)) => {
                        warn!(%status, "PlatformHost exited; restarting");
                        break;
                    }
                    Ok(None) => {}
                    Err(error) => {
                        error!(%error, "failed to inspect PlatformHost");
                        stop_child(&mut child);
                        break;
                    }
                },
            }
        }

        if receiver.recv_timeout(Duration::from_secs(1)) != Err(RecvTimeoutError::Timeout) {
            return;
        }
    }
}

fn stop_child(child: &mut Child) {
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

#[cfg(windows)]
fn spawn_platform_host(path: &Path) -> std::io::Result<Child> {
    use std::os::windows::process::CommandExt;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    Command::new(path)
        .args(["--parent-pid", &std::process::id().to_string()])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
}

#[cfg(not(windows))]
fn spawn_platform_host(path: &Path) -> std::io::Result<Child> {
    Command::new(path)
        .args(["--parent-pid", &std::process::id().to_string()])
        .spawn()
}
