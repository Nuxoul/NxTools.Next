//! General job extraction reuses the original immutable plan and native output transaction.
use crate::planning::{ExtractionPlan, ExtractionSelection, TargetStrategy};
use crate::{
    ArchiveEntry, ArchiveError, ArchiveSnapshot, OperationControl, Result, worker::WorkerRequest,
};
use nxtools_protocol::archive_job::{ArchiveJobExtract, ArchiveJobStrategy};

pub(crate) fn stem(path: &str) -> String {
    let file = path.rsplit(['\\', '/']).next().unwrap_or("archive");
    let lower = file.to_ascii_lowercase();
    for suffix in [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst", ".tar.zstd"] {
        if lower.ends_with(suffix) {
            return file[..file.len() - suffix.len()].into();
        }
    }
    file.rsplit_once('.')
        .map_or(file, |(base, _)| base)
        .to_owned()
}
pub(crate) fn plan(request: &WorkerRequest, snapshot: &ArchiveSnapshot) -> Result<ExtractionPlan> {
    if let Some(options) = &request.job_extract {
        let selection = if let Some(selected) = &options.selection {
            ExtractionSelection::Entries(
                selected
                    .ordinals
                    .iter()
                    .map(|ordinal| {
                        snapshot
                            .entries()
                            .get(*ordinal as usize)
                            .filter(|entry| entry.id().ordinal == *ordinal)
                            .map(ArchiveEntry::id)
                            .ok_or(ArchiveError::SourceChanged)
                    })
                    .collect::<Result<Vec<_>>>()?,
            )
        } else {
            ExtractionSelection::All
        };
        if snapshot.entries().len() > 65536
            || request.limits.max_metadata_bytes
                < (snapshot.entries().len() as u64).saturating_mul(512)
        {
            return Err(ArchiveError::ResourceLimit);
        }
        let plan = ExtractionPlan::build(snapshot, selection, strategy(options, &request.path))?;
        if plan.files().len() > 4096 {
            return Err(ArchiveError::ResourceLimit);
        }
        Ok(plan)
    } else {
        if let Some(entry) = crate::worker_browser::selected(request, snapshot.entries())? {
            ExtractionPlan::browser_entry(snapshot, entry)
        } else {
            ExtractionPlan::build(snapshot, ExtractionSelection::All, TargetStrategy::Here)
        }
    }
}
fn strategy(options: &ArchiveJobExtract, path: &str) -> TargetStrategy {
    match options.strategy {
        ArchiveJobStrategy::Here => TargetStrategy::Here,
        ArchiveJobStrategy::Name => TargetStrategy::Name(stem(path)),
        ArchiveJobStrategy::Smart => TargetStrategy::Smart {
            archive_stem: stem(path),
            single_file_folder: options.single_file_folder,
        },
    }
}

/// Only fully known planned regular-file sizes can authorize output-only readmission.
pub(crate) fn declared_output(
    plan: &ExtractionPlan,
) -> Option<crate::output_readmission::OutputBudget> {
    Some(crate::output_readmission::OutputBudget {
        entry_bytes: plan
            .files()
            .iter()
            .try_fold(1u64, |largest, file| Some(largest.max(file.size()?)))?,
        output_bytes: plan.total_bytes()?.max(1),
    })
}

