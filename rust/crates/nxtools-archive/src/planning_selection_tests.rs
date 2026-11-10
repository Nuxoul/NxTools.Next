use super::*;
use crate::{ArchiveEntry, ArchiveFormat, NameProvenance, OperationControl};
use std::{sync::Arc, time::Instant};

fn snapshot(names: impl IntoIterator<Item = (String, EntryKind)>) -> ArchiveSnapshot {
    let session = uuid::Uuid::new_v4();
    let entries = names
        .into_iter()
        .enumerate()
        .map(|(ordinal, (name, kind))| ArchiveEntry {
            id: EntryId {
                session,
                ordinal: ordinal as u64,
            },
            raw_name: name.as_bytes().to_vec(),
            display_path: name,
            provenance: NameProvenance::FormatUnicode,
            kind,
            size: Some(u64::from(kind == EntryKind::File)),
            compressed_size: None,
            method: 0,
            encrypted: false,
            crc32: None,
            dos_modified: None,
            metadata: Default::default(),
            warnings: Vec::new(),
            zip: None,
        })
        .collect::<Vec<_>>();
    ArchiveSnapshot {
        revision: crate::model::SourceRevision {
            path: "synthetic-opt05.zip".into(),
            size: 0,
            modified: std::time::UNIX_EPOCH,
            digest: [0; 32],
            volumes: Vec::new(),
        },
        session,
        format: ArchiveFormat::Zip,
        capabilities: Default::default(),
        entries: entries.into(),
        comment: String::new(),
        backend_safe: true,
    }
}

// 128 component-ended roots, 32 nested directory IDs each, 480 files per chain.
// Maximum member depth is 33, and the identical input is reused for all completed trials.
fn large() -> (ArchiveSnapshot, Vec<EntryId>) {
    let mut names = Vec::new();
    for root in (0..128).rev() {
        let mut path = format!("r{root:03}");
        for depth in 0..32 {
            if depth != 0 {
                path.push_str("/d");
            }
            names.push((format!("{path}/"), EntryKind::Directory));
        }
        for file in 0..480 {
            let name = format!("{path}/f{file:03}");
            names.push((
                if file % 2 == 0 {
                    name.replace('/', "\\")
                } else {
                    name
                },
                EntryKind::File,
            ));
        }
    }
    let snapshot = snapshot(names);
    let ids = snapshot
        .entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Directory)
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    assert_eq!(snapshot.entries.len(), 65_536);
    assert_eq!(ids.len(), 4_096);
    (snapshot, ids)
}

