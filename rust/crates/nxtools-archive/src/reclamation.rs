//! Whole-file Stored ZIP reclamation decisions. Windows mutation and durable I/O stay native.
use crate::{
    ArchiveError, ArchiveFormat, ArchiveProgress, ArchiveSnapshot, EntryKind, OperationControl,
    ResourceLimits, Result,
    metadata::OutputMetadata,
    output::ArchiveOutput,
    worker::WorkerRequest,
    worker_output::{PrivateOutput, ReplyPort},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use nxtools_protocol::archive_job::ArchiveReclamationMode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{fs::File, io::Write};
use uuid::Uuid;

const MAX_JOURNAL: usize = 32 << 20;
const MAX_RECORD: usize = 65536;
const MAX_FILES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanEntry {
    pub index: u32,
    pub ordinal: u64,
    pub path: String,
    pub header_start: u64,
    pub payload_start: u64,
    pub payload_end: u64,
    pub record_end: u64,
    pub release_start: u64,
    pub release_end: u64,
    pub size: u64,
    pub crc32: u32,
    pub metadata: OutputMetadata,
}
#[derive(Deserialize)]
struct NativeInfo {
    schema: u32,
    recovery_id: Uuid,
    cluster_bytes: u64,
    journal_bytes: u64,
    source_identity: String,
    root_identity: String,
}
#[derive(Deserialize)]
struct Envelope {
    schema: u32,
    sequence: u64,
    kind: String,
    value: Value,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum UnitState {
    #[default]
    Absent,
    Partial,
    Ready,
    Pending,
    Applied,
}
#[derive(Default)]
struct Recovery {
    entries: Vec<PlanEntry>,
    units: Vec<UnitState>,
    structure: Option<String>,
    completed: bool,
    released_bytes: u64,
}
fn invalid<T>() -> Result<T> {
    Err(ArchiveError::ReclamationRecoveryInvalid)
}

/// Parse actual native append frames. A complete pending intent is never discarded to retry.
fn replay(bytes: &[u8]) -> Result<Recovery> {
    if bytes.len() > MAX_JOURNAL {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut result = Recovery::default();
    let mut offset = 0usize;
    let mut sequence = 1u64;
    let mut frontier = 0usize;
    while offset < bytes.len() {
        let tail = &bytes[offset..];
        if tail.len() < 4 {
            return if result.units.get(frontier) == Some(&UnitState::Pending) {
                Ok(result)
            } else {
                invalid()
            };
        }
        let length = u32::from_le_bytes([tail[0], tail[1], tail[2], tail[3]]) as usize;
        if length == 0 || length > MAX_RECORD {
            return invalid();
        }
        if tail.len() < length + 8 {
            return if result.units.get(frontier) == Some(&UnitState::Pending) {
                Ok(result)
            } else {
                invalid()
            };
        }
        if tail[length + 4..length + 8] != tail[..4] {
            return invalid();
        }
        let record: Envelope = serde_json::from_slice(&tail[4..length + 4])
            .map_err(|_| ArchiveError::ReclamationRecoveryInvalid)?;
        if record.schema != 1 || record.sequence != sequence || result.completed {
            return invalid();
        }
        match record.kind.as_str() {
            "header" if sequence == 1 && record.value.is_object() => {}
            "plan_entry" if sequence > 1 && result.structure.is_none() => {
                let entry: PlanEntry = serde_json::from_value(record.value)
                    .map_err(|_| ArchiveError::ReclamationRecoveryInvalid)?;
                if result.entries.len() >= MAX_FILES
                    || entry.index as usize != result.entries.len()
                    || entry.release_end < entry.release_start
                    || entry.header_start > entry.payload_start
                    || entry.payload_start > entry.payload_end
                    || entry.payload_end > entry.record_end
                    || entry.release_start != entry.release_end
                        && (entry.release_start < entry.payload_start
                            || entry.release_end > entry.payload_end)
                    || entry.release_start == entry.release_end && entry.release_start != 0
                    || entry.payload_end - entry.payload_start != entry.size
                {
                    return invalid();
                }
                result.entries.push(entry);
            }
            "sealed" if sequence > 1 && result.structure.is_none() => {
                if record.value["count"].as_u64() != Some(result.entries.len() as u64)
                    || result.entries.is_empty()
                {
                    return invalid();
                }
                let structure = record.value["structure"]
                    .as_str()
                    .ok_or(ArchiveError::ReclamationRecoveryInvalid)?;
                if structure.len() > MAX_RECORD {
                    return invalid();
                }
                result.structure = Some(structure.to_owned());
                result.units.resize(result.entries.len(), UnitState::Absent);
            }
            "partial" | "ready" | "pending" | "applied" if result.structure.is_some() => {
                let index = record.value["index"]
                    .as_u64()
                    .ok_or(ArchiveError::ReclamationRecoveryInvalid)?;
                if index != frontier as u64 || frontier >= result.units.len() {
                    return invalid();
                }
                let state = &mut result.units[frontier];
                let next = match (record.kind.as_str(), *state) {
                    ("partial", UnitState::Absent) => UnitState::Partial,
                    ("ready", UnitState::Partial) => UnitState::Ready,
                    ("pending", UnitState::Ready) => UnitState::Pending,
                    ("applied", UnitState::Pending) => UnitState::Applied,
                    _ => return invalid(),
                };
                *state = next;
                if next == UnitState::Applied {
                    let released = record.value["released_bytes"]
                        .as_u64()
                        .ok_or(ArchiveError::ReclamationRecoveryInvalid)?;
                    if released
                        > result.entries[frontier].release_end
                            - result.entries[frontier].release_start
                    {
                        return invalid();
                    }
                    result.released_bytes = result
                        .released_bytes
                        .checked_add(released)
                        .ok_or(ArchiveError::ResourceLimit)?;
                    frontier += 1;
                }
            }
            "completed_consumed"
                if result.structure.is_some() && frontier == result.entries.len() =>
            {
                result.completed = true;
            }
            _ => return invalid(),
        }
        offset += length + 8;
        sequence += 1;
    }
    if sequence == 1 {
        return invalid();
    }
    Ok(result)
}
fn call_json<W: Write>(
    output: &mut PrivateOutput<'_, W>,
    action: &str,
    value: &impl Serialize,
) -> Result<Option<String>> {
    let bytes = serde_json::to_vec(value).map_err(|_| ArchiveError::ReclamationRecoveryInvalid)?;
    if bytes.len() > MAX_RECORD {
        return Err(ArchiveError::ResourceLimit);
    }
    Ok(output.call(action, None, None, &bytes)?.read_data)
}
fn json_reply<T: serde::de::DeserializeOwned>(reply: Option<String>) -> Result<T> {
    let text = reply.ok_or(ArchiveError::ReclamationRecoveryInvalid)?;
    if text.len() > 87384 {
        return Err(ArchiveError::ResourceLimit);
    }
    serde_json::from_str(&text).map_err(|_| ArchiveError::ReclamationRecoveryInvalid)
}
fn aligned_interval(start: u64, end: u64, cluster: u64) -> Result<(u64, u64)> {
    if cluster == 0 || !cluster.is_power_of_two() || cluster > 2 << 20 || end < start {
        return Err(ArchiveError::ReclamationUnsupported);
    }
    let first = start
        .checked_add(cluster - 1)
        .ok_or(ArchiveError::ResourceLimit)?
        / cluster
        * cluster;
    let last = end / cluster * cluster;
    Ok(if first < last { (first, last) } else { (0, 0) })
}

pub(crate) fn execute<W: Write + Send>(
    request: &WorkerRequest,
    control: &OperationControl,
    writer: &mut W,
    replies: ReplyPort,
) -> Result<(u64, u64)> {
    let intent = request
        .extract_options
        .as_ref()
        .and_then(|options| options.reclamation)
        .ok_or(ArchiveError::ReclamationUnsupported)?;
    if request.volume_paths.len() > 1 || !request.path.to_ascii_lowercase().ends_with(".zip") {
        return Err(ArchiveError::ReclamationUnsupported);
    }
    let limits: ResourceLimits = request.limits.into();
    let mut source = File::open(&request.path)?;
    tracing::info!(target: "nxtools_archive", operation_id=%control.operation_id(), recovery_id=%intent.recovery_id,
        mode=?intent.mode, phase="accepted", "archive.reclamation");
    let mut bounded = limits;
    bounded.max_entries = bounded.max_entries.min(MAX_FILES as u64);
    let (entries, comment, safe) = crate::formats::zip::list(
        &mut source,
        request
            .encoding()
            .map_err(|_| ArchiveError::ReclamationUnsupported)?,
        &bounded,
        control,
    )?;
    if !safe
        || entries.iter().any(|entry| {
            let path = entry.display_path().replace('\\', "/").to_ascii_lowercase();
            matches!(
                path.as_str(),
                "[content_types].xml"
                    | "meta-inf/manifest.mf"
                    | "androidmanifest.xml"
                    | "classes.dex"
            )
        })
    {
        return Err(ArchiveError::ReclamationUnsupported);
    }
    let crate::formats::zip::ReclamationLayout { extents, structure } =
        crate::formats::zip::reclamation_layout(&mut source, &entries, &bounded, control)?;
    let source_metadata = source.metadata()?;
    let session = entries
        .first()
        .ok_or(ArchiveError::ReclamationUnsupported)?
        .id()
        .session;
    let snapshot = ArchiveSnapshot {
        revision: crate::model::SourceRevision {
            path: request.path.clone().into(),
            size: source_metadata.len(),
            modified: source_metadata.modified()?,
            digest: [0; 32],
            volumes: Vec::new(),
        },
        session,
        format: ArchiveFormat::Zip,
        capabilities: Default::default(),
        entries: entries.into(),
        comment,
        backend_safe: safe,
    };
    if crate::zip_edit::blocked_container(&snapshot) {
        return Err(ArchiveError::ReclamationUnsupported);
    }
    let plan = crate::worker_job::plan(request, &snapshot, control)?;
    if plan.subfolder().is_some() || plan.skipped_special() != 0 || plan.files().is_empty() {
        return Err(ArchiveError::ReclamationUnsupported);
    }
    if plan
        .total_bytes()
        .is_none_or(|total| total > limits.max_output_bytes)
        || plan
            .files()
            .iter()
            .any(|file| file.size().is_none_or(|size| size > limits.max_entry_bytes))
    {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut output = PrivateOutput::new(writer, control, replies).extraction_metadata(true);
    let info: NativeInfo = json_reply(output.call("hz5_info", None, None, &[])?.read_data)?;
    if info.schema != 1
        || info.recovery_id != intent.recovery_id
        || info.source_identity.is_empty()
        || info.source_identity.len() > 256
        || info.root_identity.is_empty()
        || info.root_identity.len() > 256
        || info.journal_bytes > MAX_JOURNAL as u64
    {
        return invalid();
    }
    let mut units = Vec::with_capacity(plan.files().len());
    for (index, planned) in plan.files().iter().enumerate() {
        let entry = &snapshot.entries()[planned.id().ordinal as usize];
        let (start, end, record_end) = extents[planned.id().ordinal as usize];
        let (release_start, release_end) = aligned_interval(start, end, info.cluster_bytes)?;
        units.push(PlanEntry {
            index: index as u32,
            ordinal: planned.id().ordinal,
            path: planned.relative_path().to_owned(),
            header_start: entry
                .zip
                .as_ref()
                .ok_or(ArchiveError::ReclamationUnsupported)?
                .header_offset,
            payload_start: start,
            payload_end: end,
            record_end,
            release_start,
            release_end,
            size: entry.size().ok_or(ArchiveError::ReclamationUnsupported)?,
            crc32: entry.crc32().ok_or(ArchiveError::CorruptArchive)?,
            metadata: entry.metadata(),
        });
    }
    if !units
        .iter()
        .any(|unit| unit.release_end > unit.release_start)
    {
        return Err(ArchiveError::ReclamationNoAllocation);
    }
    let encoded_structure = STANDARD.encode(structure);
    let state = if intent.mode == ArchiveReclamationMode::Continue {
        let mut bytes = Vec::with_capacity(info.journal_bytes as usize);
        while bytes.len() < info.journal_bytes as usize {
            let count = (info.journal_bytes as usize - bytes.len()).min(MAX_RECORD) as u32;
            let encoded = output
                .call_at(
                    "hz5_read",
                    None,
                    None,
                    &[],
                    Some((bytes.len() as u64, count)),
                )?
                .read_data
                .ok_or(ArchiveError::ReclamationRecoveryInvalid)?;
            if encoded.len() > 87384 {
                return Err(ArchiveError::ResourceLimit);
            }
            let part = STANDARD
                .decode(encoded)
                .map_err(|_| ArchiveError::ReclamationRecoveryInvalid)?;
            if part.is_empty() || part.len() > count as usize {
                return invalid();
            }
            bytes.extend_from_slice(&part);
        }
        let state = replay(&bytes)?;
        if state.entries != units || state.structure.as_deref() != Some(encoded_structure.as_str())
        {
            return invalid();
        }
        state
    } else {
        // Charge both the immutable plan and finite append allowance to the existing metadata owner.
        let plan_bytes = serde_json::to_vec(&units)
            .map_err(|_| ArchiveError::ResourceLimit)?
            .len() as u64;
        if plan_bytes
            .checked_add(MAX_JOURNAL as u64)
            .is_none_or(|bytes| bytes > limits.max_metadata_bytes)
            || (MAX_JOURNAL as u64).saturating_add(plan_bytes) > limits.max_working_bytes
        {
            return Err(ArchiveError::ResourceLimit);
        }
        for unit in &units {
            call_json(&mut output, "hz5_plan_entry", unit)?;
        }
        call_json(
            &mut output,
            "hz5_seal",
            &json!({"count":units.len(),"structure":encoded_structure}),
        )?;
        Recovery {
            units: vec![UnitState::Absent; units.len()],
            ..Default::default()
        }
    };
    // Validate the entire consumed prefix before any new release, not lazily after mutation.
    for (index, stage) in state.units.iter().enumerate() {
        if *stage == UnitState::Absent {
            break;
        }
        let observed: Value = json_reply(call_json(
            &mut output,
            "hz5_observe",
            &json!({"index":index}),
        )?)?;
        let expected = if *stage == UnitState::Applied {
            "applied"
        } else {
            "published"
        };
        if observed["status"].as_str() != Some(expected) {
            return invalid();
        }
    }
    if state.completed {
        let bytes = plan
            .total_bytes()
            .ok_or(ArchiveError::ReclamationRecoveryInvalid)?;
        let files = units.len() as u64;
        control.finish_progress(ArchiveProgress {
            bytes,
            files,
            total_bytes: Some(bytes),
        });
        return Ok((files, bytes));
    }
    for directory in plan.directories() {
        output.ensure_directory(directory)?;
    }
    let mut files = 0u64;
    let mut bytes = 0u64;
    let mut decoded = 0u64;
    for (index, unit) in units.iter().enumerate() {
        control.checkpoint()?;
        let current = state.units[index];
        if current != UnitState::Applied {
            if current == UnitState::Absent {
                call_json(&mut output, "hz5_select", &json!({"index":index}))?;
                let entry = &snapshot.entries()[unit.ordinal as usize];
                control.set_current_member(&unit.path);
                let token = output.begin_file(&unit.path)?;
                // No generic abort after a destructive unit begins: the persistent native owner
                // decides whether an owned partial is disposable or indispensable on cleanup.
                crate::formats::zip::stream_entry(
                    &mut source,
                    entry,
                    &limits,
                    control,
                    None,
                    &mut decoded,
                    |chunk| output.write_file(token, chunk),
                )?;
                output.set_file_metadata(token, unit.metadata)?;
                output.commit_file(token)?;
            }
            if current != UnitState::Pending {
                call_json(&mut output, "hz5_pending", &json!({"index":index}))?;
            }
            let applied: Value = json_reply(call_json(
                &mut output,
                "hz5_apply",
                &json!({"index":index}),
            )?)?;
            if applied["released_bytes"]
                .as_u64()
                .is_none_or(|released| released > unit.release_end - unit.release_start)
            {
                return invalid();
            }
            tracing::info!(target: "nxtools_archive", operation_id=%control.operation_id(), recovery_id=%intent.recovery_id,
                phase="applied", checkpoint=index, released_bytes=applied["released_bytes"].as_u64(), "archive.reclamation");
        }
        files += 1;
        bytes = bytes
            .checked_add(unit.size)
            .ok_or(ArchiveError::ResourceLimit)?;
        control.publish(ArchiveProgress {
            bytes,
            files,
            total_bytes: plan.total_bytes(),
        });
    }
    let mut directories = plan
        .explicit_directories
        .iter()
        .map(|id| &snapshot.entries()[id.ordinal as usize])
        .collect::<Vec<_>>();
    directories
        .sort_by_key(|entry| std::cmp::Reverse(entry.display_path().matches(['/', '\\']).count()));
    for entry in directories {
        if entry.kind() != EntryKind::Directory {
            return invalid();
        }
        output.finalize_directory_metadata(
            entry
                .display_path()
                .replace('\\', "/")
                .trim_end_matches('/'),
            entry.metadata(),
        )?;
    }
    if !state.completed {
        output.call("hz5_complete", None, None, &[])?;
    }
    control.finish_progress(ArchiveProgress {
        bytes,
        files,
        total_bytes: Some(bytes),
    });
    Ok((files, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn frame(sequence: u64, kind: &str, value: Value) -> Vec<u8> {
        let body =
            serde_json::to_vec(&json!({"schema":1,"sequence":sequence,"kind":kind,"value":value}))
                .expect("encode recovery record");
        let mut bytes = (body.len() as u32).to_le_bytes().to_vec();
        bytes.extend_from_slice(&body);
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes
    }
    fn prefix() -> Vec<u8> {
        let entry = PlanEntry {
            index: 0,
            ordinal: 0,
            path: "a.bin".into(),
            header_start: 0,
            payload_start: 35,
            payload_end: 16419,
            record_end: 16419,
            release_start: 4096,
            release_end: 16384,
            size: 16384,
            crc32: 1,
            metadata: Default::default(),
        };
        [
            frame(1, "header", json!({})),
            frame(
                2,
                "plan_entry",
                serde_json::to_value(entry).expect("encode plan entry"),
            ),
            frame(3, "sealed", json!({"count":1,"structure":"UEs="})),
        ]
        .concat()
    }
    #[test]
    fn reclamation_release_requires_durable_ready_then_pending_and_monotone_frontier() {
        let mut bytes = prefix();
        bytes.extend(frame(
            4,
            "applied",
            json!({"index":0,"released_bytes":4096}),
        ));
        assert!(replay(&bytes).is_err());
        let mut bytes = prefix();
        bytes.extend(frame(4, "partial", json!({"index":0})));
        bytes.extend(frame(5, "ready", json!({"index":0})));
        bytes.extend(frame(6, "pending", json!({"index":0})));
        assert_eq!(
            replay(&bytes).expect("valid pending journal").units,
            vec![UnitState::Pending]
        );
        let mut torn = bytes.clone();
        torn.extend([20, 0, 0, 0, b'{']);
        assert_eq!(
            replay(&torn).expect("pending permits torn tail").units,
            vec![UnitState::Pending]
        );
        bytes.extend(frame(
            7,
            "applied",
            json!({"index":0,"released_bytes":12288}),
        ));
        bytes.extend(frame(8, "completed_consumed", json!({})));
        let recovered = replay(&bytes).expect("completed recovery journal");
        assert!(recovered.completed);
        assert_eq!(recovered.released_bytes, 12288);
        bytes.extend(frame(9, "pending", json!({"index":0})));
        assert!(replay(&bytes).is_err());
    }
    #[test]
    fn reclamation_complete_corruption_gap_and_unapproved_torn_tail_are_refused() {
        let mut bytes = prefix();
        bytes.push(1);
        assert!(replay(&bytes).is_err());
        let mut bytes = prefix();
        bytes.extend(frame(5, "partial", json!({"index":0})));
        assert!(replay(&bytes).is_err());
        let mut bytes = prefix();
        bytes.extend(frame(4, "unknown", json!({})));
        assert!(replay(&bytes).is_err());
        let mut bytes = prefix();
        let last = bytes.len() - 1;
        bytes[last] ^= 1;
        assert!(replay(&bytes).is_err());
    }
    #[test]
    fn reclamation_alignment_never_includes_header_or_following_record() {
        assert_eq!(
            aligned_interval(35, 16419, 4096).expect("aligned payload"),
            (4096, 16384)
        );
        assert_eq!(
            aligned_interval(35, 99, 4096).expect("short payload"),
            (0, 0)
        );
        assert!(aligned_interval(1, 100, 0).is_err());
        assert!(aligned_interval(1, 100, 3000).is_err());
        assert!(aligned_interval(u64::MAX - 1, u64::MAX, 4096).is_err());
    }
}

#[cfg(test)]
mod layout_tests {
    use super::*;
    use std::io::{Seek, SeekFrom};

    #[test]
    fn reclamation_qualifies_stored_and_preserves_structure_after_payload_consumption() {
        let scratch = std::env::var_os("PI_SCRATCH_DIR").expect("session scratch root");
        let root = std::path::PathBuf::from(scratch).join(format!("hz5-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).expect("create owned fixture");
        let path = root.join("owned.zip");
        let mut writer = zip::ZipWriter::new(File::create(&path).expect("create owned ZIP"));
        writer
            .start_file(
                "payload.bin",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )
            .expect("start stored file");
        writer
            .write_all(&vec![0x5a; 65536])
            .expect("write stored payload");
        writer.finish().expect("finish stored ZIP");
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .expect("open owned ZIP");
        let control = OperationControl::default();
        let limits = ResourceLimits::default();
        let (entries, _, _) =
            crate::formats::zip::list(&mut file, crate::NameEncoding::Auto, &limits, &control)
                .expect("list stored ZIP");
        let crate::formats::zip::ReclamationLayout {
            extents,
            structure: before,
        } = crate::formats::zip::reclamation_layout(&mut file, &entries, &limits, &control)
            .expect("qualify stored ZIP");
        let (start, end, _) = extents[0];
        let (first, last) = aligned_interval(start, end, 4096).expect("aligned fixture payload");
        assert!(first >= start && last <= end && first < last);
        file.seek(SeekFrom::Start(first))
            .expect("seek owned payload");
        file.write_all(&vec![0; (last - first) as usize])
            .expect("consume owned payload");
        let (fresh, _, _) =
            crate::formats::zip::list(&mut file, crate::NameEncoding::Auto, &limits, &control)
                .expect("list retained ZIP structure");
        let crate::formats::zip::ReclamationLayout {
            extents: recovered,
            structure: after,
        } = crate::formats::zip::reclamation_layout(&mut file, &fresh, &limits, &control)
            .expect("qualify retained structure");
        assert_eq!(extents, recovered);
        assert_eq!(before, after);
        let error = crate::formats::zip::stream_entry(
            &mut file,
            &fresh[0],
            &limits,
            &control,
            None,
            &mut 0,
            |_| Ok(()),
        )
        .expect_err("consumed payload must fail ordinary CRC verification");
        assert_eq!(
            error,
            ArchiveError::CorruptArchive,
            "consumed payload must never reenter ordinary decode"
        );
        drop(file);
        std::fs::remove_dir_all(root).expect("remove owned fixture");
    }

    #[test]
    fn reclamation_refuses_deflate_and_zip64_before_any_output() {
        let scratch = std::env::var_os("PI_SCRATCH_DIR").expect("session scratch root");
        let root = std::path::PathBuf::from(scratch).join(format!("hz5-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).expect("create owned qualification fixture");
        for (name, method, large) in [
            ("deflate.zip", zip::CompressionMethod::Deflated, false),
            ("zip64.zip", zip::CompressionMethod::Stored, true),
        ] {
            let path = root.join(name);
            let mut writer =
                zip::ZipWriter::new(File::create(&path).expect("create qualification ZIP"));
            writer
                .start_file(
                    "a.bin",
                    zip::write::SimpleFileOptions::default()
                        .compression_method(method)
                        .large_file(large),
                )
                .expect("start qualification member");
            writer
                .write_all(&vec![7; 65536])
                .expect("write qualification payload");
            writer.finish().expect("finish qualification ZIP");
            let mut file = File::open(path).expect("open qualification ZIP");
            let limits = ResourceLimits::default();
            let control = OperationControl::default();
            let (entries, _, _) =
                crate::formats::zip::list(&mut file, crate::NameEncoding::Auto, &limits, &control)
                    .expect("list qualification ZIP");
            assert!(
                crate::formats::zip::reclamation_layout(&mut file, &entries, &limits, &control)
                    .is_err()
            );
        }
        std::fs::remove_dir_all(root).expect("remove qualification fixture");
    }
}
