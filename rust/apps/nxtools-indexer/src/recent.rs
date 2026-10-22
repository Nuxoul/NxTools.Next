//! Cached recent-modified view; never opens a file or materializes the complete mapped base.
use std::collections::HashSet;

use nxtools_finder::FileQuery;
use nxtools_protocol::{IndexSearchItem, IndexSearchRequest, IndexSettings};

use crate::mft::{MftEntryRef, MftSnapshot};

const PER_FOLDER: usize = 2;
const PER_PROJECT: usize = 3;
const MAX_DEPTH: usize = 128;
const MAX_PATH_BYTES: usize = 32_768;
const MARKER_CACHE_BYTES: usize = 1024 * 1024;
const MARKER_CACHE_ROOTS: usize = 4096;
const MAX_OVERFLOW_CLASSIFIER_WORK: usize = 1_000_000;
fn visible_entries<'a>(
    snapshot: &'a MftSnapshot,
    is_current: &'a impl Fn() -> bool,
) -> impl Iterator<Item = MftEntryRef<'a>> {
    (0..snapshot.base_len())
        .take_while(move |row| !row.is_multiple_of(128) || is_current())
        .filter_map(move |row| snapshot.base_entry(row))
        .filter(move |entry| {
            entry.identity.is_none_or(|id| {
                !snapshot.overlay.deleted.contains(&id.file_reference)
                    && !snapshot
                        .overlay
                        .replacements
                        .contains_key(&id.file_reference)
            })
        })
        .chain(
            snapshot
                .overlay
                .replacements
                .iter()
                .enumerate()
                .take_while(move |(row, _)| !row.is_multiple_of(128) || is_current())
                .filter(move |(_, (id, _))| !snapshot.overlay.deleted.contains(id))
                .map(|(_, (_, entry))| entry.as_ref()),
        )
}

fn marker(name: &str) -> bool {
    [
        "CACHEDIR.TAG",
        "CMakeCache.txt",
        "build.ninja",
        "pyvenv.cfg",
    ]
    .iter()
    .any(|candidate| name.eq_ignore_ascii_case(candidate))
}

fn noise(name: &str) -> bool {
    if name.is_empty() || name.starts_with("~$") || name.ends_with('~') {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    if [".ninja_log", ".ninja_deps", ".ds_store"].contains(&lower.as_str()) {
        return true;
    }
    let ext = lower.rsplit_once('.').map(|(_, ext)| ext).unwrap_or("");
    [
        "tmp",
        "temp",
        "lock",
        "lck",
        "swp",
        "swo",
        "crdownload",
        "part",
        "partial",
        "download",
        "opdownload",
        "!ut",
        "obj",
        "o",
        "pdb",
        "ilk",
        "idb",
        "iobj",
        "ipdb",
        "pch",
        "ipch",
        "tlog",
        "lastbuildstate",
        "pyc",
        "class",
    ]
    .contains(&ext)
        || ["-wal", "-shm", "-journal"]
            .iter()
            .any(|suffix| lower.ends_with(suffix))
}

fn normalized(path: &str) -> Option<String> {
    if path.len() > MAX_PATH_BYTES {
        return None;
    }
    let path = path.replace('/', "\\").to_ascii_lowercase();
    let components = path.split('\\').filter(|part| !part.is_empty()).count();
    (components <= MAX_DEPTH).then_some(path)
}

fn folder(path: &str) -> &str {
    path.rsplit_once('\\')
        .map(|(parent, _)| parent)
        .unwrap_or("")
}

fn inside_output(parent: &str, roots: &HashSet<String>) -> bool {
    let mut ancestor = parent;
    for _ in 0..MAX_DEPTH {
        if roots.contains(ancestor) {
            return true;
        }
        let Some((next, _)) = ancestor.rsplit_once('\\') else {
            break;
        };
        ancestor = next;
    }
    false
}
fn project(parent: &str) -> String {
    let parts = parent
        .split('\\')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    // UNC server/share is the volume, not a project. Users/name/known-folder is a container.
    let root = if parent.starts_with("\\\\") { 2 } else { 1 };
    let profile = parts.get(root).is_some_and(|part| *part == "users");
    let known = parts.get(root + 2).is_some_and(|part| {
        [
            "desktop",
            "documents",
            "downloads",
            "pictures",
            "music",
            "videos",
            "onedrive",
        ]
        .contains(part)
    });
    let take = if profile && known {
        root + 4
    } else if profile {
        root + 3
    } else {
        root + 1
    };
    let prefix = if parent.starts_with("\\\\") {
        "\\\\"
    } else {
        ""
    };
    format!("{prefix}{}", parts[..take.min(parts.len())].join("\\"))
}

struct Candidate<'a> {
    entry: MftEntryRef<'a>,
    key: String,
    folder: String,
    project: String,
    time: i64,
}

