//! Immutable worker-side readmission for the host-held ordered source set.
//! The host owns native object/namespace protection; this verifies encoded bytes at
//! each decode boundary. No sibling discovery or path synthesis happens in this module.
use super::*;
use std::io::SeekFrom;

pub(super) struct SourceSet {
    paths: Vec<std::path::PathBuf>,
    revisions: Vec<([u8; 32], u64)>,
    digest: [u8; 32],
    length: u64,
}
impl SourceSet {
    #[cfg(windows)]
    pub(super) fn open_rar(request: &WorkerRequest, control: &OperationControl) -> Result<Self> {
        // RAR metadata must be reachable with a deficient output allowance. Reuse the
        // original finite default for encoded input only, never for actual decode/output.
        let encoded_limit =
            if request.operation == "extract" && request.output_readmission.is_some() {
                request
                    .limits
                    .max_output_bytes
                    .max(ResourceLimits::default().max_output_bytes)
            } else {
                request.limits.max_output_bytes
            };
        Self::open_bounded(request, control, encoded_limit)
    }
    pub(super) fn open_sevenz(request: &WorkerRequest, control: &OperationControl) -> Result<Self> {
        let encoded_limit = if request.output_readmission.is_some() {
            request
                .limits
                .max_output_bytes
                .max(ResourceLimits::default().max_output_bytes)
        } else {
            request.limits.max_output_bytes
        };
        Self::open_bounded(request, control, encoded_limit)
    }
    fn open_bounded(
        request: &WorkerRequest,
        control: &OperationControl,
        encoded_limit: u64,
    ) -> Result<Self> {
        let paths: Vec<std::path::PathBuf> = if request.volume_paths.is_empty() {
            vec![request.path.clone().into()]
        } else {
            if request.volume_paths.len() > 1024 || request.volume_paths[0] != request.path {
                return Err(ArchiveError::MissingVolume);
            }
            request
                .volume_paths
                .iter()
                .map(Into::into)
                .collect::<Vec<_>>()
        };
        let mut revisions = Vec::with_capacity(paths.len());
        let mut total = 0u64;
        // Declared-output readmission must reach the original header/plan preflight even
        // when the deficient output allowance is smaller than the encoded source itself.
        // Keep encoded scanning finite under the existing default source/decode ceiling;
        // this does not change actual output, dictionary, metadata or native Job budgets.
        let mut digest = Sha256::new();
        for path in &paths {
            control.checkpoint()?;
            let part = fingerprint(path, control)?;
            if part.1 == 0 {
                return Err(ArchiveError::CorruptArchive);
            }
            total = total
                .checked_add(part.1)
                .ok_or(ArchiveError::ResourceLimit)?;
            if total > encoded_limit {
                return Err(ArchiveError::ResourceLimit);
            }
            digest.update(part.1.to_le_bytes());
            digest.update(part.0);
            revisions.push(part);
        }
        let combined = if revisions.len() == 1 {
            revisions[0].0
        } else {
            digest.finalize().into()
        };
        Ok(Self {
            paths,
            revisions,
            digest: combined,
            length: total,
        })
    }
    pub(super) fn digest(&self) -> [u8; 32] {
        self.digest
    }
    pub(super) fn length(&self) -> u64 {
        self.length
    }
    pub(super) fn primary(&self) -> &Path {
        &self.paths[0]
    }
    pub(super) fn verify(&self, control: &OperationControl) -> Result<()> {
        for (path, revision) in self.paths.iter().zip(&self.revisions) {
            if fingerprint(path, control)? != *revision {
                return Err(ArchiveError::SourceChanged);
            }
        }
        Ok(())
    }
    pub(super) fn sevenz_input(&self, control: &OperationControl) -> Result<File> {
        // An anonymous encoded spool is the exact concatenation of a first .7z.001
        // signature + subsequent encoded segments. The original codec retains all
        // header/CRC/dictionary/entry and payload-integrity checks on this seekable file.
        // No plaintext is spooled and no candidate receives publication authority.
        if self.paths.len() == 1 {
            return Ok(File::open(&self.paths[0])?);
        }
        let mut spool = tempfile::tempfile()?;
        let mut copied = 0u64;
        for (path, revision) in self.paths.iter().zip(&self.revisions) {
            control.checkpoint()?;
            let mut remaining = revision.1;
            let mut source = File::open(path)?;
            let mut buffer = [0u8; 65536];
            while remaining != 0 {
                control.checkpoint()?;
                let bound = remaining.min(buffer.len() as u64) as usize;
                let count = source.read(&mut buffer[..bound])?;
                if count == 0 {
                    return Err(ArchiveError::SourceChanged);
                }
                spool.write_all(&buffer[..count])?;
                remaining -= count as u64;
            }
            copied = copied
                .checked_add(revision.1)
                .ok_or(ArchiveError::ResourceLimit)?;
        }
        if copied != self.length {
            return Err(ArchiveError::SourceChanged);
        }
        self.verify(control)?;
        spool.seek(SeekFrom::Start(0))?;
        Ok(spool)
    }
}
