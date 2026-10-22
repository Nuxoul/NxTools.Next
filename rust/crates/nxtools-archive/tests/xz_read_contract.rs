use nxtools_archive::output::{ArchiveOutput, ExtractRequest};
use nxtools_archive::planning::{ExtractionPlan, ExtractionSelection, TargetStrategy};
use nxtools_archive::{
    ArchiveEngine, ArchiveError, ArchiveFormat, ArchiveRequest, ArchiveSnapshot, OpenRequest,
    OperationControl, ResourceLimits, Result,
};
use std::{collections::BTreeMap, path::PathBuf};
use uuid::Uuid;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn payload() -> Vec<u8> {
    let mut bytes = "独立 gzip fixture\n".as_bytes().to_vec();
    bytes.extend(0u8..=255);
    bytes
}
#[derive(Default)]
struct Output {
    partials: BTreeMap<Uuid, Vec<u8>>,
    committed: Vec<Vec<u8>>,
    aborted: Vec<Uuid>,
    directories: Vec<String>,
    write_hook: Option<Box<dyn FnMut() -> Result<()>>>,
}
impl ArchiveOutput for Output {
    fn ensure_directory(&mut self, name: &str) -> Result<()> {
        self.directories.push(name.to_owned());
        Ok(())
    }
    fn begin_file(&mut self, _: &str) -> Result<Uuid> {
        let token = Uuid::new_v4();
        self.partials.insert(token, Vec::new());
        Ok(token)
    }
    fn write_file(&mut self, token: Uuid, bytes: &[u8]) -> Result<()> {
        assert!(bytes.len() <= 65536);
        self.partials
            .get_mut(&token)
            .expect("owned token")
            .extend_from_slice(bytes);
        if let Some(hook) = &mut self.write_hook {
            hook()?;
        }
        Ok(())
    }
    fn set_file_metadata(
        &mut self,
        token: Uuid,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> Result<()> {
        assert!(self.partials.contains_key(&token));
        metadata.validate()
    }
    fn finalize_directory_metadata(
        &mut self,
        _: &str,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> Result<()> {
        metadata.validate()
    }
    fn commit_file(&mut self, token: Uuid) -> Result<()> {
        self.committed
            .push(self.partials.remove(&token).expect("owned token"));
        Ok(())
    }
    fn abort_file(&mut self, token: Uuid) {
        self.partials.remove(&token);
        self.aborted.push(token);
    }
}
fn extract(
    engine: &ArchiveEngine,
    snapshot: ArchiveSnapshot,
    control: &OperationControl,
    output: &mut Output,
) -> Result<nxtools_archive::output::ExtractOutcome> {
    let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)?;
    engine.extract(ExtractRequest::new(snapshot, plan), control, output)
}

#[test]
fn xz_public_test_extract_alias_padding_and_unknown_metadata_are_truthful() {
    let owned = tempfile::tempdir().expect("owned aliases");
    for mode in [
        "single",
        "multi",
        "padding",
        "member_padding",
        "tar.xz",
        "TXZ",
    ] {
        let compound = matches!(mode, "tar.xz" | "TXZ");
        let mut bytes = std::fs::read(fixture(if compound {
            "independent.tar.xz"
        } else if matches!(mode, "multi" | "member_padding") {
            "multi.xz"
        } else {
            "independent.xz"
        }))
        .expect("independent XZ fixture");
        if mode == "padding" {
            bytes.extend([0; 8]);
        }
        if mode == "member_padding" {
            let second = bytes
                .windows(6)
                .enumerate()
                .skip(6)
                .find(|(_, value)| *value == b"\xfd7zXZ\0")
                .expect("second independent member")
                .0;
            bytes.splice(second..second, [0; 4]);
        }
        let path = owned.path().join(if compound {
            format!("payload.{mode}")
        } else {
            format!("{mode}.xz")
        });
        std::fs::write(&path, &bytes).expect("owned alias");
        let engine = ArchiveEngine::with_limits(ResourceLimits {
            // The TAR spool is larger than any individual TAR file.
            max_entry_bytes: payload().len() as u64,
            max_output_bytes: 1 << 20,
            ..Default::default()
        });
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(&path), &control)
            .expect("public open");
        assert_eq!(
            snapshot.format(),
            if compound {
                ArchiveFormat::TarXz
            } else {
                ArchiveFormat::Xz
            }
        );
        assert!(snapshot.capabilities().test && snapshot.capabilities().extract);
        assert!(!snapshot.capabilities().create && !snapshot.capabilities().modify);
        assert_eq!(snapshot.entries().len(), if compound { 3 } else { 1 });
        if !compound {
            let entry = &snapshot.entries()[0];
            assert_eq!(entry.size(), None);
            assert_eq!(entry.compressed_size(), Some(bytes.len() as u64));
            assert_eq!(entry.id().session, snapshot.session_id());
            assert_eq!(entry.id().ordinal, 0);
            assert_eq!(entry.display_path(), mode);
            assert_eq!(entry.method(), 0x21);
        }
        let expected = if matches!(mode, "multi" | "member_padding") {
            b"first\nsecond\n".to_vec()
        } else {
            payload()
        };
        let tested = engine
            .execute(ArchiveRequest::test(snapshot.clone()), &control)
            .expect("public integrity");
        assert_eq!(tested.files_verified(), 1);
        assert_eq!(tested.bytes_verified(), expected.len() as u64);
        let mut output = Output::default();
        let outcome = extract(&engine, snapshot, &control, &mut output).expect("public extraction");
        assert_eq!(outcome.files_committed, 1);
        assert_eq!(outcome.bytes_committed, expected.len() as u64);
        assert_eq!(outcome.skipped_special, u64::from(compound));
        assert_eq!(output.committed, vec![expected]);
        assert!(output.partials.is_empty());
        assert_eq!(std::fs::read(path).expect("unchanged source"), bytes);
    }
}

