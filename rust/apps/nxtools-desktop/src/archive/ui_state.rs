//! Independent, path-free archive-browser presentation persistence.
//! The owner performs all file IO on one bounded writer thread. A corrupt or invalid
//! existing file is preserved and locks out writes until restart/explicit recovery;
//! defaults never overwrite evidence that failed to load.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, Write as _},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{SystemTime, UNIX_EPOCH},
};

const VERSION: u32 = 1;
const MAX_FILE_BYTES: u64 = 16 * 1024;
const MIN_WINDOW_WIDTH: i32 = 720;
const MIN_WINDOW_HEIGHT: i32 = 480;
const MAX_WINDOW_WIDTH: i32 = 16_384;
const MAX_WINDOW_HEIGHT: i32 = 16_384;
const MAX_ABS_ORIGIN: i32 = 1_000_000;
const MIN_COLUMN_WIDTH: f32 = 64.0;
const MAX_COLUMN_WIDTH: f32 = 640.0;

const DEFAULT_WIDTHS: [f32; 10] = [240., 112., 112., 112., 192., 112., 80., 112., 112., 280.];
const DEFAULT_VISIBLE: [bool; 10] = [
    true, true, true, true, true, false, false, false, false, false,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum UiSortKey {
    Name,
    Size,
    Packed,
    Type,
    Modified,
    Method,
    Encrypted,
    Crc,
    Attributes,
    Path,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct UiGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub maximized: bool,
}

impl Default for UiGeometry {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 960,
            height: 640,
            maximized: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct UiState {
    pub version: u32,
    pub geometry: UiGeometry,
    pub widths: [f32; 10],
    pub visible: [bool; 10],
    pub show_tree: bool,
    pub show_comment: bool,
    pub sort: UiSortKey,
    pub descending: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            version: VERSION,
            geometry: UiGeometry::default(),
            widths: DEFAULT_WIDTHS,
            visible: DEFAULT_VISIBLE,
            show_tree: true,
            show_comment: false,
            sort: UiSortKey::Name,
            descending: false,
        }
    }
}

