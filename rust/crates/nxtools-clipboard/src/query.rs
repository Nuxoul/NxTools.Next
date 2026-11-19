use rusqlite::{Row, params};

use super::{
    CLIPBOARD_SUMMARY_CHARACTERS, ClipboardCategoryKind, ClipboardHistory, ClipboardHistoryCounts,
    ClipboardHistoryError, ClipboardHistoryPage, ClipboardItem, ClipboardItemSummary,
    ClipboardKind, ClipboardPageCursor, ClipboardSearchScope, ClipboardSummaryPage, map_item,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClipboardHistoryFilter {
    pub source_process: Option<String>,
    /// Inclusive start, exclusive end; both refer to capture time, not last use.
    pub captured_between_unix_ms: Option<(i64, i64)>,
}

// One filtered/order owner for the original full and bounded summary projections.
const FILTER: &str = "deleted_at_unix_ms = 0
    AND (?1 IS NULL OR content_type = ?1)
    AND (?2 = 0 OR is_favorite = 1)
    AND (?3 IS NULL OR EXISTS (
        SELECT 1 FROM clipboard_category_sources AS category_source
        WHERE category_source.category_id = ?3
          AND category_source.process_name = clipboard_items.source_process_name
    ))
    AND (?6 IS NULL OR source_process_name = ?6 COLLATE NOCASE)
    AND (?7 IS NULL OR captured_at_unix_ms >= ?7)
    AND (?8 IS NULL OR captured_at_unix_ms < ?8)
    AND (?4 = '' OR instr(lower(text), lower(?4)) > 0
        OR (?5 = 1 AND (instr(lower(image_path), lower(?4)) > 0
            OR instr(lower(file_paths_json), lower(?4)) > 0
            OR instr(lower(source_process_name), lower(?4)) > 0
            OR instr(lower(source_window_title), lower(?4)) > 0)))";
const ORDER: &str = "is_pinned DESC, last_used_at_unix_ms DESC, captured_at_unix_ms DESC, id DESC";
const FULL_COLUMNS: &str = "id, content_type, text, image_path, image_width, image_height,
    file_paths_json, source_window_handle, source_process_id, source_process_name,
    source_process_icon_path, source_window_title, captured_at_unix_ms,
    last_used_at_unix_ms, copy_count, is_favorite, is_pinned, deleted_at_unix_ms";
const SUMMARY_COLUMNS: &str = "id, content_type, nxt_clip_summary(text), image_path,
    image_width, image_height, file_paths_json, source_window_handle, source_process_id,
    source_process_name, source_process_icon_path, source_window_title, captured_at_unix_ms,
    last_used_at_unix_ms, copy_count, is_favorite, is_pinned, deleted_at_unix_ms, nxt_clip_chars(text)";
// Keep the SQL projection's scalar limit tied to the public API constant.
const _: [(); CLIPBOARD_SUMMARY_CHARACTERS] = [(); 400];
const AFTER_CURSOR: &str = "(is_pinned < ?9
    OR (is_pinned = ?9 AND last_used_at_unix_ms < ?10)
    OR (is_pinned = ?9 AND last_used_at_unix_ms = ?10 AND captured_at_unix_ms < ?11)
    OR (is_pinned = ?9 AND last_used_at_unix_ms = ?10
        AND captured_at_unix_ms = ?11 AND id < ?12))";

fn map_summary(row: &Row<'_>) -> rusqlite::Result<ClipboardItemSummary> {
    Ok(ClipboardItemSummary {
        item: map_item(row)?,
        text_character_count: row.get::<_, i64>(18)? as usize,
    })
}

pub(super) fn register_summary_functions(
    connection: &rusqlite::Connection,
) -> rusqlite::Result<()> {
    let flags = rusqlite::functions::FunctionFlags::SQLITE_UTF8
        | rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC
        | rusqlite::functions::FunctionFlags::SQLITE_INNOCUOUS;
    connection.create_scalar_function("nxt_clip_summary", 1, flags, |context| {
        let text = context.get_raw(0).as_str()?;
        Ok(text
            .chars()
            .take(CLIPBOARD_SUMMARY_CHARACTERS)
            .collect::<String>())
    })?;
    connection.create_scalar_function("nxt_clip_chars", 1, flags, |context| {
        Ok(context.get_raw(0).as_str()?.chars().count() as i64)
    })
}