#[test]
fn xz_public_damage_matrix_rejects_before_any_publication() {
    let owned = tempfile::tempdir().expect("owned damage matrix");
    for compound in [false, true] {
        for mode in [
            "header",
            "index",
            "footer",
            "payload",
            "truncated",
            "garbage",
            "bad_padding",
            "second_payload",
        ] {
            let mut bytes = std::fs::read(fixture(if compound {
                "independent.tar.xz"
            } else if mode == "second_payload" {
                "multi.xz"
            } else {
                "independent.xz"
            }))
            .expect("independent damage input");
            let footer = bytes.len() - 12;
            let index = footer
                - (u32::from_le_bytes(
                    bytes[footer + 4..footer + 8]
                        .try_into()
                        .expect("footer u32"),
                ) as usize
                    + 1)
                    * 4;
            match mode {
                "header" => bytes[8] ^= 1,
                "index" => bytes[index + 1] ^= 1,
                "footer" => bytes[footer] ^= 1,
                "payload" => bytes[34] ^= 0x40,
                "truncated" => bytes.truncate(bytes.len() - 3),
                "garbage" => bytes.extend(b"garbage"),
                "bad_padding" => bytes.push(0),
                "second_payload" if !compound => {
                    let second = bytes
                        .windows(6)
                        .enumerate()
                        .skip(6)
                        .find(|(_, value)| *value == b"\xfd7zXZ\0")
                        .expect("second damaged member")
                        .0;
                    bytes[second + 28] ^= 1;
                }
                "second_payload" => bytes[34] ^= 0x40,
                _ => unreachable!(),
            }
            let path = owned
                .path()
                .join(if compound { "bad.tar.xz" } else { "bad.xz" });
            std::fs::write(&path, bytes).expect("owned mutation");
            let engine = ArchiveEngine::default();
            let control = OperationControl::default();
            let mut output = Output::default();
            match engine.open(OpenRequest::new(path), &control) {
                Ok(snapshot) => {
                    assert!(!compound, "outer integrity must precede TAR metadata");
                    assert!(matches!(mode, "payload" | "second_payload"), "{mode}");
                    assert_eq!(snapshot.entries()[0].size(), None);
                    assert_eq!(
                        engine.execute(ArchiveRequest::test(snapshot.clone()), &control),
                        Err(ArchiveError::CorruptArchive)
                    );
                    assert_eq!(
                        extract(&engine, snapshot, &control, &mut output),
                        Err(ArchiveError::CorruptArchive)
                    );
                    assert_eq!(output.aborted.len(), 1);
                }
                Err(failure) => {
                    assert_eq!(failure, ArchiveError::CorruptArchive, "{compound}/{mode}")
                }
            }
            assert!(output.committed.is_empty());
            assert!(output.partials.is_empty());
            assert!(output.directories.is_empty());
        }
    }
}

