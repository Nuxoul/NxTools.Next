#[path = "common/bulk_peer.rs"]
mod bulk_peer;
use base64::Engine as _;
use bulk_peer::{read_frame, write_frame};
use nxtools_archive::{
    creation::CreationOptions,
    worker::{SevenzCreateOptions, WorkerRequest, ZipCreateOptions},
};
use nxtools_protocol::{
    archive_job::{
        ArchiveJobCreate, ArchiveJobFormat, ArchiveJobKind, ArchiveJobLevel, ArchiveJobLimits,
        ArchiveJobMethod, ArchiveJobRequest,
    },
    archive_password::PrivatePassword,
};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use uuid::Uuid;

const SECRET: &str = "opt03-private-secret";

#[derive(Clone, Copy, PartialEq, Eq)]
enum FixtureKind {
    Single,
    Pair,
    StrippedDirectory,
}

#[derive(Clone, Copy)]
struct CreateCase {
    label: &'static str,
    format: ArchiveJobFormat,
    operation: &'static str,
    output: &'static str,
    level: Option<(ArchiveJobLevel, &'static str)>,
    method: Option<ArchiveJobMethod>,
    password: Option<&'static str>,
    solid: Option<bool>,
    encrypt_header: Option<bool>,
    volume_bytes: Option<u64>,
    fixture: FixtureKind,
    strip_single_root: bool,
    exclude_patterns: &'static [&'static str],
}

fn cases() -> Vec<CreateCase> {
    vec![
        CreateCase {
            label: "zip-store-utf8-strip-exclude",
            format: ArchiveJobFormat::Zip,
            operation: "create_zip",
            output: "unicode.zip",
            level: Some((ArchiveJobLevel::Store, "store")),
            method: Some(ArchiveJobMethod::Store),
            password: None,
            solid: None,
            encrypt_header: None,
            volume_bytes: None,
            fixture: FixtureKind::StrippedDirectory,
            strip_single_root: true,
            exclude_patterns: &["Thumbs.db"],
        },
        CreateCase {
            label: "zip-deflate-aes-split",
            format: ArchiveJobFormat::Zip,
            operation: "create_zip",
            output: "encrypted-split.zip",
            level: Some((ArchiveJobLevel::Maximum, "maximum")),
            method: Some(ArchiveJobMethod::Deflate),
            password: Some(SECRET),
            solid: None,
            encrypt_header: None,
            volume_bytes: Some(65536),
            fixture: FixtureKind::Single,
            strip_single_root: false,
            exclude_patterns: &[],
        },
        CreateCase {
            label: "7z-fast-nonsolid",
            format: ArchiveJobFormat::SevenZ,
            operation: "create_7z",
            output: "plain.7z",
            level: Some((ArchiveJobLevel::Fast, "fast")),
            method: None,
            password: None,
            solid: Some(false),
            encrypt_header: Some(false),
            volume_bytes: None,
            fixture: FixtureKind::Single,
            strip_single_root: false,
            exclude_patterns: &[],
        },
        CreateCase {
            label: "7z-maximum-solid-aes-header-split",
            format: ArchiveJobFormat::SevenZ,
            operation: "create_7z",
            output: "encrypted.7z",
            level: Some((ArchiveJobLevel::Maximum, "maximum")),
            method: None,
            password: Some(SECRET),
            solid: Some(true),
            encrypt_header: Some(true),
            volume_bytes: Some(65536),
            fixture: FixtureKind::Pair,
            strip_single_root: false,
            exclude_patterns: &[],
        },
        CreateCase {
            label: "tar-no-level",
            format: ArchiveJobFormat::Tar,
            operation: "create_tar",
            output: "plain.tar",
            level: None,
            method: None,
            password: None,
            solid: None,
            encrypt_header: None,
            volume_bytes: None,
            fixture: FixtureKind::Single,
            strip_single_root: false,
            exclude_patterns: &[],
        },
        CreateCase {
            label: "tgz-fast",
            format: ArchiveJobFormat::Tgz,
            operation: "create_tgz",
            output: "fast.tar.gz",
            level: Some((ArchiveJobLevel::Fast, "fast")),
            method: None,
            password: None,
            solid: None,
            encrypt_header: None,
            volume_bytes: None,
            fixture: FixtureKind::Single,
            strip_single_root: false,
            exclude_patterns: &[],
        },
        CreateCase {
            label: "txz-maximum",
            format: ArchiveJobFormat::Txz,
            operation: "create_txz",
            output: "maximum.tar.xz",
            level: Some((ArchiveJobLevel::Maximum, "maximum")),
            method: None,
            password: None,
            solid: None,
            encrypt_header: None,
            volume_bytes: None,
            fixture: FixtureKind::Single,
            strip_single_root: false,
            exclude_patterns: &[],
        },
        CreateCase {
            label: "gzip-normal",
            format: ArchiveJobFormat::Gz,
            operation: "create_gzip",
            output: "single.gz",
            level: Some((ArchiveJobLevel::Normal, "normal")),
            method: None,
            password: None,
            solid: None,
            encrypt_header: None,
            volume_bytes: None,
            fixture: FixtureKind::Single,
            strip_single_root: false,
            exclude_patterns: &[],
        },
        CreateCase {
            label: "xz-maximum",
            format: ArchiveJobFormat::Xz,
            operation: "create_xz",
            output: "single.xz",
            level: Some((ArchiveJobLevel::Maximum, "maximum")),
            method: None,
            password: None,
            solid: None,
            encrypt_header: None,
            volume_bytes: None,
            fixture: FixtureKind::Single,
            strip_single_root: false,
            exclude_patterns: &[],
        },
    ]
}

