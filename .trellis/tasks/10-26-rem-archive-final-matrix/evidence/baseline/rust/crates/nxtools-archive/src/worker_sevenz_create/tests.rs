//! Real create -> held-partial readback -> verifier -> commit/abort transaction.
//! Only the owned verification File's Read/Seek is adapted; no product wire fault flags.
use super::*;
use crate::worker::{SevenzCreateOptions, read_frame};
use crate::worker_output::NativeReply;
use base64::Engine as _;
use serde_json::{Value, json};
use std::io::{self, Cursor, SeekFrom};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Point {
    Signature,
    BudgetHeader,
    ParserHeader,
    Payload,
    Seek,
}
#[derive(Clone, Copy, Debug)]
enum Fault {
    None,
    Storage(Point, io::ErrorKind),
    Typed(Point, ArchiveError),
    Corrupt,
    Truncated,
    PayloadDamage,
}
struct Input {
    file: File,
    fault: Fault,
    next_header: u64,
    signature_reads: usize,
    hit: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl Input {
    fn fail(&self, point: Point) -> io::Result<()> {
        let error = match self.fault {
            Fault::Storage(at, kind) if at == point => io::Error::from(kind),
            Fault::Typed(at, error) if at == point => io::Error::other(error),
            _ => return Ok(()),
        };
        self.hit.store(true, std::sync::atomic::Ordering::SeqCst);
        Err(error)
    }
}
impl Read for Input {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        let position = self.file.stream_position()?;
        if position == 0 {
            self.signature_reads += 1;
            self.fail(if self.signature_reads == 1 {
                Point::Signature
            } else {
                Point::ParserHeader
            })?;
        }
        if position == self.next_header {
            self.fail(Point::BudgetHeader)?;
        }
        if position == 32 {
            self.fail(Point::Payload)?;
        }
        self.file.read(bytes)
    }
}
impl Seek for Input {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        self.fail(Point::Seek)?;
        self.file.seek(from)
    }
}

