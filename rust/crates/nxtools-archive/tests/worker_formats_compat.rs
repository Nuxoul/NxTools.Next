#![cfg(feature = "isolated-codecs")]
#[path = "common/bulk_peer.rs"]
mod bulk_peer;
use bulk_peer::{read_frame, write_frame};
use nxtools_archive::worker::WorkerRequest;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use uuid::Uuid;

// Opt-in product corpus: copy one original source at a time under the worker's owned scratch.
// The native-output fixture keeps partial bytes private until the original commit acknowledgement.
#[test]
fn compatibility_corpus_runs_original_worker_list_test_and_capability_extraction() {
    let Ok(corpus) = std::env::var("NXTOOLS_ARCHIVE_COMPAT_CORPUS") else {
        return;
    };
    let scratch = PathBuf::from(std::env::var("PI_SCRATCH_DIR").expect("owned scratch"));
    assert_eq!(Path::new(&std::env::var("TEMP").expect("TEMP")), scratch);
    assert_eq!(Path::new(&std::env::var("TMP").expect("TMP")), scratch);
    let name = format!("archive-product-{}-", Uuid::new_v4());
    let owned = tempfile::Builder::new()
        .prefix(&name)
        .tempdir_in(&scratch)
        .expect("owned corpus copy");
    let corpus = Path::new(&corpus);
    for fixture in [
        "image/cab-mszip.cab",
        "image/cab-lzx.cab",
        "image/iso9660-only.iso",
        "image/iso-joliet.iso",
        "image/iso-rockridge.iso",
        "image/iso-udf.iso",
        "image/wim-lzms.wim",
        "image/ar-archive.a",
        "image/cpio-newc.cpio",
        "image/cpio-bsdtar.cpio",
        "stream/single.lzma",
        "stream/single.lz4",
        "stream/single-block.lz4",
        "stream/single.br",
    ] {
        let original = corpus.join("samples").join(fixture);
        assert!(original.is_file(), "fixture present: {fixture}");
        let copied = owned.path().join(original.file_name().expect("leaf"));
        fs::copy(&original, &copied).expect("copy source");
        let (listed, catalog) = worker(&copied, "open");
        assert_eq!(listed["op"], "complete", "list {fixture}: {listed}");
        assert!(!catalog.is_empty(), "nonempty catalog {fixture}");
        let (tested, _) = worker(&copied, "test");
        assert_eq!(tested["op"], "complete", "test {fixture}: {tested}");
        let (extracted, payloads) = worker(&copied, "extract");
        if fixture == "image/cpio-bsdtar.cpio" {
            assert_eq!(extracted["op"], "failure");
            assert_eq!(extracted["error_code"], "UnsafePath");
            assert!(
                payloads.is_empty(),
                "producer's question-mark names cannot be silently rewritten"
            );
            continue;
        }
        assert_eq!(
            extracted["op"], "complete",
            "extract {fixture}: {extracted}"
        );
        assert!(!payloads.is_empty(), "committed payload {fixture}");
        assert_eq!(
            payloads.len() as u64,
            extracted["files_verified"].as_u64().expect("file count")
        );
        if fixture.ends_with(".iso") {
            let mut expected: Vec<_> = [
                "readme.txt",
                "crlf.txt",
                "zero.txt",
                "images/logo.png",
                "docs/notes.md",
            ]
            .iter()
            .map(|name| fs::read(corpus.join("src/content").join(name)).expect("ISO source"))
            .collect();
            if fixture == "image/iso-joliet.iso" {
                expected.push("中文内容\n".as_bytes().to_vec());
            }
            let mut actual: Vec<_> = payloads.values().cloned().collect();
            expected.sort();
            actual.sort();
            assert_eq!(actual, expected, "ISO/UDF decoded payloads {fixture}");
        } else {
            for (name, payload) in &payloads {
                let name = name.replace('\\', "/");
                let name = name.trim_start_matches("./");
                let expected = if fixture.starts_with("stream/") {
                    fs::read(corpus.join("src/stream/text-512k.txt")).expect("stream source")
                } else if name == "odd.txt" {
                    b"odd length payload".to_vec()
                } else if fixture.starts_with("image/cab-") {
                    let candidate = if name == "notes.md" {
                        "docs/notes.md"
                    } else {
                        name
                    };
                    fs::read(corpus.join("src/cab-stage").join(candidate)).expect("CAB source")
                } else if fixture == "image/ar-archive.a" || fixture == "image/cpio-newc.cpio" {
                    fs::read(corpus.join("src/content").join(name)).expect("ar/cpio source")
                } else if fixture == "image/wim-lzms.wim" {
                    fs::read(corpus.join("src/content").join(name)).expect("WIM root source")
                } else {
                    fs::read(corpus.join("src").join(name)).unwrap_or_else(|error| {
                        panic!("independent source {fixture}: {name}: {error}")
                    })
                };
                assert_eq!(payload, &expected, "{fixture}: {name}");
            }
        }
        fs::remove_file(copied).expect("retire source copy");
    }
}

