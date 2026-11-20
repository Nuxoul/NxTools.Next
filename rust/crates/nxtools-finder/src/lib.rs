use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use rusqlite::{Connection, OptionalExtension, params};

mod calculator;
mod custom_command;
mod environment_variable;
mod extensions;
mod pinyin;
mod query;
mod ranking;
mod windows_applications;
use windows_applications::system_application_aliases;
pub use windows_applications::{InstalledApplication, is_packaged_application_target};

#[cfg(test)]
use ranking::unified_search_rank;
use ranking::{canonical_history_id, merge_usage_record};

pub use calculator::{
    CalculatorAnswer, CalculatorOutput, CalculatorRepresentation, calculator_answer,
};
pub use custom_command::{
    PreparedCustomCommand, command_display_line, matching_custom_commands, prepare_custom_command,
    query_matches_custom_command, validate_custom_command_template,
};
pub use extensions::{prepare_package_action, validate_package_commands};
pub use query::{FileQuery, search_char_mask};

const APPLICATION_EXTENSIONS: [&str; 3] = ["lnk", "url", "exe"];
type UsageRecord = (u64, Option<i64>);
type UsageHistory = HashMap<String, UsageRecord>;
type QueryUsageHistory = HashMap<(String, String), UsageRecord>;
const MAX_RECENT_DIRECTORIES: usize = 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationSource {
    Calculator,
    EnvironmentVariable,
    CustomCommand,
    StartMenu,
    Desktop,
    System,
    IndexedFile,
    IndexedDirectory,
}

