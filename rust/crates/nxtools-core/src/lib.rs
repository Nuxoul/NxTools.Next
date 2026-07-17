use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, bail};
use directories::ProjectDirs;
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub config_dir: PathBuf,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub log_dir: PathBuf,
}

impl AppPaths {
    pub fn discover() -> Result<Self> {
        let dirs = ProjectDirs::from("dev", "NxTools", "NxTools.Next")
            .context("Windows did not provide application data directories")?;
        let paths = Self {
            config_dir: dirs.config_dir().to_path_buf(),
            data_dir: dirs.data_dir().to_path_buf(),
            cache_dir: dirs.cache_dir().to_path_buf(),
            log_dir: dirs.data_local_dir().join("logs"),
        };
        paths.ensure_created()?;
        Ok(paths)
    }

    fn ensure_created(&self) -> Result<()> {
        for path in [
            &self.config_dir,
            &self.data_dir,
            &self.cache_dir,
            &self.log_dir,
        ] {
            fs::create_dir_all(path)
                .with_context(|| format!("failed to create {}", path.display()))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub theme: ThemePreference,
    pub platform_pipe_name: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: ThemePreference::Dark,
            platform_pipe_name: "nxtools-next-platform-v1".into(),
        }
    }
}

impl AppConfig {
    pub fn load_or_create(paths: &AppPaths) -> Result<Self> {
        let path = paths.config_dir.join("config.json");
        match fs::read_to_string(&path) {
            Ok(json) => serde_json::from_str(&json)
                .with_context(|| format!("failed to parse {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let config = Self::default();
                fs::write(&path, serde_json::to_string_pretty(&config)?)
                    .with_context(|| format!("failed to write {}", path.display()))?;
                Ok(config)
            }
            Err(error) => Err(error).with_context(|| format!("failed to read {}", path.display())),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ThemePreference {
    Dark,
    Light,
    System,
}

pub struct SingleInstanceGuard {
    _file: File,
}

impl SingleInstanceGuard {
    pub fn acquire(paths: &AppPaths, process_name: &str) -> Result<Self> {
        let path = paths.cache_dir.join(format!("{process_name}.lock"));
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;

        if file.try_lock_exclusive().is_err() {
            bail!("another {process_name} instance is already running");
        }

        Ok(Self { _file: file })
    }
}

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

fn delete_expired_logs(log_dir: &Path, process_name: &str, retention: Duration) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_is_stable_and_serializable() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).expect("serialize");
        let decoded = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(config, decoded);
    }

    #[test]
    fn log_retention_only_deletes_matching_process_logs() {
        let directory = std::env::temp_dir().join(format!(
            "nxtools-log-retention-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        fs::create_dir_all(&directory).expect("create test directory");
        let desktop_log = directory.join("desktop.log.2000-01-01");
        let other_log = directory.join("indexer.log.2000-01-01");
        fs::write(&desktop_log, b"old").expect("write desktop log");
        fs::write(&other_log, b"old").expect("write other log");

        delete_expired_logs(&directory, "desktop", Duration::ZERO);

        assert!(!desktop_log.exists());
        assert!(other_log.exists());
        fs::remove_dir_all(directory).expect("remove test directory");
    }
}