#[test]
fn corrupted_brotli_does_not_publish_a_partial() {
    let Ok(_) = std::env::var("NXTOOLS_ARCHIVE_COMPAT_CORPUS") else {
        return;
    };
    let scratch = PathBuf::from(std::env::var("PI_SCRATCH_DIR").expect("owned scratch"));
    let owned = tempfile::Builder::new()
        .prefix("archive-product-")
        .tempdir_in(scratch)
        .expect("owned source");
    let invalid = owned.path().join("invalid.br");
    fs::write(&invalid, b"not a brotli stream").expect("corrupt source");
    let (terminal, published) = worker(&invalid, "extract");
    assert_eq!(terminal["op"], "failure");
    assert!(published.is_empty(), "no publication of unverified bytes");
}

#[test]
fn native_write_limit_is_the_terminal_error_and_discards_the_partial() {
    let Ok(corpus) = std::env::var("NXTOOLS_ARCHIVE_COMPAT_CORPUS") else {
        return;
    };
    let scratch = PathBuf::from(std::env::var("PI_SCRATCH_DIR").expect("owned scratch"));
    let owned = tempfile::Builder::new()
        .prefix("archive-product-")
        .tempdir_in(scratch)
        .expect("owned source");
    let path = owned.path().join("single.br");
    fs::copy(Path::new(&corpus).join("samples/stream/single.br"), &path).expect("copy source");
    let (terminal, published) = worker_with_fault(&path, "extract", Some("write"));
    assert_eq!(terminal["op"], "failure");
    assert_eq!(terminal["error_code"], "ResourceLimit");
    assert!(published.is_empty(), "failed native write never commits");
}

fn worker(path: &Path, operation: &str) -> (Value, BTreeMap<String, Vec<u8>>) {
    worker_with_fault(path, operation, None)
}
fn worker_with_fault(
    path: &Path,
    operation: &str,
    fail_action: Option<&str>,
) -> (Value, BTreeMap<String, Vec<u8>>) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("real isolated worker");
    let id = Uuid::new_v4();
    let mut request = WorkerRequest::open(id, path.to_string_lossy().into_owned());
    request.version = 2;
    request.operation = operation.into();
    request.bulk_transfer = (operation == "extract").then_some(1);
    write_frame(child.stdin.as_mut().expect("stdin"), &request).expect("start");
    let mut catalog = Vec::new();
    let mut partials = BTreeMap::<Uuid, (String, Vec<u8>)>::new();
    let mut committed = BTreeMap::<String, Vec<u8>>::new();
    let terminal = loop {
        let frame: Value = read_frame(child.stdout.as_mut().expect("stdout"))
            .expect("bounded frame")
            .expect("terminal before EOF");
        assert_eq!(frame["operation_id"], id.to_string());
        match frame["op"].as_str().expect("op") {
            "entry_batch" => catalog.extend(
                frame["entries"]
                    .as_array()
                    .expect("catalog")
                    .iter()
                    .cloned(),
            ),
            "native_call" => {
                let call = &frame["native"];
                let action = call["action"].as_str().expect("native action");
                let mut token = None;
                match action {
                    "source_acquire"
                    | "source_validated"
                    | "source_precommit"
                    | "output_ready"
                    | "destination_preflight"
                    | "directory"
                    | "directory_new"
                    | "directory_metadata"
                    | "file_metadata" => {}
                    "source_stamp" => {}
                    "begin" => {
                        let key = Uuid::new_v4();
                        partials.insert(
                            key,
                            (call["path"].as_str().expect("path").into(), Vec::new()),
                        );
                        token = Some(key);
                    }
                    "write" => {
                        let key = Uuid::parse_str(call["token"].as_str().expect("token"))
                            .expect("token UUID");
                        let bytes = call["data"].as_array().expect("bounded bytes");
                        assert!(bytes.len() <= 65536);
                        partials
                            .get_mut(&key)
                            .expect("owned partial")
                            .1
                            .extend(bytes.iter().map(|v| v.as_u64().expect("byte") as u8));
                    }
                    "commit" => {
                        let key = Uuid::parse_str(call["token"].as_str().expect("token"))
                            .expect("token UUID");
                        let (path, data) = partials.remove(&key).expect("partial");
                        assert!(committed.insert(path, data).is_none(), "exclusive target");
                    }
                    "abort" => {
                        let key = Uuid::parse_str(call["token"].as_str().expect("token"))
                            .expect("token UUID");
                        partials.remove(&key);
                    }
                    _ => panic!("unexpected native action {action}"),
                }
                let mut reply = json!({"version":1,"operation_id":id,"op":"native_result","call_id":call["call_id"],"success":true,"token":token});
                if action == "source_stamp" {
                    reply["read_data"] = json!("v1:owned-corpus-source");
                }
                if Some(action) == fail_action {
                    reply["success"] = json!(false);
                    reply["error_code"] = json!("write_limit");
                }
                write_frame(child.stdin.as_mut().expect("stdin"), &reply).expect("native ack");
            }
            "complete" | "failure" => break frame,
            _ => {}
        }
    };
    assert!(
        partials.is_empty(),
        "no unpublished partials after terminal"
    );
    drop(child.stdin.take());
    let result = child.wait_with_output().expect("exited worker");
    assert!(
        result.status.success() && result.stderr.is_empty(),
        "worker exit: {result:?}"
    );
    if operation == "open" {
        (
            terminal,
            catalog
                .into_iter()
                .enumerate()
                .map(|(i, e)| (i.to_string(), serde_json::to_vec(&e).expect("entry")))
                .collect(),
        )
    } else {
        (terminal, committed)
    }
}

