use crate::model::SourceRevision;
use crate::{
    ArchiveCapabilities, ArchiveError, ArchiveFormat, ArchiveOutcome, ArchiveRequest,
    ArchiveSnapshot, OpenRequest, OperationControl, ResourceLimits, Result,
    formats::{bzip, gzip, tar, tgz, xz, zip, zstd},
};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

/// Live format facts belong to one opened worker attempt, never to ArchiveSnapshot.
#[derive(Default)]
pub(crate) struct RetainedCatalog {
    pub(crate) input: Option<File>,
    pub(crate) tar_plan: Option<tar::ValidatedPlan>,
    gzip_members: Option<u64>,
}

#[derive(Default)]
pub struct ArchiveEngine<'password> {
    limits: ResourceLimits,
    password: Option<&'password str>,
    zip_volumes: bool,
    held_zip_paths: Option<&'password [String]>,
}
impl<'password> ArchiveEngine<'password> {
    pub fn with_limits(limits: ResourceLimits) -> Self {
        Self {
            limits,
            password: None,
            zip_volumes: false,
            held_zip_paths: None,
        }
    }
    /// Generate a complete unpublished ZIP edit candidate against immutable source/entry identity.
    pub fn zip_edit_candidate(
        &self,
        snapshot: &ArchiveSnapshot,
        edits: &[crate::zip_edit::ZipEdit],
        additions: Option<&mut File>,
        control: &OperationControl,
    ) -> Result<File> {
        crate::zip_edit::candidate(
            snapshot,
            edits,
            additions,
            &self.limits,
            control,
            self.password,
        )
    }
    /// Build unpublished encoded ZIP volume candidates. The caller owns native no-replace
    /// publication and must validate all candidates before publishing any member.
    pub fn zip_volume_candidates(
        &self,
        input: &mut File,
        volume_bytes: u64,
        control: &OperationControl,
    ) -> Result<Vec<File>> {
        crate::zip_container::split_encoded(input, volume_bytes, &self.limits, control)
    }
    /// Opt-in container adapter. A Windows host must lease every source volume before use.
    /// Production worker admission does not enable this until native volume-set binding exists.
    pub fn with_zip_volumes(mut self) -> Self {
        self.zip_volumes = true;
        self
    }
    pub(crate) fn with_held_zip_volumes(mut self, paths: &'password [String]) -> Self {
        self.zip_volumes = true;
        self.held_zip_paths = Some(paths);
        self
    }
    /// Borrow a private operation-scoped secret; never retain it in archive metadata.
    pub fn with_password(mut self, password: &'password str) -> Self {
        self.password = Some(password);
        self
    }
    pub fn open(
        &self,
        request: OpenRequest,
        control: &OperationControl,
    ) -> Result<ArchiveSnapshot> {
        let mut retained = RetainedCatalog::default();
        let mut snapshot = self.open_impl(request, control, false, true, Some(&mut retained))?;
        if snapshot.format == ArchiveFormat::Zip && snapshot.revision.volumes.is_empty() {
            return Ok(snapshot);
        }
        let refusal = if matches!(
            snapshot.format,
            ArchiveFormat::Tar | ArchiveFormat::TarGz | ArchiveFormat::TarXz
        ) {
            self.tar_eligibility_from_open(&snapshot, &mut retained, control)
                .err()
        } else if snapshot.format == ArchiveFormat::Zip {
            self.modification_eligibility(&snapshot, control).err()
        } else {
            Some(crate::ArchiveModificationRefusal::BackendMissing)
        };
        snapshot.capabilities = ArchiveCapabilities {
            split_read: !snapshot.revision.volumes.is_empty(),
            ..ArchiveCapabilities::opened(snapshot.format, refusal)
        };
        control.checkpoint()?;
        Ok(snapshot)
    }
    /// Fresh read-only admission against this immutable revision, not a creation-format claim.
    pub fn modification_eligibility(
        &self,
        snapshot: &ArchiveSnapshot,
        control: &OperationControl,
    ) -> std::result::Result<(), crate::ArchiveModificationRefusal> {
        use crate::ArchiveModificationRefusal as Refusal;
        control.record_work(|work| work.eligibility_checks += 1);
        if snapshot.format == ArchiveFormat::Zip {
            return crate::zip_edit::eligibility(snapshot, &self.limits, control);
        }
        let format = match snapshot.format {
            ArchiveFormat::Tar => "Tar",
            ArchiveFormat::TarGz => "TarGz",
            ArchiveFormat::TarXz => "TarXz",
            _ => return Err(Refusal::BackendMissing),
        };
        if !snapshot.revision.volumes.is_empty() {
            return Err(Refusal::Volumes);
        }
        let inspect = || -> Result<()> {
            let mut source = File::open(&snapshot.revision.path)?;
            ensure_revision(&mut source, &snapshot.revision, control)?;
            let strict_limits = self.strict_tar_limits(snapshot, None)?;
            let (_, mut plain) = crate::tar_edit_outer::Outer::open(
                source.try_clone()?,
                format,
                &strict_limits,
                control,
            )?;
            let members = crate::tar_edit::inspect(&mut plain, &strict_limits, control)?;
            if members.len() != snapshot.entries.len()
                || members.iter().zip(snapshot.entries.iter()).any(|(a, b)| {
                    a.name != b.display_path
                        || a.directory != (b.kind == crate::EntryKind::Directory)
                        || Some(a.size) != b.size
                })
            {
                return Err(ArchiveError::UnsupportedOperation);
            }
            ensure_revision(&mut source, &snapshot.revision, control)
        };
        inspect().map_err(|error| match error {
            ArchiveError::SourceChanged => Refusal::SourceChanged,
            ArchiveError::CorruptArchive => Refusal::Corrupt,
            ArchiveError::IoFailure => Refusal::SourceUnavailable,
            ArchiveError::ResourceLimit | ArchiveError::Cancelled => Refusal::UnverifiedMetadata,
            _ => Refusal::UnsupportedStructure,
        })
    }
    fn strict_tar_limits(
        &self,
        snapshot: &ArchiveSnapshot,
        plan: Option<&tar::ValidatedPlan>,
    ) -> Result<ResourceLimits> {
        let metadata = plan.map_or_else(
            || tar::metadata_bytes(snapshot),
            |plan| Ok(plan.metadata_bytes()),
        )?;
        let locators = plan.map_or(0, tar::ValidatedPlan::locator_bytes);
        Ok(ResourceLimits {
            max_metadata_bytes: self
                .limits
                .max_metadata_bytes
                .checked_sub(metadata)
                .ok_or(ArchiveError::ResourceLimit)?,
            max_working_bytes: self
                .limits
                .max_working_bytes
                .checked_sub(locators)
                .ok_or(ArchiveError::ResourceLimit)?,
            ..self.limits
        })
    }
    fn tar_eligibility_from_open(
        &self,
        snapshot: &ArchiveSnapshot,
        retained: &mut RetainedCatalog,
        control: &OperationControl,
    ) -> std::result::Result<(), crate::ArchiveModificationRefusal> {
        use crate::ArchiveModificationRefusal as Refusal;
        control.record_work(|work| work.eligibility_checks += 1);
        let inspect = || -> Result<()> {
            let plan = retained
                .tar_plan
                .as_ref()
                .ok_or(ArchiveError::ResourceLimit)?;
            let strict_limits = self.strict_tar_limits(snapshot, Some(plan))?;
            let mut source = File::open(&snapshot.revision.path)?;
            ensure_revision(&mut source, &snapshot.revision, control)?;
            crate::tar_edit_outer::Outer::qualify_verified(
                &mut source,
                &format!("{:?}", snapshot.format),
                retained.gzip_members,
                &strict_limits,
                control,
            )?;
            let plain = retained.input.as_mut().ok_or(ArchiveError::SourceChanged)?;
            let members = crate::tar_edit::inspect(plain, &strict_limits, control)?;
            if members.len() != snapshot.entries.len()
                || members.iter().zip(snapshot.entries.iter()).any(|(a, b)| {
                    a.name != b.display_path
                        || a.directory != (b.kind == crate::EntryKind::Directory)
                        || Some(a.size) != b.size
                })
            {
                return Err(ArchiveError::UnsupportedOperation);
            }
            ensure_revision(&mut source, &snapshot.revision, control)
        };
        let mut inspect = inspect;
        inspect().map_err(|error| match error {
            ArchiveError::SourceChanged => Refusal::SourceChanged,
            ArchiveError::CorruptArchive => Refusal::Corrupt,
            ArchiveError::IoFailure => Refusal::SourceUnavailable,
            ArchiveError::ResourceLimit | ArchiveError::Cancelled => Refusal::UnverifiedMetadata,
            _ => Refusal::UnsupportedStructure,
        })
    }

