//! Browser-only revision and single ordinal policy. Reuses original source/output runtime.
use crate::{
    ArchiveEntry, ArchiveError, EntryKind, OperationControl, Result,
    worker::{WorkerFrame, WorkerRequest, write_frame},
    worker_output::{PrivateOutput, ReplyPort},
};
use nxtools_protocol::archive_browser::{ArchiveBrowserContext, ArchiveSourceBinding};
use serde::{Deserialize, Serialize};
use std::{io::Write, time::UNIX_EPOCH};
use uuid::Uuid;
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BrowserEntry {
    pub ordinal: u64,
    pub expected: ArchiveBrowserContext,
}
pub(crate) fn selected<'a>(
    request: &WorkerRequest,
    entries: &'a [ArchiveEntry],
) -> Result<Option<&'a ArchiveEntry>> {
    let Some(selection) = &request.browser_entry else {
        return Ok(None);
    };
    let entry = entries
        .get(usize::try_from(selection.ordinal).map_err(|_| ArchiveError::SourceChanged)?)
        .ok_or(ArchiveError::SourceChanged)?;
    if entry.id().ordinal != selection.ordinal {
        return Err(ArchiveError::SourceChanged);
    }
    if entry.kind() != EntryKind::File
        || entry
            .warnings()
            .iter()
            .any(|warning| *warning != crate::EntryWarning::UncertainEncoding)
    {
        return Err(ArchiveError::UnsupportedOperation);
    }
    if entry.size().is_some_and(|size| size > 512 << 20) {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(Some(entry))
}

/// Solid blocks may decode unselected bytes, but these are not temporary output.
/// Retain the engine's original finite decode-work ceilings and the admitted working memory.
#[cfg(feature = "isolated-codecs")]
pub(crate) fn decode_limits(request: &WorkerRequest) -> crate::ResourceLimits {
    let mut limits: crate::ResourceLimits = request.limits.into();
    if request.browser_entry.is_some() {
        let original = crate::ResourceLimits::default();
        limits.max_entry_bytes = original.max_entry_bytes;
        limits.max_output_bytes = original.max_output_bytes;
    }
    limits
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn bind<W: Write>(
    request: &WorkerRequest,
    snapshot: Uuid,
    format: &str,
    comment: &str,
    digest: [u8; 32],
    size: u64,
    modified: std::time::SystemTime,
    output: &mut W,
    control: &OperationControl,
    replies: ReplyPort,
) -> Result<()> {
    if !(request.browser_entry.is_some()
        || request.browser_authorize.is_some()
        || request
            .job_extract
            .as_ref()
            .is_some_and(|options| options.selection.is_some())
        || request.read_contract == Some(2)
            && request
                .read_transport
                .is_some_and(|transport| transport.browser_context)
            && request.operation == "open")
    {
        return Ok(());
    }
    let modified_unix_ns = u64::try_from(
        modified
            .duration_since(UNIX_EPOCH)
            .map_err(|_| ArchiveError::SourceChanged)?
            .as_nanos(),
    )
    .map_err(|_| ArchiveError::SourceChanged)?;
    let native = PrivateOutput::new(output, control, replies).source_identity()?;
    let source = ArchiveSourceBinding {
        native,
        digest,
        size,
        modified_unix_ns,
    };
    if let Some(selection) = request
        .job_extract
        .as_ref()
        .and_then(|options| options.selection.as_ref())
    {
        if source != selection.browser.source || format != selection.browser.format {
            return Err(ArchiveError::SourceChanged);
        }
        return Ok(());
    }
    if let Some(expected) = &request.browser_authorize {
        if source != expected.source || format != expected.format {
            return Err(ArchiveError::SourceChanged);
        }
        return Ok(());
    }
    if let Some(selection) = &request.browser_entry {
        if source != selection.expected.source || format != selection.expected.format {
            return Err(ArchiveError::SourceChanged);
        }
    } else {
        // One bounded context is sent even for an empty archive; no zero-entry batch/extra credit.
        let mut end = comment.len().min(65536);
        while !comment.is_char_boundary(end) {
            end -= 1;
        }
        let mut frame = WorkerFrame::state(request.operation_id, "browser_context");
        frame.browser = Some(ArchiveBrowserContext {
            snapshot_id: snapshot,
            format: format.into(),
            comment: comment[..end].into(),
            source,
        });
        write_frame(output, &frame).map_err(|_| ArchiveError::IoFailure)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ArchiveEngine, OpenRequest};

    #[test]
    fn encrypted_single_entry_requires_private_interaction_and_preserves_snapshot() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/combo-ae2-store-256.zip");
        let control = OperationControl::default();
        let snapshot = ArchiveEngine::default()
            .open(OpenRequest::new(&path), &control)
            .expect("small encrypted ZIP fixture");
        let encrypted = snapshot
            .entries()
            .iter()
            .find(|entry| entry.encrypted())
            .expect("encrypted entry");
        let mut request = WorkerRequest::open(Uuid::new_v4(), path.to_string_lossy().into_owned());
        request.version = 2;
        request.operation = "extract".into();
        request.limits.max_entry_bytes = 512 << 20;
        request.limits.max_output_bytes = 512 << 20;
        request.limits.max_entries = 100_000;
        request.limits.max_metadata_bytes = 128 << 20;
        request.browser_entry = Some(BrowserEntry {
            ordinal: encrypted.id().ordinal,
            expected: ArchiveBrowserContext {
                snapshot_id: Uuid::new_v4(),
                format: "Zip".into(),
                comment: String::new(),
                source: ArchiveSourceBinding {
                    native: "v1:test".into(),
                    digest: [0; 32],
                    size: 0,
                    modified_unix_ns: 0,
                },
            },
        });
        assert!(selected(&request, snapshot.entries()).is_ok());
        request.password_interaction = Some(true);
        assert!(request.validate().is_ok());
        request.password = Some("must-not-be-in-browser-start".into());
        assert!(request.validate().is_err());
    }
}