#[test]
fn xz_public_limits_apply_to_metadata_dictionary_members_actual_output_and_spool() {
    for (name, limits) in [
        (
            "independent.xz",
            ResourceLimits {
                max_working_bytes: 1 << 20,
                ..Default::default()
            },
        ),
        (
            "independent.xz",
            ResourceLimits {
                max_metadata_bytes: 1,
                ..Default::default()
            },
        ),
        (
            "multi.xz",
            ResourceLimits {
                max_entries: 1,
                ..Default::default()
            },
        ),
        (
            "independent.xz",
            ResourceLimits {
                max_entry_bytes: 1,
                ..Default::default()
            },
        ),
        (
            "independent.xz",
            ResourceLimits {
                max_output_bytes: 1,
                ..Default::default()
            },
        ),
        (
            "independent.tar.xz",
            ResourceLimits {
                max_output_bytes: 512,
                ..Default::default()
            },
        ),
    ] {
        let engine = ArchiveEngine::with_limits(limits);
        let control = OperationControl::default();
        assert_eq!(
            engine.open(OpenRequest::new(fixture(name)), &control).err(),
            Some(ArchiveError::ResourceLimit),
            "{name}/{limits:?}"
        );
        // A previously admitted snapshot cannot bypass a later operation's actual limits.
        let snapshot = ArchiveEngine::default()
            .open(OpenRequest::new(fixture(name)), &control)
            .expect("default admission");
        assert_eq!(
            engine.execute(ArchiveRequest::test(snapshot.clone()), &control),
            Err(ArchiveError::ResourceLimit)
        );
        let mut output = Output::default();
        assert_eq!(
            extract(&engine, snapshot, &control, &mut output),
            Err(ArchiveError::ResourceLimit)
        );
        assert!(output.committed.is_empty() && output.partials.is_empty());
    }
}

#[test]
fn xz_public_cancellation_io_and_original_source_changes_abort_only_owned_partial() {
    let owned = tempfile::tempdir().expect("owned safety matrix");
    for compound in [false, true] {
        for mode in ["cancel", "io", "source"] {
            let path = owned
                .path()
                .join(if compound { "held.tar.xz" } else { "held.xz" });
            let original = std::fs::read(fixture(if compound {
                "independent.tar.xz"
            } else {
                "independent.xz"
            }))
            .expect("independent safety input");
            std::fs::write(&path, &original).expect("owned safety copy");
            let engine = ArchiveEngine::default();
            let control = OperationControl::default();
            let snapshot = engine
                .open(OpenRequest::new(&path), &control)
                .expect("safety snapshot");
            let hook_control = control.clone();
            let hook_path = path.clone();
            let mut output = Output {
                write_hook: Some(Box::new(move || {
                    match mode {
                        "cancel" => hook_control.cancel(),
                        "io" => return Err(ArchiveError::IoFailure),
                        "source" => {
                            use std::io::Write;
                            // Header is already consumed; preserve length and the remaining decoder input.
                            let mut writer = std::fs::OpenOptions::new()
                                .write(true)
                                .open(&hook_path)
                                .expect("owned source writer");
                            writer.write_all(b"X").expect("same-size source change");
                            writer.flush().expect("source change flush");
                        }
                        _ => unreachable!(),
                    }
                    Ok(())
                })),
                ..Default::default()
            };
            let unrelated = Uuid::new_v4();
            output.partials.insert(unrelated, b"unrelated".to_vec());
            output.committed.push(b"already published".to_vec());
            assert_eq!(
                extract(&engine, snapshot, &control, &mut output),
                Err(match mode {
                    "cancel" => ArchiveError::Cancelled,
                    "io" => ArchiveError::IoFailure,
                    _ => ArchiveError::SourceChanged,
                }),
                "{compound}/{mode}"
            );
            assert_eq!(output.committed, vec![b"already published".to_vec()]);
            assert_eq!(output.aborted.len(), 1);
            assert_ne!(output.aborted[0], unrelated);
            assert_eq!(
                output.partials,
                BTreeMap::from([(unrelated, b"unrelated".to_vec())])
            );
            if mode != "source" {
                assert_eq!(std::fs::read(path).expect("preserved source"), original);
            }
        }
    }
}

#[test]
fn xz_public_pre_cancel_and_changed_snapshot_fail_without_output() {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    control.cancel();
    assert_eq!(
        engine
            .open(OpenRequest::new(fixture("independent.tar.xz")), &control)
            .err(),
        Some(ArchiveError::Cancelled)
    );
    let owned = tempfile::tempdir().expect("owned stale snapshots");
    for name in ["independent.xz", "independent.tar.xz"] {
        let path = owned.path().join(name);
        std::fs::copy(fixture(name), &path).expect("owned snapshot copy");
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(&path), &control)
            .expect("original snapshot");
        std::fs::write(path, b"replaced").expect("owned source replacement");
        assert_eq!(
            engine.execute(ArchiveRequest::test(snapshot.clone()), &control),
            Err(ArchiveError::SourceChanged)
        );
        let mut output = Output::default();
        assert_eq!(
            extract(&engine, snapshot, &control, &mut output),
            Err(ArchiveError::SourceChanged)
        );
        assert!(
            output.committed.is_empty()
                && output.partials.is_empty()
                && output.directories.is_empty()
        );
    }
}