#[derive(Clone)]
struct SourceNode {
    id: Uuid,
    name: String,
    directory: bool,
    payload: Vec<u8>,
}

struct SourceFixture {
    roots: Vec<Uuid>,
    children: HashMap<Uuid, Vec<Uuid>>,
    nodes: HashMap<Uuid, SourceNode>,
    paths: Vec<String>,
}

impl SourceFixture {
    fn build(root: &Path, case: CreateCase) -> Self {
        let source_root = root.join(case.label);
        std::fs::create_dir_all(&source_root).expect("owned source root");
        let large = case.volume_bytes.is_some();
        match case.fixture {
            FixtureKind::Single => {
                let payload = if large {
                    pseudo_random(150_000, 0x1357_2468)
                } else {
                    b"OPT03 independent source bytes\n".to_vec()
                };
                let path = source_root.join("资料.bin");
                std::fs::write(&path, &payload).expect("owned source file");
                Self::files(vec![(path, "资料.bin", payload)])
            }
            FixtureKind::Pair => {
                let first = pseudo_random(100_000, 0x1020_3040);
                let second = pseudo_random(100_000, 0x5060_7080);
                let first_path = source_root.join("first.bin");
                let second_path = source_root.join("second.bin");
                std::fs::write(&first_path, &first).expect("first source");
                std::fs::write(&second_path, &second).expect("second source");
                Self::files(vec![
                    (first_path, "first.bin", first),
                    (second_path, "second.bin", second),
                ])
            }
            FixtureKind::StrippedDirectory => {
                let included = b"unicode member bytes\n".to_vec();
                let excluded = b"excluded bytes\n".to_vec();
                let directory = source_root.join("根目录");
                std::fs::create_dir(&directory).expect("owned directory source");
                std::fs::write(directory.join("资料.bin"), &included).expect("included source");
                std::fs::write(directory.join("Thumbs.db"), &excluded).expect("excluded source");
                let root_id = Uuid::new_v4();
                let included_id = Uuid::new_v4();
                let excluded_id = Uuid::new_v4();
                let nodes = [
                    SourceNode {
                        id: root_id,
                        name: "根目录".into(),
                        directory: true,
                        payload: Vec::new(),
                    },
                    SourceNode {
                        id: included_id,
                        name: "资料.bin".into(),
                        directory: false,
                        payload: included,
                    },
                    SourceNode {
                        id: excluded_id,
                        name: "Thumbs.db".into(),
                        directory: false,
                        payload: excluded,
                    },
                ]
                .into_iter()
                .map(|node| (node.id, node))
                .collect();
                Self {
                    roots: vec![root_id],
                    children: HashMap::from([(root_id, vec![included_id, excluded_id])]),
                    nodes,
                    paths: vec![path_string(&directory)],
                }
            }
        }
    }

