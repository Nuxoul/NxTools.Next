use std::collections::HashMap;

use super::{
    ApplicationSource, FinderCatalog, SearchResult, SearchResultAction, UsageRecord, normalize,
    normalize_target,
};

/// Provider labels do not divide the usage history of one Windows path.
pub(super) fn canonical_history_id(id: &str) -> String {
    id.strip_prefix("app:")
        .or_else(|| id.strip_prefix("indexed:"))
        .map(|path| format!("app:{}", normalize_target(path)))
        .unwrap_or_else(|| id.to_owned())
}

pub(super) fn merge_usage_record(target: &mut UsageRecord, other: UsageRecord) {
    target.0 = target.0.saturating_add(other.0);
    target.1 = target.1.max(other.1);
}

const GENERAL_RESULTS: i8 = 2;
const GENERAL_HEADROOM: usize = 10;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct UnifiedSearchRank {
    group: i8,
    keyword_history: u8,
    history_recency: Option<i64>,
    match_quality: i64,
    source_priority: u8,
    last_activated: Option<i64>,
    activation_count: u64,
    compact_path: i64,
}

pub(super) fn unified_search_rank(result: &SearchResult) -> UnifiedSearchRank {
    rank_with_prefix(result, None)
}

fn rank_with_prefix(result: &SearchResult, prefix_recency: Option<i64>) -> UnifiedSearchRank {
    let good_match = result.score >= 2_000;
    let keyword_history = if result.query_activation_count > 0 {
        2
    } else if prefix_recency.is_some() {
        1
    } else {
        0
    };
    let (group, history_recency) = if matches!(
        result.entry.source,
        ApplicationSource::Calculator
            | ApplicationSource::EnvironmentVariable
            | ApplicationSource::CustomCommand
    ) {
        (
            7,
            result.last_query_activation_at_unix_ms.or(prefix_recency),
        )
    } else if result.query_activation_count > 0 {
        (6, result.last_query_activation_at_unix_ms)
    } else if prefix_recency.is_some() {
        (5, prefix_recency)
    } else if result.activation_count > 0 && good_match {
        (4, None)
    } else if is_application(result.entry.source) && good_match {
        (3, None)
    } else if result.activation_count > 0 {
        (1, result.last_activated_at_unix_ms)
    } else if is_application(result.entry.source) {
        (0, None)
    } else {
        (GENERAL_RESULTS, None)
    };
    UnifiedSearchRank {
        group,
        keyword_history,
        history_recency,
        match_quality: result.score
            - if is_generated_dependency_path(&result.entry.target) {
                2_500
            } else {
                0
            },
        source_priority: match result.entry.source {
            ApplicationSource::StartMenu | ApplicationSource::System => 4,
            ApplicationSource::Desktop => 3,
            ApplicationSource::IndexedDirectory => 2,
            _ => 1,
        },
        last_activated: result.last_activated_at_unix_ms,
        activation_count: result.activation_count,
        compact_path: -(result.entry.target.matches(['\\', '/']).count() as i64),
    }
}

fn is_application(source: ApplicationSource) -> bool {
    matches!(
        source,
        ApplicationSource::StartMenu | ApplicationSource::Desktop | ApplicationSource::System
    )
}

fn shortcut_key(result: &SearchResult) -> Option<String> {
    if !is_application(result.entry.source) || result.action != SearchResultAction::OpenTarget {
        return None;
    }
    let name = result
        .entry
        .target
        .rsplit(['\\', '/'])
        .next()?
        .to_lowercase();
    let (_, extension) = name.rsplit_once('.')?;
    matches!(extension, "lnk" | "url" | "appref-ms").then_some(name)
}

struct Candidate {
    result: SearchResult,
    prefix_recency: Option<i64>,
}

