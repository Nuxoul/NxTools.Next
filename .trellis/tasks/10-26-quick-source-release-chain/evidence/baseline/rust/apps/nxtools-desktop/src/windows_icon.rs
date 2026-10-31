const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";

pub fn encode_png_ico(images: &[(u8, &[u8])]) -> Result<Vec<u8>, &'static str> {
    if images.is_empty() || images.len() > u16::MAX as usize {
        return Err("a Windows icon needs a bounded non-empty image set");
    }
    if images
        .iter()
        .any(|(size, bytes)| *size == 0 || !bytes.starts_with(PNG_SIGNATURE))
    {
        return Err("every Windows icon entry must be a sized PNG image");
    }

    let directory_len = 6usize
        .checked_add(
            16usize
                .checked_mul(images.len())
                .ok_or("icon directory overflow")?,
        )
        .ok_or("icon directory overflow")?;
    let payload_len = images.iter().try_fold(0usize, |total, (_, bytes)| {
        total
            .checked_add(bytes.len())
            .ok_or("icon payload overflow")
    })?;
    let mut encoded = Vec::with_capacity(
        directory_len
            .checked_add(payload_len)
            .ok_or("icon payload overflow")?,
    );
    encoded.extend_from_slice(&0u16.to_le_bytes());
    encoded.extend_from_slice(&1u16.to_le_bytes());
    encoded.extend_from_slice(&(images.len() as u16).to_le_bytes());

    let mut offset = u32::try_from(directory_len).map_err(|_| "icon directory overflow")?;
    for (size, bytes) in images {
        let byte_len = u32::try_from(bytes.len()).map_err(|_| "icon image is too large")?;
        encoded.push(*size);
        encoded.push(*size);
        encoded.push(0);
        encoded.push(0);
        encoded.extend_from_slice(&1u16.to_le_bytes());
        encoded.extend_from_slice(&32u16.to_le_bytes());
        encoded.extend_from_slice(&byte_len.to_le_bytes());
        encoded.extend_from_slice(&offset.to_le_bytes());
        offset = offset
            .checked_add(byte_len)
            .ok_or("icon payload overflow")?;
    }
    for (_, bytes) in images {
        encoded.extend_from_slice(bytes);
    }
    Ok(encoded)
}

#[cfg(test)]
mod tests {
    use super::encode_png_ico;

    #[test]
    fn encodes_bounded_png_entries_with_stable_offsets() {
        let first = b"\x89PNG\r\n\x1a\nfirst";
        let second = b"\x89PNG\r\n\x1a\nsecond";
        let encoded = encode_png_ico(&[(16, first), (32, second)]).expect("encode icon");

        assert_eq!(&encoded[0..6], &[0, 0, 1, 0, 2, 0]);
        assert_eq!(&encoded[6..8], &[16, 16]);
        assert_eq!(
            u32::from_le_bytes(encoded[18..22].try_into().expect("offset")),
            38
        );
        assert_eq!(&encoded[22..24], &[32, 32]);
        assert_eq!(
            u32::from_le_bytes(encoded[34..38].try_into().expect("offset")),
            38 + first.len() as u32
        );
        assert_eq!(&encoded[38..38 + first.len()], first);
        assert_eq!(&encoded[38 + first.len()..], second);
    }

    #[test]
    fn rejects_empty_or_non_png_entries() {
        assert!(encode_png_ico(&[]).is_err());
        assert!(encode_png_ico(&[(16, b"not-png")]).is_err());
        assert!(encode_png_ico(&[(0, b"\x89PNG\r\n\x1a\n")]).is_err());
    }
}
