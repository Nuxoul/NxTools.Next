use nxtools_archive::{
    ArchiveEngine, ArchiveError, ArchiveRequest, OpenRequest, OperationControl, zip_edit::ZipEdit,
};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Seek, Write},
    path::{Path, PathBuf},
};
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn save(mut file: File, path: &Path) {
    file.rewind().expect("rewind");
    std::io::copy(
        &mut file,
        &mut File::create(path).expect("candidate fixture"),
    )
    .expect("copy");
}
fn zip(path: &Path) {
    let mut out = zip::ZipWriter::new(File::create(path).expect("owned ZIP"));
    out.start_file("z-last.txt", zip::write::SimpleFileOptions::default())
        .expect("entry");
    out.write_all(b"last").expect("data");
    out.start_file("a-first.txt", zip::write::SimpleFileOptions::default())
        .expect("entry");
    out.write_all(b"first").expect("data");
    out.finish().expect("finish");
}
#[test]
fn ordinary_zip_edit_actions_use_original_ids_and_merge_existing_encoder_records() {
    let root = tempfile::tempdir().expect("owned");
    let source = root.path().join("edit.zip");
    zip(&source);
    let before = fs::read(&source).expect("original");
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(&source), &control)
        .expect("snapshot");
    let rename = snapshot.entries()[0].id();
    let delete = snapshot.entries()[1].id();
    let additions_path = root.path().join("add.zip");
    let mut additions = zip::ZipWriter::new(File::create(&additions_path).expect("add candidate"));
    additions
        .start_file("added/file.txt", zip::write::SimpleFileOptions::default())
        .expect("add entry");
    additions.write_all(b"added").expect("data");
    additions.finish().expect("finish");
    let candidate = engine
        .zip_edit_candidate(
            &snapshot,
            &[
                ZipEdit::Rename {
                    entry: rename,
                    name: "renamed.txt".into(),
                },
                ZipEdit::Delete {
                    entries: vec![delete],
                },
                ZipEdit::NewDirectory {
                    name: "empty/".into(),
                },
                ZipEdit::Comment {
                    text: "new-comment".into(),
                },
            ],
            Some(&mut File::open(additions_path).expect("encoded additions")),
            &control,
        )
        .expect("all ordinary ZIP actions");
    let final_path = root.path().join("candidate.zip");
    save(candidate, &final_path);
    let read = engine
        .open(OpenRequest::new(&final_path), &control)
        .expect("candidate snapshot");
    assert_eq!(read.comment(), "new-comment");
    assert_eq!(
        read.entries()
            .iter()
            .map(|v| v.display_path())
            .collect::<Vec<_>>(),
        vec!["renamed.txt", "empty/", "added/file.txt"]
    );
    assert_eq!(
        engine
            .execute(ArchiveRequest::test(read), &control)
            .expect("full candidate verify")
            .bytes_verified(),
        9
    );
    assert_eq!(fs::read(source).expect("unchanged source"), before);
}
#[test]
fn encrypted_unchanged_local_records_and_flags_survive_raw_rewrite() {
    let root = tempfile::tempdir().expect("owned");
    let source = root.path().join("crypto.zip");
    fs::copy(fixture("independent-zipcrypto-dd.zip"), &source).expect("fixture");
    let engine = ArchiveEngine::default().with_password("synthetic-private-ZIP-secret");
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(&source), &control)
        .expect("encrypted snapshot");
    let original = fs::read(&source).expect("bytes");
    let first_central = original
        .windows(4)
        .position(|v| v == b"PK\x01\x02")
        .expect("central");
    let candidate = engine
        .zip_edit_candidate(
            &snapshot,
            &[ZipEdit::Comment {
                text: "unchanged payload".into(),
            }],
            None,
            &control,
        )
        .expect("encrypted raw preservation");
    let path = root.path().join("result.zip");
    save(candidate, &path);
    let result = fs::read(&path).expect("candidate bytes");
    assert_eq!(
        &result[..first_central],
        &original[..first_central],
        "all local/encryption/data/descriptor bytes exact"
    );
    let read = engine
        .open(OpenRequest::new(path), &control)
        .expect("candidate");
    assert!(read.entries().iter().all(|v| v.encrypted()));
    assert_eq!(
        engine
            .execute(ArchiveRequest::test(read), &control)
            .expect("crypto integrity")
            .bytes_verified(),
        1050
    );
}
#[test]
fn stale_cancel_and_application_container_edit_refusals_preserve_source() {
    let root = tempfile::tempdir().expect("owned");
    let path = root.path().join("ordinary.zip");
    zip(&path);
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(&path), &control)
        .expect("snapshot");
    let original = fs::read(&path).expect("original");
    let hash = Sha256::digest(&original);
    control.cancel();
    assert!(matches!(
        engine.zip_edit_candidate(
            &snapshot,
            &[ZipEdit::Comment { text: "x".into() }],
            None,
            &control
        ),
        Err(ArchiveError::Cancelled)
    ));
    assert_eq!(Sha256::digest(fs::read(&path).expect("unchanged")), hash);
    let control = OperationControl::default();
    fs::write(&path, b"replaced").expect("owned replacement");
    assert!(matches!(
        engine.zip_edit_candidate(
            &snapshot,
            &[ZipEdit::Comment { text: "x".into() }],
            None,
            &control
        ),
        Err(ArchiveError::SourceChanged)
    ));
    let jar = root.path().join("application.jar");
    fs::write(&jar, &original).expect("jar");
    let snapshot = engine
        .open(OpenRequest::new(&jar), &control)
        .expect("ZIP container browse");
    assert!(matches!(
        engine.zip_edit_candidate(
            &snapshot,
            &[ZipEdit::Comment { text: "x".into() }],
            None,
            &control
        ),
        Err(ArchiveError::UnsupportedOperation)
    ));
}

