//! Nonblocking process log initialization and process-scoped retention.
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use tracing_subscriber::EnvFilter;

use crate::AppPaths;

pub type LogGuard = tracing_appender::non_blocking::WorkerGuard;

pub fn init_file_logging(paths: &AppPaths, process_name: &str) -> Result<LogGuard> {
    delete_expired_logs(
        &paths.log_dir,
        process_name,
        Duration::from_secs(14 * 24 * 60 * 60),
    );
    let appender = tracing_appender::rolling::daily(&paths.log_dir, format!("{process_name}.log"));
    let (writer, guard) = tracing_appender::non_blocking(appender);
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_ansi(false)
        .with_writer(writer)
        .try_init()
        .map_err(|error| anyhow::anyhow!("failed to initialize logging: {error}"))?;
    Ok(guard)
}

pub(super) fn delete_expired_logs(log_dir: &Path, process_name: &str, retention: Duration) {
    let prefix = format!("{process_name}.log.");
    let Some(cutoff) = SystemTime::now().checked_sub(retention) else {
        return;
    };
    let Ok(entries) = fs::read_dir(log_dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let is_process_log = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(&prefix));
        if !is_process_log {
            continue;
        }
        let is_expired = entry
            .metadata()
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| modified < cutoff);
        if is_expired {
            let _ = fs::remove_file(path);
        }
    }
}