impl ApplicationSource {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Calculator => "计算",
            Self::EnvironmentVariable => "环境变量",
            Self::CustomCommand => "命令",
            Self::StartMenu => "开始菜单",
            Self::Desktop => "桌面",
            Self::System => "系统工具",
            Self::IndexedFile => "文件",
            Self::IndexedDirectory => "文件夹",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationEntry {
    pub id: String,
    pub name: String,
    pub target: String,
    pub source: ApplicationSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalSearchEntry {
    pub id: String,
    pub name: String,
    pub target: String,
    pub source: ApplicationSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchResultAction {
    OpenTarget,
    CopyText { text: String },
    RunCommand { command: PreparedCustomCommand },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub entry: ApplicationEntry,
    pub action: SearchResultAction,
    pub subtitle: Option<String>,
    pub score: i64,
    pub activation_count: u64,
    pub last_activated_at_unix_ms: Option<i64>,
    pub query_activation_count: u64,
    pub last_query_activation_at_unix_ms: Option<i64>,
}

impl SearchResult {
    pub fn opens_target(&self) -> bool {
        self.action == SearchResultAction::OpenTarget
    }

    pub fn copy_text(&self) -> Option<&str> {
        match &self.action {
            SearchResultAction::OpenTarget | SearchResultAction::RunCommand { .. } => None,
            SearchResultAction::CopyText { text } => Some(text),
        }
    }

    pub fn prepared_command(&self) -> Option<&PreparedCustomCommand> {
        match &self.action {
            SearchResultAction::RunCommand { command } => Some(command),
            _ => None,
        }
    }
}

pub fn query_requests_file_index(query: &str) -> bool {
    calculator_answer(query).is_none()
        && environment_variable::environment_variable_name(query).is_none()
        && !query
            .trim()
            .strip_prefix('*')
            .unwrap_or(query.trim())
            .trim()
            .is_empty()
}

pub fn query_requests_file_index_with_commands(
    query: &str,
    _commands: &[nxtools_core::CustomCommandSettings],
) -> bool {
    query_requests_file_index(query)
}

fn environment_variable_search_results(
    variable: environment_variable::ResolvedEnvironmentVariable,
    limit: usize,
) -> impl Iterator<Item = SearchResult> {
    let variable_name = variable.name;
    variable
        .targets
        .into_iter()
        .take(limit)
        .enumerate()
        .map(move |(index, target)| {
            let (action, entry_target, action_label) = match target.kind {
                environment_variable::EnvironmentVariableTargetKind::Directory => (
                    SearchResultAction::OpenTarget,
                    target.value.clone(),
                    "打开文件夹",
                ),
                environment_variable::EnvironmentVariableTargetKind::File => (
                    SearchResultAction::OpenTarget,
                    target.value.clone(),
                    "打开文件",
                ),
                environment_variable::EnvironmentVariableTargetKind::CopyValue => (
                    SearchResultAction::CopyText {
                        text: target.value.clone(),
                    },
                    String::new(),
                    "复制值",
                ),
            };
            SearchResult {
                entry: ApplicationEntry {
                    id: format!(
                        "environment-variable:{}:{index}",
                        variable_name.to_ascii_lowercase()
                    ),
                    name: target.value,
                    target: entry_target,
                    source: ApplicationSource::EnvironmentVariable,
                },
                action,
                subtitle: Some(format!("%{variable_name}% · {action_label}")),
                score: 1_000 - index as i64,
                activation_count: 0,
                last_activated_at_unix_ms: None,
                query_activation_count: 0,
                last_query_activation_at_unix_ms: None,
            }
        })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentDirectory {
    pub path: String,
    pub visit_count: u64,
    pub last_visited_at_unix_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedDirectoryVisit {
    path_key: String,
    directory: RecentDirectory,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct UsageStats {
    pub activation_count: u64,
    pub last_activated_at_unix_ms: Option<i64>,
    pub query_activation_count: u64,
    pub last_query_activation_at_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CatalogStatus {
    pub indexed_items: usize,
    pub scanned_roots: usize,
    pub last_scan_at_unix_ms: i64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplicationRoot {
    pub path: PathBuf,
    pub source: ApplicationSource,
}

pub struct ApplicationCatalogSnapshot {
    entries: Vec<ApplicationEntry>,
    status: CatalogStatus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QuickLaunchHistorySummary {
    pub ranked_items: usize,
    pub query_associations: usize,
    pub recent_directories: usize,
    pub total_activations: u64,
    pub total_directory_visits: u64,
}

struct ApplicationMatchMaterial {
    folded_name: String,
    initialism: String,
    pinyin_initials: String,
}

impl ApplicationMatchMaterial {
    fn build(entry: &ApplicationEntry) -> Self {
        Self {
            folded_name: entry.name.to_lowercase(),
            initialism: application_initialism(&entry.name),
            pinyin_initials: pinyin::pinyin_initials(&entry.name),
        }
    }
}

pub struct FinderCatalog {
    entries: Vec<ApplicationEntry>,
    root_entries: Vec<ApplicationEntry>,
    installed_entries: Vec<ApplicationEntry>,
    match_materials: Vec<ApplicationMatchMaterial>,
    history: Connection,
    history_path: PathBuf,
    usage: UsageHistory,
    query_usage: QueryUsageHistory,
    recent_directories: HashMap<String, RecentDirectory>,
    status: CatalogStatus,
    roots: Vec<ApplicationRoot>,
}

impl FinderCatalog {
    pub fn open(history_path: impl AsRef<Path>, roots: &[ApplicationRoot]) -> Result<Self> {
        let history_path = history_path.as_ref();
        if let Some(parent) = history_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let history = Connection::open(history_path)
            .with_context(|| format!("failed to open {}", history_path.display()))?;
        history.execute_batch(
            "CREATE TABLE IF NOT EXISTS quick_launch_usage (
                entry_id TEXT PRIMARY KEY,
                activation_count INTEGER NOT NULL,
                last_activated_at_unix_ms INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS quick_launch_query_usage (
                entry_id TEXT NOT NULL,
                query TEXT NOT NULL,
                activation_count INTEGER NOT NULL,
                last_activated_at_unix_ms INTEGER NOT NULL,
                PRIMARY KEY (entry_id, query)
            );
            CREATE TABLE IF NOT EXISTS quick_launch_directory_history (
                path_key TEXT PRIMARY KEY,
                display_path TEXT NOT NULL,
                visit_count INTEGER NOT NULL,
                last_visited_at_unix_ms INTEGER NOT NULL
            );",
        )?;
        let usage = load_history(&history)?;
        let query_usage = load_query_history(&history)?;
        let recent_directories = load_recent_directories(&history)?;
        let snapshot = scan_application_roots(roots);
        let mut catalog = Self {
            root_entries: snapshot.entries,
            installed_entries: Vec::new(),
            match_materials: Vec::new(),
            entries: Vec::new(),
            history,
            history_path: history_path.to_path_buf(),
            usage,
            query_usage,
            recent_directories,
            status: snapshot.status,
            roots: roots.to_vec(),
        };
        catalog.rebuild_application_entries();
        Ok(catalog)
    }

    pub fn entries(&self) -> &[ApplicationEntry] {
        &self.entries
    }

    pub fn status(&self) -> &CatalogStatus {
        &self.status
    }

    pub fn roots(&self) -> &[ApplicationRoot] {
        &self.roots
    }

    pub fn history_path(&self) -> &Path {
        &self.history_path
    }

    pub fn history_summary(&self) -> QuickLaunchHistorySummary {
        QuickLaunchHistorySummary {
            ranked_items: self.usage.len(),
            query_associations: self.query_usage.len(),
            recent_directories: self.recent_directories.len(),
            total_activations: self
                .usage
                .values()
                .fold(0_u64, |total, (count, _)| total.saturating_add(*count)),
            total_directory_visits: self
                .recent_directories
                .values()
                .fold(0_u64, |total, directory| {
                    total.saturating_add(directory.visit_count)
                }),
        }
    }

    pub fn clear_history(&mut self) -> Result<()> {
        let transaction = self.history.transaction()?;
        transaction.execute_batch(
            "DELETE FROM quick_launch_usage;
             DELETE FROM quick_launch_query_usage;
             DELETE FROM quick_launch_directory_history;",
        )?;
        transaction.commit()?;
        self.usage.clear();
        self.query_usage.clear();
        self.recent_directories.clear();
        Ok(())
    }

    pub fn refresh(&mut self, roots: &[ApplicationRoot]) {
        self.roots = roots.to_vec();
        self.replace_snapshot(scan_application_roots(roots));
    }

    pub fn replace_snapshot(&mut self, snapshot: ApplicationCatalogSnapshot) {
        self.root_entries = snapshot.entries;
        self.status = snapshot.status;
        self.rebuild_application_entries();
    }

    pub fn apply_refresh_snapshot(
        &mut self,
        snapshot: ApplicationCatalogSnapshot,
    ) -> std::result::Result<(), Vec<String>> {
        if snapshot.status.errors.is_empty() {
            self.replace_snapshot(snapshot);
            return Ok(());
        }
        let errors = snapshot.status.errors;
        self.status.scanned_roots = snapshot.status.scanned_roots;
        self.status.last_scan_at_unix_ms = snapshot.status.last_scan_at_unix_ms;
        self.status.errors = errors.clone();
        Err(errors)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search_with_external(query, &[], limit)
    }

    pub fn search_with_external(
        &self,
        query: &str,
        external_entries: &[ExternalSearchEntry],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        self.search_with_sources(query, external_entries, &[], limit)
    }

    pub fn search_with_sources(
        &self,
        query: &str,
        external_entries: &[ExternalSearchEntry],
        custom_commands: &[nxtools_core::CustomCommandSettings],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let normal =
            self.search_with_sources_normal(query, external_entries, custom_commands, limit)?;
        if !normal.is_empty() || limit == 0 {
            return Ok(normal);
        }
        let variants = FileQuery::parse(query).homophone_variants();
        if variants.is_empty() {
            return Ok(normal);
        }
        let normalized_query = normalize(query);
        let matches = self
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.id.as_str(),
                    entry.name.as_str(),
                    entry.target.as_str(),
                    entry.source,
                )
            })
            .chain(external_entries.iter().map(|entry| {
                (
                    entry.id.as_str(),
                    entry.name.as_str(),
                    entry.target.as_str(),
                    entry.source,
                )
            }))
            .filter_map(|(id, name, target, source)| {
                let score = variants
                    .iter()
                    .filter_map(|variant| variant.score(name, target))
                    .max()?;
                let usage = self.usage_for(id, &normalized_query);
                Some(SearchResult {
                    entry: ApplicationEntry {
                        id: id.into(),
                        name: name.into(),
                        target: target.into(),
                        source,
                    },
                    action: SearchResultAction::OpenTarget,
                    subtitle: Some(format!("同音近似匹配 · {target}")),
                    score,
                    activation_count: usage.activation_count,
                    last_activated_at_unix_ms: usage.last_activated_at_unix_ms,
                    query_activation_count: usage.query_activation_count,
                    last_query_activation_at_unix_ms: usage.last_query_activation_at_unix_ms,
                })
            })
            .collect::<Vec<_>>();
        Ok(self.rank_results(matches, &normalized_query, limit))
    }

    /// Ordinary provider phase, before all providers have completed.
    pub fn search_with_sources_normal(
        &self,
        query: &str,
        external_entries: &[ExternalSearchEntry],
        custom_commands: &[nxtools_core::CustomCommandSettings],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let matches = self.search_candidates(query, external_entries, custom_commands, limit)?;
        Ok(self.rank_results(matches, query, limit))
    }

    fn search_candidates(
        &self,
        query: &str,
        external_entries: &[ExternalSearchEntry],
        custom_commands: &[nxtools_core::CustomCommandSettings],
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let mut matches = Vec::new();
        self.visit_search_candidates(query, external_entries, custom_commands, limit, |result| {
            matches.push(result);
            true
        })?;
        Ok(matches)
    }

    fn visit_search_candidates(
        &self,
        query: &str,
        external_entries: &[ExternalSearchEntry],
        custom_commands: &[nxtools_core::CustomCommandSettings],
        limit: usize,
        mut visit: impl FnMut(SearchResult) -> bool,
    ) -> Result<()> {
        if limit == 0 {
            return Ok(());
        }
        let raw_query = query;
        let file_query = FileQuery::parse(raw_query);
        let calculator_answer = calculator_answer(query);
        let environment_variable = environment_variable::environment_variable_targets(query);
        let query = normalize(query);
        if query.is_empty() {
            return Ok(());
        }
        if let Some(variable) = environment_variable {
            for result in environment_variable_search_results(variable, limit) {
                if !visit(result) {
                    break;
                }
            }
            return Ok(());
        }
        let command_results = matching_custom_commands(raw_query, custom_commands)
            .into_iter()
            .take(limit)
            .enumerate()
            .filter_map(|(index, (command, input))| {
                let prepared = prepare_custom_command(command, input.as_deref()).ok()?;
                let display_line = command_display_line(&prepared);
                let entry_id = format!("custom-command:{}", command.keyword.to_lowercase());
                let usage = self.usage_for(&entry_id, &query);
                Some(SearchResult {
                    entry: ApplicationEntry {
                        id: entry_id,
                        name: if command.title.is_empty() {
                            command.keyword.clone()
                        } else {
                            command.title.clone()
                        },
                        target: prepared.executable.clone(),
                        source: ApplicationSource::CustomCommand,
                    },
                    action: SearchResultAction::RunCommand { command: prepared },
                    subtitle: Some(display_line),
                    score: 10_000 - index as i64,
                    activation_count: usage.activation_count,
                    last_activated_at_unix_ms: usage.last_activated_at_unix_ms,
                    query_activation_count: usage.query_activation_count,
                    last_query_activation_at_unix_ms: usage.last_query_activation_at_unix_ms,
                })
            });
        for result in command_results {
            if !visit(result) {
                return Ok(());
            }
        }
        let applications =
            self.entries
                .iter()
                .zip(&self.match_materials)
                .filter_map(|(entry, material)| {
                    let direct_score = file_query
                        .score_prepared(&entry.name, &material.folded_name, &entry.target)
                        .into_iter()
                        .chain(file_query.score_literal(&material.initialism, &entry.target))
                        .chain(
                            system_application_aliases(entry)
                                .iter()
                                .filter_map(|alias| file_query.score_literal(alias, &entry.target))
                                .max(),
                        )
                        .max();
                    // The pinyin channel only contributes below the direct tiers so a
                    // one-letter query cannot fuzzy-match the initials of an ASCII
                    // name that already matched directly.
                    let textual_score = direct_score
                        .filter(|score| *score >= 2_000)
                        .or_else(|| {
                            file_query.score_literal(&material.pinyin_initials, &entry.target)
                        })
                        .or_else(|| {
                            file_query
                                .score_literal(&entry.name, &entry.target)
                                .is_none()
                                .then(|| file_query.score(&entry.name, &entry.target))
                                .flatten()
                        })?;
                    let usage = self.usage_for(&entry.id, &query);
                    Some(SearchResult {
                        entry: entry.clone(),
                        action: SearchResultAction::OpenTarget,
                        subtitle: None,
                        score: textual_score,
                        activation_count: usage.activation_count,
                        last_activated_at_unix_ms: usage.last_activated_at_unix_ms,
                        query_activation_count: usage.query_activation_count,
                        last_query_activation_at_unix_ms: usage.last_query_activation_at_unix_ms,
                    })
                });
        for result in applications {
            if !visit(result) {
                return Ok(());
            }
        }
        if let Some(answer) = calculator_answer {
            let expression_id = normalize(&answer.expression);
            let calculator_results =
                answer
                    .outputs
                    .into_iter()
                    .enumerate()
                    .map(|(index, output)| SearchResult {
                        entry: ApplicationEntry {
                            id: format!(
                                "calculator:{expression_id}:{}",
                                output.representation.id()
                            ),
                            name: output.value.clone(),
                            target: String::new(),
                            source: ApplicationSource::Calculator,
                        },
                        action: SearchResultAction::CopyText { text: output.value },
                        subtitle: Some(output.representation.label().into()),
                        score: 1_000 - index as i64,
                        activation_count: 0,
                        last_activated_at_unix_ms: None,
                        query_activation_count: 0,
                        last_query_activation_at_unix_ms: None,
                    });
            for result in calculator_results {
                if !visit(result) {
                    return Ok(());
                }
            }
        }
        let external = external_entries.iter().filter_map(|entry| {
            let textual_score = file_query.score(&entry.name, &entry.target)?;
            let usage = self.usage_for(&entry.id, &query);
            Some(SearchResult {
                entry: ApplicationEntry {
                    id: entry.id.clone(),
                    name: entry.name.clone(),
                    target: entry.target.clone(),
                    source: entry.source,
                },
                action: SearchResultAction::OpenTarget,
                subtitle: None,
                // This score is recomputed by the shared Rust query language. Provider-local
                // scores are intentionally not accepted at this boundary.
                score: textual_score,
                activation_count: usage.activation_count,
                last_activated_at_unix_ms: usage.last_activated_at_unix_ms,
                query_activation_count: usage.query_activation_count,
                last_query_activation_at_unix_ms: usage.last_query_activation_at_unix_ms,
            })
        });
        for result in external {
            if !visit(result) {
                return Ok(());
            }
        }
        Ok(())
    }

    fn usage_for(&self, entry_id: &str, query: &str) -> UsageStats {
        let entry_id = canonical_history_id(entry_id);
        let (activation_count, last_activated_at_unix_ms) =
            self.usage.get(&entry_id).copied().unwrap_or((0, None));
        let query = normalize(query);
        let (query_activation_count, last_query_activation_at_unix_ms) = self
            .query_usage
            .get(&(entry_id.to_owned(), query))
            .copied()
            .unwrap_or((0, None));
        UsageStats {
            activation_count,
            last_activated_at_unix_ms,
            query_activation_count,
            last_query_activation_at_unix_ms,
        }
    }

    pub fn record_activation(
        &mut self,
        entry_id: &str,
        query: &str,
        activated_at_unix_ms: i64,
    ) -> Result<()> {
        let entry_id = canonical_history_id(entry_id);
        let query = normalize(query);
        let transaction = self.history.transaction()?;
        transaction.execute(
            "INSERT INTO quick_launch_usage (
                entry_id, activation_count, last_activated_at_unix_ms
            ) VALUES (?1, 1, ?2)
            ON CONFLICT(entry_id) DO UPDATE SET
                activation_count = activation_count + 1,
                last_activated_at_unix_ms = excluded.last_activated_at_unix_ms",
            params![entry_id, activated_at_unix_ms],
        )?;
        if !query.is_empty() {
            transaction.execute(
                "INSERT INTO quick_launch_query_usage (
                    entry_id, query, activation_count, last_activated_at_unix_ms
                ) VALUES (?1, ?2, 1, ?3)
                ON CONFLICT(entry_id, query) DO UPDATE SET
                    activation_count = activation_count + 1,
                    last_activated_at_unix_ms = excluded.last_activated_at_unix_ms",
                params![entry_id, query, activated_at_unix_ms],
            )?;
        }
        transaction.commit()?;
        let usage = self.usage.entry(entry_id.to_owned()).or_insert((0, None));
        usage.0 = usage.0.saturating_add(1);
        usage.1 = Some(activated_at_unix_ms);
        if !query.is_empty() {
            let usage = self
                .query_usage
                .entry((entry_id.to_owned(), query))
                .or_insert((0, None));
            usage.0 = usage.0.saturating_add(1);
            usage.1 = Some(activated_at_unix_ms);
        }
        Ok(())
    }

    pub fn record_directory_visit(
        &mut self,
        path: impl AsRef<Path>,
        visited_at_unix_ms: i64,
    ) -> Result<bool> {
        let Some(recorded) = persist_directory_visit_with_connection(
            &mut self.history,
            path.as_ref(),
            visited_at_unix_ms,
        )?
        else {
            return Ok(false);
        };
        self.apply_recorded_directory_visit(recorded);
        Ok(true)
    }

    pub fn persist_directory_visit(
        history_path: impl AsRef<Path>,
        path: impl AsRef<Path>,
        visited_at_unix_ms: i64,
    ) -> Result<Option<RecordedDirectoryVisit>> {
        let mut history = Connection::open(history_path.as_ref())
            .with_context(|| format!("failed to open {}", history_path.as_ref().display()))?;
        history.busy_timeout(Duration::from_secs(2))?;
        persist_directory_visit_with_connection(&mut history, path.as_ref(), visited_at_unix_ms)
    }

    pub fn apply_recorded_directory_visit(&mut self, recorded: RecordedDirectoryVisit) {
        let entry = self
            .recent_directories
            .entry(recorded.path_key)
            .or_insert_with(|| recorded.directory.clone());
        if recorded.directory.last_visited_at_unix_ms >= entry.last_visited_at_unix_ms {
            entry.path = recorded.directory.path;
        }
        entry.visit_count = entry.visit_count.max(recorded.directory.visit_count);
        entry.last_visited_at_unix_ms = entry
            .last_visited_at_unix_ms
            .max(recorded.directory.last_visited_at_unix_ms);
        trim_recent_directories(&mut self.recent_directories);
    }

    pub fn recent_directories(&self, limit: usize) -> Vec<RecentDirectory> {
        let mut recent = self
            .recent_directories
            .values()
            .cloned()
            .collect::<Vec<_>>();
        sort_recent_directories(&mut recent);
        recent.truncate(limit);
        recent
    }
}

fn load_history(connection: &Connection) -> Result<UsageHistory> {
    let mut statement = connection.prepare(
        "SELECT entry_id, activation_count, last_activated_at_unix_ms
         FROM quick_launch_usage",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            (
                row.get::<_, i64>(1)?.max(0) as u64,
                Some(row.get::<_, i64>(2)?),
            ),
        ))
    })?;
    let mut history = UsageHistory::new();
    for row in rows {
        let (id, usage) = row?;
        merge_usage_record(history.entry(canonical_history_id(&id)).or_default(), usage);
    }
    Ok(history)
}

