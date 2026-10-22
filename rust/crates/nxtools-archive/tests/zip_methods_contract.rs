use nxtools_archive::output::{ArchiveOutput, ExtractRequest};
use nxtools_archive::planning::{ExtractionPlan, ExtractionSelection, TargetStrategy};
use nxtools_archive::{ArchiveEngine, ArchiveRequest, OpenRequest, OperationControl, Result};
use std::{collections::BTreeMap, path::PathBuf};
use uuid::Uuid;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
fn expected() -> Vec<u8> {
    let mut bytes = "独立 ZIP method fixture\n".as_bytes().to_vec();
    bytes.extend((0u8..=255).cycle().take(1024));
    bytes
}
#[derive(Default)]
struct Output {
    partials: BTreeMap<Uuid, (String, Vec<u8>)>,
    committed: BTreeMap<String, Vec<u8>>,
    hook: Option<Box<dyn FnMut() -> Result<()>>>,
}
impl ArchiveOutput for Output {
    fn ensure_directory(&mut self, _: &str) -> Result<()> {
        Ok(())
    }
    fn begin_file(&mut self, name: &str) -> Result<Uuid> {
        let token = Uuid::new_v4();
        self.partials.insert(token, (name.into(), Vec::new()));
        Ok(token)
    }
    fn write_file(&mut self, token: Uuid, bytes: &[u8]) -> Result<()> {
        assert!(bytes.len() <= 65536);
        self.partials
            .get_mut(&token)
            .expect("owned token")
            .1
            .extend_from_slice(bytes);
        if let Some(hook) = &mut self.hook {
            hook()?;
        }
        Ok(())
    }
    fn finalize_directory_metadata(
        &mut self,
        _: &str,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> Result<()> {
        metadata.validate()
    }
    fn set_file_metadata(
        &mut self,
        token: Uuid,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> Result<()> {
        assert!(self.partials.contains_key(&token));
        metadata.validate()
    }
    fn commit_file(&mut self, token: Uuid) -> Result<()> {
        let (name, bytes) = self.partials.remove(&token).expect("owned token");
        self.committed.insert(name, bytes);
        Ok(())
    }
    fn abort_file(&mut self, token: Uuid) {
        self.partials.remove(&token);
    }
}
#[test]
fn zip_zstd_independent_public_test_and_extract() {
    independent_read("independent-zstd.zip", 93);
}
#[test]
fn zip_xz_independent_public_test_and_extract() {
    independent_read("independent-xz.zip", 95);
}
#[test]
fn zip_lzma_independent_public_test_and_extract() {
    independent_read("independent-lzma.zip", 14);
}
#[test]
fn zip_deflate64_independent_public_test_and_extract() {
    independent_read("independent-deflate64.zip", 9);
}
fn independent_read(name: &str, method: u16) {
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(fixture(name)), &control)
        .expect("independent ZIP");
    assert_eq!(snapshot.entries().len(), 3);
    assert_eq!(snapshot.entries()[0].method(), method);
    for (ordinal, entry) in snapshot.entries().iter().enumerate() {
        assert_eq!(entry.id().ordinal, ordinal as u64);
        assert_eq!(entry.id().session, snapshot.session_id());
    }
    let verified = engine
        .execute(ArchiveRequest::test(snapshot.clone()), &control)
        .expect("ZIP93 stream");
    assert_eq!(verified.files_verified(), 2);
    assert_eq!(verified.bytes_verified(), expected().len() as u64);
    let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)
        .expect("plan");
    let mut output = Output::default();
    let extracted = engine
        .extract(ExtractRequest::new(snapshot, plan), &control, &mut output)
        .expect("ZIP93 output");
    assert_eq!(extracted.files_committed, 2);
    assert_eq!(
        output.committed,
        BTreeMap::from([
            ("资料/payload.bin".into(), expected()),
            ("empty.bin".into(), Vec::new())
        ])
    );
    assert!(output.partials.is_empty());
}

