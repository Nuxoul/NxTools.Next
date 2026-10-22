use std::collections::HashSet;

use rusqlite::{Connection, OptionalExtension, params};

use super::{
    BUILTIN_ALL_CATEGORY_ID, BUILTIN_FAVORITE_CATEGORY_ID, BUILTIN_FILES_CATEGORY_ID,
    BUILTIN_IMAGE_CATEGORY_ID, BUILTIN_TEXT_CATEGORY_ID, ClipboardCategory, ClipboardCategoryKind,
    ClipboardHistory, ClipboardHistoryError,
};

impl ClipboardHistory {
    pub fn categories(&self) -> Result<Vec<ClipboardCategory>, ClipboardHistoryError> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, kind, sort_order
             FROM clipboard_categories
             ORDER BY sort_order, id",
        )?;
        statement
            .query_map([], map_category)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn create_category(
        &mut self,
        name: &str,
    ) -> Result<ClipboardCategory, ClipboardHistoryError> {
        let name = validate_category_name(name)?;
        self.ensure_category_name_available(name, None)?;
        let sort_order = self.connection.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM clipboard_categories",
            [],
            |row| row.get::<_, i64>(0),
        )?;
        let id = format!("custom-{}", uuid::Uuid::new_v4());
        self.connection.execute(
            "INSERT INTO clipboard_categories (id, name, kind, sort_order)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, name, ClipboardCategoryKind::Custom.as_str(), sort_order],
        )?;
        self.category_by_id(&id)
    }

    pub fn rename_category(
        &mut self,
        category_id: &str,
        name: &str,
    ) -> Result<ClipboardCategory, ClipboardHistoryError> {
        let category = self.category_by_id(category_id)?;
        if category.kind.is_builtin() {
            return Err(ClipboardHistoryError::BuiltInCategoryImmutable(
                category_id.to_owned(),
            ));
        }
        let name = validate_category_name(name)?;
        self.ensure_category_name_available(name, Some(category_id))?;
        self.connection.execute(
            "UPDATE clipboard_categories SET name = ?1 WHERE id = ?2",
            params![name, category_id],
        )?;
        self.category_by_id(category_id)
    }

    pub fn delete_category(&mut self, category_id: &str) -> Result<(), ClipboardHistoryError> {
        let category = self.category_by_id(category_id)?;
        if category.kind.is_builtin() {
            return Err(ClipboardHistoryError::BuiltInCategoryImmutable(
                category_id.to_owned(),
            ));
        }
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM clipboard_category_sources WHERE category_id = ?1",
            [category_id],
        )?;
        transaction.execute(
            "DELETE FROM clipboard_categories WHERE id = ?1",
            [category_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn reorder_categories(
        &mut self,
        ordered_ids: &[String],
    ) -> Result<Vec<ClipboardCategory>, ClipboardHistoryError> {
        let existing = self.categories()?;
        let expected = existing
            .iter()
            .map(|category| category.id.as_str())
            .collect::<HashSet<_>>();
        let supplied = ordered_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        if ordered_ids.len() != existing.len() || supplied != expected {
            return Err(ClipboardHistoryError::InvalidCategoryOrder);
        }
        let transaction = self.connection.transaction()?;
        for (sort_order, id) in ordered_ids.iter().enumerate() {
            transaction.execute(
                "UPDATE clipboard_categories SET sort_order = ?1 WHERE id = ?2",
                params![sort_order as i64, id],
            )?;
        }
        transaction.commit()?;
        self.categories()
    }

    pub fn category_sources(
        &self,
        category_id: &str,
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        let category = self.category_by_id(category_id)?;
        if category.kind.is_builtin() {
            return Ok(Vec::new());
        }
        let mut statement = self.connection.prepare(
            "SELECT process_name FROM clipboard_category_sources
             WHERE category_id = ?1
             ORDER BY process_name COLLATE NOCASE",
        )?;
        statement
            .query_map([category_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn source_process_names(&self) -> Result<Vec<String>, ClipboardHistoryError> {
        let mut statement = self.connection.prepare(
            "SELECT DISTINCT source_process_name
             FROM clipboard_items
             WHERE deleted_at_unix_ms = 0 AND trim(source_process_name) != ''
             ORDER BY source_process_name COLLATE NOCASE",
        )?;
        statement
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(Into::into)
    }

    pub fn add_category_source(
        &mut self,
        category_id: &str,
        process_name: &str,
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        self.ensure_custom_category(category_id)?;
        let process_name = validate_category_source(process_name)?;
        self.connection.execute(
            "INSERT OR IGNORE INTO clipboard_category_sources (category_id, process_name)
             VALUES (?1, ?2)",
            params![category_id, process_name],
        )?;
        self.category_sources(category_id)
    }

    pub fn remove_category_source(
        &mut self,
        category_id: &str,
        process_name: &str,
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        self.ensure_custom_category(category_id)?;
        let process_name = validate_category_source(process_name)?;
        self.connection.execute(
            "DELETE FROM clipboard_category_sources
             WHERE category_id = ?1 AND process_name = ?2 COLLATE NOCASE",
            params![category_id, process_name],
        )?;
        self.category_sources(category_id)
    }

    pub fn replace_category_sources(
        &mut self,
        category_id: &str,
        process_names: &[String],
    ) -> Result<Vec<String>, ClipboardHistoryError> {
        self.ensure_custom_category(category_id)?;
        let mut normalized = HashSet::new();
        let mut validated = Vec::new();
        for process_name in process_names {
            let process_name = validate_category_source(process_name)?.to_owned();
            if normalized.insert(process_name.to_lowercase()) {
                validated.push(process_name);
            }
        }
        validated.sort_by_key(|value| value.to_lowercase());

        let transaction = self.connection.transaction()?;
        transaction.execute(
            "DELETE FROM clipboard_category_sources WHERE category_id = ?1",
            [category_id],
        )?;
        for process_name in &validated {
            transaction.execute(
                "INSERT OR IGNORE INTO clipboard_category_sources (category_id, process_name)
                 VALUES (?1, ?2)",
                params![category_id, process_name],
            )?;
        }
        transaction.commit()?;
        self.category_sources(category_id)
    }

    fn ensure_custom_category(
        &self,
        category_id: &str,
    ) -> Result<ClipboardCategory, ClipboardHistoryError> {
        let category = self.category_by_id(category_id)?;
        if category.kind.is_builtin() {
            return Err(ClipboardHistoryError::BuiltInCategoryImmutable(
                category_id.to_owned(),
            ));
        }
        Ok(category)
    }

    fn ensure_category_name_available(
        &self,
        name: &str,
        excluded_id: Option<&str>,
    ) -> Result<(), ClipboardHistoryError> {
        let existing = self
            .connection
            .query_row(
                "SELECT id FROM clipboard_categories
                 WHERE name = ?1 COLLATE NOCASE AND (?2 IS NULL OR id != ?2)
                 LIMIT 1",
                params![name, excluded_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        if existing.is_some() {
            return Err(ClipboardHistoryError::CategoryNameInUse(name.to_owned()));
        }
        Ok(())
    }

    pub(super) fn category_by_id(
        &self,
        category_id: &str,
    ) -> Result<ClipboardCategory, ClipboardHistoryError> {
        self.connection
            .query_row(
                "SELECT id, name, kind, sort_order FROM clipboard_categories WHERE id = ?1",
                [category_id],
                map_category,
            )
            .optional()?
            .ok_or_else(|| ClipboardHistoryError::CategoryNotFound(category_id.to_owned()))
    }
}

fn map_category(row: &rusqlite::Row<'_>) -> rusqlite::Result<ClipboardCategory> {
    let kind = row.get::<_, String>(2)?;
    let kind = ClipboardCategoryKind::from_str(&kind).map_err(|message| {
        rusqlite::Error::FromSqlConversionFailure(
            2,
            rusqlite::types::Type::Text,
            Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                message,
            )),
        )
    })?;
    Ok(ClipboardCategory {
        id: row.get(0)?,
        name: row.get(1)?,
        kind,
        sort_order: row.get(3)?,
    })
}

pub(super) fn seed_builtin_categories(connection: &Connection) -> Result<(), rusqlite::Error> {
    let builtins = [
        (BUILTIN_ALL_CATEGORY_ID, "全部", ClipboardCategoryKind::All),
        (
            BUILTIN_FAVORITE_CATEGORY_ID,
            "星标",
            ClipboardCategoryKind::Favorite,
        ),
        (
            BUILTIN_TEXT_CATEGORY_ID,
            "文字",
            ClipboardCategoryKind::Text,
        ),
        (
            BUILTIN_IMAGE_CATEGORY_ID,
            "图片",
            ClipboardCategoryKind::Image,
        ),
        (
            BUILTIN_FILES_CATEGORY_ID,
            "文件",
            ClipboardCategoryKind::Files,
        ),
    ];
    let transaction = connection.unchecked_transaction()?;
    for (sort_order, (id, name, kind)) in builtins.into_iter().enumerate() {
        transaction.execute(
            "INSERT INTO clipboard_categories (id, name, kind, sort_order)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(id) DO UPDATE SET name = excluded.name, kind = excluded.kind",
            params![id, name, kind.as_str(), sort_order as i64],
        )?;
    }
    transaction.commit()
}

fn validate_category_name(name: &str) -> Result<&str, ClipboardHistoryError> {
    let name = name.trim();
    if name.is_empty() {
        Err(ClipboardHistoryError::EmptyCategoryName)
    } else {
        Ok(name)
    }
}

fn validate_category_source(process_name: &str) -> Result<&str, ClipboardHistoryError> {
    let process_name = process_name.trim();
    if process_name.is_empty() {
        Err(ClipboardHistoryError::EmptyCategorySource)
    } else {
        Ok(process_name)
    }
}
