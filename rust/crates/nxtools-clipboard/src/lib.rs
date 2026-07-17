use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureSource {
    pub window_handle: i64,
    pub process_id: u32,
    pub process_name: String,
    pub window_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextCapture {
    pub text: String,
    pub source: CaptureSource,
    pub captured_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardItem {
    pub id: i64,
    pub text: String,
    pub source: CaptureSource,
    pub captured_at_unix_ms: i64,
    pub copy_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureOutcome {
    Inserted(ClipboardItem),
    Updated(ClipboardItem),
}

pub struct ClipboardHistory {
    connection: Connection,
}

impl ClipboardHistory {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ClipboardHistoryError> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                source_window_handle INTEGER NOT NULL,
                source_process_id INTEGER NOT NULL,
                source_process_name TEXT NOT NULL,
                source_window_title TEXT NOT NULL,
                captured_at_unix_ms INTEGER NOT NULL,
                copy_count INTEGER NOT NULL DEFAULT 1
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_items_recent
                ON clipboard_items(captured_at_unix_ms DESC, id DESC);",
        )?;
        Ok(Self { connection })
    }

    pub fn capture(
        &mut self,
        capture: TextCapture,
    ) -> Result<CaptureOutcome, ClipboardHistoryError> {
        let latest = self
            .connection
            .query_row(
                "SELECT id, text FROM clipboard_items
                 ORDER BY captured_at_unix_ms DESC, id DESC LIMIT 1",
                [],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;

        if let Some((id, latest_text)) = latest
            && latest_text == capture.text
        {
            self.connection.execute(
                "UPDATE clipboard_items
                 SET source_window_handle = ?1,
                     source_process_id = ?2,
                     source_process_name = ?3,
                     source_window_title = ?4,
                     captured_at_unix_ms = ?5,
                     copy_count = copy_count + 1
                 WHERE id = ?6",
                params![
                    capture.source.window_handle,
                    capture.source.process_id,
                    capture.source.process_name,
                    capture.source.window_title,
                    capture.captured_at_unix_ms,
                    id,
                ],
            )?;
            return Ok(CaptureOutcome::Updated(self.item_by_id(id)?));
        }

        self.connection.execute(
            "INSERT INTO clipboard_items (
                text, source_window_handle, source_process_id, source_process_name,
                source_window_title, captured_at_unix_ms, copy_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1)",
            params![
                capture.text,
                capture.source.window_handle,
                capture.source.process_id,
                capture.source.process_name,
                capture.source.window_title,
                capture.captured_at_unix_ms,
            ],
        )?;
        let id = self.connection.last_insert_rowid();
        let item = self.item_by_id(id)?;
        Ok(CaptureOutcome::Inserted(item))
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        let mut statement = self.connection.prepare(
            "SELECT id, text, source_window_handle, source_process_id,
                    source_process_name, source_window_title, captured_at_unix_ms, copy_count
             FROM clipboard_items
             ORDER BY captured_at_unix_ms DESC, id DESC
             LIMIT ?1",
        )?;
        let rows = statement.query_map([limit as i64], map_item)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        if query.is_empty() {
            return self.recent(limit);
        }

        let mut statement = self.connection.prepare(
            "SELECT id, text, source_window_handle, source_process_id,
                    source_process_name, source_window_title, captured_at_unix_ms, copy_count
             FROM clipboard_items
             WHERE instr(lower(text), lower(?1)) > 0
             ORDER BY captured_at_unix_ms DESC, id DESC
             LIMIT ?2",
        )?;
        let rows = statement.query_map(params![query, limit as i64], map_item)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn item_by_id(&self, id: i64) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.connection
            .query_row(
                "SELECT id, text, source_window_handle, source_process_id,
                        source_process_name, source_window_title, captured_at_unix_ms, copy_count
                 FROM clipboard_items WHERE id = ?1",
                [id],
                map_item,
            )
            .map_err(Into::into)
    }
}

fn map_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardItem> {
    Ok(ClipboardItem {
        id: row.get(0)?,
        text: row.get(1)?,
        source: CaptureSource {
            window_handle: row.get(2)?,
            process_id: row.get(3)?,
            process_name: row.get(4)?,
            window_title: row.get(5)?,
        },
        captured_at_unix_ms: row.get(6)?,
        copy_count: row.get(7)?,
    })
}

#[derive(Debug, Error)]
pub enum ClipboardHistoryError {
    #[error("clipboard database error: {0}")]
    Database(#[from] rusqlite::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn captured_text_is_returned_as_the_most_recent_item() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");

        let outcome = history
            .capture(TextCapture {
                text: "first real clipboard item".into(),
                source: CaptureSource {
                    window_handle: 42,
                    process_id: 7,
                    process_name: "notepad".into(),
                    window_title: "notes.txt".into(),
                },
                captured_at_unix_ms: 1_700_000_000_000,
            })
            .expect("capture text");

        let CaptureOutcome::Inserted(inserted) = outcome else {
            panic!("first capture must insert an item");
        };
        let recent = history.recent(10).expect("read recent history");
        assert_eq!(recent, vec![inserted]);
        assert_eq!(recent[0].text, "first real clipboard item");
        assert_eq!(recent[0].source.process_name, "notepad");
        assert_eq!(recent[0].copy_count, 1);
    }

    #[test]
    fn adjacent_duplicate_updates_the_existing_item() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");

        let first = history
            .capture(TextCapture {
                text: "same text".into(),
                source: CaptureSource {
                    window_handle: 1,
                    process_id: 10,
                    process_name: "first-app".into(),
                    window_title: "first window".into(),
                },
                captured_at_unix_ms: 100,
            })
            .expect("first capture");
        let CaptureOutcome::Inserted(first_item) = first else {
            panic!("first capture must insert");
        };

        let duplicate = history
            .capture(TextCapture {
                text: "same text".into(),
                source: CaptureSource {
                    window_handle: 2,
                    process_id: 20,
                    process_name: "second-app".into(),
                    window_title: "second window".into(),
                },
                captured_at_unix_ms: 200,
            })
            .expect("duplicate capture");
        let CaptureOutcome::Updated(updated) = duplicate else {
            panic!("adjacent duplicate must update");
        };

        assert_eq!(updated.id, first_item.id);
        assert_eq!(updated.copy_count, 2);
        assert_eq!(updated.captured_at_unix_ms, 200);
        assert_eq!(updated.source.process_name, "second-app");
        assert_eq!(history.recent(10).expect("recent"), vec![updated]);
    }

    #[test]
    fn search_returns_matching_items_in_recent_order() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");

        for (text, captured_at) in [
            ("Rust clipboard architecture", 100),
            ("unrelated note", 200),
            ("Rust GPUI integration", 300),
        ] {
            history
                .capture(TextCapture {
                    text: text.into(),
                    source: CaptureSource {
                        window_handle: 1,
                        process_id: 10,
                        process_name: "editor".into(),
                        window_title: "design.md".into(),
                    },
                    captured_at_unix_ms: captured_at,
                })
                .expect("capture fixture");
        }

        let matches = history.search("rust", 10).expect("search history");
        assert_eq!(
            matches
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>(),
            vec!["Rust GPUI integration", "Rust clipboard architecture"]
        );
    }

    #[test]
    fn history_survives_closing_and_reopening_the_database() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        {
            let mut history = ClipboardHistory::open(&database).expect("open history");
            history
                .capture(TextCapture {
                    text: "persist across restart".into(),
                    source: CaptureSource {
                        window_handle: 5,
                        process_id: 50,
                        process_name: "terminal".into(),
                        window_title: "PowerShell".into(),
                    },
                    captured_at_unix_ms: 500,
                })
                .expect("capture persistent item");
        }

        let reopened = ClipboardHistory::open(&database).expect("reopen history");
        let items = reopened.recent(10).expect("read reopened history");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].text, "persist across restart");
        assert_eq!(items[0].source.window_title, "PowerShell");
    }
}
