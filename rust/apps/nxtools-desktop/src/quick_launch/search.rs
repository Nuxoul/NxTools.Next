use super::*;

pub(crate) fn recent_modified_filter(query: &str) -> Option<&str> {
    let query = query.trim();
    query
        .get(..7)
        .filter(|prefix| prefix.eq_ignore_ascii_case("recent:"))
        .map(|_| query[7..].trim())
}

pub(crate) fn recent_modified_results(items: &[IndexSearchItem]) -> Vec<SearchResult> {
    items
        .iter()
        .filter(|item| !item.is_directory && item.modified_at_unix_ms.is_some())
        .take(RESULT_LIMIT)
        .map(|item| SearchResult {
            entry: nxtools_finder::ApplicationEntry {
                id: format!("indexed:{}", item.path.replace('/', "\\").to_lowercase()),
                name: item.name.clone(),
                target: item.path.clone(),
                source: ApplicationSource::IndexedFile,
            },
            action: SearchResultAction::OpenTarget,
            subtitle: Some(format!("最近修改 · {}", item.path)),
            score: item.score,
            activation_count: 0,
            last_activated_at_unix_ms: None,
            query_activation_count: 0,
            last_query_activation_at_unix_ms: None,
        })
        .collect()
}

pub(crate) fn index_settings_from_quick_launch(settings: &QuickLaunchSettings) -> IndexSettings {
    IndexSettings {
        indexed_volumes: settings.indexed_volumes.clone(),
        excluded_paths: settings.excluded_paths.clone(),
    }
}

pub(crate) fn quick_launch_results_visible(
    query: &str,
    action_mode_active: bool,
    recent_directory_mode_active: bool,
) -> bool {
    action_mode_active || recent_directory_mode_active || !query.trim().is_empty()
}

pub(crate) fn quick_launch_catalog_refresh_query(
    is_open: bool,
    action_mode_active: bool,
    query: String,
) -> Option<String> {
    (is_open && !action_mode_active).then_some(query)
}

pub(crate) fn quick_launch_panel_height(list_height: f32, has_status: bool) -> f32 {
    COLLAPSED_PANEL_HEIGHT
        + list_height
        + if has_status {
            LAUNCHER_STATUS_FOOTER_HEIGHT
        } else {
            0.0
        }
}

pub(crate) fn recent_directory_results(
    recent: &[RecentDirectory],
    filter: &str,
) -> Vec<SearchResult> {
    let filter = filter.trim().to_lowercase();
    recent
        .iter()
        .filter(|directory| filter.is_empty() || directory.path.to_lowercase().contains(&filter))
        .map(|directory| {
            let name = Path::new(&directory.path)
                .file_name()
                .and_then(|name| name.to_str())
                .filter(|name| !name.is_empty())
                .unwrap_or(&directory.path)
                .to_owned();
            SearchResult {
                entry: nxtools_finder::ApplicationEntry {
                    id: format!(
                        "recent-directory:{}",
                        directory.path.replace('/', "\\").to_lowercase()
                    ),
                    name,
                    target: directory.path.clone(),
                    source: ApplicationSource::IndexedDirectory,
                },
                action: SearchResultAction::OpenTarget,
                subtitle: None,
                score: 0,
                activation_count: directory.visit_count,
                last_activated_at_unix_ms: Some(directory.last_visited_at_unix_ms),
                query_activation_count: 0,
                last_query_activation_at_unix_ms: None,
            }
        })
        .collect()
}

pub(crate) fn is_navigable_directory_target(target: &str) -> bool {
    let value = target.trim();
    Path::new(value).is_absolute()
        || value
            .strip_prefix("lib://")
            .is_some_and(|library| !library.trim_matches(['\\', '/']).is_empty())
}

