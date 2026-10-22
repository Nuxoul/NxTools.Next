//! Private, bounded file control for this opt-in tool; never a product IPC endpoint.
use nxtools_archive::OperationControl;
use serde_json::{Value, json};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
    thread::JoinHandle,
    time::{Duration, Instant},
};
use uuid::Uuid;

const MAX_CONTROL: u64 = 16 * 1024;
pub const POLL: Duration = Duration::from_millis(20);
struct State {
    document: Value,
    finished: bool,
    stop_at: Option<Instant>,
}
pub struct ProbeStatus {
    directory: PathBuf,
    run_id: String,
    state: Mutex<State>,
}
impl ProbeStatus {
    pub fn from_env(root: &Path) -> io::Result<Arc<Self>> {
        let directory = PathBuf::from(
            std::env::var_os("NXTOOLS_RESOURCE_CONTROL")
                .ok_or_else(|| io::Error::other("missing_control"))?,
        );
        if !directory.is_absolute() || directory.parent() != Some(root) {
            return Err(io::Error::other("control_not_owned"));
        }
        let run_id = std::env::var("NXTOOLS_RESOURCE_RUN_ID")
            .map_err(|_| io::Error::other("missing_run_id"))?;
        let id = Uuid::parse_str(&run_id).map_err(|_| io::Error::other("invalid_run_id"))?;
        if id.is_nil() || id.to_string() != run_id {
            return Err(io::Error::other("invalid_run_id"));
        }
        Self::new(&directory, id)
    }
    pub fn new(directory: &Path, run_id: Uuid) -> io::Result<Arc<Self>> {
        let metadata = std::fs::symlink_metadata(directory)?;
        if !directory.is_absolute() || !metadata.is_dir() || is_reparse(&metadata) {
            return Err(io::Error::other("control_not_owned"));
        }
        let directory = directory.canonicalize()?;
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            if !matches!(entry.file_name().to_str(), Some("stop.json" | "stop.tmp")) {
                return Err(io::Error::other("control_not_fresh"));
            }
        }
        // A create-new claim prevents a second probe writer or reuse of an old run directory.
        let mut claim = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("probe-owner"))?;
        claim.write_all(run_id.to_string().as_bytes())?;
        if directory.join("status.json").exists() || directory.join("status.tmp").exists() {
            return Err(io::Error::other("control_not_fresh"));
        }
        let status = Arc::new(Self {
            directory: directory.to_owned(),
            run_id: run_id.to_string(),
            state: Mutex::new(State {
                finished: false,
                stop_at: None,
                document: json!({
                    "schema":2,"run_id":run_id,"state":"Running", "identities":[],"expected_images":{},
                    "cleanup":{"status":"Unknown","confirmed":false},
                    "stop":{"requested":false,"reason":null,"dispatch_count":0},
                    "operation":{"status":"NotStarted"},"errors":[]
                }),
            }),
        });
        let executable = exact_image(std::env::current_exe()?.canonicalize()?);
        status.identity(std::process::id(), None, "probe_owner", &executable, None);
        Ok(status)
    }
    pub fn run_id(&self) -> String {
        self.run_id.clone()
    }
    pub fn cooperation_remaining(&self) -> Duration {
        let state = self
            .state
            .lock()
            .unwrap_or_else(|failure| failure.into_inner());
        state.stop_at.map_or(Duration::from_secs(10), |start| {
            Duration::from_secs(10).saturating_sub(start.elapsed())
        })
    }
    pub fn bind_operation(&self, id: Uuid) {
        self.update(|state| {
            state.document["operation_id"] = json!(id);
            // Keep prior operation provenance; same host/helper can serve a later ordinary
            // assertion only when there has been no resource stop.
            for identity in state.document["identities"]
                .as_array_mut()
                .expect("identities")
            {
                if identity["operation_id"].is_null() {
                    identity["operation_id"] = json!(id);
                }
            }
            if state.document["worker_observation"].is_object() {
                state.document["worker_observation"]["operation_id"] = json!(id);
            }
        });
    }
    fn update(&self, action: impl FnOnce(&mut State)) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|failure| failure.into_inner());
        action(&mut state);
        let result = (|| -> io::Result<()> {
            let bytes = serde_json::to_vec(&state.document)?;
            if bytes.len() as u64 > MAX_CONTROL {
                return Err(io::Error::other("status_too_large"));
            }
            let mut temporary = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(self.directory.join("status.tmp"))?;
            temporary.write_all(&bytes)?;
            drop(temporary);
            std::fs::rename(
                self.directory.join("status.tmp"),
                self.directory.join("status.json"),
            )
        })();
        if let Err(failure) = result {
            let first_failure = !state.document["errors"]
                .as_array()
                .expect("errors")
                .iter()
                .any(|row| row["reason"] == "status_write_failure");
            add_error(
                &mut state.document,
                "status_write_failure",
                failure.raw_os_error(),
            );
            accept_stop(&mut state, "control_write_failure");
            // One fixed diagnostic, never a high-frequency retry log or substitute evidence.
            if first_failure {
                eprintln!(
                    "RESOURCE_CONTROL_WRITE_FAILED native={:?}",
                    failure.raw_os_error()
                );
            }
        }
    }
    pub fn document(&self) -> Value {
        self.state
            .lock()
            .unwrap_or_else(|failure| failure.into_inner())
            .document
            .clone()
    }
    pub fn stopped(&self) -> bool {
        self.document()["stop"]["requested"].as_bool() == Some(true)
    }
    pub fn request_stop(&self, reason: &'static str) {
        self.update(|state| accept_stop(state, reason));
    }
    pub fn poll_stop(&self) -> bool {
        {
            let state = self
                .state
                .lock()
                .unwrap_or_else(|failure| failure.into_inner());
            if state.finished || state.document["stop"]["requested"] == true {
                return state.document["stop"]["requested"] == true;
            }
        }
        match read_bounded(&self.directory.join("stop.json")) {
            Ok(bytes) => match serde_json::from_slice::<Value>(&bytes) {
                Ok(value) if value["run_id"].as_str() == Some(self.run_id.as_str()) => {
                    let reason = fixed_reason(value["reason"].as_str().unwrap_or("invalid_stop"));
                    self.request_stop(reason);
                }
                Ok(_) => {} // Stale/foreign run intent has no authority over this operation.
                Err(_) => {
                    self.error("control_invalid_json", None);
                    self.request_stop("control_read_failure");
                }
            },
            Err(failure) if failure.kind() == io::ErrorKind::NotFound => {
                if !self.directory.is_dir() {
                    self.request_stop("control_read_failure");
                    self.error("control_directory_missing", failure.raw_os_error());
                }
            }
            Err(failure) => {
                self.error("control_read_failure", failure.raw_os_error());
                self.request_stop("control_read_failure");
            }
        }
        self.stopped()
    }
    pub fn phase(&self, phase: &'static str) {
        self.update(|state| {
            if !state.finished {
                state.document["state"] = json!(phase);
            }
        });
    }
    pub fn fact(&self, name: &'static str, value: Value) {
        self.update(|state| state.document["cleanup"][name] = value);
    }
    pub fn operation(&self, value: Value) {
        self.update(|state| state.document["operation"] = value);
    }
    pub fn error(&self, reason: &'static str, native_error: Option<i32>) {
        self.update(|state| add_error(&mut state.document, reason, native_error));
    }
    pub fn identity(
        &self,
        pid: u32,
        parent_pid: Option<u32>,
        role: &'static str,
        executable: &Path,
        operation_id: Option<Uuid>,
    ) {
        self.update(|state| {
            let identities = state.document["identities"].as_array_mut().expect("bounded identities");
            if identities.len() < 32 { identities.push(json!({"pid":pid,"parent_pid":parent_pid,"role":role,"executable":executable,"operation_id":operation_id})); }
        });
    }
    pub fn expect_images(&self, host: &Path, helper: &Path, operation_id: Uuid) {
        self.update(|state| {
            state.document["expected_images"] = json!({"isolated_csharp_host":host,"actual_worker":helper});
            state.document["worker_observation"] = json!({"status":"DriverDiscoveryRequired","role":"actual_worker","operation_id":operation_id,"parent_role":"isolated_csharp_host","parent_pid":null});
        });
    }
    pub fn host_parent(&self, pid: u32) {
        self.update(|state| state.document["worker_observation"]["parent_pid"] = json!(pid));
    }
    pub fn finish(&self, confirmed: bool, forced: bool) {
        self.update(|state| {
            if state.finished {
                return;
            }
            state.finished = true;
            state.document["state"] = json!(if forced {
                "Forced"
            } else if confirmed {
                "Cleaned"
            } else {
                "CleanupIncomplete"
            });
            state.document["cleanup"]["confirmed"] = json!(confirmed && !forced);
            state.document["cleanup"]["status"] = json!(if forced {
                "Unknown"
            } else if confirmed {
                "Confirmed"
            } else {
                "Incomplete"
            });
        });
    }
}
fn add_error(document: &mut Value, reason: &'static str, native_error: Option<i32>) {
    let errors = document["errors"].as_array_mut().expect("bounded errors");
    if errors.len() < 16
        && !errors
            .iter()
            .any(|row| row["reason"] == reason && row["native_error"] == json!(native_error))
    {
        errors.push(json!({"reason":reason,"native_error":native_error}));
    }
}
fn accept_stop(state: &mut State, reason: &'static str) {
    if !state.finished && state.document["stop"]["requested"] != true {
        state.stop_at = Some(Instant::now());
        state.document["stop"] = json!({"requested":true,"reason":reason,"dispatch_count":1});
        state.document["state"] = json!("StopRequested");
    }
}
fn fixed_reason(reason: &str) -> &'static str {
    match reason {
        "physical_memory_threshold" => "physical_memory_threshold",
        "commit_headroom_threshold" => "commit_headroom_threshold",
        "disk_threshold" => "disk_threshold",
        "sampler_deadline" => "sampler_deadline",
        "sampler_failure" => "sampler_failure",
        "driver_exception" => "driver_exception",
        "driver_deadline" => "driver_deadline",
        "driver_wait_deadline" => "driver_wait_deadline",
        "probe_owner_sample_unavailable" => "probe_owner_sample_unavailable",
        "injected_partial_threshold" => "injected_partial_threshold",
        "owner_exited_before_descendants" => "owner_exited_before_descendants",
        "physical_threshold" => "physical_threshold",
        "commit_threshold" => "commit_threshold",
        "invalid_stop" => "invalid_stop",
        _ => "driver_requested",
    }
}
pub fn read_bounded(path: &Path) -> io::Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || is_reparse(&metadata) {
        return Err(io::Error::other("control_not_regular"));
    }
    let file = File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::other("control_not_regular"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_CONTROL + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_CONTROL {
        return Err(io::Error::other("control_too_large"));
    }
    Ok(bytes)
}

