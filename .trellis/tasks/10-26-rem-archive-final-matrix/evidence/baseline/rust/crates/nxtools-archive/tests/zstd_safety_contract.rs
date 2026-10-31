use nxtools_archive::output::{ArchiveOutput, ExtractRequest};
use nxtools_archive::planning::{ExtractionPlan, ExtractionSelection, TargetStrategy};
use nxtools_archive::{
    ArchiveEngine, ArchiveError, ArchiveFormat, ArchiveRequest, OpenRequest, OperationControl,
    ResourceLimits, Result,
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
    directories: usize,
    aborted: usize,
    hook: Option<Box<dyn FnMut() -> Result<()>>>,
}
impl ArchiveOutput for Output {
    fn ensure_directory(&mut self, _: &str) -> Result<()> {
        self.directories += 1;
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
            .expect("own token")
            .extend(bytes);
        if let Some(hook) = &mut self.hook {
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
            .push(self.partials.remove(&token).expect("own token"));
        Ok(())
    }
    fn abort_file(&mut self, token: Uuid) {
        self.partials.remove(&token);
        self.aborted += 1;
    }
}
fn extract(
    engine: &ArchiveEngine,
    snapshot: nxtools_archive::ArchiveSnapshot,
    control: &OperationControl,
    output: &mut Output,
) -> Result<nxtools_archive::output::ExtractOutcome> {
    let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)?;
    engine.extract(ExtractRequest::new(snapshot, plan), control, output)
}
#[test]
fn zstd_checked_aliases_skippable_frames_unknown_metadata_and_exact_output() {
    let owned = tempfile::tempdir().expect("owned aliases");
    for mode in [
        "single", "multi", "checked", "skip", "tar.zst", "tar.zstd", "TZST",
    ] {
        let compound = matches!(mode, "tar.zst" | "tar.zstd" | "TZST");
        let mut bytes = std::fs::read(fixture(if compound {
            "checked.tar.zst"
        } else if mode == "multi" {
            "multi.zst"
        } else {
            "checked.zst"
        }))
        .expect("independent Zstd");
        if mode == "skip" {
            let mut prefix = vec![0x50, 0x2a, 0x4d, 0x18, 3, 0, 0, 0, 1, 2, 3];
            prefix.extend(bytes);
            bytes = prefix;
            bytes.extend([0x5f, 0x2a, 0x4d, 0x18, 0, 0, 0, 0]);
        }
        let path = owned.path().join(if compound {
            format!("payload.{mode}")
        } else {
            format!("{mode}.zst")
        });
        std::fs::write(&path, &bytes).expect("owned source");
        let engine = ArchiveEngine::with_limits(ResourceLimits {
            max_entry_bytes: payload().len() as u64,
            max_output_bytes: 1 << 20,
            ..Default::default()
        });
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(&path), &control)
            .expect("metadata");
        assert_eq!(
            snapshot.format(),
            if compound {
                ArchiveFormat::TarZstd
            } else {
                ArchiveFormat::Zstd
            }
        );
        if !compound {
            assert_eq!(snapshot.entries()[0].size(), None);
            assert_eq!(
                snapshot.entries()[0].compressed_size(),
                Some(bytes.len() as u64)
            );
        }
        let expected = if mode == "multi" {
            b"first\nsecond\n".to_vec()
        } else {
            payload()
        };
        assert_eq!(
            engine
                .execute(ArchiveRequest::test(snapshot.clone()), &control)
                .expect("full test")
                .bytes_verified(),
            expected.len() as u64
        );
        let mut output = Output::default();
        let outcome = extract(&engine, snapshot, &control, &mut output).expect("output");
        assert_eq!(outcome.files_committed, 1);
        assert_eq!(outcome.skipped_special, u64::from(compound));
        assert_eq!(output.committed, vec![expected]);
        assert!(output.partials.is_empty());
    }
}
#[test]
fn zstd_damage_limits_dictionary_and_outer_failure_never_publish() {
    let owned = tempfile::tempdir().expect("owned mutation");
    for compound in [false, true] {
        for mode in [
            "header",
            "checksum",
            "truncated",
            "garbage",
            "dictionary",
            "window",
            "metadata",
            "work",
            "output",
            "entry",
            "frames",
        ] {
            let mut bytes = std::fs::read(fixture(if compound {
                "checked.tar.zst"
            } else if mode == "frames" {
                "multi.zst"
            } else {
                "checked.zst"
            }))
            .expect("independent checked input");
            match mode {
                "header" => bytes[4] |= 8,
                "checksum" => {
                    let last = bytes.len() - 1;
                    bytes[last] ^= 1;
                }
                "truncated" => bytes.truncate(bytes.len() - 2),
                "garbage" => bytes.extend(b"garbage"),
                "dictionary" => {
                    bytes[4] |= 1;
                    bytes.insert(6, 1);
                }
                "window" => bytes[5] = 0xff,
                _ => {}
            }
            let path = owned
                .path()
                .join(if compound { "bad.tzst" } else { "bad.zst" });
            std::fs::write(&path, bytes).expect("owned mutation");
            let engine = ArchiveEngine::with_limits(ResourceLimits {
                max_metadata_bytes: if mode == "metadata" { 1 } else { 1 << 20 },
                max_working_bytes: if mode == "work" { 1 << 20 } else { 512 << 20 },
                max_output_bytes: if mode == "output" { 1 } else { 1 << 20 },
                max_entry_bytes: if mode == "entry" { 1 } else { 1 << 20 },
                max_entries: if mode == "frames" { 1 } else { 100 },
            });
            let control = OperationControl::default();
            let mut output = Output::default();
            let result = engine
                .open(OpenRequest::new(path), &control)
                .and_then(|snapshot| extract(&engine, snapshot, &control, &mut output));
            assert_eq!(
                result,
                Err(if mode == "dictionary" {
                    ArchiveError::UnsupportedMethod
                } else if matches!(
                    mode,
                    "window" | "metadata" | "work" | "output" | "entry" | "frames"
                ) {
                    ArchiveError::ResourceLimit
                } else {
                    ArchiveError::CorruptArchive
                }),
                "{compound}/{mode}"
            );
            assert!(output.committed.is_empty() && output.partials.is_empty());
            if compound {
                assert_eq!(output.directories, 0);
            }
        }
    }
}
#[test]
fn zstd_cancel_io_original_source_and_existing_snapshot_limits_abort_owned_token() {
    let owned = tempfile::tempdir().expect("owned safety inputs");
    for compound in [false, true] {
        for mode in ["cancel", "io", "source", "budget"] {
            let path = owned.path().join(if compound {
                "source.tzst"
            } else {
                "source.zst"
            });
            std::fs::copy(
                fixture(if compound {
                    "checked.tar.zst"
                } else {
                    "checked.zst"
                }),
                &path,
            )
            .expect("owned copy");
            let control = OperationControl::default();
            let snapshot = ArchiveEngine::default()
                .open(OpenRequest::new(&path), &control)
                .expect("admitted");
            let hook_control = control.clone();
            let hook_path = path.clone();
            let mut output = Output {
                hook: Some(Box::new(move || {
                    match mode {
                        "cancel" => hook_control.cancel(),
                        "io" => return Err(ArchiveError::IoFailure),
                        "source" => {
                            use std::io::Write;
                            let mut writer = std::fs::OpenOptions::new()
                                .write(true)
                                .open(&hook_path)
                                .expect("source writer");
                            writer.write_all(b"X").expect("equal-size source change");
                            writer.flush().expect("flush");
                        }
                        _ => {}
                    }
                    Ok(())
                })),
                ..Default::default()
            };
            let other = Uuid::new_v4();
            output.partials.insert(other, b"unrelated".to_vec());
            let engine = ArchiveEngine::with_limits(ResourceLimits {
                max_output_bytes: if mode == "budget" { 1 } else { 1 << 20 },
                ..Default::default()
            });
            assert_eq!(
                extract(&engine, snapshot, &control, &mut output),
                Err(match mode {
                    "cancel" => ArchiveError::Cancelled,
                    "io" => ArchiveError::IoFailure,
                    "source" => ArchiveError::SourceChanged,
                    _ => ArchiveError::ResourceLimit,
                }),
                "{compound}/{mode}"
            );
            assert!(output.committed.is_empty());
            assert_eq!(
                output.partials,
                BTreeMap::from([(other, b"unrelated".to_vec())])
            );
        }
    }
}