pub(crate) fn directory_target_diagnostic_fields(
    target: &str,
) -> (&'static str, String, Option<String>, usize, usize, bool) {
    let value = target.trim();
    let is_library = value.starts_with("lib://");
    let kind = if is_library {
        "library_uri"
    } else if Path::new(value).is_absolute() {
        "filesystem_absolute"
    } else {
        "other"
    };
    let root = if is_library {
        "lib://".to_owned()
    } else {
        Path::new(value)
            .components()
            .next()
            .map(|component| component.as_os_str().to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let library = is_library.then(|| value["lib://".len()..].trim_matches(['\\', '/']).to_owned());
    let segments = value
        .split(['\\', '/'])
        .filter(|segment| !segment.is_empty())
        .count();
    (
        kind,
        root,
        library,
        value.len(),
        segments,
        value.ends_with(['\\', '/']),
    )
}

pub(crate) fn recent_directory_navigation_action(
    result: &SearchResult,
    navigation_context: Option<&nxtools_protocol::DirectoryNavigationContext>,
) -> Option<QuickLaunchAction> {
    let context = navigation_context.filter(|context| context.window_handle != 0)?;
    (result.entry.source == ApplicationSource::IndexedDirectory
        && is_navigable_directory_target(&result.entry.target))
    .then(|| QuickLaunchAction::SwitchDirectory {
        host: context.host,
        window_handle: context.window_handle,
        target_directory: result.entry.target.clone(),
        host_context_id: context.host_context_id.clone(),
    })
}

pub(crate) fn elevated_directory_helper_command(
    request: Option<&ElevatedDirectoryHelperRequest>,
    request_pending: bool,
    acknowledged_pipe_name: Option<&str>,
    indexer_connected: bool,
) -> Option<IndexerCommand> {
    (!request_pending
        && indexer_connected
        && request
            .is_some_and(|request| acknowledged_pipe_name != Some(request.pipe_name.as_str())))
    .then(|| request.cloned())
    .flatten()
    .map(IndexerCommand::EnsureElevatedDirectoryHelper)
}

pub(crate) fn elevated_helper_status_matches_request(
    request: Option<&ElevatedDirectoryHelperRequest>,
    status: &nxtools_protocol::ElevatedDirectoryHelperStatus,
) -> bool {
    request.is_some_and(|request| request.pipe_name == status.pipe_name)
}

pub(crate) fn directory_context_resolution_matches(
    active_summon: Option<(i64, i64)>,
    resolution: &nxtools_protocol::QuickLaunchDirectoryContextResolution,
    now_unix_ms: i64,
) -> bool {
    let age = now_unix_ms.saturating_sub(resolution.requested_at_unix_ms);
    age <= REQUEST_MAX_AGE_MS
        && active_summon
            == Some((
                resolution.target_window_handle,
                resolution.requested_at_unix_ms,
            ))
}

pub(crate) fn auto_quick_switch_command(
    settings: &QuickLaunchSettings,
    request: &nxtools_protocol::QuickLaunchAutoSwitchRequest,
    now_unix_ms: i64,
) -> Option<NavigateDirectoryCommand> {
    let age = now_unix_ms.saturating_sub(request.requested_at_unix_ms);
    let source_directory = request.source_directory.trim();
    (settings.enabled
        && settings.auto_quick_switch
        && age <= AUTO_QUICK_SWITCH_REQUEST_MAX_AGE_MS
        && request.target.host == DirectoryNavigationHost::FileDialog
        && request.target.window_handle != 0
        && !source_directory.is_empty())
    .then(|| NavigateDirectoryCommand {
        host: DirectoryNavigationHost::FileDialog,
        window_handle: request.target.window_handle,
        target_directory: source_directory.to_owned(),
        host_context_id: request.target.host_context_id.clone(),
    })
}

pub(crate) fn visible_icon_prefetch_range(
    top_visible: usize,
    bottom_visible: usize,
    result_count: usize,
) -> std::ops::Range<usize> {
    let start = top_visible
        .saturating_sub(ICON_VISIBLE_OVERSCAN)
        .min(result_count);
    let end = bottom_visible
        .saturating_add(1)
        .saturating_add(ICON_VISIBLE_OVERSCAN)
        .min(result_count)
        .max(start);
    start..end
}

pub(crate) fn quick_launch_shortcut_hint(visible_position: usize) -> Option<&'static str> {
    QUICK_LAUNCH_RESULT_SHORTCUT_HINTS
        .get(visible_position)
        .copied()
}

pub(crate) fn quick_launch_shortcut_target(
    key: &str,
    modifiers: gpui_kit::Modifiers,
    visible_range: Range<usize>,
    result_count: usize,
) -> Option<usize> {
    if !modifiers.control
        || modifiers.alt
        || modifiers.shift
        || modifiers.platform
        || modifiers.function
    {
        return None;
    }
    let shortcut_position = key.parse::<usize>().ok()?.checked_sub(1)?;
    QUICK_LAUNCH_RESULT_SHORTCUT_HINTS.get(shortcut_position)?;
    let target = visible_range.start.checked_add(shortcut_position)?;
    (target < visible_range.end && target < result_count).then_some(target)
}

pub(crate) fn quick_launch_match_ranges(text: &str, query: &str) -> Vec<std::ops::Range<usize>> {
    nxtools_finder::FileQuery::parse(query).match_ranges(text)
}

#[cfg(test)]
pub(crate) fn match_ranges_for_pattern(text: &str, pattern: &str) -> Vec<std::ops::Range<usize>> {
    let text_chars = text.char_indices().collect::<Vec<_>>();
    let pattern_chars = pattern.chars().collect::<Vec<_>>();
    if pattern_chars.is_empty() || pattern_chars.len() > text_chars.len() {
        return Vec::new();
    }
    let case_sensitive = pattern_chars
        .iter()
        .any(|character| character.is_uppercase());
    let matches = |left: char, right: char| {
        if case_sensitive {
            left == right
        } else {
            left.to_lowercase().eq(right.to_lowercase())
        }
    };
    for start in 0..=text_chars.len() - pattern_chars.len() {
        if pattern_chars
            .iter()
            .enumerate()
            .all(|(offset, pattern)| matches(text_chars[start + offset].1, *pattern))
        {
            let end_index = start + pattern_chars.len();
            let end = text_chars
                .get(end_index)
                .map(|(index, _)| *index)
                .unwrap_or(text.len());
            return std::iter::once(text_chars[start].0..end).collect();
        }
    }
    let mut ranges = Vec::with_capacity(pattern_chars.len());
    let mut text_offset = 0;
    for pattern in pattern_chars {
        let Some((relative, (byte_index, character))) = text_chars[text_offset..]
            .iter()
            .enumerate()
            .find(|(_, (_, character))| matches(*character, pattern))
        else {
            return Vec::new();
        };
        ranges.push(*byte_index..*byte_index + character.len_utf8());
        text_offset += relative + 1;
    }
    ranges
}

/// Publish current local results without waiting for the indexed provider.
pub(crate) fn should_publish_immediate_results(
    _indexer_connected: bool,
    _search_queued: bool,
) -> bool {
    true
}

/// Only retained rows from an older query stay blocked while a search is queued.
/// Current-query rows are usable before the provider's terminal batch; provider
/// completion remains owned by IndexSearchSession::complete.
pub(crate) fn search_update_pending(search_queued: bool, has_current_results: bool) -> bool {
    search_queued && !has_current_results
}

pub(crate) fn elapsed_microseconds(started: Instant) -> u64 {
    started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
}

pub(crate) fn should_search_indexer(indexer_connected: bool, index_ready: bool) -> bool {
    indexer_connected && index_ready
}

pub(crate) fn index_status_has_searchable_snapshot(status: Option<&IndexStatus>) -> bool {
    status.is_some_and(|status| {
        status.phase == nxtools_protocol::IndexPhase::Ready || status.indexed_items > 0
    })
}

pub(crate) fn should_retry_index_search_after_status(
    previous: Option<&IndexStatus>,
    current: &IndexStatus,
) -> bool {
    let was_ready =
        previous.is_some_and(|status| status.phase == nxtools_protocol::IndexPhase::Ready);
    !was_ready && current.phase == nxtools_protocol::IndexPhase::Ready
}

pub(crate) fn index_status_affects_presentation(
    previous: Option<&IndexStatus>,
    current: &IndexStatus,
) -> bool {
    let Some(previous) = previous else {
        return true;
    };
    previous.phase != current.phase
        || previous.indexed_items != current.indexed_items
        || previous.last_error != current.last_error
        || previous.drives.len() != current.drives.len()
        || previous
            .drives
            .iter()
            .zip(&current.drives)
            .any(|(previous, current)| {
                previous.mount_point != current.mount_point
                    || previous.file_system != current.file_system
                    || previous.indexed_items != current.indexed_items
                    || previous.journal_id != current.journal_id
            })
}

/// True when the presentation changed only through item counts: phase, error
/// and drive structure are identical, so the change is high-frequency file
/// churn telemetry rather than a state transition worth its own log record.
pub(crate) fn index_status_count_only_change(
    previous: Option<&IndexStatus>,
    current: &IndexStatus,
) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    index_status_affects_presentation(Some(previous), current)
        && previous.phase == current.phase
        && previous.last_error == current.last_error
        && previous.drives.len() == current.drives.len()
        && previous
            .drives
            .iter()
            .zip(&current.drives)
            .all(|(previous, current)| {
                previous.mount_point == current.mount_point
                    && previous.file_system == current.file_system
                    && previous.journal_id == current.journal_id
            })
}