/// Extension selects only the existing verified compound decoder, never archive capability.
pub(crate) fn compound(path: &str) -> Option<crate::ArchiveFormat> {
    let path = std::path::Path::new(path);
    if crate::formats::tgz::matches(path) {
        Some(crate::ArchiveFormat::TarGz)
    } else if crate::formats::tgz::matches_bzip(path) {
        Some(crate::ArchiveFormat::TarBz2)
    } else if crate::formats::tgz::matches_xz(path) {
        Some(crate::ArchiveFormat::TarXz)
    } else if crate::formats::tgz::matches_zstd(path) {
        Some(crate::ArchiveFormat::TarZstd)
    } else {
        None
    }
}
pub(crate) fn spool_limits(limits: crate::ResourceLimits) -> crate::ResourceLimits {
    crate::ResourceLimits {
        // Header discovery cannot use a deficient target-output allowance. This is a finite
        // anonymous disk spool reservation, not a changed Job/request/inner-output allowance.
        max_output_bytes: limits.max_output_bytes.max(limits.max_metadata_bytes),
        ..limits
    }
}
pub(crate) fn extract_compound(
    request: &WorkerRequest,
    snapshot: &ArchiveSnapshot,
    plan: &ExtractionPlan,
    control: &OperationControl,
    output: &mut dyn crate::output::ArchiveOutput,
) -> Result<crate::output::ExtractOutcome> {
    let limits: crate::ResourceLimits = request.limits.into();
    let working = (snapshot.entries().len() as u64)
        .checked_mul(128)
        .and_then(|bytes| bytes.checked_add(256 * 1024))
        .ok_or(ArchiveError::ResourceLimit)?;
    if working > limits.max_working_bytes {
        return Err(ArchiveError::ResourceLimit);
    }
    let spool_limits = spool_limits(limits);
    let mut source = std::fs::File::open(&request.path)?;
    crate::engine::ensure_revision(&mut source, &snapshot.revision, control)?;
    let spool = match snapshot.format() {
        crate::ArchiveFormat::TarGz => {
            crate::formats::tgz::decode(&mut source, &spool_limits, control)?
        }
        crate::ArchiveFormat::TarBz2 => {
            crate::formats::tgz::decode_bzip(&mut source, &spool_limits, control)?
        }
        crate::ArchiveFormat::TarXz => {
            crate::formats::tgz::decode_xz(&mut source, &spool_limits, control)?
        }
        crate::ArchiveFormat::TarZstd => {
            crate::formats::tgz::decode_zstd(&mut source, &spool_limits, control)?
        }
        _ => return Err(ArchiveError::UnsupportedOperation),
    };
    // Existing TAR transaction validates the immutable records, enforces the ORIGINAL target
    // byte limits, and stages through full original-source recheck before native commits.
    let result =
        crate::formats::tar::extract(spool, snapshot, plan, &limits, control, &mut source, output)?;
    let mut directories = plan
        .explicit_directories
        .iter()
        .map(|id| {
            snapshot
                .entries()
                .get(id.ordinal as usize)
                .filter(|entry| entry.id() == *id)
                .ok_or(ArchiveError::SourceChanged)
        })
        .collect::<Result<Vec<_>>>()?;
    directories
        .sort_by_key(|entry| std::cmp::Reverse(entry.display_path().matches(['/', '\\']).count()));
    for entry in directories {
        control.checkpoint()?;
        let name = entry.display_path().replace('\\', "/");
        let name = name.trim_end_matches('/');
        let relative = plan
            .subfolder()
            .map_or_else(|| name.to_owned(), |root| format!("{root}/{name}"));
        entry.metadata.validate()?;
        output.finalize_directory_metadata(&relative, entry.metadata)?;
    }
    crate::engine::ensure_revision(&mut source, &snapshot.revision, control)?;
    Ok(result)
}
#[cfg(feature = "isolated-codecs")]
pub(crate) fn codec_plan(
    request: &WorkerRequest,
    entries: std::sync::Arc<[ArchiveEntry]>,
) -> Result<Option<ExtractionPlan>> {
    if request.job_extract.is_none() && request.output_readmission.is_none() {
        return Ok(None);
    }
    let snapshot = ArchiveSnapshot {
        revision: crate::model::SourceRevision {
            path: request.path.clone().into(),
            size: 0,
            modified: std::time::UNIX_EPOCH,
            digest: [0; 32],
            volumes: Vec::new(),
        },
        session: request.operation_id,
        format: crate::ArchiveFormat::SevenZ,
        capabilities: Default::default(),
        entries,
        comment: String::new(),
        backend_safe: true,
    };
    plan(request, &snapshot).map(Some)
}

