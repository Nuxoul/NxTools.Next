use nxtools_archive::{ArchiveEngine, ArchiveError, ArchiveRequest, OpenRequest, OperationControl};
use std::path::PathBuf;
const PASSWORD: &str = "synthetic-private-ZIP-secret";
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}
#[test]
fn independent_zipcrypto_crc_and_descriptor_time_headers_require_final_integrity() {
    for name in ["independent-zipcrypto.zip", "independent-zipcrypto-dd.zip"] {
        let engine = ArchiveEngine::default();
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(fixture(name)), &control)
            .expect("independent encrypted ZIP");
        assert_eq!(snapshot.entries().len(), 3);
        assert!(snapshot.entries().iter().all(|entry| entry.encrypted()));
        assert_eq!(
            engine.execute(ArchiveRequest::test(snapshot.clone()), &control),
            Err(ArchiveError::PasswordRequired)
        );
        let engine = engine.with_password(PASSWORD);
        let result = engine
            .execute(ArchiveRequest::test(snapshot), &control)
            .expect("correct private password");
        assert_eq!(result.files_verified(), 2);
        assert_eq!(result.bytes_verified(), 1050);
    }
}

#[derive(Default)]
struct Output {
    partials: std::collections::BTreeMap<uuid::Uuid, Vec<u8>>,
    committed: Vec<Vec<u8>>,
    hook: Option<Box<dyn FnMut() -> nxtools_archive::Result<()>>>,
}
impl nxtools_archive::output::ArchiveOutput for Output {
    fn ensure_directory(&mut self, _: &str) -> nxtools_archive::Result<()> {
        Ok(())
    }
    fn begin_file(&mut self, _: &str) -> nxtools_archive::Result<uuid::Uuid> {
        let token = uuid::Uuid::new_v4();
        self.partials.insert(token, Vec::new());
        Ok(token)
    }
    fn write_file(&mut self, token: uuid::Uuid, bytes: &[u8]) -> nxtools_archive::Result<()> {
        self.partials
            .get_mut(&token)
            .expect("owned token")
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
    ) -> nxtools_archive::Result<()> {
        metadata.validate()
    }
    fn set_file_metadata(
        &mut self,
        token: uuid::Uuid,
        metadata: nxtools_archive::metadata::OutputMetadata,
    ) -> nxtools_archive::Result<()> {
        assert!(self.partials.contains_key(&token));
        metadata.validate()
    }
    fn commit_file(&mut self, token: uuid::Uuid) -> nxtools_archive::Result<()> {
        self.committed
            .push(self.partials.remove(&token).expect("owned token"));
        Ok(())
    }
    fn abort_file(&mut self, token: uuid::Uuid) {
        self.partials.remove(&token);
    }
}
fn extract(
    engine: &ArchiveEngine,
    snapshot: nxtools_archive::ArchiveSnapshot,
    control: &OperationControl,
    output: &mut Output,
) -> nxtools_archive::Result<nxtools_archive::output::ExtractOutcome> {
    use nxtools_archive::planning::{ExtractionPlan, ExtractionSelection, TargetStrategy};
    let plan = ExtractionPlan::build(&snapshot, ExtractionSelection::All, TargetStrategy::Here)?;
    engine.extract(
        nxtools_archive::output::ExtractRequest::new(snapshot, plan),
        control,
        output,
    )
}
#[test]
fn zipcrypto_private_password_damage_limits_cancel_io_and_source_never_publish_invalid_output() {
    use nxtools_archive::ResourceLimits;
    let owned = tempfile::tempdir().expect("owned crypto cases");
    for name in ["independent-zipcrypto.zip", "independent-zipcrypto-dd.zip"] {
        for mode in [
            "normal", "missing", "wrong", "header", "cipher", "time", "budget", "metadata",
            "cancel", "io", "source",
        ] {
            let mut bytes = std::fs::read(fixture(name)).expect("independent crypto");
            let offset = 30 + u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
            match mode {
                "header" => bytes[offset + 11] ^= 1,
                "cipher" => bytes[offset + 20] ^= 0x40,
                "time" => bytes[11] ^= 1,
                _ => {}
            }
            let path = owned.path().join("private.zip");
            std::fs::write(&path, &bytes).expect("owned source");
            let control = OperationControl::default();
            let snapshot = ArchiveEngine::default()
                .open(OpenRequest::new(&path), &control)
                .expect("central metadata");
            let engine = ArchiveEngine::with_limits(ResourceLimits {
                max_entry_bytes: if mode == "budget" { 1 } else { 1 << 20 },
                max_metadata_bytes: if mode == "metadata" { 1 } else { 1 << 20 },
                ..Default::default()
            });
            let engine = if mode == "missing" {
                engine
            } else {
                engine.with_password(if mode == "wrong" {
                    "wrong-private-secret"
                } else {
                    PASSWORD
                })
            };
            let expected = match mode {
                "missing" => ArchiveError::PasswordRequired,
                "wrong" | "header" | "cipher" => ArchiveError::AuthenticationOrDataError,
                "time" => ArchiveError::CorruptArchive,
                "budget" | "metadata" => ArchiveError::ResourceLimit,
                "cancel" => ArchiveError::Cancelled,
                "io" => ArchiveError::IoFailure,
                _ => ArchiveError::SourceChanged,
            };
            if !matches!(mode, "normal" | "cancel" | "io" | "source") {
                assert_eq!(
                    engine.execute(ArchiveRequest::test(snapshot.clone()), &control),
                    Err(expected),
                    "{name}/{mode}"
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
                            file.write_all(b"X").expect("source change");
                        }
                        _ => {}
                    }
                    Ok(())
                })),
                ..Default::default()
            };
            if mode == "normal" {
                let outcome = extract(&engine, snapshot, &control, &mut output)
                    .expect("correct password extract");
                assert_eq!(outcome.files_committed, 2);
                let mut expected = "独立 ZIP crypto fixture\n".as_bytes().to_vec();
                expected.extend((0u8..=255).cycle().take(1024));
                assert_eq!(output.committed, vec![expected, Vec::new()]);
            } else {
                assert_eq!(
                    extract(&engine, snapshot, &control, &mut output),
                    Err(expected),
                    "{name}/{mode}"
                );
                assert!(output.committed.is_empty());
                assert!(!format!("{expected:?}/{expected}").contains(PASSWORD));
            }
            assert!(output.partials.is_empty());
            if mode != "source" {
                assert_eq!(std::fs::read(path).expect("source retained"), bytes);
            }
        }
    }
}

