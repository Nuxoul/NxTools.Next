//! Bounded disk spool: complete outer integrity before inner metadata or native output.
use crate::{OperationControl, ResourceLimits, Result};
use std::{
    fs::File,
    io::{Seek, Write},
    path::Path,
};

/// Completed spool writes, including bytes retained before a failed outer decode. This is
/// anonymous-spool output, not encoded source I/O or target-output accounting.
pub(crate) fn observe_spool(spool: &File, control: &OperationControl) {
    if let Ok(metadata) = spool.metadata() {
        control.record_work(|work| work.outer_decoded_bytes += metadata.len());
    }
}

pub(crate) fn matches(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let name = name.to_ascii_lowercase();
            name.ends_with(".tar.gz") || name.ends_with(".tgz")
        })
}
pub(crate) fn decode(
    source: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<File> {
    decode_members(source, limits, control).map(|(spool, _)| spool)
}
/// The count comes from the same full CRC/footer/member validation that produced the spool.
pub(crate) fn decode_members(
    source: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<(File, u64)> {
    control.record_work(|work| work.outer_decodes += 1);
    let mut spool = tempfile::tempfile()?;
    let stream_limits = ResourceLimits {
        max_entry_bytes: limits.max_output_bytes,
        ..*limits
    };
    let decoded = super::gzip::stream_members(source, &stream_limits, control, |bytes| {
        spool.write_all(bytes)?;
        Ok(())
    });
    observe_spool(&spool, control);
    let (_, members) = decoded?;
    spool.flush()?;
    spool.rewind()?;
    Ok((spool, members))
}

pub(crate) fn matches_bzip(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let name = name.to_ascii_lowercase();
            name.ends_with(".tar.bz2") || name.ends_with(".tbz2") || name.ends_with(".tbz")
        })
}
pub(crate) fn decode_bzip(
    source: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<File> {
    control.record_work(|work| work.outer_decodes += 1);
    let mut spool = tempfile::tempfile()?;
    let stream_limits = ResourceLimits {
        max_entry_bytes: limits.max_output_bytes,
        ..*limits
    };
    let decoded = super::bzip::stream(source, &stream_limits, control, |bytes| {
        spool.write_all(bytes)?;
        Ok(())
    });
    observe_spool(&spool, control);
    decoded?;
    spool.flush()?;
    spool.rewind()?;
    Ok(spool)
}

pub(crate) fn matches_xz(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let name = name.to_ascii_lowercase();
            name.ends_with(".tar.xz") || name.ends_with(".txz")
        })
}
pub(crate) fn decode_xz(
    source: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<File> {
    control.record_work(|work| work.outer_decodes += 1);
    let mut spool = tempfile::tempfile()?;
    let stream_limits = ResourceLimits {
        max_entry_bytes: limits.max_output_bytes,
        ..*limits
    };
    let decoded = super::xz::stream(source, &stream_limits, control, |bytes| {
        spool.write_all(bytes)?;
        Ok(())
    });
    observe_spool(&spool, control);
    decoded?;
    spool.flush()?;
    spool.rewind()?;
    Ok(spool)
}

pub(crate) fn matches_zstd(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            let name = name.to_ascii_lowercase();
            name.ends_with(".tar.zst") || name.ends_with(".tar.zstd") || name.ends_with(".tzst")
        })
}
pub(crate) fn decode_zstd(
    source: &mut File,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<File> {
    control.record_work(|work| work.outer_decodes += 1);
    let mut spool = tempfile::tempfile()?;
    let stream_limits = ResourceLimits {
        max_entry_bytes: limits.max_output_bytes,
        ..*limits
    };
    let decoded = super::zstd::stream(source, &stream_limits, control, |bytes| {
        spool.write_all(bytes)?;
        Ok(())
    });
    observe_spool(&spool, control);
    decoded?;
    spool.flush()?;
    spool.rewind()?;
    Ok(spool)
}
