//! Original engine/worker boundaries. The reply peer is an owned fixture, not a Windows
//! source lease/output-session acceptance test. Baseline reporting never asserts reuse counts.
#[path = "common/bulk_peer.rs"]
mod bulk_peer;
use bulk_peer::{read_frame, write_frame};
use nxtools_archive::{
    ArchiveEngine, ArchiveError, ArchiveModificationRefusal, ArchiveRequest,
    ArchiveWorkObservation, OpenRequest, OperationControl,
    output::{ArchiveOutput, ExtractRequest},
    planning::{ExtractionPlan, ExtractionSelection, TargetStrategy},
    worker::{DragMaterialization, DragMember, WorkerRequest},
};
use nxtools_protocol::archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    time::{Instant, UNIX_EPOCH},
};
use uuid::Uuid;

const FILES: u64 = 16;
const MEMBER_BYTES: u64 = 65536;
const ENTRIES: usize = FILES as usize + 2;
const FORMATS: [(&str, &str); 3] = [("tar", "Tar"), ("tgz", "TarGz"), ("txz", "TarXz")];
const NATIVE_STAMP: &str = "v1:owned-opt05-source";
const DEMANDS: [(u32, u64); 3] = [(4, 17), (1, 2), (8, 9)];

fn fixtures() -> PathBuf {
    let scratch = std::env::var_os("PI_SCRATCH_DIR").expect("session scratch directory");
    let root = tempfile::Builder::new()
        .prefix("opt05-tar-")
        .tempdir_in(scratch)
        .expect("owned scratch fixtures")
        .keep();
    // Independent deterministic USTAR bytes are identical beneath both compressed wrappers.
    let seed = r#"
import gzip, io, lzma, pathlib, sys, tarfile, zlib
root = pathlib.Path(sys.argv[1])
buffer = io.BytesIO()
with tarfile.open(fileobj=buffer, mode='w', format=tarfile.USTAR_FORMAT) as archive:
    for name in ('root/', 'empty/'):
        h = tarfile.TarInfo(name); h.type = tarfile.DIRTYPE; h.mode = 0o755
        archive.addfile(h)
    for n in range(16):
        h = tarfile.TarInfo('root/member-%02d.bin' % n)
        h.size = 65536; h.mode = 0o644
        archive.addfile(h, io.BytesIO(bytes([n]) * h.size))
plain = buffer.getvalue()
gz = gzip.compress(plain, compresslevel=6, mtime=0)
xz = lzma.compress(plain, preset=1, check=lzma.CHECK_CRC64)
for suffix, data in [('tar', plain), ('tgz', gz), ('txz', xz)]:
    (root / ('source.' + suffix)).write_bytes(data)
    truncated = data[:1024 + 512 + 65536 - 7] if suffix == 'tar' else data[:-7]
    (root / ('truncated.' + suffix)).write_bytes(truncated)
    (root / ('garbage.' + suffix)).write_bytes(data + b'not-a-stream')
for suffix, data in [('tgz', gz), ('txz', xz)]:
    damaged = bytearray(data); damaged[-8] ^= 1
    (root / ('bad-check.' + suffix)).write_bytes(damaged)
    bad_inner = plain[:-1] + b'!'
    packed = gzip.compress(bad_inner, mtime=0) if suffix == 'tgz' else lzma.compress(bad_inner, preset=1)
    (root / ('bad-inner.' + suffix)).write_bytes(packed)
for label, flag, extra in [('ftext', 1, b''), ('fextra', 4, b'\x04\x00abcd'), ('fhcrc', 2, None)]:
    fixed = bytearray(gz[:10]); fixed[3] |= flag
    if extra is None: extra = (zlib.crc32(fixed) & 65535).to_bytes(2, 'little')
    (root / (label + '.tgz')).write_bytes(fixed + extra + gz[10:])
fixed = bytearray(gz); fixed[8] = 1
(root / 'xfl.tgz').write_bytes(fixed)
(root / 'multi.tgz').write_bytes(gz + gzip.compress(b'', mtime=0))
(root / 'multi.txz').write_bytes(xz + lzma.compress(b'', preset=1))
for check in [lzma.CHECK_NONE, lzma.CHECK_CRC32, lzma.CHECK_CRC64, lzma.CHECK_SHA256]:
    (root / ('check-%d.txz' % check)).write_bytes(lzma.compress(plain, preset=1, check=check))
"#;
    let generated = Command::new("python")
        .args(["-c", seed])
        .arg(&root)
        .output()
        .expect("independent CPython fixture producer");
    assert!(
        generated.status.success(),
        "independent fixture producer: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
    root
}

fn measurement_fixtures() -> PathBuf {
    // Positive measurement routes may reread the exact before fixture directory. Preservation
    // tests always get a separate owned root, including the source-change regression.
    std::env::var_os("NXTOOLS_OPT05_TAR_FIXTURES").map_or_else(fixtures, PathBuf::from)
}

fn browser(path: &Path, format: &str) -> ArchiveBrowserContext {
    // Fixture setup is outside the measured worker operation; this is the same source-binding
    // shape consumed by the existing browser/private capability rendezvous.
    let data = std::fs::read(path).expect("independent fixture bytes");
    let metadata = path.metadata().expect("fixture metadata");
    ArchiveBrowserContext {
        snapshot_id: Uuid::new_v4(),
        format: format.into(),
        comment: String::new(),
        source: ArchiveSourceBinding {
            native: NATIVE_STAMP.into(),
            digest: Sha256::digest(&data).into(),
            size: metadata.len(),
            modified_unix_ns: u64::try_from(
                metadata
                    .modified()
                    .expect("modified")
                    .duration_since(UNIX_EPOCH)
                    .expect("fixture timestamp")
                    .as_nanos(),
            )
            .expect("timestamp range"),
        },
        sevenz_edit: None,
        tar_edit: None,
    }
}

fn request(path: &Path, format: &str, route: &str) -> WorkerRequest {
    let mut request = WorkerRequest::open(Uuid::new_v4(), path.to_string_lossy().into_owned());
    request.version = 2;
    request.operation = if route == "worker_test" {
        "test"
    } else {
        "extract"
    }
    .into();
    request.bulk_transfer = (request.operation == "extract").then_some(1);
    if matches!(route, "worker_materialize" | "worker_retained") {
        request.limits.max_entries = 100_000;
        request.limits.max_metadata_bytes = 128 << 20;
        request.browser_entry = Some(
            serde_json::from_value(json!({
                "ordinal": DEMANDS[0].1,
                "expected": browser(path, format)
            }))
            .expect("original browser entry"),
        );
    }
    if route == "worker_retained" {
        request.drag_materialization = Some(DragMaterialization {
            version: 1,
            members: DEMANDS
                .iter()
                .map(|&(index, ordinal)| DragMember { index, ordinal })
                .collect(),
        });
    }
    request.validate().expect("original request admission");
    request
}

struct OwnedWorker(Child);
impl Drop for OwnedWorker {
    fn drop(&mut self) {
        if self.0.try_wait().ok().flatten().is_none() {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

struct Run {
    terminal: Value,
    actions: Vec<String>,
    committed: BTreeMap<String, Vec<u8>>,
    ready: usize,
    elapsed_us: u128,
    terminal_to_exit_us: u128,
}

fn demand(child: &mut Child, id: Uuid, position: usize) -> (Uuid, u32, u64) {
    let (index, ordinal) = DEMANDS[position];
    let request_id = Uuid::new_v4();
    write_frame(
        child.stdin.as_mut().expect("original control"),
        &json!({"version":1,"operation_id":id,"op":"drag_demand","demand":{
            "request_id":request_id,"index":index,"ordinal":ordinal,"output_limit":MEMBER_BYTES
        }}),
    )
    .expect("original retained demand");
    (request_id, index, ordinal)
}

fn worker(request: &WorkerRequest, cancel_at: Option<&str>) -> Run {
    let started = Instant::now();
    let mut child = OwnedWorker(
        Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
            .arg("--archive-worker")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("original worker executable"),
    );
    let id = request.operation_id;
    write_frame(child.0.stdin.as_mut().expect("input"), request).expect("original request");
    let mut accepted = false;
    let mut partials = BTreeMap::<Uuid, (String, Vec<u8>)>::new();
    let mut committed = BTreeMap::new();
    let mut actions = Vec::new();
    let mut ready = 0;
    let mut active = None;
    let mut cancelled = false;
    let terminal = loop {
        let frame: Value = read_frame(child.0.stdout.as_mut().expect("output"))
            .expect("bounded original worker frame")
            .expect("terminal before EOF");
        assert_eq!(frame["operation_id"], id.to_string());
        match frame["op"].as_str().expect("operation") {
            "accepted" => {
                assert!(!accepted, "single original admission");
                accepted = true;
                let capabilities = frame["capabilities"].as_array().expect("capabilities");
                for capability in ["source_lease_v1", "budget_reservations_v1"] {
                    assert!(capabilities.iter().any(|value| value == capability));
                }
                if request.browser_entry.is_some() {
                    assert!(capabilities.iter().any(|value| value == "browser_entry_v1"));
                }
                if request.drag_materialization.is_some() {
                    assert!(
                        capabilities
                            .iter()
                            .any(|value| value == "drag_materialization_v1")
                    );
                    active = Some(demand(&mut child.0, id, 0));
                }
            }
            "native_call" => {
                assert!(
                    accepted,
                    "capability rendezvous precedes source/output effects"
                );
                let call = &frame["native"];
                let action = call["action"].as_str().expect("native action");
                actions.push(action.to_owned());
                if cancel_at == Some(action) && !cancelled {
                    cancelled = true;
                    write_frame(
                        child.0.stdin.as_mut().expect("control"),
                        &json!({"version":1,"operation_id":id,"op":"cancel"}),
                    )
                    .expect("original cancellation");
                }
                let mut reply = json!({"version":1,"operation_id":id,"op":"native_result",
                    "call_id":call["call_id"],"success":true});
                let token = || {
                    Uuid::parse_str(call["token"].as_str().expect("token")).expect("original token")
                };
                match action {
                    "source_stamp" => reply["read_data"] = json!(NATIVE_STAMP),
                    "begin" => {
                        let key = Uuid::new_v4();
                        partials.insert(
                            key,
                            (
                                call["path"].as_str().expect("relative path").into(),
                                Vec::new(),
                            ),
                        );
                        reply["token"] = json!(key);
                    }
                    "write" => {
                        let bytes = call["data"].as_array().expect("bounded private payload");
                        assert!(bytes.len() <= 65536);
                        partials.get_mut(&token()).expect("owned partial").1.extend(
                            bytes.iter().map(|byte| {
                                u8::try_from(byte.as_u64().expect("byte")).expect("byte range")
                            }),
                        );
                    }
                    "commit" => {
                        let (name, bytes) = partials.remove(&token()).expect("owned partial");
                        assert!(
                            committed.insert(name, bytes).is_none(),
                            "no duplicate publication"
                        );
                    }
                    "abort" => {
                        partials.remove(&token());
                    }
                    "source_acquire" | "source_validated" | "codec_opened" | "source_decoding"
                    | "source_precommit" | "output_ready" | "directory" | "member_acquire" => {}
                    other => panic!("unexpected native action {other}"),
                }
                if !cancelled
                    || request.operation == "extract"
                        && request.browser_entry.is_none()
                        && action == "write"
                {
                    write_frame(child.0.stdin.as_mut().expect("control"), &reply)
                        .expect("original reply");
                }
            }
            "drag_member_ready" => {
                let (request_id, index, ordinal) = active.take().expect("one active demand");
                let member = &frame["member_ready"];
                assert_eq!(member["request_id"], request_id.to_string());
                assert_eq!(member["index"], index);
                assert_eq!(member["ordinal"], ordinal);
                assert_eq!(member["bytes"], MEMBER_BYTES);
                assert!(
                    partials.is_empty(),
                    "member-ready follows acknowledged commit"
                );
                assert_eq!(committed.len(), ready + 1);
                ready += 1;
                if ready < DEMANDS.len() {
                    active = Some(demand(&mut child.0, id, ready));
                } else {
                    write_frame(
                        child.0.stdin.as_mut().expect("control"),
                        &json!({"version":1,"operation_id":id,"op":"drag_end"}),
                    )
                    .expect("end original retained operation");
                }
            }
            "complete" | "failure" => break frame,
            "progress" | "extraction_summary" => {}
            other => panic!("unexpected worker frame {other}"),
        }
    };
    let terminal_at = Instant::now();
    assert!(accepted, "real worker protocol admission");
    if terminal["op"] == "complete" {
        assert!(partials.is_empty(), "successful partials committed");
    }
    drop(child.0.stdin.take());
    let status = child.0.wait().expect("actual worker cleanup/exit");
    let terminal_to_exit_us = terminal_at.elapsed().as_micros();
    let elapsed_us = started.elapsed().as_micros();
    use std::io::Read;
    let mut stderr = Vec::new();
    child
        .0
        .stderr
        .take()
        .expect("stderr")
        .read_to_end(&mut stderr)
        .expect("worker stderr");
    assert!(
        status.success() && stderr.is_empty(),
        "worker exited successfully"
    );
    // An acknowledged cancellation may prevent the worker's abort RPC. The fixture peer
    // retires its unpublished buffers only after actual exit; native cleanup is a separate gate.
    drop(partials);
    Run {
        terminal,
        actions,
        committed,
        ready,
        elapsed_us,
        terminal_to_exit_us,
    }
}

fn assert_success(run: &Run, route: &str) {
    assert_eq!(run.terminal["op"], "complete", "{}", run.terminal);
    let files = match route {
        "worker_materialize" => 1,
        "worker_retained" => DEMANDS.len() as u64,
        _ => FILES,
    };
    assert_eq!(run.terminal["files_verified"], files);
    assert_eq!(run.terminal["bytes_verified"], files * MEMBER_BYTES);
    if route == "worker_test" {
        assert!(run.committed.is_empty());
        assert!(
            !run.actions
                .iter()
                .any(|action| matches!(action.as_str(), "begin" | "write" | "commit"))
        );
    } else {
        assert_eq!(run.committed.len(), files as usize);
        for (name, bytes) in &run.committed {
            let index: u8 = name
                .strip_prefix("root/member-")
                .expect("literal original path")
                .strip_suffix(".bin")
                .expect("literal extension")
                .parse()
                .expect("fixture index");
            assert_eq!(bytes.len(), MEMBER_BYTES as usize);
            assert!(
                bytes.iter().all(|byte| *byte == index),
                "exact independent payload"
            );
        }
    }
    if route == "worker_retained" {
        assert_eq!(run.ready, DEMANDS.len());
        for (_, ordinal) in DEMANDS {
            assert!(
                run.committed
                    .contains_key(&format!("root/member-{:02}.bin", ordinal - 2))
            );
        }
    }
}

fn sample(path: &Path, format: &str, route: &str, trial: u32) -> Value {
    if route == "engine_open" {
        let control = OperationControl::default();
        let started = Instant::now();
        let snapshot = ArchiveEngine::default()
            .open(OpenRequest::new(path), &control)
            .expect("original listing and edit qualification");
        let elapsed_us = started.elapsed().as_micros();
        assert_eq!(snapshot.entries().len(), ENTRIES);
        assert!(snapshot.capabilities().add && snapshot.capabilities().extract);
        for (ordinal, entry) in snapshot.entries().iter().enumerate() {
            assert_eq!(entry.id().ordinal, ordinal as u64);
        }
        json!({"route":route,"format":format,"trial":trial,"elapsed_us":elapsed_us,
            "elapsed_scope":"public_open_including_qualification","terminal_to_exit_us":null,
            "work":control.work_observation(),"total_io_bytes":null})
    } else {
        let request = request(path, format, route);
        let run = worker(&request, None);
        assert_success(&run, route);
        json!({"route":route,"format":format,"trial":trial,"elapsed_us":run.elapsed_us,
            "elapsed_scope":"worker_spawn_to_actual_exit","terminal_to_exit_us":run.terminal_to_exit_us,
            "work":run.terminal["work"],"total_io_bytes":null})
    }
}

fn record(root: &Path, mode: &str, samples: &[Value]) {
    let label = std::env::var("NXTOOLS_OPT05_TAR_REPORT_LABEL").unwrap_or_else(|_| "trial".into());
    let report = json!({"schema":1,"mode":mode,"label":label,
        "build":if cfg!(debug_assertions) { "debug" } else { "release" },
        "fixture":"CPython USTAR; 2 directories + 16 x 65536-byte regular files; identical inner bytes",
        "native_boundary":"owned private reply peer; not Windows source/output acceptance",
        "tar_record_scans_scope":"formats/tar fixed-header parser; strict tar_edit parser not included",
        "tar_plan_bytes_scope":"cumulative admitted ordinal Vec backing capacity x 8 at parser retirement; includes failed scans; excludes snapshot/allocator/strict edit records; not peak RSS",
        "samples":samples});
    let path = root.join(format!("{label}-{mode}.json"));
    std::fs::write(
        &path,
        serde_json::to_vec_pretty(&report).expect("bounded report"),
    )
    .expect("scratch evidence report");
    println!("OPT05 TAR evidence: {}", path.display());
}

#[test]
#[ignore = "opt-in three completed trials; writes owned scratch evidence and asserts correctness, not optimized counts"]
fn opt05_tar_baseline_three_completed_trials() {
    let root = measurement_fixtures();
    let mut samples = Vec::new();
    for trial in 1..=3 {
        for (suffix, format) in FORMATS {
            let path = root.join(format!("source.{suffix}"));
            for route in [
                "engine_open",
                "worker_test",
                "worker_extract",
                "worker_materialize",
                "worker_retained",
            ] {
                samples.push(sample(&path, format, route, trial));
            }
        }
    }
    record(&root, "baseline", &samples);
    assert_eq!(
        samples.len(),
        45,
        "three completed independent trials per route/format"
    );
}

#[test]
fn opt05_tar_engine_open_reuses_verified_outer_for_qualification() {
    let root = measurement_fixtures();
    let samples: Vec<_> = FORMATS
        .iter()
        .map(|(suffix, format)| {
            sample(
                &root.join(format!("source.{suffix}")),
                format,
                "engine_open",
                1,
            )
        })
        .collect();
    record(&root, "engine-reuse", &samples);
    let plain_bytes = root
        .join("source.tar")
        .metadata()
        .expect("inner size")
        .len();
    for sample in samples {
        let compressed = sample["format"] != "Tar";
        assert_eq!(
            sample["work"]["outer_decodes"],
            u64::from(compressed),
            "{sample}"
        );
        assert_eq!(
            sample["work"]["outer_decoded_bytes"],
            if compressed { plain_bytes } else { 0 },
            "{sample}"
        );
    }
}

#[test]
fn opt05_tar_actual_worker_reuses_catalog_spool_and_ordinal_plan() {
    let root = measurement_fixtures();
    let mut samples = Vec::new();
    for (suffix, format) in FORMATS {
        for route in [
            "worker_test",
            "worker_extract",
            "worker_materialize",
            "worker_retained",
        ] {
            samples.push(sample(
                &root.join(format!("source.{suffix}")),
                format,
                route,
                1,
            ));
        }
    }
    record(&root, "worker-reuse", &samples);
    let plain_bytes = root
        .join("source.tar")
        .metadata()
        .expect("inner size")
        .len();
    for sample in samples {
        let compressed = sample["format"] != "Tar";
        assert_eq!(
            sample["work"]["outer_decodes"],
            u64::from(compressed),
            "{sample}"
        );
        assert_eq!(
            sample["work"]["outer_decoded_bytes"],
            if compressed { plain_bytes } else { 0 },
            "{sample}"
        );
        assert_eq!(sample["work"]["tar_record_scans"], 1, "{sample}");
        assert_eq!(
            sample["work"]["tar_header_seeks"],
            (ENTRIES + 1) as u64,
            "{sample}"
        );
        assert!(
            sample["work"]["tar_index_builds"]
                .as_u64()
                .expect("index counter")
                <= 1,
            "{sample}"
        );
    }
}

#[test]
fn opt05_tar_retained_demands_build_ordinal_lookup_at_most_once() {
    let root = measurement_fixtures();
    let samples: Vec<_> = FORMATS
        .iter()
        .map(|(suffix, format)| {
            sample(
                &root.join(format!("source.{suffix}")),
                format,
                "worker_retained",
                1,
            )
        })
        .collect();
    record(&root, "retained-index-reuse", &samples);
    for sample in samples {
        assert_eq!(
            sample["work"]["tar_payload_seeks"],
            DEMANDS.len() as u64,
            "{sample}"
        );
        assert!(
            sample["work"]["tar_index_builds"]
                .as_u64()
                .expect("index builds")
                <= 1,
            "{sample}"
        );
    }
}

#[test]
fn opt05_tar_qualification_preserves_readable_but_not_editable_wrappers() {
    let root = fixtures();
    for name in [
        "ftext.tgz",
        "fextra.tgz",
        "fhcrc.tgz",
        "xfl.tgz",
        "multi.tgz",
        "multi.txz",
    ] {
        let control = OperationControl::default();
        let snapshot = ArchiveEngine::default()
            .open(OpenRequest::new(root.join(name)), &control)
            .expect("complete ordinary read remains supported");
        assert_eq!(snapshot.entries().len(), ENTRIES, "{name}");
        assert!(
            snapshot.capabilities().extract && snapshot.capabilities().test,
            "{name}"
        );
        assert_eq!(
            snapshot.capabilities().modification_refusal,
            Some(ArchiveModificationRefusal::UnsupportedStructure),
            "{name}"
        );
    }
    for check in [0, 1, 4, 10] {
        let snapshot = ArchiveEngine::default()
            .open(
                OpenRequest::new(root.join(format!("check-{check}.txz"))),
                &OperationControl::default(),
            )
            .expect("qualified one-stream XZ check");
        assert!(snapshot.capabilities().add, "check {check}");
    }
}

#[test]
fn opt05_tar_outer_and_inner_failures_precede_worker_output() {
    let root = fixtures();
    for (suffix, format) in FORMATS {
        for prefix in ["truncated", "garbage", "bad-check", "bad-inner"] {
            if suffix == "tar" && matches!(prefix, "bad-check" | "bad-inner") {
                continue;
            }
            let path = root.join(format!("{prefix}.{suffix}"));
            let run = worker(&request(&path, format, "worker_extract"), None);
            assert_eq!(run.terminal["op"], "failure", "{prefix}.{suffix}");
            assert_eq!(
                run.terminal["error_code"], "CorruptArchive",
                "{prefix}.{suffix}"
            );
            assert!(run.committed.is_empty());
            assert!(
                !run.actions.iter().any(|action| matches!(
                    action.as_str(),
                    "directory" | "begin" | "write" | "commit"
                )),
                "{prefix}.{suffix}"
            );
        }
    }
}

#[derive(Default)]
struct CountingOutput {
    pending: BTreeMap<Uuid, u64>,
    effects: u64,
}
impl ArchiveOutput for CountingOutput {
    fn ensure_directory(&mut self, _: &str) -> nxtools_archive::Result<()> {
        self.effects += 1;
        Ok(())
    }
    fn begin_file(&mut self, _: &str) -> nxtools_archive::Result<Uuid> {
        self.effects += 1;
        let token = Uuid::new_v4();
        self.pending.insert(token, 0);
        Ok(token)
    }
    fn write_file(&mut self, token: Uuid, bytes: &[u8]) -> nxtools_archive::Result<()> {
        *self.pending.get_mut(&token).expect("owned fixture token") += bytes.len() as u64;
        Ok(())
    }
    fn set_file_metadata(
        &mut self,
        token: Uuid,
        _: nxtools_archive::metadata::OutputMetadata,
    ) -> nxtools_archive::Result<()> {
        assert!(self.pending.contains_key(&token));
        Ok(())
    }
    fn finalize_directory_metadata(
        &mut self,
        _: &str,
        _: nxtools_archive::metadata::OutputMetadata,
    ) -> nxtools_archive::Result<()> {
        Ok(())
    }
    fn commit_file(&mut self, token: Uuid) -> nxtools_archive::Result<()> {
        self.pending.remove(&token).expect("owned fixture token");
        Ok(())
    }
    fn abort_file(&mut self, token: Uuid) {
        self.pending.remove(&token);
    }
}

#[test]
fn opt05_tar_public_operations_remain_fresh_and_detect_source_change() {
    let root = fixtures();
    for (suffix, _) in FORMATS {
        let path = root.join(format!("source.{suffix}"));
        let engine = ArchiveEngine::default();
        let snapshot = engine
            .open(OpenRequest::new(&path), &OperationControl::default())
            .expect("open");
        let control = OperationControl::default();
        let outcome = engine
            .execute(ArchiveRequest::test(snapshot.clone()), &control)
            .expect("independent public Test");
        assert_eq!(outcome.files_verified(), FILES);
        let work: ArchiveWorkObservation = control.work_observation();
        assert_eq!(
            work.outer_decodes,
            u64::from(suffix != "tar"),
            "fresh operation decodes its own source"
        );
        let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
            .expect("independent public extract plan");
        let extract_control = OperationControl::default();
        let mut output = CountingOutput::default();
        let extracted = engine
            .extract(
                ExtractRequest::new(snapshot.clone(), plan),
                &extract_control,
                &mut output,
            )
            .expect("independent public extraction");
        assert_eq!(extracted.files_committed, FILES);
        assert_eq!(extracted.bytes_committed, FILES * MEMBER_BYTES);
        assert!(output.pending.is_empty());
        assert_eq!(
            extract_control.work_observation().outer_decodes,
            u64::from(suffix != "tar")
        );
        std::fs::write(&path, b"changed owned source").expect("replace unleased fixture");
        assert!(matches!(
            engine.execute(
                ArchiveRequest::test(snapshot.clone()),
                &OperationControl::default()
            ),
            Err(ArchiveError::SourceChanged)
        ));
        assert_eq!(
            engine.modification_eligibility(&snapshot, &OperationControl::default()),
            Err(ArchiveModificationRefusal::SourceChanged)
        );
        let stale_plan =
            ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
                .expect("old immutable projection");
        let mut untouched = CountingOutput::default();
        assert!(matches!(
            engine.extract(
                ExtractRequest::new(snapshot, stale_plan),
                &OperationControl::default(),
                &mut untouched,
            ),
            Err(ArchiveError::SourceChanged)
        ));
        assert_eq!(untouched.effects, 0, "stale source rejected before output");
    }
}

#[test]
fn opt05_tar_actual_worker_cancel_prevents_commit_and_exits() {
    let root = fixtures();
    for (suffix, format) in FORMATS {
        let path = root.join(format!("source.{suffix}"));
        let run = worker(&request(&path, format, "worker_extract"), Some("write"));
        assert_eq!(run.terminal["op"], "failure");
        assert_eq!(run.terminal["error_code"], "Cancelled");
        assert!(run.actions.iter().any(|action| action == "begin"));
        assert!(!run.actions.iter().any(|action| action == "commit"));
        assert!(run.committed.is_empty());
        assert_eq!(
            std::fs::read(&path)
                .expect("source readable after child exit")
                .len() as u64,
            path.metadata().expect("source retained").len()
        );
    }
}

#[test]
fn opt05_tar_original_worker_budgets_precede_output() {
    let root = fixtures();
    for (suffix, format) in FORMATS {
        for budget in ["metadata", "working", "spool", "entry"] {
            let path = root.join(format!("source.{suffix}"));
            let mut request = request(&path, format, "worker_extract");
            match budget {
                "metadata" => request.limits.max_metadata_bytes = 512,
                "working" => request.limits.max_working_bytes = 1,
                "spool" => request.limits.max_output_bytes = MEMBER_BYTES,
                "entry" => request.limits.max_entry_bytes = 1,
                _ => unreachable!("fixed budget cases"),
            }
            let run = worker(&request, None);
            assert_eq!(run.terminal["op"], "failure", "{suffix}/{budget}");
            assert_eq!(
                run.terminal["error_code"], "ResourceLimit",
                "{suffix}/{budget}"
            );
            assert!(run.committed.is_empty());
            assert!(
                !run.actions.iter().any(|action| matches!(
                    action.as_str(),
                    "directory" | "begin" | "write" | "commit"
                )),
                "{suffix}/{budget}"
            );
        }
    }
}

#[test]
fn opt05_tar_optional_edit_metadata_budget_does_not_remove_read_capability() {
    let root = fixtures();
    let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
        max_metadata_bytes: 64 << 10,
        ..Default::default()
    });
    for (suffix, _) in FORMATS {
        let path = root.join(format!("source.{suffix}"));
        let snapshot = engine
            .open(OpenRequest::new(path), &OperationControl::default())
            .expect("ordinary catalog fits without optional strict edit records");
        assert_eq!(snapshot.entries().len(), ENTRIES);
        assert!(snapshot.capabilities().extract && snapshot.capabilities().test);
        assert_eq!(
            snapshot.capabilities().modification_refusal,
            Some(ArchiveModificationRefusal::UnverifiedMetadata)
        );
    }
}

#[test]
fn opt05_tar_strict_qualification_charges_snapshot_and_plan_together() {
    let root = fixtures();
    for (suffix, _) in FORMATS {
        let path = root.join(format!("source.{suffix}"));
        let snapshot = ArchiveEngine::default()
            .open(OpenRequest::new(&path), &OperationControl::default())
            .expect("catalog fixture");
        let catalog_bytes: u64 = snapshot
            .entries()
            .iter()
            .map(|entry| 1024 + entry.raw_name().len() as u64 * 32)
            .sum();
        let strict_bytes: u64 = snapshot
            .entries()
            .iter()
            .map(|entry| {
                let mut parents = 0;
                let mut name = entry.display_path().trim_end_matches('/');
                while let Some((prefix, _)) = name.rsplit_once('/') {
                    parents += 256 + prefix.len() as u64 * 8;
                    name = prefix;
                }
                8192 + entry.display_path().len() as u64 * 128 + parents
            })
            .sum();
        let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
            // Either catalog or strict records fits separately; their coexistence does not.
            max_metadata_bytes: strict_bytes + catalog_bytes / 2,
            ..Default::default()
        });
        let control = OperationControl::default();
        let admitted = engine
            .open(OpenRequest::new(&path), &control)
            .expect("readable catalog");
        assert!(admitted.capabilities().test && admitted.capabilities().extract);
        assert_eq!(
            admitted.capabilities().modification_refusal,
            Some(ArchiveModificationRefusal::UnverifiedMetadata),
            "{suffix}"
        );
        assert_eq!(
            control.work_observation().outer_decodes,
            u64::from(suffix != "tar")
        );
        assert_eq!(control.work_observation().tar_record_scans, 1);
        assert_eq!(
            engine.modification_eligibility(&admitted, &OperationControl::default()),
            Err(ArchiveModificationRefusal::UnverifiedMetadata)
        );
    }
}