#[derive(Default)]
pub(crate) struct ImeEnterGate {
    pub(crate) composition_seen: bool,
    pub(crate) pending_confirmation_enter: bool,
}

#[derive(Default)]
pub(crate) struct PointerSelectionGate {
    pub(crate) row: Option<usize>,
    pub(crate) position: Option<(f32, f32)>,
}

impl PointerSelectionGate {
    pub(crate) fn observe_move(&mut self, row: usize, position: (f32, f32)) -> bool {
        let should_select = self.row == Some(row) && self.position != Some(position);
        self.row = Some(row);
        self.position = Some(position);
        should_select
    }

    pub(crate) fn reset(&mut self) {
        self.row = None;
        self.position = None;
    }
}

impl ImeEnterGate {
    pub(crate) fn reset(&mut self) {
        self.composition_seen = false;
        self.pending_confirmation_enter = false;
    }

    pub(crate) fn observe_composition(&mut self, active: bool) {
        self.composition_seen |= active;
    }

    pub(crate) fn observe_change(&mut self) {
        self.pending_confirmation_enter = self.composition_seen;
        self.composition_seen = false;
    }

    pub(crate) fn consume_enter(&mut self) -> bool {
        !std::mem::take(&mut self.pending_confirmation_enter)
    }
}

pub(crate) struct QuickLaunchSearchUpdate {
    pub(crate) results: Vec<SearchResult>,
    /// Waiting for current-query rows, not for every provider to complete.
    pub(crate) pending: bool,
}

