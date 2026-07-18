use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureSource {
    pub window_handle: i64,
    pub process_id: u32,
    pub process_name: String,
    pub process_icon_path: String,
    pub window_title: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextCapture {
    pub text: String,
    pub native_formats: Vec<ClipboardNativeFormat>,
    pub source: CaptureSource,
    pub captured_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageCapture {
    pub image_path: String,
    pub width: u32,
    pub height: u32,
    pub native_formats: Vec<ClipboardNativeFormat>,
    pub source: CaptureSource,
    pub captured_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesCapture {
    pub paths: Vec<String>,
    pub native_formats: Vec<ClipboardNativeFormat>,
    pub source: CaptureSource,
    pub captured_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardNativeFormat {
    pub format_id: u32,
    pub format_name: String,
    pub data_path: String,
    pub size_bytes: u64,
    pub sort_order: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardKind {
    Text,
    Image,
    Files,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardAction {
    OriginalPaste,
    CopyOnly,
    PlainText,
    WithoutLineBreaks,
}

impl ClipboardKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Files => "files",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardItem {
    pub id: i64,
    pub kind: ClipboardKind,
    pub text: String,
    pub image_path: String,
    pub image_width: u32,
    pub image_height: u32,
    pub file_paths: Vec<String>,
    pub source: CaptureSource,
    pub captured_at_unix_ms: i64,
    pub copy_count: u32,
    pub is_favorite: bool,
    pub is_pinned: bool,
    pub deleted_at_unix_ms: Option<i64>,
}

pub fn action_text_payload(item: &ClipboardItem, action: ClipboardAction) -> Option<String> {
    match action {
        ClipboardAction::OriginalPaste | ClipboardAction::CopyOnly => {
            (item.kind == ClipboardKind::Text).then(|| item.text.clone())
        }
        ClipboardAction::PlainText => match item.kind {
            ClipboardKind::Text => Some(item.text.clone()),
            ClipboardKind::Image => Some(item.image_path.clone()),
            ClipboardKind::Files => Some(item.file_paths.join("\r\n")),
        },
        ClipboardAction::WithoutLineBreaks => (item.kind == ClipboardKind::Text)
            .then(|| item.text.replace("\r\n", "").replace(['\r', '\n'], "")),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptureOutcome {
    Inserted(ClipboardItem),
    Updated(ClipboardItem),
}

pub struct ClipboardHistory {
    connection: Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardItemFlag {
    Favorite,
    Pinned,
}

impl ClipboardHistory {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ClipboardHistoryError> {
        let connection = Connection::open(path)?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT NOT NULL DEFAULT 'text',
                text TEXT NOT NULL,
                image_path TEXT NOT NULL DEFAULT '',
                image_width INTEGER NOT NULL DEFAULT 0,
                image_height INTEGER NOT NULL DEFAULT 0,
                file_paths_json TEXT NOT NULL DEFAULT '[]',
                source_window_handle INTEGER NOT NULL,
                source_process_id INTEGER NOT NULL,
                source_process_name TEXT NOT NULL,
                source_process_icon_path TEXT NOT NULL DEFAULT '',
                source_window_title TEXT NOT NULL,
                captured_at_unix_ms INTEGER NOT NULL,
                copy_count INTEGER NOT NULL DEFAULT 1,
                is_favorite INTEGER NOT NULL DEFAULT 0,
                is_pinned INTEGER NOT NULL DEFAULT 0,
                deleted_at_unix_ms INTEGER NOT NULL DEFAULT 0
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_items_recent
                ON clipboard_items(captured_at_unix_ms DESC, id DESC);
            CREATE TABLE IF NOT EXISTS clipboard_item_formats (
                clipboard_item_id INTEGER NOT NULL,
                format_id INTEGER NOT NULL,
                format_name TEXT NOT NULL,
                data_path TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                sort_order INTEGER NOT NULL,
                PRIMARY KEY (clipboard_item_id, format_id, format_name)
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_item_formats_item
                ON clipboard_item_formats(clipboard_item_id, sort_order);",
        )?;
        ensure_column(&connection, "content_type", "TEXT NOT NULL DEFAULT 'text'")?;
        ensure_column(&connection, "image_path", "TEXT NOT NULL DEFAULT ''")?;
        ensure_column(&connection, "image_width", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(&connection, "image_height", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(&connection, "file_paths_json", "TEXT NOT NULL DEFAULT '[]'")?;
        ensure_column(
            &connection,
            "source_process_icon_path",
            "TEXT NOT NULL DEFAULT ''",
        )?;
        ensure_column(&connection, "is_favorite", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(&connection, "is_pinned", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(
            &connection,
            "deleted_at_unix_ms",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        Ok(Self { connection })
    }

    pub fn capture(
        &mut self,
        capture: TextCapture,
    ) -> Result<CaptureOutcome, ClipboardHistoryError> {
        self.capture_payload(
            ClipboardKind::Text,
            capture.text,
            String::new(),
            0,
            0,
            Vec::new(),
            capture.native_formats,
            capture.source,
            capture.captured_at_unix_ms,
        )
    }

    pub fn capture_image(
        &mut self,
        capture: ImageCapture,
    ) -> Result<CaptureOutcome, ClipboardHistoryError> {
        self.capture_payload(
            ClipboardKind::Image,
            String::new(),
            capture.image_path,
            capture.width,
            capture.height,
            Vec::new(),
            capture.native_formats,
            capture.source,
            capture.captured_at_unix_ms,
        )
    }

    pub fn capture_files(
        &mut self,
        capture: FilesCapture,
    ) -> Result<CaptureOutcome, ClipboardHistoryError> {
        self.capture_payload(
            ClipboardKind::Files,
            String::new(),
            String::new(),
            0,
            0,
            capture.paths,
            capture.native_formats,
            capture.source,
            capture.captured_at_unix_ms,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_payload(
        &mut self,
        kind: ClipboardKind,
        text: String,
        image_path: String,
        image_width: u32,
        image_height: u32,
        file_paths: Vec<String>,
        native_formats: Vec<ClipboardNativeFormat>,
        source: CaptureSource,
        captured_at_unix_ms: i64,
    ) -> Result<CaptureOutcome, ClipboardHistoryError> {
        let file_paths_json = serde_json::to_string(&file_paths)?;
        let transaction = self.connection.transaction()?;
        let latest = transaction
            .query_row(
                "SELECT id, content_type, text, image_path, file_paths_json FROM clipboard_items
                 WHERE deleted_at_unix_ms = 0
                 ORDER BY captured_at_unix_ms DESC, id DESC LIMIT 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?;

        if let Some((id, latest_kind, latest_text, latest_image, latest_files)) = latest
            && latest_kind == kind.as_str()
            && latest_text == text
            && latest_image == image_path
            && latest_files == file_paths_json
            && native_formats_match(&transaction, id, &native_formats)?
        {
            transaction.execute(
                "UPDATE clipboard_items
                 SET source_window_handle = ?1,
                     source_process_id = ?2,
                     source_process_name = ?3,
                     source_process_icon_path = ?4,
                     source_window_title = ?5,
                     captured_at_unix_ms = ?6,
                     copy_count = copy_count + 1
                 WHERE id = ?7",
                params![
                    source.window_handle,
                    source.process_id,
                    source.process_name,
                    source.process_icon_path,
                    source.window_title,
                    captured_at_unix_ms,
                    id,
                ],
            )?;
            replace_native_formats(&transaction, id, &native_formats)?;
            transaction.commit()?;
            return Ok(CaptureOutcome::Updated(self.item_by_id(id)?));
        }

        transaction.execute(
            "INSERT INTO clipboard_items (
                content_type, text, image_path, image_width, image_height, file_paths_json,
                source_window_handle, source_process_id, source_process_name,
                source_process_icon_path, source_window_title, captured_at_unix_ms, copy_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1)",
            params![
                kind.as_str(),
                text,
                image_path,
                image_width,
                image_height,
                file_paths_json,
                source.window_handle,
                source.process_id,
                source.process_name,
                source.process_icon_path,
                source.window_title,
                captured_at_unix_ms,
            ],
        )?;
        let id = transaction.last_insert_rowid();
        replace_native_formats(&transaction, id, &native_formats)?;
        transaction.commit()?;
        let item = self.item_by_id(id)?;
        Ok(CaptureOutcome::Inserted(item))
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query("", None, limit)
    }

    pub fn recent_by_kind(
        &self,
        kind: ClipboardKind,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query("", Some(kind), limit)
    }

    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query(query, None, limit)
    }

    pub fn search_by_kind(
        &self,
        query: &str,
        kind: ClipboardKind,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query(query, Some(kind), limit)
    }

    fn query(
        &self,
        query: &str,
        kind: Option<ClipboardKind>,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        let mut statement = self.connection.prepare(
            "SELECT id, content_type, text, image_path, image_width, image_height,
                    file_paths_json, source_window_handle, source_process_id,
                    source_process_name, source_process_icon_path, source_window_title,
                    captured_at_unix_ms, copy_count, is_favorite, is_pinned,
                    deleted_at_unix_ms
             FROM clipboard_items
             WHERE deleted_at_unix_ms = 0
               AND (?1 IS NULL OR content_type = ?1)
               AND (?2 = ''
                    OR instr(lower(text), lower(?2)) > 0
                    OR instr(lower(file_paths_json), lower(?2)) > 0)
             ORDER BY is_pinned DESC, captured_at_unix_ms DESC, id DESC
             LIMIT ?3",
        )?;
        let rows = statement.query_map(
            params![kind.map(ClipboardKind::as_str), query, limit as i64],
            map_item,
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn item_by_id(&self, id: i64) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.connection
            .query_row(
                "SELECT id, content_type, text, image_path, image_width, image_height,
                        file_paths_json, source_window_handle, source_process_id,
                        source_process_name, source_process_icon_path, source_window_title,
                        captured_at_unix_ms, copy_count, is_favorite, is_pinned,
                        deleted_at_unix_ms
                 FROM clipboard_items WHERE id = ?1",
                [id],
                map_item,
            )
            .map_err(Into::into)
    }

    pub fn set_favorite(
        &mut self,
        item_id: i64,
        value: bool,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.set_flag(item_id, ClipboardItemFlag::Favorite, value)
    }

    pub fn set_pinned(
        &mut self,
        item_id: i64,
        value: bool,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.set_flag(item_id, ClipboardItemFlag::Pinned, value)
    }

    fn set_flag(
        &mut self,
        item_id: i64,
        flag: ClipboardItemFlag,
        value: bool,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        let statement = match flag {
            ClipboardItemFlag::Favorite => {
                "UPDATE clipboard_items SET is_favorite = ?1 WHERE id = ?2"
            }
            ClipboardItemFlag::Pinned => "UPDATE clipboard_items SET is_pinned = ?1 WHERE id = ?2",
        };
        let changed = self
            .connection
            .execute(statement, params![value, item_id])?;
        if changed == 0 {
            return Err(ClipboardHistoryError::ItemNotFound(item_id));
        }
        self.item_by_id(item_id)
    }

    pub fn soft_delete(
        &mut self,
        item_id: i64,
        deleted_at_unix_ms: i64,
    ) -> Result<(), ClipboardHistoryError> {
        let changed = self.connection.execute(
            "UPDATE clipboard_items
             SET deleted_at_unix_ms = ?1, is_pinned = 0
             WHERE id = ?2 AND deleted_at_unix_ms = 0",
            params![deleted_at_unix_ms.max(1), item_id],
        )?;
        if changed == 0 {
            return Err(ClipboardHistoryError::ItemNotFound(item_id));
        }
        Ok(())
    }

    pub fn restore(&mut self, item_id: i64) -> Result<ClipboardItem, ClipboardHistoryError> {
        let changed = self.connection.execute(
            "UPDATE clipboard_items SET deleted_at_unix_ms = 0
             WHERE id = ?1 AND deleted_at_unix_ms != 0",
            [item_id],
        )?;
        if changed == 0 {
            return Err(ClipboardHistoryError::ItemNotFound(item_id));
        }
        self.item_by_id(item_id)
    }

    pub fn update_text(
        &mut self,
        item_id: i64,
        text: &str,
    ) -> Result<ClipboardItem, ClipboardHistoryError> {
        if text.trim().is_empty() {
            return Err(ClipboardHistoryError::EmptyTextEdit);
        }
        let transaction = self.connection.transaction()?;
        let state = transaction
            .query_row(
                "SELECT content_type, deleted_at_unix_ms FROM clipboard_items WHERE id = ?1",
                [item_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
            )
            .optional()?;
        let Some((kind, deleted_at)) = state else {
            return Err(ClipboardHistoryError::ItemNotFound(item_id));
        };
        if kind != ClipboardKind::Text.as_str() || deleted_at != 0 {
            return Err(ClipboardHistoryError::UnsupportedEdit(item_id));
        }
        transaction.execute(
            "UPDATE clipboard_items SET text = ?1 WHERE id = ?2",
            params![text, item_id],
        )?;
        transaction.execute(
            "DELETE FROM clipboard_item_formats WHERE clipboard_item_id = ?1",
            [item_id],
        )?;
        transaction.commit()?;
        self.item_by_id(item_id)
    }

    pub fn native_formats(
        &self,
        item_id: i64,
    ) -> Result<Vec<ClipboardNativeFormat>, ClipboardHistoryError> {
        read_native_formats(&self.connection, item_id).map_err(Into::into)
    }
}

fn replace_native_formats(
    transaction: &rusqlite::Transaction<'_>,
    item_id: i64,
    formats: &[ClipboardNativeFormat],
) -> Result<(), rusqlite::Error> {
    transaction.execute(
        "DELETE FROM clipboard_item_formats WHERE clipboard_item_id = ?1",
        [item_id],
    )?;
    let mut statement = transaction.prepare(
        "INSERT INTO clipboard_item_formats (
            clipboard_item_id, format_id, format_name, data_path, size_bytes, sort_order
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for format in formats {
        statement.execute(params![
            item_id,
            i64::from(format.format_id),
            format.format_name,
            format.data_path,
            format.size_bytes.min(i64::MAX as u64) as i64,
            i64::from(format.sort_order),
        ])?;
    }
    Ok(())
}

fn native_formats_match(
    transaction: &rusqlite::Transaction<'_>,
    item_id: i64,
    incoming: &[ClipboardNativeFormat],
) -> Result<bool, rusqlite::Error> {
    Ok(read_native_formats(transaction, item_id)? == incoming)
}

fn read_native_formats(
    connection: &Connection,
    item_id: i64,
) -> Result<Vec<ClipboardNativeFormat>, rusqlite::Error> {
    let mut statement = connection.prepare(
        "SELECT format_id, format_name, data_path, size_bytes, sort_order
         FROM clipboard_item_formats
         WHERE clipboard_item_id = ?1
         ORDER BY sort_order, format_id, format_name",
    )?;
    statement
        .query_map([item_id], |row| {
            let size_bytes = row.get::<_, i64>(3)?;
            let size_bytes = u64::try_from(size_bytes).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    3,
                    rusqlite::types::Type::Integer,
                    Box::new(error),
                )
            })?;
            Ok(ClipboardNativeFormat {
                format_id: row.get(0)?,
                format_name: row.get(1)?,
                data_path: row.get(2)?,
                size_bytes,
                sort_order: row.get(4)?,
            })
        })?
        .collect()
}

fn map_item(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardItem> {
    let kind = match row.get::<_, String>(1)?.as_str() {
        "image" => ClipboardKind::Image,
        "files" => ClipboardKind::Files,
        _ => ClipboardKind::Text,
    };
    let file_paths_json = row.get::<_, String>(6)?;
    Ok(ClipboardItem {
        id: row.get(0)?,
        kind,
        text: row.get(2)?,
        image_path: row.get(3)?,
        image_width: row.get(4)?,
        image_height: row.get(5)?,
        file_paths: serde_json::from_str(&file_paths_json).unwrap_or_default(),
        source: CaptureSource {
            window_handle: row.get(7)?,
            process_id: row.get(8)?,
            process_name: row.get(9)?,
            process_icon_path: row.get(10)?,
            window_title: row.get(11)?,
        },
        captured_at_unix_ms: row.get(12)?,
        copy_count: row.get(13)?,
        is_favorite: row.get::<_, i64>(14)? != 0,
        is_pinned: row.get::<_, i64>(15)? != 0,
        deleted_at_unix_ms: match row.get::<_, i64>(16)? {
            0 => None,
            timestamp => Some(timestamp),
        },
    })
}

#[derive(Debug, Error)]
pub enum ClipboardHistoryError {
    #[error("clipboard database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("clipboard data serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("clipboard item {0} was not found")]
    ItemNotFound(i64),
    #[error("clipboard item {0} cannot be edited as text")]
    UnsupportedEdit(i64),
    #[error("clipboard text edit cannot be empty")]
    EmptyTextEdit,
}

fn ensure_column(
    connection: &Connection,
    name: &str,
    definition: &str,
) -> Result<(), rusqlite::Error> {
    let mut statement = connection.prepare("PRAGMA table_info(clipboard_items)")?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;
    if !columns.iter().any(|column| column == name) {
        connection.execute(
            &format!("ALTER TABLE clipboard_items ADD COLUMN {name} {definition}"),
            [],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_source() -> CaptureSource {
        CaptureSource {
            window_handle: 1,
            process_id: 2,
            process_name: "fixture".into(),
            process_icon_path: String::new(),
            window_title: "fixture".into(),
        }
    }

    #[test]
    fn clipboard_actions_have_one_canonical_text_fallback() {
        let text = ClipboardItem {
            id: 1,
            kind: ClipboardKind::Text,
            text: "first\r\nsecond\nthird\rfourth".into(),
            image_path: String::new(),
            image_width: 0,
            image_height: 0,
            file_paths: Vec::new(),
            source: fixture_source(),
            captured_at_unix_ms: 1,
            copy_count: 1,
            is_favorite: false,
            is_pinned: false,
            deleted_at_unix_ms: None,
        };
        let files = ClipboardItem {
            id: 2,
            kind: ClipboardKind::Files,
            text: String::new(),
            image_path: String::new(),
            image_width: 0,
            image_height: 0,
            file_paths: vec!["C:\\one.txt".into(), "D:\\two.txt".into()],
            source: fixture_source(),
            captured_at_unix_ms: 2,
            copy_count: 1,
            is_favorite: false,
            is_pinned: false,
            deleted_at_unix_ms: None,
        };

        assert_eq!(
            action_text_payload(&text, ClipboardAction::PlainText).as_deref(),
            Some("first\r\nsecond\nthird\rfourth")
        );
        assert_eq!(
            action_text_payload(&text, ClipboardAction::WithoutLineBreaks).as_deref(),
            Some("firstsecondthirdfourth")
        );
        assert_eq!(
            action_text_payload(&files, ClipboardAction::PlainText).as_deref(),
            Some("C:\\one.txt\r\nD:\\two.txt")
        );
        assert_eq!(
            action_text_payload(&files, ClipboardAction::WithoutLineBreaks),
            None
        );
    }

    #[test]
    fn captured_text_is_returned_as_the_most_recent_item() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");

        let outcome = history
            .capture(TextCapture {
                text: "first real clipboard item".into(),
                native_formats: Vec::new(),
                source: CaptureSource {
                    window_handle: 42,
                    process_id: 7,
                    process_name: "notepad".into(),
                    process_icon_path: "C:/icons/notepad.png".into(),
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
        assert_eq!(recent[0].source.process_icon_path, "C:/icons/notepad.png");
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
                native_formats: Vec::new(),
                source: CaptureSource {
                    window_handle: 1,
                    process_id: 10,
                    process_name: "first-app".into(),
                    process_icon_path: String::new(),
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
                native_formats: Vec::new(),
                source: CaptureSource {
                    window_handle: 2,
                    process_id: 20,
                    process_name: "second-app".into(),
                    process_icon_path: String::new(),
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
    fn equal_visible_text_with_different_native_formats_stays_distinct() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let native = |data_path: &str| ClipboardNativeFormat {
            format_id: 49_152,
            format_name: "HTML Format".into(),
            data_path: data_path.into(),
            size_bytes: 64,
            sort_order: 0,
        };

        let first = history
            .capture(TextCapture {
                text: "same visible text".into(),
                native_formats: vec![native("C:/formats/first.bin")],
                source: fixture_source(),
                captured_at_unix_ms: 100,
            })
            .expect("capture first native representation");
        let second = history
            .capture(TextCapture {
                text: "same visible text".into(),
                native_formats: vec![native("C:/formats/second.bin")],
                source: fixture_source(),
                captured_at_unix_ms: 200,
            })
            .expect("capture second native representation");

        assert!(matches!(first, CaptureOutcome::Inserted(_)));
        assert!(matches!(second, CaptureOutcome::Inserted(_)));
        assert_eq!(history.recent(10).expect("recent").len(), 2);
    }

    #[test]
    fn favorite_and_pin_persist_with_pinned_items_ordered_first() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");

        let first = history
            .capture(TextCapture {
                text: "older but pinned".into(),
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 100,
            })
            .expect("capture first");
        let CaptureOutcome::Inserted(first) = first else {
            panic!("first capture must insert")
        };
        let second = history
            .capture(TextCapture {
                text: "newer and favorite".into(),
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 200,
            })
            .expect("capture second");
        let CaptureOutcome::Inserted(second) = second else {
            panic!("second capture must insert")
        };

        history.set_pinned(first.id, true).expect("pin first");
        history
            .set_favorite(second.id, true)
            .expect("favorite second");
        drop(history);

        let history = ClipboardHistory::open(&database).expect("reopen history");
        let recent = history.recent(10).expect("recent");
        assert_eq!(
            recent.iter().map(|item| item.id).collect::<Vec<_>>(),
            [first.id, second.id]
        );
        assert!(recent[0].is_pinned);
        assert!(!recent[0].is_favorite);
        assert!(recent[1].is_favorite);
        assert!(!recent[1].is_pinned);
    }

    #[test]
    fn soft_delete_hides_and_unpins_an_item_until_restore() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let captured = history
            .capture(TextCapture {
                text: "recover me".into(),
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 100,
            })
            .expect("capture item");
        let CaptureOutcome::Inserted(item) = captured else {
            panic!("capture must insert")
        };
        history.set_pinned(item.id, true).expect("pin item");
        history.set_favorite(item.id, true).expect("favorite item");

        history.soft_delete(item.id, 200).expect("soft delete");
        assert!(history.recent(10).expect("recent after delete").is_empty());

        let restored = history.restore(item.id).expect("restore item");
        assert!(restored.is_favorite);
        assert!(!restored.is_pinned);
        assert_eq!(restored.deleted_at_unix_ms, None);
        assert_eq!(
            history.recent(10).expect("recent after restore"),
            [restored]
        );
    }

    #[test]
    fn editing_text_converts_it_to_plain_text_and_rejects_non_text_items() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let captured = history
            .capture(TextCapture {
                text: "rich original".into(),
                native_formats: vec![ClipboardNativeFormat {
                    format_id: 49_152,
                    format_name: "HTML Format".into(),
                    data_path: "C:/formats/rich.bin".into(),
                    size_bytes: 128,
                    sort_order: 0,
                }],
                source: fixture_source(),
                captured_at_unix_ms: 100,
            })
            .expect("capture text");
        let CaptureOutcome::Inserted(text_item) = captured else {
            panic!("text capture must insert")
        };

        let edited = history
            .update_text(text_item.id, "edited plain text")
            .expect("edit text");
        assert_eq!(edited.text, "edited plain text");
        assert!(
            history
                .native_formats(text_item.id)
                .expect("formats")
                .is_empty()
        );

        let image = history
            .capture_image(ImageCapture {
                image_path: "C:/images/capture.png".into(),
                width: 100,
                height: 80,
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 200,
            })
            .expect("capture image");
        let CaptureOutcome::Inserted(image_item) = image else {
            panic!("image capture must insert")
        };
        assert!(matches!(
            history.update_text(image_item.id, "not allowed"),
            Err(ClipboardHistoryError::UnsupportedEdit(id)) if id == image_item.id
        ));
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
                    native_formats: Vec::new(),
                    source: CaptureSource {
                        window_handle: 1,
                        process_id: 10,
                        process_name: "editor".into(),
                        process_icon_path: String::new(),
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
                    native_formats: Vec::new(),
                    source: CaptureSource {
                        window_handle: 5,
                        process_id: 50,
                        process_name: "terminal".into(),
                        process_icon_path: String::new(),
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

    #[test]
    fn image_and_files_are_persisted_with_their_real_types() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let source = CaptureSource {
            window_handle: 1,
            process_id: 2,
            process_name: "capture".into(),
            process_icon_path: String::new(),
            window_title: "capture".into(),
        };
        history
            .capture_image(ImageCapture {
                image_path: "C:/data/image.png".into(),
                width: 640,
                height: 480,
                native_formats: Vec::new(),
                source: source.clone(),
                captured_at_unix_ms: 10,
            })
            .expect("capture image");
        history
            .capture_files(FilesCapture {
                paths: vec!["C:/one.txt".into(), "C:/two.txt".into()],
                native_formats: Vec::new(),
                source,
                captured_at_unix_ms: 20,
            })
            .expect("capture files");

        let items = history.recent(10).expect("recent");
        assert_eq!(items[0].kind, ClipboardKind::Files);
        assert_eq!(items[0].file_paths.len(), 2);
        assert_eq!(items[1].kind, ClipboardKind::Image);
        assert_eq!((items[1].image_width, items[1].image_height), (640, 480));
    }

    #[test]
    fn kind_filter_is_applied_before_the_result_limit() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let source = CaptureSource {
            window_handle: 1,
            process_id: 2,
            process_name: "capture".into(),
            process_icon_path: String::new(),
            window_title: "capture".into(),
        };
        history
            .capture_image(ImageCapture {
                image_path: "C:/data/older-image.png".into(),
                width: 10,
                height: 10,
                native_formats: Vec::new(),
                source: source.clone(),
                captured_at_unix_ms: 1,
            })
            .expect("capture image");
        for index in 0..3 {
            history
                .capture(TextCapture {
                    text: format!("newer text {index}"),
                    native_formats: Vec::new(),
                    source: source.clone(),
                    captured_at_unix_ms: 10 + index,
                })
                .expect("capture text");
        }

        let images = history
            .recent_by_kind(ClipboardKind::Image, 1)
            .expect("filtered recent");
        assert_eq!(images.len(), 1);
        assert_eq!(images[0].image_path, "C:/data/older-image.png");
    }

    #[test]
    fn native_formats_survive_capture_and_database_reopen() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let item_id = {
            let mut history = ClipboardHistory::open(&database).expect("open history");
            let outcome = history
                .capture(TextCapture {
                    text: "formatted".into(),
                    native_formats: vec![ClipboardNativeFormat {
                        format_id: 49_222,
                        format_name: "HTML Format".into(),
                        data_path: "C:/data/html-format.bin".into(),
                        size_bytes: 128,
                        sort_order: 1,
                    }],
                    source: fixture_source(),
                    captured_at_unix_ms: 10,
                })
                .expect("capture text");
            match outcome {
                CaptureOutcome::Inserted(item) | CaptureOutcome::Updated(item) => item.id,
            }
        };

        let reopened = ClipboardHistory::open(&database).expect("reopen history");
        let formats = reopened
            .native_formats(item_id)
            .expect("load native formats");
        assert_eq!(formats.len(), 1);
        assert_eq!(formats[0].format_name, "HTML Format");
        assert_eq!(formats[0].data_path, "C:/data/html-format.bin");
        assert_eq!(formats[0].size_bytes, 128);
    }
}