// A writer wrapper samples the ORIGINAL control only at serialized frame boundaries. Native
// chunks/checkpoints already bound those boundaries; no second writer, queue or timer thread.
pub(crate) struct ProgressWriter<'a, W> {
    pub inner: &'a mut W,
    pub control: &'a OperationControl,
    pub enabled: bool,
    last: std::time::Instant,
    header: [u8; 4],
    header_len: usize,
    remaining: usize,
}
impl<'a, W> ProgressWriter<'a, W> {
    pub fn new(inner: &'a mut W, control: &'a OperationControl, enabled: bool) -> Self {
        Self {
            inner,
            control,
            enabled,
            last: std::time::Instant::now(),
            header: [0; 4],
            header_len: 0,
            remaining: 0,
        }
    }
    fn at_frame_boundary(&self) -> bool {
        self.header_len == 0 && self.remaining == 0
    }
}
impl<W: std::io::Write> std::io::Write for ProgressWriter<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let written = self.inner.write(bytes)?;
        let mut cursor = &bytes[..written];
        while !cursor.is_empty() {
            if self.header_len < 4 {
                let take = cursor.len().min(4 - self.header_len);
                self.header[self.header_len..self.header_len + take]
                    .copy_from_slice(&cursor[..take]);
                self.header_len += take;
                cursor = &cursor[take..];
                if self.header_len == 4 {
                    self.remaining = u32::from_le_bytes(self.header) as usize;
                }
            } else {
                let take = cursor.len().min(self.remaining);
                self.remaining -= take;
                cursor = &cursor[take..];
                if self.remaining == 0 {
                    self.header_len = 0;
                }
            }
        }
        Ok(written)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()?;
        if self.enabled
            && self.at_frame_boundary()
            && self.last.elapsed() >= std::time::Duration::from_millis(100)
        {
            self.last = std::time::Instant::now();
            if let Some(snapshot) = self.control.progress_snapshot() {
                let progress = snapshot.progress;
                let mut frame = serde_json::json!({"schema":1,"operation_id":self.control.operation_id(),"op":"progress","progress":{"phase":snapshot.phase,"bytes_done":progress.bytes,"bytes_total":progress.total_bytes,"files_done":progress.files,"files_total":snapshot.files_total,"current_member":snapshot.current_member}});
                if let Some(done) = snapshot.current_bytes_done {
                    frame["progress"]["current_bytes_done"] = serde_json::json!(done);
                }
                if let Some(total) = snapshot.current_bytes_total {
                    frame["progress"]["current_bytes_total"] = serde_json::json!(total);
                }
                crate::worker::write_frame(self.inner, &frame)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn extraction_snapshot_keeps_member_and_plan_totals_independent_of_commits() {
        let control = OperationControl::default();
        control.configure_extraction_progress(Some(30), 2);
        control.begin_extracted_member("first.txt", Some(10));
        control.publish_extraction_progress(
            crate::ArchiveProgress {
                bytes: 4,
                files: 0,
                total_bytes: None,
            },
            4,
        );
        let mut bytes = Vec::new();
        let mut writer = ProgressWriter::new(&mut bytes, &control, true);
        writer.last = std::time::Instant::now() - std::time::Duration::from_millis(100);
        writer.flush().expect("first member snapshot");
        let first_size = writer.inner.len();
        let first: serde_json::Value =
            crate::worker::read_frame(&mut std::io::Cursor::new(&writer.inner[..first_size]))
                .expect("frame")
                .expect("progress");
        assert_eq!(first["progress"]["bytes_done"], 4);
        assert_eq!(first["progress"]["bytes_total"], 30);
        assert_eq!(first["progress"]["files_done"], 0);
        assert_eq!(first["progress"]["files_total"], 2);
        assert_eq!(first["progress"]["current_member"], "first.txt");
        assert_eq!(first["progress"]["current_bytes_done"], 4);
        assert_eq!(first["progress"]["current_bytes_total"], 10);
        control.begin_extracted_member("second.txt", None);
        control.publish_extraction_progress(
            crate::ArchiveProgress {
                bytes: 10,
                files: 1,
                total_bytes: None,
            },
            0,
        );
        writer.last = std::time::Instant::now() - std::time::Duration::from_millis(100);
        writer.flush().expect("second member snapshot");
        let second: serde_json::Value =
            crate::worker::read_frame(&mut std::io::Cursor::new(&writer.inner[first_size..]))
                .expect("frame")
                .expect("progress");
        assert_eq!(second["progress"]["bytes_done"], 10);
        assert_eq!(second["progress"]["files_done"], 1);
        assert_eq!(second["progress"]["current_member"], "second.txt");
        assert_eq!(second["progress"]["current_bytes_done"], 0);
        assert!(second["progress"].get("current_bytes_total").is_none());
    }
    #[test]
    fn extraction_unknown_zero_overrun_and_phase_clear_keep_truthful_wire() {
        let control = OperationControl::default();
        control.configure_extraction_progress(None, 2);
        control.begin_extracted_member("empty", Some(0));
        let mut bytes = Vec::new();
        let mut writer = ProgressWriter::new(&mut bytes, &control, true);
        writer.last = std::time::Instant::now() - std::time::Duration::from_millis(100);
        writer.flush().expect("zero snapshot");
        let first_size = writer.inner.len();
        let empty: serde_json::Value =
            crate::worker::read_frame(&mut std::io::Cursor::new(&writer.inner[..first_size]))
                .expect("frame")
                .expect("progress");
        assert!(empty["progress"]["bytes_total"].is_null());
        assert_eq!(empty["progress"]["current_bytes_done"], 0);
        assert_eq!(empty["progress"]["current_bytes_total"], 0);
        control.clear_extracted_member();
        control.begin_extracted_member("overrun", Some(1));
        control.publish_extraction_progress(
            crate::ArchiveProgress {
                bytes: 2,
                files: 0,
                total_bytes: None,
            },
            2,
        );
        writer.last = std::time::Instant::now() - std::time::Duration::from_millis(100);
        writer.flush().expect("processed overrun");
        let overrun: serde_json::Value =
            crate::worker::read_frame(&mut std::io::Cursor::new(&writer.inner[first_size..]))
                .expect("frame")
                .expect("progress");
        assert_eq!(overrun["progress"]["bytes_done"], 2);
        assert_eq!(overrun["progress"]["files_done"], 0);
        assert_eq!(overrun["progress"]["current_bytes_done"], 2);
        assert!(overrun["progress"].get("current_bytes_total").is_none());
        control.set_progress_phase("edit_reading");
        control.finish_progress(crate::ArchiveProgress {
            bytes: 1,
            files: 0,
            total_bytes: None,
        });
        let snapshot = control.progress_snapshot().expect("stage snapshot");
        assert_eq!(snapshot.phase, "edit_reading");
        assert!(
            snapshot.current_member.is_none()
                && snapshot.current_bytes_done.is_none()
                && snapshot.files_total.is_none()
        );
        let creation = OperationControl::default();
        creation.publish_extraction_progress(
            crate::ArchiveProgress {
                bytes: 7,
                files: 0,
                total_bytes: None,
            },
            7,
        );
        assert_eq!(
            creation
                .latest_progress()
                .expect("original creation progress")
                .bytes,
            7
        );
    }
    #[test]
    fn original_progress_one_thousand_updates_emit_only_time_bounded_serial_snapshot() {
        let control = OperationControl::default();
        control.set_current_member("bounded/current.txt");
        let mut bytes = Vec::new();
        let mut writer = ProgressWriter::new(&mut bytes, &control, true);
        for value in 0..1000 {
            control.finish_progress(crate::ArchiveProgress {
                bytes: value,
                files: 0,
                total_bytes: Some(1000),
            });
            writer.flush().expect("bounded flush");
        }
        assert!(
            writer.inner.is_empty(),
            "byte milestones must not bypass100ms"
        );
        writer.last = std::time::Instant::now() - std::time::Duration::from_millis(100);
        writer.flush().expect("one latest snapshot");
        let frame: serde_json::Value =
            crate::worker::read_frame(&mut std::io::Cursor::new(&*writer.inner))
                .expect("serialized frame")
                .expect("progress");
        assert_eq!(frame["progress"]["bytes_done"], 999);
        assert_eq!(frame["progress"]["current_member"], "bounded/current.txt");
        let size = writer.inner.len();
        writer.flush().expect("same time");
        assert_eq!(writer.inner.len(), size);
    }
    #[test]
    fn stdout_flush_between_header_and_body_cannot_inject_progress_into_private_frame() {
        let control = OperationControl::default();
        control.finish_progress(crate::ArchiveProgress {
            bytes: 1,
            files: 0,
            total_bytes: None,
        });
        let mut bytes = Vec::new();
        let mut writer = ProgressWriter::new(&mut bytes, &control, true);
        writer.last = std::time::Instant::now() - std::time::Duration::from_millis(100);
        let body = br#"{"op":"native_call"}"#;
        writer
            .write_all(&(body.len() as u32).to_le_bytes())
            .expect("header");
        writer.flush().expect("midframe flush");
        assert_eq!(writer.inner.len(), 4, "no midframe injection");
        writer.write_all(body).expect("body");
        writer.flush().expect("boundary progress");
        let mut cursor = std::io::Cursor::new(&*writer.inner);
        let first: serde_json::Value = crate::worker::read_frame(&mut cursor)
            .expect("intact first")
            .expect("first");
        let second: serde_json::Value = crate::worker::read_frame(&mut cursor)
            .expect("intact progress")
            .expect("second");
        assert_eq!(first["op"], "native_call");
        assert_eq!(second["op"], "progress");
    }
}