    fn files(values: Vec<(PathBuf, &'static str, Vec<u8>)>) -> Self {
        let mut roots = Vec::new();
        let mut nodes = HashMap::new();
        let mut paths = Vec::new();
        for (path, name, payload) in values {
            let id = Uuid::new_v4();
            roots.push(id);
            paths.push(path_string(&path));
            nodes.insert(
                id,
                SourceNode {
                    id,
                    name: name.into(),
                    directory: false,
                    payload,
                },
            );
        }
        Self {
            roots,
            children: HashMap::new(),
            nodes,
            paths,
        }
    }

    fn dto(&self, id: Uuid) -> Value {
        let node = self.nodes.get(&id).expect("known source node");
        json!({
            "id": node.id,
            "name": node.name,
            "directory": node.directory,
            "size": if node.directory { 0 } else { node.payload.len() as u64 },
            "dos_modified": null
        })
    }

    fn roots_json(&self) -> String {
        serde_json::to_string(
            &self
                .roots
                .iter()
                .map(|id| self.dto(*id))
                .collect::<Vec<_>>(),
        )
        .expect("source roots JSON")
    }

    fn child_json(&self, directory: Uuid, offset: usize) -> String {
        let child = self
            .children
            .get(&directory)
            .and_then(|children| children.get(offset))
            .map(|id| self.dto(*id));
        serde_json::to_string(&child).expect("source child JSON")
    }

    fn payload(&self, id: Uuid) -> &[u8] {
        &self.nodes.get(&id).expect("known source payload").payload
    }

    fn id_named(&self, name: &str) -> Uuid {
        self.nodes
            .values()
            .find(|node| node.name == name)
            .map(|node| node.id)
            .expect("named source")
    }
}

fn pseudo_random(length: usize, mut state: u32) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(length);
    for _ in 0..length {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        bytes.push((state >> 8) as u8);
    }
    bytes
}

fn path_string(path: &Path) -> String {
    path.to_str().expect("Unicode owned path").to_owned()
}

fn typed_request(case: CreateCase, paths: Vec<String>, destination: String) -> ArchiveJobRequest {
    ArchiveJobRequest {
        kind: ArchiveJobKind::Create,
        paths,
        destination: Some(destination),
        codepage: 65001,
        limits: ArchiveJobLimits::default(),
        extract: None,
        create: Some(ArchiveJobCreate {
            format: case.format,
            level: case.level.map(|(level, _)| level),
            method: case.method,
            password: case.password.map(PrivatePassword::new),
            solid: case.solid,
            encrypt_header: case.encrypt_header,
            strip_single_root: case.strip_single_root,
            exclude_patterns: case
                .exclude_patterns
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            test_after_create: true,
            name: case.output.into(),
            volume_bytes: case.volume_bytes,
        }),
        modify: None,
        readmission: None,
    }
}

