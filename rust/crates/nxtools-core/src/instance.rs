//! Process-instance lock lifetime and advisory window hints.
use std::fs::{File, OpenOptions};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use fs2::FileExt;

use crate::AppPaths;

pub struct SingleInstanceGuard {
    _file: File,
}

impl SingleInstanceGuard {
    pub fn acquire(paths: &AppPaths, process_name: &str) -> Result<Self> {
        Self::acquire_with_timeout(paths, process_name, Duration::ZERO)
    }

    /// Returns whether another process currently holds the instance lock.
    ///
    /// This is intentionally separate from reading the hint file: a crashed
    /// process can leave its last window handle behind after releasing the
    /// lock, and that handle must not trigger a stale IPC summon.
    pub fn is_locked(paths: &AppPaths, process_name: &str) -> bool {
        let path = paths.cache_dir.join(format!("{process_name}.lock"));
        let Ok(file) = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path)
        else {
            return true;
        };
        file.try_lock_exclusive().is_err()
    }

    /// Persists a fresh hint file for this instance. Callers update the
    /// window handle after the main window exists; a second launch reads it
    /// to summon the running instance instead of failing.
    pub fn capture_hints(paths: &AppPaths, process_name: &str) {
        let hint_path = paths.cache_dir.join(format!("{process_name}.hints"));
        let _ = std::fs::write(hint_path, std::process::id().to_string());
    }

    /// Reads the running instance's main-window handle recorded by the
    /// desktop at startup. Returns 0 when no valid hint exists.
    pub fn read_main_window_handle(paths: &AppPaths, process_name: &str) -> i64 {
        let hint_path = paths.cache_dir.join(format!("{process_name}.hints"));
        std::fs::read_to_string(hint_path)
            .ok()
            .and_then(|text| text.trim().parse::<i64>().ok())
            .unwrap_or(0)
    }

    /// Records the main window handle into the hint file.
    pub fn record_main_window_handle(paths: &AppPaths, process_name: &str, handle: i64) {
        let hint_path = paths.cache_dir.join(format!("{process_name}.hints"));
        let _ = std::fs::write(hint_path, handle.to_string());
    }

    pub fn acquire_with_timeout(
        paths: &AppPaths,
        process_name: &str,
        timeout: Duration,
    ) -> Result<Self> {
        let path = paths.cache_dir.join(format!("{process_name}.lock"));
        let deadline = Instant::now() + timeout;
        loop {
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&path)
                .with_context(|| format!("failed to open {}", path.display()))?;

            if file.try_lock_exclusive().is_ok() {
                return Ok(Self { _file: file });
            }
            if Instant::now() >= deadline {
                bail!("another {process_name} instance is already running");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