fn expand(snapshot: &ArchiveSnapshot, ids: &[EntryId]) -> (ExtractionPlan, SelectionWork, u128) {
    let mut work = SelectionWork::default();
    let selection = ExtractionSelection::Entries(ids.to_vec());
    let started = Instant::now();
    let plan = ExtractionPlan::build_observed(snapshot, selection, TargetStrategy::Here, &mut work)
        .expect("original extraction boundary");
    let elapsed_us = started.elapsed().as_micros();
    assert_eq!(
        plan.files.iter().map(|entry| entry.id).collect::<Vec<_>>(),
        snapshot
            .entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::File)
            .map(|entry| entry.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(plan.explicit_directories.len(), 4_096);
    assert!(Arc::ptr_eq(&plan.projection, &snapshot.entries));
    (plan, work, elapsed_us)
}

#[test]
fn opt05_before_extraction_selection_three_trials() {
    let (snapshot, ids) = large();
    for trial in 1..=3 {
        let (plan, work, elapsed_us) = expand(&snapshot, &ids);
        eprintln!(
            "OPT05_SELECTION {}",
            serde_json::json!({
                "boundary": "extraction_plan", "trial": trial,
                "build_mode": if cfg!(debug_assertions) { "debug" } else { "release" },
                "catalog": snapshot.entries.len(), "selected_ids": ids.len(),
                "files": plan.files.len(), "elapsed_us": elapsed_us, "work": work,
            })
        );
    }
}

#[test]
fn opt05_red_extraction_selection_single_pass() {
    let (snapshot, ids) = large();
    let (_, work, _) = expand(&snapshot, &ids);
    assert!(
        work.catalog_visits <= 65_536
            && work.path_normalizations <= 65_536
            && work.path_normalizations + work.root_normalizations <= 65_536
            && work.projection_normalizations == 0
            && work.normalization_allocations <= 30_720
            && work.directory_roots <= 128
            && work.ordinal_probes <= 65_536 + 4_096
            && work.prefix_probes <= 65_536 * 16 + 4_096 * 64
            && work.prefix_allocations <= 4_096,
        "selection must normalize/expand once with merged component prefixes: {work:?}"
    );
}

#[test]
fn opt05_extraction_selection_preserves_boundaries_raw_names_and_empty_directories() {
    let snapshot = snapshot([
        ("a\\sub\\z".into(), EntryKind::File),
        ("ab/peer".into(), EntryKind::File),
        ("a/".into(), EntryKind::Directory),
        ("a/empty/".into(), EntryKind::Directory),
        ("a/sub/x".into(), EntryKind::File),
        ("loose".into(), EntryKind::File),
    ]);
    let raw = snapshot
        .entries
        .iter()
        .map(|entry| entry.raw_name.clone())
        .collect::<Vec<_>>();
    let mut work = SelectionWork::default();
    let plan = ExtractionPlan::build_observed(
        &snapshot,
        ExtractionSelection::Entries(vec![
            snapshot.entries[2].id,
            snapshot.entries[3].id,
            snapshot.entries[0].id,
        ]),
        TargetStrategy::Here,
        &mut work,
    )
    .expect("overlap and empty directory");
    assert_eq!(
        plan.files
            .iter()
            .map(|entry| entry.id.ordinal)
            .collect::<Vec<_>>(),
        [0, 4]
    );
    assert_eq!(plan.files[0].relative, "a/sub/z");
    assert_eq!(plan.directories, ["a", "a/empty", "a/sub"]);
    assert_eq!(
        plan.explicit_directories
            .iter()
            .map(|id| id.ordinal)
            .collect::<Vec<_>>(),
        [2, 3]
    );
    assert_eq!(
        snapshot
            .entries
            .iter()
            .map(|entry| entry.raw_name.clone())
            .collect::<Vec<_>>(),
        raw
    );
    let empty = ExtractionPlan::build(
        &snapshot,
        ExtractionSelection::Entries(Vec::new()),
        TargetStrategy::Name("unused".into()),
    )
    .expect("empty selection");
    assert!(empty.files.is_empty() && empty.directories.is_empty() && empty.subfolder.is_none());
    for stale in [
        EntryId {
            session: uuid::Uuid::new_v4(),
            ordinal: 0,
        },
        EntryId {
            session: snapshot.session,
            ordinal: 6,
        },
    ] {
        assert_eq!(
            ExtractionPlan::build(
                &snapshot,
                ExtractionSelection::Entries(vec![stale]),
                TargetStrategy::Here
            )
            .err(),
            Some(ArchiveError::SourceChanged)
        );
    }
}

#[test]
fn opt05_extraction_selection_keeps_collision_authority() {
    for names in [["a", "A"], ["a", "a/child"]] {
        let mut snapshot = snapshot(names.map(|name| (name.into(), EntryKind::File)));
        crate::path_policy::annotate(
            Arc::make_mut(&mut snapshot.entries),
            &OperationControl::default(),
            0,
            1 << 20,
        )
        .expect("original collision authority");
        assert_eq!(
            ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here).err(),
            Some(ArchiveError::OutputCollision)
        );
    }
}

#[test]
fn opt05_extraction_selection_budget_covers_original_scratch_and_output() {
    let snapshot = snapshot([
        ("a/".into(), EntryKind::Directory),
        ("a/leaf".into(), EntryKind::File),
    ]);
    let limits = crate::ResourceLimits {
        max_metadata_bytes: 1,
        ..Default::default()
    };
    let mut work = SelectionWork::default();
    assert_eq!(
        ExtractionPlan::build_observed_with_limits(
            &snapshot,
            ExtractionSelection::Entries(vec![snapshot.entries[0].id]),
            TargetStrategy::Here,
            &limits,
            &mut work
        )
        .err(),
        Some(ArchiveError::ResourceLimit)
    );
    let original = snapshot
        .entries
        .iter()
        .map(|entry| {
            std::mem::size_of::<ArchiveEntry>() as u64
                + entry.raw_name.len() as u64
                + entry.display_path.len() as u64
        })
        .sum::<u64>();
    let limits = crate::ResourceLimits {
        max_metadata_bytes: original + 128,
        ..Default::default()
    };
    assert_eq!(
        ExtractionPlan::build_observed_with_limits(
            &snapshot,
            ExtractionSelection::Entries(vec![snapshot.entries[0].id]),
            TargetStrategy::Here,
            &limits,
            &mut work
        )
        .err(),
        Some(ArchiveError::ResourceLimit)
    );
}