impl UiState {
    pub(crate) fn validate(&self) -> Result<(), UiStateValidationError> {
        if self.version != VERSION {
            return Err(UiStateValidationError::Version);
        }
        if self.geometry.x.unsigned_abs() > MAX_ABS_ORIGIN as u32
            || self.geometry.y.unsigned_abs() > MAX_ABS_ORIGIN as u32
            || !(MIN_WINDOW_WIDTH..=MAX_WINDOW_WIDTH).contains(&self.geometry.width)
            || !(MIN_WINDOW_HEIGHT..=MAX_WINDOW_HEIGHT).contains(&self.geometry.height)
        {
            return Err(UiStateValidationError::Geometry);
        }
        if !self.visible[0] {
            return Err(UiStateValidationError::NameHidden);
        }
        if self.widths.iter().any(|width| {
            !width.is_finite() || !(MIN_COLUMN_WIDTH..=MAX_COLUMN_WIDTH).contains(width)
        }) {
            return Err(UiStateValidationError::ColumnWidth);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiStateValidationError {
    Version,
    Geometry,
    NameHidden,
    ColumnWidth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiStateLoadIssue {
    Corrupt,
    Invalid,
    TooLarge,
    Io,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UiStateLoad {
    Loading,
    Ready(UiState),
    Preserved(UiStateLoadIssue),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UiStateStoreError {
    Closed,
    Preserved,
    Invalid,
}

struct Shared {
    load: UiStateLoad,
    current: UiState,
    writable: bool,
    dirty: bool,
    closed: bool,
}

/// One nonblocking wake slot; the worker always reads the newest path-free snapshot.
pub(crate) struct UiStateStore {
    shared: Arc<Mutex<Shared>>,
    sender: SyncSender<()>,
    worker: Option<JoinHandle<()>>,
}

impl UiStateStore {
    // The production caller supplies its already discovered local data path before GPUI starts.

    pub(crate) fn initialize(path: PathBuf) -> io::Result<Self> {
        let shared = Arc::new(Mutex::new(Shared {
            load: UiStateLoad::Loading,
            current: UiState::default(),
            writable: false,
            dirty: false,
            closed: false,
        }));
        let (sender, receiver) = mpsc::sync_channel(1);
        let worker_path = path.clone();
        let worker_shared = shared.clone();
        let worker = thread::Builder::new()
            .name("archive-ui-state".into())
            .spawn(move || {
                let loaded = load(&worker_path);
                let mut writable = false;
                if let Ok(mut state) = worker_shared.lock() {
                    match loaded {
                        Ok(snapshot) => {
                            state.current = snapshot.clone();
                            state.load = UiStateLoad::Ready(snapshot);
                            state.writable = true;
                            writable = true;
                        }
                        Err(issue) => {
                            state.load = UiStateLoad::Preserved(issue);
                            state.writable = false;
                        }
                    }
                }
                while receiver.recv().is_ok() {
                    let (snapshot, dirty, closed) = match worker_shared.lock() {
                        Ok(mut state) => {
                            let value = (state.current.clone(), state.dirty, state.closed);
                            state.dirty = false;
                            value
                        }
                        Err(_) => break,
                    };
                    if writable && dirty {
                        if save(&worker_path, &snapshot).is_err() {
                            writable = false;
                            if let Ok(mut state) = worker_shared.lock() {
                                state.load = UiStateLoad::Preserved(UiStateLoadIssue::Io);
                                state.writable = false;
                            }
                        } else if let Ok(mut state) = worker_shared.lock() {
                            state.load = UiStateLoad::Ready(state.current.clone());
                        }
                    }
                    if closed {
                        break;
                    }
                }
            })?;
        Ok(Self {
            shared,
            sender,
            worker: Some(worker),
        })
    }

    pub(crate) fn load_status(&self) -> UiStateLoad {
        self.shared
            .lock()
            .map(|state| state.load.clone())
            .unwrap_or(UiStateLoad::Preserved(UiStateLoadIssue::Io))
    }

    pub(crate) fn current_snapshot(&self) -> UiState {
        self.shared
            .lock()
            .map(|state| state.current.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().current.clone())
    }

    /// Nonblocking ordinary save. A full queue coalesces into `current_snapshot`; Drop sends that
    /// exact latest value as the terminal writer request, giving close a lossless final state.
    pub(crate) fn enqueue_save(&self, snapshot: UiState) -> Result<(), UiStateStoreError> {
        snapshot
            .validate()
            .map_err(|_| UiStateStoreError::Invalid)?;
        let writable = self
            .shared
            .lock()
            .map(|mut state| {
                if state.writable {
                    state.current = snapshot.clone();
                    state.dirty = true;
                }
                state.writable
            })
            .map_err(|_| UiStateStoreError::Closed)?;
        if !writable {
            return Err(UiStateStoreError::Preserved);
        }
        match self.sender.try_send(()) {
            Ok(()) | Err(TrySendError::Full(_)) => Ok(()),
            Err(TrySendError::Disconnected(_)) => Err(UiStateStoreError::Closed),
        }
    }
    fn close(&self) {
        if let Ok(mut state) = self.shared.lock() {
            state.closed = true;
        }
        let _ = self.sender.try_send(());
    }
    #[cfg(test)]
    pub(crate) fn finish_for_test(mut self) {
        self.close();
        if let Some(worker) = self.worker.take() {
            worker.join().expect("archive UI state writer");
        }
    }
}

impl Drop for UiStateStore {
    fn drop(&mut self) {
        self.close();
        // JoinHandle drop detaches. The one writer flushes its final snapshot then exits.
        drop(self.worker.take());
    }
}

fn load(path: &Path) -> Result<UiState, UiStateLoadIssue> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(UiState::default()),
        Err(_) => return Err(UiStateLoadIssue::Io),
    };
    if !metadata.is_file() {
        return Err(UiStateLoadIssue::Io);
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err(UiStateLoadIssue::TooLarge);
    }
    use std::io::Read as _;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| UiStateLoadIssue::Io)?
        .take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| UiStateLoadIssue::Io)?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(UiStateLoadIssue::TooLarge);
    }
    let state: UiState = serde_json::from_slice(&bytes).map_err(|_| UiStateLoadIssue::Corrupt)?;
    state.validate().map_err(|_| UiStateLoadIssue::Invalid)?;
    Ok(state)
}
fn save(path: &Path, state: &UiState) -> io::Result<()> {
    load(path).map_err(|_| io::Error::other("existing archive UI state preserved"))?;
    state
        .validate()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid archive UI state"))?;
    let json = serde_json::to_vec_pretty(state).map_err(io::Error::other)?;
    if json.len() as u64 > MAX_FILE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "archive UI state exceeds its finite file budget",
        ));
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file_name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "archive-ui-state.json".into());
    let temporary = path.with_file_name(format!(
        ".{file_name}.{}.{:x}.tmp",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos())
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(&json)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}