fn load_query_history(connection: &Connection) -> Result<QueryUsageHistory> {
    let mut statement = connection.prepare(
        "SELECT entry_id, query, activation_count, last_activated_at_unix_ms
         FROM quick_launch_query_usage",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            (row.get::<_, String>(0)?, row.get::<_, String>(1)?),
            (
                row.get::<_, i64>(2)?.max(0) as u64,
                Some(row.get::<_, i64>(3)?),
            ),
        ))
    })?;
    let mut history = QueryUsageHistory::new();
    for row in rows {
        let ((id, query), usage) = row?;
        merge_usage_record(
            history
                .entry((canonical_history_id(&id), normalize(&query)))
                .or_default(),
            usage,
        );
    }
    Ok(history)
}

fn load_recent_directories(connection: &Connection) -> Result<HashMap<String, RecentDirectory>> {
    let mut statement = connection.prepare(
        "SELECT path_key, display_path, visit_count, last_visited_at_unix_ms
         FROM quick_launch_directory_history
         ORDER BY last_visited_at_unix_ms DESC, visit_count DESC, path_key ASC
         LIMIT ?1",
    )?;
    let rows = statement.query_map(params![MAX_RECENT_DIRECTORIES as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            RecentDirectory {
                path: row.get(1)?,
                visit_count: row.get::<_, i64>(2)?.max(0) as u64,
                last_visited_at_unix_ms: row.get(3)?,
            },
        ))
    })?;
    Ok(rows.filter_map(Result::ok).collect())
}