/// Windows QueryFullProcessImageName reports DOS paths, not canonicalize's verbatim prefix.
pub fn exact_image(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{unc}"));
        }
        if let Some(dos) = text.strip_prefix(r"\\?\") {
            return PathBuf::from(dos);
        }
    }
    path
}

fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

/// Exactly one joinable monitor; bounded file reads, no blocked stdin-reader thread.
pub struct EngineMonitor {
    done: mpsc::Sender<()>,
    thread: Option<JoinHandle<()>>,
    status: Arc<ProbeStatus>,
}
impl EngineMonitor {
    pub fn start(status: Arc<ProbeStatus>, control: OperationControl) -> Self {
        let (done, receiver) = mpsc::channel();
        let observed = status.clone();
        let thread = std::thread::spawn(move || {
            loop {
                if observed.poll_stop() {
                    observed.phase("Cancelling");
                    control.cancel(); // The clone shares the ORIGINAL operation, including paused checkpoints.
                    observed.phase("Draining");
                    break;
                }
                match receiver.recv_timeout(POLL) {
                    Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    Err(mpsc::RecvTimeoutError::Timeout) => {}
                }
            }
        });
        Self {
            done,
            thread: Some(thread),
            status,
        }
    }
}
impl Drop for EngineMonitor {
    fn drop(&mut self) {
        let _ = self.done.send(());
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            self.status.error("monitor_join_failure", None);
            self.status.request_stop("monitor_failure");
        }
    }
}
