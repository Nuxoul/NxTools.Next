use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use rusqlite::types::Value;
use rusqlite::{Connection, OptionalExtension, params, params_from_iter};
use serde::Serialize;
use serde::de::DeserializeOwned;
use thiserror::Error;
use uuid::Uuid;

use crate::{
    DownloadFault, DownloadProbe, DownloadQueue, DownloadRequest, DownloadSegment,
    DownloadSegmentState, DownloadStatus, DownloadTask, DownloadTaskId,
    INTERRUPTED_DOWNLOAD_STATUSES, RequestOrigin, ResourceValidator, TransferPlan,
};

pub const DOWNLOAD_SCHEMA_VERSION: u32 = 2;
const GLOBAL_SPEED_LIMIT_SETTING: &str = "global_speed_limit_bytes_per_second";

pub struct SqliteDownloadStore {
    connection: Mutex<Connection>,
}

impl SqliteDownloadStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, DownloadStorageError> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(std::time::Duration::from_secs(5))?;
        reject_newer_schema(&connection)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
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
                    completed_at_unix_ms
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
                completed_at_unix_ms
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
        let eligible = {
            let mut statement = transaction.prepare(
                "SELECT task_id FROM download_queue_items WHERE queue_id = ?1 AND is_eligible = 1",
            )?;
            statement
                .query_map([queue_id], |row| row.get::<_, String>(0))?
                .collect::<Result<std::collections::HashSet<_>, _>>()?
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
                INSERT INTO download_queue_items(queue_id, task_id, position, is_eligible)
                VALUES (?1, ?2, ?3, ?4)
                "#,
                params![
                    queue_id,
                    task_id.as_uuid().to_string(),
                    i64::try_from(position).map_err(|_| DownloadStorageError::IntegerOverflow)?,
                    eligible.contains(&task_id.as_uuid().to_string())
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
            SELECT task_id
            FROM download_queue_items
            WHERE queue_id = ?1 AND is_eligible = 1
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
                   speed_limit_bytes_per_second, schedule_json
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
                })
            })?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
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
        transaction.commit()?;
        Ok(())
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
                    error_message
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
                    });
                    let error = match (error_code, error_message) {
                        (Some(code), Some(message)) => Some(DownloadFault {
                            code: enum_from_name(&code, 6)?,
                            message,
                            retryable: false,
                        }),
                        _ => None,
                    };
                    Ok(StoredTransferState {
                        probe,
                        downloaded_bytes,
                        error,
                        temporary_path: None,
                        final_path: None,
                        final_committed: false,
                    })
                },
            )
            .optional()?
            .ok_or(DownloadStorageError::TaskNotFound(task_id))?;
        let mut statement = connection
            .prepare("SELECT kind, path, state FROM download_artifacts WHERE task_id = ?1")?;
        let artifacts = statement.query_map([task_id.as_uuid().to_string()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                PathBuf::from(row.get::<_, String>(1)?),
                row.get::<_, String>(2)?,
            ))
        })?;
        for artifact in artifacts {
            let (kind, path, artifact_state) = artifact?;
            match kind.as_str() {
                "temporary" => state.temporary_path = Some(path),
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
            SET status = 'paused', updated_at_unix_ms = ?
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
    pub temporary_path: Option<PathBuf>,
    pub final_path: Option<PathBuf>,
    pub final_committed: bool,
}

fn save_task_row(connection: &Connection, task: &DownloadTask) -> Result<(), DownloadStorageError> {
    let request_json = serde_json::to_string(&task.request)?;
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
            completed_at_unix_ms
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ON CONFLICT(task_id) DO UPDATE SET
            request_json = excluded.request_json,
            origin = excluded.origin,
            status = excluded.status,
            updated_at_unix_ms = excluded.updated_at_unix_ms,
            completed_at_unix_ms = excluded.completed_at_unix_ms
        "#,
        params![
            task.id.as_uuid().to_string(),
            request_json,
            origin,
            task.status.storage_name(),
            task.created_at_unix_ms,
            task.updated_at_unix_ms,
            task.completed_at_unix_ms,
        ],
    )?;
    Ok(())
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
    let (error_code, error_message) = match state.error.as_ref() {
        Some(error) => (Some(enum_name(&error.code)?), Some(error.message.as_str())),
        None => (None, None),
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
            error_message = ?9
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
            } else {
                "pending"
            },
            Some(state.downloaded_bytes),
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
    let request = serde_json::from_str::<DownloadRequest>(&request_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let origin = enum_from_name::<RequestOrigin>(&raw_origin, 2)?;
    let status = enum_from_name::<DownloadStatus>(&raw_status, 3)?;

    DownloadTask::restore(
        DownloadTaskId::from_uuid(id),
        request,
        origin,
        status,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    )
    .map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(3, rusqlite::types::Type::Text, Box::new(error))
    })
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
    Json(#[from] serde_json::Error),
    #[error("download database schema {current} is newer than supported schema {supported}")]
    UnsupportedSchema { current: u32, supported: u32 },
    #[error("download database lock is poisoned")]
    LockPoisoned,
    #[error("enum did not serialize to a string")]
    InvalidEnumEncoding,
    #[error("download integer does not fit SQLite INTEGER")]
    IntegerOverflow,
    #[error("download task does not exist: {0:?}")]
    TaskNotFound(DownloadTaskId),
    #[error("download queue does not exist: {0}")]
    QueueNotFound(String),
    #[error("download segment {segment_index} does not exist for task {task_id:?}")]
    SegmentNotFound {
        task_id: DownloadTaskId,
        segment_index: u32,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DownloadFault, DownloadProbe, DownloadRequest, DownloadStatus};

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

    fn sample_task(file_name: &str) -> DownloadTask {
        let directory = std::env::temp_dir();
        DownloadTask::new(
            DownloadRequest {
                source_url: format!("https://example.test/{file_name}"),
                save_directory: directory,
                file_name: Some(file_name.into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                scheduled_at_unix_ms: None,
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
                file_name: Some("archive.bin".into()),
                headers: Vec::new(),
                cookie: None,
                referrer: None,
                connection_limit: Some(1),
                speed_limit_bytes_per_second: None,
                scheduled_at_unix_ms: None,
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
            }),
            downloaded_bytes: 16,
            error: None::<DownloadFault>,
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