fn worker_request(case: CreateCase, path: String) -> WorkerRequest {
    let mut request = WorkerRequest::open(Uuid::new_v4(), path);
    request.version = 2;
    request.operation = case.operation.into();
    request.bulk_transfer = Some(1);
    request.codepage = 65001;
    request.create_name = Some(case.output.into());
    request.compression_level = case.level.map(|(_, level)| level.to_owned());
    request.password = case.password.map(str::to_owned);
    request.volume_bytes = case.volume_bytes;
    let tree = CreationOptions {
        strip_single_root: case.strip_single_root,
        exclude_patterns: case
            .exclude_patterns
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    };
    match case.format {
        ArchiveJobFormat::Zip => {
            request.zip_options = Some(ZipCreateOptions {
                method: if case.method == Some(ArchiveJobMethod::Store) {
                    "store"
                } else {
                    "deflate"
                }
                .into(),
                encryption: if case.password.is_some() {
                    "aes256"
                } else {
                    "none"
                }
                .into(),
                strip_single_root: tree.strip_single_root,
                exclude_patterns: tree.exclude_patterns,
            });
        }
        ArchiveJobFormat::SevenZ => {
            request.tree_options = Some(tree);
            request.sevenz_options = Some(SevenzCreateOptions {
                solid: case.solid.unwrap_or(false),
                encryption: if case.password.is_some() {
                    "aes256"
                } else {
                    "none"
                }
                .into(),
                encrypt_header: case.encrypt_header.unwrap_or(false),
            });
        }
        ArchiveJobFormat::Tar | ArchiveJobFormat::Tgz | ArchiveJobFormat::Txz => {
            request.tree_options = Some(tree);
        }
        ArchiveJobFormat::Gz | ArchiveJobFormat::Xz => {}
    }
    request
}

struct WriterResult {
    artifacts: Vec<(String, Vec<u8>)>,
    source_reads: HashSet<Uuid>,
}