fn raw_metadata_zip(path: &Path, unknown: bool) {
    let name = b"metadata.txt";
    let payload = b"metadata-payload";
    let crc = crc32fast::hash(payload);
    let local_extra = if unknown {
        vec![0xef, 0xbe, 1, 0, 7]
    } else {
        vec![
            0x55, 0x54, 9, 0, 3, 0, 0xe1, 0x0b, 0x5e, 1, 0xe1, 0x0b, 0x5e,
        ]
    };
    let central_extra = if unknown {
        vec![0xef, 0xbe, 1, 0, 7]
    } else {
        vec![0x55, 0x54, 5, 0, 1, 0, 0xe1, 0x0b, 0x5e]
    };
    let mut local = vec![0u8; 30];
    local[..4].copy_from_slice(b"PK\x03\x04");
    local[4..6].copy_from_slice(&20u16.to_le_bytes());
    local[12..14].copy_from_slice(&0x5021u16.to_le_bytes());
    local[14..18].copy_from_slice(&crc.to_le_bytes());
    local[18..22].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    local[22..26].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    local[26..28].copy_from_slice(&(name.len() as u16).to_le_bytes());
    local[28..30].copy_from_slice(&(local_extra.len() as u16).to_le_bytes());
    local.extend(name);
    local.extend(local_extra);
    local.extend(payload);
    let offset = local.len();
    let mut central = vec![0u8; 46];
    central[..4].copy_from_slice(b"PK\x01\x02");
    central[4..6].copy_from_slice(&20u16.to_le_bytes());
    central[6..8].copy_from_slice(&20u16.to_le_bytes());
    central[14..16].copy_from_slice(&0x5021u16.to_le_bytes());
    central[16..20].copy_from_slice(&crc.to_le_bytes());
    central[20..24].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    central[24..28].copy_from_slice(&(payload.len() as u32).to_le_bytes());
    central[28..30].copy_from_slice(&(name.len() as u16).to_le_bytes());
    central[30..32].copy_from_slice(&(central_extra.len() as u16).to_le_bytes());
    central[32..34].copy_from_slice(&5u16.to_le_bytes());
    central[36..38].copy_from_slice(&1u16.to_le_bytes());
    central[38..42].copy_from_slice(&0x23u32.to_le_bytes());
    central.extend(name);
    central.extend(central_extra);
    central.extend(b"entry");
    let mut end = vec![0u8; 22];
    end[..4].copy_from_slice(b"PK\x05\x06");
    end[8..10].copy_from_slice(&1u16.to_le_bytes());
    end[10..12].copy_from_slice(&1u16.to_le_bytes());
    end[12..16].copy_from_slice(&(central.len() as u32).to_le_bytes());
    end[16..20].copy_from_slice(&(offset as u32).to_le_bytes());
    local.extend(central);
    local.extend(end);
    fs::write(path, local).expect("independent metadata ZIP");
}
#[test]
fn distinct_local_central_metadata_attrs_and_unknown_rule_are_enforced() {
    let root = tempfile::tempdir().expect("owned");
    let path = root.path().join("metadata.zip");
    raw_metadata_zip(&path, false);
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(&path), &control)
        .expect("metadata ZIP");
    let original = fs::read(&path).expect("original");
    let candidate = engine
        .zip_edit_candidate(
            &snapshot,
            &[ZipEdit::Comment { text: "new".into() }],
            None,
            &control,
        )
        .expect("metadata preserving comment");
    let target = root.path().join("result.zip");
    save(candidate, &target);
    let bytes = fs::read(&target).expect("result");
    assert_eq!(
        &bytes[..original.len() - 22],
        &original[..original.len() - 22],
        "distinct local+central extras,attributes,entry comment exact"
    );
    raw_metadata_zip(&path, true);
    let snapshot = engine
        .open(OpenRequest::new(&path), &control)
        .expect("unknown extra browse");
    let hash = Sha256::digest(fs::read(&path).expect("original"));
    assert!(matches!(
        engine.zip_edit_candidate(
            &snapshot,
            &[ZipEdit::Rename {
                entry: snapshot.entries()[0].id(),
                name: "other.txt".into()
            }],
            None,
            &control
        ),
        Err(ArchiveError::UnsupportedOperation)
    ));
    assert_eq!(Sha256::digest(fs::read(&path).expect("retained")), hash);
}
#[test]
fn aes_and_zipcrypto_rename_preserves_packed_bytes_and_encryption_metadata() {
    let root = tempfile::tempdir().expect("owned");
    for (index, name) in ["independent-zipcrypto-dd.zip", "combo-ae2-store-256.zip"]
        .iter()
        .enumerate()
    {
        let path = root.path().join(format!("source{index}.zip"));
        fs::copy(fixture(name), &path).expect("independent crypto input");
        let engine = ArchiveEngine::default().with_password("synthetic-private-ZIP-secret");
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(&path), &control)
            .expect("snapshot");
        let mut before = zip::ZipArchive::new(File::open(&path).expect("source"))
            .expect("independent ZIP header reader");
        let source = before.by_index_raw(0).expect("raw member");
        let at = source.data_start().expect("raw member data offset") as usize;
        let size = source.compressed_size() as usize;
        let original = fs::read(&path).expect("source bytes");
        let candidate = engine
            .zip_edit_candidate(
                &snapshot,
                &[ZipEdit::Rename {
                    entry: snapshot.entries()[0].id(),
                    name: "renamed.txt".into(),
                }],
                None,
                &control,
            )
            .expect("encrypted rename");
        let target = root.path().join(format!("renamed{index}.zip"));
        save(candidate, &target);
        let mut reader = zip::ZipArchive::new(File::open(&target).expect("candidate"))
            .expect("independent header reader");
        let entry = reader.by_index_raw(0).expect("raw member");
        assert!(entry.encrypted());
        assert_eq!(entry.name(), "renamed.txt");
        let new_at = entry.data_start().expect("raw candidate data offset") as usize;
        let bytes = fs::read(&target).expect("candidate bytes");
        assert_eq!(
            &bytes[new_at..new_at + size],
            &original[at..at + size],
            "raw encryption header/ciphertext/MAC unchanged"
        );
        let snapshot = engine
            .open(OpenRequest::new(target), &control)
            .expect("candidate snapshot");
        engine
            .execute(ArchiveRequest::test(snapshot), &control)
            .expect("full original crypto integrity");
    }
}

