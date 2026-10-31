use std::cmp::Reverse;
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};

use nxtools_core::{DownloadCategorySettings, DownloadDuplicateStrategySettings, DownloadSettings};
use thiserror::Error;

use crate::{DownloadDuplicateStrategy, DownloadRequest};

const MAX_CATEGORY_COUNT: usize = 32;
const MAX_RULES_PER_CATEGORY: usize = 64;
const MAX_CATEGORY_TEXT_BYTES: usize = 8 * 1024;
const MAX_NUMBERED_CANDIDATES: usize = 10_000;

#[derive(Debug, Clone)]
pub struct DownloadOrganizationPolicy {
    current: Arc<RwLock<DownloadOrganizationSnapshot>>,
}

#[derive(Debug, Clone)]
struct DownloadOrganizationSnapshot {
    default_directory: PathBuf,
    automatic_categories_enabled: bool,
    categories: Vec<DownloadCategorySettings>,
    duplicate_strategy: DownloadDuplicateStrategy,
}

#[derive(Debug, Error)]
pub enum DownloadOrganizationPolicyError {
    #[error("download organization policy lock is poisoned")]
    LockPoisoned,
    #[error("download categories exceed the bounded category or rule limit")]
    TooManyRules,
    #[error("download category contains invalid or duplicate identity")]
    InvalidCategoryIdentity,
    #[error("download category contains an invalid save directory")]
    InvalidCategoryDirectory,
    #[error("the selected download category is no longer available")]
    CategoryUnavailable,
    #[error("download category text exceeds the bounded size limit")]
    CategoryTextTooLarge,
    #[error("the download target is already occupied")]
    TargetOccupied,
    #[error("no bounded numbered target is available")]
    NumberedTargetExhausted,
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl DownloadOrganizationPolicy {
    pub fn from_download_settings(
        settings: &DownloadSettings,
    ) -> Result<Self, DownloadOrganizationPolicyError> {
        Ok(Self {
            current: Arc::new(RwLock::new(parse_snapshot(settings)?)),
        })
    }

    pub fn update_from_download_settings(
        &self,
        settings: &DownloadSettings,
    ) -> Result<(), DownloadOrganizationPolicyError> {
        let next = parse_snapshot(settings)?;
        *self
            .current
            .write()
            .map_err(|_| DownloadOrganizationPolicyError::LockPoisoned)? = next;
        Ok(())
    }

    pub(crate) fn apply_to_request(
        &self,
        request: &mut DownloadRequest,
        mut target_is_reserved: impl FnMut(&Path) -> bool,
    ) -> Result<(), DownloadOrganizationPolicyError> {
        let snapshot = self
            .current
            .read()
            .map_err(|_| DownloadOrganizationPolicyError::LockPoisoned)?
            .clone();
        let initial_directory = request.save_directory.clone();
        request.duplicate_strategy = match request.duplicate_strategy {
            DownloadDuplicateStrategy::UseDefault => snapshot.duplicate_strategy,
            strategy => strategy,
        };
        request.category_name = None;
        let selected = if let Some(category_id) = request.category_id.as_deref() {
            Some(
                snapshot
                    .categories
                    .iter()
                    .find(|category| {
                        category.enabled && category.id.eq_ignore_ascii_case(category_id)
                    })
                    .ok_or(DownloadOrganizationPolicyError::CategoryUnavailable)?,
            )
        } else if snapshot.automatic_categories_enabled
            && same_windows_path(&initial_directory, &snapshot.default_directory)
        {
            select_category(&snapshot.categories, request)
        } else {
            None
        };
        if let Some(category) = selected {
            request.category_id = Some(category.id.clone());
            request.category_name = Some(category.name.clone());
            if !category.save_directory.is_empty()
                && !request.save_directory_explicit
                && same_windows_path(&initial_directory, &snapshot.default_directory)
            {
                let configured = PathBuf::from(&category.save_directory);
                request.save_directory = if configured.is_absolute() {
                    configured
                } else {
                    resolve_relative_category_directory(&initial_directory, &configured)?
                };
            }
        } else {
            request.category_id = None;
        }
        std::fs::create_dir_all(&request.save_directory)?;

        let file_name = request.file_name.clone().unwrap_or_default();
        let initial_target = request.save_directory.join(&file_name);
        if !initial_target.exists() && !target_is_reserved(&initial_target) {
            return Ok(());
        }
        match request.duplicate_strategy {
            DownloadDuplicateStrategy::Reject => {
                Err(DownloadOrganizationPolicyError::TargetOccupied)
            }
            DownloadDuplicateStrategy::AddNumbered | DownloadDuplicateStrategy::UseDefault => {
                let stem = Path::new(&file_name)
                    .file_stem()
                    .and_then(|value| value.to_str())
                    .unwrap_or("download");
                let extension = Path::new(&file_name)
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|value| format!(".{value}"))
                    .unwrap_or_default();
                for index in 1..=MAX_NUMBERED_CANDIDATES {
                    let candidate = format!("{stem} ({index}){extension}");
                    let target = request.save_directory.join(&candidate);
                    if !target.exists() && !target_is_reserved(&target) {
                        request.file_name = Some(candidate);
                        return Ok(());
                    }
                }
                Err(DownloadOrganizationPolicyError::NumberedTargetExhausted)
            }
        }
    }
}

