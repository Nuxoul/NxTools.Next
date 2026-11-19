#![cfg(feature = "isolated-codecs")]
#[path = "common/bulk_peer.rs"]
mod bulk_peer;
use bulk_peer::{read_frame, write_frame};
use nxtools_archive::worker::WorkerRequest;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
};
use uuid::Uuid;

#[test]
fn ref19_alz_stored_browse_test_extract_exact_bytes() {
    let scratch = std::env::var("PI_SCRATCH_DIR").expect("owned scratch");
    let owned = tempfile::Builder::new()
        .prefix("ref19-alz-")
        .tempdir_in(scratch)
        .expect("owned source directory");
    let source = owned.path().join("ordinary.alz");
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/alzeg/store_old.alz"),
        &source,
    )
    .expect("copy independently verified CC0 sample");
    let (listed, catalog, outputs) = worker(&source, "open");
    assert_eq!(listed["op"], "complete", "ALZ Browse: {listed}");
    assert_eq!(catalog.len(), 1);
    assert_eq!(catalog[0]["name"], "text.txt");
    assert_eq!(catalog[0]["size"], 3600);
    assert!(outputs.is_empty(), "Browse is read-only");

    let (tested, _, outputs) = worker(&source, "test");
    assert_eq!(tested["op"], "complete", "ALZ Test: {tested}");
    assert_eq!(tested["files_verified"], 1);
    assert_eq!(tested["bytes_verified"], 3600);
    assert!(outputs.is_empty(), "Test is read-only");

    let (extracted, _, outputs) = worker(&source, "extract");
    assert_eq!(extracted["op"], "complete", "ALZ extract: {extracted}");
    assert_eq!(outputs.len(), 1);
    assert_eq!(
        outputs.get("text.txt").expect("exact original member name"),
        &b"The quick brown fox jumps over the lazy dog. ".repeat(80)
    );
    fs::remove_file(source).expect("source reader released after actual child exit");
}

#[test]
fn ref19_ordinary_alz_deflate() {
    ordinary("deflate_empty.alz", five_members());
}
#[test]
fn ref19_ordinary_egg_stored() {
    ordinary("store5.egg", five_members());
}
#[test]
fn ref19_ordinary_egg_deflate() {
    ordinary("deflate5.egg", five_members());
}
#[test]
fn ref19_ordinary_egg_mixed_methods() {
    ordinary("mixed_store_deflate5.egg", five_members());
}
#[test]
fn ref19_ordinary_egg_lzma() {
    ordinary("lzma5.egg", five_members());
}
#[test]
fn ref19_ordinary_egg_bzip2() {
    ordinary(
        "bzip2_big.egg",
        BTreeMap::from([(
            "huge.txt".into(),
            b"The quick brown fox jumps over the lazy dog. ".repeat(180000),
        )]),
    );
}
#[test]
fn ref19_ordinary_egg_empty_file() {
    ordinary(
        "empty_noblock.egg",
        BTreeMap::from([("empty.txt".into(), Vec::new())]),
    );
}
fn five_members() -> Payloads {
    BTreeMap::from([
        ("empty.txt".into(), Vec::new()),
        (
            "precompressed.gz".into(),
            include_bytes!("fixtures/alzeg/expected/1.bin").to_vec(),
        ),
        (
            "prog.sys".into(),
            include_bytes!("fixtures/alzeg/expected/2.bin").to_vec(),
        ),
        (
            "random.bin".into(),
            include_bytes!("fixtures/alzeg/expected/3.bin").to_vec(),
        ),
        (
            "text.txt".into(),
            include_bytes!("fixtures/alzeg/expected/4.bin").to_vec(),
        ),
    ])
}
fn ordinary(fixture: &str, expected: Payloads) {
    let scratch = std::env::var("PI_SCRATCH_DIR").expect("owned scratch");
    let owned = tempfile::Builder::new()
        .prefix("ref19-ordinary-")
        .tempdir_in(scratch)
        .expect("owned directory");
    let source = owned.path().join(fixture);
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/alzeg")
            .join(fixture),
        &source,
    )
    .expect("independently qualified sample");
    let (listed, catalog, outputs) = worker(&source, "open");
    assert_eq!(listed["op"], "complete", "{fixture} Browse: {listed}");
    let actual: BTreeMap<_, _> = catalog
        .iter()
        .map(|entry| {
            (
                entry["name"].as_str().expect("name").to_owned(),
                entry["size"].as_u64().expect("size"),
            )
        })
        .collect();
    let expected_sizes: BTreeMap<_, _> = expected
        .iter()
        .map(|(name, bytes)| (name.clone(), bytes.len() as u64))
        .collect();
    assert_eq!(
        catalog.len(),
        expected.len(),
        "exact catalog without duplicate names"
    );
    assert_eq!(actual, expected_sizes, "exact original names and sizes");
    assert!(outputs.is_empty(), "Browse is read-only");
    let (tested, _, outputs) = worker(&source, "test");
    assert_eq!(tested["op"], "complete", "{fixture} Test: {tested}");
    assert_eq!(tested["files_verified"], expected.len() as u64);
    assert_eq!(
        tested["bytes_verified"],
        expected
            .values()
            .map(|bytes| bytes.len() as u64)
            .sum::<u64>()
    );
    assert!(outputs.is_empty(), "Test is read-only");
    let (extracted, _, outputs) = worker(&source, "extract");
    assert_eq!(
        extracted["op"], "complete",
        "{fixture} extract: {extracted}"
    );
    assert_eq!(
        outputs, expected,
        "every member byte equals independent oracle"
    );
    fs::remove_file(source).expect("source reader released after actual child exits");
}

fn owned_source(fixture: &str) -> (tempfile::TempDir, std::path::PathBuf) {
    let scratch = std::env::var("PI_SCRATCH_DIR").expect("owned scratch");
    let owned = tempfile::Builder::new()
        .prefix("ref19-failure-")
        .tempdir_in(scratch)
        .expect("owned directory");
    let source = owned.path().join(fixture);
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/alzeg")
            .join(fixture),
        &source,
    )
    .expect("qualified fixture");
    (owned, source)
}
#[test]
fn ref19_failure_crc_aborts_unverified_output() {
    for (fixture, crc_offset) in [("store_old.alz", 23), ("bzip2_big.egg", 83)] {
        let (_owned, source) = owned_source(fixture);
        let mut data = fs::read(&source).expect("source");
        data[crc_offset] ^= 1;
        fs::write(&source, data).expect("owned corrupt CRC");
        assert_eq!(worker(&source, "open").0["op"], "complete");
        for operation in ["test", "extract"] {
            let (terminal, _, outputs) = worker(&source, operation);
            assert_eq!(
                terminal["error_code"], "CorruptArchive",
                "{fixture} {operation}"
            );
            assert!(outputs.is_empty(), "unverified member never commits");
        }
        fs::remove_file(source).expect("reader released");
    }
}
#[test]
fn ref19_failure_structural_end_and_extent() {
    for fixture in ["store_old.alz", "bzip2_big.egg"] {
        let (_owned, source) = owned_source(fixture);
        let original = fs::read(&source).expect("source");
        let mut wrong_end = original.clone();
        *wrong_end.last_mut().expect("end") ^= 0x80;
        let mut trailing = original.clone();
        trailing.push(0);
        let mut cases = vec![wrong_end, trailing, original[..original.len() - 1].to_vec()];
        if fixture.ends_with(".egg") {
            for marker in [14, 65, 87] {
                let mut data = original.clone();
                data[marker] ^= 1;
                cases.push(data);
            }
        }
        for data in cases {
            fs::write(&source, data).expect("owned corrupt framing");
            let (terminal, catalog, outputs) = worker(&source, "open");
            assert_eq!(
                terminal["error_code"], "CorruptArchive",
                "{fixture}: {terminal}"
            );
            assert!(
                catalog.is_empty() && outputs.is_empty(),
                "no partial catalog or output"
            );
        }
        fs::remove_file(source).expect("reader released");
    }
}
#[test]
fn ref19_failure_trailing_packed_bytes_are_not_ignored() {
    let (_owned, source) = owned_source("bzip2_big.egg");
    let mut data = fs::read(&source).expect("source");
    let packed = u32::from_le_bytes(data[79..83].try_into().expect("packed"));
    data.splice(91 + packed as usize..91 + packed as usize, [0]);
    data[79..83].copy_from_slice(&(packed + 1).to_le_bytes());
    fs::write(&source, data).expect("complete envelope with trailing packed byte");
    assert_eq!(worker(&source, "open").0["op"], "complete");
    for operation in ["test", "extract"] {
        let (terminal, _, outputs) = worker(&source, operation);
        assert_eq!(terminal["error_code"], "CorruptArchive");
        assert!(
            outputs.is_empty(),
            "exact packed consumption precedes commit"
        );
    }
    fs::remove_file(source).expect("reader released");
}
#[test]
fn ref19_failure_finite_budgets_preserve_typed_outcome() {
    for fixture in [
        "deflate_empty.alz",
        "store5.egg",
        "lzma5.egg",
        "bzip2_big.egg",
    ] {
        let (_owned, source) = owned_source(fixture);
        for budget in ["metadata", "working", "entry", "total"] {
            let operation = if budget == "metadata" { "open" } else { "test" };
            let (terminal, _, outputs) = worker_options(
                &source,
                operation,
                |request| match budget {
                    "metadata" => request.limits.max_metadata_bytes = 512,
                    "working" => request.limits.max_working_bytes = 1,
                    "entry" => request.limits.max_entry_bytes = 1,
                    "total" => request.limits.max_output_bytes = 1,
                    _ => unreachable!(),
                },
                Effect::None,
            );
            assert_eq!(
                terminal["error_code"], "ResourceLimit",
                "{fixture}/{budget}: {terminal}"
            );
            assert!(outputs.is_empty());
        }
        fs::remove_file(source).expect("reader released");
    }
}
#[test]
fn ref19_failure_real_cancel_drains_child_and_output() {
    for fixture in ["store_old.alz", "bzip2_big.egg"] {
        let (_owned, source) = owned_source(fixture);
        let (terminal, _, outputs) =
            worker_options(&source, "extract", |_| {}, Effect::CancelFirstWrite);
        assert_eq!(terminal["error_code"], "Cancelled", "{fixture}: {terminal}");
        assert!(outputs.is_empty(), "cancelled member never commits");
        fs::remove_file(source).expect("reader released after actual exit");
    }
}
#[test]
fn ref19_failure_native_write_is_terminal_and_aborts_output() {
    for fixture in ["store_old.alz", "bzip2_big.egg"] {
        let (_owned, source) = owned_source(fixture);
        let (terminal, _, outputs) =
            worker_options(&source, "extract", |_| {}, Effect::FailFirstWrite);
        assert_eq!(
            terminal["error_code"], "ResourceLimit",
            "{fixture}: {terminal}"
        );
        assert!(outputs.is_empty(), "failed native write never commits");
        fs::remove_file(source).expect("reader released after actual exit");
    }
}
#[test]
fn ref19_ordinary_egg_lzma_opaque_prefix() {
    let (_owned, source) = owned_source("lzma5.egg");
    let mut data = fs::read(&source).expect("source");
    data[151..155].copy_from_slice(&[0, 0, 0, 0]);
    fs::write(&source, data).expect("opaque EGG prefix");
    let (terminal, _, outputs) = worker(&source, "extract");
    assert_eq!(terminal["op"], "complete", "{terminal}");
    assert_eq!(outputs, five_members());
    fs::remove_file(source).expect("reader released");
}