#[test]
fn opt05_selection_index_preserves_noncontiguous_ordinals_and_missing_nested_roots() {
    let catalog = [
        (9, "a\\sub\\z", false),
        (3, "a/empty/", true),
        (7, "ab/peer", false),
        (2, "a/sub/x", false),
        (19, "a-/peer", false),
    ];
    let mut work = SelectionWork::default();
    let positions = bind_ordinals(
        &catalog,
        &[2, 9],
        1 << 20,
        |item| item.0,
        || Ok(()),
        &mut work,
    )
    .expect("original IDs, not vector offsets");
    assert_eq!(positions, [3, 0]);
    let mut paths = Vec::new();
    let selected = expand_selection(
        &catalog,
        CatalogSelection::Entries {
            positions: &positions,
            directories: &["a", "a/sub"],
        },
        1 << 20,
        |_, item| SelectionEntry {
            ordinal: item.0,
            path: item.1,
            directory: item.2,
        },
        |_, _, path, _| {
            paths.push(path.to_owned());
            Ok(paths
                .iter()
                .map(|path| 2 * std::mem::size_of::<String>() as u64 + path.len() as u64)
                .sum())
        },
        || Ok(()),
        &mut work,
    )
    .expect("merged component roots");
    assert_eq!(selected, [9, 3, 2]);
    assert_eq!(paths, ["a/sub/z", "a/empty/", "a/sub/x"]);
    assert_eq!(work.directory_roots, 1);
    assert_eq!(catalog[0].1, "a\\sub\\z");
    assert_eq!(
        expand_selection(
            &catalog,
            CatalogSelection::Entries {
                positions: &[],
                directories: &["a", "a/missing"]
            },
            1 << 20,
            |_, item| SelectionEntry {
                ordinal: item.0,
                path: item.1,
                directory: item.2
            },
            |_, _, _, _| Ok(0),
            || Ok(()),
            &mut work
        )
        .err(),
        Some(ArchiveError::SourceChanged)
    );
    assert_eq!(
        expand_selection(
            &catalog,
            CatalogSelection::All,
            1,
            |_, item| SelectionEntry {
                ordinal: item.0,
                path: item.1,
                directory: item.2
            },
            |_, _, _, _| Ok(0),
            || Ok(()),
            &mut work
        )
        .err(),
        Some(ArchiveError::ResourceLimit)
    );
    let control = OperationControl::default();
    control.cancel();
    assert_eq!(
        expand_selection(
            &catalog,
            CatalogSelection::All,
            1 << 20,
            |_, item| SelectionEntry {
                ordinal: item.0,
                path: item.1,
                directory: item.2
            },
            |_, _, _, _| panic!("cancelled expansion must not emit"),
            || control.checkpoint(),
            &mut work
        )
        .err(),
        Some(ArchiveError::Cancelled)
    );
}

#[test]
fn opt05_selection_work_accumulates_one_terminal_aggregate() {
    let first = SelectionWork {
        catalog_visits: 5,
        root_normalizations: 2,
        ..Default::default()
    };
    let second = SelectionWork {
        catalog_visits: 7,
        path_normalizations: 6,
        ordinal_probes: 3,
        ..Default::default()
    };
    let mut aggregate = first;
    aggregate += second;
    assert_eq!(aggregate.catalog_visits, 12);
    assert_eq!(aggregate.root_normalizations, 2);
    assert_eq!(aggregate.path_normalizations, 6);
    assert_eq!(aggregate.ordinal_probes, 3);
}

#[test]
fn opt05_selection_temp_path_credit_includes_retained_output() {
    let long = format!("b\\{}", "x".repeat(200));
    let catalog = [(9, "a/file"), (7, long.as_str())];
    let mut calls = 0;
    let mut work = SelectionWork::default();
    let result = expand_selection(
        &catalog,
        CatalogSelection::Entries {
            positions: &[],
            directories: &["a"],
        },
        512,
        |_, item| SelectionEntry {
            ordinal: item.0,
            path: item.1,
            directory: false,
        },
        |_, _, _, _| {
            calls += 1;
            Ok(100)
        },
        || Ok(()),
        &mut work,
    );
    assert_eq!(result.err(), Some(ArchiveError::ResourceLimit));
    assert_eq!(calls, 1);
    assert_eq!(
        work.normalization_allocations, 0,
        "credit checked before allocating the later temporary path"
    );
}

#[test]
fn opt05_selection_working_limit_bounds_extraction_scratch() {
    let snapshot = snapshot([("a/file".into(), EntryKind::File)]);
    let limits = crate::ResourceLimits {
        max_working_bytes: 1,
        ..Default::default()
    };
    assert_eq!(
        ExtractionPlan::build_observed_with_limits(
            &snapshot,
            ExtractionSelection::All,
            TargetStrategy::Here,
            &limits,
            &mut SelectionWork::default()
        )
        .err(),
        Some(ArchiveError::ResourceLimit)
    );
}