#[test]
fn zstd_empty_single_segment_declared_size_and_skippable_only_have_explicit_contracts() {
    let owned = tempfile::tempdir().expect("owned specification frames");
    // RFC8878 standard magic, SingleSegment/one-byte content size, final raw block.
    for (name, bytes, expected) in [
        (
            "empty",
            vec![0x28, 0xb5, 0x2f, 0xfd, 0x20, 0, 1, 0, 0],
            Some(Vec::new()),
        ),
        (
            "raw",
            vec![0x28, 0xb5, 0x2f, 0xfd, 0x20, 3, 25, 0, 0, b'a', b'b', b'c'],
            Some(b"abc".to_vec()),
        ),
        (
            "size_mismatch",
            vec![0x28, 0xb5, 0x2f, 0xfd, 0x20, 3, 17, 0, 0, b'a', b'b'],
            None,
        ),
        ("skip_only", vec![0x50, 0x2a, 0x4d, 0x18, 0, 0, 0, 0], None),
        (
            "skip_truncated",
            vec![0x50, 0x2a, 0x4d, 0x18, 255, 255, 255, 255],
            None,
        ),
    ] {
        let path = owned.path().join(format!("{name}.zst"));
        std::fs::write(&path, bytes).expect("specification frame");
        let engine = ArchiveEngine::default();
        let control = OperationControl::default();
        let mut output = Output::default();
        let result = engine
            .open(OpenRequest::new(path), &control)
            .and_then(|snapshot| {
                engine.execute(ArchiveRequest::test(snapshot.clone()), &control)?;
                extract(&engine, snapshot, &control, &mut output)
            });
        if let Some(expected) = expected {
            assert_eq!(
                result.expect("valid frame").bytes_committed,
                expected.len() as u64
            );
            assert_eq!(output.committed, vec![expected]);
        } else {
            assert_eq!(result, Err(ArchiveError::CorruptArchive), "{name}");
            assert!(output.committed.is_empty());
        }
        assert!(output.partials.is_empty());
    }
}