const SPLIT_FAMILIES: [&str; 4] = [
    "split_store.alz",
    "split.alz",
    "split_store.vol1.egg",
    "split.vol1.egg",
];
fn split_fixture(first: &str) -> (tempfile::TempDir, Vec<std::path::PathBuf>, Payloads) {
    let scratch = std::env::var("PI_SCRATCH_DIR").expect("owned scratch");
    let owned = tempfile::Builder::new()
        .prefix("ref19-split-")
        .tempdir_in(scratch)
        .expect("owned family");
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/alzeg");
    let manifest: Value =
        serde_json::from_str(include_str!("fixtures/alzeg/expected/split/manifest.json"))
            .expect("independent manifest");
    let family = manifest["families"]
        .as_array()
        .expect("families")
        .iter()
        .find(|family| family["first"] == first)
        .expect("qualified family");
    let parts = family["parts"]
        .as_array()
        .expect("parts")
        .iter()
        .map(|part| {
            let name = part.as_str().expect("part name");
            let target = owned.path().join(name);
            fs::copy(fixtures.join(name), &target).expect("unchanged CC0 volume");
            target
        })
        .collect();
    let expected = family["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|member| {
            let encoded = fs::File::open(
                fixtures
                    .join("expected/split")
                    .join(member["expected"].as_str().expect("expected payload")),
            )
            .expect("independent bytes");
            let mut bytes = Vec::new();
            flate2::read::GzDecoder::new(encoded)
                .read_to_end(&mut bytes)
                .expect("expected gzip envelope");
            assert_eq!(bytes.len() as u64, member["size"].as_u64().expect("size"));
            (member["name"].as_str().expect("name").into(), bytes)
        })
        .collect();
    (owned, parts, expected)
}
fn family_worker(
    parts: &[std::path::PathBuf],
    operation: &str,
    effect: Effect,
) -> (Value, Vec<Value>, Payloads) {
    worker_options(
        &parts[0],
        operation,
        |request| {
            request.volume_paths = parts
                .iter()
                .map(|path| path.to_string_lossy().into_owned())
                .collect();
        },
        effect,
    )
}
fn release_family(parts: &[std::path::PathBuf]) {
    for part in parts {
        fs::remove_file(part).expect("every source released after actual child/pipe exit");
    }
}
fn split_exact(first: &str) {
    let (_owned, parts, expected) = split_fixture(first);
    let (terminal, catalog, outputs) = family_worker(&parts, "open", Effect::None);
    assert_eq!(terminal["op"], "complete", "{first} Browse: {terminal}");
    let sizes: BTreeMap<_, _> = catalog
        .iter()
        .map(|entry| {
            (
                entry["name"].as_str().expect("name").to_owned(),
                entry["size"].as_u64().expect("size"),
            )
        })
        .collect();
    assert_eq!(catalog.len(), expected.len(), "no duplicate catalog names");
    assert_eq!(
        sizes,
        expected
            .iter()
            .map(|(name, bytes)| (name.clone(), bytes.len() as u64))
            .collect()
    );
    assert!(outputs.is_empty());
    let (terminal, _, outputs) = family_worker(&parts, "test", Effect::None);
    assert_eq!(terminal["op"], "complete", "{first} Test: {terminal}");
    assert_eq!(terminal["files_verified"], expected.len() as u64);
    assert_eq!(
        terminal["bytes_verified"],
        expected
            .values()
            .map(|bytes| bytes.len() as u64)
            .sum::<u64>()
    );
    assert!(outputs.is_empty());
    let (terminal, _, outputs) = family_worker(&parts, "extract", Effect::None);
    assert_eq!(terminal["op"], "complete", "{first} extract: {terminal}");
    assert_eq!(
        outputs, expected,
        "all cross-volume bytes equal independent oracle"
    );
    release_family(&parts);
}
#[test]
fn ref19_split_alz_stored() {
    split_exact("split_store.alz");
}
#[test]
fn ref19_split_alz_deflate() {
    split_exact("split.alz");
}
#[test]
fn ref19_split_egg_stored() {
    split_exact("split_store.vol1.egg");
}
#[test]
fn ref19_split_egg_deflate() {
    split_exact("split.vol1.egg");
}
#[test]
fn ref19_split_missing_or_reordered_supplied_set_never_discovers_siblings() {
    for first in SPLIT_FAMILIES {
        let (_owned, parts, _) = split_fixture(first);
        let mut missing_last = parts.clone();
        missing_last.pop();
        let mut missing_middle = parts.clone();
        missing_middle.remove(1);
        let mut reordered = parts.clone();
        reordered.swap(1, 2);
        for supplied in [
            vec![parts[0].clone()],
            missing_last,
            missing_middle,
            reordered,
        ] {
            let (terminal, catalog, outputs) = family_worker(&supplied, "open", Effect::None);
            assert_eq!(
                terminal["error_code"], "MissingVolume",
                "{first}: {terminal}"
            );
            assert!(
                catalog.is_empty() && outputs.is_empty(),
                "no partial success"
            );
        }
        release_family(&parts);
    }
}
#[test]
fn ref19_split_corrupt_physical_framing_is_not_concatenated() {
    for first in SPLIT_FAMILIES {
        let (_owned, parts, _) = split_fixture(first);
        let original = fs::read(&parts[1]).expect("continuation");
        let marker = if first.ends_with(".alz") {
            original.len() - 1
        } else {
            29
        };
        for at in [0, marker] {
            let mut damaged = original.clone();
            damaged[at] ^= 1;
            fs::write(&parts[1], damaged).expect("owned damaged framing");
            let (terminal, catalog, outputs) = family_worker(&parts, "open", Effect::None);
            assert_eq!(
                terminal["error_code"], "CorruptArchive",
                "{first}: {terminal}"
            );
            assert!(catalog.is_empty() && outputs.is_empty());
        }
        release_family(&parts);
    }
}
#[test]
fn ref19_split_cross_volume_crc_precedes_output_commit() {
    for first in ["split.alz", "split.vol1.egg"] {
        let (_owned, parts, _) = split_fixture(first);
        let mut damaged = fs::read(&parts[1]).expect("continuation payload");
        damaged[200] ^= 1;
        fs::write(&parts[1], damaged).expect("owned corrupt payload");
        assert_eq!(
            family_worker(&parts, "open", Effect::None).0["op"],
            "complete"
        );
        for operation in ["test", "extract"] {
            let (terminal, _, outputs) = family_worker(&parts, operation, Effect::None);
            assert_eq!(
                terminal["error_code"], "CorruptArchive",
                "{first}: {terminal}"
            );
            assert!(outputs.is_empty(), "first spanning member never commits");
        }
        release_family(&parts);
    }
}
#[test]
fn ref19_split_real_cancel_drains_all_sources() {
    for first in SPLIT_FAMILIES {
        let (_owned, parts, _) = split_fixture(first);
        let (terminal, _, outputs) = family_worker(&parts, "extract", Effect::CancelFirstWrite);
        assert_eq!(terminal["error_code"], "Cancelled", "{first}: {terminal}");
        assert!(outputs.is_empty());
        release_family(&parts);
    }
}
#[test]
fn ref19_split_native_write_failure_drains_all_sources() {
    for first in SPLIT_FAMILIES {
        let (_owned, parts, _) = split_fixture(first);
        let (terminal, _, outputs) = family_worker(&parts, "extract", Effect::FailFirstWrite);
        assert_eq!(
            terminal["error_code"], "ResourceLimit",
            "{first}: {terminal}"
        );
        assert!(outputs.is_empty());
        release_family(&parts);
    }
}
#[test]
fn ref19_split_continuation_change_prevents_output_commit() {
    for first in ["split.alz", "split.vol1.egg"] {
        let (_owned, parts, _) = split_fixture(first);
        let (terminal, _, outputs) = family_worker(&parts, "extract", Effect::ChangeContinuation);
        assert_eq!(
            terminal["error_code"], "SourceChanged",
            "{first}: {terminal}"
        );
        assert!(outputs.is_empty());
        release_family(&parts);
    }
}
#[test]
fn ref19_split_egg_file_header_must_not_cross_physical_boundary() {
    let (_owned, parts, _) = split_fixture("split_store.vol1.egg");
    let first = fs::read(&parts[0]).expect("first");
    let mut second = fs::read(&parts[1]).expect("second");
    second.splice(33..33, first[50..].iter().copied());
    fs::write(&parts[0], &first[..50]).expect("same logical data, file header crosses boundary");
    fs::write(&parts[1], second).expect("same reciprocal linked prefix");
    let (terminal, catalog, outputs) = family_worker(&parts, "open", Effect::None);
    assert_eq!(terminal["error_code"], "CorruptArchive", "{terminal}");
    assert!(catalog.is_empty() && outputs.is_empty());
    release_family(&parts);
}
#[test]
fn ref19_split_finite_budgets_preserve_typed_outcome() {
    for first in SPLIT_FAMILIES {
        let (_owned, parts, _) = split_fixture(first);
        for budget in ["metadata", "working", "entry", "total"] {
            let (terminal, _, outputs) = worker_options(
                &parts[0],
                if budget == "metadata" { "open" } else { "test" },
                |request| {
                    request.volume_paths = parts
                        .iter()
                        .map(|path| path.to_string_lossy().into_owned())
                        .collect();
                    match budget {
                        "metadata" => request.limits.max_metadata_bytes = 32,
                        "working" => request.limits.max_working_bytes = 1,
                        "entry" => request.limits.max_entry_bytes = 1,
                        "total" => request.limits.max_output_bytes = 1,
                        _ => unreachable!(),
                    }
                },
                Effect::None,
            );
            assert_eq!(
                terminal["error_code"], "ResourceLimit",
                "{first}/{budget}: {terminal}"
            );
            assert!(outputs.is_empty());
        }
        release_family(&parts);
    }
}