fn extract_snapshot(
    engine: &ArchiveEngine,
    snapshot: nxtools_archive::ArchiveSnapshot,
    control: &OperationControl,
    output: &mut Output,
) -> Result<nxtools_archive::output::ExtractOutcome> {
    let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)?;
    engine.extract(ExtractRequest::new(snapshot, plan), control, output)
}

#[test]
fn zip_zstd_lower_execution_entry_budget_and_forged_descriptor_are_rejected() {
    use nxtools_archive::{ArchiveError, ResourceLimits};
    let control = OperationControl::default();
    let snapshot = ArchiveEngine::default()
        .open(OpenRequest::new(fixture("independent-zstd.zip")), &control)
        .expect("baseline snapshot");
    let engine = ArchiveEngine::with_limits(ResourceLimits {
        max_entries: 1,
        ..Default::default()
    });
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot), &control),
        Err(ArchiveError::ResourceLimit)
    );
    let owned = tempfile::tempdir().expect("owned descriptor");
    let mut bytes = std::fs::read(fixture("independent-zstd.zip")).expect("independent source");
    // Set descriptor flags on both headers; no descriptor actually follows this entry.
    bytes[6] |= 8;
    bytes[14..26].fill(0);
    let central = bytes
        .windows(4)
        .position(|part| part == b"PK\x01\x02")
        .expect("central");
    bytes[central + 8] |= 8;
    let path = owned.path().join("forged.zip");
    std::fs::write(&path, bytes).expect("owned mutation");
    let engine = ArchiveEngine::default();
    let snapshot = engine
        .open(OpenRequest::new(path), &control)
        .expect("central remains readable");
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot), &control),
        Err(ArchiveError::CorruptArchive)
    );
}

