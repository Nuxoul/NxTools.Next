use thiserror::Error;
use url::Url;

use crate::{DownloadProbe, HlsProbe, HlsVariantProbe};

pub(crate) const MAX_HLS_MANIFEST_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_HLS_VARIANTS: usize = 32;
pub const MAX_HLS_VARIANT_URL_BYTES: usize = 8 * 1024;
pub const MAX_HLS_ATTRIBUTE_BYTES: usize = 512;
pub const MAX_HLS_PROBE_SERIALIZED_BYTES: usize = 512 * 1024;

pub(crate) fn validate_hls_probe_size(probe: &DownloadProbe) -> Result<(), HlsProbeError> {
    let encoded = serde_json::to_vec(probe).map_err(|_| HlsProbeError::MetadataTooLarge)?;
    if encoded.len() > MAX_HLS_PROBE_SERIALIZED_BYTES {
        return Err(HlsProbeError::MetadataTooLarge);
    }
    Ok(())
}

pub(crate) fn parse_hls_probe(playlist_url: &Url, bytes: &[u8]) -> Result<HlsProbe, HlsProbeError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| HlsProbeError::InvalidManifest("manifest is not UTF-8"))?;
    let text = text.trim_start_matches('\u{feff}');
    if text
        .lines()
        .next()
        .is_none_or(|line| line.trim() != "#EXTM3U")
    {
        return Err(HlsProbeError::InvalidManifest("missing EXTM3U header"));
    }

    let lines = text.lines().map(str::trim).collect::<Vec<_>>();
    let mut variants = Vec::new();
    let mut has_media_segment = false;
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        if let Some(attributes) = line.strip_prefix("#EXT-X-STREAM-INF:") {
            if variants.len() >= MAX_HLS_VARIANTS {
                return Err(HlsProbeError::TooManyVariants);
            }
            let uri = following_variant_uri(&lines, index)?;
            let attributes = parse_attributes(attributes)?;
            let bandwidth = parse_u64(attribute(&attributes, "BANDWIDTH")).ok_or(
                HlsProbeError::InvalidManifest("variant bandwidth is missing"),
            )?;
            let average_bandwidth = parse_u64(attribute(&attributes, "AVERAGE-BANDWIDTH"));
            let resolution = attribute(&attributes, "RESOLUTION").and_then(parse_resolution);
            let frame_rate_milli = attribute(&attributes, "FRAME-RATE").and_then(parse_frame_rate);
            let playlist_url = resolve_http_url(playlist_url, uri, "variant URI is invalid")?;
            if playlist_url.as_str().len() > MAX_HLS_VARIANT_URL_BYTES {
                return Err(HlsProbeError::MetadataTooLarge);
            }
            let codecs = bounded_attribute(&attributes, "CODECS")?
                .unwrap_or_default()
                .to_owned();
            variants.push(HlsVariantProbe {
                playlist_url: playlist_url.to_string(),
                bandwidth,
                average_bandwidth,
                resolution,
                frame_rate_milli,
                codecs,
                audio_group: bounded_optional_attribute(&attributes, "AUDIO")?,
                subtitles_group: bounded_optional_attribute(&attributes, "SUBTITLES")?,
            });
        } else if line.starts_with("#EXTINF:") {
            let uri = following_media_uri(&lines, index)?;
            resolve_http_url(playlist_url, uri, "media segment URI is invalid")?;
            has_media_segment = true;
        }
        index += 1;
    }
    if variants.is_empty() && !has_media_segment {
        return Err(HlsProbeError::InvalidManifest(
            "playlist contains no playable entries",
        ));
    }
    variants.sort_by(|left, right| {
        effective_bandwidth(right)
            .cmp(&effective_bandwidth(left))
            .then_with(|| right.resolution.cmp(&left.resolution))
            .then_with(|| left.playlist_url.cmp(&right.playlist_url))
    });
    Ok(HlsProbe {
        is_master: !variants.is_empty(),
        variants,
    })
}

fn following_variant_uri<'a>(lines: &'a [&str], index: usize) -> Result<&'a str, HlsProbeError> {
    for candidate in &lines[index + 1..] {
        if candidate.is_empty() {
            continue;
        }
        if candidate.starts_with('#') {
            return Err(HlsProbeError::InvalidManifest("variant URI is missing"));
        }
        return Ok(candidate);
    }
    Err(HlsProbeError::InvalidManifest("variant URI is missing"))
}

fn following_media_uri<'a>(lines: &'a [&str], index: usize) -> Result<&'a str, HlsProbeError> {
    for candidate in &lines[index + 1..] {
        if candidate.is_empty() || candidate.starts_with('#') {
            if candidate.starts_with("#EXTINF:") || candidate.starts_with("#EXT-X-STREAM-INF:") {
                break;
            }
            continue;
        }
        return Ok(candidate);
    }
    Err(HlsProbeError::InvalidManifest(
        "media segment URI is missing",
    ))
}

fn resolve_http_url(
    base_url: &Url,
    value: &str,
    invalid_reason: &'static str,
) -> Result<Url, HlsProbeError> {
    let url = base_url
        .join(value)
        .map_err(|_| HlsProbeError::InvalidManifest(invalid_reason))?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(HlsProbeError::InvalidManifest(invalid_reason));
    }
    Ok(url)
}