#[test]
fn ref19_source_revision_work_is_stage_bounded() {
    for first in SPLIT_FAMILIES {
        let (_owned, parts, expected) = split_fixture(first);
        let size: u64 = parts
            .iter()
            .map(|part| fs::metadata(part).expect("source size").len())
            .sum();
        let (terminal, _, outputs) = family_worker(&parts, "extract", Effect::None);
        assert_eq!(terminal["op"], "complete", "{first}: {terminal}");
        assert_eq!(
            outputs, expected,
            "all independently qualified member bytes"
        );
        assert_eq!(
            terminal["work"]["revision_passes"],
            parts.len() as u64 * 4,
            "initial, normalized, first-commit and final source scans only: {first}"
        );
        assert_eq!(
            terminal["work"]["revision_bytes"],
            size * 4,
            "no per-member family amplification"
        );
        release_family(&parts);
    }
    for fixture in ["store_old.alz", "store5.egg"] {
        let (_owned, source) = owned_source(fixture);
        let size = fs::metadata(&source).expect("source size").len();
        let (terminal, _, outputs) = worker(&source, "extract");
        assert_eq!(terminal["op"], "complete", "{terminal}");
        assert_eq!(
            outputs,
            if fixture.ends_with(".alz") {
                BTreeMap::from([(
                    "text.txt".into(),
                    b"The quick brown fox jumps over the lazy dog. ".repeat(80),
                )])
            } else {
                five_members()
            }
        );
        assert_eq!(
            terminal["work"]["revision_passes"], 3,
            "ordinary source has no normalization scan"
        );
        assert_eq!(terminal["work"]["revision_bytes"], size * 3);
        fs::remove_file(source).expect("actual reader released");
    }
}
#[test]
fn ref19_final_source_change_retains_completed_output_facts() {
    for first in ["split_store.alz", "split_store.vol1.egg"] {
        let (_owned, parts, expected) = split_fixture(first);
        let (terminal, _, outputs) =
            family_worker(&parts, "extract", Effect::ChangeAfterLastCommit);
        assert_eq!(
            terminal["error_code"], "SourceChanged",
            "final source validation: {first}: {terminal}"
        );
        assert_eq!(
            outputs, expected,
            "already completed bytes are not rolled back or denied"
        );
        release_family(&parts);
    }
}
#[test]
fn ref19_low_output_reaches_source_bound_shortfall_without_writes() {
    for first in ["split_store.alz", "split_store.vol1.egg"] {
        let (_owned, parts, _) = split_fixture(first);
        let (terminal, _, outputs) = worker_options(
            &parts[0],
            "extract",
            |request| {
                request.volume_paths = parts
                    .iter()
                    .map(|part| part.to_string_lossy().into_owned())
                    .collect();
                request.limits.max_metadata_bytes = 8192;
                request.limits.max_entry_bytes = 1;
                request.limits.max_output_bytes = 1;
                request.output_readmission =
                    Some(nxtools_archive::output_readmission::ReadmissionOptions {
                        logical_id: request.operation_id,
                        expected_source: None,
                    });
            },
            Effect::None,
        );
        assert_eq!(terminal["error_code"], "ResourceLimit", "{terminal}");
        assert_eq!(
            terminal["output_budget_shortfall"]["current"],
            json!({"entry_bytes":1,"output_bytes":1})
        );
        assert_eq!(
            terminal["output_budget_shortfall"]["required"],
            json!({"entry_bytes":40000,"output_bytes":160000}),
            "original declared-output preflight must remain reachable: {first}"
        );
        assert_eq!(
            terminal["output_budget_shortfall"]["source"]["native"],
            "v1:ref19-owned-source"
        );
        assert!(outputs.is_empty());
        release_family(&parts);
    }
}

fn encrypted_expected(fixture: &str) -> Payloads {
    let manifest: Value = serde_json::from_str(include_str!(
        "fixtures/alzeg/expected/encrypted/manifest.json"
    ))
    .expect("independent oracle manifest");
    let family = manifest["fixtures"]
        .as_array()
        .expect("fixtures")
        .iter()
        .find(|item| item["fixture"] == fixture)
        .expect("qualified encrypted fixture");
    family["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|member| {
            let encoded = fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/alzeg/expected/encrypted")
                    .join(member["expected"].as_str().expect("expected path")),
            )
            .expect("independent expected envelope");
            let mut bytes = Vec::new();
            flate2::read::GzDecoder::new(encoded.as_slice())
                .read_to_end(&mut bytes)
                .expect("independent expected bytes");
            (member["name"].as_str().expect("name").to_owned(), bytes)
        })
        .collect()
}
fn encrypted_exact(fixture: &str) {
    let (_owned, source) = owned_source(fixture);
    let expected = encrypted_expected(fixture);
    let (listed, catalog, output) = worker(&source, "open");
    assert_eq!(listed["op"], "complete");
    assert_eq!(catalog.len(), expected.len());
    assert!(output.is_empty());
    for entry in &catalog {
        let name = entry["name"].as_str().expect("name");
        assert_eq!(entry["size"], expected[name].len() as u64);
        assert_eq!(entry["encrypted"], name != "empty.txt");
    }
    for operation in ["test", "extract"] {
        let (terminal, _, output) = worker_options(
            &source,
            operation,
            |request| request.password = Some("test1234".into()),
            Effect::None,
        );
        assert_eq!(
            terminal["op"], "complete",
            "{fixture} {operation}: {terminal}"
        );
        if operation == "test" {
            assert_eq!(terminal["files_verified"], expected.len() as u64);
            assert_eq!(
                terminal["bytes_verified"],
                expected
                    .values()
                    .map(|bytes| bytes.len() as u64)
                    .sum::<u64>()
            );
            assert!(output.is_empty());
        } else {
            assert_eq!(
                output, expected,
                "every decrypted byte equals independent oracle"
            );
        }
    }
    fs::remove_file(source).expect("encrypted reader released after actual exit");
}
#[test]
fn ref19_alz_encrypted_stored_exact() {
    encrypted_exact("enc_store.alz");
}
#[test]
fn ref19_alz_encrypted_deflate_exact() {
    encrypted_exact("enc_deflate.alz");
}
#[test]
fn ref19_alz_encrypted_zip20_names_and_empty_exact() {
    encrypted_exact("zip20.alz");
}
#[test]
fn ref19_alz_password_and_damaged_ciphertext_are_typed() {
    for fixture in ["enc_store.alz", "enc_deflate.alz"] {
        let (_owned, source) = owned_source(fixture);
        for operation in ["test", "extract"] {
            for (secret, code) in [
                (None, "PasswordRequired"),
                (Some("wrong-alz-password"), "AuthenticationOrDataError"),
                (Some("🦀"), "AuthenticationOrDataError"),
            ] {
                let (terminal, _, output) = worker_options(
                    &source,
                    operation,
                    |request| request.password = secret.map(str::to_owned),
                    Effect::None,
                );
                assert_eq!(
                    terminal["error_code"], code,
                    "{fixture} {operation}: {terminal}"
                );
                assert!(output.is_empty(), "no unverified output");
            }
        }
        let original = fs::read(&source).expect("owned sample");
        let header = if fixture == "enc_store.alz" { 39 } else { 36 };
        for offset in [header + 11, header + 12] {
            let mut damaged = original.clone();
            damaged[offset] ^= 0x80;
            fs::write(&source, damaged).expect("owned ciphertext mutation");
            assert_eq!(
                worker(&source, "open").0["op"],
                "complete",
                "clear catalog does not authenticate payload"
            );
            for operation in ["test", "extract"] {
                let (terminal, _, output) = worker_options(
                    &source,
                    operation,
                    |request| request.password = Some("test1234".into()),
                    Effect::None,
                );
                assert_eq!(
                    terminal["error_code"], "AuthenticationOrDataError",
                    "damaged {fixture} {operation}: {terminal}"
                );
                assert!(output.is_empty());
            }
        }
        fs::remove_file(source).expect("failed reader released");
    }
}
#[test]
fn ref19_alz_original_password_session_prepares_before_output() {
    for operation in ["test", "extract"] {
        for effect in [
            Effect::PasswordCorrect,
            Effect::PasswordWrong,
            Effect::PasswordCancel,
        ] {
            let (_owned, source) = owned_source("enc_deflate.alz");
            let (terminal, _, output) = worker_options(
                &source,
                operation,
                |request| {
                    request.password_interaction = Some(true);
                    request.job_execution = Some(true);
                },
                effect,
            );
            let states = terminal["password_states"]
                .as_array()
                .expect("real password states");
            assert_eq!(
                states
                    .iter()
                    .filter(|state| state["state"] == "awaiting_password")
                    .count(),
                1,
                "one original ticket; ambiguous failure never retries"
            );
            if effect == Effect::PasswordCorrect {
                assert_eq!(terminal["op"], "complete", "{terminal}");
                assert_eq!(
                    states
                        .iter()
                        .filter(|state| state["state"] == "extracting")
                        .count(),
                    usize::from(operation == "extract")
                );
                if operation == "extract" {
                    assert_eq!(output, encrypted_expected("enc_deflate.alz"));
                } else {
                    assert!(output.is_empty());
                }
            } else {
                assert_eq!(
                    terminal["error_code"],
                    if effect == Effect::PasswordCancel {
                        "Cancelled"
                    } else {
                        "AuthenticationOrDataError"
                    },
                    "{terminal}"
                );
                assert!(output.is_empty());
                assert!(!states.iter().any(|state| state["state"] == "extracting"));
            }
            fs::remove_file(source).expect("password wait releases actual child/source");
        }
    }
}
#[test]
fn ref19_alz_encrypted_cancel_and_native_failure_keep_first_outcome() {
    for (effect, code) in [
        (Effect::CancelFirstWrite, "Cancelled"),
        (Effect::FailFirstWrite, "ResourceLimit"),
    ] {
        let (_owned, source) = owned_source("enc_store.alz");
        let (terminal, _, output) = worker_options(
            &source,
            "extract",
            |request| request.password = Some("test1234".into()),
            effect,
        );
        assert_eq!(terminal["error_code"], code);
        assert!(output.is_empty());
        fs::remove_file(source).expect("actual failure cleanup");
    }
}

