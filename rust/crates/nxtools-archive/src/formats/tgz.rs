//! Bounded disk spool: complete outer integrity before inner metadata or native output.
use crate::{OperationControl, ResourceLimits, Result};
use std::{
    fs::File,
    io::{Seek, Write},
    path::Path,
};

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
    let mut spool = tempfile::tempfile()?;
    let stream_limits = ResourceLimits {
        max_entry_bytes: limits.max_output_bytes,
        ..*limits
    };
    super::gzip::stream(source, &stream_limits, control, |bytes| {
        spool.write_all(bytes)?;
        Ok(())
    })?;
    spool.flush()?;
    spool.rewind()?;
    Ok(spool)
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
    let mut spool = tempfile::tempfile()?;
    let stream_limits = ResourceLimits {
        max_entry_bytes: limits.max_output_bytes,
        ..*limits
    };
    super::bzip::stream(source, &stream_limits, control, |bytes| {
        spool.write_all(bytes)?;
        Ok(())
    })?;
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
    let mut spool = tempfile::tempfile()?;
    let stream_limits = ResourceLimits {
        max_entry_bytes: limits.max_output_bytes,
        ..*limits
    };
    super::xz::stream(source, &stream_limits, control, |bytes| {
        spool.write_all(bytes)?;
        Ok(())
    })?;
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
    let mut spool = tempfile::tempfile()?;
    let stream_limits = ResourceLimits {
        max_entry_bytes: limits.max_output_bytes,
        ..*limits
    };
    super::zstd::stream(source, &stream_limits, control, |bytes| {
        spool.write_all(bytes)?;
        Ok(())
    })?;
    spool.flush()?;
    spool.rewind()?;
    Ok(spool)
}
