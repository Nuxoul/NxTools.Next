//! Durable delta transactions: write cost follows changed rows, not the cumulative overlay.
use super::*;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom};

const JOURNAL_MAGIC: u64 = 0x0000_474F_4C4A_4D4E; // "NMJLOG\0"
const JOURNAL_VERSION: u32 = 1;
const JOURNAL_HEADER: usize = 128;
const FRAME_HEADER: usize = 40;
const CHECKSUM: usize = 32;
const FULL: u32 = 1;
const MAX_FRAME_BYTES: usize = 256 * 1024 * 1024;
const MIN_CHECKPOINT_OVERHEAD: u64 = 1024 * 1024;

#[derive(Clone, Debug)]
pub(in crate::mft) struct DeltaCheckpoint {
    overlay: MftOverlay,
    journal_id: Option<u64>,
    next_usn: Option<i64>,
    visible_count: usize,
    valid_len: u64,
    checkpoint_len: u64,
    base_token: [u8; CHECKSUM],
}

pub(super) fn is_journal(bytes: &[u8]) -> bool {
    bytes
        .get(..8)
        .is_some_and(|magic| magic == JOURNAL_MAGIC.to_le_bytes())
}

fn base_token(path: &Path, mount: &str, base_cursor: Option<i64>, rows: usize) -> [u8; CHECKSUM] {
    let mut digest = Sha256::new();
    digest.update(mount.to_ascii_lowercase().as_bytes());
    digest.update([0]);
    digest.update(
        path.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .as_bytes(),
    );
    digest.update([0]);
    digest.update(base_cursor.unwrap_or(NONE_I64).to_le_bytes());
    digest.update((rows as u64).to_le_bytes());
    digest.finalize().into()
}

pub(super) fn validate_base_binding(
    bytes: &[u8],
    path: &Path,
    mount: &str,
    base_cursor: Option<i64>,
    rows: usize,
) -> Result<()> {
    ensure!(
        bytes.len() >= JOURNAL_HEADER,
        "delta journal binding truncated"
    );
    ensure!(
        read_i64(bytes, 24)? == base_cursor.unwrap_or(NONE_I64)
            && bytes[32..64] == base_token(path, mount, base_cursor, rows),
        "delta belongs to another immutable base"
    );
    Ok(())
}

fn checkpoint(
    snapshot: &MftSnapshot,
    valid_len: u64,
    checkpoint_len: u64,
    base_token: [u8; CHECKSUM],
) -> DeltaCheckpoint {
    DeltaCheckpoint {
        overlay: snapshot.overlay.clone(),
        journal_id: snapshot.journal_id,
        next_usn: snapshot.next_usn,
        visible_count: snapshot.visible_len(),
        valid_len,
        checkpoint_len,
        base_token,
    }
}