pub(crate) fn apply_search_update<T>(
    committed_results: &mut Vec<T>,
    update_results: Vec<T>,
    pending: bool,
) -> bool {
    if pending && update_results.is_empty() {
        return false;
    }
    *committed_results = update_results;
    true
}

pub(crate) fn should_hide_for_foreground(
    is_open: bool,
    grace_elapsed: bool,
    popup_window_handle: i64,
    preview_window_handle: i64,
    foreground_window_handle: i64,
) -> bool {
    is_open
        && grace_elapsed
        && foreground_window_handle != popup_window_handle
        && (preview_window_handle == 0 || foreground_window_handle != preview_window_handle)
}

pub(crate) fn should_hide_for_pointer(
    is_open: bool,
    popup_window_handle: i64,
    pointer_root_window_handle: i64,
    pointer_process_id: u32,
    desktop_process_id: u32,
) -> bool {
    is_open
        && pointer_root_window_handle != popup_window_handle
        && pointer_process_id != desktop_process_id
}

#[derive(Default)]
pub(crate) struct IndexSearchSession {
    pub(crate) generation: u64,
    pub(crate) query: String,
    pub(crate) items: Vec<IndexSearchItem>,
    pub(crate) complete: bool,
    pub(crate) last_completed_elapsed_us: Option<u64>,
}

#[derive(Default)]
pub(crate) struct IndexerServiceBootstrap {
    pub(crate) pending: Option<uuid::Uuid>,
    pub(crate) pending_automatic: bool,
    pub(crate) automatic_attempted: bool,
}

impl IndexerServiceBootstrap {
    pub(crate) fn on_platform_connected(&mut self) -> bool {
        // A correlation id belongs to one PlatformHost pipe generation. A reconnect can never
        // deliver the old result, so retaining it would permanently disable the manual retry.
        self.pending = None;
        self.pending_automatic = false;
        if self.automatic_attempted {
            return false;
        }
        self.automatic_attempted = true;
        true
    }

    pub(crate) fn begin(&mut self, correlation_id: uuid::Uuid, automatic: bool) {
        self.pending = Some(correlation_id);
        self.pending_automatic = automatic;
    }

    pub(crate) fn finish(&mut self, correlation_id: uuid::Uuid) -> Option<bool> {
        if self.pending != Some(correlation_id) {
            return None;
        }
        self.pending = None;
        Some(std::mem::take(&mut self.pending_automatic))
    }
}

impl IndexSearchSession {
    pub(crate) fn begin(&mut self, query: impl Into<String>, limit: usize) -> IndexSearchRequest {
        self.invalidate();
        self.query = query.into();
        IndexSearchRequest {
            generation: self.generation,
            query: self.query.clone(),
            limit,
            context: IndexSearchContext {
                directory: None,
                preferred_directory: None,
                include_files: true,
                include_directories: true,
                retain_kinds_apart: true,
                allow_homophone_fallback: true,
                recent_modified: recent_modified_filter(&self.query).is_some(),
            },
        }
    }

    pub(crate) fn invalidate(&mut self) {
        self.generation = self.generation.wrapping_add(1).max(1);
        self.query.clear();
        self.items.clear();
        self.complete = false;
    }

    pub(crate) fn apply(&mut self, batch: IndexSearchBatch) -> bool {
        if batch.generation != self.generation {
            return false;
        }
        self.items.extend(batch.items);
        self.complete = batch.complete;
        if batch.complete {
            self.last_completed_elapsed_us = Some(batch.elapsed_us);
        }
        true
    }
}
