use super::*;
use crate::mft::{
    RawMftRecord, SnapshotConstructor, enumerate_initial_snapshot, scan_volume_and_persist_with,
};
use crate::{IndexerService, VolumeDescriptor};
use nxtools_protocol::{IndexPhase, IndexSearchContext, IndexSearchRequest};
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::{Event, Metadata, Subscriber};

const MOUNT: &str = r"C:\";
const STAGES: [&str; 6] = [
    "indexer.snapshot.native_enumeration",
    "indexer.snapshot.enumeration",
    "indexer.snapshot.construction",
    "indexer.snapshot.scan",
    "indexer.snapshot.persist",
    "indexer.snapshot.reopen",
];

#[derive(Clone, Debug, Default)]
struct Stage {
    event: String,
    metrics: BTreeMap<String, u64>,
}
impl Visit for Stage {
    fn record_u64(&mut self, field: &Field, value: u64) {
        if matches!(
            field.name(),
            "rows"
                | "raw_rows"
                | "elapsed_ms"
                | "metadata_rows"
                | "metadata_elapsed_ms"
                | "metadata_reads"
                | "metadata_unknown"
                | "metadata_fallback_reads"
        ) {
            self.metrics.insert(field.name().into(), value);
        }
    }
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "event" && STAGES.contains(&value) {
            self.event = value.into();
        }
    }
    fn record_debug(&mut self, _: &Field, _: &dyn std::fmt::Debug) {}
}

#[derive(Clone, Default)]
struct StageCapture {
    stages: Arc<Mutex<Vec<Stage>>>,
    next_span: Arc<AtomicU64>,
}
impl Subscriber for StageCapture {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= tracing::Level::INFO
    }
    fn new_span(&self, _: &Attributes<'_>) -> Id {
        Id::from_u64(self.next_span.fetch_add(1, Ordering::Relaxed) + 1)
    }
    fn record(&self, _: &Id, _: &Record<'_>) {}
    fn record_follows_from(&self, _: &Id, _: &Id) {}
    fn event(&self, event: &Event<'_>) {
        let mut stage = Stage::default();
        event.record(&mut stage);
        if !stage.event.is_empty() {
            let mut stages = self.stages.lock().expect("profile stage lock");
            assert!(
                stages.len() < 16,
                "profile must collect only bounded phase events"
            );
            stages.push(stage);
        }
    }
    fn enter(&self, _: &Id) {}
    fn exit(&self, _: &Id) {}
}