// A bounded synchronous native rendezvous with real owned source/partial/final files.
// C# integration separately checks the protected Windows handle/Job implementation.
struct Native {
    root: tempfile::TempDir,
    replies: ReplyPort,
    frame: Vec<u8>,
    source: Uuid,
    token: Uuid,
    active_source: Option<File>,
    partial: Option<File>,
    opens: usize,
    closes: usize,
    commits: usize,
    aborts: Vec<Uuid>,
    readback: bool,
}
impl Native {
    fn new(replies: ReplyPort, existing: bool) -> Self {
        let root = tempfile::tempdir().expect("owned verifier fixture");
        std::fs::write(root.path().join("source.bin"), b"payload").expect("source");
        std::fs::write(root.path().join("foreign.partial"), b"not ours").expect("foreign partial");
        if existing {
            std::fs::write(root.path().join("created.7z"), b"previous target").expect("old target");
        }
        Self {
            root,
            replies,
            frame: Vec::new(),
            source: Uuid::new_v4(),
            token: Uuid::new_v4(),
            active_source: None,
            partial: None,
            opens: 0,
            closes: 0,
            commits: 0,
            aborts: Vec::new(),
            readback: false,
        }
    }
    fn token(&self, call: &Value) {
        assert_eq!(
            call["token"],
            json!(self.token),
            "exact owned partial token"
        );
    }
}
impl Write for Native {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        assert!(self.frame.len() + bytes.len() <= crate::worker::MAX_FRAME_BYTES + 4);
        self.frame.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        let frame: Value =
            read_frame(&mut Cursor::new(std::mem::take(&mut self.frame)))?.expect("native frame");
        let call = &frame["native"];
        let mut reply = NativeReply {
            call_id: serde_json::from_value(call["call_id"].clone()).expect("call id"),
            success: true,
            token: None,
            error_code: None,
            read_data: None,
            existing_bytes: None,
        };
        match call["action"].as_str().expect("native action") {
            "creation_planning" | "source_validated" | "creation_encoding" | "creation_verifying" | "source_precommit" | "source_check" => {},
            "source_roots" => reply.read_data = Some(serde_json::to_string(&json!([{"id":self.source,"name":"source.bin","directory":false,"size":7,"dos_modified":null}])).expect("source metadata")),
            "source_open" => {
                assert!(self.active_source.is_none());
                assert_eq!(call["token"], json!(self.source));
                self.active_source = Some(File::open(self.root.path().join("source.bin"))?);
                self.opens += 1;
            },
            "source_read" => {
                assert_eq!(call["token"], json!(self.source));
                let source = self.active_source.as_mut().expect("source still open");
                source.seek(SeekFrom::Start(call["offset"].as_u64().expect("offset")))?;
                let length = call["length"].as_u64().expect("length") as usize;
                assert!(length <= 65536);
                let mut bytes = vec![0; length];
                let count = source.read(&mut bytes)?;
                reply.read_data = Some(base64::engine::general_purpose::STANDARD.encode(&bytes[..count]));
            },
            "source_close" => {
                assert_eq!(call["token"], json!(self.source));
                assert!(self.active_source.take().is_some());
                self.closes += 1;
            },
            "begin" => {
                assert_eq!(call["path"], "created.7z");
                assert!(self.partial.is_none());
                self.partial = Some(File::options().create_new(true).read(true).write(true).open(self.root.path().join("owned.partial"))?);
                reply.token = Some(self.token);
            },
            "write" => {
                self.token(call);
                let bytes: Vec<u8> = serde_json::from_value(call["data"].clone()).expect("bounded output bytes");
                assert!(bytes.len() <= 65536);
                self.partial.as_mut().expect("held partial").write_all(&bytes)?;
            },
            "read" => {
                self.token(call);
                self.readback = true;
                let length = call["length"].as_u64().expect("length") as usize;
                assert!(length <= 65536);
                let partial = self.partial.as_mut().expect("same held partial");
                partial.seek(SeekFrom::Start(call["offset"].as_u64().expect("offset")))?;
                let mut bytes = vec![0; length];
                let count = partial.read(&mut bytes)?;
                reply.read_data = Some(base64::engine::general_purpose::STANDARD.encode(&bytes[..count]));
            },
            "commit" => {
                self.token(call);
                assert!(self.readback && self.active_source.is_none());
                drop(self.partial.take().expect("owned partial"));
                std::fs::rename(self.root.path().join("owned.partial"), self.root.path().join("created.7z"))?;
                self.commits += 1;
            },
            "abort" => {
                self.token(call);
                assert!(self.active_source.is_none());
                drop(self.partial.take().expect("exact held partial"));
                std::fs::remove_file(self.root.path().join("owned.partial"))?;
                self.aborts.push(self.token);
            },
            other => panic!("unexpected native action {other}"),
        }
        assert!(
            self.replies.deliver(reply),
            "one outstanding acknowledged call"
        );
        Ok(())
    }
}
fn transaction(fault: Fault, expected: Result<(u64, u64)>, existing: bool, encrypted: bool) {
    let replies = ReplyPort::new();
    let mut native = Native::new(replies.clone(), existing);
    let mut request = WorkerRequest::open(Uuid::new_v4(), "no ordinary source authority".into());
    request.version = 2;
    request.operation = "create_7z".into();
    request.create_name = Some("created.7z".into());
    request.compression_level = Some("fast".into());
    request.tree_options = Some(crate::creation::CreationOptions::default());
    request.sevenz_options = Some(SevenzCreateOptions {
        solid: false,
        encryption: if encrypted { "aes256" } else { "none" }.into(),
        encrypt_header: encrypted,
    });
    if encrypted {
        request.password = Some("synthetic verifier secret".into());
    }
    request.validate().expect("real valid create request");
    let hit = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let result = create_with_reader(
        &request,
        &OperationControl::default(),
        &mut native,
        replies,
        |mut file| {
            // The encoder and full held-partial hash/length comparison have already succeeded.
            let mut header = [0; 32];
            file.read_exact(&mut header)
                .expect("valid encoded signature");
            assert_eq!(&header[..6], b"7z\xbc\xaf\x27\x1c");
            let next_header = 32 + u64::from_le_bytes(header[12..20].try_into().expect("offset"));
            match fault {
                Fault::Corrupt => {
                    file.rewind().expect("rewind");
                    file.write_all(b"bad").expect("corrupt signature");
                }
                Fault::Truncated => file.set_len(12).expect("truncate verifier spool"),
                Fault::PayloadDamage => {
                    file.seek(SeekFrom::Start(32)).expect("payload");
                    file.write_all(&[0xff]).expect("payload damage");
                }
                _ => {}
            }
            file.rewind().expect("verifier start");
            Input {
                file,
                fault,
                next_header,
                signature_reads: 0,
                hit: hit.clone(),
            }
        },
    );
    // Check transaction cleanup even while the old category assertion is red.
    assert!(native.readback, "real full held-partial readback reached");
    assert_eq!((native.opens, native.closes), (1, 1));
    assert!(native.active_source.is_none() && native.partial.is_none());
    assert!(!native.root.path().join("owned.partial").exists());
    assert_eq!(
        std::fs::read(native.root.path().join("source.bin")).expect("source preserved"),
        b"payload"
    );
    assert_eq!(
        std::fs::read(native.root.path().join("foreign.partial"))
            .expect("foreign partial preserved"),
        b"not ours"
    );
    if expected.is_err() {
        assert_eq!(native.commits, 0);
        assert_eq!(native.aborts, [native.token]);
        if existing {
            assert_eq!(
                std::fs::read(native.root.path().join("created.7z")).expect("old target retained"),
                b"previous target"
            );
        } else {
            assert!(!native.root.path().join("created.7z").exists());
        }
    } else {
        assert_eq!(native.commits, 1);
        assert!(native.aborts.is_empty());
        assert!(native.root.path().join("created.7z").exists());
    }
    if matches!(fault, Fault::Storage(..) | Fault::Typed(..)) {
        assert!(
            hit.load(std::sync::atomic::Ordering::SeqCst),
            "requested real input boundary reached: {fault:?}"
        );
    }
    assert_eq!(
        result, expected,
        "verifier transaction: {fault:?}, existing={existing}, encrypted={encrypted}"
    );
}
#[test]
fn signature_storage_failure_is_io_failure_and_aborts_owned_partial() {
    for existing in [false, true] {
        transaction(
            Fault::Storage(Point::Signature, io::ErrorKind::Other),
            Err(ArchiveError::IoFailure),
            existing,
            false,
        );
    }
}
#[test]
fn budget_header_storage_failure_is_io_failure_and_aborts_owned_partial() {
    transaction(
        Fault::Storage(Point::BudgetHeader, io::ErrorKind::Other),
        Err(ArchiveError::IoFailure),
        true,
        false,
    );
}
#[test]
fn parser_header_storage_failure_is_io_failure_and_aborts_owned_partial() {
    transaction(
        Fault::Storage(Point::ParserHeader, io::ErrorKind::InvalidData),
        Err(ArchiveError::IoFailure),
        false,
        true,
    );
}
#[test]
fn payload_storage_failure_is_io_failure_and_aborts_owned_partial() {
    for existing in [false, true] {
        transaction(
            Fault::Storage(Point::Payload, io::ErrorKind::Other),
            Err(ArchiveError::IoFailure),
            existing,
            false,
        );
    }
}
#[test]
fn corrupt_truncated_and_payload_damage_stay_corrupt_without_publication() {
    for fault in [Fault::Corrupt, Fault::Truncated, Fault::PayloadDamage] {
        transaction(fault, Err(ArchiveError::CorruptArchive), true, false);
    }
}
#[test]
fn normal_creation_still_verifies_and_publishes() {
    transaction(Fault::None, Ok((1, 7)), false, false);
}
#[test]
fn typed_and_resource_input_failures_survive_header_payload_and_seek_wrappers() {
    for point in [
        Point::Signature,
        Point::BudgetHeader,
        Point::ParserHeader,
        Point::Payload,
        Point::Seek,
    ] {
        for expected in [
            ArchiveError::Cancelled,
            ArchiveError::ResourceLimit,
            ArchiveError::SourceChanged,
        ] {
            transaction(Fault::Typed(point, expected), Err(expected), true, false);
        }
        transaction(
            Fault::Storage(point, io::ErrorKind::OutOfMemory),
            Err(ArchiveError::ResourceLimit),
            false,
            false,
        );
    }
    transaction(
        Fault::Storage(Point::Payload, io::ErrorKind::Other),
        Err(ArchiveError::IoFailure),
        true,
        true,
    );
}