impl Candidate {
    fn merge(&mut self, other: Self) {
        let same_identity = canonical_history_id(&self.result.entry.id)
            == canonical_history_id(&other.result.entry.id);
        let count = |left: u64, right: u64| {
            if same_identity {
                left.max(right)
            } else {
                left.saturating_add(right)
            }
        };
        let activations = count(self.result.activation_count, other.result.activation_count);
        let query_activations = count(
            self.result.query_activation_count,
            other.result.query_activation_count,
        );
        let last = self
            .result
            .last_activated_at_unix_ms
            .max(other.result.last_activated_at_unix_ms);
        let last_query = self
            .result
            .last_query_activation_at_unix_ms
            .max(other.result.last_query_activation_at_unix_ms);
        let score = self.result.score.max(other.result.score);
        self.prefix_recency = self.prefix_recency.max(other.prefix_recency);
        // Prefer the application display name over a raw indexed filename, then
        // choose a stable path regardless of provider arrival order.
        let prefer_other = (
            is_application(other.result.entry.source),
            std::cmp::Reverse(normalize_target(&other.result.entry.target)),
        ) > (
            is_application(self.result.entry.source),
            std::cmp::Reverse(normalize_target(&self.result.entry.target)),
        );
        if prefer_other {
            self.result = other.result;
        }
        self.result.score = score;
        self.result.activation_count = activations;
        self.result.query_activation_count = query_activations;
        self.result.last_activated_at_unix_ms = last;
        self.result.last_query_activation_at_unix_ms = last_query;
    }
}

impl FinderCatalog {
    pub(super) fn rank_results(
        &self,
        results: Vec<SearchResult>,
        query: &str,
        limit: usize,
    ) -> Vec<SearchResult> {
        let query = normalize(query);
        // Visit history once per query, never once per candidate/comparison.
        let mut prefixes = HashMap::<&str, i64>::new();
        for ((id, keyword), (_, last)) in &self.query_usage {
            if keyword.len() > query.len()
                && keyword.starts_with(&query)
                && let Some(last) = last
            {
                prefixes
                    .entry(id.as_str())
                    .and_modify(|value| *value = (*value).max(*last))
                    .or_insert(*last);
            }
        }
        let mut paths = HashMap::<String, usize>::new();
        let mut candidates: Vec<Candidate> = Vec::with_capacity(results.len());
        for result in results {
            let prefix_recency = prefixes
                .get(canonical_history_id(&result.entry.id).as_str())
                .copied();
            let path_item = result.action == SearchResultAction::OpenTarget
                && !matches!(
                    result.entry.source,
                    ApplicationSource::CustomCommand | ApplicationSource::EnvironmentVariable
                );
            let candidate = Candidate {
                result,
                prefix_recency,
            };
            if path_item {
                let path = normalize_target(&candidate.result.entry.target);
                if let Some(&index) = paths.get(&path) {
                    candidates[index].merge(candidate);
                    continue;
                }
                paths.insert(path, candidates.len());
            }
            candidates.push(candidate);
        }
        let mut shortcuts = HashMap::<String, usize>::new();
        let mut merged: Vec<Candidate> = Vec::with_capacity(candidates.len());
        for candidate in candidates {
            if let Some(key) = shortcut_key(&candidate.result) {
                if let Some(&index) = shortcuts.get(&key) {
                    merged[index].merge(candidate);
                    continue;
                }
                shortcuts.insert(key, merged.len());
            }
            merged.push(candidate);
        }
        let mut ranked: Vec<_> = merged
            .into_iter()
            .map(|candidate| {
                let rank = if candidate.prefix_recency.is_some() {
                    rank_with_prefix(&candidate.result, candidate.prefix_recency)
                } else {
                    unified_search_rank(&candidate.result)
                };
                (rank, candidate.result)
            })
            .collect();
        let compare = |left: &(UnifiedSearchRank, SearchResult),
                       right: &(UnifiedSearchRank, SearchResult)| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.entry.name.cmp(&right.1.entry.name))
                .then_with(|| left.1.entry.target.cmp(&right.1.entry.target))
        };
        ranked.sort_by(compare);
        let mut general_count = 0;
        for (rank, _) in &mut ranked {
            if rank.group == GENERAL_RESULTS {
                general_count += 1;
                if general_count > GENERAL_HEADROOM {
                    rank.group = -1;
                }
            }
        }
        ranked.sort_by(compare);
        ranked
            .into_iter()
            .take(limit)
            .map(|(_, result)| result)
            .collect()
    }
}

fn is_generated_dependency_path(target: &str) -> bool {
    let normalized = normalize_target(target);
    if normalized.matches('\\').count() < 4 {
        return false;
    }
    [
        "\\.cargo\\",
        "\\.cmd.root\\",
        "\\registry\\src\\",
        "\\node_modules\\",
        "\\appdata\\local\\packages\\",
        "\\lib\\bundles\\",
        "\\plugins\\textmate-plugin\\",
        "\\vendor\\",
        "\\target\\debug\\",
        "\\target\\release\\",
    ]
    .iter()
    .any(|segment| normalized.contains(segment))
}
