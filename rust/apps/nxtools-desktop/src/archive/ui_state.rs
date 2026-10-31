//! Independent archive-browser presentation and bounded local recent-path persistence.
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

pub(crate) const VERSION: u32 = 2;
const MAX_FILE_BYTES: u64 = 1024 * 1024;
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
    #[serde(default = "default_details_view")]
    pub details_view: bool,
    #[serde(default)]
    pub recent: Vec<PathBuf>,
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
            details_view: default_details_view(),
            recent: Vec::new(),
        }
    }
}

fn default_details_view() -> bool {
    true
}

impl UiState {
    pub(crate) fn validate(&self) -> Result<(), UiStateValidationError> {
        if self.version != VERSION {
            return Err(UiStateValidationError::Version);
        }
        if self.recent.len() > 30
            || self
                .recent
                .iter()
                .map(|path| path.to_string_lossy().len())
                .sum::<usize>()
                > 128 * 1024
            || self.recent.iter().enumerate().any(|(index, path)| {
                !path.is_absolute()
                    || path.as_os_str().len() > 32_768
                    || path.to_string_lossy().contains('\0')
                    || self.recent[..index].iter().any(|old| {
                        old.to_string_lossy()
                            .eq_ignore_ascii_case(&path.to_string_lossy())
                    })
            })
        {
            return Err(UiStateValidationError::Recent);
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
    Recent,
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
    flush_waiters: Vec<async_channel::Sender<bool>>,
}

/// One nonblocking wake slot; the worker always reads the newest owned snapshot.
pub(crate) struct UiStateStore {
    shared: Arc<Mutex<Shared>>,
    sender: SyncSender<()>,
    _changed: async_channel::Sender<()>,
    changes: async_channel::Receiver<()>,
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
            flush_waiters: Vec::new(),
        }));
        let (changed, changes) = async_channel::bounded(1);
        let worker_changed = changed.clone();
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
                let _ = worker_changed.try_send(());
                while receiver.recv().is_ok() {
                    let (snapshot, dirty, closed, waiters) = match worker_shared.lock() {
                        Ok(mut state) => {
                            let value = (
                                state.current.clone(),
                                state.dirty,
                                state.closed,
                                std::mem::take(&mut state.flush_waiters),
                            );
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
                    for waiter in waiters {
                        let _ = waiter.try_send(writable);
                    }
                    let _ = worker_changed.try_send(());
                    if closed {
                        break;
                    }
                }
            })?;
        Ok(Self {
            shared,
            sender,
            worker: Some(worker),
            _changed: changed,
            changes,
        })
    }
    pub(crate) fn changes(&self) -> async_channel::Receiver<()> {
        self.changes.clone()
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
                    let mut snapshot = snapshot.clone();
                    // Browser close/resize snapshots do not own history. Never resurrect cleared data.
                    snapshot.recent = state.current.recent.clone();
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
    pub(crate) fn update_recent(
        &self,
        mutate: impl FnOnce(&mut Vec<PathBuf>),
    ) -> Result<(), UiStateStoreError> {
        {
            let mut state = self.shared.lock().map_err(|_| UiStateStoreError::Closed)?;
            if !state.writable {
                return Err(UiStateStoreError::Preserved);
            }
            let mut next = state.current.clone();
            mutate(&mut next.recent);
            next.validate().map_err(|_| UiStateStoreError::Invalid)?;
            state.current = next;
            state.dirty = true;
        }
        match self.sender.try_send(()) {
            Ok(()) | Err(TrySendError::Full(_)) => Ok(()),
            Err(TrySendError::Disconnected(_)) => Err(UiStateStoreError::Closed),
        }
    }
    pub(crate) fn flush(&self) -> Result<async_channel::Receiver<bool>, UiStateStoreError> {
        let (sender, receiver) = async_channel::bounded(1);
        {
            let mut state = self.shared.lock().map_err(|_| UiStateStoreError::Closed)?;
            if !state.writable || state.flush_waiters.len() >= 4 {
                return Err(UiStateStoreError::Preserved);
            }
            state.flush_waiters.push(sender);
            // The latest snapshot may be in flight; another owned write makes this receipt durable.
            state.dirty = true;
        }
        match self.sender.try_send(()) {
            Ok(()) | Err(TrySendError::Full(_)) => Ok(receiver),
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
    let mut state: UiState =
        serde_json::from_slice(&bytes).map_err(|_| UiStateLoadIssue::Corrupt)?;
    let mut legacy_name_mode = false;
    if state.version == 1 {
        // Keep typed deserialization (including duplicate/null-field rejection). v1's
        // absent mode meant name-list, unlike the current additive Details default.
        #[derive(Deserialize)]
        struct LegacyMode {
            #[serde(default)]
            details_view: bool,
        }
        let legacy: LegacyMode =
            serde_json::from_slice(&bytes).map_err(|_| UiStateLoadIssue::Corrupt)?;
        legacy_name_mode = !legacy.details_view;
        state.version = VERSION;
    }
    // Validate the unmodified preferences first: migration must not repair invalid evidence.
    state.validate().map_err(|_| UiStateLoadIssue::Invalid)?;
    if legacy_name_mode {
        state.details_view = true;
        state.visible[..5].fill(true);
    }
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

#[cfg(test)]
#[path = "ui_state_migration_tests.rs"]
mod migration_tests;