impl ClipboardHistory {
    /// Resolve a real active item into the original all-history order without loading
    /// unbounded payloads. Count and page use the same worker-owned SQLite transaction.
    pub fn page_containing_item(
        &mut self,
        item_id: i64,
        limit: usize,
    ) -> Result<(usize, ClipboardHistoryPage), ClipboardHistoryError> {
        let transaction = self.connection.unchecked_transaction()?;
        let position = self.position_before_item(item_id)?;
        let page_limit = limit.clamp(1, 500);
        let offset = position / page_limit * page_limit;
        let page = self.query_page(
            "",
            None,
            false,
            None,
            ClipboardSearchScope::Content,
            offset,
            page_limit,
            None,
            None,
        )?;
        if !page.items.iter().any(|item| item.id == item_id) {
            return Err(ClipboardHistoryError::ItemNotFound(item_id));
        }
        transaction.commit()?;
        Ok((offset, page))
    }

    /// Locate an active row in the all-history summary order within one SQLite snapshot.
    pub fn summary_page_containing_item(
        &mut self,
        item_id: i64,
        limit: usize,
    ) -> Result<(usize, ClipboardSummaryPage), ClipboardHistoryError> {
        let transaction = self.connection.unchecked_transaction()?;
        let position = self.position_before_item(item_id)?;
        let page_limit = limit.clamp(1, 500);
        let offset = position / page_limit * page_limit;
        let page = self.summary_category_page_with_total(
            "",
            super::BUILTIN_ALL_CATEGORY_ID,
            ClipboardSearchScope::Content,
            offset,
            page_limit,
            None,
        )?;
        if !page.items.iter().any(|row| row.item.id == item_id) {
            return Err(ClipboardHistoryError::ItemNotFound(item_id));
        }
        transaction.commit()?;
        Ok((offset, page))
    }

    fn position_before_item(&self, item_id: i64) -> Result<usize, ClipboardHistoryError> {
        Ok(self.connection.query_row(
            "SELECT COUNT(*) FROM clipboard_items AS candidate
             JOIN clipboard_items AS target ON target.id = ?1 AND target.deleted_at_unix_ms = 0
             WHERE candidate.deleted_at_unix_ms = 0 AND
               (candidate.is_pinned, candidate.last_used_at_unix_ms, candidate.captured_at_unix_ms, candidate.id)
               > (target.is_pinned, target.last_used_at_unix_ms, target.captured_at_unix_ms, target.id)",
            params![item_id], |row| row.get::<_, i64>(0),
        )? as usize)
    }