    /// Browser read-only projection bound to the original opened source version. Native
    /// object identity remains C# final admission authority, never inferred from a filename.
    pub fn inspect_modification_source(
        &self,
        request: OpenRequest,
        expected: &nxtools_protocol::archive_browser::ArchiveSourceBinding,
        control: &OperationControl,
    ) -> std::result::Result<(), crate::ArchiveModificationRefusal> {
        use crate::ArchiveModificationRefusal as Refusal;
        let snapshot = self.open(request, control).map_err(|error| match error {
            ArchiveError::SourceChanged => Refusal::SourceChanged,
            ArchiveError::CorruptArchive => Refusal::Corrupt,
            ArchiveError::UnsupportedEncryption => Refusal::UnsupportedEncryption,
            ArchiveError::IoFailure => Refusal::SourceUnavailable,
            _ => Refusal::UnverifiedMetadata,
        })?;
        let modified = snapshot
            .revision
            .modified
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|duration| u64::try_from(duration.as_nanos()).ok());
        if expected.validate().is_err()
            || snapshot.revision.digest != expected.digest
            || snapshot.revision.size != expected.size
            || modified != Some(expected.modified_unix_ns)
        {
            return Err(Refusal::SourceChanged);
        }
        snapshot.capabilities.admit(crate::ArchiveOperation::Add)
    }
    /// Internal read paths do not discover editing capabilities they never consume.
    pub(crate) fn open_catalog(
        &self,
        request: OpenRequest,
        control: &OperationControl,
    ) -> Result<ArchiveSnapshot> {
        self.open_impl(request, control, false, false, None)
    }
    /// Retain the normalized ZIP source or verified outer TAR spool for one worker operation.
    pub(crate) fn open_retained_catalog(
        &self,
        request: OpenRequest,
        control: &OperationControl,
    ) -> Result<(ArchiveSnapshot, RetainedCatalog)> {
        let opened = self.open_attempt_catalog(request, control, false)?;
        if opened.0.format() != ArchiveFormat::Zip {
            control.record_work(|work| work.catalog_loads += 1);
        }
        Ok(opened)
    }
    /// Private extraction admission only. Public open/TEST keep unknown stream metadata.
    pub(crate) fn open_attempt_catalog(
        &self,
        request: OpenRequest,
        control: &OperationControl,
        stream_declarations: bool,
    ) -> Result<(ArchiveSnapshot, RetainedCatalog)> {
        let mut retained = RetainedCatalog::default();
        let snapshot = self.open_impl(
            request,
            control,
            stream_declarations,
            false,
            Some(&mut retained),
        )?;
        Ok((snapshot, retained))
    }
    fn open_impl(
        &self,
        request: OpenRequest,
        control: &OperationControl,
        stream_declarations: bool,
        qualify_zip: bool,
        mut retained_input: Option<&mut RetainedCatalog>,
    ) -> Result<ArchiveSnapshot> {
        control.source_delete_begin();
        control.checkpoint()?;
        if self.zip_volumes
            && let Some(mut set) = crate::zip_container::discover(
                &request.path,
                &self.limits,
                control,
                self.held_zip_paths,
            )?
        {
            let mut revision = volume_revision(&mut set, control)?;
            let mut file = crate::zip_container::normalize(&mut set, &self.limits, control)?;
            let (entries, comment, backend_safe) =
                zip::list(&mut file, request.encoding, &self.limits, control)?;
            ensure_revision(&mut file, &revision, control)?;
            let session = entries
                .first()
                .map_or_else(uuid::Uuid::new_v4, |entry| entry.id.session);
            revision.path = set
                .paths
                .last()
                .cloned()
                .ok_or(ArchiveError::MissingVolume)?;
            // The revision binds the external volumes, not the normalized anonymous file.
            if let Some(retained) = retained_input.as_mut() {
                retained.input = Some(file);
            }
            return Ok(ArchiveSnapshot {
                revision,
                session,
                format: ArchiveFormat::Zip,
                capabilities: ArchiveCapabilities {
                    test: true,
                    extract: true,
                    split_read: true,
                    ..Default::default()
                },
                entries: entries.into(),
                comment,
                backend_safe,
            });
        }
        let mut file = File::open(&request.path)?;
        let mut signature = [0u8; 8];
        let size = file.read(&mut signature)?;
        file.rewind()?;
        let opened = if size >= 4 && matches!(&signature[..4], b"PK\x03\x04" | b"PK\x05\x06") {
            let revision = revision(&mut file, &request.path, control)?;
            let mut directory = None;
            let listed = zip::list_with_directory(
                &mut file,
                request.encoding,
                &self.limits,
                control,
                qualify_zip.then_some(&mut directory),
            );
            let (entries, comment, backend_safe) = match listed {
                Err(ArchiveError::ResourceLimit) if qualify_zip => {
                    // Editing metadata is optional; retain the original catalog budget.
                    zip::list(&mut file, request.encoding, &self.limits, control)?
                }
                result => result?,
            };
            let session = entries
                .first()
                .map_or_else(uuid::Uuid::new_v4, |entry| entry.id.session);
            let mut snapshot = ArchiveSnapshot {
                revision,
                session,
                format: ArchiveFormat::Zip,
                capabilities: ArchiveCapabilities {
                    test: true,
                    ..Default::default()
                },
                entries: entries.into(),
                comment,
                backend_safe,
            };
            if let Some(directory) = directory {
                control.record_work(|work| work.eligibility_checks += 1);
                let refusal = crate::zip_edit::eligibility_from_open(
                    &mut file,
                    &snapshot,
                    &directory,
                    &self.limits,
                    control,
                )
                .err();
                snapshot.capabilities = ArchiveCapabilities::opened(ArchiveFormat::Zip, refusal);
            } else if qualify_zip {
                snapshot.capabilities = ArchiveCapabilities::opened(
                    ArchiveFormat::Zip,
                    Some(crate::ArchiveModificationRefusal::UnverifiedMetadata),
                );
            }
            ensure_revision(&mut file, &snapshot.revision, control)?;
            Ok(snapshot)
        } else if size >= 3 && signature[..3] == [0x1f, 0x8b, 8] {
            let revision = revision(&mut file, &request.path, control)?;
            let compound = tgz::matches(&request.path);
            let entries = if compound {
                let (mut spool, members) = tgz::decode_members(&mut file, &self.limits, control)?;
                let (entries, plan) =
                    tar::list_planned(&mut spool, request.encoding, &self.limits, control)?;
                if let Some(retained) = retained_input.as_mut() {
                    retained.input = Some(spool);
                    retained.tar_plan = Some(plan);
                    retained.gzip_members = Some(members);
                }
                entries
            } else {
                gzip::list(&mut file, &request.path, &self.limits, control)?
            };
            let session = entries
                .first()
                .map_or_else(uuid::Uuid::new_v4, |entry| entry.id.session);
            ensure_revision(&mut file, &revision, control)?;
            Ok(ArchiveSnapshot {
                revision,
                session,
                format: if compound {
                    ArchiveFormat::TarGz
                } else {
                    ArchiveFormat::Gzip
                },
                capabilities: ArchiveCapabilities {
                    test: true,
                    extract: true,
                    ..Default::default()
                },
                entries: entries.into(),
                comment: String::new(),
                backend_safe: true,
            })
        } else if size >= 3 && &signature[..3] == b"BZh" {
            let revision = revision(&mut file, &request.path, control)?;
            let compound = tgz::matches_bzip(&request.path);
            let entries = if compound {
                let mut spool = tgz::decode_bzip(&mut file, &self.limits, control)?;
                let (entries, plan) =
                    tar::list_planned(&mut spool, request.encoding, &self.limits, control)?;
                if let Some(retained) = retained_input.as_mut() {
                    retained.input = Some(spool);
                    retained.tar_plan = Some(plan);
                }
                entries
            } else {
                bzip::list(&mut file, &request.path, &self.limits, control)?
            };
            let session = entries
                .first()
                .map_or_else(uuid::Uuid::new_v4, |entry| entry.id.session);
            ensure_revision(&mut file, &revision, control)?;
            Ok(ArchiveSnapshot {
                revision,
                session,
                format: if compound {
                    ArchiveFormat::TarBz2
                } else {
                    ArchiveFormat::Bzip2
                },
                capabilities: ArchiveCapabilities {
                    test: true,
                    extract: true,
                    ..Default::default()
                },
                entries: entries.into(),
                comment: String::new(),
                backend_safe: true,
            })
        } else if size >= 6 && &signature[..6] == b"\xfd7zXZ\0" {
            let revision = revision(&mut file, &request.path, control)?;
            let compound = tgz::matches_xz(&request.path);
            let entries = if compound {
                let mut spool = tgz::decode_xz(&mut file, &self.limits, control)?;
                let (entries, plan) =
                    tar::list_planned(&mut spool, request.encoding, &self.limits, control)?;
                if let Some(retained) = retained_input.as_mut() {
                    retained.input = Some(spool);
                    retained.tar_plan = Some(plan);
                }
                entries
            } else {
                if stream_declarations {
                    xz::list_for_output_preflight(
                        &mut file,
                        &request.path,
                        &self.limits,
                        control,
                        true,
                    )?
                } else {
                    xz::list(&mut file, &request.path, &self.limits, control)?
                }
            };
            let session = entries
                .first()
                .map_or_else(uuid::Uuid::new_v4, |entry| entry.id.session);
            ensure_revision(&mut file, &revision, control)?;
            Ok(ArchiveSnapshot {
                revision,
                session,
                format: if compound {
                    ArchiveFormat::TarXz
                } else {
                    ArchiveFormat::Xz
                },
                capabilities: ArchiveCapabilities {
                    test: true,
                    extract: true,
                    ..Default::default()
                },
                entries: entries.into(),
                comment: String::new(),
                backend_safe: true,
            })
        } else if size >= 4
            && (signature[..4] == [0x28, 0xb5, 0x2f, 0xfd]
                || u32::from_le_bytes(signature[..4].try_into().expect("four signature bytes"))
                    & 0xfffffff0
                    == 0x184d2a50)
        {
            let revision = revision(&mut file, &request.path, control)?;
            let compound = tgz::matches_zstd(&request.path);
            let entries = if compound {
                let mut spool = tgz::decode_zstd(&mut file, &self.limits, control)?;
                let (entries, plan) =
                    tar::list_planned(&mut spool, request.encoding, &self.limits, control)?;
                if let Some(retained) = retained_input.as_mut() {
                    retained.input = Some(spool);
                    retained.tar_plan = Some(plan);
                }
                entries
            } else {
                if stream_declarations {
                    zstd::list_for_output_preflight(
                        &mut file,
                        &request.path,
                        &self.limits,
                        control,
                        true,
                    )?
                } else {
                    zstd::list(&mut file, &request.path, &self.limits, control)?
                }
            };
            let session = entries
                .first()
                .map_or_else(uuid::Uuid::new_v4, |entry| entry.id.session);
            ensure_revision(&mut file, &revision, control)?;
            Ok(ArchiveSnapshot {
                revision,
                session,
                format: if compound {
                    ArchiveFormat::TarZstd
                } else {
                    ArchiveFormat::Zstd
                },
                capabilities: ArchiveCapabilities {
                    test: true,
                    extract: true,
                    ..Default::default()
                },
                entries: entries.into(),
                comment: String::new(),
                backend_safe: true,
            })
        } else if is_tar(&mut file, &request.path)? {
            let revision = revision(&mut file, &request.path, control)?;
            let (entries, plan) =
                tar::list_planned(&mut file, request.encoding, &self.limits, control)?;
            if let Some(retained) = retained_input.as_mut() {
                retained.input = Some(file.try_clone()?);
                retained.tar_plan = Some(plan);
            }
            let session = entries
                .first()
                .map_or_else(uuid::Uuid::new_v4, |entry| entry.id.session);
            ensure_revision(&mut file, &revision, control)?;
            Ok(ArchiveSnapshot {
                revision,
                session,
                format: ArchiveFormat::Tar,
                capabilities: ArchiveCapabilities {
                    test: true,
                    extract: true,
                    ..Default::default()
                },
                entries: entries.into(),
                comment: String::new(),
                backend_safe: true,
            })
        } else {
            Err(ArchiveError::UnsupportedFormat)
        };
        let mut snapshot = opened?;
        if let Some(plan) = retained_input
            .as_ref()
            .and_then(|retained| retained.tar_plan.as_ref())
        {
            // Empty TAR has no first entry from which to recover its parser session.
            snapshot.session = plan.session_id();
        }
        Ok(snapshot)
    }
    /// Re-read ZIP names under the original revision; call off the UI thread, like open.
    /// Entry identity survives code-page changes, but output plans must be rebuilt.
    pub fn reproject(
        &self,
        snapshot: &ArchiveSnapshot,
        encoding: crate::NameEncoding,
        control: &OperationControl,
    ) -> Result<ArchiveSnapshot> {
        control.source_delete_begin();
        control.checkpoint()?;
        if snapshot.format != ArchiveFormat::Zip {
            return Err(ArchiveError::UnsupportedOperation);
        }
        let mut file = open_snapshot(snapshot, &self.limits, control)?;
        ensure_revision(&mut file, &snapshot.revision, control)?;
        let (mut entries, comment, backend_safe) =
            zip::list(&mut file, encoding, &self.limits, control)?;
        ensure_revision(&mut file, &snapshot.revision, control)?;
        for entry in &mut entries {
            entry.id.session = snapshot.session;
        }
        let mut projected = ArchiveSnapshot {
            entries: entries.into(),
            comment,
            backend_safe,
            ..snapshot.clone()
        };
        projected.capabilities = ArchiveCapabilities {
            split_read: !projected.revision.volumes.is_empty(),
            ..ArchiveCapabilities::opened(
                projected.format,
                self.modification_eligibility(&projected, control).err(),
            )
        };
        control.checkpoint()?;
        Ok(projected)
    }

    pub fn execute(
        &self,
        request: ArchiveRequest,
        control: &OperationControl,
    ) -> Result<ArchiveOutcome> {
        self.execute_impl(request, control, None)
    }
    pub(crate) fn execute_retained(
        &self,
        request: ArchiveRequest,
        control: &OperationControl,
        retained: &RetainedCatalog,
    ) -> Result<ArchiveOutcome> {
        self.execute_impl(request, control, Some(retained))
    }
    fn execute_impl(
        &self,
        request: ArchiveRequest,
        control: &OperationControl,
        retained: Option<&RetainedCatalog>,
    ) -> Result<ArchiveOutcome> {
        control.source_delete_begin();
        control.checkpoint()?;
        let snapshot = request.snapshot;
        let mut file = open_snapshot(&snapshot, &self.limits, control)?;
        ensure_revision(&mut file, &snapshot.revision, control)?;
        if !snapshot.backend_safe {
            return Err(ArchiveError::OutputCollision);
        }
        if matches!(
            snapshot.format,
            ArchiveFormat::Gzip | ArchiveFormat::Bzip2 | ArchiveFormat::Xz | ArchiveFormat::Zstd
        ) {
            let outcome = match snapshot.format {
                ArchiveFormat::Gzip => gzip::stream(&mut file, &self.limits, control, |_| Ok(()))?,
                ArchiveFormat::Bzip2 => bzip::stream(&mut file, &self.limits, control, |_| Ok(()))?,
                ArchiveFormat::Xz => xz::stream(&mut file, &self.limits, control, |_| Ok(()))?,
                ArchiveFormat::Zstd => zstd::stream(&mut file, &self.limits, control, |_| Ok(()))?,
                _ => unreachable!("standalone stream format"),
            };
            ensure_revision(&mut file, &snapshot.revision, control)?;
            control.finish_progress(crate::ArchiveProgress {
                bytes: outcome.bytes_verified,
                files: 1,
                total_bytes: None,
            });
            return Ok(outcome);
        }
        let mut total = 0u64;
        for entry in snapshot.entries.iter() {
            control.checkpoint()?;
            let size = entry.size.ok_or(ArchiveError::CorruptArchive)?;
            if size > self.limits.max_entry_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
            total = total.checked_add(size).ok_or(ArchiveError::ResourceLimit)?;
            if total > self.limits.max_output_bytes {
                return Err(ArchiveError::ResourceLimit);
            }
        }
        let operation_memory = (snapshot.entries.len() as u64)
            .checked_mul(128)
            .and_then(|bytes| {
                bytes.checked_add(
                    retained
                        .and_then(|state| state.tar_plan.as_ref())
                        .map_or(0, tar::ValidatedPlan::locator_bytes),
                )
            })
            .and_then(|bytes| bytes.checked_add(256 * 1024))
            .ok_or(ArchiveError::ResourceLimit)?;
        if operation_memory > self.limits.max_working_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let outcome = match snapshot.format {
            ArchiveFormat::Zip => zip::test(
                file.try_clone()?,
                &snapshot,
                &self.limits,
                control,
                self.password,
            )?,
            _ if retained.is_some_and(|retained| retained.tar_plan.is_some()) => {
                let retained = retained.ok_or(ArchiveError::SourceChanged)?;
                tar::test_validated(
                    retained
                        .input
                        .as_ref()
                        .ok_or(ArchiveError::SourceChanged)?
                        .try_clone()?,
                    &snapshot,
                    retained
                        .tar_plan
                        .as_ref()
                        .ok_or(ArchiveError::SourceChanged)?,
                    &self.limits,
                    control,
                )?
            }
            ArchiveFormat::Tar => tar::test(file.try_clone()?, &snapshot, &self.limits, control)?,
            ArchiveFormat::TarGz => {
                let spool = tgz::decode(&mut file, &self.limits, control)?;
                tar::test(spool, &snapshot, &self.limits, control)?
            }
            ArchiveFormat::TarBz2 => {
                let spool = tgz::decode_bzip(&mut file, &self.limits, control)?;
                tar::test(spool, &snapshot, &self.limits, control)?
            }
            ArchiveFormat::TarXz => {
                let spool = tgz::decode_xz(&mut file, &self.limits, control)?;
                tar::test(spool, &snapshot, &self.limits, control)?
            }
            ArchiveFormat::TarZstd => {
                let spool = tgz::decode_zstd(&mut file, &self.limits, control)?;
                tar::test(spool, &snapshot, &self.limits, control)?
            }
            _ => return Err(ArchiveError::UnsupportedOperation),
        };
        ensure_revision(&mut file, &snapshot.revision, control)?;
        control.finish_progress(crate::ArchiveProgress {
            bytes: outcome.bytes_verified,
            files: outcome.files_verified,
            total_bytes: Some(total),
        });
        Ok(outcome)
    }
    pub fn extract(
        &self,
        request: crate::output::ExtractRequest,
        control: &OperationControl,
        output: &mut dyn crate::output::ArchiveOutput,
    ) -> Result<crate::output::ExtractOutcome> {
        self.extract_with_source_lease(request, control, output, None)
    }

    pub(crate) fn extract_with_source_lease(
        &self,
        request: crate::output::ExtractRequest,
        control: &OperationControl,
        output: &mut dyn crate::output::ArchiveOutput,
        source_lease: Option<&crate::worker::NativeReadLease>,
    ) -> Result<crate::output::ExtractOutcome> {
        self.extract_impl(request, control, output, source_lease, None, None, None)
    }
    pub(crate) fn extract_with_zip_plan(
        &self,
        request: crate::output::ExtractRequest,
        control: &OperationControl,
        output: &mut dyn crate::output::ArchiveOutput,
        source_lease: Option<&crate::worker::NativeReadLease>,
        records: &mut zip::ValidatedZip<'_>,
    ) -> Result<crate::output::ExtractOutcome> {
        self.extract_impl(
            request,
            control,
            output,
            source_lease,
            None,
            Some(records),
            None,
        )
    }
    pub(crate) fn extract_with_zip_parallel(
        &self,
        request: crate::output::ExtractRequest,
        control: &OperationControl,
        source_lease: Option<&crate::worker::NativeReadLease>,
        records: &mut zip::ValidatedZip<'_>,
        factory: &dyn zip::parallel::OutputFactory,
    ) -> Result<crate::output::ExtractOutcome> {
        let mut output = factory.output();
        control.enable_parallel_progress(self.limits.max_output_bytes);
        self.extract_impl(
            request,
            control,
            &mut *output,
            source_lease,
            None,
            Some(records),
            Some(factory),
        )
    }
    pub(crate) fn extract_retained(
        &self,
        request: crate::output::ExtractRequest,
        control: &OperationControl,
        output: &mut dyn crate::output::ArchiveOutput,
        source_lease: Option<&crate::worker::NativeReadLease>,
        retained: &RetainedCatalog,
    ) -> Result<crate::output::ExtractOutcome> {
        self.extract_impl(
            request,
            control,
            output,
            source_lease,
            Some(retained),
            None,
            None,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn extract_impl(
        &self,
        request: crate::output::ExtractRequest,
        control: &OperationControl,
        output: &mut dyn crate::output::ArchiveOutput,
        source_lease: Option<&crate::worker::NativeReadLease>,
        retained: Option<&RetainedCatalog>,
        zip_plan: Option<&mut zip::ValidatedZip<'_>>,
        parallel: Option<&dyn zip::parallel::OutputFactory>,
    ) -> Result<crate::output::ExtractOutcome> {
        let lifecycle = control.source_delete_begin();
        control.checkpoint()?;
        let snapshot = request.snapshot;
        if request.plan.session != snapshot.session
            || !std::sync::Arc::ptr_eq(&request.plan.projection, &snapshot.entries)
        {
            return Err(ArchiveError::SourceChanged);
        }
        let source_limits =
            if request.plan.isolated_browser_entry && !snapshot.revision.volumes.is_empty() {
                ResourceLimits {
                    max_output_bytes: ResourceLimits::default().max_output_bytes,
                    ..self.limits
                }
            } else {
                self.limits
            };
        let mut file = open_snapshot(&snapshot, &source_limits, control)?;
        ensure_revision(&mut file, &snapshot.revision, control)?;
        if !snapshot.backend_safe && !request.plan.isolated_browser_entry {
            return Err(ArchiveError::OutputCollision);
        }
        let total = request.plan.total_bytes();
        if total.is_some_and(|total| total > self.limits.max_output_bytes)
            || total.is_none()
                && !matches!(
                    snapshot.format,
                    ArchiveFormat::Gzip
                        | ArchiveFormat::Bzip2
                        | ArchiveFormat::Xz
                        | ArchiveFormat::Zstd
                )
            || request.plan.files().iter().any(|entry| {
                entry
                    .size()
                    .is_some_and(|size| size > self.limits.max_entry_bytes)
            })
        {
            return Err(ArchiveError::ResourceLimit);
        }
        let working = (snapshot.entries.len() as u64)
            .checked_mul(128)
            .and_then(|bytes| {
                bytes.checked_add(
                    retained
                        .and_then(|state| state.tar_plan.as_ref())
                        .map_or(0, tar::ValidatedPlan::locator_bytes),
                )
            })
            .and_then(|value| value.checked_add(256 * 1024))
            .ok_or(ArchiveError::ResourceLimit)?;
        if working > self.limits.max_working_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let outcome = match snapshot.format {
            ArchiveFormat::Zip => {
                let context = zip::ReadContext {
                    password: self.password,
                    source_lease,
                };
                if let Some(records) = zip_plan {
                    let parallel_outcome = if let Some(factory) = parallel {
                        zip::parallel::extract(
                            records,
                            &snapshot,
                            &request.plan,
                            &self.limits,
                            control,
                            context,
                            output,
                            factory,
                        )?
                    } else {
                        None
                    };
                    if let Some(outcome) = parallel_outcome {
                        outcome
                    } else {
                        zip::extract_validated(
                            records,
                            &snapshot,
                            &request.plan,
                            &self.limits,
                            control,
                            context,
                            output,
                        )?
                    }
                } else {
                    zip::extract(
                        file.try_clone()?,
                        &snapshot,
                        &request.plan,
                        &self.limits,
                        control,
                        context,
                        output,
                    )?
                }
            }
            ArchiveFormat::Gzip => gzip::extract(
                file.try_clone()?,
                &snapshot,
                &request.plan,
                &self.limits,
                control,
                output,
            )?,
            ArchiveFormat::Bzip2 => bzip::extract(
                file.try_clone()?,
                &snapshot,
                &request.plan,
                &self.limits,
                control,
                output,
            )?,
            ArchiveFormat::Xz => xz::extract(
                file.try_clone()?,
                &snapshot,
                &request.plan,
                &self.limits,
                control,
                output,
            )?,
            ArchiveFormat::Zstd => zstd::extract(
                file.try_clone()?,
                &snapshot,
                &request.plan,
                &self.limits,
                control,
                output,
            )?,
            _ if retained.is_some_and(|retained| retained.tar_plan.is_some()) => {
                let retained = retained.ok_or(ArchiveError::SourceChanged)?;
                tar::extract_validated(
                    (
                        retained
                            .input
                            .as_ref()
                            .ok_or(ArchiveError::SourceChanged)?
                            .try_clone()?,
                        retained
                            .tar_plan
                            .as_ref()
                            .ok_or(ArchiveError::SourceChanged)?,
                    ),
                    &snapshot,
                    &request.plan,
                    &self.limits,
                    control,
                    &mut file,
                    output,
                )?
            }
            ArchiveFormat::Tar => tar::extract(
                file.try_clone()?,
                &snapshot,
                &request.plan,
                &self.limits,
                control,
                &mut file,
                output,
            )?,
            ArchiveFormat::TarGz => {
                let spool = tgz::decode(&mut file, &self.limits, control)?;
                tar::extract(
                    spool,
                    &snapshot,
                    &request.plan,
                    &self.limits,
                    control,
                    &mut file,
                    output,
                )?
            }
            ArchiveFormat::TarBz2 => {
                let spool = tgz::decode_bzip(&mut file, &self.limits, control)?;
                tar::extract(
                    spool,
                    &snapshot,
                    &request.plan,
                    &self.limits,
                    control,
                    &mut file,
                    output,
                )?
            }
            ArchiveFormat::TarXz => {
                let spool = tgz::decode_xz(&mut file, &self.limits, control)?;
                tar::extract(
                    spool,
                    &snapshot,
                    &request.plan,
                    &self.limits,
                    control,
                    &mut file,
                    output,
                )?
            }
            ArchiveFormat::TarZstd => {
                let spool = tgz::decode_zstd(&mut file, &self.limits, control)?;
                tar::extract(
                    spool,
                    &snapshot,
                    &request.plan,
                    &self.limits,
                    control,
                    &mut file,
                    output,
                )?
            }
            _ => return Err(ArchiveError::UnsupportedOperation),
        };
        ensure_revision(&mut file, &snapshot.revision, control)?;
        let mut directories: Vec<_> = request
            .plan
            .explicit_directories
            .iter()
            .map(|id| {
                usize::try_from(id.ordinal)
                    .ok()
                    .and_then(|index| snapshot.entries.get(index))
                    .filter(|entry| entry.id == *id)
                    .ok_or(ArchiveError::SourceChanged)
            })
            .collect::<Result<_>>()?;
        directories.sort_by_key(|entry| {
            std::cmp::Reverse(entry.display_path.matches(['/', '\\']).count())
        });
        for entry in directories {
            control.checkpoint()?;
            let name = entry.display_path.replace('\\', "/");
            let name = name.trim_end_matches('/');
            let relative = request
                .plan
                .subfolder()
                .map_or_else(|| name.to_owned(), |root| format!("{root}/{name}"));
            entry.metadata.validate()?;
            output.finalize_directory_metadata(&relative, entry.metadata)?;
        }
        control.source_delete_finish(
            &lifecycle,
            if !request.plan.full_archive() {
                "selected_only"
            } else if output.skipped_files() != 0
                || outcome.skipped_special != 0
                || request.plan.skipped_special() != 0
            {
                "members_skipped"
            } else if request.plan.files().is_empty() {
                "empty_source_set"
            } else if outcome.files_committed != request.plan.files().len() as u64 {
                "coverage_incomplete"
            } else {
                "eligible"
            },
            vec![control.operation_id()],
        );
        Ok(outcome)
    }
}

fn revision(file: &mut File, path: &Path, control: &OperationControl) -> Result<SourceRevision> {
    let mut observation = control.observe_revision();
    let before = file.metadata()?;
    if !before.is_file() {
        return Err(ArchiveError::UnsupportedFormat);
    }
    file.rewind()?;
    let mut digest = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        control.checkpoint()?;
        let count = file.read(&mut buffer)?;
        observation.bytes += count as u64;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    let after = file.metadata()?;
    if before.len() != after.len() || before.modified()? != after.modified()? {
        return Err(ArchiveError::SourceChanged);
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(SourceRevision {
        path: path.to_owned(),
        size: before.len(),
        modified: before.modified()?,
        digest: digest.finalize().into(),
        volumes: Vec::new(),
    })
}
pub(crate) fn ensure_revision(
    file: &mut File,
    expected: &SourceRevision,
    control: &OperationControl,
) -> Result<()> {
    // The protected source cannot be written while this worker runs, so the digest taken when it
    // was opened still describes it; confirm the handle is that source without re-reading it.
    if expected.volumes.is_empty() && control.source_protected(&expected.path) {
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.len() != expected.size
            || metadata.modified()? != expected.modified
        {
            return Err(ArchiveError::SourceChanged);
        }
        file.seek(SeekFrom::Start(0))?;
        return Ok(());
    }
    if !expected.volumes.is_empty() {
        for volume in &expected.volumes {
            let mut source = File::open(&volume.path).map_err(|_| ArchiveError::SourceChanged)?;
            if revision(&mut source, &volume.path, control)? != *volume {
                return Err(ArchiveError::SourceChanged);
            }
        }
        return Ok(());
    }
    if revision(file, &expected.path, control)? != *expected {
        return Err(ArchiveError::SourceChanged);
    }
    Ok(())
}

fn volume_revision(
    set: &mut crate::zip_container::DiskSet,
    control: &OperationControl,
) -> Result<SourceRevision> {
    let mut volumes = Vec::new();
    let mut digest = Sha256::new();
    for (path, file) in set.paths.iter().zip(&mut set.files) {
        let item = revision(file, path, control)?;
        digest.update(item.size.to_le_bytes());
        digest.update(item.digest);
        volumes.push(item);
    }
    Ok(SourceRevision {
        path: set
            .paths
            .last()
            .cloned()
            .ok_or(ArchiveError::MissingVolume)?,
        size: set.length,
        modified: volumes.last().ok_or(ArchiveError::MissingVolume)?.modified,
        digest: digest.finalize().into(),
        volumes,
    })
}
pub(crate) fn open_snapshot(
    snapshot: &ArchiveSnapshot,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<File> {
    if snapshot.revision.volumes.is_empty() {
        return Ok(File::open(&snapshot.revision.path)?);
    }
    let mut scratch = tempfile::tempfile()?;
    ensure_revision(&mut scratch, &snapshot.revision, control)?;
    let paths: Vec<String> = snapshot
        .revision
        .volumes
        .iter()
        .map(|volume| volume.path.to_string_lossy().into_owned())
        .collect();
    let mut set =
        crate::zip_container::discover(&snapshot.revision.path, limits, control, Some(&paths))?
            .ok_or(ArchiveError::SourceChanged)?;
    if volume_revision(&mut set, control)? != snapshot.revision {
        return Err(ArchiveError::SourceChanged);
    }
    crate::zip_container::normalize(&mut set, limits, control)
}

fn is_tar(file: &mut File, path: &Path) -> Result<bool> {
    let mut header = [0u8; 512];
    file.rewind()?;
    let size = file.read(&mut header)?;
    file.rewind()?;
    Ok(
        size == 512 && (header[257..262] == *b"ustar" || header.iter().all(|byte| *byte == 0))
            || path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tar")),
    )
}

#[cfg(test)]
mod protected_revision_tests {
    use super::*;
    use std::io::Write;

    fn fixture(path: &Path) {
        let mut zip = ::zip::ZipWriter::new(File::create(path).expect("owned fixture"));
        for index in 0..4 {
            zip.start_file(
                format!("member-{index}.bin"),
                ::zip::write::SimpleFileOptions::default(),
            )
            .expect("member");
            zip.write_all(&vec![index as u8; 65536]).expect("payload");
        }
        zip.finish().expect("complete fixture");
    }

    #[test]
    fn protected_source_is_hashed_once_per_worker() {
        let root = tempfile::tempdir().expect("owned root");
        let path = root.path().join("held.zip");
        fixture(&path);
        let control = OperationControl::default();
        control.protect_source(path.clone());
        let snapshot = ArchiveEngine::default()
            .open(OpenRequest::new(&path), &control)
            .expect("protected open");
        let mut file = File::open(&path).expect("reopen");
        ensure_revision(&mut file, &snapshot.revision, &control).expect("held source unchanged");
        let work = control.work_observation();
        assert_eq!(
            work.revision_passes, 1,
            "only the opening digest reads the held source"
        );
        assert_eq!(work.revision_bytes, path.metadata().expect("size").len());
    }

    #[test]
    fn protected_revision_still_rejects_a_different_file_identity() {
        let root = tempfile::tempdir().expect("owned root");
        let path = root.path().join("held.zip");
        fixture(&path);
        let control = OperationControl::default();
        control.protect_source(path.clone());
        let snapshot = ArchiveEngine::default()
            .open(OpenRequest::new(&path), &control)
            .expect("protected open");
        let mut grown = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("writer");
        grown.write_all(b"tail").expect("grow");
        drop(grown);
        let mut file = File::open(&path).expect("reopen");
        assert!(matches!(
            ensure_revision(&mut file, &snapshot.revision, &control),
            Err(ArchiveError::SourceChanged)
        ));
        let unprotected = OperationControl::default();
        assert!(matches!(
            ensure_revision(&mut file, &snapshot.revision, &unprotected),
            Err(ArchiveError::SourceChanged)
        ));
        assert_eq!(
            unprotected.work_observation().revision_passes,
            1,
            "unprotected path rereads"
        );
    }
}
