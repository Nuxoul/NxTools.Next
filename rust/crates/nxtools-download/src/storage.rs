use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};
use std::time::Duration;

use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params, params_from_iter};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use thiserror::Error;
use tracing::warn;
use uuid::Uuid;

use crate::dash::{DashManifest, DashTrackKind};
use crate::hls::{HlsMediaPlaylist, HlsMediaSegment};
use crate::{
    BrowserHandoffState, DownloadFault, DownloadProbe, DownloadQueue, DownloadRequest,
    DownloadSchedule, DownloadSegment, DownloadSegmentState, DownloadStatus, DownloadTask,
    DownloadTaskId, DownloadTaskTimelineEntry, DownloadTaskTimelineEvent,
    INTERRUPTED_DOWNLOAD_STATUSES, RequestOrigin, ResourceValidator, TransferPlan,
};

pub const DOWNLOAD_SCHEMA_VERSION: u32 = 15;
const GLOBAL_SPEED_LIMIT_SETTING: &str = "global_speed_limit_bytes_per_second";
const BROWSER_HANDOFF_RETENTION_MS: i64 = 7 * 24 * 60 * 60 * 1_000;
const MAX_BROWSER_HANDOFFS: i64 = 4_096;
pub(crate) const CONTROLLED_BROWSER_HANDOFF_LEASE_MS: i64 = 30_000;
pub(crate) fn request_fingerprint(
    request: &DownloadRequest,
) -> Result<String, DownloadStorageError> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(request)?)
    ))
}

// Keep the existing request_json row and transaction owner. Legacy readers ignore this
// optional field; new readers restore provenance without a destructive schema migration.
#[derive(serde::Serialize, serde::Deserialize)]
struct StoredResolvedRequest {
    #[serde(flatten)]
    request: DownloadRequest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    resolution: Option<crate::DownloadResolutionProvenance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    probe: Option<DownloadProbe>,
    #[serde(
        default,
        skip_serializing_if = "crate::DownloadCompletionIntention::is_empty"
    )]
    completion_intention: crate::DownloadCompletionIntention,
    #[serde(default, skip_serializing_if = "crate::is_zero")]
    completion_attempt: u64,
}

fn sqlite_extended_code(error: &DownloadStorageError) -> Option<i32> {
    match error {
        DownloadStorageError::Sqlite(rusqlite::Error::SqliteFailure(native, _)) => {
            Some(native.extended_code)
        }
        _ => None,
    }
}

fn should_retry_open_error(error: &DownloadStorageError) -> bool {
    let DownloadStorageError::Sqlite(rusqlite::Error::SqliteFailure(native, _)) = error else {
        return false;
    };
    matches!(
        native.code,
        rusqlite::ffi::ErrorCode::DatabaseBusy
            | rusqlite::ffi::ErrorCode::DatabaseLocked
            | rusqlite::ffi::ErrorCode::SystemIoFailure
    ) && (native.extended_code == rusqlite::ffi::SQLITE_IOERR_TRUNCATE
        || matches!(
            native.code,
            rusqlite::ffi::ErrorCode::DatabaseBusy | rusqlite::ffi::ErrorCode::DatabaseLocked
        ))
}

fn database_sidecar(path: &Path, suffix: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("downloads.sqlite3");
    path.with_file_name(format!("{file_name}{suffix}"))
}

fn quarantine_empty_database_file(path: &Path) -> Result<(), DownloadStorageError> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if metadata.len() != 0
        || database_sidecar(path, "-wal").exists()
        || database_sidecar(path, "-shm").exists()
    {
        return Ok(());
    }
    let quarantine = path.with_file_name(format!(
        "{}.empty-{}.bak",
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("downloads.sqlite3"),
        Uuid::new_v4()
    ));
    std::fs::rename(path, quarantine)?;
    warn!(
        event = "download.storage.empty_database_quarantined",
        "quarantined an empty download database before recreating it"
    );
    Ok(())
}

pub struct SqliteDownloadStore {
    connection: Mutex<Connection>,
}

pub(crate) struct QueueAdmission<'a> {
    pub(crate) id: &'a str,
    pub(crate) eligible: bool,
}

impl SqliteDownloadStore {
    /// Runs a fixture operation while holding the actual SQLite connection mutex.
    #[cfg(feature = "test-support")]
    pub(crate) fn with_connection_lock_for_test<R>(&self, operation: impl FnOnce() -> R) -> R {
        let _connection = self
            .connection
            .lock()
            .expect("synthetic store connection lock");
        operation()
    }

    pub fn open(path: impl AsRef<Path>) -> Result<Self, DownloadStorageError> {
        let path = path.as_ref();
        quarantine_empty_database_file(path)?;
        let mut last_error = None;
        for attempt in 0..=3 {
            match Self::open_once(path) {
                Ok(store) => return Ok(store),
                Err(error) if should_retry_open_error(&error) && attempt < 3 => {
                    let extended_code = sqlite_extended_code(&error);
                    warn!(
                        event = "download.storage.open_retry",
                        attempt = attempt + 1,
                        sqlite_extended_code = ?extended_code,
                        "retrying download database open after a transient SQLite I/O boundary"
                    );
                    last_error = Some(error);
                    std::thread::sleep(Duration::from_millis(100 * (attempt + 1) as u64));
                }
                Err(error) => return Err(error),
            }
        }
        Err(last_error.expect("download database retry must retain its last error"))
    }