fn run_writer(case: CreateCase, request: &WorkerRequest, fixture: &SourceFixture) -> WriterResult {
    let mut child = Command::new(env!("CARGO_BIN_EXE_nxtools-archive-worker"))
        .arg("--archive-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("owned archive worker");
    write_frame(child.stdin.as_mut().expect("worker stdin"), request).expect("start writer");
    let mut staged = HashMap::<Uuid, (String, Vec<u8>)>::new();
    let mut committed = Vec::new();
    let mut aborted = HashSet::new();
    let mut source_reads = HashSet::new();
    let terminal = loop {
        let frame: Value = read_frame(child.stdout.as_mut().expect("worker stdout"))
            .expect("bounded worker frame")
            .expect("terminal before EOF");
        if matches!(frame["op"].as_str(), Some("complete" | "failure")) {
            break frame;
        }
        if frame["op"] != "native_call" {
            continue;
        }
        let call = &frame["native"];
        let action = call["action"].as_str().expect("native action");
        let mut reply = json!({
            "version": 1,
            "operation_id": request.operation_id,
            "op": "native_result",
            "call_id": call["call_id"],
            "success": true
        });
        match action {
            "source_acquire" | "source_validated" | "source_precommit" | "creation_planning"
            | "creation_encoding" | "creation_verifying" | "source_open" | "source_close"
            | "source_check" | "flush_volume" | "volumes_verified" => {}
            "source_roots" => reply["read_data"] = json!(fixture.roots_json()),
            "source_next" => {
                let directory: Uuid =
                    serde_json::from_value(call["token"].clone()).expect("directory source token");
                let offset = call["offset"].as_u64().expect("child offset") as usize;
                reply["read_data"] = json!(fixture.child_json(directory, offset));
            }
            "source_read" => {
                let source: Uuid =
                    serde_json::from_value(call["token"].clone()).expect("source token");
                source_reads.insert(source);
                let offset = call["offset"].as_u64().expect("source offset") as usize;
                let length = call["length"].as_u64().expect("source length") as usize;
                let payload = fixture.payload(source);
                let bytes = if offset >= payload.len() {
                    &[][..]
                } else {
                    &payload[offset..(offset + length).min(payload.len())]
                };
                reply["read_data"] = json!(base64::engine::general_purpose::STANDARD.encode(bytes));
            }
            "begin" => {
                let token = Uuid::new_v4();
                let name = call["path"].as_str().expect("candidate name").to_owned();
                staged.insert(token, (name, Vec::new()));
                reply["token"] = json!(token);
            }
            "write" => {
                let token: Uuid =
                    serde_json::from_value(call["token"].clone()).expect("write token");
                let bytes = call["data"]
                    .as_array()
                    .expect("write bytes")
                    .iter()
                    .map(|value| value.as_u64().expect("byte") as u8)
                    .collect::<Vec<_>>();
                staged
                    .get_mut(&token)
                    .expect("started candidate")
                    .1
                    .extend_from_slice(&bytes);
            }
            "read" => {
                let token: Uuid =
                    serde_json::from_value(call["token"].clone()).expect("read token");
                let offset = call["offset"].as_u64().expect("partial offset") as usize;
                let length = call["length"].as_u64().expect("partial length") as usize;
                let bytes = &staged.get(&token).expect("read candidate").1;
                let slice = if offset >= bytes.len() {
                    &[][..]
                } else {
                    &bytes[offset..(offset + length).min(bytes.len())]
                };
                reply["read_data"] = json!(base64::engine::general_purpose::STANDARD.encode(slice));
            }
            "commit" => {
                let token: Uuid =
                    serde_json::from_value(call["token"].clone()).expect("commit token");
                committed.push(token);
            }
            "abort" => {
                let token: Uuid =
                    serde_json::from_value(call["token"].clone()).expect("abort token");
                aborted.insert(token);
            }
            other => panic!("unexpected native action for {}: {other}", case.label),
        }
        write_frame(child.stdin.as_mut().expect("worker stdin"), &reply).expect("native reply");
    };
    drop(child.stdin.take());
    let output = child.wait_with_output().expect("worker cleanup");
    assert!(
        output.status.success() && output.stderr.is_empty(),
        "{} worker process",
        case.label
    );
    assert_eq!(terminal["op"], "complete", "{}: {terminal}", case.label);
    let mut artifacts = committed
        .into_iter()
        .map(|token| {
            assert!(!aborted.contains(&token), "committed token was aborted");
            staged.remove(&token).expect("committed candidate")
        })
        .collect::<Vec<_>>();
    artifacts.sort_by(|left, right| left.0.cmp(&right.0));
    WriterResult {
        artifacts,
        source_reads,
    }
}

fn combined(artifacts: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for (_, part) in artifacts {
        bytes.extend_from_slice(part);
    }
    bytes
}

fn assert_format_magic(case: CreateCase, bytes: &[u8]) {
    match case.format {
        ArchiveJobFormat::Zip => {
            let magic = if case.volume_bytes.is_some() {
                b"PK\x07\x08"
            } else {
                b"PK\x03\x04"
            };
            assert!(bytes.starts_with(magic), "{}", case.label);
        }
        ArchiveJobFormat::SevenZ => {
            assert!(bytes.starts_with(b"7z\xbc\xaf\x27\x1c"), "{}", case.label)
        }
        ArchiveJobFormat::Tar => {
            assert!(
                bytes.len() > 262 && &bytes[257..262] == b"ustar",
                "{}",
                case.label
            )
        }
        ArchiveJobFormat::Tgz | ArchiveJobFormat::Gz => {
            assert!(bytes.starts_with(&[0x1f, 0x8b]), "{}", case.label)
        }
        ArchiveJobFormat::Txz | ArchiveJobFormat::Xz => assert!(
            bytes.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]),
            "{}",
            case.label
        ),
    }
}

fn zip_local_header(bytes: &[u8]) -> (u16, u16, String) {
    let bytes = bytes.strip_prefix(b"PK\x07\x08").unwrap_or(bytes);
    assert!(bytes.len() >= 30 && bytes.starts_with(b"PK\x03\x04"));
    let flags = u16::from_le_bytes([bytes[6], bytes[7]]);
    let method = u16::from_le_bytes([bytes[8], bytes[9]]);
    let name_length = u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
    let name = std::str::from_utf8(&bytes[30..30 + name_length])
        .expect("UTF-8 local name")
        .to_owned();
    (flags, method, name)
}