#[test]
fn zip_zstd_damage_actual_limits_cancel_io_and_source_preserve_outputs() {
    damage_matrix("independent-zstd.zip");
}
#[test]
fn zip_xz_damage_actual_limits_cancel_io_and_source_preserve_outputs() {
    damage_matrix("independent-xz.zip");
}
#[test]
fn zip_lzma_damage_actual_limits_cancel_io_and_source_preserve_outputs() {
    damage_matrix("independent-lzma.zip");
}
#[test]
fn zip_deflate64_damage_actual_limits_cancel_io_and_source_preserve_outputs() {
    damage_matrix("independent-deflate64.zip");
}
fn damage_matrix(name: &str) {
    use nxtools_archive::{ArchiveError, ResourceLimits};
    let owned = tempfile::tempdir().expect("owned safety");
    for mode in [
        "payload",
        "truncated",
        "garbage",
        "local",
        "crc",
        "unknown",
        "legacy20",
        "work",
        "metadata",
        "entry",
        "total",
        "cancel",
        "io",
        "source",
    ] {
        let mut bytes = std::fs::read(fixture(name)).expect("independent input");
        let central = bytes
            .windows(4)
            .position(|part| part == b"PK\x01\x02")
            .expect("central");
        let offset = 30 + u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
        let packed = u32::from_le_bytes(bytes[18..22].try_into().expect("packed"));
        match mode {
            "payload" => bytes[offset + 15] ^= 0x40,
            "truncated" => {
                bytes[18..22].copy_from_slice(&(packed - 1).to_le_bytes());
                bytes[central + 20..central + 24].copy_from_slice(&(packed - 1).to_le_bytes());
            }
            "garbage" => {
                bytes[18..22].copy_from_slice(&(packed + 1).to_le_bytes());
                bytes[central + 20..central + 24].copy_from_slice(&(packed + 1).to_le_bytes());
            }
            "local" => bytes[8] = 8,
            "crc" => {
                bytes[14] ^= 1;
                bytes[central + 16] ^= 1;
            }
            "unknown" | "legacy20" => {
                let id: u16 = if mode == "unknown" { 222 } else { 20 };
                bytes[8..10].copy_from_slice(&id.to_le_bytes());
                bytes[central + 10..central + 12].copy_from_slice(&id.to_le_bytes());
            }
            _ => {}
        }
        let path = owned.path().join(format!("{mode}.zip"));
        std::fs::write(&path, &bytes).expect("owned copy");
        let control = OperationControl::default();
        let snapshot = ArchiveEngine::default()
            .open(OpenRequest::new(&path), &control)
            .expect("readable central");
        let limits = ResourceLimits {
            max_working_bytes: if mode == "work" { 512 << 10 } else { 512 << 20 },
            max_metadata_bytes: if mode == "metadata" { 1 } else { 256 << 20 },
            max_entry_bytes: if mode == "entry" { 1 } else { 32 << 30 },
            max_output_bytes: if mode == "total" { 1 } else { 100 << 30 },
            ..Default::default()
        };
        let engine = ArchiveEngine::with_limits(limits);
        let expected_error = match mode {
            "work" | "metadata" | "entry" | "total" => ArchiveError::ResourceLimit,
            "cancel" => ArchiveError::Cancelled,
            "io" => ArchiveError::IoFailure,
            "source" => ArchiveError::SourceChanged,
            "unknown" | "legacy20" => ArchiveError::UnsupportedMethod,
            _ => ArchiveError::CorruptArchive,
        };
        if !matches!(mode, "cancel" | "io" | "source") {
            assert_eq!(
                engine.execute(ArchiveRequest::test(snapshot.clone()), &control),
                Err(expected_error),
                "test {mode}"
            );
        }
        let hook_control = control.clone();
        let hook_path = path.clone();
        let mut output = Output {
            hook: Some(Box::new(move || {
                match mode {
                    "cancel" => hook_control.cancel(),
                    "io" => return Err(ArchiveError::IoFailure),
                    "source" => {
                        use std::io::Write;
                        let mut file = std::fs::OpenOptions::new()
                            .write(true)
                            .open(&hook_path)
                            .expect("owned writer");
                        file.write_all(b"X").expect("source modification");
                        file.flush().expect("flush source");
                    }
                    _ => {}
                }
                Ok(())
            })),
            ..Default::default()
        };
        let unowned = Uuid::new_v4();
        output
            .partials
            .insert(unowned, ("unrelated".into(), b"user partial".to_vec()));
        output
            .committed
            .insert("prior".into(), b"published".to_vec());
        assert_eq!(
            extract_snapshot(&engine, snapshot, &control, &mut output),
            Err(expected_error),
            "extract {mode}"
        );
        assert_eq!(
            output.committed,
            BTreeMap::from([("prior".into(), b"published".to_vec())])
        );
        assert_eq!(
            output.partials,
            BTreeMap::from([(unowned, ("unrelated".into(), b"user partial".to_vec()))])
        );
        if mode != "source" {
            assert_eq!(std::fs::read(path).expect("source retained"), bytes);
        }
    }
}

#[test]
fn zip_lzma_properties_dictionary_and_eos_are_checked_before_publication() {
    use nxtools_archive::ArchiveError;
    let owned = tempfile::tempdir().expect("owned LZMA cases");
    for mode in ["properties", "dictionary", "missing_eos", "clear_eos_flag"] {
        let mut bytes = std::fs::read(fixture("independent-lzma.zip")).expect("independent LZMA");
        let central = bytes
            .windows(4)
            .position(|part| part == b"PK\x01\x02")
            .expect("central");
        let offset = 30 + u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
        let packed = u32::from_le_bytes(bytes[18..22].try_into().expect("packed"));
        match mode {
            "properties" => bytes[offset + 2] = 6,
            "dictionary" => {
                bytes[offset + 5..offset + 9].copy_from_slice(&(1u32 << 30).to_le_bytes())
            }
            "missing_eos" => {
                bytes[18..22].copy_from_slice(&(packed - 6).to_le_bytes());
                bytes[central + 20..central + 24].copy_from_slice(&(packed - 6).to_le_bytes());
            }
            "clear_eos_flag" => {
                bytes[6] &= !2;
                bytes[central + 8] &= !2;
            }
            _ => unreachable!(),
        }
        let path = owned.path().join("case.zip");
        std::fs::write(&path, bytes).expect("owned mutation");
        let engine = ArchiveEngine::default();
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(path), &control)
            .expect("central metadata");
        let expected = match mode {
            "properties" => ArchiveError::UnsupportedMethod,
            "dictionary" => ArchiveError::ResourceLimit,
            _ => ArchiveError::CorruptArchive,
        };
        assert_eq!(
            engine.execute(ArchiveRequest::test(snapshot.clone()), &control),
            Err(expected),
            "{mode}"
        );
        let mut output = Output::default();
        assert_eq!(
            extract_snapshot(&engine, snapshot, &control, &mut output),
            Err(expected),
            "{mode}"
        );
        assert!(output.partials.is_empty() && output.committed.is_empty());
    }
}