    fn open_once(path: &Path) -> Result<Self, DownloadStorageError> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        reject_newer_schema(&connection)?;
        let journal_mode: String =
            connection.query_row("PRAGMA journal_mode", [], |row| row.get(0))?;
        if !journal_mode.eq_ignore_ascii_case("wal") {
            connection.pragma_update(None, "journal_mode", "WAL")?;
        }
        Self::initialize(connection)
    }

    pub fn open_in_memory() -> Result<Self, DownloadStorageError> {
        Self::initialize(Connection::open_in_memory()?)
    }

    fn initialize(connection: Connection) -> Result<Self, DownloadStorageError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        let current: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if current > DOWNLOAD_SCHEMA_VERSION {
            return Err(DownloadStorageError::UnsupportedSchema {
                current,
                supported: DOWNLOAD_SCHEMA_VERSION,
            });
        }

        if current == 0 {
            connection.execute_batch(SCHEMA_V1)?;
        }
        if current < 2 {
            connection.execute_batch(SCHEMA_V2)?;
        }
        if current < 3 {
            connection.execute_batch(SCHEMA_V3)?;
        }
        if current < 4 {
            connection.execute_batch(SCHEMA_V4)?;
        }
        if current < 5 {
            connection.execute_batch(SCHEMA_V5)?;
        }
        if current < 6 {
            connection.execute_batch(SCHEMA_V6)?;
        }
        if current < 7 {
            connection.execute_batch(SCHEMA_V7)?;
        }
        if current < 8 {
            connection.execute_batch(SCHEMA_V8)?;
        }
        if current < 9 {
            connection.execute_batch(SCHEMA_V9)?;
        }
        if current < 10 {
            connection.execute_batch(SCHEMA_V10)?;
        }

        if current < 11 {
            connection.execute_batch(SCHEMA_V11)?;
        }
        if current < 12 {
            connection.execute_batch(SCHEMA_V12)?;
        }
        if current < 13 {
            connection.execute_batch(SCHEMA_V13)?;
        }
        if current < 14 {
            connection.execute_batch(SCHEMA_V14)?;
        }
        if current < 15 {
            connection.execute_batch(SCHEMA_V15)?;
        }

        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub fn schema_version(&self) -> Result<u32, DownloadStorageError> {
        let connection = self.lock()?;
        Ok(connection.query_row("PRAGMA user_version", [], |row| row.get(0))?)
    }

    pub fn save_global_speed_limit(
        &self,
        bytes_per_second: Option<u64>,
    ) -> Result<(), DownloadStorageError> {
        let value_json = serde_json::to_string(&bytes_per_second)?;
        let connection = self.lock()?;
        connection.execute(
            r#"
            INSERT INTO download_settings(key, value_json, updated_at_unix_ms)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at_unix_ms = excluded.updated_at_unix_ms
            "#,
            params![GLOBAL_SPEED_LIMIT_SETTING, value_json, now_unix_ms()],
        )?;
        Ok(())
    }

    pub fn load_global_speed_limit(&self) -> Result<Option<u64>, DownloadStorageError> {
        let connection = self.lock()?;
        let stored = connection
            .query_row(
                "SELECT value_json FROM download_settings WHERE key = ?1",
                [GLOBAL_SPEED_LIMIT_SETTING],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        stored
            .map(|value| serde_json::from_str::<Option<u64>>(&value))
            .transpose()
            .map(Option::flatten)
            .map_err(Into::into)
    }

    pub fn save_task(&self, task: &DownloadTask) -> Result<(), DownloadStorageError> {
        let connection = self.lock()?;
        save_task_row(&connection, task)
    }

    pub fn load_task(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<Option<DownloadTask>, DownloadStorageError> {
        let connection = self.lock()?;
        connection
            .query_row(
                r#"
                SELECT
                    task_id,
                    request_json,
                    origin,
                    status,
                    created_at_unix_ms,
                    updated_at_unix_ms,
                    completed_at_unix_ms,
                    measured_payload_bytes,
                    active_download_ms
                FROM download_tasks
                WHERE task_id = ?1
                "#,
                [task_id.as_uuid().to_string()],
                decode_task_row,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn load_tasks(&self) -> Result<Vec<DownloadTask>, DownloadStorageError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            r#"
            SELECT
                task_id,
                request_json,
                origin,
                status,
                created_at_unix_ms,
                updated_at_unix_ms,
                completed_at_unix_ms,
                measured_payload_bytes,
                active_download_ms
            FROM download_tasks
            ORDER BY created_at_unix_ms DESC
            "#,
        )?;
        statement
            .query_map([], decode_task_row)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn replace_queue_order(
        &self,
        queue_id: &str,
        ordered_task_ids: &[DownloadTaskId],
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            r#"
            INSERT INTO download_queues(queue_id, name, position, max_concurrent_tasks, is_running)
            VALUES (?1, ?2, 0, 3, 1)
            ON CONFLICT(queue_id) DO NOTHING
            "#,
            params![queue_id, queue_id],
        )?;
        let admission = {
            let mut statement = transaction.prepare(
                r#"
                SELECT task_id, is_eligible, is_user_paused
                FROM download_queue_items
                WHERE queue_id = ?1
                "#,
            )?;
            statement
                .query_map([queue_id], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        (row.get::<_, bool>(1)?, row.get::<_, bool>(2)?),
                    ))
                })?
                .collect::<Result<std::collections::HashMap<_, _>, _>>()?
        };
        let existing = {
            let mut statement = transaction.prepare(
                "SELECT task_id FROM download_queue_items WHERE queue_id = ?1 ORDER BY position",
            )?;
            statement
                .query_map([queue_id], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut complete_order = ordered_task_ids.to_vec();
        let requested = ordered_task_ids
            .iter()
            .map(|task_id| task_id.as_uuid().to_string())
            .collect::<std::collections::HashSet<_>>();
        complete_order.extend(
            existing
                .into_iter()
                .filter(|task_id| !requested.contains(task_id))
                .map(|task_id| {
                    Uuid::parse_str(&task_id)
                        .map(DownloadTaskId::from_uuid)
                        .map_err(|error| DownloadStorageError::InvalidTaskId(error.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
        transaction.execute(
            "DELETE FROM download_queue_items WHERE queue_id = ?1",
            [queue_id],
        )?;
        for (position, task_id) in complete_order.iter().enumerate() {
            transaction.execute(
                r#"
                INSERT INTO download_queue_items(
                    queue_id, task_id, position, is_eligible, is_user_paused
                )
                VALUES (?1, ?2, ?3, ?4, ?5)
                "#,
                params![
                    queue_id,
                    task_id.as_uuid().to_string(),
                    i64::try_from(position).map_err(|_| DownloadStorageError::IntegerOverflow)?,
                    admission
                        .get(&task_id.as_uuid().to_string())
                        .is_some_and(|(is_eligible, _)| *is_eligible),
                    admission
                        .get(&task_id.as_uuid().to_string())
                        .is_some_and(|(_, is_user_paused)| *is_user_paused)
                ],
            )?;
            transaction.execute(
                "UPDATE download_tasks SET queue_id = ?2 WHERE task_id = ?1",
                params![task_id.as_uuid().to_string(), queue_id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Move existing memberships atomically without resetting admission or user pause.
    pub fn assign_queue_tasks(
        &self,
        queue_id: &str,
        task_ids: &[DownloadTaskId],
    ) -> Result<usize, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        transaction.execute("INSERT INTO download_queues(queue_id, name, position, max_concurrent_tasks, is_running) VALUES (?1, ?1, 0, 3, 1) ON CONFLICT(queue_id) DO NOTHING", [queue_id])?;
        let mut moved = 0;
        for task_id in task_ids {
            let id = task_id.as_uuid().to_string();
            let source: String = transaction
                .query_row(
                    "SELECT queue_id FROM download_queue_items WHERE task_id = ?1",
                    [&id],
                    |row| row.get(0),
                )
                .optional()?
                .ok_or(DownloadStorageError::TaskNotFound(*task_id))?;
            if source == queue_id {
                continue;
            }
            let position: i64 = transaction.query_row("SELECT COALESCE(MAX(position) + 1, 0) FROM download_queue_items WHERE queue_id = ?1", [queue_id], |row| row.get(0))?;
            transaction.execute(
                "UPDATE download_queue_items SET queue_id = ?2, position = ?3 WHERE task_id = ?1",
                params![id, queue_id, position],
            )?;
            transaction.execute(
                "UPDATE download_tasks SET queue_id = ?2 WHERE task_id = ?1",
                params![id, queue_id],
            )?;
            moved += 1;
        }
        transaction.commit()?;
        Ok(moved)
    }

    pub(crate) fn queue_id_for_task(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<String, DownloadStorageError> {
        self.lock()?
            .query_row(
                "SELECT queue_id FROM download_queue_items WHERE task_id = ?1",
                [task_id.as_uuid().to_string()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(DownloadStorageError::TaskNotFound(task_id))
    }

    pub fn append_to_queue(
        &self,
        queue_id: &str,
        task_id: DownloadTaskId,
        is_eligible: bool,
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        append_to_queue_row(&transaction, queue_id, task_id, is_eligible)?;
        transaction.commit()?;
        Ok(())
    }

    pub fn set_queue_task_eligible(
        &self,
        task_id: DownloadTaskId,
        is_eligible: bool,
    ) -> Result<(), DownloadStorageError> {
        let connection = self.lock()?;
        let changed = connection.execute(
            "UPDATE download_queue_items SET is_eligible = ?2 WHERE task_id = ?1",
            params![task_id.as_uuid().to_string(), is_eligible],
        )?;
        if changed == 0 {
            return Err(DownloadStorageError::TaskNotFound(task_id));
        }
        Ok(())
    }

    pub fn set_queue_task_admission(
        &self,
        task_id: DownloadTaskId,
        is_eligible: bool,
        is_user_paused: bool,
    ) -> Result<(), DownloadStorageError> {
        let connection = self.lock()?;
        let changed = connection.execute(
            r#"
            UPDATE download_queue_items
            SET is_eligible = ?2, is_user_paused = ?3
            WHERE task_id = ?1
            "#,
            params![task_id.as_uuid().to_string(), is_eligible, is_user_paused],
        )?;
        if changed == 0 {
            return Err(DownloadStorageError::TaskNotFound(task_id));
        }
        Ok(())
    }

    pub fn is_queue_task_user_paused(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<bool, DownloadStorageError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT is_user_paused FROM download_queue_items WHERE task_id = ?1",
                [task_id.as_uuid().to_string()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(DownloadStorageError::TaskNotFound(task_id))
    }

    pub fn is_queue_task_eligible(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<bool, DownloadStorageError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT is_eligible FROM download_queue_items WHERE task_id = ?1",
                [task_id.as_uuid().to_string()],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(DownloadStorageError::TaskNotFound(task_id))
    }

    pub fn load_eligible_queue_order(
        &self,
        queue_id: &str,
    ) -> Result<Vec<DownloadTaskId>, DownloadStorageError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            r#"
            SELECT q.task_id
            FROM download_queue_items q
            WHERE q.queue_id = ?1 AND q.is_eligible = 1
              AND NOT EXISTS (
                  SELECT 1
                  FROM download_controlled_browser_handoffs h
                  WHERE h.task_id = q.task_id AND h.state = 'held'
              )
            ORDER BY q.position ASC
            "#,
        )?;
        statement
            .query_map([queue_id], |row| {
                let task_id = row.get::<_, String>(0)?;
                Uuid::parse_str(&task_id)
                    .map(DownloadTaskId::from_uuid)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn load_queue_order(
        &self,
        queue_id: &str,
    ) -> Result<Vec<DownloadTaskId>, DownloadStorageError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            r#"
            SELECT task_id
            FROM download_queue_items
            WHERE queue_id = ?1
            ORDER BY position ASC
            "#,
        )?;
        statement
            .query_map([queue_id], |row| {
                let task_id = row.get::<_, String>(0)?;
                Uuid::parse_str(&task_id)
                    .map(DownloadTaskId::from_uuid)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn load_queues(&self) -> Result<Vec<DownloadQueue>, DownloadStorageError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            r#"
            SELECT queue_id, name, position, max_concurrent_tasks, connection_limit,
                   speed_limit_bytes_per_second, schedule_json, is_running, completion_intention_json
            FROM download_queues
            ORDER BY position ASC
            "#,
        )?;
        statement
            .query_map([], |row| {
                let schedule_json = row.get::<_, Option<String>>(6)?;
                let schedule = schedule_json
                    .map(|value| serde_json::from_str(&value))
                    .transpose()
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            6,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                let completion_intention = json_from_text(&row.get::<_, String>(8)?, 8)?;
                Ok(DownloadQueue {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    position: row.get(2)?,
                    max_concurrent_tasks: row.get(3)?,
                    connection_limit: row.get(4)?,
                    speed_limit_bytes_per_second: row
                        .get::<_, Option<i64>>(5)?
                        .map(|value| value as u64),
                    default_save_directory: None,
                    schedule,
                    is_running: row.get(7)?,
                    completion_intention,
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn set_queue_completion_intention(
        &self,
        queue_id: &str,
        mut intention: crate::DownloadCompletionIntention,
        baseline: Option<crate::DownloadCompletionIntention>,
    ) -> Result<DownloadQueue, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        if let Some(baseline) = baseline {
            let json: String = transaction.query_row(
                "SELECT completion_intention_json FROM download_queues WHERE queue_id=?1",
                [queue_id],
                |row| row.get(0),
            )?;
            let mut current: crate::DownloadCompletionIntention = serde_json::from_str(&json)?;
            if intention.action != baseline.action {
                current.action = intention.action;
            }
            if intention.open_directory != baseline.open_directory {
                current.open_directory = intention.open_directory;
            }
            intention = current;
        }
        let json = serde_json::to_string(&intention)?;
        let changed = transaction.execute(
            "UPDATE download_queues SET completion_intention_json = ?2 WHERE queue_id = ?1",
            params![queue_id, json],
        )?;
        if changed == 0 {
            return Err(DownloadStorageError::QueueNotFound(queue_id.to_owned()));
        }
        transaction.commit()?;
        drop(connection);
        self.load_queues()?
            .into_iter()
            .find(|queue| queue.id == queue_id)
            .ok_or_else(|| DownloadStorageError::QueueNotFound(queue_id.to_owned()))
    }

    pub fn set_queue_max_concurrent(
        &self,
        queue_id: &str,
        max_concurrent_tasks: u16,
    ) -> Result<DownloadQueue, DownloadStorageError> {
        let connection = self.lock()?;
        let changed = connection.execute(
            r#"
            UPDATE download_queues
            SET max_concurrent_tasks = ?2
            WHERE queue_id = ?1
            "#,
            params![queue_id, max_concurrent_tasks],
        )?;
        if changed == 0 {
            return Err(DownloadStorageError::QueueNotFound(queue_id.into()));
        }
        drop(connection);
        self.load_queues()?
            .into_iter()
            .find(|queue| queue.id == queue_id)
            .ok_or_else(|| DownloadStorageError::QueueNotFound(queue_id.into()))
    }

    pub fn set_queue_schedule(
        &self,
        queue_id: &str,
        schedule: Option<&DownloadSchedule>,
    ) -> Result<DownloadQueue, DownloadStorageError> {
        let schedule_json = schedule.map(serde_json::to_string).transpose()?;
        let connection = self.lock()?;
        let changed = connection.execute(
            "UPDATE download_queues SET schedule_json = ?2 WHERE queue_id = ?1",
            params![queue_id, schedule_json],
        )?;
        if changed == 0 {
            return Err(DownloadStorageError::QueueNotFound(queue_id.into()));
        }
        drop(connection);
        self.load_queues()?
            .into_iter()
            .find(|queue| queue.id == queue_id)
            .ok_or_else(|| DownloadStorageError::QueueNotFound(queue_id.into()))
    }

    pub fn set_queue_running(
        &self,
        queue_id: &str,
        is_running: bool,
    ) -> Result<(), DownloadStorageError> {
        let connection = self.lock()?;
        let changed = connection.execute(
            "UPDATE download_queues SET is_running = ?2 WHERE queue_id = ?1",
            params![queue_id, is_running],
        )?;
        if changed == 0 {
            return Err(DownloadStorageError::QueueNotFound(queue_id.into()));
        }
        Ok(())
    }

    pub fn save_transfer_plan(
        &self,
        task_id: DownloadTaskId,
        plan: &TransferPlan,
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let plan_json = serde_json::to_string(plan)?;
        let updated = transaction.execute(
            "UPDATE download_tasks SET transfer_plan_json = ?2 WHERE task_id = ?1",
            params![task_id.as_uuid().to_string(), plan_json],
        )?;
        if updated == 0 {
            return Err(DownloadStorageError::TaskNotFound(task_id));
        }

        transaction.execute(
            "DELETE FROM download_segments WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )?;
        for segment in plan.segments() {
            save_segment_row(&transaction, task_id, segment)?;
        }
        transaction.execute(
            r#"
            INSERT INTO download_task_timeline(
                task_id, occurred_at_unix_ms, event_kind, status, segment_count
            )
            VALUES (?1, ?2, 'transfer_plan', NULL, ?3)
            "#,
            params![
                task_id.as_uuid().to_string(),
                now_unix_ms(),
                i64::try_from(plan.segments().len())
                    .map_err(|_| DownloadStorageError::IntegerOverflow)?
            ],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load_task_timeline(
        &self,
        task_id: DownloadTaskId,
        limit: u32,
    ) -> Result<Vec<DownloadTaskTimelineEntry>, DownloadStorageError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            r#"
            SELECT sequence, occurred_at_unix_ms, event_kind, status, segment_count
            FROM download_task_timeline
            WHERE task_id = ?1
            ORDER BY sequence DESC
            LIMIT ?2
            "#,
        )?;
        statement
            .query_map(
                params![task_id.as_uuid().to_string(), i64::from(limit)],
                |row| {
                    let sequence = nonnegative_u64(row.get(0)?, 0)?;
                    let occurred_at_unix_ms = row.get(1)?;
                    let event_kind: String = row.get(2)?;
                    let status: Option<String> = row.get(3)?;
                    let segment_count: Option<i64> = row.get(4)?;
                    let event = match event_kind.as_str() {
                        "created" => DownloadTaskTimelineEvent::TaskCreated,
                        "status" => DownloadTaskTimelineEvent::StatusChanged {
                            status: enum_from_name(
                                status.as_deref().ok_or_else(|| {
                                    rusqlite::Error::InvalidColumnType(
                                        3,
                                        "status".into(),
                                        rusqlite::types::Type::Null,
                                    )
                                })?,
                                3,
                            )?,
                        },
                        "transfer_plan" => {
                            let segment_count = segment_count
                                .ok_or_else(|| {
                                    rusqlite::Error::InvalidColumnType(
                                        4,
                                        "segment_count".into(),
                                        rusqlite::types::Type::Null,
                                    )
                                })
                                .and_then(|count| {
                                    u32::try_from(count).map_err(|error| {
                                        rusqlite::Error::FromSqlConversionFailure(
                                            4,
                                            rusqlite::types::Type::Integer,
                                            Box::new(error),
                                        )
                                    })
                                })?;
                            DownloadTaskTimelineEvent::TransferPlanChanged { segment_count }
                        }
                        _ => return Err(rusqlite::Error::InvalidQuery),
                    };
                    Ok(DownloadTaskTimelineEntry {
                        sequence,
                        occurred_at_unix_ms,
                        event,
                    })
                },
            )?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn load_transfer_plan(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<Option<TransferPlan>, DownloadStorageError> {
        let connection = self.lock()?;
        let Some(stored_plan) = connection
            .query_row(
                "SELECT transfer_plan_json FROM download_tasks WHERE task_id = ?1",
                [task_id.as_uuid().to_string()],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
        else {
            return Ok(None);
        };
        let metadata = serde_json::from_str::<TransferPlan>(&stored_plan)?;
        let mut statement = connection.prepare(
            r#"
            SELECT segment_index, start_byte, end_byte, downloaded_bytes, state
            FROM download_segments
            WHERE task_id = ?1
            ORDER BY start_byte, segment_index
            "#,
        )?;
        let segments = statement
            .query_map([task_id.as_uuid().to_string()], decode_segment_row)?
            .collect::<Result<Vec<_>, _>>()?;
        let plan = TransferPlan::new(
            metadata.protocol(),
            metadata.final_url().to_owned(),
            metadata.total_bytes(),
            metadata.supports_ranges(),
            metadata.validator().clone(),
            segments,
        )
        .map_err(DownloadStorageError::InvalidTransferPlan)?;
        Ok(Some(plan))
    }

    pub fn save_segment_checkpoint(
        &self,
        task_id: DownloadTaskId,
        segment: &DownloadSegment,
    ) -> Result<(), DownloadStorageError> {
        let connection = self.lock()?;
        let updated = connection.execute(
            r#"
            UPDATE download_segments
            SET downloaded_bytes = ?3, state = ?4, updated_at_unix_ms = ?5
            WHERE task_id = ?1
              AND segment_index = ?2
              AND start_byte = ?6
              AND end_byte = ?7
            "#,
            params![
                task_id.as_uuid().to_string(),
                i64::from(segment.index()),
                i64::try_from(segment.downloaded_bytes())
                    .map_err(|_| DownloadStorageError::IntegerOverflow)?,
                enum_name(&segment.state)?,
                now_unix_ms(),
                i64::try_from(segment.start_byte())
                    .map_err(|_| DownloadStorageError::IntegerOverflow)?,
                i64::try_from(segment.end_byte())
                    .map_err(|_| DownloadStorageError::IntegerOverflow)?,
            ],
        )?;
        if updated == 0 {
            return Err(DownloadStorageError::SegmentNotFound {
                task_id,
                segment_index: segment.index(),
            });
        }
        Ok(())
    }

    pub(crate) fn save_hls_transfer_plan(
        &self,
        task_id: DownloadTaskId,
        playlist: &HlsMediaPlaylist,
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM download_tasks WHERE task_id = ?1)",
            [task_id.as_uuid().to_string()],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(DownloadStorageError::TaskNotFound(task_id));
        }
        transaction.execute(
            r#"
            INSERT INTO download_hls_plans(task_id, final_url, media_sequence, updated_at_unix_ms)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(task_id) DO UPDATE SET
                final_url = excluded.final_url,
                media_sequence = excluded.media_sequence,
                updated_at_unix_ms = excluded.updated_at_unix_ms
            "#,
            params![
                task_id.as_uuid().to_string(),
                playlist.final_url,
                i64::try_from(playlist.media_sequence)
                    .map_err(|_| DownloadStorageError::IntegerOverflow)?,
                now_unix_ms(),
            ],
        )?;
        transaction.execute(
            "DELETE FROM download_hls_segments WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )?;
        for segment in &playlist.segments {
            transaction.execute(
                r#"
                INSERT INTO download_hls_segments(
                    task_id, segment_index, sequence_number, url, duration_millis,
                    encryption_json, downloaded_bytes, state, updated_at_unix_ms
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7, ?8)
                "#,
                params![
                    task_id.as_uuid().to_string(),
                    i64::from(segment.index),
                    i64::try_from(segment.sequence)
                        .map_err(|_| DownloadStorageError::IntegerOverflow)?,
                    segment.url,
                    i64::try_from(segment.duration_millis)
                        .map_err(|_| DownloadStorageError::IntegerOverflow)?,
                    serde_json::to_string(&segment.encryption)?,
                    enum_name(&DownloadSegmentState::Waiting)?,
                    now_unix_ms(),
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn load_hls_transfer_plan(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<Option<StoredHlsTransferPlan>, DownloadStorageError> {
        let connection = self.lock()?;
        let Some((final_url, media_sequence)) = connection
            .query_row(
                "SELECT final_url, media_sequence FROM download_hls_plans WHERE task_id = ?1",
                [task_id.as_uuid().to_string()],
                |row| Ok((row.get::<_, String>(0)?, nonnegative_u64(row.get(1)?, 1)?)),
            )
            .optional()?
        else {
            return Ok(None);
        };
        let mut statement = connection.prepare(
            r#"
            SELECT segment_index, sequence_number, url, duration_millis, encryption_json,
                   downloaded_bytes, state
            FROM download_hls_segments
            WHERE task_id = ?1
            ORDER BY segment_index
            "#,
        )?;
        let segments = statement
            .query_map([task_id.as_uuid().to_string()], |row| {
                Ok(StoredHlsSegment {
                    media: HlsMediaSegment {
                        index: u32::try_from(nonnegative_u64(row.get(0)?, 0)?).map_err(
                            |error| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    0,
                                    rusqlite::types::Type::Integer,
                                    Box::new(error),
                                )
                            },
                        )?,
                        sequence: nonnegative_u64(row.get(1)?, 1)?,
                        url: row.get(2)?,
                        duration_millis: nonnegative_u64(row.get(3)?, 3)?,
                        encryption: json_from_text(&row.get::<_, String>(4)?, 4)?,
                    },
                    downloaded_bytes: nonnegative_u64(row.get(5)?, 5)?,
                    state: enum_from_name(&row.get::<_, String>(6)?, 6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(StoredHlsTransferPlan {
            final_url,
            media_sequence,
            segments,
        }))
    }

    pub(crate) fn refresh_hls_transfer_plan_urls(
        &self,
        task_id: DownloadTaskId,
        playlist: &HlsMediaPlaylist,
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let updated = transaction.execute(
            r#"
            UPDATE download_hls_plans
            SET final_url = ?2, media_sequence = ?3, updated_at_unix_ms = ?4
            WHERE task_id = ?1
            "#,
            params![
                task_id.as_uuid().to_string(),
                playlist.final_url,
                i64::try_from(playlist.media_sequence)
                    .map_err(|_| DownloadStorageError::IntegerOverflow)?,
                now_unix_ms(),
            ],
        )?;
        if updated == 0 {
            return Err(DownloadStorageError::TaskNotFound(task_id));
        }
        for segment in &playlist.segments {
            let updated = transaction.execute(
                r#"
                UPDATE download_hls_segments
                SET url = ?3, duration_millis = ?4, encryption_json = ?5,
                    updated_at_unix_ms = ?6
                WHERE task_id = ?1 AND segment_index = ?2 AND sequence_number = ?7
                "#,
                params![
                    task_id.as_uuid().to_string(),
                    i64::from(segment.index),
                    segment.url,
                    i64::try_from(segment.duration_millis)
                        .map_err(|_| DownloadStorageError::IntegerOverflow)?,
                    serde_json::to_string(&segment.encryption)?,
                    now_unix_ms(),
                    i64::try_from(segment.sequence)
                        .map_err(|_| DownloadStorageError::IntegerOverflow)?,
                ],
            )?;
            if updated == 0 {
                return Err(DownloadStorageError::SegmentNotFound {
                    task_id,
                    segment_index: segment.index,
                });
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn save_hls_segment_checkpoint(
        &self,
        task_id: DownloadTaskId,
        segment_index: u32,
        state: DownloadSegmentState,
        downloaded_bytes: u64,
    ) -> Result<(), DownloadStorageError> {
        let connection = self.lock()?;
        let updated = connection.execute(
            r#"
            UPDATE download_hls_segments
            SET downloaded_bytes = ?3, state = ?4, updated_at_unix_ms = ?5
            WHERE task_id = ?1 AND segment_index = ?2
            "#,
            params![
                task_id.as_uuid().to_string(),
                i64::from(segment_index),
                i64::try_from(downloaded_bytes)
                    .map_err(|_| DownloadStorageError::IntegerOverflow)?,
                enum_name(&state)?,
                now_unix_ms(),
            ],
        )?;
        if updated == 0 {
            return Err(DownloadStorageError::SegmentNotFound {
                task_id,
                segment_index,
            });
        }
        Ok(())
    }

    pub(crate) fn clear_hls_transfer_plan(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<(), DownloadStorageError> {
        let connection = self.lock()?;
        connection.execute(
            "DELETE FROM download_hls_plans WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )?;
        Ok(())
    }

    pub(crate) fn save_dash_transfer_plan(
        &self,
        task_id: DownloadTaskId,
        manifest: &DashManifest,
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let exists = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM download_tasks WHERE task_id = ?1)",
            [task_id.as_uuid().to_string()],
            |row| row.get::<_, bool>(0),
        )?;
        if !exists {
            return Err(DownloadStorageError::TaskNotFound(task_id));
        }
        transaction.execute(
            r#"
            INSERT INTO download_dash_plans(task_id, manifest_json, updated_at_unix_ms)
            VALUES (?1, ?2, ?3)
            ON CONFLICT(task_id) DO UPDATE SET
                manifest_json = excluded.manifest_json,
                updated_at_unix_ms = excluded.updated_at_unix_ms
            "#,
            params![
                task_id.as_uuid().to_string(),
                serde_json::to_string(manifest)?,
                now_unix_ms(),
            ],
        )?;
        transaction.execute(
            "DELETE FROM download_dash_fragments WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )?;
        for track in &manifest.tracks {
            if track.initialization_url.is_some() {
                insert_dash_fragment_row(&transaction, task_id, track.kind, -1)?;
            }
            for segment in &track.segments {
                insert_dash_fragment_row(
                    &transaction,
                    task_id,
                    track.kind,
                    i64::from(segment.index),
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn load_dash_transfer_plan(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<Option<StoredDashTransferPlan>, DownloadStorageError> {
        let connection = self.lock()?;
        let Some(manifest) = connection
            .query_row(
                "SELECT manifest_json FROM download_dash_plans WHERE task_id = ?1",
                [task_id.as_uuid().to_string()],
                |row| json_from_text::<DashManifest>(&row.get::<_, String>(0)?, 0),
            )
            .optional()?
        else {
            return Ok(None);
        };
        let mut statement = connection.prepare(
            r#"
            SELECT track_kind, fragment_index, downloaded_bytes, state
            FROM download_dash_fragments
            WHERE task_id = ?1
            ORDER BY track_kind, fragment_index
            "#,
        )?;
        let fragments = statement
            .query_map([task_id.as_uuid().to_string()], |row| {
                let fragment_index = row.get::<_, i64>(1)?;
                let segment_index = if fragment_index == -1 {
                    None
                } else {
                    Some(
                        u32::try_from(nonnegative_u64(fragment_index, 1)?).map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                1,
                                rusqlite::types::Type::Integer,
                                Box::new(error),
                            )
                        })?,
                    )
                };
                Ok(StoredDashFragment {
                    kind: enum_from_name(&row.get::<_, String>(0)?, 0)?,
                    segment_index,
                    downloaded_bytes: nonnegative_u64(row.get(2)?, 2)?,
                    state: enum_from_name(&row.get::<_, String>(3)?, 3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(StoredDashTransferPlan {
            manifest,
            fragments,
        }))
    }

    pub(crate) fn save_dash_fragment_checkpoint(
        &self,
        task_id: DownloadTaskId,
        kind: DashTrackKind,
        segment_index: Option<u32>,
        state: DownloadSegmentState,
        downloaded_bytes: u64,
    ) -> Result<(), DownloadStorageError> {
        let connection = self.lock()?;
        let fragment_index = segment_index.map_or(-1, i64::from);
        let updated = connection.execute(
            r#"
            UPDATE download_dash_fragments
            SET downloaded_bytes = ?4, state = ?5, updated_at_unix_ms = ?6
            WHERE task_id = ?1 AND track_kind = ?2 AND fragment_index = ?3
            "#,
            params![
                task_id.as_uuid().to_string(),
                enum_name(&kind)?,
                fragment_index,
                i64::try_from(downloaded_bytes)
                    .map_err(|_| DownloadStorageError::IntegerOverflow)?,
                enum_name(&state)?,
                now_unix_ms(),
            ],
        )?;
        if updated == 0 {
            return Err(DownloadStorageError::DashFragmentNotFound {
                task_id,
                track_kind: dash_track_kind_name(kind),
                segment_index,
            });
        }
        Ok(())
    }

    pub(crate) fn refresh_dash_transfer_plan_manifest(
        &self,
        task_id: DownloadTaskId,
        manifest: &DashManifest,
    ) -> Result<(), DownloadStorageError> {
        let connection = self.lock()?;
        let updated = connection.execute(
            r#"
            UPDATE download_dash_plans
            SET manifest_json = ?2, updated_at_unix_ms = ?3
            WHERE task_id = ?1
            "#,
            params![
                task_id.as_uuid().to_string(),
                serde_json::to_string(manifest)?,
                now_unix_ms(),
            ],
        )?;
        if updated == 0 {
            return Err(DownloadStorageError::TaskNotFound(task_id));
        }
        Ok(())
    }

    pub(crate) fn clear_dash_transfer_plan(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<(), DownloadStorageError> {
        let connection = self.lock()?;
        connection.execute(
            "DELETE FROM download_dash_plans WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )?;
        Ok(())
    }

    pub fn clear_transfer_plan(&self, task_id: DownloadTaskId) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let updated = transaction.execute(
            "UPDATE download_tasks SET transfer_plan_json = NULL WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )?;
        if updated == 0 {
            return Err(DownloadStorageError::TaskNotFound(task_id));
        }
        transaction.execute(
            "DELETE FROM download_segments WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn save_task_with_transfer(
        &self,
        task: &DownloadTask,
        state: &StoredTransferState,
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        save_task_row(&transaction, task)?;
        save_transfer_state_row(&transaction, task.id(), state)?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn save_paused_url_replacement(
        &self,
        task: &DownloadTask,
        state: &StoredTransferState,
        plan: &TransferPlan,
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        save_task_row(&transaction, task)?;
        save_transfer_state_row(&transaction, task.id(), state)?;
        let changed = transaction.execute(
            "UPDATE download_tasks SET transfer_plan_json = ?2 WHERE task_id = ?1",
            params![
                task.id().as_uuid().to_string(),
                serde_json::to_string(plan)?
            ],
        )?;
        if changed != 1 {
            return Err(DownloadStorageError::TaskNotFound(task.id()));
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn save_task_with_transfer_and_queue(
        &self,
        task: &DownloadTask,
        state: &StoredTransferState,
        queue_id: &str,
        is_eligible: bool,
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        save_task_row(&transaction, task)?;
        save_transfer_state_row(&transaction, task.id(), state)?;
        append_to_queue_row(&transaction, queue_id, task.id(), is_eligible)?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn submission_receipt(
        &self,
        origin: RequestOrigin,
        idempotency_key: Uuid,
    ) -> Result<Option<SubmissionReceipt>, DownloadStorageError> {
        let origin = enum_name(&origin)?;
        let connection = self.lock()?;
        let record = connection
            .query_row(
                r#"
             SELECT task_id, request_fingerprint
             FROM download_submission_receipts
             WHERE origin = ?1 AND idempotency_key = ?2
             "#,
                params![origin, idempotency_key.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        record
            .map(|(task_id, request_fingerprint)| {
                let task_id = Uuid::parse_str(&task_id)
                    .map(DownloadTaskId::from_uuid)
                    .map_err(|_| DownloadStorageError::InvalidTaskId(task_id))?;
                Ok(SubmissionReceipt {
                    task_id,
                    request_fingerprint,
                })
            })
            .transpose()
    }

    pub(crate) fn save_idempotent_task_with_transfer_and_queue(
        &self,
        origin: RequestOrigin,
        idempotency_key: Uuid,
        request_fingerprint: &str,
        task: &DownloadTask,
        state: &StoredTransferState,
        queue: QueueAdmission<'_>,
    ) -> Result<IdempotentSubmissionSave, DownloadStorageError> {
        let origin_name = enum_name(&origin)?;
        let mut connection = self.lock()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        transaction.execute(
            r#"
         DELETE FROM download_submission_receipts
         WHERE NOT EXISTS (
             SELECT 1
             FROM download_tasks
             WHERE download_tasks.task_id = download_submission_receipts.task_id
         )
         "#,
            [],
        )?;
        let existing = transaction
            .query_row(
                r#"
             SELECT task_id, request_fingerprint
             FROM download_submission_receipts
             WHERE origin = ?1 AND idempotency_key = ?2
             "#,
                params![origin_name, idempotency_key.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((task_id, stored_fingerprint)) = existing {
            if stored_fingerprint != request_fingerprint {
                return Err(DownloadStorageError::IdempotencyConflict {
                    key: idempotency_key,
                });
            }
            let task_id = Uuid::parse_str(&task_id)
                .map(DownloadTaskId::from_uuid)
                .map_err(|_| DownloadStorageError::InvalidTaskId(task_id))?;
            transaction.commit()?;
            return Ok(IdempotentSubmissionSave::Duplicate(task_id));
        }

        save_task_row(&transaction, task)?;
        save_transfer_state_row(&transaction, task.id(), state)?;
        append_to_queue_row(&transaction, queue.id, task.id(), queue.eligible)?;
        transaction.execute(
            r#"
         INSERT INTO download_submission_receipts(
             origin, idempotency_key, request_fingerprint, task_id, accepted_at_unix_ms
         )
         VALUES (?1, ?2, ?3, ?4, ?5)
         "#,
            params![
                origin_name,
                idempotency_key.to_string(),
                request_fingerprint,
                task.id().as_uuid().to_string(),
                now_unix_ms(),
            ],
        )?;
        transaction.commit()?;
        Ok(IdempotentSubmissionSave::Inserted(task.id()))
    }

    pub(crate) fn save_redownload_task_with_transfer_and_queue(
        &self,
        old_task_id: DownloadTaskId,
        task: &DownloadTask,
        state: &StoredTransferState,
        queue_id: &str,
        is_eligible: bool,
    ) -> Result<RedownloadOperationSave, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = transaction
            .query_row(
                "SELECT replacement_task_id FROM download_redownload_operations WHERE old_task_id = ?1",
                [old_task_id.as_uuid().to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if let Some(replacement_task_id) = existing {
            let replacement_task_id = Uuid::parse_str(&replacement_task_id)
                .map(DownloadTaskId::from_uuid)
                .map_err(|_| DownloadStorageError::InvalidTaskId(replacement_task_id))?;
            transaction.commit()?;
            return Ok(RedownloadOperationSave::Existing(replacement_task_id));
        }
        let old_exists: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM download_tasks WHERE task_id = ?1)",
            [old_task_id.as_uuid().to_string()],
            |row| row.get(0),
        )?;
        if !old_exists {
            return Err(DownloadStorageError::TaskNotFound(old_task_id));
        }
        save_task_row(&transaction, task)?;
        save_transfer_state_row(&transaction, task.id(), state)?;
        append_to_queue_row(&transaction, queue_id, task.id(), is_eligible)?;
        transaction.execute(
            r#"
            INSERT INTO download_redownload_operations(
                old_task_id, replacement_task_id, created_at_unix_ms
            )
            VALUES (?1, ?2, ?3)
            "#,
            params![
                old_task_id.as_uuid().to_string(),
                task.id().as_uuid().to_string(),
                now_unix_ms(),
            ],
        )?;
        transaction.commit()?;
        Ok(RedownloadOperationSave::Inserted(task.id()))
    }

    pub(crate) fn load_redownload_operations(
        &self,
    ) -> Result<Vec<(DownloadTaskId, DownloadTaskId)>, DownloadStorageError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT old_task_id, replacement_task_id FROM download_redownload_operations",
        )?;
        statement
            .query_map([], |row| {
                let old_task_id = row.get::<_, String>(0)?;
                let replacement_task_id = row.get::<_, String>(1)?;
                let old_task_id = Uuid::parse_str(&old_task_id)
                    .map(DownloadTaskId::from_uuid)
                    .map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(DownloadStorageError::InvalidTaskId(old_task_id)),
                        )
                    })?;
                let replacement_task_id = Uuid::parse_str(&replacement_task_id)
                    .map(DownloadTaskId::from_uuid)
                    .map_err(|_| {
                        rusqlite::Error::FromSqlConversionFailure(
                            1,
                            rusqlite::types::Type::Text,
                            Box::new(DownloadStorageError::InvalidTaskId(replacement_task_id)),
                        )
                    })?;
                Ok((old_task_id, replacement_task_id))
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub(crate) fn complete_redownload_operation(
        &self,
        old_task_id: DownloadTaskId,
        replacement_task_id: DownloadTaskId,
    ) -> Result<bool, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(stored_replacement_task_id) = transaction
            .query_row(
                "SELECT replacement_task_id FROM download_redownload_operations WHERE old_task_id = ?1",
                [old_task_id.as_uuid().to_string()],
                |row| row.get::<_, String>(0),
            )
            .optional()?
        else {
            transaction.commit()?;
            return Ok(false);
        };
        if stored_replacement_task_id != replacement_task_id.as_uuid().to_string() {
            return Err(DownloadStorageError::TaskNotFound(replacement_task_id));
        }
        let changed = transaction.execute(
            "UPDATE download_queue_items SET is_eligible = 1, is_user_paused = 0 WHERE task_id = ?1",
            [replacement_task_id.as_uuid().to_string()],
        )?;
        if changed == 0 {
            return Err(DownloadStorageError::TaskNotFound(replacement_task_id));
        }
        let removed = transaction.execute(
            "DELETE FROM download_tasks WHERE task_id = ?1",
            [old_task_id.as_uuid().to_string()],
        )?;
        if removed == 0 {
            return Err(DownloadStorageError::TaskNotFound(old_task_id));
        }
        transaction.commit()?;
        Ok(true)
    }

    pub(crate) fn save_browser_task_with_transfer_and_queue(
        &self,
        handoff_id: Uuid,
        request_fingerprint: &str,
        task: &DownloadTask,
        state: &StoredTransferState,
        queue_id: &str,
        is_eligible: bool,
    ) -> Result<BrowserHandoffSave, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let accepted_at_unix_ms = now_unix_ms();
        transaction.execute(
            r#"
            DELETE FROM download_browser_handoffs
            WHERE accepted_at_unix_ms < ?1
              AND NOT EXISTS (
                  SELECT 1
                  FROM download_tasks
                  WHERE download_tasks.task_id = download_browser_handoffs.task_id
              )
            "#,
            [accepted_at_unix_ms.saturating_sub(BROWSER_HANDOFF_RETENTION_MS)],
        )?;
        let existing = transaction
            .query_row(
                r#"
                SELECT task_id, request_fingerprint
                FROM download_browser_handoffs
                WHERE handoff_id = ?1
                "#,
                [handoff_id.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        if let Some((task_id, stored_fingerprint)) = existing {
            if !stored_fingerprint.is_empty() && stored_fingerprint != request_fingerprint {
                return Err(DownloadStorageError::BrowserHandoffConflict { handoff_id });
            }
            if stored_fingerprint.is_empty() {
                transaction.execute(
                    "UPDATE download_browser_handoffs SET request_fingerprint = ?2 WHERE handoff_id = ?1",
                    params![handoff_id.to_string(), request_fingerprint],
                )?;
            }
            transaction.commit()?;
            let task_id = Uuid::parse_str(&task_id)
                .map(DownloadTaskId::from_uuid)
                .map_err(|_| DownloadStorageError::InvalidTaskId(task_id))?;
            return Ok(BrowserHandoffSave::Duplicate(task_id));
        }

        save_task_row(&transaction, task)?;
        save_transfer_state_row(&transaction, task.id(), state)?;
        append_to_queue_row(&transaction, queue_id, task.id(), is_eligible)?;
        transaction.execute(
            r#"
            INSERT INTO download_browser_handoffs(
                handoff_id, task_id, request_fingerprint, accepted_at_unix_ms
            )
            VALUES (?1, ?2, ?3, ?4)
            "#,
            params![
                handoff_id.to_string(),
                task.id().as_uuid().to_string(),
                request_fingerprint,
                accepted_at_unix_ms,
            ],
        )?;
        transaction.execute(
            r#"
            DELETE FROM download_browser_handoffs
            WHERE handoff_id <> ?1
              AND handoff_id IN (
                SELECT h.handoff_id
                FROM download_browser_handoffs h
                WHERE h.handoff_id <> ?1
                  AND NOT EXISTS (
                      SELECT 1
                      FROM download_tasks t
                      WHERE t.task_id = h.task_id
                  )
                ORDER BY h.accepted_at_unix_ms DESC, h.handoff_id DESC
                LIMIT -1 OFFSET ?2
            )
            "#,
            params![handoff_id.to_string(), MAX_BROWSER_HANDOFFS - 1],
        )?;
        transaction.commit()?;
        Ok(BrowserHandoffSave::Inserted(task.id()))
    }

    pub(crate) fn browser_handoff(
        &self,
        handoff_id: Uuid,
    ) -> Result<Option<BrowserHandoff>, DownloadStorageError> {
        let connection = self.lock()?;
        let record = connection
            .query_row(
                r#"
                SELECT task_id, request_fingerprint
                FROM download_browser_handoffs
                WHERE handoff_id = ?1
                "#,
                [handoff_id.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        record
            .map(|(task_id, stored_fingerprint)| {
                let task_id = Uuid::parse_str(&task_id)
                    .map(DownloadTaskId::from_uuid)
                    .map_err(|_| DownloadStorageError::InvalidTaskId(task_id))?;
                Ok(BrowserHandoff {
                    task_id,
                    request_fingerprint: (!stored_fingerprint.is_empty())
                        .then_some(stored_fingerprint),
                })
            })
            .transpose()
    }

    pub(crate) fn backfill_browser_handoff_fingerprint(
        &self,
        handoff_id: Uuid,
        request_fingerprint: &str,
    ) -> Result<bool, DownloadStorageError> {
        let connection = self.lock()?;
        let updated = connection.execute(
            r#"
            UPDATE download_browser_handoffs
            SET request_fingerprint = ?2
            WHERE handoff_id = ?1 AND request_fingerprint = ''
            "#,
            params![handoff_id.to_string(), request_fingerprint],
        )?;
        Ok(updated != 0)
    }

    pub(crate) fn prepare_controlled_browser_handoff(
        &self,
        handoff_id: Uuid,
        request_fingerprint: &str,
        now_unix_ms: i64,
    ) -> Result<ControlledHandoffPrepare, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let expired = expire_controlled_handoff(&transaction, handoff_id, now_unix_ms)?;
        if expired {
            prune_controlled_handoffs(&transaction, handoff_id)?;
        }
        if let Some(record) = load_controlled_handoff(&transaction, handoff_id)? {
            if record
                .request_fingerprint
                .as_deref()
                .is_some_and(|stored| stored != request_fingerprint)
            {
                return Err(DownloadStorageError::BrowserHandoffConflict { handoff_id });
            }
            transaction.commit()?;
            return Ok(ControlledHandoffPrepare {
                record,
                inserted: false,
                expired,
            });
        }
        let deadline_unix_ms = now_unix_ms.saturating_add(CONTROLLED_BROWSER_HANDOFF_LEASE_MS);
        transaction.execute(
            r#"
            INSERT INTO download_controlled_browser_handoffs(
                handoff_id, state, task_id, request_fingerprint, start_now,
                deadline_unix_ms, created_at_unix_ms, updated_at_unix_ms
            ) VALUES (?1, 'pending', NULL, ?2, NULL, ?3, ?4, ?4)
            "#,
            params![
                handoff_id.to_string(),
                request_fingerprint,
                deadline_unix_ms,
                now_unix_ms
            ],
        )?;
        prune_controlled_handoffs(&transaction, handoff_id)?;
        transaction.commit()?;
        Ok(ControlledHandoffPrepare {
            record: ControlledBrowserHandoff {
                handoff_id,
                state: BrowserHandoffState::Pending,
                task_id: None,
                request_fingerprint: Some(request_fingerprint.to_owned()),
                start_now: None,
            },
            inserted: true,
            expired: false,
        })
    }

    pub(crate) fn controlled_browser_handoff_status(
        &self,
        handoff_id: Uuid,
        now_unix_ms: i64,
        renew_pending: bool,
    ) -> Result<ControlledHandoffStatus, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let expired = expire_controlled_handoff(&transaction, handoff_id, now_unix_ms)?;
        if expired {
            prune_controlled_handoffs(&transaction, handoff_id)?;
        }
        let record = load_controlled_handoff(&transaction, handoff_id)?
            .unwrap_or_else(|| ControlledBrowserHandoff::missing(handoff_id));
        if renew_pending && record.state == BrowserHandoffState::Pending {
            let deadline_unix_ms = now_unix_ms.saturating_add(CONTROLLED_BROWSER_HANDOFF_LEASE_MS);
            transaction.execute(
                "UPDATE download_controlled_browser_handoffs SET deadline_unix_ms = ?2, updated_at_unix_ms = ?3 WHERE handoff_id = ?1 AND state = 'pending'",
                params![handoff_id.to_string(), deadline_unix_ms, now_unix_ms],
            )?;
        }
        transaction.commit()?;
        Ok(ControlledHandoffStatus { record, expired })
    }

    pub(crate) fn accept_controlled_browser_handoff(
        &self,
        handoff_id: Uuid,
        task: &DownloadTask,
        state: &StoredTransferState,
        queue_id: &str,
        start_now: bool,
        now_unix_ms: i64,
    ) -> Result<ControlledHandoffAcceptSave, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        expire_controlled_handoff(&transaction, handoff_id, now_unix_ms)?;
        let Some(record) = load_controlled_handoff(&transaction, handoff_id)? else {
            transaction.commit()?;
            return Ok(ControlledHandoffAcceptSave::Existing(
                ControlledBrowserHandoff::missing(handoff_id),
            ));
        };
        if record.state != BrowserHandoffState::Pending {
            transaction.commit()?;
            return Ok(ControlledHandoffAcceptSave::Existing(record));
        }
        save_task_row(&transaction, task)?;
        save_transfer_state_row(&transaction, task.id(), state)?;
        append_to_queue_row(&transaction, queue_id, task.id(), false)?;
        let changed = transaction.execute(
            r#"
            UPDATE download_controlled_browser_handoffs
            SET state = 'held', task_id = ?2, start_now = ?3,
                deadline_unix_ms = NULL, updated_at_unix_ms = ?4
            WHERE handoff_id = ?1 AND state = 'pending'
            "#,
            params![
                handoff_id.to_string(),
                task.id().as_uuid().to_string(),
                start_now,
                now_unix_ms
            ],
        )?;
        if changed != 1 {
            return Err(DownloadStorageError::BrowserHandoffStateConflict { handoff_id });
        }
        transaction.commit()?;
        Ok(ControlledHandoffAcceptSave::Inserted(task.id()))
    }

    pub(crate) fn commit_controlled_browser_handoff(
        &self,
        handoff_id: Uuid,
        task: &DownloadTask,
        schedule_due: bool,
        now_unix_ms: i64,
    ) -> Result<ControlledHandoffCommit, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(record) = load_controlled_handoff(&transaction, handoff_id)? else {
            transaction.commit()?;
            return Ok(ControlledHandoffCommit {
                record: ControlledBrowserHandoff::missing(handoff_id),
                changed: false,
                queue_eligible: false,
            });
        };
        if record.state != BrowserHandoffState::Held {
            transaction.commit()?;
            return Ok(ControlledHandoffCommit {
                record,
                changed: false,
                queue_eligible: false,
            });
        }
        if record.task_id != Some(task.id()) {
            return Err(DownloadStorageError::BrowserHandoffTaskMismatch { handoff_id });
        }
        let queue_eligible = record.start_now == Some(true) && schedule_due;
        if queue_eligible {
            save_task_row(&transaction, task)?;
        }
        let changed = transaction.execute(
            "UPDATE download_queue_items SET is_eligible = ?2, is_user_paused = 0 WHERE task_id = ?1",
            params![task.id().as_uuid().to_string(), queue_eligible],
        )?;
        if changed != 1 {
            return Err(DownloadStorageError::TaskNotFound(task.id()));
        }
        transaction.execute(
            "UPDATE download_controlled_browser_handoffs SET state = 'committed', deadline_unix_ms = NULL, updated_at_unix_ms = ?2 WHERE handoff_id = ?1 AND state = 'held'",
            params![handoff_id.to_string(), now_unix_ms],
        )?;
        prune_controlled_handoffs(&transaction, handoff_id)?;
        let mut committed = record;
        committed.state = BrowserHandoffState::Committed;
        transaction.commit()?;
        Ok(ControlledHandoffCommit {
            record: committed,
            changed: true,
            queue_eligible,
        })
    }

    pub(crate) fn abort_controlled_browser_handoff(
        &self,
        handoff_id: Uuid,
        cancelled_task: Option<&DownloadTask>,
        now_unix_ms: i64,
    ) -> Result<ControlledHandoffAbort, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let expired = expire_controlled_handoff(&transaction, handoff_id, now_unix_ms)?;
        if expired {
            prune_controlled_handoffs(&transaction, handoff_id)?;
        }
        let Some(mut record) = load_controlled_handoff(&transaction, handoff_id)? else {
            transaction.execute(
                r#"
                INSERT INTO download_controlled_browser_handoffs(
                    handoff_id, state, task_id, request_fingerprint, start_now,
                    deadline_unix_ms, created_at_unix_ms, updated_at_unix_ms
                ) VALUES (?1, 'aborted', NULL, NULL, NULL, NULL, ?2, ?2)
                "#,
                params![handoff_id.to_string(), now_unix_ms],
            )?;
            prune_controlled_handoffs(&transaction, handoff_id)?;
            transaction.commit()?;
            return Ok(ControlledHandoffAbort {
                record: ControlledBrowserHandoff {
                    handoff_id,
                    state: BrowserHandoffState::Aborted,
                    task_id: None,
                    request_fingerprint: None,
                    start_now: None,
                },
                changed: true,
                cancelled_task_id: None,
                expired: false,
            });
        };
        if record.state == BrowserHandoffState::Committed
            || record.state == BrowserHandoffState::Aborted
            || record.state == BrowserHandoffState::Expired
        {
            transaction.commit()?;
            return Ok(ControlledHandoffAbort {
                record,
                changed: false,
                cancelled_task_id: None,
                expired,
            });
        }
        let cancelled_task_id = if record.state == BrowserHandoffState::Held {
            let task = cancelled_task
                .filter(|task| Some(task.id()) == record.task_id)
                .ok_or(DownloadStorageError::BrowserHandoffTaskMismatch { handoff_id })?;
            save_task_row(&transaction, task)?;
            let changed = transaction.execute(
                "UPDATE download_queue_items SET is_eligible = 0, is_user_paused = 1 WHERE task_id = ?1",
                [task.id().as_uuid().to_string()],
            )?;
            if changed != 1 {
                return Err(DownloadStorageError::TaskNotFound(task.id()));
            }
            Some(task.id())
        } else {
            None
        };
        transaction.execute(
            "UPDATE download_controlled_browser_handoffs SET state = 'aborted', deadline_unix_ms = NULL, updated_at_unix_ms = ?2 WHERE handoff_id = ?1",
            params![handoff_id.to_string(), now_unix_ms],
        )?;
        prune_controlled_handoffs(&transaction, handoff_id)?;
        record.state = BrowserHandoffState::Aborted;
        transaction.commit()?;
        Ok(ControlledHandoffAbort {
            record,
            changed: true,
            cancelled_task_id,
            expired,
        })
    }

    pub(crate) fn is_controlled_browser_handoff_held_task(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<bool, DownloadStorageError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM download_controlled_browser_handoffs WHERE task_id = ?1 AND state = 'held')",
                [task_id.as_uuid().to_string()],
                |row| row.get(0),
            )
            .map_err(Into::into)
    }

    pub(crate) fn controlled_browser_handoff_start_now(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<Option<bool>, DownloadStorageError> {
        self.lock()?
            .query_row(
                "SELECT start_now FROM download_controlled_browser_handoffs WHERE task_id = ?1 AND state IN ('held', 'committed')",
                [task_id.as_uuid().to_string()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn save_task_with_transfer_and_eligibility(
        &self,
        task: &DownloadTask,
        state: &StoredTransferState,
        is_eligible: bool,
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        save_task_row(&transaction, task)?;
        save_transfer_state_row(&transaction, task.id(), state)?;
        let changed = transaction.execute(
            "UPDATE download_queue_items SET is_eligible = ?2 WHERE task_id = ?1",
            params![task.id().as_uuid().to_string(), is_eligible],
        )?;
        if changed == 0 {
            return Err(DownloadStorageError::TaskNotFound(task.id()));
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn save_task_with_transfer_and_admission(
        &self,
        task: &DownloadTask,
        state: &StoredTransferState,
        is_eligible: bool,
        is_user_paused: bool,
    ) -> Result<(), DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        save_task_row(&transaction, task)?;
        save_transfer_state_row(&transaction, task.id(), state)?;
        let changed = transaction.execute(
            r#"
            UPDATE download_queue_items
            SET is_eligible = ?2, is_user_paused = ?3
            WHERE task_id = ?1
            "#,
            params![task.id().as_uuid().to_string(), is_eligible, is_user_paused],
        )?;
        if changed == 0 {
            return Err(DownloadStorageError::TaskNotFound(task.id()));
        }
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn load_transfer_state(
        &self,
        task_id: DownloadTaskId,
    ) -> Result<StoredTransferState, DownloadStorageError> {
        let connection = self.lock()?;
        let mut state = connection
            .query_row(
                r#"
                SELECT
                    final_url,
                    total_bytes,
                    downloaded_bytes,
                    supports_ranges,
                    etag,
                    last_modified,
                    error_code,
                    error_message,
                    error_retryable,
                    request_json
                FROM download_tasks
                WHERE task_id = ?1
                "#,
                [task_id.as_uuid().to_string()],
                |row| {
                    let final_url: Option<String> = row.get(0)?;
                    let total_bytes = optional_nonnegative_u64(row.get(1)?, 1)?;
                    let downloaded_bytes = nonnegative_u64(row.get(2)?, 2)?;
                    let supports_ranges: Option<bool> = row.get(3)?;
                    let etag: Option<String> = row.get(4)?;
                    let last_modified: Option<String> = row.get(5)?;
                    let error_code: Option<String> = row.get(6)?;
                    let error_message: Option<String> = row.get(7)?;
                    let error_retryable: bool = row.get(8)?;
                    let probe = final_url.map(|final_url| DownloadProbe {
                        final_url,
                        file_name: None,
                        total_bytes,
                        supports_ranges: supports_ranges.unwrap_or(false),
                        content_type: None,
                        validator: ResourceValidator {
                            etag,
                            last_modified,
                        },
                        media: None,
                        resolution: None,
                        server_observation: None,
                    });
                    let stored: StoredResolvedRequest =
                        json_from_text(&row.get::<_, String>(9)?, 9)?;
                    let probe = stored.probe.or(probe);
                    let error = match (error_code, error_message) {
                        (Some(code), Some(message)) => Some(DownloadFault {
                            code: enum_from_name(&code, 6)?,
                            message,
                            retryable: error_retryable,
                        }),
                        _ => None,
                    };
                    Ok(StoredTransferState {
                        probe,
                        downloaded_bytes,
                        error,
                        prepared_size_bytes: None,
                        temporary_path: None,
                        final_path: None,
                        final_committed: false,
                    })
                },
            )
            .optional()?
            .ok_or(DownloadStorageError::TaskNotFound(task_id))?;
        let mut statement = connection.prepare(
            "SELECT kind, path, state, size_bytes FROM download_artifacts WHERE task_id = ?1",
        )?;
        let artifacts = statement.query_map([task_id.as_uuid().to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                PathBuf::from(row.get::<_, String>(1)?),
                row.get::<_, String>(2)?,
                optional_nonnegative_u64(row.get(3)?, 3)?,
            ))
        })?;
        for artifact in artifacts {
            let (kind, path, artifact_state, size_bytes) = artifact?;
            match kind.as_str() {
                "temporary" => {
                    state.temporary_path = Some(path);
                    state.prepared_size_bytes = (artifact_state == "prepared")
                        .then_some(size_bytes)
                        .flatten();
                }
                "final" => {
                    state.final_path = Some(path);
                    state.final_committed = artifact_state == "committed";
                }
                _ => {}
            }
        }
        if let Some(probe) = state.probe.as_mut()
            && probe.file_name.is_none()
        {
            probe.file_name = state
                .final_path
                .as_ref()
                .and_then(|path| path.file_name())
                .map(|name| name.to_string_lossy().into_owned());
        }
        Ok(state)
    }

    pub fn remove_task(&self, task_id: DownloadTaskId) -> Result<bool, DownloadStorageError> {
        let connection = self.lock()?;
        let removed = connection.execute(
            "DELETE FROM download_tasks WHERE task_id = ?1",
            [task_id.as_uuid().to_string()],
        )?;
        Ok(removed > 0)
    }

    pub fn recover_interrupted_tasks(
        &self,
        now_unix_ms: i64,
    ) -> Result<usize, DownloadStorageError> {
        let mut connection = self.lock()?;
        let transaction = connection.transaction()?;
        let placeholders = vec!["?"; INTERRUPTED_DOWNLOAD_STATUSES.len()].join(", ");
        let active_status_parameters = INTERRUPTED_DOWNLOAD_STATUSES
            .iter()
            .map(|status| Value::Text(status.storage_name().to_owned()))
            .collect::<Vec<_>>();
        let segment_statement = format!(
            r#"
            UPDATE download_segments
            SET state = 'waiting', updated_at_unix_ms = ?
            WHERE state = 'active'
              AND task_id IN (
                  SELECT task_id
                  FROM download_tasks
                  WHERE status IN ({placeholders})
              )
            "#
        );
        let segment_parameters = std::iter::once(Value::Integer(now_unix_ms))
            .chain(active_status_parameters)
            .collect::<Vec<_>>();
        transaction.execute(&segment_statement, params_from_iter(segment_parameters))?;
        let statement = format!(
            r#"
            UPDATE download_tasks
            SET status = 'paused', updated_at_unix_ms = ?,
                measured_payload_bytes = NULL, active_download_ms = NULL
            WHERE status IN ({placeholders})
            "#
        );
        let parameters = std::iter::once(Value::Integer(now_unix_ms))
            .chain(
                INTERRUPTED_DOWNLOAD_STATUSES
                    .iter()
                    .map(|status| Value::Text(status.storage_name().to_owned())),
            )
            .collect::<Vec<_>>();
        let updated = transaction.execute(&statement, params_from_iter(parameters))?;
        transaction.commit()?;
        Ok(updated)
    }

    #[cfg(test)]
    pub(crate) fn inject_artifact_update_failure(&self) -> Result<(), DownloadStorageError> {
        self.lock()?.execute_batch(
            r#"
            CREATE TRIGGER reject_artifact_update_for_test
            BEFORE UPDATE ON download_artifacts
            BEGIN
                SELECT RAISE(ABORT, 'injected artifact update failure');
            END;
            "#,
        )?;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn inject_browser_handoff_insert_failure(&self) -> Result<(), DownloadStorageError> {
        self.lock()?.execute_batch(
            r#"
            CREATE TRIGGER reject_browser_handoff_insert_for_test
            BEFORE INSERT ON download_browser_handoffs
            BEGIN
                SELECT RAISE(ABORT, 'injected browser handoff failure');
            END;
            "#,
        )?;
        Ok(())
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, DownloadStorageError> {
        self.connection
            .lock()
            .map_err(|_| DownloadStorageError::LockPoisoned)
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StoredTransferState {
    pub probe: Option<DownloadProbe>,
    pub downloaded_bytes: u64,
    pub error: Option<DownloadFault>,
    pub prepared_size_bytes: Option<u64>,
    pub temporary_path: Option<PathBuf>,
    pub final_path: Option<PathBuf>,
    pub final_committed: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredHlsTransferPlan {
    pub final_url: String,
    pub media_sequence: u64,
    pub segments: Vec<StoredHlsSegment>,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredHlsSegment {
    pub media: HlsMediaSegment,
    pub downloaded_bytes: u64,
    pub state: DownloadSegmentState,
}

#[derive(Debug, Clone)]
pub(crate) struct StoredDashTransferPlan {
    pub manifest: DashManifest,
    pub fragments: Vec<StoredDashFragment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredDashFragment {
    pub kind: DashTrackKind,
    pub segment_index: Option<u32>,
    pub downloaded_bytes: u64,
    pub state: DownloadSegmentState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RedownloadOperationSave {
    Inserted(DownloadTaskId),
    Existing(DownloadTaskId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserHandoffSave {
    Inserted(DownloadTaskId),
    Duplicate(DownloadTaskId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BrowserHandoff {
    pub task_id: DownloadTaskId,
    pub request_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlledBrowserHandoff {
    pub(crate) handoff_id: Uuid,
    pub(crate) state: BrowserHandoffState,
    pub(crate) task_id: Option<DownloadTaskId>,
    pub(crate) request_fingerprint: Option<String>,
    pub(crate) start_now: Option<bool>,
}

impl ControlledBrowserHandoff {
    fn missing(handoff_id: Uuid) -> Self {
        Self {
            handoff_id,
            state: BrowserHandoffState::Missing,
            task_id: None,
            request_fingerprint: None,
            start_now: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlledHandoffPrepare {
    pub(crate) record: ControlledBrowserHandoff,
    pub(crate) inserted: bool,
    pub(crate) expired: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlledHandoffStatus {
    pub(crate) record: ControlledBrowserHandoff,
    pub(crate) expired: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ControlledHandoffAcceptSave {
    Inserted(DownloadTaskId),
    Existing(ControlledBrowserHandoff),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlledHandoffCommit {
    pub(crate) record: ControlledBrowserHandoff,
    pub(crate) changed: bool,
    pub(crate) queue_eligible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ControlledHandoffAbort {
    pub(crate) record: ControlledBrowserHandoff,
    pub(crate) changed: bool,
    pub(crate) cancelled_task_id: Option<DownloadTaskId>,
    pub(crate) expired: bool,
}

fn load_controlled_handoff(
    connection: &Connection,
    handoff_id: Uuid,
) -> Result<Option<ControlledBrowserHandoff>, DownloadStorageError> {
    connection
        .query_row(
            r#"
            SELECT state, task_id, request_fingerprint, start_now
            FROM download_controlled_browser_handoffs
            WHERE handoff_id = ?1
            "#,
            [handoff_id.to_string()],
            |row| {
                let state_name = row.get::<_, String>(0)?;
                let state = match state_name.as_str() {
                    "pending" => BrowserHandoffState::Pending,
                    "held" => BrowserHandoffState::Held,
                    "committed" => BrowserHandoffState::Committed,
                    "aborted" => BrowserHandoffState::Aborted,
                    "expired" => BrowserHandoffState::Expired,
                    _ => {
                        return Err(rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(DownloadStorageError::InvalidEnumEncoding),
                        ));
                    }
                };
                let task_id = row
                    .get::<_, Option<String>>(1)?
                    .map(|task_id| {
                        Uuid::parse_str(&task_id)
                            .map(DownloadTaskId::from_uuid)
                            .map_err(|_| {
                                rusqlite::Error::FromSqlConversionFailure(
                                    1,
                                    rusqlite::types::Type::Text,
                                    Box::new(DownloadStorageError::InvalidTaskId(task_id)),
                                )
                            })
                    })
                    .transpose()?;
                Ok(ControlledBrowserHandoff {
                    handoff_id,
                    state,
                    task_id,
                    request_fingerprint: row.get(2)?,
                    start_now: row.get(3)?,
                })
            },
        )
        .optional()
        .map_err(Into::into)
}

fn expire_controlled_handoff(
    connection: &Connection,
    handoff_id: Uuid,
    now_unix_ms: i64,
) -> Result<bool, DownloadStorageError> {
    Ok(connection.execute(
        r#"
        UPDATE download_controlled_browser_handoffs
        SET state = 'expired', deadline_unix_ms = NULL, updated_at_unix_ms = ?2
        WHERE handoff_id = ?1 AND state = 'pending'
          AND deadline_unix_ms IS NOT NULL AND deadline_unix_ms <= ?2
        "#,
        params![handoff_id.to_string(), now_unix_ms],
    )? != 0)
}

fn prune_controlled_handoffs(
    connection: &Connection,
    retained_handoff_id: Uuid,
) -> Result<(), DownloadStorageError> {
    connection.execute(
        r#"
        DELETE FROM download_controlled_browser_handoffs
        WHERE handoff_id <> ?1
          AND state IN ('committed', 'aborted', 'expired')
          AND handoff_id IN (
              SELECT handoff_id
              FROM download_controlled_browser_handoffs
              WHERE handoff_id <> ?1
                AND state IN ('committed', 'aborted', 'expired')
              ORDER BY updated_at_unix_ms DESC, handoff_id DESC
              LIMIT -1 OFFSET ?2
          )
        "#,
        params![retained_handoff_id.to_string(), MAX_BROWSER_HANDOFFS - 1],
    )?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdempotentSubmissionSave {
    Inserted(DownloadTaskId),
    Duplicate(DownloadTaskId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SubmissionReceipt {
    pub task_id: DownloadTaskId,
    pub request_fingerprint: String,
}

fn save_task_row(connection: &Connection, task: &DownloadTask) -> Result<(), DownloadStorageError> {
    let request_json = serde_json::to_string(&StoredResolvedRequest {
        request: task.request().clone(),
        resolution: task.resolution().cloned(),
        probe: None,
        completion_intention: task.completion_intention(),
        completion_attempt: task.completion_attempt(),
    })?;
    let origin = enum_name(&task.origin)?;
    connection.execute(
        r#"
        INSERT INTO download_tasks (
            task_id,
            request_json,
            origin,
            status,
            created_at_unix_ms,
            updated_at_unix_ms,
            completed_at_unix_ms,
            measured_payload_bytes,
            active_download_ms
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(task_id) DO UPDATE SET
            request_json = json_set(excluded.request_json, '$.probe',
                json_extract(download_tasks.request_json, '$.probe')),
            origin = excluded.origin,
            status = excluded.status,
            updated_at_unix_ms = excluded.updated_at_unix_ms,
            completed_at_unix_ms = excluded.completed_at_unix_ms,
            measured_payload_bytes = excluded.measured_payload_bytes,
            active_download_ms = excluded.active_download_ms
        "#,
        params![
            task.id.as_uuid().to_string(),
            request_json,
            origin,
            task.status.storage_name(),
            task.created_at_unix_ms,
            task.updated_at_unix_ms,
            task.completed_at_unix_ms,
            task.transfer_statistics()
                .map(|stats| i64::try_from(stats.transferred_bytes))
                .transpose()
                .map_err(|_| DownloadStorageError::IntegerOverflow)?,
            task.transfer_statistics()
                .map(|stats| i64::try_from(stats.active_download_ms))
                .transpose()
                .map_err(|_| DownloadStorageError::IntegerOverflow)?,
        ],
    )?;
    Ok(())
}

fn insert_dash_fragment_row(
    connection: &Connection,
    task_id: DownloadTaskId,
    kind: DashTrackKind,
    fragment_index: i64,
) -> Result<(), DownloadStorageError> {
    connection.execute(
        r#"
        INSERT INTO download_dash_fragments(
            task_id, track_kind, fragment_index, downloaded_bytes, state, updated_at_unix_ms
        )
        VALUES (?1, ?2, ?3, 0, ?4, ?5)
        "#,
        params![
            task_id.as_uuid().to_string(),
            enum_name(&kind)?,
            fragment_index,
            enum_name(&DownloadSegmentState::Waiting)?,
            now_unix_ms(),
        ],
    )?;
    Ok(())
}

fn dash_track_kind_name(kind: DashTrackKind) -> &'static str {
    match kind {
        DashTrackKind::Video => "video",
        DashTrackKind::Audio => "audio",
    }
}

fn append_to_queue_row(
    connection: &Connection,
    queue_id: &str,
    task_id: DownloadTaskId,
    is_eligible: bool,
) -> Result<(), DownloadStorageError> {
    connection.execute(
        r#"
        INSERT INTO download_queues(queue_id, name, position, max_concurrent_tasks, is_running)
        VALUES (?1, ?2, 0, 3, 1)
        ON CONFLICT(queue_id) DO NOTHING
        "#,
        params![queue_id, queue_id],
    )?;
    connection.execute(
        r#"
        INSERT INTO download_queue_items(queue_id, task_id, position, is_eligible)
        SELECT ?1, ?2, COALESCE(MAX(position) + 1, 0), ?3
        FROM download_queue_items
        WHERE queue_id = ?1
        ON CONFLICT(task_id) DO NOTHING
        "#,
        params![queue_id, task_id.as_uuid().to_string(), is_eligible],
    )?;
    connection.execute(
        "UPDATE download_tasks SET queue_id = ?2 WHERE task_id = ?1",
        params![task_id.as_uuid().to_string(), queue_id],
    )?;
    Ok(())
}

fn save_segment_row(
    connection: &Connection,
    task_id: DownloadTaskId,
    segment: &DownloadSegment,
) -> Result<(), DownloadStorageError> {
    connection.execute(
        r#"
        INSERT INTO download_segments (
            task_id,
            segment_index,
            start_byte,
            end_byte,
            downloaded_bytes,
            state,
            updated_at_unix_ms
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            task_id.as_uuid().to_string(),
            i64::from(segment.index()),
            i64::try_from(segment.start_byte())
                .map_err(|_| DownloadStorageError::IntegerOverflow)?,
            i64::try_from(segment.end_byte()).map_err(|_| DownloadStorageError::IntegerOverflow)?,
            i64::try_from(segment.downloaded_bytes())
                .map_err(|_| DownloadStorageError::IntegerOverflow)?,
            enum_name(&segment.state)?,
            now_unix_ms(),
        ],
    )?;
    Ok(())
}

fn decode_segment_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DownloadSegment> {
    let index = u32::try_from(row.get::<_, i64>(0)?).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            0,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })?;
    let start_byte = nonnegative_u64(row.get(1)?, 1)?;
    let end_byte = nonnegative_u64(row.get(2)?, 2)?;
    let downloaded_bytes = nonnegative_u64(row.get(3)?, 3)?;
    let state = enum_from_name::<DownloadSegmentState>(&row.get::<_, String>(4)?, 4)?;
    DownloadSegment::new(index, start_byte, end_byte, downloaded_bytes, state).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(error))
    })
}

fn save_transfer_state_row(
    connection: &Connection,
    task_id: DownloadTaskId,
    state: &StoredTransferState,
) -> Result<(), DownloadStorageError> {
    let (error_code, error_message, error_retryable) = match state.error.as_ref() {
        Some(error) => (
            Some(enum_name(&error.code)?),
            Some(error.message.as_str()),
            error.retryable,
        ),
        None => (None, None, false),
    };
    connection.execute(
        r#"
        UPDATE download_tasks
        SET
            final_url = ?2,
            total_bytes = ?3,
            downloaded_bytes = ?4,
            supports_ranges = ?5,
            etag = ?6,
            last_modified = ?7,
            error_code = ?8,
            error_message = ?9,
            error_retryable = ?10,
            request_json = json_set(request_json, '$.probe', json(?11))
        WHERE task_id = ?1
        "#,
        params![
            task_id.as_uuid().to_string(),
            state.probe.as_ref().map(|probe| probe.final_url.as_str()),
            state
                .probe
                .as_ref()
                .and_then(|probe| probe.total_bytes)
                .map(i64::try_from)
                .transpose()
                .map_err(|_| DownloadStorageError::IntegerOverflow)?,
            i64::try_from(state.downloaded_bytes)
                .map_err(|_| DownloadStorageError::IntegerOverflow)?,
            state
                .probe
                .as_ref()
                .map(|probe| i64::from(probe.supports_ranges)),
            state
                .probe
                .as_ref()
                .and_then(|probe| probe.validator.etag.as_deref()),
            state
                .probe
                .as_ref()
                .and_then(|probe| probe.validator.last_modified.as_deref()),
            error_code,
            error_message,
            error_retryable,
            state
                .probe
                .as_ref()
                .map(serde_json::to_string)
                .transpose()?,
        ],
    )?;
    if let Some(path) = state.temporary_path.as_ref() {
        save_artifact(
            connection,
            task_id,
            "temporary",
            path,
            if state.final_committed {
                "removed"
            } else if state.prepared_size_bytes.is_some() {
                "prepared"
            } else {
                "pending"
            },
            state.prepared_size_bytes.or(Some(state.downloaded_bytes)),
            None,
        )?;
    }
    if let Some(path) = state.final_path.as_ref() {
        save_artifact(
            connection,
            task_id,
            "final",
            path,
            if state.final_committed {
                "committed"
            } else {
                "pending"
            },
            state.final_committed.then_some(state.downloaded_bytes),
            state.final_committed.then_some(now_unix_ms()),
        )?;
    }
    Ok(())
}

fn save_artifact(
    connection: &Connection,
    task_id: DownloadTaskId,
    kind: &str,
    path: &Path,
    state: &str,
    size_bytes: Option<u64>,
    committed_at_unix_ms: Option<i64>,
) -> Result<(), DownloadStorageError> {
    let size_bytes = size_bytes
        .map(i64::try_from)
        .transpose()
        .map_err(|_| DownloadStorageError::IntegerOverflow)?;
    connection.execute(
        r#"
        INSERT INTO download_artifacts (
            artifact_id,
            task_id,
            kind,
            path,
            state,
            size_bytes,
            created_at_unix_ms,
            committed_at_unix_ms
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        ON CONFLICT(artifact_id) DO UPDATE SET
            path = excluded.path,
            state = excluded.state,
            size_bytes = excluded.size_bytes,
            committed_at_unix_ms = excluded.committed_at_unix_ms
        "#,
        params![
            format!("{}:{kind}", task_id.as_uuid()),
            task_id.as_uuid().to_string(),
            kind,
            path.to_string_lossy(),
            state,
            size_bytes,
            now_unix_ms(),
            committed_at_unix_ms,
        ],
    )?;
    Ok(())
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            i64::try_from(duration.as_millis()).unwrap_or(i64::MAX)
        })
}

fn reject_newer_schema(connection: &Connection) -> Result<(), DownloadStorageError> {
    let current: u32 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if current > DOWNLOAD_SCHEMA_VERSION {
        return Err(DownloadStorageError::UnsupportedSchema {
            current,
            supported: DOWNLOAD_SCHEMA_VERSION,
        });
    }
    Ok(())
}

fn decode_task_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DownloadTask> {
    let raw_id: String = row.get(0)?;
    let request_json: String = row.get(1)?;
    let raw_origin: String = row.get(2)?;
    let raw_status: String = row.get(3)?;

    let id = Uuid::parse_str(&raw_id).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let stored = serde_json::from_str::<StoredResolvedRequest>(&request_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let origin = enum_from_name::<RequestOrigin>(&raw_origin, 2)?;
    let status = enum_from_name::<DownloadStatus>(&raw_status, 3)?;

    let mut task = DownloadTask::restore(
        DownloadTaskId::from_uuid(id),
        stored.request,
        origin,
        status,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    )
    .map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let bytes = optional_nonnegative_u64(row.get(7)?, 7)?;
    let duration = optional_nonnegative_u64(row.get(8)?, 8)?;
    let statistics = match (bytes, duration) {
        (Some(transferred_bytes), Some(active_download_ms)) => {
            Some(crate::DownloadTransferStatistics {
                transferred_bytes,
                active_download_ms,
            })
        }
        (None, None) => None,
        _ => return Err(rusqlite::Error::InvalidQuery),
    };
    task.set_transfer_statistics(statistics);
    task.set_resolution(stored.resolution);
    task.set_completion_intention(stored.completion_intention);
    task.set_completion_attempt(stored.completion_attempt);
    Ok(task)
}

fn enum_name<T: Serialize>(value: &T) -> Result<String, DownloadStorageError> {
    let value = serde_json::to_value(value)?;
    value
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or(DownloadStorageError::InvalidEnumEncoding)
}

fn enum_from_name<T: DeserializeOwned>(value: &str, column: usize) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn json_from_text<T: DeserializeOwned>(value: &str, column: usize) -> rusqlite::Result<T> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn nonnegative_u64(value: i64, column: usize) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            column,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}

fn optional_nonnegative_u64(value: Option<i64>, column: usize) -> rusqlite::Result<Option<u64>> {
    value
        .map(|value| nonnegative_u64(value, column))
        .transpose()
}

#[derive(Debug, Error)]
pub enum DownloadStorageError {
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("download database schema {current} is newer than supported schema {supported}")]
    UnsupportedSchema { current: u32, supported: u32 },
    #[error("download database lock is poisoned")]
    LockPoisoned,
    #[error("enum did not serialize to a string")]
    InvalidEnumEncoding,
    #[error("download integer does not fit SQLite INTEGER")]
    IntegerOverflow,
    #[error("idempotency key conflicts with an existing request: {key}")]
    IdempotencyConflict { key: Uuid },
    #[error("browser handoff conflicts with an existing request: {handoff_id}")]
    BrowserHandoffConflict { handoff_id: Uuid },
    #[error("browser handoff changed state during a durable mutation: {handoff_id}")]
    BrowserHandoffStateConflict { handoff_id: Uuid },
    #[error("browser handoff task identity does not match: {handoff_id}")]
    BrowserHandoffTaskMismatch { handoff_id: Uuid },
    #[error("download task does not exist: {0:?}")]
    TaskNotFound(DownloadTaskId),
    #[error("download queue does not exist: {0}")]
    QueueNotFound(String),
    #[error("download segment {segment_index} does not exist for task {task_id:?}")]
    SegmentNotFound {
        task_id: DownloadTaskId,
        segment_index: u32,
    },
    #[error("DASH {track_kind:?} fragment {segment_index:?} does not exist for task {task_id:?}")]
    DashFragmentNotFound {
        task_id: DownloadTaskId,
        track_kind: &'static str,
        segment_index: Option<u32>,
    },
    #[error("stored download transfer plan is invalid: {0}")]
    InvalidTransferPlan(crate::TransferPlanError),
    #[error("stored download task id is invalid: {0}")]
    InvalidTaskId(String),
}

const SCHEMA_V1: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE download_tasks (
    task_id TEXT PRIMARY KEY NOT NULL,
    request_json TEXT NOT NULL,
    origin TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    completed_at_unix_ms INTEGER,
    final_url TEXT,
    total_bytes INTEGER,
    downloaded_bytes INTEGER NOT NULL DEFAULT 0 CHECK(downloaded_bytes >= 0),
    supports_ranges INTEGER,
    etag TEXT,
    last_modified TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0 CHECK(retry_count >= 0),
    error_code TEXT,
    error_message TEXT,
    queue_id TEXT,
    transfer_plan_json TEXT
);

CREATE INDEX download_tasks_status_updated_idx
    ON download_tasks(status, updated_at_unix_ms DESC);
CREATE INDEX download_tasks_queue_created_idx
    ON download_tasks(queue_id, created_at_unix_ms);

CREATE TABLE download_segments (
    task_id TEXT NOT NULL REFERENCES download_tasks(task_id) ON DELETE CASCADE,
    segment_index INTEGER NOT NULL CHECK(segment_index >= 0),
    start_byte INTEGER NOT NULL CHECK(start_byte >= 0),
    end_byte INTEGER NOT NULL CHECK(end_byte >= start_byte),
    downloaded_bytes INTEGER NOT NULL DEFAULT 0 CHECK(downloaded_bytes >= 0),
    state TEXT NOT NULL,
    epoch INTEGER NOT NULL DEFAULT 0,
    updated_at_unix_ms INTEGER NOT NULL,
    PRIMARY KEY(task_id, segment_index)
);

CREATE TABLE download_queues (
    queue_id TEXT PRIMARY KEY NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    max_concurrent_tasks INTEGER,
    connection_limit INTEGER,
    speed_limit_bytes_per_second INTEGER,
    schedule_json TEXT,
    is_running INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE download_artifacts (
    artifact_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL REFERENCES download_tasks(task_id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    path TEXT NOT NULL,
    state TEXT NOT NULL,
    size_bytes INTEGER,
    created_at_unix_ms INTEGER NOT NULL,
    committed_at_unix_ms INTEGER
);

CREATE INDEX download_artifacts_task_idx ON download_artifacts(task_id);

CREATE TABLE download_settings (
    key TEXT PRIMARY KEY NOT NULL,
    value_json TEXT NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);

PRAGMA user_version = 1;
COMMIT;
"#;

const SCHEMA_V2: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE IF NOT EXISTS download_queue_items (
    queue_id TEXT NOT NULL REFERENCES download_queues(queue_id) ON DELETE CASCADE,
    task_id TEXT NOT NULL UNIQUE REFERENCES download_tasks(task_id) ON DELETE CASCADE,
    position INTEGER NOT NULL CHECK(position >= 0),
    is_eligible INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY(queue_id, task_id)
);

CREATE UNIQUE INDEX IF NOT EXISTS download_queue_items_position_idx
    ON download_queue_items(queue_id, position);

INSERT INTO download_queues(queue_id, name, position, max_concurrent_tasks, is_running)
VALUES ('default', '默认队列', 0, 3, 1)
ON CONFLICT(queue_id) DO NOTHING;

INSERT OR IGNORE INTO download_queue_items(queue_id, task_id, position, is_eligible)
SELECT 'default', task_id, ROW_NUMBER() OVER (
    ORDER BY created_at_unix_ms ASC, task_id ASC
) - 1, 0
FROM download_tasks;

PRAGMA user_version = 2;
COMMIT;
"#;

const SCHEMA_V3: &str = r#"
BEGIN IMMEDIATE;

ALTER TABLE download_queue_items
    ADD COLUMN is_user_paused INTEGER NOT NULL DEFAULT 0;

PRAGMA user_version = 3;
COMMIT;
"#;

const SCHEMA_V4: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE download_task_timeline (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL REFERENCES download_tasks(task_id) ON DELETE CASCADE,
    occurred_at_unix_ms INTEGER NOT NULL,
    event_kind TEXT NOT NULL,
    status TEXT,
    segment_count INTEGER,
    CHECK (
        (event_kind = 'created' AND status IS NULL AND segment_count IS NULL)
        OR (event_kind = 'status' AND status IS NOT NULL AND segment_count IS NULL)
        OR (event_kind = 'transfer_plan' AND status IS NULL AND segment_count >= 0)
    )
);

CREATE INDEX download_task_timeline_task_sequence_idx
    ON download_task_timeline(task_id, sequence DESC);

INSERT INTO download_task_timeline(
    task_id, occurred_at_unix_ms, event_kind, status, segment_count
)
SELECT task_id, created_at_unix_ms, 'created', NULL, NULL
FROM download_tasks;

CREATE TRIGGER download_task_timeline_created
AFTER INSERT ON download_tasks
BEGIN
    INSERT INTO download_task_timeline(
        task_id, occurred_at_unix_ms, event_kind, status, segment_count
    )
    VALUES (NEW.task_id, NEW.created_at_unix_ms, 'created', NULL, NULL);
END;

CREATE TRIGGER download_task_timeline_status_changed
AFTER UPDATE OF status ON download_tasks
WHEN OLD.status <> NEW.status
BEGIN
    INSERT INTO download_task_timeline(
        task_id, occurred_at_unix_ms, event_kind, status, segment_count
    )
    VALUES (NEW.task_id, NEW.updated_at_unix_ms, 'status', NEW.status, NULL);
END;

CREATE TRIGGER download_task_timeline_bounded
AFTER INSERT ON download_task_timeline
BEGIN
    DELETE FROM download_task_timeline
    WHERE task_id = NEW.task_id
      AND sequence NOT IN (
          SELECT sequence
          FROM download_task_timeline
          WHERE task_id = NEW.task_id
          ORDER BY sequence DESC
          LIMIT 300
      );
END;

PRAGMA user_version = 4;
COMMIT;
"#;

const SCHEMA_V5: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE download_browser_handoffs (
    handoff_id TEXT PRIMARY KEY NOT NULL,
    task_id TEXT NOT NULL UNIQUE REFERENCES download_tasks(task_id) ON DELETE CASCADE,
    accepted_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX download_browser_handoffs_accepted_idx
    ON download_browser_handoffs(accepted_at_unix_ms DESC);

PRAGMA user_version = 5;
COMMIT;
"#;

const SCHEMA_V6: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE download_hls_plans (
    task_id TEXT PRIMARY KEY NOT NULL REFERENCES download_tasks(task_id) ON DELETE CASCADE,
    final_url TEXT NOT NULL,
    media_sequence INTEGER NOT NULL CHECK(media_sequence >= 0),
    updated_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE download_hls_segments (
    task_id TEXT NOT NULL REFERENCES download_hls_plans(task_id) ON DELETE CASCADE,
    segment_index INTEGER NOT NULL CHECK(segment_index >= 0),
    sequence_number INTEGER NOT NULL CHECK(sequence_number >= 0),
    url TEXT NOT NULL,
    duration_millis INTEGER NOT NULL CHECK(duration_millis >= 0),
    downloaded_bytes INTEGER NOT NULL DEFAULT 0 CHECK(downloaded_bytes >= 0),
    state TEXT NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    PRIMARY KEY(task_id, segment_index)
);

CREATE INDEX download_hls_segments_task_state_idx
    ON download_hls_segments(task_id, state, segment_index);

PRAGMA user_version = 6;
COMMIT;
"#;

const SCHEMA_V7: &str = r#"
BEGIN IMMEDIATE;

ALTER TABLE download_hls_segments
    ADD COLUMN encryption_json TEXT NOT NULL DEFAULT '"None"';

PRAGMA user_version = 7;
COMMIT;
"#;

const SCHEMA_V8: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE download_dash_plans (
    task_id TEXT PRIMARY KEY NOT NULL REFERENCES download_tasks(task_id) ON DELETE CASCADE,
    manifest_json TEXT NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL
);

CREATE TABLE download_dash_fragments (
    task_id TEXT NOT NULL REFERENCES download_dash_plans(task_id) ON DELETE CASCADE,
    track_kind TEXT NOT NULL,
    fragment_index INTEGER NOT NULL CHECK(fragment_index >= -1),
    downloaded_bytes INTEGER NOT NULL DEFAULT 0 CHECK(downloaded_bytes >= 0),
    state TEXT NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    PRIMARY KEY(task_id, track_kind, fragment_index)
);

CREATE INDEX download_dash_fragments_task_state_idx
    ON download_dash_fragments(task_id, state, track_kind, fragment_index);

PRAGMA user_version = 8;
COMMIT;
"#;

const SCHEMA_V9: &str = r#"
BEGIN IMMEDIATE;

ALTER TABLE download_browser_handoffs
    ADD COLUMN request_fingerprint TEXT NOT NULL DEFAULT '';

PRAGMA user_version = 9;
COMMIT;
"#;

const SCHEMA_V10: &str = r#"
    BEGIN IMMEDIATE;

    CREATE TABLE download_submission_receipts (
        origin TEXT NOT NULL,
        idempotency_key TEXT NOT NULL,
        request_fingerprint TEXT NOT NULL,
        task_id TEXT NOT NULL UNIQUE REFERENCES download_tasks(task_id) ON DELETE CASCADE,
        accepted_at_unix_ms INTEGER NOT NULL,
        PRIMARY KEY(origin, idempotency_key)
    );

    CREATE INDEX download_submission_receipts_task_idx
        ON download_submission_receipts(task_id);

    PRAGMA user_version = 10;
    COMMIT;
    "#;

const SCHEMA_V11: &str = r#"
BEGIN IMMEDIATE;

ALTER TABLE download_tasks
    ADD COLUMN error_retryable INTEGER NOT NULL DEFAULT 0;

PRAGMA user_version = 11;
COMMIT;
"#;

const SCHEMA_V12: &str = r#"
BEGIN IMMEDIATE;

CREATE TABLE download_redownload_operations (
    old_task_id TEXT PRIMARY KEY NOT NULL REFERENCES download_tasks(task_id) ON DELETE CASCADE,
    replacement_task_id TEXT NOT NULL UNIQUE REFERENCES download_tasks(task_id) ON DELETE CASCADE,
    created_at_unix_ms INTEGER NOT NULL
);

CREATE INDEX download_redownload_operations_replacement_idx
    ON download_redownload_operations(replacement_task_id);

PRAGMA user_version = 12;
COMMIT;
"#;

const SCHEMA_V13: &str = r#"
BEGIN IMMEDIATE;
ALTER TABLE download_tasks ADD COLUMN measured_payload_bytes INTEGER
    CHECK(measured_payload_bytes IS NULL OR measured_payload_bytes >= 0);
ALTER TABLE download_tasks ADD COLUMN active_download_ms INTEGER
    CHECK(active_download_ms IS NULL OR active_download_ms >= 0)
    CHECK((measured_payload_bytes IS NULL) = (active_download_ms IS NULL));
PRAGMA user_version = 13;
COMMIT;
"#;

const SCHEMA_V14: &str = r#"
BEGIN IMMEDIATE;
ALTER TABLE download_queues ADD COLUMN completion_intention_json TEXT NOT NULL DEFAULT '{}';
PRAGMA user_version = 14;
COMMIT;
"#;

const SCHEMA_V15: &str = r#"
BEGIN IMMEDIATE;
CREATE TABLE download_controlled_browser_handoffs (
    handoff_id TEXT PRIMARY KEY NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('pending', 'held', 'committed', 'aborted', 'expired')),
    task_id TEXT UNIQUE,
    request_fingerprint TEXT,
    start_now INTEGER,
    deadline_unix_ms INTEGER,
    created_at_unix_ms INTEGER NOT NULL,
    updated_at_unix_ms INTEGER NOT NULL,
    CHECK((state = 'pending') = (deadline_unix_ms IS NOT NULL))
);
CREATE INDEX download_controlled_browser_handoffs_state_updated_idx
    ON download_controlled_browser_handoffs(state, updated_at_unix_ms DESC);
PRAGMA user_version = 15;
COMMIT;
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hls::{HlsMediaPlaylist, HlsMediaSegment, HlsSegmentEncryption};
    use crate::{DownloadFault, DownloadProbe, DownloadRequest, DownloadStatus};

    fn save_legacy_task_row(connection: &Connection, task: &DownloadTask) {
        connection.execute(
            "INSERT INTO download_tasks(task_id, request_json, origin, status, created_at_unix_ms, updated_at_unix_ms, completed_at_unix_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![task.id().as_uuid().to_string(), serde_json::to_string(task.request()).expect("legacy request"),
                enum_name(&task.origin()).expect("legacy origin"), task.status().storage_name(),
                task.created_at_unix_ms(), task.updated_at_unix_ms(), task.completed_at_unix_ms()],
        ).expect("seed actual old schema without new statistics columns");
    }

    #[test]
    fn average_transaction_failure_rolls_back_statistics_with_artifact_and_task() {
        let store = SqliteDownloadStore::open_in_memory().expect("statistics rollback store");
        let mut task = sample_task("rollback-average.bin");
        let initial = task.transfer_statistics();
        let transfer = StoredTransferState {
            temporary_path: Some(PathBuf::from("rollback-average.part")),
            ..StoredTransferState::default()
        };
        store
            .save_task_with_transfer(&task, &transfer)
            .expect("initial durable pair");
        task.set_transfer_statistics(Some(crate::DownloadTransferStatistics {
            transferred_bytes: 100,
            active_download_ms: 1_000,
        }));
        store
            .inject_artifact_update_failure()
            .expect("abort artifact transaction");
        assert!(store.save_task_with_transfer(&task, &transfer).is_err());
        assert_eq!(
            store
                .load_task(task.id())
                .expect("read last good task")
                .expect("task")
                .transfer_statistics(),
            initial
        );
    }

    #[test]
    fn average_interrupted_crash_window_invalidates_unpaired_statistics() {
        let store = SqliteDownloadStore::open_in_memory().expect("crash-window statistics store");
        let mut task = sample_task("crash-average.bin");
        for status in [
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
        ] {
            task.transition_to(status, task.created_at_unix_ms() + 1)
                .expect("active crash stage");
        }
        task.set_transfer_statistics(Some(crate::DownloadTransferStatistics {
            transferred_bytes: 100,
            active_download_ms: 1_000,
        }));
        store
            .save_task(&task)
            .expect("persist last observed pair before crash");
        assert_eq!(
            store
                .recover_interrupted_tasks(task.created_at_unix_ms() + 2)
                .expect("recover crash"),
            1
        );
        let restored = store
            .load_task(task.id())
            .expect("read recovered task")
            .expect("task");
        assert_eq!(restored.status(), DownloadStatus::Paused);
        assert_eq!(
            restored.transfer_statistics(),
            None,
            "checkpoint may be ahead of measured time"
        );
    }

    #[test]
    fn average_schema_twelve_migration_retains_unknown_legacy_duration() {
        let connection = Connection::open_in_memory().expect("v12 fixture");
        for schema in [
            SCHEMA_V1, SCHEMA_V2, SCHEMA_V3, SCHEMA_V4, SCHEMA_V5, SCHEMA_V6, SCHEMA_V7, SCHEMA_V8,
            SCHEMA_V9, SCHEMA_V10, SCHEMA_V11, SCHEMA_V12,
        ] {
            connection
                .execute_batch(schema)
                .expect("apply actual released migration");
        }
        let task = sample_task("v12-average.bin");
        save_legacy_task_row(&connection, &task);
        let store = SqliteDownloadStore::initialize(connection).expect("v13 migration");
        assert_eq!(
            store.schema_version().expect("migrated version"),
            DOWNLOAD_SCHEMA_VERSION
        );
        assert_eq!(
            store
                .load_task(task.id())
                .expect("legacy row")
                .expect("task")
                .transfer_statistics(),
            None
        );
    }

    #[test]
    fn empty_database_file_is_quarantined_before_recreation() {
        let directory = tempfile::tempdir().expect("create database directory");
        let database = directory.path().join("downloads.sqlite3");
        std::fs::File::create(&database).expect("create empty database placeholder");

        let store = SqliteDownloadStore::open(&database).expect("recreate empty database");
        assert_eq!(
            store.schema_version().expect("read schema version"),
            DOWNLOAD_SCHEMA_VERSION
        );
        assert!(
            std::fs::read_dir(directory.path())
                .expect("read database directory")
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("downloads.sqlite3.empty-"))
        );
    }

    #[test]
    fn queue_membership_and_order_survive_store_reopen() {
        let directory = tempfile::tempdir().expect("create database directory");
        let database = directory.path().join("downloads.db");
        let first = sample_task("first.bin");
        let second = sample_task("second.bin");

        {
            let store = SqliteDownloadStore::open(&database).expect("open store");
            store.save_task(&first).expect("save first task");
            store.save_task(&second).expect("save second task");
            store
                .replace_queue_order("default", &[second.id(), first.id()])
                .expect("save queue order");
        }

        let reopened = SqliteDownloadStore::open(&database).expect("reopen store");
        assert_eq!(
            reopened
                .load_queue_order("default")
                .expect("load queue order"),
            vec![second.id(), first.id()]
        );
    }

    #[test]
    fn hls_transfer_plan_and_segment_checkpoint_survive_store_reopen() {
        let directory = tempfile::tempdir().expect("create HLS database directory");
        let database = directory.path().join("downloads.db");
        let task = sample_task("movie.ts");
        let playlist = HlsMediaPlaylist {
            final_url: "https://media.example.test/high/index.m3u8".into(),
            media_sequence: 42,
            segments: vec![
                HlsMediaSegment {
                    index: 0,
                    sequence: 42,
                    url: "https://media.example.test/high/42.ts".into(),
                    duration_millis: 4_000,
                    encryption: HlsSegmentEncryption::Aes128 {
                        key_url: "https://media.example.test/high/key.bin".into(),
                        key_slot: 0,
                        explicit_iv: None,
                    },
                },
                HlsMediaSegment {
                    index: 1,
                    sequence: 43,
                    url: "https://media.example.test/high/43.ts".into(),
                    duration_millis: 2_000,
                    encryption: HlsSegmentEncryption::None,
                },
            ],
        };

        {
            let store = SqliteDownloadStore::open(&database).expect("open HLS store");
            store.save_task(&task).expect("save HLS task");
            store
                .save_hls_transfer_plan(task.id(), &playlist)
                .expect("save HLS plan");
            store
                .save_hls_segment_checkpoint(task.id(), 0, DownloadSegmentState::Completed, 12)
                .expect("save HLS segment checkpoint");
            let mut refreshed = playlist.clone();
            refreshed.final_url = "https://cdn.example.test/high/index.m3u8?token=fresh".into();
            refreshed.segments[0].url = "https://cdn.example.test/high/42.ts?token=fresh".into();
            refreshed.segments[0].encryption = HlsSegmentEncryption::Aes128 {
                key_url: "https://cdn.example.test/high/key.bin?token=fresh".into(),
                key_slot: 0,
                explicit_iv: None,
            };
            refreshed.segments[1].url = "https://cdn.example.test/high/43.ts?token=fresh".into();
            store
                .refresh_hls_transfer_plan_urls(task.id(), &refreshed)
                .expect("refresh signed HLS URLs");
        }

        let reopened = SqliteDownloadStore::open(&database).expect("reopen HLS store");
        let restored = reopened
            .load_hls_transfer_plan(task.id())
            .expect("load HLS plan")
            .expect("HLS plan exists");
        assert_eq!(
            restored.final_url,
            "https://cdn.example.test/high/index.m3u8?token=fresh"
        );
        assert_eq!(restored.media_sequence, 42);
        assert_eq!(restored.segments.len(), 2);
        assert_eq!(restored.segments[0].state, DownloadSegmentState::Completed);
        assert_eq!(restored.segments[0].downloaded_bytes, 12);
        assert_eq!(
            restored.segments[0].media.url,
            "https://cdn.example.test/high/42.ts?token=fresh"
        );
        assert_eq!(restored.segments[1].state, DownloadSegmentState::Waiting);
        assert_eq!(
            restored.segments[0].media.encryption,
            HlsSegmentEncryption::Aes128 {
                key_url: "https://cdn.example.test/high/key.bin?token=fresh".into(),
                key_slot: 0,
                explicit_iv: None,
            }
        );
    }

    #[test]
    fn dash_transfer_plan_and_fragment_checkpoint_survive_store_reopen() {
        use crate::dash::{DashManifest, DashSegment, DashTrack, DashTrackKind};

        let directory = tempfile::tempdir().expect("create DASH database directory");
        let database = directory.path().join("downloads.db");
        let task = sample_task("movie.mp4");
        let manifest = DashManifest {
            final_url: "https://media.example.test/manifest.mpd".into(),
            tracks: vec![DashTrack {
                kind: DashTrackKind::Video,
                representation_id: "v1080".into(),
                bandwidth: 4_200_000,
                resolution: Some((1920, 1080)),
                codecs: "avc1.640028".into(),
                mime_type: "video/mp4".into(),
                language: None,
                initialization_url: Some("https://media.example.test/v1080/init.mp4".into()),
                segments: vec![DashSegment {
                    index: 0,
                    number: 1,
                    time: Some(0),
                    url: "https://media.example.test/v1080/0.m4s".into(),
                    duration_millis: 2_000,
                    duration_units: 2_000,
                    timescale: 1_000,
                }],
            }],
        };

        {
            let store = SqliteDownloadStore::open(&database).expect("open DASH store");
            store.save_task(&task).expect("save DASH task");
            store
                .save_dash_transfer_plan(task.id(), &manifest)
                .expect("save DASH plan");
            store
                .save_dash_fragment_checkpoint(
                    task.id(),
                    DashTrackKind::Video,
                    Some(0),
                    DownloadSegmentState::Completed,
                    3,
                )
                .expect("save DASH media checkpoint");
        }

        let reopened = SqliteDownloadStore::open(&database).expect("reopen DASH store");
        let restored = reopened
            .load_dash_transfer_plan(task.id())
            .expect("load DASH plan")
            .expect("DASH plan exists");
        assert_eq!(restored.manifest, manifest);
        assert_eq!(
            restored.fragments.len(),
            2,
            "init and media checkpoints persist"
        );
        let media = restored
            .fragments
            .iter()
            .find(|fragment| fragment.segment_index == Some(0))
            .expect("media checkpoint exists");
        assert_eq!(media.state, DownloadSegmentState::Completed);
        assert_eq!(media.downloaded_bytes, 3);
    }

    #[test]
    fn task_timeline_is_durable_newest_first_and_bounded_to_three_hundred_entries() {
        let directory = tempfile::tempdir().expect("create database directory");
        let database = directory.path().join("downloads.db");
        let task = sample_task("timeline.bin");
        {
            let store = SqliteDownloadStore::open(&database).expect("open store");
            assert_eq!(
                store.schema_version().expect("schema version"),
                DOWNLOAD_SCHEMA_VERSION
            );
            store.save_task(&task).expect("save task");
            let connection = store.lock().expect("lock store");
            for segment_count in 0..305_i64 {
                connection
                    .execute(
                        r#"
                        INSERT INTO download_task_timeline(
                            task_id, occurred_at_unix_ms, event_kind, status, segment_count
                        )
                        VALUES (?1, ?2, 'transfer_plan', NULL, ?3)
                        "#,
                        params![
                            task.id().as_uuid().to_string(),
                            10_000 + segment_count,
                            segment_count
                        ],
                    )
                    .expect("append timeline entry");
            }
        }

        let reopened = SqliteDownloadStore::open(&database).expect("reopen store");
        let timeline = reopened
            .load_task_timeline(task.id(), 400)
            .expect("load timeline");
        assert_eq!(timeline.len(), 300);
        assert!(matches!(
            timeline.first().map(|entry| &entry.event),
            Some(DownloadTaskTimelineEvent::TransferPlanChanged { segment_count: 304 })
        ));
        assert!(matches!(
            timeline.last().map(|entry| &entry.event),
            Some(DownloadTaskTimelineEvent::TransferPlanChanged { segment_count: 5 })
        ));
    }

    #[test]
    fn schema_three_upgrade_seeds_a_safe_created_entry_for_existing_tasks() {
        let connection = Connection::open_in_memory().expect("open legacy database");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("enable foreign keys");
        connection
            .execute_batch(SCHEMA_V1)
            .expect("create schema one");
        connection
            .execute_batch(SCHEMA_V2)
            .expect("create schema two");
        connection
            .execute_batch(SCHEMA_V3)
            .expect("create schema three");
        let task = sample_task("existing.bin");
        save_legacy_task_row(&connection, &task);

        let store = SqliteDownloadStore::initialize(connection).expect("upgrade store");
        assert_eq!(
            store.schema_version().expect("schema version"),
            DOWNLOAD_SCHEMA_VERSION
        );
        let timeline = store
            .load_task_timeline(task.id(), 10)
            .expect("load seeded timeline");
        assert_eq!(timeline.len(), 1);
        assert!(matches!(
            timeline[0].event,
            DownloadTaskTimelineEvent::TaskCreated
        ));
    }

    #[test]
    fn schema_six_hls_rows_upgrade_as_unencrypted_without_losing_checkpoints() {
        let connection = Connection::open_in_memory().expect("open legacy HLS database");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("enable foreign keys");
        for schema in [
            SCHEMA_V1, SCHEMA_V2, SCHEMA_V3, SCHEMA_V4, SCHEMA_V5, SCHEMA_V6,
        ] {
            connection
                .execute_batch(schema)
                .expect("apply legacy schema");
        }
        let task = sample_task("legacy-hls.ts");
        save_legacy_task_row(&connection, &task);
        connection
            .execute(
                r#"
                INSERT INTO download_hls_plans(task_id, final_url, media_sequence, updated_at_unix_ms)
                VALUES (?1, 'https://media.example.test/index.m3u8', 9, 1)
                "#,
                [task.id().as_uuid().to_string()],
            )
            .expect("save legacy HLS plan");
        connection
            .execute(
                r#"
                INSERT INTO download_hls_segments(
                    task_id, segment_index, sequence_number, url, duration_millis,
                    downloaded_bytes, state, updated_at_unix_ms
                )
                VALUES (?1, 0, 9, 'https://media.example.test/9.ts', 1000, 12, 'completed', 1)
                "#,
                [task.id().as_uuid().to_string()],
            )
            .expect("save legacy HLS segment");

        let store = SqliteDownloadStore::initialize(connection).expect("upgrade HLS store");
        let restored = store
            .load_hls_transfer_plan(task.id())
            .expect("load upgraded HLS plan")
            .expect("upgraded HLS plan exists");

        assert_eq!(
            store.schema_version().expect("schema version"),
            DOWNLOAD_SCHEMA_VERSION
        );
        assert_eq!(
            restored.segments[0].media.encryption,
            HlsSegmentEncryption::None
        );
        assert_eq!(restored.segments[0].downloaded_bytes, 12);
        assert_eq!(restored.segments[0].state, DownloadSegmentState::Completed);
    }

    #[test]
    fn queue_membership_failure_rolls_back_the_new_task_acceptance() {
        let store = SqliteDownloadStore::open_in_memory().expect("open store");
        store
            .lock()
            .expect("lock store")
            .execute_batch(
                r#"
                CREATE TRIGGER reject_queue_membership
                BEFORE INSERT ON download_queue_items
                BEGIN
                    SELECT RAISE(ABORT, 'injected queue failure');
                END;
                "#,
            )
            .expect("install failure trigger");
        let task = sample_task("atomic.bin");
        let transfer = StoredTransferState::default();

        assert!(
            store
                .save_task_with_transfer_and_queue(&task, &transfer, "default", true)
                .is_err()
        );
        assert!(
            store
                .load_task(task.id())
                .expect("load rolled back task")
                .is_none(),
            "a rejected submission must not leave a durable orphan task"
        );
    }

    #[test]
    fn browser_handoff_fingerprint_conflict_does_not_create_a_second_task() {
        let store = SqliteDownloadStore::open_in_memory().expect("open store");
        let handoff_id = Uuid::new_v4();
        let first = sample_task("first-browser.bin");
        let second = sample_task("second-browser.bin");
        let transfer = StoredTransferState::default();

        assert_eq!(
            store
                .save_browser_task_with_transfer_and_queue(
                    handoff_id,
                    "fingerprint-a",
                    &first,
                    &transfer,
                    "default",
                    true,
                )
                .expect("save first handoff"),
            BrowserHandoffSave::Inserted(first.id())
        );
        assert!(matches!(
            store.save_browser_task_with_transfer_and_queue(
                handoff_id,
                "fingerprint-b",
                &second,
                &transfer,
                "default",
                true,
            ),
            Err(DownloadStorageError::BrowserHandoffConflict { handoff_id: conflict })
                if conflict == handoff_id
        ));
        assert!(
            store
                .load_task(second.id())
                .expect("load conflicting task")
                .is_none(),
            "a conflicting handoff must not leave a second task"
        );
        assert!(
            store
                .load_task(first.id())
                .expect("load original task")
                .is_some()
        );
    }

    #[test]
    fn handoff_retention_does_not_evict_a_receipt_for_an_existing_task() {
        let store = SqliteDownloadStore::open_in_memory().expect("open store");
        let first_handoff = Uuid::new_v4();
        let second_handoff = Uuid::new_v4();
        let first = sample_task("retained-first.bin");
        let second = sample_task("retained-second.bin");
        let transfer = StoredTransferState::default();
        let first_fingerprint = request_fingerprint(first.request()).expect("fingerprint first");
        let second_fingerprint = request_fingerprint(second.request()).expect("fingerprint second");

        store
            .save_browser_task_with_transfer_and_queue(
                first_handoff,
                &first_fingerprint,
                &first,
                &transfer,
                "default",
                true,
            )
            .expect("save first handoff");
        store
            .lock()
            .expect("lock store")
            .execute(
                "UPDATE download_browser_handoffs SET accepted_at_unix_ms = 0 WHERE handoff_id = ?1",
                [first_handoff.to_string()],
            )
            .expect("age first handoff");
        store
            .save_browser_task_with_transfer_and_queue(
                second_handoff,
                &second_fingerprint,
                &second,
                &transfer,
                "default",
                true,
            )
            .expect("save second handoff");

        assert_eq!(
            store
                .save_browser_task_with_transfer_and_queue(
                    first_handoff,
                    &first_fingerprint,
                    &sample_task("retained-replay.bin"),
                    &transfer,
                    "default",
                    true,
                )
                .expect("replay retained handoff"),
            BrowserHandoffSave::Duplicate(first.id())
        );
    }

    #[test]
    fn legacy_browser_handoff_fingerprint_is_backfilled_from_the_first_replay() {
        let store = SqliteDownloadStore::open_in_memory().expect("open store");
        let handoff_id = Uuid::new_v4();
        let first = sample_task("legacy-browser.bin");
        let transfer = StoredTransferState::default();
        store
            .save_browser_task_with_transfer_and_queue(
                handoff_id, "", &first, &transfer, "default", true,
            )
            .expect("save legacy handoff fixture");

        let fingerprint = request_fingerprint(first.request()).expect("fingerprint request");
        let replay = sample_task("legacy-browser.bin");
        assert_eq!(
            store
                .save_browser_task_with_transfer_and_queue(
                    handoff_id,
                    &fingerprint,
                    &replay,
                    &transfer,
                    "default",
                    true,
                )
                .expect("replay legacy handoff"),
            BrowserHandoffSave::Duplicate(first.id())
        );

        let stored = store
            .browser_handoff(handoff_id)
            .expect("read backfilled handoff")
            .expect("backfilled handoff exists");
        assert_eq!(
            stored.request_fingerprint.as_deref(),
            Some(fingerprint.as_str())
        );
        let different = sample_task("legacy-different.bin");
        let different_fingerprint =
            request_fingerprint(different.request()).expect("fingerprint different request");
        assert!(matches!(
            store.save_browser_task_with_transfer_and_queue(
                handoff_id,
                &different_fingerprint,
                &different,
                &transfer,
                "default",
                true,
            ),
            Err(DownloadStorageError::BrowserHandoffConflict { handoff_id: conflict })
                if conflict == handoff_id
        ));
    }

    #[test]
    fn legacy_browser_handoff_backfill_reports_a_lost_claim() {
        let store = SqliteDownloadStore::open_in_memory().expect("open store");
        let handoff_id = Uuid::new_v4();
        let task = sample_task("legacy-claim.bin");
        let transfer = StoredTransferState::default();
        store
            .save_browser_task_with_transfer_and_queue(
                handoff_id, "", &task, &transfer, "default", true,
            )
            .expect("save legacy handoff fixture");

        assert!(
            store
                .backfill_browser_handoff_fingerprint(handoff_id, "fingerprint-a")
                .expect("claim legacy fingerprint")
        );
        assert!(
            !store
                .backfill_browser_handoff_fingerprint(handoff_id, "fingerprint-b")
                .expect("observe lost legacy fingerprint claim")
        );
        let stored = store
            .browser_handoff(handoff_id)
            .expect("read claimed handoff")
            .expect("claimed handoff exists");
        assert_eq!(stored.request_fingerprint.as_deref(), Some("fingerprint-a"));
    }

    #[test]
    fn idempotent_submission_conflict_does_not_create_a_second_task() {
        let store = SqliteDownloadStore::open_in_memory().expect("open store");
        let first = sample_task("first-idempotent.bin");
        let second = sample_task("second-idempotent.bin");
        let key = Uuid::new_v4();
        let transfer = StoredTransferState::default();

        assert_eq!(
            store
                .save_idempotent_task_with_transfer_and_queue(
                    RequestOrigin::Cli,
                    key,
                    "fingerprint-a",
                    &first,
                    &transfer,
                    QueueAdmission {
                        id: "default",
                        eligible: true,
                    },
                )
                .expect("save first idempotent task"),
            IdempotentSubmissionSave::Inserted(first.id())
        );
        assert_eq!(
            store
                .save_idempotent_task_with_transfer_and_queue(
                    RequestOrigin::Cli,
                    key,
                    "fingerprint-a",
                    &second,
                    &transfer,
                    QueueAdmission {
                        id: "default",
                        eligible: true,
                    },
                )
                .expect("replay idempotent task"),
            IdempotentSubmissionSave::Duplicate(first.id())
        );
        assert!(matches!(
            store.save_idempotent_task_with_transfer_and_queue(
                RequestOrigin::Cli,
                key,
                "fingerprint-b",
                &second,
                &transfer,
                QueueAdmission {
                    id: "default",
                    eligible: true,
                },
            ),
            Err(DownloadStorageError::IdempotencyConflict { key: conflict })
                if conflict == key
        ));
        assert!(
            store
                .load_task(second.id())
                .expect("load conflicting task")
                .is_none()
        );
    }

    #[test]
    fn idempotency_key_is_scoped_by_origin() {
        let store = SqliteDownloadStore::open_in_memory().expect("open store");
        let first = sample_task("origin-cli.bin");
        let second = sample_task("origin-ui.bin");
        let key = Uuid::new_v4();
        let transfer = StoredTransferState::default();

        assert!(matches!(
            store.save_idempotent_task_with_transfer_and_queue(
                RequestOrigin::Cli,
                key,
                "fingerprint",
                &first,
                &transfer,
                QueueAdmission {
                    id: "default",
                    eligible: true,
                },
            ),
            Ok(IdempotentSubmissionSave::Inserted(task_id)) if task_id == first.id()
        ));
        assert!(matches!(
            store.save_idempotent_task_with_transfer_and_queue(
                RequestOrigin::Ui,
                key,
                "fingerprint",
                &second,
                &transfer,
                QueueAdmission {
                    id: "default",
                    eligible: true,
                },
            ),
            Ok(IdempotentSubmissionSave::Inserted(task_id)) if task_id == second.id()
        ));
    }

    #[test]
    fn idempotent_submission_receipt_failure_rolls_back_task_and_queue() {
        let store = SqliteDownloadStore::open_in_memory().expect("open store");
        store
            .lock()
            .expect("lock store")
            .execute_batch(
                r#"
                CREATE TRIGGER reject_submission_receipt
                BEFORE INSERT ON download_submission_receipts
                BEGIN
                    SELECT RAISE(ABORT, 'injected receipt failure');
                END;
                "#,
            )
            .expect("install receipt failure trigger");
        let task = sample_task("receipt-rollback.bin");
        let result = store.save_idempotent_task_with_transfer_and_queue(
            RequestOrigin::Cli,
            Uuid::new_v4(),
            "fingerprint",
            &task,
            &StoredTransferState::default(),
            QueueAdmission {
                id: "default",
                eligible: true,
            },
        );

        assert!(result.is_err());
        assert!(
            store
                .load_task(task.id())
                .expect("load rolled back task")
                .is_none()
        );
    }

    fn sample_task(file_name: &str) -> DownloadTask {
        let directory = std::env::temp_dir();
        DownloadTask::new(
            DownloadRequest {
                source_url: format!("https://example.test/{file_name}"),
                save_directory: directory,
                save_directory_explicit: false,
                file_name: Some(file_name.into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                source_page: None,
                certificate_policy: Default::default(),
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
            },
            RequestOrigin::Cli,
            now_unix_ms(),
        )
    }

    #[test]
    fn task_completion_and_artifact_commit_roll_back_as_one_transaction() {
        let store = SqliteDownloadStore::open_in_memory().expect("open store");
        let directory = tempfile::tempdir().expect("create artifact directory");
        let mut task = DownloadTask::new(
            DownloadRequest {
                source_url: "https://example.test/archive.bin".into(),
                save_directory: directory.path().to_path_buf(),
                save_directory_explicit: false,
                file_name: Some("archive.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                source_page: None,
                certificate_policy: Default::default(),
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                expected_total_bytes: None,
                scheduled_at_unix_ms: None,
                adaptive_segment_policy: None,
                media: None,
                category_id: None,
                category_name: None,
                duplicate_strategy: crate::DownloadDuplicateStrategy::UseDefault,
            },
            RequestOrigin::Internal,
            1_000,
        );
        for status in [
            DownloadStatus::Probing,
            DownloadStatus::Waiting,
            DownloadStatus::Downloading,
            DownloadStatus::Finalizing,
        ] {
            task.transition_to(status, 1_100)
                .expect("advance task to finalizing");
        }
        let mut transfer = StoredTransferState {
            probe: Some(DownloadProbe {
                final_url: task.request().source_url.clone(),
                file_name: Some("archive.bin".into()),
                total_bytes: Some(16),
                supports_ranges: false,
                content_type: Some("application/octet-stream".into()),
                validator: ResourceValidator::default(),
                media: None,
                resolution: None,
                server_observation: None,
            }),
            downloaded_bytes: 16,
            error: None::<DownloadFault>,
            prepared_size_bytes: None,
            temporary_path: Some(directory.path().join(".archive.part")),
            final_path: Some(directory.path().join("archive.bin")),
            final_committed: false,
        };
        store
            .save_task_with_transfer(&task, &transfer)
            .expect("persist finalizing task");
        store
            .lock()
            .expect("lock store")
            .execute_batch(
                r#"
                CREATE TRIGGER reject_artifact_commit
                BEFORE UPDATE ON download_artifacts
                BEGIN
                    SELECT RAISE(ABORT, 'injected artifact commit failure');
                END;
                "#,
            )
            .expect("install failure trigger");

        task.transition_to(DownloadStatus::Completed, 1_200)
            .expect("advance task to completed");
        transfer.final_committed = true;
        assert!(
            store.save_task_with_transfer(&task, &transfer).is_err(),
            "artifact failure must abort the completion transaction"
        );

        let restored_task = store
            .load_task(task.id())
            .expect("load task")
            .expect("task exists");
        let restored_transfer = store.load_transfer_state(task.id()).expect("load transfer");
        assert_eq!(restored_task.status(), DownloadStatus::Finalizing);
        assert!(!restored_transfer.final_committed);
    }
}
