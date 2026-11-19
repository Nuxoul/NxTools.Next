//! All candidate volumes precede first publication. The original native journal owns each fact.
use crate::{
    ArchiveError, OperationControl, Result, output::ArchiveOutput, worker::WorkerRequest,
    worker_output::PrivateOutput,
};
use std::{
    fs::File,
    io::{Read, Seek, Write},
};

pub(crate) fn publish<W: Write + Send>(
    encoded: &mut File,
    request: &WorkerRequest,
    native: &mut PrivateOutput<'_, W>,
    control: &OperationControl,
) -> Result<u64> {
    let name = request
        .create_name
        .as_deref()
        .ok_or(ArchiveError::UnsupportedOperation)?;
    let self_extracting = request.operation == "create_zip"
        && request
            .zip_options
            .as_ref()
            .is_some_and(|options| options.self_extracting);
    let mut candidates = if request.operation == "create_zip" {
        if self_extracting {
            crate::zip_container::sfx_encoded(
                encoded,
                request.volume_bytes,
                &request.limits.into(),
                control,
            )?
        } else {
            crate::zip_container::split_encoded(
                encoded,
                request
                    .volume_bytes
                    .ok_or(ArchiveError::UnsupportedOperation)?,
                &request.limits.into(),
                control,
            )?
        }
    } else {
        let maximum = request
            .volume_bytes
            .ok_or(ArchiveError::UnsupportedOperation)?;
        let length = encoded.metadata()?.len();
        let count = length.div_ceil(maximum).max(1);
        if count > 1024 {
            return Err(ArchiveError::ResourceLimit);
        }
        encoded.rewind()?;
        let mut result = Vec::new();
        let mut remaining = length;
        let mut bytes = [0u8; 65536];
        for _ in 0..count {
            let mut candidate = tempfile::tempfile()?;
            let mut left = remaining.min(maximum);
            remaining -= left;
            while left > 0 {
                control.checkpoint()?;
                let count = left.min(bytes.len() as u64) as usize;
                encoded.read_exact(&mut bytes[..count])?;
                candidate.write_all(&bytes[..count])?;
                left -= count as u64;
            }
            candidate.rewind()?;
            result.push(candidate);
        }
        result
    };
    if candidates.len() > 1024 {
        return Err(ArchiveError::ResourceLimit);
    }
    let count = candidates.len();
    let mut tokens = Vec::new();
    let staged = (|| {
        for (index, candidate) in candidates.iter_mut().enumerate() {
            control.checkpoint()?;
            let target = if request.operation == "create_zip" {
                let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
                if self_extracting && index == 0 {
                    name.to_owned()
                } else if index + 1 == count {
                    if self_extracting {
                        format!("{stem}.zip")
                    } else {
                        name.to_owned()
                    }
                } else {
                    format!("{stem}.z{:02}", index + 1)
                }
            } else {
                format!("{name}.{:03}", index + 1)
            };
            let token = native.begin_file(&target)?;
            tokens.push(token);
            // The existing full encoded output was already independently decoded/verified.
            // Same-held-token byte readback verifies every encoded segment before any publication.
            let _readback = crate::worker_tree_io::stage_and_readback(
                candidate,
                native,
                token,
                request.limits.max_output_bytes,
                control,
            )?;
            native.call("flush_volume", None, Some(token), &[])?;
        }
        if request.operation == "create_zip" {
            let mut set = crate::zip_container::candidate_set(&candidates, &request.limits.into())?;
            let mut normalized =
                crate::zip_container::normalize(&mut set, &request.limits.into(), control)?;
            crate::zip_edit::verify(
                &mut normalized,
                &request.limits.into(),
                control,
                request.password.as_deref(),
            )?;
        }
        native.call(
            "volumes_verified",
            None,
            None,
            &serde_json::to_vec(&tokens).map_err(|_| ArchiveError::IoFailure)?,
        )?;
        native.source_checkpoint("source_precommit")?;
        for token in &tokens {
            control.checkpoint()?;
            native.commit_file(*token)?;
        }
        Ok(tokens.len() as u64)
    })();
    if staged.is_err() {
        for token in tokens {
            native.abort_file(token);
        }
    }
    staged
}