fn milliseconds(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn profile_pipeline(
    directory: &Path,
    scan: impl Fn(&str, SnapshotConstructor) -> Result<MftSnapshot>,
    request: &IndexSearchRequest,
    expected_reference: u64,
) -> (IndexerService, serde_json::Value) {
    let path = directory.join("profile-base.idx");
    assert!(
        !path.exists(),
        "profile must start without an existing index"
    );
    let capture = StageCapture::default();
    let mapped_at = Cell::new(None::<Duration>);
    let (service, build_elapsed, first_success, query_elapsed, matches) =
        tracing::subscriber::with_default(capture.clone(), || {
            let mut service =
                IndexerService::from_volume_descriptors([VolumeDescriptor::new(MOUNT, "NTFS")]);
            let started = Instant::now();
            service.rebuild_with(|mount| {
                let outcome = scan_volume_and_persist_with(mount, &scan, |snapshot| {
                    assert!(
                        snapshot.search_data.is_none(),
                        "unpublished base must be persistence-only"
                    );
                    assert!(snapshot.reference_index.is_empty());
                    assert!(snapshot.children_index.is_empty());
                    assert!(snapshot.owned_name_masks.is_none());
                    persist_snapshot(snapshot, &path)?;
                    snapshot.set_cache_path(path.clone());
                    Ok(())
                });
                mapped_at.set(Some(started.elapsed()));
                outcome
            });
            let build_elapsed = started.elapsed();
            assert_eq!(service.status().phase, IndexPhase::Ready);
            assert!(
                service.status().last_error.is_none(),
                "isolated build must succeed"
            );
            assert!(matches!(
                service.snapshots[0].base,
                crate::mft::MftBase::Mapped(_)
            ));
            assert_eq!(service.snapshots[0].cache_path(), Some(path.as_path()));
            let expected = service.snapshots[0]
                .visible_entry(expected_reference)
                .expect("independent witness identity is indexed");
            let mut probe = request.clone();
            probe.context.preferred_directory = expected
                .path
                .rsplit_once('\\')
                .map(|(parent, _)| parent.to_owned());
            let query_started = Instant::now();
            let results = service.search_snapshot(&probe);
            assert!(
                results
                    .iter()
                    .any(|item| item.path.eq_ignore_ascii_case(&expected.path)),
                "first real search must find the independent witness"
            );
            let query_elapsed = query_started.elapsed();
            let first_success = started.elapsed();
            (
                service,
                build_elapsed,
                first_success,
                query_elapsed,
                results.len(),
            )
        });
    let stages = capture.stages.lock().expect("completed profile stages");
    for required in STAGES {
        let matching = stages
            .iter()
            .filter(|stage| stage.event == required)
            .collect::<Vec<_>>();
        assert_eq!(
            matching.len(),
            1,
            "each production phase must be observed once: {required}"
        );
        assert!(
            matching[0].metrics.contains_key("elapsed_ms"),
            "phase timing is required"
        );
    }
    let stage_values = stages
        .iter()
        .map(|stage| {
            serde_json::json!({
                "event": stage.event, "metrics": stage.metrics,
            })
        })
        .collect::<Vec<_>>();
    let report = serde_json::json!({
        "rows": service.status().indexed_items,
        "base_bytes": fs::metadata(&path).expect("profile base metadata").len(),
        "mapped_ready_ms": milliseconds(mapped_at.get().expect("mapped completion")),
        "build_complete_ms": milliseconds(build_elapsed),
        "publish_and_cleanup_ms": milliseconds(build_elapsed.saturating_sub(mapped_at.get().expect("mapped completion"))),
        "first_query_ms": milliseconds(query_elapsed),
        "first_search_success_ms": milliseconds(first_success),
        "first_query_matches": matches,
        "publication": "isolated_in_process",
        "stages": stage_values,
    });
    (service, report)
}

fn request(query: &str, preferred_directory: &str) -> IndexSearchRequest {
    IndexSearchRequest {
        generation: 1,
        query: query.into(),
        limit: 20,
        context: IndexSearchContext {
            include_files: true,
            include_directories: false,
            preferred_directory: Some(preferred_directory.into()),
            ..Default::default()
        },
    }
}

#[test]
fn release_profile_seam_captures_real_persistence_reopen_and_search() {
    let directory = tempfile::tempdir().expect("isolated profile regression cache");
    let (service, report) = profile_pipeline(
        directory.path(),
        |mount, construct| {
            enumerate_initial_snapshot(
                mount,
                17,
                1234,
                |high_usn| {
                    assert_eq!(high_usn, i64::MAX);
                    tracing::info!(
                        event = "indexer.snapshot.native_enumeration",
                        rows = 1_u64,
                        elapsed_ms = 0_u64,
                        ignored_payload = "synthetic private payload"
                    );
                    Ok(vec![RawMftRecord {
                        last_usn: 100,
                        file_reference: 10,
                        parent_reference: 5,
                        name: "Budget.txt".into(),
                        is_directory: false,
                        modified_at_unix_ms: Some(123),
                    }])
                },
                construct,
            )
        },
        &request("\"Budget.txt\"", MOUNT),
        10,
    );
    assert_eq!(report["rows"], 1);
    assert_eq!(report["first_query_matches"], 1);
    assert_eq!(report["stages"].as_array().expect("phase array").len(), 6);
    assert!(!report.to_string().contains("synthetic private payload"));
    drop(service);
}

#[test]
#[ignore = "requires an optimized Release build, explicit C: scan authorization and elevation"]
fn initial_build_release_phase_profile() {
    if cfg!(debug_assertions) {
        panic!("native timing requires an optimized Release build");
    }
    assert_eq!(
        std::env::var("NXTOOLS_INITIAL_SCAN_VOLUME").expect("authorized scan volume"),
        MOUNT
    );
    let scratch = PathBuf::from(std::env::var_os("PI_SCRATCH_DIR").expect("owned profile scratch"));
    let report_path = PathBuf::from(
        std::env::var_os("NXTOOLS_INITIAL_PROFILE_REPORT").expect("owned report path"),
    );
    assert_eq!(
        report_path.parent(),
        Some(scratch.as_path()),
        "write the report only in this session scratch"
    );
    let run_id = std::env::var("NXTOOLS_INITIAL_PROFILE_RUN").expect("profile correlation ID");
    let directory = tempfile::Builder::new()
        .prefix("initial-index-profile-")
        .tempdir_in(&scratch)
        .expect("new isolated profile cache");
    let system =
        PathBuf::from(std::env::var_os("SystemRoot").expect("Windows root")).join("System32");
    assert!(
        system
            .to_string_lossy()
            .to_ascii_lowercase()
            .starts_with(r"c:\")
    );
    let witnesses = ["kernel32.dll", "cmd.exe", "notepad.exe"].map(|name| {
        let path = system.join(name);
        let before = crate::mft::initial_build_tests::system_file_metadata(&path);
        (path, before)
    });
    let attributes_before = crate::mft::time_metadata::attribute_read_count();
    let fallbacks_before = crate::mft::bulk_times::fallback_read_count();
    let expected_reference = witnesses[0].1.reference;
    let (service, mut report) = profile_pipeline(
        directory.path(),
        crate::mft::windows::scan_volume,
        &request("\"kernel32.dll\"", &system.to_string_lossy()),
        expected_reference,
    );
    let mut verified = 0_u64;
    for (path, before) in witnesses {
        if crate::mft::initial_build_tests::system_file_metadata(&path) != before {
            continue;
        }
        let row = service.snapshots[0]
            .visible_entry(before.reference)
            .expect("stable witness identity");
        assert_eq!(
            row.modified_at_unix_ms, before.modified,
            "mapped true-time witness"
        );
        verified += 1;
    }
    assert!(
        verified > 0,
        "at least one independent witness must remain stable"
    );
    let attribute_reads = crate::mft::time_metadata::attribute_read_count() - attributes_before;
    let freshness_fallbacks = crate::mft::bulk_times::fallback_read_count() - fallbacks_before;
    assert_eq!(attribute_reads, freshness_fallbacks);
    report["attribute_reads"] = attribute_reads.into();
    report["freshness_fallbacks"] = freshness_fallbacks.into();
    report["verified_system_files"] = verified.into();
    report["optimized_release"] = true.into();
    report["run_id"] = run_id.into();
    let bytes = serde_json::to_vec_pretty(&report).expect("numeric profile serialization");
    fs::write(report_path, bytes).expect("write owned profile report");
    println!("INITIAL_RELEASE_PROFILE {report}");
    drop(service);
}