fn better(left: &Candidate<'_>, right: &Candidate<'_>) -> bool {
    left.time > right.time
        || (left.time == right.time
            && (left.key.as_str(), left.entry.path) < (right.key.as_str(), right.entry.path))
}

// Folder/project/global constraints are nested. Exchange the worst member in the smallest
// saturated set (the candidate's circuit), not a fixed global pre-diversification top window.
// This weighted streaming selection retains at most the requested limit, even for a bulk export.
fn retain<'a>(kept: &mut Vec<Candidate<'a>>, candidate: Candidate<'a>, limit: usize) {
    if let Some(index) = kept.iter().position(|old| old.key == candidate.key) {
        if better(&candidate, &kept[index]) {
            kept[index] = candidate;
        }
        return;
    }
    let folder_full = kept
        .iter()
        .filter(|old| old.folder == candidate.folder)
        .count()
        >= PER_FOLDER;
    let project_full = kept
        .iter()
        .filter(|old| old.project == candidate.project)
        .count()
        >= PER_PROJECT;
    let worst = kept
        .iter()
        .enumerate()
        .filter(|(_, old)| {
            if folder_full {
                old.folder == candidate.folder
            } else if project_full {
                old.project == candidate.project
            } else {
                kept.len() >= limit
            }
        })
        .max_by(|(_, left), (_, right)| {
            if better(left, right) {
                std::cmp::Ordering::Less
            } else if better(right, left) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .map(|(index, _)| index);
    if let Some(worst) = worst {
        if !better(&candidate, &kept[worst]) {
            return;
        }
        kept.swap_remove(worst);
    }
    kept.push(candidate);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClassifierBudgetExceeded;

pub(crate) fn search_recent_modified(
    snapshots: &[MftSnapshot],
    settings: &IndexSettings,
    request: &IndexSearchRequest,
    now_unix_ms: i64,
    is_current: &(impl Fn() -> bool + Sync),
) -> Result<Option<Vec<IndexSearchItem>>, ClassifierBudgetExceeded> {
    if !is_current() {
        return Ok(None);
    }
    let limit = request.limit.min(crate::MAX_SEARCH_RESULTS);
    if limit == 0 || !request.context.include_files {
        return Ok(Some(Vec::new()));
    }
    let query = FileQuery::parse(&request.query);
    let ordinary = !request.query.trim().is_empty();
    let mut output_roots = HashSet::new();
    let mut marker_bytes = 0;
    let mut marker_overflow = false;
    let mut work = 0_usize;
    let mut overflow_work = 0_usize;
    for snapshot in snapshots {
        for entry in visible_entries(snapshot, is_current) {
            work += 1;
            if work.is_multiple_of(128) && !is_current() {
                return Ok(None);
            }
            if !entry.is_directory
                && marker(entry.name)
                && let Some(path) = normalized(entry.path)
            {
                let root = folder(&path);
                if output_roots.contains(root) {
                    continue;
                }
                if output_roots.len() < MARKER_CACHE_ROOTS
                    && marker_bytes + root.len() <= MARKER_CACHE_BYTES
                {
                    marker_bytes += root.len();
                    output_roots.insert(root.to_owned());
                } else {
                    marker_overflow = true;
                }
            }
        }
    }
    let mut kept = Vec::with_capacity(limit);
    for snapshot in snapshots {
        for entry in visible_entries(snapshot, is_current) {
            work += 1;
            if work.is_multiple_of(128) && !is_current() {
                return Ok(None);
            }
            let Some(time) = entry
                .modified_at_unix_ms
                .filter(|time| *time > 0 && *time <= now_unix_ms)
            else {
                continue;
            };
            if entry.is_directory || noise(entry.name) {
                continue;
            }
            if !crate::index_settings_include_path(
                settings,
                entry.path,
                query.bypasses_exclusions(),
            ) || request
                .context
                .directory
                .as_deref()
                .filter(|dir| !dir.trim().is_empty())
                .is_some_and(|dir| !crate::path_is_within(entry.path, dir))
                || (ordinary && query.score(entry.name, entry.path).is_none())
            {
                continue;
            }
            let Some(key) = normalized(entry.path) else {
                continue;
            };
            let parent = folder(&key);
            if parent
                .split('\\')
                .any(|part| ["cmakefiles", "obj"].contains(&part))
                || inside_output(parent, &output_roots)
            {
                continue;
            }
            // Cache overflow is a performance fallback, never a silent classifier miss. Iterate
            // only cached rows, with cancellation, and allocate no whole-index marker collection.
            let mut output = false;
            if marker_overflow {
                'markers: for source in snapshots {
                    for marker_entry in visible_entries(source, is_current) {
                        work += 1;
                        overflow_work += 1;
                        if overflow_work > MAX_OVERFLOW_CLASSIFIER_WORK {
                            tracing::warn!(
                                event = "indexer.recent.classifier_budget_exhausted",
                                generation = request.generation,
                                work_limit = MAX_OVERFLOW_CLASSIFIER_WORK,
                                "recent view refused rather than perform unbounded cached rescans"
                            );
                            return Err(ClassifierBudgetExceeded);
                        }
                        if work.is_multiple_of(128) && !is_current() {
                            return Ok(None);
                        }
                        if !marker_entry.is_directory
                            && marker(marker_entry.name)
                            && let Some(marker_path) = normalized(marker_entry.path)
                            && crate::path_is_within(parent, folder(&marker_path))
                        {
                            output = true;
                            break 'markers;
                        }
                    }
                }
            }
            if output {
                continue;
            }
            let project = project(parent);
            let parent = parent.to_owned();
            retain(
                &mut kept,
                Candidate {
                    entry,
                    key,
                    folder: parent,
                    project,
                    time,
                },
                limit,
            );
        }
    }
    if !is_current() {
        return Ok(None);
    }
    kept.sort_by(|left, right| {
        right
            .time
            .cmp(&left.time)
            .then_with(|| left.key.cmp(&right.key))
            .then_with(|| left.entry.path.cmp(right.entry.path))
    });
    Ok(Some(
        kept.into_iter()
            .map(|candidate| IndexSearchItem {
                path: candidate.entry.path.to_owned(),
                name: candidate.entry.name.to_owned(),
                is_directory: false,
                size_bytes: candidate.entry.size_bytes,
                modified_at_unix_ms: Some(candidate.time),
                score: candidate.time,
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mft::{MftEntry, MftIdentity};
    use nxtools_protocol::IndexSearchContext;

    fn entry(id: u64, path: &str, time: Option<i64>) -> MftEntry {
        MftEntry {
            identity: Some(MftIdentity {
                file_reference: id,
                parent_reference: 5,
            }),
            path: path.into(),
            name: path
                .rsplit(['\\', '/'])
                .next()
                .expect("fixture name")
                .into(),
            is_directory: false,
            size_bytes: Some(10),
            modified_at_unix_ms: time,
        }
    }
    fn request() -> IndexSearchRequest {
        IndexSearchRequest {
            generation: 7,
            query: String::new(),
            limit: 20,
            context: IndexSearchContext {
                include_files: true,
                include_directories: true,
                ..Default::default()
            },
        }
    }
    fn search(snapshot: &MftSnapshot, request: &IndexSearchRequest) -> Vec<IndexSearchItem> {
        search_recent_modified(
            std::slice::from_ref(snapshot),
            &IndexSettings::default(),
            request,
            10_000,
            &|| true,
        )
        .expect("within classifier budget")
        .expect("current fixture")
    }
    #[test]
    fn w4_recent_valid_times_noise_and_ties() {
        let paths = [
            ("C:\\A\\z.txt", Some(40)),
            ("C:\\A\\a.txt", Some(40)),
            ("C:\\B\\future.txt", Some(10_001)),
            ("C:\\C\\unknown.txt", None),
            ("C:\\D\\zero.txt", Some(0)),
            ("C:\\E\\negative.txt", Some(-1)),
            ("C:\\F\\a.tmp", Some(90)),
            ("C:\\G\\a.partial", Some(90)),
            ("C:\\H\\~$report.docx", Some(90)),
            ("C:\\I\\a.obj", Some(90)),
            ("C:\\J\\real.db", Some(30)),
            ("C:\\K\\real.dat", Some(20)),
            ("C:\\L\\.gitignore", Some(10)),
        ];
        let snapshot = MftSnapshot::from_entries(
            "C:\\",
            Some(1),
            Some(100),
            paths
                .iter()
                .enumerate()
                .map(|(id, (path, time))| entry(id as u64 + 10, path, *time))
                .collect(),
        );
        let results = search(&snapshot, &request());
        assert_eq!(
            results
                .iter()
                .map(|hit| hit.name.as_str())
                .collect::<Vec<_>>(),
            ["a.txt", "z.txt", "real.db", "real.dat", ".gitignore"]
        );
    }
    #[test]
    fn w4_recent_bulk_project_cannot_starve_older_useful_work() {
        let mut rows = (0..10_000)
            .map(|i| {
                entry(
                    i + 10,
                    &format!("C:\\Export\\batch{}\\file{i}.txt", i / 2),
                    Some(9000),
                )
            })
            .collect::<Vec<_>>();
        rows.push(entry(20_000, "C:\\Other\\old.txt", Some(1)));
        let snapshot = MftSnapshot::from_entries("C:\\", Some(1), Some(100), rows);
        let mut req = request();
        req.limit = 4;
        let results = search(&snapshot, &req);
        assert_eq!(results.len(), 4);
        assert_eq!(results.last().expect("older useful work").name, "old.txt");
        assert_eq!(
            results
                .iter()
                .filter(|item| item.path.starts_with("C:\\Export\\"))
                .count(),
            3
        );
        req.limit = 2;
        assert_eq!(search(&snapshot, &req), results[..2]);
    }
    #[test]
    fn w4_recent_effective_delta_markers_and_deleted_replaced_rows() {
        let mut snapshot = MftSnapshot::from_entries(
            "C:\\",
            Some(1),
            Some(100),
            vec![
                entry(10, "C:\\Old\\CACHEDIR.TAG", None),
                entry(11, "C:\\Old\\good.txt", Some(90)),
                entry(12, "C:\\Gone\\CMakeCache.txt", None),
                entry(13, "C:\\Gone\\good.txt", Some(80)),
                entry(14, "C:\\New\\good.txt", Some(70)),
                entry(15, "C:\\Old\\deleted.txt", Some(100)),
                entry(16, "C:\\Tree\\obj\\nested\\bad.txt", Some(100)),
            ],
        );
        snapshot.overlay.deleted.extend([12, 15]);
        snapshot
            .overlay
            .replacements
            .insert(10, entry(10, "C:\\Moved\\CACHEDIR.TAG", None));
        snapshot
            .overlay
            .replacements
            .insert(17, entry(17, "C:\\New\\pyvenv.cfg", None));
        snapshot
            .overlay
            .replacements
            .insert(11, entry(11, "C:\\Old\\updated.txt", Some(95)));
        let results = search(&snapshot, &request());
        assert_eq!(
            results
                .iter()
                .map(|hit| hit.path.as_str())
                .collect::<Vec<_>>(),
            ["C:\\Old\\updated.txt", "C:\\Gone\\good.txt"]
        );
    }
    #[test]
    fn w4_recent_scope_ordinary_query_and_files_only() {
        let snapshot = MftSnapshot::from_entries(
            "C:\\",
            Some(1),
            Some(100),
            vec![
                entry(10, "C:\\Work\\report.md", Some(90)),
                entry(11, "C:\\WorkElse\\report.md", Some(100)),
                entry(12, "C:\\Work\\private\\report.md", Some(100)),
                entry(13, "D:\\report.md", Some(100)),
            ],
        );
        let settings = IndexSettings {
            indexed_volumes: vec!["C:\\".into()],
            excluded_paths: vec!["C:\\Work\\private".into()],
        };
        let mut req = request();
        req.query = "'report".into();
        req.context.directory = Some("C:/Work".into());
        let results = search_recent_modified(
            std::slice::from_ref(&snapshot),
            &settings,
            &req,
            10_000,
            &|| true,
        )
        .expect("within budget")
        .expect("current");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "C:\\Work\\report.md");
        req.context.include_files = false;
        assert!(search(&snapshot, &req).is_empty());
    }
    #[test]
    fn w4_recent_depth_aware_drive_profile_unc_and_folder_quota() {
        assert_eq!(
            project("c:\\users\\me\\documents\\alpha\\src"),
            "c:\\users\\me\\documents\\alpha"
        );
        assert_eq!(
            project("c:\\users\\me\\documents"),
            "c:\\users\\me\\documents"
        );
        assert_eq!(
            project("\\\\server\\share\\alpha\\src"),
            "\\\\server\\share\\alpha"
        );
        assert_eq!(project("d:\\alpha\\src"), "d:\\alpha");
        let snapshot = MftSnapshot::from_entries(
            "C:\\",
            Some(1),
            Some(100),
            (0..10)
                .map(|i| {
                    entry(
                        i + 10,
                        &format!("C:/Users/me/Documents/Alpha/a{i}.txt"),
                        Some(100 - i as i64),
                    )
                })
                .chain(std::iter::once(entry(
                    30,
                    "C:\\Users\\me\\Documents\\Beta\\b.txt",
                    Some(1),
                )))
                .collect(),
        );
        assert_eq!(search(&snapshot, &request()).len(), 3);
    }
    #[test]
    fn w4_recent_cancellation_discards_partial_result() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let snapshot = MftSnapshot::from_entries(
            "C:\\",
            Some(1),
            Some(100),
            (0..1000)
                .map(|i| entry(i + 10, &format!("C:\\P{i}\\a.txt"), Some(1)))
                .collect(),
        );
        let checks = AtomicUsize::new(0);
        assert!(
            search_recent_modified(
                &[snapshot],
                &IndexSettings::default(),
                &request(),
                10_000,
                &|| checks.fetch_add(1, Ordering::Relaxed) < 4
            )
            .expect("within budget")
            .is_none()
        );
    }
    #[test]
    fn w4_recent_normal_search_still_finds_noise_and_classifier_bounds_cancel() {
        let snapshot = MftSnapshot::from_entries(
            "C:\\",
            Some(1),
            Some(100),
            vec![entry(10, "C:\\Output\\result.tmp", Some(9))],
        );
        let mut req = request();
        req.query = "'result".into();
        assert!(search(&snapshot, &req).is_empty());
        let normal = crate::search_snapshots_while(
            std::slice::from_ref(&snapshot),
            &IndexSettings::default(),
            &req,
            || true,
        )
        .expect("ordinary current search");
        assert_eq!(normal.len(), 1);
        assert_eq!(normal[0].name, "result.tmp");
        assert!(normalized(&format!("C:\\{}", "x\\".repeat(MAX_DEPTH + 1))).is_none());
        assert!(normalized(&"x".repeat(MAX_PATH_BYTES + 1)).is_none());
        let mut rows = (0..MARKER_CACHE_ROOTS + 1)
            .map(|i| entry(i as u64 + 10, &format!("C:\\P{i}\\CACHEDIR.TAG"), None))
            .collect::<Vec<_>>();
        rows.push(entry(
            100_000,
            &format!("C:\\P{}\\notes.txt", MARKER_CACHE_ROOTS),
            Some(9),
        ));
        rows.push(entry(100_001, "C:\\Human\\notes.txt", Some(1)));
        let overflow = MftSnapshot::from_entries("C:\\", Some(1), Some(100), rows);
        let results = search(&overflow, &request());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "C:\\Human\\notes.txt");
    }
}