fn persist_directory_visit_with_connection(
    history: &mut Connection,
    path: &Path,
    visited_at_unix_ms: i64,
) -> Result<Option<RecordedDirectoryVisit>> {
    let Some((path_key, display_path)) = normalize_recent_directory(path) else {
        return Ok(None);
    };
    let transaction = history.transaction()?;
    transaction.execute(
        "INSERT INTO quick_launch_directory_history (
            path_key, display_path, visit_count, last_visited_at_unix_ms
        ) VALUES (?1, ?2, 1, ?3)
        ON CONFLICT(path_key) DO UPDATE SET
            display_path = CASE
                WHEN excluded.last_visited_at_unix_ms >= last_visited_at_unix_ms
                THEN excluded.display_path
                ELSE display_path
            END,
            visit_count = visit_count + 1,
            last_visited_at_unix_ms = MAX(
                last_visited_at_unix_ms,
                excluded.last_visited_at_unix_ms
            )",
        params![path_key, display_path, visited_at_unix_ms],
    )?;
    transaction.execute(
        "DELETE FROM quick_launch_directory_history
         WHERE path_key NOT IN (
             SELECT path_key
             FROM quick_launch_directory_history
             ORDER BY last_visited_at_unix_ms DESC, visit_count DESC, path_key ASC
             LIMIT ?1
         )",
        params![MAX_RECENT_DIRECTORIES as i64],
    )?;
    let directory = transaction
        .query_row(
            "SELECT display_path, visit_count, last_visited_at_unix_ms
             FROM quick_launch_directory_history
             WHERE path_key = ?1",
            params![path_key],
            |row| {
                Ok(RecentDirectory {
                    path: row.get(0)?,
                    visit_count: row.get::<_, i64>(1)?.max(0) as u64,
                    last_visited_at_unix_ms: row.get(2)?,
                })
            },
        )
        .optional()?;
    transaction.commit()?;
    Ok(directory.map(|directory| RecordedDirectoryVisit {
        path_key,
        directory,
    }))
}