#[test]
fn opt05_tar_worker_charges_locator_work_alongside_execution() {
    let root = fixtures();
    let mut request = request(&root.join("source.tar"), "Tar", "worker_extract");
    request.limits.max_working_bytes = 256 * 1024 + ENTRIES as u64 * 128;
    let run = worker(&request, None);
    assert_eq!(run.terminal["op"], "failure");
    assert_eq!(run.terminal["error_code"], "ResourceLimit");
    assert_eq!(run.terminal["work"]["tar_record_scans"], 1);
    assert!(
        run.terminal["work"]["tar_plan_bytes"]
            .as_u64()
            .expect("locator bytes")
            > 0
    );
    assert!(
        !run.actions
            .iter()
            .any(|action| matches!(action.as_str(), "directory" | "begin" | "write" | "commit"))
    );
    assert!(run.committed.is_empty());
}

#[test]
fn opt05_tar_public_execution_charges_locator_work_alongside_execution() {
    let root = fixtures();
    let snapshot = ArchiveEngine::default()
        .open(
            OpenRequest::new(root.join("source.tar")),
            &OperationControl::default(),
        )
        .expect("independent catalog");
    let engine = ArchiveEngine::with_limits(nxtools_archive::ResourceLimits {
        max_working_bytes: 256 * 1024 + ENTRIES as u64 * 128,
        ..Default::default()
    });
    assert_eq!(
        engine.execute(
            ArchiveRequest::test(snapshot.clone()),
            &OperationControl::default()
        ),
        Err(ArchiveError::ResourceLimit),
        "fresh parser locators coexist with execution state"
    );
    let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
        .expect("original extraction plan");
    let mut output = CountingOutput::default();
    assert!(matches!(
        engine.extract(
            ExtractRequest::new(snapshot, plan),
            &OperationControl::default(),
            &mut output
        ),
        Err(ArchiveError::ResourceLimit)
    ));
    assert_eq!(output.effects, 0);
    assert!(output.pending.is_empty());
}
