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

#[derive(Default)]
pub struct ArchiveEngine<'password> {
    limits: ResourceLimits,
    password: Option<&'password str>,
    zip_volumes: bool,
}
impl<'password> ArchiveEngine<'password> {
    pub fn with_limits(limits: ResourceLimits) -> Self {
        Self {
            limits,
            password: None,
            zip_volumes: false,
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
        control.source_delete_begin();
        control.checkpoint()?;
        if self.zip_volumes
            && let Some(mut set) =
                crate::zip_container::discover(&request.path, &self.limits, control)?
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
        if size >= 4 && matches!(&signature[..4], b"PK\x03\x04" | b"PK\x05\x06") {
            let revision = revision(&mut file, &request.path, control)?;
            let (entries, comment, backend_safe) =
                zip::list(&mut file, request.encoding, &self.limits, control)?;
            let session = entries
                .first()
                .map_or_else(uuid::Uuid::new_v4, |entry| entry.id.session);
            ensure_revision(&mut file, &revision, control)?;
            Ok(ArchiveSnapshot {
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
            })
        } else if size >= 3 && signature[..3] == [0x1f, 0x8b, 8] {
            let revision = revision(&mut file, &request.path, control)?;
            let compound = tgz::matches(&request.path);
            let entries = if compound {
                let mut spool = tgz::decode(&mut file, &self.limits, control)?;
                tar::list(&mut spool, request.encoding, &self.limits, control)?
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
                tar::list(&mut spool, request.encoding, &self.limits, control)?
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
                tar::list(&mut spool, request.encoding, &self.limits, control)?
            } else {
                xz::list(&mut file, &request.path, &self.limits, control)?
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
                tar::list(&mut spool, request.encoding, &self.limits, control)?
            } else {
                zstd::list(&mut file, &request.path, &self.limits, control)?
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
            let entries = tar::list(&mut file, request.encoding, &self.limits, control)?;
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
        }
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
        Ok(ArchiveSnapshot {
            entries: entries.into(),
            comment,
            backend_safe,
            ..snapshot.clone()
        })
    }

    pub fn execute(
        &self,
        request: ArchiveRequest,
        control: &OperationControl,
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
        let lifecycle = control.source_delete_begin();
        control.checkpoint()?;
        let snapshot = request.snapshot;
        if request.plan.session != snapshot.session
            || !std::sync::Arc::ptr_eq(&request.plan.projection, &snapshot.entries)
        {
            return Err(ArchiveError::SourceChanged);
        }
        let mut file = open_snapshot(&snapshot, &self.limits, control)?;
        ensure_revision(&mut file, &snapshot.revision, control)?;
        if !snapshot.backend_safe {
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
            .and_then(|value| value.checked_add(256 * 1024))
            .ok_or(ArchiveError::ResourceLimit)?;
        if working > self.limits.max_working_bytes {
            return Err(ArchiveError::ResourceLimit);
        }
        let outcome = match snapshot.format {
            ArchiveFormat::Zip => zip::extract(
                file.try_clone()?,
                &snapshot,
                &request.plan,
                &self.limits,
                control,
                self.password,
                output,
            )?,
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
    let mut set = crate::zip_container::discover(&snapshot.revision.path, limits, control)?
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