#[test]
fn opt03_all_admitted_create_formats_reach_actual_writers_with_option_snapshot() {
    let owned = tempfile::tempdir().expect("owned create matrix");
    let mut executed = 0usize;
    for case in cases() {
        if case.format == ArchiveJobFormat::SevenZ && !cfg!(feature = "isolated-codecs") {
            continue;
        }
        let fixture = SourceFixture::build(owned.path(), case);
        let request = typed_request(case, fixture.paths.clone(), path_string(owned.path()));
        request
            .validate()
            .unwrap_or_else(|_| panic!("typed request rejected: {}", case.label));
        let worker = worker_request(case, fixture.paths[0].clone());
        worker
            .validate()
            .unwrap_or_else(|_| panic!("worker request rejected: {}", case.label));
        let result = run_writer(case, &worker, &fixture);
        assert!(!result.artifacts.is_empty(), "{}", case.label);
        if let Some(maximum) = case.volume_bytes {
            assert!(result.artifacts.len() >= 2, "{}", case.label);
            assert!(
                result
                    .artifacts
                    .iter()
                    .all(|(_, bytes)| bytes.len() as u64 <= maximum),
                "{}",
                case.label
            );
        } else {
            assert_eq!(result.artifacts.len(), 1, "{}", case.label);
            assert_eq!(result.artifacts[0].0, case.output, "{}", case.label);
        }
        let encoded = combined(&result.artifacts);
        assert_format_magic(case, &encoded);
        if case.label == "zip-store-utf8-strip-exclude" {
            let (flags, method, name) = zip_local_header(&encoded);
            assert_ne!(flags & (1 << 11), 0, "ZIP Unicode name flag");
            assert_eq!(method, 0, "Store must reach the writer");
            assert_eq!(name, "资料.bin", "single root must be stripped");
            assert!(
                result.source_reads.contains(&fixture.id_named("资料.bin"))
                    && !result.source_reads.contains(&fixture.id_named("Thumbs.db")),
                "excluded source must not reach payload reads"
            );
        }
        if case.label == "zip-deflate-aes-split" {
            let (flags, method, _) = zip_local_header(&encoded);
            assert_ne!(flags & 1, 0, "AES member must be encrypted");
            assert_eq!(
                method, 99,
                "AES wrapper must retain Deflate as its inner method"
            );
            assert!(
                result
                    .artifacts
                    .first()
                    .is_some_and(|(name, _)| name.ends_with(".z01"))
                    && result
                        .artifacts
                        .last()
                        .is_some_and(|(name, _)| name.ends_with(".zip")),
                "genuine ZIP family names"
            );
        }
        #[cfg(feature = "isolated-codecs")]
        if case.label == "7z-maximum-solid-aes-header-split" {
            assert!(
                result
                    .artifacts
                    .iter()
                    .enumerate()
                    .all(|(index, (name, _))| name.ends_with(&format!(".{:03}", index + 1))),
                "7z split names"
            );
            assert!(
                sevenz_rust2::ArchiveReader::new(
                    std::io::Cursor::new(encoded.clone()),
                    sevenz_rust2::Password::empty(),
                )
                .is_err(),
                "header-encrypted 7z must not open without its password"
            );
            let reader = sevenz_rust2::ArchiveReader::new(
                std::io::Cursor::new(encoded),
                sevenz_rust2::Password::from(SECRET),
            )
            .expect("password opens created 7z");
            assert!(
                reader.archive().is_solid,
                "solid choice must reach the writer"
            );
        }
        executed += 1;
    }
    assert_eq!(
        executed,
        if cfg!(feature = "isolated-codecs") {
            9
        } else {
            7
        }
    );
}

