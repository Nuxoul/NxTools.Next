mod archive;
mod categories;
mod query;
mod storage;

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;

pub use archive::{
    ClipboardArchiveError, ClipboardArchiveExportReport, ClipboardArchiveImportReport,
};
pub use storage::{
    ClipboardDatabaseBackupReport, ClipboardDatabaseCompactionReport, ClipboardStorageError,
    ClipboardStorageMigrationReport,
};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClipboardSearchScope {
    #[default]
    Content,
    Extended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardCapturePolicy {
    pub enabled: bool,
    pub capture_images: bool,
    pub capture_files: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardRetentionPolicy {
    pub max_items: usize,
    pub retention_days: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardCleanupReport {
    pub soft_deleted: usize,
    pub purged: usize,
    pub purged_asset_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardUsage {
    pub item_id: i64,
    pub last_used_at_unix_ms: i64,
    pub use_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardHistoryPage {
    pub items: Vec<ClipboardItem>,
    pub total: usize,
    pub has_more: bool,
}

/// List-only text prefix budget, measured in Unicode scalar values.
pub const CLIPBOARD_SUMMARY_CHARACTERS: usize = 400;

/// Presentation snapshot only: hydrate by immutable ID before actions or editing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardItemSummary {
    pub item: ClipboardItem,
    pub text_character_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardSummaryPage {
    pub items: Vec<ClipboardItemSummary>,
    pub total: usize,
    pub has_more: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClipboardHistoryCounts {
    pub total: usize,
    pub text: usize,
    pub images: usize,
    pub files: usize,
}

impl ClipboardCapturePolicy {
    pub fn allows(self, kind: ClipboardKind) -> bool {
        self.enabled
            && match kind {
                ClipboardKind::Text => true,
                ClipboardKind::Image => self.capture_images,
                ClipboardKind::Files => self.capture_files,
            }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardAction {
    OriginalPaste,
    CopyOnly,
    PlainText,
    WithoutLineBreaks,
}

pub const BUILTIN_ALL_CATEGORY_ID: &str = "builtin-all";
pub const BUILTIN_FAVORITE_CATEGORY_ID: &str = "builtin-favorite";
pub const BUILTIN_TEXT_CATEGORY_ID: &str = "builtin-text";
pub const BUILTIN_IMAGE_CATEGORY_ID: &str = "builtin-image";
pub const BUILTIN_FILES_CATEGORY_ID: &str = "builtin-files";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardCategoryKind {
    All,
    Favorite,
    Text,
    Image,
    Files,
    Custom,
}

impl ClipboardCategoryKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Favorite => "favorite",
            Self::Text => "text",
            Self::Image => "image",
            Self::Files => "files",
            Self::Custom => "custom",
        }
    }

    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "all" => Ok(Self::All),
            "favorite" => Ok(Self::Favorite),
            "text" => Ok(Self::Text),
            "image" => Ok(Self::Image),
            "files" => Ok(Self::Files),
            "custom" => Ok(Self::Custom),
            _ => Err(format!("unknown clipboard category kind: {value}")),
        }
    }

    pub fn is_builtin(self) -> bool {
        self != Self::Custom
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardCategory {
    pub id: String,
    pub name: String,
    pub kind: ClipboardCategoryKind,
    pub sort_order: i64,
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
    pub last_used_at_unix_ms: i64,
    pub copy_count: u32,
    pub is_favorite: bool,
    pub is_pinned: bool,
    pub deleted_at_unix_ms: Option<i64>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipboardPageCursor {
    pub is_pinned: bool,
    pub last_used_at_unix_ms: i64,
    pub captured_at_unix_ms: i64,
    pub id: i64,
}

impl From<&ClipboardItem> for ClipboardPageCursor {
    fn from(item: &ClipboardItem) -> Self {
        Self {
            is_pinned: item.is_pinned,
            last_used_at_unix_ms: item.last_used_at_unix_ms,
            captured_at_unix_ms: item.captured_at_unix_ms,
            id: item.id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardOpenTargetKind {
    Web,
    Directory,
    File,
    RegistryKey,
    SystemTool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipboardOpenTarget {
    pub value: String,
    pub kind: ClipboardOpenTargetKind,
}

impl ClipboardOpenTarget {
    pub fn select_in_explorer(&self) -> bool {
        self.kind == ClipboardOpenTargetKind::File
    }
}

pub fn resolve_open_targets(item: &ClipboardItem) -> Vec<ClipboardOpenTarget> {
    let mut targets = Vec::new();
    let mut seen = HashSet::new();

    for path in &item.file_paths {
        push_path_target(path, &mut targets, &mut seen);
    }
    if item.kind == ClipboardKind::Text {
        for url in extract_web_urls(&item.text) {
            push_target(
                ClipboardOpenTarget {
                    value: url,
                    kind: ClipboardOpenTargetKind::Web,
                },
                &mut targets,
                &mut seen,
            );
        }
        for line in item.text.lines() {
            if let Some(value) = normalize_registry_key(line) {
                push_target(
                    ClipboardOpenTarget {
                        value,
                        kind: ClipboardOpenTargetKind::RegistryKey,
                    },
                    &mut targets,
                    &mut seen,
                );
                continue;
            }
            if let Some(value) = normalize_system_tool(line) {
                push_target(
                    ClipboardOpenTarget {
                        value,
                        kind: ClipboardOpenTargetKind::SystemTool,
                    },
                    &mut targets,
                    &mut seen,
                );
                continue;
            }
            push_path_target(line, &mut targets, &mut seen);
            for quoted in quoted_segments(line) {
                push_path_target(quoted, &mut targets, &mut seen);
            }
        }
    }
    targets
}

fn normalize_registry_key(candidate: &str) -> Option<String> {
    let value = candidate.trim().trim_matches(['"', '\'']);
    let value = value.strip_prefix("计算机\\").unwrap_or(value);
    let value = if value
        .get(.."Computer\\".len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("Computer\\"))
    {
        &value["Computer\\".len()..]
    } else {
        value
    };
    let (root, remainder) = value.split_once('\\').unwrap_or((value, ""));
    let canonical_root = match root.to_ascii_uppercase().as_str() {
        "HKLM" | "HKEY_LOCAL_MACHINE" => "HKEY_LOCAL_MACHINE",
        "HKCU" | "HKEY_CURRENT_USER" => "HKEY_CURRENT_USER",
        "HKCR" | "HKEY_CLASSES_ROOT" => "HKEY_CLASSES_ROOT",
        "HKU" | "HKEY_USERS" => "HKEY_USERS",
        "HKCC" | "HKEY_CURRENT_CONFIG" => "HKEY_CURRENT_CONFIG",
        _ => return None,
    };
    Some(if remainder.is_empty() {
        canonical_root.to_owned()
    } else {
        format!("{canonical_root}\\{remainder}")
    })
}

fn normalize_system_tool(candidate: &str) -> Option<String> {
    let value = candidate.trim().to_ascii_lowercase();
    let executable = match value.as_str() {
        "msconfig" | "msconfig.exe" => "msconfig.exe",
        "taskmgr" | "taskmgr.exe" => "taskmgr.exe",
        "services.msc" => "services.msc",
        "devmgmt.msc" => "devmgmt.msc",
        "diskmgmt.msc" => "diskmgmt.msc",
        "eventvwr" | "eventvwr.msc" => "eventvwr.msc",
        "compmgmt.msc" => "compmgmt.msc",
        "gpedit.msc" => "gpedit.msc",
        "secpol.msc" => "secpol.msc",
        "optionalfeatures" | "optionalfeatures.exe" => "optionalfeatures.exe",
        "cleanmgr" | "cleanmgr.exe" => "cleanmgr.exe",
        "dxdiag" | "dxdiag.exe" => "dxdiag.exe",
        "winver" | "winver.exe" => "winver.exe",
        "resmon" | "resmon.exe" => "resmon.exe",
        "perfmon" | "perfmon.exe" => "perfmon.exe",
        "mrt" | "mrt.exe" => "mrt.exe",
        _ => return None,
    };
    Some(executable.to_owned())
}

fn push_path_target(
    candidate: &str,
    targets: &mut Vec<ClipboardOpenTarget>,
    seen: &mut HashSet<String>,
) {
    let value = candidate.trim().trim_matches(['"', '\'']);
    if value.is_empty() {
        return;
    }
    let path = Path::new(value);
    let kind = if path.is_file() {
        ClipboardOpenTargetKind::File
    } else if path.is_dir() {
        ClipboardOpenTargetKind::Directory
    } else {
        return;
    };
    push_target(
        ClipboardOpenTarget {
            value: value.to_owned(),
            kind,
        },
        targets,
        seen,
    );
}

fn push_target(
    target: ClipboardOpenTarget,
    targets: &mut Vec<ClipboardOpenTarget>,
    seen: &mut HashSet<String>,
) {
    if seen.insert(target.value.to_lowercase()) {
        targets.push(target);
    }
}

fn extract_web_urls(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut offset = 0;
    while offset < text.len() {
        let remaining = &text[offset..];
        let next_http = remaining.find("http://");
        let next_https = remaining.find("https://");
        let Some(start) = [next_http, next_https].into_iter().flatten().min() else {
            break;
        };
        let absolute_start = offset + start;
        let tail = &text[absolute_start..];
        let end = tail
            .char_indices()
            .find_map(|(index, character)| {
                (character.is_whitespace()
                    || matches!(
                        character,
                        '"' | '\'' | '<' | '>' | '，' | '。' | '；' | '！' | '？' | '）' | '】'
                    ))
                .then_some(index)
            })
            .unwrap_or(tail.len());
        let url = tail[..end].trim_end_matches(|character: char| {
            matches!(
                character,
                '.' | ',' | ';' | ':' | '!' | '?' | ')' | ']' | '}'
            )
        });
        if url.len() > "https://".len() {
            urls.push(url.to_owned());
        }
        offset = absolute_start + end.max(1);
    }
    urls
}

fn quoted_segments(line: &str) -> impl Iterator<Item = &str> {
    line.split('"').skip(1).step_by(2)
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
    IgnoredEmpty,
}

pub struct ClipboardHistory {
    connection: Connection,
    database_path: PathBuf,
    read_session: uuid::Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardItemFlag {
    Favorite,
    Pinned,
}

impl ClipboardHistory {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, ClipboardHistoryError> {
        let database_path = path.as_ref().to_owned();
        let connection = Connection::open(&database_path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        query::register_summary_functions(&connection)?;
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
                last_used_at_unix_ms INTEGER NOT NULL DEFAULT 0,
                copy_count INTEGER NOT NULL DEFAULT 1,
                use_count INTEGER NOT NULL DEFAULT 0,
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
                ON clipboard_item_formats(clipboard_item_id, sort_order);
            CREATE TABLE IF NOT EXISTS clipboard_categories (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                kind TEXT NOT NULL CHECK(kind IN ('all', 'favorite', 'text', 'image', 'files', 'custom')),
                sort_order INTEGER NOT NULL
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_clipboard_categories_name
                ON clipboard_categories(name COLLATE NOCASE);
            CREATE TABLE IF NOT EXISTS clipboard_category_sources (
                category_id TEXT NOT NULL,
                process_name TEXT NOT NULL COLLATE NOCASE,
                PRIMARY KEY (category_id, process_name)
            );
            CREATE INDEX IF NOT EXISTS idx_clipboard_category_sources_category
                ON clipboard_category_sources(category_id);
            CREATE TABLE IF NOT EXISTS clipboard_metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
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
            "last_used_at_unix_ms",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        ensure_column(&connection, "use_count", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(
            &connection,
            "deleted_at_unix_ms",
            "INTEGER NOT NULL DEFAULT 0",
        )?;
        connection.execute(
            "UPDATE clipboard_items
             SET last_used_at_unix_ms = captured_at_unix_ms
             WHERE last_used_at_unix_ms = 0",
            [],
        )?;
        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_clipboard_items_usage
             ON clipboard_items(is_pinned DESC, last_used_at_unix_ms DESC,
                                captured_at_unix_ms DESC, id DESC)",
            [],
        )?;
        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_clipboard_items_active_usage
             ON clipboard_items(is_pinned DESC, last_used_at_unix_ms DESC,
                                captured_at_unix_ms DESC, id DESC)
             WHERE deleted_at_unix_ms = 0",
            [],
        )?;
        categories::seed_builtin_categories(&connection)?;
        promote_legacy_single_image_rows(&connection)?;
        Ok(Self {
            connection,
            database_path,
            read_session: uuid::Uuid::new_v4(),
        })
    }

    pub fn database_path(&self) -> &Path {
        &self.database_path
    }

    pub fn create_database_backup(
        &self,
        backup_directory: &Path,
    ) -> Result<ClipboardDatabaseBackupReport, ClipboardStorageError> {
        storage::create_database_backup(self, backup_directory)
    }

    pub fn compact_database(
        &mut self,
        backup_directory: &Path,
    ) -> Result<ClipboardDatabaseCompactionReport, ClipboardStorageError> {
        storage::compact_database(self, backup_directory)
    }

    pub fn migrate_storage(
        &self,
        source_asset_root: &Path,
        destination_root: &Path,
    ) -> Result<ClipboardStorageMigrationReport, ClipboardStorageError> {
        storage::migrate_storage(self, source_asset_root, destination_root)
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
        if text.trim().is_empty() && image_path.is_empty() && file_paths.is_empty() {
            return Ok(CaptureOutcome::IgnoredEmpty);
        }
        let file_paths_json = serde_json::to_string(&file_paths)?;
        let transaction = self.connection.transaction()?;
        let matching_payload_ids = {
            let mut statement = transaction.prepare(
                "SELECT id FROM clipboard_items
                 WHERE content_type = ?1 AND text = ?2 AND image_path = ?3
                   AND file_paths_json = ?4
                 ORDER BY CASE WHEN deleted_at_unix_ms = 0 THEN 0 ELSE 1 END,
                          captured_at_unix_ms DESC, id DESC",
            )?;
            statement
                .query_map(
                    params![kind.as_str(), text, image_path, file_paths_json],
                    |row| row.get::<_, i64>(0),
                )?
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut matching_id = None;
        for id in matching_payload_ids {
            if native_formats_match(&transaction, id, &native_formats)? {
                matching_id = Some(id);
                break;
            }
        }

        if let Some(id) = matching_id {
            transaction.execute(
                "UPDATE clipboard_items
                 SET image_width = ?1,
                     image_height = ?2,
                     source_window_handle = ?3,
                     source_process_id = ?4,
                     source_process_name = ?5,
                     source_process_icon_path = ?6,
                     source_window_title = ?7,
                     captured_at_unix_ms = ?8,
                     last_used_at_unix_ms = ?8,
                     copy_count = copy_count + 1,
                     deleted_at_unix_ms = 0
                 WHERE id = ?9",
                params![
                    image_width,
                    image_height,
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
                source_process_icon_path, source_window_title, captured_at_unix_ms, copy_count,
                last_used_at_unix_ms, use_count
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 1, ?12, 0)",
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

    pub fn export_archive(
        &self,
        destination: &Path,
    ) -> Result<ClipboardArchiveExportReport, ClipboardArchiveError> {
        archive::export_archive(self, destination)
    }

    pub fn import_archive(
        &mut self,
        archive_path: &Path,
        asset_root: &Path,
    ) -> Result<ClipboardArchiveImportReport, ClipboardArchiveError> {
        archive::import_archive(self, archive_path, asset_root)
    }

    fn item_by_id(&self, id: i64) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.connection
            .query_row(
                "SELECT id, content_type, text, image_path, image_width, image_height,
                        file_paths_json, source_window_handle, source_process_id,
                        source_process_name, source_process_icon_path, source_window_title,
                        captured_at_unix_ms, last_used_at_unix_ms, copy_count, is_favorite, is_pinned,
                        deleted_at_unix_ms
                 FROM clipboard_items WHERE id = ?1",
                [id],
                map_item,
            )
            .map_err(Into::into)
    }

    /// Retrieve a full active row by immutable ID; unlike item_by_id, deleted rows are rejected.
    pub fn active_item(&self, item_id: i64) -> Result<ClipboardItem, ClipboardHistoryError> {
        self.connection
            .query_row(
                "SELECT id, content_type, text, image_path, image_width, image_height,
                    file_paths_json, source_window_handle, source_process_id,
                    source_process_name, source_process_icon_path, source_window_title,
                    captured_at_unix_ms, last_used_at_unix_ms, copy_count, is_favorite, is_pinned,
                    deleted_at_unix_ms
             FROM clipboard_items WHERE id = ?1 AND deleted_at_unix_ms = 0",
                [item_id],
                map_item,
            )
            .optional()?
            .ok_or(ClipboardHistoryError::ItemNotFound(item_id))
    }

    /// Read the full active row and its native formats from one SQLite snapshot.
    pub fn active_item_with_native_formats(
        &mut self,
        item_id: i64,
    ) -> Result<(ClipboardItem, Vec<ClipboardNativeFormat>), ClipboardHistoryError> {
        let transaction = self.connection.unchecked_transaction()?;
        let item = self.active_item(item_id)?;
        let formats = read_native_formats(&self.connection, item_id)?;
        transaction.commit()?;
        Ok((item, formats))
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

    pub fn mark_used(
        &mut self,
        item_id: i64,
        used_at_unix_ms: i64,
    ) -> Result<ClipboardUsage, ClipboardHistoryError> {
        let changed = self.connection.execute(
            "UPDATE clipboard_items
             SET last_used_at_unix_ms = ?1, use_count = use_count + 1
             WHERE id = ?2 AND deleted_at_unix_ms = 0",
            params![used_at_unix_ms, item_id],
        )?;
        if changed == 0 {
            return Err(ClipboardHistoryError::ItemNotFound(item_id));
        }
        self.connection
            .query_row(
                "SELECT id, last_used_at_unix_ms, use_count
                 FROM clipboard_items WHERE id = ?1",
                [item_id],
                |row| {
                    Ok(ClipboardUsage {
                        item_id: row.get(0)?,
                        last_used_at_unix_ms: row.get(1)?,
                        use_count: row.get(2)?,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn cleanup(
        &mut self,
        policy: ClipboardRetentionPolicy,
        now_unix_ms: i64,
    ) -> Result<ClipboardCleanupReport, ClipboardHistoryError> {
        const DAY_MS: i64 = 86_400_000;
        let max_items = policy.max_items.clamp(100, 100_000);
        let retention_days = policy.retention_days.clamp(1, 3650);
        let cutoff = now_unix_ms.saturating_sub(i64::from(retention_days) * DAY_MS);
        let purge_days = retention_days.max(7);
        let purge_cutoff = now_unix_ms.saturating_sub(i64::from(purge_days) * DAY_MS);
        let transaction = self.connection.transaction()?;

        let expired = transaction.execute(
            "UPDATE clipboard_items
             SET deleted_at_unix_ms = ?1
             WHERE deleted_at_unix_ms = 0
               AND is_favorite = 0
               AND is_pinned = 0
               AND last_used_at_unix_ms < ?2",
            params![now_unix_ms, cutoff],
        )?;
        let overflow = transaction.execute(
            "UPDATE clipboard_items
             SET deleted_at_unix_ms = ?1
             WHERE deleted_at_unix_ms = 0 AND id IN (
                 SELECT id FROM clipboard_items
                 WHERE deleted_at_unix_ms = 0
                   AND is_favorite = 0
                   AND is_pinned = 0
                 ORDER BY last_used_at_unix_ms DESC, captured_at_unix_ms DESC, id DESC
                 LIMIT -1 OFFSET ?2
             )",
            params![now_unix_ms, max_items as i64],
        )?;

        let purged = {
            let mut statement = transaction.prepare(
                "SELECT id, image_path, source_process_icon_path FROM clipboard_items
                 WHERE deleted_at_unix_ms > 0 AND deleted_at_unix_ms < ?1",
            )?;
            statement
                .query_map([purge_cutoff], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                })?
                .collect::<Result<Vec<_>, _>>()?
        };
        let mut candidate_assets = HashSet::new();
        for (id, image_path, source_icon_path) in &purged {
            if !image_path.is_empty() {
                candidate_assets.insert(image_path.clone());
            }
            if !source_icon_path.is_empty() {
                candidate_assets.insert(source_icon_path.clone());
            }
            let mut statement = transaction.prepare(
                "SELECT data_path FROM clipboard_item_formats
                 WHERE clipboard_item_id = ?1 AND data_path != ''",
            )?;
            let paths = statement
                .query_map([id], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            candidate_assets.extend(paths);
        }
        for (id, _, _) in &purged {
            transaction.execute(
                "DELETE FROM clipboard_item_formats WHERE clipboard_item_id = ?1",
                [id],
            )?;
            transaction.execute("DELETE FROM clipboard_items WHERE id = ?1", [id])?;
        }
        let mut purged_asset_paths = Vec::new();
        for path in candidate_assets {
            let still_referenced = transaction.query_row(
                "SELECT
                    EXISTS(SELECT 1 FROM clipboard_items
                           WHERE image_path = ?1 OR source_process_icon_path = ?1)
                    OR EXISTS(SELECT 1 FROM clipboard_item_formats WHERE data_path = ?1)",
                [&path],
                |row| row.get::<_, i64>(0),
            )? != 0;
            if !still_referenced {
                purged_asset_paths.push(path);
            }
        }
        purged_asset_paths.sort();
        transaction.commit()?;
        Ok(ClipboardCleanupReport {
            soft_deleted: expired + overflow,
            purged: purged.len(),
            purged_asset_paths,
        })
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

    pub fn referenced_asset_paths(&self) -> Result<HashSet<String>, ClipboardHistoryError> {
        let mut paths = HashSet::new();
        let mut statement = self.connection.prepare(
            "SELECT image_path, source_process_icon_path
             FROM clipboard_items
             WHERE deleted_at_unix_ms = 0",
        )?;
        for row in statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })? {
            let (image_path, source_icon_path) = row?;
            if !image_path.is_empty() {
                paths.insert(image_path);
            }
            if !source_icon_path.is_empty() {
                paths.insert(source_icon_path);
            }
        }
        let mut statement = self
            .connection
            .prepare("SELECT data_path FROM clipboard_item_formats WHERE data_path != ''")?;
        for path in statement.query_map([], |row| row.get::<_, String>(0))? {
            paths.insert(path?);
        }
        Ok(paths)
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
        last_used_at_unix_ms: row.get(13)?,
        copy_count: row.get(14)?,
        is_favorite: row.get::<_, i64>(15)? != 0,
        is_pinned: row.get::<_, i64>(16)? != 0,
        deleted_at_unix_ms: match row.get::<_, i64>(17)? {
            0 => None,
            timestamp => Some(timestamp),
        },
    })
}

fn promote_legacy_single_image_rows(connection: &Connection) -> Result<(), ClipboardHistoryError> {
    let migration_complete = connection
        .query_row(
            "SELECT value FROM clipboard_metadata WHERE key = 'single_image_file_promotion_v1'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()?
        .is_some();
    if migration_complete {
        return Ok(());
    }
    let candidates = {
        let mut statement = connection.prepare(
            "SELECT id, file_paths_json FROM clipboard_items WHERE content_type = 'files'",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for (id, paths_json) in candidates {
        let paths = serde_json::from_str::<Vec<String>>(&paths_json).unwrap_or_default();
        let Some(path) = paths.first().filter(|_| paths.len() == 1) else {
            continue;
        };
        if !is_existing_image_path(path) {
            continue;
        }
        connection.execute(
            "UPDATE clipboard_items
             SET content_type = 'image', image_path = ?2, image_width = 0,
                 image_height = 0, file_paths_json = '[]'
             WHERE id = ?1 AND content_type = 'files'",
            params![id, path],
        )?;
    }
    connection.execute(
        "INSERT OR REPLACE INTO clipboard_metadata (key, value)
         VALUES ('single_image_file_promotion_v1', 'complete')",
        [],
    )?;
    Ok(())
}

fn is_existing_image_path(value: &str) -> bool {
    use std::io::Read as _;

    let path = Path::new(value);
    if !path.is_file() {
        return false;
    }
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "gif" | "webp" | "tif" | "tiff"
            )
        })
    {
        return true;
    }
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut header = [0_u8; 12];
    let Ok(read) = file.read(&mut header) else {
        return false;
    };
    (read >= 8 && header[..8] == [137, 80, 78, 71, 13, 10, 26, 10])
        || (read >= 3 && header[..3] == [0xff, 0xd8, 0xff])
        || (read >= 6 && matches!(&header[..6], b"GIF87a" | b"GIF89a"))
        || (read >= 2 && header[..2] == *b"BM")
        || (read >= 12 && header[..4] == *b"RIFF" && header[8..12] == *b"WEBP")
}

#[derive(Debug, Error)]
pub enum ClipboardHistoryError {
    #[error("clipboard query superseded by a newer request")]
    QuerySuperseded,
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
    #[error("clipboard category {0} was not found")]
    CategoryNotFound(String),
    #[error("built-in clipboard category {0} cannot be changed")]
    BuiltInCategoryImmutable(String),
    #[error("clipboard category name cannot be empty")]
    EmptyCategoryName,
    #[error("clipboard category name {0} is already in use")]
    CategoryNameInUse(String),
    #[error("clipboard category order must contain every category exactly once")]
    InvalidCategoryOrder,
    #[error("clipboard category source process cannot be empty")]
    EmptyCategorySource,
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

    #[test]
    fn capture_policy_controls_optional_kinds_without_disabling_text() {
        let policy = ClipboardCapturePolicy {
            enabled: true,
            capture_images: false,
            capture_files: true,
        };
        assert!(policy.allows(ClipboardKind::Text));
        assert!(!policy.allows(ClipboardKind::Image));
        assert!(policy.allows(ClipboardKind::Files));

        let disabled = ClipboardCapturePolicy {
            enabled: false,
            ..policy
        };
        assert!(!disabled.allows(ClipboardKind::Text));
        assert!(!disabled.allows(ClipboardKind::Files));
    }

    fn fixture_source() -> CaptureSource {
        fixture_source_for("fixture")
    }

    fn fixture_source_for(process_name: &str) -> CaptureSource {
        CaptureSource {
            window_handle: 1,
            process_id: 2,
            process_name: process_name.into(),
            process_icon_path: String::new(),
            window_title: "fixture".into(),
        }
    }

    #[test]
    fn open_targets_include_multiple_urls_and_existing_paths() {
        let directory = tempfile::tempdir().expect("temp directory");
        let file = directory.path().join("one.txt");
        std::fs::write(&file, "fixture").expect("write fixture");
        let item = ClipboardItem {
            id: 1,
            kind: ClipboardKind::Text,
            text: format!(
                "打开 https://example.com/a，然后 http://example.org/b。\n{}",
                file.display()
            ),
            image_path: String::new(),
            image_width: 0,
            image_height: 0,
            file_paths: Vec::new(),
            source: fixture_source(),
            captured_at_unix_ms: 1,
            last_used_at_unix_ms: 1,
            copy_count: 1,
            is_favorite: false,
            is_pinned: false,
            deleted_at_unix_ms: None,
        };

        let targets = resolve_open_targets(&item);
        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].value, "https://example.com/a");
        assert_eq!(targets[1].value, "http://example.org/b");
        assert_eq!(targets[2].kind, ClipboardOpenTargetKind::File);
        assert!(targets[2].select_in_explorer());
    }

    #[test]
    fn open_targets_recognize_registry_keys_and_safe_windows_tools() {
        let item = ClipboardItem {
            id: 1,
            kind: ClipboardKind::Text,
            text: "计算机\\HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Services\\ProfSvc\nmsconfig\ncmd /c whoami".into(),
            image_path: String::new(),
            image_width: 0,
            image_height: 0,
            file_paths: Vec::new(),
            source: fixture_source(),
            captured_at_unix_ms: 1,
            last_used_at_unix_ms: 1,
            copy_count: 1,
            is_favorite: false,
            is_pinned: false,
            deleted_at_unix_ms: None,
        };

        let targets = resolve_open_targets(&item);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].kind, ClipboardOpenTargetKind::RegistryKey);
        assert_eq!(
            targets[0].value,
            "HKEY_LOCAL_MACHINE\\SYSTEM\\CurrentControlSet\\Services\\ProfSvc"
        );
        assert_eq!(targets[1].kind, ClipboardOpenTargetKind::SystemTool);
        assert_eq!(targets[1].value, "msconfig.exe");
    }

    #[test]
    fn legacy_single_image_file_rows_are_promoted_on_reopen() {
        let directory = tempfile::tempdir().expect("temp directory");
        let image = directory.path().join("qq-capture.png");
        std::fs::write(&image, [137, 80, 78, 71, 13, 10, 26, 10]).expect("write image signature");
        let database = directory.path().join("clipboard.db");
        {
            let mut history = ClipboardHistory::open(&database).expect("open history");
            history
                .capture_files(FilesCapture {
                    paths: vec![image.to_string_lossy().into_owned()],
                    native_formats: Vec::new(),
                    source: fixture_source_for("qq"),
                    captured_at_unix_ms: 1,
                })
                .expect("capture legacy file row");
            history
                .connection
                .execute(
                    "DELETE FROM clipboard_metadata WHERE key = 'single_image_file_promotion_v1'",
                    [],
                )
                .expect("simulate pre-migration database");
        }

        let history = ClipboardHistory::open(&database).expect("reopen history");
        let items = history.recent(10).expect("recent items");
        assert_eq!(items[0].kind, ClipboardKind::Image);
        assert_eq!(items[0].image_path, image.to_string_lossy());
        assert!(items[0].file_paths.is_empty());
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
            last_used_at_unix_ms: 1,
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
            last_used_at_unix_ms: 2,
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
    fn empty_capture_with_native_formats_is_ignored() {
        let directory = tempfile::tempdir().expect("create clipboard test directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let outcome = history
            .capture(TextCapture {
                text: " \r\n ".into(),
                native_formats: vec![ClipboardNativeFormat {
                    format_id: 13,
                    format_name: String::new(),
                    data_path: "C:/staging/empty-text.bin".into(),
                    size_bytes: 2,
                    sort_order: 0,
                }],
                source: fixture_source(),
                captured_at_unix_ms: 10,
            })
            .expect("empty capture should be ignored without a database error");
        assert!(matches!(outcome, CaptureOutcome::IgnoredEmpty));
        assert!(history.recent(10).expect("read recent history").is_empty());
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
    fn non_adjacent_duplicate_reuses_and_revives_the_existing_record() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");

        let CaptureOutcome::Inserted(first) = history
            .capture(TextCapture {
                text: "reuse me".into(),
                native_formats: Vec::new(),
                source: fixture_source_for("first-app"),
                captured_at_unix_ms: 100,
            })
            .expect("capture first")
        else {
            panic!("first capture must insert");
        };
        history
            .set_favorite(first.id, true)
            .expect("favorite first record");
        history
            .soft_delete(first.id, 150)
            .expect("soft delete first record");
        history
            .capture(TextCapture {
                text: "different content".into(),
                native_formats: Vec::new(),
                source: fixture_source_for("middle-app"),
                captured_at_unix_ms: 200,
            })
            .expect("capture middle record");

        let CaptureOutcome::Updated(reused) = history
            .capture(TextCapture {
                text: "reuse me".into(),
                native_formats: Vec::new(),
                source: fixture_source_for("last-app"),
                captured_at_unix_ms: 300,
            })
            .expect("recapture first content")
        else {
            panic!("a global duplicate must update its existing record");
        };

        assert_eq!(reused.id, first.id);
        assert_eq!(reused.copy_count, 2);
        assert!(reused.is_favorite);
        assert_eq!(reused.deleted_at_unix_ms, None);
        assert_eq!(reused.source.process_name, "last-app");
        let recent = history.recent(10).expect("recent after global duplicate");
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].id, first.id);
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
    fn marking_an_item_used_moves_it_to_the_front_and_counts_activations() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");

        let CaptureOutcome::Inserted(first) = history
            .capture(TextCapture {
                text: "first".into(),
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 100,
            })
            .expect("capture first")
        else {
            panic!("first capture must insert");
        };
        history
            .capture(TextCapture {
                text: "second".into(),
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 200,
            })
            .expect("capture second");

        let first_use = history.mark_used(first.id, 300).expect("mark first used");
        let second_use = history
            .mark_used(first.id, 400)
            .expect("mark first used again");

        assert_eq!(first_use.use_count, 1);
        assert_eq!(first_use.last_used_at_unix_ms, 300);
        assert_eq!(second_use.use_count, 2);
        assert_eq!(second_use.last_used_at_unix_ms, 400);
        assert_eq!(history.recent(10).expect("recent")[0].id, first.id);
    }

    #[test]
    fn built_in_categories_are_seeded_and_favorite_filtering_survives_reopen() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");

        let categories = history.categories().expect("load categories");
        assert_eq!(
            categories
                .iter()
                .map(|category| category.id.as_str())
                .collect::<Vec<_>>(),
            [
                BUILTIN_ALL_CATEGORY_ID,
                BUILTIN_FAVORITE_CATEGORY_ID,
                BUILTIN_TEXT_CATEGORY_ID,
                BUILTIN_IMAGE_CATEGORY_ID,
                BUILTIN_FILES_CATEGORY_ID,
            ]
        );

        let first = history
            .capture(TextCapture {
                text: "ordinary".into(),
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 100,
            })
            .expect("capture ordinary");
        let CaptureOutcome::Inserted(_first) = first else {
            panic!("ordinary capture must insert")
        };
        let second = history
            .capture(TextCapture {
                text: "favorite".into(),
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 200,
            })
            .expect("capture favorite");
        let CaptureOutcome::Inserted(second) = second else {
            panic!("favorite capture must insert")
        };
        history
            .set_favorite(second.id, true)
            .expect("favorite item");

        let favorites = history
            .query_category("", BUILTIN_FAVORITE_CATEGORY_ID, 10)
            .expect("query favorites");
        assert_eq!(
            favorites.iter().map(|item| item.id).collect::<Vec<_>>(),
            [second.id]
        );
        drop(history);

        let history = ClipboardHistory::open(&database).expect("reopen history");
        assert_eq!(
            history.categories().expect("reloaded categories"),
            categories
        );
    }

    #[test]
    fn custom_categories_can_be_renamed_reordered_deleted_and_reopened() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");

        let work = history.create_category("工作").expect("create work");
        let chat = history.create_category("聊天").expect("create chat");
        assert_eq!(work.kind, ClipboardCategoryKind::Custom);
        let renamed = history
            .rename_category(&work.id, "专注")
            .expect("rename work");
        assert_eq!(renamed.name, "专注");

        let order = [
            chat.id.clone(),
            BUILTIN_ALL_CATEGORY_ID.into(),
            renamed.id.clone(),
            BUILTIN_FAVORITE_CATEGORY_ID.into(),
            BUILTIN_TEXT_CATEGORY_ID.into(),
            BUILTIN_IMAGE_CATEGORY_ID.into(),
            BUILTIN_FILES_CATEGORY_ID.into(),
        ];
        history
            .reorder_categories(&order)
            .expect("reorder categories");
        history
            .delete_category(&chat.id)
            .expect("delete custom category");
        assert!(matches!(
            history.delete_category(BUILTIN_ALL_CATEGORY_ID),
            Err(ClipboardHistoryError::BuiltInCategoryImmutable(id))
                if id == BUILTIN_ALL_CATEGORY_ID
        ));
        drop(history);

        let history = ClipboardHistory::open(&database).expect("reopen history");
        let categories = history.categories().expect("reloaded categories");
        assert_eq!(
            categories
                .iter()
                .map(|category| category.id.as_str())
                .collect::<Vec<_>>(),
            [
                BUILTIN_ALL_CATEGORY_ID,
                renamed.id.as_str(),
                BUILTIN_FAVORITE_CATEGORY_ID,
                BUILTIN_TEXT_CATEGORY_ID,
                BUILTIN_IMAGE_CATEGORY_ID,
                BUILTIN_FILES_CATEGORY_ID,
            ]
        );
        assert_eq!(categories[1].name, "专注");
    }

    #[test]
    fn invalid_category_kind_is_rejected_instead_of_becoming_all() {
        let directory = tempfile::tempdir().expect("temp directory");
        let history =
            ClipboardHistory::open(directory.path().join("clipboard.db")).expect("open history");
        history
            .connection
            .execute("PRAGMA ignore_check_constraints = ON", [])
            .expect("allow corruption fixture");
        history
            .connection
            .execute(
                "UPDATE clipboard_categories SET kind = 'future-kind' WHERE id = ?1",
                [BUILTIN_ALL_CATEGORY_ID],
            )
            .expect("corrupt category kind");

        assert!(matches!(
            history.categories(),
            Err(ClipboardHistoryError::Database(
                rusqlite::Error::FromSqlConversionFailure(2, _, _)
            ))
        ));
    }

    #[test]
    fn custom_category_sources_filter_history_and_can_be_removed() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let category = history.create_category("聊天").expect("create category");

        let qq = history
            .capture(TextCapture {
                text: "message".into(),
                native_formats: Vec::new(),
                source: fixture_source_for("QQ.exe"),
                captured_at_unix_ms: 100,
            })
            .expect("capture QQ");
        let CaptureOutcome::Inserted(qq) = qq else {
            panic!("QQ capture must insert")
        };
        let zed = history
            .capture(TextCapture {
                text: "code".into(),
                native_formats: Vec::new(),
                source: fixture_source_for("Zed.exe"),
                captured_at_unix_ms: 200,
            })
            .expect("capture Zed");
        let CaptureOutcome::Inserted(zed) = zed else {
            panic!("Zed capture must insert")
        };

        history
            .add_category_source(&category.id, "qq.EXE")
            .expect("add QQ source");
        assert_eq!(
            history
                .query_category("", &category.id, 10)
                .expect("query QQ category")
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            [qq.id]
        );
        history
            .add_category_source(&category.id, "Zed.exe")
            .expect("add Zed source");
        assert_eq!(
            history
                .category_sources(&category.id)
                .expect("category sources"),
            ["qq.EXE", "Zed.exe"]
        );
        history
            .remove_category_source(&category.id, "QQ.exe")
            .expect("remove QQ source");
        assert_eq!(
            history
                .query_category("", &category.id, 10)
                .expect("query Zed category")
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>(),
            [zed.id]
        );
        drop(history);

        let history = ClipboardHistory::open(&database).expect("reopen history");
        assert_eq!(
            history
                .category_sources(&category.id)
                .expect("reloaded sources"),
            ["Zed.exe"]
        );
    }

    #[test]
    fn replacing_category_sources_persists_the_editor_selection_atomically() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let category = history.create_category("工作").expect("create category");

        history
            .add_category_source(&category.id, "old.exe")
            .expect("seed old source");
        let sources = history
            .replace_category_sources(
                &category.id,
                &[
                    "Editor.exe".into(),
                    "editor.EXE".into(),
                    "Browser.exe".into(),
                ],
            )
            .expect("replace category sources");

        assert_eq!(sources, ["Browser.exe", "Editor.exe"]);
        assert_eq!(
            history
                .category_sources(&category.id)
                .expect("read replaced sources"),
            ["Browser.exe", "Editor.exe"]
        );
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
    fn extended_search_is_opt_in_for_asset_and_source_metadata() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");

        history
            .capture_image(ImageCapture {
                image_path: "C:/captures/dragon-boss.png".into(),
                width: 640,
                height: 360,
                native_formats: Vec::new(),
                source: CaptureSource {
                    window_handle: 1,
                    process_id: 2,
                    process_name: "SnippingTool.exe".into(),
                    process_icon_path: String::new(),
                    window_title: "截图工具".into(),
                },
                captured_at_unix_ms: 100,
            })
            .expect("capture image");
        history
            .capture(TextCapture {
                text: "opaque content".into(),
                native_formats: Vec::new(),
                source: CaptureSource {
                    window_handle: 3,
                    process_id: 4,
                    process_name: "Rider64.exe".into(),
                    process_icon_path: String::new(),
                    window_title: "NxTools – clipboard.rs".into(),
                },
                captured_at_unix_ms: 200,
            })
            .expect("capture source fixture");

        assert!(
            history
                .search("dragon-boss", 10)
                .expect("plain search")
                .is_empty()
        );
        assert!(
            history
                .search("rider64", 10)
                .expect("plain search")
                .is_empty()
        );
        assert_eq!(
            history.search("opaque", 10).expect("content search").len(),
            1
        );
        assert_eq!(
            history
                .search_extended("dragon-boss", 10)
                .expect("image search")
                .len(),
            1
        );
        assert_eq!(
            history
                .search_extended("rider64", 10)
                .expect("source search")
                .len(),
            1
        );
        assert_eq!(
            history
                .search_extended("clipboard.rs", 10)
                .expect("title search")
                .len(),
            1
        );
    }

    #[test]
    fn category_pages_are_bounded_complete_and_report_the_filtered_total() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        for index in 0..105 {
            history
                .capture(TextCapture {
                    text: format!("paged item {index:03}"),
                    native_formats: Vec::new(),
                    source: fixture_source(),
                    captured_at_unix_ms: index + 1,
                })
                .expect("capture page fixture");
        }

        let first = history
            .query_category_page(
                "paged item",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                0,
                40,
            )
            .expect("first page");
        let second = history
            .query_category_page(
                "paged item",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                40,
                40,
            )
            .expect("second page");
        let last = history
            .query_category_page(
                "paged item",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                80,
                40,
            )
            .expect("last page");

        assert_eq!(
            (first.items.len(), second.items.len(), last.items.len()),
            (40, 40, 25)
        );
        assert_eq!((first.total, second.total, last.total), (105, 105, 105));
        assert!(first.has_more && second.has_more && !last.has_more);
        let ids = first
            .items
            .iter()
            .chain(&second.items)
            .chain(&last.items)
            .map(|item| item.id)
            .collect::<HashSet<_>>();
        assert_eq!(ids.len(), 105);
    }
    #[test]
    fn cursor_pages_match_offset_pages_and_preserve_has_more() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        for index in 0..105 {
            history
                .capture(TextCapture {
                    text: format!("cursor item {index:03}"),
                    native_formats: Vec::new(),
                    source: fixture_source(),
                    captured_at_unix_ms: index + 1,
                })
                .expect("capture cursor fixture");
        }

        let first = history
            .query_category_page(
                "cursor item",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                0,
                40,
            )
            .expect("cursor first page");
        let second = history
            .query_category_page(
                "cursor item",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                40,
                40,
            )
            .expect("cursor second offset page");
        let cursor = ClipboardPageCursor::from(first.items.last().expect("first cursor item"));
        let after = history
            .query_category_page_after(
                "cursor item",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                cursor,
                40,
                Some(first.total),
            )
            .expect("cursor second page");

        assert_eq!(after.items, second.items);
        assert_eq!(after.total, second.total);
        assert_eq!(after.has_more, second.has_more);
        history
            .mark_used(cursor.id, 10_000)
            .expect("move cursor item after its page");
        let after_mutation = history
            .query_category_page_after(
                "cursor item",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                cursor,
                40,
                Some(first.total),
            )
            .expect("cursor page after ordering mutation");
        assert_eq!(after_mutation.items, second.items);

        history
            .soft_delete(cursor.id, 10_001)
            .expect("delete cursor item");
        let after_delete = history
            .query_category_page_after(
                "cursor item",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                cursor,
                40,
                Some(first.total.saturating_sub(1)),
            )
            .expect("cursor page after cursor deletion");
        assert!(!after_delete.items.is_empty());
        assert!(!after_delete.items.iter().any(|item| item.id == cursor.id));
    }

    #[test]
    fn search_plan_uses_index_for_deep_pages() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let transaction = history.connection.transaction().expect("seed transaction");
        for index in 0..100_000_i64 {
            transaction
                .execute(
                    "INSERT INTO clipboard_items (
                        content_type, text, image_path, image_width, image_height,
                        file_paths_json, source_window_handle, source_process_id,
                        source_process_name, source_process_icon_path, source_window_title,
                        captured_at_unix_ms, last_used_at_unix_ms, copy_count, use_count,
                        is_favorite, is_pinned, deleted_at_unix_ms
                    ) VALUES ('text', ?1, '', 0, 0, '[]', 1, 1, 'benchmark', '',
                              '', ?2, ?2, 1, 0, 0, 0, 0)",
                    rusqlite::params![format!("deep page item {index:05}"), index],
                )
                .expect("seed clipboard row");
        }
        transaction.commit().expect("commit seed rows");

        let mut plan_statement = history
            .connection
            .prepare(
                "EXPLAIN QUERY PLAN
                 SELECT id FROM clipboard_items
                 WHERE deleted_at_unix_ms = 0
                   AND (?1 = '' OR instr(lower(text), lower(?1)) > 0)
                 ORDER BY is_pinned DESC, last_used_at_unix_ms DESC,
                          captured_at_unix_ms DESC, id DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .expect("prepare search query plan");
        let plan = plan_statement
            .query_map(rusqlite::params!["deep", 80_i64, 80_000_i64], |row| {
                row.get::<_, String>(3)
            })
            .expect("query search plan")
            .collect::<Result<Vec<_>, _>>()
            .expect("collect search plan");
        assert!(!plan.is_empty());
        assert!(
            plan.iter()
                .all(|detail| !detail.contains("USE TEMP B-TREE")),
            "search ordering should use the active usage index: {plan:?}"
        );

        for offset in [0_usize, 25_000, 80_000] {
            let page = history
                .query_category_page(
                    "deep",
                    BUILTIN_ALL_CATEGORY_ID,
                    ClipboardSearchScope::Content,
                    offset,
                    80,
                )
                .expect("query deep page");
            assert_eq!(page.items.len(), 80);
        }
    }

    #[test]
    fn history_counts_cover_every_visible_kind_without_loading_the_rows() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let CaptureOutcome::Inserted(text) = history
            .capture(TextCapture {
                text: "counted text".into(),
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 1,
            })
            .expect("capture text")
        else {
            panic!("text fixture must insert");
        };
        history
            .capture_image(ImageCapture {
                image_path: "C:/counted.png".into(),
                width: 10,
                height: 10,
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 2,
            })
            .expect("capture image");
        history
            .capture_files(FilesCapture {
                paths: vec!["C:/counted.txt".into()],
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 3,
            })
            .expect("capture files");
        history.soft_delete(text.id, 4).expect("delete text");

        assert_eq!(
            history.counts().expect("history counts"),
            ClipboardHistoryCounts {
                total: 2,
                text: 0,
                images: 1,
                files: 1,
            }
        );
    }

    #[test]
    fn portable_archive_moves_images_icons_and_native_formats_between_storage_roots() {
        let directory = tempfile::tempdir().expect("temp directory");
        let source_root = directory.path().join("source-assets");
        let target_root = directory.path().join("target-assets");
        std::fs::create_dir_all(source_root.join("native-formats")).expect("source asset root");
        let image = source_root.join("image.png");
        let icon = source_root.join("source-icon.png");
        let native = source_root.join("native-formats").join("format.bin");
        std::fs::write(&image, b"portable image bytes").expect("write image");
        std::fs::write(&icon, b"portable icon bytes").expect("write icon");
        std::fs::write(&native, b"portable native format").expect("write native format");

        let source_db = directory.path().join("source.db");
        let mut source = ClipboardHistory::open(&source_db).expect("open source history");
        source
            .capture_image(ImageCapture {
                image_path: image.to_string_lossy().into_owned(),
                width: 320,
                height: 180,
                native_formats: vec![ClipboardNativeFormat {
                    format_id: 8,
                    format_name: "CF_DIB".into(),
                    data_path: native.to_string_lossy().into_owned(),
                    size_bytes: 22,
                    sort_order: 0,
                }],
                source: CaptureSource {
                    process_icon_path: icon.to_string_lossy().into_owned(),
                    ..fixture_source_for("source-app")
                },
                captured_at_unix_ms: 100,
            })
            .expect("capture image fixture");
        source
            .capture(TextCapture {
                text: "portable text".into(),
                native_formats: Vec::new(),
                source: fixture_source_for("editor"),
                captured_at_unix_ms: 200,
            })
            .expect("capture text fixture");
        let archive = directory.path().join("portable.nxtclip");

        let exported = source
            .export_archive(&archive)
            .expect("export portable archive");
        assert_eq!(exported.item_count, 2);
        assert_eq!(exported.asset_count, 3);

        let target_db = directory.path().join("target.db");
        let mut target = ClipboardHistory::open(&target_db).expect("open target history");
        let imported = target
            .import_archive(&archive, &target_root)
            .expect("import portable archive");
        assert_eq!((imported.inserted, imported.updated), (2, 0));
        let items = target.recent(10).expect("target history");
        assert_eq!(items.len(), 2);
        let imported_image = items
            .iter()
            .find(|item| item.kind == ClipboardKind::Image)
            .expect("imported image");
        assert!(Path::new(&imported_image.image_path).starts_with(&target_root));
        assert_eq!(
            std::fs::read(&imported_image.image_path).expect("read imported image"),
            b"portable image bytes"
        );
        assert!(Path::new(&imported_image.source.process_icon_path).starts_with(&target_root));
        let formats = target
            .native_formats(imported_image.id)
            .expect("imported formats");
        assert_eq!(formats.len(), 1);
        assert!(Path::new(&formats[0].data_path).starts_with(&target_root));
        assert_eq!(
            std::fs::read(&formats[0].data_path).expect("read imported format"),
            b"portable native format"
        );

        let reimported = target
            .import_archive(&archive, &target_root)
            .expect("reimport portable archive");
        assert_eq!((reimported.inserted, reimported.updated), (0, 2));
        assert_eq!(target.recent(10).expect("deduplicated history").len(), 2);
    }

    #[test]
    fn archive_import_rejects_asset_paths_that_escape_the_managed_directory() {
        use std::io::Write as _;

        let directory = tempfile::tempdir().expect("temp directory");
        let archive_path = directory.path().join("unsafe.nxtclip");
        let output = std::fs::File::create(&archive_path).expect("create archive");
        let mut archive = zip::ZipWriter::new(output);
        archive
            .start_file("manifest.json", zip::write::SimpleFileOptions::default())
            .expect("start manifest");
        archive
            .write_all(
                br#"{"version":1,"items":[{"kind":"image","text":"","image_entry":"images/../../escape.png","image_width":1,"image_height":1,"file_paths":[],"source":{"window_handle":0,"process_id":0,"process_name":"test","icon_entry":null,"window_title":""},"captured_at_unix_ms":1,"is_favorite":false,"is_pinned":false,"formats":[]}]}"#,
            )
            .expect("write manifest");
        archive.finish().expect("finish archive");

        let mut history =
            ClipboardHistory::open(directory.path().join("history.db")).expect("open history");
        let asset_root = directory.path().join("assets");
        let error = history
            .import_archive(&archive_path, &asset_root)
            .expect_err("unsafe entry must be rejected");

        assert!(matches!(error, ClipboardArchiveError::Invalid(_)));
        assert!(!directory.path().join("escape.png").exists());
        assert!(history.recent(10).expect("empty history").is_empty());
    }

    // ZIP fixtures use the same manifest and asset layout as the exported archive.
    #[test]
    fn failed_archive_import_removes_only_owned_new_assets_without_rows() {
        use sha2::Digest as _;
        use std::io::Write as _;
        let directory = tempfile::tempdir().expect("create archive fixture directory");
        let db = directory.path().join("history.db");
        let assets = directory.path().join("assets");
        std::fs::create_dir_all(&assets).expect("create shared asset directory");
        let existing_bytes = b"shared asset";
        let digest = format!("{:x}", sha2::Sha256::digest(existing_bytes));
        let existing = assets.join(format!("{digest}.png"));
        std::fs::write(&existing, existing_bytes).expect("write shared asset");
        let archive_path = directory.path().join("partial.nxtclip");
        let mut zip = zip::ZipWriter::new(
            std::fs::File::create(&archive_path).expect("create archive fixture"),
        );
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", options)
            .expect("start archive manifest");
        let item = |image: &str, time: i64| {
            serde_json::json!({
                "kind":"image", "text":"", "image_entry":image, "image_width":1,
                "image_height":1, "file_paths":[], "source":{"window_handle":0,
                "process_id":0,"process_name":"test","icon_entry":null,"window_title":""},
                "captured_at_unix_ms":time,"is_favorite":false,"is_pinned":false,"formats":[]
            })
        };
        let manifest = serde_json::json!({"version":1,"items":[
            item("images/existing.png", 1), item("images/new.png", 2),
            item("images/missing.png", 3)]});
        zip.write_all(manifest.to_string().as_bytes())
            .expect("write archive manifest");
        zip.start_file("images/existing.png", options)
            .expect("start shared asset entry");
        zip.write_all(existing_bytes)
            .expect("write shared asset entry");
        zip.start_file("images/new.png", options)
            .expect("start owned asset entry");
        zip.write_all(b"new asset")
            .expect("write owned asset entry");
        zip.finish().expect("finish archive fixture");
        let mut history = ClipboardHistory::open(&db).expect("open archive history");
        assert!(history.import_archive(&archive_path, &assets).is_err());
        assert_eq!(
            history
                .counts()
                .expect("count rows after failed import")
                .total,
            0
        );
        assert_eq!(
            std::fs::read(existing).expect("read retained shared asset"),
            existing_bytes
        );
        assert_eq!(
            std::fs::read_dir(&assets)
                .expect("list remaining assets")
                .count(),
            2
        ); // existing + native-formats
        assert!(
            !std::fs::read_dir(&assets)
                .expect("list import temporary entries")
                .any(|entry| {
                    entry
                        .expect("read asset directory entry")
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".import-")
                })
        );
    }

    #[test]
    fn archive_import_late_database_failure_reports_committed_rows_explicitly() {
        use std::io::Write as _;
        let directory = tempfile::tempdir().expect("create late-failure fixture directory");
        let archive_path = directory.path().join("late.nxtclip");
        let mut zip = zip::ZipWriter::new(
            std::fs::File::create(&archive_path).expect("create late-failure archive"),
        );
        zip.start_file("manifest.json", zip::write::SimpleFileOptions::default())
            .expect("start late-failure manifest");
        let item = |text: &str, time: i64| {
            serde_json::json!({
                "kind":"text", "text":text, "image_entry":null, "image_width":0,
                "image_height":0, "file_paths":[], "source":{"window_handle":0,
                "process_id":0,"process_name":"test","icon_entry":null,"window_title":""},
                "captured_at_unix_ms":time,"is_favorite":false,"is_pinned":false,"formats":[]
            })
        };
        let mut first = item("first", 1);
        first["source"]["icon_entry"] = serde_json::json!("icons/first.png");
        let mut second = item("second", 2);
        second["source"]["icon_entry"] = serde_json::json!("icons/second.png");
        zip.write_all(
            serde_json::json!({"version":1,"items":[first, second]})
                .to_string()
                .as_bytes(),
        )
        .expect("write late-failure manifest");
        zip.start_file("icons/first.png", zip::write::SimpleFileOptions::default())
            .expect("start first icon entry");
        zip.write_all(b"first asset").expect("write first icon");
        zip.start_file("icons/second.png", zip::write::SimpleFileOptions::default())
            .expect("start second icon entry");
        zip.write_all(b"second orphan asset")
            .expect("write second icon");
        zip.finish().expect("finish late-failure archive");
        let mut history = ClipboardHistory::open(directory.path().join("history.db"))
            .expect("open late-failure history");
        history.connection.execute_batch("CREATE TRIGGER reject_second BEFORE INSERT ON clipboard_items WHEN NEW.text = 'second' BEGIN SELECT RAISE(ABORT, 'synthetic failure'); END;").expect("install late-insert failure trigger");
        let error = history
            .import_archive(&archive_path, &directory.path().join("assets"))
            .expect_err("second row must hit the synthetic insert failure");
        assert!(matches!(
            error,
            ClipboardArchiveError::Partial {
                inserted: 1,
                updated: 0,
                ..
            }
        ));
        assert_eq!(
            history
                .counts()
                .expect("count partially committed rows")
                .total,
            1
        );
        let references = history
            .referenced_asset_paths()
            .expect("query committed asset references");
        let actual = std::fs::read_dir(directory.path().join("assets"))
            .expect("list assets after partial import")
            .flatten()
            .filter(|entry| entry.path().is_file())
            .map(|entry| entry.path().to_string_lossy().into_owned())
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(
            actual, references,
            "late failure must remove new unreferenced resources, retain committed row assets"
        );
    }

    #[test]
    fn archive_import_skipped_row_does_not_extract_its_assets() {
        use std::io::Write as _;
        let directory = tempfile::tempdir().expect("create skipped-row fixture directory");
        let archive_path = directory.path().join("skipped.nxtclip");
        let mut zip = zip::ZipWriter::new(
            std::fs::File::create(&archive_path).expect("create skipped-row archive"),
        );
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("manifest.json", options)
            .expect("start skipped-row manifest");
        zip.write_all(br#"{"version":1,"items":[{"kind":"unknown","text":"","image_entry":"images/orphan.png","image_width":1,"image_height":1,"file_paths":[],"source":{"window_handle":0,"process_id":0,"process_name":"","icon_entry":null,"window_title":""},"captured_at_unix_ms":1,"is_favorite":false,"is_pinned":false,"formats":[]}]}"#).expect("write skipped-row manifest");
        zip.start_file("images/orphan.png", options)
            .expect("start orphan asset entry");
        zip.write_all(b"orphan").expect("write orphan asset entry");
        zip.finish().expect("finish skipped-row archive");
        let mut history = ClipboardHistory::open(directory.path().join("history.db"))
            .expect("open skipped-row history");
        let root = directory.path().join("assets");
        let report = history
            .import_archive(&archive_path, &root)
            .expect("import skipped-row archive");
        assert_eq!(report.skipped, 1);
        assert_eq!(report.assets_written, 0);
        assert_eq!(
            std::fs::read_dir(root)
                .expect("list skipped-row assets")
                .count(),
            1
        ); // native-formats only
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
                CaptureOutcome::IgnoredEmpty => panic!("fixture must not be empty"),
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

    #[test]
    fn retention_soft_deletes_expired_items_but_protects_favorites_and_pins() {
        const DAY_MS: i64 = 86_400_000;
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let mut captured = Vec::new();
        for (text, captured_at) in [
            ("expired normal", DAY_MS),
            ("expired favorite", DAY_MS + 1),
            ("expired pinned", DAY_MS + 2),
            ("recent normal", 35 * DAY_MS),
        ] {
            let CaptureOutcome::Inserted(item) = history
                .capture(TextCapture {
                    text: text.into(),
                    native_formats: Vec::new(),
                    source: fixture_source(),
                    captured_at_unix_ms: captured_at,
                })
                .expect("capture retention fixture")
            else {
                panic!("fixture must insert")
            };
            captured.push(item);
        }
        history
            .set_favorite(captured[1].id, true)
            .expect("favorite expired item");
        history
            .set_pinned(captured[2].id, true)
            .expect("pin expired item");

        let report = history
            .cleanup(
                ClipboardRetentionPolicy {
                    max_items: 1000,
                    retention_days: 30,
                },
                40 * DAY_MS,
            )
            .expect("apply retention");

        assert_eq!(report.soft_deleted, 1);
        let visible = history.recent(10).expect("visible history");
        assert_eq!(visible.len(), 3);
        assert!(visible.iter().any(|item| item.text == "expired favorite"));
        assert!(visible.iter().any(|item| item.text == "expired pinned"));
        assert!(visible.iter().any(|item| item.text == "recent normal"));
        assert!(history.restore(captured[0].id).is_ok());
    }

    #[test]
    fn retention_soft_deletes_oldest_normal_items_over_the_limit() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        for index in 0..102 {
            history
                .capture(TextCapture {
                    text: format!("retention item {index}"),
                    native_formats: Vec::new(),
                    source: fixture_source(),
                    captured_at_unix_ms: index + 1,
                })
                .expect("capture retention fixture");
        }

        let report = history
            .cleanup(
                ClipboardRetentionPolicy {
                    max_items: 100,
                    retention_days: 3650,
                },
                1_000,
            )
            .expect("apply retention");

        assert_eq!(report.soft_deleted, 2);
        let visible = history.recent(200).expect("visible history");
        assert_eq!(visible.len(), 100);
        assert_eq!(
            visible.last().map(|item| item.text.as_str()),
            Some("retention item 2")
        );
    }

    #[test]
    fn retention_physically_purges_old_deleted_rows_and_reports_image_files() {
        const DAY_MS: i64 = 86_400_000;
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let image_path = directory.path().join("old-image.png");
        let CaptureOutcome::Inserted(image) = history
            .capture_image(ImageCapture {
                image_path: image_path.to_string_lossy().into_owned(),
                width: 100,
                height: 80,
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: DAY_MS,
            })
            .expect("capture image fixture")
        else {
            panic!("fixture must insert")
        };
        history
            .soft_delete(image.id, 2 * DAY_MS)
            .expect("soft delete image");

        let report = history
            .cleanup(
                ClipboardRetentionPolicy {
                    max_items: 1000,
                    retention_days: 30,
                },
                40 * DAY_MS,
            )
            .expect("apply purge");

        assert_eq!(report.purged, 1);
        assert_eq!(
            report.purged_asset_paths,
            vec![image_path.to_string_lossy()]
        );
        let remaining_rows = history
            .connection
            .query_row(
                "SELECT COUNT(*) FROM clipboard_items WHERE id = ?1",
                [image.id],
                |row| row.get::<_, i64>(0),
            )
            .expect("count purged rows");
        assert_eq!(remaining_rows, 0);
    }

    #[test]
    fn deleted_row_purge_uses_seven_day_floor_or_longer_retention() {
        const DAY_MS: i64 = 86_400_000;
        for (days, before, after) in [(1, 6, 8), (7, 6, 8), (14, 10, 16), (30, 10, 32)] {
            let directory = tempfile::tempdir().expect("retention case directory");
            let mut history = ClipboardHistory::open(directory.path().join("history.db"))
                .expect("open retention fixture");
            let CaptureOutcome::Inserted(row) = history
                .capture(TextCapture {
                    text: format!("retention {days}"),
                    native_formats: Vec::new(),
                    source: fixture_source(),
                    captured_at_unix_ms: DAY_MS,
                })
                .expect("capture retention fixture")
            else {
                panic!("must insert")
            };
            history
                .soft_delete(row.id, 2 * DAY_MS)
                .expect("soft delete");
            let policy = ClipboardRetentionPolicy {
                max_items: 1000,
                retention_days: days,
            };
            assert_eq!(
                history
                    .cleanup(policy, (2 + before) * DAY_MS)
                    .expect("clean retention fixture before expiry")
                    .purged,
                0,
                "{days}-day policy purged recoverable row too soon"
            );
            assert_eq!(
                history
                    .cleanup(policy, (2 + after) * DAY_MS)
                    .expect("clean retention fixture after expiry")
                    .purged,
                1,
                "{days}-day policy failed to purge expired row"
            );
        }
    }

    #[test]
    fn retention_reports_only_asset_files_without_remaining_database_references() {
        const DAY_MS: i64 = 86_400_000;
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        let common_image = "C:/assets/common.png";
        let common_icon = "C:/assets/common-icon.png";
        let common_native = "C:/assets/common-format.bin";
        let unique_image = "C:/assets/unique.png";
        let unique_icon = "C:/assets/unique-icon.png";
        let unique_native = "C:/assets/unique-format.bin";
        let format = |path: &str, id: u32| ClipboardNativeFormat {
            format_id: id,
            format_name: format!("format-{id}"),
            data_path: path.into(),
            size_bytes: 10,
            sort_order: id,
        };
        let source = |icon: &str| CaptureSource {
            process_icon_path: icon.into(),
            ..fixture_source()
        };

        let CaptureOutcome::Inserted(shared_deleted) = history
            .capture_image(ImageCapture {
                image_path: common_image.into(),
                width: 100,
                height: 80,
                native_formats: vec![format(common_native, 1)],
                source: source(common_icon),
                captured_at_unix_ms: DAY_MS,
            })
            .expect("capture shared deleted row")
        else {
            panic!("fixture must insert");
        };
        let CaptureOutcome::Inserted(shared_live) = history
            .capture_image(ImageCapture {
                image_path: common_image.into(),
                width: 100,
                height: 80,
                native_formats: vec![
                    format(common_native, 1),
                    format("C:/assets/live-extra.bin", 2),
                ],
                source: source(common_icon),
                captured_at_unix_ms: DAY_MS + 1,
            })
            .expect("capture shared live row")
        else {
            panic!("fixture must insert");
        };
        let CaptureOutcome::Inserted(unique_deleted) = history
            .capture_image(ImageCapture {
                image_path: unique_image.into(),
                width: 120,
                height: 90,
                native_formats: vec![format(unique_native, 3)],
                source: source(unique_icon),
                captured_at_unix_ms: DAY_MS + 2,
            })
            .expect("capture unique deleted row")
        else {
            panic!("fixture must insert");
        };
        history
            .set_favorite(shared_live.id, true)
            .expect("protect shared live row");
        history
            .soft_delete(shared_deleted.id, 2 * DAY_MS)
            .expect("delete shared row");
        history
            .soft_delete(unique_deleted.id, 2 * DAY_MS)
            .expect("delete unique row");

        let report = history
            .cleanup(
                ClipboardRetentionPolicy {
                    max_items: 1000,
                    retention_days: 30,
                },
                40 * DAY_MS,
            )
            .expect("purge old deleted rows");
        let assets = report
            .purged_asset_paths
            .into_iter()
            .collect::<HashSet<_>>();

        assert_eq!(
            assets,
            HashSet::from([
                unique_image.to_owned(),
                unique_icon.to_owned(),
                unique_native.to_owned(),
            ])
        );
        assert!(!assets.contains(common_image));
        assert!(!assets.contains(common_icon));
        assert!(!assets.contains(common_native));
    }

    #[test]
    fn database_compaction_creates_a_readable_backup_before_vacuum() {
        let directory = tempfile::tempdir().expect("temp directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open history");
        history
            .capture(TextCapture {
                text: "survives database maintenance".into(),
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 42,
            })
            .expect("capture maintenance fixture");

        let report = history
            .compact_database(&directory.path().join("backups"))
            .expect("compact database");

        assert!(report.backup_path.is_file());
        assert!(report.before_bytes > 0);
        assert_eq!(
            report.after_bytes,
            std::fs::metadata(&database)
                .expect("read compacted database metadata")
                .len()
        );
        let backup = ClipboardHistory::open(&report.backup_path).expect("open backup");
        assert_eq!(
            backup.recent(10).expect("read backup")[0].text,
            "survives database maintenance"
        );
    }

    #[test]
    fn storage_migration_preserves_history_metadata_and_managed_assets() {
        let directory = tempfile::tempdir().expect("temp directory");
        let source_root = directory.path().join("source");
        let source_assets = source_root.join("clipboard-images");
        std::fs::create_dir_all(source_assets.join("native-formats")).expect("source assets");
        let image = source_assets.join("image.png");
        let icon = source_assets.join("source-icon.png");
        let native = source_assets.join("native-formats/format.bin");
        std::fs::write(&image, b"migration image").expect("image fixture");
        std::fs::write(&icon, b"migration icon").expect("icon fixture");
        std::fs::write(&native, b"migration native").expect("native fixture");
        let mut source =
            ClipboardHistory::open(source_root.join("clipboard.db")).expect("source history");
        let CaptureOutcome::Inserted(item) = source
            .capture_image(ImageCapture {
                image_path: image.to_string_lossy().into_owned(),
                width: 200,
                height: 100,
                native_formats: vec![ClipboardNativeFormat {
                    format_id: 8,
                    format_name: "CF_DIB".into(),
                    data_path: native.to_string_lossy().into_owned(),
                    size_bytes: 16,
                    sort_order: 0,
                }],
                source: CaptureSource {
                    process_icon_path: icon.to_string_lossy().into_owned(),
                    ..fixture_source_for("migration-app")
                },
                captured_at_unix_ms: 100,
            })
            .expect("capture migration fixture")
        else {
            panic!("fixture must insert");
        };
        source
            .set_favorite(item.id, true)
            .expect("favorite fixture");
        source.set_pinned(item.id, true).expect("pin fixture");
        let category = source
            .create_category("工作")
            .expect("custom category fixture");

        let destination = directory.path().join("destination");
        let report = source
            .migrate_storage(&source_assets, &destination)
            .expect("migrate storage");

        assert_eq!(report.item_count, 1);
        assert_eq!(report.asset_count, 3);
        assert!(image.is_file(), "migration must not remove source data");
        let target = ClipboardHistory::open(destination.join("clipboard.db"))
            .expect("open migrated history");
        let migrated = &target.recent(10).expect("migrated items")[0];
        assert!(migrated.is_favorite && migrated.is_pinned);
        assert!(Path::new(&migrated.image_path).starts_with(destination.join("clipboard-images")));
        assert!(
            Path::new(&migrated.source.process_icon_path)
                .starts_with(destination.join("clipboard-images"))
        );
        let formats = target
            .native_formats(migrated.id)
            .expect("migrated formats");
        assert!(Path::new(&formats[0].data_path).starts_with(destination.join("clipboard-images")));
        assert!(
            target
                .categories()
                .expect("migrated categories")
                .iter()
                .any(|candidate| candidate.id == category.id)
        );
    }

    #[test]
    fn storage_migration_rejects_a_nonempty_destination_without_changing_either_side() {
        let directory = tempfile::tempdir().expect("temp directory");
        let source_root = directory.path().join("source");
        let source_assets = source_root.join("clipboard-images");
        std::fs::create_dir_all(&source_assets).expect("source assets");
        let mut source =
            ClipboardHistory::open(source_root.join("clipboard.db")).expect("source history");
        source
            .capture(TextCapture {
                text: "source remains current".into(),
                native_formats: Vec::new(),
                source: fixture_source(),
                captured_at_unix_ms: 1,
            })
            .expect("source fixture");
        let destination = directory.path().join("occupied");
        std::fs::create_dir_all(&destination).expect("destination");
        let marker = destination.join("do-not-touch.txt");
        std::fs::write(&marker, b"existing data").expect("marker");

        let result = source.migrate_storage(&source_assets, &destination);

        assert!(matches!(
            result,
            Err(ClipboardStorageError::InvalidDestination(_))
        ));
        assert_eq!(
            std::fs::read(&marker).expect("marker remains"),
            b"existing data"
        );
        assert_eq!(
            source.recent(10).expect("source remains readable")[0].text,
            "source remains current"
        );
    }
    const QUERY_BENCHMARK_PAGE_SIZE: usize = 40;

    struct QueryBenchmarkFixture {
        _directory: tempfile::TempDir,
        history: ClipboardHistory,
        content_matches: usize,
        extended_matches: usize,
    }

    fn query_benchmark_fixture(row_count: usize) -> QueryBenchmarkFixture {
        let directory = tempfile::tempdir().expect("create query benchmark directory");
        let database = directory.path().join("clipboard.db");
        let mut history = ClipboardHistory::open(&database).expect("open query benchmark history");
        let transaction = history
            .connection
            .transaction()
            .expect("start query benchmark transaction");
        let mut insert = transaction
            .prepare(
                "INSERT INTO clipboard_items (
                    content_type, text, image_path, image_width, image_height,
                    file_paths_json, source_window_handle, source_process_id,
                    source_process_name, source_process_icon_path, source_window_title,
                    captured_at_unix_ms, last_used_at_unix_ms, copy_count, use_count,
                    is_favorite, is_pinned, deleted_at_unix_ms
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, 1, ?7, '', ?8, ?9, ?9, 1, 0, 0, 0, 0)",
            )
            .expect("prepare query benchmark insert");
        let mut content_matches = 0;
        let mut extended_matches = 0;

        for index in 0..row_count {
            let content_type = match index % 3 {
                0 => "text",
                1 => "image",
                _ => "files",
            };
            let content_match = content_type == "text" && index % 11 == 0;
            let metadata_match = !content_match && index % 17 == 1;
            if content_match {
                content_matches += 1;
            }
            if content_match || metadata_match {
                extended_matches += 1;
            }

            let text = if content_type == "text" {
                if content_match {
                    format!("benchmark query-marker content row {index}")
                } else {
                    format!("benchmark content row {index}")
                }
            } else {
                String::new()
            };
            let (image_path, image_width, image_height, file_paths_json) = match content_type {
                "image" => (
                    if metadata_match {
                        format!("C:/benchmark/query-marker-image-{index}.png")
                    } else {
                        format!("C:/benchmark/image-{index}.png")
                    },
                    640,
                    360,
                    "[]".to_owned(),
                ),
                "files" => (
                    String::new(),
                    0,
                    0,
                    if metadata_match {
                        format!("[\"C:/benchmark/query-marker-file-{index}.txt\"]")
                    } else {
                        format!("[\"C:/benchmark/file-{index}.txt\"]")
                    },
                ),
                _ => (String::new(), 0, 0, "[]".to_owned()),
            };
            let window_title = if metadata_match && content_type == "text" {
                format!("benchmark query-marker metadata window {index}")
            } else {
                format!("benchmark window {index}")
            };

            insert
                .execute(rusqlite::params![
                    content_type,
                    text,
                    image_path,
                    image_width,
                    image_height,
                    file_paths_json,
                    "clipboard-benchmark",
                    window_title,
                    index as i64 + 1,
                ])
                .expect("insert query benchmark row");
        }
        drop(insert);
        transaction.commit().expect("commit query benchmark rows");

        QueryBenchmarkFixture {
            _directory: directory,
            history,
            content_matches,
            extended_matches,
        }
    }

    fn benchmark_matching_count(
        history: &ClipboardHistory,
        query: &str,
        search_scope: ClipboardSearchScope,
    ) -> usize {
        history
            .connection
            .query_row(
                "SELECT COUNT(*)
                 FROM clipboard_items
                 WHERE deleted_at_unix_ms = 0
                   AND (?1 IS NULL OR content_type = ?1)
                   AND (?2 = 0 OR is_favorite = 1)
                   AND (?3 IS NULL OR EXISTS (
                        SELECT 1 FROM clipboard_category_sources AS category_source
                        WHERE category_source.category_id = ?3
                          AND category_source.process_name = clipboard_items.source_process_name
                   ))
                   AND (?4 = ''
                        OR instr(lower(text), lower(?4)) > 0
                        OR (?5 = 1 AND (
                            instr(lower(image_path), lower(?4)) > 0
                            OR instr(lower(file_paths_json), lower(?4)) > 0
                            OR instr(lower(source_process_name), lower(?4)) > 0
                            OR instr(lower(source_window_title), lower(?4)) > 0
                        )))",
                rusqlite::params![
                    Option::<&str>::None,
                    false,
                    Option::<&str>::None,
                    query,
                    search_scope == ClipboardSearchScope::Extended,
                ],
                |row| row.get::<_, i64>(0),
            )
            .expect("count query benchmark rows") as usize
    }

    fn query_benchmark_deep_offset(total: usize) -> usize {
        ((total - QUERY_BENCHMARK_PAGE_SIZE) / QUERY_BENCHMARK_PAGE_SIZE)
            * QUERY_BENCHMARK_PAGE_SIZE
    }

    #[test]
    fn clipboard_history_query_benchmark_emits_json_for_10k_and_100k_fixtures() {
        for row_count in [10_000, 100_000] {
            let fixture = query_benchmark_fixture(row_count);
            let query_cases = [
                ("recent", "", ClipboardSearchScope::Content, row_count),
                (
                    "content",
                    "query-marker",
                    ClipboardSearchScope::Content,
                    fixture.content_matches,
                ),
                (
                    "extended",
                    "query-marker",
                    ClipboardSearchScope::Extended,
                    fixture.extended_matches,
                ),
            ];

            assert!(fixture.content_matches < fixture.extended_matches);
            assert!(fixture.extended_matches <= row_count);

            for (case, query, search_scope, expected_total) in query_cases {
                assert!(expected_total > QUERY_BENCHMARK_PAGE_SIZE);
                let offsets = [0, query_benchmark_deep_offset(expected_total)];
                let scope_name = match search_scope {
                    ClipboardSearchScope::Content => "content",
                    ClipboardSearchScope::Extended => "extended",
                };

                for offset in offsets {
                    let warm_page = fixture
                        .history
                        .query_category_page(
                            query,
                            BUILTIN_ALL_CATEGORY_ID,
                            search_scope,
                            offset,
                            QUERY_BENCHMARK_PAGE_SIZE,
                        )
                        .expect("warm query benchmark page");
                    let warm_items = fixture
                        .history
                        .query_items(
                            query,
                            None,
                            false,
                            None,
                            search_scope,
                            offset,
                            QUERY_BENCHMARK_PAGE_SIZE,
                        )
                        .expect("warm query benchmark items");
                    let warm_count =
                        benchmark_matching_count(&fixture.history, query, search_scope);
                    assert_eq!(warm_page.total, expected_total);
                    assert_eq!(warm_count, expected_total);
                    assert_eq!(warm_page.items, warm_items);

                    let page_started = std::time::Instant::now();
                    let page = fixture
                        .history
                        .query_category_page(
                            query,
                            BUILTIN_ALL_CATEGORY_ID,
                            search_scope,
                            offset,
                            QUERY_BENCHMARK_PAGE_SIZE,
                        )
                        .expect("measure query benchmark page");
                    let page_with_count_us = page_started.elapsed().as_micros();
                    let page_without_count_started = std::time::Instant::now();
                    let page_without_count = fixture
                        .history
                        .query_category_page_with_total(
                            query,
                            BUILTIN_ALL_CATEGORY_ID,
                            search_scope,
                            offset,
                            QUERY_BENCHMARK_PAGE_SIZE,
                            Some(expected_total),
                        )
                        .expect("measure query benchmark page without count");
                    let page_without_count_us = page_without_count_started.elapsed().as_micros();
                    let page_after_cursor_us = if offset == 0 {
                        None
                    } else {
                        let previous_page = fixture
                            .history
                            .query_category_page_with_total(
                                query,
                                BUILTIN_ALL_CATEGORY_ID,
                                search_scope,
                                offset.saturating_sub(QUERY_BENCHMARK_PAGE_SIZE),
                                QUERY_BENCHMARK_PAGE_SIZE,
                                Some(expected_total),
                            )
                            .expect("load cursor benchmark predecessor");
                        let cursor = ClipboardPageCursor::from(
                            previous_page
                                .items
                                .last()
                                .expect("cursor benchmark predecessor"),
                        );
                        let warm_after = fixture
                            .history
                            .query_category_page_after(
                                query,
                                BUILTIN_ALL_CATEGORY_ID,
                                search_scope,
                                cursor,
                                QUERY_BENCHMARK_PAGE_SIZE,
                                Some(expected_total),
                            )
                            .expect("warm cursor benchmark page");
                        assert_eq!(warm_after.items, page.items);
                        let cursor_started = std::time::Instant::now();
                        let after = fixture
                            .history
                            .query_category_page_after(
                                query,
                                BUILTIN_ALL_CATEGORY_ID,
                                search_scope,
                                cursor,
                                QUERY_BENCHMARK_PAGE_SIZE,
                                Some(expected_total),
                            )
                            .expect("measure cursor benchmark page");
                        assert_eq!(after.items, page.items);
                        assert_eq!(after.total, page.total);
                        assert_eq!(after.has_more, page.has_more);
                        Some(cursor_started.elapsed().as_micros())
                    };

                    let query_started = std::time::Instant::now();
                    let items = fixture
                        .history
                        .query_items(
                            query,
                            None,
                            false,
                            None,
                            search_scope,
                            offset,
                            QUERY_BENCHMARK_PAGE_SIZE,
                        )
                        .expect("measure query benchmark items");
                    let query_us = query_started.elapsed().as_micros();

                    let count_started = std::time::Instant::now();
                    let count = benchmark_matching_count(&fixture.history, query, search_scope);
                    let count_us = count_started.elapsed().as_micros();

                    assert_eq!(page.total, expected_total);
                    assert_eq!(page_without_count.total, expected_total);
                    assert_eq!(page_without_count.items, page.items);
                    assert_eq!(page_without_count.has_more, page.has_more);
                    assert_eq!(page.total, count);
                    assert_eq!(page.items, items);
                    assert!(items.len() <= QUERY_BENCHMARK_PAGE_SIZE);
                    assert_eq!(
                        page.has_more,
                        offset.saturating_add(items.len()) < expected_total
                    );
                    let item_ids = items.iter().map(|item| item.id).collect::<HashSet<_>>();
                    assert_eq!(item_ids.len(), items.len());

                    println!(
                        "{}",
                        serde_json::json!({
                            "benchmark": "clipboard_history_query",
                            "fixture_rows": row_count,
                            "case": case,
                            "scope": scope_name,
                            "offset": offset,
                            "limit": QUERY_BENCHMARK_PAGE_SIZE,
                            "items": items.len(),
                            "total": page.total,
                            "has_more": page.has_more,
                            "query_us": query_us,
                            "count_us": count_us,
                            "page_with_count_us": page_with_count_us,
                            "page_without_count_us": page_without_count_us,
                            "page_after_cursor_us": page_after_cursor_us,
                        })
                    );
                }
            }
        }
    }
}
