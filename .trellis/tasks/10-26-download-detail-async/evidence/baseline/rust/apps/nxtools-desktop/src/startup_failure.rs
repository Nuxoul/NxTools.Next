//! Bootstrap-level startup failure reporting (fix for
//! ERR-20260901-PACKAGED-SILENT-STARTUP-FAILURE).
//!
//! The Release desktop is a GUI-subsystem binary: `eprintln!` at the
//! top-level error path is invisible, so schema-guard and other startup
//! rejections looked like "double-click does nothing". This module appends a
//! persistent report into the log directory and, for interactive launch
//! modes, asks the sibling PlatformHost to surface it as a native dialog
//! (`--error-dialog`); the dialog message is read from the report file C#-side.
//!
//! Boundary note: this is bootstrap fatal-error reporting that runs before any
//! product surface exists. It does not move product Windows adaptation
//! (clipboard, shell, hotkeys, toggles) out of the C# PlatformHost.

use anyhow::Error;
use std::path::{Path, PathBuf};

pub(crate) const STARTUP_FAILURE_LOG: &str = "startup-failure.log";

pub(crate) fn write_report(log_dir: &Path, error: &Error) -> anyhow::Result<PathBuf> {
    let path = log_dir.join(STARTUP_FAILURE_LOG);
    let entry = format!(
        "[{}] NxTools could not start:\n{error:#}\n\n",
        chrono::Local::now().to_rfc3339(),
    );
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    file.write_all(entry.as_bytes())?;
    file.flush()?;
    Ok(path)
}

/// Asks the sibling PlatformHost to show one native error dialog presenting
/// the report file (EX-10 boundary: Windows dialog ownership stays in C#).
/// Best effort: spawn failures leave the persistent report as the only surface.
fn spawn_error_dialog(platform_host: &Path, report_path: &Path) -> anyhow::Result<()> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new(platform_host)
        .arg("--error-dialog")
        .arg(report_path)
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
    Ok(())
}

/// Reports a fatal startup failure: appends the persistent report when the log
/// directory is known and, for interactive launches, asks the sibling
/// PlatformHost to show one native error dialog.
pub(crate) fn report(
    log_dir: Option<&Path>,
    platform_host: Option<&Path>,
    error: &Error,
    interactive: bool,
) {
    eprintln!("NxTools could not start: {error:#}");
    let report_path = log_dir.and_then(|dir| write_report(dir, error).ok());
    if let (true, Some(host), Some(path)) = (interactive, platform_host, report_path.as_deref()) {
        let _ = spawn_error_dialog(host, path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "nxtools-startup-failure-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("temp log dir");
        dir
    }

    #[test]
    fn write_report_appends_one_entry_per_failure() {
        let dir = temp_dir("append");
        let error = anyhow::anyhow!("desktop startup failed")
            .context("failed to initialize the download engine");
        let first = write_report(&dir, &error).expect("first report");
        let second = write_report(&dir, &error).expect("second report");
        assert_eq!(first, second);
        let contents = fs::read_to_string(&first).expect("report contents");
        assert_eq!(
            contents.match_indices("NxTools could not start").count(),
            2,
            "each failure appends one entry"
        );
        assert!(contents.contains("failed to initialize the download engine"));
        assert!(contents.contains("desktop startup failed"));
        let _ = fs::remove_dir_all(&dir);
    }
}