fn effective_bandwidth(variant: &HlsVariantProbe) -> u64 {
    variant.average_bandwidth.unwrap_or(variant.bandwidth)
}

fn parse_attributes(input: &str) -> Result<Vec<(String, String)>, HlsProbeError> {
    let mut attributes = Vec::new();
    let mut cursor = 0;
    let bytes = input.as_bytes();
    while cursor < bytes.len() {
        while cursor < bytes.len() && matches!(bytes[cursor], b',' | b' ' | b'\t') {
            cursor += 1;
        }
        if cursor == bytes.len() {
            break;
        }
        let key_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'=' {
            cursor += 1;
        }
        if cursor == bytes.len() {
            return Err(HlsProbeError::InvalidManifest("malformed attribute"));
        }
        let key = input[key_start..cursor].trim().to_ascii_uppercase();
        cursor += 1;
        let value = if cursor < bytes.len() && bytes[cursor] == b'"' {
            cursor += 1;
            let value_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b'"' {
                cursor += 1;
            }
            if cursor == bytes.len() {
                return Err(HlsProbeError::InvalidManifest(
                    "unterminated quoted attribute",
                ));
            }
            let value = input[value_start..cursor].to_owned();
            cursor += 1;
            value
        } else {
            let value_start = cursor;
            while cursor < bytes.len() && bytes[cursor] != b',' {
                cursor += 1;
            }
            input[value_start..cursor].trim().to_owned()
        };
        attributes.push((key, value));
    }
    Ok(attributes)
}

fn attribute<'a>(attributes: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attributes
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.as_str())
}

fn bounded_attribute<'a>(
    attributes: &'a [(String, String)],
    name: &str,
) -> Result<Option<&'a str>, HlsProbeError> {
    let value = attribute(attributes, name);
    if value.is_some_and(|value| value.len() > MAX_HLS_ATTRIBUTE_BYTES) {
        return Err(HlsProbeError::MetadataTooLarge);
    }
    Ok(value)
}

fn bounded_optional_attribute(
    attributes: &[(String, String)],
    name: &str,
) -> Result<Option<String>, HlsProbeError> {
    Ok(bounded_attribute(attributes, name)?
        .filter(|value| !value.is_empty())
        .map(str::to_owned))
}

fn parse_u64(value: Option<&str>) -> Option<u64> {
    value?.parse().ok()
}

fn parse_resolution(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.split_once(['x', 'X'])?;
    Some((width.parse().ok()?, height.parse().ok()?))
}

fn parse_frame_rate(value: &str) -> Option<u32> {
    let value: f64 = value.parse().ok()?;
    value
        .is_finite()
        .then(|| (value * 1000.0).round())
        .filter(|value| *value >= 0.0 && *value <= u32::MAX as f64)
        .map(|value| value as u32)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(crate) enum HlsProbeError {
    #[error("invalid HLS manifest: {0}")]
    InvalidManifest(&'static str),
    #[error("HLS manifest contains more than 32 variants")]
    TooManyVariants,
    #[error("HLS presentation metadata exceeds its bounded field limits")]
    MetadataTooLarge,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_variant_cannot_steal_the_uri_from_a_later_stream_entry() {
        let manifest = br#"#EXTM3U
#EXT-X-STREAM-INF:BANDWIDTH=800000
#EXT-X-STREAM-INF:BANDWIDTH=4200000
high/index.m3u8
"#;

        let error = parse_hls_probe(
            &Url::parse("https://example.test/master.m3u8").expect("base URL"),
            manifest,
        )
        .expect_err("reject missing first variant URI");

        assert_eq!(
            error,
            HlsProbeError::InvalidManifest("variant URI is missing")
        );
    }

    #[test]
    fn presentation_strings_are_bounded_before_they_reach_ipc() {
        let oversized_codecs = "x".repeat(MAX_HLS_ATTRIBUTE_BYTES + 1);
        let manifest = format!(
            "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=800000,CODECS=\"{oversized_codecs}\"\nlow.m3u8\n"
        );

        let error = parse_hls_probe(
            &Url::parse("https://example.test/master.m3u8").expect("base URL"),
            manifest.as_bytes(),
        )
        .expect_err("reject oversized presentation metadata");

        assert_eq!(error, HlsProbeError::MetadataTooLarge);
    }

    #[test]
    fn aggregate_probe_budget_includes_top_level_response_fields() {
        let probe = crate::DownloadProbe {
            final_url: "u".repeat(MAX_HLS_PROBE_SERIALIZED_BYTES),
            file_name: Some("master.m3u8".into()),
            total_bytes: Some(1),
            supports_ranges: false,
            content_type: Some("application/vnd.apple.mpegurl".into()),
            validator: crate::ResourceValidator::default(),
            media: Some(crate::DownloadMediaProbe::Hls(HlsProbe {
                is_master: true,
                variants: Vec::new(),
            })),
        };

        assert_eq!(
            validate_hls_probe_size(&probe),
            Err(HlsProbeError::MetadataTooLarge)
        );
    }
}