#[test]
fn independent_aes_ae1_ae2_all_strengths_require_authentication_and_exact_bytes() {
    for name in [
        "independent-ae1-128.zip",
        "independent-ae1-192.zip",
        "independent-ae1-256.zip",
        "independent-ae2-128.zip",
        "independent-ae2-192.zip",
        "independent-ae2-256.zip",
        "independent-ae2-store.zip",
    ] {
        let engine = ArchiveEngine::default().with_password(PASSWORD);
        let control = OperationControl::default();
        let snapshot = engine
            .open(OpenRequest::new(fixture(name)), &control)
            .expect("independent AES metadata");
        assert_eq!(snapshot.entries().len(), 3);
        assert_eq!(
            ArchiveEngine::default().execute(ArchiveRequest::test(snapshot.clone()), &control),
            Err(ArchiveError::PasswordRequired)
        );
        let outcome = engine
            .execute(ArchiveRequest::test(snapshot.clone()), &control)
            .expect("AES full authentication");
        assert_eq!(outcome.files_verified(), 2);
        assert_eq!(outcome.bytes_verified(), 1050);
        let mut output = Output::default();
        let result =
            extract(&engine, snapshot, &control, &mut output).expect("AES verified output");
        assert_eq!(result.files_committed, 2);
        let mut expected = "独立 ZIP crypto fixture\n".as_bytes().to_vec();
        expected.extend((0u8..=255).cycle().take(1024));
        assert_eq!(output.committed, vec![expected, Vec::new()]);
        assert!(output.partials.is_empty());
    }
}