#[test]
fn tiny_forced_zip64_local_record_survives_rename_and_comment() {
    let root = tempfile::tempdir().expect("owned");
    let source = root.path().join("forced.zip");
    let mut writer = zip::ZipWriter::new(File::create(&source).expect("owned ZIP64"));
    writer
        .start_file(
            "old.txt",
            zip::write::SimpleFileOptions::default().large_file(true),
        )
        .expect("forced ZIP64 entry");
    writer.write_all(b"tiny ZIP64").expect("tiny data");
    writer.finish().expect("finish");
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(&source), &control)
        .expect("original ZIP64");
    let candidate = engine
        .zip_edit_candidate(
            &snapshot,
            &[
                ZipEdit::Rename {
                    entry: snapshot.entries()[0].id(),
                    name: "longer-renamed.txt".into(),
                },
                ZipEdit::Comment {
                    text: "zip64-comment".into(),
                },
            ],
            None,
            &control,
        )
        .expect("record rewrite");
    let output = root.path().join("edited.zip");
    save(candidate, &output);
    let mut independent = zip::ZipArchive::new(File::open(&output).expect("independent source"))
        .expect("independent ZIP64 read");
    let mut entry = independent
        .by_name("longer-renamed.txt")
        .expect("renamed original");
    let mut payload = Vec::new();
    std::io::Read::read_to_end(&mut entry, &mut payload).expect("independent payload");
    assert_eq!(payload, b"tiny ZIP64");
    let snapshot = engine
        .open(OpenRequest::new(&output), &control)
        .expect("rewritten snapshot");
    assert_eq!(snapshot.comment(), "zip64-comment");
    assert_eq!(
        engine
            .execute(ArchiveRequest::test(snapshot), &control)
            .expect("full validation")
            .bytes_verified(),
        10
    );
}