    pub fn recent(&self, limit: usize) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query("", None, false, None, ClipboardSearchScope::Content, limit)
    }

    pub fn counts(&self) -> Result<ClipboardHistoryCounts, ClipboardHistoryError> {
        self.connection
            .query_row(
                "SELECT COUNT(*),
                        COALESCE(SUM(content_type = 'text'), 0),
                        COALESCE(SUM(content_type = 'image'), 0),
                        COALESCE(SUM(content_type = 'files'), 0)
                 FROM clipboard_items
                 WHERE deleted_at_unix_ms = 0",
                [],
                |row| {
                    Ok(ClipboardHistoryCounts {
                        total: row.get::<_, i64>(0)? as usize,
                        text: row.get::<_, i64>(1)? as usize,
                        images: row.get::<_, i64>(2)? as usize,
                        files: row.get::<_, i64>(3)? as usize,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn recent_by_kind(
        &self,
        kind: ClipboardKind,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query(
            "",
            Some(kind),
            false,
            None,
            ClipboardSearchScope::Content,
            limit,
        )
    }

    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query(
            query,
            None,
            false,
            None,
            ClipboardSearchScope::Content,
            limit,
        )
    }

    /// Independent full-content keyset page for trusted extension reads. The worker
    /// compares this connection's mutation count before continuing an existing cursor.
    pub fn extension_page(
        &self,
        query: &str,
        cursor: Option<ClipboardPageCursor>,
        limit: usize,
    ) -> Result<(ClipboardHistoryPage, String), ClipboardHistoryError> {
        let revision = self.mutation_revision()?;
        let page = self.query_page(
            query,
            None,
            false,
            None,
            ClipboardSearchScope::Extended,
            0,
            limit.clamp(1, 32),
            None,
            cursor,
        )?;
        Ok((page, revision))
    }

    pub fn mutation_revision(&self) -> Result<String, ClipboardHistoryError> {
        let external: i64 = self
            .connection
            .query_row("PRAGMA data_version", [], |row| row.get(0))?;
        Ok(format!(
            "{}:{}:{external}",
            self.read_session,
            self.connection.total_changes()
        ))
    }

    pub fn search_extended(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query(
            query,
            None,
            false,
            None,
            ClipboardSearchScope::Extended,
            limit,
        )
    }

    pub fn search_by_kind(
        &self,
        query: &str,
        kind: ClipboardKind,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query(
            query,
            Some(kind),
            false,
            None,
            ClipboardSearchScope::Content,
            limit,
        )
    }

    pub fn query_category(
        &self,
        query: &str,
        category_id: &str,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query_category_with_scope(query, category_id, ClipboardSearchScope::Content, limit)
    }

    pub fn query_category_with_scope(
        &self,
        query: &str,
        category_id: &str,
        search_scope: ClipboardSearchScope,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        let category = self.category_by_id(category_id)?;
        match category.kind {
            ClipboardCategoryKind::All => self.query(query, None, false, None, search_scope, limit),
            ClipboardCategoryKind::Favorite => {
                self.query(query, None, true, None, search_scope, limit)
            }
            ClipboardCategoryKind::Text => self.query(
                query,
                Some(ClipboardKind::Text),
                false,
                None,
                search_scope,
                limit,
            ),
            ClipboardCategoryKind::Image => self.query(
                query,
                Some(ClipboardKind::Image),
                false,
                None,
                search_scope,
                limit,
            ),
            ClipboardCategoryKind::Files => self.query(
                query,
                Some(ClipboardKind::Files),
                false,
                None,
                search_scope,
                limit,
            ),
            ClipboardCategoryKind::Custom => {
                self.query(query, None, false, Some(category_id), search_scope, limit)
            }
        }
    }

    pub fn query_category_page(
        &self,
        query: &str,
        category_id: &str,
        search_scope: ClipboardSearchScope,
        offset: usize,
        limit: usize,
    ) -> Result<ClipboardHistoryPage, ClipboardHistoryError> {
        self.query_category_page_with_total(query, category_id, search_scope, offset, limit, None)
    }

    pub fn query_category_page_with_total(
        &self,
        query: &str,
        category_id: &str,
        search_scope: ClipboardSearchScope,
        offset: usize,
        limit: usize,
        known_total: Option<usize>,
    ) -> Result<ClipboardHistoryPage, ClipboardHistoryError> {
        self.query_category_page_with_cursor(
            query,
            category_id,
            search_scope,
            offset,
            limit,
            None,
            known_total,
        )
    }

    pub fn query_category_page_after(
        &self,
        query: &str,
        category_id: &str,
        search_scope: ClipboardSearchScope,
        cursor: ClipboardPageCursor,
        limit: usize,
        known_total: Option<usize>,
    ) -> Result<ClipboardHistoryPage, ClipboardHistoryError> {
        self.query_category_page_with_cursor(
            query,
            category_id,
            search_scope,
            0,
            limit,
            Some(cursor),
            known_total,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn query_category_page_with_cursor(
        &self,
        query: &str,
        category_id: &str,
        search_scope: ClipboardSearchScope,
        offset: usize,
        limit: usize,
        cursor: Option<ClipboardPageCursor>,
        known_total: Option<usize>,
    ) -> Result<ClipboardHistoryPage, ClipboardHistoryError> {
        let category = self.category_by_id(category_id)?;
        match category.kind {
            ClipboardCategoryKind::All => self.query_page(
                query,
                None,
                false,
                None,
                search_scope,
                offset,
                limit,
                known_total,
                cursor,
            ),
            ClipboardCategoryKind::Favorite => self.query_page(
                query,
                None,
                true,
                None,
                search_scope,
                offset,
                limit,
                known_total,
                cursor,
            ),
            ClipboardCategoryKind::Text => self.query_page(
                query,
                Some(ClipboardKind::Text),
                false,
                None,
                search_scope,
                offset,
                limit,
                known_total,
                cursor,
            ),
            ClipboardCategoryKind::Image => self.query_page(
                query,
                Some(ClipboardKind::Image),
                false,
                None,
                search_scope,
                offset,
                limit,
                known_total,
                cursor,
            ),
            ClipboardCategoryKind::Files => self.query_page(
                query,
                Some(ClipboardKind::Files),
                false,
                None,
                search_scope,
                offset,
                limit,
                known_total,
                cursor,
            ),
            ClipboardCategoryKind::Custom => self.query_page(
                query,
                None,
                false,
                Some(category_id),
                search_scope,
                offset,
                limit,
                known_total,
                cursor,
            ),
        }
    }

    /// Bounded list projection; search, categories and ordering use the full-history predicate.
    pub fn summary_category_page_with_total(
        &self,
        query: &str,
        category_id: &str,
        scope: ClipboardSearchScope,
        offset: usize,
        limit: usize,
        known_total: Option<usize>,
    ) -> Result<ClipboardSummaryPage, ClipboardHistoryError> {
        self.filtered_summary_page(
            query,
            category_id,
            scope,
            offset,
            limit,
            None,
            known_total,
            &ClipboardHistoryFilter::default(),
        )
    }

    pub fn summary_category_page_after(
        &self,
        query: &str,
        category_id: &str,
        scope: ClipboardSearchScope,
        cursor: ClipboardPageCursor,
        limit: usize,
        known_total: Option<usize>,
    ) -> Result<ClipboardSummaryPage, ClipboardHistoryError> {
        self.filtered_summary_page(
            query,
            category_id,
            scope,
            0,
            limit,
            Some(cursor),
            known_total,
            &ClipboardHistoryFilter::default(),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn filtered_summary_page(
        &self,
        query: &str,
        category_id: &str,
        scope: ClipboardSearchScope,
        offset: usize,
        limit: usize,
        cursor: Option<ClipboardPageCursor>,
        known_total: Option<usize>,
        filters: &ClipboardHistoryFilter,
    ) -> Result<ClipboardSummaryPage, ClipboardHistoryError> {
        let category = self.category_by_id(category_id)?;
        let (kind, favorite, source) = match category.kind {
            ClipboardCategoryKind::All => (None, false, None),
            ClipboardCategoryKind::Favorite => (None, true, None),
            ClipboardCategoryKind::Text => (Some(ClipboardKind::Text), false, None),
            ClipboardCategoryKind::Image => (Some(ClipboardKind::Image), false, None),
            ClipboardCategoryKind::Files => (Some(ClipboardKind::Files), false, None),
            ClipboardCategoryKind::Custom => (None, false, Some(category_id)),
        };
        let total =
            self.filtered_total(query, kind, favorite, source, scope, known_total, filters)?;
        let page_limit = limit.clamp(1, 500);
        let (mut items, has_more) = if let Some(cursor) = cursor {
            let mut items = self.select_items_after(
                query,
                kind,
                favorite,
                source,
                scope,
                filters,
                cursor,
                page_limit.saturating_add(1),
                SUMMARY_COLUMNS,
                map_summary,
            )?;
            let more = items.len() > page_limit;
            items.truncate(page_limit);
            (items, more)
        } else {
            let items = self.select_items(
                query,
                kind,
                favorite,
                source,
                scope,
                filters,
                offset,
                page_limit,
                SUMMARY_COLUMNS,
                map_summary,
            )?;
            let more = offset.saturating_add(items.len()) < total;
            (items, more)
        };
        Ok(ClipboardSummaryPage {
            items: std::mem::take(&mut items),
            total,
            has_more,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn filtered_total(
        &self,
        query: &str,
        kind: Option<ClipboardKind>,
        favorite: bool,
        source: Option<&str>,
        scope: ClipboardSearchScope,
        known_total: Option<usize>,
        filters: &ClipboardHistoryFilter,
    ) -> Result<usize, ClipboardHistoryError> {
        if let Some(total) = known_total {
            return Ok(total);
        }
        let sql = format!("SELECT COUNT(*) FROM clipboard_items WHERE {FILTER}");
        Ok(self.connection.query_row(
            &sql,
            params![
                kind.map(ClipboardKind::as_str),
                favorite,
                source,
                query,
                scope == ClipboardSearchScope::Extended,
                filters.source_process.as_deref(),
                filters.captured_between_unix_ms.map(|(start, _)| start),
                filters.captured_between_unix_ms.map(|(_, end)| end),
            ],
            |row| row.get::<_, i64>(0),
        )? as usize)
    }

    #[allow(clippy::too_many_arguments)]
    fn select_items<T>(
        &self,
        query: &str,
        kind: Option<ClipboardKind>,
        favorite: bool,
        source: Option<&str>,
        scope: ClipboardSearchScope,
        filters: &ClipboardHistoryFilter,
        offset: usize,
        limit: usize,
        columns: &str,
        map: fn(&Row<'_>) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, ClipboardHistoryError> {
        let sql = format!(
            "SELECT {columns} FROM clipboard_items WHERE {FILTER} ORDER BY {ORDER} LIMIT ?9 OFFSET ?10"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(
            params![
                kind.map(ClipboardKind::as_str),
                favorite,
                source,
                query,
                scope == ClipboardSearchScope::Extended,
                filters.source_process.as_deref(),
                filters.captured_between_unix_ms.map(|(start, _)| start),
                filters.captured_between_unix_ms.map(|(_, end)| end),
                limit as i64,
                offset as i64,
            ],
            map,
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    #[allow(clippy::too_many_arguments)]
    fn select_items_after<T>(
        &self,
        query: &str,
        kind: Option<ClipboardKind>,
        favorite: bool,
        source: Option<&str>,
        scope: ClipboardSearchScope,
        filters: &ClipboardHistoryFilter,
        cursor: ClipboardPageCursor,
        limit: usize,
        columns: &str,
        map: fn(&Row<'_>) -> rusqlite::Result<T>,
    ) -> Result<Vec<T>, ClipboardHistoryError> {
        let sql = format!(
            "SELECT {columns} FROM clipboard_items WHERE {FILTER} AND {AFTER_CURSOR} ORDER BY {ORDER} LIMIT ?13"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let rows = statement.query_map(
            params![
                kind.map(ClipboardKind::as_str),
                favorite,
                source,
                query,
                scope == ClipboardSearchScope::Extended,
                filters.source_process.as_deref(),
                filters.captured_between_unix_ms.map(|(start, _)| start),
                filters.captured_between_unix_ms.map(|(_, end)| end),
                i64::from(cursor.is_pinned),
                cursor.last_used_at_unix_ms,
                cursor.captured_at_unix_ms,
                cursor.id,
                limit as i64,
            ],
            map,
        )?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }

    fn query(
        &self,
        query: &str,
        kind: Option<ClipboardKind>,
        favorite_only: bool,
        source_category_id: Option<&str>,
        search_scope: ClipboardSearchScope,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.query_items(
            query,
            kind,
            favorite_only,
            source_category_id,
            search_scope,
            0,
            limit,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn query_page(
        &self,
        query: &str,
        kind: Option<ClipboardKind>,
        favorite_only: bool,
        source_category_id: Option<&str>,
        search_scope: ClipboardSearchScope,
        offset: usize,
        limit: usize,
        known_total: Option<usize>,
        cursor: Option<ClipboardPageCursor>,
    ) -> Result<ClipboardHistoryPage, ClipboardHistoryError> {
        let total = self.filtered_total(
            query,
            kind,
            favorite_only,
            source_category_id,
            search_scope,
            known_total,
            &ClipboardHistoryFilter::default(),
        )?;
        let page_limit = limit.clamp(1, 500);
        let (mut items, has_more) = if let Some(cursor) = cursor {
            let mut items = self.query_items_after(
                query,
                kind,
                favorite_only,
                source_category_id,
                search_scope,
                cursor,
                page_limit.saturating_add(1),
            )?;
            let has_more = items.len() > page_limit;
            items.truncate(page_limit);
            (items, has_more)
        } else {
            let items = self.query_items(
                query,
                kind,
                favorite_only,
                source_category_id,
                search_scope,
                offset,
                page_limit,
            )?;
            let has_more = offset.saturating_add(items.len()) < total;
            (items, has_more)
        };
        Ok(ClipboardHistoryPage {
            has_more,
            items: std::mem::take(&mut items),
            total,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn query_items(
        &self,
        query: &str,
        kind: Option<ClipboardKind>,
        favorite_only: bool,
        source_category_id: Option<&str>,
        search_scope: ClipboardSearchScope,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.select_items(
            query,
            kind,
            favorite_only,
            source_category_id,
            search_scope,
            &ClipboardHistoryFilter::default(),
            offset,
            limit,
            FULL_COLUMNS,
            map_item,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn query_items_after(
        &self,
        query: &str,
        kind: Option<ClipboardKind>,
        favorite_only: bool,
        source_category_id: Option<&str>,
        search_scope: ClipboardSearchScope,
        cursor: ClipboardPageCursor,
        limit: usize,
    ) -> Result<Vec<ClipboardItem>, ClipboardHistoryError> {
        self.select_items_after(
            query,
            kind,
            favorite_only,
            source_category_id,
            search_scope,
            &ClipboardHistoryFilter::default(),
            cursor,
            limit,
            FULL_COLUMNS,
            map_item,
        )
    }
}

#[cfg(test)]
mod result_location_tests {
    use super::*;
    use crate::{CaptureOutcome, CaptureSource, TextCapture};
    #[test]
    fn page_containing_item_preserves_actual_order_beyond_first_page_and_missing_is_error() {
        let directory = tempfile::tempdir().expect("owned history directory");
        let mut history =
            ClipboardHistory::open(directory.path().join("history.db")).expect("owned history");
        let mut ids = Vec::new();
        for index in 0..190 {
            let result = history
                .capture(TextCapture {
                    text: format!("fixture-{index}"),
                    native_formats: Vec::new(),
                    source: CaptureSource {
                        window_handle: 0,
                        process_id: 0,
                        process_name: String::new(),
                        process_icon_path: String::new(),
                        window_title: String::new(),
                    },
                    captured_at_unix_ms: index + 1,
                })
                .expect("capture");
            if let CaptureOutcome::Inserted(item) = result {
                ids.push(item.id);
            }
        }
        let target = ids[20];
        history.set_pinned(ids[0], true).expect("pin");
        let ordered = history.recent(500).expect("full ordering");
        let position = ordered
            .iter()
            .position(|item| item.id == target)
            .expect("target");
        assert!(position >= 80);
        let (offset, page) = history.page_containing_item(target, 80).expect("locate");
        assert_eq!(offset, position / 80 * 80);
        assert_eq!(
            page.items,
            ordered[offset..(offset + 80).min(ordered.len())]
        );
        assert_eq!(page.total, 190);
        history.soft_delete(target, 1000).expect("delete");
        assert!(
            matches!(history.page_containing_item(target,80),Err(ClipboardHistoryError::ItemNotFound(id)) if id==target)
        );
        assert!(matches!(
            history.page_containing_item(i64::MAX, 80),
            Err(ClipboardHistoryError::ItemNotFound(_))
        ));
        assert_eq!(history.counts().expect("count").total, 189);
    }
}

#[cfg(test)]
mod summary_seam_tests {
    use super::*;
    use crate::{
        BUILTIN_ALL_CATEGORY_ID, BUILTIN_FAVORITE_CATEGORY_ID, BUILTIN_TEXT_CATEGORY_ID,
        CLIPBOARD_SUMMARY_CHARACTERS, CaptureOutcome, CaptureSource, ClipboardNativeFormat,
        TextCapture,
    };

    fn capture(
        history: &mut ClipboardHistory,
        text: String,
        source: &str,
        time: i64,
        formats: Vec<ClipboardNativeFormat>,
    ) -> ClipboardItem {
        let outcome = history
            .capture(TextCapture {
                text,
                native_formats: formats,
                source: CaptureSource {
                    window_handle: 7,
                    process_id: 8,
                    process_name: source.into(),
                    process_icon_path: String::new(),
                    window_title: "fixture title".into(),
                },
                captured_at_unix_ms: time,
            })
            .expect("capture fixture");
        match outcome {
            CaptureOutcome::Inserted(item) => item,
            other => panic!("expected new row: {other:?}"),
        }
    }

    #[test]
    fn combined_filter_reuses_the_same_predicate_for_count_offset_and_cursor() {
        let directory = tempfile::tempdir().expect("history directory");
        let mut history =
            ClipboardHistory::open(directory.path().join("history.db")).expect("history");
        let older = capture(&mut history, "invoice A".into(), "EXCEL.exe", 20, vec![]);
        let newer = capture(&mut history, "invoice B".into(), "excel.exe", 29, vec![]);
        capture(&mut history, "invoice C".into(), "excel.exe", 30, vec![]);
        capture(&mut history, "invoice D".into(), "excel.exe", 19, vec![]);
        capture(&mut history, "invoice E".into(), "word.exe", 24, vec![]);
        capture(&mut history, "report F".into(), "excel.exe", 25, vec![]);
        let filters = ClipboardHistoryFilter {
            source_process: Some("Excel.exe".into()),
            captured_between_unix_ms: Some((20, 30)),
        };
        let page = |offset, cursor, total| {
            history
                .filtered_summary_page(
                    "invoice",
                    BUILTIN_TEXT_CATEGORY_ID,
                    ClipboardSearchScope::Content,
                    offset,
                    1,
                    cursor,
                    total,
                    &filters,
                )
                .expect("filtered page")
        };
        let first = page(0, None, None);
        assert_eq!(first.total, 2);
        assert_eq!(first.items[0].item.id, newer.id);
        assert!(first.has_more);
        let cursor = ClipboardPageCursor::from(&first.items[0].item);
        let next = page(0, Some(cursor), Some(first.total));
        assert_eq!(next, page(1, None, None));
        assert_eq!(next.items[0].item.id, older.id);
        assert!(!next.has_more);
    }

    #[test]
    fn summary_projects_unicode_scalar_prefix_and_count_without_truncating_full_search() {
        let directory = tempfile::tempdir().expect("history directory");
        let mut history =
            ClipboardHistory::open(directory.path().join("history.db")).expect("history");
        let text = format!("{}needle-tail", "🦀中".repeat(50_000));
        let full = capture(&mut history, text.clone(), "fixture", 1, Vec::new());
        let page = history
            .summary_category_page_with_total(
                "needle-tail",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                0,
                10,
                None,
            )
            .expect("summary search");
        assert_eq!(page.total, 1);
        assert!(!page.has_more);
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].text_character_count, text.chars().count());
        assert_eq!(
            page.items[0].item.text,
            text.chars()
                .take(CLIPBOARD_SUMMARY_CHARACTERS)
                .collect::<String>()
        );
        assert_eq!(
            page.items[0].item.text.chars().count(),
            CLIPBOARD_SUMMARY_CHARACTERS
        );
        println!(
            "synthetic summary UTF-8 body bytes: full={} projected={}",
            text.len(),
            page.items[0].item.text.len()
        );
        assert_eq!(
            history
                .query_category_page_with_total(
                    "needle-tail",
                    BUILTIN_ALL_CATEGORY_ID,
                    ClipboardSearchScope::Content,
                    0,
                    10,
                    None
                )
                .expect("original full page")
                .items,
            vec![full.clone()]
        );
        assert_eq!(
            history
                .search("needle-tail", 10)
                .expect("original full search"),
            vec![full.clone()]
        );
        assert_eq!(history.active_item(full.id).expect("full lookup"), full);

        // The bounded projection and count must include embedded NUL, not SQLite's short prefix.
        let nul_text = format!("{}\0后🦀", "a".repeat(30));
        let nul_item = capture(&mut history, nul_text.clone(), "fixture", 2, Vec::new());
        let short = history
            .summary_category_page_with_total(
                "",
                BUILTIN_ALL_CATEGORY_ID,
                ClipboardSearchScope::Content,
                0,
                2,
                None,
            )
            .expect("short summary");
        let row = short
            .items
            .iter()
            .find(|row| row.item.id == nul_item.id)
            .expect("NUL row");
        assert_eq!(row.item.text, nul_text);
        assert_eq!(row.text_character_count, 33);
        assert_eq!(
            history.active_item(nul_item.id).expect("full NUL row").text,
            nul_text
        );
    }

    #[test]
    fn summary_cursor_offset_category_scope_and_known_total_match_full_pages() {
        let directory = tempfile::tempdir().expect("history directory");
        let mut history =
            ClipboardHistory::open(directory.path().join("history.db")).expect("history");
        for index in 0..9 {
            let item = capture(
                &mut history,
                format!("item-{index} {}", "🦀".repeat(700)),
                if index % 2 == 0 { "alpha" } else { "beta" },
                index + 1,
                Vec::new(),
            );
            if index % 3 == 0 {
                history.set_favorite(item.id, true).expect("favorite");
            }
            if index == 0 {
                history.set_pinned(item.id, true).expect("pin");
            }
        }
        let custom = history.create_category("alpha-group").expect("category");
        history
            .replace_category_sources(&custom.id, &["alpha".into()])
            .expect("sources");
        for (category, query, scope) in [
            (BUILTIN_ALL_CATEGORY_ID, "", ClipboardSearchScope::Content),
            (
                BUILTIN_TEXT_CATEGORY_ID,
                "item",
                ClipboardSearchScope::Content,
            ),
            (
                BUILTIN_FAVORITE_CATEGORY_ID,
                "",
                ClipboardSearchScope::Content,
            ),
            (custom.id.as_str(), "alpha", ClipboardSearchScope::Extended),
        ] {
            let full = history
                .query_category_page_with_total(query, category, scope, 0, 2, None)
                .expect("full first");
            let summary = history
                .summary_category_page_with_total(query, category, scope, 0, 2, None)
                .expect("summary first");
            assert_eq!(
                (summary.total, summary.has_more),
                (full.total, full.has_more)
            );
            assert_eq!(
                summary
                    .items
                    .iter()
                    .map(|row| row.item.id)
                    .collect::<Vec<_>>(),
                full.items.iter().map(|item| item.id).collect::<Vec<_>>()
            );
            if let Some(last) = full.items.last() {
                let cursor = ClipboardPageCursor::from(last);
                let full_next = history
                    .query_category_page_after(query, category, scope, cursor, 2, Some(full.total))
                    .expect("full cursor");
                let summary_next = history
                    .summary_category_page_after(
                        query,
                        category,
                        scope,
                        cursor,
                        2,
                        Some(full.total),
                    )
                    .expect("summary cursor");
                let offset_next = history
                    .summary_category_page_with_total(
                        query,
                        category,
                        scope,
                        2,
                        2,
                        Some(full.total),
                    )
                    .expect("summary offset");
                assert_eq!(
                    (summary_next.total, summary_next.has_more),
                    (full_next.total, full_next.has_more)
                );
                assert_eq!(summary_next, offset_next);
                assert_eq!(
                    summary_next
                        .items
                        .iter()
                        .map(|row| row.item.id)
                        .collect::<Vec<_>>(),
                    full_next
                        .items
                        .iter()
                        .map(|item| item.id)
                        .collect::<Vec<_>>()
                );
            }
        }
        assert!(matches!(
            history.summary_category_page_with_total(
                "",
                "missing",
                ClipboardSearchScope::Content,
                0,
                2,
                None
            ),
            Err(ClipboardHistoryError::CategoryNotFound(_))
        ));
    }

    #[test]
    fn active_item_and_native_formats_hydrate_exact_live_row_only() {
        let directory = tempfile::tempdir().expect("history directory");
        let mut history =
            ClipboardHistory::open(directory.path().join("history.db")).expect("history");
        let format = ClipboardNativeFormat {
            format_id: 42,
            format_name: "fixture".into(),
            data_path: "owned/fixture".into(),
            size_bytes: 512,
            sort_order: 1,
        };
        let item = capture(
            &mut history,
            format!("{}end", "🦀".repeat(800)),
            "fixture",
            1,
            vec![format.clone()],
        );
        assert_eq!(history.active_item(item.id).expect("full row"), item);
        assert_eq!(
            history
                .active_item_with_native_formats(item.id)
                .expect("atomic full row and formats"),
            (item.clone(), vec![format])
        );
        history.soft_delete(item.id, 100).expect("delete");
        for id in [item.id, i64::MAX] {
            assert!(
                matches!(history.active_item(id), Err(ClipboardHistoryError::ItemNotFound(value)) if value == id)
            );
            assert!(
                matches!(history.active_item_with_native_formats(id), Err(ClipboardHistoryError::ItemNotFound(value)) if value == id)
            );
        }
        // The pre-existing private lookup still returns soft-deleted rows for restoration.
        assert_eq!(
            history.item_by_id(item.id).expect("legacy lookup").id,
            item.id
        );
    }

    #[test]
    fn summary_page_containing_item_locates_later_page_without_full_text() {
        let directory = tempfile::tempdir().expect("history directory");
        let mut history =
            ClipboardHistory::open(directory.path().join("history.db")).expect("history");
        let mut ids = Vec::new();
        for index in 0..11 {
            ids.push(
                capture(
                    &mut history,
                    format!("{}-{index}", "🦀".repeat(1000)),
                    "fixture",
                    index + 1,
                    Vec::new(),
                )
                .id,
            );
        }
        history.set_pinned(ids[0], true).expect("pin");
        let (offset, page) = history
            .summary_page_containing_item(ids[2], 3)
            .expect("locate");
        let (full_offset, full_page) = history
            .page_containing_item(ids[2], 3)
            .expect("full locate");
        assert!(offset >= 3);
        assert_eq!(offset, full_offset);
        assert_eq!(
            (page.total, page.has_more),
            (full_page.total, full_page.has_more)
        );
        assert_eq!(
            page.items.iter().map(|row| row.item.id).collect::<Vec<_>>(),
            full_page
                .items
                .iter()
                .map(|item| item.id)
                .collect::<Vec<_>>()
        );
        assert!(
            page.items
                .iter()
                .all(|row| row.item.text.chars().count() <= CLIPBOARD_SUMMARY_CHARACTERS)
        );
        history.soft_delete(ids[2], 100).expect("delete");
        assert!(matches!(
            history.summary_page_containing_item(ids[2], 3),
            Err(ClipboardHistoryError::ItemNotFound(_))
        ));
        assert!(matches!(
            history.summary_page_containing_item(i64::MAX, 3),
            Err(ClipboardHistoryError::ItemNotFound(_))
        ));
    }
}