fn normalize_recent_directory(path: &Path) -> Option<(String, String)> {
    let path = lexical_absolute_path(path)?;
    if !path.is_dir() {
        return None;
    }
    let mut display_path = path.to_string_lossy().replace('/', "\\");
    while display_path.len() > 3 && display_path.ends_with('\\') {
        display_path.pop();
    }
    if display_path.is_empty() {
        return None;
    }
    Some((display_path.to_lowercase(), display_path))
}

fn lexical_absolute_path(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let _ = normalized.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    Some(normalized)
}

fn sort_recent_directories(recent: &mut [RecentDirectory]) {
    recent.sort_by(compare_recent_directories);
}

fn compare_recent_directories(
    left: &RecentDirectory,
    right: &RecentDirectory,
) -> std::cmp::Ordering {
    right
        .last_visited_at_unix_ms
        .cmp(&left.last_visited_at_unix_ms)
        .then_with(|| right.visit_count.cmp(&left.visit_count))
        .then_with(|| left.path.to_lowercase().cmp(&right.path.to_lowercase()))
}

fn trim_recent_directories(recent: &mut HashMap<String, RecentDirectory>) {
    if recent.len() <= MAX_RECENT_DIRECTORIES {
        return;
    }
    let mut ranked = recent
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect::<Vec<_>>();
    ranked.sort_by(|(_, left), (_, right)| compare_recent_directories(left, right));
    for (key, _) in ranked.into_iter().skip(MAX_RECENT_DIRECTORIES) {
        recent.remove(&key);
    }
}

pub fn default_application_roots() -> Vec<ApplicationRoot> {
    let mut roots = Vec::new();
    if let Some(program_data) = std::env::var_os("ProgramData") {
        roots.push(ApplicationRoot {
            path: PathBuf::from(program_data).join(r"Microsoft\Windows\Start Menu\Programs"),
            source: ApplicationSource::StartMenu,
        });
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        roots.push(ApplicationRoot {
            path: PathBuf::from(app_data).join(r"Microsoft\Windows\Start Menu\Programs"),
            source: ApplicationSource::StartMenu,
        });
    }
    if let Some(public) = std::env::var_os("PUBLIC") {
        roots.push(ApplicationRoot {
            path: PathBuf::from(public).join("Desktop"),
            source: ApplicationSource::Desktop,
        });
    }
    if let Some(profile) = std::env::var_os("USERPROFILE") {
        roots.push(ApplicationRoot {
            path: PathBuf::from(profile).join("Desktop"),
            source: ApplicationSource::Desktop,
        });
    }
    roots
}

pub fn scan_application_roots(roots: &[ApplicationRoot]) -> ApplicationCatalogSnapshot {
    let mut entries = system_entries();
    let mut seen = entries
        .iter()
        .map(|entry| entry.target.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let mut errors = Vec::new();
    let mut scanned_roots = 0;
    for root in roots {
        if !root.path.exists() {
            continue;
        }
        scanned_roots += 1;
        scan_directory(
            &root.path,
            root.source,
            &mut entries,
            &mut seen,
            &mut errors,
        );
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    let status = CatalogStatus {
        indexed_items: entries.len(),
        scanned_roots,
        last_scan_at_unix_ms: unix_time_ms(),
        errors,
    };
    ApplicationCatalogSnapshot { entries, status }
}

fn scan_directory(
    directory: &Path,
    source: ApplicationSource,
    entries: &mut Vec<ApplicationEntry>,
    seen: &mut HashSet<String>,
    errors: &mut Vec<String>,
) {
    let read_dir = match fs::read_dir(directory) {
        Ok(read_dir) => read_dir,
        Err(error) => {
            errors.push(format!("{}: {error}", directory.display()));
            return;
        }
    };
    for item in read_dir {
        let item = match item {
            Ok(item) => item,
            Err(error) => {
                errors.push(format!("{}: {error}", directory.display()));
                continue;
            }
        };
        let path = item.path();
        let file_type = match item.file_type() {
            Ok(file_type) => file_type,
            Err(error) => {
                errors.push(format!("{}: {error}", path.display()));
                continue;
            }
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            scan_directory(&path, source, entries, seen, errors);
            continue;
        }
        let supported = path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                APPLICATION_EXTENSIONS
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(extension))
            });
        if !supported {
            continue;
        }
        let target = path.to_string_lossy().into_owned();
        if !seen.insert(target.to_ascii_lowercase()) {
            continue;
        }
        let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
            continue;
        };
        entries.push(ApplicationEntry {
            id: stable_entry_id(&target),
            name: name.to_owned(),
            target,
            source,
        });
    }
}

fn system_entries() -> Vec<ApplicationEntry> {
    [
        ("命令提示符", "cmd.exe"),
        ("PowerShell", "powershell.exe"),
        ("任务管理器", "taskmgr.exe"),
        ("系统配置", "msconfig.exe"),
        ("注册表编辑器", "regedit.exe"),
        ("控制面板", "control.exe"),
        ("记事本", "notepad.exe"),
        ("计算器", "calc.exe"),
        ("画图", "mspaint.exe"),
        ("设置", "ms-settings:"),
    ]
    .into_iter()
    .map(|(name, target)| ApplicationEntry {
        id: stable_entry_id(target),
        name: name.into(),
        target: target.into(),
        source: ApplicationSource::System,
    })
    .collect()
}

fn stable_entry_id(target: &str) -> String {
    format!("app:{}", normalize_target(target))
}

fn normalize(value: &str) -> String {
    value.trim().to_lowercase()
}

fn normalize_target(value: &str) -> String {
    value.replace('/', "\\").to_lowercase()
}

fn application_initialism(value: &str) -> String {
    let mut initialism = String::new();
    let mut at_word_start = true;
    for character in value.chars() {
        if !character.is_alphanumeric() {
            at_word_start = true;
            continue;
        }
        if at_word_start || character.is_uppercase() {
            initialism.extend(character.to_lowercase());
        }
        at_word_start = false;
    }
    initialism
}