#[test]
fn zip_deflate64_complete_output_and_crc_do_not_replace_final_block_end() {
    use nxtools_archive::ArchiveError;
    let owned = tempfile::tempdir().expect("owned unfinished Deflate64");
    let mut bytes =
        std::fs::read(fixture("independent-deflate64.zip")).expect("independent Deflate64");
    let offset = 30 + u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
    let old_packed = u32::from_le_bytes(bytes[18..22].try_into().expect("packed")) as usize;
    let data = expected();
    let length = data.len() as u16;
    let mut unfinished = vec![0]; // BFINAL=0, BTYPE=stored, byte alignment.
    unfinished.extend(length.to_le_bytes());
    unfinished.extend((!length).to_le_bytes());
    unfinished.extend(data);
    let delta = unfinished.len() as i64 - old_packed as i64;
    let packed = unfinished.len() as u32;
    bytes.splice(offset..offset + old_packed, unfinished);
    bytes[18..22].copy_from_slice(&packed.to_le_bytes());
    let central = bytes
        .windows(4)
        .position(|part| part == b"PK\x01\x02")
        .expect("central");
    bytes[central + 20..central + 24].copy_from_slice(&packed.to_le_bytes());
    let mut cursor = central;
    for index in 0..3 {
        if index > 0 {
            let old = u32::from_le_bytes(
                bytes[cursor + 42..cursor + 46]
                    .try_into()
                    .expect("local offset"),
            );
            bytes[cursor + 42..cursor + 46]
                .copy_from_slice(&((i64::from(old) + delta) as u32).to_le_bytes());
        }
        cursor += 46 + u16::from_le_bytes([bytes[cursor + 28], bytes[cursor + 29]]) as usize;
    }
    let eocd = bytes.len() - 22;
    bytes[eocd + 16..eocd + 20].copy_from_slice(&(central as u32).to_le_bytes());
    let path = owned.path().join("unfinished.zip");
    std::fs::write(&path, bytes).expect("owned mutation");
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(OpenRequest::new(path), &control)
        .expect("readable central");
    assert_eq!(
        engine.execute(ArchiveRequest::test(snapshot.clone()), &control),
        Err(ArchiveError::CorruptArchive)
    );
    let mut output = Output::default();
    assert_eq!(
        extract_snapshot(&engine, snapshot, &control, &mut output),
        Err(ArchiveError::CorruptArchive)
    );
    assert!(output.partials.is_empty() && output.committed.is_empty());
}