fn binary_cpio(little: bool, link: bool) -> Vec<u8> {
    fn entry(out: &mut Vec<u8>, name: &str, data: &[u8], mode: u16, little: bool) {
        for word in [
            0x71c7u16,
            0,
            1,
            mode,
            0,
            0,
            1,
            0,
            0,
            0,
            name.len() as u16 + 1,
            0,
            data.len() as u16,
        ] {
            out.extend_from_slice(&if little {
                word.to_le_bytes()
            } else {
                word.to_be_bytes()
            });
        }
        out.extend_from_slice(name.as_bytes());
        out.push(0);
        if !out.len().is_multiple_of(2) {
            out.push(0);
        }
        out.extend_from_slice(data);
        if !out.len().is_multiple_of(2) {
            out.push(0);
        }
    }
    let mut encoded = Vec::new();
    entry(
        &mut encoded,
        "binary.txt",
        b"binary payload",
        0o100644,
        little,
    );
    if link {
        entry(&mut encoded, "link", b"binary.txt", 0o120777, little);
    }
    entry(&mut encoded, "TRAILER!!!", b"", 0, little);
    encoded
}

#[test]
fn binary_cpio_both_byte_orders_and_link_test_preserve_regular_payloads() {
    let root = tempfile::tempdir().expect("owned binary CPIO");
    for little in [false, true] {
        let path = root.path().join("binary.cpio");
        fs::write(&path, binary_cpio(little, true)).expect("independent binary records");
        let (listed, _) = worker(&path, "open");
        assert_eq!(listed["op"], "complete");
        let (tested, _) = worker(&path, "test");
        assert_eq!(
            tested["op"], "complete",
            "links do not prevent read-only verification"
        );
        assert_eq!(tested["files_verified"], 1);
        let (extracted, output) = worker(&path, "extract");
        assert_eq!(extracted["op"], "complete");
        assert_eq!(output.len(), 1);
        assert_eq!(output["binary.txt"], b"binary payload");
    }
}

#[test]
fn wim_global_ordinals_select_their_original_image_and_local_member() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/two-image-members.wim");
    let (listed, entries) = worker(&path, "open");
    assert_eq!(listed["op"], "complete");
    assert_eq!(entries.len(), 2);
    let (tested, _) = worker(&path, "test");
    assert_eq!(tested["op"], "complete");
    assert_eq!(tested["files_verified"], 2);
    let (extracted, output) = worker(&path, "extract");
    assert_eq!(extracted["op"], "complete");
    assert_eq!(output.len(), 2);
    assert_eq!(output["1/same.txt"], b"image one\n");
    assert_eq!(output["2/same.txt"], b"image two\n");
}