#[test]
fn ref19_egg_two_blocks_keep_cipher_and_authentication_state() {
    let fixtures = [
        "spec_egg_zipcrypto_two_blocks.egg",
        "spec_egg_aes128_two_blocks.egg",
        "spec_egg_aes256_two_blocks.egg",
        "spec_egg_lea128_two_blocks.egg",
        "spec_egg_lea256_two_blocks.egg",
    ];
    egg_password_exact(&fixtures);
    for fixture in fixtures {
        let (_owned, source) = owned_source(fixture);
        let family = egg_password_family(fixture);
        let offset = family["members"][0]["blocks"][1]["offset"]
            .as_u64()
            .expect("second block") as usize;
        let mut damaged = fs::read(&source).expect("owned source");
        damaged[offset] ^= 0x80;
        fs::write(&source, damaged).expect("owned second-block mutation");
        for operation in ["test", "extract"] {
            let (terminal, _, output) = worker_options(
                &source,
                operation,
                |request| request.password = Some("test1234".into()),
                Effect::None,
            );
            assert_eq!(
                terminal["error_code"], "AuthenticationOrDataError",
                "second-block authentication {fixture}: {terminal}"
            );
            assert!(
                output.is_empty(),
                "first block cannot commit an incomplete member"
            );
        }
        fs::remove_file(source).expect("multi-block crypto actually released");
    }
}

