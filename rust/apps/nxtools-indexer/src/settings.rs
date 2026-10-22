use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use nxtools_protocol::IndexSettings;

pub(crate) fn load_system_index_settings() -> Result<IndexSettings> {
    load_index_settings(&system_index_settings_path())
}

fn load_index_settings(path: &std::path::Path) -> Result<IndexSettings> {
    match read_index_settings(path) {
        Ok(settings) => Ok(settings),
        Err(primary_error) => {
            let backup = backup_path(path);
            match read_index_settings(&backup) {
                Ok(settings) => Ok(settings),
                Err(_backup_error) if !path.exists() && !backup.exists() => {
                    Ok(IndexSettings::default())
                }
                Err(backup_error) => Err(primary_error).with_context(|| {
                    format!(
                        "last-good index settings {} also failed: {backup_error}",
                        backup.display()
                    )
                }),
            }
        }
    }
}

fn read_index_settings(path: &std::path::Path) -> Result<IndexSettings> {
    let bytes =
        fs::read(path).with_context(|| format!("read index settings {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("parse index settings {}", path.display()))
}

pub(crate) fn persist_system_index_settings(settings: &IndexSettings) -> Result<()> {
    persist_index_settings(&system_index_settings_path(), settings)
}

pub(super) fn persist_index_settings(
    path: &std::path::Path,
    settings: &IndexSettings,
) -> Result<()> {
    let directory = path.parent().context("index settings path has no parent")?;
    fs::create_dir_all(directory)
        .with_context(|| format!("create index settings directory {}", directory.display()))?;
    let temporary = temporary_path(path);
    let backup = backup_path(path);
    let bytes = serde_json::to_vec_pretty(settings)?;
    let mut file = File::create(&temporary)
        .with_context(|| format!("create index settings {}", temporary.display()))?;
    file.write_all(&bytes)
        .with_context(|| format!("write index settings {}", temporary.display()))?;
    file.sync_all()
        .with_context(|| format!("flush index settings {}", temporary.display()))?;
    drop(file);

    if path.exists() && read_index_settings(path).is_ok() {
        if backup.exists() {
            fs::remove_file(&backup)
                .with_context(|| format!("remove old settings backup {}", backup.display()))?;
        }
        fs::rename(path, &backup).with_context(|| {
            format!(
                "move index settings {} to {}",
                path.display(),
                backup.display()
            )
        })?;
    } else if path.exists() {
        fs::remove_file(path)
            .with_context(|| format!("remove corrupt index settings {}", path.display()))?;
    }
    if let Err(error) = fs::rename(&temporary, path) {
        if backup.exists() && !path.exists() {
            let _ = fs::copy(&backup, path);
        }
        return Err(error).with_context(|| {
            format!(
                "publish index settings {} from {}",
                path.display(),
                temporary.display()
            )
        });
    }
    Ok(())
}

fn temporary_path(path: &std::path::Path) -> PathBuf {
    path.with_extension("json.tmp")
}

fn backup_path(path: &std::path::Path) -> PathBuf {
    path.with_extension("json.last-good")
}

fn system_index_settings_path() -> PathBuf {
    let root = std::env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    root.join("NxTools")
        .join("NxTools.Next")
        .join("indexer")
        .join("settings.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_index_settings_survive_a_restart_round_trip() {
        let directory = tempfile::tempdir().expect("temporary settings directory");
        let path = directory.path().join("settings.json");
        let settings = IndexSettings {
            indexed_volumes: vec![r"D:\".into()],
            excluded_paths: vec![r"D:\Build".into()],
        };

        persist_index_settings(&path, &settings).expect("persist index settings");

        assert_eq!(
            load_index_settings(&path).expect("restore index settings"),
            settings
        );
    }

    #[test]
    fn a_truncated_primary_setting_recovers_the_last_good_scope() {
        let directory = tempfile::tempdir().expect("temporary settings directory");
        let path = directory.path().join("settings.json");
        let first = IndexSettings {
            indexed_volumes: vec![r"D:\".into()],
            excluded_paths: vec![r"D:\Private".into()],
        };
        let second = IndexSettings {
            indexed_volumes: vec![r"C:\".into()],
            excluded_paths: Vec::new(),
        };
        persist_index_settings(&path, &first).expect("persist first settings");
        persist_index_settings(&path, &second).expect("persist second settings");
        fs::write(&path, b"{").expect("truncate primary settings");

        assert_eq!(
            load_index_settings(&path).expect("recover last-good settings"),
            first
        );
    }

    #[test]
    fn saving_after_backup_recovery_never_promotes_the_corrupt_primary() {
        let directory = tempfile::tempdir().expect("temporary settings directory");
        let path = directory.path().join("settings.json");
        let last_good = IndexSettings {
            indexed_volumes: vec![r"D:\".into()],
            excluded_paths: vec![r"D:\Private".into()],
        };
        let damaged_generation = IndexSettings {
            indexed_volumes: vec![r"C:\".into()],
            excluded_paths: Vec::new(),
        };
        let replacement = IndexSettings {
            indexed_volumes: vec![r"E:\".into()],
            excluded_paths: vec![r"E:\Cache".into()],
        };
        persist_index_settings(&path, &last_good).expect("persist last good settings");
        persist_index_settings(&path, &damaged_generation).expect("persist next settings");
        fs::write(&path, b"{").expect("damage primary settings");

        persist_index_settings(&path, &replacement).expect("replace damaged primary");
        fs::write(&path, b"{").expect("damage replacement primary");

        assert_eq!(
            load_index_settings(&path).expect("last good remains recoverable"),
            last_good
        );
    }
}