#[test]
fn aes_damage_framing_crc_budgets_and_operation_failures_never_publish_invalid_bytes() {
    use nxtools_archive::ResourceLimits;
    let owned = tempfile::tempdir().expect("owned AES safety matrix");
    for name in [
        "independent-ae1-128.zip",
        "independent-ae1-192.zip",
        "independent-ae1-256.zip",
        "independent-ae2-128.zip",
        "independent-ae2-192.zip",
        "independent-ae2-256.zip",
        "independent-ae2-store.zip",
    ] {
        for mode in [
            "wrong",
            "salt",
            "verifier",
            "cipher",
            "mac",
            "empty_mac",
            "local_extra",
            "version",
            "strength",
            "vendor",
            "method",
            "strong",
            "short",
            "crc",
            "entry",
            "total",
            "work",
            "metadata",
            "cancel",
            "io",
            "source",
        ] {
            let mut bytes = std::fs::read(fixture(name)).expect("independent AES input");
            let central = bytes
                .windows(4)
                .position(|value| value == b"PK\x01\x02")
                .expect("central");
            let local_extra = 30 + u16::from_le_bytes([bytes[26], bytes[27]]) as usize;
            let central_extra = central
                + 46
                + u16::from_le_bytes([bytes[central + 28], bytes[central + 29]]) as usize;
            let offset = local_extra + u16::from_le_bytes([bytes[28], bytes[29]]) as usize;
            let packed = u32::from_le_bytes(bytes[18..22].try_into().expect("packed")) as usize;
            let salt = (bytes[local_extra + 8] as usize + 1) * 4;
            match mode {
                "salt" => bytes[offset] ^= 1,
                "verifier" => bytes[offset + salt] ^= 1,
                "cipher" => bytes[offset + salt + 2] ^= 0x40,
                "mac" => bytes[offset + packed - 1] ^= 1,
                "empty_mac" => {
                    let next = offset + packed;
                    let data = next
                        + 30
                        + u16::from_le_bytes([bytes[next + 26], bytes[next + 27]]) as usize
                        + u16::from_le_bytes([bytes[next + 28], bytes[next + 29]]) as usize;
                    let size = u32::from_le_bytes(
                        bytes[next + 18..next + 22]
                            .try_into()
                            .expect("empty packed"),
                    ) as usize;
                    bytes[data + size - 1] ^= 1;
                }
                "local_extra" => {
                    bytes[local_extra + 4] = if bytes[local_extra + 4] == 1 { 2 } else { 1 }
                }
                "version" => {
                    bytes[local_extra + 4] = 3;
                    bytes[central_extra + 4] = 3;
                }
                "strength" => {
                    bytes[local_extra + 8] = 4;
                    bytes[central_extra + 8] = 4;
                }
                "vendor" => {
                    bytes[local_extra + 6] = b'X';
                    bytes[central_extra + 6] = b'X';
                }
                "method" => {
                    bytes[local_extra + 9] = 222;
                    bytes[central_extra + 9] = 222;
                }
                "strong" => {
                    bytes[6] |= 0x40;
                    bytes[central + 8] |= 0x40;
                }
                "short" => {
                    bytes[18..22].copy_from_slice(&1u32.to_le_bytes());
                    bytes[central + 20..central + 24].copy_from_slice(&1u32.to_le_bytes());
                }
                "crc" => {
                    bytes[14] ^= 1;
                    bytes[central + 16] ^= 1;
                }
                _ => {}
            }
            let path = owned.path().join("case.zip");
            std::fs::write(&path, bytes).expect("owned mutation");
            let control = OperationControl::default();
            let engine = ArchiveEngine::with_limits(ResourceLimits {
                max_entry_bytes: if mode == "entry" { 1 } else { 1 << 20 },
                max_output_bytes: if mode == "total" { 1 } else { 1 << 20 },
                max_working_bytes: if mode == "work" { 512 << 10 } else { 512 << 20 },
                max_metadata_bytes: if mode == "metadata" { 1 } else { 1 << 20 },
                ..Default::default()
            })
            .with_password(if mode == "wrong" {
                "wrong-private-secret"
            } else {
                PASSWORD
            });
            let expected = match mode {
                "version" | "strength" | "strong" => ArchiveError::UnsupportedEncryption,
                "method" => ArchiveError::UnsupportedMethod,
                "vendor" | "short" | "local_extra" => ArchiveError::CorruptArchive,
                "entry" | "total" | "work" | "metadata" => ArchiveError::ResourceLimit,
                "cancel" => ArchiveError::Cancelled,
                "io" => ArchiveError::IoFailure,
                "source" => ArchiveError::SourceChanged,
                _ => ArchiveError::AuthenticationOrDataError,
            };
            let snapshot = match ArchiveEngine::default().open(OpenRequest::new(&path), &control) {
                Ok(snapshot) => snapshot,
                Err(failure) => {
                    assert!(matches!(
                        mode,
                        "version" | "strength" | "strong" | "vendor" | "short"
                    ));
                    assert_eq!(failure, expected, "{name}/{mode}");
                    continue;
                }
            };
            let crc_exception = mode == "crc" && name.contains("ae2");
            if !matches!(mode, "cancel" | "io" | "source") {
                let result = engine.execute(ArchiveRequest::test(snapshot.clone()), &control);
                if crc_exception {
                    assert_eq!(
                        result
                            .expect("AE2 CRC is not content authentication")
                            .bytes_verified(),
                        1050
                    );
                } else {
                    assert_eq!(result, Err(expected), "test {name}/{mode}");
                }
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
                                .expect("owned source");
                            file.write_all(b"X").expect("source mutation");
                        }
                        _ => {}
                    }
                    Ok(())
                })),
                ..Default::default()
            };
            let result = extract(&engine, snapshot, &control, &mut output);
            if crc_exception {
                assert_eq!(
                    result
                        .expect("AE2 independent authentication")
                        .files_committed,
                    2
                );
            } else {
                assert_eq!(result, Err(expected), "extract {name}/{mode}");
                assert_eq!(
                    output.committed.len(),
                    usize::from(mode == "empty_mac"),
                    "only prior verified file may remain"
                );
            }
            assert!(output.partials.is_empty());
        }
    }
}