fn egg_password_manifest() -> Value {
    serde_json::from_str(include_str!(
        "fixtures/alzeg/expected/egg-encrypted/manifest.json"
    ))
    .expect("independent EGG oracle")
}
fn egg_password_family(fixture: &str) -> Value {
    egg_password_manifest()["fixtures"]
        .as_array()
        .expect("fixtures")
        .iter()
        .find(|family| family["fixture"] == fixture)
        .expect("qualified EGG sample")
        .clone()
}
fn egg_password_expected(fixture: &str) -> Payloads {
    egg_password_family(fixture)["members"]
        .as_array()
        .expect("members")
        .iter()
        .filter(|member| member["kind"] == "file")
        .map(|member| {
            let encoded = fs::read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/alzeg/expected/egg-encrypted")
                    .join(member["expected"].as_str().expect("expected")),
            )
            .expect("oracle envelope");
            let mut bytes = Vec::new();
            flate2::read::GzDecoder::new(encoded.as_slice())
                .read_to_end(&mut bytes)
                .expect("oracle bytes");
            (member["name"].as_str().expect("name").to_owned(), bytes)
        })
        .collect()
}
fn egg_password_exact(fixtures: &[&str]) {
    for &fixture in fixtures {
        let (_owned, source) = owned_source(fixture);
        let family = egg_password_family(fixture);
        let expected = egg_password_expected(fixture);
        let (terminal, entries, outputs) = worker(&source, "open");
        assert_eq!(terminal["op"], "complete", "{fixture}: {terminal}");
        assert_eq!(
            entries.len(),
            family["members"].as_array().expect("members").len()
        );
        assert!(outputs.is_empty());
        for (entry, member) in entries
            .iter()
            .zip(family["members"].as_array().expect("members"))
        {
            assert_eq!(entry["name"], member["name"]);
            assert_eq!(entry["encrypted"], member["encrypted"]);
            assert_eq!(entry["size"], member["size"]);
        }
        for operation in ["test", "extract"] {
            let (terminal, _, outputs) = worker_options(
                &source,
                operation,
                |request| request.password = Some("test1234".into()),
                Effect::None,
            );
            assert_eq!(
                terminal["op"], "complete",
                "{fixture} {operation}: {terminal}"
            );
            if operation == "test" {
                assert_eq!(terminal["files_verified"], family["files"]);
                assert_eq!(terminal["bytes_verified"], family["bytes"]);
                assert!(outputs.is_empty());
            } else {
                assert_eq!(outputs, expected, "every independent EGG byte: {fixture}");
            }
        }
        fs::remove_file(source).expect("actual encrypted EGG reader release");
    }
}
#[test]
fn ref19_egg_zipcrypto_exact() {
    egg_password_exact(&["enc_zip20.egg", "zip20.egg", "oversized_encrypt_zip20.egg"]);
}
#[test]
fn ref19_egg_aes128_exact() {
    egg_password_exact(&[
        "enc_aes128.egg",
        "aes128.egg",
        "oversized_encrypt_aes128.egg",
    ]);
}
#[test]
fn ref19_egg_aes256_exact() {
    egg_password_exact(&[
        "enc_aes256.egg",
        "aes256.egg",
        "oversized_encrypt_aes256.egg",
    ]);
}
#[test]
fn ref19_egg_lea128_exact() {
    egg_password_exact(&["lea128.egg"]);
}
#[test]
fn ref19_egg_lea256_exact() {
    egg_password_exact(&["lea256.egg"]);
}
#[test]
fn ref19_egg_encrypted_bzip2_exact() {
    egg_password_exact(&[
        "enc_zip20_bzip2.egg",
        "enc_aes256_bzip2.egg",
        "enc_lea_bzip2.egg",
    ]);
}
#[test]
fn ref19_egg_password_ciphertext_and_authentication_are_typed() {
    for fixture in [
        "enc_zip20.egg",
        "enc_aes128.egg",
        "enc_aes256.egg",
        "lea128.egg",
        "lea256.egg",
    ] {
        let (_owned, source) = owned_source(fixture);
        for operation in ["test", "extract"] {
            for (password, code) in [
                (None, "PasswordRequired"),
                (Some("wrong-alz-password"), "AuthenticationOrDataError"),
            ] {
                let (terminal, _, output) = worker_options(
                    &source,
                    operation,
                    |request| request.password = password.map(str::to_owned),
                    Effect::None,
                );
                assert_eq!(
                    terminal["error_code"], code,
                    "{fixture} {operation}: {terminal}"
                );
                assert!(output.is_empty());
            }
        }
        let member = egg_password_family(fixture)["members"]
            .as_array()
            .expect("members")
            .iter()
            .find(|m| m["encrypted"] == true)
            .expect("encrypted member")
            .clone();
        let start = member["encrypt_info_offset"].as_u64().expect("offset") as usize;
        let size = member["encrypt_info_size"].as_u64().expect("size") as usize;
        let ciphertext = member["blocks"][0]["offset"].as_u64().expect("ciphertext") as usize;
        let verifier = start
            + if member["encryption"] == 0 {
                12
            } else if matches!(member["encryption"].as_u64(), Some(1 | 5)) {
                9
            } else {
                17
            };
        let original = fs::read(&source).expect("source");
        for offset in [verifier, start + size - 1, ciphertext] {
            let mut damaged = original.clone();
            damaged[offset] ^= 0x80;
            fs::write(&source, damaged).expect("owned ciphertext or tag mutation");
            assert_eq!(
                worker(&source, "open").0["op"],
                "complete",
                "clear Browse is not authentication"
            );
            for operation in ["test", "extract"] {
                let (terminal, _, output) = worker_options(
                    &source,
                    operation,
                    |request| request.password = Some("test1234".into()),
                    Effect::None,
                );
                assert_eq!(
                    terminal["error_code"], "AuthenticationOrDataError",
                    "damaged {fixture}: {terminal}"
                );
                assert!(output.is_empty(), "no unauthenticated commit");
            }
        }
        fs::remove_file(source).expect("failed crypto releases source");
    }
}
#[test]
fn ref19_egg_original_password_session_prepares_before_output() {
    for fixture in [
        "enc_zip20.egg",
        "enc_aes128.egg",
        "enc_aes256.egg",
        "lea128.egg",
        "lea256.egg",
    ] {
        for operation in ["test", "extract"] {
            for effect in [
                Effect::PasswordCorrect,
                Effect::PasswordWrong,
                Effect::PasswordCancel,
            ] {
                let (_owned, source) = owned_source(fixture);
                let (terminal, _, output) = worker_options(
                    &source,
                    operation,
                    |request| {
                        request.password_interaction = Some(true);
                        request.job_execution = Some(true);
                    },
                    effect,
                );
                let states = terminal["password_states"]
                    .as_array()
                    .expect("real password states");
                assert_eq!(
                    states
                        .iter()
                        .filter(|s| s["state"] == "awaiting_password")
                        .count(),
                    1,
                    "one ticket; ambiguous authentication failure cannot automatically retry"
                );
                if effect == Effect::PasswordCorrect {
                    assert_eq!(terminal["op"], "complete", "{fixture}: {terminal}");
                    assert_eq!(
                        states.iter().filter(|s| s["state"] == "extracting").count(),
                        usize::from(operation == "extract")
                    );
                    if operation == "extract" {
                        assert_eq!(output, egg_password_expected(fixture));
                    } else {
                        assert!(output.is_empty());
                    }
                } else {
                    assert_eq!(
                        terminal["error_code"],
                        if effect == Effect::PasswordCancel {
                            "Cancelled"
                        } else {
                            "AuthenticationOrDataError"
                        }
                    );
                    assert!(output.is_empty());
                    assert!(!states.iter().any(|s| s["state"] == "extracting"));
                }
                fs::remove_file(source).expect("actual child and password cleanup");
            }
        }
    }
}
#[test]
fn ref19_egg_encrypted_cancel_native_failure_and_working_limits() {
    for fixture in [
        "enc_zip20.egg",
        "enc_aes128.egg",
        "enc_aes256.egg",
        "lea128.egg",
        "lea256.egg",
    ] {
        for (effect, code, working) in [
            (Effect::CancelFirstWrite, "Cancelled", None),
            (Effect::FailFirstWrite, "ResourceLimit", None),
            (Effect::None, "ResourceLimit", Some(1)),
        ] {
            let (_owned, source) = owned_source(fixture);
            let (terminal, _, outputs) = worker_options(
                &source,
                "extract",
                |request| {
                    request.password = Some("test1234".into());
                    if let Some(limit) = working {
                        request.limits.max_working_bytes = limit;
                    }
                },
                effect,
            );
            assert_eq!(terminal["error_code"], code, "{fixture}: {terminal}");
            assert!(outputs.is_empty());
            fs::remove_file(source).expect("actual failed native/crypto cleanup");
        }
    }
}
#[test]
fn ref19_egg_encryption_header_boundaries_are_not_empty_success() {
    for fixture in ["enc_zip20.egg", "enc_aes128.egg", "enc_aes256.egg"] {
        let (_owned, source) = owned_source(fixture);
        let member = &egg_password_family(fixture)["members"][0];
        let offset = member["encrypt_info_offset"].as_u64().expect("info") as usize;
        let size = member["encrypt_info_size"].as_u64().expect("size") as usize;
        let original = fs::read(&source).expect("source");
        for (position, value, code) in [
            (offset, 7, "UnsupportedEncryption"),
            (offset - 2, 0, "CorruptArchive"),
            (offset + size, 0, "CorruptArchive"),
        ] {
            let mut damaged = original.clone();
            damaged[position] = value;
            fs::write(&source, damaged).expect("owned encryption header mutation");
            for operation in ["open", "test", "extract"] {
                let (terminal, _, output) = worker_options(
                    &source,
                    operation,
                    |request| request.password = Some("test1234".into()),
                    Effect::None,
                );
                assert_eq!(
                    terminal["error_code"], code,
                    "{fixture} {operation}: {terminal}"
                );
                assert!(output.is_empty());
            }
        }
        fs::remove_file(source).expect("invalid crypto header releases source");
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Effect {
    None,
    CancelFirstWrite,
    FailFirstWrite,
    ChangeContinuation,
    ChangeAfterLastCommit,
    PasswordCorrect,
    PasswordWrong,
    PasswordCancel,
}

struct OwnedChild(Child);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

type Payloads = BTreeMap<String, Vec<u8>>;
fn worker(path: &Path, operation: &str) -> (Value, Vec<Value>, Payloads) {
    worker_options(path, operation, |_| {}, Effect::None)
}
fn worker_options(
    path: &Path,
    operation: &str,
    configure: impl FnOnce(&mut WorkerRequest),
    effect: Effect,
) -> (Value, Vec<Value>, Payloads) {
    let mut owned = OwnedChild(
        Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
            .arg("--archive-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("original isolated worker"),
    );
    let child = &mut owned.0;
    let id = Uuid::new_v4();
    let mut request = WorkerRequest::open(id, path.to_string_lossy().into_owned());
    request.version = 2;
    request.operation = operation.into();
    request.bulk_transfer = (operation == "extract").then_some(1);
    configure(&mut request);
    write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
    let mut catalog = Vec::new();
    let mut incomplete = BTreeMap::<Uuid, (String, Vec<u8>)>::new();
    let mut held = BTreeMap::<Uuid, tempfile::NamedTempFile>::new();
    let mut completed = Payloads::new();
    let mut triggered = false;
    let mut shortfall = None;
    let mut password_states = Vec::new();
    let terminal = loop {
        let frame: Value = read_frame(child.stdout.as_mut().expect("stdout"))
            .expect("bounded private frame")
            .expect("terminal before EOF");
        assert_eq!(frame["operation_id"], id.to_string());
        match frame["op"].as_str().expect("operation") {
            "entry_batch" => catalog.extend(frame["entries"].as_array().expect("entries").clone()),
            "native_call" => {
                let call = &frame["native"];
                let action = call["action"].as_str().expect("native action");
                let mut token = None;
                let write_failed = triggered
                    && effect == Effect::FailFirstWrite
                    && !matches!(action, "write" | "abort");
                match action {
                    _ if write_failed => {}
                    "source_acquire"
                    | "source_validated"
                    | "source_precommit"
                    | "source_stamp"
                    | "output_ready"
                    | "destination_preflight"
                    | "directory"
                    | "directory_new"
                    | "directory_metadata"
                    | "file_metadata" => {}
                    "begin" => {
                        let key = Uuid::new_v4();
                        assert!(
                            !(request.output_readmission.is_some()
                                && request.limits.max_output_bytes == 1),
                            "deficient output must not begin any native file"
                        );
                        incomplete.insert(
                            key,
                            (call["path"].as_str().expect("name").into(), Vec::new()),
                        );
                        held.insert(
                            key,
                            tempfile::NamedTempFile::new_in(
                                path.parent().expect("owned directory"),
                            )
                            .expect("owned native-peer partial"),
                        );
                        token = Some(key);
                    }
                    "write" => {
                        let key =
                            Uuid::parse_str(call["token"].as_str().expect("token")).expect("UUID");
                        let bytes = call["data"].as_array().expect("bounded bytes");
                        assert!(bytes.len() <= 65536);
                        incomplete.get_mut(&key).expect("owned output").1.extend(
                            bytes.iter().map(|value| {
                                u8::try_from(value.as_u64().expect("byte")).expect("u8")
                            }),
                        );
                        let bytes = &incomplete.get(&key).expect("owned bytes").1;
                        let file = held.get_mut(&key).expect("held native-peer file");
                        std::io::Write::write_all(
                            file,
                            &bytes
                                [bytes.len() - call["length"].as_u64().expect("length") as usize..],
                        )
                        .expect("actual peer file write");
                    }
                    "commit" => {
                        let key =
                            Uuid::parse_str(call["token"].as_str().expect("token")).expect("UUID");
                        let (name, bytes) = incomplete.remove(&key).expect("owned output");
                        let file = held.remove(&key).expect("held partial");
                        let readback = fs::read(file.path()).expect("peer file readback");
                        assert_eq!(
                            readback, bytes,
                            "actual file bytes before controlled publication"
                        );
                        file.close().expect("native-peer temporary released");
                        assert!(completed.insert(name, bytes).is_none(), "no replacement");
                        if effect == Effect::ChangeAfterLastCommit && completed.len() == 4 {
                            let continuation =
                                request.volume_paths.get(1).expect("owned continuation");
                            let mut bytes = fs::read(continuation).expect("continuation");
                            bytes[100] ^= 1;
                            fs::write(continuation, bytes)
                                .expect("source change after last actual commit");
                            triggered = true;
                        }
                    }
                    "abort" => {
                        let key =
                            Uuid::parse_str(call["token"].as_str().expect("token")).expect("UUID");
                        incomplete.remove(&key);
                        if let Some(file) = held.remove(&key) {
                            file.close().expect("aborted native-peer file removed");
                        }
                    }
                    _ => panic!("unexpected original native action {action}"),
                }
                let mut reply = json!({"version":1,"operation_id":id,"op":"native_result","call_id":call["call_id"],"success":true,"token":token});
                if action == "source_acquire" && !request.volume_paths.is_empty() {
                    reply["read_data"] = json!(
                        json!({"path": request.path, "volume_paths": request.volume_paths})
                            .to_string()
                    );
                }
                if action == "source_stamp" {
                    reply["read_data"] = json!("v1:ref19-owned-source");
                }
                if action == "write" && effect != Effect::None && !triggered {
                    triggered = true;
                    if effect == Effect::CancelFirstWrite {
                        write_frame(child.stdin.as_mut().expect("stdin"), &reply)
                            .expect("consume pipelined peer write");
                        write_frame(
                            child.stdin.as_mut().expect("stdin"),
                            &json!({"version":1,"operation_id":id,"op":"cancel"}),
                        )
                        .expect("real cancellation frame");
                        continue;
                    }
                    if effect == Effect::ChangeContinuation {
                        let continuation =
                            request.volume_paths.get(1).expect("supplied continuation");
                        let mut bytes = fs::read(continuation).expect("owned continuation");
                        bytes[100] ^= 1;
                        fs::write(continuation, bytes)
                            .expect("source changes after encoded spool read");
                    }
                }
                if triggered
                    && effect == Effect::FailFirstWrite
                    && !matches!(action, "write" | "abort")
                {
                    reply["success"] = json!(false);
                    reply["error_code"] = json!("resource_limit");
                }
                if let Err(error) = write_frame(child.stdin.as_mut().expect("stdin"), &reply) {
                    assert!(
                        effect == Effect::CancelFirstWrite
                            && triggered
                            && error.kind() == std::io::ErrorKind::BrokenPipe,
                        "native reply before acknowledged cancellation/actual child exit: {error}"
                    );
                }
            }
            "password_state" => {
                let state = frame["password_state"].clone();
                if state["state"] == "awaiting_password" {
                    assert!(
                        incomplete.is_empty() && completed.is_empty() && held.is_empty(),
                        "password preparation has zero output"
                    );
                    assert_eq!(state["error_code"], "PasswordRequired");
                    let password = match effect {
                        Effect::PasswordCorrect => Some("test1234"),
                        Effect::PasswordWrong => Some("wrong-alz-password"),
                        Effect::PasswordCancel => None,
                        _ => panic!("unexpected original password prompt"),
                    };
                    write_frame(child.stdin.as_mut().expect("stdin"), &json!({"version":1,"operation_id":id,"op":"password_reply","password_reply":{"ticket":state["ticket"],"password":password}})).expect("original ticketed password answer");
                    triggered = true;
                }
                password_states.push(state);
            }
            "output_budget_shortfall" => {
                shortfall = Some(frame["output_budget_shortfall"].clone());
            }
            "complete" | "failure" => break frame,
            _ => {}
        }
    };
    drop(child.stdin.take());
    let exited = child.wait().expect("actual worker exit");
    let mut stderr = Vec::new();
    child
        .stderr
        .as_mut()
        .expect("stderr")
        .read_to_end(&mut stderr)
        .expect("stderr drain");
    assert!(exited.success() && stderr.is_empty(), "clean worker exit");
    if effect == Effect::CancelFirstWrite && terminal["error_code"] == "Cancelled" {
        for file in held.into_values() {
            let partial_path = file.path().to_owned();
            file.close()
                .expect("parent cleanup after actual child/pipe exit");
            assert!(
                !partial_path.exists(),
                "cancelled native-peer partial actually removed"
            );
        }
        incomplete.clear();
    } else {
        assert!(held.is_empty(), "all native-peer file handles released");
    }
    assert!(incomplete.is_empty(), "no incomplete output after terminal");
    assert!(
        effect == Effect::None || triggered,
        "fault/cancel reached real native write"
    );
    let mut terminal = terminal;
    if let Some(shortfall) = shortfall {
        terminal["output_budget_shortfall"] = shortfall;
    }
    terminal["password_states"] = json!(password_states);
    (terminal, catalog, completed)
}

fn dlz_expected(fixture: &str) -> Payloads {
    let manifest: Value =
        serde_json::from_str(include_str!("fixtures/alzeg/expected/dlz/manifest.json"))
            .expect("independent DLZ oracle manifest");
    let family = manifest["fixtures"]
        .as_array()
        .expect("fixtures")
        .iter()
        .find(|item| item["fixture"] == fixture)
        .expect("qualified DLZ fixture");
    family["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|member| {
            let file = fs::File::open(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/alzeg/expected/dlz")
                    .join(member["expected"].as_str().expect("expected")),
            )
            .expect("oracle bytes");
            let mut bytes = Vec::new();
            flate2::read::GzDecoder::new(file)
                .read_to_end(&mut bytes)
                .expect("oracle envelope");
            (member["name"].as_str().expect("name").to_owned(), bytes)
        })
        .collect()
}
#[test]
fn ref19_alz_dlz_genuine_bzip2_exact() {
    ordinary("bzip2.alz", dlz_expected("bzip2.alz"));
}
#[test]
fn ref19_alz_dlz_genuine_text_exact() {
    ordinary("bzip2_dlz.alz", dlz_expected("bzip2_dlz.alz"));
}
#[test]
fn ref19_alz_dlz_specification_multiblock_exact() {
    ordinary(
        "spec_alz_dlz_multiblock.alz",
        dlz_expected("spec_alz_dlz_multiblock.alz"),
    );
}
fn set_dlz_bits(data: &mut [u8], start: usize, count: usize, value: u32) {
    for bit in 0..count {
        let position = start + bit;
        let mask = 1 << (7 - position % 8);
        data[position / 8] =
            (data[position / 8] & !mask) | (u8::from(value & (1 << (count - bit - 1)) != 0) * mask);
    }
}
#[test]
fn ref19_alz_dlz_malformed_bits_end_consumption_and_crc_never_commit() {
    let (_owned, source) = owned_source("spec_alz_dlz_multiblock.alz");
    let original = fs::read(&source).expect("owned sample");
    let at = 49usize;
    let mut cases = Vec::new();
    for offset in [at, 23] {
        let mut data = original.clone();
        data[offset] ^= 0x80;
        cases.push(data);
    }
    let mut data = original.clone();
    data[at + 4..at + 7].fill(0xff);
    cases.push(data);
    let mut data = original.clone();
    data[at + 7..at + 9].fill(0);
    cases.push(data);
    let used = u16::from_be_bytes([original[at + 7], original[at + 8]]);
    let group_bit = (at + 9) * 8 + used.count_ones() as usize * 16;
    let mut data = original.clone();
    set_dlz_bits(&mut data, group_bit, 3, 0);
    cases.push(data);
    let mut data = original.clone();
    set_dlz_bits(&mut data, group_bit + 3, 15, 0);
    cases.push(data);
    let mut data = original.clone();
    set_dlz_bits(&mut data, at * 8 + 16024, 32, 0);
    cases.push(data);
    let mut data = original.clone();
    set_dlz_bits(&mut data, at * 8 + 44855, 32, 0);
    cases.push(data);
    let packed = u32::from_le_bytes(original[27..31].try_into().expect("packed"));
    let mut data = original.clone();
    data.remove(at + packed as usize - 1);
    data[27..31].copy_from_slice(&(packed - 1).to_le_bytes());
    cases.push(data);
    let mut data = original.clone();
    data.insert(at + packed as usize, 0);
    data[27..31].copy_from_slice(&(packed + 1).to_le_bytes());
    cases.push(data);
    for (index, data) in cases.into_iter().enumerate() {
        fs::write(&source, data).expect("owned damaged stream");
        assert_eq!(
            worker(&source, "open").0["op"],
            "complete",
            "clear container {index}"
        );
        for operation in ["test", "extract"] {
            let (terminal, _, outputs) = worker(&source, operation);
            assert_eq!(
                terminal["error_code"], "CorruptArchive",
                "case {index}/{operation}: {terminal}"
            );
            assert!(outputs.is_empty(), "damaged member never commits");
        }
    }
    fs::remove_file(source).expect("source released after actual child drain");
}
#[test]
fn ref19_alz_dlz_finite_workspace_and_original_output_limits() {
    let fixture = "spec_alz_dlz_multiblock.alz";
    let (_owned, source) = owned_source(fixture);
    for boundary in ["working", "entry", "total"] {
        let (terminal, _, outputs) = worker_options(
            &source,
            "extract",
            |request| match boundary {
                "working" => request.limits.max_working_bytes = (5 << 20) - 1,
                "entry" => request.limits.max_entry_bytes = 1,
                "total" => request.limits.max_output_bytes = 1,
                _ => unreachable!(),
            },
            Effect::None,
        );
        assert_eq!(
            terminal["error_code"], "ResourceLimit",
            "{boundary}: {terminal}"
        );
        assert!(outputs.is_empty());
    }
    let (terminal, _, outputs) = worker_options(
        &source,
        "extract",
        |request| {
            request.limits.max_working_bytes = 5 << 20;
        },
        Effect::None,
    );
    assert_eq!(
        terminal["op"], "complete",
        "fixed workspace admits all blocks: {terminal}"
    );
    assert_eq!(outputs, dlz_expected(fixture));
    fs::remove_file(source).expect("source released");
}
#[test]
fn ref19_alz_dlz_cancel_and_native_first_failure_drain() {
    for (effect, code) in [
        (Effect::CancelFirstWrite, "Cancelled"),
        (Effect::FailFirstWrite, "ResourceLimit"),
    ] {
        let (_owned, source) = owned_source("spec_alz_dlz_multiblock.alz");
        let (terminal, _, outputs) = worker_options(&source, "extract", |_| {}, effect);
        assert_eq!(terminal["error_code"], code, "{terminal}");
        assert!(
            outputs.is_empty(),
            "no unverified output after real stop or native failure"
        );
        fs::remove_file(source).expect("source released after actual child drain");
    }
}

fn solid_manifest() -> Value {
    serde_json::from_str(include_str!("fixtures/alzeg/expected/solid/manifest.json"))
        .expect("independent solid manifest")
}
fn solid_expected(fixture: &str) -> Payloads {
    let manifest = solid_manifest();
    let sample = manifest["fixtures"]
        .as_array()
        .expect("fixtures")
        .iter()
        .find(|item| item["fixture"] == fixture)
        .expect("sample");
    sample["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|member| {
            let file = fs::File::open(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/alzeg/expected/solid")
                    .join(member["expected"].as_str().expect("expected")),
            )
            .expect("independent bytes");
            let mut bytes = Vec::new();
            flate2::read::GzDecoder::new(file)
                .read_to_end(&mut bytes)
                .expect("oracle envelope");
            (member["name"].as_str().expect("name").to_owned(), bytes)
        })
        .collect()
}
#[test]
fn ref19_egg_solid_genuine_deflate_exact() {
    ordinary("solid.egg", solid_expected("solid.egg"));
}
#[test]
fn ref19_egg_solid_three_blocks_cross_file_spans_exact() {
    ordinary(
        "spec_egg_solid_deflate_three_blocks.egg",
        solid_expected("spec_egg_solid_deflate_three_blocks.egg"),
    );
}
#[test]
fn ref19_egg_solid_verifies_late_crc_and_exact_stream_before_first_commit() {
    let fixture = "spec_egg_solid_deflate_three_blocks.egg";
    let (_owned, source) = owned_source(fixture);
    let original = fs::read(&source).expect("source");
    let manifest = solid_manifest();
    let last = &manifest["fixtures"][1]["blocks"][2];
    let header = last["header"].as_u64().expect("header") as usize;
    let offset = last["offset"].as_u64().expect("offset") as usize;
    let packed = last["packed"].as_u64().expect("packed") as usize;
    for mode in ["crc", "payload", "unused", "expanded"] {
        let mut data = original.clone();
        match mode {
            "crc" => data[header + 14] ^= 1,
            "payload" => data[offset + packed / 2] ^= 0x80,
            "unused" => {
                data.insert(offset + packed, 0);
                data[header + 10..header + 14]
                    .copy_from_slice(&((packed + 1) as u32).to_le_bytes());
            }
            "expanded" => data[header + 6] ^= 1,
            _ => unreachable!(),
        }
        fs::write(&source, data).expect("owned mutation");
        for operation in ["test", "extract"] {
            let (terminal, _, outputs) = worker(&source, operation);
            assert_eq!(
                terminal["error_code"], "CorruptArchive",
                "{mode}/{operation}: {terminal}"
            );
            assert!(
                outputs.is_empty(),
                "late solid failure cannot publish an earlier file"
            );
        }
    }
    fs::remove_file(source).expect("actual source release");
}
#[test]
fn ref19_egg_solid_finite_workspace_and_original_output_limits() {
    let fixture = "spec_egg_solid_deflate_three_blocks.egg";
    let (_owned, source) = owned_source(fixture);
    for boundary in ["working", "entry", "total"] {
        let (terminal, _, outputs) = worker_options(
            &source,
            "extract",
            |request| match boundary {
                "working" => request.limits.max_working_bytes = (256 << 10) - 1,
                "entry" => request.limits.max_entry_bytes = 1,
                "total" => request.limits.max_output_bytes = 1,
                _ => unreachable!(),
            },
            Effect::None,
        );
        assert_eq!(
            terminal["error_code"], "ResourceLimit",
            "{boundary}: {terminal}"
        );
        assert!(outputs.is_empty());
    }
    let (terminal, _, outputs) = worker_options(
        &source,
        "extract",
        |request| request.limits.max_working_bytes = 256 << 10,
        Effect::None,
    );
    assert_eq!(
        terminal["op"], "complete",
        "bounded continuous decoder: {terminal}"
    );
    assert_eq!(outputs, solid_expected(fixture));
    fs::remove_file(source).expect("actual source release");
}
#[test]
fn ref19_egg_solid_cancel_and_native_first_failure_drain() {
    for (effect, code) in [
        (Effect::CancelFirstWrite, "Cancelled"),
        (Effect::FailFirstWrite, "ResourceLimit"),
    ] {
        let (_owned, source) = owned_source("spec_egg_solid_deflate_three_blocks.egg");
        let (terminal, _, outputs) = worker_options(&source, "extract", |_| {}, effect);
        assert_eq!(terminal["error_code"], code, "{terminal}");
        assert!(
            outputs.is_empty(),
            "first file remains uncommitted after stop/failure"
        );
        fs::remove_file(source).expect("source released after actual child drain");
    }
}

fn azo_expected(fixture: &str) -> Payloads {
    let manifest: Value =
        serde_json::from_str(include_str!("fixtures/alzeg/expected/azo/manifest.json"))
            .expect("independent AZO oracle");
    let sample = manifest["fixtures"]
        .as_array()
        .expect("fixtures")
        .iter()
        .find(|item| item["fixture"] == fixture)
        .expect("qualified sample");
    sample["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|member| {
            let file = fs::File::open(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/alzeg/expected/azo")
                    .join(member["expected"].as_str().expect("expected")),
            )
            .expect("independent bytes");
            let mut bytes = Vec::new();
            flate2::read::GzDecoder::new(file)
                .read_to_end(&mut bytes)
                .expect("oracle envelope");
            (member["name"].as_str().expect("name").to_owned(), bytes)
        })
        .collect()
}
#[test]
fn ref19_egg_azo_genuine_exact() {
    ordinary("azo.egg", azo_expected("azo.egg"));
}
#[test]
fn ref19_egg_azo_genuine_optimal_mixed_exact() {
    assert_eq!(azo_expected("optimal5.egg"), five_members());
    ordinary("optimal5.egg", azo_expected("optimal5.egg"));
}
#[test]
fn ref19_egg_azo_multiblock_history_and_x86_exact() {
    ordinary(
        "spec_egg_azo_multiblock.egg",
        azo_expected("spec_egg_azo_multiblock.egg"),
    );
}
#[test]
fn ref19_egg_azo_malformed_range_sizes_terminal_and_consumption() {
    let (_owned, source) = owned_source("azo.egg");
    let original = fs::read(&source).expect("source");
    let mut cases = Vec::new();
    for (offset, code) in [
        (94, "UnsupportedMethod"),
        (95, "UnsupportedMethod"),
        (104, "CorruptArchive"),
        (86, "CorruptArchive"),
        (146, "CorruptArchive"),
    ] {
        let mut data = original.clone();
        data[offset] ^= 0x80;
        cases.push((data, code));
    }
    let mut data = original.clone();
    data[108..112].fill(0);
    cases.push((data, "CorruptArchive"));
    for (expanded, packed) in [(0u32, 27u32), (1602, 0), (1, 27)] {
        let mut data = original.clone();
        data[96..100].copy_from_slice(&expanded.to_be_bytes());
        data[100..104].copy_from_slice(&packed.to_be_bytes());
        data[104..108].copy_from_slice(&(expanded ^ packed).to_be_bytes());
        cases.push((data, "CorruptArchive"));
    }
    for delta in [-1i32, 8] {
        let mut data = original.clone();
        if delta < 0 {
            data.remove(134);
        } else {
            data.splice(135..135, [0; 8]);
        }
        let packed = (27 + delta) as u32;
        data[100..104].copy_from_slice(&packed.to_be_bytes());
        data[104..108].copy_from_slice(&(1602 ^ packed).to_be_bytes());
        data[82..86].copy_from_slice(&((53 + delta) as u32).to_le_bytes());
        cases.push((data, "CorruptArchive"));
    }
    let mut data = original.clone();
    data.insert(147, 0);
    data[82..86].copy_from_slice(&54u32.to_le_bytes());
    cases.push((data, "CorruptArchive"));
    for (index, (data, code)) in cases.into_iter().enumerate() {
        fs::write(&source, data).expect("owned mutation");
        assert_eq!(
            worker(&source, "open").0["op"],
            "complete",
            "clear catalog {index}"
        );
        for operation in ["test", "extract"] {
            let (terminal, _, outputs) = worker(&source, operation);
            assert_eq!(
                terminal["error_code"], code,
                "{index}/{operation}: {terminal}"
            );
            assert!(outputs.is_empty(), "unverified AZO member never commits");
        }
    }
    fs::remove_file(source).expect("source released after child drain");
}
#[test]
fn ref19_egg_azo_dictionary_credit_and_original_output_limits() {
    let fixture = "spec_egg_azo_multiblock.egg";
    let (_owned, source) = owned_source(fixture);
    for boundary in ["working", "distance", "entry", "total"] {
        let (terminal, _, outputs) = worker_options(
            &source,
            "extract",
            |request| match boundary {
                "working" => request.limits.max_working_bytes = (256 << 10) - 1,
                "distance" => request.limits.max_working_bytes = (256 << 10) + 65538,
                "entry" => request.limits.max_entry_bytes = 1,
                "total" => request.limits.max_output_bytes = 1,
                _ => unreachable!(),
            },
            Effect::None,
        );
        assert_eq!(
            terminal["error_code"], "ResourceLimit",
            "{boundary}: {terminal}"
        );
        assert!(outputs.is_empty());
    }
    let (terminal, _, outputs) = worker_options(
        &source,
        "extract",
        |request| request.limits.max_working_bytes = (256 << 10) + 65539,
        Effect::None,
    );
    assert_eq!(
        terminal["op"], "complete",
        "exact circular dictionary credit: {terminal}"
    );
    assert_eq!(outputs, azo_expected(fixture));
    fs::remove_file(source).expect("source released");
}
#[test]
fn ref19_egg_azo_cancel_and_native_first_failure_drain() {
    for (effect, code) in [
        (Effect::CancelFirstWrite, "Cancelled"),
        (Effect::FailFirstWrite, "ResourceLimit"),
    ] {
        let (_owned, source) = owned_source("spec_egg_azo_multiblock.egg");
        let (terminal, _, outputs) = worker_options(&source, "extract", |_| {}, effect);
        assert_eq!(terminal["error_code"], code, "{terminal}");
        assert!(
            outputs.is_empty(),
            "actual AZO stop/first failure leaves no committed member"
        );
        fs::remove_file(source).expect("source released after actual child drain");
    }
}