pub(super) fn persist(snapshot: &MftSnapshot, path: &Path) -> Result<()> {
    // Snapshot clones share only the last successfully synced disk state. Failed commits never
    // acknowledge newer rows/cursors, and the existing maintenance owner retries its latest view.
    let mut durable = snapshot
        .delta_checkpoint
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let token = base_token(
        snapshot
            .cache_path()
            .context("delta omitted base identity")?,
        &snapshot.mount_point,
        snapshot.base_next_usn,
        snapshot.base_len(),
    );
    if let Some(previous) = durable.as_ref() {
        ensure!(
            previous.base_token == token,
            "durable delta base identity changed"
        );
        ensure!(
            previous.journal_id == snapshot.journal_id,
            "delta journal identity changed"
        );
        ensure!(
            previous.next_usn <= snapshot.next_usn,
            "refusing stale delta cursor"
        );
        let existing_len = match fs::metadata(path) {
            Ok(metadata) => Some(metadata.len()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error).context("inspect incremental delta"),
        };
        if existing_len == Some(previous.valid_len)
            && previous.next_usn == snapshot.next_usn
            && previous.visible_count == snapshot.visible_len()
            && previous.overlay == snapshot.overlay
        {
            return Ok(());
        }
        let frame = encode_frame(snapshot, Some(&previous.overlay))?;
        let limit = previous
            .checkpoint_len
            .saturating_add(previous.checkpoint_len.max(MIN_CHECKPOINT_OVERHEAD));
        if existing_len.is_some_and(|length| length >= previous.valid_len)
            && previous.valid_len.saturating_add(frame.len() as u64) <= limit
        {
            let mut file = OpenOptions::new()
                .write(true)
                .open(path)
                .context("open incremental delta commit")?;
            let length = file.metadata()?.len();
            ensure!(
                length >= previous.valid_len,
                "durable delta changed during append admission"
            );
            if length != previous.valid_len {
                file.set_len(previous.valid_len)
                    .context("retire incomplete delta tail")?;
            }
            file.seek(SeekFrom::Start(previous.valid_len))?;
            file.write_all(&frame)
                .context("append incremental delta transaction")?;
            file.sync_all()
                .context("sync incremental delta transaction")?;
            *durable = Some(checkpoint(
                snapshot,
                previous.valid_len + frame.len() as u64,
                previous.checkpoint_len,
                token,
            ));
            return Ok(());
        }
        // Missing/short backing files can be rebuilt from the latest usable memory state;
        // failure still leaves the old successful owner intact for the normal retry path.
    }
    let frame = encode_frame(snapshot, None)?;
    let parent = path.parent().context("delta journal omitted directory")?;
    fs::create_dir_all(parent)?;
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = path.with_extension(format!("tmp-{}-{nonce}", std::process::id()));
    let result = (|| -> Result<()> {
        let mut file = File::create(&temporary)?;
        let mut header = [0_u8; JOURNAL_HEADER];
        put_u64(&mut header, 0, JOURNAL_MAGIC);
        put_u32(&mut header, 8, JOURNAL_VERSION);
        put_u32(&mut header, 12, JOURNAL_HEADER as u32);
        put_u64(&mut header, 16, snapshot.journal_id.unwrap_or(NONE_U64));
        put_i64(&mut header, 24, snapshot.base_next_usn.unwrap_or(NONE_I64));
        header[32..64].copy_from_slice(&token);
        let header_digest = Sha256::digest(&header[..96]);
        header[96..128].copy_from_slice(&header_digest);
        file.write_all(&header)?;
        file.write_all(&frame)?;
        file.sync_all()?;
        drop(file);
        atomic_replace(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    let length = (JOURNAL_HEADER + frame.len()) as u64;
    *durable = Some(checkpoint(snapshot, length, length, token));
    Ok(())
}

fn encode_frame(snapshot: &MftSnapshot, previous: Option<&MftOverlay>) -> Result<Vec<u8>> {
    let mut replacements = snapshot
        .overlay
        .replacements
        .iter()
        .filter(|(reference, entry)| {
            previous.is_none_or(|old| old.replacements.get(reference) != Some(*entry))
        })
        .collect::<Vec<_>>();
    replacements.sort_unstable_by_key(|(reference, _)| **reference);
    let mut deleted = snapshot
        .overlay
        .deleted
        .iter()
        .filter(|reference| previous.is_none_or(|old| !old.deleted.contains(reference)))
        .copied()
        .collect::<Vec<_>>();
    deleted.sort_unstable();
    let mut reset = previous
        .into_iter()
        .flat_map(|old| old.replacements.keys().chain(old.deleted.iter()))
        .filter(|reference| {
            !snapshot.overlay.replacements.contains_key(reference)
                && !snapshot.overlay.deleted.contains(reference)
        })
        .copied()
        .collect::<Vec<_>>();
    reset.sort_unstable();
    reset.dedup();
    let mut bytes = vec![0_u8; FRAME_HEADER];
    put_i64(&mut bytes, 8, snapshot.next_usn.unwrap_or(NONE_I64));
    put_u64(&mut bytes, 16, snapshot.visible_len() as u64);
    put_u32(&mut bytes, 24, u32::try_from(replacements.len())?);
    put_u32(&mut bytes, 28, u32::try_from(deleted.len())?);
    put_u32(&mut bytes, 32, u32::try_from(reset.len())?);
    put_u32(&mut bytes, 36, if previous.is_none() { FULL } else { 0 });
    for (reference, entry) in replacements {
        ensure!(
            bytes
                .len()
                .checked_add(DELTA_RECORD_SIZE)
                .and_then(|size| size.checked_add(entry.path.len()))
                .and_then(|size| size.checked_add(entry.name.len()))
                .is_some_and(|size| size <= MAX_FRAME_BYTES - CHECKSUM),
            "delta transaction exceeds bounded frame"
        );
        let mut record = [0_u8; DELTA_RECORD_SIZE];
        put_u64(&mut record, 0, *reference);
        let mut flags = 0;
        if let Some(identity) = entry.identity {
            ensure!(
                identity.file_reference == *reference,
                "delta replacement identity mismatch"
            );
            flags |= FLAG_IDENTITY;
            put_u64(&mut record, 8, identity.parent_reference);
        }
        put_u64(&mut record, 16, entry.size_bytes.unwrap_or(NONE_U64));
        put_i64(
            &mut record,
            24,
            entry.modified_at_unix_ms.unwrap_or(NONE_I64),
        );
        put_u32(&mut record, 32, u32::try_from(entry.path.len())?);
        put_u32(&mut record, 36, u32::try_from(entry.name.len())?);
        if entry.is_directory {
            flags |= FLAG_DIRECTORY;
        }
        record[48] = flags;
        bytes.extend_from_slice(&record);
        bytes.extend_from_slice(entry.path.as_bytes());
        bytes.extend_from_slice(entry.name.as_bytes());
    }
    for reference in deleted.into_iter().chain(reset) {
        bytes.extend_from_slice(&reference.to_le_bytes());
    }
    ensure!(
        bytes.len() <= MAX_FRAME_BYTES - CHECKSUM,
        "delta transaction exceeds bounded frame"
    );
    let length = (bytes.len() + CHECKSUM) as u64;
    put_u64(&mut bytes, 0, length);
    let digest = Sha256::digest(&bytes);
    bytes.extend_from_slice(&digest);
    Ok(bytes)
}

pub(super) fn decode(bytes: &[u8], expected_journal: Option<u64>) -> Result<PersistedDelta> {
    ensure!(
        bytes.len() >= JOURNAL_HEADER,
        "delta journal header truncated"
    );
    ensure!(
        is_journal(bytes)
            && read_u32(bytes, 8)? == JOURNAL_VERSION
            && read_u32(bytes, 12)? as usize == JOURNAL_HEADER,
        "delta journal header invalid"
    );
    ensure!(
        Sha256::digest(&bytes[..96]).as_slice() == &bytes[96..128],
        "delta journal header checksum mismatch"
    );
    let raw_id = read_u64(bytes, 16)?;
    let journal_id = (raw_id != NONE_U64).then_some(raw_id);
    ensure!(
        journal_id == expected_journal,
        "delta journal identity mismatch"
    );
    let mut overlay = MftOverlay::default();
    let mut cursor = JOURNAL_HEADER;
    let mut next_usn = None;
    let mut visible_count = 0;
    let mut checkpoint_len = 0;
    while bytes.len() - cursor >= 8 {
        let length = usize::try_from(read_u64(bytes, cursor)?)?;
        ensure!(
            (FRAME_HEADER + CHECKSUM..=MAX_FRAME_BYTES).contains(&length),
            "delta transaction length invalid"
        );
        let end = cursor
            .checked_add(length)
            .context("delta transaction offset overflow")?;
        if end > bytes.len() {
            break;
        } // A torn last append never publishes its cursor or rows.
        let frame = &bytes[cursor..end];
        let payload_end = length - CHECKSUM;
        ensure!(
            Sha256::digest(&frame[..payload_end]).as_slice() == &frame[payload_end..],
            "delta transaction checksum mismatch"
        );
        let flags = read_u32(frame, 36)?;
        ensure!(
            flags == 0 || flags == FULL,
            "delta transaction flags invalid"
        );
        ensure!(
            (cursor == JOURNAL_HEADER && flags == FULL) || (cursor != JOURNAL_HEADER && flags == 0),
            "delta checkpoint must appear only as the first transaction"
        );
        let raw_cursor = read_i64(frame, 8)?;
        let committed_cursor = (raw_cursor != NONE_I64).then_some(raw_cursor);
        ensure!(
            next_usn <= committed_cursor,
            "delta transaction cursor moved backwards"
        );
        if flags == FULL {
            overlay = MftOverlay::default();
            checkpoint_len = end as u64;
        }
        let replacements = read_u32(frame, 24)? as usize;
        let deleted = read_u32(frame, 28)? as usize;
        let resets = read_u32(frame, 32)? as usize;
        let mut offset = FRAME_HEADER;
        let mut touched = HashSet::new();
        for _ in 0..replacements {
            let end = offset
                .checked_add(DELTA_RECORD_SIZE)
                .context("delta record overflow")?;
            let record = frame
                .get(offset..end)
                .filter(|_| end <= payload_end)
                .context("delta record truncated")?;
            offset = end;
            let reference = read_u64(record, 0)?;
            ensure!(touched.insert(reference), "duplicate delta reference");
            let path_len = read_u32(record, 32)? as usize;
            let name_len = read_u32(record, 36)? as usize;
            let end = offset
                .checked_add(path_len)
                .and_then(|end| end.checked_add(name_len))
                .context("delta strings overflow")?;
            let text = frame
                .get(offset..end)
                .filter(|_| end <= payload_end)
                .context("delta strings truncated")?;
            let flags = record[48];
            ensure!(
                flags & !(FLAG_IDENTITY | FLAG_DIRECTORY) == 0,
                "delta record flags invalid"
            );
            let size = read_u64(record, 16)?;
            let modified = read_i64(record, 24)?;
            overlay.deleted.remove(&reference);
            overlay.replacements.insert(
                reference,
                MftEntry {
                    identity: (flags & FLAG_IDENTITY != 0).then_some(MftIdentity {
                        file_reference: reference,
                        parent_reference: read_u64(record, 8)?,
                    }),
                    path: std::str::from_utf8(&text[..path_len])?.to_owned(),
                    name: std::str::from_utf8(&text[path_len..])?.to_owned(),
                    is_directory: flags & FLAG_DIRECTORY != 0,
                    size_bytes: (size != NONE_U64).then_some(size),
                    modified_at_unix_ms: (modified != NONE_I64).then_some(modified),
                }
                .into(),
            );
            offset = end;
        }
        for index in 0..deleted
            .checked_add(resets)
            .context("delta reference count overflow")?
        {
            ensure!(offset + 8 <= payload_end, "delta reference truncated");
            let reference = read_u64(frame, offset)?;
            ensure!(touched.insert(reference), "duplicate delta reference");
            overlay.replacements.remove(&reference);
            if index < deleted {
                overlay.deleted.insert(reference);
            } else {
                overlay.deleted.remove(&reference);
            }
            offset += 8;
        }
        ensure!(
            offset == payload_end,
            "delta transaction payload length mismatch"
        );
        next_usn = committed_cursor;
        visible_count = usize::try_from(read_u64(frame, 16)?)?;
        cursor = end;
    }
    ensure!(
        checkpoint_len != 0,
        "delta journal has no complete checkpoint"
    );
    Ok(PersistedDelta {
        journal_id,
        next_usn,
        visible_count,
        checkpoint: Some(DeltaCheckpoint {
            overlay: overlay.clone(),
            journal_id,
            next_usn,
            visible_count,
            valid_len: cursor as u64,
            checkpoint_len,
            base_token: bytes[32..64]
                .try_into()
                .expect("validated base token length"),
        }),
        overlay,
    })
}

#[cfg(test)]
mod tests;