#[test]
fn capability_foundation_opened_zip_reports_implemented_reads_and_edits_not_suffix() {
    let root = tempfile::tempdir().expect("owned fixture");
    let path = root.path().join("content-not-extension.tar");
    fs::copy(fixture("stored.zip"), &path).expect("independent ZIP");
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(&path), &OperationControl::default())
        .expect("content-detected ZIP");
    assert!(
        snapshot.capabilities().extract,
        "implemented ZIP extraction is independent of editing"
    );
    assert!(
        snapshot.capabilities().modify,
        "ordinary opened ZIP uses its implemented record editor, not its suffix"
    );
}

#[test]
fn capability_foundation_independent_operation_matrix_and_truthful_refusals() {
    use nxtools_archive::{
        ArchiveCapabilities, ArchiveFormat as F, ArchiveModificationRefusal as R,
        ArchiveOperation as O,
    };
    for format in [
        F::Zip,
        F::SevenZ,
        F::Rar4,
        F::Rar5,
        F::Tar,
        F::TarGz,
        F::TarBz2,
        F::TarXz,
        F::TarZstd,
        F::Gzip,
        F::Bzip2,
        F::Xz,
        F::Zstd,
    ] {
        let caps = ArchiveCapabilities::opened(format, None);
        for op in [O::Browse, O::Extract, O::Preview] {
            assert_eq!(caps.admit(op), Ok(()));
        }
        for op in [
            O::Add,
            O::Delete,
            O::RenameFile,
            O::RenameDirectorySubtree,
            O::NewDirectory,
            O::Comment,
        ] {
            assert_eq!(
                caps.admit(op),
                if format == F::Zip
                    || matches!(format, F::SevenZ | F::Tar | F::TarGz | F::TarXz)
                        && op != O::Comment
                {
                    Ok(())
                } else {
                    Err(R::BackendMissing)
                }
            );
        }
        assert_eq!(
            caps.admit(O::ReplaceContent),
            if matches!(format, F::SevenZ | F::Tar | F::TarGz | F::TarXz) {
                Ok(())
            } else {
                Err(R::BackendMissing)
            }
        );
        assert!(
            !caps.create,
            "an opened archive is not a create-format registry"
        );
    }
    for refusal in [
        R::SnapshotNotReady,
        R::SourceChanged,
        R::Volumes,
        R::SelfExtracting,
        R::ApplicationContainer,
        R::UnsupportedEncryption,
        R::UnsupportedStructure,
        R::Corrupt,
        R::UnverifiedMetadata,
        R::SourceUnavailable,
    ] {
        let caps = ArchiveCapabilities::opened(F::Zip, Some(refusal));
        assert_eq!(caps.admit(O::Add), Err(refusal));
        assert_eq!(
            caps.admit(O::Browse),
            Ok(()),
            "write refusal never globally breaks reading"
        );
    }
}