#[test]
fn ref19_egg_azo_genuine_pe_x86_exact() {
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(
        include_bytes!("fixtures/alzeg/expected/azo-pe/0.bin.gz").as_slice(),
    )
    .read_to_end(&mut bytes)
    .expect("independently decoded CC0 PE bytes; never executed");
    assert_eq!(bytes.len(), 148781);
    ordinary("azo_pe.egg", BTreeMap::from([("hello.sys".into(), bytes)]));
}

fn kinds_short_family(fixture: &str) -> Value {
    let manifest: Value = serde_json::from_str(include_str!(
        "fixtures/alzeg/expected/kinds-short/manifest.json"
    ))
    .expect("independently qualified POSIX and short-profile manifest");
    manifest["fixtures"]
        .as_array()
        .expect("fixtures")
        .iter()
        .find(|item| item["fixture"] == fixture)
        .expect("fixture")
        .clone()
}
fn kinds_short_expected(fixture: &str) -> Payloads {
    kinds_short_family(fixture)["members"]
        .as_array()
        .expect("members")
        .iter()
        .filter(|member| member["kind"] == "File")
        .map(|member| {
            let mut payload = Vec::new();
            let file = fs::File::open(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("tests/fixtures/alzeg")
                    .join(member["expected_gzip"].as_str().expect("expected")),
            )
            .expect("independent payload");
            flate2::read::GzDecoder::new(file)
                .read_to_end(&mut payload)
                .expect("gzip");
            (member["name"].as_str().expect("name").to_owned(), payload)
        })
        .collect()
}
#[test]
fn ref19_egg_genuine_posix_payload_exact() {
    ordinary("posix.egg", kinds_short_expected("posix.egg"));
}
#[test]
fn ref19_egg_kinds_do_not_downgrade_with_header_order() {
    let fixture = "spec_egg_kind_headers.egg";
    let (_owned, source) = owned_source(fixture);
    let (terminal, catalog, outputs) = worker(&source, "open");
    assert_eq!(terminal["op"], "complete", "{terminal}");
    assert!(outputs.is_empty());
    let family = kinds_short_family(fixture);
    let expected = family["members"].as_array().expect("members");
    assert_eq!(catalog.len(), expected.len());
    for (actual, member) in catalog.iter().zip(expected) {
        for key in ["name", "kind", "size"] {
            assert_eq!(
                actual[key], member[key],
                "order-independent {key}: {actual}"
            );
        }
        assert_eq!(actual["ordinal"], member["index"]);
    }
    for operation in ["test", "extract"] {
        let (terminal, _, outputs) = worker(&source, operation);
        assert_eq!(terminal["op"], "complete", "{operation}: {terminal}");
        let payloads = kinds_short_expected(fixture);
        assert_eq!(
            terminal["files_verified"], 1,
            "only regular file is decoded"
        );
        assert_eq!(terminal["bytes_verified"], family["materializable_bytes"]);
        assert_eq!(
            outputs,
            if operation == "extract" {
                payloads
            } else {
                Payloads::new()
            }
        );
    }
    fs::remove_file(source).expect("all kind readers released after child drain");
}
#[test]
fn ref19_egg_short_unauthenticated_profiles_are_typed_unsupported() {
    for method in [1, 2, 5, 6] {
        let (_owned, source) = owned_source(&format!("spec_egg_short_{method}.egg"));
        for operation in ["open", "test", "extract"] {
            for password in [None, Some("test1234")] {
                let (terminal, catalog, outputs) = worker_options(
                    &source,
                    operation,
                    |request| request.password = password.map(str::to_owned),
                    Effect::None,
                );
                assert_eq!(
                    terminal["error_code"], "UnsupportedEncryption",
                    "{method}/{operation}: {terminal}"
                );
                assert!(
                    catalog.is_empty() && outputs.is_empty(),
                    "no unsupported catalog/output success"
                );
            }
        }
        fs::remove_file(source).expect("unsupported profile releases source");
    }
}
#[test]
fn ref19_egg_short_header_invalid_end_size_and_truncation_are_corrupt() {
    for method in [1, 2, 5, 6] {
        let (_owned, source) = owned_source(&format!("spec_egg_short_{method}.egg"));
        let original = fs::read(&source).expect("source");
        let header = original
            .windows(4)
            .position(|b| b == 0x08d1470fu32.to_le_bytes())
            .expect("encrypt header");
        let end = header + 7 + if method == 1 || method == 5 { 11 } else { 19 };
        for mutation in 0..3 {
            let mut bytes = original.clone();
            match mutation {
                0 => bytes[end] ^= 1,
                1 => bytes[header + 5] = 1,
                _ => bytes.truncate(end - 1),
            }
            fs::write(&source, bytes).expect("owned short-header mutation");
            for operation in ["open", "test", "extract"] {
                let (terminal, catalog, outputs) = worker(&source, operation);
                assert_eq!(
                    terminal["error_code"], "CorruptArchive",
                    "{method}/{mutation}/{operation}: {terminal}"
                );
                assert!(catalog.is_empty() && outputs.is_empty());
            }
        }
        fs::remove_file(source).expect("invalid short header releases source");
    }
}
#[test]
fn ref19_egg_posix_original_budgets_are_finite() {
    let (_owned, source) = owned_source("posix.egg");
    for budget in ["metadata", "working", "entry", "total"] {
        let (terminal, _, outputs) = worker_options(
            &source,
            if budget == "metadata" {
                "open"
            } else {
                "extract"
            },
            |request| match budget {
                "metadata" => request.limits.max_metadata_bytes = 512,
                "working" => request.limits.max_working_bytes = 1,
                "entry" => request.limits.max_entry_bytes = 1,
                "total" => request.limits.max_output_bytes = 1,
                _ => unreachable!(),
            },
            Effect::None,
        );
        assert_eq!(
            terminal["error_code"], "ResourceLimit",
            "{budget}: {terminal}"
        );
        assert!(outputs.is_empty());
    }
    fs::remove_file(source).expect("budget failure releases source");
}
#[test]
fn ref19_egg_posix_cancel_and_native_first_failure_drain() {
    for (effect, code) in [
        (Effect::CancelFirstWrite, "Cancelled"),
        (Effect::FailFirstWrite, "ResourceLimit"),
    ] {
        let (_owned, source) = owned_source("posix.egg");
        let (terminal, _, outputs) = worker_options(&source, "extract", |_| {}, effect);
        assert_eq!(terminal["error_code"], code, "{terminal}");
        assert!(outputs.is_empty());
        fs::remove_file(source).expect("actual POSIX child/output/source drain");
    }
}