fn parse_snapshot(
    settings: &DownloadSettings,
) -> Result<DownloadOrganizationSnapshot, DownloadOrganizationPolicyError> {
    if settings.categories.len() > MAX_CATEGORY_COUNT {
        return Err(DownloadOrganizationPolicyError::TooManyRules);
    }
    let mut identities = HashSet::new();
    let mut text_bytes = 0_usize;
    for category in &settings.categories {
        if category.id.is_empty()
            || category.id.len() > 64
            || !category
                .id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            || category.name.is_empty()
            || category.name.len() > 64
            || !identities.insert(category.id.to_ascii_lowercase())
        {
            return Err(DownloadOrganizationPolicyError::InvalidCategoryIdentity);
        }
        if category.extensions.len() > MAX_RULES_PER_CATEGORY
            || category.url_patterns.len() > MAX_RULES_PER_CATEGORY
        {
            return Err(DownloadOrganizationPolicyError::TooManyRules);
        }
        if !category.save_directory.is_empty()
            && !Path::new(&category.save_directory).is_absolute()
            && Path::new(&category.save_directory)
                .components()
                .any(|component| {
                    matches!(
                        component,
                        Component::ParentDir | Component::RootDir | Component::Prefix(_)
                    )
                })
        {
            return Err(DownloadOrganizationPolicyError::InvalidCategoryDirectory);
        }
        text_bytes = text_bytes
            .saturating_add(category.id.len())
            .saturating_add(category.name.len())
            .saturating_add(category.save_directory.len())
            .saturating_add(category.extensions.iter().map(String::len).sum::<usize>())
            .saturating_add(category.url_patterns.iter().map(String::len).sum::<usize>());
    }
    if text_bytes > MAX_CATEGORY_TEXT_BYTES {
        return Err(DownloadOrganizationPolicyError::CategoryTextTooLarge);
    }
    Ok(DownloadOrganizationSnapshot {
        default_directory: PathBuf::from(&settings.default_directory),
        automatic_categories_enabled: settings.automatic_categories_enabled,
        categories: settings.categories.clone(),
        duplicate_strategy: match settings.duplicate_strategy {
            DownloadDuplicateStrategySettings::AddNumbered => {
                DownloadDuplicateStrategy::AddNumbered
            }
            DownloadDuplicateStrategySettings::Reject => DownloadDuplicateStrategy::Reject,
        },
    })
}

fn select_category<'a>(
    categories: &'a [DownloadCategorySettings],
    request: &DownloadRequest,
) -> Option<&'a DownloadCategorySettings> {
    let extension = request
        .file_name
        .as_deref()
        .and_then(|name| Path::new(name).extension())
        .and_then(|value| value.to_str());
    categories
        .iter()
        .enumerate()
        .filter(|(_, category)| category.enabled && category_has_rules(category))
        .filter(|(_, category)| {
            (category.extensions.is_empty()
                || extension.is_some_and(|extension| {
                    category
                        .extensions
                        .iter()
                        .any(|candidate| candidate.eq_ignore_ascii_case(extension))
                }))
                && (category.url_patterns.is_empty()
                    || category
                        .url_patterns
                        .iter()
                        .any(|pattern| wildcard_match(&request.source_url, pattern)))
        })
        .min_by_key(|(index, category)| {
            let url_specificity = category
                .url_patterns
                .iter()
                .filter(|pattern| wildcard_match(&request.source_url, pattern))
                .map(|pattern| {
                    pattern
                        .bytes()
                        .filter(|byte| !matches!(byte, b'*' | b'?'))
                        .count()
                })
                .max()
                .unwrap_or_default();
            (
                Reverse(!category.url_patterns.is_empty()),
                Reverse(url_specificity),
                Reverse(!category.extensions.is_empty()),
                *index,
            )
        })
        .map(|(_, category)| category)
}

fn category_has_rules(category: &DownloadCategorySettings) -> bool {
    !category.extensions.is_empty() || !category.url_patterns.is_empty()
}

fn wildcard_match(value: &str, pattern: &str) -> bool {
    let value = value.as_bytes();
    let pattern = pattern.as_bytes();
    let (mut value_index, mut pattern_index, mut star, mut matched) = (0, 0, None, 0);
    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?'
                || pattern[pattern_index].eq_ignore_ascii_case(&value[value_index]))
        {
            value_index += 1;
            pattern_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star = Some(pattern_index);
            pattern_index += 1;
            matched = value_index;
        } else if let Some(star_index) = star {
            pattern_index = star_index + 1;
            matched += 1;
            value_index = matched;
        } else {
            return false;
        }
    }
    pattern[pattern_index..].iter().all(|byte| *byte == b'*')
}

fn same_windows_path(left: &Path, right: &Path) -> bool {
    windows_path_key(left) == windows_path_key(right)
}

pub(crate) fn windows_path_key(path: &Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_owned())
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .replace('/', "\\")
        .to_lowercase()
}

fn resolve_relative_category_directory(
    root: &Path,
    relative: &Path,
) -> Result<PathBuf, DownloadOrganizationPolicyError> {
    std::fs::create_dir_all(root)?;
    let canonical_root = std::fs::canonicalize(root)?;
    let root_key = windows_path_key(&canonical_root);
    let mut current = canonical_root.clone();
    for component in relative.components() {
        match component {
            Component::CurDir => continue,
            Component::Normal(component) => current.push(component),
            _ => return Err(DownloadOrganizationPolicyError::InvalidCategoryDirectory),
        }
        match std::fs::create_dir(&current) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        let canonical = std::fs::canonicalize(&current)?;
        let candidate_key = windows_path_key(&canonical);
        let within = candidate_key == root_key
            || candidate_key
                .strip_prefix(&root_key)
                .is_some_and(|suffix| suffix.starts_with('\\'));
        if !within {
            return Err(DownloadOrganizationPolicyError::InvalidCategoryDirectory);
        }
        current = canonical;
    }
    Ok(root.join(relative))
}