#[test]
fn capability_foundation_unknown_metadata_stale_revision_and_positive_original_zip() {
    use nxtools_archive::{ArchiveModificationRefusal as R, ArchiveOperation as O};
    let root = tempfile::tempdir().expect("owned fixture");
    let path = root.path().join("fixture.zip");
    raw_metadata_zip(&path, false);
    let control = OperationControl::default();
    let engine = ArchiveEngine::default();
    let snapshot = engine
        .open(OpenRequest::new(&path), &control)
        .expect("known metadata");
    assert_eq!(snapshot.capabilities().admit(O::Add), Ok(()));
    let metadata = fs::metadata(&path).expect("version");
    let binding = nxtools_protocol::archive_browser::ArchiveSourceBinding {
        native: "v1:fixture".into(),
        digest: Sha256::digest(fs::read(&path).expect("bytes")).into(),
        size: metadata.len(),
        modified_unix_ns: metadata
            .modified()
            .expect("mtime")
            .duration_since(std::time::UNIX_EPOCH)
            .expect("unix")
            .as_nanos() as u64,
    };
    assert_eq!(
        engine.inspect_modification_source(OpenRequest::new(&path), &binding, &control),
        Ok(())
    );
    raw_metadata_zip(&path, true);
    assert_eq!(
        engine.modification_eligibility(&snapshot, &control),
        Err(R::SourceChanged)
    );
    assert_eq!(
        engine.inspect_modification_source(OpenRequest::new(&path), &binding, &control),
        Err(R::SourceChanged)
    );
    let unknown = engine
        .open(OpenRequest::new(&path), &control)
        .expect("unknown metadata browsable");
    assert_eq!(
        unknown.capabilities().admit(O::Add),
        Err(R::UnverifiedMetadata)
    );
    assert!(
        unknown.capabilities().browse
            && unknown.capabilities().extract
            && unknown.capabilities().preview
    );
    let before = fs::read(&path).expect("before");
    assert!(matches!(
        engine.zip_edit_candidate(
            &unknown,
            &[ZipEdit::Comment { text: "x".into() }],
            None,
            &control
        ),
        Err(ArchiveError::UnsupportedOperation)
    ));
    assert_eq!(fs::read(&path).expect("retained"), before);
}

#[test]
fn capability_foundation_delete_unknown_record_preserves_original_zip_edit_compatibility() {
    let root = tempfile::tempdir().expect("owned");
    let path = root.path().join("unknown.zip");
    raw_metadata_zip(&path, true);
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(&path), &control)
        .expect("readable ZIP");
    assert_eq!(
        snapshot
            .capabilities()
            .admit(nxtools_archive::ArchiveOperation::Delete),
        Ok(()),
        "metadata policy must not downgrade original delete-all-unsupported path"
    );
    let original = fs::read(&path).expect("source");
    let candidate = engine
        .zip_edit_candidate(
            &snapshot,
            &[ZipEdit::Delete {
                entries: vec![snapshot.entries()[0].id()],
            }],
            None,
            &control,
        )
        .expect("existing record-preserving delete");
    let target = root.path().join("deleted.zip");
    save(candidate, &target);
    assert!(
        engine
            .open(OpenRequest::new(target), &control)
            .expect("valid empty result")
            .entries()
            .is_empty()
    );
    assert_eq!(fs::read(path).expect("retained source"), original);
}