#[test]
fn independent_encrypted_method_combinations_zip64_descriptors_and_extra_failures() {
    let manifest: Vec<serde_json::Value> = serde_json::from_slice(
        &std::fs::read(fixture("zip-combinations.json")).expect("independent manifest"),
    )
    .expect("manifest JSON");
    assert_eq!(manifest.len(), 33);
    for item in manifest {
        let name = item["name"].as_str().expect("fixture name");
        let control = OperationControl::default();
        let engine = ArchiveEngine::default().with_password(PASSWORD);
        let expected = match item["error"].as_str() {
            Some("AuthenticationOrDataError") => Some(ArchiveError::AuthenticationOrDataError),
            Some("CorruptArchive") => Some(ArchiveError::CorruptArchive),
            None => None,
            _ => panic!("unknown manifest error"),
        };
        let snapshot = match engine.open(OpenRequest::new(fixture(name)), &control) {
            Ok(snapshot) => snapshot,
            Err(failure) => {
                assert_eq!(Some(failure), expected, "open {name}");
                continue;
            }
        };
        let result = engine.execute(ArchiveRequest::test(snapshot.clone()), &control);
        let mut output = Output::default();
        if let Some(expected) = expected {
            assert_eq!(result, Err(expected), "test {name}");
            assert_eq!(
                extract(&engine, snapshot, &control, &mut output),
                Err(expected),
                "extract {name}"
            );
            assert!(output.partials.is_empty() && output.committed.is_empty());
        } else {
            let result = result.expect(name);
            assert_eq!(result.files_verified(), 2);
            assert_eq!(result.bytes_verified(), 1050);
            assert_eq!(
                extract(&engine, snapshot.clone(), &control, &mut output)
                    .expect(name)
                    .files_committed,
                2
            );
            let mut expected = "独立 ZIP method fixture\n".as_bytes().to_vec();
            expected.extend((0u8..=255).cycle().take(1024));
            assert_eq!(output.committed, vec![expected, Vec::new()], "{name}");
            let wrong = ArchiveEngine::default().with_password("wrong-combination-secret");
            assert_eq!(
                wrong.execute(ArchiveRequest::test(snapshot), &control),
                Err(ArchiveError::AuthenticationOrDataError),
                "wrong password {name}"
            );
        }
    }
}