#[test]
fn opt03_invalid_create_combinations_fail_at_typed_and_writer_boundaries() {
    let all = cases();
    let zip = all[0];
    let sevenz = all[2];
    let tar = all[4];
    let tgz = all[5];
    let gzip = all[7];
    let xz = all[8];
    let paths = vec![r"C:\owned\source.bin".to_owned()];
    let destination = r"C:\owned".to_owned();

    let mut invalid = typed_request(zip, paths.clone(), destination.clone());
    let create = invalid.create.as_mut().expect("ZIP create");
    create.level = Some(ArchiveJobLevel::Normal);
    create.method = Some(ArchiveJobMethod::Store);
    assert!(invalid.validate().is_err(), "ZIP level/method mismatch");

    let mut invalid = typed_request(sevenz, paths.clone(), destination.clone());
    let create = invalid.create.as_mut().expect("7z create");
    create.method = Some(ArchiveJobMethod::Deflate);
    assert!(invalid.validate().is_err(), "7z cannot receive ZIP method");

    let mut invalid = typed_request(sevenz, paths.clone(), destination.clone());
    let create = invalid.create.as_mut().expect("7z create");
    create.password = None;
    create.encrypt_header = Some(true);
    assert!(
        invalid.validate().is_err(),
        "header encryption requires password"
    );

    let mut invalid = typed_request(tar, paths.clone(), destination.clone());
    invalid.create.as_mut().expect("TAR create").level = Some(ArchiveJobLevel::Normal);
    assert!(invalid.validate().is_err(), "plain TAR has no level");

    let mut invalid = typed_request(tgz, paths.clone(), destination.clone());
    invalid.create.as_mut().expect("TGZ create").password = Some(PrivatePassword::new(SECRET));
    assert!(invalid.validate().is_err(), "TGZ has no encryption");

    let mut invalid = typed_request(gzip, paths.clone(), destination.clone());
    invalid.paths.push(r"C:\owned\second.bin".into());
    assert!(invalid.validate().is_err(), "GZ is one file per request");

    let mut invalid = typed_request(xz, paths.clone(), destination.clone());
    invalid
        .create
        .as_mut()
        .expect("XZ create")
        .strip_single_root = true;
    assert!(invalid.validate().is_err(), "XZ has no tree options");

    let mut invalid = typed_request(tar, paths.clone(), destination.clone());
    invalid.create.as_mut().expect("TAR create").volume_bytes = Some(65536);
    assert!(invalid.validate().is_err(), "TAR has no split writer");

    let mut invalid = typed_request(zip, paths.clone(), destination);
    invalid.create.as_mut().expect("ZIP create").volume_bytes = Some(65535);
    assert!(invalid.validate().is_err(), "split size is bounded");

    let mut invalid = worker_request(zip, paths[0].clone());
    invalid.compression_level = Some("normal".into());
    invalid.zip_options.as_mut().expect("ZIP options").method = "store".into();
    assert!(invalid.validate().is_err(), "writer rejects ZIP mismatch");

    let mut invalid = worker_request(sevenz, paths[0].clone());
    invalid.password = None;
    let options = invalid.sevenz_options.as_mut().expect("7z options");
    options.encryption = "none".into();
    options.encrypt_header = true;
    assert!(
        invalid.validate().is_err(),
        "writer rejects header without AES password"
    );

    let mut invalid = worker_request(tar, paths[0].clone());
    invalid.password = Some(SECRET.into());
    assert!(invalid.validate().is_err(), "writer rejects TAR password");

    let mut invalid = worker_request(tgz, paths[0].clone());
    invalid.tree_options = None;
    assert!(
        invalid.validate().is_err(),
        "tree writer requires tree options"
    );

    let mut invalid = worker_request(gzip, paths[0].clone());
    invalid.tree_options = Some(CreationOptions::default());
    assert!(
        invalid.validate().is_err(),
        "stream writer rejects tree options"
    );

    let mut invalid = worker_request(tar, paths[0].clone());
    invalid.volume_bytes = Some(65536);
    assert!(invalid.validate().is_err(), "writer rejects TAR split");
}
