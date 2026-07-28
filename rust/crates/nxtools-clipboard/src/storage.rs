use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{Connection, MAIN_DB, params};
use thiserror::Error;

use crate::ClipboardHistory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardDatabaseBackupReport {
    pub backup_path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardDatabaseCompactionReport {
    pub backup_path: PathBuf,
    pub before_bytes: u64,
    pub after_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardStorageMigrationReport {
    pub database_path: PathBuf,
    pub asset_root: PathBuf,
    pub item_count: usize,
    pub asset_count: usize,
}

#[derive(Debug, Error)]
pub enum ClipboardStorageError {
    #[error("clipboard database operation failed: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("clipboard storage I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid clipboard storage destination: {0}")]
    InvalidDestination(String),
}

pub(crate) fn create_database_backup(
    history: &ClipboardHistory,
    backup_directory: &Path,
) -> Result<ClipboardDatabaseBackupReport, ClipboardStorageError> {
    fs::create_dir_all(backup_directory)?;
    let suffix = unique_suffix();
    let backup_path = backup_directory.join(format!("clipboard-backup-{suffix}.db"));
    let temporary = backup_directory.join(format!(".clipboard-backup-{suffix}.tmp"));
    let result = history.connection.backup(MAIN_DB, &temporary, None);
    if let Err(error) = result {
        let _ = fs::remove_file(&temporary);
        return Err(error.into());
    }
    fs::rename(&temporary, &backup_path)?;
    let size_bytes = fs::metadata(&backup_path)?.len();
    Ok(ClipboardDatabaseBackupReport {
        backup_path,
        size_bytes,
    })
}

pub(crate) fn compact_database(
    history: &mut ClipboardHistory,
    backup_directory: &Path,
) -> Result<ClipboardDatabaseCompactionReport, ClipboardStorageError> {
    let before_bytes = fs::metadata(history.database_path())?.len();
    let backup = create_database_backup(history, backup_directory)?;
    history.connection.execute_batch("VACUUM;")?;
    let after_bytes = fs::metadata(history.database_path())?.len();
    Ok(ClipboardDatabaseCompactionReport {
        backup_path: backup.backup_path,
        before_bytes,
        after_bytes,
    })
}

pub(crate) fn migrate_storage(
    history: &ClipboardHistory,
    source_asset_root: &Path,
    destination_root: &Path,
) -> Result<ClipboardStorageMigrationReport, ClipboardStorageError> {
    validate_destination(history, destination_root)?;
    let parent = destination_root.parent().ok_or_else(|| {
        ClipboardStorageError::InvalidDestination("destination has no parent directory".into())
    })?;
    fs::create_dir_all(parent)?;
    let staging = parent.join(format!(
        ".nxtools-clipboard-migration-{}",
        uuid::Uuid::new_v4()
    ));
    let staged_database = staging.join("clipboard.db");
    let staged_assets = staging.join("clipboard-images");
    let final_assets = destination_root.join("clipboard-images");
    let result = (|| {
        fs::create_dir_all(&staged_assets)?;
        history.connection.backup(MAIN_DB, &staged_database, None)?;
        let asset_count = copy_asset_tree(source_asset_root, &staged_assets)?;
        let migrated = Connection::open(&staged_database)?;
        migrated.busy_timeout(std::time::Duration::from_secs(5))?;
        rewrite_managed_paths(
            &migrated,
            source_asset_root,
            &final_assets,
            "clipboard_items",
            "image_path",
        )?;
        rewrite_managed_paths(
            &migrated,
            source_asset_root,
            &final_assets,
            "clipboard_items",
            "source_process_icon_path",
        )?;
        rewrite_managed_paths(
            &migrated,
            source_asset_root,
            &final_assets,
            "clipboard_item_formats",
            "data_path",
        )?;
        let integrity: String = migrated.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if integrity != "ok" {
            return Err(ClipboardStorageError::InvalidDestination(format!(
                "migrated database integrity check failed: {integrity}"
            )));
        }
        let item_count = migrated.query_row("SELECT COUNT(*) FROM clipboard_items", [], |row| {
            row.get::<_, i64>(0)
        })?;
        let item_count = usize::try_from(item_count).map_err(|_| {
            ClipboardStorageError::InvalidDestination("invalid migrated item count".into())
        })?;
        drop(migrated);

        let destination_existed = destination_root.exists();
        if destination_existed {
            fs::remove_dir(destination_root)?;
        }
        if let Err(error) = fs::rename(&staging, destination_root) {
            if destination_existed {
                let _ = fs::create_dir(destination_root);
            }
            return Err(error.into());
        }
        Ok(ClipboardStorageMigrationReport {
            database_path: destination_root.join("clipboard.db"),
            asset_root: final_assets,
            item_count,
            asset_count,
        })
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn validate_destination(
    history: &ClipboardHistory,
    destination_root: &Path,
) -> Result<(), ClipboardStorageError> {
    if destination_root.as_os_str().is_empty() {
        return Err(ClipboardStorageError::InvalidDestination(
            "destination directory is empty".into(),
        ));
    }
    if let Some(source_root) = history.database_path().parent()
        && paths_equivalent(source_root, destination_root)
    {
        return Err(ClipboardStorageError::InvalidDestination(
            "destination is already the active storage directory".into(),
        ));
    }
    if destination_root.exists() {
        if !destination_root.is_dir() {
            return Err(ClipboardStorageError::InvalidDestination(
                "destination is not a directory".into(),
            ));
        }
        if fs::read_dir(destination_root)?.next().is_some() {
            return Err(ClipboardStorageError::InvalidDestination(
                "destination directory must be empty".into(),
            ));
        }
    }
    Ok(())
}

fn paths_equivalent(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_owned());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_owned());
    if cfg!(windows) {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    } else {
        left == right
    }
}

fn copy_asset_tree(source: &Path, destination: &Path) -> Result<usize, ClipboardStorageError> {
    if !source.exists() {
        return Ok(0);
    }
    let mut copied = 0;
    copy_asset_directory(source, destination, &mut copied)?;
    Ok(copied)
}

fn copy_asset_directory(
    source: &Path,
    destination: &Path,
    copied: &mut usize,
) -> Result<(), ClipboardStorageError> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = destination.join(entry.file_name());
        if file_type.is_symlink() {
            return Err(ClipboardStorageError::InvalidDestination(format!(
                "managed asset tree contains a symbolic link: {}",
                entry.path().display()
            )));
        }
        if file_type.is_dir() {
            copy_asset_directory(&entry.path(), &target, copied)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), target)?;
            *copied += 1;
        }
    }
    Ok(())
}

fn rewrite_managed_paths(
    connection: &Connection,
    source_root: &Path,
    destination_root: &Path,
    table: &str,
    column: &str,
) -> Result<(), ClipboardStorageError> {
    let select = format!("SELECT rowid, {column} FROM {table} WHERE {column} != ''");
    let mut statement = connection.prepare(&select)?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    drop(statement);
    let update = format!("UPDATE {table} SET {column} = ?1 WHERE rowid = ?2");
    for (row_id, value) in rows {
        let path = Path::new(&value);
        let Ok(relative) = path.strip_prefix(source_root) else {
            continue;
        };
        let rewritten = destination_root.join(relative);
        connection.execute(&update, params![rewritten.to_string_lossy(), row_id])?;
    }
    Ok(())
}

fn unique_suffix() -> String {
    let unix_millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{unix_millis}-{}", uuid::Uuid::new_v4())
}