fn names_manifest() -> Value {
    serde_json::from_str(include_str!("fixtures/alzeg/expected/names/manifest.json"))
        .expect("independent raw-name qualification")
}
fn raw_name_boundary(fixture: &str) {
    let manifest = names_manifest();
    let expected = manifest["unsafe_names"].as_array().expect("raw names");
    let (_owned, source) = owned_source(fixture);
    let (terminal, catalog, outputs) = worker(&source, "open");
    assert_eq!(terminal["op"], "complete", "{fixture}: {terminal}");
    assert!(outputs.is_empty());
    assert_eq!(catalog.len(), expected.len());
    for (entry, member) in catalog.iter().zip(expected) {
        assert_eq!(entry["name"], member["name"], "raw name is never sanitized");
        assert_eq!(entry["ordinal"], member["index"]);
        assert_eq!(entry["size"], member["size"]);
        assert!(
            entry["warning_codes"]
                .as_array()
                .expect("warnings")
                .iter()
                .any(|warning| warning == "UnsafePath")
        );
    }
    let (terminal, _, outputs) = worker(&source, "test");
    assert_eq!(terminal["op"], "complete", "read-only Test: {terminal}");
    assert_eq!(terminal["files_verified"], expected.len() as u64);
    assert_eq!(
        terminal["bytes_verified"],
        expected
            .iter()
            .map(|member| member["size"].as_u64().expect("size"))
            .sum::<u64>()
    );
    assert!(outputs.is_empty());
    let (terminal, _, outputs) = worker(&source, "extract");
    assert_eq!(
        terminal["error_code"], "UnsafePath",
        "before native output: {terminal}"
    );
    assert!(outputs.is_empty(), "no sanitized or escaping output");
    fs::remove_file(source).expect("raw-name source released after actual child exit");
}
#[test]
fn ref19_alz_raw_unsafe_names_preserved_and_not_materialized() {
    raw_name_boundary("spec_raw_names.alz");
}
#[test]
fn ref19_egg_raw_unsafe_names_preserved_and_not_materialized() {
    raw_name_boundary("spec_raw_names.egg");
}
#[test]
fn ref19_egg_locale_parent_and_encrypted_name_declarations_are_typed() {
    let manifest = names_manifest();
    for profile in manifest["profiles"].as_array().expect("profiles") {
        let fixture = profile["fixture"].as_str().expect("fixture");
        let (_owned, source) = owned_source(fixture);
        for operation in ["open", "test", "extract"] {
            for password in [None, Some("test1234")] {
                let (terminal, catalog, outputs) = worker_options(
                    &source,
                    operation,
                    |request| request.password = password.map(str::to_owned),
                    Effect::None,
                );
                assert_eq!(
                    terminal["error_code"], profile["typed"],
                    "{fixture}/{operation}: {terminal}"
                );
                assert!(
                    catalog.is_empty() && outputs.is_empty(),
                    "unsupported names never become empty success"
                );
            }
        }
        fs::remove_file(source).expect("unsupported declaration source/child released");
    }
}