fn unix_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn catalog_with_files(files: &[&str]) -> (TempDir, FinderCatalog) {
        let directory = TempDir::new().expect("temporary application directory");
        for file in files {
            let path = directory.path().join(file);
            fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
            fs::write(path, b"fixture").expect("fixture shortcut");
        }
        let history_path = directory.path().join("usage.db");
        let catalog = FinderCatalog::open(
            history_path,
            &[ApplicationRoot {
                path: directory.path().to_path_buf(),
                source: ApplicationSource::StartMenu,
            }],
        )
        .expect("open catalog");
        (directory, catalog)
    }

    #[test]
    fn environment_variable_mapping_preserves_file_and_copy_actions() {
        let root = TempDir::new().expect("temporary environment targets");
        let file = root.path().join("tool.exe");
        let missing = root.path().join("missing");
        fs::write(&file, b"fixture").expect("create environment file target");
        let targets = environment_variable::environment_variable_targets_from_value(&format!(
            "{};{};{}",
            root.path().display(),
            file.display(),
            missing.display()
        ));

        let results = environment_variable_search_results(
            environment_variable::ResolvedEnvironmentVariable {
                name: "NXTOOLS_TEST_PATHS".into(),
                targets,
            },
            10,
        )
        .collect::<Vec<_>>();

        assert_eq!(results.len(), 3);
        assert_eq!(results[0].action, SearchResultAction::OpenTarget);
        assert_eq!(results[0].entry.target, results[0].entry.name);
        assert_eq!(results[1].action, SearchResultAction::OpenTarget);
        assert_eq!(results[1].entry.target, results[1].entry.name);
        assert_eq!(
            results[2].action,
            SearchResultAction::CopyText {
                text: missing.to_string_lossy().into_owned(),
            }
        );
        assert!(results[2].entry.target.is_empty());
    }

    #[test]
    fn application_scan_is_recursive_filters_extensions_and_deduplicates_targets() {
        let (_directory, catalog) = catalog_with_files(&[
            "Development/Visual Studio Code.lnk",
            "Development/readme.txt",
            "Browser.url",
        ]);

        assert!(
            catalog
                .entries()
                .iter()
                .any(|entry| entry.name == "Visual Studio Code")
        );
        assert!(
            catalog
                .entries()
                .iter()
                .any(|entry| entry.name == "Browser")
        );
        assert!(!catalog.entries().iter().any(|entry| entry.name == "readme"));
        assert_eq!(catalog.status().scanned_roots, 1);
    }

    #[test]
    fn fuzzy_search_matches_non_contiguous_application_name() {
        let (_directory, catalog) =
            catalog_with_files(&["Visual Studio Code.lnk", "Visual Studio.lnk"]);

        let results = catalog.search("vsc", 10).expect("search catalog");
        assert_eq!(
            results.first().map(|item| item.entry.name.as_str()),
            Some("Visual Studio Code")
        );
    }

    #[test]
    fn an_empty_query_does_not_publish_an_initial_catalog() {
        let (_directory, catalog) = catalog_with_files(&["Alpha.lnk"]);

        assert!(catalog.search("", 10).expect("empty query").is_empty());
        assert!(catalog.search("   ", 10).expect("blank query").is_empty());
    }

    #[test]
    fn a_strong_launcher_match_precedes_an_ordinary_indexed_path() {
        let (_directory, catalog) = catalog_with_files(&["NxTools Launcher.lnk"]);
        let external = [ExternalSearchEntry {
            id: "indexed:nxtools".into(),
            name: "NxTools".into(),
            target: r"C:\Work\NxTools".into(),
            source: ApplicationSource::IndexedDirectory,
        }];

        let results = catalog
            .search_with_external("nxtools", &external, 8)
            .expect("unified search");

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].entry.source, ApplicationSource::StartMenu);
        assert!(results[0].entry.target.ends_with("NxTools Launcher.lnk"));
    }

    #[test]
    fn windows_notepad_and_settings_are_application_matches_in_both_languages() {
        let (_directory, catalog) = catalog_with_files(&["记事本2.0.lnk", "设置工具.lnk"]);
        for (query, name, target) in [
            ("记事本", "记事本", "notepad.exe"),
            ("notepad", "记事本", "notepad.exe"),
            ("设置", "设置", "ms-settings:"),
            ("windows settings", "设置", "ms-settings:"),
        ] {
            let external = [ExternalSearchEntry {
                id: format!("fixture:{query}"),
                name: query.into(),
                target: format!(r"C:\Fixture\{query}.txt"),
                source: ApplicationSource::IndexedFile,
            }];
            let results = catalog
                .search_with_external(query, &external, 8)
                .expect("merged results");
            let first = results.first().expect("system application result");
            assert_eq!(first.entry.source, ApplicationSource::System, "{query}");
            assert_eq!(first.entry.name, name);
            assert_eq!(first.entry.target, target);
            assert_eq!(first.action, SearchResultAction::OpenTarget);
        }
    }

    #[test]
    fn ordinary_executable_and_directory_rank_by_match_without_extension_bonus() {
        let (_directory, catalog) = catalog_with_files(&[]);
        let external = [
            ExternalSearchEntry {
                id: "indexed:ayugram".into(),
                name: "AyuGram.exe".into(),
                target: r"D:\Tools\AyuGram\AyuGram.exe".into(),
                source: ApplicationSource::IndexedFile,
            },
            ExternalSearchEntry {
                id: "indexed:ayugram-directory".into(),
                name: "AyuGram".into(),
                target: r"D:\Tools\AyuGram".into(),
                source: ApplicationSource::IndexedDirectory,
            },
            ExternalSearchEntry {
                id: "indexed:gram".into(),
                name: "gram".into(),
                target: r"C:\Users\Administrator\.cmd.root\mod\gram".into(),
                source: ApplicationSource::IndexedDirectory,
            },
            ExternalSearchEntry {
                id: "indexed:grammar".into(),
                name: "grammar".into(),
                target: r"D:\Code\registry\src\index.crates.io\lalrpop\src\grammar".into(),
                source: ApplicationSource::IndexedDirectory,
            },
        ];

        let results = catalog
            .search_with_external("gram", &external, 50)
            .expect("mixed launcher search");

        assert_eq!(
            results.first().map(|result| result.entry.target.as_str()),
            Some(r"D:\Tools\AyuGram")
        );
        assert_eq!(
            results.get(1).map(|result| result.entry.target.as_str()),
            Some(r"D:\Tools\AyuGram\AyuGram.exe")
        );
    }

    #[test]
    fn exact_application_initialism_beats_a_shorter_incidental_subsequence() {
        let (_directory, catalog) =
            catalog_with_files(&["Vivid Screen Capture.lnk", "Viscera.lnk"]);

        let results = catalog.search("vsc", 10).expect("search catalog");

        assert_eq!(
            results.first().map(|item| item.entry.name.as_str()),
            Some("Vivid Screen Capture")
        );
    }

    #[test]
    fn weak_application_match_follows_strong_general_results() {
        let (_directory, catalog) = catalog_with_files(&["Helium.lnk"]);
        let external = [
            ExternalSearchEntry {
                id: "indexed:hume".into(),
                name: "hume".into(),
                target: r"D:\Code\heybox-src\sources\com\bytedance\hume".into(),
                source: ApplicationSource::IndexedDirectory,
            },
            ExternalSearchEntry {
                id: "indexed:humanoid".into(),
                name: "Humanoid".into(),
                target: r"D:\Roblox\New\src\Workspace\Frost King Bee\Humanoid".into(),
                source: ApplicationSource::IndexedDirectory,
            },
        ];

        let results = catalog
            .search_with_external("hum", &external, 10)
            .expect("unified search");

        assert_eq!(
            results.first().map(|result| result.entry.name.as_str()),
            Some("hume")
        );
        assert!(results.iter().any(|result| result.entry.name == "Helium"));
    }

    #[test]
    fn activation_history_promotes_a_frequently_used_result_and_survives_reopen() {
        let directory = TempDir::new().expect("temporary catalog");
        fs::write(directory.path().join("Alpha Editor.lnk"), b"fixture").expect("alpha");
        fs::write(directory.path().join("Alpha Notes.lnk"), b"fixture").expect("notes");
        let database = directory.path().join("usage.db");
        let roots = [ApplicationRoot {
            path: directory.path().to_path_buf(),
            source: ApplicationSource::StartMenu,
        }];
        let mut catalog = FinderCatalog::open(&database, &roots).expect("open catalog");
        let notes = catalog
            .entries()
            .iter()
            .find(|entry| entry.name == "Alpha Notes")
            .expect("notes entry")
            .id
            .clone();
        catalog
            .record_activation(&notes, "alpha", 1_700_000_000_000)
            .expect("first activation");
        catalog
            .record_activation(&notes, "alpha", 1_700_000_001_000)
            .expect("second activation");
        drop(catalog);

        let reopened = FinderCatalog::open(&database, &roots).expect("reopen catalog");
        let results = reopened.search("alpha", 10).expect("search catalog");
        assert_eq!(
            results.first().map(|item| item.entry.name.as_str()),
            Some("Alpha Notes")
        );
        assert_eq!(results.first().map(|item| item.activation_count), Some(2));
        assert_eq!(
            results.first().map(|item| item.query_activation_count),
            Some(2)
        );
        let unrelated = reopened.usage_for(&notes, "editor");
        assert_eq!(unrelated.activation_count, 2);
        assert_eq!(unrelated.query_activation_count, 0);
    }

    #[test]
    fn recent_directories_are_deduplicated_ranked_and_persisted() {
        let directory = TempDir::new().expect("temporary catalog");
        let older = directory.path().join("Older");
        let newer = directory.path().join("Newer");
        fs::create_dir_all(&older).expect("older directory");
        fs::create_dir_all(&newer).expect("newer directory");
        let database = directory.path().join("usage.db");
        let mut catalog = FinderCatalog::open(&database, &[]).expect("open catalog");

        assert!(
            catalog
                .record_directory_visit(&older, 1_700_000_000_000)
                .expect("record older")
        );
        assert!(
            catalog
                .record_directory_visit(
                    older.join(".").to_string_lossy().replace('\\', "/"),
                    1_700_000_001_000,
                )
                .expect("record lexically equivalent duplicate")
        );
        assert!(
            catalog
                .record_directory_visit(&newer, 1_700_000_002_000)
                .expect("record newer")
        );
        assert!(
            !catalog
                .record_directory_visit("relative-directory", 1_700_000_003_000)
                .expect("reject relative")
        );

        let recent = catalog.recent_directories(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(Path::new(&recent[0].path), newer.as_path());
        assert_eq!(Path::new(&recent[1].path), older.as_path());
        assert_eq!(recent[1].visit_count, 2);
        drop(catalog);

        let reopened = FinderCatalog::open(&database, &[]).expect("reopen catalog");
        let recent = reopened.recent_directories(10);
        assert_eq!(recent.len(), 2);
        assert_eq!(Path::new(&recent[0].path), newer.as_path());
        assert_eq!(recent[1].visit_count, 2);
    }

    #[test]
    fn history_summary_and_clear_cover_ranking_queries_and_recent_directories() {
        let directory = TempDir::new().expect("temporary catalog");
        let target = directory.path().join("Target");
        fs::create_dir_all(&target).expect("target directory");
        let database = directory.path().join("usage.db");
        let mut catalog = FinderCatalog::open(&database, &[]).expect("open catalog");

        catalog
            .record_activation("editor", "edit", 1_700_000_000_000)
            .expect("first activation");
        catalog
            .record_activation("editor", "edit", 1_700_000_001_000)
            .expect("second activation");
        catalog
            .record_activation("terminal", "", 1_700_000_002_000)
            .expect("activation without query");
        catalog
            .record_directory_visit(&target, 1_700_000_003_000)
            .expect("directory visit");

        assert_eq!(
            catalog.history_summary(),
            QuickLaunchHistorySummary {
                ranked_items: 2,
                query_associations: 1,
                recent_directories: 1,
                total_activations: 3,
                total_directory_visits: 1,
            }
        );

        catalog.clear_history().expect("clear history");
        assert_eq!(
            catalog.history_summary(),
            QuickLaunchHistorySummary::default()
        );
        assert!(catalog.recent_directories(10).is_empty());
        drop(catalog);

        let reopened = FinderCatalog::open(&database, &[]).expect("reopen catalog");
        assert_eq!(
            reopened.history_summary(),
            QuickLaunchHistorySummary::default()
        );
    }

    #[test]
    fn a_new_query_has_no_dependency_on_results_from_an_older_query() {
        let (_directory, catalog) =
            catalog_with_files(&["Calculator.lnk", "Calendar.lnk", "Notepad.lnk"]);

        let old = catalog.search("cal", 10).expect("old query");
        let new = catalog.search("note", 10).expect("new query");
        assert!(old.iter().any(|result| result.entry.name == "Calculator"));
        assert_eq!(
            new.first().map(|result| result.entry.name.as_str()),
            Some("Notepad")
        );
        assert!(!new.iter().any(|result| result.entry.name == "Calculator"));
    }

    #[test]
    fn application_and_file_results_share_the_full_query_language() {
        let (_directory, catalog) = catalog_with_files(&[
            "Visual Studio Code.lnk",
            "Visual Studio Preview.lnk",
            "Notepad.lnk",
        ]);

        let results = catalog
            .search("^Visual !Preview | ^Notepad", 10)
            .expect("shared application query");
        let names = results
            .iter()
            .map(|result| result.entry.name.as_str())
            .collect::<Vec<_>>();

        assert!(names.contains(&"Visual Studio Code"));
        assert!(names.contains(&"Notepad"));
        assert!(!names.contains(&"Visual Studio Preview"));
    }

    #[test]
    fn r8_catalog_search_preserves_legacy_scores_rank_and_history_after_mutation() {
        let (directory, mut catalog) = catalog_with_files(&[
            "Visual Studio Code.lnk",
            "VSCode.lnk",
            "计算器.lnk",
            "QQ音乐.lnk",
            "Helium.lnk",
            "hume.lnk",
            "Kelvin.lnk",
            "İstanbul.lnk",
        ]);
        let learned = catalog
            .entries()
            .iter()
            .find(|entry| entry.name == "VSCode")
            .expect("fixture app")
            .id
            .clone();
        catalog
            .record_activation(&learned, "vsc", 123)
            .expect("record query history");
        for refresh in [false, true] {
            if refresh {
                fs::write(directory.path().join("New Calculator.lnk"), b"fixture")
                    .expect("new app");
                catalog.replace_snapshot(scan_application_roots(catalog.roots()));
            }
            for raw in [
                "vsc",
                "VSC",
                "jsq",
                "yy",
                "计",
                "hum",
                "k",
                "istan",
                "'code",
                "^visual",
                "lnk$",
                "code | jsq",
                "code !preview",
                "::Studio code",
            ] {
                let query = FileQuery::parse(raw);
                let mut expected = catalog
                    .entries()
                    .iter()
                    .filter_map(|entry| {
                        // Frozen pre-R8 application matching at the real catalog caller.
                        let direct =
                            query
                                .score_literal(&entry.name, &entry.target)
                                .into_iter()
                                .chain(query.score_literal(
                                    &application_initialism(&entry.name),
                                    &entry.target,
                                ))
                                .max();
                        let score = direct.filter(|score| *score >= 2_000).or_else(|| {
                            query
                                .score_literal(&pinyin::pinyin_initials(&entry.name), &entry.target)
                        })?;
                        let usage = catalog.usage_for(&entry.id, &normalize(raw));
                        Some(SearchResult {
                            entry: entry.clone(),
                            action: SearchResultAction::OpenTarget,
                            subtitle: None,
                            score,
                            activation_count: usage.activation_count,
                            last_activated_at_unix_ms: usage.last_activated_at_unix_ms,
                            query_activation_count: usage.query_activation_count,
                            last_query_activation_at_unix_ms: usage
                                .last_query_activation_at_unix_ms,
                        })
                    })
                    .collect::<Vec<_>>();
                expected.sort_by(|left, right| {
                    unified_search_rank(right)
                        .cmp(&unified_search_rank(left))
                        .then_with(|| left.entry.name.cmp(&right.entry.name))
                        .then_with(|| left.entry.target.cmp(&right.entry.target))
                });
                let actual = catalog.search(raw, 200).expect("actual catalog search");
                let retained = actual
                    .iter()
                    .filter(|item| expected.iter().any(|old| old.entry.id == item.entry.id))
                    .cloned()
                    .collect::<Vec<_>>();
                assert_eq!(
                    retained, expected,
                    "legacy scores/order changed: {raw}, refresh={refresh}"
                );
                for added in actual
                    .iter()
                    .filter(|item| !expected.iter().any(|old| old.entry.id == item.entry.id))
                {
                    assert!(
                        added.score < 2_000
                            && query
                                .score(&added.entry.name, &added.entry.target)
                                .is_some(),
                        "not a new low-tier pinyin match"
                    );
                }
                for limit in [1, 3, 200] {
                    assert_eq!(
                        catalog.search(raw, limit).expect("limited search"),
                        actual.iter().take(limit).cloned().collect::<Vec<_>>(),
                        "{raw}, refresh={refresh}"
                    );
                }
            }
        }
    }

    #[test]
    fn w1_application_full_spelling_preserves_legacy_initial_scores_and_refresh() {
        let (directory, mut catalog) =
            catalog_with_files(&["计算器.lnk", "资源管理器.lnk", "重庆.lnk"]);
        let legacy = catalog.search("jsq", 20).expect("legacy initials");
        assert_eq!(legacy.len(), 1);
        assert_eq!(
            legacy[0].score,
            FileQuery::parse("jsq")
                .score_literal("jsq", &legacy[0].entry.target)
                .expect("frozen score")
        );
        for (query, name) in [
            ("jisuanqi", "计算器"),
            ("jisuq", "计算器"),
            ("chongqing", "重庆"),
        ] {
            assert!(
                catalog
                    .search(query, 20)
                    .expect("full spelling")
                    .iter()
                    .any(|item| item.entry.name == name)
            );
        }
        fs::write(directory.path().join("银行卡.lnk"), b"fixture").expect("new shortcut");
        catalog.replace_snapshot(scan_application_roots(catalog.roots()));
        assert!(
            catalog
                .search("yinhang", 20)
                .expect("refreshed pinyin")
                .iter()
                .any(|item| item.entry.name == "银行卡")
        );
    }

    #[test]
    fn a_background_rescan_snapshot_can_replace_catalog_entries_without_reopening_history() {
        let (directory, mut catalog) = catalog_with_files(&["Alpha.lnk"]);
        fs::write(directory.path().join("Beta.lnk"), b"fixture").expect("beta shortcut");

        let snapshot = scan_application_roots(catalog.roots());
        catalog.replace_snapshot(snapshot);

        let results = catalog.search("beta", 10).expect("refreshed search");
        assert_eq!(
            results.first().map(|item| item.entry.name.as_str()),
            Some("Beta")
        );
    }

    #[test]
    fn failed_refresh_keeps_the_last_good_catalog() {
        let (_directory, mut catalog) = catalog_with_files(&["Alpha.lnk"]);
        let failed = ApplicationCatalogSnapshot {
            entries: vec![ApplicationEntry {
                id: "gamma".into(),
                name: "Gamma".into(),
                target: "Gamma.lnk".into(),
                source: ApplicationSource::StartMenu,
            }],
            status: CatalogStatus {
                indexed_items: 1,
                scanned_roots: 1,
                last_scan_at_unix_ms: 10,
                errors: vec!["start menu unavailable".into()],
            },
        };

        assert!(catalog.apply_refresh_snapshot(failed).is_err());
        assert_eq!(
            catalog
                .search("alpha", 10)
                .expect("preserved search")
                .first()
                .map(|item| item.entry.name.as_str()),
            Some("Alpha")
        );
        assert_eq!(catalog.status().errors, ["start menu unavailable"]);
    }
}