#[test]
fn zip_payload_trailing_byte_inside_nonoverlapping_extent_is_not_stream_end() {
    use nxtools_archive::ArchiveError;
    let owned = tempfile::tempdir().expect("owned trailing inputs");
    for name in [
        "independent-deflate64.zip",
        "independent-lzma.zip",
        "independent-xz.zip",
        "independent-zstd.zip",
    ] {
        let original = std::fs::read(fixture(name)).expect("independent input");
        for suffix in [0u8, 0xff, 0x50] {
            let mut bytes = original.clone();
            let packed = u32::from_le_bytes(bytes[18..22].try_into().expect("packed"));
            let offset = 30 + u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
            bytes.insert(offset + packed as usize, suffix);
            bytes[18..22].copy_from_slice(&(packed + 1).to_le_bytes());
            let central = bytes
                .windows(4)
                .position(|part| part == b"PK\x01\x02")
                .expect("central");
            bytes[central + 20..central + 24].copy_from_slice(&(packed + 1).to_le_bytes());
            let mut cursor = central;
            for index in 0..3 {
                if index > 0 {
                    let old = u32::from_le_bytes(
                        bytes[cursor + 42..cursor + 46]
                            .try_into()
                            .expect("local offset"),
                    );
                    bytes[cursor + 42..cursor + 46].copy_from_slice(&(old + 1).to_le_bytes());
                }
                cursor +=
                    46 + u16::from_le_bytes([bytes[cursor + 28], bytes[cursor + 29]]) as usize;
            }
            let eocd = bytes.len() - 22;
            bytes[eocd + 16..eocd + 20].copy_from_slice(&(central as u32).to_le_bytes());
            let path = owned.path().join("tail.zip");
            std::fs::write(&path, bytes).expect("owned mutation");
            let engine = ArchiveEngine::default();
            let control = OperationControl::default();
            let snapshot = engine
                .open(OpenRequest::new(path), &control)
                .expect("nonoverlapping central");
            assert_eq!(
                engine.execute(ArchiveRequest::test(snapshot.clone()), &control),
                Err(ArchiveError::CorruptArchive),
                "{name}/{suffix}"
            );
            let mut output = Output::default();
            assert_eq!(
                extract_snapshot(&engine, snapshot, &control, &mut output),
                Err(ArchiveError::CorruptArchive),
                "{name}/{suffix}"
            );
            assert!(output.partials.is_empty() && output.committed.is_empty());
        }
    }
}

#[test]
fn independent_zip_lzma_without_eos_uses_known_size_and_consumes_exact_input() {
    use nxtools_archive::ArchiveError;
    let engine = ArchiveEngine::default();
    let control = OperationControl::default();
    let snapshot = engine
        .open(
            OpenRequest::new(fixture("independent-lzma-no-eos.zip")),
            &control,
        )
        .expect("stock7-Zip eos=off metadata");
    assert_eq!(snapshot.entries().len(), 1);
    assert_eq!(snapshot.entries()[0].method(), 14);
    assert_eq!(
        engine
            .execute(ArchiveRequest::test(snapshot.clone()), &control)
            .expect("known-size termination")
            .bytes_verified(),
        1050
    );
    let mut output = Output::default();
    assert_eq!(
        extract_snapshot(&engine, snapshot, &control, &mut output)
            .expect("known-size native capability")
            .files_committed,
        1
    );
    assert_eq!(
        output.committed,
        BTreeMap::from([("资料/payload.bin".into(), expected())])
    );
    let owned = tempfile::tempdir().expect("owned noEOS mutations");
    for mode in ["truncated", "wrong_size", "requires_eos"] {
        let mut bytes =
            std::fs::read(fixture("independent-lzma-no-eos.zip")).expect("independent noEOS");
        let central = bytes
            .windows(4)
            .position(|value| value == b"PK\x01\x02")
            .expect("central");
        match mode {
            "truncated" => {
                let packed = u32::from_le_bytes(bytes[18..22].try_into().expect("packed")) - 2;
                bytes[18..22].copy_from_slice(&packed.to_le_bytes());
                bytes[central + 20..central + 24].copy_from_slice(&packed.to_le_bytes());
            }
            "wrong_size" => {
                bytes[22..26].copy_from_slice(&1051u32.to_le_bytes());
                bytes[central + 24..central + 28].copy_from_slice(&1051u32.to_le_bytes());
            }
            "requires_eos" => {
                bytes[6] |= 2;
                bytes[central + 8] |= 2;
            }
            _ => unreachable!(),
        }
        let path = owned.path().join("bad.zip");
        std::fs::write(&path, bytes).expect("owned mutation");
        let snapshot = engine
            .open(OpenRequest::new(path), &control)
            .expect("central");
        assert_eq!(
            engine.execute(ArchiveRequest::test(snapshot), &control),
            Err(ArchiveError::CorruptArchive),
            "{mode}"
        );
    }
}
