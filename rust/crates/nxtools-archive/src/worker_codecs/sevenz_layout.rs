//! Original ordinals remain authoritative when empty records interrupt a solid block's file span.
use super::*;

pub(super) struct DecoderLayout {
    pub archive: sevenz_rust2::Archive,
    pub ordinals: Vec<usize>,
}

pub(super) fn normalize(
    archive: &sevenz_rust2::Archive,
    retained: u64,
    limits: &ResourceLimits,
    control: &OperationControl,
) -> Result<Option<DecoderLayout>> {
    let interleaved = archive.files.iter().enumerate().any(|(index, file)| {
        !file.has_stream && archive.stream_map.file_block_index[index].is_some()
    });
    if !interleaved {
        return Ok(None);
    }
    let mapping_bytes = (archive.files.len() as u64)
        .checked_mul(2 * std::mem::size_of::<usize>() as u64)
        .ok_or(ArchiveError::ResourceLimit)?;
    if retained
        .checked_mul(2)
        .and_then(|cost| cost.checked_add(mapping_bytes))
        .is_none_or(|cost| cost > limits.max_metadata_bytes)
    {
        return Err(ArchiveError::ResourceLimit);
    }
    let mut ordinals: Vec<usize> = (0..archive.files.len()).collect();
    ordinals.sort_unstable_by_key(|&index| {
        let file = &archive.files[index];
        let block = file
            .has_stream
            .then(|| archive.stream_map.file_block_index[index])
            .flatten()
            .unwrap_or(usize::MAX);
        (block, index)
    });
    control.checkpoint()?;
    let mut normalized = archive.clone();
    let mut destinations = vec![0; ordinals.len()];
    for (decoded, &original) in ordinals.iter().enumerate() {
        destinations[original] = decoded;
    }
    for index in 0..destinations.len() {
        control.checkpoint()?;
        while destinations[index] != index {
            let destination = destinations[index];
            normalized.files.swap(index, destination);
            destinations.swap(index, destination);
        }
    }
    for (decoded, &original) in ordinals.iter().enumerate() {
        normalized.stream_map.file_block_index[decoded] = normalized.files[decoded]
            .has_stream
            .then(|| archive.stream_map.file_block_index[original])
            .flatten();
    }
    let mut cursor = 0;
    let mut empty = std::io::Cursor::new([]);
    for block in 0..normalized.blocks.len() {
        control.checkpoint()?;
        normalized.stream_map.block_first_file_index[block] = cursor;
        let expected = sevenz_rust2::BlockDecoder::new(
            1,
            block,
            &normalized,
            &sevenz_rust2::Password::default(),
            &mut empty,
        )
        .entry_count();
        let end = cursor
            .checked_add(expected)
            .filter(|&end| end <= normalized.files.len())
            .ok_or(ArchiveError::CorruptArchive)?;
        if (cursor..end).any(|index| {
            !normalized.files[index].has_stream
                || normalized.stream_map.file_block_index[index] != Some(block)
        }) {
            return Err(ArchiveError::CorruptArchive);
        }
        cursor = end;
    }
    if normalized.files[cursor..]
        .iter()
        .any(|file| file.has_stream)
    {
        return Err(ArchiveError::CorruptArchive);
    }
    Ok(Some(DecoderLayout {
        archive: normalized,
        ordinals,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interleaved_empty_records_preserve_all_original_ordinals_and_payload_sizes() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/py7zr-interleaved-empty.7z");
        let source = File::open(path).expect("independent py7zr source");
        let original = sevenz_rust2::ArchiveReader::new(
            source.try_clone().expect("source reader"),
            sevenz_rust2::Password::default(),
        )
        .expect("catalog");
        let layout = normalize(
            original.archive(),
            1 << 20,
            &ResourceLimits::default(),
            &OperationControl::default(),
        )
        .expect("finite layout")
        .expect("interspersed empty entries");
        assert_eq!(layout.ordinals.len(), 38);
        let mut reader = sevenz_rust2::ArchiveReader::from_archive(
            layout.archive,
            source,
            sevenz_rust2::Password::default(),
        );
        reader.set_thread_count(1);
        let mut cursor = 0;
        let mut files = 0;
        reader
            .for_each_entries(|entry, input| {
                let original = &original.archive().files[layout.ordinals[cursor]];
                assert_eq!(entry.name, original.name);
                assert_eq!(entry.is_directory, original.is_directory);
                let bytes =
                    std::io::copy(input, &mut std::io::sink()).expect("full original member");
                assert_eq!(bytes, original.size);
                files += usize::from(!entry.is_directory);
                cursor += 1;
                Ok(true)
            })
            .expect("every member integrity");
        assert_eq!(cursor, 38, "no catalog item is silently dropped");
        assert_eq!(
            files,
            original
                .archive()
                .files
                .iter()
                .filter(|file| !file.is_directory)
                .count()
        );
        let insufficient = ResourceLimits {
            max_metadata_bytes: 1024,
            ..Default::default()
        };
        assert!(matches!(
            normalize(
                original.archive(),
                1 << 20,
                &insufficient,
                &OperationControl::default()
            ),
            Err(ArchiveError::ResourceLimit)
        ));
    }
}
